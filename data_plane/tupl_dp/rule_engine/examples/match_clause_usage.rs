//
// This example demonstrates the complete usage of the MatchClause module,
// including all three evaluation tiers: FastMatch, MatchExpression, and WasmHook.
//
// Run with: cargo run --example match_clause_usage

use rule_engine::match_clause::{
    ComparisonOp, EventContext, FastMatch, FastMatchBuilder, FieldComparison, FieldValue, HeaderFlags,
    JsonPathQuery, MatchClause, MatchExpression, PayloadData, RegexMatch, WasmHookRef,
};
use rule_engine::AgentId;
use std::collections::HashMap;
use std::time::Duration;

fn main() {
    println!("=== Match Clause - Comprehensive Examples ===\n");

    // ========================================================================
    // Example 1: Fast Match - O(1) Predicates
    // ========================================================================
    println!("Example 1: FastMatch Evaluation");
    println!("-----------------------------------");

    let fast_match = FastMatchBuilder::new()
        .add_source_agent(AgentId::new("gpt-4"))
        .add_source_agent(AgentId::new("claude-3"))
        .add_payload_type("application/json")
        .build();

    // Create event context that matches
    let mut ctx = EventContext::new(
        AgentId::new("gpt-4"),
        None,
        None,
        "application/json".to_string(),
        HeaderFlags::empty(),
        HashMap::new(),
    );
    println!("Event from GPT-4 with JSON payload:");
    println!("  Matches: {}", fast_match.evaluate(&ctx));

    // Create event context that doesn't match
    ctx.source_agent = AgentId::new("llama-2");
    println!("Event from Llama-2 with JSON payload:");
    println!("  Matches: {}", fast_match.evaluate(&ctx));
    println!();

    // ========================================================================
    // Example 2: Header Flags - Bitset Operations
    // ========================================================================
    println!("Example 2: Header Flags");
    println!("--------------------------");

    let mut flags = HeaderFlags::empty();
    println!("Empty flags: {:064b}", flags.bits());

    flags.set(HeaderFlags::ENCRYPTED);
    flags.set(HeaderFlags::AUTHENTICATED);
    println!("After setting ENCRYPTED and AUTHENTICATED: {:064b}", flags.bits());
    println!("  Has ENCRYPTED: {}", flags.has(HeaderFlags::ENCRYPTED));
    println!("  Has AUTHENTICATED: {}", flags.has(HeaderFlags::AUTHENTICATED));
    println!("  Has PII: {}", flags.has(HeaderFlags::CONTAINS_PII));

    // FastMatch with required flags
    let fast_match_secure = FastMatchBuilder::new()
        .require_flags(HeaderFlags::from_bits(
            HeaderFlags::ENCRYPTED | HeaderFlags::AUTHENTICATED,
        ))
        .build();

    let mut ctx = EventContext::new(
        AgentId::new("test"),
        None,
        None,
        "text/plain".to_string(),
        HeaderFlags::empty(),
        HashMap::new(),
    );
    println!("\nEvent without security flags:");
    println!("  Matches secure rule: {}", fast_match_secure.evaluate(&ctx));

    ctx.header_flags.set(HeaderFlags::ENCRYPTED);
    ctx.header_flags.set(HeaderFlags::AUTHENTICATED);
    println!("Event with security flags:");
    println!("  Matches secure rule: {}", fast_match_secure.evaluate(&ctx));
    println!();

    // ========================================================================
    // Example 3: Field Comparisons
    // ========================================================================
    println!("Example 3: Field Comparisons");
    println!("--------------------------------");

    let mut ctx = EventContext::new(
        AgentId::new("test"),
        None,
        None,
        "application/json".to_string(),
        HeaderFlags::empty(),
        HashMap::new(),
    );
    ctx.set_header("severity".to_string(), FieldValue::String("critical".to_string()));
    ctx.set_header("priority".to_string(), FieldValue::Integer(10));

    // String equality comparison
    let severity_check = FieldComparison {
        field_path: "severity".to_string(),
        operator: ComparisonOp::Equal,
        value: FieldValue::String("critical".to_string()),
    };

    println!("Checking if severity == 'critical':");
    println!("  Result: {}", severity_check.evaluate(&ctx, None));

    // Integer comparison
    let priority_check = FieldComparison {
        field_path: "priority".to_string(),
        operator: ComparisonOp::GreaterThan,
        value: FieldValue::Integer(5),
    };

    println!("Checking if priority > 5:");
    println!("  Result: {}", priority_check.evaluate(&ctx, None));

    // String contains
    ctx.set_header("message".to_string(), FieldValue::String("Error: database connection failed".to_string()));
    let contains_check = FieldComparison {
        field_path: "message".to_string(),
        operator: ComparisonOp::Contains,
        value: FieldValue::String("database".to_string()),
    };

    println!("Checking if message contains 'database':");
    println!("  Result: {}", contains_check.evaluate(&ctx, None));
    println!();

    // ========================================================================
    // Example 4: Match Expressions - Logical Operators
    // ========================================================================
    println!("Example 4: Match Expressions with Logic");
    println!("------------------------------------------");

    // AND: Both conditions must be true
    let and_expr = MatchExpression::And(vec![
        MatchExpression::Field(FieldComparison {
            field_path: "severity".to_string(),
            operator: ComparisonOp::Equal,
            value: FieldValue::String("critical".to_string()),
        }),
        MatchExpression::Field(FieldComparison {
            field_path: "priority".to_string(),
            operator: ComparisonOp::GreaterThan,
            value: FieldValue::Integer(5),
        }),
    ]);

    println!("AND expression (severity='critical' AND priority>5):");
    println!("  Result: {}", and_expr.evaluate(&ctx, None));

    // OR: At least one condition must be true
    let or_expr = MatchExpression::Or(vec![
        MatchExpression::Field(FieldComparison {
            field_path: "severity".to_string(),
            operator: ComparisonOp::Equal,
            value: FieldValue::String("low".to_string()),
        }),
        MatchExpression::Field(FieldComparison {
            field_path: "priority".to_string(),
            operator: ComparisonOp::GreaterThan,
            value: FieldValue::Integer(5),
        }),
    ]);

    println!("OR expression (severity='low' OR priority>5):");
    println!("  Result: {}", or_expr.evaluate(&ctx, None));

    // NOT: Inverts the result
    let not_expr = MatchExpression::Not(Box::new(MatchExpression::Field(FieldComparison {
        field_path: "severity".to_string(),
        operator: ComparisonOp::Equal,
        value: FieldValue::String("low".to_string()),
    })));

    println!("NOT expression (severity != 'low'):");
    println!("  Result: {}", not_expr.evaluate(&ctx, None));
    println!();

    // ========================================================================
    // Example 5: Complex Nested Expressions
    // ========================================================================
    println!("Example 5: Complex Nested Expressions");
    println!("----------------------------------------");

    // (severity='critical' OR priority>8) AND NOT (message contains 'test')
    let complex_expr = MatchExpression::And(vec![
        MatchExpression::Or(vec![
            MatchExpression::Field(FieldComparison {
                field_path: "severity".to_string(),
                operator: ComparisonOp::Equal,
                value: FieldValue::String("critical".to_string()),
            }),
            MatchExpression::Field(FieldComparison {
                field_path: "priority".to_string(),
                operator: ComparisonOp::GreaterThan,
                value: FieldValue::Integer(8),
            }),
        ]),
        MatchExpression::Not(Box::new(MatchExpression::Field(FieldComparison {
            field_path: "message".to_string(),
            operator: ComparisonOp::Contains,
            value: FieldValue::String("test".to_string()),
        }))),
    ]);

    println!("Complex expression:");
    println!("  (severity='critical' OR priority>8) AND NOT (message contains 'test')");
    println!("  Result: {}", complex_expr.evaluate(&ctx, None));
    println!();

    // ========================================================================
    // Example 6: Regex Matching
    // ========================================================================
    println!("Example 6: Regex Pattern Matching");
    println!("------------------------------------");

    let mut ctx = EventContext::new(
        AgentId::new("test"),
        None,
        None,
        "text/plain".to_string(),
        HeaderFlags::empty(),
        HashMap::new(),
    );
    ctx.set_header("email".to_string(), FieldValue::String("user@example.com".to_string()));

    let regex_match = RegexMatch {
        field_path: "email".to_string(),
        pattern: "@example.com".to_string(), // Simplified - would use regex in production
        full_match: false,
    };

    println!("Checking if email contains '@example.com':");
    println!("  Result: {}", regex_match.evaluate(&ctx, None));

    let regex_expr = MatchExpression::Regex(regex_match);
    println!("Using MatchExpression::Regex:");
    println!("  Result: {}", regex_expr.evaluate(&ctx, None));
    println!();

    // ========================================================================
    // Example 7: JSONPath Queries
    // ========================================================================
    println!("Example 7: JSONPath Queries");
    println!("-------------------------------");

    let jsonpath = JsonPathQuery {
        path: "$.user.email".to_string(),
        expected_value: Some(FieldValue::String("admin@company.com".to_string())),
        exists_only: false,
    };

    let mut fields = HashMap::new();
    fields.insert(
        "$.user.email".to_string(),
        FieldValue::String("admin@company.com".to_string()),
    );
    let payload = PayloadData::new(Vec::new(), fields);

    let jsonpath_expr = MatchExpression::JsonPath(jsonpath);
    println!("Checking if $.user.email == 'admin@company.com':");
    println!("  Result: {}", jsonpath_expr.evaluate(&ctx, Some(&payload)));

    // Existence check
    let exists_query = JsonPathQuery {
        path: "$.user.email".to_string(),
        expected_value: None,
        exists_only: true,
    };

    let exists_expr = MatchExpression::JsonPath(exists_query);
    println!("Checking if $.user.email exists:");
    println!("  Result: {}", exists_expr.evaluate(&ctx, Some(&payload)));
    println!();

    // ========================================================================
    // Example 8: WASM Hook Reference
    // ========================================================================
    println!("Example 8: WASM Hook Configuration");
    println!("-------------------------------------");

    let wasm_hook = WasmHookRef {
        hook_id: "sentiment-analyzer".to_string(),
        module_digest: "sha256:abcd1234567890...".to_string(),
        max_exec_time: Duration::from_millis(50),
        memory_limit_bytes: 10 * 1024 * 1024, // 10 MB
        cpu_shares: 100,
    };

    println!("WASM Hook Configuration:");
    println!("  Hook ID: {}", wasm_hook.hook_id);
    println!("  Module Digest: {}", wasm_hook.module_digest);
    println!("  Max Execution Time: {:?}", wasm_hook.max_exec_time);
    println!("  Memory Limit: {} MB", wasm_hook.memory_limit_bytes / (1024 * 1024));
    println!("  CPU Shares: {}", wasm_hook.cpu_shares);
    println!();

    // ========================================================================
    // Example 9: Complete MatchClause - All Three Tiers
    // ========================================================================
    println!("Example 9: Complete MatchClause Evaluation");
    println!("---------------------------------------------");

    let fast_match = FastMatchBuilder::new()
        .add_source_agent(AgentId::new("gpt-4"))
        .add_payload_type("application/json")
        .build();

    let match_expr = MatchExpression::Field(FieldComparison {
        field_path: "severity".to_string(),
        operator: ComparisonOp::Equal,
        value: FieldValue::String("critical".to_string()),
    });

    let wasm_hook = WasmHookRef::new(
        "security-validator".to_string(),
        "sha256:secure123".to_string(),
    );

    let complete_clause = MatchClause::complete(fast_match, match_expr, wasm_hook);

    let mut ctx = EventContext::new(
        AgentId::new("gpt-4"),
        None,
        None,
        "application/json".to_string(),
        HeaderFlags::empty(),
        HashMap::new(),
    );
    ctx.set_header("severity".to_string(), FieldValue::String("critical".to_string()));

    println!("Evaluating complete MatchClause with all three tiers:");
    let result = complete_clause.evaluate(&ctx, None);
    println!("  Matched: {}", result.is_match);
    println!("  Decided by: {:?}", result.tier);
    println!("  Max tier: {:?}", complete_clause.max_evaluation_tier());
    println!("  Requires payload: {}", complete_clause.requires_payload());
    println!();

    // ========================================================================
    // Example 10: Evaluation Flow - Early Termination
    // ========================================================================
    println!("⚡ Example 10: Evaluation Flow with Early Termination");
    println!("----------------------------------------------------");

    // Create a clause that will fail at FastMatch
    let fast_match_fail = FastMatchBuilder::new()
        .add_source_agent(AgentId::new("gpt-4"))
        .build();

    let clause_fail_fast = MatchClause::fast_only(fast_match_fail);

    let ctx_wrong_agent = EventContext::new(
        AgentId::new("claude"),
        None,
        None,
        "text/plain".to_string(),
        HeaderFlags::empty(),
        HashMap::new(),
    );
    let result = clause_fail_fast.evaluate(&ctx_wrong_agent, None);

    println!("Scenario 1: FastMatch fails");
    println!("  Expected agent: gpt-4, Got: claude");
    println!("  Matched: {}", result.is_match);
    println!("  Failed at: {:?}", result.tier);
    println!("  → Evaluation stopped early, no expensive checks performed!");
    println!();

    // Create a clause that passes FastMatch but fails MatchExpression
    let fast_match_pass = FastMatchBuilder::new()
        .add_source_agent(AgentId::new("gpt-4"))
        .build();

    let match_expr_fail = MatchExpression::Field(FieldComparison {
        field_path: "severity".to_string(),
        operator: ComparisonOp::Equal,
        value: FieldValue::String("critical".to_string()),
    });

    let clause_fail_expr = MatchClause::with_expression(fast_match_pass, match_expr_fail);

    let mut ctx_pass_fast = EventContext::new(
        AgentId::new("gpt-4"),
        None,
        None,
        "text/plain".to_string(),
        HeaderFlags::empty(),
        HashMap::new(),
    );
    ctx_pass_fast.set_header("severity".to_string(), FieldValue::String("low".to_string()));

    let result = clause_fail_expr.evaluate(&ctx_pass_fast, None);

    println!("Scenario 2: FastMatch passes, MatchExpression fails");
    println!("  FastMatch: ✓ (agent is gpt-4)");
    println!("  MatchExpression: ✗ (severity is 'low', not 'critical')");
    println!("  Matched: {}", result.is_match);
    println!("  Failed at: {:?}", result.tier);
    println!("  → WASM hook never executed (saved expensive computation)!");
    println!();

    // ========================================================================
    // Example 11: Performance Characteristics
    // ========================================================================
    println!("Example 11: Performance Characteristics");
    println!("------------------------------------------");

    let fast_only = MatchClause::fast_only(FastMatch::new());
    let with_expr = MatchClause::with_expression(FastMatch::new(), MatchExpression::Always);
    let complete = MatchClause::complete(
        FastMatch::new(),
        MatchExpression::Always,
        WasmHookRef::new("test".to_string(), "sha256:test".to_string()),
    );

    println!("Match Clause Types and Their Cost:");
    println!("  FastMatch only:");
    println!("    Max tier: {:?}", fast_only.max_evaluation_tier());
    println!("    Cost: O(1) - Hash lookups only");
    println!();
    println!("  FastMatch + MatchExpression:");
    println!("    Max tier: {:?}", with_expr.max_evaluation_tier());
    println!("    Cost: O(log n) to O(n) - Depends on expression complexity");
    println!();
    println!("  Complete (with WASM):");
    println!("    Max tier: {:?}", complete.max_evaluation_tier());
    println!("    Cost: O(timeout) - Bounded by WASM execution time");
    println!();

    // ========================================================================
    // Example 12: Real-World Use Cases
    // ========================================================================
    println!("Example 12: Real-World Use Cases");
    println!("-----------------------------------");

    // Use Case 1: PII Detection Rule
    println!("Use Case 1: PII Detection");
    let pii_fast = FastMatchBuilder::new()
        .require_flags(HeaderFlags::from_bits(HeaderFlags::CONTAINS_PII))
        .build();

    let pii_expr = MatchExpression::Or(vec![
        MatchExpression::Field(FieldComparison {
            field_path: "content".to_string(),
            operator: ComparisonOp::Contains,
            value: FieldValue::String("SSN".to_string()),
        }),
        MatchExpression::Field(FieldComparison {
            field_path: "content".to_string(),
            operator: ComparisonOp::Contains,
            value: FieldValue::String("credit card".to_string()),
        }),
    ]);

    let _pii_clause = MatchClause::with_expression(pii_fast, pii_expr);
    println!("  ✓ Detects content with PII flag and sensitive keywords");
    println!();

    // Use Case 2: Rate Limiting Rule
    println!("Use Case 2: Rate Limiting");
    let _rate_limit_fast = FastMatchBuilder::new()
        .add_source_agent(AgentId::new("public-api"))
        .forbid_flags(HeaderFlags::from_bits(HeaderFlags::HIGH_PRIORITY))
        .build();

    println!("  ✓ Applies rate limits to public API, except high-priority requests");
    println!();

    // Use Case 3: Security Validation
    println!("Use Case 3: Security Validation");
    let security_fast = FastMatchBuilder::new()
        .require_flags(HeaderFlags::from_bits(HeaderFlags::AUTHENTICATED))
        .build();

    let security_wasm = WasmHookRef::new(
        "jwt-validator".to_string(),
        "sha256:security".to_string(),
    );

    let _security_clause = MatchClause::complete(
        security_fast,
        MatchExpression::Always,
        security_wasm,
    );

    println!("  ✓ Validates authentication flag, then runs JWT validation via WASM");
    println!();

    println!("All examples completed successfully!");
    println!("\nKey Takeaways:");
    println!("  1. FastMatch provides O(1) filtering with hash lookups");
    println!("  2. MatchExpression allows complex logical conditions");
    println!("  3. WASM hooks enable custom semantic validation");
    println!("  4. Early termination optimizes performance");
    println!("  5. Composable expressions enable powerful rule logic");
}
