# Plan: Shift All Auth to Developer Platform & Simplify Identity Across Modules
Created: 2025-11-29
Status: Draft
Owner: (assign)

## Goal
Make the entire Tupl stack trust the Developer Platform for authentication/identity, while keeping current UI features working (even if UI temporarily runs without its own login). All components should consume a unified tenant identity (`auth.users.id`) provided by the Developer Platform, not validate end-user tokens themselves.  
**Note:** The MCP Gateway is being removed (see `2025-11-29-remove-mcp-gateway.md`); any gateway-specific steps are dropped.

## Scope
- Management Plane
- Data Plane ingress (Nginx auth_request + gRPC metadata)
- MCP UI (how it obtains identity and calls backend)
- Shared auth/identity helpers
- Docs & configs
(-) MCP Gateway (service removed)

## Non-goals
- Changing embedding/enforcement logic
- Changing Supabase/Chroma schemas
- Removing SDK enforcement modes

## High-Level Strategy
1) Introduce a single, internal identity contract: every trusted call into the stack must carry `X-Tenant-Id: <auth.users.id>` (and optionally `X-User-Id`) set by the Developer Platform.
2) Stop doing end-user auth in stack components; replace with header-based tenant extraction guarded by an internal secret or mTLS where needed.
3) Remove all Supabase JWT authentication from the stack - all auth goes through Developer Platform headers only.

## Work Items (Atomic Tasks)

### A. Cross-cutting contracts
1. Define identity header contract doc:
   - Header names: `X-Tenant-Id`, optional `X-User-Id`, optional `X-Internal-Auth` (shared secret).
   - Expected values: Supabase `auth.users.id`.
   - Add to repo as `docs/contracts/identity-headers.md`.

### B. Management Plane
2. Add new tenant resolver helper:
   - New function `get_current_tenant()` in `app/auth.py` that:
     - Reads `X-Tenant-Id` (required) and `X-User-Id` (optional).
     - Verifies `X-Internal-Auth` against an env var (e.g., `INTERNAL_SHARED_SECRET`) when set.
   - Returns a lightweight `TenantContext` with `tenant_id`, `user_id`, `source="internal-header"`.
3. Migrate enforcement & agent routes:
   - Switch dependencies in `/api/v1/enforce`, `/api/v1/agents/*`, `/api/v1/telemetry/*` to use `get_current_tenant()` instead of `get_current_user()`.
   - Remove `get_current_user()` and all Supabase JWT validation code.
4. Remove token-specific logic from telemetry queries:
   - Ensure `DataPlaneClient` calls don't rely on user tokens; pass tenant_id only when needed for filtering.
5. Remove `/api/v1/auth/validate-token` endpoint:
   - No longer needed as token validation happens at Developer Platform level.

### C. Nginx / gRPC ingress
6. Adjust `nginx.conf`:
   - `auth_request` for `/rule_installation.DataPlane/*` should call MP with headers:
     - `X-Tenant-Id` forwarded from upstream (set by dev platform) or reject if missing.
     - `X-Internal-Auth` shared secret.
   - Remove assumptions about Bearer tokens in `auth_request`.
7. Ensure gRPC metadata propagation:
   - Nginx sets `grpc_set_header X-Tenant-Id $upstream_http_x_tenant_id` on DP gRPC upstream.

### D. MCP UI
8. Remove Supabase authentication entirely:
   - Remove all Supabase client initialization and auth state management.
   - Remove login/signup routes and components.
   - UI assumes Developer Platform reverse-proxies all requests and injects `X-Tenant-Id`.
   - Keep pages functional without local auth state.
9. API clients (`agent-api`, `telemetry-api`):
   - Remove Authorization header attachment.
   - Rely entirely on headers forwarded by Developer Platform proxy.
10. Settings page updates:
    - Remove `user_tokens` fetch and display (token issuance handled by Developer Platform).
    - Update UI to reflect that authentication is managed externally.

### E. SDK (minimal changes now, optional)
11. Document new recommended deployment pattern:
    - Agents call Developer Platform, not MP directly, in production.
    - For now, keep SDK behavior unchanged; mark `base_url` guidance in docs.
    - (Optional follow-up) Add an option for SDK to send `X-Tenant-Id` instead of Bearer tokens when behind dev platform proxy.

### F. Configuration & Secrets
12. Add new env vars:
    - `INTERNAL_SHARED_SECRET` for MP.
13. Remove old env vars:
    - Remove all Supabase-related config from MP and UI.
    - Remove `VITE_SUPABASE_URL`, `VITE_SUPABASE_ANON_KEY`, etc.
14. Update env templates and README notes with new auth model.

### G. Testing & rollout
15. Local dev proxy harness:
    - Add a small dev proxy script that injects `X-Tenant-Id` for local testing.
    - Provide instructions for running UI/MP behind the proxy during development.
16. Regression tests:
    - MP `/api/v1/agents/*` and `/api/v1/enforce` should reject missing tenant headers.
    - Telemetry endpoints should still return data filtered by tenant_id.
    - All endpoints should reject requests without proper `X-Internal-Auth` secret.
17. Cleanup pass:
    - Remove all Supabase client code from UI and MP.
    - Remove `user_tokens` table queries and related code.
    - Update README + STATUS with the new auth model.

## Migration/Backout Plan
- This is a breaking change - implement all components at once.
- All Supabase JWT authentication removed in favor of Developer Platform headers.
- Backout: revert to previous git commit; no database schema changes involved.
- **IMPORTANT**: Developer Platform must be running and configured to proxy requests before this change is deployed.

## Open Questions / Follow-ups
- Do we want mTLS between Developer Platform and MP/DP for stronger trust than shared secret?
- Should the developer platform issue short-lived signed headers (HMAC/JWT) instead of a static shared secret?
- How should SDK users authenticate when calling the Developer Platform? (out of scope for this plan, but needs documentation)
