"""Test suite for agent management HTTP endpoints."""

from __future__ import annotations

from datetime import datetime
from unittest.mock import MagicMock, AsyncMock, patch

import pytest
from fastapi.testclient import TestClient

from app.auth import User, get_current_user
from app.database import get_db
from app.endpoints import agents as agents_module
from app.main import app
from app.nl_policy_parser import PolicyRules


# ---------------------------------------------------------------------------
# Dependency overrides
# ---------------------------------------------------------------------------

TEST_USER = User(id="tenant_test", email="test@example.com", role="authenticated")


@pytest.fixture(autouse=True)
def override_auth():
    """Provide a deterministic authenticated user for every test."""
    app.dependency_overrides[get_current_user] = lambda: TEST_USER
    yield
    app.dependency_overrides.pop(get_current_user, None)


@pytest.fixture
def mock_db():
    """Inject a mock Supabase client into the dependency graph."""
    mock = MagicMock()
    app.dependency_overrides[get_db] = lambda: mock
    yield mock
    app.dependency_overrides.pop(get_db, None)


@pytest.fixture
def mock_policy_parser():
    """Provide a fake NL policy parser for Task 6 endpoints."""
    mock = MagicMock()
    mock.parse_policy = AsyncMock()
    app.dependency_overrides[agents_module.get_policy_parser] = lambda: mock
    yield mock
    app.dependency_overrides.pop(agents_module.get_policy_parser, None)


@pytest.fixture
def client() -> TestClient:
    """FastAPI TestClient bound to the Management Plane app."""
    with TestClient(app) as test_client:
        yield test_client


def test_register_agent_creates_new_agent(client: TestClient, mock_db: MagicMock):
    """POST /agents/register inserts the agent when none exists."""
    mock_db.select.return_value = []
    mock_db.insert.return_value = [
        {
            "id": "agent-uuid-123",
            "agent_id": "test-agent",
            "first_seen": datetime(2025, 11, 23, 12, 0, 0).isoformat(),
            "last_seen": datetime(2025, 11, 23, 12, 0, 0).isoformat(),
            "sdk_version": "1.3.0",
        }
    ]

    response = client.post(
        "/api/v1/agents/register",
        json={"agent_id": "test-agent", "sdk_version": "1.3.0", "metadata": {}},
    )

    assert response.status_code == 200
    data = response.json()
    assert data["agent_id"] == "test-agent"
    assert data["sdk_version"] == "1.3.0"
    mock_db.insert.assert_called_once()


def test_register_agent_updates_existing(client: TestClient, mock_db: MagicMock):
    """Re-registration updates last_seen + sdk_version for existing agent."""
    existing_ts = datetime(2025, 11, 23, 11, 0, 0).isoformat()
    updated_ts = datetime(2025, 11, 23, 12, 0, 0).isoformat()

    mock_db.select.return_value = [
        {
            "id": "agent-uuid-124",
            "agent_id": "dup-agent",
            "first_seen": existing_ts,
            "last_seen": existing_ts,
            "sdk_version": "1.3.0",
        }
    ]
    mock_db.update.return_value = [
        {
            "id": "agent-uuid-124",
            "agent_id": "dup-agent",
            "first_seen": existing_ts,
            "last_seen": updated_ts,
            "sdk_version": "1.3.1",
        }
    ]

    response = client.post(
        "/api/v1/agents/register",
        json={"agent_id": "dup-agent", "sdk_version": "1.3.1"},
    )

    assert response.status_code == 200
    data = response.json()
    assert data["first_seen"].startswith("2025-11-23T11:00:00")
    assert data["sdk_version"] == "1.3.1"
    mock_db.update.assert_called_once()


def test_list_agents_returns_paginated(client: TestClient, mock_db: MagicMock):
    """GET /agents/list returns total + serialized agents."""
    mock_db.count.return_value = 2
    mock_db.select.return_value = [
        {
            "id": "agent-uuid-1",
            "agent_id": "agent-1",
            "first_seen": datetime(2025, 11, 22, 12, 0, 0).isoformat(),
            "last_seen": datetime(2025, 11, 23, 12, 0, 0).isoformat(),
            "sdk_version": "1.3.0",
        },
        {
            "id": "agent-uuid-2",
            "agent_id": "agent-2",
            "first_seen": datetime(2025, 11, 20, 15, 0, 0).isoformat(),
            "last_seen": datetime(2025, 11, 23, 11, 0, 0).isoformat(),
            "sdk_version": "1.2.5",
        },
    ]

    response = client.get("/api/v1/agents/list", params={"limit": 10, "offset": 0})

    assert response.status_code == 200
    payload = response.json()
    assert payload["total"] == 2
    assert len(payload["agents"]) == 2
    assert {agent["agent_id"] for agent in payload["agents"]} == {"agent-1", "agent-2"}
    mock_db.count.assert_called_once_with("registered_agents", eq={"tenant_id": TEST_USER.id})
    mock_db.select.assert_called_once()


def _sample_policy_rules_dict() -> dict:
    return {
        "decision": "min",
        "globalThreshold": None,
        "thresholds": {
            "action": 0.5,
            "resource": 0.5,
            "data": 0.5,
            "risk": 0.5,
        },
        "constraints": {
            "action": {"actions": ["read"], "actor_types": ["user"]},
            "resource": {"types": ["database"], "names": None, "locations": None},
            "data": {"sensitivity": ["public"], "pii": False, "volume": "single"},
            "risk": {"authn": "required"},
        },
    }


def test_create_policy_success(client: TestClient, mock_db: MagicMock, mock_policy_parser: MagicMock):
    """POST /agents/policies inserts policy when agent exists."""
    mock_db.select.side_effect = [
        [{"id": "agent-uuid", "agent_id": "policy-agent"}],  # registered agent
        [],  # no existing policy yet
    ]
    policy_payload = _sample_policy_rules_dict()
    policy_obj = PolicyRules.model_validate(policy_payload)
    mock_policy_parser.parse_policy.return_value = policy_obj
    embedded_meta = {
        "rule_id": "policy-agent:database_read_only",
        "chroma_synced_at": "2025-11-23T12:00:00+00:00",
    }
    mock_db.insert.return_value = [
        {
            "id": "policy-uuid",
            "agent_id": "policy-agent",
            "template_id": "database_read_only",
            "template_text": "Allow reading",
            "customization": "only public data",
            "policy_rules": policy_payload,
            "embedding_metadata": embedded_meta,
            "created_at": "2025-11-23T12:00:00+00:00",
            "updated_at": "2025-11-23T12:00:00+00:00",
        }
    ]

    response = client.post(
        "/api/v1/agents/policies",
        json={
            "agent_id": "policy-agent",
            "template_id": "database_read_only",
            "template_text": "Allow reading",
            "customization": "only public data",
        },
    )

    assert response.status_code == 200
    data = response.json()
    assert data["agent_id"] == "policy-agent"
    assert data["template_id"] == "database_read_only"
    assert data["policy_rules"]["constraints"]["action"]["actions"] == ["read"]
    assert data["embedding_metadata"]["rule_id"].startswith("policy-agent:")
    mock_policy_parser.parse_policy.assert_called_once()
    mock_db.insert.assert_called_once()


def test_create_policy_persists_rule_payload(
    client: TestClient,
    mock_db: MagicMock,
    mock_policy_parser: MagicMock,
):
    """Policy creation should encode anchors and upsert them to Chroma."""

    mock_db.select.side_effect = [
        [{"id": "agent-uuid", "agent_id": "policy-agent"}],
        [],
    ]
    policy_payload = _sample_policy_rules_dict()
    policy_obj = PolicyRules.model_validate(policy_payload)
    mock_policy_parser.parse_policy.return_value = policy_obj
    embedded_meta = {
        "rule_id": "policy-agent:database_read_only",
        "chroma_synced_at": "2025-11-23T12:00:00+00:00",
    }
    mock_db.insert.return_value = [
        {
            "id": "policy-uuid",
            "agent_id": "policy-agent",
            "template_id": "database_read_only",
            "template_text": "Allow reading",
            "customization": "only public data",
            "policy_rules": policy_payload,
            "embedding_metadata": embedded_meta,
            "created_at": "2025-11-23T12:00:00+00:00",
            "updated_at": "2025-11-23T12:00:00+00:00",
        }
    ]

    dummy_anchors = {
        "action_anchors": [[0.1] * 32],
        "action_count": 1,
        "resource_anchors": [[0.2] * 32],
        "resource_count": 1,
        "data_anchors": [[0.3] * 32],
        "data_count": 1,
        "risk_anchors": [[0.4] * 32],
        "risk_count": 1,
    }

    with patch(
        "app.rule_installer.build_tool_whitelist_anchors",
        new_callable=AsyncMock,
    ) as mock_builder, patch("app.rule_installer.upsert_rule_payload") as mock_upsert:
        mock_builder.return_value = dummy_anchors
        response = client.post(
            "/api/v1/agents/policies",
            json={
                "agent_id": "policy-agent",
                "template_id": "database_read_only",
                "template_text": "Allow reading",
            },
        )

    assert response.status_code == 200
    mock_builder.assert_awaited()
    mock_upsert.assert_called_once()
    args, kwargs = mock_upsert.call_args
    assert args[0] == TEST_USER.id
    assert args[1].startswith("policy-agent:")
    payload = args[2]
    assert payload["anchors"] == dummy_anchors
    inserted_payload = mock_db.insert.call_args[0][1]
    assert inserted_payload["embedding_metadata"]["rule_id"].startswith("policy-agent:")

def test_create_policy_unregistered_agent_returns_404(
    client: TestClient, mock_db: MagicMock, mock_policy_parser: MagicMock
):
    """Creating policy for unknown agent should 404 and skip parser."""
    mock_db.select.return_value = []

    response = client.post(
        "/api/v1/agents/policies",
        json={
            "agent_id": "missing-agent",
            "template_id": "database_read_only",
            "template_text": "Allow reading",
        },
    )

    assert response.status_code == 404
    detail = response.json()["detail"]
    assert detail["error"] == "agent_not_registered"
    mock_policy_parser.parse_policy.assert_not_called()


def test_get_policy_returns_policy(client: TestClient, mock_db: MagicMock, mock_policy_parser: MagicMock):
    """GET /agents/policies/{agent_id} returns stored policy."""
    mock_db.select.return_value = [
        {
            "id": "policy-uuid",
            "agent_id": "get-agent",
            "template_id": "database_read_only",
            "template_text": "Allow reading",
            "customization": None,
            "policy_rules": _sample_policy_rules_dict(),
            "created_at": "2025-11-23T12:00:00+00:00",
            "updated_at": "2025-11-23T12:00:00+00:00",
        }
    ]

    response = client.get("/api/v1/agents/policies/get-agent")

    assert response.status_code == 200
    payload = response.json()
    assert payload["agent_id"] == "get-agent"
    assert payload["policy_rules"]["thresholds"]["action"] == 0.5


def test_list_templates_returns_all(client: TestClient):
    """GET /agents/templates returns built-in templates."""
    response = client.get("/api/v1/agents/templates")

    assert response.status_code == 200
    templates = response.json()["templates"]
    assert len(templates) >= 1
    assert {"id", "name", "category"}.issubset(templates[0].keys())


def test_list_templates_filters_by_category(client: TestClient):
    """Filtering templates by category trims list."""
    response = client.get("/api/v1/agents/templates", params={"category": "database"})

    assert response.status_code == 200
    templates = response.json()["templates"]
    assert all(t["category"] == "database" for t in templates)


def test_get_template_by_id(client: TestClient):
    """GET /agents/templates/{id} returns template detail."""
    response = client.get("/api/v1/agents/templates/database_read_only")

    assert response.status_code == 200
    template = response.json()
    assert template["id"] == "database_read_only"
    assert "example_customizations" in template
