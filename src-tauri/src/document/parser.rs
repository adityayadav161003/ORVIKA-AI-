//! Document parser — Sprint 3.
//!
//! Drives `python/parser.py` (MarkItDown) via the existing `PythonManager`
//! subprocess mechanism and deserialises the structured block output into
//! `Vec<ParsedBlock>`.
//!
//! For scanned PDFs / images the caller should use `document::ocr::OcrProcessor`
//! instead, which drives `ocr_parser.py`.

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};

use serde::Deserialize;

use crate::document::types::ParsedBlock;
use crate::python::manager::PythonManager;
use crate::utils::error::{AppError, AppResult};

// ---------------------------------------------------------------------------
// Wire types that map to parser.py JSON output
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct PyBlock {
    heading_path: Vec<String>,
    page_number: Option<u32>,
    text: String,
    ocr_confidence: Option<f32>,
}

#[derive(Deserialize)]
struct PyParseResponse {
    status: String,
    blocks: Option<Vec<PyBlock>>,
    /// Legacy flat-text field; used as fallback when `blocks` is absent.
    content: Option<String>,
    error: Option<String>,
    traceback: Option<String>,
}

// ---------------------------------------------------------------------------
// DocumentParser
// ---------------------------------------------------------------------------

pub struct DocumentParser<'a> {
    python_manager: &'a PythonManager,
}

impl<'a> DocumentParser<'a> {
    pub fn new(python_manager: &'a PythonManager) -> Self {
        Self { python_manager }
    }

    /// Parse a document at `file_path` and return structured blocks.
    ///
    /// Supported formats (whatever MarkItDown handles): PDF, DOCX, XLSX,
    /// PPTX, TXT, MD, HTML, and more.
    ///
    /// Returns `Err` only for hard failures (Python error, subprocess crash).
    /// Empty documents return `Ok(vec![])`.
    pub fn parse(&self, file_path: &Path) -> AppResult<Vec<ParsedBlock>> {
        let python_exe = self.python_manager.python_exe()?;
        let script_path = self.python_manager.resolve_script("parser.py")?;

        let mut child = Command::new(&python_exe)
            .arg(&script_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| AppError::Other(format!("Failed to spawn parser.py: {}", e)))?;

        // Send the request JSON
        let request = serde_json::json!({ "file_path": file_path.to_string_lossy() });
        let request_line = serde_json::to_string(&request).unwrap() + "\n";

        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(request_line.as_bytes())
                .map_err(|e| AppError::Other(format!("Failed to write to parser stdin: {}", e)))?;
        }

        // Read one response line
        let stdout = child.stdout.take().expect("stdout should be piped");
        let mut reader = BufReader::new(stdout);
        let mut response_line = String::new();
        reader
            .read_line(&mut response_line)
            .map_err(|e| AppError::Other(format!("Failed to read parser stdout: {}", e)))?;

        let _ = child.wait();

        if response_line.trim().is_empty() {
            return Err(AppError::Other(
                "parser.py returned no output — check Python environment".into(),
            ));
        }

        let response: PyParseResponse =
            serde_json::from_str(response_line.trim()).map_err(|e| {
                AppError::Other(format!(
                    "Failed to deserialise parser.py response: {} — raw: {}",
                    e,
                    &response_line[..response_line.len().min(200)]
                ))
            })?;

        if response.status != "success" {
            return Err(AppError::Other(format!(
                "parser.py error: {} {:?}",
                response.error.unwrap_or_default(),
                response.traceback
            )));
        }

        // Prefer structured blocks; fall back to wrapping flat content in one block.
        let blocks = match response.blocks {
            Some(py_blocks) if !py_blocks.is_empty() => py_blocks
                .into_iter()
                .map(|b| ParsedBlock {
                    heading_path: b.heading_path,
                    page_number: b.page_number,
                    text: b.text,
                    ocr_confidence: b.ocr_confidence,
                })
                .collect(),
            _ => {
                // Fallback: wrap the whole content in a single block with no heading.
                let text = response.content.unwrap_or_default();
                if text.trim().is_empty() {
                    vec![]
                } else {
                    vec![ParsedBlock {
                        heading_path: vec![],
                        page_number: Some(1),
                        text,
                        ocr_confidence: None,
                    }]
                }
            }
        };

        tracing::info!(
            file = %file_path.display(),
            block_count = blocks.len(),
            "Document parsed into structured blocks"
        );

        Ok(blocks)
    }
}
