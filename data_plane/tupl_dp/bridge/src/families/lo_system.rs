// L0 System layer Rule families
// Defines rule structures for the system layer:
//! - NetworkEgressRule: Control network destinations
//! - SidecarSpawnRule: Restrict sidecar launches

use crate::types::{
    now_ms, LayerId, NetworkProtocol, RuleAction, RuleFamilyId, RuleInstance, RuleScope,
};
use std::sync::Arc;

// ================================================================================================
// NETWORK EGRESS RULE
// ================================================================================================

/// Controls which network destinations an agent or sidecar can contact
///
/// # Fields
/// - `dest_domains`: List of destination domain patterns
/// - `port_range`: Optional port range constraint
/// - `protocol`: Network protocol (TCP, UDP, HTTP, HTTPS)
/// - `action`: Action to take (ALLOW, DENY, REDIRECT)
/// - `redirect_target`: Optional redirect destination
///
/// # Matching
/// - Fast match: (src_agent_id, dest_domain_pattern, port)
/// - Syntactic match: (header["Host"], header["Port"])
///

#[derive(Debug, Clone)]
pub struct NetworkEgressRule {
    /// Unique rule identifier
    pub rule_id: String,
    /// Priority (higher=evaluated first)
    pub priority: u32,
    /// Rule scope (which agnets this applies to)
    pub scope: RuleScope,
    /// Destination domain patterns (supports wildcards)
    pub dest_domains: Vec<String>,

    /// Port range constraint (min, max)
    pub port_range: Option<(u16, u16)>,

    /// Network protocol
    pub protocol: NetworkProtocol,

    /// Action to take
    pub action: RuleAction,

    /// Redirect target (if action = REDIRECT)
    pub redirect_target: Option<String>,

    /// Creation timestamp
    pub created_at: u64,

    /// Optional description
    pub description: Option<String>,

    /// Whether rule is enabled
    pub enabled: bool,
}

impl NetworkEgressRule {
    /// Creates a new NetworkEgressRule with defaults
    pub fn new(rule_id: impl Into<String>) -> Self {
        NetworkEgressRule {
            rule_id: rule_id.into(),
            priority: 0,
            scope: RuleScope::global(),
            dest_domains: vec![],
            port_range: None,
            protocol: NetworkProtocol::default(),
            action: RuleAction::Deny,
            redirect_target: None,
            created_at: now_ms(),
            description: None,
            enabled: true,
        }
    }

    // Builder methods
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_scope(mut self, scope: RuleScope) -> Self {
        self.scope = scope;
        self
    }

    pub fn for_agent(mut self, agent_id: impl Into<String>) -> Self {
        self.scope = RuleScope::for_agent(agent_id.into());
        self
    }

    pub fn with_dest_domains(mut self, domains: Vec<String>) -> Self {
        self.dest_domains = domains;
        self
    }

    pub fn with_port_range(mut self, min: u16, max: u16) -> Self {
        self.port_range = Some((min, max));
        self
    }

    pub fn with_protocol(mut self, protocol: NetworkProtocol) -> Self {
        self.protocol = protocol;
        self
    }

    pub fn with_action(mut self, action: RuleAction) -> Self {
        self.action = action;
        self
    }

    pub fn with_redirect_target(mut self, target: impl Into<String>) -> Self {
        self.redirect_target = Some(target.into());
        self
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Checks if a destination domain matches any pattern in this rule
    pub fn matches_domain(&self, domain: &str) -> bool {
        for pattern in &self.dest_domains {
            if Self::domain_matches(pattern, domain) {
                return true;
            }
        }
        false
    }

    /// Checks if a port falls within the rule's port range
    pub fn matches_port(&self, port: u16) -> bool {
        match self.port_range {
            Some((min, max)) => port >= min && port <= max,
            None => true, // No constraint = matches all ports
        }
    }

    /// Simple wildcard matching for domains
    fn domain_matches(pattern: &str, domain: &str) -> bool {
        if pattern == "*" {
            return true;
        }

        if pattern.starts_with("*.") {
            let suffix = &pattern[2..];
            domain.ends_with(suffix) || domain == suffix
        } else {
            pattern == domain
        }
    }
}

impl RuleInstance for NetworkEgressRule {
    fn rule_id(&self) -> &str {
        &self.rule_id
    }

    fn priority(&self) -> u32 {
        self.priority
    }

    fn scope(&self) -> &RuleScope {
        &self.scope
    }

    fn family_id(&self) -> RuleFamilyId {
        RuleFamilyId::NetworkEgress
    }

    fn created_at(&self) -> u64 {
        self.created_at
    }

    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }
}

// ================================================================================================
// SIDECAR SPAWN RULE
// ================================================================================================

/// Restricts which sidecars an agent may launch
///
/// # Fields
/// - `allowed_images`: List of allowed container images
/// - `max_ttl`: Maximum time-to-live in seconds
/// - `max_instances`: Maximum number of concurrent instances
/// - `cpu_limit`: CPU limit (millicores)
/// - `mem_limit`: Memory limit (MB)
///
/// # Matching
/// - Fast match: (agent_id)
/// - Semantic match: Check if requested_image in allowed_images
///

#[derive(Debug, Clone)]
pub struct SidecarSpawnRule {
    /// Unique rule identifier
    pub rule_id: String,

    /// Priority (higher = evaluated first)
    pub priority: u32,

    /// Rule scope (which agents this applies to)
    pub scope: RuleScope,

    /// Allowed container images
    pub allowed_images: Vec<String>,

    /// Maximum TTL in seconds
    pub max_ttl: Option<u32>,

    /// Maximum concurrent instances
    pub max_instances: Option<u32>,

    /// CPU limit in millicores
    pub cpu_limit: Option<u32>,

    /// Memory limit in MB
    pub mem_limit: Option<u32>,

    /// Creation timestamp
    pub created_at: u64,

    /// Optional description
    pub description: Option<String>,

    /// Whether rule is enabled
    pub enabled: bool,
}

impl SidecarSpawnRule {
    /// Creates a new SidecarSpawnRule with defaults (deny all)
    pub fn new(rule_id: impl Into<String>) -> Self {
        SidecarSpawnRule {
            rule_id: rule_id.into(),
            priority: 0,
            scope: RuleScope::global(),
            allowed_images: vec![],
            max_ttl: None,
            max_instances: None,
            cpu_limit: None,
            mem_limit: None,
            created_at: now_ms(),
            description: None,
            enabled: true,
        }
    }

    // Builder methods
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_scope(mut self, scope: RuleScope) -> Self {
        self.scope = scope;
        self
    }

    pub fn for_agent(mut self, agent_id: impl Into<String>) -> Self {
        self.scope = RuleScope::for_agent(agent_id.into());
        self
    }

    pub fn with_allowed_images(mut self, images: Vec<String>) -> Self {
        self.allowed_images = images;
        self
    }

    pub fn with_max_ttl(mut self, ttl: u32) -> Self {
        self.max_ttl = Some(ttl);
        self
    }

    pub fn with_max_instances(mut self, instances: u32) -> Self {
        self.max_instances = Some(instances);
        self
    }

    pub fn with_cpu_limit(mut self, limit: u32) -> Self {
        self.cpu_limit = Some(limit);
        self
    }

    pub fn with_mem_limit(mut self, limit: u32) -> Self {
        self.mem_limit = Some(limit);
        self
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Checks if an image is allowed by this rule
    pub fn is_image_allowed(&self, image: &str) -> bool {
        self.allowed_images
            .iter()
            .any(|allowed| Self::image_matches(allowed, image))
    }

    /// Simple image matching (supports wildcards and version patterns)
    fn image_matches(pattern: &str, image: &str) -> bool {
        if pattern == "*" {
            return true;
        }

        // Exact match
        if pattern == image {
            return true;
        }

        // Wildcard matching for versions (e.g., "redis:*")
        if pattern.ends_with(":*") {
            let base = &pattern[..pattern.len() - 2];
            image.starts_with(base)
        } else {
            false
        }
    }
}

impl RuleInstance for SidecarSpawnRule {
    fn rule_id(&self) -> &str {
        &self.rule_id
    }

    fn priority(&self) -> u32 {
        self.priority
    }

    fn scope(&self) -> &RuleScope {
        &self.scope
    }

    fn family_id(&self) -> RuleFamilyId {
        RuleFamilyId::SidecarSpawn
    }

    fn created_at(&self) -> u64 {
        self.created_at
    }

    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }
}
