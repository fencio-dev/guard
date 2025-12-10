"""
Rule-to-anchor conversion for Layer-Based Enforcement (v1.3).

Replaces template-based anchors with a unified LLM generator and shared
encoding helpers so every rule family uses the same anchor workflow.
"""

import logging
from typing import Any

import numpy as np

from app.encoding import get_encoder_model, get_projection_matrix
from app.llm_anchor_generator import AnchorSlots, get_llm_generator

logger = logging.getLogger(__name__)


def encode_anchor_text(text: str, slot_name: str, seed: int) -> np.ndarray:
    """
    Encode a single anchor text to a 32-dim vector (normalized).
    """
    model = get_encoder_model()
    projection = get_projection_matrix(slot_name, seed)

    embedding = model.encode(text, convert_to_numpy=True, normalize_embeddings=False)
    embedding = embedding.astype(np.float32)

    projected = projection @ embedding
    norm = np.linalg.norm(projected)
    if norm > 0:
        projected = projected / norm

    return projected


def encode_anchor_list(
    anchors: list[str],
    slot_name: str,
    seed: int,
    max_anchors: int = 16
) -> tuple[list[list[float]], int]:
    """
    Encode anchor strings into padded lists of 32-dim vectors.
    """
    if len(anchors) > max_anchors:
        logger.warning("Truncating %d anchors to %d for slot %s", len(anchors), max_anchors, slot_name)
        anchors = anchors[:max_anchors]

    encoded: list[list[float]] = []
    for anchor in anchors:
        vector = encode_anchor_text(anchor, slot_name, seed)
        encoded.append(vector.tolist())

    while len(encoded) < max_anchors:
        encoded.append([0.0] * 32)

    return encoded, len(anchors)


async def build_rule_anchors(rule: dict[str, Any], family_id: str) -> dict[str, Any]:
    """
    Generate anchor embeddings for any rule family via the LLM generator.
    """
    logger.info("Building anchors for %s rule %s", family_id, rule.get("rule_id"))
    llm_generator = get_llm_generator()
    anchor_slots = await llm_generator.generate_rule_anchors(rule, family_id)

    action_anchors, action_count = encode_anchor_list(anchor_slots.action, "action", 42)
    resource_anchors, resource_count = encode_anchor_list(anchor_slots.resource, "resource", 43)
    data_anchors, data_count = encode_anchor_list(anchor_slots.data, "data", 44)
    risk_anchors, risk_count = encode_anchor_list(anchor_slots.risk, "risk", 45)

    logger.debug(
        "Encoded %s rule: %d action, %d resource, %d data, %d risk anchors",
        family_id,
        action_count,
        resource_count,
        data_count,
        risk_count,
    )

    return {
        "action_anchors": action_anchors,
        "action_count": action_count,
        "resource_anchors": resource_anchors,
        "resource_count": resource_count,
        "data_anchors": data_anchors,
        "data_count": data_count,
        "risk_anchors": risk_anchors,
        "risk_count": risk_count,
    }


async def build_tool_whitelist_anchors(rule: dict[str, Any]) -> dict[str, Any]:
    """Build anchors for ToolWhitelist rules via the unified generator."""
    return await build_rule_anchors(rule, "tool_whitelist")


async def build_tool_param_constraint_anchors(rule: dict[str, Any]) -> dict[str, Any]:
    """Build anchors for ToolParamConstraint rules via the unified generator."""
    return await build_rule_anchors(rule, "tool_param_constraint")
