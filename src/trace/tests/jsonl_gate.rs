use super::*;

#[test]
fn trace_schema_jsonl_gate_checks_non_empty_records() {
    let path = temp_path("trace-schema");
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace schema jsonl", TaskProfile::General),
        &mut backend,
    );
    fs::write(
        &path,
        format!(
            "\n{}\n",
            trace_json_line("trace schema jsonl", TaskProfile::General, 8, &outcome)
        ),
    )
    .unwrap();

    let report = evaluate_trace_schema_jsonl(&path).unwrap();

    assert!(report.passed, "{:?}", report.failures);
    assert_eq!(report.checked_lines, 1);
    assert_eq!(report.rust_check_events, 0);
    assert_eq!(report.rust_check_feedback_applied, 0);
    assert_eq!(report.runtime_error_events, 0);
    assert_eq!(report.runtime_timeout_events, 0);
    assert!(report.summary_line().contains("passed=true"));
    assert!(report.summary_line().contains("rust_check_events=0"));
    assert!(report.summary_line().contains("runtime_error_events=0"));
    assert!(report.summary_line().contains("runtime_timeout_events=0"));
    cleanup(path);
}

#[test]
fn trace_schema_jsonl_gate_aggregates_memory_admission_and_kv_fusion() {
    let path = temp_path("trace-schema-memory-admission");
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new(
            "trace schema jsonl memory admission for a Rust runtime adapter tool",
            TaskProfile::Coding,
        ),
        &mut backend,
    );
    let line = trace_json_line(
        "trace schema jsonl memory admission for a Rust runtime adapter tool",
        TaskProfile::Coding,
        8,
        &outcome,
    );
    fs::write(&path, format!("{line}\n")).unwrap();

    let report = evaluate_trace_schema_jsonl(&path).unwrap();
    let admission = json_object_after_field(&line, "memory_admission").unwrap();
    let fusion = json_object_after_field(&line, "kv_fusion").unwrap();

    assert!(report.passed, "{:?}", report.failures);
    assert_eq!(report.checked_lines, 1);
    assert_eq!(report.memory_admission_events, 1);
    assert_eq!(
        report.memory_admission_candidates,
        extract_json_usize_field(admission, "candidates").unwrap()
    );
    assert!(report.memory_admission_candidates >= 1);
    assert_eq!(
        report.memory_admission_review_packets,
        report.memory_admission_candidates
    );
    assert_eq!(
        report.memory_admission_ledger_records,
        report.memory_admission_candidates
    );
    assert_eq!(report.memory_admission_ledger_authorized, 0);
    assert_eq!(report.memory_admission_ledger_applied, 0);
    assert_eq!(report.memory_admission_admitted, 0);
    assert_eq!(report.kv_fusion_events, 1);
    assert_eq!(
        report.kv_fusion_candidates,
        extract_json_usize_field(fusion, "candidates").unwrap()
    );
    assert!(report.kv_fusion_candidates >= 1);
    assert_eq!(
        report
            .kv_fusion_retained_tokens
            .saturating_add(report.kv_fusion_saved_tokens),
        report.kv_fusion_input_tokens
    );
    assert!(
        report
            .summary_line()
            .contains("memory_admission_ledger_records=")
    );
    assert!(report.summary_line().contains("kv_fusion_saved_tokens="));
    cleanup(path);
}
