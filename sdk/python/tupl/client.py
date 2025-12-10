"""
TuplClient - HTTP client for sending IntentEvents to the Management Plane.

Provides both sync and async APIs for capturing and sending intent events.
"""

import logging
import os
from typing import Optional
import httpx

from .types import IntentEvent, ComparisonResult
from .data_plane_client import DataPlaneError
from .buffer import EventBuffer

logger = logging.getLogger(__name__)


class TuplClient:
    """
    Client for capturing and sending IntentEvents to the Management Plane.

    Supports both immediate sending and buffered batching of events.

    Example (immediate mode):
        client = TuplClient()  # defaults to https://guard.fencio.dev
        result = client.capture(intent_event)

    Example (buffered mode):
        client = TuplClient(
            buffered=True,
            buffer_size=10,
            buffer_timeout=5.0
        )
        client.capture(intent_event)  # Buffered
        client.flush()  # Send all buffered events
    """

    def __init__(
        self,
        endpoint: Optional[str] = None,
        api_version: str = "v1",
        buffered: bool = False,
        buffer_size: int = 10,
        buffer_timeout: float = 5.0,
        timeout: float = 10.0,
        retry_count: int = 3,
        token: Optional[str] = None,
    ):
        """
        Initialize the Tupl client.

        Args:
            endpoint: Management Plane base URL
            api_version: API version to use (default: v1)
            buffered: Enable event buffering (default: False)
            buffer_size: Max events before auto-flush (default: 10)
            buffer_timeout: Seconds before auto-flush (default: 5.0)
            timeout: HTTP request timeout in seconds (default: 10.0)
            retry_count: Number of retries on failure (default: 3)
            token: Optional authentication token (Tupl gateway token starting with t_)
        """
        default_endpoint = (
            endpoint
            or os.environ.get("FENCIO_BASE_URL")
            or os.environ.get("TUPL_BASE_URL")
            or "https://guard.fencio.dev"
        )
        self.endpoint = default_endpoint.rstrip("/")
        self.api_version = api_version
        self.timeout = timeout
        self.retry_count = retry_count
        self.token = token

        # Build API URLs
        self.compare_url = f"{self.endpoint}/api/{api_version}/intents/compare"
        self.enforce_url = f"{self.endpoint}/api/{api_version}/enforce"

        # HTTP client with auth headers if token provided
        headers = {}
        if token:
            headers["Authorization"] = f"Bearer {token}"
        self.client = httpx.Client(timeout=timeout, headers=headers)

        # Optional buffering
        self.buffered = buffered
        self.buffer: Optional[EventBuffer] = None
        if buffered:
            self.buffer = EventBuffer(
                on_flush=self._send_batch,
                max_size=buffer_size,
                flush_interval=buffer_timeout,
            )
            logger.info(
                f"TuplClient initialized with buffering "
                f"(size={buffer_size}, timeout={buffer_timeout}s)"
            )
        else:
            logger.info("TuplClient initialized in immediate mode")

    def capture(self, event: IntentEvent) -> Optional[ComparisonResult]:
        """
        Capture an IntentEvent and send to Management Plane.

        In immediate mode: Sends the event immediately and returns the result.
        In buffered mode: Adds event to buffer and returns None.

        Args:
            event: The IntentEvent to capture

        Returns:
            ComparisonResult if immediate mode, None if buffered mode
        """
        if self.buffered and self.buffer:
            # Add to buffer (will auto-flush when full or timeout)
            self.buffer.add(event)
            return None
        else:
            # Send immediately
            return self._send_single(event)

    def _send_single(self, event: IntentEvent) -> Optional[ComparisonResult]:
        """
        Send a single IntentEvent to the Management Plane.

        Args:
            event: The IntentEvent to send

        Returns:
            ComparisonResult on success, None on failure
        """
        try:
            response = self.client.post(
                self.compare_url,
                json=event.model_dump(mode="json"),
                headers={"Content-Type": "application/json"},
            )
            response.raise_for_status()

            # Parse response
            result_data = response.json()
            result = ComparisonResult(**result_data)

            logger.debug(
                f"Intent {event.id} - Decision: {result.decision}, "
                f"Similarities: {result.slice_similarities}"
            )

            return result

        except httpx.HTTPError as e:
            logger.error(f"HTTP error sending intent {event.id}: {e}")
            return None
        except Exception as e:
            logger.error(f"Error sending intent {event.id}: {e}")
            return None

    def enforce_intent(self, event: IntentEvent) -> ComparisonResult:
        """Call the Management Plane enforcement proxy."""
        try:
            response = self.client.post(
                self.enforce_url,
                json=event.model_dump(mode="json"),
                headers={"Content-Type": "application/json"},
            )
            response.raise_for_status()
            return ComparisonResult(**response.json())
        except httpx.HTTPError as exc:  # type: ignore[name-defined]
            raise DataPlaneError(f"Management Plane proxy error: {exc}") from exc

    def _send_batch(self, events: list[IntentEvent]) -> None:
        """
        Send a batch of IntentEvents to the Management Plane.

        Currently sends events individually. Future optimization: batch endpoint.

        Args:
            events: List of IntentEvents to send
        """
        logger.info(f"Flushing {len(events)} events to Management Plane")

        for event in events:
            self._send_single(event)

    def flush(self) -> None:
        """
        Manually flush buffered events.

        Only applicable when buffered=True.
        """
        if self.buffer:
            self.buffer.flush()
        else:
            logger.warning("flush() called but client is not in buffered mode")

    def close(self) -> None:
        """
        Close the HTTP client and flush any buffered events.
        """
        if self.buffer:
            self.buffer.flush()
            self.buffer.stop()

        self.client.close()
        logger.info("TuplClient closed")

    def __enter__(self):
        """Context manager support."""
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        """Context manager cleanup."""
        self.close()


class AsyncTuplClient:
    """
    Async client for capturing and sending IntentEvents to the Management Plane.

    Example:
        async with AsyncTuplClient() as client:
            result = await client.capture(intent_event)
    """

    def __init__(
        self,
        endpoint: Optional[str] = None,
        api_version: str = "v1",
        timeout: float = 10.0,
        retry_count: int = 3,
    ):
        """
        Initialize the async Tupl client.

        Args:
            endpoint: Management Plane base URL
            api_version: API version to use (default: v1)
            timeout: HTTP request timeout in seconds (default: 10.0)
            retry_count: Number of retries on failure (default: 3)
        """
        default_endpoint = (
            endpoint
            or os.environ.get("FENCIO_BASE_URL")
            or os.environ.get("TUPL_BASE_URL")
            or "https://guard.fencio.dev"
        )
        self.endpoint = default_endpoint.rstrip("/")
        self.api_version = api_version
        self.timeout = timeout
        self.retry_count = retry_count

        # Build API URL
        self.compare_url = f"{self.endpoint}/api/{api_version}/intents/compare"

        # HTTP client
        self.client = httpx.AsyncClient(timeout=timeout)

        logger.info("AsyncTuplClient initialized")

    async def capture(self, event: IntentEvent) -> Optional[ComparisonResult]:
        """
        Capture an IntentEvent and send to Management Plane.

        Args:
            event: The IntentEvent to capture

        Returns:
            ComparisonResult on success, None on failure
        """
        try:
            response = await self.client.post(
                self.compare_url,
                json=event.model_dump(mode="json"),
                headers={"Content-Type": "application/json"},
            )
            response.raise_for_status()

            # Parse response
            result_data = response.json()
            result = ComparisonResult(**result_data)

            logger.debug(
                f"Intent {event.id} - Decision: {result.decision}, "
                f"Similarities: {result.slice_similarities}"
            )

            return result

        except httpx.HTTPError as e:
            logger.error(f"HTTP error sending intent {event.id}: {e}")
            return None
        except Exception as e:
            logger.error(f"Error sending intent {event.id}: {e}")
            return None

    async def close(self) -> None:
        """Close the HTTP client."""
        await self.client.aclose()
        logger.info("AsyncTuplClient closed")

    async def __aenter__(self):
        """Async context manager support."""
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """Async context manager cleanup."""
        await self.close()
