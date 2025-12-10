# AuditRecord Module

## Overview

The AuditRecord module provides comprehensive audit trails for rule execution with cryptographic provenance verification, detailed timestamp tracking, and decision classification. It ensures every rule evaluation is traceable, verifiable, and compliant with governance requirements.

## Design Principles

1. **Tamper-Evident**: Cryptographic hashing prevents undetected modifications
2. **Compact for Fast-Path**: Minimal overhead for high-throughput logging
3. **Complete Provenance**: Full execution context for debugging and compliance
4. **Async-Friendly**: Designed for asynchronous persistence
5. **Queryable**: Rich metadata enables efficient log analysis
6. **Multi-Level**: Configurable log levels balance detail vs. volume

## Core Components

### 1. CompactDecisionRecord

Ultra-lightweight record for fast-path logging:

```rust
pub struct CompactDecisionRecord {
    pub seq: SequenceNumber,           // Monotonic sequence
    pub rule_id: String,                // Rule identifier
    pub rule_version: u64,              // Rule version
    pub decision: String,               // Decision summary
    pub timestamp_ms: u64,              // Unix timestamp (ms)
    pub decision_hash: String,          // SHA-256 hash
    pub payload_refs: Vec<PayloadRef>,  // SHM references
}
```

**Use Case**: Hot-path logging where every microsecond counts

**Overhead**: ~100 bytes per record

**Features**:
- Automatic hash computation
- Hash verification for tamper detection
- Conversion to full audit records
- Minimal serialization size

### 2. AuditRecord

Complete audit record with full context:

```rust
pub struct AuditRecord {
    pub seq: SequenceNumber,
    pub rule_id: String,
    pub rule_version: u64,
    pub bundle_id: Option<String>,
    pub outcome: DecisionOutcome,
    pub timestamps: EvaluationTimestamps,
    pub provenance_hash: String,
    pub payload_refs: Vec<PayloadRef>,
    pub context: AuditContext,
    pub log_level: AuditLogLevel,
    pub emit_event: bool,
    pub explanation: Option<String>,
    pub metadata: HashMap<String, String>,
}
```

**Use Case**: Detailed audit trails for compliance, debugging, and analytics

**Overhead**: ~500-1000 bytes per record

**Features**:
- Rich decision context
- Multi-level timestamps
- Provenance verification
- Extensible metadata

### 3. DecisionOutcome

Strongly-typed decision classification:

```rust
pub enum DecisionOutcome {
    Allow { metadata: Option<...> },
    Deny { reason: String, code: Option<String> },
    Rewrite { transform_type: String },
    Redact { redacted_fields: Vec<String> },
    Route { destination: String },
    SpawnSidecar { sidecar_type: String },
    RateLimit { scope: String, action: String },
    SandboxExecute { sandbox_type: String, result: String },
    ConstraintViolation { violation_type: String, fail_open: bool },
    Error { message: String, code: String },
    Skip,
}
```

**Methods**:
- `is_blocking()`: Check if decision blocks the request
- `is_modification()`: Check if decision modifies payload
- `summary()`: Get human-readable summary

### 4. EvaluationTimestamps

Multi-stage timestamp tracking:

```rust
pub struct EvaluationTimestamps {
    pub received_at: SystemTime,       // Event received
    pub eval_started_at: SystemTime,   // Evaluation started
    pub eval_completed_at: SystemTime, // Evaluation completed
    pub decision_at: SystemTime,       // Decision finalized
    pub audit_created_at: SystemTime,  // Audit record created
}
```

**Derived Metrics**:
- `total_eval_time_us()`: Rule evaluation time
- `total_processing_time_us()`: End-to-end processing time
- `received_at_millis()`: Unix timestamp in milliseconds

### 5. AuditContext

Rich execution context:

```rust
pub struct AuditContext {
    pub source_agent: Option<String>,
    pub dest_agent: Option<String>,
    pub flow_id: Option<String>,
    pub payload_dtype: Option<String>,
    pub enforcement_class: Option<String>,
    pub constraint_violations: Vec<String>,
    pub exec_stats: Option<ExecutionStatistics>,
    pub tenant_id: Option<String>,
    pub request_id: Option<String>,
}
```

**Builder Pattern**:
```rust
let context = AuditContext::builder()
    .source_agent("agent_1".to_string())
    .flow_id("flow_123".to_string())
    .add_violation("timeout".to_string())
    .build();
```

### 6. AuditLogLevel

Hierarchical log levels:

```rust
pub enum AuditLogLevel {
    Critical = 0,  // Only security-critical decisions
    High = 1,      // Denials and violations
    Medium = 2,    // All enforcement actions
    Low = 3,       // All decisions including allows
    Trace = 4,     // Everything including debug
}
```

**Filtering**:
```rust
if record.should_log(AuditLogLevel::High) {
    persist_audit_record(record);
}
```

### 7. AuditTrail

In-memory audit trail manager:

```rust
pub struct AuditTrail {
    next_seq: SequenceNumber,
    records: Vec<AuditRecord>,
    max_in_memory: usize,
}
```

**Features**:
- Sequence number generation
- In-memory buffering
- Query by rule ID
- Query by time range
- Automatic overflow management

## Audit Flow

```
┌─────────────────────────────────────────────────────────┐
│                  Rule Evaluation                        │
└─────────────────────────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────────┐
│  1. Start Timestamps (received_at, eval_started_at)    │
└─────────────────────────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────────┐
│  2. Evaluate Rule (with ExecutionBudget)               │
│     - Fast match                                        │
│     - Syntactic checks                                  │
│     - WASM hooks                                        │
│     - Action execution                                  │
└─────────────────────────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────────┐
│  3. Record Decision Outcome                             │
│     - Allow/Deny/Rewrite/etc.                          │
│     - Constraint violations                             │
│     - Error information                                 │
└─────────────────────────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────────┐
│  4. Complete Timestamps (eval_completed_at,             │
│     decision_at, audit_created_at)                      │
└─────────────────────────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────────┐
│  5. Compute Provenance Hash (SHA-256)                   │
│     - seq, rule_id, rule_version                        │
│     - outcome, timestamps                               │
│     - payload references                                │
└─────────────────────────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────────┐
│  6. Create Audit Record (Compact or Full)               │
│     - Fast path: CompactDecisionRecord                  │
│     - Detailed: Full AuditRecord                        │
└─────────────────────────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────────┐
│  7. Emit to Audit Trail                                 │
│     - Add to in-memory buffer                           │
│     - Trigger async persistence                         │
│     - Emit event (if configured)                        │
└─────────────────────────────────────────────────────────┘
```

## Usage Patterns

### Pattern 1: Fast-Path Logging (Minimal Overhead)

```rust
use audit_record::*;

fn evaluate_fast_rule(rule: &Rule) -> RuleDecision {
    let mut trail = AuditTrail::new(1000);
    let seq = trail.next_seq();
    
    // Evaluate rule
    let decision = rule.evaluate();
    
    // Create compact record (fast!)
    let record = CompactDecisionRecord::new(
        seq,
        rule.id.clone(),
        rule.version,
        decision.to_string(),
        vec![PayloadRef::new("shm_1".to_string(), 0, 1024)],
    );
    
    // Emit to async logger (fire and forget)
    async_logger.log_compact(record);
    
    decision
}
```

### Pattern 2: Full Audit Record (Compliance)

```rust
use audit_record::*;

fn evaluate_rule_with_full_audit(rule: &Rule, event: &Event) -> RuleDecision {
    let mut trail = AuditTrail::new(1000);
    let seq = trail.next_seq();
    
    let timestamps = EvaluationTimestamps::now();
    
    // Evaluate rule
    let (decision, context) = rule.evaluate_with_context(event);
    
    // Build full audit record
    let record = AuditRecord::builder(seq, rule.id.clone(), rule.version)
        .outcome(decision.to_outcome())
        .timestamps(timestamps)
        .context(context)
        .log_level(AuditLogLevel::Medium)
        .explanation(format!("Rule {} matched on field X", rule.id))
        .add_metadata("tenant_id".to_string(), "tenant_123".to_string())
        .build()
        .unwrap();
    
    // Verify integrity
    assert!(record.verify_provenance());
    
    // Add to trail
    trail.add_record(record);
    
    decision
}
```

### Pattern 3: Context-Rich Audit

```rust
use audit_record::*;

fn evaluate_with_rich_context(
    rule: &Rule,
    event: &Event,
    exec_stats: ExecutionStatistics,
) -> AuditRecord {
    let context = AuditContext::builder()
        .source_agent(event.source_agent.clone())
        .dest_agent(event.dest_agent.clone())
        .flow_id(event.flow_id.clone())
        .payload_dtype(event.payload_type.clone())
        .enforcement_class(rule.enforcement_class.to_string())
        .exec_stats(exec_stats)
        .request_id(event.request_id.clone())
        .build();
    
    let outcome = match rule.action {
        ActionType::Deny => DecisionOutcome::Deny {
            reason: "Policy violation".to_string(),
            code: Some("POL001".to_string()),
        },
        ActionType::Allow => DecisionOutcome::Allow {
            metadata: Some(HashMap::from([
                ("reason".to_string(), "passed all checks".to_string())
            ])),
        },
        _ => DecisionOutcome::Skip,
    };
    
    AuditRecord::builder(0, rule.id.clone(), rule.version)
        .outcome(outcome)
        .context(context)
        .build()
        .unwrap()
}
```

### Pattern 4: Constraint Violation Audit

```rust
use audit_record::*;
use execution_constraints::*;

fn evaluate_with_constraint_tracking(
    rule: &Rule,
    budget: &ExecutionBudget,
) -> AuditRecord {
    let violations = budget.get_violations();
    
    let outcome = if !violations.is_empty() {
        DecisionOutcome::ConstraintViolation {
            violation_type: format!("{:?}", violations[0]),
            fail_open: !rule.fail_closed_on_timeout,
        }
    } else {
        DecisionOutcome::Allow { metadata: None }
    };
    
    let mut context = AuditContext::new();
    for violation in violations {
        context.constraint_violations.push(format!("{:?}", violation));
    }
    context.exec_stats = Some(ExecutionStatistics {
        eval_time_us: budget.elapsed_ms() * 1000,
        memory_used_bytes: 0,
        cpu_time_us: 0,
        rules_evaluated: 1,
        constraint_checks: 1,
    });
    
    AuditRecord::builder(0, rule.id.clone(), rule.version)
        .outcome(outcome)
        .context(context)
        .log_level(AuditLogLevel::High)
        .build()
        .unwrap()
}
```

### Pattern 5: Query Audit Trail

```rust
use audit_record::*;

fn analyze_audit_trail(trail: &AuditTrail) {
    // Get all denials
    let denials: Vec<_> = trail
        .get_records()
        .iter()
        .filter(|r| r.outcome.is_blocking())
        .collect();
    
    println!("Total denials: {}", denials.len());
    
    // Get records for specific rule
    let rule_records = trail.get_records_by_rule("rule_001");
    println!("Records for rule_001: {}", rule_records.len());
    
    // Get records in time range
    let start = SystemTime::now() - Duration::from_secs(3600);
    let end = SystemTime::now();
    let recent = trail.get_records_in_range(start, end);
    println!("Records in last hour: {}", recent.len());
    
    // Verify provenance for all records
    for record in trail.get_records() {
        if !record.verify_provenance() {
            eprintln!("TAMPER DETECTED: record seq={}", record.seq);
        }
    }
}
```

## Integration with Other Modules

### With RuleMetadata

```rust
use audit_record::*;
use rule_metadata::*;

fn create_audit_from_metadata(
    metadata: &RuleMetadata,
    outcome: DecisionOutcome,
) -> AuditRecord {
    AuditRecord::builder(0, metadata.rule_id.to_string(), metadata.version)
        .bundle_id(metadata.bundle_id.clone().map(|id| id.to_string()))
        .outcome(outcome)
        .log_level(match metadata.enforcement_mode {
            EnforcementMode::Hard => AuditLogLevel::High,
            EnforcementMode::Soft => AuditLogLevel::Low,
        })
        .emit_event(true)
        .build()
        .unwrap()
}
```

### With ExecutionConstraints

```rust
use audit_record::*;
use execution_constraints::*;

fn audit_constrained_execution(
    rule_id: String,
    budget: &ExecutionBudget,
    outcome: DecisionOutcome,
) -> AuditRecord {
    let stats = budget.stats();
    let exec_stats = ExecutionStatistics {
        eval_time_us: stats.elapsed_ms * 1000,
        memory_used_bytes: stats.memory_used_bytes,
        cpu_time_us: stats.cpu_time_us,
        rules_evaluated: 1,
        constraint_checks: 1,
    };
    
    let mut context = AuditContext::new();
    context.exec_stats = Some(exec_stats);
    
    AuditRecord::builder(0, rule_id, 1)
        .outcome(outcome)
        .context(context)
        .build()
        .unwrap()
}
```

### With ActionClause

```rust
use audit_record::*;
use action_clause::*;

fn outcome_from_action(action: &ActionClause) -> DecisionOutcome {
    match &action.action_type {
        ActionType::Allow => DecisionOutcome::Allow { metadata: None },
        ActionType::Deny => DecisionOutcome::Deny {
            reason: "Rule matched".to_string(),
            code: None,
        },
        ActionType::Rewrite => DecisionOutcome::Rewrite {
            transform_type: "payload_transform".to_string(),
        },
        ActionType::Redact => DecisionOutcome::Redact {
            redacted_fields: vec!["ssn".to_string(), "credit_card".to_string()],
        },
        ActionType::RouteTo => DecisionOutcome::Route {
            destination: "alternate_endpoint".to_string(),
        },
        ActionType::SpawnSidecar => DecisionOutcome::SpawnSidecar {
            sidecar_type: "security_scanner".to_string(),
        },
        ActionType::RateLimit => DecisionOutcome::RateLimit {
            scope: "per_user".to_string(),
            action: "throttle".to_string(),
        },
        _ => DecisionOutcome::Skip,
    }
}
```

## Performance Characteristics

### Memory Footprint

| Component | Size | Notes |
|-----------|------|-------|
| CompactDecisionRecord | ~100 bytes | Stack + small heap |
| AuditRecord | ~500-1000 bytes | Depends on metadata |
| EvaluationTimestamps | 80 bytes | 5 × SystemTime |
| AuditContext | ~200 bytes | Without stats |
| PayloadRef | 64 bytes | Per reference |

### Computational Overhead

| Operation | Time | Notes |
|-----------|------|-------|
| Create compact record | ~500ns | Including hash |
| Create full record | ~2μs | Including builder |
| Compute SHA-256 hash | ~300ns | For provenance |
| Verify hash | ~300ns | Recompute and compare |
| Serialize to JSON | ~5μs | Depends on size |
| Add to AuditTrail | ~100ns | Append to vector |

### Best Practices

1. **Use Compact Records on Hot Path**
   - CompactDecisionRecord for fast rules
   - Convert to full record asynchronously

2. **Batch Async Persistence**
   - Buffer records in-memory
   - Flush periodically or on buffer full
   - Use separate thread/task for I/O

3. **Configure Log Levels Appropriately**
   - Production: Medium or High
   - Development: Trace
   - Compliance: Low (log everything)

4. **Hash Verification Strategy**
   - Verify on read from persistent storage
   - Skip verification on trusted in-memory records
   - Periodic integrity checks

5. **Metadata Discipline**
   - Keep metadata keys consistent
   - Avoid large values (>100 bytes)
   - Use structured data when possible

## Security Considerations

### Tamper Detection

All records include SHA-256 provenance hash:

```rust
let record = AuditRecord::new(...);
assert!(record.verify_provenance()); // Verify integrity

// After tampering
record.rule_id = "different_id";
assert!(!record.verify_provenance()); // Fails!
```

### Sensitive Data Handling

**Do NOT log**:
- Raw payload contents (use references)
- API keys or secrets
- Personally identifiable information (PII)
- Credit card numbers or passwords

**DO log**:
- Payload references (SHM IDs)
- Content hashes
- Decision outcomes
- Timing information
- Redacted field names (not values)

### Compliance Requirements

For SOC2/ISO27001/GDPR compliance:

```rust
// Set appropriate log level
record.log_level = AuditLogLevel::Medium;

// Add compliance metadata
record.add_metadata("compliance_region".to_string(), "EU".to_string());
record.add_metadata("data_classification".to_string(), "sensitive".to_string());

// Set retention policy indicator
record.add_metadata("retention_days".to_string(), "90".to_string());

// Add human explanation
record.set_explanation(
    "Request denied due to failed PII redaction check"
);
```

## Testing

### Unit Tests

All core functionality is tested:

```bash
cargo test audit_record
```

**Coverage**:
- Record creation and validation ✓
- Hash computation and verification ✓
- Builder patterns ✓
- Outcome classification ✓
- Timestamp calculations ✓
- Trail management ✓
- Query operations ✓

### Integration Testing

```rust
#[test]
fn test_full_audit_flow() {
    let mut trail = AuditTrail::new(100);
    let seq = trail.next_seq();
    
    let record = AuditRecord::builder(seq, "rule_001".to_string(), 1)
        .outcome(DecisionOutcome::Allow { metadata: None })
        .build()
        .unwrap();
    
    assert!(record.verify_provenance());
    trail.add_record(record);
    
    assert_eq!(trail.get_records().len(), 1);
}
```

## Future Enhancements

### Phase 1: Advanced Querying
- Index by multiple fields
- Time-series compression
- Aggregation queries

### Phase 2: Distributed Audit
- Cross-node audit trails
- Consistent sequence numbers
- Merkle tree verification

### Phase 3: Analytics Integration
- Stream to data warehouse
- Real-time dashboards
- Anomaly detection

### Phase 4: Retention Policies
- Automatic archival
- Tiered storage
- GDPR right-to-deletion

## Dependencies

```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
sha2 = "0.10"  # For SHA-256 hashing
```

## API Stability

- **Stable**: Core types (AuditRecord, DecisionOutcome, timestamps)
- **Stable**: Hash computation (SHA-256)
- **Experimental**: AuditTrail query interface
- **Internal**: Serialization format may change

## Thread Safety

- `CompactDecisionRecord`: `Send + Sync`
- `AuditRecord`: `Send + Sync`
- `AuditTrail`: `!Send + !Sync` (use per-thread or with mutex)

For concurrent access, wrap in `Arc<Mutex<AuditTrail>>` or use per-thread trails with external coordination.

## License

Part of the FastPath rule engine. See parent project for license details.