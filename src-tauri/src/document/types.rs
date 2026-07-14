//! Core types for document parsing and chunking (Sprint 3).
//!
//! ParsedBlock  — output unit from parser.py / ocr_parser.py
//! Chunk        — pre-embedding chunk, mirrors db::chunk_repo::NewChunk
//! ChunkingConfig — tuneable parameters (used in Sprint 11 eval harness too)

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Parser output
// ---------------------------------------------------------------------------

/// A single structural block produced by the document parser.
///
/// `heading_path` is the breadcrumb of headings that contain this block,
/// e.g. `["Chapter 3", "Methodology", "Data Collection"]`.
/// Empty when text precedes the first heading in the document.
///
/// `page_number` is 1-indexed; `None` for formats that carry no page concept
/// (e.g. plain TXT, MD).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedBlock {
    pub heading_path: Vec<String>,
    pub page_number: Option<u32>,
    pub text: String,
    /// Average OCR confidence in [0, 100]; None for native text extractions.
    pub ocr_confidence: Option<f32>,
}

impl ParsedBlock {
    /// The deepest heading in the breadcrumb, suitable for storing in
    /// `document_chunks.section_heading`.
    pub fn section_heading(&self) -> Option<&str> {
        self.heading_path.last().map(|s| s.as_str())
    }

    /// Serialised breadcrumb string for `heading_path` column.
    pub fn heading_path_str(&self) -> String {
        self.heading_path.join(" > ")
    }
}

// ---------------------------------------------------------------------------
// Chunk (pre-embedding)
// ---------------------------------------------------------------------------

/// A single chunk ready to be inserted into `document_chunks` via
/// `db::chunk_repo::insert_batch`.  No vector yet — that is Sprint 5.
#[derive(Debug, Clone)]
pub struct Chunk {
    /// UUID assigned by the chunker (matches what will become
    /// `document_chunks.id`).
    pub id: String,
    pub document_id: String,
    pub chunk_index: u32,
    pub content: String,
    pub token_count: u32,
    pub page_number: Option<u32>,
    /// Deepest heading — stored in `document_chunks.section_heading`.
    pub section_heading: Option<String>,
    /// Full breadcrumb path serialised as "H1 > H2 > H3".
    pub heading_path: Option<String>,
    /// Byte offset of the chunk's start within the full document text.
    pub start_char: Option<u32>,
    /// Byte offset of the chunk's end.
    pub end_char: Option<u32>,
    /// JSON object for extra fields, e.g. `{"ocr_confidence": 87.2}`.
    pub metadata: Option<String>,
}

// ---------------------------------------------------------------------------
// Chunking configuration
// ---------------------------------------------------------------------------

/// Parameters that control how a document is split into chunks.
/// All values are tuneable — Sprint 11 eval harness will sweep these.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkingConfig {
    /// Target chunk size in tokens (whitespace-split approximation).
    /// Default 500 — middle of the 400-600 range from the sprint spec.
    pub chunk_size_tokens: usize,
    /// Overlap between adjacent chunks *within the same section*.
    /// Never spans heading boundaries.
    /// Default 75 ≈ 15% of 500.
    pub overlap_tokens: usize,
    /// Chunks below this size are merged into their neighbour instead of
    /// being indexed standalone.  Catches stray headers and page numbers.
    pub min_tokens: usize,
}

impl Default for ChunkingConfig {
    fn default() -> Self {
        Self {
            chunk_size_tokens: 500,
            overlap_tokens: 75,
            min_tokens: 30,
        }
    }
}
