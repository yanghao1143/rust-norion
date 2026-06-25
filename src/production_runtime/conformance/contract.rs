#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProductionKernelConformanceGate {
    pub require_tokens: bool,
    pub require_trace: bool,
    pub require_forward_energy: bool,
    pub require_kv_influence: bool,
    pub require_layer_mode_coverage: bool,
    pub require_kv_export_when_enabled: bool,
    pub require_runtime_kv_segment_signal: bool,
}

impl Default for ProductionKernelConformanceGate {
    fn default() -> Self {
        Self {
            require_tokens: true,
            require_trace: true,
            require_forward_energy: true,
            require_kv_influence: true,
            require_layer_mode_coverage: true,
            require_kv_export_when_enabled: true,
            require_runtime_kv_segment_signal: true,
        }
    }
}
