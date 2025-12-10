 # ExecutionConstraints Module

## Overview

The ExecutionConstraints module provides deterministic, low-overhead enforcement of runtime constraints for rule execution in the FastPath rule engine. It ensures that rule evaluation remains within acceptable latency, memory, and CPU bounds, particularly critical for WASM hooks and semantic validation operations.

## Design Principles

1. **Zero-Cost Abstraction**: Constraint checking is designed to be as lightweight as possible, with minimal overhead on the hot path
2. **Fail-Safe Defaults**: Hard rules fail closed on timeout; soft rules can be configured to fail open
3. **Composable Constraints**: Multiple constraint types (time, memory, CPU) can be enforced simultaneously
4. **Auditable**: All constraint violations are tracked and can be logged for debugging and compliance
5. **Flexible**: Supports different constraint profiles for different rule types (fast, semantic, observational)

## Core Components

### 1. ExecutionConstraints

Defines the constraint configuration for a rule or operation:

```rust
pub struct ExecutionConstraints {
    pub max_exec_ms: u64,              // Maximum execution time
    pub cpu_shares: Option<u32>,        // CPU allocation (0-100)
    pub memory_limit_bytes: Option<u64>, // Memory limit
    pub sampling_rate: f64,             // Execution sampling (0.0-1.0)
    pub fail_closed_on_timeout: bool,   // Failure policy
    pub max_retries: Option<u32>,       // Retry attempts
    pub retry_backoff_ms: Option<u64>,  // Backoff between retries
}
```

**Preset Configurations:**

- **`fast_rule()`**: Strict constraints for fast-path rules (5ms, 1MB memory)
- **`semantic_rule()`**: Relaxed constraints for semantic validation (100ms, 10MB memory)
- **`wasm_hook(max_ms)`**: Configurable constraints for WASM sandboxes
- **`observational(sampling_rate)`**: Lightweight constraints for logging/metrics

### 2. ExecutionBudget

Runtime tracker that enforces constraints during execution:

```rust
pub struct ExecutionBudget {
    constraints: ExecutionConstraints,
    start_time: Instant,
    memory_used: u64,
    cpu_time_us: u64,
    violations: Vec<ConstraintViolationType>,
}
```

**Key Methods:**

- `elapsed_ms()`: Get elapsed time since budget creation
- `remaining_ms()`: Get remaining time budget
- `is_timeout()`: Check if time budget exhausted
- `check()`: Validate all constraints, return violations
- `enforce<F, T>(f: F)`: Execute a closure with constraint enforcement
- `stats()`: Get execution statistics

### 3. ConstraintEnforcer

Central manager for constraint policies:

```rust
pub struct ConstraintEnforcer {
    fast_rule_constraints: ExecutionConstraints,
    semantic_rule_constraints: ExecutionConstraints,
    wasm_hook_constraints: ExecutionConstraints,
    observational_constraints: ExecutionConstraints,
}
```

**Usage:**

```rust
let enforcer = ConstraintEnforcer::new();
let result = enforcer.execute_with_constraints(
    RuleType::Fast,
    || {
        // Your rule evaluation logic
        Ok(evaluate_rule())
    }
);
```

### 4. ConstraintViolationType

Enumeration of all possible constraint violations:

- **`TimeoutExceeded`**: Execution time exceeded limit
- **`MemoryExceeded`**: Memory usage exceeded limit
- **`CpuExceeded`**: CPU usage exceeded allocation
- **`SampledOut`**: Operation not executed due to sampling
- **`MultipleViolations`**: Multiple constraints violated simultaneously

## Integration with Rule Engine

The ExecutionConstraints module integrates into the rule evaluation pipeline:

```
┌─────────────────────────────────────────────────────────┐
│                   Rule Evaluation                       │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  1. Create Budget                                       │
│     budget = constraints.create_budget()                │
│                                                         │
│  2. Check Sampling                                      │
│     if !should_sample() -> SampledOut                   │
│                                                         │
│  3. Execute with Budget                                 │
│     budget.enforce(|| {                                 │
│         - Fast match layer                              │
│         - Syntactic checks                              │
│         - WASM hook (if needed)                         │
│         - Action execution                              │
│     })                                                  │
│                                                         │
│  4. Check Constraints                                   │
│     - Timeout check                                     │
│     - Memory check                                      │
│     - CPU check                                         │
│                                                         │
│  5. Handle Violations                                   │
│     - Log violation                                     │
│     - Apply failure policy (fail open/closed)           │
│     - Emit audit event                                  │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

## Constraint Profiles by Rule Type

### Fast Rules
- **Latency**: 1-5ms (strict)
- **Memory**: 1MB
- **CPU**: 10 shares
- **Policy**: Fail closed on timeout
- **Use Case**: Header matching, simple field checks, deny rules

### Semantic Rules
- **Latency**: 50-200ms (relaxed)
- **Memory**: 10MB
- **CPU**: 50 shares
- **Policy**: Fail open on timeout (configurable)
- **Retries**: Up to 2 attempts with 10ms backoff
- **Use Case**: ML inference, complex graph reasoning, content analysis

### WASM Hooks
- **Latency**: Configurable (default 10ms)
- **Memory**: 5MB (sandbox isolated)
- **CPU**: 20 shares
- **Policy**: Fail closed on timeout
- **Use Case**: Custom match logic, transform functions, policy extensions

### Observational Rules
- **Latency**: 1-2ms (minimal)
- **Memory**: 512KB
- **CPU**: 5 shares
- **Sampling**: Configurable (0.0-1.0)
- **Policy**: Always fail open (never block)
- **Use Case**: Logging, metrics, audit trails, telemetry

## Sampling Support

Observational rules support probabilistic execution via sampling rates:

```rust
// Execute 50% of the time
let constraints = ExecutionConstraints::observational(0.5);

// Check if should sample
if constraints.should_sample() {
    // Execute logging/metric logic
}
```

**Sampling Algorithm:**
- Uses thread-local RNG for efficiency
- Deterministic for sampling_rate = 0.0 or 1.0
- Probabilistic for 0.0 < sampling_rate < 1.0

## Retry Policies

For transient failures (network, temporary unavailability):

```rust
let policy = RetryPolicy::new(3, 10) // 3 attempts, 10ms backoff
    .with_exponential_backoff();

let result = policy.execute(|| {
    // Operation that may fail transiently
    call_external_service()
});
```

**Backoff Strategies:**
- **Fixed**: Constant delay between retries
- **Exponential**: Delay = base * 2^attempt (1x, 2x, 4x, 8x...)

## Performance Considerations

### Memory Layout
- `ExecutionConstraints`: 64 bytes (stack allocated)
- `ExecutionBudget`: 128 bytes including timing state
- Zero heap allocations during constraint checks

### Timing Overhead
- Constraint check: ~50-100 nanoseconds
- Sampling decision: ~10-20 nanoseconds
- Budget creation: ~100 nanoseconds

### Optimization Techniques
1. **Fast-path checks first**: Timeout check before memory/CPU
2. **Lazy violation tracking**: Only allocate violation vector when needed
3. **Inline small methods**: `elapsed_ms()`, `is_timeout()` marked for inlining
4. **Lock-free reads**: Budget checks don't require synchronization

## Error Handling

The module uses a strongly-typed error hierarchy:

```rust
pub enum ConstraintError {
    Violation(ConstraintViolationType),
    InvalidConfiguration(String),
    ResourceExhausted(String),
    EnforcementFailure(String),
}
```

**Error Handling Patterns:**

```rust
// Pattern 1: Fail fast on timeout
match budget.check() {
    Ok(_) => continue_execution(),
    Err(ConstraintError::Violation(v)) => {
        if fail_closed {
            return Err(RuleError::Denied)
        } else {
            log_violation(v);
            return Ok(default_action())
        }
    }
}

// Pattern 2: Graceful degradation
let result = budget.enforce(|| expensive_operation())
    .unwrap_or_else(|e| {
        metrics.record_violation(&e);
        fallback_result()
    });
```

## Audit and Monitoring

### Violation Tracking

All violations are recorded in the budget:

```rust
let violations = budget.get_violations();
for violation in violations {
    audit_log.record(AuditEvent {
        event_type: "constraint_violation",
        rule_id: rule.id,
        violation: violation,
        timestamp: Instant::now(),
    });
}
```

### Execution Statistics

Detailed stats available after execution:

```rust
let stats = budget.stats();
println!("Execution took {}ms", stats.elapsed_ms);
println!("Memory used: {} bytes", stats.memory_used_bytes);
println!("CPU time: {}μs", stats.cpu_time_us);
println!("Violations: {}", stats.violation_count);
```

## Configuration Examples

### Strict Fast-Path Configuration
```rust
let enforcer = ConstraintEnforcer::with_constraints(
    ExecutionConstraints {
        max_exec_ms: 1,
        cpu_shares: Some(5),
        memory_limit_bytes: Some(512 * 1024),
        sampling_rate: 1.0,
        fail_closed_on_timeout: true,
        max_retries: None,
        retry_backoff_ms: None,
    },
    // ... other constraint sets
)?;
```

### Relaxed Development Configuration
```rust
let enforcer = ConstraintEnforcer::with_constraints(
    ExecutionConstraints {
        max_exec_ms: 100,
        cpu_shares: None,
        memory_limit_bytes: None,
        sampling_rate: 1.0,
        fail_closed_on_timeout: false,
        max_retries: Some(3),
        retry_backoff_ms: Some(50),
    },
    // ... other constraint sets
)?;
```

### Production with Aggressive Sampling
```rust
let observational = ExecutionConstraints {
    max_exec_ms: 2,
    cpu_shares: Some(5),
    memory_limit_bytes: Some(256 * 1024),
    sampling_rate: 0.01, // Only 1% of traffic
    fail_closed_on_timeout: false,
    max_retries: None,
    retry_backoff_ms: None,
};
```

## Testing

The module includes comprehensive unit tests:

```bash
cargo test execution_constraints
```

**Test Coverage:**
- Constraint validation
- Timeout enforcement
- Memory limit enforcement
- Sampling behavior (probabilistic testing)
- Retry policies
- Statistics collection
- Error propagation

## Future Enhancements

1. **Adaptive Constraints**: Automatically adjust limits based on P99 latencies
2. **Resource Quotas**: Per-agent or per-flow cumulative resource tracking
3. **Priority Queues**: CPU scheduling based on constraint priorities
4. **Histogram Metrics**: Detailed latency distribution tracking
5. **Circuit Breakers**: Automatic rule disabling on repeated violations
6. **Memory Profiling**: Integration with WASM sandbox memory accounting
7. **Distributed Tracing**: Constraint spans for end-to-end visibility

## Dependencies

```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
thiserror = "1.0"
rand = "0.8"
```

## Thread Safety

- `ExecutionConstraints`: `Send + Sync` (immutable after creation)
- `ExecutionBudget`: `!Send + !Sync` (single-threaded use)
- `ConstraintEnforcer`: `Send + Sync` (thread-safe reads)

## API Stability

- **Stable**: `ExecutionConstraints`, `ExecutionBudget`, `ConstraintEnforcer`
- **Experimental**: `RetryPolicy`, adaptive features
- **Internal**: Violation tracking internals may change

## License

Part of the FastPath rule engine. See parent project for license details.