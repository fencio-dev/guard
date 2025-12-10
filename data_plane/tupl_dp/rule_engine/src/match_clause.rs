// This module implements the matching component of rules, which determines 
// where a rule should be applied to a given event/request. It follows a three
// tier eval method

// 1. FastMatch: O(1) cheap predicates using bitsets and hash lookups
// 2. MatchExpression: Structured syntactic checks (regex, JSONPath, field comparisons)
// 3. WasmHook: Optional semantic validation via sandboxed WASM execution

// The evaluation proceeds from cheapest to the most expensive. 

use crate::{AgentId, FlowId};
use serde::{Deserialize, Serialize};
use std::collections::{HashSet, HashMap};
use std::time::Duration;

// ============================================================================
// FAST MATCH - O(1) CHEAP PREDICATES
// ============================================================================

/// Fast matching predicates using bitsets and hash lookups.
///
/// This is the first and fastest layer of rule evaluation. It performs
/// cheap O(1) checks using indexed data structures (HashSets, flags).
/// These checks don't require loading the payload and work purely on
/// event metadata and headers.
///
/// # Design Principles
/// - **O(1) operations only**: Hash lookups, bitset checks
/// - **No payload access**: Works on headers/metadata only
/// - **Early termination**: Fail fast if predicates don't match
/// - **Index-friendly**: Can be pre-indexed for ultra-fast lookups
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FastMatch {
    /// Set of allowed source agents. If empty, any source is allowed.
    /// If non-empty, source_agent MUST be in this set.
    pub source_agents: HashSet<AgentId>,

    /// Set of allowed destination agents. If empty, any destination is allowed.
    pub dest_agents: HashSet<AgentId>,

    /// Set of allowed flow IDs. If empty, any flow is allowed.
    pub flow_ids: HashSet<FlowId>,

    /// Set of allowed payload types (MIME types). If empty, any type is allowed.
    /// Examples: "application/json", "text/plain", "application/protobuf"
    pub payload_types: HashSet<String>,

    /// Header flags that must be present. Bitset represented as u64.
    /// Each bit represents a specific flag:
    /// - Bit 0: ENCRYPTED
    /// - Bit 1: AUTHENTICATED
    /// - Bit 2: RATE_LIMITED
    /// - Bit 3: HIGH_PRIORITY
    /// - Bit 4: CONTAINS_PII
    /// - Bits 5-63: Reserved for future use
    pub required_flags: HeaderFlags,

    /// Header flags that must NOT be present.
    pub forbidden_flags: HeaderFlags,
}

impl FastMatch {
    ///Creates a new empty FastMatch instance with no restrictions.
    /// Use the builder pattern for more control. 
    pub fn new() -> Self {
        FastMatch {
            source_agents: HashSet::new(),
            dest_agents: HashSet::new(),
            flow_ids: HashSet::new(),
            payload_types: HashSet::new(),
            required_flags: HeaderFlags::empty(),
            forbidden_flags: HeaderFlags::empty(),
        }
    }

    /// Creates a FastMatch instance that matches nothing. 
    pub fn match_none() -> Self {
        let mut fast_match = Self::new();
        //Set contradictory flags to ensure no match
        fast_match.required_flags = HeaderFlags::all();
        fast_match.forbidden_flags = HeaderFlags::all();
        fast_match
    }

    /// Evaluates this FastMatch against event context.
    ///
    /// Returns `true` if all predicates match, `false` otherwise.
    pub fn evaluate(&self, ctx: &EventContext) -> bool {
        // Check source agent
        if !self.source_agents.is_empty() && !self.source_agents.contains(
            &ctx.source_agent) {
        return false;

        }
        //Check destination agent
        if !self.dest_agents.is_empty(){
            if let Some(dest) = &ctx.dest_agent {
                if !self.dest_agents.contains(dest) {
                    return false;
                }
            }else {
                return false;
            }
        }

        // Check flow id
        if !self.flow_ids.is_empty() {
            if let Some(flow) = &ctx.flow_id {
                if !self.flow_ids.contains(flow) {
                    return false;
                }
            } else {
                return false;
            }
        }
        // Check payload type
        if !self.payload_types.is_empty() && !self.payload_types.contains(&ctx.payload_type) {
            return false;
        }

        // Check required flags are present
        if !ctx.header_flags.contains(self.required_flags) {
            return false;
        }

        // Check forbidden flags are absent
        if ctx.header_flags.intersects(self.forbidden_flags) {
            return false;
        }

        true
    }
    /// Returns true if this FastMatch will match everything
    pub fn matches_all(&self) -> bool {
        self.source_agents.is_empty() &&
        self.dest_agents.is_empty() &&
        self.flow_ids.is_empty() &&
        self.payload_types.is_empty() &&
        self.required_flags.is_empty() &&
        self.forbidden_flags.is_empty()
    }
}

impl Default for FastMatch {
    fn default() -> Self {
        Self::new()
    }
}


// ============================================================================
// HEADER FLAGS - BITSET FOR FAST CHECKS
// ============================================================================

/// Header flags represented as a bitset for fast operations.
///
/// This uses a u64 to represent up to 64 boolean flags. Bitwise operations
/// are extremely fast and cache-friendly.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct HeaderFlags(u64);

impl HeaderFlags {
    // Flag bit positions
    pub const ENCRYPTED: u64 = 1 << 0;
    pub const AUTHENTICATED: u64 = 1 << 1;
    pub const RATE_LIMITED: u64 = 1 << 2;
    pub const HIGH_PRIORITY: u64 = 1 << 3;
    pub const CONTAINS_PII: u64 = 1 << 4;
    pub const REQUIRES_AUDIT: u64 = 1 << 5;
    pub const SYNTHETIC: u64 = 1 << 6;
    pub const CACHED: u64 = 1 << 7;

    /// Returns the raw bits value.
    pub const fn bits(&self) -> u64 {
        self.0
    }

    /// Creates an empty HeaderFlags (no flags set).
    pub fn empty() -> Self {
        HeaderFlags(0)
    }

    /// Creates a HeaderFlags with all flags set.
    pub fn all() -> Self {
        HeaderFlags(u64::MAX)
    }

    /// Creates flags from a raw u64 value.
    pub fn from_bits(bits: u64) -> Self {
        HeaderFlags(bits)
    }

    /// Checks if this contains all flags in `other`.
    pub const fn contains(&self, other: HeaderFlags) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Checks if this has any flags in common with `other`.
    pub const fn intersects(&self, other: HeaderFlags) -> bool {
        (self.0 & other.0) != 0
    }

    /// Returns true if no flags are set.
    pub const fn is_empty(&self) -> bool {
        self.0 == 0
    }

    /// Sets a flag.
    pub fn set(&mut self, flag: u64) {
        self.0 |= flag;
    }

    /// Clears a flag.
    pub fn clear(&mut self, flag: u64) {
        self.0 &= !flag;
    }

    /// Checks if a specific flag is set.
    pub const fn has(&self, flag: u64) -> bool {
        (self.0 & flag) != 0
    }
}

// ============================================================================
// MATCH EXPRESSION - STRUCTURED SYNTACTIC CHECKS
// ============================================================================

/// Structured expressions for syntactic validation.
///
/// This is the second layer of evaluation, applied after FastMatch succeeds.
/// It performs more complex checks like regex matching, field comparisons,
/// and JSONPath queries.
///
/// # Design Principles
/// - **Compiled on activation**: Expressions are pre-compiled (regex, JSONPath)
/// - **Composable**: Can combine multiple expressions with AND/OR/NOT
/// - **Lazy evaluation**: Short-circuits on first failure
/// - **Minimal payload access**: Only loads payload if necessary

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MatchExpression {
    /// Always matches
    Always,

    /// Never matches
    Never,

    /// Compare a field value
    Field(FieldComparison),

    /// Match against a regex pattern
    Regex(RegexMatch),

    /// Query using JSONPath.
    JsonPath(JsonPathQuery),

    /// Logical AND: all sub-expressions must match.
    And(Vec<MatchExpression>),

    /// Logical OR: at least one sub-expression must match.
    Or(Vec<MatchExpression>),

    /// Logical NOT: inverts the result.
    Not(Box<MatchExpression>),
}

impl MatchExpression {
    /// Evaluates the expression against the event context and payload. 
    /// Returns true if the expression matches else false

    pub fn evaluate(&self, ctx: &EventContext, payload: Option<&PayloadData>) -> bool {
        match self {
            MatchExpression::Always => true,
            MatchExpression::Never => false,
            MatchExpression::Field(field_comp) => field_comp.evaluate(ctx, payload),
            MatchExpression::Regex(regex) => regex.evaluate(ctx, payload),
            MatchExpression::JsonPath(jsonpath) => jsonpath.evaluate(ctx, payload),
            MatchExpression::And(exprs) => {
                // ALl must match 
                exprs.iter().all(|expr| expr.evaluate(ctx, payload))
            }
            MatchExpression::Or(exprs) => {
                // At least one must match
                exprs.iter().any(|expr| expr.evaluate(ctx, payload))
            }
            MatchExpression::Not(expr) => {
                // Invert result
                !expr.evaluate(ctx, payload)
            }
        }
    }

    /// Returns true if the expression requires the payload access. 
    /// This helps optimize evaluation by avoiding unnecessary payload loading.

    pub fn required_payload(&self) -> bool {
        match self {
            MatchExpression::Always | MatchExpression::Never => false,
            MatchExpression::Field(field_comp) => field_comp.requires_payload(),
            MatchExpression::Regex(regex) => regex.requires_payload(),
            MatchExpression::JsonPath(_) => true,// JsonPath always needs payload
            MatchExpression::And(exprs) | MatchExpression::Or(exprs) => {
                exprs.iter().any(|expr| expr.required_payload())
            }
            MatchExpression::Not(expr) => expr.required_payload(),
        }
    }
}

/// Field comparison operations
/// Compares a field value against a reference value using various operators.

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldComparison {
    /// path to the field (dot-separated for nested fields)
    pub field_path: String,

    /// Comparison operator
    pub operator: ComparisonOp,

    /// Reference value to compare against
    pub value: FieldValue,
}

impl FieldComparison {
    pub fn evaluate(&self, ctx: &EventContext, payload: Option<&PayloadData>) -> bool {
        // Try to get field from context first
        if let Some(field_value) = ctx.get_header(&self.field_path) {
            return self.compare(&field_value);
        }

        // If not in context, try payload if available
        if let Some(payload) = payload {
            if let Some(field_value) = payload.get_field(&self.field_path) {
                return self.compare(&field_value);
            }
        }
        false // Field not found
    }

    fn compare(&self, field_value: &FieldValue) -> bool {
        match self.operator {
            ComparisonOp::Equal => field_value == &self.value,
            ComparisonOp::NotEqual => field_value != &self.value,
            ComparisonOp::GreaterThan => field_value > &self.value,
            ComparisonOp::GreaterThanOrEqual => field_value >= &self.value,
            ComparisonOp::LessThan => field_value < &self.value,
            ComparisonOp::LessThanOrEqual => field_value <= &self.value,
            ComparisonOp::Contains => match (field_value, &self.value) {
                (FieldValue::String(s), FieldValue::String(pattern)) => s.contains(pattern),
                _ => false,
            },
            ComparisonOp::StartsWith => match (field_value, &self.value) {
                (FieldValue::String(s), FieldValue::String(prefix)) => s.starts_with(prefix),
                _ => false,
            },
            ComparisonOp::EndsWith => match (field_value, &self.value) {
                (FieldValue::String(s), FieldValue::String(suffix)) => s.ends_with(suffix),
                _ => false,
            },
            ComparisonOp::In => match &self.value {
                FieldValue::Array(arr) => arr.contains(field_value),
                _ => false,
            },
        }
    }

    fn requires_payload(&self) -> bool {
        // If field path is a known header field, we dont need payload
        // For simplicity, assume we might need paylaod. 
        true
    }
}

/// Comparison operators for field comparisons
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]

pub enum ComparisonOp {
    Equal,
    NotEqual,
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
    Contains,    // String contains substring
    StartsWith,  // String starts with prefix
    EndsWith,    // String ends with suffix
    In,          // Value is in array
}

/// Field value types for comparisons.
#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum FieldValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Array(Vec<FieldValue>),
    Null,
}

impl From<String> for FieldValue {
    fn from(s: String) -> Self {
        FieldValue::String(s)
    }
}

impl From<&str> for FieldValue {
    fn from(s: &str) -> Self {
        FieldValue::String(s.to_string())
    }
}

impl From<i64> for FieldValue {
    fn from(i: i64) -> Self {
        FieldValue::Integer(i)
    }
}

impl From<f64> for FieldValue {
    fn from(f: f64) -> Self {
        FieldValue::Float(f)
    }
}

impl From<bool> for FieldValue {
    fn from(b: bool) -> Self {
        FieldValue::Boolean(b)
    }
}

/// Regex patterns matching
/// The actual regex compilation happens during rule activation. 
/// This struct stores the pattern and the metadata

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegexMatch {
    /// Field path to apply the regex on
    pub field_path: String,
    /// Regex pattern as string
    pub pattern: String,
    ///Whether the pattern should match the entire field or any substring
    pub full_match: bool,
}

impl RegexMatch {
    pub fn evaluate(&self, ctx: &EventContext, payload: Option<&PayloadData>) -> bool {
        //Get Field value
        let field_value = if let Some(val) = ctx.get_header(&self.field_path) {
            val
        } else if let Some(payload) = payload {
            if let Some(val) = payload.get_field(&self.field_path) {
                val
            } else {
                return false; // Field not found
            }
        } else {
            return false; // Field not found
        };

        // Convert field to string for regex matching
        let text = match field_value {
            FieldValue::String(s) => s.clone(),
            FieldValue::Integer(i) => i.to_string(),
            FieldValue::Float(f) => f.to_string(),
            FieldValue::Boolean(b) => b.to_string(),
            _ => return false, // Cannot apply regex on non-string types
        };

        // In production, we need to use a pre-compiled regex
        // For now, we'll do a simple pattern check
        // TODO: Integrate with regex crate and compile patterns on rule activation
        if self.full_match {
            text == self.pattern // Simplified - should use regex
        } else {
            text.contains(&self.pattern) // Simplified - should use regex
        }
    }

    fn requires_payload(&self) -> bool {
        true // Conservative - assume we need payload
    }
}

/// JSONPath query matching
/// Queries nested JSON structures using JSON Path syntax. 
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonPathQuery {
    /// JSONPath expression
    pub path: String,

    /// Expected value at the JSONPath
    pub expected_value: Option<FieldValue>,

    /// If true, just checks if the path exists (ignores expected_value)
    pub exists_only: bool,
}

impl JsonPathQuery {
    pub fn evaluate(&self, _ctx: &EventContext, payload: Option<&PayloadData>) -> bool {
        // JSONPath always requires payload
        let payload = match payload {
            Some(p) => p,
            None => return false,
        };
        // In production, you'd use a JSONPath library
        // For now, simplified implementation
        // TODO: Integrate with jsonpath crate

        if self.exists_only {
            // Just check if path exists
            payload.has_path(&self.path)
        } else if let Some(expected) = &self.expected_value {
            // Check if path value matches expected
            if let Some(actual) = payload.query_path(&self.path) {
                actual == *expected
            } else {
                false
            }
        } else {
            // No expected value and not an existence check - match if path exists
            payload.has_path(&self.path)
        }
    }
}


// ============================================================================
// WASM HOOK - SEMANTIC VALIDATION
// ============================================================================

/// Reference to a WASM module for semantic validation.
///
/// This is the third and most expensive layer of evaluation. WASM hooks
/// are only invoked if FastMatch and MatchExpression both succeed.
///
/// # Design Principles
/// - **Sandboxed execution**: Runs in isolated WASM runtime
/// - **Time-limited**: Must complete within max_exec_time
/// - **Memory-limited**: Cannot exceed memory_limit
/// - **CPU-limited**: Bounded CPU usage via cpu_shares
/// - **Fail-closed**: If hook fails/times out, treat as no match (for HARD rules)
/// TODO: Need to check if we can implement an SLM here to perform specific semantic 
/// validations as per user requirements.

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WasmHookRef {
    /// Unique identifier for this hook.
    pub hook_id: String,

    /// Digest of the WASM module (for integrity verification).
    /// Format: "sha256:..." or "sha512:..."
    pub module_digest: String,

    /// Maximum execution time before timeout.
    #[serde(with = "duration_serde")]
    pub max_exec_time: Duration,

    /// Maximum memory the WASM instance can use (in bytes).
    pub memory_limit_bytes: usize,

    /// CPU shares allocation (soft limit for scheduling).
    pub cpu_shares: u32,
}

impl WasmHookRef {
    /// Creates a new WASM hook ref with default resources. 
    /// Default limits:
    /// - Execution time: 50ms
    /// - Memory: 10 MB
    /// - CPU shares: 100
    pub fn new(hook_id: String, module_digest: String) -> Self {
        WasmHookRef {
            hook_id,
            module_digest,
            max_exec_time: Duration::from_millis(50),
            memory_limit_bytes: 10 * 1024 * 1024,
            cpu_shares: 100,
        }
    }
    /// Evaluates this WASM hook against event context and payload.
    ///
    /// This is a placeholder that returns true. In production, this would:
    /// 1. Load the WASM module from cache or storage
    /// 2. Create an isolated runtime instance
    /// 3. Call the hook's `evaluate` function with context and payload
    /// 4. Enforce time/memory/CPU limits
    /// 5. Return the result (or false on timeout/error)

    pub fn evaluate(&self, _ctx: &EventContext, _payload: Option<&PayloadData>) -> bool {
        // TODO: Implement actual WASM runtime integration
        // This would involve:
        // 1. Loading the WASM module by module_digest
        // 2. Creating a sandboxed instance with resource limits
        // 3. Serializing ctx and payload for WASM
        // 4. Invoking the evaluate function
        // 5. Handling timeouts and errors
        // 6. Deserializing the result

        // For now, return true as placeholder
        true
    }
}
// Custom serde module for Duration
mod duration_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.as_millis().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis = u64::deserialize(deserializer)?;
        Ok(Duration::from_millis(millis))
    }
}


// ============================================================================
// MATCH CLAUSE - COMPLETE MATCHING LOGIC
// ============================================================================

/// Complete matching logic for a rule.
///
/// MatchClause combines all three evaluation tiers:
/// 1. FastMatch (cheap O(1) predicates)
/// 2. MatchExpression (syntactic checks)
/// 3. WasmHook (semantic validation)
///
/// Evaluation proceeds in order, with early termination if any tier fails.
///
/// # Evaluation Flow
/// ```text
/// Event → FastMatch? → MatchExpr? → WasmHook? → MATCH
///            ↓             ↓           ↓
///          FAIL          FAIL        FAIL
///            ↓             ↓           ↓
///         NO MATCH      NO MATCH    NO MATCH
/// ```


#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatchClause {
    /// Fast match predicates
    pub fast_match: FastMatch,

    /// Structured match expression
    pub match_expression: MatchExpression,

    /// Optional WASM hook for semantic validation
    pub wasm_hook: Option<WasmHookRef>,
}

impl MatchClause {
    /// Creates a new empty match clause that matches everything
    pub fn new() -> Self {
        MatchClause {
            fast_match: FastMatch::new(),
            match_expression: MatchExpression::Always,
            wasm_hook: None,
        }
    }

    /// Creates a MatchClause that only uses FastMatch.
    pub fn fast_only(fast_match: FastMatch) -> Self {
        MatchClause {
            fast_match,
            match_expression: MatchExpression::Always,
            wasm_hook: None,
        }
    }

    /// Creates a MatchClause with FastMatch and MatchExpression.
    pub fn with_expression(fast_match: FastMatch, expr: MatchExpression) -> Self {
        MatchClause {
            fast_match,
            match_expression: expr,
            wasm_hook: None,
        }
    }

    /// Creates a complete MatchClause with all three tiers.
    pub fn complete(
        fast_match: FastMatch,
        expr: MatchExpression,
        hook: WasmHookRef,
    ) -> Self {
        MatchClause {
            fast_match,
            match_expression: expr,
            wasm_hook: Some(hook),
        }
    }

    /// Evaluates this MatchClause against an event.
    ///
    /// Returns `MatchResult` indicating whether the rule matched and
    /// which tier made the decision.

    pub fn evaluate(&self, ctx: &EventContext, payload: Option<&PayloadData>) -> MatchResult {
        // Tier 1: FastMatch
        if !self.fast_match.evaluate(ctx) {
            return MatchResult::no_match(MatchTier::FastMatch);
        }
        // Tier 2: MatchExpression
        if !self.match_expression.evaluate(ctx, payload) {
            return MatchResult::no_match(MatchTier::MatchExpression);
        }
        // Tier 3: WasmHook
        if let Some(hook) = &self.wasm_hook {
            if !hook.evaluate(ctx, payload) {
                return MatchResult::no_match(MatchTier::WasmHook);
            }
        }
        MatchResult::matched()
    }

    /// Returns true if this clause requires paylaod access
    pub fn requires_payload(&self) -> bool {
        if self.match_expression.required_payload() {
            return true;
        }
        if self.wasm_hook.is_some() {
            return true;
        }
        false
    }

    /// Returns the most expensive tier this clause uses
    pub fn max_tier(&self) -> MatchTier {
        if self.wasm_hook.is_some() {
            MatchTier::WasmHook
        } else if !matches!(self.match_expression, MatchExpression::Always) {
            MatchTier::MatchExpression
        } else if !self.fast_match.matches_all() {
            MatchTier::FastMatch
        } else {
            MatchTier::None
        }
    }

    /// Alias for max_tier for backward compatibility
    pub fn max_evaluation_tier(&self) -> MatchTier {
        self.max_tier()
    }
}

impl Default for MatchClause {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// MATCH RESULT
// ============================================================================

/// Result of evaluating a MatchClause.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchResult {
    /// Whether the rule matched
    pub is_match: bool,

    /// Tier that made the decision
    pub tier: MatchTier,
}

impl MatchResult {
    /// Creates a MatchResult indicating a match.
    pub fn matched() -> Self {
        MatchResult {
            is_match: true,
            tier: MatchTier::None,
        }
    }

    /// Creates a MatchResult indicating no match at the given tier.
    pub fn no_match(tier: MatchTier) -> Self {
        MatchResult {
            is_match: false,
            tier,
        }
    }
}

/// Evaluation tiers for match clauses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MatchTier {
    None,            // No evaluation tiers present
    FastMatch,       // Failed at fast match
    MatchExpression, // Failed at match expression
    WasmHook,        // Failed at WASM hook
    Complete,        // Passed all tiers
}



// ============================================================================
// EVENT CONTEXT - INPUT TO MATCH EVALUATION
// ============================================================================

/// Event context for match evaluation.
///
/// This represents the metadata and headers of an incoming event/request
/// that rules are evaluated against.

#[derive(Debug, Clone, PartialEq)]
pub struct EventContext {
    /// Source agent ID.
    pub source_agent: AgentId,

    /// Destination agent ID (if applicable).
    pub dest_agent: Option<AgentId>,

    /// Flow ID (if part of a flow).
    pub flow_id: Option<FlowId>,

    /// Payload MIME type.
    pub payload_type: String,

    /// Header flags.
    pub header_flags: HeaderFlags,

    /// Additional headers (key-value pairs).
    pub headers: HashMap<String, FieldValue>,
}

impl EventContext {
    /// Creates a new event Context
    pub fn new(
        source_agent: AgentId,
        dest_agent: Option<AgentId>,
        flow_id: Option<FlowId>,
        payload_type: String,
        header_flags: HeaderFlags,
        headers: HashMap<String, FieldValue>,
    ) -> Self {
        EventContext {
            source_agent,
            dest_agent,
            flow_id,
            payload_type,
            header_flags,
            headers,
        }
    }
    pub fn get_header(&self, key: &str) -> Option<&FieldValue> {
        self.headers.get(key)
    }

    /// Sets a header value. 
    pub fn set_header(&mut self, key: String, value: FieldValue) {
        self.headers.insert(key, value);
    }
}

// ============================================================================
// PAYLOAD DATA - PAYLOAD REPRESENTATION
// ============================================================================

/// Payload data for match evaluation.
///
/// This represents the actual payload content that may be queried
/// during match expression evaluation.
#[derive(Debug, Clone, PartialEq)]

pub struct PayloadData {
    /// Raw payload bytes
    pub raw_data: Vec<u8>,

    /// Parsed JSON representation (if applicable)
    pub fields: HashMap<String, FieldValue>,
}

impl PayloadData {
    /// Creates a new PayloadData instance from raw bytes and parsed fields.
    pub fn new(raw_data: Vec<u8>, fields: HashMap<String, FieldValue>) -> Self {
        PayloadData { raw_data, fields }
    }

    /// Creates payload from raw JSON bytes.
    pub fn from_bytes(json_bytes: Vec<u8>) -> Self {
        PayloadData {
            raw_data: json_bytes,
            fields: HashMap::new(), // TODO: Parse JSON into fields
        }
    }
    /// Gets a field value by path.
    pub fn get_field(&self, path: &str) -> Option<&FieldValue> {
        self.fields.get(path)
    }

    /// Checks if a JSONPath exists.
    pub fn has_path(&self, _path: &str) -> bool {
        // TODO: Implement actual JSONPath checking
        false
    }

    /// Queries a JSONPath and returns the result.
    pub fn query_path(&self, _path: &str) -> Option<FieldValue> {
        // TODO: Implement actual JSONPath query
        None
    }
}

impl Default for PayloadData {
    fn default() -> Self {
        Self::new(Vec::new(), HashMap::new())
    }
}

// ============================================================================
// BUILDER PATTERN FOR FAST MATCH
// ============================================================================

/// Builder for FastMatch.
#[derive(Debug, Default)]
pub struct FastMatchBuilder {
    source_agents: HashSet<AgentId>,
    dest_agents: HashSet<AgentId>,
    flow_ids: HashSet<FlowId>,
    payload_types: HashSet<String>,
    required_flags: HeaderFlags,
    forbidden_flags: HeaderFlags,
}

impl FastMatchBuilder {
    /// Creates a new builder.
    pub fn new() -> Self {
        FastMatchBuilder::default()
    }

    /// Adds a source agent constraint.
    pub fn add_source_agent(mut self, agent: AgentId) -> Self {
        self.source_agents.insert(agent);
        self
    }

    /// Adds multiple source agent constraints.
    pub fn source_agents(mut self, agents: impl IntoIterator<Item = AgentId>) -> Self {
        self.source_agents.extend(agents);
        self
    }

    /// Adds a destination agent constraint.
    pub fn add_dest_agent(mut self, agent: AgentId) -> Self {
        self.dest_agents.insert(agent);
        self
    }

    /// Adds a flow ID constraint.
    pub fn add_flow_id(mut self, flow: FlowId) -> Self {
        self.flow_ids.insert(flow);
        self
    }

    /// Adds a payload type constraint.
    pub fn add_payload_type(mut self, mime_type: impl Into<String>) -> Self {
        self.payload_types.insert(mime_type.into());
        self
    }

    /// Sets required flags.
    pub fn require_flags(mut self, flags: HeaderFlags) -> Self {
        self.required_flags = flags;
        self
    }

    /// Sets forbidden flags.
    pub fn forbid_flags(mut self, flags: HeaderFlags) -> Self {
        self.forbidden_flags = flags;
        self
    }

    /// Builds the FastMatch.
    pub fn build(self) -> FastMatch {
        FastMatch {
            source_agents: self.source_agents,
            dest_agents: self.dest_agents,
            flow_ids: self.flow_ids,
            payload_types: self.payload_types,
            required_flags: self.required_flags,
            forbidden_flags: self.forbidden_flags,
        }
    }
}

