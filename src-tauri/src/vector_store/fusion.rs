use crate::vector_store::types::FusedCandidate;
use std::collections::HashMap;

/// Reciprocal Rank Fusion (RRF) merges dense and sparse rankings.
/// Input lists must be pre-sorted from most relevant (rank 1) to least relevant.
pub fn reciprocal_rank_fusion(
    dense: Vec<(String, f32)>,
    sparse: Vec<(String, f32)>,
    k: f32,
) -> Vec<FusedCandidate> {
    let mut scores: HashMap<String, (Option<usize>, Option<usize>, f32)> = HashMap::new();

    // Process dense list (rank starts at 1)
    for (rank_idx, (id, _)) in dense.into_iter().enumerate() {
        let rank = rank_idx + 1;
        let entry = scores.entry(id).or_insert((None, None, 0.0));
        entry.0 = Some(rank);
        entry.2 += 1.0 / (k + rank as f32);
    }

    // Process sparse list (rank starts at 1)
    for (rank_idx, (id, _)) in sparse.into_iter().enumerate() {
        let rank = rank_idx + 1;
        let entry = scores.entry(id).or_insert((None, None, 0.0));
        entry.1 = Some(rank);
        entry.2 += 1.0 / (k + rank as f32);
    }

    // Convert HashMap to Vec of FusedCandidates
    let mut candidates: Vec<FusedCandidate> = scores
        .into_iter()
        .map(
            |(chunk_id, (dense_rank, sparse_rank, fused_score))| FusedCandidate {
                chunk_id,
                dense_rank,
                sparse_rank,
                fused_score,
            },
        )
        .collect();

    // Sort by fused score descending
    candidates.sort_by(|a, b| {
        b.fused_score
            .partial_cmp(&a.fused_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    candidates
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reciprocal_rank_fusion() {
        // Doc A: dense rank 1, sparse rank 2
        // Doc B: dense rank 2, sparse rank 1
        // Doc C: dense rank 3, not in sparse
        // Doc D: not in dense, sparse rank 3
        let dense = vec![
            ("A".to_string(), 0.9),
            ("B".to_string(), 0.8),
            ("C".to_string(), 0.7),
        ];
        let sparse = vec![
            ("B".to_string(), 5.0),
            ("A".to_string(), 4.0),
            ("D".to_string(), 3.0),
        ];

        // Using k = 60.0
        // Score A = 1 / (60 + 1) + 1 / (60 + 2) = 1/61 + 1/62 = 0.01639 + 0.01613 = 0.03252
        // Score B = 1 / (60 + 2) + 1 / (60 + 1) = 1/62 + 1/61 = 0.03252
        // Score C = 1 / (60 + 3) = 1/63 = 0.01587
        // Score D = 1 / (60 + 3) = 1/63 = 0.01587

        let fused = reciprocal_rank_fusion(dense, sparse, 60.0);
        assert_eq!(fused.len(), 4);

        // A and B should be top (tied score, order depends on sort stability but they are the highest)
        assert!(fused[0].chunk_id == "A" || fused[0].chunk_id == "B");
        assert!(fused[1].chunk_id == "A" || fused[1].chunk_id == "B");

        // C and D should be next
        assert!(fused[2].chunk_id == "C" || fused[2].chunk_id == "D");
        assert!(fused[3].chunk_id == "C" || fused[3].chunk_id == "D");
    }
}
