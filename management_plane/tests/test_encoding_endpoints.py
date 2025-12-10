"""
Endpoint tests for Management Plane encoding APIs (v1.3).

Verifies:
- POST /api/v1/encode/intent returns a 128-length vector
- POST /api/v1/encode/rule/tool_whitelist returns padded anchors with counts
- POST /api/v1/encode/rule/tool_param_constraint returns padded anchors with counts
"""

from __future__ import annotations

import numpy as np
from fastapi.testclient import TestClient

from app.main import app


client = TestClient(app)


def _assert_anchor_response(payload: dict) -> None:
    for key in ("action_anchors", "resource_anchors", "data_anchors", "risk_anchors"):
        assert key in payload
        block = payload[key]
        assert isinstance(block, list)
        assert len(block) == 16
        for row in block:
            assert isinstance(row, list)
            assert len(row) == 32

    for key in ("action_count", "resource_count", "data_count", "risk_count"):
        assert key in payload
        assert 0 <= int(payload[key]) <= 16


def test_encode_intent_endpoint_returns_128_vector() -> None:
    # Minimal valid v1.3 IntentEvent
    event = {
        "id": "intent_123",
        "schemaVersion": "v1.3",
        "tenantId": "tenant_test",
        "timestamp": 1700000000.0,
        "actor": {"id": "agent-1", "type": "agent"},
        "action": "read",
        "resource": {"type": "api", "name": "web_search", "location": "cloud"},
        "data": {"sensitivity": ["internal"], "pii": False, "volume": "single"},
        "risk": {"authn": "required"},
        "layer": "L4",
        "tool_name": "web_search",
        "tool_method": "query",
        "tool_params": {"query": "example"},
    }

    resp = client.post("/api/v1/encode/intent", json=event)
    assert resp.status_code == 200, resp.text
    data = resp.json()
    assert "vector" in data
    vec = np.array(data["vector"], dtype=float)
    assert vec.shape == (128,)


def test_encode_rule_tool_whitelist_endpoint() -> None:
    rule = {
        "rule_id": "tw_api",
        "allowed_tool_ids": ["web_search", "db_query"],
        "allowed_methods": ["query", "read"],
        "rate_limit_per_min": 60,
    }

    resp = client.post("/api/v1/encode/rule/tool_whitelist", json=rule)
    assert resp.status_code == 200, resp.text
    payload = resp.json()
    _assert_anchor_response(payload)


def test_encode_rule_tool_param_constraint_endpoint() -> None:
    rule = {
        "rule_id": "tpc_api",
        "tool_id": "web_search",
        "param_name": "query",
        "param_type": "string",
        "max_len": 100,
        "allowed_values": ["foo", "bar"],
        "enforcement_mode": "hard",
    }

    resp = client.post("/api/v1/encode/rule/tool_param_constraint", json=rule)
    assert resp.status_code == 200, resp.text
    payload = resp.json()
    _assert_anchor_response(payload)

