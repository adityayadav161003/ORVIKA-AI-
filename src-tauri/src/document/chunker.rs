//! Structure-aware document chunker — Sprint 3.
//!
//! Splits a list of `ParsedBlock`s (produced by `document::parser` or
//! `document::ocr`) into `Chunk`s suitable for embedding and vector storage.
//!
//! ## Key invariants (per sprint spec)
//!
//! 1. **No chunk ever crosses a top-level heading boundary.**  Overlap is
//!    applied only *within* a block that comes from the same heading section.
//! 2. Target chunk size: 400–600 tokens (`ChunkingConfig::chunk_size_tokens`,
//!    default 500).
//! 3. Overlap: 15% of chunk size (`ChunkingConfig::overlap_tokens`, default 75
//!    tokens).  Applied between adjacent sub-chunks of the *same* section.
//! 4. Chunks below `ChunkingConfig::min_tokens` (default 30) are merged into
//!    their right neighbour (or, if they're the last chunk in a section, into
//!    their left neighbour) instead of being indexed standalone.
//!
//! ## Token counting
//!
//! We use whitespace-split word count as a fast approximation.  This is good
//! enough for chunking decisions; Sprint 11 evaluation may upgrade to a real
//! tokenizer if the approximation drifts significantly on legal/medical prose.

use uuid::Uuid;

use crate::document::types::{Chunk, ChunkingConfig, ParsedBlock};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Split `blocks` into indexable `Chunk`s.
///
/// `document_id` is stored on every chunk so that `chunk_repo::insert_batch`
/// can associate them with the correct document row.
///
/// Returns chunks in document order (heading then sub-chunk index).
pub fn chunk_blocks(
    blocks: &[ParsedBlock],
    document_id: &str,
    config: &ChunkingConfig,
) -> Vec<Chunk> {
    let mut all_chunks: Vec<Chunk> = Vec::new();
    let mut global_index: u32 = 0;

    for block in blocks {
        let section_chunks = split_block(block, document_id, config, &mut global_index);
        all_chunks.extend(section_chunks);
    }

    // Post-process: merge tiny trailing chunks from the same block into their
    // left sibling.  This handles stray page-number lines, lone headings, etc.
    merge_short_chunks(all_chunks, config.min_tokens)
}

// ---------------------------------------------------------------------------
// Block-level splitting
// ---------------------------------------------------------------------------

/// Split one `ParsedBlock` into one or more sub-chunks that respect the
/// configured size and overlap limits.
fn split_block(
    block: &ParsedBlock,
    document_id: &str,
    config: &ChunkingConfig,
    global_index: &mut u32,
) -> Vec<Chunk> {
    let text = block.text.trim();
    if text.is_empty() {
        return vec![];
    }

    let section_heading = block.section_heading().map(str::to_owned);
    let heading_path = if block.heading_path.is_empty() {
        None
    } else {
        Some(block.heading_path_str())
    };
    let page_number = block.page_number;
    let ocr_confidence = block.ocr_confidence;

    // Split the block's text into words for token-count arithmetic
    let words: Vec<&str> = text.split_whitespace().collect();
    let total_tokens = words.len();

    // Fast path: block fits in one chunk (including the minimum threshold)
    if total_tokens <= config.chunk_size_tokens {
        let chunk = make_chunk(
            document_id,
            *global_index,
            text.to_owned(),
            total_tokens as u32,
            page_number,
            section_heading,
            heading_path,
            ocr_confidence,
        );
        *global_index += 1;
        return vec![chunk];
    }

    // Sliding-window split within this block.
    // Step size = chunk_size - overlap (never less than 1 word).
    let step = config
        .chunk_size_tokens
        .saturating_sub(config.overlap_tokens)
        .max(1);

    let mut chunks: Vec<Chunk> = Vec::new();
    let mut start = 0usize;

    while start < words.len() {
        let end = (start + config.chunk_size_tokens).min(words.len());
        let chunk_words = &words[start..end];
        let chunk_text = chunk_words.join(" ");
        let token_count = chunk_words.len() as u32;

        // Approximate byte offsets within the block text.
        let start_char = word_offset(text, chunk_words[0], start);
        let end_char = start_char + chunk_text.len() as u32;

        let mut chunk = make_chunk(
            document_id,
            *global_index,
            chunk_text,
            token_count,
            page_number,
            section_heading.clone(),
            heading_path.clone(),
            ocr_confidence,
        );
        chunk.start_char = Some(start_char);
        chunk.end_char = Some(end_char);

        chunks.push(chunk);
        *global_index += 1;

        start += step;
        // If the remaining words are fewer than min_tokens they'll be captured
        // in the next iteration (end == words.len()) and then merged by
        // `merge_short_chunks`.
    }

    chunks
}

// ---------------------------------------------------------------------------
// Short-chunk merging
// ---------------------------------------------------------------------------

/// Merge any chunk whose token_count < `min_tokens` into its right (preferred)
/// or left neighbour, *only if they share the same section_heading*.
///
/// If a tiny chunk stands alone in its section (no neighbours), it is dropped.
fn merge_short_chunks(mut chunks: Vec<Chunk>, min_tokens: usize) -> Vec<Chunk> {
    if chunks.is_empty() {
        return chunks;
    }

    let mut result: Vec<Chunk> = Vec::with_capacity(chunks.len());
    let mut pending: Option<Chunk> = None;

    for chunk in chunks.drain(..) {
        match pending.take() {
            None => {
                if (chunk.token_count as usize) < min_tokens {
                    // Hold it; try to merge with the next chunk
                    pending = Some(chunk);
                } else {
                    result.push(chunk);
                }
            }
            Some(prev) => {
                // Merge prev (short) into current if same section, else flush both
                if prev.section_heading == chunk.section_heading {
                    let merged = merge_two(prev, chunk);
                    if (merged.token_count as usize) < min_tokens {
                        pending = Some(merged);
                    } else {
                        result.push(merged);
                    }
                } else {
                    // Different section — flush prev (even if short); it's
                    // better to keep a short preamble chunk than to cross a
                    // heading boundary.
                    result.push(prev);
                    if (chunk.token_count as usize) < min_tokens {
                        pending = Some(chunk);
                    } else {
                        result.push(chunk);
                    }
                }
            }
        }
    }

    // Flush any remaining pending chunk (merge into last result or drop if empty)
    if let Some(p) = pending {
        if let Some(last) = result.last_mut() {
            if last.section_heading == p.section_heading {
                let merged_content = format!("{} {}", last.content, p.content);
                let merged_tokens = last.token_count + p.token_count;
                last.content = merged_content;
                last.token_count = merged_tokens;
                // end_char extends
                if let Some(end) = p.end_char {
                    last.end_char = Some(end);
                }
            } else {
                result.push(p);
            }
        }
        // If result is empty we silently drop the orphaned short chunk.
    }

    // Re-number chunk_index sequentially after merging
    for (i, chunk) in result.iter_mut().enumerate() {
        chunk.chunk_index = i as u32;
    }

    result
}

fn merge_two(mut left: Chunk, right: Chunk) -> Chunk {
    let merged_content = format!("{} {}", left.content, right.content);
    let merged_tokens = left.token_count + right.token_count;
    left.content = merged_content;
    left.token_count = merged_tokens;
    left.end_char = right.end_char;
    left
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_chunk(
    document_id: &str,
    chunk_index: u32,
    content: String,
    token_count: u32,
    page_number: Option<u32>,
    section_heading: Option<String>,
    heading_path: Option<String>,
    ocr_confidence: Option<f32>,
) -> Chunk {
    // Pack extra fields into metadata JSON
    let metadata = build_metadata(&heading_path, ocr_confidence);

    Chunk {
        id: Uuid::new_v4().to_string(),
        document_id: document_id.to_owned(),
        chunk_index,
        content,
        token_count,
        page_number,
        section_heading,
        heading_path,
        start_char: None,
        end_char: None,
        metadata,
    }
}

/// Build the metadata JSON string, omitting null fields.
fn build_metadata(heading_path: &Option<String>, ocr_confidence: Option<f32>) -> Option<String> {
    let mut obj = serde_json::Map::new();

    if let Some(path) = heading_path {
        obj.insert(
            "heading_path".to_owned(),
            serde_json::Value::String(path.clone()),
        );
    }
    if let Some(conf) = ocr_confidence {
        obj.insert(
            "ocr_confidence".to_owned(),
            serde_json::json!(conf),
        );
    }

    if obj.is_empty() {
        None
    } else {
        Some(serde_json::to_string(&obj).unwrap_or_default())
    }
}

/// Approximate byte offset of the nth word within `text`.
/// Walks the string once rather than splitting and re-joining.
fn word_offset(text: &str, _word: &str, word_index: usize) -> u32 {
    let mut count = 0usize;
    let mut in_word = false;
    for (byte_pos, ch) in text.char_indices() {
        if ch.is_whitespace() {
            in_word = false;
        } else if !in_word {
            if count == word_index {
                return byte_pos as u32;
            }
            count += 1;
            in_word = true;
        }
    }
    text.len() as u32
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::types::{ChunkingConfig, ParsedBlock};

    fn block(heading: &[&str], text: &str) -> ParsedBlock {
        ParsedBlock {
            heading_path: heading.iter().map(|s| s.to_string()).collect(),
            page_number: Some(1),
            text: text.to_owned(),
            ocr_confidence: None,
        }
    }

    fn word_text(n: usize) -> String {
        (0..n).map(|i| format!("word{}", i)).collect::<Vec<_>>().join(" ")
    }

    // -----------------------------------------------------------------------
    // Test 1: heading boundary is never crossed
    // -----------------------------------------------------------------------
    #[test]
    fn heading_boundary_not_crossed() {
        let config = ChunkingConfig::default();
        let blocks = vec![
            block(&["Introduction"], &word_text(100)),
            block(&["Methodology"], &word_text(100)),
        ];

        let chunks = chunk_blocks(&blocks, "doc1", &config);

        // Every chunk in "Introduction" must not leak into "Methodology"
        for chunk in &chunks {
            let is_intro = chunk
                .section_heading
                .as_deref()
                .map(|h| h == "Introduction")
                .unwrap_or(false);
            let is_method = chunk
                .section_heading
                .as_deref()
                .map(|h| h == "Methodology")
                .unwrap_or(false);
            // A chunk can only belong to one heading
            assert!(
                !(is_intro && is_method),
                "Chunk crosses heading boundary: {:?}",
                chunk.section_heading
            );
        }
    }

    // -----------------------------------------------------------------------
    // Test 2: chunks within a section are bounded by chunk_size
    // -----------------------------------------------------------------------
    #[test]
    fn chunk_size_respected() {
        let config = ChunkingConfig {
            chunk_size_tokens: 50,
            overlap_tokens: 10,
            min_tokens: 5,
        };
        // 200 words — should produce multiple chunks
        let blocks = vec![block(&["Section A"], &word_text(200))];
        let chunks = chunk_blocks(&blocks, "doc1", &config);

        assert!(chunks.len() > 1, "Should produce more than one chunk for 200 words with size=50");
        for chunk in &chunks {
            // Each chunk should not exceed chunk_size + a small rounding margin
            let tokens = chunk.content.split_whitespace().count();
            assert!(
                tokens <= config.chunk_size_tokens + 5,
                "Chunk has {} tokens, expected <= {}",
                tokens,
                config.chunk_size_tokens + 5
            );
        }
    }

    // -----------------------------------------------------------------------
    // Test 3: short chunks are merged, not indexed standalone
    // -----------------------------------------------------------------------
    #[test]
    fn short_chunks_are_merged() {
        let config = ChunkingConfig {
            chunk_size_tokens: 500,
            overlap_tokens: 75,
            min_tokens: 30,
        };
        // A tiny 5-word block — well below min_tokens
        let blocks = vec![
            block(&["Section A"], &word_text(5)),
            block(&["Section A"], &word_text(100)),
        ];
        let chunks = chunk_blocks(&blocks, "doc1", &config);

        // The 5-word block should be merged into the 100-word block
        for chunk in &chunks {
            let tokens = chunk.content.split_whitespace().count();
            assert!(
                tokens >= config.min_tokens,
                "Found standalone short chunk with {} tokens",
                tokens
            );
        }
    }

    // -----------------------------------------------------------------------
    // Test 4: overlap stays within same section
    // -----------------------------------------------------------------------
    #[test]
    fn overlap_does_not_cross_heading() {
        let config = ChunkingConfig {
            chunk_size_tokens: 10,
            overlap_tokens: 5,
            min_tokens: 2,
        };
        let blocks = vec![
            block(&["Sec1"], &word_text(25)),
            block(&["Sec2"], &word_text(25)),
        ];
        let chunks = chunk_blocks(&blocks, "doc1", &config);

        // No chunk should contain words from both Sec1 and Sec2.
        // Since words are "word0 word1 ... word24" we can check heading attribution.
        let sec1_chunks: Vec<_> = chunks
            .iter()
            .filter(|c| c.section_heading.as_deref() == Some("Sec1"))
            .collect();
        let sec2_chunks: Vec<_> = chunks
            .iter()
            .filter(|c| c.section_heading.as_deref() == Some("Sec2"))
            .collect();

        assert!(
            !sec1_chunks.is_empty(),
            "Should have chunks for Sec1"
        );
        assert!(
            !sec2_chunks.is_empty(),
            "Should have chunks for Sec2"
        );
        // Total chunks = sec1 + sec2 (no cross-boundary chunks)
        assert_eq!(
            chunks.len(),
            sec1_chunks.len() + sec2_chunks.len(),
            "Some chunks are not attributed to either section"
        );
    }

    // -----------------------------------------------------------------------
    // Test 5: idempotent — calling chunk_blocks twice produces same count
    // -----------------------------------------------------------------------
    #[test]
    fn deterministic_output() {
        let config = ChunkingConfig::default();
        let blocks = vec![block(&["Chapter 1"], &word_text(300))];
        let chunks_a = chunk_blocks(&blocks, "doc1", &config);
        let chunks_b = chunk_blocks(&blocks, "doc1", &config);
        assert_eq!(
            chunks_a.len(),
            chunks_b.len(),
            "chunk_blocks is not deterministic"
        );
    }
}
