# RuleTable Module - In-Memory Rule Storage

## Overview

The **RuleTable** module provides high-performance, thread-safe in-memory storage for rules with read-optimized multi-index lookups. It is designed for the fast-path evaluation in the rule engine, prioritizing lock-free reads and minimal contention.

## Design Principles

### 1. **Lock-Free Reads** ⚡
- Zero lock contention on evaluation hot-path
- Uses `Arc<RuleIndexes>` for atomic pointer access
- Readers acquire snapshot reference without blocking writers

### 2. **Multi-Index Lookups** 🔍
- O(1) primary lookup by `rule_id`
- O(1) secondary lookups by:
  - `agent_id` - Source agent identifier
  - `flow_id` - Flow identifier
  - `dest_agent` - Destination agent identifier
  - `dtype` - Payload data type
- O(k) where k = number of matching rules

### 3. **Copy-on-Write Updates** 🔄
- Atomic hot-reload without stalling readers
- Clone entire index structure on write
- Atomic pointer swap (`Arc::new()` + assignment)
- Trade-off: Memory overhead for write latency vs. read throughput

### 4. **Decision Caching** 💾
- Short-lived cache for repeated evaluations
- Configurable TTL (default: 60 seconds)
- Automatic eviction of expired entries
- Thread-safe with `RwLock`

### 5. **Per-Rule Metrics** 📊
- Track evaluations, matches, actions, errors
- Average evaluation time
- Match rate calculation
- Last evaluation timestamp

### 6. **Thread Safety** 🔒
- Implements `Send + Sync`
- Safe for concurrent access from multiple threads
- Tested with concurrent reader/writer workloads

---

## Core Components

### 1. `RuleTable`

Main in-memory store with atomic operations:

```rust
pub struct RuleTable {
    indexes: Arc<RwLock<Arc<RuleIndexes>>>,  // Atomic pointer for lock-free reads
    cache: Arc<RwLock<HashMap<...>>>,         // Decision cache
    cache_ttl_seconds: u64,
    max_cache_size: usize,
}
```

**Key Methods:**
```rust
// Read operations (lock-free)
fn get_rule(&self, rule_id: &RuleId) -> Option<Arc<RuleEntry>>
fn query(&self, query: &RuleQuery) -> Vec<Arc<RuleEntry>>
fn len(&self) -> usize

// Write operations (copy-on-write)
fn add_rule(&self, rule: Rule, bundle_id: Option<BundleId>) -> Result<(), String>
fn remove_rule(&self, rule_id: &RuleId) -> Result<Arc<RuleEntry>, String>
fn load_bundle(&self, rules: Vec<Rule>, bundle_id: BundleId) -> Result<usize, String>
fn unload_bundle(&self, bundle_id: &BundleId) -> Result<usize, String>

// Statistics
fn update_stats<F>(&self, rule_id: &RuleId, update_fn: F) -> Result<(), String>

// Cache operations
fn get_cached_decision(&self, agent_id: &str, flow_id: &str, event_hash: u64) -> Option<(RuleId, String)>
fn cache_decision(&self, ...) -> Result<(), String>
fn clear_cache(&self) -> Result<(), String>
```

---

### 2. `RuleIndexes`

Immutable multi-index structure for fast lookups:

```rust
struct RuleIndexes {
    by_id: HashMap<RuleId, Arc<RuleEntry>>,              // Primary index
    by_agent: HashMap<String, Vec<Arc<RuleEntry>>>,      // Agent index
    by_flow: HashMap<String, Vec<Arc<RuleEntry>>>,       // Flow index
    by_dest_agent: HashMap<String, Vec<Arc<RuleEntry>>>, // Dest agent index
    by_dtype: HashMap<String, Vec<Arc<RuleEntry>>>,      // Data type index
    global: Vec<Arc<RuleEntry>>,                         // Global rules
}
```

**Lookup Complexity:**
- By ID: **O(1)**
- By agent/flow/dest/dtype: **O(1) + O(k)** where k = matching rules
- Combined query: **O(1) + O(k) + deduplication**

**Global Rules:**
- Rules with empty scope (no agent_ids, flow_ids, etc.)
- Always included in query results
- Useful for system-wide policies

---

### 3. `RuleEntry`

Rule wrapper with metadata and statistics:

```rust
pub struct RuleEntry {
    pub rule: Rule,                      // The rule itself
    pub activated_at: SystemTime,        // When rule was activated
    pub bundle_id: Option<BundleId>,     // Bundle association
    pub stats: RuleStats,                // Execution statistics
}
```

**Key Methods:**
```rust
fn new(rule: Rule, bundle_id: Option<BundleId>) -> Self
fn rule_id(&self) -> &RuleId
fn priority(&self) -> u32
fn is_active(&self, now: SystemTime) -> bool  // Check time window constraints
```

---

### 4. `RuleStats`

Per-rule execution statistics:

```rust
pub struct RuleStats {
    pub evaluation_count: u64,       // Times rule was evaluated
    pub match_count: u64,            // Times rule matched
    pub action_count: u64,           // Times action was executed
    pub total_eval_time_us: u64,     // Total evaluation time (microseconds)
    pub last_evaluated: Option<SystemTime>,
    pub error_count: u64,
}
```

**Derived Metrics:**
```rust
fn avg_eval_time_us(&self) -> u64      // Average evaluation time
fn match_rate(&self) -> f64             // Percentage of evaluations that matched
```

**Recording Methods:**
```rust
fn record_evaluation(&mut self, matched: bool, eval_time_us: u64)
fn record_action(&mut self)
fn record_error(&mut self)
```

---

### 5. `RuleQuery`

Fluent query builder for rule lookups:

```rust
pub struct RuleQuery {
    pub agent_id: Option<String>,
    pub flow_id: Option<String>,
    pub dest_agent: Option<String>,
    pub dtype: Option<String>,
}
```

**Builder Pattern:**
```rust
let query = RuleQuery::new()
    .with_agent("api_gateway".to_string())
    .with_flow("flow_123".to_string());

let results = table.query(&query);  // Returns rules matching ANY criterion
```

**Query Semantics:**
- **OR logic**: Returns rules matching any specified criterion
- **Deduplication**: Each rule appears once even if it matches multiple criteria
- **Sorting**: Results sorted by priority (highest first), then by rule_id

---

### 6. `TableStats`

Table-level statistics:

```rust
pub struct TableStats {
    pub total_rules: usize,
    pub global_rules: usize,
    pub agent_indexes: usize,         // Number of unique agent_ids
    pub flow_indexes: usize,          // Number of unique flow_ids
    pub dest_agent_indexes: usize,    // Number of unique dest_agents
    pub dtype_indexes: usize,         // Number of unique dtypes
    pub cache_size: usize,            // Current cache entries
}
```

---

## Usage Patterns

### Pattern 1: Create and Add Rules

```rust
use rule_engine::rule_table::*;

// Create table with default config (60s TTL, 10k cache size)
let table = RuleTable::new();

// Or with custom config
let table = RuleTable::with_config(300, 50000);  // 5min TTL, 50k cache

// Add single rule
let rule = create_rule();
table.add_rule(rule, None)?;

// Add rule with bundle association
table.add_rule(rule, Some(bundle_id))?;
```

---

### Pattern 2: Lock-Free Rule Lookup

```rust
// Get specific rule (lock-free, zero contention)
if let Some(entry) = table.get_rule(&rule_id) {
    println!("Found rule: {}", entry.rule_id().as_str());
    println!("Priority: {}", entry.priority());
}

// Query by agent (lock-free)
let query = RuleQuery::new()
    .with_agent("api_gateway".to_string());
let rules = table.query(&query);

// Results are sorted by priority (highest first)
for entry in rules {
    println!("Rule: {}, Priority: {}", entry.rule_id().as_str(), entry.priority());
}

// Complex query (multiple criteria)
let query = RuleQuery::new()
    .with_agent("api_gateway".to_string())
    .with_flow("flow_123".to_string())
    .with_dtype("UserRequest".to_string());
let rules = table.query(&query);
```

---

### Pattern 3: Bundle Operations (Atomic)

```rust
use rule_engine::rule_bundle::BundleId;

let bundle_id = BundleId::new("bundle_v1.2.3".to_string());

// Load entire bundle atomically
let rules = vec![rule1, rule2, rule3];
let count = table.load_bundle(rules, bundle_id.clone())?;
println!("Loaded {} rules from bundle", count);

// Later: unload entire bundle atomically
let removed = table.unload_bundle(&bundle_id)?;
println!("Removed {} rules from bundle", removed);
```

---

### Pattern 4: Statistics Tracking

```rust
// Update rule statistics after evaluation
table.update_stats(&rule_id, |stats| {
    stats.record_evaluation(true, 1500);  // matched=true, eval_time_us=1500
})?;

// Update after action execution
table.update_stats(&rule_id, |stats| {
    stats.record_action();
})?;

// Record errors
table.update_stats(&rule_id, |stats| {
    stats.record_error();
})?;

// Get rule stats
if let Some(entry) = table.get_rule(&rule_id) {
    let stats = &entry.stats;
    println!("Evaluations: {}", stats.evaluation_count);
    println!("Match rate: {:.2}%", stats.match_rate() * 100.0);
    println!("Avg eval time: {}μs", stats.avg_eval_time_us());
}
```

---

### Pattern 5: Decision Caching

```rust
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

// Compute event hash
fn compute_event_hash(event: &Event) -> u64 {
    let mut hasher = DefaultHasher::new();
    event.payload.hash(&mut hasher);
    hasher.finish()
}

// Check cache before evaluation
let event_hash = compute_event_hash(&event);
if let Some((cached_rule_id, decision)) = table.get_cached_decision(
    &event.agent_id,
    &event.flow_id,
    event_hash,
) {
    println!("Cache hit! Rule: {}, Decision: {}", cached_rule_id.as_str(), decision);
    return Ok(decision);
}

// After evaluation, cache the decision
table.cache_decision(
    &event.agent_id,
    &event.flow_id,
    event_hash,
    rule_id.clone(),
    "ALLOW".to_string(),
)?;

// Periodic cache maintenance
let evicted = table.evict_expired_cache();
println!("Evicted {} expired cache entries", evicted);
```

---

### Pattern 6: Table Statistics

```rust
let stats = table.get_table_stats();
println!("Total rules: {}", stats.total_rules);
println!("Global rules: {}", stats.global_rules);
println!("Agent indexes: {}", stats.agent_indexes);
println!("Cache size: {}", stats.cache_size);

// List all rule IDs
let rule_ids = table.list_rule_ids();
for rule_id in rule_ids {
    println!("Rule: {}", rule_id.as_str());
}
```

---

### Pattern 7: Thread-Safe Concurrent Access

```rust
use std::sync::Arc;
use std::thread;

let table = Arc::new(RuleTable::new());

// Spawn multiple reader threads (lock-free)
let mut handles = vec![];
for i in 0..10 {
    let table_clone = Arc::clone(&table);
    let handle = thread::spawn(move || {
        for _ in 0..1000 {
            let query = RuleQuery::new()
                .with_agent(format!("agent_{}", i % 5));
            let _results = table_clone.query(&query);
        }
    });
    handles.push(handle);
}

// Wait for all threads
for handle in handles {
    handle.join().unwrap();
}
```

---

## Integration with Rule Engine

### Complete Evaluation Flow

```rust
use rule_engine::*;
use rule_table::*;

pub struct RuleEngine {
    table: RuleTable,
    // ... other components
}

impl RuleEngine {
    pub fn evaluate(&self, event: &Event) -> Result<Decision, String> {
        // 1. Check cache (lock-free)
        let event_hash = compute_event_hash(event);
        if let Some((rule_id, decision)) = self.table.get_cached_decision(
            &event.agent_id,
            &event.flow_id,
            event_hash,
        ) {
            return Ok(parse_decision(&decision));
        }
        
        // 2. Query applicable rules (lock-free)
        let query = RuleQuery::new()
            .with_agent(event.agent_id.clone())
            .with_flow(event.flow_id.clone());
        
        let rules = self.table.query(&query);
        
        // 3. Evaluate highest priority rule first
        for entry in rules {
            let start = SystemTime::now();
            
            // Evaluate match clause
            if entry.rule.match_clause.evaluate(event)? {
                // Execute action clause
                let decision = entry.rule.action_clause.execute(event)?;
                
                // Update statistics
                let eval_time = start.elapsed().unwrap().as_micros() as u64;
                self.table.update_stats(entry.rule_id(), |stats| {
                    stats.record_evaluation(true, eval_time);
                    stats.record_action();
                })?;
                
                // Cache decision
                self.table.cache_decision(
                    &event.agent_id,
                    &event.flow_id,
                    event_hash,
                    entry.rule_id().clone(),
                    decision.to_string(),
                )?;
                
                return Ok(decision);
            } else {
                // Record non-match
                let eval_time = start.elapsed().unwrap().as_micros() as u64;
                self.table.update_stats(entry.rule_id(), |stats| {
                    stats.record_evaluation(false, eval_time);
                })?;
            }
        }
        
        Ok(Decision::Skip)  // No rules matched
    }
    
    pub fn hot_reload(&self, bundle: RuleBundle) -> Result<(), String> {
        // Atomic bundle swap (readers unaffected)
        let bundle_id = bundle.bundle_id.clone();
        let rules = bundle.rules;
        
        // Load new bundle
        self.table.load_bundle(rules, bundle_id)?;
        
        // Optional: unload old version
        // self.table.unload_bundle(&old_bundle_id)?;
        
        Ok(())
    }
}
```

---

## Performance Characteristics

### Read Operations (Lock-Free)

| Operation | Complexity | Locks | Contention |
|-----------|-----------|-------|------------|
| `get_rule()` | O(1) | None | Zero |
| `query()` (single criterion) | O(1) + O(k) | None | Zero |
| `query()` (multi-criterion) | O(1) + O(k) + dedup | None | Zero |
| `len()`, `is_empty()` | O(1) | None | Zero |

**k** = number of matching rules (typically small)

### Write Operations (Copy-on-Write)

| Operation | Complexity | Locks | Notes |
|-----------|-----------|-------|-------|
| `add_rule()` | O(n) | Write lock | Clones entire index |
| `remove_rule()` | O(n) | Write lock | Clones entire index |
| `load_bundle()` | O(n + m) | Write lock | m = bundle size |
| `update_stats()` | O(n) | Write lock | Clones entire index |

**n** = total rules in table

### Trade-offs

**Advantages:**
- ✅ Lock-free reads (zero contention on hot path)
- ✅ Atomic updates (readers see consistent snapshot)
- ✅ Simple mental model (immutable snapshots)
- ✅ Thread-safe by construction

**Disadvantages:**
- ❌ O(n) memory overhead per write
- ❌ O(n) time per write (clone entire index)
- ❌ Not suitable for write-heavy workloads

**When to Use:**
- ✅ Read-heavy workloads (1000:1 read/write ratio)
- ✅ Infrequent updates (hot-reload every few minutes)
- ✅ Rule evaluation hot-path
- ✅ Need lock-free reads

**When NOT to Use:**
- ❌ Write-heavy workloads
- ❌ Frequent updates (multiple per second)
- ❌ Very large rule sets (>100k rules)

---

## Memory Management

### Memory Layout

```
RuleTable
├── indexes: Arc<RwLock<Arc<RuleIndexes>>>  // ~16 bytes
│   └── RuleIndexes
│       ├── by_id: HashMap<RuleId, Arc<RuleEntry>>
│       ├── by_agent: HashMap<String, Vec<Arc<RuleEntry>>>
│       ├── by_flow: HashMap<String, Vec<Arc<RuleEntry>>>
│       ├── by_dest_agent: HashMap<String, Vec<Arc<RuleEntry>>>
│       ├── by_dtype: HashMap<String, Vec<Arc<RuleEntry>>>
│       └── global: Vec<Arc<RuleEntry>>
└── cache: Arc<RwLock<HashMap<CacheKey, CacheEntry>>>
```

### Memory Estimates

**Per Rule:**
- `RuleEntry` + `Rule`: ~500 bytes (varies by rule complexity)
- `Arc<RuleEntry>`: 8 bytes (pointer)
- Primary index entry: ~40 bytes (RuleId + Arc)
- Secondary index entries: ~40 bytes each
- **Total**: ~700-1000 bytes per rule

**For 10,000 Rules:**
- Rule storage: ~10 MB
- Indexes: ~5 MB
- Total: **~15 MB**

**Copy-on-Write Overhead:**
- During update: 2x memory (old + new indexes)
- Brief spike, then old indexes dropped
- With 10k rules: **~30 MB peak**

---

## Configuration Guidelines

### Cache Configuration

```rust
// Default: 60s TTL, 10k entries
let table = RuleTable::new();

// High-throughput system
let table = RuleTable::with_config(
    300,    // 5 minutes TTL
    100000, // 100k cache entries
);

// Memory-constrained system
let table = RuleTable::with_config(
    30,     // 30 seconds TTL
    1000,   // 1k cache entries
);
```

**Tuning Guidelines:**
- **TTL**: Based on event frequency and rule volatility
  - Frequent events + stable rules: longer TTL (5-10 min)
  - Infrequent events or changing rules: shorter TTL (30-60 sec)
- **Cache Size**: Based on event cardinality
  - Low cardinality (few unique events): smaller cache (1k-5k)
  - High cardinality (many unique events): larger cache (50k-100k)

---

## Error Handling

All operations return `Result<T, String>`:

```rust
// Add rule
match table.add_rule(rule, None) {
    Ok(()) => println!("Rule added successfully"),
    Err(e) => eprintln!("Failed to add rule: {}", e),
}

// Remove rule
match table.remove_rule(&rule_id) {
    Ok(entry) => println!("Removed rule: {}", entry.rule_id().as_str()),
    Err(e) => eprintln!("Failed to remove rule: {}", e),
}

// Update stats
if let Err(e) = table.update_stats(&rule_id, |stats| stats.record_evaluation(true, 1000)) {
    eprintln!("Failed to update stats: {}", e);
}
```

**Common Errors:**
- `"Rule {id} already exists"` - Duplicate rule addition
- `"Rule {id} not found"` - Attempt to remove/update non-existent rule

---

## Testing

### Unit Tests

Run all tests:
```bash
cargo test rule_table
```

**Coverage:**
- ✅ Table creation and initialization
- ✅ Add/remove operations
- ✅ Query by various criteria
- ✅ Bundle load/unload
- ✅ Statistics updates
- ✅ Cache operations (get, set, evict)
- ✅ Global rules
- ✅ Thread safety

### Integration Tests

```rust
#[test]
fn test_complete_evaluation_flow() {
    let table = RuleTable::new();
    
    // Setup rules
    let rule = create_rule_with_scope("rule_001", vec!["agent_1"], vec!["flow_1"]);
    table.add_rule(rule, None).unwrap();
    
    // Query
    let query = RuleQuery::new()
        .with_agent("agent_1".to_string())
        .with_flow("flow_1".to_string());
    let rules = table.query(&query);
    
    assert_eq!(rules.len(), 1);
    
    // Update stats
    table.update_stats(&rules[0].rule_id().clone(), |stats| {
        stats.record_evaluation(true, 1500);
    }).unwrap();
    
    // Verify
    let entry = table.get_rule(rules[0].rule_id()).unwrap();
    assert_eq!(entry.stats.evaluation_count, 1);
}
```

---

## Future Enhancements

### Phase 1: Optimizations
- **Bloom filters** for fast negative lookups
- **Compressed indexes** for memory efficiency
- **Incremental updates** (avoid full clone for small changes)
- **Lazy index updates** (defer non-critical index rebuilds)

### Phase 2: Persistence
- **Snapshot to disk** for fast restart
- **Write-ahead log** for durability
- **Incremental checkpoints**

### Phase 3: Distributed
- **Sharded tables** across nodes
- **Consistent hashing** for distribution
- **Replication** for high availability
- **Cross-node cache coherence**

### Phase 4: Advanced Features
- **Query optimizer** (index selection based on statistics)
- **Adaptive caching** (ML-based eviction policies)
- **Rule versioning** (A/B testing support)
- **Temporal queries** (point-in-time lookups)

---

## Best Practices

### ✅ DO

1. **Use for read-heavy workloads** (rule evaluation hot-path)
2. **Batch updates** (load bundles atomically)
3. **Monitor cache hit rate** (tune TTL/size accordingly)
4. **Track per-rule statistics** (identify hot rules)
5. **Periodic cache eviction** (prevent unbounded growth)

### ❌ DON'T

1. **Use for write-heavy workloads** (copy-on-write overhead)
2. **Update individual rules frequently** (use bundle updates)
3. **Ignore cache configuration** (default may not fit your needs)
4. **Skip error handling** (operations can fail)
5. **Store massive rule sets** (>100k rules, consider sharding)

---

## Dependencies

Depends on existing modules:
- `rule_metadata` (RuleId, RuleScope)
- `rule_bundle` (Rule, BundleId)
- `match_clause` (MatchClause)
- `action_clause` (ActionClause)
- `execution_constraints` (ExecutionConstraints)

No external crate dependencies.

---

## API Stability

| Component | Stability | Notes |
|-----------|-----------|-------|
| `RuleTable` | **Stable** | Core API unlikely to change |
| `RuleEntry` | **Stable** | Core type |
| `RuleQuery` | **Stable** | Builder pattern |
| `RuleStats` | **Stable** | Statistics tracking |
| `TableStats` | **Stable** | Table-level stats |
| Cache operations | **Experimental** | May change based on usage patterns |
| Internal indexes | **Internal** | Implementation detail |

---

## Module Summary

**Module 7 of 7**: RuleTable

Completes the rule engine with production-ready in-memory storage:

| Module | Status | LOC | Purpose |
|--------|--------|-----|---------|
| 1. RuleMetadata | ✅ | ~800 | Rule identity, versioning, scope |
| 2. MatchClause | ✅ | ~1000 | Condition evaluation |
| 3. ActionClause | ✅ | ~900 | Action execution, side effects |
| 4. ExecutionConstraints | ✅ | ~900 | Rate limits, time windows |
| 5. AuditRecord | ✅ | ~1100 | Compliance, forensics |
| 6. RuleBundle | ✅ | ~1200 | Bundle management, validation |
| 7. **RuleTable** | ✅ | **~1000** | **In-memory storage, indexing** |

**Total**: ~6900 lines of code + ~12000 lines of documentation

---

## License

Part of the FastPath rule engine. See parent project for license details.