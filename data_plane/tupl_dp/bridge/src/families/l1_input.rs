// L1 Input rule families
// Defines rule structures for the input layer:
//! - InputSchemaRule: Enforce payload schema, size and type
//! - InputSanitizationRule: Sanitize and validate input data

use crate::types::{now_ms, LayerId, RuleAction, RuleFamilyId, RuleInstance, RuleScope};
use std::sync::Arc;

// ================================================================================================
// INPUT SCHEMA RULE
// ================================================================================================

/// Enforces payload schema, size, and type validation
///
/// # Fields
/// - `schema_ref`: JSONSchema identifier for validation
/// - `payload_dtype`: Expected data type (string identifier)
/// - `max_bytes`: Maximum payload size in bytes
/// - `action`: Action on validation failure (ALLOW, DENY, REWRITE)
///
/// # Matching
/// - Syntactic match: validate_json(schema_ref)
///

#[derive(Debug, Clone)]
pub struct InputSchemaRule {
    /// Unique rule identifier
    pub rule_id: String,

    /// Priority (higher = evaluated first)
    pub priority: u32,

    /// Rule scope (which agents this applies to)
    pub scope: RuleScope,

    /// JSONSchema reference ID
    pub schema_ref: String,

    /// Expected payload data type
    pub payload_dtype: String,

    /// Maximum payload size in bytes
    pub max_bytes: u32,

    /// Action on validation failure
    pub action: RuleAction,

    /// Creation timestamp
    pub created_at: u64,

    /// Optional description
    pub description: Option<String>,

    /// Whether rule is enabled
    pub enabled: bool,
}

impl InputSchemaRule {
    /// Creates a new InputSchemaRule with defaults
    pub fn new(rule_id: impl Into<String>) -> Self {
        InputSchemaRule {
            rule_id: rule_id.into(),
            priority: 0,
            scope: RuleScope::global(),
            schema_ref: String::new(),
            payload_dtype: "application/json".to_string(),
            max_bytes: 1_000_000, // 1MB default
            action: RuleAction::Deny,
            created_at: now_ms(),
            description: None,
            enabled: true,
        }
    }

    // Builder methods
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

    pub fn with_schema_ref(mut self, schema_ref: impl Into<String>) -> Self {
        self.schema_ref = schema_ref.into();
        self
    }

    pub fn with_payload_dtype(mut self, dtype: impl Into<String>) -> Self {
        self.payload_dtype = dtype.into();
        self
    }

    pub fn with_max_bytes(mut self, max_bytes: u32) -> Self {
        self.max_bytes = max_bytes;
        self
    }

    pub fn with_action(mut self, action: RuleAction) -> Self {
        self.action = action;
        self
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Checks if payload size is within limits
    pub fn is_size_valid(&self, payload_size: usize) -> bool {
        payload_size <= self.max_bytes as usize
    }
}

impl RuleInstance for InputSchemaRule {
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
        RuleFamilyId::InputSchema
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
// INPUT SANITIZATION RULE
// ================================================================================================

/// Sanitizes and validates input data for security
///
/// # Fields
/// - `patterns_to_strip`: Regex patterns to remove from input
/// - `allowed_chars`: Character whitelist (if specified)
/// - `blocked_patterns`: Patterns that trigger rejection
/// - `max_length`: Maximum string length after sanitization
/// - `action`: Action on detection (DENY, REWRITE)
///
/// # Matching
/// - Syntactic match: Pattern matching against input
///

#[derive(Debug, Clone)]
pub struct InputSanitizationRule {
    /// Unique rule identifier
    pub rule_id: String,

    /// Priority (higher = evaluated first)
    pub priority: u32,

    /// Rule scope (which agents this applies to)
    pub scope: RuleScope,

    /// Patterns to strip from input (regex strings)
    pub patterns_to_strip: Vec<String>,

    /// Character whitelist (empty = no restriction)
    pub allowed_chars: Option<String>,

    /// Blocked patterns that trigger rejection
    pub blocked_patterns: Vec<String>,

    /// Maximum length after sanitization
    pub max_length: Option<usize>,

    /// Action on detection
    pub action: RuleAction,

    /// Whether to normalize unicode
    pub normalize_unicode: bool,

    /// Whether to trim whitespace
    pub trim_whitespace: bool,

    /// Creation timestamp
    pub created_at: u64,

    /// Optional description
    pub description: Option<String>,

    /// Whether rule is enabled
    pub enabled: bool,
}

impl InputSanitizationRule {
    /// Creates a new InputSanitizationRule with defaults
    pub fn new(rule_id: impl Into<String>) -> Self {
        InputSanitizationRule {
            rule_id: rule_id.into(),
            priority: 0,
            scope: RuleScope::global(),
            patterns_to_strip: vec![],
            allowed_chars: None,
            blocked_patterns: vec![],
            max_length: None,
            action: RuleAction::Deny,
            normalize_unicode: true,
            trim_whitespace: true,
            created_at: now_ms(),
            description: None,
            enabled: true,
        }
    }

    // Builder methods
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

    pub fn with_patterns_to_strip(mut self, patterns: Vec<String>) -> Self {
        self.patterns_to_strip = patterns;
        self
    }

    pub fn with_allowed_chars(mut self, chars: impl Into<String>) -> Self {
        self.allowed_chars = Some(chars.into());
        self
    }

    pub fn with_blocked_patterns(mut self, patterns: Vec<String>) -> Self {
        self.blocked_patterns = patterns;
        self
    }

    pub fn with_max_length(mut self, length: usize) -> Self {
        self.max_length = Some(length);
        self
    }

    pub fn with_action(mut self, action: RuleAction) -> Self {
        self.action = action;
        self
    }

    pub fn with_normalize_unicode(mut self, normalize: bool) -> Self {
        self.normalize_unicode = normalize;
        self
    }

    pub fn with_trim_whitespace(mut self, trim: bool) -> Self {
        self.trim_whitespace = trim;
        self
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Checks if input contains any blocked patterns (simple substring check)
    pub fn contains_blocked_pattern(&self, input: &str) -> bool {
        let lower_input = input.to_lowercase();
        self.blocked_patterns
            .iter()
            .any(|pattern| lower_input.contains(&pattern.to_lowercase()))
    }

    /// Checks if input length is within limits
    pub fn is_length_valid(&self, input: &str) -> bool {
        match self.max_length {
            Some(max) => input.len() <= max,
            None => true,
        }
    }
}

impl RuleInstance for InputSanitizationRule {
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
        RuleFamilyId::InputSanitize
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
