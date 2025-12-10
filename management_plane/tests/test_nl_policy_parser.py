# tests/test_nl_policy_parser.py
import pytest
import os
from app.nl_policy_parser import (
    ActionConstraints,
    ResourceConstraints,
    DataConstraints,
    RiskConstraints,
    PolicyConstraints,
    SliceThresholds,
    PolicyRules,
    NLPolicyParser
)

def test_policy_rules_schema():
    """Test PolicyRules schema validation."""
    policy = PolicyRules(
        thresholds=SliceThresholds(),
        decision="min",
        constraints=PolicyConstraints(
            action=ActionConstraints(actions=["read"], actor_types=["user"]),
            resource=ResourceConstraints(types=["database"]),
            data=DataConstraints(sensitivity=["public"]),
            risk=RiskConstraints(authn="required")
        )
    )

    assert policy.thresholds.action == 0.5
    assert policy.decision == "min"
    assert policy.constraints.action.actions == ["read"]

@pytest.mark.asyncio
async def test_parse_simple_template():
    """Test parsing a simple template without customization."""
    api_key = os.getenv("GEMINI_API_KEY")
    if not api_key:
        pytest.skip("GEMINI_API_KEY not set")

    parser = NLPolicyParser(api_key=api_key)
    policy = await parser.parse_policy(
        template_id="database_read_only",
        template_text="Allow reading from databases",
        customization=None
    )

    assert policy.constraints.action.actions == ["read"]
    assert "database" in policy.constraints.resource.types
    assert policy.thresholds.action == 0.5

@pytest.mark.asyncio
async def test_parse_with_customization():
    """Test parsing with natural language customization."""
    api_key = os.getenv("GEMINI_API_KEY")
    if not api_key:
        pytest.skip("GEMINI_API_KEY not set")

    parser = NLPolicyParser(api_key=api_key)
    policy = await parser.parse_policy(
        template_id="database_read_only",
        template_text="Allow reading from databases",
        customization="only public data"
    )

    assert policy.constraints.data.sensitivity == ["public"]

def test_vocabulary_validation_rejects_invalid_values():
    """Test that vocabulary validation rejects invalid values."""
    parser = NLPolicyParser(api_key="dummy")

    # Test invalid action
    policy = PolicyRules(
        thresholds=SliceThresholds(),
        decision="min",
        constraints=PolicyConstraints(
            action=ActionConstraints(actions=["invalid_action"], actor_types=["user"]),
            resource=ResourceConstraints(types=["database"]),
            data=DataConstraints(sensitivity=["public"]),
            risk=RiskConstraints(authn="required")
        )
    )
    with pytest.raises(ValueError, match="Invalid action 'invalid_action'"):
        parser._validate_vocabulary_compliance(policy)

    # Test invalid actor_type
    policy = PolicyRules(
        thresholds=SliceThresholds(),
        decision="min",
        constraints=PolicyConstraints(
            action=ActionConstraints(actions=["read"], actor_types=["invalid_actor"]),
            resource=ResourceConstraints(types=["database"]),
            data=DataConstraints(sensitivity=["public"]),
            risk=RiskConstraints(authn="required")
        )
    )
    with pytest.raises(ValueError, match="Invalid actor_type 'invalid_actor'"):
        parser._validate_vocabulary_compliance(policy)

    # Test invalid resource type
    policy = PolicyRules(
        thresholds=SliceThresholds(),
        decision="min",
        constraints=PolicyConstraints(
            action=ActionConstraints(actions=["read"], actor_types=["user"]),
            resource=ResourceConstraints(types=["invalid_resource"]),
            data=DataConstraints(sensitivity=["public"]),
            risk=RiskConstraints(authn="required")
        )
    )
    with pytest.raises(ValueError, match="Invalid resource type 'invalid_resource'"):
        parser._validate_vocabulary_compliance(policy)

    # Test invalid sensitivity
    policy = PolicyRules(
        thresholds=SliceThresholds(),
        decision="min",
        constraints=PolicyConstraints(
            action=ActionConstraints(actions=["read"], actor_types=["user"]),
            resource=ResourceConstraints(types=["database"]),
            data=DataConstraints(sensitivity=["invalid_sensitivity"]),
            risk=RiskConstraints(authn="required")
        )
    )
    with pytest.raises(ValueError, match="Invalid sensitivity 'invalid_sensitivity'"):
        parser._validate_vocabulary_compliance(policy)

    # Test invalid volume
    policy = PolicyRules(
        thresholds=SliceThresholds(),
        decision="min",
        constraints=PolicyConstraints(
            action=ActionConstraints(actions=["read"], actor_types=["user"]),
            resource=ResourceConstraints(types=["database"]),
            data=DataConstraints(sensitivity=["public"], volume="invalid_volume"),
            risk=RiskConstraints(authn="required")
        )
    )
    with pytest.raises(ValueError, match="Invalid volume 'invalid_volume'"):
        parser._validate_vocabulary_compliance(policy)

    # Test invalid authn
    policy = PolicyRules(
        thresholds=SliceThresholds(),
        decision="min",
        constraints=PolicyConstraints(
            action=ActionConstraints(actions=["read"], actor_types=["user"]),
            resource=ResourceConstraints(types=["database"]),
            data=DataConstraints(sensitivity=["public"]),
            risk=RiskConstraints(authn="invalid_authn")
        )
    )
    with pytest.raises(ValueError, match="Invalid authn 'invalid_authn'"):
        parser._validate_vocabulary_compliance(policy)

    # Test valid policy passes validation
    policy = PolicyRules(
        thresholds=SliceThresholds(),
        decision="min",
        constraints=PolicyConstraints(
            action=ActionConstraints(actions=["read", "write"], actor_types=["user", "agent"]),
            resource=ResourceConstraints(types=["database", "file"]),
            data=DataConstraints(sensitivity=["public", "internal"], volume="single"),
            risk=RiskConstraints(authn="required")
        )
    )
    # Should not raise any exception
    parser._validate_vocabulary_compliance(policy)
