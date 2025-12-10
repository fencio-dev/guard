# SDK v1.2 Enhancement Plan: LangGraph Auto-Instrumentation

**Date**: 2025-11-13
**Status**: Design Complete - Ready for Implementation
**Goal**: Enable 5-line integration for automatic intent capture from LangGraph agents

---

## Executive Summary

This plan describes the v1.2 enhancement to the Tupl Python SDK, introducing automatic intent capture for LangGraph agents through a callback-based instrumentation pattern. The enhancement reduces integration effort from "manual logging at every operation" to "pass callback to graph.invoke()" - approximately **95% reduction in integration code**.

**Key Innovation**: Leverage LangGraph's native callback system to automatically capture LLM calls, tool invocations, and state transitions without requiring developers to manually instrument their code.

**Developer Experience**:
```python
from tupl.agent import AgentCallback

tupl = AgentCallback(base_url="http://localhost:8000", tenant_id="my-tenant")
result = graph.invoke(state, config={"callbacks": [tupl]})
```

---

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [Component Design](#2-component-design)
3. [IntentEvent Mapping Strategy](#3-intentevent-mapping-strategy)
4. [Schema Changes (v1.2)](#4-schema-changes-v12)
5. [Decision Enforcement](#5-decision-enforcement)
6. [Configuration API](#6-configuration-api)
7. [Testing Strategy](#7-testing-strategy)
8. [Demo Application](#8-demo-application)
9. [Implementation Tasks](#9-implementation-tasks)

---

## 1. Architecture Overview

### 1.1 Design Decision: Callback Handler Pattern

After evaluating three approaches (callback handler, context manager, graph wrapper), we selected the **callback handler pattern** as it is:
- Most idiomatic for the LangGraph ecosystem
- Composable with other callbacks developers may use
- Provides access to all event types (LLM, tool, state transitions)
- Requires minimal code changes (3 lines)

### 1.2 Component Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    LangGraph Application                     │
│  ┌─────────────┐         ┌──────────────┐                  │
│  │   Agent     │────────▶│   LLM Call   │                  │
│  │   Graph     │         └──────────────┘                  │
│  └─────────────┘                                            │
│         │                                                    │
│         │ graph.invoke(state, config={"callbacks": [...]})  │
│         ▼                                                    │
│  ┌─────────────────────────────────────────┐               │
│  │        AgentCallback (New in v1.2)       │               │
│  │  - on_chat_model_start / on_llm_start    │               │
│  │  - on_tool_start / on_tool_end           │               │
│  │  - on_chain_start / on_chain_end         │               │
│  └─────────────────────────────────────────┘               │
│         │ IntentEvents                                       │
└─────────┼────────────────────────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────────────────────────────────┐
│               Tupl Python SDK (v1.0 + v1.2)                  │
│  ┌────────────────┐         ┌──────────────────┐           │
│  │  TuplClient    │◀────────│  EventBuffer     │           │
│  │  (Existing)    │         │  (Existing)      │           │
│  └────────────────┘         └──────────────────┘           │
└─────────────────────────────────────────────────────────────┘
          │ POST /api/v1/intents/compare
          ▼
┌─────────────────────────────────────────────────────────────┐
│                    Management Plane                          │
│  Encoding → Rust Comparison → Decision                      │
└─────────────────────────────────────────────────────────────┘
```

### 1.3 Event Flow

1. **LangGraph executes** → Triggers callback methods
2. **AgentCallback captures** → Builds IntentEvent from callback data
3. **SDK sends to Management Plane** → Uses existing TuplClient/EventBuffer
4. **Decision returned** → Enforced based on `enforcement_mode`
5. **Graph continues or halts** → Depending on decision and mode

### 1.4 Key Design Decisions

- **Stateless callback instances**: Safe to reuse across invocations
- **Async-compatible**: Implements both sync and async callback methods
- **Graceful degradation**: Network failures log warnings but never block execution by default
- **Buffering support**: Optional EventBuffer integration for batching
- **Thread-safe**: Uses thread-local storage for tracking nested calls

---

## 2. Component Design

### 2.1 AgentCallback Class

**Location**: `tupl_sdk/python/tupl/agent.py` (new file)

**Inheritance**: `langchain_core.callbacks.BaseCallbackHandler`

**Responsibilities**:
- Intercept LangGraph callback events
- Map events to IntentEvent schema
- Send IntentEvents to Management Plane
- Enforce security decisions based on configuration

### 2.2 Callback Method Implementation

#### LLM Event Capture

```python
def on_chat_model_start(self, serialized, messages, run_id, parent_run_id=None, **kwargs):
    """
    Capture LLM call initiation.

    Maps to IntentEvent:
    - action: "read" (LLMs read data to generate responses)
    - actor_type: "llm" (NEW in v1.2)
    - resource_type: "api"
    """
    # Extract: model name, prompt messages, run metadata
    # Build IntentEvent with action="read"
    # Store in thread-local pending events dict keyed by run_id

def on_llm_end(self, response, run_id, **kwargs):
    """
    Capture LLM response completion.

    Enriches the IntentEvent created in on_chat_model_start:
    - Add token usage metadata
    - Add completion text
    - Send to Management Plane (immediate or buffered)
    """
    # Retrieve pending IntentEvent by run_id
    # Enrich with response metadata
    # Send via TuplClient or EventBuffer
    # Clean up thread-local storage
```

#### Tool Event Capture

```python
def on_tool_start(self, serialized, input_str, run_id, parent_run_id=None, **kwargs):
    """
    Capture tool invocation.

    Maps to IntentEvent:
    - action: Inferred from tool name (read/write/delete/execute)
    - actor_type: "agent" (NEW in v1.2)
    - resource_type: Inferred from tool name (database/file/api)
    """
    tool_name = serialized.get("name")

    # Infer action type from tool name using action_mapper
    action = self._infer_action(tool_name, input_str)

    # Infer resource type from tool name
    resource_type = self._infer_resource_type(tool_name)

    # Build IntentEvent
    # Store in thread-local pending events

def on_tool_end(self, output, run_id, **kwargs):
    """
    Capture tool completion.

    Enriches the IntentEvent created in on_tool_start:
    - Add tool result metadata
    - Add execution time
    - Send to Management Plane
    """
    # Retrieve, enrich, send, cleanup
```

#### State Transition Capture

```python
def on_chain_start(self, serialized, inputs, run_id, parent_run_id=None, **kwargs):
    """
    Capture node/chain execution (state transitions).

    Maps to IntentEvent:
    - action: "execute" (state transitions are execution events)
    - actor_type: "agent"
    - resource_type: "api" (internal state machine)

    NOTE: This is OFF by default (capture_state=False) to reduce noise.
    """
    # Only process if capture_state=True
    # Extract: node name, input state
    # Build IntentEvent for state transition tracking
```

### 2.3 Thread Safety and Run Correlation

```python
import threading

class AgentCallback(BaseCallbackHandler):
    def __init__(self, ...):
        self._pending_events = threading.local()
        # Each thread gets its own pending events dict

    def _get_pending_events(self) -> dict:
        """Get thread-local pending events dict."""
        if not hasattr(self._pending_events, 'events'):
            self._pending_events.events = {}
        return self._pending_events.events

    def _store_pending(self, run_id: str, event: IntentEvent):
        """Store pending event keyed by run_id."""
        self._get_pending_events()[run_id] = event

    def _retrieve_pending(self, run_id: str) -> Optional[IntentEvent]:
        """Retrieve and remove pending event."""
        return self._get_pending_events().pop(run_id, None)
```

### 2.4 Error Handling in Callbacks

```python
def on_llm_error(self, error, run_id, **kwargs):
    """Capture LLM call failures."""
    # Retrieve pending event
    # Mark as failed
    # Send telemetry (don't block on error)
    # Clean up

def on_tool_error(self, error, run_id, **kwargs):
    """Capture tool execution failures."""
    # Similar to on_llm_error
```

---

## 3. IntentEvent Mapping Strategy

### 3.1 LLM Calls → IntentEvent

**Rationale**: LLM calls are READ operations on external API resources. The LLM reads data (prompts, context) to generate responses.

```python
IntentEvent(
    id=f"intent_{run_id}",
    schemaVersion="v1.2",
    tenantId=self.tenant_id,
    timestamp=datetime.utcnow().timestamp(),
    actor=Actor(
        id=model_name,  # e.g., "gpt-4", "claude-sonnet-4"
        type="llm"  # NEW in v1.2
    ),
    action="read",
    resource=Resource(
        type="api",
        name=f"llm://{provider}/{model}",
        location="cloud"
    ),
    data=Data(
        sensitivity=["internal"],  # Prompt data is internal
        pii=None,
        volume="single"
    ),
    risk=Risk(
        authn="required"  # API key required
    )
)
```

### 3.2 Tool Calls → IntentEvent

**Rationale**: Tools perform concrete operations (read/write/delete/execute) on resources (databases, files, APIs). Action and resource types are inferred from tool name and inputs.

**Default Tool Action Mapping**:
```python
TOOL_ACTION_MAP = {
    # READ operations
    "search": "read",
    "get": "read",
    "fetch": "read",
    "list": "read",
    "query": "read",

    # WRITE operations
    "create": "write",
    "update": "write",
    "insert": "write",
    "write": "write",

    # DELETE operations
    "delete": "delete",
    "remove": "delete",
    "drop": "delete",

    # EXPORT operations
    "export": "export",
    "download": "export",

    # EXECUTE operations
    "execute": "execute",
    "run": "execute",
    "call": "execute",
}

def infer_action_from_tool(tool_name: str, tool_inputs: dict) -> str:
    """Infer action type from tool name using keyword matching."""
    tool_lower = tool_name.lower()

    # Check exact matches first
    for keyword, action in TOOL_ACTION_MAP.items():
        if keyword in tool_lower:
            return action

    # Fallback: inspect inputs
    if any(key in tool_inputs for key in ["query", "search", "filter"]):
        return "read"

    # Conservative default
    return "execute"
```

**Resource Type Inference**:
```python
def infer_resource_type(tool_name: str) -> str:
    """Infer resource type from tool name."""
    tool_lower = tool_name.lower()

    if any(kw in tool_lower for kw in ["database", "db", "sql", "query"]):
        return "database"
    elif any(kw in tool_lower for kw in ["file", "document", "pdf", "csv"]):
        return "file"
    else:
        return "api"  # Default fallback
```

**Example Tool IntentEvent**:
```python
# Tool: delete_customer_record(customer_id="123")
IntentEvent(
    id=f"intent_{run_id}",
    schemaVersion="v1.2",
    tenantId=self.tenant_id,
    timestamp=datetime.utcnow().timestamp(),
    actor=Actor(
        id=parent_run_id,  # Link to parent agent execution
        type="agent"  # NEW in v1.2
    ),
    action="delete",  # Inferred from tool name
    resource=Resource(
        type="database",  # Inferred from tool name
        name="delete_customer_record",
        location="cloud"
    ),
    data=Data(
        sensitivity=["internal"],  # Default or from sensitivity_rules
        pii=True,  # Can be inferred from tool metadata
        volume="single"
    ),
    risk=Risk(
        authn="required"
    )
)
```

### 3.3 State Transitions → IntentEvent

**Rationale**: State transitions represent internal workflow execution. Less security-critical than LLM/tool calls, so OFF by default.

```python
# Only captured if capture_state=True
IntentEvent(
    id=f"intent_{run_id}",
    schemaVersion="v1.2",
    tenantId=self.tenant_id,
    timestamp=datetime.utcnow().timestamp(),
    actor=Actor(
        id=graph_name,
        type="agent"
    ),
    action="execute",
    resource=Resource(
        type="api",
        name=f"node://{node_name}",
        location="local"
    ),
    data=Data(
        sensitivity=["internal"],
        pii=False,
        volume="single"
    ),
    risk=Risk(
        authn="not_required"  # Internal transitions
    )
)
```

### 3.4 Customization API

Developers can override default mapping logic:

```python
def custom_action_mapper(tool_name: str, tool_inputs: dict) -> str:
    """Custom action inference logic."""
    if "customer" in tool_name and "delete" in tool_name:
        return "delete"
    # ... custom logic
    return "execute"

tupl = AgentCallback(
    base_url="http://localhost:8000",
    tenant_id="my-tenant",
    action_mapper=custom_action_mapper,  # Override default
    sensitivity_rules={
        "search_public_api": "public",
        "query_user_data": "internal"
    }
)
```

---

## 4. Schema Changes (v1.2)

### 4.1 New Actor Types

**Current (v1.1)**:
```python
actor.type: Literal["user", "service"]
```

**Updated (v1.2)**:
```python
actor.type: Literal["user", "service", "llm", "agent"]
```

**Additions**:
- `"llm"`: Large language model (GPT-4, Claude, etc.)
- `"agent"`: AI agent or autonomous system

**Rationale**: LangGraph agents operate autonomously and make decisions via LLMs. Distinguishing between human users, backend services, LLMs, and agents enables more granular policy enforcement.

**Example Policies Enabled**:
- "Block delete operations initiated by agents"
- "Require human approval for agent-initiated bulk exports"
- "Allow LLM read access but not write access"

### 4.2 Action Type Already Exists

The `"execute"` action type already exists in v1.1 schema, so no changes needed for actions.

```python
action: Literal["read", "write", "delete", "export", "execute", "update"]
```

### 4.3 Schema Version

**IntentEvent**:
```python
schemaVersion: Literal["v1.2"] = "v1.2"  # Updated from "v1.1"
```

**DesignBoundary**:
```python
boundarySchemaVersion: Literal["v1.2"] = "v1.2"  # Updated from "v1.1"
```

### 4.4 Backward Compatibility

**v1.1 → v1.2 Migration**:
- Existing boundaries with `actor_types: ["user", "service"]` continue to work
- New boundaries can specify `actor_types: ["user", "service", "llm", "agent"]`
- Management Plane accepts both v1.1 and v1.2 schema versions
- Encoding logic handles all actor types uniformly (text canonicalization is version-agnostic)

---

## 5. Decision Enforcement

### 5.1 Enforcement Modes

**Three enforcement strategies**:

```python
tupl = AgentCallback(
    base_url="http://localhost:8000",
    tenant_id="my-tenant",
    enforcement_mode="warn"  # Options: "block", "warn", "log"
)
```

| Mode | Behavior | Use Case |
|------|----------|----------|
| **block** | Raises `AgentSecurityException` on BLOCK decision | Production systems with mandatory policies |
| **warn** | Logs warning, allows execution to continue | Gradual rollout, monitoring before enforcement |
| **log** | Only logs intent and decision, no interruption | Development, A/B testing, telemetry collection |

### 5.2 Block Mode Implementation

```python
class AgentSecurityException(Exception):
    """Raised when an intent is blocked by Tupl policy."""

    def __init__(self, intent_id: str, boundary_id: str, decision_metadata: dict):
        self.intent_id = intent_id
        self.boundary_id = boundary_id
        self.metadata = decision_metadata
        super().__init__(
            f"Intent {intent_id} blocked by boundary {boundary_id}"
        )

# In callback handler
def _enforce_decision(self, intent: IntentEvent, result: ComparisonResult):
    if result.decision == 0:  # BLOCK
        if self.enforcement_mode == "block":
            raise AgentSecurityException(
                intent_id=intent.id,
                boundary_id=result.metadata.get("boundary_id"),
                decision_metadata=result.metadata
            )
        elif self.enforcement_mode == "warn":
            logger.warning(
                f"Intent {intent.id} blocked but allowed in warn mode",
                extra={"metadata": result.metadata}
            )
        elif self.enforcement_mode == "log":
            logger.info(
                f"Intent {intent.id} would be blocked",
                extra={"metadata": result.metadata}
            )
```

### 5.3 Developer Control

```python
from tupl.agent import AgentCallback, AgentSecurityException

tupl = AgentCallback(
    base_url="...",
    tenant_id="...",
    enforcement_mode="block"
)

try:
    result = graph.invoke(state, config={"callbacks": [tupl]})
except AgentSecurityException as e:
    # Handle security block
    logger.warning(f"Operation blocked: {e.metadata}")
    # Implement custom fallback behavior
    result = fallback_response(state)
```

### 5.4 Error Handling and Graceful Degradation

**Network Failures**:
```python
try:
    response = self.client.compare_intent(intent)
except (httpx.TimeoutException, httpx.ConnectError) as e:
    logger.warning(f"Management Plane unreachable: {e}")
    # Graceful degradation: ALLOW by default
    return ComparisonResult(decision=1, slice_similarities=[1.0, 1.0, 1.0, 1.0])
```

**Timeout Configuration**:
```python
tupl = AgentCallback(
    base_url="http://localhost:8000",
    tenant_id="my-tenant",
    timeout=2.0,  # Request timeout in seconds
    fallback_on_timeout=True  # ALLOW on timeout (default)
)
```

**Management Plane Errors**:
```python
if response.status_code >= 500:
    logger.error(f"Management Plane error: {response.status_code}")
    # Treat as temporary failure, ALLOW execution
    return ComparisonResult(decision=1, slice_similarities=[1.0, 1.0, 1.0, 1.0])
```

**Schema Validation Errors**:
```python
try:
    result = ComparisonResult.model_validate(response.json())
except ValidationError as e:
    logger.error(f"Invalid response schema: {e}")
    # Fallback to ALLOW
    return ComparisonResult(decision=1, slice_similarities=[1.0, 1.0, 1.0, 1.0])
```

---

## 6. Configuration API

### 6.1 Comprehensive Configuration

```python
from tupl.agent import AgentCallback

callback = AgentCallback(
    # Connection settings
    base_url="http://localhost:8000",
    api_key="sk-...",  # Optional API key for authentication
    timeout=2.0,  # Request timeout in seconds (default: 2.0)

    # Capture toggles
    capture_llm=True,  # Capture LLM calls (default: True)
    capture_tools=True,  # Capture tool calls (default: True)
    capture_state=False,  # Capture state transitions (default: False)

    # Enforcement
    enforcement_mode="warn",  # "block", "warn", or "log" (default: "warn")
    fallback_on_timeout=True,  # ALLOW on timeout (default: True)

    # Batching (integrates with existing EventBuffer)
    batch_size=10,  # Buffer up to N events before sending (default: 1 = immediate)
    batch_timeout=5.0,  # Send buffer after N seconds (default: 5.0)

    # Custom mapping functions
    action_mapper=custom_action_mapper,  # Override default tool→action mapping
    resource_type_mapper=custom_resource_mapper,  # Override resource inference
    sensitivity_rules={  # Tool-specific sensitivity mapping
        "search_database": "internal",
        "public_api": "public"
    },

    # Metadata enrichment
    tenant_id="tenant-123",  # Tenant identifier (required)
    context={"environment": "production", "version": "v2.0"},  # Additional context
)
```

### 6.2 Simple Use Case (Defaults)

```python
# Minimal configuration for quick start
from tupl.agent import AgentCallback

tupl = AgentCallback(
    base_url="http://localhost:8000",
    tenant_id="my-tenant"
)

result = graph.invoke(state, config={"callbacks": [tupl]})
```

**Defaults provide**:
- Warn mode (logs but doesn't block)
- Captures LLM and tool calls, not state transitions
- Immediate sending (no batching)
- 2-second timeout with fallback to ALLOW

### 6.3 Production Use Case

```python
# Production configuration with strict enforcement
tupl = AgentCallback(
    base_url="https://tupl.company.com",
    api_key=os.environ["TUPL_API_KEY"],
    tenant_id=os.environ["TENANT_ID"],
    enforcement_mode="block",  # Strict enforcement
    batch_size=20,  # Batch for efficiency
    timeout=5.0,  # Higher timeout for production
    context={
        "environment": "production",
        "service": "customer-support-agent",
        "version": "2.1.0"
    }
)
```

---

## 7. Testing Strategy

### 7.1 Unit Tests

**Location**: `tupl_sdk/python/tests/test_agent_callback.py`

**Coverage**:
- Test each callback method in isolation
- Test IntentEvent mapping logic
- Test custom action mappers and sensitivity rules
- Test all three enforcement modes
- Test error handling and graceful degradation
- Test batching behavior with EventBuffer
- Test thread safety with concurrent callbacks

**Example Tests**:
```python
def test_on_chat_model_start_creates_intent_event():
    """Test LLM call creates IntentEvent with correct fields."""
    callback = AgentCallback(base_url="http://test", tenant_id="test")

    callback.on_chat_model_start(
        serialized={"name": "gpt-4"},
        messages=[{"role": "user", "content": "test"}],
        run_id="run-123"
    )

    # Verify IntentEvent stored in pending
    pending = callback._retrieve_pending("run-123")
    assert pending.action == "read"
    assert pending.actor.type == "llm"
    assert pending.resource.type == "api"

def test_tool_action_inference():
    """Test tool name → action mapping."""
    callback = AgentCallback(base_url="http://test", tenant_id="test")

    assert callback._infer_action("search_database", {}) == "read"
    assert callback._infer_action("delete_record", {}) == "delete"
    assert callback._infer_action("update_user", {}) == "write"
    assert callback._infer_action("execute_query", {}) == "execute"

def test_enforcement_mode_block_raises_exception():
    """Test block mode raises AgentSecurityException."""
    callback = AgentCallback(
        base_url="http://test",
        tenant_id="test",
        enforcement_mode="block"
    )

    # Mock Management Plane response with BLOCK decision
    with patch.object(callback.client, 'compare_intent') as mock:
        mock.return_value = ComparisonResult(decision=0, slice_similarities=[0.5]*4)

        intent = create_test_intent()
        with pytest.raises(AgentSecurityException) as exc_info:
            callback._send_and_enforce(intent)

        assert "blocked" in str(exc_info.value)

def test_network_failure_graceful_degradation():
    """Test network failures don't break execution."""
    callback = AgentCallback(base_url="http://unreachable", tenant_id="test")

    intent = create_test_intent()
    # Should log warning but not raise exception
    result = callback._send_and_enforce(intent)
    assert result.decision == 1  # ALLOW on failure
```

### 7.2 Integration Tests

**Location**: `tupl_sdk/python/tests/test_langgraph_integration.py`

**Coverage**:
- Build simple LangGraph agent with mocked LLM and tools
- Verify callback captures all expected events
- Test end-to-end flow: LangGraph → AgentCallback → Management Plane → Decision
- Test BLOCK enforcement stops execution
- Test WARN mode allows continuation
- Test network failure scenarios

**Example Integration Test**:
```python
def test_langgraph_agent_with_tupl_callback():
    """Test full integration with LangGraph agent."""
    # Create mock tools
    @tool
    def mock_search(query: str) -> str:
        return "test results"

    @tool
    def mock_delete(id: str) -> str:
        return "deleted"

    # Build test graph
    def agent_node(state: MessagesState):
        # Mock LLM response with tool call
        return {
            "messages": [
                AIMessage(
                    content="",
                    tool_calls=[{
                        "name": "mock_delete",
                        "args": {"id": "123"},
                        "id": "call-1"
                    }]
                )
            ]
        }

    graph = StateGraph(MessagesState)
    graph.add_node("agent", agent_node)
    graph.set_entry_point("agent")
    compiled = graph.compile()

    # Create callback with mock Management Plane
    with patch('httpx.Client.post') as mock_post:
        mock_post.return_value.json.return_value = {
            "decision": 0,  # BLOCK
            "slice_similarities": [0.5, 0.5, 0.5, 0.5]
        }

        tupl = AgentCallback(
            base_url="http://test",
            tenant_id="test",
            enforcement_mode="block"
        )

        # Should raise exception on BLOCK
        with pytest.raises(AgentSecurityException):
            compiled.invoke(
                {"messages": [{"role": "user", "content": "test"}]},
                config={"callbacks": [tupl]}
            )

        # Verify API was called
        assert mock_post.called
        request_body = mock_post.call_args[1]["json"]
        assert request_body["action"] == "delete"
        assert request_body["actor"]["type"] == "agent"
```

### 7.3 Performance Tests

**Location**: `tupl_sdk/python/tests/test_agent_performance.py`

**Coverage**:
- Measure callback overhead on graph execution (target: < 5ms per event)
- Test batching efficiency (10 events vs 100 events)
- Test concurrent graph executions with shared callback
- Verify no memory leaks with long-running agents

**Example Performance Test**:
```python
def test_callback_overhead():
    """Measure callback overhead on graph execution."""
    graph = create_test_graph()

    # Baseline: graph without callback
    start = time.time()
    for _ in range(100):
        graph.invoke({"messages": [{"role": "user", "content": "test"}]})
    baseline_time = time.time() - start

    # With callback
    tupl = AgentCallback(base_url="http://test", tenant_id="test")
    start = time.time()
    for _ in range(100):
        graph.invoke(
            {"messages": [{"role": "user", "content": "test"}]},
            config={"callbacks": [tupl]}
        )
    callback_time = time.time() - start

    overhead = (callback_time - baseline_time) / 100
    assert overhead < 0.005  # Less than 5ms per event
```

---

## 8. Demo Application

### 8.1 Demo Structure

**Location**: `examples/langgraph_demo/`

```
examples/langgraph_demo/
├── demo_agent.py          # Main LangGraph agent with Tupl integration
├── tools.py               # Sample tools (database, file, API operations)
├── policies.py            # Script to create sample boundaries in Management Plane
├── requirements.txt       # Dependencies (langgraph, langchain, tupl)
├── .env.example          # Environment variables template
└── README.md             # Setup and usage instructions
```

### 8.2 Demo Agent Implementation

**File**: `examples/langgraph_demo/demo_agent.py`

```python
"""
Demo LangGraph Agent with Tupl Security Integration

This demo showcases automatic intent capture and policy enforcement
for a customer support agent with database operations.
"""
from langgraph.graph import StateGraph, MessagesState
from langchain_openai import ChatOpenAI
from tupl.agent import AgentCallback, AgentSecurityException
from tools import search_database, update_record, delete_record, export_data
import os

def build_demo_agent():
    """Build a simple customer support agent with tool-calling capability."""

    def agent_node(state: MessagesState):
        model = ChatOpenAI(model="gpt-4").bind_tools([
            search_database,
            update_record,
            delete_record,
            export_data
        ])
        response = model.invoke(state["messages"])
        return {"messages": [response]}

    graph = StateGraph(MessagesState)
    graph.add_node("agent", agent_node)
    graph.set_entry_point("agent")
    return graph.compile()

def run_demo():
    """Run demo scenarios with different enforcement modes."""
    graph = build_demo_agent()

    # Configure Tupl callback
    tupl = AgentCallback(
        base_url=os.getenv("TUPL_BASE_URL", "http://localhost:8000"),
        tenant_id="demo-tenant",
        enforcement_mode="warn",  # Start with warn mode
        capture_state=False  # Only capture LLM and tool calls
    )

    # Demo scenarios
    scenarios = [
        ("Search for customer john@example.com", "read", "✅ Should ALLOW"),
        ("Update customer email to new@example.com", "write", "⚠️ Depends on boundary"),
        ("Delete customer account for user-123", "delete", "🚫 Likely BLOCK"),
        ("Export all customer data to CSV", "export", "🚫 Likely BLOCK (bulk)"),
    ]

    print("=" * 70)
    print("Tupl SDK v1.2 Demo: LangGraph Agent with Auto-Instrumentation")
    print("=" * 70)
    print(f"Enforcement Mode: {tupl.enforcement_mode}")
    print(f"Management Plane: {tupl.base_url}")
    print("=" * 70)

    for scenario_desc, action, expected in scenarios:
        print(f"\n🎯 Scenario: {scenario_desc}")
        print(f"   Expected Action: {action}")
        print(f"   Expected Outcome: {expected}")
        print("-" * 70)

        try:
            result = graph.invoke(
                {"messages": [{"role": "user", "content": scenario_desc}]},
                config={"callbacks": [tupl]}
            )
            print(f"✅ Result: {result['messages'][-1].content}")
        except AgentSecurityException as e:
            print(f"🚫 BLOCKED: {e}")
            print(f"   Intent ID: {e.intent_id}")
            print(f"   Boundary ID: {e.boundary_id}")

if __name__ == "__main__":
    run_demo()
```

### 8.3 Sample Tools

**File**: `examples/langgraph_demo/tools.py`

```python
"""Sample tools for customer support agent demo."""
from langchain.tools import tool

@tool
def search_database(query: str) -> str:
    """Search customer database by email or ID."""
    # Simulated database search
    return f"Found 1 customer matching: {query}"

@tool
def update_record(customer_id: str, field: str, value: str) -> str:
    """Update a customer record field (e.g., email, phone)."""
    # Simulated database update
    return f"Updated {field}={value} for customer {customer_id}"

@tool
def delete_record(customer_id: str) -> str:
    """Permanently delete a customer record. CAUTION: This is irreversible."""
    # Simulated database deletion
    return f"Deleted customer {customer_id} from database"

@tool
def export_data(format: str = "csv") -> str:
    """Export all customer data to file. WARNING: Contains PII."""
    # Simulated bulk export
    return f"Exported 1000 customer records to customers.{format}"
```

### 8.4 Demo Outcomes

**Expected Results**:

1. **"Search for customer john@example.com"**
   - LLM call: action="read", actor_type="llm" → ALLOW
   - Tool call: search_database → action="read", actor_type="agent" → ALLOW
   - ✅ Execution completes successfully

2. **"Update customer email to new@example.com"**
   - LLM call → ALLOW
   - Tool call: update_record → action="write", actor_type="agent"
   - ⚠️ Depends on boundary configuration (may ALLOW or BLOCK)

3. **"Delete customer account for user-123"**
   - LLM call → ALLOW
   - Tool call: delete_record → action="delete", actor_type="agent"
   - 🚫 BLOCK (matches "Block Risky Operations" boundary)
   - Enforcement: Raises `AgentSecurityException` in block mode, logs warning in warn mode

4. **"Export all customer data to CSV"**
   - LLM call → ALLOW
   - Tool call: export_data → action="export", actor_type="agent", volume="bulk", pii=True
   - 🚫 BLOCK (matches "Block Risky Operations" boundary)

---

## 9. Implementation Tasks

### Task 1: Update Management Plane Data Contracts (v1.2 Schema)

**File**: `management-plane/app/types.py`

**Changes Required**:

1. **Update Actor type field (Line ~29)**:
   ```python
   # OLD (v1.1)
   type: Literal["user", "service"]

   # NEW (v1.2)
   type: Literal["user", "service", "llm", "agent"]
   ```

2. **Update validation vocabularies (Line ~330)**:
   ```python
   # OLD (v1.1)
   VALID_ACTOR_TYPES = {"user", "service"}

   # NEW (v1.2)
   VALID_ACTOR_TYPES = {"user", "service", "llm", "agent"}  # Added llm, agent
   ```

3. **Update ActionConstraint.actor_types (Line ~188)**:
   ```python
   # OLD (v1.1)
   actor_types: list[Literal["user", "service"]]

   # NEW (v1.2)
   actor_types: list[Literal["user", "service", "llm", "agent"]]
   ```

4. **Update schema versions**:
   ```python
   # IntentEvent (Line ~93)
   schemaVersion: Literal["v1.2"] = "v1.2"  # Updated from "v1.1"

   # DesignBoundary (Line ~285)
   boundarySchemaVersion: Literal["v1.2"] = "v1.2"  # Updated from "v1.1"
   ```

5. **Update docstrings and examples** to reflect v1.2 changes

**Testing Requirements**:
- Update `management-plane/tests/test_types.py` to include tests for new actor types:
  - `test_actor_with_llm_type()`
  - `test_actor_with_agent_type()`
  - `test_intent_event_v1_2_schema()`
  - `test_action_constraint_with_new_actor_types()`
- Verify backward compatibility (v1.1 events still validate)
- All existing tests must continue to pass

**Acceptance Criteria**:
- [ ] All Pydantic models updated with new actor types
- [ ] Validation vocabularies updated
- [ ] Schema versions bumped to v1.2
- [ ] All tests passing (41 existing + new v1.2 tests)
- [ ] No breaking changes to existing v1.1 data

---

### Task 2: Update plan.md Documentation

**File**: `plan.md`

**Changes Required**:

1. **Create new appendix section** "Appendix: Slot Contract V1.2"
2. **Document new actor types** with rationale and use cases
3. **Update IntentEvent schema example** to show v1.2 with new actor types
4. **Update DesignBoundary schema example** to show v1.2 constraints
5. **Add migration guide** from v1.1 → v1.2
6. **Update all references** to schema version throughout the document

**New Section Template**:
```markdown
## Appendix: Slot Contract V1.2 (LangGraph Support)

### Changes from V1.1

**New Actor Types**:
- `llm`: Large language model (GPT-4, Claude, etc.) - represents AI decision-making entity
- `agent`: AI agent or autonomous system - represents agentic tool-calling entity

**Rationale**:
LangGraph agents operate autonomously with LLMs making decisions and agents executing tools.
Distinguishing between human users, backend services, LLMs, and agents enables:
- Granular policy enforcement (e.g., "block delete operations by agents")
- Audit trails showing which entity initiated each action
- Risk-based controls (e.g., require human approval for agent-initiated deletes)

**Example Use Cases**:
- Policy: "Agents can read database but cannot delete records"
- Policy: "LLMs can access public APIs but not internal databases"
- Policy: "Require human approval for any bulk export initiated by agents"

**Backward Compatibility**:
- V1.1 events with `actor_type: "user" | "service"` continue to work
- V1.2 boundaries can specify any combination of actor types
- Encoding pipeline handles all actor types uniformly (text canonicalization)

### V1.2 Schema Examples

[Include updated IntentEvent and DesignBoundary examples with new actor types]
```

**Acceptance Criteria**:
- [ ] New appendix section added to plan.md
- [ ] All schema examples updated to v1.2
- [ ] Migration guide documented
- [ ] Rationale clearly explained
- [ ] Use cases provided for clarity

---

### Task 3: Update Encoding Pipeline (If Needed)

**File**: `management-plane/app/encoding.py`

**Analysis Required**:

Check if slot builders need updates for new actor types. The current implementation should handle them automatically since text canonicalization treats actor types as string values.

**Current slot builder (action slot, Line ~268)**:
```python
def build_action_slot_text_for_intent(event: IntentEvent) -> str:
    """Build text for action slot (simplified v1.1)."""
    return f"action: {event.action} | actor_type: {event.actor.type}"
```

**Expected Behavior**:
- Current implementation should handle `"llm"` and `"agent"` actor types automatically
- Text canonicalization treats them as simple string values
- No special handling needed (embeddings will naturally distinguish them)

**Verification Steps**:

1. **Run existing encoding tests with new actor types**:
   ```python
   # Should work without code changes
   event = IntentEvent(..., actor=Actor(id="gpt-4", type="llm"), ...)
   vector = encode_to_128d(event)
   ```

2. **Verify determinism** (same input → same 128d vector)

3. **Check semantic similarity** between similar operations with different actor types:
   - Example: `actor_type="user"` vs `actor_type="agent"` for same action
   - Expected: High similarity (~0.8-0.9) since action and resource are the same
   - Action slot should have slightly different embeddings but still high similarity

**Testing Requirements**:

Add new test cases to `management-plane/tests/test_encoding.py`:

```python
def test_encoding_with_llm_actor_type():
    """Test encoding IntentEvent with actor_type=llm."""
    event = IntentEvent(
        id="test-001",
        schemaVersion="v1.2",
        tenantId="test",
        timestamp=time.time(),
        actor=Actor(id="gpt-4", type="llm"),
        action="read",
        resource=Resource(type="api", name="openai", location="cloud"),
        data=Data(sensitivity=["internal"], volume="single"),
        risk=Risk(authn="required")
    )

    vector = encode_to_128d(event)
    assert vector.shape == (128,)

    # Test determinism
    vector2 = encode_to_128d(event)
    assert np.allclose(vector, vector2)

def test_encoding_with_agent_actor_type():
    """Test encoding IntentEvent with actor_type=agent."""
    event = IntentEvent(
        id="test-002",
        schemaVersion="v1.2",
        tenantId="test",
        timestamp=time.time(),
        actor=Actor(id="langgraph-agent", type="agent"),
        action="delete",
        resource=Resource(type="database", name="users", location="cloud"),
        data=Data(sensitivity=["internal"], pii=True, volume="single"),
        risk=Risk(authn="required")
    )

    vector = encode_to_128d(event)
    assert vector.shape == (128,)

def test_semantic_similarity_across_actor_types():
    """Test that similar operations have high similarity regardless of actor type."""
    # Same operation, different actor types
    event_user = create_read_event(actor_type="user")
    event_agent = create_read_event(actor_type="agent")

    vec_user = encode_to_128d(event_user)
    vec_agent = encode_to_128d(event_agent)

    # Compute overall similarity
    similarity = np.dot(vec_user, vec_agent)

    # Should have high similarity (>0.8) since action/resource/data/risk are identical
    assert similarity > 0.8
```

**Acceptance Criteria**:
- [ ] Encoding tests pass with new actor types
- [ ] Determinism verified for v1.2 events
- [ ] Semantic similarity patterns validated (similar ops have >0.8 similarity)
- [ ] No performance regression
- [ ] No code changes needed (verification only)

---

### Task 4: Update All Tests for V1.2 Validation

**Files to Update**:

1. **`management-plane/tests/test_types.py`** - Add v1.2 actor type tests
2. **`management-plane/tests/test_encoding.py`** - Add encoding tests with new actors
3. **`management-plane/test_e2e_real_encoding.py`** - Update test payloads to v1.2
4. **`management-plane/test_v1_1_similarity.py`** - Rename to `test_v1_2_similarity.py` and update

**Specific Test Cases to Add**:

**In test_types.py**:
```python
def test_actor_with_llm_type():
    """Test Actor with llm type (v1.2)."""
    actor = Actor(id="gpt-4", type="llm")
    assert actor.type == "llm"

def test_actor_with_agent_type():
    """Test Actor with agent type (v1.2)."""
    actor = Actor(id="agent-123", type="agent")
    assert actor.type == "agent"

def test_intent_event_v1_2_schema():
    """Test IntentEvent with v1.2 schema version."""
    event = create_test_intent_v1_2()
    assert event.schemaVersion == "v1.2"
    assert event.actor.type in ["user", "service", "llm", "agent"]

def test_action_constraint_with_new_actor_types():
    """Test ActionConstraint accepts llm and agent (v1.2)."""
    constraint = ActionConstraint(
        actions=["read"],
        actor_types=["user", "llm", "agent"]
    )
    assert "llm" in constraint.actor_types
    assert "agent" in constraint.actor_types

def test_backward_compatibility_v1_1():
    """Test that v1.1 events still work (backward compatibility)."""
    event = IntentEvent(
        id="test-v1.1",
        schemaVersion="v1.1",  # Old version
        tenantId="test",
        timestamp=time.time(),
        actor=Actor(id="user-123", type="user"),  # Old actor type
        action="read",
        resource=Resource(type="database", name="users", location="cloud"),
        data=Data(sensitivity=["internal"], volume="single"),
        risk=Risk(authn="required")
    )
    # Should validate without error
    assert event.actor.type == "user"
```

**In test_encoding.py**:
- Add `test_encoding_with_llm_actor_type()` (from Task 3)
- Add `test_encoding_with_agent_actor_type()` (from Task 3)
- Add `test_semantic_similarity_across_actor_types()` (from Task 3)

**In test_v1_2_similarity.py** (renamed from test_v1_1_similarity.py):
```python
def test_v1_2_semantic_alignment_with_agent():
    """Test v1.2 semantic alignment with agent actor type."""
    # Create intent with agent actor
    intent = IntentEvent(
        id="intent-001",
        schemaVersion="v1.2",
        tenantId="test",
        timestamp=time.time(),
        actor=Actor(id="agent-123", type="agent"),
        action="read",
        resource=Resource(type="database", name="customers", location="cloud"),
        data=Data(sensitivity=["internal"], volume="single"),
        risk=Risk(authn="required")
    )

    # Create matching boundary
    boundary = DesignBoundary(
        id="boundary-001",
        name="Test",
        status="active",
        type="mandatory",
        boundarySchemaVersion="v1.2",
        scope=BoundaryScope(tenantId="test"),
        rules=BoundaryRules(
            thresholds=SliceThresholds(action=0.8, resource=0.8, data=0.8, risk=0.8),
            decision="min"
        ),
        constraints=BoundaryConstraints(
            action=ActionConstraint(actions=["read"], actor_types=["agent"]),
            resource=ResourceConstraint(types=["database"], locations=["cloud"]),
            data=DataConstraint(sensitivity=["internal"], volume="single"),
            risk=RiskConstraint(authn="required")
        ),
        createdAt=time.time(),
        updatedAt=time.time()
    )

    # Encode and compare
    intent_vec = encode_to_128d(intent)
    boundary_vec = encode_boundary_to_128d(boundary)

    # Compute per-slot similarities
    similarities = []
    for i in range(4):
        start = i * 32
        end = start + 32
        sim = np.dot(intent_vec[start:end], boundary_vec[start:end])
        similarities.append(sim)

    # Should have high similarity (>0.9) for matching operations
    assert similarities[0] > 0.9  # action slot
    assert np.mean(similarities) > 0.85  # overall

def test_v1_2_semantic_alignment_with_llm():
    """Test v1.2 semantic alignment with llm actor type."""
    # Similar test but with actor_type="llm"
    # ...
```

**Acceptance Criteria**:
- [ ] All test suites updated with v1.2 test cases
- [ ] Tests for both "llm" and "agent" actor types
- [ ] Semantic similarity validated for v1.2 events
- [ ] Backward compatibility tests added (v1.1 events still work)
- [ ] All tests passing (50+ total including new v1.2 tests)
- [ ] Test coverage remains at 90%+

---

### Task 5: Create User-Facing SDK Documentation

**File**: `SDK_USAGE.md` (project root)

**Sections to Include**:

1. **Quick Start** (5-line integration example)
2. **Installation** (`pip install tupl-sdk`)
3. **Basic Usage** (simple example with minimal config)
4. **Configuration Options** (all parameters explained with examples)
5. **Enforcement Modes** (block/warn/log with examples and use cases)
6. **Custom Mapping** (action_mapper, sensitivity_rules examples)
7. **Error Handling** (AgentSecurityException handling patterns)
8. **Performance Considerations** (batching, timeouts, overhead)
9. **Troubleshooting** (common issues and solutions)
10. **API Reference** (AgentCallback methods and parameters)

**Content Template**:

```markdown
# Tupl SDK v1.2: LangGraph Integration Guide

## Quick Start

Integrate Tupl security into your LangGraph agent in 3 lines:

```python
from tupl.agent import AgentCallback

tupl = AgentCallback(base_url="http://localhost:8000", tenant_id="my-tenant")
result = graph.invoke(state, config={"callbacks": [tupl]})
```

## Installation

```bash
pip install tupl-sdk
```

## Basic Usage

```python
from langgraph.graph import StateGraph, MessagesState
from langchain_openai import ChatOpenAI
from tupl.agent import AgentCallback

# Build your LangGraph agent
def agent_node(state: MessagesState):
    model = ChatOpenAI(model="gpt-4")
    response = model.invoke(state["messages"])
    return {"messages": [response]}

graph = StateGraph(MessagesState)
graph.add_node("agent", agent_node)
graph.set_entry_point("agent")
compiled = graph.compile()

# Add Tupl security
tupl = AgentCallback(
    base_url="http://localhost:8000",
    tenant_id="my-tenant"
)

# Run with security checks
result = compiled.invoke(
    {"messages": [{"role": "user", "content": "Hello"}]},
    config={"callbacks": [tupl]}
)
```

## Configuration Options

### Connection Settings

- `base_url` (str, required): Management Plane URL
- `tenant_id` (str, required): Your tenant identifier
- `api_key` (str, optional): API key for authentication
- `timeout` (float, default=2.0): Request timeout in seconds

### Capture Toggles

- `capture_llm` (bool, default=True): Capture LLM calls
- `capture_tools` (bool, default=True): Capture tool invocations
- `capture_state` (bool, default=False): Capture state transitions

### Enforcement Settings

- `enforcement_mode` (str, default="warn"): "block", "warn", or "log"
- `fallback_on_timeout` (bool, default=True): Allow on timeout/error

### Batching Settings

- `batch_size` (int, default=1): Buffer events before sending
- `batch_timeout` (float, default=5.0): Send buffer after N seconds

### Custom Mapping

- `action_mapper` (Callable, optional): Custom tool→action mapping
- `resource_type_mapper` (Callable, optional): Custom resource inference
- `sensitivity_rules` (dict, optional): Tool-specific sensitivity

### Metadata

- `context` (dict, optional): Additional context for telemetry

## Enforcement Modes

### Block Mode (Production)

```python
tupl = AgentCallback(
    base_url="https://tupl.company.com",
    tenant_id="prod-tenant",
    enforcement_mode="block"  # Raise exception on BLOCK
)

from tupl.agent import AgentSecurityException

try:
    result = graph.invoke(state, config={"callbacks": [tupl]})
except AgentSecurityException as e:
    logger.warning(f"Operation blocked: {e.metadata}")
    result = fallback_response(state)
```

**Use case**: Production systems with mandatory security policies

### Warn Mode (Gradual Rollout)

```python
tupl = AgentCallback(
    base_url="https://tupl.company.com",
    tenant_id="staging-tenant",
    enforcement_mode="warn"  # Log warning, allow execution
)

result = graph.invoke(state, config={"callbacks": [tupl]})
# Execution continues even if decision is BLOCK
# Check logs for warnings
```

**Use case**: Monitor policy violations before enforcing

### Log Mode (Development)

```python
tupl = AgentCallback(
    base_url="http://localhost:8000",
    tenant_id="dev-tenant",
    enforcement_mode="log"  # Only log, no warnings
)

result = graph.invoke(state, config={"callbacks": [tupl]})
# All operations allowed, decisions logged for telemetry
```

**Use case**: Development, A/B testing, telemetry collection

## Custom Mapping

### Custom Action Mapper

```python
def my_action_mapper(tool_name: str, tool_inputs: dict) -> str:
    """Map tool names to action types."""
    if "customer" in tool_name:
        if "delete" in tool_name:
            return "delete"
        elif "update" in tool_name or "create" in tool_name:
            return "write"
        else:
            return "read"
    return "execute"

tupl = AgentCallback(
    base_url="http://localhost:8000",
    tenant_id="my-tenant",
    action_mapper=my_action_mapper
)
```

### Sensitivity Rules

```python
tupl = AgentCallback(
    base_url="http://localhost:8000",
    tenant_id="my-tenant",
    sensitivity_rules={
        "search_public_api": "public",
        "query_user_data": "internal",
        "access_payment_info": "internal"
    }
)
```

## Error Handling

### Handling Blocked Operations

```python
from tupl.agent import AgentCallback, AgentSecurityException

tupl = AgentCallback(
    base_url="http://localhost:8000",
    tenant_id="my-tenant",
    enforcement_mode="block"
)

try:
    result = graph.invoke(state, config={"callbacks": [tupl]})
except AgentSecurityException as e:
    # Log the security event
    logger.error(f"Security violation: {e.intent_id}")
    logger.error(f"Blocked by boundary: {e.boundary_id}")
    logger.error(f"Metadata: {e.metadata}")

    # Implement fallback logic
    result = safe_fallback_response(state)
```

### Graceful Degradation

Network failures and timeouts default to ALLOW:

```python
tupl = AgentCallback(
    base_url="http://unreachable:8000",
    tenant_id="my-tenant",
    timeout=2.0,
    fallback_on_timeout=True  # ALLOW on failure (default)
)

# Even if Management Plane is down, agent continues
result = graph.invoke(state, config={"callbacks": [tupl]})
```

## Performance Considerations

### Callback Overhead

- Typical overhead: **< 1ms per event**
- LLM calls dominate latency (100-1000ms)
- Callback processing negligible in comparison

### Batching for High-Throughput

```python
tupl = AgentCallback(
    base_url="http://localhost:8000",
    tenant_id="my-tenant",
    batch_size=20,  # Send 20 events at once
    batch_timeout=5.0  # Or send after 5 seconds
)
```

**Benefits**:
- Reduces network requests (20:1 ratio)
- Lower latency for high-frequency operations
- Better throughput for production workloads

### Timeout Configuration

```python
tupl = AgentCallback(
    base_url="http://localhost:8000",
    tenant_id="my-tenant",
    timeout=5.0  # Higher timeout for production
)
```

**Guidelines**:
- Development: 2-3 seconds
- Production: 5-10 seconds
- Critical path: Consider async patterns

## Troubleshooting

### Issue: "Management Plane unreachable"

**Symptoms**: Warnings in logs about connection failures

**Solutions**:
1. Check Management Plane is running: `curl http://localhost:8000/health`
2. Verify network connectivity
3. Check firewall rules
4. Increase timeout if needed

### Issue: "All operations getting BLOCKED"

**Symptoms**: All intents return decision=0

**Solutions**:
1. Check boundary configuration in Management Plane
2. Verify boundary matches your operations
3. Review boundary thresholds (may be too strict)
4. Check actor_types in boundary (must include "agent" or "llm")

### Issue: "Callback not capturing events"

**Symptoms**: No API calls to Management Plane

**Solutions**:
1. Verify callback passed to `graph.invoke()`:
   ```python
   result = graph.invoke(state, config={"callbacks": [tupl]})
   ```
2. Check capture toggles (`capture_llm`, `capture_tools`)
3. Enable debug logging:
   ```python
   import logging
   logging.basicConfig(level=logging.DEBUG)
   ```

## API Reference

### AgentCallback

```python
class AgentCallback(BaseCallbackHandler):
    """LangGraph callback handler for Tupl security."""

    def __init__(
        self,
        base_url: str,
        tenant_id: str,
        api_key: Optional[str] = None,
        timeout: float = 2.0,
        capture_llm: bool = True,
        capture_tools: bool = True,
        capture_state: bool = False,
        enforcement_mode: str = "warn",
        fallback_on_timeout: bool = True,
        batch_size: int = 1,
        batch_timeout: float = 5.0,
        action_mapper: Optional[Callable] = None,
        resource_type_mapper: Optional[Callable] = None,
        sensitivity_rules: Optional[Dict[str, str]] = None,
        context: Optional[Dict[str, Any]] = None,
    ):
        """Initialize AgentCallback."""
```

### AgentSecurityException

```python
class AgentSecurityException(Exception):
    """Raised when an intent is blocked by Tupl policy."""

    intent_id: str  # ID of the blocked intent
    boundary_id: str  # ID of the boundary that blocked
    metadata: dict  # Additional decision metadata
```

## Next Steps

- Review [Demo Application](examples/langgraph_demo/) for complete examples
- Configure [Boundaries](docs/boundaries.md) in Management Plane
- Set up [Telemetry](docs/telemetry.md) for monitoring
- Deploy to [Production](docs/deployment.md)

---

**Version**: 1.2.0
**Last Updated**: 2025-11-13
**Support**: https://github.com/your-org/tupl-sdk/issues
```

**Acceptance Criteria**:
- [ ] Comprehensive documentation covering all features
- [ ] Multiple code examples for common use cases
- [ ] Clear explanation of enforcement modes
- [ ] Troubleshooting guide with common issues
- [ ] API reference with all parameters documented
- [ ] Easy to follow for developers unfamiliar with system
- [ ] Professional formatting and clear writing

---

### Task 6: Create Demo LangGraph Application

**Location**: `examples/langgraph_demo/`

**Files to Create**:

1. **`demo_agent.py`** - Main demo application (see Section 8.2)
2. **`tools.py`** - Sample tools (see Section 8.3)
3. **`policies.py`** - Boundary setup script
4. **`requirements.txt`** - Dependencies
5. **`.env.example`** - Environment variables template
6. **`README.md`** - Setup and usage instructions

**Content for each file**:

**requirements.txt**:
```txt
langgraph>=0.2.0
langchain>=0.3.0
langchain-openai>=0.2.0
tupl-sdk>=1.2.0
python-dotenv>=1.0.0
```

**.env.example**:
```bash
# Tupl Management Plane
TUPL_BASE_URL=http://localhost:8000
TUPL_TENANT_ID=demo-tenant

# OpenAI (for LLM)
OPENAI_API_KEY=sk-...

# Enforcement mode: block, warn, or log
TUPL_ENFORCEMENT_MODE=warn
```

**README.md**:
```markdown
# LangGraph Demo with Tupl Security

This demo showcases automatic intent capture and policy enforcement for a LangGraph agent.

## Setup

1. **Install dependencies**:
   ```bash
   pip install -r requirements.txt
   ```

2. **Configure environment**:
   ```bash
   cp .env.example .env
   # Edit .env with your settings
   ```

3. **Start Management Plane**:
   ```bash
   cd ../../management-plane
   ./run.sh
   ```

4. **Create sample boundaries**:
   ```bash
   python policies.py
   ```

## Run Demo

```bash
python demo_agent.py
```

## Expected Outcomes

### Scenario 1: Search for customer
- **Action**: `read`
- **Actor**: `agent`
- **Expected**: ✅ ALLOW

### Scenario 2: Update customer email
- **Action**: `write`
- **Actor**: `agent`
- **Expected**: ⚠️ Depends on boundary configuration

### Scenario 3: Delete customer account
- **Action**: `delete`
- **Actor**: `agent`
- **Expected**: 🚫 BLOCK (risky operation)

### Scenario 4: Export all customer data
- **Action**: `export`
- **Actor**: `agent`
- **Expected**: 🚫 BLOCK (bulk + PII)

## Customization

### Change Enforcement Mode

Edit `.env`:
```bash
TUPL_ENFORCEMENT_MODE=block  # or warn, log
```

### Add Custom Tools

Edit `tools.py` and add new `@tool` decorated functions.

### Modify Boundaries

Edit `policies.py` to create different security policies.

## Troubleshooting

**Issue**: All operations getting blocked

**Solution**: Check boundary configuration in `policies.py`. Ensure `actor_types` includes `"agent"`.

**Issue**: Management Plane unreachable

**Solution**: Ensure Management Plane is running on port 8000.
```

**policies.py** - Boundary setup script:
```python
"""Script to create sample boundaries in Management Plane."""
import httpx
from datetime import datetime
import os
from dotenv import load_dotenv

load_dotenv()

BASE_URL = os.getenv("TUPL_BASE_URL", "http://localhost:8000")
TENANT_ID = os.getenv("TUPL_TENANT_ID", "demo-tenant")

# Sample boundary: Allow safe read operations
safe_read_boundary = {
    "id": "boundary-demo-safe-read",
    "name": "Safe Read Access",
    "status": "active",
    "type": "mandatory",
    "boundarySchemaVersion": "v1.2",
    "scope": {"tenantId": TENANT_ID},
    "rules": {
        "thresholds": {
            "action": 0.85,
            "resource": 0.80,
            "data": 0.75,
            "risk": 0.70
        },
        "decision": "min"
    },
    "constraints": {
        "action": {
            "actions": ["read"],
            "actor_types": ["user", "service", "llm", "agent"]
        },
        "resource": {
            "types": ["database", "file", "api"],
            "locations": ["cloud"]
        },
        "data": {
            "sensitivity": ["internal"],
            "pii": False,
            "volume": "single"
        },
        "risk": {
            "authn": "required"
        }
    },
    "createdAt": datetime.utcnow().timestamp(),
    "updatedAt": datetime.utcnow().timestamp()
}

# Sample boundary: Block risky operations (delete, export)
block_risky_boundary = {
    "id": "boundary-demo-block-risky",
    "name": "Block Risky Operations",
    "status": "active",
    "type": "mandatory",
    "boundarySchemaVersion": "v1.2",
    "scope": {"tenantId": TENANT_ID},
    "rules": {
        "thresholds": {
            "action": 0.20,  # Very low threshold = block
            "resource": 0.20,
            "data": 0.20,
            "risk": 0.20
        },
        "decision": "min"
    },
    "constraints": {
        "action": {
            "actions": ["delete", "export"],
            "actor_types": ["llm", "agent"]
        },
        "resource": {
            "types": ["database"],
            "locations": ["cloud"]
        },
        "data": {
            "sensitivity": ["internal"],
            "pii": True,
            "volume": "bulk"
        },
        "risk": {
            "authn": "required"
        }
    },
    "createdAt": datetime.utcnow().timestamp(),
    "updatedAt": datetime.utcnow().timestamp()
}

def setup_policies():
    """Create sample boundaries in Management Plane."""
    client = httpx.Client(base_url=BASE_URL)

    print("Creating sample boundaries...")
    print(f"Base URL: {BASE_URL}")
    print(f"Tenant ID: {TENANT_ID}")
    print()

    # Create safe read boundary
    try:
        response = client.post("/api/v1/boundaries", json=safe_read_boundary)
        if response.status_code in [200, 201]:
            print("✅ Created: Safe Read Access boundary")
        else:
            print(f"❌ Failed to create safe read boundary: {response.text}")
    except Exception as e:
        print(f"❌ Error creating safe read boundary: {e}")

    # Create block risky boundary
    try:
        response = client.post("/api/v1/boundaries", json=block_risky_boundary)
        if response.status_code in [200, 201]:
            print("✅ Created: Block Risky Operations boundary")
        else:
            print(f"❌ Failed to create risky boundary: {response.text}")
    except Exception as e:
        print(f"❌ Error creating risky boundary: {e}")

    print("\nPolicies ready! Run demo_agent.py to test.")

if __name__ == "__main__":
    setup_policies()
```

**Acceptance Criteria**:
- [ ] All demo files created and functional
- [ ] Demo runs successfully end-to-end
- [ ] All scenarios execute with expected outcomes
- [ ] Clear setup instructions in README
- [ ] Environment variables template provided
- [ ] Boundary setup script works correctly
- [ ] Demo showcases both ALLOW and BLOCK decisions

---

## Summary

This comprehensive plan defines the v1.2 enhancement to the Tupl SDK, introducing automatic LangGraph instrumentation through a callback-based pattern. The enhancement achieves the goal of "5-line integration" while maintaining flexibility, security, and developer control.

**Key Deliverables**:
1. ✅ Schema updates (v1.2 with new actor types: llm, agent)
2. ✅ AgentCallback implementation
3. ✅ Comprehensive testing (unit + integration + performance)
4. ✅ Demo application showcasing real-world usage
5. ✅ User-facing documentation (SDK_USAGE.md)
6. ✅ Updated plan.md with v1.2 specification

**Estimated Timeline**: 2-3 weeks for full implementation and testing

**Implementation Order**:
1. Task 1: Update Management Plane types (1 day)
2. Task 2: Update plan.md documentation (1 day)
3. Task 3: Verify encoding pipeline (1 day)
4. Task 4: Update all tests (2 days)
5. Task 5: Create user documentation (2 days)
6. Task 6: Create demo application (2 days)

**Total Estimated Effort**: 9-10 working days

---

**Document Version**: 1.0
**Last Updated**: 2025-11-13
**Status**: ✅ Ready for Implementation
