//! # Rule Bridge Library
//!
//! High-performance rule storage and query engine for multi-layer enforcement.

// Core modules
pub mod api_types;
pub mod bridge;
pub mod enforcement_engine;
pub mod families;
pub mod grpc_server;
pub mod indices;
pub mod rule_converter;
pub mod rule_vector;
pub mod table;
pub mod telemetry;
pub mod types;

// Re-export commonly used types
pub use bridge::Bridge;
pub use rule_vector::RuleVector;
pub use types::{LayerId, RuleFamilyId, RuleInstance, RuleScope};
