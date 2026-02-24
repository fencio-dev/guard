use crate::families::DesignBoundaryRule;
use crate::rule_vector::RuleVector;
use crate::types::{now_ms, RuleInstance, RuleMetadata};
use parking_lot::{Mutex, RwLock};
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

// ================================================================================================
// STORAGE CONFIGURATION
// ================================================================================================

/// Configuration for Bridge storage.
#[derive(Clone, Debug)]
pub struct StorageConfig {
    /// Path to cold storage database (SQLite)
    pub cold_storage_path: PathBuf,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            cold_storage_path: PathBuf::from("./var/data/cold_storage.db"),
        }
    }
}

// ================================================================================================
// SQLITE SCHEMA
// ================================================================================================

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS rules (
    id          TEXT PRIMARY KEY,
    tenant_id   TEXT NOT NULL,
    layer       TEXT,
    priority    INTEGER NOT NULL DEFAULT 0,
    rule_json   TEXT NOT NULL,
    anchors_bin BLOB NOT NULL,
    status      TEXT NOT NULL DEFAULT 'active',
    updated_at  REAL NOT NULL
);
";

// ================================================================================================
// BRIDGE STRUCTURE
// ================================================================================================

/// The Bridge is the root data structure for storing all rules in the data plane.
///
/// Rules are stored in a single in-memory HashMap (the fast read path) backed by
/// SQLite as the single source of truth for persistence. The HashMap is rebuilt
/// from SQLite on startup and on InstallRules calls.
#[derive(Debug)]
pub struct Bridge {
    active_version: Arc<RwLock<u64>>,
    staged_version: Arc<RwLock<Option<u64>>>,
    created_at: u64,
    /// In-memory store: rule_id → (rule instance, rule vector)
    rules: Arc<RwLock<HashMap<String, (Arc<dyn RuleInstance>, RuleVector)>>>,
    /// SQLite connection for persistence
    db: Arc<Mutex<Connection>>,
}

impl Bridge {
    /// Initializes a new Bridge with default storage configuration.
    pub fn init() -> Result<Self, String> {
        Self::new(StorageConfig::default())
    }

    /// Creates a new Bridge with the specified storage configuration.
    pub fn new(storage_config: StorageConfig) -> Result<Self, String> {
        // Ensure parent directory exists
        if let Some(parent) = storage_config.cold_storage_path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create storage directory: {}", e))?;
            }
        }

        let conn = Connection::open(&storage_config.cold_storage_path)
            .map_err(|e| format!("Failed to open SQLite database: {}", e))?;

        conn.execute_batch(SCHEMA)
            .map_err(|e| format!("Failed to create schema: {}", e))?;

        let db = Arc::new(Mutex::new(conn));
        let rules = Arc::new(RwLock::new(HashMap::new()));

        let bridge = Bridge {
            active_version: Arc::new(RwLock::new(0)),
            staged_version: Arc::new(RwLock::new(None)),
            created_at: now_ms(),
            rules,
            db,
        };

        bridge.rebuild_from_db()?;

        Ok(bridge)
    }

    /// Creates a Bridge with default storage paths.
    pub fn with_defaults() -> Result<Self, String> {
        Self::new(StorageConfig::default())
    }

    // ============================================================================================
    // PRIVATE: REBUILD FROM DATABASE
    // ============================================================================================

    /// Rebuilds the in-memory HashMap from all rows in SQLite.
    /// Called at init and can be called on reconnect.
    fn rebuild_from_db(&self) -> Result<(), String> {
        // Collect all rows while holding the DB lock, then release it before
        // acquiring the rules write lock to avoid lock ordering issues.
        // The named binding for `rows` forces collection before the block closes,
        // ensuring conn and stmt are dropped before we take the write lock on rules.
        let rows: Vec<(String, Option<String>, i64, String, Vec<u8>)> = {
            let conn = self.db.lock();

            let mut stmt = conn
                .prepare(
                    "SELECT id, layer, priority, rule_json, anchors_bin FROM rules WHERE status = 'active'",
                )
                .map_err(|e| format!("Prepare failed during rebuild: {}", e))?;

            let collected: Result<Vec<_>, _> = stmt
                .query_map([], |row| {
                    let id: String = row.get(0)?;
                    let layer: Option<String> = row.get(1)?;
                    let priority: i64 = row.get(2)?;
                    let rule_json: String = row.get(3)?;
                    let anchors_bin: Vec<u8> = row.get(4)?;
                    Ok((id, layer, priority, rule_json, anchors_bin))
                })
                .map_err(|e| format!("Query failed during rebuild: {}", e))?
                .collect();

            // stmt and conn are still alive here but collected is owned; we can drop them
            drop(stmt);
            drop(conn);

            collected.map_err(|e| format!("Row collection failed during rebuild: {}", e))?
        };

        let mut map = self.rules.write();
        map.clear();

        for (id, _layer, priority, rule_json, anchors_bin) in rows {
            // Deserialize metadata from the stored JSON
            let metadata: RuleMetadata = match serde_json::from_str(&rule_json) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("Skipping rule {} with invalid JSON: {}", id, e);
                    continue;
                }
            };

            // Deserialize the RuleVector from raw little-endian f32 bytes
            let rule_vector = match deserialize_rule_vector(&anchors_bin) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("Skipping rule {} with invalid anchors: {}", id, e);
                    continue;
                }
            };

            let rule: Arc<dyn RuleInstance> = Arc::new(DesignBoundaryRule::new(
                metadata.rule_id.clone(),
                metadata.priority,
                metadata.scope,
                metadata.layer,
                metadata.created_at_ms,
                metadata.enabled,
                metadata.description,
                metadata.params,
            ));

            // Suppress unused variable warning for priority — it came from the DB column
            let _ = priority;

            map.insert(id, (rule, rule_vector));
        }

        Ok(())
    }

    /// Public wrapper for rebuild_from_db — called by RefreshService.
    pub fn rebuild_from_db_public(&self) -> Result<(), String> {
        self.rebuild_from_db()
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

    /// Returns the number of installed rules
    pub fn rule_count(&self) -> usize {
        self.rules.read().len()
    }

    /// Returns a clone of all installed rule instances.
    pub fn all_rules(&self) -> Vec<Arc<dyn RuleInstance>> {
        self.rules
            .read()
            .values()
            .map(|(rule, _)| Arc::clone(rule))
            .collect()
    }

    /// Returns a specific rule by ID if present.
    pub fn get_rule(&self, rule_id: &str) -> Option<Arc<dyn RuleInstance>> {
        self.rules
            .read()
            .get(rule_id)
            .map(|(rule, _)| Arc::clone(rule))
    }

    // ============================================================================================
    // RULE OPERATIONS
    // ============================================================================================

    /// Adds a rule and stores its pre-encoded anchors. Upserts into SQLite AND inserts into HashMap.
    pub fn add_rule_with_anchors(
        &self,
        rule: Arc<dyn RuleInstance>,
        anchors: RuleVector,
    ) -> Result<(), String> {
        let rule_id = rule.rule_id().to_string();
        let metadata = RuleMetadata::from_rule(rule.as_ref());

        let rule_json = serde_json::to_string(&metadata)
            .map_err(|e| format!("Failed to serialize rule metadata: {}", e))?;

        let anchors_bin = serialize_rule_vector(&anchors);

        let tenant_id = metadata
            .scope
            .agent_ids
            .first()
            .cloned()
            .unwrap_or_default();

        let layer: Option<&str> = metadata.layer.as_deref();
        let updated_at = (now_ms() as f64) / 1000.0;

        {
            let conn = self.db.lock();
            conn.execute(
                "INSERT OR REPLACE INTO rules (id, tenant_id, layer, priority, rule_json, anchors_bin, status, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'active', ?7)",
                params![
                    rule_id,
                    tenant_id,
                    layer,
                    metadata.priority as i64,
                    rule_json,
                    anchors_bin,
                    updated_at,
                ],
            )
            .map_err(|e| format!("SQLite upsert failed: {}", e))?;
        }

        self.rules
            .write()
            .insert(rule_id, (Arc::clone(&rule), anchors));

        self.increment_version();
        Ok(())
    }

    /// Removes a rule by ID. Returns true if the rule was present.
    pub fn remove_rule(&self, rule_id: &str) -> Result<bool, String> {
        let removed = self.rules.write().remove(rule_id).is_some();
        if !removed {
            return Ok(false);
        }

        {
            let conn = self.db.lock();
            conn.execute("DELETE FROM rules WHERE id = ?1", params![rule_id])
                .map_err(|e| format!("SQLite delete failed: {}", e))?;
        }

        self.increment_version();
        Ok(true)
    }

    /// Clears all rules and storage state.
    pub fn clear_all(&self) {
        self.rules.write().clear();

        let conn = self.db.lock();
        let _ = conn.execute("DELETE FROM rules", []);

        drop(conn);
        self.increment_version();
    }

    /// Get rule anchors. Reads directly from the in-memory HashMap (no SQLite hit).
    pub fn get_rule_anchors(&self, rule_id: &str) -> Option<RuleVector> {
        self.rules
            .read()
            .get(rule_id)
            .map(|(_, vector)| vector.clone())
    }

    // ============================================================================================
    // STATISTICS & MONITORING
    // ============================================================================================

    /// Returns statistics about the bridge
    pub fn stats(&self) -> BridgeStats {
        let rules = self.rules.read();
        let total_rules = rules.len();
        let global_rules = rules
            .values()
            .filter(|(rule, _)| rule.scope().is_global)
            .count();

        BridgeStats {
            version: self.version(),
            total_rules,
            global_rules,
            scoped_rules: total_rules.saturating_sub(global_rules),
            created_at: self.created_at,
        }
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
// SERIALIZATION HELPERS
// ================================================================================================

/// Serialize a RuleVector to raw little-endian f32 bytes.
///
/// Layout: action_anchors (16×32 f32s) + action_count (u64 LE) +
///         resource_anchors (16×32 f32s) + resource_count (u64 LE) +
///         data_anchors (16×32 f32s) + data_count (u64 LE) +
///         risk_anchors (16×32 f32s) + risk_count (u64 LE)
fn serialize_rule_vector(v: &RuleVector) -> Vec<u8> {
    use crate::rule_vector::{MAX_ANCHORS_PER_SLOT, SLOT_WIDTH};

    let floats_per_block = MAX_ANCHORS_PER_SLOT * SLOT_WIDTH;
    let bytes_per_block = floats_per_block * 4;
    let total = 4 * (bytes_per_block + 8); // 4 slots × (floats + u64 count)

    let mut out = Vec::with_capacity(total);

    let write_block = |out: &mut Vec<u8>, block: &[[f32; SLOT_WIDTH]; MAX_ANCHORS_PER_SLOT], count: usize| {
        for row in block.iter() {
            for &f in row.iter() {
                out.extend_from_slice(&f.to_le_bytes());
            }
        }
        out.extend_from_slice(&(count as u64).to_le_bytes());
    };

    write_block(&mut out, &v.action_anchors, v.action_count);
    write_block(&mut out, &v.resource_anchors, v.resource_count);
    write_block(&mut out, &v.data_anchors, v.data_count);
    write_block(&mut out, &v.risk_anchors, v.risk_count);

    out
}

/// Deserialize a RuleVector from raw little-endian f32 bytes.
fn deserialize_rule_vector(bytes: &[u8]) -> Result<RuleVector, String> {
    use crate::rule_vector::{MAX_ANCHORS_PER_SLOT, SLOT_WIDTH};

    let floats_per_block = MAX_ANCHORS_PER_SLOT * SLOT_WIDTH;
    let bytes_per_block = floats_per_block * 4;
    let block_with_count = bytes_per_block + 8;
    let expected = 4 * block_with_count;

    if bytes.len() != expected {
        return Err(format!(
            "Expected {} bytes for RuleVector, got {}",
            expected,
            bytes.len()
        ));
    }

    let read_block = |data: &[u8]| -> Result<([[f32; SLOT_WIDTH]; MAX_ANCHORS_PER_SLOT], usize), String> {
        let mut block = [[0f32; SLOT_WIDTH]; MAX_ANCHORS_PER_SLOT];
        let mut offset = 0;
        for row in block.iter_mut() {
            for f in row.iter_mut() {
                let chunk: [u8; 4] = data[offset..offset + 4]
                    .try_into()
                    .map_err(|_| "Slice conversion failed".to_string())?;
                *f = f32::from_le_bytes(chunk);
                offset += 4;
            }
        }
        let count_chunk: [u8; 8] = data[offset..offset + 8]
            .try_into()
            .map_err(|_| "Count slice conversion failed".to_string())?;
        let count = u64::from_le_bytes(count_chunk) as usize;
        Ok((block, count))
    };

    let (action_anchors, action_count) = read_block(&bytes[0..block_with_count])?;
    let (resource_anchors, resource_count) =
        read_block(&bytes[block_with_count..2 * block_with_count])?;
    let (data_anchors, data_count) =
        read_block(&bytes[2 * block_with_count..3 * block_with_count])?;
    let (risk_anchors, risk_count) =
        read_block(&bytes[3 * block_with_count..4 * block_with_count])?;

    Ok(RuleVector {
        action_anchors,
        action_count,
        resource_anchors,
        resource_count,
        data_anchors,
        data_count,
        risk_anchors,
        risk_count,
    })
}

// ================================================================================================
// STATISTICS STRUCTURES
// ================================================================================================

/// Bridge-level statistics
#[derive(Debug, Clone)]
pub struct BridgeStats {
    /// Current bridge version
    pub version: u64,
    /// Total rules stored
    pub total_rules: usize,
    /// Number of global rules (scope.is_global)
    pub global_rules: usize,
    /// Number of scoped rules (total - global)
    pub scoped_rules: usize,
    /// Bridge creation timestamp
    pub created_at: u64,
}
