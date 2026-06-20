use super::*;

#[test]
fn trace_schema_gate_accepts_memory_governance_consistency() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace memory governance consistency", TaskProfile::General),
        &mut backend,
    );
    let line = trace_json_line(
        "trace memory governance consistency",
        TaskProfile::General,
        5,
        &outcome,
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn trace_schema_gate_rejects_memory_feedback_count_mismatch() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace memory feedback mismatch", TaskProfile::General),
        &mut backend,
    );
    let line = replace_in_trace_object(
        &trace_json_line(
            "trace memory feedback mismatch",
            TaskProfile::General,
            5,
            &outcome,
        ),
        "memory",
        "\"feedback_updates\":0",
        "\"feedback_updates\":1",
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("memory feedback_updates")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_incomplete_runtime_device_execution() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new(
            "trace incomplete runtime device execution",
            TaskProfile::General,
        ),
        &mut backend,
    );
    let line = replace_in_trace_object(
        &trace_json_line(
            "trace incomplete runtime device execution",
            TaskProfile::General,
            5,
            &outcome,
        ),
        "runtime_diagnostics",
        "\"has_forward_signal\":false",
        "\"has_forward_signal\":true",
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("device execution diagnostics are incomplete")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_runtime_device_execution_mismatch() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new(
            "trace runtime device execution mismatch",
            TaskProfile::General,
        ),
        &mut backend,
    );
    let line = trace_json_line(
        "trace runtime device execution mismatch",
        TaskProfile::General,
        5,
        &outcome,
    );
    let line = replace_in_trace_object(
        &line,
        "runtime_diagnostics",
        "\"device_profile\":null",
        "\"device_profile\":\"server\"",
    );
    let line = replace_in_trace_object(
        &line,
        "runtime_diagnostics",
        "\"primary_lane\":null",
        "\"primary_lane\":\"cuda\"",
    );
    let line = replace_in_trace_object(
        &line,
        "runtime_diagnostics",
        "\"fallback_lane\":null",
        "\"fallback_lane\":\"cpu-simd\"",
    );
    let line = replace_in_trace_object(
        &line,
        "runtime_diagnostics",
        "\"memory_mode\":null",
        "\"memory_mode\":\"gpu-resident\"",
    );
    let line = replace_in_trace_object(
        &line,
        "runtime_diagnostics",
        "\"device_execution_source\":null",
        "\"device_execution_source\":\"runtime-reported\"",
    );
    let line = replace_in_trace_object(
        &line,
        "runtime_diagnostics",
        "\"has_forward_signal\":false",
        "\"has_forward_signal\":true",
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("runtime_diagnostics device_profile=server")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_invalid_runtime_kv_precision_order() {
    let mut engine = NoironEngine::new();
    let mut backend = RuntimePrecisionBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace runtime kv precision", TaskProfile::General),
        &mut backend,
    );
    let line = trace_json_line(
        "trace runtime kv precision",
        TaskProfile::General,
        5,
        &outcome,
    );
    let line = replace_in_trace_object(
        &line,
        "runtime_diagnostics",
        "\"hot_kv_precision_bits\":8,\"cold_kv_precision_bits\":4",
        "\"hot_kv_precision_bits\":4,\"cold_kv_precision_bits\":8",
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures.iter().any(|failure| failure
            .contains("runtime_diagnostics device execution is missing valid KV precision")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_runtime_kv_precision_execution_mismatch() {
    let mut engine = NoironEngine::new();
    let mut backend = RuntimePrecisionBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace runtime kv precision mismatch", TaskProfile::General),
        &mut backend,
    );
    let line = trace_json_line(
        "trace runtime kv precision mismatch",
        TaskProfile::General,
        5,
        &outcome,
    );
    let line = replace_in_trace_object(
        &line,
        "runtime_diagnostics",
        "\"hot_kv_precision_bits\":8",
        "\"hot_kv_precision_bits\":4",
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("runtime_diagnostics hot_kv_precision_bits=4")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_retention_count_mismatch() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace retention count mismatch", TaskProfile::General),
        &mut backend,
    );
    let line = replace_in_trace_object(
        &trace_json_line(
            "trace retention count mismatch",
            TaskProfile::General,
            5,
            &outcome,
        ),
        "retention",
        "\"removed\":0",
        "\"removed\":2",
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("retention") && failure.contains("after+removed")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_compaction_count_mismatch() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace compaction count mismatch", TaskProfile::General),
        &mut backend,
    );
    let line = replace_in_trace_object(
        &trace_json_line(
            "trace compaction count mismatch",
            TaskProfile::General,
            5,
            &outcome,
        ),
        "memory_compaction",
        "\"merged\":0",
        "\"merged\":1",
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("memory_compaction merged 1")),
        "{failures:?}"
    );
}

#[test]
fn scoped_json_object_extraction_keeps_duplicate_fields_separate() {
    let line = "{\"retention\":{\"before\":1,\"after\":1},\"memory_compaction\":{\"before\":3,\"after\":2,\"note\":\"keeps {quoted} braces\"}}";
    let retention = json_object_after_field(line, "retention").unwrap();
    let compaction = json_object_after_field(line, "memory_compaction").unwrap();

    assert_eq!(extract_json_usize_field(retention, "before"), Some(1));
    assert_eq!(extract_json_usize_field(compaction, "before"), Some(3));
    assert_eq!(extract_json_usize_field(compaction, "after"), Some(2));
}
