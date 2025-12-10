"""
Test suite for data contract type definitions.

Validates:
1. Model instantiation with valid data
2. Validation rules (field constraints, literal values)
3. JSON serialization/deserialization
4. Edge cases and error conditions
"""

import pytest
import json
from pydantic import ValidationError

import sys
from pathlib import Path

# Add parent directory to path to import app module
sys.path.insert(0, str(Path(__file__).parent.parent))

from app.models import (
    Actor,
    Resource,
    Data,
    Risk,
    IntentEvent,
    BoundaryScope,
    SliceThresholds,
    SliceWeights,
    BoundaryRules,
    DesignBoundary,
    ComparisonResult,
    ActionConstraint,
    ResourceConstraint,
    DataConstraint,
    RiskConstraint,
    BoundaryConstraints,
)


# ============================================================================
# Test IntentEvent Components
# ============================================================================

class TestActor:
    def test_valid_user_actor(self):
        actor = Actor(id="user-123", type="user")
        assert actor.id == "user-123"
        assert actor.type == "user"

    def test_valid_service_actor(self):
        actor = Actor(id="service-456", type="service")
        assert actor.id == "service-456"
        assert actor.type == "service"

    def test_invalid_actor_type(self):
        with pytest.raises(ValidationError):
            Actor(id="bad-actor", type="robot")

    def test_valid_llm_actor(self):
        """Test v1.2: LLM actor type for language models"""
        actor = Actor(id="llm-gpt4", type="llm")
        assert actor.id == "llm-gpt4"
        assert actor.type == "llm"

    def test_valid_agent_actor(self):
        """Test v1.2: Agent actor type for AI agents"""
        actor = Actor(id="agent-123", type="agent")
        assert actor.id == "agent-123"
        assert actor.type == "agent"


class TestResource:
    def test_minimal_resource(self):
        resource = Resource(type="database")
        assert resource.type == "database"
        assert resource.name is None
        assert resource.location is None

    def test_full_resource(self):
        resource = Resource(
            type="database",
            name="users_db",
            location="cloud"
        )
        assert resource.type == "database"
        assert resource.name == "users_db"
        assert resource.location == "cloud"

    def test_resource_missing_required_field(self):
        with pytest.raises(ValidationError):
            Resource(name="users_db")


class TestData:
    def test_minimal_data(self):
        """Test v1.1/v1.2: Data with sensitivity field"""
        data = Data(sensitivity=["internal"])
        assert data.sensitivity == ["internal"]
        assert data.pii is None
        assert data.volume is None

    def test_full_data(self):
        """Test v1.1/v1.2: Data with all fields"""
        data = Data(
            sensitivity=["internal", "public"],
            pii=True,
            volume="bulk"
        )
        assert data.sensitivity == ["internal", "public"]
        assert data.pii is True
        assert data.volume == "bulk"

    def test_invalid_volume(self):
        with pytest.raises(ValidationError):
            Data(sensitivity=["internal"], volume="invalid")

    def test_empty_sensitivity(self):
        """Test v1.1/v1.2: Empty sensitivity list is valid"""
        data = Data(sensitivity=[])
        assert data.sensitivity == []


class TestRisk:
    def test_required_authn(self):
        """Test v1.1/v1.2: Risk with required authentication"""
        risk = Risk(authn="required")
        assert risk.authn == "required"

    def test_not_required_authn(self):
        """Test v1.1/v1.2: Risk with not_required authentication"""
        risk = Risk(authn="not_required")
        assert risk.authn == "not_required"

    def test_invalid_authn(self):
        """Test v1.1/v1.2: Invalid authn value should fail"""
        with pytest.raises(ValidationError):
            Risk(authn="invalid")


class TestIntentEvent:
    def test_minimal_intent_event(self):
        """Test v1.3: Minimal IntentEvent with defaults"""
        event = IntentEvent(
            id="550e8400-e29b-41d4-a716-446655440000",
            tenantId="tenant-123",
            timestamp=1699564800.0,
            actor=Actor(id="user-123", type="user"),
            action="read",
            resource=Resource(type="database"),
            data=Data(sensitivity=["internal"]),
            risk=Risk(authn="required"),
        )
        assert event.schemaVersion == "v1.3"  # v1.3 is now default
        assert event.context is None

    def test_full_intent_event(self):
        """Test v1.2: Full IntentEvent with all fields"""
        event = IntentEvent(
            id="550e8400-e29b-41d4-a716-446655440000",
            schemaVersion="v1.2",
            tenantId="tenant-123",
            timestamp=1699564800.0,
            actor=Actor(id="user-123", type="user"),
            action="read",
            resource=Resource(type="database", name="users_db", location="cloud"),
            data=Data(sensitivity=["internal"], pii=True, volume="single"),
            risk=Risk(authn="required"),
            context={"request_id": "req-123"},
        )
        assert event.id == "550e8400-e29b-41d4-a716-446655440000"
        assert event.context == {"request_id": "req-123"}

    def test_invalid_action(self):
        """Test v1.2: Invalid action should fail validation"""
        with pytest.raises(ValidationError):
            IntentEvent(
                id="550e8400-e29b-41d4-a716-446655440000",
                tenantId="tenant-123",
                timestamp=1699564800.0,
                actor=Actor(id="user-123", type="user"),
                action="invalid_action",
                resource=Resource(type="database"),
                data=Data(sensitivity=["internal"]),
                risk=Risk(authn="required"),
            )

    def test_intent_event_json_serialization(self):
        """Test v1.2: IntentEvent JSON serialization/deserialization"""
        event = IntentEvent(
            id="550e8400-e29b-41d4-a716-446655440000",
            tenantId="tenant-123",
            timestamp=1699564800.0,
            actor=Actor(id="user-123", type="user"),
            action="read",
            resource=Resource(type="database", name="users_db"),
            data=Data(sensitivity=["internal"]),
            risk=Risk(authn="required"),
        )

        # Serialize to JSON
        json_str = event.model_dump_json()
        data = json.loads(json_str)

        # Deserialize back
        event2 = IntentEvent.model_validate(data)

        assert event.id == event2.id
        assert event.action == event2.action
        assert event.resource.name == event2.resource.name

    def test_intent_event_v1_2_with_llm_actor(self):
        """Test v1.2: IntentEvent with LLM actor type"""
        event = IntentEvent(
            id="550e8400-e29b-41d4-a716-446655440000",
            schemaVersion="v1.2",
            tenantId="tenant-123",
            timestamp=1699564800.0,
            actor=Actor(id="llm-gpt4", type="llm"),
            action="read",
            resource=Resource(type="api"),
            data=Data(sensitivity=["public"], pii=False, volume="single"),
            risk=Risk(authn="required"),
        )
        assert event.schemaVersion == "v1.2"
        assert event.actor.type == "llm"
        assert event.actor.id == "llm-gpt4"

    def test_intent_event_v1_2_with_agent_actor(self):
        """Test v1.2: IntentEvent with agent actor type"""
        event = IntentEvent(
            id="550e8400-e29b-41d4-a716-446655440001",
            schemaVersion="v1.2",
            tenantId="tenant-123",
            timestamp=1699564800.0,
            actor=Actor(id="agent-123", type="agent"),
            action="delete",
            resource=Resource(type="database", name="users_db"),
            data=Data(sensitivity=["internal"], pii=True, volume="single"),
            risk=Risk(authn="required"),
        )
        assert event.schemaVersion == "v1.2"
        assert event.actor.type == "agent"
        assert event.action == "delete"


# ============================================================================
# Test DesignBoundary Components
# ============================================================================

class TestBoundaryScope:
    def test_minimal_scope(self):
        scope = BoundaryScope(tenantId="tenant-123")
        assert scope.tenantId == "tenant-123"
        assert scope.domains is None

    def test_full_scope(self):
        scope = BoundaryScope(
            tenantId="tenant-123",
            domains=["database", "file"]
        )
        assert scope.tenantId == "tenant-123"
        assert scope.domains == ["database", "file"]


class TestSliceThresholds:
    def test_valid_thresholds(self):
        thresholds = SliceThresholds(
            action=0.85,
            resource=0.80,
            data=0.75,
            risk=0.70
        )
        assert thresholds.action == 0.85
        assert thresholds.resource == 0.80
        assert thresholds.data == 0.75
        assert thresholds.risk == 0.70

    def test_threshold_range_valid(self):
        thresholds = SliceThresholds(
            action=0.0,
            resource=0.5,
            data=1.0,
            risk=0.99
        )
        assert thresholds.action == 0.0
        assert thresholds.data == 1.0

    def test_threshold_range_invalid(self):
        with pytest.raises(ValidationError):
            SliceThresholds(action=-0.1, resource=0.5, data=0.5, risk=0.5)
        with pytest.raises(ValidationError):
            SliceThresholds(action=0.5, resource=1.1, data=0.5, risk=0.5)


class TestSliceWeights:
    def test_default_weights(self):
        weights = SliceWeights()
        assert weights.action == 1.0
        assert weights.resource == 1.0
        assert weights.data == 1.0
        assert weights.risk == 1.0

    def test_custom_weights(self):
        weights = SliceWeights(
            action=1.0,
            resource=1.5,
            data=2.0,
            risk=0.5
        )
        assert weights.resource == 1.5
        assert weights.data == 2.0
        assert weights.risk == 0.5

    def test_weight_negative_invalid(self):
        with pytest.raises(ValidationError):
            SliceWeights(action=-1.0)


class TestBoundaryRules:
    def test_min_mode_rules(self):
        rules = BoundaryRules(
            thresholds=SliceThresholds(
                action=0.85,
                resource=0.80,
                data=0.75,
                risk=0.70
            ),
            decision="min"
        )
        assert rules.decision == "min"
        assert rules.weights is None
        assert rules.globalThreshold is None

    def test_weighted_avg_rules(self):
        rules = BoundaryRules(
            thresholds=SliceThresholds(
                action=0.85,
                resource=0.80,
                data=0.75,
                risk=0.70
            ),
            weights=SliceWeights(
                action=1.0,
                resource=1.0,
                data=1.5,
                risk=0.5
            ),
            decision="weighted-avg",
            globalThreshold=0.78
        )
        assert rules.decision == "weighted-avg"
        assert rules.globalThreshold == 0.78

    def test_invalid_decision_mode(self):
        with pytest.raises(ValidationError):
            BoundaryRules(
                thresholds=SliceThresholds(
                    action=0.85,
                    resource=0.80,
                    data=0.75,
                    risk=0.70
                ),
                decision="invalid"
            )


class TestDesignBoundary:
    def test_minimal_boundary(self):
        """Test v1.2: Minimal DesignBoundary with constraints"""
        boundary = DesignBoundary(
            id="boundary-001",
            name="Test Boundary",
            status="active",
            type="mandatory",
            scope=BoundaryScope(tenantId="tenant-123"),
            rules=BoundaryRules(
                thresholds=SliceThresholds(
                    action=0.85,
                    resource=0.80,
                    data=0.75,
                    risk=0.70
                ),
                decision="min"
            ),
            constraints=BoundaryConstraints(
                action=ActionConstraint(actions=["read"], actor_types=["user"]),
                resource=ResourceConstraint(types=["database"]),
                data=DataConstraint(sensitivity=["internal"]),
                risk=RiskConstraint(authn="required")
            ),
            createdAt=1699564800.0,
            updatedAt=1699564800.0
        )
        assert boundary.boundarySchemaVersion == "v1.2"  # v1.2 is now default
        assert boundary.notes is None

    def test_full_boundary(self):
        """Test v1.2: Full DesignBoundary with all fields"""
        boundary = DesignBoundary(
            id="boundary-001",
            name="Prevent PII exports",
            status="active",
            type="mandatory",
            boundarySchemaVersion="v1.2",
            scope=BoundaryScope(
                tenantId="tenant-123",
                domains=["database"]
            ),
            rules=BoundaryRules(
                thresholds=SliceThresholds(
                    action=0.85,
                    resource=0.80,
                    data=0.75,
                    risk=0.70
                ),
                decision="min"
            ),
            constraints=BoundaryConstraints(
                action=ActionConstraint(actions=["read"], actor_types=["user", "llm"]),
                resource=ResourceConstraint(types=["database"], names=["prod_db"], locations=["cloud"]),
                data=DataConstraint(sensitivity=["internal"], pii=True, volume="single"),
                risk=RiskConstraint(authn="required")
            ),
            notes="Block all export operations on PII data",
            createdAt=1699564800.0,
            updatedAt=1699564800.0
        )
        assert boundary.name == "Prevent PII exports"
        assert boundary.notes == "Block all export operations on PII data"

    def test_boundary_json_serialization(self):
        """Test v1.2: DesignBoundary JSON serialization/deserialization"""
        boundary = DesignBoundary(
            id="boundary-001",
            name="Test Boundary",
            status="active",
            type="mandatory",
            scope=BoundaryScope(tenantId="tenant-123"),
            rules=BoundaryRules(
                thresholds=SliceThresholds(
                    action=0.85,
                    resource=0.80,
                    data=0.75,
                    risk=0.70
                ),
                decision="min"
            ),
            constraints=BoundaryConstraints(
                action=ActionConstraint(actions=["read"], actor_types=["agent"]),
                resource=ResourceConstraint(types=["database"]),
                data=DataConstraint(sensitivity=["internal"]),
                risk=RiskConstraint(authn="required")
            ),
            createdAt=1699564800.0,
            updatedAt=1699564800.0
        )

        # Serialize to JSON
        json_str = boundary.model_dump_json()
        data = json.loads(json_str)

        # Deserialize back
        boundary2 = DesignBoundary.model_validate(data)

        assert boundary.id == boundary2.id
        assert boundary.name == boundary2.name
        assert boundary.rules.thresholds.action == boundary2.rules.thresholds.action


# ============================================================================
# Test v1.1 Boundary Constraints
# ============================================================================

class TestActionConstraint:
    def test_action_constraint_with_user_and_service(self):
        """Test v1.1: ActionConstraint with original actor types"""
        constraint = ActionConstraint(
            actions=["read", "write"],
            actor_types=["user", "service"]
        )
        assert constraint.actions == ["read", "write"]
        assert constraint.actor_types == ["user", "service"]

    def test_action_constraint_with_new_actor_types(self):
        """Test v1.2: ActionConstraint with llm and agent actor types"""
        constraint = ActionConstraint(
            actions=["read", "execute"],
            actor_types=["llm", "agent"]
        )
        assert constraint.actions == ["read", "execute"]
        assert constraint.actor_types == ["llm", "agent"]

    def test_action_constraint_with_mixed_actor_types(self):
        """Test v1.2: ActionConstraint with all four actor types"""
        constraint = ActionConstraint(
            actions=["read"],
            actor_types=["user", "service", "llm", "agent"]
        )
        assert len(constraint.actor_types) == 4
        assert "llm" in constraint.actor_types
        assert "agent" in constraint.actor_types


# ============================================================================
# Test FFI Boundary Types
# ============================================================================

class TestComparisonResult:
    def test_allow_result(self):
        result = ComparisonResult(
            decision=1,
            slice_similarities=[0.92, 0.88, 0.85, 0.90]
        )
        assert result.decision == 1
        assert len(result.slice_similarities) == 4

    def test_block_result(self):
        result = ComparisonResult(
            decision=0,
            slice_similarities=[0.92, 0.72, 0.85, 0.90]
        )
        assert result.decision == 0

    def test_invalid_decision(self):
        with pytest.raises(ValidationError):
            ComparisonResult(
                decision=2,
                slice_similarities=[0.92, 0.88, 0.85, 0.90]
            )

    def test_invalid_similarities_length(self):
        with pytest.raises(ValidationError):
            ComparisonResult(
                decision=1,
                slice_similarities=[0.92, 0.88]  # Only 2 elements
            )

    def test_comparison_result_json_serialization(self):
        result = ComparisonResult(
            decision=1,
            slice_similarities=[0.92, 0.88, 0.85, 0.90]
        )

        # Serialize to JSON
        json_str = result.model_dump_json()
        data = json.loads(json_str)

        # Deserialize back
        result2 = ComparisonResult.model_validate(data)

        assert result.decision == result2.decision
        assert result.slice_similarities == result2.slice_similarities


# ============================================================================
# Integration Tests
# ============================================================================

class TestDeterministicSerialization:
    """Test v1.2: Serialization is deterministic (same input → same output)"""

    def test_intent_event_determinism(self):
        """Test v1.2: IntentEvent deterministic serialization"""
        event = IntentEvent(
            id="550e8400-e29b-41d4-a716-446655440000",
            tenantId="tenant-123",
            timestamp=1699564800.0,
            actor=Actor(id="user-123", type="user"),
            action="read",
            resource=Resource(type="database", name="users_db"),
            data=Data(sensitivity=["internal", "public"], pii=True, volume="single"),
            risk=Risk(authn="required"),
        )

        json1 = event.model_dump_json()
        json2 = event.model_dump_json()

        assert json1 == json2

    def test_boundary_determinism(self):
        """Test v1.2: DesignBoundary deterministic serialization"""
        boundary = DesignBoundary(
            id="boundary-001",
            name="Test Boundary",
            status="active",
            type="mandatory",
            scope=BoundaryScope(tenantId="tenant-123"),
            rules=BoundaryRules(
                thresholds=SliceThresholds(
                    action=0.85,
                    resource=0.80,
                    data=0.75,
                    risk=0.70
                ),
                decision="min"
            ),
            constraints=BoundaryConstraints(
                action=ActionConstraint(actions=["read"], actor_types=["user"]),
                resource=ResourceConstraint(types=["database"]),
                data=DataConstraint(sensitivity=["internal"]),
                risk=RiskConstraint(authn="required")
            ),
            createdAt=1699564800.0,
            updatedAt=1699564800.0
        )

        json1 = boundary.model_dump_json()
        json2 = boundary.model_dump_json()

        assert json1 == json2
