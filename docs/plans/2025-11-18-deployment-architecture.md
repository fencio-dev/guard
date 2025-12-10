# Production Deployment Architecture

**Date:** 2025-11-18
**Status:** Design Complete - Ready for Implementation
**Goal:** Containerize and deploy three separate systems for production use

---

## Overview

This document outlines the deployment architecture for the complete Tupl platform, consisting of three independently deployable components:

1. **MCP Gateway** - Multi-tenant MCP server aggregation with intelligence layer
2. **AI Security Stack** - LLM security enforcement (Management Plane + Data Plane + Control Plane)
3. **UI Console** - User-facing dashboard for policy configuration and MCP server management

**Key Requirements:**
- Three separate docker-compose deployments (different EC2 instances)
- Local filesystem volumes for persistence
- Supabase for authentication
- Production-ready ASAP (simple, proven approaches)

---

## Deployment Structure

```
mgmt-plane/
├── deployment/
│   ├── gateway/
│   │   ├── docker-compose.yml
│   │   ├── Dockerfile
│   │   ├── .env.example
│   │   └── README.md
│   ├── security-stack/
│   │   ├── docker-compose.yml
│   │   ├── Dockerfile
│   │   ├── supervisord.conf
│   │   ├── .env.example
│   │   └── README.md
│   └── ui/
│       ├── docker-compose.yml
│       ├── Dockerfile
│       ├── .env.example
│       └── README.md
├── mcp-gateway/           # Source code
├── management-plane/      # Source code
├── tupl_data_plane/       # Source code
├── policy_control_plane/  # Source code
├── semantic-sandbox/      # Source code
└── mcp-ui/               # Source code
```

**Design Principles:**
- One docker-compose.yml per deployment (independent deployment units)
- Dockerfiles in deployment/ directory (clean separation of concerns)
- Volume mounts for persistent data (filesystem-based storage)
- Environment-based configuration (.env files for secrets and instance config)
- Health checks for orchestration readiness

---

## 1. MCP Gateway Deployment

### Architecture

**Services:**
- `mcp-gateway-http` - Node.js HTTP server (port 3000)
- `chromadb` - Vector store for intelligence layer (port 8001)

**Volumes:**
- `gateway-tenants:/app/tenants` - Multi-tenant storage
  - Per-tenant structure: `tenants/{tenant-id}/config.json`, `workspace/`, `generated/`
- `gateway-chromadb:/chroma/chroma` - ChromaDB persistent storage

**Environment Variables:**
```bash
GEMINI_API_KEY=<your-gemini-key>              # Required for intelligence layer
MCP_GATEWAY_TENANTS_ROOT=/app/tenants         # Tenant isolation root
MCP_GATEWAY_CHROMA_URL=http://chromadb:8001   # Internal ChromaDB connection
MCP_GATEWAY_HTTP_PORT=3000                    # HTTP server port
```

### Tenant Isolation

**API Key Authentication:**
- Tenants authenticate via `X-Tupl-Api-Key` or `Authorization: Bearer <key>` header
- API keys map to tenant IDs via `tenants.json` file
- Each tenant gets isolated directory: `/app/tenants/{tenant-id}/`

**Tenant Directory Structure:**
```
/app/tenants/{tenant-id}/
├── config.json          # MCP server configurations
├── workspace/           # User workspace files
│   └── skills/         # Reusable code snippets
└── generated/          # Generated tool wrappers
    └── servers/        # Per-server TypeScript wrappers
```

### Missing Components

1. **Health endpoint** - Add `GET /health` to http-server.ts
2. **API key management** - Generate, validate, revoke API keys
3. **Config management API** - REST endpoints for CRUD on tenant config.json
4. **Tenant initialization** - Auto-create tenant directories on first request
5. **Graceful shutdown** - Handle SIGTERM, close connections cleanly

---

## 2. AI Security Stack Deployment

### Architecture

**Single Container, Multiple Processes (supervisord):**

1. **Data Plane** (Rust) - gRPC server on port 50051 (internal)
2. **Management Plane** (Python/FastAPI) - HTTP API on port 8000 (exposed)
3. **Control Plane** (Python) - Policy API on port 8001 (exposed)

**Startup Order:** Data Plane → Management Plane → Control Plane

**Volumes:**
- `security-data:/app/data` - SQLite databases, policy storage, telemetry
- `security-models:/root/.cache/huggingface` - Cached sentence-transformer models

**Environment Variables:**
```bash
GOOGLE_API_KEY=<your-google-key>                    # For LLM anchor generation
DATA_PLANE_GRPC_URL=localhost:50051                 # Management → Data Plane
MANAGEMENT_PLANE_URL=http://localhost:8000          # Control → Management
SUPABASE_URL=<your-supabase-url>                    # For JWT validation
SUPABASE_JWT_SECRET=<your-supabase-jwt-secret>      # For JWT validation
```

### Multi-Process Management

**supervisord.conf structure:**
```ini
[supervisord]
nodaemon=true

[program:data-plane]
command=/app/bridge-server
priority=1
autostart=true
autorestart=true

[program:management-plane]
command=uvicorn app.main:app --host 0.0.0.0 --port 8000
priority=2
autostart=true
autorestart=true

[program:control-plane]
command=python server.py
priority=3
autostart=true
autorestart=true
```

### Inter-Process Communication

- **Management Plane → Data Plane:** gRPC over localhost:50051
- **Control Plane → Management Plane:** HTTP over localhost:8000
- **Shared filesystem:** Rust FFI library (`libsemantic_sandbox.so`) accessible to all

### Missing Components

1. **Health endpoints** - Add `/health` to all three components
2. **Database initialization** - SQLite schema creation on first run
3. **Boundary/Policy CRUD API** - REST endpoints in Management Plane
4. **Supabase JWT middleware** - Validate JWT tokens from UI
5. **Tenant-scoped queries** - Filter database queries by tenant_id
6. **Model download handling** - Pre-download sentence-transformers on build or handle first-run delay

---

## 3. UI Console Deployment

### Architecture

**Single Service:**
- `tupl-ui` - Nginx serving static React build (port 80/443)

**No backend needed** - Supabase handles all auth, UI is static SPA

**Volumes:**
- None (stateless application)

**Environment Variables (build-time):**
```bash
VITE_API_BASE_URL=https://security-api.tupl.io       # AI Security Stack Management Plane
VITE_GATEWAY_BASE_URL=https://gateway.tupl.io         # MCP Gateway HTTP endpoint
VITE_SUPABASE_URL=<your-supabase-url>                 # Supabase project URL
VITE_SUPABASE_ANON_KEY=<your-supabase-anon-key>       # Supabase public key
```

### Supabase Authentication Flow

1. User clicks "Sign in with Google"
2. Supabase handles OAuth flow
3. User redirected back to UI with session
4. UI stores session in localStorage (Supabase SDK handles this)
5. Protected routes check `supabase.auth.getSession()` before rendering
6. API calls include Supabase JWT in `Authorization: Bearer` header

### UI Features

**Pages to Build:**

1. **Login Page** - Supabase Google OAuth sign-in button
2. **Dashboard** - Overview of agents, policies, telemetry summary
3. **MCP Server Management**
   - List configured MCP servers
   - Add/edit/delete MCP server configurations
   - Test connection to servers
4. **API Key Management**
   - Generate new API keys for Gateway access
   - List active API keys with creation dates
   - Revoke API keys
5. **Policy Configuration**
   - Select rule families to apply (L4 only for MVP)
   - Configure boundaries for agents
   - L0-L3, L5-L6 greyed out with "Coming Soon" label
6. **Telemetry Viewer** - View enforcement decisions (allow/block)

### Missing Components

1. **Supabase integration** - Install `@supabase/supabase-js`, configure client
2. **Auth context/hooks** - React context for auth state, protected route guards
3. **MCP Server management UI** - Forms and tables for CRUD operations
4. **API Key management UI** - Generate/list/revoke interface
5. **Policy configuration UI** - Rule family selector, boundary config forms
6. **Backend API integration** - Axios client with interceptors for JWT/API keys
7. **Nginx production config** - Gzip, caching headers, SPA fallback routing

---

## Cross-Cutting Concerns

### Inter-Service Communication

| Source | Destination | Protocol | Auth |
|--------|-------------|----------|------|
| UI | MCP Gateway | HTTP REST | API Key in `X-Tupl-Api-Key` header |
| UI | Management Plane | HTTP REST | Supabase JWT in `Authorization: Bearer` |
| Management Plane | Data Plane | gRPC | localhost (same container) |
| Management Plane | Gemini API | HTTPS | `GOOGLE_API_KEY` |
| Gateway | MCP Servers | stdio/SSE | Per-server config |

### Authentication & Authorization

**UI Users (Supabase):**
- Google OAuth via Supabase
- JWT tokens stored in localStorage
- JWT validated by Management Plane using Supabase JWT secret

**Gateway API Clients:**
- API key authentication (separate from user auth)
- API keys generated in UI, stored in Gateway's `tenants.json`
- Each API key maps to a tenant ID

**Tenant Isolation:**
- Supabase user ID = tenant ID (consistent identifier)
- Gateway: Filesystem isolation per tenant
- Security Stack: Row-level filtering by tenant_id in database

### Configuration Management

**Deployment-Specific (.env files):**
- API keys and secrets (Gemini, Supabase, etc.)
- Service URLs (inter-service communication)
- Port mappings
- Volume paths

**Tenant-Specific:**
- MCP server configs (stored in Gateway filesystem per tenant)
- Policy boundaries (stored in Security Stack database per tenant)

### Health Checks & Monitoring

**Health Endpoints:**
- Gateway: `GET /health` → 200 OK if ChromaDB reachable
- Management Plane: `GET /health` → 200 OK if Data Plane gRPC healthy
- Control Plane: `GET /health` → 200 OK if Management Plane reachable
- UI: `GET /health` → 200 OK always (static site)

**Docker Compose Health Checks:**
```yaml
healthcheck:
  test: ["CMD", "curl", "-f", "http://localhost:8000/health"]
  interval: 30s
  timeout: 10s
  retries: 3
  start_period: 40s
```

**Logging:**
- All containers log to stdout/stderr
- Docker json-file logging driver (default)
- Future: Aggregate to CloudWatch/Datadog

---

## Implementation Priorities

### Phase 1: Deployment Infrastructure (Week 1)

**Goal:** Get all three components containerized and deployable

1. Create 3 Dockerfiles
2. Create 3 docker-compose.yml files
3. Create supervisord.conf for Security Stack
4. Create Nginx config for UI
5. Create .env.example files
6. Test local builds and deployments

**Success Criteria:**
- `docker-compose up` works for all three deployments
- Containers start and stay healthy
- Volumes persist data correctly

### Phase 2: Authentication Integration (Week 1)

**Goal:** Get Supabase working end-to-end

1. Set up Supabase project
2. Enable Google OAuth in Supabase
3. Add Supabase client to UI (login/logout)
4. Add JWT validation middleware to Management Plane
5. Create protected route guards in UI

**Success Criteria:**
- Users can log in via Google
- JWT tokens validated by backend
- Protected routes redirect to login when not authenticated

### Phase 3: Core UI Features (Week 2)

**Goal:** Essential user-facing functionality

1. **MCP Server Management**
   - List/add/edit/delete MCP server configs
   - Gateway needs config CRUD API endpoints
2. **API Key Management**
   - Generate/list/revoke API keys
   - Gateway needs key management API endpoints
3. **Dashboard**
   - Show active agents, policy summary
   - Basic telemetry stats

**Success Criteria:**
- Users can configure MCP servers via UI
- Users can generate API keys for Gateway access
- Changes persist and work across deployments

### Phase 4: Policy Configuration UI (Week 2)

**Goal:** L4 policy configuration interface

1. Build policy configuration pages
2. Add boundary CRUD API to Management Plane
3. Implement L4 rule family selector
4. Grey out L0-L3, L5-L6 with "Coming Soon"

**Success Criteria:**
- Users can create boundaries with L4 rules
- Policies apply to agent runs
- Telemetry shows enforcement decisions

### Phase 5: Polish & Production Readiness (Week 3)

**Goal:** Production-quality deployments

1. Add comprehensive health checks
2. Improve error handling and user feedback
3. Write deployment README files
4. Add monitoring/alerting hooks
5. Security hardening (CORS, rate limiting, input validation)
6. Performance testing

**Success Criteria:**
- Zero-downtime deployments possible
- Clear deployment documentation
- Error scenarios handled gracefully

---

## Open Questions & Decisions Needed

1. **Domain names:** What domains for each deployment? (e.g., gateway.tupl.io, api.tupl.io, console.tupl.io)
2. **SSL/TLS:** Who handles certificates? (Let's Encrypt, CloudFlare, ALB?)
3. **Database:** Should Security Stack use PostgreSQL instead of SQLite for production?
4. **Backup strategy:** How to backup volumes? (EBS snapshots, S3 sync, pg_dump?)
5. **Scaling:** Which components need horizontal scaling first? (Likely Gateway)
6. **Monitoring:** Prefer CloudWatch, Datadog, or self-hosted Prometheus?

---

## Next Steps

1. **Review & approve this design**
2. **Create implementation plan** (using writing-plans skill)
3. **Set up git worktree** for deployment work
4. **Implement in phases** (Infrastructure → Auth → Core Features → Polish)

---

## References

- [MCP Gateway README](../../mcp-gateway/README.md)
- [Management Plane pyproject.toml](../../management-plane/pyproject.toml)
- [Data Plane Architecture](../../tupl_data_plane/tupl_dp/ARCH_DIAGRAM.md)
- [STATUS.md](../../STATUS.md) - Current project status
