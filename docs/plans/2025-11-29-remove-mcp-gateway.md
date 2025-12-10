# Plan: Remove MCP Gateway & Simplify Stack to SDK + UI + ai-security-stack
Created: 2025-11-29
Status: Draft
Owner: (assign)

## Goal
Eliminate the MCP Gateway service (and its token-reduction/vm2 tooling) from the architecture. The product becomes:
- Tupl SDK (PyPI) for enforcement integration.
- MCP UI for NL policy authoring and telemetry.
- ai-security-stack (Management Plane + Data Plane + Chroma) + Supabase.
- Developer Platform remains the sole auth front door (see auth unification plan).

## Out of Scope
- Changing NL guardrail pipeline, Data Plane rule format, or telemetry internals.
- Altering SDK enforcement modes beyond removing gateway references.
- Replacing Claude/Cursor MCP features with alternatives (they’re dropped).

## High-Level Outcome
- Services: keep `ai-security-stack` and `mcp-ui`; drop `mcp-gateway` service, its volumes, and resolver logic.
- UX: Users install SDK from PyPI and add the `enforcement_agent` wrapper manually; UI remains the place to create NL policies and view telemetry/hitlogs.
- Deployment: docker-compose and Nginx routes no longer include gateway; fewer env vars.

## Work Items (Atomic Tasks)

### A. Topology & Deploy Config
1) **docker-compose.production.yml**  
   - Remove `mcp-gateway-http` service, its volumes (`gateway-tenants`, `gateway-workspace`, `gateway-chromadb`), and dependencies.  
   - Remove gateway healthcheck logic from deploy script expectations.

2) **docker-compose.local.yml / deployment/gateway/deploy-local.sh**  
   - Mirror the removal locally; ensure local stack runs `ai-security-stack` + `mcp-ui` + Chroma only.

3) **deployment/gateway/deploy-production.sh**  
   - Drop gateway env validation (MCP_GATEWAY_* variables) and health waits for port 3000.  
   - Remove submodule update if only used for gateway.  
   - Adjust success URLs printed to omit `/mcp` and gateway base URL.

4) **Nginx config (`deployment/gateway/nginx.conf`)**  
   - Remove upstream `mcp_gateway`.  
   - Remove `/mcp` and `/api` (gateway) locations.  
   - Keep `/` → UI, `/api/v1` → Management Plane, `/rule_installation.DataPlane/*` → Data Plane gRPC, `/internal/validate-token`.  
   - Update rate-limit zones accordingly.

### B. UI Cleanup (mcp-ui)
5) Remove Gateway management UI:
   - Delete routes/components for MCP servers and gateway API keys (e.g., `McpServersPage`, `ApiKeysPage`, related sidebar links).  
   - Remove `gateway-api.ts` client and references.
6) Simplify Settings page:
   - Remove gateway token/API key references.  
   - Keep display of Tupl API token only if still relevant; otherwise note that auth is handled by the Developer Platform (align with auth plan).
7) Nav/UX polish:
   - Ensure default landing is agents/telemetry/policies.  
   - Remove leftover feature flags or “MCP” wording in the UI.

### C. SDK & Docs
8) SDK docs (README/SDK_USAGE/README.md root):
   - Remove references to MCP Gateway integration and `wrap_agent` tool.  
   - Emphasize manual wrapper snippet with `enforcement_agent`, base_url pointing to Management Plane via Developer Platform proxy (after auth plan).  
   - Clarify enforcement modes; note gateway removal in a “What changed” section.
9) Remove gateway-generated code references:
   - If any examples mention `wrap_agent` output or gateway tokens, replace with direct SDK snippet.

### D. Codebase Removal
10) Delete or archive `mcp-gateway/` directory from build/test paths:  
    - If keeping for reference, mark as archived; otherwise remove from repo and root package.json scripts.  
    - Remove gateway from root `package.json`/`Makefile` scripts if present.
11) Remove gateway-related scripts or debug clients (`mcp-gateway/debug-client`, etc.) from CI paths.

### E. Telemetry & UI data flow (no gateway)
12) Verify telemetry UI still works end-to-end: UI → MP `/api/v1/telemetry/...` → DP gRPC → hitlogs.
13) Adjust README architecture diagrams/text to omit gateway; explain new simplified flow.

### F. Env & Secrets
14) Prune env templates (`.env.production`, UI VITE_ vars) to remove gateway variables.  
15) Update docs on required env: Supabase, Gemini, Chroma, Management Plane ports, Data Plane gRPC, internal shared secret (from auth plan).

### G. Testing & Validation
16) Local smoke test (post-removal):
    - docker-compose up: MP health at :8000, DP gRPC at :50051, UI at :8080, Chroma up.  
    - UI: create policy, ensure it installs and enforces via SDK sample.  
    - Telemetry: verify sessions populate and load in UI.
17) Production staging dry-run:
    - Deploy with updated compose + Nginx; verify no references to :3000 and no missing env vars.  
    - SDK against staging: agent register + enforce.

### H. Auth Plan Alignment (follow-up)
18) After gateway removal, re-apply auth unification plan steps focusing on MP/UI/DP only (skip gateway steps):  
    - Identity headers (`X-Tenant-Id`, `X-User-Id`, `X-Internal-Auth`).  
    - Nginx auth_request updates already planned.  
    - UI “platform mode” without local login.

## Risks / Mitigations
- **Hidden runtime deps on gateway**: Search for any service expecting gateway health; remove checks.  
- **UI dead links**: Removing routes could leave orphaned nav items—validate navigation.  
- **Docs drift**: README and STATUS must be updated concurrently to avoid confusion.

## Deliverables
- Updated deployment artifacts (compose, nginx, deploy scripts) without gateway.
- Updated UI without MCP server/key management screens.
- Updated SDK/docs without gateway references.
- README/STATUS refreshed to the new architecture.

