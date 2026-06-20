use super::*;

#[test]
fn rust_check_trace_schema_accepts_compiler_feedback_event() {
    let report = RustSnippetCheckReport {
        passed: true,
        edition: "2021".to_owned(),
        status_code: Some(0),
        stdout: String::new(),
        stderr: String::new(),
        source_path: PathBuf::from("target/model-service-rust-check/lib.rs"),
        metadata_path: PathBuf::from("target/model-service-rust-check/check.rmeta"),
    };
    let updates = vec![MemoryUpdateReport::applied(
        7,
        MemoryUpdateAction::Reinforce,
        0.45,
        1.0,
        1.45,
        false,
    )];

    let line = rust_check_trace_json_line(
        Some("compiler-feedback"),
        &report,
        RewardAction::Reinforce,
        0.45,
        Some(3),
        None,
        &[7],
        &updates,
    );
    let failures = evaluate_trace_schema_line(&line);

    assert!(line.contains("\"schema\":\"rust-norion-rust-check-v1\""));
    assert!(line.contains("\"case\":\"compiler-feedback\""));
    assert!(line.contains("\"memory_ids\":[7]"));
    assert!(failures.is_empty(), "{failures:?}");

    let path = temp_path("rust-check-trace-schema");
    fs::write(&path, format!("{line}\n")).unwrap();
    let report = evaluate_trace_schema_jsonl(&path).unwrap();

    assert!(report.passed, "{:?}", report.failures);
    assert_eq!(report.checked_lines, 1);
    assert_eq!(report.rust_check_events, 1);
    assert_eq!(report.rust_check_passed, 1);
    assert_eq!(report.rust_check_failed, 0);
    assert_eq!(report.rust_check_feedback_updates, 1);
    assert_eq!(report.rust_check_feedback_applied, 1);
    assert!(report.summary_line().contains("rust_check_events=1"));
    cleanup(path);
}

#[test]
fn rust_check_trace_schema_rejects_feedback_contract_mismatch() {
    let report = RustSnippetCheckReport {
        passed: true,
        edition: "2021".to_owned(),
        status_code: Some(0),
        stdout: String::new(),
        stderr: String::new(),
        source_path: PathBuf::from("target/model-service-rust-check/lib.rs"),
        metadata_path: PathBuf::from("target/model-service-rust-check/check.rmeta"),
    };
    let line = rust_check_trace_json_line(
        Some("compiler-feedback"),
        &report,
        RewardAction::Reinforce,
        0.45,
        Some(3),
        None,
        &[7],
        &[],
    )
    .replacen(
        "\"label\":\"rustc_passed\"",
        "\"label\":\"rustc_failed\"",
        1,
    )
    .replacen("\"action\":\"reinforce\"", "\"action\":\"penalize\"", 1);
    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("requires label rustc_passed")),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("passed checks must not penalize")),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("applied+missing 0 does not match memory_ids 1")),
        "{failures:?}"
    );
}

#[test]
fn business_contract_trace_schema_accepts_audit_event() {
    let line = business_contract_trace_json_line(
        "gemma-service-rust-feedback",
        Some(9),
        4,
        4,
        &[],
        true,
        false,
        false,
        false,
        true,
        true,
        "raw_direct",
        false,
        false,
    );
    let failures = evaluate_trace_schema_line(&line);

    assert!(line.contains("\"schema\":\"rust-norion-business-contract-v1\""));
    assert!(line.contains("\"case\":\"gemma-service-rust-feedback\""));
    assert!(line.contains("\"experience_id\":9"));
    assert!(failures.is_empty(), "{failures:?}");

    let path = temp_path("business-contract-trace-schema");
    fs::write(&path, format!("{line}\n")).unwrap();
    let report = evaluate_trace_schema_jsonl(&path).unwrap();

    assert!(report.passed, "{:?}", report.failures);
    assert_eq!(report.checked_lines, 1);
    assert_eq!(report.business_contract_events, 1);
    assert_eq!(report.business_contract_event_passed, 1);
    assert_eq!(report.business_contract_event_failed, 0);
    assert_eq!(report.business_contract_event_missing_signals, 0);
    assert_eq!(report.business_contract_event_raw_passed, 1);
    assert_eq!(report.business_contract_event_raw_failed, 0);
    assert_eq!(report.business_contract_event_response_normalized, 0);
    assert_eq!(report.business_contract_event_canonical_fallbacks, 0);
    assert!(report.summary_line().contains("business_contract_events=1"));
    cleanup(path);
}

#[test]
fn business_contract_trace_schema_rejects_false_pass() {
    let line = business_contract_trace_json_line(
        "gemma-service-rust-feedback",
        Some(9),
        4,
        3,
        &["to memory".to_owned()],
        true,
        false,
        false,
        false,
        true,
        false,
        "canonical_fallback",
        true,
        true,
    )
    .replacen("\"passed\":false", "\"passed\":true", 1);
    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("passed=true requires no missing signals")),
        "{failures:?}"
    );
}
