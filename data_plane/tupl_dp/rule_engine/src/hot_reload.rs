// Advanced atomic Table Swap for zero downtime deployment. 
// Supprts staged rollout, blue-green deployments, canary releases, 
// automatic rollbacks, and A/B testing with copy on write atomicity. 

// Design Principles:
// 1. Zero-downtime deployment - readers never blocked
// 2. Atomic swap - all-or-nothing rule activation
// 3. Staged rollout - gradual traffic migration
// 4. Automatic rollback - revert on failure
// 5. Version history - track all deployments
// 6. Health monitoring - detect issues during rollout
//
// Architecture:
// - DeploymentManager: Orchestrates hot reload operations
// - VersionRegistry: Tracks multiple rule versions
// - RolloutController: Manages gradual rollout
// - HealthMonitor: Detects issues during deployment
// - RollbackManager: Automatic revert on failure
//
// Key Features:
// - Blue-green deployments (instant swap)
// - Canary deployments (gradual percentage-based)
// - A/B testing (traffic splitting)
// - Scheduled deployments (time-based activation)
// - Automatic rollback (health-based)
// - Zero reader blocking (copy-on-write)

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock, Mutex};
use std::time::{SystemTime, Duration, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

use crate::rule_bundle::{BundleId, RuleBundle};
use crate::rule_table::RuleTable;

// ============================================================================
// Core Types
// ============================================================================

/// Deployment version identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VersionId {
    id: String
}

impl VersionId {
    pub fn new(id: String) -> Self {
        Self {
            id,
        }
    }

    pub fn from_bundle(bundle_id: &BundleId) -> Self {
        Self {
            id: format!("v_{}", bundle_id.as_str()),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.id
    }
}

/// Deployment State
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeploymentState {
    /// Preparing deployment (Validation)
    Preparing,
    /// Staged(rules loaded but not active)
    Staged,
    /// Rolling out (gradual activation)
    RollingOut {current_percentage: f64},
    ///Active (fully deployed)
    Active,
    /// Rolling back (reverted to previous)
    RollingBack,
    /// Rolled back to previous version
    RolledBack,
    /// Failed (deployment Failed)
    Failed {reason: String},
}

/// Deployment Strategy
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeploymentStrategy {
    /// Blue-green: instant swap
    BlueGreen,
    /// Canary: gradual percentage-based rollout
    Canary {
        /// Stages: percentage milestones (e.g., [10, 25, 50, 100])
        stages: Vec<f64>,
        /// Duration between stages (seconds)
        stage_duration_secs: u64,
    },
    /// A/B testing: split traffic between versions
    ABTest {
        /// Traffic split (0.0-1.0 for version A)
        split_ratio: f64,
        /// Duration of test (seconds)
        test_duration_secs: u64,
    },
    /// Scheduled: activate at specific time
    Scheduled {
        /// Activation time (Unix timestamp)
        activation_time: u64,
    },
}

/// Deployment Metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentMetadata {
    /// Deployment version
    pub version_id: VersionId,
    /// Bundle being deployed
    pub bundle_id: BundleId,
    /// Deployment strategy
    pub strategy: DeploymentStrategy,
    /// Current state
    pub state: DeploymentState,
    /// When deployment started
    pub started_at: SystemTime,
    /// When deployment completed (if applicable)
    pub completed_at: Option<SystemTime>,
    /// Deployed by
    pub deployed_by: String,
    /// Health metrics during deployment
    pub health_metrics: HealthMetrics,
}

/// Health metrics for monitoring deployment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthMetrics {
    /// Total evaluations since deployment started
    pub total_evaluations: u64,
    /// Number of errors
    pub error_count: u64,
    /// Number of timeouts
    pub timeout_count: u64,
    /// Average latency (microseconds)
    pub avg_latency_us: u64,
    /// Error rate (0.0-1.0)
    pub error_rate: f64,
    /// Last health check timestamp
    pub last_check: SystemTime,
}

impl HealthMetrics {
    pub fn new() -> Self {
        Self {
            total_evaluations: 0,
            error_count: 0,
            timeout_count: 0,
            avg_latency_us: 0,
            error_rate: 0.0,
            last_check: SystemTime::now(),
        }
    }
    
    /// Check if metrics indicate healthy deployment
    pub fn is_healthy(&self, thresholds: &HealthThresholds) -> bool {
        self.error_rate <= thresholds.max_error_rate
            && self.avg_latency_us <= thresholds.max_latency_us
            && self.timeout_count <= thresholds.max_timeouts
    }
}

impl Default for HealthMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Health check thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthThresholds {
    /// Maximum acceptable error rate (0.0-1.0)
    pub max_error_rate: f64,
    /// Maximum acceptable latency (microseconds)
    pub max_latency_us: u64,
    /// Maximum acceptable timeouts
    pub max_timeouts: u64,
}

impl Default for HealthThresholds {
    fn default() -> Self {
        Self {
            max_error_rate: 0.01,    // 1% error rate
            max_latency_us: 10000,    // 10ms
            max_timeouts: 100,
        }
    }
}

/// Version entry in registry
#[derive(Clone)]
struct VersionEntry {
    version_id: VersionId,
    table: Arc<RuleTable>,
    metadata: DeploymentMetadata,
}

// ============================================================================
// Version Registry
// ============================================================================

/// Registry of deployed versions
struct VersionRegistry {
    /// Active version (what's currently serving traffic)
    active_version: Option<VersionId>,
    /// Staged version (prepared but not active)
    staged_version: Option<VersionId>,
    /// All version entries
    versions: HashMap<VersionId, VersionEntry>,
    /// Version history (most recent first)
    history: VecDeque<VersionId>,
    /// Maximum history size
    max_history: usize,
}

impl VersionRegistry {
    fn new(max_history: usize) -> Self {
        Self {
            active_version: None, 
            staged_version: None, 
            versions: HashMap::new(),
            history: VecDeque::new(),
            max_history,
        }
    }

    /// Add a new version
    fn add_version(&mut self, entry: VersionEntry) {
        let version_id = entry.version_id.clone();
        self.versions.insert(version_id.clone(), entry);

        //Add to history
        self.history.push_front(version_id.clone());

        // Trim history
        while self.history.len() > self.max_history {
            if let Some(old_id) = self.history.pop_back() {
                //Dont remove if its active or staged
                if Some(&old_id) != self.active_version.as_ref() && Some(&old_id) != self.staged_version.as_ref() {
                    self.versions.remove(&old_id);
                }
            }
        }
    }

    /// Get active version
    fn get_active(&self) -> Option<&VersionEntry> {
        self.active_version
            .as_ref()
            .and_then(|id| self.versions.get(id))
    }
    
    /// Get staged version
    fn get_staged(&self) -> Option<&VersionEntry> {
        self.staged_version
            .as_ref()
            .and_then(|id| self.versions.get(id))
    }
    
    /// Get any version
    fn get_version(&self, version_id: &VersionId) -> Option<&VersionEntry> {
        self.versions.get(version_id)
    }
    
    /// Set active version (atomic swap)
    fn set_active(&mut self, version_id: VersionId) {
        self.active_version = Some(version_id);
    }
    
    /// Set staged version
    fn set_staged(&mut self, version_id: VersionId) {
        self.staged_version = Some(version_id);
    }
    
    /// Get version history
    fn get_history(&self) -> Vec<VersionId> {
        self.history.iter().cloned().collect()
    }
}

// ============================================================================
// Rollout Controller
// ============================================================================
/// Controls gradual rollout of new version
struct RolloutController {
    /// Current rollout state
    state: RolloutState, 
    /// Traffic router for splitting requests
    router: TrafficRouter
}

#[derive(Debug, Clone)]
struct RolloutState {
    /// Which verson is being rolled out
    target_version: VersionId, 
    /// Current traffic percentage to new version
    current_percentage: f64, 
    /// Rollout Stages
    stages: Vec<f64>,
    /// Current stage index
    current_stage: usize,
    /// When current stage started
    stage_started_at: SystemTime, 
    /// Stage Duration
    stage_duration: Duration
}

impl RolloutController {
    fn new() -> Self {
        Self {
            state: RolloutState {
                target_version: VersionId::new("".to_string()),
                current_percentage: 0.0,
                stages: vec![],
                current_stage: 0,
                stage_started_at: SystemTime::now(),
                stage_duration: Duration::from_secs(300),
            },
            router: TrafficRouter::new(),
        }
    }

    /// Start a new rollout
    fn start_rollout(
        &mut self,
        target_version: VersionId,
        strategy: &DeploymentStrategy,
    ) -> Result<(), String> {
        match strategy {
            DeploymentStrategy::Canary { stages, stage_duration_secs } => {
                self.state = RolloutState {
                    target_version,
                    current_percentage: 0.0,
                    stages: stages.clone(),
                    current_stage: 0,
                    stage_started_at: SystemTime::now(),
                    stage_duration: Duration::from_secs(*stage_duration_secs),
                };
                Ok(())
            }
            _ => Err("Strategy not supported for gradual rollout".to_string()),
        }
    }

    /// Advance to next stage if ready
    fn advance_stage(&mut self) -> Result<bool, String> {
        // Check if enough time has passed
        let elapsed = SystemTime::now()
            .duration_since(self.state.stage_started_at)
            .unwrap_or_default();
        
        if elapsed < self.state.stage_duration {
            return Ok(false); // Not ready yet
        }
        
        // Check if we have more stages
        if self.state.current_stage >= self.state.stages.len() {
            return Ok(false); // Already at final stage
        }
        
        // Advance to next stage
        self.state.current_percentage = self.state.stages[self.state.current_stage];
        self.state.current_stage += 1;
        self.state.stage_started_at = SystemTime::now();
        
        Ok(true)
    }

    /// Check if rollout is complete
    fn is_complete(&self) -> bool {
        self.state.current_stage >= self.state.stages.len()
            && self.state.current_percentage >= 100.0
    }
    
    /// Route a request to appropriate version
    fn route_request(&self, request_hash: u64) -> VersionSelection {
        if self.state.current_percentage >= 100.0 {
            return VersionSelection::New;
        }
        
        if self.state.current_percentage <= 0.0 {
            return VersionSelection::Old;
        }
        
        // Use consistent hashing for stable routing
        self.router.route(request_hash, self.state.current_percentage)
    }
}

/// Version selection for request routing
#[derive(Debug, Clone, PartialEq)]
enum VersionSelection {
    Old,  // Route to old version
    New,  // Route to new version
}

/// Traffic router for consistent hashing
struct TrafficRouter {
    hash_ring_size: u64,
}

impl TrafficRouter {
    fn new() -> Self {
        Self {
            hash_ring_size: 10000,
        }
    }
    
    /// Route request based on hash and percentage
    fn route(&self, request_hash: u64, percentage: f64) -> VersionSelection {
        let threshold = (percentage / 100.0 * self.hash_ring_size as f64) as u64;
        let position = request_hash % self.hash_ring_size;
        
        if position < threshold {
            VersionSelection::New
        } else {
            VersionSelection::Old
        }
    }
}


// ============================================================================
// Deployment Manager
// ============================================================================

/// Main hot reload manager
pub struct DeploymentManager {
    /// Version registry
    registry: Arc<RwLock<VersionRegistry>>,
    /// Rollout controller
    rollout: Arc<Mutex<RolloutController>>,
    /// Health thresholds
    health_thresholds: HealthThresholds,
    /// Automatic rollback enabled
    auto_rollback: bool,
}

impl DeploymentManager {
    /// Create new deployment manager
    pub fn new() -> Self {
        Self {
            registry: Arc::new(RwLock::new(VersionRegistry::new(10))),
            rollout: Arc::new(Mutex::new(RolloutController::new())),
            health_thresholds: HealthThresholds::default(),
            auto_rollback: true,
        }
    }
    
    /// Create with custom configuration
    pub fn with_config(
        max_history: usize,
        health_thresholds: HealthThresholds,
        auto_rollback: bool,
    ) -> Self {
        Self {
            registry: Arc::new(RwLock::new(VersionRegistry::new(max_history))),
            rollout: Arc::new(Mutex::new(RolloutController::new())),
            health_thresholds,
            auto_rollback,
        }
    }
    // ========================================================================
    // Deployment Operations
    // ========================================================================
    
    /// Prepare a new deployment (stage rules without activating)
    pub fn prepare_deployment(
        &self,
        bundle: RuleBundle,
        strategy: DeploymentStrategy,
        deployed_by: String,
    ) -> Result<VersionId, String> {
        let version_id = VersionId::from_bundle(&bundle.metadata.bundle_id);

        // Create new rule table
        let table = RuleTable::new();

        // Load rules from bundle
        table.load_bundle(bundle.rules, bundle.metadata.bundle_id.clone())?;

        // Create deployment metadata
        let metadata = DeploymentMetadata {
            version_id: version_id.clone(),
            bundle_id: bundle.metadata.bundle_id,
            strategy,
            state: DeploymentState::Staged,
            started_at: SystemTime::now(),
            completed_at: None,
            deployed_by,
            health_metrics: HealthMetrics::new(),
        };
        // Create version entry
        let entry = VersionEntry {
            version_id: version_id.clone(),
            table: Arc::new(table),
            metadata,
        };
        
        // Add to registry
        let mut registry = self.registry.write().unwrap();
        registry.add_version(entry);
        registry.set_staged(version_id.clone());
        
        Ok(version_id)
    }

    /// Activate staged deployment (atomic swap)
    pub fn activate_deployment(&self, version_id: &VersionId) -> Result<(), String> {
        let mut registry = self.registry.write().unwrap();

        // Verify version exists and is staged
        let entry = registry.get_version(version_id).ok_or_else(|| format!("Version {} not found", version_id.as_str()))?;
        if entry.metadata.state != DeploymentState::Staged {
            return Err(format!(
                "Version {} is not staged (state: {:?})",
                version_id.as_str(),
                entry.metadata.state
            ));
        }
        
        // Get deployment strategy
        let strategy = entry.metadata.strategy.clone();
        
        match strategy {
            DeploymentStrategy::BlueGreen => {
                // Instant atomic swap
                registry.set_active(version_id.clone());
                
                // Update state
                if let Some(entry) = registry.versions.get_mut(version_id) {
                    entry.metadata.state = DeploymentState::Active;
                    entry.metadata.completed_at = Some(SystemTime::now());
                }
                
                Ok(())
            }
            DeploymentStrategy::Canary { .. } => {
                // Start gradual rollout
                drop(registry); // Release lock before starting rollout
                
                let mut rollout = self.rollout.lock().unwrap();
                rollout.start_rollout(version_id.clone(), &strategy)?;
                
                // Update state
                let mut registry = self.registry.write().unwrap();
                if let Some(entry) = registry.versions.get_mut(version_id) {
                    entry.metadata.state = DeploymentState::RollingOut {
                        current_percentage: 0.0,
                    };
                }
                
                Ok(())
            }
            DeploymentStrategy::ABTest { .. } => {
                // A/B test activation
                registry.set_active(version_id.clone());
                
                if let Some(entry) = registry.versions.get_mut(version_id) {
                    entry.metadata.state = DeploymentState::Active;
                }
                
                Ok(())
            }
            DeploymentStrategy::Scheduled { activation_time } => {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                
                if now >= activation_time {
                    registry.set_active(version_id.clone());
                    
                    if let Some(entry) = registry.versions.get_mut(version_id) {
                        entry.metadata.state = DeploymentState::Active;
                        entry.metadata.completed_at = Some(SystemTime::now());
                    }
                    
                    Ok(())
                } else {
                    Err(format!(
                        "Activation time not reached (scheduled: {})",
                        activation_time
                    ))
                }
            }
        }
    }
    /// Advance canary rollout to next stage
    pub fn advance_rollout(&self) -> Result<bool, String> {
        let mut rollout = self.rollout.lock().unwrap();
        let advanced = rollout.advance_stage()?;
        
        if advanced {
            let version_id = rollout.state.target_version.clone();
            let percentage = rollout.state.current_percentage;
            
            // Update state
            let mut registry = self.registry.write().unwrap();
            if let Some(entry) = registry.versions.get_mut(&version_id) {
                entry.metadata.state = DeploymentState::RollingOut {
                    current_percentage: percentage,
                };
            }
            
            // Check if rollout is complete
            if rollout.is_complete() {
                registry.set_active(version_id.clone());
                
                if let Some(entry) = registry.versions.get_mut(&version_id) {
                    entry.metadata.state = DeploymentState::Active;
                    entry.metadata.completed_at = Some(SystemTime::now());
                }
            }
        }
        
        Ok(advanced)
    }
    
    /// Rollback to previous version
    pub fn rollback(&self) -> Result<VersionId, String> {
        let mut registry = self.registry.write().unwrap();
        
        // Get current and previous versions
        let history = registry.get_history();
        if history.len() < 2 {
            return Err("No previous version to rollback to".to_string());
        }
        
        let current_version = &history[0];
        let previous_version = &history[1];
        
        // Mark current as rolled back
        if let Some(entry) = registry.versions.get_mut(current_version) {
            entry.metadata.state = DeploymentState::RolledBack;
        }
        
        // Activate previous version
        registry.set_active(previous_version.clone());
        
        if let Some(entry) = registry.versions.get_mut(previous_version) {
            entry.metadata.state = DeploymentState::Active;
        }
        
        Ok(previous_version.clone())
    }
    // ========================================================================
    // Query Operations
    // ========================================================================
    
    /// Get rule table for evaluation (lock-free)
    pub fn get_active_table(&self) -> Option<Arc<RuleTable>> {
        let registry = self.registry.read().unwrap();
        registry.get_active().map(|entry| Arc::clone(&entry.table))
    }
    
    /// Route request and get appropriate table
    pub fn route_and_get_table(&self, request_hash: u64) -> Option<Arc<RuleTable>> {
        let rollout = self.rollout.lock().unwrap();
        let selection = rollout.route_request(request_hash);
        drop(rollout);
        
        let registry = self.registry.read().unwrap();
        
        match selection {
            VersionSelection::New => {
                registry.get_staged().map(|entry| Arc::clone(&entry.table))
            }
            VersionSelection::Old => {
                registry.get_active().map(|entry| Arc::clone(&entry.table))
            }
        }
    }
    
    /// Get deployment metadata
    pub fn get_deployment_info(&self, version_id: &VersionId) -> Option<DeploymentMetadata> {
        let registry = self.registry.read().unwrap();
        registry
            .get_version(version_id)
            .map(|entry| entry.metadata.clone())
    }
    
    /// Get active version ID
    pub fn get_active_version_id(&self) -> Option<VersionId> {
        let registry = self.registry.read().unwrap();
        registry.active_version.clone()
    }
    
    /// Get deployment history
    pub fn get_deployment_history(&self) -> Vec<VersionId> {
        let registry = self.registry.read().unwrap();
        registry.get_history()
    }
    
    // ========================================================================
    // Health Monitoring
    // ========================================================================
    
    /// Update health metrics for a version
    pub fn update_health_metrics(
        &self,
        version_id: &VersionId,
        evaluations: u64,
        errors: u64,
        timeouts: u64,
        avg_latency_us: u64,
    ) -> Result<(), String> {
        let mut registry = self.registry.write().unwrap();
        
        if let Some(entry) = registry.versions.get_mut(version_id) {
            let metrics = &mut entry.metadata.health_metrics;
            
            metrics.total_evaluations += evaluations;
            metrics.error_count += errors;
            metrics.timeout_count += timeouts;
            metrics.avg_latency_us = avg_latency_us;
            
            if metrics.total_evaluations > 0 {
                metrics.error_rate = metrics.error_count as f64 / metrics.total_evaluations as f64;
            }
            
            metrics.last_check = SystemTime::now();
            
            // Check if automatic rollback needed
            if self.auto_rollback && !metrics.is_healthy(&self.health_thresholds) {
                drop(registry); // Release lock before rollback
                self.rollback()?;
            }
            
            Ok(())
        } else {
            Err(format!("Version {} not found", version_id.as_str()))
        }
    }
    
    /// Get health status
    pub fn get_health_status(&self, version_id: &VersionId) -> Option<bool> {
        let registry = self.registry.read().unwrap();
        registry.get_version(version_id).map(|entry| {
            entry
                .metadata
                .health_metrics
                .is_healthy(&self.health_thresholds)
        })
    }
}

impl Default for DeploymentManager {
    fn default() -> Self {
        Self::new()
    }
}

// Thread-safety markers
unsafe impl Send for DeploymentManager {}
unsafe impl Sync for DeploymentManager {}

// ============================================================================
// Helper Functions
// ============================================================================

/// Compute request hash for consistent routing
pub fn compute_request_hash(agent_id: &str, flow_id: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    agent_id.hash(&mut hasher);
    flow_id.hash(&mut hasher);
    hasher.finish()
}


