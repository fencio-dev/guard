"""
Data contract type definitions for the Tupl Python SDK.

This module defines Pydantic models for capturing and sending IntentEvents.
Types are synchronized with the Management Plane canonical schemas.

Key constraints:
- No Dict[str, Any] fields (Google GenAI compatibility)
- All fields explicitly typed
- Deterministic serialization
"""

from pydantic import BaseModel, Field, ConfigDict
from typing import Literal, Optional


# ============================================================================
# IntentEvent Types
# ============================================================================

class Actor(BaseModel):
    """
    Represents the entity initiating an action.

    v1.2: Added "llm" and "agent" actor types for AI/autonomous systems.

    Examples:
        {"id": "user-123", "type": "user"}
        {"id": "gpt-4", "type": "llm"}
        {"id": "agent-123", "type": "agent"}
    """
    id: str
    type: Literal["user", "service", "llm", "agent"]


class Resource(BaseModel):
    """
    Describes the resource being accessed (v1.1 simplified).

    MVP vocabulary: database, file, api only.

    Example:
        {"type": "database", "name": "users_db", "location": "cloud"}
    """
    type: Literal["database", "file", "api"]
    name: Optional[str] = None
    location: Optional[Literal["local", "cloud"]] = None


class Data(BaseModel):
    """
    Describes the data characteristics of the operation (v1.1 simplified).

    MVP: Simplified to sensitivity (internal/public), pii flag, and volume (single/bulk).

    Example:
        {"sensitivity": ["internal"], "pii": false, "volume": "single"}
    """
    sensitivity: list[Literal["internal", "public"]]
    pii: Optional[bool] = None
    volume: Optional[Literal["single", "bulk"]] = None


class Risk(BaseModel):
    """
    Describes the risk context of the operation (v1.1 simplified).

    MVP: Only authentication requirement (required/not_required).

    Example:
        {"authn": "required"}
    """
    authn: Literal["required", "not_required"]


class RateLimitContext(BaseModel):
    """
    Rate limit tracking context (v1.3).

    Tracks the number of calls within a time window for rate limit enforcement.

    Example:
        {"agent_id": "agent-123", "window_start": 1699564800.0, "call_count": 5}
    """
    agent_id: str
    window_start: float  # Unix timestamp
    call_count: int = 0


class IntentEvent(BaseModel):
    """
    Structured record of an LLM/tool call intent (v1.3 with layer-based enforcement).

    This is the canonical IntentEvent schema.
    Captured by SDKs and sent to the Management Plane for encoding and comparison.

    v1.2: Added "llm" and "agent" actor types for AI/autonomous systems.
    v1.3: Added layer-based enforcement fields (layer, tool_name, tool_method,
          tool_params, rate_limit_context) for Data Plane rule enforcement.

    Example:
        {
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "schemaVersion": "v1.3",
            "tenantId": "tenant-123",
            "timestamp": 1699564800.0,
            "actor": {"id": "agent-123", "type": "agent"},
            "action": "read",
            "resource": {"type": "database", "name": "users_db", "location": "cloud"},
            "data": {"sensitivity": ["internal"], "pii": false, "volume": "single"},
            "risk": {"authn": "required"},
            "layer": "L4",
            "tool_name": "web_search",
            "tool_method": "query",
            "tool_params": {"query": "example search"},
            "rate_limit_context": {"agent_id": "agent-123", "window_start": 1699564800.0, "call_count": 5}
        }
    """
    # Existing v1.2 fields
    id: str  # UUID
    schemaVersion: Literal["v1.1", "v1.2", "v1.3"] = "v1.3"  # v1.3: Added support for v1.3
    tenantId: str
    timestamp: float  # Unix timestamp
    actor: Actor
    action: Literal["read", "write", "delete", "export", "execute", "update"]
    resource: Resource
    data: Data
    risk: Risk
    context: Optional[dict] = None  # Future extensibility

    # NEW v1.3 fields for layer-based enforcement
    layer: Optional[str] = None  # "L0", "L1", ..., "L6"
    tool_name: Optional[str] = None
    tool_method: Optional[str] = None
    tool_params: Optional[dict] = None
    rate_limit_context: Optional[RateLimitContext] = None


# ============================================================================
# Response Types
# ============================================================================

class BoundaryEvidence(BaseModel):
    """
    Evidence about a boundary's evaluation for debugging and audit purposes.

    Provides visibility into which boundaries were evaluated and how they contributed
    to the final decision.

    Fields:
    - boundary_id: Unique identifier for the boundary
    - boundary_name: Human-readable boundary name
    - effect: Policy effect (allow or deny)
    - decision: Individual boundary decision (0 = block, 1 = allow)
    - similarities: Per-slot similarity scores [action, resource, data, risk]

    Example:
        {
            "boundary_id": "allow-read-ops",
            "boundary_name": "Allow Read Operations",
            "effect": "allow",
            "decision": 1,
            "similarities": [0.92, 0.88, 0.85, 0.90]
        }
    """
    boundary_id: str
    boundary_name: str
    effect: Literal["allow", "deny"]
    decision: Literal[0, 1]
    similarities: list[float] = Field(min_length=4, max_length=4)


class ComparisonResult(BaseModel):
    """
    Result from Management Plane comparison with boundary evidence.

    Fields:
    - decision: 0 = block, 1 = allow
    - slice_similarities: Per-slot similarity scores [action, resource, data, risk]
    - boundaries_evaluated: Number of boundaries evaluated (for diagnostics)
    - timestamp: Unix timestamp of comparison
    - evidence: List of boundary evaluations (for debugging/audit)

    Example:
        {
            "decision": 1,
            "slice_similarities": [0.92, 0.88, 0.85, 0.90],
            "boundaries_evaluated": 3,
            "timestamp": 1699564800.0,
            "evidence": [...]
        }
    """
    decision: int = Field(ge=0, le=1)  # 0 = block, 1 = allow
    slice_similarities: list[float] = Field(min_length=4, max_length=4)
    boundaries_evaluated: int = Field(default=0, ge=0)
    timestamp: float = Field(default=0.0)
    evidence: list[BoundaryEvidence] = Field(default_factory=list)

    model_config = ConfigDict(
        json_schema_extra={
            "example": {
                "decision": 1,
                "slice_similarities": [0.92, 0.88, 0.85, 0.90],
                "boundaries_evaluated": 3,
                "timestamp": 1699564800.0,
                "evidence": []
            }
        }
    )


# ============================================================================
# Validation Vocabularies (v1.2)
# ============================================================================

VALID_ACTIONS = {"read", "write", "delete", "export", "execute", "update"}
VALID_ACTOR_TYPES = {"user", "service", "llm", "agent"}  # v1.2: Added llm, agent
VALID_RESOURCE_TYPES = {"database", "file", "api"}  # v1.1: Simplified
VALID_RESOURCE_LOCATIONS = {"local", "cloud"}  # v1.1: Simplified
VALID_DATA_SENSITIVITY = {"internal", "public"}  # v1.1: Simplified from categories
VALID_DATA_VOLUMES = {"single", "bulk"}  # v1.1: Simplified
VALID_AUTHN_LEVELS = {"required", "not_required"}  # v1.1: Simplified
