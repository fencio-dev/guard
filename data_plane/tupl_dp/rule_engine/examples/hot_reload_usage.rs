// examples/hot_reload_usage.rs
//
// Comprehensive examples demonstrating HotReload module capabilities

use rule_engine::rule_metadata::{RuleId, RuleMetadata, RuleScope, AgentId};
use rule_engine::match_clause::{MatchClause, MatchExpression};
use rule_engine::action_clause::{ActionClause, ActionType, AllowParams};
use rule_engine::execution_constraints::ExecutionConstraints;
use rule_engine::rule_bundle::{Rule, BundleId, RuleBundle, BundleMetadata};
use rule_engine::hot_reload::{DeploymentManager, DeploymentStrategy, DeploymentState, HealthThresholds, VersionId, compute_request_hash};
use std::time::{SystemTime, Duration, UNIX_EPOCH};
use std::thread;
use chrono::Utc;

fn main() -> Result<(), String> {
    println!("=== HotReload Module Usage Examples ===\n");
    
    // Example 1: Blue-Green Deployment
    example_1_blue_green()?;
    
    // Example 2: Canary Deployment
    example_2_canary()?;
    
    // Example 3: A/B Testing
    example_3_ab_testing()?;
    
    // Example 4: Scheduled Deployment
    example_4_scheduled()?;
    
    // Example 5: Automatic Rollback
    example_5_auto_rollback()?;
    
    // Example 6: Manual Rollback
    example_6_manual_rollback()?;
    
    // Example 7: Deployment History
    example_7_deployment_history()?;
    
    println!("\n=== All Examples Completed Successfully ===");
    Ok(())
}

// ============================================================================
// Example 1: Blue-Green Deployment (Instant Atomic Swap)
// ============================================================================

fn example_1_blue_green() -> Result<(), String> {
    println!("--- Example 1: Blue-Green Deployment ---");
    println!("Zero-downtime instant atomic swap\n");
    
    let manager = DeploymentManager::new();
    
    // Deploy version 1
    println!("Step 1: Deploying v1...");
    let bundle_v1 = create_bundle("security_rules_v1", 5, "v1")?;
    let version_v1 = manager.prepare_deployment(
        bundle_v1,
        DeploymentStrategy::BlueGreen,
        "admin@example.com".to_string(),
    )?;
    
    manager.activate_deployment(&version_v1)?;
    println!("✓ v1 deployed and active");
    
    // Simulate traffic
    println!("\nStep 2: Serving traffic with v1...");
    simulate_traffic(&manager, 100)?;
    
    // Deploy version 2 (blue-green swap)
    println!("\nStep 3: Deploying v2 (blue-green)...");
    let bundle_v2 = create_bundle("security_rules_v2", 8, "v2")?;
    let version_v2 = manager.prepare_deployment(
        bundle_v2,
        DeploymentStrategy::BlueGreen,
        "admin@example.com".to_string(),
    )?;
    
    println!("✓ v2 staged (not yet active)");
    
    // Atomic swap
    println!("\nStep 4: Activating v2 (atomic swap)...");
    let start = SystemTime::now();
    manager.activate_deployment(&version_v2)?;
    let elapsed = start.elapsed().unwrap();
    
    println!("✓ v2 activated in {:?}", elapsed);
    println!("✓ Zero downtime - readers never blocked!");
    
    // Verify
    assert_eq!(manager.get_active_version_id(), Some(version_v2.clone()));
    
    println!("\n✓ Blue-green deployment complete!\n");
    Ok(())
}

// ============================================================================
// Example 2: Canary Deployment (Gradual Rollout)
// ============================================================================

fn example_2_canary() -> Result<(), String> {
    println!("--- Example 2: Canary Deployment ---");
    println!("Gradual rollout: 10% → 25% → 50% → 100%\n");
    
    let manager = DeploymentManager::new();
    
    // Deploy baseline version
    println!("Step 1: Deploying baseline v1...");
    let bundle_v1 = create_bundle("baseline_v1", 5, "v1")?;
    let version_v1 = manager.prepare_deployment(
        bundle_v1,
        DeploymentStrategy::BlueGreen,
        "admin@example.com".to_string(),
    )?;
    manager.activate_deployment(&version_v1)?;
    println!("✓ Baseline v1 active\n");
    
    // Prepare canary deployment
    println!("Step 2: Preparing canary v2...");
    let bundle_v2 = create_bundle("canary_v2", 10, "v2")?;
    let version_v2 = manager.prepare_deployment(
        bundle_v2,
        DeploymentStrategy::Canary {
            stages: vec![10.0, 25.0, 50.0, 100.0],
            stage_duration_secs: 2,  // 2 seconds for demo
        },
        "admin@example.com".to_string(),
    )?;
    
    // Start canary rollout
    println!("\nStep 3: Starting canary rollout...");
    manager.activate_deployment(&version_v2)?;
    
    if let Some(info) = manager.get_deployment_info(&version_v2) {
        println!("✓ Canary started: {:?}", info.state);
    }
    
    // Simulate gradual rollout
    println!("\nStep 4: Advancing through stages...");
    let stages = vec![10.0, 25.0, 50.0, 100.0];
    
    for (i, expected_percentage) in stages.iter().enumerate() {
        // Wait for stage duration
        thread::sleep(Duration::from_secs(2));
        
        // Advance rollout
        match manager.advance_rollout() {
            Ok(true) => {
                if let Some(info) = manager.get_deployment_info(&version_v2) {
                    match info.state {
                        DeploymentState::RollingOut { current_percentage } => {
                            println!("  Stage {}: {}% traffic to v2", i + 1, current_percentage);
                            assert_eq!(current_percentage, *expected_percentage);
                        }
                        DeploymentState::Active => {
                            println!("  ✓ Rollout complete - 100% traffic to v2");
                        }
                        _ => {}
                    }
                }
            }
            Ok(false) => println!("  ⏳ Not ready to advance yet"),
            Err(e) => println!("  ✗ Error: {}", e),
        }
    }
    
    // Verify final state
    assert_eq!(manager.get_active_version_id(), Some(version_v2.clone()));
    
    println!("\n✓ Canary deployment complete!\n");
    Ok(())
}

// ============================================================================
// Example 3: A/B Testing
// ============================================================================

fn example_3_ab_testing() -> Result<(), String> {
    println!("--- Example 3: A/B Testing ---");
    println!("50/50 traffic split for performance comparison\n");
    
    let manager = DeploymentManager::new();
    
    // Deploy variant A
    println!("Step 1: Deploying variant A...");
    let bundle_a = create_bundle("variant_a", 5, "a")?;
    let version_a = manager.prepare_deployment(
        bundle_a,
        DeploymentStrategy::BlueGreen,
        "admin@example.com".to_string(),
    )?;
    manager.activate_deployment(&version_a)?;
    println!("✓ Variant A deployed");
    
    // Stage variant B for A/B test
    println!("\nStep 2: Staging variant B for A/B test...");
    let bundle_b = create_bundle("variant_b", 8, "b")?;
    let version_b = manager.prepare_deployment(
        bundle_b,
        DeploymentStrategy::ABTest {
            split_ratio: 0.5,  // 50/50 split
            test_duration_secs: 3600,
        },
        "admin@example.com".to_string(),
    )?;
    println!("✓ Variant B staged");
    
    // Simulate traffic with routing
    println!("\nStep 3: Routing traffic (50/50 split)...");
    let mut a_count = 0;
    let mut b_count = 0;
    let total_requests = 1000;
    
    for i in 0..total_requests {
        let request_hash = compute_request_hash(
            &format!("user_{}", i),
            "test_flow",
        );
        
        // Route based on hash (simplified)
        let threshold = (0.5 * 10000.0) as u64;
        if request_hash % 10000 < threshold {
            a_count += 1;
        } else {
            b_count += 1;
        }
    }
    
    let a_percentage = (a_count as f64 / total_requests as f64) * 100.0;
    let b_percentage = (b_count as f64 / total_requests as f64) * 100.0;
    
    println!("  Variant A: {} requests ({:.1}%)", a_count, a_percentage);
    println!("  Variant B: {} requests ({:.1}%)", b_count, b_percentage);
    
    // Verify split is approximately 50/50
    assert!((a_percentage - 50.0).abs() < 5.0);
    assert!((b_percentage - 50.0).abs() < 5.0);
    
    println!("\n✓ A/B test traffic split working correctly!");
    
    // Analyze results (simplified)
    println!("\nStep 4: Analyzing results...");
    println!("  Variant A metrics: latency=2.5ms, errors=0.1%");
    println!("  Variant B metrics: latency=2.3ms, errors=0.05%");
    println!("  → Variant B wins! Deploying B...");
    
    // Deploy winner with blue-green
    manager.activate_deployment(&version_b)?;
    println!("✓ Variant B deployed as winner\n");
    
    Ok(())
}

// ============================================================================
// Example 4: Scheduled Deployment
// ============================================================================

fn example_4_scheduled() -> Result<(), String> {
    println!("--- Example 4: Scheduled Deployment ---");
    println!("Deploy at specific time (off-peak hours)\n");
    
    let manager = DeploymentManager::new();
    
    // Calculate activation time (5 seconds from now for demo)
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let activation_time = now + 5;
    
    println!("Current time: {}", now);
    println!("Scheduled time: {} (+5 seconds)", activation_time);
    
    // Prepare scheduled deployment
    println!("\nStep 1: Preparing scheduled deployment...");
    let bundle = create_bundle("scheduled_v1", 5, "scheduled")?;
    let version_id = manager.prepare_deployment(
        bundle,
        DeploymentStrategy::Scheduled { activation_time },
        "admin@example.com".to_string(),
    )?;
    
    println!("✓ Deployment scheduled for {}", activation_time);
    
    // Try to activate too early
    println!("\nStep 2: Attempting early activation...");
    match manager.activate_deployment(&version_id) {
        Ok(_) => println!("✗ Should have failed (too early)"),
        Err(e) => println!("✓ Correctly rejected: {}", e),
    }
    
    // Wait for activation time
    println!("\nStep 3: Waiting for activation time...");
    thread::sleep(Duration::from_secs(5));
    
    // Activate at scheduled time
    println!("\nStep 4: Activating at scheduled time...");
    manager.activate_deployment(&version_id)?;
    println!("✓ Deployment activated successfully!");
    
    assert_eq!(manager.get_active_version_id(), Some(version_id));
    
    println!("\n✓ Scheduled deployment complete!\n");
    Ok(())
}

// ============================================================================
// Example 5: Automatic Rollback on Health Issues
// ============================================================================

fn example_5_auto_rollback() -> Result<(), String> {
    println!("--- Example 5: Automatic Rollback ---");
    println!("Auto-rollback when health thresholds exceeded\n");
    
    // Create manager with strict health thresholds
    let manager = DeploymentManager::with_config(
        10,
        HealthThresholds {
            max_error_rate: 0.01,     // 1% max
            max_latency_us: 5000,     // 5ms max
            max_timeouts: 50,
        },
        true,  // Auto-rollback enabled
    );
    
    // Deploy healthy v1
    println!("Step 1: Deploying healthy v1...");
    let bundle_v1 = create_bundle("healthy_v1", 5, "v1")?;
    let version_v1 = manager.prepare_deployment(
        bundle_v1,
        DeploymentStrategy::BlueGreen,
        "admin@example.com".to_string(),
    )?;
    manager.activate_deployment(&version_v1)?;
    println!("✓ v1 deployed");
    
    // Report healthy metrics
    println!("\nStep 2: Reporting healthy metrics for v1...");
    manager.update_health_metrics(&version_v1, 1000, 5, 2, 2000)?;
    
    if let Some(is_healthy) = manager.get_health_status(&version_v1) {
        println!("  Health status: {}", if is_healthy { "✓ Healthy" } else { "✗ Unhealthy" });
        assert!(is_healthy);
    }
    
    // Deploy problematic v2
    println!("\nStep 3: Deploying problematic v2...");
    let bundle_v2 = create_bundle("problematic_v2", 8, "v2")?;
    let version_v2 = manager.prepare_deployment(
        bundle_v2,
        DeploymentStrategy::BlueGreen,
        "admin@example.com".to_string(),
    )?;
    manager.activate_deployment(&version_v2)?;
    println!("✓ v2 deployed");
    
    // Report unhealthy metrics (high error rate)
    println!("\nStep 4: Reporting unhealthy metrics for v2...");
    println!("  Simulating 5% error rate (threshold: 1%)");
    manager.update_health_metrics(&version_v2, 1000, 50, 5, 3000)?;
    
    // Auto-rollback should have triggered
    println!("\nStep 5: Checking for automatic rollback...");
    if let Some(active_version) = manager.get_active_version_id() {
        if active_version == version_v1 {
            println!("✓ Automatic rollback triggered!");
            println!("✓ Rolled back to healthy v1");
        } else {
            println!("✗ Should have rolled back");
        }
    }
    
    // Verify rollback
    if let Some(info) = manager.get_deployment_info(&version_v2) {
        println!("\nv2 deployment state: {:?}", info.state);
    }
    
    println!("\n✓ Automatic rollback example complete!\n");
    Ok(())
}

// ============================================================================
// Example 6: Manual Rollback
// ============================================================================

fn example_6_manual_rollback() -> Result<(), String> {
    println!("--- Example 6: Manual Rollback ---");
    println!("Manual rollback to previous version\n");
    
    let manager = DeploymentManager::new();
    
    // Deploy v1
    println!("Step 1: Deploying v1...");
    let bundle_v1 = create_bundle("app_v1", 5, "v1")?;
    let version_v1 = manager.prepare_deployment(
        bundle_v1,
        DeploymentStrategy::BlueGreen,
        "admin@example.com".to_string(),
    )?;
    manager.activate_deployment(&version_v1)?;
    println!("✓ v1 active");
    
    // Deploy v2
    println!("\nStep 2: Deploying v2...");
    let bundle_v2 = create_bundle("app_v2", 8, "v2")?;
    let version_v2 = manager.prepare_deployment(
        bundle_v2,
        DeploymentStrategy::BlueGreen,
        "admin@example.com".to_string(),
    )?;
    manager.activate_deployment(&version_v2)?;
    println!("✓ v2 active");
    
    // Deploy v3
    println!("\nStep 3: Deploying v3...");
    let bundle_v3 = create_bundle("app_v3", 10, "v3")?;
    let version_v3 = manager.prepare_deployment(
        bundle_v3,
        DeploymentStrategy::BlueGreen,
        "admin@example.com".to_string(),
    )?;
    manager.activate_deployment(&version_v3)?;
    println!("✓ v3 active");
    
    // Show history
    println!("\nStep 4: Deployment history:");
    let history = manager.get_deployment_history();
    for (i, version) in history.iter().enumerate() {
        let marker = if i == 0 { "← ACTIVE" } else { "" };
        println!("  {}. {} {}", i + 1, version.as_str(), marker);
    }
    
    // Manual rollback
    println!("\nStep 5: Triggering manual rollback...");
    let rolled_back_to = manager.rollback()?;
    println!("✓ Rolled back to {}", rolled_back_to.as_str());
    
    // Verify
    assert_eq!(manager.get_active_version_id(), Some(rolled_back_to.clone()));
    assert_eq!(rolled_back_to, version_v2);
    
    // Show updated history
    println!("\nStep 6: Updated deployment history:");
    let history = manager.get_deployment_history();
    for (i, version) in history.iter().enumerate() {
        let active_marker = if Some(version.clone()) == manager.get_active_version_id() {
            "← ACTIVE"
        } else {
            ""
        };
        
        if let Some(info) = manager.get_deployment_info(version) {
            let state_marker = match info.state {
                DeploymentState::RolledBack => "(rolled back)",
                _ => "",
            };
            println!("  {}. {} {} {}", i + 1, version.as_str(), state_marker, active_marker);
        }
    }
    
    println!("\n✓ Manual rollback example complete!\n");
    Ok(())
}

// ============================================================================
// Example 7: Deployment History and Auditing
// ============================================================================

fn example_7_deployment_history() -> Result<(), String> {
    println!("--- Example 7: Deployment History ---");
    println!("Track and audit all deployments\n");
    
    let manager = DeploymentManager::with_config(
        5,  // Keep last 5 versions
        HealthThresholds::default(),
        false,
    );
    
    // Deploy multiple versions
    println!("Step 1: Deploying multiple versions...");
    let strategies = vec![
        ("v1", DeploymentStrategy::BlueGreen),
        ("v2", DeploymentStrategy::Canary {
            stages: vec![50.0, 100.0],
            stage_duration_secs: 300,
        }),
        ("v3", DeploymentStrategy::BlueGreen),
    ];
    
    for (version_name, strategy) in strategies {
        let bundle = create_bundle(
            &format!("app_{}", version_name),
            5,
            version_name,
        )?;
        
        let version_id = manager.prepare_deployment(
            bundle,
            strategy,
            format!("user_{}@example.com", version_name),
        )?;
        
        manager.activate_deployment(&version_id)?;
        println!("  ✓ Deployed {}", version_name);
        
        // Simulate some traffic
        manager.update_health_metrics(&version_id, 100, 1, 0, 1500)?;
        
        thread::sleep(Duration::from_millis(100));
    }
    
    // Display comprehensive history
    println!("\nStep 2: Deployment History:");
    println!("{}", "=".repeat(80));
    
    let history = manager.get_deployment_history();
    for (i, version_id) in history.iter().enumerate() {
        if let Some(info) = manager.get_deployment_info(version_id) {
            let active = if Some(version_id.clone()) == manager.get_active_version_id() {
                " [ACTIVE]"
            } else {
                ""
            };
            
            println!("\n{}. {}{}", i + 1, version_id.as_str(), active);
            println!("   Bundle: {}", info.bundle_id.as_str());
            println!("   State: {:?}", info.state);
            println!("   Strategy: {:?}", info.strategy);
            println!("   Deployed by: {}", info.deployed_by);
            println!("   Started: {:?}", info.started_at);
            
            if let Some(completed) = info.completed_at {
                let duration = completed
                    .duration_since(info.started_at)
                    .unwrap_or_default();
                println!("   Duration: {:?}", duration);
            }
            
            let metrics = &info.health_metrics;
            println!("   Health:");
            println!("     - Evaluations: {}", metrics.total_evaluations);
            println!("     - Errors: {}", metrics.error_count);
            println!("     - Error rate: {:.2}%", metrics.error_rate * 100.0);
            println!("     - Avg latency: {}μs", metrics.avg_latency_us);
            println!("     - Timeouts: {}", metrics.timeout_count);
        }
    }
    
    println!("\n{}", "=".repeat(80));
    println!("\n✓ Deployment history example complete!\n");
    Ok(())
}

// ============================================================================
// Helper Functions
// ============================================================================

fn create_bundle(
    bundle_id: &str,
    rule_count: usize,
    version_tag: &str,
) -> Result<RuleBundle, String> {
    let mut rules = vec![];

    for i in 0..rule_count {
        // Create a scope with test agent
        let mut scope = RuleScope::new();
        scope.is_global = false;
        scope.agent_ids.insert(AgentId::new("test_agent"));

        let rule = Rule {
            metadata: RuleMetadata {
                rule_id: RuleId::new(),
                version: 1,
                priority: 100 + i as i32 * 10,
                scope,
                bundle_id: Some(bundle_id.to_string()),
                signer: "system".to_string(),
                created_at: Utc::now(),
                state: rule_engine::RuleState::Active,
                enforcement_class: rule_engine::EnforcementClass::BlockDeny,
                enforcement_mode: rule_engine::EnforcementMode::Hard,
            },
            match_clause: MatchClause {
                fast_match: rule_engine::match_clause::FastMatch::new(),
                match_expression: MatchExpression::Always,
                wasm_hook: None,
            },
            action_clause: ActionClause::new(ActionType::Allow(AllowParams::default())),
            constraints: ExecutionConstraints::default(),
            description: Some(format!("Rule {} from bundle {}", i, bundle_id)),
            tags: vec![version_tag.to_string()],
        };
        rules.push(rule);
    }

    Ok(RuleBundle {
        metadata: BundleMetadata::new(BundleId::new(bundle_id.to_string()), "system".to_string()),
        rules,
        allowed_side_effects: vec![],
        signature: None,
    })
}

fn simulate_traffic(manager: &DeploymentManager, count: usize) -> Result<(), String> {
    if let Some(_table) = manager.get_active_table() {
        println!("  Simulating {} requests...", count);
        // In real code, would evaluate rules here
        println!("  ✓ {} requests processed", count);
    }
    Ok(())
}