"""
End-to-end test of the new layer-based enforcement flow using a shim data plane.

This test exercises the intended SDK → Data Plane → Management Plane (encode)
→ Data Plane (compare via Rust sandbox) → SDK decision path without requiring
the real Data Plane server. It uses:

- sdk SecureGraphProxy (enforcement_agent) wrapping a stub LangGraph-like
  graph that emits a tool_call.
- A lightweight DataPlaneShimClient that implements the v1.3 evaluation:
  - Encode intent (uses app.encoding.encode_to_128d)
  - Encode L4 ToolGateway rule anchors (uses app.rule_encoding)
  - Compare via Rust semantic-sandbox FFI (app.ffi_bridge)
  - Short-circuit on first BLOCK

To keep the test hermetic (no network), we monkeypatch the embedding function
so SentenceTransformer is never loaded. The patch produces a deterministic
384‑dim “one‑hot by hash” vector which, after projection, yields cosine = 1.0
for identical texts and low similarity for different texts. This is sufficient
to validate control flow and short-circuit behaviour.
"""

from __future__ import annotations

import time
import zlib
from dataclasses import dataclass
from typing import Any, Optional

import numpy as np
import pytest

# SDK enforcement wrapper
from tupl.agent import enforcement_agent
from tupl.types import ComparisonResult

# Management plane internals we reuse for encoding + FFI
from app import encoding as enc
from app import rule_encoding as re_rules
from app.ffi_bridge import get_sandbox


# ---------------------------------------------------------------------------
# Helpers: monkeypatch embedding to be offline/fast and deterministic
# ---------------------------------------------------------------------------

def _fake_embed_384(text: str) -> np.ndarray:
    """Deterministic 384‑d bag‑of‑tokens vector (order‑invariant).

    Splits on whitespace, hashes each token to an index, accumulates counts,
    then L2‑normalizes. Identical token sets (regardless of order) map to the
    same embedding, which makes anchor vs slot strings comparable even when the
    field order differs.
    """
    v = np.zeros(384, dtype=np.float32)
    for raw in text.replace("|", " ").replace(":", " ").split():
        tok = raw.strip().lower()
        if not tok:
            continue
        if tok in {"is", "equals"}:
            continue  # drop common glue tokens
        w = 3.0 if tok in {"true", "false", "read", "write", "delete", "export", "execute"} else 1.0
        idx = zlib.adler32(tok.encode("utf-8")) % 384
        v[int(idx)] += w
    n = np.linalg.norm(v)
    if n > 0:
        v /= n
    return v.astype(np.float32)


@pytest.fixture(autouse=True)
def patch_fake_embeddings(monkeypatch: pytest.MonkeyPatch):
    """
    Patch app.encoding.encode_text_cached and app.rule_encoding.encode_anchor_text
    so no external model is loaded. Uses the same projection matrices and
    normalization as production code to keep math identical downstream.
    """

    # Patch the 384-d text encoder used by all slot builders
    monkeypatch.setattr(enc, "encode_text_cached", _fake_embed_384, raising=True)

    # Patch rule anchor encoder to project with the real matrices
    def _fake_encode_anchor_text(text: str, slot_name: str, seed: int) -> np.ndarray:
        e384 = _fake_embed_384(text)
        pm = enc.get_projection_matrix(slot_name, seed)
        vec32 = pm @ e384
        n = np.linalg.norm(vec32)
        if n > 0:
            vec32 = vec32 / n
        return vec32.astype(np.float32)

    monkeypatch.setattr(re_rules, "encode_anchor_text", _fake_encode_anchor_text, raising=True)


# ---------------------------------------------------------------------------
# Minimal Data Plane shim (Python) implementing v1.3 evaluation
# ---------------------------------------------------------------------------

class DataPlaneShimClient:
    """
    Minimal stand-in for the Data Plane. Implements:
      - intent → 128d via app.encoding
      - rule → anchors via app.rule_encoding
      - compare via Rust CDylib (app.ffi_bridge)
      - min-mode thresholds + short-circuit on first BLOCK

    Returns a tupl.types.ComparisonResult for SDK consumption.
    """

    def __init__(self, rules: list[dict], thresholds: list[float] | None = None):
        self.rules = rules
        # Stricter defaults so mismatched fields fail decisively in this shim
        # Calibrated for the fake embedding: action must strongly match; resource/data fairly high; risk lenient
        self.thresholds = thresholds or [0.95, 0.90, 0.90, 0.50]
        self.sandbox = get_sandbox()

    def _encode_rule(self, rule: dict) -> tuple[np.ndarray, int, np.ndarray, int, np.ndarray, int, np.ndarray, int]:
        # Map family → anchors (MVP: ToolWhitelist only for these tests)
        anchors = re_rules.build_tool_whitelist_anchors(rule)
        act = np.array(anchors["action_anchors"], dtype=np.float32)
        rac = np.array(anchors["resource_anchors"], dtype=np.float32)
        dac = np.array(anchors["data_anchors"], dtype=np.float32)
        krc = np.array(anchors["risk_anchors"], dtype=np.float32)
        return (
            act, anchors["action_count"],
            rac, anchors["resource_count"],
            dac, anchors["data_count"],
            krc, anchors["risk_count"],
        )

    def capture(self, event) -> Optional[ComparisonResult]:  # SDK expects this signature
        # Encode intent
        intent_vec = enc.encode_to_128d(event)

        evidence = []
        rules_evaluated = 0

        # Evaluate in priority order (list order for this shim)
        for rule in self.rules:
            (
                action_anchors, action_count,
                resource_anchors, resource_count,
                data_anchors, data_count,
                risk_anchors, risk_count,
            ) = self._encode_rule(rule)

            decision, sims = self.sandbox.compare(
                intent_vector=intent_vec,
                action_anchors=action_anchors,
                action_anchor_count=action_count,
                resource_anchors=resource_anchors,
                resource_anchor_count=resource_count,
                data_anchors=data_anchors,
                data_anchor_count=data_count,
                risk_anchors=risk_anchors,
                risk_anchor_count=risk_count,
                thresholds=self.thresholds,
                weights=[1.0, 1.0, 1.0, 1.0],
                decision_mode=0,
                global_threshold=0.0,
            )

            rules_evaluated += 1
            if decision == 0:
                # Short-circuit on first BLOCK
                return ComparisonResult(
                    decision=0,
                    slice_similarities=list(map(float, sims)),
                    boundaries_evaluated=rules_evaluated,
                    timestamp=time.time(),
                    evidence=[],
                )

        # All passed → ALLOW
        return ComparisonResult(
            decision=1,
            slice_similarities=[1.0, 1.0, 1.0, 1.0],  # Not used by SDK logic, keep simple
            boundaries_evaluated=rules_evaluated,
            timestamp=time.time(),
            evidence=[],
        )


# ---------------------------------------------------------------------------
# Stub LangGraph-like compiled graph and messages
# ---------------------------------------------------------------------------

@dataclass
class _Msg:
    tool_calls: list[dict]


class DummyGraph:
    """Minimal graph with .stream() that emits a single tool_call state."""

    def __init__(self, tool_name: str, tool_args: dict):
        self.tool_name = tool_name
        self.tool_args = tool_args

    def stream(self, inputs: dict, config: Optional[dict] = None, stream_mode: str = "values", **kwargs):
        yield {"messages": [_Msg(tool_calls=[{"name": self.tool_name, "args": self.tool_args, "id": "t1"}])]}    


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------

def _policies_allow_search_only() -> list[dict]:
    """Return a simple L4 ToolWhitelist that allows only the search tool."""
    return [
        {
            "rule_id": "tw_allow_search",
            "allowed_tool_ids": ["search_database"],
            "allowed_methods": ["query", "read"],
            "rate_limit_per_min": 60,
        }
    ]


def test_layered_flow_allows_search_tool():
    """Search tool should be ALLOW under ToolWhitelist policy."""
    graph = DummyGraph("search_database", {"query": "find alice"})
    shim = DataPlaneShimClient(_policies_allow_search_only())
    # Map all tools to 'api' so resource slot matches ToolWhitelist anchors
    secure = enforcement_agent(
        graph,
        boundary_id="all",
        client=shim,
        resource_type_mapper=lambda _tool_name: "api",
    )

    # Should not raise
    result_state = secure.invoke({"messages": [{"role": "user", "content": "query"}]})
    assert isinstance(result_state, dict)


def test_layered_flow_blocks_delete_tool():
    """Delete tool should be BLOCK under ToolWhitelist policy (method mismatch)."""
    graph = DummyGraph("delete_record", {"id": "cust-123"})
    shim = DataPlaneShimClient(_policies_allow_search_only())
    secure = enforcement_agent(
        graph,
        boundary_id="all",
        client=shim,
        resource_type_mapper=lambda _tool_name: "api",
    )

    with pytest.raises(PermissionError):
        secure.invoke({"messages": [{"role": "user", "content": "delete customer"}]})
