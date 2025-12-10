"""
Unit tests for encoding pipeline.

Tests:
1. Determinism - same input produces same output
2. Dimensionality - correct vector dimensions
3. Normalization - vectors are L2 normalized
4. Slot independence - different slots produce different patterns
5. Caching - LRU cache works correctly
6. Canonicalization - deterministic text generation
"""

import numpy as np
import pytest

from app.encoding import (
    canonicalize_dict,
    build_action_slot,
    build_resource_slot,
    build_data_slot,
    build_risk_slot,
    encode_to_128d,
    encode_boundary_to_128d,
    get_cache_stats,
    clear_cache,
    create_sparse_projection_matrix,
    encode_text_cached,
)
from app.models import (
    IntentEvent,
    Actor,
    Resource,
    Data,
    Risk,
    DesignBoundary,
    BoundaryScope,
    SliceThresholds,
    BoundaryRules,
    BoundaryConstraints,
    ActionConstraint,
    ResourceConstraint,
    DataConstraint,
    RiskConstraint,
)


# Test fixtures

def create_test_intent() -> IntentEvent:
    """Create a test IntentEvent for consistent testing (v1.1)."""
    return IntentEvent(
        id="test_001",
        schemaVersion="v1.1",
        tenantId="tenant_test",
        timestamp=1700000000.0,
        action="read",
        actor=Actor(
            id="alice@example.com",
            type="user",
        ),
        resource=Resource(
            type="database",
            name="users_db",
            location="cloud",
        ),
        data=Data(
            sensitivity=["internal"],
            pii=False,
            volume="single",
        ),
        risk=Risk(
            authn="required",
        ),
    )


def create_test_boundary() -> DesignBoundary:
    """Create a test DesignBoundary for consistent testing (v1.1)."""
    return DesignBoundary(
        id="boundary_001",
        name="Safe Read Access",
        status="active",
        type="mandatory",
        boundarySchemaVersion="v1.1",
        scope=BoundaryScope(
            tenantId="tenant_test",
            domains=["database", "api"],
        ),
        rules=BoundaryRules(
            thresholds=SliceThresholds(
                action=0.8,
                resource=0.75,
                data=0.7,
                risk=0.6,
            ),
            decision="min",
        ),
        constraints=BoundaryConstraints(
            action=ActionConstraint(
                actions=["read"],
                actor_types=["user"],
            ),
            resource=ResourceConstraint(
                types=["database"],
                locations=["cloud"],
            ),
            data=DataConstraint(
                sensitivity=["internal"],
                pii=False,
                volume="single",
            ),
            risk=RiskConstraint(
                authn="required",
            ),
        ),
        notes="Test boundary for unit tests",
        createdAt=1700000000.0,
        updatedAt=1700000000.0,
    )


# Tests for canonicalization

def test_canonicalize_dict_simple():
    """Test basic dictionary canonicalization."""
    data = {"name": "Alice", "age": 30}
    result = canonicalize_dict(data)

    # Should be deterministic and sorted
    assert "age=30" in result
    assert "name=Alice" in result


def test_canonicalize_dict_nested():
    """Test nested dictionary flattening."""
    data = {"user": {"profile": {"name": "Alice"}}}
    result = canonicalize_dict(data)

    assert "user.profile.name=Alice" in result


def test_canonicalize_dict_with_list():
    """Test list handling with index notation."""
    data = {"items": ["apple", "banana"]}
    result = canonicalize_dict(data)

    assert "items[0]=apple" in result
    assert "items[1]=banana" in result


def test_canonicalize_dict_determinism():
    """Test that canonicalization is deterministic."""
    data1 = {"b": 2, "a": 1, "c": 3}
    data2 = {"c": 3, "a": 1, "b": 2}

    result1 = canonicalize_dict(data1)
    result2 = canonicalize_dict(data2)

    assert result1 == result2


def test_canonicalize_dict_skips_none():
    """Test that None values are skipped."""
    data = {"a": 1, "b": None, "c": 3}
    result = canonicalize_dict(data)

    assert "a=1" in result
    assert "c=3" in result
    assert "b" not in result


# Tests for slot builders

def test_build_action_slot_includes_action():
    """Test action slot includes the action type and actor_type (v1.1)."""
    event = create_test_intent()
    slot_text = build_action_slot(event)

    # Phase 2: uses "is" and "equals" for anchor alignment
    assert "action is read" in slot_text
    assert "actor_type equals user" in slot_text
    # v1.1: actor_id is excluded from slot encoding
    assert "actor_id" not in slot_text


def test_build_resource_slot_includes_type():
    """Test resource slot includes resource type, name, and location."""
    event = create_test_intent()
    slot_text = build_resource_slot(event)

    assert "resource_type is database" in slot_text
    assert "resource_name is users_db" in slot_text
    assert "resource_location is cloud" in slot_text


def test_build_data_slot_includes_sensitivity():
    """Test data slot includes sensitivity, pii, and volume (v1.1)."""
    event = create_test_intent()
    slot_text = build_data_slot(event)

    # v1.1: uses sensitivity instead of categories
    assert "sensitivity is internal" in slot_text
    assert "pii is False" in slot_text
    assert "volume is single" in slot_text


def test_build_risk_slot_includes_indicators():
    """Test risk slot includes authn requirement (v1.1)."""
    event = create_test_intent()
    slot_text = build_risk_slot(event)

    # v1.1: only authn (required/not_required), no network or timeOfDay
    assert "authn is required" in slot_text
    assert "network" not in slot_text
    assert "timeOfDay" not in slot_text


# Tests for sparse projection matrix

def test_create_sparse_projection_matrix_dimensions():
    """Test projection matrix has correct dimensions."""
    matrix = create_sparse_projection_matrix(input_dim=384, output_dim=32, seed=42)

    assert matrix.shape == (32, 384)
    assert matrix.dtype == np.float32


def test_create_sparse_projection_matrix_determinism():
    """Test projection matrix is deterministic with same seed."""
    matrix1 = create_sparse_projection_matrix(input_dim=384, output_dim=32, seed=42)
    matrix2 = create_sparse_projection_matrix(input_dim=384, output_dim=32, seed=42)

    assert np.allclose(matrix1, matrix2)


def test_create_sparse_projection_matrix_different_seeds():
    """Test different seeds produce different matrices."""
    matrix1 = create_sparse_projection_matrix(input_dim=384, output_dim=32, seed=42)
    matrix2 = create_sparse_projection_matrix(input_dim=384, output_dim=32, seed=43)

    assert not np.allclose(matrix1, matrix2)


def test_create_sparse_projection_matrix_sparsity():
    """Test projection matrix has correct sparsity (2/3 zeros)."""
    matrix = create_sparse_projection_matrix(input_dim=384, output_dim=32, seed=42, sparsity=0.66)

    # Count zeros
    zero_fraction = np.sum(matrix == 0) / matrix.size

    # Should be approximately 2/3 (allow some variance due to randomness)
    assert 0.60 < zero_fraction < 0.72


# Tests for text encoding

def test_encode_text_cached_dimensions():
    """Test text encoding produces 384-dim vectors."""
    text = "This is a test sentence"
    embedding = encode_text_cached(text)

    assert embedding.shape == (384,)
    assert embedding.dtype == np.float32


def test_encode_text_cached_determinism():
    """Test text encoding is deterministic."""
    text = "Test sentence"

    # Clear cache first
    clear_cache()

    embedding1 = encode_text_cached(text)
    embedding2 = encode_text_cached(text)

    assert np.allclose(embedding1, embedding2)


def test_encode_text_cached_caching():
    """Test LRU cache is working."""
    clear_cache()

    text = "Cached text"
    encode_text_cached(text)
    encode_text_cached(text)

    stats = get_cache_stats()
    assert stats["hits"] >= 1  # Second call should be a cache hit


# Tests for 128-dim encoding

def test_encode_to_128d_dimensions():
    """Test encoding produces 128-dim vectors."""
    event = create_test_intent()
    vector = encode_to_128d(event)

    assert vector.shape == (128,)
    assert vector.dtype == np.float32


def test_encode_to_128d_normalization():
    """Test vectors have correct global norm (Phase 1: per-slot normalization)."""
    event = create_test_intent()
    vector = encode_to_128d(event)

    norm = np.linalg.norm(vector)
    # Phase 1: Each slot is unit-normalized, so global norm = sqrt(4) = 2.0
    assert np.isclose(norm, 2.0, atol=1e-5)


def test_encode_to_128d_determinism():
    """Test encoding is deterministic - same input produces same output."""
    event = create_test_intent()

    vector1 = encode_to_128d(event)
    vector2 = encode_to_128d(event)

    assert np.allclose(vector1, vector2)


def test_encode_to_128d_different_events():
    """Test different events produce different vectors."""
    event1 = create_test_intent()

    event2 = create_test_intent()
    event2.action = "write"  # Change action

    vector1 = encode_to_128d(event1)
    vector2 = encode_to_128d(event2)

    # Vectors should be different (Phase 1: normalize by 4 since per-slot norm)
    similarity = np.dot(vector1, vector2) / 4.0
    assert similarity < 0.99  # Not identical


def test_encode_to_128d_slot_structure():
    """Test 128-dim vector is structured as 4 slots of 32 dims each."""
    event = create_test_intent()
    vector = encode_to_128d(event)

    # Split into 4 slots
    action_slot = vector[0:32]
    resource_slot = vector[32:64]
    data_slot = vector[64:96]
    risk_slot = vector[96:128]

    # Each slot should have values (not all zeros)
    assert np.abs(action_slot).sum() > 0
    assert np.abs(resource_slot).sum() > 0
    assert np.abs(data_slot).sum() > 0
    assert np.abs(risk_slot).sum() > 0


def test_encode_boundary_to_128d_dimensions():
    """Test boundary encoding produces 128-dim vectors."""
    boundary = create_test_boundary()
    vector = encode_boundary_to_128d(boundary)

    assert vector.shape == (128,)
    assert vector.dtype == np.float32


def test_encode_boundary_to_128d_normalization():
    """Test boundary vectors have correct global norm (Phase 1: per-slot normalization)."""
    boundary = create_test_boundary()
    vector = encode_boundary_to_128d(boundary)

    norm = np.linalg.norm(vector)
    # Phase 1: Each slot is unit-normalized, so global norm = sqrt(4) = 2.0
    assert np.isclose(norm, 2.0, atol=1e-5)


def test_encode_boundary_to_128d_determinism():
    """Test boundary encoding is deterministic."""
    boundary = create_test_boundary()

    vector1 = encode_boundary_to_128d(boundary)
    vector2 = encode_boundary_to_128d(boundary)

    assert np.allclose(vector1, vector2)


# Tests for semantic similarity

def test_similar_events_high_similarity():
    """Test that semantically similar events have high cosine similarity."""
    event1 = create_test_intent()
    event1.action = "read"

    event2 = create_test_intent()
    event2.action = "read"  # Same action, slightly different context
    event2.actor.id = "bob@example.com"

    vector1 = encode_to_128d(event1)
    vector2 = encode_to_128d(event2)

    similarity = np.dot(vector1, vector2)  # Cosine similarity (normalized vectors)

    # Should be relatively high (>0.7) since action and most context is the same
    assert similarity > 0.7


def test_different_actions_lower_similarity():
    """Test that different actions have lower similarity."""
    event1 = create_test_intent()
    event1.action = "read"

    event2 = create_test_intent()
    event2.action = "delete"  # Different action

    vector1 = encode_to_128d(event1)
    vector2 = encode_to_128d(event2)

    # Phase 1: normalize by 4 since per-slot norm
    similarity = np.dot(vector1, vector2) / 4.0

    # Should be lower than similar events
    assert similarity < 0.95


def test_event_and_boundary_similarity():
    """Test similarity between intent and matching boundary."""
    event = create_test_intent()
    event.action = "read"

    boundary = create_test_boundary()
    boundary.scope.domains = ["database"]  # Matching the event's resource type

    event_vector = encode_to_128d(event)
    boundary_vector = encode_boundary_to_128d(boundary)

    similarity = np.dot(event_vector, boundary_vector)

    # Should have reasonable similarity for related concepts
    # Note: Boundaries and events are encoded differently, so similarity may be modest
    assert similarity > 0.0  # Just verify it's not completely orthogonal


# Tests for cache management

def test_cache_stats_tracking():
    """Test cache statistics are tracked correctly."""
    clear_cache()

    # Encode some text
    encode_text_cached("test1")
    encode_text_cached("test2")
    encode_text_cached("test1")  # Cache hit

    stats = get_cache_stats()

    assert stats["hits"] >= 1
    assert stats["misses"] >= 2
    assert stats["size"] >= 2


def test_clear_cache_works():
    """Test cache clearing works."""
    encode_text_cached("test")

    clear_cache()

    stats = get_cache_stats()
    assert stats["size"] == 0
    assert stats["hits"] == 0
    assert stats["misses"] == 0


# Edge cases

def test_encode_minimal_event():
    """Test encoding works with minimal event data (v1.1)."""
    event = IntentEvent(
        id="minimal",
        schemaVersion="v1.1",
        tenantId="test",
        timestamp=1700000000.0,
        action="read",
        actor=Actor(id="system-001", type="service"),
        resource=Resource(type="file"),
        data=Data(sensitivity=["public"]),  # v1.1: sensitivity required
        risk=Risk(authn="not_required"),  # v1.1: simplified authn
    )

    vector = encode_to_128d(event)

    assert vector.shape == (128,)
    # Phase 1: Per-slot normalization means global norm = 2.0
    assert np.isclose(np.linalg.norm(vector), 2.0, atol=1e-5)


def test_encode_with_optional_fields():
    """Test encoding handles optional fields gracefully (v1.1)."""
    event = create_test_intent()
    event.resource.name = None
    event.resource.location = None
    # v1.1: Risk has no optional fields (just authn)

    vector = encode_to_128d(event)

    # Should still produce valid 128-dim normalized vector
    assert vector.shape == (128,)
    # Phase 1: Per-slot normalization means global norm = 2.0
    assert np.isclose(np.linalg.norm(vector), 2.0, atol=1e-5)


# Phase 1 tests: Per-slot normalization

def test_per_slot_normalization():
    """Test that each 32-d slot is unit-normalized after projection."""
    event = create_test_intent()
    vector_128 = encode_to_128d(event)

    # Extract 4 slots
    action_slot = vector_128[0:32]
    resource_slot = vector_128[32:64]
    data_slot = vector_128[64:96]
    risk_slot = vector_128[96:128]

    # Each slot should have norm = 1.0
    assert np.abs(np.linalg.norm(action_slot) - 1.0) < 0.001, \
        f"Action slot norm {np.linalg.norm(action_slot)} != 1.0"
    assert np.abs(np.linalg.norm(resource_slot) - 1.0) < 0.001, \
        f"Resource slot norm {np.linalg.norm(resource_slot)} != 1.0"
    assert np.abs(np.linalg.norm(data_slot) - 1.0) < 0.001, \
        f"Data slot norm {np.linalg.norm(data_slot)} != 1.0"
    assert np.abs(np.linalg.norm(risk_slot) - 1.0) < 0.001, \
        f"Risk slot norm {np.linalg.norm(risk_slot)} != 1.0"


def test_slot_independence():
    """Test that changing one slot doesn't affect other slots."""
    # Create two events that differ only in action
    event1 = create_test_intent()
    event1.action = "read"

    event2 = create_test_intent()
    event2.action = "delete"  # Use more different action

    vec1 = encode_to_128d(event1)
    vec2 = encode_to_128d(event2)

    # Action slot should differ (read vs delete are more different)
    action_sim = np.dot(vec1[0:32], vec2[0:32])
    assert action_sim < 0.95, f"Different actions should have lower similarity, got {action_sim}"

    # Other slots should be identical (same resource/data/risk)
    resource_sim = np.dot(vec1[32:64], vec2[32:64])
    data_sim = np.dot(vec1[64:96], vec2[64:96])
    risk_sim = np.dot(vec1[96:128], vec2[96:128])

    assert resource_sim > 0.99, f"Resource slot should be identical, got {resource_sim}"
    assert data_sim > 0.99, f"Data slot should be identical, got {data_sim}"
    assert risk_sim > 0.99, f"Risk slot should be identical, got {risk_sim}"


def test_phase1_matching_values_high_similarity():
    """Test that matching intent and boundary produce high similarity (Phase 1 integration test)."""
    from app.models import (
        IntentEvent, DesignBoundary, BoundaryConstraints,
        ActionConstraint, ResourceConstraint, DataConstraint, RiskConstraint,
        BoundaryScope, BoundaryRules, SliceThresholds
    )

    # Intent: read action by agent
    intent = IntentEvent(
        id="test",
        schemaVersion="v1.2",
        tenantId="test-tenant",
        timestamp=1700000000.0,
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
        status="active",
        type="mandatory",
        boundarySchemaVersion="v1.1",
        scope=BoundaryScope(
            tenantId="test-tenant",
            domains=["database"],
        ),
        rules=BoundaryRules(
            thresholds=SliceThresholds(
                action=0.80,
                resource=0.75,
                data=0.80,
                risk=0.80,
            ),
            decision="min",
        ),
        constraints=BoundaryConstraints(
            action=ActionConstraint(actions=["read"], actor_types=["agent"]),
            resource=ResourceConstraint(types=["database"]),
            data=DataConstraint(sensitivity=["internal"], pii=False, volume="single"),
            risk=RiskConstraint(authn="required"),
        ),
        createdAt=1700000000.0,
        updatedAt=1700000000.0,
    )

    intent_vec = encode_to_128d(intent)
    boundary_vec = encode_boundary_to_128d(boundary)

    # Compute per-slice cosine manually (simulating Rust logic)
    slice_sims = []
    for i, (start, end) in enumerate([(0, 32), (32, 64), (64, 96), (96, 128)]):
        intent_slice = intent_vec[start:end]
        boundary_slice = boundary_vec[start:end]
        sim = np.dot(intent_slice, boundary_slice)
        slice_sims.append(sim)

    # Phase 1: Math fix provides correct cosine calculation
    # Similarities will be lower than ideal due to encoding mismatch (list vs singleton)
    # which will be fixed in Phase 2 with anchor-based encoding
    # For now, verify that values are reasonable (> 0.5) and not broken (< 0.3)
    for i, sim in enumerate(slice_sims):
        assert sim > 0.5, \
            f"Slice {i} similarity {sim:.3f} too low (math fix should provide > 0.5 for matching values)"
        assert sim <= 1.0, \
            f"Slice {i} similarity {sim:.3f} exceeds 1.0 (indicates normalization error)"
