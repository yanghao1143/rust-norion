use crate::agent_team::AgentTeamPlan;
use crate::hierarchy::{HierarchyController, HierarchyWeights, TaskProfile};
use crate::infini_memory::InfiniMemoryCounts;
use crate::recursive_scheduler::RecursiveSchedule;
use crate::router::{GenerationMetrics, RouteBudget};
use crate::tiered_cache::TierCounts;
use crate::toolsmith::ToolsmithPlan;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RewardAction {
    Reinforce,
    Hold,
    Penalize,
}

impl RewardAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Reinforce => "reinforce",
            Self::Hold => "hold",
            Self::Penalize => "penalize",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "reinforce" => Some(Self::Reinforce),
            "hold" => Some(Self::Hold),
            "penalize" => Some(Self::Penalize),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ProcessRewardComponents {
    pub route: f32,
    pub memory: f32,
    pub hierarchy: f32,
    pub reflection: f32,
    pub latency: f32,
    pub admission: f32,
}

impl Default for ProcessRewardComponents {
    fn default() -> Self {
        Self {
            route: 0.5,
            memory: 0.5,
            hierarchy: 0.5,
            reflection: 0.5,
            latency: 0.5,
            admission: 0.5,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProcessRewardReport {
    pub total: f32,
    pub components: ProcessRewardComponents,
    pub action: RewardAction,
    pub notes: Vec<String>,
}

impl Default for ProcessRewardReport {
    fn default() -> Self {
        Self {
            total: 0.5,
            components: ProcessRewardComponents::default(),
            action: RewardAction::Hold,
            notes: Vec::new(),
        }
    }
}

impl ProcessRewardReport {
    pub fn summary(&self) -> String {
        format!(
            "total={:.3} action={} route={:.3} memory={:.3} hierarchy={:.3} reflection={:.3} latency={:.3} admission={:.3}",
            self.total,
            self.action.as_str(),
            self.components.route,
            self.components.memory,
            self.components.hierarchy,
            self.components.reflection,
            self.components.latency,
            self.components.admission
        )
    }
}

#[derive(Debug, Clone)]
pub struct ProcessRewardInput {
    pub profile: TaskProfile,
    pub route_budget: RouteBudget,
    pub hierarchy: HierarchyWeights,
    pub metrics: GenerationMetrics,
    pub quality: f32,
    pub contradiction_count: usize,
    pub reflection_issue_count: usize,
    pub critical_reflection_issue_count: usize,
    pub revision_action_count: usize,
    pub used_memories: usize,
    pub used_experiences: usize,
    pub tier_counts: TierCounts,
    pub infini_counts: InfiniMemoryCounts,
    pub recursive_schedule: RecursiveSchedule,
    pub recursive_runtime_calls: usize,
    pub stream_windows: usize,
    pub stored_memory: bool,
    pub stored_gist_memories: usize,
    pub stored_runtime_kv_memories: usize,
    pub gist_records: usize,
    pub toolsmith_plan: ToolsmithPlan,
    pub agent_team_plan: AgentTeamPlan,
}

#[derive(Debug, Clone, Default)]
pub struct ProcessRewarder;

impl ProcessRewarder {
    pub fn new() -> Self {
        Self
    }

    pub fn score(&self, input: ProcessRewardInput) -> ProcessRewardReport {
        let quality = input.quality.clamp(0.0, 1.0);
        let quality_score = input.metrics.quality_score();
        let components = ProcessRewardComponents {
            route: route_reward(input.route_budget, quality_score),
            memory: memory_reward(
                input.used_memories,
                input.used_experiences,
                input.gist_records,
                input.contradiction_count,
                quality,
            ),
            hierarchy: hierarchy_reward(input.profile, input.hierarchy),
            reflection: reflection_reward(
                quality,
                input.contradiction_count,
                input.reflection_issue_count,
                input.critical_reflection_issue_count,
                input.revision_action_count,
            ),
            latency: latency_reward(
                input.route_budget,
                input.stream_windows,
                input.tier_counts,
                input.infini_counts,
                &input.recursive_schedule,
                input.recursive_runtime_calls,
            ),
            admission: admission_reward(
                quality,
                input.contradiction_count,
                input.stored_memory,
                input.stored_gist_memories,
                input.stored_runtime_kv_memories,
            ),
        };
        let total = coordination_adjusted_total(
            toolsmith_adjusted_total(weighted_total(components), &input.toolsmith_plan),
            &input.agent_team_plan,
        );
        let action = if total >= 0.72 {
            RewardAction::Reinforce
        } else if total <= 0.42 {
            RewardAction::Penalize
        } else {
            RewardAction::Hold
        };
        let notes = reward_notes(&input, components, total);

        ProcessRewardReport {
            total,
            components,
            action,
            notes,
        }
    }
}

fn route_reward(route_budget: RouteBudget, quality_score: f32) -> f32 {
    let attention = route_budget.attention_fraction.clamp(0.0, 1.0);
    let fast_fraction = 1.0 - attention;

    if quality_score >= 0.78 {
        (0.58 + fast_fraction * 0.34 + attention * 0.08).clamp(0.0, 1.0)
    } else if quality_score < 0.50 {
        (0.18 + attention * 0.42).clamp(0.0, 1.0)
    } else {
        (0.42 + (1.0 - (attention - 0.45).abs()) * 0.28).clamp(0.0, 1.0)
    }
}

fn memory_reward(
    used_memories: usize,
    used_experiences: usize,
    gist_records: usize,
    contradiction_count: usize,
    quality: f32,
) -> f32 {
    let reuse = ((used_memories + used_experiences) as f32 / 6.0).min(1.0);
    let gist_bonus = (gist_records as f32 / 6.0).min(1.0) * 0.14;
    let contradiction_penalty = (contradiction_count as f32 * 0.16).min(0.48);

    (0.34 + quality * 0.34 + reuse * 0.20 + gist_bonus - contradiction_penalty).clamp(0.0, 1.0)
}

fn hierarchy_reward(profile: TaskProfile, hierarchy: HierarchyWeights) -> f32 {
    let target = HierarchyController::target_for_profile(profile);
    let distance = ((hierarchy.global - target.global).abs()
        + (hierarchy.local - target.local).abs()
        + (hierarchy.convolution - target.convolution).abs())
        / 2.0;

    (1.0 - distance).clamp(0.0, 1.0)
}

fn reflection_reward(
    quality: f32,
    contradiction_count: usize,
    issue_count: usize,
    critical_issue_count: usize,
    revision_action_count: usize,
) -> f32 {
    let contradiction_penalty = (contradiction_count as f32 * 0.13).min(0.45);
    let issue_penalty = (issue_count as f32 * 0.04).min(0.20);
    let critical_penalty = (critical_issue_count as f32 * 0.18).min(0.50);
    let action_credit = (revision_action_count as f32 * 0.02).min(0.08);
    (quality - contradiction_penalty - issue_penalty - critical_penalty + action_credit)
        .clamp(0.0, 1.0)
}

fn latency_reward(
    route_budget: RouteBudget,
    stream_windows: usize,
    tier_counts: TierCounts,
    infini_counts: InfiniMemoryCounts,
    recursive_schedule: &RecursiveSchedule,
    recursive_runtime_calls: usize,
) -> f32 {
    let fast_fraction = 1.0 - route_budget.attention_fraction.clamp(0.0, 1.0);
    let stream_pressure = (stream_windows as f32 / 48.0).min(0.30);
    let cold_pressure = (tier_counts.cold_disk as f32 / 12.0).min(0.18);
    let sparse_bonus = if infini_counts.skipped > 0 { 0.08 } else { 0.0 };
    let recursion_pressure = if recursive_schedule.requires_recursion {
        let wave_pressure = (recursive_schedule.execution_wave_count() as f32 / 32.0).min(0.14);
        let chunk_overhead = (recursive_schedule.chunk_count() as f32 / 128.0).min(0.04);
        let call_pressure = (recursive_runtime_calls.saturating_sub(1) as f32 / 160.0).min(0.18);
        wave_pressure + chunk_overhead + call_pressure
    } else {
        0.0
    };

    (0.42 + fast_fraction * 0.34 + sparse_bonus
        - stream_pressure
        - cold_pressure
        - recursion_pressure)
        .clamp(0.0, 1.0)
}

fn admission_reward(
    quality: f32,
    contradiction_count: usize,
    stored_memory: bool,
    stored_gist_memories: usize,
    stored_runtime_kv_memories: usize,
) -> f32 {
    let stored_any = stored_memory || stored_gist_memories > 0 || stored_runtime_kv_memories > 0;
    let store_bonus =
        ((stored_gist_memories + stored_runtime_kv_memories) as f32 / 4.0).min(1.0) * 0.18;

    match (quality >= 0.60 && contradiction_count == 0, stored_any) {
        (true, true) => (0.72 + store_bonus).clamp(0.0, 1.0),
        (true, false) => 0.44,
        (false, false) => 0.72,
        (false, true) => 0.24,
    }
}

fn weighted_total(components: ProcessRewardComponents) -> f32 {
    (components.route * 0.18
        + components.memory * 0.16
        + components.hierarchy * 0.15
        + components.reflection * 0.24
        + components.latency * 0.12
        + components.admission * 0.15)
        .clamp(0.0, 1.0)
}

fn toolsmith_adjusted_total(total: f32, plan: &ToolsmithPlan) -> f32 {
    if !plan.passed_rust_gate() {
        return (total - 0.18).clamp(0.0, 1.0);
    }
    if plan.ready_count() > 0 {
        return (total + 0.04).clamp(0.0, 1.0);
    }
    total
}

fn coordination_adjusted_total(total: f32, plan: &AgentTeamPlan) -> f32 {
    if !plan.enabled {
        return total;
    }
    if !plan.collision_free() {
        return (total - 0.12).clamp(0.0, 1.0);
    }
    if plan.evolution_signal_count() > 0 {
        return (total + 0.03).clamp(0.0, 1.0);
    }
    total
}

fn reward_notes(
    input: &ProcessRewardInput,
    components: ProcessRewardComponents,
    total: f32,
) -> Vec<String> {
    let mut notes = Vec::new();

    if components.route >= 0.75 {
        notes.push("route:efficient_for_quality".to_owned());
    } else if components.route <= 0.40 {
        notes.push("route:under_allocated_attention".to_owned());
    }

    if components.memory >= 0.72 {
        notes.push("memory:useful_reuse_or_gist".to_owned());
    } else if input.used_memories > 0 && input.contradiction_count > 0 {
        notes.push("memory:reuse_needs_penalty".to_owned());
    }

    if input.critical_reflection_issue_count > 0 {
        notes.push(format!(
            "reflection:critical_issues={}",
            input.critical_reflection_issue_count
        ));
    } else if input.reflection_issue_count > 0 {
        notes.push(format!(
            "reflection:issues={}:actions={}",
            input.reflection_issue_count, input.revision_action_count
        ));
    }

    if input.recursive_schedule.requires_recursion {
        notes.push(format!(
            "recursive:chunks={}:merge_rounds={}:waves={}:parallel={}:runtime_calls={}",
            input.recursive_schedule.chunk_count(),
            input.recursive_schedule.merge_round_count(),
            input.recursive_schedule.execution_wave_count(),
            input.recursive_schedule.max_parallel_chunks,
            input.recursive_runtime_calls
        ));
    }
    if input.recursive_runtime_calls > input.recursive_schedule.chunk_count().max(1) * 2 {
        notes.push(format!(
            "latency:recursive_runtime_calls={}",
            input.recursive_runtime_calls
        ));
    }

    if components.admission <= 0.35 {
        notes.push("admission:stored_low_quality_memory".to_owned());
    }
    if input.stored_runtime_kv_memories > 0 {
        notes.push(format!(
            "runtime_kv:stored={}",
            input.stored_runtime_kv_memories
        ));
    }
    notes.extend(input.toolsmith_plan.reward_notes());
    notes.extend(input.agent_team_plan.reward_notes());

    notes.push(format!(
        "total:{total:.3}:{}",
        action_for_total(total).as_str()
    ));
    notes
}

fn action_for_total(total: f32) -> RewardAction {
    if total >= 0.72 {
        RewardAction::Reinforce
    } else if total <= 0.42 {
        RewardAction::Penalize
    } else {
        RewardAction::Hold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn high_quality_fast_path_is_reinforced() {
        let report = ProcessRewarder::new().score(input(0.92, 0, 0.05, false));

        assert!(report.total >= 0.72);
        assert_eq!(report.action, RewardAction::Reinforce);
        assert!(report.components.route > 0.8);
    }

    #[test]
    fn low_quality_stored_memory_is_penalized() {
        let mut input = input(0.25, 2, 0.0, false);
        input.stored_memory = true;

        let report = ProcessRewarder::new().score(input);

        assert_eq!(report.action, RewardAction::Penalize);
        assert!(report.components.admission < 0.35);
    }

    #[test]
    fn recursive_plan_adds_trace_note() {
        let mut input = input(0.82, 0, 0.20, true);
        input.recursive_schedule = crate::recursive_scheduler::RecursiveScheduler::new(8, 6, 2, 2)
            .plan("one two three four five six seven eight nine ten");

        let report = ProcessRewarder::new().score(input);

        assert!(report.notes.iter().any(|note| note.contains("recursive")));
    }

    #[test]
    fn recursive_runtime_calls_reduce_latency_reward() {
        let mut cheap = input(0.82, 0, 0.20, true);
        cheap.recursive_runtime_calls = cheap.recursive_schedule.chunk_count().max(1);
        let mut expensive = cheap.clone();
        expensive.recursive_runtime_calls = 96;

        let cheap_report = ProcessRewarder::new().score(cheap);
        let expensive_report = ProcessRewarder::new().score(expensive);

        assert!(expensive_report.components.latency < cheap_report.components.latency);
        assert!(
            expensive_report
                .notes
                .iter()
                .any(|note| note == "latency:recursive_runtime_calls=96")
        );
    }

    fn input(
        quality: f32,
        contradiction_count: usize,
        attention_fraction: f32,
        requires_recursion: bool,
    ) -> ProcessRewardInput {
        let recursive_schedule = if requires_recursion {
            crate::recursive_scheduler::RecursiveScheduler::new(8, 6, 2, 2)
                .plan("one two three four five six seven eight nine ten")
        } else {
            RecursiveSchedule::default()
        };

        ProcessRewardInput {
            profile: TaskProfile::Coding,
            route_budget: RouteBudget {
                threshold: 0.5,
                attention_tokens: (attention_fraction * 10.0).round() as usize,
                fast_tokens: ((1.0 - attention_fraction) * 10.0).round() as usize,
                attention_fraction,
            },
            hierarchy: HierarchyController::target_for_profile(TaskProfile::Coding),
            metrics: GenerationMetrics {
                perplexity: 6.0,
                semantic_consistency: quality,
                contradiction_count,
                token_count: 64,
            },
            quality,
            contradiction_count,
            reflection_issue_count: contradiction_count,
            critical_reflection_issue_count: 0,
            revision_action_count: contradiction_count,
            used_memories: 2,
            used_experiences: 1,
            tier_counts: TierCounts::default(),
            infini_counts: InfiniMemoryCounts::default(),
            recursive_schedule,
            recursive_runtime_calls: if requires_recursion { 6 } else { 1 },
            stream_windows: 4,
            stored_memory: quality > 0.45,
            stored_gist_memories: if quality > 0.45 { 1 } else { 0 },
            stored_runtime_kv_memories: 0,
            gist_records: if quality > 0.45 { 3 } else { 0 },
            toolsmith_plan: ToolsmithPlan::default(),
            agent_team_plan: AgentTeamPlan::default(),
        }
    }

    #[test]
    fn critical_reflection_issues_reduce_reward() {
        let mut input = input(0.70, 1, 0.45, false);
        input.reflection_issue_count = 3;
        input.critical_reflection_issue_count = 2;
        input.revision_action_count = 3;

        let report = ProcessRewarder::new().score(input);

        assert!(report.components.reflection < 0.30);
        assert!(
            report
                .notes
                .iter()
                .any(|note| note == "reflection:critical_issues=2")
        );
    }
}
