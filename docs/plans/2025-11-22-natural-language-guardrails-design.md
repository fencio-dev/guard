# Natural Language Guardrails Design

**Date**: 2025-11-22
**Status**: Design Approved
**Approach**: LLM-First with Direct Policy Generation

---

## Overview

This design introduces natural language guardrails for Tupl, allowing users to configure agent security policies using simple, high-level statements instead of complex rule configurations. Users select from a template library and optionally customize with natural language, which the system converts to structured policies using an LLM module.

### Key Features

- **Template-driven configuration**: Users select from predefined templates (e.g., "Database Read-Only Access")
- **Natural language customization**: Optional freeform text to refine templates (e.g., "only analytics_db")
- **Agent-level policies**: One policy per agent with automatic agent discovery
- **LLM-based parsing**: Gemini converts natural language to structured BoundaryRules
- **Vocabulary-grounded**: Uses existing canonical vocabulary for semantic alignment
- **Soft-block mode**: Log violations without halting execution for development/testing
- **Multi-channel configuration**: Via web UI or MCP Gateway tools
- **Tenant isolation**: Full RLS policies for multi-tenant security

---

## Motivation

### Current State (v0.9.0)

Users configure policies by manually specifying:
- BoundaryRules with thresholds (action: 0.85, resource: 0.80, etc.)
- Constraints with vocabulary values (actions: ["read"], resource_types: ["database"])
- Decision modes ("min" vs "weighted-avg")

**Problems:**
1. Too technical for non-experts
2. Requires understanding of semantic security concepts
3. Trial-and-error to get thresholds right
4. No agent discovery mechanism
5. Hard-block only (no development mode)

### Desired State

Users configure policies by:
1. Selecting agent from auto-discovered list
2. Browsing template cards
3. Optionally adding natural language customization
4. System generates complete policy automatically

**Benefits:**
- Accessible to non-technical users
- Predictable behavior via templates
- Flexible customization via natural language
- Automatic agent registration
- Soft-block mode for safe iteration

---

## Architecture

### High-Level Flow

```
┌─────────────────────────────────────────────────────────────┐
│ User Interface (Web Console / MCP Gateway)                  │
│ • Template Library (predefined examples)                    │
│ • Customization Text Box                                    │
│ • Per-Agent Configuration                                   │
└────────────────────┬────────────────────────────────────────┘
                     ↓
┌─────────────────────────────────────────────────────────────┐
│ Natural Language Policy Parser (NEW)                        │
│ • LLM Module (Gemini 2.5 Flash Lite)                       │
│ • Input: Template + Customization + Vocabulary              │
│ • Output: PolicyRules JSON (constraints + thresholds)       │
│ • Tenant-scoped storage                                     │
└────────────────────┬────────────────────────────────────────┘
                     ↓
┌─────────────────────────────────────────────────────────────┐
│ Policy Storage (Database)                                   │
│ • registered_agents table (auto-discovered agents)          │
│ • agent_policies table (tenant_id, agent_id, policy_json)  │
│ • RLS policies for tenant isolation                         │
└────────────────────┬────────────────────────────────────────┘
                     ↓
┌─────────────────────────────────────────────────────────────┐
│ Existing Encoding Pipeline (REUSE)                          │
│ • llm_anchor_generator.py (vocabulary → anchors)            │
│ • encoding.py (anchors → embeddings)                        │
│ • Data Plane enforcement (soft-block mode support)          │
└─────────────────────────────────────────────────────────────┘
```

### Components

#### 1. Agent Discovery & Registration

**Purpose**: Automatically register agents when wrapped with `enforcement_agent()`

**SDK Changes** (`tupl_sdk/python/tupl/agent.py`):
```python
class SecureGraphProxy:
    def __init__(
        self,
        graph: Any,
        agent_id: str,  # NEW: Required parameter
        boundary_id: str,
        # ... other params
    ):
        self.agent_id = agent_id
        # ... existing init

        # NEW: Auto-register agent
        self._register_agent()

    def _register_agent(self):
        """Auto-register agent with Management Plane."""
        try:
            response = httpx.post(
                f"{self.base_url}/api/v1/agents/register",
                json={
                    "agent_id": self.agent_id,
                    "tenant_id": self.tenant_id,
                    "sdk_version": version("tupl"),
                    "metadata": {}
                },
                headers={"Authorization": f"Bearer {self.token}"},
                timeout=2.0
            )
            logger.info(f"Agent '{self.agent_id}' registered")
        except Exception as e:
            # Non-critical - don't break enforcement
            logger.debug(f"Agent registration failed: {e}")
```

**Database Schema**:
```sql
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

-- RLS policies
ALTER TABLE registered_agents ENABLE ROW LEVEL SECURITY;

CREATE POLICY "Users can view their own agents"
    ON registered_agents FOR SELECT
    USING (auth.uid() = tenant_id);

CREATE POLICY "Users can register their own agents"
    ON registered_agents FOR INSERT
    WITH CHECK (auth.uid() = tenant_id);
```

#### 2. Natural Language Policy Parser

**Purpose**: Convert template + customization to structured PolicyRules

**Module** (`management-plane/app/nl_policy_parser.py`):

**Key Requirements**:
- **Gemini Structured Outputs**: All nested objects must be explicit Pydantic models (no `Dict[str, Any]`)
- **Vocabulary validation**: Ensure LLM uses only canonical vocabulary values
- **Caching**: Hash-based cache like existing `llm_anchor_generator.py`
- **Error handling**: User-friendly messages on parse failures

**Schema Design**:
```python
# All nested models explicitly defined for Gemini compatibility

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
    """
    Note: 'effect' field removed - all policies are ALLOW-only.
    System is fail-closed by default.
    """
    thresholds: SliceThresholds
    decision: Literal["min", "weighted-avg"] = "min"
    globalThreshold: Optional[float] = None
    constraints: PolicyConstraints
```

**LLM Prompt Template**:
```python
prompt = f"""You are a security policy generator for an AI agent guardrail system.

INPUT:
Template: {template_text}
Customization: {customization or "none"}

CANONICAL VOCABULARY:
Actions: {vocab.get_valid_actions()}  # [read, write, delete, export, execute]
Resource Types: {vocab.get_valid_resource_types()}  # [database, file, api]
Sensitivity Levels: {vocab.get_sensitivity_levels()}  # [public, internal, confidential]
Volumes: {vocab.get_volumes()}  # [single, bulk]
Authn Levels: {vocab.get_authn_levels()}  # [required, not_required]

TASK:
Generate a PolicyRules object that represents an ALLOW policy for this guardrail.

RULES:
1. Use ONLY vocabulary values listed above
2. Set default thresholds: action=0.85, resource=0.80, data=0.75, risk=0.70
3. Use "min" decision mode by default
4. Extract constraints from the natural language

OUTPUT: Return JSON matching the PolicyRules schema.
"""
```

**Implementation**:
```python
class NLPolicyParser:
    def __init__(self, api_key: str):
        self.client = genai.Client(api_key=api_key)
        self.model = "gemini-2.5-flash-lite"
        self._cache: dict[str, PolicyRules] = {}
        self.vocab = VOCABULARY

    async def parse_policy(
        self,
        template_id: str,
        template_text: str,
        customization: Optional[str]
    ) -> PolicyRules:
        # Check cache
        cache_key = self._compute_cache_key(template_id, customization)
        if cache_key in self._cache:
            return self._cache[cache_key]

        # Call Gemini with structured output
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
        return policy_rules
```

#### 3. Template Library

**Purpose**: Provide predefined policy templates for common scenarios

**Module** (`management-plane/app/policy_templates.py`):

```python
class PolicyTemplate(BaseModel):
    id: str
    name: str
    description: str
    template_text: str
    category: str  # "database", "file", "api", "general"
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
    # More templates...
]
```

#### 4. Policy Storage

**Database Schema**:
```sql
CREATE TABLE agent_policies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    agent_id TEXT NOT NULL,

    -- Natural language inputs
    template_id TEXT NOT NULL,
    template_text TEXT NOT NULL,
    customization TEXT,

    -- Generated policy (stored as JSONB)
    policy_rules JSONB NOT NULL,

    -- Metadata
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Foreign key to registered agents
    FOREIGN KEY (tenant_id, agent_id)
        REFERENCES registered_agents(tenant_id, agent_id)
        ON DELETE CASCADE,

    UNIQUE(tenant_id, agent_id)
);

-- RLS policies
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

-- Indexes
CREATE INDEX idx_agent_policies_tenant_id ON agent_policies(tenant_id);
CREATE INDEX idx_agent_policies_agent_id ON agent_policies(agent_id);
CREATE INDEX idx_agent_policies_tenant_agent ON agent_policies(tenant_id, agent_id);
```

#### 5. Soft-Block Enforcement Mode

**Purpose**: Allow agents to run with policy violations logged but not blocked

**SDK Changes** (`tupl_sdk/python/tupl/agent.py`):

```python
class SecureGraphProxy:
    def __init__(
        self,
        # ... existing params
        soft_block: bool = False,  # NEW
        on_soft_block: Optional[Callable] = None,  # NEW
    ):
        self.soft_block = soft_block
        self.on_soft_block = on_soft_block or self._default_soft_block_handler

    def _default_soft_block_handler(self, event: IntentEvent, result: ComparisonResult):
        """Log violation without halting execution."""
        logger.warning(
            f"SOFT-BLOCK: Tool call '{event.tool_name}' would be blocked "
            f"by boundary '{self.boundary_id}'. "
            f"Intent ID: {event.id}, Similarities: {result.slice_similarities}"
        )

    def invoke(self, state, config=None):
        # ... existing code ...

        if result.decision == 0:  # BLOCK
            if self.soft_block:
                # Soft-block: Log and continue
                self.on_soft_block(event, result)
                # Continue execution - don't raise
            else:
                # Hard-block: Raise exception
                if self.on_violation:
                    self.on_violation(event, result)
                raise PermissionError(...)
```

**Usage**:
```python
# Development: soft-block mode
secure_agent = enforcement_agent(
    agent=base_agent,
    agent_id="dev-agent",
    boundary_id="dev-policy",
    soft_block=True  # Log violations, don't halt
)

# Production: hard-block mode (default)
secure_agent = enforcement_agent(
    agent=prod_agent,
    agent_id="prod-agent",
    boundary_id="prod-policy"
    # soft_block defaults to False
)
```

---

## API Endpoints

### Agent Management

**POST /api/v1/agents/register**
```json
Request:
{
  "agent_id": "my-agent",
  "tenant_id": "user-123",
  "sdk_version": "1.3.0",
  "metadata": {}
}

Response:
{
  "id": "uuid",
  "agent_id": "my-agent",
  "first_seen": "2025-11-22T...",
  "last_seen": "2025-11-22T..."
}
```

**GET /api/v1/agents/list**
```json
Query Params: ?tenant_id=user-123&limit=100&offset=0

Response:
{
  "total": 3,
  "agents": [
    {
      "agent_id": "analytics-agent",
      "last_seen": "2025-11-22T...",
      "sdk_version": "1.3.0"
    }
  ]
}
```

### Policy Management

**POST /api/v1/agents/policies**
```json
Request:
{
  "agent_id": "my-agent",
  "template_id": "database_read_only",
  "template_text": "Allow reading from databases",
  "customization": "only analytics_db with public data"
}

Response:
{
  "id": "uuid",
  "agent_id": "my-agent",
  "template_id": "database_read_only",
  "customization": "only analytics_db with public data",
  "policy_rules": {
    "thresholds": {...},
    "constraints": {...}
  },
  "created_at": "2025-11-22T..."
}
```

**GET /api/v1/agents/policies/{agent_id}**
```json
Response:
{
  "id": "uuid",
  "agent_id": "my-agent",
  "template_id": "database_read_only",
  "customization": "only analytics_db",
  "policy_rules": {...},
  "created_at": "2025-11-22T..."
}
```

**PUT /api/v1/agents/policies/{agent_id}**
```json
Request: (same as POST)
Response: (updated policy)
```

**DELETE /api/v1/agents/policies/{agent_id}**
```json
Response: { "success": true }
```

### Templates

**GET /api/v1/agents/templates**
```json
Query Params: ?category=database

Response:
{
  "templates": [
    {
      "id": "database_read_only",
      "name": "Database Read-Only Access",
      "description": "...",
      "template_text": "Allow reading from databases",
      "category": "database",
      "example_customizations": [...]
    }
  ]
}
```

**GET /api/v1/agents/templates/{template_id}**
```json
Response: (single template object)
```

---

## MCP Gateway Integration

### New Tools

**Tool: `configure_agent_policy`**
```typescript
{
  name: 'configure_agent_policy',
  description: 'Configure natural language security policy for an agent',
  inputSchema: {
    type: 'object',
    required: ['agent_id', 'template_id'],
    properties: {
      agent_id: { type: 'string' },
      template_id: { type: 'string', enum: [...] },
      customization: { type: 'string' }
    }
  }
}
```

**Tool: `list_registered_agents`**
```typescript
{
  name: 'list_registered_agents',
  description: 'List all registered agents for the current tenant',
  inputSchema: { type: 'object', properties: {} }
}
```

**Tool: `list_policy_templates`**
```typescript
{
  name: 'list_policy_templates',
  description: 'List all available natural language policy templates',
  inputSchema: {
    type: 'object',
    properties: {
      category: { type: 'string', enum: ['database', 'file', 'api', 'general'] }
    }
  }
}
```

**Tool: `get_agent_policy`**
```typescript
{
  name: 'get_agent_policy',
  description: 'View the current natural language policy for an agent',
  inputSchema: {
    type: 'object',
    required: ['agent_id'],
    properties: {
      agent_id: { type: 'string' }
    }
  }
}
```

### Implementation

**File**: `mcp-gateway/src/tupl/tools/agent-policy.ts`
```typescript
export async function configureAgentPolicy(
  client: ManagementPlaneClient,
  tenantId: string,
  agentId: string,
  templateId: string,
  customization?: string
): Promise<string> {
  const response = await client.createAgentPolicy({
    agent_id: agentId,
    template_id: templateId,
    customization: customization || null
  });

  return (
    `✅ Policy configured for agent "${agentId}":\n\n` +
    `Template: ${templateId}\n` +
    `Customization: ${customization || 'none'}\n\n` +
    `Generated Policy:\n${JSON.stringify(response.policy, null, 2)}`
  );
}
```

**Client Updates**: `mcp-gateway/src/tupl/clients/management-plane.ts`
```typescript
export class ManagementPlaneClient {
  // Add new methods for agent policy APIs
  async listRegisteredAgents(params?: { tenant_id?: string }): Promise<any>
  async listTemplates(params?: { category?: string }): Promise<any>
  async createAgentPolicy(input: {...}): Promise<any>
  async getAgentPolicy(agentId: string): Promise<any>
}
```

---

## User Experience

### Web UI Flow

1. **Navigate to "Agent Policies" page**
2. **Select Agent** from dropdown (auto-populated with registered agents)
   - Shows: agent_id + last_seen timestamp
3. **Browse Templates** as visual cards
   - Cards show: icon, name, description, template text
   - Organized by category (database, file, api, general)
4. **Select Template** (card highlights)
5. **Customize** (optional)
   - Text area with placeholder examples
   - Shows example customizations from template
6. **Create Policy**
   - Shows generated PolicyRules in preview
   - Confirms creation
7. **View in Telemetry**
   - Navigate to telemetry page
   - Filter by agent_id to see enforcement decisions

### MCP Flow (via Claude)

```
User: "Show me my registered agents"
Claude: *calls list_registered_agents*
→ - analytics-agent (last seen: 2 minutes ago)
  - export-agent (last seen: 1 hour ago)

User: "What policy templates are available?"
Claude: *calls list_policy_templates*
→ [Shows template cards with descriptions]

User: "Configure my analytics-agent to only read from public databases"
Claude: *calls configure_agent_policy with:
  agent_id: "analytics-agent"
  template_id: "database_read_only"
  customization: "only public data"
*
→ ✅ Policy configured for agent "analytics-agent":
  Template: database_read_only
  Customization: only public data

  Generated Policy:
  {
    "thresholds": {...},
    "constraints": {
      "action": {"actions": ["read"]},
      "resource": {"types": ["database"]},
      "data": {"sensitivity": ["public"]}
    }
  }

User: "Show me the current policy for analytics-agent"
Claude: *calls get_agent_policy*
→ [Full policy details]
```

---

## Error Handling

### 1. Agent Not Registered
```python
raise HTTPException(
    status_code=404,
    detail={
        "error": "agent_not_registered",
        "message": f"Agent '{agent_id}' not found. Wrap with enforcement_agent() first.",
        "agent_id": agent_id
    }
)
```

### 2. LLM Parsing Failure
```python
raise ValueError(
    "Failed to parse policy from natural language. "
    "Please try rephrasing your customization or use a different template."
)
```

### 3. Vocabulary Violation
```python
raise ValueError(
    f"Invalid action '{action}'. "
    f"Must be one of: {', '.join(VOCABULARY.get_valid_actions())}"
)
```

### 4. No Policy Configured (Enforcement Time)
```python
if not policy:
    if self.soft_block:
        logger.warning("Soft-block: allowing operation despite no policy")
    else:
        raise PermissionError(
            f"No security policy configured for agent '{self.agent_id}'. "
            "Configure a policy via UI or MCP before using this agent."
        )
```

### 5. Template Not Found
```python
raise HTTPException(
    status_code=404,
    detail={
        "error": "template_not_found",
        "message": f"Template '{template_id}' not found.",
        "available_templates": [t.id for t in POLICY_TEMPLATES]
    }
)
```

### 6. SDK Registration Failures (Non-Critical)
```python
# Never raise - registration is optional
try:
    response = httpx.post(...)
except Exception as e:
    logger.debug(f"Agent registration failed: {e} (non-critical)")
```

---

## Testing Strategy

### Unit Tests

1. **NL Policy Parser**
   - Parse simple templates
   - Parse with customization
   - Vocabulary validation
   - Caching behavior

2. **Agent Registration**
   - Register new agent
   - Update last_seen on duplicate
   - Cross-tenant isolation

3. **Policy CRUD**
   - Create for unregistered agent (should fail)
   - Create and retrieve
   - Update policy
   - Delete policy

### Integration Tests

4. **End-to-End Policy Enforcement**
   - Register agent via SDK
   - Configure policy via API
   - Trigger enforcement (ALLOW/BLOCK scenarios)

5. **Soft-Block Mode**
   - Verify violations logged without raising
   - Custom handler invocation

### UI Tests

6. **Template Selection Flow**
   - Select agent
   - Select template card
   - Add customization
   - Create policy
   - Verify success message

---

## Migration Path

### Phase 1: Development & Testing
1. User wraps agent with `soft_block=True`
2. Configures natural language policy via UI/MCP
3. Observes violations in telemetry
4. Iterates on policy customization

### Phase 2: Refinement
1. Reviews telemetry patterns
2. Adjusts customization based on actual usage
3. Validates policy catches intended violations

### Phase 3: Production
1. Switches to `soft_block=False` (hard-block)
2. Policy now enforces with PermissionError on violations
3. Continues monitoring via telemetry

---

## Implementation Checklist

### SDK Changes
- [ ] Add `agent_id` parameter to `enforcement_agent()`
- [ ] Add `soft_block` and `on_soft_block` parameters
- [ ] Implement `_register_agent()` in `SecureGraphProxy`
- [ ] Update soft-block enforcement logic in `invoke()`

### Management Plane
- [ ] Create `nl_policy_parser.py` module
- [ ] Create `policy_templates.py` with initial templates
- [ ] Create `endpoints/agents.py` with all API routes
- [ ] Create `rule_encoding.py` bridge to existing pipeline
- [ ] Add database migrations for tables and RLS

### MCP Gateway
- [ ] Add new tool definitions to `tupl-tools.ts`
- [ ] Implement `tupl/tools/agent-policy.ts`
- [ ] Update `ManagementPlaneClient` with new methods

### Web UI
- [ ] Create `AgentPoliciesPage.tsx`
- [ ] Create `TemplateCard.tsx` component
- [ ] Create `lib/agent-api.ts` client
- [ ] Update navigation to include Agent Policies link

### Testing
- [ ] Unit tests for NL parser
- [ ] Unit tests for agent registration
- [ ] Unit tests for policy CRUD
- [ ] Integration tests for E2E enforcement
- [ ] UI tests for template flow

### Documentation
- [ ] Update README with natural language guardrails
- [ ] Add examples to SDK usage guide
- [ ] Document template library
- [ ] Update mental models

---

## Risks & Mitigations

### Risk 1: LLM Generates Invalid Policies
**Mitigation**:
- Strict Pydantic validation
- Vocabulary compliance checks
- Extensive prompt engineering
- Cache successful parses

### Risk 2: Template Library Insufficient
**Mitigation**:
- Start with 5-7 core templates
- Gather user feedback
- Iteratively add templates based on common patterns
- Allow freeform customization for edge cases

### Risk 3: Agent Discovery Failures
**Mitigation**:
- Make registration non-critical (don't break enforcement)
- Retry logic with exponential backoff
- Manual agent registration via UI as fallback

### Risk 4: Soft-Block Confusion
**Mitigation**:
- Clear documentation on when to use soft-block
- Default to hard-block (fail-safe)
- Prominent warnings in logs for soft-block mode

---

## Future Enhancements

### v1.1
- Advanced template editor (allow users to create custom templates)
- Template versioning and rollback
- Policy diff view (compare changes)

### v1.2
- Multi-policy per agent (layered policies)
- Workspace-level global policies
- Policy inheritance and overrides

### v1.3
- Visual policy editor (drag-and-drop constraints)
- Policy simulation (test before deploying)
- Automated policy recommendations based on telemetry

---

## References

- [Release v0.9.0](../releases/RELEASE_0.9.0.md)
- [System Overview](../models/00-system-overview.md)
- [Vocabulary Specification](../../vocabulary.yaml)
- [LLM Anchor Generator](../../management-plane/app/llm_anchor_generator.py)
- [Encoding Pipeline](../../management-plane/app/encoding.py)

---

**Status**: Design Complete ✅
**Next Steps**: Create implementation plan via writing-plans skill
