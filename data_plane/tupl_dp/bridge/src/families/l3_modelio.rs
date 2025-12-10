//! # L3 Model I/O Layer Rule Families
//!
//! Defines rule structures for Model I/O controls:
//! - ModelOutputScanRule: Scan output for PII/sensitive content
//! - ModelOutputEscalateRule: Escalate uncertain responses

use crate::types::{now_ms, LayerId, RuleAction, RuleFamilyId, RuleInstance, RuleScope};
use std::sync::Arc;

// ================================================================================================
// MODEL OUTPUT SCAN RULE
// ================================================================================================

/// Scans model output for PII, jailbreak, or sensitive content
#[derive(Debug, Clone)]
pub struct ModelOutputScanRule {
    pub rule_id: String,
    pub priority: u32,
    pub scope: RuleScope,

    /// WASM module reference for semantic scanning
    pub semantic_hook: String,

    /// Maximum execution time in milliseconds
    pub max_exec_ms: u32,

    /// Action on detection
    pub action: RuleAction,

    /// Redaction template (if action = REDACT)
    pub redact_template: Option<String>,

    /// Escalation target (if action = ESCALATE)
    pub escalate_target: Option<String>,

    pub created_at: u64,
    pub description: Option<String>,
    pub enabled: bool,
}

impl ModelOutputScanRule {
    pub fn new(rule_id: impl Into<String>) -> Self {
        ModelOutputScanRule {
            rule_id: rule_id.into(),
            priority: 0,
            scope: RuleScope::global(),
            semantic_hook: String::new(),
            max_exec_ms: 30,
            action: RuleAction::Redact,
            redact_template: Some("[REDACTED]".to_string()),
            escalate_target: None,
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

    pub fn with_max_exec_ms(mut self, ms: u32) -> Self {
        self.max_exec_ms = ms;
        self
    }

    pub fn with_action(mut self, action: RuleAction) -> Self {
        self.action = action;
        self
    }

    pub fn with_redact_template(mut self, template: impl Into<String>) -> Self {
        self.redact_template = Some(template.into());
        self
    }

    pub fn with_escalate_target(mut self, target: impl Into<String>) -> Self {
        self.escalate_target = Some(target.into());
        self
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

impl RuleInstance for ModelOutputScanRule {
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
        RuleFamilyId::ModelOutputScan
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
// MODEL OUTPUT ESCALATE RULE
// ================================================================================================

/// Diverts uncertain or low-confidence responses to human review
#[derive(Debug, Clone)]
pub struct ModelOutputEscalateRule {
    pub rule_id: String,
    pub priority: u32,
    pub scope: RuleScope,

    /// Confidence threshold (0.0 - 1.0)
    pub confidence_threshold: f32,

    /// Escalation target (e.g., "human-review")
    pub escalate_target: String,

    /// WASM module for confidence assessment
    pub semantic_hook: Option<String>,

    pub created_at: u64,
    pub description: Option<String>,
    pub enabled: bool,
}

impl ModelOutputEscalateRule {
    pub fn new(rule_id: impl Into<String>) -> Self {
        ModelOutputEscalateRule {
            rule_id: rule_id.into(),
            priority: 0,
            scope: RuleScope::global(),
            confidence_threshold: 0.5,
            escalate_target: "human-review".to_string(),
            semantic_hook: None,
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

    pub fn with_confidence_threshold(mut self, threshold: f32) -> Self {
        self.confidence_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    pub fn with_escalate_target(mut self, target: impl Into<String>) -> Self {
        self.escalate_target = target.into();
        self
    }

    pub fn with_semantic_hook(mut self, hook: impl Into<String>) -> Self {
        self.semantic_hook = Some(hook.into());
        self
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn should_escalate(&self, confidence: f32) -> bool {
        confidence < self.confidence_threshold
    }
}

impl RuleInstance for ModelOutputEscalateRule {
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
        RuleFamilyId::ModelOutputEscalate
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
