"""
Telemetry response models for Management Plane API.

Pydantic models for telemetry query responses, matching the gRPC proto
definitions from rule_installation.proto.
"""

from pydantic import BaseModel, Field
from typing import Any


class SessionSummary(BaseModel):
    """
    Summary of an enforcement session.
    
    Matches EnforcementSessionSummary from proto.
    
    Example:
        {
            "session_id": "session_001",
            "agent_id": "agent_123",
            "tenant_id": "tenant_abc",
            "layer": "L4",
            "timestamp_ms": 1700000000000,
            "final_decision": 1,
            "rules_evaluated_count": 3,
            "duration_us": 1250,
            "intent_summary": "web_search"
        }
    """
    session_id: str = Field(..., description="Unique session identifier")
    agent_id: str = Field(..., description="Agent that triggered enforcement")
    tenant_id: str = Field(..., description="Tenant ID")
    layer: str = Field(..., description="Layer (L0-L6)")
    timestamp_ms: int = Field(..., description="Unix timestamp in milliseconds")
    final_decision: int = Field(..., description="0=BLOCK, 1=ALLOW")
    rules_evaluated_count: int = Field(..., description="Number of rules evaluated")
    duration_us: int = Field(..., description="Enforcement duration in microseconds")
    intent_summary: str = Field(..., description="Tool name or action summary")


class TelemetrySessionsResponse(BaseModel):
    """
    Response for GET /sessions endpoint.
    
    Contains paginated list of session summaries with total count.
    
    Example:
        {
            "sessions": [...],
            "total_count": 42,
            "limit": 50,
            "offset": 0
        }
    """
    sessions: list[SessionSummary] = Field(..., description="List of session summaries")
    total_count: int = Field(..., description="Total number of matching sessions")
    limit: int = Field(..., description="Pagination limit")
    offset: int = Field(..., description="Pagination offset")


class SessionDetail(BaseModel):
    """
    Full details for a specific enforcement session.
    
    Contains the complete session data including all rule evaluations,
    intent details, and timing information.
    
    Example:
        {
            "session": {
                "session_id": "session_001",
                "agent_id": "agent_123",
                "final_decision": 1,
                "rules_evaluated": [...],
                "intent": {...}
            }
        }
    """
    session: dict[str, Any] = Field(..., description="Full session data as JSON object")
