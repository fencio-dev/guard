// examples/rule_bundle_usage.rs
//
// This example demonstrates the complete usage of the RuleBundle module.
//
// Run with: cargo run --example rule_bundle_usage

use rule_engine::action_clause::*;
use rule_engine::execution_constraints::ExecutionConstraints;
use rule_engine::match_clause::*;
use rule_engine::rule_bundle::*;
use rule_engine::{AgentId, EnforcementClass, EnforcementMode, RuleMetadata, RuleScope, RuleState};
use std::time::Duration;

fn main() {
    println!("=== Rule Bundle - Usage Examples ===\n");

    example_1_basic_bundle();
    example_2_validation();
    example_3_bundle_queries();
}

/// Example 1: Creating a Basic Bundle with Rules
fn example_1_basic_bundle() {
    println!("Example 1: Creating a Basic Bundle");
    println!("=====================================");

    // Create a new bundle
    let mut bundle = RuleBundle::new(
        BundleId::new("web_security_v1".to_string()),
        "security-team".to_string(),
    );

    // Modify metadata directly
    bundle.metadata.description = Some("Web application security rules".to_string());

    // Add a deny rule
    let deny_rule = Rule {
        metadata: RuleMetadata::builder()
            .signer("security-team".to_string())
            .scope(RuleScope::global())
            .enforcement_mode(EnforcementMode::Hard)
            .enforcement_class(EnforcementClass::BlockDeny)
            .priority(900)
            .state(RuleState::Active)
            .build(),
        match_clause: MatchClause::fast_only(FastMatch::new()),
        action_clause: ActionClause::new(ActionType::Deny(DenyParams {
            reason: "Suspicious request pattern detected".to_string(),
            error_code: 403,
            http_status: Some(403),
        })),
        constraints: ExecutionConstraints::default(),
        description: Some("Block suspicious requests".to_string()),
        tags: vec!["security".to_string(), "api".to_string()],
    };

    bundle.add_rule(deny_rule);

    // Add a rate limit rule
    let rate_limit_rule = Rule {
        metadata: RuleMetadata::builder()
            .signer("security-team".to_string())
            .scope(RuleScope::global())
            .enforcement_mode(EnforcementMode::Hard)
            .enforcement_class(EnforcementClass::Control)
            .priority(800)
            .state(RuleState::Active)
            .build(),
        match_clause: MatchClause::fast_only(FastMatch::new()),
        action_clause: ActionClause::new(ActionType::RateLimit(RateLimitParams {
            max_requests: 100,
            window: Duration::from_secs(60),
            scope: RateLimitScope::Global,
            action_on_exceed: Box::new(ActionType::Deny(DenyParams {
                reason: "Rate limit exceeded".to_string(),
                error_code: 429,
                http_status: Some(429),
            })),
        })),
        constraints: ExecutionConstraints::default(),
        description: Some("Rate limit API requests".to_string()),
        tags: vec!["rate-limit".to_string(), "api".to_string()],
    };

    bundle.add_rule(rate_limit_rule);

    println!("Bundle created: {}", bundle.metadata.bundle_id.0);
    println!("Total rules: {}", bundle.rules.len());
    println!("Signer: {}", bundle.metadata.signer);
    println!("Description: {}", bundle.metadata.description.as_ref().unwrap_or(&"None".to_string()));
    println!();
}

/// Example 2: Bundle Validation
fn example_2_validation() {
    println!("Example 2: Bundle Validation");
    println!("================================");

    let mut bundle = RuleBundle::new(
        BundleId::new("test_bundle".to_string()),
        "test-team".to_string(),
    );

    // Add some rules
    let rule1 = Rule {
        metadata: RuleMetadata::builder()
            .signer("test-team".to_string())
            .scope(RuleScope::global())
            .enforcement_mode(EnforcementMode::Hard)
            .priority(100)
            .state(RuleState::Active)
            .build(),
        match_clause: MatchClause::fast_only(FastMatch::new()),
        action_clause: ActionClause::new(ActionType::Allow(AllowParams {
            log_decision: true,
            reason: Some("Test allow".to_string()),
        })),
        constraints: ExecutionConstraints::default(),
        description: Some("Test rule 1".to_string()),
        tags: vec![],
    };

    bundle.add_rule(rule1);

    // Validate the bundle
    let validator = BundleValidator::default();
    let result = validator.validate(&bundle);

    println!("Validation result:");
    println!("  Valid: {}", result.valid);
    println!("  Errors: {}", result.errors.len());
    println!("  Warnings: {}", result.warnings.len());

    for error in &result.errors {
        println!("  ❌ Error: {}", error);
    }

    for warning in &result.warnings {
        println!("  ⚠️  Warning: {:?}", warning);
    }
    println!();
}

/// Example 3: Querying Bundle Contents
fn example_3_bundle_queries() {
    println!("Example 3: Querying Bundle Contents");
    println!("======================================");

    let mut bundle = RuleBundle::new(
        BundleId::new("query_test".to_string()),
        "ops-team".to_string(),
    );

    // Add multiple rules with different priorities
    for i in 0..5 {
        let rule = Rule {
            metadata: RuleMetadata::builder()
                .signer("ops-team".to_string())
                .scope(RuleScope::global())
                .enforcement_mode(EnforcementMode::Hard)
                .enforcement_class(EnforcementClass::BlockDeny)
                .priority(100 * (i + 1))
                .state(if i % 2 == 0 { RuleState::Active } else { RuleState::Paused })
                .build(),
            match_clause: MatchClause::fast_only(FastMatch::new()),
            action_clause: ActionClause::new(ActionType::Allow(AllowParams {
                log_decision: false,
                reason: None,
            })),
            constraints: ExecutionConstraints::default(),
            description: Some(format!("Rule {}", i)),
            tags: vec![],
        };
        bundle.add_rule(rule);
    }

    println!("Total rules: {}", bundle.rules.len());
    println!("Active rules: {}", bundle.active_rules().len());
    println!("Paused rules: {}", bundle.rules.iter().filter(|r| !r.is_active()).count());

    // Get rules sorted by priority
    println!("\nRules by priority:");
    let rules = bundle.rules_by_priority();
    for rule in rules {
        println!("  Priority {}: {} ({})",
            rule.priority(),
            rule.description.as_ref().unwrap_or(&"No description".to_string()),
            if rule.is_active() { "Active" } else { "Paused" }
        );
    }

    // Count by enforcement class
    println!("\nRules by enforcement class:");
    println!("  BlockDeny: {}", bundle.count_by_class(EnforcementClass::BlockDeny));
    println!("  Transform: {}", bundle.count_by_class(EnforcementClass::Transform));
    println!("  Observational: {}", bundle.count_by_class(EnforcementClass::Observational));
    println!();
}
