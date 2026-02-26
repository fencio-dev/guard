use serde_json::Value;

use crate::types::{PolicyType, RuleInstance, RuleScope};

/// Lightweight rule instance representing a DesignBoundary-derived rule.
#[derive(Debug)]
pub struct DesignBoundaryRule {
    rule_id: String,
    priority: u32,
    scope: RuleScope,
    layer: Option<String>,
    created_at_ms: u64,
    description: Option<String>,
    enabled: bool,
    params: Value,
    /// AARM policy classification for this rule.
    policy_type: PolicyType,
    /// Drift threshold; 0.0 means drift enforcement is disabled.
    drift_threshold: f32,
    /// Optional JSON patch applied when decision is MODIFY.
    modification_spec: Option<Value>,
    /// Per-slice weights [action, resource, data, risk].
    slice_weights: [f32; 4],
}

impl DesignBoundaryRule {
    pub fn new(
        rule_id: String,
        priority: u32,
        scope: RuleScope,
        layer: Option<String>,
        created_at_ms: u64,
        enabled: bool,
        description: Option<String>,
        params: Value,
    ) -> Self {
        Self {
            rule_id,
            priority,
            scope,
            layer,
            created_at_ms,
            description,
            enabled,
            params,
            policy_type: PolicyType::default(),
            drift_threshold: 0.0,
            modification_spec: None,
            slice_weights: [0.25; 4],
        }
    }

    /// Construct with explicit AARM policy fields.
    pub fn new_with_policy(
        rule_id: String,
        priority: u32,
        scope: RuleScope,
        layer: Option<String>,
        created_at_ms: u64,
        enabled: bool,
        description: Option<String>,
        params: Value,
        policy_type: PolicyType,
        drift_threshold: f32,
        modification_spec: Option<Value>,
        slice_weights: [f32; 4],
    ) -> Self {
        Self {
            rule_id,
            priority,
            scope,
            layer,
            created_at_ms,
            description,
            enabled,
            params,
            policy_type,
            drift_threshold,
            modification_spec,
            slice_weights,
        }
    }
}

impl RuleInstance for DesignBoundaryRule {
    fn rule_id(&self) -> &str {
        &self.rule_id
    }

    fn priority(&self) -> u32 {
        self.priority
    }

    fn scope(&self) -> &RuleScope {
        &self.scope
    }

    fn layer(&self) -> Option<&str> {
        self.layer.as_deref()
    }

    fn created_at(&self) -> u64 {
        self.created_at_ms
    }

    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn management_plane_payload(&self) -> Value {
        self.params.clone()
    }

    fn policy_type(&self) -> PolicyType {
        self.policy_type.clone()
    }

    fn drift_threshold(&self) -> f32 {
        self.drift_threshold
    }

    fn modification_spec(&self) -> Option<&Value> {
        self.modification_spec.as_ref()
    }

    fn slice_weights(&self) -> [f32; 4] {
        self.slice_weights
    }
}
