use super::*;
use rust_norion::{
    EvolutionGoal, EvolutionGoalEvidenceKind, EvolutionGoalQueue, EvolutionGoalQueueDiskStore,
    EvolutionGoalQueueStoreApproval, EvolutionGoalQueueStorePolicy, EvolutionGoalStatus,
    EvolutionGoalSuccessGate, SelfGoalAdmissionDecision, TenantResourceLane, TenantScope,
    stable_redaction_digest,
};
use std::fs;

#[test]
fn self_goal_queue_cli_preview_holds_default_active_goal() {
    let args = Args::parse(vec!["--self-goal-queue".to_owned()]);
    let report = crate::cli::self_goal_queue::run_self_goal_queue_report(&args).unwrap();
    let summary = report.summary_lines().join("\n");

    assert_eq!(report.current_goal_count, 1);
    assert!(!report.current_queue_loaded_from_store);
    assert_eq!(report.queue_run.decisions.len(), 1);
    assert!(report.queue_run.active_goal_id.is_some());
    assert!(report.run_plan.active);
    assert_eq!(report.run_plan.required_evidence.len(), 4);
    assert_eq!(report.continuation_plan.source, "current_queue");
    assert!(report.continuation_plan.ready);
    assert_eq!(
        report.continuation_plan.plan.active_goal_id,
        report.run_plan.active_goal_id
    );
    assert_eq!(report.admission.preview_admissible_count, 0);
    assert_eq!(report.queue_preview.append_preview_count, 0);
    assert!(!report.apply.explicit_apply_required);
    assert!(!report.append_execution.applied);
    assert!(report.store_read.is_none());
    assert!(report.store_write.is_none());
    assert!(summary.contains("redaction-digest:"));
    assert!(!summary.contains("English/Chinese/Rust coding service"));
    assert!(!summary.contains("local model can persist useful experience"));
}

#[test]
fn self_goal_queue_cli_evaluates_current_queue_evidence_by_queue_index() {
    let args = Args::parse(vec![
        "--self-goal-queue".to_owned(),
        "--self-goal-queue-evidence".to_owned(),
        "queue_index=0;kind=cargo_check;passed=true".to_owned(),
        "--self-goal-queue-evidence".to_owned(),
        "queue_index=0;kind=focused_tests;passed=true;items=3;failures=0".to_owned(),
        "--self-goal-queue-evidence".to_owned(),
        "queue_index=0;kind=trace_schema_gate;passed=true".to_owned(),
        "--self-goal-queue-evidence".to_owned(),
        "queue_index=0;kind=operator_approval;passed=true;approval=true".to_owned(),
    ]);

    let report = crate::cli::self_goal_queue::run_self_goal_queue_report(&args).unwrap();
    let summary = report.summary_lines().join("\n");

    assert_eq!(report.evidence.packet_count, 4);
    assert_eq!(report.evidence.valid_packet_count, 4);
    assert_eq!(report.evidence.invalid_packet_count, 0);
    assert_eq!(report.evidence.run_count, 1);
    assert_eq!(report.evidence.approval_count, 1);
    assert_eq!(report.queue_run.passed_count, 1);
    assert!(report.queue_run.active_goal_id.is_none());
    assert_eq!(
        report.queue_run.decisions[0].status,
        EvolutionGoalStatus::Passed
    );
    assert_eq!(
        report.queue_run.decisions[0].reason_codes,
        ["success_gate_passed"]
    );
    assert!(!report.run_plan.active);
    assert!(report.completion_preview.ready);
    assert_eq!(report.completion_preview.completed_count, 1);
    assert_eq!(report.completion_preview.retained_count, 0);
    assert_eq!(
        report.continuation_plan.source,
        "completion_resulting_queue"
    );
    assert!(!report.continuation_plan.ready);
    assert!(!report.continuation_plan.plan.active);
    assert_eq!(
        report.completion_writer_gate.preview_only_records,
        1,
        "{}",
        report.completion_writer_gate.summary_line()
    );
    assert!(summary.contains("self_goal_queue_run decisions=1 active=none passed=1"));
    assert!(summary.contains("self_goal_queue_completion ready=true completed=1 retained=0"));
    assert!(summary.contains("self_goal_queue_run_evolution_goal_decision_v1"));
    assert!(summary.contains("status=passed"));
    assert!(!summary.contains("R97 English/Chinese/Rust coding service"));
}

#[test]
fn self_goal_queue_cli_store_apply_prunes_completed_current_goal() {
    let dir = temp_asset_dir("self-goal-queue-cli-completion");
    fs::create_dir_all(&dir).unwrap();
    let store_path = dir.join("queue.ndkv");
    let trace_path = dir.join("completion-trace.jsonl");
    let args = Args::parse(vec![
        "--self-goal-queue".to_owned(),
        "--self-goal-queue-store".to_owned(),
        store_path.display().to_string(),
        "--self-goal-queue-store-apply".to_owned(),
        "--self-goal-queue-evidence".to_owned(),
        "queue_index=0;kind=cargo_check;passed=true".to_owned(),
        "--self-goal-queue-evidence".to_owned(),
        "queue_index=0;kind=focused_tests;passed=true;items=3;failures=0".to_owned(),
        "--self-goal-queue-evidence".to_owned(),
        "queue_index=0;kind=trace_schema_gate;passed=true".to_owned(),
        "--self-goal-queue-evidence".to_owned(),
        "queue_index=0;kind=operator_approval;passed=true;approval=true".to_owned(),
        "--trace-schema-gate".to_owned(),
        trace_path.display().to_string(),
    ]);

    let report = crate::cli::self_goal_queue::run_self_goal_queue_report(&args).unwrap();
    let store_write = report.store_write.as_ref().expect("store write report");
    let trace_report = evaluate_trace_schema_jsonl(&trace_path).unwrap();
    let scope = TenantScope::new("local", "default", "interactive");
    let key = scope.scoped_key(TenantResourceLane::EvolutionGoalQueue, "pursuit");
    let store = EvolutionGoalQueueDiskStore::open(&store_path).unwrap();
    let read = store.read_queue(&scope, key.as_str()).unwrap();
    let summary = report.summary_lines().join("\n");
    let trace = fs::read_to_string(&trace_path).unwrap();

    assert!(report.completion_preview.ready);
    assert_eq!(report.completion_preview.completed_count, 1);
    assert_eq!(report.completion_preview.retained_count, 0);
    assert_eq!(report.completion_writer_gate.ready_records, 1);
    assert_eq!(
        report.continuation_plan.source,
        "completion_resulting_queue"
    );
    assert!(!report.continuation_plan.ready);
    assert!(store_write.applied, "{:?}", store_write.reason_codes);
    assert!(read.found);
    assert!(read.decoded);
    assert_eq!(read.queue.as_ref().unwrap().goals.len(), 0);
    assert!(trace_report.passed, "{:?}", trace_report.failures);
    assert!(summary.contains("self_goal_queue_completion ready=true completed=1 retained=0"));
    assert!(trace.contains("rust-norion-self-goal-queue-continuation-plan-v1"));
    assert!(trace.contains("rust-norion-evolution-goal-queue-store-write-v1"));
    assert!(!summary.contains("R97 English/Chinese/Rust coding service"));
    assert!(!trace.contains("R97 English/Chinese/Rust coding service"));

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn self_goal_queue_cli_completion_continuation_activates_retained_goal() {
    let dir = temp_asset_dir("self-goal-queue-cli-continuation");
    fs::create_dir_all(&dir).unwrap();
    let store_path = dir.join("queue.ndkv");
    let trace_path = dir.join("continuation-trace.jsonl");
    let first = sample_self_goal_queue_goal(
        10,
        "first raw queue objective must stay out of summaries",
        [
            EvolutionGoalEvidenceKind::CargoCheck,
            EvolutionGoalEvidenceKind::FocusedTests,
            EvolutionGoalEvidenceKind::TraceSchemaGate,
            EvolutionGoalEvidenceKind::OperatorApproval,
        ],
    );
    let second = sample_self_goal_queue_goal(
        20,
        "second raw queue objective must stay out of summaries",
        [
            EvolutionGoalEvidenceKind::CargoCheck,
            EvolutionGoalEvidenceKind::BenchmarkGate,
            EvolutionGoalEvidenceKind::TraceSchemaGate,
            EvolutionGoalEvidenceKind::OperatorApproval,
        ],
    );
    let second_goal_id = second.stable_id.clone();
    seed_self_goal_queue_store(
        &store_path,
        EvolutionGoalQueue::new(vec![second.clone(), first]),
    );

    let args = Args::parse(vec![
        "--self-goal-queue".to_owned(),
        "--self-goal-queue-store".to_owned(),
        store_path.display().to_string(),
        "--self-goal-queue-store-apply".to_owned(),
        "--self-goal-queue-evidence".to_owned(),
        "queue_index=0;kind=cargo_check;passed=true".to_owned(),
        "--self-goal-queue-evidence".to_owned(),
        "queue_index=0;kind=focused_tests;passed=true;items=3;failures=0".to_owned(),
        "--self-goal-queue-evidence".to_owned(),
        "queue_index=0;kind=trace_schema_gate;passed=true".to_owned(),
        "--self-goal-queue-evidence".to_owned(),
        "queue_index=0;kind=operator_approval;passed=true;approval=true".to_owned(),
        "--trace-schema-gate".to_owned(),
        trace_path.display().to_string(),
    ]);

    let report = crate::cli::self_goal_queue::run_self_goal_queue_report(&args).unwrap();
    let trace_report = evaluate_trace_schema_jsonl(&trace_path).unwrap();
    let scope = TenantScope::new("local", "default", "interactive");
    let key = scope.scoped_key(TenantResourceLane::EvolutionGoalQueue, "pursuit");
    let store = EvolutionGoalQueueDiskStore::open(&store_path).unwrap();
    let read = store.read_queue(&scope, key.as_str()).unwrap();
    let summary = report.summary_lines().join("\n");
    let trace = fs::read_to_string(&trace_path).unwrap();

    assert!(report.completion_preview.ready);
    assert_eq!(report.completion_preview.completed_count, 1);
    assert_eq!(report.completion_preview.retained_count, 1);
    assert_eq!(
        report.queue_run.active_goal_id,
        Some(second_goal_id.clone())
    );
    assert_eq!(
        report.continuation_plan.source,
        "completion_resulting_queue"
    );
    assert!(report.continuation_plan.ready);
    assert_eq!(report.continuation_plan.goal_count, 1);
    assert_eq!(
        report.continuation_plan.plan.active_goal_id,
        Some(second_goal_id.clone())
    );
    assert!(
        report
            .continuation_plan
            .plan
            .required_evidence
            .contains(&"benchmark_gate".to_owned())
    );
    assert!(
        !report
            .continuation_plan
            .plan
            .required_evidence
            .contains(&"focused_tests".to_owned())
    );
    assert!(read.found);
    assert!(read.decoded);
    let queue = read.queue.as_ref().expect("retained queue");
    assert_eq!(queue.goals.len(), 1);
    assert_eq!(queue.goals[0].stable_id, second_goal_id);
    assert!(trace_report.passed, "{:?}", trace_report.failures);
    assert!(summary.contains(
        "self_goal_queue_continuation source=completion_resulting_queue ready=true goals=1"
    ));
    assert!(trace.contains("rust-norion-self-goal-queue-continuation-plan-v1"));
    assert!(
        summary.contains("required=cargo_check|benchmark_gate|trace_schema_gate|operator_approval")
    );
    assert!(!summary.contains("raw queue objective"));
    assert!(!trace.contains("raw queue objective"));

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn self_goal_queue_cli_store_apply_remains_gated_without_admission_evidence() {
    let dir = temp_asset_dir("self-goal-queue-cli");
    fs::create_dir_all(&dir).unwrap();
    let store_path = dir.join("queue.ndkv");
    let trace_path = dir.join("self-goal-trace.jsonl");
    let args = Args::parse(vec![
        "--self-goal-queue".to_owned(),
        "--self-goal-queue-store".to_owned(),
        store_path.display().to_string(),
        "--self-goal-queue-store-apply".to_owned(),
        "--trace-schema-gate".to_owned(),
        trace_path.display().to_string(),
    ]);

    let report = crate::cli::self_goal_queue::run_self_goal_queue_report(&args).unwrap();
    let store_read = report.store_read.as_ref().expect("store read report");
    let store_write = report.store_write.as_ref().expect("store write report");
    let trace_report = evaluate_trace_schema_jsonl(&trace_path).unwrap();
    let trace = fs::read_to_string(&trace_path).unwrap();

    assert!(!store_read.found);
    assert!(!store_read.applied);
    assert!(!report.append_execution.applied);
    assert!(!store_write.applied);
    assert!(!store_write.write_allowed);
    assert!(trace_report.passed, "{:?}", trace_report.failures);
    assert!(trace.contains("rust-norion-self-goal-queue-apply-plan-v1"));
    assert!(trace.contains("rust-norion-self-goal-queue-append-execution-v1"));
    assert!(trace.contains("rust-norion-self-goal-queue-continuation-plan-v1"));
    assert!(trace.contains("rust-norion-evolution-goal-queue-store-write-v1"));
    assert!(!trace.contains("English/Chinese/Rust coding service"));

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn self_goal_queue_cli_uses_evidence_file_when_queue_is_clear() {
    let dir = temp_asset_dir("self-goal-queue-cli-evidence");
    fs::create_dir_all(&dir).unwrap();
    let store_path = dir.join("queue.ndkv");
    let evidence_path = dir.join("evidence.txt");
    let trace_path = dir.join("self-goal-evidence-trace.jsonl");
    seed_empty_self_goal_queue_store(&store_path);
    fs::write(
        &evidence_path,
        [
            "candidate_index=0;kind=cargo_check;passed=true",
            "candidate_index=0;kind=focused_tests;passed=true;items=3;failures=0",
            "candidate_index=0;kind=trace_schema_gate;passed=true",
            "candidate_index=0;kind=operator_approval;passed=true;approval=true",
        ]
        .join("\n"),
    )
    .unwrap();

    let args = Args::parse(vec![
        "--self-goal-queue".to_owned(),
        "--self-goal-queue-store".to_owned(),
        store_path.display().to_string(),
        "--self-goal-queue-store-apply".to_owned(),
        "--self-goal-queue-evidence-file".to_owned(),
        evidence_path.display().to_string(),
        "--trace-schema-gate".to_owned(),
        trace_path.display().to_string(),
    ]);

    let report = crate::cli::self_goal_queue::run_self_goal_queue_report(&args).unwrap();
    let trace_report = evaluate_trace_schema_jsonl(&trace_path).unwrap();
    let summary = report.summary_lines().join("\n");
    let trace = fs::read_to_string(&trace_path).unwrap();

    assert_eq!(report.current_goal_count, 0);
    assert!(report.current_queue_loaded_from_store);
    assert_eq!(report.evidence.packet_count, 4);
    assert_eq!(report.evidence.valid_packet_count, 4);
    assert_eq!(report.evidence.invalid_packet_count, 0);
    assert_eq!(report.evidence.run_count, 1);
    assert_eq!(report.evidence.evidence_count, 4);
    assert_eq!(report.evidence.approval_count, 1);
    assert_eq!(report.admission.preview_admissible_count, 1);
    assert_eq!(report.queue_preview.append_preview_count, 1);
    assert!(report.apply.explicit_apply_required);
    assert!(report.append_execution.applied);
    assert!(
        report
            .store_write
            .as_ref()
            .is_some_and(|store_write| store_write.applied)
    );
    assert!(trace_report.passed, "{:?}", trace_report.failures);
    assert!(summary.contains("self_goal_queue_evidence packets=4 valid=4 invalid=0"));
    assert!(summary.contains("redaction-digest:"));
    assert!(trace.contains("rust-norion-self-goal-queue-continuation-plan-v1"));
    assert!(trace.contains("rust-norion-self-goal-queue-append-execution-v1"));
    assert!(!summary.contains("R97 endpoint and CLI runner wiring"));
    assert!(!trace.contains("R97 endpoint and CLI runner wiring"));

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn self_goal_queue_cli_blocks_malformed_evidence_packets() {
    let dir = temp_asset_dir("self-goal-queue-cli-bad-evidence");
    fs::create_dir_all(&dir).unwrap();
    let store_path = dir.join("queue.ndkv");
    let evidence_path = dir.join("evidence.txt");
    seed_empty_self_goal_queue_store(&store_path);
    fs::write(
        &evidence_path,
        [
            "candidate_index=0;kind=cargo_check;passed=true",
            "candidate_index=0;kind=focused_tests;passed=true;items=3;failures=0",
            "candidate_index=0;kind=trace_schema_gate;passed=true",
            "candidate_index=0;kind=operator_approval;passed=true;approval=true",
            "candidate_index=0;kind=unknown_gate;passed=true",
        ]
        .join("\n"),
    )
    .unwrap();

    let args = Args::parse(vec![
        "--self-goal-queue".to_owned(),
        "--self-goal-queue-store".to_owned(),
        store_path.display().to_string(),
        "--self-goal-queue-evidence-file".to_owned(),
        evidence_path.display().to_string(),
    ]);

    let report = crate::cli::self_goal_queue::run_self_goal_queue_report(&args).unwrap();
    let summary = report.summary_lines().join("\n");

    assert_eq!(report.evidence.packet_count, 5);
    assert_eq!(report.evidence.valid_packet_count, 4);
    assert_eq!(report.evidence.invalid_packet_count, 1);
    assert_eq!(report.evidence.run_count, 0);
    assert_eq!(report.admission.preview_admissible_count, 0);
    assert_eq!(report.queue_preview.append_preview_count, 0);
    assert!(!report.append_execution.applied);
    assert!(summary.contains("self_goal_queue_evidence packets=5 valid=4 invalid=1"));
    assert!(!summary.contains("unknown_gate"));

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn self_goal_queue_cli_ignores_approval_flag_on_non_operator_evidence() {
    let dir = temp_asset_dir("self-goal-queue-cli-non-operator-approval");
    fs::create_dir_all(&dir).unwrap();
    let store_path = dir.join("queue.ndkv");
    let evidence_path = dir.join("evidence.txt");
    seed_empty_self_goal_queue_store(&store_path);
    fs::write(
        &evidence_path,
        [
            "candidate_index=0;kind=cargo_check;passed=true;approval=true",
            "candidate_index=0;kind=focused_tests;passed=true;items=3;failures=0",
            "candidate_index=0;kind=trace_schema_gate;passed=true",
            "candidate_index=0;kind=operator_approval;passed=true;approval=false",
        ]
        .join("\n"),
    )
    .unwrap();

    let args = Args::parse(vec![
        "--self-goal-queue".to_owned(),
        "--self-goal-queue-store".to_owned(),
        store_path.display().to_string(),
        "--self-goal-queue-store-apply".to_owned(),
        "--self-goal-queue-evidence-file".to_owned(),
        evidence_path.display().to_string(),
    ]);

    let report = crate::cli::self_goal_queue::run_self_goal_queue_report(&args).unwrap();

    assert_eq!(report.evidence.packet_count, 4);
    assert_eq!(report.evidence.valid_packet_count, 4);
    assert_eq!(report.evidence.invalid_packet_count, 0);
    assert_eq!(report.evidence.run_count, 1);
    assert_eq!(report.evidence.approval_count, 0);
    assert_eq!(report.admission.preview_admissible_count, 0);
    assert_eq!(
        report.admission.records[0].decision,
        SelfGoalAdmissionDecision::HeldForApproval
    );
    assert_eq!(report.queue_preview.append_preview_count, 0);
    assert!(!report.append_execution.applied);

    fs::remove_dir_all(dir).unwrap();
}

fn sample_self_goal_queue_goal(
    priority: u32,
    objective: &str,
    required_evidence: impl IntoIterator<Item = EvolutionGoalEvidenceKind>,
) -> EvolutionGoal {
    EvolutionGoal::new(
        priority,
        objective,
        EvolutionGoalSuccessGate::new(required_evidence),
        ["self-goal-cli-test", objective],
    )
}

fn seed_empty_self_goal_queue_store(path: &std::path::Path) {
    seed_self_goal_queue_store(path, EvolutionGoalQueue::new(Vec::new()));
}

fn seed_self_goal_queue_store(path: &std::path::Path, queue: EvolutionGoalQueue) {
    let scope = TenantScope::new("local", "default", "interactive");
    let key = scope.scoped_key(TenantResourceLane::EvolutionGoalQueue, "pursuit");
    let queue_digest = queue.redaction_digest();
    let rollback_anchor =
        stable_redaction_digest(["self-goal-cli-queue-test", queue_digest.as_str()]);
    let approval = EvolutionGoalQueueStoreApproval::for_queue(
        "operator:local",
        "ticket:self-goal-queue-cli",
        &key,
        &queue,
        &rollback_anchor,
    );
    let mut store = EvolutionGoalQueueDiskStore::open_with_policy(
        path,
        EvolutionGoalQueueStorePolicy::explicit_durable_write(),
    )
    .unwrap();
    let report = store
        .write_queue(&scope, &key, &queue, &rollback_anchor, Some(&approval))
        .unwrap();

    assert!(report.applied, "{:?}", report.reason_codes);
}
