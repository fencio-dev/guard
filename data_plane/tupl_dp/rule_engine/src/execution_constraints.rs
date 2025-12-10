// Define and enfroce runtime contraints for rule execution including
// latency bounds, CPR/memor and sampling rates. 
// THis module probvides:
//1. Contraint definitions for different execution contexts(fast path rules, 
// semantic rules, WASM hooks)
//2. Runtime enforcement mechanisms with timeout tracking
//3. Resource budget management
//4. Runtime enforcement mechanisms with timeout tracking
//5. Sampling support for observational rules

use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Represents different types of constraint violations that can occur during
/// rule execution
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConstraintViolationType {
    /// Execution exceeded maximum allowed time
    TimeoutExceeded {
        allowed_ms: u64,
        actual_ms: u64,
    },
    /// Memory usage exceeded the limit
    MemoryExceeded {
        limit_bytes: u64,
        actual_bytes: u64,
    },
    /// CPU usage exceeded allocated shares
    CpuExceeded {
        shares: u32,
        actual_usage_percent: f64,
    },
    /// Rule was sampled out (not executed due to sampling rate)
    SampledOut {
        sampling_rate: f64,
    },
    /// Multiple constraints violated simultaneously
    MultipleViolations(Vec<ConstraintViolationType>),
}

/// Error types for constraint operations
#[derive(Debug, Error)]
pub enum ConstraintError{
    #[error("Constraint violation: {0:?}")]
    Violation(ConstraintViolationType),

    #[error("Invalid constraint configuration: {0}")]
    InvalidConfiguration(String),
    
    #[error("Resource exhausted: {0}")]
    ResourceExhausted(String),
    
    #[error("Enforcement failed: {0}")]
    EnforcementFailure(String),
}

/// Defines execution constraints for a rule or operation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionConstraints {
    /// Maximum execution time in milliseconds
    /// For fast rules: typically 1-5ms
    /// For semantic rules: typically 50-200ms
    /// For WASM hooks: configurable, default 10ms
    pub max_exec_ms: u64,

    /// Soft CPU share allocation (0-100)
    /// Used for prioritization and resource accounting
    /// Not a hard limit, but used by scheduler
    pub cpu_shares: Option<u32>,
    
    /// Maximum memory allowed in bytes
    /// Enforced for WASM sandboxes and semantic evaluation contexts
    pub memory_limit_bytes: Option<u64>,
    
    /// Sampling rate for observational rules (0.0 - 1.0)
    /// 1.0 = always execute, 0.5 = execute 50% of the time, 0.0 = never execute
    pub sampling_rate: f64,
    
    /// Whether to fail closed (deny) or open (allow) on timeout
    /// HARD rules default to fail_closed=true
    /// SOFT rules default to fail_closed=false
    pub fail_closed_on_timeout: bool,
    
    /// Maximum number of retries on transient failures
    pub max_retries: Option<u32>,
    
    /// Backoff duration between retries in milliseconds
    pub retry_backoff_ms: Option<u64>,
}

impl ExecutionConstraints {
    ///Create constraints for fast path rules (strict, low latency)
    pub fn fast_rule() -> Self {
        Self {
            max_exec_ms: 5,
            cpu_shares: Some(10),
            memory_limit_bytes: Some(1024*1024), // 1 MiB
            sampling_rate: 1.0,
            fail_closed_on_timeout: true,
            max_retries: None,
            retry_backoff_ms: None,
        }
    }

    /// Create constraint for semantic rules
    pub fn semantic_rule() -> Self {
        Self {
            max_exec_ms: 100,
            cpu_shares: Some(50),
            memory_limit_bytes: Some(10*1024*1024), //10 Mib
            sampling_rate: 1.0, 
            fail_closed_on_timeout: false, 
            max_retries: Some(2),
            retry_backoff_ms: Some(10),
        }
    }

    /// Create constraints for WASM hooks
    pub fn wasm_hook(max_ms: u64) -> Self {
        Self {
            max_exec_ms: max_ms,
            cpu_shares: Some(20),
            memory_limit_bytes: Some(5*1024*1024), //5 Mib
            sampling_rate: 1.0, 
            fail_closed_on_timeout: true, 
            max_retries: Some(2),
            retry_backoff_ms: Some(10),
        }
    }

    // Create constraints for observational rules (logging, metrics)
    pub fn observational(sampling_rate: f64) -> Self {
        Self {
            max_exec_ms: 2,
            cpu_shares: Some(5),
            memory_limit_bytes: Some(512 * 1024), // 512 KB
            sampling_rate: sampling_rate.clamp(0.0, 1.0),
            fail_closed_on_timeout: false, // Never block on observational rules
            max_retries: None,
            retry_backoff_ms: None,
        }
    }

    /// Validate constraint config
    pub fn validate(&self) -> Result<(), ConstraintError> {
        if self.max_exec_ms == 0 {
            return Err(ConstraintError::InvalidConfiguration(
                "max_exec_ms must be greater than 0".to_string()
            ));
        }

        if let Some(shares) = self.cpu_shares {
            if shares > 100 {
                return Err(ConstraintError::InvalidConfiguration(
                    "cpu_shares must be between 0 and 100".to_string()
                ));
            }
        }

        if self.sampling_rate < 0.0 || self.sampling_rate > 1.0 {
            return Err(ConstraintError::InvalidConfiguration(
                "sampling_rate must be between 0.0 and 1.0".to_string()
            ));
        }

        if let Some(retries) = self.max_retries {
            if retries > 10 {
                return Err(ConstraintError::InvalidConfiguration(
                    "max_retries should not exceed 10".to_string()
                ));
            }
        }
        Ok(())
    }
    
    ///Check if an operation should be sampled (executed)
    pub fn should_sample(&self) -> bool {
        if self.sampling_rate >= 1.0 {
            return true;
        }

        if self.sampling_rate <=0.0 {
            return false;
        }
        use rand::Rng;
        let mut rng = rand::thread_rng();
        rng.gen::<f64>() < self.sampling_rate
    }

    /// Create a budget tracker for this constraint set
    pub fn create_budget(&self) -> ExecutionBudget {
        ExecutionBudget::new(self.clone())
    }
}

impl Default for ExecutionConstraints {
    fn default() -> Self {
        Self::fast_rule()
    }
}

/// Runtime budget tracker for enforcing execution constraints
#[derive(Debug)]
pub struct ExecutionBudget {
    constraints: ExecutionConstraints,
    start_time: Instant,
    memory_used: u64,
    cpu_time_us: u64,
    violations: Vec<ConstraintViolationType>,
}

impl ExecutionBudget {
    /// Create a new budget tracker
    pub fn new(constraints: ExecutionConstraints) -> Self {
        Self {
            constraints,
            start_time: Instant::now(),
            memory_used: 0,
            cpu_time_us: 0,
            violations: Vec::new(),
        }
    }

    /// Get elapsed time since budget creation
    pub fn elapsed_ms(&self) -> u64 {
        self.start_time.elapsed().as_millis() as u64
    }
    
    /// Get remaining time budget in milliseconds
    pub fn remaining_ms(&self) -> i64 {
        self.constraints.max_exec_ms as i64 - self.elapsed_ms() as i64
    }
    
    /// Check if time budget is exhausted
    pub fn is_timeout(&self) -> bool {
        self.elapsed_ms() >= self.constraints.max_exec_ms
    }
    
    /// Record memory usage
    pub fn record_memory_usage(&mut self, bytes: u64) {
        self.memory_used = bytes;
    }
    
    /// Record CPU time usage
    pub fn record_cpu_time(&mut self, microseconds: u64) {
        self.cpu_time_us = microseconds;
    }
    
    /// Check all constraints and return violations
    pub fn check(&mut self) -> Result<(), ConstraintError> {
        let mut violations = Vec::new();
        
        // Check timeout
        let elapsed = self.elapsed_ms();
        if elapsed >= self.constraints.max_exec_ms {
            violations.push(ConstraintViolationType::TimeoutExceeded {
                allowed_ms: self.constraints.max_exec_ms,
                actual_ms: elapsed,
            });
        }
        
        // Check memory limit
        if let Some(limit) = self.constraints.memory_limit_bytes {
            if self.memory_used > limit {
                violations.push(ConstraintViolationType::MemoryExceeded {
                    limit_bytes: limit,
                    actual_bytes: self.memory_used,
                });
            }
        }
        
        if !violations.is_empty() {
            self.violations.extend(violations.clone());
            
            let violation = if violations.len() == 1 {
                violations.into_iter().next().unwrap()
            } else {
                ConstraintViolationType::MultipleViolations(violations)
            };
            
            return Err(ConstraintError::Violation(violation));
        }
        
        Ok(())
    }
    
    /// Enforce constraints with a closure, returning result or timeout error
    pub fn enforce<F, T>(&mut self, f: F) -> Result<T, ConstraintError>
    where
        F: FnOnce() -> Result<T, ConstraintError>,
    {
        // Check sampling first
        if !self.constraints.should_sample() {
            return Err(ConstraintError::Violation(
                ConstraintViolationType::SampledOut {
                    sampling_rate: self.constraints.sampling_rate,
                }
            ));
        }
        
        // Check timeout before execution
        self.check()?;
        
        // Execute the operation
        let result = f()?;
        
        // Check constraints after execution
        self.check()?;
        
        Ok(result)
    }
    
    /// Get all recorded violations
    pub fn get_violations(&self) -> &[ConstraintViolationType] {
        &self.violations
    }
    
    /// Get execution statistics
    pub fn stats(&self) -> ExecutionStats {
        ExecutionStats {
            elapsed_ms: self.elapsed_ms(),
            memory_used_bytes: self.memory_used,
            cpu_time_us: self.cpu_time_us,
            timeout_occurred: self.is_timeout(),
            violation_count: self.violations.len(),
        }
    }
}

/// Statistics about an execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStats {
    pub elapsed_ms: u64,
    pub memory_used_bytes: u64,
    pub cpu_time_us: u64,
    pub timeout_occurred: bool,
    pub violation_count: usize,
}

/// Constraint enforcer that manages budgets and policies
#[derive(Debug)]
pub struct ConstraintEnforcer {
    /// Default constraints for different rule types
    fast_rule_constraints: ExecutionConstraints,
    semantic_rule_constraints: ExecutionConstraints,
    wasm_hook_constraints: ExecutionConstraints,
    observational_constraints: ExecutionConstraints,
}

impl ConstraintEnforcer {
    /// Create a new constraint enforcer with default settings
    pub fn new() -> Self {
        Self {
            fast_rule_constraints: ExecutionConstraints::fast_rule(),
            semantic_rule_constraints: ExecutionConstraints::semantic_rule(),
            wasm_hook_constraints: ExecutionConstraints::wasm_hook(10),
            observational_constraints: ExecutionConstraints::observational(1.0),
        }
    }
    
    /// Create with custom constraint sets
    pub fn with_constraints(
        fast_rule: ExecutionConstraints,
        semantic_rule: ExecutionConstraints,
        wasm_hook: ExecutionConstraints,
        observational: ExecutionConstraints,
    ) -> Result<Self, ConstraintError> {
        // Validate all constraints
        fast_rule.validate()?;
        semantic_rule.validate()?;
        wasm_hook.validate()?;
        observational.validate()?;
        
        Ok(Self {
            fast_rule_constraints: fast_rule,
            semantic_rule_constraints: semantic_rule,
            wasm_hook_constraints: wasm_hook,
            observational_constraints: observational,
        })
    }
    
    /// Get constraints for a specific rule type
    pub fn get_constraints(&self, rule_type: RuleType) -> &ExecutionConstraints {
        match rule_type {
            RuleType::Fast => &self.fast_rule_constraints,
            RuleType::Semantic => &self.semantic_rule_constraints,
            RuleType::WasmHook => &self.wasm_hook_constraints,
            RuleType::Observational => &self.observational_constraints,
        }
    }
    
    /// Create a budget for a specific rule type
    pub fn create_budget(&self, rule_type: RuleType) -> ExecutionBudget {
        self.get_constraints(rule_type).create_budget()
    }
    
    /// Execute with enforced constraints
    pub fn execute_with_constraints<F, T>(
        &self,
        rule_type: RuleType,
        f: F,
    ) -> Result<T, ConstraintError>
    where
        F: FnOnce() -> Result<T, ConstraintError>,
    {
        let mut budget = self.create_budget(rule_type);
        budget.enforce(f)
    }
}

impl Default for ConstraintEnforcer {
    fn default() -> Self {
        Self::new()
    }
}

/// Type of rule for constraint selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleType {
    /// Fast-path rules with strict latency requirements
    Fast,
    /// Semantic rules with relaxed constraints
    Semantic,
    /// WASM hook execution
    WasmHook,
    /// Observational rules (logging, metrics)
    Observational,
}

/// Retry policy for handling transient failures
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    max_attempts: u32,
    backoff_ms: u64,
    exponential_backoff: bool,
}

impl RetryPolicy {
    pub fn new(max_attempts: u32, backoff_ms: u64) -> Self {
        Self {
            max_attempts,
            backoff_ms,
            exponential_backoff: false,
        }
    }
    
    pub fn with_exponential_backoff(mut self) -> Self {
        self.exponential_backoff = true;
        self
    }
    
    /// Execute a function with retry logic
    pub fn execute<F, T, E>(&self, mut f: F) -> Result<T, E>
    where
        F: FnMut() -> Result<T, E>,
    {
        let mut last_error = None;
        
        for attempt in 0..self.max_attempts {
            match f() {
                Ok(result) => return Ok(result),
                Err(e) => {
                    last_error = Some(e);
                    
                    if attempt < self.max_attempts - 1 {
                        let delay = if self.exponential_backoff {
                            self.backoff_ms * 2_u64.pow(attempt)
                        } else {
                            self.backoff_ms
                        };
                        
                        std::thread::sleep(Duration::from_millis(delay));
                    }
                }
            }
        }
        
        Err(last_error.unwrap())
    }
}