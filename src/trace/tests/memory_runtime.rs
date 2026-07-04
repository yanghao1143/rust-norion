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
fn trace_json_line_emits_redacted_runtime_budget_report() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new(
            "trace runtime budget should not leak prompt payload",
            TaskProfile::General,
        ),
        &mut backend,
    );
    let line = trace_json_line(
        "trace runtime budget should not leak prompt payload",
        TaskProfile::General,
        5,
        &outcome,
    );
    let hardware = json_object_after_field(&line, "hardware").unwrap();
    let budget = json_object_after_field(hardware, "runtime_budget").unwrap();
    let failures = evaluate_trace_schema_line(&line);

    assert!(failures.is_empty(), "{failures:?}");
    assert_eq!(
        extract_json_string_field(budget, "requested_device"),
        Some("auto".to_owned())
    );
    assert_eq!(
        extract_json_string_field(budget, "selected_device"),
        Some("cpu".to_owned())
    );
    assert_eq!(
        extract_json_string_field(budget, "selected_adapter"),
        Some("portable-rust".to_owned())
    );
    assert_eq!(
        extract_json_string_field(budget, "quantization_profile"),
        Some("cpu-stub".to_owned())
    );
    assert_eq!(
        extract_json_string_field(budget, "fallback_reason"),
        Some("auto-device-cpu-stub".to_owned())
    );
    assert_eq!(
        extract_json_bool_field(budget, "fail_closed_cpu_stub"),
        Some(true)
    );
    assert_eq!(extract_json_bool_field(budget, "read_only"), Some(true));
    assert_eq!(
        extract_json_bool_field(budget, "write_allowed"),
        Some(false)
    );
    assert_eq!(extract_json_bool_field(budget, "applied"), Some(false));
    assert_eq!(
        extract_json_usize_field(budget, "model_weight_bytes")
            .unwrap()
            .saturating_add(extract_json_usize_field(budget, "kv_cache_bytes").unwrap())
            .saturating_add(extract_json_usize_field(budget, "gene_segment_cache_bytes").unwrap())
            .saturating_add(
                extract_json_usize_field(budget, "routing_reflection_overhead_bytes").unwrap()
            ),
        extract_json_usize_field(budget, "total_required_bytes").unwrap()
    );
    assert!(!budget.contains("prompt payload"));
    assert!(!budget.contains("answer:"));
    assert!(!budget.contains("secret"));
}

#[test]
fn trace_json_line_emits_runtime_kv_activity_diagnostics() {
    struct RuntimeKvActivityBackend;

    impl InferenceBackend for RuntimeKvActivityBackend {
        fn generate(&mut self, context: GenerationContext<'_>) -> InferenceDraft {
            let diagnostics = RuntimeDiagnostics {
                model_id: Some("trace-runtime-kv".to_owned()),
                selected_adapter: Some("portable-rust".to_owned()),
                imported_kv_blocks: 2,
                weak_runtime_kv_imports_skipped: 3,
                budget_limited_runtime_kv_imports_skipped: 4,
                exported_kv_blocks: 1,
                runtime_kv_segments_included: 2,
                runtime_kv_segments_skipped: 1,
                runtime_kv_segments_rejected: 1,
                ..RuntimeDiagnostics::default()
            }
            .with_device_execution(
                context.hardware_plan.device.as_str(),
                context.hardware_plan.execution.primary_lane.as_str(),
                context.hardware_plan.execution.fallback_lane.as_str(),
                context.hardware_plan.execution.memory_mode.as_str(),
            )
            .with_kv_precision(
                context.hardware_plan.execution.hot_kv_precision_bits,
                context.hardware_plan.execution.cold_kv_precision_bits,
            );

            InferenceDraft::new(
                "Runtime KV activity diagnostics are recorded for trace gates.",
                vec![ReasoningStep::new(
                    "runtime",
                    "runtime kv import/export and weak skip diagnostics",
                    0.91,
                )],
            )
            .with_exported_kv_blocks(vec![RuntimeKvBlock::new(
                1,
                0,
                0,
                1,
                vec![0.1, 0.2],
                vec![0.3, 0.4],
            )])
            .with_runtime_diagnostics(diagnostics)
        }
    }

    let mut engine = NoironEngine::new();
    let mut backend = RuntimeKvActivityBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace runtime kv activity", TaskProfile::Coding),
        &mut backend,
    );
    let line = trace_json_line(
        "trace runtime kv activity",
        TaskProfile::Coding,
        5,
        &outcome,
    );
    let runtime = json_object_after_field(&line, "runtime_diagnostics").unwrap();
    let failures = evaluate_trace_schema_line(&line);

    assert!(failures.is_empty(), "{failures:?}");
    assert_eq!(
        extract_json_usize_field(runtime, "imported_kv_blocks"),
        Some(2)
    );
    assert_eq!(
        extract_json_usize_field(runtime, "exported_kv_blocks"),
        Some(1)
    );
    assert_eq!(
        extract_json_usize_field(runtime, "weak_runtime_kv_imports_skipped"),
        Some(3)
    );
    assert_eq!(
        extract_json_usize_field(runtime, "budget_limited_runtime_kv_imports_skipped"),
        Some(4)
    );
    assert_eq!(
        extract_json_usize_field(runtime, "runtime_kv_segments_included"),
        Some(2)
    );
    assert_eq!(
        extract_json_usize_field(runtime, "runtime_kv_segments_skipped"),
        Some(1)
    );
    assert_eq!(
        extract_json_usize_field(runtime, "runtime_kv_segments_rejected"),
        Some(1)
    );
    assert_eq!(
        extract_json_usize_field(runtime, "runtime_kv_segment_count"),
        Some(4)
    );
    assert_eq!(
        extract_json_usize_field(runtime, "runtime_kv_segment_lifecycle_records"),
        Some(4)
    );
    let lifecycle_summaries =
        extract_json_string_array_field(runtime, "runtime_kv_segment_lifecycle_summaries").unwrap();
    assert_eq!(lifecycle_summaries.len(), 3);
    assert!(
        lifecycle_summaries
            .iter()
            .any(|summary| summary.contains("lifecycle=active")
                && summary.contains("shadow_state=ready_for_explicit_apply")
                && summary.contains("drift_state=drift_passed")
                && summary.contains("source_ids=2")
                && summary.contains("expires_after_steps=168")
                && summary.contains("score_milli=1000")
                && summary.contains("drift_gate_domains=golden_fixture:pass|routing_behavior:pass|memory_hygiene:pass|privacy:pass|trace_schema:pass")
                && summary.contains("rollback=redaction-digest:")
                && summary.contains("operator_approval_required=false"))
    );
    assert!(lifecycle_summaries.iter().any(
        |summary| summary.contains("lifecycle=recycle_candidate")
            && summary.contains("shadow_state=benchmark_pending")
            && summary.contains("drift_state=benchmark_pending")
            && summary.contains("source_ids=1")
            && summary.contains("expires_after_steps=72")
            && summary.contains("score_milli=450")
            && summary.contains("drift_gate_domains=golden_fixture:pending|routing_behavior:pending|memory_hygiene:pending|privacy:pending|trace_schema:pending")
            && summary.contains("readmission_gate=hold_until_budget_and_verifier_pass")
            && summary.contains("operator_approval_required=true")
    ));
    assert!(
        lifecycle_summaries
            .iter()
            .any(|summary| summary.contains("lifecycle=rejected_final")
                && summary.contains("shadow_state=quarantined")
                && summary.contains("drift_state=drift_failed")
                && summary.contains("source_ids=1")
                && summary.contains("expires_after_steps=0")
                && summary.contains("score_milli=0")
                && summary.contains("drift_gate_domains=golden_fixture:reject|routing_behavior:reject|memory_hygiene:reject|privacy:reject|trace_schema:reject")
                && summary
                    .contains("reason_code=runtime_kv_segment_rejected_by_safety_or_contract")
                && summary.contains("source_digest=redaction-digest:")
                && summary.contains("parent_lineage=runtime_kv_segment:trace_runtime_diagnostics")
                && summary.contains("rollback_anchor=runtime_kv_segment_preview_only_no_apply")
                && summary.contains("affected_scope=runtime_kv_segment_candidate"))
    );
    assert!(
        (extract_json_nullable_f32_field(runtime, "runtime_kv_segment_yield").unwrap() - 0.25)
            .abs()
            < 0.000_1
    );
    assert_eq!(
        extract_json_bool_field(runtime, "has_runtime_kv_activity_signal"),
        Some(true)
    );
    assert_eq!(
        extract_json_bool_field(runtime, "has_runtime_kv_segment_signal"),
        Some(true)
    );
    assert_eq!(
        extract_json_bool_field(runtime, "has_forward_signal"),
        Some(true)
    );
}

#[test]
fn trace_json_line_drops_untrusted_runtime_selected_adapter() {
    struct UntrustedAdapterBackend;

    impl InferenceBackend for UntrustedAdapterBackend {
        fn generate(&mut self, context: GenerationContext<'_>) -> InferenceDraft {
            let diagnostics = RuntimeDiagnostics {
                model_id: Some("trace-untrusted-adapter".to_owned()),
                selected_adapter: Some("unknown-adapter secret=sk-trace".to_owned()),
                forward_energy: Some(0.20),
                kv_influence: Some(0.41),
                ..RuntimeDiagnostics::default()
            }
            .with_device_execution(
                context.hardware_plan.device.as_str(),
                context.hardware_plan.execution.primary_lane.as_str(),
                context.hardware_plan.execution.fallback_lane.as_str(),
                context.hardware_plan.execution.memory_mode.as_str(),
            )
            .with_kv_precision(
                context.hardware_plan.execution.hot_kv_precision_bits,
                context.hardware_plan.execution.cold_kv_precision_bits,
            );

            InferenceDraft::new(
                "Runtime diagnostics with an unknown adapter should stay audit safe.",
                vec![ReasoningStep::new(
                    "runtime",
                    "unknown adapter label is not trusted trace evidence",
                    0.91,
                )],
            )
            .with_runtime_diagnostics(diagnostics)
        }
    }

    let mut engine = NoironEngine::new();
    let mut backend = UntrustedAdapterBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace unknown runtime adapter", TaskProfile::Coding),
        &mut backend,
    );
    let line = trace_json_line(
        "trace unknown runtime adapter",
        TaskProfile::Coding,
        5,
        &outcome,
    );
    let runtime = json_object_after_field(&line, "runtime_diagnostics").unwrap();
    let failures = evaluate_trace_schema_line(&line);

    assert_eq!(
        outcome.runtime_diagnostics.selected_adapter.as_deref(),
        Some("unknown-adapter secret=sk-trace")
    );
    assert!(failures.is_empty(), "{failures:?}");
    assert!(runtime.contains("\"selected_adapter\":null"));
    for marker in ["unknown-adapter", "secret=", "sk-trace"] {
        assert!(!line.contains(marker), "{line}");
    }
}

#[test]
fn trace_schema_gate_rejects_runtime_kv_activity_signal_mismatch() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace runtime kv activity mismatch", TaskProfile::General),
        &mut backend,
    );
    let line = replace_in_trace_object(
        &trace_json_line(
            "trace runtime kv activity mismatch",
            TaskProfile::General,
            5,
            &outcome,
        ),
        "runtime_diagnostics",
        "\"has_runtime_kv_activity_signal\":false",
        "\"has_runtime_kv_activity_signal\":true",
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("has_runtime_kv_activity_signal=true")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_runtime_kv_segment_count_mismatch() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new(
            "trace runtime kv segment count mismatch",
            TaskProfile::General,
        ),
        &mut backend,
    );
    let line = replace_in_trace_object(
        &trace_json_line(
            "trace runtime kv segment count mismatch",
            TaskProfile::General,
            5,
            &outcome,
        ),
        "runtime_diagnostics",
        "\"runtime_kv_segment_count\":0",
        "\"runtime_kv_segment_count\":1",
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("runtime_kv_segment_count=1")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_runtime_kv_segment_lifecycle_missing_evidence() {
    struct RuntimeKvSegmentLifecycleBackend;

    impl InferenceBackend for RuntimeKvSegmentLifecycleBackend {
        fn generate(&mut self, context: GenerationContext<'_>) -> InferenceDraft {
            let diagnostics = RuntimeDiagnostics {
                model_id: Some("trace-runtime-kv-lifecycle".to_owned()),
                selected_adapter: Some("portable-rust".to_owned()),
                runtime_kv_segments_included: 1,
                runtime_kv_segments_skipped: 1,
                runtime_kv_segments_rejected: 1,
                ..RuntimeDiagnostics::default()
            }
            .with_device_execution(
                context.hardware_plan.device.as_str(),
                context.hardware_plan.execution.primary_lane.as_str(),
                context.hardware_plan.execution.fallback_lane.as_str(),
                context.hardware_plan.execution.memory_mode.as_str(),
            )
            .with_kv_precision(
                context.hardware_plan.execution.hot_kv_precision_bits,
                context.hardware_plan.execution.cold_kv_precision_bits,
            );

            InferenceDraft::new(
                "Runtime KV segment lifecycle evidence is recorded.",
                vec![ReasoningStep::new(
                    "runtime_kv_segment_lifecycle",
                    "segment lifecycle evidence",
                    0.8,
                )],
            )
            .with_runtime_diagnostics(diagnostics)
        }
    }

    let mut engine = NoironEngine::new();
    let mut backend = RuntimeKvSegmentLifecycleBackend;
    let outcome = engine.infer(
        InferenceRequest::new(
            "trace runtime kv segment lifecycle evidence",
            TaskProfile::General,
        ),
        &mut backend,
    );
    let line = replace_in_trace_object(
        &trace_json_line(
            "trace runtime kv segment lifecycle evidence",
            TaskProfile::General,
            5,
            &outcome,
        ),
        "runtime_diagnostics",
        "reason_code=",
        "reason_missing=",
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures.iter().any(
            |failure| failure.contains("runtime_kv_segment_lifecycle_summaries")
                && failure.contains("reason_code=")
        ),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_runtime_kv_segment_lifecycle_missing_shadow_evidence() {
    struct RuntimeKvSegmentLifecycleBackend;

    impl InferenceBackend for RuntimeKvSegmentLifecycleBackend {
        fn generate(&mut self, context: GenerationContext<'_>) -> InferenceDraft {
            let diagnostics = RuntimeDiagnostics {
                model_id: Some("trace-runtime-kv-lifecycle-shadow".to_owned()),
                selected_adapter: Some("portable-rust".to_owned()),
                runtime_kv_segments_included: 1,
                ..RuntimeDiagnostics::default()
            }
            .with_device_execution(
                context.hardware_plan.device.as_str(),
                context.hardware_plan.execution.primary_lane.as_str(),
                context.hardware_plan.execution.fallback_lane.as_str(),
                context.hardware_plan.execution.memory_mode.as_str(),
            )
            .with_kv_precision(
                context.hardware_plan.execution.hot_kv_precision_bits,
                context.hardware_plan.execution.cold_kv_precision_bits,
            );

            InferenceDraft::new(
                "Runtime KV segment lifecycle shadow evidence is recorded.",
                vec![ReasoningStep::new(
                    "runtime_kv_segment_lifecycle_shadow",
                    "segment lifecycle shadow evidence",
                    0.8,
                )],
            )
            .with_runtime_diagnostics(diagnostics)
        }
    }

    let mut engine = NoironEngine::new();
    let mut backend = RuntimeKvSegmentLifecycleBackend;
    let outcome = engine.infer(
        InferenceRequest::new(
            "trace runtime kv segment lifecycle shadow evidence",
            TaskProfile::General,
        ),
        &mut backend,
    );
    let line = replace_in_trace_object(
        &trace_json_line(
            "trace runtime kv segment lifecycle shadow evidence",
            TaskProfile::General,
            5,
            &outcome,
        ),
        "runtime_diagnostics",
        "shadow_state=",
        "shadow_missing=",
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures.iter().any(
            |failure| failure.contains("runtime_kv_segment_lifecycle_summaries")
                && failure.contains("shadow_state=")
        ),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_runtime_budget_write_enabled() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace runtime budget write gate", TaskProfile::General),
        &mut backend,
    );
    let line = replace_in_trace_object(
        &trace_json_line(
            "trace runtime budget write gate",
            TaskProfile::General,
            5,
            &outcome,
        ),
        "runtime_budget",
        "\"write_allowed\":false",
        "\"write_allowed\":true",
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("runtime_budget write_allowed")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_runtime_budget_total_mismatch() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace runtime budget total mismatch", TaskProfile::General),
        &mut backend,
    );
    let line = replace_trace_object_usize(
        &trace_json_line(
            "trace runtime budget total mismatch",
            TaskProfile::General,
            5,
            &outcome,
        ),
        "runtime_budget",
        "total_required_bytes",
        1,
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("runtime_budget total_required_bytes")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_runtime_budget_cpu_stub_device_drift() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new(
            "trace runtime budget selected device drift",
            TaskProfile::General,
        ),
        &mut backend,
    );
    let line = replace_in_trace_object(
        &trace_json_line(
            "trace runtime budget selected device drift",
            TaskProfile::General,
            5,
            &outcome,
        ),
        "runtime_budget",
        "\"selected_device\":\"cpu\"",
        "\"selected_device\":\"server\"",
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures.iter().any(|failure| failure
            .contains("runtime_budget selected_device=server must be cpu for CPU stub fallback")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_memory_residency_write_enabled() {
    let policy = crate::kv_cache::MemoryResidencyPolicy {
        tenant_id: "tenant-a".to_owned(),
        ..crate::kv_cache::MemoryResidencyPolicy::default()
    };
    let candidates = vec![
        crate::kv_cache::MemoryResidencyCandidate::new(201, "tenant-a", "semantic")
            .with_scores(0.84, 4, 0, 9)
            .with_high_frequency_gene(true),
    ];
    let plan = crate::kv_cache::plan_memory_residency(&candidates, &policy, 10);
    let line = memory_residency_trace_json_line(&plan)
        .replace("\"write_allowed\":false", "\"write_allowed\":true");

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("memory_residency write_allowed must be false")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_compute_budget_write_enabled() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace compute budget write mismatch", TaskProfile::Coding)
            .with_max_tokens(Some(64)),
        &mut backend,
    );
    let line = replace_in_trace_object(
        &trace_json_line(
            "trace compute budget write mismatch",
            TaskProfile::Coding,
            5,
            &outcome,
        ),
        "compute_budget",
        "\"write_allowed\":false",
        "\"write_allowed\":true",
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("compute_budget write_allowed must be false")),
        "{failures:?}"
    );
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
fn trace_json_line_emits_memory_admission_preview() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new(
            "trace memory admission preview by building a Rust runtime adapter tool",
            TaskProfile::Coding,
        ),
        &mut backend,
    );
    let line = trace_json_line(
        "trace memory admission preview by building a Rust runtime adapter tool",
        TaskProfile::Coding,
        5,
        &outcome,
    );
    let admission = json_object_after_field(&line, "memory_admission").unwrap();
    let fusion = json_object_after_field(&line, "kv_fusion").unwrap();
    let failures = evaluate_trace_schema_line(&line);

    assert!(failures.is_empty(), "{failures:?}");
    assert!(extract_json_usize_field(admission, "candidates").unwrap_or(0) >= 1);
    assert_eq!(
        extract_json_usize_field(admission, "review_packets"),
        extract_json_usize_field(admission, "candidates")
    );
    assert_eq!(
        extract_json_usize_field(admission, "ledger_records"),
        extract_json_usize_field(admission, "candidates")
    );
    assert_eq!(
        extract_json_usize_field(admission, "ledger_authorized"),
        Some(0)
    );
    assert_eq!(
        extract_json_usize_field(admission, "ledger_applied"),
        Some(0)
    );
    assert!(
        extract_json_string_array_field(admission, "kinds")
            .unwrap()
            .iter()
            .any(|kind| kind == "tool_reliability_observation")
    );
    assert_eq!(extract_json_usize_field(admission, "admitted"), Some(0));
    assert_eq!(extract_json_bool_field(admission, "read_only"), Some(true));
    assert_eq!(
        extract_json_bool_field(admission, "write_allowed"),
        Some(false)
    );
    assert_eq!(extract_json_bool_field(admission, "applied"), Some(false));
    assert!(
        !extract_json_string_array_field(admission, "candidate_summaries")
            .unwrap()
            .iter()
            .any(|summary| summary.contains("prompt:") || summary.contains("answer:"))
    );
    assert!(
        extract_json_string_array_field(admission, "candidate_summaries")
            .unwrap()
            .iter()
            .all(|summary| {
                summary.contains("profile=")
                    && summary.contains("shadow_state=")
                    && summary.contains("drift_state=")
                    && summary.contains("source_ids=")
                    && summary.contains("expires_after_steps=")
                    && summary.contains("score_milli=")
                    && summary.contains("drift_gate_domains=")
                    && summary.contains("golden_fixture:")
                    && summary.contains("routing_behavior:")
                    && summary.contains("memory_hygiene:")
                    && summary.contains("privacy:")
                    && summary.contains("trace_schema:")
                    && summary.contains("rollback=")
                    && summary.contains("source_hash=")
                    && summary.contains("privacy=")
                    && summary.contains("validation=")
            })
    );
    assert!(
        extract_json_string_array_field(admission, "review_packet_summaries")
            .unwrap()
            .iter()
            .all(|summary| {
                summary.contains("approval=")
                    && summary.contains("next=")
                    && summary.contains("rollback=")
                    && summary.contains("write_allowed=false")
            })
    );
    assert!(
        extract_json_string_array_field(admission, "ledger_summaries")
            .unwrap()
            .iter()
            .all(|summary| {
                summary.contains("rollback=")
                    && summary.contains("source_hash=")
                    && summary.contains("privacy=")
                    && summary.contains("validation=")
                    && summary.contains("authorized=false")
                    && summary.contains("applied=false")
            })
    );
    assert!(extract_json_usize_field(fusion, "candidates").unwrap_or(0) >= 1);
    assert_eq!(extract_json_bool_field(fusion, "read_only"), Some(true));
    assert_eq!(
        extract_json_bool_field(fusion, "write_allowed"),
        Some(false)
    );
    assert_eq!(extract_json_bool_field(fusion, "applied"), Some(false));
    assert_eq!(
        extract_json_usize_field(fusion, "retained_tokens")
            .unwrap()
            .saturating_add(extract_json_usize_field(fusion, "saved_tokens").unwrap()),
        extract_json_usize_field(fusion, "input_tokens").unwrap()
    );
    assert!(
        extract_json_string_array_field(fusion, "score_summaries")
            .unwrap()
            .iter()
            .all(|summary| {
                summary.contains("source=")
                    && summary.contains("decision=")
                    && summary.contains("score=")
                    && summary.contains("components=")
                    && summary.contains("rollback=")
            })
    );
    assert!(
        !extract_json_string_array_field(fusion, "score_summaries")
            .unwrap()
            .iter()
            .any(|summary| summary.contains("prompt:") || summary.contains("answer:"))
    );
}

#[test]
fn trace_schema_gate_rejects_memory_admission_gene_segment_metadata_gap() {
    let line = gene_segment_metadata_trace_line();
    let failures = evaluate_trace_memory_admission(&line);

    assert!(failures.is_empty(), "{failures:?}");

    let line = line.replace(" tenant_scope_digest=redaction-digest:tenant", "");
    let failures = evaluate_trace_memory_admission(&line);

    assert!(
        failures.iter().any(|failure| {
            failure.contains(
                "memory_admission GeneSegment ledger metadata 0 does not match source_gene_segment 1",
            )
        }),
        "{failures:?}"
    );
}

fn gene_segment_metadata_trace_line() -> String {
    r#"{"memory_admission":{"candidates":1,"ready":1,"blocked":0,"admitted":0,"hold":0,"reject":0,"quarantine":0,"kinds":["gene_segment_kv_evidence"],"source_semantic":0,"source_gist":0,"source_runtime_kv":0,"source_cold":0,"source_gene_segment":1,"decisions":["ready"],"candidate_summaries":["ready:gene_segment_kv_evidence:memory_admission:coding:gene-segment-kv:record-a profile=Coding shadow_state=ready_for_explicit_apply drift_state=drift_passed source_ids=3 expires_after_steps=168 score_milli=900 drift_gate_domains=golden_fixture:pass|routing_behavior:pass|memory_hygiene:pass|privacy:pass|trace_schema:pass rollback=rollback:gene q=0.900 reward=0.900 runtime_kv_influence=none runtime_kv_segment_yield=none runtime_kv_budget_pressure=none runtime_kv_weak_import_pressure=none critical=0 revisions=0 source_hash=sha256:gene privacy=digest_only validation=5 privacy_checked=true durable_write_authorized=false applied=false"],"review_packets":1,"review_packet_summaries":["review:memory_admission:coding:gene-segment-kv:record-a approval=pending_approval next=apply risks=none evidence=1 validation=5 source_hash=sha256:gene privacy=digest_only rollback=rollback:gene read_only=true write_allowed=false applied=false"],"trace_segment_priors":0,"trace_segment_prior_summaries":[],"ledger_records":1,"ledger_authorized":0,"ledger_applied":0,"ledger_preview_only":1,"ledger_held":0,"ledger_rejected":0,"ledger_duplicate":0,"ledger_decayed":0,"ledger_merged":0,"ledger_rollback":0,"ledger_summaries":["preview_only:gene_segment_kv_evidence:memory_admission:coding:gene-segment-kv:record-a lifecycle=suspect approval=pending_approval authorized=false applied=false rollback=rollback:gene source_hash=sha256:gene profile=coding source=runtime_gene_segment tenant_scope_digest=redaction-digest:tenant session_scope_digest=redaction-digest:session privacy=digest_only validation=5 reasons=none gene_segment_kv=true"],"read_only":true,"write_allowed":false,"applied":false}}"#.to_owned()
}

#[test]
fn trace_schema_gate_rejects_memory_admission_candidate_missing_drift_domain() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace memory admission drift domain", TaskProfile::Coding),
        &mut backend,
    );
    let line = trace_json_line(
        "trace memory admission drift domain",
        TaskProfile::Coding,
        5,
        &outcome,
    )
    .replace("golden_fixture:", "golden:");

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures.iter().any(|failure| failure
            .contains("memory_admission candidate 0 missing golden_fixture:")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_memory_admission_candidate_missing_shadow_evidence() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new(
            "trace memory admission shadow evidence",
            TaskProfile::Coding,
        ),
        &mut backend,
    );
    let line = trace_json_line(
        "trace memory admission shadow evidence",
        TaskProfile::Coding,
        5,
        &outcome,
    )
    .replace(" profile=Coding", "");

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("memory_admission candidate 0 missing profile=")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_memory_admission_count_mismatch() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace memory admission count mismatch", TaskProfile::Coding),
        &mut backend,
    );
    let line = trace_json_line(
        "trace memory admission count mismatch",
        TaskProfile::Coding,
        5,
        &outcome,
    );
    let line = increment_trace_object_usize(&line, "memory_admission", "candidates");

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("memory_admission decisions")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_kv_fusion_count_mismatch() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace kv fusion count mismatch", TaskProfile::Coding),
        &mut backend,
    );
    let line = trace_json_line(
        "trace kv fusion count mismatch",
        TaskProfile::Coding,
        5,
        &outcome,
    );
    let line = increment_trace_object_usize(&line, "kv_fusion", "candidates");

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("kv_fusion decisions")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_kv_fusion_write_enabled() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace kv fusion write gate", TaskProfile::Coding),
        &mut backend,
    );
    let line = replace_in_trace_object(
        &trace_json_line(
            "trace kv fusion write gate",
            TaskProfile::Coding,
            5,
            &outcome,
        ),
        "kv_fusion",
        "\"write_allowed\":false",
        "\"write_allowed\":true",
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("kv_fusion write_allowed")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_memory_admission_review_packet_mismatch() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new(
            "trace memory admission packet mismatch",
            TaskProfile::Coding,
        ),
        &mut backend,
    );
    let line = trace_json_line(
        "trace memory admission packet mismatch",
        TaskProfile::Coding,
        5,
        &outcome,
    );
    let line = increment_trace_object_usize(&line, "memory_admission", "review_packets");

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("memory_admission review_packet_summaries")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_memory_admission_write_enabled() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace memory admission write gate", TaskProfile::Coding),
        &mut backend,
    );
    let line = replace_in_trace_object(
        &trace_json_line(
            "trace memory admission write gate",
            TaskProfile::Coding,
            5,
            &outcome,
        ),
        "memory_admission",
        "\"write_allowed\":false",
        "\"write_allowed\":true",
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("memory_admission write_allowed")),
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
fn trace_json_line_emits_memory_compaction_pair_evidence() {
    let policy = crate::kv_cache::MemoryCompactionPolicy {
        similarity_threshold: 0.90,
        max_candidates: 8,
        max_merges: 2,
    };
    let mut compaction_cache = crate::kv_cache::KvFusionCache::with_limits(0.99, 4096);
    let weaker = compaction_cache.store_or_fuse(
        "trace_compaction_pair:old duplicate",
        vec![1.0, 0.0, 0.0],
        0.35,
    );
    let stronger = compaction_cache.store_or_fuse(
        "trace_compaction_pair:strong duplicate",
        vec![0.93, 0.37, 0.0],
        0.90,
    );
    let report = compaction_cache.compact_similar(policy.clone());
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let mut outcome = engine.infer(
        InferenceRequest::new("trace compaction pair evidence", TaskProfile::General),
        &mut backend,
    );
    outcome.memory_compaction_policy = policy;
    outcome.memory_compaction_report = report;

    assert_eq!(outcome.memory_compaction_report.merged.len(), 1);
    assert_eq!(
        outcome.memory_compaction_report.merged[0].primary_id,
        stronger
    );
    assert_eq!(
        outcome.memory_compaction_report.merged[0].removed_id,
        weaker
    );
    let line = trace_json_line(
        "trace compaction pair evidence",
        TaskProfile::General,
        5,
        &outcome,
    );
    let compaction = json_object_after_field(&line, "memory_compaction").unwrap();
    let pairs = json_array_after_field(compaction, "pairs")
        .and_then(json_object_array_items)
        .unwrap();
    let failures = evaluate_trace_schema_line(&line);

    assert!(failures.is_empty(), "{failures:?}");
    assert_eq!(extract_json_usize_field(compaction, "merged"), Some(1));
    assert_eq!(pairs.len(), 1);
    assert_eq!(
        extract_json_usize_field(pairs[0], "primary_id"),
        Some(stronger as usize)
    );
    assert_eq!(
        extract_json_usize_field(pairs[0], "removed_id"),
        Some(weaker as usize)
    );
    assert_eq!(
        extract_json_string_field(pairs[0], "namespace"),
        Some("semantic".to_owned())
    );
    assert_eq!(
        extract_json_usize_field(pairs[0], "primary_vector_dimensions"),
        Some(3)
    );
    assert_eq!(
        extract_json_usize_field(pairs[0], "removed_vector_dimensions"),
        Some(3)
    );
    assert_eq!(
        extract_json_bool_field(pairs[0], "primary_protected"),
        Some(false)
    );
    assert_eq!(
        extract_json_bool_field(pairs[0], "removed_protected"),
        Some(false)
    );
    assert!(!line.contains("old duplicate"));
    assert!(!line.contains("[1.0,0.0,0.0]"));
}

#[test]
fn trace_schema_gate_rejects_unsafe_compaction_pair_evidence() {
    let policy = crate::kv_cache::MemoryCompactionPolicy {
        similarity_threshold: 0.90,
        max_candidates: 8,
        max_merges: 2,
    };
    let mut compaction_cache = crate::kv_cache::KvFusionCache::with_limits(0.99, 4096);
    compaction_cache.store_or_fuse(
        "trace_compaction_pair:old duplicate",
        vec![1.0, 0.0, 0.0],
        0.35,
    );
    compaction_cache.store_or_fuse(
        "trace_compaction_pair:strong duplicate",
        vec![0.93, 0.37, 0.0],
        0.90,
    );
    let report = compaction_cache.compact_similar(policy.clone());
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let mut outcome = engine.infer(
        InferenceRequest::new("trace bad compaction pair evidence", TaskProfile::General),
        &mut backend,
    );
    outcome.memory_compaction_policy = policy;
    outcome.memory_compaction_report = report;
    let line = replace_in_trace_object(
        &trace_json_line(
            "trace bad compaction pair evidence",
            TaskProfile::General,
            5,
            &outcome,
        ),
        "memory_compaction",
        "\"removed_protected\":false",
        "\"removed_protected\":true",
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("must not remove a protected memory")),
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
