//! # Bridge Types Module
//!
//! Core type definitions, enums, and traits for the Bridge and RuleFamilyTable system.
//!
//! This module provides:
//! - Layer and Family identification enums
//! - Common trait definitions for rule instances
//! - Action and match type definitions
//! - Scope and constraint types

use std::collections::HashMap;
use std::fmt;

use serde_json::{json, Value};

// ================================================================================================
// LAYER & FAMILY IDENTIFICATION
// ================================================================================================

/// Represents the 7 enforcement layers in the data plane

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LayerId {
    /// L0: System level controls
    L0System,
    /// L1: Input Validation
    L1Input,
    /// L2: Prompt assembly and planning
    L2Planner,
    /// L3: Model input/output controls
    L3ModelIO,
    /// L4: Tool invocation gateway
    L4ToolGateway,
    /// L5: RAG and retrieval controls
    L5RAG,
    /// L6: Output filtering and audit
    L6Egress,
}

impl LayerId {
    /// Returns all layer IDs in evaluation order
    pub fn all() -> Vec<LayerId> {
        vec![
            LayerId::L0System,
            LayerId::L1Input,
            LayerId::L2Planner,
            LayerId::L3ModelIO,
            LayerId::L4ToolGateway,
            LayerId::L5RAG,
            LayerId::L6Egress,
        ]
    }

    /// Returns the numeric layer number
    pub fn layer_num(&self) -> u8 {
        match self {
            LayerId::L0System => 0,
            LayerId::L1Input => 1,
            LayerId::L2Planner => 2,
            LayerId::L3ModelIO => 3,
            LayerId::L4ToolGateway => 4,
            LayerId::L5RAG => 5,
            LayerId::L6Egress => 6,
        }
    }
}

impl fmt::Display for LayerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LayerId::L0System => write!(f, "L0_System"),
            LayerId::L1Input => write!(f, "L1_Input"),
            LayerId::L2Planner => write!(f, "L2_Planner"),
            LayerId::L3ModelIO => write!(f, "L3_ModelIO"),
            LayerId::L4ToolGateway => write!(f, "L4_ToolGateway"),
            LayerId::L5RAG => write!(f, "L5_RAG"),
            LayerId::L6Egress => write!(f, "L6_Egress"),
        }
    }
}

/// Represents the 14 distinct rule families across all layers
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RuleFamilyId {
    // L0: System Layer (2 families)
    NetworkEgress,
    SidecarSpawn,

    // L1: Input Layer (2 families)
    InputSchema,
    InputSanitize,

    // L2: Planner Layer (2 families)
    PromptAssembly,
    PromptLength,

    // L3: Model I/O Layer (2 families)
    ModelOutputScan,
    ModelOutputEscalate,

    // L4: Tool Gateway Layer (2 families)
    ToolWhitelist,
    ToolParamConstraint,

    // L5: RAG Layer (2 families)
    RAGSource,
    RAGDocSensitivity,

    // L6: Egress Layer (2 families)
    OutputPII,
    OutputAudit,
}

impl RuleFamilyId {
    /// Returns all rule family IDs
    pub fn all() -> Vec<RuleFamilyId> {
        vec![
            // L0
            RuleFamilyId::NetworkEgress,
            RuleFamilyId::SidecarSpawn,
            // L1
            RuleFamilyId::InputSchema,
            RuleFamilyId::InputSanitize,
            // L2
            RuleFamilyId::PromptAssembly,
            RuleFamilyId::PromptLength,
            // L3
            RuleFamilyId::ModelOutputScan,
            RuleFamilyId::ModelOutputEscalate,
            // L4
            RuleFamilyId::ToolWhitelist,
            RuleFamilyId::ToolParamConstraint,
            // L5
            RuleFamilyId::RAGSource,
            RuleFamilyId::RAGDocSensitivity,
            // L6
            RuleFamilyId::OutputPII,
            RuleFamilyId::OutputAudit,
        ]
    }

    /// Returns the parent layer for this family
    pub fn layer(&self) -> LayerId {
        match self {
            RuleFamilyId::NetworkEgress | RuleFamilyId::SidecarSpawn => LayerId::L0System,
            RuleFamilyId::InputSchema | RuleFamilyId::InputSanitize => LayerId::L1Input,
            RuleFamilyId::PromptAssembly | RuleFamilyId::PromptLength => LayerId::L2Planner,
            RuleFamilyId::ModelOutputScan | RuleFamilyId::ModelOutputEscalate => LayerId::L3ModelIO,
            RuleFamilyId::ToolWhitelist | RuleFamilyId::ToolParamConstraint => {
                LayerId::L4ToolGateway
            }
            RuleFamilyId::RAGSource | RuleFamilyId::RAGDocSensitivity => LayerId::L5RAG,
            RuleFamilyId::OutputPII | RuleFamilyId::OutputAudit => LayerId::L6Egress,
        }
    }

    /// Returns the family identifier string
    pub fn family_id(&self) -> &'static str {
        match self {
            RuleFamilyId::NetworkEgress => "net_egress",
            RuleFamilyId::SidecarSpawn => "sidecar_spawn",
            RuleFamilyId::InputSchema => "input_schema",
            RuleFamilyId::InputSanitize => "input_sanitize",
            RuleFamilyId::PromptAssembly => "prompt_assembly",
            RuleFamilyId::PromptLength => "prompt_length",
            RuleFamilyId::ModelOutputScan => "model_output_scan",
            RuleFamilyId::ModelOutputEscalate => "model_output_escalate",
            RuleFamilyId::ToolWhitelist => "tool_whitelist",
            RuleFamilyId::ToolParamConstraint => "tool_param_constraint",
            RuleFamilyId::RAGSource => "rag_source",
            RuleFamilyId::RAGDocSensitivity => "rag_doc_sensitivity",
            RuleFamilyId::OutputPII => "output_pii",
            RuleFamilyId::OutputAudit => "output_audit",
        }
    }

    /// Returns a human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            RuleFamilyId::NetworkEgress => {
                "Control which network destinations an agent or sidecar can contact"
            }
            RuleFamilyId::SidecarSpawn => "Restrict which sidecars an agent may launch",
            RuleFamilyId::InputSchema => "Enforce payload schema, size and type",
            RuleFamilyId::InputSanitize => "Sanitize and validate input data",
            RuleFamilyId::PromptAssembly => {
                "Allow only approved context sources during prompt building"
            }
            RuleFamilyId::PromptLength => "Prevent runaway token count in composed prompt",
            RuleFamilyId::ModelOutputScan => {
                "Scan model output for PII, jailbreak or sensitive content"
            }
            RuleFamilyId::ModelOutputEscalate => "Divert uncertain responses to review",
            RuleFamilyId::ToolWhitelist => "Allow only specific tools for an agent",
            RuleFamilyId::ToolParamConstraint => {
                "Enforce parameter type and value bounds for tool calls"
            }
            RuleFamilyId::RAGSource => "Restrict retriever to specific sources or indices",
            RuleFamilyId::RAGDocSensitivity => {
                "Block sensitive or classified docs from being injected"
            }
            RuleFamilyId::OutputPII => "Detect and redact/deny PII before response leaves system",
            RuleFamilyId::OutputAudit => "Emit decision record for final user-facing outputs",
        }
    }
}

impl fmt::Display for RuleFamilyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.family_id())
    }
}

// ================================================================================================
// RULE INSTANCE TRAIT
// ================================================================================================

/// Common trait that all rule instances must implement
///
/// This trait provides a unified interface for accessing rule metadata
/// regardless of the specific family type.
pub trait RuleInstance: Send + Sync {
    /// Unique identifier for this rule instance
    fn rule_id(&self) -> &str;

    /// Priority value (higher = evaluated first)
    fn priority(&self) -> u32;

    /// Scope definition for this rule
    fn scope(&self) -> &RuleScope;

    /// Rule family this instance belongs to
    fn family_id(&self) -> RuleFamilyId;

    /// Layer this rule belongs to
    fn layer_id(&self) -> LayerId {
        self.family_id().layer()
    }

    /// Timestamp when rule was created
    fn created_at(&self) -> u64;

    /// Optional description for this rule
    fn description(&self) -> Option<&str> {
        None
    }

    /// Whether this rule is currently enabled
    fn is_enabled(&self) -> bool {
        true
    }

    /// Returns a Management Plane payload for encoding APIs (default empty for unsupported families)
    fn management_plane_payload(&self) -> Value {
        json!({})
    }
}

// ================================================================================================
// SCOPE DEFINITION
// ================================================================================================

/// Defines the scope/applicability of a rule
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleScope {
    /// Agent IDs this rule applies to (empty = all agents)
    pub agent_ids: Vec<String>,

    /// Tags for additional scoping
    pub tags: HashMap<String, String>,

    /// Whether this is a global rule (applies to all)
    pub is_global: bool,
}

impl RuleScope {
    /// Creates a new global scope
    pub fn global() -> Self {
        RuleScope {
            agent_ids: vec![],
            tags: HashMap::new(),
            is_global: true,
        }
    }

    /// Creates a scope for specific agents
    pub fn for_agents(agent_ids: Vec<String>) -> Self {
        RuleScope {
            agent_ids,
            tags: HashMap::new(),
            is_global: false,
        }
    }

    /// Creates a scope for a single agent
    pub fn for_agent(agent_id: String) -> Self {
        RuleScope {
            agent_ids: vec![agent_id],
            tags: HashMap::new(),
            is_global: false,
        }
    }

    /// Checks if this scope applies to a given agent
    pub fn applies_to(&self, agent_id: &str) -> bool {
        self.is_global || self.agent_ids.iter().any(|id| id == agent_id)
    }

    /// Adds a tag to this scope
    pub fn with_tag(mut self, key: String, value: String) -> Self {
        self.tags.insert(key, value);
        self
    }
}

impl Default for RuleScope {
    fn default() -> Self {
        RuleScope::global()
    }
}

// ================================================================================================
// ACTION TYPES
// ================================================================================================

/// Common action types across rule families
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleAction {
    /// Allow the operation
    Allow,
    /// Deny/block the operation
    Deny,
    /// Redirect to alternative target
    Redirect,
    /// Rewrite/modify the payload
    Rewrite,
    /// Redact sensitive information
    Redact,
    /// Escalate to human review
    Escalate,
    /// Truncate to fit constraints
    Truncate,
    /// Log but don't enforce
    Audit,
    /// Drop context from memory
    DropContext,
}

impl fmt::Display for RuleAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuleAction::Allow => write!(f, "ALLOW"),
            RuleAction::Deny => write!(f, "DENY"),
            RuleAction::Redirect => write!(f, "REDIRECT"),
            RuleAction::Rewrite => write!(f, "REWRITE"),
            RuleAction::Redact => write!(f, "REDACT"),
            RuleAction::Escalate => write!(f, "ESCALATE"),
            RuleAction::Truncate => write!(f, "TRUNCATE"),
            RuleAction::Audit => write!(f, "AUDIT"),
            RuleAction::DropContext => write!(f, "DROP_CONTEXT"),
        }
    }
}

// ================================================================================================
// MATCH TYPES
// ================================================================================================

/// Network protocols for L0 network egress rules
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkProtocol {
    TCP,
    UDP,
    HTTP,
    HTTPS,
}

impl Default for NetworkProtocol {
    fn default() -> Self {
        NetworkProtocol::HTTPS
    }
}

/// Parameter types for tool constraint validation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamType {
    String,
    Int,
    Float,
    Bool,
}

// ================================================================================================
// METADATA
// ================================================================================================

/// Metadata about a rule family table
#[derive(Debug, Clone)]
pub struct TableMetadata {
    /// Total number of rules in this table
    pub rule_count: usize,

    /// Number of global rules
    pub global_count: usize,

    /// Number of agent-scoped rules
    pub scoped_count: usize,

    /// Timestamp of last update
    pub last_updated: u64,

    /// Schema version for this family
    pub schema_version: u32,
}

impl TableMetadata {
    pub fn new(schema_version: u32) -> Self {
        TableMetadata {
            rule_count: 0,
            global_count: 0,
            scoped_count: 0,
            last_updated: 0,
            schema_version,
        }
    }
}

// ================================================================================================
// UTILITY FUNCTIONS
// ================================================================================================

/// Returns current timestamp in milliseconds
pub fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
