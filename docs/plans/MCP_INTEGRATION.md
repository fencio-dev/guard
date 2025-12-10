# Tupl SDK - MCP Integration Guide

This guide explains how to use the Tupl SDK with the Model Context Protocol (MCP) for agent rule configuration and enforcement.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│  User's Python Code                                      │
│  ┌───────────────────────────────────────────────────┐  │
│  │  TuplAgentWrapper (MCP Client)                    │  │
│  │  - Configure agent rules                          │  │
│  │  - Wrap agents with enforcement                   │  │
│  │  - Access telemetry                               │  │
│  └───────────────┬───────────────────────────────────┘  │
└──────────────────┼──────────────────────────────────────┘
                   │ MCP Protocol (stdio)
                   ▼
┌─────────────────────────────────────────────────────────┐
│  MCP Tupl Server                                         │
│  ┌───────────────────────────────────────────────────┐  │
│  │  MCP Tools:                                        │  │
│  │  - tupl_configure_agent_rules                     │  │
│  │  - tupl_list_rule_families                        │  │
│  │  - tupl_wrap_agent                                │  │
│  │  - tupl_test_intent                               │  │
│  │  - tupl_get_telemetry                             │  │
│  └───────────────┬───────────────────────────────────┘  │
└──────────────────┼──────────────────────────────────────┘
                   │ HTTP
         ┌─────────┴──────────┐
         ▼                    ▼
┌─────────────────┐  ┌─────────────────┐
│ Policy Control  │  │ Management      │
│ Plane           │  │ Plane           │
│ (Rule Config)   │  │ (Enforcement)   │
└─────────────────┘  └─────────────────┘
```

## Components

### 1. MCP Tupl Server (`mcp-tupl-server/`)

Exposes Tupl SDK capabilities via MCP protocol.

**Location:** `/mcp-tupl-server/`

**Features:**
- 9 MCP tools for rule configuration and enforcement
- HTTP clients for Control Plane and Management Plane
- Rule family catalog with L0-L6 schemas
- Automatic rule compilation

### 2. TuplAgentWrapper (`tupl_sdk/python/tupl/mcp_client.py`)

Python client wrapper that simplifies MCP interaction.

**Location:** `/tupl_sdk/python/tupl/mcp_client.py`

**Features:**
- Pythonic async API
- Automatic MCP session management
- Rule configuration helpers
- Agent enforcement wrapping

## Quick Start

### 1. Installation

```bash
# Install MCP server
cd mcp-tupl-server
uv pip install -e .

# Install Tupl SDK with MCP support
cd ../tupl_sdk/python
uv pip install -e ".[mcp]"
```

### 2. Start Required Services

```bash
# Terminal 1: Start Policy Control Plane
cd policy_control_plane
uv run python server.py

# Terminal 2: Start Management Plane (if using enforcement)
cd management-plane
./run.sh
```

### 3. Basic Usage

```python
import asyncio
from tupl.mcp_client import TuplAgentWrapper

async def main():
    # Initialize wrapper
    tupl = TuplAgentWrapper(tenant_id="my-team")

    # Configure agent rules
    config = await tupl.configure_agent(
        agent_id="cs-agent",
        owner="ops-team",
        rule_families={
            "tool_whitelist": {
                "enabled": True,
                "params": {
                    "allowed_tool_ids": ["search", "update"],
                    "action": "DENY"
                }
            },
            "output_pii": {
                "enabled": True,
                "params": {
                    "action": "REDACT",
                    "pii_types": ["SSN", "CREDIT_CARD"]
                }
            }
        }
    )

    print(f"Configured {config['rule_count']} rules")

    # Wrap agent with enforcement (requires LangGraph agent)
    # secure_agent = await tupl.wrap_agent(
    #     agent=my_langgraph_agent,
    #     agent_id="cs-agent",
    #     enforcement_mode="block"
    # )

    await tupl.close()

asyncio.run(main())
```

## Rule Families

### Layer Breakdown

| Layer | Name | Rule Families |
|-------|------|---------------|
| L0 | System | `net_egress`, `sidecar_spawn` |
| L1 | Input | `input_schema`, `input_sanitize` |
| L2 | Planner | `prompt_assembly`, `prompt_length` |
| L3 | Model I/O | `model_output_scan`, `model_output_escalate` |
| L4 | Tool Gateway | `tool_whitelist`, `tool_param_constraint` |
| L5 | RAG | `rag_source`, `rag_doc_sensitivity` |
| L6 | Egress | `output_pii`, `output_audit` |

### Common Use Cases

#### Customer Service Agent

```python
rule_families = {
    "tool_whitelist": {
        "enabled": True,
        "params": {
            "allowed_tool_ids": ["search_customer", "update_email"],
            "action": "DENY"
        }
    },
    "output_pii": {
        "enabled": True,
        "params": {
            "action": "REDACT",
            "pii_types": ["SSN", "CREDIT_CARD", "EMAIL"]
        }
    },
    "output_audit": {
        "enabled": True,
        "params": {
            "emit_decision_event": True,
            "sampling_rate": 1.0
        }
    }
}
```

#### Data Pipeline

```python
rule_families = {
    "net_egress": {
        "enabled": True,
        "params": {
            "dest_domains": ["s3.amazonaws.com", "analytics.company.com"],
            "protocol": "HTTPS",
            "action": "DENY"
        }
    },
    "output_pii": {
        "enabled": True,
        "params": {
            "action": "DENY",  # Block PII in exports
            "pii_types": ["SSN", "CREDIT_CARD"]
        }
    },
    "rag_doc_sensitivity": {
        "enabled": True,
        "params": {
            "max_sensitivity_level": "internal",
            "action": "DENY"
        }
    }
}
```

## API Reference

### TuplAgentWrapper

#### `configure_agent(agent_id, owner, rule_families, description=None)`

Configure rule families for an agent.

**Returns:** Configuration with compiled rules

#### `get_agent_rules(agent_id, include_compiled=True)`

Get rule configuration for an agent.

**Returns:** Agent configuration

#### `list_rule_families(layer=None)`

List available rule families.

**Returns:** List of rule families

#### `get_rule_family_schema(family_id)`

Get detailed schema for a rule family.

**Returns:** Complete schema with parameters

#### `wrap_agent(agent, agent_id, boundary_id="all", enforcement_mode="audit")`

Wrap a LangGraph agent with enforcement.

**Returns:** SecureGraphProxy instance

#### `test_intent(action, actor_type, resource_type, **kwargs)`

Test an intent against boundaries.

**Returns:** Decision with similarities

#### `get_telemetry(limit=100, tenant_id=None)`

Retrieve enforcement telemetry.

**Returns:** Telemetry events

## MCP Tools

### Rule Configuration

- `tupl_configure_agent_rules` - Configure rule families
- `tupl_get_agent_rules` - Get agent configuration
- `tupl_list_agent_rules` - List all configurations
- `tupl_list_rule_families` - List available families
- `tupl_get_rule_family_schema` - Get family schema

### Enforcement

- `tupl_wrap_agent` - Create enforcement config
- `tupl_test_intent` - Test intents

### Telemetry

- `tupl_get_telemetry` - Get audit logs

## Examples

See `/mcp-tupl-server/examples/` for complete examples:

- `basic_agent_config.py` - Configure rules and explore schemas
- `agent_with_enforcement.py` - Full workflow with enforcement

## Environment Variables

```bash
# MCP Server
export CONTROL_PLANE_URL=http://localhost:8000
export MANAGEMENT_PLANE_URL=http://localhost:8000

# Client (optional overrides)
export TUPL_TENANT_ID=my-team
```

## Troubleshooting

### MCP Server Not Starting

```bash
# Check if ports are in use
lsof -i :8000

# Check logs
uv run mcp-tupl-server 2>&1 | tee mcp-server.log
```

### Client Connection Errors

```python
# Verify MCP server is running
tupl = TuplAgentWrapper(tenant_id="test")
await tupl._ensure_session()  # Will raise if server not available
```

### Rule Configuration Failed

```python
# Check Control Plane is running
curl http://localhost:8000/

# Verify rule family schema
schema = await tupl.get_rule_family_schema("tool_whitelist")
print(schema['params_schema'])
```

## Development

### Running Tests

```bash
# MCP Server tests
cd mcp-tupl-server
uv run pytest

# SDK tests
cd tupl_sdk/python
uv run pytest
```

### Adding New Rule Families

1. Add schema to `/mcp-tupl-server/src/mcp_tupl/schemas/rule_families.py`
2. Update models in `/policy_control_plane/models.py`
3. Update compiler in `/policy_control_plane/compiler.py`
4. Test with `tupl.get_rule_family_schema("new_family")`

## Production Deployment

### Security Considerations

1. **Authentication:** Add API keys to Control/Management Planes
2. **TLS:** Use HTTPS for all HTTP communication
3. **Access Control:** Restrict MCP server access
4. **Audit Logs:** Monitor telemetry for suspicious activity

### Scaling

- MCP server is stateless (can run multiple instances)
- Control Plane and Management Plane can be scaled independently
- Use load balancer for HTTP endpoints

## License

See root project LICENSE file.
