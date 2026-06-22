use crate::hierarchy::{TaskAwareHierarchyPlan, TaskComputeBudget, TaskProfile};

use super::types::{
    AdaptiveRouteAction, AdaptiveRouteCandidate, AdaptiveRouteDecision, AdaptiveRouteSource,
    AdaptiveRoutingPlan, Route, RoutingContext,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ComputeBudgetPolicy {
    pub low_budget_threshold_lift: f32,
    pub pressure_threshold_lift: f32,
    pub expanded_budget_threshold_relief: f32,
    pub max_low_budget_fanout: usize,
    pub max_normal_budget_fanout: usize,
    pub max_expanded_budget_fanout: usize,
    pub low_budget_kv_lookups: usize,
    pub normal_budget_kv_lookups: usize,
    pub expanded_budget_kv_lookups: usize,
    pub low_budget_reflection_passes: usize,
    pub normal_budget_reflection_passes: usize,
    pub expanded_budget_reflection_passes: usize,
    pub validation_run_cost_tokens: usize,
    pub reflection_pass_cost_tokens: usize,
}

impl Default for ComputeBudgetPolicy {
    fn default() -> Self {
        Self {
            low_budget_threshold_lift: 0.12,
            pressure_threshold_lift: 0.10,
            expanded_budget_threshold_relief: 0.04,
            max_low_budget_fanout: 1,
            max_normal_budget_fanout: 3,
            max_expanded_budget_fanout: 5,
            low_budget_kv_lookups: 1,
            normal_budget_kv_lookups: 3,
            expanded_budget_kv_lookups: 6,
            low_budget_reflection_passes: 1,
            normal_budget_reflection_passes: 2,
            expanded_budget_reflection_passes: 3,
            validation_run_cost_tokens: 96,
            reflection_pass_cost_tokens: 64,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ComputeBudgetContext {
    pub profile: TaskProfile,
    pub compute_budget: TaskComputeBudget,
    pub validation_mode: bool,
    pub prompt_tokens: usize,
    pub max_tokens: Option<usize>,
    pub route_fanout: usize,
}

impl ComputeBudgetContext {
    pub fn from_task_plan(plan: &TaskAwareHierarchyPlan, prompt_tokens: usize) -> Self {
        Self {
            profile: plan.profile,
            compute_budget: plan.signals.compute_budget,
            validation_mode: plan.signals.validation_mode,
            prompt_tokens,
            max_tokens: None,
            route_fanout: plan.route_fanout,
        }
    }

    pub fn with_max_tokens(mut self, max_tokens: Option<usize>) -> Self {
        self.max_tokens = max_tokens.map(|tokens| tokens.max(1));
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComputeBudgetSchedule {
    pub profile: TaskProfile,
    pub compute_budget: TaskComputeBudget,
    pub base_threshold: f32,
    pub threshold_after: f32,
    pub threshold_delta: f32,
    pub route_fanout_before: usize,
    pub route_fanout_after: usize,
    pub candidate_count: usize,
    pub selected_candidates: usize,
    pub anchor_count: usize,
    pub anchors_preserved: usize,
    pub low_value_skipped: usize,
    pub kv_lookup_budget: usize,
    pub kv_lookups_planned: usize,
    pub kv_lookups_skipped: usize,
    pub reflection_pass_budget: usize,
    pub validation_run_budget: usize,
    pub validation_cost_tokens: usize,
    pub input_tokens: usize,
    pub retained_tokens: usize,
    pub saved_tokens: usize,
    pub estimated_budget_tokens: usize,
    pub estimated_spent_tokens: usize,
    pub wasted_compute_avoided_tokens: usize,
    pub fallback_triggered: bool,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
    pub notes: Vec<String>,
}

impl ComputeBudgetSchedule {
    pub fn empty(profile: TaskProfile) -> Self {
        Self {
            profile,
            compute_budget: TaskComputeBudget::Normal,
            base_threshold: 0.52,
            threshold_after: 0.52,
            threshold_delta: 0.0,
            route_fanout_before: 0,
            route_fanout_after: 0,
            candidate_count: 0,
            selected_candidates: 0,
            anchor_count: 0,
            anchors_preserved: 0,
            low_value_skipped: 0,
            kv_lookup_budget: 0,
            kv_lookups_planned: 0,
            kv_lookups_skipped: 0,
            reflection_pass_budget: 0,
            validation_run_budget: 0,
            validation_cost_tokens: 0,
            input_tokens: 0,
            retained_tokens: 0,
            saved_tokens: 0,
            estimated_budget_tokens: 0,
            estimated_spent_tokens: 0,
            wasted_compute_avoided_tokens: 0,
            fallback_triggered: false,
            read_only: true,
            write_allowed: false,
            applied: false,
            notes: Vec::new(),
        }
    }

    pub fn anchors_preserved(&self) -> bool {
        self.anchors_preserved == self.anchor_count
    }

    pub fn budget_accounting_matches(&self) -> bool {
        self.retained_tokens.saturating_add(self.saved_tokens) == self.input_tokens
            && self.wasted_compute_avoided_tokens
                <= self
                    .saved_tokens
                    .saturating_add(self.kv_lookups_skipped.saturating_mul(16))
            && self.estimated_spent_tokens <= self.estimated_budget_tokens
    }

    pub fn summary_line(&self) -> String {
        format!(
            "compute_budget_schedule profile={} budget={} threshold={:.6}->{:.6} fanout={}->{} candidates={} selected={} anchors={}/{} kv={}/{} kv_skipped={} reflection_passes={} validation_runs={} validation_cost={} input_tokens={} retained_tokens={} saved_tokens={} avoided_tokens={} fallback={} read_only={} write_allowed={} applied={}",
            profile_slug(self.profile),
            self.compute_budget.as_str(),
            self.base_threshold,
            self.threshold_after,
            self.route_fanout_before,
            self.route_fanout_after,
            self.candidate_count,
            self.selected_candidates,
            self.anchors_preserved,
            self.anchor_count,
            self.kv_lookups_planned,
            self.kv_lookup_budget,
            self.kv_lookups_skipped,
            self.reflection_pass_budget,
            self.validation_run_budget,
            self.validation_cost_tokens,
            self.input_tokens,
            self.retained_tokens,
            self.saved_tokens,
            self.wasted_compute_avoided_tokens,
            self.fallback_triggered,
            self.read_only,
            self.write_allowed,
            self.applied
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BudgetedAdaptiveRoutingPlan {
    pub routing_plan: AdaptiveRoutingPlan,
    pub schedule: ComputeBudgetSchedule,
}

#[derive(Debug, Clone)]
pub struct ComputeBudgetScheduler {
    pub policy: ComputeBudgetPolicy,
}

impl Default for ComputeBudgetScheduler {
    fn default() -> Self {
        Self {
            policy: ComputeBudgetPolicy::default(),
        }
    }
}

impl ComputeBudgetScheduler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_policy(mut self, policy: ComputeBudgetPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn threshold_for(
        &self,
        base_threshold: f32,
        context: RoutingContext,
        budget: ComputeBudgetContext,
        candidates: &[AdaptiveRouteCandidate],
    ) -> f32 {
        compute_threshold(&self.policy, base_threshold, context, budget, candidates)
    }

    pub fn schedule(
        &self,
        base_threshold: f32,
        context: RoutingContext,
        budget: ComputeBudgetContext,
        candidates: &[AdaptiveRouteCandidate],
        plan: AdaptiveRoutingPlan,
    ) -> BudgetedAdaptiveRoutingPlan {
        let mut notes = Vec::new();
        let threshold_after =
            compute_threshold(&self.policy, base_threshold, context, budget, candidates);
        let anchor_count = candidates
            .iter()
            .filter(|candidate| candidate.anchor_required)
            .count();
        let kv_candidates = candidates
            .iter()
            .filter(|candidate| source_uses_kv_lookup(candidate.source))
            .count();
        let base_fanout_cap = fanout_cap(&self.policy, budget.compute_budget);
        let route_fanout_before = budget.route_fanout.max(1);
        let mut route_fanout_after = route_fanout_before.min(base_fanout_cap).max(1);
        if route_fanout_after < anchor_count {
            route_fanout_after = anchor_count;
            notes.push("correctness_anchors_raise_fanout_cap".to_owned());
        }

        let anchor_kv_count = candidates
            .iter()
            .filter(|candidate| {
                candidate.anchor_required && source_uses_kv_lookup(candidate.source)
            })
            .count();
        let mut kv_lookup_budget = kv_lookup_budget(&self.policy, budget.compute_budget);
        if kv_lookup_budget < anchor_kv_count {
            kv_lookup_budget = anchor_kv_count;
            notes.push("correctness_anchors_raise_kv_lookup_budget".to_owned());
        }
        let reflection_pass_budget = reflection_pass_budget(&self.policy, budget.compute_budget);
        let validation_run_budget: usize = if budget.validation_mode {
            match budget.compute_budget {
                TaskComputeBudget::Low => 1,
                TaskComputeBudget::Normal | TaskComputeBudget::Expanded => 2,
            }
        } else {
            0
        };

        let mut decisions = plan.decisions;
        let fanout_pruned = apply_fanout_cap(&mut decisions, route_fanout_after);
        let kv_pruned = apply_kv_lookup_cap(&mut decisions, kv_lookup_budget);
        let low_value_skipped = fanout_pruned.saturating_add(kv_pruned);
        let routing_plan =
            AdaptiveRoutingPlan::from_decisions(plan.profile, threshold_after, decisions);
        let selected_candidates = routing_plan.include.saturating_add(routing_plan.compress);
        let anchors_preserved = routing_plan
            .decisions
            .iter()
            .filter(|decision| decision.anchor_required && decision.action.retains_tokens())
            .count();
        let kv_lookups_planned = routing_plan
            .decisions
            .iter()
            .filter(|decision| {
                source_uses_kv_lookup(decision.source) && decision.action.retains_tokens()
            })
            .count();
        let kv_lookups_skipped = kv_candidates.saturating_sub(kv_lookups_planned);
        let validation_cost_tokens =
            validation_run_budget.saturating_mul(self.policy.validation_run_cost_tokens);
        let reflection_cost_tokens =
            reflection_pass_budget.saturating_mul(self.policy.reflection_pass_cost_tokens);
        let max_tokens = budget.max_tokens.unwrap_or(match budget.compute_budget {
            TaskComputeBudget::Low => 128,
            TaskComputeBudget::Normal => 512,
            TaskComputeBudget::Expanded => 2048,
        });
        let estimated_budget_tokens = budget
            .prompt_tokens
            .saturating_add(max_tokens)
            .saturating_add(reflection_cost_tokens)
            .saturating_add(validation_cost_tokens);
        let estimated_spent_tokens = budget
            .prompt_tokens
            .saturating_add(routing_plan.retained_tokens)
            .saturating_add(reflection_cost_tokens)
            .saturating_add(validation_cost_tokens)
            .min(estimated_budget_tokens);
        let fallback_triggered = selected_candidates == 0 || anchors_preserved < anchor_count;
        if fallback_triggered {
            notes.push("fallback_fast_projection_or_anchor_hold".to_owned());
        }
        if low_value_skipped > 0 {
            notes.push("low_value_candidates_pruned_by_fanout_budget".to_owned());
        }
        if kv_lookups_skipped > 0 {
            notes.push("kv_lookup_budget_saved_work".to_owned());
        }

        let schedule = ComputeBudgetSchedule {
            profile: budget.profile,
            compute_budget: budget.compute_budget,
            base_threshold: finite_unit(base_threshold),
            threshold_after,
            threshold_delta: (threshold_after - finite_unit(base_threshold)).abs(),
            route_fanout_before,
            route_fanout_after,
            candidate_count: routing_plan.candidates,
            selected_candidates,
            anchor_count,
            anchors_preserved,
            low_value_skipped,
            kv_lookup_budget,
            kv_lookups_planned,
            kv_lookups_skipped,
            reflection_pass_budget,
            validation_run_budget,
            validation_cost_tokens,
            input_tokens: routing_plan.input_tokens,
            retained_tokens: routing_plan.retained_tokens,
            saved_tokens: routing_plan.saved_tokens,
            estimated_budget_tokens,
            estimated_spent_tokens,
            wasted_compute_avoided_tokens: routing_plan
                .saved_tokens
                .saturating_add(kv_lookups_skipped.saturating_mul(16))
                .saturating_sub(low_value_skipped.min(1)),
            fallback_triggered,
            read_only: true,
            write_allowed: false,
            applied: false,
            notes,
        };

        BudgetedAdaptiveRoutingPlan {
            routing_plan,
            schedule,
        }
    }
}

fn compute_threshold(
    policy: &ComputeBudgetPolicy,
    base_threshold: f32,
    context: RoutingContext,
    budget: ComputeBudgetContext,
    candidates: &[AdaptiveRouteCandidate],
) -> f32 {
    let base = finite_unit(base_threshold);
    let candidate_pressure = (candidate_token_sum(candidates) as f32 / 4096.0).min(1.0);
    let output_pressure = budget
        .max_tokens
        .map(|tokens| (96.0 / tokens.max(1) as f32).min(1.0))
        .unwrap_or(0.0);
    let hardware_pressure = context.hardware_pressure.clamp(0.0, 1.0);
    let headroom_pressure = (0.50 - context.compute_headroom.clamp(0.0, 1.0)).max(0.0);
    let pressure = (candidate_pressure * 0.30
        + output_pressure * 0.20
        + hardware_pressure * 0.30
        + headroom_pressure * 0.20)
        .clamp(0.0, 1.0);
    let budget_lift = match budget.compute_budget {
        TaskComputeBudget::Low => policy.low_budget_threshold_lift,
        TaskComputeBudget::Normal => 0.0,
        TaskComputeBudget::Expanded => -policy.expanded_budget_threshold_relief,
    };
    (base + budget_lift + pressure * policy.pressure_threshold_lift).clamp(0.18, 0.92)
}

fn apply_fanout_cap(decisions: &mut [AdaptiveRouteDecision], route_fanout_after: usize) -> usize {
    let mut retained = decisions
        .iter()
        .filter(|decision| decision.action.retains_tokens())
        .count();
    if retained <= route_fanout_after {
        return 0;
    }

    let mut prune_order = decisions
        .iter()
        .enumerate()
        .filter(|(_, decision)| decision.action.retains_tokens() && !decision.anchor_required)
        .map(|(index, decision)| {
            (
                index,
                decision.score,
                decision.components.compute_cost,
                decision.estimated_tokens,
            )
        })
        .collect::<Vec<_>>();
    prune_order.sort_by(|left, right| {
        left.1
            .partial_cmp(&right.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                right
                    .2
                    .partial_cmp(&left.2)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| right.3.cmp(&left.3))
            .then_with(|| left.0.cmp(&right.0))
    });

    let mut pruned = 0usize;
    for (index, _, _, _) in prune_order {
        if retained <= route_fanout_after {
            break;
        }
        let decision = &mut decisions[index];
        decision.action = AdaptiveRouteAction::Skip;
        decision.route = Route::FastProjection;
        decision.retained_tokens = 0;
        decision.reason = format!("{} budget_fanout_pruned=true", decision.reason);
        retained = retained.saturating_sub(1);
        pruned = pruned.saturating_add(1);
    }
    pruned
}

fn apply_kv_lookup_cap(decisions: &mut [AdaptiveRouteDecision], kv_lookup_budget: usize) -> usize {
    let mut retained_kv = decisions
        .iter()
        .filter(|decision| {
            source_uses_kv_lookup(decision.source) && decision.action.retains_tokens()
        })
        .count();
    if retained_kv <= kv_lookup_budget {
        return 0;
    }

    let mut prune_order = decisions
        .iter()
        .enumerate()
        .filter(|(_, decision)| {
            source_uses_kv_lookup(decision.source)
                && decision.action.retains_tokens()
                && !decision.anchor_required
        })
        .map(|(index, decision)| {
            (
                index,
                decision.score,
                decision.components.compute_cost,
                decision.estimated_tokens,
            )
        })
        .collect::<Vec<_>>();
    prune_order.sort_by(|left, right| {
        left.1
            .partial_cmp(&right.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                right
                    .2
                    .partial_cmp(&left.2)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| right.3.cmp(&left.3))
            .then_with(|| left.0.cmp(&right.0))
    });

    let mut pruned = 0usize;
    for (index, _, _, _) in prune_order {
        if retained_kv <= kv_lookup_budget {
            break;
        }
        let decision = &mut decisions[index];
        decision.action = AdaptiveRouteAction::Skip;
        decision.route = Route::FastProjection;
        decision.retained_tokens = 0;
        decision.reason = format!("{} budget_kv_lookup_pruned=true", decision.reason);
        retained_kv = retained_kv.saturating_sub(1);
        pruned = pruned.saturating_add(1);
    }
    pruned
}

fn fanout_cap(policy: &ComputeBudgetPolicy, budget: TaskComputeBudget) -> usize {
    match budget {
        TaskComputeBudget::Low => policy.max_low_budget_fanout,
        TaskComputeBudget::Normal => policy.max_normal_budget_fanout,
        TaskComputeBudget::Expanded => policy.max_expanded_budget_fanout,
    }
    .max(1)
}

fn kv_lookup_budget(policy: &ComputeBudgetPolicy, budget: TaskComputeBudget) -> usize {
    match budget {
        TaskComputeBudget::Low => policy.low_budget_kv_lookups,
        TaskComputeBudget::Normal => policy.normal_budget_kv_lookups,
        TaskComputeBudget::Expanded => policy.expanded_budget_kv_lookups,
    }
}

fn reflection_pass_budget(policy: &ComputeBudgetPolicy, budget: TaskComputeBudget) -> usize {
    match budget {
        TaskComputeBudget::Low => policy.low_budget_reflection_passes,
        TaskComputeBudget::Normal => policy.normal_budget_reflection_passes,
        TaskComputeBudget::Expanded => policy.expanded_budget_reflection_passes,
    }
}

fn candidate_token_sum(candidates: &[AdaptiveRouteCandidate]) -> usize {
    candidates
        .iter()
        .map(|candidate| candidate.estimated_tokens)
        .sum()
}

fn source_uses_kv_lookup(source: AdaptiveRouteSource) -> bool {
    matches!(
        source,
        AdaptiveRouteSource::SemanticMemory
            | AdaptiveRouteSource::GistMemory
            | AdaptiveRouteSource::RuntimeKv
            | AdaptiveRouteSource::ReasoningGenome
    )
}

fn finite_unit(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.52
    }
}

fn profile_slug(profile: TaskProfile) -> &'static str {
    match profile {
        TaskProfile::General => "general",
        TaskProfile::Coding => "coding",
        TaskProfile::Writing => "writing",
        TaskProfile::LongDocument => "long_document",
    }
}
