# Rule Installation and E2E Testing Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Create Python gRPC client wrapper for InstallRules RPC, install L4 test rules, and validate full E2E enforcement flow with ALLOW/BLOCK scenarios.

**Architecture:** Wrap the existing Data Plane gRPC InstallRules service with a Python client (`RuleClient`), install ToolWhitelist and ToolParamConstraint rules to the in-memory Bridge storage, and validate that the SDK enforcement flow correctly allows/blocks tool calls based on semantic similarity to installed rules.

**Tech Stack:** Python 3.14, grpcio, protobuf, LangGraph, pytest

---

## Background

**Current State (Session 27 Complete):**
- ✅ Data Plane gRPC server running (tonic 0.12, port 50051)
- ✅ Management Plane HTTP server running (FastAPI, port 8000)
- ✅ Python gRPC client stubs generated (`rule_installation_pb2.py`, `rule_installation_pb2_grpc.py`)
- ✅ SDK DataPlaneClient wrapper for Enforce RPC (gRPC → Management Plane → Semantic Sandbox)
- ✅ SecureGraphProxy enforcement proxy (intercepts tool calls, fail-closed behavior)
- ✅ Full E2E validation: SDK → Data Plane → Management Plane → Sandbox (empty rule set = BLOCK)

**Gap:**
- No Python wrapper for InstallRules RPC (proto exists, gRPC service implemented in Rust)
- No rules installed → All tool calls blocked (fail-closed)
- Demo can't test ALLOW scenarios without rules

**Proto Reference:** `proto/rule_installation.proto`
```protobuf
service DataPlane {
  rpc InstallRules(InstallRulesRequest) returns (InstallRulesResponse);
  rpc RemoveAgentRules(RemoveAgentRulesRequest) returns (RemoveAgentRulesResponse);
  rpc GetRuleStats(GetRuleStatsRequest) returns (GetRuleStatsResponse);
  rpc Enforce(EnforceRequest) returns (EnforceResponse);
}
```

---

## Task 1: Create RuleClient Wrapper

**Files:**
- Create: `tupl_sdk/python/tupl/rule_client.py`
- Modify: `tupl_sdk/python/tupl/__init__.py` (add export)
- Test: `tupl_sdk/python/tests/test_rule_client.py`

**Step 1: Write the failing test**

Create: `tupl_sdk/python/tests/test_rule_client.py`

```python
"""Tests for RuleClient gRPC wrapper."""
import pytest
from tupl.rule_client import RuleClient, RuleClientError


def test_rule_client_instantiation():
    """Test that RuleClient can be instantiated with default config."""
    client = RuleClient(url="localhost:50051", insecure=True)
    assert client is not None
    assert hasattr(client, 'install_rules')
    assert hasattr(client, 'remove_agent_rules')
    assert hasattr(client, 'get_stats')


def test_install_rules_basic_structure():
    """Test that install_rules accepts expected parameters."""
    client = RuleClient(url="localhost:50051", insecure=True)

    # Should accept list of rule dicts
    rules = [{
        "rule_id": "test-001",
        "family_id": "ToolWhitelist",
        "layer": "L4",
        "agent_id": "test-agent",
        "priority": 100,
        "enabled": True,
        "created_at_ms": 1700000000000,
        "params": {}
    }]

    # Method should exist (will fail until implemented)
    try:
        result = client.install_rules(rules)
    except RuleClientError:
        pass  # Expected - gRPC service might not be running
```

**Step 2: Run test to verify it fails**

Run:
```bash
cd tupl_sdk/python
uv run pytest tests/test_rule_client.py::test_rule_client_instantiation -v
```

Expected: `ModuleNotFoundError: No module named 'tupl.rule_client'`

**Step 3: Write minimal implementation**

Create: `tupl_sdk/python/tupl/rule_client.py`

```python
"""
RuleClient - Python wrapper for Data Plane gRPC rule management.

Provides high-level interface for InstallRules, RemoveAgentRules, and GetRuleStats RPCs.
"""
import logging
from typing import List, Dict, Any, Optional
import grpc

from .generated import rule_installation_pb2
from .generated import rule_installation_pb2_grpc

logger = logging.getLogger(__name__)


class RuleClientError(Exception):
    """Raised when rule management operations fail."""
    pass


class RuleClient:
    """
    Python client for Data Plane rule management via gRPC.

    Example:
        client = RuleClient(url="localhost:50051", insecure=True)

        rules = [{
            "rule_id": "tw-001",
            "family_id": "ToolWhitelist",
            "layer": "L4",
            "agent_id": "demo-agent",
            "priority": 100,
            "enabled": True,
            "created_at_ms": 1700000000000,
            "params": {
                "allowed_tool_ids": {"string_list": {"values": ["search", "read"]}}
            }
        }]

        result = client.install_rules(rules)
        print(f"Installed {result['rules_installed']} rules")
    """

    def __init__(
        self,
        url: str = "localhost:50051",
        timeout: float = 10.0,
        insecure: bool = True,
        max_retries: int = 3
    ):
        """
        Initialize RuleClient.

        Args:
            url: Data Plane gRPC URL (default: localhost:50051)
            timeout: Request timeout in seconds (default: 10.0)
            insecure: Use insecure channel (default: True for development)
            max_retries: Maximum retry attempts (default: 3)
        """
        self.url = url
        self.timeout = timeout
        self.max_retries = max_retries

        # Create gRPC channel
        if insecure:
            self.channel = grpc.insecure_channel(url)
        else:
            credentials = grpc.ssl_channel_credentials()
            self.channel = grpc.secure_channel(url, credentials)

        # Create stub
        self.stub = rule_installation_pb2_grpc.DataPlaneStub(self.channel)

        logger.info(f"RuleClient initialized: url={url}, insecure={insecure}")

    def _dict_to_param_value(self, value: Any) -> rule_installation_pb2.ParamValue:
        """
        Convert Python dict value to protobuf ParamValue.

        Supports:
        - string_value: str
        - int_value: int
        - float_value: float
        - bool_value: bool
        - string_list: {"values": [...]}
        - int_list: {"values": [...]}
        """
        param_value = rule_installation_pb2.ParamValue()

        if isinstance(value, str):
            param_value.string_value = value
        elif isinstance(value, int):
            param_value.int_value = value
        elif isinstance(value, float):
            param_value.float_value = value
        elif isinstance(value, bool):
            param_value.bool_value = value
        elif isinstance(value, dict):
            # Handle nested structures (string_list, int_list, etc.)
            if "string_list" in value:
                param_value.string_list.values.extend(value["string_list"]["values"])
            elif "int_list" in value:
                param_value.int_list.values.extend(value["int_list"]["values"])
            elif "string_value" in value:
                param_value.string_value = value["string_value"]
            elif "int_value" in value:
                param_value.int_value = value["int_value"]
            elif "float_value" in value:
                param_value.float_value = value["float_value"]
            elif "bool_value" in value:
                param_value.bool_value = value["bool_value"]
            else:
                raise ValueError(f"Unsupported param value structure: {value}")
        else:
            raise ValueError(f"Unsupported param value type: {type(value)}")

        return param_value

    def install_rules(self, rules: List[Dict[str, Any]]) -> Dict[str, Any]:
        """
        Install rules to Data Plane via InstallRules RPC.

        Args:
            rules: List of rule dicts with structure:
                {
                    "rule_id": str,
                    "family_id": str,
                    "layer": str,
                    "agent_id": str,
                    "priority": int,
                    "enabled": bool,
                    "created_at_ms": int,
                    "params": Dict[str, Any]
                }

        Returns:
            Dict with keys: rules_installed, failures

        Raises:
            RuleClientError: If gRPC call fails
        """
        try:
            # Convert Python dicts to protobuf messages
            proto_rules = []
            for rule in rules:
                proto_rule = rule_installation_pb2.Rule(
                    rule_id=rule["rule_id"],
                    family_id=rule["family_id"],
                    layer=rule["layer"],
                    agent_id=rule["agent_id"],
                    priority=rule["priority"],
                    enabled=rule["enabled"],
                    created_at_ms=rule["created_at_ms"]
                )

                # Convert params dict to protobuf map
                for key, value in rule.get("params", {}).items():
                    proto_rule.params[key].CopyFrom(self._dict_to_param_value(value))

                proto_rules.append(proto_rule)

            # Create request
            request = rule_installation_pb2.InstallRulesRequest(rules=proto_rules)

            # Call gRPC service
            logger.debug(f"Installing {len(rules)} rules via gRPC")
            response = self.stub.InstallRules(request, timeout=self.timeout)

            logger.info(
                f"InstallRules response: installed={response.rules_installed}, "
                f"failures={len(response.failures)}"
            )

            return {
                "rules_installed": response.rules_installed,
                "failures": [
                    {"rule_id": f.rule_id, "reason": f.reason}
                    for f in response.failures
                ]
            }

        except grpc.RpcError as e:
            logger.error(f"gRPC error in install_rules: {e.code()} - {e.details()}")
            raise RuleClientError(
                f"Failed to install rules: {e.code()} - {e.details()}"
            )
        except Exception as e:
            logger.error(f"Unexpected error in install_rules: {e}")
            raise RuleClientError(f"Failed to install rules: {str(e)}")

    def remove_agent_rules(self, agent_id: str) -> Dict[str, Any]:
        """
        Remove all rules for an agent via RemoveAgentRules RPC.

        Args:
            agent_id: Agent identifier

        Returns:
            Dict with keys: rules_removed

        Raises:
            RuleClientError: If gRPC call fails
        """
        try:
            request = rule_installation_pb2.RemoveAgentRulesRequest(agent_id=agent_id)
            response = self.stub.RemoveAgentRules(request, timeout=self.timeout)

            logger.info(f"Removed {response.rules_removed} rules for agent {agent_id}")

            return {"rules_removed": response.rules_removed}

        except grpc.RpcError as e:
            logger.error(f"gRPC error in remove_agent_rules: {e.code()} - {e.details()}")
            raise RuleClientError(
                f"Failed to remove rules: {e.code()} - {e.details()}"
            )

    def get_stats(self) -> Dict[str, Any]:
        """
        Get Bridge statistics via GetRuleStats RPC.

        Returns:
            Dict with keys: total_tables, total_rules, global_rules, scoped_rules

        Raises:
            RuleClientError: If gRPC call fails
        """
        try:
            request = rule_installation_pb2.GetRuleStatsRequest()
            response = self.stub.GetRuleStats(request, timeout=self.timeout)

            logger.debug(
                f"Bridge stats: tables={response.total_tables}, "
                f"rules={response.total_rules}"
            )

            return {
                "total_tables": response.total_tables,
                "total_rules": response.total_rules,
                "global_rules": response.global_rules,
                "scoped_rules": response.scoped_rules
            }

        except grpc.RpcError as e:
            logger.error(f"gRPC error in get_stats: {e.code()} - {e.details()}")
            raise RuleClientError(
                f"Failed to get stats: {e.code()} - {e.details()}"
            )

    def close(self):
        """Close gRPC channel."""
        self.channel.close()
        logger.debug("RuleClient channel closed")

    def __enter__(self):
        """Context manager entry."""
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        """Context manager exit."""
        self.close()
```

**Step 4: Run test to verify it passes**

Run:
```bash
cd tupl_sdk/python
uv run pytest tests/test_rule_client.py::test_rule_client_instantiation -v
```

Expected: PASS (client instantiates successfully)

**Step 5: Export RuleClient from SDK**

Modify: `tupl_sdk/python/tupl/__init__.py`

Add to imports section:
```python
# Rule management client (v1.3)
from .rule_client import RuleClient, RuleClientError
```

Add to `__all__`:
```python
__all__ = [
    # ... existing exports ...
    "DataPlaneClient",
    "DataPlaneError",
    "RuleClient",        # NEW
    "RuleClientError",   # NEW
]
```

**Step 6: Commit**

```bash
git add tupl_sdk/python/tupl/rule_client.py
git add tupl_sdk/python/tupl/__init__.py
git add tupl_sdk/python/tests/test_rule_client.py
git commit -m "feat: add RuleClient wrapper for InstallRules gRPC"
```

---

## Task 2: Create E2E Rule Installation Test

**Files:**
- Create: `tupl_sdk/python/tests/test_rule_installation_e2e.py`

**Step 1: Write the E2E test**

Create: `tupl_sdk/python/tests/test_rule_installation_e2e.py`

```python
"""
End-to-end test for rule installation and enforcement.

Requirements:
- Data Plane gRPC server running on localhost:50051
- Management Plane HTTP server running on localhost:8000
"""
import pytest
import time
from tupl.rule_client import RuleClient, RuleClientError


@pytest.mark.integration
def test_install_l4_toolwhitelist_rule():
    """Test installing a ToolWhitelist rule via gRPC."""

    client = RuleClient(url="localhost:50051", insecure=True)

    # Define L4 ToolWhitelist rule
    rules = [{
        "rule_id": "test-tw-001",
        "family_id": "ToolWhitelist",
        "layer": "L4",
        "agent_id": "test-agent",
        "priority": 100,
        "enabled": True,
        "created_at_ms": int(time.time() * 1000),
        "params": {
            "allowed_tool_ids": {
                "string_list": {
                    "values": ["search_database", "update_record"]
                }
            },
            "description": {
                "string_value": "Allow safe database operations"
            }
        }
    }]

    # Install rules
    result = client.install_rules(rules)

    # Verify installation
    assert result["rules_installed"] == 1
    assert len(result["failures"]) == 0

    # Verify via stats
    stats = client.get_stats()
    assert stats["total_rules"] >= 1

    # Cleanup
    cleanup = client.remove_agent_rules("test-agent")
    assert cleanup["rules_removed"] >= 1


@pytest.mark.integration
def test_install_l4_toolparamconstraint_rule():
    """Test installing a ToolParamConstraint rule via gRPC."""

    client = RuleClient(url="localhost:50051", insecure=True)

    # Define L4 ToolParamConstraint rule
    rules = [{
        "rule_id": "test-tpc-001",
        "family_id": "ToolParamConstraint",
        "layer": "L4",
        "agent_id": "test-agent",
        "priority": 90,
        "enabled": True,
        "created_at_ms": int(time.time() * 1000),
        "params": {
            "param_name": {
                "string_value": "query"
            },
            "param_type": {
                "string_value": "string"
            },
            "max_len": {
                "int_value": 100
            },
            "description": {
                "string_value": "Query must be under 100 chars"
            }
        }
    }]

    # Install rules
    result = client.install_rules(rules)

    # Verify installation
    assert result["rules_installed"] == 1
    assert len(result["failures"]) == 0

    # Cleanup
    cleanup = client.remove_agent_rules("test-agent")
    assert cleanup["rules_removed"] >= 1


@pytest.mark.integration
def test_get_bridge_stats():
    """Test retrieving Bridge statistics."""

    client = RuleClient(url="localhost:50051", insecure=True)

    stats = client.get_stats()

    # Verify stats structure
    assert "total_tables" in stats
    assert "total_rules" in stats
    assert "global_rules" in stats
    assert "scoped_rules" in stats

    # Should have 14 family tables
    assert stats["total_tables"] == 14
```

**Step 2: Run tests to verify they pass (requires servers running)**

Run:
```bash
# Start servers in background (if not already running)
cd tupl_data_plane/tupl_dp/bridge && cargo run --bin bridge-server &
cd management-plane && uv run uvicorn app.main:app --reload &

# Wait for startup
sleep 3

# Run tests
cd tupl_sdk/python
uv run pytest tests/test_rule_installation_e2e.py -v -m integration
```

Expected: All tests PASS

**Step 3: Commit**

```bash
git add tupl_sdk/python/tests/test_rule_installation_e2e.py
git commit -m "test: add E2E tests for rule installation via gRPC"
```

---

## Task 3: Update Demo with Rule Installation

**Files:**
- Modify: `examples/langgraph_demo/demo_layer_enforcement.py`

**Step 1: Add rule installation to demo**

Modify: `examples/langgraph_demo/demo_layer_enforcement.py`

Replace the `install_l4_rules()` function:

```python
def install_l4_rules():
    """Install sample L4 rules to Data Plane via gRPC."""
    from tupl.rule_client import RuleClient, RuleClientError

    print("=" * 80)
    print("Installing L4 Rules to Data Plane")
    print("=" * 80)

    client = RuleClient(url=os.getenv("TUPL_DATA_PLANE_URL", "localhost:50051"), insecure=True)

    try:
        # Install rules via gRPC
        result = client.install_rules(SAMPLE_L4_RULES)

        print(f"\n✅ Successfully installed {result['rules_installed']} rules")

        if result['failures']:
            print(f"\n⚠️  {len(result['failures'])} failures:")
            for failure in result['failures']:
                print(f"  - {failure['rule_id']}: {failure['reason']}")

        # Show stats
        stats = client.get_stats()
        print(f"\nBridge Statistics:")
        print(f"  - Total tables: {stats['total_tables']}")
        print(f"  - Total rules: {stats['total_rules']}")
        print(f"  - Global rules: {stats['global_rules']}")
        print(f"  - Scoped rules: {stats['scoped_rules']}")
        print()

    except RuleClientError as e:
        print(f"\n❌ Failed to install rules: {e}")
        print("   Make sure Data Plane gRPC server is running on localhost:50051")
        sys.exit(1)
```

**Step 2: Add cleanup at demo end**

Add cleanup function after `run_layer_enforcement_demo()`:

```python
def cleanup_rules():
    """Remove demo rules from Data Plane."""
    from tupl.rule_client import RuleClient

    print("\n" + "=" * 80)
    print("Cleaning Up Rules")
    print("=" * 80)

    client = RuleClient(url=os.getenv("TUPL_DATA_PLANE_URL", "localhost:50051"), insecure=True)

    try:
        result = client.remove_agent_rules("demo-agent")
        print(f"✅ Removed {result['rules_removed']} rules for demo-agent")
    except Exception as e:
        print(f"⚠️  Cleanup warning: {e}")
```

Modify `main()` to call cleanup:

```python
def main():
    """Main entry point."""

    # Validate environment
    if not os.getenv("GOOGLE_API_KEY"):
        print("ERROR: GOOGLE_API_KEY not set in environment")
        print("Please copy .env.example to .env and add your API key")
        sys.exit(1)

    # Install L4 rules
    install_l4_rules()

    # Run demo
    try:
        run_layer_enforcement_demo()
    except KeyboardInterrupt:
        print("\n\nDemo interrupted by user")
    except Exception as e:
        print(f"\n\nFatal error: {e}")
        if os.getenv("DEBUG"):
            import traceback
            traceback.print_exc()
    finally:
        # Always cleanup rules
        cleanup_rules()
```

**Step 3: Update demo expected behavior message**

Replace the "Current Behavior" section in the final status output:

```python
    print("Current Behavior:")
    print("  • Rules installed → Enforcement based on semantic similarity")
    print("  • ALLOW: Tools/params matching installed rules (high similarity)")
    print("  • BLOCK: Tools/params NOT matching rules (low similarity)")
    print("  • This validates full E2E enforcement with real rules")
```

**Step 4: Run updated demo**

Run:
```bash
cd examples/langgraph_demo
TUPL_ENFORCEMENT_MODE=data_plane uv run demo_layer_enforcement.py
```

Expected output:
```
================================================================================
Installing L4 Rules to Data Plane
================================================================================

✅ Successfully installed 2 rules

Bridge Statistics:
  - Total tables: 14
  - Total rules: 2
  - Global rules: 0
  - Scoped rules: 2

================================================================================
Running Test Scenarios
================================================================================

Scenario 1: Allowed Tool (search_database)
🚀 Invoking agent...
✅ ALLOWED - Agent executed successfully
   (Tool matches ToolWhitelist rule)

Scenario 2: Blocked Tool (delete_record)
🚀 Invoking agent...
🚫 BLOCKED - Tool call 'delete_record' blocked by boundary 'demo-ops-policy'
   (Tool NOT in ToolWhitelist)

...
```

**Step 5: Commit**

```bash
git add examples/langgraph_demo/demo_layer_enforcement.py
git commit -m "feat: integrate RuleClient for L4 rule installation in demo"
```

---

## Task 4: Create Standalone Test Script

**Files:**
- Create: `examples/langgraph_demo/test_rule_installation.py`

**Step 1: Write standalone test script**

Create: `examples/langgraph_demo/test_rule_installation.py`

```python
#!/usr/bin/env python3
"""
Standalone test script for rule installation and enforcement.

Tests:
1. Install L4 ToolWhitelist rule (allow: search_database, update_record)
2. Install L4 ToolParamConstraint rule (query max_len: 100)
3. Invoke agent with allowed tool → Should ALLOW
4. Invoke agent with blocked tool → Should BLOCK
5. Cleanup rules

Requirements:
- Data Plane gRPC server running on localhost:50051
- Management Plane HTTP server running on localhost:8000
- GOOGLE_API_KEY environment variable set
"""
import os
import sys
import time
from dotenv import load_dotenv

# Load environment
load_dotenv()

# Validate environment
if not os.getenv("GOOGLE_API_KEY"):
    print("ERROR: GOOGLE_API_KEY not set")
    sys.exit(1)

from langchain_google_genai import ChatGoogleGenerativeAI
from langgraph.prebuilt import create_react_agent
from tupl.agent import enforcement_agent
from tupl.rule_client import RuleClient

# Import demo tools
from tools import search_database, update_record, delete_record


def main():
    print("=" * 80)
    print("Rule Installation and Enforcement Test")
    print("=" * 80)
    print()

    # Step 1: Install rules
    print("Step 1: Installing L4 rules via gRPC...")
    client = RuleClient(url="localhost:50051", insecure=True)

    rules = [
        {
            "rule_id": "test-tw-001",
            "family_id": "ToolWhitelist",
            "layer": "L4",
            "agent_id": "test-agent",
            "priority": 100,
            "enabled": True,
            "created_at_ms": int(time.time() * 1000),
            "params": {
                "allowed_tool_ids": {
                    "string_list": {
                        "values": ["search_database", "update_record"]
                    }
                },
                "description": {
                    "string_value": "Allow safe database operations"
                }
            }
        },
        {
            "rule_id": "test-tpc-001",
            "family_id": "ToolParamConstraint",
            "layer": "L4",
            "agent_id": "test-agent",
            "priority": 90,
            "enabled": True,
            "created_at_ms": int(time.time() * 1000),
            "params": {
                "param_name": {
                    "string_value": "query"
                },
                "max_len": {
                    "int_value": 100
                },
                "description": {
                    "string_value": "Query max 100 chars"
                }
            }
        }
    ]

    result = client.install_rules(rules)
    print(f"✅ Installed {result['rules_installed']} rules")
    print()

    # Step 2: Build agent with enforcement
    print("Step 2: Building agent with enforcement...")
    model = ChatGoogleGenerativeAI(
        model="gemini-2.0-flash-exp",
        google_api_key=os.getenv("GOOGLE_API_KEY"),
        temperature=0
    )

    tools = [search_database, update_record, delete_record]
    agent = create_react_agent(model, tools)
    secure_agent = enforcement_agent(
        graph=agent,
        boundary_id="test-policy",
        tenant_id="test-agent"
    )
    print("✅ Agent ready")
    print()

    # Step 3: Test ALLOW scenario
    print("Step 3: Testing ALLOW scenario (search_database with short query)...")
    try:
        result = secure_agent.invoke({
            "messages": [{"role": "user", "content": "Search database for user@example.com"}]
        })
        print("✅ ALLOWED - Tool call executed successfully")
    except PermissionError as e:
        print(f"❌ UNEXPECTED BLOCK: {e}")
    print()

    # Step 4: Test BLOCK scenario (tool not in whitelist)
    print("Step 4: Testing BLOCK scenario (delete_record not in whitelist)...")
    try:
        result = secure_agent.invoke({
            "messages": [{"role": "user", "content": "Delete record ID 12345"}]
        })
        print("❌ UNEXPECTED ALLOW - Tool should have been blocked")
    except PermissionError as e:
        print(f"✅ BLOCKED - {str(e)[:100]}...")
    print()

    # Step 5: Cleanup
    print("Step 5: Cleaning up rules...")
    cleanup = client.remove_agent_rules("test-agent")
    print(f"✅ Removed {cleanup['rules_removed']} rules")
    print()

    print("=" * 80)
    print("Test Complete")
    print("=" * 80)


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print("\n\nTest interrupted")
        sys.exit(0)
    except Exception as e:
        print(f"\n\nFatal error: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)
```

**Step 2: Make script executable**

Run:
```bash
chmod +x examples/langgraph_demo/test_rule_installation.py
```

**Step 3: Run script to verify**

Run:
```bash
cd examples/langgraph_demo
uv run python test_rule_installation.py
```

Expected: Tests pass with ALLOW and BLOCK scenarios working correctly

**Step 4: Commit**

```bash
git add examples/langgraph_demo/test_rule_installation.py
git commit -m "test: add standalone rule installation test script"
```

---

## Task 5: Add Documentation

**Files:**
- Create: `docs/rule-installation-guide.md`

**Step 1: Write usage documentation**

Create: `docs/rule-installation-guide.md`

```markdown
# Rule Installation Guide

## Overview

This guide explains how to install L4 ToolGateway rules to the Data Plane and test enforcement scenarios.

## Prerequisites

1. **Data Plane gRPC server** running on `localhost:50051`
   ```bash
   cd tupl_data_plane/tupl_dp/bridge
   cargo run --bin bridge-server
   ```

2. **Management Plane HTTP server** running on `localhost:8000`
   ```bash
   cd management-plane
   uv run uvicorn app.main:app --reload
   ```

3. **Python SDK** installed with gRPC dependencies
   ```bash
   cd tupl_sdk/python
   uv sync
   ```

## Rule Structure

### L4 ToolWhitelist Rule

```python
{
    "rule_id": "tw-001",
    "family_id": "ToolWhitelist",
    "layer": "L4",
    "agent_id": "my-agent",
    "priority": 100,
    "enabled": True,
    "created_at_ms": 1700000000000,
    "params": {
        "allowed_tool_ids": {
            "string_list": {
                "values": ["search_database", "update_record"]
            }
        },
        "description": {
            "string_value": "Allow safe database operations"
        }
    }
}
```

### L4 ToolParamConstraint Rule

```python
{
    "rule_id": "tpc-001",
    "family_id": "ToolParamConstraint",
    "layer": "L4",
    "agent_id": "my-agent",
    "priority": 90,
    "enabled": True,
    "created_at_ms": 1700000000000,
    "params": {
        "param_name": {
            "string_value": "query"
        },
        "param_type": {
            "string_value": "string"
        },
        "max_len": {
            "int_value": 100
        },
        "description": {
            "string_value": "Query must be under 100 characters"
        }
    }
}
```

## Installation API

### Basic Usage

```python
from tupl.rule_client import RuleClient

# Create client
client = RuleClient(url="localhost:50051", insecure=True)

# Install rules
rules = [...]  # See rule structures above
result = client.install_rules(rules)

print(f"Installed: {result['rules_installed']}")
print(f"Failures: {len(result['failures'])}")

# Get stats
stats = client.get_stats()
print(f"Total rules: {stats['total_rules']}")

# Remove rules for an agent
cleanup = client.remove_agent_rules("my-agent")
print(f"Removed: {cleanup['rules_removed']}")
```

### Context Manager Pattern

```python
from tupl.rule_client import RuleClient

with RuleClient(url="localhost:50051", insecure=True) as client:
    result = client.install_rules(rules)
    # Channel closes automatically
```

## Testing Enforcement

### With LangGraph Agent

```python
from langchain_google_genai import ChatGoogleGenerativeAI
from langgraph.prebuilt import create_react_agent
from tupl.agent import enforcement_agent
from tupl.rule_client import RuleClient

# 1. Install rules
client = RuleClient()
client.install_rules(rules)

# 2. Build agent with enforcement
model = ChatGoogleGenerativeAI(model="gemini-2.0-flash-exp")
agent = create_react_agent(model, tools)
secure_agent = enforcement_agent(
    graph=agent,
    boundary_id="my-policy",
    tenant_id="my-agent"
)

# 3. Test ALLOW scenario
result = secure_agent.invoke({
    "messages": [{"role": "user", "content": "Search for user@example.com"}]
})

# 4. Test BLOCK scenario (raises PermissionError)
try:
    result = secure_agent.invoke({
        "messages": [{"role": "user", "content": "Delete record 12345"}]
    })
except PermissionError as e:
    print(f"Blocked: {e}")
```

## Running Tests

### Unit Tests
```bash
cd tupl_sdk/python
uv run pytest tests/test_rule_client.py -v
```

### Integration Tests
```bash
# Requires servers running
uv run pytest tests/test_rule_installation_e2e.py -v -m integration
```

### Demo Script
```bash
cd examples/langgraph_demo
TUPL_ENFORCEMENT_MODE=data_plane uv run demo_layer_enforcement.py
```

### Standalone Test
```bash
cd examples/langgraph_demo
uv run python test_rule_installation.py
```

## Troubleshooting

### gRPC Connection Errors

**Error:** `Failed to install rules: UNAVAILABLE - failed to connect to all addresses`

**Solution:** Ensure Data Plane server is running:
```bash
cd tupl_data_plane/tupl_dp/bridge
cargo run --bin bridge-server
# Should show "Starting Data Plane gRPC server on 0.0.0.0:50051"
```

### Management Plane Timeout

**Error:** `Intent encoding failed: operation timed out`

**Solution:** Ensure Management Plane is running:
```bash
cd management-plane
uv run uvicorn app.main:app --reload
# Should show "Application startup complete"
```

### All Tool Calls Blocked

**Issue:** All tool calls are blocked even with rules installed

**Diagnosis:**
1. Check rules were installed: `client.get_stats()` should show `total_rules > 0`
2. Verify agent_id matches: Rule `agent_id` must match `tenant_id` in `enforcement_agent()`
3. Check rule enabled: `"enabled": True` in rule definition

## Next Steps

- Install rules for other layers (L0, L1, L2, L3, L5, L6)
- Implement database persistence for rules
- Add rule versioning and audit logs
```

**Step 2: Commit**

```bash
git add docs/rule-installation-guide.md
git commit -m "docs: add rule installation and testing guide"
```

---

## Verification Checklist

After completing all tasks, verify:

- [ ] `RuleClient` can be imported: `from tupl import RuleClient`
- [ ] Unit tests pass: `pytest tests/test_rule_client.py -v`
- [ ] Integration tests pass: `pytest tests/test_rule_installation_e2e.py -v -m integration`
- [ ] Demo installs rules successfully
- [ ] Demo shows ALLOW scenarios (with rules installed)
- [ ] Demo shows BLOCK scenarios (tools not in whitelist)
- [ ] Standalone test script runs successfully
- [ ] Documentation is clear and accurate

---

## Expected Outcomes

**Before Implementation:**
- Empty rule set → All tool calls BLOCKED (fail-closed)
- Demo can't test ALLOW scenarios

**After Implementation:**
- Rules installed via `RuleClient.install_rules()`
- Demo shows ALLOW for whitelisted tools
- Demo shows BLOCK for non-whitelisted tools
- Full E2E flow validated: SDK → Data Plane → Management Plane → Semantic Sandbox → ALLOW/BLOCK decision based on semantic similarity to installed rules

---

## Files Summary

**Created:**
- `tupl_sdk/python/tupl/rule_client.py` (350 lines)
- `tupl_sdk/python/tests/test_rule_client.py` (60 lines)
- `tupl_sdk/python/tests/test_rule_installation_e2e.py` (120 lines)
- `examples/langgraph_demo/test_rule_installation.py` (180 lines)
- `docs/rule-installation-guide.md` (250 lines)

**Modified:**
- `tupl_sdk/python/tupl/__init__.py` (+2 exports)
- `examples/langgraph_demo/demo_layer_enforcement.py` (~50 line changes)

**Total:** ~960 lines of new code + documentation
