use crate::adaptive_state::{EvolutionLedger, LiveInferenceEvolution};
use crate::agent_team::AgentTeamPlan;
use crate::drift::DriftReport;
use crate::experience::{ExperienceMatch, ExperienceRuntimeTokenMetrics};
use crate::experience_replay::ExperienceReplayReport;
use crate::gist_memory::GistRecord;
use crate::hardware::HardwarePlan;
use crate::hierarchy::{HierarchyWeights, TaskProfile};
use crate::infini_memory::InfiniMemoryPlan;
use crate::kv_cache::{
    MemoryCompactionPolicy, MemoryCompactionReport, MemoryMatch, MemoryRetentionPolicy,
    MemoryUpdateReport, RetentionReport,
};
use crate::memory_admission::MemoryAdmissionPreview;
use crate::process_reward::ProcessRewardReport;
use crate::reasoning_genome::{DnaSplicePreview, GenomeExpression};
use crate::recursive_scheduler::RecursiveSchedule;
use crate::reflection::{DraftToken, InferenceDraft, ReflectionReport, RuntimeDiagnostics};
use crate::router::{AdaptiveRoutingPlan, GenerationMetrics, RouteBudget};
use crate::runtime::RuntimeAdapterObservation;
use crate::tiered_cache::{TierMigration, TieredCachePlan};
use crate::token_stream::TokenWindowReport;
use crate::toolsmith::ToolsmithPlan;
use crate::transformer::TransformerRefactorPlan;
#[derive(Debug, Clone)]
pub struct InferenceRequest {
    pub prompt: String,
    pub profile: TaskProfile,
    pub max_tokens: Option<usize>,
}

impl InferenceRequest {
    pub fn new(prompt: impl Into<String>, profile: TaskProfile) -> Self {
        Self {
            prompt: prompt.into(),
            profile,
            max_tokens: None,
        }
    }

    pub fn with_max_tokens(mut self, max_tokens: Option<usize>) -> Self {
        self.max_tokens = max_tokens.map(|value| value.max(1));
        self
    }
}

#[derive(Debug, Clone)]
pub struct GenerationContext<'a> {
    pub prompt: &'a str,
    pub profile: TaskProfile,
    pub memories: &'a [MemoryMatch],
    pub route_budget: RouteBudget,
    pub hierarchy: HierarchyWeights,
    pub tier_plan: &'a TieredCachePlan,
    pub infini_memory_plan: &'a InfiniMemoryPlan,
    pub recursive_schedule: &'a RecursiveSchedule,
    pub hardware_plan: &'a HardwarePlan,
    pub experiences: &'a [ExperienceMatch],
    pub toolsmith_plan: &'a ToolsmithPlan,
    pub agent_team_plan: &'a AgentTeamPlan,
    pub transformer_plan: &'a TransformerRefactorPlan,
}

impl<'a> GenerationContext<'a> {
    pub(super) fn with_prompt<'b>(&'b self, prompt: &'b str) -> GenerationContext<'b>
    where
        'a: 'b,
    {
        GenerationContext {
            prompt,
            profile: self.profile,
            memories: self.memories,
            route_budget: self.route_budget,
            hierarchy: self.hierarchy,
            tier_plan: self.tier_plan,
            infini_memory_plan: self.infini_memory_plan,
            recursive_schedule: self.recursive_schedule,
            hardware_plan: self.hardware_plan,
            experiences: self.experiences,
            toolsmith_plan: self.toolsmith_plan,
            agent_team_plan: self.agent_team_plan,
            transformer_plan: self.transformer_plan,
        }
    }
}

pub trait InferenceBackend {
    fn configure_generation(&mut self, _max_tokens: Option<usize>) {}

    fn configure_runtime_endpoint_override(
        &mut self,
        _base_url: Option<&str>,
    ) -> Result<bool, String> {
        Ok(false)
    }

    fn runtime_endpoint_override_active(&self) -> Option<&str> {
        None
    }

    fn runtime_native_context_window(&self) -> Option<usize> {
        None
    }

    fn embed_text(&mut self, _text: &str) -> Option<Vec<f32>> {
        None
    }

    fn generate(&mut self, context: GenerationContext<'_>) -> InferenceDraft;

    fn generate_stream(
        &mut self,
        context: GenerationContext<'_>,
        on_token: &mut dyn FnMut(&DraftToken),
    ) -> InferenceDraft {
        let draft = self.generate(context);
        for token in &draft.tokens {
            on_token(token);
        }
        draft
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingSource {
    Runtime,
    Fallback,
}

impl EmbeddingSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Runtime => "runtime",
            Self::Fallback => "fallback",
        }
    }
}

impl Default for EmbeddingSource {
    fn default() -> Self {
        Self::Fallback
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EmbeddingCallDiagnostics {
    pub source: EmbeddingSource,
    pub dimensions: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EmbeddingDiagnostics {
    pub query: EmbeddingCallDiagnostics,
    pub memory_write: Option<EmbeddingCallDiagnostics>,
    pub gist_writes: Vec<EmbeddingCallDiagnostics>,
    pub runtime_calls: usize,
    pub fallback_calls: usize,
}

impl EmbeddingDiagnostics {
    pub(super) fn from_query(query: EmbeddingCallDiagnostics) -> Self {
        let mut diagnostics = Self {
            query,
            ..Self::default()
        };
        diagnostics.record_call(query);
        diagnostics
    }

    pub(super) fn record_memory_write(&mut self, call: EmbeddingCallDiagnostics) {
        self.memory_write = Some(call);
        self.record_call(call);
    }

    pub(super) fn record_gist_write(&mut self, call: EmbeddingCallDiagnostics) {
        self.gist_writes.push(call);
        self.record_call(call);
    }

    fn record_call(&mut self, call: EmbeddingCallDiagnostics) {
        match call.source {
            EmbeddingSource::Runtime => self.runtime_calls += 1,
            EmbeddingSource::Fallback => self.fallback_calls += 1,
        }
    }

    pub fn runtime_embedding_available(&self) -> bool {
        self.runtime_calls > 0
    }

    pub fn fallback_embedding_used(&self) -> bool {
        self.fallback_calls > 0
    }

    pub fn total_calls(&self) -> usize {
        1 + usize::from(self.memory_write.is_some()) + self.gist_writes.len()
    }

    pub fn gist_write_runtime_calls(&self) -> usize {
        self.gist_writes
            .iter()
            .filter(|call| call.source == EmbeddingSource::Runtime)
            .count()
    }

    pub fn gist_write_fallback_calls(&self) -> usize {
        self.gist_writes
            .iter()
            .filter(|call| call.source == EmbeddingSource::Fallback)
            .count()
    }
}

#[derive(Debug, Clone)]
pub(super) struct EmbeddingCall {
    pub(super) diagnostics: EmbeddingCallDiagnostics,
    pub(super) vector: Vec<f32>,
}

#[derive(Debug, Clone)]
pub struct InferenceOutcome {
    pub raw_answer: String,
    pub answer: String,
    pub report: ReflectionReport,
    pub auto_replay_report: Option<ExperienceReplayReport>,
    pub metrics: GenerationMetrics,
    pub runtime_token_metrics: RuntimeTokenMetrics,
    pub embedding_diagnostics: EmbeddingDiagnostics,
    pub runtime_diagnostics: RuntimeDiagnostics,
    pub runtime_adapter_observations: Vec<RuntimeAdapterObservation>,
    pub recursive_runtime_calls: usize,
    pub route_budget: RouteBudget,
    pub adaptive_route_plan: AdaptiveRoutingPlan,
    pub hierarchy: HierarchyWeights,
    pub tier_plan: TieredCachePlan,
    pub tier_migrations: Vec<TierMigration>,
    pub infini_memory_plan: InfiniMemoryPlan,
    pub recursive_schedule: RecursiveSchedule,
    pub hardware_plan: HardwarePlan,
    pub transformer_plan: TransformerRefactorPlan,
    pub toolsmith_plan: ToolsmithPlan,
    pub agent_team_plan: AgentTeamPlan,
    pub stream_reports: Vec<TokenWindowReport>,
    pub used_memories: Vec<MemoryMatch>,
    pub memory_feedback: MemoryFeedbackReport,
    pub used_experiences: Vec<ExperienceMatch>,
    pub gist_records: Vec<GistRecord>,
    pub stored_memory_id: Option<u64>,
    pub stored_gist_memory_ids: Vec<u64>,
    pub exported_runtime_kv_blocks: usize,
    pub stored_runtime_kv_memory_ids: Vec<u64>,
    pub memory_admission: MemoryAdmissionPreview,
    pub drift_report: DriftReport,
    pub process_reward: ProcessRewardReport,
    pub reasoning_genome: GenomeExpression,
    pub reasoning_genome_splice: DnaSplicePreview,
    pub memory_retention_policy: MemoryRetentionPolicy,
    pub memory_compaction_policy: MemoryCompactionPolicy,
    pub retention_report: RetentionReport,
    pub memory_compaction_report: MemoryCompactionReport,
    pub experience_id: u64,
    pub router_threshold_after: f32,
    pub live_evolution: LiveInferenceEvolution,
    pub evolution_ledger: EvolutionLedger,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MemoryFeedbackReport {
    pub reinforced: usize,
    pub penalized: usize,
    pub reinforcement_amount: f32,
    pub penalty_amount: f32,
    pub updates: Vec<MemoryUpdateReport>,
}

impl MemoryFeedbackReport {
    pub fn total_updates(&self) -> usize {
        self.reinforced + self.penalized
    }

    pub fn record_reinforcement(&mut self, amount: f32, update: MemoryUpdateReport) {
        self.reinforced += 1;
        self.reinforcement_amount += amount;
        self.updates.push(update);
    }

    pub fn record_penalty(&mut self, amount: f32, update: MemoryUpdateReport) {
        self.penalized += 1;
        self.penalty_amount += amount;
        self.updates.push(update);
    }

    pub fn applied_updates(&self) -> usize {
        self.updates
            .iter()
            .filter(|update| update.was_applied())
            .count()
    }

    pub fn removed_updates(&self) -> usize {
        self.updates.iter().filter(|update| update.removed).count()
    }

    pub fn strength_delta(&self) -> f32 {
        self.updates
            .iter()
            .map(|update| update.strength_delta.abs())
            .sum()
    }

    pub fn missing_updates(&self) -> usize {
        self.updates
            .iter()
            .filter(|update| !update.was_applied())
            .count()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuntimeTokenMetrics {
    pub token_count: usize,
    pub entropy_count: usize,
    pub logprob_count: usize,
    pub average_entropy: Option<f32>,
    pub average_neg_logprob: Option<f32>,
    pub uncertainty_perplexity: Option<f32>,
}

impl RuntimeTokenMetrics {
    pub fn from_draft(draft: &InferenceDraft) -> Self {
        let mut entropy_total = 0.0;
        let mut entropy_count = 0;
        let mut neg_logprob_total = 0.0;
        let mut logprob_count = 0;
        let mut loss_total = 0.0;
        let mut loss_count = 0;

        for token in &draft.tokens {
            let entropy = token.entropy.and_then(bounded_entropy);
            let neg_logprob = token.logprob.and_then(bounded_neg_logprob);

            if let Some(entropy) = entropy {
                entropy_total += entropy;
                entropy_count += 1;
            }
            if let Some(neg_logprob) = neg_logprob {
                neg_logprob_total += neg_logprob;
                logprob_count += 1;
            }

            match (entropy, neg_logprob) {
                (Some(entropy), Some(neg_logprob)) => {
                    loss_total += 2.0 + entropy * 4.0 + neg_logprob;
                    loss_count += 1;
                }
                (Some(entropy), None) => {
                    loss_total += 2.0 + entropy * 4.0;
                    loss_count += 1;
                }
                (None, Some(neg_logprob)) => {
                    loss_total += 2.0 + neg_logprob;
                    loss_count += 1;
                }
                (None, None) => {}
            }
        }

        Self {
            token_count: draft.tokens.len(),
            entropy_count,
            logprob_count,
            average_entropy: average(entropy_total, entropy_count),
            average_neg_logprob: average(neg_logprob_total, logprob_count),
            uncertainty_perplexity: average(loss_total, loss_count),
        }
    }

    pub fn has_uncertainty_signal(self) -> bool {
        self.uncertainty_perplexity.is_some()
            || self.average_entropy.is_some()
            || self.average_neg_logprob.is_some()
    }
}

impl From<RuntimeTokenMetrics> for ExperienceRuntimeTokenMetrics {
    fn from(metrics: RuntimeTokenMetrics) -> Self {
        Self {
            token_count: metrics.token_count,
            entropy_count: metrics.entropy_count,
            logprob_count: metrics.logprob_count,
            average_entropy: metrics.average_entropy,
            average_neg_logprob: metrics.average_neg_logprob,
            uncertainty_perplexity: metrics.uncertainty_perplexity,
        }
    }
}

pub(super) fn bounded_entropy(value: f32) -> Option<f32> {
    value.is_finite().then(|| value.clamp(0.0, 4.0))
}

pub(super) fn bounded_neg_logprob(value: f32) -> Option<f32> {
    let value = -value;
    value.is_finite().then(|| value.clamp(0.0, 12.0))
}

fn average(total: f32, count: usize) -> Option<f32> {
    if count == 0 {
        None
    } else {
        Some(total / count as f32)
    }
}
