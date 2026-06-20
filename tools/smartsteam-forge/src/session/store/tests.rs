use std::fs;

use super::*;

#[test]
fn writes_jsonl_transcript_and_rotates_session() {
    let root = std::env::temp_dir().join(format!(
        "smartsteam_store_test_{}_{}",
        unix_timestamp_millis(),
        std::process::id()
    ));

    let mut store = SessionStore::open(&root).unwrap();
    let first_path = store.transcript_path().to_path_buf();
    store.append_message("user", "hello").unwrap();
    store.append_message("assistant", "hi").unwrap();

    let content = fs::read_to_string(&first_path).unwrap();
    assert!(content.contains("\"kind\":\"message\""));
    assert!(content.contains("\"role\":\"assistant\""));

    let second = store.rotate().unwrap();
    assert_ne!(second.transcript_path, first_path);
    assert!(second.transcript_path.exists());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn lists_recent_sessions_with_prompt_and_answer_preview() {
    let root = std::env::temp_dir().join(format!(
        "smartsteam_list_test_{}_{}",
        unix_timestamp_millis(),
        std::process::id()
    ));

    let mut first = SessionStore::open(&root).unwrap();
    let first_id = first.current().id.clone();
    first.append_message("user", "first prompt").unwrap();
    first.append_message("assistant", "first answer").unwrap();
    let second = first.rotate().unwrap();
    first
        .append_event("preflight", "require_safe_device=true ok")
        .unwrap();
    first.append_message("user", "second prompt").unwrap();
    first
        .append_event(
            "final_payload",
            "{\"ok\":true,\"business_cycle\":{\"passed\":false,\"feedback_applied\":0},\"generate\":{\"runtime_model\":\"mock\",\"runtime_token_count\":8,\"answer\":\"second answer\"}}",
        )
        .unwrap();
    first
        .append_event("gate_report", "Business-cycle gate report\noverall: FAIL")
        .unwrap();

    let records = first.list_recent(5).unwrap();

    assert!(records.iter().any(|record| record.id == first_id));
    assert_eq!(records[0].id, second.id);
    assert_eq!(records[0].first_user.as_deref(), Some("second prompt"));
    assert_eq!(records[0].preflight_count, 1);
    assert_eq!(records[0].final_payload_count, 1);
    assert!(
        records[0]
            .latest_final_status
            .as_deref()
            .unwrap()
            .contains("runtime_model=mock")
    );
    assert_eq!(records[0].gate_report_count, 1);
    assert!(
        records[0]
            .summary_line()
            .contains("preflights=1 final_payloads=1 gate=FAIL gate_reports=1")
    );
    assert!(
        records
            .iter()
            .any(|record| record.last_assistant.as_deref() == Some("first answer"))
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn filters_recent_sessions_by_gate_outcome() {
    let root = std::env::temp_dir().join(format!(
        "smartsteam_filter_test_{}_{}",
        unix_timestamp_millis(),
        std::process::id()
    ));

    let mut store = SessionStore::open(&root).unwrap();
    let failed = store.current().id.clone();
    store
        .append_event("gate_report", "Business-cycle gate report\noverall: FAIL")
        .unwrap();
    let passed = store.rotate().unwrap().id;
    store
        .append_event("gate_report", "Business-cycle gate report\noverall: PASS")
        .unwrap();
    let no_gate = store.rotate().unwrap().id;

    let failed_records = store
        .list_recent_filtered(SessionFilter::Failed, 10)
        .unwrap();
    let passed_records = store
        .list_recent_filtered(SessionFilter::Passed, 10)
        .unwrap();
    let all_records = store.list_recent_filtered(SessionFilter::All, 10).unwrap();

    assert_eq!(failed_records.len(), 1);
    assert_eq!(failed_records[0].id, failed);
    assert_eq!(passed_records.len(), 1);
    assert_eq!(passed_records[0].id, passed);
    assert!(all_records.iter().any(|record| record.id == no_gate));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn resumes_session_by_recent_index_into_recent_messages() {
    let root = std::env::temp_dir().join(format!(
        "smartsteam_resume_test_{}_{}",
        unix_timestamp_millis(),
        std::process::id()
    ));

    let mut store = SessionStore::open(&root).unwrap();
    store.append_message("user", "first prompt").unwrap();
    store.append_message("assistant", "first answer").unwrap();
    let second = store.rotate().unwrap();
    store.append_message("user", "second prompt").unwrap();
    store.append_message("assistant", "second answer").unwrap();

    let resumed = store.resume("1", 3).unwrap();

    assert_eq!(resumed.record.id, second.id);
    assert_eq!(store.current().id, second.id);
    assert_eq!(resumed.messages.len(), 2);
    assert_eq!(resumed.messages[0].content, "second prompt");
    assert_eq!(resumed.messages[1].role, "assistant");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn summarizes_session_to_markdown_file() {
    let root = std::env::temp_dir().join(format!(
        "smartsteam_summary_test_{}_{}",
        unix_timestamp_millis(),
        std::process::id()
    ));

    let store = SessionStore::open(&root).unwrap();
    store.append_message("user", "summarize this").unwrap();
    store.append_message("assistant", "summary answer").unwrap();
    store
        .append_event("health_check", "service=mock ok=true")
        .unwrap();
    store
        .append_event("preflight", "require_safe_device=true service=mock ok=true")
        .unwrap();
    store
        .append_event("diagnostic_report", "SmartSteam Forge doctor\nhealth: PASS")
        .unwrap();
    store
        .append_event(
            "final_payload",
            "{\"ok\":true,\"business_cycle\":{\"passed\":true,\"feedback_applied\":1,\"rust_check_feedback_applied\":1},\"generate\":{\"runtime_model\":\"gemma\",\"runtime_token_count\":21,\"answer\":\"summary answer\"}}",
        )
        .unwrap();
    store
        .append_event("gate_report", "Business-cycle gate report\noverall: PASS")
        .unwrap();

    let summary = store.summarize("").unwrap();

    assert_eq!(summary.message_count, 2);
    assert_eq!(summary.user_count, 1);
    assert_eq!(summary.assistant_count, 1);
    assert_eq!(summary.health_check_count, 1);
    assert_eq!(summary.preflight_count, 1);
    assert_eq!(summary.diagnostic_count, 1);
    assert_eq!(summary.final_payload_count, 1);
    assert_eq!(summary.gate_report_count, 1);
    assert_eq!(summary.latest_user.as_deref(), Some("summarize this"));
    assert!(
        summary
            .latest_preflight
            .as_deref()
            .unwrap()
            .contains("require_safe_device=true")
    );
    assert!(
        summary
            .latest_final_status
            .as_deref()
            .unwrap()
            .contains("runtime_model=gemma")
    );
    assert!(
        summary
            .latest_gate_report
            .as_deref()
            .unwrap()
            .contains("overall: PASS")
    );
    assert!(summary.summary_path.exists());
    let markdown = fs::read_to_string(&summary.summary_path).unwrap();
    assert!(markdown.contains("SmartSteam Forge Session Summary"));
    assert!(markdown.contains("Health checks: 1"));
    assert!(markdown.contains("Latest Preflight"));
    assert!(markdown.contains("Latest Final Payload"));
    assert!(markdown.contains("runtime_model=gemma"));
    assert!(markdown.contains("summary answer"));
    assert!(markdown.contains("overall: PASS"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn summarizes_current_session_after_resume() {
    let root = std::env::temp_dir().join(format!(
        "smartsteam_current_summary_test_{}_{}",
        unix_timestamp_millis(),
        std::process::id()
    ));

    let store = SessionStore::open(&root).unwrap();
    store.append_message("user", "resume me").unwrap();
    store.append_message("assistant", "resumed answer").unwrap();

    let summary = store.summarize_current().unwrap();

    assert_eq!(summary.user_count, 1);
    assert_eq!(summary.assistant_count, 1);
    assert!(summary.to_context_prompt().contains("resume me"));
    assert!(summary.summary_path.exists());

    let _ = fs::remove_dir_all(root);
}
