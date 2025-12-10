# Match Clause Module - Detailed Documentation

## Overview

The `match_clause` module implements the matching logic for rules in the AI Security Layer. It determines whether a rule should be applied to a given event/request using a three-tier evaluation model.

## Three-Tier Evaluation Model

```
┌─────────────────────────────────────────────────────────────┐
│                    EVALUATION PIPELINE                       │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Event → [FastMatch] → [MatchExpression] → [WasmHook] → ✓  │
│             ↓               ↓                  ↓             │
│           FAIL            FAIL               FAIL            │
│             ↓               ↓                  ↓             │
│          NO MATCH        NO MATCH          NO MATCH          │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### Tier 1: FastMatch (O(1) Predicates)

**Purpose**: Ultra-fast filtering using hash lookups and bitset operations.

**Characteristics**:
- **Speed**: O(1) operations only
- **Data Access**: Headers and metadata only (no payload)
- **Use Case**: Quick rejection of obviously non-matching events

**What It Checks**:
- Source agent ID (is it in the allowed set?)
- Destination agent ID (is it in the allowed set?)
- Flow ID (is this flow allowed?)
- Payload type (MIME type check)
- Header flags (required and forbidden flags)

**Example**:
```rust
let fast_match = FastMatchBuilder::new()
    .add_source_agent(AgentId::new("gpt-4"))
    .add_source_agent(AgentId::new("claude"))
    .add_payload_type("application/json")
    .require_flags(HeaderFlags::from_bits(HeaderFlags::ENCRYPTED))
    .build();
```

**Performance**: Typically < 100ns per evaluation on modern hardware.

---

### Tier 2: MatchExpression (Syntactic Checks)

**Purpose**: Structured validation using field comparisons, regex, and JSONPath.

**Characteristics**:
- **Speed**: O(log n) to O(n) depending on expression complexity
- **Data Access**: Headers first, payload if needed
- **Use Case**: Complex pattern matching and field validation

**Expression Types**:

1. **Field Comparisons**
   ```rust
   MatchExpression::Field(FieldComparison {
       field_path: "severity".to_string(),
       operator: ComparisonOp::Equal,
       value: FieldValue::String("critical".to_string()),
   })
   ```

2. **Regex Matching**
   ```rust
   MatchExpression::Regex(RegexMatch {
       field_path: "email".to_string(),
       pattern: r"^[a-z]+@example\.com$".to_string(),
       full_match: true,
   })
   ```

3. **JSONPath Queries**
   ```rust
   MatchExpression::JsonPath(JsonPathQuery {
       path: "$.user.roles".to_string(),
       expected: Some(FieldValue::Array(vec![
           FieldValue::String("admin".to_string())
       ])),
       exists_check: false,
   })
   ```

4. **Logical Operators**
   ```rust
   // AND: All must match
   MatchExpression::And(vec![expr1, expr2])
   
   // OR: At least one must match
   MatchExpression::Or(vec![expr1, expr2])
   
   // NOT: Inverts result
   MatchExpression::Not(Box::new(expr))
   ```

**Performance**: Typically 1-100μs depending on complexity.

---

### Tier 3: WasmHook (Semantic Validation)

**Purpose**: Custom validation logic via sandboxed WASM execution.

**Characteristics**:
- **Speed**: O(timeout) - bounded by execution limits
- **Data Access**: Full event context and payload
- **Use Case**: Complex semantic checks (ML models, business logic)

**Configuration**:
```rust
let wasm_hook = WasmHookRef {
    hook_id: "sentiment-analyzer".to_string(),
    module_digest: "sha256:abcd1234...".to_string(),
    max_exec_time: Duration::from_millis(50),
    memory_limit_bytes: 10 * 1024 * 1024, // 10 MB
    cpu_shares: 100,
};
```

**Resource Limits**:
- **Execution Time**: Hard timeout (e.g., 50ms)
- **Memory**: Maximum allocation (e.g., 10 MB)
- **CPU**: Soft scheduling limit

**Failure Modes**:
- Timeout → Return false (no match)
- OOM → Return false (no match)
- Exception → Return false (no match)
- For HARD rules: fail-closed (deny by default)
- For SOFT rules: configurable fallback

**Performance**: Typically 10-100ms depending on hook complexity.

---

## Complete MatchClause Structure

```rust
pub struct MatchClause {
    pub fast_match: Option<FastMatch>,
    pub match_expr: Option<MatchExpression>,
    pub wasm_hook: Option<WasmHookRef>,
}
```

**Construction Patterns**:

```rust
// Fast-only (cheapest)
let clause = MatchClause::fast_only(fast_match);

// Fast + Expression (common)
let clause = MatchClause::with_expression(fast_match, expr);

// Complete (all tiers)
let clause = MatchClause::complete(fast_match, expr, hook);
```

---

## Header Flags - Bitset Operations

Header flags use a `u64` bitset for ultra-fast checks:

```rust
pub struct HeaderFlags(u64);

// Predefined flags
HeaderFlags::ENCRYPTED         // Bit 0
HeaderFlags::AUTHENTICATED     // Bit 1
HeaderFlags::RATE_LIMITED      // Bit 2
HeaderFlags::HIGH_PRIORITY     // Bit 3
HeaderFlags::CONTAINS_PII      // Bit 4
HeaderFlags::REQUIRES_AUDIT    // Bit 5
HeaderFlags::SYNTHETIC         // Bit 6
HeaderFlags::CACHED            // Bit 7
// Bits 8-63 reserved
```

**Operations**:
```rust
let mut flags = HeaderFlags::empty();
flags.set(HeaderFlags::ENCRYPTED);
flags.set(HeaderFlags::AUTHENTICATED);

// Check if contains all flags
flags.contains(HeaderFlags::from_bits(
    HeaderFlags::ENCRYPTED | HeaderFlags::AUTHENTICATED
));

// Check if has any flags in common
flags.intersects(HeaderFlags::from_bits(HeaderFlags::ENCRYPTED));
```

---

## Event Context

The input to match evaluation:

```rust
pub struct EventContext {
    pub source_agent: AgentId,
    pub dest_agent: Option<AgentId>,
    pub flow_id: Option<FlowId>,
    pub payload_type: String,
    pub header_flags: HeaderFlags,
    pub headers: HashMap<String, FieldValue>,
}
```

**Usage**:
```rust
let mut ctx = EventContext::new(
    AgentId::new("gpt-4"),
    "application/json".to_string()
);

ctx.dest_agent = Some(AgentId::new("client-api"));
ctx.flow_id = Some(FlowId::new("conversation-123"));
ctx.header_flags.set(HeaderFlags::ENCRYPTED);
ctx.set_header("severity".to_string(), FieldValue::String("high".to_string()));
```

---

## Payload Data

Represents the actual payload:

```rust
pub struct PayloadData {
    pub raw: Vec<u8>,
    pub fields: HashMap<String, FieldValue>,
}
```

**Usage**:
```rust
let mut payload = PayloadData::from_bytes(json_bytes);

// Manually add parsed fields
payload.fields.insert(
    "user.email".to_string(),
    FieldValue::String("admin@company.com".to_string())
);
```

---

## Field Values

Supported field value types:

```rust
pub enum FieldValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Array(Vec<FieldValue>),
    Null,
}
```

**Conversions**:
```rust
let val: FieldValue = "hello".into();
let val: FieldValue = 42i64.into();
let val: FieldValue = 3.14f64.into();
let val: FieldValue = true.into();
```

---

## Comparison Operators

```rust
pub enum ComparisonOp {
    Equal,              // ==
    NotEqual,           // !=
    GreaterThan,        // >
    GreaterThanOrEqual, // >=
    LessThan,           // <
    LessThanOrEqual,    // <=
    Contains,           // String contains substring
    StartsWith,         // String starts with prefix
    EndsWith,           // String ends with suffix
    In,                 // Value in array
}
```

---

## Match Result

The output of evaluation:

```rust
pub struct MatchResult {
    pub matched: bool,
    pub decided_by: MatchTier,
}

pub enum MatchTier {
    None,            // No tiers present
    FastMatch,       // Failed at fast match
    MatchExpression, // Failed at match expression
    WasmHook,        // Failed at WASM hook
    Complete,        // Passed all tiers
}
```

**Usage**:
```rust
let result = clause.evaluate(&ctx, Some(&payload));

if result.matched {
    println!("Rule matched!");
} else {
    println!("Failed at: {:?}", result.decided_by);
}
```

---

## Performance Optimization Tips

### 1. Always Use FastMatch First

```rust
// ❌ Bad: Expensive expression with no fast pre-filter
let clause = MatchClause {
    fast_match: None,
    match_expr: Some(expensive_expression),
    wasm_hook: None,
};

// ✅ Good: Fast filter before expensive check
let clause = MatchClause {
    fast_match: Some(fast_filter),
    match_expr: Some(expensive_expression),
    wasm_hook: None,
};
```

### 2. Order Expressions by Cost

```rust
// ✅ Put cheap checks first in AND expressions
let expr = MatchExpression::And(vec![
    cheap_header_check,    // Check this first
    expensive_regex_check, // Only if first passes
]);
```

### 3. Avoid Unnecessary Payload Access

```rust
// Check if expression requires payload
if !clause.requires_payload() {
    // Can evaluate without loading payload!
    let result = clause.evaluate(&ctx, None);
}
```

### 4. Use Appropriate Data Types

```rust
// ✅ Use integers for numeric comparisons
FieldValue::Integer(42)

// ❌ Don't use strings for numbers
FieldValue::String("42".to_string())
```

### 5. Compile Regex Patterns Once

In production, regex patterns should be compiled once during rule activation and cached:

```rust
// Pseudo-code for production implementation
struct CompiledRegex {
    pattern: String,
    compiled: Regex, // From regex crate
}

// Compile during rule activation
let compiled = Regex::new(&pattern)?;
```

---

## Real-World Use Cases

### Use Case 1: Content Filtering

```rust
// Block requests containing PII
let pii_clause = MatchClause::with_expression(
    FastMatchBuilder::new()
        .require_flags(HeaderFlags::from_bits(HeaderFlags::CONTAINS_PII))
        .build(),
    MatchExpression::Or(vec![
        MatchExpression::Field(FieldComparison {
            field_path: "content".to_string(),
            operator: ComparisonOp::Contains,
            value: "SSN".into(),
        }),
        MatchExpression::Field(FieldComparison {
            field_path: "content".to_string(),
            operator: ComparisonOp::Contains,
            value: "credit card".into(),
        }),
    ])
);
```

### Use Case 2: Agent-Specific Rate Limiting

```rust
// Rate limit specific agents
let rate_limit_clause = MatchClause::fast_only(
    FastMatchBuilder::new()
        .add_source_agent(AgentId::new("public-api"))
        .forbid_flags(HeaderFlags::from_bits(HeaderFlags::HIGH_PRIORITY))
        .build()
);
```

### Use Case 3: Security Validation

```rust
// Validate JWT tokens with semantic checks
let security_clause = MatchClause::complete(
    FastMatchBuilder::new()
        .require_flags(HeaderFlags::from_bits(HeaderFlags::AUTHENTICATED))
        .build(),
    MatchExpression::Field(FieldComparison {
        field_path: "auth.type".to_string(),
        operator: ComparisonOp::Equal,
        value: "jwt".into(),
    }),
    WasmHookRef::new("jwt-validator".to_string(), "sha256:...".to_string())
);
```

### Use Case 4: Data Quality Checks

```rust
// Ensure data meets quality standards
let quality_clause = MatchClause::with_expression(
    FastMatchBuilder::new()
        .add_payload_type("application/json")
        .build(),
    MatchExpression::And(vec![
        MatchExpression::JsonPath(JsonPathQuery {
            path: "$.schema_version".to_string(),
            expected: Some(FieldValue::String("v2".to_string())),
            exists_check: false,
        }),
        MatchExpression::Field(FieldComparison {
            field_path: "validation_status".to_string(),
            operator: ComparisonOp::Equal,
            value: "passed".into(),
        }),
    ])
);
```

---

## Testing Match Clauses

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_clause() {
        // Create a test clause
        let clause = MatchClause::fast_only(
            FastMatchBuilder::new()
                .add_source_agent(AgentId::new("test-agent"))
                .build()
        );

        // Create test context
        let ctx = EventContext::new(
            AgentId::new("test-agent"),
            "text/plain".to_string()
        );

        // Evaluate
        let result = clause.evaluate(&ctx, None);
        assert!(result.matched);
    }
}
```

---

## Future Enhancements

### Planned Features:
1. **Compiled Regex Support**: Integrate `regex` crate for production
2. **JSONPath Library**: Integrate `jsonpath_lib` for real queries
3. **WASM Runtime**: Integrate `wasmtime` or `wasmer` for sandboxing
4. **Expression Compiler**: Compile expressions to bytecode for faster evaluation
5. **Bloom Filters**: Add bloom filter support for FastMatch
6. **Caching**: Cache evaluation results for identical contexts

### Performance Goals:
- FastMatch: < 100ns
- MatchExpression: < 10μs
- WasmHook: < 50ms (configurable)
- Overall throughput: > 100K evaluations/sec per core

---

## Integration with RuleMetadata

Match clauses work alongside RuleMetadata:

```rust
use rule_engine::{RuleMetadata, RuleScope, EnforcementMode, MatchClause};

// Create rule metadata
let metadata = RuleMetadata::new(
    "security-admin".to_string(),
    RuleScope::global(),
    EnforcementMode::Hard,
);

// Create match clause
let match_clause = MatchClause::fast_only(
    FastMatchBuilder::new()
        .require_flags(HeaderFlags::from_bits(HeaderFlags::AUTHENTICATED))
        .build()
);

// In the complete system, these would be combined:
// struct Rule {
//     metadata: RuleMetadata,
//     match_clause: MatchClause,
//     action_clause: ActionClause, // To be implemented
// }
```

---

## Best Practices

1. **Always start with FastMatch** - It's the cheapest check
2. **Use specific types** - Don't use strings for everything
3. **Compose expressions** - Build complex logic from simple parts
4. **Test edge cases** - Empty sets, missing fields, null values
5. **Monitor performance** - Track which tier causes most failures
6. **Use WASM sparingly** - It's expensive, reserve for truly complex logic
7. **Fail fast** - Order checks from cheapest to most expensive

---

## Common Pitfalls

### ❌ Don't: Skip FastMatch for expensive expressions
```rust
// This evaluates expensive regex on every event!
MatchClause {
    fast_match: None,
    match_expr: Some(expensive_regex),
    wasm_hook: None,
}
```

### ✅ Do: Pre-filter with FastMatch
```rust
MatchClause {
    fast_match: Some(agent_filter),
    match_expr: Some(expensive_regex),
    wasm_hook: None,
}
```

### ❌ Don't: Load payload unnecessarily
```rust
// Always loads payload even if not needed
let payload = load_payload();
clause.evaluate(&ctx, Some(&payload));
```

### ✅ Do: Check if payload is required
```rust
let payload = if clause.requires_payload() {
    Some(load_payload())
} else {
    None
};
clause.evaluate(&ctx, payload.as_ref());
```

---

## Conclusion

The MatchClause module provides a powerful, composable, and performant way to express matching logic for rules. By leveraging the three-tier evaluation model, it achieves both flexibility and speed, making it suitable for high-throughput security enforcement in production systems.