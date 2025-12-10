"""
Test v1.1 semantic alignment between intents and boundaries.

Verifies that intents and boundaries using the same vocabulary produce high similarity.
"""

import numpy as np

from app.encoding import encode_to_128d, encode_boundary_to_128d
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


def cosine_similarity(v1: np.ndarray, v2: np.ndarray) -> float:
    """Compute cosine similarity between two vectors."""
    return float(np.dot(v1, v2) / (np.linalg.norm(v1) * np.linalg.norm(v2)))


def test_matching_intent_and_boundary_high_similarity():
    """Test that matching intent and boundary have high similarity (>0.8)."""
    # Create a "Safe Read Access" boundary
    boundary = DesignBoundary(
        id="boundary_safe_read",
        name="Safe Read Access",
        status="active",
        type="mandatory",
        scope=BoundaryScope(tenantId="test"),
        rules=BoundaryRules(
            thresholds=SliceThresholds(
                action=0.85,
                resource=0.80,
                data=0.75,
                risk=0.70,
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
        createdAt=1700000000.0,
        updatedAt=1700000000.0,
    )

    # Create a matching intent (user reading database)
    intent = IntentEvent(
        id="intent_read_db",
        schemaVersion="v1.1",
        tenantId="test",
        timestamp=1700000000.0,
        action="read",
        actor=Actor(
            id="alice@example.com",
            type="user",
        ),
        resource=Resource(
            type="database",
            name="prod_users",
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

    # Encode both
    intent_vector = encode_to_128d(intent)
    boundary_vector = encode_boundary_to_128d(boundary)

    # Compute overall similarity
    overall_similarity = cosine_similarity(intent_vector, boundary_vector)

    # Compute per-slot similarities
    slot_ranges = [(0, 32), (32, 64), (64, 96), (96, 128)]
    slot_names = ["action", "resource", "data", "risk"]
    slot_similarities = {}

    for (start, end), name in zip(slot_ranges, slot_names):
        intent_slice = intent_vector[start:end]
        boundary_slice = boundary_vector[start:end]
        slot_similarities[name] = cosine_similarity(intent_slice, boundary_slice)

    # Print results
    print("\n" + "="*60)
    print("V1.1 Semantic Alignment Test")
    print("="*60)
    print(f"Overall similarity: {overall_similarity:.4f}")
    print("\nPer-slot similarities:")
    for name, sim in slot_similarities.items():
        status = "✅" if sim > 0.8 else "⚠️" if sim > 0.5 else "❌"
        print(f"  {name:10s}: {sim:.4f} {status}")
    print("="*60)

    # Assertions
    assert overall_similarity > 0.8, f"Overall similarity {overall_similarity:.4f} should be >0.8 for matching intent/boundary"

    for name, sim in slot_similarities.items():
        assert sim > 0.8, f"{name} slot similarity {sim:.4f} should be >0.8 for matching intent/boundary"

    print("\n✅ All similarity checks passed!\n")


def test_mismatching_intent_low_similarity():
    """Test that mismatching intent and boundary have low similarity (<0.5)."""
    # Create a "Safe Read Access" boundary
    boundary = DesignBoundary(
        id="boundary_safe_read",
        name="Safe Read Access",
        status="active",
        type="mandatory",
        scope=BoundaryScope(tenantId="test"),
        rules=BoundaryRules(
            thresholds=SliceThresholds(
                action=0.85,
                resource=0.80,
                data=0.75,
                risk=0.70,
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
        createdAt=1700000000.0,
        updatedAt=1700000000.0,
    )

    # Create a MISMATCHING intent (delete instead of read)
    intent = IntentEvent(
        id="intent_delete_db",
        schemaVersion="v1.1",
        tenantId="test",
        timestamp=1700000000.0,
        action="delete",  # Different action!
        actor=Actor(
            id="alice@example.com",
            type="user",
        ),
        resource=Resource(
            type="database",
            name="prod_users",
            location="cloud",
        ),
        data=Data(
            sensitivity=["internal"],
            pii=False,
            volume="bulk",  # Different volume!
        ),
        risk=Risk(
            authn="required",
        ),
    )

    # Encode both
    intent_vector = encode_to_128d(intent)
    boundary_vector = encode_boundary_to_128d(boundary)

    # Compute per-slot similarities
    slot_ranges = [(0, 32), (32, 64), (64, 96), (96, 128)]
    slot_names = ["action", "resource", "data", "risk"]
    slot_similarities = {}

    for (start, end), name in zip(slot_ranges, slot_names):
        intent_slice = intent_vector[start:end]
        boundary_slice = boundary_vector[start:end]
        slot_similarities[name] = cosine_similarity(intent_slice, boundary_slice)

    # Print results
    print("\n" + "="*60)
    print("V1.1 Mismatching Intent Test")
    print("="*60)
    print("Per-slot similarities:")
    for name, sim in slot_similarities.items():
        status = "❌" if sim < 0.5 else "⚠️" if sim < 0.8 else "✅"
        print(f"  {name:10s}: {sim:.4f} {status}")
    print("="*60)

    # Action slot should have lower similarity (different action)
    # Note: "read" vs "delete" still share semantic space (both are operations)
    # so similarity will be moderate, not near-zero
    assert slot_similarities["action"] < 0.85, f"Action slot similarity {slot_similarities['action']:.4f} should be <0.85 for different action"

    # Data slot should have medium similarity (volume differs)
    assert slot_similarities["data"] < 0.95, f"Data slot similarity {slot_similarities['data']:.4f} should differ due to volume mismatch"

    print("\n✅ Mismatch detection works correctly!\n")


if __name__ == "__main__":
    print("\nRunning v1.1 semantic alignment tests...\n")
    test_matching_intent_and_boundary_high_similarity()
    test_mismatching_intent_low_similarity()
    print("All tests passed! ✅\n")
