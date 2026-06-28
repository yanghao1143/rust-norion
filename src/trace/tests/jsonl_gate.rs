use super::*;
use crate::kv_cache::{plan_memory_residency, MemoryResidencyCandidate, MemoryResidencyPolicy};
use crate::memory_admission::MemoryAdmissionKind;
use crate::reasoning_genome::{
    DnaEvolutionController, DnaEvolutionValidationEvidence, GeneScissorsIntent,
    GeneScissorsOperatorDecision, GeneScissorsTransactionJournal, MutationPlan,
};
use crate::self_evolving_memory::{
    MemoryConsolidationEvidenceClass, MemoryConsolidationRecord, SelfEvolvingEpisodeInput,
    SelfEvolvingHeuristicInput, SelfEvolvingMemoryAdmissionCandidatePreview,
    SelfEvolvingMemoryAdmissionPreview, SelfEvolvingMemoryApproval,
    SelfEvolvingMemoryConsolidationPolicy, SelfEvolvingMemoryConsolidationWorker,
    SelfEvolvingMemoryMaintenancePolicy, SelfEvolvingMemoryQuery,
    SelfEvolvingMemoryRetrievalReport, SelfEvolvingMemoryRuntimeWritebackReport,
    SelfEvolvingMemorySourceQuarantineReport, SelfEvolvingMemoryStore,
};
use crate::{
    default_self_goal_admission_report, default_self_goal_proposal_report,
    default_self_goal_queue_apply_report, default_self_goal_queue_preview_report,
    EvolutionGoalEvidence, EvolutionGoalEvidenceKind, EvolutionGoalQueue,
    EvolutionGoalQueueDiskStore, EvolutionGoalQueueStoreApproval, EvolutionGoalQueueStorePolicy,
    EvolutionGoalRunEvidence, SelfGoalProposalReport, SelfGoalQueueAppendApproval,
    SelfGoalQueueAppendExecutionReport, SelfGoalQueueAppendExecutor, SelfGoalQueueApplyReport,
    TenantResourceLane, TenantScope, UnifiedWriterGate, UnifiedWriterGateCandidate,
    UnifiedWriterGateDomain, UnifiedWriterGatePolicy, UnifiedWriterGateWriteScope,
};

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
    assert_eq!(report.noiron_orchestration_events, 1);
    assert!(report.noiron_orchestration_stages > 0);
    assert_eq!(report.noiron_orchestration_failed_stages, 0);
    assert_eq!(report.noiron_orchestration_writes_gated, 1);
    assert!(report.noiron_orchestration_fht_dke_total_tokens > 0);
    assert_eq!(report.orchestration_audit_events, 1);
    assert!(report.orchestration_audit_checked_fields > 0);
    assert_eq!(report.orchestration_audit_integrity_failed_fields, 0);
    assert!(report.summary_line().contains("passed=true"));
    assert!(report.summary_line().contains("rust_check_events=0"));
    assert!(report.summary_line().contains("runtime_error_events=0"));
    assert!(report.summary_line().contains("runtime_timeout_events=0"));
    assert!(report
        .summary_line()
        .contains("noiron_orchestration_events=1"));
    assert!(report
        .summary_line()
        .contains("noiron_orchestration_writes_gated=1"));
    cleanup(path);
}

#[test]
fn trace_schema_jsonl_gate_aggregates_process_reward_actions() {
    let path = temp_path("trace-schema-process-reward-actions");
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace process reward action counts", TaskProfile::Coding),
        &mut backend,
    );
    let line = trace_json_line(
        "trace process reward action counts",
        TaskProfile::Coding,
        8,
        &outcome,
    );
    let original = format!("\"action\":\"{}\"", outcome.process_reward.action.as_str());
    let reinforce = line.replace(&original, "\"action\":\"reinforce\"");
    let hold = line.replace(&original, "\"action\":\"hold\"");
    let penalize = line.replace(&original, "\"action\":\"penalize\"");
    fs::write(&path, format!("{reinforce}\n{hold}\n{penalize}\n")).unwrap();

    let report = evaluate_trace_schema_jsonl(&path).unwrap();
    let snapshot = report.operator_health_snapshot();
    let learning = snapshot.section("learning").unwrap();

    assert!(report.passed, "{:?}", report.failures);
    assert_eq!(report.checked_lines, 3);
    assert_eq!(report.process_reward_events, 3);
    assert_eq!(report.process_reward_positive, 3);
    assert_eq!(report.process_reward_reinforce, 1);
    assert_eq!(report.process_reward_hold, 1);
    assert_eq!(report.process_reward_penalize, 1);
    assert!(report.summary_line().contains("process_reward_reinforce=1"));
    assert!(report.summary_line().contains("process_reward_hold=1"));
    assert!(report.summary_line().contains("process_reward_penalize=1"));
    assert_eq!(learning.metric("process_reward_reinforce"), Some(1));
    assert_eq!(learning.metric("process_reward_hold"), Some(1));
    assert_eq!(learning.metric("process_reward_penalize"), Some(1));
    cleanup(path);
}

#[test]
fn trace_schema_jsonl_gate_accepts_dna_evolution_controller_trace() {
    let plans = vec![
        MutationPlan::preview(
            "trace-dna-repair",
            GeneScissorsIntent::Repair,
            "gene:trace-repair",
            "repair validated genome candidate",
            "improve fitness without durable mutation",
            "rollback:trace-dna-repair",
        ),
        MutationPlan::preview(
            "trace-dna-regenerate",
            GeneScissorsIntent::Regenerate,
            "gene:trace-regenerate",
            "regenerate stale stable-anchor candidate",
            "replace drifted candidate through preview gates",
            "rollback:trace-dna-regenerate",
        ),
    ];
    let journal = GeneScissorsTransactionJournal::from_mutation_plans(
        TaskProfile::Coding,
        "stable-anchor:trace-dna",
        &plans,
    );
    let controller = DnaEvolutionController::default().preview_plans(
        TaskProfile::Coding,
        "parent-anchor:trace-dna",
        "stable-anchor:trace-dna",
        &plans,
        &DnaEvolutionValidationEvidence::passing(),
        GeneScissorsOperatorDecision::Approved,
        Some(&journal),
    );
    let line = controller.redacted_trace_line();
    let path = temp_path("trace-schema-dna-evolution-controller");
    fs::write(&path, format!("{line}\n")).unwrap();

    let report = evaluate_trace_schema_jsonl(&path).unwrap();

    assert!(report.passed, "{:?}", report.failures);
    assert_eq!(report.checked_lines, 1);
    assert!(line.contains("\"schema\":\"dna_evolution_controller_v1\""));
    assert!(line.contains("\"generation_id\":"));
    assert!(line.contains("\"parent_anchors\":"));
    assert!(line.contains("\"fitness_delta_summary\":"));
    assert!(line.contains("\"validation_status\":\"passed\""));
    assert!(line.contains("\"approval_status\":\"approved\""));
    assert!(line.contains("\"raw_payload_included\":false"));
    assert!(!line.contains("gene:trace-repair"));
    assert!(!line.contains("gene:trace-regenerate"));
    cleanup(path);
}

#[test]
fn trace_schema_jsonl_gate_aggregates_coding_service_eval_runner_feed() {
    let path = temp_path("trace-schema-coding-service-eval-runner");
    let report = crate::default_coding_service_eval_runner_report();
    let line = crate::coding_service_eval_runner_trace_json_line(&report);
    fs::write(&path, format!("{line}\n")).unwrap();

    let trace_report = evaluate_trace_schema_jsonl(&path).unwrap();

    assert!(trace_report.passed, "{:?}", trace_report.failures);
    assert_eq!(trace_report.checked_lines, 1);
    assert_eq!(trace_report.coding_service_eval_events, 1);
    assert_eq!(trace_report.coding_service_eval_runner_events, 1);
    assert_eq!(trace_report.coding_service_eval_readiness_events, 0);
    assert_eq!(trace_report.coding_service_eval_passed, 1);
    assert_eq!(trace_report.coding_service_eval_requests, 5);
    assert_eq!(trace_report.coding_service_eval_completed, 5);
    assert_eq!(trace_report.coding_service_eval_evidence_packets, 5);
    assert_eq!(trace_report.coding_service_eval_rust_validation_checked, 2);
    assert_eq!(trace_report.coding_service_eval_compile_checked, 2);
    assert_eq!(trace_report.coding_service_eval_unit_test_checked, 2);
    assert_eq!(trace_report.coding_service_eval_write_allowed, 0);
    assert_eq!(trace_report.coding_service_eval_applied, 0);
    assert!(trace_report
        .summary_line()
        .contains("coding_service_eval_runner_events=1"));
    assert!(!line.contains("\"messages\""));
    assert!(!line.contains("\"evidence_packets\""));
    assert!(!line.contains("\"run_records\""));
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
    assert!(report
        .summary_line()
        .contains("memory_admission_ledger_records="));
    assert!(report.summary_line().contains("kv_fusion_saved_tokens="));
    cleanup(path);
}

#[test]
fn trace_schema_jsonl_gate_aggregates_adaptive_routing_and_task_hierarchy() {
    let path = temp_path("trace-schema-adaptive-routing");
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let first_outcome = engine.infer(
        InferenceRequest::new(
            "trace schema jsonl adaptive routing for Rust code memory",
            TaskProfile::Coding,
        ),
        &mut backend,
    );
    let second_outcome = engine.infer(
        InferenceRequest::new(
            "trace schema jsonl adaptive routing for long document context",
            TaskProfile::LongDocument,
        ),
        &mut backend,
    );
    let first_line = trace_json_line(
        "trace schema jsonl adaptive routing for Rust code memory",
        TaskProfile::Coding,
        8,
        &first_outcome,
    );
    let second_line = trace_json_line(
        "trace schema jsonl adaptive routing for long document context",
        TaskProfile::LongDocument,
        13,
        &second_outcome,
    );
    fs::write(&path, format!("{first_line}\n{second_line}\n")).unwrap();

    let report = evaluate_trace_schema_jsonl(&path).unwrap();
    let first_routing = json_object_after_field(&first_line, "adaptive_routing").unwrap();
    let second_routing = json_object_after_field(&second_line, "adaptive_routing").unwrap();
    let first_budget = json_object_after_field(&first_line, "compute_budget").unwrap();
    let second_budget = json_object_after_field(&second_line, "compute_budget").unwrap();
    let first_task = json_object_after_field(&first_line, "task_hierarchy").unwrap();
    let second_task = json_object_after_field(&second_line, "task_hierarchy").unwrap();
    let first_fht = json_object_after_field(&first_line, "fht_dke").unwrap();
    let second_fht = json_object_after_field(&second_line, "fht_dke").unwrap();

    let expected_candidates = extract_json_usize_field(first_routing, "candidates")
        .unwrap()
        .saturating_add(extract_json_usize_field(second_routing, "candidates").unwrap());
    let expected_include = extract_json_usize_field(first_routing, "include")
        .unwrap()
        .saturating_add(extract_json_usize_field(second_routing, "include").unwrap());
    let expected_compress = extract_json_usize_field(first_routing, "compress")
        .unwrap()
        .saturating_add(extract_json_usize_field(second_routing, "compress").unwrap());
    let expected_defer = extract_json_usize_field(first_routing, "defer")
        .unwrap()
        .saturating_add(extract_json_usize_field(second_routing, "defer").unwrap());
    let expected_skip = extract_json_usize_field(first_routing, "skip")
        .unwrap()
        .saturating_add(extract_json_usize_field(second_routing, "skip").unwrap());
    let expected_input_tokens = extract_json_usize_field(first_routing, "input_tokens")
        .unwrap()
        .saturating_add(extract_json_usize_field(second_routing, "input_tokens").unwrap());
    let expected_retained_tokens = extract_json_usize_field(first_routing, "retained_tokens")
        .unwrap()
        .saturating_add(extract_json_usize_field(second_routing, "retained_tokens").unwrap());
    let expected_saved_tokens = extract_json_usize_field(first_routing, "saved_tokens")
        .unwrap()
        .saturating_add(extract_json_usize_field(second_routing, "saved_tokens").unwrap());
    let expected_mutation_records = extract_json_usize_field(first_task, "mutation_records")
        .unwrap()
        .saturating_add(extract_json_usize_field(second_task, "mutation_records").unwrap());
    let expected_route_pressure_milli =
        trace_milli(extract_json_f32_field(first_task, "route_pressure").unwrap()).saturating_add(
            trace_milli(extract_json_f32_field(second_task, "route_pressure").unwrap()),
        );
    let expected_compute_reduction_milli =
        trace_milli(extract_json_f32_field(first_task, "compute_reduction").unwrap())
            .saturating_add(trace_milli(
                extract_json_f32_field(second_task, "compute_reduction").unwrap(),
            ));
    let expected_task_hierarchy_depth_total =
        extract_json_usize_field(first_task, "hierarchy_depth")
            .unwrap()
            .saturating_add(extract_json_usize_field(second_task, "hierarchy_depth").unwrap());
    let expected_task_hierarchy_route_fanout_total =
        extract_json_usize_field(first_task, "route_fanout")
            .unwrap()
            .saturating_add(extract_json_usize_field(second_task, "route_fanout").unwrap());
    let expected_task_hierarchy_threshold_delta_milli =
        trace_milli(extract_json_f32_field(first_task, "threshold_delta").unwrap()).saturating_add(
            trace_milli(extract_json_f32_field(second_task, "threshold_delta").unwrap()),
        );
    let expected_task_hierarchy_selected_lanes =
        extract_json_string_array_field(first_task, "selected_lanes")
            .unwrap()
            .len()
            .saturating_add(
                extract_json_string_array_field(second_task, "selected_lanes")
                    .unwrap()
                    .len(),
            );
    let expected_task_hierarchy_skipped_lanes =
        extract_json_string_array_field(first_task, "skipped_lanes")
            .unwrap()
            .len()
            .saturating_add(
                extract_json_string_array_field(second_task, "skipped_lanes")
                    .unwrap()
                    .len(),
            );
    let expected_task_hierarchy_memory_lanes =
        extract_json_string_array_field(first_task, "memory_lanes")
            .unwrap()
            .len()
            .saturating_add(
                extract_json_string_array_field(second_task, "memory_lanes")
                    .unwrap()
                    .len(),
            );
    let expected_task_hierarchy_skipped_memory_lanes =
        extract_json_string_array_field(first_task, "skipped_memory_lanes")
            .unwrap()
            .len()
            .saturating_add(
                extract_json_string_array_field(second_task, "skipped_memory_lanes")
                    .unwrap()
                    .len(),
            );
    let expected_budget_threshold_delta_milli =
        trace_milli(extract_json_f32_field(first_budget, "threshold_delta").unwrap())
            .saturating_add(trace_milli(
                extract_json_f32_field(second_budget, "threshold_delta").unwrap(),
            ));
    let expected_budget_runtime_kv_pressure_milli =
        trace_milli(extract_json_f32_field(first_budget, "runtime_kv_budget_pressure").unwrap())
            .saturating_add(trace_milli(
                extract_json_f32_field(second_budget, "runtime_kv_budget_pressure").unwrap(),
            ));
    let expected_budget_selected = extract_json_usize_field(first_budget, "selected_candidates")
        .unwrap()
        .saturating_add(extract_json_usize_field(second_budget, "selected_candidates").unwrap());
    let expected_budget_saved = extract_json_usize_field(first_budget, "saved_tokens")
        .unwrap()
        .saturating_add(extract_json_usize_field(second_budget, "saved_tokens").unwrap());
    let expected_budget_sem_fusion_saved =
        extract_json_usize_field(first_budget, "self_evolving_memory_fusion_saved_tokens")
            .unwrap()
            .saturating_add(
                extract_json_usize_field(second_budget, "self_evolving_memory_fusion_saved_tokens")
                    .unwrap(),
            );
    let expected_budget_avoided =
        extract_json_usize_field(first_budget, "wasted_compute_avoided_tokens")
            .unwrap()
            .saturating_add(
                extract_json_usize_field(second_budget, "wasted_compute_avoided_tokens").unwrap(),
            );
    let expected_budget_fanout_before =
        extract_json_usize_field(first_budget, "route_fanout_before")
            .unwrap()
            .saturating_add(
                extract_json_usize_field(second_budget, "route_fanout_before").unwrap(),
            );
    let expected_budget_fanout_after = extract_json_usize_field(first_budget, "route_fanout_after")
        .unwrap()
        .saturating_add(extract_json_usize_field(second_budget, "route_fanout_after").unwrap());
    let expected_budget_fanout_reduction =
        extract_json_usize_field(first_budget, "route_fanout_before")
            .unwrap()
            .saturating_sub(extract_json_usize_field(first_budget, "route_fanout_after").unwrap())
            .saturating_add(
                extract_json_usize_field(second_budget, "route_fanout_before")
                    .unwrap()
                    .saturating_sub(
                        extract_json_usize_field(second_budget, "route_fanout_after").unwrap(),
                    ),
            );
    let expected_budget_estimated_budget_tokens =
        extract_json_usize_field(first_budget, "estimated_budget_tokens")
            .unwrap()
            .saturating_add(
                extract_json_usize_field(second_budget, "estimated_budget_tokens").unwrap(),
            );
    let expected_budget_estimated_spent_tokens =
        extract_json_usize_field(first_budget, "estimated_spent_tokens")
            .unwrap()
            .saturating_add(
                extract_json_usize_field(second_budget, "estimated_spent_tokens").unwrap(),
            );
    let expected_budget_estimated_saved_tokens =
        extract_json_usize_field(first_budget, "estimated_budget_tokens")
            .unwrap()
            .saturating_sub(
                extract_json_usize_field(first_budget, "estimated_spent_tokens").unwrap(),
            )
            .saturating_add(
                extract_json_usize_field(second_budget, "estimated_budget_tokens")
                    .unwrap()
                    .saturating_sub(
                        extract_json_usize_field(second_budget, "estimated_spent_tokens").unwrap(),
                    ),
            );
    let expected_budget_anchor_count = extract_json_usize_field(first_budget, "anchor_count")
        .unwrap()
        .saturating_add(extract_json_usize_field(second_budget, "anchor_count").unwrap());
    let expected_budget_anchors_preserved =
        extract_json_usize_field(first_budget, "anchors_preserved_count")
            .unwrap()
            .saturating_add(
                extract_json_usize_field(second_budget, "anchors_preserved_count").unwrap(),
            );
    let expected_budget_anchor_preservation_failures = usize::from(
        extract_json_usize_field(first_budget, "anchors_preserved_count").unwrap()
            < extract_json_usize_field(first_budget, "anchor_count").unwrap(),
    )
    .saturating_add(usize::from(
        extract_json_usize_field(second_budget, "anchors_preserved_count").unwrap()
            < extract_json_usize_field(second_budget, "anchor_count").unwrap(),
    ));
    let expected_budget_fallback_triggered =
        usize::from(extract_json_bool_field(first_budget, "fallback_triggered").unwrap())
            .saturating_add(usize::from(
                extract_json_bool_field(second_budget, "fallback_triggered").unwrap(),
            ));
    let expected_fht_total = extract_json_usize_field(first_fht, "total_tokens")
        .unwrap()
        .saturating_add(extract_json_usize_field(second_fht, "total_tokens").unwrap());
    let expected_fht_dense = extract_json_usize_field(first_fht, "dense_tokens")
        .unwrap()
        .saturating_add(extract_json_usize_field(second_fht, "dense_tokens").unwrap());
    let expected_fht_routed = extract_json_usize_field(first_fht, "routed_tokens")
        .unwrap()
        .saturating_add(extract_json_usize_field(second_fht, "routed_tokens").unwrap());
    let expected_fht_kv_exchange = extract_json_usize_field(first_fht, "kv_exchange_blocks")
        .unwrap()
        .saturating_add(extract_json_usize_field(second_fht, "kv_exchange_blocks").unwrap());
    let expected_fht_attention_threshold_milli =
        trace_milli(extract_json_f32_field(first_fht, "attention_threshold").unwrap())
            .saturating_add(trace_milli(
                extract_json_f32_field(second_fht, "attention_threshold").unwrap(),
            ));
    let expected_fht_route_pressure_milli =
        trace_milli(extract_json_f32_field(first_fht, "route_pressure").unwrap()).saturating_add(
            trace_milli(extract_json_f32_field(second_fht, "route_pressure").unwrap()),
        );

    assert!(report.passed, "{:?}", report.failures);
    assert_eq!(report.checked_lines, 2);
    assert_eq!(report.adaptive_routing_events, 2);
    assert_eq!(report.adaptive_routing_candidates, expected_candidates);
    assert!(report.adaptive_routing_candidates >= 2);
    assert_eq!(report.adaptive_routing_include, expected_include);
    assert_eq!(report.adaptive_routing_compress, expected_compress);
    assert_eq!(report.adaptive_routing_defer, expected_defer);
    assert_eq!(report.adaptive_routing_skip, expected_skip);
    assert_eq!(
        report.adaptive_routing_include
            + report.adaptive_routing_compress
            + report.adaptive_routing_defer
            + report.adaptive_routing_skip,
        report.adaptive_routing_candidates
    );
    assert_eq!(report.adaptive_routing_input_tokens, expected_input_tokens);
    assert_eq!(
        report.adaptive_routing_retained_tokens,
        expected_retained_tokens
    );
    assert_eq!(report.adaptive_routing_saved_tokens, expected_saved_tokens);
    assert_eq!(
        report
            .adaptive_routing_retained_tokens
            .saturating_add(report.adaptive_routing_saved_tokens),
        report.adaptive_routing_input_tokens
    );
    assert_eq!(report.task_hierarchy_events, 2);
    assert_eq!(
        report.task_hierarchy_mutation_records,
        expected_mutation_records
    );
    assert!(report.task_hierarchy_mutation_records >= 2);
    assert_eq!(
        report.task_hierarchy_route_pressure_milli,
        expected_route_pressure_milli
    );
    assert_eq!(
        report.task_hierarchy_compute_reduction_milli,
        expected_compute_reduction_milli
    );
    assert_eq!(
        report.task_hierarchy_depth_total,
        expected_task_hierarchy_depth_total
    );
    assert_eq!(
        report.task_hierarchy_route_fanout_total,
        expected_task_hierarchy_route_fanout_total
    );
    assert_eq!(
        report.task_hierarchy_threshold_delta_milli,
        expected_task_hierarchy_threshold_delta_milli
    );
    assert_eq!(
        report.task_hierarchy_selected_lanes,
        expected_task_hierarchy_selected_lanes
    );
    assert_eq!(
        report.task_hierarchy_skipped_lanes,
        expected_task_hierarchy_skipped_lanes
    );
    assert_eq!(
        report.task_hierarchy_memory_lanes,
        expected_task_hierarchy_memory_lanes
    );
    assert_eq!(
        report.task_hierarchy_skipped_memory_lanes,
        expected_task_hierarchy_skipped_memory_lanes
    );
    assert_eq!(report.fht_dke_events, 2);
    assert_eq!(report.fht_dke_enabled, 2);
    assert_eq!(report.fht_dke_total_tokens, expected_fht_total);
    assert_eq!(report.fht_dke_dense_tokens, expected_fht_dense);
    assert_eq!(report.fht_dke_routed_tokens, expected_fht_routed);
    assert_eq!(report.fht_dke_kv_exchange_blocks, expected_fht_kv_exchange);
    assert_eq!(report.fht_dke_token_split_invalid, 0);
    assert_eq!(
        report.fht_dke_attention_threshold_milli,
        expected_fht_attention_threshold_milli
    );
    assert_eq!(
        report.fht_dke_route_pressure_milli,
        expected_fht_route_pressure_milli
    );
    assert_eq!(report.compute_budget_events, 2);
    assert_eq!(
        report.compute_budget_threshold_delta_milli,
        expected_budget_threshold_delta_milli
    );
    assert_eq!(
        report.compute_budget_runtime_kv_budget_pressure_milli,
        expected_budget_runtime_kv_pressure_milli
    );
    assert_eq!(
        report.compute_budget_selected_candidates,
        expected_budget_selected
    );
    assert_eq!(report.compute_budget_saved_tokens, expected_budget_saved);
    assert_eq!(
        report.compute_budget_self_evolving_memory_fusion_saved_tokens,
        expected_budget_sem_fusion_saved
    );
    assert_eq!(
        report.compute_budget_avoided_tokens,
        expected_budget_avoided
    );
    assert_eq!(
        report.compute_budget_fanout_before,
        expected_budget_fanout_before
    );
    assert_eq!(
        report.compute_budget_fanout_after,
        expected_budget_fanout_after
    );
    assert_eq!(
        report.compute_budget_fanout_reduction,
        expected_budget_fanout_reduction
    );
    assert_eq!(
        report.compute_budget_estimated_budget_tokens,
        expected_budget_estimated_budget_tokens
    );
    assert_eq!(
        report.compute_budget_estimated_spent_tokens,
        expected_budget_estimated_spent_tokens
    );
    assert_eq!(
        report.compute_budget_estimated_saved_tokens,
        expected_budget_estimated_saved_tokens
    );
    assert_eq!(
        report.compute_budget_anchor_count,
        expected_budget_anchor_count
    );
    assert_eq!(
        report.compute_budget_anchors_preserved,
        expected_budget_anchors_preserved
    );
    assert_eq!(
        report.compute_budget_anchor_preservation_failures,
        expected_budget_anchor_preservation_failures
    );
    assert_eq!(
        report.compute_budget_fallback_triggered,
        expected_budget_fallback_triggered
    );
    assert_eq!(report.compute_budget_write_allowed, 0);
    assert_eq!(report.compute_budget_applied, 0);
    assert!(report
        .summary_line()
        .contains("adaptive_routing_candidates="));
    assert!(report
        .summary_line()
        .contains("task_hierarchy_mutation_records="));
    assert!(report
        .summary_line()
        .contains("task_hierarchy_threshold_delta_milli="));
    assert!(report.summary_line().contains("compute_budget_events=2"));
    assert!(report
        .summary_line()
        .contains("compute_budget_threshold_delta_milli="));
    assert!(report
        .summary_line()
        .contains("compute_budget_runtime_kv_budget_pressure_milli="));
    assert!(report
        .summary_line()
        .contains("compute_budget_self_evolving_memory_fusion_saved_tokens="));
    assert!(report
        .summary_line()
        .contains("compute_budget_fanout_reduction="));
    assert!(report
        .summary_line()
        .contains("compute_budget_estimated_saved_tokens="));
    assert!(report
        .summary_line()
        .contains("compute_budget_anchor_preservation_failures="));
    assert!(report.summary_line().contains("fht_dke_events=2"));
    assert!(report
        .summary_line()
        .contains("fht_dke_token_split_invalid=0"));
    cleanup(path);
}

struct BudgetLimitedRuntimeKvTraceBackend;

impl InferenceBackend for BudgetLimitedRuntimeKvTraceBackend {
    fn generate(&mut self, _context: GenerationContext<'_>) -> InferenceDraft {
        let diagnostics = RuntimeDiagnostics {
            imported_kv_blocks: 2,
            weak_runtime_kv_imports_skipped: 3,
            budget_limited_runtime_kv_imports_skipped: 4,
            ..RuntimeDiagnostics::default()
        };

        InferenceDraft::new(
            "Runtime KV pressure trace keeps compute budget evidence visible.",
            vec![ReasoningStep::new(
                "runtime_kv",
                "runtime skipped KV imports under budget pressure",
                0.82,
            )],
        )
        .with_runtime_diagnostics(diagnostics)
        .with_exported_kv_blocks(vec![RuntimeKvBlock::new(
            4,
            2,
            0,
            4,
            vec![0.2, 0.1],
            vec![0.4, 0.3],
        )])
    }
}

#[test]
fn trace_schema_jsonl_gate_aggregates_compute_budget_runtime_kv_pressure() {
    let path = temp_path("trace-schema-compute-budget-runtime-kv-pressure");
    let mut engine = NoironEngine::new();
    let mut backend = BudgetLimitedRuntimeKvTraceBackend;
    let outcome = engine.infer(
        InferenceRequest::new("trace runtime kv pressure", TaskProfile::Coding)
            .with_max_tokens(Some(64)),
        &mut backend,
    );
    let line = trace_json_line(
        "trace runtime kv pressure",
        TaskProfile::Coding,
        8,
        &outcome,
    );
    let budget = json_object_after_field(&line, "compute_budget").unwrap();
    let runtime = json_object_after_field(&line, "runtime_diagnostics").unwrap();

    assert_eq!(
        trace_milli(extract_json_f32_field(budget, "runtime_kv_budget_pressure").unwrap()),
        800
    );
    assert_eq!(
        trace_milli(
            extract_json_nullable_f32_field(runtime, "runtime_kv_weak_import_pressure").unwrap()
        ),
        600
    );
    fs::write(&path, format!("{line}\n")).unwrap();

    let report = evaluate_trace_schema_jsonl(&path).unwrap();
    let snapshot = report.operator_health_snapshot();
    let routing = snapshot.section("routing").unwrap();
    let learning = snapshot.section("learning").unwrap();

    assert!(report.passed, "{:?}", report.failures);
    assert_eq!(report.compute_budget_runtime_kv_budget_pressure_milli, 800);
    assert_eq!(report.runtime_kv_weak_import_pressure_milli, 600);
    assert!(report
        .summary_line()
        .contains("compute_budget_runtime_kv_budget_pressure_milli=800"));
    assert!(report
        .summary_line()
        .contains("runtime_kv_weak_import_pressure_milli=600"));
    assert_eq!(
        routing.metric("compute_budget_runtime_kv_budget_pressure_milli"),
        Some(800)
    );
    assert_eq!(
        learning.metric("runtime_kv_weak_import_pressure_milli"),
        Some(600)
    );
    assert!(routing.review_required);
    assert!(learning.review_required);
    cleanup(path);
}

#[test]
fn trace_schema_jsonl_gate_aggregates_self_evolution_experiments() {
    let path = temp_path("trace-schema-self-evolution-experiments");
    let mut ledger = SelfEvolutionExperimentLedger::new();
    let admitted = ledger.append_admission_report(
        "experiment-pass",
        &self_evolution_experiment_passing_report("candidate-pass"),
    );
    let held = ledger.append_admission_report(
        "experiment-hold",
        &self_evolution_experiment_hold_report("candidate-hold"),
    );
    let rejected = ledger.append_admission_report(
        "experiment-reject",
        &self_evolution_experiment_reject_report("candidate-reject"),
    );
    let rollback = ledger.append_admission_report(
        "experiment-pass",
        &self_evolution_experiment_rollback_report("candidate-rollback"),
    );

    append_self_evolution_experiment_trace_jsonl(&path, &admitted).unwrap();
    append_self_evolution_experiment_trace_jsonl(&path, &held).unwrap();
    append_self_evolution_experiment_trace_jsonl(&path, &rejected).unwrap();
    append_self_evolution_experiment_trace_jsonl(&path, &rollback).unwrap();

    let report = evaluate_trace_schema_jsonl(&path).unwrap();

    assert!(report.passed, "{:?}", report.failures);
    assert_eq!(report.checked_lines, 4);
    assert_eq!(report.self_evolution_experiment_events, 4);
    assert_eq!(report.self_evolution_experiment_admit, 1);
    assert_eq!(report.self_evolution_experiment_hold, 1);
    assert_eq!(report.self_evolution_experiment_reject, 1);
    assert_eq!(report.self_evolution_experiment_rollback, 1);
    assert_eq!(report.self_evolution_experiment_repeated, 1);
    assert_eq!(report.self_evolution_experiment_conflicts, 0);
    assert_eq!(report.self_evolution_experiment_rollback_replayable, 1);
    assert_eq!(report.self_evolution_experiment_active_candidates, 0);
    assert_eq!(report.self_evolution_experiment_write_allowed, 0);
    assert_eq!(report.self_evolution_experiment_applied, 0);
    assert!(report
        .summary_line()
        .contains("self_evolution_experiment_events=4"));
    assert!(report
        .summary_line()
        .contains("self_evolution_experiment_rollback=1"));
    cleanup(path);
}

#[test]
fn trace_schema_jsonl_gate_aggregates_self_evolution_rollback_replay_plans() {
    let path = temp_path("trace-schema-self-evolution-rollback-replay");
    let mut ledger = SelfEvolutionExperimentLedger::new();
    ledger.append_admission_report(
        "experiment-pass",
        &self_evolution_experiment_passing_report("candidate-pass"),
    );
    ledger.append_admission_report(
        "experiment-rollback",
        &self_evolution_experiment_rollback_report("candidate-rollback"),
    );
    let plan = ledger.rollback_replay_plan();
    let gate_report = SelfEvolutionRollbackReplayGate::new().evaluate(&plan);

    append_self_evolution_rollback_replay_trace_jsonl(&path, &plan).unwrap();
    append_self_evolution_rollback_replay_gate_trace_jsonl(&path, &gate_report).unwrap();

    let report = evaluate_trace_schema_jsonl(&path).unwrap();

    assert!(report.passed, "{:?}", report.failures);
    assert_eq!(report.checked_lines, 2);
    assert_eq!(report.self_evolution_rollback_replay_events, 1);
    assert_eq!(report.self_evolution_rollback_replay_items, 1);
    assert_eq!(report.self_evolution_rollback_replay_replayable, 1);
    assert_eq!(report.self_evolution_rollback_replay_blocked, 0);
    assert_eq!(report.self_evolution_rollback_replay_all_replayable, 1);
    assert_eq!(
        report.self_evolution_rollback_replay_rollback_anchor_ids,
        plan.rollback_anchor_ids().len()
    );
    assert_eq!(
        report.self_evolution_rollback_replay_evidence_ids,
        plan.evidence_ids().len()
    );
    assert_eq!(report.self_evolution_rollback_replay_active_candidates, 0);
    assert_eq!(report.self_evolution_rollback_replay_item_write_allowed, 0);
    assert_eq!(report.self_evolution_rollback_replay_item_applied, 0);
    assert_eq!(report.self_evolution_rollback_replay_write_allowed, 0);
    assert_eq!(report.self_evolution_rollback_replay_applied, 0);
    assert_eq!(report.self_evolution_rollback_replay_gate_events, 1);
    assert_eq!(report.self_evolution_rollback_replay_gate_admitted, 1);
    assert_eq!(report.self_evolution_rollback_replay_gate_held, 0);
    assert_eq!(
        report.self_evolution_rollback_replay_gate_review_packets,
        gate_report.review_packet.approval_review_packet_ids.len()
    );
    assert_eq!(
        report.self_evolution_rollback_replay_gate_review_evidence_ids,
        gate_report.review_packet.evidence_ids.len()
    );
    assert_eq!(
        report.self_evolution_rollback_replay_gate_missing_review_packet_refs,
        0
    );
    assert_eq!(report.self_evolution_rollback_replay_gate_items, 1);
    assert_eq!(report.self_evolution_rollback_replay_gate_replayable, 1);
    assert_eq!(report.self_evolution_rollback_replay_gate_blocked, 0);
    assert_eq!(report.self_evolution_rollback_replay_gate_all_replayable, 1);
    assert_eq!(
        report.self_evolution_rollback_replay_gate_rollback_anchor_ids,
        gate_report.rollback_anchor_ids.len()
    );
    assert_eq!(
        report.self_evolution_rollback_replay_gate_evidence_ids,
        gate_report.evidence_ids.len()
    );
    assert_eq!(
        report.self_evolution_rollback_replay_gate_active_candidates,
        0
    );
    assert_eq!(
        report.self_evolution_rollback_replay_gate_item_write_allowed,
        0
    );
    assert_eq!(report.self_evolution_rollback_replay_gate_item_applied, 0);
    assert_eq!(
        report.self_evolution_rollback_replay_gate_plan_write_allowed,
        0
    );
    assert_eq!(report.self_evolution_rollback_replay_gate_plan_applied, 0);
    assert_eq!(report.self_evolution_rollback_replay_gate_write_allowed, 0);
    assert_eq!(report.self_evolution_rollback_replay_gate_applied, 0);
    assert!(report
        .summary_line()
        .contains("self_evolution_rollback_replay_events=1"));
    assert!(report
        .summary_line()
        .contains("self_evolution_rollback_replay_replayable=1"));
    assert!(report
        .summary_line()
        .contains("self_evolution_rollback_replay_gate_admitted=1"));
    assert!(report
        .summary_line()
        .contains("self_evolution_rollback_replay_gate_review_packets=1"));
    cleanup(path);
}

#[test]
fn trace_schema_jsonl_gate_accepts_held_self_evolution_rollback_replay_gate() {
    let path = temp_path("trace-schema-self-evolution-rollback-replay-gate-held");
    let plan = SelfEvolutionRollbackReplayPlan::new(Vec::new());
    let gate_report = SelfEvolutionRollbackReplayGate::new().evaluate(&plan);

    append_self_evolution_rollback_replay_gate_trace_jsonl(&path, &gate_report).unwrap();

    let report = evaluate_trace_schema_jsonl(&path).unwrap();

    assert!(report.passed, "{:?}", report.failures);
    assert_eq!(report.checked_lines, 1);
    assert_eq!(report.self_evolution_rollback_replay_gate_events, 1);
    assert_eq!(report.self_evolution_rollback_replay_gate_admitted, 0);
    assert_eq!(report.self_evolution_rollback_replay_gate_held, 1);
    assert_eq!(
        report.self_evolution_rollback_replay_gate_review_packets,
        gate_report.review_packet.approval_review_packet_ids.len()
    );
    assert_eq!(
        report.self_evolution_rollback_replay_gate_review_evidence_ids,
        gate_report.review_packet.evidence_ids.len()
    );
    assert_eq!(
        report.self_evolution_rollback_replay_gate_missing_review_packet_refs,
        0
    );
    assert_eq!(report.self_evolution_rollback_replay_gate_items, 0);
    assert_eq!(report.self_evolution_rollback_replay_gate_replayable, 0);
    assert_eq!(report.self_evolution_rollback_replay_gate_blocked, 0);
    assert_eq!(report.self_evolution_rollback_replay_gate_all_replayable, 1);
    assert_eq!(
        report.self_evolution_rollback_replay_gate_rollback_anchor_ids,
        0
    );
    assert_eq!(report.self_evolution_rollback_replay_gate_evidence_ids, 0);
    assert!(report
        .summary_line()
        .contains("self_evolution_rollback_replay_gate_held=1"));
    cleanup(path);
}

#[test]
fn trace_schema_jsonl_gate_aggregates_self_evolution_operator_approvals() {
    let path = temp_path("trace-schema-self-evolution-operator-approval");
    let mut ledger = SelfEvolutionExperimentLedger::new();
    ledger.append_admission_report(
        "experiment-rollback",
        &self_evolution_experiment_rollback_report("candidate-rollback"),
    );
    let plan = ledger.rollback_replay_plan();
    let replay_gate = SelfEvolutionRollbackReplayGate::new().evaluate(&plan);
    let evidence = SelfEvolutionOperatorApprovalEvidence::from_review_packet(
        "maintainer-jy",
        "approval-ticket-jsonl",
        &replay_gate.review_packet,
        "approved for trace gate aggregation",
    );
    let approved =
        SelfEvolutionOperatorApprovalGate::new().evaluate(&replay_gate.review_packet, &evidence);
    let mut held_evidence = evidence.clone();
    held_evidence.approval_ticket_id.clear();
    let held = SelfEvolutionOperatorApprovalGate::new()
        .evaluate(&replay_gate.review_packet, &held_evidence);

    append_self_evolution_operator_approval_trace_jsonl(&path, &approved).unwrap();
    append_self_evolution_operator_approval_trace_jsonl(&path, &held).unwrap();

    let report = evaluate_trace_schema_jsonl(&path).unwrap();

    assert!(report.passed, "{:?}", report.failures);
    assert_eq!(report.checked_lines, 2);
    assert_eq!(report.self_evolution_operator_approval_events, 2);
    assert_eq!(report.self_evolution_operator_approval_approved, 1);
    assert_eq!(report.self_evolution_operator_approval_held, 1);
    assert_eq!(
        report.self_evolution_operator_approval_review_packets,
        replay_gate
            .review_packet
            .approval_review_packet_ids
            .len()
            .saturating_mul(2)
    );
    assert_eq!(
        report.self_evolution_operator_approval_evidence_ids,
        replay_gate
            .review_packet
            .evidence_ids
            .len()
            .saturating_mul(2)
    );
    assert_eq!(
        report.self_evolution_operator_approval_rollback_anchor_ids,
        replay_gate
            .review_packet
            .rollback_anchor_ids
            .len()
            .saturating_mul(2)
    );
    assert_eq!(
        report.self_evolution_operator_approval_content_digests,
        replay_gate
            .review_packet
            .content_digests
            .len()
            .saturating_mul(2)
    );
    assert_eq!(
        report.self_evolution_operator_approval_source_report_schemas,
        replay_gate
            .review_packet
            .source_report_schemas
            .len()
            .saturating_mul(2)
    );
    assert_eq!(
        report.self_evolution_operator_approval_missing_review_packet_refs,
        0
    );
    assert_eq!(report.self_evolution_operator_approval_write_allowed, 0);
    assert_eq!(report.self_evolution_operator_approval_applied, 0);
    let counters = report.self_evolution_operator_approval_service_counters();
    assert!(counters.data_present);
    assert!(!counters.approval_ready);
    assert!(counters.review_required);
    assert!(!counters.blocked);
    assert!(counters.validation_failures().is_empty());
    assert!(!counters.activation_allowed);
    assert!(!counters.memory_write_allowed);
    assert!(!counters.genome_write_allowed);
    assert!(!counters.kv_write_allowed);
    assert!(report
        .summary_line()
        .contains("self_evolution_operator_approval_events=2"));
    assert!(report
        .summary_line()
        .contains("self_evolution_operator_approval_held=1"));
    cleanup(path);
}

#[test]
fn self_evolution_operator_approval_service_counters_mark_clean_approval_ready() {
    let report = TraceSchemaGateReport {
        passed: true,
        checked_lines: 1,
        self_evolution_operator_approval_events: 1,
        self_evolution_operator_approval_approved: 1,
        self_evolution_operator_approval_review_packets: 1,
        self_evolution_operator_approval_evidence_ids: 2,
        self_evolution_operator_approval_rollback_anchor_ids: 1,
        self_evolution_operator_approval_content_digests: 1,
        self_evolution_operator_approval_source_report_schemas: 1,
        ..TraceSchemaGateReport::default()
    };

    let counters = report.self_evolution_operator_approval_service_counters();
    let json = counters.json_object();

    assert!(counters.data_present);
    assert!(counters.approval_ready);
    assert!(!counters.review_required);
    assert!(!counters.blocked);
    assert!(counters.validation_failures().is_empty());
    assert!(!counters.activation_allowed);
    assert!(!counters.memory_write_allowed);
    assert!(!counters.genome_write_allowed);
    assert!(!counters.kv_write_allowed);
    assert!(json.contains("\"approval_ready\":true"));
    assert!(json.contains("\"activation_allowed\":false"));
    assert!(json.contains("\"memory_write_allowed\":false"));
    assert!(json.contains("\"genome_write_allowed\":false"));
    assert!(json.contains("\"kv_write_allowed\":false"));
    assert!(json.contains("\"validation_failures\":[]"));
}

#[test]
fn self_evolution_operator_approval_service_counters_fail_closed_on_mutating_flags() {
    let report = TraceSchemaGateReport {
        passed: true,
        checked_lines: 1,
        self_evolution_operator_approval_events: 1,
        self_evolution_operator_approval_approved: 1,
        self_evolution_operator_approval_review_packets: 0,
        self_evolution_operator_approval_evidence_ids: 0,
        self_evolution_operator_approval_rollback_anchor_ids: 0,
        self_evolution_operator_approval_content_digests: 0,
        self_evolution_operator_approval_source_report_schemas: 0,
        self_evolution_operator_approval_missing_review_packet_refs: 1,
        self_evolution_operator_approval_write_allowed: 1,
        self_evolution_operator_approval_applied: 1,
        ..TraceSchemaGateReport::default()
    };

    let counters = report.self_evolution_operator_approval_service_counters();
    let failures = counters.validation_failures();
    let json = counters.json_object();

    assert!(counters.data_present);
    assert!(!counters.approval_ready);
    assert!(counters.review_required);
    assert!(counters.blocked);
    assert!(failures
        .contains(&"self_evolution_operator_approval_approved_missing_review_packets".to_owned()));
    assert!(failures
        .contains(&"self_evolution_operator_approval_approved_missing_evidence_ids".to_owned()));
    assert!(failures.contains(
        &"self_evolution_operator_approval_approved_missing_rollback_anchors".to_owned()
    ));
    assert!(failures
        .contains(&"self_evolution_operator_approval_approved_missing_content_digests".to_owned()));
    assert!(failures.contains(
        &"self_evolution_operator_approval_approved_missing_source_report_schemas".to_owned()
    ));
    assert!(failures
        .contains(&"self_evolution_operator_approval_missing_review_packet_refs".to_owned()));
    assert!(failures.contains(&"self_evolution_operator_approval_write_allowed".to_owned()));
    assert!(failures.contains(&"self_evolution_operator_approval_applied".to_owned()));
    assert!(!counters.activation_allowed);
    assert!(!counters.memory_write_allowed);
    assert!(!counters.genome_write_allowed);
    assert!(!counters.kv_write_allowed);
    assert!(json.contains("\"blocked\":true"));
    assert!(json.contains("\"approval_ready\":false"));
    assert!(json.contains("self_evolution_operator_approval_write_allowed"));
}

#[test]
fn trace_schema_jsonl_gate_aggregates_self_evolution_rollback_replay_apply_preflights() {
    let path = temp_path("trace-schema-self-evolution-rollback-replay-apply");
    let mut ledger = SelfEvolutionExperimentLedger::new();
    ledger.append_admission_report(
        "experiment-rollback",
        &self_evolution_experiment_rollback_report("candidate-rollback"),
    );
    let plan = ledger.rollback_replay_plan();
    let replay_gate = SelfEvolutionRollbackReplayGate::new().evaluate(&plan);
    let evidence = SelfEvolutionOperatorApprovalEvidence::from_review_packet(
        "maintainer-jy",
        "approval-ticket-apply-jsonl",
        &replay_gate.review_packet,
        "approved for rollback replay apply trace aggregation",
    );
    let approved =
        SelfEvolutionOperatorApprovalGate::new().evaluate(&replay_gate.review_packet, &evidence);
    let ready = SelfEvolutionRollbackReplayApplyGate::new().evaluate(&replay_gate, &approved);
    let mut held_evidence = evidence.clone();
    held_evidence.approval_ticket_id.clear();
    let held_approval = SelfEvolutionOperatorApprovalGate::new()
        .evaluate(&replay_gate.review_packet, &held_evidence);
    let held = SelfEvolutionRollbackReplayApplyGate::new().evaluate(&replay_gate, &held_approval);

    append_self_evolution_rollback_replay_apply_trace_jsonl(&path, &ready).unwrap();
    append_self_evolution_rollback_replay_apply_trace_jsonl(&path, &held).unwrap();

    let report = evaluate_trace_schema_jsonl(&path).unwrap();

    assert!(report.passed, "{:?}", report.failures);
    assert_eq!(report.checked_lines, 2);
    assert_eq!(report.self_evolution_rollback_replay_apply_events, 2);
    assert_eq!(report.self_evolution_rollback_replay_apply_ready, 1);
    assert_eq!(report.self_evolution_rollback_replay_apply_held, 1);
    assert_eq!(
        report.self_evolution_rollback_replay_apply_items,
        ready.item_count * 2
    );
    assert_eq!(
        report.self_evolution_rollback_replay_apply_replayable,
        ready.replayable * 2
    );
    assert_eq!(
        report.self_evolution_rollback_replay_apply_blocked,
        ready.blocked + held.blocked
    );
    assert_eq!(
        report.self_evolution_rollback_replay_apply_review_packets,
        ready.review_packet_count + held.review_packet_count
    );
    assert_eq!(
        report.self_evolution_rollback_replay_apply_evidence_ids,
        ready.evidence_id_count + held.evidence_id_count
    );
    assert_eq!(
        report.self_evolution_rollback_replay_apply_rollback_anchor_ids,
        ready.rollback_anchor_count + held.rollback_anchor_count
    );
    assert_eq!(
        report.self_evolution_rollback_replay_apply_content_digests,
        ready.content_digest_count + held.content_digest_count
    );
    assert_eq!(
        report.self_evolution_rollback_replay_apply_source_report_schemas,
        ready.source_report_schema_count + held.source_report_schema_count
    );
    assert_eq!(report.self_evolution_rollback_replay_apply_missing_refs, 0);
    assert_eq!(
        report.self_evolution_rollback_replay_apply_blocked_reasons,
        held.blocked_reasons.len()
    );
    assert_eq!(report.self_evolution_rollback_replay_apply_write_allowed, 0);
    assert_eq!(report.self_evolution_rollback_replay_apply_applied, 0);
    assert!(report
        .summary_line()
        .contains("self_evolution_rollback_replay_apply_events=2"));
    assert!(report
        .summary_line()
        .contains("self_evolution_rollback_replay_apply_ready=1"));
    cleanup(path);
}

#[test]
fn trace_schema_jsonl_gate_aggregates_self_evolution_promotion_preflights() {
    let path = temp_path("trace-schema-self-evolution-promotion-preflight");
    let ready = self_evolution_promotion_preflight_report(true, "candidate-promotion-ready");
    let held = self_evolution_promotion_preflight_report(false, "candidate-promotion-held");

    append_self_evolution_promotion_preflight_trace_jsonl(&path, &ready).unwrap();
    append_self_evolution_promotion_preflight_trace_jsonl(&path, &held).unwrap();

    let report = evaluate_trace_schema_jsonl(&path).unwrap();

    assert!(report.passed, "{:?}", report.failures);
    assert_eq!(report.checked_lines, 2);
    assert_eq!(report.self_evolution_promotion_preflight_events, 2);
    assert_eq!(report.self_evolution_promotion_preflight_ready, 1);
    assert_eq!(report.self_evolution_promotion_preflight_held, 1);
    assert_eq!(
        report.self_evolution_promotion_preflight_review_packets,
        ready.review_packet_count + held.review_packet_count
    );
    assert_eq!(
        report.self_evolution_promotion_preflight_evidence_ids,
        ready.evidence_id_count + held.evidence_id_count
    );
    assert_eq!(
        report.self_evolution_promotion_preflight_rollback_anchor_ids,
        ready.rollback_anchor_count + held.rollback_anchor_count
    );
    assert_eq!(
        report.self_evolution_promotion_preflight_content_digests,
        ready.content_digest_count + held.content_digest_count
    );
    assert_eq!(
        report.self_evolution_promotion_preflight_source_report_schemas,
        ready.source_report_schema_count + held.source_report_schema_count
    );
    assert_eq!(report.self_evolution_promotion_preflight_missing_refs, 0);
    assert_eq!(
        report.self_evolution_promotion_preflight_blocked_reasons,
        held.blocked_reasons.len()
    );
    assert_eq!(report.self_evolution_promotion_preflight_write_allowed, 0);
    assert_eq!(report.self_evolution_promotion_preflight_applied, 0);
    assert!(report
        .summary_line()
        .contains("self_evolution_promotion_preflight_events=2"));
    assert!(report
        .summary_line()
        .contains("self_evolution_promotion_preflight_ready=1"));
    cleanup(path);
}

#[test]
fn trace_schema_jsonl_gate_aggregates_self_evolving_memory_store_reports() {
    let path = temp_path("trace-schema-self-evolving-memory-store");
    let mut store = SelfEvolvingMemoryStore::new();
    let approval = SelfEvolvingMemoryApproval::approved(
        "rollback:trace:self-evolving-memory",
        vec!["cargo-test:self-evolving-memory".to_owned()],
    );
    store.append_episode(
        SelfEvolvingEpisodeInput {
            problem: "private prompt must stay out of store trace JSONL".to_owned(),
            solution_path: "private solution path should be digested".to_owned(),
            outcome: "retrieval evidence stays redacted".to_owned(),
            key_insights: vec!["do not log raw payloads".to_owned()],
            tags: vec!["rust".to_owned(), "trace".to_owned()],
            profile: TaskProfile::Coding,
            quality: 0.91,
            token_estimate: 32,
            source_case_id: "case:self-evolving-memory-jsonl".to_owned(),
        },
        &approval,
    );
    store.append_heuristic(
        SelfEvolvingHeuristicInput {
            rule: "private heuristic should not appear in trace JSONL".to_owned(),
            tags: vec!["rust".to_owned(), "trace".to_owned()],
            profile: TaskProfile::Coding,
            priority: 0.80,
            confidence: 0.30,
            source_case_id: "case:self-evolving-memory-heuristic".to_owned(),
            updated_step: 1,
        },
        &approval,
    );
    let retrieval = store.retrieve_context(&SelfEvolvingMemoryQuery {
        prompt: "private retrieval prompt should be reduced to digest evidence".to_owned(),
        profile: TaskProfile::Coding,
        tags: vec!["rust".to_owned()],
        record_limit: 4,
        token_budget: 48,
    });
    let maintenance = store.maintain(&SelfEvolvingMemoryMaintenancePolicy {
        current_step: 20,
        stale_after_steps: 5,
        heuristic_decay: 0.50,
        tool_reliability_decay: 0.95,
        quarantine_below_confidence: 0.20,
        merge_duplicate_episodes: false,
    });
    let admission = SelfEvolvingMemoryAdmissionPreview {
        candidates: vec![
            SelfEvolvingMemoryAdmissionCandidatePreview {
                candidate_id: "sem_candidate_ready".to_owned(),
                kind: MemoryAdmissionKind::RetrospectiveEpisode,
                source_hash: "sha256:ready".to_owned(),
                rollback_anchor_id: "rollback:ready".to_owned(),
                validation_evidence_count: 1,
                eligible_for_store: true,
                blocked_reasons: Vec::new(),
                read_only: true,
                write_allowed: false,
                applied: false,
            },
            SelfEvolvingMemoryAdmissionCandidatePreview {
                candidate_id: "sem_candidate_blocked".to_owned(),
                kind: MemoryAdmissionKind::ProceduralHeuristic,
                source_hash: "sha256:blocked".to_owned(),
                rollback_anchor_id: "rollback:blocked".to_owned(),
                validation_evidence_count: 0,
                eligible_for_store: false,
                blocked_reasons: vec!["self_evolving_memory_validation_evidence_missing".to_owned()],
                read_only: true,
                write_allowed: false,
                applied: false,
            },
        ],
        read_only: true,
        write_allowed: false,
        applied: false,
    };
    let consolidation =
        SelfEvolvingMemoryConsolidationWorker::new(SelfEvolvingMemoryConsolidationPolicy {
            current_step: 20,
            stale_after_steps: 5,
            ..SelfEvolvingMemoryConsolidationPolicy::default()
        })
        .plan(&[
            MemoryConsolidationRecord::new(
                "episode:primary",
                "tenant:trace",
                MemoryConsolidationEvidenceClass::RetrospectiveEpisode,
                "trace-source",
                "trace-content",
                TaskProfile::Coding,
            )
            .with_scores(0.90, 0.90)
            .with_last_touched_step(20)
            .with_token_estimate(32)
            .with_validation_evidence_count(2),
            MemoryConsolidationRecord::new(
                "episode:duplicate",
                "tenant:trace",
                MemoryConsolidationEvidenceClass::RetrospectiveEpisode,
                "trace-source",
                "trace-content",
                TaskProfile::Coding,
            )
            .with_scores(0.85, 0.85)
            .with_last_touched_step(20)
            .with_token_estimate(32)
            .with_validation_evidence_count(2),
            MemoryConsolidationRecord::new(
                "heuristic:stale-low",
                "tenant:trace",
                MemoryConsolidationEvidenceClass::ProceduralHeuristic,
                "trace-stale",
                "trace-low-quality",
                TaskProfile::Coding,
            )
            .with_scores(0.04, 0.04)
            .with_last_touched_step(0)
            .with_token_estimate(4)
            .with_validation_evidence_count(1),
        ]);
    let quarantine = SelfEvolvingMemorySourceQuarantineReport {
        source_case_digest: "fnv64:1111111111111111".to_owned(),
        reason_code: "context_polluted".to_owned(),
        deactivated_episodes: 1,
        quarantined_heuristics: 1,
        removed_tool_observations: 1,
        tool_reliability_before: 2,
        tool_reliability_after: 1,
        redacted: true,
        applied: true,
    };
    fs::write(
        &path,
        format!(
            "{}\n{}\n{}\n{}\n{}\n",
            retrieval.json_line(),
            maintenance.json_line(),
            admission.json_line(),
            consolidation.json_line(),
            quarantine.json_line(
                true,
                Some("fnv64:2222222222222222"),
                Some("fnv64:2222222222222222"),
            )
        ),
    )
    .unwrap();

    let report = evaluate_trace_schema_jsonl(&path).unwrap();

    assert!(report.passed, "{:?}", report.failures);
    assert_eq!(report.checked_lines, 5);
    assert_eq!(report.self_evolving_memory_store_events, 5);
    assert_eq!(report.self_evolving_memory_store_retrieval_events, 1);
    assert_eq!(report.self_evolving_memory_store_maintenance_events, 1);
    assert_eq!(
        report.self_evolving_memory_store_admission_preview_events,
        1
    );
    assert_eq!(report.self_evolving_memory_store_consolidation_events, 1);
    assert_eq!(
        report.self_evolving_memory_store_consolidation_actions,
        consolidation.action_count()
    );
    assert_eq!(report.self_evolving_memory_store_merge_previews, 1);
    assert_eq!(report.self_evolving_memory_store_decay_previews, 0);
    assert_eq!(report.self_evolving_memory_store_tombstone_previews, 1);
    assert_eq!(report.self_evolving_memory_store_merge_rejections, 0);
    assert_eq!(
        report.self_evolving_memory_store_contexts,
        retrieval.total_contexts()
    );
    assert_eq!(
        report.self_evolving_memory_store_saved_tokens,
        retrieval.saved_tokens
    );
    assert_eq!(report.self_evolving_memory_store_saved_tokens, 32);
    assert_eq!(
        report.self_evolving_memory_store_maintenance_actions,
        maintenance.action_count()
    );
    assert_eq!(report.self_evolving_memory_store_admission_candidates, 2);
    assert_eq!(report.self_evolving_memory_store_write_allowed, 1);
    assert_eq!(report.self_evolving_memory_store_durable_write_allowed, 1);
    assert_eq!(report.self_evolving_memory_store_applied, 1);
    assert_eq!(report.self_evolving_memory_store_applied_to_disk, 1);
    assert_eq!(
        report.self_evolving_memory_store_source_quarantine_events,
        1
    );
    assert_eq!(
        report.self_evolving_memory_store_source_quarantine_actions,
        quarantine.action_count()
    );
    assert!(report
        .summary_line()
        .contains("self_evolving_memory_store_events=5"));
    assert!(report
        .summary_line()
        .contains("self_evolving_memory_store_consolidation_events=1"));
    assert!(report
        .summary_line()
        .contains("self_evolving_memory_store_saved_tokens=32"));
    assert!(report
        .summary_line()
        .contains("self_evolving_memory_store_source_quarantine_events=1"));
    cleanup(path);
}

#[test]
fn trace_schema_gate_rejects_retrieval_tokens_without_context() {
    let retrieval = SelfEvolvingMemoryRetrievalReport {
        requested_limit: 4,
        token_budget: 48,
        retained_tokens: 16,
        saved_tokens: 0,
        skipped_by_budget: 0,
        skipped_cross_profile: 0,
        episodes: Vec::new(),
        heuristics: Vec::new(),
        tool_reliability: Vec::new(),
        redacted: true,
    };

    let failures = evaluate_trace_schema_line(&retrieval.json_line());

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("retained_tokens requires retained context")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_retrieval_context_without_tokens() {
    let retrieval = SelfEvolvingMemoryRetrievalReport {
        requested_limit: 4,
        token_budget: 48,
        retained_tokens: 0,
        saved_tokens: 0,
        skipped_by_budget: 0,
        skipped_cross_profile: 0,
        episodes: vec![crate::self_evolving_memory::SelfEvolvingEpisodeContext {
            record_id: "episode:trace".to_owned(),
            problem_digest: "fnv64:1111111111111111".to_owned(),
            solution_path_digest: "fnv64:2222222222222222".to_owned(),
            outcome_digest: "fnv64:3333333333333333".to_owned(),
            key_insight_count: 1,
            source_case_digest: "fnv64:4444444444444444".to_owned(),
            score: 0.9,
            token_estimate: 1,
        }],
        heuristics: Vec::new(),
        tool_reliability: Vec::new(),
        redacted: true,
    };

    let failures = evaluate_trace_schema_line(&retrieval.json_line());

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("retained context requires retained_tokens")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_retrieval_context_count_above_tokens() {
    let retrieval = SelfEvolvingMemoryRetrievalReport {
        requested_limit: 4,
        token_budget: 48,
        retained_tokens: 1,
        saved_tokens: 0,
        skipped_by_budget: 0,
        skipped_cross_profile: 0,
        episodes: Vec::new(),
        heuristics: vec![
            crate::self_evolving_memory::SelfEvolvingHeuristicContext {
                record_id: "heuristic:one".to_owned(),
                rule_digest: "fnv64:1111111111111111".to_owned(),
                source_case_digest: "fnv64:2222222222222222".to_owned(),
                priority: 0.8,
                confidence: 0.7,
                score: 0.75,
                token_estimate: 32,
            },
            crate::self_evolving_memory::SelfEvolvingHeuristicContext {
                record_id: "heuristic:two".to_owned(),
                rule_digest: "fnv64:3333333333333333".to_owned(),
                source_case_digest: "fnv64:4444444444444444".to_owned(),
                priority: 0.7,
                confidence: 0.6,
                score: 0.65,
                token_estimate: 32,
            },
        ],
        tool_reliability: Vec::new(),
        redacted: true,
    };

    let failures = evaluate_trace_schema_line(&retrieval.json_line());

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("contexts 2 exceeds retained_tokens 1")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_retrieval_skipped_count_above_saved_tokens() {
    let retrieval = SelfEvolvingMemoryRetrievalReport {
        requested_limit: 4,
        token_budget: 48,
        retained_tokens: 1,
        saved_tokens: 1,
        skipped_by_budget: 2,
        skipped_cross_profile: 0,
        episodes: vec![crate::self_evolving_memory::SelfEvolvingEpisodeContext {
            record_id: "episode:trace".to_owned(),
            problem_digest: "fnv64:1111111111111111".to_owned(),
            solution_path_digest: "fnv64:2222222222222222".to_owned(),
            outcome_digest: "fnv64:3333333333333333".to_owned(),
            key_insight_count: 1,
            source_case_digest: "fnv64:4444444444444444".to_owned(),
            score: 0.9,
            token_estimate: 1,
        }],
        heuristics: Vec::new(),
        tool_reliability: Vec::new(),
        redacted: true,
    };

    let failures = evaluate_trace_schema_line(&retrieval.json_line());

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("skipped_by_budget 2 exceeds saved_tokens 1")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_retrieval_read_only_disabled() {
    let retrieval = SelfEvolvingMemoryRetrievalReport {
        requested_limit: 4,
        token_budget: 48,
        retained_tokens: 1,
        saved_tokens: 0,
        skipped_by_budget: 0,
        skipped_cross_profile: 0,
        episodes: vec![crate::self_evolving_memory::SelfEvolvingEpisodeContext {
            record_id: "episode:trace".to_owned(),
            problem_digest: "fnv64:1111111111111111".to_owned(),
            solution_path_digest: "fnv64:2222222222222222".to_owned(),
            outcome_digest: "fnv64:3333333333333333".to_owned(),
            key_insight_count: 1,
            source_case_digest: "fnv64:4444444444444444".to_owned(),
            score: 0.9,
            token_estimate: 1,
        }],
        heuristics: Vec::new(),
        tool_reliability: Vec::new(),
        redacted: true,
    };
    let line = retrieval
        .json_line()
        .replace("\"read_only\":true", "\"read_only\":false");

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("self_evolving_memory_store read_only must be true")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_retrieval_malformed_evidence_digest() {
    let retrieval = SelfEvolvingMemoryRetrievalReport {
        requested_limit: 4,
        token_budget: 48,
        retained_tokens: 1,
        saved_tokens: 0,
        skipped_by_budget: 0,
        skipped_cross_profile: 0,
        episodes: vec![crate::self_evolving_memory::SelfEvolvingEpisodeContext {
            record_id: "episode:trace".to_owned(),
            problem_digest: "fnv64:1111111111111111".to_owned(),
            solution_path_digest: "fnv64:2222222222222222".to_owned(),
            outcome_digest: "fnv64:3333333333333333".to_owned(),
            key_insight_count: 1,
            source_case_digest: "fnv64:4444444444444444".to_owned(),
            score: 0.9,
            token_estimate: 1,
        }],
        heuristics: Vec::new(),
        tool_reliability: Vec::new(),
        redacted: true,
    };
    let line = retrieval.json_line();
    let digest = extract_json_string_field(&line, "evidence_digest").unwrap();
    let line = line.replace(
        &format!("\"evidence_digest\":\"{digest}\""),
        "\"evidence_digest\":\"fnv64:not-a-digest\"",
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("evidence_digest must be stable fnv64")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_source_quarantine_raw_payload() {
    let report = SelfEvolvingMemorySourceQuarantineReport {
        source_case_digest: "fnv64:1111111111111111".to_owned(),
        reason_code: "context_polluted".to_owned(),
        deactivated_episodes: 1,
        quarantined_heuristics: 1,
        removed_tool_observations: 1,
        tool_reliability_before: 2,
        tool_reliability_after: 1,
        redacted: true,
        applied: true,
    };
    let line = report
        .json_line(
            true,
            Some("fnv64:2222222222222222"),
            Some("fnv64:2222222222222222"),
        )
        .replace(
            "\"evidence_digest\"",
            "\"solution_path\":\"private raw payload\",\"evidence_digest\"",
        );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("raw memory payloads")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_source_quarantine_malformed_digest_fields() {
    let report = SelfEvolvingMemorySourceQuarantineReport {
        source_case_digest: "fnv64:not-a-digest".to_owned(),
        reason_code: "context_polluted".to_owned(),
        deactivated_episodes: 1,
        quarantined_heuristics: 1,
        removed_tool_observations: 1,
        tool_reliability_before: 2,
        tool_reliability_after: 1,
        redacted: true,
        applied: true,
    };
    let line = report.json_line(true, Some("fnv64:not-a-digest"), Some("fnv64:not-a-digest"));
    let digest = extract_json_string_field(&line, "evidence_digest").unwrap();
    let line = line.replace(
        &format!("\"evidence_digest\":\"{digest}\""),
        "\"evidence_digest\":\"fnv64:not-a-digest\"",
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("source_case_digest must be stable fnv64")),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("evidence_digest must be stable fnv64")),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("snapshot_digest must be stable fnv64")),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("disk_snapshot_digest must be stable fnv64")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_source_quarantine_disk_readback_mismatch() {
    let report = SelfEvolvingMemorySourceQuarantineReport {
        source_case_digest: "fnv64:1111111111111111".to_owned(),
        reason_code: "context_polluted".to_owned(),
        deactivated_episodes: 1,
        quarantined_heuristics: 1,
        removed_tool_observations: 1,
        tool_reliability_before: 2,
        tool_reliability_after: 1,
        redacted: true,
        applied: true,
    };
    let line = report.json_line(
        true,
        Some("fnv64:2222222222222222"),
        Some("fnv64:3333333333333333"),
    );

    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("disk_snapshot_digest must match snapshot_digest")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_jsonl_gate_aggregates_self_evolving_memory_writebacks() {
    let path = temp_path("trace-schema-self-evolving-memory-writeback");
    let writeback = SelfEvolvingMemoryRuntimeWritebackReport {
        operation: "runtime_writeback",
        tool_name: "dispatch".to_owned(),
        profile: crate::TaskProfile::Coding,
        experience_id: 7,
        source_case_digest: "fnv64:3333333333333333".to_owned(),
        attempted_records: 3,
        accepted_records: 3,
        records_before: 2,
        records_after: 6,
        episodes_after: 2,
        active_episodes_after: 1,
        heuristics_after: 1,
        tool_reliability_after: 1,
        tool_observations_after: 2,
        maintenance_actions: 1,
        merged_duplicate_episodes: 1,
        redacted: true,
        write_allowed: true,
        durable_write_allowed: true,
        applied: true,
        applied_to_disk: true,
        snapshot_before_digest: "fnv64:1111111111111111".to_owned(),
        snapshot_digest: "fnv64:2222222222222222".to_owned(),
        disk_snapshot_digest: "fnv64:2222222222222222".to_owned(),
    };
    let line = writeback.json_line();
    fs::write(&path, format!("{line}\n")).unwrap();

    let failures = evaluate_trace_schema_line(&line);
    let report = evaluate_trace_schema_jsonl(&path).unwrap();
    let health = report.operator_health_snapshot();
    let memory = health.section("memory").unwrap();
    let health_json = health.json_line();

    assert!(failures.is_empty(), "{failures:?}");
    assert!(line.contains("\"snapshot_before_digest\":\"fnv64:"));
    assert!(line.contains("\"snapshot_digest\":\"fnv64:"));
    assert!(line.contains("\"disk_snapshot_digest\":\"fnv64:"));
    assert!(line.contains("\"source_case_digest\":\"fnv64:"));
    assert!(report.passed, "{:?}", report.failures);
    assert_eq!(report.checked_lines, 1);
    assert_eq!(report.self_evolving_memory_writeback_events, 1);
    assert_eq!(report.self_evolving_memory_writeback_source_case_digests, 1);
    assert_eq!(report.self_evolving_memory_writeback_attempted_records, 3);
    assert_eq!(report.self_evolving_memory_writeback_accepted_records, 3);
    assert_eq!(report.self_evolving_memory_writeback_records_before, 2);
    assert_eq!(report.self_evolving_memory_writeback_records_after, 6);
    assert_eq!(
        report.self_evolving_memory_writeback_tool_reliability_after,
        1
    );
    assert_eq!(
        report.self_evolving_memory_writeback_tool_observations_after,
        2
    );
    assert_eq!(report.self_evolving_memory_writeback_maintenance_actions, 1);
    assert_eq!(
        report.self_evolving_memory_writeback_merged_duplicate_episodes,
        1
    );
    assert_eq!(report.self_evolving_memory_writeback_write_allowed, 1);
    assert_eq!(
        report.self_evolving_memory_writeback_durable_write_allowed,
        1
    );
    assert_eq!(report.self_evolving_memory_writeback_applied, 1);
    assert_eq!(report.self_evolving_memory_writeback_applied_to_disk, 1);
    assert_eq!(report.self_evolving_memory_writeback_snapshot_changes, 1);
    assert_eq!(memory.metric("self_evolving_writeback_events"), Some(1));
    assert_eq!(
        memory.metric("self_evolving_writeback_applied_to_disk"),
        Some(1)
    );
    assert_eq!(
        memory.metric("self_evolving_writeback_snapshot_changes"),
        Some(1)
    );
    assert!(health_json.contains("\"self_evolving_writeback_events\":1"));
    assert!(health_json.contains("\"self_evolving_writeback_applied_to_disk\":1"));
    assert!(report
        .summary_line()
        .contains("self_evolving_memory_writeback_records_before=2"));
    assert!(report
        .summary_line()
        .contains("self_evolving_memory_writeback_events=1"));
    assert!(report
        .summary_line()
        .contains("self_evolving_memory_writeback_source_case_digests=1"));
    assert!(report
        .summary_line()
        .contains("self_evolving_memory_writeback_snapshot_changes=1"));
    cleanup(path);
}

#[test]
fn trace_schema_gate_rejects_writeback_malformed_digest_fields() {
    let path = temp_path("trace-schema-self-evolving-memory-writeback-malformed-digest");
    let writeback = SelfEvolvingMemoryRuntimeWritebackReport {
        operation: "runtime_writeback",
        tool_name: "dispatch".to_owned(),
        profile: crate::TaskProfile::Coding,
        experience_id: 7,
        source_case_digest: "fnv64:not-a-digest".to_owned(),
        attempted_records: 3,
        accepted_records: 3,
        records_before: 2,
        records_after: 6,
        episodes_after: 2,
        active_episodes_after: 1,
        heuristics_after: 1,
        tool_reliability_after: 1,
        tool_observations_after: 2,
        maintenance_actions: 1,
        merged_duplicate_episodes: 1,
        redacted: true,
        write_allowed: true,
        durable_write_allowed: true,
        applied: true,
        applied_to_disk: true,
        snapshot_before_digest: "fnv64:short".to_owned(),
        snapshot_digest: "fnv64:zzzzzzzzzzzzzzzz".to_owned(),
        disk_snapshot_digest: "fnv64:zzzzzzzzzzzzzzzz".to_owned(),
    };
    let line = writeback.json_line();
    let digest = extract_json_string_field(&line, "evidence_digest").unwrap();
    let line = line.replace(
        &format!("\"evidence_digest\":\"{digest}\""),
        "\"evidence_digest\":\"fnv64:not-a-digest\"",
    );
    fs::write(&path, format!("{line}\n")).unwrap();

    let failures = evaluate_trace_schema_line(&line);
    let report = evaluate_trace_schema_jsonl(&path).unwrap();

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("source_case_digest must be stable fnv64")),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("snapshot_before_digest must be stable fnv64")),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("snapshot_digest must be stable fnv64")),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("disk_snapshot_digest must be stable fnv64")),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("evidence_digest must be stable fnv64")),
        "{failures:?}"
    );
    assert!(!report.passed);
    assert_eq!(report.self_evolving_memory_writeback_source_case_digests, 0);
    cleanup(path);
}

#[test]
fn trace_schema_gate_rejects_writeback_without_snapshot_change() {
    let writeback = SelfEvolvingMemoryRuntimeWritebackReport {
        operation: "runtime_writeback",
        tool_name: "dispatch".to_owned(),
        profile: crate::TaskProfile::Coding,
        experience_id: 7,
        source_case_digest: "fnv64:3333333333333333".to_owned(),
        attempted_records: 3,
        accepted_records: 3,
        records_before: 1,
        records_after: 5,
        episodes_after: 2,
        active_episodes_after: 1,
        heuristics_after: 1,
        tool_reliability_after: 1,
        tool_observations_after: 1,
        maintenance_actions: 0,
        merged_duplicate_episodes: 0,
        redacted: true,
        write_allowed: true,
        durable_write_allowed: true,
        applied: true,
        applied_to_disk: true,
        snapshot_before_digest: "fnv64:2222222222222222".to_owned(),
        snapshot_digest: "fnv64:2222222222222222".to_owned(),
        disk_snapshot_digest: "fnv64:2222222222222222".to_owned(),
    };

    let failures = evaluate_trace_schema_line(&writeback.json_line());

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("snapshot digest must change")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_writeback_disk_readback_mismatch() {
    let writeback = SelfEvolvingMemoryRuntimeWritebackReport {
        operation: "runtime_writeback",
        tool_name: "dispatch".to_owned(),
        profile: crate::TaskProfile::Coding,
        experience_id: 7,
        source_case_digest: "fnv64:3333333333333333".to_owned(),
        attempted_records: 3,
        accepted_records: 3,
        records_before: 1,
        records_after: 5,
        episodes_after: 2,
        active_episodes_after: 1,
        heuristics_after: 1,
        tool_reliability_after: 1,
        tool_observations_after: 1,
        maintenance_actions: 0,
        merged_duplicate_episodes: 0,
        redacted: true,
        write_allowed: true,
        durable_write_allowed: true,
        applied: true,
        applied_to_disk: true,
        snapshot_before_digest: "fnv64:1111111111111111".to_owned(),
        snapshot_digest: "fnv64:2222222222222222".to_owned(),
        disk_snapshot_digest: "fnv64:3333333333333333".to_owned(),
    };

    let failures = evaluate_trace_schema_line(&writeback.json_line());

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("disk_snapshot_digest must match snapshot_digest")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_writeback_without_record_growth() {
    let writeback = SelfEvolvingMemoryRuntimeWritebackReport {
        operation: "runtime_writeback",
        tool_name: "dispatch".to_owned(),
        profile: crate::TaskProfile::Coding,
        experience_id: 7,
        source_case_digest: "fnv64:3333333333333333".to_owned(),
        attempted_records: 3,
        accepted_records: 3,
        records_before: 5,
        records_after: 5,
        episodes_after: 2,
        active_episodes_after: 1,
        heuristics_after: 1,
        tool_reliability_after: 1,
        tool_observations_after: 1,
        maintenance_actions: 0,
        merged_duplicate_episodes: 0,
        redacted: true,
        write_allowed: true,
        durable_write_allowed: true,
        applied: true,
        applied_to_disk: true,
        snapshot_before_digest: "fnv64:1111111111111111".to_owned(),
        snapshot_digest: "fnv64:2222222222222222".to_owned(),
        disk_snapshot_digest: "fnv64:2222222222222222".to_owned(),
    };

    let failures = evaluate_trace_schema_line(&writeback.json_line());

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("records_after must exceed records_before")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_writeback_growth_below_accepted_records() {
    let writeback = SelfEvolvingMemoryRuntimeWritebackReport {
        operation: "runtime_writeback",
        tool_name: "dispatch".to_owned(),
        profile: crate::TaskProfile::Coding,
        experience_id: 7,
        source_case_digest: "fnv64:3333333333333333".to_owned(),
        attempted_records: 3,
        accepted_records: 3,
        records_before: 3,
        records_after: 5,
        episodes_after: 2,
        active_episodes_after: 1,
        heuristics_after: 1,
        tool_reliability_after: 1,
        tool_observations_after: 1,
        maintenance_actions: 0,
        merged_duplicate_episodes: 0,
        redacted: true,
        write_allowed: true,
        durable_write_allowed: true,
        applied: true,
        applied_to_disk: true,
        snapshot_before_digest: "fnv64:1111111111111111".to_owned(),
        snapshot_digest: "fnv64:2222222222222222".to_owned(),
        disk_snapshot_digest: "fnv64:2222222222222222".to_owned(),
    };

    let failures = evaluate_trace_schema_line(&writeback.json_line());

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("record growth must cover accepted_records")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_writeback_without_trc_persistence() {
    let writeback = SelfEvolvingMemoryRuntimeWritebackReport {
        operation: "runtime_writeback",
        tool_name: "dispatch".to_owned(),
        profile: crate::TaskProfile::Coding,
        experience_id: 7,
        source_case_digest: "fnv64:3333333333333333".to_owned(),
        attempted_records: 2,
        accepted_records: 2,
        records_before: 0,
        records_after: 3,
        episodes_after: 2,
        active_episodes_after: 1,
        heuristics_after: 1,
        tool_reliability_after: 0,
        tool_observations_after: 0,
        maintenance_actions: 1,
        merged_duplicate_episodes: 0,
        redacted: true,
        write_allowed: true,
        durable_write_allowed: true,
        applied: true,
        applied_to_disk: true,
        snapshot_before_digest: "fnv64:1111111111111111".to_owned(),
        snapshot_digest: "fnv64:2222222222222222".to_owned(),
        disk_snapshot_digest: "fnv64:2222222222222222".to_owned(),
    };

    let failures = evaluate_trace_schema_line(&writeback.json_line());

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("tool_reliability_after must be positive")),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("tool_observations_after must be positive")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_writeback_without_episode_or_heuristic_persistence() {
    let writeback = SelfEvolvingMemoryRuntimeWritebackReport {
        operation: "runtime_writeback",
        tool_name: "dispatch".to_owned(),
        profile: crate::TaskProfile::Coding,
        experience_id: 7,
        source_case_digest: "fnv64:3333333333333333".to_owned(),
        attempted_records: 3,
        accepted_records: 3,
        records_before: 0,
        records_after: 3,
        episodes_after: 0,
        active_episodes_after: 0,
        heuristics_after: 0,
        tool_reliability_after: 1,
        tool_observations_after: 2,
        maintenance_actions: 1,
        merged_duplicate_episodes: 0,
        redacted: true,
        write_allowed: true,
        durable_write_allowed: true,
        applied: true,
        applied_to_disk: true,
        snapshot_before_digest: "fnv64:1111111111111111".to_owned(),
        snapshot_digest: "fnv64:2222222222222222".to_owned(),
        disk_snapshot_digest: "fnv64:2222222222222222".to_owned(),
    };

    let failures = evaluate_trace_schema_line(&writeback.json_line());

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("episodes_after must be positive")),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("heuristics_after must be positive")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_writeback_without_active_episode_persistence() {
    let writeback = SelfEvolvingMemoryRuntimeWritebackReport {
        operation: "runtime_writeback",
        tool_name: "dispatch".to_owned(),
        profile: crate::TaskProfile::Coding,
        experience_id: 7,
        source_case_digest: "fnv64:3333333333333333".to_owned(),
        attempted_records: 3,
        accepted_records: 3,
        records_before: 1,
        records_after: 5,
        episodes_after: 2,
        active_episodes_after: 0,
        heuristics_after: 1,
        tool_reliability_after: 1,
        tool_observations_after: 1,
        maintenance_actions: 1,
        merged_duplicate_episodes: 0,
        redacted: true,
        write_allowed: true,
        durable_write_allowed: true,
        applied: true,
        applied_to_disk: true,
        snapshot_before_digest: "fnv64:1111111111111111".to_owned(),
        snapshot_digest: "fnv64:2222222222222222".to_owned(),
        disk_snapshot_digest: "fnv64:2222222222222222".to_owned(),
    };

    let failures = evaluate_trace_schema_line(&writeback.json_line());

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("active_episodes_after must be positive")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_writeback_missing_apply_fields() {
    let writeback = SelfEvolvingMemoryRuntimeWritebackReport {
        operation: "runtime_writeback",
        tool_name: "dispatch".to_owned(),
        profile: crate::TaskProfile::Coding,
        experience_id: 7,
        source_case_digest: "fnv64:3333333333333333".to_owned(),
        attempted_records: 3,
        accepted_records: 3,
        records_before: 2,
        records_after: 6,
        episodes_after: 2,
        active_episodes_after: 1,
        heuristics_after: 1,
        tool_reliability_after: 1,
        tool_observations_after: 2,
        maintenance_actions: 1,
        merged_duplicate_episodes: 1,
        redacted: true,
        write_allowed: true,
        durable_write_allowed: true,
        applied: true,
        applied_to_disk: true,
        snapshot_before_digest: "fnv64:1111111111111111".to_owned(),
        snapshot_digest: "fnv64:2222222222222222".to_owned(),
        disk_snapshot_digest: "fnv64:2222222222222222".to_owned(),
    };
    let line = writeback.json_line();

    for (from, to, field) in [
        (
            "\"write_allowed\":true",
            "\"missing_write_allowed\":true",
            "write_allowed",
        ),
        (
            "\"durable_write_allowed\":true",
            "\"missing_durable_write_allowed\":true",
            "durable_write_allowed",
        ),
        ("\"applied\":true", "\"missing_applied\":true", "applied"),
        (
            "\"applied_to_disk\":true",
            "\"missing_applied_to_disk\":true",
            "applied_to_disk",
        ),
    ] {
        let failures = evaluate_trace_schema_line(&line.replacen(from, to, 1));

        assert!(
            failures.iter().any(|failure| failure.contains(&format!(
                "missing self_evolving_memory_writeback field {field}"
            ))),
            "{field}: {failures:?}"
        );
    }
}

#[test]
fn trace_schema_gate_rejects_partial_writeback_apply() {
    let writeback = SelfEvolvingMemoryRuntimeWritebackReport {
        operation: "runtime_writeback",
        tool_name: "dispatch".to_owned(),
        profile: crate::TaskProfile::Coding,
        experience_id: 8,
        source_case_digest: "fnv64:3333333333333333".to_owned(),
        attempted_records: 3,
        accepted_records: 2,
        records_before: 1,
        records_after: 5,
        episodes_after: 2,
        active_episodes_after: 1,
        heuristics_after: 1,
        tool_reliability_after: 1,
        tool_observations_after: 1,
        maintenance_actions: 1,
        merged_duplicate_episodes: 0,
        redacted: true,
        write_allowed: true,
        durable_write_allowed: true,
        applied: true,
        applied_to_disk: true,
        snapshot_before_digest: "fnv64:1111111111111111".to_owned(),
        snapshot_digest: "fnv64:2222222222222222".to_owned(),
        disk_snapshot_digest: "fnv64:2222222222222222".to_owned(),
    };

    let failures = evaluate_trace_schema_line(&writeback.json_line());

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("accepted_records must match attempted_records")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_writeback_maintenance_subcount_overflow() {
    let writeback = SelfEvolvingMemoryRuntimeWritebackReport {
        operation: "runtime_writeback",
        tool_name: "dispatch".to_owned(),
        profile: crate::TaskProfile::Coding,
        experience_id: 9,
        source_case_digest: "fnv64:3333333333333333".to_owned(),
        attempted_records: 3,
        accepted_records: 3,
        records_before: 1,
        records_after: 5,
        episodes_after: 2,
        active_episodes_after: 1,
        heuristics_after: 1,
        tool_reliability_after: 1,
        tool_observations_after: 1,
        maintenance_actions: 0,
        merged_duplicate_episodes: 1,
        redacted: true,
        write_allowed: true,
        durable_write_allowed: true,
        applied: true,
        applied_to_disk: true,
        snapshot_before_digest: "fnv64:1111111111111111".to_owned(),
        snapshot_digest: "fnv64:2222222222222222".to_owned(),
        disk_snapshot_digest: "fnv64:2222222222222222".to_owned(),
    };

    let failures = evaluate_trace_schema_line(&writeback.json_line());

    assert!(
        failures.iter().any(
            |failure| failure.contains("merged_duplicate_episodes exceeds maintenance_actions")
        ),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_gate_rejects_writeback_digest_mismatch() {
    let writeback = SelfEvolvingMemoryRuntimeWritebackReport {
        operation: "runtime_writeback",
        tool_name: "dispatch".to_owned(),
        profile: crate::TaskProfile::Coding,
        experience_id: 10,
        source_case_digest: "fnv64:3333333333333333".to_owned(),
        attempted_records: 3,
        accepted_records: 3,
        records_before: 1,
        records_after: 5,
        episodes_after: 2,
        active_episodes_after: 1,
        heuristics_after: 1,
        tool_reliability_after: 1,
        tool_observations_after: 1,
        maintenance_actions: 1,
        merged_duplicate_episodes: 0,
        redacted: true,
        write_allowed: true,
        durable_write_allowed: true,
        applied: true,
        applied_to_disk: true,
        snapshot_before_digest: "fnv64:1111111111111111".to_owned(),
        snapshot_digest: "fnv64:2222222222222222".to_owned(),
        disk_snapshot_digest: "fnv64:2222222222222222".to_owned(),
    };
    let line = writeback.json_line();
    let digest = extract_json_string_field(&line, "evidence_digest").unwrap();
    let tampered = line.replace(
        &format!("\"evidence_digest\":\"{digest}\""),
        "\"evidence_digest\":\"fnv64:0000000000000000\"",
    );

    let failures = evaluate_trace_schema_line(&tampered);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("evidence_digest does not match writeback fields")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_jsonl_gate_aggregates_memory_residency_plans() {
    let path = temp_path("trace-schema-memory-residency");
    let policy = MemoryResidencyPolicy {
        tenant_id: "tenant-a".to_owned(),
        max_hot: 1,
        max_warm: 2,
        ..MemoryResidencyPolicy::default()
    };
    let candidates = vec![
        MemoryResidencyCandidate::new(101, "tenant-a", "semantic")
            .with_scores(0.94, 8, 0, 18)
            .with_high_frequency_gene(true),
        MemoryResidencyCandidate::new(102, "tenant-a", "runtime_kv:l0h0:0-1")
            .with_scores(0.90, 7, 0, 18)
            .with_high_frequency_gene(true),
        MemoryResidencyCandidate::new(103, "tenant-b", "semantic")
            .with_scores(0.98, 9, 0, 18)
            .with_high_frequency_gene(true),
        MemoryResidencyCandidate::new(104, "tenant-a", "gist")
            .with_scores(0.36, 0, 0, 12)
            .with_rollback_anchor("rollback:private-anchor-must-not-leak", true),
    ];
    let plan = plan_memory_residency(&candidates, &policy, 20);
    let line = memory_residency_trace_json_line(&plan);
    fs::write(&path, format!("{line}\n")).unwrap();

    let report = evaluate_trace_schema_jsonl(&path).unwrap();

    assert!(report.passed, "{:?}", report.failures);
    assert_eq!(report.checked_lines, 1);
    assert_eq!(report.memory_residency_events, 1);
    assert_eq!(report.memory_residency_decisions, plan.decisions.len());
    assert_eq!(report.memory_residency_hot, 1);
    assert_eq!(report.memory_residency_warm, 1);
    assert_eq!(report.memory_residency_cold, 1);
    assert_eq!(report.memory_residency_quarantined, 1);
    assert_eq!(report.memory_residency_retired, 0);
    assert_eq!(
        report.memory_residency_protected_rollback_anchors,
        plan.protected_rollback_anchor_count()
    );
    assert_eq!(
        report.memory_residency_blocked_reasons,
        plan.blocked_reason_count()
    );
    assert_eq!(
        report.memory_residency_token_estimate,
        plan.total_token_estimate()
    );
    assert_eq!(report.memory_residency_write_allowed, 0);
    assert_eq!(report.memory_residency_durable_write_allowed, 0);
    assert_eq!(report.memory_residency_applied, 0);
    assert!(line.contains("\"schema\":\"rust-norion-memory-residency-plan-v1\""));
    assert!(line.contains("tenant=fnv64:"));
    assert!(!line.contains("tenant-a"));
    assert!(!line.contains("tenant-b"));
    assert!(!line.contains("rollback:private-anchor-must-not-leak"));
    assert!(report.summary_line().contains("memory_residency_events=1"));
    cleanup(path);
}

#[test]
fn trace_schema_jsonl_gate_aggregates_unified_writer_gate_reports() {
    let path = temp_path("trace-schema-unified-writer-gate");
    let preview_report = UnifiedWriterGate::new().evaluate([unified_writer_gate_ready_candidate(
        UnifiedWriterGateDomain::Memory,
        "memory:ready-for-preview",
    )]);
    let held_report = UnifiedWriterGate::new().evaluate([UnifiedWriterGateCandidate::new(
        UnifiedWriterGateDomain::Genome,
        "genome:held-for-evidence",
        [UnifiedWriterGateWriteScope::Genome],
    )
    .with_evidence(false, false, false, true, true)
    .with_operator_approval(false, false)]);
    let goal_queue_report =
        UnifiedWriterGate::new().evaluate([unified_writer_gate_ready_candidate(
            UnifiedWriterGateDomain::EvolutionGoalQueue,
            "goal-queue:ready-for-preview",
        )]);

    append_unified_writer_gate_trace_jsonl(&path, &preview_report).unwrap();
    append_unified_writer_gate_trace_jsonl(&path, &held_report).unwrap();
    append_unified_writer_gate_trace_jsonl(&path, &goal_queue_report).unwrap();

    let report = evaluate_trace_schema_jsonl(&path).unwrap();

    assert!(report.passed, "{:?}", report.failures);
    assert_eq!(report.checked_lines, 3);
    assert_eq!(report.unified_writer_gate_events, 3);
    assert_eq!(report.unified_writer_gate_records, 3);
    assert_eq!(report.unified_writer_gate_memory_records, 1);
    assert_eq!(report.unified_writer_gate_genome_records, 1);
    assert_eq!(report.unified_writer_gate_experiment_ledger_records, 0);
    assert_eq!(report.unified_writer_gate_evolution_goal_queue_records, 1);
    assert_eq!(report.unified_writer_gate_ready_records, 0);
    assert_eq!(report.unified_writer_gate_preview_only_records, 2);
    assert_eq!(report.unified_writer_gate_held_records, 1);
    assert_eq!(report.unified_writer_gate_rejected_records, 0);
    assert_eq!(report.unified_writer_gate_write_allowed, 0);
    assert_eq!(report.unified_writer_gate_durable_write_allowed, 0);
    assert_eq!(report.unified_writer_gate_applied, 0);
    assert!(report
        .summary_line()
        .contains("unified_writer_gate_events=3"));
    cleanup(path);
}

#[test]
fn trace_schema_gate_rejects_unified_writer_gate_ready_write_enabled() {
    let report = UnifiedWriterGate::new()
        .with_policy(UnifiedWriterGatePolicy {
            durable_writes_enabled: true,
            ..UnifiedWriterGatePolicy::default()
        })
        .evaluate([unified_writer_gate_ready_candidate(
            UnifiedWriterGateDomain::ExperimentLedger,
            "experiment:ready",
        )]);
    let failures = evaluate_trace_schema_line(&report.json_line());

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("ready_records require a separate explicit apply")),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("durable_write_allowed=true expected false")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_jsonl_gate_aggregates_self_goal_queue_apply_held_reports() {
    let path = temp_path("trace-schema-self-goal-queue-apply");
    let held = self_goal_queue_apply_report(false);

    append_self_goal_queue_apply_trace_jsonl(&path, &held).unwrap();

    let report = evaluate_trace_schema_jsonl(&path).unwrap();

    assert!(report.passed, "{:?}", report.failures);
    assert_eq!(report.checked_lines, 1);
    assert_eq!(report.self_goal_queue_apply_events, 1);
    assert_eq!(report.self_goal_queue_apply_records, held.record_count);
    assert_eq!(report.self_goal_queue_apply_ready_records, 0);
    assert_eq!(report.self_goal_queue_apply_held_records, held.held_count);
    assert_eq!(report.self_goal_queue_apply_rejected_records, 0);
    assert!(
        report.self_goal_queue_apply_reason_codes >= held.held_count,
        "{}",
        report.summary_line()
    );
    assert_eq!(report.self_goal_queue_apply_explicit_apply_required, 0);
    assert_eq!(report.self_goal_queue_apply_write_allowed, 0);
    assert_eq!(report.self_goal_queue_apply_applied, 0);
    assert!(report
        .summary_line()
        .contains("self_goal_queue_apply_events=1"));
    cleanup(path);
}

#[test]
fn trace_schema_gate_accepts_self_goal_queue_apply_ready_after_executor_exists() {
    let ready = self_goal_queue_apply_report(true);
    let failures = evaluate_trace_schema_line(&ready.json_line());

    assert_eq!(ready.ready_count, 1);
    assert!(ready.explicit_apply_required);
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn trace_schema_gate_rejects_self_goal_queue_apply_write_allowed_trace() {
    let held = self_goal_queue_apply_report(false);
    let line = held
        .json_line()
        .replacen("\"write_allowed\":false", "\"write_allowed\":true", 1);
    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("write_allowed=true expected false")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_jsonl_gate_accepts_self_goal_queue_continuation_report() {
    let path = temp_path("trace-schema-self-goal-queue-continuation");
    let ready_line = self_goal_queue_continuation_line();
    let held_line = ready_line
        .replacen(
            "\"source\":\"completion_resulting_queue\"",
            "\"source\":\"current_queue\"",
            1,
        )
        .replacen("\"ready\":true", "\"ready\":false", 1)
        .replacen("\"goals\":1", "\"goals\":0", 1)
        .replacen("\"active\":true", "\"active\":false", 1)
        .replacen(
            "\"active_goal_id\":\"redaction-digest:goal\"",
            "\"active_goal_id\":\"none\"",
            1,
        )
        .replacen("\"required_evidence_count\":4", "\"required_evidence_count\":0", 1)
        .replacen(
            "\"required_evidence\":[\"cargo_check\",\"benchmark_gate\",\"trace_schema_gate\",\"operator_approval\"]",
            "\"required_evidence\":[]",
            1,
        )
        .replacen("ready=true goals=1", "ready=false goals=0", 1);

    fs::write(&path, format!("{ready_line}\n{held_line}\n")).unwrap();
    let gate = evaluate_trace_schema_jsonl(&path).unwrap();
    let self_goal_queue = gate
        .operator_health_snapshot()
        .section("self_goal_queue")
        .unwrap()
        .clone();

    assert!(gate.passed, "{:?}", gate.failures);
    assert_eq!(gate.checked_lines, 2);
    assert_eq!(gate.self_goal_queue_continuation_events, 2);
    assert_eq!(gate.self_goal_queue_continuation_ready, 1);
    assert_eq!(gate.self_goal_queue_continuation_held, 1);
    assert_eq!(gate.self_goal_queue_continuation_current_queue, 1);
    assert_eq!(
        gate.self_goal_queue_continuation_completion_resulting_queue,
        1
    );
    assert_eq!(gate.self_goal_queue_continuation_goals, 1);
    assert_eq!(gate.self_goal_queue_continuation_required_evidence, 4);
    assert_eq!(gate.self_goal_queue_continuation_reason_codes, 4);
    assert_eq!(gate.self_goal_queue_continuation_budget_attempts, 6);
    assert_eq!(gate.self_goal_queue_continuation_budget_steps, 24);
    assert_eq!(gate.self_goal_queue_continuation_budget_tokens, 160_000);
    assert_eq!(
        gate.self_goal_queue_continuation_budget_runtime_ms,
        1_800_000
    );
    assert_eq!(gate.self_goal_queue_continuation_write_allowed, 0);
    assert_eq!(gate.self_goal_queue_continuation_applied, 0);
    assert!(gate
        .summary_line()
        .contains("self_goal_queue_continuation_ready=1"));
    assert!(gate
        .summary_line()
        .contains("self_goal_queue_continuation_held=1"));
    assert!(self_goal_queue.data_present);
    assert!(self_goal_queue.review_required);
    assert!(!self_goal_queue.blocked);
    assert_eq!(
        self_goal_queue.metric("continuation_events"),
        Some(gate.self_goal_queue_continuation_events)
    );
    assert_eq!(
        self_goal_queue.metric("continuation_required_evidence"),
        Some(gate.self_goal_queue_continuation_required_evidence)
    );
    assert!(gate
        .operator_health_json()
        .contains("\"name\":\"self_goal_queue\""));
    cleanup(path);
}

#[test]
fn trace_schema_gate_rejects_self_goal_queue_continuation_write_allowed_trace() {
    let line = self_goal_queue_continuation_line().replacen(
        "\"write_allowed\":false",
        "\"write_allowed\":true",
        1,
    );
    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("write_allowed=true expected false")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_jsonl_gate_aggregates_self_goal_queue_evidence_plan_report() {
    let path = temp_path("trace-schema-self-goal-queue-evidence-plan");
    let line = self_goal_queue_evidence_plan_line();

    fs::write(&path, format!("{line}\n")).unwrap();
    let gate = evaluate_trace_schema_jsonl(&path).unwrap();
    let self_goal_queue = gate
        .operator_health_snapshot()
        .section("self_goal_queue")
        .unwrap()
        .clone();

    assert!(gate.passed, "{:?}", gate.failures);
    assert_eq!(gate.checked_lines, 1);
    assert_eq!(gate.self_goal_queue_evidence_plan_events, 1);
    assert_eq!(gate.self_goal_queue_evidence_plan_ready, 1);
    assert_eq!(gate.self_goal_queue_evidence_plan_held, 0);
    assert_eq!(gate.self_goal_queue_evidence_plan_steps, 4);
    assert_eq!(gate.self_goal_queue_evidence_plan_auto_collectible, 3);
    assert_eq!(gate.self_goal_queue_evidence_plan_manual, 1);
    assert_eq!(gate.self_goal_queue_evidence_plan_required_evidence, 4);
    assert_eq!(gate.self_goal_queue_evidence_plan_packet_templates, 4);
    assert_eq!(gate.self_goal_queue_evidence_plan_command_templates, 4);
    assert_eq!(gate.self_goal_queue_evidence_plan_write_allowed, 0);
    assert_eq!(gate.self_goal_queue_evidence_plan_applied, 0);
    assert!(gate
        .summary_line()
        .contains("self_goal_queue_evidence_plan_events=1"));
    assert_eq!(
        self_goal_queue.metric("evidence_plan_steps"),
        Some(gate.self_goal_queue_evidence_plan_steps)
    );
    assert_eq!(
        self_goal_queue.metric("evidence_plan_manual"),
        Some(gate.self_goal_queue_evidence_plan_manual)
    );
    cleanup(path);
}

#[test]
fn trace_schema_gate_rejects_self_goal_queue_evidence_plan_command_leak() {
    let line = self_goal_queue_evidence_plan_line().replacen(
        "\"command_digests\":[",
        "\"commands\":[\"cargo check\"],\"command_digests\":[",
        1,
    );
    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures.iter().any(|failure| {
            failure.contains("self_goal_queue_evidence_plan must expose plan counts/digests only")
        }),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_jsonl_gate_aggregates_self_goal_queue_evidence_collection_report() {
    let path = temp_path("trace-schema-self-goal-queue-evidence-collection");
    let line = self_goal_queue_evidence_collection_line();

    fs::write(&path, format!("{line}\n")).unwrap();
    let gate = evaluate_trace_schema_jsonl(&path).unwrap();
    let self_goal_queue = gate
        .operator_health_snapshot()
        .section("self_goal_queue")
        .unwrap()
        .clone();

    assert!(gate.passed, "{:?}", gate.failures);
    assert_eq!(gate.checked_lines, 1);
    assert_eq!(gate.self_goal_queue_evidence_collection_events, 1);
    assert_eq!(gate.self_goal_queue_evidence_collection_ready, 1);
    assert_eq!(gate.self_goal_queue_evidence_collection_complete, 0);
    assert_eq!(gate.self_goal_queue_evidence_collection_steps, 4);
    assert_eq!(gate.self_goal_queue_evidence_collection_collected, 2);
    assert_eq!(gate.self_goal_queue_evidence_collection_passed, 1);
    assert_eq!(gate.self_goal_queue_evidence_collection_failed, 1);
    assert_eq!(gate.self_goal_queue_evidence_collection_missing, 1);
    assert_eq!(gate.self_goal_queue_evidence_collection_manual_missing, 1);
    assert_eq!(gate.self_goal_queue_evidence_collection_write_allowed, 0);
    assert_eq!(gate.self_goal_queue_evidence_collection_applied, 0);
    assert!(gate
        .summary_line()
        .contains("self_goal_queue_evidence_collection_events=1"));
    assert_eq!(
        self_goal_queue.metric("evidence_collection_passed"),
        Some(gate.self_goal_queue_evidence_collection_passed)
    );
    assert_eq!(
        self_goal_queue.metric("evidence_collection_manual_missing"),
        Some(gate.self_goal_queue_evidence_collection_manual_missing)
    );
    cleanup(path);
}

#[test]
fn trace_schema_gate_rejects_self_goal_queue_evidence_collection_command_leak() {
    let line = self_goal_queue_evidence_collection_line().replacen(
        "\"collection_packet_digests\":[",
        "\"commands\":[\"cargo check\"],\"collection_packet_digests\":[",
        1,
    );
    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures.iter().any(|failure| {
            failure.contains(
                "self_goal_queue_evidence_collection must expose collection counts/digests only",
            )
        }),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_jsonl_gate_accepts_self_goal_queue_append_execution_report() {
    let path = temp_path("trace-schema-self-goal-queue-append-execution");
    let report = self_goal_queue_append_execution_report();

    append_self_goal_queue_append_execution_trace_jsonl(&path, &report).unwrap();

    let gate = evaluate_trace_schema_jsonl(&path).unwrap();

    assert!(report.passed(), "{}", report.summary_line());
    assert!(gate.passed, "{:?}", gate.failures);
    assert_eq!(gate.checked_lines, 1);
    cleanup(path);
}

#[test]
fn trace_schema_gate_rejects_self_goal_queue_append_execution_durable_write() {
    let report = self_goal_queue_append_execution_report();
    let line = report.json_line().replacen(
        "\"durable_write_allowed\":false",
        "\"durable_write_allowed\":true",
        1,
    );
    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("durable_write_allowed=true expected false")),
        "{failures:?}"
    );
}

#[test]
fn trace_schema_jsonl_gate_accepts_evolution_goal_queue_store_write_report() {
    let trace_path = temp_path("trace-schema-evolution-goal-queue-store-write");
    let store_path = temp_path("evolution-goal-queue-store-write");
    let mut store = EvolutionGoalQueueDiskStore::open_with_policy(
        &store_path,
        EvolutionGoalQueueStorePolicy::explicit_durable_write(),
    )
    .unwrap();
    let scope = TenantScope::local_single_user();
    let key = scope.scoped_key(TenantResourceLane::EvolutionGoalQueue, "active");
    let append_report = self_goal_queue_append_execution_report();
    let resulting_queue = append_report
        .resulting_queue
        .as_ref()
        .expect("append executor produced queue");
    let approval = EvolutionGoalQueueStoreApproval::for_queue(
        "operator",
        "queue-store-ticket",
        &key,
        resulting_queue,
        &append_report.rollback_anchor_digest,
    );
    let write = store
        .write_append_execution_result(&scope, &key, &append_report, Some(&approval))
        .unwrap();

    append_evolution_goal_queue_store_write_trace_jsonl(&trace_path, &write).unwrap();
    let gate = evaluate_trace_schema_jsonl(&trace_path).unwrap();

    assert!(append_report.passed(), "{}", append_report.summary_line());
    assert!(write.passed(), "{}", write.summary_line());
    assert!(gate.passed, "{:?}", gate.failures);
    assert_eq!(gate.checked_lines, 1);
    assert_eq!(gate.evolution_goal_queue_store_write_events, 1);
    assert_eq!(gate.evolution_goal_queue_store_write_applied, 1);
    assert_eq!(
        gate.evolution_goal_queue_store_write_durable_write_allowed,
        1
    );
    assert_eq!(gate.evolution_goal_queue_store_write_applied_to_disk, 1);
    assert!(gate
        .summary_line()
        .contains("evolution_goal_queue_store_write_events=1"));
    cleanup(trace_path);
    cleanup(store_path);
}

#[test]
fn trace_schema_gate_rejects_evolution_goal_queue_store_write_raw_digest() {
    let store_path = temp_path("evolution-goal-queue-store-raw-digest");
    let mut store = EvolutionGoalQueueDiskStore::open_with_policy(
        &store_path,
        EvolutionGoalQueueStorePolicy::explicit_durable_write(),
    )
    .unwrap();
    let scope = TenantScope::local_single_user();
    let key = scope.scoped_key(TenantResourceLane::EvolutionGoalQueue, "active");
    let append_report = self_goal_queue_append_execution_report();
    let resulting_queue = append_report
        .resulting_queue
        .as_ref()
        .expect("append executor produced queue");
    let approval = EvolutionGoalQueueStoreApproval::for_queue(
        "operator",
        "queue-store-ticket",
        &key,
        resulting_queue,
        &append_report.rollback_anchor_digest,
    );
    let write = store
        .write_append_execution_result(&scope, &key, &append_report, Some(&approval))
        .unwrap();
    let line = write.json_line().replacen(
        "\"queue_digest\":\"redaction-digest:",
        "\"queue_digest\":\"raw-queue:",
        1,
    );
    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("queue_digest must be redaction digest")),
        "{failures:?}"
    );
    cleanup(store_path);
}

#[test]
fn trace_schema_jsonl_gate_exports_redacted_operator_health_snapshot() {
    let path = temp_path("trace-schema-operator-health");
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let prompt = "operator health private prompt sk-test-private-operator-health";
    let outcome = engine.infer(
        InferenceRequest::new(prompt, TaskProfile::Coding),
        &mut backend,
    );
    let line = trace_json_line(prompt, TaskProfile::Coding, 9, &outcome);
    let memory_object = json_object_after_field(&line, "memory").unwrap();
    let expected_memory_runtime_kv_exported =
        extract_json_usize_field(memory_object, "runtime_kv_exported").unwrap();
    let expected_memory_runtime_kv_stored =
        extract_json_usize_field(memory_object, "runtime_kv_stored").unwrap();
    let expected_memory_runtime_kv_hold =
        usize::from(extract_json_bool_field(memory_object, "runtime_kv_hold").unwrap_or(false));
    let expected_memory_runtime_kv_held =
        extract_json_usize_field(memory_object, "runtime_kv_held").unwrap();
    let expected_memory_feedback_reinforced =
        extract_json_usize_field(memory_object, "feedback_reinforced").unwrap();
    let expected_memory_feedback_penalized =
        extract_json_usize_field(memory_object, "feedback_penalized").unwrap();
    let expected_memory_feedback_reinforcement_milli = trace_milli(
        extract_json_f32_field(memory_object, "feedback_reinforcement_amount").unwrap(),
    );
    let expected_memory_feedback_penalty_milli =
        trace_milli(extract_json_f32_field(memory_object, "feedback_penalty_amount").unwrap());
    let expected_memory_feedback_updates =
        extract_json_usize_field(memory_object, "feedback_updates").unwrap();
    let expected_memory_feedback_applied =
        extract_json_usize_field(memory_object, "feedback_applied").unwrap();
    let expected_memory_feedback_removed =
        extract_json_usize_field(memory_object, "feedback_removed").unwrap();
    let expected_memory_feedback_missing =
        extract_json_usize_field(memory_object, "feedback_missing").unwrap();
    let expected_memory_feedback_strength_delta_milli =
        trace_milli(extract_json_f32_field(memory_object, "feedback_strength_delta").unwrap());
    let live_evolution = json_object_after_field(&line, "live_evolution").unwrap();
    let expected_live_memory_reinforcements =
        extract_json_usize_field(live_evolution, "live_memory_reinforcements").unwrap();
    let expected_live_memory_penalties =
        extract_json_usize_field(live_evolution, "live_memory_penalties").unwrap();
    let expected_live_stored_memories =
        usize::from(extract_json_bool_field(live_evolution, "live_stored_memory").unwrap());
    let expected_live_stored_gist_memories =
        extract_json_usize_field(live_evolution, "live_stored_gist_memories").unwrap();
    let expected_live_stored_runtime_kv_memories =
        extract_json_usize_field(live_evolution, "live_stored_runtime_kv_memories").unwrap();
    let evolution_ledger = json_object_after_field(&line, "evolution_ledger").unwrap();
    let expected_evolution_live_inference_runs =
        extract_json_usize_field(evolution_ledger, "live_inference_runs").unwrap();
    let expected_evolution_live_memory_reinforcements =
        extract_json_usize_field(evolution_ledger, "cumulative_live_memory_reinforcements")
            .unwrap();
    let expected_evolution_live_router_threshold_mutations = extract_json_usize_field(
        evolution_ledger,
        "cumulative_live_router_threshold_mutations",
    )
    .unwrap();
    let expected_evolution_live_hierarchy_weight_mutations = extract_json_usize_field(
        evolution_ledger,
        "cumulative_live_hierarchy_weight_mutations",
    )
    .unwrap();
    let expected_evolution_live_router_threshold_delta_milli = trace_milli(
        extract_json_f32_field(evolution_ledger, "cumulative_live_router_threshold_delta").unwrap(),
    );
    let expected_evolution_live_hierarchy_weight_delta_milli = trace_milli(
        extract_json_f32_field(evolution_ledger, "cumulative_live_hierarchy_weight_delta").unwrap(),
    );
    let expected_evolution_live_online_reward_feedbacks =
        extract_json_usize_field(evolution_ledger, "cumulative_live_online_reward_feedbacks")
            .unwrap();
    let expected_evolution_live_online_reward_reinforcements = extract_json_usize_field(
        evolution_ledger,
        "cumulative_live_online_reward_reinforcements",
    )
    .unwrap();
    let expected_evolution_live_online_reward_penalties =
        extract_json_usize_field(evolution_ledger, "cumulative_live_online_reward_penalties")
            .unwrap();
    let expected_evolution_live_online_reward_strength_milli = trace_milli(
        extract_json_f32_field(evolution_ledger, "cumulative_live_online_reward_strength").unwrap(),
    );
    let expected_evolution_live_online_reward_reinforcement_strength_milli = trace_milli(
        extract_json_f32_field(
            evolution_ledger,
            "cumulative_live_online_reward_reinforcement_strength",
        )
        .unwrap(),
    );
    let expected_evolution_live_online_reward_penalty_strength_milli = trace_milli(
        extract_json_f32_field(
            evolution_ledger,
            "cumulative_live_online_reward_penalty_strength",
        )
        .unwrap(),
    );
    let expected_evolution_live_memory_penalties =
        extract_json_usize_field(evolution_ledger, "cumulative_live_memory_penalties").unwrap();
    let expected_evolution_live_memory_updates =
        extract_json_usize_field(evolution_ledger, "cumulative_live_memory_updates").unwrap();
    let expected_evolution_live_stored_memories =
        extract_json_usize_field(evolution_ledger, "cumulative_live_stored_memories").unwrap();
    let expected_evolution_live_stored_gist_memories =
        extract_json_usize_field(evolution_ledger, "cumulative_live_stored_gist_memories").unwrap();
    let expected_evolution_live_stored_runtime_kv_memories = extract_json_usize_field(
        evolution_ledger,
        "cumulative_live_stored_runtime_kv_memories",
    )
    .unwrap();
    let expected_evolution_live_stored_memory_updates =
        extract_json_usize_field(evolution_ledger, "cumulative_live_stored_memory_updates")
            .unwrap();
    let expected_evolution_live_reflection_issues =
        extract_json_usize_field(evolution_ledger, "cumulative_live_reflection_issues").unwrap();
    let expected_evolution_live_critical_reflection_issues = extract_json_usize_field(
        evolution_ledger,
        "cumulative_live_critical_reflection_issues",
    )
    .unwrap();
    let expected_evolution_live_revision_actions =
        extract_json_usize_field(evolution_ledger, "cumulative_live_revision_actions").unwrap();
    fs::write(&path, format!("{line}\n")).unwrap();

    let report = evaluate_trace_schema_jsonl(&path).unwrap();
    let snapshot = report.operator_health_snapshot();
    let json = report.operator_health_json();

    assert!(report.passed, "{:?}", report.failures);
    assert_eq!(snapshot.schema, OPERATOR_HEALTH_SCHEMA);
    assert_eq!(snapshot.checked_lines, 1);
    assert_eq!(snapshot.failure_count, 0);
    assert_eq!(snapshot.trace_ids, report.trace_experience_ids);
    assert!(snapshot.passed);

    let trace_gate = snapshot.section("trace_gate").unwrap();
    assert_eq!(trace_gate.status(), "ready");
    assert_eq!(trace_gate.metric("checked_lines"), Some(1));
    assert_eq!(
        trace_gate.metric("trace_id_count"),
        Some(snapshot.trace_ids.len())
    );

    let memory = snapshot.section("memory").unwrap();
    assert!(memory.data_present);
    assert_eq!(
        report.memory_runtime_kv_exported,
        expected_memory_runtime_kv_exported
    );
    assert_eq!(
        report.memory_runtime_kv_stored,
        expected_memory_runtime_kv_stored
    );
    assert_eq!(
        report.memory_runtime_kv_hold,
        expected_memory_runtime_kv_hold
    );
    assert_eq!(
        report.memory_runtime_kv_held,
        expected_memory_runtime_kv_held
    );
    assert_eq!(
        report.memory_feedback_reinforced,
        expected_memory_feedback_reinforced
    );
    assert_eq!(
        report.memory_feedback_penalized,
        expected_memory_feedback_penalized
    );
    assert_eq!(
        report.memory_feedback_reinforcement_milli,
        expected_memory_feedback_reinforcement_milli
    );
    assert_eq!(
        report.memory_feedback_penalty_milli,
        expected_memory_feedback_penalty_milli
    );
    assert_eq!(
        report.memory_feedback_updates,
        expected_memory_feedback_updates
    );
    assert_eq!(
        report.memory_feedback_applied,
        expected_memory_feedback_applied
    );
    assert_eq!(
        report.memory_feedback_removed,
        expected_memory_feedback_removed
    );
    assert_eq!(
        report.memory_feedback_missing,
        expected_memory_feedback_missing
    );
    assert_eq!(
        report.memory_feedback_strength_delta_milli,
        expected_memory_feedback_strength_delta_milli
    );
    assert!(report
        .summary_line()
        .contains("memory_runtime_kv_exported="));
    assert!(report
        .summary_line()
        .contains("memory_feedback_strength_delta_milli="));
    assert_eq!(
        memory.metric("admission_events"),
        Some(report.memory_admission_events)
    );
    assert_eq!(
        memory.metric("runtime_kv_exported"),
        Some(report.memory_runtime_kv_exported)
    );
    assert_eq!(
        memory.metric("runtime_kv_stored"),
        Some(report.memory_runtime_kv_stored)
    );
    assert_eq!(
        memory.metric("runtime_kv_hold"),
        Some(report.memory_runtime_kv_hold)
    );
    assert_eq!(
        memory.metric("runtime_kv_held"),
        Some(report.memory_runtime_kv_held)
    );
    assert_eq!(
        memory.metric("feedback_reinforced"),
        Some(report.memory_feedback_reinforced)
    );
    assert_eq!(
        memory.metric("feedback_penalized"),
        Some(report.memory_feedback_penalized)
    );
    assert_eq!(
        memory.metric("feedback_reinforcement_milli"),
        Some(report.memory_feedback_reinforcement_milli)
    );
    assert_eq!(
        memory.metric("feedback_penalty_milli"),
        Some(report.memory_feedback_penalty_milli)
    );
    assert_eq!(
        memory.metric("feedback_updates"),
        Some(report.memory_feedback_updates)
    );
    assert_eq!(
        memory.metric("feedback_applied"),
        Some(report.memory_feedback_applied)
    );
    assert_eq!(
        memory.metric("feedback_removed"),
        Some(report.memory_feedback_removed)
    );
    assert_eq!(
        memory.metric("feedback_missing"),
        Some(report.memory_feedback_missing)
    );
    assert_eq!(
        memory.metric("feedback_strength_delta_milli"),
        Some(report.memory_feedback_strength_delta_milli)
    );
    assert_eq!(
        memory.metric("kv_fusion_events"),
        Some(report.kv_fusion_events)
    );
    assert_eq!(
        memory.metric("kv_fusion_candidates"),
        Some(report.kv_fusion_candidates)
    );
    assert_eq!(
        memory.metric("kv_fusion_fused"),
        Some(report.kv_fusion_fused)
    );
    assert_eq!(
        memory.metric("kv_fusion_compressed"),
        Some(report.kv_fusion_compressed)
    );
    assert_eq!(
        memory.metric("kv_fusion_skipped"),
        Some(report.kv_fusion_skipped)
    );
    assert_eq!(memory.metric("kv_fusion_held"), Some(report.kv_fusion_held));
    assert_eq!(
        memory.metric("kv_fusion_rejected"),
        Some(report.kv_fusion_rejected)
    );
    assert_eq!(
        memory.metric("kv_fusion_input_tokens"),
        Some(report.kv_fusion_input_tokens)
    );
    assert_eq!(
        memory.metric("kv_fusion_retained_tokens"),
        Some(report.kv_fusion_retained_tokens)
    );
    assert_eq!(
        memory.metric("kv_fusion_saved_tokens"),
        Some(report.kv_fusion_saved_tokens)
    );
    assert_eq!(
        memory.metric("kv_fusion_approval_blocked"),
        Some(report.kv_fusion_approval_blocked)
    );
    assert_eq!(
        memory.metric("self_evolving_store_saved_tokens"),
        Some(report.self_evolving_memory_store_saved_tokens)
    );
    assert_eq!(
        memory.metric("self_evolving_store_retrieval_events"),
        Some(report.self_evolving_memory_store_retrieval_events)
    );
    assert_eq!(
        memory.metric("self_evolving_store_maintenance_events"),
        Some(report.self_evolving_memory_store_maintenance_events)
    );
    assert_eq!(
        memory.metric("self_evolving_store_admission_preview_events"),
        Some(report.self_evolving_memory_store_admission_preview_events)
    );
    assert_eq!(
        memory.metric("self_evolving_store_contexts"),
        Some(report.self_evolving_memory_store_contexts)
    );
    assert_eq!(
        memory.metric("self_evolving_store_maintenance_actions"),
        Some(report.self_evolving_memory_store_maintenance_actions)
    );
    assert_eq!(
        memory.metric("self_evolving_store_admission_candidates"),
        Some(report.self_evolving_memory_store_admission_candidates)
    );
    assert_eq!(
        memory.metric("self_evolving_store_write_allowed"),
        Some(report.self_evolving_memory_store_write_allowed)
    );
    assert_eq!(
        memory.metric("self_evolving_store_durable_write_allowed"),
        Some(report.self_evolving_memory_store_durable_write_allowed)
    );
    assert_eq!(
        memory.metric("self_evolving_store_applied"),
        Some(report.self_evolving_memory_store_applied)
    );
    assert_eq!(
        memory.metric("self_evolving_store_applied_to_disk"),
        Some(report.self_evolving_memory_store_applied_to_disk)
    );
    assert_eq!(
        memory.metric("self_evolving_consolidation_events"),
        Some(report.self_evolving_memory_store_consolidation_events)
    );
    assert_eq!(
        memory.metric("self_evolving_consolidation_actions"),
        Some(report.self_evolving_memory_store_consolidation_actions)
    );
    assert_eq!(
        memory.metric("self_evolving_merge_previews"),
        Some(report.self_evolving_memory_store_merge_previews)
    );
    assert_eq!(
        memory.metric("self_evolving_decay_previews"),
        Some(report.self_evolving_memory_store_decay_previews)
    );
    assert_eq!(
        memory.metric("self_evolving_tombstone_previews"),
        Some(report.self_evolving_memory_store_tombstone_previews)
    );
    assert_eq!(
        memory.metric("self_evolving_merge_rejections"),
        Some(report.self_evolving_memory_store_merge_rejections)
    );
    assert_eq!(
        memory.metric("residency_events"),
        Some(report.memory_residency_events)
    );
    assert_eq!(
        memory.metric("residency_decisions"),
        Some(report.memory_residency_decisions)
    );
    assert_eq!(
        memory.metric("residency_hot"),
        Some(report.memory_residency_hot)
    );
    assert_eq!(
        memory.metric("residency_warm"),
        Some(report.memory_residency_warm)
    );
    assert_eq!(
        memory.metric("residency_cold"),
        Some(report.memory_residency_cold)
    );
    assert_eq!(
        memory.metric("residency_quarantined"),
        Some(report.memory_residency_quarantined)
    );
    assert_eq!(
        memory.metric("residency_retired"),
        Some(report.memory_residency_retired)
    );
    assert_eq!(
        memory.metric("residency_protected_rollback_anchors"),
        Some(report.memory_residency_protected_rollback_anchors)
    );
    assert_eq!(
        memory.metric("residency_blocked_reasons"),
        Some(report.memory_residency_blocked_reasons)
    );
    assert_eq!(
        memory.metric("residency_token_estimate"),
        Some(report.memory_residency_token_estimate)
    );
    assert_eq!(
        memory.metric("residency_write_allowed"),
        Some(report.memory_residency_write_allowed)
    );
    assert_eq!(
        memory.metric("residency_durable_write_allowed"),
        Some(report.memory_residency_durable_write_allowed)
    );
    assert_eq!(
        memory.metric("residency_applied"),
        Some(report.memory_residency_applied)
    );
    assert_eq!(
        memory.metric("self_evolving_writeback_events"),
        Some(report.self_evolving_memory_writeback_events)
    );
    assert_eq!(
        memory.metric("self_evolving_writeback_records_before"),
        Some(report.self_evolving_memory_writeback_records_before)
    );
    assert_eq!(
        memory.metric("self_evolving_writeback_source_case_digests"),
        Some(report.self_evolving_memory_writeback_source_case_digests)
    );
    assert_eq!(
        memory.metric("self_evolving_writeback_attempted_records"),
        Some(report.self_evolving_memory_writeback_attempted_records)
    );
    assert_eq!(
        memory.metric("self_evolving_writeback_accepted_records"),
        Some(report.self_evolving_memory_writeback_accepted_records)
    );
    assert_eq!(
        memory.metric("self_evolving_writeback_rejected_records"),
        Some(report.self_evolving_memory_writeback_rejected_records())
    );
    assert_eq!(
        memory.metric("self_evolving_writeback_records_after"),
        Some(report.self_evolving_memory_writeback_records_after)
    );
    assert_eq!(
        memory.metric("self_evolving_writeback_tool_reliability_after"),
        Some(report.self_evolving_memory_writeback_tool_reliability_after)
    );
    assert_eq!(
        memory.metric("self_evolving_writeback_tool_observations_after"),
        Some(report.self_evolving_memory_writeback_tool_observations_after)
    );
    assert_eq!(
        memory.metric("self_evolving_writeback_write_allowed"),
        Some(report.self_evolving_memory_writeback_write_allowed)
    );
    assert_eq!(
        memory.metric("self_evolving_writeback_durable_write_allowed"),
        Some(report.self_evolving_memory_writeback_durable_write_allowed)
    );
    assert_eq!(
        memory.metric("self_evolving_writeback_applied"),
        Some(report.self_evolving_memory_writeback_applied)
    );
    assert_eq!(
        memory.metric("self_evolving_writeback_applied_to_disk"),
        Some(report.self_evolving_memory_writeback_applied_to_disk)
    );
    assert_eq!(
        memory.metric("self_evolving_writeback_snapshot_changes"),
        Some(report.self_evolving_memory_writeback_snapshot_changes)
    );

    let genome = snapshot.section("genome").unwrap();
    assert!(genome.data_present);
    assert_eq!(
        genome.metric("events"),
        Some(report.reasoning_genome_events)
    );
    assert_eq!(genome.metric("genes"), Some(report.reasoning_genome_genes));
    assert!(report.reasoning_genome_events >= 1);
    assert!(report.reasoning_genome_genes >= 1);

    let routing = snapshot.section("routing").unwrap();
    assert!(routing.data_present);
    assert_eq!(
        routing.metric("adaptive_routing_events"),
        Some(report.adaptive_routing_events)
    );
    assert_eq!(
        routing.metric("adaptive_routing_candidates"),
        Some(report.adaptive_routing_candidates)
    );
    assert_eq!(
        routing.metric("adaptive_routing_include"),
        Some(report.adaptive_routing_include)
    );
    assert_eq!(
        routing.metric("adaptive_routing_compress"),
        Some(report.adaptive_routing_compress)
    );
    assert_eq!(
        routing.metric("adaptive_routing_defer"),
        Some(report.adaptive_routing_defer)
    );
    assert_eq!(
        routing.metric("adaptive_routing_skip"),
        Some(report.adaptive_routing_skip)
    );
    assert_eq!(
        routing.metric("adaptive_routing_input_tokens"),
        Some(report.adaptive_routing_input_tokens)
    );
    assert_eq!(
        routing.metric("adaptive_routing_retained_tokens"),
        Some(report.adaptive_routing_retained_tokens)
    );
    assert_eq!(
        routing.metric("adaptive_routing_saved_tokens"),
        Some(report.adaptive_routing_saved_tokens)
    );
    assert_eq!(
        routing.metric("task_hierarchy_route_pressure_milli"),
        Some(report.task_hierarchy_route_pressure_milli)
    );
    assert_eq!(
        routing.metric("task_hierarchy_compute_reduction_milli"),
        Some(report.task_hierarchy_compute_reduction_milli)
    );
    assert_eq!(
        routing.metric("task_hierarchy_depth_total"),
        Some(report.task_hierarchy_depth_total)
    );
    assert_eq!(
        routing.metric("task_hierarchy_route_fanout_total"),
        Some(report.task_hierarchy_route_fanout_total)
    );
    assert_eq!(
        routing.metric("task_hierarchy_threshold_delta_milli"),
        Some(report.task_hierarchy_threshold_delta_milli)
    );
    assert_eq!(
        routing.metric("task_hierarchy_selected_lanes"),
        Some(report.task_hierarchy_selected_lanes)
    );
    assert_eq!(
        routing.metric("task_hierarchy_skipped_lanes"),
        Some(report.task_hierarchy_skipped_lanes)
    );
    assert_eq!(
        routing.metric("task_hierarchy_memory_lanes"),
        Some(report.task_hierarchy_memory_lanes)
    );
    assert_eq!(
        routing.metric("task_hierarchy_skipped_memory_lanes"),
        Some(report.task_hierarchy_skipped_memory_lanes)
    );
    assert_eq!(
        routing.metric("compute_budget_saved_tokens"),
        Some(report.compute_budget_saved_tokens)
    );
    assert_eq!(
        routing.metric("compute_budget_self_evolving_memory_fusion_saved_tokens"),
        Some(report.compute_budget_self_evolving_memory_fusion_saved_tokens)
    );
    assert_eq!(
        routing.metric("compute_budget_fanout_before"),
        Some(report.compute_budget_fanout_before)
    );
    assert_eq!(
        routing.metric("compute_budget_fanout_after"),
        Some(report.compute_budget_fanout_after)
    );
    assert_eq!(
        routing.metric("compute_budget_fanout_reduction"),
        Some(report.compute_budget_fanout_reduction)
    );
    assert_eq!(
        routing.metric("compute_budget_estimated_budget_tokens"),
        Some(report.compute_budget_estimated_budget_tokens)
    );
    assert_eq!(
        routing.metric("compute_budget_estimated_spent_tokens"),
        Some(report.compute_budget_estimated_spent_tokens)
    );
    assert_eq!(
        routing.metric("compute_budget_estimated_saved_tokens"),
        Some(report.compute_budget_estimated_saved_tokens)
    );
    assert_eq!(
        routing.metric("compute_budget_anchor_count"),
        Some(report.compute_budget_anchor_count)
    );
    assert_eq!(
        routing.metric("compute_budget_anchors_preserved"),
        Some(report.compute_budget_anchors_preserved)
    );
    assert_eq!(
        routing.metric("compute_budget_anchor_preservation_failures"),
        Some(report.compute_budget_anchor_preservation_failures)
    );
    assert_eq!(
        routing.metric("compute_budget_fallback_triggered"),
        Some(report.compute_budget_fallback_triggered)
    );
    assert_eq!(
        routing.metric("compute_budget_write_allowed"),
        Some(report.compute_budget_write_allowed)
    );
    assert_eq!(
        routing.metric("compute_budget_applied"),
        Some(report.compute_budget_applied)
    );
    assert_eq!(
        routing.metric("compute_budget_threshold_delta_milli"),
        Some(report.compute_budget_threshold_delta_milli)
    );
    assert_eq!(
        routing.metric("fht_dke_total_tokens"),
        Some(report.fht_dke_total_tokens)
    );
    assert_eq!(
        routing.metric("fht_dke_dense_tokens"),
        Some(report.fht_dke_dense_tokens)
    );
    assert_eq!(
        routing.metric("fht_dke_routed_tokens"),
        Some(report.fht_dke_routed_tokens)
    );
    assert_eq!(
        routing.metric("fht_dke_kv_exchange_blocks"),
        Some(report.fht_dke_kv_exchange_blocks)
    );
    assert_eq!(
        routing.metric("fht_dke_attention_threshold_milli"),
        Some(report.fht_dke_attention_threshold_milli)
    );
    assert_eq!(
        routing.metric("fht_dke_route_pressure_milli"),
        Some(report.fht_dke_route_pressure_milli)
    );
    assert_eq!(
        routing.metric("noiron_orchestration_events"),
        Some(report.noiron_orchestration_events)
    );
    assert_eq!(
        routing.metric("noiron_orchestration_stages"),
        Some(report.noiron_orchestration_stages)
    );
    assert_eq!(
        routing.metric("noiron_orchestration_failed_stages"),
        Some(report.noiron_orchestration_failed_stages)
    );
    assert_eq!(
        routing.metric("noiron_orchestration_writes_gated"),
        Some(report.noiron_orchestration_writes_gated)
    );
    assert_eq!(
        routing.metric("noiron_orchestration_fht_dke_total_tokens"),
        Some(report.noiron_orchestration_fht_dke_total_tokens)
    );
    let failed_noiron_report = TraceSchemaGateReport {
        passed: true,
        noiron_orchestration_events: 1,
        noiron_orchestration_failed_stages: 1,
        ..TraceSchemaGateReport::default()
    };
    let failed_noiron_snapshot = failed_noiron_report.operator_health_snapshot();
    let failed_noiron_routing = failed_noiron_snapshot.section("routing").unwrap();
    assert_eq!(failed_noiron_routing.status(), "review_required");
    assert_eq!(
        failed_noiron_routing.metric("noiron_orchestration_failed_stages"),
        Some(1)
    );

    let learning = snapshot.section("learning").unwrap();
    assert!(learning.data_present);
    assert_eq!(
        learning.metric("process_reward_events"),
        Some(report.process_reward_events)
    );
    assert_eq!(
        learning.metric("process_reward_positive"),
        Some(report.process_reward_positive)
    );
    assert_eq!(
        learning.metric("process_reward_reinforce"),
        Some(report.process_reward_reinforce)
    );
    assert_eq!(
        learning.metric("process_reward_hold"),
        Some(report.process_reward_hold)
    );
    assert_eq!(
        learning.metric("process_reward_penalize"),
        Some(report.process_reward_penalize)
    );
    assert_eq!(
        learning.metric("process_reward_total_milli"),
        Some(report.process_reward_total_milli)
    );
    assert_eq!(
        learning.metric("live_evolution_events"),
        Some(report.live_evolution_events)
    );
    assert_eq!(
        learning.metric("live_router_threshold_delta_milli"),
        Some(report.live_router_threshold_delta_milli)
    );
    assert_eq!(
        learning.metric("live_hierarchy_weight_delta_milli"),
        Some(report.live_hierarchy_weight_delta_milli)
    );
    assert_eq!(
        learning.metric("live_online_reward_feedbacks"),
        Some(report.live_online_reward_feedbacks)
    );
    assert_eq!(
        learning.metric("live_online_reward_reinforcements"),
        Some(report.live_online_reward_reinforcements)
    );
    assert_eq!(
        learning.metric("live_online_reward_penalties"),
        Some(report.live_online_reward_penalties)
    );
    assert_eq!(
        learning.metric("live_online_reward_strength_milli"),
        Some(report.live_online_reward_strength_milli)
    );
    assert_eq!(
        report.live_memory_reinforcements,
        expected_live_memory_reinforcements
    );
    assert_eq!(report.live_memory_penalties, expected_live_memory_penalties);
    assert_eq!(
        learning.metric("live_memory_reinforcements"),
        Some(report.live_memory_reinforcements)
    );
    assert_eq!(
        learning.metric("live_memory_penalties"),
        Some(report.live_memory_penalties)
    );
    assert_eq!(
        learning.metric("live_memory_updates"),
        Some(report.live_memory_updates)
    );
    assert_eq!(report.live_stored_memories, expected_live_stored_memories);
    assert_eq!(
        report.live_stored_gist_memories,
        expected_live_stored_gist_memories
    );
    assert_eq!(
        report.live_stored_runtime_kv_memories,
        expected_live_stored_runtime_kv_memories
    );
    assert_eq!(
        learning.metric("live_stored_memories"),
        Some(report.live_stored_memories)
    );
    assert_eq!(
        learning.metric("live_stored_gist_memories"),
        Some(report.live_stored_gist_memories)
    );
    assert_eq!(
        learning.metric("live_stored_runtime_kv_memories"),
        Some(report.live_stored_runtime_kv_memories)
    );
    assert_eq!(
        learning.metric("live_stored_memory_updates"),
        Some(report.live_stored_memory_updates)
    );
    assert_eq!(
        learning.metric("live_reflection_issues"),
        Some(report.live_reflection_issues)
    );
    assert_eq!(
        learning.metric("live_critical_reflection_issues"),
        Some(report.live_critical_reflection_issues)
    );
    assert_eq!(
        learning.metric("live_revision_actions"),
        Some(report.live_revision_actions)
    );
    assert_eq!(
        report.evolution_live_inference_runs,
        expected_evolution_live_inference_runs
    );
    assert_eq!(
        report.evolution_live_router_threshold_mutations,
        expected_evolution_live_router_threshold_mutations
    );
    assert_eq!(
        report.evolution_live_hierarchy_weight_mutations,
        expected_evolution_live_hierarchy_weight_mutations
    );
    assert_eq!(
        report.evolution_live_router_threshold_delta_milli,
        expected_evolution_live_router_threshold_delta_milli
    );
    assert_eq!(
        report.evolution_live_hierarchy_weight_delta_milli,
        expected_evolution_live_hierarchy_weight_delta_milli
    );
    assert_eq!(
        report.evolution_live_online_reward_feedbacks,
        expected_evolution_live_online_reward_feedbacks
    );
    assert_eq!(
        report.evolution_live_online_reward_reinforcements,
        expected_evolution_live_online_reward_reinforcements
    );
    assert_eq!(
        report.evolution_live_online_reward_penalties,
        expected_evolution_live_online_reward_penalties
    );
    assert_eq!(
        report.evolution_live_online_reward_strength_milli,
        expected_evolution_live_online_reward_strength_milli
    );
    assert_eq!(
        report.evolution_live_online_reward_reinforcement_strength_milli,
        expected_evolution_live_online_reward_reinforcement_strength_milli
    );
    assert_eq!(
        report.evolution_live_online_reward_penalty_strength_milli,
        expected_evolution_live_online_reward_penalty_strength_milli
    );
    assert_eq!(
        report.evolution_live_memory_reinforcements,
        expected_evolution_live_memory_reinforcements
    );
    assert_eq!(
        report.evolution_live_memory_penalties,
        expected_evolution_live_memory_penalties
    );
    assert_eq!(
        report.evolution_live_memory_updates,
        expected_evolution_live_memory_updates
    );
    assert_eq!(
        report.evolution_live_stored_memories,
        expected_evolution_live_stored_memories
    );
    assert_eq!(
        report.evolution_live_stored_gist_memories,
        expected_evolution_live_stored_gist_memories
    );
    assert_eq!(
        report.evolution_live_stored_runtime_kv_memories,
        expected_evolution_live_stored_runtime_kv_memories
    );
    assert_eq!(
        report.evolution_live_stored_memory_updates,
        expected_evolution_live_stored_memory_updates
    );
    assert_eq!(
        report.evolution_live_reflection_issues,
        expected_evolution_live_reflection_issues
    );
    assert_eq!(
        report.evolution_live_critical_reflection_issues,
        expected_evolution_live_critical_reflection_issues
    );
    assert_eq!(
        report.evolution_live_revision_actions,
        expected_evolution_live_revision_actions
    );
    assert!(report
        .summary_line()
        .contains("evolution_live_stored_runtime_kv_memories="));
    assert!(report.summary_line().contains("live_stored_memories="));
    assert!(report
        .summary_line()
        .contains("live_stored_runtime_kv_memories="));
    assert!(report
        .summary_line()
        .contains("evolution_live_online_reward_strength_milli="));
    assert!(report
        .summary_line()
        .contains("evolution_live_revision_actions="));
    assert_eq!(
        learning.metric("evolution_live_inference_runs"),
        Some(report.evolution_live_inference_runs)
    );
    assert_eq!(
        learning.metric("evolution_live_router_threshold_mutations"),
        Some(report.evolution_live_router_threshold_mutations)
    );
    assert_eq!(
        learning.metric("evolution_live_hierarchy_weight_mutations"),
        Some(report.evolution_live_hierarchy_weight_mutations)
    );
    assert_eq!(
        learning.metric("evolution_live_router_threshold_delta_milli"),
        Some(report.evolution_live_router_threshold_delta_milli)
    );
    assert_eq!(
        learning.metric("evolution_live_hierarchy_weight_delta_milli"),
        Some(report.evolution_live_hierarchy_weight_delta_milli)
    );
    assert_eq!(
        learning.metric("evolution_live_online_reward_feedbacks"),
        Some(report.evolution_live_online_reward_feedbacks)
    );
    assert_eq!(
        learning.metric("evolution_live_online_reward_reinforcements"),
        Some(report.evolution_live_online_reward_reinforcements)
    );
    assert_eq!(
        learning.metric("evolution_live_online_reward_penalties"),
        Some(report.evolution_live_online_reward_penalties)
    );
    assert_eq!(
        learning.metric("evolution_live_online_reward_strength_milli"),
        Some(report.evolution_live_online_reward_strength_milli)
    );
    assert_eq!(
        learning.metric("evolution_live_online_reward_reinforcement_strength_milli"),
        Some(report.evolution_live_online_reward_reinforcement_strength_milli)
    );
    assert_eq!(
        learning.metric("evolution_live_online_reward_penalty_strength_milli"),
        Some(report.evolution_live_online_reward_penalty_strength_milli)
    );
    assert_eq!(
        learning.metric("evolution_live_memory_reinforcements"),
        Some(report.evolution_live_memory_reinforcements)
    );
    assert_eq!(
        learning.metric("evolution_live_memory_penalties"),
        Some(report.evolution_live_memory_penalties)
    );
    assert_eq!(
        learning.metric("evolution_live_memory_updates"),
        Some(report.evolution_live_memory_updates)
    );
    assert_eq!(
        learning.metric("evolution_live_stored_memories"),
        Some(report.evolution_live_stored_memories)
    );
    assert_eq!(
        learning.metric("evolution_live_stored_gist_memories"),
        Some(report.evolution_live_stored_gist_memories)
    );
    assert_eq!(
        learning.metric("evolution_live_stored_runtime_kv_memories"),
        Some(report.evolution_live_stored_runtime_kv_memories)
    );
    assert_eq!(
        learning.metric("evolution_live_stored_memory_updates"),
        Some(report.evolution_live_stored_memory_updates)
    );
    assert_eq!(
        learning.metric("evolution_live_reflection_issues"),
        Some(report.evolution_live_reflection_issues)
    );
    assert_eq!(
        learning.metric("evolution_live_critical_reflection_issues"),
        Some(report.evolution_live_critical_reflection_issues)
    );
    assert_eq!(
        learning.metric("evolution_live_revision_actions"),
        Some(report.evolution_live_revision_actions)
    );
    assert_eq!(
        learning.metric("runtime_kv_weak_import_pressure_milli"),
        Some(report.runtime_kv_weak_import_pressure_milli)
    );

    let approval = snapshot.section("approval").unwrap();
    assert!(!approval.data_present);
    assert_eq!(approval.status(), "missing");

    assert!(json.contains("\"schema\":\"rust-norion-operator-health-v1\""));
    assert!(json.contains("\"trace_id_count\":"));
    assert!(json.contains("\"trace_ids\":["));
    assert!(json.contains("\"name\":\"memory\""));
    assert!(json.contains("\"runtime_kv_exported\":"));
    assert!(json.contains("\"runtime_kv_stored\":"));
    assert!(json.contains("\"runtime_kv_hold\":"));
    assert!(json.contains("\"runtime_kv_held\":"));
    assert!(json.contains("\"feedback_reinforced\":"));
    assert!(json.contains("\"feedback_penalized\":"));
    assert!(json.contains("\"feedback_reinforcement_milli\":"));
    assert!(json.contains("\"feedback_penalty_milli\":"));
    assert!(json.contains("\"feedback_updates\":"));
    assert!(json.contains("\"feedback_applied\":"));
    assert!(json.contains("\"feedback_removed\":"));
    assert!(json.contains("\"feedback_missing\":"));
    assert!(json.contains("\"feedback_strength_delta_milli\":"));
    assert!(json.contains("\"kv_fusion_events\":"));
    assert!(json.contains("\"kv_fusion_candidates\":"));
    assert!(json.contains("\"kv_fusion_fused\":"));
    assert!(json.contains("\"kv_fusion_compressed\":"));
    assert!(json.contains("\"kv_fusion_skipped\":"));
    assert!(json.contains("\"kv_fusion_held\":"));
    assert!(json.contains("\"kv_fusion_rejected\":"));
    assert!(json.contains("\"kv_fusion_input_tokens\":"));
    assert!(json.contains("\"kv_fusion_retained_tokens\":"));
    assert!(json.contains("\"kv_fusion_saved_tokens\":"));
    assert!(json.contains("\"kv_fusion_approval_blocked\":"));
    assert!(json.contains("\"name\":\"genome\""));
    assert!(json.contains("\"name\":\"routing\""));
    assert!(json.contains("\"adaptive_routing_include\":"));
    assert!(json.contains("\"adaptive_routing_retained_tokens\":"));
    assert!(json.contains("\"adaptive_routing_saved_tokens\":"));
    assert!(json.contains("\"task_hierarchy_route_pressure_milli\":"));
    assert!(json.contains("\"task_hierarchy_compute_reduction_milli\":"));
    assert!(json.contains("\"fht_dke_total_tokens\":"));
    assert!(json.contains("\"fht_dke_dense_tokens\":"));
    assert!(json.contains("\"fht_dke_routed_tokens\":"));
    assert!(json.contains("\"fht_dke_kv_exchange_blocks\":"));
    assert!(json.contains("\"fht_dke_attention_threshold_milli\":"));
    assert!(json.contains("\"fht_dke_route_pressure_milli\":"));
    assert!(json.contains("\"noiron_orchestration_events\":"));
    assert!(json.contains("\"noiron_orchestration_failed_stages\":"));
    assert!(json.contains("\"noiron_orchestration_writes_gated\":"));
    assert!(json.contains("\"noiron_orchestration_fht_dke_total_tokens\":"));
    assert!(json.contains("\"self_evolving_writeback_events\":"));
    assert!(json.contains("\"self_evolving_store_saved_tokens\":"));
    assert!(json.contains("\"self_evolving_store_retrieval_events\":"));
    assert!(json.contains("\"self_evolving_store_maintenance_events\":"));
    assert!(json.contains("\"self_evolving_store_admission_preview_events\":"));
    assert!(json.contains("\"self_evolving_store_contexts\":"));
    assert!(json.contains("\"self_evolving_store_maintenance_actions\":"));
    assert!(json.contains("\"self_evolving_store_admission_candidates\":"));
    assert!(json.contains("\"self_evolving_store_write_allowed\":"));
    assert!(json.contains("\"self_evolving_store_durable_write_allowed\":"));
    assert!(json.contains("\"self_evolving_store_applied\":"));
    assert!(json.contains("\"self_evolving_store_applied_to_disk\":"));
    assert!(json.contains("\"self_evolving_consolidation_events\":"));
    assert!(json.contains("\"self_evolving_consolidation_actions\":"));
    assert!(json.contains("\"self_evolving_merge_previews\":"));
    assert!(json.contains("\"self_evolving_decay_previews\":"));
    assert!(json.contains("\"self_evolving_tombstone_previews\":"));
    assert!(json.contains("\"self_evolving_merge_rejections\":"));
    assert!(json.contains("\"residency_events\":"));
    assert!(json.contains("\"residency_decisions\":"));
    assert!(json.contains("\"residency_hot\":"));
    assert!(json.contains("\"residency_warm\":"));
    assert!(json.contains("\"residency_cold\":"));
    assert!(json.contains("\"residency_quarantined\":"));
    assert!(json.contains("\"residency_retired\":"));
    assert!(json.contains("\"residency_protected_rollback_anchors\":"));
    assert!(json.contains("\"residency_blocked_reasons\":"));
    assert!(json.contains("\"residency_token_estimate\":"));
    assert!(json.contains("\"residency_write_allowed\":"));
    assert!(json.contains("\"residency_durable_write_allowed\":"));
    assert!(json.contains("\"residency_applied\":"));
    assert!(json.contains("\"self_evolving_writeback_records_before\":"));
    assert!(json.contains("\"self_evolving_writeback_attempted_records\":"));
    assert!(json.contains("\"self_evolving_writeback_accepted_records\":"));
    assert!(json.contains("\"self_evolving_writeback_rejected_records\":"));
    assert!(json.contains("\"self_evolving_writeback_source_case_digests\":"));
    assert!(json.contains("\"self_evolving_writeback_records_after\":"));
    assert!(json.contains("\"self_evolving_writeback_write_allowed\":"));
    assert!(json.contains("\"self_evolving_writeback_durable_write_allowed\":"));
    assert!(json.contains("\"self_evolving_writeback_applied\":"));
    assert!(json.contains("\"self_evolving_writeback_applied_to_disk\":"));
    assert!(json.contains("\"self_evolving_writeback_snapshot_changes\":"));
    assert!(json.contains("\"self_evolving_writeback_tool_reliability_after\":"));
    assert!(json.contains("\"self_evolving_writeback_tool_observations_after\":"));
    assert!(json.contains("\"name\":\"learning\""));
    assert!(json.contains("\"process_reward_events\":"));
    assert!(json.contains("\"process_reward_total_milli\":"));
    assert!(json.contains("\"live_router_threshold_delta_milli\":"));
    assert!(json.contains("\"live_hierarchy_weight_delta_milli\":"));
    assert!(json.contains("\"live_online_reward_feedbacks\":"));
    assert!(json.contains("\"live_memory_reinforcements\":"));
    assert!(json.contains("\"live_memory_penalties\":"));
    assert!(json.contains("\"live_stored_memories\":"));
    assert!(json.contains("\"live_stored_gist_memories\":"));
    assert!(json.contains("\"live_stored_runtime_kv_memories\":"));
    assert!(json.contains("\"live_reflection_issues\":"));
    assert!(json.contains("\"live_critical_reflection_issues\":"));
    assert!(json.contains("\"live_revision_actions\":"));
    assert!(json.contains("\"evolution_live_inference_runs\":"));
    assert!(json.contains("\"evolution_live_router_threshold_mutations\":"));
    assert!(json.contains("\"evolution_live_hierarchy_weight_mutations\":"));
    assert!(json.contains("\"evolution_live_router_threshold_delta_milli\":"));
    assert!(json.contains("\"evolution_live_hierarchy_weight_delta_milli\":"));
    assert!(json.contains("\"evolution_live_online_reward_feedbacks\":"));
    assert!(json.contains("\"evolution_live_online_reward_reinforcements\":"));
    assert!(json.contains("\"evolution_live_online_reward_penalties\":"));
    assert!(json.contains("\"evolution_live_online_reward_strength_milli\":"));
    assert!(json.contains("\"evolution_live_online_reward_reinforcement_strength_milli\":"));
    assert!(json.contains("\"evolution_live_online_reward_penalty_strength_milli\":"));
    assert!(json.contains("\"evolution_live_memory_reinforcements\":"));
    assert!(json.contains("\"evolution_live_memory_penalties\":"));
    assert!(json.contains("\"evolution_live_memory_updates\":"));
    assert!(json.contains("\"evolution_live_stored_memories\":"));
    assert!(json.contains("\"evolution_live_stored_gist_memories\":"));
    assert!(json.contains("\"evolution_live_stored_runtime_kv_memories\":"));
    assert!(json.contains("\"evolution_live_stored_memory_updates\":"));
    assert!(json.contains("\"evolution_live_reflection_issues\":"));
    assert!(json.contains("\"evolution_live_critical_reflection_issues\":"));
    assert!(json.contains("\"evolution_live_revision_actions\":"));
    assert!(json.contains("\"runtime_kv_weak_import_pressure_milli\":"));
    assert!(json.contains("\"compute_budget_write_allowed\":"));
    assert!(json.contains("\"compute_budget_applied\":"));
    assert!(!json.contains("sk-test-private-operator-health"));
    assert!(!json.contains("prompt_preview"));
    cleanup(path);
}

#[test]
fn trace_schema_jsonl_gate_exports_replay_live_evolution_health() {
    let path = temp_path("trace-schema-replay-live-evolution-health");
    let line = auto_replay_trace_line()
        .replacen(
            "\"live_memory_feedback_items\":0",
            "\"live_memory_feedback_items\":1",
            1,
        )
        .replacen(
            "\"live_memory_feedback_updates\":0",
            "\"live_memory_feedback_updates\":1",
            1,
        )
        .replacen(
            "\"live_memory_feedback_reinforcements\":0",
            "\"live_memory_feedback_reinforcements\":1",
            1,
        )
        .replacen(
            "\"live_memory_feedback_detail_items\":0",
            "\"live_memory_feedback_detail_items\":1",
            1,
        )
        .replacen(
            "\"live_memory_feedback_applied\":0",
            "\"live_memory_feedback_applied\":1",
            1,
        )
        .replacen(
            "\"live_memory_feedback_strength_delta\":-0.000000",
            "\"live_memory_feedback_strength_delta\":0.250000",
            1,
        )
        .replacen(
            "\"cumulative_replay_live_memory_feedback_items\":0",
            "\"cumulative_replay_live_memory_feedback_items\":1",
            1,
        )
        .replacen(
            "\"cumulative_replay_live_memory_feedback_updates\":0",
            "\"cumulative_replay_live_memory_feedback_updates\":1",
            1,
        )
        .replacen(
            "\"cumulative_replay_live_memory_feedback_reinforcements\":0",
            "\"cumulative_replay_live_memory_feedback_reinforcements\":1",
            1,
        )
        .replacen(
            "\"cumulative_replay_live_memory_feedback_detail_items\":0",
            "\"cumulative_replay_live_memory_feedback_detail_items\":1",
            1,
        )
        .replacen(
            "\"cumulative_replay_live_memory_feedback_applied\":0",
            "\"cumulative_replay_live_memory_feedback_applied\":1",
            1,
        )
        .replacen(
            "\"cumulative_replay_live_memory_feedback_strength_delta\":0.000000",
            "\"cumulative_replay_live_memory_feedback_strength_delta\":0.250000",
            1,
        )
        .replacen(
            "\"recursive_runtime_items\":0",
            "\"recursive_runtime_items\":1",
            1,
        )
        .replacen(
            "\"recursive_runtime_calls\":0",
            "\"recursive_runtime_calls\":4",
            1,
        )
        .replacen(
            "\"avg_recursive_call_pressure\":0.000000",
            "\"avg_recursive_call_pressure\":0.400000",
            1,
        )
        .replacen(
            "\"max_recursive_call_pressure\":0.000000",
            "\"max_recursive_call_pressure\":0.800000",
            1,
        )
        .replacen(
            "\"cumulative_recursive_replay_items\":0",
            "\"cumulative_recursive_replay_items\":1",
            1,
        )
        .replacen(
            "\"cumulative_recursive_runtime_calls\":0",
            "\"cumulative_recursive_runtime_calls\":4",
            1,
        );
    let auto_replay = json_object_after_field(&line, "auto_replay").unwrap();
    let ledger = json_object_after_field(&line, "evolution_ledger").unwrap();
    let expected_auto_feedback_items =
        extract_json_usize_field(auto_replay, "live_memory_feedback_items").unwrap();
    let expected_replay_feedback_items =
        extract_json_usize_field(ledger, "cumulative_replay_live_memory_feedback_items").unwrap();
    let expected_items =
        extract_json_usize_field(ledger, "cumulative_replay_live_evolution_items").unwrap();
    let expected_feedbacks = extract_json_usize_field(
        ledger,
        "cumulative_replay_live_evolution_online_reward_feedbacks",
    )
    .unwrap();

    assert!(expected_auto_feedback_items > 0, "{line}");
    assert!(expected_replay_feedback_items > 0, "{line}");
    assert!(expected_items > 0, "{line}");
    assert!(expected_feedbacks > 0, "{line}");
    fs::write(&path, format!("{line}\n")).unwrap();

    let report = evaluate_trace_schema_jsonl(&path).unwrap();
    let snapshot = report.operator_health_snapshot();
    let routing = snapshot.section("routing").unwrap();
    let learning = snapshot.section("learning").unwrap();
    let json = report.operator_health_json();
    let routing_checks = [
        (
            "auto_replay_recursive_runtime_items",
            report.auto_replay_recursive_runtime_items,
            extract_json_usize_field(auto_replay, "recursive_runtime_items").unwrap(),
        ),
        (
            "auto_replay_recursive_runtime_calls",
            report.auto_replay_recursive_runtime_calls,
            extract_json_usize_field(auto_replay, "recursive_runtime_calls").unwrap(),
        ),
        (
            "auto_replay_avg_recursive_call_pressure_milli",
            report.auto_replay_avg_recursive_call_pressure_milli,
            trace_milli(
                extract_json_f32_field(auto_replay, "avg_recursive_call_pressure").unwrap(),
            ),
        ),
        (
            "auto_replay_max_recursive_call_pressure_milli",
            report.auto_replay_max_recursive_call_pressure_milli,
            trace_milli(
                extract_json_f32_field(auto_replay, "max_recursive_call_pressure").unwrap(),
            ),
        ),
        (
            "evolution_recursive_replay_items",
            report.evolution_recursive_replay_items,
            extract_json_usize_field(ledger, "cumulative_recursive_replay_items").unwrap(),
        ),
        (
            "evolution_recursive_runtime_calls",
            report.evolution_recursive_runtime_calls,
            extract_json_usize_field(ledger, "cumulative_recursive_runtime_calls").unwrap(),
        ),
    ];
    let checks = [
        (
            "auto_replay_live_memory_feedback_items",
            report.auto_replay_live_memory_feedback_items,
            expected_auto_feedback_items,
        ),
        (
            "auto_replay_live_memory_feedback_updates",
            report.auto_replay_live_memory_feedback_updates,
            extract_json_usize_field(auto_replay, "live_memory_feedback_updates").unwrap(),
        ),
        (
            "auto_replay_live_memory_feedback_reinforcements",
            report.auto_replay_live_memory_feedback_reinforcements,
            extract_json_usize_field(auto_replay, "live_memory_feedback_reinforcements").unwrap(),
        ),
        (
            "auto_replay_live_memory_feedback_penalties",
            report.auto_replay_live_memory_feedback_penalties,
            extract_json_usize_field(auto_replay, "live_memory_feedback_penalties").unwrap(),
        ),
        (
            "auto_replay_live_memory_feedback_detail_items",
            report.auto_replay_live_memory_feedback_detail_items,
            extract_json_usize_field(auto_replay, "live_memory_feedback_detail_items").unwrap(),
        ),
        (
            "auto_replay_live_memory_feedback_applied",
            report.auto_replay_live_memory_feedback_applied,
            extract_json_usize_field(auto_replay, "live_memory_feedback_applied").unwrap(),
        ),
        (
            "auto_replay_live_memory_feedback_removed",
            report.auto_replay_live_memory_feedback_removed,
            extract_json_usize_field(auto_replay, "live_memory_feedback_removed").unwrap(),
        ),
        (
            "auto_replay_live_memory_feedback_missing",
            report.auto_replay_live_memory_feedback_missing,
            extract_json_usize_field(auto_replay, "live_memory_feedback_missing").unwrap(),
        ),
        (
            "auto_replay_live_memory_feedback_strength_delta_milli",
            report.auto_replay_live_memory_feedback_strength_delta_milli,
            trace_milli(
                extract_json_f32_field(auto_replay, "live_memory_feedback_strength_delta").unwrap(),
            ),
        ),
        (
            "replay_live_memory_feedback_items",
            report.replay_live_memory_feedback_items,
            expected_replay_feedback_items,
        ),
        (
            "replay_live_memory_feedback_updates",
            report.replay_live_memory_feedback_updates,
            extract_json_usize_field(ledger, "cumulative_replay_live_memory_feedback_updates")
                .unwrap(),
        ),
        (
            "replay_live_memory_feedback_reinforcements",
            report.replay_live_memory_feedback_reinforcements,
            extract_json_usize_field(
                ledger,
                "cumulative_replay_live_memory_feedback_reinforcements",
            )
            .unwrap(),
        ),
        (
            "replay_live_memory_feedback_penalties",
            report.replay_live_memory_feedback_penalties,
            extract_json_usize_field(ledger, "cumulative_replay_live_memory_feedback_penalties")
                .unwrap(),
        ),
        (
            "replay_live_memory_feedback_detail_items",
            report.replay_live_memory_feedback_detail_items,
            extract_json_usize_field(
                ledger,
                "cumulative_replay_live_memory_feedback_detail_items",
            )
            .unwrap(),
        ),
        (
            "replay_live_memory_feedback_applied",
            report.replay_live_memory_feedback_applied,
            extract_json_usize_field(ledger, "cumulative_replay_live_memory_feedback_applied")
                .unwrap(),
        ),
        (
            "replay_live_memory_feedback_removed",
            report.replay_live_memory_feedback_removed,
            extract_json_usize_field(ledger, "cumulative_replay_live_memory_feedback_removed")
                .unwrap(),
        ),
        (
            "replay_live_memory_feedback_missing",
            report.replay_live_memory_feedback_missing,
            extract_json_usize_field(ledger, "cumulative_replay_live_memory_feedback_missing")
                .unwrap(),
        ),
        (
            "replay_live_memory_feedback_strength_delta_milli",
            report.replay_live_memory_feedback_strength_delta_milli,
            trace_milli(
                extract_json_f32_field(
                    ledger,
                    "cumulative_replay_live_memory_feedback_strength_delta",
                )
                .unwrap(),
            ),
        ),
        (
            "replay_live_evolution_items",
            report.replay_live_evolution_items,
            expected_items,
        ),
        (
            "replay_live_evolution_router_threshold_mutations",
            report.replay_live_evolution_router_threshold_mutations,
            extract_json_usize_field(
                ledger,
                "cumulative_replay_live_evolution_router_threshold_mutations",
            )
            .unwrap(),
        ),
        (
            "replay_live_evolution_hierarchy_weight_mutations",
            report.replay_live_evolution_hierarchy_weight_mutations,
            extract_json_usize_field(
                ledger,
                "cumulative_replay_live_evolution_hierarchy_weight_mutations",
            )
            .unwrap(),
        ),
        (
            "replay_live_evolution_router_threshold_delta_milli",
            report.replay_live_evolution_router_threshold_delta_milli,
            trace_milli(
                extract_json_f32_field(
                    ledger,
                    "cumulative_replay_live_evolution_router_threshold_delta",
                )
                .unwrap(),
            ),
        ),
        (
            "replay_live_evolution_hierarchy_weight_delta_milli",
            report.replay_live_evolution_hierarchy_weight_delta_milli,
            trace_milli(
                extract_json_f32_field(
                    ledger,
                    "cumulative_replay_live_evolution_hierarchy_weight_delta",
                )
                .unwrap(),
            ),
        ),
        (
            "replay_live_evolution_online_reward_feedbacks",
            report.replay_live_evolution_online_reward_feedbacks,
            expected_feedbacks,
        ),
        (
            "replay_live_evolution_online_reward_reinforcements",
            report.replay_live_evolution_online_reward_reinforcements,
            extract_json_usize_field(
                ledger,
                "cumulative_replay_live_evolution_online_reward_reinforcements",
            )
            .unwrap(),
        ),
        (
            "replay_live_evolution_online_reward_penalties",
            report.replay_live_evolution_online_reward_penalties,
            extract_json_usize_field(
                ledger,
                "cumulative_replay_live_evolution_online_reward_penalties",
            )
            .unwrap(),
        ),
        (
            "replay_live_evolution_online_reward_strength_milli",
            report.replay_live_evolution_online_reward_strength_milli,
            trace_milli(
                extract_json_f32_field(
                    ledger,
                    "cumulative_replay_live_evolution_online_reward_strength",
                )
                .unwrap(),
            ),
        ),
        (
            "replay_live_evolution_online_reward_reinforcement_strength_milli",
            report.replay_live_evolution_online_reward_reinforcement_strength_milli,
            trace_milli(
                extract_json_f32_field(
                    ledger,
                    "cumulative_replay_live_evolution_online_reward_reinforcement_strength",
                )
                .unwrap(),
            ),
        ),
        (
            "replay_live_evolution_online_reward_penalty_strength_milli",
            report.replay_live_evolution_online_reward_penalty_strength_milli,
            trace_milli(
                extract_json_f32_field(
                    ledger,
                    "cumulative_replay_live_evolution_online_reward_penalty_strength",
                )
                .unwrap(),
            ),
        ),
        (
            "replay_live_evolution_memory_updates",
            report.replay_live_evolution_memory_updates,
            extract_json_usize_field(ledger, "cumulative_replay_live_evolution_memory_updates")
                .unwrap(),
        ),
        (
            "replay_live_evolution_stored_memory_updates",
            report.replay_live_evolution_stored_memory_updates,
            extract_json_usize_field(
                ledger,
                "cumulative_replay_live_evolution_stored_memory_updates",
            )
            .unwrap(),
        ),
        (
            "replay_live_evolution_reflection_issues",
            report.replay_live_evolution_reflection_issues,
            extract_json_usize_field(ledger, "cumulative_replay_live_evolution_reflection_issues")
                .unwrap(),
        ),
        (
            "replay_live_evolution_critical_reflection_issues",
            report.replay_live_evolution_critical_reflection_issues,
            extract_json_usize_field(
                ledger,
                "cumulative_replay_live_evolution_critical_reflection_issues",
            )
            .unwrap(),
        ),
        (
            "replay_live_evolution_revision_actions",
            report.replay_live_evolution_revision_actions,
            extract_json_usize_field(ledger, "cumulative_replay_live_evolution_revision_actions")
                .unwrap(),
        ),
    ];

    assert!(report.passed, "{:?}", report.failures);
    assert!(report
        .summary_line()
        .contains("auto_replay_live_memory_feedback_strength_delta_milli="));
    assert!(report
        .summary_line()
        .contains("replay_live_memory_feedback_strength_delta_milli="));
    assert!(report
        .summary_line()
        .contains("replay_live_evolution_online_reward_strength_milli="));
    assert!(report
        .summary_line()
        .contains("auto_replay_max_recursive_call_pressure_milli=800"));
    assert!(report
        .summary_line()
        .contains("evolution_recursive_runtime_calls=4"));
    for (name, actual, expected) in routing_checks {
        assert_eq!(actual, expected, "{name}");
        assert_eq!(routing.metric(name), Some(expected), "{name}");
        assert!(json.contains(&format!("\"{name}\":")), "{name}");
    }
    for (name, actual, expected) in checks {
        assert_eq!(actual, expected, "{name}");
        assert_eq!(learning.metric(name), Some(expected), "{name}");
        assert!(json.contains(&format!("\"{name}\":")), "{name}");
    }
    cleanup(path);
}

#[test]
fn operator_health_snapshot_covers_approval_and_rollback_gates() {
    let path = temp_path("trace-schema-operator-health-approval");
    let mut ledger = SelfEvolutionExperimentLedger::new();
    ledger.append_admission_report(
        "operator-health-rollback",
        &self_evolution_experiment_rollback_report("candidate-operator-health"),
    );
    let plan = ledger.rollback_replay_plan();
    let replay_gate = SelfEvolutionRollbackReplayGate::new().evaluate(&plan);
    let evidence = SelfEvolutionOperatorApprovalEvidence::from_review_packet(
        "maintainer-jy",
        "approval-ticket-operator-health",
        &replay_gate.review_packet,
        "approved for operator health export",
    );
    let approved =
        SelfEvolutionOperatorApprovalGate::new().evaluate(&replay_gate.review_packet, &evidence);
    let mut held_evidence = evidence.clone();
    held_evidence.approval_ticket_id.clear();
    let held = SelfEvolutionOperatorApprovalGate::new()
        .evaluate(&replay_gate.review_packet, &held_evidence);

    append_self_evolution_rollback_replay_trace_jsonl(&path, &plan).unwrap();
    append_self_evolution_rollback_replay_gate_trace_jsonl(&path, &replay_gate).unwrap();
    append_self_evolution_operator_approval_trace_jsonl(&path, &approved).unwrap();
    append_self_evolution_operator_approval_trace_jsonl(&path, &held).unwrap();

    let report = evaluate_trace_schema_jsonl(&path).unwrap();
    let snapshot = report.operator_health_snapshot();
    let json = snapshot.json_line();

    assert!(report.passed, "{:?}", report.failures);

    let approval = snapshot.section("approval").unwrap();
    assert!(approval.data_present);
    assert!(approval.review_required);
    assert!(approval.blocked);
    assert_eq!(
        approval.metric("operator_approval_events"),
        Some(report.self_evolution_operator_approval_events)
    );
    assert_eq!(approval.metric("operator_approved"), Some(1));
    assert_eq!(approval.metric("operator_held"), Some(1));

    let rollback = snapshot.section("rollback").unwrap();
    assert!(rollback.data_present);
    assert!(rollback.review_required);
    assert_eq!(
        rollback.metric("replay_events"),
        Some(report.self_evolution_rollback_replay_events)
    );
    assert_eq!(
        rollback.metric("gate_events"),
        Some(report.self_evolution_rollback_replay_gate_events)
    );

    let memory = snapshot.section("memory").unwrap();
    assert!(!memory.data_present);
    assert_eq!(memory.status(), "missing");

    assert!(!json.contains("approval-ticket-operator-health"));
    assert!(!json.contains("approved for operator health export"));
    cleanup(path);
}

#[test]
fn operator_health_snapshot_reports_failed_trace_gate_without_leaking_failures() {
    let path = temp_path("trace-schema-operator-health-failed-gate");
    fs::write(&path, "{\"schema\":\"rust-norion-trace-v1\"}\n").unwrap();

    let report = evaluate_trace_schema_jsonl(&path).unwrap();
    let snapshot = report.operator_health_snapshot();
    let json = snapshot.json_line();

    assert!(!report.passed);
    assert_eq!(snapshot.checked_lines, 1);
    assert_eq!(snapshot.failure_count, report.failures.len());
    assert!(snapshot.trace_ids.is_empty());
    assert!(snapshot.failure_count > 0);

    let trace_gate = snapshot.section("trace_gate").unwrap();
    assert!(trace_gate.data_present);
    assert!(trace_gate.review_required);
    assert!(trace_gate.blocked);
    assert_eq!(trace_gate.status(), "blocked");
    assert_eq!(
        trace_gate.metric("failure_count"),
        Some(report.failures.len())
    );
    assert_eq!(trace_gate.metric("trace_id_count"), Some(0));

    assert!(json.contains("\"failure_count\":"));
    assert!(json.contains("\"trace_id_count\":0"));
    assert!(json.contains("\"status\":\"blocked\""));
    assert!(!json.contains("missing trace field"));
    assert!(!json.contains("prompt_preview"));
    cleanup(path);
}

fn self_evolution_promotion_preflight_report(
    ready: bool,
    candidate_id: &str,
) -> SelfEvolutionPromotionPreflightReport {
    let router_preview = RouterThresholdAdjustmentPreviewPlanner::new().preview(
        NoironRouter::new().state(),
        TaskProfile::Coding,
        GenerationMetrics {
            perplexity: 36.0,
            semantic_consistency: 0.20,
            contradiction_count: 2,
            token_count: 64,
        },
    );
    let evidence = SelfEvolutionAdmissionEvidence::from_benchmark_gate(
        candidate_id,
        EvolutionLedger {
            replay_rust_check_items: 2,
            replay_rust_check_passed: 2,
            replay_rust_check_failed: 0,
            ..EvolutionLedger::default()
        },
        &BenchmarkGateReport {
            passed: true,
            failures: Vec::new(),
        },
    )
    .with_validation_evidence(SelfEvolutionValidationEvidence::from_lanes(
        SelfEvolutionValidationLane::new(2, 2, 0),
        SelfEvolutionValidationLane::new(3, 3, 0),
        SelfEvolutionValidationLane::new(1, 1, 0),
        SelfEvolutionValidationLane::new(1, 1, 0),
    ))
    .with_router_threshold_preview_report(&router_preview);
    let admission = SelfEvolutionAdmissionGate::new().evaluate(&evidence);
    let mut ledger = SelfEvolutionExperimentLedger::new();
    let experiment = ledger.append_admission_report("promotion-preflight-jsonl", &admission);
    let mut approval_evidence = SelfEvolutionOperatorApprovalEvidence::from_review_packet(
        "maintainer-jy",
        "promotion-preflight-jsonl-ticket",
        &admission.review_packet,
        "approved for promotion preflight trace aggregation",
    );
    if !ready {
        approval_evidence.approval_ticket_id.clear();
    }
    let approval = SelfEvolutionOperatorApprovalGate::new()
        .evaluate(&admission.review_packet, &approval_evidence);
    let report =
        SelfEvolutionPromotionPreflightGate::new().evaluate(&admission, &experiment, &approval);

    assert_eq!(report.ready_for_explicit_promotion, ready);
    report
}

fn self_evolution_experiment_passing_report(candidate_id: &str) -> SelfEvolutionAdmissionReport {
    let router_preview = RouterThresholdAdjustmentPreviewPlanner::new().preview(
        NoironRouter::new().state(),
        TaskProfile::Coding,
        GenerationMetrics {
            perplexity: 36.0,
            semantic_consistency: 0.20,
            contradiction_count: 2,
            token_count: 64,
        },
    );
    let evidence = SelfEvolutionAdmissionEvidence::from_benchmark_gate(
        candidate_id,
        EvolutionLedger {
            replay_rust_check_items: 2,
            replay_rust_check_passed: 2,
            replay_rust_check_failed: 0,
            ..EvolutionLedger::default()
        },
        &BenchmarkGateReport {
            passed: true,
            failures: Vec::new(),
        },
    )
    .with_validation_evidence(SelfEvolutionValidationEvidence::from_lanes(
        SelfEvolutionValidationLane::new(2, 2, 0),
        SelfEvolutionValidationLane::new(2, 2, 0),
        SelfEvolutionValidationLane::new(1, 1, 0),
        SelfEvolutionValidationLane::new(1, 1, 0),
    ))
    .with_router_threshold_preview_report(&router_preview);

    SelfEvolutionAdmissionGate::new().evaluate(&evidence)
}

fn self_evolution_experiment_hold_report(candidate_id: &str) -> SelfEvolutionAdmissionReport {
    let evidence = SelfEvolutionAdmissionEvidence::from_benchmark_gate(
        candidate_id,
        EvolutionLedger {
            replay_rust_check_items: 2,
            replay_rust_check_passed: 2,
            replay_rust_check_failed: 0,
            ..EvolutionLedger::default()
        },
        &BenchmarkGateReport {
            passed: true,
            failures: Vec::new(),
        },
    )
    .with_validation_evidence(SelfEvolutionValidationEvidence::from_lanes(
        SelfEvolutionValidationLane::new(2, 2, 0),
        SelfEvolutionValidationLane::new(2, 2, 0),
        SelfEvolutionValidationLane::new(1, 1, 0),
        SelfEvolutionValidationLane::new(1, 1, 0),
    ));

    SelfEvolutionAdmissionGate::new().evaluate(&evidence)
}

fn self_evolution_experiment_reject_report(candidate_id: &str) -> SelfEvolutionAdmissionReport {
    let evidence = SelfEvolutionAdmissionEvidence::from_benchmark_gate(
        candidate_id,
        EvolutionLedger {
            replay_rust_check_items: 1,
            replay_rust_check_passed: 0,
            replay_rust_check_failed: 1,
            ..EvolutionLedger::default()
        },
        &BenchmarkGateReport {
            passed: false,
            failures: vec!["trace experiment failed benchmark gate".to_owned()],
        },
    )
    .with_validation_evidence(SelfEvolutionValidationEvidence::from_lanes(
        SelfEvolutionValidationLane::new(1, 0, 1),
        SelfEvolutionValidationLane::new(1, 0, 1),
        SelfEvolutionValidationLane::new(1, 0, 1),
        SelfEvolutionValidationLane::new(1, 0, 1),
    ));

    SelfEvolutionAdmissionGate::new().evaluate(&evidence)
}

fn self_evolution_experiment_rollback_report(candidate_id: &str) -> SelfEvolutionAdmissionReport {
    let router_preview = RouterThresholdAdjustmentPreviewPlanner::new().preview(
        NoironRouter::new().state(),
        TaskProfile::Coding,
        GenerationMetrics {
            perplexity: 36.0,
            semantic_consistency: 0.20,
            contradiction_count: 2,
            token_count: 64,
        },
    );
    let evidence = SelfEvolutionAdmissionEvidence::from_benchmark_gate(
        candidate_id,
        EvolutionLedger {
            replay_rust_check_items: 2,
            replay_rust_check_passed: 2,
            replay_rust_check_failed: 0,
            drift_rollbacks: 1,
            rollback_router_threshold_delta: 0.02,
            rollback_hierarchy_weight_delta: 0.03,
            ..EvolutionLedger::default()
        },
        &BenchmarkGateReport {
            passed: true,
            failures: Vec::new(),
        },
    )
    .with_validation_evidence(SelfEvolutionValidationEvidence::from_lanes(
        SelfEvolutionValidationLane::new(2, 2, 0),
        SelfEvolutionValidationLane::new(2, 2, 0),
        SelfEvolutionValidationLane::new(1, 1, 0),
        SelfEvolutionValidationLane::new(1, 1, 0),
    ))
    .with_router_threshold_preview_report(&router_preview);

    SelfEvolutionAdmissionGate::new().evaluate(&evidence)
}

fn self_goal_queue_apply_report(write_enabled: bool) -> SelfGoalQueueApplyReport {
    let queue = EvolutionGoalQueue::new(Vec::new());
    let proposal = default_self_goal_proposal_report(&queue);
    let run = self_goal_passing_run_for_first_candidate(&proposal);
    let admission = default_self_goal_admission_report(&proposal, &[run]);
    let preview = default_self_goal_queue_preview_report(&queue, &proposal, &admission);
    let writer_candidate = UnifiedWriterGateCandidate::self_goal_queue_preview(&preview);
    let writer_gate = if write_enabled {
        UnifiedWriterGate::new()
            .with_policy(UnifiedWriterGatePolicy {
                durable_writes_enabled: true,
                ..UnifiedWriterGatePolicy::default()
            })
            .evaluate([writer_candidate])
    } else {
        UnifiedWriterGate::new().evaluate([writer_candidate])
    };

    default_self_goal_queue_apply_report(&queue, &preview, &writer_gate)
}

fn self_goal_queue_append_execution_report() -> SelfGoalQueueAppendExecutionReport {
    let queue = EvolutionGoalQueue::new(Vec::new());
    let proposal = default_self_goal_proposal_report(&queue);
    let run = self_goal_passing_run_for_first_candidate(&proposal);
    let admission = default_self_goal_admission_report(&proposal, &[run]);
    let preview = default_self_goal_queue_preview_report(&queue, &proposal, &admission);
    let writer_gate = UnifiedWriterGate::new()
        .with_policy(UnifiedWriterGatePolicy {
            durable_writes_enabled: true,
            ..UnifiedWriterGatePolicy::default()
        })
        .evaluate([UnifiedWriterGateCandidate::self_goal_queue_preview(
            &preview,
        )]);
    let apply_report = default_self_goal_queue_apply_report(&queue, &preview, &writer_gate);
    let approval = SelfGoalQueueAppendApproval::from_apply_report(
        "operator-jy",
        "approval-ticket-self-goal-append",
        &apply_report,
    );

    SelfGoalQueueAppendExecutor::default().evaluate(
        &queue,
        &proposal,
        &preview,
        &apply_report,
        Some(&approval),
    )
}

fn self_goal_queue_continuation_line() -> String {
    "{\"schema\":\"rust-norion-self-goal-queue-continuation-plan-v1\",\"plan_schema\":\"self_goal_queue_continuation_plan_v1\",\"source\":\"completion_resulting_queue\",\"ready\":true,\"queue_digest\":\"redaction-digest:queue\",\"goals\":1,\"active\":true,\"active_goal_id\":\"redaction-digest:goal\",\"required_evidence_count\":4,\"required_evidence\":[\"cargo_check\",\"benchmark_gate\",\"trace_schema_gate\",\"operator_approval\"],\"evidence_template_digest\":\"redaction-digest:template\",\"continuation_digest\":\"redaction-digest:continuation\",\"budget_attempts\":3,\"budget_steps\":12,\"budget_tokens\":80000,\"budget_runtime_ms\":900000,\"reason_code_count\":2,\"reason_codes\":[\"completion_pruned_prefix\",\"next_goal_ready_for_evidence\"],\"read_only\":true,\"write_allowed\":false,\"applied\":false,\"summary\":\"self_goal_queue_continuation source=completion_resulting_queue ready=true goals=1\"}".to_owned()
}

fn self_goal_queue_evidence_plan_line() -> String {
    "{\"schema\":\"rust-norion-self-goal-queue-evidence-plan-v1\",\"plan_schema\":\"self_goal_queue_evidence_plan_v1\",\"source\":\"completion_resulting_queue\",\"ready\":true,\"active_goal_id\":\"redaction-digest:goal\",\"required_evidence_count\":4,\"required_evidence\":[\"cargo_check\",\"benchmark_gate\",\"trace_schema_gate\",\"operator_approval\"],\"planned_step_count\":4,\"step_kinds\":[\"cargo_check\",\"benchmark_gate\",\"trace_schema_gate\",\"operator_approval\"],\"auto_collectible_steps\":3,\"manual_steps\":1,\"evidence_template_digest\":\"redaction-digest:template\",\"evidence_plan_digest\":\"redaction-digest:evidence-plan\",\"packet_template_digests\":[\"redaction-digest:packet-1\",\"redaction-digest:packet-2\",\"redaction-digest:packet-3\",\"redaction-digest:packet-4\"],\"command_digests\":[\"redaction-digest:command-1\",\"redaction-digest:command-2\",\"redaction-digest:command-3\",\"redaction-digest:command-4\"],\"read_only\":true,\"write_allowed\":false,\"applied\":false,\"summary\":\"self_goal_queue_evidence_plan source=completion_resulting_queue ready=true steps=4\"}".to_owned()
}

fn self_goal_queue_evidence_collection_line() -> String {
    "{\"schema\":\"rust-norion-self-goal-queue-evidence-collection-v1\",\"collection_schema\":\"self_goal_queue_evidence_collection_v1\",\"source\":\"completion_resulting_queue\",\"ready\":true,\"collection_complete\":false,\"active_goal_id\":\"redaction-digest:goal\",\"planned_step_count\":4,\"step_kinds\":[\"cargo_check\",\"benchmark_gate\",\"trace_schema_gate\",\"operator_approval\"],\"step_statuses\":[\"passed\",\"failed\",\"missing\",\"manual_missing\"],\"passed_steps\":1,\"failed_steps\":1,\"missing_steps\":1,\"manual_missing_steps\":1,\"auto_collectible_steps\":3,\"manual_required_steps\":1,\"collected_evidence_count\":2,\"collected_evidence_digests\":[\"redaction-digest:evidence-1\",\"redaction-digest:evidence-2\"],\"collection_packet_digests\":[\"redaction-digest:packet-1\",\"redaction-digest:packet-2\",\"redaction-digest:packet-3\",\"redaction-digest:packet-4\"],\"evidence_collection_digest\":\"redaction-digest:collection\",\"read_only\":true,\"write_allowed\":false,\"applied\":false,\"summary\":\"self_goal_queue_evidence_collection source=completion_resulting_queue ready=true planned=4\"}".to_owned()
}

fn self_goal_passing_run_for_first_candidate(
    proposal: &SelfGoalProposalReport,
) -> EvolutionGoalRunEvidence {
    let candidate = &proposal.candidates[0];
    let mut run = EvolutionGoalRunEvidence::new(candidate.proposed_goal.stable_id.clone());
    for kind in &candidate.proposed_goal.success_gate.required_evidence {
        run = run.with_evidence([self_goal_evidence_for_kind(*kind)]);
    }
    run.with_approval()
}

fn self_goal_evidence_for_kind(kind: EvolutionGoalEvidenceKind) -> EvolutionGoalEvidence {
    match kind {
        EvolutionGoalEvidenceKind::CargoCheck => EvolutionGoalEvidence::cargo_check(true),
        EvolutionGoalEvidenceKind::FocusedTests => EvolutionGoalEvidence::focused_tests(true, 3, 0),
        EvolutionGoalEvidenceKind::BenchmarkGate => EvolutionGoalEvidence::benchmark_gate(true),
        EvolutionGoalEvidenceKind::TraceSchemaGate => {
            EvolutionGoalEvidence::trace_schema_gate(true)
        }
        EvolutionGoalEvidenceKind::ExperimentLedger => {
            EvolutionGoalEvidence::experiment_ledger(true)
        }
        EvolutionGoalEvidenceKind::OperatorApproval => {
            EvolutionGoalEvidence::operator_approval(true)
        }
    }
}

fn unified_writer_gate_ready_candidate(
    domain: UnifiedWriterGateDomain,
    candidate_id: &str,
) -> UnifiedWriterGateCandidate {
    let scope = match domain {
        UnifiedWriterGateDomain::Memory => UnifiedWriterGateWriteScope::DurableMemory,
        UnifiedWriterGateDomain::Genome => UnifiedWriterGateWriteScope::Genome,
        UnifiedWriterGateDomain::ExperimentLedger => UnifiedWriterGateWriteScope::ExperimentLedger,
        UnifiedWriterGateDomain::EvolutionGoalQueue => {
            UnifiedWriterGateWriteScope::EvolutionGoalQueue
        }
    };

    UnifiedWriterGateCandidate::new(domain, candidate_id, [scope])
        .with_refs(
            vec!["review:trace".to_owned()],
            vec!["evidence:trace".to_owned()],
            vec!["rollback:trace".to_owned()],
            vec!["digest:trace".to_owned()],
            vec!["schema:trace".to_owned()],
        )
        .with_evidence(true, true, true, true, true)
        .with_operator_approval(true, true)
}

fn trace_milli(value: f32) -> usize {
    if value.is_finite() {
        (value.clamp(0.0, 1.0) * 1000.0).round() as usize
    } else {
        0
    }
}
