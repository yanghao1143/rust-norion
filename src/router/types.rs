use crate::hierarchy::{HierarchyWeights, TaskProfile};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    FastProjection,
    LocalWindowAttention,
    GlobalAttention,
    ConvolutionalFusion,
}

impl Route {
    pub fn uses_attention_budget(self) -> bool {
        self != Self::FastProjection
    }
}

#[derive(Debug, Clone, Copy)]
pub struct GenerationMetrics {
    pub perplexity: f32,
    pub semantic_consistency: f32,
    pub contradiction_count: usize,
    pub token_count: usize,
}

impl GenerationMetrics {
    pub fn quality_score(self) -> f32 {
        let perplexity = if self.perplexity.is_finite() {
            self.perplexity.max(0.0)
        } else {
            f32::INFINITY
        };
        let perplexity_score = if perplexity.is_finite() {
            (1.0 / (1.0 + perplexity / 12.0)).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let consistency_score = if self.semantic_consistency.is_finite() {
            self.semantic_consistency.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let contradiction_penalty = (self.contradiction_count as f32 * 0.18).min(0.72);
        ((perplexity_score * 0.35) + (consistency_score * 0.65) - contradiction_penalty)
            .clamp(0.0, 1.0)
    }
}

#[derive(Debug, Clone)]
pub struct RoutingDecision {
    pub token: String,
    pub entropy: f32,
    pub score: f32,
    pub route: Route,
}

#[derive(Debug, Clone, Copy)]
pub struct RoutingContext {
    pub profile: TaskProfile,
    pub context_tokens: usize,
    pub cache_hit_rate: f32,
    pub latency_budget_ms: Option<u64>,
    pub hardware_pressure: f32,
    pub compute_headroom: f32,
    pub hierarchy: HierarchyWeights,
}

impl Default for RoutingContext {
    fn default() -> Self {
        Self {
            profile: TaskProfile::General,
            context_tokens: 0,
            cache_hit_rate: 0.0,
            latency_budget_ms: None,
            hardware_pressure: 0.0,
            compute_headroom: 0.5,
            hierarchy: HierarchyWeights::default(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RouteBudget {
    pub threshold: f32,
    pub attention_tokens: usize,
    pub fast_tokens: usize,
    pub attention_fraction: f32,
}

impl RouteBudget {
    pub fn from_decisions(threshold: f32, decisions: &[RoutingDecision]) -> Self {
        let attention_tokens = decisions
            .iter()
            .filter(|decision| decision.route.uses_attention_budget())
            .count();
        let fast_tokens = decisions.len().saturating_sub(attention_tokens);
        let total = decisions.len().max(1) as f32;

        Self {
            threshold,
            attention_tokens,
            fast_tokens,
            attention_fraction: attention_tokens as f32 / total,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RouterState {
    pub threshold: f32,
    pub observations: u64,
    pub profile_thresholds: ProfileThresholds,
    pub profile_observations: ProfileObservations,
}

#[derive(Debug, Clone, Copy)]
pub struct ProfileThresholds {
    pub general: f32,
    pub coding: f32,
    pub writing: f32,
    pub long_document: f32,
}

impl ProfileThresholds {
    pub fn from_single(threshold: f32) -> Self {
        Self {
            general: threshold,
            coding: threshold,
            writing: threshold,
            long_document: threshold,
        }
    }

    pub fn get(self, profile: TaskProfile) -> f32 {
        match profile {
            TaskProfile::General => self.general,
            TaskProfile::Coding => self.coding,
            TaskProfile::Writing => self.writing,
            TaskProfile::LongDocument => self.long_document,
        }
    }

    pub fn set(&mut self, profile: TaskProfile, threshold: f32) {
        match profile {
            TaskProfile::General => self.general = threshold,
            TaskProfile::Coding => self.coding = threshold,
            TaskProfile::Writing => self.writing = threshold,
            TaskProfile::LongDocument => self.long_document = threshold,
        }
    }

    pub fn clamp(self, min_threshold: f32, max_threshold: f32) -> Self {
        Self {
            general: self.general.clamp(min_threshold, max_threshold),
            coding: self.coding.clamp(min_threshold, max_threshold),
            writing: self.writing.clamp(min_threshold, max_threshold),
            long_document: self.long_document.clamp(min_threshold, max_threshold),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ProfileObservations {
    pub general: u64,
    pub coding: u64,
    pub writing: u64,
    pub long_document: u64,
}

impl ProfileObservations {
    pub fn from_single(observations: u64) -> Self {
        Self {
            general: observations,
            coding: 0,
            writing: 0,
            long_document: 0,
        }
    }

    pub fn get(self, profile: TaskProfile) -> u64 {
        match profile {
            TaskProfile::General => self.general,
            TaskProfile::Coding => self.coding,
            TaskProfile::Writing => self.writing,
            TaskProfile::LongDocument => self.long_document,
        }
    }

    pub fn bump(&mut self, profile: TaskProfile) {
        match profile {
            TaskProfile::General => self.general = self.general.saturating_add(1),
            TaskProfile::Coding => self.coding = self.coding.saturating_add(1),
            TaskProfile::Writing => self.writing = self.writing.saturating_add(1),
            TaskProfile::LongDocument => {
                self.long_document = self.long_document.saturating_add(1);
            }
        }
    }
}
