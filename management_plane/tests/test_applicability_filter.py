"""Rule-family applicability filter tests."""

import pytest

from app.applicability import evaluate_applicability
from app.endpoints.intents import is_boundary_applicable
from app.models import (
    ActionConstraint,
    Actor,
    BoundaryConstraints,
    BoundaryRules,
    BoundaryScope,
    Data,
    DataConstraint,
    DesignBoundary,
    IntentEvent,
    Resource,
    ResourceConstraint,
    Risk,
    RiskConstraint,
    SliceThresholds,
)


@pytest.fixture
def base_intent() -> IntentEvent:
    return IntentEvent(
        id="intent_base",
        schemaVersion="v1.2",
        tenantId="tenant_demo",
        timestamp=1700000000.0,
        actor=Actor(id="agent-7", type="agent"),
        action="read",
        resource=Resource(type="api", name="inventory_api", location=None),
        data=Data(sensitivity=["internal"], pii=None, volume=None),
        risk=Risk(authn="required"),
    )


@pytest.fixture
def base_boundary() -> DesignBoundary:
    return DesignBoundary(
        id="boundary_read_api",
        name="Allow Agent Reads",
        status="active",
        type="mandatory",
        boundarySchemaVersion="v1.2",
        scope=BoundaryScope(tenantId="tenant_demo", domains=["api"]),
        rules=BoundaryRules(
            thresholds=SliceThresholds(action=0.8, resource=0.75, data=0.7, risk=0.6),
            decision="min",
        ),
        constraints=BoundaryConstraints(
            action=ActionConstraint(actions=["read"], actor_types=["agent", "llm"]),
            resource=ResourceConstraint(types=["api"], locations=["cloud"], names=None),
            data=DataConstraint(sensitivity=["internal"], pii=False, volume="single"),
            risk=RiskConstraint(authn="required"),
        ),
        createdAt=1700000000.0,
        updatedAt=1700000000.0,
    )


def test_core_match_missing_optional_fields(base_intent, base_boundary):
    """Missing optional fields should abstain, not exclude."""
    result = evaluate_applicability(base_intent, base_boundary)
    assert result.applicable is True
    assert any(o.rule_id == "LocationRule" and o.decision == "abstain" for o in result.outcomes)
    assert any(o.rule_id == "PiiRule" and o.decision == "abstain" for o in result.outcomes)


def test_core_action_mismatch_blocks(base_intent, base_boundary):
    boundary = base_boundary.model_copy(update={
        "constraints": base_boundary.constraints.model_copy(update={
            "action": ActionConstraint(actions=["write"], actor_types=["agent"])
        })
    })
    assert is_boundary_applicable(base_intent, boundary) is False


def test_soft_mismatch_below_threshold(base_intent, base_boundary, monkeypatch):
    """Two soft mismatches should drop score below 0.5 and mark not applicable."""
    # Force location mismatch and volume mismatch
    mutated_boundary = base_boundary.model_copy(update={
        "constraints": base_boundary.constraints.model_copy(update={
            "resource": ResourceConstraint(types=["api"], locations=["cloud"], names=None),
            "data": DataConstraint(sensitivity=["internal"], pii=False, volume="bulk"),
        })
    })
    intent = base_intent.model_copy(update={
        "resource": base_intent.resource.model_copy(update={"location": "local"}),
        "data": base_intent.data.model_copy(update={"volume": "single", "pii": False}),
    })

    result = evaluate_applicability(intent, mutated_boundary)
    assert result.applicable is False
    assert result.score < 0.5


def test_soft_matches_push_score_over_threshold(base_intent, base_boundary):
    intent = base_intent.model_copy(update={
        "resource": base_intent.resource.model_copy(update={"location": "cloud"}),
        "data": base_intent.data.model_copy(update={"volume": "single", "pii": False}),
    })
    result = evaluate_applicability(intent, base_boundary)
    assert result.applicable is True
    assert result.score > 0.8


def test_no_soft_rules_defaults_to_allow(base_intent):
    boundary = DesignBoundary(
        id="boundary_minimal",
        name="Minimal",
        status="active",
        type="mandatory",
        boundarySchemaVersion="v1.2",
        scope=BoundaryScope(tenantId="tenant_demo"),
        rules=BoundaryRules(
            thresholds=SliceThresholds(action=0.8, resource=0.75, data=0.7, risk=0.6),
            decision="min",
        ),
        constraints=BoundaryConstraints(
            action=ActionConstraint(actions=["read"], actor_types=["agent"]),
            resource=ResourceConstraint(types=["api"], locations=None, names=None),
            data=DataConstraint(sensitivity=["internal"], pii=None, volume=None),
            risk=RiskConstraint(authn="required"),
        ),
        createdAt=1700000000.0,
        updatedAt=1700000000.0,
    )
    result = evaluate_applicability(base_intent, boundary)
    assert result.applicable is True
    assert result.score == pytest.approx(1.0)
