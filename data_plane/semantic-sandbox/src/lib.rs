// Semantic Sandbox - FFI interface for vector comparison
// Week 1: Dummy implementation that returns hardcoded results

mod compare;

/// FFI-compatible structure for passing intent and boundary vectors
/// along with comparison parameters
#[repr(C)]
pub struct VectorEnvelope {
    // Intent vector (unchanged)
    pub intent: [f32; 128],

    // Boundary anchors (per-slot anchor arrays)
    pub action_anchors: [[f32; 32]; 16],      // Max 16 anchors per slot
    pub action_anchor_count: usize,
    pub resource_anchors: [[f32; 32]; 16],
    pub resource_anchor_count: usize,
    pub data_anchors: [[f32; 32]; 16],
    pub data_anchor_count: usize,
    pub risk_anchors: [[f32; 32]; 16],
    pub risk_anchor_count: usize,

    // Decision parameters (unchanged)
    pub thresholds: [f32; 4],      // action, resource, data, risk
    pub weights: [f32; 4],          // for weighted-avg mode
    pub decision_mode: u8,          // 0 = min, 1 = weighted-avg
    pub global_threshold: f32,      // for weighted-avg mode
}

/// FFI-compatible structure for returning comparison results
#[repr(C)]
pub struct ComparisonResult {
    pub decision: u8,               // 0 = block, 1 = allow
    pub slice_similarities: [f32; 4], // action, resource, data, risk
}

/// Main FFI entry point for comparing vectors
///
/// # Safety
/// Caller must ensure the pointer is valid and points to a properly initialized VectorEnvelope
#[no_mangle]
pub extern "C" fn compare_vectors(envelope: *const VectorEnvelope) -> ComparisonResult {
    // Safety: Caller must ensure valid pointer
    let envelope = unsafe { &*envelope };

    compare::compare(envelope)
}

/// Health check function for testing FFI bridge
#[no_mangle]
pub extern "C" fn health_check() -> u8 {
    1  // Returns 1 if library loaded successfully
}

/// Get version information
#[no_mangle]
pub extern "C" fn get_version() -> u32 {
    1  // Version 0.0.1 encoded as integer
}
