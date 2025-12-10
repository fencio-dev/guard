"""
Test suite for telemetry query API endpoints.

Tests the Management Plane's HTTP API for querying enforcement session data
from the Data Plane via gRPC.

TDD Approach:
1. Write failing tests (RED)
2. Implement minimal code to pass (GREEN)
3. Verify all tests pass
"""

from __future__ import annotations

import json
from unittest.mock import MagicMock, patch

import pytest
from fastapi.testclient import TestClient

from app.main import app
from app.auth import get_current_user, User


# Override authentication for tests
def override_get_current_user():
    """Mock user for testing."""
    return User(id="test_user_123", email="test@example.com", role="authenticated")


app.dependency_overrides[get_current_user] = override_get_current_user

client = TestClient(app)


# ============================================================================
# Test Data Fixtures
# ============================================================================

@pytest.fixture
def mock_grpc_session_summary():
    """Mock gRPC EnforcementSessionSummary proto message."""
    mock = MagicMock()
    mock.session_id = "session_001"
    mock.agent_id = "agent_123"
    mock.tenant_id = "tenant_abc"
    mock.layer = "L4"
    mock.timestamp_ms = 1700000000000
    mock.final_decision = 1  # ALLOW
    mock.rules_evaluated_count = 3
    mock.duration_us = 1250
    mock.intent_summary = "web_search"
    return mock


@pytest.fixture
def mock_grpc_query_response(mock_grpc_session_summary):
    """Mock gRPC QueryTelemetryResponse."""
    mock = MagicMock()
    mock.sessions = [mock_grpc_session_summary]
    mock.total_count = 1
    return mock


@pytest.fixture
def mock_grpc_session_detail():
    """Mock gRPC GetSessionResponse with full session JSON."""
    mock = MagicMock()
    session_data = {
        "session_id": "session_001",
        "agent_id": "agent_123",
        "tenant_id": "tenant_abc",
        "layer": "L4",
        "timestamp_ms": 1700000000000,
        "final_decision": 1,
        "rules_evaluated": [
            {
                "rule_id": "rule_001",
                "decision": 1,
                "similarities": [0.92, 0.88, 0.85, 0.90]
            }
        ],
        "duration_us": 1250,
        "intent": {
            "id": "intent_123",
            "action": "read",
            "tool_name": "web_search"
        }
    }
    mock.session_json = json.dumps(session_data)
    return mock


# ============================================================================
# Test: GET /api/v1/sessions - Query Telemetry
# ============================================================================

@patch("app.endpoints.telemetry.get_data_plane_client")
def test_query_sessions_returns_200_with_sessions(mock_get_client, mock_grpc_query_response):
    """Test GET /sessions returns 200 with session summaries."""
    # Setup mock
    mock_client = MagicMock()
    mock_client.query_telemetry.return_value = mock_grpc_query_response
    mock_get_client.return_value = mock_client

    # Make request
    response = client.get("/api/v1/telemetry/sessions")

    # Assertions
    assert response.status_code == 200
    data = response.json()
    
    assert "sessions" in data
    assert "total_count" in data
    assert "limit" in data
    assert "offset" in data
    
    assert data["total_count"] == 1
    assert len(data["sessions"]) == 1
    
    session = data["sessions"][0]
    assert session["session_id"] == "session_001"
    assert session["agent_id"] == "agent_123"
    assert session["tenant_id"] == "tenant_abc"
    assert session["layer"] == "L4"
    assert session["timestamp_ms"] == 1700000000000
    assert session["final_decision"] == 1
    assert session["rules_evaluated_count"] == 3
    assert session["duration_us"] == 1250
    assert session["intent_summary"] == "web_search"


@patch("app.endpoints.telemetry.get_data_plane_client")
def test_query_sessions_with_filters(mock_get_client, mock_grpc_query_response):
    """Test GET /sessions with query parameters."""
    mock_client = MagicMock()
    mock_client.query_telemetry.return_value = mock_grpc_query_response
    mock_get_client.return_value = mock_client

    # Make request with filters
    response = client.get(
        "/api/v1/telemetry/sessions",
        params={
            "agent_id": "agent_123",
            "tenant_id": "tenant_abc",
            "decision": 1,
            "layer": "L4",
            "start_time_ms": 1700000000000,
            "end_time_ms": 1700100000000,
            "limit": 10,
            "offset": 0
        }
    )

    # Assertions
    assert response.status_code == 200
    
    # Verify gRPC client was called with correct params
    mock_client.query_telemetry.assert_called_once()
    call_args = mock_client.query_telemetry.call_args[1]
    assert call_args["agent_id"] == "agent_123"
    assert call_args["tenant_id"] == "tenant_abc"
    assert call_args["decision"] == 1
    assert call_args["layer"] == "L4"
    assert call_args["start_time_ms"] == 1700000000000
    assert call_args["end_time_ms"] == 1700100000000
    assert call_args["limit"] == 10
    assert call_args["offset"] == 0


@patch("app.endpoints.telemetry.get_data_plane_client")
def test_query_sessions_default_pagination(mock_get_client, mock_grpc_query_response):
    """Test GET /sessions uses default pagination values."""
    mock_client = MagicMock()
    mock_client.query_telemetry.return_value = mock_grpc_query_response
    mock_get_client.return_value = mock_client

    response = client.get("/api/v1/telemetry/sessions")

    assert response.status_code == 200
    data = response.json()
    
    # Default values
    assert data["limit"] == 50
    assert data["offset"] == 0


@patch("app.endpoints.telemetry.get_data_plane_client")
def test_query_sessions_max_limit_enforced(mock_get_client, mock_grpc_query_response):
    """Test GET /sessions enforces max limit of 500."""
    mock_client = MagicMock()
    mock_client.query_telemetry.return_value = mock_grpc_query_response
    mock_get_client.return_value = mock_client

    response = client.get("/api/v1/telemetry/sessions", params={"limit": 1000})

    assert response.status_code == 200
    
    # Verify limit was capped at 500
    call_args = mock_client.query_telemetry.call_args[1]
    assert call_args["limit"] == 500


@patch("app.endpoints.telemetry.get_data_plane_client")
def test_query_sessions_empty_results(mock_get_client):
    """Test GET /sessions with no results."""
    mock_client = MagicMock()
    mock_response = MagicMock()
    mock_response.sessions = []
    mock_response.total_count = 0
    mock_client.query_telemetry.return_value = mock_response
    mock_get_client.return_value = mock_client

    response = client.get("/api/v1/telemetry/sessions")

    assert response.status_code == 200
    data = response.json()
    assert data["sessions"] == []
    assert data["total_count"] == 0


# ============================================================================
# Test: GET /api/v1/sessions/{session_id} - Get Session Detail
# ============================================================================

@patch("app.endpoints.telemetry.get_data_plane_client")
def test_get_session_returns_200_with_detail(mock_get_client, mock_grpc_session_detail):
    """Test GET /sessions/{id} returns 200 with full session details."""
    mock_client = MagicMock()
    mock_client.get_session.return_value = mock_grpc_session_detail
    mock_get_client.return_value = mock_client

    response = client.get("/api/v1/telemetry/sessions/session_001")

    assert response.status_code == 200
    data = response.json()
    
    assert "session" in data
    session = data["session"]
    
    # Verify JSON was parsed correctly
    assert session["session_id"] == "session_001"
    assert session["agent_id"] == "agent_123"
    assert session["layer"] == "L4"
    assert session["final_decision"] == 1
    assert "rules_evaluated" in session
    assert "intent" in session


@patch("app.endpoints.telemetry.get_data_plane_client")
def test_get_session_not_found(mock_get_client):
    """Test GET /sessions/{id} returns 404 when session not found."""
    mock_client = MagicMock()
    mock_client.get_session.side_effect = Exception("Session not found")
    mock_get_client.return_value = mock_client

    response = client.get("/api/v1/telemetry/sessions/nonexistent")

    assert response.status_code == 404
    data = response.json()
    assert "detail" in data


@patch("app.endpoints.telemetry.get_data_plane_client")
def test_get_session_invalid_json(mock_get_client):
    """Test GET /sessions/{id} handles invalid JSON gracefully."""
    mock_client = MagicMock()
    mock_response = MagicMock()
    mock_response.session_json = "invalid json {"
    mock_client.get_session.return_value = mock_response
    mock_get_client.return_value = mock_client

    response = client.get("/api/v1/telemetry/sessions/session_001")

    assert response.status_code == 500
    data = response.json()
    assert "detail" in data


# ============================================================================
# Test: gRPC Client Error Handling
# ============================================================================

@patch("app.endpoints.telemetry.get_data_plane_client")
def test_query_sessions_grpc_error(mock_get_client):
    """Test GET /sessions handles gRPC errors gracefully."""
    mock_client = MagicMock()
    mock_client.query_telemetry.side_effect = Exception("gRPC connection failed")
    mock_get_client.return_value = mock_client

    response = client.get("/api/v1/telemetry/sessions")

    assert response.status_code == 500
    data = response.json()
    assert "detail" in data
