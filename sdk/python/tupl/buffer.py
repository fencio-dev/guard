"""
EventBuffer - Batching buffer for IntentEvents.

Automatically flushes events based on size and time thresholds.
"""

import logging
import threading
from typing import Callable, Generic, TypeVar
from collections import deque

logger = logging.getLogger(__name__)

T = TypeVar("T")


class EventBuffer(Generic[T]):
    """
    Thread-safe buffer for batching events with auto-flush.

    Flushes events when:
    - Buffer reaches max_size
    - flush_interval seconds have elapsed since last flush
    - Manual flush() is called

    Example:
        def handle_batch(events):
            print(f"Flushing {len(events)} events")

        buffer = EventBuffer(
            on_flush=handle_batch,
            max_size=10,
            flush_interval=5.0
        )

        buffer.add(event1)
        buffer.add(event2)
        # ... automatically flushes when max_size reached or 5s elapsed

        buffer.stop()  # Clean shutdown
    """

    def __init__(
        self,
        on_flush: Callable[[list[T]], None],
        max_size: int = 10,
        flush_interval: float = 5.0,
    ):
        """
        Initialize the event buffer.

        Args:
            on_flush: Callback function to handle flushed events
            max_size: Maximum buffer size before auto-flush (default: 10)
            flush_interval: Seconds between auto-flushes (default: 5.0)
        """
        self.on_flush = on_flush
        self.max_size = max_size
        self.flush_interval = flush_interval

        # Thread-safe buffer
        self._buffer: deque[T] = deque()
        self._lock = threading.Lock()

        # Background timer for periodic flushing
        self._timer: threading.Timer | None = None
        self._running = True

        # Start periodic flush timer
        self._schedule_flush()

        logger.debug(
            f"EventBuffer initialized (max_size={max_size}, "
            f"flush_interval={flush_interval}s)"
        )

    def add(self, item: T) -> None:
        """
        Add an item to the buffer.

        Auto-flushes if buffer reaches max_size.

        Args:
            item: Item to add to the buffer
        """
        with self._lock:
            self._buffer.append(item)

            # Auto-flush if buffer is full
            if len(self._buffer) >= self.max_size:
                logger.debug(f"Buffer full ({self.max_size}), auto-flushing")
                self._flush_unsafe()

    def flush(self) -> None:
        """
        Manually flush all buffered items.

        Thread-safe.
        """
        with self._lock:
            self._flush_unsafe()

    def _flush_unsafe(self) -> None:
        """
        Flush buffered items without acquiring lock.

        MUST be called while holding self._lock.
        """
        if not self._buffer:
            return

        # Extract all items
        items = list(self._buffer)
        self._buffer.clear()

        logger.debug(f"Flushing {len(items)} items")

        # Call flush handler (outside lock to avoid deadlock)
        # NOTE: We're still holding the lock here, which is intentional
        # to prevent concurrent flushes
        try:
            self.on_flush(items)
        except Exception as e:
            logger.error(f"Error in flush handler: {e}", exc_info=True)

    def _schedule_flush(self) -> None:
        """
        Schedule the next periodic flush.

        Uses threading.Timer for periodic background flushing.
        """
        if not self._running:
            return

        # Cancel existing timer if any
        if self._timer:
            self._timer.cancel()

        # Schedule next flush
        self._timer = threading.Timer(self.flush_interval, self._periodic_flush)
        self._timer.daemon = True
        self._timer.start()

    def _periodic_flush(self) -> None:
        """
        Periodic flush callback (runs in timer thread).
        """
        if not self._running:
            return

        # Flush if buffer has items
        with self._lock:
            if self._buffer:
                logger.debug("Periodic flush triggered")
                self._flush_unsafe()

        # Schedule next flush
        self._schedule_flush()

    def stop(self) -> None:
        """
        Stop the buffer and flush remaining items.

        Call this before shutting down to ensure no events are lost.
        """
        logger.debug("Stopping EventBuffer")

        self._running = False

        # Cancel timer
        if self._timer:
            self._timer.cancel()
            self._timer = None

        # Final flush
        self.flush()

        logger.debug("EventBuffer stopped")

    def __len__(self) -> int:
        """Return current buffer size."""
        with self._lock:
            return len(self._buffer)

    def __enter__(self):
        """Context manager support."""
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        """Context manager cleanup."""
        self.stop()
