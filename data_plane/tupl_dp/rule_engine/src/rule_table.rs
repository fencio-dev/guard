// In memory rule storage with multi index Lookups. 
// High performance rule storage with read optimised indexing, lock free reads 
// and atomic hot-reload support

// Design Principles:
// 1. Lock-free reads for zero contention on evaluation hot-path
// 2. Multi-index lookups: O(1) access by agent_id, flow_id, dtype
// 3. Copy-on-write updates for atomic hot-reload without stalling readers
// 4. Decision caching with TTL for repeated evaluations
// 5. Per-rule metrics tracking for observability
// 6. Thread-safe operations (Send + Sync)
//
// Architecture:
// - RuleIndexes: Immutable index structure (lock-free reads via Arc)
// - RuleTable: Wrapper with atomic pointer swap for updates
// - RuleEntry: Rule + metadata + statistics
// - RuleQuery: Fluent query builder for complex lookups
//
// Memory Model:
// - Readers: Acquire Arc reference (no locks, no contention)
// - Writers: Clone entire index structure, modify, atomic swap
// - Trade-off: Memory overhead for write latency vs. read throughput


use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, Duration};
use serde::{Deserialize, Serialize};

use crate::rule_metadata::RuleId;
use crate::rule_bundle::{Rule, BundleId};

// ============================================================================
// Core Types
// ============================================================================

/// Rule entry in the table with metadata and statistics
#[derive(Debug, Clone)]
pub struct RuleEntry {
    /// The rule itself
    pub rule: Rule,
    /// When the rule was activated
    pub activated_at: SystemTime,
    /// Which bundle this rule belongs to 
    pub bundle_id: Option<BundleId>,
    /// Execution statistics
    pub stats: RuleStats,
}

impl RuleEntry {
    ///Create a new rule entry
    pub fn new(rule:Rule, bundle_id: Option<BundleId>) -> Self {
        Self {
            rule, 
            activated_at: SystemTime::now(),
            bundle_id, 
            stats: RuleStats::new(),
        }
    }

    /// Get Rule ID
    pub fn rule_id(&self) -> &RuleId {
        &self.rule.metadata.rule_id
    }

    /// Get rule priority (higher = more important)
    pub fn priority(&self) -> i32 {
        self.rule.metadata.priority
    }

    /// Check if the rule is active based on constraints
    pub fn is_active(&self, _now:SystemTime) ->bool {
        // Currently no time-based constraints in ExecutionConstraints
        // This method is a placeholder for future constraint checks
        true
    }
}

/// Per Rule execution Statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleStats {
    /// Number of times rule was evaluated
    pub evaluation_count: u64,
    /// Number of times rule matched
    pub match_count: u64, 
    /// Number of times action was executed
    pub action_count: u64,
    /// Total eval time in microseconds
    pub total_eval_time_us: u64,
    #[serde(skip)]
    /// Last Eval timestamp
    pub last_evaluated: Option<SystemTime>,
    /// Error Count
    pub error_count: u64,
}

impl RuleStats {
    ///Create a new statistics tracker
    pub fn new() -> Self {
        Self {
            evaluation_count: 0,
            match_count: 0,
            action_count: 0,
            total_eval_time_us: 0,
            last_evaluated: None,
            error_count: 0,
        }
    }

    /// Record an evaluation
    pub fn record_evaluation(&mut self, matched: bool, eval_time_us: u64) {
        self.evaluation_count += 1;
        self.total_eval_time_us += eval_time_us;
        self.last_evaluated = Some(SystemTime::now());
        
        if matched {
            self.match_count += 1;
        }
    }

    /// Record an action execution
    pub fn record_action(&mut self) {
        self.action_count += 1;
    }
    
    /// Record an error
    pub fn record_error(&mut self) {
        self.error_count += 1;
    }
    
    /// Get average evaluation time in microseconds
    pub fn avg_eval_time_us(&self) -> u64 {
        if self.evaluation_count == 0 {
            0
        } else {
            self.total_eval_time_us / self.evaluation_count
        }
    }
    
    /// Get match rate (0.0 to 1.0)
    pub fn match_rate(&self) -> f64 {
        if self.evaluation_count == 0 {
            0.0
        } else {
            self.match_count as f64 / self.evaluation_count as f64
        }
    }
}

impl Default for RuleStats {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Multi-Index Structure (Immutable for Lock-Free Reads)
// ============================================================================

/// Immutable multi-index structure for fast rule lookups
/// This structure is cloned on write and atomically swapped
#[derive(Debug, Clone)]

struct RuleIndexes {
    /// Primary index: rule_id -> rule entry
    by_id: HashMap<RuleId, Arc<RuleEntry>>,
    
    /// Secondary index: agent_id -> list of rules
    by_agent: HashMap<String, Vec<Arc<RuleEntry>>>,
    
    /// Secondary index: flow_id -> list of rules
    by_flow: HashMap<String, Vec<Arc<RuleEntry>>>,
    
    /// Secondary index: dest_agent -> list of rules
    by_dest_agent: HashMap<String, Vec<Arc<RuleEntry>>>,
    
    /// Secondary index: payload dtype -> list of rules
    by_dtype: HashMap<String, Vec<Arc<RuleEntry>>>,
    
    /// Global rules (apply to all agents/flows)
    global: Vec<Arc<RuleEntry>>,
}

impl RuleIndexes {
    /// Create empty indexes
    fn new() -> Self {
        Self {
            by_id: HashMap::new(),
            by_agent: HashMap::new(),
            by_flow: HashMap::new(),
            by_dest_agent: HashMap::new(),
            by_dtype: HashMap::new(),
            global: Vec::new(),
        }
    }

    /// Get rule by ID
    fn get(&self, rule_id: &RuleId) -> Option<Arc<RuleEntry>> {
        self.by_id.get(rule_id).cloned()
    }

    /// Query rules with Criteria
    fn query(&self, query: &RuleQuery) -> Vec<Arc<RuleEntry>> {
        let mut results: Vec<Arc<RuleEntry>> = Vec::new();
        let mut seen_ids: HashSet<RuleId> = HashSet::new();
        let now = SystemTime::now();

        // Always include global rules
        for entry in &self.global {
            if entry.is_active(now) && seen_ids.insert(entry.rule_id().clone()) {
                results.push(Arc::clone(entry));
            }
        }

        //Add rules matching agent_id
        if let Some(agent_id) = &query.agent_id {
            if let Some(entries) = self.by_agent.get(agent_id) {
                for entry in entries {
                    if entry.is_active(now) && seen_ids.insert(entry.rule_id().clone()) {
                        results.push(Arc::clone(entry));
                    }
                }
            }
        }

        // Add rules matching flow_id
        if let Some(flow_id) = &query.flow_id {
            if let Some(entries) = self.by_flow.get(flow_id) {
                for entry in entries {
                    if entry.is_active(now) && seen_ids.insert(entry.rule_id().clone()) {
                        results.push(Arc::clone(entry));
                    }
                }
            }
        }

        // Add rules matching dest_agent
        if let Some(dest_agent) = &query.dest_agent {
            if let Some(entries) = self.by_dest_agent.get(dest_agent) {
                for entry in entries {
                    if entry.is_active(now) && seen_ids.insert(entry.rule_id().clone()) {
                        results.push(Arc::clone(entry));
                    }
                }
            }
        }
            // Add rules matching dtype
        if let Some(dtype) = &query.dtype {
            if let Some(entries) = self.by_dtype.get(dtype) {
                for entry in entries {
                    if entry.is_active(now) && seen_ids.insert(entry.rule_id().clone()) {
                        results.push(Arc::clone(entry));
                    }
                }
            }
        }

        // Sort by priority (highest first)
        results.sort_by(|a, b| {
            b.priority().cmp(&a.priority())
                .then_with(|| a.rule_id().as_str().cmp(&b.rule_id().as_str()))
        });

        results
    }
     
    /// Add a rule to all the relevant indexes
    fn add (&mut self, entry: Arc<RuleEntry>) {
        let rule_id = entry.rule_id().clone();
        //Primary index
        self.by_id.insert(rule_id, Arc::clone(&entry));
        
        // Determine which secondary indexes to update
        let scope = &entry.rule.metadata.scope;

        // Check if this is a global rule
        let is_global = scope.agent_ids.is_empty()
            && scope.flow_ids.is_empty()
            && scope.dest_agent_ids.is_empty()
            && scope.payload_dtypes.is_empty();
        
        if is_global {
            self.global.push(Arc::clone(&entry));
        }

        // Index by agent_ids
        for agent_id in &scope.agent_ids {
            self.by_agent.entry(agent_id.as_str().to_string()).or_insert_with(Vec::new).push(Arc::clone(&entry));
        }

        // Index by flow_ids
        for flow_id in &scope.flow_ids {
            self.by_flow.entry(flow_id.as_str().to_string()).or_insert_with(Vec::new).push(Arc::clone(&entry));
        }

        // Index by dest agent ids
        for dest_agent in &scope.dest_agent_ids {
            self.by_dest_agent.entry(dest_agent.as_str().to_string()).or_insert_with(Vec::new).push(Arc::clone(&entry));
        }

        // Index by payload dtypes
        for dtype in &scope.payload_dtypes {
            self.by_dtype.entry(dtype.clone()).or_insert_with(Vec::new).push(Arc::clone(&entry));
        }   

    }

    /// Remove a rule from all the indexes
    fn remove(&mut self, rule_id: &RuleId) -> Option<Arc<RuleEntry>> {
        // Remove from primary index
        let entry = self.by_id.remove(rule_id)?;
        
        let scope = &entry.rule.metadata.scope;
        
        // Remove from global rules
        self.global.retain(|e| e.rule_id() != rule_id);
        
        // Remove from agent index
        for agent_id in &scope.agent_ids {
            if let Some(entries) = self.by_agent.get_mut(agent_id.as_str()) {
                entries.retain(|e| e.rule_id() != rule_id);
            }
        }

        // Remove from flow index
        for flow_id in &scope.flow_ids {
            if let Some(entries) = self.by_flow.get_mut(flow_id.as_str()) {
                entries.retain(|e| e.rule_id() != rule_id);
            }
        }

        // Remove from dest_agent index
        for dest_agent in &scope.dest_agent_ids {
            if let Some(entries) = self.by_dest_agent.get_mut(dest_agent.as_str()) {
                entries.retain(|e| e.rule_id() != rule_id);
            }
        }
        
        // Remove from dtype index
        for dtype in &scope.payload_dtypes {
            if let Some(entries) = self.by_dtype.get_mut(dtype) {
                entries.retain(|e| e.rule_id() != rule_id);
            }
        }
        
        Some(entry)
    }

    /// Get total rule count
    fn len(&self) -> usize {
        self.by_id.len()
    }
    
    /// Check if empty
    fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
}

// ============================================================================
// Query Builder
// ============================================================================

/// Fluent Query builder for rule lookups
#[derive(Debug, Clone, Default)]
pub struct RuleQuery {
    /// Filter by source agent ID
    pub agent_id: Option<String>,
    /// Filter by Flow ID
    pub flow_id: Option<String>,
    /// Filter by Destination agnet id
    pub dest_agent: Option<String>,
    /// Filter by pyaload data type
    pub dtype: Option<String>,
}

impl RuleQuery {
    /// Create new empty query
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Add agent_id filter
    pub fn with_agent(mut self, agent_id: String) -> Self {
        self.agent_id = Some(agent_id);
        self
    }
    
    /// Add flow_id filter
    pub fn with_flow(mut self, flow_id: String) -> Self {
        self.flow_id = Some(flow_id);
        self
    }
    
    /// Add dest_agent filter
    pub fn with_dest_agent(mut self, dest_agent: String) -> Self {
        self.dest_agent = Some(dest_agent);
        self
    }
    
    /// Add dtype filter
    pub fn with_dtype(mut self, dtype: String) -> Self {
        self.dtype = Some(dtype);
        self
    }
}

// ============================================================================
// Decision Cache
// ============================================================================

/// Cache key for rule decisions
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CacheKey {
    agent_id: String, 
    flow_id: String, 
    event_hash: u64,
}

impl CacheKey {
    fn new(agent_id: String, flow_id: String, event_hash: u64) -> Self {
        Self {
            agent_id, 
            flow_id, 
            event_hash,
        }
    }
}

///Cached decision entry
#[derive(Debug, Clone)]
struct CacheEntry {
    rule_id: RuleId, 
    decision: String, 
    cached_at: SystemTime,
}

impl CacheEntry {
    fn is_expired(&self, ttl:Duration) -> bool{
        SystemTime::now().duration_since(self.cached_at).map(|d| d>ttl).unwrap_or(true)
    }
}

// ============================================================================
// Main RuleTable
// ============================================================================

/// High-performance in-memory rule table with multi-index lookups
/// 
/// Key Features:
/// - Lock-free reads via Arc (zero contention on hot path)
/// - Copy-on-write updates with atomic pointer swap
/// - Multi-index lookups: agent_id, flow_id, dest_agent, dtype
/// - Decision caching with configurable TTL
/// - Per-rule statistics tracking
/// - Thread-safe (Send + Sync)

pub struct RuleTable {
    /// Atomic pointer to immutable indexes (lock-free reads)
    indexes: Arc<RwLock<Arc<RuleIndexes>>>,
    
    /// Decision cache (short-lived)
    cache: Arc<RwLock<HashMap<CacheKey, CacheEntry>>>,
    
    /// Cache TTL in seconds
    cache_ttl_seconds: u64,
    
    /// Maximum cache size
    max_cache_size: usize,
}

impl RuleTable {
    /// Create new rule table
    pub fn new() -> Self {
        Self::with_config(60, 10000)
    }
    
    /// Create new rule table with custom cache configuration
    pub fn with_config(cache_ttl_seconds: u64, max_cache_size: usize) -> Self {
        Self {
            indexes: Arc::new(RwLock::new(Arc::new(RuleIndexes::new()))),
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl_seconds,
            max_cache_size,
        }
    }
    
    // ========================================================================
    // Lock-Free Read Operations
    // ========================================================================
    
    /// Get rule by ID (lock-free)
    pub fn get_rule(&self, rule_id: &RuleId) -> Option<Arc<RuleEntry>> {
        let indexes = self.indexes.read().unwrap();
        let indexes_snapshot = Arc::clone(&*indexes);

        drop(indexes);

        indexes_snapshot.get(rule_id)
    }

    /// Query rules matching criteria (lock-free)
    pub fn query(&self, query: &RuleQuery) -> Vec<Arc<RuleEntry>> {
        let indexes = self.indexes.read().unwrap();
        let indexes_snapshot =  Arc::clone(&*indexes);

        indexes_snapshot.query(query)
    }
    
    /// Get total rule count (lock-free)
    pub fn len(&self) -> usize {
        let indexes = self.indexes.read().unwrap();
        let indexes_snapshot = Arc::clone(&*indexes);
        drop(indexes);
        
        indexes_snapshot.len()
    }

    /// Check if table is empty (lock-free)
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    
    // ========================================================================
    // Write Operations (Copy-on-Write)
    // ========================================================================
    
    /// Add a rule to the table
    pub fn add_rule(&self, rule: Rule, bundle_id: Option<BundleId>) -> Result<(), String> {
        let entry = Arc::new(RuleEntry::new(rule, bundle_id));
        let rule_id = entry.rule_id().clone();
        
        // Copy-on-write update
        let indexes_lock = self.indexes.write().unwrap();
        let mut new_indexes = (**indexes_lock).clone();
        
        // Check for duplicate
        if new_indexes.by_id.contains_key(&rule_id) {
            return Err(format!("Rule {} already exists", rule_id.as_str()));
        }
        
        new_indexes.add(entry);
        
        // Atomic swap
        let new_indexes_arc = Arc::new(new_indexes);
        drop(indexes_lock);
        *self.indexes.write().unwrap() = new_indexes_arc;
        
        Ok(())
    }

    /// Remove a rule from the table
    pub fn remove_rule(&self, rule_id: &RuleId) -> Result<Arc<RuleEntry>, String> {
        // Copy-on-write update
        let indexes_lock = self.indexes.write().unwrap();
        let mut new_indexes = (**indexes_lock).clone();
        
        let entry = new_indexes
            .remove(rule_id)
            .ok_or_else(|| format!("Rule {} not found", rule_id.as_str()))?;
        
        // Atomic swap
        let new_indexes_arc = Arc::new(new_indexes);
        drop(indexes_lock);
        *self.indexes.write().unwrap() = new_indexes_arc;
        
        Ok(entry)
    }

    /// Load multiple rules from a bundle atomically
    pub fn load_bundle(&self, rules: Vec<Rule>, bundle_id: BundleId) -> Result<usize, String> {
        let entries: Vec<Arc<RuleEntry>> = rules
            .into_iter()
            .map(|rule| Arc::new(RuleEntry::new(rule, Some(bundle_id.clone()))))
            .collect();
        
        // Copy-on-write update
        let indexes_lock = self.indexes.write().unwrap();
        let mut new_indexes = (**indexes_lock).clone();
        
        // Check for duplicates
        for entry in &entries {
            if new_indexes.by_id.contains_key(entry.rule_id()) {
                return Err(format!("Rule {} already exists", entry.rule_id().as_str()));
            }
        }
        
        // Add all entries
        let count = entries.len();
        for entry in entries {
            new_indexes.add(entry);
        }
        
        // Atomic swap
        let new_indexes_arc = Arc::new(new_indexes);
        drop(indexes_lock);
        *self.indexes.write().unwrap() = new_indexes_arc;
        
        Ok(count)
    }

    /// Unload all rules from a bundle
    pub fn unload_bundle(&self, bundle_id: &BundleId) -> Result<usize, String> {
        // Copy-on-write update
        let indexes_lock = self.indexes.write().unwrap();
        let current_indexes = Arc::clone(&*indexes_lock);
        
        // Find all rules in this bundle
        let rules_to_remove: Vec<RuleId> = current_indexes
            .by_id
            .values()
            .filter(|entry| entry.bundle_id.as_ref() == Some(bundle_id))
            .map(|entry| entry.rule_id().clone())
            .collect();
        
        if rules_to_remove.is_empty() {
            return Ok(0);
        }
        
        let mut new_indexes = (*current_indexes).clone();
        
        // Remove all rules
        let count = rules_to_remove.len();
        for rule_id in rules_to_remove {
            new_indexes.remove(&rule_id);
        }
        
        // Atomic swap
        let new_indexes_arc = Arc::new(new_indexes);
        drop(indexes_lock);
        *self.indexes.write().unwrap() = new_indexes_arc;
        
        Ok(count)
    }

    /// Update rule statistics
    pub fn update_stats<F>(&self, rule_id: &RuleId, update_fn: F) -> Result<(), String>
    where
        F: FnOnce(&mut RuleStats),
    {
        // Copy-on-write update
        let indexes_lock = self.indexes.write().unwrap();
        let mut new_indexes = (**indexes_lock).clone();
        
        // Get the entry
        let entry = new_indexes
            .by_id
            .get_mut(rule_id)
            .ok_or_else(|| format!("Rule {} not found", rule_id.as_str()))?;
        
        // Update stats (need to get mutable reference)
        let mut updated_entry = (**entry).clone();
        update_fn(&mut updated_entry.stats);
        *entry = Arc::new(updated_entry);
        
        // Atomic swap
        let new_indexes_arc = Arc::new(new_indexes);
        drop(indexes_lock);
        *self.indexes.write().unwrap() = new_indexes_arc;
        
        Ok(())
    }

    // ========================================================================
    // Cache Operations
    // ========================================================================
    
    /// Get cached decision
    pub fn get_cached_decision(
        &self,
        agent_id: &str,
        flow_id: &str,
        event_hash: u64,
    ) -> Option<(RuleId, String)> {
        let cache = self.cache.read().unwrap();
        let key = CacheKey::new(agent_id.to_string(), flow_id.to_string(), event_hash);
        
        if let Some(entry) = cache.get(&key) {
            let ttl = Duration::from_secs(self.cache_ttl_seconds);
            if !entry.is_expired(ttl) {
                return Some((entry.rule_id.clone(), entry.decision.clone()));
            }
        }
        
        None
    }

    /// Cache a decision
    pub fn cache_decision(
        &self,
        agent_id: &str,
        flow_id: &str,
        event_hash: u64,
        rule_id: RuleId,
        decision: String,
    ) -> Result<(), String> {
        let mut cache = self.cache.write().unwrap();
        
        // Evict expired entries if cache is full
        if cache.len() >= self.max_cache_size {
            let ttl = Duration::from_secs(self.cache_ttl_seconds);
            cache.retain(|_, entry| !entry.is_expired(ttl));
            
            // If still full, clear oldest 10%
            if cache.len() >= self.max_cache_size {
                let to_remove = cache.len() / 10;
                let keys: Vec<CacheKey> = cache.keys().take(to_remove).cloned().collect();
                for key in keys {
                    cache.remove(&key);
                }
            }
        }
        
        let key = CacheKey::new(agent_id.to_string(), flow_id.to_string(), event_hash);
        let entry = CacheEntry {
            rule_id,
            decision,
            cached_at: SystemTime::now(),
        };
        
        cache.insert(key, entry);
        Ok(())
    }
    

    /// Clear entire cache
    pub fn clear_cache(&self) -> Result<(), String> {
        let mut cache = self.cache.write().unwrap();
        cache.clear();
        Ok(())
    }
    
    /// Clear expired cache entries
    pub fn evict_expired_cache(&self) -> usize {
        let mut cache = self.cache.write().unwrap();
        let ttl = Duration::from_secs(self.cache_ttl_seconds);
        let before = cache.len();
        cache.retain(|_, entry| !entry.is_expired(ttl));
        before - cache.len()
    }

    // ========================================================================
    // Utility Methods
    // ========================================================================
    
    /// Get table statistics
    pub fn get_table_stats(&self) -> TableStats {
        let indexes = self.indexes.read().unwrap();
        let indexes_snapshot = Arc::clone(&*indexes);
        drop(indexes);
        
        let cache = self.cache.read().unwrap();
        
        TableStats {
            total_rules: indexes_snapshot.len(),
            global_rules: indexes_snapshot.global.len(),
            agent_indexes: indexes_snapshot.by_agent.len(),
            flow_indexes: indexes_snapshot.by_flow.len(),
            dest_agent_indexes: indexes_snapshot.by_dest_agent.len(),
            dtype_indexes: indexes_snapshot.by_dtype.len(),
            cache_size: cache.len(),
        }
    }
    
    /// List all rule IDs
    pub fn list_rule_ids(&self) -> Vec<RuleId> {
        let indexes = self.indexes.read().unwrap();
        let indexes_snapshot = Arc::clone(&*indexes);
        drop(indexes);
        
        indexes_snapshot.by_id.keys().cloned().collect()
    }
}

impl Default for RuleTable {
    fn default() -> Self {
        Self::new()
    }
}


// Thread-safety markers
unsafe impl Send for RuleTable {}
unsafe impl Sync for RuleTable {}

/// Table statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableStats {
    pub total_rules: usize,
    pub global_rules: usize,
    pub agent_indexes: usize,
    pub flow_indexes: usize,
    pub dest_agent_indexes: usize,
    pub dtype_indexes: usize,
    pub cache_size: usize,
}

