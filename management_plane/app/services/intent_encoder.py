"""
Intent Encoder for 128-dimensional semantic intent vectors.

Subclass of SemanticEncoder that:
1. Extracts 4 semantic slots from canonical IntentEvent: action, resource, data, risk
2. Encodes each slot to 32-dimensional vector
3. Concatenates to 128-dimensional intent vector
4. Per-slot normalization (not global)

The base class handles:
- Model loading (sentence-transformers)
- Embedding generation (384d)
- Projection matrix creation (sparse random projection)
- Caching

This class adds:
- Slot extraction from IntentEvent
- Text assembly for each slot
- Vector aggregation

Example:
    encoder = IntentEncoder()
    canonical_intent = IntentEvent(...)
    vector = encoder.encode(canonical_intent)  # Returns np.ndarray of shape (128,)
"""

import logging
from typing import Any

import numpy as np

from app.models import IntentEvent
from app.services.semantic_encoder import SemanticEncoder
from app.vocab import VOCABULARY

logger = logging.getLogger(__name__)


class IntentEncoder(SemanticEncoder):
    """
    Semantic encoder for IntentEvent to 128-dimensional vectors.

    Encodes canonical IntentEvent by:
    1. Building 4 slot strings (action, resource, data, risk)
    2. Encoding each to 384-dim
    3. Projecting each to 32-dim
    4. Concatenating to 128-dim
    5. Per-slot normalization (L2)
    """

    def __init__(self, embedding_model: str = SemanticEncoder.MODEL_NAME):
        """
        Initialize intent encoder.

        Args:
            embedding_model: Name of sentence-transformers model
        """
        super().__init__(embedding_model=embedding_model)

    def _build_action_slot(self, event: IntentEvent) -> str:
        """
        Build action slot string for encoding.

        Uses vocabulary templates to assemble canonical text.

        Args:
            event: Canonical IntentEvent

        Returns:
            Slot text string
        """
        fields = {
            "action": event.action,
            "actor_type": event.actor.type,
        }

        # Add tool call if available
        if event.tool_name:
            method = event.tool_method or "unspecified_method"
            fields["tool_call"] = f"{event.tool_name}.{method}"

        return VOCABULARY.assemble_anchor("action", fields)

    def _build_resource_slot(self, event: IntentEvent) -> str:
        """
        Build resource slot string for encoding.

        Args:
            event: Canonical IntentEvent

        Returns:
            Slot text string
        """
        fields: dict[str, Any] = {
            "resource_type": event.resource.type,
        }

        if event.resource.location:
            fields["resource_location"] = event.resource.location

        if event.resource.name:
            fields["resource_name"] = event.resource.name

        if event.tool_name:
            fields["tool_name"] = event.tool_name
            fields["tool_method"] = event.tool_method or event.action

        return VOCABULARY.assemble_anchor("resource", fields)

    def _build_data_slot(self, event: IntentEvent) -> str:
        """
        Build data slot string for encoding.

        Args:
            event: Canonical IntentEvent

        Returns:
            Slot text string
        """
        sensitivity = event.data.sensitivity[0] if event.data.sensitivity else "public"
        pii = event.data.pii if event.data.pii is not None else False
        volume = event.data.volume or "single"

        fields: dict[str, Any] = {
            "sensitivity": sensitivity,
            "pii": pii,
            "volume": volume,
        }

        # Add params_length if tool_params available
        if event.tool_params and event.tool_name:
            from app.encoding import canonicalize_dict

            canonical_params = canonicalize_dict(event.tool_params)
            if canonical_params:
                fields["params_length"] = "short" if len(canonical_params) <= 120 else "long"

        return VOCABULARY.assemble_anchor("data", fields)

    def _build_risk_slot(self, event: IntentEvent) -> str:
        """
        Build risk slot string for encoding.

        Args:
            event: Canonical IntentEvent

        Returns:
            Slot text string
        """
        fields = {"authn": event.risk.authn}
        return VOCABULARY.assemble_anchor("risk", fields)

    def encode(self, event: IntentEvent) -> np.ndarray:
        """
        Encode canonical IntentEvent to 128-dimensional vector.

        Steps:
        1. Build 4 slot strings (action, resource, data, risk)
        2. Encode each slot using base class method
        3. Concatenate to 128-dim
        4. Each slot is per-slot normalized (no global normalization)

        Args:
            event: Canonical IntentEvent

        Returns:
            128-dimensional vector (float32), per-slot normalized
        """
        # Build slot strings
        slot_texts = {
            "action": self._build_action_slot(event),
            "resource": self._build_resource_slot(event),
            "data": self._build_data_slot(event),
            "risk": self._build_risk_slot(event),
        }

        # Encode and project each slot
        slot_vectors = []
        for slot_name in ["action", "resource", "data", "risk"]:
            text = slot_texts[slot_name]
            slot_vector = self.encode_slot(text, slot_name)
            slot_vectors.append(slot_vector)

        # Concatenate to 128-dim
        vector_128 = np.concatenate(slot_vectors)

        return vector_128
