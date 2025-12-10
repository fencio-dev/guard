// This example demonstrates how to use the RuleMetadata module.
// Run with cargo run --example rule_metadata_usage
use rule_engine::{AgentId, EnforcementClass, EnforcementMode, FlowId,
    RuleMetadata, RuleScope, RuleState};

fn main() {
    println!("=== Rule Engine - Basic Usage Examples ===\n");

    // Example 1: Creating a simple global rule
    println!("Example 1: Simple Global Rule");
    println!("----------------------------------");
    let global_rule = RuleMetadata::new(
        "security-admin".to_string(),
        RuleScope::global(),
        EnforcementMode::Hard,
    );
    println!("{}", global_rule.describe());
    println!("Rule ID: {}", global_rule.rule_id());
    println!("Is active: {}", global_rule.is_active());
    println!("Is evaluable: {}", global_rule.is_evaluable());
    println!();

    // Example 2: Using the builder for more control
    println!("Example 2: Using Builder Pattern");
    println!("------------------------------------");
    let rate_limit_rule = RuleMetadata::builder()
        .signer("ops-team".to_string())
        .scope(RuleScope::global())
        .enforcement_mode(EnforcementMode::Hard)
        .enforcement_class(EnforcementClass::RateLimit)
        .priority(200)
        .state(RuleState::Active)
        .bundle_id("rate-limiting-bundle-v1".to_string())
        .build();
    println!("{}", rate_limit_rule.describe());
    println!("Bundle ID: {:?}", rate_limit_rule.bundle_id());
    println!("Priority: {}", rate_limit_rule.priority());
    println!();

    // Example 3: Agent-specific rules
    println!("Example 3: Agent-Specific Rules");
    println!("-----------------------------------");
    let agent_rule = RuleMetadata::builder()
        .signer("ml-team".to_string())
        .scope(RuleScope::for_agents(vec![
            AgentId::new("gpt-4"),
            AgentId::new("claude-3"),
            AgentId::new("gemini"),
        ]))
        .enforcement_mode(EnforcementMode::Soft)
        .enforcement_class(EnforcementClass::Observational)
        .priority(100)
        .build();
    println!("{}", agent_rule.describe());

    // Test scope matching
    let gpt4 = AgentId::new("gpt-4");
    let other_agent = AgentId::new("llama-2");
    println!("Matches GPT-4: {}", agent_rule.matches_scope(Some(&gpt4), None));
    println!("Matches Llama-2: {}", agent_rule.matches_scope(Some(&other_agent), None));
    println!();

    // Example 4: Flow-specific rules
    println!("Example 4: Flow-Specific Rules");
    println!("----------------------------------");
    let flow_rule = RuleMetadata::builder()
        .signer("compliance-team".to_string())
        .scope(RuleScope::for_flows(vec![
            FlowId::new("healthcare-conversation"),
            FlowId::new("financial-transaction"),
        ]))
        .enforcement_mode(EnforcementMode::Hard)
        .enforcement_class(EnforcementClass::Transform)
        .priority(300)
        .build();
    println!("{}", flow_rule.describe());
    
    let healthcare_flow = FlowId::new("healthcare-conversation");
    let casual_flow = FlowId::new("casual-chat");
    println!("Matches healthcare flow: {}", flow_rule.matches_scope(None, Some(&healthcare_flow)));
    println!("Matches casual flow: {}", flow_rule.matches_scope(None, Some(&casual_flow)));
    println!();

    // Example 5: Mixed scope (agents + flows)
    println!("Example 5: Mixed Scope");
    println!("-------------------------");
    let mut mixed_scope = RuleScope::new();
    mixed_scope.add_agent(AgentId::new("gpt-4"));
    mixed_scope.add_flow(FlowId::new("sensitive-data"));
    
    let mixed_rule = RuleMetadata::builder()
        .signer("security-admin".to_string())
        .scope(mixed_scope)
        .enforcement_mode(EnforcementMode::Hard)
        .enforcement_class(EnforcementClass::BlockDeny)
        .priority(500)
        .build();
    println!("{}", mixed_rule.describe());
    // Test various matching scenarios
    let gpt4 = AgentId::new("gpt-4");
    let claude = AgentId::new("claude");
    let sensitive_flow = FlowId::new("sensitive-data");
    let normal_flow = FlowId::new("normal");
    
    println!("Matches (GPT-4, normal): {}", mixed_rule.matches_scope(Some(&gpt4), Some(&normal_flow)));
    println!("Matches (Claude, sensitive): {}", mixed_rule.matches_scope(Some(&claude), Some(&sensitive_flow)));
    println!("Matches (Claude, normal): {}", mixed_rule.matches_scope(Some(&claude), Some(&normal_flow)));
    println!();

    // Example 6: Rule state transitions
    println!("Example 6: State Transitions");
    println!("--------------------------------");
    let mut stateful_rule = RuleMetadata::new(
        "admin".to_string(),
        RuleScope::global(),
        EnforcementMode::Hard,
    );
    
    println!("Initial state: {}", stateful_rule.state());
    println!("Is active: {}", stateful_rule.is_active());
    
    stateful_rule.activate();
    println!("After activation: {}", stateful_rule.state());
    println!("Is active: {}", stateful_rule.is_active());
    
    stateful_rule.pause();
    println!("After pause: {}", stateful_rule.state());
    println!("Is evaluable: {}", stateful_rule.is_evaluable());
    
    stateful_rule.revoke();
    println!("After revoke: {}", stateful_rule.state());
    println!("Is evaluable: {}", stateful_rule.is_evaluable());
    println!();

    // Example 7: Rule versioning
    println!("Example 7: Rule Versioning");
    println!("-----------------------------");
    let rule_v1 = RuleMetadata::new(
        "admin-v1".to_string(),
        RuleScope::global(),
        EnforcementMode::Hard,
    );
    println!("Version 1: {} (signer: {})", rule_v1.version(), rule_v1.signer());
    println!("Rule ID: {}", rule_v1.rule_id());
    
    let rule_v2 = rule_v1.new_version("admin-v2".to_string());
    println!("Version 2: {} (signer: {})", rule_v2.version(), rule_v2.signer());
    println!("Rule ID: {}", rule_v2.rule_id());
    println!("Same rule ID: {}", rule_v1.rule_id() == rule_v2.rule_id());
    println!();

    // Example 8: Serialization to JSON
    println!("Example 8: Serialization");
    println!("---------------------------");
    let serializable_rule = RuleMetadata::builder()
        .signer("api-user".to_string())
        .scope(RuleScope::global())
        .enforcement_mode(EnforcementMode::Soft)
        .build();
    
    match serde_json::to_string_pretty(&serializable_rule) {
        Ok(json) => {
            println!("Serialized to JSON:");
            println!("{}", json);
            
            // Deserialize back
            match serde_json::from_str::<RuleMetadata>(&json) {
                Ok(deserialized) => {
                    println!("\n✅ Successfully deserialized!");
                    println!("Rule matches: {}", serializable_rule == deserialized);
                }
                Err(e) => println!("❌ Deserialization error: {}", e),
            }
        }
        Err(e) => println!("❌ Serialization error: {}", e),
    }
    println!();

    // Example 9: Different enforcement classes
    println!("Example 9: Enforcement Classes");
    println!("----------------------------------");
    let classes = vec![
        (EnforcementClass::BlockDeny, "Block/Deny - Hard enforcement"),
        (EnforcementClass::Transform, "Transform - Mutate payloads"),
        (EnforcementClass::Augment, "Augment - Add metadata"),
        (EnforcementClass::Observational, "Observational - Log only"),
        (EnforcementClass::Control, "Control - Spawn sidecars"),
        (EnforcementClass::RateLimit, "RateLimit - Enforce quotas"),
        (EnforcementClass::Graceful, "Graceful - Log and allow"),
    ];
    
    for (class, description) in classes {
        let rule = RuleMetadata::builder()
            .signer("demo".to_string())
            .scope(RuleScope::global())
            .enforcement_mode(EnforcementMode::Hard)
            .enforcement_class(class)
            .build();
        println!("{}: {}", class, description);
    }
    println!();

    // Example 10: Priority-based evaluation order
    println!("Example 10: Priority-Based Rules");
    println!("------------------------------------");
    let mut rules = vec![
        RuleMetadata::builder()
            .signer("low-priority".to_string())
            .scope(RuleScope::global())
            .enforcement_mode(EnforcementMode::Hard)
            .priority(100)
            .build(),
        RuleMetadata::builder()
            .signer("high-priority".to_string())
            .scope(RuleScope::global())
            .enforcement_mode(EnforcementMode::Hard)
            .priority(1000)
            .build(),
        RuleMetadata::builder()
            .signer("medium-priority".to_string())
            .scope(RuleScope::global())
            .enforcement_mode(EnforcementMode::Hard)
            .priority(500)
            .build(),
    ];
    
    // Sort by priority (highest first)
    rules.sort_by(|a, b| b.priority().cmp(&a.priority()));
    
    println!("Rules in evaluation order:");
    for (i, rule) in rules.iter().enumerate() {
        println!("  {}. {} (priority: {})", i + 1, rule.signer(), rule.priority());
    }
    println!();

    println!("All examples completed successfully!");
}

