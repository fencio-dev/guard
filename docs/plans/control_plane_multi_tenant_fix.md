# Control Plane Multi-Tenant Security Fix

**Status**: 🔴 CRITICAL - Deferred to Post-MVP
**Priority**: HIGH
**Severity**: SECURITY VULNERABILITY
**Created**: 2025-11-20
**Target Release**: v1.1 (Post-MVP)

---

## Executive Summary

The Control Plane API currently has **no authentication or tenant isolation**, creating a critical security vulnerability where policies configured by one user can be accessed and manipulated by any other user. This must be fixed before the Control Plane can be safely exposed in production.

**Current State**: Control Plane is deployed but UI cannot connect due to missing environment configuration. This is **intentional** - we've blocked access until security is implemented.

**Immediate Mitigation**: Control Plane routes configured but UI has no `VITE_CONTROL_PLANE_URL` set, preventing user access to vulnerable endpoints.

---

## Security Risk Analysis

### Vulnerability Overview

| Risk Factor | Status | Impact |
|------------|--------|--------|
| Authentication | ❌ None | Any caller can access all endpoints |
| Authorization | ❌ None | No permission checks on operations |
| Tenant Isolation | ❌ None | All policies stored in global namespace |
| Data Leakage | 🔴 HIGH | Users can read other users' policies |
| Data Tampering | 🔴 HIGH | Users can modify/delete others' policies |
| Policy Hijacking | 🔴 HIGH | Attackers can configure victim agents |

### Attack Scenarios

#### Scenario 1: Cross-Tenant Policy Theft
```bash
# Attacker (no authentication required):
curl -X GET https://platform.tupl.xyz/api/control/api/v1/rules

# Returns ALL policies from ALL tenants
{
  "total": 50,
  "configurations": [
    {"agent_id": "victim_agent_1", "owner": "user_123", ...},
    {"agent_id": "victim_agent_2", "owner": "user_456", ...}
  ]
}
```

#### Scenario 2: Policy Modification Attack
```bash
# Attacker modifies victim's security rules:
curl -X POST https://platform.tupl.xyz/api/control/api/v1/agents/victim_agent/rules \
  -H "Content-Type: application/json" \
  -d '{
    "profile": {
      "agent_id": "victim_agent",
      "owner": "attacker",
      "rule_families": {
        "tool_whitelist": {
          "enabled": false  // Disable victim's security
        }
      }
    }
  }'

# Victim's agent now has no tool restrictions!
```

#### Scenario 3: Denial of Service
```bash
# Attacker deletes all policies:
curl -X DELETE https://platform.tupl.xyz/api/control/api/v1/agents/victim_agent/rules

# Victim's agent loses all configured security policies
```

---

## Root Cause Analysis

### 1. Data Model - No Tenant Context

**File**: `policy_control_plane/models.py`

```python
class AgentProfile(BaseModel):
    agent_id: str
    owner: str  # ❌ Just a string, no tenant validation
    description: Optional[str] = None
    rule_families: AgentRuleFamilies = Field(default_factory=AgentRuleFamilies)
```

**Problem**: The `owner` field is an arbitrary string with no relationship to actual user/tenant identities.

### 2. API Endpoints - No Authentication

**File**: `policy_control_plane/server.py`

```python
# ❌ No authentication decorator
@app.post("/api/v1/agents/{agent_id}/rules")
async def configure_agent_rules(agent_id: str, request: RuleConfigRequest):
    # No user context
    # No tenant validation
    # Anyone can modify any agent
    pass

@app.get("/api/v1/rules")
async def list_rule_configs():
    return agents_store  # ❌ Returns ALL agents globally!
```

**Compare to Management Plane** (which has auth):
```python
@router.post("/compare", response_model=ComparisonResult)
async def compare_intent(
    event: IntentEvent,
    current_user: User = Depends(get_current_user)  # ✅ Auth enforced
):
    # Has user context
    pass
```

### 3. Storage Layer - Global Namespace

**File**: `policy_control_plane/server.py`

```python
# In-memory storage - NO tenant scoping
agents_store: Dict[str, Dict] = {}  # Key is agent_id only

# List returns everything
async def list_rule_configs(agent_id: Optional[str] = None):
    if agent_id:
        return {agent_id: agents_store[agent_id]}  # No tenant check
    else:
        return agents_store  # ❌ ALL agents from ALL tenants
```

### 4. Integration Gap - No Tenant Context Passed

The gateway resolves tenant context but **never passes it to Control Plane**:

```typescript
// mcp-gateway/src/tenant-resolver.ts
async resolveTenantId(token?: string): Promise<string | null> {
  // Resolves token → user_id via Supabase
  return user_id;  // ✅ Has tenant context
}

// But Control Plane never receives this tenant_id!
```

---

## Required Changes

### Phase 1: Data Model (Estimated: 2 hours)

#### 1.1 Add tenant_id to AgentProfile

**File**: `policy_control_plane/models.py`

```python
class AgentProfile(BaseModel):
    agent_id: str
    tenant_id: str  # ✅ NEW - Validated tenant identifier
    owner: str      # Keep for display purposes
    description: Optional[str] = None
    rule_families: AgentRuleFamilies = Field(default_factory=AgentRuleFamilies)
```

#### 1.2 Update Storage Schema

**File**: `policy_control_plane/server.py`

```python
# Change storage key to include tenant_id
agents_store: Dict[str, Dict[str, Dict]] = {}  # {tenant_id: {agent_id: config}}

# Or use tuple keys
agents_store: Dict[Tuple[str, str], Dict] = {}  # {(tenant_id, agent_id): config}
```

### Phase 2: Authentication (Estimated: 3 hours)

#### 2.1 Add JWT Authentication Dependency

**File**: `policy_control_plane/server.py`

```python
from fastapi import Depends, HTTPException, status
from fastapi.security import HTTPBearer, HTTPAuthorizationCredentials
from jose import jwt, JWTError
import os

security = HTTPBearer()
SUPABASE_JWT_SECRET = os.getenv("SUPABASE_JWT_SECRET")

async def get_current_user(
    credentials: HTTPAuthorizationCredentials = Depends(security)
) -> str:
    """Extract and validate JWT token, return tenant_id (user_id)."""
    token = credentials.credentials

    try:
        payload = jwt.decode(
            token,
            SUPABASE_JWT_SECRET,
            algorithms=["HS256"],
            options={"verify_aud": False}
        )
        tenant_id = payload.get("sub")
        if not tenant_id:
            raise HTTPException(
                status_code=status.HTTP_401_UNAUTHORIZED,
                detail="Invalid token: no subject"
            )
        return tenant_id
    except JWTError:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="Invalid or expired token"
        )
```

#### 2.2 Add Auth to All Endpoints

```python
@app.post("/api/v1/agents/{agent_id}/rules")
async def configure_agent_rules(
    agent_id: str,
    request: RuleConfigRequest,
    tenant_id: str = Depends(get_current_user)  # ✅ Auth required
):
    # Validate tenant_id matches request
    if request.profile.tenant_id != tenant_id:
        raise HTTPException(status_code=403, detail="Forbidden")

    # Store with tenant scoping
    key = (tenant_id, agent_id)
    agents_store[key] = request.profile.dict()
    pass

@app.get("/api/v1/rules")
async def list_rule_configs(
    tenant_id: str = Depends(get_current_user),  # ✅ Auth required
    agent_id: Optional[str] = None
):
    # Filter by authenticated tenant only
    tenant_configs = {
        aid: config
        for (tid, aid), config in agents_store.items()
        if tid == tenant_id
    }

    if agent_id:
        return {agent_id: tenant_configs.get(agent_id)}
    return tenant_configs

@app.get("/api/v1/agents/{agent_id}/rules")
async def get_agent_rules(
    agent_id: str,
    tenant_id: str = Depends(get_current_user)  # ✅ Auth required
):
    key = (tenant_id, agent_id)
    config = agents_store.get(key)
    if not config:
        raise HTTPException(status_code=404, detail="Agent not found")
    return config

@app.delete("/api/v1/agents/{agent_id}/rules")
async def delete_agent_rules(
    agent_id: str,
    tenant_id: str = Depends(get_current_user)  # ✅ Auth required
):
    key = (tenant_id, agent_id)
    if key not in agents_store:
        raise HTTPException(status_code=404, detail="Agent not found")
    del agents_store[key]
    return {"status": "deleted"}
```

### Phase 3: Persistent Storage (Estimated: 4 hours)

**Current**: In-memory dictionary (data lost on restart)
**Required**: Database with proper tenant indexing

#### Option A: SQLite (Simple, File-based)

```python
import sqlite3
from contextlib import contextmanager

@contextmanager
def get_db():
    conn = sqlite3.connect("control_plane.db")
    conn.row_factory = sqlite3.Row
    try:
        yield conn
    finally:
        conn.close()

# Schema
CREATE TABLE agent_profiles (
    tenant_id TEXT NOT NULL,
    agent_id TEXT NOT NULL,
    owner TEXT,
    config JSON NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (tenant_id, agent_id)
);

CREATE INDEX idx_tenant_id ON agent_profiles(tenant_id);
```

#### Option B: PostgreSQL (Production-grade)

Use existing Supabase PostgreSQL instance:

```python
from supabase import create_client

supabase = create_client(
    os.getenv("SUPABASE_URL"),
    os.getenv("SUPABASE_SERVICE_KEY")
)

async def save_agent_profile(tenant_id: str, agent_id: str, config: dict):
    supabase.table("agent_profiles").upsert({
        "tenant_id": tenant_id,
        "agent_id": agent_id,
        "config": config
    }).execute()

async def get_agent_profile(tenant_id: str, agent_id: str):
    result = supabase.table("agent_profiles") \
        .select("*") \
        .eq("tenant_id", tenant_id) \
        .eq("agent_id", agent_id) \
        .single() \
        .execute()
    return result.data
```

### Phase 4: Data Plane Integration (Estimated: 2 hours)

Update gRPC client to pass tenant_id:

**File**: `policy_control_plane/dataplane_client.py`

```python
def install_rules(
    self,
    agent_id: str,
    tenant_id: str,  # ✅ NEW - Pass tenant context
    rules: List[Any],
    config_id: str,
    owner: str
) -> Dict[str, Any]:
    request = rule_installation_pb2.InstallRulesRequest(
        agent_id=agent_id,
        tenant_id=tenant_id,  # ✅ Include in gRPC request
        rules=proto_rules,
        config_id=config_id,
        owner=owner
    )
    # ...
```

Update proto definition:

**File**: `proto/rule_installation.proto`

```protobuf
message InstallRulesRequest {
  string agent_id = 1;
  string tenant_id = 2;  // NEW
  repeated PolicyRule rules = 3;
  string config_id = 4;
  string owner = 5;
}
```

### Phase 5: UI Integration (Estimated: 1 hour)

Enable UI access after security is implemented:

**File**: `deployment/ui/.env.production`

```bash
# BEFORE (blocked for security):
# VITE_CONTROL_PLANE_URL not set

# AFTER (safe to enable):
VITE_CONTROL_PLANE_URL=https://platform.tupl.xyz/api/control
```

**File**: `mcp-ui/src/lib/control-plane-api.ts`

Already supports auth:
```typescript
const CONTROL_PLANE_BASE_URL = import.meta.env.VITE_CONTROL_PLANE_URL;

async function request<T>(path: string, options: RequestInit = {}): Promise<T> {
  if (!CONTROL_PLANE_BASE_URL) {
    // Gracefully degrade
    return null;
  }

  const response = await fetch(`${CONTROL_PLANE_BASE_URL}${path}`, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${getAccessToken()}`,  // ✅ Already includes JWT
      ...options.headers,
    },
  });
  // ...
}
```

---

## Testing Requirements

### Unit Tests

```python
# tests/test_auth.py
def test_unauthenticated_request_returns_401():
    response = client.get("/api/v1/rules")
    assert response.status_code == 401

def test_invalid_token_returns_401():
    response = client.get(
        "/api/v1/rules",
        headers={"Authorization": "Bearer invalid_token"}
    )
    assert response.status_code == 401

def test_tenant_cannot_access_other_tenant_policies():
    # Create policy as tenant_a
    tenant_a_token = generate_jwt("tenant_a")
    client.post("/api/v1/agents/agent1/rules", ..., headers={"Authorization": f"Bearer {tenant_a_token}"})

    # Try to access as tenant_b
    tenant_b_token = generate_jwt("tenant_b")
    response = client.get("/api/v1/rules", headers={"Authorization": f"Bearer {tenant_b_token}"})

    # Should not see tenant_a's policies
    assert "agent1" not in response.json()
```

### Integration Tests

```python
# tests/test_e2e_tenant_isolation.py
def test_full_tenant_isolation():
    # Two tenants configure different policies
    tenant_a_creates_policy()
    tenant_b_creates_policy()

    # Each tenant can only see their own
    assert tenant_a_list_policies() == ["agent_a"]
    assert tenant_b_list_policies() == ["agent_b"]

    # Tenant A cannot delete tenant B's policy
    with pytest.raises(HTTPException) as exc:
        tenant_a_delete_policy("agent_b")
    assert exc.value.status_code == 404  # Not found (filtered by tenant)
```

---

## Migration Strategy

### Step 1: Backward Compatibility (Existing Data)

For existing in-memory data without tenant_id:

```python
# Migration function
def migrate_existing_data():
    """Assign unknown policies to a default tenant."""
    migrated = {}
    for agent_id, config in agents_store.items():
        if isinstance(agent_id, str):  # Old format
            tenant_id = config.get("owner", "legacy_tenant")
            migrated[(tenant_id, agent_id)] = config
    return migrated

agents_store = migrate_existing_data()
```

### Step 2: Database Migration (If using Supabase)

```sql
-- Create agent_profiles table
CREATE TABLE agent_profiles (
    tenant_id UUID NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
    agent_id TEXT NOT NULL,
    owner TEXT,
    config JSONB NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (tenant_id, agent_id)
);

-- Indexes for performance
CREATE INDEX idx_agent_profiles_tenant_id ON agent_profiles(tenant_id);
CREATE INDEX idx_agent_profiles_created_at ON agent_profiles(created_at DESC);

-- RLS policies for tenant isolation
ALTER TABLE agent_profiles ENABLE ROW LEVEL SECURITY;

CREATE POLICY "Users can only access their own policies"
    ON agent_profiles
    FOR ALL
    USING (auth.uid() = tenant_id);
```

---

## Rollout Plan

### Pre-Deployment

1. ✅ Document vulnerability (this file)
2. ✅ Update STATUS.md with deferred item
3. ✅ Block UI access via missing `VITE_CONTROL_PLANE_URL`
4. ⏸️ Keep Control Plane disabled in production

### Development Phase (Estimated: 12 hours)

| Task | Estimated Time | Priority |
|------|---------------|----------|
| Add tenant_id to data model | 2h | P0 |
| Implement JWT authentication | 3h | P0 |
| Add tenant filtering to all endpoints | 2h | P0 |
| Implement persistent storage (SQLite) | 3h | P0 |
| Write unit tests | 2h | P0 |
| Write integration tests | 1h | P1 |
| Update Data Plane gRPC integration | 1h | P1 |
| Documentation updates | 1h | P2 |

### Testing Phase (Estimated: 4 hours)

1. Run unit tests (100% coverage on auth logic)
2. Run integration tests (tenant isolation)
3. Manual security testing (penetration tests)
4. Performance testing (database queries)

### Deployment Phase (Estimated: 2 hours)

1. Run database migration
2. Deploy Control Plane with auth
3. Set `VITE_CONTROL_PLANE_URL` in UI
4. Rebuild and deploy UI
5. Verify policies page works with real data
6. Monitor for errors

---

## Dependencies

### Required Packages

```bash
# Python (Control Plane)
pip install python-jose[cryptography]  # JWT validation
pip install python-multipart           # Form data parsing

# Optional: Database
pip install aiosqlite                  # SQLite async
# OR
pip install supabase                   # PostgreSQL via Supabase
```

### Environment Variables

```bash
# Control Plane
SUPABASE_JWT_SECRET=<same as gateway>
SUPABASE_URL=https://azkrxuiqcpxmsgydlyun.supabase.co
SUPABASE_SERVICE_KEY=<service role key>

# UI (enable after fix)
VITE_CONTROL_PLANE_URL=https://platform.tupl.xyz/api/control
```

---

## Current Mitigation

**Status**: ✅ Access Blocked

The Control Plane is currently deployed but **inaccessible from the UI** due to:

1. ✅ Nginx route configured: `location /api/control/` → `http://control_plane/`
2. ✅ Docker port exposed: `127.0.0.1:8001:8001`
3. ❌ UI environment variable **not set**: `VITE_CONTROL_PLANE_URL` (intentionally omitted)
4. ❌ UI API client returns `null` when URL missing (graceful degradation)

**Result**: Users cannot access policies page features until this fix is implemented.

---

## Success Criteria

- [ ] All API endpoints require valid JWT authentication
- [ ] Policies are scoped to tenant_id extracted from JWT
- [ ] No cross-tenant data leakage (verified by tests)
- [ ] UI policies page works with real Control Plane data
- [ ] Database stores policies persistently
- [ ] 100% test coverage on authentication logic
- [ ] Security penetration testing passes
- [ ] Documentation updated with new auth requirements

---

## References

- **Management Plane Auth Pattern**: [management-plane/app/auth.py](../../../management-plane/app/auth.py)
- **Gateway Tenant Resolver**: [mcp-gateway/src/tenant-resolver.ts](../../../mcp-gateway/src/tenant-resolver.ts)
- **Control Plane Current Code**: [policy_control_plane/server.py](../../../policy_control_plane/server.py)
- **UI API Client**: [mcp-ui/src/lib/control-plane-api.ts](../../../mcp-ui/src/lib/control-plane-api.ts)
- **Supabase JWT Docs**: https://supabase.com/docs/guides/auth/server-side/validating-jwts

---

**Last Updated**: 2025-11-20
**Next Review**: Post-MVP (v1.1 release planning)
