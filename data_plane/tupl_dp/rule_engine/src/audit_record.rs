// Track rule execution decisions with complete provenance, timestamps
// and cryptographic verification for compliance and debugging.
// This module provides:
// 1. Compact decision records for Fast Path Logging
// 2. Full audit records with detailed execution context
// 3. Provenance hash computation for tamper detection. 
// 4. Multi level timestamp tracking
// 5. Decision outcomes classification
// 6. Serialization for asyunc persistance. 

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use sha2::{Sha256, Digest};

/// Unique sequence number for audit records
pub type SequenceNumber = u64;

/// Reference to shared memory payload data
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PayloadRef {
    ///Shared memeory segment ID
    pub shm_id: String,

    /// Offset within the segment
    pub offset: u64, 

    /// Size of the paylaod
    pub size: u64,

    /// Optional content hash for verification
    pub content_hash: Option<String>,
}

impl PayloadRef {
    pub fn new(shm_id: String, offset: u64, size:u64) -> Self {
        Self {
            shm_id, 
            offset, 
            size, 
            content_hash: None,
        }
    }

    pub fn with_hash(mut self, hash: String) -> Self {
        self.content_hash = Some(hash);
        self
    }
}

/// Log Level for audit records
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AuditLogLevel {
    /// Only log critical security decisions
    Critical = 0, 
    /// Log denials and violations
    High = 1, 
    /// Log all enforcement actions
    Medium = 2, 
    ///Log all decisions including allows
    Low = 3, 
    /// Log everything including debug info
    Trace = 4,
}

impl AuditLogLevel {
    pub fn should_log(&self, configured_level: AuditLogLevel) -> bool {
        *self <= configured_level
    }
}

/// Decision outcomes from rule evaluation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DecisionOutcome {
    /// Request allowed to proceed
    Allow {
        /// Optional metadata attached
        metadata: Option<HashMap<String, String>>,
    },
    /// Request Denied/Blocked
    Deny {
        reason: String, 
        ///Error Code
        code: Option<String>,
    },
    /// Payload transformed/rewritten
    Rewrite {
        /// Type of transformation applied
        transform_type: String,
    },
    /// Payload redacted (PII removed)
    Redact {
        /// Fields that were redacted
        redacted_fields: Vec<String>,
    },
    /// Request routed to different destination
    Route {
        /// Destination agent/endpoint
        destination: String,
    },
    /// Sidecar spawned for additional processing
    SpawnSidecar {
        /// Sidecar type/spec reference
        sidecar_type: String,
    },
    /// Rate limit applied
    RateLimit {
        /// Scope of rate limit
        scope: String,
        /// Action taken (allow/deny/delay)
        action: String,
    },
    /// Sandbox execution performed
    SandboxExecute {
        /// Sandbox type (WASM/semantic)
        sandbox_type: String,
        /// Execution result
        result: String,
    },
    /// Constraint violation occurred
    ConstraintViolation {
        /// Type of violation
        violation_type: String,
        /// Whether request was allowed despite violation
        fail_open: bool,
    },
    /// Error during rule evaluation
    Error {
        /// Error message
        message: String,
        /// Error code
        code: String,
    },
    /// Rule did not match (skipped)
    Skip,
}

impl DecisionOutcome {
    ///Check if this outcome represents a blocking decision
    pub fn is_blocking(&self) -> bool {
        matches!(
            self, 
            DecisionOutcome::Deny {..}
            | DecisionOutcome::ConstraintViolation {fail_open: false, ..}
            | DecisionOutcome::Error {..}
        )
    }

    /// Check if this outcome represents a modification
    pub fn is_modification(&self) -> bool {
        matches!(
            self,
            DecisionOutcome::Rewrite { .. }
            | DecisionOutcome::Redact { .. }
            | DecisionOutcome::Route { .. }
        )
    }

    /// Get a summary string for the outcome
    pub fn summary(&self) -> String {
        match self {
            DecisionOutcome::Allow { .. } => "ALLOW".to_string(),
            DecisionOutcome::Deny { reason, .. } => format!("DENY: {}", reason),
            DecisionOutcome::Rewrite { transform_type } => format!("REWRITE: {}", transform_type),
            DecisionOutcome::Redact { redacted_fields } => {
                format!("REDACT: {} fields", redacted_fields.len())
            }
            DecisionOutcome::Route { destination } => format!("ROUTE: {}", destination),
            DecisionOutcome::SpawnSidecar { sidecar_type } => {
                format!("SIDECAR: {}", sidecar_type)
            }
            DecisionOutcome::RateLimit { action, .. } => format!("RATELIMIT: {}", action),
            DecisionOutcome::SandboxExecute { result, .. } => format!("SANDBOX: {}", result),
            DecisionOutcome::ConstraintViolation { violation_type, fail_open } => {
                format!("VIOLATION: {} (fail_open={})", violation_type, fail_open)
            }
            DecisionOutcome::Error { code, .. } => format!("ERROR: {}", code),
            DecisionOutcome::Skip => "SKIP".to_string(),
        }
    }
}

/// Detailed timetstamp tracking for rule evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct EvaluationTimestamps {
    /// When the event was received by the FastPath
    pub received_at: SystemTime,
    /// When the rule evaluation started
    pub eval_started_at: SystemTime, 
    /// When the rule evaluations completed
    pub eval_completed_at: SystemTime, 
    /// When the decision was finalised
    pub decision_at: SystemTime,
    /// When audit record was created
    pub audit_created_at: SystemTime, 
}

impl EvaluationTimestamps {
    /// Create timestamps with received time
    pub fn new(received_at: SystemTime) -> Self {
        Self {
            received_at,
            eval_started_at: SystemTime::now(),
            eval_completed_at: SystemTime::now(),
            decision_at: SystemTime::now(),
            audit_created_at: SystemTime::now(),
        }
    }

    /// Create timestamps starting now
    pub fn now() -> Self {
        let now = SystemTime::now();
        Self {
            received_at: now,
            eval_started_at: now,
            eval_completed_at: now,
            decision_at: now,
            audit_created_at: now,
        }
    }
    /// Calculate total evaluation time in microseconds
    pub fn total_eval_time_us(&self) -> u64 {
        self.eval_completed_at
            .duration_since(self.eval_started_at)
            .unwrap_or_default()
            .as_micros() as u64
    }
    
    /// Calculate total processing time in microseconds
    pub fn total_processing_time_us(&self) -> u64 {
        self.decision_at
            .duration_since(self.received_at)
            .unwrap_or_default()
            .as_micros() as u64
    }
    
    /// Get Unix timestamp in milliseconds for received_at
    pub fn received_at_millis(&self) -> u64 {
        self.received_at
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
}

/// Compact decision records for fast path logging
/// This is the minimal information needed for high throughput audit trails
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactDecisionRecord {
    /// Sequence number (monotonically increasing)
    pub seq: SequenceNumber,
    /// Rule that made the decision
    pub rule_id: String,
    /// Version of the rule
    pub rule_version: u64,
    /// Decision outcome
    pub decision: String,
    /// When the decision was made (Unix timestamp in ms)
    pub timestamp_ms: u64,
    /// Cryptographic hash of the decision for tamper detection
    pub decision_hash: String,
    /// References to payload data in shared memory
    pub payload_refs: Vec<PayloadRef>,
}

impl CompactDecisionRecord {
    /// Create a new compact record
    pub fn new(
        seq: SequenceNumber,
        rule_id: String,
        rule_version: u64,
        decision: String,
        payload_refs: Vec<PayloadRef>,
    ) -> Self {
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        
        let mut record = Self {
            seq,
            rule_id,
            rule_version,
            decision,
            timestamp_ms,
            decision_hash: String::new(),
            payload_refs,
        };
        
        // Compute hash after all fields are set
        record.decision_hash = record.compute_hash();
        record
    }

    /// Compute cryptographic hash of the record
    pub fn compute_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.seq.to_le_bytes());
        hasher.update(self.rule_id.as_bytes());
        hasher.update(self.rule_version.to_le_bytes());
        hasher.update(self.decision.as_bytes());
        hasher.update(self.timestamp_ms.to_le_bytes());
        
        for payload_ref in &self.payload_refs {
            hasher.update(payload_ref.shm_id.as_bytes());
            hasher.update(payload_ref.offset.to_le_bytes());
            hasher.update(payload_ref.size.to_le_bytes());
        }
        
        format!("{:x}", hasher.finalize())
    }
    
    /// Verify the decision hash
    pub fn verify_hash(&self) -> bool {
        let computed = self.compute_hash();
        computed == self.decision_hash
    }

    /// Convert to full audit record (requires additional context)
    pub fn to_full_record(
        self,
        outcome: DecisionOutcome,
        timestamps: EvaluationTimestamps,
        context: AuditContext,
    ) -> AuditRecord {
        AuditRecord {
            seq: self.seq,
            rule_id: self.rule_id,
            rule_version: self.rule_version,
            bundle_id: None,
            outcome,
            timestamps,
            provenance_hash: self.decision_hash,
            payload_refs: self.payload_refs,
            context,
            log_level: AuditLogLevel::Low,
            emit_event: true,
            explanation: None,
            metadata: HashMap::new(),
        }
    }
}

/// Additional context for audit records
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditContext {
    /// Source agent ID
    pub source_agent: Option<String>,
    /// Destination agent ID
    pub dest_agent: Option<String>,
    /// Flow ID
    pub flow_id: Option<String>,
    /// Payload type/data type
    pub payload_dtype: Option<String>,
    /// Rule enforcement class
    pub enforcement_class: Option<String>,
    /// Constraint violations (if any)
    pub constraint_violations: Vec<String>,
    /// Execution statistics
    pub exec_stats: Option<ExecutionStatistics>,
    /// User/tenant ID (for multi-tenancy)
    pub tenant_id: Option<String>,
    /// Request ID for tracing
    pub request_id: Option<String>,
}

impl AuditContext {
    pub fn new() -> Self {
        Self {
            source_agent: None,
            dest_agent: None,
            flow_id: None,
            payload_dtype: None,
            enforcement_class: None,
            constraint_violations: Vec::new(),
            exec_stats: None,
            tenant_id: None,
            request_id: None,
        }
    }
    
    pub fn builder() -> AuditContextBuilder {
        AuditContextBuilder::new()
    }
}

impl Default for AuditContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for AuditContext
pub struct AuditContextBuilder {
    context: AuditContext,
}

impl AuditContextBuilder {
    pub fn new() -> Self {
        Self {
            context: AuditContext::new(),
        }
    }
    
    pub fn source_agent(mut self, agent: String) -> Self {
        self.context.source_agent = Some(agent);
        self
    }
    
    pub fn dest_agent(mut self, agent: String) -> Self {
        self.context.dest_agent = Some(agent);
        self
    }

    pub fn flow_id(mut self, flow: String) -> Self {
        self.context.flow_id = Some(flow);
        self
    }
    
    pub fn payload_dtype(mut self, dtype: String) -> Self {
        self.context.payload_dtype = Some(dtype);
        self
    }
    
    pub fn enforcement_class(mut self, class: String) -> Self {
        self.context.enforcement_class = Some(class);
        self
    }
    
    pub fn add_violation(mut self, violation: String) -> Self {
        self.context.constraint_violations.push(violation);
        self
    }
    
    pub fn exec_stats(mut self, stats: ExecutionStatistics) -> Self {
        self.context.exec_stats = Some(stats);
        self
    }
    
    pub fn tenant_id(mut self, tenant: String) -> Self {
        self.context.tenant_id = Some(tenant);
        self
    }
    
    pub fn request_id(mut self, request: String) -> Self {
        self.context.request_id = Some(request);
        self
    }
    
    pub fn build(self) -> AuditContext {
        self.context
    }
}
impl Default for AuditContextBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Execution statistics for audit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStatistics {
    /// Evaluation time in microseconds
    pub eval_time_us: u64,
    /// Memory used in bytes
    pub memory_used_bytes: u64,
    /// CPU time in microseconds
    pub cpu_time_us: u64,
    /// Number of rules evaluated
    pub rules_evaluated: u32,
    /// Number of constraint checks
    pub constraint_checks: u32,
}

impl ExecutionStatistics {
    pub fn new() -> Self {
        Self {
            eval_time_us: 0,
            memory_used_bytes: 0,
            cpu_time_us: 0,
            rules_evaluated: 0,
            constraint_checks: 0,
        }
    }
}

impl Default for ExecutionStatistics {
    fn default() -> Self {
        Self::new()
    }
}

/// Full audit record with complete provenance and context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    /// Sequence number (monotonically increasing)
    pub seq: SequenceNumber,
    
    /// Rule identification
    pub rule_id: String,
    pub rule_version: u64,
    pub bundle_id: Option<String>,
    
    /// Decision details
    pub outcome: DecisionOutcome,
    
    /// Timing information
    pub timestamps: EvaluationTimestamps,
    
    /// Provenance and integrity
    pub provenance_hash: String,
    
    /// Payload references
    pub payload_refs: Vec<PayloadRef>,
    
    /// Evaluation context
    pub context: AuditContext,
    
    /// Audit configuration
    pub log_level: AuditLogLevel,
    pub emit_event: bool,
    
    /// Human-readable explanation
    pub explanation: Option<String>,
    
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl AuditRecord {
    /// Create a new audit record
    pub fn new(
        seq: SequenceNumber,
        rule_id: String,
        rule_version: u64,
        outcome: DecisionOutcome,
    ) -> Self {
        let timestamps = EvaluationTimestamps::now();
        let provenance_hash = Self::compute_provenance_hash(
            seq,
            &rule_id,
            rule_version,
            &outcome,
            &timestamps,
        );
        
        Self {
            seq,
            rule_id,
            rule_version,
            bundle_id: None,
            outcome,
            timestamps,
            provenance_hash,
            payload_refs: Vec::new(),
            context: AuditContext::new(),
            log_level: AuditLogLevel::Low,
            emit_event: true,
            explanation: None,
            metadata: HashMap::new(),
        }
    }

    /// Create builder for audit record
    pub fn builder(seq: SequenceNumber, rule_id: String, rule_version: u64) -> AuditRecordBuilder {
        AuditRecordBuilder::new(seq, rule_id, rule_version)
    }
    
    /// Compute provenance hash for tamper detection
    pub fn compute_provenance_hash(
        seq: SequenceNumber,
        rule_id: &str,
        rule_version: u64,
        outcome: &DecisionOutcome,
        timestamps: &EvaluationTimestamps,
    ) -> String {
        let mut hasher = Sha256::new();
        
        hasher.update(seq.to_le_bytes());
        hasher.update(rule_id.as_bytes());
        hasher.update(rule_version.to_le_bytes());
        
        // Hash the outcome summary
        hasher.update(outcome.summary().as_bytes());
        
        // Hash the timestamp
        hasher.update(timestamps.received_at_millis().to_le_bytes());
        
        format!("{:x}", hasher.finalize())
    }

    /// Verify the provenance hash
    pub fn verify_provenance(&self) -> bool {
        let computed = Self::compute_provenance_hash(
            self.seq,
            &self.rule_id,
            self.rule_version,
            &self.outcome,
            &self.timestamps,
        );
        computed == self.provenance_hash
    }
    
    /// Convert to compact record for fast logging
    pub fn to_compact(&self) -> CompactDecisionRecord {
        CompactDecisionRecord {
            seq: self.seq,
            rule_id: self.rule_id.clone(),
            rule_version: self.rule_version,
            decision: self.outcome.summary(),
            timestamp_ms: self.timestamps.received_at_millis(),
            decision_hash: self.provenance_hash.clone(),
            payload_refs: self.payload_refs.clone(),
        }
    }
    
    /// Check if this record should be logged at given level
    pub fn should_log(&self, configured_level: AuditLogLevel) -> bool {
        self.log_level.should_log(configured_level)
    }
    
    /// Get a summary string for logging
    pub fn summary(&self) -> String {
        format!(
            "seq={} rule={} v{} outcome={} time={}μs",
            self.seq,
            self.rule_id,
            self.rule_version,
            self.outcome.summary(),
            self.timestamps.total_eval_time_us()
        )
    }

    /// Add metadata key-value pair
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }
    
    /// Set explanation template
    pub fn set_explanation(&mut self, explanation: String) {
        self.explanation = Some(explanation);
    }
}

/// Builder for AuditRecord
pub struct AuditRecordBuilder {
    seq: SequenceNumber,
    rule_id: String,
    rule_version: u64,
    bundle_id: Option<String>,
    outcome: Option<DecisionOutcome>,
    timestamps: Option<EvaluationTimestamps>,
    payload_refs: Vec<PayloadRef>,
    context: AuditContext,
    log_level: AuditLogLevel,
    emit_event: bool,
    explanation: Option<String>,
    metadata: HashMap<String, String>,
}

impl AuditRecordBuilder {
    pub fn new(seq: SequenceNumber, rule_id: String, rule_version: u64) -> Self {
        Self {
            seq,
            rule_id,
            rule_version,
            bundle_id: None,
            outcome: None,
            timestamps: None,
            payload_refs: Vec::new(),
            context: AuditContext::new(),
            log_level: AuditLogLevel::Low,
            emit_event: true,
            explanation: None,
            metadata: HashMap::new(),
        }
    }
    
    pub fn bundle_id(mut self, bundle_id: String) -> Self {
        self.bundle_id = Some(bundle_id);
        self
    }

    pub fn outcome(mut self, outcome: DecisionOutcome) -> Self {
        self.outcome = Some(outcome);
        self
    }
    
    pub fn timestamps(mut self, timestamps: EvaluationTimestamps) -> Self {
        self.timestamps = Some(timestamps);
        self
    }
    
    pub fn add_payload_ref(mut self, payload_ref: PayloadRef) -> Self {
        self.payload_refs.push(payload_ref);
        self
    }
    
    pub fn context(mut self, context: AuditContext) -> Self {
        self.context = context;
        self
    }
    
    pub fn log_level(mut self, level: AuditLogLevel) -> Self {
        self.log_level = level;
        self
    }
    
    pub fn emit_event(mut self, emit: bool) -> Self {
        self.emit_event = emit;
        self
    }
    
    pub fn explanation(mut self, explanation: String) -> Self {
        self.explanation = Some(explanation);
        self
    }
    
    pub fn add_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    pub fn build(self) -> Result<AuditRecord, String> {
        let outcome = self.outcome.ok_or("outcome is required")?;
        let timestamps = self.timestamps.unwrap_or_else(EvaluationTimestamps::now);
        
        let provenance_hash = AuditRecord::compute_provenance_hash(
            self.seq,
            &self.rule_id,
            self.rule_version,
            &outcome,
            &timestamps,
        );
        
        Ok(AuditRecord {
            seq: self.seq,
            rule_id: self.rule_id,
            rule_version: self.rule_version,
            bundle_id: self.bundle_id,
            outcome,
            timestamps,
            provenance_hash,
            payload_refs: self.payload_refs,
            context: self.context,
            log_level: self.log_level,
            emit_event: self.emit_event,
            explanation: self.explanation,
            metadata: self.metadata,
        })
    }
}

/// Audit trail manager for managing sequences and persistence
pub struct AuditTrail {
    next_seq: SequenceNumber,
    records: Vec<AuditRecord>,
    max_in_memory: usize,
}

impl AuditTrail {
    pub fn new(max_in_memory: usize) -> Self {
        Self {
            next_seq: 0,
            records: Vec::with_capacity(max_in_memory),
            max_in_memory,
        }
    }
    
    /// Get next sequence number
    pub fn next_seq(&mut self) -> SequenceNumber {
        let seq = self.next_seq;
        self.next_seq += 1;
        seq
    }
    
    /// Add record to trail
    pub fn add_record(&mut self, record: AuditRecord) {
        self.records.push(record);
        
        // If we exceed max in-memory, drop oldest records
        // In production, these would be flushed to persistent storage
        if self.records.len() > self.max_in_memory {
            self.records.drain(0..self.records.len() - self.max_in_memory);
        }
    }

    /// Get all records
    pub fn get_records(&self) -> &[AuditRecord] {
        &self.records
    }
    
    /// Get records by rule ID
    pub fn get_records_by_rule(&self, rule_id: &str) -> Vec<&AuditRecord> {
        self.records
            .iter()
            .filter(|r| r.rule_id == rule_id)
            .collect()
    }
    
    /// Get records in time range
    pub fn get_records_in_range(
        &self,
        start: SystemTime,
        end: SystemTime,
    ) -> Vec<&AuditRecord> {
        self.records
            .iter()
            .filter(|r| {
                r.timestamps.received_at >= start && r.timestamps.received_at <= end
            })
            .collect()
    }
    
    /// Clear all records
    pub fn clear(&mut self) {
        self.records.clear();
    }
}
