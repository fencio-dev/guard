//! # Rule Converter Module
//!
//! Converts control plane rules (from gRPC) to native Rust rule instances.
//! This module handles the conversion for all 14 rule families.

use crate::types::{NetworkProtocol, RuleAction, RuleFamilyId, RuleInstance, RuleScope};
use std::collections::HashMap;
use std::sync::Arc;

// Import all rule family types
use crate::families::l1_input::{InputSanitizationRule, InputSchemaRule};
use crate::families::l2_planner::{PromptAssemblyRule, PromptLengthRule};
use crate::families::l3_modelio::{ModelOutputEscalateRule, ModelOutputScanRule};
use crate::families::l4_tool_gateway::{ToolParamConstraintRule, ToolWhitelistRule};
use crate::families::l5_rag::{RAGDocSensitivityRule, RAGSourceRule};
use crate::families::l6_egress::{OutputAuditRule, OutputPIIRule};
use crate::families::lo_system::{NetworkEgressRule, SidecarSpawnRule};

// ================================================================================================
// CONTROL PLANE RULE REPRESENTATION
// ================================================================================================

/// Represents a rule from the control plane
#[derive(Debug, Clone)]
pub struct ControlPlaneRule {
    pub rule_id: String,
    pub family_id: String,
    pub layer: String,
    pub agent_id: String,
    pub priority: i32,
    pub enabled: bool,
    pub created_at_ms: i64,
    pub params: HashMap<String, ParamValue>,
}

/// Parameter value from control plane
#[derive(Debug, Clone)]
pub enum ParamValue {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    StringList(Vec<String>),
}

impl ParamValue {
    pub fn as_string(&self) -> Option<String> {
        match self {
            ParamValue::String(s) => Some(s.clone()),
            _ => None,
        }
    }

    pub fn as_string_or_default(&self, default: &str) -> String {
        self.as_string().unwrap_or_else(|| default.to_string())
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            ParamValue::Int(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_int_or_default(&self, default: i64) -> i64 {
        self.as_int().unwrap_or(default)
    }

    pub fn as_float(&self) -> Option<f64> {
        match self {
            ParamValue::Float(f) => Some(*f),
            _ => None,
        }
    }

    pub fn as_float_or_default(&self, default: f64) -> f64 {
        self.as_float().unwrap_or(default)
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ParamValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_bool_or_default(&self, default: bool) -> bool {
        self.as_bool().unwrap_or(default)
    }

    pub fn as_string_list(&self) -> Option<Vec<String>> {
        match self {
            ParamValue::StringList(list) => Some(list.clone()),
            _ => None,
        }
    }

    pub fn as_string_list_or_default(&self) -> Vec<String> {
        self.as_string_list().unwrap_or_default()
    }
}

// ================================================================================================
// RULE CONVERTER
// ================================================================================================

pub struct RuleConverter;

impl RuleConverter {
    /// Convert a control plane rule to a bridge rule instance
    pub fn convert(cp_rule: &ControlPlaneRule) -> Result<Arc<dyn RuleInstance>, String> {
        // Parse family ID
        let family_id = Self::parse_family_id(&cp_rule.family_id)?;

        // Create scope
        let scope = RuleScope::for_agent(cp_rule.agent_id.clone());

        // Convert based on family
        match family_id {
            // L0 - System Layer
            RuleFamilyId::NetworkEgress => Self::convert_network_egress(cp_rule, scope),
            RuleFamilyId::SidecarSpawn => Self::convert_sidecar_spawn(cp_rule, scope),

            // L1 - Input Layer
            RuleFamilyId::InputSchema => Self::convert_input_schema(cp_rule, scope),
            RuleFamilyId::InputSanitize => Self::convert_input_sanitize(cp_rule, scope),

            // L2 - Planner Layer
            RuleFamilyId::PromptAssembly => Self::convert_prompt_assembly(cp_rule, scope),
            RuleFamilyId::PromptLength => Self::convert_prompt_length(cp_rule, scope),

            // L3 - Model I/O Layer
            RuleFamilyId::ModelOutputScan => Self::convert_model_output_scan(cp_rule, scope),
            RuleFamilyId::ModelOutputEscalate => {
                Self::convert_model_output_escalate(cp_rule, scope)
            }

            // L4 - Tool Gateway Layer
            RuleFamilyId::ToolWhitelist => Self::convert_tool_whitelist(cp_rule, scope),
            RuleFamilyId::ToolParamConstraint => {
                Self::convert_tool_param_constraint(cp_rule, scope)
            }

            // L5 - RAG Layer
            RuleFamilyId::RAGSource => Self::convert_rag_source(cp_rule, scope),
            RuleFamilyId::RAGDocSensitivity => Self::convert_rag_doc_sensitivity(cp_rule, scope),

            // L6 - Egress Layer
            RuleFamilyId::OutputPII => Self::convert_output_pii(cp_rule, scope),
            RuleFamilyId::OutputAudit => Self::convert_output_audit(cp_rule, scope),
        }
    }

    /// Parse family ID string to RuleFamilyId enum
    fn parse_family_id(family_str: &str) -> Result<RuleFamilyId, String> {
        match family_str {
            "net_egress" => Ok(RuleFamilyId::NetworkEgress),
            "sidecar_spawn" => Ok(RuleFamilyId::SidecarSpawn),
            "input_schema" => Ok(RuleFamilyId::InputSchema),
            "input_sanitize" => Ok(RuleFamilyId::InputSanitize),
            "prompt_assembly" => Ok(RuleFamilyId::PromptAssembly),
            "prompt_length" => Ok(RuleFamilyId::PromptLength),
            "model_output_scan" => Ok(RuleFamilyId::ModelOutputScan),
            "model_output_escalate" => Ok(RuleFamilyId::ModelOutputEscalate),
            "tool_whitelist" => Ok(RuleFamilyId::ToolWhitelist),
            "tool_param_constraint" => Ok(RuleFamilyId::ToolParamConstraint),
            "rag_source" => Ok(RuleFamilyId::RAGSource),
            "rag_doc_sensitivity" => Ok(RuleFamilyId::RAGDocSensitivity),
            "output_pii" => Ok(RuleFamilyId::OutputPII),
            "output_audit" => Ok(RuleFamilyId::OutputAudit),
            _ => Err(format!("Unknown family ID: {}", family_str)),
        }
    }

    // ============================================================================================
    // L0 - SYSTEM LAYER CONVERTERS
    // ============================================================================================

    fn convert_network_egress(
        cp_rule: &ControlPlaneRule,
        scope: RuleScope,
    ) -> Result<Arc<dyn RuleInstance>, String> {
        let dest_domains = cp_rule
            .params
            .get("dest_domains")
            .and_then(|v| v.as_string_list())
            .unwrap_or_default();

        let protocol_str = cp_rule
            .params
            .get("protocol")
            .and_then(|v| v.as_string())
            .unwrap_or_else(|| "HTTPS".to_string());

        let protocol = match protocol_str.as_str() {
            "TCP" => NetworkProtocol::TCP,
            "UDP" => NetworkProtocol::UDP,
            "HTTP" => NetworkProtocol::HTTP,
            "HTTPS" => NetworkProtocol::HTTPS,
            _ => NetworkProtocol::HTTPS,
        };

        let action_str = cp_rule
            .params
            .get("action")
            .and_then(|v| v.as_string())
            .unwrap_or_else(|| "DENY".to_string());

        let action = Self::parse_action(&action_str)?;

        let mut rule = NetworkEgressRule::new(cp_rule.rule_id.clone())
            .with_priority(cp_rule.priority as u32)
            .with_scope(scope)
            .with_dest_domains(dest_domains)
            .with_protocol(protocol)
            .with_action(action);

        // Optional port range
        if let (Some(min), Some(max)) = (
            cp_rule.params.get("port_min").and_then(|v| v.as_int()),
            cp_rule.params.get("port_max").and_then(|v| v.as_int()),
        ) {
            rule = rule.with_port_range(min as u16, max as u16);
        }

        // Optional redirect target
        if let Some(redirect) = cp_rule
            .params
            .get("redirect_target")
            .and_then(|v| v.as_string())
        {
            rule = rule.with_redirect_target(redirect);
        }

        Ok(Arc::new(rule) as Arc<dyn RuleInstance>)
    }

    fn convert_sidecar_spawn(
        cp_rule: &ControlPlaneRule,
        scope: RuleScope,
    ) -> Result<Arc<dyn RuleInstance>, String> {
        let allowed_images = cp_rule
            .params
            .get("allowed_images")
            .and_then(|v| v.as_string_list())
            .unwrap_or_default();

        let mut rule = SidecarSpawnRule::new(cp_rule.rule_id.clone())
            .with_priority(cp_rule.priority as u32)
            .with_scope(scope)
            .with_allowed_images(allowed_images);

        // Optional constraints
        if let Some(ttl) = cp_rule.params.get("max_ttl").and_then(|v| v.as_int()) {
            rule = rule.with_max_ttl(ttl as u32);
        }

        if let Some(instances) = cp_rule.params.get("max_instances").and_then(|v| v.as_int()) {
            rule = rule.with_max_instances(instances as u32);
        }

        if let Some(cpu) = cp_rule.params.get("cpu_limit").and_then(|v| v.as_int()) {
            rule = rule.with_cpu_limit(cpu as u32);
        }

        if let Some(mem) = cp_rule.params.get("mem_limit").and_then(|v| v.as_int()) {
            rule = rule.with_mem_limit(mem as u32);
        }

        Ok(Arc::new(rule) as Arc<dyn RuleInstance>)
    }

    // ============================================================================================
    // L1 - INPUT LAYER CONVERTERS
    // ============================================================================================

    fn convert_input_schema(
        cp_rule: &ControlPlaneRule,
        scope: RuleScope,
    ) -> Result<Arc<dyn RuleInstance>, String> {
        let schema_ref = cp_rule
            .params
            .get("schema_ref")
            .and_then(|v| v.as_string())
            .unwrap_or_default();

        let payload_dtype = cp_rule
            .params
            .get("payload_dtype")
            .and_then(|v| v.as_string())
            .unwrap_or_else(|| "application/json".to_string());

        let max_bytes = cp_rule
            .params
            .get("max_bytes")
            .and_then(|v| v.as_int())
            .unwrap_or(1_000_000) as u32;

        let action_str = cp_rule
            .params
            .get("action")
            .and_then(|v| v.as_string())
            .unwrap_or_else(|| "DENY".to_string());

        let action = Self::parse_action(&action_str)?;

        let rule = InputSchemaRule::new(cp_rule.rule_id.clone())
            .with_priority(cp_rule.priority as u32)
            .with_scope(scope)
            .with_schema_ref(schema_ref)
            .with_payload_dtype(payload_dtype)
            .with_max_bytes(max_bytes)
            .with_action(action);

        Ok(Arc::new(rule) as Arc<dyn RuleInstance>)
    }

    fn convert_input_sanitize(
        cp_rule: &ControlPlaneRule,
        scope: RuleScope,
    ) -> Result<Arc<dyn RuleInstance>, String> {
        let strip_fields = cp_rule
            .params
            .get("strip_fields")
            .and_then(|v| v.as_string_list())
            .unwrap_or_default();

        let max_depth = cp_rule
            .params
            .get("max_depth")
            .and_then(|v| v.as_int())
            .unwrap_or(10) as u32;

        let normalize_unicode = cp_rule
            .params
            .get("normalize_unicode")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let rule = InputSanitizationRule::new(cp_rule.rule_id.clone())
            .with_priority(cp_rule.priority as u32)
            .with_scope(scope)
            .with_patterns_to_strip(strip_fields)
            .with_normalize_unicode(normalize_unicode);

        Ok(Arc::new(rule) as Arc<dyn RuleInstance>)
    }

    // ============================================================================================
    // L2 - PLANNER LAYER CONVERTERS
    // ============================================================================================

    fn convert_prompt_assembly(
        cp_rule: &ControlPlaneRule,
        scope: RuleScope,
    ) -> Result<Arc<dyn RuleInstance>, String> {
        let allowed_context_ids = cp_rule
            .params
            .get("allowed_context_ids")
            .and_then(|v| v.as_string_list())
            .unwrap_or_default();

        let enforce_provenance = cp_rule
            .params
            .get("enforce_provenance")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let max_prompt_tokens = cp_rule
            .params
            .get("max_prompt_tokens")
            .and_then(|v| v.as_int())
            .unwrap_or(8192) as u32;

        let rule = PromptAssemblyRule::new(cp_rule.rule_id.clone())
            .with_priority(cp_rule.priority as u32)
            .with_scope(scope)
            .with_allowed_context_ids(allowed_context_ids)
            .with_enforce_provenance(enforce_provenance)
            .with_max_prompt_tokens(max_prompt_tokens);

        Ok(Arc::new(rule) as Arc<dyn RuleInstance>)
    }

    fn convert_prompt_length(
        cp_rule: &ControlPlaneRule,
        scope: RuleScope,
    ) -> Result<Arc<dyn RuleInstance>, String> {
        let max_prompt_tokens = cp_rule
            .params
            .get("max_prompt_tokens")
            .and_then(|v| v.as_int())
            .unwrap_or(8192) as u32;

        let rule = PromptLengthRule::new(cp_rule.rule_id.clone())
            .with_priority(cp_rule.priority as u32)
            .with_scope(scope)
            .with_max_prompt_tokens(max_prompt_tokens);

        Ok(Arc::new(rule) as Arc<dyn RuleInstance>)
    }

    // ============================================================================================
    // L3 - MODEL I/O LAYER CONVERTERS
    // ============================================================================================

    fn convert_model_output_scan(
        cp_rule: &ControlPlaneRule,
        scope: RuleScope,
    ) -> Result<Arc<dyn RuleInstance>, String> {
        let semantic_hook = cp_rule
            .params
            .get("semantic_hook")
            .and_then(|v| v.as_string())
            .unwrap_or_else(|| "default-scanner".to_string());

        let max_exec_ms = cp_rule
            .params
            .get("max_exec_ms")
            .and_then(|v| v.as_int())
            .unwrap_or(40) as u32;

        let action_str = cp_rule
            .params
            .get("action")
            .and_then(|v| v.as_string())
            .unwrap_or_else(|| "REDACT".to_string());

        let action = Self::parse_action(&action_str)?;

        let redact_template = cp_rule
            .params
            .get("redact_template")
            .and_then(|v| v.as_string())
            .unwrap_or_else(|| "[REDACTED]".to_string());

        let rule = ModelOutputScanRule::new(cp_rule.rule_id.clone())
            .with_priority(cp_rule.priority as u32)
            .with_scope(scope)
            .with_semantic_hook(semantic_hook)
            .with_max_exec_ms(max_exec_ms)
            .with_action(action)
            .with_redact_template(redact_template);

        Ok(Arc::new(rule) as Arc<dyn RuleInstance>)
    }

    fn convert_model_output_escalate(
        cp_rule: &ControlPlaneRule,
        scope: RuleScope,
    ) -> Result<Arc<dyn RuleInstance>, String> {
        let confidence_threshold = cp_rule
            .params
            .get("confidence_threshold")
            .and_then(|v| v.as_float())
            .unwrap_or(0.75) as f32;

        let escalate_target = cp_rule
            .params
            .get("escalate_target")
            .and_then(|v| v.as_string())
            .unwrap_or_else(|| "human-review".to_string());

        let rule = ModelOutputEscalateRule::new(cp_rule.rule_id.clone())
            .with_priority(cp_rule.priority as u32)
            .with_scope(scope)
            .with_confidence_threshold(confidence_threshold)
            .with_escalate_target(escalate_target);

        Ok(Arc::new(rule) as Arc<dyn RuleInstance>)
    }

    // ============================================================================================
    // L4 - TOOL GATEWAY LAYER CONVERTERS
    // ============================================================================================

    fn convert_tool_whitelist(
        cp_rule: &ControlPlaneRule,
        scope: RuleScope,
    ) -> Result<Arc<dyn RuleInstance>, String> {
        let allowed_tool_ids = cp_rule
            .params
            .get("allowed_tool_ids")
            .and_then(|v| v.as_string_list())
            .unwrap_or_default();

        let allowed_methods = cp_rule
            .params
            .get("allowed_methods")
            .and_then(|v| v.as_string_list())
            .unwrap_or_default();

        let mut rule = ToolWhitelistRule::new(cp_rule.rule_id.clone())
            .with_priority(cp_rule.priority as u32)
            .with_scope(scope)
            .with_allowed_tool_ids(allowed_tool_ids)
            .with_allowed_methods(allowed_methods);

        if let Some(rate_limit) = cp_rule
            .params
            .get("rate_limit_per_min")
            .and_then(|v| v.as_int())
        {
            rule = rule.with_rate_limit_per_min(rate_limit as u32);
        }

        Ok(Arc::new(rule) as Arc<dyn RuleInstance>)
    }

    fn convert_tool_param_constraint(
        cp_rule: &ControlPlaneRule,
        scope: RuleScope,
    ) -> Result<Arc<dyn RuleInstance>, String> {
        let tool_id = cp_rule
            .params
            .get("tool_id")
            .and_then(|v| v.as_string())
            .ok_or("tool_id parameter required for ToolParamConstraintRule")?;

        let param_name = cp_rule
            .params
            .get("param_name")
            .and_then(|v| v.as_string())
            .unwrap_or_else(|| "*".to_string());

        let mut rule = ToolParamConstraintRule::new(cp_rule.rule_id.clone())
            .with_tool_id(tool_id)
            .with_param_name(param_name)
            .with_priority(cp_rule.priority as u32)
            .with_scope(scope);

        // Add optional constraints
        if let Some(regex) = cp_rule.params.get("regex").and_then(|v| v.as_string()) {
            rule = rule.with_regex(regex);
        }

        if let Some(max_len) = cp_rule.params.get("max_len").and_then(|v| v.as_int()) {
            rule = rule.with_max_len(max_len as usize);
        }

        if let Some(min_val) = cp_rule.params.get("min_value").and_then(|v| v.as_float()) {
            rule = rule.with_min_value(min_val);
        }

        if let Some(max_val) = cp_rule.params.get("max_value").and_then(|v| v.as_float()) {
            rule = rule.with_max_value(max_val);
        }

        Ok(Arc::new(rule) as Arc<dyn RuleInstance>)
    }

    // ============================================================================================
    // L5 - RAG LAYER CONVERTERS
    // ============================================================================================

    fn convert_rag_source(
        cp_rule: &ControlPlaneRule,
        scope: RuleScope,
    ) -> Result<Arc<dyn RuleInstance>, String> {
        let allowed_sources = cp_rule
            .params
            .get("allowed_sources")
            .and_then(|v| v.as_string_list())
            .unwrap_or_default();

        let max_docs = cp_rule
            .params
            .get("max_docs")
            .and_then(|v| v.as_int())
            .unwrap_or(5) as u32;

        let max_tokens_per_doc = cp_rule
            .params
            .get("max_tokens_per_doc")
            .and_then(|v| v.as_int())
            .unwrap_or(1000) as u32;

        let rule = RAGSourceRule::new(cp_rule.rule_id.clone())
            .with_priority(cp_rule.priority as u32)
            .with_scope(scope)
            .with_allowed_sources(allowed_sources)
            .with_max_docs(max_docs)
            .with_max_tokens_per_doc(max_tokens_per_doc);

        Ok(Arc::new(rule) as Arc<dyn RuleInstance>)
    }

    fn convert_rag_doc_sensitivity(
        cp_rule: &ControlPlaneRule,
        scope: RuleScope,
    ) -> Result<Arc<dyn RuleInstance>, String> {
        let semantic_hook = cp_rule
            .params
            .get("semantic_hook")
            .and_then(|v| v.as_string())
            .unwrap_or_else(|| "sensitivity-classifier-v1".to_string());

        let action_str = cp_rule
            .params
            .get("action")
            .and_then(|v| v.as_string())
            .unwrap_or_else(|| "DENY".to_string());

        let action = Self::parse_action(&action_str)?;

        let rule = RAGDocSensitivityRule::new(cp_rule.rule_id.clone())
            .with_priority(cp_rule.priority as u32)
            .with_scope(scope)
            .with_semantic_hook(semantic_hook)
            .with_action(action);

        Ok(Arc::new(rule) as Arc<dyn RuleInstance>)
    }

    // ============================================================================================
    // L6 - EGRESS LAYER CONVERTERS
    // ============================================================================================

    fn convert_output_pii(
        cp_rule: &ControlPlaneRule,
        scope: RuleScope,
    ) -> Result<Arc<dyn RuleInstance>, String> {
        let semantic_hook = cp_rule
            .params
            .get("semantic_hook")
            .and_then(|v| v.as_string())
            .unwrap_or_else(|| "pii-detector-v1".to_string());

        let action_str = cp_rule
            .params
            .get("action")
            .and_then(|v| v.as_string())
            .unwrap_or_else(|| "REDACT".to_string());

        let action = Self::parse_action(&action_str)?;

        let redact_template = cp_rule
            .params
            .get("redact_template")
            .and_then(|v| v.as_string())
            .unwrap_or_else(|| "[REDACTED]".to_string());

        let rule = OutputPIIRule::new(cp_rule.rule_id.clone())
            .with_priority(cp_rule.priority as u32)
            .with_scope(scope)
            .with_semantic_hook(semantic_hook)
            .with_action(action)
            .with_redact_template(redact_template);

        Ok(Arc::new(rule) as Arc<dyn RuleInstance>)
    }

    fn convert_output_audit(
        cp_rule: &ControlPlaneRule,
        scope: RuleScope,
    ) -> Result<Arc<dyn RuleInstance>, String> {
        let emit_decision_event = cp_rule
            .params
            .get("emit_decision_event")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let sampling_rate = cp_rule
            .params
            .get("sampling_rate")
            .and_then(|v| v.as_float())
            .unwrap_or(1.0) as f32;

        let rule = OutputAuditRule::new(cp_rule.rule_id.clone())
            .with_priority(cp_rule.priority as u32)
            .with_scope(scope)
            .with_emit_decision_event(emit_decision_event)
            .with_sampling_rate(sampling_rate);

        Ok(Arc::new(rule) as Arc<dyn RuleInstance>)
    }

    // ============================================================================================
    // HELPER METHODS
    // ============================================================================================

    fn parse_action(action_str: &str) -> Result<RuleAction, String> {
        match action_str.to_uppercase().as_str() {
            "ALLOW" => Ok(RuleAction::Allow),
            "DENY" => Ok(RuleAction::Deny),
            "REDIRECT" => Ok(RuleAction::Redirect),
            "REDACT" => Ok(RuleAction::Redact),
            "REWRITE" => Ok(RuleAction::Rewrite),
            "DROP_CONTEXT" => Ok(RuleAction::DropContext),
            "TRUNCATE" => Ok(RuleAction::Truncate),
            "ESCALATE" => Ok(RuleAction::Escalate),
            _ => Err(format!("Unknown action: {}", action_str)),
        }
    }
}
