use crate::adaptive_state::LiveInferenceEvolution;
use crate::gist_memory::GistRecord;
use crate::hierarchy::{HierarchyWeights, TaskProfile};
use crate::process_reward::{ProcessRewardReport, RewardAction};
use crate::reflection::{ReflectionIssue, RuntimeDiagnostics};
use crate::router::RouteBudget;

#[derive(Debug, Clone)]
pub struct ExperienceInput {
    pub prompt: String,
    pub profile: TaskProfile,
    pub lesson: String,
    pub quality: f32,
    pub contradictions: Vec<String>,
    pub reflection_issues: Vec<ReflectionIssue>,
    pub revision_actions: Vec<String>,
    pub stored_memory_id: Option<u64>,
    pub router_threshold_after: f32,
    pub stream_windows: usize,
    pub route_budget: RouteBudget,
    pub hierarchy: HierarchyWeights,
    pub used_memory_ids: Vec<u64>,
    pub gist_records: Vec<GistRecord>,
    pub gist_memory_ids: Vec<u64>,
    pub stored_runtime_kv_memory_ids: Vec<u64>,
    pub runtime_diagnostics: RuntimeDiagnostics,
    pub runtime_token_metrics: ExperienceRuntimeTokenMetrics,
    pub process_reward: ProcessRewardReport,
    pub live_evolution: LiveInferenceEvolution,
}

#[derive(Debug, Clone)]
pub struct ExperienceRecord {
    pub id: u64,
    pub prompt: String,
    pub profile: TaskProfile,
    pub lesson: String,
    pub quality: f32,
    pub contradictions: Vec<String>,
    pub reflection_issues: Vec<ReflectionIssue>,
    pub revision_actions: Vec<String>,
    pub stored_memory_id: Option<u64>,
    pub router_threshold_after: f32,
    pub stream_windows: usize,
    pub route_budget: RouteBudget,
    pub hierarchy: HierarchyWeights,
    pub used_memory_ids: Vec<u64>,
    pub gist_records: Vec<GistRecord>,
    pub gist_memory_ids: Vec<u64>,
    pub stored_runtime_kv_memory_ids: Vec<u64>,
    pub runtime_diagnostics: RuntimeDiagnostics,
    pub runtime_token_metrics: ExperienceRuntimeTokenMetrics,
    pub process_reward: ProcessRewardReport,
    pub live_evolution: LiveInferenceEvolution,
}

#[derive(Debug, Clone)]
pub struct ExperienceMatch {
    pub id: u64,
    pub prompt: String,
    pub lesson: String,
    pub quality: f32,
    pub score: f32,
    pub gist_hints: Vec<String>,
    pub reflection_issue_codes: Vec<String>,
    pub revision_actions: Vec<String>,
    pub process_reward: f32,
    pub reward_action: RewardAction,
    pub used_memory_count: usize,
    pub stored_runtime_kv_memory_ids: Vec<u64>,
    pub route_threshold: f32,
    pub route_attention_tokens: usize,
    pub route_fast_tokens: usize,
    pub route_attention_fraction: f32,
    pub runtime_model_id: Option<String>,
    pub runtime_selected_adapter: Option<String>,
    pub runtime_device_profile: Option<String>,
    pub runtime_primary_lane: Option<String>,
    pub runtime_fallback_lane: Option<String>,
    pub runtime_memory_mode: Option<String>,
    pub runtime_device_execution_source: Option<String>,
    pub runtime_forward_energy: Option<f32>,
    pub runtime_kv_influence: Option<f32>,
    pub runtime_uncertainty_perplexity: Option<f32>,
    pub recursive_runtime_calls: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct ExperienceRetrievalReport {
    pub prompt: String,
    pub profile: TaskProfile,
    pub total_records: usize,
    pub requested_limit: usize,
    pub skipped_cross_task_pollution: usize,
    pub development_evidence_surface_blocked_candidates: usize,
    pub retrieval_noise_penalized_candidates: usize,
    pub retrieval_noise_filtered_candidates: usize,
    pub suppressed_prompt_index_candidates: usize,
    pub max_retrieval_noise_penalty: f32,
    pub matches: Vec<ExperienceMatch>,
}

impl ExperienceRetrievalReport {
    pub fn match_count(&self) -> usize {
        self.matches.len()
    }

    pub fn max_score(&self) -> Option<f32> {
        self.matches
            .iter()
            .map(|item| item.score)
            .max_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal))
    }

    pub fn has_matches(&self) -> bool {
        !self.matches.is_empty()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ExperienceRuntimeTokenMetrics {
    pub token_count: usize,
    pub entropy_count: usize,
    pub logprob_count: usize,
    pub average_entropy: Option<f32>,
    pub average_neg_logprob: Option<f32>,
    pub uncertainty_perplexity: Option<f32>,
}

impl ExperienceRuntimeTokenMetrics {
    pub fn has_uncertainty_signal(&self) -> bool {
        self.average_entropy.is_some()
            || self.average_neg_logprob.is_some()
            || self.uncertainty_perplexity.is_some()
            || self.entropy_count > 0
            || self.logprob_count > 0
    }
}
