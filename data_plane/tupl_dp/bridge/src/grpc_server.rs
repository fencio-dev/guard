//! # gRPC Server for Rule Installation and Enforcement
//!
//! Implements a tonic gRPC server that receives:
//! 1. Rule installation requests from the control plane
//! 2. Enforcement requests from the SDK
//!
//! This server provides the data plane side of the control/data plane integration.

use crate::bridge::Bridge;
use crate::enforcement_engine::EnforcementEngine;
use crate::refresh::RefreshService;
use crate::rule_converter::{ControlPlaneRule, ParamValue, RuleConverter};
use crate::rule_vector::{convert_anchor_block, RuleVector};
use crate::types::{RuleFamilyId, RuleInstance};
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::convert::TryInto;
use std::sync::Arc;
use std::time::Duration;
use tonic::{transport::Server, Request, Response, Status};

#[derive(Debug, Deserialize)]
struct RuleAnchorsResponse {
    action_anchors: Vec<Vec<f32>>,
    action_count: usize,
    resource_anchors: Vec<Vec<f32>>,
    resource_count: usize,
    data_anchors: Vec<Vec<f32>>,
    data_count: usize,
    risk_anchors: Vec<Vec<f32>>,
    risk_count: usize,
}

fn convert_proto_rule_anchors(
    payload: RuleAnchorsPayload,
) -> Result<RuleVector, String> {
    fn convert_block(
        slot: &str,
        vectors: Vec<Vec<f32>>,
        count: i32,
    ) -> Result<([[f32; crate::rule_vector::SLOT_WIDTH]; crate::rule_vector::MAX_ANCHORS_PER_SLOT], usize), String> {
        if count < 0 {
            return Err(format!("Slot '{}' has negative count {}", slot, count));
        }
        convert_anchor_block(slot, &vectors, count as usize)
    }

    let RuleAnchorsPayload {
        action_anchors,
        action_count,
        resource_anchors,
        resource_count,
        data_anchors,
        data_count,
        risk_anchors,
        risk_count,
    } = payload;

    let action_vecs: Vec<Vec<f32>> = action_anchors.into_iter().map(|v| v.values).collect();
    let resource_vecs: Vec<Vec<f32>> = resource_anchors.into_iter().map(|v| v.values).collect();
    let data_vecs: Vec<Vec<f32>> = data_anchors.into_iter().map(|v| v.values).collect();
    let risk_vecs: Vec<Vec<f32>> = risk_anchors.into_iter().map(|v| v.values).collect();

    let (action_block, action_count) = convert_block("action", action_vecs, action_count)?;
    let (resource_block, resource_count) = convert_block("resource", resource_vecs, resource_count)?;
    let (data_block, data_count) = convert_block("data", data_vecs, data_count)?;
    let (risk_block, risk_count) = convert_block("risk", risk_vecs, risk_count)?;

    Ok(RuleVector {
        action_anchors: action_block,
        action_count,
        resource_anchors: resource_block,
        resource_count,
        data_anchors: data_block,
        data_count,
        risk_anchors: risk_block,
        risk_count,
    })
}

// Include the generated protobuf code
pub mod rule_installation {
    tonic::include_proto!("rule_installation");
}

use rule_installation::{
    data_plane_server::{DataPlane, DataPlaneServer},
    EnforceRequest, EnforceResponse, EnforcementSessionSummary, GetRuleStatsRequest,
    GetRuleStatsResponse, GetSessionRequest, GetSessionResponse, InstallRulesRequest,
    InstallRulesResponse, QueryTelemetryRequest, QueryTelemetryResponse, RefreshRulesRequest,
    RefreshRulesResponse, RemoveAgentRulesRequest, RemoveAgentRulesResponse, RuleAnchorsPayload,
    RuleEvidence, TableStats,
};

// ================================================================================================
// DATA PLANE SERVICE IMPLEMENTATION
// ================================================================================================

/// gRPC service implementation for rule installation and enforcement
pub struct DataPlaneService {
    /// Shared reference to the bridge
    bridge: Arc<Bridge>,

    /// Enforcement engine for v1.3 layer-based enforcement
    enforcement_engine: Arc<EnforcementEngine>,

    /// HTTP client for encoding rules
    encoding_http_client: Client,

    /// Base URL for the management plane
    management_plane_url: String,

    /// Hitlog query engine for telemetry
    hitlog_query: Arc<crate::telemetry::query::HitlogQuery>,

    /// Service for rule refresh from warm storage
    refresh_service: Arc<RefreshService>,
}

const CONNECT_TIMEOUT_MS: u64 = 2_000;
const REQUEST_TIMEOUT_MS: u64 = 30_000;

impl DataPlaneService {
    /// Create a new data plane service
    pub fn new(bridge: Arc<Bridge>, management_plane_url: String) -> Self {
        let sanitized_url = management_plane_url.trim_end_matches('/').to_string();

        // Enable telemetry for enforcement tracking
        use crate::telemetry::{TelemetryRecorder, TelemetryConfig};
        let hitlog_dir = std::env::var("HITLOG_DIR")
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                format!("{}/var/hitlogs", home)
            });

        let telemetry_config = TelemetryConfig {
            hitlog_dir: hitlog_dir.clone(),
            ..TelemetryConfig::default()
        };

        let telemetry = TelemetryRecorder::new(telemetry_config)
            .expect("Failed to initialize telemetry recorder");

        let enforcement_engine = Arc::new(
            EnforcementEngine::with_telemetry(
                Arc::clone(&bridge),
                sanitized_url.clone(),
                Some(Arc::new(telemetry)),
            )
            .expect("Failed to create enforcement engine with telemetry"),
        );

        let encoding_http_client = Client::builder()
            .connect_timeout(Duration::from_millis(CONNECT_TIMEOUT_MS))
            .timeout(Duration::from_millis(REQUEST_TIMEOUT_MS))
            .build()
            .expect("Failed to build reqwest client for rule encoding");

        // Initialize hitlog query for telemetry (clone hitlog_dir as it was moved to telemetry)
        let hitlog_query = Arc::new(crate::telemetry::query::HitlogQuery::new(&hitlog_dir));

        // Initialize refresh service for warm storage refresh
        let refresh_service = Arc::new(RefreshService::new(Arc::clone(&bridge)));

        DataPlaneService {
            bridge,
            enforcement_engine,
            encoding_http_client,
            management_plane_url: sanitized_url,
            hitlog_query,
            refresh_service,
        }
    }

    fn encoding_endpoint(&self, path: &str) -> String {
        let trimmed = path.trim_start_matches('/');
        format!("{}/{}", self.management_plane_url, trimmed)
    }

    async fn encode_rule_during_installation(
        &self,
        rule: &Arc<dyn RuleInstance>,
    ) -> Result<RuleVector, String> {
        let endpoint = match rule.family_id() {
            RuleFamilyId::ToolWhitelist => "/encode/rule/tool_whitelist",
            RuleFamilyId::ToolParamConstraint => "/encode/rule/tool_param_constraint",
            other => {
                return Err(format!("Unsupported rule family {:?} for encoding", other));
            }
        };

        let payload = rule.management_plane_payload();
        let is_empty = payload.as_object().map_or(true, |obj| obj.is_empty());
        if payload.is_null() || is_empty {
            return Err(format!(
                "Rule '{}' missing encoding payload (fail-closed)",
                rule.rule_id()
            ));
        }

        let response = self
            .encoding_http_client
            .post(self.encoding_endpoint(endpoint))
            .json(&payload)
            .send()
            .await
            .map_err(|e| {
                format!(
                    "Failed to call Management Plane {} for rule {}: {}",
                    endpoint,
                    rule.rule_id(),
                    e
                )
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "<unavailable>".to_string());
            return Err(format!(
                "{} returned {} for rule {}: {}",
                endpoint,
                status,
                rule.rule_id(),
                body
            ));
        }

        let anchors: RuleAnchorsResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse {} response: {}", endpoint, e))?;

        let (action_anchors, action_count) =
            convert_anchor_block("action", &anchors.action_anchors, anchors.action_count)?;
        let (resource_anchors, resource_count) = convert_anchor_block(
            "resource",
            &anchors.resource_anchors,
            anchors.resource_count,
        )?;
        let (data_anchors, data_count) =
            convert_anchor_block("data", &anchors.data_anchors, anchors.data_count)?;
        let (risk_anchors, risk_count) =
            convert_anchor_block("risk", &anchors.risk_anchors, anchors.risk_count)?;

        Ok(RuleVector {
            action_anchors,
            action_count,
            resource_anchors,
            resource_count,
            data_anchors,
            data_count,
            risk_anchors,
            risk_count,
        })
    }
}

#[tonic::async_trait]
impl DataPlane for DataPlaneService {
    /// Install rules from the control plane into the bridge
    async fn install_rules(
        &self,
        request: Request<InstallRulesRequest>,
    ) -> Result<Response<InstallRulesResponse>, Status> {
        let req = request.into_inner();

        println!("================================================");
        println!("  Installing Rules for Agent: {}", req.agent_id);
        println!("================================================");
        println!("  Config ID: {}", req.config_id);
        println!("  Owner: {}", req.owner);
        println!("  Rules to install: {}", req.rules.len());
        println!();

        let mut installed_count = 0;
        let mut rules_by_layer = HashMap::new();
        let mut failed_rules = Vec::new();

        for proto_rule in req.rules {
            let anchor_payload = proto_rule.anchors.clone();
            // Convert proto RuleInstance to ControlPlaneRule
            let cp_rule = ControlPlaneRule {
                rule_id: proto_rule.rule_id.clone(),
                family_id: proto_rule.family_id.clone(),
                layer: proto_rule.layer.clone(),
                agent_id: proto_rule.agent_id.clone(),
                priority: proto_rule.priority,
                enabled: proto_rule.enabled,
                created_at_ms: proto_rule.created_at_ms,
                params: proto_rule
                    .params
                    .into_iter()
                    .map(|(k, v)| {
                        let param_value = if let Some(value) = v.value {
                            match value {
                                rule_installation::param_value::Value::StringValue(s) => {
                                    ParamValue::String(s)
                                }
                                rule_installation::param_value::Value::IntValue(i) => {
                                    ParamValue::Int(i)
                                }
                                rule_installation::param_value::Value::FloatValue(f) => {
                                    ParamValue::Float(f)
                                }
                                rule_installation::param_value::Value::BoolValue(b) => {
                                    ParamValue::Bool(b)
                                }
                                rule_installation::param_value::Value::StringList(list) => {
                                    ParamValue::StringList(list.values)
                                }
                            }
                        } else {
                            ParamValue::String(String::new())
                        };
                        (k, param_value)
                    })
                    .collect(),
            };

            println!(
                "Processing rule: {} (family: {}, layer: {})",
                cp_rule.rule_id, cp_rule.family_id, cp_rule.layer
            );

            // Convert control plane rule to bridge rule instance
            match RuleConverter::convert(&cp_rule) {
                Ok(bridge_rule) => {
                    let family_id = bridge_rule.family_id();
                    let layer_id = family_id.layer();

                    println!("  ✓ Converted to {} rule", family_id.family_id());
                    println!(
                        "  → Installing into {} table ({})",
                        family_id.family_id(),
                        layer_id
                    );

                    // Encode rule and add to bridge
                    let rule_vector_result = if let Some(payload) = anchor_payload.clone() {
                        match convert_proto_rule_anchors(payload) {
                            Ok(vector) => Ok(vector),
                            Err(err) => {
                                eprintln!(
                                    "  ! Invalid supplied anchors for {}: {} (falling back to HTTP encoding)",
                                    cp_rule.rule_id, err
                                );
                                self.encode_rule_during_installation(&bridge_rule).await
                            }
                        }
                    } else {
                        self.encode_rule_during_installation(&bridge_rule).await
                    };

                    match rule_vector_result {
                        Ok(rule_vector) => {
                            match self.bridge.add_rule_with_anchors(bridge_rule, rule_vector) {
                                Ok(_) => {
                                    installed_count += 1;
                                    *rules_by_layer.entry(cp_rule.layer.clone()).or_insert(0) += 1;
                                    println!("  ✓ Successfully installed\n");
                                }
                                Err(e) => {
                                    let error_msg = format!(
                                        "Failed to add rule {} to bridge: {}",
                                        cp_rule.rule_id, e
                                    );
                                    eprintln!("  ✗ {}\n", error_msg);
                                    failed_rules.push(error_msg);
                                }
                            }
                        }
                        Err(e) => {
                            let error_msg =
                                format!("Failed to encode rule {}: {}", cp_rule.rule_id, e);
                            eprintln!("  ✗ {}\n", error_msg);
                            failed_rules.push(error_msg);
                        }
                    }
                }
                Err(e) => {
                    let error_msg = format!("Failed to convert rule {}: {}", cp_rule.rule_id, e);
                    eprintln!("  ✗ {}\n", error_msg);
                    failed_rules.push(error_msg);
                }
            }
        }

        println!("================================================");
        println!("  Installation Summary");
        println!("================================================");
        println!("  Successfully installed: {}", installed_count);
        println!("  Failed: {}", failed_rules.len());
        println!("  Bridge version: {}", self.bridge.version());
        println!("=================================================\n");

        if !failed_rules.is_empty() {
            return Err(Status::internal(format!(
                "Failed to install {} rules: {:?}",
                failed_rules.len(),
                failed_rules
            )));
        }

        Ok(Response::new(InstallRulesResponse {
            success: true,
            message: format!(
                "Successfully installed {} rules for agent {}",
                installed_count, req.agent_id
            ),
            rules_installed: installed_count as i32,
            rules_by_layer: rules_by_layer
                .into_iter()
                .map(|(k, v)| (k, v as i32))
                .collect(),
            bridge_version: self.bridge.version() as i64,
        }))
    }

    /// Remove all rules for an agent from the bridge
    async fn remove_agent_rules(
        &self,
        request: Request<RemoveAgentRulesRequest>,
    ) -> Result<Response<RemoveAgentRulesResponse>, Status> {
        let req = request.into_inner();
        println!("Removing rules for agent {}", req.agent_id);

        let mut removed_count = 0;

        // Iterate through all families and remove rules for this agent
        for family_id in RuleFamilyId::all() {
            if let Some(table) = self.bridge.get_table(&family_id) {
                let table_guard = table.read();

                // Get all rules for this agent
                let agent_rules = table_guard.query_by_agent(&req.agent_id);
                drop(table_guard);

                // Remove each rule
                for rule in agent_rules {
                    match self.bridge.remove_rule(&family_id, rule.rule_id()) {
                        Ok(removed) if removed => removed_count += 1,
                        Ok(_) => {}
                        Err(e) => eprintln!("Failed to remove rule {}: {}", rule.rule_id(), e),
                    }
                }
            }
        }

        Ok(Response::new(RemoveAgentRulesResponse {
            success: true,
            message: format!("Removed {} rules for agent {}", removed_count, req.agent_id),
            rules_removed: removed_count as i32,
        }))
    }

    /// Get current rule statistics from the bridge
    async fn get_rule_stats(
        &self,
        _request: Request<GetRuleStatsRequest>,
    ) -> Result<Response<GetRuleStatsResponse>, Status> {
        let stats = self.bridge.stats();
        let table_stats = self.bridge.table_stats();

        Ok(Response::new(GetRuleStatsResponse {
            bridge_version: stats.version as i64,
            total_tables: stats.total_tables as i32,
            total_rules: stats.total_rules as i32,
            total_global_rules: stats.total_global_rules as i32,
            total_scoped_rules: stats.total_scoped_rules as i32,
            table_stats: table_stats
                .iter()
                .map(|ts| TableStats {
                    family_id: ts.family_id.family_id().to_string(),
                    layer_id: format!("{}", ts.layer_id),
                    version: ts.version as i64,
                    rule_count: ts.rule_count as i32,
                    global_count: ts.global_count as i32,
                    scoped_count: ts.scoped_count as i32,
                })
                .collect(),
        }))
    }

    /// Enforce rules against an IntentEvent (v1.3)
    async fn enforce(
        &self,
        request: Request<EnforceRequest>,
    ) -> Result<Response<EnforceResponse>, Status> {
        let req = request.into_inner();

        println!("================================================");
        println!("  Enforcing Intent");
        println!("================================================");

        let vector_override = if req.intent_vector.is_empty() {
            None
        } else if req.intent_vector.len() == 128 {
            match req.intent_vector.clone().try_into() {
                Ok(arr) => Some(arr),
                Err(_) => {
                    eprintln!(
                        "Invalid intent_vector length {} (expected 128). Ignoring override.",
                        req.intent_vector.len()
                    );
                    None
                }
            }
        } else {
            eprintln!(
                "Invalid intent_vector length {} (expected 128). Ignoring override.",
                req.intent_vector.len()
            );
            None
        };

        // Call enforcement engine
        let result = self
            .enforcement_engine
            .enforce(&req.intent_event_json, vector_override)
            .await
            .map_err(|e| Status::internal(format!("Enforcement failed: {}", e)))?;

        println!(
            "Enforcement Decision: {}",
            if result.decision == 1 {
                "ALLOW"
            } else {
                "BLOCK"
            }
        );
        println!("Rules Evaluated: {}", result.rules_evaluated);
        println!("=================================================\n");

        Ok(Response::new(EnforceResponse {
            decision: result.decision as i32,
            slice_similarities: result.slice_similarities.to_vec(),
            rules_evaluated: result.rules_evaluated as i32,
            evidence: result
                .evidence
                .iter()
                .map(|ev| RuleEvidence {
                    rule_id: ev.rule_id.clone(),
                    rule_name: ev.rule_name.clone(),
                    decision: ev.decision as i32,
                    similarities: ev.similarities.to_vec(),
                })
                .collect(),
        }))
    }

    /// Query telemetry sessions from hitlogs
    async fn query_telemetry(
        &self,
        request: Request<QueryTelemetryRequest>,
    ) -> Result<Response<QueryTelemetryResponse>, Status> {
        let req = request.into_inner();

        // Build filter from request
        let decision_filter = match req.decision {
            Some(d) if d >= 0 => Some(d as u8),
            _ => None,
        };

        let filter = crate::telemetry::query::QueryFilter {
            agent_id: req.agent_id,
            tenant_id: req.tenant_id,
            decision: decision_filter,
            layer: req.layer,
            start_time_ms: req.start_time_ms.map(|t| t as u64),
            end_time_ms: req.end_time_ms.map(|t| t as u64),
            limit: Some(req.limit.min(500) as usize),
            offset: Some(req.offset as usize),
            ..Default::default()
        };

        // Query hitlogs
        let result = self
            .hitlog_query
            .query(&filter)
            .map_err(|e| Status::internal(format!("Hitlog query failed: {}", e)))?;

        // Convert sessions to summaries
        let summaries: Vec<EnforcementSessionSummary> = result
            .sessions
            .iter()
            .map(|s| EnforcementSessionSummary {
                session_id: s.session_id.clone(),
                agent_id: s.agent_id.clone().unwrap_or_default(),
                tenant_id: s.tenant_id.clone().unwrap_or_default(),
                layer: s.layer.clone(),
                timestamp_ms: s.timestamp_ms as i64,
                final_decision: s.final_decision as i32,
                rules_evaluated_count: s.rules_evaluated.len() as i32,
                duration_us: s.duration_us as i64,
                intent_summary: extract_intent_summary(&s.intent_json),
            })
            .collect();

        Ok(Response::new(QueryTelemetryResponse {
            sessions: summaries,
            total_count: result.total_matched as i32,
        }))
    }

    /// Get full details for a specific session
    async fn get_session(
        &self,
        request: Request<GetSessionRequest>,
    ) -> Result<Response<GetSessionResponse>, Status> {
        let req = request.into_inner();

        // Query for specific session using session_id filter
        let filter = crate::telemetry::query::QueryFilter {
            session_id: Some(req.session_id.clone()),
            limit: Some(1),
            ..Default::default()
        };

        let result = self
            .hitlog_query
            .query(&filter)
            .map_err(|e| Status::internal(format!("Query failed: {}", e)))?;

        // Check if session was found
        if result.sessions.is_empty() {
            return Err(Status::not_found(format!(
                "Session not found: {}",
                req.session_id
            )));
        }

        let session = &result.sessions[0];

        // Serialize session to JSON
        let session_json = serde_json::to_string(session)
            .map_err(|e| Status::internal(format!("Serialization failed: {}", e)))?;

        Ok(Response::new(GetSessionResponse { session_json }))
    }

    /// Handle RefreshRules gRPC request.
    ///
    /// Triggers immediate refresh of rules from warm storage.
    /// Useful when:
    /// - Rules changed externally
    /// - Need to sync hot cache with persistent storage
    /// - Testing refresh mechanism
    async fn refresh_rules(
        &self,
        _request: Request<RefreshRulesRequest>,
    ) -> Result<Response<RefreshRulesResponse>, Status> {
        println!("================================================");
        println!("  RefreshRules RPC called");
        println!("================================================");

        // Call refresh service
        match self.refresh_service.refresh_from_storage().await {
            Ok(stats) => {
                println!(
                    "Refresh completed: {} rules in {}ms",
                    stats.rules_refreshed, stats.duration_ms
                );
                println!("=================================================\n");

                Ok(Response::new(RefreshRulesResponse {
                    success: true,
                    message: format!(
                        "Refreshed {} rules in {}ms",
                        stats.rules_refreshed, stats.duration_ms
                    ),
                    rules_refreshed: stats.rules_refreshed as i32,
                    duration_ms: stats.duration_ms as i64,
                }))
            }
            Err(e) => {
                eprintln!("  ✗ Refresh failed: {}", e);
                println!("=================================================\n");
                Err(Status::internal(format!("Refresh failed: {}", e)))
            }
        }
    }
}

// ================================================================================================
// HELPER FUNCTIONS
// ================================================================================================

/// Extract a summary from the intent JSON (tool_name or action)
fn extract_intent_summary(intent_json: &str) -> String {
    // Try to parse JSON and extract tool_name or action
    if let Ok(intent) = serde_json::from_str::<serde_json::Value>(intent_json) {
        // Try tool_name first (for tool calls)
        if let Some(tool_name) = intent.get("tool_name").and_then(|v| v.as_str()) {
            return tool_name.to_string();
        }
        // Fall back to action
        if let Some(action) = intent.get("action").and_then(|v| v.as_str()) {
            return action.to_string();
        }
    }
    // Default fallback
    "unknown".to_string()
}

// ================================================================================================
// SERVER STARTUP
// ================================================================================================

/// Start the gRPC server for rule installation and enforcement
pub async fn start_grpc_server(
    bridge: Arc<Bridge>,
    port: u16,
    management_plane_url: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let addr = format!("0.0.0.0:{}", port).parse()?;

    println!("Starting Data Plane gRPC server on {}", addr);
    println!("Management Plane URL: {}", management_plane_url);

    let service = DataPlaneService::new(bridge, management_plane_url);

    Server::builder()
        .add_service(DataPlaneServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}
