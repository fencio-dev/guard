# Layer-Based Rule Enforcement Design

**Date:** 2025-11-15
**Status:** Design Complete
**Version:** v1.3

## Overview

This design extends the semantic security system to support **layer-based rule enforcement** using the Data Plane's 7-layer rule family architecture. Instead of DesignBoundaries with applicability filters, IntentEvents now carry a `layer` field that directly maps to Data Plane rule families (L0-L6), enabling more precise and performant policy enforcement.

**MVP Scope:** L4 ToolGateway layer (tool call enforcement)

## Goals

1. **Direct Layer Mapping:** IntentEvents specify layer → Data Plane queries specific rule families
2. **Reuse Existing Infrastructure:** Leverage anchor-based embedding system from Phase 2
3. **Fail-Closed Security:** All rules must pass; first failure blocks execution with short-circuit evaluation
4. **Performance:** TTL-based caching for rule embeddings, short-circuit on first block
5. **Clean Integration:** Minimal changes to SDK, extend Management Plane encoding, new Data Plane enforcement engine

## Non-Goals

- Multi-layer enforcement (single layer per intent event for MVP)
- Custom applicability filters (layer is the only filter)
- Syntactic-only rule validation (all rules use semantic matching)
- Other layers beyond L4 (future work)

---

## Architecture

### High-Level Flow

```
1. SDK captures tool call
   ↓ [auto-infers layer=L4]

2. SDK → Data Plane
   ↓ [POST /enforce with IntentEvent]

3. Data Plane queries L4 rules
   ↓ [ToolWhitelist + ToolParamConstraint from memory]

4. Data Plane → Management Plane
   ↓ [POST /encode/intent → 128d vector]

5. Data Plane embeds rules (cached)
   ↓ [POST /encode/rule/{family} → anchor arrays]

6. Data Plane → Semantic Sandbox
   ↓ [Compare intent vs rules, SHORT-CIRCUIT on first BLOCK]

7. Data Plane → SDK
   ↓ [decision + similarities + evidence]

8. SDK enforcement
   ↓ [BLOCK → raise exception, halt execution]
```

### Component Changes

| Component | Changes |
|-----------|---------|
| **SDK** | Add v1.3 fields to IntentEvent, auto-infer layer=L4 for tool calls, track rate limit context |
| **Data Plane** | New `/enforce` endpoint, layer-based rule query, embedding cache with TTL, short-circuit evaluation |
| **Management Plane** | New `/encode/rule/{family}` endpoints, rule-to-anchor conversion functions |
| **Semantic Sandbox** | No changes (already supports anchor arrays) |

---

## Data Contracts

### IntentEvent v1.3 Schema Extensions

```python
class RateLimitContext(BaseModel):
    """Rate limit tracking context"""
    agent_id: str
    window_start: float  # Unix timestamp
    call_count: int = 0

class IntentEvent(BaseModel):
    # Existing v1.2 fields (unchanged)
    id: str
    schemaVersion: Literal["v1.2", "v1.3"] = "v1.3"
    tenantId: str
    timestamp: float
    actor: Actor
    action: Literal["read", "write", "delete", "export", "execute", "update"]
    resource: Resource
    data: Data
    risk: Risk
    context: Optional[dict] = None

    # NEW v1.3 fields
    layer: Optional[str] = None  # "L0", "L1", ..., "L6"
    tool_name: Optional[str] = None
    tool_method: Optional[str] = None
    tool_params: Optional[dict] = None
    rate_limit_context: Optional[RateLimitContext] = None
```

### Field Inference in SDK

**From `enforcement_agent` (SecureGraphProxy):**

```python
def _create_intent_from_tool_call(self, tool_call: dict) -> IntentEvent:
    tool_name = tool_call.get("name")       # Direct from LangGraph
    tool_args = tool_call.get("args")       # Direct from LangGraph
    tool_id = tool_call.get("id")           # Direct from LangGraph

    # Inferred fields
    tool_method = self._infer_tool_method(tool_name, tool_args)
    action = self.action_mapper(tool_name, tool_args)
    resource_type = self.resource_type_mapper(tool_name)
    rate_context = self._get_rate_limit_context()  # Tracked by proxy

    return IntentEvent(
        layer="L4",  # Hardcoded for MVP
        tool_name=tool_name,
        tool_method=tool_method,
        tool_params=tool_args,
        rate_limit_context=rate_context,
        # ... other fields
    )
```

**Method Inference Logic:**

```python
def _infer_tool_method(self, tool_name: str, tool_args: dict) -> str:
    # Check tool name for method keywords
    for method in ["read", "write", "query", "execute", "delete"]:
        if method in tool_name.lower():
            return method

    # Infer from primary input parameter
    if "query" in tool_args or "search" in tool_args:
        return "query"
    elif "path" in tool_args or "file" in tool_args:
        return "read" if "write" not in tool_name.lower() else "write"

    return "execute"  # Default
```

### Data Plane API

**New Enforcement Endpoint:**

```rust
// POST /enforce
Request: {
    "event": IntentEvent,
}

Response: {
    "decision": 0 | 1,  // 0=BLOCK, 1=ALLOW
    "slice_similarities": [f32; 4],
    "rules_evaluated": usize,
    "evidence": [
        {
            "rule_id": "string",
            "rule_name": "string",
            "decision": 0 | 1,
            "similarities": [f32; 4]
        }
    ]
}
```

### Management Plane API

**New Encoding Endpoints:**

```python
# POST /api/v1/encode/intent
Request: IntentEvent
Response: {
    "vector": [f32; 128]
}

# POST /api/v1/encode/rule/tool_whitelist
Request: ToolWhitelistRule
Response: RuleAnchors {
    "action_anchors": [[f32; 32]; 16],
    "action_count": usize,
    "resource_anchors": [[f32; 32]; 16],
    "resource_count": usize,
    "data_anchors": [[f32; 32]; 16],
    "data_count": usize,
    "risk_anchors": [[f32; 32]; 16],
    "risk_count": usize,
}

# POST /api/v1/encode/rule/tool_param_constraint
Request: ToolParamConstraintRule
Response: RuleAnchors (same structure)
```

---

## Rule-to-Anchor Conversion

### Reusing Phase 2 Anchor Architecture

The system already uses anchor-based encoding for DesignBoundaries. We extend this to Data Plane rules by creating family-specific anchor builders.

### ToolWhitelist Anchor Mapping

```python
def build_tool_whitelist_action_anchors(rule: ToolWhitelistRule) -> list[str]:
    """
    Maps allowed_methods to action vocabulary.

    Example:
        allowed_methods = ["query", "read"]
        →
        [
            "action is read | actor_type equals agent",
            "action is read | actor_type equals agent"
        ]
    """
    anchors = []
    method_to_action = {
        "query": "read",
        "read": "read",
        "write": "write",
        "execute": "execute",
        "delete": "delete",
    }

    for method in sorted(rule.allowed_methods):
        action = method_to_action.get(method, "execute")
        anchor = f"action is {action} | actor_type equals agent"
        anchors.append(anchor)

    if not anchors:
        anchors.append("action is execute | actor_type equals agent")

    return anchors

def build_tool_whitelist_resource_anchors(rule: ToolWhitelistRule) -> list[str]:
    """
    Maps allowed_tool_ids to resource names.

    Example:
        allowed_tool_ids = ["web_search", "database_query"]
        →
        [
            "resource_type is api | resource_name is web_search | resource_location is cloud",
            "resource_type is api | resource_name is database_query | resource_location is cloud"
        ]
    """
    anchors = []
    for tool_id in sorted(rule.allowed_tool_ids):
        anchor = f"resource_type is api | resource_name is {tool_id} | resource_location is cloud"
        anchors.append(anchor)

    if not anchors:
        # Empty whitelist = block everything
        anchors.append("resource_type is none | resource_name is blocked")

    return anchors

def build_tool_whitelist_data_anchors(rule: ToolWhitelistRule) -> list[str]:
    """Generic data anchor for tool calls."""
    return ["sensitivity is internal | pii is False | volume is single"]

def build_tool_whitelist_risk_anchors(rule: ToolWhitelistRule) -> list[str]:
    """Risk anchor (rate limit is checked syntactically, not semantically)."""
    return ["authn is required"]
```

### ToolParamConstraint Anchor Mapping

```python
def build_tool_param_constraint_action_anchors(rule: ToolParamConstraintRule) -> list[str]:
    """Encodes enforcement mode as action semantics."""
    mode = "strict" if rule.enforcement_mode == "hard" else "lenient"
    return [f"action is execute | enforcement is {mode}"]

def build_tool_param_constraint_resource_anchors(rule: ToolParamConstraintRule) -> list[str]:
    """Encodes tool_id and param_name."""
    return [
        f"resource_type is api | resource_name is {rule.tool_id}",
        f"parameter_name is {rule.param_name}"
    ]

def build_tool_param_constraint_data_anchors(rule: ToolParamConstraintRule) -> list[str]:
    """
    Converts param constraints to natural language.

    Example:
        param_type = "string"
        max_len = 100
        allowed_values = ["option1", "option2"]
        →
        [
            "parameter type is string",
            "allowed values include option1, option2",
            "maximum length is 100 characters"
        ]
    """
    anchors = []

    anchors.append(f"parameter type is {rule.param_type}")

    if rule.allowed_values:
        values_str = ", ".join(rule.allowed_values[:5])
        anchors.append(f"allowed values include {values_str}")

    if rule.max_len:
        anchors.append(f"maximum length is {rule.max_len} characters")

    if rule.min_value is not None:
        anchors.append(f"minimum value is {rule.min_value}")
    if rule.max_value is not None:
        anchors.append(f"maximum value is {rule.max_value}")

    if rule.regex:
        # Convert common patterns to semantic descriptions
        if rule.regex in ["^[a-zA-Z0-9]+$", "[a-zA-Z0-9]+"]:
            anchors.append("alphanumeric characters only")
        elif rule.regex in ["^[0-9]+$", "[0-9]+"]:
            anchors.append("numeric characters only")
        else:
            anchors.append("pattern constrained input")

    return anchors

def build_tool_param_constraint_risk_anchors(rule: ToolParamConstraintRule) -> list[str]:
    """Encodes validation strictness."""
    if rule.enforcement_mode == "hard":
        return ["authn is required | validation is strict"]
    else:
        return ["authn is required | validation is permissive"]
```

---

## Data Plane Implementation

### EnforcementEngine Architecture

```rust
pub struct EnforcementEngine {
    // Rule tables indexed by layer
    rule_tables: HashMap<LayerId, Vec<Box<dyn RuleInstance>>>,

    // Embedding cache with TTL
    embedding_cache: Arc<RwLock<EmbeddingCache>>,

    // HTTP client for Management Plane
    encoding_client: reqwest::Client,
    encoding_endpoint: String,
}

struct EmbeddingCache {
    // Map: rule_id → (RuleVector, expiry_timestamp)
    entries: HashMap<String, (RuleVector, u64)>,
    ttl_seconds: u64,  // Default: 300 (5 minutes)
}

struct RuleVector {
    action_anchors: [[f32; 32]; 16],
    action_count: usize,
    resource_anchors: [[f32; 32]; 16],
    resource_count: usize,
    data_anchors: [[f32; 32]; 16],
    data_count: usize,
    risk_anchors: [[f32; 32]; 16],
    risk_count: usize,
}
```

### Short-Circuit Enforcement Logic

```rust
impl EnforcementEngine {
    pub async fn enforce(
        &self,
        intent: IntentEvent,
    ) -> Result<EnforcementResult> {
        // 1. Parse layer from intent
        let layer_id = self.parse_layer(&intent.layer)?;

        // 2. Query all rules for this layer (sorted by priority)
        let rules = self.get_rules_for_layer(layer_id);

        if rules.is_empty() {
            // No rules = fail-closed (BLOCK)
            return Ok(EnforcementResult::block_default("No rules configured"));
        }

        // 3. Embed intent (call Management Plane once)
        let intent_vector = self.embed_intent(&intent).await?;

        // 4. Evaluate rules with SHORT-CIRCUIT
        let mut evidence = Vec::new();

        for rule in rules {
            // Get cached or embed rule
            let rule_vector = self.get_or_embed_rule(rule).await?;

            // Compare using semantic sandbox
            let result = self.sandbox.compare(
                &intent_vector,
                &rule_vector.action_anchors,
                rule_vector.action_count,
                &rule_vector.resource_anchors,
                rule_vector.resource_count,
                &rule_vector.data_anchors,
                rule_vector.data_count,
                &rule_vector.risk_anchors,
                rule_vector.risk_count,
                &rule.thresholds(),
                &rule.weights(),
                rule.decision_mode(),
                rule.global_threshold(),
            )?;

            // Record evidence
            evidence.push(RuleEvidence {
                rule_id: rule.rule_id().to_string(),
                rule_name: rule.description().unwrap_or("").to_string(),
                decision: result.decision,
                similarities: result.slice_similarities,
            });

            // SHORT CIRCUIT: First failure = immediate BLOCK
            if result.decision == 0 {
                log::info!(
                    "BLOCKED by rule '{}' (priority {}). Short-circuiting.",
                    rule.rule_id(),
                    rule.priority()
                );

                return Ok(EnforcementResult {
                    decision: 0,
                    slice_similarities: result.slice_similarities,
                    rules_evaluated: evidence.len(),
                    evidence,
                });
            }
        }

        // All rules passed - ALLOW
        log::info!("ALLOWED: All {} rules passed", rules.len());

        let avg_similarities = self.average_similarities(&evidence);

        Ok(EnforcementResult {
            decision: 1,
            slice_similarities: avg_similarities,
            rules_evaluated: evidence.len(),
            evidence,
        })
    }
}
```

### Embedding Cache (TTL-Based)

```rust
impl EmbeddingCache {
    pub fn get(&self, rule_id: &str) -> Option<RuleVector> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if let Some((vector, expiry)) = self.entries.get(rule_id) {
            if now < *expiry {
                return Some(vector.clone());
            } else {
                // Expired - remove from cache
                self.entries.remove(rule_id);
            }
        }

        None
    }

    pub fn set(&mut self, rule_id: String, vector: RuleVector) {
        let expiry = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() + self.ttl_seconds;

        self.entries.insert(rule_id, (vector, expiry));
    }
}

impl EnforcementEngine {
    async fn get_or_embed_rule(&self, rule: &Box<dyn RuleInstance>) -> Result<RuleVector> {
        // Check cache first
        {
            let cache = self.embedding_cache.read().unwrap();
            if let Some(vector) = cache.get(rule.rule_id()) {
                log::debug!("Cache HIT for rule '{}'", rule.rule_id());
                return Ok(vector);
            }
        }

        log::debug!("Cache MISS for rule '{}'", rule.rule_id());

        // Embed via Management Plane
        let family_id = rule.family_id();
        let endpoint = match family_id {
            RuleFamilyId::ToolWhitelist => "/encode/rule/tool_whitelist",
            RuleFamilyId::ToolParamConstraint => "/encode/rule/tool_param_constraint",
            _ => return Err(anyhow::anyhow!("Unsupported rule family")),
        };

        let response = self.encoding_client
            .post(&format!("{}{}", self.encoding_endpoint, endpoint))
            .json(rule)
            .send()
            .await?;

        let vector: RuleVector = response.json().await?;

        // Cache it
        {
            let mut cache = self.embedding_cache.write().unwrap();
            cache.set(rule.rule_id().to_string(), vector.clone());
        }

        Ok(vector)
    }
}
```

### Rule Loading from PostgreSQL

```rust
impl EnforcementEngine {
    pub async fn load_layer_rules(&mut self, layer_id: LayerId) -> Result<()> {
        match layer_id {
            LayerId::L4ToolGateway => {
                // Query both L4 families from DB
                let whitelist_rules = self.db.query_tool_whitelist_rules().await?;
                let param_rules = self.db.query_tool_param_constraint_rules().await?;

                let mut rules: Vec<Box<dyn RuleInstance>> = Vec::new();
                rules.extend(whitelist_rules.into_iter().map(|r| Box::new(r) as Box<dyn RuleInstance>));
                rules.extend(param_rules.into_iter().map(|r| Box::new(r) as Box<dyn RuleInstance>));

                // Sort by priority (higher = evaluated first)
                rules.sort_by_key(|r| std::cmp::Reverse(r.priority()));

                self.rule_tables.insert(layer_id, rules);

                log::info!("Loaded {} L4 rules", rules.len());
            },
            _ => {
                log::warn!("Layer {:?} not yet supported", layer_id);
            }
        }

        Ok(())
    }
}
```

---

## SDK Implementation

### Rate Limit Tracking in SecureGraphProxy

```python
class SecureGraphProxy:
    def __init__(self, ...):
        # ... existing fields ...
        self._rate_limit_tracker = {}  # {agent_id: {window_start, call_count}}
        self._rate_window_seconds = 60  # 1 minute window

    def _get_rate_limit_context(self) -> RateLimitContext:
        """Track rate limit state per agent."""
        agent_id = "agent"
        current_time = time.time()

        if agent_id not in self._rate_limit_tracker:
            self._rate_limit_tracker[agent_id] = {
                "window_start": current_time,
                "call_count": 0
            }

        tracker = self._rate_limit_tracker[agent_id]

        # Reset window if expired
        if current_time - tracker["window_start"] > self._rate_window_seconds:
            tracker["window_start"] = current_time
            tracker["call_count"] = 0

        # Increment count
        tracker["call_count"] += 1

        return RateLimitContext(
            agent_id=agent_id,
            window_start=tracker["window_start"],
            call_count=tracker["call_count"]
        )
```

### Updated Tool Call Capture

```python
def _create_intent_from_tool_call(self, tool_call: dict) -> IntentEvent:
    tool_name = tool_call.get("name", "unknown-tool")
    tool_args = tool_call.get("args", {})
    tool_id = tool_call.get("id", str(uuid4()))

    # Infer fields
    action = self.action_mapper(tool_name, tool_args)
    resource_type = self.resource_type_mapper(tool_name)
    tool_method = self._infer_tool_method(tool_name, tool_args)
    rate_context = self._get_rate_limit_context()

    # Determine PII and sensitivity
    pii = action in ["delete", "export"]
    sensitivity = ["public"] if action == "read" else ["internal"]

    # Create IntentEvent with v1.3 extensions
    event = IntentEvent(
        id=f"intent_{tool_id}",
        schemaVersion="v1.3",  # Updated
        tenantId=self.tenant_id,
        timestamp=time.time(),
        actor=Actor(id="agent", type="agent"),
        action=action,
        resource=Resource(
            type=resource_type,
            name=tool_name,
            location="cloud"
        ),
        data=Data(
            sensitivity=sensitivity,
            pii=pii,
            volume="single"
        ),
        risk=Risk(authn="required"),

        # NEW v1.3 fields
        layer="L4",
        tool_name=tool_name,
        tool_method=tool_method,
        tool_params=tool_args,
        rate_limit_context=rate_context,

        context={"tool_args": tool_args}
    )

    return event
```

---

## Performance Characteristics

### Short-Circuit Performance

```
Scenario: 10 rules in L4, rule #3 fails

Without short-circuit:
- Embed intent: 1 call (~10ms)
- Embed 10 rules: 10 cache lookups (~50ms total)
- Compare 10 times: 10 sandbox calls (~10ms)
Total: ~70ms

With short-circuit:
- Embed intent: 1 call (~10ms)
- Embed 3 rules: 3 cache lookups (~5ms)
- Compare 3 times: 3 sandbox calls (~3ms)
Total: ~18ms ✅ 74% faster
```

### Cache Hit Rates

```
Assumption: 100 tool calls, 5 unique rules

Without cache:
- 100 tool calls × 5 rules = 500 encoding calls
- 500 × 10ms = 5000ms total encoding time

With cache (TTL=300s):
- First 5 calls: 5 misses (50ms)
- Next 95 calls: cache hits (0ms)
Total: 50ms ✅ 99% faster
```

### Target Latencies

| Operation | Target | Notes |
|-----------|--------|-------|
| Intent embedding | < 10ms | Single Management Plane call |
| Rule embedding (cache hit) | < 1ms | Memory lookup |
| Rule embedding (cache miss) | < 10ms | Management Plane call + cache store |
| Semantic comparison | < 1ms | Rust sandbox per rule |
| Full enforcement (3 rules, all cached) | < 20ms | P50 target |

---

## Database Schema

### L4 Rule Tables

```sql
-- ToolWhitelist rules
CREATE TABLE l4_tool_whitelist_rules (
    rule_id VARCHAR(255) PRIMARY KEY,
    priority INT NOT NULL DEFAULT 0,
    scope_type VARCHAR(50) NOT NULL,  -- 'global' or 'agent'
    scope_agent_ids TEXT[],

    allowed_tool_ids TEXT[] NOT NULL,
    allowed_methods TEXT[] NOT NULL,
    rate_limit_per_min INT,

    enabled BOOLEAN NOT NULL DEFAULT true,
    description TEXT,
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL
);

-- ToolParamConstraint rules
CREATE TABLE l4_tool_param_constraint_rules (
    rule_id VARCHAR(255) PRIMARY KEY,
    priority INT NOT NULL DEFAULT 0,
    scope_type VARCHAR(50) NOT NULL,
    scope_agent_ids TEXT[],

    tool_id VARCHAR(255) NOT NULL,
    param_name VARCHAR(255) NOT NULL,
    param_type VARCHAR(50) NOT NULL,  -- 'string', 'int', 'float', 'bool'

    regex TEXT,
    allowed_values TEXT[],
    max_len INT,
    min_value DOUBLE PRECISION,
    max_value DOUBLE PRECISION,

    enforcement_mode VARCHAR(50) NOT NULL,  -- 'hard' or 'soft'

    enabled BOOLEAN NOT NULL DEFAULT true,
    description TEXT,
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL
);

CREATE INDEX idx_l4_whitelist_priority ON l4_tool_whitelist_rules(priority DESC);
CREATE INDEX idx_l4_param_priority ON l4_tool_param_constraint_rules(priority DESC);
```

---

## Migration Path

### Phase 1: Core Infrastructure (Week 1)

- [ ] Add v1.3 fields to IntentEvent schema (SDK + Management Plane)
- [ ] Implement layer inference in SDK (`layer="L4"` for tool calls)
- [ ] Implement rate limit tracking in `SecureGraphProxy`
- [ ] Add `/encode/intent` endpoint to Management Plane
- [ ] Add `/encode/rule/tool_whitelist` and `/encode/rule/tool_param_constraint` endpoints
- [ ] Implement rule-to-anchor conversion functions

### Phase 2: Data Plane Core (Week 1-2)

- [ ] Create Data Plane EnforcementEngine struct
- [ ] Implement `/enforce` endpoint
- [ ] Implement layer-based rule query logic
- [ ] Implement short-circuit evaluation
- [ ] Implement TTL-based embedding cache
- [ ] Wire up semantic sandbox integration

### Phase 3: Database & Rule Loading (Week 2)

- [ ] Create L4 rule tables (PostgreSQL migrations)
- [ ] Implement rule loading from DB on startup
- [ ] Implement periodic rule refresh (every 60s)
- [ ] Add rule CRUD endpoints to Control Plane (future)

### Phase 4: Testing & Validation (Week 2-3)

- [ ] Unit tests for rule-to-anchor conversion
- [ ] Integration tests for SDK → Data Plane → Management Plane flow
- [ ] Performance testing (cache hit rates, short-circuit effectiveness)
- [ ] End-to-end test with real LangGraph agent

### Phase 5: Documentation & Deployment (Week 3)

- [ ] API documentation for new endpoints
- [ ] Update SDK examples with v1.3 usage
- [ ] Deployment guide for Data Plane
- [ ] Monitoring and observability setup

---

## Future Work

### Layer Expansion

- L0 System (NetworkEgress, SidecarSpawn)
- L1 Input (InputSchema, InputSanitize)
- L2 Planner (PromptAssembly, PromptLength)
- L3 ModelIO (ModelOutputScan, ModelOutputEscalate)
- L5 RAG (RAGSource, RAGDocSensitivity)
- L6 Egress (OutputPII, OutputAudit)

### Hybrid Semantic + Syntactic Matching

For rules with precise constraints (regex, numeric bounds), add syntactic validation pass after semantic matching:

```rust
// After semantic comparison passes
if let Some(regex) = &rule.regex {
    if !regex.is_match(&intent.tool_params[param_name]) {
        return BLOCK;  // Syntactic validation failed
    }
}
```

### Multi-Layer Enforcement

Allow IntentEvents to specify multiple layers:

```python
layer: list[str] = ["L4", "L5"]  # Enforce both ToolGateway and RAG rules
```

### Rule Priority Groups

Group rules by priority tiers for more nuanced evaluation:

```sql
ALTER TABLE l4_tool_whitelist_rules ADD COLUMN priority_tier VARCHAR(50);
-- 'critical', 'high', 'medium', 'low'
```

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Enforcement latency (P50) | < 20ms | Data Plane → SDK response time |
| Enforcement latency (P99) | < 50ms | Data Plane → SDK response time |
| Cache hit rate | > 95% | Embedding cache hits / total requests |
| Short-circuit effectiveness | > 50% | Rules skipped / total rules |
| False positive rate | < 1% | Blocked legitimate calls / total calls |
| False negative rate | < 0.1% | Allowed malicious calls / total calls |

---

## Open Questions

1. **Cache Invalidation:** How to handle rule updates? (Proposed: Pub/sub notification from Control Plane → Data Plane clears cache)
2. **Multi-Tenancy:** How to isolate rules per tenant? (Proposed: Add `tenant_id` to rule tables, filter on load)
3. **Rule Conflicts:** What if two rules contradict? (Proposed: Priority-based, highest priority wins)
4. **Observability:** What metrics to expose? (Proposed: Rule evaluation counts, cache stats, latency histograms)

---

## References

- **Data Plane Rule Families:** `tupl_data_plane/tupl_dp/bridge/src/families/`
- **Current Encoding Logic:** `management-plane/app/encoding.py`
- **SDK Enforcement:** `tupl_sdk/python/tupl/agent.py` (SecureGraphProxy)
- **Semantic Sandbox:** `semantic-sandbox/src/lib.rs`

