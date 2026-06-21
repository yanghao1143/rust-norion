use super::*;
use crate::kv_cache::{MemoryResidencyCandidate, MemoryResidencyPolicy, plan_memory_residency};
use crate::memory_admission::MemoryAdmissionKind;
use crate::self_evolving_memory::{
    SelfEvolvingEpisodeInput, SelfEvolvingHeuristicInput,
    SelfEvolvingMemoryAdmissionCandidatePreview, SelfEvolvingMemoryAdmissionPreview,
    SelfEvolvingMemoryApproval, SelfEvolvingMemoryMaintenancePolicy, SelfEvolvingMemoryQuery,
    SelfEvolvingMemoryStore,
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

fn trace_milli(value: f32) -> usize {
    if value.is_finite() {
        (value.clamp(0.0, 1.0) * 1000.0).round() as usize
    } else {
        0
    }
}
