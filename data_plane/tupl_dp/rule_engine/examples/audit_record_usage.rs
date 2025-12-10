// examples/audit_record_usage.rs
//
// Comprehensive example demonstrating the AuditRecord module
// Run with: cargo run --example audit_record_usage

use std::collections::HashMap;
use std::time::{Duration, SystemTime};

// Import the audit_record module
use rule_engine::audit_record::*;

/// Example 1: Basic Compact Decision Record
fn example_compact_decision_record() {
    println!("\n=== Example 1: Compact Decision Record ===");
    
    let record = CompactDecisionRecord::new(
        1,
        "rule_001".to_string(),
        1,
        "ALLOW".to_string(),
        vec![
            PayloadRef::new("shm_segment_1".to_string(), 0, 1024),
            PayloadRef::new("shm_segment_2".to_string(), 1024, 2048),
        ],
    );
    
    println!("Created compact record:");
    println!("  Sequence: {}", record.seq);
    println!("  Rule ID: {}", record.rule_id);
    println!("  Version: {}", record.rule_version);
    println!("  Decision: {}", record.decision);
    println!("  Timestamp: {}", record.timestamp_ms);
    println!("  Hash: {}", record.decision_hash);
    println!("  Payload refs: {}", record.payload_refs.len());
    
    // Verify hash integrity
    if record.verify_hash() {
        println!("✓ Hash verification passed");
    } else {
        println!("✗ Hash verification failed");
    }
}

/// Example 2: Full Audit Record with Builder
fn example_full_audit_record() {
    println!("\n=== Example 2: Full Audit Record ===");
    
    let record = AuditRecord::builder(1, "rule_security_001".to_string(), 2)
        .bundle_id("security_bundle_v1".to_string())
        .outcome(DecisionOutcome::Deny {
            reason: "Suspicious payload detected".to_string(),
            code: Some("SEC001".to_string()),
        })
        .log_level(AuditLogLevel::Critical)
        .explanation("Request denied due to failed signature verification".to_string())
        .add_metadata("tenant_id".to_string(), "tenant_123".to_string())
        .add_metadata("source_ip".to_string(), "192.168.1.100".to_string())
        .build()
        .unwrap();
    
    println!("Created full audit record:");
    println!("  Sequence: {}", record.seq);
    println!("  Rule: {} v{}", record.rule_id, record.rule_version);
    println!("  Bundle: {:?}", record.bundle_id);
    println!("  Outcome: {}", record.outcome.summary());
    println!("  Log Level: {:?}", record.log_level);
    println!("  Provenance Hash: {}", record.provenance_hash);
    println!("  Explanation: {:?}", record.explanation);
    println!("  Metadata: {} entries", record.metadata.len());
    
    // Verify provenance
    if record.verify_provenance() {
        println!("✓ Provenance verification passed");
    } else {
        println!("✗ Provenance verification failed");
    }
    
    // Print summary
    println!("  Summary: {}", record.summary());
}

/// Example 3: Decision Outcome Types
fn example_decision_outcomes() {
    println!("\n=== Example 3: Decision Outcome Types ===");
    
    let outcomes = vec![
        (
            "Allow",
            DecisionOutcome::Allow {
                metadata: Some(HashMap::from([
                    ("reason".to_string(), "passed all checks".to_string()),
                ])),
            },
        ),
        (
            "Deny",
            DecisionOutcome::Deny {
                reason: "Rate limit exceeded".to_string(),
                code: Some("RATE001".to_string()),
            },
        ),
        (
            "Rewrite",
            DecisionOutcome::Rewrite {
                transform_type: "normalize_json".to_string(),
            },
        ),
        (
            "Redact",
            DecisionOutcome::Redact {
                redacted_fields: vec![
                    "ssn".to_string(),
                    "credit_card".to_string(),
                    "password".to_string(),
                ],
            },
        ),
        (
            "Route",
            DecisionOutcome::Route {
                destination: "alternate_endpoint".to_string(),
            },
        ),
        (
            "SpawnSidecar",
            DecisionOutcome::SpawnSidecar {
                sidecar_type: "ml_analyzer".to_string(),
            },
        ),
        (
            "RateLimit",
            DecisionOutcome::RateLimit {
                scope: "per_user".to_string(),
                action: "throttle".to_string(),
            },
        ),
        (
            "ConstraintViolation",
            DecisionOutcome::ConstraintViolation {
                violation_type: "timeout_exceeded".to_string(),
                fail_open: false,
            },
        ),
        (
            "Error",
            DecisionOutcome::Error {
                message: "WASM execution failed".to_string(),
                code: "WASM001".to_string(),
            },
        ),
        ("Skip", DecisionOutcome::Skip),
    ];
    
    println!("Decision Outcome Analysis:");
    for (name, outcome) in outcomes {
        println!("\n  {}:", name);
        println!("    Summary: {}", outcome.summary());
        println!("    Is Blocking: {}", outcome.is_blocking());
        println!("    Is Modification: {}", outcome.is_modification());
    }
}

/// Example 4: Timestamp Tracking
fn example_timestamp_tracking() {
    println!("\n=== Example 4: Timestamp Tracking ===");
    
    let mut timestamps = EvaluationTimestamps::now();
    
    println!("Initial timestamps:");
    println!("  Received at: {:?}", timestamps.received_at);
    
    // Simulate some processing
    std::thread::sleep(Duration::from_millis(5));
    timestamps.eval_started_at = SystemTime::now();
    
    std::thread::sleep(Duration::from_millis(10));
    timestamps.eval_completed_at = SystemTime::now();
    
    std::thread::sleep(Duration::from_millis(2));
    timestamps.decision_at = SystemTime::now();
    
    timestamps.audit_created_at = SystemTime::now();
    
    println!("\nTiming analysis:");
    println!("  Eval time: {}μs", timestamps.total_eval_time_us());
    println!("  Total processing: {}μs", timestamps.total_processing_time_us());
    println!("  Received at (ms): {}", timestamps.received_at_millis());
}

/// Example 5: Audit Context Building
fn example_audit_context() {
    println!("\n=== Example 5: Audit Context ===");
    
    let exec_stats = ExecutionStatistics {
        eval_time_us: 1500,
        memory_used_bytes: 2048,
        cpu_time_us: 1200,
        rules_evaluated: 3,
        constraint_checks: 5,
    };
    
    let context = AuditContext::builder()
        .source_agent("api_gateway".to_string())
        .dest_agent("backend_service".to_string())
        .flow_id("flow_abc123".to_string())
        .payload_dtype("application/json".to_string())
        .enforcement_class("HARD".to_string())
        .add_violation("timeout_exceeded".to_string())
        .add_violation("memory_limit_exceeded".to_string())
        .exec_stats(exec_stats)
        .tenant_id("tenant_xyz".to_string())
        .request_id("req_456789".to_string())
        .build();
    
    println!("Audit Context:");
    println!("  Source: {:?}", context.source_agent);
    println!("  Destination: {:?}", context.dest_agent);
    println!("  Flow: {:?}", context.flow_id);
    println!("  Payload Type: {:?}", context.payload_dtype);
    println!("  Enforcement: {:?}", context.enforcement_class);
    println!("  Violations: {} detected", context.constraint_violations.len());
    for (i, v) in context.constraint_violations.iter().enumerate() {
        println!("    {}. {}", i + 1, v);
    }
    println!("  Tenant: {:?}", context.tenant_id);
    println!("  Request ID: {:?}", context.request_id);
    
    if let Some(stats) = context.exec_stats {
        println!("\n  Execution Stats:");
        println!("    Eval time: {}μs", stats.eval_time_us);
        println!("    Memory used: {} bytes", stats.memory_used_bytes);
        println!("    CPU time: {}μs", stats.cpu_time_us);
        println!("    Rules evaluated: {}", stats.rules_evaluated);
        println!("    Constraint checks: {}", stats.constraint_checks);
    }
}

/// Example 6: Audit Trail Management
fn example_audit_trail() {
    println!("\n=== Example 6: Audit Trail Management ===");
    
    let mut trail = AuditTrail::new(100);
    
    println!("Creating audit trail with max 100 in-memory records");
    
    // Add multiple records
    for i in 0..5 {
        let seq = trail.next_seq();
        let rule_id = if i % 2 == 0 {
            "rule_001".to_string()
        } else {
            "rule_002".to_string()
        };
        
        let outcome = if i % 3 == 0 {
            DecisionOutcome::Deny {
                reason: "Test denial".to_string(),
                code: None,
            }
        } else {
            DecisionOutcome::Allow { metadata: None }
        };
        
        let record = AuditRecord::new(seq, rule_id, 1, outcome);
        trail.add_record(record);
        
        // Simulate some time passing
        std::thread::sleep(Duration::from_millis(10));
    }
    
    println!("\nAdded 5 records to trail");
    println!("Total records: {}", trail.get_records().len());
    
    // Query by rule ID
    let rule_001_records = trail.get_records_by_rule("rule_001");
    println!("\nRecords for rule_001: {}", rule_001_records.len());
    for record in rule_001_records {
        println!("  Seq {}: {}", record.seq, record.outcome.summary());
    }
    
    let rule_002_records = trail.get_records_by_rule("rule_002");
    println!("\nRecords for rule_002: {}", rule_002_records.len());
    for record in rule_002_records {
        println!("  Seq {}: {}", record.seq, record.outcome.summary());
    }
    
    // Query by time range
    let now = SystemTime::now();
    let one_minute_ago = now - Duration::from_secs(60);
    let recent_records = trail.get_records_in_range(one_minute_ago, now);
    println!("\nRecords in last minute: {}", recent_records.len());
}

/// Example 7: Log Level Filtering
fn example_log_levels() {
    println!("\n=== Example 7: Log Level Filtering ===");
    
    let records = vec![
        (
            "Critical security event",
            AuditLogLevel::Critical,
            DecisionOutcome::Deny {
                reason: "Security violation".to_string(),
                code: Some("SEC001".to_string()),
            },
        ),
        (
            "Rate limit triggered",
            AuditLogLevel::High,
            DecisionOutcome::RateLimit {
                scope: "global".to_string(),
                action: "deny".to_string(),
            },
        ),
        (
            "Payload transformed",
            AuditLogLevel::Medium,
            DecisionOutcome::Rewrite {
                transform_type: "normalize".to_string(),
            },
        ),
        (
            "Request allowed",
            AuditLogLevel::Low,
            DecisionOutcome::Allow { metadata: None },
        ),
        (
            "Debug trace",
            AuditLogLevel::Trace,
            DecisionOutcome::Skip,
        ),
    ];
    
    let configured_levels = vec![
        AuditLogLevel::Critical,
        AuditLogLevel::High,
        AuditLogLevel::Medium,
        AuditLogLevel::Low,
        AuditLogLevel::Trace,
    ];
    
    for configured in configured_levels {
        println!("\nConfigured level: {:?}", configured);
        println!("  Records that would be logged:");
        
        for (desc, level, outcome) in &records {
            if level.should_log(configured) {
                println!("    ✓ [{}] {}: {}", 
                    match level {
                        AuditLogLevel::Critical => "CRIT",
                        AuditLogLevel::High => "HIGH",
                        AuditLogLevel::Medium => "MED ",
                        AuditLogLevel::Low => "LOW ",
                        AuditLogLevel::Trace => "TRAC",
                    },
                    desc,
                    outcome.summary()
                );
            }
        }
    }
}

/// Example 8: Provenance Verification and Tamper Detection
fn example_provenance_verification() {
    println!("\n=== Example 8: Provenance Verification ===");
    
    let mut record = AuditRecord::new(
        1,
        "rule_001".to_string(),
        1,
        DecisionOutcome::Allow { metadata: None },
    );
    
    println!("Original record:");
    println!("  Rule: {}", record.rule_id);
    println!("  Provenance hash: {}", record.provenance_hash);
    println!("  Verification: {}", record.verify_provenance());
    
    // Simulate tampering
    println!("\n⚠️  Simulating tampering...");
    record.rule_id = "rule_002_tampered".to_string();
    
    println!("\nAfter tampering:");
    println!("  Rule: {}", record.rule_id);
    println!("  Provenance hash (unchanged): {}", record.provenance_hash);
    println!("  Verification: {}", record.verify_provenance());
    
    if !record.verify_provenance() {
        println!("\n✗ TAMPER DETECTED! Record has been modified.");
    }
}

/// Example 9: Compact to Full Conversion
fn example_compact_to_full_conversion() {
    println!("\n=== Example 9: Compact to Full Conversion ===");
    
    // Create compact record (fast path)
    let compact = CompactDecisionRecord::new(
        1,
        "rule_001".to_string(),
        1,
        "ALLOW".to_string(),
        vec![PayloadRef::new("shm_1".to_string(), 0, 1024)],
    );
    
    println!("Compact record created (fast path)");
    println!("  Size: ~100 bytes");
    println!("  Hash: {}", compact.decision_hash);
    
    // Later, convert to full record for detailed analysis
    let outcome = DecisionOutcome::Allow {
        metadata: Some(HashMap::from([
            ("cache_hit".to_string(), "true".to_string()),
        ])),
    };
    
    let timestamps = EvaluationTimestamps::now();
    
    let context = AuditContext::builder()
        .source_agent("agent_1".to_string())
        .flow_id("flow_123".to_string())
        .build();
    
    let full = compact.to_full_record(outcome, timestamps, context);
    
    println!("\nConverted to full record:");
    println!("  Sequence: {}", full.seq);
    println!("  Rule: {}", full.rule_id);
    println!("  Outcome: {}", full.outcome.summary());
    println!("  Context fields populated: {}", 
        full.context.source_agent.is_some() as i32 +
        full.context.flow_id.is_some() as i32
    );
}

/// Example 10: Real-World Audit Scenario
fn example_real_world_scenario() {
    println!("\n=== Example 10: Real-World Audit Scenario ===");
    println!("Simulating API request processing with full audit trail\n");
    
    let mut trail = AuditTrail::new(1000);
    
    // Request arrives
    let received_at = SystemTime::now();
    println!("1. Request received from client");
    
    // Start evaluation
    std::thread::sleep(Duration::from_millis(2));
    let eval_started = SystemTime::now();
    println!("2. Starting rule evaluation");
    
    // Multiple rules evaluated
    let rules_to_check = vec![
        ("rate_limit_check", AuditLogLevel::Medium, true),
        ("authentication_check", AuditLogLevel::High, true),
        ("authorization_check", AuditLogLevel::High, false), // This one denies
    ];
    
    for (i, (rule_name, level, passes)) in rules_to_check.iter().enumerate() {
        std::thread::sleep(Duration::from_millis(5));
        
        let seq = trail.next_seq();
        let outcome = if *passes {
            DecisionOutcome::Allow { metadata: None }
        } else {
            DecisionOutcome::Deny {
                reason: "Insufficient permissions".to_string(),
                code: Some("AUTH002".to_string()),
            }
        };
        
        let mut timestamps = EvaluationTimestamps::now();
        timestamps.received_at = received_at;
        timestamps.eval_started_at = eval_started;
        
        let context = AuditContext::builder()
            .source_agent("api_gateway".to_string())
            .dest_agent("user_service".to_string())
            .flow_id("req_abc123".to_string())
            .request_id(format!("req_{}_{}", seq, rule_name))
            .exec_stats(ExecutionStatistics {
                eval_time_us: 5000 * (i as u64 + 1),
                memory_used_bytes: 1024,
                cpu_time_us: 4000,
                rules_evaluated: i as u32 + 1,
                constraint_checks: (i as u32 + 1) * 2,
            })
            .build();
        
        let record = AuditRecord::builder(seq, rule_name.to_string(), 1)
            .outcome(outcome.clone())
            .timestamps(timestamps)
            .context(context)
            .log_level(*level)
            .explanation(format!("Rule {} evaluation", rule_name))
            .build()
            .unwrap();
        
        println!("   {}. Rule '{}': {}", 
            i + 1, 
            rule_name, 
            outcome.summary()
        );
        
        trail.add_record(record);
        
        // If denied, stop processing
        if !passes {
            println!("\n   ⛔ Request denied, stopping evaluation");
            break;
        }
    }
    
    // Final summary
    println!("\n3. Audit trail summary:");
    println!("   Total records: {}", trail.get_records().len());
    
    let denials: Vec<_> = trail
        .get_records()
        .iter()
        .filter(|r| r.outcome.is_blocking())
        .collect();
    println!("   Denials: {}", denials.len());
    
    let allows: Vec<_> = trail
        .get_records()
        .iter()
        .filter(|r| matches!(r.outcome, DecisionOutcome::Allow { .. }))
        .collect();
    println!("   Allows: {}", allows.len());
    
    // Verify all records
    println!("\n4. Provenance verification:");
    let mut all_valid = true;
    for record in trail.get_records() {
        if !record.verify_provenance() {
            println!("   ✗ Record {} failed verification", record.seq);
            all_valid = false;
        }
    }
    if all_valid {
        println!("   ✓ All records verified successfully");
    }
    
    // Print detailed audit log
    println!("\n5. Detailed audit log:");
    for record in trail.get_records() {
        println!("\n   Seq {}: {} v{}", record.seq, record.rule_id, record.rule_version);
        println!("     Outcome: {}", record.outcome.summary());
        println!("     Eval time: {}μs", record.timestamps.total_eval_time_us());
        println!("     Provenance: {}", &record.provenance_hash[..16]);
        if let Some(stats) = &record.context.exec_stats {
            println!("     Rules evaluated: {}", stats.rules_evaluated);
        }
    }
}

/// Example 11: Performance Benchmarking
fn example_performance_benchmark() {
    println!("\n=== Example 11: Performance Benchmark ===");
    
    let iterations = 10000;
    
    println!("Benchmarking audit record operations ({} iterations):", iterations);
    
    // Benchmark: Compact record creation
    let start = std::time::Instant::now();
    for i in 0..iterations {
        let _record = CompactDecisionRecord::new(
            i,
            "rule_001".to_string(),
            1,
            "ALLOW".to_string(),
            vec![],
        );
    }
    let compact_time = start.elapsed();
    
    println!("\n  Compact record creation:");
    println!("    Total time: {:?}", compact_time);
    println!("    Per record: {:?}", compact_time / iterations as u32);
    
    // Benchmark: Full record creation
    let start = std::time::Instant::now();
    for i in 0..iterations {
        let _record = AuditRecord::builder(i, "rule_001".to_string(), 1)
            .outcome(DecisionOutcome::Allow { metadata: None })
            .build()
            .unwrap();
    }
    let full_time = start.elapsed();
    
    println!("\n  Full record creation:");
    println!("    Total time: {:?}", full_time);
    println!("    Per record: {:?}", full_time / iterations as u32);
    
    // Benchmark: Hash verification
    let record = CompactDecisionRecord::new(
        1,
        "rule_001".to_string(),
        1,
        "ALLOW".to_string(),
        vec![],
    );
    
    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let _ = record.verify_hash();
    }
    let verify_time = start.elapsed();
    
    println!("\n  Hash verification:");
    println!("    Total time: {:?}", verify_time);
    println!("    Per verification: {:?}", verify_time / iterations as u32);
}

fn main() {
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║            AuditRecord Module Examples                     ║");
    println!("╚════════════════════════════════════════════════════════════╝");
    
    // Run all examples
    example_compact_decision_record();
    example_full_audit_record();
    example_decision_outcomes();
    example_timestamp_tracking();
    example_audit_context();
    example_audit_trail();
    example_log_levels();
    example_provenance_verification();
    example_compact_to_full_conversion();
    example_real_world_scenario();
    example_performance_benchmark();
    
    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║                 All Examples Completed                     ║");
    println!("╚════════════════════════════════════════════════════════════╝");
}

#[cfg(test)]
mod example_tests {
    use super::*;
    
    #[test]
    fn test_all_examples_run() {
        // Ensure examples don't panic
        example_compact_decision_record();
        example_full_audit_record();
        example_decision_outcomes();
        example_timestamp_tracking();
        example_audit_context();
        example_audit_trail();
        example_log_levels();
        example_provenance_verification();
        example_compact_to_full_conversion();
        // Skip real_world_scenario and performance_benchmark in tests
    }
}