//! # Telemetry Module - conntrack for Tupl Data Plane
//!
//! Records every intent evaluation with complete visibility into:
//! - Intent details
//! - Rules evaluated
//! - Per-rule decisions
//! - Final enforcement outcome
//! - Performance metrics
//!
//! Analogous to Linux conntrack, providing complete audit trail of all
//! enforcement decisions.

pub mod recorder;
pub mod session;
pub mod writer;
pub mod query;

pub use recorder::{TelemetryRecorder, TelemetryConfig};
pub use session::{EnforcementSession, SessionEvent, RuleEvaluationEvent};
pub use writer::{HitlogWriter, HitlogConfig, RotationPolicy};
pub use query::{HitlogQuery, QueryFilter};
