use super::fields::*;

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
        for marker in ["source=", "action=", "route=", "score=", "threshold="] {
            if !summary.contains(marker) {
                failures.push(format!(
                    "adaptive_routing score summary {index} missing {marker} evidence"
                ));
            }
        }
        if summary.contains("prompt:") || summary.contains("answer:") {
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
        if summary.contains("prompt:") || summary.contains("answer:") {
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
