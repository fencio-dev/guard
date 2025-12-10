//! # L5 RAG Layer Rule Families
//!
//! Defines rule structures for RAG controls:
//! - RAGSourceRule: Restrict retrieval sources
//! - RAGDocSensitivityRule: Block sensitive documents

use crate::types::{now_ms, LayerId, RuleAction, RuleFamilyId, RuleInstance, RuleScope};
use std::sync::Arc;

// ================================================================================================
// RAG SOURCE RULE
// ================================================================================================

/// Restricts retriever to specific sources or indices
#[derive(Debug, Clone)]
pub struct RAGSourceRule {
    pub rule_id: String,
    pub priority: u32,
    pub scope: RuleScope,

    /// Allowed retrieval source IDs
    pub allowed_sources: Vec<String>,

    /// Maximum documents to retrieve
    pub max_docs: u32,

    /// Maximum tokens per document
    pub max_tokens_per_doc: u32,

    pub created_at: u64,
    pub description: Option<String>,
    pub enabled: bool,
}

impl RAGSourceRule {
    pub fn new(rule_id: impl Into<String>) -> Self {
        RAGSourceRule {
            rule_id: rule_id.into(),
            priority: 0,
            scope: RuleScope::global(),
            allowed_sources: vec![],
            max_docs: 5,
            max_tokens_per_doc: 1000,
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

    pub fn with_allowed_sources(mut self, sources: Vec<String>) -> Self {
        self.allowed_sources = sources;
        self
    }

    pub fn with_max_docs(mut self, max: u32) -> Self {
        self.max_docs = max;
        self
    }

    pub fn with_max_tokens_per_doc(mut self, max: u32) -> Self {
        self.max_tokens_per_doc = max;
        self
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn is_source_allowed(&self, source_id: &str) -> bool {
        self.allowed_sources.is_empty() || self.allowed_sources.iter().any(|s| s == source_id)
    }
}

impl RuleInstance for RAGSourceRule {
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
        RuleFamilyId::RAGSource
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
// RAG DOC SENSITIVITY RULE
// ================================================================================================

/// Blocks sensitive or classified documents from being injected
#[derive(Debug, Clone)]
pub struct RAGDocSensitivityRule {
    pub rule_id: String,
    pub priority: u32,
    pub scope: RuleScope,

    /// WASM module for sensitivity classification
    pub semantic_hook: String,

    /// Action on detection (DENY, ESCALATE)
    pub action: RuleAction,

    /// Escalation target
    pub escalate_target: Option<String>,

    /// Sensitivity levels to block
    pub blocked_levels: Vec<String>,

    pub created_at: u64,
    pub description: Option<String>,
    pub enabled: bool,
}

impl RAGDocSensitivityRule {
    pub fn new(rule_id: impl Into<String>) -> Self {
        RAGDocSensitivityRule {
            rule_id: rule_id.into(),
            priority: 0,
            scope: RuleScope::global(),
            semantic_hook: String::new(),
            action: RuleAction::Deny,
            escalate_target: None,
            blocked_levels: vec!["confidential".to_string(), "secret".to_string()],
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

    pub fn with_semantic_hook(mut self, hook: impl Into<String>) -> Self {
        self.semantic_hook = hook.into();
        self
    }

    pub fn with_action(mut self, action: RuleAction) -> Self {
        self.action = action;
        self
    }

    pub fn with_escalate_target(mut self, target: impl Into<String>) -> Self {
        self.escalate_target = Some(target.into());
        self
    }

    pub fn with_blocked_levels(mut self, levels: Vec<String>) -> Self {
        self.blocked_levels = levels;
        self
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn is_level_blocked(&self, level: &str) -> bool {
        self.blocked_levels
            .iter()
            .any(|l| l.eq_ignore_ascii_case(level))
    }
}

impl RuleInstance for RAGDocSensitivityRule {
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
        RuleFamilyId::RAGDocSensitivity
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
