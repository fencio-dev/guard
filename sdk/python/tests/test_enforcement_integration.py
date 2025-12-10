"""
Integration tests for SecureGraphProxy and enforcement_agent.

Tests the v1.3 enforcement pattern with streaming interception.
"""

import pytest
from unittest.mock import Mock, MagicMock, patch
from tupl.agent import SecureGraphProxy, enforcement_agent
from tupl.types import IntentEvent, ComparisonResult, Actor, Resource, Data, Risk
from tupl.client import TuplClient

# Try importing langchain_core for real AIMessage, fall back to mock
try:
    from langchain_core.messages import AIMessage
    HAS_LANGCHAIN = True
except ImportError:
    HAS_LANGCHAIN = False
    AIMessage = None


# ============================================================================
# Test Fixtures
# ============================================================================

@pytest.fixture
def mock_graph():
    """Create a mock compiled LangGraph graph."""
    graph = Mock()
    graph.stream = Mock()
    graph.astream = Mock()
    return graph


@pytest.fixture
def mock_client():
    """Create a mock TuplClient."""
    client = Mock(spec=TuplClient)
    return client


@pytest.fixture
def allow_result():
    """ComparisonResult for ALLOW decision."""
    return ComparisonResult(
        decision=1,
        slice_similarities=[0.9, 0.85, 0.88, 0.92],
        boundaries_evaluated=1,
        timestamp=1699564800.0
    )


@pytest.fixture
def block_result():
    """ComparisonResult for BLOCK decision."""
    return ComparisonResult(
        decision=0,
        slice_similarities=[0.5, 0.4, 0.3, 0.6],
        boundaries_evaluated=1,
        timestamp=1699564800.0
    )


# ============================================================================
# Helper Functions
# ============================================================================

def create_tool_call_message(tool_name: str):
    """Create a mock AIMessage with tool_calls."""
    message = Mock()
    message.tool_calls = [
        {
            "name": tool_name,
            "args": {"param": "value"},
            "id": "call_123"
        }
    ]
    return message


def create_state_with_tool_call(tool_name: str):
    """Create a mock state dict with AIMessage containing tool call."""
    if HAS_LANGCHAIN:
        message = AIMessage(
            content="I'll use the tool",
            tool_calls=[
                {
                    "name": tool_name,
                    "args": {"param": "value"},
                    "id": "call_123"
                }
            ]
        )
    else:
        # Mock AIMessage
        message = Mock()
        message.tool_calls = [
            {
                "name": tool_name,
                "args": {"param": "value"},
                "id": "call_123"
            }
        ]

    return {"messages": [message]}


# ============================================================================
# Test: SecureGraphProxy Initialization
# ============================================================================

def test_secure_graph_proxy_init_with_client(mock_graph, mock_client):
    """Test SecureGraphProxy initialization with provided client."""
    proxy = SecureGraphProxy(
        graph=mock_graph,
        boundary_id="test-boundary",
        client=mock_client,
    )

    assert proxy._graph == mock_graph
    assert proxy.boundary_id == "test-boundary"
    assert proxy.client == mock_client


def test_secure_graph_proxy_init_creates_client(mock_graph):
    """Test SecureGraphProxy initialization creates client if not provided."""
    proxy = SecureGraphProxy(
        graph=mock_graph,
        boundary_id="test-boundary",
        base_url="http://test:8000",
        timeout=5.0,
    )

    assert proxy._graph == mock_graph
    assert proxy.boundary_id == "test-boundary"
    assert isinstance(proxy.client, TuplClient)


# ============================================================================
# Test: Intent Creation from Tool Calls
# ============================================================================

def test_create_intent_from_tool_call_delete():
    """Test intent creation for DELETE tool."""
    proxy = SecureGraphProxy(
        graph=Mock(),
        boundary_id="test",
        client=Mock(spec=TuplClient),
    )

    tool_call = {
        "name": "delete_record",
        "args": {"record_id": "123"},
        "id": "call_123"
    }

    event = proxy._create_intent_from_tool_call(tool_call)

    assert event.action == "delete"
    assert event.resource.name == "delete_record"
    assert event.data.pii is True  # DELETE implies PII
    assert event.actor.type == "agent"


def test_create_intent_from_tool_call_read():
    """Test intent creation for READ tool."""
    proxy = SecureGraphProxy(
        graph=Mock(),
        boundary_id="test",
        client=Mock(spec=TuplClient),
    )

    tool_call = {
        "name": "search_database",
        "args": {"query": "john"},
        "id": "call_456"
    }

    event = proxy._create_intent_from_tool_call(tool_call)

    assert event.action == "read"
    assert event.resource.type == "database"  # Inferred from tool name
    assert event.data.pii is False  # READ doesn't imply PII
    assert event.data.sensitivity == ["public"]


def test_create_intent_from_tool_call_export():
    """Test intent creation for EXPORT tool."""
    proxy = SecureGraphProxy(
        graph=Mock(),
        boundary_id="test",
        client=Mock(spec=TuplClient),
    )

    tool_call = {
        "name": "export_data",
        "args": {"format": "csv"},
        "id": "call_789"
    }

    event = proxy._create_intent_from_tool_call(tool_call)

    assert event.action == "export"
    assert event.data.pii is True  # EXPORT implies PII
    assert event.data.sensitivity == ["internal"]


# ============================================================================
# Test: Tool Call Enforcement
# ============================================================================

def test_enforce_tool_calls_allow(mock_client, allow_result):
    """Test enforcement allows tool call when policy passes."""
    mock_client.capture = Mock(return_value=allow_result)

    proxy = SecureGraphProxy(
        graph=Mock(),
        boundary_id="test",
        client=mock_client,
    )

    state = create_state_with_tool_call("search_database")

    # Should not raise exception
    proxy._enforce_tool_calls(state)

    # Should call client.capture once
    assert mock_client.capture.call_count == 1


def test_enforce_tool_calls_block(mock_client, block_result):
    """Test enforcement blocks tool call when policy fails."""
    mock_client.capture = Mock(return_value=block_result)

    proxy = SecureGraphProxy(
        graph=Mock(),
        boundary_id="test",
        client=mock_client,
    )

    state = create_state_with_tool_call("delete_record")

    # Should raise PermissionError
    with pytest.raises(PermissionError) as exc_info:
        proxy._enforce_tool_calls(state)

    assert "delete_record" in str(exc_info.value)
    assert "test" in str(exc_info.value)  # boundary_id in error message


def test_enforce_tool_calls_network_error(mock_client):
    """Test enforcement fails open on network error."""
    mock_client.capture = Mock(return_value=None)  # Network error returns None

    proxy = SecureGraphProxy(
        graph=Mock(),
        boundary_id="test",
        client=mock_client,
    )

    state = create_state_with_tool_call("search_database")

    # Should not raise exception (fail-open)
    proxy._enforce_tool_calls(state)


def test_enforce_tool_calls_multiple_tools(mock_client, allow_result, block_result):
    """Test enforcement with multiple tool calls - blocks on first failure."""
    # First tool ALLOW, second tool BLOCK
    mock_client.capture = Mock(side_effect=[allow_result, block_result])

    proxy = SecureGraphProxy(
        graph=Mock(),
        boundary_id="test",
        client=mock_client,
    )

    if HAS_LANGCHAIN:
        message = AIMessage(
            content="I'll use multiple tools",
            tool_calls=[
                {"name": "search_database", "args": {}, "id": "call_1"},
                {"name": "delete_record", "args": {}, "id": "call_2"},
            ]
        )
    else:
        # Mock AIMessage
        message = Mock()
        message.tool_calls = [
            {"name": "search_database", "args": {}, "id": "call_1"},
            {"name": "delete_record", "args": {}, "id": "call_2"},
        ]

    state = {"messages": [message]}

    # Should raise on second tool call
    with pytest.raises(PermissionError) as exc_info:
        proxy._enforce_tool_calls(state)

    assert "delete_record" in str(exc_info.value)

    # Should have called capture twice
    assert mock_client.capture.call_count == 2


# ============================================================================
# Test: Invoke Method
# ============================================================================

def test_invoke_with_enforcement(mock_graph, mock_client, allow_result):
    """Test invoke method with enforcement."""
    mock_client.capture = Mock(return_value=allow_result)

    # Create mock messages without tool_calls
    msg1 = Mock(content="Thinking...")
    msg1.tool_calls = []
    msg3 = Mock(content="Done")
    msg3.tool_calls = []

    # Mock graph.stream to return states with tool call
    state1 = {"messages": [msg1]}
    state2 = create_state_with_tool_call("search_database")
    state3 = {"messages": [msg3]}

    mock_graph.stream = Mock(return_value=[state1, state2, state3])

    proxy = SecureGraphProxy(
        graph=mock_graph,
        boundary_id="test",
        client=mock_client,
    )

    result = proxy.invoke({"messages": []})

    # Should return final state
    assert result == state3

    # Should have called graph.stream
    mock_graph.stream.assert_called_once()

    # Should have called capture for tool call
    assert mock_client.capture.call_count == 1


def test_invoke_blocks_on_policy_violation(mock_graph, mock_client, block_result):
    """Test invoke raises PermissionError when policy blocks."""
    mock_client.capture = Mock(return_value=block_result)

    # Mock graph.stream to return state with tool call
    state_with_tool = create_state_with_tool_call("delete_record")
    mock_graph.stream = Mock(return_value=[state_with_tool])

    proxy = SecureGraphProxy(
        graph=mock_graph,
        boundary_id="test",
        client=mock_client,
    )

    # Should raise PermissionError
    with pytest.raises(PermissionError):
        proxy.invoke({"messages": []})


# ============================================================================
# Test: Stream Method
# ============================================================================

def test_stream_with_enforcement(mock_graph, mock_client, allow_result):
    """Test stream method with enforcement."""
    mock_client.capture = Mock(return_value=allow_result)

    # Create mock message without tool_calls
    msg1 = Mock(content="Thinking...")
    msg1.tool_calls = []

    # Mock graph.stream to return states
    state1 = {"messages": [msg1]}
    state2 = create_state_with_tool_call("search_database")

    mock_graph.stream = Mock(return_value=[state1, state2])

    proxy = SecureGraphProxy(
        graph=mock_graph,
        boundary_id="test",
        client=mock_client,
    )

    states = list(proxy.stream({"messages": []}))

    # Should yield all states
    assert len(states) == 2
    assert states[0] == state1
    assert states[1] == state2


# ============================================================================
# Test: Transparent Proxy (__getattr__)
# ============================================================================

def test_proxy_forwards_attributes(mock_graph):
    """Test that proxy forwards unknown attributes to graph."""
    mock_graph.custom_method = Mock(return_value="custom_result")
    mock_graph.custom_attribute = "custom_value"

    proxy = SecureGraphProxy(
        graph=mock_graph,
        boundary_id="test",
        client=Mock(spec=TuplClient),
    )

    # Should forward method calls
    assert proxy.custom_method() == "custom_result"
    mock_graph.custom_method.assert_called_once()

    # Should forward attribute access
    assert proxy.custom_attribute == "custom_value"


# ============================================================================
# Test: enforcement_agent Factory Function
# ============================================================================

def test_enforcement_agent_factory(mock_graph):
    """Test enforcement_agent factory function."""
    secure_agent = enforcement_agent(
        graph=mock_graph,
        boundary_id="test-boundary",
        base_url="http://test:8000",
        timeout=5.0,
    )

    assert isinstance(secure_agent, SecureGraphProxy)
    assert secure_agent._graph == mock_graph
    assert secure_agent.boundary_id == "test-boundary"


# ============================================================================
# Test: Custom Mappers
# ============================================================================

def test_custom_action_mapper():
    """Test custom action mapper."""
    def custom_mapper(tool_name: str, tool_inputs: dict) -> str:
        if "sensitive" in tool_inputs:
            return "delete"
        return "read"

    proxy = SecureGraphProxy(
        graph=Mock(),
        boundary_id="test",
        client=Mock(spec=TuplClient),
        action_mapper=custom_mapper,
    )

    # Test custom mapping
    tool_call = {
        "name": "custom_tool",
        "args": {"sensitive": True},
        "id": "call_123"
    }

    event = proxy._create_intent_from_tool_call(tool_call)
    assert event.action == "delete"

    # Test default fallback
    tool_call2 = {
        "name": "custom_tool",
        "args": {},
        "id": "call_456"
    }

    event2 = proxy._create_intent_from_tool_call(tool_call2)
    assert event2.action == "read"


def test_custom_resource_type_mapper():
    """Test custom resource type mapper."""
    def custom_mapper(tool_name: str) -> str:
        if "cloud" in tool_name:
            return "api"
        return "database"

    proxy = SecureGraphProxy(
        graph=Mock(),
        boundary_id="test",
        client=Mock(spec=TuplClient),
        resource_type_mapper=custom_mapper,
    )

    # Test custom mapping
    tool_call = {
        "name": "cloud_storage_tool",
        "args": {},
        "id": "call_123"
    }

    event = proxy._create_intent_from_tool_call(tool_call)
    assert event.resource.type == "api"


# ============================================================================
# Test: Violation Callback
# ============================================================================

def test_violation_callback(mock_client, block_result):
    """Test violation callback is called on block."""
    violation_called = []

    def on_violation(event: IntentEvent, result: ComparisonResult):
        violation_called.append((event, result))

    mock_client.capture = Mock(return_value=block_result)

    proxy = SecureGraphProxy(
        graph=Mock(),
        boundary_id="test",
        client=mock_client,
        on_violation=on_violation,
    )

    state = create_state_with_tool_call("delete_record")

    # Should raise PermissionError
    with pytest.raises(PermissionError):
        proxy._enforce_tool_calls(state)

    # Callback should have been called
    assert len(violation_called) == 1
    event, result = violation_called[0]
    assert event.action == "delete"
    assert result.decision == 0
