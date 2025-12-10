# Tupl Platform - Mental Models Index

**Version:** 0.9.0
**Date:** 2025-11-22

This directory contains comprehensive mental models for each major component of the Tupl platform. Each mental model documents the purpose, architecture, interfaces, and implementation details of a component.

---

## Component Mental Models

### Core Platform
1. **[System Overview](./00-system-overview.md)** - Complete platform architecture and component relationships
2. **[Management Plane](./01-management-plane.md)** - Python/FastAPI service for intent encoding and enforcement
3. **Data Plane** (tupl_data_plane submodule) - Rust gRPC service for high-performance rule enforcement
4. **Semantic Sandbox** (semantic-sandbox/) - Rust FFI library for vector comparison

### Control & Policy
5. **Control Plane** (policy_control_plane/) - Policy compilation and management
6. **Python SDK** (tupl_sdk/python/) - Client library for intent capture and enforcement

### Developer Experience
7. **MCP Gateway** (mcp-gateway/) - Multi-server MCP aggregation with token reduction
8. **MCP UI** (mcp-ui/) - React web console for configuration and monitoring

---

## Component Dependency Graph

```
┌─────────────────────────────────────────────────────────┐
│                      MCP UI (Console)                    │
│                     (User Interface)                     │
└────────────┬───────────────┬──────────────┬─────────────┘
             │               │              │
             ↓               ↓              ↓
    ┌───────────────┐ ┌──────────────┐ ┌──────────────┐
    │  MCP Gateway  │ │  Management  │ │   Control    │
    │               │ │    Plane     │ │    Plane     │
    └───────┬───────┘ └──────┬───────┘ └──────┬───────┘
            │                │                 │
            │                ↓                 │
            │         ┌──────────────┐         │
            │         │  Data Plane  │         │
            │         │   (gRPC)     │         │
            │         └──────┬───────┘         │
            │                ↓                 │
            │         ┌──────────────┐         │
            │         │  Semantic    │         │
            │         │  Sandbox     │         │
            │         │   (FFI)      │         │
            │         └──────────────┘         │
            │                                  │
            ↓                                  ↓
    ┌──────────────┐                  ┌──────────────┐
    │ Upstream MCP │                  │ Rule         │
    │   Servers    │                  │ Instances    │
    └──────────────┘                  └──────────────┘

    ┌──────────────┐
    │  Python SDK  │ ─────→ Management Plane
    │              │        (Intent Capture)
    └──────────────┘
```

---

## Component Summary

### Management Plane
- **Purpose**: Encode intents and enforce semantic security policies
- **Technology**: Python/FastAPI
- **Key Features**: 128-dim vector encoding, applicability filtering, gRPC coordination
- **API**: `/api/v1/intents/compare`, `/api/v1/encode/*`, `/api/v1/boundaries`
- **Performance**: <10ms encoding, <100ms full enforcement

### Data Plane
- **Purpose**: High-performance rule enforcement via gRPC
- **Technology**: Rust (tonic)
- **Key Features**: Multi-layer enforcement (L0-L6), TTL caching, FFI bridge
- **Services**: `InstallRules`, `Enforce`, `GetRuleStats`, `QueryTelemetryRPC`
- **Performance**: <5ms per enforcement call

### Semantic Sandbox
- **Purpose**: Fast vector comparison via FFI
- **Technology**: Rust (CDylib)
- **Key Features**: Slice-based cosine similarity, anchor containment, sub-millisecond
- **Interface**: C ABI (`compare_vectors`)
- **Performance**: <1ms per comparison

### Control Plane
- **Purpose**: Policy compilation and management
- **Technology**: Python/FastAPI
- **Key Features**: AgentProfile model, DetBoundary compiler, L0-L6 rule generation
- **API**: `/api/v1/agents`, `/api/v1/policies`
- **Status**: ⚠️ Single-tenant (multi-tenant security deferred to v1.1)

### MCP Gateway
- **Purpose**: Unified interface for multiple MCP servers
- **Technology**: TypeScript/Node.js
- **Key Features**: Multi-server aggregation, 95-98% token reduction, intelligence layer
- **Transports**: stdio (default), HTTP (port 3000)
- **Intelligence**: Artifact caching, semantic search, summarization

### MCP UI
- **Purpose**: Web-based management console
- **Technology**: React/TypeScript
- **Key Features**: OAuth authentication, server management, telemetry visualization
- **Pages**: Login, Servers, Telemetry, Settings
- **Deployment**: platform.tupl.xyz

### Python SDK
- **Purpose**: Client library for intent capture
- **Technology**: Python
- **Key Features**: LangGraph integration, SecureGraphProxy, remote/local enforcement
- **Integration**: `enforcement_agent()`, `AgentCallback`
- **Modes**: `audit` (log only), `block` (raise exception)

---

## Technology Stack Summary

### Languages
- **Python 3.11+** - Management Plane, Control Plane, SDK
- **Rust 1.75+** - Data Plane, Semantic Sandbox
- **TypeScript 5.0+** - MCP Gateway, MCP UI
- **JavaScript (Node 18+)** - MCP Gateway runtime

### Frameworks
- **FastAPI** - Python web services
- **tonic** - Rust gRPC framework
- **React 18** - UI framework
- **Vite** - Build tooling

### AI/ML
- **sentence-transformers** - Text embeddings
- **all-MiniLM-L6-v2** - Embedding model (384-dim)
- **NumPy** - Vector operations
- **Google Gemini** - Intelligence layer (optional)

### Infrastructure
- **Docker Compose** - Container orchestration
- **Nginx** - Reverse proxy & SSL
- **Supabase** - Auth & database
- **PostgreSQL** - User management
- **SQLite** - Local storage

---

## Data Flow Patterns

### Semantic Enforcement
```
Client App + SDK
    ↓ IntentEvent
Management Plane
    ↓ Encode (128-dim)
    ↓ Filter Boundaries
    ↓ gRPC
Data Plane
    ↓ FFI
Semantic Sandbox
    ↓ Similarities
Management Plane (Aggregate)
    ↓ Decision
SDK (Enforce/Log)
```

### MCP Aggregation
```
Claude Code
    ↓ MCP Protocol
MCP Gateway
    ├─→ Upstream Server 1
    ├─→ Upstream Server 2
    └─→ Upstream Server 3
    ↓ Aggregate
    ↓ Code-based access
    ↓ Intelligence layer (optional)
Claude Code
```

### Multi-Tenant Isolation
```
User Login (OAuth)
    ↓ Supabase
Token Generation
    ↓
Nginx (Token extraction)
    ↓
Tenant Resolver (5-min cache)
    ↓ user_id
Workspace Isolation
    /app/tenants/{user_id}/
```

---

## Integration Points

### External Services
- **Supabase** - OAuth, user database, token management
- **Google Gemini** - Intelligence layer (optional)
- **ChromaDB** - Semantic artifact storage (optional)
- **Upstream MCP Servers** - Context7, FileSystem, etc.

### Internal Communication
- **HTTP/REST** - UI → Management/Control Planes
- **gRPC** - Management Plane → Data Plane
- **FFI (C ABI)** - Data Plane → Semantic Sandbox
- **MCP Protocol** - Gateway ↔ Upstream servers

---

## Security Model

### Authentication
- **UI**: Supabase OAuth (Google)
- **Management Plane**: JWT tokens
- **MCP Gateway**: Token-based tenant resolution
- **Control Plane**: ⚠️ No auth (UI blocked)

### Authorization
- **Tenant Isolation**: Per-user workspaces
- **RLS Policies**: Supabase Row Level Security
- **Rate Limiting**: 100 req/min (Nginx)

### Enforcement Layers
- **L0**: System (sidecar, network)
- **L1**: Input (schema, sanitization)
- **L2**: Planner (prompt assembly)
- **L3**: Model I/O (hallucination detection)
- **L4**: Tool Gateway (whitelist, constraints)
- **L5**: RAG (source restrictions)
- **L6**: Output (data exfiltration)

---

## Performance Targets

| Component | Latency | Throughput |
|-----------|---------|------------|
| Semantic Sandbox | <1ms | N/A |
| Management Plane (encoding) | <10ms | N/A |
| Data Plane (enforcement) | <5ms | N/A |
| Full Stack (100 rules) | <100ms P50 | N/A |
| MCP Gateway | Variable | Depends on upstreams |
| Token Reduction | N/A | 95-98% vs direct |

---

## Testing Coverage

### Unit Tests
- **Management Plane**: `pytest management-plane/tests/`
- **Semantic Sandbox**: `cargo test` (Rust)
- **Data Plane**: `cargo test` (Rust)
- **MCP Gateway**: `npm test` (91/92 passing)

### Integration Tests
- **FFI Bridge**: Python ↔ Rust
- **gRPC**: Management Plane ↔ Data Plane
- **E2E**: SDK → Full stack
- **Multi-Tenant**: HTTP isolation

---

## Deployment Architecture

### Production Environment
- **Platform**: AWS EC2 (Ubuntu 22.04)
- **Domain**: platform.tupl.xyz
- **SSL**: Let's Encrypt (auto-renewal)
- **Orchestration**: Docker Compose

### Container Services
```yaml
services:
  mcp-ui:        # React frontend
  mcp-gateway:   # MCP aggregation
  management-plane:  # Enforcement API
  control-plane: # Policy management
  security-stack:    # Data Plane gRPC
  nginx:         # Reverse proxy
```

### Persistence
- **Supabase Cloud**: User auth, tokens
- **SQLite**: Local policy storage
- **File System**: Workspace isolation

---

## Configuration Files

### Environment
- `deployment/gateway/.env` - Production secrets
- `deployment/ui/.env.production` - UI config
- `mcp-ui/supabase/.env.local` - Supabase config

### Docker
- `deployment/gateway/docker-compose.production.yml`
- `deployment/ui/Dockerfile`
- `deployment/gateway/nginx.conf`

### MCP
- User-defined JSON in UI (per-server config)
- `.mcp.json` - Local MCP configuration

---

## Known Issues (v0.9.0)

### Critical
- **Control Plane Security**: No authentication/tenant isolation (UI blocked)

### Minor
- **MCP Gateway Tests**: 1/92 tests failing (timeout)
- **Multi-Tenant HTTP**: 1 remaining timeout test

### Performance
- **CPU-Only PyTorch**: Slower than GPU but smaller images
- **Embedding Cache**: Limited to 10K entries

---

## Development Workflow

### Local Setup
```bash
# Management Plane
cd management-plane && uv run uvicorn app.main:app --reload

# Control Plane
cd policy_control_plane && uvicorn main:app --reload --port 8001

# MCP Gateway
cd mcp-gateway && npm run dev

# UI
cd mcp-ui && npm run dev
```

### Testing
```bash
# Python
pytest management-plane/tests/
pytest tupl_sdk/python/tests/

# Rust
cd semantic-sandbox && cargo test
cd tupl_data_plane && cargo test

# TypeScript
cd mcp-gateway && npm test
cd mcp-ui && npm test
```

### Deployment
```bash
cd deployment/gateway
./deploy-production.sh
```

---

## Documentation Links

- [README.md](../../README.md) - Project overview
- [STATUS.md](../../STATUS.md) - Current status
- [plan.md](../../plan.md) - Implementation plan
- [algo.md](../../algo.md) - Mathematical foundations
- [INTEGRATION_GUIDE.md](../../INTEGRATION_GUIDE.md)
- [SDK_USAGE.md](../../SDK_USAGE.md)

---

## Roadmap

### v1.0 (Next)
- Fix remaining test failures
- Performance optimization
- Documentation polish

### v1.1
- Control Plane multi-tenant security
- Enhanced telemetry
- Skills discovery

### v1.2
- Vocabulary-grounded LLM anchors
- Advanced policy UI
- Real-time monitoring

---

**Last Updated**: 2025-11-22
**Release**: v0.9.0
**Maintainer**: Tupl Engineering Team
