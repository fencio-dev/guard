use crate::rule_vector::RuleVector;
use crate::table::RuleFamilyTable;
use crate::types::{now_ms, LayerId, RuleFamilyId, RuleInstance};
use parking_lot::RwLock;
/// Implements the bridge structure that manages all the rule family tables.
/// The Bridge acts as a multiplexer for 14 rule family tables (one per family),
/// providing unified access, versioning and lifecycle management.
///
/// # Architecture
/// - One table per rule family (not per layer)
/// - Lock-free reads via atomic Arc pointers
/// - Copy-on-write for hot-reload scenarios
/// - Per-family indexing optimized for evaluation patterns
use std::collections::HashMap;
use std::sync::Arc;

// ================================================================================================
// BRIDGE STRUCTURE
// ================================================================================================

/// The Bridge is the root data structure for storing all rules in the data plane
///
/// It maintains 14 independent tables (one per rule family), each optimized
/// for a specific rule schema and indexing strategy.
///
/// # Thread Safety
/// - Tables are stored behind Arc<RwLock<>> for safe concurrent access
/// - Reads can occur simultaneously across all tables
/// - Writes to one table don't block reads/writes to other tables
///
/// # Versioning
/// - Bridge has a global version number
/// - Each table has its own version number
/// - Versions increment on any modification

#[derive(Debug)]
pub struct Bridge {
    /// Map of family ID to table
    tables: HashMap<RuleFamilyId, Arc<RwLock<RuleFamilyTable>>>,
    ///Global bridge version (Increments on any table modifications)
    active_version: Arc<RwLock<u64>>,
    ///Optional staged version for atomic hot reload
    staged_version: Arc<RwLock<Option<u64>>>,
    ///Creation timestamp
    created_at: u64,
    /// Pre-encoded anchors for installed rules
    rule_anchors: Arc<RwLock<HashMap<String, RuleVector>>>,
}

impl Bridge {
    ///Initializes a new Bridge with empty tables for all rule families.
    /// Each table is created with deafult settings and no rules.
    /// Tables can be populated later through the add_rule method or hot-reload
    /// options.
    ///
    pub fn init() -> Self {
        let families = RuleFamilyId::all();
        let mut tables = HashMap::new();

        //Create one table per family
        for family in families {
            let layer = family.layer();
            let table = RuleFamilyTable::new(family.clone(), layer);

            tables.insert(family.clone(), Arc::new(RwLock::new(table)));
        }
        Bridge {
            tables,
            active_version: Arc::new(RwLock::new(0)),
            staged_version: Arc::new(RwLock::new(None)),
            created_at: now_ms(),
            rule_anchors: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // ============================================================================================
    // ACCESSORS
    // ============================================================================================

    /// Returns the current global version

    pub fn version(&self) -> u64 {
        *self.active_version.read()
    }

    /// Returns the staged version (if any)
    pub fn staged_version(&self) -> Option<u64> {
        *self.staged_version.read()
    }

    /// Returns the creation timestamp
    pub fn created_at(&self) -> u64 {
        self.created_at
    }

    /// Returns the number of tables in the bridge
    pub fn table_count(&self) -> usize {
        self.tables.len()
    }

    /// Returns a list of all family IDs in the bridge
    pub fn family_ids(&self) -> Vec<RuleFamilyId> {
        self.tables.keys().cloned().collect()
    }

    // ============================================================================================
    // TABLE ACCESS
    // ============================================================================================

    /// Gets a reference to a specific table by family ID

    pub fn get_table(&self, family_id: &RuleFamilyId) -> Option<Arc<RwLock<RuleFamilyTable>>> {
        self.tables.get(family_id).map(Arc::clone)
    }

    /// Gets all tables for a specific layer
    pub fn get_tables_by_layer(
        &self,
        layer_id: &LayerId,
    ) -> Vec<(RuleFamilyId, Arc<RwLock<RuleFamilyTable>>)> {
        self.tables
            .iter()
            .filter(|(fam_id, _)| fam_id.layer() == *layer_id)
            .map(|(fam_id, table)| (fam_id.clone(), Arc::clone(table)))
            .collect()
    }

    // ============================================================================================
    // RULE OPERATIONS (CONVENIENCE WRAPPERS)
    // ============================================================================================

    /// Adds a rule to the appropriate table based on its family
    ///
    /// This is a convenience wrapper that automatically routes the rule
    /// to the correct table based on the rule's family_id().

    pub fn add_rule(&self, rule: Arc<dyn RuleInstance>) -> Result<(), String> {
        let family_id = rule.family_id();
        match self.get_table(&family_id) {
            Some(table) => {
                let result = table.write().add_rule(rule);
                if result.is_ok() {
                    self.increment_version();
                }
                result
            }
            None => Err(format!("Table for family {} not found", family_id)),
        }
    }

    /// Adds a rule and stores its pre-encoded anchors
    pub fn add_rule_with_anchors(
        &self,
        rule: Arc<dyn RuleInstance>,
        anchors: RuleVector,
    ) -> Result<(), String> {
        let family_id = rule.family_id();

        match self.get_table(&family_id) {
            Some(table) => {
                table.write().add_rule(Arc::clone(&rule))?;
                self.rule_anchors
                    .write()
                    .insert(rule.rule_id().to_string(), anchors);
                self.increment_version();
                Ok(())
            }
            None => Err(format!("Table for family {} not found", family_id)),
        }
    }

    /// Get anchors for a rule that was installed
    pub fn get_rule_anchors(&self, rule_id: &str) -> Option<RuleVector> {
        self.rule_anchors.read().get(rule_id).cloned()
    }

    /// Add multiple rules in a batch (more efficient than individual adds)
    pub fn add_rules_batch(&self, rules: Vec<Arc<dyn RuleInstance>>) -> Result<(), String> {
        if rules.is_empty() {
            return Ok(());
        }

        // Group rules by family
        let mut by_family: HashMap<RuleFamilyId, Vec<Arc<dyn RuleInstance>>> = HashMap::new();

        for rule in rules {
            by_family
                .entry(rule.family_id())
                .or_insert_with(Vec::new)
                .push(rule);
        }

        // Add rules to each table
        for (family_id, family_rules) in by_family {
            match self.get_table(&family_id) {
                Some(table) => {
                    table.write().add_rules_batch(family_rules)?;
                }
                None => {
                    return Err(format!("Table for family {} not found", family_id));
                }
            }
        }

        self.increment_version();
        Ok(())
    }

    // Removes a rule from the appropriate table
    pub fn remove_rule(&self, family_id: &RuleFamilyId, rule_id: &str) -> Result<bool, String> {
        match self.get_table(family_id) {
            Some(table) => {
                let result = table.write().remove_rule(rule_id);
                if result.is_ok() && result.as_ref().unwrap() == &true {
                    self.increment_version();
                }
                result
            }
            None => Err(format!("Table for family {} not found", family_id)),
        }
    }

    /// Clears all rules from a specific table
    pub fn clear_table(&self, family_id: &RuleFamilyId) -> Result<(), String> {
        match self.get_table(family_id) {
            Some(table) => {
                table.write().clear();
                self.increment_version();
                Ok(())
            }
            None => Err(format!("Table for family {} not found", family_id)),
        }
    }

    /// Clears all rules from all tables
    pub fn clear_all(&self) {
        for table in self.tables.values() {
            table.write().clear();
        }
        self.increment_version();
    }

    // ============================================================================================
    // QUERY OPERATIONS
    // ============================================================================================

    /// Queries rules from a specific table by agent ID
    pub fn query_by_agent(
        &self,
        family_id: &RuleFamilyId,
        agent_id: &str,
    ) -> Result<Vec<Arc<dyn RuleInstance>>, String> {
        match self.get_table(family_id) {
            Some(table) => Ok(table.read().query_by_secondary(agent_id)),
            None => Err(format!("Table for family {} not found", family_id)),
        }
    }

    /// Queries global rules from a specific table
    pub fn query_global(
        &self,
        family_id: &RuleFamilyId,
    ) -> Result<Vec<Arc<dyn RuleInstance>>, String> {
        match self.get_table(family_id) {
            Some(table) => Ok(table.read().query_globals()),
            None => Err(format!("Table for family {} not found", family_id)),
        }
    }

    /// Finds a specific rule across all tables
    pub fn find_rule(&self, rule_id: &str) -> Option<Arc<dyn RuleInstance>> {
        for table in self.tables.values() {
            if let Some(rule) = table.read().find_rule(rule_id) {
                return Some(rule);
            }
        }
        None
    }

    // ============================================================================================
    // STATISTICS & MONITORING
    // ============================================================================================

    /// Returns statistics about the bridge
    pub fn stats(&self) -> BridgeStats {
        let mut total_rules = 0;
        let mut total_global_rules = 0;
        let mut total_scoped_rules = 0;
        let mut tables_with_rules = 0;

        for table in self.tables.values() {
            let meta = table.read().metadata();
            total_rules += meta.rule_count;
            total_global_rules += meta.global_count;
            total_scoped_rules += meta.scoped_count;

            if meta.rule_count > 0 {
                tables_with_rules += 1;
            }
        }

        BridgeStats {
            version: self.version(),
            total_tables: self.tables.len(),
            tables_with_rules,
            total_rules,
            total_global_rules,
            total_scoped_rules,
            created_at: self.created_at,
        }
    }

    /// Returns per-table statistics
    pub fn table_stats(&self) -> Vec<TableStats> {
        let mut stats = Vec::new();

        for (family_id, table) in &self.tables {
            let table_guard = table.read();
            let meta = table_guard.metadata();

            stats.push(TableStats {
                family_id: family_id.clone(),
                layer_id: table_guard.layer_id().clone(),
                version: table_guard.version(),
                rule_count: meta.rule_count,
                global_count: meta.global_count,
                scoped_count: meta.scoped_count,
            });
        }

        stats.sort_by_key(|s| s.layer_id.layer_num());
        stats
    }

    // ============================================================================================
    // VERSIONING
    // ============================================================================================

    /// Increments the bridge version
    fn increment_version(&self) {
        *self.active_version.write() += 1;
    }

    /// Sets the staged version for hot-reload
    pub fn set_staged_version(&self, version: u64) {
        *self.staged_version.write() = Some(version);
    }

    /// Clears the staged version
    pub fn clear_staged_version(&self) {
        *self.staged_version.write() = None;
    }

    /// Promotes staged version to active (atomic hot-reload)
    pub fn promote_staged(&self) -> Result<(), String> {
        let staged = *self.staged_version.read();

        match staged {
            Some(v) => {
                *self.active_version.write() = v;
                self.clear_staged_version();
                Ok(())
            }
            None => Err("No staged version to promote".to_string()),
        }
    }
}

// ================================================================================================
// STATISTICS STRUCTURES
// ================================================================================================

/// Bridge-level statistics
#[derive(Debug, Clone)]
pub struct BridgeStats {
    /// Current bridge version
    pub version: u64,

    /// Total number of tables
    pub total_tables: usize,

    /// Number of tables with at least one rule
    pub tables_with_rules: usize,

    /// Total rules across all tables
    pub total_rules: usize,

    /// Total global rules
    pub total_global_rules: usize,

    /// Total scoped rules
    pub total_scoped_rules: usize,

    /// Bridge creation timestamp
    pub created_at: u64,
}

/// Per-table statistics
#[derive(Debug, Clone)]
pub struct TableStats {
    /// Rule family ID
    pub family_id: RuleFamilyId,

    /// Parent layer
    pub layer_id: LayerId,

    /// Table version
    pub version: u64,

    /// Number of rules
    pub rule_count: usize,

    /// Number of global rules
    pub global_count: usize,

    /// Number of scoped rules
    pub scoped_count: usize,
}
