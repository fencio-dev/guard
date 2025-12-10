"""
Tests for AgentCallback (LangGraph integration).

Following TDD approach: Write tests first, then implement AgentCallback.
"""

import pytest
import time
from unittest.mock import Mock, patch, MagicMock
from uuid import uuid4

# Import the types we need
import sys
sys.path.insert(0, '/Users/sid/Projects/mgmt-plane/tupl_sdk/python')

from tupl.types import IntentEvent, Actor, Resource, Data, Risk, ComparisonResult


# ============================================================================
# Test Fixtures
# ============================================================================

@pytest.fixture
def mock_tupl_client():
    """Mock TuplClient for testing."""
    client = Mock()
    client.capture = Mock(return_value=ComparisonResult(
        decision=1,
        slice_similarities=[0.9, 0.9, 0.9, 0.9]
    ))
    return client


@pytest.fixture
def agent_callback(mock_tupl_client):
    """Create AgentCallback instance for testing."""
    from tupl.agent import AgentCallback

    callback = AgentCallback(
        base_url="http://test:8000",
        tenant_id="test-tenant"
    )
    # Replace client with mock
    callback.client = mock_tupl_client
    return callback


# ============================================================================
# Test: AgentCallback Initialization
# ============================================================================

def test_agent_callback_initialization():
    """Test AgentCallback can be initialized with required parameters."""
    from tupl.agent import AgentCallback

    callback = AgentCallback(
        base_url="http://localhost:8000",
        tenant_id="test-tenant"
    )

    assert callback.base_url == "http://localhost:8000"
    assert callback.tenant_id == "test-tenant"
    assert callback.enforcement_mode == "warn"  # default
    assert callback.capture_llm is True  # default
    assert callback.capture_tools is True  # default
    assert callback.capture_state is False  # default


def test_agent_callback_full_configuration():
    """Test AgentCallback with all configuration options."""
    from tupl.agent import AgentCallback

    def custom_mapper(tool_name, tool_inputs):
        return "execute"

    callback = AgentCallback(
        base_url="http://localhost:8000",
        tenant_id="test-tenant",
        api_key="sk-test",
        timeout=5.0,
        capture_llm=False,
        capture_tools=True,
        capture_state=True,
        enforcement_mode="block",
        fallback_on_timeout=False,
        batch_size=20,
        batch_timeout=10.0,
        action_mapper=custom_mapper,
        sensitivity_rules={"search": "public"},
        context={"env": "test"}
    )

    assert callback.api_key == "sk-test"
    assert callback.timeout == 5.0
    assert callback.capture_llm is False
    assert callback.capture_state is True
    assert callback.enforcement_mode == "block"
    assert callback.action_mapper == custom_mapper


# ============================================================================
# Test: LLM Event Capture
# ============================================================================

def test_on_chat_model_start_creates_pending_intent(agent_callback):
    """Test on_chat_model_start creates pending IntentEvent."""
    run_id = str(uuid4())

    agent_callback.on_chat_model_start(
        serialized={"name": "gpt-4"},
        messages=[{"role": "user", "content": "Hello"}],
        run_id=run_id
    )

    # Check that pending event was created
    pending = agent_callback._retrieve_pending(run_id)
    assert pending is not None
    assert pending.action == "read"
    assert pending.actor.type == "llm"
    assert pending.resource.type == "api"


def test_on_llm_end_sends_intent_event(agent_callback, mock_tupl_client):
    """Test on_llm_end sends the completed IntentEvent."""
    run_id = str(uuid4())

    # Start LLM call
    agent_callback.on_chat_model_start(
        serialized={"name": "gpt-4"},
        messages=[{"role": "user", "content": "Hello"}],
        run_id=run_id
    )

    # End LLM call
    response = Mock()
    response.generations = [[Mock(text="Hi there!")]]

    agent_callback.on_llm_end(response=response, run_id=run_id)

    # Verify client was called
    assert mock_tupl_client.capture.called
    captured_event = mock_tupl_client.capture.call_args[0][0]
    assert isinstance(captured_event, IntentEvent)
    assert captured_event.action == "read"
    assert captured_event.actor.type == "llm"


def test_llm_capture_disabled(mock_tupl_client):
    """Test LLM events not captured when capture_llm=False."""
    from tupl.agent import AgentCallback

    callback = AgentCallback(
        base_url="http://test:8000",
        tenant_id="test-tenant",
        capture_llm=False
    )
    callback.client = mock_tupl_client

    run_id = str(uuid4())
    callback.on_chat_model_start(
        serialized={"name": "gpt-4"},
        messages=[{"role": "user", "content": "Hello"}],
        run_id=run_id
    )

    # Should not create pending event
    pending = callback._retrieve_pending(run_id)
    assert pending is None


# ============================================================================
# Test: Tool Event Capture
# ============================================================================

def test_on_tool_start_creates_pending_intent(agent_callback):
    """Test on_tool_start creates pending IntentEvent with inferred action."""
    run_id = str(uuid4())

    agent_callback.on_tool_start(
        serialized={"name": "search_database"},
        input_str="query: john@example.com",
        run_id=run_id
    )

    # Check that pending event was created
    pending = agent_callback._retrieve_pending(run_id)
    assert pending is not None
    assert pending.action == "read"  # Inferred from "search"
    assert pending.actor.type == "agent"
    assert pending.resource.type == "database"  # Inferred from "database"


def test_on_tool_end_sends_intent_event(agent_callback, mock_tupl_client):
    """Test on_tool_end sends the completed IntentEvent."""
    run_id = str(uuid4())

    # Start tool call
    agent_callback.on_tool_start(
        serialized={"name": "delete_record"},
        input_str="id: 123",
        run_id=run_id
    )

    # End tool call
    agent_callback.on_tool_end(output="Deleted record 123", run_id=run_id)

    # Verify client was called
    assert mock_tupl_client.capture.called
    captured_event = mock_tupl_client.capture.call_args[0][0]
    assert isinstance(captured_event, IntentEvent)
    assert captured_event.action == "delete"  # Inferred from tool name
    assert captured_event.actor.type == "agent"


def test_tool_action_inference():
    """Test tool name to action mapping."""
    from tupl.agent import AgentCallback

    callback = AgentCallback(
        base_url="http://test:8000",
        tenant_id="test-tenant"
    )

    # Test various tool names
    assert callback._infer_action("search_database", {}) == "read"
    assert callback._infer_action("get_user", {}) == "read"
    assert callback._infer_action("delete_record", {}) == "delete"
    assert callback._infer_action("update_user", {}) == "write"
    assert callback._infer_action("create_item", {}) == "write"
    assert callback._infer_action("export_data", {}) == "export"
    assert callback._infer_action("execute_query", {}) == "execute"
    assert callback._infer_action("unknown_tool", {}) == "execute"  # fallback


def test_tool_resource_type_inference():
    """Test tool name to resource type mapping."""
    from tupl.agent import AgentCallback

    callback = AgentCallback(
        base_url="http://test:8000",
        tenant_id="test-tenant"
    )

    # Test various tool names
    assert callback._infer_resource_type("search_database") == "database"
    assert callback._infer_resource_type("query_db") == "database"
    assert callback._infer_resource_type("read_file") == "file"
    assert callback._infer_resource_type("write_document") == "file"
    assert callback._infer_resource_type("call_api") == "api"
    assert callback._infer_resource_type("unknown_tool") == "api"  # fallback


def test_custom_action_mapper():
    """Test custom action mapper override."""
    from tupl.agent import AgentCallback

    def my_mapper(tool_name, tool_inputs):
        if "dangerous" in tool_name:
            return "delete"
        return "read"

    callback = AgentCallback(
        base_url="http://test:8000",
        tenant_id="test-tenant",
        action_mapper=my_mapper
    )

    assert callback._infer_action("dangerous_operation", {}) == "delete"
    assert callback._infer_action("safe_operation", {}) == "read"


# ============================================================================
# Test: Enforcement Modes
# ============================================================================

def test_enforcement_mode_block_raises_exception(mock_tupl_client):
    """Test block mode raises AgentSecurityException on BLOCK decision."""
    from tupl.agent import AgentCallback, AgentSecurityException

    # Mock client to return BLOCK decision
    mock_tupl_client.capture.return_value = ComparisonResult(
        decision=0,  # BLOCK
        slice_similarities=[0.5, 0.5, 0.5, 0.5]
    )

    callback = AgentCallback(
        base_url="http://test:8000",
        tenant_id="test-tenant",
        enforcement_mode="block"
    )
    callback.client = mock_tupl_client

    run_id = str(uuid4())
    callback.on_tool_start(
        serialized={"name": "delete_database"},
        input_str="",
        run_id=run_id
    )

    # Should raise exception on BLOCK
    with pytest.raises(AgentSecurityException) as exc_info:
        callback.on_tool_end(output="Done", run_id=run_id)

    assert "blocked" in str(exc_info.value).lower()


def test_enforcement_mode_warn_allows_execution(mock_tupl_client):
    """Test warn mode logs warning but allows execution."""
    from tupl.agent import AgentCallback

    # Mock client to return BLOCK decision
    mock_tupl_client.capture.return_value = ComparisonResult(
        decision=0,  # BLOCK
        slice_similarities=[0.5, 0.5, 0.5, 0.5]
    )

    callback = AgentCallback(
        base_url="http://test:8000",
        tenant_id="test-tenant",
        enforcement_mode="warn"  # default
    )
    callback.client = mock_tupl_client

    run_id = str(uuid4())
    callback.on_tool_start(
        serialized={"name": "delete_database"},
        input_str="",
        run_id=run_id
    )

    # Should NOT raise exception in warn mode
    callback.on_tool_end(output="Done", run_id=run_id)
    # Execution continues (no exception)


def test_enforcement_mode_log_silent(mock_tupl_client):
    """Test log mode only logs, no warnings."""
    from tupl.agent import AgentCallback

    # Mock client to return BLOCK decision
    mock_tupl_client.capture.return_value = ComparisonResult(
        decision=0,  # BLOCK
        slice_similarities=[0.5, 0.5, 0.5, 0.5]
    )

    callback = AgentCallback(
        base_url="http://test:8000",
        tenant_id="test-tenant",
        enforcement_mode="log"
    )
    callback.client = mock_tupl_client

    run_id = str(uuid4())
    callback.on_tool_start(
        serialized={"name": "delete_database"},
        input_str="",
        run_id=run_id
    )

    # Should NOT raise exception in log mode
    callback.on_tool_end(output="Done", run_id=run_id)
    # Silent execution


# ============================================================================
# Test: Error Handling
# ============================================================================

def test_network_failure_graceful_degradation():
    """Test network failures don't break execution."""
    from tupl.agent import AgentCallback
    import httpx

    callback = AgentCallback(
        base_url="http://unreachable:9999",
        tenant_id="test-tenant",
        timeout=0.1,  # Very short timeout
        fallback_on_timeout=True
    )

    run_id = str(uuid4())
    callback.on_tool_start(
        serialized={"name": "test_tool"},
        input_str="",
        run_id=run_id
    )

    # Should handle network error gracefully (no exception)
    callback.on_tool_end(output="Done", run_id=run_id)


def test_on_llm_error_cleanup(agent_callback):
    """Test on_llm_error cleans up pending events."""
    run_id = str(uuid4())

    # Start LLM call
    agent_callback.on_chat_model_start(
        serialized={"name": "gpt-4"},
        messages=[{"role": "user", "content": "Hello"}],
        run_id=run_id
    )

    # Simulate error
    error = Exception("LLM timeout")
    agent_callback.on_llm_error(error=error, run_id=run_id)

    # Pending event should be cleaned up
    pending = agent_callback._retrieve_pending(run_id)
    assert pending is None


def test_on_tool_error_cleanup(agent_callback):
    """Test on_tool_error cleans up pending events."""
    run_id = str(uuid4())

    # Start tool call
    agent_callback.on_tool_start(
        serialized={"name": "test_tool"},
        input_str="",
        run_id=run_id
    )

    # Simulate error
    error = Exception("Tool failed")
    agent_callback.on_tool_error(error=error, run_id=run_id)

    # Pending event should be cleaned up
    pending = agent_callback._retrieve_pending(run_id)
    assert pending is None


# ============================================================================
# Test: Thread Safety
# ============================================================================

def test_thread_local_storage():
    """Test thread-local storage for pending events."""
    from tupl.agent import AgentCallback

    callback = AgentCallback(
        base_url="http://test:8000",
        tenant_id="test-tenant"
    )

    # Create events with different run IDs
    run_id_1 = str(uuid4())
    run_id_2 = str(uuid4())

    callback.on_tool_start(
        serialized={"name": "tool1"},
        input_str="",
        run_id=run_id_1
    )

    callback.on_tool_start(
        serialized={"name": "tool2"},
        input_str="",
        run_id=run_id_2
    )

    # Both events should be stored separately
    pending_1 = callback._retrieve_pending(run_id_1)
    pending_2 = callback._retrieve_pending(run_id_2)

    assert pending_1 is not None
    assert pending_2 is not None
    # First retrieve should remove from dict
    assert callback._retrieve_pending(run_id_1) is None


# ============================================================================
# Test: Sensitivity Rules
# ============================================================================

def test_sensitivity_rules_applied():
    """Test custom sensitivity rules are applied."""
    from tupl.agent import AgentCallback

    callback = AgentCallback(
        base_url="http://test:8000",
        tenant_id="test-tenant",
        sensitivity_rules={
            "search_public_api": "public",
            "query_user_data": "internal"
        }
    )

    # Test that sensitivity is applied correctly
    assert callback._get_sensitivity("search_public_api") == ["public"]
    assert callback._get_sensitivity("query_user_data") == ["internal"]
    assert callback._get_sensitivity("unknown_tool") == ["internal"]  # default


# ============================================================================
# Test: IntentEvent Schema V1.2
# ============================================================================

def test_intent_event_schema_v1_2(agent_callback, mock_tupl_client):
    """Test generated IntentEvents use v1.2 schema."""
    run_id = str(uuid4())

    # Capture LLM call
    agent_callback.on_chat_model_start(
        serialized={"name": "gpt-4"},
        messages=[{"role": "user", "content": "Hello"}],
        run_id=run_id
    )

    response = Mock()
    response.generations = [[Mock(text="Hi!")]]
    agent_callback.on_llm_end(response=response, run_id=run_id)

    # Check captured event
    captured_event = mock_tupl_client.capture.call_args[0][0]
    assert captured_event.schemaVersion == "v1.2"
    assert captured_event.actor.type == "llm"  # v1.2 actor type
    assert isinstance(captured_event.data.sensitivity, list)  # v1.1 field
    assert captured_event.risk.authn in ["required", "not_required"]  # v1.1 field


# ============================================================================
# Test: AgentSecurityException
# ============================================================================

def test_agent_security_exception_attributes():
    """Test AgentSecurityException has required attributes."""
    from tupl.agent import AgentSecurityException

    exc = AgentSecurityException(
        intent_id="intent-123",
        boundary_id="boundary-456",
        decision_metadata={"reason": "test"}
    )

    assert exc.intent_id == "intent-123"
    assert exc.boundary_id == "boundary-456"
    assert exc.metadata == {"reason": "test"}
    assert "intent-123" in str(exc)
    assert "boundary-456" in str(exc)
