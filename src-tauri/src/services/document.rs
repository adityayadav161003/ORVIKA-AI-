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
use crate::embedding::EmbeddingEngine;
use crate::python::manager::PythonManager;
use crate::utils::error::{AppError, AppResult};
use crate::vector_store::VectorStore;

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
pub async fn ingest(
    document_id: &str,
    file_path: &Path,
    file_type: &str,
    db: &Arc<Database>,
    python_manager: &Arc<PythonManager>,
    embedding_engine: &Arc<EmbeddingEngine>,
    vector_store: &Arc<VectorStore>,
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
    // Also delete any existing vectors from Chroma
    let _ = vector_store.remove_document(document_id).await;

    tracing::debug!(document_id, "Cleared existing chunks");

    // -----------------------------------------------------------------------
    // Step 2: parse → Vec<ParsedBlock>
    // -----------------------------------------------------------------------
    let is_image_type = matches!(
        file_type,
        "png" | "jpg" | "jpeg" | "tiff" | "tif" | "bmp" | "webp"
    );

    let blocks = if is_image_type {
        // Images always go through OCR
        let ocr = OcrProcessor::new(python_manager.as_ref());
        ocr.process(file_path).map_err(|e| {
            AppError::Other(format!(
                "OCR failed for image '{}': {}",
                file_path.display(),
                e
            ))
        })?
    } else {
        // Try MarkItDown first (handles PDF, DOCX, TXT, MD, HTML, …)
        let parser = DocumentParser::new(python_manager.as_ref());
        let mut parsed = parser.parse(file_path).map_err(|e| {
            AppError::Other(format!("Parse failed for '{}': {}", file_path.display(), e))
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

    tracing::debug!(
        document_id,
        block_count = blocks.len(),
        "Parsed into blocks"
    );

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
        .iter()
        .map(|c| NewChunk {
            id: c.id.clone(),
            document_id: c.document_id.clone(),
            chunk_index: c.chunk_index,
            content: c.content.clone(),
            token_count: c.token_count,
            page_number: c.page_number,
            section_heading: c.section_heading.clone(),
            metadata: c.metadata.clone(),
        })
        .collect();

    db.with_connection(|conn| chunk_repo::insert_batch(conn, &new_chunks))?;

    tracing::debug!(document_id, chunk_count, "Chunks inserted into SQLite");

    // -----------------------------------------------------------------------
    // Step 5: compute embeddings and index into Chroma (Sprint 5)
    // -----------------------------------------------------------------------
    let chunk_texts: Vec<String> = chunks.iter().map(|c| c.content.clone()).collect();
    let chunk_ids: Vec<String> = chunks.iter().map(|c| c.id.clone()).collect();

    tracing::info!(
        document_id,
        count = chunk_count,
        "Generating embeddings for chunks..."
    );
    match embedding_engine.embed_batch(&chunk_texts) {
        Ok(embeddings) => {
            let mut indexable_chunks = Vec::with_capacity(chunk_count);
            for (i, emb) in embeddings.into_iter().enumerate() {
                let chunk = &chunks[i];
                indexable_chunks.push(crate::vector_store::types::IndexableChunk {
                    id: chunk.id.clone(),
                    embedding: emb,
                    document_id: document_id.to_string(),
                    chunk_index: chunk.chunk_index,
                    page_number: chunk.page_number,
                    section_heading: chunk.section_heading.clone(),
                    heading_path: None, // Can be populated if heading paths are supported in parser
                    source_type: "document".to_string(),
                    token_count: chunk.token_count,
                    content: chunk.content.clone(),
                });
            }

            tracing::info!(document_id, "Writing chunks to Chroma...");
            match vector_store.add_chunks(indexable_chunks).await {
                Ok(_) => {
                    tracing::info!(document_id, "Successfully indexed chunks in Chroma.");
                    let _ = db.with_connection(|conn| {
                        chunk_repo::update_indexing_status(conn, &chunk_ids, true)?;
                        document_repo::update_indexed_status(conn, document_id)
                    });
                }
                Err(err) => {
                    tracing::error!(document_id, error = %err, "Failed to index chunks in Chroma");
                    let _ = db.with_connection(|conn| {
                        chunk_repo::update_indexing_status(conn, &chunk_ids, false)
                    });
                }
            }
        }
        Err(err) => {
            tracing::error!(document_id, error = %err, "Failed to compute embeddings");
            let _ = db.with_connection(|conn| {
                chunk_repo::update_indexing_status(conn, &chunk_ids, false)
            });
        }
    }

    // -----------------------------------------------------------------------
    // Step 6: mark document as parsed
    // -----------------------------------------------------------------------
    db.with_connection(|conn| {
        document_repo::update_parsed_status(conn, document_id, chunk_count as u32)
    })?;

    tracing::info!(
        document_id,
        chunk_count,
        "Document ingestion complete (chunks & vectors indexed)"
    );

    Ok(chunk_count)
}
