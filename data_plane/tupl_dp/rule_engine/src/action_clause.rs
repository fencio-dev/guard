// This module implements the action component of the rules, which defines
// what happens when a rule matches an event. Actions are atomic operations with
// explicit side effects and controlled resource usage. 

// # Design Principles
// - **Atomic actions**: Each action is a discrete, well-defined operation
// - **Explicit side effects**: All side effects must be declared and approved
// - **Type-safe parameters**: Each action type has its own parameter structure
// - **Resource constraints**: Actions have time/memory/CPU limits
// - **Auditable**: Every action execution is logged with provenance

use crate::{AgentId, FlowId};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::Duration;

// ============================================================================
// ACTION TYPE - ATOMIC ACTION ENUM
// ============================================================================

/// Atomic action types for rule enforcement.
///
/// Each action type represents a discrete operation that can be performed
/// when a rule matches. Actions are designed to be:
/// - **Atomic**: Complete successfully or fail entirely (no partial states)
/// - **Deterministic**: Same input always produces same output
/// - **Fast**: Execute within bounded time
/// - **Auditable**: All executions are logged
///
/// # Action Categories
/// - **Control Flow**: DENY, ALLOW
/// - **Transformation**: REWRITE, REDACT
/// - **Routing**: ROUTE_TO, SPAWN_SIDECAR
/// - **Rate Limiting**: RATE_LIMIT
/// - **Observability**: LOG, ATTACH_METADATA
/// - **Integration**: CALLBACK, SANDBOX_EXECUTE

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "params")]

pub enum ActionType {
    /// Block the request/event (hard enforcement).
    ///
    /// # Side Effects
    /// - Request is denied
    /// - Error response returned to caller
    /// - Audit log entry created
    Deny(DenyParams),
    /// Allow the request/event to proceed.
    ///
    /// # Side Effects
    /// - Request continues through pipeline
    /// - Optional audit log entry
    Allow(AllowParams),
    /// Rewrite/modify the payload before forwarding.
    ///
    /// # Side Effects
    /// - Payload is modified in-place
    /// - Original payload may be logged for audit
    Rewrite(RewriteParams),
    /// Redact sensitive information from payload.
    ///
    /// # Side Effects
    /// - Sensitive fields are removed or masked
    /// - Redaction is logged for audit
    Redact(RedactParams),
    /// Spawn a sidecar process for analysis/processing.
    ///
    /// # Side Effects
    /// - New process/container launched
    /// - Resources allocated
    /// - Original request may be delayed
    SpawnSidecar(SpawnSidecarParams),
    /// Route request to a different agent/queue.
    ///
    /// # Side Effects
    /// - Request forwarded to new destination
    /// - Original destination bypassed
    RouteTo(RouteToParams),
    /// Apply rate limiting to the event/agent.
    ///
    /// # Side Effects
    /// - Counter incremented
    /// - May deny request if limit exceeded
    RateLimit(RateLimitParams),
    /// Log the event for observability.
    ///
    /// # Side Effects
    /// - Log entry written
    /// - Metrics updated
    Log(LogParams),
    /// Attach metadata/tags to the event.
    ///
    /// # Side Effects
    /// - Metadata added to event headers
    /// - Available to downstream processors
    AttachMetadata(AttachMetadataParams),
    /// Send callback event to control plane.
    ///
    /// # Side Effects
    /// - Async event sent to control plane
    /// - Does not block request processing
    Callback(CallbackParams),
    /// Execute custom logic in sandbox (WASM).
    ///
    /// # Side Effects
    /// - WASM module executed in sandbox
    /// - May modify payload based on module logic
    SandboxExecute(SandboxExecuteParams),
}

impl ActionType {
    /// Returns the name of this action type as a string. 
    pub fn name(&self) -> &'static str {
        match self {
            ActionType::Deny(_) => "DENY",
            ActionType::Allow(_) => "ALLOW",
            ActionType::Rewrite(_) => "REWRITE",
            ActionType::Redact(_) => "REDACT",
            ActionType::SpawnSidecar(_) => "SPAWN_SIDECAR",
            ActionType::RouteTo(_) => "ROUTE_TO",
            ActionType::RateLimit(_) => "RATE_LIMIT",
            ActionType::Log(_) => "LOG",
            ActionType::AttachMetadata(_) => "ATTACH_METADATA",
            ActionType::Callback(_) => "CALLBACK",
            ActionType::SandboxExecute(_) => "SANDBOX_EXECUTE",
        }
    }

    /// Returns true if this action has side effects.
    pub fn requires_authorization(&self) -> bool {
        matches!(
            self, 
            ActionType::SpawnSidecar(_)
                | ActionType::SandboxExecute(_)
                | ActionType::Callback(_)
                | ActionType::RouteTo(_)
        )
    }

    /// Returns true if this action modifies the payload.
    pub fn modifies_payload(&self) -> bool {
        matches!(self, ActionType::Rewrite(_) | ActionType::Redact(_))
    }

    ///Return true if this action blocks request
    pub fn is_blocking(&self) -> bool {
        matches!(self, ActionType::Deny(_))
    }
}

// ============================================================================
// ACTION PARAMETERS - TYPE-SAFE PARAMETER STRUCTURES
// ============================================================================

/// Parameters for DENY action.

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct DenyParams {
    /// Human readable reason for denial.
    pub reason: String, 
    /// Machine readable error code. 
    pub error_code: u16,
    /// HTTP status code 
    pub http_status:Option<u16>,
}

impl Default for DenyParams {
    fn default() -> Self {
        DenyParams {
            reason: "Request denied by policy".to_string(),
            error_code: 403,
            http_status: Some(403),
        }
    }
}

/// Parameters for ALLOW action.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AllowParams {
    /// Whether to log this allow decision.
    pub log_decision: bool,

    /// Optional reason for allowing.
    pub reason: Option<String>,
}

impl Default for AllowParams {
    fn default() -> Self {
        AllowParams {
            log_decision: false,
            reason: None,
        }
    }
}

/// Parameters for REWRITE action.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RewriteParams {
    /// List of rewrite operations to apply.
    pub operations: Vec<RewriteOperation>,

    /// Whether to preserve original payload for audit.
    pub preserve_original: bool,
}

/// Individual rewrite operation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RewriteOperation {
    /// Set a field to a specific value.
    SetField {
        path: String,
        value: String,
    },

    /// Delete a field.
    DeleteField {
        path: String,
    },

    /// Rename a field.
    RenameField {
        from: String,
        to: String,
    },

    /// Apply a transformation function.
    Transform {
        path: String,
        function: TransformFunction,
    },
}

/// Transformation functions for rewrite operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TransformFunction {
    /// Convert to uppercase.
    Uppercase,

    /// Convert to lowercase.
    Lowercase,

    /// Trim whitespace.
    Trim,

    /// Base64 encode.
    Base64Encode,

    /// Base64 decode.
    Base64Decode,

    /// Hash (SHA256).
    Hash,
}

/// Parameters for REDACT action.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RedactParams {
    /// List of field paths to redact.
    pub fields: Vec<String>,

    /// Redaction strategy.
    pub strategy: RedactionStrategy,

    /// Template for redacted values (for Mask strategy).
    pub redaction_template: Option<String>,
}

/// Redaction strategies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RedactionStrategy {
    /// Remove the field entirely.
    Remove,

    /// Replace with a mask string.
    Mask,

    /// Replace with hash of original value.
    Hash,

    /// Keep length, replace characters with '*'.
    Partial,
}

/// Parameters for SPAWN_SIDECAR action.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpawnSidecarParams {
    /// Specification for the sidecar.
    pub sidecar_spec: SidecarSpec,

    /// Whether to block until sidecar completes.
    pub block_on_completion: bool,

    /// Whether to pass payload to sidecar.
    pub pass_payload: bool,
}

/// Sidecar specification.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SidecarSpec {
    /// Type of sidecar (e.g., "ml-analyzer", "data-enricher").
    pub sidecar_type: String,

    /// Container image or executable path.
    pub image: String,

    /// CPU shares allocation.
    pub cpu_shares: u32,

    /// Memory limit in megabytes.
    pub memory_limit_mb: usize,

    /// Maximum execution time.
    #[serde(with = "duration_serde")]
    pub timeout: Duration,
}

/// Parameters for ROUTE_TO action.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RouteToParams {
    /// Destination agent (if routing to agent).
    pub dest_agent: Option<AgentId>,

    /// Queue name (if routing to queue).
    pub queue_name: Option<String>,

    /// Whether to preserve original headers.
    pub preserve_headers: bool,
}

/// Parameters for RATE_LIMIT action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RateLimitParams {
    /// Maximum number of requests.
    pub max_requests: u64,

    /// Time window for rate limit.
    #[serde(with = "duration_serde")]
    pub window: Duration,

    /// Scope of rate limit.
    pub scope: RateLimitScope,

    /// Action to take when limit exceeded.
    pub action_on_exceed: Box<ActionType>,
}

/// Rate limit scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RateLimitScope {
    /// Per source agent.
    PerAgent,

    /// Per flow.
    PerFlow,

    /// Per destination.
    PerDestination,

    /// Global (across all).
    Global,

    /// Per custom key (e.g., user_id, api_key).
    PerKey,
}

/// Parameters for LOG action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogParams {
    /// Log level.
    pub level: LogLevel,

    /// Log message.
    pub message: String,

    /// Whether to include payload in log.
    pub include_payload: bool,

    /// Additional structured data.
    pub structured_data: Option<HashMap<String, String>>,
}

/// Log levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

/// Parameters for ATTACH_METADATA action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachMetadataParams {
    /// Metadata key-value pairs to attach.
    pub metadata: HashMap<String, String>,

    /// Whether to overwrite existing metadata.
    pub overwrite_existing: bool,
}

/// Parameters for CALLBACK action.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CallbackParams {
    /// Callback endpoint URL.
    pub endpoint: String,

    /// Type of event to send.
    pub event_type: String,

    /// Whether to include payload in callback.
    pub include_payload: bool,

    /// Whether to deliver asynchronously.
    pub async_delivery: bool,
}

/// Parameters for SANDBOX_EXECUTE action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxExecuteParams {
    /// WASM module identifier.
    pub module_id: String,

    /// Module integrity digest.
    pub module_digest: String,

    /// Maximum execution time.
    #[serde(with = "duration_serde")]
    pub max_exec_time: Duration,

    /// Memory limit in megabytes.
    pub memory_limit_mb: usize,

    /// Input parameters to pass to module.
    pub input_params: Option<HashMap<String, String>>,
}

// Custom serde module for Duration (same as in match_clause)
mod duration_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.as_millis().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis = u64::deserialize(deserializer)?;
        Ok(Duration::from_millis(millis))
    }
}

// ============================================================================
// ACTION CLAUSE - MAIN STRUCTURE
// ============================================================================

/// Complete action specification for a rule.
///
/// ActionClause defines what should happen when a rule matches. It includes:
/// - The primary action to execute
/// - Optional secondary actions (executed if primary succeeds)
/// - Allowed side effects (must be approved by control plane)
/// - Resource constraints for action execution
///
/// # Design Principles
/// - **Atomic execution**: All actions either complete or rollback
/// - **Explicit side effects**: Must declare all side effects upfront
/// - **Bounded resources**: Time, memory, and CPU limits enforced
/// - **Auditable**: Every execution logged with provenance

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActionClause {
    /// Primary action to execute.
    pub primary_action: ActionType,
    /// Secondary actions to execute if primary succeeds.
    pub secondary_actions: Vec<ActionType>,
    /// Set of allowed side effects.
    pub allowed_side_effects: HashSet<AllowedSideEffect>,
    /// Maximum time for all the actions to comnplete. 
    #[serde(with = "duration_serde")]
    pub max_execution_time: Duration,
    /// Whether to rollback on failure.
    pub rollback_on_failure: bool,
}

impl ActionClause {
    /// Creates a new action clause with a single action.
    pub fn new(action: ActionType) -> Self {
        // Infer allowed side effects from the action type
        let allowed_side_effects = Self::infer_side_effects(&action);

        ActionClause {
            primary_action: action,
            secondary_actions: Vec::new(),
            allowed_side_effects,
            max_execution_time: Duration::from_millis(100), // Default 100ms
            rollback_on_failure: true,
        }
    }

    /// Creates a builder for constructing ActionClause.
    pub fn builder(action: ActionType) -> ActionClauseBuilder {
        ActionClauseBuilder::new(action)
    }

    /// Validates this ActionClause.
    ///
    /// Checks:
    /// - All actions have necessary side effect permissions
    /// - Resource limits are reasonable
    /// - Action combinations are valid
    ///
    /// # Returns
    /// `Ok(())` if valid, `Err(String)` with error message if invalid
    pub fn validate(&self) -> Result<(), String> {
        // Check primary action has required side effects
        let required = self.primary_action.required_side_effects();
        for effect in &required {
            if !self.allowed_side_effects.contains(effect) {
                return Err(format!(
                    "Primary action requires {:?} side effect but it's not allowed",
                    effect
                ));
            }
        }

        // Check secondary actions
        for action in &self.secondary_actions {
            let required = action.required_side_effects();
            for effect in &required {
                if !self.allowed_side_effects.contains(effect) {
                    return Err(format!(
                        "Secondary action {} requires {:?} side effect but it's not allowed",
                        action.name(),
                        effect
                    ));
                }
            }
        }

        // Check execution time is reasonable
        if self.max_execution_time > Duration::from_secs(30) {
            return Err("max_execution_time exceeds 30 seconds".to_string());
        }

        // Check for conflicting actions
        if self.primary_action.is_blocking() && !self.secondary_actions.is_empty() {
            return Err("DENY action cannot have secondary actions".to_string());
        }

        Ok(())
    }

    /// Returns all actions (primary + secondary).
    pub fn all_actions(&self) -> Vec<&ActionType> {
        let mut actions = vec![&self.primary_action];
        actions.extend(self.secondary_actions.iter());
        actions
    }

    /// Returns true if this clause requires payload access.
    pub fn requires_payload(&self) -> bool {
        self.all_actions()
            .iter()
            .any(|a| a.modifies_payload() || matches!(a, ActionType::Redact(_)))
    }

    /// Infers required side effects from an action.
    fn infer_side_effects(action: &ActionType) -> HashSet<AllowedSideEffect> {
        action.required_side_effects().into_iter().collect()
    }
}

impl ActionType {
    /// Returns the side effects required by this action.
    fn required_side_effects(&self) -> Vec<AllowedSideEffect> {
        match self {
            ActionType::Deny(_) | ActionType::Allow(_) => {
                vec![AllowedSideEffect::Logging, AllowedSideEffect::Metrics]
            }
            ActionType::Rewrite(_) | ActionType::Redact(_) => {
                vec![
                    AllowedSideEffect::PayloadModification,
                    AllowedSideEffect::Logging,
                ]
            }
            ActionType::SpawnSidecar(_) => {
                vec![
                    AllowedSideEffect::ProcessSpawn,
                    AllowedSideEffect::ResourceAllocation,
                    AllowedSideEffect::Logging,
                ]
            }
            ActionType::RouteTo(_) => {
                vec![AllowedSideEffect::Routing, AllowedSideEffect::Logging]
            }
            ActionType::RateLimit(_) => {
                vec![
                    AllowedSideEffect::StateModification,
                    AllowedSideEffect::Logging,
                ]
            }
            ActionType::Log(_) => {
                vec![AllowedSideEffect::Logging]
            }
            ActionType::AttachMetadata(_) => {
                vec![AllowedSideEffect::MetadataModification]
            }
            ActionType::Callback(_) => {
                vec![AllowedSideEffect::NetworkCall, AllowedSideEffect::Logging]
            }
            ActionType::SandboxExecute(_) => {
                vec![
                    AllowedSideEffect::SandboxExecution,
                    AllowedSideEffect::ResourceAllocation,
                    AllowedSideEffect::Logging,
                ]
            }
        }
    }
}
// ============================================================================
// ALLOWED SIDE EFFECTS
// ============================================================================

/// Side effects that can be caused by actions.
///
/// All side effects must be explicitly declared and approved by the control
/// plane before a rule can be activated. This ensures:
/// - Security: No unexpected operations
/// - Auditability: Clear understanding of rule behavior
/// - Resource management: Proper allocation and limits

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AllowedSideEffect {
    /// Write to logs.
    Logging,

    /// Update metrics/counters.
    Metrics,

    /// Modify request payload.
    PayloadModification,

    /// Modify event metadata/headers.
    MetadataModification,

    /// Modify internal state (e.g., rate limit counters).
    StateModification,

    /// Spawn a new process/container.
    ProcessSpawn,

    /// Allocate computational resources.
    ResourceAllocation,

    /// Make network calls (callbacks, webhooks).
    NetworkCall,

    /// Change routing destination.
    Routing,

    /// Execute code in sandbox.
    SandboxExecution,
}

// ============================================================================
// ACTION EXECUTION CONTEXT
// ============================================================================

/// Context for action execution.
///
/// Provides all necessary information to execute an action, including:
/// - Event context and payload
/// - Rule metadata
/// - Execution constraints
#[derive(Debug)]
pub struct ActionContext<'a> {
    /// Rule ID that triggered this action.
    pub rule_id: &'a crate::RuleId,

    /// Rule version.
    pub rule_version: u64,

    /// Source agent.
    pub source_agent: &'a AgentId,

    /// Destination agent (if any).
    pub dest_agent: Option<&'a AgentId>,

    /// Flow ID (if any).
    pub flow_id: Option<&'a FlowId>,

    /// Mutable reference to payload (for modifications).
    pub payload: Option<&'a mut Vec<u8>>,

    /// Metadata (can be modified).
    pub metadata: &'a mut HashMap<String, String>,

    /// Remaining execution time budget.
    pub time_budget: Duration,
}

// ============================================================================
// ACTION EXECUTION RESULT
// ============================================================================

/// Result of executing an action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionResult {
    /// Action executed successfully.
    Success {
        /// Human-readable message.
        message: String,

        /// Whether payload was modified.
        payload_modified: bool,

        /// Whether metadata was modified.
        metadata_modified: bool,
    },

    /// Action was denied/blocked.
    Denied {
        /// Reason for denial.
        reason: String,

        /// Error code.
        error_code: String,
    },

    /// Action failed with error.
    Failed {
        /// Error message.
        error: String,

        /// Whether failure is retryable.
        retryable: bool,
    },

    /// Action timed out.
    Timeout {
        /// Time spent before timeout.
        elapsed: Duration,
    },

    /// Action was skipped (conditional execution).
    Skipped {
        /// Reason for skipping.
        reason: String,
    },
}

impl ActionResult {
    /// Returns true if the action succeeded.
    pub fn is_success(&self) -> bool {
        matches!(self, ActionResult::Success { .. })
    }

    /// Returns true if the action was denied.
    pub fn is_denied(&self) -> bool {
        matches!(self, ActionResult::Denied { .. })
    }

    /// Returns true if the action failed.
    pub fn is_failed(&self) -> bool {
        matches!(self, ActionResult::Failed { .. })
    }

    /// Returns true if the result indicates request should be blocked.
    pub fn should_block(&self) -> bool {
        matches!(
            self,
            ActionResult::Denied { .. } | ActionResult::Failed { retryable: false, .. }
        )
    }
}

// ============================================================================
// BUILDER PATTERN FOR ACTION CLAUSE
// ============================================================================

/// Builder for ActionClause.
#[derive(Debug)]
pub struct ActionClauseBuilder {
    primary_action: ActionType,
    secondary_actions: Vec<ActionType>,
    allowed_side_effects: HashSet<AllowedSideEffect>,
    max_execution_time: Duration,
    rollback_on_failure: bool,
}

impl ActionClauseBuilder {
    /// Creates a new builder with the given primary action.
    pub fn new(action: ActionType) -> Self {
        let allowed_side_effects = action.required_side_effects().into_iter().collect();

        ActionClauseBuilder {
            primary_action: action,
            secondary_actions: Vec::new(),
            allowed_side_effects,
            max_execution_time: Duration::from_millis(100),
            rollback_on_failure: true,
        }
    }

    /// Adds a secondary action.
    pub fn add_secondary(mut self, action: ActionType) -> Self {
        // Add required side effects
        for effect in action.required_side_effects() {
            self.allowed_side_effects.insert(effect);
        }
        self.secondary_actions.push(action);
        self
    }

    /// Sets multiple secondary actions.
    pub fn secondary_actions(mut self, actions: Vec<ActionType>) -> Self {
        for action in &actions {
            for effect in action.required_side_effects() {
                self.allowed_side_effects.insert(effect);
            }
        }
        self.secondary_actions = actions;
        self
    }

    /// Adds an allowed side effect.
    pub fn allow_side_effect(mut self, effect: AllowedSideEffect) -> Self {
        self.allowed_side_effects.insert(effect);
        self
    }

    /// Sets allowed side effects.
    pub fn allowed_side_effects(mut self, effects: HashSet<AllowedSideEffect>) -> Self {
        self.allowed_side_effects = effects;
        self
    }

    /// Sets maximum execution time.
    pub fn max_execution_time(mut self, duration: Duration) -> Self {
        self.max_execution_time = duration;
        self
    }

    /// Sets rollback behavior.
    pub fn rollback_on_failure(mut self, rollback: bool) -> Self {
        self.rollback_on_failure = rollback;
        self
    }

    /// Builds the ActionClause.
    ///
    /// # Returns
    /// `Ok(ActionClause)` if valid, `Err(String)` if validation fails
    pub fn build(self) -> Result<ActionClause, String> {
        let clause = ActionClause {
            primary_action: self.primary_action,
            secondary_actions: self.secondary_actions,
            allowed_side_effects: self.allowed_side_effects,
            max_execution_time: self.max_execution_time,
            rollback_on_failure: self.rollback_on_failure,
        };

        // Validate before returning
        clause.validate()?;

        Ok(clause)
    }
}
