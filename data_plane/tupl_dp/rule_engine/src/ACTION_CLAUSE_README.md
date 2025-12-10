# Action Clause Module - Detailed Documentation

## Overview

The `action_clause` module implements the action execution component of rules in the AI Security Layer. It defines what should happen when a rule matches an event, with atomic operations, explicit side effects, and resource constraints.

## Core Design Principles

1. **Atomic Actions**: Each action completes fully or fails entirely (no partial states)
2. **Type-Safe Parameters**: Each action type has its own strongly-typed parameter structure
3. **Explicit Side Effects**: All side effects must be declared and approved by control plane
4. **Resource Constraints**: Actions have bounded time, memory, and CPU usage
5. **Auditable**: Every action execution is logged with full provenance

---

## Action Types

### 1. DENY - Block Requests

**Purpose**: Stop request processing and return error to caller.

**Parameters**:
```rust
pub struct DenyParams {
    pub reason: String,        // Human-readable reason
    pub error_code: String,    // Machine-readable code
    pub http_status: Option<u16>, // HTTP status (if applicable)
}
```

**Side Effects**:
- Request is denied
- Error response returned
- Audit log entry created

**Example**:
```rust
let deny = ActionType::Deny(DenyParams {
    reason: "PII detected in request".to_string(),
    error_code: "ERR_PII_DETECTED".to_string(),
    http_status: Some(403),
});
```

**Use Cases**:
- Block malicious requests
- Enforce security policies
- Content filtering

---

### 2. ALLOW - Explicit Allow

**Purpose**: Explicitly allow request to proceed (with optional logging).

**Parameters**:
```rust
pub struct AllowParams {
    pub log_decision: bool,
    pub reason: Option<String>,
}
```

**Side Effects**:
- Request continues
- Optional audit log

**Example**:
```rust
let allow = ActionType::Allow(AllowParams {
    log_decision: true,
    reason: Some("Passed security validation".to_string()),
});
```

**Use Cases**:
- Whitelist trusted sources
- Explicitly mark safe content
- Audit allow decisions

---

### 3. REWRITE - Modify Payload

**Purpose**: Transform payload before forwarding to destination.

**Parameters**:
```rust
pub struct RewriteParams {
    pub operations: Vec<RewriteOperation>,
    pub preserve_original: bool,
}

pub enum RewriteOperation {
    SetField { path: String, value: String },
    DeleteField { path: String },
    RenameField { from: String, to: String },
    Transform { path: String, function: TransformFunction },
}

pub enum TransformFunction {
    Uppercase, Lowercase, Trim,
    Base64Encode, Base64Decode, Hash,
}
```

**Side Effects**:
- Payload modified in-place
- Original may be logged

**Example**:
```rust
let rewrite = ActionType::Rewrite(RewriteParams {
    operations: vec![
        RewriteOperation::SetField {
            path: "metadata.version".to_string(),
            value: "v2".to_string(),
        },
        RewriteOperation::Transform {
            path: "user.email".to_string(),
            function: TransformFunction::Lowercase,
        },
    ],
    preserve_original: true,
});
```

**Use Cases**:
- Normalize data formats
- Add metadata
- Convert protocols

---

### 4. REDACT - Remove Sensitive Data

**Purpose**: Remove or mask sensitive information from payload.

**Parameters**:
```rust
pub struct RedactParams {
    pub fields: Vec<String>,
    pub strategy: RedactionStrategy,
    pub redaction_template: Option<String>,
}

pub enum RedactionStrategy {
    Remove,   // Delete field entirely
    Mask,     // Replace with template
    Hash,     // Replace with hash
    Partial,  // Show only part (e.g., last 4 digits)
}
```

**Side Effects**:
- Sensitive fields removed/masked
- Redaction logged

**Example**:
```rust
let redact = ActionType::Redact(RedactParams {
    fields: vec!["ssn".to_string(), "credit_card".to_string()],
    strategy: RedactionStrategy::Mask,
    redaction_template: Some("***REDACTED***".to_string()),
});
```

**Use Cases**:
- PII protection
- Compliance (GDPR, HIPAA)
- Data minimization

---

### 5. SPAWN_SIDECAR - Launch Analysis Process

**Purpose**: Spawn a separate process/container for analysis or processing.

**Parameters**:
```rust
pub struct SpawnSidecarParams {
    pub sidecar_spec: SidecarSpec,
    pub block_on_completion: bool,
    pub pass_payload: bool,
}

pub struct SidecarSpec {
    pub sidecar_type: String,
    pub image: String,
    pub cpu_shares: u32,
    pub memory_limit_mb: usize,
    pub timeout: Duration,
}
```

**Side Effects**:
- New process launched
- Resources allocated
- Request may be delayed

**Example**:
```rust
let sidecar = ActionType::SpawnSidecar(SpawnSidecarParams {
    sidecar_spec: SidecarSpec {
        sidecar_type: "ml-analyzer".to_string(),
        image: "security/ml-analyzer:v1".to_string(),
        cpu_shares: 200,
        memory_limit_mb: 512,
        timeout: Duration::from_secs(30),
    },
    block_on_completion: false,
    pass_payload: true,
});
```

**Use Cases**:
- ML model inference
- Deep packet inspection
- Content analysis

---

### 6. ROUTE_TO - Change Destination

**Purpose**: Forward request to different agent or queue.

**Parameters**:
```rust
pub struct RouteToParams {
    pub dest_agent: Option<AgentId>,
    pub queue_name: Option<String>,
    pub preserve_headers: bool,
}
```

**Side Effects**:
- Request routed to new destination
- Original destination bypassed

**Example**:
```rust
let route = ActionType::RouteTo(RouteToParams {
    dest_agent: Some(AgentId::new("security-review")),
    queue_name: None,
    preserve_headers: true,
});
```

**Use Cases**:
- A/B testing
- Load balancing
- Security review pipeline

---

### 7. RATE_LIMIT - Enforce Quotas

**Purpose**: Limit request rate per agent/flow/destination.

**Parameters**:
```rust
pub struct RateLimitParams {
    pub max_requests: u64,
    pub window: Duration,
    pub scope: RateLimitScope,
    pub action_on_exceed: Box<ActionType>,
}

pub enum RateLimitScope {
    PerAgent,
    PerFlow,
    PerDestination,
    Global,
    PerKey,
}
```

**Side Effects**:
- Counter incremented
- May deny if exceeded

**Example**:
```rust
let rate_limit = ActionType::RateLimit(RateLimitParams {
    max_requests: 100,
    window: Duration::from_secs(60),
    scope: RateLimitScope::PerAgent,
    action_on_exceed: Box::new(ActionType::Deny(DenyParams {
        reason: "Rate limit exceeded".to_string(),
        error_code: "ERR_RATE_LIMIT".to_string(),
        http_status: Some(429),
    })),
});
```

**Use Cases**:
- DDoS protection
- Fair resource allocation
- Cost control

---

### 8. LOG - Observability

**Purpose**: Record event for monitoring and analysis.

**Parameters**:
```rust
pub struct LogParams {
    pub level: LogLevel,
    pub message: String,
    pub include_payload: bool,
    pub structured_data: Option<HashMap<String, String>>,
}

pub enum LogLevel {
    Debug, Info, Warning, Error, Critical,
}
```

**Side Effects**:
- Log entry written
- Metrics updated

**Example**:
```rust
let log = ActionType::Log(LogParams {
    level: LogLevel::Warning,
    message: "Suspicious activity detected".to_string(),
    include_payload: false,
    structured_data: Some({
        let mut m = HashMap::new();
        m.insert("category".to_string(), "security".to_string());
        m
    }),
});
```

**Use Cases**:
- Security monitoring
- Debugging
- Compliance auditing

---

### 9. ATTACH_METADATA - Enrich Events

**Purpose**: Add metadata/tags to event for downstream processors.

**Parameters**:
```rust
pub struct AttachMetadataParams {
    pub metadata: HashMap<String, String>,
    pub overwrite_existing: bool,
}
```

**Side Effects**:
- Metadata added to headers
- Available downstream

**Example**:
```rust
let metadata = ActionType::AttachMetadata(AttachMetadataParams {
    metadata: {
        let mut m = HashMap::new();
        m.insert("security_level".to_string(), "high".to_string());
        m
    },
    overwrite_existing: false,
});
```

**Use Cases**:
- Request tagging
- Context enrichment
- Downstream routing hints

---

### 10. CALLBACK - Notify Control Plane

**Purpose**: Send asynchronous event to control plane or external system.

**Parameters**:
```rust
pub struct CallbackParams {
    pub endpoint: String,
    pub event_type: String,
    pub include_payload: bool,
    pub async_delivery: bool,
}
```

**Side Effects**:
- Async event sent
- Does not block request

**Example**:
```rust
let callback = ActionType::Callback(CallbackParams {
    endpoint: "https://control-plane/events".to_string(),
    event_type: "policy_violation".to_string(),
    include_payload: true,
    async_delivery: true,
});
```

**Use Cases**:
- Alerting
- Analytics
- Integration with SIEM

---

### 11. SANDBOX_EXECUTE - Custom Logic

**Purpose**: Execute custom logic via sandboxed WASM module.

**Parameters**:
```rust
pub struct SandboxExecuteParams {
    pub module_id: String,
    pub module_digest: String,
    pub max_exec_time: Duration,
    pub memory_limit_mb: usize,
    pub input_params: Option<HashMap<String, String>>,
}
```

**Side Effects**:
- WASM executed in sandbox
- May modify payload

**Example**:
```rust
let sandbox = ActionType::SandboxExecute(SandboxExecuteParams {
    module_id: "custom-filter".to_string(),
    module_digest: "sha256:abc123...".to_string(),
    max_exec_time: Duration::from_millis(100),
    memory_limit_mb: 50,
    input_params: None,
});
```

**Use Cases**:
- Custom business logic
- Complex transformations
- ML inference

---

## Action Clause Structure

```rust
pub struct ActionClause {
    pub primary_action: ActionType,
    pub secondary_actions: Vec<ActionType>,
    pub allowed_side_effects: HashSet<AllowedSideEffect>,
    pub max_execution_time: Duration,
    pub rollback_on_failure: bool,
}
```

### Execution Semantics

1. **Primary Action**: Executed first, determines success/failure
2. **Secondary Actions**: Executed only if primary succeeds
3. **Atomicity**: All actions complete or none do (if rollback enabled)
4. **Time Budget**: Total time for all actions must be under limit

### Builder Pattern

```rust
let clause = ActionClause::builder(ActionType::Allow(AllowParams::default()))
    .add_secondary(ActionType::Log(log_params))
    .add_secondary(ActionType::AttachMetadata(metadata_params))
    .max_execution_time(Duration::from_millis(200))
    .rollback_on_failure(true)
    .build()?;
```

---

## Side Effects Management

All side effects must be explicitly declared:

```rust
pub enum AllowedSideEffect {
    Logging,
    Metrics,
    PayloadModification,
    MetadataModification,
    StateModification,
    ProcessSpawn,
    ResourceAllocation,
    NetworkCall,
    Routing,
    SandboxExecution,
}
```

### Automatic Inference

The system automatically infers required side effects:

```rust
// DENY automatically gets: Logging, Metrics
// REWRITE automatically gets: PayloadModification, Logging
// SPAWN_SIDECAR gets: ProcessSpawn, ResourceAllocation, Logging
```

### Validation

Actions are validated against allowed side effects:

```rust
let clause = ActionClause::new(action);
clause.validate()?; // Returns error if side effects not allowed
```

---

## Action Results

```rust
pub enum ActionResult {
    Success {
        message: String,
        payload_modified: bool,
        metadata_modified: bool,
    },
    Denied {
        reason: String,
        error_code: String,
    },
    Failed {
        error: String,
        retryable: bool,
    },
    Timeout {
        elapsed: Duration,
    },
    Skipped {
        reason: String,
    },
}
```

### Result Handling

```rust
match result {
    ActionResult::Success { .. } => {
        // Continue processing
    }
    ActionResult::Denied { reason, .. } => {
        // Block request, return error
    }
    ActionResult::Failed { error, retryable } => {
        if retryable {
            // Retry with backoff
        } else {
            // Fail permanently
        }
    }
    ActionResult::Timeout { .. } => {
        // Handle timeout (fail-closed for HARD rules)
    }
    ActionResult::Skipped { .. } => {
        // Continue to next action
    }
}
```

---

## Real-World Examples

### Example 1: PII Detection and Redaction

```rust
let pii_clause = ActionClause::builder(
    ActionType::Redact(RedactParams {
        fields: vec!["ssn".to_string(), "credit_card".to_string()],
        strategy: RedactionStrategy::Mask,
        redaction_template: Some("***".to_string()),
    })
)
.add_secondary(ActionType::Log(LogParams {
    level: LogLevel::Warning,
    message: "PII detected and redacted".to_string(),
    include_payload: false,
    structured_data: None,
}))
.add_secondary(ActionType::Callback(CallbackParams {
    endpoint: "https://compliance/pii-events".to_string(),
    event_type: "pii_detected".to_string(),
    include_payload: false,
    async_delivery: true,
}))
.build()?;
```

### Example 2: Rate Limiting with Graceful Degradation

```rust
let rate_limit_clause = ActionClause::builder(
    ActionType::RateLimit(RateLimitParams {
        max_requests: 100,
        window: Duration::from_secs(60),
        scope: RateLimitScope::PerAgent,
        action_on_exceed: Box::new(ActionType::RouteTo(RouteToParams {
            dest_agent: None,
            queue_name: Some("overflow-queue".to_string()),
            preserve_headers: true,
        })),
    })
)
.build()?;
```

### Example 3: ML-Based Content Moderation

```rust
let moderation_clause = ActionClause::builder(
    ActionType::SpawnSidecar(SpawnSidecarParams {
        sidecar_spec: SidecarSpec {
            sidecar_type: "content-moderator".to_string(),
            image: "ml/moderator:v2".to_string(),
            cpu_shares: 200,
            memory_limit_mb: 512,
            timeout: Duration::from_secs(30),
        },
        block_on_completion: true,
        pass_payload: true,
    })
)
.add_secondary(ActionType::AttachMetadata(AttachMetadataParams {
    metadata: {
        let mut m = HashMap::new();
        m.insert("moderation_status".to_string(), "checked".to_string());
        m
    },
    overwrite_existing: false,
}))
.build()?;
```

---

## Performance Considerations

### Action Execution Times

| Action Type | Typical Time | Max Recommended |
|-------------|-------------|-----------------|
| DENY | < 1μs | 10μs |
| ALLOW | < 1μs | 10μs |
| REWRITE | 10-100μs | 1ms |
| REDACT | 10-100μs | 1ms |
| LOG | 1-10μs | 100μs |
| ATTACH_METADATA | < 1μs | 10μs |
| RATE_LIMIT | 1-10μs | 100μs |
| ROUTE_TO | 10-100μs | 1ms |
| CALLBACK | 1-5ms | 10ms |
| SPAWN_SIDECAR | 100ms-5s | 30s |
| SANDBOX_EXECUTE | 10-100ms | 500ms |

### Optimization Tips

1. **Use DENY early**: If request should be blocked, do it immediately
2. **Batch secondary actions**: Group similar operations
3. **Async callbacks**: Don't block on callbacks
4. **Cache rate limits**: Use in-memory counters
5. **Limit sidecar spawning**: Reserve for critical analysis only

---

## Validation Rules

ActionClause validation checks:

1. **Side effects**: All actions have required side effect permissions
2. **Execution time**: Total time < 30 seconds
3. **Action conflicts**: DENY cannot have secondary actions
4. **Resource limits**: Sidecars have reasonable limits
5. **Authorization**: Actions requiring auth have proper approval

---

## Integration with Other Modules

```rust
// Complete rule structure
struct Rule {
    metadata: RuleMetadata,    // Who, what, when, where
    match_clause: MatchClause, // When to trigger
    action_clause: ActionClause, // What to do
}

// Execution flow
if match_clause.evaluate(&ctx, payload).matched {
    let result = action_clause.execute(&mut action_ctx);
    audit.log_decision(metadata.rule_id(), result);
}
```

---

## Testing Actions

```rust
#[test]
fn test_redact_action() {
    let action = ActionType::Redact(RedactParams {
        fields: vec!["ssn".to_string()],
        strategy: RedactionStrategy::Mask,
        redaction_template: Some("***".to_string()),
    });
    
    assert!(action.modifies_payload());
    assert!(!action.is_blocking());
    
    let clause = ActionClause::new(action);
    assert!(clause.validate().is_ok());
}
```

---

## Best Practices

1. **Start simple**: Use ALLOW/DENY for most rules
2. **Log judiciously**: Don't log every action (performance impact)
3. **Validate early**: Call `clause.validate()` before activation
4. **Use secondary actions**: For observability without blocking
5. **Set realistic timeouts**: Based on action complexity
6. **Test with production data**: Ensure actions work at scale
7. **Monitor execution times**: Alert on slow actions
8. **Use rollback**: Enable for multi-action clauses
9. **Document side effects**: Make them explicit in config
10. **Version actions**: Track changes for audit trail

---

## Future Enhancements

Planned features:
1. **Conditional actions**: Execute based on runtime conditions
2. **Action chaining**: Complex multi-step workflows
3. **Partial rollback**: Rollback only some actions
4. **Action templates**: Reusable action patterns
5. **Dynamic parameters**: Runtime parameter resolution
6. **Action composition**: Combine actions into macros
7. **Performance profiling**: Per-action timing
8. **Action replay**: Re-execute actions for testing

---

## Conclusion

The ActionClause module provides a comprehensive, type-safe, and performant way to define what happens when rules match. With 11 atomic action types, explicit side effect management, and strong validation, it ensures secure and reliable policy enforcement in production systems.