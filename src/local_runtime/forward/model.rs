use crate::transformer::AttentionKind;

#[derive(Debug, Clone)]
pub(in crate::local_runtime) struct LocalForwardState {
    pub(in crate::local_runtime) vector: Vec<f32>,
    pub(in crate::local_runtime) layer_summaries: Vec<LocalLayerSummary>,
    pub(in crate::local_runtime) energy: f32,
    pub(in crate::local_runtime) kv_influence: f32,
}

#[derive(Debug, Clone)]
pub(in crate::local_runtime) struct LocalLayerSummary {
    pub(in crate::local_runtime) layer_index: usize,
    pub(in crate::local_runtime) attention: AttentionKind,
    pub(in crate::local_runtime) window_size: usize,
    pub(in crate::local_runtime) compute_fraction: f32,
    pub(in crate::local_runtime) activation: f32,
}
