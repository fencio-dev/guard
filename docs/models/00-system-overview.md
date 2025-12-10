# Tupl Platform - System Overview Mental Model

**Version:** 0.9.0
**Date:** 2025-11-22
**Status:** Production-Ready Multi-Tenant SaaS Platform

---

## Purpose

Tupl is a comprehensive security and developer productivity platform that combines:
1. **Semantic Security** - LLM-based security policy enforcement using vector embeddings
2. **MCP Gateway** - Unified interface for multiple Model Context Protocol servers with 98% token reduction
3. **SaaS Platform** - Multi-tenant web console for configuration and monitoring

The platform enables developers to:
- Enforce security policies on AI agent actions using semantic similarity
- Aggregate multiple MCP servers into a single interface
- Monitor and manage AI agent behavior through a web console
- Deploy production-ready security layers with multi-tenant isolation

---

## Architecture

### High-Level System Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         MCP UI (Console)                         │
│                    (React + Vite + TypeScript)                   │
│                                                                   │
│  - User Authentication (Supabase)                                │
│  - MCP Server Management                                         │
│  - Telemetry Visualization                                       │
│  - Settings & Token Management                                   │
└────────────────────────────┬────────────────────────────────────┘
                             │ HTTPS/REST
                             │
                             ↓
┌─────────────────────────────────────────────────────────────────┐
│                      Production Gateway                          │
│                    (Nginx Reverse Proxy)                         │
│                                                                   │
│  - SSL Termination                                               │
│  - Rate Limiting (100 req/min)                                   │
│  - CORS Configuration                                            │
│  - Service Routing                                               │
└──────────┬─────────────┬─────────────┬─────────────┬────────────┘
           │             │             │             │
           ↓             ↓             ↓             ↓
     ┌─────────┐   ┌──────────┐  ┌──────────┐  ┌──────────┐
     │   MCP   │   │  Mgmt    │  │ Control  │  │ Security │
     │ Gateway │   │  Plane   │  │  Plane   │  │  Stack   │
     │ :3000   │   │  :8000   │  │  :8001   │  │  :50051  │
     └─────────┘   └──────────┘  └──────────┘  └──────────┘
           │             │             │             │
           │             │             │             │
           ↓             ↓             ↓             ↓
     ┌─────────────────────────────────────────────────────┐
     │         Multi-Tenant Workspace Isolation             │
     │         /app/tenants/{user_id}/                      │
     └─────────────────────────────────────────────────────┘
```

### Component Relationship Diagram

```
Client Applications
    │
    ├─→ Python SDK → Management Plane (Enforcement API)
    │                      ↓
    │                 Data Plane (Rust gRPC)
    │                      ↓
    │                 Semantic Sandbox (FFI)
    │
    ├─→ MCP Gateway → Upstream MCP Servers
    │        ↓
    │   Intelligence Layer
    │   (Artifact Cache, Semantic Search)
    │
    └─→ Web UI → Management/Control Planes
             ↓
        Telemetry & Monitoring
```

---

## Technology Stack

### Frontend
- **React 18** - UI framework
- **TypeScript** - Type safety
- **Vite** - Build tooling
- **TanStack Query** - Server state management
- **Tailwind CSS** - Styling
- **Framer Motion** - Animations

### Backend Services
- **Python/FastAPI** - Management & Control Planes
- **Node.js/TypeScript** - MCP Gateway
- **Rust** - Data Plane (gRPC) & Semantic Sandbox (FFI)
- **Pydantic** - Data validation
- **tonic** - gRPC framework (Rust)

### Infrastructure
- **Nginx** - Reverse proxy & SSL termination
- **Docker Compose** - Container orchestration
- **Supabase** - Authentication & database
- **PostgreSQL** - User & token management
- **SQLite** - Local policy storage

### AI/ML
- **sentence-transformers** - Text embeddings (all-MiniLM-L6-v2)
- **NumPy** - Vector operations
- **Google Gemini** - Intelligence layer (optional)
- **ChromaDB** - Semantic artifact storage (optional)

---

## Major Components

### 1. Management Plane
- **Purpose**: Encode intents and enforce security policies
- **Technology**: Python/FastAPI
- **Location**: `management-plane/`
- **Key Features**:
  - Intent encoding to 128-dim vectors
  - Policy evaluation and enforcement
  - Telemetry collection
  - gRPC client for Data Plane
- **API Endpoints**:
  - `POST /api/v1/intents/compare` - Enforce policies
  - `POST /api/v1/encode/intent` - Encode intent events
  - `GET /api/v1/boundaries` - List policies
  - `GET /api/v1/telemetry/sessions` - Query telemetry

### 2. Data Plane
- **Purpose**: High-performance rule enforcement via gRPC
- **Technology**: Rust (tonic)
- **Location**: `tupl_data_plane/` (submodule)
- **Key Features**:
  - gRPC server for rule installation and enforcement
  - Multi-layer enforcement architecture (L0-L6)
  - TTL caching for performance
  - FFI bridge to Semantic Sandbox
- **gRPC Services**:
  - `InstallRules` - Install/update rules
  - `RemoveAgentRules` - Remove agent rules
  - `Enforce` - Evaluate enforcement decisions
  - `GetRuleStats` - Query rule statistics
  - `QueryTelemetryRPC` - Retrieve telemetry

### 3. Semantic Sandbox
- **Purpose**: Fast vector comparison via FFI
- **Technology**: Rust (CDylib)
- **Location**: `semantic-sandbox/`
- **Key Features**:
  - Slice-based cosine similarity (4 × 32-dim)
  - Anchor-based containment logic
  - Sub-millisecond comparison (<1ms)
  - FFI-safe interface (C ABI)
- **Operations**:
  - `compare_vectors` - Compare intent vs boundary
  - Min mode (all slices must pass)
  - Weighted-avg mode (soft thresholds)

### 4. Control Plane
- **Purpose**: Policy management and compilation
- **Technology**: Python/FastAPI
- **Location**: `policy_control_plane/`
- **Key Features**:
  - AgentProfile data model
  - DetBoundary policy compiler
  - Rule generation (L0-L6 layers)
  - REST API for policy CRUD
- **Security Note**: Currently single-tenant (multi-tenant security deferred to v1.1)

### 5. MCP Gateway
- **Purpose**: Unified interface for multiple MCP servers
- **Technology**: TypeScript/Node.js
- **Location**: `mcp-gateway/`
- **Key Features**:
  - Multi-server aggregation
  - Code-based tool access (95-98% token reduction)
  - Progressive disclosure via MCP Resources
  - Secure sandboxing with vm2
  - Intelligence layer with artifact caching
- **Transports**:
  - stdio (default) - Direct MCP protocol
  - HTTP (port 3000) - REST API
- **Intelligence Features**:
  - Artifact caching (reduce token usage)
  - Semantic search (ChromaDB)
  - Context summarization (Gemini)

### 6. MCP UI (Console)
- **Purpose**: Web-based management console
- **Technology**: React/TypeScript
- **Location**: `mcp-ui/`
- **Key Features**:
  - Google OAuth authentication
  - MCP server configuration (JSON-based)
  - Telemetry visualization
  - User token management
  - Settings page with HTTP MCP config
- **Pages**:
  - `/login` - Authentication
  - `/servers` - MCP server management
  - `/telemetry` - Enforcement sessions
  - `/policies` - Policy management (UI blocked - security pending)
  - `/settings` - User tokens & configuration

### 7. Python SDK
- **Purpose**: Client library for intent capture
- **Technology**: Python
- **Location**: `tupl_sdk/python/tupl/`
- **Key Features**:
  - LangGraph integration
  - SecureGraphProxy (enforcement wrapper)
  - AgentCallback (observability)
  - Remote enforcement via gRPC
  - Local enforcement via HTTP
- **Integration Points**:
  - `on_tool_start` - Capture tool calls
  - `on_llm_start` - Capture LLM calls
  - `enforcement_agent()` - Wrap LangGraph agents

### 8. MCP Tupl Server
- **Purpose**: MCP integration for Tupl platform
- **Technology**: Python
- **Location**: `mcp-tupl-server/`
- **Key Features**:
  - Expose Tupl capabilities via MCP
  - Tool definitions for Claude Code
  - Integration with Management Plane

---

## Data Flow Examples

### Semantic Security Enforcement Flow

```
1. Agent Action → SDK Captures Intent
   └─ IntentEvent: {action, resource, data, risk}

2. SDK → Management Plane POST /intents/compare
   └─ Encode intent to 128-dim vector

3. Management Plane → Filter Applicable Boundaries
   └─ Applicability scoring (attribute matching)

4. For Each Boundary:
   ├─ Encode boundary to 128-dim vector (cached)
   └─ Management Plane → Data Plane gRPC Enforce
       └─ Data Plane → Semantic Sandbox (FFI)
           └─ Compute slice similarities [4]
           └─ Return {decision: 0|1, similarities}

5. Management Plane → Aggregate Decisions
   ├─ Deny-first logic (any deny → BLOCK)
   └─ All mandatory allow must pass

6. Return Verdict to SDK
   ├─ ALLOW (1) → Continue execution
   └─ BLOCK (0) → Raise PermissionError
```

### MCP Gateway Flow

```
1. Claude Code → MCP Gateway (stdio)
   └─ Request: list_tools, call_tool, etc.

2. MCP Gateway → Upstream MCP Servers
   └─ Aggregate responses from multiple servers

3. Code-Based Tool Access
   ├─ Tools exposed as TypeScript functions
   └─ Progressive disclosure via resources

4. Intelligence Layer (Optional)
   ├─ Artifact caching → ChromaDB
   ├─ Semantic search → Find relevant artifacts
   └─ Summarization → Gemini API

5. Return Response → Claude Code
   └─ Token-efficient results
```

### Multi-Tenant Isolation

```
1. User Login → Supabase OAuth
   └─ Generate user_id and access token

2. Request → Nginx Reverse Proxy
   └─ Extract token from query params

3. MCP Gateway → SupabaseTenantResolver
   ├─ Resolve token → user_id (5-min cache)
   └─ Create workspace /app/tenants/{user_id}/

4. Service Routing
   ├─ /api/mgmt → Management Plane :8000
   ├─ /api/control → Control Plane :8001
   ├─ /api/security → Security Stack :50051
   └─ /mcp → MCP Gateway :3000

5. Workspace Isolation
   └─ Each user has isolated directory
```

---

## Security Model

### Authentication & Authorization
- **UI**: Supabase OAuth (Google)
- **API**: JWT tokens (Management Plane)
- **MCP Gateway**: Token-based tenant resolution
- **Control Plane**: ⚠️ No auth (UI access blocked until v1.1)

### Multi-Tenant Isolation
- **User Data**: Isolated workspaces per `user_id`
- **Tokens**: Auto-generated with RLS policies
- **Rate Limiting**: 100 requests/minute (Nginx)
- **CORS**: Configured for platform.tupl.xyz

### Security Layers (DetBoundary Policy)
- **L0**: System (sidecar spawn, network egress)
- **L1**: Input (schema validation, sanitization)
- **L2**: Planner (prompt assembly, length limits)
- **L3**: Model I/O (hallucination detection, redaction)
- **L4**: Tool Gateway (whitelist, parameter constraints)
- **L5**: RAG (source restrictions, sensitivity)
- **L6**: Output (data exfiltration prevention)

---

## Performance Characteristics

| Component | Target Latency | Measurement |
|-----------|----------------|-------------|
| Semantic Sandbox | <1ms | Single comparison |
| Management Plane | <10ms | Intent encoding |
| Data Plane gRPC | <5ms | Rule enforcement |
| Full Enforcement (100 rules) | <100ms | P50 latency |
| MCP Gateway | Variable | Depends on upstream |
| Token Reduction | 95-98% | vs direct tool calls |

---

## Deployment Architecture

### Production Environment
- **Platform**: AWS EC2 (Ubuntu)
- **Domain**: platform.tupl.xyz
- **SSL**: Let's Encrypt (auto-renewal)
- **Orchestration**: Docker Compose
- **Monitoring**: Logs + Telemetry UI

### Container Services
- `mcp-ui` - React frontend (port 80/443)
- `mcp-gateway` - MCP aggregation (port 3000)
- `management-plane` - Enforcement API (port 8000)
- `control-plane` - Policy management (port 8001)
- `security-stack` - Data Plane gRPC (port 50051)
- `nginx` - Reverse proxy & SSL

### Storage
- **Supabase**: User auth, tokens (PostgreSQL)
- **SQLite**: Local policy storage (Control Plane)
- **File System**: Workspace isolation (`/app/tenants/`)

---

## Development Workflow

### Local Development
```bash
# Management Plane
cd management-plane
uv run uvicorn app.main:app --reload

# Control Plane
cd policy_control_plane
uvicorn main:app --reload --port 8001

# MCP Gateway
cd mcp-gateway
npm run dev  # stdio mode
npm run dev:http  # HTTP mode

# UI
cd mcp-ui
npm run dev
```

### Testing
```bash
# Python tests
pytest management-plane/tests/
pytest tupl_sdk/python/tests/

# Rust tests
cd semantic-sandbox && cargo test
cd tupl_data_plane && cargo test

# TypeScript tests
cd mcp-gateway && npm test
cd mcp-ui && npm test
```

### Deployment
```bash
cd deployment/gateway
./deploy-production.sh
```

---

## Configuration

### Environment Variables
- `SUPABASE_URL` - Supabase project URL
- `SUPABASE_ANON_KEY` - Public API key
- `VITE_CONTROL_PLANE_URL` - Control Plane endpoint (currently disabled)
- `MCP_GATEWAY_HTTP_PORT` - Gateway HTTP port (default: 3000)
- `GEMINI_API_KEY` - For intelligence layer (optional)

### MCP Configuration
Users configure MCP servers via JSON in the UI:
```json
{
  "command": "npx",
  "args": ["-y", "@modelcontextprotocol/server-context7"],
  "env": {}
}
```

---

## Known Limitations (v0.9.0)

### Control Plane Security
- **Issue**: No authentication or tenant isolation
- **Impact**: UI access intentionally blocked
- **Mitigation**: `VITE_CONTROL_PLANE_URL` not set
- **Planned**: v1.1 (estimated 18 hours)

### Test Coverage
- **MCP Gateway**: 91/92 tests passing (99%)
- **Multi-Tenant HTTP**: 1 timeout test remaining

### Performance
- **CPU-Only PyTorch**: Used in production for smaller Docker images
- **Embedding Cache**: LRU cache (10,000 entries)

---

## Future Roadmap

### v1.0 (Next)
- Fix remaining test failures
- Improve error handling
- Performance optimization
- Documentation polish

### v1.1
- Control Plane multi-tenant security
- Enhanced telemetry features
- Additional MCP integrations
- Skills discovery for MCP Gateway

### v1.2
- Vocabulary-grounded LLM anchors
- Advanced policy authoring UI
- Real-time monitoring dashboard
- API rate limiting per tenant

---

## Related Documentation

- [README.md](../../README.md) - Project overview
- [STATUS.md](../../STATUS.md) - Current implementation status
- [plan.md](../../plan.md) - Original implementation plan
- [algo.md](../../algo.md) - Mathematical foundations
- [INTEGRATION_GUIDE.md](../../INTEGRATION_GUIDE.md) - Integration guide
- [SDK_USAGE.md](../../SDK_USAGE.md) - SDK documentation

---

## Component Mental Models

- [Management Plane](./01-management-plane.md)
- [Data Plane](./02-data-plane.md)
- [Semantic Sandbox](./03-semantic-sandbox.md)
- [Control Plane](./04-control-plane.md)
- [MCP Gateway](./05-mcp-gateway.md)
- [MCP UI](./06-mcp-ui.md)
- [Python SDK](./07-python-sdk.md)

---

**Last Updated**: 2025-11-22
**Release**: v0.9.0
**Maintainer**: Tupl Engineering Team
