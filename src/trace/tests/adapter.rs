use super::*;

#[test]
fn trace_schema_gate_rejects_selected_adapter_outside_device_contract() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace adapter contract mismatch", TaskProfile::General),
        &mut backend,
    );
    let line = trace_json_line(
        "trace adapter contract mismatch",
        TaskProfile::General,
        5,
        &outcome,
    );
    let mismatched = line.replacen(
        "\"selected_adapter\":null",
        "\"selected_adapter\":\"cuda\"",
        1,
    );

    let failures = evaluate_trace_schema_line(&mismatched);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("selected_adapter cuda")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_accepts_valid_runtime_adapter_observation() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace adapter observation", TaskProfile::Coding),
        &mut backend,
    );
    let line = trace_json_line(
        "trace adapter observation",
        TaskProfile::Coding,
        5,
        &outcome,
    )
    .replacen("\"observation_count\":0", "\"observation_count\":1", 1)
    .replacen(
        "\"best_adapter\":null",
        "\"best_adapter\":\"portable-rust\"",
        1,
    )
    .replacen("\"best_score\":null", "\"best_score\":0.510000", 1)
    .replacen("\"best_reward\":null", "\"best_reward\":0.500000", 1)
    .replacen("\"best_quality\":null", "\"best_quality\":0.800000", 1)
    .replacen("\"best_experience_id\":null", "\"best_experience_id\":7", 1);

    let failures = evaluate_trace_schema_line(&line);

    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn trace_schema_gate_accepts_runtime_adapter_selection_mismatch_evidence() {
    let mut engine = NoironEngine::new();
    let mut backend = RuntimePrecisionBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace adapter selection evidence", TaskProfile::Coding),
        &mut backend,
    );
    let line = trace_json_line(
        "trace adapter selection evidence",
        TaskProfile::Coding,
        5,
        &outcome,
    )
    .replacen("\"observation_count\":0", "\"observation_count\":1", 1)
    .replacen("\"best_adapter\":null", "\"best_adapter\":\"cpu-simd\"", 1)
    .replacen(
        "\"selection_mismatch\":false",
        "\"selection_mismatch\":true",
        1,
    )
    .replacen("\"best_score\":null", "\"best_score\":0.510000", 1)
    .replacen("\"best_reward\":null", "\"best_reward\":0.500000", 1)
    .replacen("\"best_quality\":null", "\"best_quality\":0.800000", 1)
    .replacen("\"best_experience_id\":null", "\"best_experience_id\":7", 1);

    let failures = evaluate_trace_schema_line(&line);

    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn trace_line_records_adapter_stream_write_gate_state() {
    let mut engine = NoironEngine::new();
    let mut backend = RuntimePrecisionBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace adapter stream write gate", TaskProfile::Coding),
        &mut backend,
    );

    let line = trace_json_line(
        "trace adapter stream write gate",
        TaskProfile::Coding,
        5,
        &outcome,
    );
    let failures = evaluate_trace_schema_line(&line);

    assert!(line.contains("\"adapter_stream_read_only\":true"));
    assert!(line.contains("\"adapter_stream_write_allowed\":false"));
    assert!(line.contains("\"adapter_stream_applied\":false"));
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn trace_schema_gate_rejects_unsafe_adapter_stream_write_gate_state() {
    let mut engine = NoironEngine::new();
    let mut backend = RuntimePrecisionBackend;
    let outcome = engine.infer(
        InferenceRequest::new(
            "trace unsafe adapter stream write gate",
            TaskProfile::Coding,
        ),
        &mut backend,
    );
    let line = trace_json_line(
        "trace unsafe adapter stream write gate",
        TaskProfile::Coding,
        5,
        &outcome,
    );

    for (from, to, expected) in [
        (
            "\"adapter_stream_gate_summary_digest\":\"fnv64:0123456789abcdef\"",
            "\"adapter_stream_gate_summary_digest\":\"bad-digest\"",
            "adapter_stream_gate_summary_digest must be fnv64 digest",
        ),
        (
            "\"adapter_stream_read_only\":true",
            "\"adapter_stream_read_only\":false",
            "adapter_stream_read_only must be true",
        ),
        (
            "\"adapter_stream_read_only\":true",
            "\"adapter_stream_read_only\":null",
            "adapter_stream_read_only must be true",
        ),
        (
            "\"adapter_stream_write_allowed\":false",
            "\"adapter_stream_write_allowed\":true",
            "adapter_stream_write_allowed must be false",
        ),
        (
            "\"adapter_stream_applied\":false",
            "\"adapter_stream_applied\":true",
            "adapter_stream_applied must be false",
        ),
    ] {
        let failures = evaluate_trace_schema_line(&line.replacen(from, to, 1));

        assert!(
            failures.iter().any(|failure| failure.contains(expected)),
            "{failures:?}"
        );
    }
}

#[test]
fn trace_schema_gate_rejects_control_plane_filled_runtime_device_execution() {
    let mut engine = NoironEngine::new();
    let mut backend = RuntimePrecisionBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace control plane device source", TaskProfile::Coding),
        &mut backend,
    );
    let line = trace_json_line(
        "trace control plane device source",
        TaskProfile::Coding,
        5,
        &outcome,
    )
    .replacen(
        "\"device_execution_source\":\"runtime-reported\"",
        "\"device_execution_source\":\"control-plane-filled\"",
        1,
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("is not runtime-reported")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_accepts_control_plane_filled_static_runtime_architecture() {
    struct StaticArchitectureBackend;

    impl InferenceBackend for StaticArchitectureBackend {
        fn generate(&mut self, context: GenerationContext<'_>) -> InferenceDraft {
            InferenceDraft::new(
                "Static Gemma architecture diagnostics can be audited without device execution.",
                vec![ReasoningStep::new(
                    "runtime_architecture",
                    "static runtime architecture and KV precision recorded",
                    0.82,
                )],
            )
            .with_runtime_diagnostics(RuntimeDiagnostics {
                model_id: Some("gemma-static".to_owned()),
                device_profile: Some(context.hardware_plan.device.as_str().to_owned()),
                primary_lane: Some(
                    context
                        .hardware_plan
                        .execution
                        .primary_lane
                        .as_str()
                        .to_owned(),
                ),
                fallback_lane: Some(
                    context
                        .hardware_plan
                        .execution
                        .fallback_lane
                        .as_str()
                        .to_owned(),
                ),
                memory_mode: Some(
                    context
                        .hardware_plan
                        .execution
                        .memory_mode
                        .as_str()
                        .to_owned(),
                ),
                device_execution_source: Some(
                    RuntimeDiagnostics::control_plane_filled_device_execution_source().to_owned(),
                ),
                hot_kv_precision_bits: Some(context.hardware_plan.execution.hot_kv_precision_bits),
                cold_kv_precision_bits: Some(
                    context.hardware_plan.execution.cold_kv_precision_bits,
                ),
                layer_count: 48,
                hidden_size: 3840,
                local_window_tokens: 1024,
                ..RuntimeDiagnostics::default()
            })
        }
    }

    let mut engine = NoironEngine::new();
    let mut backend = StaticArchitectureBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace static Gemma architecture", TaskProfile::Coding),
        &mut backend,
    );
    let line = trace_json_line(
        "trace static Gemma architecture",
        TaskProfile::Coding,
        5,
        &outcome,
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(line.contains("\"has_runtime_architecture_signal\":true"));
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn trace_schema_gate_rejects_runtime_architecture_signal_mismatch() {
    let mut engine = NoironEngine::new();
    let mut backend = RuntimePrecisionBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace architecture signal mismatch", TaskProfile::Coding),
        &mut backend,
    );
    let line = trace_json_line(
        "trace architecture signal mismatch",
        TaskProfile::Coding,
        5,
        &outcome,
    )
    .replacen(
        "\"has_runtime_architecture_signal\":false",
        "\"has_runtime_architecture_signal\":true",
        1,
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("has_runtime_architecture_signal=true")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_runtime_adapter_selection_mismatch_flag_mismatch() {
    let mut engine = NoironEngine::new();
    let mut backend = RuntimePrecisionBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace adapter selection flag mismatch", TaskProfile::Coding),
        &mut backend,
    );
    let line = trace_json_line(
        "trace adapter selection flag mismatch",
        TaskProfile::Coding,
        5,
        &outcome,
    )
    .replacen("\"observation_count\":0", "\"observation_count\":1", 1)
    .replacen(
        "\"best_adapter\":null",
        "\"best_adapter\":\"portable-rust\"",
        1,
    )
    .replacen(
        "\"selection_mismatch\":false",
        "\"selection_mismatch\":true",
        1,
    )
    .replacen("\"best_score\":null", "\"best_score\":0.510000", 1)
    .replacen("\"best_reward\":null", "\"best_reward\":0.500000", 1)
    .replacen("\"best_quality\":null", "\"best_quality\":0.800000", 1)
    .replacen("\"best_experience_id\":null", "\"best_experience_id\":7", 1);

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("selection_mismatch true")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_runtime_adapter_observation_contract_mismatch() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace adapter observation mismatch", TaskProfile::General),
        &mut backend,
    );
    let line = trace_json_line(
        "trace adapter observation mismatch",
        TaskProfile::General,
        5,
        &outcome,
    )
    .replacen("\"observation_count\":0", "\"observation_count\":1", 1)
    .replacen(
        "\"best_adapter\":null",
        "\"best_adapter\":\"not-a-device-adapter\"",
        1,
    )
    .replacen("\"best_score\":null", "\"best_score\":0.510000", 1)
    .replacen("\"best_reward\":null", "\"best_reward\":0.500000", 1)
    .replacen("\"best_quality\":null", "\"best_quality\":0.800000", 1)
    .replacen("\"best_experience_id\":null", "\"best_experience_id\":7", 1);

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("best_adapter not-a-device-adapter")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_incomplete_runtime_adapter_observation() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace adapter observation incomplete", TaskProfile::General),
        &mut backend,
    );
    let line = trace_json_line(
        "trace adapter observation incomplete",
        TaskProfile::General,
        5,
        &outcome,
    )
    .replacen("\"observation_count\":0", "\"observation_count\":1", 1)
    .replacen(
        "\"best_adapter\":null",
        "\"best_adapter\":\"portable-rust\"",
        1,
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("best_score is missing")),
        "{failures:?}"
    );
}
