// examples/action_clause_usage.rs
//
// This example demonstrates the complete usage of the ActionClause module,
// including all action types, parameters, and execution patterns.
//
// Run with: cargo run --example action_clause_usage

use rule_engine::action_clause::*;
use rule_engine::AgentId;
use std::collections::HashMap;
use std::time::Duration;

fn main() {
    println!("=== Action Clause - Comprehensive Examples ===\n");

    // ========================================================================
    // Example 1: DENY Action - Block Requests
    // ========================================================================
    println!("Example 1: DENY Action");
    println!("--------------------------");

    let deny_action = ActionType::Deny(DenyParams {
        reason: "Request contains prohibited content".to_string(),
        error_code: 403,
        http_status: Some(403),
    });

    println!("Action: {}", deny_action.name());
    println!("  Is blocking: {}", deny_action.is_blocking());
    println!("  Requires auth: {}", deny_action.requires_authorization());
    println!("  Modifies payload: {}", deny_action.modifies_payload());

    let deny_clause = ActionClause::new(deny_action);
    println!("  Validation: {:?}", deny_clause.validate());
    println!();

    // ========================================================================
    // Example 2: ALLOW Action - Explicit Allow with Logging
    // ========================================================================
    println!("Example 2: ALLOW Action");
    println!("--------------------------");

    let allow_clause = ActionClause::builder(ActionType::Allow(AllowParams {
        log_decision: true,
        reason: Some("Passed all security checks".to_string()),
    }))
    .add_secondary(ActionType::Log(LogParams {
        level: LogLevel::Info,
        message: "Request allowed after validation".to_string(),
        include_payload: false,
        structured_data: None,
    }))
    .max_execution_time(Duration::from_millis(50))
    .build()
    .unwrap();

    println!("Primary action: {}", allow_clause.primary_action.name());
    println!("Secondary actions: {}", allow_clause.secondary_actions.len());
    println!("Allowed side effects: {:?}", allow_clause.allowed_side_effects);
    println!();

    // ========================================================================
    // Example 3: REWRITE Action - Modify Payload
    // ========================================================================
    println!("Example 3: REWRITE Action");
    println!("----------------------------");

    let rewrite_action = ActionType::Rewrite(RewriteParams {
        operations: vec![
            RewriteOperation::SetField {
                path: "metadata.processed_at".to_string(),
                value: "2024-01-15T10:30:00Z".to_string(),
            },
            RewriteOperation::SetField {
                path: "metadata.version".to_string(),
                value: "v2".to_string(),
            },
            RewriteOperation::Transform {
                path: "user.email".to_string(),
                function: TransformFunction::Lowercase,
            },
        ],
        preserve_original: true,
    });

    let rewrite_clause = ActionClause::new(rewrite_action);
    println!("Modifies payload: {}", rewrite_clause.requires_payload());
    println!("Number of operations: 3");
    println!("  - SetField: metadata.processed_at");
    println!("  - SetField: metadata.version");
    println!("  - Transform: user.email → lowercase");
    println!();

    // ========================================================================
    // Example 4: REDACT Action - Remove Sensitive Data
    // ========================================================================
    println!("Example 4: REDACT Action");
    println!("---------------------------");

    // Mask strategy
    let mask_redact = ActionType::Redact(RedactParams {
        fields: vec![
            "ssn".to_string(),
            "credit_card".to_string(),
            "password".to_string(),
        ],
        strategy: RedactionStrategy::Mask,
        redaction_template: Some("***REDACTED***".to_string()),
    });

    println!("Mask Redaction:");
    println!("  Fields: ssn, credit_card, password");
    println!("  Strategy: Mask with '***REDACTED***'");

    // Hash strategy
    let hash_redact = ActionType::Redact(RedactParams {
        fields: vec!["email".to_string()],
        strategy: RedactionStrategy::Hash,
        redaction_template: None,
    });

    println!("\nHash Redaction:");
    println!("  Fields: email");
    println!("  Strategy: Replace with SHA256 hash");

    // Partial strategy
    let partial_redact = ActionType::Redact(RedactParams {
        fields: vec!["phone".to_string()],
        strategy: RedactionStrategy::Partial,
        redaction_template: None,
    });

    println!("\nPartial Redaction:");
    println!("  Fields: phone");
    println!("  Strategy: Show last 4 digits only");
    println!("  Example: 555-123-4567 → ***-***-4567");
    println!();

    // ========================================================================
    // Example 5: SPAWN_SIDECAR Action - Launch Analysis Process
    // ========================================================================
    println!("Example 5: SPAWN_SIDECAR Action");
    println!("----------------------------------");

    let sidecar_action = ActionType::SpawnSidecar(SpawnSidecarParams {
        sidecar_spec: SidecarSpec {
            sidecar_type: "ml-sentiment-analyzer".to_string(),
            image: "security/ml-analyzer:v1.2.3".to_string(),
            cpu_shares: 200,
            memory_limit_mb: 512,
            timeout: Duration::from_secs(30),
        },
        block_on_completion: false,
        pass_payload: true,
    });

    println!("Sidecar Configuration:");
    if let ActionType::SpawnSidecar(params) = &sidecar_action {
        println!("  Type: {}", params.sidecar_spec.sidecar_type);
        println!("  Image: {}", params.sidecar_spec.image);
        println!("  CPU Shares: {}", params.sidecar_spec.cpu_shares);
        println!("  Memory Limit: {} MB", params.sidecar_spec.memory_limit_mb);
        println!("  Timeout: {:?}", params.sidecar_spec.timeout);
        println!("  Blocking: {}", params.block_on_completion);
        println!("  Pass Payload: {}", params.pass_payload);
    }
    println!("  Requires authorization: {}", sidecar_action.requires_authorization());
    println!();

    // ========================================================================
    // Example 6: ROUTE_TO Action - Change Destination
    // ========================================================================
    println!("Example 6: ROUTE_TO Action");
    println!("-----------------------------");

    // Route to different agent
    let route_agent = ActionType::RouteTo(RouteToParams {
        dest_agent: Some(AgentId::new("security-review-agent")),
        queue_name: None,
        preserve_headers: true,
    });

    println!("Route to Agent:");
    if let ActionType::RouteTo(params) = &route_agent {
        if let Some(agent) = &params.dest_agent {
            println!("  Destination: {}", agent);
        }
        println!("  Preserve Headers: {}", params.preserve_headers);
    }

    // Route to queue
    let route_queue = ActionType::RouteTo(RouteToParams {
        dest_agent: None,
        queue_name: Some("high-priority-queue".to_string()),
        preserve_headers: true,
    });

    println!("\nRoute to Queue:");
    if let ActionType::RouteTo(params) = &route_queue {
        if let Some(queue) = &params.queue_name {
            println!("  Queue: {}", queue);
        }
    }
    println!();

    // ========================================================================
    // Example 7: RATE_LIMIT Action - Enforce Quotas
    // ========================================================================
    println!("Example 7: RATE_LIMIT Action");
    println!("--------------------------------");

    // Per-agent rate limit
    let rate_limit_agent = ActionType::RateLimit(RateLimitParams {
        max_requests: 100,
        window: Duration::from_secs(60),
        scope: RateLimitScope::PerAgent,
        action_on_exceed: Box::new(ActionType::Deny(DenyParams {
            reason: "Rate limit exceeded".to_string(),
            error_code: 429,
            http_status: Some(429),
        })),
    });

    println!("Per-Agent Rate Limit:");
    if let ActionType::RateLimit(params) = &rate_limit_agent {
        println!("  Max Requests: {}", params.max_requests);
        println!("  Window: {:?}", params.window);
        println!("  Scope: {:?}", params.scope);
        println!("  On Exceed: {}", params.action_on_exceed.name());
    }

    // Global rate limit
    let rate_limit_global = ActionType::RateLimit(RateLimitParams {
        max_requests: 1000,
        window: Duration::from_secs(3600),
        scope: RateLimitScope::Global,
        action_on_exceed: Box::new(ActionType::Log(LogParams {
            level: LogLevel::Warning,
            message: "Global rate limit approaching".to_string(),
            include_payload: false,
            structured_data: None,
        })),
    });

    println!("\nGlobal Rate Limit:");
    if let ActionType::RateLimit(params) = &rate_limit_global {
        println!("  Max Requests: {} per hour", params.max_requests);
        println!("  Scope: {:?}", params.scope);
    }
    println!();

    // ========================================================================
    // Example 8: LOG Action - Observability
    // ========================================================================
    println!("Example 8: LOG Action");
    println!("------------------------");

    let mut structured_data = HashMap::new();
    structured_data.insert("severity".to_string(), "high".to_string());
    structured_data.insert("category".to_string(), "security".to_string());
    structured_data.insert("rule_id".to_string(), "rule-001".to_string());

    let log_action = ActionType::Log(LogParams {
        level: LogLevel::Warning,
        message: "Suspicious activity detected: Multiple failed auth attempts".to_string(),
        include_payload: false,
        structured_data: Some(structured_data),
    });

    println!("Log Configuration:");
    if let ActionType::Log(params) = &log_action {
        println!("  Level: {:?}", params.level);
        println!("  Message: {}", params.message);
        println!("  Include Payload: {}", params.include_payload);
        if let Some(data) = &params.structured_data {
            println!("  Structured Data:");
            for (key, value) in data {
                println!("    {}: {}", key, value);
            }
        }
    }

    println!("\nLog Levels (ordered by severity):");
    println!("  Debug < Info < Warning < Error < Critical");
    println!();

    // ========================================================================
    // Example 9: ATTACH_METADATA Action - Enrich Events
    // ========================================================================
    println!("Example 9: ATTACH_METADATA Action");
    println!("-------------------------------------");

    let mut metadata = HashMap::new();
    metadata.insert("security_scan".to_string(), "passed".to_string());
    metadata.insert("scan_timestamp".to_string(), "2024-01-15T10:30:00Z".to_string());
    metadata.insert("scan_version".to_string(), "v2.1.0".to_string());
    metadata.insert("threat_level".to_string(), "low".to_string());

    let metadata_action = ActionType::AttachMetadata(AttachMetadataParams {
        metadata: metadata.clone(),
        overwrite_existing: false,
    });

    println!("Metadata to Attach:");
    for (key, value) in &metadata {
        println!("  {}: {}", key, value);
    }
    println!("  Overwrite Existing: false");
    println!();

    // ========================================================================
    // Example 10: CALLBACK Action - Notify Control Plane
    // ========================================================================
    println!("Example 10: CALLBACK Action");
    println!("------------------------------");

    let callback_action = ActionType::Callback(CallbackParams {
        endpoint: "https://control-plane.example.com/events".to_string(),
        event_type: "security.policy_violation".to_string(),
        include_payload: true,
        async_delivery: true,
    });

    println!("Callback Configuration:");
    if let ActionType::Callback(params) = &callback_action {
        println!("  Endpoint: {}", params.endpoint);
        println!("  Event Type: {}", params.event_type);
        println!("  Include Payload: {}", params.include_payload);
        println!("  Async Delivery: {}", params.async_delivery);
    }
    println!("  Requires authorization: {}", callback_action.requires_authorization());
    println!();

    // ========================================================================
    // Example 11: SANDBOX_EXECUTE Action - Custom Logic
    // ========================================================================
    println!("Example 11: SANDBOX_EXECUTE Action");
    println!("-------------------------------------");

    let mut input_params = HashMap::new();
    input_params.insert("threshold".to_string(), "0.8".to_string());
    input_params.insert("model".to_string(), "sentiment-v2".to_string());

    let sandbox_action = ActionType::SandboxExecute(SandboxExecuteParams {
        module_id: "custom-content-filter".to_string(),
        module_digest: "sha256:fedcba9876543210fedcba9876543210".to_string(),
        max_exec_time: Duration::from_millis(100),
        memory_limit_mb: 50,
        input_params: Some(input_params),
    });

    println!("Sandbox Configuration:");
    if let ActionType::SandboxExecute(params) = &sandbox_action {
        println!("  Module ID: {}", params.module_id);
        println!("  Module Digest: {}", params.module_digest);
        println!("  Max Execution Time: {:?}", params.max_exec_time);
        println!("  Memory Limit: {} MB", params.memory_limit_mb);
        if let Some(params_map) = &params.input_params {
            println!("  Input Parameters:");
            for (key, value) in params_map {
                println!("    {}: {}", key, value);
            }
        }
    }
    println!();

    // ========================================================================
    // Example 12: Complex Action Clause - Multiple Actions
    // ========================================================================
    println!("Example 12: Complex Action Clause");
    println!("------------------------------------");

    let complex_clause = ActionClause::builder(ActionType::Allow(AllowParams {
        log_decision: true,
        reason: Some("Passed enhanced security checks".to_string()),
    }))
    .add_secondary(ActionType::AttachMetadata(AttachMetadataParams {
        metadata: {
            let mut m = HashMap::new();
            m.insert("security_level".to_string(), "high".to_string());
            m.insert("checked_at".to_string(), "2024-01-15T10:30:00Z".to_string());
            m
        },
        overwrite_existing: false,
    }))
    .add_secondary(ActionType::Log(LogParams {
        level: LogLevel::Info,
        message: "High-security request processed".to_string(),
        include_payload: false,
        structured_data: None,
    }))
    .max_execution_time(Duration::from_millis(200))
    .rollback_on_failure(true)
    .build()
    .unwrap();

    println!("Action Clause Configuration:");
    println!("  Primary Action: {}", complex_clause.primary_action.name());
    println!("  Secondary Actions: {}", complex_clause.secondary_actions.len());
    for (i, action) in complex_clause.secondary_actions.iter().enumerate() {
        println!("    {}. {}", i + 1, action.name());
    }
    println!("  Max Execution Time: {:?}", complex_clause.max_execution_time);
    println!("  Rollback on Failure: {}", complex_clause.rollback_on_failure);
    println!("  Allowed Side Effects: {} types", complex_clause.allowed_side_effects.len());
    println!();

    // ========================================================================
    // Example 13: Action Results - Execution Outcomes
    // ========================================================================
    println!("📊 Example 13: Action Results");
    println!("-----------------------------");

    let success = ActionResult::Success {
        message: "Action executed successfully".to_string(),
        payload_modified: true,
        metadata_modified: true,
    };
    println!("Success Result:");
    println!("  Is Success: {}", success.is_success());
    println!("  Should Block: {}", success.should_block());

    let denied = ActionResult::Denied {
        reason: "Policy violation detected".to_string(),
        error_code: "ERR_POLICY_001".to_string(),
    };
    println!("\nDenied Result:");
    println!("  Is Denied: {}", denied.is_denied());
    println!("  Should Block: {}", denied.should_block());

    let failed = ActionResult::Failed {
        error: "Network timeout".to_string(),
        retryable: true,
    };
    println!("\nFailed Result:");
    println!("  Is Failed: {}", failed.is_failed());
    println!("  Should Block: {}", failed.should_block());

    let timeout = ActionResult::Timeout {
        elapsed: Duration::from_millis(150),
    };
    println!("\nTimeout Result:");
    println!("  Elapsed: {:?}", if let ActionResult::Timeout { elapsed } = timeout {
        elapsed
    } else {
        Duration::from_secs(0)
    });
    println!();

    // ========================================================================
    // Example 14: Side Effects Management
    // ========================================================================
    println!("Example 14: Side Effects Management");
    println!("--------------------------------------");

    println!("Available Side Effects:");
    println!("  - Logging: Write to logs");
    println!("  - Metrics: Update counters");
    println!("  - PayloadModification: Modify request");
    println!("  - MetadataModification: Add headers");
    println!("  - StateModification: Update state");
    println!("  - ProcessSpawn: Launch processes");
    println!("  - ResourceAllocation: Allocate resources");
    println!("  - NetworkCall: Make external calls");
    println!("  - Routing: Change destination");
    println!("  - SandboxExecution: Execute WASM");

    println!("\nSide Effect Requirements by Action:");
    let actions = vec![
        ActionType::Deny(DenyParams::default()),
        ActionType::Rewrite(RewriteParams {
            operations: vec![],
            preserve_original: false,
        }),
        ActionType::SpawnSidecar(SpawnSidecarParams {
            sidecar_spec: SidecarSpec {
                sidecar_type: "test".to_string(),
                image: "test:latest".to_string(),
                cpu_shares: 100,
                memory_limit_mb: 256,
                timeout: Duration::from_secs(10),
            },
            block_on_completion: false,
            pass_payload: false,
        }),
    ];

    for action in actions {
        println!("\n  {}:", action.name());
        let clause = ActionClause::new(action);
        for effect in &clause.allowed_side_effects {
            println!("    - {:?}", effect);
        }
    }
    println!();

    // ========================================================================
    // Example 15: Real-World Use Cases
    // ========================================================================
    println!("Example 15: Real-World Use Cases");
    println!("-----------------------------------");

    // Use Case 1: PII Detection and Redaction
    println!("Use Case 1: PII Detection and Redaction");
    let pii_clause = ActionClause::builder(ActionType::Redact(RedactParams {
        fields: vec!["ssn".to_string(), "credit_card".to_string()],
        strategy: RedactionStrategy::Mask,
        redaction_template: Some("***".to_string()),
    }))
    .add_secondary(ActionType::Log(LogParams {
        level: LogLevel::Warning,
        message: "PII detected and redacted".to_string(),
        include_payload: false,
        structured_data: None,
    }))
    .build()
    .unwrap();
    println!("  ✓ Redacts SSN and credit cards, then logs the event");

    // Use Case 2: Rate Limiting with Logging
    println!("\nUse Case 2: API Rate Limiting");
    let _rate_limit_clause = ActionClause::builder(ActionType::RateLimit(RateLimitParams {
        max_requests: 1000,
        window: Duration::from_secs(3600),
        scope: RateLimitScope::PerAgent,
        action_on_exceed: Box::new(ActionType::Deny(DenyParams {
            reason: "Rate limit exceeded".to_string(),
            error_code: 429,
            http_status: Some(429),
        })),
    }))
    .build()
    .unwrap();
    println!("  ✓ 1000 requests per hour per agent, deny if exceeded");

    // Use Case 3: Content Moderation Pipeline
    println!("\nUse Case 3: Content Moderation Pipeline");
    let _moderation_clause = ActionClause::builder(ActionType::SpawnSidecar(
        SpawnSidecarParams {
            sidecar_spec: SidecarSpec {
                sidecar_type: "content-moderator".to_string(),
                image: "ml/content-mod:v2".to_string(),
                cpu_shares: 200,
                memory_limit_mb: 512,
                timeout: Duration::from_secs(30),
            },
            block_on_completion: true,
            pass_payload: true,
        },
    ))
    .add_secondary(ActionType::Callback(CallbackParams {
        endpoint: "https://control/moderation-results".to_string(),
        event_type: "moderation_complete".to_string(),
        include_payload: false,
        async_delivery: true,
    }))
    .build()
    .unwrap();
    println!("  ✓ Launch ML content moderator, wait for result, send callback");
    println!();

    println!("All examples completed successfully!");
    println!("\nKey Takeaways:");
    println!("   1. 11 atomic action types for different use cases");
    println!("   2. Type-safe parameters for each action");
    println!("   3. Explicit side effect management");
    println!("   4. Builder pattern for complex action clauses");
    println!("   5. Comprehensive validation and error handling");
}