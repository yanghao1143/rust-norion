use crate::adaptive_state::LiveInferenceEvolution;
use crate::hierarchy::TaskProfile;
use crate::process_reward::RewardAction;
use crate::reflection::RuntimeDiagnostics;
use crate::router::RouteBudget;

use super::stats::{
    BusinessContractReplayStats, LiveMemoryFeedbackStats, PoolDispatchReplayStats,
    RecursiveReplayStats, RustCheckReplayStats, recursive_call_pressure,
};

#[derive(Debug, Clone, Default)]
pub struct ExperienceReplayPlan {
    pub items: Vec<ExperienceReplayItem>,
}

impl ExperienceReplayPlan {
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct ExperienceReplayItem {
    pub experience_id: u64,
    pub profile: TaskProfile,
    pub action: RewardAction,
    pub reward: f32,
    pub quality: f32,
    pub contradiction_count: usize,
    pub reflection_issue_count: usize,
    pub critical_reflection_issue_count: usize,
    pub revision_action_count: usize,
    pub stream_windows: usize,
    pub route_budget: RouteBudget,
    pub memory_ids: Vec<u64>,
    pub runtime_diagnostics: RuntimeDiagnostics,
    pub live_evolution: LiveInferenceEvolution,
    pub recursive_runtime_calls: Option<usize>,
    pub recursive_stats: Option<RecursiveReplayStats>,
    pub live_memory_feedback: Option<LiveMemoryFeedbackStats>,
    pub rust_check_stats: Option<RustCheckReplayStats>,
    pub rust_check_live_memory_feedback: Option<LiveMemoryFeedbackStats>,
    pub business_contract_stats: Option<BusinessContractReplayStats>,
    pub pool_dispatch_stats: Option<PoolDispatchReplayStats>,
    pub priority: f32,
    pub lesson: String,
}

impl ExperienceReplayItem {
    pub fn route_token_count(&self) -> usize {
        (self.route_budget.attention_tokens + self.route_budget.fast_tokens).max(1)
    }

    pub fn runtime_kv_budget_pressure(&self) -> f32 {
        runtime_kv_budget_pressure(&self.runtime_diagnostics)
    }

    pub fn runtime_kv_weak_import_pressure(&self) -> f32 {
        runtime_kv_weak_import_pressure(&self.runtime_diagnostics)
    }

    pub fn recursive_call_pressure(&self) -> f32 {
        recursive_call_pressure(
            self.recursive_runtime_calls,
            self.recursive_stats,
            self.route_token_count(),
        )
    }
}

pub(crate) fn runtime_kv_budget_pressure(diagnostics: &RuntimeDiagnostics) -> f32 {
    let skipped = diagnostics.budget_limited_runtime_kv_imports_skipped;
    let total = diagnostics.exported_kv_blocks.saturating_add(skipped);
    if total == 0 {
        return 0.0;
    }

    (skipped as f32 / total as f32).clamp(0.0, 1.0)
}

pub(crate) fn runtime_kv_weak_import_pressure(diagnostics: &RuntimeDiagnostics) -> f32 {
    let skipped = diagnostics.weak_runtime_kv_imports_skipped;
    let total = diagnostics.imported_kv_blocks.saturating_add(skipped);
    if total == 0 {
        return 0.0;
    }

    (skipped as f32 / total as f32).clamp(0.0, 1.0)
}
