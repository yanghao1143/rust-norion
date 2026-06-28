use super::*;

#[test]
fn coding_service_eval_runner_cli_writes_digest_only_trace_feed() {
    let dir = temp_asset_dir("coding-service-eval-runner-cli");
    fs::create_dir_all(&dir).unwrap();
    let trace_path = dir.join("coding-service-eval.jsonl");
    let trace_gate_path = dir.join("coding-service-eval-gate.jsonl");
    let args = Args::parse(vec![
        "--coding-service-eval-runner".to_owned(),
        "--trace".to_owned(),
        trace_path.display().to_string(),
        "--trace-schema-gate".to_owned(),
        trace_gate_path.display().to_string(),
    ]);

    let passed = crate::cli::coding_service_eval::run_coding_service_eval_runner_cli(&args)
        .expect("coding service eval runner");
    let trace_report = evaluate_trace_schema_jsonl(&trace_path).unwrap();
    let trace_gate_report = evaluate_trace_schema_jsonl(&trace_gate_path).unwrap();
    let trace = fs::read_to_string(&trace_path).unwrap();
    let trace_gate = fs::read_to_string(&trace_gate_path).unwrap();

    assert!(passed);
    assert!(trace_report.passed, "{:?}", trace_report.failures);
    assert!(trace_gate_report.passed, "{:?}", trace_gate_report.failures);
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
    assert_eq!(
        trace_gate_report.coding_service_eval_events,
        trace_report.coding_service_eval_events
    );
    assert_eq!(
        trace_gate_report.coding_service_eval_runner_events,
        trace_report.coding_service_eval_runner_events
    );
    assert_eq!(
        trace_gate_report.coding_service_eval_requests,
        trace_report.coding_service_eval_requests
    );
    assert_eq!(
        trace_gate_report.coding_service_eval_completed,
        trace_report.coding_service_eval_completed
    );
    assert!(trace.contains("rust-norion-coding-service-eval-readiness-v1"));
    assert!(trace_gate.contains("rust-norion-coding-service-eval-readiness-v1"));
    assert!(trace.contains("\"report_kind\":\"runner\""));
    assert!(trace_gate.contains("\"report_kind\":\"runner\""));
    assert!(
        trace.contains("\"result_classes\":[\"failed\",\"passed\",\"runner_contract_failed\"]")
    );
    assert!(trace.contains("\"failure_classes\":[]"));
    assert!(trace.contains("redaction-digest:"));
    assert!(!trace.contains("\"messages\""));
    assert!(!trace.contains("\"evidence_packets\""));
    assert!(!trace.contains("\"run_records\""));
    assert!(!trace.contains("fn parse_port"));
    assert!(!trace.contains("借用 和 所有权"));

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn coding_service_eval_readiness_cli_can_share_trace_gate_path() {
    let dir = temp_asset_dir("coding-service-eval-readiness-cli");
    fs::create_dir_all(&dir).unwrap();
    let trace_path = dir.join("coding-service-eval-readiness.jsonl");
    let args = Args::parse(vec![
        "--coding-service-eval-readiness".to_owned(),
        "--trace-schema-gate".to_owned(),
        trace_path.display().to_string(),
    ]);

    let passed = crate::cli::coding_service_eval::run_coding_service_eval_readiness_cli(&args)
        .expect("coding service eval readiness");
    let trace_report = evaluate_trace_schema_jsonl(&trace_path).unwrap();
    let trace = fs::read_to_string(&trace_path).unwrap();

    assert!(passed);
    assert!(trace_report.passed, "{:?}", trace_report.failures);
    assert_eq!(trace_report.coding_service_eval_events, 1);
    assert_eq!(trace_report.coding_service_eval_readiness_events, 1);
    assert_eq!(trace_report.coding_service_eval_runner_events, 0);
    assert_eq!(trace_report.coding_service_eval_requests, 5);
    assert_eq!(trace_report.coding_service_eval_completed, 0);
    assert_eq!(trace_report.coding_service_eval_evidence_packets, 5);
    assert_eq!(trace_report.coding_service_eval_rust_validation_checked, 0);
    assert!(trace.contains("\"report_kind\":\"readiness\""));
    assert!(!trace.contains("\"request_evidence_packets\""));
    assert!(!trace.contains("\"prompt\""));

    fs::remove_dir_all(dir).unwrap();
}
