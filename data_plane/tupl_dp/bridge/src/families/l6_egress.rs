//! # L6 Egress Layer Rule Families
//!
//! Defines rule structures for Egress controls:
//! - OutputPIIRule: Detect and redact PII
//! - OutputAuditRule: Emit decision records

use crate::types::{now_ms, LayerId, RuleAction, RuleFamilyId, RuleInstance, RuleScope};
use std::sync::Arc;

// ================================================================================================
// OUTPUT PII RULE
// ================================================================================================

/// Detects and redacts/denies PII before response leaves system
#[derive(Debug, Clone)]
pub struct OutputPIIRule {
    pub rule_id: String,
    pub priority: u32,
    pub scope: RuleScope,

    /// WASM module for PII detection
    pub semantic_hook: String,

    /// Action on detection (REDACT, DENY)
    pub action: RuleAction,

    /// Redaction template
    pub redact_template: String,

    /// PII types to detect
    pub pii_types: Vec<String>,

    pub created_at: u64,
    pub description: Option<String>,
    pub enabled: bool,
}

impl OutputPIIRule {
    pub fn new(rule_id: impl Into<String>) -> Self {
        OutputPIIRule {
            rule_id: rule_id.into(),
            priority: 0,
            scope: RuleScope::global(),
            semantic_hook: "pii-detector-v1".to_string(),
            action: RuleAction::Redact,
            redact_template: "[REDACTED]".to_string(),
            pii_types: vec![
                "email".to_string(),
                "ssn".to_string(),
                "phone".to_string(),
                "credit_card".to_string(),
            ],
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

    pub fn with_redact_template(mut self, template: impl Into<String>) -> Self {
        self.redact_template = template.into();
        self
    }

    pub fn with_pii_types(mut self, types: Vec<String>) -> Self {
        self.pii_types = types;
        self
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn should_detect_type(&self, pii_type: &str) -> bool {
        self.pii_types.is_empty()
            || self
                .pii_types
                .iter()
                .any(|t| t.eq_ignore_ascii_case(pii_type))
    }
}

impl RuleInstance for OutputPIIRule {
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
        RuleFamilyId::OutputPII
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
// OUTPUT AUDIT RULE
// ================================================================================================

/// Emits decision record for final user-facing outputs
#[derive(Debug, Clone)]
pub struct OutputAuditRule {
    pub rule_id: String,
    pub priority: u32,
    pub scope: RuleScope,

    /// Whether to emit decision events
    pub emit_decision_event: bool,

    /// Sampling rate (0.0 - 1.0)
    pub sampling_rate: f32,

    /// Audit fields to include
    pub audit_fields: Vec<String>,

    /// Destination for audit logs
    pub audit_destination: Option<String>,

    pub created_at: u64,
    pub description: Option<String>,
    pub enabled: bool,
}

impl OutputAuditRule {
    pub fn new(rule_id: impl Into<String>) -> Self {
        OutputAuditRule {
            rule_id: rule_id.into(),
            priority: 0,
            scope: RuleScope::global(),
            emit_decision_event: true,
            sampling_rate: 1.0,
            audit_fields: vec![
                "rule_id".to_string(),
                "agent_id".to_string(),
                "timestamp".to_string(),
                "action".to_string(),
            ],
            audit_destination: None,
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

    pub fn with_emit_decision_event(mut self, emit: bool) -> Self {
        self.emit_decision_event = emit;
        self
    }

    pub fn with_sampling_rate(mut self, rate: f32) -> Self {
        self.sampling_rate = rate.clamp(0.0, 1.0);
        self
    }

    pub fn with_audit_fields(mut self, fields: Vec<String>) -> Self {
        self.audit_fields = fields;
        self
    }

    pub fn with_audit_destination(mut self, dest: impl Into<String>) -> Self {
        self.audit_destination = Some(dest.into());
        self
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn should_audit(&self) -> bool {
        use rand::Rng;
        self.emit_decision_event && rand::thread_rng().gen::<f32>() < self.sampling_rate
    }
}

impl RuleInstance for OutputAuditRule {
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
        RuleFamilyId::OutputAudit
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
