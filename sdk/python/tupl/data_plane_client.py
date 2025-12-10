"""
Data Plane gRPC client for rule enforcement.

Provides a high-level Python interface to the Rust Data Plane's gRPC server
for layer-based rule enforcement using the v1.3 IntentEvent schema.
"""

import json
import grpc
from typing import Optional
from .generated.rule_installation_pb2 import EnforceRequest, EnforceResponse
from .generated.rule_installation_pb2_grpc import DataPlaneStub
from .types import IntentEvent, ComparisonResult, BoundaryEvidence


class DataPlaneError(Exception):
    """Error communicating with the Data Plane gRPC server."""

    def __init__(self, message: str, status_code: Optional[grpc.StatusCode] = None):
        super().__init__(message)
        self.status_code = status_code


class DataPlaneClient:
    """
    gRPC client for the Rust Data Plane enforcement engine.

    Provides a Pythonic interface to the Data Plane's Enforce RPC for
    layer-based rule evaluation using semantic similarity comparison.

    Usage:
        client = DataPlaneClient("localhost:50051")
        result = client.enforce(intent_event)
        if result.decision == 0:
            raise PermissionError(f"Intent blocked: {result.evidence}")

    Configuration:
        - url: Data Plane gRPC server address (default: "localhost:50051")
        - timeout: Request timeout in seconds (default: 5.0)
        - retry: Enable automatic retries on transient failures (default: True)
    """

    def __init__(
        self,
        url: str = "localhost:50051",
        timeout: float = 5.0,
        retry: bool = True,
        insecure: bool = True,
        token: Optional[str] = None,
    ):
        """
        Initialize the Data Plane gRPC client.

        Args:
            url: Data Plane gRPC server address
                 - Local: "localhost:50051" (requires insecure=True)
                 - Remote: "guard.fencio.dev:443" (requires insecure=False, token)
            timeout: Request timeout in seconds
            retry: Enable automatic retries on transient failures
            insecure: Use insecure channel (no TLS) for local development
            token: Tenant token for authentication (required for remote connections)
                   Get tokens from: https://developer.fencio.dev
        """
        self.url = url
        self.timeout = timeout
        self.retry = retry
        self.insecure = insecure
        self.token = token

        # Create channel (connection is established lazily)
        if insecure:
            self.channel = grpc.insecure_channel(url)
        else:
            # Production: use TLS with system root certificates
            # Nginx terminates TLS, so standard SSL channel works
            credentials = grpc.ssl_channel_credentials()
            self.channel = grpc.secure_channel(url, credentials)

        self.stub = DataPlaneStub(self.channel)

    def enforce(
        self,
        intent: IntentEvent,
        intent_vector: Optional[list[float]] = None,
    ) -> ComparisonResult:
        """
        Enforce rules against an IntentEvent using layer-based evaluation.

        Calls the Data Plane's Enforce RPC which:
        1. Queries rules for the intent's layer (e.g., "L4")
        2. Fetches rule embeddings from Management Plane (with TTL cache)
        3. Compares intent against rules using semantic similarity
        4. Short-circuits on first BLOCK decision
        5. Returns aggregated result with per-rule evidence

        Args:
            intent: IntentEvent to evaluate (must include v1.3 fields like layer)

        Returns:
            ComparisonResult with decision (0=BLOCK, 1=ALLOW), similarities, and evidence

        Raises:
            DataPlaneError: On gRPC communication failure or invalid response
            ValueError: If intent is missing required v1.3 fields

        Example:
            result = client.enforce(intent_event)
            if result.decision == 0:
                blocked_rule = result.evidence[0]
                raise PermissionError(
                    f"Blocked by rule {blocked_rule.boundary_name}: "
                    f"similarity {min(blocked_rule.similarities):.2f}"
                )
        """
        # Validate v1.3 fields
        if not intent.layer:
            raise ValueError("IntentEvent must include 'layer' field for enforcement")

        # Serialize IntentEvent to JSON (gRPC expects JSON string)
        try:
            intent_json = intent.model_dump_json()
        except Exception as e:
            raise ValueError(f"Failed to serialize IntentEvent: {e}")

        # Create gRPC request
        request = EnforceRequest(
            intent_event_json=intent_json,
            intent_vector=intent_vector or [],
        )

        # Prepare metadata with token for authentication
        metadata = []
        if self.token:
            metadata.append(("authorization", f"Bearer {self.token}"))

        # Call Data Plane Enforce RPC with authentication
        try:
            response: EnforceResponse = self.stub.Enforce(
                request,
                timeout=self.timeout,
                metadata=metadata if metadata else None,
            )
        except grpc.RpcError as e:
            # Map gRPC errors to DataPlaneError
            status_code = e.code()
            details = e.details()

            # Fail-closed: treat all errors as BLOCK
            if status_code == grpc.StatusCode.UNAVAILABLE:
                raise DataPlaneError(
                    f"Data Plane unavailable: {details} (fail-closed: BLOCK)",
                    status_code
                )
            elif status_code == grpc.StatusCode.DEADLINE_EXCEEDED:
                raise DataPlaneError(
                    f"Data Plane timeout after {self.timeout}s (fail-closed: BLOCK)",
                    status_code
                )
            else:
                raise DataPlaneError(
                    f"Data Plane error [{status_code}]: {details} (fail-closed: BLOCK)",
                    status_code
                )

        # Convert gRPC response to ComparisonResult
        try:
            return self._convert_response(response)
        except Exception as e:
            raise DataPlaneError(f"Failed to parse Data Plane response: {e}")

    def _convert_response(self, response: EnforceResponse) -> ComparisonResult:
        """
        Convert gRPC EnforceResponse to SDK ComparisonResult.

        Maps proto fields to Pydantic model:
        - decision: 0=BLOCK, 1=ALLOW
        - slice_similarities: [action, resource, data, risk]
        - evidence: List of rule evaluations (boundary-compatible format)
        """
        # Convert RuleEvidence to BoundaryEvidence (legacy compat)
        evidence = [
            BoundaryEvidence(
                boundary_id=ev.rule_id,
                boundary_name=ev.rule_name,
                effect="deny" if ev.decision == 0 else "allow",
                decision=ev.decision,
                similarities=list(ev.similarities),
            )
            for ev in response.evidence
        ]

        return ComparisonResult(
            decision=response.decision,
            slice_similarities=list(response.slice_similarities),
            boundaries_evaluated=response.rules_evaluated,
            timestamp=0.0,  # TODO: Add timestamp to proto
            evidence=evidence,
        )

    def query_telemetry(
        self,
        agent_id: Optional[str] = None,
        tenant_id: Optional[str] = None,
        decision: Optional[int] = None,
        layer: Optional[str] = None,
        start_time_ms: Optional[int] = None,
        end_time_ms: Optional[int] = None,
        limit: int = 50,
        offset: int = 0,
    ):
        """
        Query telemetry sessions from Data Plane hitlogs.

        Args:
            agent_id: Filter by agent ID
            tenant_id: Filter by tenant ID
            decision: Filter by decision (0=BLOCK, 1=ALLOW, None=all)
            layer: Filter by layer (e.g., "L4")
            start_time_ms: Start time in milliseconds (Unix timestamp)
            end_time_ms: End time in milliseconds (Unix timestamp)
            limit: Maximum number of results (default 50, max 500)
            offset: Pagination offset

        Returns:
            QueryTelemetryResponse with sessions and total_count

        Raises:
            DataPlaneError: On gRPC communication failure

        Example:
            response = client.query_telemetry(
                agent_id="agent_123",
                layer="L4",
                limit=10
            )
            for session in response.sessions:
                print(f"Session {session.session_id}: {session.intent_summary}")
        """
        from .generated import QueryTelemetryRequest

        # Build request
        request = QueryTelemetryRequest(
            limit=min(limit, 500),  # Cap at 500
            offset=offset,
        )

        # Add optional filters
        if agent_id:
            request.agent_id = agent_id
        if tenant_id:
            request.tenant_id = tenant_id
        if decision is not None:
            request.decision = decision
        if layer:
            request.layer = layer
        if start_time_ms is not None:
            request.start_time_ms = start_time_ms
        if end_time_ms is not None:
            request.end_time_ms = end_time_ms

        # Call Data Plane QueryTelemetry RPC
        try:
            response = self.stub.QueryTelemetry(
                request,
                timeout=self.timeout,
            )
            return response
        except grpc.RpcError as e:
            status_code = e.code()
            details = e.details()
            raise DataPlaneError(
                f"QueryTelemetry failed [{status_code}]: {details}",
                status_code
            )

    def get_session(self, session_id: str):
        """
        Get full details for a specific enforcement session.

        Args:
            session_id: Unique session identifier

        Returns:
            GetSessionResponse with session_json (full session data as JSON string)

        Raises:
            DataPlaneError: On gRPC communication failure or session not found

        Example:
            response = client.get_session("session_001")
            session_data = json.loads(response.session_json)
            print(f"Decision: {session_data['final_decision']}")
        """
        from .generated import GetSessionRequest

        request = GetSessionRequest(session_id=session_id)

        try:
            response = self.stub.GetSession(
                request,
                timeout=self.timeout,
            )
            return response
        except grpc.RpcError as e:
            status_code = e.code()
            details = e.details()
            
            # Map NOT_FOUND to more specific error
            if status_code == grpc.StatusCode.NOT_FOUND:
                raise DataPlaneError(
                    f"Session not found: {session_id}",
                    status_code
                )
            
            raise DataPlaneError(
                f"GetSession failed [{status_code}]: {details}",
                status_code
            )


    def close(self):
        """Close the gRPC channel and cleanup resources."""
        if self.channel:
            self.channel.close()

    def __enter__(self):
        """Context manager support."""
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        """Context manager cleanup."""
        self.close()
