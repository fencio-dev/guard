// Comparison logic for semantic vectors
// Week 1: Dummy implementation
// Week 2: Real implementation with dot products and threshold logic

use crate::{VectorEnvelope, ComparisonResult};

/// Compute dot product of two slices (used for cosine similarity on normalized vectors)
#[inline]
fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

/// Compute cosine similarity between two vectors
#[inline]
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a < 1e-8 || norm_b < 1e-8 {
        0.0
    } else {
        let sim = dot / (norm_a * norm_b);
        sim.min(1.0).max(-1.0)  // Clamp to [-1, 1]
    }
}

/// Compute maximum cosine similarity between intent slice and anchor set
#[inline]
fn max_anchor_similarity(intent_slice: &[f32], anchors: &[[f32; 32]], count: usize) -> f32 {
    if count == 0 {
        // No anchors = wildcard (always pass)
        return 1.0;
    }

    anchors[..count]
        .iter()
        .map(|anchor| cosine_similarity(intent_slice, anchor))
        .fold(0.0f32, f32::max)
}

/// Compare two 128-dim vectors using slice-based logic
///
/// Computes per-slice cosine similarity (via dot product on normalized vectors)
/// and applies threshold-based decision logic.
pub fn compare(envelope: &VectorEnvelope) -> ComparisonResult {
    compare_real(envelope)
}

/// Real implementation with slice-based comparison
fn compare_real(envelope: &VectorEnvelope) -> ComparisonResult {
    let mut slice_similarities = [0.0f32; 4];

    // Extract intent slices
    let intent_action = &envelope.intent[0..32];
    let intent_resource = &envelope.intent[32..64];
    let intent_data = &envelope.intent[64..96];
    let intent_risk = &envelope.intent[96..128];

    // Compute max-of-anchors similarity per slot
    slice_similarities[0] = max_anchor_similarity(
        intent_action,
        &envelope.action_anchors,
        envelope.action_anchor_count,
    );
    slice_similarities[1] = max_anchor_similarity(
        intent_resource,
        &envelope.resource_anchors,
        envelope.resource_anchor_count,
    );
    slice_similarities[2] = max_anchor_similarity(
        intent_data,
        &envelope.data_anchors,
        envelope.data_anchor_count,
    );
    slice_similarities[3] = max_anchor_similarity(
        intent_risk,
        &envelope.risk_anchors,
        envelope.risk_anchor_count,
    );

    // Decision logic based on mode
    let decision = if envelope.decision_mode == 0 {
        // Mode 0: min (mandatory boundaries)
        // All slices must meet their thresholds
        let all_pass = slice_similarities
            .iter()
            .zip(envelope.thresholds.iter())
            .all(|(sim, thresh)| sim >= thresh);

        if all_pass { 1 } else { 0 }
    } else {
        // Mode 1: weighted-avg (optional boundaries)
        // Compute weighted average and compare to global threshold
        let weighted_sum: f32 = slice_similarities
            .iter()
            .zip(envelope.weights.iter())
            .map(|(sim, weight)| sim * weight)
            .sum();

        let total_weight: f32 = envelope.weights.iter().sum();
        let weighted_avg = if total_weight > 0.0 {
            weighted_sum / total_weight
        } else {
            0.0
        };

        if weighted_avg >= envelope.global_threshold { 1 } else { 0 }
    };

    ComparisonResult {
        decision,
        slice_similarities,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dummy_compare() {
        let envelope = VectorEnvelope {
            intent: [0.9f32; 128],
            action_anchors: [[1.0; 32]; 16],
            action_anchor_count: 1,
            resource_anchors: [[1.0; 32]; 16],
            resource_anchor_count: 1,
            data_anchors: [[1.0; 32]; 16],
            data_anchor_count: 1,
            risk_anchors: [[1.0; 32]; 16],
            risk_anchor_count: 1,
            thresholds: [0.85, 0.85, 0.85, 0.85],
            weights: [1.0, 1.0, 1.0, 1.0],
            decision_mode: 0,
            global_threshold: 0.85,
        };

        let result = compare(&envelope);

        // Week 1: Just verify structure is correct
        assert!(result.decision == 0 || result.decision == 1);
        assert_eq!(result.slice_similarities.len(), 4);
    }

    #[test]
    fn test_dot_product() {
        let a = [1.0, 2.0, 3.0];
        let b = [4.0, 5.0, 6.0];
        let result = dot_product(&a, &b);
        assert_eq!(result, 32.0);  // 1*4 + 2*5 + 3*6 = 32
    }

    #[test]
    fn test_min_mode_all_pass() {
        let envelope = VectorEnvelope {
            intent: [0.9f32; 128],
            action_anchors: [[1.0; 32]; 16],
            action_anchor_count: 1,
            resource_anchors: [[1.0; 32]; 16],
            resource_anchor_count: 1,
            data_anchors: [[1.0; 32]; 16],
            data_anchor_count: 1,
            risk_anchors: [[1.0; 32]; 16],
            risk_anchor_count: 1,
            thresholds: [0.85, 0.85, 0.85, 0.85],
            weights: [1.0, 1.0, 1.0, 1.0],
            decision_mode: 0,
            global_threshold: 0.85,
        };

        let result = compare_real(&envelope);
        assert_eq!(result.decision, 1);  // Should allow
    }

    #[test]
    fn test_min_mode_one_fail() {
        // Create intent with action slice that will produce low similarity
        let mut intent = [1.0f32; 128];
        intent[0..32].fill(-1.0);  // Action slice opposite direction

        let mut action_anchors = [[0.0f32; 32]; 16];
        action_anchors[0].fill(1.0);  // Opposite direction from intent

        let envelope = VectorEnvelope {
            intent,
            action_anchors,
            action_anchor_count: 1,
            resource_anchors: [[1.0; 32]; 16],
            resource_anchor_count: 1,
            data_anchors: [[1.0; 32]; 16],
            data_anchor_count: 1,
            risk_anchors: [[1.0; 32]; 16],
            risk_anchor_count: 1,
            thresholds: [0.85, 0.85, 0.85, 0.85],
            weights: [1.0, 1.0, 1.0, 1.0],
            decision_mode: 0,
            global_threshold: 0.85,
        };

        let result = compare_real(&envelope);
        // Note: For this test to work properly, vectors should be normalized
        // For now, we expect low similarity on action slice
        assert_eq!(result.decision, 0);  // Should block
    }

    #[test]
    fn test_weighted_avg_mode() {
        let envelope = VectorEnvelope {
            intent: [0.8f32; 128],
            action_anchors: [[1.0; 32]; 16],
            action_anchor_count: 1,
            resource_anchors: [[1.0; 32]; 16],
            resource_anchor_count: 1,
            data_anchors: [[1.0; 32]; 16],
            data_anchor_count: 1,
            risk_anchors: [[1.0; 32]; 16],
            risk_anchor_count: 1,
            thresholds: [0.0, 0.0, 0.0, 0.0],  // Not used in weighted-avg
            weights: [1.0, 1.0, 1.0, 1.0],
            decision_mode: 1,
            global_threshold: 0.75,
        };

        let result = compare_real(&envelope);
        assert_eq!(result.decision, 1);  // 0.8 >= 0.75, should allow
    }

    // Phase 1 tests: Per-slice cosine similarity

    #[test]
    fn test_identical_vectors_cosine_one() {
        // Create non-normalized identical vectors
        let intent = [0.5f32; 128];

        let envelope = VectorEnvelope {
            intent,
            action_anchors: [[0.5; 32]; 16],
            action_anchor_count: 1,
            resource_anchors: [[0.5; 32]; 16],
            resource_anchor_count: 1,
            data_anchors: [[0.5; 32]; 16],
            data_anchor_count: 1,
            risk_anchors: [[0.5; 32]; 16],
            risk_anchor_count: 1,
            thresholds: [0.8, 0.8, 0.8, 0.8],
            weights: [1.0, 1.0, 1.0, 1.0],
            decision_mode: 0,
            global_threshold: 0.8,
        };

        let result = compare(&envelope);

        for (i, sim) in result.slice_similarities.iter().enumerate() {
            assert!(
                (*sim - 1.0).abs() < 0.01,
                "Slice {} expected cosine ~1.0, got {}",
                i, sim
            );
        }
    }

    #[test]
    fn test_orthogonal_vectors_cosine_zero() {
        let mut intent = [0.0f32; 128];
        intent[0..16].fill(1.0);  // First half of action slice

        // Make action anchor orthogonal (different half of dimensions)
        let mut action_anchors = [[0.0f32; 32]; 16];
        action_anchors[0][16..32].fill(1.0);  // Second half of action slice

        let envelope = VectorEnvelope {
            intent,
            action_anchors,
            action_anchor_count: 1,
            resource_anchors: [[0.0; 32]; 16],
            resource_anchor_count: 0,  // Wildcards for other slots
            data_anchors: [[0.0; 32]; 16],
            data_anchor_count: 0,
            risk_anchors: [[0.0; 32]; 16],
            risk_anchor_count: 0,
            thresholds: [0.0, 0.0, 0.0, 0.0],
            weights: [1.0, 1.0, 1.0, 1.0],
            decision_mode: 0,
            global_threshold: 0.0,
        };

        let result = compare(&envelope);

        // First slice should be ~0 (orthogonal - no overlap in dimensions)
        assert!(result.slice_similarities[0].abs() < 0.05,
                "Expected ~0, got {}", result.slice_similarities[0]);
    }

    #[test]
    fn test_zero_norm_guard() {
        let intent = [0.0f32; 128];

        let envelope = VectorEnvelope {
            intent,
            action_anchors: [[1.0; 32]; 16],
            action_anchor_count: 1,
            resource_anchors: [[0.0; 32]; 16],
            resource_anchor_count: 0,
            data_anchors: [[0.0; 32]; 16],
            data_anchor_count: 0,
            risk_anchors: [[0.0; 32]; 16],
            risk_anchor_count: 0,
            thresholds: [0.8, 0.8, 0.8, 0.8],
            weights: [1.0, 1.0, 1.0, 1.0],
            decision_mode: 0,
            global_threshold: 0.8,
        };

        let result = compare(&envelope);

        // Should return 0.0 for action (zero-norm intent), not NaN or panic
        assert!(!result.slice_similarities[0].is_nan());
        assert_eq!(result.slice_similarities[0], 0.0);
    }

    // Phase 2 tests: Max-of-anchors containment

    #[test]
    fn test_max_anchor_similarity_containment() {
        // Intent: action = "read"
        let mut intent = [0.0f32; 128];
        intent[0] = 1.0;  // Simplified: use first dim

        // Anchors: ["read", "write", "delete"]
        let mut read_anchor = [0.0f32; 32];
        read_anchor[0] = 1.0;  // Same as intent

        let mut write_anchor = [0.0f32; 32];
        write_anchor[1] = 1.0;  // Different dim

        let mut delete_anchor = [0.0f32; 32];
        delete_anchor[2] = 1.0;  // Different dim

        let mut action_anchors = [[0.0f32; 32]; 16];
        action_anchors[0] = read_anchor;
        action_anchors[1] = write_anchor;
        action_anchors[2] = delete_anchor;

        let envelope = VectorEnvelope {
            intent,
            action_anchors,
            action_anchor_count: 3,
            resource_anchors: [[0.0; 32]; 16],
            resource_anchor_count: 0,
            data_anchors: [[0.0; 32]; 16],
            data_anchor_count: 0,
            risk_anchors: [[0.0; 32]; 16],
            risk_anchor_count: 0,
            thresholds: [0.9, 0.0, 0.0, 0.0],
            weights: [1.0, 1.0, 1.0, 1.0],
            decision_mode: 0,
            global_threshold: 0.8,
        };

        let result = compare(&envelope);

        // Should match "read" anchor with sim ~1.0
        assert!(result.slice_similarities[0] > 0.99,
                "Expected action sim ~1.0, got {}", result.slice_similarities[0]);
        assert_eq!(result.decision, 1);  // Should allow
    }

    #[test]
    fn test_max_anchor_similarity_no_match() {
        // Intent: action = "export" (dimension 3)
        let mut intent = [0.0f32; 128];
        intent[3] = 1.0;

        // Anchors: ["read", "write", "delete"] (no "export")
        let mut read_anchor = [0.0f32; 32];
        read_anchor[0] = 1.0;

        let mut write_anchor = [0.0f32; 32];
        write_anchor[1] = 1.0;

        let mut delete_anchor = [0.0f32; 32];
        delete_anchor[2] = 1.0;

        let mut action_anchors = [[0.0f32; 32]; 16];
        action_anchors[0] = read_anchor;
        action_anchors[1] = write_anchor;
        action_anchors[2] = delete_anchor;

        let envelope = VectorEnvelope {
            intent,
            action_anchors,
            action_anchor_count: 3,
            resource_anchors: [[0.0; 32]; 16],
            resource_anchor_count: 0,
            data_anchors: [[0.0; 32]; 16],
            data_anchor_count: 0,
            risk_anchors: [[0.0; 32]; 16],
            risk_anchor_count: 0,
            thresholds: [0.8, 0.0, 0.0, 0.0],
            weights: [1.0, 1.0, 1.0, 1.0],
            decision_mode: 0,
            global_threshold: 0.8,
        };

        let result = compare(&envelope);

        // Should have low similarity (orthogonal)
        assert!(result.slice_similarities[0] < 0.1,
                "Expected low sim, got {}", result.slice_similarities[0]);
        assert_eq!(result.decision, 0);  // Should block
    }

    #[test]
    fn test_empty_anchor_set_wildcard() {
        let intent = [1.0f32; 128];

        let envelope = VectorEnvelope {
            intent,
            action_anchors: [[0.0; 32]; 16],
            action_anchor_count: 0,  // Empty = wildcard
            resource_anchors: [[0.0; 32]; 16],
            resource_anchor_count: 0,
            data_anchors: [[0.0; 32]; 16],
            data_anchor_count: 0,
            risk_anchors: [[0.0; 32]; 16],
            risk_anchor_count: 0,
            thresholds: [0.8, 0.8, 0.8, 0.8],
            weights: [1.0, 1.0, 1.0, 1.0],
            decision_mode: 0,
            global_threshold: 0.8,
        };

        let result = compare(&envelope);

        // Empty anchor set = wildcard = always pass with sim 1.0
        assert_eq!(result.slice_similarities[0], 1.0);
        assert_eq!(result.decision, 1);  // Should allow
    }
}
