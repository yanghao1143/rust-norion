use crate::router::{GenerationMetrics, Route};

#[derive(Debug, Clone)]
pub struct TokenObservation {
    pub token: String,
    pub entropy: f32,
    pub route: Route,
    pub loss: f32,
    pub consistency: f32,
}

#[derive(Debug, Clone)]
pub struct TokenWindowReport {
    pub start_token: usize,
    pub end_token: usize,
    pub metrics: GenerationMetrics,
    pub attention_fraction: f32,
    pub threshold_after: f32,
    pub observations: Vec<TokenObservation>,
}
