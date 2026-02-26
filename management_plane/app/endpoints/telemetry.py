"""Telemetry query endpoints for management plane."""

import logging

from fastapi import APIRouter, HTTPException, Query

from app.services import session_store
from app.telemetry_models import TelemetrySessionsResponse, SessionDetail

logger = logging.getLogger(__name__)

router = APIRouter(tags=["telemetry"])


@router.get("/telemetry/sessions", response_model=TelemetrySessionsResponse)
def query_sessions(
    agent_id: str | None = Query(None),
    tenant_id: str | None = Query(None),
    decision: str | None = Query(None),
    layer: str | None = Query(None),
    start_time_ms: int | None = Query(None),
    end_time_ms: int | None = Query(None),
    limit: int = Query(50),
    offset: int = Query(0),
):
    result = session_store.list_sessions(
        limit=limit,
        offset=offset,
        agent_id=agent_id,
        decision=decision,
        start_time_ms=start_time_ms,
        end_time_ms=end_time_ms,
    )

    sessions = []
    for s in result["sessions"]:
        sessions.append(
            {
                "session_id": s["session_id"],
                "agent_id": s["agent_id"],
                "tenant_id": s.get("tenant_id") or tenant_id or "",
                "layer": s.get("layer") or layer or "",
                "timestamp_ms": s["last_seen_at_ms"],
                "final_decision": 1 if s["final_decision"] == "allow" else 0,
                "rules_evaluated_count": s["call_count"],
                "duration_us": 0,
                "intent_summary": s["final_decision"] or "",
            }
        )

    return TelemetrySessionsResponse(
        sessions=sessions,
        total_count=result["total_count"],
        limit=result["limit"],
        offset=result["offset"],
    )


@router.get("/telemetry/sessions/{session_id}", response_model=SessionDetail)
def get_session_detail(
    session_id: str,
):
    session = session_store.get_session(session_id)
    if session is None:
        raise HTTPException(status_code=404, detail="Session not found")

    return SessionDetail(session=session)
