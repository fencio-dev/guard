"""
Fencio SDK - Security enforcement for LangGraph agents.

This package provides a thin compatibility layer that re-exports all symbols
from the tupl package, allowing users to:
  - Install via: pip install fencio
  - Import via: from fencio.agent import enforcement_agent
  - Or:          from fencio import TuplClient, IntentEvent

The underlying implementation remains in the tupl package for backward compatibility.
"""

# Re-export all public symbols from tupl
from tupl import (
    TuplClient,
    IntentEvent,
    Actor,
    Resource,
    Data,
    Risk,
    RateLimitContext,
    ComparisonResult,
)

__version__ = "1.2.5"

__all__ = [
    # Core client
    "TuplClient",
    # Event types
    "IntentEvent",
    "Actor",
    "Resource",
    "Data",
    "Risk",
    "RateLimitContext",
    "ComparisonResult",
]
