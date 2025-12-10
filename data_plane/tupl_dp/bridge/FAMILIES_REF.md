# Bridge Module - Rule Families Quick Reference

Complete reference for all 14 rule families across 7 layers.

---

## L0 - System Layer

### NetworkEgressRule (`net_egress`)

**Purpose**: Control which network destinations an agent or sidecar can contact.

**Key Fields**:
```rust
pub struct NetworkEgressRule {
    pub dest_domains: Vec<String>,        // required
    pub port_range: Option<(u16, u16)>,   // optional
    pub protocol: NetworkProtocol,        // default: HTTPS
    pub action: RuleAction,               // ALLOW, DENY, REDIRECT
    pub redirect_target: Option<String>,  // for REDIRECT action
}
```

**Example**:
```rust
let rule = NetworkEgressRule::new("block-untrusted")
    .for_agent("agent_1")
    .with_dest_domains(vec!["*.malicious.com".to_string()])
    .with_protocol(NetworkProtocol::HTTPS)
    .with_action(RuleAction::Deny);
```

**Indexing**: Agent + Domain (secondary)

---

### SidecarSpawnRule (`sidecar_spawn`)

**Purpose**: Restrict which sidecars an agent may launch.

**Key Fields**:
```rust
pub struct SidecarSpawnRule {
    pub allowed_images: Vec<String>,  // required
    pub max_ttl: Option<u64>,         // seconds
    pub max_instances: Option<u32>,   // count
    pub cpu_limit: Option<u32>,       // millicores
    pub mem_limit: Option<u32>,       // MB
}
```

**Example**:
```rust
let rule = SidecarSpawnRule::new("restrict-sidecars")
    .for_agent("agent_1")
    .with_allowed_images(vec![
        "approved-sidecar:v1".to_string(),
        "approved-sidecar:v2".to_string()
    ])
    .with_max_instances(3)
    .with_cpu_limit(2000)  // 2 cores
    .with_mem_limit(4096); // 4GB
```

**Indexing**: Agent + Image (secondary)

---

## L1 - Input Layer

### InputSchemaRule (`input_schema`)

**Purpose**: Enforce payload schema, size, and type.

**Key Fields**:
```rust
pub struct InputSchemaRule {
    pub schema_ref: String,            // JSONSchema ID (required)
    pub payload_dtype: String,         // content type
    pub max_bytes: Option<usize>,      // size limit
    pub action: RuleAction,            // ALLOW, DENY, REWRITE
}
```

**Example**:
```rust
let rule = InputSchemaRule::new("validate-customer-payload")
    .for_agent("agent_1")
    .with_schema_ref("customer-schema-v1".to_string())
    .with_payload_dtype("application/json".to_string())
    .with_max_bytes(1024 * 1024)  // 1MB
    .with_action(RuleAction::Deny);
```

**Indexing**: Agent only

---

### InputSanitizationRule (`input_sanitize`)

**Purpose**: Sanitize and validate input data.

**Key Fields**:
```rust
pub struct InputSanitizationRule {
    pub pattern: String,               // regex pattern
    pub action: RuleAction,            // DENY, REWRITE
    pub replacement: Option<String>,   // for REWRITE
    pub max_matches: Option<u32>,      // limit replacements
}
```

**Example**:
```rust
let rule = InputSanitizationRule::new("strip-html-tags")
    .for_agent("agent_1")
    .with_pattern("<[^>]+>".to_string())
    .with_action(RuleAction::Rewrite)
    .with_replacement("".to_string());
```

**Indexing**: Agent only

---

## L2 - Planner Layer

### PromptAssemblyRule (`prompt_assembly`)

**Purpose**: Allow only approved context sources during prompt building.

**Key Fields**:
```rust
pub struct PromptAssemblyRule {
    pub allowed_context_ids: Vec<String>,  // required
    pub enforce_provenance: bool,          // default: true
    pub max_prompt_tokens: u32,            // default: 8192
}
```

**Example**:
```rust
let rule = PromptAssemblyRule::new("restrict-context")
    .for_agent("agent_1")
    .with_allowed_context_ids(vec![
        "approved-kb-1".to_string(),
        "approved-kb-2".to_string()
    ])
    .with_enforce_provenance(true)
    .with_max_prompt_tokens(8192);
```

**Indexing**: Agent only

---

### PromptLengthRule (`prompt_length`)

**Purpose**: Prevent runaway token count in composed prompt.

**Key Fields**:
```rust
pub struct PromptLengthRule {
    pub max_prompt_tokens: u32,                    // required
    pub action_on_violation: ViolationAction,      // TRUNCATE, DENY
}
```

**Example**:
```rust
let rule = PromptLengthRule::new("limit-prompt-tokens")
    .for_agent("agent_1")
    .with_max_prompt_tokens(8192)
    .with_action_on_violation(ViolationAction::Truncate);
```

**Indexing**: Agent only

---

## L3 - Model I/O Layer

### ModelOutputScanRule (`model_output_scan`)

**Purpose**: Scan model output for PII, jailbreak, or sensitive content.

**Key Fields**:
```rust
pub struct ModelOutputScanRule {
    pub semantic_hook: String,             // WASM module ref
    pub max_exec_ms: u32,                  // timeout
    pub action: RuleAction,                // REDACT, DENY, ESCALATE
    pub redact_template: Option<String>,   // for REDACT
    pub escalate_target: Option<String>,   // for ESCALATE
}
```

**Example**:
```rust
let rule = ModelOutputScanRule::new("pii-detection")
    .for_agent("agent_1")
    .with_semantic_hook("pii-detector-v1".to_string())
    .with_max_exec_ms(50)
    .with_action(RuleAction::Redact)
    .with_redact_template("[REDACTED]".to_string());
```

**Indexing**: Agent only

---

### ModelOutputEscalateRule (`model_output_escalate`)

**Purpose**: Divert uncertain responses to review.

**Key Fields**:
```rust
pub struct ModelOutputEscalateRule {
    pub confidence_threshold: f32,     // 0.0 - 1.0
    pub escalate_target: String,       // queue/endpoint
}
```

**Example**:
```rust
let rule = ModelOutputEscalateRule::new("low-confidence-review")
    .for_agent("agent_1")
    .with_confidence_threshold(0.7)
    .with_escalate_target("human-review".to_string());
```

**Indexing**: Agent only

---

## L4 - Tool Gateway Layer

### ToolWhitelistRule (`tool_whitelist`)

**Purpose**: Allow only specific tools for an agent.

**Key Fields**:
```rust
pub struct ToolWhitelistRule {
    pub allowed_tool_ids: Vec<String>,       // required
    pub allowed_methods: Vec<String>,        // optional
    pub rate_limit_per_min: Option<u32>,     // optional
}
```

**Example**:
```rust
let rule = ToolWhitelistRule::new("allow-database-tools")
    .for_agent("agent_1")
    .with_allowed_tool_ids(vec![
        "postgres".to_string(),
        "redis".to_string()
    ])
    .with_allowed_methods(vec![
        "query".to_string(),
        "get".to_string()
    ])
    .with_rate_limit_per_min(100);
```

**Indexing**: Agent + Tool (secondary)

**Helper Methods**:
```rust
rule.is_tool_allowed("postgres");     // true
rule.is_method_allowed("query");      // true
```

---

### ToolParamConstraintRule (`tool_param_constraint`)

**Purpose**: Enforce parameter type and value bounds for tool calls.

**Key Fields**:
```rust
pub struct ToolParamConstraintRule {
    pub tool_id: String,                  // required
    pub param_name: String,               // required
    pub param_type: ParamType,            // String, Int, Float, Bool
    pub regex: Option<String>,            // for string validation
    pub allowed_values: Vec<String>,      // enum values
    pub max_len: Option<usize>,           // string length
    pub min_value: Option<f64>,           // numeric min
    pub max_value: Option<f64>,           // numeric max
    pub enforcement_mode: EnforcementMode, // HARD, SOFT
}
```

**Example**:
```rust
let rule = ToolParamConstraintRule::new("limit-query-length")
    .for_agent("agent_1")
    .with_tool_id("postgres")
    .with_param_name("query")
    .with_param_type(ParamType::String)
    .with_max_len(10000)
    .with_enforcement_mode(EnforcementMode::Hard);
```

**Indexing**: Agent + Tool (secondary)

---

## L5 - RAG Layer

### RAGSourceRule (`rag_source`)

**Purpose**: Restrict retriever to specific sources or indices.

**Key Fields**:
```rust
pub struct RAGSourceRule {
    pub allowed_sources: Vec<String>,    // required
    pub max_docs: u32,                   // default: 5
    pub max_tokens_per_doc: u32,         // default: 1000
}
```

**Example**:
```rust
let rule = RAGSourceRule::new("restrict-retrieval-sources")
    .for_agent("agent_1")
    .with_allowed_sources(vec![
        "public-docs".to_string(),
        "help-center".to_string()
    ])
    .with_max_docs(5)
    .with_max_tokens_per_doc(1000);
```

**Indexing**: Agent + Source (secondary)

---

### RAGDocSensitivityRule (`rag_doc_sensitivity`)

**Purpose**: Block sensitive or classified docs from being injected.

**Key Fields**:
```rust
pub struct RAGDocSensitivityRule {
    pub semantic_hook: String,         // WASM classifier
    pub action: RuleAction,            // DENY, ESCALATE
}
```

**Example**:
```rust
let rule = RAGDocSensitivityRule::new("block-classified-docs")
    .for_agent("agent_1")
    .with_semantic_hook("sensitivity-classifier-v1".to_string())
    .with_action(RuleAction::Deny);
```

**Indexing**: Agent only

---

## L6 - Egress Layer

### OutputPIIRule (`output_pii`)

**Purpose**: Detect and redact/deny PII before response leaves system.

**Key Fields**:
```rust
pub struct OutputPIIRule {
    pub semantic_hook: String,             // PII detector
    pub action: RuleAction,                // REDACT, DENY
    pub redact_template: String,           // default: "[REDACTED]"
}
```

**Example**:
```rust
let rule = OutputPIIRule::new("output-pii-redaction")
    .for_agent("agent_1")
    .with_semantic_hook("pii-detector-v1".to_string())
    .with_action(RuleAction::Redact)
    .with_redact_template("[CONFIDENTIAL]".to_string());
```

**Indexing**: Agent only

---

### OutputAuditRule (`output_audit`)

**Purpose**: Emit decision record for final user-facing outputs.

**Key Fields**:
```rust
pub struct OutputAuditRule {
    pub emit_decision_event: bool,     // default: true
    pub sampling_rate: f32,            // 0.0 - 1.0, default: 1.0
}
```

**Example**:
```rust
let rule = OutputAuditRule::new("audit-all-outputs")
    .for_agent("agent_1")
    .with_emit_decision_event(true)
    .with_sampling_rate(1.0);  // 100% sampling
```

**Indexing**: Agent only

---

## Common Patterns Across All Families

### Basic Rule Creation

```rust
let rule = RuleFamilyType::new("unique-rule-id")
    .with_priority(100)                    // Higher = evaluated first
    .for_agent("agent_1")                  // Or .with_scope(RuleScope::global())
    .with_description("Human-readable");   // Optional
```

### Scope Options

```rust
// Global rule (applies to all agents)
.with_scope(RuleScope::global())

// Single agent
.for_agent("agent_1")

// Multiple agents
.with_scope(RuleScope::for_agents(vec![
    "agent_1".to_string(),
    "agent_2".to_string()
]))

// With tags
.with_scope(
    RuleScope::for_agent("agent_1".to_string())
        .with_tag("environment".to_string(), "prod".to_string())
)
```

### Priority Guidelines

| Range | Purpose |
|-------|---------|
| 1000+ | Critical security rules (always evaluate first) |
| 500-999 | Standard enforcement rules |
| 100-499 | Default policies |
| 1-99 | Fallback/catch-all rules |

---

## Index Types by Family

| Family | Primary Index | Secondary Index |
|--------|--------------|-----------------|
| NetworkEgress | Agent | Domain |
| SidecarSpawn | Agent | Image |
| InputSchema | Agent | None |
| InputSanitize | Agent | None |
| PromptAssembly | Agent | None |
| PromptLength | Agent | None |
| ModelOutputScan | Agent | None |
| ModelOutputEscalate | Agent | None |
| ToolWhitelist | Agent | Tool |
| ToolParamConstraint | Agent | Tool |
| RAGSource | Agent | Source |
| RAGDocSensitivity | Agent | None |
| OutputPII | Agent | None |
| OutputAudit | Agent | None |

---

## Query Patterns by Index Type

### Agent-Only Families

```rust
// Query by agent (returns agent rules + global rules)
let rules = bridge.query_by_agent(&family_id, "agent_1")?;

// Query globals only
let rules = bridge.query_globals(&family_id)?;
```

### Agent + Secondary Families

```rust
// Query by agent
let rules = bridge.query_by_agent(&family_id, "agent_1")?;

// Query by secondary key (tool/source/domain)
let rules = bridge.query_by_secondary(&family_id, "postgres")?;

// Query by both (via table)
if let Some(table) = bridge.get_table(&family_id) {
    let rules = table.read().query_by_agent_and_secondary(
        "agent_1",
        "postgres"
    );
}
```

---

## Rule Instance Trait

All rules implement the `RuleInstance` trait:

```rust
pub trait RuleInstance: Send + Sync {
    fn rule_id(&self) -> &str;
    fn priority(&self) -> u32;
    fn scope(&self) -> &RuleScope;
    fn family_id(&self) -> RuleFamilyId;
    fn layer_id(&self) -> LayerId;
    fn created_at(&self) -> u64;
    fn description(&self) -> Option<&str>;
    fn is_enabled(&self) -> bool;
}
```

This allows generic handling:

```rust
fn log_rule(rule: &dyn RuleInstance) {
    println!("Rule: {}", rule.rule_id());
    println!("  Family: {}", rule.family_id().family_id());
    println!("  Layer: {}", rule.layer_id());
    println!("  Priority: {}", rule.priority());
}
```

---

## Type Casting for Family-Specific Logic

```rust
use std::any::Any;

// Add this to each rule struct:
impl RuleFamilyType {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

// Then cast when needed:
if let Some(whitelist) = rule.as_any().downcast_ref::<ToolWhitelistRule>() {
    if whitelist.is_tool_allowed(tool_id) {
        // ...
    }
}
```

---

## Complete Example: Multi-Layer Setup

```rust
use bridge::*;
use std::sync::Arc;

fn setup_agent_rules(bridge: &Bridge, agent_id: &str) -> Result<(), String> {
    // L0: Network restrictions
    bridge.add_rule(Arc::new(
        NetworkEgressRule::new("block-external")
            .for_agent(agent_id)
            .with_dest_domains(vec!["*.external.com".to_string()])
            .with_action(RuleAction::Deny)
    ) as Arc<dyn RuleInstance>)?;
    
    // L1: Input validation
    bridge.add_rule(Arc::new(
        InputSchemaRule::new("validate-input")
            .for_agent(agent_id)
            .with_schema_ref("standard-v1".to_string())
            .with_max_bytes(1024 * 1024)
    ) as Arc<dyn RuleInstance>)?;
    
    // L2: Prompt controls
    bridge.add_rule(Arc::new(
        PromptLengthRule::new("limit-tokens")
            .for_agent(agent_id)
            .with_max_prompt_tokens(8192)
    ) as Arc<dyn RuleInstance>)?;
    
    // L3: Output scanning
    bridge.add_rule(Arc::new(
        ModelOutputScanRule::new("pii-scan")
            .for_agent(agent_id)
            .with_semantic_hook("pii-detector-v1".to_string())
            .with_action(RuleAction::Redact)
    ) as Arc<dyn RuleInstance>)?;
    
    // L4: Tool restrictions
    bridge.add_rule(Arc::new(
        ToolWhitelistRule::new("allow-tools")
            .for_agent(agent_id)
            .with_allowed_tool_ids(vec!["approved-tool".to_string()])
    ) as Arc<dyn RuleInstance>)?;
    
    // L5: RAG restrictions
    bridge.add_rule(Arc::new(
        RAGSourceRule::new("limit-sources")
            .for_agent(agent_id)
            .with_allowed_sources(vec!["public-kb".to_string()])
    ) as Arc<dyn RuleInstance>)?;
    
    // L6: Output audit
    bridge.add_rule(Arc::new(
        OutputAuditRule::new("audit-output")
            .for_agent(agent_id)
            .with_emit_decision_event(true)
    ) as Arc<dyn RuleInstance>)?;
    
    Ok(())
}
```