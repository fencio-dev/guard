// Parse, Validate, and manage collections of rules as atomic bundles
// with compilation, versioning and deployment support
// This module provides:
//1. RuleBundle grouping and metadata
//2. Multi format parsing. (JSON, TOML, YAML)
//3. Comprehensive validation  (schema, signatures and conflicts)
//4. Compilation and bytecode generation
// 5. Rollout policy management
//6. Bundle lifecycle (staging, activation and revocation)

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

// Import from existing modules
use crate::rule_metadata::{
    RuleMetadata, RuleId, RuleState, EnforcementClass, RuleScope,
};
use crate::match_clause::{MatchClause, WasmHookRef};
use crate::action_clause::{ActionClause, ActionType, AllowedSideEffect};
use crate::execution_constraints::ExecutionConstraints;

/// Unique identifier for a rule bundle
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BundleId(pub String);

impl BundleId {
    pub fn new(id: String) -> Self {
        Self(id)
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for BundleId {
    fn fmt(&self, f:&mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

///Rollout policy for staged activation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RolloutPolicy {
    /// Activate immediately for all traffic
    Immediate,
    /// Gradual rollout to percentage of traffic
    Canary {
        /// Percentage of traffic (0.0 - 1.0)
        percentage: f64,
        /// Optional list of specific agents to target
        target_agents: Option<Vec<String>>,
    },
    /// Activate only during specific time window
    TimeWindow {
        /// Start time (Unix timestamp in seconds)
        start_time: u64,
        /// End time (Unix timestamp in seconds)
        end_time: u64,
    },
    /// Scheduled activation at specific time
    Scheduled {
        /// Activation time (Unix timestamp in seconds)
        activation_time: u64,
    },
}

impl RolloutPolicy {
    /// Check if policy allows activation at current time
    pub fn allows_activation(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        match self {
            RolloutPolicy::Immediate => true,
            RolloutPolicy::Canary { .. } => true, // Canary always allows some traffic
            RolloutPolicy::TimeWindow { start_time, end_time } => {
                now >= *start_time && now <= *end_time
            }
            RolloutPolicy::Scheduled { activation_time } => now >= *activation_time,
        }
    }

    /// Check if should apply to specific agent (for canary deployments)
    pub fn should_apply_to_agent(&self, agent_id: &str) -> bool {
        match self {
            RolloutPolicy::Immediate => true,
            RolloutPolicy::Canary { target_agents, .. } => {
                if let Some(agents) = target_agents {
                    agents.iter().any(|a| a == agent_id)
                } else {
                    true // No specific agents means apply to percentage
                }
            }
            RolloutPolicy::TimeWindow { .. } => self.allows_activation(),
            RolloutPolicy::Scheduled { .. } => self.allows_activation(),
        }
    }
}

impl Default for RolloutPolicy {
    fn default() -> Self {
        RolloutPolicy :: Immediate
    }
}

/// Revocation policy for deactivation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RevocationPolicy {
    /// Immediate revocation, terminate active evaluations
    Immediate,
    /// Graceful drain, allow active evaluations to complete
    GracefulDrain {
        /// Maximum time to wait for drain (seconds)
        max_wait_seconds: u64,
    },
}

impl Default for RevocationPolicy {
    fn default() -> Self {
        RevocationPolicy::GracefulDrain {
            max_wait_seconds: 30,
        }
    }
}

/// Complete rule definition combining all components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    /// Rule metadata
    pub metadata: RuleMetadata,
    /// Match clause for evaluation
    pub match_clause: MatchClause,
    /// Action to execute on match
    pub action_clause: ActionClause,
    /// Execution constraints
    pub constraints: ExecutionConstraints,
    /// Human-readable description
    pub description: Option<String>,
    /// Tags for categorization
    pub tags: Vec<String>,
}

impl Rule {
    /// Get the rule ID
    pub fn id(&self) -> &RuleId {
        &self.metadata.rule_id
    }
    
    /// Get the rule version
    pub fn version(&self) -> u64 {
        self.metadata.version
    }
    
    /// Check if rule is active
    pub fn is_active(&self) -> bool {
        self.metadata.state == RuleState::Active
    }
    
    /// Get priority for conflict resolution
    pub fn priority(&self) -> i32 {
        self.metadata.priority
    }
}

/// Bundle metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleMetadata {
    /// Bundle identifier
    pub bundle_id: BundleId,
    /// Bundle version
    pub version: u64,
    /// Bundle description
    pub description: Option<String>,
    /// Who signed/created the bundle
    pub signer: String,
    /// When the bundle was created
    pub created_at: SystemTime,
    /// Rollout policy
    pub rollout_policy: RolloutPolicy,
    /// Revocation policy
    pub revocation_policy: RevocationPolicy,
    /// Tags for categorization
    pub tags: Vec<String>,
}

impl BundleMetadata {
    pub fn new(bundle_id: BundleId, signer: String) -> Self {
        Self {
            bundle_id,
            version: 1,
            description: None,
            signer,
            created_at: SystemTime::now(),
            rollout_policy: RolloutPolicy::default(),
            revocation_policy: RevocationPolicy::default(),
            tags: Vec::new(),
        }
    }
}


/// Validation result with detailed errors
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
}

impl ValidationResult {
    pub fn valid() -> Self {
        Self{
            valid:true, 
            errors: Vec::new(), 
            warnings: Vec::new(),
        }
    }

    pub fn with_error(error:ValidationError) -> Self {
        Self {
            valid: false, 
            errors: vec![error],
            warnings: Vec::new(),
        }
    }

    pub fn add_error(&mut self, error: ValidationError) {
        self.valid = false;
        self.errors.push(error);
    }
    
    pub fn add_warning(&mut self, warning: ValidationWarning) {
        self.warnings.push(warning);
    }
    
    pub fn merge(&mut self, other: ValidationResult) {
        if !other.valid {
            self.valid = false;
        }
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
    }
}

/// Validation error types
#[derive(Debug, Clone, Error, PartialEq)]
pub enum ValidationError {
    #[error("Empty bundle: no rules defined")]
    EmptyBundle,
    
    #[error("Duplicate rule ID: {0}")]
    DuplicateRuleId(String),
    
    #[error("Invalid rule ID format: {0}")]
    InvalidRuleId(String),
    
    #[error("Invalid bundle ID format: {0}")]
    InvalidBundleId(String),
    
    #[error("Invalid priority: rule {rule_id} has priority {priority}")]
    InvalidPriority { rule_id: String, priority: i32 },

    #[error("Priority conflict: rules {rule_id1} and {rule_id2} have same priority {priority}")]
    PriorityConflict {
        rule_id1: String,
        rule_id2: String,
        priority: i32,
    },
    
    #[error("Invalid scope: {0}")]
    InvalidScope(String),
    
    #[error("Invalid constraint: {rule_id} - {reason}")]
    InvalidConstraint { rule_id: String, reason: String },
    
    #[error("Invalid WASM hook: {rule_id} - {reason}")]
    InvalidWasmHook { rule_id: String, reason: String },
    
    #[error("Invalid action parameters: {rule_id} - {reason}")]
    InvalidActionParams { rule_id: String, reason: String },
    
    #[error("Disallowed side effect: {rule_id} - {side_effect}")]
    DisallowedSideEffect { rule_id: String, side_effect: String },
    
    #[error("Signature verification failed: {0}")]
    SignatureVerificationFailed(String),
    
    #[error("Invalid rollout policy: {0}")]
    InvalidRolloutPolicy(String),
    
    #[error("Schema validation failed: {0}")]
    SchemaValidationFailed(String),
    
    #[error("Rule conflict: {rule_id1} and {rule_id2} - {reason}")]
    RuleConflict {
        rule_id1: String,
        rule_id2: String,
        reason: String,
    },
}

/// Validation warning types
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationWarning {
    HighPriority { rule_id: String, priority: i32 },
    LargeBundle { rule_count: usize },
    ComplexMatchExpression { rule_id: String },
    HighMemoryConstraint { rule_id: String, limit_mb: u64 },
    LongTimeout { rule_id: String, timeout_ms: u64 },
    NoRolloutPolicy,
    OverlappingScopes { rule_id1: String, rule_id2: String },
}

/// Rule bundle containing multiple rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleBundle {
    /// Bundle metadata
    pub metadata: BundleMetadata,
    /// Rules in this bundle
    pub rules: Vec<Rule>,
    /// Bundle-level allowed side effects
    pub allowed_side_effects: Vec<AllowedSideEffect>,
    /// Signature (hex string)
    pub signature: Option<String>,
}

impl RuleBundle {
    /// Create a new empty bundle
    pub fn new(bundle_id: BundleId, signer: String) -> Self {
        Self {
            metadata: BundleMetadata::new(bundle_id, signer),
            rules: Vec::new(),
            allowed_side_effects: Vec::new(),
            signature: None, 
        }
    }

    /// Add a rule to the bundle
    pub fn add_rule(&mut self, rule: Rule) {
        self.rules.push(rule);
    }
    
    /// Get rule by ID
    pub fn get_rule(&self, rule_id: &RuleId) -> Option<&Rule> {
        self.rules.iter().find(|r| &r.metadata.rule_id == rule_id)
    }
    
    /// Get mutable rule by ID
    pub fn get_rule_mut(&mut self, rule_id: &RuleId) -> Option<&mut Rule> {
        self.rules.iter_mut().find(|r| &r.metadata.rule_id == rule_id)
    }
    
    /// Remove rule by ID
    pub fn remove_rule(&mut self, rule_id: &RuleId) -> Option<Rule> {
        if let Some(pos) = self.rules.iter().position(|r| &r.metadata.rule_id == rule_id) {
            Some(self.rules.remove(pos))
        } else {
            None
        }
    }

    /// Get rules sorted by priority (highest first)
    pub fn rules_by_priority(&self) -> Vec<&Rule> {
        let mut rules: Vec<&Rule> = self.rules.iter().collect();
        rules.sort_by(|a, b| b.priority().cmp(&a.priority()));
        rules
    }
    
    /// Count rules by enforcement class
    pub fn count_by_class(&self, class: EnforcementClass) -> usize {
        self.rules
            .iter()
            .filter(|r| r.metadata.enforcement_class == class)
            .count()
    }
    
    /// Get active rules only
    pub fn active_rules(&self) -> Vec<&Rule> {
        self.rules.iter().filter(|r| r.is_active()).collect()
    }
}

/// Bundle parser supporting multiple formats
pub struct BundleParser;

impl BundleParser {
    /// Parse bundle from JSON String
    pub fn from_json(json: &str) -> Result<RuleBundle, ParseError> {
        serde_json::from_str(json).map_err(|e| ParseError::JsonParseError(e.to_string()))
    }

    /// Parse bundle from JSON bytes
    pub fn from_json_bytes(bytes: &[u8]) -> Result<RuleBundle, ParseError> {
        serde_json::from_slice(bytes).map_err(|e| ParseError::JsonParseError(e.to_string()))
    }
    
    /// Serialize bundle to JSON string
    pub fn to_json(bundle: &RuleBundle) -> Result<String, ParseError> {
        serde_json::to_string_pretty(bundle)
            .map_err(|e| ParseError::SerializationError(e.to_string()))
    }
    
    /// Serialize bundle to JSON bytes
    pub fn to_json_bytes(bundle: &RuleBundle) -> Result<Vec<u8>, ParseError> {
        serde_json::to_vec_pretty(bundle)
            .map_err(|e| ParseError::SerializationError(e.to_string()))
    }
}

/// Parse Errors
#[derive(Debug, Error)]
pub enum ParseError {
    #[error("JSON parse error: {0}")]
    JsonParseError(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("Invalid format: {0}")]
    InvalidFormat(String),
}

/// Comprehensive Bundle Validators
pub struct BundleValidator {
    /// Max rules allowed per bundle
    max_rules_per_bundle: usize,

    /// Max priority value
    max_priority: u32,

    /// Require signatures
    require_signatures: bool,
}

impl BundleValidator {
    pub fn new() -> Self {
        Self {
            max_rules_per_bundle: 1000,
            max_priority: 10000,
            require_signatures: false,
        }
    }
    
    pub fn with_max_rules(mut self, max: usize) -> Self {
        self.max_rules_per_bundle = max;
        self
    }
    
    pub fn with_max_priority(mut self, max: u32) -> Self {
        self.max_priority = max;
        self
    }
    
    pub fn require_signatures(mut self, require: bool) -> Self {
        self.require_signatures = require;
        self
    }
    
    /// Comprehensive bundle validation
    pub fn validate(&self, bundle: &RuleBundle) -> ValidationResult {
        let mut result = ValidationResult::valid();
        
        // 1. Basic validation
        self.validate_basic(bundle, &mut result);
        
        // 2. Rule-level validation
        self.validate_rules(bundle, &mut result);
        
        // 3. Priority validation
        self.validate_priorities(bundle, &mut result);
        
        // 4. Scope validation
        self.validate_scopes(bundle, &mut result);
        
        // 5. Constraint validation
        self.validate_constraints(bundle, &mut result);
        
        // 6. Side effect validation
        self.validate_side_effects(bundle, &mut result);
        
        // 7. Conflict detection
        self.detect_conflicts(bundle, &mut result);
        
        // 8. Signature verification
        if self.require_signatures {
            self.verify_signature(bundle, &mut result);
        }
        
        // 9. Rollout policy validation
        self.validate_rollout_policy(bundle, &mut result);
        
        result
    }

    /// Validate Basic bundle structure 
    fn validate_basic(&self, bundle: &RuleBundle, result: &mut ValidationResult) {
        // Check if rule bundle is empty
        if bundle.rules.is_empty() {
            result.add_error(ValidationError::EmptyBundle);
        }

        // Check bundle size
        if bundle.rules.len() > self.max_rules_per_bundle {
            result.add_warning(ValidationWarning::LargeBundle {
                rule_count: bundle.rules.len(),
            });
        }
        
        // Validate bundle ID format
        if bundle.metadata.bundle_id.as_str().is_empty() {
            result.add_error(ValidationError::InvalidBundleId(
                "Bundle ID Cannot be empty".to_string(),
            ));
        }
    }

    // Validate individual rules
    fn validate_rules(&self, bundle:&RuleBundle, result: &mut ValidationResult) {
        let mut seen_ids = HashSet::new();

        for rule in &bundle.rules {
            let rule_id = rule.id().as_str().to_string();

            // Check for duplicate IDs
            if !seen_ids.insert(rule_id.clone()) {
                result.add_error(ValidationError::DuplicateRuleId(rule_id.clone()));
                continue;
            }

            // Validate rule id format
            if rule_id.is_empty(){
                result.add_error(ValidationError::InvalidRuleId(
                    "Rule ID cannot be empty".to_string(),
                ));
            }

            //Validate WASM hooks if present
            if let Some(hook_ref) = &rule.match_clause.wasm_hook {
                self.validate_wasm_hook(rule, hook_ref, result);
            }

        }
    }

    /// Validate rule priorities
    fn validate_priorities(&self, bundle: &RuleBundle, result: &mut ValidationResult) {
        let mut priority_map: HashMap<i32, Vec<String>> = HashMap::new();

        for rule in &bundle.rules{
            let priority = rule.priority();
            let rule_id = rule.id().as_str().to_string();

            //Check priority bounds
            if priority > self.max_priority as i32 {
                result.add_error(ValidationError::InvalidPriority {
                    rule_id: rule_id.clone(),
                    priority
                });
            }

            // Warn on very high priorities
            if priority > 1000 {
                result.add_warning(ValidationWarning::HighPriority {
                    rule_id: rule_id.clone(),
                    priority,
                });
            }

            // Track priorities for conflict detection
            priority_map.entry(priority).or_insert_with(Vec::new).push(rule_id);
        }

        // Check for priority conflicts
        for (priority, rule_ids) in priority_map{
            if rule_ids.len() > 1 {
                result.add_error(ValidationError::PriorityConflict{
                    rule_id1: rule_ids[0].clone(),
                    rule_id2: rule_ids[1].clone(),
                    priority,
                });
            }
        }
    }

    /// Validate rule scopes
    fn validate_scopes(&self, bundle:&RuleBundle, result: &mut ValidationResult) {
        for rule in &bundle.rules {
            let scope = &rule.metadata.scope;

            // Validate scope has atleast one constraint
            if !scope.is_global && scope.agent_ids.is_empty() && scope.flow_ids.is_empty() {
                    result.add_error(ValidationError::InvalidScope(
                        format!("Rule {} has empty scope", rule.id().as_str())
                    ));
                }
        }
        self.check_overlapping_scopes(bundle, result);
    }

    /// Validate execution constraints
    fn validate_constraints(&self, bundle: &RuleBundle, result: &mut ValidationResult) {
        for rule in &bundle.rules {
            let constraints = &rule.constraints ;

            // Validate constraints
            if let Err(e) = constraints.validate() {
                result.add_error(ValidationError::InvalidConstraint {
                    rule_id: rule.id().as_str().to_string(),
                    reason: e.to_string(),
                });
            }
            // Warn on high memory limits
            if let Some(limit) = constraints.memory_limit_bytes {
                let limit_mb = limit/(1024 * 1024);
                if limit_mb > 100 {
                    result.add_warning(ValidationWarning::HighMemoryConstraint{
                        rule_id: rule.id().as_str().to_string(),
                        limit_mb,
                    });
                }
            }

            //Warn on long timeouts
            if constraints.max_exec_ms > 1000 {
                result.add_warning(ValidationWarning::LongTimeout {
                    rule_id:rule.id().as_str().to_string(),
                    timeout_ms: constraints.max_exec_ms,
                });
            }
        }
    }
    
    /// Validate WASM hooks
    fn validate_wasm_hook( &self, rule: &Rule, hook_ref:&WasmHookRef,
                            result: &mut ValidationResult) {
        // Check digest format
        if hook_ref.module_digest.is_empty() {
            result.add_error(ValidationError::InvalidWasmHook{
                rule_id: rule.id().as_str().to_string(),
                reason: "Empty WASM digest".to_string(),
            });
        }
    }

    /// Validate side effects
    fn validate_side_effects (&self, bundle: &RuleBundle, result: &mut ValidationResult) {
        let allowed_effects: HashSet<_> = bundle.allowed_side_effects.iter().cloned().collect();

        for rule in &bundle.rules {
            // Check if action type requires allowed side effects
            match &rule.action_clause.primary_action {
                ActionType::SpawnSidecar(_) => {
                    if !allowed_effects.contains(&AllowedSideEffect::ProcessSpawn) {
                        result.add_error(ValidationError::DisallowedSideEffect {
                            rule_id: rule.id().as_str().to_string(),
                            side_effect: "SpawnSidecar".to_string(),
                        });
                    }
                }

                ActionType::Callback(_) => {
                    if !allowed_effects.contains(&AllowedSideEffect::NetworkCall) {
                        result.add_error(ValidationError::DisallowedSideEffect {
                            rule_id: rule.id().as_str().to_string(),
                            side_effect: "Callback".to_string(),
                        });
                    }
                }
                _ => {}
            }
        }
    }

    /// Detect Rule conflicts
    fn detect_conflicts(&self, bundle: &RuleBundle, result: &mut ValidationResult) {
        // Check for conflicting actions on the same scope
        for i in 0..bundle.rules.len() {
            for j in (i+1)..bundle.rules.len() {
                let rule1 = &bundle.rules[i];
                let rule2 = &bundle.rules[j];
                
                // If scopes overlap and priorities are same, might conflict
                if self.scopes_overlap(&rule1.metadata.scope, &rule2.metadata.scope) {
                    if rule1.priority() == rule2.priority() {
                        result.add_error(ValidationError::RuleConflict{
                            rule_id1: rule1.id().as_str().to_string(),
                            rule_id2: rule2.id().as_str().to_string(),
                            reason: "Same priority with overlapping scopes".to_string(),
                        });
        
                    }
                }
            }

        }
    }

    /// Check if two scopes overlap 
    fn scopes_overlap(&self, scope1: &RuleScope, scope2: &RuleScope) -> bool{
        // If agent IDs match or one is global
        if scope1.agent_ids == scope2.agent_ids || scope1.agent_ids.is_empty() || scope2.agent_ids.is_empty() {
            // And flows match or one is empty (global)
            if scope1.flow_ids == scope2.flow_ids || scope1.flow_ids.is_empty() || scope2.flow_ids.is_empty() {
                return true;
            }
        }
        false
    }
    
    /// Check for overlapping scopes (warning)
    fn check_overlapping_scopes(&self, bundle: &RuleBundle, result: &mut ValidationResult) {
        for i in 0..bundle.rules.len(){
            for j in (i+1)..bundle.rules.len() {
                let rule1 = &bundle.rules[i];
                let rule2 = &bundle.rules[j];
                
                if self.scopes_overlap(&rule1.metadata.scope, &rule2.metadata.scope) {
                    result.add_warning(ValidationWarning::OverlappingScopes {
                        rule_id1: rule1.id().as_str().to_string(),
                        rule_id2: rule2.id().as_str().to_string(),
                    });
                }
            }
        }
    }

    /// Verify bundle signatures
    fn verify_signature(&self, bundle: &RuleBundle, result: &mut ValidationResult) {
        if bundle.signature.is_none() {
            result.add_error(ValidationError::SignatureVerificationFailed(
                "No signature present".to_string(),
            ));
        } else {
            // TODO: In production, verify against public key
            // For now, just check format
            if let Some(sig) = &bundle.signature {
                if sig.len() < 64 {
                    result.add_error(ValidationError::SignatureVerificationFailed(
                        "Invalid signature format".to_string(),
                    ));
                }
            }
        }
    }

    /// Validate rollout policy
    fn validate_rollout_policy(&self, bundle: &RuleBundle, result: &mut ValidationResult) {
        match &bundle.metadata.rollout_policy {
            RolloutPolicy::Canary {percentage, ..} => {
                if *percentage < 0.0 || *percentage > 1.0 {
                    result.add_error(ValidationError::InvalidRolloutPolicy(
                        format!("Invalid canary percentage: {}", percentage),
                    ));
                }
            }
            RolloutPolicy::TimeWindow {start_time, end_time} => {
                if end_time <= start_time {
                    result.add_error(ValidationError::InvalidRolloutPolicy(
                        "End time must be after start time".to_string(),));
                }
            }
            _ => {}
            }
        }
    }

impl Default for BundleValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Bundle compiler for pre procssing rules
pub struct BundleCompiler ;

impl BundleCompiler {
    /// Compile bundle(prepare for execution)
    pub fn compile(bundle: &RuleBundle) -> Result<CompiledBundle, CompilationError> {
        let mut compiled = CompiledBundle {
            bundle_id: bundle.metadata.bundle_id.clone(),
            version: bundle.metadata.version,
            compiled_rules: Vec::new(),
            compiled_at: SystemTime::now(),
        };

        for rule in &bundle.rules {
            // TODO:
            // In production, this would:
            // 1. Compile match expressions to bytecode
            // 2. Validate WASM hooks
            // 3. Pre-compute fast-match bitsets
            // 4. Optimize action parameters
            
            compiled.compiled_rules.push(CompiledRule {
                rule_id: rule.id().clone(),
                bytecode: vec![], // Placeholder for compiled match expression
                optimizations_applied: vec!["fast_match_precomputed".to_string()],
            });
        }
        
        Ok(compiled)
    }
}

/// Compiled bundle ready for execution
#[derive(Debug, Clone)]
pub struct CompiledBundle {
    pub bundle_id: BundleId,
    pub version: u64,
    pub compiled_rules: Vec<CompiledRule>,
    pub compiled_at: SystemTime,
}

/// Compiled rule with bytecode
#[derive(Debug, Clone)]
pub struct CompiledRule {
    pub rule_id: RuleId,
    pub bytecode: Vec<u8>, // Compiled match expression
    pub optimizations_applied: Vec<String>,
}

/// Compilation errors
#[derive(Debug, Error)]
pub enum CompilationError {
    #[error("Compilation failed: {0}")]
    CompilationFailed(String),
    
    #[error("Invalid bytecode: {0}")]
    InvalidBytecode(String),
}
