"""
Tupl SDK - Python client library for Semantic Security.

Capture and send IntentEvents to the Management Plane for policy enforcement.

Example usage (v1.2):
    from tupl import TuplClient, IntentEvent, Actor, Resource, Data, Risk

    # Create client
    client = TuplClient(endpoint="http://localhost:8000")

    # Create intent event (v1.2 with agent actor type)
    event = IntentEvent(
        id="evt-001",
        schemaVersion="v1.2",
        tenantId="tenant-123",
        timestamp=1699564800.0,
        actor=Actor(id="agent-123", type="agent"),
        action="read",
        resource=Resource(type="database", name="users_db", location="cloud"),
        data=Data(sensitivity=["internal"], pii=False, volume="single"),
        risk=Risk(authn="required")
    )

    # Send intent
    result = client.capture(event)
    if result:
        print(f"Decision: {'allow' if result.decision else 'block'}")
"""

__version__ = "0.1.0"

# Core client
from .client import TuplClient, AsyncTuplClient

# Data Plane gRPC client (v1.3)
from .data_plane_client import DataPlaneClient, DataPlaneError

# Rule management client (v1.3)
from .rule_client import RuleClient, RuleClientError

# Type definitions
from .types import (
    IntentEvent,
    Actor,
    Resource,
    Data,
    Risk,
    RateLimitContext,
    ComparisonResult,
    VALID_ACTIONS,
    VALID_ACTOR_TYPES,
    VALID_RESOURCE_TYPES,
    VALID_RESOURCE_LOCATIONS,
    VALID_DATA_SENSITIVITY,
    VALID_DATA_VOLUMES,
    VALID_AUTHN_LEVELS,
)

from .vocabulary import VocabularyRegistry

# Buffer (advanced usage)
from .buffer import EventBuffer

# LangGraph integration (v1.2 + v1.3)
try:
    from .agent import AgentCallback, AgentSecurityException, enforcement_agent, SecureGraphProxy
    _has_agent = True
except ImportError:
    # langchain not installed
    _has_agent = False
    AgentCallback = None
    AgentSecurityException = None
    enforcement_agent = None
    SecureGraphProxy = None

__all__ = [
    # Version
    "__version__",
    # Clients
    "TuplClient",
    "AsyncTuplClient",
    "DataPlaneClient",
    "DataPlaneError",
    "RuleClient",
    "RuleClientError",
    # Types
    "IntentEvent",
    "Actor",
    "Resource",
    "Data",
    "Risk",
    "RateLimitContext",
    "ComparisonResult",
    # Vocabularies
    "VALID_ACTIONS",
    "VALID_ACTOR_TYPES",
    "VALID_RESOURCE_TYPES",
    "VALID_RESOURCE_LOCATIONS",
    "VALID_DATA_SENSITIVITY",
    "VALID_DATA_VOLUMES",
    "VALID_AUTHN_LEVELS",
    "VocabularyRegistry",
    # Buffer (advanced)
    "EventBuffer",
    # LangGraph integration (v1.2 + v1.3)
    "AgentCallback",
    "AgentSecurityException",
    "enforcement_agent",
    "SecureGraphProxy",
]
