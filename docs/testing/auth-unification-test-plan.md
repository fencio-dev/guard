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