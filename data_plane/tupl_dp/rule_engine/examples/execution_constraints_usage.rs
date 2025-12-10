// examples/execution_constraints_usage.rs
//
// Comprehensive example demonstrating the ExecutionConstraints module
// Run with: cargo run --example execution_constraints_usage

use std::time::Duration;
use std::thread;

// Import the execution constraints module
use rule_engine::{
    ConstraintEnforcer, ConstraintError, ConstraintViolationType, ExecutionBudget,
    ExecutionConstraints, ExecutionStats, RetryPolicy, RuleType,
};

/// Simulates a rule evaluation operation
fn simulate_rule_evaluation(duration_ms: u64) -> Result<String, ConstraintError> {
    thread::sleep(Duration::from_millis(duration_ms));
    Ok(format!("Rule evaluated in {}ms", duration_ms))
}

/// Simulates a memory-intensive operation
fn simulate_memory_operation(memory_bytes: u64) -> Result<(), ConstraintError> {
    // In real code, this would allocate and use memory
    println!("Simulating memory usage: {} bytes", memory_bytes);
    Ok(())
}

/// Example 1: Basic usage with fast rules
fn example_fast_rule_evaluation() {
    println!("\n=== Example 1: Fast Rule Evaluation ===");
    
    let constraints = ExecutionConstraints::fast_rule();
    let mut budget = constraints.create_budget();
    
    println!("Max allowed time: {}ms", constraints.max_exec_ms);
    
    // Execute a fast operation (should succeed)
    let result = budget.enforce(|| {
        simulate_rule_evaluation(2)
    });
    
    match result {
        Ok(msg) => {
            println!("✓ Success: {}", msg);
            let stats = budget.stats();
            println!("  Elapsed: {}ms", stats.elapsed_ms);
        }
        Err(e) => println!("✗ Failed: {:?}", e),
    }
    
    // Try a slow operation (should timeout)
    let mut budget = constraints.create_budget();
    let result = budget.enforce(|| {
        simulate_rule_evaluation(10) // Exceeds 5ms limit
    });
    
    match result {
        Ok(_) => println!("✓ Success (unexpected)"),
        Err(e) => println!("✗ Expected timeout: {:?}", e),
    }
}

/// Example 2: Semantic rule with retries
fn example_semantic_rule_with_retries() {
    println!("\n=== Example 2: Semantic Rule with Retries ===");
    
    let constraints = ExecutionConstraints::semantic_rule();
    let mut budget = constraints.create_budget();
    
    println!("Max allowed time: {}ms", constraints.max_exec_ms);
    println!("Retries allowed: {:?}", constraints.max_retries);
    
    // Simulate operation that might need retries
    let mut attempt = 0;
    let result = budget.enforce(|| {
        attempt += 1;
        println!("  Attempt {}", attempt);
        simulate_rule_evaluation(30)
    });
    
    match result {
        Ok(msg) => {
            println!("✓ Success: {}", msg);
            let stats = budget.stats();
            println!("  Total time: {}ms", stats.elapsed_ms);
        }
        Err(e) => println!("✗ Failed: {:?}", e),
    }
}

/// Example 3: WASM hook with custom constraints
fn example_wasm_hook_execution() {
    println!("\n=== Example 3: WASM Hook Execution ===");
    
    // Custom WASM hook with 20ms timeout
    let constraints = ExecutionConstraints::wasm_hook(20);
    let mut budget = constraints.create_budget();
    
    println!("WASM hook timeout: {}ms", constraints.max_exec_ms);
    println!("Memory limit: {:?} bytes", constraints.memory_limit_bytes);
    
    // Simulate WASM execution with memory tracking
    let result = budget.enforce(|| {
        simulate_rule_evaluation(15)
    });

    // Record memory usage after execution
    budget.record_memory_usage(2 * 1024 * 1024); // 2 MB
    
    match result {
        Ok(msg) => {
            println!("✓ WASM execution succeeded: {}", msg);
            let stats = budget.stats();
            println!("  Memory used: {} bytes", stats.memory_used_bytes);
            println!("  Elapsed: {}ms", stats.elapsed_ms);
        }
        Err(e) => println!("✗ WASM execution failed: {:?}", e),
    }
    
    // Test memory violation
    println!("\nTesting memory limit violation:");
    let mut budget = constraints.create_budget();

    // Record excessive memory usage
    let memory_used = 10 * 1024 * 1024; // 10 MB (exceeds 5MB limit)
    budget.record_memory_usage(memory_used);

    let result = budget.enforce(|| {
        simulate_memory_operation(memory_used)?;
        Ok("Should not reach here".to_string())
    });
    
    match result {
        Ok(_) => println!("✓ Success (unexpected)"),
        Err(e) => println!("✗ Expected memory violation: {:?}", e),
    }
}

/// Example 4: Observational rules with sampling
fn example_observational_with_sampling() {
    println!("\n=== Example 4: Observational Rules with Sampling ===");
    
    // Only execute 30% of the time
    let sampling_rates = vec![0.0, 0.3, 0.5, 1.0];
    
    for rate in sampling_rates {
        let constraints = ExecutionConstraints::observational(rate);
        
        println!("\nSampling rate: {:.1}%", rate * 100.0);
        
        let mut executed = 0;
        let mut sampled_out = 0;
        let trials = 100;
        
        for _ in 0..trials {
            let mut budget = constraints.create_budget();
            
            let result = budget.enforce(|| {
                Ok("Log entry created")
            });
            
            match result {
                Ok(_) => executed += 1,
                Err(ConstraintError::Violation(
                    ConstraintViolationType::SampledOut { .. }
                )) => sampled_out += 1,
                Err(e) => println!("  Unexpected error: {:?}", e),
            }
        }
        
        println!("  Executed: {} / {}", executed, trials);
        println!("  Sampled out: {} / {}", sampled_out, trials);
        println!("  Actual rate: {:.2}%", (executed as f64 / trials as f64) * 100.0);
    }
}

/// Example 5: Using ConstraintEnforcer for different rule types
fn example_constraint_enforcer() {
    println!("\n=== Example 5: ConstraintEnforcer for Different Rule Types ===");
    
    let enforcer = ConstraintEnforcer::new();
    
    // Fast rule
    println!("\nEvaluating fast rule:");
    let result = enforcer.execute_with_constraints(
        RuleType::Fast,
        || simulate_rule_evaluation(3)
    );
    println!("  Result: {:?}", result);
    
    // Semantic rule
    println!("\nEvaluating semantic rule:");
    let result = enforcer.execute_with_constraints(
        RuleType::Semantic,
        || simulate_rule_evaluation(80)
    );
    println!("  Result: {:?}", result);
    
    // WASM hook
    println!("\nEvaluating WASM hook:");
    let result = enforcer.execute_with_constraints(
        RuleType::WasmHook,
        || simulate_rule_evaluation(8)
    );
    println!("  Result: {:?}", result);
    
    // Observational rule
    println!("\nEvaluating observational rule:");
    let result = enforcer.execute_with_constraints(
        RuleType::Observational,
        || Ok("Metric recorded".to_string())
    );
    println!("  Result: {:?}", result);
}

/// Example 6: Custom constraints and validation
fn example_custom_constraints() {
    println!("\n=== Example 6: Custom Constraints ===");
    
    let custom = ExecutionConstraints {
        max_exec_ms: 50,
        cpu_shares: Some(25),
        memory_limit_bytes: Some(2 * 1024 * 1024), // 2 MB
        sampling_rate: 0.8,
        fail_closed_on_timeout: true,
        max_retries: Some(2),
        retry_backoff_ms: Some(5),
    };
    
    match custom.validate() {
        Ok(_) => {
            println!("✓ Custom constraints are valid");
            let mut budget = custom.create_budget();
            
            let result = budget.enforce(|| {
                simulate_rule_evaluation(30)
            });
            
            let stats = budget.stats();
            println!("  Execution stats:");
            println!("    - Elapsed: {}ms", stats.elapsed_ms);
            println!("    - Timeout: {}", stats.timeout_occurred);
            println!("    - Violations: {}", stats.violation_count);
        }
        Err(e) => println!("✗ Invalid constraints: {:?}", e),
    }
    
    // Invalid constraint example
    println!("\nTesting invalid constraints:");
    let invalid = ExecutionConstraints {
        max_exec_ms: 0, // Invalid!
        cpu_shares: Some(150), // Invalid!
        memory_limit_bytes: None,
        sampling_rate: 1.5, // Invalid!
        fail_closed_on_timeout: true,
        max_retries: None,
        retry_backoff_ms: None,
    };
    
    match invalid.validate() {
        Ok(_) => println!("✓ Valid (unexpected)"),
        Err(e) => println!("✗ Expected validation error: {:?}", e),
    }
}

/// Example 7: Retry policy demonstration
fn example_retry_policy() {
    println!("\n=== Example 7: Retry Policy ===");
    
    // Fixed backoff policy
    let policy = RetryPolicy::new(3, 10);
    
    println!("Testing retry with fixed backoff (3 attempts, 10ms):");
    let mut attempt_count = 0;
    
    let result = policy.execute(|| {
        attempt_count += 1;
        println!("  Attempt {}", attempt_count);
        
        if attempt_count < 3 {
            Err("Simulated failure")
        } else {
            Ok("Success!")
        }
    });
    
    match result {
        Ok(msg) => println!("✓ Retry succeeded: {}", msg),
        Err(e) => println!("✗ All retries failed: {}", e),
    }
    
    // Exponential backoff policy
    println!("\nTesting retry with exponential backoff:");
    let policy = RetryPolicy::new(4, 5).with_exponential_backoff();
    
    let mut attempt_count = 0;
    let start = std::time::Instant::now();
    
    let _result = policy.execute(|| {
        attempt_count += 1;
        let elapsed = start.elapsed().as_millis();
        println!("  Attempt {} at {}ms", attempt_count, elapsed);
        
        if attempt_count < 4 {
            Err("Simulated failure")
        } else {
            Ok("Success!")
        }
    });
    
    println!("  Total time: {}ms", start.elapsed().as_millis());
}

/// Example 8: Budget monitoring and violation tracking
fn example_budget_monitoring() {
    println!("\n=== Example 8: Budget Monitoring ===");
    
    let constraints = ExecutionConstraints {
        max_exec_ms: 20,
        cpu_shares: Some(30),
        memory_limit_bytes: Some(5 * 1024 * 1024),
        sampling_rate: 1.0,
        fail_closed_on_timeout: true,
        max_retries: None,
        retry_backoff_ms: None,
    };
    
    let mut budget = constraints.create_budget();
    
    println!("Starting budget monitoring...");
    println!("Initial remaining budget: {}ms", budget.remaining_ms());
    
    // Simulate incremental work
    for i in 1..=5 {
        thread::sleep(Duration::from_millis(3));
        
        println!("\nCheckpoint {}:", i);
        println!("  Elapsed: {}ms", budget.elapsed_ms());
        println!("  Remaining: {}ms", budget.remaining_ms());
        
        if budget.is_timeout() {
            println!("  ⚠ TIMEOUT DETECTED!");
            break;
        }
        
        // Check constraints periodically
        if let Err(e) = budget.check() {
            println!("  ✗ Constraint violation: {:?}", e);
            break;
        }
    }
    
    // Final statistics
    let stats = budget.stats();
    println!("\nFinal statistics:");
    println!("  Total elapsed: {}ms", stats.elapsed_ms);
    println!("  Timeout occurred: {}", stats.timeout_occurred);
    println!("  Violations: {}", stats.violation_count);
    
    let violations = budget.get_violations();
    if !violations.is_empty() {
        println!("\nRecorded violations:");
        for (i, v) in violations.iter().enumerate() {
            println!("  {}. {:?}", i + 1, v);
        }
    }
}

/// Example 9: Real-world rule evaluation pipeline
fn example_rule_evaluation_pipeline() {
    println!("\n=== Example 9: Rule Evaluation Pipeline ===");
    
    struct Rule {
        id: String,
        rule_type: RuleType,
        complexity: &'static str,
    }
    
    let rules = vec![
        Rule {
            id: "rule_001".to_string(),
            rule_type: RuleType::Fast,
            complexity: "simple header check",
        },
        Rule {
            id: "rule_002".to_string(),
            rule_type: RuleType::Semantic,
            complexity: "ML inference",
        },
        Rule {
            id: "rule_003".to_string(),
            rule_type: RuleType::WasmHook,
            complexity: "custom WASM validation",
        },
        Rule {
            id: "rule_004".to_string(),
            rule_type: RuleType::Observational,
            complexity: "metrics collection",
        },
    ];
    
    let enforcer = ConstraintEnforcer::new();
    
    for rule in rules {
        println!("\nEvaluating {} ({})", rule.id, rule.complexity);
        
        let result = enforcer.execute_with_constraints(
            rule.rule_type,
            || {
                // Simulate different execution times based on complexity
                let duration = match rule.rule_type {
                    RuleType::Fast => 2,
                    RuleType::Semantic => 60,
                    RuleType::WasmHook => 8,
                    RuleType::Observational => 1,
                };
                
                simulate_rule_evaluation(duration)
            }
        );
        
        match result {
            Ok(msg) => println!("  ✓ {}", msg),
            Err(e) => println!("  ✗ Error: {:?}", e),
        }
    }
}

/// Example 10: Performance benchmarking
fn example_performance_benchmark() {
    println!("\n=== Example 10: Performance Benchmark ===");
    
    let iterations = 10000;
    
    println!("Benchmarking constraint overhead ({} iterations):", iterations);
    
    // Benchmark: No constraints
    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let _ = simulate_rule_evaluation(0);
    }
    let no_constraint_time = start.elapsed();
    
    // Benchmark: With constraints
    let constraints = ExecutionConstraints::fast_rule();
    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let mut budget = constraints.create_budget();
        let _ = budget.enforce(|| simulate_rule_evaluation(0));
    }
    let with_constraint_time = start.elapsed();
    
    println!("\nResults:");
    println!("  Without constraints: {:?}", no_constraint_time);
    println!("  With constraints:    {:?}", with_constraint_time);
    println!("  Overhead per call:   {:?}", 
        (with_constraint_time - no_constraint_time) / iterations);
}

fn main() {
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║        ExecutionConstraints Module Examples               ║");
    println!("╚════════════════════════════════════════════════════════════╝");
    
    // Run all examples
    example_fast_rule_evaluation();
    example_semantic_rule_with_retries();
    example_wasm_hook_execution();
    example_observational_with_sampling();
    example_constraint_enforcer();
    example_custom_constraints();
    example_retry_policy();
    example_budget_monitoring();
    example_rule_evaluation_pipeline();
    example_performance_benchmark();
    
    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║                 All Examples Completed                     ║");
    println!("╚════════════════════════════════════════════════════════════╝");
}

#[cfg(test)]
mod example_tests {
    use super::*;
    
    #[test]
    fn test_example_runs() {
        // Ensure examples don't panic
        example_fast_rule_evaluation();
        example_semantic_rule_with_retries();
        example_wasm_hook_execution();
        example_observational_with_sampling();
        example_constraint_enforcer();
        example_custom_constraints();
        example_retry_policy();
        example_budget_monitoring();
        example_rule_evaluation_pipeline();
    }
}