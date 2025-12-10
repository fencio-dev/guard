# Semantic Similarity Fix: Math Correction + Anchor-Based Encoding

**Date**: 2025-11-13
**Status**: Design Complete, Ready for Implementation
**Scope**: Fix low similarity scores (0.16-0.29) causing all intents to block

---

## Executive Summary

Current system shows critically low similarity scores (0.16-0.29) when comparing intents against boundaries, causing all operations to block despite valid matches. Root cause analysis identified two interconnected issues:

1. **Math Bug**: Per-slice cosine calculation in Rust computes raw dot product on globally-normalized vectors, returning energy shares (~0.25) instead of true cosine similarity (~0.9+)
2. **Encoding Mismatch**: Boundary constraint lists (`"actor_type: user, service, llm, agent"`) vs intent singletons (`"actor_type: agent"`) create semantic misalignment in sentence-transformer embeddings

**Solution**: Two-phase sequential fix:
- **Phase 1**: Correct per-slice cosine math in Rust + per-slot normalization in Python
- **Phase 2**: Implement max-of-anchors encoding for boundaries with FFI extension

**Expected Outcome**: Similarity scores improve from 0.16-0.29 to 0.90-0.95 for valid matches.

---

## Problem Analysis

### Current Behavior (Session 12)

**Observed Similarities**:
```
Action:   0.16 (threshold: 0.80) ❌
Resource: 0.17 (threshold: 0.75) ❌
Data:     0.24 (threshold: 0.80) ❌
Risk:     0.20 (threshold: 0.80) ❌
```

**Expected Similarities** (from Session 7 v1.1 tests):
```
Action:   96-100%
Resource: 85-90%
Data:     95-100%
Risk:     95-100%
```

### Root Cause 1: Math Bug

**Location**: `semantic-sandbox/src/compare.rs:33-37`

**Current Code**:
```rust
for (i, (start, end)) in SLICE_RANGES.iter().enumerate() {
    let intent_slice = &envelope.intent[*start..*end];
    let boundary_slice = &envelope.boundary[*start..*end];
    slice_similarities[i] = dot_product(intent_slice, boundary_slice);  // ❌ BUG
}
```

**Problem**:
- Both 128-d vectors are L2-normalized globally in Python: `||v_128|| = 1`
- Per-slice dot product returns: `dot(slice_i, slice_j) = ||slice_i|| × ||slice_j|| × cos(θ)`
- With equal energy distribution: `||slice_i||² ≈ 1/4`
- Therefore: `dot(slice_i, slice_j) ≈ 0.25 × cos(θ)` instead of `cos(θ)`

**Mathematical Validation**:
```
For identical vectors with global norm = 1:
- Each slice norm: ||slice_i|| ≈ 0.5 (since ||slice_i||² ≈ 1/4)
- Dot product: dot(slice_i, slice_i) = ||slice_i||² ≈ 0.25 ✓ (matches observed 0.16-0.29)
- True cosine: cos(θ) = dot / (||slice_i|| × ||slice_i||) = 1.0
```

**Location**: `management-plane/app/encoding.py:439-443, :485-489`

**Current Code**:
```python
# Concatenate to 128-dim
vector_128 = np.concatenate(slot_vectors)

# L2 normalize (GLOBAL NORMALIZATION - causes bug)
norm = np.linalg.norm(vector_128)
if norm > 0:
    vector_128 = vector_128 / norm  # ❌ Scales all slices together
```

**Problem**: Global normalization reduces per-slice magnitudes non-uniformly, breaking per-slice cosine calculations.

---

### Root Cause 2: Encoding Mismatch

**Location**: `management-plane/app/encoding.py:260-347`

**Current Boundary Encoding**:
```python
def build_boundary_action_slot(boundary: DesignBoundary) -> str:
    actions_str = ", ".join(sorted(boundary.constraints.action.actions))
    actor_types_str = ", ".join(sorted(boundary.constraints.action.actor_types))
    return f"action: {actions_str} | actor_type: {actor_types_str}"
    # Returns: "action: read, write, delete | actor_type: user, service, llm, agent"
```

**Current Intent Encoding**:
```python
def build_action_slot(event: IntentEvent) -> str:
    return f"action: {event.action} | actor_type: {event.actor.type}"
    # Returns: "action: read | actor_type: agent"
```

**Problem**:
- Sentence-transformers struggle with "mixture vs atom" semantics
- `ST("action: read, write, delete")` creates a blended embedding
- `ST("action: read")` creates a specific embedding
- Cosine similarity between blended and specific is low even when "read" ∈ ["read", "write", "delete"]

**Semantic Issue**: We need **containment logic** (is intent value in allowed set?) but current encoding only measures **similarity** (how close are two strings?).

---

## Solution Design

### Approach: Sequential Two-Phase Fix

**Rationale**:
- Phase 1 (math fix) is isolated and low-risk - validates our diagnosis
- Phase 2 (encoding) builds on proven Phase 1 foundation
- Sequential approach enables faster debugging if issues arise

---

## Phase 1: Math Fix

**Goal**: Fix per-slice cosine calculation to return true cosine similarity (0.9+ for matches).

### Changes

#### 1. Rust: Compute True Per-Slice Cosine

**File**: `semantic-sandbox/src/compare.rs`

**Before** (lines 33-37):
```rust
for (i, (start, end)) in SLICE_RANGES.iter().enumerate() {
    let intent_slice = &envelope.intent[*start..*end];
    let boundary_slice = &envelope.boundary[*start..*end];
    slice_similarities[i] = dot_product(intent_slice, boundary_slice);
}
```

**After**:
```rust
for (i, (start, end)) in SLICE_RANGES.iter().enumerate() {
    let intent_slice = &envelope.intent[*start..*end];
    let boundary_slice = &envelope.boundary[*start..*end];

    let dot = dot_product(intent_slice, boundary_slice);
    let norm_i = intent_slice.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b = boundary_slice.iter().map(|x| x * x).sum::<f32>().sqrt();

    slice_similarities[i] = if norm_i < 1e-8 || norm_b < 1e-8 {
        0.0  // Zero-norm guard
    } else {
        let sim = dot / (norm_i * norm_b);
        sim.min(1.0).max(-1.0)  // Clamp to [-1, 1] to guard against FP errors
    };
}
```

**Rationale**:
- Computes true cosine: `cos(θ) = dot(u, v) / (||u|| × ||v||)`
- Zero-norm guard prevents division by zero
- Clamping prevents NaN/Inf propagation from floating-point errors

---

#### 2. Python: Per-Slot Normalization

**File**: `management-plane/app/encoding.py`

**Before** (lines 421-443):
```python
for slot_name in ["action", "resource", "data", "risk"]:
    text = slot_texts[slot_name]
    embedding_384 = encode_text_cached(text)
    projection_matrix = get_projection_matrix(slot_name, slot_seeds[slot_name])
    projected_32 = projection_matrix @ embedding_384
    slot_vectors.append(projected_32)  # ❌ Not normalized

# Concatenate to 128-dim
vector_128 = np.concatenate(slot_vectors)

# L2 normalize (GLOBAL - causes bug)
norm = np.linalg.norm(vector_128)
if norm > 0:
    vector_128 = vector_128 / norm
```

**After**:
```python
for slot_name in ["action", "resource", "data", "risk"]:
    text = slot_texts[slot_name]
    embedding_384 = encode_text_cached(text)
    projection_matrix = get_projection_matrix(slot_name, slot_seeds[slot_name])
    projected_32 = projection_matrix @ embedding_384

    # ✅ Normalize per-slot (NEW)
    norm = np.linalg.norm(projected_32)
    if norm > 0:
        projected_32 = projected_32 / norm

    slot_vectors.append(projected_32)

# Concatenate to 128-dim
vector_128 = np.concatenate(slot_vectors)

# ❌ DELETE global normalization (lines 439-443)
# norm = np.linalg.norm(vector_128)
# if norm > 0:
#     vector_128 = vector_128 / norm
```

**Apply Same Change** to `encode_boundary_to_128d()` (lines 467-489).

**Rationale**:
- Each 32-d slot becomes unit vector: `||slot|| = 1`
- Concatenated 128-d vector has `||v_128|| = 2` (sum of 4 unit vectors)
- Per-slice cosine in Rust correctly computes `dot(u, v) / (1 × 1) = dot(u, v) = cos(θ)`

---

### Testing (Phase 1)

#### Rust Unit Tests

**File**: `semantic-sandbox/src/compare.rs`

```rust
#[test]
fn test_identical_vectors_cosine_one() {
    // Create non-normalized identical vectors
    let intent = [0.5f32; 128];
    let boundary = intent.clone();

    let envelope = VectorEnvelope {
        intent,
        boundary,
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
    let mut boundary = [0.0f32; 128];

    // Make first slice orthogonal
    intent[0..32].fill(1.0);
    boundary[32..64].fill(1.0);

    let envelope = VectorEnvelope {
        intent,
        boundary,
        thresholds: [0.0, 0.0, 0.0, 0.0],
        weights: [1.0, 1.0, 1.0, 1.0],
        decision_mode: 0,
        global_threshold: 0.0,
    };

    let result = compare(&envelope);

    // First slice should be ~0 (orthogonal)
    assert!(result.slice_similarities[0].abs() < 0.05);
}

#[test]
fn test_zero_norm_guard() {
    let intent = [0.0f32; 128];
    let boundary = [1.0f32; 128];

    let envelope = VectorEnvelope {
        intent,
        boundary,
        thresholds: [0.8, 0.8, 0.8, 0.8],
        weights: [1.0, 1.0, 1.0, 1.0],
        decision_mode: 0,
        global_threshold: 0.8,
    };

    let result = compare(&envelope);

    // Should return 0.0, not NaN or panic
    for sim in result.slice_similarities {
        assert!(!sim.is_nan());
        assert_eq!(sim, 0.0);
    }
}
```

#### Python Unit Tests

**File**: `management-plane/tests/test_encoding.py`

```python
def test_per_slot_normalization():
    """Test that each 32-d slot is unit-normalized after projection."""
    event = create_test_intent_event()
    vector_128 = encode_to_128d(event)

    # Extract 4 slots
    action_slot = vector_128[0:32]
    resource_slot = vector_128[32:64]
    data_slot = vector_128[64:96]
    risk_slot = vector_128[96:128]

    # Each slot should have norm = 1.0
    assert np.abs(np.linalg.norm(action_slot) - 1.0) < 0.001
    assert np.abs(np.linalg.norm(resource_slot) - 1.0) < 0.001
    assert np.abs(np.linalg.norm(data_slot) - 1.0) < 0.001
    assert np.abs(np.linalg.norm(risk_slot) - 1.0) < 0.001

def test_slot_independence():
    """Test that changing one slot doesn't affect other slots."""
    event1 = create_test_intent_event(action="read")
    event2 = create_test_intent_event(action="write")

    vec1 = encode_to_128d(event1)
    vec2 = encode_to_128d(event2)

    # Action slot should differ
    action_sim = np.dot(vec1[0:32], vec2[0:32])
    assert action_sim < 0.9  # Different actions

    # Other slots should be identical (same resource/data/risk)
    resource_sim = np.dot(vec1[32:64], vec2[32:64])
    data_sim = np.dot(vec1[64:96], vec2[64:96])
    risk_sim = np.dot(vec1[96:128], vec2[96:128])

    assert resource_sim > 0.99
    assert data_sim > 0.99
    assert risk_sim > 0.99
```

#### Integration Test

```python
def test_phase1_matching_values_high_similarity():
    """Test that matching intent and boundary produce high similarity."""
    from app.encoding import encode_to_128d, encode_boundary_to_128d
    from app.types import IntentEvent, DesignBoundary

    # Intent: read action by agent
    intent = IntentEvent(
        id="test",
        schemaVersion="v1.2",
        timestamp="2025-11-13T00:00:00Z",
        actor=Actor(id="agent_1", type="agent"),
        action="read",
        resource=Resource(type="database", name="users"),
        data=Data(sensitivity=["internal"], pii=False, volume="single"),
        risk=Risk(authn="required"),
    )

    # Boundary: allows read by agent
    boundary = DesignBoundary(
        id="allow_read",
        name="Allow Read",
        constraints=Constraints(
            action=ActionConstraint(actions=["read"], actor_types=["agent"]),
            resource=ResourceConstraint(types=["database"]),
            data=DataConstraint(sensitivity=["internal"], pii=False, volume="single"),
            risk=RiskConstraint(authn="required"),
        ),
        # ... other fields
    )

    intent_vec = encode_to_128d(intent)
    boundary_vec = encode_boundary_to_128d(boundary)

    # Compute per-slice cosine manually
    for i, (start, end) in enumerate([(0, 32), (32, 64), (64, 96), (96, 128)]):
        intent_slice = intent_vec[start:end]
        boundary_slice = boundary_vec[start:end]
        sim = np.dot(intent_slice, boundary_slice)

        # After Phase 1, should be ≥ 0.85
        assert sim >= 0.85, f"Slice {i} similarity {sim} below threshold"
```

---

### Success Criteria (Phase 1)

- ✅ All Rust tests pass
- ✅ All Python tests pass
- ✅ Existing 45 type validation tests still pass
- ✅ Integration test: matching values → sims ≥ 0.85
- ✅ Per-slot norms = 1.0 ± 0.001
- ✅ Rust: identical vectors → slice sims = 1.0 ± 0.01

**Expected Similarity Improvements**:
```
Action:   0.16 → 0.92
Resource: 0.17 → 0.88
Data:     0.24 → 0.90
Risk:     0.20 → 0.91
```

---

## Phase 2: Anchor-Based Encoding

**Goal**: Implement containment semantics using max-of-anchors comparison.

### Problem

Current boundary encoding creates semantic mismatch:
- Boundary: `"action: read, write, delete | actor_type: user, service, llm, agent"`
- Intent: `"action: read | actor_type: agent"`

Even after Phase 1 math fix, sentence-transformer embeddings for "mixture" vs "atom" won't align well. We need **logical OR** semantics: if intent value ∈ allowed set, similarity should be ~1.0.

### Solution: Max-of-Anchors

For each boundary slot, generate **atomic anchor embeddings** for every allowed value:

```
Boundary allows actions = ["read", "write", "delete"]
→ Generate 3 anchors:
  - anchor1 = encode("action is read")
  - anchor2 = encode("action is write")
  - anchor3 = encode("action is delete")

Intent has action = "read"
→ intent_emb = encode("action is read")

Slot similarity = max(cosine(intent_emb, anchor) for anchor in anchors)
                = max(0.95, 0.30, 0.25) = 0.95 ✅
```

**Key Insight**: Max implements logical OR containment - if intent matches ANY allowed anchor, similarity is high.

---

### FFI Extension

**File**: `semantic-sandbox/src/lib.rs`

**Current Struct**:
```rust
#[repr(C)]
pub struct VectorEnvelope {
    pub intent: [f32; 128],
    pub boundary: [f32; 128],  // ❌ Single vector - doesn't support multiple anchors
    pub thresholds: [f32; 4],
    pub weights: [f32; 4],
    pub decision_mode: u8,
    pub global_threshold: f32,
}
```

**New Struct** (in-place update):
```rust
#[repr(C)]
pub struct VectorEnvelope {
    // Intent vector (unchanged)
    pub intent: [f32; 128],

    // Boundary anchors (REPLACE single boundary vector)
    pub action_anchors: [[f32; 32]; 16],      // Max 16 anchors per slot
    pub action_anchor_count: usize,
    pub resource_anchors: [[f32; 32]; 16],
    pub resource_anchor_count: usize,
    pub data_anchors: [[f32; 32]; 16],
    pub data_anchor_count: usize,
    pub risk_anchors: [[f32; 32]; 16],
    pub risk_anchor_count: usize,

    // Decision parameters (unchanged)
    pub thresholds: [f32; 4],
    pub weights: [f32; 4],
    pub decision_mode: u8,
    pub global_threshold: f32,
}
```

**Design Rationale**:
- **Fixed-size arrays**: FFI-safe (no pointers, no heap allocation across boundary)
- **Per-slot anchors**: Each slot has independent allowed values
- **Max 16 anchors**: Reasonable limit (most boundaries have <10 options per slot)
- **32-d slots**: Store normalized 32-d vectors directly (not full 128-d)

---

### Rust Comparison Logic

**File**: `semantic-sandbox/src/compare.rs`

**New Helper Function**:
```rust
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
```

**Updated `compare_real()` Function**:
```rust
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

    // Decision logic (min/weighted-avg) - unchanged
    let decision = if envelope.decision_mode == 0 {
        // Mode 0: min (mandatory boundaries)
        let all_pass = slice_similarities
            .iter()
            .zip(envelope.thresholds.iter())
            .all(|(sim, thresh)| sim >= thresh);
        if all_pass { 1 } else { 0 }
    } else {
        // Mode 1: weighted-avg (optional boundaries)
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
```

---

### Python FFI Binding Update

**File**: `management-plane/app/rust_ffi.py`

**Before**:
```python
class VectorEnvelope(ctypes.Structure):
    _fields_ = [
        ("intent", ctypes.c_float * 128),
        ("boundary", ctypes.c_float * 128),
        ("thresholds", ctypes.c_float * 4),
        ("weights", ctypes.c_float * 4),
        ("decision_mode", ctypes.c_uint8),
        ("global_threshold", ctypes.c_float),
    ]
```

**After**:
```python
class VectorEnvelope(ctypes.Structure):
    _fields_ = [
        ("intent", ctypes.c_float * 128),

        # Anchor arrays (4 slots × 16 anchors × 32 dims)
        ("action_anchors", (ctypes.c_float * 32) * 16),
        ("action_anchor_count", ctypes.c_size_t),
        ("resource_anchors", (ctypes.c_float * 32) * 16),
        ("resource_anchor_count", ctypes.c_size_t),
        ("data_anchors", (ctypes.c_float * 32) * 16),
        ("data_anchor_count", ctypes.c_size_t),
        ("risk_anchors", (ctypes.c_float * 32) * 16),
        ("risk_anchor_count", ctypes.c_size_t),

        ("thresholds", ctypes.c_float * 4),
        ("weights", ctypes.c_float * 4),
        ("decision_mode", ctypes.c_uint8),
        ("global_threshold", ctypes.c_float),
    ]
```

---

### Python Encoding: Anchor Generation

**File**: `management-plane/app/encoding.py`

**New Functions**:

```python
def build_boundary_action_anchors(boundary: DesignBoundary) -> list[str]:
    """
    Build canonical anchor strings for action slot.

    Returns one string per (action, actor_type) combination.
    Uses atomic templates for semantic alignment with intents.

    Example:
        actions = ["read", "write"]
        actor_types = ["user", "agent"]

        Returns:
            [
                "action is read | actor_type equals user",
                "action is read | actor_type equals agent",
                "action is write | actor_type equals user",
                "action is write | actor_type equals agent"
            ]
    """
    anchors = []
    for action in sorted(boundary.constraints.action.actions):
        for actor_type in sorted(boundary.constraints.action.actor_types):
            # Use "is" and "equals" for semantic alignment with intents
            anchor = f"action is {action} | actor_type equals {actor_type}"
            anchors.append(anchor)
    return anchors


def build_boundary_resource_anchors(boundary: DesignBoundary) -> list[str]:
    """Build canonical anchor strings for resource slot."""
    anchors = []

    # Generate anchors for each combination of type × location
    types = sorted(boundary.constraints.resource.types)
    locations = sorted(boundary.constraints.resource.locations or ["unspecified"])

    for rtype in types:
        for location in locations:
            anchor = f"resource_type is {rtype} | resource_location is {location}"
            anchors.append(anchor)

    # If specific names are constrained, add them
    if boundary.constraints.resource.names:
        for name in sorted(boundary.constraints.resource.names):
            anchor = f"resource_name is {name}"
            anchors.append(anchor)

    return anchors


def build_boundary_data_anchors(boundary: DesignBoundary) -> list[str]:
    """Build canonical anchor strings for data slot."""
    anchors = []

    # Generate anchors for each combination of sensitivity × pii × volume
    sensitivities = sorted(boundary.constraints.data.sensitivity)
    pii_values = [boundary.constraints.data.pii] if boundary.constraints.data.pii is not None else [True, False]
    volumes = [boundary.constraints.data.volume] if boundary.constraints.data.volume else ["single", "bulk"]

    for sensitivity in sensitivities:
        for pii in pii_values:
            for volume in volumes:
                anchor = f"sensitivity is {sensitivity} | pii is {pii} | volume is {volume}"
                anchors.append(anchor)

    return anchors


def build_boundary_risk_anchors(boundary: DesignBoundary) -> list[str]:
    """Build canonical anchor strings for risk slot."""
    # Risk slot is simple - just authn values
    authn = boundary.constraints.risk.authn
    return [f"authn is {authn}"]


def encode_anchors_to_32d(
    anchor_texts: list[str],
    slot_name: str,
    slot_seed: int,
    max_anchors: int = 16
) -> tuple[np.ndarray, int]:
    """
    Encode list of anchor texts to normalized 32-d vectors.

    Args:
        anchor_texts: List of canonical anchor strings
        slot_name: Name of slot (for logging)
        slot_seed: Random seed for projection matrix
        max_anchors: Maximum number of anchors (truncate if exceeded)

    Returns:
        Tuple of (anchor_array, count) where:
        - anchor_array: np.ndarray of shape (max_anchors, 32) with padding
        - count: Actual number of anchors (before padding)
    """
    if len(anchor_texts) > max_anchors:
        logger.warning(
            f"Boundary {slot_name} slot has {len(anchor_texts)} anchors, "
            f"truncating to {max_anchors}"
        )
        anchor_texts = anchor_texts[:max_anchors]

    # Encode each anchor text
    anchor_vecs = []
    for text in anchor_texts:
        # Encode to 384-d
        embedding_384 = encode_text_cached(text)

        # Project to 32-d
        projection_matrix = get_projection_matrix(slot_name, slot_seed)
        projected_32 = projection_matrix @ embedding_384

        # Normalize per-slot
        norm = np.linalg.norm(projected_32)
        if norm > 0:
            projected_32 = projected_32 / norm

        anchor_vecs.append(projected_32)

    # Pad to max_anchors with zeros
    anchor_array = np.zeros((max_anchors, 32), dtype=np.float32)
    for i, vec in enumerate(anchor_vecs):
        anchor_array[i] = vec

    return anchor_array, len(anchor_texts)
```

**Update Intent Encoding** (align template format):

```python
def build_action_slot(event: IntentEvent) -> str:
    """
    Build text representation for the action slot (aligned with anchors).

    Uses "is" and "equals" to match anchor templates.
    """
    return f"action is {event.action} | actor_type equals {event.actor.type}"


def build_resource_slot(event: IntentEvent) -> str:
    """Build text representation for the resource slot (aligned with anchors)."""
    parts = [f"resource_type is {event.resource.type}"]

    if event.resource.location:
        parts.append(f"resource_location is {event.resource.location}")

    if event.resource.name:
        parts.append(f"resource_name is {event.resource.name}")

    return " | ".join(parts)


def build_data_slot(event: IntentEvent) -> str:
    """Build text representation for the data slot (aligned with anchors)."""
    sensitivity = event.data.sensitivity[0] if event.data.sensitivity else "unspecified"
    pii = event.data.pii if event.data.pii is not None else False
    volume = event.data.volume or "single"

    return f"sensitivity is {sensitivity} | pii is {pii} | volume is {volume}"


def build_risk_slot(event: IntentEvent) -> str:
    """Build text representation for the risk slot (aligned with anchors)."""
    return f"authn is {event.risk.authn}"
```

---

### Python Endpoint: Populate Envelope

**File**: `management-plane/app/endpoints/intents.py`

**Before** (lines 186-215):
```python
# Encode the boundary to 128-dim vector (with caching)
boundary_json = boundary.model_dump_json()
boundary_vector = encode_boundary_to_128d_cached(boundary.id, boundary_json)

# Call Rust sandbox for comparison
decision, similarities = sandbox.compare(
    intent_vector=intent_vector,
    boundary_vector=boundary_vector,
    thresholds=thresholds,
    weights=weights,
    decision_mode=decision_mode,
    global_threshold=global_threshold,
)
```

**After**:
```python
from app.encoding import (
    build_boundary_action_anchors,
    build_boundary_resource_anchors,
    build_boundary_data_anchors,
    build_boundary_risk_anchors,
    encode_anchors_to_32d,
)

# Generate anchor embeddings per slot
action_anchors, action_count = encode_anchors_to_32d(
    build_boundary_action_anchors(boundary),
    slot_name="action",
    slot_seed=42,
)
resource_anchors, resource_count = encode_anchors_to_32d(
    build_boundary_resource_anchors(boundary),
    slot_name="resource",
    slot_seed=43,
)
data_anchors, data_count = encode_anchors_to_32d(
    build_boundary_data_anchors(boundary),
    slot_name="data",
    slot_seed=44,
)
risk_anchors, risk_count = encode_anchors_to_32d(
    build_boundary_risk_anchors(boundary),
    slot_name="risk",
    slot_seed=45,
)

# Call Rust sandbox with anchor arrays
decision, similarities = sandbox.compare(
    intent_vector=intent_vector,
    action_anchors=action_anchors,
    action_anchor_count=action_count,
    resource_anchors=resource_anchors,
    resource_anchor_count=resource_count,
    data_anchors=data_anchors,
    data_anchor_count=data_count,
    risk_anchors=risk_anchors,
    risk_anchor_count=risk_count,
    thresholds=thresholds,
    weights=weights,
    decision_mode=decision_mode,
    global_threshold=global_threshold,
)
```

**Note**: Update `sandbox.compare()` method in `rust_ffi.py` to accept new parameters.

---

### Testing (Phase 2)

#### Rust Unit Tests

**File**: `semantic-sandbox/src/compare.rs`

```rust
#[test]
fn test_max_anchor_similarity_containment() {
    // Intent: action = "read"
    let mut intent_action = [0.0f32; 32];
    intent_action[0] = 1.0;  // Simplified: use first dim

    // Anchors: ["read", "write", "delete"]
    let mut read_anchor = [0.0f32; 32];
    read_anchor[0] = 1.0;  // Same as intent

    let mut write_anchor = [0.0f32; 32];
    write_anchor[1] = 1.0;  // Different dim

    let mut delete_anchor = [0.0f32; 32];
    delete_anchor[2] = 1.0;  // Different dim

    let anchors = [read_anchor, write_anchor, delete_anchor];

    let max_sim = max_anchor_similarity(&intent_action, &anchors, 3);

    // Should match "read" anchor with sim ~1.0
    assert!(max_sim > 0.99, "Expected max_sim ~1.0, got {}", max_sim);
}

#[test]
fn test_max_anchor_similarity_no_match() {
    // Intent: action = "export"
    let mut intent_action = [0.0f32; 32];
    intent_action[3] = 1.0;

    // Anchors: ["read", "write", "delete"] (no "export")
    let mut read_anchor = [0.0f32; 32];
    read_anchor[0] = 1.0;

    let mut write_anchor = [0.0f32; 32];
    write_anchor[1] = 1.0;

    let mut delete_anchor = [0.0f32; 32];
    delete_anchor[2] = 1.0;

    let anchors = [read_anchor, write_anchor, delete_anchor];

    let max_sim = max_anchor_similarity(&intent_action, &anchors, 3);

    // Should have low similarity (orthogonal)
    assert!(max_sim < 0.1, "Expected low sim, got {}", max_sim);
}

#[test]
fn test_empty_anchor_set_wildcard() {
    let intent_action = [1.0f32; 32];
    let anchors: [[f32; 32]; 16] = [[0.0; 32]; 16];

    let max_sim = max_anchor_similarity(&intent_action, &anchors, 0);

    // Empty anchor set = wildcard = always pass
    assert_eq!(max_sim, 1.0);
}
```

#### Python Unit Tests

**File**: `management-plane/tests/test_encoding.py`

```python
def test_anchor_generation_action_slot():
    """Test that action slot generates correct number of anchors."""
    boundary = DesignBoundary(
        id="test",
        name="Test",
        constraints=Constraints(
            action=ActionConstraint(
                actions=["read", "write"],
                actor_types=["user", "agent"]
            ),
            # ... other constraints
        ),
    )

    anchors = build_boundary_action_anchors(boundary)

    # Should have 2 actions × 2 actor_types = 4 anchors
    assert len(anchors) == 4
    assert "action is read | actor_type equals user" in anchors
    assert "action is write | actor_type equals agent" in anchors


def test_anchor_encoding_canonical_templates():
    """Test that anchor templates align with intent format."""
    boundary = DesignBoundary(
        id="test",
        name="Test",
        constraints=Constraints(
            action=ActionConstraint(actions=["read"], actor_types=["agent"]),
            # ... other constraints
        ),
    )

    # Generate anchor
    anchors = build_boundary_action_anchors(boundary)
    assert len(anchors) == 1
    anchor_text = anchors[0]

    # Create matching intent
    intent = IntentEvent(
        id="test",
        schemaVersion="v1.2",
        actor=Actor(id="a1", type="agent"),
        action="read",
        # ... other fields
    )
    intent_text = build_action_slot(intent)

    # Anchor and intent should have exact same format
    assert anchor_text == intent_text


def test_containment_high_similarity():
    """Test that intent value in allowed set produces high similarity."""
    # Boundary allows actions = ["read", "write", "delete"]
    boundary = DesignBoundary(
        id="test",
        name="Test",
        constraints=Constraints(
            action=ActionConstraint(
                actions=["read", "write", "delete"],
                actor_types=["agent"]
            ),
            # ... other constraints
        ),
    )

    # Intent has action = "read" (in allowed set)
    intent = IntentEvent(
        id="test",
        schemaVersion="v1.2",
        actor=Actor(id="a1", type="agent"),
        action="read",
        # ... other fields
    )

    # Encode intent to 128-d
    intent_vec = encode_to_128d(intent)
    intent_action_slot = intent_vec[0:32]

    # Generate action anchors
    anchor_texts = build_boundary_action_anchors(boundary)
    anchor_array, count = encode_anchors_to_32d(anchor_texts, "action", 42)

    # Compute max similarity
    max_sim = max(
        np.dot(intent_action_slot, anchor_array[i])
        for i in range(count)
    )

    # Should have high similarity (≥ 0.90)
    assert max_sim >= 0.90, f"Expected ≥0.90, got {max_sim}"
```

#### Integration Test

```python
def test_phase2_end_to_end_demo_scenario():
    """Test full pipeline with demo scenario: search customer (read)."""
    from examples.langgraph_demo.policies import create_sample_boundaries

    # Use real demo boundary
    boundaries = create_sample_boundaries()
    allow_read_boundary = next(b for b in boundaries if b.name == "Allow Read Operations")

    # Create intent matching demo SDK output
    intent = IntentEvent(
        id="test",
        schemaVersion="v1.2",
        timestamp="2025-11-13T00:00:00Z",
        actor=Actor(id="agent_1", type="agent"),
        action="read",
        resource=Resource(type="api", name="search_customer", location="cloud"),
        data=Data(sensitivity=["internal"], pii=False, volume="single"),
        risk=Risk(authn="required"),
    )

    # Encode intent
    intent_vec = encode_to_128d(intent)

    # Generate boundary anchors
    action_anchors, action_count = encode_anchors_to_32d(
        build_boundary_action_anchors(allow_read_boundary),
        "action", 42
    )
    # ... other slots

    # Call Rust comparison
    sandbox = get_sandbox()
    decision, similarities = sandbox.compare(
        intent_vector=intent_vec,
        action_anchors=action_anchors,
        action_anchor_count=action_count,
        # ... other params
    )

    # Should ALLOW with high action similarity
    assert decision == 1, "Should ALLOW read operation"
    assert similarities[0] >= 0.90, f"Action similarity {similarities[0]} too low"
```

---

### Success Criteria (Phase 2)

- ✅ All Rust tests pass (including new anchor tests)
- ✅ All Python tests pass (including new anchor generation tests)
- ✅ Integration test: containment semantics validated
- ✅ demo_prebuilt.py search scenario → ALLOW with action sim ≥ 0.90
- ✅ demo_prebuilt.py delete scenario → BLOCK (policy violation)
- ✅ No performance regression (still <100ms for 100 boundaries)

**Expected Similarity Improvements**:
```
Action:   0.16 → 0.92 (Phase 1) → 0.95 (Phase 2)
Resource: 0.17 → 0.88 (Phase 1) → 0.92 (Phase 2)
Data:     0.24 → 0.90 (Phase 1) → 0.94 (Phase 2)
Risk:     0.20 → 0.91 (Phase 1) → 0.95 (Phase 2)
```

---

## Implementation Sequence

### Phase 1: Math Fix (Estimated: 2-3 hours)

1. **Rust**:
   - Update `compare.rs:33-37` with per-slice cosine
   - Add 3 unit tests
   - `cargo test` → verify all pass
   - `cargo build --release`

2. **Python**:
   - Update `encoding.py:433-434, 478-479` with per-slot normalization
   - Delete `encoding.py:439-443, 485-489` (global L2)
   - Add 2 unit tests
   - `pytest tests/test_encoding.py` → verify all pass

3. **Integration**:
   - Add integration test for matching values
   - Run full test suite: `pytest`
   - Validate: sims ≥ 0.85 for matches

4. **Validation**:
   - Check Management Plane logs for similarity scores
   - Expected: action/resource/data/risk ≥ 0.85

---

### Phase 2: Anchor Encoding (Estimated: 4-6 hours)

1. **Rust FFI Struct** (`semantic-sandbox/src/lib.rs`):
   - Replace `boundary: [f32; 128]` with 4 anchor arrays
   - Update `VectorEnvelope` struct

2. **Rust Comparison** (`semantic-sandbox/src/compare.rs`):
   - Add `max_anchor_similarity()` helper
   - Add `cosine_similarity()` helper
   - Update `compare_real()` to use anchors
   - Add 3 unit tests
   - `cargo test` → verify
   - `cargo build --release`

3. **Python FFI Binding** (`management-plane/app/rust_ffi.py`):
   - Update `VectorEnvelope` ctypes struct
   - Update `compare()` method signature

4. **Python Encoding** (`management-plane/app/encoding.py`):
   - Add 4 `build_boundary_*_anchors()` functions
   - Add `encode_anchors_to_32d()` function
   - Update 4 `build_*_slot()` functions (align templates)
   - Add 3 unit tests

5. **Python Endpoint** (`management-plane/app/endpoints/intents.py`):
   - Update comparison loop (lines 186-215)
   - Generate anchors per slot
   - Populate envelope with anchor arrays

6. **Integration Testing**:
   - Add end-to-end test with demo scenario
   - Run `pytest`
   - Run `examples/langgraph_demo/demo_prebuilt.py`
   - Verify: search → ALLOW, delete → BLOCK

---

## Performance Considerations

**Memory**:
- VectorEnvelope size: ~4KB per boundary comparison
  - 128 floats (intent) = 512 bytes
  - 4 slots × 16 anchors × 32 floats = 8,192 bytes
  - Metadata = ~100 bytes
- Total: ~9KB per comparison (acceptable for stack allocation)

**Compute**:
- Anchor encoding (Python): Batch encode all anchors per slot → ~10-20ms
- Max-of-anchors (Rust): 4 slots × 16 anchors × cosine → ~1-2ms
- Total per boundary: ~12-22ms (still well under 100ms for 100 boundaries)

**Caching**:
- Cache boundary anchor arrays by boundary ID (LRU cache, maxsize=1000)
- Encoding cache already exists for text → 384-d embeddings
- Expected cache hit rate: >90% for active boundaries

---

## Edge Cases & Error Handling

**Rust**:
1. Zero-norm vectors → return 0.0 similarity
2. Empty anchor sets → return 1.0 (wildcard / always pass)
3. NaN/Inf from FP errors → clamp and return 0.0
4. Anchor count > 16 → use first 16 (Python truncates)

**Python**:
1. Anchor count > 16 → log warning, truncate
2. Empty slot text → raise ValueError
3. Encoding failure → HTTPException 500
4. Invalid boundary constraints → validation error at boundary creation

---

## Rollback & Migration

Not applicable - this is active development. All changes deployed together.

---

## Success Metrics Summary

**Phase 1**:
- ✅ Math bug fixed: identical vectors → sims = 1.0
- ✅ Per-slot normalization working
- ✅ Similarity improvements: 0.16-0.29 → 0.85-0.92

**Phase 2**:
- ✅ Containment semantics implemented
- ✅ Max-of-anchors comparison working
- ✅ Similarity improvements: 0.85-0.92 → 0.90-0.95
- ✅ Demo scenarios validated (search → ALLOW, delete → BLOCK)

**Final Acceptance**:
- ✅ All tests passing (Rust + Python + integration)
- ✅ Performance maintained (<100ms for 100 boundaries)
- ✅ Demo working end-to-end

---

## Files Modified Summary

### Phase 1
- `semantic-sandbox/src/compare.rs` (lines 33-37, new tests)
- `management-plane/app/encoding.py` (lines 433-434, 439-443, 478-479, 485-489)
- `management-plane/tests/test_encoding.py` (new tests)

### Phase 2
- `semantic-sandbox/src/lib.rs` (VectorEnvelope struct)
- `semantic-sandbox/src/compare.rs` (new helpers, updated logic)
- `management-plane/app/rust_ffi.py` (VectorEnvelope binding)
- `management-plane/app/encoding.py` (new anchor functions)
- `management-plane/app/endpoints/intents.py` (lines 186-215)
- `management-plane/tests/test_encoding.py` (new anchor tests)

---

## References

- **Original Analysis**: Codex conversation (2025-11-13)
- **Related Sessions**:
  - Session 7: v1.1 encoding achieving 96.7% similarity
  - Session 12: Discovery of low similarity issue
- **Algorithm Documentation**: `algo.md` sections on slice-based comparison
- **Plan Documentation**: `plan.md` sections 2.1-2.4 (data contracts)

---

**End of Design Document**
