# Fencio Python SDK

**Security enforcement for LangGraph agents** - Wrap your LangGraph agents with natural-language policies in one line.

[![PyPI version](https://badge.fury.io/py/fencio.svg)](https://pypi.org/project/fencio/)
[![Python 3.12+](https://img.shields.io/badge/python-3.12+-blue.svg)](https://www.python.org/downloads/)

## Quick Start

### 1. Install

```bash
pip install fencio
```

Requires Python 3.12+.

### 2. Get your API key

1. Go to the [Fencio Developer Platform](https://developer.fencio.dev)
2. Create or copy an API key
3. Set it in your environment:

```bash
export FENCIO_API_KEY="fencio_live_xxx_your_api_key"
```

### 3. Wrap your LangGraph agent

Wrap any compiled LangGraph agent with `enforcement_agent()`. The SDK will automatically:
- Register your agent with the Guard platform
- Enforce policies on every tool call
- Send telemetry for visibility

```python
import os
from langchain_openai import ChatOpenAI
from langgraph.prebuilt import create_react_agent
from fencio.agent import enforcement_agent

# 1) Build your normal LangGraph agent
model = ChatOpenAI(model="gpt-4o-mini", temperature=0)
tools = [...]  # your tools here
agent = create_react_agent(model, tools)

# 2) Wrap with Fencio enforcement (one line)
secure_agent = enforcement_agent(
    graph=agent,
    agent_id="customer-support-agent",      # stable ID for this agent
    token=os.environ["FENCIO_API_KEY"],     # API key from developer platform
)

# 3) Use it exactly like before
result = secure_agent.invoke({
    "messages": [("user", "Delete customer record cust-12345")]
})
print(result)
```

**What happens automatically:**
- ✅ Agent registration in the Guard platform
- ✅ Every tool call becomes an **IntentEvent** and is enforced
- ✅ Decisions return as **ALLOW/BLOCK** with evidence
- ✅ **Soft-block by default** - violations are logged but don't break execution
- ✅ Full telemetry and session tracking

### 4. Create a policy

Once your wrapped agent runs, it appears in the [Guard Console](https://guard.fencio.dev):

1. Go to **Agent Policies**
2. Select your `agent_id` (e.g., `customer-support-agent`)
3. Choose a **policy template** or write custom natural-language rules
4. Click **Create Policy**

No SDK changes needed - your agent automatically uses the new policy on the next run.

### 5. View telemetry

See enforcement decisions in real-time:

1. In Guard Console, go to **Agents**
2. View recent **enforcement sessions** per agent
3. Click a session to see:
   - Full IntentEvent details
   - All rules evaluated with similarity scores
   - Decision (ALLOW/BLOCK) with evidence
   - Performance timings and execution timeline

---

## Configuration

### Required Parameters

- **`agent_id`** (str): Stable identifier for this agent - used for registration, policies, and telemetry
- **`token`** (str): API key from the developer platform (usually from `FENCIO_API_KEY` env var)

### Optional Parameters

- **`boundary_id`** (str): Human-readable label for logs/UI (default: `"default"`)
- **`base_url`** (str): Override the Guard API endpoint (default: `"https://guard.fencio.dev"`)
  - For local development: `"http://localhost:8000"`
- **`soft_block`** (bool): Log violations without blocking execution (default: `True`)
  - Set to `False` for hard-block mode (raises exceptions on violations)

### Advanced Configuration

```python
from fencio.agent import enforcement_agent

secure_agent = enforcement_agent(
    graph=agent,
    agent_id="my-agent",
    token=os.environ["FENCIO_API_KEY"],

    # Optional overrides
    boundary_id="production",           # Label for telemetry
    base_url="http://localhost:8000",   # Local development
    soft_block=False,                   # Hard-block mode
)
```

---

## Features

### 🛡️ Policy Enforcement
- Natural-language policies (no code changes needed)
- Template library for common security patterns
- Context-aware enforcement using semantic similarity
- Multi-layer enforcement (tool, resource, data)

### 📊 Telemetry & Observability
- Real-time session tracking
- Full decision audit trail with evidence
- Performance metrics (latency, rule evaluation time)
- Filterable session history by agent, decision, time

### 🔐 Authentication & Multi-Tenancy
- API key-based authentication
- Tenant isolation for policies and telemetry
- Developer platform integration for key management

### ⚡ Performance
- Async-first architecture
- Minimal overhead on agent execution
- Batched telemetry writes
- Efficient semantic search with ChromaDB

---

## Import Options

The SDK supports both modern `fencio` imports and legacy `tupl` imports for backward compatibility:

```python
# Modern imports (recommended)
from fencio.agent import enforcement_agent
from fencio import TuplClient, IntentEvent

# Legacy imports (still supported)
from tupl.agent import enforcement_agent
from tupl import TuplClient, IntentEvent
```

Both import paths work identically.

---

## Examples

### Basic LangGraph Integration

```python
from langchain_openai import ChatOpenAI
from langgraph.prebuilt import create_react_agent
from fencio.agent import enforcement_agent
import os

# Create agent
model = ChatOpenAI(model="gpt-4o-mini")
tools = [...]
agent = create_react_agent(model, tools)

# Wrap with enforcement
secure_agent = enforcement_agent(
    graph=agent,
    agent_id="support-bot",
    token=os.environ["FENCIO_API_KEY"]
)

# Run
result = secure_agent.invoke({"messages": [("user", "help me")]})
```

### Local Development

```python
secure_agent = enforcement_agent(
    graph=agent,
    agent_id="dev-agent",
    base_url="http://localhost:8000",  # Local Guard stack
    token="dev-key-123",
)
```

### Hard-Block Mode

```python
secure_agent = enforcement_agent(
    graph=agent,
    agent_id="strict-agent",
    token=os.environ["FENCIO_API_KEY"],
    soft_block=False,  # Raise exceptions on violations
)
```

---

## Architecture

```
┌─────────────┐
│  LangGraph  │
│    Agent    │
└──────┬──────┘
       │ wrapped by
       ▼
┌─────────────────┐      ┌──────────────────┐
│ enforcement_    │─────▶│  Management      │
│ agent()         │      │  Plane           │
│ (SDK Proxy)     │◀─────│  /api/v1/enforce │
└─────────────────┘      └────────┬─────────┘
       │                          │
       │ tool calls               │ gRPC
       ▼                          ▼
┌─────────────────┐      ┌──────────────────┐
│  Tool           │      │  Data Plane      │
│  Execution      │      │  (Policy Engine) │
└─────────────────┘      └──────────────────┘
                                 │
                                 ▼
                         ┌──────────────────┐
                         │  ChromaDB        │
                         │  (Policy Store)  │
                         └──────────────────┘
```

**Flow:**
1. SDK wraps LangGraph agent and registers it
2. On tool calls, SDK creates IntentEvents
3. Events sent to Management Plane `/api/v1/enforce`
4. Management Plane proxies to Data Plane gRPC
5. Data Plane queries ChromaDB for relevant policies
6. Decision (ALLOW/BLOCK) returned with evidence
7. SDK logs telemetry and executes/blocks tool call

---

## Development

### Project Structure

```
tupl_sdk/python/
├── fencio/              # Modern package alias
│   └── __init__.py      # Re-exports from tupl
├── tupl/                # Core implementation
│   ├── agent.py         # enforcement_agent() wrapper
│   ├── client.py        # TuplClient for API calls
│   ├── types.py         # IntentEvent, Actor, etc.
│   └── vocabulary.py    # Tool call → IntentEvent mapping
├── tests/               # Unit tests
├── examples/            # Usage examples
└── pyproject.toml       # Package config
```

### Running Tests

```bash
cd tupl_sdk/python
pip install -e ".[dev]"
pytest tests/ -v
```

### Contributing

See the main project repository for contribution guidelines.

---

## Troubleshooting

### Agent not appearing in Guard UI

- Ensure the wrapped agent has run at least once (SDK registers on first run)
- Check API key is valid and has correct permissions
- Verify `base_url` points to the correct Guard endpoint

### Connection errors

```bash
# Test Guard API health
curl https://guard.fencio.dev/health

# For local development
curl http://localhost:8000/health
```

### Import errors

Make sure you've installed the package:
```bash
pip install fencio
```

For development:
```bash
pip install -e .
```

---

## Links

- [Documentation](https://docs.fencio.dev)
- [Guard Console](https://guard.fencio.dev)
- [Developer Platform](https://developer.fencio.dev)
- [GitHub Issues](https://github.com/fencio/fencio/issues)

---

## License

See the main project repository for license information.
