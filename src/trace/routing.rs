use super::fields::*;
use crate::privacy_redaction::contains_private_or_executable_marker;

pub(super) fn evaluate_trace_adaptive_routing(line: &str) -> Vec<String> {
    let mut failures = Vec::new();
    let Some(routing) = json_object_after_field(line, "adaptive_routing") else {
        failures.push("adaptive_routing object is missing or invalid".to_owned());
        return failures;
    };

    let candidates = extract_json_usize_field(routing, "candidates").unwrap_or(0);
    let include = extract_json_usize_field(routing, "include").unwrap_or(0);
    let compress = extract_json_usize_field(routing, "compress").unwrap_or(0);
    let defer = extract_json_usize_field(routing, "defer").unwrap_or(0);
    let skip = extract_json_usize_field(routing, "skip").unwrap_or(0);
    let input_tokens = extract_json_usize_field(routing, "input_tokens").unwrap_or(0);
    let retained_tokens = extract_json_usize_field(routing, "retained_tokens").unwrap_or(0);
    let saved_tokens = extract_json_usize_field(routing, "saved_tokens").unwrap_or(0);
    let min_score = extract_json_f32_field(routing, "min_score").unwrap_or(f32::NAN);
    let max_score = extract_json_f32_field(routing, "max_score").unwrap_or(f32::NAN);
    let average_score = extract_json_f32_field(routing, "average_score").unwrap_or(f32::NAN);
    let actions = extract_json_string_array_field(routing, "actions").unwrap_or_default();
    let selected_routes =
        extract_json_string_array_field(routing, "selected_routes").unwrap_or_default();
    let score_summaries =
        extract_json_string_array_field(routing, "score_summaries").unwrap_or_default();
    let read_only = extract_json_bool_field(routing, "read_only");
    let write_allowed = extract_json_bool_field(routing, "write_allowed");
    let applied = extract_json_bool_field(routing, "applied");

    let decision_total = include
        .saturating_add(compress)
        .saturating_add(defer)
        .saturating_add(skip);
    if decision_total != candidates {
        failures.push(format!(
            "adaptive_routing decisions {decision_total} do not match candidates {candidates}"
        ));
    }
    if retained_tokens.saturating_add(saved_tokens) != input_tokens {
        failures.push(format!(
            "adaptive_routing retained+saved {} does not match input_tokens {input_tokens}",
            retained_tokens.saturating_add(saved_tokens)
        ));
    }
    if retained_tokens > input_tokens {
        failures.push(format!(
            "adaptive_routing retained_tokens {retained_tokens} exceeds input_tokens {input_tokens}"
        ));
    }
    if candidates > 0 && actions.is_empty() {
        failures.push("adaptive_routing candidates require action summaries".to_owned());
    }
    if include.saturating_add(compress) > 0 && selected_routes.is_empty() {
        failures.push("adaptive_routing retained candidates require selected_routes".to_owned());
    }
    if candidates > 0 && score_summaries.is_empty() {
        failures.push("adaptive_routing candidates require score_summaries".to_owned());
    }
    if score_summaries.len() > candidates {
        failures.push(format!(
            "adaptive_routing score_summaries {} exceeds candidates {candidates}",
            score_summaries.len()
        ));
    }
    if !unit_score(min_score) || !unit_score(max_score) || !unit_score(average_score) {
        failures.push(format!(
            "adaptive_routing scores must stay within 0.0..=1.0, got min={min_score:.6} max={max_score:.6} average={average_score:.6}"
        ));
    }
    if candidates > 0 && min_score > average_score {
        failures.push("adaptive_routing min_score exceeds average_score".to_owned());
    }
    if candidates > 0 && average_score > max_score {
        failures.push("adaptive_routing average_score exceeds max_score".to_owned());
    }
    if read_only != Some(true) {
        failures.push("adaptive_routing read_only must be true".to_owned());
    }
    if write_allowed != Some(false) {
        failures.push("adaptive_routing write_allowed must be false".to_owned());
    }
    if applied != Some(false) {
        failures.push("adaptive_routing applied must be false".to_owned());
    }
    for (index, summary) in score_summaries.iter().enumerate() {
        for marker in [
            "candidate_digest=",
            "source=",
            "action=",
            "route=",
            "score=",
            "threshold=",
            "verifier_rule=",
            "verifier_test=",
            "verifier_logic=",
            "verifier_reward=",
            "verifier_cluster=",
            "verifier_evidence_digest=",
        ] {
            if !summary.contains(marker) {
                failures.push(format!(
                    "adaptive_routing score summary {index} missing {marker} evidence"
                ));
            }
        }
        if summary_value(summary, "id").is_some() {
            failures.push(format!(
                "adaptive_routing score summary {index} must use candidate_digest, not raw id"
            ));
        }
        match summary_value(summary, "candidate_digest") {
            Some(value)
                if value.starts_with("redaction-digest:")
                    && !contains_private_or_executable_marker(value) => {}
            Some(_) => failures.push(format!(
                "adaptive_routing score summary {index} has invalid candidate_digest"
            )),
            None => {}
        }
        let rule = verifier_decision(summary, "verifier_rule", index, &mut failures);
        let test = verifier_decision(summary, "verifier_test", index, &mut failures);
        let logic = verifier_decision(summary, "verifier_logic", index, &mut failures);
        let reward = verifier_decision(summary, "verifier_reward", index, &mut failures);
        let cluster = verifier_decision(summary, "verifier_cluster", index, &mut failures);
        if let (Some(rule), Some(test), Some(logic), Some(reward), Some(cluster)) =
            (rule, test, logic, reward, cluster)
        {
            let expected = verifier_cluster_decision(rule, test, logic, reward);
            if cluster != expected {
                failures.push(format!(
                    "adaptive_routing score summary {index} verifier_cluster={} does not match expected {}",
                    cluster.as_str(),
                    expected.as_str()
                ));
            }
        }
        match summary_value(summary, "verifier_evidence_digest") {
            Some(value)
                if value.len() > "fnv64:".len()
                    && value.starts_with("fnv64:")
                    && !contains_private_or_executable_marker(value) => {}
            Some(_) => failures.push(format!(
                "adaptive_routing score summary {index} has invalid verifier_evidence_digest"
            )),
            None => {}
        }
        if contains_private_or_executable_marker(summary) {
            failures.push(format!(
                "adaptive_routing score summary {index} must not leak raw prompt or answer payloads"
            ));
        }
        if summary.contains("anchor=true") && summary.contains("action=skip") {
            failures.push(format!(
                "adaptive_routing score summary {index} must not skip required anchors"
            ));
        }
    }

    failures
}

pub(super) fn evaluate_trace_compute_budget(line: &str) -> Vec<String> {
    let mut failures = Vec::new();
    let Some(budget) = json_object_after_field(line, "compute_budget") else {
        failures.push("compute_budget object is missing or invalid".to_owned());
        return failures;
    };

    let task = json_object_after_field(line, "task_hierarchy");
    let routing = json_object_after_field(line, "adaptive_routing");
    let compute_budget = extract_json_string_field(budget, "budget").unwrap_or_default();
    let task_compute_budget = task
        .and_then(|task| extract_json_string_field(task, "compute_budget"))
        .unwrap_or_default();
    let base_threshold = extract_json_f32_field(budget, "base_threshold").unwrap_or(f32::NAN);
    let threshold_after = extract_json_f32_field(budget, "threshold_after").unwrap_or(f32::NAN);
    let threshold_delta = extract_json_f32_field(budget, "threshold_delta").unwrap_or(f32::NAN);
    let route_fanout_before = extract_json_usize_field(budget, "route_fanout_before").unwrap_or(0);
    let route_fanout_after = extract_json_usize_field(budget, "route_fanout_after").unwrap_or(0);
    let candidate_count = extract_json_usize_field(budget, "candidate_count").unwrap_or(0);
    let selected_candidates = extract_json_usize_field(budget, "selected_candidates").unwrap_or(0);
    let anchor_count = extract_json_usize_field(budget, "anchor_count").unwrap_or(0);
    let anchors_preserved = extract_json_bool_field(budget, "anchors_preserved");
    let anchors_preserved_count =
        extract_json_usize_field(budget, "anchors_preserved_count").unwrap_or(0);
    let low_value_skipped = extract_json_usize_field(budget, "low_value_skipped").unwrap_or(0);
    let kv_lookup_budget = extract_json_usize_field(budget, "kv_lookup_budget").unwrap_or(0);
    let kv_lookups_planned = extract_json_usize_field(budget, "kv_lookups_planned").unwrap_or(0);
    let kv_lookups_skipped = extract_json_usize_field(budget, "kv_lookups_skipped").unwrap_or(0);
    let reflection_pass_budget =
        extract_json_usize_field(budget, "reflection_pass_budget").unwrap_or(0);
    let validation_run_budget =
        extract_json_usize_field(budget, "validation_run_budget").unwrap_or(0);
    let validation_cost_tokens =
        extract_json_usize_field(budget, "validation_cost_tokens").unwrap_or(0);
    let input_tokens = extract_json_usize_field(budget, "input_tokens").unwrap_or(0);
    let retained_tokens = extract_json_usize_field(budget, "retained_tokens").unwrap_or(0);
    let saved_tokens = extract_json_usize_field(budget, "saved_tokens").unwrap_or(0);
    let estimated_budget_tokens =
        extract_json_usize_field(budget, "estimated_budget_tokens").unwrap_or(0);
    let estimated_spent_tokens =
        extract_json_usize_field(budget, "estimated_spent_tokens").unwrap_or(0);
    let wasted_compute_avoided_tokens =
        extract_json_usize_field(budget, "wasted_compute_avoided_tokens").unwrap_or(0);
    let fallback_triggered = extract_json_bool_field(budget, "fallback_triggered");
    let notes = extract_json_string_array_field(budget, "notes").unwrap_or_default();
    let read_only = extract_json_bool_field(budget, "read_only");
    let write_allowed = extract_json_bool_field(budget, "write_allowed");
    let applied = extract_json_bool_field(budget, "applied");

    if !matches!(compute_budget.as_str(), "low" | "normal" | "expanded") {
        failures.push(format!(
            "compute_budget budget {compute_budget} is not recognized"
        ));
    }
    if !task_compute_budget.is_empty() && compute_budget != task_compute_budget {
        failures.push(format!(
            "compute_budget budget {compute_budget} does not match task_hierarchy compute_budget {task_compute_budget}"
        ));
    }
    for (name, value) in [
        ("base_threshold", base_threshold),
        ("threshold_after", threshold_after),
        ("threshold_delta", threshold_delta),
    ] {
        if !unit_score(value) {
            failures.push(format!(
                "compute_budget {name} {value:.6} must stay within 0.0..=1.0"
            ));
        }
    }
    if route_fanout_before == 0 || route_fanout_after == 0 {
        failures.push("compute_budget route fanout values must be positive".to_owned());
    }
    if selected_candidates > candidate_count {
        failures.push(format!(
            "compute_budget selected_candidates {selected_candidates} exceeds candidate_count {candidate_count}"
        ));
    }
    if anchors_preserved_count > anchor_count {
        failures.push(format!(
            "compute_budget anchors_preserved_count {anchors_preserved_count} exceeds anchor_count {anchor_count}"
        ));
    }
    if anchors_preserved != Some(anchors_preserved_count == anchor_count) {
        failures.push("compute_budget anchors_preserved boolean/count mismatch".to_owned());
    }
    if retained_tokens.saturating_add(saved_tokens) != input_tokens {
        failures.push(format!(
            "compute_budget retained+saved {} does not match input_tokens {input_tokens}",
            retained_tokens.saturating_add(saved_tokens)
        ));
    }
    if wasted_compute_avoided_tokens
        > saved_tokens.saturating_add(kv_lookups_skipped.saturating_mul(16))
    {
        failures.push(format!(
            "compute_budget wasted_compute_avoided_tokens {wasted_compute_avoided_tokens} exceeds saved token evidence"
        ));
    }
    if estimated_spent_tokens > estimated_budget_tokens {
        failures.push(format!(
            "compute_budget estimated_spent_tokens {estimated_spent_tokens} exceeds estimated_budget_tokens {estimated_budget_tokens}"
        ));
    }
    if kv_lookups_planned > kv_lookup_budget {
        failures.push(format!(
            "compute_budget kv_lookups_planned {kv_lookups_planned} exceeds kv_lookup_budget {kv_lookup_budget}"
        ));
    }
    if reflection_pass_budget == 0 {
        failures.push("compute_budget reflection_pass_budget must be positive".to_owned());
    }
    if validation_run_budget > 0 && validation_cost_tokens == 0 {
        failures.push("compute_budget validation runs require validation_cost_tokens".to_owned());
    }
    if low_value_skipped > 0
        && !notes
            .iter()
            .any(|note| note == "low_value_candidates_pruned_by_fanout_budget")
    {
        failures.push("compute_budget low_value_skipped requires pruning note".to_owned());
    }
    if fallback_triggered.is_none() {
        failures.push("compute_budget fallback_triggered must be boolean".to_owned());
    }
    if read_only != Some(true) {
        failures.push("compute_budget read_only must be true".to_owned());
    }
    if write_allowed != Some(false) {
        failures.push("compute_budget write_allowed must be false".to_owned());
    }
    if applied != Some(false) {
        failures.push("compute_budget applied must be false".to_owned());
    }
    if let Some(routing) = routing {
        for (field, observed, expected) in [
            (
                "candidate_count",
                candidate_count,
                extract_json_usize_field(routing, "candidates").unwrap_or(0),
            ),
            (
                "input_tokens",
                input_tokens,
                extract_json_usize_field(routing, "input_tokens").unwrap_or(0),
            ),
            (
                "retained_tokens",
                retained_tokens,
                extract_json_usize_field(routing, "retained_tokens").unwrap_or(0),
            ),
            (
                "saved_tokens",
                saved_tokens,
                extract_json_usize_field(routing, "saved_tokens").unwrap_or(0),
            ),
        ] {
            if observed != expected {
                failures.push(format!(
                    "compute_budget {field} {observed} does not match adaptive_routing {expected}"
                ));
            }
        }
    }

    failures
}

pub(super) fn evaluate_trace_task_hierarchy(line: &str) -> Vec<String> {
    let mut failures = Vec::new();
    let Some(task) = json_object_after_field(line, "task_hierarchy") else {
        failures.push("task_hierarchy object is missing or invalid".to_owned());
        return failures;
    };

    let hierarchy_depth = extract_json_usize_field(task, "hierarchy_depth").unwrap_or(0);
    let route_fanout = extract_json_usize_field(task, "route_fanout").unwrap_or(0);
    let route_pressure = extract_json_f32_field(task, "route_pressure").unwrap_or(f32::NAN);
    let compute_reduction = extract_json_f32_field(task, "compute_reduction").unwrap_or(f32::NAN);
    let threshold_before = extract_json_f32_field(task, "threshold_before").unwrap_or(f32::NAN);
    let threshold_after = extract_json_f32_field(task, "threshold_after").unwrap_or(f32::NAN);
    let selected_lanes =
        extract_json_string_array_field(task, "selected_lanes").unwrap_or_default();
    let memory_lanes = extract_json_string_array_field(task, "memory_lanes").unwrap_or_default();
    let mutation_records = extract_json_usize_field(task, "mutation_records").unwrap_or(0);
    let mutation_summaries =
        extract_json_string_array_field(task, "mutation_summaries").unwrap_or_default();
    let replayable = extract_json_bool_field(task, "replayable");
    let runtime_applied = extract_json_bool_field(task, "runtime_applied");
    let state_write_allowed = extract_json_bool_field(task, "state_write_allowed");
    let adaptive_state_write_allowed =
        extract_json_bool_field(task, "adaptive_state_write_allowed");
    let ndkv_write_allowed = extract_json_bool_field(task, "ndkv_write_allowed");

    for marker in [
        "\"mode\":\"",
        "\"language\":\"",
        "\"compute_budget\":\"",
        "\"rollback_anchor_id\":\"task_hierarchy:",
    ] {
        if !task.contains(marker) {
            failures.push(format!("task_hierarchy missing marker {marker}"));
        }
    }
    if hierarchy_depth == 0 {
        failures.push("task_hierarchy hierarchy_depth must be positive".to_owned());
    }
    if route_fanout == 0 {
        failures.push("task_hierarchy route_fanout must be positive".to_owned());
    }
    if selected_lanes.is_empty() {
        failures.push("task_hierarchy selected_lanes must not be empty".to_owned());
    }
    if memory_lanes.is_empty() {
        failures.push("task_hierarchy memory_lanes must not be empty".to_owned());
    }
    for (name, value) in [
        ("route_pressure", route_pressure),
        ("compute_reduction", compute_reduction),
        ("threshold_before", threshold_before),
        ("threshold_after", threshold_after),
    ] {
        if !unit_score(value) {
            failures.push(format!(
                "task_hierarchy {name} {value:.6} must stay within 0.0..=1.0"
            ));
        }
    }
    if mutation_records == 0 {
        failures.push("task_hierarchy mutation_records must be positive".to_owned());
    }
    if mutation_summaries.len() != mutation_records {
        failures.push(format!(
            "task_hierarchy mutation_summaries {} do not match mutation_records {mutation_records}",
            mutation_summaries.len()
        ));
    }
    for (index, summary) in mutation_summaries.iter().enumerate() {
        for marker in ["kind=", "rollback=", "replayable=true", "preview_only=true"] {
            if !summary.contains(marker) {
                failures.push(format!(
                    "task_hierarchy mutation summary {index} missing {marker} evidence"
                ));
            }
        }
        if contains_private_or_executable_marker(summary) {
            failures.push(format!(
                "task_hierarchy mutation summary {index} must not leak raw prompt or answer payloads"
            ));
        }
    }
    if replayable != Some(true) {
        failures.push("task_hierarchy replayable must be true".to_owned());
    }
    if runtime_applied != Some(true) {
        failures.push("task_hierarchy runtime_applied must be true".to_owned());
    }
    if state_write_allowed != Some(false) {
        failures.push("task_hierarchy state_write_allowed must be false".to_owned());
    }
    if adaptive_state_write_allowed != Some(false) {
        failures.push("task_hierarchy adaptive_state_write_allowed must be false".to_owned());
    }
    if ndkv_write_allowed != Some(false) {
        failures.push("task_hierarchy ndkv_write_allowed must be false".to_owned());
    }

    failures
}

fn unit_score(score: f32) -> bool {
    score.is_finite() && (0.0..=1.0).contains(&score)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TraceVerifierDecision {
    Pass,
    HoldForReview,
    Reject,
}

impl TraceVerifierDecision {
    fn from_str(value: &str) -> Option<Self> {
        match value {
            "pass" => Some(Self::Pass),
            "hold_for_review" => Some(Self::HoldForReview),
            "reject" => Some(Self::Reject),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::HoldForReview => "hold_for_review",
            Self::Reject => "reject",
        }
    }
}

fn summary_value<'a>(summary: &'a str, key: &str) -> Option<&'a str> {
    let prefix = format!("{key}=");
    summary
        .split_whitespace()
        .find_map(|part| part.strip_prefix(&prefix))
}

fn verifier_decision(
    summary: &str,
    key: &str,
    index: usize,
    failures: &mut Vec<String>,
) -> Option<TraceVerifierDecision> {
    match summary_value(summary, key) {
        Some(value) => match TraceVerifierDecision::from_str(value) {
            Some(decision) => Some(decision),
            None => {
                failures.push(format!(
                    "adaptive_routing score summary {index} has invalid {key}={value}"
                ));
                None
            }
        },
        None => None,
    }
}

fn verifier_cluster_decision(
    rule: TraceVerifierDecision,
    test: TraceVerifierDecision,
    logic: TraceVerifierDecision,
    reward: TraceVerifierDecision,
) -> TraceVerifierDecision {
    if [rule, test, logic].contains(&TraceVerifierDecision::Reject) {
        TraceVerifierDecision::Reject
    } else if reward == TraceVerifierDecision::Reject
        || [rule, test, logic, reward].contains(&TraceVerifierDecision::HoldForReview)
    {
        TraceVerifierDecision::HoldForReview
    } else {
        TraceVerifierDecision::Pass
    }
}
