// examples/bundle_crud_usage.rs
//
// Comprehensive examples demonstrating BundleCRUD module capabilities

use rule_engine::rule_metadata::{RuleId, RuleMetadata, RuleScope, AgentId, FlowId, EnforcementMode, EnforcementClass};
use rule_engine::match_clause::{MatchClause, MatchExpression};
use rule_engine::action_clause::{ActionClause, ActionType, AllowParams, DenyParams};
use rule_engine::execution_constraints::ExecutionConstraints;
use rule_engine::rule_bundle::{Rule, BundleId, RuleBundle, RolloutPolicy};
use rule_engine::rule_table::RuleTable;
use rule_engine::hot_reload::DeploymentManager;
use rule_engine::audit_record::AuditTrail;
use rule_engine::bundle_crud::*;
use std::sync::Arc;
use std::collections::{HashMap, HashSet};
use chrono::Utc;

fn main() -> Result<(), String> {
    println!("=== BundleCRUD Module Usage Examples ===\n");

    // Example 1: Create Rule (Immediate)
    example_1_create_immediate()?;

    // Example 2: Create Rule (Staged)
    example_2_create_staged()?;

    // Example 3: Update Rule (Versioning)
    example_3_update_rule()?;

    // Example 4: Deactivate and Reactivate
    example_4_deactivate_reactivate()?;

    // Example 5: Revoke Rule
    example_5_revoke_rule()?;

    // Example 6: Bundle Operations
    example_6_bundle_operations()?;

    // Example 7: Query Operations
    example_7_query_operations()?;

    // Example 8: Complete Integration
    example_8_complete_integration()?;

    println!("\n=== All Examples Completed Successfully ===");
    Ok(())
}

// ============================================================================
// Example 1: Create Rule (Immediate Activation)
// ============================================================================

fn example_1_create_immediate() -> Result<(), String> {
    println!("--- Example 1: Create Rule (Immediate Activation) ---");

    let crud = setup_crud();

    // Create a security rule with immediate activation
    let rule_id = RuleId::new();
    let mut agent_ids = HashSet::new();
    agent_ids.insert(AgentId::new("api_gateway"));

    let mut flow_ids = HashSet::new();
    flow_ids.insert(FlowId::new("user_login"));

    let scope = RuleScope {
        is_global: false,
        agent_ids,
        flow_ids,
        dest_agent_ids: HashSet::new(),
        payload_dtypes: HashSet::new(),
    };

    let metadata = RuleMetadata::builder()
        .rule_id(rule_id.clone())
        .signer("admin".to_string())
        .scope(scope)
        .priority(100)
        .enforcement_mode(EnforcementMode::Hard)
        .enforcement_class(EnforcementClass::BlockDeny)
        .build();

    let rule = Rule {
        metadata,
        match_clause: MatchClause::new(),
        action_clause: ActionClause::new(ActionType::Allow(AllowParams::default())),
        constraints: ExecutionConstraints::default(),
        description: Some("Security check for user login".to_string()),
        tags: vec!["security".to_string(), "authentication".to_string()],
    };

    // Create with immediate rollout
    let handle = crud.create_rule(
        rule,
        None,
        Some(RolloutPolicy::Immediate),
        "admin@example.com".to_string(),
    )?;

    println!("✓ Rule created: {}", handle.operation_id());
    println!("✓ Rule ID: {}", handle.rule_id().as_str());
    println!("✓ State: ACTIVE (immediate rollout)");

    // Verify
    let stats = crud.get_rule_stats(handle.rule_id()).unwrap();
    assert_eq!(stats.state, RuleState::Active);
    assert_eq!(stats.version, 1);

    println!("✓ Verification complete\n");
    Ok(())
}

// ============================================================================
// Example 2: Create Rule (Staged Activation)
// ============================================================================

fn example_2_create_staged() -> Result<(), String> {
    println!("--- Example 2: Create Rule (Staged Activation) ---");

    let crud = setup_crud();

    // Create rule with staged activation (canary)
    let rule = create_test_rule(200);

    let handle = crud.create_rule(
        rule,
        None,
        Some(RolloutPolicy::Canary {
            percentage: 10.0,
            target_agents: None,
        }),
        "admin@example.com".to_string(),
    )?;

    println!("✓ Rule created: {}", handle.operation_id());
    println!("✓ State: STAGED (waiting for activation)");

    // Note: Stats not available for staged rules (not in rule_table yet)
    // Verify by checking if rule exists
    assert!(crud.get_rule(handle.rule_id()).is_some());

    println!("\nActivating staged rule...");
    crud.activate_rule(handle.rule_id())?;

    // Verify active
    let stats = crud.get_rule_stats(handle.rule_id()).unwrap();
    assert_eq!(stats.state, RuleState::Active);

    println!("✓ Rule activated");
    println!("✓ State: ACTIVE\n");
    Ok(())
}

// ============================================================================
// Example 3: Update Rule (Versioning)
// ============================================================================

fn example_3_update_rule() -> Result<(), String> {
    println!("--- Example 3: Update Rule (Versioning) ---");

    let crud = setup_crud();

    // Create initial rule (v1)
    let rule_v1 = create_test_rule(100);
    let rule_id = rule_v1.metadata.rule_id.clone();

    println!("Step 1: Creating v1...");
    crud.create_rule(
        rule_v1,
        None,
        Some(RolloutPolicy::Immediate),
        "admin@example.com".to_string(),
    )?;

    let history = crud.get_rule_history(&rule_id);
    println!("✓ Version history: {:?}", history);
    assert_eq!(history, vec![1]);

    // Update to v2
    println!("\nStep 2: Updating to v2...");
    let mut rule_v2 = crud.get_rule(&rule_id).unwrap();
    rule_v2.metadata.priority = 200;  // Change priority

    crud.update_rule(&rule_id, rule_v2, "admin@example.com".to_string())?;

    let history = crud.get_rule_history(&rule_id);
    println!("✓ Version history: {:?}", history);
    assert_eq!(history, vec![1, 2]);

    // At this point:
    // - v1 is still ACTIVE
    // - v2 is STAGED
    println!("✓ v1 still active, v2 staged");

    // Activate v2
    println!("\nStep 3: Activating v2...");
    crud.activate_rule(&rule_id)?;

    // Now:
    // - v2 is ACTIVE
    // - v1 is DEPRECATED
    let stats = crud.get_rule_stats(&rule_id).unwrap();
    println!("✓ v2 now active (version: {})", stats.version);
    assert_eq!(stats.version, 2);

    println!("✓ Version update complete\n");
    Ok(())
}

// ============================================================================
// Example 4: Deactivate and Reactivate
// ============================================================================

fn example_4_deactivate_reactivate() -> Result<(), String> {
    println!("--- Example 4: Deactivate and Reactivate ---");

    let crud = setup_crud();

    // Create rule
    let rule = create_test_rule(100);
    let rule_id = rule.metadata.rule_id.clone();

    println!("Step 1: Creating rule...");
    crud.create_rule(
        rule,
        None,
        Some(RolloutPolicy::Immediate),
        "admin@example.com".to_string(),
    )?;

    let stats = crud.get_rule_stats(&rule_id).unwrap();
    println!("✓ Rule created");
    println!("✓ State: {:?}", stats.state);
    assert_eq!(stats.state, RuleState::Active);

    // Deactivate (temporary pause)
    println!("\nStep 2: Deactivating rule (temporary)...");
    crud.deactivate_rule(&rule_id)?;

    // Note: Stats not available for paused rules (removed from rule_table)
    // Verify by checking if rule still exists
    assert!(crud.get_rule(&rule_id).is_some());
    println!("✓ Rule deactivated");
    println!("✓ State: Paused");
    println!("✓ Fast path stops evaluating this rule");

    // Reactivate
    println!("\nStep 3: Reactivating rule...");
    crud.reactivate_rule(&rule_id)?;

    let stats = crud.get_rule_stats(&rule_id).unwrap();
    println!("✓ Rule reactivated");
    println!("✓ State: {:?}", stats.state);
    assert_eq!(stats.state, RuleState::Active);

    println!("✓ Deactivate/reactivate cycle complete\n");
    Ok(())
}

// ============================================================================
// Example 5: Revoke Rule
// ============================================================================

fn example_5_revoke_rule() -> Result<(), String> {
    println!("--- Example 5: Revoke Rule ---");

    let crud = setup_crud();

    // Create rule
    let rule = create_test_rule(100);
    let rule_id = rule.metadata.rule_id.clone();

    println!("Step 1: Creating rule...");
    crud.create_rule(
        rule,
        None,
        Some(RolloutPolicy::Immediate),
        "admin@example.com".to_string(),
    )?;
    println!("✓ Rule created");

    // Revoke with immediate policy
    println!("\nStep 2: Revoking rule (immediate)...");
    crud.revoke_rule(&rule_id, RevocationPolicy::Immediate)?;

    // Note: Stats not available for revoked rules (removed from rule_table)
    // Verify by checking if rule still exists
    assert!(crud.get_rule(&rule_id).is_some());
    println!("✓ Rule revoked");
    println!("✓ State: Revoked");
    println!("✓ This is permanent - cannot reactivate");

    // Try to reactivate (should fail)
    println!("\nStep 3: Attempting reactivation...");
    match crud.reactivate_rule(&rule_id) {
        Ok(_) => println!("✗ Should have failed"),
        Err(e) => println!("✓ Correctly rejected: {}", e),
    }

    println!("✓ Revoke example complete\n");
    Ok(())
}

// ============================================================================
// Example 6: Bundle Operations
// ============================================================================

fn example_6_bundle_operations() -> Result<(), String> {
    println!("--- Example 6: Bundle Operations ---");

    let crud = setup_crud();

    // Create bundle with multiple rules
    println!("Step 1: Creating bundle with 3 rules...");
    let mut bundle = RuleBundle::new(
        BundleId::new("security_bundle_v1".to_string()),
        "admin".to_string(),
    );
    bundle.add_rule(create_test_rule(100));
    bundle.add_rule(create_test_rule(200));
    bundle.add_rule(create_test_rule(300));

    let handles = crud.create_bundle(
        bundle,
        None,
        "admin@example.com".to_string(),
    )?;

    println!("✓ Created {} rules", handles.len());
    for (i, handle) in handles.iter().enumerate() {
        println!("  {}. {} ({})", i + 1, handle.rule_id().as_str(), handle.operation_id());
    }

    // List active rules
    println!("\nStep 2: Listing active rules...");
    let active_rules = crud.list_rules(Some(RuleState::Active));
    println!("✓ Active rules: {}", active_rules.len());

    // Deactivate entire bundle
    println!("\nStep 3: Deactivating entire bundle...");
    let bundle_id = BundleId::new("security_bundle_v1".to_string());
    let deactivate_handles = crud.deactivate_bundle(&bundle_id)?;
    println!("✓ Deactivated {} rules", deactivate_handles.len());

    // Verify
    let paused_rules = crud.list_rules(Some(RuleState::Paused));
    println!("✓ Paused rules: {}", paused_rules.len());

    println!("✓ Bundle operations complete\n");
    Ok(())
}

// ============================================================================
// Example 7: Query Operations
// ============================================================================

fn example_7_query_operations() -> Result<(), String> {
    println!("--- Example 7: Query Operations ---");

    let crud = setup_crud();

    // Create multiple rules with different states
    println!("Step 1: Creating test rules...");

    // Active rule
    let rule1 = create_test_rule(100);
    let rule_id_1 = rule1.metadata.rule_id.clone();
    crud.create_rule(
        rule1,
        None,
        Some(RolloutPolicy::Immediate),
        "admin@example.com".to_string(),
    )?;

    // Staged rule
    let rule2 = create_test_rule(200);
    let rule_id_2 = rule2.metadata.rule_id.clone();
    crud.create_rule(
        rule2,
        None,
        Some(RolloutPolicy::Canary {
            percentage: 50.0,
            target_agents: None,
        }),
        "admin@example.com".to_string(),
    )?;

    // Paused rule
    let rule3 = create_test_rule(300);
    let rule_id_3 = rule3.metadata.rule_id.clone();
    crud.create_rule(
        rule3,
        None,
        Some(RolloutPolicy::Immediate),
        "admin@example.com".to_string(),
    )?;
    crud.deactivate_rule(&rule_id_3)?;

    println!("✓ Created 3 rules (1 active, 1 staged, 1 paused)");

    // Query all rules
    println!("\nStep 2: Query operations...");
    let all_rules = crud.list_rules(None);
    println!("Total rules: {}", all_rules.len());

    let active_rules = crud.list_rules(Some(RuleState::Active));
    println!("Active rules: {}", active_rules.len());

    let staged_rules = crud.list_rules(Some(RuleState::Staged));
    println!("Staged rules: {}", staged_rules.len());

    let paused_rules = crud.list_rules(Some(RuleState::Paused));
    println!("Paused rules: {}", paused_rules.len());

    // Get specific rule
    println!("\nStep 3: Get specific rule...");
    if let Some(rule) = crud.get_rule(&rule_id_1) {
        println!("✓ Found rule: {}", rule.metadata.rule_id.as_str());
        println!("  Priority: {}", rule.metadata.priority);
        println!("  Description: {:?}", rule.description);
    }

    // Get rule statistics
    println!("\nStep 4: Get rule statistics...");
    if let Some(stats) = crud.get_rule_stats(&rule_id_1) {
        println!("✓ Rule Statistics:");
        println!("  Rule ID: {}", stats.rule_id.as_str());
        println!("  Version: {}", stats.version);
        println!("  State: {:?}", stats.state);
        println!("  Evaluations: {}", stats.evaluation_count);
        println!("  Matches: {}", stats.match_count);
        println!("  Avg latency: {}μs", stats.avg_latency_us);
    }

    // Get version history
    println!("\nStep 5: Get version history...");
    let history = crud.get_rule_history(&rule_id_1);
    println!("✓ Version history for {}: {:?}", rule_id_1.as_str(), history);

    println!("✓ Query operations complete\n");
    Ok(())
}

// ============================================================================
// Example 8: Complete Integration
// ============================================================================

fn example_8_complete_integration() -> Result<(), String> {
    println!("--- Example 8: Complete Integration ---");

    let crud = setup_crud();

    // Scenario: Deploying a new security policy
    println!("Scenario: Deploying new security policy\n");

    // 1. Create initial rule
    println!("Step 1: Create initial security rule...");
    let rule_id = RuleId::new();

    let mut agent_ids = HashSet::new();
    agent_ids.insert(AgentId::new("api_gateway"));

    let mut flow_ids = HashSet::new();
    flow_ids.insert(FlowId::new("user_registration"));

    let scope = RuleScope {
        is_global: false,
        agent_ids,
        flow_ids,
        dest_agent_ids: HashSet::new(),
        payload_dtypes: HashSet::new(),
    };

    let metadata = RuleMetadata::builder()
        .rule_id(rule_id.clone())
        .signer("security_team".to_string())
        .scope(scope)
        .priority(500)
        .enforcement_mode(EnforcementMode::Hard)
        .enforcement_class(EnforcementClass::BlockDeny)
        .build();

    let rule_v1 = Rule {
        metadata,
        match_clause: MatchClause::new(),
        action_clause: ActionClause::new(ActionType::Allow(AllowParams::default())),
        constraints: ExecutionConstraints::default(),
        description: Some("Initial security policy for user registration".to_string()),
        tags: vec!["security".to_string(), "registration".to_string()],
    };

    let handle = crud.create_rule(
        rule_v1,
        None,
        Some(RolloutPolicy::Immediate),
        "security_team@example.com".to_string(),
    )?;

    println!("✓ Rule created: {}", handle.operation_id());

    // 2. Monitor for a while
    println!("\nStep 2: Monitoring rule performance...");
    if let Some(stats) = crud.get_rule_stats(&rule_id) {
        println!("✓ Initial state: {:?}", stats.state);
    }

    // 3. Update rule (tighten security)
    println!("\nStep 3: Updating rule (tightening security)...");
    let mut rule_v2 = crud.get_rule(&rule_id).unwrap();
    rule_v2.metadata.priority = 800;  // Higher priority
    // In real scenario, would update match/action logic

    crud.update_rule(&rule_id, rule_v2, "security_team@example.com".to_string())?;
    println!("✓ Update staged (v2)");

    let history = crud.get_rule_history(&rule_id);
    println!("✓ Version history: {:?}", history);

    // 4. Activate new version
    println!("\nStep 4: Activating new version...");
    crud.activate_rule(&rule_id)?;
    println!("✓ v2 activated");

    // 5. Verify final state
    println!("\nStep 5: Verification...");
    if let Some(stats) = crud.get_rule_stats(&rule_id) {
        println!("✓ Final Statistics:");
        println!("  Version: {}", stats.version);
        println!("  State: {:?}", stats.state);
        println!("  Priority: {}", crud.get_rule(&rule_id).unwrap().metadata.priority);
    }

    println!("\n✓ Complete integration example finished\n");
    Ok(())
}

// ============================================================================
// Helper Functions
// ============================================================================

fn setup_crud() -> BundleCRUD {
    let table = Arc::new(RuleTable::new());
    let deployment = Arc::new(DeploymentManager::new());
    let audit = Arc::new(AuditTrail::new(1000)); // max 1000 records in memory

    BundleCRUD::new(table, deployment, audit)
}

fn create_test_rule(priority: i32) -> Rule {
    let rule_id = RuleId::new();

    let mut agent_ids = HashSet::new();
    agent_ids.insert(AgentId::new("test_agent"));

    let scope = RuleScope {
        is_global: false,
        agent_ids,
        flow_ids: HashSet::new(),
        dest_agent_ids: HashSet::new(),
        payload_dtypes: HashSet::new(),
    };

    let metadata = RuleMetadata::builder()
        .rule_id(rule_id)
        .signer("test".to_string())
        .scope(scope)
        .priority(priority)
        .enforcement_mode(EnforcementMode::Hard)
        .enforcement_class(EnforcementClass::BlockDeny)
        .build();

    Rule {
        metadata,
        match_clause: MatchClause::new(),
        action_clause: ActionClause::new(ActionType::Allow(AllowParams::default())),
        constraints: ExecutionConstraints::default(),
        description: Some(format!("Test rule with priority {}", priority)),
        tags: vec![],
    }
}
