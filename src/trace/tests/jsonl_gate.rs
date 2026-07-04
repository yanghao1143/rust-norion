use super::*;
use crate::hierarchy::TaskProfile;
use crate::kv_cache::{MemoryResidencyCandidate, MemoryResidencyPolicy, plan_memory_residency};
use crate::memory_admission::{MemoryAdmissionKind, MemoryShadowCandidateState};
use crate::reasoning_genome::{
    DnaEvolutionController, DnaEvolutionValidationEvidence, GeneScissorsIntent,
    GeneScissorsOperatorDecision, GeneScissorsTransactionJournal, MutationPlan,
};
use crate::self_evolving_memory::{
    SelfEvolvingEpisodeInput, SelfEvolvingHeuristicInput,
    SelfEvolvingMemoryAdmissionCandidatePreview, SelfEvolvingMemoryAdmissionPreview,
    SelfEvolvingMemoryApproval, SelfEvolvingMemoryMaintenancePolicy, SelfEvolvingMemoryQuery,
    SelfEvolvingMemoryStore,
};
use crate::{
    EvolutionGoalEvidence, EvolutionGoalEvidenceKind, EvolutionGoalQueue,
    EvolutionGoalQueueDiskStore, EvolutionGoalQueueStoreApproval, EvolutionGoalQueueStorePolicy,
    EvolutionGoalRunEvidence, MemoryVerifierDecision, SelfGoalProposalReport,
    SelfGoalQueueAppendApproval, SelfGoalQueueAppendExecutionReport, SelfGoalQueueAppendExecutor,
    SelfGoalQueueApplyReport, TenantResourceLane, TenantScope, UnifiedWriterGate,
    UnifiedWriterGateCandidate, UnifiedWriterGateDomain, UnifiedWriterGatePolicy,
    UnifiedWriterGateWriteScope, default_self_goal_admission_report,
    default_self_goal_proposal_report, default_self_goal_queue_apply_report,
    default_self_goal_queue_preview_report,
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
    assert!(report.summary_line().contains("passed=true"));
    assert!(report.summary_line().contains("rust_check_events=0"));
    assert!(report.summary_line().contains("runtime_error_events=0"));
    assert!(report.summary_line().contains("runtime_timeout_events=0"));
    cleanup(path);
}

#[test]
fn trace_schema_gate_blocks_development_polluted_trace_surface() {
    let path = temp_path("trace-schema-development-pollution");
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let prompt = "development_evidence_contamination trace payload";
    let outcome = engine.infer(
        InferenceRequest::new(prompt, TaskProfile::Coding),
        &mut backend,
    );
    let line = trace_json_line(prompt, TaskProfile::Coding, 8, &outcome);
    let surface_start = line
        .find("\"development_evidence_surface\":{")
        .expect("trace line should include development evidence surface metadata");
    let surface_end = line[surface_start..]
        .find(",\"elapsed_ms\":")
        .map(|offset| surface_start + offset)
        .expect("trace line should keep elapsed_ms after surface metadata");
    let blocked_surface = "\"development_evidence_surface\":{\"surface\":\"trace\",\"allowed\":false,\"decision\":\"block\",\"reason\":\"development_evidence_contamination\",\"source_digest\":\"redaction-digest:blocked-trace\"}";
    let line = format!(
        "{}{}{}",
        &line[..surface_start],
        blocked_surface,
        &line[surface_end..]
    );

    fs::write(&path, format!("{line}\n")).unwrap();

    let report = evaluate_trace_schema_jsonl(&path).unwrap();

    assert!(!report.passed);
    assert!(
        report
            .failures
            .iter()
            .any(|failure| failure.contains("development_evidence_surface blocked trace evidence"))
    );
    assert!(line.contains("\"development_evidence_surface\":{"));
    assert!(line.contains("\"allowed\":false"));
    assert!(!line.contains("trace payload"));
    assert!(!report.failures.join("\n").contains("trace payload"));
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
    assert!(
        trace_report
            .summary_line()
            .contains("coding_service_eval_runner_events=1")
    );
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
    assert_eq!(
        report.memory_admission_read_only,
        report.memory_admission_events
    );
    assert_eq!(report.memory_admission_write_allowed, 0);
    assert_eq!(report.memory_admission_applied, 0);
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
    assert!(
        report
            .summary_line()
            .contains("memory_admission_write_allowed=0")
    );
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
    let expected_budget_selected = extract_json_usize_field(first_budget, "selected_candidates")
        .unwrap()
        .saturating_add(extract_json_usize_field(second_budget, "selected_candidates").unwrap());
    let expected_budget_saved = extract_json_usize_field(first_budget, "saved_tokens")
        .unwrap()
        .saturating_add(extract_json_usize_field(second_budget, "saved_tokens").unwrap());
    let expected_budget_avoided =
        extract_json_usize_field(first_budget, "wasted_compute_avoided_tokens")
            .unwrap()
            .saturating_add(
                extract_json_usize_field(second_budget, "wasted_compute_avoided_tokens").unwrap(),
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
    assert_eq!(report.compute_budget_events, 2);
    assert_eq!(
        report.compute_budget_selected_candidates,
        expected_budget_selected
    );
    assert_eq!(report.compute_budget_saved_tokens, expected_budget_saved);
    assert_eq!(
        report.compute_budget_avoided_tokens,
        expected_budget_avoided
    );
    assert_eq!(report.compute_budget_write_allowed, 0);
    assert_eq!(report.compute_budget_applied, 0);
    assert!(
        report
            .summary_line()
            .contains("adaptive_routing_candidates=")
    );
    assert!(
        report
            .summary_line()
            .contains("task_hierarchy_mutation_records=")
    );
    assert!(report.summary_line().contains("compute_budget_events=2"));
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
    assert!(
        report
            .summary_line()
            .contains("self_evolution_experiment_events=4")
    );
    assert!(
        report
            .summary_line()
            .contains("self_evolution_experiment_rollback=1")
    );
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
    assert!(
        report
            .summary_line()
            .contains("self_evolution_rollback_replay_events=1")
    );
    assert!(
        report
            .summary_line()
            .contains("self_evolution_rollback_replay_replayable=1")
    );
    assert!(
        report
            .summary_line()
            .contains("self_evolution_rollback_replay_gate_admitted=1")
    );
    assert!(
        report
            .summary_line()
            .contains("self_evolution_rollback_replay_gate_review_packets=1")
    );
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
    assert!(
        report
            .summary_line()
            .contains("self_evolution_rollback_replay_gate_held=1")
    );
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
    assert!(
        report
            .summary_line()
            .contains("self_evolution_operator_approval_events=2")
    );
    assert!(
        report
            .summary_line()
            .contains("self_evolution_operator_approval_held=1")
    );
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
    assert!(
        failures.contains(
            &"self_evolution_operator_approval_approved_missing_review_packets".to_owned()
        )
    );
    assert!(
        failures
            .contains(&"self_evolution_operator_approval_approved_missing_evidence_ids".to_owned())
    );
    assert!(failures.contains(
        &"self_evolution_operator_approval_approved_missing_rollback_anchors".to_owned()
    ));
    assert!(
        failures.contains(
            &"self_evolution_operator_approval_approved_missing_content_digests".to_owned()
        )
    );
    assert!(failures.contains(
        &"self_evolution_operator_approval_approved_missing_source_report_schemas".to_owned()
    ));
    assert!(
        failures
            .contains(&"self_evolution_operator_approval_missing_review_packet_refs".to_owned())
    );
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
    assert!(
        report
            .summary_line()
            .contains("self_evolution_rollback_replay_apply_events=2")
    );
    assert!(
        report
            .summary_line()
            .contains("self_evolution_rollback_replay_apply_ready=1")
    );
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
    assert!(
        report
            .summary_line()
            .contains("self_evolution_promotion_preflight_events=2")
    );
    assert!(
        report
            .summary_line()
            .contains("self_evolution_promotion_preflight_ready=1")
    );
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
        token_budget: 96,
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
                shadow_state: MemoryShadowCandidateState::ReadyForExplicitApply,
                task_profile: TaskProfile::Coding,
                score_milli: 900,
                source_hash: "sha256:ready".to_owned(),
                source_ids: vec![
                    "candidate:sem_candidate_ready".to_owned(),
                    "source:sha256:ready".to_owned(),
                ],
                rollback_anchor_id: "rollback:ready".to_owned(),
                expires_after_steps: 168,
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
                shadow_state: MemoryShadowCandidateState::BenchmarkPending,
                task_profile: TaskProfile::Coding,
                score_milli: 450,
                source_hash: "sha256:blocked".to_owned(),
                source_ids: vec![
                    "candidate:sem_candidate_blocked".to_owned(),
                    "source:sha256:blocked".to_owned(),
                ],
                rollback_anchor_id: "rollback:blocked".to_owned(),
                expires_after_steps: 72,
                validation_evidence_count: 0,
                eligible_for_store: false,
                blocked_reasons: vec![
                    "self_evolving_memory_validation_evidence_missing".to_owned(),
                ],
                read_only: true,
                write_allowed: false,
                applied: false,
            },
        ],
        read_only: true,
        write_allowed: false,
        applied: false,
    };
    fs::write(
        &path,
        format!(
            "{}\n{}\n{}\n",
            retrieval.json_line(),
            maintenance.json_line(),
            admission.json_line()
        ),
    )
    .unwrap();

    let report = evaluate_trace_schema_jsonl(&path).unwrap();

    assert!(report.passed, "{:?}", report.failures);
    assert_eq!(report.checked_lines, 3);
    assert_eq!(report.self_evolving_memory_store_events, 3);
    assert_eq!(report.self_evolving_memory_store_retrieval_events, 1);
    assert_eq!(report.self_evolving_memory_store_maintenance_events, 1);
    assert_eq!(
        report.self_evolving_memory_store_admission_preview_events,
        1
    );
    assert_eq!(
        report.self_evolving_memory_store_contexts,
        retrieval.total_contexts()
    );
    assert_eq!(
        report.self_evolving_memory_store_maintenance_actions,
        maintenance.action_count()
    );
    assert_eq!(report.self_evolving_memory_store_admission_candidates, 2);
    assert_eq!(report.self_evolving_memory_store_write_allowed, 0);
    assert_eq!(report.self_evolving_memory_store_durable_write_allowed, 0);
    assert_eq!(report.self_evolving_memory_store_applied, 0);
    assert_eq!(report.self_evolving_memory_store_applied_to_disk, 0);
    assert!(
        report
            .summary_line()
            .contains("self_evolving_memory_store_events=3")
    );
    cleanup(path);
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
    assert!(
        report
            .summary_line()
            .contains("unified_writer_gate_events=3")
    );
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
    assert!(
        report
            .summary_line()
            .contains("self_goal_queue_apply_events=1")
    );
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
    assert!(
        gate.summary_line()
            .contains("self_goal_queue_continuation_ready=1")
    );
    assert!(
        gate.summary_line()
            .contains("self_goal_queue_continuation_held=1")
    );
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
    assert!(
        gate.operator_health_json()
            .contains("\"name\":\"self_goal_queue\"")
    );
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
    assert!(
        gate.summary_line()
            .contains("self_goal_queue_evidence_plan_events=1")
    );
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
    assert!(
        gate.summary_line()
            .contains("self_goal_queue_evidence_collection_events=1")
    );
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
    assert!(
        gate.summary_line()
            .contains("evolution_goal_queue_store_write_events=1")
    );
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
        memory.metric("admission_events"),
        Some(report.memory_admission_events)
    );
    assert_eq!(
        memory.metric("kv_fusion_saved_tokens"),
        Some(report.kv_fusion_saved_tokens)
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
        routing.metric("compute_budget_saved_tokens"),
        Some(report.compute_budget_saved_tokens)
    );

    let approval = snapshot.section("approval").unwrap();
    assert!(!approval.data_present);
    assert_eq!(approval.status(), "missing");

    assert!(json.contains("\"schema\":\"rust-norion-operator-health-v1\""));
    assert!(json.contains("\"trace_id_count\":"));
    assert!(json.contains("\"trace_ids\":["));
    assert!(json.contains("\"name\":\"memory\""));
    assert!(json.contains("\"name\":\"genome\""));
    assert!(json.contains("\"name\":\"routing\""));
    assert!(!json.contains("sk-test-private-operator-health"));
    assert!(!json.contains("prompt_preview"));
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
        .with_verifier_cluster(
            MemoryVerifierDecision::Pass,
            MemoryVerifierDecision::Pass,
            MemoryVerifierDecision::Pass,
            MemoryVerifierDecision::Pass,
        )
        .with_operator_approval(true, true)
}

fn trace_milli(value: f32) -> usize {
    if value.is_finite() {
        (value.clamp(0.0, 1.0) * 1000.0).round() as usize
    } else {
        0
    }
}
