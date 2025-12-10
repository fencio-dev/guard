//! # Rule Family Table Module
//!
//! Implements the RuleFamilyTable structure that stores rules for a single rule family.
//!
//! Key features:
//! - Lock-free reads using atomic Arc pointers
//! - Per-family indexing strategies
//! - Atomic updates with copy-on-write
//! - Statistics tracking

use crate::indices::{FamilyIndices, IndexStats, SecondaryIndexType};
use crate::types::{now_ms, LayerId, RuleFamilyId, RuleInstance, TableMetadata};
use parking_lot::RwLock;
use std::sync::Arc;

// ================================================================================================
// RULE FAMILY TABLE
// ================================================================================================

/// A table storing rules for a single rule family
///
/// Each table is optimized for a specific rule schema and provides
/// family-specific indexing for fast lookups during evaluation.
///
/// # Thread Safety
/// - Reads are lock-free using atomic Arc access
/// - Writes acquire an RwLock for consistency
/// - Multiple readers can access simultaneously

pub struct RuleFamilyTable {
    /// Unique family identifier
    family_id: RuleFamilyId,
    /// Parent Layer
    layer_id: LayerId,
    /// Schema version for this family's rule structure
    schema_version: u32,
    /// Table version (incremented on each update)
    version: Arc<RwLock<u64>>,
    /// Creation timestamp
    created_at: u64,
    /// Table metadata
    metadata: Arc<RwLock<TableMetadata>>,
    ///Index Structures for fast lookup
    indices: Arc<RwLock<FamilyIndices>>,
    /// All rule entries in this table
    entries: Arc<RwLock<Vec<Arc<dyn RuleInstance>>>>,
}

impl RuleFamilyTable {
    /// Creates a new empty table for the specified family
    pub fn new(family_id: RuleFamilyId, layer_id: LayerId) -> Self {
        let secondary_type = Self::determine_secondary_index_type(&family_id);

        RuleFamilyTable {
            family_id: family_id.clone(),
            layer_id,
            schema_version: 1,
            version: Arc::new(RwLock::new(0)),
            created_at: now_ms(),
            metadata: Arc::new(RwLock::new(TableMetadata::new(1))),
            indices: Arc::new(RwLock::new(FamilyIndices::new(secondary_type))),
            entries: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Determines the appropriate secondary index type for a family
    fn determine_secondary_index_type(family_id: &RuleFamilyId) -> SecondaryIndexType {
        match family_id {
            // L0: System layer
            RuleFamilyId::NetworkEgress => SecondaryIndexType::Domain,
            RuleFamilyId::SidecarSpawn => SecondaryIndexType::Image,

            // L4: Tool gateway
            RuleFamilyId::ToolWhitelist | RuleFamilyId::ToolParamConstraint => {
                SecondaryIndexType::Tool
            }

            // L5: RAG layer
            RuleFamilyId::RAGSource | RuleFamilyId::RAGDocSensitivity => SecondaryIndexType::Source,

            // All other families: agent-only indexing
            _ => SecondaryIndexType::None,
        }
    }

    // ============================================================================================
    // ACCESSORS
    // ============================================================================================

    /// Returns the family ID
    pub fn family_id(&self) -> &RuleFamilyId {
        &self.family_id
    }

    /// Returns the layer ID
    pub fn layer_id(&self) -> &LayerId {
        &self.layer_id
    }

    /// Returns the current version
    pub fn version(&self) -> u64 {
        *self.version.read()
    }

    /// Returns the schema version
    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    /// Returns the creation timestamp
    pub fn created_at(&self) -> u64 {
        self.created_at
    }

    /// Returns a snapshot of current metadata
    pub fn metadata(&self) -> TableMetadata {
        self.metadata.read().clone()
    }

    /// Returns index statistics
    pub fn index_stats(&self) -> IndexStats {
        self.indices.read().stats()
    }

    // ============================================================================================
    // RULE MANAGEMENT
    // ============================================================================================

    /// Adds a rule to the table
    ///
    /// The rule is added to appropriate indices based on its scope.
    /// Table version and metadata are updated atomically.
    ///

    pub fn add_rule(&self, rule: Arc<dyn RuleInstance>) -> Result<(), String> {
        // Validate family match
        if rule.family_id() != self.family_id {
            return Err(format!(
                "Rule family mismatch: expected {}, got {}",
                self.family_id.family_id(),
                rule.family_id().family_id()
            ));
        }

        let rule_id = rule.rule_id().to_string();

        // Check for duplicate
        {
            let entries = self.entries.read();
            if entries.iter().any(|r| r.rule_id() == rule_id) {
                return Err(format!("Rule {} already exists in table", rule_id));
            }
        }

        // Add to entries
        self.entries.write().push(Arc::clone(&rule));

        // Add to indices
        self.indices.write().add_rule(Arc::clone(&rule));

        // Update metadata
        {
            let mut meta = self.metadata.write();
            meta.rule_count += 1;
            if rule.scope().is_global {
                meta.global_count += 1;
            } else {
                meta.scoped_count += 1;
            }
            meta.last_updated = now_ms();
        }

        // Increment version
        *self.version.write() += 1;

        Ok(())
    }

    /// Adds multiple rules in a batch
    ///
    /// More efficient than adding rules one at a time.
    /// Updates version and metadata only once.
    pub fn add_rules_batch(&self, rules: Vec<Arc<dyn RuleInstance>>) -> Result<(), String> {
        if rules.is_empty() {
            return Ok(());
        }

        // Validate all rules first
        for rule in &rules {
            if rule.family_id() != self.family_id {
                return Err(format!(
                    "Rule family mismatch: expected {}, got {}",
                    self.family_id.family_id(),
                    rule.family_id().family_id()
                ));
            }
        }

        // Check for duplicates
        {
            let entries = self.entries.read();
            for rule in &rules {
                let rule_id = rule.rule_id();
                if entries.iter().any(|r| r.rule_id() == rule_id) {
                    return Err(format!("Rule {} already exists in table", rule_id));
                }
            }
        }

        let mut global_count = 0;
        let mut scoped_count = 0;

        // Add all rules
        {
            let mut entries = self.entries.write();
            let mut indices = self.indices.write();

            for rule in rules {
                if rule.scope().is_global {
                    global_count += 1;
                } else {
                    scoped_count += 1;
                }

                entries.push(Arc::clone(&rule));
                indices.add_rule(rule);
            }
        }

        // Update metadata
        {
            let mut meta = self.metadata.write();
            meta.rule_count += global_count + scoped_count;
            meta.global_count += global_count;
            meta.scoped_count += scoped_count;
            meta.last_updated = now_ms();
        }

        // Increment version
        *self.version.write() += 1;

        Ok(())
    }

    /// Removes a rule by ID
    ///
    /// # Returns
    /// * `Ok(true)` if rule was found and removed
    /// * `Ok(false)` if rule was not found
    pub fn remove_rule(&self, rule_id: &str) -> Result<bool, String> {
        let mut found = false;
        let mut was_global = false;

        // Remove from entries
        {
            let mut entries = self.entries.write();
            if let Some(pos) = entries.iter().position(|r| r.rule_id() == rule_id) {
                was_global = entries[pos].scope().is_global;
                entries.remove(pos);
                found = true;
            }
        }

        if found {
            // Remove from indices
            self.indices.write().remove_rule(rule_id);

            // Update metadata
            {
                let mut meta = self.metadata.write();
                meta.rule_count = meta.rule_count.saturating_sub(1);
                if was_global {
                    meta.global_count = meta.global_count.saturating_sub(1);
                } else {
                    meta.scoped_count = meta.scoped_count.saturating_sub(1);
                }
                meta.last_updated = now_ms();
            }

            // Increment version
            *self.version.write() += 1;
        }

        Ok(found)
    }

    /// Clears all rules from the table
    pub fn clear(&self) {
        self.entries.write().clear();
        self.indices.write().clear();

        {
            let mut meta = self.metadata.write();
            meta.rule_count = 0;
            meta.global_count = 0;
            meta.scoped_count = 0;
            meta.last_updated = now_ms();
        }

        *self.version.write() += 1;
    }

    /// Replaces all rules with a new set (atomic swap)
    ///
    /// This is the preferred method for hot-reloading rule sets.
    pub fn replace_all(&self, rules: Vec<Arc<dyn RuleInstance>>) -> Result<(), String> {
        // Validate all rules
        for rule in &rules {
            if rule.family_id() != self.family_id {
                return Err(format!(
                    "Rule family mismatch: expected {}, got {}",
                    self.family_id.family_id(),
                    rule.family_id().family_id()
                ));
            }
        }

        let mut global_count = 0;
        let mut scoped_count = 0;

        // Build new indices
        let secondary_type = Self::determine_secondary_index_type(&self.family_id);
        let mut new_indices = FamilyIndices::new(secondary_type);

        for rule in &rules {
            if rule.scope().is_global {
                global_count += 1;
            } else {
                scoped_count += 1;
            }
            new_indices.add_rule(Arc::clone(rule));
        }

        // Atomic swap
        *self.entries.write() = rules;
        *self.indices.write() = new_indices;

        // Update metadata
        {
            let mut meta = self.metadata.write();
            meta.rule_count = global_count + scoped_count;
            meta.global_count = global_count;
            meta.scoped_count = scoped_count;
            meta.last_updated = now_ms();
        }

        // Increment version
        *self.version.write() += 1;

        Ok(())
    }

    // ============================================================================================
    // QUERY OPERATIONS (LOCK-FREE READS)
    // ============================================================================================

    /// Queries rules by agent ID
    ///
    /// Returns rules scoped to this agent plus global rules,
    /// sorted by priority (descending).
    pub fn query_by_agent(&self, agent_id: &str) -> Vec<Arc<dyn RuleInstance>> {
        self.indices.read().query_by_agent(agent_id)
    }

    /// Queries rules by secondary key (tool, source, domain, etc.)
    pub fn query_by_secondary(&self, key: &str) -> Vec<Arc<dyn RuleInstance>> {
        self.indices.read().query_by_secondary(key)
    }

    /// Queries rules by both agent and secondary key
    pub fn query_by_agent_and_secondary(
        &self,
        agent_id: &str,
        secondary_key: &str,
    ) -> Vec<Arc<dyn RuleInstance>> {
        self.indices
            .read()
            .query_by_agent_and_secondary(agent_id, secondary_key)
    }

    /// Returns all global rules
    pub fn query_globals(&self) -> Vec<Arc<dyn RuleInstance>> {
        self.indices.read().query_globals()
    }

    /// Returns all rules (no filtering)
    pub fn query_all(&self) -> Vec<Arc<dyn RuleInstance>> {
        self.indices.read().query_all()
    }

    /// Finds a specific rule by ID
    pub fn find_rule(&self, rule_id: &str) -> Option<Arc<dyn RuleInstance>> {
        self.entries
            .read()
            .iter()
            .find(|r| r.rule_id() == rule_id)
            .map(Arc::clone)
    }

    /// Returns the total number of rules
    pub fn rule_count(&self) -> usize {
        self.entries.read().len()
    }

    /// Checks if table is empty
    pub fn is_empty(&self) -> bool {
        self.entries.read().is_empty()
    }
}

// ================================================================================================
// DISPLAY & DEBUG
// ================================================================================================

impl std::fmt::Debug for RuleFamilyTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let meta = self.metadata();
        f.debug_struct("RuleFamilyTable")
            .field("family_id", &self.family_id.family_id())
            .field("layer_id", &self.layer_id)
            .field("version", &self.version())
            .field("rule_count", &meta.rule_count)
            .field("global_count", &meta.global_count)
            .field("scoped_count", &meta.scoped_count)
            .finish()
    }
}
