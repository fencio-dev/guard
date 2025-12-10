# Semantic Security MVP Implementation Plan

**Version:** 1.0
**Date:** 2025-11-12
**Target:** Local development on laptop
**Timeline:** 4 weeks

---

## 1. System Overview

### Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Control Plane                            │
│                    (React + Vite + TypeScript)                   │
│                                                                   │
│  - Design Boundary Editor                                        │
│  - Decision Viewer                                               │
│  - Policy Management                                             │
└────────────────────────────┬────────────────────────────────────┘
                             │ HTTP/REST
                             │ (Publish boundaries, View telemetry)
                             ↓
┌─────────────────────────────────────────────────────────────────┐
│                       Management Plane                           │
│                           (Python)                               │
│                                                                   │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ FastAPI Endpoints                                        │   │
│  │  - POST /encode/intent                                   │   │
│  │  - GET  /boundaries/candidates                           │   │
│  │  - POST /compare                                         │   │
│  │  - POST /telemetry                                       │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                   │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ Encoding Pipeline                                        │   │
│  │  1. Canonicalize IntentEvent/DesignBoundary             │   │
│  │  2. Build 4 slot strings (action, resource, data, risk) │   │
│  │  3. sentence-transformers → 4 embeddings                │   │
│  │  4. Sparse random projection → 4x32 dims                │   │
│  │  5. Concat + L2 normalize → 128-dim vector              │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                   │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ Candidate Filter (Simple attribute matching)            │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                   │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ FFI Bridge to Semantic Sandbox (CDylib)                 │   │
│  └───────────────────────┬──────────────────────────────────┘   │
└──────────────────────────┼──────────────────────────────────────┘
                           │ FFI call per boundary comparison
                           │ (intent128, boundary128, thresholds)
                           ↓
┌─────────────────────────────────────────────────────────────────┐
│                      Semantic Sandbox                            │
│                        (Rust CDylib)                             │
│                                                                   │
│  Input:  intent[128], boundary[128], thresholds[4], weights[4]  │
│  Logic:  Slice comparison (4 x dot product)                     │
│  Output: {decision: 0|1, slice_similarities: [f32; 4]}          │
└─────────────────────────────────────────────────────────────────┘


┌─────────────────────────────────────────────────────────────────┐
│                          Client Code                             │
│                    (User's application with                      │
│                     LangGraph/LLM calls)                         │
│                                                                   │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ SDK (TypeScript or Python)                               │   │
│  │  - Wrap LLM/tool calls                                   │   │
│  │  - Capture IntentEvent                                   │   │
│  │  - POST to Management Plane                              │   │
│  └──────────────────────────────────────────────────────────┘   │
└────────────────────────────┬────────────────────────────────────┘
                             │ HTTP/REST
                             │ (Send IntentEvents)
                             ↓
                   Management Plane
```

### Data Flow

1. **SDK captures intent** → POST IntentEvent to Management Plane
2. **Management Plane encodes intent** → 128-dim vector
3. **Candidate filter** → Fetch relevant DesignBoundaries (10-100)
4. **For each boundary:**
   - Encode boundary → 128-dim vector (cached)
   - Call Rust sandbox via FFI
   - Sandbox returns {decision, slice_similarities}
5. **Aggregate decisions** → ALL mandatory must pass, optional use weighted average
6. **Store telemetry** → Log decision + slice scores
7. **Return verdict** → ALLOW (1) or BLOCK (0)

### Technology Stack

| Component          | Technology                                      |
|--------------------|-------------------------------------------------|
| SDK (TypeScript)   | TypeScript, axios for HTTP                      |
| SDK (Python)       | Python 3.11+, requests                          |
| Control Plane      | React 18, Vite, TypeScript, TanStack Query      |
| Management Plane   | Python 3.11+, FastAPI, sentence-transformers    |
| Semantic Sandbox   | Rust 1.75+, CDylib target                       |
| Data Store         | SQLite for MVP (boundaries + telemetry)         |
| Embeddings         | sentence-transformers (all-MiniLM-L6-v2, 384d)  |

---

## 2. Data Contracts

### 2.1 Slot Contract (Versioned: v1)

Four slots with fixed vocabularies and scaling rules. Both IntentEvent and DesignBoundary must conform.

```python
# Slot definitions
SLOT_CONTRACT_V1 = {
    "version": "v1",
    "slots": {
        "action": {
            "dim_range": [0, 31],
            "vocabulary": ["read", "write", "delete", "export", "execute", "update"],
            "encoding": "categorical_enum"
        },
        "resource": {
            "dim_range": [32, 63],
            "fields": ["type", "name", "location"],
            "vocabulary": {
                "type": ["database", "file", "api", "service", "user_data"],
                "location": ["local", "cloud", "external"]
            },
            "encoding": "structured_text"
        },
        "data": {
            "dim_range": [64, 95],
            "fields": ["categories", "pii", "volume"],
            "vocabulary": {
                "categories": ["pii", "financial", "medical", "public", "internal"],
                "volume": ["row", "table", "dump", "bulk"]
            },
            "encoding": "multi_label"
        },
        "risk": {
            "dim_range": [96, 127],
            "fields": ["authn", "network", "timeOfDay"],
            "vocabulary": {
                "authn": ["none", "user", "mfa", "service"],
                "network": ["corp", "vpn", "public"]
            },
            "units": {
                "timeOfDay": "hour_0_23"
            },
            "encoding": "mixed"
        }
    }
}
```

### 2.2 IntentEvent Schema

```typescript
// TypeScript SDK
interface IntentEvent {
  id: string;                    // UUID
  schemaVersion: "v1";
  tenantId: string;
  timestamp: number;             // Unix timestamp

  actor: {
    id: string;
    type: "user" | "service";
  };

  action: "read" | "write" | "delete" | "export" | "execute" | "update";

  resource: {
    type: string;                // from vocabulary
    name?: string;
    location?: string;           // from vocabulary
  };

  data: {
    categories: string[];        // from vocabulary
    pii?: boolean;
    volume?: "row" | "table" | "dump" | "bulk";
  };

  risk: {
    authn: "none" | "user" | "mfa" | "service";
    network: "corp" | "vpn" | "public";
    timeOfDay?: number;          // 0-23
  };

  context?: Record<string, any>; // Future extensibility
}
```

```python
# Python SDK
from pydantic import BaseModel, Field
from typing import Literal, Optional
from datetime import datetime

class Actor(BaseModel):
    id: str
    type: Literal["user", "service"]

class Resource(BaseModel):
    type: str
    name: Optional[str] = None
    location: Optional[str] = None

class Data(BaseModel):
    categories: list[str]
    pii: Optional[bool] = None
    volume: Optional[Literal["row", "table", "dump", "bulk"]] = None

class Risk(BaseModel):
    authn: Literal["none", "user", "mfa", "service"]
    network: Literal["corp", "vpn", "public"]
    timeOfDay: Optional[int] = Field(None, ge=0, le=23)

class IntentEvent(BaseModel):
    id: str
    schemaVersion: Literal["v1"] = "v1"
    tenantId: str
    timestamp: float
    actor: Actor
    action: Literal["read", "write", "delete", "export", "execute", "update"]
    resource: Resource
    data: Data
    risk: Risk
    context: Optional[dict] = None
```

### 2.3 DesignBoundary Schema

```typescript
// TypeScript Control Plane
interface DesignBoundary {
  id: string;
  name: string;
  status: "active" | "disabled";
  type: "mandatory" | "optional";
  boundarySchemaVersion: "v1";

  scope: {
    tenantId: string;
    domains?: string[];          // For candidate filtering
  };

  rules: {
    // Per-slice thresholds (0.0 - 1.0)
    thresholds: {
      action: number;
      resource: number;
      data: number;
      risk: number;
    };

    // For optional boundaries
    weights?: {
      action: number;
      resource: number;
      data: number;
      risk: number;
    };

    // Aggregation method
    decision: "min" | "weighted-avg";

    // For weighted-avg
    globalThreshold?: number;    // 0.0 - 1.0
  };

  notes?: string;
  createdAt: number;
  updatedAt: number;
}
```

```python
# Python Management Plane
from pydantic import BaseModel, Field
from typing import Literal, Optional

class BoundaryScope(BaseModel):
    tenantId: str
    domains: Optional[list[str]] = None

class SliceThresholds(BaseModel):
    action: float = Field(ge=0.0, le=1.0)
    resource: float = Field(ge=0.0, le=1.0)
    data: float = Field(ge=0.0, le=1.0)
    risk: float = Field(ge=0.0, le=1.0)

class SliceWeights(BaseModel):
    action: float = Field(default=1.0, ge=0.0)
    resource: float = Field(default=1.0, ge=0.0)
    data: float = Field(default=1.0, ge=0.0)
    risk: float = Field(default=1.0, ge=0.0)

class BoundaryRules(BaseModel):
    thresholds: SliceThresholds
    weights: Optional[SliceWeights] = None
    decision: Literal["min", "weighted-avg"]
    globalThreshold: Optional[float] = Field(None, ge=0.0, le=1.0)

class DesignBoundary(BaseModel):
    id: str
    name: str
    status: Literal["active", "disabled"]
    type: Literal["mandatory", "optional"]
    boundarySchemaVersion: Literal["v1"] = "v1"
    scope: BoundaryScope
    rules: BoundaryRules
    notes: Optional[str] = None
    createdAt: float
    updatedAt: float
```

### 2.4 VectorEnvelope (FFI Boundary)

```rust
// Rust CDylib interface
#[repr(C)]
pub struct VectorEnvelope {
    pub intent: [f32; 128],
    pub boundary: [f32; 128],
    pub thresholds: [f32; 4],      // action, resource, data, risk
    pub weights: [f32; 4],          // for weighted-avg
    pub decision_mode: u8,          // 0 = min, 1 = weighted-avg
    pub global_threshold: f32,      // for weighted-avg
}

#[repr(C)]
pub struct ComparisonResult {
    pub decision: u8,               // 0 = block, 1 = allow
    pub slice_similarities: [f32; 4], // action, resource, data, risk
}
```

```python
# Python FFI bridge using ctypes
import ctypes
import numpy as np
from pathlib import Path

class VectorEnvelope(ctypes.Structure):
    _fields_ = [
        ("intent", ctypes.c_float * 128),
        ("boundary", ctypes.c_float * 128),
        ("thresholds", ctypes.c_float * 4),
        ("weights", ctypes.c_float * 4),
        ("decision_mode", ctypes.c_uint8),
        ("global_threshold", ctypes.c_float),
    ]

class ComparisonResult(ctypes.Structure):
    _fields_ = [
        ("decision", ctypes.c_uint8),
        ("slice_similarities", ctypes.c_float * 4),
    ]

# Load the CDylib
sandbox_lib = ctypes.CDLL(str(Path(__file__).parent / "libsemantic_sandbox.so"))
sandbox_lib.compare_vectors.argtypes = [ctypes.POINTER(VectorEnvelope)]
sandbox_lib.compare_vectors.restype = ComparisonResult
```

---

## 3. Component Implementation

### 3.1 SDK (TypeScript)

**Location:** `tupl_sdk/typescript/`

**File Structure:**
```
tupl_sdk/typescript/
├── src/
│   ├── index.ts              # Main export
│   ├── client.ts             # SDK client class
│   ├── types.ts              # IntentEvent types
│   ├── capture.ts            # Intent capture helpers
│   └── buffer.ts             # Async buffering queue
├── package.json
├── tsconfig.json
└── README.md
```

**Key Files:**

**`src/client.ts`**
```typescript
import axios, { AxiosInstance } from 'axios';
import { IntentEvent } from './types';
import { EventBuffer } from './buffer';

export interface SDKConfig {
  managementPlaneUrl: string;
  apiKey: string;
  tenantId: string;
  flushIntervalMs?: number;  // Default: 1000
  bufferSize?: number;        // Default: 1000
}

export class TuplSDK {
  private client: AxiosInstance;
  private buffer: EventBuffer;
  private config: SDKConfig;

  constructor(config: SDKConfig) {
    this.config = config;
    this.client = axios.create({
      baseURL: config.managementPlaneUrl,
      headers: {
        'Authorization': `Bearer ${config.apiKey}`,
        'Content-Type': 'application/json',
      },
    });

    this.buffer = new EventBuffer({
      maxSize: config.bufferSize || 1000,
      flushInterval: config.flushIntervalMs || 1000,
      onFlush: (events) => this.sendBatch(events),
    });
  }

  async captureIntent(event: Omit<IntentEvent, 'id' | 'timestamp' | 'schemaVersion' | 'tenantId'>): Promise<void> {
    const fullEvent: IntentEvent = {
      id: crypto.randomUUID(),
      schemaVersion: 'v1',
      tenantId: this.config.tenantId,
      timestamp: Date.now(),
      ...event,
    };

    this.buffer.add(fullEvent);
  }

  private async sendBatch(events: IntentEvent[]): Promise<void> {
    try {
      await this.client.post('/intents/batch', { events });
    } catch (error) {
      console.error('Failed to send intent batch:', error);
      // TODO: Add retry logic
    }
  }

  async flush(): Promise<void> {
    await this.buffer.flush();
  }

  close(): void {
    this.buffer.close();
  }
}
```

**`src/buffer.ts`**
```typescript
export interface BufferConfig<T> {
  maxSize: number;
  flushInterval: number;
  onFlush: (items: T[]) => Promise<void>;
}

export class EventBuffer<T> {
  private buffer: T[] = [];
  private timer: NodeJS.Timeout | null = null;
  private config: BufferConfig<T>;

  constructor(config: BufferConfig<T>) {
    this.config = config;
    this.startTimer();
  }

  add(item: T): void {
    this.buffer.push(item);
    if (this.buffer.length >= this.config.maxSize) {
      this.flush();
    }
  }

  async flush(): Promise<void> {
    if (this.buffer.length === 0) return;

    const items = this.buffer.splice(0);
    await this.config.onFlush(items);
  }

  private startTimer(): void {
    this.timer = setInterval(() => {
      this.flush();
    }, this.config.flushInterval);
  }

  close(): void {
    if (this.timer) {
      clearInterval(this.timer);
      this.timer = null;
    }
    this.flush();
  }
}
```

**Integration Example:**
```typescript
import { TuplSDK } from 'tupl_sdk';

const sdk = new TuplSDK({
  managementPlaneUrl: 'http://localhost:8000',
  apiKey: 'dev-key',
  tenantId: 'tenant-1',
});

// Wrap LLM call
async function callLLM(prompt: string) {
  await sdk.captureIntent({
    actor: { id: 'user-123', type: 'user' },
    action: 'execute',
    resource: { type: 'api', name: 'openai' },
    data: { categories: ['public'], pii: false },
    risk: { authn: 'user', network: 'public', timeOfDay: new Date().getHours() },
  });

  const response = await openai.chat.completions.create({ /* ... */ });
  return response;
}
```

### 3.2 SDK (Python)

**Location:** `tupl_sdk/python/`

**File Structure:**
```
tupl_sdk/python/
├── tupl_sdk/
│   ├── __init__.py
│   ├── client.py
│   ├── types.py
│   ├── buffer.py
│   └── capture.py
├── tests/
├── pyproject.toml
└── README.md
```

**Key Files:**

**`tupl_sdk/client.py`**
```python
import requests
import uuid
import time
from typing import Optional
from .types import IntentEvent, Actor, Resource, Data, Risk
from .buffer import EventBuffer

class TuplSDK:
    def __init__(
        self,
        management_plane_url: str,
        api_key: str,
        tenant_id: str,
        flush_interval_ms: int = 1000,
        buffer_size: int = 1000,
    ):
        self.management_plane_url = management_plane_url
        self.api_key = api_key
        self.tenant_id = tenant_id
        self.session = requests.Session()
        self.session.headers.update({
            "Authorization": f"Bearer {api_key}",
            "Content-Type": "application/json",
        })

        self.buffer = EventBuffer(
            max_size=buffer_size,
            flush_interval=flush_interval_ms / 1000.0,
            on_flush=self._send_batch,
        )

    def capture_intent(
        self,
        actor: Actor,
        action: str,
        resource: Resource,
        data: Data,
        risk: Risk,
        context: Optional[dict] = None,
    ) -> None:
        event = IntentEvent(
            id=str(uuid.uuid4()),
            schemaVersion="v1",
            tenantId=self.tenant_id,
            timestamp=time.time(),
            actor=actor,
            action=action,
            resource=resource,
            data=data,
            risk=risk,
            context=context,
        )
        self.buffer.add(event.model_dump())

    def _send_batch(self, events: list[dict]) -> None:
        try:
            response = self.session.post(
                f"{self.management_plane_url}/intents/batch",
                json={"events": events},
            )
            response.raise_for_status()
        except Exception as e:
            print(f"Failed to send intent batch: {e}")
            # TODO: Add retry logic

    def flush(self) -> None:
        self.buffer.flush()

    def close(self) -> None:
        self.buffer.close()

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        self.close()
```

### 3.3 Control Plane (React + Vite + TypeScript)

**Location:** `control-plane/`

**File Structure:**
```
control-plane/
├── src/
│   ├── main.tsx
│   ├── App.tsx
│   ├── pages/
│   │   ├── BoundaryEditor.tsx
│   │   ├── BoundaryList.tsx
│   │   └── DecisionViewer.tsx
│   ├── components/
│   │   ├── BoundaryForm.tsx
│   │   ├── SliceThresholdEditor.tsx
│   │   └── DecisionTable.tsx
│   ├── api/
│   │   └── client.ts
│   ├── types/
│   │   └── index.ts
│   └── lib/
│       └── utils.ts
├── package.json
├── vite.config.ts
└── tsconfig.json
```

**Key Pages:**

**`src/pages/BoundaryEditor.tsx`** - Form to create/edit DesignBoundary with:
- Name, type (mandatory/optional), status
- Per-slice threshold sliders (0.0 - 1.0)
- Optional weights for weighted-avg
- Scope configuration (tenantId, domains)

**`src/pages/DecisionViewer.tsx`** - Table showing recent decisions with:
- Timestamp, IntentEvent summary
- Decision (ALLOW/BLOCK)
- Per-slice similarities
- Which boundary caused block (if blocked)

**`src/api/client.ts`**
```typescript
import axios from 'axios';
import { DesignBoundary } from '../types';

const client = axios.create({
  baseURL: 'http://localhost:8000',
  headers: { 'Content-Type': 'application/json' },
});

export const api = {
  // Boundaries
  async listBoundaries(tenantId: string): Promise<DesignBoundary[]> {
    const response = await client.get(`/boundaries?tenantId=${tenantId}`);
    return response.data;
  },

  async createBoundary(boundary: DesignBoundary): Promise<DesignBoundary> {
    const response = await client.post('/boundaries', boundary);
    return response.data;
  },

  async updateBoundary(id: string, boundary: Partial<DesignBoundary>): Promise<DesignBoundary> {
    const response = await client.put(`/boundaries/${id}`, boundary);
    return response.data;
  },

  async deleteBoundary(id: string): Promise<void> {
    await client.delete(`/boundaries/${id}`);
  },

  // Telemetry
  async listDecisions(tenantId: string, limit: number = 100) {
    const response = await client.get(`/telemetry/decisions?tenantId=${tenantId}&limit=${limit}`);
    return response.data;
  },
};
```

### 3.4 Management Plane (Python + FastAPI)

**Location:** `management-plane/`

**File Structure:**
```
management-plane/
├── app/
│   ├── __init__.py
│   ├── main.py                    # FastAPI app
│   ├── api/
│   │   ├── intents.py             # Intent ingestion endpoints
│   │   ├── boundaries.py          # Boundary CRUD
│   │   ├── compare.py             # Comparison endpoint
│   │   └── telemetry.py           # Telemetry endpoints
│   ├── encoding/
│   │   ├── __init__.py
│   │   ├── canonicalize.py        # Path normalization
│   │   ├── slots.py               # Slot string builders
│   │   ├── embeddings.py          # sentence-transformers wrapper
│   │   └── projection.py          # Sparse random projection
│   ├── matching/
│   │   ├── __init__.py
│   │   ├── candidates.py          # Candidate filtering
│   │   └── sandbox_bridge.py      # FFI to Rust
│   ├── storage/
│   │   ├── __init__.py
│   │   ├── database.py            # SQLite wrapper
│   │   └── models.py              # SQLAlchemy models
│   └── config.py
├── tests/
├── requirements.txt
└── README.md
```

**Key Files:**

**`app/main.py`**
```python
from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
from app.api import intents, boundaries, compare, telemetry
from app.storage.database import init_db

app = FastAPI(title="Semantic Security Management Plane")

app.add_middleware(
    CORSMiddleware,
    allow_origins=["http://localhost:5173"],  # Control Plane dev server
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

app.include_router(intents.router, prefix="/intents", tags=["intents"])
app.include_router(boundaries.router, prefix="/boundaries", tags=["boundaries"])
app.include_router(compare.router, prefix="/compare", tags=["compare"])
app.include_router(telemetry.router, prefix="/telemetry", tags=["telemetry"])

@app.on_event("startup")
async def startup_event():
    init_db()

@app.get("/health")
async def health():
    return {"status": "healthy"}
```

**`app/encoding/embeddings.py`**
```python
import numpy as np
from sentence_transformers import SentenceTransformer
from functools import lru_cache

class EmbeddingService:
    def __init__(self, model_name: str = "all-MiniLM-L6-v2"):
        self.model = SentenceTransformer(model_name)
        self.embedding_dim = self.model.get_sentence_embedding_dimension()  # 384

    @lru_cache(maxsize=10000)
    def encode(self, text: str) -> np.ndarray:
        """Encode text to embedding vector."""
        return self.model.encode(text, convert_to_numpy=True)

    def encode_batch(self, texts: list[str]) -> np.ndarray:
        """Encode multiple texts."""
        return self.model.encode(texts, convert_to_numpy=True)

# Global instance
embedding_service = EmbeddingService()
```

**`app/encoding/slots.py`**
```python
from app.types import IntentEvent, DesignBoundary

def build_action_slot_string(action: str) -> str:
    """Build deterministic string for action slot."""
    return f"action={action}"

def build_resource_slot_string(resource: dict) -> str:
    """Build deterministic string for resource slot."""
    parts = [f"type={resource['type']}"]
    if resource.get('name'):
        parts.append(f"name={resource['name']}")
    if resource.get('location'):
        parts.append(f"location={resource['location']}")
    return ";".join(parts)

def build_data_slot_string(data: dict) -> str:
    """Build deterministic string for data slot."""
    parts = []
    if data.get('categories'):
        cats = sorted(data['categories'])  # Deterministic order
        parts.append(f"categories={','.join(cats)}")
    if data.get('pii') is not None:
        parts.append(f"pii={str(data['pii']).lower()}")
    if data.get('volume'):
        parts.append(f"volume={data['volume']}")
    return ";".join(parts)

def build_risk_slot_string(risk: dict) -> str:
    """Build deterministic string for risk slot."""
    parts = [
        f"authn={risk['authn']}",
        f"network={risk['network']}",
    ]
    if risk.get('timeOfDay') is not None:
        parts.append(f"timeOfDay={risk['timeOfDay']}")
    return ";".join(parts)

def extract_slot_strings(event: IntentEvent | DesignBoundary) -> tuple[str, str, str, str]:
    """Extract 4 slot strings from IntentEvent or DesignBoundary."""
    # Implementation depends on whether it's an IntentEvent or boundary
    # For IntentEvent:
    if isinstance(event, IntentEvent):
        return (
            build_action_slot_string(event.action),
            build_resource_slot_string(event.resource.model_dump()),
            build_data_slot_string(event.data.model_dump()),
            build_risk_slot_string(event.risk.model_dump()),
        )
    # For DesignBoundary: derive from boundary rules or prototype
    # (Details depend on how boundaries encode their "pattern")
    raise NotImplementedError("Boundary encoding not yet implemented")
```

**`app/encoding/projection.py`**
```python
import numpy as np
from typing import Optional

class SparseRandomProjection:
    """Sparse random projection for dimensionality reduction."""

    def __init__(self, input_dim: int, output_dim: int = 32, seed: int = 42):
        self.input_dim = input_dim
        self.output_dim = output_dim
        self.seed = seed
        self.matrix = self._create_projection_matrix()

    def _create_projection_matrix(self) -> np.ndarray:
        """Create sparse random projection matrix with sqrt(3) scaling."""
        rng = np.random.RandomState(self.seed)
        s = 3  # sparsity parameter
        sqrt_s = np.sqrt(s)

        # Sample from {sqrt(3), 0, -sqrt(3)} with probabilities {1/6, 2/3, 1/6}
        matrix = rng.choice(
            [sqrt_s, 0, -sqrt_s],
            size=(self.output_dim, self.input_dim),
            p=[1/6, 2/3, 1/6]
        )
        return matrix

    def project(self, vector: np.ndarray) -> np.ndarray:
        """Project vector to lower dimension and L2 normalize."""
        projected = self.matrix @ vector
        norm = np.linalg.norm(projected)
        if norm > 0:
            projected = projected / norm
        return projected

# Global projectors (one per slot, fixed seeds for determinism)
PROJECTORS = {
    "action": SparseRandomProjection(384, 32, seed=42),
    "resource": SparseRandomProjection(384, 32, seed=43),
    "data": SparseRandomProjection(384, 32, seed=44),
    "risk": SparseRandomProjection(384, 32, seed=45),
}
```

**`app/encoding/__init__.py`**
```python
import numpy as np
from app.encoding.slots import extract_slot_strings
from app.encoding.embeddings import embedding_service
from app.encoding.projection import PROJECTORS
from app.types import IntentEvent

def encode_to_128d(event: IntentEvent) -> np.ndarray:
    """
    Encode IntentEvent to 128-dimensional vector with 4 slots.

    Pipeline:
    1. Extract 4 slot strings
    2. Embed each with sentence-transformers (384d)
    3. Project each to 32d via sparse random projection
    4. Concatenate to 128d
    5. L2 normalize
    """
    # Step 1: Extract slot strings
    slot_strings = extract_slot_strings(event)

    # Step 2: Embed each slot
    embeddings = [embedding_service.encode(s) for s in slot_strings]

    # Step 3: Project each to 32d
    slot_names = ["action", "resource", "data", "risk"]
    projected = [PROJECTORS[name].project(emb) for name, emb in zip(slot_names, embeddings)]

    # Step 4: Concatenate
    vector_128 = np.concatenate(projected)

    # Step 5: L2 normalize
    norm = np.linalg.norm(vector_128)
    if norm > 0:
        vector_128 = vector_128 / norm

    return vector_128.astype(np.float32)
```

**`app/matching/sandbox_bridge.py`**
```python
import ctypes
import numpy as np
from pathlib import Path
from typing import Tuple

# Load Rust CDylib
SANDBOX_LIB_PATH = Path(__file__).parent.parent.parent / "semantic-sandbox" / "target" / "release" / "libsemantic_sandbox.dylib"  # .so on Linux, .dll on Windows

class VectorEnvelope(ctypes.Structure):
    _fields_ = [
        ("intent", ctypes.c_float * 128),
        ("boundary", ctypes.c_float * 128),
        ("thresholds", ctypes.c_float * 4),
        ("weights", ctypes.c_float * 4),
        ("decision_mode", ctypes.c_uint8),
        ("global_threshold", ctypes.c_float),
    ]

class ComparisonResult(ctypes.Structure):
    _fields_ = [
        ("decision", ctypes.c_uint8),
        ("slice_similarities", ctypes.c_float * 4),
    ]

class SandboxBridge:
    def __init__(self, lib_path: Path = SANDBOX_LIB_PATH):
        self.lib = ctypes.CDLL(str(lib_path))
        self.lib.compare_vectors.argtypes = [ctypes.POINTER(VectorEnvelope)]
        self.lib.compare_vectors.restype = ComparisonResult

    def compare(
        self,
        intent: np.ndarray,
        boundary: np.ndarray,
        thresholds: np.ndarray,
        weights: np.ndarray,
        decision_mode: str,
        global_threshold: float,
    ) -> Tuple[int, np.ndarray]:
        """
        Compare intent and boundary vectors.

        Returns:
            (decision, slice_similarities) where decision is 0 (block) or 1 (allow)
        """
        envelope = VectorEnvelope()
        envelope.intent[:] = intent.astype(np.float32)
        envelope.boundary[:] = boundary.astype(np.float32)
        envelope.thresholds[:] = thresholds.astype(np.float32)
        envelope.weights[:] = weights.astype(np.float32)
        envelope.decision_mode = 0 if decision_mode == "min" else 1
        envelope.global_threshold = global_threshold

        result = self.lib.compare_vectors(ctypes.byref(envelope))

        return (
            int(result.decision),
            np.array(result.slice_similarities, dtype=np.float32)
        )

# Global instance
sandbox = SandboxBridge()
```

**`app/api/compare.py`**
```python
from fastapi import APIRouter, HTTPException
from pydantic import BaseModel
from app.encoding import encode_to_128d
from app.matching.sandbox_bridge import sandbox
from app.storage.database import get_boundaries_for_tenant
from app.types import IntentEvent

router = APIRouter()

class CompareRequest(BaseModel):
    intent: IntentEvent

class BoundaryEvaluation(BaseModel):
    boundaryId: str
    decision: int  # 0 or 1
    sliceSimilarities: list[float]

class CompareResponse(BaseModel):
    finalDecision: int  # 0 = block, 1 = allow
    evaluations: list[BoundaryEvaluation]
    mandatoryPassed: bool
    optionalScore: float

@router.post("/", response_model=CompareResponse)
async def compare_intent(request: CompareRequest):
    """
    Compare an intent against all active boundaries for the tenant.

    Logic:
    1. Encode intent to 128d
    2. Fetch candidate boundaries
    3. For each boundary:
       - Encode boundary to 128d (cached)
       - Call Rust sandbox
       - Store result
    4. Aggregate:
       - ALL mandatory must pass
       - Optional use weighted average
    5. Return verdict
    """
    # Step 1: Encode intent
    intent_vector = encode_to_128d(request.intent)

    # Step 2: Fetch boundaries
    boundaries = get_boundaries_for_tenant(
        request.intent.tenantId,
        status="active"
    )

    if not boundaries:
        return CompareResponse(
            finalDecision=1,
            evaluations=[],
            mandatoryPassed=True,
            optionalScore=1.0,
        )

    # Step 3: Compare against each boundary
    evaluations = []
    mandatory_results = []
    optional_results = []

    for boundary in boundaries:
        # Encode boundary (cached in practice)
        boundary_vector = encode_boundary_to_128d(boundary)

        # Prepare comparison parameters
        thresholds = np.array([
            boundary.rules.thresholds.action,
            boundary.rules.thresholds.resource,
            boundary.rules.thresholds.data,
            boundary.rules.thresholds.risk,
        ])

        weights = np.array([1.0, 1.0, 1.0, 1.0])
        if boundary.rules.weights:
            weights = np.array([
                boundary.rules.weights.action,
                boundary.rules.weights.resource,
                boundary.rules.weights.data,
                boundary.rules.weights.risk,
            ])

        global_threshold = boundary.rules.globalThreshold or 0.85

        # Call sandbox
        decision, slice_sims = sandbox.compare(
            intent=intent_vector,
            boundary=boundary_vector,
            thresholds=thresholds,
            weights=weights,
            decision_mode=boundary.rules.decision,
            global_threshold=global_threshold,
        )

        eval_result = BoundaryEvaluation(
            boundaryId=boundary.id,
            decision=decision,
            sliceSimilarities=slice_sims.tolist(),
        )
        evaluations.append(eval_result)

        if boundary.type == "mandatory":
            mandatory_results.append(decision)
        else:
            optional_results.append((decision, weights, slice_sims))

    # Step 4: Aggregate
    mandatory_passed = all(r == 1 for r in mandatory_results) if mandatory_results else True

    optional_score = 1.0
    if optional_results:
        # Weighted average of optional similarities
        total_weight = sum(w.sum() for _, w, _ in optional_results)
        weighted_sum = sum((w * sims).sum() for _, w, sims in optional_results)
        optional_score = weighted_sum / total_weight if total_weight > 0 else 0.0

    final_decision = 1 if mandatory_passed and optional_score >= 0.5 else 0

    return CompareResponse(
        finalDecision=final_decision,
        evaluations=evaluations,
        mandatoryPassed=mandatory_passed,
        optionalScore=float(optional_score),
    )
```

### 3.5 Semantic Sandbox (Rust CDylib)

**Location:** `semantic-sandbox/`

**File Structure:**
```
semantic-sandbox/
├── src/
│   ├── lib.rs              # Main library entry
│   └── compare.rs          # Comparison logic
├── Cargo.toml
└── README.md
```

**`Cargo.toml`**
```toml
[package]
name = "semantic-sandbox"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
# No external dependencies needed for MVP
```

**`src/lib.rs`**
```rust
mod compare;

use std::slice;

#[repr(C)]
pub struct VectorEnvelope {
    pub intent: [f32; 128],
    pub boundary: [f32; 128],
    pub thresholds: [f32; 4],
    pub weights: [f32; 4],
    pub decision_mode: u8,  // 0 = min, 1 = weighted-avg
    pub global_threshold: f32,
}

#[repr(C)]
pub struct ComparisonResult {
    pub decision: u8,  // 0 = block, 1 = allow
    pub slice_similarities: [f32; 4],
}

#[no_mangle]
pub extern "C" fn compare_vectors(envelope: *const VectorEnvelope) -> ComparisonResult {
    // Safety: Caller must ensure valid pointer
    let envelope = unsafe { &*envelope };

    compare::compare(envelope)
}
```

**`src/compare.rs`**
```rust
use crate::{VectorEnvelope, ComparisonResult};

/// Compute dot product of two slices
#[inline]
fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

/// Compare two 128-dim vectors using slice-based logic
pub fn compare(envelope: &VectorEnvelope) -> ComparisonResult {
    // Slice ranges: [0..31], [32..63], [64..95], [96..127]
    const SLICE_RANGES: [(usize, usize); 4] = [
        (0, 32),    // action
        (32, 64),   // resource
        (64, 96),   // data
        (96, 128),  // risk
    ];

    let mut slice_similarities = [0.0f32; 4];

    // Compute per-slice cosine similarity (dot product since vectors are normalized)
    for (i, (start, end)) in SLICE_RANGES.iter().enumerate() {
        let intent_slice = &envelope.intent[*start..*end];
        let boundary_slice = &envelope.boundary[*start..*end];
        slice_similarities[i] = dot_product(intent_slice, boundary_slice);
    }

    // Decision logic based on mode
    let decision = if envelope.decision_mode == 0 {
        // Mode 0: min (mandatory boundaries)
        // All slices must meet their thresholds
        let all_pass = slice_similarities
            .iter()
            .zip(envelope.thresholds.iter())
            .all(|(sim, thresh)| sim >= thresh);

        if all_pass { 1 } else { 0 }
    } else {
        // Mode 1: weighted-avg (optional boundaries)
        // Compute weighted average and compare to global threshold
        let weighted_sum: f32 = slice_similarities
            .iter()
            .zip(envelope.weights.iter())
            .map(|(sim, weight)| sim * weight)
            .sum();

        let total_weight: f32 = envelope.weights.iter().sum();
        let weighted_avg = if total_weight > 0.0 {
            weighted_sum / total_weight
        } else {
            0.0
        };

        if weighted_avg >= envelope.global_threshold { 1 } else { 0 }
    };

    ComparisonResult {
        decision,
        slice_similarities,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_min_mode_all_pass() {
        let envelope = VectorEnvelope {
            intent: [0.9f32; 128],
            boundary: [1.0f32; 128],
            thresholds: [0.85, 0.85, 0.85, 0.85],
            weights: [1.0, 1.0, 1.0, 1.0],
            decision_mode: 0,
            global_threshold: 0.85,
        };

        let result = compare(&envelope);
        assert_eq!(result.decision, 1);  // Should allow
    }

    #[test]
    fn test_min_mode_one_fail() {
        let mut intent = [0.9f32; 128];
        intent[0..32].fill(0.5);  // Action slice below threshold

        let envelope = VectorEnvelope {
            intent,
            boundary: [1.0f32; 128],
            thresholds: [0.85, 0.85, 0.85, 0.85],
            weights: [1.0, 1.0, 1.0, 1.0],
            decision_mode: 0,
            global_threshold: 0.85,
        };

        let result = compare(&envelope);
        assert_eq!(result.decision, 0);  // Should block
    }

    #[test]
    fn test_weighted_avg_mode() {
        let envelope = VectorEnvelope {
            intent: [0.8f32; 128],
            boundary: [1.0f32; 128],
            thresholds: [0.0, 0.0, 0.0, 0.0],  // Not used in weighted-avg
            weights: [1.0, 1.0, 1.0, 1.0],
            decision_mode: 1,
            global_threshold: 0.75,
        };

        let result = compare(&envelope);
        assert_eq!(result.decision, 1);  // 0.8 >= 0.75, should allow
    }
}
```

**Build Instructions:**
```bash
cd semantic-sandbox
cargo build --release

# Output: target/release/libsemantic_sandbox.dylib (macOS)
#         target/release/libsemantic_sandbox.so (Linux)
#         target/release/semantic_sandbox.dll (Windows)
```

---

## 4. Development Milestones

### Week 1: Contracts and Skeletons

**Goal:** All components can communicate, no real logic yet.

**Tasks:**
1. Create monorepo structure
2. Implement data contracts (TypeScript, Python, Rust)
3. SDK: Basic client that sends IntentEvents via HTTP
4. Control Plane: Empty React app with routing
5. Management Plane: FastAPI skeleton with placeholder endpoints
6. Semantic Sandbox: Rust CDylib that returns dummy results
7. Integration test: SDK → Management Plane → Sandbox (end-to-end ping)

**Acceptance Criteria:**
- [ ] SDK can send IntentEvent batch to Management Plane
- [ ] Control Plane can fetch empty boundary list
- [ ] Management Plane can load and call Rust sandbox via FFI
- [ ] All type definitions match across components

### Week 2: Encoding and Sandbox

**Goal:** Full encoding pipeline and real sandbox logic.

**Tasks:**
1. Implement canonicalization (path normalization)
2. Implement slot string builders (4 slots)
3. Integrate sentence-transformers in Management Plane
4. Implement sparse random projection (4 projectors)
5. Implement encode_to_128d pipeline
6. Implement Rust sandbox comparison logic (min + weighted-avg)
7. Write unit tests for encoding determinism
8. Write unit tests for sandbox logic

**Acceptance Criteria:**
- [ ] Same IntentEvent always produces same 128-dim vector
- [ ] Sandbox correctly implements min and weighted-avg logic
- [ ] End-to-end: IntentEvent → 128d → comparison → decision
- [ ] Unit tests pass for encoding and sandbox

### Week 3: Candidates and Telemetry

**Goal:** Boundary storage, candidate filtering, decision logging.

**Tasks:**
1. Implement SQLite schema (boundaries, telemetry tables)
2. Control Plane: Boundary editor form (create/edit/delete)
3. Management Plane: Boundary CRUD endpoints
4. Management Plane: Candidate filtering by action/domain/type
5. Management Plane: Encode boundaries to 128d (with caching)
6. Management Plane: Full comparison endpoint (/compare)
7. Management Plane: Telemetry storage endpoint
8. Control Plane: Decision viewer table

**Acceptance Criteria:**
- [ ] Can create a boundary in Control Plane and it saves to DB
- [ ] Comparison endpoint returns correct decision for 1 mandatory boundary
- [ ] Telemetry stores decision + slice similarities
- [ ] Decision viewer shows last 100 decisions

### Week 4: Hardening

**Goal:** Production-ready code, error handling, performance validation.

**Tasks:**
1. Add retry logic to SDK buffer
2. Add error handling and logging to all components
3. Add input validation (Pydantic validators, Zod in TS)
4. Implement boundary vector caching (LRU cache)
5. Add circuit breaker for embedding service
6. Write integration tests (full scenarios)
7. Performance benchmarks (latency, throughput)
8. Documentation (README for each component)

**Acceptance Criteria:**
- [ ] SDK handles Management Plane downtime gracefully
- [ ] Invalid inputs return clear error messages
- [ ] P50 latency < 15ms for single comparison
- [ ] P99 latency < 50ms for 100 candidates
- [ ] All integration tests pass
- [ ] README explains how to run each component

---

## 5. Local Development Setup

### 5.1 Monorepo Structure

```
mgmt-plane/                          # Root (this repo)
├── algo.md                          # Algorithm documentation
├── architecture design.png          # Architecture diagram
├── plan.md                          # This file
├── docs/
│   └── plans/
├── tupl_sdk/
│   ├── typescript/                  # TypeScript SDK
│   │   ├── package.json
│   │   └── src/
│   └── python/                      # Python SDK
│       ├── pyproject.toml
│       └── tupl_sdk/
├── control-plane/                   # React app
│   ├── package.json
│   ├── vite.config.ts
│   └── src/
├── management-plane/                # FastAPI service
│   ├── requirements.txt
│   ├── app/
│   └── tests/
├── semantic-sandbox/                # Rust CDylib
│   ├── Cargo.toml
│   └── src/
└── scripts/
    ├── setup.sh                     # One-command setup
    └── start-all.sh                 # Start all services
```

### 5.2 Prerequisites

**System Requirements:**
- Python 3.11+
- Node.js 20+
- Rust 1.75+
- SQLite 3.x (usually pre-installed on macOS)

**Install Dependencies:**
```bash
# Python
cd management-plane
python -m venv venv
source venv/bin/activate
pip install -r requirements.txt

# Node.js (Control Plane)
cd control-plane
npm install

# Node.js (TypeScript SDK)
cd tupl_sdk/typescript
npm install

# Rust
cd semantic-sandbox
cargo build --release
```

### 5.3 Running Components Locally

**Management Plane:**
```bash
cd management-plane
source venv/bin/activate
uvicorn app.main:app --reload --port 8000
# → http://localhost:8000
# → http://localhost:8000/docs (Swagger UI)
```

**Control Plane:**
```bash
cd control-plane
npm run dev
# → http://localhost:5173
```

**Semantic Sandbox:**
```bash
cd semantic-sandbox
cargo build --release
# Library will be loaded by Management Plane via FFI
```

**SDK (for testing):**
```typescript
// TypeScript
cd tupl_sdk/typescript
npm run build
npm link  # For local testing

// In your test app:
npm link tupl_sdk
```

```python
# Python
cd tupl_sdk/python
pip install -e .  # Editable install for development
```

### 5.4 Environment Configuration

**Management Plane (`.env`):**
```bash
# management-plane/.env
DATABASE_URL=sqlite:///./semantic_security.db
EMBEDDING_MODEL=all-MiniLM-L6-v2
SANDBOX_LIB_PATH=../semantic-sandbox/target/release/libsemantic_sandbox.dylib
LOG_LEVEL=INFO
```

**Control Plane (`.env`):**
```bash
# control-plane/.env
VITE_API_URL=http://localhost:8000
```

**SDK Configuration (in code):**
```typescript
const sdk = new TuplSDK({
  managementPlaneUrl: 'http://localhost:8000',
  apiKey: 'dev-key-123',
  tenantId: 'tenant-1',
});
```

### 5.5 Database Setup

**SQLite Schema (auto-created on startup):**
```sql
-- management-plane/app/storage/schema.sql
CREATE TABLE boundaries (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    name TEXT NOT NULL,
    status TEXT NOT NULL,
    type TEXT NOT NULL,
    boundary_schema_version TEXT NOT NULL,
    scope_json TEXT NOT NULL,
    rules_json TEXT NOT NULL,
    notes TEXT,
    created_at REAL NOT NULL,
    updated_at REAL NOT NULL,
    vector_128 BLOB  -- Cached 128-dim vector
);

CREATE INDEX idx_boundaries_tenant ON boundaries(tenant_id);
CREATE INDEX idx_boundaries_status ON boundaries(status);

CREATE TABLE telemetry (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    timestamp REAL NOT NULL,
    intent_id TEXT NOT NULL,
    intent_json TEXT NOT NULL,
    final_decision INTEGER NOT NULL,  -- 0 or 1
    mandatory_passed INTEGER NOT NULL,
    optional_score REAL NOT NULL,
    evaluations_json TEXT NOT NULL  -- Array of boundary evaluations
);

CREATE INDEX idx_telemetry_tenant_time ON telemetry(tenant_id, timestamp DESC);
```

---

## 6. Testing Strategy

### 6.1 Unit Tests

**SDK (TypeScript):**
```bash
cd tupl_sdk/typescript
npm test
```
- Test buffer flushing logic
- Test event serialization
- Test retry logic

**Management Plane (Python):**
```bash
cd management-plane
pytest tests/
```
- Test canonicalization (same input → same output)
- Test slot string builders (determinism)
- Test encoding pipeline (128-dim output)
- Test projection (L2 normalization)
- Test FFI bridge (mock sandbox)

**Semantic Sandbox (Rust):**
```bash
cd semantic-sandbox
cargo test
```
- Test min mode (all pass / one fail)
- Test weighted-avg mode
- Test slice similarity computation

### 6.2 Integration Tests

**Scenario 1: Allow by mandatory boundary**
- Create mandatory boundary with low thresholds
- Send intent that meets all thresholds
- Verify decision = 1 (allow)

**Scenario 2: Block by mandatory boundary**
- Create mandatory boundary with high thresholds
- Send intent that fails one threshold
- Verify decision = 0 (block)
- Verify telemetry logged

**Scenario 3: Optional boundary weighted average**
- Create 3 optional boundaries with different weights
- Send intent
- Verify weighted score calculation
- Verify decision based on global threshold

**Scenario 4: Mixed mandatory + optional**
- Create 1 mandatory + 2 optional boundaries
- Test: mandatory fails → block (regardless of optional)
- Test: mandatory passes, optional below threshold → block
- Test: mandatory passes, optional above threshold → allow

### 6.3 Performance Benchmarks

**Target Metrics:**
- Single comparison (Management Plane → Sandbox): < 5ms
- Encoding (IntentEvent → 128d): < 10ms (including embedding)
- Full comparison with 100 candidates: < 100ms p50, < 200ms p99
- SDK buffer throughput: 1K events/sec with < 1% drops

**Benchmark Script:**
```python
# management-plane/tests/benchmark.py
import time
import numpy as np
from app.encoding import encode_to_128d
from app.matching.sandbox_bridge import sandbox

def benchmark_encoding():
    intent = create_test_intent()

    times = []
    for _ in range(1000):
        start = time.perf_counter()
        vector = encode_to_128d(intent)
        elapsed = (time.perf_counter() - start) * 1000  # ms
        times.append(elapsed)

    print(f"Encoding - p50: {np.percentile(times, 50):.2f}ms, p99: {np.percentile(times, 99):.2f}ms")

def benchmark_sandbox():
    intent = np.random.rand(128).astype(np.float32)
    boundary = np.random.rand(128).astype(np.float32)

    times = []
    for _ in range(10000):
        start = time.perf_counter()
        result = sandbox.compare(
            intent, boundary,
            np.array([0.85, 0.85, 0.85, 0.85]),
            np.array([1.0, 1.0, 1.0, 1.0]),
            "min", 0.85
        )
        elapsed = (time.perf_counter() - start) * 1000
        times.append(elapsed)

    print(f"Sandbox - p50: {np.percentile(times, 50):.2f}ms, p99: {np.percentile(times, 99):.2f}ms")
```

---

## 7. Implementation Notes

### 7.1 Key Design Decisions

**1. sentence-transformers (local) for MVP**
- **Why:** Zero API costs, full control, good quality embeddings
- **Trade-off:** Need to manage model (~100MB), GPU recommended for speed
- **Future:** Can swap to OpenAI/Cohere with minimal code changes

**2. CDylib FFI for sandbox**
- **Why:** Lowest overhead, fastest execution (sub-ms comparisons)
- **Trade-off:** Crash in Rust can affect Python process
- **Mitigation:** Rust code is minimal and thoroughly tested

**3. SQLite for MVP**
- **Why:** Zero setup, embedded, perfect for local dev
- **Trade-off:** Not suitable for distributed deployment
- **Future:** Migrate to PostgreSQL when scaling out

**4. Weighted average for optional boundaries**
- **Why:** Simple, single threshold to tune
- **Trade-off:** Less granular than per-slice floors
- **Note:** Can add per-slice floors later if needed

**5. Simple attribute filtering (no LSH)**
- **Why:** Fewer moving parts, sufficient for 100-1K boundaries
- **Trade-off:** O(n) candidate selection
- **Future:** Add LSH when boundaries > 1K

### 7.2 Encoding Determinism

**Critical for reproducibility:**
- Fixed seeds for all random projections (42, 43, 44, 45)
- Sorted keys in canonicalization where order doesn't matter
- Explicit order preservation for paths (action.subaction)
- Cache embedding model to avoid version drift

**Validation:**
```python
# Test determinism
intent = create_test_intent()
vector1 = encode_to_128d(intent)
vector2 = encode_to_128d(intent)
assert np.allclose(vector1, vector2), "Encoding is not deterministic!"
```

### 7.3 Slice Semantics Preservation

**How slicing is maintained:**
1. Each slot gets its own embedding call with its own projection
2. Projectors use different fixed seeds (42-45)
3. Concatenation order is fixed: [action, resource, data, risk]
4. Sandbox compares slice-to-slice, never whole vector
5. Thresholds and weights are per-slice

**Why this matters:**
- Prevents cross-slice contamination
- Enables per-slice explainability
- Matches algorithm design from algo.md

### 7.4 Caching Strategy

**What to cache:**
- Boundary vectors (128d) → LRU cache, 1000 entries
- Slot string embeddings (384d) → LRU cache, 10K entries
- Loaded embedding model → Singleton

**What NOT to cache:**
- IntentEvent vectors (always fresh, never repeated)
- Comparison results (needed for telemetry)

### 7.5 Error Handling

**SDK:**
- Network errors → retry with exponential backoff (max 3 attempts)
- Buffer full → drop oldest events, log warning
- Invalid event → log error, skip

**Management Plane:**
- Invalid request → 400 with detailed error message
- Sandbox crash → 500, log stack trace, circuit breaker
- Embedding timeout → 503, retry queue

**Control Plane:**
- API errors → toast notification with retry button
- Invalid form → inline validation errors
- Network offline → cache writes, sync when online

---

## 8. Next Steps After MVP

### 8.1 Post-MVP Enhancements

1. **Learned Projection Matrix**
   - Replace sparse random projection with LDA/contrastive learning
   - Train on accumulated telemetry (allow/block labels)
   - Expected improvement: 20-30% accuracy gain

2. **Boundary Evolution**
   - Anchor addition for false negatives
   - Threshold tuning via ROC analysis
   - Region splitting for high-variance boundaries

3. **LSH for Candidate Selection**
   - When boundaries > 1K
   - Cosine LSH with 10 tables, 16-bit hashes

4. **Multi-Region Boundaries**
   - Support OR logic (multiple valid patterns)
   - Prototype + anchors per region

5. **Real-Time Feedback Loop**
   - Human-in-the-loop boundary refinement
   - A/B testing for threshold changes

### 8.2 Scaling Beyond Laptop

**When to scale:**
- More than 1 tenant with boundaries
- More than 10K intents/day
- Need for HA/disaster recovery

**Migration path:**
- SQLite → PostgreSQL
- Single Management Plane → Load-balanced replicas
- Local embedding model → Dedicated embedding service
- CDylib → WASM for better isolation

---

## 9. Glossary

| Term | Definition |
|------|------------|
| **IntentEvent** | Structured record of an LLM/tool call captured by the SDK |
| **DesignBoundary** | Policy rule with per-slice thresholds and aggregation logic |
| **Slot** | One of four semantic categories: action, resource, data, risk |
| **Slice** | 32-dim subvector within the 128-dim embedding |
| **Canonicalization** | Normalization of paths/keys for deterministic encoding |
| **Sparse Random Projection** | Dimensionality reduction via sparse random matrix |
| **Sandbox** | Isolated Rust library that performs vector comparison |
| **CDylib** | C-compatible dynamic library for FFI |
| **Mandatory Boundary** | Must pass (min aggregation), single failure = block |
| **Optional Boundary** | Nice-to-have (weighted-avg aggregation) |
| **Telemetry** | Logged decision records with slice similarities |

---

## 10. References

- **Algorithm Document:** `algo.md` (this repo)
- **Architecture Diagram:** `architecture design.png` (this repo)
- **sentence-transformers:** https://www.sbert.net/
- **FastAPI:** https://fastapi.tiangolo.com/
- **Rust FFI:** https://doc.rust-lang.org/nomicon/ffi.html
- **Sparse Random Projection:** Johnson-Lindenstrauss lemma
- **Cosine Similarity:** https://en.wikipedia.org/wiki/Cosine_similarity

---

**END OF PLAN**
\n---

## Appendix: Slot Contract v1.1 - Constraints-Only Encoding (MVP)

**Date:** 2025-11-12
**Status:** Simplified MVP - constraints only, exemplars deferred

### Key Changes from v1.0

1. **actor_type in action slot** - Include `actor.type` (user/service) in action slot encoding; exclude `actor.id` (telemetry only)
2. **Boundary constraints** - Boundaries now encode allowed operation patterns using same vocabulary as intents
3. **Simplified vocabularies** - Reduced to MVP essentials (database/file/api only, no exemplars)

### Simplified Constraint Schema

```python
class ActionConstraint(BaseModel):
    actions: list[Literal["read", "write", "delete", "export", "execute", "update"]]
    actor_types: list[Literal["user", "service"]]

class ResourceConstraint(BaseModel):
    types: list[Literal["database", "file", "api"]]  # MVP: removed service, user_data
    names: Optional[list[str]] = None  # Exact match only for MVP
    locations: Optional[list[Literal["local", "cloud"]]] = None  # MVP: removed external

class DataConstraint(BaseModel):
    sensitivity: list[Literal["internal", "public"]]  # Simplified from categories
    pii: Optional[bool] = None
    volume: Optional[Literal["single", "bulk"]] = None  # Simplified from row/table/dump/bulk

class RiskConstraint(BaseModel):
    authn: Literal["required", "not_required"]  # Simplified from none/user/mfa/service
    # MVP: removed network, timeOfDay

class BoundaryConstraints(BaseModel):
    action: ActionConstraint
    resource: ResourceConstraint
    data: DataConstraint
    risk: RiskConstraint

class DesignBoundaryV1_1(BaseModel):
    # ... existing v1.0 fields (id, name, status, type, scope, rules) ...
    boundarySchemaVersion: Literal["v1.1"] = "v1.1"
    constraints: BoundaryConstraints  # NEW: replaces encoding block
```

### Slot Encoding Examples

**Action slot:**
```
Boundary: "action: read, write | actor_type: user"
Intent:   "action: read | actor_type: user"
Result:   High similarity (>0.8) ✓
```

**Resource slot:**
```
Boundary: "type: database | location: cloud"
Intent:   "type: database | name: prod_users | location: cloud"
Result:   High similarity (type + location match) ✓
```

**Data slot:**
```
Boundary: "sensitivity: internal | pii: false | volume: single"
Intent:   "sensitivity: internal | pii: false | volume: single"
Result:   Perfect match ✓
```

**Risk slot:**
```
Boundary: "authn: required"
Intent:   "authn: mfa"  (mfa = required)
Result:   High similarity ✓
```

### Encoding Algorithm (Constraints-Only)

For each slot:
1. Build constraint slot string (e.g., `"action: read, write | actor_type: user"`)
2. Embed with sentence-transformers → 384d
3. Sparse random projection → 32d
4. L2 normalize
5. Concatenate 4 slots → 128d → final L2 normalize

**No blending needed** - constraints only (exemplars deferred post-MVP)

### Deferred for Post-MVP

- Exemplars (example-based encoding)
- Blend configuration (constraints vs exemplars weights)
- Complex name patterns (glob, regex)
- Hour ranges for timeOfDay
- Network context in risk slot

### Expected Test Results

- Read database (user) vs "Safe Read Access" → all slices >0.8 → ALLOW
- Delete database (user) vs "Safe Read Access" → action slice <0.5 → BLOCK
- Deterministic encoding: same input → identical 128d vector
