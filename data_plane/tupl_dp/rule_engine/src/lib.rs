pub mod rule_metadata;
pub mod match_clause;
pub mod action_clause;
pub mod execution_constraints;
pub mod audit_record;
pub mod rule_bundle;
pub mod rule_table;
pub mod hot_reload;
pub mod bundle_crud;

pub use rule_metadata::{
    AgentId, EnforcementClass, EnforcementMode, FlowId, RuleId, RuleMetadata,
    RuleMetadataBuilder, RuleScope, RuleState,
};

pub use match_clause::{
    ComparisonOp, EventContext, FastMatch, FastMatchBuilder, FieldComparison, FieldValue,
    HeaderFlags, JsonPathQuery, MatchClause, MatchExpression, MatchResult, MatchTier,
    PayloadData, RegexMatch, WasmHookRef,
};

// Re-export action_clause types
pub use action_clause::{
    ActionClause, ActionClauseBuilder, ActionContext, ActionResult, ActionType,
    AllowedSideEffect, AllowParams, AttachMetadataParams, CallbackParams, DenyParams, LogLevel,
    LogParams, RateLimitParams, RateLimitScope, RedactParams, RedactionStrategy,
    RewriteOperation, RewriteParams, RouteToParams, SandboxExecuteParams, SidecarSpec,
    SpawnSidecarParams, TransformFunction,
};

pub use execution_constraints::{
    ConstraintEnforcer, ConstraintError, ConstraintViolationType, ExecutionBudget,
    ExecutionConstraints, ExecutionStats, RetryPolicy, RuleType,
};

pub use audit_record::{
    AuditContext, AuditContextBuilder, AuditLogLevel, AuditRecord, AuditRecordBuilder,
    AuditTrail, CompactDecisionRecord, DecisionOutcome, EvaluationTimestamps,
    ExecutionStatistics, PayloadRef, SequenceNumber,
};

pub use rule_bundle::{
    BundleCompiler,         // Bundle compilation
    BundleId,               // Bundle identifier type
    BundleMetadata,         // Bundle metadata
    BundleParser,           // JSON parsing/serialization
    BundleValidator,        // Comprehensive validator
    CompiledBundle,         // Compiled bundle output
    CompiledRule,           // Compiled rule
    CompilationError,       // Compilation errors
    ParseError,             // Parse errors
    RevocationPolicy,       // Deactivation policy
    RolloutPolicy,          // Deployment policy
    Rule,                   // Complete rule definition
    RuleBundle,             // Rule collection
    ValidationError,        // Validation error types
    ValidationResult,       // Validation outcome
    ValidationWarning,      // Validation warning types
};

pub use rule_table::{
    RuleTable,              // Main in-memory rule storage
    RuleQuery,              // Query builder for rule lookups
    RuleEntry,              // Rule entry with metadata
    RuleStats,              // Per-rule execution statistics
    TableStats,             // Table-level statistics
};

pub use hot_reload::{
    DeploymentManager,      // Main hot reload manager
    DeploymentState,        // Deployment state enum
    DeploymentStrategy,     // Deployment strategy enum
    HealthMetrics,          // Health monitoring metrics
    HealthThresholds,       // Health check thresholds
    VersionId,              // Deployment version identifier
    compute_request_hash,   // Helper for request routing
};

pub use bundle_crud::{
    BundleCRUD,             // Main CRUD manager
    ConflictInfo,           // Conflict detection result
    ConflictType,           // Type of conflict
    OperationHandle,        // Operation handle returned from CRUD
    RevocationPolicy as CRUDRevocationPolicy,       // Revocation policy enum (from CRUD)
    RuleState as CRUDRuleState,              // Rule lifecycle state (from CRUD)
    RuleStats as CRUDRuleStats,              // Rule statistics (from CRUD)
};

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
