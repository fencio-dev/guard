"""
Encoding endpoints for layer-based rule enforcement (v1.3).

Provides REST API for encoding IntentEvents and Data Plane rules to vectors.
Used by the Data Plane for real-time enforcement.
"""

import logging
from typing import Optional

import numpy as np
from fastapi import APIRouter, Depends, HTTPException
from pydantic import BaseModel, Field

from app.encoding import encode_to_128d
from app.models import IntentEvent

logger = logging.getLogger(__name__)

router = APIRouter(prefix="/encode", tags=["encoding"])


# ============================================================================
# Request/Response Models
# ============================================================================

class IntentEncodingResponse(BaseModel):
    """Response from intent encoding endpoint."""
    vector: list[float] = Field(min_length=128, max_length=128)


class RuleAnchors(BaseModel):
    """
    Anchor-based encoding for a Data Plane rule.

    Uses Phase 2 anchor system with up to 16 anchors per slot.
    Each anchor is a 32-dimensional vector.
    """
    action_anchors: list[list[float]] = Field(max_length=16)
    action_count: int = Field(ge=0, le=16)
    resource_anchors: list[list[float]] = Field(max_length=16)
    resource_count: int = Field(ge=0, le=16)
    data_anchors: list[list[float]] = Field(max_length=16)
    data_count: int = Field(ge=0, le=16)
    risk_anchors: list[list[float]] = Field(max_length=16)
    risk_count: int = Field(ge=0, le=16)


# ============================================================================
# Endpoints
# ============================================================================

@router.post("/intent", response_model=IntentEncodingResponse)
async def encode_intent(event: IntentEvent) -> IntentEncodingResponse:
    """
    Encode an IntentEvent to a 128-dimensional vector.

    Used by the Data Plane to convert incoming events to vectors
    for comparison against rule embeddings.

    Args:
        event: IntentEvent to encode (v1.3 with layer fields)

    Returns:
        128-dim vector (4 slots × 32 dims)

    Raises:
        HTTPException: If encoding fails
    """
    try:
        logger.debug(f"Encoding intent {event.id} (layer={event.layer})")

        # Encode to 128d vector using existing encoding pipeline
        vector = encode_to_128d(event)

        # Convert numpy array to list for JSON serialization
        vector_list = vector.tolist()

        logger.debug(f"Intent {event.id} encoded successfully")

        return IntentEncodingResponse(vector=vector_list)

    except Exception as e:
        logger.error(f"Failed to encode intent {event.id}: {e}", exc_info=True)
        raise HTTPException(
            status_code=500,
            detail=f"Encoding failed: {str(e)}"
        )


@router.post("/rule/tool_whitelist", response_model=RuleAnchors)
async def encode_tool_whitelist_rule(rule: dict) -> RuleAnchors:
    """
    Encode a ToolWhitelist rule to anchor arrays.

    Converts rule fields to natural language anchors, then embeds each anchor.
    Used by the Data Plane to cache rule embeddings for comparison.

    Args:
        rule: ToolWhitelistRule dict with fields:
            - allowed_tool_ids: List of allowed tool names
            - allowed_methods: List of allowed methods (query, read, write, etc.)
            - rate_limit_per_min: Optional rate limit

    Returns:
        RuleAnchors with embedded anchor arrays

    Raises:
        HTTPException: If encoding fails
    """
    try:
        logger.debug(f"Encoding ToolWhitelist rule: {rule.get('rule_id', 'unknown')}")

        # Import rule-to-anchor conversion functions (will be implemented in next task)
        from app.rule_encoding import build_tool_whitelist_anchors

        # Convert rule to anchors and encode
        anchors = await build_tool_whitelist_anchors(rule)

        logger.debug(f"ToolWhitelist rule encoded successfully")

        return anchors

    except ImportError:
        # Rule encoding functions not yet implemented
        logger.error("Rule encoding functions not implemented yet")
        raise HTTPException(
            status_code=501,
            detail="Rule encoding not yet implemented"
        )
    except Exception as e:
        logger.error(f"Failed to encode ToolWhitelist rule: {e}", exc_info=True)
        raise HTTPException(
            status_code=500,
            detail=f"Encoding failed: {str(e)}"
        )


@router.post("/rule/tool_param_constraint", response_model=RuleAnchors)
async def encode_tool_param_constraint_rule(rule: dict) -> RuleAnchors:
    """
    Encode a ToolParamConstraint rule to anchor arrays.

    Converts parameter constraints to natural language anchors, then embeds each anchor.
    Used by the Data Plane to cache rule embeddings for comparison.

    Args:
        rule: ToolParamConstraintRule dict with fields:
            - tool_id: Tool this constraint applies to
            - param_name: Parameter name
            - param_type: string, int, float, bool
            - max_len: Optional max length for strings
            - allowed_values: Optional list of allowed values
            - min_value, max_value: Optional numeric bounds
            - regex: Optional regex pattern
            - enforcement_mode: hard or soft

    Returns:
        RuleAnchors with embedded anchor arrays

    Raises:
        HTTPException: If encoding fails
    """
    try:
        logger.debug(f"Encoding ToolParamConstraint rule: {rule.get('rule_id', 'unknown')}")

        # Import rule-to-anchor conversion functions (will be implemented in next task)
        from app.rule_encoding import build_tool_param_constraint_anchors

        # Convert rule to anchors and encode
        anchors = await build_tool_param_constraint_anchors(rule)

        logger.debug(f"ToolParamConstraint rule encoded successfully")

        return anchors

    except ImportError:
        # Rule encoding functions not yet implemented
        logger.error("Rule encoding functions not implemented yet")
        raise HTTPException(
            status_code=501,
            detail="Rule encoding not yet implemented"
        )
    except Exception as e:
        logger.error(f"Failed to encode ToolParamConstraint rule: {e}", exc_info=True)
        raise HTTPException(
            status_code=500,
            detail=f"Encoding failed: {str(e)}"
        )
