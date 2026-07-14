//! Document service — Sprint 3.
//!
//! Orchestrates the full ingestion pipeline:
//!   parse (or OCR) → chunk → insert into SQLite → mark document as parsed.
//!
//! ## Idempotency guarantee
//!
//! Calling `ingest()` on a document that has already been ingested is safe:
//! existing chunks are deleted before re-inserting, so a re-upload never
//! produces duplicates.  This also means a mid-parse crash can be recovered by
//! simply re-running ingestion — the previous partial batch is wiped first.
//!
//! ## Scanned vs. native detection
//!
//! For PDFs that come back with < 20 characters of text from MarkItDown we
//! assume the file is scanned and fall back to the OCR path automatically.
//! This heuristic covers the common case; it can be made more sophisticated in
//! a future sprint.

use std::path::Path;
use std::sync::Arc;

use crate::db::chunk_repo::{self, NewChunk};
use crate::db::document_repo;
use crate::db::Database;
use crate::document::chunker::chunk_blocks;
use crate::document::ocr::OcrProcessor;
use crate::document::parser::DocumentParser;
use crate::document::types::ChunkingConfig;
use crate::python::manager::PythonManager;
use crate::utils::error::{AppError, AppResult};

/// Minimum character count from MarkItDown to consider a PDF "native text".
/// Below this we fall back to OCR.
const SCANNED_PDF_THRESHOLD_CHARS: usize = 20;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Ingest a document: parse (or OCR), chunk, and store chunks in SQLite.
///
/// Returns the number of chunks inserted.
///
/// # Arguments
/// * `document_id` — the UUID already created in the `documents` table
/// * `file_path`   — path to the source file (already copied to app_data_dir)
/// * `file_type`   — lowercase extension, e.g. `"pdf"`, `"docx"`, `"txt"`
/// * `db`          — shared database handle
/// * `python_manager` — manages the Python subprocess environment
/// * `config`      — optional chunking config; falls back to `ChunkingConfig::default()`
pub fn ingest(
    document_id: &str,
    file_path: &Path,
    file_type: &str,
    db: &Arc<Database>,
    python_manager: &Arc<PythonManager>,
    config: Option<ChunkingConfig>,
) -> AppResult<usize> {
    let config = config.unwrap_or_default();

    tracing::info!(
        document_id,
        file = %file_path.display(),
        file_type,
        "Starting document ingestion"
    );

    // -----------------------------------------------------------------------
    // Step 1: delete any existing chunks for this document (idempotency)
    // -----------------------------------------------------------------------
    db.with_connection(|conn| chunk_repo::delete_for_document(conn, document_id))?;

    tracing::debug!(document_id, "Cleared existing chunks");

    // -----------------------------------------------------------------------
    // Step 2: parse → Vec<ParsedBlock>
    // -----------------------------------------------------------------------
    let is_image_type = matches!(file_type, "png" | "jpg" | "jpeg" | "tiff" | "tif" | "bmp" | "webp");

    let blocks = if is_image_type {
        // Images always go through OCR
        let ocr = OcrProcessor::new(python_manager.as_ref());
        ocr.process(file_path).map_err(|e| {
            AppError::Other(format!("OCR failed for image '{}': {}", file_path.display(), e))
        })?
    } else {
        // Try MarkItDown first (handles PDF, DOCX, TXT, MD, HTML, …)
        let parser = DocumentParser::new(python_manager.as_ref());
        let mut parsed = parser.parse(file_path).map_err(|e| {
            AppError::Other(format!(
                "Parse failed for '{}': {}",
                file_path.display(),
                e
            ))
        })?;

        // Scanned-PDF heuristic: if the native parser returned almost no text,
        // fall back to OCR (PDF only — DOCX etc. shouldn't need it).
        let total_chars: usize = parsed.iter().map(|b| b.text.len()).sum();
        if file_type == "pdf" && total_chars < SCANNED_PDF_THRESHOLD_CHARS {
            tracing::info!(
                document_id,
                total_chars,
                "Detected scanned PDF — falling back to OCR"
            );
            let ocr = OcrProcessor::new(python_manager.as_ref());
            parsed = ocr.process(file_path).map_err(|e| {
                AppError::Other(format!(
                    "OCR fallback failed for '{}': {}",
                    file_path.display(),
                    e
                ))
            })?;
        }

        parsed
    };

    if blocks.is_empty() {
        tracing::warn!(document_id, "Parser produced zero blocks — empty document?");
        // Still mark as parsed so the UI doesn't show a perpetual spinner.
        db.with_connection(|conn| document_repo::update_parsed_status(conn, document_id, 0))?;
        return Ok(0);
    }

    tracing::debug!(document_id, block_count = blocks.len(), "Parsed into blocks");

    // -----------------------------------------------------------------------
    // Step 3: chunk blocks
    // -----------------------------------------------------------------------
    let chunks = chunk_blocks(&blocks, document_id, &config);
    let chunk_count = chunks.len();

    tracing::debug!(document_id, chunk_count, "Chunked document");

    // -----------------------------------------------------------------------
    // Step 4: insert all chunks in one batch transaction
    // -----------------------------------------------------------------------
    let new_chunks: Vec<NewChunk> = chunks
        .into_iter()
        .map(|c| NewChunk {
            id: c.id,
            document_id: c.document_id,
            chunk_index: c.chunk_index,
            content: c.content,
            token_count: c.token_count,
            page_number: c.page_number,
            section_heading: c.section_heading,
            metadata: c.metadata,
        })
        .collect();

    db.with_connection(|conn| chunk_repo::insert_batch(conn, &new_chunks))?;

    tracing::debug!(document_id, chunk_count, "Chunks inserted into SQLite");

    // -----------------------------------------------------------------------
    // Step 5: mark document as parsed
    // -----------------------------------------------------------------------
    db.with_connection(|conn| {
        document_repo::update_parsed_status(conn, document_id, chunk_count as u32)
    })?;

    tracing::info!(
        document_id,
        chunk_count,
        "Document ingestion complete (no vectors yet — Sprint 5)"
    );

    Ok(chunk_count)
}
