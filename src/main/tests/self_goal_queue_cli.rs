use super::*;
use std::fs;

#[test]
fn self_goal_queue_cli_preview_holds_default_active_goal() {
    let args = Args::parse(vec!["--self-goal-queue".to_owned()]);
    let report = crate::cli::self_goal_queue::run_self_goal_queue_report(&args).unwrap();
    let summary = report.summary_lines().join("\n");

    assert_eq!(report.current_goal_count, 1);
    assert!(!report.current_queue_loaded_from_store);
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
    assert!(trace.contains("rust-norion-evolution-goal-queue-store-write-v1"));
    assert!(!trace.contains("English/Chinese/Rust coding service"));

    fs::remove_dir_all(dir).unwrap();
}
