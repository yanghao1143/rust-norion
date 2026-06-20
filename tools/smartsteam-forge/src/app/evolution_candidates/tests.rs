use super::{
    mark_evolution_candidate, render_evolution_candidate_apply_check,
    render_evolution_candidate_gate, render_evolution_candidate_list, render_evolution_candidates,
    run_evolution_candidate_gate_to, validate_evolution_candidate,
};
use crate::app::evolution_candidate_lifecycle::normalize_candidate_status;
use crate::app::evolution_candidate_model::{BACKLOG_FILE, LEDGER_FILE, REPORT_FILE};
use std::{
    env, fs, io,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn prefers_report_last_candidate_over_ledger() {
    let work_dir = temp_work_dir("prefers-report");
    fs::create_dir_all(&work_dir).unwrap();
    fs::write(
        work_dir.join(REPORT_FILE),
        r#"{
                "last": {
                    "round": 7,
                    "case": "smartsteam-evolution-loop-0007",
                    "runtime_model": "google/gemma-4-12B-it",
                    "runtime_tokens": 64,
                    "elapsed_ms": 1234,
                    "feedback_applied": 4,
                    "self_improve_passed": true,
                    "answer": "**Improvement Candidate:** report wins"
                }
            }"#,
    )
    .unwrap();
    fs::write(
        work_dir.join(LEDGER_FILE),
        r#"{"round":8,"case":"ledger","runtime_model":"ledger-model","runtime_tokens":1,"elapsed_ms":2,"feedback_applied":3,"self_improve_passed":false,"answer":"ledger fallback"}"#,
    )
    .unwrap();

    let output = render_evolution_candidates(&work_dir.to_string_lossy(), 5, None).unwrap();
    let _ = fs::remove_dir_all(&work_dir);

    assert!(output.contains("source=report.last"));
    assert!(output.contains("count=1 limit=5"));
    assert!(output.contains("round=7 case=smartsteam-evolution-loop-0007"));
    assert!(output.contains("model=google/gemma-4-12B-it tokens=64"));
    assert!(output.contains("elapsed_ms=1234 feedback=4 self_improve=true"));
    assert!(output.contains("answer_preview=**Improvement Candidate:** report wins"));
    assert!(!output.contains("ledger fallback"));
}

#[test]
fn reads_recent_ledger_candidates_newest_first_when_report_missing() {
    let work_dir = temp_work_dir("ledger-recent");
    fs::create_dir_all(&work_dir).unwrap();
    fs::write(
        work_dir.join(LEDGER_FILE),
        [
            r#"{"round":1,"case":"case-1","runtime_model":"model-a","runtime_tokens":11,"elapsed_ms":101,"feedback_applied":1,"self_improve_passed":true,"answer":"candidate one"}"#,
            r#"{"round":2,"case":"case-2","runtime_model":"model-b","runtime_tokens":22,"elapsed_ms":202,"feedback_applied":2,"self_improve_passed":false,"answer":"candidate two"}"#,
            r#"{"round":3,"case":"case-3","runtime_model":"model-c","runtime_tokens":33,"elapsed_ms":303,"feedback_applied":3,"self_improve_passed":true,"answer":"candidate three"}"#,
        ]
        .join("\n"),
    )
    .unwrap();

    let output = render_evolution_candidates(&work_dir.to_string_lossy(), 2, None).unwrap();
    let _ = fs::remove_dir_all(&work_dir);

    assert!(output.contains("source=ledger"));
    assert!(output.contains("count=2 limit=2 order=newest_first"));
    assert!(output.contains("round=3 case=case-3 model=model-c"));
    assert!(output.contains("round=2 case=case-2 model=model-b"));
    assert!(!output.contains("round=1 case=case-1"));
    assert!(output.find("round=3").unwrap() < output.find("round=2").unwrap());
}

#[test]
fn reports_empty_sources_without_writes() {
    let work_dir = temp_work_dir("missing");
    fs::create_dir_all(&work_dir).unwrap();

    let output = render_evolution_candidates(&work_dir.to_string_lossy(), 3, None).unwrap();
    let _ = fs::remove_dir_all(&work_dir);

    assert!(output.contains("source=none count=0 limit=3"));
    assert!(output.contains("read_only=true starts_process=false sends_prompt=false"));
    assert!(output.contains("writes_files=false"));
}

#[test]
fn saves_candidates_to_default_backlog_with_dedupe() {
    let work_dir = temp_work_dir("save-default");
    fs::create_dir_all(&work_dir).unwrap();
    fs::write(
        work_dir.join(REPORT_FILE),
        r#"{
                "last": {
                    "round": 9,
                    "case": "smartsteam-evolution-loop-0009",
                    "runtime_model": "google/gemma-4-12B-it",
                    "runtime_tokens": 64,
                    "elapsed_ms": 999,
                    "feedback_applied": 4,
                    "self_improve_passed": true,
                    "answer": "**Improvement Candidate:** persist this one"
                }
            }"#,
    )
    .unwrap();

    let first = render_evolution_candidates(&work_dir.to_string_lossy(), 5, Some("")).unwrap();
    let second = render_evolution_candidates(&work_dir.to_string_lossy(), 5, Some("")).unwrap();
    let backlog = fs::read_to_string(work_dir.join(BACKLOG_FILE)).unwrap();
    let _ = fs::remove_dir_all(&work_dir);

    assert!(first.contains("writes_files=true"));
    assert!(first.contains("backlog path="));
    assert!(first.contains("appended=1 skipped_duplicate=0"));
    assert!(second.contains("appended=0 skipped_duplicate=1"));
    assert_eq!(backlog.lines().count(), 1);
    assert!(backlog.contains("\"schema\":\"smartsteam.evolution_candidate.v1\""));
    assert!(backlog.contains("\"status\":\"new\""));
    assert!(backlog.contains("\"source\":\"report.last\""));
    assert!(backlog.contains("persist this one"));
}

#[test]
fn saves_ledger_candidates_to_custom_backlog() {
    let work_dir = temp_work_dir("save-custom");
    fs::create_dir_all(&work_dir).unwrap();
    let backlog_path = work_dir.join("nested").join("candidate-backlog.jsonl");
    fs::write(
        work_dir.join(LEDGER_FILE),
        [
            r#"{"round":1,"case":"case-1","runtime_model":"model-a","runtime_tokens":11,"elapsed_ms":101,"feedback_applied":1,"self_improve_passed":true,"answer":"candidate one"}"#,
            r#"{"round":2,"case":"case-2","runtime_model":"model-b","runtime_tokens":22,"elapsed_ms":202,"feedback_applied":2,"self_improve_passed":false,"answer":"candidate two"}"#,
        ]
        .join("\n"),
    )
    .unwrap();

    let backlog_text = backlog_path.to_string_lossy().to_string();
    let output =
        render_evolution_candidates(&work_dir.to_string_lossy(), 2, Some(&backlog_text)).unwrap();
    let backlog = fs::read_to_string(&backlog_path).unwrap();
    let _ = fs::remove_dir_all(&work_dir);

    assert!(output.contains("source=ledger"));
    assert!(output.contains("appended=2 skipped_duplicate=0"));
    assert_eq!(backlog.lines().count(), 2);
    assert!(backlog.contains("\"source\":\"ledger\""));
    assert!(backlog.contains("\"case\":\"case-2\""));
    assert!(backlog.contains("\"case\":\"case-1\""));
}

#[test]
fn marks_candidate_status_with_append_only_audit_event() {
    let work_dir = temp_work_dir("mark-status");
    fs::create_dir_all(&work_dir).unwrap();
    let backlog = work_dir.join(BACKLOG_FILE);
    fs::write(
        &backlog,
        r#"{"schema":"smartsteam.evolution_candidate.v1","candidate_id":"smartsteam-candidate-test","status":"new","source":"report.last","round":"1","case":"case-1","model":"model-a","tokens":"64","elapsed_ms":"100","feedback":"4","self_improve":"true","answer_preview":"candidate"}"#,
    )
    .unwrap();

    let output = mark_evolution_candidate(
        &work_dir.to_string_lossy(),
        None,
        "smartsteam-candidate-test",
        "accepted",
        Some("looks useful"),
    )
    .unwrap();
    let text = fs::read_to_string(&backlog).unwrap();
    let _ = fs::remove_dir_all(&work_dir);

    assert!(output.contains("previous_status=new"));
    assert!(output.contains("status=accepted"));
    assert!(output.contains("appended=true"));
    assert_eq!(text.lines().count(), 2);
    assert!(text.contains("\"schema\":\"smartsteam.evolution_candidate_status.v1\""));
    assert!(text.contains("\"candidate_id\":\"smartsteam-candidate-test\""));
    assert!(text.contains("\"status\":\"accepted\""));
    assert!(text.contains("\"note\":\"looks useful\""));
}

#[test]
fn lists_candidate_backlog_with_status_filter() {
    let work_dir = temp_work_dir("list-status");
    fs::create_dir_all(&work_dir).unwrap();
    let backlog = work_dir.join(BACKLOG_FILE);
    fs::write(
        &backlog,
        [
            r#"{"schema":"smartsteam.evolution_candidate.v1","candidate_id":"smartsteam-candidate-one","status":"new","source":"report.last","round":"1","case":"case-1","model":"model-a","tokens":"64","elapsed_ms":"100","feedback":"4","self_improve":"true","answer_preview":"first candidate"}"#,
            r#"{"schema":"smartsteam.evolution_candidate.v1","candidate_id":"smartsteam-candidate-two","status":"new","source":"ledger","round":"2","case":"case-2","model":"model-b","tokens":"32","elapsed_ms":"200","feedback":"2","self_improve":"false","answer_preview":"second candidate ready"}"#,
            r#"{"schema":"smartsteam.evolution_candidate_status.v1","candidate_id":"smartsteam-candidate-two","status":"accepted","note":"ready for implementation queue","changed_unix":123}"#,
        ]
        .join("\n"),
    )
    .unwrap();

    let output =
        render_evolution_candidate_list(&work_dir.to_string_lossy(), None, Some("accepted"), 5)
            .unwrap();
    let _ = fs::remove_dir_all(&work_dir);

    assert!(output.contains("SmartSteam evolution candidate backlog"));
    assert!(
        output
            .contains("read_only=true starts_process=false sends_prompt=false writes_files=false")
    );
    assert!(output.contains("status_filter=accepted total=2 matched=1 invalid=0 limit=5"));
    assert!(output.contains("id=smartsteam-candidate-two status=accepted round=2 case=case-2"));
    assert!(output.contains(
        "model=model-b source=ledger tokens=32 elapsed_ms=200 feedback=2 self_improve=false"
    ));
    assert!(output.contains("note=ready for implementation queue"));
    assert!(output.contains("changed_unix=123"));
    assert!(output.contains("answer_preview=second candidate ready"));
    assert!(!output.contains("smartsteam-candidate-one status=new"));
}

#[test]
fn candidate_list_reports_missing_backlog_and_rejects_bad_filter() {
    let work_dir = temp_work_dir("list-missing");
    fs::create_dir_all(&work_dir).unwrap();

    let output =
        render_evolution_candidate_list(&work_dir.to_string_lossy(), None, Some("all"), 2).unwrap();
    let bad_filter =
        render_evolution_candidate_list(&work_dir.to_string_lossy(), None, Some("maybe"), 2)
            .unwrap_err();
    let _ = fs::remove_dir_all(&work_dir);

    assert!(output.contains("backlog path="));
    assert!(output.contains("exists=false"));
    assert!(output.contains("status_filter=all total=0 matched=0 invalid=0 limit=2"));
    assert_eq!(bad_filter.kind(), io::ErrorKind::InvalidInput);
}

#[test]
fn apply_check_selects_next_accepted_candidate_and_suggests_validation() {
    let work_dir = temp_work_dir("apply-check-next");
    fs::create_dir_all(&work_dir).unwrap();
    fs::write(
        work_dir.join(BACKLOG_FILE),
        [
            r#"{"schema":"smartsteam.evolution_candidate.v1","candidate_id":"smartsteam-candidate-one","status":"new","source":"report.last","round":"1","case":"case-1","model":"model-a","tokens":"64","elapsed_ms":"100","feedback":"4","self_improve":"true","answer_preview":"candidate one"}"#,
            r#"{"schema":"smartsteam.evolution_candidate.v1","candidate_id":"smartsteam-candidate-two","status":"new","source":"report.last","round":"2","case":"case-2","model":"model-b","tokens":"61","elapsed_ms":"28313","feedback":"4","self_improve":"true","answer_preview":"Improvement candidate: Implement a memory_pressure_gate check before test-gate dispatch"}"#,
            r#"{"schema":"smartsteam.evolution_candidate_status.v1","candidate_id":"smartsteam-candidate-two","status":"accepted","note":"ready","changed_unix":456}"#,
        ]
        .join("\n"),
    )
    .unwrap();

    let output =
        render_evolution_candidate_apply_check(&work_dir.to_string_lossy(), None, "next").unwrap();
    let _ = fs::remove_dir_all(&work_dir);

    assert!(output.contains("SmartSteam evolution candidate apply check"));
    assert!(
        output
            .contains("read_only=true starts_process=false sends_prompt=false writes_files=false")
    );
    assert!(output.contains("candidate_selector=next"));
    assert!(output.contains("candidate_id=smartsteam-candidate-two status=accepted apply_ready=true status_gate=pass block_reason=none"));
    assert!(output.contains("round=2 case=case-2 model=model-b source=report.last"));
    assert!(output.contains("suggested_scope=tools/evolution-loop,tools/smartsteam-forge"));
    assert!(output.contains("suggested_validation_command=cargo test -q --manifest-path tools/evolution-loop/Cargo.toml && cargo test -q --manifest-path tools/smartsteam-forge/Cargo.toml"));
    assert!(output.contains("suggested_next_status=implemented after validation evidence; rejected if scope risk is too high"));
    assert!(output.contains("note=ready"));
}

#[test]
fn apply_check_blocks_non_accepted_candidate_without_writing() {
    let work_dir = temp_work_dir("apply-check-block");
    fs::create_dir_all(&work_dir).unwrap();
    fs::write(
        work_dir.join(BACKLOG_FILE),
        r#"{"schema":"smartsteam.evolution_candidate.v1","candidate_id":"smartsteam-candidate-new","status":"new","source":"ledger","round":"1","case":"case-1","model":"model-a","tokens":"64","elapsed_ms":"100","feedback":"4","self_improve":"true","answer_preview":"candidate backlog listing"}"#,
    )
    .unwrap();

    let output = render_evolution_candidate_apply_check(
        &work_dir.to_string_lossy(),
        None,
        "smartsteam-candidate-new",
    )
    .unwrap();
    let text = fs::read_to_string(work_dir.join(BACKLOG_FILE)).unwrap();
    let _ = fs::remove_dir_all(&work_dir);

    assert!(output.contains("candidate_id=smartsteam-candidate-new status=new apply_ready=false status_gate=blocked block_reason=candidate_status_not_accepted"));
    assert!(output.contains("suggested_scope=tools/smartsteam-forge"));
    assert_eq!(text.lines().count(), 1);
}

#[test]
fn records_candidate_validation_evidence_append_only() {
    let work_dir = temp_work_dir("validate-evidence");
    fs::create_dir_all(&work_dir).unwrap();
    let backlog = work_dir.join(BACKLOG_FILE);
    fs::write(
        &backlog,
        r#"{"schema":"smartsteam.evolution_candidate.v1","candidate_id":"smartsteam-candidate-validated","status":"new","source":"report.last","round":"1","case":"case-1","model":"model-a","tokens":"64","elapsed_ms":"100","feedback":"4","self_improve":"true","answer_preview":"candidate backlog validation"}"#,
    )
    .unwrap();

    let output = validate_evolution_candidate(
        &work_dir.to_string_lossy(),
        None,
        "smartsteam-candidate-validated",
        "cargo test -q --manifest-path tools/smartsteam-forge/Cargo.toml",
        "0",
        Some("green"),
    )
    .unwrap();
    let list = render_evolution_candidate_list(&work_dir.to_string_lossy(), None, None, 5).unwrap();
    let text = fs::read_to_string(&backlog).unwrap();
    let _ = fs::remove_dir_all(&work_dir);

    assert!(output.contains("SmartSteam evolution candidate validation"));
    assert!(
        output
            .contains("read_only=false starts_process=false sends_prompt=false writes_files=true")
    );
    assert!(output.contains("candidate_id=smartsteam-candidate-validated"));
    assert!(output.contains("validation_status_code=0"));
    assert!(output.contains("validation_passed=true"));
    assert!(output.contains("appended=true"));
    assert_eq!(text.lines().count(), 2);
    assert!(text.contains("\"schema\":\"smartsteam.evolution_candidate_validation.v1\""));
    assert!(text.contains(
        "\"command\":\"cargo test -q --manifest-path tools/smartsteam-forge/Cargo.toml\""
    ));
    assert!(text.contains("\"status_code\":0"));
    assert!(text.contains("\"passed\":true"));
    assert!(text.contains("\"note\":\"green\""));
    assert!(list.contains("validation_passed=true validation_status_code=0"));
    assert!(list.contains(
        "validation_command=cargo test -q --manifest-path tools/smartsteam-forge/Cargo.toml"
    ));
    assert!(list.contains("validation_note=green"));
}

#[test]
fn candidate_gate_passes_for_validated_implemented_backlog() {
    let work_dir = temp_work_dir("gate-passes");
    fs::create_dir_all(&work_dir).unwrap();
    fs::write(
        work_dir.join(BACKLOG_FILE),
        [
            r#"{"schema":"smartsteam.evolution_candidate.v1","candidate_id":"smartsteam-candidate-validated","status":"new","source":"report.last","round":"1","case":"case-1","model":"model-a","tokens":"64","elapsed_ms":"100","feedback":"4","self_improve":"true","answer_preview":"candidate"}"#,
            r#"{"schema":"smartsteam.evolution_candidate_status.v1","candidate_id":"smartsteam-candidate-validated","status":"implemented","note":"done","changed_unix":123}"#,
            r#"{"schema":"smartsteam.evolution_candidate_validation.v1","candidate_id":"smartsteam-candidate-validated","command":"cargo test","status_code":0,"passed":true,"note":"green","validated_unix":456}"#,
        ]
        .join("\n"),
    )
    .unwrap();

    let (output, ready) =
        render_evolution_candidate_gate(&work_dir.to_string_lossy(), None).unwrap();
    let _ = fs::remove_dir_all(&work_dir);

    assert!(ready);
    assert!(output.contains("SmartSteam evolution candidate gate"));
    assert!(
        output
            .contains("read_only=true starts_process=false sends_prompt=false writes_files=false")
    );
    assert!(output.contains("candidate_lifecycle ready=true accepted_pending=0 implemented_validated=1 implemented_unvalidated=0 implemented_failed=0"));
}

#[test]
fn candidate_gate_blocks_pending_unvalidated_failed_and_invalid_records() {
    let work_dir = temp_work_dir("gate-blocks");
    fs::create_dir_all(&work_dir).unwrap();
    fs::write(
        work_dir.join(BACKLOG_FILE),
        [
            r#"{"schema":"smartsteam.evolution_candidate.v1","candidate_id":"smartsteam-candidate-accepted","status":"new","source":"report.last","round":"1","case":"case-1","model":"model-a","tokens":"64","elapsed_ms":"100","feedback":"4","self_improve":"true","answer_preview":"accepted"}"#,
            r#"{"schema":"smartsteam.evolution_candidate_status.v1","candidate_id":"smartsteam-candidate-accepted","status":"accepted","note":"todo","changed_unix":111}"#,
            r#"{"schema":"smartsteam.evolution_candidate.v1","candidate_id":"smartsteam-candidate-unvalidated","status":"implemented","source":"report.last","round":"2","case":"case-2","model":"model-b","tokens":"64","elapsed_ms":"100","feedback":"4","self_improve":"true","answer_preview":"unvalidated"}"#,
            r#"{"schema":"smartsteam.evolution_candidate.v1","candidate_id":"smartsteam-candidate-failed","status":"implemented","source":"report.last","round":"3","case":"case-3","model":"model-c","tokens":"64","elapsed_ms":"100","feedback":"4","self_improve":"true","answer_preview":"failed"}"#,
            r#"{"schema":"smartsteam.evolution_candidate_validation.v1","candidate_id":"smartsteam-candidate-failed","command":"cargo test","status_code":1,"passed":false,"note":"red","validated_unix":222}"#,
            "not json",
        ]
        .join("\n"),
    )
    .unwrap();

    let (output, ready) =
        render_evolution_candidate_gate(&work_dir.to_string_lossy(), None).unwrap();
    let mut buffer = Vec::new();
    let gate_result =
        run_evolution_candidate_gate_to(&work_dir.to_string_lossy(), None, &mut buffer);
    let _ = fs::remove_dir_all(&work_dir);

    assert!(!ready);
    assert!(output.contains("candidate_lifecycle ready=false accepted_pending=1 implemented_validated=0 implemented_unvalidated=1 implemented_failed=1"));
    assert!(output.contains("accepted_pending_ids=smartsteam-candidate-accepted"));
    assert!(output.contains("implemented_unvalidated_ids=smartsteam-candidate-unvalidated"));
    assert!(output.contains("implemented_failed_ids=smartsteam-candidate-failed"));
    assert!(output.contains("invalid_records=1"));
    assert_eq!(gate_result.unwrap_err().kind(), io::ErrorKind::Other);
    let text = String::from_utf8(buffer).unwrap();
    assert!(text.contains("SmartSteam evolution candidate gate"));
}

#[test]
fn rejects_candidate_validation_for_unknown_candidate_or_bad_status() {
    let work_dir = temp_work_dir("validate-rejects");
    fs::create_dir_all(&work_dir).unwrap();
    fs::write(work_dir.join(BACKLOG_FILE), "").unwrap();

    let missing = validate_evolution_candidate(
        &work_dir.to_string_lossy(),
        None,
        "smartsteam-candidate-missing",
        "cargo test",
        "0",
        None,
    )
    .unwrap_err();
    let bad_status = validate_evolution_candidate(
        &work_dir.to_string_lossy(),
        None,
        "smartsteam-candidate-missing",
        "cargo test",
        "ok",
        None,
    )
    .unwrap_err();
    let _ = fs::remove_dir_all(&work_dir);

    assert_eq!(missing.kind(), io::ErrorKind::NotFound);
    assert_eq!(bad_status.kind(), io::ErrorKind::InvalidInput);
}

#[test]
fn rejects_mark_for_unknown_candidate_or_status() {
    let work_dir = temp_work_dir("mark-rejects");
    fs::create_dir_all(&work_dir).unwrap();
    fs::write(work_dir.join(BACKLOG_FILE), "").unwrap();

    let missing = mark_evolution_candidate(
        &work_dir.to_string_lossy(),
        None,
        "smartsteam-candidate-missing",
        "accepted",
        None,
    )
    .unwrap_err();
    let bad_status =
        normalize_candidate_status("maybe").expect_err("unsupported status should fail");
    let _ = fs::remove_dir_all(&work_dir);

    assert_eq!(missing.kind(), io::ErrorKind::NotFound);
    assert_eq!(bad_status.kind(), io::ErrorKind::InvalidInput);
}

fn temp_work_dir(name: &str) -> PathBuf {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    env::temp_dir().join(format!(
        "smartsteam-forge-evolution-candidates-{name}-{}-{now}",
        std::process::id()
    ))
}
