use crate::router::{GenerationMetrics, RouteBudget};

#[derive(Debug, Clone, Copy)]
pub struct DriftInput {
    pub quality: f32,
    pub contradiction_count: usize,
    pub metrics: GenerationMetrics,
    pub route_budget: RouteBudget,
    pub used_memories: usize,
    pub exported_runtime_kv_blocks: usize,
    pub stream_windows: usize,
}
