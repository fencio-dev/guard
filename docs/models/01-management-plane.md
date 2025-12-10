# Management Plane Mental Model

**Version:** 0.9.0
**Date:** 2025-11-22
**Component**: management-plane/
**Technology**: Python, FastAPI, sentence-transformers

---

## Purpose

The Management Plane is the central orchestration service for semantic security enforcement. It encodes intents and boundaries into 128-dimensional vectors, filters applicable policies, coordinates with the Data Plane for enforcement decisions, and collects telemetry.

**Core Responsibilities:**
1. Encode IntentEvents and DesignBoundaries to 128-dim vectors
2. Filter applicable boundaries using attribute-based rules
3. Coordinate enforcement via Data Plane gRPC
4. Aggregate enforcement decisions (deny-first logic)
5. Collect and serve telemetry data
6. Cache encoded vectors for performance

---

## Architecture

### Directory Structure
```
management-plane/
├── app/
│   ├── main.py              # FastAPI application entry
│   ├── types.py             # Pydantic data models
│   ├── encoding.py          # Vector encoding pipeline
│   ├── applicability.py     # Boundary filtering logic
│   ├── ffi.py               # FFI bridge to Rust sandbox
│   ├── grpc_client.py       # Data Plane gRPC client
│   ├── endpoints/
│   │   ├── intents.py       # /intents/* endpoints
│   │   ├── boundaries.py    # /boundaries/* endpoints
│   │   ├── encoding.py      # /encode/* endpoints
│   │   └── telemetry.py     # /telemetry/* endpoints
│   └── tests/
│       ├── test_encoding.py
│       ├── test_applicability.py
│       └── test_ffi.py
├── pyproject.toml           # Dependencies
└── README.md
```

### Key Dependencies
- **FastAPI** - Web framework (async ASGI)
- **Pydantic** - Data validation and serialization
- **sentence-transformers** - Text embedding (all-MiniLM-L6-v2)
- **NumPy** - Vector operations
- **grpcio** - gRPC client for Data Plane
- **ctypes** - FFI to Semantic Sandbox

---

## Data Models

### IntentEvent (v1.3)
Structured record of an LLM/tool action.

**File**: [app/types.py:20-60](../../management-plane/app/types.py)

```python
class IntentEvent(BaseModel):
    id: str
    schemaVersion: Literal["v1.3"]
    tenantId: str
    timestamp: str

    actor: Actor  # {id, type: user|service|llm|agent}
    action: Literal["read", "write", "delete", "export", "execute", "update"]
    resource: Resource  # {type, name?, location?}
    data: Data  # {sensitivity[], pii?, volume?}
    risk: Risk  # {authn}

    tool_gateway: Optional[ToolGateway]  # v1.3 addition
    context: Optional[Dict[str, Any]]
```

**Key Fields**:
- `action` - What operation is being performed
- `resource.type` - What is being accessed (database, file, api)
- `data.sensitivity` - Classification level (internal, public)
- `risk.authn` - Authentication requirement

### DesignBoundary
Security policy definition.

**File**: [app/types.py:100-150](../../management-plane/app/types.py)

```python
class DesignBoundary(BaseModel):
    id: str
    name: str
    effect: Literal["allow", "deny"]

    scope: Scope  # {tenantId, domains?}
    rules: Rules  # {thresholds, decision, weights?}
    constraints: Constraints  # v1.2 attribute matching
```

**Threshold Modes**:
- `min` - All slices must pass individual thresholds
- `weighted-avg` - Weighted average must exceed global threshold

### ComparisonResult
Enforcement decision with evidence.

**File**: [app/types.py:200-220](../../management-plane/app/types.py)

```python
class ComparisonResult(BaseModel):
    decision: Literal[0, 1]  # 0=BLOCK, 1=ALLOW
    similarities: List[float]  # [action, resource, data, risk]
    evidence: List[BoundaryEvidence]  # Per-boundary results
    metadata: Dict[str, Any]
```

---

## Encoding Pipeline

### Overview
Deterministic encoding of intents/boundaries to 128-dim vectors.

**File**: [app/encoding.py](../../management-plane/app/encoding.py)

**Pipeline Steps**:
```
IntentEvent/DesignBoundary
    ↓
1. Canonicalize to 4 slot strings
    ↓
2. Embed each with all-MiniLM-L6-v2 (384-dim)
    ↓
3. Sparse random projection → 4×32-dim
    ↓
4. L2 normalize each slice
    ↓
5. Concatenate → 128-dim vector
```

### Slot Construction

**Action Slot**:
```python
def build_action_slot(event: IntentEvent) -> str:
    # Format: "action={action} actor={actor.type}"
    return f"action={event.action} actor={event.actor.type}"
```

**Resource Slot**:
```python
def build_resource_slot(event: IntentEvent) -> str:
    parts = [f"type={event.resource.type}"]
    if event.resource.name:
        parts.append(f"name={event.resource.name}")
    if event.resource.location:
        parts.append(f"location={event.resource.location}")
    return " ".join(parts)
```

**Data Slot**:
```python
def build_data_slot(event: IntentEvent) -> str:
    sensitivity = ",".join(sorted(event.data.sensitivity))
    parts = [f"sensitivity={sensitivity}"]
    if event.data.pii is not None:
        parts.append(f"pii={event.data.pii}")
    if event.data.volume:
        parts.append(f"volume={event.data.volume}")
    return " ".join(parts)
```

**Risk Slot**:
```python
def build_risk_slot(event: IntentEvent) -> str:
    return f"authn={event.risk.authn}"
```

### Determinism Principles

1. **Fixed Random Seeds**: Projection matrices use seeds [42, 43, 44, 45]
2. **Sorted Vocabularies**: Lists sorted before joining
3. **Explicit dtypes**: `.astype(np.float32)` everywhere
4. **Stable Formatting**: Consistent string templates
5. **No Timestamps**: Exclude temporal fields from encoding

### Caching Strategy

```python
from functools import lru_cache

@lru_cache(maxsize=10000)
def encode_slot(text: str, seed: int) -> np.ndarray:
    # Embed with sentence-transformers
    embedding = model.encode(text)  # 384-dim

    # Project to 32-dim
    projection = sparse_random_projection(embedding, seed)

    # Normalize
    return normalize(projection)
```

**Cache Hit Rate**: ~85% for boundaries (reused frequently)

---

## Applicability Filtering

### Purpose
Filter boundaries before expensive encoding/comparison.

**File**: [app/applicability.py](../../management-plane/app/applicability.py)

### Rule Families

**Core Rules** (must match):
- Action match: `event.action in boundary.constraints.action.actions`
- Actor type match: `event.actor.type in boundary.constraints.action.actor_types`
- Resource type match: `event.resource.type in boundary.constraints.resource.types`

**Soft Rules** (voting-based):
- Resource location match
- PII presence match
- Volume match
- Domain scope match
- Resource name match

### Scoring Algorithm

```python
def calculate_applicability_score(
    event: IntentEvent,
    boundary: DesignBoundary
) -> float:
    # Core rules (fail-fast)
    if not all_core_rules_match(event, boundary):
        return 0.0

    # Soft rules (vote)
    soft_score = 0.0
    total_weight = 0.0

    for rule in SOFT_RULES:
        if rule.applies(event, boundary):
            soft_score += rule.weight * (1.0 if rule.matches else 0.0)
            total_weight += rule.weight

    return soft_score / total_weight if total_weight > 0 else 1.0
```

### Configuration

```python
# Environment variables
APPLICABILITY_MODE = "soft"  # or "strict"
APPLICABILITY_MIN_SCORE = 0.5  # threshold for soft mode
```

### Security Defaults

- **No active boundaries** → ALLOW (bootstrap mode)
- **No applicable boundaries** → BLOCK (fail-closed)
- **Applicability score below threshold** → Skip boundary

---

## Enforcement Coordination

### Flow

```
POST /api/v1/intents/compare
    ↓
1. Validate IntentEvent
    ↓
2. Encode intent → 128-dim
    ↓
3. Filter applicable boundaries
    ↓
4. For each applicable boundary:
    ├─ Encode boundary → 128-dim (cached)
    ├─ Call Data Plane gRPC Enforce
    └─ Receive {decision, similarities}
    ↓
5. Aggregate decisions (deny-first)
    ↓
6. Return ComparisonResult
```

### gRPC Client

**File**: [app/grpc_client.py](../../management-plane/app/grpc_client.py)

```python
class DataPlaneClient:
    def __init__(self, address: str = "localhost:50051"):
        self.channel = grpc.insecure_channel(address)
        self.stub = EnforcementServiceStub(self.channel)

    async def enforce(
        self,
        agent_id: str,
        intent_vector: np.ndarray
    ) -> EnforcementResult:
        request = EnforceRequest(
            agent_id=agent_id,
            intent_vector=intent_vector.tolist()
        )
        response = await self.stub.Enforce(request)
        return response
```

### Aggregation Logic

**Deny-First Algorithm**:
```python
def aggregate_decisions(
    evidence: List[BoundaryEvidence]
) -> Literal[0, 1]:
    # Step 1: Any DENY boundary that matches → immediate BLOCK
    for ev in evidence:
        if ev.effect == "deny" and ev.decision == 1:
            return 0  # BLOCK

    # Step 2: All mandatory ALLOW must pass
    for ev in evidence:
        if ev.effect == "allow" and ev.mandatory and ev.decision == 0:
            return 0  # BLOCK

    # Step 3: Default ALLOW
    return 1
```

---

## API Endpoints

### Intent Enforcement

**POST /api/v1/intents/compare**

Request:
```json
{
  "id": "intent_123",
  "action": "read",
  "resource": {"type": "database"},
  "data": {"sensitivity": ["internal"]},
  "risk": {"authn": "required"}
}
```

Response:
```json
{
  "decision": 1,
  "similarities": [0.92, 0.88, 0.95, 0.90],
  "evidence": [
    {
      "boundary_id": "bd_001",
      "name": "Allow Internal DB Reads",
      "effect": "allow",
      "decision": 1,
      "similarities": [0.92, 0.88, 0.95, 0.90]
    }
  ]
}
```

### Encoding

**POST /api/v1/encode/intent**

Request: `IntentEvent`
Response: `{"vector": [0.1, 0.2, ..., 0.9]}`  # 128 floats

**POST /api/v1/encode/rule/{family}**

Request: Rule family name + rule data
Response: Encoded anchor vectors

### Boundaries

**GET /api/v1/boundaries**

List all boundaries (in-memory store in v0.9.0)

**POST /api/v1/boundaries**

Create new boundary

### Telemetry

**GET /api/v1/telemetry/sessions**

Query parameters:
- `agent_id` - Filter by agent
- `decision` - Filter by ALLOW/BLOCK
- `start_time`, `end_time` - Time range
- `limit`, `offset` - Pagination

Response:
```json
{
  "sessions": [...],
  "total": 42
}
```

---

## FFI Bridge

### Purpose
Call Rust Semantic Sandbox for comparison.

**File**: [app/ffi.py](../../management-plane/app/ffi.py)

### C Structure Layout

```python
import ctypes

class VectorEnvelope(ctypes.Structure):
    _fields_ = [
        ("intent", ctypes.c_float * 128),
        ("boundary", ctypes.c_float * 128),
        ("thresholds", ctypes.c_float * 4),
        ("weights", ctypes.c_float * 4),
        ("mode", ctypes.c_uint8),  # 0=min, 1=weighted-avg
    ]

class ComparisonResult(ctypes.Structure):
    _fields_ = [
        ("decision", ctypes.c_uint8),  # 0=BLOCK, 1=ALLOW
        ("slice_similarities", ctypes.c_float * 4),
    ]
```

### Usage

```python
# Load Rust library
lib = ctypes.CDLL("../semantic-sandbox/target/release/libsemantic_sandbox.so")

# Call comparison
envelope = VectorEnvelope(
    intent=(ctypes.c_float * 128)(*intent_vector),
    boundary=(ctypes.c_float * 128)(*boundary_vector),
    thresholds=(ctypes.c_float * 4)(*[0.7, 0.7, 0.7, 0.7]),
    weights=(ctypes.c_float * 4)(*[1.0, 1.0, 1.0, 1.0]),
    mode=0,  # min mode
)

result = lib.compare_vectors(ctypes.byref(envelope))
```

### Safety Considerations

- **Input Validation**: Check vectors for NaN/Inf before FFI call
- **Memory Safety**: Rust never retains Python pointers
- **Error Handling**: Rust returns error codes, Python raises exceptions
- **Type Safety**: Exact struct layout match required

---

## Performance Characteristics

### Latency Targets

| Operation | Target | Measurement |
|-----------|--------|-------------|
| Intent encoding | <10ms | Single event |
| Boundary encoding | <10ms | With cache hit: <1ms |
| Applicability filter | <1ms | Per boundary |
| FFI call | <1ms | Single comparison |
| Full enforcement (10 boundaries) | <50ms | P50 |
| Full enforcement (100 boundaries) | <100ms | P50 |

### Caching

```python
# LRU cache for embeddings
@lru_cache(maxsize=10000)
def encode_text(text: str) -> np.ndarray:
    return model.encode(text)

# Boundary vector cache
boundary_cache: Dict[str, np.ndarray] = {}

def get_boundary_vector(boundary_id: str) -> np.ndarray:
    if boundary_id in boundary_cache:
        return boundary_cache[boundary_id]

    vector = encode_boundary(boundary)
    boundary_cache[boundary_id] = vector
    return vector
```

**Cache Hit Rates**:
- Embeddings: ~70% (common phrases reused)
- Boundaries: ~85% (policies change infrequently)

### Optimization Opportunities (v1.0+)

1. **Batch encoding** - Encode multiple intents in parallel
2. **Async gRPC** - Non-blocking Data Plane calls
3. **Boundary pre-encoding** - Encode all at startup
4. **Redis cache** - Shared cache across instances

---

## Testing

### Unit Tests

**test_encoding.py**:
```python
def test_encoding_determinism():
    event = create_test_intent()
    vector1 = encode_to_128d(event)
    vector2 = encode_to_128d(event)
    assert np.allclose(vector1, vector2)

def test_vector_dimensions():
    event = create_test_intent()
    vector = encode_to_128d(event)
    assert vector.shape == (128,)
    assert vector.dtype == np.float32
```

**test_applicability.py**:
```python
def test_core_rules_filter():
    event = IntentEvent(action="read", ...)
    boundary = DesignBoundary(constraints=...)

    score = calculate_applicability_score(event, boundary)
    assert score > 0.5

def test_no_applicable_boundaries_blocks():
    result = enforce_intent(event_with_no_matches)
    assert result.decision == 0  # BLOCK
```

### Integration Tests

**test_ffi.py**:
```python
def test_ffi_comparison():
    intent = np.random.rand(128).astype(np.float32)
    boundary = np.random.rand(128).astype(np.float32)

    result = compare_via_ffi(intent, boundary, thresholds=[0.7]*4)
    assert result.decision in [0, 1]
    assert len(result.slice_similarities) == 4
```

---

## Error Handling

### Common Errors

**ValidationError** (400):
```python
# Missing required fields
raise HTTPException(
    status_code=400,
    detail="IntentEvent validation failed: missing 'action' field"
)
```

**EncodingError** (500):
```python
# Embedding model failure
raise HTTPException(
    status_code=500,
    detail="Failed to encode intent: model not loaded"
)
```

**gRPC Timeout** (504):
```python
# Data Plane unavailable
raise HTTPException(
    status_code=504,
    detail="Data Plane timeout after 5s"
)
```

### Logging

```python
import logging

logger = logging.getLogger("management_plane")

# Log enforcement decisions
logger.info(
    "Enforcement decision",
    extra={
        "intent_id": event.id,
        "decision": result.decision,
        "boundaries_evaluated": len(result.evidence),
        "latency_ms": latency,
    }
)
```

---

## Configuration

### Environment Variables

```bash
# Server
UVICORN_HOST=0.0.0.0
UVICORN_PORT=8000

# Data Plane gRPC
DATA_PLANE_ADDRESS=localhost:50051
DATA_PLANE_TIMEOUT=5

# Embedding Model
EMBEDDING_MODEL=all-MiniLM-L6-v2
EMBEDDING_DEVICE=cpu  # or cuda

# Applicability
APPLICABILITY_MODE=soft
APPLICABILITY_MIN_SCORE=0.5

# Cache
EMBEDDING_CACHE_SIZE=10000
BOUNDARY_CACHE_SIZE=1000

# Auth
JWT_SECRET_KEY=your-secret-key
JWT_ALGORITHM=HS256
```

---

## Deployment

### Docker
```dockerfile
FROM python:3.11-slim

WORKDIR /app
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY app/ ./app/
CMD ["uvicorn", "app.main:app", "--host", "0.0.0.0", "--port", "8000"]
```

### Production Considerations

1. **CPU-Only PyTorch**: Use `torch.cpu` for smaller images
2. **Model Warmup**: Load embedding model at startup
3. **Health Checks**: `/health` endpoint for orchestration
4. **Metrics**: Prometheus metrics for monitoring
5. **Logging**: Structured JSON logs

---

## Related Documentation

- [System Overview](./00-system-overview.md)
- [Data Plane](./02-data-plane.md)
- [Semantic Sandbox](./03-semantic-sandbox.md)
- [Python SDK](./07-python-sdk.md)

---

**Last Updated**: 2025-11-22
**Release**: v0.9.0
