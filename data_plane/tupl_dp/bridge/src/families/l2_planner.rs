//! # L2 Planner Layer Rule Families
//!
//! Defines rule structures for the Planner layer:
//! - PromptAssemblyRule: Control approved context sources
//! - PromptLengthRule: Prevent runaway token counts

use crate::types::{now_ms, LayerId, RuleAction, RuleFamilyId, RuleInstance, RuleScope};
use std::sync::Arc;

// ================================================================================================
// PROMPT ASSEMBLY RULE
// ================================================================================================

/// Controls which context sources can be included during prompt assembly
#[derive(Debug, Clone)]
pub struct PromptAssemblyRule {
    pub rule_id: String,
    pub priority: u32,
    pub scope: RuleScope,

    /// Allowed context source IDs
    pub allowed_context_ids: Vec<String>,

    /// Enforce provenance tracking
    pub enforce_provenance: bool,

    /// Maximum tokens in composed prompt
    pub max_prompt_tokens: u32,

    pub created_at: u64,
    pub description: Option<String>,
    pub enabled: bool,
}

impl PromptAssemblyRule {
    pub fn new(rule_id: impl Into<String>) -> Self {
        PromptAssemblyRule {
            rule_id: rule_id.into(),
            priority: 0,
            scope: RuleScope::global(),
            allowed_context_ids: vec![],
            enforce_provenance: true,
            max_prompt_tokens: 8192,
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

    pub fn with_allowed_context_ids(mut self, ids: Vec<String>) -> Self {
        self.allowed_context_ids = ids;
        self
    }

    pub fn with_enforce_provenance(mut self, enforce: bool) -> Self {
        self.enforce_provenance = enforce;
        self
    }

    pub fn with_max_prompt_tokens(mut self, tokens: u32) -> Self {
        self.max_prompt_tokens = tokens;
        self
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn is_context_allowed(&self, context_id: &str) -> bool {
        self.allowed_context_ids.is_empty()
            || self.allowed_context_ids.iter().any(|id| id == context_id)
    }
}

impl RuleInstance for PromptAssemblyRule {
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
        RuleFamilyId::PromptAssembly
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
}

// ================================================================================================
// PROMPT LENGTH RULE
// ================================================================================================

/// Prevents runaway token count in composed prompts
#[derive(Debug, Clone)]
pub struct PromptLengthRule {
    pub rule_id: String,
    pub priority: u32,
    pub scope: RuleScope,

    /// Maximum tokens allowed
    pub max_prompt_tokens: u32,

    /// Action on violation
    pub action_on_violation: RuleAction,

    pub created_at: u64,
    pub description: Option<String>,
    pub enabled: bool,
}

impl PromptLengthRule {
    pub fn new(rule_id: impl Into<String>) -> Self {
        PromptLengthRule {
            rule_id: rule_id.into(),
            priority: 0,
            scope: RuleScope::global(),
            max_prompt_tokens: 8192,
            action_on_violation: RuleAction::Truncate,
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

    pub fn with_max_prompt_tokens(mut self, tokens: u32) -> Self {
        self.max_prompt_tokens = tokens;
        self
    }

    pub fn with_action_on_violation(mut self, action: RuleAction) -> Self {
        self.action_on_violation = action;
        self
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn is_token_count_valid(&self, token_count: u32) -> bool {
        token_count <= self.max_prompt_tokens
    }
}

impl RuleInstance for PromptLengthRule {
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
        RuleFamilyId::PromptLength
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
}
