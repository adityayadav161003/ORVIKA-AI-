//! OCR processor — Sprint 3.
//!
//! Drives `python/ocr_parser.py` (Tesseract + pdf2image) via `PythonManager`
//! and converts the flat OCR output into `Vec<ParsedBlock>` — the same format
//! that `document::parser::DocumentParser` produces — so the chunker is
//! format-agnostic and works identically for both native-text and scanned docs.
//!
//! The OCR confidence scores emitted by `ocr_parser.py` are stored in each
//! block's `ocr_confidence` field and ultimately land in the chunk's
//! `metadata` JSON column.  This lets the UI flag low-confidence citations
//! differently from clean text extractions (Sprint 7).

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};

use serde::Deserialize;

use crate::document::types::ParsedBlock;
use crate::python::manager::PythonManager;
use crate::utils::error::{AppError, AppResult};

// ---------------------------------------------------------------------------
// Wire types that map to ocr_parser.py JSON output
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct OcrResponse {
    status: String,
    /// Flat OCR text with `--- Page N ---` markers inserted by ocr_parser.py
    content: Option<String>,
    /// One confidence value per page, in [0, 100].
    confidence_per_page: Option<Vec<f32>>,
    error: Option<String>,
    traceback: Option<String>,
}

// ---------------------------------------------------------------------------
// OcrProcessor
// ---------------------------------------------------------------------------

pub struct OcrProcessor<'a> {
    python_manager: &'a PythonManager,
}

impl<'a> OcrProcessor<'a> {
    pub fn new(python_manager: &'a PythonManager) -> Self {
        Self { python_manager }
    }

    /// Run OCR on `file_path` and return structured blocks.
    ///
    /// Each page becomes its own block (no heading information available from
    /// OCR output — heading_path is empty for all OCR blocks).  The per-page
    /// confidence score is stored in `ocr_confidence`.
    ///
    /// Returns `Err` only for hard failures (Tesseract not installed, subprocess
    /// crash).  A successfully OCR'd page with low confidence is returned as a
    /// normal block — the caller / UI decides how to surface the quality warning.
    pub fn process(&self, file_path: &Path) -> AppResult<Vec<ParsedBlock>> {
        let python_exe = self.python_manager.python_exe()?;
        let script_path = self.python_manager.resolve_script("ocr_parser.py")?;

        let mut child = Command::new(&python_exe)
            .arg(&script_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| {
                AppError::Other(format!("Failed to spawn ocr_parser.py: {}", e))
            })?;

        let request =
            serde_json::json!({ "file_path": file_path.to_string_lossy() });
        let request_line = serde_json::to_string(&request).unwrap() + "\n";

        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(request_line.as_bytes())
                .map_err(|e| {
                    AppError::Other(format!("Failed to write to ocr_parser stdin: {}", e))
                })?;
        }

        let stdout = child.stdout.take().expect("stdout should be piped");
        let mut reader = BufReader::new(stdout);
        let mut response_line = String::new();
        reader
            .read_line(&mut response_line)
            .map_err(|e| {
                AppError::Other(format!("Failed to read ocr_parser stdout: {}", e))
            })?;

        let _ = child.wait();

        if response_line.trim().is_empty() {
            return Err(AppError::Other(
                "ocr_parser.py returned no output — is Tesseract installed?".into(),
            ));
        }

        let response: OcrResponse = serde_json::from_str(response_line.trim())
            .map_err(|e| {
                AppError::Other(format!(
                    "Failed to deserialise ocr_parser.py response: {}",
                    e
                ))
            })?;

        if response.status != "success" {
            return Err(AppError::Other(format!(
                "OCR error: {} {:?}",
                response.error.unwrap_or_default(),
                response.traceback
            )));
        }

        let content = response.content.unwrap_or_default();
        let confidences = response.confidence_per_page.unwrap_or_default();

        let blocks = self.split_into_blocks(&content, &confidences);

        tracing::info!(
            file = %file_path.display(),
            page_count = blocks.len(),
            avg_confidence = %avg_confidence(&confidences),
            "OCR completed"
        );

        Ok(blocks)
    }

    // -----------------------------------------------------------------------
    // Internal: split flat OCR text on "--- Page N ---" markers
    // -----------------------------------------------------------------------

    /// `ocr_parser.py` inserts `\n--- Page {i+1} ---\n` between pages.
    /// We split on those markers and assign each segment to a page block.
    fn split_into_blocks(&self, content: &str, confidences: &[f32]) -> Vec<ParsedBlock> {
        // Split on the page marker pattern produced by ocr_parser.py.
        // The regex is not available here (no dep import), so we use a simple
        // line scan.
        let mut blocks: Vec<ParsedBlock> = Vec::new();
        let mut current_page_text: Vec<&str> = Vec::new();
        let mut current_page: u32 = 1;

        for line in content.lines() {
            let trimmed = line.trim();
            // Detect "--- Page N ---" markers
            if trimmed.starts_with("--- Page") && trimmed.ends_with("---") {
                // Flush current page
                let text = current_page_text.join("\n").trim().to_owned();
                if !text.is_empty() {
                    let confidence =
                        confidences.get((current_page as usize) - 1).copied();
                    blocks.push(ParsedBlock {
                        heading_path: vec![],
                        page_number: Some(current_page),
                        text,
                        ocr_confidence: confidence,
                    });
                }
                current_page_text.clear();
                // Parse the page number from the marker if possible
                if let Some(page_num) = parse_page_number(trimmed) {
                    current_page = page_num;
                } else {
                    current_page += 1;
                }
            } else {
                current_page_text.push(line);
            }
        }

        // Flush final page
        let text = current_page_text.join("\n").trim().to_owned();
        if !text.is_empty() {
            let confidence = confidences.get((current_page as usize) - 1).copied();
            blocks.push(ParsedBlock {
                heading_path: vec![],
                page_number: Some(current_page),
                text,
                ocr_confidence: confidence,
            });
        }

        // If no page markers were present, treat the whole content as page 1.
        if blocks.is_empty() && !content.trim().is_empty() {
            let confidence = confidences.first().copied();
            blocks.push(ParsedBlock {
                heading_path: vec![],
                page_number: Some(1),
                text: content.trim().to_owned(),
                ocr_confidence: confidence,
            });
        }

        blocks
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Extract the page number from a marker like `--- Page 3 ---`.
fn parse_page_number(marker: &str) -> Option<u32> {
    // Marker format: "--- Page N ---"
    let inner = marker.trim_matches('-').trim();
    let page_part = inner.trim_start_matches("Page").trim();
    page_part.parse::<u32>().ok()
}

fn avg_confidence(confidences: &[f32]) -> String {
    if confidences.is_empty() {
        return "n/a".to_owned();
    }
    let avg = confidences.iter().sum::<f32>() / confidences.len() as f32;
    format!("{:.1}%", avg)
}
