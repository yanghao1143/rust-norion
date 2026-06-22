use super::*;
use crate::improvement_corpus::{
    ImprovementCorpus, ImprovementEpisodeClass, ImprovementEpisodeInput, ImprovementEvidenceLane,
    ImprovementValidationStatus,
};

fn improvement_corpus_line() -> String {
    let mut corpus = ImprovementCorpus::new("trace-self-training-corpus");
    corpus.push_episode(
        ImprovementEpisodeInput::accepted("trace-accepted")
            .with_task_label("rust coding compiler test benchmark fix")
            .with_patch_summary("sanitized patch summary")
            .with_evidence_id("compiler:passed")
            .with_evidence_id("tests:passed")
            .with_evidence_id("benchmark:won")
            .with_source_trace_id("trace:rust-check:accepted"),
    );
    corpus.push_episode(
        ImprovementEpisodeInput::new("trace-flaky", ImprovementEpisodeClass::Flaky)
            .with_validation_status(ImprovementValidationStatus::Flaky)
            .with_compiler(ImprovementEvidenceLane::new(1, 1, 0, 0))
            .with_tests(ImprovementEvidenceLane::new(2, 1, 0, 1))
            .with_benchmarks(ImprovementEvidenceLane::new(1, 1, 0, 0))
            .with_rollback_anchor("rollback:flaky")
            .with_rollback_replayed(true),
    );
    corpus.report().json_line()
}

#[test]
fn improvement_corpus_trace_schema_accepts_preview_only_report() {
    let line = improvement_corpus_line();
    let failures = evaluate_trace_schema_line(&line);

    assert!(line.contains("\"schema\":\"rust-norion-improvement-corpus-v1\""));
    assert!(line.contains("\"dataset_export_enabled\":false"));
    assert!(line.contains("\"raw_prompt_payloads_stored\":0"));
    assert!(line.contains("\"secret_leaks\":0"));
    assert!(failures.is_empty(), "{failures:?}");

    let path = temp_path("improvement-corpus-trace-schema");
    fs::write(&path, format!("{line}\n")).unwrap();
    let report = evaluate_trace_schema_jsonl(&path).unwrap();

    assert!(report.passed, "{:?}", report.failures);
    assert_eq!(report.checked_lines, 1);
    assert_eq!(report.improvement_corpus_events, 1);
    assert_eq!(report.improvement_corpus_episodes, 2);
    assert_eq!(report.improvement_corpus_active_adaptation, 1);
    assert_eq!(report.improvement_corpus_compiler_passed, 2);
    assert_eq!(report.improvement_corpus_test_passed, 2);
    assert_eq!(report.improvement_corpus_benchmark_passed, 2);
    assert_eq!(report.improvement_corpus_secret_leaks, 0);
    assert!(
        report
            .summary_line()
            .contains("improvement_corpus_active_adaptation=1")
    );
    cleanup(path);
}

#[test]
fn improvement_corpus_trace_append_is_gate_consumable() {
    let mut corpus = ImprovementCorpus::new("trace-append-corpus");
    corpus.push_episode(ImprovementEpisodeInput::accepted("trace-append-accepted"));
    let report = corpus.report();

    let path = temp_path("improvement-corpus-trace-append");
    append_improvement_corpus_trace_jsonl(&path, &report).unwrap();
    let gate = evaluate_trace_schema_jsonl(&path).unwrap();

    assert!(gate.passed, "{:?}", gate.failures);
    assert_eq!(gate.improvement_corpus_events, 1);
    assert_eq!(gate.improvement_corpus_episodes, 1);
    assert_eq!(gate.improvement_corpus_active_adaptation, 1);
    cleanup(path);
}

#[test]
fn improvement_corpus_trace_schema_rejects_count_mismatch_and_export() {
    let line = improvement_corpus_line()
        .replacen("\"accepted\":1", "\"accepted\":3", 1)
        .replacen(
            "\"dataset_export_enabled\":false",
            "\"dataset_export_enabled\":true",
            1,
        );
    let failures = evaluate_trace_schema_line(&line);

    assert!(failures.iter().any(|failure| {
        failure.contains("classified records") && failure.contains("does not match total")
    }));
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("dataset_export_enabled=true"))
    );
}

#[test]
fn improvement_corpus_trace_schema_rejects_active_without_validation_or_replay() {
    let line = improvement_corpus_line()
        .replacen("\"approved\":1", "\"approved\":0", 1)
        .replacen("\"passed\":1", "\"passed\":0", 1)
        .replacen("\"replayed\":2", "\"replayed\":0", 1);
    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("exceeds approved"))
    );
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("exceeds validation_passed"))
    );
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("exceeds rollback_replayed"))
    );
}

#[test]
fn improvement_corpus_trace_schema_rejects_raw_payload_or_secret_leak() {
    let line = improvement_corpus_line()
        .replacen(
            "\"raw_prompt_payloads_stored\":0",
            "\"raw_prompt_payloads_stored\":1",
            1,
        )
        .replacen("\"secret_leaks\":0", "\"secret_leaks\":1", 1);
    let failures = evaluate_trace_schema_line(&line);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("raw_prompt_payloads_stored must be 0"))
    );
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("secret_leaks must be 0"))
    );
}
