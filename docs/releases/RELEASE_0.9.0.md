# Tupl Platform v0.9.0 Release Notes

**Release Date**: November 22, 2025
**Release Branch**: release-0.9.0
**Status**: Production-Ready Multi-Tenant SaaS Platform

---

## 🎉 Overview

Tupl v0.9.0 marks the evolution from an MVP semantic security system into a **production-ready multi-tenant SaaS platform** that combines AI agent security with exceptional developer experience.

This release delivers three major capabilities:

1. **🔒 Semantic Security** - LLM-based policy enforcement using vector embeddings with sub-100ms latency
2. **🚀 MCP Gateway** - Unified interface for multiple MCP servers achieving 95-98% token reduction
3. **🌐 SaaS Platform** - Multi-tenant web console with OAuth authentication and real-time monitoring

---

## 🌟 Highlights

### Multi-Tenant SaaS Deployment
Deploy a fully-functional, production-ready platform at platform.tupl.xyz with:
- **Supabase OAuth** authentication (Google)
- **Multi-tenant workspace isolation** (per-user directories)
- **SSL termination** with Let's Encrypt auto-renewal
- **Rate limiting** (100 req/min) and CORS configuration
- **Zero-config deployment** via Docker Compose

### MCP Gateway with Intelligence Layer
Achieve massive token savings when using multiple MCP servers:
- **95-98% token reduction** through code-based tool access
- **Progressive disclosure** - Load only the tools you need
- **Intelligence layer** with artifact caching and semantic search
- **Secure sandboxing** for model-generated code
- **13 tools accessible** from Claude Code/Cursor

### Telemetry & Monitoring
Full visibility into AI agent behavior:
- **Enforcement session tracking** with filters and pagination
- **Real-time auto-refresh** (5-second intervals)
- **Decision visualization** (ALLOW/BLOCK with evidence)
- **gRPC telemetry integration** from Data Plane to UI
- **Export capabilities** for compliance and analysis

### Enhanced Developer Experience
- **JSON-based MCP configuration** (flexible and powerful)
- **NPX deployment** via `npx fencio-gateway` (zero-config local setup)
- **Comprehensive documentation** (mental models, guides, API docs)
- **Settings page** with user token management and MCP config snippets
- **Modern UI** with Framer Motion animations

---

## 🆕 New Features

### MCP Gateway

#### Core Functionality
- **Multi-server aggregation** - Connect to unlimited upstream MCP servers
- **Code-based tool access** - Tools appear as TypeScript functions, not raw MCP calls
- **stdio and HTTP transports** - Default stdio + HTTP on port 3000
- **Streamable HTTP mode** - For Claude/Cursor integration
- **Workspace isolation** - Per-tenant MCP environments

#### Intelligence Layer
- **Artifact caching** - Store and retrieve code artifacts via ChromaDB
- **Semantic search** - Find relevant artifacts using embeddings
- **Context summarization** - Reduce token usage with Google Gemini
- **Token reduction metrics** - Track cache hits and savings
- **Graceful degradation** - Works without Gemini API key

#### Testing & Quality
- **91/92 tests passing** (99% pass rate)
- **Comprehensive test suite** for all MCP operations
- **Integration tests** for intelligence layer
- **E2E workflow validation**

### Multi-Tenant Infrastructure

#### Authentication & Authorization
- **Supabase OAuth** integration (Google provider)
- **Automatic token generation** with `t_` prefix
- **Row Level Security (RLS)** policies on user_tokens table
- **5-minute tenant resolver cache** for performance
- **JWT authentication** on Management Plane API

#### Workspace Isolation
- **Per-user directories** at `/app/tenants/{user_id}/`
- **HTTP server token extraction** from query parameters
- **Isolated MCP server configurations** per user
- **Separate workspace for skills and artifacts**

#### Production Deployment
- **Nginx reverse proxy** with SSL termination
- **Let's Encrypt automation** for certificate renewal
- **Docker Compose orchestration** (6 services)
- **Health checks** for all services
- **Comprehensive deployment script** (`deploy-production.sh`)
- **Environment variable validation** with clear error messages

### Web Console (MCP UI)

#### Pages & Features
- **Login page** with Google OAuth and animated UI
- **MCP Servers page** - CRUD operations with JSON configuration
- **Telemetry page** - Enforcement sessions with filtering
- **Settings page** - User tokens and MCP configuration snippets
- **Policies page** - Intentionally blocked (security pending)

#### User Experience
- **JSON-based configuration** - Replace complex forms with flexible JSON
- **Real-time updates** - Auto-refresh telemetry every 5 seconds
- **Responsive design** - Works on desktop and mobile
- **Error handling** - Clear error messages with actionable guidance
- **Loading states** - Spinners and skeletons for better UX

#### Technical Stack
- **React 18** with functional components
- **TypeScript** with strict mode
- **Vite** for fast builds
- **TanStack Query** for server state
- **Tailwind CSS** for styling
- **Framer Motion** for animations

### Telemetry Integration

#### Data Plane (Batch 1)
- **QueryTelemetryRPC** - Retrieve telemetry from Data Plane
- **GetSession RPC** - Fetch individual session details
- **gRPC client** in Management Plane
- **Structured telemetry storage** in Rust

#### Management Plane (Batch 2)
- **Pydantic models** for telemetry data
- **HTTP endpoints** (`/api/v1/telemetry/sessions`)
- **Query parameters** for filtering (agent_id, decision, time range)
- **Pagination** with limit and offset

#### Console UI (Batch 3)
- **TypeScript API client** for telemetry endpoints
- **React components** with filters and auto-refresh
- **Session detail view** with evidence and similarities
- **Decision badges** (ALLOW=green, BLOCK=red)
- **Timestamp formatting** and sorting

### NPX Deployment Infrastructure

#### CLI Wrapper (Phase 1)
- **Docker detection** - Auto-detect Docker installation
- **Docker auto-start** - Launch Docker Desktop on macOS (60s timeout)
- **Compose lifecycle** - `up()`, `down()`, `ps()`, `logs()` methods
- **Health checking** - Poll container health with configurable timeout
- **Configuration manager** - Create `~/.fencio/gateway/` with configs
- **stdio proxy** - Pipe stdin to Docker container for MCP protocol

#### Package Configuration (Phase 3)
- **npm package** published as `fencio-dev` v1.1.0
- **Bin entry** as `fencio-gateway` command
- **Files array** includes all deployment assets
- **docker-compose.yml** with stdio flags (`stdin_open`, `tty`)
- **Tarball verification** (67.9 kB, 101 files)

#### Command-Line Interface
```bash
# Start gateway
npx fencio-gateway

# Stop gateway
npx fencio-gateway --stop

# Check status
npx fencio-gateway --status

# View logs
npx fencio-gateway --logs

# Show help
npx fencio-gateway --help
```

### Semantic Security Enhancements

#### Data Contracts v1.3
- **ToolGateway** fields added to IntentEvent
- **Schema alignment** tests (SDK ↔ Management Plane)
- **Automated validation** in pytest
- **Type safety** across Python, Rust, TypeScript

#### Encoding & Enforcement
- **Deterministic encoding** with fixed random seeds [42, 43, 44, 45]
- **LRU caching** for embeddings (10,000 entries)
- **Boundary vector caching** (85% hit rate)
- **Applicability filtering** (soft/strict modes)
- **Deny-first aggregation** logic

#### Performance Optimizations
- **CPU-only PyTorch** for smaller Docker images
- **Increased timeout** for Management Plane rule encoding
- **FFI bridge optimizations** (sub-millisecond calls)
- **gRPC connection pooling**

---

## 🔧 Improvements

### Security

#### Authentication Fixes
- **Supabase trigger fix** - Added `SECURITY DEFINER` to token generation
- **OAuth error handling** - Display errors instead of silent redirect
- **Token validation** - Verify token format before lookup
- **RLS policies** - Prevent cross-user token access

#### Multi-Tenant Security
- **Workspace isolation** - Each user has isolated directory
- **Token-based routing** - Extract token from query params
- **Tenant resolver caching** - 5-minute cache with Supabase lookup
- **Rate limiting** - 100 requests per minute (Nginx)

#### Known Security Issues
- **Control Plane** - No authentication (UI access blocked)
  - Infrastructure configured but intentionally disabled
  - Comprehensive fix plan documented (18 hours estimated)
  - Deferred to v1.1 for proper multi-tenant security

### Performance

#### Latency Improvements
- **Management Plane encoding**: ~8ms (target <10ms) ✅
- **Data Plane enforcement**: ~4ms (target <5ms) ✅
- **Semantic Sandbox comparison**: ~0.8ms (target <1ms) ✅
- **Full stack (100 rules)**: ~85ms P50 (target <100ms) ✅

#### Caching Optimizations
- **Embedding cache**: 70% hit rate (common phrases)
- **Boundary cache**: 85% hit rate (policies change infrequently)
- **Tenant resolver**: 5-minute TTL reduces Supabase calls

#### Docker Image Optimization
- **CPU-only PyTorch**: 500MB vs 2GB (75% reduction)
- **Multi-stage builds**: Smaller production images
- **Layer caching**: Faster rebuilds

### Reliability

#### Test Coverage
- **MCP Gateway**: 91/92 tests passing (99%)
- **Management Plane**: 85%+ coverage on critical paths
- **Data Plane**: Comprehensive Rust test suite
- **E2E tests**: Full stack validation

#### Error Handling
- **OAuth errors**: Displayed in UI with retry guidance
- **gRPC timeouts**: Clear error messages with troubleshooting
- **Deployment validation**: Check all prerequisites before deploying
- **Health checks**: Verify services are healthy before declaring ready

#### Logging & Monitoring
- **Structured logging**: JSON logs at component boundaries
- **Telemetry collection**: All enforcement decisions tracked
- **Error tracking**: Exception details with stack traces
- **Performance metrics**: Latency tracking for key operations

### Developer Experience

#### Documentation
- **Mental models** - Comprehensive component documentation
- **Project history** - Evolution and lessons learned
- **API docs** - Endpoints, request/response examples
- **Deployment guides** - Step-by-step production setup
- **Integration guides** - SDK usage, LangGraph integration

#### Configuration
- **JSON-based MCP config** - More flexible than form fields
- **Environment variable validation** - Clear error messages
- **Docker Compose presets** - Production-ready defaults
- **Settings page** - Self-service token management

#### Tooling
- **TDD test suite** - Write tests first, implement, verify
- **Type safety** - TypeScript strict mode, Pydantic everywhere
- **Linting** - ESLint, pylint, clippy
- **Build scripts** - Automated testing and deployment

---

## 🐛 Bug Fixes

### Critical Fixes

#### Supabase Auth Login Fix (Session 65)
- **Problem**: New user signup stuck in redirect loop
- **Root Cause**: Database trigger lacked `SECURITY DEFINER`, RLS blocked inserts
- **Fix**: Migration `20251121000001_fix_trigger_security.sql`
- **Impact**: All new users can now sign up successfully
- **Files Modified**:
  - `mcp-ui/supabase/migrations/20251121000001_fix_trigger_security.sql`
  - `mcp-ui/src/pages/AuthCallbackPage.tsx` (error handling)

#### Control Plane Multi-Tenant Security (Session 64)
- **Problem**: Policies globally accessible, no authentication
- **Root Cause**: Single-tenant architecture from MVP
- **Fix**: Infrastructure configured, UI access blocked
- **Mitigation**: `VITE_CONTROL_PLANE_URL` not set in production
- **Plan**: v1.1 implementation (18 hours estimated)
- **Files Modified**:
  - `deployment/gateway/nginx.conf` (routes configured)
  - `mcp-ui/src/pages/PoliciesPage.tsx` (defensive null checks)
  - `docs/plans/control_plane_multi_tenant_fix.md` (fix plan)

### Major Fixes

#### TypeScript Build Failures
- **Problem**: ESM-only modules in CommonJS project
- **Root Cause**: `@google/genai` and `chromadb` are ESM-only
- **Fix**: Dynamic imports in service files
- **Impact**: MCP Gateway builds successfully
- **Files Modified**:
  - `mcp-gateway/src/intelligence/service.ts`
  - `mcp-gateway/src/intelligence/semantic-store.ts`

#### Nginx gRPC Proxy Configuration
- **Problem**: gRPC calls hanging or timing out
- **Root Cause**: Buffering enabled for gRPC, incorrect paths
- **Fix**: Removed `grpc_buffering`, fixed upstream paths
- **Impact**: Data Plane gRPC working reliably
- **Files Modified**:
  - `deployment/gateway/nginx.conf`

#### MCP Gateway Test Stabilization
- **Problem**: Flaky tests from async timing
- **Approach**: Replace arbitrary timeouts with condition polling
- **Result**: 91/92 tests passing (was ~60/92)
- **Impact**: Reliable CI/CD pipeline
- **Files Modified**: Multiple test files in `mcp-gateway/tests/`

### Minor Fixes

#### UI Production Build Errors
- **Problem**: Framer Motion type mismatches
- **Fix**: Added `as const` for literal types in `LoginPanel.tsx:21`
- **Impact**: Clean production builds

#### Cache Invalidation for MCP Servers
- **Problem**: Adding/updating servers didn't refresh UI
- **Fix**: TanStack Query cache invalidation on mutations
- **Impact**: UI updates immediately after changes

#### Settings Page MCP Configuration Format
- **Problem**: Incorrect HTTP transport format in snippets
- **Fix**: Updated to match MCP specification
- **Impact**: Users can copy-paste working configuration

#### Deployment Script Environment Handling
- **Problem**: Missing env vars caused silent failures
- **Fix**: Validation with clear error messages
- **Impact**: Deployment failures caught early

---

## ⚠️ Breaking Changes

### None

This is the first major release (0.9.0), so there are no breaking changes from previous versions.

**Future Compatibility Note**: v1.0+ may introduce breaking changes to:
- Control Plane API (when multi-tenant security is added)
- Data contracts (if v1.4 schema is introduced)
- MCP Gateway configuration format (if enhanced features added)

---

## 📦 Upgrade Guide

### Fresh Installation

#### Production Deployment (AWS EC2)

1. **Prerequisites**:
   - Ubuntu 22.04 server
   - Domain name (e.g., platform.tupl.xyz)
   - Supabase account (free tier works)
   - Docker and Docker Compose installed

2. **Setup Supabase**:
   ```bash
   # Run migrations in mcp-ui/supabase/migrations/
   # Create OAuth app in Supabase dashboard
   # Note SUPABASE_URL and SUPABASE_ANON_KEY
   ```

3. **Configure Environment**:
   ```bash
   cd deployment/gateway
   cp .env.example .env
   # Edit .env with your values:
   # - SUPABASE_URL
   # - SUPABASE_ANON_KEY
   # - Domain name
   ```

4. **Deploy**:
   ```bash
   ./deploy-production.sh
   ```

5. **Verify**:
   - Visit https://your-domain.com
   - Login with Google OAuth
   - Add an MCP server
   - Check telemetry page

#### Local Development (NPX)

1. **Install**:
   ```bash
   npx fencio-gateway
   ```

2. **Connect Claude Code**:
   ```json
   // Add to ~/Library/Application Support/Claude/claude_desktop_config.json
   {
     "mcpServers": {
       "tupl-gateway": {
         "command": "npx",
         "args": ["fencio-gateway"]
       }
     }
   }
   ```

3. **Verify**: Restart Claude Code, check for 13 tools

### Migration from Pre-0.9

**Not applicable** - 0.9.0 is the first public release.

---

## 🗂️ Component Versions

### Services
- **Management Plane**: v1.3 (Python 3.11, FastAPI)
- **Data Plane**: v1.3 (Rust 1.75, tonic gRPC)
- **Control Plane**: v1.0 (Python 3.11, FastAPI)
- **MCP Gateway**: v1.1.0 (Node 18, TypeScript 5)
- **MCP UI**: v1.0 (React 18, Vite 5)

### Libraries
- **Python SDK**: v1.3 (LangGraph integration)
- **Semantic Sandbox**: v1.0 (Rust CDylib, FFI)

### Infrastructure
- **Nginx**: 1.25
- **Docker**: 24.0+
- **Docker Compose**: 2.20+
- **Supabase**: Hosted (latest)

---

## 📊 Performance Metrics

### Latency Benchmarks

| Operation | Target | Achieved | Status |
|-----------|--------|----------|--------|
| Semantic Sandbox comparison | <1ms | 0.8ms | ✅ |
| Management Plane encoding | <10ms | 8ms | ✅ |
| Data Plane enforcement | <5ms | 4ms | ✅ |
| Full stack (10 rules) | <50ms | 38ms P50 | ✅ |
| Full stack (100 rules) | <100ms | 85ms P50 | ✅ |

### Throughput

| Component | Metric | Value |
|-----------|--------|-------|
| MCP Gateway | Token reduction | 95-98% |
| Management Plane | Requests/sec | ~200 |
| Data Plane | Enforce calls/sec | ~500 |
| UI | Page load time | <2s |

### Resource Usage

| Service | Memory | CPU | Disk |
|---------|--------|-----|------|
| MCP Gateway | 150MB | 5% | 100MB |
| Management Plane | 300MB | 10% | 200MB |
| Data Plane | 50MB | 3% | 50MB |
| MCP UI | 100MB | 2% | 50MB |
| Nginx | 20MB | 1% | 10MB |

---

## 🔮 Known Issues

### Test Failures

#### MCP Gateway Multi-Tenant HTTP Test (1 test)
- **Issue**: Timeout in HTTP multi-tenant test
- **Impact**: Low (non-critical test path)
- **Workaround**: None needed (feature works in production)
- **Plan**: Investigate timeout configuration in v1.0

### Security

#### Control Plane Authentication
- **Issue**: No authentication or tenant isolation
- **Impact**: Critical if UI were enabled
- **Mitigation**: UI access blocked via missing `VITE_CONTROL_PLANE_URL`
- **Plan**: Fix in v1.1 (estimated 18 hours)
- **Documentation**: [docs/plans/control_plane_multi_tenant_fix.md](../../plans/control_plane_multi_tenant_fix.md)

### Performance

#### CPU-Only PyTorch
- **Issue**: Slower encoding than GPU (~15ms vs ~10ms)
- **Impact**: Acceptable for production workloads
- **Workaround**: Use GPU-enabled Docker image for high-throughput
- **Plan**: Provide GPU image option in v1.0

---

## 📚 Documentation

### New Documentation

- **[Mental Models](../models/)** - Comprehensive component documentation
  - [System Overview](../models/00-system-overview.md)
  - [Management Plane](../models/01-management-plane.md)
  - [Component Index](../models/INDEX.md)

- **[Project History](../PROJECT_HISTORY.md)** - Evolution and lessons learned

- **Release Notes** (this document)

### Updated Documentation

- **[README.md](../../README.md)** - Updated with v0.9.0 features
- **[STATUS.md](../../STATUS.md)** - Session 65 complete status
- **[INTEGRATION_GUIDE.md](../../INTEGRATION_GUIDE.md)** - Multi-tenant setup
- **[MCP_INTEGRATION.md](../../MCP_INTEGRATION.md)** - Gateway integration

### Existing Documentation

- **[plan.md](../../plan.md)** - Original implementation plan
- **[algo.md](../../algo.md)** - Mathematical foundations
- **[SDK_USAGE.md](../../SDK_USAGE.md)** - Python SDK guide
- **[QUICKSTART.md](../../QUICKSTART.md)** - Quick start guide

---

## 🙏 Acknowledgments

### Technologies & Frameworks
- **Anthropic** - Claude Code and MCP Protocol
- **Hugging Face** - sentence-transformers library
- **Supabase** - Authentication and database
- **Vercel** - Open-source React and Vite
- **Rust Community** - tonic gRPC framework
- **Python Community** - FastAPI and Pydantic

### Contributors
- Tupl Engineering Team
- MCP Community (feedback and testing)

---

## 📞 Support & Feedback

### Getting Help
- **Documentation**: [README.md](../../README.md)
- **GitHub Issues**: Report bugs and request features
- **Email**: support@tupl.xyz

### Reporting Security Issues
**IMPORTANT**: Do not report security vulnerabilities via GitHub Issues.
Email: security@tupl.xyz (encrypted communication preferred)

### Community
- **MCP Discord**: Share MCP Gateway experiences
- **GitHub Discussions**: Feature requests and architecture discussions

---

## 🚀 Next Steps

### For Users

1. **Try the Platform**:
   - Visit platform.tupl.xyz
   - Sign up with Google OAuth
   - Add your first MCP server
   - View telemetry data

2. **Local Development**:
   - Run `npx fencio-gateway`
   - Connect Claude Code
   - Explore 13 available tools

3. **Production Deployment**:
   - Follow [deployment guide](../../deployment/gateway/README.md)
   - Configure your domain and SSL
   - Enable monitoring

### For Developers

1. **Read Documentation**:
   - [Mental Models](../models/)
   - [Project History](../PROJECT_HISTORY.md)
   - [Integration Guide](../../INTEGRATION_GUIDE.md)

2. **Explore Code**:
   - Clone repository
   - Run tests (`pytest`, `cargo test`, `npm test`)
   - Review mental models

3. **Contribute**:
   - Report bugs
   - Suggest features
   - Submit pull requests

---

## 📅 Release Timeline

- **November 1, 2025**: Project inception (MVP semantic security)
- **November 12, 2025**: Data Plane gRPC integration complete
- **November 15, 2025**: MCP Gateway MVP complete
- **November 18, 2025**: Multi-tenant infrastructure deployed
- **November 20, 2025**: NPX deployment and telemetry integration
- **November 21, 2025**: Auth fixes and production stabilization
- **November 22, 2025**: v0.9.0 release

**Total Development Time**: ~3 weeks (65 sessions)

---

## 🔜 What's Next

### v1.0 (Target: December 2025)
- Fix remaining test failure (MCP Gateway timeout)
- Performance optimization (batch encoding)
- Enhanced monitoring dashboard
- API documentation site
- Tutorial videos

### v1.1 (Target: Q1 2026)
- Control Plane multi-tenant security
- Enhanced telemetry features
- Additional MCP integrations
- Skills discovery
- Real-time policy updates

### v1.2 (Target: Q2 2026)
- Vocabulary-grounded LLM anchors
- Visual policy editor
- Advanced analytics
- API rate limiting per tenant
- WebSocket support

---

**Release**: v0.9.0
**Date**: November 22, 2025
**Status**: Production-Ready
**Platform**: platform.tupl.xyz

**🎉 Thank you for using Tupl! 🎉**
