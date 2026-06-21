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
