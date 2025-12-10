//! # L4 Tool Gateway Layer Rule Families
//!
//! Defines rule structures for Tool Gateway controls:
//! - ToolWhitelistRule: Allow only specific tools
//! - ToolParamConstraintRule: Enforce parameter constraints

use std::sync::Arc;

use serde_json::json;

use crate::types::{now_ms, ParamType, RuleAction, RuleFamilyId, RuleInstance, RuleScope};

// ================================================================================================
// TOOL WHITELIST RULE
// ================================================================================================

/// Controls which tools an agent is allowed to invoke
#[derive(Debug, Clone)]
pub struct ToolWhitelistRule {
    pub rule_id: String,
    pub priority: u32,
    pub scope: RuleScope,

    /// Allowed tool IDs
    pub allowed_tool_ids: Vec<String>,

    /// Allowed methods per tool
    pub allowed_methods: Vec<String>,

    /// Rate limit per minute
    pub rate_limit_per_min: Option<u32>,

    pub created_at: u64,
    pub description: Option<String>,
    pub enabled: bool,
}

impl ToolWhitelistRule {
    pub fn new(rule_id: impl Into<String>) -> Self {
        ToolWhitelistRule {
            rule_id: rule_id.into(),
            priority: 0,
            scope: RuleScope::global(),
            allowed_tool_ids: vec![],
            allowed_methods: vec![],
            rate_limit_per_min: None,
            created_at: now_ms(),
            description: None,
            enabled: true,
        }
    }

    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_scope(mut self, scope: RuleScope) -> Self {
        self.scope = scope;
        self
    }

    pub fn for_agent(mut self, agent_id: impl Into<String>) -> Self {
        self.scope = RuleScope::for_agent(agent_id.into());
        self
    }

    pub fn with_allowed_tool_ids(mut self, tool_ids: Vec<String>) -> Self {
        self.allowed_tool_ids = tool_ids;
        self
    }

    pub fn with_allowed_methods(mut self, methods: Vec<String>) -> Self {
        self.allowed_methods = methods;
        self
    }

    pub fn with_rate_limit_per_min(mut self, limit: u32) -> Self {
        self.rate_limit_per_min = Some(limit);
        self
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn is_tool_allowed(&self, tool_id: &str) -> bool {
        self.allowed_tool_ids.is_empty() || self.allowed_tool_ids.iter().any(|id| id == tool_id)
    }

    pub fn is_method_allowed(&self, method: &str) -> bool {
        self.allowed_methods.is_empty() || self.allowed_methods.iter().any(|m| m == method)
    }
}

impl RuleInstance for ToolWhitelistRule {
    fn rule_id(&self) -> &str {
        &self.rule_id
    }
    fn priority(&self) -> u32 {
        self.priority
    }
    fn scope(&self) -> &RuleScope {
        &self.scope
    }
    fn family_id(&self) -> RuleFamilyId {
        RuleFamilyId::ToolWhitelist
    }
    fn created_at(&self) -> u64 {
        self.created_at
    }
    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn management_plane_payload(&self) -> serde_json::Value {
        json!({
            "rule_id": self.rule_id.clone(),
            "allowed_tool_ids": self.allowed_tool_ids.clone(),
            "allowed_methods": self.allowed_methods.clone(),
            "rate_limit_per_min": self.rate_limit_per_min,
        })
    }
}

// ================================================================================================
// TOOL PARAM CONSTRAINT RULE
// ================================================================================================

/// Enforces parameter type and value bounds for tool calls
#[derive(Debug, Clone)]
pub struct ToolParamConstraintRule {
    pub rule_id: String,
    pub priority: u32,
    pub scope: RuleScope,

    /// Tool ID this constraint applies to
    pub tool_id: String,

    /// Parameter name
    pub param_name: String,

    /// Parameter type
    pub param_type: ParamType,

    /// Regex pattern for string validation
    pub regex: Option<String>,

    /// Allowed values (for enums)
    pub allowed_values: Vec<String>,

    /// Maximum length for strings
    pub max_len: Option<usize>,

    /// Minimum value for numbers
    pub min_value: Option<f64>,

    /// Maximum value for numbers
    pub max_value: Option<f64>,

    /// Enforcement mode (HARD = reject, SOFT = warn)
    pub enforcement_mode: EnforcementMode,

    pub created_at: u64,
    pub description: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnforcementMode {
    Hard,
    Soft,
}

impl ToolParamConstraintRule {
    pub fn new(rule_id: impl Into<String>) -> Self {
        ToolParamConstraintRule {
            rule_id: rule_id.into(),
            priority: 0,
            scope: RuleScope::global(),
            tool_id: String::new(),
            param_name: String::new(),
            param_type: ParamType::String,
            regex: None,
            allowed_values: vec![],
            max_len: None,
            min_value: None,
            max_value: None,
            enforcement_mode: EnforcementMode::Hard,
            created_at: now_ms(),
            description: None,
            enabled: true,
        }
    }

    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_scope(mut self, scope: RuleScope) -> Self {
        self.scope = scope;
        self
    }

    pub fn for_agent(mut self, agent_id: impl Into<String>) -> Self {
        self.scope = RuleScope::for_agent(agent_id.into());
        self
    }

    pub fn with_tool_id(mut self, tool_id: impl Into<String>) -> Self {
        self.tool_id = tool_id.into();
        self
    }

    pub fn with_param_name(mut self, name: impl Into<String>) -> Self {
        self.param_name = name.into();
        self
    }

    pub fn with_param_type(mut self, param_type: ParamType) -> Self {
        self.param_type = param_type;
        self
    }

    pub fn with_regex(mut self, regex: impl Into<String>) -> Self {
        self.regex = Some(regex.into());
        self
    }

    pub fn with_allowed_values(mut self, values: Vec<String>) -> Self {
        self.allowed_values = values;
        self
    }

    pub fn with_max_len(mut self, len: usize) -> Self {
        self.max_len = Some(len);
        self
    }

    pub fn with_min_value(mut self, min: f64) -> Self {
        self.min_value = Some(min);
        self
    }

    pub fn with_max_value(mut self, max: f64) -> Self {
        self.max_value = Some(max);
        self
    }

    pub fn with_enforcement_mode(mut self, mode: EnforcementMode) -> Self {
        self.enforcement_mode = mode;
        self
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

impl RuleInstance for ToolParamConstraintRule {
    fn rule_id(&self) -> &str {
        &self.rule_id
    }
    fn priority(&self) -> u32 {
        self.priority
    }
    fn scope(&self) -> &RuleScope {
        &self.scope
    }
    fn family_id(&self) -> RuleFamilyId {
        RuleFamilyId::ToolParamConstraint
    }
    fn created_at(&self) -> u64 {
        self.created_at
    }
    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn management_plane_payload(&self) -> serde_json::Value {
        let param_type = match self.param_type {
            ParamType::String => "string",
            ParamType::Int => "int",
            ParamType::Float => "float",
            ParamType::Bool => "bool",
        };

        json!({
            "rule_id": self.rule_id.clone(),
            "tool_id": self.tool_id.clone(),
            "param_name": self.param_name.clone(),
            "param_type": param_type,
            "allowed_values": self.allowed_values.clone(),
            "regex": self.regex.clone(),
            "max_len": self.max_len,
            "min_value": self.min_value,
            "max_value": self.max_value,
            "enforcement_mode": match self.enforcement_mode {
                EnforcementMode::Hard => "hard",
                EnforcementMode::Soft => "soft",
            },
        })
    }
}
