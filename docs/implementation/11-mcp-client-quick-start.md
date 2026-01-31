# MCP Client Quick Start

Build a minimal MCP client that connects to the Guard MCP server running at `http://13.205.39.173:3001/mcp`.

## Installation

```bash
pip install fastmcp httpx
```

## Minimal Client

Create a file `mcp_client.py`:

```python
import asyncio
import json
from fastmcp import Client
from fastmcp.client.transports import StreamableHttpTransport


async def main():
    # Connect to the MCP server
    transport = StreamableHttpTransport(url="http://13.205.39.173:3001/mcp")
    client = Client(transport)

    async with client:
        # List available tools
        tools = await client.list_tools()
        print("Available tools:")
        for tool in tools:
            print(f"  - {tool.name}: {tool.description}")

        # Call send_intent tool
        result = await client.call_tool("send_intent", {
            "action": "read",
            "resource": {
                "type": "database",
                "name": "customers",
                "location": "cloud"
            },
            "data": {
                "sensitivity": ["internal"],
                "pii": True,
                "volume": "single"
            },
            "risk": {
                "authn": "required"
            }
        })
        print(f"Result: {json.dumps(result, indent=2)}")


if __name__ == "__main__":
    asyncio.run(main())
```

Run it:

```bash
python mcp_client.py
```

## With Authentication

Add headers for API key or tenant/user authentication:

```python
headers = {
    # "Authorization": "Bearer YOUR_API_KEY"
    # OR for tenant-based auth:
    "X-Tenant-Id": "tenant-123",
    # "X-User-Id": "user-456"
}

transport = StreamableHttpTransport(
    url="http://13.205.39.173:3001/mcp",
    headers=headers
)
```

## Common Patterns

### List Tools

```python
tools = await client.list_tools()
for tool in tools:
    print(f"{tool.name}: {tool.description}")
    print(f"  Input schema: {tool.input_schema}")
```

### Call a Tool

```python
result = await client.call_tool("tool_name", {
    "param1": "value1",
    "param2": "value2"
})
```

### Handle Errors

```python
try:
    result = await client.call_tool("my_tool", args)
except Exception as e:
    print(f"Error: {e}")
```

## send_intent Payload Structure

### Required Fields

```python
{
    "action": str,          # e.g., "read", "write", "delete", "execute", "update"
    "resource": {           # Resource being accessed
        "type": str,        # Resource type (loose vocabulary, will be canonicalized)
        "name": str,        # Optional: resource name
        "location": str     # Optional: "local" or "cloud"
    },
    "data": {               # Data characteristics
        "sensitivity": [str],  # e.g., ["internal"], ["public"], ["internal", "public"]
        "pii": bool,           # Optional: whether data contains PII
        "volume": str          # Optional: "single" or "bulk"
    },
    "risk": {               # Risk context
        "authn": str        # "required" or "not_required" (strict enum)
    }
}
```

### Optional: Add Context

```python
{
    # ... required fields ...
    "context": {
        "layer": str,           # Optional: e.g., "L0", "L4"
        "tool_name": str,       # Optional: name of the calling tool
        "tool_method": str,     # Optional: method being called
        "tool_params": dict     # Optional: tool parameters
    }
}
```

### Response Format

```python
{
    "decision": str,                    # "ALLOW" or "DENY"
    "request_id": str,                  # Unique request identifier
    "rationale": str,                   # Human-readable explanation
    "enforcement_latency_ms": float,    # Latency in milliseconds
    "metadata": dict                    # Additional metadata
}
```

### Example: Minimal Payload

```python
response = await client.call_tool("send_intent", {
    "action": "read",
    "resource": {"type": "database", "name": "customers", "location": "cloud"},
    "data": {"sensitivity": ["internal"], "pii": True, "volume": "single"},
    "risk": {"authn": "required"}
})

print(response["decision"])  # "ALLOW" or "DENY"
print(response["rationale"])  # Explanation from Guard
```

### Example: With Context

```python
response = await client.call_tool("send_intent", {
    "action": "write",
    "resource": {"type": "file", "name": "config.yaml", "location": "local"},
    "data": {"sensitivity": ["internal"], "pii": False, "volume": "single"},
    "risk": {"authn": "required"},
    "context": {
        "tool_name": "deployment_agent",
        "tool_method": "update_config",
        "layer": "L4"
    }
})
```

## Complete Example

```python
import asyncio
import json
from fastmcp import Client
from fastmcp.client.transports import StreamableHttpTransport


async def main():
    # Setup
    transport = StreamableHttpTransport(url="http://13.205.39.173:3001/mcp")
    client = Client(transport)

    async with client:
        # List available tools
        tools = await client.list_tools()
        print(f"Available tools: {[t.name for t in tools]}\n")

        # Send intent to Guard
        print("Sending intent to Guard...")
        response = await client.call_tool("send_intent", {
            "action": "read",
            "resource": {
                "type": "database",
                "name": "customers",
                "location": "cloud"
            },
            "data": {
                "sensitivity": ["internal"],
                "pii": True,
                "volume": "single"
            },
            "risk": {
                "authn": "required"
            },
            "context": {
                "tool_name": "my_app",
                "layer": "L4"
            }
        })

        # Parse response
        decision = response.get("decision")
        rationale = response.get("rationale")
        latency = response.get("enforcement_latency_ms")

        print(f"\n✓ Decision: {decision}")
        print(f"✓ Rationale: {rationale}")
        print(f"✓ Latency: {latency}ms")
        print(f"\nFull response:\n{json.dumps(response, indent=2)}")

        # Handle decision
        if decision == "ALLOW":
            print("\n✅ Access granted - proceed with operation")
        else:
            print("\n❌ Access denied - operation blocked")


if __name__ == "__main__":
    asyncio.run(main())
```
