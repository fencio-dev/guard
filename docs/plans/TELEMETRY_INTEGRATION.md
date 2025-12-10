# Telemetry Integration Plan: Hitlogs → Console UI

**Status:** In Progress
**Created:** 2025-11-20
**Owner:** Engineering
**Implementation Started:** 2025-11-20

## Implementation Approach

**Test-Driven Development (TDD)**
This plan will be executed using a rigorous TDD approach:
1. Write tests FIRST for each component (gRPC handlers, API endpoints, UI components)
2. Watch tests FAIL to verify they test real behavior
3. Write MINIMAL code to make tests pass
4. Refactor only after tests are green

**Batch Execution with Review Checkpoints**
- Execute in 3-task batches using the executing-plans skill
- Report progress after each batch for review
- Apply feedback before proceeding to next batch
- Verify all tests pass before marking tasks complete

**Key Principles:**
- Tests before implementation (no exceptions)
- One failing test at a time
- Verify test failure before writing implementation
- Run all tests before claiming completion
- Document decisions and approach for future sessions

## Overview
Connect the Data Plane hitlog telemetry system to the Console UI's Agents page, enabling users to see complete enforcement context for every agent run.

## Current Architecture Understanding

**User Workflow:**
1. User builds LangGraph agents in their IDE using MCP tools
2. User wraps agent: `secure_agent = enforcement_agent(agent, boundary_id="ops")`
3. SDK's `SecureGraphProxy` intercepts tool calls and sends to Data Plane gRPC
4. Data Plane enforces policies (ALLOW/BLOCK) and writes telemetry to hitlogs
5. Telemetry stored in `~/var/hitlogs/enforcement.hitlog` (NDJSON format)

**Current Gap:**
- ✅ Rich telemetry already captured in hitlogs (session_id, agent_id, rules evaluated, decisions)
- ❌ Console UI shows mock data - not connected to real hitlogs
- ❌ No API to query hitlogs from UI

## Implementation Strategy

**Approach: Query Hitlogs Directly (No duplicate storage)**
- Leverage existing `HitlogQuery` API in Data Plane
- Expose via gRPC → Management Plane HTTP → Console UI
- Map `EnforcementSession` to UI's `AgentRun` interface
- **Note:** Database integration deferred to future enhancement - hitlog files are already persistent via Docker volumes

---

## Phase 1: Data Plane - Expose Hitlog Query via gRPC (3-4 hours)

### 1.1 Add gRPC Service Definition
**File:** `proto/rule_installation.proto`

Add new RPC methods:
```protobuf
service DataPlane {
  // Existing methods...
  rpc Enforce(EnforceRequest) returns (EnforceResponse);

  // NEW: Telemetry query methods
  rpc QueryTelemetry(QueryTelemetryRequest) returns (QueryTelemetryResponse);
  rpc GetSession(GetSessionRequest) returns (GetSessionResponse);
}

message QueryTelemetryRequest {
  optional string agent_id = 1;
  optional string tenant_id = 2;
  optional int32 decision = 3;  // 0=BLOCK, 1=ALLOW, -1=all
  optional string layer = 4;    // L0-L6
  optional int64 start_time_ms = 5;
  optional int64 end_time_ms = 6;
  int32 limit = 7;              // default 50, max 500
  int32 offset = 8;             // for pagination
}

message QueryTelemetryResponse {
  repeated EnforcementSessionSummary sessions = 1;
  int32 total_count = 2;
}

message EnforcementSessionSummary {
  string session_id = 1;
  string agent_id = 2;
  string tenant_id = 3;
  string layer = 4;
  int64 timestamp_ms = 5;
  int32 final_decision = 6;
  int32 rules_evaluated_count = 7;
  int64 duration_us = 8;
  string intent_summary = 9;  // tool_name or action
}

message GetSessionRequest {
  string session_id = 1;
}

message GetSessionResponse {
  string session_json = 1;  // Full EnforcementSession as JSON
}
```

### 1.2 Implement gRPC Handler
**File:** `tupl_data_plane/tupl_dp/bridge/src/grpc_server.rs`

Add methods to `DataPlaneService`:
```rust
async fn query_telemetry(
    &self,
    request: Request<QueryTelemetryRequest>,
) -> Result<Response<QueryTelemetryResponse>, Status> {
    let req = request.into_inner();

    // Build filter from request
    let filter = QueryFilter {
        agent_id: req.agent_id,
        tenant_id: req.tenant_id,
        decision: if req.decision >= 0 { Some(req.decision as u8) } else { None },
        layer: req.layer,
        start_time_ms: req.start_time_ms,
        end_time_ms: req.end_time_ms,
        limit: Some(req.limit.min(500) as usize),
        offset: Some(req.offset as usize),
    };

    // Query hitlogs using existing HitlogQuery
    let sessions = self.hitlog_query.query(filter)
        .map_err(|e| Status::internal(format!("Query failed: {}", e)))?;

    // Convert to summary format
    let summaries = sessions.iter().map(|s| EnforcementSessionSummary {
        session_id: s.session_id.to_string(),
        agent_id: s.agent_id.clone(),
        tenant_id: s.tenant_id.clone(),
        layer: s.layer.clone(),
        timestamp_ms: s.timestamp_ms,
        final_decision: s.final_decision as i32,
        rules_evaluated_count: s.rules_evaluated.len() as i32,
        duration_us: s.duration_us,
        intent_summary: extract_intent_summary(&s.intent_json),
    }).collect();

    Ok(Response::new(QueryTelemetryResponse {
        sessions: summaries,
        total_count: sessions.len() as i32,
    }))
}

async fn get_session(
    &self,
    request: Request<GetSessionRequest>,
) -> Result<Response<GetSessionResponse>, Status> {
    let req = request.into_inner();

    // Query specific session
    let session = self.hitlog_query.by_session(&req.session_id)
        .map_err(|e| Status::not_found(format!("Session not found: {}", e)))?;

    // Serialize to JSON
    let session_json = serde_json::to_string(&session)
        .map_err(|e| Status::internal(format!("Serialization failed: {}", e)))?;

    Ok(Response::new(GetSessionResponse {
        session_json,
    }))
}
```

### 1.3 Update gRPC Server Setup
**File:** `tupl_data_plane/tupl_dp/bridge/src/grpc_server.rs`

Add `HitlogQuery` to server struct:
```rust
pub struct DataPlaneServiceImpl {
    enforcement_engine: Arc<EnforcementEngine>,
    hitlog_query: Arc<HitlogQuery>,  // NEW
}

// In server startup
let hitlog_dir = std::env::var("HITLOG_DIR").unwrap_or_else(|_| {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    format!("{}/var/hitlogs", home)
});

let hitlog_query = Arc::new(HitlogQuery::new(&hitlog_dir));

let service = DataPlaneServiceImpl {
    enforcement_engine,
    hitlog_query,
};
```

### 1.4 Generate Python gRPC Client Stubs
**Commands:**
```bash
cd /Users/sid/Projects/mgmt-plane
python -m grpc_tools.protoc \
  -I./proto \
  --python_out=./tupl_sdk/python/tupl/generated \
  --grpc_python_out=./tupl_sdk/python/tupl/generated \
  ./proto/rule_installation.proto
```

---

## Phase 2: Management Plane - HTTP Telemetry API (2-3 hours)

### 2.1 Create Telemetry Query Endpoint
**File:** `management-plane/app/endpoints/telemetry.py`

Add new endpoint (currently only has POST for ingestion):
```python
from typing import Optional
from fastapi import Query, HTTPException, status
from app.data_plane_client import DataPlaneClient
import json
import os

# Initialize Data Plane client
data_plane_client = DataPlaneClient(
    url=os.getenv("DATA_PLANE_GRPC_URL", "localhost:50051")
)

@router.get("/sessions", response_model=TelemetrySessionsResponse)
async def get_sessions(
    agent_id: Optional[str] = None,
    tenant_id: Optional[str] = None,
    decision: Optional[int] = None,  # 0=BLOCK, 1=ALLOW
    layer: Optional[str] = None,
    start_time: Optional[int] = None,  # unix timestamp ms
    end_time: Optional[int] = None,
    limit: int = Query(50, le=500),
    offset: int = Query(0, ge=0),
) -> TelemetrySessionsResponse:
    """
    Query enforcement telemetry sessions from Data Plane hitlogs.

    Returns paginated list of agent runs with enforcement decisions.
    """
    try:
        # Call Data Plane gRPC
        response = await data_plane_client.query_telemetry(
            agent_id=agent_id,
            tenant_id=tenant_id,
            decision=decision,
            layer=layer,
            start_time_ms=start_time,
            end_time_ms=end_time,
            limit=limit,
            offset=offset,
        )

        return TelemetrySessionsResponse(
            sessions=[
                SessionSummary(
                    sessionId=s.session_id,
                    agentId=s.agent_id,
                    tenantId=s.tenant_id,
                    layer=s.layer,
                    timestamp=s.timestamp_ms,
                    decision=s.final_decision,
                    rulesEvaluated=s.rules_evaluated_count,
                    durationUs=s.duration_us,
                    intentSummary=s.intent_summary,
                )
                for s in response.sessions
            ],
            totalCount=response.total_count,
        )
    except Exception as e:
        logger.error(f"Telemetry query failed: {e}")
        raise HTTPException(status_code=500, detail=str(e))

@router.get("/sessions/{session_id}", response_model=SessionDetail)
async def get_session_detail(session_id: str) -> SessionDetail:
    """Get full details for a specific enforcement session."""
    try:
        response = await data_plane_client.get_session(session_id)
        session_data = json.loads(response.session_json)

        return SessionDetail(
            sessionId=session_data["session_id"],
            agentId=session_data["agent_id"],
            tenantId=session_data["tenant_id"],
            layer=session_data["layer"],
            timestamp=session_data["timestamp_ms"],
            decision=session_data["final_decision"],
            intentEvent=session_data["intent_json"],
            rulesEvaluated=session_data["rules_evaluated"],
            events=session_data["events"],
            performance=session_data["performance"],
        )
    except Exception as e:
        logger.error(f"Session detail fetch failed: {e}")
        raise HTTPException(status_code=404, detail="Session not found")
```

### 2.2 Create Response Models
**File:** `management-plane/app/models.py`

Add telemetry response models:
```python
class SessionSummary(BaseModel):
    sessionId: str
    agentId: str
    tenantId: str
    layer: str
    timestamp: int  # unix timestamp ms
    decision: int   # 0=BLOCK, 1=ALLOW
    rulesEvaluated: int
    durationUs: int
    intentSummary: str  # e.g., "postgres_query" or "write"

class TelemetrySessionsResponse(BaseModel):
    sessions: list[SessionSummary]
    totalCount: int

class SessionDetail(BaseModel):
    sessionId: str
    agentId: str
    tenantId: str
    layer: str
    timestamp: int
    decision: int
    intentEvent: str  # JSON string of IntentEvent
    rulesEvaluated: list[dict]  # Full rule evaluation results
    events: list[dict]  # Timeline events
    performance: dict  # Performance metrics
```

### 2.3 Add Data Plane Client Method
**File:** `management-plane/app/data_plane_client.py`

Add telemetry query methods:
```python
from typing import Optional

async def query_telemetry(
    self,
    agent_id: Optional[str] = None,
    tenant_id: Optional[str] = None,
    decision: Optional[int] = None,
    layer: Optional[str] = None,
    start_time_ms: Optional[int] = None,
    end_time_ms: Optional[int] = None,
    limit: int = 50,
    offset: int = 0,
) -> QueryTelemetryResponse:
    """Query telemetry sessions from Data Plane hitlogs."""
    request = QueryTelemetryRequest(
        agent_id=agent_id,
        tenant_id=tenant_id,
        decision=decision if decision is not None else -1,
        layer=layer,
        start_time_ms=start_time_ms,
        end_time_ms=end_time_ms,
        limit=limit,
        offset=offset,
    )

    response = await self.stub.QueryTelemetry(request)
    return response

async def get_session(self, session_id: str) -> GetSessionResponse:
    """Get full details for a specific session."""
    request = GetSessionRequest(session_id=session_id)
    response = await self.stub.GetSession(request)
    return response
```

---

## Phase 3: Console UI - Connect to Real Telemetry (3-4 hours)

### 3.1 Create Telemetry API Client
**File:** `mcp-ui/src/lib/telemetry-api.ts`

Create new file:
```typescript
const MANAGEMENT_PLANE_URL = process.env.NEXT_PUBLIC_MANAGEMENT_PLANE_URL || 'http://localhost:8000';

export interface AgentRunSummary {
  sessionId: string;
  agentId: string;
  tenantId: string;
  layer: string;
  timestamp: number;
  decision: 0 | 1;  // BLOCK | ALLOW
  rulesEvaluated: number;
  durationUs: number;
  intentSummary: string;
}

export interface TelemetrySessionsResponse {
  sessions: AgentRunSummary[];
  totalCount: number;
}

export interface SessionDetail {
  sessionId: string;
  agentId: string;
  tenantId: string;
  layer: string;
  timestamp: number;
  decision: 0 | 1;
  intentEvent: string;
  rulesEvaluated: Array<{
    rule_id: string;
    rule_family: string;
    decision: number;
    slice_similarities: number[];
  }>;
  events: Array<{
    type: string;
    timestamp_us: number;
    [key: string]: any;
  }>;
  performance: {
    encoding_duration_us: number;
    rule_query_duration_us: number;
    evaluation_duration_us: number;
  };
}

export async function fetchAgentRuns(params: {
  agentId?: string;
  tenantId?: string;
  decision?: 0 | 1;
  layer?: string;
  startTime?: number;
  endTime?: number;
  limit?: number;
  offset?: number;
}): Promise<TelemetrySessionsResponse> {
  const queryParams = new URLSearchParams();
  if (params.agentId) queryParams.set('agent_id', params.agentId);
  if (params.tenantId) queryParams.set('tenant_id', params.tenantId);
  if (params.decision !== undefined) queryParams.set('decision', params.decision.toString());
  if (params.layer) queryParams.set('layer', params.layer);
  if (params.startTime) queryParams.set('start_time', params.startTime.toString());
  if (params.endTime) queryParams.set('end_time', params.endTime.toString());
  if (params.limit) queryParams.set('limit', params.limit.toString());
  if (params.offset) queryParams.set('offset', params.offset.toString());

  const response = await fetch(
    `${MANAGEMENT_PLANE_URL}/api/v1/telemetry/sessions?${queryParams}`
  );

  if (!response.ok) {
    throw new Error(`Telemetry query failed: ${response.statusText}`);
  }

  return response.json();
}

export async function fetchSessionDetail(sessionId: string): Promise<SessionDetail> {
  const response = await fetch(
    `${MANAGEMENT_PLANE_URL}/api/v1/telemetry/sessions/${sessionId}`
  );

  if (!response.ok) {
    throw new Error(`Session detail fetch failed: ${response.statusText}`);
  }

  return response.json();
}
```

### 3.2 Update AgentsIndexPage to Use Real Data
**File:** `mcp-ui/src/pages/AgentsIndexPage.tsx`

Replace mock data with real API calls:
```typescript
import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { fetchAgentRuns } from '@/lib/telemetry-api';

export default function AgentsIndexPage() {
  const [filters, setFilters] = useState({
    agentId: '',
    decision: undefined as 0 | 1 | undefined,
    limit: 50,
  });

  // Replace mock data with real API call
  const { data, isLoading, error } = useQuery({
    queryKey: ['agent-runs', filters],
    queryFn: () => fetchAgentRuns(filters),
    refetchInterval: 5000,  // Poll every 5 seconds for updates
  });

  if (isLoading) return <LoadingSpinner />;
  if (error) return <ErrorMessage error={error} />;

  // Map API response to UI format
  const agentRuns = data?.sessions.map(session => ({
    id: session.sessionId,
    agentName: session.agentId,
    status: session.decision === 1 ? 'success' : 'failed' as const,
    started: new Date(session.timestamp),
    duration: session.durationUs / 1000,  // Convert to ms
    policiesApplied: session.rulesEvaluated,
    trace: [`Enforcement decision: ${session.decision === 1 ? 'ALLOW' : 'BLOCK'}`],
  })) ?? [];

  return (
    <div className="p-6">
      <h1 className="text-2xl font-bold mb-6">Agent Runs</h1>

      {/* Add filters */}
      <div className="mb-4 flex gap-4">
        <input
          className="px-3 py-2 border rounded"
          placeholder="Filter by agent ID"
          value={filters.agentId}
          onChange={(e) => setFilters({ ...filters, agentId: e.target.value })}
        />
        <select
          className="px-3 py-2 border rounded"
          value={filters.decision ?? ''}
          onChange={(e) => setFilters({
            ...filters,
            decision: e.target.value ? parseInt(e.target.value) as 0 | 1 : undefined
          })}
        >
          <option value="">All decisions</option>
          <option value="1">Allowed</option>
          <option value="0">Blocked</option>
        </select>
      </div>

      <AgentRunsTable runs={agentRuns} />
    </div>
  );
}
```

### 3.3 Implement AgentDetailPage
**File:** `mcp-ui/src/pages/AgentDetailPage.tsx`

Create new detail page:
```typescript
import { useParams } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { fetchSessionDetail } from '@/lib/telemetry-api';

export default function AgentDetailPage() {
  const { sessionId } = useParams<{ sessionId: string }>();

  const { data: session, isLoading, error } = useQuery({
    queryKey: ['session-detail', sessionId],
    queryFn: () => fetchSessionDetail(sessionId!),
    enabled: !!sessionId,
  });

  if (isLoading) return <LoadingSpinner />;
  if (error) return <ErrorMessage error={error} />;
  if (!session) return <div>Session not found</div>;

  return (
    <div className="p-6">
      <h1 className="text-2xl font-bold mb-6">Agent Run: {session.sessionId}</h1>

      <div className="grid grid-cols-2 gap-4 mb-6">
        <InfoCard label="Agent ID" value={session.agentId} />
        <InfoCard label="Tenant ID" value={session.tenantId} />
        <InfoCard label="Layer" value={session.layer} />
        <InfoCard
          label="Decision"
          value={session.decision === 1 ? 'ALLOW' : 'BLOCK'}
          className={session.decision === 1 ? 'text-green-600' : 'text-red-600'}
        />
        <InfoCard
          label="Duration"
          value={`${(session.performance.evaluation_duration_us / 1000).toFixed(2)}ms`}
        />
      </div>

      {/* Intent Event */}
      <section className="mb-6">
        <h2 className="text-xl font-semibold mb-3">Intent Event</h2>
        <pre className="bg-gray-100 p-4 rounded overflow-auto">
          {JSON.stringify(JSON.parse(session.intentEvent), null, 2)}
        </pre>
      </section>

      {/* Rules Evaluated */}
      <section className="mb-6">
        <h2 className="text-xl font-semibold mb-3">Rules Evaluated ({session.rulesEvaluated.length})</h2>
        <table className="w-full border">
          <thead>
            <tr className="bg-gray-100">
              <th className="p-2 text-left">Rule ID</th>
              <th className="p-2 text-left">Family</th>
              <th className="p-2 text-left">Decision</th>
              <th className="p-2 text-left">Similarities</th>
            </tr>
          </thead>
          <tbody>
            {session.rulesEvaluated.map((rule, i) => (
              <tr key={i} className="border-t">
                <td className="p-2">{rule.rule_id}</td>
                <td className="p-2">{rule.rule_family}</td>
                <td className="p-2">{rule.decision === 1 ? 'ALLOW' : 'BLOCK'}</td>
                <td className="p-2 font-mono text-sm">
                  {rule.slice_similarities.map(s => s.toFixed(3)).join(', ')}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>

      {/* Execution Timeline */}
      <section className="mb-6">
        <h2 className="text-xl font-semibold mb-3">Execution Timeline</h2>
        <ul className="space-y-2">
          {session.events.map((event, i) => (
            <li key={i} className="flex gap-4">
              <span className="text-gray-500 font-mono text-sm">
                {new Date(event.timestamp_us / 1000).toLocaleTimeString()}
              </span>
              <span>{event.type}</span>
            </li>
          ))}
        </ul>
      </section>
    </div>
  );
}
```

---

## Phase 4: Docker Compose Integration (1 hour)

### 4.1 Update docker-compose.yml
**File:** `deployment/security-stack/docker-compose.yml`

Add hitlog volume and telemetry configuration:
```yaml
version: '3.8'

services:
  ai-security-stack:
    build:
      context: ../..
      dockerfile: deployment/security-stack/Dockerfile
    ports:
      - "8000:8000"  # Management Plane
      - "8001:8001"  # Control Plane (UI)
      - "50051:50051" # Data Plane gRPC
    environment:
      # Existing
      - SEMANTIC_SANDBOX_LIB_PATH=/app/libs/libsemantic_sandbox.so
      - GOOGLE_API_KEY=${GOOGLE_API_KEY}
      - SUPABASE_URL=${SUPABASE_URL}
      - SUPABASE_JWT_SECRET=${SUPABASE_JWT_SECRET}
      - DATA_PLANE_GRPC_URL=${DATA_PLANE_GRPC_URL:-localhost:50051}
      - MANAGEMENT_PLANE_URL=${MANAGEMENT_PLANE_URL:-http://localhost:8000}

      # NEW: Telemetry configuration
      - HITLOG_DIR=/var/hitlogs
      - TELEMETRY_ENABLED=${TELEMETRY_ENABLED:-true}
      - TELEMETRY_SAMPLE_RATE=${TELEMETRY_SAMPLE_RATE:-1.0}
      - HITLOG_ROTATION_POLICY=${HITLOG_ROTATION_POLICY:-BySize}
      - HITLOG_ROTATION_SIZE_MB=${HITLOG_ROTATION_SIZE_MB:-100}
      - HITLOG_MAX_ROTATED_FILES=${HITLOG_MAX_ROTATED_FILES:-10}

    volumes:
      - security-data:/app/data
      - security-models:/root/.cache/huggingface
      - hitlogs:/var/hitlogs  # NEW: Persist telemetry hitlogs
    restart: unless-stopped

volumes:
  security-data:
    driver: local
  security-models:
    driver: local
  hitlogs:  # NEW: Telemetry storage
    driver: local
```

### 4.2 Update README
**File:** `deployment/security-stack/README.md`

Add telemetry section:
```markdown
## Telemetry & Observability

The security stack automatically captures enforcement telemetry in hitlogs.

### Viewing Agent Runs
1. Access Console UI at http://localhost:8001
2. Navigate to "Agents" page
3. View all enforcement sessions with filtering by agent, decision, layer

### Telemetry Storage
- Location: `/var/hitlogs/enforcement.hitlog` (inside container)
- Format: NDJSON (newline-delimited JSON)
- Rotation: Automatic based on size (default 100MB)
- Persistence: Stored in Docker volume `hitlogs`

### Configuration
Environment variables:
- `TELEMETRY_ENABLED` - Enable/disable telemetry (default: true)
- `TELEMETRY_SAMPLE_RATE` - Sample rate 0.0-1.0 (default: 1.0 = 100%)
- `HITLOG_ROTATION_POLICY` - BySize, ByTime, Daily, Hourly (default: BySize)
- `HITLOG_ROTATION_SIZE_MB` - Rotation threshold (default: 100)
- `HITLOG_MAX_ROTATED_FILES` - Max rotated files to keep (default: 10)
```

### 4.3 Create .env.example
**File:** `deployment/security-stack/.env.example`

```bash
# API Keys
GOOGLE_API_KEY=your_google_api_key_here

# Supabase (for authentication)
SUPABASE_URL=https://your-project.supabase.co
SUPABASE_JWT_SECRET=your_jwt_secret_here

# Service URLs (usually don't need to change)
DATA_PLANE_GRPC_URL=localhost:50051
MANAGEMENT_PLANE_URL=http://localhost:8000

# Telemetry Configuration
TELEMETRY_ENABLED=true
TELEMETRY_SAMPLE_RATE=1.0
HITLOG_ROTATION_POLICY=BySize
HITLOG_ROTATION_SIZE_MB=100
HITLOG_MAX_ROTATED_FILES=10
```

---

## Success Criteria

✅ User wraps agent with `enforcement_agent()` in their IDE
✅ Agent makes tool calls that get enforced by Data Plane
✅ Telemetry automatically written to hitlogs in real-time
✅ Console UI "Agents" page shows all enforcement sessions
✅ User can filter by agent ID, decision (ALLOW/BLOCK), layer
✅ User can click session to see full details (rules evaluated, timeline, performance)
✅ Telemetry persists across container restarts (Docker volume)
✅ No duplicate storage - direct query from hitlogs

---

## Estimated Total Time: 9-12 hours

**Phase 1 (Data Plane):** 3-4 hours
**Phase 2 (Management Plane):** 2-3 hours
**Phase 3 (Console UI):** 3-4 hours
**Phase 4 (Docker Compose):** 1 hour

---

## Key Design Decisions

### Why query hitlogs directly?
- All data already exists in rich format
- No duplicate storage overhead
- Real-time visibility (no sync delay)
- Hitlog format is stable and well-documented
- Persistent via Docker volumes

### Why not stream to database?
- Adds complexity (schema, migrations, sync worker)
- Duplicate storage (hitlogs + database)
- Hitlog query performance is sufficient for MVP
- **Can add later as enhancement if needed** (e.g., for multi-Data Plane deployments or advanced analytics)

### Session ID vs Agent Run ID
- Hitlog uses `session_id` (per enforcement call)
- One agent invocation = multiple sessions (one per tool call)
- Future enhancement: group sessions by agent run context

---

## Files to Modify/Create

### New Files (~6)
- `mcp-ui/src/lib/telemetry-api.ts` - API client for telemetry
- `mcp-ui/src/pages/AgentDetailPage.tsx` - Session detail view
- `deployment/security-stack/.env.example` - Environment variables template

### Modified Files (~8)
- `proto/rule_installation.proto` - Add QueryTelemetry RPC
- `tupl_data_plane/tupl_dp/bridge/src/grpc_server.rs` - Implement query handler
- `management-plane/app/endpoints/telemetry.py` - Add query endpoints
- `management-plane/app/models.py` - Add response models
- `management-plane/app/data_plane_client.py` - Add query methods
- `mcp-ui/src/pages/AgentsIndexPage.tsx` - Replace mock data
- `deployment/security-stack/docker-compose.yml` - Add hitlog volume
- `deployment/security-stack/README.md` - Document telemetry

---

## Future Enhancements (Post-MVP)

1. **Database Integration**
   - Stream hitlogs to PostgreSQL/SQLite for advanced querying
   - Enable time-series analytics and aggregations
   - Support for larger datasets and historical analysis

2. **Agent Run Grouping**
   - Add `agent_run_id` to IntentEvent schema
   - Group multiple enforcement sessions by agent invocation
   - Show hierarchical view of tool calls within a run

3. **Real-time Dashboard**
   - WebSocket streaming for live updates
   - Charts and metrics (block rate over time, latency distribution)
   - Policy effectiveness analytics

4. **Advanced Filtering**
   - Full-text search on intent events
   - Multi-tenant filtering
   - Custom date range pickers
   - Export to CSV/JSON

5. **Alerting**
   - Configure alerts for high block rates
   - Notification on policy violations
   - Anomaly detection

---

## Notes

- Hitlog files are already persistent via Docker volumes - no separate database needed for MVP
- Data Plane's `HitlogQuery` API already has all necessary query capabilities
- UI can poll every 5 seconds for near-real-time updates
- For production deployments with high traffic, consider database integration for better query performance
