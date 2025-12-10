"""
Unit tests for deny-first aggregation semantics.

Tests the deny-first policy aggregation logic in the comparison endpoint:
1. Deny boundaries are checked first - any match causes immediate BLOCK
2. Allow boundaries are checked second - all mandatory must pass for ALLOW
3. Deny wins over allow when both match

This validates Issue #3 fix from fix_prod_run_plan_1_0.md.
"""

import pytest
import numpy as np
from unittest.mock import Mock, patch, MagicMock
from app.models import (
    IntentEvent,
    DesignBoundary,
    Actor,
    Resource,
    Data,
    Risk,
    BoundaryScope,
    BoundaryRules,
    SliceThresholds,
    BoundaryConstraints,
    ActionConstraint,
    ResourceConstraint,
    DataConstraint,
    RiskConstraint,
    ComparisonResult,
)


# ============================================================================
# Test Fixtures
# ============================================================================

@pytest.fixture
def sample_intent() -> IntentEvent:
    """Create a sample intent (delete operation by agent)."""
    return IntentEvent(
        id="test_intent_delete",
        schemaVersion="v1.2",
        tenantId="tenant_test",
        timestamp=1700000000.0,
        actor=Actor(id="agent_123", type="agent"),
        action="delete",
        resource=Resource(type="database", name="users_db", location="cloud"),
        data=Data(sensitivity=["internal"], pii=True, volume="bulk"),
        risk=Risk(authn="required"),
    )


@pytest.fixture
def allow_boundary() -> DesignBoundary:
    """Create an allow boundary for delete operations."""
    return DesignBoundary(
        id="allow_delete",
        name="Allow Delete Operations",
        status="active",
        type="mandatory",
        boundarySchemaVersion="v1.2",
        scope=BoundaryScope(tenantId="tenant_test"),
        rules=BoundaryRules(
            effect="allow",
            thresholds=SliceThresholds(action=0.8, resource=0.75, data=0.7, risk=0.6),
            decision="min",
        ),
        constraints=BoundaryConstraints(
            action=ActionConstraint(actions=["delete"], actor_types=["agent"]),
            resource=ResourceConstraint(types=["database"], locations=["cloud"]),
            data=DataConstraint(sensitivity=["internal"], pii=True, volume="bulk"),
            risk=RiskConstraint(authn="required"),
        ),
        createdAt=1700000000.0,
        updatedAt=1700000000.0,
    )


@pytest.fixture
def deny_boundary() -> DesignBoundary:
    """Create a deny boundary for delete operations."""
    return DesignBoundary(
        id="deny_delete",
        name="Deny Delete Operations",
        status="active",
        type="mandatory",
        boundarySchemaVersion="v1.2",
        scope=BoundaryScope(tenantId="tenant_test"),
        rules=BoundaryRules(
            effect="deny",
            thresholds=SliceThresholds(action=0.8, resource=0.75, data=0.7, risk=0.6),
            decision="min",
        ),
        constraints=BoundaryConstraints(
            action=ActionConstraint(actions=["delete"], actor_types=["agent"]),
            resource=ResourceConstraint(types=["database"], locations=["cloud"]),
            data=DataConstraint(sensitivity=["internal"], pii=True, volume="bulk"),
            risk=RiskConstraint(authn="required"),
        ),
        createdAt=1700000000.0,
        updatedAt=1700000000.0,
    )


# ============================================================================
# Integration Tests (with endpoint)
# ============================================================================

@pytest.mark.asyncio
async def test_deny_boundary_match_immediate_block(sample_intent, deny_boundary):
    """Test that a deny boundary match causes immediate BLOCK."""
    from app.endpoints.intents import compare_intent
    from app.endpoints.boundaries import _boundaries_store

    # Mock the encoding and FFI sandbox to return high similarities (match)
    with patch("app.endpoints.intents.encode_to_128d") as mock_encode, \
         patch("app.endpoints.intents.get_sandbox") as mock_sandbox:

        # Setup mocks
        mock_encode.return_value = np.array([0.5] * 128, dtype=np.float32)  # Dummy vector
        mock_sandbox_instance = Mock()
        mock_sandbox.return_value = mock_sandbox_instance

        # Mock sandbox.compare to return ALLOW (decision=1) with high similarities
        # For deny boundary, decision=1 means "matches deny criteria" → BLOCK
        mock_sandbox_instance.compare.return_value = (
            1,  # decision=1 (matches)
            [0.95, 0.90, 0.88, 0.92]  # High similarities
        )

        # Clear and seed boundary store
        _boundaries_store.clear()
        _boundaries_store[deny_boundary.id] = deny_boundary

        # Call endpoint
        result = await compare_intent(sample_intent)

        # Verify: deny match should cause BLOCK
        assert result.decision == 0, "Deny boundary match should cause BLOCK"
        assert result.slice_similarities == [0.95, 0.90, 0.88, 0.92]

        # Verify sandbox was called once (deny check, then short-circuit)
        assert mock_sandbox_instance.compare.call_count == 1


@pytest.mark.asyncio
async def test_deny_boundary_no_match_check_allow(sample_intent, deny_boundary, allow_boundary):
    """Test that if deny doesn't match, allow boundaries are checked."""
    from app.endpoints.intents import compare_intent
    from app.endpoints.boundaries import _boundaries_store

    with patch("app.endpoints.intents.encode_to_128d") as mock_encode, \
         patch("app.endpoints.intents.get_sandbox") as mock_sandbox:

        mock_encode.return_value = np.array([0.5] * 128, dtype=np.float32)
        mock_sandbox_instance = Mock()
        mock_sandbox.return_value = mock_sandbox_instance

        # First call (deny boundary): decision=0 (no match)
        # Second call (allow boundary): decision=1 (match)
        mock_sandbox_instance.compare.side_effect = [
            (0, [0.50, 0.60, 0.55, 0.58]),  # Deny doesn't match (low similarities)
            (1, [0.95, 0.90, 0.88, 0.92]),  # Allow matches
        ]

        # Seed both boundaries
        _boundaries_store.clear()
        _boundaries_store[deny_boundary.id] = deny_boundary
        _boundaries_store[allow_boundary.id] = allow_boundary

        result = await compare_intent(sample_intent)

        # Verify: deny didn't match, allow matched → ALLOW
        assert result.decision == 1, "Allow boundary should permit operation"
        # Similarities should be from allow boundary (averaged, but only one boundary)
        assert result.slice_similarities == [0.95, 0.90, 0.88, 0.92]

        # Verify sandbox was called twice (deny + allow)
        assert mock_sandbox_instance.compare.call_count == 2


@pytest.mark.asyncio
async def test_deny_wins_over_allow_when_both_match(sample_intent, deny_boundary, allow_boundary):
    """Test that deny takes precedence when both deny and allow match."""
    from app.endpoints.intents import compare_intent
    from app.endpoints.boundaries import _boundaries_store

    with patch("app.endpoints.intents.encode_to_128d") as mock_encode, \
         patch("app.endpoints.intents.get_sandbox") as mock_sandbox:

        mock_encode.return_value = np.array([0.5] * 128, dtype=np.float32)
        mock_sandbox_instance = Mock()
        mock_sandbox.return_value = mock_sandbox_instance

        # Both boundaries match (high similarities)
        # But deny is checked first and should short-circuit
        mock_sandbox_instance.compare.return_value = (
            1,  # Both match
            [0.95, 0.90, 0.88, 0.92]
        )

        # Seed both boundaries
        _boundaries_store.clear()
        _boundaries_store[deny_boundary.id] = deny_boundary
        _boundaries_store[allow_boundary.id] = allow_boundary

        result = await compare_intent(sample_intent)

        # Verify: deny match should cause BLOCK
        assert result.decision == 0, "Deny should win when both match"

        # Current implementation: Evaluates all applicable boundaries, then separates by effect
        # This is functionally correct but not optimally efficient
        # TODO: After implementing deny short-circuit optimization (see docs/plans/deny_short_circuit_optimization.md),
        #       change this to assert call_count == 1 (only deny evaluated, then short-circuit)
        assert mock_sandbox_instance.compare.call_count == 2  # Both boundaries evaluated


@pytest.mark.asyncio
async def test_multiple_allow_boundaries_all_must_pass():
    """Test that all mandatory allow boundaries must pass for ALLOW."""
    from app.endpoints.intents import compare_intent
    from app.endpoints.boundaries import _boundaries_store

    intent = IntentEvent(
        id="test_intent_read",
        schemaVersion="v1.2",
        tenantId="tenant_test",
        timestamp=1700000000.0,
        actor=Actor(id="user_123", type="user"),
        action="read",
        resource=Resource(type="database", name="users_db", location="cloud"),
        data=Data(sensitivity=["internal"], pii=False, volume="single"),
        risk=Risk(authn="required"),
    )

    allow_boundary_1 = DesignBoundary(
        id="allow_read_1",
        name="Allow Read Policy 1",
        status="active",
        type="mandatory",
        boundarySchemaVersion="v1.2",
        scope=BoundaryScope(tenantId="tenant_test"),
        rules=BoundaryRules(
            effect="allow",
            thresholds=SliceThresholds(action=0.8, resource=0.75, data=0.7, risk=0.6),
            decision="min",
        ),
        constraints=BoundaryConstraints(
            action=ActionConstraint(actions=["read"], actor_types=["user"]),
            resource=ResourceConstraint(types=["database"]),
            data=DataConstraint(sensitivity=["internal"]),
            risk=RiskConstraint(authn="required"),
        ),
        createdAt=1700000000.0,
        updatedAt=1700000000.0,
    )

    allow_boundary_2 = DesignBoundary(
        id="allow_read_2",
        name="Allow Read Policy 2",
        status="active",
        type="mandatory",
        boundarySchemaVersion="v1.2",
        scope=BoundaryScope(tenantId="tenant_test"),
        rules=BoundaryRules(
            effect="allow",
            thresholds=SliceThresholds(action=0.8, resource=0.75, data=0.7, risk=0.6),
            decision="min",
        ),
        constraints=BoundaryConstraints(
            action=ActionConstraint(actions=["read"], actor_types=["user"]),
            resource=ResourceConstraint(types=["database"]),
            data=DataConstraint(sensitivity=["internal"]),
            risk=RiskConstraint(authn="required"),
        ),
        createdAt=1700000000.0,
        updatedAt=1700000000.0,
    )

    with patch("app.endpoints.intents.encode_to_128d") as mock_encode, \
         patch("app.endpoints.intents.get_sandbox") as mock_sandbox:

        mock_encode.return_value = np.array([0.5] * 128, dtype=np.float32)
        mock_sandbox_instance = Mock()
        mock_sandbox.return_value = mock_sandbox_instance

        # First boundary passes, second fails
        mock_sandbox_instance.compare.side_effect = [
            (1, [0.95, 0.90, 0.88, 0.92]),  # Boundary 1: PASS
            (0, [0.70, 0.60, 0.65, 0.68]),  # Boundary 2: FAIL
        ]

        _boundaries_store.clear()
        _boundaries_store[allow_boundary_1.id] = allow_boundary_1
        _boundaries_store[allow_boundary_2.id] = allow_boundary_2

        result = await compare_intent(intent)

        # Verify: one allow boundary failed → BLOCK
        assert result.decision == 0, "Should BLOCK when any mandatory allow boundary fails"
        # Should use minimum similarities across boundaries
        assert result.slice_similarities == [0.70, 0.60, 0.65, 0.68]


@pytest.mark.asyncio
async def test_no_mandatory_allow_boundaries_defaults_to_block():
    """Test that no mandatory allow boundaries results in BLOCK (fail-closed)."""
    from app.endpoints.intents import compare_intent
    from app.endpoints.boundaries import _boundaries_store

    intent = IntentEvent(
        id="test_intent_execute",
        schemaVersion="v1.2",
        tenantId="tenant_test",
        timestamp=1700000000.0,
        actor=Actor(id="agent_123", type="agent"),
        action="execute",
        resource=Resource(type="api", name="run_command"),
        data=Data(sensitivity=["internal"], pii=False, volume="single"),
        risk=Risk(authn="required"),
    )

    # Create only a deny boundary (no allow boundaries)
    deny_boundary = DesignBoundary(
        id="deny_execute",
        name="Deny Execute",
        status="active",
        type="mandatory",
        boundarySchemaVersion="v1.2",
        scope=BoundaryScope(tenantId="tenant_test"),
        rules=BoundaryRules(
            effect="deny",
            thresholds=SliceThresholds(action=0.8, resource=0.75, data=0.7, risk=0.6),
            decision="min",
        ),
        constraints=BoundaryConstraints(
            action=ActionConstraint(actions=["delete"], actor_types=["agent"]),  # Different action
            resource=ResourceConstraint(types=["api"]),
            data=DataConstraint(sensitivity=["internal"]),
            risk=RiskConstraint(authn="required"),
        ),
        createdAt=1700000000.0,
        updatedAt=1700000000.0,
    )

    with patch("app.endpoints.intents.encode_to_128d") as mock_encode, \
         patch("app.endpoints.intents.get_sandbox") as mock_sandbox:

        mock_encode.return_value = np.array([0.5] * 128, dtype=np.float32)
        mock_sandbox_instance = Mock()
        mock_sandbox.return_value = mock_sandbox_instance

        # Deny boundary doesn't match (action mismatch)
        mock_sandbox_instance.compare.return_value = (
            0,  # No match
            [0.50, 0.60, 0.55, 0.58]
        )

        _boundaries_store.clear()
        _boundaries_store[deny_boundary.id] = deny_boundary

        result = await compare_intent(intent)

        # Verify: no deny match, but no allow boundaries → BLOCK (fail-closed)
        assert result.decision == 0, "Should BLOCK when no mandatory allow boundaries exist"
        assert result.slice_similarities == [0.0, 0.0, 0.0, 0.0]


@pytest.mark.asyncio
async def test_optional_boundaries_ignored_in_mvp():
    """Test that optional boundaries are currently ignored (MVP scope)."""
    from app.endpoints.intents import compare_intent
    from app.endpoints.boundaries import _boundaries_store

    intent = IntentEvent(
        id="test_intent_read",
        schemaVersion="v1.2",
        tenantId="tenant_test",
        timestamp=1700000000.0,
        actor=Actor(id="user_123", type="user"),
        action="read",
        resource=Resource(type="database"),
        data=Data(sensitivity=["internal"]),
        risk=Risk(authn="required"),
    )

    # Only optional boundary, no mandatory
    optional_boundary = DesignBoundary(
        id="optional_read",
        name="Optional Read Policy",
        status="active",
        type="optional",  # Optional, not mandatory
        boundarySchemaVersion="v1.2",
        scope=BoundaryScope(tenantId="tenant_test"),
        rules=BoundaryRules(
            effect="allow",
            thresholds=SliceThresholds(action=0.8, resource=0.75, data=0.7, risk=0.6),
            decision="min",
        ),
        constraints=BoundaryConstraints(
            action=ActionConstraint(actions=["read"], actor_types=["user"]),
            resource=ResourceConstraint(types=["database"]),
            data=DataConstraint(sensitivity=["internal"]),
            risk=RiskConstraint(authn="required"),
        ),
        createdAt=1700000000.0,
        updatedAt=1700000000.0,
    )

    with patch("app.endpoints.intents.encode_to_128d") as mock_encode, \
         patch("app.endpoints.intents.get_sandbox") as mock_sandbox:

        mock_encode.return_value = np.array([0.5] * 128, dtype=np.float32)
        mock_sandbox_instance = Mock()
        mock_sandbox.return_value = mock_sandbox_instance

        mock_sandbox_instance.compare.return_value = (
            1,  # Match
            [0.95, 0.90, 0.88, 0.92]
        )

        _boundaries_store.clear()
        _boundaries_store[optional_boundary.id] = optional_boundary

        result = await compare_intent(intent)

        # Verify: no mandatory boundaries → BLOCK (fail-closed)
        assert result.decision == 0, "Should BLOCK when only optional boundaries exist (MVP)"
        assert result.slice_similarities == [0.0, 0.0, 0.0, 0.0]


@pytest.mark.asyncio
async def test_no_applicable_boundaries_defaults_to_block():
    """Test that when no boundaries are applicable, default is BLOCK (fail-closed)."""
    from app.endpoints.intents import compare_intent
    from app.endpoints.boundaries import _boundaries_store

    # Intent with action "execute"
    intent = IntentEvent(
        id="test_intent_execute",
        schemaVersion="v1.2",
        tenantId="tenant_test",
        timestamp=1700000000.0,
        actor=Actor(id="user_123", type="user"),
        action="execute",  # Different action
        resource=Resource(type="database"),
        data=Data(sensitivity=["internal"]),
        risk=Risk(authn="required"),
    )

    # Boundary only for "read" action
    read_boundary = DesignBoundary(
        id="read_only",
        name="Read Only Policy",
        status="active",
        type="mandatory",
        boundarySchemaVersion="v1.2",
        scope=BoundaryScope(tenantId="tenant_test"),
        rules=BoundaryRules(
            effect="allow",
            thresholds=SliceThresholds(action=0.8, resource=0.75, data=0.7, risk=0.6),
            decision="min",
        ),
        constraints=BoundaryConstraints(
            action=ActionConstraint(actions=["read"], actor_types=["user"]),  # Only read
            resource=ResourceConstraint(types=["database"]),
            data=DataConstraint(sensitivity=["internal"]),
            risk=RiskConstraint(authn="required"),
        ),
        createdAt=1700000000.0,
        updatedAt=1700000000.0,
    )

    with patch("app.endpoints.intents.encode_to_128d") as mock_encode:
        mock_encode.return_value = np.array([0.5] * 128, dtype=np.float32)

        _boundaries_store.clear()
        _boundaries_store[read_boundary.id] = read_boundary

        result = await compare_intent(intent)

        # Verify: no applicable boundaries (action mismatch) → BLOCK (fail-closed)
        assert result.decision == 0, "Should BLOCK when no boundaries are applicable"
        assert result.slice_similarities == [0.0, 0.0, 0.0, 0.0]
