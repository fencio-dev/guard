//! # Rule Indices Module
//!
//! Provides efficient indexing structures for fast rule lookups.
//!
//! Each rule family can have custom indexing strategies optimized for
//! their evaluation patterns. Common index types include:
//! - Agent-based: Rules scoped to specific agents
//! - Tool-based: Rules for specific tools (L4)
//! - Source-based: Rules for specific data sources (L5)
//! - Global: Rules that apply to all contexts

use crate::types::RuleInstance;
use std::collections::HashMap;
use std::sync::Arc;

// ================================================================================================
// INDEX KEY TYPES
// ================================================================================================

/// Primary index key types for rule lookups

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum IndexKey {
    ///Index by agent Id
    Agent(String),
    /// Index by tool ID (L4 families)
    Tool(String),
    /// Index by data source ID(L5 families)
    Source(String),
    /// Index by destination domain(L0 network egress)
    Domain(String),
    /// Index by sidecar image (L0 sidecar spawn)
    Image(String),
    /// Global rules (apply to all)
    Global,
}

impl IndexKey {
    /// Create an agent index key
    pub fn agent(agent_id: impl Into<String>) -> Self {
        IndexKey::Agent(agent_id.into())
    }

    /// Creates a tool index key
    pub fn tool(tool_id: impl Into<String>) -> Self {
        IndexKey::Tool(tool_id.into())
    }
    ///Create a source index key
    pub fn source(source_id: impl Into<String>) -> Self {
        IndexKey::Source(source_id.into())
    }

    /// Creates a domain index key
    pub fn domain(domain: impl Into<String>) -> Self {
        IndexKey::Domain(domain.into())
    }

    /// Creates a global index key
    pub fn global() -> Self {
        IndexKey::Global
    }
}

// ================================================================================================
// FAMILY INDICES STRUCTURE
// ================================================================================================

/// Index structure for a rule family table
///
/// Maintains multiple index types for efficient rule lookups.
/// Uses Arc to enable zero-copy access across threads.

#[derive(Clone)]
pub struct FamilyIndices {
    /// Primary index: agent ID -> rules
    pub by_agent: HashMap<String, Vec<Arc<dyn RuleInstance>>>,

    /// Secondary index: varies by family (tool, source, domain, etc.)
    pub by_secondary: HashMap<String, Vec<Arc<dyn RuleInstance>>>,

    /// Global rules (apply to all contexts)
    pub globals: Vec<Arc<dyn RuleInstance>>,

    /// Index type for secondary index
    pub secondary_type: SecondaryIndexType,
}

/// Defines what the secondary index tracks
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecondaryIndexType {
    /// No secondary index
    None,
    /// Index by tool ID (L4)
    Tool,
    /// Index by source ID (L5)
    Source,
    /// Index by domain (L0)
    Domain,
    /// Index by image name (L0)
    Image,
}

impl FamilyIndices {
    /// Creates a new empty index structure
    pub fn new(secondary_type: SecondaryIndexType) -> Self {
        FamilyIndices {
            by_agent: HashMap::new(),
            by_secondary: HashMap::new(),
            globals: Vec::new(),
            secondary_type,
        }
    }

    /// Adds a rule to the approporiate indices
    pub fn add_rule(&mut self, rule: Arc<dyn RuleInstance>) {
        let scope = rule.scope();

        // Add to global index if applicable
        if scope.is_global {
            self.globals.push(Arc::clone(&rule));
        }

        // Add to agent index
        for agent_id in &scope.agent_ids {
            self.by_agent
                .entry(agent_id.clone())
                .or_insert_with(Vec::new)
                .push(Arc::clone(&rule));
        }

        // Secondary index population is family specific and would be handled
        // by family specific logic during insertion
    }

    /// Adds a rule to the secondary index
    pub fn add_to_secondary(&mut self, key: String, rule: Arc<dyn RuleInstance>) {
        self.by_secondary
            .entry(key)
            .or_insert_with(Vec::new)
            .push(rule);
    }

    /// Queries rules by agent id
    /// Returns both agent specific rules and global rules
    /// Rules are returned in priority order (highest first)
    pub fn query_by_agent(&self, agent_id: &str) -> Vec<Arc<dyn RuleInstance>> {
        let mut results = Vec::new();
        // Add agent specific rules
        if let Some(agent_rules) = self.by_agent.get(agent_id) {
            results.extend(agent_rules.iter().map(Arc::clone));
        }

        // Add global rules
        results.extend(self.globals.iter().map(Arc::clone));

        // Sort by priority (descending)
        results.sort_by(|a, b| b.priority().cmp(&a.priority()));

        results
    }

    /// Queries rules by secondary key (tool, source, domain, etc)
    pub fn query_by_secondary(&self, key: &str) -> Vec<Arc<dyn RuleInstance>> {
        let mut results = Vec::new();

        //Add secondary specific rules
        if let Some(secondary_rules) = self.by_secondary.get(key) {
            results.extend(secondary_rules.iter().map(Arc::clone));
        }

        // Add global rules
        results.extend(self.globals.iter().map(Arc::clone));

        // Sort by priority
        results.sort_by(|a, b| b.priority().cmp(&a.priority()));

        results
    }

    /// Queries rules by both agent and secondary key
    /// Returns rules that match either condition, plus global rules.
    pub fn query_by_agent_and_secondary(
        &self,
        agent_id: &str,
        secondary_key: &str,
    ) -> Vec<Arc<dyn RuleInstance>> {
        let mut results = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        // Add agent specific rules
        if let Some(agent_rules) = self.by_agent.get(agent_id) {
            for rule in agent_rules {
                if seen_ids.insert(rule.rule_id().to_string()) {
                    results.push(Arc::clone(rule));
                }
            }
        }

        // Add secondary rules
        if let Some(secondary_rules) = self.by_secondary.get(secondary_key) {
            for rule in secondary_rules {
                if seen_ids.insert(rule.rule_id().to_string()) {
                    results.push(Arc::clone(rule));
                }
            }
        }

        // Add global rules
        for rule in &self.globals {
            if seen_ids.insert(rule.rule_id().to_string()) {
                results.push(Arc::clone(rule));
            }
        }
        // Sort by priority
        results.sort_by(|a, b| b.priority().cmp(&a.priority()));

        results
    }

    /// Returns all global rules
    pub fn query_globals(&self) -> Vec<Arc<dyn RuleInstance>> {
        let mut results = self.globals.clone();
        results.sort_by(|a, b| b.priority().cmp(&a.priority()));
        results
    }

    /// Returns all rules (no filtering)
    pub fn query_all(&self) -> Vec<Arc<dyn RuleInstance>> {
        let mut results: Vec<Arc<dyn RuleInstance>> = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        // Collect all rules from all indices
        for rules in self.by_agent.values() {
            for rule in rules {
                if seen_ids.insert(rule.rule_id().to_string()) {
                    results.push(Arc::clone(rule));
                }
            }
        }

        for rules in self.by_secondary.values() {
            for rule in rules {
                if seen_ids.insert(rule.rule_id().to_string()) {
                    results.push(Arc::clone(rule));
                }
            }
        }

        for rule in &self.globals {
            if seen_ids.insert(rule.rule_id().to_string()) {
                results.push(Arc::clone(rule));
            }
        }

        // Sort by priority
        results.sort_by(|a, b| b.priority().cmp(&a.priority()));
        results
    }

    /// Removes a rule from all indices by rule ID
    pub fn remove_rule(&mut self, rule_id: &str) -> bool {
        let mut found = false;

        // Remove from agent index
        for rules in self.by_agent.values_mut() {
            if let Some(pos) = rules.iter().position(|r| r.rule_id() == rule_id) {
                rules.remove(pos);
                found = true;
            }
        }

        // Remove from secondary index
        for rules in self.by_secondary.values_mut() {
            if let Some(pos) = rules.iter().position(|r| r.rule_id() == rule_id) {
                rules.remove(pos);
                found = true;
            }
        }

        // Remove from globals
        if let Some(pos) = self.globals.iter().position(|r| r.rule_id() == rule_id) {
            self.globals.remove(pos);
            found = true;
        }

        found
    }

    /// Clears all indices
    pub fn clear(&mut self) {
        self.by_agent.clear();
        self.by_secondary.clear();
        self.globals.clear();
    }

    /// Returns statistics about the indices
    pub fn stats(&self) -> IndexStats {
        let total_agent_rules: usize = self.by_agent.values().map(|v| v.len()).sum();
        let total_secondary_rules: usize = self.by_secondary.values().map(|v| v.len()).sum();

        IndexStats {
            agent_index_size: self.by_agent.len(),
            secondary_index_size: self.by_secondary.len(),
            global_count: self.globals.len(),
            total_agent_rules,
            total_secondary_rules,
            secondary_type: self.secondary_type,
        }
    }
}

/// Statistics about index usage
#[derive(Debug, Clone)]
pub struct IndexStats {
    /// Number of unique agents indexed
    pub agent_index_size: usize,

    /// Number of unique secondary keys indexed
    pub secondary_index_size: usize,

    /// Number of global rules
    pub global_count: usize,

    /// Total rules in agent index (with duplication)
    pub total_agent_rules: usize,

    /// Total rules in secondary index (with duplication)
    pub total_secondary_rules: usize,

    /// Type of secondary index
    pub secondary_type: SecondaryIndexType,
}
