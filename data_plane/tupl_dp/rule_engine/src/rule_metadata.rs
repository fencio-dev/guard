// Core metadata structure for rules
//
// This module defines the fundamental metadata structure that accompanies 
// every rule in the system. It provides identity, 
// versioning, scope, enforcement config and lifecycle management for rules. 

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

// These type aliases make the code more readable and provide type safety. 
// Instead of using String everywhere, we create specific types that
// convey semnatic meaning

/// Unique identifier for a rule
/// This is a UUID v4 that uniquely identifies a rule across the system.
/// Even if a rule is updated, this rule_id reamins the same. 

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RuleId(Uuid);

impl RuleId {
    /// Create a new random Rule UUID.
    pub fn new() -> Self {
        RuleId(Uuid::new_v4())
    }

    /// Returns the underlying rule uuid
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
    ///Converts to a string representation
    pub fn as_str(&self) -> String {
        self.0.to_string()
    }
}

impl Default for RuleId{
    fn default() -> Self {
        Self::new()
    }
}
impl From<Uuid> for RuleId {
    fn from(uuid: Uuid) -> Self {
        RuleId(uuid)
    }
}

impl std::fmt::Display for RuleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for an agent in the system
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(String);

impl AgentId {
    ///Creates a new Agent Id from a string
    pub fn new(id: impl Into<String>) -> Self {
        AgentId(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for AgentId {
    fn from(s: String) -> Self {
        AgentId(s)
    }
}
impl From<&str> for AgentId {
    fn from(s: &str) -> Self {
        AgentId(s.to_string())
    }
}
impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a flow (sequence of operations)
/// Flow represents a sequence of operations or steps that are executed.
/// Rules can be scoped to a specific flow as well.

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FlowId(String);

impl FlowId {
    /// Creates a new FlowId from a string.
    pub fn new(id: impl Into<String>) -> Self {
        FlowId(id.into())
    }

    /// Returns a reference to the inner string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for FlowId {
    fn from(s: String) -> Self {
        FlowId(s)
    }
}

impl From<&str> for FlowId {
    fn from(s: &str) -> Self {
        FlowId(s.to_string())
    }
}

impl std::fmt::Display for FlowId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ENUMs for the rule state and enforcement logic
/// Represent the current lifecyclr state of a rule

/// Rules transition through the following states during their lifecycle:
/// Staged -> Active -> Paused -> Revoked

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]

pub enum RuleState {
    /// Rule is staged but not yet active
    Staged,
    /// Rule is active and being enforced
    Active,
    /// Rule is paused temporarily
    Paused,
    /// Rule is revoked and no longer enforced
    Revoked,
}

impl RuleState {
    /// Returns true if the rule is in an active state that should be enforced
    pub fn is_active(&self) -> bool {
        matches!(self, RuleState::Active)
    }

    /// Returns true if the rule should be evaluated
    pub fn is_evaluable(&self) -> bool {
        matches!(self, RuleState::Staged | RuleState::Active)
    }

    /// Returns true if the rule is permanently disabled.
    pub fn is_revoked(&self) -> bool {
        matches!(self, RuleState::Revoked)
    }
}

impl Default for RuleState {
    fn default() -> Self {
        RuleState::Staged
    }
}

impl std::fmt::Display for RuleState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuleState::Staged => write!(f, "staged"),
            RuleState::Active => write!(f, "active"),
            RuleState::Paused => write!(f, "paused"),
            RuleState::Revoked => write!(f, "revoked"),
        }
    }
}

/// Defines how strictly a rule should be enforced
/// Enforcement Modes
/// Hard - Rule violations result in blocking or denial of action
/// Soft - Rule violations are simply logger
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]

pub enum EnforcementMode {
    /// Hard enforcement - violations block actions
    Hard,
    /// Soft enforcement - violations are logged only
    Soft,
}

impl Default for EnforcementMode {
    fn default() -> Self {
        EnforcementMode::Hard
    }
}

impl std::fmt::Display for EnforcementMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EnforcementMode::Hard => write!(f, "HARD"),
            EnforcementMode::Soft => write!(f, "SOFT"),
        }
    }
}

/// Categorise the rules by their primary funtion
/// This helps with indexing, prioritization and understanding the rule's
/// purpose
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]

pub enum EnforcementClass {
    /// Hard rules - must be enforced inline (DENY/ALLOW)
    BlockDeny,
    
    /// Transform rules - mutate payloads (redact PII, normalize)
    Transform,
    
    /// Augment rules - enrich with metadata/provenance
    Augment,
    
    /// Observational rules - logging, metrics, alerts
    Observational,
    
    /// Control rules - spawn sidecars, route to pipelines
    Control,
    
    /// Rate limiting rules - enforce quotas
    RateLimit,
    
    /// Graceful/soft rules - log + allow (monitoring)
    Graceful,
}

impl std::fmt::Display for EnforcementClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EnforcementClass::BlockDeny => write!(f, "block_deny"),
            EnforcementClass::Transform => write!(f, "transform"),
            EnforcementClass::Augment => write!(f, "augment"),
            EnforcementClass::Observational => write!(f, "observational"),
            EnforcementClass::Control => write!(f, "control"),
            EnforcementClass::RateLimit => write!(f, "rate_limit"),
            EnforcementClass::Graceful => write!(f, "graceful"),
        }
    }
}

// RULE SCOPE
/// Defines the scope where a rules applies
/// Rules can be scoped to:
/// - Specific agents
/// - Specific flows
/// - Global (applies everywhere)
///
/// # Scope Evaluation
/// A rule matches if:
/// - `is_global` is true, OR
/// - The target agent_id is in `agent_ids`, OR
/// - The target flow_id is in `flow_ids`

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleScope {
    /// If true, the rule applies globally
    pub is_global: bool,
    /// Set of agent IDs the rule applies to
    pub agent_ids: HashSet<AgentId>,
    /// Set of flow IDs the rule applies to
    pub flow_ids: HashSet<FlowId>,
    /// Set of destination agent IDs the rule applies to
    pub dest_agent_ids: HashSet<AgentId>,
    /// Set of payload data types the rule applies to
    pub payload_dtypes: HashSet<String>,
}

impl RuleScope {
    ///Creates an new empty scope
    pub fn new() -> Self {
        RuleScope {
            is_global: false,
            agent_ids: HashSet::new(),
            flow_ids: HashSet::new(),
            dest_agent_ids: HashSet::new(),
            payload_dtypes: HashSet::new(),
        }
    }

    /// Creates a global scope that applies to all the agents and flows
    /// This is the most common scope for security policies
    pub fn global() -> Self {
        RuleScope {
            is_global: true,
            agent_ids: HashSet::new(),
            flow_ids: HashSet::new(),
            dest_agent_ids: HashSet::new(),
            payload_dtypes: HashSet::new(),
        }
    }

    pub fn for_agents(agent_ids: impl IntoIterator<Item = AgentId>) -> Self {
        RuleScope {
            is_global: false,
            agent_ids: agent_ids.into_iter().collect(),
            flow_ids: HashSet::new(),
            dest_agent_ids: HashSet::new(),
            payload_dtypes: HashSet::new(),
        }
    }

    pub fn for_flows(flow_ids: impl IntoIterator<Item = FlowId>) -> Self {
        RuleScope {
            is_global: false,
            agent_ids: HashSet::new(),
            flow_ids: flow_ids.into_iter().collect(),
            dest_agent_ids: HashSet::new(),
            payload_dtypes: HashSet::new(),
        }
    }

    ///Sets the scope to global
    pub fn set_global(&mut self) {
        self.is_global = true;
    }

    /// Adds an agent to this scope.
    pub fn add_agent(&mut self, agent_id: AgentId) {
        self.is_global = false;
        self.agent_ids.insert(agent_id);
    }

    /// Adds a flow to this scope.
    pub fn add_flow(&mut self, flow_id: FlowId) {
        self.is_global = false;
        self.flow_ids.insert(flow_id);
    }

    /// Checks if the scope matches for a given agent or flow
    /// Returns true if:
    /// the scope is global or
    /// the agent_id is in the scope or 
    /// the flow_id is in the scope
    pub fn matches(&self, agent_id: Option<&AgentId>, 
            flow_id: Option<&FlowId>) -> bool {
        if self.is_global {
            return true;
        }
        if let Some(agent) = agent_id {
            if self.agent_ids.contains(agent) {
                return true;
            }
        }
        if let Some(flow) = flow_id {
            if self.flow_ids.contains(flow) {
                return true;
            }
        }
        false
    }

    /// Returns true if this is a global scope.
    pub fn is_global(&self) -> bool {
        self.is_global
    }

    /// Returns the number of entities (agents + flows) in scope.
    pub fn entity_count(&self) -> usize {
        if self.is_global {
            usize::MAX // Represents "all"
        } else {
            self.agent_ids.len() + self.flow_ids.len()
        }
    }
}

impl Default for RuleScope {
    fn default() -> Self {
        Self::new()
    }
}

// RULE METADATA STRUCTURE

/// Core metadata structure for a rule in the data plane. 
/// This structure contains all the metadata needed to identify, version, scope, 
/// enforce and manage the lifecycle of a rule.
/// # Design Principles
/// - **Immutability**: Most fields are set at creation and shouldn't change
/// - **Versioning**: Version increments on any update; old versions preserved
/// - **Auditability**: Every field supports audit trail and provenance
/// - **Performance**: Structure is designed for fast filtering and indexing

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuleMetadata {
    // -------- Identity --------
    /// Unique identifier for this rule (persists across versions)
    pub rule_id: RuleId,
    
    /// Version number (increments on updates)
    pub version: u64,
    
    /// Optional bundle ID if this rule belongs to a rule bundle
    pub bundle_id: Option<String>,
    
    /// Identity of the entity that signed/created this rule
    pub signer: String,
    
    /// Timestamp when this rule was created
    pub created_at: DateTime<Utc>,
    
    // -------- Scope & Priority --------
    /// Where this rule applies (global, specific agents, specific flows)
    pub scope: RuleScope,
    
    /// Priority for rule evaluation (higher = evaluated first)
    /// Typical range: 0-1000, default: 500
    pub priority: i32,
    
    // -------- Enforcement Configuration --------
    /// Current lifecycle state of the rule
    pub state: RuleState,
    
    /// Type of enforcement (what the rule does)
    pub enforcement_class: EnforcementClass,
    
    /// How strictly to enforce (HARD = block, SOFT = log)
    pub enforcement_mode: EnforcementMode,
}

impl RuleMetadata {
    ///Creates a new RuleMetadata wtih default values and sepcified core fields.
    pub fn new(signer: String, scope: RuleScope, mode: EnforcementMode) -> Self {
        RuleMetadata {
            rule_id: RuleId::new(),
            version: 1,
            bundle_id: None,
            signer,
            created_at: Utc::now(),
            scope,
            priority: 500, // Default mid priority
            state: RuleState::Staged,
            enforcement_class: EnforcementClass::BlockDeny,
            enforcement_mode: mode,
        }
    }

    /// Creating a builder for more control over the rule creation process.
    pub fn builder() -> RuleMetadataBuilder {
        RuleMetadataBuilder::default()
    }
    // -------- Getters --------

    /// Returns the unique rule identifier.
    pub fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    /// Returns the current version number.
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Returns the bundle ID if this rule belongs to a bundle.
    pub fn bundle_id(&self) -> Option<&str> {
        self.bundle_id.as_deref()
    }

    /// Returns the signer identity.
    pub fn signer(&self) -> &str {
        &self.signer
    }

    /// Returns the creation timestamp.
    pub fn created_at(&self) -> &DateTime<Utc> {
        &self.created_at
    }

    /// Returns the scope configuration.
    pub fn scope(&self) -> &RuleScope {
        &self.scope
    }

    /// Returns the priority value.
    pub fn priority(&self) -> i32 {
        self.priority
    }

    /// Returns the current state.
    pub fn state(&self) -> RuleState {
        self.state
    }

    /// Returns the enforcement class.
    pub fn enforcement_class(&self) -> EnforcementClass {
        self.enforcement_class
    }

    /// Returns the enforcement mode.
    pub fn enforcement_mode(&self) -> EnforcementMode {
        self.enforcement_mode
    }
    // -------- State Transitions --------

    /// Activates the rule (transitions to Active state).
    ///
    /// This should be called after staging/testing is complete.
    pub fn activate(&mut self) {
        self.state = RuleState::Active;
    }

    /// Pauses the rule (can be re-activated later).
    pub fn pause(&mut self) {
        self.state = RuleState::Paused;
    }

    /// Permanently revokes the rule.
    pub fn revoke(&mut self) {
        self.state = RuleState::Revoked;
    }

    /// Checks if this rule is currently active.
    pub fn is_active(&self) -> bool {
        self.state.is_active()
    }

    /// Checks if this rule should be evaluated.
    pub fn is_evaluable(&self) -> bool {
        self.state.is_evaluable()
    }

    // Versioning
    // Creates a new version of this rules with an incremented version number. 
    // The new verison is in staged state and preserves the rule id. 
    /// This allows tracking rule evolution while maintaining identity. 
    pub fn new_version(&self, signer: String) -> Self {
        RuleMetadata {
            rule_id: self.rule_id,
            version: self.version + 1,
            bundle_id: self.bundle_id.clone(),
            signer,
            created_at: Utc::now(),
            scope: self.scope.clone(),
            priority: self.priority,
            state: RuleState::Staged,
            enforcement_class: self.enforcement_class,
            enforcement_mode: self.enforcement_mode,
        }
    }

    // Util methods
    /// Checks if this rule matches teh given agen or flow. 
    pub fn matches_scope(&self, agent_id: Option<&AgentId>, 
            flow_id: Option<&FlowId>) -> bool {
        self.scope.matches(agent_id, flow_id)
    }

    pub fn describe(&self) -> String {
        format!(
            "Rule[ID: {}, Ver: {}, State: {}, Mode: {}, Class: {}, 
                Priority: {}, Scope: {} agents, {} flows]",
            self.rule_id,
            self.version,
            self.state,
            self.enforcement_mode,
            self.enforcement_class,
            self.priority,
            self.scope.agent_ids.len(),
            self.scope.flow_ids.len()
        )
    }
}

// BUILDER PATTERNS FOR RULE METADATA
/// Builder for creating RuleMetadata with finegrained control. 
/// This follows the builder pattern to allow step-by-step construction of
/// RuleMetadata instances.

#[derive(Debug, Default)]
pub struct RuleMetadataBuilder {
    rule_id: Option<RuleId>,
    version: Option<u64>,
    bundle_id: Option<String>,
    signer: Option<String>,
    created_at: Option<DateTime<Utc>>,
    scope: Option<RuleScope>,
    priority: Option<i32>,
    state: Option<RuleState>,
    enforcement_class: Option<EnforcementClass>,
    enforcement_mode: Option<EnforcementMode>,
}

impl RuleMetadataBuilder {
    ///Creates a new builder instance with all fields set to None.
    pub fn new() -> Self {
        RuleMetadataBuilder::default()
    }
    ///Sets the rule id
    pub fn rule_id(mut self, rule_id: RuleId) -> Self {
        self.rule_id = Some(rule_id);
        self
    }
    ///Sets the version number
    pub fn version(mut self, version: u64) -> Self {
        self.version = Some(version);
        self
    }
    ///Sets the bundle id
    pub fn bundle_id(mut self, bundle_id: impl Into<String>) -> Self {
        self.bundle_id = Some(bundle_id.into());
        self
    }       
    ///Sets the signer identity
    pub fn signer(mut self, signer: impl Into<String>) -> Self {
        self.signer = Some(signer.into());
        self
    }
    ///Sets the creation timestamp
    pub fn created_at(mut self, created_at: DateTime<Utc>) -> Self {
        self.created_at = Some(created_at);     
        self
    }
    ///Sets the scope configuration
    pub fn scope(mut self, scope: RuleScope) -> Self {
        self.scope = Some(scope);
        self
    }
    ///Sets the priority value
    pub fn priority(mut self, priority: i32) -> Self {
        self.priority = Some(priority);
        self
    }
    ///Sets the rule state
    pub fn state(mut self, state: RuleState) -> Self {
        self.state = Some(state);
        self
    }
    ///Sets the enforcement class
    pub fn enforcement_class(mut self, enforcement_class: EnforcementClass) -> Self {
        self.enforcement_class = Some(enforcement_class);
        self
    }
    ///Sets the enforcement mode
    pub fn enforcement_mode(mut self, enforcement_mode: EnforcementMode) -> Self {
        self.enforcement_mode = Some(enforcement_mode);
        self
    }
    ///Builds the RuleMetadata instance
    pub fn build(self) -> RuleMetadata {
        RuleMetadata {
            rule_id: self.rule_id.unwrap_or_else(RuleId::new),
            version: self.version.unwrap_or(1),
            bundle_id: self.bundle_id,
            signer: self.signer.expect("Signer is required"),
            created_at: self.created_at.unwrap_or_else(Utc::now),
            scope: self.scope.unwrap_or_default(),
            priority: self.priority.unwrap_or(500),
            state: self.state.unwrap_or(RuleState::Staged),
            enforcement_class: self.enforcement_class.unwrap_or(EnforcementClass::BlockDeny),
            enforcement_mode: self.enforcement_mode.unwrap_or(EnforcementMode::Hard),
        }
    }
}