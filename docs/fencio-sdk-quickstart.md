# Fencio Python SDK – Quickstart

Use the Fencio SDK to enforce natural‑language policies on your LangGraph agents with **one wrapper**. This guide assumes you already have an agent built with LangGraph (React agent pattern).

---

## 1. Install the SDK

```bash
pip install fencio
```

Requirements: Python 3.12+.

---

## 2. Get your API key

1. Go to the **Fencio Developer Platform**.
2. Create or copy an API key.
3. Set it in your environment (recommended):

   ```bash
   export FENCIO_API_KEY="fencio_live_xxx_your_api_key"
   ```

---

## 3. Wrap your LangGraph agent

Wrap any compiled LangGraph agent with `enforcement_agent()`. Enforcement will go through the **Management Plane** enforcement mode by default and use **soft‑block** behavior (log violations, don’t break your agent).

```python
import os
from langchain_openai import ChatOpenAI
from langgraph.prebuilt import create_react_agent
from fencio.agent import enforcement_agent

# 1) Build your normal agent
model = ChatOpenAI(model="gpt-4o-mini", temperature=0)
tools = [/* your tools here */]
agent = create_react_agent(model, tools)

# 2) Wrap with Fencio enforcement (one line)
secure_agent = enforcement_agent(
    graph=agent,
    agent_id="customer-support-agent",        # stable ID for this agent
    token=os.environ["FENCIO_API_KEY"],       # API key from developer platform
    # boundary_id="default",                  # optional label for logs/UI
)

# 3) Use it exactly like before
result = secure_agent.invoke({
    "messages": [("user", "Delete customer record cust-12345")]
})
print(result)
```

**What happens automatically:**

- The SDK **registers your agent**
- Every tool call is turned into an **IntentEvent** and sent for enforcement.
- Decisions come back as **ALLOW/BLOCK with evidence**; by default, violations are soft‑blocked (logged, not raised).

You don’t need to set `enforcement_mode`—the SDK defaults to Management Plane enforcement.

For local development or self-hosted stacks, override the base URL:

```python
secure_agent = enforcement_agent(
    graph=agent,
    agent_id="customer-support-agent",
    base_url="http://localhost:8000",
    token=os.environ["FENCIO_API_KEY"],
)
```

---

## 4. Create a policy for your agent

Once your wrapped agent has run at least once, it will appear in the Guard UI.

1. Open **Guard Console** (`https://guard.fencio.dev`).
2. Go to **Agent Policies**.
3. Select your `agent_id` (for example, `customer-support-agent`).
4. Choose a **policy template** (e.g. “Block destructive database operations”).
5. Optionally add natural‑language customization.
6. Click **Create Policy**.

No SDK changes are required—your wrapped agent will automatically use the new policy.

---

## 5. View telemetry and decisions

To see how enforcement is behaving:

1. In Guard Console, go to **Agents**.
2. You’ll see recent **enforcement sessions** (per agent):
   - decision (ALLOW/BLOCK),
   - layer (L4 tool enforcement),
   - intent summary,
   - duration and rule count.
3. Click a session to see full **telemetry**:
   - the original IntentEvent,
   - all rules evaluated and their similarity scores,
   - performance timings and execution timeline.

---

## 6. Minimal configuration reference

Most integrations only need these knobs:

- `agent_id` (required)  
  Stable string for this agent, used as the key for registration, policies, and rules.

- `token` (required)  
  API key from the developer platform (`api_keys` table), usually passed as:

  ```bash
  export FENCIO_API_KEY="fencio_live_xxx"
  ```

- `boundary_id` (optional)  
  Human‑readable label used in logs and UI; doesn’t change routing. A simple `"default"` is fine.

That’s it—install `fencio`, wrap your agent with `enforcement_agent()`, create a policy in the UI, and you have Guard enforcement on every tool call.

---
