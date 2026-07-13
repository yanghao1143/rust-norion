use crate::hierarchy::{TaskComputeBudget, TaskProfile};
use norion_core::RuntimeToolResultProjectionBudget;

use super::adaptive::AdaptiveRoutingPlanner;
use super::budget::{BudgetedAdaptiveRoutingPlan, ComputeBudgetContext, ComputeBudgetSchedule};
use super::types::{
    AdaptiveRouteAction, AdaptiveRouteCandidate, AdaptiveRouteDecision,
    AdaptiveRouteScoreComponents, AdaptiveRouteSource, Route, RoutingContext,
};

pub const ROUTER_DECISION_TRACE_SCHEMA: &str = "rust-norion-router-decision-trace-v1";
pub const TOOL_RESULT_PROJECTION_TRACE_SCHEMA: &str = "rust-norion-tool-result-projection-trace-v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolResultProjectionTrace {
    pub schema: &'static str,
    pub tool_name: String,
    pub tokens_before: usize,
    pub tokens_after: usize,
    pub tokens_saved: usize,
    pub handle_present: bool,
    pub digest_only: bool,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl ToolResultProjectionTrace {
    pub fn new(tool_name: &str, budget: RuntimeToolResultProjectionBudget) -> Self {
        Self {
            schema: TOOL_RESULT_PROJECTION_TRACE_SCHEMA,
            tool_name: sanitize_public_label(tool_name, "tool").value,
            tokens_before: budget.tokens_before,
            tokens_after: budget.tokens_after,
            tokens_saved: budget.tokens_saved,
            handle_present: budget.handle_present,
            digest_only: budget.digest_only,
            read_only: true,
            write_allowed: false,
            applied: false,
        }
    }

    pub fn to_json(&self) -> String {
        format!(
            "{{\"schema\":\"{}\",\"tool_name\":\"{}\",\"tokens_before\":{},\"tokens_after\":{},\"tokens_saved\":{},\"handle_present\":{},\"digest_only\":{},\"read_only\":{},\"write_allowed\":{},\"applied\":{}}}",
            self.schema,
            json_escape(&self.tool_name),
            self.tokens_before,
            self.tokens_after,
            self.tokens_saved,
            self.handle_present,
            self.digest_only,
            self.read_only,
            self.write_allowed,
            self.applied,
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RouterDecisionTraceRow {
    pub candidate_id: String,
    pub source: AdaptiveRouteSource,
    pub action: AdaptiveRouteAction,
    pub route: Route,
    pub selected_lane: String,
    pub selected: bool,
    pub rejected: bool,
    pub score: f32,
    pub threshold: f32,
    pub score_delta: f32,
    pub compute_pressure: f32,
    pub budget_pressure: f32,
    pub estimated_tokens: usize,
    pub retained_tokens: usize,
    pub saved_tokens: usize,
    pub anchor_required: bool,
    pub components: AdaptiveRouteScoreComponents,
    pub kv_fusion_lane: bool,
    pub kv_fusion_contribution: f32,
    pub fallback_path: bool,
    pub reason: String,
}

impl RouterDecisionTraceRow {
    fn from_decision(decision: &AdaptiveRouteDecision, budget_pressure: f32) -> Sanitized<Self> {
        let candidate_id = sanitize_public_label(&decision.candidate_id, "candidate");
        let reason = sanitize_public_text(&decision.reason, 240);
        let components = decision.components.clamp();
        let kv_fusion_lane = decision.source.prefers_fusion();
        let kv_fusion_contribution = if kv_fusion_lane {
            (components.memory_fitness * 0.50
                + components.trust * 0.30
                + components.reward_history * 0.20)
                .clamp(0.0, 1.0)
        } else {
            0.0
        };
        let selected = decision.action.retains_tokens();
        let row = Self {
            candidate_id: candidate_id.value,
            source: decision.source,
            action: decision.action,
            route: decision.route,
            selected_lane: route_lane(decision.route).to_owned(),
            selected,
            rejected: !selected,
            score: finite_unit(decision.score),
            threshold: finite_unit(decision.threshold),
            score_delta: finite_f32(decision.score - decision.threshold),
            compute_pressure: finite_unit(decision.compute_pressure),
            budget_pressure,
            estimated_tokens: decision.estimated_tokens,
            retained_tokens: decision.retained_tokens,
            saved_tokens: decision.saved_tokens(),
            anchor_required: decision.anchor_required,
            components,
            kv_fusion_lane,
            kv_fusion_contribution,
            fallback_path: decision.route == Route::FastProjection && !selected,
            reason: reason.value,
        };
        Sanitized {
            value: row,
            redactions: candidate_id.redactions.saturating_add(reason.redactions),
            payload_markers: candidate_id
                .payload_markers
                .saturating_add(reason.payload_markers),
        }
    }

    pub fn summary_line(&self) -> String {
        format!(
            "candidate={} source={} action={} lane={} route={} selected={} score={:.6} threshold={:.6} delta={:.6} pressure={:.6} budget_pressure={:.6} retained={} saved={} kv_fusion={:.6} fallback={}",
            self.candidate_id,
            self.source.as_str(),
            self.action.as_str(),
            self.selected_lane,
            self.route.as_str(),
            self.selected,
            self.score,
            self.threshold,
            self.score_delta,
            self.compute_pressure,
            self.budget_pressure,
            self.retained_tokens,
            self.saved_tokens,
            self.kv_fusion_contribution,
            self.fallback_path
        )
    }

    fn to_json(&self) -> String {
        format!(
            "{{\"candidate_id\":\"{}\",\"source\":\"{}\",\"action\":\"{}\",\"selected_lane\":\"{}\",\"route\":\"{}\",\"selected\":{},\"rejected\":{},\"score\":{:.6},\"threshold\":{:.6},\"score_delta\":{:.6},\"compute_pressure\":{:.6},\"budget_pressure\":{:.6},\"estimated_tokens\":{},\"retained_tokens\":{},\"saved_tokens\":{},\"anchor_required\":{},\"kv_fusion_lane\":{},\"kv_fusion_contribution\":{:.6},\"fallback_path\":{},\"components\":{{\"task_intent\":{:.6},\"language_mode\":{:.6},\"code_mode\":{:.6},\"memory_fitness\":{:.6},\"recency\":{:.6},\"trust\":{:.6},\"compute_cost\":{:.6},\"reward_history\":{:.6}}},\"reason\":\"{}\"}}",
            json_escape(&self.candidate_id),
            self.source.as_str(),
            self.action.as_str(),
            json_escape(&self.selected_lane),
            self.route.as_str(),
            self.selected,
            self.rejected,
            self.score,
            self.threshold,
            self.score_delta,
            self.compute_pressure,
            self.budget_pressure,
            self.estimated_tokens,
            self.retained_tokens,
            self.saved_tokens,
            self.anchor_required,
            self.kv_fusion_lane,
            self.kv_fusion_contribution,
            self.fallback_path,
            self.components.task_intent,
            self.components.language_mode,
            self.components.code_mode,
            self.components.memory_fitness,
            self.components.recency,
            self.components.trust,
            self.components.compute_cost,
            self.components.reward_history,
            json_escape(&self.reason)
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RouterDecisionTrace {
    pub schema: &'static str,
    pub trace_id: String,
    pub profile: TaskProfile,
    pub compute_budget: TaskComputeBudget,
    pub base_threshold: f32,
    pub adaptive_threshold: f32,
    pub threshold_delta: f32,
    pub budget_pressure: f32,
    pub candidate_count: usize,
    pub selected_candidates: usize,
    pub route_fanout_before: usize,
    pub route_fanout_after: usize,
    pub kv_lookup_budget: usize,
    pub kv_lookups_planned: usize,
    pub kv_lookups_skipped: usize,
    pub retained_tokens: usize,
    pub saved_tokens: usize,
    pub wasted_compute_avoided_tokens: usize,
    pub fallback_triggered: bool,
    pub fallback_reason: String,
    pub rows: Vec<RouterDecisionTraceRow>,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
    pub export_allowed: bool,
    pub redactions: usize,
    pub payload_markers: usize,
    pub blocked_reasons: Vec<String>,
}

impl RouterDecisionTrace {
    pub fn from_budgeted_plan(plan: &BudgetedAdaptiveRoutingPlan) -> Self {
        let schedule = &plan.schedule;
        let budget_pressure = budget_pressure(schedule);
        let mut redactions = 0usize;
        let mut payload_markers = 0usize;
        let rows = plan
            .routing_plan
            .decisions
            .iter()
            .map(|decision| {
                let row = RouterDecisionTraceRow::from_decision(decision, budget_pressure);
                redactions = redactions.saturating_add(row.redactions);
                payload_markers = payload_markers.saturating_add(row.payload_markers);
                row.value
            })
            .collect::<Vec<_>>();
        let fallback_reason = fallback_reason(schedule);
        let trace_id = stable_trace_id(schedule, &rows);

        let mut trace = Self {
            schema: ROUTER_DECISION_TRACE_SCHEMA,
            trace_id,
            profile: plan.routing_plan.profile,
            compute_budget: schedule.compute_budget,
            base_threshold: finite_unit(schedule.base_threshold),
            adaptive_threshold: finite_unit(schedule.threshold_after),
            threshold_delta: finite_unit(schedule.threshold_delta),
            budget_pressure,
            candidate_count: plan.routing_plan.candidates,
            selected_candidates: schedule.selected_candidates,
            route_fanout_before: schedule.route_fanout_before,
            route_fanout_after: schedule.route_fanout_after,
            kv_lookup_budget: schedule.kv_lookup_budget,
            kv_lookups_planned: schedule.kv_lookups_planned,
            kv_lookups_skipped: schedule.kv_lookups_skipped,
            retained_tokens: plan.routing_plan.retained_tokens,
            saved_tokens: plan.routing_plan.saved_tokens,
            wasted_compute_avoided_tokens: schedule.wasted_compute_avoided_tokens,
            fallback_triggered: schedule.fallback_triggered,
            fallback_reason,
            rows,
            read_only: plan.routing_plan.read_only && schedule.read_only,
            write_allowed: plan.routing_plan.write_allowed || schedule.write_allowed,
            applied: plan.routing_plan.applied || schedule.applied,
            export_allowed: true,
            redactions,
            payload_markers,
            blocked_reasons: Vec::new(),
        };
        trace.blocked_reasons = trace.validation_failures();
        trace.export_allowed = trace.blocked_reasons.is_empty();
        trace
    }

    pub fn selected_candidate_ids(&self) -> Vec<String> {
        self.rows
            .iter()
            .filter(|row| row.selected)
            .map(|row| row.candidate_id.clone())
            .collect()
    }

    pub fn selected_lanes(&self) -> Vec<String> {
        unique_strings(
            self.rows
                .iter()
                .filter(|row| row.selected)
                .map(|row| row.selected_lane.clone()),
        )
    }

    pub fn rejected_candidate_ids(&self) -> Vec<String> {
        self.rows
            .iter()
            .filter(|row| row.rejected)
            .map(|row| row.candidate_id.clone())
            .collect()
    }

    pub fn decision_summaries(&self, limit: usize) -> Vec<String> {
        self.rows
            .iter()
            .take(limit)
            .map(RouterDecisionTraceRow::summary_line)
            .collect()
    }

    pub fn validation_failures(&self) -> Vec<String> {
        let mut failures = Vec::new();
        if self.schema != ROUTER_DECISION_TRACE_SCHEMA {
            failures.push("router_trace_schema_mismatch".to_owned());
        }
        if self.rows.len() != self.candidate_count {
            failures.push(format!(
                "router_trace_candidate_count_mismatch rows={} candidates={}",
                self.rows.len(),
                self.candidate_count
            ));
        }
        let selected = self.rows.iter().filter(|row| row.selected).count();
        if selected != self.selected_candidates {
            failures.push(format!(
                "router_trace_selected_count_mismatch rows={} schedule={}",
                selected, self.selected_candidates
            ));
        }
        if self.route_fanout_before == 0 || self.route_fanout_after == 0 {
            failures.push("router_trace_route_fanout_must_be_positive".to_owned());
        }
        if self.kv_lookups_planned > self.kv_lookup_budget {
            failures.push("router_trace_kv_lookup_budget_exceeded".to_owned());
        }
        if self.retained_tokens.saturating_add(self.saved_tokens)
            != self
                .rows
                .iter()
                .map(|row| row.estimated_tokens)
                .sum::<usize>()
        {
            failures.push("router_trace_token_accounting_mismatch".to_owned());
        }
        if self.fallback_triggered && self.fallback_reason == "none" {
            failures.push("router_trace_fallback_missing_reason".to_owned());
        }
        if !self.read_only || self.write_allowed || self.applied {
            failures.push("router_trace_must_remain_read_only".to_owned());
        }
        if self.redactions > 0 {
            failures.push("router_trace_redactions_present".to_owned());
        }
        if self.payload_markers > 0 {
            failures.push("router_trace_payload_marker_present".to_owned());
        }
        for (index, row) in self.rows.iter().enumerate() {
            if !unit_score(row.score)
                || !unit_score(row.threshold)
                || !unit_score(row.compute_pressure)
                || !unit_score(row.budget_pressure)
                || !unit_score(row.kv_fusion_contribution)
            {
                failures.push(format!(
                    "router_trace_row_{index}_non_finite_or_out_of_range"
                ));
            }
            if row.retained_tokens.saturating_add(row.saved_tokens) != row.estimated_tokens {
                failures.push(format!(
                    "router_trace_row_{index}_token_accounting_mismatch"
                ));
            }
            if row.selected != row.action.retains_tokens() {
                failures.push(format!("router_trace_row_{index}_selected_action_mismatch"));
            }
            if row.anchor_required && row.rejected {
                failures.push(format!("router_trace_row_{index}_anchor_rejected"));
            }
            if row.kv_fusion_lane && row.kv_fusion_contribution <= 0.0 {
                failures.push(format!(
                    "router_trace_row_{index}_missing_kv_fusion_contribution"
                ));
            }
            for value in [&row.candidate_id, &row.reason, &row.selected_lane] {
                if contains_payload_marker(value) || contains_sensitive_marker(value) {
                    failures.push(format!("router_trace_row_{index}_unredacted_payload"));
                    break;
                }
            }
        }
        failures
    }

    pub fn to_visualization_json(&self) -> String {
        let rows = self
            .rows
            .iter()
            .map(RouterDecisionTraceRow::to_json)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"schema\":\"{}\",\"trace_id\":\"{}\",\"profile\":\"{}\",\"compute_budget\":\"{}\",\"base_threshold\":{:.6},\"adaptive_threshold\":{:.6},\"threshold_delta\":{:.6},\"budget_pressure\":{:.6},\"candidate_count\":{},\"selected_candidates\":{},\"selected_candidate_ids\":{},\"selected_lanes\":{},\"rejected_candidate_ids\":{},\"route_fanout_before\":{},\"route_fanout_after\":{},\"kv_lookup_budget\":{},\"kv_lookups_planned\":{},\"kv_lookups_skipped\":{},\"retained_tokens\":{},\"saved_tokens\":{},\"wasted_compute_avoided_tokens\":{},\"fallback_triggered\":{},\"fallback_reason\":\"{}\",\"read_only\":{},\"write_allowed\":{},\"applied\":{},\"export_allowed\":{},\"redactions\":{},\"payload_markers\":{},\"blocked_reasons\":{},\"decisions\":[{}]}}",
            self.schema,
            json_escape(&self.trace_id),
            profile_slug(self.profile),
            self.compute_budget.as_str(),
            self.base_threshold,
            self.adaptive_threshold,
            self.threshold_delta,
            self.budget_pressure,
            self.candidate_count,
            self.selected_candidates,
            string_array_json(&self.selected_candidate_ids()),
            string_array_json(&self.selected_lanes()),
            string_array_json(&self.rejected_candidate_ids()),
            self.route_fanout_before,
            self.route_fanout_after,
            self.kv_lookup_budget,
            self.kv_lookups_planned,
            self.kv_lookups_skipped,
            self.retained_tokens,
            self.saved_tokens,
            self.wasted_compute_avoided_tokens,
            self.fallback_triggered,
            json_escape(&self.fallback_reason),
            self.read_only,
            self.write_allowed,
            self.applied,
            self.export_allowed,
            self.redactions,
            self.payload_markers,
            string_array_json(&self.blocked_reasons),
            rows
        )
    }

    pub fn summary_line(&self) -> String {
        format!(
            "router_decision_trace schema={} id={} profile={} budget={} candidates={} selected={} threshold={:.6}->{:.6} pressure={:.6} saved={} avoided={} fallback={} export_allowed={} blockers={}",
            self.schema,
            self.trace_id,
            profile_slug(self.profile),
            self.compute_budget.as_str(),
            self.candidate_count,
            self.selected_candidates,
            self.base_threshold,
            self.adaptive_threshold,
            self.budget_pressure,
            self.saved_tokens,
            self.wasted_compute_avoided_tokens,
            self.fallback_triggered,
            self.export_allowed,
            self.blocked_reasons.len()
        )
    }
}

#[derive(Debug, Clone)]
pub struct RoutingTraceReplayFixture {
    pub fixture_id: String,
    pub profile: TaskProfile,
    pub base_threshold: f32,
    pub context: RoutingContext,
    pub budget: ComputeBudgetContext,
    pub candidates: Vec<AdaptiveRouteCandidate>,
    pub expected_selected_candidate_ids: Vec<String>,
    pub expected_selected_lanes: Vec<String>,
    pub expected_fallback_triggered: bool,
}

impl RoutingTraceReplayFixture {
    pub fn new(
        fixture_id: impl AsRef<str>,
        profile: TaskProfile,
        base_threshold: f32,
        context: RoutingContext,
        budget: ComputeBudgetContext,
        candidates: Vec<AdaptiveRouteCandidate>,
    ) -> Self {
        Self {
            fixture_id: sanitize_public_label(fixture_id.as_ref(), "fixture").value,
            profile,
            base_threshold: finite_unit(base_threshold),
            context,
            budget,
            candidates,
            expected_selected_candidate_ids: Vec::new(),
            expected_selected_lanes: Vec::new(),
            expected_fallback_triggered: false,
        }
    }

    pub fn with_expected_selected(mut self, ids: &[&str]) -> Self {
        self.expected_selected_candidate_ids = ids
            .iter()
            .map(|id| sanitize_public_label(id, "candidate").value)
            .collect();
        self
    }

    pub fn with_expected_lanes(mut self, lanes: &[&str]) -> Self {
        self.expected_selected_lanes = ids_to_sorted_strings(lanes);
        self
    }

    pub fn with_expected_fallback(mut self, expected: bool) -> Self {
        self.expected_fallback_triggered = expected;
        self
    }

    pub fn replay(&self) -> RoutingTraceReplayReport {
        let plan = AdaptiveRoutingPlanner::new().plan_with_compute_budget(
            self.profile,
            self.base_threshold,
            self.context,
            self.budget,
            self.candidates.clone(),
        );
        let trace = RouterDecisionTrace::from_budgeted_plan(&plan);
        let actual_selected = trace.selected_candidate_ids();
        let actual_lanes = trace.selected_lanes();
        let mut blockers = Vec::new();

        if !self.expected_selected_candidate_ids.is_empty()
            && actual_selected != self.expected_selected_candidate_ids
        {
            blockers.push(format!(
                "routing_trace_replay_selected_mismatch expected={} actual={}",
                self.expected_selected_candidate_ids.join("+"),
                actual_selected.join("+")
            ));
        }
        if !self.expected_selected_lanes.is_empty() && actual_lanes != self.expected_selected_lanes
        {
            blockers.push(format!(
                "routing_trace_replay_lane_mismatch expected={} actual={}",
                self.expected_selected_lanes.join("+"),
                actual_lanes.join("+")
            ));
        }
        if trace.fallback_triggered != self.expected_fallback_triggered {
            blockers.push(format!(
                "routing_trace_replay_fallback_mismatch expected={} actual={}",
                self.expected_fallback_triggered, trace.fallback_triggered
            ));
        }
        blockers.extend(trace.blocked_reasons.iter().cloned());
        let replay_matched = blockers.is_empty();
        let visualization_json = trace.to_visualization_json();

        RoutingTraceReplayReport {
            fixture_id: self.fixture_id.clone(),
            trace_id: trace.trace_id.clone(),
            replay_matched,
            export_allowed: trace.export_allowed,
            expected_selected_candidate_ids: self.expected_selected_candidate_ids.clone(),
            actual_selected_candidate_ids: actual_selected,
            expected_selected_lanes: self.expected_selected_lanes.clone(),
            actual_selected_lanes: actual_lanes,
            fallback_triggered: trace.fallback_triggered,
            adaptive_threshold: trace.adaptive_threshold,
            blocked_reasons: blockers,
            visualization_json,
            trace,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RoutingTraceReplayReport {
    pub fixture_id: String,
    pub trace_id: String,
    pub replay_matched: bool,
    pub export_allowed: bool,
    pub expected_selected_candidate_ids: Vec<String>,
    pub actual_selected_candidate_ids: Vec<String>,
    pub expected_selected_lanes: Vec<String>,
    pub actual_selected_lanes: Vec<String>,
    pub fallback_triggered: bool,
    pub adaptive_threshold: f32,
    pub blocked_reasons: Vec<String>,
    pub visualization_json: String,
    pub trace: RouterDecisionTrace,
}

impl RoutingTraceReplayReport {
    pub fn summary_line(&self) -> String {
        format!(
            "routing_trace_replay fixture={} trace={} matched={} export_allowed={} selected={} lanes={} fallback={} threshold={:.6} blockers={}",
            self.fixture_id,
            self.trace_id,
            self.replay_matched,
            self.export_allowed,
            self.actual_selected_candidate_ids.join("+"),
            self.actual_selected_lanes.join("+"),
            self.fallback_triggered,
            self.adaptive_threshold,
            self.blocked_reasons.len()
        )
    }
}

fn fallback_reason(schedule: &ComputeBudgetSchedule) -> String {
    if schedule.fallback_triggered {
        schedule
            .notes
            .iter()
            .find(|note| note.contains("fallback"))
            .cloned()
            .unwrap_or_else(|| "fallback_fast_projection_or_anchor_hold".to_owned())
    } else {
        "none".to_owned()
    }
}

fn budget_pressure(schedule: &ComputeBudgetSchedule) -> f32 {
    if schedule.estimated_budget_tokens == 0 {
        return 0.0;
    }
    (schedule.estimated_spent_tokens as f32 / schedule.estimated_budget_tokens as f32)
        .clamp(0.0, 1.0)
}

fn stable_trace_id(schedule: &ComputeBudgetSchedule, rows: &[RouterDecisionTraceRow]) -> String {
    let mut seed = format!(
        "{}:{}:{:.6}:{:.6}:{}:{}:{}:{}:{}",
        profile_slug(schedule.profile),
        schedule.compute_budget.as_str(),
        schedule.base_threshold,
        schedule.threshold_after,
        schedule.route_fanout_before,
        schedule.route_fanout_after,
        schedule.selected_candidates,
        schedule.saved_tokens,
        schedule.wasted_compute_avoided_tokens
    );
    for row in rows {
        seed.push('|');
        seed.push_str(&row.summary_line());
    }
    format!("router-trace-{:016x}", stable_hash(&seed))
}

fn route_lane(route: Route) -> &'static str {
    match route {
        Route::FastProjection => "fast_projection",
        Route::LocalWindowAttention => "local",
        Route::GlobalAttention => "global",
        Route::ConvolutionalFusion => "convolutional_fusion",
    }
}

fn finite_unit(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn finite_f32(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

fn unit_score(value: f32) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

fn profile_slug(profile: TaskProfile) -> &'static str {
    match profile {
        TaskProfile::General => "general",
        TaskProfile::Coding => "coding",
        TaskProfile::Writing => "writing",
        TaskProfile::LongDocument => "long_document",
    }
}

fn ids_to_sorted_strings(ids: &[&str]) -> Vec<String> {
    let mut out = ids.iter().map(|id| (*id).to_owned()).collect::<Vec<_>>();
    out.sort();
    out.dedup();
    out
}

fn unique_strings(values: impl Iterator<Item = String>) -> Vec<String> {
    let mut out = values.collect::<Vec<_>>();
    out.sort();
    out.dedup();
    out
}

struct Sanitized<T> {
    value: T,
    redactions: usize,
    payload_markers: usize,
}

fn sanitize_public_label(value: &str, fallback: &str) -> Sanitized<String> {
    let text = sanitize_public_text(value, 120);
    if text.payload_markers > 0 || text.redactions > 0 {
        return Sanitized {
            value: format!("{fallback}-redacted-{:016x}", stable_hash(value)),
            redactions: text.redactions.max(1),
            payload_markers: text.payload_markers,
        };
    }
    let sanitized = text
        .value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':' | '#') {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .chars()
        .take(120)
        .collect::<String>();
    Sanitized {
        value: if sanitized.is_empty() {
            fallback.to_owned()
        } else {
            sanitized
        },
        redactions: text.redactions,
        payload_markers: text.payload_markers,
    }
}

fn sanitize_public_text(value: &str, max_chars: usize) -> Sanitized<String> {
    let payload_markers = usize::from(contains_payload_marker(value));
    let mut redactions = 0usize;
    let sanitized = value
        .split_whitespace()
        .map(|token| {
            if contains_sensitive_marker(token) {
                redactions = redactions.saturating_add(1);
                "[redacted]".to_owned()
            } else if contains_payload_marker(token) {
                redactions = redactions.saturating_add(1);
                "[payload-redacted]".to_owned()
            } else {
                token
                    .chars()
                    .filter(|ch| !ch.is_control())
                    .collect::<String>()
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    Sanitized {
        value: compact(&sanitized, max_chars),
        redactions,
        payload_markers,
    }
}

fn contains_payload_marker(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "prompt:",
        "answer:",
        "raw prompt",
        "raw_prompt",
        "raw answer",
        "raw_response",
        "conversation transcript",
        "<conversation",
        "begin private",
        "-----begin",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn contains_sensitive_marker(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains("password")
        || lower.contains("passwd")
        || lower.contains("secret")
        || lower.contains("api_key")
        || lower.contains("apikey")
        || lower.contains("token=")
        || lower.contains("access_token")
        || lower.contains("bearer")
        || lower.starts_with("sk-")
}

fn compact(value: &str, max_chars: usize) -> String {
    let mut out = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}

fn string_array_json(items: &[String]) -> String {
    let values = items
        .iter()
        .map(|item| format!("\"{}\"", json_escape(item)))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{values}]")
}

fn json_escape(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out
}

fn stable_hash(value: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hierarchy::HierarchyWeights;

    #[test]
    fn decision_trace_exports_stable_visualization_json() {
        let trace = trace_from_fixture(base_fixture().replay());
        let json = trace.to_visualization_json();

        assert!(trace.export_allowed);
        assert!(trace.blocked_reasons.is_empty());
        assert_eq!(trace.candidate_count, trace.rows.len());
        assert!(trace.adaptive_threshold >= trace.base_threshold);
        assert!(trace.selected_candidates > 0);
        assert!(json.contains("\"schema\":\"rust-norion-router-decision-trace-v1\""));
        assert!(json.contains("\"decisions\":["));
        assert!(json.contains("\"selected_lane\":"));
        assert!(json.contains("\"components\":"));
        assert!(json.contains("\"export_allowed\":true"));
        assert!(!json.contains("prompt:"));
        assert!(!json.contains("answer:"));
    }

    #[test]
    fn replay_fixture_reproduces_selected_route() {
        let report = base_fixture().replay();

        assert!(report.replay_matched, "{:?}", report.blocked_reasons);
        assert_eq!(
            report.actual_selected_candidate_ids,
            vec!["anchor".to_owned()]
        );
        assert_eq!(report.actual_selected_lanes, vec!["local".to_owned()]);
        assert!(!report.fallback_triggered);
        assert!(
            report
                .visualization_json
                .contains("\"selected_candidates\":1")
        );
    }

    #[test]
    fn trace_records_threshold_budget_pressure_and_kv_fusion_contribution() {
        let report = kv_fixture().replay();
        let trace = &report.trace;
        let kv_row = trace
            .rows
            .iter()
            .find(|row| row.candidate_id == "runtime-kv")
            .expect("runtime kv trace row");

        assert!(report.replay_matched, "{:?}", report.blocked_reasons);
        assert!(trace.adaptive_threshold < trace.base_threshold);
        assert!(trace.budget_pressure > 0.0);
        assert!(kv_row.kv_fusion_lane);
        assert!(kv_row.kv_fusion_contribution > 0.60);
        assert_eq!(kv_row.source, AdaptiveRouteSource::RuntimeKv);
    }

    #[test]
    fn trace_reports_fallback_path_when_no_candidate_selected() {
        let report = fallback_fixture().replay();
        let trace = &report.trace;

        assert!(report.replay_matched, "{:?}", report.blocked_reasons);
        assert!(trace.export_allowed);
        assert!(trace.fallback_triggered);
        assert_ne!(trace.fallback_reason, "none");
        assert_eq!(trace.selected_candidates, 0);
        assert!(trace.rows.iter().all(|row| row.fallback_path));
        assert!(
            trace
                .to_visualization_json()
                .contains("\"fallback_triggered\":true")
        );
    }

    #[test]
    fn trace_rejects_unredacted_payload_markers_without_leaking_json() {
        let report = polluted_fixture().replay();
        let json = &report.visualization_json;

        assert!(!report.replay_matched);
        assert!(!report.export_allowed);
        assert!(
            report
                .blocked_reasons
                .contains(&"router_trace_redactions_present".to_owned())
        );
        assert!(
            report
                .blocked_reasons
                .contains(&"router_trace_payload_marker_present".to_owned())
        );
        assert!(!json.contains("letmein"));
        assert!(!json.contains("sk-secret"));
        assert!(!json.contains("prompt:"));
        assert!(json.contains("\"export_allowed\":false"));
    }

    #[test]
    fn tool_result_projection_trace_is_digest_only_and_accounted() {
        let budget = RuntimeToolResultProjectionBudget::new(4_000, 400, true, false);
        let trace = ToolResultProjectionTrace::new("cargo-test", budget);
        let json = trace.to_json();

        assert!(json.contains("\"tokens_before\":1000"));
        assert!(json.contains("\"tokens_after\":100"));
        assert!(json.contains("\"tokens_saved\":900"));
        assert!(json.contains("\"handle_present\":true"));
        assert!(json.contains("\"digest_only\":false"));
        assert!(json.contains("\"write_allowed\":false"));
        assert!(!json.contains("tool output payload"));
    }

    fn trace_from_fixture(report: RoutingTraceReplayReport) -> RouterDecisionTrace {
        assert!(report.replay_matched, "{:?}", report.blocked_reasons);
        report.trace
    }

    fn base_fixture() -> RoutingTraceReplayFixture {
        let context = RoutingContext {
            profile: TaskProfile::Coding,
            hardware_pressure: 0.88,
            compute_headroom: 0.10,
            latency_budget_ms: Some(100),
            hierarchy: HierarchyWeights::new(0.20, 0.65, 0.15),
            ..RoutingContext::default()
        };
        let budget = ComputeBudgetContext {
            profile: TaskProfile::Coding,
            compute_budget: TaskComputeBudget::Low,
            validation_mode: true,
            prompt_tokens: 320,
            max_tokens: Some(64),
            route_fanout: 3,
            runtime_kv_budget_pressure: 0.0,
        };
        RoutingTraceReplayFixture::new(
            "low-budget-anchor",
            TaskProfile::Coding,
            0.48,
            context,
            budget,
            vec![
                candidate(
                    "anchor",
                    AdaptiveRouteSource::PromptChunk,
                    32,
                    0.98,
                    0.94,
                    0.98,
                    0.04,
                    0.92,
                )
                .with_anchor_required(true),
                candidate(
                    "semantic-memory",
                    AdaptiveRouteSource::SemanticMemory,
                    96,
                    0.84,
                    0.76,
                    0.80,
                    0.38,
                    0.76,
                ),
            ],
        )
        .with_expected_selected(&["anchor"])
        .with_expected_lanes(&["local"])
        .with_expected_fallback(false)
    }

    fn kv_fixture() -> RoutingTraceReplayFixture {
        let context = RoutingContext {
            profile: TaskProfile::LongDocument,
            hardware_pressure: 0.18,
            compute_headroom: 0.90,
            hierarchy: HierarchyWeights::new(0.30, 0.24, 0.46),
            ..RoutingContext::default()
        };
        let budget = ComputeBudgetContext {
            profile: TaskProfile::LongDocument,
            compute_budget: TaskComputeBudget::Expanded,
            validation_mode: false,
            prompt_tokens: 512,
            max_tokens: Some(512),
            route_fanout: 3,
            runtime_kv_budget_pressure: 0.0,
        };
        RoutingTraceReplayFixture::new(
            "expanded-kv-fusion",
            TaskProfile::LongDocument,
            0.50,
            context,
            budget,
            vec![candidate(
                "runtime-kv",
                AdaptiveRouteSource::RuntimeKv,
                160,
                0.90,
                0.88,
                0.90,
                0.22,
                0.86,
            )],
        )
        .with_expected_selected(&["runtime-kv"])
        .with_expected_lanes(&["global"])
        .with_expected_fallback(false)
    }

    fn fallback_fixture() -> RoutingTraceReplayFixture {
        let context = RoutingContext {
            profile: TaskProfile::General,
            hardware_pressure: 0.94,
            compute_headroom: 0.04,
            latency_budget_ms: Some(80),
            ..RoutingContext::default()
        };
        let budget = ComputeBudgetContext {
            profile: TaskProfile::General,
            compute_budget: TaskComputeBudget::Low,
            validation_mode: false,
            prompt_tokens: 128,
            max_tokens: Some(48),
            route_fanout: 1,
            runtime_kv_budget_pressure: 0.0,
        };
        RoutingTraceReplayFixture::new(
            "fallback-fast-projection",
            TaskProfile::General,
            0.74,
            context,
            budget,
            vec![
                candidate(
                    "weak-memory",
                    AdaptiveRouteSource::SemanticMemory,
                    128,
                    0.20,
                    0.18,
                    0.25,
                    0.90,
                    0.12,
                ),
                candidate(
                    "weak-tool",
                    AdaptiveRouteSource::ToolOutput,
                    80,
                    0.18,
                    0.10,
                    0.20,
                    0.86,
                    0.10,
                ),
            ],
        )
        .with_expected_selected(&[])
        .with_expected_lanes(&[])
        .with_expected_fallback(true)
    }

    fn polluted_fixture() -> RoutingTraceReplayFixture {
        let mut fixture = base_fixture();
        fixture.fixture_id = "polluted".to_owned();
        fixture.expected_selected_candidate_ids = Vec::new();
        fixture.expected_selected_lanes = Vec::new();
        fixture.candidates = vec![candidate(
            "prompt: password=letmein sk-secret",
            AdaptiveRouteSource::SemanticMemory,
            64,
            0.88,
            0.80,
            0.82,
            0.20,
            0.74,
        )];
        fixture
    }

    fn candidate(
        id: &str,
        source: AdaptiveRouteSource,
        estimated_tokens: usize,
        task_intent: f32,
        memory_fitness: f32,
        trust: f32,
        compute_cost: f32,
        reward_history: f32,
    ) -> AdaptiveRouteCandidate {
        AdaptiveRouteCandidate::new(
            id,
            source,
            estimated_tokens,
            AdaptiveRouteScoreComponents::new(
                task_intent,
                0.62,
                0.90,
                memory_fitness,
                0.72,
                trust,
                compute_cost,
                reward_history,
            ),
        )
    }
}
