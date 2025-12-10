// examples/rule_table_usage.rs
//
// Comprehensive example demonstrating RuleTable usage patterns

use rule_engine::rule_metadata::{RuleId, RuleMetadata, RuleScope, AgentId, RuleState, EnforcementClass, EnforcementMode};
use rule_engine::match_clause::MatchClause;
use rule_engine::action_clause::{ActionClause, ActionType, AllowParams};
use rule_engine::execution_constraints::ExecutionConstraints;
use rule_engine::rule_bundle::{Rule, BundleId};
use rule_engine::rule_table::{RuleTable, RuleQuery};
use std::collections::HashSet;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use chrono::Utc;

fn main() -> Result<(), String> {
    println!("=== RuleTable Usage Examples ===\n");

    // Example 1: Basic operations
    basic_operations()?;

    // Example 2: Multi-index queries
    multi_index_queries()?;

    // Example 3: Bundle operations
    bundle_operations()?;

    // Example 4: Statistics tracking
    statistics_tracking()?;

    // Example 5: Decision caching
    decision_caching()?;

    // Example 6: Thread-safe concurrent access
    concurrent_access()?;

    println!("\n=== All Examples Completed Successfully ===");
    Ok(())
}

// ============================================================================
// Example 1: Basic Operations
// ============================================================================

fn basic_operations() -> Result<(), String> {
    println!("--- Example 1: Basic Operations ---");

    // Create table with default config (60s TTL, 10k cache)
    let table = RuleTable::new();

    // Create a simple rule
    let rule = create_sample_rule(
        "api_gateway",
        100,
        "Rate limit API requests",
    );

    let rule_id = rule.metadata.rule_id.clone();

    // Add rule
    table.add_rule(rule, None)?;
    println!("✓ Added rule: {}", rule_id.as_str());

    // Get rule
    if let Some(entry) = table.get_rule(&rule_id) {
        println!("✓ Retrieved rule: {}", entry.rule_id().as_str());
        println!("  Priority: {}", entry.priority());
        println!("  Activated at: {:?}", entry.activated_at);
    }

    // Check table stats
    println!("✓ Total rules: {}", table.len());

    // Remove rule
    let removed = table.remove_rule(&rule_id)?;
    println!("✓ Removed rule: {}", removed.rule_id().as_str());
    println!("✓ Total rules after removal: {}\n", table.len());

    Ok(())
}

// ============================================================================
// Example 2: Multi-Index Queries
// ============================================================================

fn multi_index_queries() -> Result<(), String> {
    println!("--- Example 2: Multi-Index Queries ---");

    let table = RuleTable::new();

    // Add rules with different scopes
    table.add_rule(create_sample_rule("agent1", 100, "Rule for agent1"), None)?;
    table.add_rule(create_sample_rule("agent2", 90, "Rule for agent2"), None)?;
    table.add_rule(create_global_rule(50, "Global rule"), None)?;

    println!("✓ Added 3 rules");

    // Query by agent
    let query = RuleQuery::new()
        .with_agent("agent1".to_string());
    let results = table.query(&query);
    println!("✓ Query by agent1: {} rules (includes global)", results.len());

    // Query by flow
    let query = RuleQuery::new()
        .with_flow("flow1".to_string());
    let results = table.query(&query);
    println!("✓ Query by flow1: {} rules (global only)", results.len());

    // Query by dest_agent
    let query = RuleQuery::new()
        .with_dest_agent("dest1".to_string());
    let results = table.query(&query);
    println!("✓ Query by dest1: {} rules\n", results.len());

    Ok(())
}

// ============================================================================
// Example 3: Bundle Operations
// ============================================================================

fn bundle_operations() -> Result<(), String> {
    println!("--- Example 3: Bundle Operations ---");

    let table = RuleTable::new();
    let bundle_id = BundleId::new("auth_bundle_v1".to_string());

    // Create rules for bundle
    let rules = vec![
        create_sample_rule("auth_service", 100, "Auth rule 1"),
        create_sample_rule("auth_service", 90, "Auth rule 2"),
        create_sample_rule("auth_service", 80, "Auth rule 3"),
    ];

    // Load entire bundle atomically
    let count = table.load_bundle(rules, bundle_id.clone())?;
    println!("✓ Loaded bundle '{}' with {} rules", bundle_id.as_str(), count);
    println!("  Total rules in table: {}", table.len());

    // Unload bundle
    let count = table.unload_bundle(&bundle_id)?;
    println!("✓ Unloaded bundle: {} rules removed", count);
    println!("  Total rules in table: {}\n", table.len());

    Ok(())
}

// ============================================================================
// Example 4: Statistics Tracking
// ============================================================================

fn statistics_tracking() -> Result<(), String> {
    println!("--- Example 4: Statistics Tracking ---");

    let table = RuleTable::new();
    let rule = create_sample_rule("metrics_test", 100, "Test rule for stats");
    let rule_id = rule.metadata.rule_id.clone();

    table.add_rule(rule, None)?;
    println!("✓ Added rule for statistics tracking");

    // Simulate evaluations
    for i in 0..5 {
        table.update_stats(&rule_id, |stats| {
            stats.record_evaluation(i % 2 == 0, 100 + i * 10);
        })?;
    }
    println!("✓ Recorded 5 evaluations");

    // Get stats
    if let Some(entry) = table.get_rule(&rule_id) {
        let stats = &entry.stats;
        println!("  Evaluation count: {}", stats.evaluation_count);
        println!("  Match count: {}", stats.match_count);
        println!("  Match rate: {:.2}%", stats.match_rate() * 100.0);
        println!("  Avg eval time: {} μs", stats.avg_eval_time_us());
    }

    // Get table-level stats
    let table_stats = table.get_table_stats();
    println!("\n  Table Statistics:");
    println!("    Total rules: {}", table_stats.total_rules);
    println!("    Global rules: {}", table_stats.global_rules);
    println!("    Agent indexes: {}", table_stats.agent_indexes);
    println!("    Cache size: {}\n", table_stats.cache_size);

    Ok(())
}

// ============================================================================
// Example 5: Decision Caching
// ============================================================================

fn decision_caching() -> Result<(), String> {
    println!("--- Example 5: Decision Caching ---");

    let table = RuleTable::with_config(30, 1000); // 30s TTL, 1k cache size

    let rule = create_sample_rule("cache_test", 100, "Test rule for caching");
    let rule_id = rule.metadata.rule_id.clone();
    table.add_rule(rule, None)?;

    let agent_id = "test_agent";
    let flow_id = "test_flow";
    let event_hash = 12345u64;

    // Cache a decision
    table.cache_decision(
        agent_id,
        flow_id,
        event_hash,
        rule_id.clone(),
        "allow".to_string(),
    )?;
    println!("✓ Cached decision for event hash {}", event_hash);

    // Retrieve cached decision
    if let Some((cached_rule_id, decision)) = table.get_cached_decision(agent_id, flow_id, event_hash) {
        println!("✓ Retrieved cached decision:");
        println!("  Rule ID: {}", cached_rule_id.as_str());
        println!("  Decision: {}", decision);
    }

    // Clear cache
    table.clear_cache()?;
    println!("✓ Cache cleared");

    // Try to get (should be None now)
    let result = table.get_cached_decision(agent_id, flow_id, event_hash);
    println!("✓ After clear: {:?}\n", result.is_some());

    Ok(())
}

// ============================================================================
// Example 6: Thread-Safe Concurrent Access
// ============================================================================

fn concurrent_access() -> Result<(), String> {
    println!("--- Example 6: Thread-Safe Concurrent Access ---");

    let table = Arc::new(RuleTable::new());

    // Pre-populate with some rules
    for i in 0..10 {
        let rule = create_sample_rule(
            &format!("agent_{}", i),
            100 - (i as i32),
            &format!("Rule {}", i),
        );
        table.add_rule(rule, None)?;
    }
    println!("✓ Pre-populated table with 10 rules");

    // Spawn reader threads (lock-free!)
    let mut handles = vec![];
    for i in 0..5 {
        let table_clone = Arc::clone(&table);
        let handle = thread::spawn(move || {
            for _ in 0..100 {
                let query = RuleQuery::new()
                    .with_agent(format!("agent_{}", i));
                let _results = table_clone.query(&query);
            }
        });
        handles.push(handle);
    }

    // Wait for completion
    for handle in handles {
        handle.join().unwrap();
    }

    println!("✓ Completed 500 concurrent queries across 5 threads");
    println!("  Final rule count: {}\n", table.len());

    Ok(())
}

// ============================================================================
// Helper Functions
// ============================================================================

fn create_sample_rule(agent_id: &str, priority: i32, description: &str) -> Rule {
    let mut agent_ids = HashSet::new();
    agent_ids.insert(AgentId::new(agent_id));

    Rule {
        metadata: RuleMetadata {
            rule_id: RuleId::new(),
            version: 1,
            bundle_id: None,
            signer: "admin".to_string(),
            created_at: Utc::now(),
            scope: RuleScope {
                is_global: false,
                agent_ids,
                flow_ids: HashSet::new(),
                dest_agent_ids: HashSet::new(),
                payload_dtypes: HashSet::new(),
            },
            priority,
            state: RuleState::Active,
            enforcement_class: EnforcementClass::BlockDeny,
            enforcement_mode: EnforcementMode::Hard,
        },
        match_clause: MatchClause::default(),
        action_clause: ActionClause {
            primary_action: ActionType::Allow(AllowParams::default()),
            secondary_actions: vec![],
            allowed_side_effects: HashSet::new(),
            max_execution_time: Duration::from_millis(100),
            rollback_on_failure: false,
        },
        constraints: ExecutionConstraints::default(),
        description: Some(description.to_string()),
        tags: vec!["example".to_string()],
    }
}

fn create_global_rule(priority: i32, description: &str) -> Rule {
    Rule {
        metadata: RuleMetadata {
            rule_id: RuleId::new(),
            version: 1,
            bundle_id: None,
            signer: "admin".to_string(),
            created_at: Utc::now(),
            scope: RuleScope {
                is_global: true,
                agent_ids: HashSet::new(),
                flow_ids: HashSet::new(),
                dest_agent_ids: HashSet::new(),
                payload_dtypes: HashSet::new(),
            },
            priority,
            state: RuleState::Active,
            enforcement_class: EnforcementClass::BlockDeny,
            enforcement_mode: EnforcementMode::Hard,
        },
        match_clause: MatchClause::default(),
        action_clause: ActionClause {
            primary_action: ActionType::Allow(AllowParams::default()),
            secondary_actions: vec![],
            allowed_side_effects: HashSet::new(),
            max_execution_time: Duration::from_millis(100),
            rollback_on_failure: false,
        },
        constraints: ExecutionConstraints::default(),
        description: Some(description.to_string()),
        tags: vec!["global".to_string()],
    }
}
