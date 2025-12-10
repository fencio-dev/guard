"""
AgentCallback - LangGraph callback handler for Tupl security integration.

Automatically captures LLM calls and tool executions as IntentEvents.

Example usage (v1.2 - Callback Pattern):
    from tupl.agent import AgentCallback

    tupl = AgentCallback(base_url="http://localhost:8000", tenant_id="my-tenant")
    result = graph.invoke(state, config={"callbacks": [tupl]})

Example usage (v1.3 - Enforcement Pattern):
    from tupl.agent import enforcement_agent

    agent = create_react_agent(model, tools)
    secure_agent = enforcement_agent(agent, boundary_id="ops")
    result = secure_agent.invoke({"messages": [...]})
"""

import logging
import threading
import time
from typing import Any, Optional, Dict, Callable
from uuid import uuid4
import httpx
from importlib.metadata import version

try:
    from langchain_core.callbacks import BaseCallbackHandler
    from langchain_core.messages import AIMessage
except ImportError:
    # Fallback if langchain not installed
    class BaseCallbackHandler:
        """Minimal BaseCallbackHandler for when langchain is not installed."""
        pass
    AIMessage = None

from .types import IntentEvent, Actor, Resource, Data, Risk, RateLimitContext, ComparisonResult
from .client import TuplClient
from .data_plane_client import DataPlaneClient, DataPlaneError
from .vocabulary import VocabularyRegistry
import os

logger = logging.getLogger(__name__)


# ============================================================================
# Exception Types
# ============================================================================

class AgentSecurityException(Exception):
    """Raised when an intent is blocked by Tupl policy."""

    def __init__(self, intent_id: str, boundary_id: str, decision_metadata: dict):
        self.intent_id = intent_id
        self.boundary_id = boundary_id
        self.metadata = decision_metadata
        super().__init__(
            f"Intent {intent_id} blocked by boundary {boundary_id}"
        )


# ============================================================================
# AgentCallback Implementation
# ============================================================================

class AgentCallback(BaseCallbackHandler):
    """
    LangGraph callback handler for Tupl security.

    Automatically captures LLM calls and tool executions as IntentEvents
    and enforces security policies based on Management Plane decisions.

    Example:
        tupl = AgentCallback(
            base_url="http://localhost:8000",
            tenant_id="my-tenant",
            enforcement_mode="warn"
        )

        result = graph.invoke(
            state,
            config={"callbacks": [tupl]}
        )
    """

    def __init__(
        self,
        base_url: str,
        tenant_id: str,
        api_key: Optional[str] = None,
        timeout: float = 2.0,
        capture_llm: bool = True,
        capture_tools: bool = True,
        capture_state: bool = False,
        enforcement_mode: str = "warn",  # "block", "warn", or "log"
        fallback_on_timeout: bool = True,
        batch_size: int = 1,
        batch_timeout: float = 5.0,
        action_mapper: Optional[Callable[[str, dict], str]] = None,
        resource_type_mapper: Optional[Callable[[str], str]] = None,
        sensitivity_rules: Optional[Dict[str, str]] = None,
        context: Optional[Dict[str, Any]] = None,
    ):
        """
        Initialize AgentCallback.

        Args:
            base_url: Management Plane URL
            tenant_id: Tenant identifier
            api_key: Optional API key for authentication
            timeout: Request timeout in seconds (default: 2.0)
            capture_llm: Capture LLM calls (default: True)
            capture_tools: Capture tool calls (default: True)
            capture_state: Capture state transitions (default: False)
            enforcement_mode: "block", "warn", or "log" (default: "warn")
            fallback_on_timeout: ALLOW on timeout (default: True)
            batch_size: Buffer size before sending (default: 1 = immediate)
            batch_timeout: Buffer timeout in seconds (default: 5.0)
            action_mapper: Custom tool→action mapping function
            resource_type_mapper: Custom resource type inference function
            sensitivity_rules: Tool-specific sensitivity mapping
            context: Additional context for telemetry
        """
        self.base_url = base_url
        self.tenant_id = tenant_id
        self.api_key = api_key
        self.timeout = timeout
        self.capture_llm = capture_llm
        self.capture_tools = capture_tools
        self.capture_state = capture_state
        self.enforcement_mode = enforcement_mode
        self.fallback_on_timeout = fallback_on_timeout
        self.batch_size = batch_size
        self.batch_timeout = batch_timeout
        self.action_mapper = action_mapper
        self.resource_type_mapper = resource_type_mapper
        self.sensitivity_rules = sensitivity_rules or {}
        self.context_metadata = context or {}
        self.vocab = VocabularyRegistry()

        # Initialize Tupl client
        self.client = TuplClient(
            endpoint=base_url,
            timeout=timeout,
            buffered=(batch_size > 1),
            buffer_size=batch_size,
            buffer_timeout=batch_timeout
        )

        # Thread-local storage for pending events
        self._pending_events = threading.local()

        logger.info(
            f"AgentCallback initialized: tenant={tenant_id}, mode={enforcement_mode}"
        )

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

    def _infer_action(self, tool_name: str, tool_inputs: dict) -> str:
        """
        Infer action type from tool name.

        Uses custom mapper if provided, otherwise uses default mapping.
        """
        # Use custom mapper if provided
        if self.action_mapper:
            return self.action_mapper(tool_name, tool_inputs)

        # Use vocabulary for inference
        action = self.vocab.infer_action_from_tool_name(tool_name)
        if action:
            return action

        if any(key in tool_inputs for key in ["query", "search", "filter"]):
            return "read"

        return "execute"

    def _infer_resource_type(self, tool_name: str) -> str:
        """
        Infer resource type from tool name.

        Uses custom mapper if provided, otherwise uses default mapping.
        """
        if self.resource_type_mapper:
            return self.resource_type_mapper(tool_name)
        return self.vocab.infer_resource_type_from_tool_name(tool_name)

    def _get_sensitivity(self, tool_name: str) -> list[str]:
        """Get sensitivity for tool based on rules."""
        if tool_name in self.sensitivity_rules:
            return [self.sensitivity_rules[tool_name]]
        return ["internal"]  # Default

    def _send_and_enforce(self, event: IntentEvent):
        """
        Send IntentEvent to Management Plane and enforce decision.

        Handles enforcement based on mode and network failures.
        """
        try:
            result = self.client.capture(event)

            # Handle buffered mode (returns None)
            if result is None:
                return

            # Enforce decision
            if result.decision == 0:  # BLOCK
                if self.enforcement_mode == "block":
                    # Extract blocking boundary from evidence
                    boundary_id = "unknown"
                    boundary_name = "unknown"
                    if result.evidence:
                        # Find first deny boundary that matched, or first allow boundary that failed
                        deny_match = next((e for e in result.evidence if e.effect == "deny" and e.decision == 1), None)
                        allow_fail = next((e for e in result.evidence if e.effect == "allow" and e.decision == 0), None)
                        blocking = deny_match or allow_fail
                        if blocking:
                            boundary_id = blocking.boundary_id
                            boundary_name = blocking.boundary_name

                    raise AgentSecurityException(
                        intent_id=event.id,
                        boundary_id=f"{boundary_id} ({boundary_name})",
                        decision_metadata={"similarities": result.slice_similarities}
                    )
                elif self.enforcement_mode == "warn":
                    logger.warning(
                        f"Intent {event.id} blocked but allowed in warn mode"
                    )
                elif self.enforcement_mode == "log":
                    logger.info(
                        f"Intent {event.id} would be blocked (log mode)"
                    )

        except AgentSecurityException:
            # Re-raise security exceptions (not network errors)
            raise
        except Exception as e:
            # Graceful degradation on network failures only
            if self.fallback_on_timeout:
                logger.warning(f"Management Plane error: {e}. Allowing execution.")
            else:
                raise

    # ========================================================================
    # LLM Event Handlers
    # ========================================================================

    def on_chat_model_start(
        self,
        serialized: Dict[str, Any],
        messages: list,
        run_id: Any,
        parent_run_id: Any = None,
        **kwargs: Any
    ):
        """Capture LLM call initiation."""
        if not self.capture_llm:
            return

        # Extract model name
        model_name = serialized.get("name", "unknown-llm")

        # Create IntentEvent
        event = IntentEvent(
            id=f"intent_{run_id}",
            schemaVersion="v1.2",
            tenantId=self.tenant_id,
            timestamp=time.time(),
            actor=Actor(
                id=model_name,
                type="llm"
            ),
            action="read",  # LLM calls read data
            resource=Resource(
                type="api",
                name=f"llm://{model_name}",
                location="cloud"
            ),
            data=Data(
                sensitivity=["internal"],  # Prompts are internal
                pii=None,
                volume="single"
            ),
            risk=Risk(
                authn="required"  # API key required
            ),
            context=self.context_metadata
        )

        # Store pending event
        self._store_pending(str(run_id), event)

    def on_llm_start(
        self,
        serialized: Dict[str, Any],
        prompts: list,
        run_id: Any,
        parent_run_id: Any = None,
        **kwargs: Any
    ):
        """Fallback for non-chat LLM calls."""
        if not self.capture_llm:
            return

        # Treat same as chat model
        self.on_chat_model_start(serialized, prompts, run_id, parent_run_id, **kwargs)

    def on_llm_end(self, response: Any, run_id: Any, **kwargs: Any):
        """Capture LLM completion and send event."""
        if not self.capture_llm:
            return

        # Retrieve pending event
        event = self._retrieve_pending(str(run_id))
        if event is None:
            return

        # Send and enforce
        self._send_and_enforce(event)

    def on_llm_error(self, error: Exception, run_id: Any, **kwargs: Any):
        """Clean up pending event on LLM error."""
        # Clean up pending event
        self._retrieve_pending(str(run_id))
        logger.debug(f"LLM error: {error}")

    # ========================================================================
    # Tool Event Handlers
    # ========================================================================

    def on_tool_start(
        self,
        serialized: Dict[str, Any],
        input_str: str,
        run_id: Any,
        parent_run_id: Any = None,
        **kwargs: Any
    ):
        """Capture tool invocation."""
        if not self.capture_tools:
            return

        # Extract tool name
        tool_name = serialized.get("name", "unknown-tool")

        # Parse inputs (may be string or dict)
        tool_inputs = kwargs.get("inputs", {})
        if isinstance(tool_inputs, str):
            tool_inputs = {}

        # Infer action and resource type
        action = self._infer_action(tool_name, tool_inputs)
        resource_type = self._infer_resource_type(tool_name)
        sensitivity = self._get_sensitivity(tool_name)

        # Determine if PII based on action type
        pii = action in ["delete", "export"]

        # Create IntentEvent
        event = IntentEvent(
            id=f"intent_{run_id}",
            schemaVersion="v1.2",
            tenantId=self.tenant_id,
            timestamp=time.time(),
            actor=Actor(
                id=str(parent_run_id) if parent_run_id else "agent",
                type="agent"
            ),
            action=action,
            resource=Resource(
                type=resource_type,
                name=tool_name,
                location="cloud"
            ),
            data=Data(
                sensitivity=sensitivity,
                pii=pii,
                volume="single"
            ),
            risk=Risk(
                authn="required"
            ),
            context=self.context_metadata
        )

        # Store pending event
        self._store_pending(str(run_id), event)

    def on_tool_end(self, output: str, run_id: Any, **kwargs: Any):
        """Capture tool completion and send event."""
        if not self.capture_tools:
            return

        # Retrieve pending event
        event = self._retrieve_pending(str(run_id))
        if event is None:
            return

        # Send and enforce
        self._send_and_enforce(event)

    def on_tool_error(self, error: Exception, run_id: Any, **kwargs: Any):
        """Clean up pending event on tool error."""
        # Clean up pending event
        self._retrieve_pending(str(run_id))
        logger.debug(f"Tool error: {error}")

    # ========================================================================
    # State Transition Handlers (Optional)
    # ========================================================================

    def on_chain_start(
        self,
        serialized: Dict[str, Any],
        inputs: Dict[str, Any],
        run_id: Any,
        parent_run_id: Any = None,
        **kwargs: Any
    ):
        """Capture state transitions (if enabled)."""
        if not self.capture_state:
            return

        # Extract node/chain name
        node_name = serialized.get("name", "unknown-node")

        # Create IntentEvent for state transition
        event = IntentEvent(
            id=f"intent_{run_id}",
            schemaVersion="v1.2",
            tenantId=self.tenant_id,
            timestamp=time.time(),
            actor=Actor(
                id="agent",
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
            ),
            context=self.context_metadata
        )

        # Store pending event
        self._store_pending(str(run_id), event)

    def on_chain_end(self, outputs: Dict[str, Any], run_id: Any, **kwargs: Any):
        """Capture chain/state completion."""
        if not self.capture_state:
            return

        # Retrieve and send event
        event = self._retrieve_pending(str(run_id))
        if event:
            self._send_and_enforce(event)


# ============================================================================
# Enforcement Pattern (v1.3) - Transparent Proxy Wrapper
# ============================================================================

class SecureGraphProxy:
    """
    Transparent proxy wrapper for LangGraph compiled graphs that enforces
    security policies via streaming interception.

    Intercepts tool calls between LLM decision and tool execution, enforcing
    policies before tools run. Works with any compiled graph (create_react_agent,
    custom StateGraph, etc.).

    Example:
        from tupl.agent import enforcement_agent
        from langgraph.prebuilt import create_react_agent

        # Build normal agent
        agent = create_react_agent(model, tools)

        # Wrap with enforcement (5 lines)
        secure_agent = enforcement_agent(
            agent,
            boundary_id="ops-policy",
            base_url="http://localhost:8000"
        )

        # Use normally - enforcement is automatic
        result = secure_agent.invoke({"messages": [...]})
    """

    def __init__(
        self,
        graph: Any,
        agent_id: str,
        boundary_id: str,
        client: Optional[TuplClient] = None,
        base_url: Optional[str] = None,
        tenant_id: str = "default",
        timeout: float = 10.0,
        on_violation: Optional[Callable[[IntentEvent, ComparisonResult], None]] = None,
        action_mapper: Optional[Callable[[str, dict], str]] = None,
        resource_type_mapper: Optional[Callable[[str], str]] = None,
        enforcement_mode: Optional[str] = None,
        data_plane_url: Optional[str] = None,
        token: Optional[str] = None,
        soft_block: bool = True,
        on_soft_block: Optional[Callable[[IntentEvent, ComparisonResult], None]] = None,
    ):
        """
        Initialize SecureGraphProxy.

        Args:
            graph: Compiled LangGraph graph to wrap
            agent_id: Agent identifier (required for registration)
            boundary_id: Boundary ID to enforce against
            client: Optional TuplClient instance (created if not provided)
            base_url: Management Plane URL (defaults to https://guard.fencio.dev, overridable via FENCIO_BASE_URL/TUPL_BASE_URL)
            tenant_id: Tenant identifier (default: "default")
            timeout: Request timeout in seconds (default: 10.0)
            on_violation: Optional callback when policy is violated
            action_mapper: Custom tool→action mapping function
            resource_type_mapper: Custom resource type inference function
            enforcement_mode: Enforcement mode: "data_plane" or "management_plane"
                              (default: read from TUPL_ENFORCEMENT_MODE env var or "management_plane")
            data_plane_url: Data Plane gRPC URL (default: read from TUPL_DATA_PLANE_URL or "localhost:50051")
            token: Tenant token for authentication (required for remote Data Plane connections)
                   Get tokens from: https://developer.fencio.dev
            soft_block: If True, log violations without raising exceptions (default: True)
            on_soft_block: Optional callback for soft-block violations
        """
        self._graph = graph
        self.agent_id = agent_id
        self.boundary_id = boundary_id
        self.on_violation = on_violation
        self.action_mapper = action_mapper or self._default_action_mapper
        self.resource_type_mapper = resource_type_mapper or self._default_resource_type_mapper
        self.tenant_id = tenant_id
        self.token = token
        resolved_base_url = (
            base_url
            or os.environ.get("FENCIO_BASE_URL")
            or os.environ.get("TUPL_BASE_URL")
            or "https://guard.fencio.dev"
        )
        self.base_url = resolved_base_url.rstrip("/")
        self.soft_block = soft_block
        self.on_soft_block = on_soft_block or self._default_soft_block_handler

        # NEW v1.3: Rate limit tracking
        self._rate_limit_tracker: Dict[str, Dict[str, Any]] = {}  # {agent_id: {window_start, call_count}}
        self._rate_window_seconds = 60  # 1 minute window
        self.vocab = VocabularyRegistry()

        # Track enforced tool calls to avoid duplicate enforcement in stream
        self._enforced_tool_call_ids: set[str] = set()

        # NEW v1.3: Enforcement mode configuration
        self.enforcement_mode = enforcement_mode or os.environ.get("TUPL_ENFORCEMENT_MODE", "management_plane")
        data_plane_url = data_plane_url or os.environ.get("TUPL_DATA_PLANE_URL", "localhost:50051")

        # Initialize appropriate client based on enforcement mode
        if self.enforcement_mode == "data_plane":
            # Determine if remote connection (requires TLS and token)
            is_remote = data_plane_url and \
                        "localhost" not in data_plane_url and \
                        "127.0.0.1" not in data_plane_url

            # Use gRPC Data Plane for enforcement
            self.data_plane_client = DataPlaneClient(
                url=data_plane_url,
                timeout=timeout,
                insecure=not is_remote,  # TLS for remote, insecure for local
                token=token,  # Pass token for authentication
            )
            self.client = None  # Don't use Management Plane client
            logger.info(
                f"SecureGraphProxy initialized: mode=data_plane, boundary={boundary_id}, "
                f"data_plane={data_plane_url}, remote={is_remote}, authenticated={bool(token)}"
            )
        else:
            # Management Plane proxy mode: Use Management Plane HTTP API
            self.data_plane_client = None
            if client is None:
                self.client = TuplClient(
                    endpoint=base_url,
                    timeout=timeout,
                    buffered=False,  # Immediate mode for enforcement
                    token=token,  # Pass token for authentication
                )
            else:
                self.client = client
            logger.info(
                f"SecureGraphProxy initialized: mode=management_plane, boundary={boundary_id}, "
                f"endpoint={self.base_url if client is None else 'custom client'}, "
                f"authenticated={bool(token)}"
            )

        # Auto-register agent
        self._register_agent()

        # Auto-install policy if using data_plane mode
        if self.enforcement_mode == "data_plane":
            self._fetch_and_install_policy()

    def _register_agent(self):
        """Auto-register agent with Management Plane."""
        # Try to get version, preferring fencio package name
        sdk_version = "unknown"
        for pkg_name in ["fencio", "tupl", "tupl-sdk"]:
            try:
                sdk_version = version(pkg_name)
                break
            except Exception:
                continue

        try:
            response = httpx.post(
                f"{self.base_url}/api/v1/agents/register",
                json={
                    "agent_id": self.agent_id,
                    "sdk_version": sdk_version,
                    "metadata": {}
                },
                headers={"Authorization": f"Bearer {self.token}"} if self.token else {},
                timeout=2.0
            )

            if response.status_code == 200:
                logger.info(f"Agent '{self.agent_id}' registered successfully")
            else:
                logger.warning(
                    "Agent registration failed",
                    extra={
                        "agent_id": self.agent_id,
                        "status": response.status_code,
                        "body": response.text,
                    }
                )

        except Exception as e:
            # Non-critical - don't break enforcement
            logger.warning(f"Agent registration exception: {e}")

    def _fetch_and_install_policy(self):
        """Fetch agent policy from Management Plane and install to Data Plane."""
        if not self.data_plane_client:
            logger.debug("No Data Plane client - skipping policy installation")
            return

        try:
            # Fetch policy from Management Plane
            response = httpx.get(
                f"{self.base_url}/api/v1/agents/policies/{self.agent_id}",
                headers={"Authorization": f"Bearer {self.token}"} if self.token else {},
                timeout=2.0
            )

            if response.status_code == 404:
                logger.info(f"No policy configured for agent '{self.agent_id}' yet")
                return
            elif response.status_code != 200:
                logger.warning(
                    f"Failed to fetch policy for agent '{self.agent_id}': "
                    f"HTTP {response.status_code}"
                )
                return

            policy_data = response.json()
            logger.info(
                f"Fetched policy for agent '{self.agent_id}': "
                f"template={policy_data['template_id']}"
            )

            # Policy installation is now handled by Management Plane
            # The rules are already installed when the policy is created
            # This method serves as a validation check
            logger.debug(
                f"Policy for agent '{self.agent_id}' should already be installed "
                "by Management Plane"
            )

        except Exception as e:
            # Non-critical - don't break enforcement
            logger.warning(f"Policy fetch exception for agent '{self.agent_id}': {e}")

    def _default_soft_block_handler(self, event: IntentEvent, result: ComparisonResult):
        """Log violation without halting execution."""
        logger.warning(
            f"SOFT-BLOCK: Tool call '{event.tool_name}' would be blocked "
            f"by boundary '{self.boundary_id}'. "
            f"Intent ID: {event.id}, Similarities: {result.slice_similarities}"
        )

    def _handle_block_decision(self, event: IntentEvent, result: ComparisonResult):
        """Handle block decision based on soft/hard block mode."""
        if result.decision == 0:  # BLOCK
            if self.soft_block:
                # Soft-block: Log and continue
                self.on_soft_block(event, result)
            else:
                # Hard-block: Raise exception
                if self.on_violation:
                    self.on_violation(event, result)
                raise PermissionError(
                    f"Tool call '{event.tool_name}' blocked by boundary '{self.boundary_id}'. "
                    f"Similarities: {result.slice_similarities}"
                )

    def _default_action_mapper(self, tool_name: str, tool_inputs: dict) -> str:
        """Default tool name to action mapping using the canonical vocabulary."""
        action = self.vocab.infer_action_from_tool_name(tool_name)
        if action:
            return action

        if any(keyword in tool_inputs for keyword in ["query", "search", "filter"]):
            return "read"

        return "execute"  # Conservative default

    def _default_resource_type_mapper(self, tool_name: str) -> str:
        """Default resource type inference using the canonical vocabulary."""
        return self.vocab.infer_resource_type_from_tool_name(tool_name)

    def _get_rate_limit_context(self) -> RateLimitContext:
        """
        Track rate limit state per agent (v1.3).

        Returns:
            RateLimitContext with current window state
        """
        agent_id = self.agent_id
        current_time = time.time()

        if agent_id not in self._rate_limit_tracker:
            self._rate_limit_tracker[agent_id] = {
                "window_start": current_time,
                "call_count": 0
            }

        tracker = self._rate_limit_tracker[agent_id]

        # Reset window if expired
        if current_time - tracker["window_start"] > self._rate_window_seconds:
            tracker["window_start"] = current_time
            tracker["call_count"] = 0

        # Increment count
        tracker["call_count"] += 1

        return RateLimitContext(
            agent_id=agent_id,
            window_start=tracker["window_start"],
            call_count=tracker["call_count"]
        )

    def _infer_tool_method(self, tool_name: str, tool_args: dict) -> str:
        """
        Infer tool method from tool name and arguments.

        Args:
            tool_name: Tool name
            tool_args: Tool arguments

        Returns:
            Inferred method: query, read, write, execute, delete
        """
        tool_lower = tool_name.lower()

        # Check tool name for method keywords
        for method in ["read", "write", "query", "execute", "delete"]:
            if method in tool_lower:
                return method

        # Infer from primary input parameter
        if "query" in tool_args or "search" in tool_args:
            return "query"
        elif "path" in tool_args or "file" in tool_args:
            return "read" if "write" not in tool_lower else "write"

        return "execute"  # Default

    def _create_intent_from_tool_call(self, tool_call: dict) -> IntentEvent:
        """
        Create IntentEvent from LangChain tool call dict (v1.3 with layer inference).

        Args:
            tool_call: Tool call dict with keys: name, args, id

        Returns:
            IntentEvent for enforcement
        """
        tool_name = tool_call.get("name", "unknown-tool")
        tool_args = tool_call.get("args", {})
        tool_id = tool_call.get("id", str(uuid4()))

        # Infer action and resource type
        action = self.action_mapper(tool_name, tool_args)
        resource_type = self.resource_type_mapper(tool_name)

        # NEW v1.3: Infer tool method and get rate limit context
        tool_method = self._infer_tool_method(tool_name, tool_args)
        rate_context = self._get_rate_limit_context()

        # Determine PII and sensitivity based on action
        pii = action in ["delete", "export"]
        sensitivity = ["public"] if action == "read" else ["internal"]

        # Create IntentEvent with v1.3 fields
        event = IntentEvent(
            id=f"intent_{tool_id}",
            schemaVersion="v1.3",  # Updated to v1.3
            tenantId=self.tenant_id,
            timestamp=time.time(),
            actor=Actor(
                id=self.agent_id,
                type="agent"
            ),
            action=action,
            resource=Resource(
                type=resource_type,
                name=tool_name,
                location="cloud"
            ),
            data=Data(
                sensitivity=sensitivity,
                pii=pii,
                volume="single"
            ),
            risk=Risk(
                authn="required"
            ),
            context={"tool_args": tool_args},
            # NEW v1.3 fields for layer-based enforcement
            layer="L4",  # Hardcoded for MVP - tool calls are L4 ToolGateway
            tool_name=tool_name,
            tool_method=tool_method,
            tool_params=tool_args,
            rate_limit_context=rate_context
        )

        return event

    def _enforce_tool_calls(self, state: dict) -> None:
        """
        Enforce policy on tool calls in state.

        Inspects the state for AIMessage with tool_calls and enforces
        policy for each tool call. Raises PermissionError on BLOCK.

        Args:
            state: Graph state dict with "messages" key

        Raises:
            PermissionError: If any tool call is blocked by policy
        """
        messages = state.get("messages", [])

        # Find AIMessage with tool_calls (last message should be AI response)
        for message in reversed(messages):
            # Check for tool_calls attribute (works with both real AIMessage and mocks)
            tool_calls = getattr(message, "tool_calls", [])

            if tool_calls:
                logger.debug(f"[ENFORCEMENT] Found {len(tool_calls)} tool_calls in message type={type(message).__name__}")

            if tool_calls:
                # Enforce each tool call
                for tool_call in tool_calls:
                    # Check if we've already enforced this tool call (deduplication)
                    tool_call_id = tool_call.get("id")
                    if tool_call_id and tool_call_id in self._enforced_tool_call_ids:
                        logger.debug(f"Skipping already-enforced tool call: {tool_call_id}")
                        continue

                    # Create intent event
                    event = self._create_intent_from_tool_call(tool_call)

                    logger.debug(
                        f"Enforcing tool call: {tool_call.get('name')} "
                        f"(action={event.action}, layer={event.layer}, mode={self.enforcement_mode})"
                    )

                    # Enforce via Data Plane (v1.3) or Management Plane (legacy)
                    try:
                        if self.enforcement_mode == "data_plane":
                            # Use gRPC Data Plane for layer-based enforcement
                            result = self.data_plane_client.enforce(event)
                        else:
                            # Proxy through Management Plane enforcement endpoint
                            result = self.client.enforce_intent(event)
                    except DataPlaneError as e:
                        # Data Plane error - fail closed (BLOCK)
                        logger.error(
                            f"Data Plane error for tool {tool_call.get('name')}: {e}. "
                            f"Blocking execution (fail-closed)."
                        )
                        raise PermissionError(
                            f"Tool call '{tool_call.get('name')}' blocked due to Data Plane error. "
                            f"Intent ID: {event.id}, Error: {str(e)}"
                        )

                    # Check decision
                    if result.decision == 0:  # BLOCK
                        # Always notify violation callback when provided
                        if self.on_violation:
                            self.on_violation(event, result)

                        # Extract blocking boundary from evidence for better messaging
                        boundary_info = self.boundary_id
                        if result.evidence:
                            deny_match = next((e for e in result.evidence if e.effect == "deny" and e.decision == 1), None)
                            allow_fail = next((e for e in result.evidence if e.effect == "allow" and e.decision == 0), None)
                            blocking = deny_match or allow_fail
                            if blocking:
                                boundary_info = f"{blocking.boundary_id} ({blocking.boundary_name})"

                        if self.soft_block:
                            # Soft block: raise callback/log but allow tool execution to continue
                            self.on_soft_block(event, result)
                            logger.info(
                                "Soft-blocked tool call '%s' by boundary '%s' (similarities=%s)",
                                tool_call.get('name'),
                                boundary_info,
                                result.slice_similarities,
                            )
                            # Mark as enforced even on soft block
                            if tool_call_id:
                                self._enforced_tool_call_ids.add(tool_call_id)
                            continue

                        # Hard block - raise to halt execution
                        raise PermissionError(
                            f"Tool call '{tool_call.get('name')}' blocked by boundary '{boundary_info}'. "
                            f"Intent ID: {event.id}, Similarities: {result.slice_similarities}"
                        )
                    else:
                        logger.debug(
                            f"Tool call '{tool_call.get('name')}' allowed by boundary '{self.boundary_id}'. "
                            f"Similarities: {result.slice_similarities}"
                        )

                    # Mark as enforced
                    if tool_call_id:
                        self._enforced_tool_call_ids.add(tool_call_id)

                # Only process first message with tool_calls
                break

    def invoke(self, inputs: dict, config: Optional[dict] = None, **kwargs) -> dict:
        """
        Invoke graph with enforcement.

        Streams execution to intercept tool calls before execution.

        Args:
            inputs: Graph inputs
            config: Optional config dict
            **kwargs: Additional kwargs passed to graph.stream

        Returns:
            Final graph state

        Raises:
            PermissionError: If tool call is blocked by policy
        """
        # Clear enforced tool calls for new invocation
        self._enforced_tool_call_ids.clear()

        # Stream execution to intercept tool calls
        final_state = None

        for state in self._graph.stream(inputs, config=config, stream_mode="values", **kwargs):
            # Enforce policy on tool calls in this state
            self._enforce_tool_calls(state)

            # Keep track of final state
            final_state = state

        return final_state

    def stream(self, inputs: dict, config: Optional[dict] = None, **kwargs):
        """
        Stream graph execution with enforcement.

        Yields states while enforcing policy on tool calls.

        Args:
            inputs: Graph inputs
            config: Optional config dict
            **kwargs: Additional kwargs passed to graph.stream

        Yields:
            Graph states

        Raises:
            PermissionError: If tool call is blocked by policy
        """
        # Clear enforced tool calls for new stream
        self._enforced_tool_call_ids.clear()

        for state in self._graph.stream(inputs, config=config, stream_mode="values", **kwargs):
            # Enforce policy on tool calls
            self._enforce_tool_calls(state)

            # Yield state to caller
            yield state

    async def ainvoke(self, inputs: dict, config: Optional[dict] = None, **kwargs) -> dict:
        """
        Async invoke graph with enforcement.

        Args:
            inputs: Graph inputs
            config: Optional config dict
            **kwargs: Additional kwargs passed to graph.astream

        Returns:
            Final graph state

        Raises:
            PermissionError: If tool call is blocked by policy
        """
        # Stream execution to intercept tool calls
        final_state = None

        async for state in self._graph.astream(inputs, config=config, stream_mode="values", **kwargs):
            # Enforce policy on tool calls in this state
            self._enforce_tool_calls(state)

            # Keep track of final state
            final_state = state

        return final_state

    async def astream(self, inputs: dict, config: Optional[dict] = None, **kwargs):
        """
        Async stream graph execution with enforcement.

        Yields states while enforcing policy on tool calls.

        Args:
            inputs: Graph inputs
            config: Optional config dict
            **kwargs: Additional kwargs passed to graph.astream

        Yields:
            Graph states

        Raises:
            PermissionError: If tool call is blocked by policy
        """
        async for state in self._graph.astream(inputs, config=config, stream_mode="values", **kwargs):
            # Enforce policy on tool calls
            self._enforce_tool_calls(state)

            # Yield state to caller
            yield state

    def __getattr__(self, name: str):
        """
        Proxy all other attributes/methods to wrapped graph.

        Makes SecureGraphProxy transparent - it looks like the original graph.
        """
        return getattr(self._graph, name)


def enforcement_agent(
    graph: Any,
    agent_id: str,
    boundary_id: str = "default",
    client: Optional[TuplClient] = None,
    base_url: Optional[str] = None,
    tenant_id: str = "default",
    timeout: float = 10.0,
    on_violation: Optional[Callable[[IntentEvent, ComparisonResult], None]] = None,
    action_mapper: Optional[Callable[[str, dict], str]] = None,
    resource_type_mapper: Optional[Callable[[str], str]] = None,
    enforcement_mode: Optional[str] = None,
    data_plane_url: Optional[str] = None,
    token: Optional[str] = None,
    soft_block: bool = True,
    on_soft_block: Optional[Callable[[IntentEvent, ComparisonResult], None]] = None,
) -> SecureGraphProxy:
    """
    Wrap a LangGraph compiled graph with policy enforcement.

    This is the primary integration point for the v1.3 enforcement pattern.
    Simply wrap your compiled graph and use it normally - enforcement is automatic.

    Example (Local):
        from tupl.agent import enforcement_agent
        from langgraph.prebuilt import create_react_agent

        # Build agent
        agent = create_react_agent(model, tools)

        # Wrap with local enforcement
        secure_agent = enforcement_agent(
            agent,
            agent_id="my-agent",
            boundary_id="ops",
            enforcement_mode="data_plane",
            data_plane_url="localhost:50051"
        )

    Example (Remote):
        # Wrap with remote enforcement (requires token)
        secure_agent = enforcement_agent(
            agent,
            agent_id="my-agent",
            boundary_id="ops",
            enforcement_mode="data_plane",
            data_plane_url="guard.fencio.dev:443",
            token=os.getenv("TUPL_TOKEN")
        )

    Args:
        graph: Compiled LangGraph graph (from create_react_agent or StateGraph.compile())
        agent_id: Agent identifier (required for registration)
        boundary_id: Boundary ID to enforce against
        client: Optional TuplClient instance
        base_url: Management Plane URL (defaults to https://guard.fencio.dev, overridable via FENCIO_BASE_URL/TUPL_BASE_URL)
        tenant_id: Tenant identifier (default: "default")
        timeout: Request timeout in seconds (default: 10.0)
        on_violation: Optional callback when policy is violated
        action_mapper: Custom tool→action mapping function
        resource_type_mapper: Custom resource type inference function
        enforcement_mode: Enforcement mode: "data_plane" or "management_plane"
        data_plane_url: Data Plane gRPC URL (e.g., "localhost:50051" or "guard.fencio.dev:443")
        token: Tenant token for authentication (required for remote data_plane_url)
                Get tokens from: https://developer.fencio.dev
        soft_block: If True, log violations without raising exceptions (default: True)
        on_soft_block: Optional callback for soft-block violations

    Returns:
        SecureGraphProxy wrapping the graph with enforcement
    """
    return SecureGraphProxy(
        graph=graph,
        agent_id=agent_id,
        boundary_id=boundary_id,
        client=client,
        base_url=base_url,
        tenant_id=tenant_id,
        timeout=timeout,
        on_violation=on_violation,
        action_mapper=action_mapper,
        resource_type_mapper=resource_type_mapper,
        enforcement_mode=enforcement_mode,
        data_plane_url=data_plane_url,
        token=token,
        soft_block=soft_block,
        on_soft_block=on_soft_block,
    )
