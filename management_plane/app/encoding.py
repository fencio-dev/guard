"""
Encoding Pipeline for Semantic Security

Converts IntentEvent and DesignBoundary into 128-dimensional vectors for comparison.

Architecture:
1. Canonicalize: Convert structured data to deterministic text representation
2. Build slot strings: Create 4 text strings (action, resource, data, risk)
3. Embed: Use sentence-transformers to get 384-dim embeddings
4. Project: Use sparse random projection to get 4×32 = 128 dims
5. Normalize: L2 normalize for cosine similarity

Performance:
- Target: <10ms per encoding
- LRU caching for boundary vectors
- Singleton pattern for model loading
"""

import logging
from functools import lru_cache
from typing import Any

import numpy as np
from sentence_transformers import SentenceTransformer

from app.models import IntentEvent, DesignBoundary
from app.vocab import VOCABULARY

logger = logging.getLogger(__name__)

# Global model instance (lazy loaded)
_MODEL: SentenceTransformer | None = None
_PROJECTION_MATRICES: dict[str, np.ndarray] = {}

def get_encoder_model() -> SentenceTransformer:
    """
    Get singleton sentence-transformers model.

    Using all-MiniLM-L6-v2:
    - 384 dimensions
    - Fast inference (~10ms on CPU)
    - Good semantic quality
    - Small model size (~90MB)
    """
    global _MODEL
    if _MODEL is None:
        logger.info("Loading sentence-transformers model: all-MiniLM-L6-v2")
        _MODEL = SentenceTransformer('sentence-transformers/all-MiniLM-L6-v2')
        logger.info("Model loaded successfully")
    return _MODEL


def create_sparse_projection_matrix(
    input_dim: int,
    output_dim: int,
    seed: int,
    sparsity: float = 0.66
) -> np.ndarray:
    """
    Create sparse random projection matrix for dimensionality reduction.

    Uses the sparse random projection technique from algo.md:
    - Each element is +√3 (prob 1/6), 0 (prob 2/3), or -√3 (prob 1/6)
    - Preserves distances (Johnson-Lindenstrauss lemma)
    - Deterministic with fixed seed

    Args:
        input_dim: Input dimensionality (e.g., 384 from sentence-transformers)
        output_dim: Output dimensionality (e.g., 32 for each slot)
        seed: Random seed for determinism
        sparsity: Fraction of zeros (default 2/3)

    Returns:
        Sparse projection matrix of shape (output_dim, input_dim)
    """
    rng = np.random.RandomState(seed)
    s = 1 / (1 - sparsity)  # s = 3 for sparsity=0.66
    sqrt_s = np.sqrt(s)

    # Probabilities: [+sqrt(s), 0, -sqrt(s)]
    prob_pos = 1 / (2 * s)  # 1/6
    prob_zero = 1 - 1 / s    # 2/3
    prob_neg = 1 / (2 * s)  # 1/6

    matrix = rng.choice(
        [sqrt_s, 0.0, -sqrt_s],
        size=(output_dim, input_dim),
        p=[prob_pos, prob_zero, prob_neg]
    )

    return matrix.astype(np.float32)


def get_projection_matrix(slot_name: str, seed: int) -> np.ndarray:
    """
    Get or create projection matrix for a specific slot.

    Cached globally to ensure same matrix is used across all encodings.

    Args:
        slot_name: Name of the slot (action, resource, data, risk)
        seed: Random seed (42, 43, 44, 45 for the 4 slots)

    Returns:
        Projection matrix of shape (32, 384)
    """
    global _PROJECTION_MATRICES

    if slot_name not in _PROJECTION_MATRICES:
        logger.info(f"Creating projection matrix for slot '{slot_name}' with seed {seed}")
        _PROJECTION_MATRICES[slot_name] = create_sparse_projection_matrix(
            input_dim=384,  # all-MiniLM-L6-v2 output
            output_dim=32,  # Each slot gets 32 dims
            seed=seed,
            sparsity=0.66
        )

    return _PROJECTION_MATRICES[slot_name]


def canonicalize_dict(data: dict[str, Any]) -> str:
    """
    Convert dictionary to deterministic canonical string.

    Rules:
    1. Sort keys alphabetically
    2. Flatten nested structures with dot notation
    3. Handle lists with index notation
    4. Remove null/None values

    Args:
        data: Dictionary to canonicalize

    Returns:
        Deterministic string representation
    """
    def flatten(obj: Any, prefix: str = "") -> list[tuple[str, Any]]:
        """Recursively flatten nested structures."""
        items = []

        if isinstance(obj, dict):
            for key in sorted(obj.keys()):  # Sort for determinism
                value = obj[key]
                if value is None:
                    continue  # Skip None values

                new_key = f"{prefix}.{key}" if prefix else key
                items.extend(flatten(value, new_key))

        elif isinstance(obj, list):
            for idx, item in enumerate(obj):
                new_key = f"{prefix}[{idx}]"
                items.extend(flatten(item, new_key))

        else:
            # Leaf value - convert to string
            items.append((prefix, str(obj)))

        return items

    # Flatten and create key=value pairs
    flattened = flatten(data)
    pairs = [f"{key}={value}" for key, value in flattened]

    # Join with semicolons
    return "; ".join(pairs)


def _format_tool_call(event: IntentEvent) -> str | None:
    """
    Create a deterministic representation of the tool call.

    Returns:
        "tool_name.tool_method" or None if tool_name is not set.
    """
    if not event.tool_name:
        return None

    method = event.tool_method or "unspecified_method"
    return f"{event.tool_name}.{method}"



def build_action_slot(event: IntentEvent) -> str:
    """
    Build the action slot string using vocabulary templates.
    """
    fields = {
        "action": event.action,
        "actor_type": event.actor.type,
    }

    tool_call = _format_tool_call(event)
    if tool_call:
        fields["tool_call"] = tool_call

    return VOCABULARY.assemble_anchor("action", fields)


def build_resource_slot(event: IntentEvent) -> str:
    """
    Build the resource slot string using vocabulary templates.
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

def build_data_slot(event: IntentEvent) -> str:
    """
    Build the data slot string using vocabulary templates.
    """
    sensitivity = event.data.sensitivity[0] if event.data.sensitivity else "public"
    pii = event.data.pii if event.data.pii is not None else False
    volume = event.data.volume or "single"

    fields: dict[str, Any] = {
        "sensitivity": sensitivity,
        "pii": pii,
        "volume": volume,
    }

    if event.tool_params and event.tool_name:
        canonical = canonicalize_dict(event.tool_params)
        if canonical:
            fields["params_length"] = "short" if len(canonical) <= 120 else "long"

    return VOCABULARY.assemble_anchor("data", fields)


def build_risk_slot(event: IntentEvent) -> str:
    """
    Build the risk slot string using vocabulary templates.
    """
    fields = {"authn": event.risk.authn}
    return VOCABULARY.assemble_anchor("risk", fields)


def build_boundary_action_slot(boundary: DesignBoundary) -> str:
    """
    Build action slot text for a DesignBoundary (v1.1 with constraints).

    Encodes allowed actions and actor types using same vocabulary as intents.

    Returns:
        Canonical string matching intent action slot format
    """
    # Encode allowed actions (sorted for determinism)
    actions_str = ", ".join(sorted(boundary.constraints.action.actions))
    actor_types_str = ", ".join(sorted(boundary.constraints.action.actor_types))

    parts = [
        f"action: {actions_str}",
        f"actor_type: {actor_types_str}",
    ]

    return " | ".join(parts)


def build_boundary_resource_slot(boundary: DesignBoundary) -> str:
    """
    Build resource slot text for a DesignBoundary (v1.1 with constraints).

    Encodes allowed resource types, names, and locations using same vocabulary as intents.

    Returns:
        Canonical string matching intent resource slot format
    """
    parts = []

    # Encode allowed resource types (sorted for determinism)
    types_str = ", ".join(sorted(boundary.constraints.resource.types))
    parts.append(f"resource_type: {types_str}")

    # Encode allowed resource names if specified
    if boundary.constraints.resource.names:
        names_str = ", ".join(sorted(boundary.constraints.resource.names))
        parts.append(f"resource_name: {names_str}")

    # Encode allowed locations if specified
    if boundary.constraints.resource.locations:
        locations_str = ", ".join(sorted(boundary.constraints.resource.locations))
        parts.append(f"resource_location: {locations_str}")

    return " | ".join(parts)


def build_boundary_data_slot(boundary: DesignBoundary) -> str:
    """
    Build data slot text for a DesignBoundary (v1.1 with constraints).

    Encodes allowed data sensitivity, pii, and volume using same vocabulary as intents.

    Returns:
        Canonical string matching intent data slot format
    """
    parts = []

    # Encode allowed sensitivity levels (sorted for determinism)
    sensitivity_str = ", ".join(sorted(boundary.constraints.data.sensitivity))
    parts.append(f"sensitivity: {sensitivity_str}")

    # Encode PII constraint if specified
    if boundary.constraints.data.pii is not None:
        parts.append(f"pii: {boundary.constraints.data.pii}")

    # Encode volume constraint if specified
    if boundary.constraints.data.volume:
        parts.append(f"volume: {boundary.constraints.data.volume}")

    return " | ".join(parts)


def build_boundary_risk_slot(boundary: DesignBoundary) -> str:
    """
    Build risk slot text for a DesignBoundary (v1.1 with constraints).

    Encodes authentication requirement using same vocabulary as intents.

    Returns:
        Canonical string matching intent risk slot format
    """
    parts = [
        f"authn: {boundary.constraints.risk.authn}",
    ]

    return " | ".join(parts)


@lru_cache(maxsize=10000)
def encode_text_cached(text: str) -> np.ndarray:
    """
    Encode text to 384-dim vector with LRU caching.

    Cache improves performance when encoding similar boundaries or intents.

    Args:
        text: Input text string

    Returns:
        384-dimensional embedding vector (float32)
    """
    model = get_encoder_model()
    embedding = model.encode(text, convert_to_numpy=True, show_progress_bar=False)
    return embedding.astype(np.float32)


@lru_cache(maxsize=1000)
def encode_boundary_to_128d_cached(boundary_id: str, boundary_json: str) -> np.ndarray:
    """
    Encode DesignBoundary to 128-dimensional vector with LRU caching.

    This cache is separate from encode_boundary_to_128d() to allow caching by
    boundary ID. The boundary_json parameter ensures cache invalidation when
    boundary changes.

    Args:
        boundary_id: Unique identifier for the boundary
        boundary_json: JSON representation of boundary (for cache invalidation)

    Returns:
        128-dimensional normalized vector (float32)
    """
    # Import here to avoid circular dependency
    from app.models import DesignBoundary
    import json

    # Reconstruct boundary from JSON
    boundary_dict = json.loads(boundary_json)
    boundary = DesignBoundary(**boundary_dict)

    # Encode using the main function
    return encode_boundary_to_128d(boundary)


def encode_to_128d(event: IntentEvent) -> np.ndarray:
    """
    Encode IntentEvent to 128-dimensional vector.

    Steps:
    1. Build 4 slot strings (action, resource, data, risk)
    2. Encode each to 384-dim using sentence-transformers
    3. Project each to 32-dim using sparse random projection
    4. Concatenate to 128-dim
    5. L2 normalize

    Args:
        event: IntentEvent to encode

    Returns:
        128-dimensional normalized vector (float32)
    """
    # Build slot strings
    slot_texts = {
        "action": build_action_slot(event),
        "resource": build_resource_slot(event),
        "data": build_data_slot(event),
        "risk": build_risk_slot(event),
    }

    # Encode and project each slot
    slot_seeds = {"action": 42, "resource": 43, "data": 44, "risk": 45}
    slot_vectors = []

    for slot_name in ["action", "resource", "data", "risk"]:
        # Encode text to 384-dim
        text = slot_texts[slot_name]
        embedding_384 = encode_text_cached(text)

        # Project to 32-dim
        projection_matrix = get_projection_matrix(slot_name, slot_seeds[slot_name])
        projected_32 = projection_matrix @ embedding_384

        # Normalize per-slot (Phase 1 fix)
        norm = np.linalg.norm(projected_32)
        if norm > 0:
            projected_32 = projected_32 / norm

        slot_vectors.append(projected_32)

    # Concatenate to 128-dim
    vector_128 = np.concatenate(slot_vectors)

    # Note: Global normalization removed in Phase 1 - each slot is now unit-normalized
    return vector_128


def encode_boundary_to_128d(boundary: DesignBoundary) -> np.ndarray:
    """
    Encode DesignBoundary to 128-dimensional vector.

    Similar to IntentEvent encoding but uses boundary-specific slot builders.

    Args:
        boundary: DesignBoundary to encode

    Returns:
        128-dimensional normalized vector (float32)
    """
    # Build slot strings
    slot_texts = {
        "action": build_boundary_action_slot(boundary),
        "resource": build_boundary_resource_slot(boundary),
        "data": build_boundary_data_slot(boundary),
        "risk": build_boundary_risk_slot(boundary),
    }

    # Encode and project each slot
    slot_seeds = {"action": 42, "resource": 43, "data": 44, "risk": 45}
    slot_vectors = []

    for slot_name in ["action", "resource", "data", "risk"]:
        # Encode text to 384-dim
        text = slot_texts[slot_name]
        embedding_384 = encode_text_cached(text)

        # Project to 32-dim
        projection_matrix = get_projection_matrix(slot_name, slot_seeds[slot_name])
        projected_32 = projection_matrix @ embedding_384

        # Normalize per-slot (Phase 1 fix)
        norm = np.linalg.norm(projected_32)
        if norm > 0:
            projected_32 = projected_32 / norm

        slot_vectors.append(projected_32)

    # Concatenate to 128-dim
    vector_128 = np.concatenate(slot_vectors)

    # Note: Global normalization removed in Phase 1 - each slot is now unit-normalized
    return vector_128


# Phase 2: Anchor Generation Functions

def build_boundary_action_anchors(boundary: DesignBoundary) -> list[str]:
    """
    Build canonical anchor strings for action slot.

    Returns one string per (action, actor_type) combination.
    Uses atomic templates for semantic alignment with intents.

    Example:
        actions = ["read", "write"]
        actor_types = ["user", "agent"]

        Returns:
            [
                "action is read | actor_type equals user",
                "action is read | actor_type equals agent",
                "action is write | actor_type equals user",
                "action is write | actor_type equals agent"
            ]
    """
    anchors = []
    for action in sorted(boundary.constraints.action.actions):
        for actor_type in sorted(boundary.constraints.action.actor_types):
            anchor = f"action is {action} | actor_type equals {actor_type}"
            anchors.append(anchor)
    return anchors


def build_boundary_resource_anchors(boundary: DesignBoundary) -> list[str]:
    """Build canonical anchor strings for resource slot."""
    anchors = []

    # Generate anchors for each combination of type × location
    types = sorted(boundary.constraints.resource.types)
    locations = sorted(boundary.constraints.resource.locations or ["unspecified"])

    for rtype in types:
        for location in locations:
            anchor = f"resource_type is {rtype} | resource_location is {location}"
            anchors.append(anchor)

    # If specific names are constrained, add them
    if boundary.constraints.resource.names:
        for name in sorted(boundary.constraints.resource.names):
            anchor = f"resource_name is {name}"
            anchors.append(anchor)

    return anchors


def build_boundary_data_anchors(boundary: DesignBoundary) -> list[str]:
    """Build canonical anchor strings for data slot."""
    anchors = []

    # Generate anchors for each combination of sensitivity × pii × volume
    sensitivities = sorted(boundary.constraints.data.sensitivity)
    pii_values = [boundary.constraints.data.pii] if boundary.constraints.data.pii is not None else [True, False]
    volumes = [boundary.constraints.data.volume] if boundary.constraints.data.volume else ["single", "bulk"]

    for sensitivity in sensitivities:
        for pii in pii_values:
            for volume in volumes:
                anchor = f"sensitivity is {sensitivity} | pii is {pii} | volume is {volume}"
                anchors.append(anchor)

    return anchors


def build_boundary_risk_anchors(boundary: DesignBoundary) -> list[str]:
    """Build canonical anchor strings for risk slot."""
    # Risk slot is simple - just authn values
    authn = boundary.constraints.risk.authn
    return [f"authn is {authn}"]


def encode_anchors_to_32d(
    anchor_texts: list[str],
    slot_name: str,
    slot_seed: int,
    max_anchors: int = 16
) -> tuple[np.ndarray, int]:
    """
    Encode list of anchor texts to normalized 32-d vectors.

    Args:
        anchor_texts: List of canonical anchor strings
        slot_name: Name of slot (for logging)
        slot_seed: Random seed for projection matrix
        max_anchors: Maximum number of anchors (truncate if exceeded)

    Returns:
        Tuple of (anchor_array, count) where:
        - anchor_array: np.ndarray of shape (max_anchors, 32) with padding
        - count: Actual number of anchors (before padding)
    """
    if len(anchor_texts) > max_anchors:
        logger.warning(
            f"Boundary {slot_name} slot has {len(anchor_texts)} anchors, "
            f"truncating to {max_anchors}"
        )
        anchor_texts = anchor_texts[:max_anchors]

    # Encode each anchor text
    anchor_vecs = []
    for text in anchor_texts:
        # Encode to 384-d
        embedding_384 = encode_text_cached(text)

        # Project to 32-d
        projection_matrix = get_projection_matrix(slot_name, slot_seed)
        projected_32 = projection_matrix @ embedding_384

        # Normalize per-slot
        norm = np.linalg.norm(projected_32)
        if norm > 0:
            projected_32 = projected_32 / norm

        anchor_vecs.append(projected_32)

    # Pad to max_anchors with zeros
    anchor_array = np.zeros((max_anchors, 32), dtype=np.float32)
    for i, vec in enumerate(anchor_vecs):
        anchor_array[i] = vec

    return anchor_array, len(anchor_texts)


def get_cache_stats() -> dict[str, Any]:
    """
    Get encoding cache statistics.

    Returns:
        Dictionary with cache hits, misses, and size
    """
    cache_info = encode_text_cached.cache_info()
    return {
        "hits": cache_info.hits,
        "misses": cache_info.misses,
        "size": cache_info.currsize,
        "maxsize": cache_info.maxsize,
    }


def clear_cache() -> None:
    """Clear the encoding cache."""
    encode_text_cached.cache_clear()
    logger.info("Encoding cache cleared")
