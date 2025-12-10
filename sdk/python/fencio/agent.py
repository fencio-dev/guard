"""
Fencio agent module - re-exports from tupl.agent for compatibility.

This allows users to import via:
    from fencio.agent import enforcement_agent
"""

# Re-export everything from tupl.agent
from tupl.agent import *

__all__ = ["enforcement_agent"]
