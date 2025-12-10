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

Optional overrides:
- `FENCIO_BASE_URL`: Override the default `https://guard.fencio.dev` Management Plane URL (useful for local/self-hosted stacks).  
  `TUPL_BASE_URL` remains supported for backward compatibility.

## Trust Boundaries

- **developer.fencio.dev → guard.fencio.dev**: JWT + API key via URL params
- **guard.fencio.dev → MP**: X-Tenant-Id header (trusted via nginx)
- **SDK → MP**: JWT authentication (existing flow)
