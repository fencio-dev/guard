# Natural Language Guardrails Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement natural language policy configuration for AI agent guardrails using LLM-based parsing and template library.

**Architecture:** Backend uses Gemini 2.5 Flash Lite to convert natural language templates to structured PolicyRules. SDK auto-registers agents and supports soft-block mode. Web UI and MCP Gateway provide multi-channel configuration.

**Tech Stack:** FastAPI, Pydantic, Google GenAI SDK, PostgreSQL with RLS, TypeScript (MCP Gateway), React (Web UI)

---

## Task 1: Database Schema Setup

**Files:**
- Create: `management-plane/migrations/add_agent_policies_tables.sql`

**Step 1: Write migration for registered_agents table**

```sql
-- Add registered_agents table
CREATE TABLE registered_agents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
    agent_id TEXT NOT NULL,
    first_seen TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    sdk_version TEXT,
    metadata JSONB DEFAULT '{}',
    UNIQUE(tenant_id, agent_id)
);

CREATE INDEX idx_registered_agents_tenant_id ON registered_agents(tenant_id);
CREATE INDEX idx_registered_agents_tenant_agent ON registered_agents(tenant_id, agent_id);

ALTER TABLE registered_agents ENABLE ROW LEVEL SECURITY;

CREATE POLICY "Users can view their own agents"
    ON registered_agents FOR SELECT
    USING (auth.uid() = tenant_id);

CREATE POLICY "Users can register their own agents"
    ON registered_agents FOR INSERT
    WITH CHECK (auth.uid() = tenant_id);
```

**Step 2: Add agent_policies table to migration**

```sql
-- Add agent_policies table
CREATE TABLE agent_policies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    agent_id TEXT NOT NULL,
    template_id TEXT NOT NULL,
    template_text TEXT NOT NULL,
    customization TEXT,
    policy_rules JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    FOREIGN KEY (tenant_id, agent_id)
        REFERENCES registered_agents(tenant_id, agent_id)
        ON DELETE CASCADE,
    UNIQUE(tenant_id, agent_id)
);

CREATE INDEX idx_agent_policies_tenant_id ON agent_policies(tenant_id);
CREATE INDEX idx_agent_policies_agent_id ON agent_policies(agent_id);
CREATE INDEX idx_agent_policies_tenant_agent ON agent_policies(tenant_id, agent_id);

ALTER TABLE agent_policies ENABLE ROW LEVEL SECURITY;

CREATE POLICY "Users can view their own agent policies"
    ON agent_policies FOR SELECT
    USING (auth.uid() = tenant_id);

CREATE POLICY "Users can insert their own agent policies"
    ON agent_policies FOR INSERT
    WITH CHECK (auth.uid() = tenant_id);

CREATE POLICY "Users can update their own agent policies"
    ON agent_policies FOR UPDATE
    USING (auth.uid() = tenant_id);

CREATE POLICY "Users can delete their own agent policies"
    ON agent_policies FOR DELETE
    USING (auth.uid() = tenant_id);
```

**Step 3: Apply migration**

Run: `psql $DATABASE_URL -f management-plane/migrations/add_agent_policies_tables.sql`

Expected: Tables created successfully with RLS policies

**Step 4: Commit**

```bash
git add management-plane/migrations/add_agent_policies_tables.sql
git commit -m "feat: add database schema for agent policies and registration"
```

---

## Task 2: Policy Templates Module

**Files:**
- Create: `management-plane/app/policy_templates.py`
- Create: `management-plane/tests/test_policy_templates.py`

**Step 1: Write test for template structure**

```python
# tests/test_policy_templates.py
import pytest
from app.policy_templates import PolicyTemplate, POLICY_TEMPLATES

def test_policy_template_structure():
    """Test that all templates have required fields."""
    assert len(POLICY_TEMPLATES) > 0

    for template in POLICY_TEMPLATES:
        assert template.id
        assert template.name
        assert template.description
        assert template.template_text
        assert template.category in ["database", "file", "api", "general"]
        assert isinstance(template.example_customizations, list)

def test_template_ids_unique():
    """Test that template IDs are unique."""
    ids = [t.id for t in POLICY_TEMPLATES]
    assert len(ids) == len(set(ids))
```

**Step 2: Run test to verify it fails**

Run: `cd management-plane && pytest tests/test_policy_templates.py -v`

Expected: FAIL - module not found

**Step 3: Implement policy templates module**

```python
# app/policy_templates.py
from pydantic import BaseModel

class PolicyTemplate(BaseModel):
    id: str
    name: str
    description: str
    template_text: str
    category: str
    example_customizations: list[str]

POLICY_TEMPLATES = [
    PolicyTemplate(
        id="database_read_only",
        name="Database Read-Only Access",
        description="Allow agent to read from databases without write permissions",
        template_text="Allow reading from databases",
        category="database",
        example_customizations=[
            "only from analytics_db",
            "only public data",
            "excluding PII fields"
        ]
    ),
    PolicyTemplate(
        id="database_write_access",
        name="Database Write Access",
        description="Allow agent to write to databases",
        template_text="Allow writing to databases",
        category="database",
        example_customizations=[
            "only to staging databases",
            "only public data",
            "single records only"
        ]
    ),
    PolicyTemplate(
        id="file_export",
        name="File Export Capabilities",
        description="Allow agent to export data to files",
        template_text="Allow exporting data to files",
        category="file",
        example_customizations=[
            "only CSV and JSON formats",
            "only public data",
            "maximum 1000 records at a time"
        ]
    ),
    PolicyTemplate(
        id="api_read_access",
        name="API Read Access",
        description="Allow agent to call external APIs for reading data",
        template_text="Allow calling external APIs for reading data",
        category="api",
        example_customizations=[
            "only public APIs",
            "excluding payment APIs",
            "with authentication required"
        ]
    ),
    PolicyTemplate(
        id="unrestricted",
        name="Unrestricted Access",
        description="Allow agent full access to all operations",
        template_text="Allow all operations",
        category="general",
        example_customizations=[
            "with authentication required",
            "excluding production databases"
        ]
    )
]

def get_template_by_id(template_id: str) -> PolicyTemplate | None:
    """Get template by ID."""
    return next((t for t in POLICY_TEMPLATES if t.id == template_id), None)

def get_templates_by_category(category: str) -> list[PolicyTemplate]:
    """Get templates by category."""
    return [t for t in POLICY_TEMPLATES if t.category == category]
```

**Step 4: Run tests to verify they pass**

Run: `cd management-plane && pytest tests/test_policy_templates.py -v`

Expected: PASS

**Step 5: Commit**

```bash
git add management-plane/app/policy_templates.py management-plane/tests/test_policy_templates.py
git commit -m "feat: add policy template library with 5 initial templates"
```

---

## Task 3: NL Policy Parser - Schema Models

**Files:**
- Create: `management-plane/app/nl_policy_parser.py`
- Create: `management-plane/tests/test_nl_policy_parser.py`

**Step 1: Write test for schema models**

```python
# tests/test_nl_policy_parser.py
import pytest
from app.nl_policy_parser import (
    ActionConstraints,
    ResourceConstraints,
    DataConstraints,
    RiskConstraints,
    PolicyConstraints,
    SliceThresholds,
    PolicyRules
)

def test_policy_rules_schema():
    """Test PolicyRules schema validation."""
    policy = PolicyRules(
        thresholds=SliceThresholds(),
        decision="min",
        constraints=PolicyConstraints(
            action=ActionConstraints(actions=["read"], actor_types=["user"]),
            resource=ResourceConstraints(types=["database"]),
            data=DataConstraints(sensitivity=["public"]),
            risk=RiskConstraints(authn="required")
        )
    )

    assert policy.thresholds.action == 0.85
    assert policy.decision == "min"
    assert policy.constraints.action.actions == ["read"]
```

**Step 2: Run test to verify it fails**

Run: `cd management-plane && pytest tests/test_nl_policy_parser.py::test_policy_rules_schema -v`

Expected: FAIL - module not found

**Step 3: Implement schema models**

```python
# app/nl_policy_parser.py
from typing import Optional, Literal
from pydantic import BaseModel

class ActionConstraints(BaseModel):
    actions: list[str]
    actor_types: list[str]

class ResourceConstraints(BaseModel):
    types: list[str]
    names: Optional[list[str]] = None
    locations: Optional[list[str]] = None

class DataConstraints(BaseModel):
    sensitivity: list[str]
    pii: Optional[bool] = None
    volume: Optional[str] = None

class RiskConstraints(BaseModel):
    authn: str

class PolicyConstraints(BaseModel):
    action: ActionConstraints
    resource: ResourceConstraints
    data: DataConstraints
    risk: RiskConstraints

class SliceThresholds(BaseModel):
    action: float = 0.85
    resource: float = 0.80
    data: float = 0.75
    risk: float = 0.70

class PolicyRules(BaseModel):
    """Structured policy rules generated from natural language."""
    thresholds: SliceThresholds
    decision: Literal["min", "weighted-avg"] = "min"
    globalThreshold: Optional[float] = None
    constraints: PolicyConstraints
```

**Step 4: Run test to verify it passes**

Run: `cd management-plane && pytest tests/test_nl_policy_parser.py::test_policy_rules_schema -v`

Expected: PASS

**Step 5: Commit**

```bash
git add management-plane/app/nl_policy_parser.py management-plane/tests/test_nl_policy_parser.py
git commit -m "feat: add Pydantic schema models for policy rules"
```

---

## Task 4: NL Policy Parser - LLM Integration

**Files:**
- Modify: `management-plane/app/nl_policy_parser.py`
- Modify: `management-plane/tests/test_nl_policy_parser.py`

**Step 1: Write test for LLM parsing**

```python
# Add to tests/test_nl_policy_parser.py
import os
from app.nl_policy_parser import NLPolicyParser

@pytest.mark.asyncio
async def test_parse_simple_template():
    """Test parsing a simple template without customization."""
    api_key = os.getenv("GEMINI_API_KEY")
    if not api_key:
        pytest.skip("GEMINI_API_KEY not set")

    parser = NLPolicyParser(api_key=api_key)
    policy = await parser.parse_policy(
        template_id="database_read_only",
        template_text="Allow reading from databases",
        customization=None
    )

    assert policy.constraints.action.actions == ["read"]
    assert "database" in policy.constraints.resource.types
    assert policy.thresholds.action == 0.85

@pytest.mark.asyncio
async def test_parse_with_customization():
    """Test parsing with natural language customization."""
    api_key = os.getenv("GEMINI_API_KEY")
    if not api_key:
        pytest.skip("GEMINI_API_KEY not set")

    parser = NLPolicyParser(api_key=api_key)
    policy = await parser.parse_policy(
        template_id="database_read_only",
        template_text="Allow reading from databases",
        customization="only public data"
    )

    assert policy.constraints.data.sensitivity == ["public"]
```

**Step 2: Run test to verify it fails**

Run: `cd management-plane && GEMINI_API_KEY=$GEMINI_API_KEY pytest tests/test_nl_policy_parser.py::test_parse_simple_template -v`

Expected: FAIL - NLPolicyParser not defined

**Step 3: Implement NLPolicyParser class**

```python
# Add to app/nl_policy_parser.py
import hashlib
import logging
from google import genai
from google.genai import types

logger = logging.getLogger(__name__)

# Vocabulary constants (reference existing vocabulary.yaml)
VALID_ACTIONS = ["read", "write", "delete", "export", "execute"]
VALID_RESOURCE_TYPES = ["database", "file", "api"]
VALID_SENSITIVITY = ["public", "internal", "confidential"]
VALID_VOLUMES = ["single", "bulk"]
VALID_AUTHN = ["required", "not_required"]
VALID_ACTOR_TYPES = ["user", "agent", "service"]

class NLPolicyParser:
    """Parse natural language policy templates into structured PolicyRules."""

    def __init__(self, api_key: str):
        self.client = genai.Client(api_key=api_key)
        self.model = "gemini-2.0-flash-lite"
        self._cache: dict[str, PolicyRules] = {}

    def _compute_cache_key(self, template_id: str, customization: Optional[str]) -> str:
        """Compute cache key from template and customization."""
        content = f"{template_id}:{customization or ''}"
        return hashlib.sha256(content.encode()).hexdigest()

    def _build_prompt(self, template_text: str, customization: Optional[str]) -> str:
        """Build LLM prompt for policy generation."""
        return f"""You are a security policy generator for an AI agent guardrail system.

INPUT:
Template: {template_text}
Customization: {customization or "none"}

CANONICAL VOCABULARY:
Actions: {VALID_ACTIONS}
Resource Types: {VALID_RESOURCE_TYPES}
Sensitivity Levels: {VALID_SENSITIVITY}
Volumes: {VALID_VOLUMES}
Authn Levels: {VALID_AUTHN}
Actor Types: {VALID_ACTOR_TYPES}

TASK:
Generate a PolicyRules object that represents an ALLOW policy for this guardrail.

RULES:
1. Use ONLY vocabulary values listed above
2. Set default thresholds: action=0.85, resource=0.80, data=0.75, risk=0.70
3. Use "min" decision mode by default
4. Extract constraints from the natural language
5. For actor_types, default to ["user", "agent"] if not specified
6. For authn, default to "required" if not specified

OUTPUT: Return JSON matching the PolicyRules schema.
"""

    def _validate_vocabulary_compliance(self, policy: PolicyRules) -> None:
        """Validate that policy uses only canonical vocabulary."""
        # Validate actions
        for action in policy.constraints.action.actions:
            if action not in VALID_ACTIONS:
                raise ValueError(f"Invalid action '{action}'. Must be one of: {VALID_ACTIONS}")

        # Validate resource types
        for rtype in policy.constraints.resource.types:
            if rtype not in VALID_RESOURCE_TYPES:
                raise ValueError(f"Invalid resource type '{rtype}'. Must be one of: {VALID_RESOURCE_TYPES}")

        # Validate sensitivity
        for sens in policy.constraints.data.sensitivity:
            if sens not in VALID_SENSITIVITY:
                raise ValueError(f"Invalid sensitivity '{sens}'. Must be one of: {VALID_SENSITIVITY}")

        # Validate volume if present
        if policy.constraints.data.volume and policy.constraints.data.volume not in VALID_VOLUMES:
            raise ValueError(f"Invalid volume '{policy.constraints.data.volume}'. Must be one of: {VALID_VOLUMES}")

        # Validate authn
        if policy.constraints.risk.authn not in VALID_AUTHN:
            raise ValueError(f"Invalid authn '{policy.constraints.risk.authn}'. Must be one of: {VALID_AUTHN}")

    async def parse_policy(
        self,
        template_id: str,
        template_text: str,
        customization: Optional[str]
    ) -> PolicyRules:
        """Parse natural language template into PolicyRules."""
        # Check cache
        cache_key = self._compute_cache_key(template_id, customization)
        if cache_key in self._cache:
            logger.info(f"Cache hit for template '{template_id}'")
            return self._cache[cache_key]

        # Build prompt
        prompt = self._build_prompt(template_text, customization)

        # Call Gemini with structured output
        try:
            response = self.client.models.generate_content(
                model=self.model,
                contents=prompt,
                config=types.GenerateContentConfig(
                    response_mime_type="application/json",
                    response_schema=PolicyRules,
                    temperature=0.3,
                ),
            )

            # Parse and validate
            policy_rules = PolicyRules.model_validate_json(response.text)

            # Validate vocabulary compliance
            self._validate_vocabulary_compliance(policy_rules)

            # Cache and return
            self._cache[cache_key] = policy_rules
            logger.info(f"Successfully parsed policy for template '{template_id}'")
            return policy_rules

        except Exception as e:
            logger.error(f"Failed to parse policy: {e}")
            raise ValueError(
                "Failed to parse policy from natural language. "
                "Please try rephrasing your customization or use a different template."
            ) from e
```

**Step 4: Run tests to verify they pass**

Run: `cd management-plane && GEMINI_API_KEY=$GEMINI_API_KEY pytest tests/test_nl_policy_parser.py -v`

Expected: PASS (both tests)

**Step 5: Commit**

```bash
git add management-plane/app/nl_policy_parser.py management-plane/tests/test_nl_policy_parser.py
git commit -m "feat: implement LLM-based natural language policy parser"
```

---

## Task 5: Agent Management API Endpoints

**Files:**
- Create: `management-plane/app/endpoints/agents.py`
- Create: `management-plane/tests/test_endpoints_agents.py`

**Step 1: Write test for agent registration endpoint**

```python
# tests/test_endpoints_agents.py
import pytest
from fastapi.testclient import TestClient
from app.main import app

client = TestClient(app)

def test_register_agent(test_db, auth_headers):
    """Test agent registration endpoint."""
    response = client.post(
        "/api/v1/agents/register",
        json={
            "agent_id": "test-agent",
            "sdk_version": "1.3.0",
            "metadata": {}
        },
        headers=auth_headers
    )

    assert response.status_code == 200
    data = response.json()
    assert data["agent_id"] == "test-agent"
    assert "first_seen" in data
    assert "last_seen" in data

def test_register_agent_duplicate_updates_last_seen(test_db, auth_headers):
    """Test that re-registering updates last_seen."""
    # Register first time
    response1 = client.post(
        "/api/v1/agents/register",
        json={"agent_id": "dup-agent", "sdk_version": "1.3.0"},
        headers=auth_headers
    )
    first_seen_1 = response1.json()["first_seen"]

    # Register again
    response2 = client.post(
        "/api/v1/agents/register",
        json={"agent_id": "dup-agent", "sdk_version": "1.3.1"},
        headers=auth_headers
    )

    assert response2.status_code == 200
    data = response2.json()
    assert data["first_seen"] == first_seen_1  # Unchanged
    assert data["sdk_version"] == "1.3.1"  # Updated

def test_list_agents(test_db, auth_headers):
    """Test listing registered agents."""
    # Register some agents
    client.post("/api/v1/agents/register", json={"agent_id": "agent-1"}, headers=auth_headers)
    client.post("/api/v1/agents/register", json={"agent_id": "agent-2"}, headers=auth_headers)

    # List agents
    response = client.get("/api/v1/agents/list", headers=auth_headers)

    assert response.status_code == 200
    data = response.json()
    assert data["total"] >= 2
    agent_ids = [a["agent_id"] for a in data["agents"]]
    assert "agent-1" in agent_ids
    assert "agent-2" in agent_ids
```

**Step 2: Run test to verify it fails**

Run: `cd management-plane && pytest tests/test_endpoints_agents.py::test_register_agent -v`

Expected: FAIL - endpoint not found

**Step 3: Implement agent registration endpoint**

```python
# app/endpoints/agents.py
from fastapi import APIRouter, Depends, HTTPException
from pydantic import BaseModel
from typing import Optional
import uuid
from datetime import datetime
from app.database import get_db
from app.auth import get_current_user

router = APIRouter(prefix="/api/v1/agents", tags=["agents"])

class RegisterAgentRequest(BaseModel):
    agent_id: str
    sdk_version: Optional[str] = None
    metadata: Optional[dict] = None

class RegisteredAgent(BaseModel):
    id: uuid.UUID
    agent_id: str
    first_seen: datetime
    last_seen: datetime
    sdk_version: Optional[str]

@router.post("/register", response_model=RegisteredAgent)
async def register_agent(
    request: RegisterAgentRequest,
    db=Depends(get_db),
    user=Depends(get_current_user)
):
    """Register or update an agent."""
    # Check if agent exists
    existing = db.execute(
        """
        SELECT id, agent_id, first_seen, last_seen, sdk_version
        FROM registered_agents
        WHERE tenant_id = %s AND agent_id = %s
        """,
        (user.id, request.agent_id)
    ).fetchone()

    if existing:
        # Update last_seen and sdk_version
        db.execute(
            """
            UPDATE registered_agents
            SET last_seen = NOW(), sdk_version = %s
            WHERE tenant_id = %s AND agent_id = %s
            RETURNING id, agent_id, first_seen, last_seen, sdk_version
            """,
            (request.sdk_version, user.id, request.agent_id)
        )
        result = db.fetchone()
    else:
        # Insert new agent
        result = db.execute(
            """
            INSERT INTO registered_agents (tenant_id, agent_id, sdk_version, metadata)
            VALUES (%s, %s, %s, %s)
            RETURNING id, agent_id, first_seen, last_seen, sdk_version
            """,
            (user.id, request.agent_id, request.sdk_version, request.metadata or {})
        ).fetchone()

    db.commit()

    return RegisteredAgent(
        id=result[0],
        agent_id=result[1],
        first_seen=result[2],
        last_seen=result[3],
        sdk_version=result[4]
    )

class ListAgentsResponse(BaseModel):
    total: int
    agents: list[RegisteredAgent]

@router.get("/list", response_model=ListAgentsResponse)
async def list_agents(
    limit: int = 100,
    offset: int = 0,
    db=Depends(get_db),
    user=Depends(get_current_user)
):
    """List all registered agents for the current tenant."""
    # Get total count
    total = db.execute(
        "SELECT COUNT(*) FROM registered_agents WHERE tenant_id = %s",
        (user.id,)
    ).fetchone()[0]

    # Get agents
    rows = db.execute(
        """
        SELECT id, agent_id, first_seen, last_seen, sdk_version
        FROM registered_agents
        WHERE tenant_id = %s
        ORDER BY last_seen DESC
        LIMIT %s OFFSET %s
        """,
        (user.id, limit, offset)
    ).fetchall()

    agents = [
        RegisteredAgent(
            id=row[0],
            agent_id=row[1],
            first_seen=row[2],
            last_seen=row[3],
            sdk_version=row[4]
        )
        for row in rows
    ]

    return ListAgentsResponse(total=total, agents=agents)
```

**Step 4: Register router in main app**

Modify `management-plane/app/main.py`:
```python
from app.endpoints import agents

app.include_router(agents.router)
```

**Step 5: Run tests to verify they pass**

Run: `cd management-plane && pytest tests/test_endpoints_agents.py -v`

Expected: PASS

**Step 6: Commit**

```bash
git add management-plane/app/endpoints/agents.py management-plane/tests/test_endpoints_agents.py management-plane/app/main.py
git commit -m "feat: add agent registration and listing endpoints"
```

---

## Task 6: Policy Management API Endpoints

**Files:**
- Modify: `management-plane/app/endpoints/agents.py`
- Modify: `management-plane/tests/test_endpoints_agents.py`

**Step 1: Write test for policy creation endpoint**

```python
# Add to tests/test_endpoints_agents.py
def test_create_policy(test_db, auth_headers):
    """Test policy creation endpoint."""
    # Register agent first
    client.post("/api/v1/agents/register", json={"agent_id": "policy-agent"}, headers=auth_headers)

    # Create policy
    response = client.post(
        "/api/v1/agents/policies",
        json={
            "agent_id": "policy-agent",
            "template_id": "database_read_only",
            "template_text": "Allow reading from databases",
            "customization": "only public data"
        },
        headers=auth_headers
    )

    assert response.status_code == 200
    data = response.json()
    assert data["agent_id"] == "policy-agent"
    assert data["template_id"] == "database_read_only"
    assert "policy_rules" in data

def test_create_policy_unregistered_agent(test_db, auth_headers):
    """Test that creating policy for unregistered agent fails."""
    response = client.post(
        "/api/v1/agents/policies",
        json={
            "agent_id": "nonexistent",
            "template_id": "database_read_only",
            "template_text": "Allow reading"
        },
        headers=auth_headers
    )

    assert response.status_code == 404
    assert "agent_not_registered" in response.json()["detail"]["error"]

def test_get_policy(test_db, auth_headers):
    """Test retrieving agent policy."""
    # Setup
    client.post("/api/v1/agents/register", json={"agent_id": "get-agent"}, headers=auth_headers)
    client.post(
        "/api/v1/agents/policies",
        json={"agent_id": "get-agent", "template_id": "database_read_only", "template_text": "Allow reading"},
        headers=auth_headers
    )

    # Get policy
    response = client.get("/api/v1/agents/policies/get-agent", headers=auth_headers)

    assert response.status_code == 200
    data = response.json()
    assert data["agent_id"] == "get-agent"
```

**Step 2: Run test to verify it fails**

Run: `cd management-plane && pytest tests/test_endpoints_agents.py::test_create_policy -v`

Expected: FAIL - endpoint not found

**Step 3: Implement policy endpoints**

```python
# Add to app/endpoints/agents.py
from app.nl_policy_parser import NLPolicyParser
from app.policy_templates import get_template_by_id
import os

# Initialize parser
nl_parser = NLPolicyParser(api_key=os.getenv("GEMINI_API_KEY"))

class CreatePolicyRequest(BaseModel):
    agent_id: str
    template_id: str
    template_text: str
    customization: Optional[str] = None

class AgentPolicy(BaseModel):
    id: uuid.UUID
    agent_id: str
    template_id: str
    customization: Optional[str]
    policy_rules: dict
    created_at: datetime

@router.post("/policies", response_model=AgentPolicy)
async def create_agent_policy(
    request: CreatePolicyRequest,
    db=Depends(get_db),
    user=Depends(get_current_user)
):
    """Create or update policy for an agent."""
    # Check if agent is registered
    agent_exists = db.execute(
        "SELECT 1 FROM registered_agents WHERE tenant_id = %s AND agent_id = %s",
        (user.id, request.agent_id)
    ).fetchone()

    if not agent_exists:
        raise HTTPException(
            status_code=404,
            detail={
                "error": "agent_not_registered",
                "message": f"Agent '{request.agent_id}' not found. Wrap with enforcement_agent() first.",
                "agent_id": request.agent_id
            }
        )

    # Parse policy using LLM
    try:
        policy_rules = await nl_parser.parse_policy(
            template_id=request.template_id,
            template_text=request.template_text,
            customization=request.customization
        )
    except ValueError as e:
        raise HTTPException(status_code=400, detail=str(e))

    # Check if policy exists
    existing = db.execute(
        "SELECT id FROM agent_policies WHERE tenant_id = %s AND agent_id = %s",
        (user.id, request.agent_id)
    ).fetchone()

    if existing:
        # Update existing policy
        result = db.execute(
            """
            UPDATE agent_policies
            SET template_id = %s, template_text = %s, customization = %s,
                policy_rules = %s, updated_at = NOW()
            WHERE tenant_id = %s AND agent_id = %s
            RETURNING id, agent_id, template_id, customization, policy_rules, created_at
            """,
            (request.template_id, request.template_text, request.customization,
             policy_rules.model_dump(), user.id, request.agent_id)
        ).fetchone()
    else:
        # Insert new policy
        result = db.execute(
            """
            INSERT INTO agent_policies (tenant_id, agent_id, template_id, template_text, customization, policy_rules)
            VALUES (%s, %s, %s, %s, %s, %s)
            RETURNING id, agent_id, template_id, customization, policy_rules, created_at
            """,
            (user.id, request.agent_id, request.template_id, request.template_text,
             request.customization, policy_rules.model_dump())
        ).fetchone()

    db.commit()

    return AgentPolicy(
        id=result[0],
        agent_id=result[1],
        template_id=result[2],
        customization=result[3],
        policy_rules=result[4],
        created_at=result[5]
    )

@router.get("/policies/{agent_id}", response_model=AgentPolicy)
async def get_agent_policy(
    agent_id: str,
    db=Depends(get_db),
    user=Depends(get_current_user)
):
    """Get policy for an agent."""
    result = db.execute(
        """
        SELECT id, agent_id, template_id, customization, policy_rules, created_at
        FROM agent_policies
        WHERE tenant_id = %s AND agent_id = %s
        """,
        (user.id, agent_id)
    ).fetchone()

    if not result:
        raise HTTPException(status_code=404, detail="Policy not found")

    return AgentPolicy(
        id=result[0],
        agent_id=result[1],
        template_id=result[2],
        customization=result[3],
        policy_rules=result[4],
        created_at=result[5]
    )

@router.delete("/policies/{agent_id}")
async def delete_agent_policy(
    agent_id: str,
    db=Depends(get_db),
    user=Depends(get_current_user)
):
    """Delete policy for an agent."""
    db.execute(
        "DELETE FROM agent_policies WHERE tenant_id = %s AND agent_id = %s",
        (user.id, agent_id)
    )
    db.commit()

    return {"success": True}
```

**Step 4: Run tests to verify they pass**

Run: `cd management-plane && GEMINI_API_KEY=$GEMINI_API_KEY pytest tests/test_endpoints_agents.py -v`

Expected: PASS

**Step 5: Commit**

```bash
git add management-plane/app/endpoints/agents.py management-plane/tests/test_endpoints_agents.py
git commit -m "feat: add policy CRUD endpoints with LLM parsing"
```

---

## Task 7: Template Endpoints

**Files:**
- Modify: `management-plane/app/endpoints/agents.py`
- Modify: `management-plane/tests/test_endpoints_agents.py`

**Step 1: Write test for template endpoints**

```python
# Add to tests/test_endpoints_agents.py
def test_list_templates():
    """Test listing policy templates."""
    response = client.get("/api/v1/agents/templates")

    assert response.status_code == 200
    data = response.json()
    assert "templates" in data
    assert len(data["templates"]) > 0

    # Check structure
    template = data["templates"][0]
    assert "id" in template
    assert "name" in template
    assert "category" in template

def test_list_templates_by_category():
    """Test filtering templates by category."""
    response = client.get("/api/v1/agents/templates?category=database")

    assert response.status_code == 200
    data = response.json()
    for template in data["templates"]:
        assert template["category"] == "database"

def test_get_template_by_id():
    """Test getting a specific template."""
    response = client.get("/api/v1/agents/templates/database_read_only")

    assert response.status_code == 200
    data = response.json()
    assert data["id"] == "database_read_only"
    assert "example_customizations" in data
```

**Step 2: Run test to verify it fails**

Run: `cd management-plane && pytest tests/test_endpoints_agents.py::test_list_templates -v`

Expected: FAIL - endpoint not found

**Step 3: Implement template endpoints**

```python
# Add to app/endpoints/agents.py
from app.policy_templates import POLICY_TEMPLATES, get_template_by_id, get_templates_by_category

@router.get("/templates")
async def list_templates(category: Optional[str] = None):
    """List all policy templates, optionally filtered by category."""
    if category:
        templates = get_templates_by_category(category)
    else:
        templates = POLICY_TEMPLATES

    return {"templates": [t.model_dump() for t in templates]}

@router.get("/templates/{template_id}")
async def get_template(template_id: str):
    """Get a specific template by ID."""
    template = get_template_by_id(template_id)

    if not template:
        raise HTTPException(
            status_code=404,
            detail={
                "error": "template_not_found",
                "message": f"Template '{template_id}' not found.",
                "available_templates": [t.id for t in POLICY_TEMPLATES]
            }
        )

    return template.model_dump()
```

**Step 4: Run tests to verify they pass**

Run: `cd management-plane && pytest tests/test_endpoints_agents.py -v -k template`

Expected: PASS

**Step 5: Commit**

```bash
git add management-plane/app/endpoints/agents.py management-plane/tests/test_endpoints_agents.py
git commit -m "feat: add template listing and retrieval endpoints"
```

---

## Task 8: SDK - Agent Registration

**Files:**
- Modify: `tupl_sdk/python/tupl/agent.py`
- Create: `tupl_sdk/python/tests/test_agent_registration.py`

**Step 1: Write test for agent registration**

```python
# tests/test_agent_registration.py
import pytest
from unittest.mock import Mock, patch
from tupl.agent import SecureGraphProxy

def test_agent_registration_on_init():
    """Test that agent auto-registers on initialization."""
    mock_graph = Mock()

    with patch('httpx.post') as mock_post:
        mock_post.return_value.status_code = 200

        proxy = SecureGraphProxy(
            graph=mock_graph,
            agent_id="test-agent",
            boundary_id="test-boundary",
            tenant_id="tenant-123",
            token="test-token",
            base_url="http://localhost:8000"
        )

        # Verify registration was called
        mock_post.assert_called_once()
        call_args = mock_post.call_args
        assert "/api/v1/agents/register" in call_args[0][0]
        assert call_args[1]["json"]["agent_id"] == "test-agent"

def test_agent_registration_failure_non_critical():
    """Test that registration failures don't break initialization."""
    mock_graph = Mock()

    with patch('httpx.post') as mock_post:
        mock_post.side_effect = Exception("Network error")

        # Should not raise
        proxy = SecureGraphProxy(
            graph=mock_graph,
            agent_id="test-agent",
            boundary_id="test-boundary",
            tenant_id="tenant-123",
            token="test-token"
        )

        assert proxy.agent_id == "test-agent"
```

**Step 2: Run test to verify it fails**

Run: `cd tupl_sdk/python && pytest tests/test_agent_registration.py::test_agent_registration_on_init -v`

Expected: FAIL - agent_id parameter not accepted

**Step 3: Implement agent registration in SDK**

```python
# Modify tupl_sdk/python/tupl/agent.py
import httpx
import logging
from importlib.metadata import version

logger = logging.getLogger(__name__)

class SecureGraphProxy:
    def __init__(
        self,
        graph: Any,
        agent_id: str,  # NEW: Required parameter
        boundary_id: str,
        tenant_id: str,
        token: str,
        base_url: str = "http://localhost:8000",
        # ... existing params
    ):
        self.agent_id = agent_id
        self.boundary_id = boundary_id
        self.tenant_id = tenant_id
        self.token = token
        self.base_url = base_url
        # ... existing init code

        # Auto-register agent
        self._register_agent()

    def _register_agent(self):
        """Auto-register agent with Management Plane."""
        try:
            sdk_version = version("tupl")
        except Exception:
            sdk_version = "unknown"

        try:
            response = httpx.post(
                f"{self.base_url}/api/v1/agents/register",
                json={
                    "agent_id": self.agent_id,
                    "tenant_id": self.tenant_id,
                    "sdk_version": sdk_version,
                    "metadata": {}
                },
                headers={"Authorization": f"Bearer {self.token}"},
                timeout=2.0
            )

            if response.status_code == 200:
                logger.info(f"Agent '{self.agent_id}' registered successfully")
            else:
                logger.debug(f"Agent registration returned status {response.status_code}")

        except Exception as e:
            # Non-critical - don't break enforcement
            logger.debug(f"Agent registration failed: {e} (non-critical)")
```

**Step 4: Update enforcement_agent wrapper**

```python
# Add to tupl_sdk/python/tupl/agent.py
def enforcement_agent(
    agent: Any,
    agent_id: str,  # NEW: Required
    boundary_id: str,
    # ... existing params
) -> SecureGraphProxy:
    """Wrap an agent with enforcement capabilities."""
    return SecureGraphProxy(
        graph=agent,
        agent_id=agent_id,
        boundary_id=boundary_id,
        # ... pass through other params
    )
```

**Step 5: Run tests to verify they pass**

Run: `cd tupl_sdk/python && pytest tests/test_agent_registration.py -v`

Expected: PASS

**Step 6: Commit**

```bash
git add tupl_sdk/python/tupl/agent.py tupl_sdk/python/tests/test_agent_registration.py
git commit -m "feat: add automatic agent registration to SDK"
```

---

## Task 9: SDK - Soft-Block Mode

**Files:**
- Modify: `tupl_sdk/python/tupl/agent.py`
- Create: `tupl_sdk/python/tests/test_soft_block.py`

**Step 1: Write test for soft-block mode**

```python
# tests/test_soft_block.py
import pytest
from unittest.mock import Mock
from tupl.agent import SecureGraphProxy

def test_soft_block_logs_without_raising():
    """Test that soft-block mode logs violations without raising."""
    mock_graph = Mock()
    mock_event = Mock(tool_name="execute_query", id="event-123")
    mock_result = Mock(decision=0, slice_similarities={"action": 0.5})

    proxy = SecureGraphProxy(
        graph=mock_graph,
        agent_id="test-agent",
        boundary_id="test-boundary",
        soft_block=True
    )

    # Should not raise
    with patch('logging.Logger.warning') as mock_log:
        proxy._handle_block_decision(mock_event, mock_result)

        # Verify warning was logged
        mock_log.assert_called_once()
        assert "SOFT-BLOCK" in str(mock_log.call_args)

def test_hard_block_raises():
    """Test that hard-block mode raises on violations."""
    mock_graph = Mock()
    mock_event = Mock(tool_name="execute_query", id="event-123")
    mock_result = Mock(decision=0, slice_similarities={"action": 0.5})

    proxy = SecureGraphProxy(
        graph=mock_graph,
        agent_id="test-agent",
        boundary_id="test-boundary",
        soft_block=False  # Hard-block (default)
    )

    with pytest.raises(PermissionError):
        proxy._handle_block_decision(mock_event, mock_result)

def test_custom_soft_block_handler():
    """Test custom soft-block handler."""
    mock_graph = Mock()
    mock_event = Mock()
    mock_result = Mock(decision=0)

    custom_handler = Mock()

    proxy = SecureGraphProxy(
        graph=mock_graph,
        agent_id="test-agent",
        boundary_id="test-boundary",
        soft_block=True,
        on_soft_block=custom_handler
    )

    proxy._handle_block_decision(mock_event, mock_result)

    # Verify custom handler was called
    custom_handler.assert_called_once_with(mock_event, mock_result)
```

**Step 2: Run test to verify it fails**

Run: `cd tupl_sdk/python && pytest tests/test_soft_block.py::test_soft_block_logs_without_raising -v`

Expected: FAIL - soft_block parameter not accepted

**Step 3: Implement soft-block mode**

```python
# Modify tupl_sdk/python/tupl/agent.py
from typing import Callable, Optional

class SecureGraphProxy:
    def __init__(
        self,
        # ... existing params
        soft_block: bool = False,  # NEW
        on_soft_block: Optional[Callable] = None,  # NEW
    ):
        # ... existing init
        self.soft_block = soft_block
        self.on_soft_block = on_soft_block or self._default_soft_block_handler

    def _default_soft_block_handler(self, event, result):
        """Log violation without halting execution."""
        logger.warning(
            f"SOFT-BLOCK: Tool call '{event.tool_name}' would be blocked "
            f"by boundary '{self.boundary_id}'. "
            f"Intent ID: {event.id}, Similarities: {result.slice_similarities}"
        )

    def _handle_block_decision(self, event, result):
        """Handle block decision based on soft/hard block mode."""
        if result.decision == 0:  # BLOCK
            if self.soft_block:
                # Soft-block: Log and continue
                self.on_soft_block(event, result)
            else:
                # Hard-block: Raise exception
                if self.on_violation:
                    self.on_violation(event, result)
                raise PermissionError(
                    f"Tool call '{event.tool_name}' blocked by boundary '{self.boundary_id}'. "
                    f"Similarities: {result.slice_similarities}"
                )

    def invoke(self, state, config=None):
        # ... existing code ...

        # Get enforcement result
        result = self._check_enforcement(event)

        # Handle decision
        self._handle_block_decision(event, result)

        # Continue execution if ALLOW or soft-block
        # ... existing code ...
```

**Step 4: Run tests to verify they pass**

Run: `cd tupl_sdk/python && pytest tests/test_soft_block.py -v`

Expected: PASS

**Step 5: Commit**

```bash
git add tupl_sdk/python/tupl/agent.py tupl_sdk/python/tests/test_soft_block.py
git commit -m "feat: add soft-block enforcement mode to SDK"
```

---

## Task 10: MCP Gateway - Client Updates

**Files:**
- Modify: `mcp-gateway/src/tupl/clients/management-plane.ts`
- Create: `mcp-gateway/src/tupl/clients/management-plane.test.ts`

**Step 1: Write test for new client methods**

```typescript
// src/tupl/clients/management-plane.test.ts
import { ManagementPlaneClient } from './management-plane';

describe('ManagementPlaneClient - Agent APIs', () => {
  let client: ManagementPlaneClient;

  beforeEach(() => {
    client = new ManagementPlaneClient({
      baseUrl: 'http://localhost:8000',
      apiKey: 'test-key'
    });
  });

  test('listRegisteredAgents calls correct endpoint', async () => {
    const mockFetch = jest.fn().mockResolvedValue({
      ok: true,
      json: async () => ({ total: 1, agents: [{ agent_id: 'test' }] })
    });
    global.fetch = mockFetch;

    await client.listRegisteredAgents();

    expect(mockFetch).toHaveBeenCalledWith(
      'http://localhost:8000/api/v1/agents/list',
      expect.objectContaining({ method: 'GET' })
    );
  });

  test('createAgentPolicy sends correct payload', async () => {
    const mockFetch = jest.fn().mockResolvedValue({
      ok: true,
      json: async () => ({ id: 'policy-123' })
    });
    global.fetch = mockFetch;

    await client.createAgentPolicy({
      agent_id: 'my-agent',
      template_id: 'database_read_only',
      template_text: 'Allow reading',
      customization: 'only public'
    });

    expect(mockFetch).toHaveBeenCalledWith(
      'http://localhost:8000/api/v1/agents/policies',
      expect.objectContaining({
        method: 'POST',
        body: expect.stringContaining('my-agent')
      })
    );
  });
});
```

**Step 2: Run test to verify it fails**

Run: `cd mcp-gateway && npm test -- management-plane.test.ts`

Expected: FAIL - methods not defined

**Step 3: Implement client methods**

```typescript
// Modify src/tupl/clients/management-plane.ts
export class ManagementPlaneClient {
  // ... existing code

  async listRegisteredAgents(params?: {
    tenant_id?: string;
    limit?: number;
    offset?: number;
  }): Promise<{
    total: number;
    agents: Array<{
      id: string;
      agent_id: string;
      last_seen: string;
      sdk_version?: string;
    }>;
  }> {
    const queryParams = new URLSearchParams();
    if (params?.tenant_id) queryParams.set('tenant_id', params.tenant_id);
    if (params?.limit) queryParams.set('limit', params.limit.toString());
    if (params?.offset) queryParams.set('offset', params.offset.toString());

    const url = `${this.baseUrl}/api/v1/agents/list?${queryParams}`;
    const response = await fetch(url, {
      method: 'GET',
      headers: this.headers
    });

    if (!response.ok) {
      throw new Error(`Failed to list agents: ${response.statusText}`);
    }

    return response.json();
  }

  async listTemplates(params?: {
    category?: string;
  }): Promise<{
    templates: Array<{
      id: string;
      name: string;
      description: string;
      category: string;
      template_text: string;
      example_customizations: string[];
    }>;
  }> {
    const queryParams = new URLSearchParams();
    if (params?.category) queryParams.set('category', params.category);

    const url = `${this.baseUrl}/api/v1/agents/templates?${queryParams}`;
    const response = await fetch(url, {
      method: 'GET',
      headers: this.headers
    });

    if (!response.ok) {
      throw new Error(`Failed to list templates: ${response.statusText}`);
    }

    return response.json();
  }

  async createAgentPolicy(input: {
    agent_id: string;
    template_id: string;
    template_text: string;
    customization?: string;
  }): Promise<{
    id: string;
    agent_id: string;
    template_id: string;
    policy_rules: any;
    created_at: string;
  }> {
    const response = await fetch(`${this.baseUrl}/api/v1/agents/policies`, {
      method: 'POST',
      headers: this.headers,
      body: JSON.stringify(input)
    });

    if (!response.ok) {
      const error = await response.json();
      throw new Error(`Failed to create policy: ${JSON.stringify(error)}`);
    }

    return response.json();
  }

  async getAgentPolicy(agentId: string): Promise<{
    id: string;
    agent_id: string;
    template_id: string;
    customization?: string;
    policy_rules: any;
    created_at: string;
  }> {
    const response = await fetch(
      `${this.baseUrl}/api/v1/agents/policies/${agentId}`,
      {
        method: 'GET',
        headers: this.headers
      }
    );

    if (!response.ok) {
      throw new Error(`Failed to get policy: ${response.statusText}`);
    }

    return response.json();
  }
}
```

**Step 4: Run tests to verify they pass**

Run: `cd mcp-gateway && npm test -- management-plane.test.ts`

Expected: PASS

**Step 5: Commit**

```bash
git add mcp-gateway/src/tupl/clients/management-plane.ts mcp-gateway/src/tupl/clients/management-plane.test.ts
git commit -m "feat: add agent policy client methods to MCP gateway"
```

---

## Task 11: MCP Gateway - Agent Policy Tools

**Files:**
- Create: `mcp-gateway/src/tupl/tools/agent-policy.ts`
- Create: `mcp-gateway/src/tupl/tools/agent-policy.test.ts`
- Modify: `mcp-gateway/src/tupl/tools/index.ts`

**Step 1: Write test for tool implementations**

```typescript
// src/tupl/tools/agent-policy.test.ts
import { configureAgentPolicy, listRegisteredAgents, listPolicyTemplates, getAgentPolicy } from './agent-policy';
import { ManagementPlaneClient } from '../clients/management-plane';

describe('Agent Policy Tools', () => {
  let mockClient: jest.Mocked<ManagementPlaneClient>;

  beforeEach(() => {
    mockClient = {
      listRegisteredAgents: jest.fn(),
      listTemplates: jest.fn(),
      createAgentPolicy: jest.fn(),
      getAgentPolicy: jest.fn()
    } as any;
  });

  test('configureAgentPolicy formats response correctly', async () => {
    mockClient.createAgentPolicy.mockResolvedValue({
      id: 'policy-123',
      agent_id: 'test-agent',
      template_id: 'database_read_only',
      policy_rules: { thresholds: {} },
      created_at: '2025-11-23T00:00:00Z'
    });

    const result = await configureAgentPolicy(
      mockClient,
      'tenant-123',
      'test-agent',
      'database_read_only',
      'only public data'
    );

    expect(result).toContain('✅ Policy configured');
    expect(result).toContain('test-agent');
  });
});
```

**Step 2: Run test to verify it fails**

Run: `cd mcp-gateway && npm test -- agent-policy.test.ts`

Expected: FAIL - module not found

**Step 3: Implement agent policy tools**

```typescript
// src/tupl/tools/agent-policy.ts
import { ManagementPlaneClient } from '../clients/management-plane';

export async function configureAgentPolicy(
  client: ManagementPlaneClient,
  tenantId: string,
  agentId: string,
  templateId: string,
  customization?: string
): Promise<string> {
  const template = await client.listTemplates();
  const selectedTemplate = template.templates.find(t => t.id === templateId);

  if (!selectedTemplate) {
    throw new Error(`Template '${templateId}' not found`);
  }

  const response = await client.createAgentPolicy({
    agent_id: agentId,
    template_id: templateId,
    template_text: selectedTemplate.template_text,
    customization: customization || undefined
  });

  return (
    `✅ Policy configured for agent "${agentId}":\n\n` +
    `Template: ${templateId}\n` +
    `Customization: ${customization || 'none'}\n\n` +
    `Generated Policy:\n${JSON.stringify(response.policy_rules, null, 2)}`
  );
}

export async function listRegisteredAgents(
  client: ManagementPlaneClient,
  tenantId: string
): Promise<string> {
  const response = await client.listRegisteredAgents({ tenant_id: tenantId });

  if (response.total === 0) {
    return 'No registered agents found. Agents are auto-registered when wrapped with enforcement_agent().';
  }

  const agentList = response.agents
    .map(a => `- ${a.agent_id} (last seen: ${new Date(a.last_seen).toLocaleString()})`)
    .join('\n');

  return `Registered Agents (${response.total}):\n\n${agentList}`;
}

export async function listPolicyTemplates(
  client: ManagementPlaneClient,
  category?: string
): Promise<string> {
  const response = await client.listTemplates(category ? { category } : undefined);

  const templateList = response.templates
    .map(t =>
      `**${t.name}** (${t.id})\n` +
      `Category: ${t.category}\n` +
      `Description: ${t.description}\n` +
      `Template: "${t.template_text}"\n` +
      `Example Customizations:\n${t.example_customizations.map(ex => `  - ${ex}`).join('\n')}`
    )
    .join('\n\n---\n\n');

  return `Available Policy Templates:\n\n${templateList}`;
}

export async function getAgentPolicy(
  client: ManagementPlaneClient,
  tenantId: string,
  agentId: string
): Promise<string> {
  try {
    const policy = await client.getAgentPolicy(agentId);

    return (
      `Policy for agent "${agentId}":\n\n` +
      `Template: ${policy.template_id}\n` +
      `Customization: ${policy.customization || 'none'}\n` +
      `Created: ${new Date(policy.created_at).toLocaleString()}\n\n` +
      `Policy Rules:\n${JSON.stringify(policy.policy_rules, null, 2)}`
    );
  } catch (error) {
    return `No policy configured for agent "${agentId}". Use configure_agent_policy to create one.`;
  }
}
```

**Step 4: Register tools in index**

```typescript
// Modify src/tupl/tools/index.ts
import {
  configureAgentPolicy,
  listRegisteredAgents,
  listPolicyTemplates,
  getAgentPolicy
} from './agent-policy';

export const tools = [
  // ... existing tools
  {
    name: 'configure_agent_policy',
    description: 'Configure natural language security policy for an agent',
    inputSchema: {
      type: 'object',
      required: ['agent_id', 'template_id'],
      properties: {
        agent_id: { type: 'string', description: 'Agent identifier' },
        template_id: {
          type: 'string',
          description: 'Template ID from list_policy_templates'
        },
        customization: {
          type: 'string',
          description: 'Optional natural language customization'
        }
      }
    },
    handler: configureAgentPolicy
  },
  {
    name: 'list_registered_agents',
    description: 'List all registered agents for the current tenant',
    inputSchema: {
      type: 'object',
      properties: {}
    },
    handler: listRegisteredAgents
  },
  {
    name: 'list_policy_templates',
    description: 'List all available natural language policy templates',
    inputSchema: {
      type: 'object',
      properties: {
        category: {
          type: 'string',
          enum: ['database', 'file', 'api', 'general'],
          description: 'Optional category filter'
        }
      }
    },
    handler: listPolicyTemplates
  },
  {
    name: 'get_agent_policy',
    description: 'View the current natural language policy for an agent',
    inputSchema: {
      type: 'object',
      required: ['agent_id'],
      properties: {
        agent_id: { type: 'string', description: 'Agent identifier' }
      }
    },
    handler: getAgentPolicy
  }
];
```

**Step 5: Run tests to verify they pass**

Run: `cd mcp-gateway && npm test -- agent-policy.test.ts`

Expected: PASS

**Step 6: Commit**

```bash
git add mcp-gateway/src/tupl/tools/agent-policy.ts mcp-gateway/src/tupl/tools/agent-policy.test.ts mcp-gateway/src/tupl/tools/index.ts
git commit -m "feat: add agent policy management tools to MCP gateway"
```

---

## Task 12: Integration Testing

**Files:**
- Create: `tests/integration/test_nl_guardrails_e2e.py`

**Step 1: Write end-to-end integration test**

```python
# tests/integration/test_nl_guardrails_e2e.py
import pytest
import httpx
from tupl.agent import enforcement_agent

@pytest.mark.integration
async def test_complete_nl_guardrails_flow():
    """Test complete flow: SDK registration → policy creation → enforcement."""
    base_url = "http://localhost:8000"
    token = "test-token"
    tenant_id = "test-tenant"

    # Step 1: Create agent with auto-registration
    mock_graph = Mock()
    agent = enforcement_agent(
        agent=mock_graph,
        agent_id="integration-test-agent",
        boundary_id="test-boundary",
        tenant_id=tenant_id,
        token=token,
        base_url=base_url,
        soft_block=True
    )

    # Step 2: Verify agent was registered
    response = httpx.get(
        f"{base_url}/api/v1/agents/list",
        headers={"Authorization": f"Bearer {token}"}
    )
    assert response.status_code == 200
    agents = response.json()["agents"]
    assert any(a["agent_id"] == "integration-test-agent" for a in agents)

    # Step 3: Create policy via API
    response = httpx.post(
        f"{base_url}/api/v1/agents/policies",
        json={
            "agent_id": "integration-test-agent",
            "template_id": "database_read_only",
            "template_text": "Allow reading from databases",
            "customization": "only public data"
        },
        headers={"Authorization": f"Bearer {token}"}
    )
    assert response.status_code == 200
    policy = response.json()
    assert policy["template_id"] == "database_read_only"
    assert "read" in policy["policy_rules"]["constraints"]["action"]["actions"]

    # Step 4: Trigger enforcement (would test actual LangGraph invocation)
    # This is a placeholder - actual enforcement testing depends on your setup
    assert agent.agent_id == "integration-test-agent"
    assert agent.soft_block == True
```

**Step 2: Run test**

Run: `pytest tests/integration/test_nl_guardrails_e2e.py -v`

Expected: PASS (assuming services are running)

**Step 3: Commit**

```bash
git add tests/integration/test_nl_guardrails_e2e.py
git commit -m "test: add end-to-end integration test for NL guardrails"
```

---

## Task 13: Documentation

**Files:**
- Create: `docs/guides/natural-language-guardrails.md`
- Modify: `README.md`

**Step 1: Write user guide**

```markdown
# docs/guides/natural-language-guardrails.md
# Natural Language Guardrails Guide

## Overview

Natural language guardrails allow you to configure AI agent security policies using simple templates and plain English customizations, without dealing with complex threshold configurations.

## Quick Start

### 1. Wrap Your Agent

```python
from tupl import enforcement_agent

secure_agent = enforcement_agent(
    agent=my_langgraph_agent,
    agent_id="my-analytics-agent",  # Unique identifier
    boundary_id="analytics-policy",
    tenant_id="your-tenant-id",
    token="your-api-token",
    soft_block=True  # Development mode - log violations without blocking
)
```

The agent automatically registers with the management plane on first use.

### 2. Configure Policy (Web UI)

1. Navigate to **Agent Policies** page
2. Select your agent from the dropdown
3. Browse template cards (e.g., "Database Read-Only Access")
4. Add optional customization: "only public data from analytics_db"
5. Click **Create Policy**

### 3. Configure Policy (via Claude/MCP)

```
You: "Show me my registered agents"
Claude: *lists agents*

You: "What policy templates are available?"
Claude: *shows template library*

You: "Configure my-analytics-agent to only read public data from databases"
Claude: *creates policy using template + customization*
```

### 4. Test & Iterate

- Review enforcement decisions in **Telemetry** page
- Adjust customization based on actual violations
- Switch to hard-block mode when ready: `soft_block=False`

## Available Templates

- **database_read_only**: Read-only database access
- **database_write_access**: Database write permissions
- **file_export**: File export capabilities
- **api_read_access**: External API read access
- **unrestricted**: Full access (use with caution)

## Customization Examples

- "only from analytics_db"
- "excluding PII fields"
- "only public and internal data"
- "with authentication required"
- "maximum 1000 records at a time"

## Best Practices

1. **Start with soft-block mode** during development
2. **Use specific templates** rather than "unrestricted"
3. **Review telemetry** before enabling hard-block
4. **Customize incrementally** - start simple, add constraints as needed
5. **Test enforcement** with representative workloads

## Troubleshooting

**Agent not showing in dropdown?**
- Ensure you've run the agent at least once with `enforcement_agent()`
- Check that `agent_id` parameter is set

**Policy violations not logging?**
- Verify `soft_block=True` is set
- Check application logs for SOFT-BLOCK warnings

**LLM parsing errors?**
- Try rephrasing your customization
- Use a different base template
- Check that you're using vocabulary terms (read/write, public/confidential, etc.)
```

**Step 2: Update README**

```markdown
# Add to README.md

## Natural Language Guardrails

Configure agent security policies using templates and plain English:

```python
# Wrap your agent
secure_agent = enforcement_agent(
    agent=my_agent,
    agent_id="analytics-agent",
    boundary_id="analytics-policy",
    soft_block=True  # Development mode
)

# Configure via UI or MCP
"Configure analytics-agent to only read public data from databases"
# → Automatically generates structured policy
```

See [Natural Language Guardrails Guide](docs/guides/natural-language-guardrails.md) for details.
```

**Step 3: Commit**

```bash
git add docs/guides/natural-language-guardrails.md README.md
git commit -m "docs: add natural language guardrails user guide"
```

---

## Summary

**Implementation complete! The natural language guardrails feature includes:**

1. ✅ Database schema with agent registration and policy storage
2. ✅ Policy template library (5 initial templates)
3. ✅ LLM-based policy parser using Gemini
4. ✅ Management Plane API endpoints (agents, policies, templates)
5. ✅ SDK auto-registration and soft-block mode
6. ✅ MCP Gateway tools for Claude integration
7. ✅ Integration tests
8. ✅ User documentation

**Total commits**: 13 bite-sized commits following TDD approach

**Next steps**:
- Deploy to staging environment
- Gather user feedback on templates
- Add more templates based on usage patterns
- Consider Web UI implementation (not included in this plan)

---

Plan complete and saved to `docs/plans/2025-11-23-natural-language-guardrails-implementation.md`.

**Two execution options:**

**1. Subagent-Driven (this session)** - I dispatch fresh subagent per task, review between tasks, fast iteration

**2. Parallel Session (separate)** - Open new session with executing-plans, batch execution with checkpoints

**Which approach?**
