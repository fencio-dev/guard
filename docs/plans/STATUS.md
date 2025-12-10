# Implementation Status

**Last Updated**: 2025-11-25 MCP Gateway Instructions Resource - Build-Time Bundling

**Project Summary**: Tupl v0.9.0 delivers a production-ready multi-tenant SaaS platform combining semantic security, MCP Gateway (95-98% token reduction), and real-time telemetry. Features include Supabase OAuth, workspace isolation, NPX deployment (`fencio-gateway`), and comprehensive monitoring. Developed over 65 sessions (3 weeks). [Full history](docs/history/SESSIONS_47_63.md).

**Current Release**: v0.9.0 - Production-Ready Multi-Tenant SaaS Platform ([Release Notes](docs/releases/RELEASE_0.9.0.md))

<format>

# Current Status
- **INVESTIGATED**: Confirmed HEAD (ad296b0) only adds community health docs and prior cleanup commit (aa471c2) removed the legacy MCP/PCP/ui trees, so the working tree matches the sanitized structure.
- **FIXED**: Telemetry tenant scoping and agent ID tagging
  - Root cause: Enforcement sessions in hitlogs were tagged with `tenant_id: "default"` (from SDK) instead of authenticated tenant ID, causing telemetry API filters to return empty results
  - Solution 1: Management Plane enforcement endpoint now overrides `event.tenantId` with `current_user.id` after authentication
  - Solution 2: SDK `_get_rate_limit_context()` now uses `self.agent_id` instead of hardcoded "agent" string
  - Result: Telemetry sessions are now properly scoped to authenticated tenant and tagged with correct agent ID (e.g., "weather-agent")
  - Files changed: [enforcement.py:38](management-plane/app/endpoints/enforcement.py#L38), [agent.py:755](tupl_sdk/python/tupl/agent.py#L755), [agent.py:839](tupl_sdk/python/tupl/agent.py#L839)
- **UPDATED**: Fencio SDK + docs default to guard.fencio.dev
  - `TuplClient` and `SecureGraphProxy` now resolve the Management Plane URL from explicit args, `FENCIO_BASE_URL`/`TUPL_BASE_URL`, or fallback to `https://guard.fencio.dev`.
  - Added `docs/fencio-sdk-quickstart.md` and refreshed SDK snippets (`SDK_USAGE.md`, `mcp-ui` SDK integration) to omit `base_url` from the happy path while documenting local overrides.
  - Documented the new `FENCIO_BASE_URL` override in `docs/deployment/environment-variables.md`.
- **ADDED**: Optional SQLite-backed hitlog persistence
  - Data Plane telemetry writer now dual-writes to SQLite when `HITLOG_SQLITE_PATH` is set
  - Telemetry queries automatically use SQLite if available, otherwise fall back to file hitlogs
  - Production compose sets `HITLOG_SQLITE_PATH=/var/hitlogs/hitlogs.db` (persisted via `hitlogs` volume)
- **COMPLETED**: MCP Gateway instructions resource bundled at build time
  - Created prebuild script (scripts/embed-instructions.js) to embed docs/mcp-server-instructions.md into TypeScript
  - Updated server.ts to use embedded constant instead of runtime filesystem read
  - Fixed resource tests to expect instructions resource even with empty generated directory
  - Added unit tests for embedded instructions constant (3/3 passing)
  - Zero runtime filesystem dependencies - works identically in dev, test, and production
  - Design document: [docs/plans/2025-11-25-bundle-instructions-resource-design.md](mcp-gateway/docs/plans/2025-11-25-bundle-instructions-resource-design.md)
  - Commit: feat(mcp-gateway): bundle instructions resource at build time
- **COMPLETED**: MCP Gateway refactor Phases 1-4 (Tasks 1-13/17) - [Implementation Plan](docs/plans/2025-11-24-mcp-gateway-refactor.md)
  - **Phase 1** (Tasks 1-6): Tupl tools cleanup
    - Reduced from 8 Tupl tools to 1 tool (87.5% reduction): only wrap_agent remains
    - Created comprehensive wrap_agent SDK documentation (management_plane mode, soft_block callbacks)
    - Removed 7 unused tools and ManagementPlaneClient dependency (~250 lines deleted)
  - **Phase 2** (Tasks 7-8): Intelligence layer decoupling
    - Created IntelligenceMiddleware as reusable middleware component
    - Refactored sandbox.ts to use middleware pattern
    - Decoupled intelligence processing from sandbox execution
  - **Phase 3** (Tasks 9-11): Documentation chunking & embedding
    - Created DocumentChunker utility (512 tokens/chunk, 64 token overlap, character-based approximation)
    - Extended IntelligenceMiddleware with indexDocumentation() method
    - Automatically index wrap_agent guide on gateway startup (chunked into markdown sections)
  - **Phase 4** (Tasks 12-13): Query-based context retrieval
    - Implemented retrieveContext() in middleware with semantic search (Gemini embeddings + ChromaDB)
    - Added retrieve_context MCP tool for querying indexed documentation
    - Enables 70-90% token reduction through targeted context delivery
  - All TypeScript builds passing, unit tests passing (6/6 for new code: DocumentChunker + IntelligenceMiddleware)
  - 8 commits total for Phases 1-4
- **COMPLETED**: Removed legacy /policies page from UI
  - Removed PoliciesPage and PolicyEditorPage imports from main.tsx
  - Removed three route definitions: /console/policies, /console/policies/new, /console/policies/:agentId
  - Removed "Manage Policies" button from AgentsIndexPage header
  - Removed "Policies" navigation link from Sidebar component
  - Cleaned up unused Shield icon import
  - UI build passes with no TypeScript errors
- **FIXED**: ChromaDB integration fully configured for local and production deployments
  - Added ChromaDB service to docker-compose.local.yml and docker-compose.production.yml
  - Configured CHROMA_URL environment variable for ai-security-stack service
  - Added depends_on: chromadb to ensure proper service startup order
  - Created Supabase migration: add_embedding_metadata_column.sql for agent_policies table
  - ChromaDB now successfully stores rule anchor embeddings and syncs with Management Plane
  - Fixed "Could not connect to a Chroma server" errors in policy creation flow
- **FIXED**: Telemetry end-to-end flow operational
  - Root cause 1: TypeScript interfaces used camelCase but backend API returns snake_case field names
  - Root cause 2: Weather agent was using old `enforcement_mode="data_plane"` bypassing Management Plane
  - Root cause 3: SDK's TuplClient wasn't passing authentication token to `/api/v1/enforce` endpoint
  - Fixed TypeScript interfaces in telemetry-api.ts (SessionSummary, SessionDetail) to match backend
  - Updated weather agent to use new `enforcement_mode="management_plane"` flow
  - Added `token` parameter to TuplClient and passed it from SecureGraphProxy
  - Telemetry now flows: SDK → MP `/enforce` → encode intent → DP gRPC → record hitlog → return decision
- **FIXED**: Embedding pipeline enforcement flow fully operational
  - Root cause identified: Legacy policies existed in Supabase but were never installed to Data Plane with embeddings
  - Solution: Policies recreated via UI/API trigger full installation flow (encode anchors → store in ChromaDB → push to Data Plane)
  - All enforcement checks now return proper similarity scores and ALLOW/BLOCK decisions
- **FIXED**: SDK duplicate enforcement bug resolved
  - Root cause: LangGraph streams emit same tool_calls in multiple states; SDK was enforcing each occurrence
  - Solution: Added deduplication tracking via `_enforced_tool_call_ids` set in SecureGraphProxy
  - Reduced from 3 enforcement calls per tool to 1 (correct behavior)
- **ENHANCED**: Added comprehensive diagnostic logging to rule installation flow
  - RuleInstaller now logs all steps: anchor retrieval, gRPC channel creation, InstallRules calls, success/failure
  - Helps debug policy installation issues in both local and production environments
- **TESTED**: End-to-end NL guardrails flow validated
  - Policy creation → Anchor encoding → ChromaDB persistence → Data Plane installation → Enforcement via gRPC
  - Weather agent enforcement working correctly with 1 call per tool execution
- **EVOLVING**: Embedding pipeline architecture complete
  - Policies encode once via Management Plane, persist anchors in Chroma, push pre-encoded vectors to Data Plane
  - Added `/api/v1/enforce` proxy for Management Plane-first enforcement path
  - SDK default enforcement mode: management_plane + soft-block (legacy data_plane mode still available)
- NL guardrails core in place: templates library, NL policy parser (Gemini 2.x), Pydantic rule schemas ([Design](docs/plans/2025-11-22-natural-language-guardrails-design.md) | [Implementation Plan](docs/plans/2025-11-23-natural-language-guardrails-implementation.md))
- Local development stack fully operational ([deploy-local.sh](deployment/gateway/deploy-local.sh))
- Tasks 1–11 complete: Database schema, templates, parser, APIs, SDK auto-registration, MCP Gateway integration
- **DOCUMENTED**: System-level architecture and flows consolidated in root README
  - Replaced stale README with a platform-wide design document for v0.9.0
  - Documented Management Plane, Data Plane, SDK, MCP Gateway, UI, and Supabase/Chroma integration
  - Captured unified auth model (Supabase JWT + single Tupl API token `t_...` per user/tenant)
  - Described agent registration flow (`enforcement_agent()` → `/api/v1/agents/register` → `registered_agents`)
  - Clarified boundary_id deprecation and migration path toward agent-centric enforcement

- **DISCUSSING**: guard.fencio.dev domain migration and Nginx layout
  - Clarified roles of `deployment/gateway/nginx.conf` (host-level reverse proxy for platform.tupl.xyz → soon guard.fencio.dev) and `deployment/ui/nginx.conf` (Nginx inside the UI container), with no code changes applied yet

# Known Issues
- Repo history still contains pre-cleanup commits, so pushing refactor-branch-v0.1.0 to the new public remote would expose the deleted MCP and PCP directories via older commits until history is rewritten.
- **RESOLVED**: Production "0 rules" issue was caused by legacy policies predating embedding pipeline
  - Recreating policies via UI/API resolves the issue (triggers full encode → persist → install flow)
  - Production policies may need recreation if they show 0 rules / 0.0 similarities
- **RESOLVED**: ChromaDB connection errors ("Could not connect to a Chroma server")
  - Fixed by adding ChromaDB service to docker-compose configurations
  - Both local and production deployments now include ChromaDB with proper networking
- Agent auto-registration uses 2s HTTP timeout, so SDK often logs `Agent registration exception: timed out` even when Management Plane finishes the call; harmless but noisy
- E2E tests require ChromaDB running (expected behavior); unit tests for new code pass (6/6)
- Auth model still partially split across JWT, Tupl API tokens (`t_...`), and gateway API keys; README now defines the simplified target design but code paths need alignment
- SDK and generated wrap_agent code still require boundary_id and tenant_id parameters even though enforcement is now agent-centric

# Next steps
- Decide on a history rewrite strategy (new orphan branch or git filter-repo) before pushing to https://github.com/fencio-dev/guard.git so the public repo never exposes legacy files.
- **MCP GATEWAY REFACTOR**: Complete Phase 5 - Integration & verification (Tasks 14-17/17)
  - Task 14: Write integration tests for full refactoring (chunking, indexing, retrieval)
  - Task 15: Update documentation (README, architecture docs, tool usage examples)
  - Task 16: Run full test suite and verify all functionality works end-to-end
  - Task 17: Create final commit and pull request for review
- **DATABASE MIGRATION**: Apply embedding_metadata column migration to Supabase
  - Run `management-plane/migrations/add_embedding_metadata_column.sql` in Supabase SQL Editor
  - Required for tracking ChromaDB synchronization metadata in agent_policies table
  - Migration adds: `ALTER TABLE agent_policies ADD COLUMN embedding_metadata JSONB DEFAULT NULL`
- **PRODUCTION DEPLOYMENT**: Update production with ChromaDB configuration
  - Production docker-compose.production.yml already updated with ChromaDB service
  - Redeploy using `deployment/gateway/deploy-production.sh` to pick up changes
  - Recreate existing policies to ensure embeddings are properly stored in ChromaDB
- **SDK IMPROVEMENT**: Raise `_register_agent` timeout or make it configurable
  - Current 2s timeout causes noisy "timed out" warnings on slow Supabase responses
  - Registration succeeds but SDK logs error; harmless but confusing
- **CLEANUP**: Remove or archive `scripts/reinstall_policy.py` (debugging script no longer needed)
- **AUTH SIMPLIFICATION**: Align implementation with unified auth model
  - Centralize token resolution (JWT + `t_...`) to always derive `auth.users.id` as tenant_id
  - Gradually phase out gateway-specific API keys for the default SaaS path in favor of `t_...`
  - Wire Nginx `/internal/validate-token` to rely solely on Management Plane validation logic
- **BOUNDARY DEPRECATION**: Remove boundary_id from primary enforcement surfaces
  - Make boundary_id optional in `enforcement_agent()` and treat it as a label only
  - Update MCP `wrap_agent` tool and docs to no longer require boundary_id or explicit tenant_id
  - In a future breaking version, remove boundary_id from SDK and converge on agent-centric policies
- **DOMAIN MIGRATION**: Plan Nginx updates for platform.tupl.xyz → guard.fencio.dev
  - Update `deployment/gateway/nginx.conf` and `deployment/gateway/deploy-production.sh` to point to guard.fencio.dev and its SSL cert paths, keeping `deployment/ui/nginx.conf` focused on in-container UI routing

</format>
