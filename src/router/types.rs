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

    pub fn as_str(self) -> &'static str {
        match self {
            Self::FastProjection => "fast_projection",
            Self::LocalWindowAttention => "local_window_attention",
            Self::GlobalAttention => "global_attention",
            Self::ConvolutionalFusion => "convolutional_fusion",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdaptiveRouteSource {
    PromptChunk,
    SemanticMemory,
    GistMemory,
    RuntimeKv,
    ReasoningGenome,
    ToolOutput,
}

impl AdaptiveRouteSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PromptChunk => "prompt_chunk",
            Self::SemanticMemory => "semantic_memory",
            Self::GistMemory => "gist_memory",
            Self::RuntimeKv => "runtime_kv",
            Self::ReasoningGenome => "reasoning_genome",
            Self::ToolOutput => "tool_output",
        }
    }

    pub fn prefers_fusion(self) -> bool {
        matches!(
            self,
            Self::SemanticMemory | Self::GistMemory | Self::RuntimeKv | Self::ReasoningGenome
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdaptiveRouteAction {
    Include,
    Compress,
    Defer,
    Skip,
}

impl AdaptiveRouteAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Include => "include",
            Self::Compress => "compress",
            Self::Defer => "defer",
            Self::Skip => "skip",
        }
    }

    pub fn retains_tokens(self) -> bool {
        matches!(self, Self::Include | Self::Compress)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct AdaptiveRouteScoreComponents {
    pub task_intent: f32,
    pub language_mode: f32,
    pub code_mode: f32,
    pub memory_fitness: f32,
    pub recency: f32,
    pub trust: f32,
    pub compute_cost: f32,
    pub reward_history: f32,
}

impl AdaptiveRouteScoreComponents {
    pub fn new(
        task_intent: f32,
        language_mode: f32,
        code_mode: f32,
        memory_fitness: f32,
        recency: f32,
        trust: f32,
        compute_cost: f32,
        reward_history: f32,
    ) -> Self {
        Self {
            task_intent,
            language_mode,
            code_mode,
            memory_fitness,
            recency,
            trust,
            compute_cost,
            reward_history,
        }
        .clamp()
    }

    pub fn clamp(self) -> Self {
        Self {
            task_intent: finite_unit_or_zero(self.task_intent),
            language_mode: finite_unit_or_zero(self.language_mode),
            code_mode: finite_unit_or_zero(self.code_mode),
            memory_fitness: finite_unit_or_zero(self.memory_fitness),
            recency: finite_unit_or_zero(self.recency),
            trust: finite_unit_or_zero(self.trust),
            compute_cost: finite_unit_or_zero(self.compute_cost),
            reward_history: finite_unit_or_zero(self.reward_history),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AdaptiveRouteCandidate {
    pub id: String,
    pub source: AdaptiveRouteSource,
    pub estimated_tokens: usize,
    pub anchor_required: bool,
    pub components: AdaptiveRouteScoreComponents,
}

impl AdaptiveRouteCandidate {
    pub fn new(
        id: impl Into<String>,
        source: AdaptiveRouteSource,
        estimated_tokens: usize,
        components: AdaptiveRouteScoreComponents,
    ) -> Self {
        Self {
            id: id.into(),
            source,
            estimated_tokens,
            anchor_required: false,
            components: components.clamp(),
        }
    }

    pub fn with_anchor_required(mut self, anchor_required: bool) -> Self {
        self.anchor_required = anchor_required;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AdaptiveRouteDecision {
    pub candidate_id: String,
    pub source: AdaptiveRouteSource,
    pub estimated_tokens: usize,
    pub retained_tokens: usize,
    pub anchor_required: bool,
    pub components: AdaptiveRouteScoreComponents,
    pub score: f32,
    pub threshold: f32,
    pub route: Route,
    pub action: AdaptiveRouteAction,
    pub compute_pressure: f32,
    pub reason: String,
}

impl AdaptiveRouteDecision {
    pub fn saved_tokens(&self) -> usize {
        self.estimated_tokens.saturating_sub(self.retained_tokens)
    }

    pub fn summary(&self) -> String {
        format!(
            "id={} source={} action={} route={} score={:.3} threshold={:.3} retained={} saved={} anchor={} task={:.3} language={:.3} code={:.3} fitness={:.3} recency={:.3} trust={:.3} cost={:.3} reward={:.3}",
            self.candidate_id,
            self.source.as_str(),
            self.action.as_str(),
            self.route.as_str(),
            self.score,
            self.threshold,
            self.retained_tokens,
            self.saved_tokens(),
            self.anchor_required,
            self.components.task_intent,
            self.components.language_mode,
            self.components.code_mode,
            self.components.memory_fitness,
            self.components.recency,
            self.components.trust,
            self.components.compute_cost,
            self.components.reward_history
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AdaptiveRoutingPlan {
    pub profile: TaskProfile,
    pub threshold: f32,
    pub candidates: usize,
    pub include: usize,
    pub compress: usize,
    pub defer: usize,
    pub skip: usize,
    pub input_tokens: usize,
    pub retained_tokens: usize,
    pub saved_tokens: usize,
    pub min_score: f32,
    pub max_score: f32,
    pub average_score: f32,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
    pub decisions: Vec<AdaptiveRouteDecision>,
}

impl AdaptiveRoutingPlan {
    pub fn empty(profile: TaskProfile) -> Self {
        Self::from_decisions(profile, 0.52, Vec::new())
    }

    pub fn from_decisions(
        profile: TaskProfile,
        threshold: f32,
        decisions: Vec<AdaptiveRouteDecision>,
    ) -> Self {
        let mut include = 0usize;
        let mut compress = 0usize;
        let mut defer = 0usize;
        let mut skip = 0usize;
        let mut input_tokens = 0usize;
        let mut retained_tokens = 0usize;
        let mut score_sum = 0.0f32;
        let mut min_score = f32::INFINITY;
        let mut max_score = f32::NEG_INFINITY;

        for decision in &decisions {
            match decision.action {
                AdaptiveRouteAction::Include => include = include.saturating_add(1),
                AdaptiveRouteAction::Compress => compress = compress.saturating_add(1),
                AdaptiveRouteAction::Defer => defer = defer.saturating_add(1),
                AdaptiveRouteAction::Skip => skip = skip.saturating_add(1),
            }
            input_tokens = input_tokens.saturating_add(decision.estimated_tokens);
            retained_tokens = retained_tokens.saturating_add(decision.retained_tokens);
            score_sum += decision.score;
            min_score = min_score.min(decision.score);
            max_score = max_score.max(decision.score);
        }

        let candidates = decisions.len();
        Self {
            profile,
            threshold: finite_unit_or_zero(threshold),
            candidates,
            include,
            compress,
            defer,
            skip,
            input_tokens,
            retained_tokens,
            saved_tokens: input_tokens.saturating_sub(retained_tokens),
            min_score: if candidates == 0 { 0.0 } else { min_score },
            max_score: if candidates == 0 { 0.0 } else { max_score },
            average_score: if candidates == 0 {
                0.0
            } else {
                score_sum / candidates as f32
            },
            read_only: true,
            write_allowed: false,
            applied: false,
            decisions,
        }
    }

    pub fn decision_count_matches(self: &Self) -> bool {
        self.include
            .saturating_add(self.compress)
            .saturating_add(self.defer)
            .saturating_add(self.skip)
            == self.candidates
    }

    pub fn token_accounting_matches(self: &Self) -> bool {
        self.retained_tokens.saturating_add(self.saved_tokens) == self.input_tokens
    }

    pub fn anchors_retained(self: &Self) -> bool {
        self.decisions
            .iter()
            .filter(|decision| decision.anchor_required)
            .all(|decision| decision.action.retains_tokens())
    }

    pub fn selected_route_summaries(&self) -> Vec<String> {
        let mut routes = Vec::new();
        for decision in &self.decisions {
            let route = decision.route.as_str().to_owned();
            if !routes.contains(&route) {
                routes.push(route);
            }
        }
        routes
    }

    pub fn action_summaries(&self) -> Vec<String> {
        let mut actions = Vec::new();
        for decision in &self.decisions {
            let action = decision.action.as_str().to_owned();
            if !actions.contains(&action) {
                actions.push(action);
            }
        }
        actions
    }

    pub fn score_summaries(&self, limit: usize) -> Vec<String> {
        self.decisions
            .iter()
            .take(limit)
            .map(AdaptiveRouteDecision::summary)
            .collect()
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

fn finite_unit_or_zero(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}
