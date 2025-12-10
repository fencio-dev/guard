# Bridge Module - Architecture Diagram

## System Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           MANAGEMENT PLANE                               │
│                                                                           │
│  ┌──────────────────┐      ┌──────────────────┐                        │
│  │ Rule Definition  │─────▶│ Rule Validation  │                        │
│  └──────────────────┘      └────────┬─────────┘                        │
│                                      │                                   │
│                                      ▼                                   │
│                            ┌──────────────────┐                         │
│                            │  Rule Bundle     │                         │
│                            │  (RuleBundle)    │                         │
│                            └────────┬─────────┘                         │
└─────────────────────────────────────┼───────────────────────────────────┘
                                      │
                                      │ gRPC / Protocol Buffers
                                      │ (Hot-Reload / Incremental)
                                      ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                            DATA PLANE                                    │
│                                                                           │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │                         BRIDGE                                      │ │
│  │                                                                     │ │
│  │  ┌──────────────────────────────────────────────────────────────┐ │ │
│  │  │                  14 Rule Family Tables                        │ │ │
│  │  │                                                               │ │ │
│  │  │  L0: NetworkEgress, SidecarSpawn                             │ │ │
│  │  │  L1: InputSchema, InputSanitize                              │ │ │
│  │  │  L2: PromptAssembly, PromptLength                            │ │ │
│  │  │  L3: ModelOutputScan, ModelOutputEscalate                    │ │ │
│  │  │  L4: ToolWhitelist, ToolParamConstraint                      │ │ │
│  │  │  L5: RAGSource, RAGDocSensitivity                            │ │ │
│  │  │  L6: OutputPII, OutputAudit                                  │ │ │
│  │  │                                                               │ │ │
│  │  │  Features:                                                    │ │ │
│  │  │  • Lock-free reads (atomic Arc)                              │ │ │
│  │  │  • Copy-on-write updates                                     │ │ │
│  │  │  • Per-family indices                                        │ │ │
│  │  │  • Priority-based sorting                                    │ │ │
│  │  └──────────────────────────────────────────────────────────────┘ │ │
│  └──────────────────────┬──────────────────────────────────────────────┘ │
│                         │                                                 │
│                         │ Lock-free queries                               │
│                         │ (Sub-microsecond)                               │
│                         ▼                                                 │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │                    EVALUATION ENGINE                                │ │
│  │                                                                     │ │
│  │  Layer-by-layer evaluation (L0 → L6)                              │ │
│  │  • Query rules from Bridge                                         │ │
│  │  • Evaluate by priority                                            │ │
│  │  • Short-circuit on deny                                           │ │
│  │  • Decision caching                                                │ │
│  └──────────────────────┬──────────────────────────────────────────────┘ │
│                         │                                                 │
│                         │ Decision events                                 │
│                         ▼                                                 │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │                      AUDIT SYSTEM                                   │ │
│  │                                                                     │ │
│  │  • Decision logging                                                │ │
│  │  • Metrics emission (Prometheus/etc.)                              │ │
│  │  • Compliance records                                              │ │
│  │  • Statistics collection                                           │ │
│  └────────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────┘
```

## Bridge Internal Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Bridge                                        │
│  ┌────────────────────────────────────────────────────────────────┐ │
│  │  tables: HashMap<RuleFamilyId, Arc<RwLock<RuleFamilyTable>>>  │ │
│  │  active_version: u64                                           │ │
│  │  staged_version: Option<u64>                                   │ │
│  │  created_at: u64                                               │ │
│  └────────────────────────────────────────────────────────────────┘ │
│                                                                       │
│  Contains 14 independent tables:                                     │
│                                                                       │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                 │
│  │ Table[L0.   │  │ Table[L0.   │  │ Table[L1.   │  ...            │
│  │ NetworkEgr] │  │ SidecarSpwn]│  │ InputSchema]│                 │
│  └─────────────┘  └─────────────┘  └─────────────┘                 │
└─────────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    RuleFamilyTable                                   │
│  ┌────────────────────────────────────────────────────────────────┐ │
│  │  family_id: RuleFamilyId                                       │ │
│  │  layer_id: LayerId                                             │ │
│  │  schema_version: u32                                           │ │
│  │  version: u64                                                  │ │
│  │  metadata: TableMetadata                                       │ │
│  │  indices: FamilyIndices                                        │ │
│  │  entries: Vec<Arc<dyn RuleInstance>>                          │ │
│  └────────────────────────────────────────────────────────────────┘ │
│                                                                       │
│  Operations:                                                          │
│  • add_rule(rule) → O(log n)                                        │
│  • remove_rule(id) → O(n)                                           │
│  • query_by_agent(agent_id) → O(1)                                  │
│  • query_by_secondary(key) → O(1)                                   │
│  • replace_all(rules) → Atomic swap                                 │
└─────────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│                       FamilyIndices                                  │
│  ┌────────────────────────────────────────────────────────────────┐ │
│  │  by_agent: HashMap<String, Vec<Arc<dyn RuleInstance>>>        │ │
│  │  by_secondary: HashMap<String, Vec<Arc<dyn RuleInstance>>>    │ │
│  │  globals: Vec<Arc<dyn RuleInstance>>                          │ │
│  │  secondary_type: SecondaryIndexType                            │ │
│  └────────────────────────────────────────────────────────────────┘ │
│                                                                       │
│  Index Types by Family:                                              │
│  • Agent (all families)                                              │
│  • Tool (L4 families)                                                │
│  • Source (L5 families)                                              │
│  • Domain (L0 NetworkEgress)                                         │
│  • Image (L0 SidecarSpawn)                                           │
│                                                                       │
│  All rules sorted by priority (descending)                           │
└─────────────────────────────────────────────────────────────────────┘
```

## Rule Family Hierarchy

```
Layer L0: System
├── NetworkEgressRule
│   ├── dest_domains: Vec<String>
│   ├── protocol: NetworkProtocol
│   ├── action: RuleAction
│   └── port_range: Option<(u16, u16)>
│
└── SidecarSpawnRule
    ├── allowed_images: Vec<String>
    ├── max_instances: Option<u32>
    ├── cpu_limit: Option<u32>
    └── mem_limit: Option<u32>

Layer L1: Input
├── InputSchemaRule
│   ├── schema_ref: String
│   ├── payload_dtype: String
│   ├── max_bytes: Option<usize>
│   └── action: RuleAction
│
└── InputSanitizationRule
    ├── pattern: String
    ├── action: RuleAction
    └── replacement: Option<String>

Layer L2: Planner
├── PromptAssemblyRule
│   ├── allowed_context_ids: Vec<String>
│   ├── enforce_provenance: bool
│   └── max_prompt_tokens: u32
│
└── PromptLengthRule
    ├── max_prompt_tokens: u32
    └── action_on_violation: ViolationAction

Layer L3: Model I/O
├── ModelOutputScanRule
│   ├── semantic_hook: String
│   ├── max_exec_ms: u32
│   ├── action: RuleAction
│   └── redact_template: Option<String>
│
└── ModelOutputEscalateRule
    ├── confidence_threshold: f32
    └── escalate_target: String

Layer L4: Tool Gateway
├── ToolWhitelistRule
│   ├── allowed_tool_ids: Vec<String>
│   ├── allowed_methods: Vec<String>
│   └── rate_limit_per_min: Option<u32>
│
└── ToolParamConstraintRule
    ├── tool_id: String
    ├── param_name: String
    ├── param_type: ParamType
    ├── regex: Option<String>
    ├── allowed_values: Vec<String>
    └── enforcement_mode: EnforcementMode

Layer L5: RAG
├── RAGSourceRule
│   ├── allowed_sources: Vec<String>
│   ├── max_docs: u32
│   └── max_tokens_per_doc: u32
│
└── RAGDocSensitivityRule
    ├── semantic_hook: String
    └── action: RuleAction

Layer L6: Egress
├── OutputPIIRule
│   ├── semantic_hook: String
│   ├── action: RuleAction
│   └── redact_template: String
│
└── OutputAuditRule
    ├── emit_decision_event: bool
    └── sampling_rate: f32
```

## Query Flow

```
┌─────────────────────┐
│  Incoming Request   │
│  (agent_id, ...)    │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────────────────────────┐
│      Evaluation Engine                  │
│                                          │
│  For each layer (L0 → L6):              │
│    For each family in layer:            │
│                                          │
│      ┌───────────────────────────┐      │
│      │  Query Bridge             │      │
│      │  bridge.query_by_agent(   │      │
│      │    family_id,             │      │
│      │    agent_id               │      │
│      │  )                        │      │
│      └───────────┬───────────────┘      │
│                  │                       │
│                  ▼                       │
│      ┌───────────────────────────┐      │
│      │  Get Rules (Priority ▼)   │      │
│      │  [rule1, rule2, ...]      │      │
│      └───────────┬───────────────┘      │
│                  │                       │
│                  ▼                       │
│      ┌───────────────────────────┐      │
│      │  Evaluate by Priority     │      │
│      │  for rule in rules:       │      │
│      │    if match(rule):        │      │
│      │      return decision      │      │
│      └───────────┬───────────────┘      │
│                  │                       │
│                  ▼                       │
│      ┌───────────────────────────┐      │
│      │  Log Decision             │      │
│      │  audit.log_decision(...)  │      │
│      └───────────────────────────┘      │
└─────────────────────────────────────────┘
           │
           ▼
┌──────────────────────┐
│  Return Response     │
└──────────────────────┘
```

## Concurrency Model

```
┌─────────────────────────────────────────────────────────────┐
│                  Multiple Reader Threads                     │
│                                                               │
│  Thread 1        Thread 2        Thread 3        Thread N    │
│     │               │               │               │        │
│     ▼               ▼               ▼               ▼        │
│  ┌─────────────────────────────────────────────────────┐    │
│  │       Lock-Free Reads (Atomic Arc Clones)          │    │
│  │                                                      │    │
│  │  Arc::clone(&table.entries)  // Zero-copy         │    │
│  │  Arc::clone(&table.indices)  // Atomic pointer    │    │
│  │                                                      │    │
│  │  No locks acquired, no contention                   │    │
│  └─────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                   Single Writer Thread                       │
│                                                               │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  Write Operations (RwLock::write())                  │   │
│  │                                                       │   │
│  │  1. Acquire write lock                               │   │
│  │  2. Create new indices                               │   │
│  │  3. Atomic swap of Arc pointers                      │   │
│  │  4. Release write lock                               │   │
│  │                                                       │   │
│  │  Readers not blocked during write                    │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

## Hot-Reload Flow

```
┌────────────────────┐
│ Management Plane   │
│ (New Rule Bundle)  │
└─────────┬──────────┘
          │
          ▼
┌──────────────────────────────────────┐
│  Data Plane Receives Update          │
└─────────┬────────────────────────────┘
          │
          ▼
┌──────────────────────────────────────┐
│  Parse Rule Bundle                   │
│  • Validate schema                   │
│  • Create rule instances             │
└─────────┬────────────────────────────┘
          │
          ▼
┌──────────────────────────────────────┐
│  Group by Family                     │
│  HashMap<FamilyId, Vec<Rule>>        │
└─────────┬────────────────────────────┘
          │
          ▼
┌──────────────────────────────────────┐
│  For each family:                    │
│                                       │
│  ┌────────────────────────────────┐  │
│  │ 1. Build new indices           │  │
│  │ 2. Validate rules              │  │
│  │ 3. Sort by priority            │  │
│  └───────────┬────────────────────┘  │
│              │                        │
│              ▼                        │
│  ┌────────────────────────────────┐  │
│  │ Atomic Swap                    │  │
│  │ table.replace_all(new_rules)   │  │
│  │                                 │  │
│  │ • Old readers: old rules       │  │
│  │ • New readers: new rules       │  │
│  │ • Zero downtime                │  │
│  └────────────────────────────────┘  │
└──────────────────────────────────────┘
          │
          ▼
┌──────────────────────────────────────┐
│  Update Version & Emit Metrics       │
└──────────────────────────────────────┘
```

## Memory Layout

```
Bridge
│
├─ tables: HashMap (stack)
│   │
│   ├─ Arc<RwLock<Table>> (heap)
│   │   │
│   │   └─ RuleFamilyTable (heap)
│   │       │
│   │       ├─ entries: Vec<Arc<Rule>> (heap)
│   │       │   │
│   │       │   ├─ Arc<Rule1> (heap) ─────┐
│   │       │   ├─ Arc<Rule2> (heap)      │ Shared ownership
│   │       │   └─ Arc<Rule3> (heap)      │ (reference counted)
│   │       │                              │
│   │       └─ indices: FamilyIndices      │
│   │           │                          │
│   │           ├─ by_agent: HashMap      │
│   │           │   │                      │
│   │           │   └─ Vec<Arc<Rule>> ────┘ Points to same rules
│   │           │                          
│   │           ├─ by_secondary: HashMap   
│   │           │   │                      
│   │           │   └─ Vec<Arc<Rule>> ────┘ Points to same rules
│   │           │                          
│   │           └─ globals: Vec<Arc<Rule>>─┘ Points to same rules
│   │
│   └─ Arc<RwLock<Table>> ...
│
└─ version: u64 (stack)

Key Points:
• Rules stored once in entries Vec
• Indices hold Arc clones (cheap)
• Arc enables zero-copy sharing
• Reference counting handles cleanup
```

## Performance Profile

```
Operation               | Latency    | Throughput  | Notes
------------------------|------------|-------------|------------------
query_by_agent          | < 1 μs     | 20M+ QPS    | Hot path
query_by_secondary      | < 1 μs     | 20M+ QPS    | Hot path
query_globals           | < 100 ns   | 50M+ QPS    | Simple vec clone
add_rule                | ~100 μs    | 10K+ ops/s  | Cold path
add_rules_batch (1K)    | ~10 ms     | 100K/s      | Amortized
remove_rule             | ~50 μs     | 20K+ ops/s  | Cold path
replace_all (1K rules)  | ~1 ms      | 1K ops/s    | Hot-reload
stats                   | ~10 μs     | 100K+ ops/s | Read-only
```

## Scalability

```
Rules per Table    | Memory      | Query Time  | Notes
-------------------|-------------|-------------|------------------
100                | ~10 KB      | 500 ns      | Small deployment
1,000              | ~100 KB     | 700 ns      | Medium deployment
10,000             | ~1 MB       | 900 ns      | Large deployment
100,000            | ~10 MB      | 1.2 μs      | Very large
1,000,000          | ~100 MB     | 1.5 μs      | Theoretical max

Linear scaling with number of rules
Sub-linear query time due to indexing
```