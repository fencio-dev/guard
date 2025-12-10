"""
Unit tests for rule-to-anchor conversion (v1.3 L4 ToolGateway).

Covers:
- ToolWhitelist: action/resource/data/risk anchors, counts, shapes, determinism, truncation
- ToolParamConstraint: anchors mapping from constraints, counts, shapes, determinism

Notes:
- Uses only public functions from app.rule_encoding
- Verifies padding to 16 anchors and 32-d vectors
"""

from __future__ import annotations

import numpy as np

from app.rule_encoding import (
    build_tool_whitelist_anchors,
    build_tool_param_constraint_anchors,
)


def _assert_anchor_block(block: list[list[float]], count: int) -> None:
    """Assert a padded anchor block is 16×32 with correct count."""
    assert isinstance(block, list)
    assert len(block) == 16, "Anchor block must have 16 rows (padded)"
    for row in block:
        assert isinstance(row, list)
        assert len(row) == 32, "Each anchor row must be 32 floats"
    assert 0 <= count <= 16, "Count must be within [0, 16]"


def test_tool_whitelist_basic_shapes_and_counts() -> None:
    rule = {
        "rule_id": "tw_1",
        "allowed_tool_ids": ["web_search", "db_query"],
        "allowed_methods": ["query", "read"],
        "rate_limit_per_min": 60,
    }

    anchors = build_tool_whitelist_anchors(rule)

    # Validate blocks and counts
    _assert_anchor_block(anchors["action_anchors"], anchors["action_count"])
    _assert_anchor_block(anchors["resource_anchors"], anchors["resource_count"])
    _assert_anchor_block(anchors["data_anchors"], anchors["data_count"])
    _assert_anchor_block(anchors["risk_anchors"], anchors["risk_count"])

    # Basic expectations
    assert anchors["action_count"] >= 1
    assert anchors["resource_count"] == 2  # two tools → two resource anchors
    assert anchors["data_count"] >= 1
    assert anchors["risk_count"] >= 1


def test_tool_whitelist_determinism() -> None:
    rule = {
        "rule_id": "tw_det",
        "allowed_tool_ids": ["web_search", "db_query"],
        "allowed_methods": ["query", "read"],
    }

    a1 = build_tool_whitelist_anchors(rule)
    a2 = build_tool_whitelist_anchors(rule)

    # Compare arrays element-wise
    for key in ("action_anchors", "resource_anchors", "data_anchors", "risk_anchors"):
        v1 = np.array(a1[key], dtype=np.float32)
        v2 = np.array(a2[key], dtype=np.float32)
        assert np.allclose(v1, v2), f"Anchors for {key} not deterministic"

    for key in ("action_count", "resource_count", "data_count", "risk_count"):
        assert a1[key] == a2[key], f"Count for {key} not deterministic"


def test_tool_whitelist_truncates_to_16_resource_anchors() -> None:
    # Create more than 16 tools
    tools = [f"tool_{i:02d}" for i in range(20)]
    rule = {
        "rule_id": "tw_many",
        "allowed_tool_ids": tools,
        "allowed_methods": ["query"],
    }

    anchors = build_tool_whitelist_anchors(rule)
    assert anchors["resource_count"] == 16, "Should truncate to 16 resource anchors"
    assert len(anchors["resource_anchors"]) == 16
    # Padded rows beyond count should be zeros
    arr = np.array(anchors["resource_anchors"], dtype=np.float32)
    assert arr.shape == (16, 32)
    assert np.allclose(np.linalg.norm(arr[: anchors["resource_count"]], axis=1), 1.0, atol=1e-3)


def test_tool_param_constraint_basic_shapes_and_counts() -> None:
    rule = {
        "rule_id": "tpc_1",
        "tool_id": "web_search",
        "param_name": "query",
        "param_type": "string",
        "max_len": 100,
        "allowed_values": ["foo", "bar"],
        "enforcement_mode": "hard",
    }

    anchors = build_tool_param_constraint_anchors(rule)

    _assert_anchor_block(anchors["action_anchors"], anchors["action_count"])
    _assert_anchor_block(anchors["resource_anchors"], anchors["resource_count"])
    _assert_anchor_block(anchors["data_anchors"], anchors["data_count"])
    _assert_anchor_block(anchors["risk_anchors"], anchors["risk_count"])

    # Expect at least one anchor in each slot
    assert anchors["action_count"] >= 1
    assert anchors["resource_count"] >= 1
    assert anchors["data_count"] >= 1
    assert anchors["risk_count"] >= 1


def test_tool_param_constraint_determinism() -> None:
    rule = {
        "rule_id": "tpc_det",
        "tool_id": "web_search",
        "param_name": "query",
        "param_type": "string",
        "max_len": 100,
        "allowed_values": ["foo", "bar"],
        "enforcement_mode": "soft",
    }

    a1 = build_tool_param_constraint_anchors(rule)
    a2 = build_tool_param_constraint_anchors(rule)

    for key in ("action_anchors", "resource_anchors", "data_anchors", "risk_anchors"):
        v1 = np.array(a1[key], dtype=np.float32)
        v2 = np.array(a2[key], dtype=np.float32)
        assert np.allclose(v1, v2), f"Anchors for {key} not deterministic"

    for key in ("action_count", "resource_count", "data_count", "risk_count"):
        assert a1[key] == a2[key], f"Count for {key} not deterministic"

