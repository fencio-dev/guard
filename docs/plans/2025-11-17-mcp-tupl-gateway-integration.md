# MCP Tupl Gateway Integration Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Integrate Tupl security enforcement tools from mcp-tupl-server into mcp-gateway as native TypeScript tools, enabling users to wrap their LangGraph agents with enforcement_agent() via MCP tools.

**Architecture:** Port 4 essential tools (wrap_agent, list_families, get_telemetry, get_config) as native gateway tools. Skip NLP generation (users configure via Control Plane UI). Create TypeScript HTTP client for Management Plane API. Port 14 rule family definitions (L0-L6) from Python to TypeScript.

**Tech Stack:** TypeScript, Node.js, axios (HTTP client), MCP SDK, Management Plane REST API

---

## Context: Full System Architecture

```
User (Claude Code/Cursor)
  ↓
MCP Gateway (TypeScript)
  ├─ Native Tupl Tools (NEW - this plan)
  │   ├─ wrap_agent        → Generate enforcement wrapper code
  │   ├─ list_rule_families → Browse 14 rule families (L0-L6)
  │   ├─ get_telemetry     → View enforcement decisions
  │   └─ get_agent_config  → View agent configuration
  │
  └─ MCP Server Proxying (EXISTING)
      └─ Proxy to other MCP servers

SDK (enforcement_agent wrapper)
  ↓
Management Plane (Python/FastAPI)
  ├─ Encode intents to 128d vectors
  ├─ Compare against boundaries
  └─ Store boundaries, telemetry
  ↓
Data Plane (Rust Bridge)
  ├─ Store 14 rule families (L0-L6)
  ├─ Layer-by-layer enforcement
  └─ Pre-encoded rule anchors
  ↓
Semantic Sandbox (Rust FFI)
  └─ Fast vector comparison (<1ms)
```

**Key Insight:** Users write agents in their coding environment, use MCP tools to get wrapper code, configure rules in Control Plane UI, and enforcement happens automatically at runtime.

---

## Task 1: Create Tupl Module Structure

**Files:**
- Create: `mcp-gateway/src/tupl/index.ts`
- Create: `mcp-gateway/src/tupl/clients/management-plane.ts`
- Create: `mcp-gateway/src/tupl/schemas/rule-families.ts`
- Create: `mcp-gateway/src/tupl/schemas/types.ts`
- Create: `mcp-gateway/src/tupl/tools/wrap-agent.ts`
- Create: `mcp-gateway/src/tupl/tools/list-families.ts`
- Create: `mcp-gateway/src/tupl/tools/telemetry.ts`
- Create: `mcp-gateway/src/tupl/tools/get-config.ts`

**Step 1: Create directory structure**

```bash
cd mcp-gateway/src
mkdir -p tupl/clients tupl/schemas tupl/tools
```

**Step 2: Install dependencies**

```bash
cd mcp-gateway
npm install axios
```

Expected: Package installed successfully

**Step 3: Create index.ts barrel export**

File: `mcp-gateway/src/tupl/index.ts`

```typescript
// Barrel export for Tupl module
export * from './clients/management-plane';
export * from './schemas/rule-families';
export * from './schemas/types';
export * from './tools/wrap-agent';
export * from './tools/list-families';
export * from './tools/telemetry';
export * from './tools/get-config';
```

**Step 4: Create types file**

File: `mcp-gateway/src/tupl/schemas/types.ts`

```typescript
/**
 * Type definitions for Tupl data structures
 * Based on Python models from management-plane and mcp-tupl-server
 */

export interface TelemetryEvent {
  id: string;
  timestamp: number;
  tenant_id: string;
  agent_id?: string;
  decision: 0 | 1; // 0=block, 1=allow
  intent: {
    action: string;
    resource: string;
    actor: string;
  };
  evidence: Array<{
    boundary_id: string;
    similarities: number[];
  }>;
}

export interface BoundaryInfo {
  id: string;
  name: string;
  status: 'active' | 'disabled';
  effect: 'allow' | 'deny';
  layers?: string[];
}

export interface AgentConfigResponse {
  agent_id: string;
  boundaries: BoundaryInfo[];
  note: string;
}

export interface TelemetryResponse {
  total: number;
  limit: number;
  events: TelemetryEvent[];
  summary: {
    allowed: number;
    blocked: number;
  };
}
```

**Step 5: Commit structure**

```bash
git add src/tupl/
git commit -m "feat(tupl): create module structure and types"
```

---

## Task 2: Implement Management Plane HTTP Client

**Files:**
- Create: `mcp-gateway/src/tupl/clients/management-plane.ts`
- Test: `mcp-gateway/tests/tupl/management-plane.test.ts`

**Step 1: Write failing test**

File: `mcp-gateway/tests/tupl/management-plane.test.ts`

```typescript
import { describe, it, expect, beforeEach } from '@jest/globals';
import { ManagementPlaneClient } from '../../src/tupl/clients/management-plane';
import nock from 'nock';

describe('ManagementPlaneClient', () => {
  let client: ManagementPlaneClient;
  const baseURL = 'http://localhost:8000';

  beforeEach(() => {
    client = new ManagementPlaneClient(baseURL);
  });

  it('should fetch telemetry with default params', async () => {
    nock(baseURL)
      .get('/api/v1/telemetry')
      .query({ limit: 100 })
      .reply(200, {
        total: 5,
        events: [
          { id: 'evt-1', decision: 1, timestamp: Date.now() },
          { id: 'evt-2', decision: 0, timestamp: Date.now() }
        ]
      });

    const result = await client.getTelemetry({ limit: 100 });
    expect(result.total).toBe(5);
    expect(result.events).toHaveLength(2);
  });

  it('should fetch boundaries', async () => {
    nock(baseURL)
      .get('/api/v1/boundaries')
      .reply(200, [
        { id: 'bnd-1', name: 'Test Boundary', status: 'active' }
      ]);

    const result = await client.getBoundaries();
    expect(result).toHaveLength(1);
    expect(result[0].id).toBe('bnd-1');
  });
});
```

**Step 2: Run test to verify it fails**

```bash
npm test tests/tupl/management-plane.test.ts
```

Expected: FAIL with "ManagementPlaneClient is not defined"

**Step 3: Implement ManagementPlaneClient**

File: `mcp-gateway/src/tupl/clients/management-plane.ts`

```typescript
import axios, { AxiosInstance } from 'axios';

/**
 * HTTP client for Tupl Management Plane API
 *
 * The Management Plane provides:
 * - Intent encoding to 128d vectors
 * - Boundary storage and retrieval
 * - Telemetry event storage
 */
export class ManagementPlaneClient {
  private baseURL: string;
  private client: AxiosInstance;

  constructor(baseURL: string) {
    this.baseURL = baseURL;
    this.client = axios.create({
      baseURL,
      timeout: 10000,
      headers: { 'Content-Type': 'application/json' }
    });
  }

  /**
   * Fetch enforcement telemetry events
   */
  async getTelemetry(params: {
    limit?: number;
    tenant_id?: string;
    agent_id?: string;
    decision?: 'allow' | 'block';
  }): Promise<any> {
    const queryParams: Record<string, any> = {
      limit: params.limit || 100
    };

    if (params.tenant_id) queryParams.tenant_id = params.tenant_id;
    if (params.agent_id) queryParams.agent_id = params.agent_id;
    if (params.decision) {
      queryParams.decision = params.decision === 'allow' ? 1 : 0;
    }

    const response = await this.client.get('/api/v1/telemetry', {
      params: queryParams
    });
    return response.data;
  }

  /**
   * Fetch all design boundaries
   */
  async getBoundaries(params?: {
    tenant_id?: string;
    agent_id?: string;
  }): Promise<any[]> {
    const response = await this.client.get('/api/v1/boundaries', {
      params: params || {}
    });
    return response.data;
  }

  /**
   * Get a specific boundary by ID
   */
  async getBoundary(boundaryId: string): Promise<any> {
    const response = await this.client.get(`/api/v1/boundaries/${boundaryId}`);
    return response.data;
  }

  /**
   * Check health of Management Plane
   */
  async health(): Promise<{ status: string }> {
    const response = await this.client.get('/health');
    return response.data;
  }
}
```

**Step 4: Install test dependencies**

```bash
npm install --save-dev nock @types/jest @jest/globals
```

**Step 5: Run test to verify it passes**

```bash
npm test tests/tupl/management-plane.test.ts
```

Expected: PASS (2 tests)

**Step 6: Commit client implementation**

```bash
git add src/tupl/clients/ tests/tupl/
git commit -m "feat(tupl): add Management Plane HTTP client with tests"
```

---

## Task 3: Port Rule Families Catalog (L0-L6)

**Files:**
- Create: `mcp-gateway/src/tupl/schemas/rule-families.ts`
- Reference: `mcp-tupl-server/src/mcp_tupl/schemas/rule_families.py` (source)
- Test: `mcp-gateway/tests/tupl/rule-families.test.ts`

**Step 1: Write failing test**

File: `mcp-gateway/tests/tupl/rule-families.test.ts`

```typescript
import { describe, it, expect } from '@jest/globals';
import { RULE_FAMILIES_CATALOG, RuleFamilyMetadata } from '../../src/tupl/schemas/rule-families';

describe('Rule Families Catalog', () => {
  it('should have 14 rule families', () => {
    const families = Object.keys(RULE_FAMILIES_CATALOG);
    expect(families).toHaveLength(14);
  });

  it('should have 2 L0 families', () => {
    const l0Families = Object.values(RULE_FAMILIES_CATALOG)
      .filter(f => f.layer === 'L0');
    expect(l0Families).toHaveLength(2);
    expect(l0Families.map(f => f.id)).toContain('net_egress');
    expect(l0Families.map(f => f.id)).toContain('sidecar_spawn');
  });

  it('should have 2 L4 families', () => {
    const l4Families = Object.values(RULE_FAMILIES_CATALOG)
      .filter(f => f.layer === 'L4');
    expect(l4Families).toHaveLength(2);
    expect(l4Families.map(f => f.id)).toContain('tool_whitelist');
    expect(l4Families.map(f => f.id)).toContain('tool_param_constraint');
  });

  it('should have valid schema for net_egress', () => {
    const netEgress = RULE_FAMILIES_CATALOG.net_egress;
    expect(netEgress.params_schema.type).toBe('object');
    expect(netEgress.params_schema.required).toContain('dest_domains');
  });
});
```

**Step 2: Run test to verify it fails**

```bash
npm test tests/tupl/rule-families.test.ts
```

Expected: FAIL with "RULE_FAMILIES_CATALOG is not defined"

**Step 3: Implement rule families catalog (Part 1: L0-L2)**

File: `mcp-gateway/src/tupl/schemas/rule-families.ts`

```typescript
/**
 * Rule Family Catalog (L0-L6)
 *
 * Port from: mcp-tupl-server/src/mcp_tupl/schemas/rule_families.py
 *
 * This catalog defines all 14 rule families across 7 enforcement layers.
 * Each family has metadata and JSON schema for parameters.
 */

export interface RuleFamilyMetadata {
  id: string;
  layer: 'L0' | 'L1' | 'L2' | 'L3' | 'L4' | 'L5' | 'L6';
  layer_name: string;
  name: string;
  description: string;
  default_priority: number;
  params_schema: {
    type: 'object';
    required?: string[];
    properties: Record<string, any>;
  };
}

export const RULE_FAMILIES_CATALOG: Record<string, RuleFamilyMetadata> = {
  // ===================================================================
  // L0 - System Layer
  // ===================================================================

  net_egress: {
    id: 'net_egress',
    layer: 'L0',
    layer_name: 'System',
    name: 'Network Egress Rule',
    description: 'Control which network destinations an agent or sidecar can contact',
    default_priority: 100,
    params_schema: {
      type: 'object',
      required: ['dest_domains'],
      properties: {
        dest_domains: {
          type: 'array',
          items: { type: 'string' },
          description: 'Allowed destination domains (e.g., ["*.company.com", "api.service.io"])'
        },
        port_range: {
          type: 'object',
          properties: {
            min: { type: 'integer' },
            max: { type: 'integer' }
          },
          description: 'Port range (optional)'
        },
        protocol: {
          type: 'string',
          enum: ['TCP', 'UDP', 'HTTP', 'HTTPS'],
          default: 'HTTPS',
          description: 'Network protocol'
        },
        action: {
          type: 'string',
          enum: ['ALLOW', 'DENY', 'REDIRECT'],
          default: 'DENY',
          description: 'Action to take for non-whitelisted domains'
        },
        redirect_target: {
          type: 'string',
          description: 'Redirect target URL (used when action=REDIRECT)'
        }
      }
    }
  },

  sidecar_spawn: {
    id: 'sidecar_spawn',
    layer: 'L0',
    layer_name: 'System',
    name: 'Sidecar Spawn Rule',
    description: 'Restrict which sidecars an agent may launch',
    default_priority: 100,
    params_schema: {
      type: 'object',
      properties: {
        allowed_images: {
          type: 'array',
          items: { type: 'string' },
          default: [],
          description: 'Allowed container images (e.g., ["postgres:14", "redis:alpine"])'
        },
        max_ttl: {
          type: 'integer',
          description: 'Maximum TTL in seconds'
        },
        max_instances: {
          type: 'integer',
          default: 2,
          description: 'Maximum concurrent instances'
        },
        cpu_limit: {
          type: 'integer',
          default: 200,
          description: 'CPU limit in millicores'
        },
        mem_limit: {
          type: 'integer',
          default: 128000000,
          description: 'Memory limit in bytes'
        },
        action: {
          type: 'string',
          enum: ['ALLOW', 'DENY'],
          default: 'DENY',
          description: 'Action when requested_image not in allowed_images'
        }
      }
    }
  },

  // ===================================================================
  // L1 - Input Layer
  // ===================================================================

  input_schema: {
    id: 'input_schema',
    layer: 'L1',
    layer_name: 'Input',
    name: 'Input Schema Validation Rule',
    description: 'Enforce payload schema, data type, and size validation',
    default_priority: 100,
    params_schema: {
      type: 'object',
      properties: {
        schema_ref: {
          type: 'string',
          default: '',
          description: 'JSON schema reference or inline schema'
        },
        payload_dtype: {
          type: 'string',
          enum: ['json', 'xml', 'protobuf', 'msgpack'],
          default: 'json',
          description: 'Expected payload data type'
        },
        max_bytes: {
          type: 'integer',
          default: 16384,
          description: 'Maximum payload size in bytes'
        },
        action: {
          type: 'string',
          enum: ['ALLOW', 'DENY'],
          default: 'DENY',
          description: 'Action on validation failure'
        }
      }
    }
  },

  input_sanitize: {
    id: 'input_sanitize',
    layer: 'L1',
    layer_name: 'Input',
    name: 'Input Sanitization Rule',
    description: 'Strip/normalize/validate input fields',
    default_priority: 100,
    params_schema: {
      type: 'object',
      properties: {
        strip_fields: {
          type: 'array',
          items: { type: 'string' },
          default: [],
          description: 'Fields to strip from input'
        },
        allowed_fields: {
          type: 'array',
          items: { type: 'string' },
          description: 'Whitelist of allowed fields (if set, strip all others)'
        },
        max_depth: {
          type: 'integer',
          default: 10,
          description: 'Maximum nesting depth'
        },
        normalize_unicode: {
          type: 'boolean',
          default: true,
          description: 'Normalize Unicode characters'
        },
        action: {
          type: 'string',
          enum: ['REWRITE', 'DENY'],
          default: 'REWRITE',
          description: 'Action to take on sanitization'
        }
      }
    }
  },

  // ===================================================================
  // L2 - Planner Layer
  // ===================================================================

  prompt_assembly: {
    id: 'prompt_assembly',
    layer: 'L2',
    layer_name: 'Planner',
    name: 'Prompt Assembly Rule',
    description: 'Restrict which context sources can be used during prompt building',
    default_priority: 100,
    params_schema: {
      type: 'object',
      properties: {
        allowed_context_ids: {
          type: 'array',
          items: { type: 'string' },
          default: [],
          description: 'Whitelist of allowed context source IDs'
        },
        enforce_provenance: {
          type: 'boolean',
          default: true,
          description: 'Require provenance tracking for all context'
        },
        max_prompt_tokens: {
          type: 'integer',
          default: 8192,
          description: 'Maximum total prompt tokens'
        },
        action: {
          type: 'string',
          enum: ['DROP_CONTEXT', 'DENY'],
          default: 'DROP_CONTEXT',
          description: 'Action when non-whitelisted context detected'
        }
      }
    }
  },

  prompt_length: {
    id: 'prompt_length',
    layer: 'L2',
    layer_name: 'Planner',
    name: 'Prompt Length Rule',
    description: 'Prevent runaway token counts',
    default_priority: 100,
    params_schema: {
      type: 'object',
      properties: {
        max_prompt_tokens: {
          type: 'integer',
          default: 8192,
          description: 'Maximum prompt tokens'
        },
        action_on_violation: {
          type: 'string',
          enum: ['TRUNCATE', 'DENY'],
          default: 'TRUNCATE',
          description: 'Action when limit exceeded'
        },
        truncate_strategy: {
          type: 'string',
          enum: ['HEAD', 'TAIL', 'MIDDLE'],
          default: 'TAIL',
          description: 'Where to truncate from'
        }
      }
    }
  },

  // ===================================================================
  // L3 - Model I/O Layer
  // ===================================================================

  model_output_scan: {
    id: 'model_output_scan',
    layer: 'L3',
    layer_name: 'Model I/O',
    name: 'Model Output Scanning Rule',
    description: 'Scan model output for PII, jailbreaks, sensitive content',
    default_priority: 100,
    params_schema: {
      type: 'object',
      properties: {
        semantic_hook: {
          type: 'string',
          default: 'pii-detector-v1',
          description: 'Semantic hook endpoint for scanning'
        },
        max_exec_ms: {
          type: 'integer',
          default: 40,
          description: 'Maximum execution time in milliseconds'
        },
        action: {
          type: 'string',
          enum: ['REDACT', 'DENY', 'ESCALATE'],
          default: 'REDACT',
          description: 'Action on detection'
        },
        redact_template: {
          type: 'string',
          default: '[REDACTED]',
          description: 'Template for redaction'
        },
        escalate_target: {
          type: 'string',
          description: 'Escalation target (for action=ESCALATE)'
        },
        semantic_checks: {
          type: 'array',
          items: { type: 'string' },
          default: ['pii_detect', 'action_intent_match', 'out_of_scope_detector'],
          description: 'List of semantic checks to run'
        },
        intent: {
          type: 'string',
          description: 'Expected intent for intent matching'
        },
        primary_domain: {
          type: 'string',
          description: 'Primary domain for scope checking'
        }
      }
    }
  },

  model_output_escalate: {
    id: 'model_output_escalate',
    layer: 'L3',
    layer_name: 'Model I/O',
    name: 'Model Output Escalation Rule',
    description: 'Divert uncertain responses to human review',
    default_priority: 100,
    params_schema: {
      type: 'object',
      properties: {
        confidence_threshold: {
          type: 'number',
          default: 0.75,
          description: 'Confidence threshold (0.0-1.0)'
        },
        escalate_target: {
          type: 'string',
          default: 'human-review',
          description: 'Escalation target endpoint'
        },
        semantic_hook: {
          type: 'string',
          description: 'Optional semantic hook for confidence scoring'
        },
        max_exec_ms: {
          type: 'integer',
          default: 40,
          description: 'Maximum execution time'
        }
      }
    }
  },

  // ===================================================================
  // L4 - Tool Gateway Layer (MOST USED)
  // ===================================================================

  tool_whitelist: {
    id: 'tool_whitelist',
    layer: 'L4',
    layer_name: 'Tool Gateway',
    name: 'Tool Whitelist Rule',
    description: 'Allow only specific tools for an agent',
    default_priority: 100,
    params_schema: {
      type: 'object',
      properties: {
        allowed_tool_ids: {
          type: 'array',
          items: { type: 'string' },
          default: [],
          description: 'Whitelist of allowed tool IDs (e.g., ["web_search", "calculator"])'
        },
        allowed_methods: {
          type: 'array',
          items: { type: 'string' },
          description: 'Whitelist of allowed methods (optional)'
        },
        rate_limit_per_min: {
          type: 'integer',
          description: 'Rate limit per minute'
        },
        action: {
          type: 'string',
          enum: ['ALLOW', 'DENY'],
          default: 'DENY',
          description: 'Action for non-whitelisted tools'
        }
      }
    }
  },

  tool_param_constraint: {
    id: 'tool_param_constraint',
    layer: 'L4',
    layer_name: 'Tool Gateway',
    name: 'Tool Parameter Constraint Rule',
    description: 'Enforce parameter types, value bounds, and patterns',
    default_priority: 100,
    params_schema: {
      type: 'object',
      properties: {
        tool_id: {
          type: 'string',
          description: 'Tool ID to constrain (optional, applies to all if not set)'
        },
        param_name: {
          type: 'string',
          default: '*',
          description: 'Parameter name (* for all params)'
        },
        param_type: {
          type: 'string',
          enum: ['string', 'number', 'boolean', 'object', 'array'],
          default: 'string',
          description: 'Expected parameter type'
        },
        regex: {
          type: 'string',
          description: 'Regex pattern for string params'
        },
        allowed_values: {
          type: 'array',
          items: { type: 'string' },
          description: 'Whitelist of allowed values'
        },
        max_len: {
          type: 'integer',
          description: 'Maximum string length'
        },
        enforcement_mode: {
          type: 'string',
          enum: ['HARD', 'SOFT'],
          default: 'HARD',
          description: 'Hard=block, Soft=log and allow'
        },
        allowed_methods: {
          type: 'array',
          items: { type: 'string' },
          default: [],
          description: 'Methods this constraint applies to'
        }
      }
    }
  },

  // ===================================================================
  // L5 - RAG Layer
  // ===================================================================

  rag_source: {
    id: 'rag_source',
    layer: 'L5',
    layer_name: 'RAG',
    name: 'RAG Source Restriction Rule',
    description: 'Restrict retrieval to specific sources or indices',
    default_priority: 100,
    params_schema: {
      type: 'object',
      properties: {
        allowed_sources: {
          type: 'array',
          items: { type: 'string' },
          default: [],
          description: 'Whitelist of allowed RAG sources'
        },
        max_docs: {
          type: 'integer',
          default: 5,
          description: 'Maximum documents to retrieve'
        },
        max_tokens_per_doc: {
          type: 'integer',
          description: 'Maximum tokens per document'
        },
        action: {
          type: 'string',
          enum: ['ALLOW', 'DENY'],
          default: 'DENY',
          description: 'Action for non-whitelisted sources'
        }
      }
    }
  },

  rag_doc_sensitivity: {
    id: 'rag_doc_sensitivity',
    layer: 'L5',
    layer_name: 'RAG',
    name: 'RAG Document Sensitivity Rule',
    description: 'Block sensitive or classified documents',
    default_priority: 100,
    params_schema: {
      type: 'object',
      properties: {
        semantic_hook: {
          type: 'string',
          default: 'sensitivity-classifier-v1',
          description: 'Semantic hook for sensitivity classification'
        },
        max_sensitivity_level: {
          type: 'string',
          enum: ['public', 'internal', 'confidential', 'secret'],
          default: 'internal',
          description: 'Maximum allowed sensitivity level'
        },
        blocked_classifications: {
          type: 'array',
          items: { type: 'string' },
          default: ['PII', 'SSN', 'CREDIT_CARD'],
          description: 'Classifications to block'
        },
        action: {
          type: 'string',
          enum: ['DENY', 'ESCALATE'],
          default: 'DENY',
          description: 'Action on detection'
        },
        escalate_target: {
          type: 'string',
          description: 'Escalation target (for action=ESCALATE)'
        },
        max_exec_ms: {
          type: 'integer',
          default: 40,
          description: 'Maximum execution time'
        }
      }
    }
  },

  // ===================================================================
  // L6 - Egress Layer
  // ===================================================================

  output_pii: {
    id: 'output_pii',
    layer: 'L6',
    layer_name: 'Egress',
    name: 'Output PII Detection Rule',
    description: 'Detect and redact/deny PII before response leaves system',
    default_priority: 100,
    params_schema: {
      type: 'object',
      properties: {
        semantic_hook: {
          type: 'string',
          default: 'pii-detector-v1',
          description: 'Semantic hook for PII detection'
        },
        action: {
          type: 'string',
          enum: ['REDACT', 'DENY'],
          default: 'REDACT',
          description: 'Action on PII detection'
        },
        redact_template: {
          type: 'string',
          default: '[REDACTED]',
          description: 'Template for redaction'
        },
        pii_types: {
          type: 'array',
          items: { type: 'string' },
          default: ['PII', 'SSN', 'CREDIT_CARD'],
          description: 'Types of PII to detect'
        },
        max_exec_ms: {
          type: 'integer',
          default: 40,
          description: 'Maximum execution time'
        },
        confidence_threshold: {
          type: 'number',
          default: 0.6,
          description: 'Confidence threshold for detection'
        }
      }
    }
  },

  output_audit: {
    id: 'output_audit',
    layer: 'L6',
    layer_name: 'Egress',
    name: 'Output Audit Rule',
    description: 'Log all decisions and outputs for compliance',
    default_priority: 100,
    params_schema: {
      type: 'object',
      properties: {
        emit_decision_event: {
          type: 'boolean',
          default: true,
          description: 'Emit decision events to telemetry'
        },
        sampling_rate: {
          type: 'number',
          default: 1.0,
          description: 'Sampling rate (0.0-1.0, 1.0=log all)'
        },
        include_payload: {
          type: 'boolean',
          default: false,
          description: 'Include full payload in audit log'
        },
        include_pii: {
          type: 'boolean',
          default: false,
          description: 'Include PII in audit log (use with caution)'
        },
        audit_targets: {
          type: 'array',
          items: { type: 'string' },
          default: [],
          description: 'Audit target endpoints'
        }
      }
    }
  }
};

/**
 * Get all rule families for a specific layer
 */
export function getRuleFamiliesByLayer(layer: string): RuleFamilyMetadata[] {
  return Object.values(RULE_FAMILIES_CATALOG).filter(f => f.layer === layer);
}

/**
 * Get layer description
 */
export function getLayerDescription(layer: string): string {
  const descriptions: Record<string, string> = {
    L0: 'System Layer - Network egress, sidecar spawning',
    L1: 'Input Layer - Schema validation, input sanitization',
    L2: 'Planner Layer - Prompt assembly, length limits',
    L3: 'Model I/O Layer - Output scanning, escalation',
    L4: 'Tool Gateway - Tool whitelisting, parameter constraints',
    L5: 'RAG Layer - Source restrictions, document sensitivity',
    L6: 'Egress Layer - PII detection, output auditing'
  };
  return descriptions[layer] || 'Unknown layer';
}
```

**Step 4: Run test to verify it passes**

```bash
npm test tests/tupl/rule-families.test.ts
```

Expected: PASS (4 tests)

**Step 5: Commit rule families catalog**

```bash
git add src/tupl/schemas/rule-families.ts tests/tupl/rule-families.test.ts
git commit -m "feat(tupl): add complete rule families catalog (L0-L6, 14 families)"
```

---

## Task 4: Implement wrap_agent Tool

**Files:**
- Create: `mcp-gateway/src/tupl/tools/wrap-agent.ts`
- Test: `mcp-gateway/tests/tupl/wrap-agent.test.ts`

**Step 1: Write failing test**

File: `mcp-gateway/tests/tupl/wrap-agent.test.ts`

```typescript
import { describe, it, expect } from '@jest/globals';
import { wrapAgent } from '../../src/tupl/tools/wrap-agent';

describe('wrapAgent', () => {
  it('should generate wrapper code with default params', async () => {
    const result = await wrapAgent({});

    expect(result.code).toContain('from tupl.agent import enforcement_agent');
    expect(result.code).toContain('secure_agent');
    expect(result.code).toContain('boundary_id="all"');
    expect(result.instructions).toBeDefined();
    expect(result.next_steps).toBeDefined();
  });

  it('should use custom agent variable name', async () => {
    const result = await wrapAgent({ agent_variable_name: 'my_agent' });

    expect(result.code).toContain('secure_my_agent');
    expect(result.code).toContain('my_agent,');
  });

  it('should use custom boundary and tenant', async () => {
    const result = await wrapAgent({
      boundary_id: 'bnd-123',
      tenant_id: 'acme-corp'
    });

    expect(result.code).toContain('boundary_id="bnd-123"');
    expect(result.code).toContain('tenant_id="acme-corp"');
  });
});
```

**Step 2: Run test to verify it fails**

```bash
npm test tests/tupl/wrap-agent.test.ts
```

Expected: FAIL with "wrapAgent is not defined"

**Step 3: Implement wrapAgent tool**

File: `mcp-gateway/src/tupl/tools/wrap-agent.ts`

```typescript
/**
 * Generate code snippet to wrap a LangGraph agent with enforcement_agent()
 *
 * This is the PRIMARY tool users will use to add Tupl security to their agents.
 */

export interface WrapAgentArgs {
  agent_variable_name?: string;
  boundary_id?: string;
  tenant_id?: string;
  base_url?: string;
}

export interface WrapAgentResult {
  code: string;
  instructions: string[];
  next_steps: {
    configure_rules: string;
    list_families: string;
    test: string;
  };
}

export async function wrapAgent(args: WrapAgentArgs): Promise<WrapAgentResult> {
  const agentVar = args.agent_variable_name || 'agent';
  const boundaryId = args.boundary_id || 'all';
  const tenantId = args.tenant_id || process.env.TUPL_TENANT_ID || 'demo-tenant';
  const baseUrl = args.base_url || process.env.TUPL_BASE_URL || 'http://localhost:8000';

  // Generate the wrapper code
  const code = `
# Add to your imports
from tupl.agent import enforcement_agent

# Wrap your agent (add after you create your agent)
secure_${agentVar} = enforcement_agent(
    ${agentVar},
    boundary_id="${boundaryId}",
    base_url="${baseUrl}",
    tenant_id="${tenantId}"
)

# Use secure_${agentVar} instead of ${agentVar}
result = secure_${agentVar}.invoke({"messages": [...]})
`.trim();

  return {
    code,
    instructions: [
      '1. Copy the import statement to the top of your file',
      '2. Add the enforcement_agent() wrapper after creating your agent',
      '3. Use the secure_agent variable for all invocations',
      '4. Configure rule families in the Control Plane UI console'
    ],
    next_steps: {
      configure_rules: 'Use Control Plane UI to configure rule families for this agent',
      list_families: 'Call list_rule_families to see available rule families',
      test: 'Run your agent and check telemetry with get_telemetry'
    }
  };
}
```

**Step 4: Run test to verify it passes**

```bash
npm test tests/tupl/wrap-agent.test.ts
```

Expected: PASS (3 tests)

**Step 5: Commit wrap-agent tool**

```bash
git add src/tupl/tools/wrap-agent.ts tests/tupl/wrap-agent.test.ts
git commit -m "feat(tupl): add wrap_agent tool for generating enforcement code"
```

---

## Task 5: Implement list_rule_families Tool

**Files:**
- Create: `mcp-gateway/src/tupl/tools/list-families.ts`
- Test: `mcp-gateway/tests/tupl/list-families.test.ts`

**Step 1: Write failing test**

File: `mcp-gateway/tests/tupl/list-families.test.ts`

```typescript
import { describe, it, expect } from '@jest/globals';
import { listRuleFamilies } from '../../src/tupl/tools/list-families';

describe('listRuleFamilies', () => {
  it('should return all 14 families without filter', async () => {
    const result = await listRuleFamilies({});

    expect(result.total).toBe(14);
    expect(result.families).toHaveLength(14);
    expect(result.layers).toBeDefined();
  });

  it('should filter by layer L4', async () => {
    const result = await listRuleFamilies({ layer: 'L4' });

    expect(result.total).toBe(2);
    expect(result.families).toHaveLength(2);
    expect(result.families.every(f => f.layer === 'L4')).toBe(true);
  });

  it('should include parameter information', async () => {
    const result = await listRuleFamilies({ layer: 'L4' });
    const toolWhitelist = result.families.find(f => f.id === 'tool_whitelist');

    expect(toolWhitelist).toBeDefined();
    expect(toolWhitelist!.parameters).toContain('allowed_tool_ids');
  });
});
```

**Step 2: Run test to verify it fails**

```bash
npm test tests/tupl/list-families.test.ts
```

Expected: FAIL with "listRuleFamilies is not defined"

**Step 3: Implement listRuleFamilies tool**

File: `mcp-gateway/src/tupl/tools/list-families.ts`

```typescript
import { RULE_FAMILIES_CATALOG, getLayerDescription } from '../schemas/rule-families';

export interface ListRuleFamiliesArgs {
  layer?: 'L0' | 'L1' | 'L2' | 'L3' | 'L4' | 'L5' | 'L6';
}

export interface RuleFamilySummary {
  id: string;
  layer: string;
  layer_name: string;
  name: string;
  description: string;
  parameters: string[];
  required: string[];
}

export interface ListRuleFamiliesResult {
  total: number;
  families: RuleFamilySummary[];
  layers: Record<string, string>;
}

export async function listRuleFamilies(
  args: ListRuleFamiliesArgs
): Promise<ListRuleFamiliesResult> {
  let families = Object.values(RULE_FAMILIES_CATALOG);

  // Filter by layer if specified
  if (args.layer) {
    families = families.filter(f => f.layer === args.layer);
  }

  // Map to summary format
  const familySummaries: RuleFamilySummary[] = families.map(f => ({
    id: f.id,
    layer: f.layer,
    layer_name: f.layer_name,
    name: f.name,
    description: f.description,
    parameters: Object.keys(f.params_schema.properties),
    required: f.params_schema.required || []
  }));

  return {
    total: familySummaries.length,
    families: familySummaries,
    layers: {
      L0: getLayerDescription('L0'),
      L1: getLayerDescription('L1'),
      L2: getLayerDescription('L2'),
      L3: getLayerDescription('L3'),
      L4: getLayerDescription('L4'),
      L5: getLayerDescription('L5'),
      L6: getLayerDescription('L6')
    }
  };
}
```

**Step 4: Run test to verify it passes**

```bash
npm test tests/tupl/list-families.test.ts
```

Expected: PASS (3 tests)

**Step 5: Commit list-families tool**

```bash
git add src/tupl/tools/list-families.ts tests/tupl/list-families.test.ts
git commit -m "feat(tupl): add list_rule_families tool"
```

---

## Task 6: Implement get_telemetry Tool

**Files:**
- Create: `mcp-gateway/src/tupl/tools/telemetry.ts`
- Test: `mcp-gateway/tests/tupl/telemetry.test.ts`

**Step 1: Write failing test**

File: `mcp-gateway/tests/tupl/telemetry.test.ts`

```typescript
import { describe, it, expect, beforeEach } from '@jest/globals';
import { getTelemetry } from '../../src/tupl/tools/telemetry';
import { ManagementPlaneClient } from '../../src/tupl/clients/management-plane';
import nock from 'nock';

describe('getTelemetry', () => {
  let client: ManagementPlaneClient;
  const baseURL = 'http://localhost:8000';

  beforeEach(() => {
    client = new ManagementPlaneClient(baseURL);
  });

  it('should fetch and format telemetry', async () => {
    nock(baseURL)
      .get('/api/v1/telemetry')
      .query(true)
      .reply(200, {
        total: 10,
        events: [
          { id: 'evt-1', decision: 1, timestamp: Date.now() },
          { id: 'evt-2', decision: 0, timestamp: Date.now() },
          { id: 'evt-3', decision: 1, timestamp: Date.now() }
        ]
      });

    const result = await getTelemetry(client, { limit: 100 });

    expect(result.total).toBe(10);
    expect(result.events).toHaveLength(3);
    expect(result.summary.allowed).toBe(2);
    expect(result.summary.blocked).toBe(1);
  });

  it('should handle filter params', async () => {
    nock(baseURL)
      .get('/api/v1/telemetry')
      .query({ limit: 50, tenant_id: 'test-tenant', decision: 0 })
      .reply(200, {
        total: 5,
        events: []
      });

    const result = await getTelemetry(client, {
      limit: 50,
      tenant_id: 'test-tenant',
      decision: 'block'
    });

    expect(result.total).toBe(5);
  });
});
```

**Step 2: Run test to verify it fails**

```bash
npm test tests/tupl/telemetry.test.ts
```

Expected: FAIL with "getTelemetry is not defined"

**Step 3: Implement getTelemetry tool**

File: `mcp-gateway/src/tupl/tools/telemetry.ts`

```typescript
import { ManagementPlaneClient } from '../clients/management-plane';
import { TelemetryResponse } from '../schemas/types';

export interface GetTelemetryArgs {
  limit?: number;
  tenant_id?: string;
  agent_id?: string;
  decision?: 'allow' | 'block';
}

export async function getTelemetry(
  client: ManagementPlaneClient,
  args: GetTelemetryArgs
): Promise<TelemetryResponse> {
  const telemetry = await client.getTelemetry(args);

  // Calculate summary stats
  const events = telemetry.events || [];
  const allowed = events.filter((e: any) => e.decision === 1).length;
  const blocked = events.filter((e: any) => e.decision === 0).length;

  return {
    total: telemetry.total || 0,
    limit: args.limit || 100,
    events,
    summary: {
      allowed,
      blocked
    }
  };
}
```

**Step 4: Run test to verify it passes**

```bash
npm test tests/tupl/telemetry.test.ts
```

Expected: PASS (2 tests)

**Step 5: Commit telemetry tool**

```bash
git add src/tupl/tools/telemetry.ts tests/tupl/telemetry.test.ts
git commit -m "feat(tupl): add get_telemetry tool"
```

---

## Task 7: Implement get_agent_config Tool

**Files:**
- Create: `mcp-gateway/src/tupl/tools/get-config.ts`
- Test: `mcp-gateway/tests/tupl/get-config.test.ts`

**Step 1: Write failing test**

File: `mcp-gateway/tests/tupl/get-config.test.ts`

```typescript
import { describe, it, expect, beforeEach } from '@jest/globals';
import { getAgentConfig } from '../../src/tupl/tools/get-config';
import { ManagementPlaneClient } from '../../src/tupl/clients/management-plane';
import nock from 'nock';

describe('getAgentConfig', () => {
  let client: ManagementPlaneClient;
  const baseURL = 'http://localhost:8000';

  beforeEach(() => {
    client = new ManagementPlaneClient(baseURL);
  });

  it('should fetch and format agent config', async () => {
    nock(baseURL)
      .get('/api/v1/boundaries')
      .query({ agent_id: 'test-agent' })
      .reply(200, [
        {
          id: 'bnd-1',
          name: 'Allow Reads',
          status: 'active',
          rules: { effect: 'allow' },
          constraints: {
            action: { actions: ['read'] }
          }
        },
        {
          id: 'bnd-2',
          name: 'Block Deletes',
          status: 'active',
          rules: { effect: 'deny' },
          constraints: {
            action: { actions: ['delete'] }
          }
        }
      ]);

    const result = await getAgentConfig(client, { agent_id: 'test-agent' });

    expect(result.agent_id).toBe('test-agent');
    expect(result.boundaries).toHaveLength(2);
    expect(result.boundaries[0].effect).toBe('allow');
    expect(result.note).toContain('Control Plane UI');
  });
});
```

**Step 2: Run test to verify it fails**

```bash
npm test tests/tupl/get-config.test.ts
```

Expected: FAIL with "getAgentConfig is not defined"

**Step 3: Implement getAgentConfig tool**

File: `mcp-gateway/src/tupl/tools/get-config.ts`

```typescript
import { ManagementPlaneClient } from '../clients/management-plane';
import { AgentConfigResponse, BoundaryInfo } from '../schemas/types';

export interface GetAgentConfigArgs {
  agent_id: string;
}

/**
 * Extract layers referenced in boundary constraints
 */
function extractLayersFromBoundary(boundary: any): string[] {
  const layers: Set<string> = new Set();

  // Check if boundary has layer-specific constraints
  if (boundary.constraints) {
    // This is simplified - in reality would parse constraints more deeply
    if (boundary.constraints.action) layers.add('L4'); // Actions typically L4
    if (boundary.constraints.resource) layers.add('L0'); // Resources often L0
  }

  return Array.from(layers);
}

export async function getAgentConfig(
  client: ManagementPlaneClient,
  args: GetAgentConfigArgs
): Promise<AgentConfigResponse> {
  // Fetch boundaries for this agent from Management Plane
  const boundaries = await client.getBoundaries({
    agent_id: args.agent_id
  });

  // Map to summary format
  const boundaryInfos: BoundaryInfo[] = boundaries.map((b: any) => ({
    id: b.id,
    name: b.name,
    status: b.status,
    effect: b.rules?.effect || 'allow',
    layers: extractLayersFromBoundary(b)
  }));

  return {
    agent_id: args.agent_id,
    boundaries: boundaryInfos,
    note: 'Configure rule families in the Control Plane UI console'
  };
}
```

**Step 4: Run test to verify it passes**

```bash
npm test tests/tupl/get-config.test.ts
```

Expected: PASS (1 test)

**Step 5: Commit get-config tool**

```bash
git add src/tupl/tools/get-config.ts tests/tupl/get-config.test.ts
git commit -m "feat(tupl): add get_agent_config tool"
```

---

## Task 8: Register Tupl Tools in Gateway

**Files:**
- Modify: `mcp-gateway/src/mcp-server/server.ts`
- Create: `mcp-gateway/src/mcp-server/tupl-tools.ts`
- Modify: `mcp-gateway/src/gateway.ts`

**Step 1: Create tool definitions file**

File: `mcp-gateway/src/mcp-server/tupl-tools.ts`

```typescript
import { Tool } from '@modelcontextprotocol/sdk/types.js';

/**
 * Tupl security enforcement tool definitions for MCP
 */

export const TUPL_WRAP_AGENT: Tool = {
  name: 'wrap_agent',
  description: 'Generate code to wrap your LangGraph agent with Tupl enforcement. Use this FIRST when adding security policies to your agent.',
  inputSchema: {
    type: 'object',
    properties: {
      agent_variable_name: {
        type: 'string',
        description: 'Variable name of your agent (default: "agent")'
      },
      boundary_id: {
        type: 'string',
        description: 'Boundary ID to check (default: "all" checks all policies)'
      },
      tenant_id: {
        type: 'string',
        description: 'Tenant ID (default: from TUPL_TENANT_ID env var)'
      },
      base_url: {
        type: 'string',
        description: 'Management Plane URL (default: http://localhost:8000)'
      }
    }
  }
};

export const TUPL_LIST_RULE_FAMILIES: Tool = {
  name: 'list_rule_families',
  description: 'Browse all 14 available rule families across L0-L6 security layers. Use this to discover what rules you can configure in the Control Plane.',
  inputSchema: {
    type: 'object',
    properties: {
      layer: {
        type: 'string',
        description: 'Filter by layer',
        enum: ['L0', 'L1', 'L2', 'L3', 'L4', 'L5', 'L6']
      }
    }
  }
};

export const TUPL_GET_TELEMETRY: Tool = {
  name: 'get_telemetry',
  description: 'View enforcement telemetry events to see which operations were allowed or blocked',
  inputSchema: {
    type: 'object',
    properties: {
      limit: {
        type: 'number',
        description: 'Max events to return (default: 100)'
      },
      tenant_id: {
        type: 'string',
        description: 'Filter by tenant ID'
      },
      agent_id: {
        type: 'string',
        description: 'Filter by agent ID'
      },
      decision: {
        type: 'string',
        enum: ['allow', 'block'],
        description: 'Filter by decision type'
      }
    }
  }
};

export const TUPL_GET_AGENT_CONFIG: Tool = {
  name: 'get_agent_config',
  description: 'View configured security policies for a specific agent',
  inputSchema: {
    type: 'object',
    required: ['agent_id'],
    properties: {
      agent_id: {
        type: 'string',
        description: 'Agent identifier'
      }
    }
  }
};

export const TUPL_TOOLS = [
  TUPL_WRAP_AGENT,
  TUPL_LIST_RULE_FAMILIES,
  TUPL_GET_TELEMETRY,
  TUPL_GET_AGENT_CONFIG
] as const;
```

**Step 2: Modify MCPGatewayServer to add Tupl handlers**

File: `mcp-gateway/src/mcp-server/server.ts` (add imports at top)

```typescript
// Add these imports at the top
import { ManagementPlaneClient } from '../tupl/clients/management-plane';
import { wrapAgent } from '../tupl/tools/wrap-agent';
import { listRuleFamilies } from '../tupl/tools/list-families';
import { getTelemetry } from '../tupl/tools/telemetry';
import { getAgentConfig } from '../tupl/tools/get-config';
import { TUPL_TOOLS } from './tupl-tools';
```

File: `mcp-gateway/src/mcp-server/server.ts` (modify constructor)

```typescript
export class MCPGatewayServer {
  // ... existing fields
  private managementPlaneClient: ManagementPlaneClient;

  constructor(
    registry: ToolRegistry,
    runtimeBridge: RuntimeBridge,
    workspace: WorkspaceManager,
    managementPlaneURL?: string
  ) {
    // ... existing constructor code

    // Initialize Management Plane client for Tupl tools
    this.managementPlaneClient = new ManagementPlaneClient(
      managementPlaneURL ||
      process.env.TUPL_BASE_URL ||
      'http://localhost:8000'
    );

    this.setupHandlers();
  }
```

File: `mcp-gateway/src/mcp-server/server.ts` (modify setupHandlers method)

```typescript
private setupHandlers() {
  // ... existing handlers setup

  // Add CallTool handler for Tupl tools
  this.server.setRequestHandler(CallToolRequestSchema, async (request) => {
    const { name, arguments: args } = request.params;

    try {
      // Existing gateway tools
      switch (name) {
        case 'run_code':
          return this.handleRunCode(args);
        case 'list_servers':
          return this.handleListServers();
        case 'search_workspace':
          return this.handleSearchWorkspace(args);
        case 'get_workspace_file':
          return this.handleGetWorkspaceFile(args);

        // Tupl security tools
        case 'wrap_agent':
          return this.handleTuplWrapAgent(args);
        case 'list_rule_families':
          return this.handleTuplListFamilies(args);
        case 'get_telemetry':
          return this.handleTuplGetTelemetry(args);
        case 'get_agent_config':
          return this.handleTuplGetAgentConfig(args);

        default:
          throw new Error(`Unknown tool: ${name}`);
      }
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      return {
        content: [{
          type: 'text' as const,
          text: JSON.stringify({ error: errorMessage }, null, 2)
        }],
        isError: true
      };
    }
  });

  // Update ListTools handler to include Tupl tools
  this.server.setRequestHandler(ListToolsRequestSchema, async () => {
    return {
      tools: [
        // Existing gateway tools
        RUN_CODE_TOOL,
        LIST_SERVERS_TOOL,
        SEARCH_WORKSPACE_TOOL,
        GET_WORKSPACE_FILE_TOOL,

        // Tupl security tools
        ...TUPL_TOOLS
      ]
    };
  });
}
```

File: `mcp-gateway/src/mcp-server/server.ts` (add Tupl handler methods)

```typescript
// Add these methods to MCPGatewayServer class

private async handleTuplWrapAgent(args: any) {
  const result = await wrapAgent(args);
  return {
    content: [{
      type: 'text' as const,
      text: JSON.stringify(result, null, 2)
    }]
  };
}

private async handleTuplListFamilies(args: any) {
  const result = await listRuleFamilies(args);
  return {
    content: [{
      type: 'text' as const,
      text: JSON.stringify(result, null, 2)
    }]
  };
}

private async handleTuplGetTelemetry(args: any) {
  const result = await getTelemetry(this.managementPlaneClient, args);
  return {
    content: [{
      type: 'text' as const,
      text: JSON.stringify(result, null, 2)
    }]
  };
}

private async handleTuplGetAgentConfig(args: any) {
  const result = await getAgentConfig(this.managementPlaneClient, args);
  return {
    content: [{
      type: 'text' as const,
      text: JSON.stringify(result, null, 2)
    }]
  };
}
```

**Step 3: Update Gateway to pass Management Plane URL**

File: `mcp-gateway/src/gateway.ts` (modify constructor)

```typescript
export class Gateway {
  // ... existing code

  constructor(config: GatewayConfig) {
    // ... existing initialization

    // Initialize MCP server with Management Plane URL
    const managementPlaneURL = process.env.TUPL_BASE_URL || 'http://localhost:8000';

    this.mcpServer = new MCPGatewayServer(
      this.registry,
      this.runtimeBridge,
      this.workspace,
      managementPlaneURL
    );
  }
}
```

**Step 4: Run tests**

```bash
npm test
```

Expected: All tests pass

**Step 5: Commit gateway integration**

```bash
git add src/mcp-server/ src/gateway.ts
git commit -m "feat(tupl): integrate Tupl tools into MCP gateway server"
```

---

## Task 9: Add Configuration and Documentation

**Files:**
- Create: `mcp-gateway/.env.example`
- Modify: `mcp-gateway/README.md`
- Modify: `mcp-gateway/package.json`

**Step 1: Create .env.example**

File: `mcp-gateway/.env.example`

```bash
# MCP Gateway Configuration
PORT=3000
CONFIG_PATH=./config.json

# Tupl Security Configuration
TUPL_BASE_URL=http://localhost:8000
TUPL_TENANT_ID=demo-tenant

# Optional: Enable debug logging
DEBUG=tupl:*
```

**Step 2: Update README with Tupl section**

File: `mcp-gateway/README.md` (add before "## Development" section)

```markdown
## Tupl Security Integration

The gateway includes native Tupl security enforcement tools for adding guardrails to your LangGraph agents.

### Available Tools

#### 1. `wrap_agent`
Generate code to wrap your agent with enforcement_agent(). **Use this first!**

**Example:**
```
Call wrap_agent with agent_variable_name="my_agent"
```

Returns Python code to copy into your agent file.

#### 2. `list_rule_families`
Browse 14 available rule families across L0-L6 security layers.

**Example:**
```
Call list_rule_families with layer="L4"
```

Returns tool gateway rules (whitelist, param constraints).

#### 3. `get_telemetry`
View enforcement decisions to see what was allowed or blocked.

**Example:**
```
Call get_telemetry with limit=50, decision="block"
```

#### 4. `get_agent_config`
View configured security policies for a specific agent.

**Example:**
```
Call get_agent_config with agent_id="my-agent"
```

### Quick Start Workflow

1. **List available rule families:**
   ```
   Call list_rule_families
   ```

2. **Generate wrapper code:**
   ```
   Call wrap_agent with agent_variable_name="my_agent"
   ```

3. **Copy code to your agent file:**
   ```python
   from tupl.agent import enforcement_agent

   secure_my_agent = enforcement_agent(
       my_agent,
       boundary_id="all",
       base_url="http://localhost:8000",
       tenant_id="demo-tenant"
   )

   result = secure_my_agent.invoke({"messages": [...]})
   ```

4. **Configure rule families:**
   - Open Control Plane UI console
   - Create boundaries with rule families
   - Assign to your agent

5. **Test and view telemetry:**
   ```
   Call get_telemetry
   ```

### Environment Variables

- `TUPL_BASE_URL` - Management Plane URL (default: http://localhost:8000)
- `TUPL_TENANT_ID` - Your tenant ID (default: demo-tenant)

### Architecture

```
User Code → enforcement_agent() → Management Plane → Data Plane → Enforcement
```

The gateway tools help you set up the enforcement wrapper. The actual enforcement happens at runtime when your agent makes tool calls.

### Rule Families (L0-L6)

- **L0 System**: Network egress, sidecar spawning
- **L1 Input**: Schema validation, sanitization
- **L2 Planner**: Prompt assembly, length limits
- **L3 Model I/O**: Output scanning, escalation
- **L4 Tool Gateway**: Tool whitelist, param constraints (most common)
- **L5 RAG**: Source restrictions, document sensitivity
- **L6 Egress**: PII detection, audit logging

For detailed information on each rule family, use `list_rule_families`.
```

**Step 3: Commit documentation**

```bash
git add .env.example README.md
git commit -m "docs(tupl): add configuration and usage documentation"
```

---

## Task 10: End-to-End Testing

**Files:**
- Create: `mcp-gateway/tests/e2e/tupl-integration.test.ts`
- Create: `mcp-gateway/examples/tupl-demo.ts`

**Step 1: Create E2E test**

File: `mcp-gateway/tests/e2e/tupl-integration.test.ts`

```typescript
import { describe, it, expect, beforeAll, afterAll } from '@jest/globals';
import { Gateway } from '../../src/gateway';
import { ManagementPlaneClient } from '../../src/tupl/clients/management-plane';

describe('Tupl Integration E2E', () => {
  let gateway: Gateway;

  beforeAll(async () => {
    // Start gateway
    gateway = new Gateway({
      configPath: './config.example.json'
    });
    await gateway.initialize();
  });

  afterAll(async () => {
    // Cleanup
  });

  it('should list all Tupl tools', async () => {
    const tools = await gateway.mcpServer.listTools();

    const tuplTools = tools.filter(t => ['wrap_agent', 'list_rule_families', 'get_telemetry', 'get_agent_config'].includes(t.name));
    expect(tuplTools).toHaveLength(4);

    const toolNames = tuplTools.map(t => t.name);
    expect(toolNames).toContain('wrap_agent');
    expect(toolNames).toContain('list_rule_families');
    expect(toolNames).toContain('get_telemetry');
    expect(toolNames).toContain('get_agent_config');
  });

  it('should generate wrapper code', async () => {
    const result = await gateway.mcpServer.callTool('wrap_agent', {
      agent_variable_name: 'test_agent'
    });

    expect(result.content[0].text).toContain('enforcement_agent');
    expect(result.content[0].text).toContain('secure_test_agent');
  });

  it('should list rule families', async () => {
    const result = await gateway.mcpServer.callTool('list_rule_families', {});

    const data = JSON.parse(result.content[0].text);
    expect(data.total).toBe(14);
    expect(data.families).toHaveLength(14);
  });

  it('should filter rule families by layer', async () => {
    const result = await gateway.mcpServer.callTool('list_rule_families', {
      layer: 'L4'
    });

    const data = JSON.parse(result.content[0].text);
    expect(data.total).toBe(2);
    expect(data.families.map((f: any) => f.id)).toContain('tool_whitelist');
  });
});
```

**Step 2: Create example usage demo**

File: `mcp-gateway/examples/tupl-demo.ts`

```typescript
/**
 * Tupl Integration Demo
 *
 * Shows how to use Tupl security tools via mcp-gateway.
 */

async function demo() {
  console.log('=== Tupl Security Tools Demo ===\n');

  // Step 1: List available rule families
  console.log('Step 1: Listing available rule families...');
  const families = await callMCPTool('fencio-dev', 'list_rule_families', {
    layer: 'L4'  // Tool Gateway layer
  });
  console.log(`Found ${families.total} L4 rule families:`);
  families.families.forEach((f: any) => {
    console.log(`  - ${f.id}: ${f.description}`);
  });
  console.log();

  // Step 2: Generate wrapper code
  console.log('Step 2: Generating enforcement wrapper code...');
  const wrapper = await callMCPTool('fencio-dev', 'wrap_agent', {
    agent_variable_name: 'customer_support_agent',
    boundary_id: 'support-policies',
    tenant_id: 'acme-corp'
  });
  console.log('Code to add to your agent:');
  console.log(wrapper.code);
  console.log();

  // Step 3: View instructions
  console.log('Step 3: Next steps:');
  wrapper.instructions.forEach((inst: string) => console.log(`  ${inst}`));
  console.log();

  // Step 4: View telemetry (after agent runs)
  console.log('Step 4: Checking enforcement telemetry...');
  const telemetry = await callMCPTool('fencio-dev', 'get_telemetry', {
    limit: 10,
    tenant_id: 'acme-corp'
  });
  console.log(`Total events: ${telemetry.total}`);
  console.log(`  Allowed: ${telemetry.summary.allowed}`);
  console.log(`  Blocked: ${telemetry.summary.blocked}`);
  console.log();

  console.log('=== Demo Complete ===');
}

// Mock callMCPTool for example purposes
async function callMCPTool(server: string, tool: string, args: any) {
  // In real usage, this would call via MCP client
  console.log(`[Mock] Calling ${server}.${tool}...`);
  return {};
}

if (require.main === module) {
  demo().catch(console.error);
}
```

**Step 3: Run E2E tests**

```bash
npm test tests/e2e/tupl-integration.test.ts
```

Expected: PASS (4 tests)

**Step 4: Commit E2E tests and examples**

```bash
git add tests/e2e/ examples/
git commit -m "test(tupl): add end-to-end integration tests and demo"
```

---

## Task 11: Archive mcp-tupl-server

**Files:**
- Modify: `mcp-tupl-server/README.md`

**Step 1: Add deprecation notice to mcp-tupl-server README**

File: `mcp-tupl-server/README.md` (add at top)

```markdown
# ⚠️ ARCHIVED - Functionality Integrated into mcp-gateway

> **This repository is archived.** All functionality has been integrated into [mcp-gateway](../mcp-gateway).

## Migration Guide

All tools are now available as native TypeScript tools in mcp-gateway:

| Old Tool (mcp-tupl-server) | New Tool (mcp-gateway) | Status |
|----------------------------|------------------------|--------|
| tupl_generate_rules_from_nlp | *Use Control Plane UI* | Deprecated |
| tupl_configure_agent_rules | *Use Control Plane UI* | Deprecated |
| tupl_get_agent_rules | get_agent_config | ✅ Migrated |
| tupl_list_rule_families | list_rule_families | ✅ Migrated |
| tupl_get_telemetry | get_telemetry | ✅ Migrated |
| *(New)* | wrap_agent | ✅ New Tool |

### Why the Change?

1. **Simpler Setup**: No separate Python server to run
2. **Better Integration**: Native TypeScript tools in gateway
3. **Reduced Token Usage**: Gateway sandbox reduces context bloat
4. **Unified Config**: Configure all MCP servers in one place

### Migration Steps

1. **Update MCP Config** (e.g., Claude Desktop config):

   **Old:**
   ```json
   {
     "mcpServers": {
       "tupl": {
         "command": "uvx",
         "args": ["--from", "mcp-tupl-server", "mcp-tupl-server"],
         "env": {
           "CONTROL_PLANE_URL": "http://localhost:8000"
         }
       }
     }
   }
   ```

   **New:**
   ```json
   {
     "mcpServers": {
       "fencio-dev": {
         "command": "node",
         "args": ["/path/to/mcp-gateway/dist/index.js"],
         "env": {
           "TUPL_BASE_URL": "http://localhost:8000",
           "TUPL_TENANT_ID": "your-tenant"
         }
       }
     }
   }
   ```

2. **Use New Workflow**:
   - Call `wrap_agent` to generate enforcement code
   - Configure rule families in Control Plane UI console
   - Use `get_telemetry` for observability

3. **Remove Python Server**: Uninstall mcp-tupl-server

   ```bash
   pip uninstall mcp-tupl-server
   ```

### Need Help?

See [mcp-gateway Tupl documentation](../mcp-gateway/README.md#tupl-security-integration) for complete setup instructions.

---

**Below is the archived README for historical reference.**

---
```

**Step 2: Commit deprecation notice**

```bash
cd mcp-tupl-server
git add README.md
git commit -m "docs: mark repository as archived, add migration guide"
cd ..
```

**Step 3: Mark repository as archived (manual GitHub step)**

*Note: This requires GitHub repository settings access*

1. Go to repository settings
2. Scroll to "Danger Zone"
3. Click "Archive this repository"
4. Confirm archival

---

## Task 12: Final Integration Testing

**Step 1: Build gateway**

```bash
cd mcp-gateway
npm run build
```

Expected: Build completes successfully

**Step 2: Test with Claude Desktop config**

Create test config file: `mcp-gateway-test-config.json`

```json
{
  "mcpServers": {
    "fencio-dev": {
      "command": "node",
      "args": ["./dist/index.js"],
      "env": {
        "TUPL_BASE_URL": "http://localhost:8000",
        "TUPL_TENANT_ID": "test-tenant"
      }
    }
  }
}
```

**Step 3: Manual test in Claude Desktop**

1. Start Management Plane: `cd management-plane && ./run.sh`
2. Start gateway: `cd mcp-gateway && npm start`
3. Open Claude Desktop with test config
4. Test each tool:
   - `list_rule_families` → should return 14 families
   - `wrap_agent` → should generate Python code
   - `get_telemetry` → should fetch from Management Plane
   - `get_agent_config` → should show boundaries

**Step 4: Verify all tests pass**

```bash
npm test
```

Expected: All unit, integration, and E2E tests pass

**Step 5: Final commit**

```bash
git add .
git commit -m "feat(tupl): complete integration - all tools working"
```

---

## Summary Checklist

- [x] Task 1: Module structure created
- [x] Task 2: Management Plane HTTP client implemented
- [x] Task 3: Rule families catalog ported (14 families, L0-L6)
- [x] Task 4: wrap_agent tool implemented
- [x] Task 5: list_rule_families tool implemented
- [x] Task 6: get_telemetry tool implemented
- [x] Task 7: get_agent_config tool implemented
- [x] Task 8: Tools registered in gateway
- [x] Task 9: Configuration and documentation added
- [x] Task 10: E2E tests passing
- [x] Task 11: mcp-tupl-server archived
- [x] Task 12: Integration verified

---

## Success Criteria

✅ **All 4 Tupl tools available in mcp-gateway**
✅ **Tools callable via MCP protocol**
✅ **Management Plane HTTP client working**
✅ **All 14 rule families ported correctly**
✅ **All tests passing (unit + integration + E2E)**
✅ **Documentation complete and accurate**
✅ **mcp-tupl-server archived with migration guide**

---

## Rollback Plan

If issues arise during deployment:

1. **Revert gateway changes:**
   ```bash
   git revert HEAD~12..HEAD
   ```

2. **Restore mcp-tupl-server:**
   - Un-archive repository
   - Update Claude Desktop config to use old server

3. **Debug issues:**
   - Check Management Plane connectivity
   - Verify tool schemas match
   - Review test failures

---

## Next Steps After Completion

1. **Monitor telemetry** for tool usage patterns
2. **Consider adding NLP tool** if users request it (6 hour effort)
3. **Add boundary CRUD tools** if UI isn't sufficient
4. **Performance optimization** if needed
5. **Integration with Control Plane UI** for seamless config

---

**Estimated Total Effort:** 10-15 hours
**Complexity:** Low-Medium
**Risk:** Low (all functionality tested, rollback available)
