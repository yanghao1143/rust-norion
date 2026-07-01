use super::*;

#[test]
fn trace_schema_gate_rejects_live_evolution_mismatch() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace live evolution mismatch", TaskProfile::General),
        &mut backend,
    );
    let line = trace_json_line(
        "trace live evolution mismatch",
        TaskProfile::General,
        5,
        &outcome,
    )
    .replacen("\"live_memory_updates\":0", "\"live_memory_updates\":99", 1);

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("live_memory_updates 99")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_live_evolution_without_cumulative_ledger() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace live evolution ledger mismatch", TaskProfile::General),
        &mut backend,
    );
    let line = trace_json_line(
        "trace live evolution ledger mismatch",
        TaskProfile::General,
        5,
        &outcome,
    )
    .replacen("\"live_inference_runs\":1", "\"live_inference_runs\":0", 1);

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("live_inference_runs 0")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_live_online_reward_count_mismatch() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace live online reward mismatch", TaskProfile::General),
        &mut backend,
    );
    let line = increment_trace_object_usize(
        &trace_json_line(
            "trace live online reward mismatch",
            TaskProfile::General,
            5,
            &outcome,
        ),
        "live_evolution",
        "live_online_reward_feedbacks",
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("live_online_reward_feedbacks")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_live_online_reward_strength_mismatch() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new(
            "trace live online reward strength mismatch",
            TaskProfile::General,
        ),
        &mut backend,
    );
    let line = trace_json_line(
        "trace live online reward strength mismatch",
        TaskProfile::General,
        5,
        &outcome,
    );
    let original_strength = extract_json_f32_field(&line, "live_online_reward_strength").unwrap();
    let line = replace_trace_object_f32(
        &line,
        "live_evolution",
        "live_online_reward_strength",
        original_strength + 1.0,
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures.iter().any(|failure| {
            failure.contains("live_online_reward_strength")
                && failure.contains("does not match reinforcement+penalty strength")
        }),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_live_online_reward_feedback_without_strength() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new(
            "trace live online reward missing strength",
            TaskProfile::General,
        ),
        &mut backend,
    );
    let line = trace_json_line(
        "trace live online reward missing strength",
        TaskProfile::General,
        5,
        &outcome,
    );
    let line =
        replace_trace_object_f32(&line, "live_evolution", "live_online_reward_strength", 0.0);
    let line = replace_trace_object_f32(
        &line,
        "live_evolution",
        "live_online_reward_reinforcement_strength",
        0.0,
    );
    let line = replace_trace_object_f32(
        &line,
        "live_evolution",
        "live_online_reward_penalty_strength",
        0.0,
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures.iter().any(|failure| {
            failure.contains("live_online_reward_strength")
                && failure.contains("requires positive strength when feedbacks > 0")
        }),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_cumulative_live_online_reward_count_mismatch() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new(
            "trace cumulative live online reward mismatch",
            TaskProfile::General,
        ),
        &mut backend,
    );
    let line = increment_trace_object_usize(
        &trace_json_line(
            "trace cumulative live online reward mismatch",
            TaskProfile::General,
            5,
            &outcome,
        ),
        "evolution_ledger",
        "cumulative_live_online_reward_feedbacks",
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("cumulative_live_online_reward_feedbacks")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_cumulative_live_online_reward_strength_mismatch() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new(
            "trace cumulative live online reward strength mismatch",
            TaskProfile::General,
        ),
        &mut backend,
    );
    let line = trace_json_line(
        "trace cumulative live online reward strength mismatch",
        TaskProfile::General,
        5,
        &outcome,
    );
    let original_strength =
        extract_json_f32_field(&line, "cumulative_live_online_reward_strength").unwrap();
    let line = replace_trace_object_f32(
        &line,
        "evolution_ledger",
        "cumulative_live_online_reward_strength",
        original_strength + 1.0,
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures.iter().any(|failure| {
            failure.contains("cumulative_live_online_reward_strength")
                && failure.contains("does not match reinforcement+penalty strength")
        }),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_cumulative_live_online_reward_feedback_without_strength() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new(
            "trace cumulative live online reward missing strength",
            TaskProfile::General,
        ),
        &mut backend,
    );
    let line = trace_json_line(
        "trace cumulative live online reward missing strength",
        TaskProfile::General,
        5,
        &outcome,
    );
    let line = replace_trace_object_f32(
        &line,
        "evolution_ledger",
        "cumulative_live_online_reward_strength",
        0.0,
    );
    let line = replace_trace_object_f32(
        &line,
        "evolution_ledger",
        "cumulative_live_online_reward_reinforcement_strength",
        0.0,
    );
    let line = replace_trace_object_f32(
        &line,
        "evolution_ledger",
        "cumulative_live_online_reward_penalty_strength",
        0.0,
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures.iter().any(|failure| {
            failure.contains("cumulative_live_online_reward_strength")
                && failure.contains("requires positive strength when feedbacks > 0")
        }),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_accepts_drift_rollback_ledger_consistency() {
    let line = rollback_trace_line();

    let failures = evaluate_trace_schema_line(&line);

    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn trace_schema_gate_rejects_drift_rollback_without_cumulative_ledger() {
    let line = rollback_trace_line().replacen(
        "\"cumulative_drift_rollbacks\":1",
        "\"cumulative_drift_rollbacks\":0",
        1,
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("cumulative_drift_rollbacks 0")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_rollback_that_writes_memory() {
    let line = rollback_trace_line().replacen("\"memory_write\":false", "\"memory_write\":true", 1);

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("rollback requires memory_write=false")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_accepts_auto_replay_ledger_consistency() {
    let line = auto_replay_trace_line();

    let failures = evaluate_trace_schema_line(&line);

    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn trace_schema_gate_accepts_auto_replay_business_contract_ledger_consistency() {
    let line = business_contract_auto_replay_trace_line();
    let auto_replay = json_object_after_field(&line, "auto_replay").expect("auto replay object");
    let evolution_ledger =
        json_object_after_field(&line, "evolution_ledger").expect("evolution ledger object");

    assert_eq!(
        extract_json_usize_field(auto_replay, "business_contract_items"),
        Some(1)
    );
    assert_eq!(
        extract_json_usize_field(
            evolution_ledger,
            "cumulative_replay_business_contract_items"
        ),
        Some(1)
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn trace_schema_gate_accepts_auto_replay_runtime_kv_budget_pressure() {
    let line = runtime_kv_budget_pressure_auto_replay_trace_line();
    let auto_replay = json_object_after_field(&line, "auto_replay").expect("auto replay object");

    assert_eq!(
        extract_json_usize_field(auto_replay, "runtime_kv_budget_pressure_items"),
        Some(1)
    );
    assert_eq!(
        extract_json_f32_field(auto_replay, "avg_runtime_kv_budget_pressure"),
        Some(0.4)
    );
    assert_eq!(
        extract_json_f32_field(auto_replay, "max_runtime_kv_budget_pressure"),
        Some(0.8)
    );
    assert_eq!(
        extract_json_usize_field(auto_replay, "runtime_kv_weak_import_pressure_items"),
        Some(1)
    );
    assert_eq!(
        extract_json_f32_field(auto_replay, "avg_runtime_kv_weak_import_pressure"),
        Some(0.3)
    );
    assert_eq!(
        extract_json_f32_field(auto_replay, "max_runtime_kv_weak_import_pressure"),
        Some(0.6)
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(failures.is_empty(), "{failures:?}");

    let path = temp_path("trace-schema-auto-replay-closed-loop-counters");
    std::fs::write(&path, format!("{line}\n")).unwrap();
    let report = evaluate_trace_schema_jsonl(&path).unwrap();

    assert!(report.passed, "{:?}", report.failures);
    assert_eq!(report.auto_replay_live_memory_feedback_items, 1);
    assert_eq!(report.auto_replay_live_memory_feedback_updates, 1);
    assert_eq!(report.auto_replay_live_memory_feedback_reinforcements, 1);
    assert_eq!(report.auto_replay_live_memory_feedback_detail_items, 1);
    assert_eq!(report.auto_replay_live_memory_feedback_applied, 1);
    assert_eq!(
        report.auto_replay_live_memory_feedback_strength_delta_milli,
        250
    );
    assert_eq!(report.auto_replay_recursive_runtime_items, 1);
    assert_eq!(report.auto_replay_recursive_runtime_calls, 2);
    assert_eq!(report.auto_replay_avg_recursive_call_pressure_milli, 500);
    assert_eq!(report.auto_replay_max_recursive_call_pressure_milli, 750);
    assert_eq!(report.auto_replay_runtime_kv_budget_pressure_items, 1);
    assert_eq!(report.auto_replay_avg_runtime_kv_budget_pressure_milli, 400);
    assert_eq!(report.auto_replay_max_runtime_kv_budget_pressure_milli, 800);
    assert_eq!(report.auto_replay_runtime_kv_weak_import_pressure_items, 1);
    assert_eq!(
        report.auto_replay_avg_runtime_kv_weak_import_pressure_milli,
        300
    );
    assert_eq!(
        report.auto_replay_max_runtime_kv_weak_import_pressure_milli,
        600
    );
    assert!(
        report
            .summary_line()
            .contains("auto_replay_recursive_runtime_calls=2")
    );
    cleanup(path);
}

#[test]
fn trace_schema_gate_rejects_auto_replay_runtime_kv_budget_pressure_item_overflow() {
    let line = replace_trace_object_usize(
        &runtime_kv_budget_pressure_auto_replay_trace_line(),
        "auto_replay",
        "runtime_kv_budget_pressure_items",
        99,
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("runtime_kv_budget_pressure_items 99")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_auto_replay_runtime_kv_budget_pressure_ordering() {
    let line = replace_trace_object_f32(
        &runtime_kv_budget_pressure_auto_replay_trace_line(),
        "auto_replay",
        "avg_runtime_kv_budget_pressure",
        0.9,
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("avg_runtime_kv_budget_pressure")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_auto_replay_runtime_kv_weak_import_pressure_ordering() {
    let line = replace_trace_object_f32(
        &runtime_kv_budget_pressure_auto_replay_trace_line(),
        "auto_replay",
        "avg_runtime_kv_weak_import_pressure",
        0.9,
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("avg_runtime_kv_weak_import_pressure")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_auto_replay_business_contract_pass_fail_mismatch() {
    let line = increment_trace_object_usize(
        &business_contract_auto_replay_trace_line(),
        "auto_replay",
        "business_contract_failed",
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures.iter().any(|failure| {
            failure.contains("auto_replay business_contract_items")
                && failure.contains("business_contract_passed+business_contract_failed")
        }),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_auto_replay_business_contract_raw_audit_mismatch() {
    let line = increment_trace_object_usize(
        &business_contract_auto_replay_trace_line(),
        "auto_replay",
        "business_contract_raw_passed",
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures.iter().any(|failure| {
            failure.contains("auto_replay business_contract_items")
                && failure.contains("business_contract_raw_passed+business_contract_raw_failed")
        }),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_auto_replay_business_contract_normalization_mismatch() {
    let line = increment_trace_object_usize(
        &business_contract_auto_replay_trace_line(),
        "auto_replay",
        "business_contract_sanitized",
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures.iter().any(|failure| {
            failure.contains("auto_replay business_contract_response_normalized")
                && failure
                    .contains("business_contract_sanitized+business_contract_canonical_fallbacks")
        }),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_auto_replay_business_contract_cumulative_mismatch() {
    let line = replace_trace_object_usize(
        &business_contract_auto_replay_trace_line(),
        "evolution_ledger",
        "cumulative_replay_business_contract_items",
        0,
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("cumulative_replay_business_contract_items")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_auto_replay_memory_mismatch() {
    let line = auto_replay_trace_line().replacen(
        "\"touched_memories\":",
        "\"touched_memories\":99,\"_old_touched_memories\":",
        1,
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("auto_replay touched_memories 99")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_auto_replay_live_feedback_detail_mismatch() {
    let line = auto_replay_trace_line().replacen(
        "\"live_memory_feedback_detail_items\":0,\"live_memory_feedback_applied\":0",
        "\"live_memory_feedback_detail_items\":1,\"live_memory_feedback_applied\":99",
        1,
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("live_memory_feedback_applied+missing")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_auto_replay_cumulative_live_feedback_detail_mismatch() {
    let line = auto_replay_trace_line()
        .replacen(
            "\"live_memory_feedback_items\":0",
            "\"live_memory_feedback_items\":1",
            1,
        )
        .replacen(
            "\"live_memory_feedback_detail_items\":0,\"live_memory_feedback_applied\":0",
            "\"live_memory_feedback_detail_items\":1,\"live_memory_feedback_applied\":1",
            1,
        );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures.iter().any(|failure| {
            failure.contains("cumulative_replay_live_memory_feedback_detail_items")
        }),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_auto_replay_cumulative_live_evolution_mismatch() {
    let line = auto_replay_trace_line();
    let live_evolution_items = extract_json_usize_field(&line, "live_evolution_items").unwrap();
    let cumulative_live_evolution_items =
        extract_json_usize_field(&line, "cumulative_replay_live_evolution_items").unwrap();
    assert!(live_evolution_items > 0, "{line}");
    assert!(
        cumulative_live_evolution_items >= live_evolution_items,
        "{line}"
    );
    let line = line.replacen(
        &format!("\"cumulative_replay_live_evolution_items\":{cumulative_live_evolution_items}"),
        "\"cumulative_replay_live_evolution_items\":0",
        1,
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| { failure.contains("cumulative_replay_live_evolution_items") }),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_auto_replay_live_evolution_online_reward_count_mismatch() {
    let line = increment_trace_object_usize(
        &auto_replay_trace_line(),
        "auto_replay",
        "live_evolution_online_reward_feedbacks",
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures.iter().any(|failure| {
            failure.contains("auto_replay live_evolution_online_reward_feedbacks")
        }),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_auto_replay_live_evolution_online_reward_strength_mismatch() {
    let line = auto_replay_trace_line();
    let original_strength =
        extract_json_f32_field(&line, "live_evolution_online_reward_strength").unwrap();
    let line = replace_trace_object_f32(
        &line,
        "auto_replay",
        "live_evolution_online_reward_strength",
        original_strength + 1.0,
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures.iter().any(|failure| {
            failure.contains("auto_replay live_evolution_online_reward_strength")
                && failure.contains("does not match reinforcement+penalty strength")
        }),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_auto_replay_live_evolution_online_reward_feedback_without_strength() {
    let line = auto_replay_trace_line();
    let line = replace_trace_object_f32(
        &line,
        "auto_replay",
        "live_evolution_online_reward_strength",
        0.0,
    );
    let line = replace_trace_object_f32(
        &line,
        "auto_replay",
        "live_evolution_online_reward_reinforcement_strength",
        0.0,
    );
    let line = replace_trace_object_f32(
        &line,
        "auto_replay",
        "live_evolution_online_reward_penalty_strength",
        0.0,
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures.iter().any(|failure| {
            failure.contains("auto_replay live_evolution_online_reward_strength")
                && failure.contains("requires positive strength when feedbacks > 0")
        }),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_cumulative_replay_live_evolution_online_reward_count_mismatch() {
    let line = increment_trace_object_usize(
        &auto_replay_trace_line(),
        "evolution_ledger",
        "cumulative_replay_live_evolution_online_reward_feedbacks",
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures.iter().any(|failure| {
            failure.contains("cumulative_replay_live_evolution_online_reward_feedbacks")
        }),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_cumulative_replay_live_evolution_online_reward_strength_mismatch() {
    let line = auto_replay_trace_line();
    let original_strength = extract_json_f32_field(
        &line,
        "cumulative_replay_live_evolution_online_reward_strength",
    )
    .unwrap();
    let line = replace_trace_object_f32(
        &line,
        "evolution_ledger",
        "cumulative_replay_live_evolution_online_reward_strength",
        original_strength + 1.0,
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures.iter().any(|failure| {
            failure.contains("cumulative_replay_live_evolution_online_reward_strength")
                && failure.contains("does not match reinforcement+penalty strength")
        }),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_cumulative_replay_live_evolution_online_reward_feedback_without_strength()
 {
    let line = auto_replay_trace_line();
    let line = replace_trace_object_f32(
        &line,
        "evolution_ledger",
        "cumulative_replay_live_evolution_online_reward_strength",
        0.0,
    );
    let line = replace_trace_object_f32(
        &line,
        "evolution_ledger",
        "cumulative_replay_live_evolution_online_reward_reinforcement_strength",
        0.0,
    );
    let line = replace_trace_object_f32(
        &line,
        "evolution_ledger",
        "cumulative_replay_live_evolution_online_reward_penalty_strength",
        0.0,
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures.iter().any(|failure| {
            failure.contains("cumulative_replay_live_evolution_online_reward_strength")
                && failure.contains("requires positive strength when feedbacks > 0")
        }),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_auto_replay_cumulative_live_evolution_critical_reflection_mismatch() {
    let line = auto_replay_trace_line();
    let live_critical_reflection_issues =
        extract_json_usize_field(&line, "live_evolution_critical_reflection_issues").unwrap();
    let cumulative_critical_reflection_issues = extract_json_usize_field(
        &line,
        "cumulative_replay_live_evolution_critical_reflection_issues",
    )
    .unwrap();
    assert!(
        cumulative_critical_reflection_issues >= live_critical_reflection_issues,
        "{line}"
    );
    let required_live_critical_reflection_issues =
        live_critical_reflection_issues.saturating_add(1);
    let line = line
            .replacen(
                &format!(
                    "\"live_evolution_critical_reflection_issues\":{live_critical_reflection_issues}"
                ),
                &format!(
                    "\"live_evolution_critical_reflection_issues\":{required_live_critical_reflection_issues}"
                ),
                1,
            )
            .replacen(
                &format!(
                    "\"cumulative_replay_live_evolution_critical_reflection_issues\":{cumulative_critical_reflection_issues}"
                ),
                "\"cumulative_replay_live_evolution_critical_reflection_issues\":0",
                1,
            );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures.iter().any(|failure| {
            failure.contains("cumulative_replay_live_evolution_critical_reflection_issues")
        }),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_auto_replay_cumulative_live_evolution_revision_action_mismatch() {
    let line = auto_replay_trace_line();
    let live_revision_actions =
        extract_json_usize_field(&line, "live_evolution_revision_actions").unwrap();
    let cumulative_revision_actions =
        extract_json_usize_field(&line, "cumulative_replay_live_evolution_revision_actions")
            .unwrap();
    assert!(
        cumulative_revision_actions >= live_revision_actions,
        "{line}"
    );
    let required_live_revision_actions = live_revision_actions.saturating_add(1);
    let line = line
            .replacen(
                &format!("\"live_evolution_revision_actions\":{live_revision_actions}"),
                &format!("\"live_evolution_revision_actions\":{required_live_revision_actions}"),
                1,
            )
            .replacen(
                &format!(
                    "\"cumulative_replay_live_evolution_revision_actions\":{cumulative_revision_actions}"
                ),
                "\"cumulative_replay_live_evolution_revision_actions\":0",
                1,
            );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures.iter().any(|failure| {
            failure.contains("cumulative_replay_live_evolution_revision_actions")
        }),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_auto_replay_without_cumulative_ledger() {
    let line = auto_replay_trace_line().replacen("\"replay_runs\":1", "\"replay_runs\":0", 1);

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("replay_runs 0")),
        "{failures:?}"
    );
}
