use super::*;

#[test]
fn trace_schema_gate_accepts_runtime_kv_storage_consistency() {
    let line = runtime_kv_trace_line()
        .replacen("\"exported_kv_blocks\":0", "\"exported_kv_blocks\":1", 1)
        .replacen(
            "\"has_runtime_kv_activity_signal\":false",
            "\"has_runtime_kv_activity_signal\":true",
            1,
        )
        .replacen(
            "\"has_forward_signal\":false",
            "\"has_forward_signal\":true",
            1,
        )
        .replacen("\"runtime_kv_exported\":0", "\"runtime_kv_exported\":1", 1)
        .replacen("\"runtime_kv_stored\":0", "\"runtime_kv_stored\":1", 1)
        .replacen(
            "\"live_stored_runtime_kv_memories\":0",
            "\"live_stored_runtime_kv_memories\":1",
            1,
        )
        .replacen(
            "\"live_stored_memory_updates\":2",
            "\"live_stored_memory_updates\":3",
            1,
        )
        .replacen(
            "\"cumulative_live_stored_runtime_kv_memories\":0",
            "\"cumulative_live_stored_runtime_kv_memories\":1",
            1,
        )
        .replacen(
            "\"cumulative_live_stored_memory_updates\":2",
            "\"cumulative_live_stored_memory_updates\":3",
            1,
        )
        .replacen("\"memory_write\":false", "\"memory_write\":true", 1)
        .replacen("\"runtime_kv_write\":false", "\"runtime_kv_write\":true", 1)
        .replacen("\"revision_passes\":1", "\"revision_passes\":0", 1);

    let failures = evaluate_trace_schema_line(&line);

    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn trace_schema_gate_accepts_fast_path_watch_runtime_kv_hold() {
    let line = fast_path_watch_trace_line();

    let failures = evaluate_trace_schema_line(&line);

    assert!(line.contains("\"runtime_kv_exported\":1"));
    assert!(line.contains("\"runtime_kv_stored\":0"));
    assert!(line.contains("\"runtime_kv_hold\":true"));
    assert!(line.contains("\"runtime_kv_held\":1"));
    assert!(line.contains("\"memory_write\":true"));
    assert!(line.contains("\"runtime_kv_write\":false"));
    assert!(line.contains("\"route:fast_path_watch\""));
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn trace_schema_gate_rejects_runtime_kv_hold_flag_mismatch() {
    let line = fast_path_watch_trace_line().replacen(
        "\"runtime_kv_hold\":true",
        "\"runtime_kv_hold\":false",
        1,
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("runtime_kv_hold false")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_runtime_kv_held_count_mismatch() {
    let line =
        fast_path_watch_trace_line().replacen("\"runtime_kv_held\":1", "\"runtime_kv_held\":0", 1);

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("runtime_kv_held 0")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_fast_path_watch_runtime_kv_storage() {
    let line = fast_path_watch_trace_line()
        .replacen("\"runtime_kv_stored\":0", "\"runtime_kv_stored\":1", 1)
        .replacen("\"runtime_kv_hold\":true", "\"runtime_kv_hold\":false", 1)
        .replacen("\"runtime_kv_held\":1", "\"runtime_kv_held\":0", 1)
        .replacen(
            "\"live_stored_runtime_kv_memories\":0",
            "\"live_stored_runtime_kv_memories\":1",
            1,
        )
        .replacen("\"runtime_kv_write\":false", "\"runtime_kv_write\":true", 1);

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures.iter().any(|failure| {
            failure.contains("route:fast_path_watch requires runtime_kv_write=false")
        }),
        "{failures:?}"
    );
    assert!(
        failures.iter().any(|failure| {
            failure.contains("route:fast_path_watch forbids runtime KV storage")
        }),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_runtime_kv_storage_without_write_permission() {
    let line = runtime_kv_trace_line()
        .replacen("\"exported_kv_blocks\":0", "\"exported_kv_blocks\":1", 1)
        .replacen("\"runtime_kv_exported\":0", "\"runtime_kv_exported\":1", 1)
        .replacen("\"runtime_kv_stored\":0", "\"runtime_kv_stored\":1", 1)
        .replacen("\"memory_write\":false", "\"memory_write\":true", 1)
        .replacen("\"runtime_kv_write\":true", "\"runtime_kv_write\":false", 1)
        .replacen("\"revision_passes\":1", "\"revision_passes\":0", 1);

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("runtime_kv_write=true")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_runtime_kv_storage_without_memory_write() {
    let line = runtime_kv_trace_line()
        .replacen("\"exported_kv_blocks\":0", "\"exported_kv_blocks\":1", 1)
        .replacen("\"runtime_kv_exported\":0", "\"runtime_kv_exported\":1", 1)
        .replacen("\"runtime_kv_stored\":0", "\"runtime_kv_stored\":1", 1)
        .replacen("\"runtime_kv_write\":false", "\"runtime_kv_write\":true", 1)
        .replacen("\"memory_write\":true", "\"memory_write\":false", 1)
        .replacen("\"revision_passes\":1", "\"revision_passes\":0", 1);

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("memory_write=true")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_runtime_kv_storage_after_revision() {
    let line = runtime_kv_trace_line()
        .replacen("\"exported_kv_blocks\":0", "\"exported_kv_blocks\":1", 1)
        .replacen("\"runtime_kv_exported\":0", "\"runtime_kv_exported\":1", 1)
        .replacen("\"runtime_kv_stored\":0", "\"runtime_kv_stored\":1", 1)
        .replacen("\"memory_write\":false", "\"memory_write\":true", 1)
        .replacen("\"runtime_kv_write\":false", "\"runtime_kv_write\":true", 1)
        .replacen("\"revision_passes\":0", "\"revision_passes\":1", 1);

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("revision_passes=0")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_runtime_kv_export_mismatch() {
    let line = runtime_kv_trace_line()
        .replacen("\"exported_kv_blocks\":0", "\"exported_kv_blocks\":2", 1)
        .replacen("\"runtime_kv_exported\":0", "\"runtime_kv_exported\":1", 1);

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("exported_kv_blocks")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_runtime_kv_stored_above_exported() {
    let line = runtime_kv_trace_line()
        .replacen("\"exported_kv_blocks\":0", "\"exported_kv_blocks\":1", 1)
        .replacen("\"runtime_kv_exported\":0", "\"runtime_kv_exported\":1", 1)
        .replacen("\"runtime_kv_stored\":0", "\"runtime_kv_stored\":2", 1)
        .replacen("\"memory_write\":false", "\"memory_write\":true", 1)
        .replacen("\"runtime_kv_write\":false", "\"runtime_kv_write\":true", 1)
        .replacen("\"revision_passes\":1", "\"revision_passes\":0", 1);

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures.iter().any(|failure| failure.contains("exceeds")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_reports_missing_required_fields() {
    let failures = evaluate_trace_schema_line("{\"schema\":\"other\"}");

    assert!(failures.iter().any(|failure| failure.contains("schema")));
    assert!(failures.iter().any(|failure| failure.contains("route")));
    assert!(failures.iter().any(|failure| failure.contains("retention")));
}

#[test]
fn trace_schema_gate_rejects_device_contract_mismatch() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace device contract mismatch", TaskProfile::General),
        &mut backend,
    );
    let line = trace_json_line(
        "trace device contract mismatch",
        TaskProfile::General,
        5,
        &outcome,
    );
    let actual_device = extract_json_string_field(&line, "device").unwrap();
    let wrong_device = if actual_device == "server" {
        "cpu"
    } else {
        "server"
    };
    let mismatched = line.replacen(
        &format!("\"device\":\"{actual_device}\""),
        &format!("\"device\":\"{wrong_device}\""),
        1,
    );

    let failures = evaluate_trace_schema_line(&mismatched);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("runtime_device_contract device=")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_invalid_kv_precision_order() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace kv precision policy", TaskProfile::General),
        &mut backend,
    );
    let line = trace_json_line(
        "trace kv precision policy",
        TaskProfile::General,
        5,
        &outcome,
    );
    let hot = extract_json_usize_field(&line, "hot_kv_bits").unwrap();
    let cold = extract_json_usize_field(&line, "cold_kv_bits").unwrap();
    let invalid = line
        .replacen(
            &format!("\"hot_kv_bits\":{hot},\"cold_kv_bits\":{cold}"),
            "\"hot_kv_bits\":4,\"cold_kv_bits\":8",
            1,
        )
        .replacen(&format!("kv_bits={hot}/{cold}"), "kv_bits=4/8", 1)
        .replacen(
            &format!("hot_kv_bits={hot} cold_kv_bits={cold}"),
            "hot_kv_bits=4 cold_kv_bits=8",
            1,
        );

    let failures = evaluate_trace_schema_line(&invalid);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("cold_kv_bits 8")),
        "{failures:?}"
    );
}
