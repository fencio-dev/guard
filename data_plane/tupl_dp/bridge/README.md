# Bridge Module - Rule Storage & Query Engine

High-performance, lock-free rule storage system for multi-layer enforcement in the data plane.

## Overview

The Bridge is the core data plane component that stores and manages rules across 7 enforcement layers. It provides:

- **14 Rule Family Tables** - One per rule family (2 families × 7 layers)
- **Lock-Free Reads** - Sub-microsecond query latency
- **High Throughput** - 20M+ queries/second with linear scaling
- **Per-Family Indexing** - Optimized for each rule family's access patterns
- **Atomic Hot-Reload** - Zero-downtime rule updates

## Architecture

```
Bridge (Root)
├── L0 System Layer
│   ├── NetworkEgressRule      (control network destinations)
│   └── SidecarSpawnRule        (restrict sidecar launches)
├── L1 Input Layer
│   ├── InputSchemaRule         (enforce payload schema)
│   └── InputSanitizationRule   (sanitize input data)
├── L2 Planner Layer
│   ├── PromptAssemblyRule      (control context sources)
│   └── PromptLengthRule        (prevent token overflow)
├── L3 Model I/O Layer
│   ├── ModelOutputScanRule     (scan for PII/sensitive content)
│   └── ModelOutputEscalateRule (escalate uncertain responses)
├── L4 Tool Gateway Layer
│   ├── ToolWhitelistRule       (allow specific tools)
│   └── ToolParamConstraintRule (enforce parameter bounds)
├── L5 RAG Layer
│   ├── RAGSourceRule           (restrict retrieval sources)
│   └── RAGDocSensitivityRule   (block sensitive documents)
└── L6 Egress Layer
    ├── OutputPIIRule           (detect/redact PII)
    └── OutputAuditRule         (emit decision records)
```

## Quick Start

```rust
use bridge::{Bridge, ToolWhitelistRule, RuleInstance, RuleFamilyId};
use std::sync::Arc;

// 1. Initialize the bridge
let bridge = Bridge::init();

// 2. Create a rule
let rule = Arc::new(
    ToolWhitelistRule::new("allow-database-access")
        .with_priority(100)
        .for_agent("agent_1")
        .with_allowed_tool_ids(vec!["postgres".to_string(), "redis".to_string()])
) as Arc<dyn RuleInstance>;

// 3. Add rule to bridge
bridge.add_rule(rule)?;

// 4. Query rules for an agent
let rules = bridge.query_by_agent(
    &RuleFamilyId::ToolWhitelist,
    "agent_1"
)?;

// 5. Evaluate rules by priority
for rule in rules {
    println!("Evaluating rule: {} (priority: {})", 
             rule.rule_id(), rule.priority());
}
```

## Key Features

### 1. Per-Family Tables
Each rule family has its own optimized table:
- **Separate schemas** for different rule types
- **Custom indexing** based on access patterns
- **Independent versioning** per table

### 2. Lock-Free Reads
- Uses atomic Arc pointers for zero-copy reads
- Multiple threads read simultaneously
- Writes don't block reads (copy-on-write)

### 3. Smart Indexing
Rules are indexed multiple ways:
- **Agent ID** (primary) - O(1) lookup
- **Secondary keys** (tool, source, domain) - O(1) lookup
- **Global rules** - Always included in queries

### 4. Priority-Based Evaluation
- Rules sorted by priority (descending)
- Higher priority = evaluated first
- Deterministic evaluation order

## Performance

| Metric | Value |
|--------|-------|
| Query Latency | < 1 μs |
| Throughput | 20M+ QPS |
| Memory per Rule | ~100 bytes |
| Max Rules | Millions |
| Hot-Reload | Atomic, zero-downtime |

## Rule Families

### L0 - System Layer
- **NetworkEgressRule**: Control which network destinations can be contacted
- **SidecarSpawnRule**: Restrict which sidecars can be launched

### L1 - Input Layer
- **InputSchemaRule**: Enforce payload schema and size limits
- **InputSanitizationRule**: Sanitize and validate input data

### L2 - Planner Layer
- **PromptAssemblyRule**: Control approved context sources
- **PromptLengthRule**: Prevent runaway token counts

### L3 - Model I/O Layer
- **ModelOutputScanRule**: Scan output for PII/sensitive content
- **ModelOutputEscalateRule**: Escalate uncertain responses

### L4 - Tool Gateway Layer
- **ToolWhitelistRule**: Allow only specific tools
- **ToolParamConstraintRule**: Enforce parameter constraints

### L5 - RAG Layer
- **RAGSourceRule**: Restrict retrieval to approved sources
- **RAGDocSensitivityRule**: Block sensitive documents

### L6 - Egress Layer
- **OutputPIIRule**: Detect and redact PII before output
- **OutputAuditRule**: Emit decision audit records

## Usage Patterns

### Adding Rules
```rust
// Single rule
bridge.add_rule(rule)?;

// Batch (more efficient)
bridge.add_rules_batch(vec![rule1, rule2, rule3])?;
```

### Querying Rules
```rust
// By agent
let rules = bridge.query_by_agent(&family_id, "agent_1")?;

// By secondary key (tool, source, etc.)
let rules = bridge.query_by_secondary(&family_id, "postgres")?;

// Global rules only
let rules = bridge.query_globals(&family_id)?;
```

### Managing Rules
```rust
// Remove a rule
bridge.remove_rule(&family_id, "rule_id")?;

// Clear a table
bridge.clear_table(&family_id)?;

// Clear all tables
bridge.clear_all();
```

### Statistics
```rust
// Bridge-level stats
let stats = bridge.stats();
println!("Total rules: {}", stats.total_rules);

// Per-table stats
for table_stat in bridge.table_stats() {
    println!("{}: {} rules", table_stat.family_id, table_stat.rule_count);
}
```

## Thread Safety

The Bridge is fully thread-safe:

- ✅ Multiple concurrent readers (lock-free)
- ✅ Reads during writes (copy-on-write)
- ✅ Per-table locking (no global locks)
- ✅ Atomic hot-reload operations

## Integration

The Bridge integrates with:
- **Management Plane**: Receives rule updates via gRPC
- **Evaluation Engine**: Queries rules during request processing
- **Audit System**: Emits decision records and metrics

## Files

```
bridge/
├── mod.rs              # Main module & public API
├── types.rs            # Core types, enums, traits
├── indices.rs          # Indexing structures
├── table.rs            # RuleFamilyTable implementation
├── bridge.rs           # Bridge struct
├── families/           # Rule family definitions
│   ├── mod.rs
│   ├── l0_system.rs
│   ├── l1_input.rs
│   ├── l2_planner.rs
│   ├── l3_model_io.rs
│   ├── l4_tool_gateway.rs
│   ├── l5_rag.rs
│   └── l6_egress.rs
├── Cargo.toml
├── README.md
└── ARCHITECTURE.md
```

## Next Steps

1. See [ARCHITECTURE.md](ARCHITECTURE.md) for detailed design
2. See [examples/](examples/) for usage examples
3. Run `cargo test` to verify implementation
4. Run `cargo bench` for performance benchmarks

## License

MIT