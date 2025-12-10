# Authentication Unification Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Unify authentication across the entire Tupl stack by centralizing Supabase JWT authentication in the developer platform (developer.fencio.dev), passing authenticated identity to guard.fencio.dev (mcp-ui) via URL parameters, and simplifying internal service communication by removing authentication between guard.fencio.dev and backend services.

**Architecture:**
- **developer.fencio.dev**: Single source of truth for Supabase authentication. Users login here.
- **guard.fencio.dev**: Receives JWT and API key via URL params, validates JWT directly with Supabase, sends `X-Tenant-Id` header to backend services.
- **Management Plane/Data Plane**: Trust requests from guard.fencio.dev via nginx internal routing without authentication.
- **SDK Authentication**: Uses `api_keys` table for token-based auth (unchanged).

**Tech Stack:** Python/FastAPI (Management Plane), TypeScript/React (mcp-ui), Nginx, Supabase

---

## Task 1: Create Identity Header Contract Documentation

**Files:**
- Create: `docs/contracts/identity-headers.md`

**Step 1: Write the contract documentation**

Create the file with the following content:

```markdown
# Identity Header Contract

This document defines the internal identity header contract used across Tupl services.

## Header Definitions

### X-Tenant-Id (Required)
- **Value**: Supabase `auth.users.id` (UUID format)
- **Source**: Extracted from validated Supabase JWT by the calling service
- **Purpose**: Identifies the tenant/user for all backend operations
- **Example**: `X-Tenant-Id: 550e8400-e29b-41d4-a716-446655440000`

### X-User-Id (Optional)
- **Value**: Supabase `auth.users.id` (same as X-Tenant-Id in current single-user model)
- **Purpose**: Reserved for future multi-user-per-tenant scenarios
- **Example**: `X-User-Id: 550e8400-e29b-41d4-a716-446655440000`

## Service Communication Flows

### guard.fencio.dev → Management Plane
- **Authentication**: None (trusted via nginx internal routing)
- **Required Headers**: `X-Tenant-Id` (extracted from validated JWT)
- **Optional Headers**: `X-User-Id`

### guard.fencio.dev → Data Plane
- **Authentication**: None (trusted via nginx internal routing)
- **Required Headers**: `X-Tenant-Id` (forwarded by nginx)

### SDK → Management Plane
- **Authentication**: API key from `api_keys` table (existing flow, unchanged)
- **Headers**: `Authorization: Bearer <api_key>`

## Developer Platform Integration

### developer.fencio.dev → guard.fencio.dev Redirect
- **Method**: URL parameters
- **Format**: `https://guard.fencio.dev?token=<supabase_jwt>&api_key=<user_api_key>`
- **JWT Validation**: guard.fencio.dev validates JWT directly with Supabase
- **API Key**: Pre-populated in UI for user convenience

## Trust Model

- **External Requests**: All authentication happens at the edge (developer.fencio.dev or SDK with API keys)
- **Internal Requests**: Services trust nginx internal routing, no additional authentication required
- **Tenant Isolation**: Enforced via `X-Tenant-Id` header passed to all backend operations
```

**Step 2: Commit the contract documentation**

```bash
git add docs/contracts/identity-headers.md
git commit -m "docs: add identity header contract for auth unification"
```

---

## Task 2: Update mcp-ui to Accept JWT and API Key from URL Parameters

**Files:**
- Modify: `mcp-ui/src/main.tsx`
- Modify: `mcp-ui/src/contexts/AuthContext.tsx`

**Step 1: Write test for URL parameter extraction**

Create: `mcp-ui/src/lib/url-auth.test.ts`

```typescript
import { describe, it, expect } from 'vitest';
import { extractAuthFromUrl } from './url-auth';

describe('extractAuthFromUrl', () => {
  it('should extract token and api_key from URL params', () => {
    const url = new URL('https://guard.fencio.dev?token=eyJhbGc&api_key=key_123');
    const result = extractAuthFromUrl(url);
    expect(result).toEqual({
      token: 'eyJhbGc',
      apiKey: 'key_123',
    });
  });

  it('should return null values if params missing', () => {
    const url = new URL('https://guard.fencio.dev');
    const result = extractAuthFromUrl(url);
    expect(result).toEqual({
      token: null,
      apiKey: null,
    });
  });

  it('should clean URL after extraction', () => {
    const url = new URL('https://guard.fencio.dev?token=eyJhbGc&api_key=key_123');
    extractAuthFromUrl(url);
    // URL should be cleaned (tested via side effect)
  });
});
```

**Step 2: Run test to verify it fails**

```bash
cd mcp-ui && npm test -- url-auth.test.ts
```

Expected: FAIL with "Cannot find module './url-auth'"

**Step 3: Write URL auth utility**

Create: `mcp-ui/src/lib/url-auth.ts`

```typescript
/**
 * Extracts authentication parameters from URL query string.
 * Used when redirecting from developer.fencio.dev to guard.fencio.dev.
 */
export interface UrlAuthParams {
  token: string | null;
  apiKey: string | null;
}

export function extractAuthFromUrl(url: URL = new URL(window.location.href)): UrlAuthParams {
  const params = new URLSearchParams(url.search);
  const token = params.get('token');
  const apiKey = params.get('api_key');

  // Clean URL by removing auth params
  if (token || apiKey) {
    params.delete('token');
    params.delete('api_key');
    const newUrl = `${url.pathname}${params.toString() ? `?${params.toString()}` : ''}${url.hash}`;
    window.history.replaceState({}, '', newUrl);
  }

  return { token, apiKey };
}
```

**Step 4: Run test to verify it passes**

```bash
cd mcp-ui && npm test -- url-auth.test.ts
```

Expected: PASS (3/3 tests)

**Step 5: Update AuthContext to use URL parameters**

Modify: `mcp-ui/src/contexts/AuthContext.tsx`

Replace lines 25-50 with:

```typescript
  useEffect(() => {
    const startTime = Date.now();
    console.log('AuthContext: Initializing...', { timestamp: startTime });

    // Check for auth params from developer platform redirect
    const { token: urlToken, apiKey: urlApiKey } = extractAuthFromUrl();

    if (urlToken) {
      console.log('AuthContext: Found token in URL params, setting session');
      // Set session from URL token
      supabase.auth.setSession({
        access_token: urlToken,
        refresh_token: '', // Not needed for this flow
      }).then(({ data: { session }, error }) => {
        if (error) {
          console.error('AuthContext: Failed to set session from URL token', error);
          setLoading(false);
          return;
        }
        console.log('AuthContext: Session set from URL token');
        setSession(session);
        setUser(session?.user ?? null);

        // Set API key from URL if provided
        if (urlApiKey) {
          setApiKey(urlApiKey);
        }
        setLoading(false);
      });
    } else {
      // Check for existing session
      supabase.auth.getSession().then(({ data: { session } }) => {
        const elapsed = Date.now() - startTime;
        console.log('AuthContext: getSession result', {
          hasSession: session ? 'Session found' : 'No session',
          elapsed: `${elapsed}ms`,
        });
        setSession(session);
        setUser(session?.user ?? null);
        setLoading(false);
      });
    }

    // Listen for auth changes
    const {
      data: { subscription },
    } = supabase.auth.onAuthStateChange((event, session) => {
      console.log('AuthContext: onAuthStateChange', { event });
      setSession(session);
      setUser(session?.user ?? null);
      if (!session) {
        setApiKey(null);
      }
    });

    return () => subscription.unsubscribe();
  }, []);
```

Add import at top:

```typescript
import { extractAuthFromUrl } from '../lib/url-auth';
```

**Step 6: Run build to verify no TypeScript errors**

```bash
cd mcp-ui && npm run build
```

Expected: Build succeeds with no errors

**Step 7: Commit**

```bash
git add mcp-ui/src/lib/url-auth.ts mcp-ui/src/lib/url-auth.test.ts mcp-ui/src/contexts/AuthContext.tsx
git commit -m "feat(mcp-ui): accept JWT and API key from URL parameters

- Add URL auth parameter extraction utility
- Update AuthContext to initialize session from URL params
- Clean URL after extracting auth params
- Support developer platform redirect flow"
```

---

## Task 3: Add Tenant ID Header to Management Plane API Calls

**Files:**
- Modify: `mcp-ui/src/lib/agent-api.ts`

**Step 1: Update getAuthHeaders to include X-Tenant-Id**

Modify: `mcp-ui/src/lib/agent-api.ts:12-25`

Replace the `getAuthHeaders` function:

```typescript
async function getAuthHeaders(extra?: HeadersInit): Promise<Headers> {
  const {
    data: { session },
  } = await supabase.auth.getSession();

  const token = session?.access_token;
  const userId = session?.user?.id;

  if (!token || !userId) {
    throw new Error('Missing Supabase session. Please sign in again.');
  }

  const headers = new Headers(extra);
  headers.set('Authorization', `Bearer ${token}`);
  headers.set('X-Tenant-Id', userId);
  return headers;
}
```

**Step 2: Run build to verify no TypeScript errors**

```bash
cd mcp-ui && npm run build
```

Expected: Build succeeds

**Step 3: Commit**

```bash
git add mcp-ui/src/lib/agent-api.ts
git commit -m "feat(mcp-ui): add X-Tenant-Id header to MP API calls

- Extract tenant ID from Supabase session
- Include X-Tenant-Id header in all authenticated requests"
```

---

## Task 4: Update Management Plane to Support Header-Based Tenant Resolution

**Files:**
- Modify: `management-plane/app/auth.py`

**Step 1: Write test for header-based tenant resolution**

Create: `management-plane/tests/test_header_auth.py`

```python
"""Tests for header-based authentication."""

import pytest
from fastapi import HTTPException
from app.auth import get_current_user_from_headers


def test_get_current_user_from_headers_success():
    """Should extract tenant from X-Tenant-Id header."""
    user = get_current_user_from_headers(
        x_tenant_id="550e8400-e29b-41d4-a716-446655440000",
        x_user_id=None
    )
    assert user.id == "550e8400-e29b-41d4-a716-446655440000"
    assert user.aud == "internal-header"
    assert user.role == "authenticated"


def test_get_current_user_from_headers_with_user_id():
    """Should support optional X-User-Id header."""
    user = get_current_user_from_headers(
        x_tenant_id="550e8400-e29b-41d4-a716-446655440000",
        x_user_id="660e8400-e29b-41d4-a716-446655440000"
    )
    assert user.id == "550e8400-e29b-41d4-a716-446655440000"
    assert user.email == "660e8400-e29b-41d4-a716-446655440000"  # Store user_id in email field


def test_get_current_user_from_headers_missing_tenant_id():
    """Should raise 401 if X-Tenant-Id missing."""
    with pytest.raises(HTTPException) as exc:
        get_current_user_from_headers(x_tenant_id=None, x_user_id=None)
    assert exc.value.status_code == 401
```

**Step 2: Run test to verify it fails**

```bash
cd management-plane && pytest tests/test_header_auth.py -v
```

Expected: FAIL with "ImportError: cannot import name 'get_current_user_from_headers'"

**Step 3: Implement header-based tenant resolver**

Modify: `management-plane/app/auth.py`

Add after line 161:

```python
async def get_current_user_from_headers(
    x_tenant_id: Optional[str] = Header(None),
    x_user_id: Optional[str] = Header(None)
) -> User:
    """
    FastAPI dependency to extract tenant identity from internal headers.

    Used for requests from guard.fencio.dev which validates JWT client-side
    and passes tenant ID via headers. No authentication required as nginx
    internal routing provides trust boundary.

    Args:
        x_tenant_id: Required tenant ID from validated JWT
        x_user_id: Optional user ID for future multi-user scenarios

    Returns:
        User object with tenant identity

    Raises:
        HTTPException: If X-Tenant-Id header missing
    """
    if not x_tenant_id:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="Missing X-Tenant-Id header",
            headers={"WWW-Authenticate": "Bearer"},
        )

    return User(
        id=x_tenant_id,
        aud="internal-header",
        role="authenticated",
        email=x_user_id  # Store user_id in email field if provided
    )
```

**Step 4: Run test to verify it passes**

```bash
cd management-plane && pytest tests/test_header_auth.py -v
```

Expected: PASS (3/3 tests)

**Step 5: Commit**

```bash
git add management-plane/app/auth.py management-plane/tests/test_header_auth.py
git commit -m "feat(management-plane): add header-based tenant resolution

- Add get_current_user_from_headers dependency
- Support X-Tenant-Id and X-User-Id headers
- Enable trusted internal service communication"
```

---

## Task 5: Update Management Plane Endpoints to Use Header-Based Auth

**Files:**
- Modify: `management-plane/app/auth.py`
- Modify: `management-plane/app/endpoints/agents.py`
- Modify: `management-plane/app/endpoints/enforcement.py`
- Modify: `management-plane/app/endpoints/telemetry.py`

**Step 1: Create dual-auth dependency helper**

Modify: `management-plane/app/auth.py`

Add after the `get_current_user_from_headers` function:

```python
async def get_current_tenant(
    # Try header-based auth first (guard.fencio.dev)
    x_tenant_id: Optional[str] = Header(None),
    x_user_id: Optional[str] = Header(None),
    # Fallback to JWT auth (SDK, direct API calls)
    token: Optional[str] = Depends(oauth2_scheme),
    x_service_auth: Optional[str] = Header(None),
) -> User:
    """
    Unified tenant resolver supporting both header-based and JWT authentication.

    Priority:
    1. X-Tenant-Id header (guard.fencio.dev → MP)
    2. JWT token (SDK, direct API calls)

    Returns:
        User object with tenant identity
    """
    # Header-based auth (guard.fencio.dev)
    if x_tenant_id:
        return await get_current_user_from_headers(x_tenant_id, x_user_id)

    # JWT auth (SDK, legacy)
    return await get_current_user(token, x_service_auth, x_user_id)
```

**Step 2: Update agents endpoint**

Modify: `management-plane/app/endpoints/agents.py:14`

Change import:

```python
from ..auth import User, get_current_tenant
```

Modify: `management-plane/app/endpoints/agents.py:95,163,213,319,337`

Replace all instances of `current_user: User = Depends(get_current_user)` with:

```python
current_user: User = Depends(get_current_tenant)
```

**Step 3: Update enforcement endpoint**

Modify: `management-plane/app/endpoints/enforcement.py:11`

Change import:

```python
from app.auth import User, get_current_tenant
```

Modify: `management-plane/app/endpoints/enforcement.py:33`

Replace:

```python
current_user: User = Depends(get_current_user),
```

with:

```python
current_user: User = Depends(get_current_tenant),
```

**Step 4: Update telemetry endpoint (if exists)**

```bash
cd management-plane && grep -n "get_current_user" app/endpoints/telemetry.py
```

If found, update import and dependencies similar to above steps.

**Step 5: Run all tests**

```bash
cd management-plane && pytest -v
```

Expected: All tests pass

**Step 6: Commit**

```bash
git add management-plane/app/auth.py management-plane/app/endpoints/
git commit -m "feat(management-plane): support dual auth (headers + JWT)

- Add get_current_tenant unified dependency
- Update agents, enforcement, telemetry endpoints
- Maintain backward compatibility with SDK JWT auth"
```

---

## Task 6: Remove user_tokens Table Dependencies

**Files:**
- Modify: `management-plane/app/auth.py`
- Modify: `mcp-ui/src/pages/SettingsPage.tsx`

**Step 1: Remove user_tokens lookup from auth.py**

Modify: `management-plane/app/auth.py:60-91`

Delete the `_lookup_user_id_from_tupl_token` function entirely.

Modify: `management-plane/app/auth.py:126-131`

Remove the Tupl token (t_...) handling block:

```python
# Delete these lines:
    # Support Tupl gateway tokens (t_...) for remote SDKs / MCP clients
    if token.startswith("t_"):
        user_id = _lookup_user_id_from_tupl_token(token)
        if not user_id:
            raise credentials_exception
        return User(id=user_id, aud="tupl_token", role="authenticated")
```

**Step 2: Remove user_tokens references from tests**

```bash
cd management-plane && grep -r "user_tokens" tests/
```

Remove any test references to `user_tokens` table.

**Step 3: Update SettingsPage to show API key guidance instead**

Modify: `mcp-ui/src/pages/SettingsPage.tsx`

Replace entire file content:

```typescript
import { Card, CardHeader, CardTitle, CardContent, CardDescription } from "@/components/ui/card";
import { useAuth } from "@/contexts/AuthContext";
import { AlertCircle } from "lucide-react";
import { Alert, AlertDescription } from "@/components/ui/alert";

export const SettingsPage = () => {
  const { user, apiKey } = useAuth();

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">Settings</h1>
        <p className="text-muted-foreground">
          Manage your account settings and authentication.
        </p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Authentication</CardTitle>
          <CardDescription>
            Your authentication is managed by the Fencio Developer Platform.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-2">
            <div className="text-sm font-medium">User ID</div>
            <div className="font-mono text-sm bg-muted p-3 rounded-md">
              {user?.id || 'Not authenticated'}
            </div>
          </div>

          {apiKey && (
            <div className="space-y-2">
              <div className="text-sm font-medium">API Key (from Developer Platform)</div>
              <div className="font-mono text-sm bg-muted p-3 rounded-md truncate">
                {apiKey}
              </div>
            </div>
          )}

          <Alert>
            <AlertCircle className="h-4 w-4" />
            <AlertDescription>
              To manage your API keys and authentication settings, visit the{' '}
              <a
                href="https://developer.fencio.dev"
                className="underline font-medium"
                target="_blank"
                rel="noopener noreferrer"
              >
                Fencio Developer Platform
              </a>
              .
            </AlertDescription>
          </Alert>
        </CardContent>
      </Card>
    </div>
  );
};
```

**Step 4: Build UI to verify no errors**

```bash
cd mcp-ui && npm run build
```

Expected: Build succeeds

**Step 5: Commit**

```bash
git add management-plane/app/auth.py management-plane/tests/ mcp-ui/src/pages/SettingsPage.tsx
git commit -m "refactor: remove user_tokens table dependencies

- Remove t_... token lookup from Management Plane
- Update SettingsPage to reference developer platform
- Simplify auth to JWT-only (from developer platform) + api_keys (SDK)"
```

---

## Task 7: Configure Nginx for Trusted Internal Routing

**Files:**
- Modify: `deployment/ui/nginx.conf`

**Step 1: Add upstream for Management Plane**

Modify: `deployment/ui/nginx.conf`

Add before the `server` block:

```nginx
# Management Plane backend
upstream management_plane {
    server management-plane:8000;
}

server {
    # ... existing config ...
}
```

**Step 2: Add proxy configuration for MP API calls**

Modify: `deployment/ui/nginx.conf`

Add after line 35 (before closing brace):

```nginx
    # Proxy API calls to Management Plane
    # Trust all requests from guard.fencio.dev UI
    location /api/v1/ {
        proxy_pass http://management_plane;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # Allow larger request bodies for policy uploads
        client_max_body_size 10M;

        # Timeout settings for long-running requests
        proxy_connect_timeout 60s;
        proxy_send_timeout 60s;
        proxy_read_timeout 60s;
    }
```

**Step 3: Verify nginx config syntax**

```bash
nginx -t -c deployment/ui/nginx.conf
```

Expected: "syntax is ok" and "test is successful"

**Step 4: Commit**

```bash
git add deployment/ui/nginx.conf
git commit -m "feat(nginx): configure trusted routing for guard.fencio.dev

- Add Management Plane upstream
- Proxy /api/v1/ to MP backend
- No auth required (trusted internal network)"
```

---

## Task 8: Remove Supabase Direct References from Auth Endpoints

**Files:**
- Modify: `management-plane/app/endpoints/auth.py`

**Step 1: Review auth endpoints**

```bash
cd management-plane && cat app/endpoints/auth.py
```

**Step 2: Remove /validate-token endpoint if it exists**

If the file contains a `/validate-token` endpoint, remove it entirely.

**Step 3: Update endpoint comments**

Update any comments referencing Supabase JWT validation to clarify that validation happens at the edge (developer platform or guard.fencio.dev).

**Step 4: Commit**

```bash
git add management-plane/app/endpoints/auth.py
git commit -m "refactor(management-plane): remove validate-token endpoint

- JWT validation now happens at edge (developer platform)
- MP trusts X-Tenant-Id headers from guard.fencio.dev"
```

---

## Task 9: Update Environment Variable Documentation

**Files:**
- Modify: `management-plane/.env.example`
- Create: `docs/deployment/environment-variables.md`

**Step 1: Update .env.example**

Modify: `management-plane/.env.example`

Remove:
```
# Remove if exists
INTERNAL_SHARED_SECRET=...
```

Keep existing Supabase vars (still needed for JWT validation in SDK flow):
```
SUPABASE_URL=https://xxx.supabase.co
SUPABASE_JWT_SECRET=xxx
SUPABASE_SERVICE_KEY=xxx
```

**Step 2: Create environment variables documentation**

Create: `docs/deployment/environment-variables.md`

```markdown
# Environment Variables

## Management Plane

### Authentication
- `SUPABASE_URL`: Supabase project URL (for JWT validation in SDK flows)
- `SUPABASE_JWT_SECRET`: Supabase JWT secret (for JWT validation)
- `SUPABASE_SERVICE_KEY`: Supabase service role key (for database access)

### Services
- `DATA_PLANE_URL`: Data Plane gRPC endpoint (default: localhost:50051)
- `CHROMA_URL`: ChromaDB endpoint for vector storage

### AI/ML
- `GOOGLE_API_KEY`: Google AI API key for policy parsing

## mcp-ui (guard.fencio.dev)

### Supabase
- `VITE_SUPABASE_URL`: Supabase project URL (for JWT validation)
- `VITE_SUPABASE_ANON_KEY`: Supabase anon key (for JWT validation)

### Backend
- `VITE_MANAGEMENT_PLANE_URL`: Management Plane URL (default: http://localhost:8000)

## Authentication Flow

### External Users (via developer.fencio.dev)
1. User logs in at developer.fencio.dev (Supabase OAuth)
2. User clicks button → redirected to `https://guard.fencio.dev?token=<jwt>&api_key=<key>`
3. guard.fencio.dev validates JWT with Supabase
4. guard.fencio.dev sends requests to MP with `X-Tenant-Id` header
5. MP trusts guard.fencio.dev requests (nginx internal routing)

### SDK Users
1. SDK uses API key from developer platform
2. SDK sends requests with `Authorization: Bearer <api_key>`
3. MP validates JWT as before (existing flow unchanged)

## Trust Boundaries

- **developer.fencio.dev → guard.fencio.dev**: JWT + API key via URL params
- **guard.fencio.dev → MP**: X-Tenant-Id header (trusted via nginx)
- **SDK → MP**: JWT authentication (existing flow)
```

**Step 3: Commit**

```bash
git add management-plane/.env.example docs/deployment/environment-variables.md
git commit -m "docs: update environment variable documentation

- Document new auth flow via developer platform
- Clarify trust boundaries and service communication
- Remove deprecated INTERNAL_SHARED_SECRET"
```

---

## Task 10: Add Developer Platform Redirect Link to Login Page

**Files:**
- Modify: `mcp-ui/src/pages/LoginPage.tsx`

**Step 1: Update LoginPage to show developer platform link**

Modify: `mcp-ui/src/pages/LoginPage.tsx`

Replace the sign-in button section with:

```typescript
<Card>
  <CardHeader>
    <CardTitle>Welcome to Fencio Guard</CardTitle>
    <CardDescription>
      Manage your AI agent policies and security settings.
    </CardDescription>
  </CardHeader>
  <CardContent className="space-y-4">
    <Alert>
      <AlertCircle className="h-4 w-4" />
      <AlertDescription>
        Please login via the{' '}
        <a
          href="https://developer.fencio.dev"
          className="underline font-medium"
        >
          Fencio Developer Platform
        </a>
        {' '}to access this application.
      </AlertDescription>
    </Alert>

    <div className="text-sm text-muted-foreground">
      <p>
        After logging in to the developer platform, you'll be redirected back here
        automatically.
      </p>
    </div>
  </CardContent>
</Card>
```

Add import:

```typescript
import { AlertCircle } from "lucide-react";
import { Alert, AlertDescription } from "@/components/ui/alert";
```

**Step 2: Build to verify**

```bash
cd mcp-ui && npm run build
```

Expected: Build succeeds

**Step 3: Commit**

```bash
git add mcp-ui/src/pages/LoginPage.tsx
git commit -m "feat(mcp-ui): update login page for developer platform flow

- Replace local auth with developer platform link
- Guide users to login via developer.fencio.dev
- Explain redirect flow"
```

---

## Task 11: Integration Testing

**Files:**
- Create: `docs/testing/auth-unification-test-plan.md`

**Step 1: Create test plan document**

Create: `docs/testing/auth-unification-test-plan.md`

```markdown
# Auth Unification Test Plan

## Manual Test Cases

### Test 1: Developer Platform → Guard Redirect
1. Navigate to `https://developer.fencio.dev`
2. Login with Google OAuth
3. Click "Open Guard Console" button
4. Verify redirect to `https://guard.fencio.dev?token=...&api_key=...`
5. Verify guard.fencio.dev loads without showing login page
6. Verify URL params are cleaned after initial load
7. Verify user ID displayed in settings page

**Expected**: User logged in successfully, URL params removed from address bar

### Test 2: Guard → Management Plane API Call
1. Complete Test 1 to login
2. Navigate to Agents page
3. Open browser DevTools → Network tab
4. Refresh page to trigger `/api/v1/agents/list` call
5. Inspect request headers
6. Verify `Authorization: Bearer <jwt>` header present
7. Verify `X-Tenant-Id: <user_id>` header present

**Expected**: Both headers present, API call succeeds, agents list displays

### Test 3: Create Agent Policy
1. Complete Test 1 to login
2. Navigate to Agents page
3. Click on an agent
4. Create a new policy
5. Verify policy creation succeeds
6. Check browser DevTools → Network → `/api/v1/agents/policies` request
7. Verify headers include `X-Tenant-Id`

**Expected**: Policy created successfully, stored with correct tenant_id

### Test 4: SDK Authentication (Unchanged)
1. Run SDK example: `cd examples/langgraph_demo && python weather_agent.py`
2. Verify agent registers successfully
3. Verify enforcement calls succeed
4. Check MP logs for JWT validation

**Expected**: SDK flow unchanged, works as before

### Test 5: Direct MP API Call (No Guard)
1. Get Supabase JWT from browser (DevTools → Application → Cookies)
2. Make direct curl request to MP:
```bash
curl -X GET https://api.fencio.dev/api/v1/agents/list \
  -H "Authorization: Bearer <jwt>" \
  -H "X-Tenant-Id: <user_id>"
```
3. Verify response returns agents

**Expected**: JWT auth still works for direct API calls

### Test 6: Unauthenticated Access
1. Clear browser storage and cookies
2. Navigate directly to `https://guard.fencio.dev/agents`
3. Verify redirect to login page
4. Verify login page shows developer platform link

**Expected**: Unauthenticated users redirected to login, guided to developer platform

## Automated Tests

Run full test suites:

```bash
# Management Plane
cd management-plane && pytest -v

# mcp-ui
cd mcp-ui && npm test
```

**Expected**: All tests pass

## Rollback Procedure

If critical issues found:

1. Revert last N commits:
```bash
git revert HEAD~N..HEAD
```

2. Redeploy previous version:
```bash
./deployment/gateway/deploy-production.sh
```

3. Verify previous auth flow restored

## Success Criteria

- [ ] Users can login via developer.fencio.dev
- [ ] Redirect to guard.fencio.dev works with URL params
- [ ] guard.fencio.dev validates JWT and extracts tenant ID
- [ ] guard.fencio.dev → MP API calls include X-Tenant-Id header
- [ ] MP trusts guard.fencio.dev requests without additional auth
- [ ] SDK authentication unchanged and working
- [ ] All tests passing
- [ ] No user_tokens table dependencies remaining
```

**Step 2: Run all automated tests**

```bash
cd management-plane && pytest -v
cd ../mcp-ui && npm test
```

Expected: All tests pass

**Step 3: Commit test plan**

```bash
git add docs/testing/auth-unification-test-plan.md
git commit -m "docs: add auth unification test plan

- Manual test cases for developer platform flow
- Automated test verification steps
- Rollback procedure
- Success criteria checklist"
```

---

## Task 12: Update README and Documentation

**Files:**
- Modify: `README.md`
- Modify: `STATUS.md`

**Step 1: Update README authentication section**

Modify: `README.md`

Find the authentication section and replace with:

```markdown
## Authentication Architecture

### User Authentication Flow

1. **Developer Platform (developer.fencio.dev)**
   - Users login via Supabase OAuth (Google, GitHub, etc.)
   - Single source of truth for authentication
   - Manages API keys in `api_keys` table

2. **Guard Console (guard.fencio.dev)**
   - Receives JWT + API key via URL redirect from developer platform
   - Validates JWT directly with Supabase
   - Sends `X-Tenant-Id` header to backend services
   - No local authentication required

3. **Management Plane**
   - Accepts two auth modes:
     - Header-based: `X-Tenant-Id` from guard.fencio.dev (trusted via nginx)
     - JWT-based: `Authorization: Bearer` from SDK (validated against Supabase)
   - Tenant isolation enforced via `X-Tenant-Id` in all database queries

4. **SDK Authentication**
   - Uses API keys from developer platform (`api_keys` table)
   - Sends `Authorization: Bearer <api_key>` header
   - MP validates JWT and extracts tenant ID

### Trust Boundaries

- **External → Internal**: Authentication at edge (developer platform + JWT validation)
- **Internal Services**: Trusted via nginx internal routing (no auth required)
- **Tenant Isolation**: Enforced via `X-Tenant-Id` header in all backend operations
```

**Step 2: Update STATUS.md**

Modify: `STATUS.md`

Update the "Current Status" section:

```markdown
# Current Status
- **COMPLETED**: Authentication unification across Tupl stack
  - Centralized auth in developer platform (developer.fencio.dev)
  - Guard console (guard.fencio.dev) receives JWT via URL params
  - Simplified internal service communication (no auth between guard → MP)
  - Removed user_tokens table dependencies
  - Dual auth support: header-based (guard) + JWT (SDK)
  - Contract documentation: docs/contracts/identity-headers.md
  - Test plan: docs/testing/auth-unification-test-plan.md
```

**Step 3: Commit documentation updates**

```bash
git add README.md STATUS.md
git commit -m "docs: update README and STATUS for auth unification

- Document new authentication architecture
- Clarify trust boundaries and service communication
- Update status with completed auth unification"
```

---

## Task 13: Final Verification and Cleanup

**Files:**
- N/A (verification only)

**Step 1: Verify no user_tokens references remain**

```bash
grep -r "user_tokens" --exclude-dir=".venv" --exclude-dir="node_modules" --exclude-dir=".git" .
```

Expected: No matches except in migration files or this plan document

**Step 2: Verify all tests pass**

```bash
cd management-plane && pytest -v
cd ../mcp-ui && npm test && npm run build
```

Expected: All tests pass, build succeeds

**Step 3: Review all changes**

```bash
git log --oneline --since="1 day ago"
```

Verify all commits are present with clear messages.

**Step 4: Create summary commit**

```bash
git commit --allow-empty -m "feat: complete auth unification implementation

Authentication now flows:
1. Users login at developer.fencio.dev (Supabase OAuth)
2. Redirect to guard.fencio.dev with JWT + API key
3. Guard validates JWT, sends X-Tenant-Id to backend
4. Backend trusts internal routing, no auth required
5. SDK flow unchanged (API keys from developer platform)

Breaking changes:
- Removed user_tokens table (use api_keys for SDK auth)
- Guard UI requires developer platform redirect
- Direct guard.fencio.dev access redirects to login

Migration:
- Deploy developer platform with redirect button first
- Deploy guard + MP together (atomic deployment)
- Update nginx configs to enable internal routing"
```

---

## Migration and Deployment

### Pre-Deployment Checklist

- [ ] Developer platform has "Open Guard Console" button implemented
- [ ] Developer platform redirects to `https://guard.fencio.dev?token=<jwt>&api_key=<key>`
- [ ] Nginx configs updated on target environment
- [ ] Environment variables verified in production

### Deployment Steps

1. **Deploy Developer Platform** (separate codebase)
   - Add redirect button to Guard Console
   - Test redirect URL format

2. **Deploy Management Plane + mcp-ui** (atomic deployment)
   ```bash
   ./deployment/gateway/deploy-production.sh
   ```

3. **Verify Nginx Configuration**
   ```bash
   docker exec <nginx-container> nginx -t
   docker exec <nginx-container> nginx -s reload
   ```

4. **Run Manual Test Plan**
   - Follow `docs/testing/auth-unification-test-plan.md`
   - Verify all test cases pass

### Rollback Plan

If issues detected:

```bash
git revert <start-commit>..HEAD
./deployment/gateway/deploy-production.sh
```

No database changes required - rollback is code-only.

---

## Open Questions

1. **Developer Platform Implementation**: Who implements the "Open Guard Console" button and redirect logic?
2. **URL Parameter Security**: Should we add HMAC signature to URL params to prevent tampering?
3. **Session Duration**: Should guard.fencio.dev refresh tokens automatically or require re-login via developer platform?
4. **API Key Management**: Should guard.fencio.dev allow creating/rotating API keys or keep that in developer platform only?

---

## Success Criteria

- [ ] Users can login via developer.fencio.dev
- [ ] Redirect to guard.fencio.dev works with JWT + API key in URL
- [ ] guard.fencio.dev validates JWT and extracts tenant ID
- [ ] guard.fencio.dev → MP includes `X-Tenant-Id` header
- [ ] MP trusts guard.fencio.dev requests via nginx routing
- [ ] SDK authentication unchanged and working
- [ ] No user_tokens table references remaining
- [ ] All automated tests passing
- [ ] Documentation updated (README, STATUS, contracts)
- [ ] Test plan created and manual tests verified

🚢 **Generated with [Claude Code](https://claude.com/claude-code)**
