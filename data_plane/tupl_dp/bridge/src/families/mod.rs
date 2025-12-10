//! # Rule Families Module
//!
//! Contains all 14 rule family implementations across 7 layers.

pub mod l1_input;
pub mod l2_planner;
pub mod l3_modelio;
pub mod l4_tool_gateway;
pub mod l5_rag;
pub mod l6_egress;
pub mod lo_system;

// Re-export all rule types
pub use l1_input::{InputSanitizationRule, InputSchemaRule};
pub use l2_planner::{PromptAssemblyRule, PromptLengthRule};
pub use l3_modelio::{ModelOutputEscalateRule, ModelOutputScanRule};
pub use l4_tool_gateway::{EnforcementMode, ToolParamConstraintRule, ToolWhitelistRule};
pub use l5_rag::{RAGDocSensitivityRule, RAGSourceRule};
pub use l6_egress::{OutputAuditRule, OutputPIIRule};
pub use lo_system::{NetworkEgressRule, SidecarSpawnRule};
