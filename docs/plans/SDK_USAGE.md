# Tupl SDK Usage Guide

**Version:** 1.3
**Last Updated:** 2025-11-14

## Overview

The Tupl SDK provides semantic security enforcement for LLM agents and tool calls. It uses vector embeddings and semantic similarity to enforce security policies based on the *intent* of operations, not just static rules.

## Quick Start

### Installation

```bash
pip install tupl-sdk
```

### Basic Configuration

Set your Management Plane URL (default: `http://localhost:8000`):

```bash
export TUPL_BASE_URL=http://localhost:8000
export TUPL_TENANT_ID=your-tenant-id
```

### Enforcement Modes

Control blocking behavior with `TUPL_ENFORCEMENT_MODE`:

- **`audit`** (default) - Logs policy violations but allows execution
- **`block`** - Raises `PermissionError` and prevents execution

```bash
export TUPL_ENFORCEMENT_MODE=block  # Enable blocking
```

---

## Use Case 1: Securing a LangGraph Agent

### Scenario

You have a customer service agent that can search, update, and delete customer records. You want to:

- ✅ Allow read operations
- ⚠️ Audit write operations
- 🚫 Block destructive operations (delete, export)

### Code

```python
from langchain_openai import ChatOpenAI
from langgraph.prebuilt import create_react_agent
from tupl_sdk import enforcement_agent

# Define your tools
from my_tools import search_customer, update_customer, delete_customer

tools = [search_customer, update_customer, delete_customer]

# Create your agent
model = ChatOpenAI(model="gpt-4", temperature=0)
agent = create_react_agent(model, tools)

# Wrap with Tupl enforcement (5 lines!)
secure_agent = enforcement_agent(
    agent=agent,
    tenant_id="customer-service-team",
    boundary_id="all"  # Check all policies
)

# Use it just like the original agent
response = secure_agent.invoke({
    "messages": [("user", "Delete customer record cust-12345")]
})
```

### What Happens

1. **Agent plans to call** `delete_customer(record_id="cust-12345")`
2. **SDK captures the tool call** and converts it to an `IntentEvent`
3. **Management Plane evaluates** the intent against all policies
4. **Policy matches**: "Deny Agent Destructive Ops" (DELETE on database)
5. **Result**: `PermissionError` raised (if `TUPL_ENFORCEMENT_MODE=block`)

### Sample Output

```
🚫 BLOCKED by Tupl Enforcement
Reason: Tool call 'delete_customer' blocked by boundary 'deny-agent-destructive-any'.
Intent ID: intent_abc123, Similarities: [1.0, 0.87, 0.99, 1.0]
```

---

## Use Case 2: Auditing API Calls in a Data Pipeline

### Scenario

You have an automated data pipeline that exports customer analytics. You want to:

- 📊 Log all export operations for compliance
- 🔍 Understand semantic similarity to policies
- 🚨 Alert (but don't block) if exports match risky patterns

### Code

```python
import os
from tupl_sdk import TuplClient
from tupl_sdk.types import IntentEvent, ActionConstraints, ResourceConstraints

# Enable audit mode (log only, don't block)
os.environ["TUPL_ENFORCEMENT_MODE"] = "audit"

# Initialize client (defaults to https://guard.fencio.dev)
client = TuplClient()

# Your export function
def export_customer_analytics(format: str, filters: dict):
    """Export customer analytics to CSV/JSON."""

    # Create an intent event
    intent = IntentEvent(
        id="export_" + str(uuid.uuid4()),
        schemaVersion="v1.2",
        action="export",
        actor={"type": "service", "id": "analytics-pipeline"},
        resource={
            "type": "database",
            "name": "customer_analytics",
            "location": "cloud"
        },
        data={
            "sensitivity": "internal",
            "pii": True,
            "volume": "bulk"
        },
        risk={"authn": "required"},
        timestamp=time.time(),
        context={"format": format, "filters": filters}
    )

    # Check policy compliance
    result = client.compare_intent(intent, boundary_id="all")

    if result.decision == "block":
        print(f"⚠️ Policy violation detected: {result.boundary_id}")
        print(f"   Similarities: {result.similarities}")
        # Log to monitoring system
        send_compliance_alert(intent, result)

    # Continue execution (audit mode)
    return perform_export(format, filters)
```

### What Happens

1. **Pipeline creates explicit IntentEvent** before export
2. **SDK checks policies** via `/api/v1/intents/compare`
3. **Management Plane returns**:
   - `decision: "block"` (matches "Block Risky Operations")
   - Similarity scores: `[1.0, 0.95, 0.88, 1.0]`
4. **Audit mode**: Logs violation but allows execution
5. **Compliance team** receives alert with semantic context

### Sample Output

```
⚠️ Policy violation detected: block-risky-ops (Block Risky Operations)
   Similarities: [1.0, 0.95, 0.88, 1.0]
   Action: export (1.0 match)
   Resource: database (0.95 similarity)
   Data: bulk PII (0.88 similarity)
   Risk: required auth (1.0 match)
```

---

## Key Concepts

### Intent Events

An `IntentEvent` captures the **who, what, where, why** of an operation:

```python
{
  "action": "delete",           # What: read, write, delete, export, etc.
  "actor": {                    # Who
    "type": "agent",           # user, service, llm, agent
    "id": "customer-agent-v2"
  },
  "resource": {                # Where
    "type": "database",        # database, file, api, etc.
    "name": "customer_records",
    "location": "cloud"
  },
  "data": {                    # Context
    "sensitivity": "internal", # public, internal, confidential
    "pii": true,
    "volume": "single"         # single, batch, bulk
  },
  "risk": {                    # Why risky
    "authn": "required"        # required, not_required
  }
}
```

### Policy Evaluation

The Management Plane:

1. **Filters** boundaries by applicability (action, resource type, actor)
2. **Encodes** intent and boundaries to 128-dim vectors (4 slots × 32 dims)
3. **Compares** via Rust sandbox using cosine similarity
4. **Aggregates** results across all policies
5. **Returns** decision + similarity scores

### Similarity Scores

Each slot (action, resource, data, risk) gets a similarity score [0.0 - 1.0]:

- **1.0** = Perfect semantic match
- **0.9+** = Very strong match
- **0.7-0.9** = Moderate match
- **< 0.7** = Weak match (typically doesn't trigger policy)

---

## Best Practices

### 1. Start with Audit Mode

```bash
export TUPL_ENFORCEMENT_MODE=audit
```

Run your agent/pipeline in audit mode first to:
- Understand which policies trigger
- Tune thresholds without disrupting operations
- Build confidence before enforcing blocks

### 2. Use Descriptive Intent Context

```python
IntentEvent(
    ...,
    context={
        "user_request": "Delete customer due to GDPR request",
        "approval_ticket": "LEGAL-12345",
        "initiated_by": "compliance-team"
    }
)
```

Rich context helps with:
- Debugging policy violations
- Compliance audits
- Root cause analysis

### 3. Check Policy Coverage

Before deploying, verify policies cover your use cases:

```bash
# List all boundaries
curl http://localhost:8000/api/v1/boundaries

# Test a specific intent
curl -X POST http://localhost:8000/api/v1/intents/compare \
  -H "Content-Type: application/json" \
  -d @test_intent.json
```

### 4. Monitor Telemetry

```python
# Retrieve recent decisions
telemetry = client.get_telemetry(limit=100)

for event in telemetry:
    if event.decision == "block":
        print(f"Blocked: {event.intent_id} - {event.boundary_id}")
        print(f"Similarities: {event.similarities}")
```

---

## Advanced: Custom Action/Resource Mappers

Override default intent inference logic:

```python
from tupl_sdk.agent import enforcement_agent

def custom_action_mapper(tool_name: str, tool_input: dict) -> str:
    """Map tool names to semantic actions."""
    if "search" in tool_name.lower():
        return "read"
    if "update" in tool_name.lower() or "modify" in tool_name.lower():
        return "write"
    if "delete" in tool_name.lower() or "remove" in tool_name.lower():
        return "delete"
    return "execute"  # fallback

secure_agent = enforcement_agent(
    agent=agent,
    tenant_id="my-tenant",
    action_mapper=custom_action_mapper,
    resource_mapper=lambda name, input: {
        "type": "api",
        "name": name,
        "location": "cloud"
    }
)
```

---

## Troubleshooting

### "No applicable boundaries" Warning

**Symptom**: All intents show `[0.0, 0.0, 0.0, 0.0]` similarities

**Cause**: Applicability filter is too strict (no policies match the intent)

**Fix**:
1. Check `APPLICABILITY_MODE` (use `soft`, not `strict`)
2. Verify policy constraints match your intent:
   - Actor type (`user`, `agent`, `service`, `llm`)
   - Resource type (`database`, `file`, `api`)
   - Data constraints (PII, volume, sensitivity)

### "409 Conflict" When Seeding Policies

**Symptom**: `Failed to seed policy: 409 Conflict`

**Cause**: Policy with same ID already exists

**Fix**:
```bash
# Reset all policies
uv run policies.py --reset

# Or delete specific policy
curl -X DELETE http://localhost:8000/api/v1/boundaries/my-policy-id
```

### Agent Blocked Unexpectedly

**Symptom**: Tool calls blocked but you expected them to pass

**Debug**:
1. Check which boundary matched: `boundary_id` in error message
2. Inspect similarity scores: Which slot failed threshold?
3. Review policy thresholds in Management Plane
4. Test intent manually:
   ```bash
   curl -X POST http://localhost:8000/api/v1/intents/compare \
     -H "Content-Type: application/json" \
     -d '{"action":"write", "actor":{"type":"agent"}, ...}'
   ```

---

## Next Steps

- **Read the API Reference**: `/docs` on your Management Plane
- **Explore Example Policies**: `examples/langgraph_demo/policies.py`
- **Run Demos**: `examples/langgraph_demo/demo_with_enforcement.py`
- **View Telemetry**: `GET /api/v1/telemetry`

## Support

- **GitHub Issues**: https://github.com/your-org/mgmt-plane/issues
- **Documentation**: https://docs.tupl.dev
- **Slack Community**: https://tupl-community.slack.com
