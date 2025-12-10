"""
Schema alignment tests between the SDK and Management Plane.

Ensures the SDK's IntentEvent and ComparisonResult models match the
canonical Management Plane definitions (v1.3).
"""

from __future__ import annotations

import sys
from pathlib import Path

from tupl.types import (
    IntentEvent as SDKIntentEvent,
    ComparisonResult as SDKComparisonResult,
)

REPO_ROOT = Path(__file__).resolve().parents[3]
MP_PATH = REPO_ROOT / "management-plane"
if str(MP_PATH) not in sys.path:
    sys.path.insert(0, str(MP_PATH))

from app.models import (  # noqa: E402
    IntentEvent as MPIntentEvent,
    ComparisonResult as MPComparisonResult,
)


def _normalize_schema(value):
    """Remove descriptive metadata to compare structural equality only."""
    if isinstance(value, dict):
        return {
            key: _normalize_schema(val)
            for key, val in value.items()
            if key not in {"description", "examples", "title"}
        }
    if isinstance(value, list):
        return [_normalize_schema(item) for item in value]
    return value


def assert_schema_equal(mp_model, sdk_model):
    mp_schema = _normalize_schema(mp_model.model_json_schema())
    sdk_schema = _normalize_schema(sdk_model.model_json_schema())
    assert mp_schema == sdk_schema, (
        f"Schema mismatch detected:\nMP: {mp_schema}\nSDK: {sdk_schema}"
    )


def test_intent_event_schema_matches_management_plane():
    assert_schema_equal(MPIntentEvent, SDKIntentEvent)


def test_comparison_result_schema_matches_management_plane():
    assert_schema_equal(MPComparisonResult, SDKComparisonResult)
