use std::time::Duration;

use smartsteam_forge::{SessionFilter, StreamEndpoint};

use super::*;

#[test]
fn parses_backend_url_without_scheme() {
    let config =
        CliConfig::parse(["--backend".to_owned(), "http://127.0.0.1:7878".to_owned()]).unwrap();

    assert_eq!(config.backend, "127.0.0.1:7878");
}

#[test]
fn parses_mock_mode() {
    let config = CliConfig::parse(["--mock".to_owned()]).unwrap();

    assert!(config.mock);
}

#[test]
fn parses_one_shot_prompt_aliases() {
    let prompt = "hello forge".to_owned();
    let config = CliConfig::parse(["--once".to_owned(), prompt.clone()]).unwrap();

    assert_eq!(config.prompt, Some(prompt));
}

#[test]
fn parses_smoke_prompt() {
    let config = CliConfig::parse(["--smoke".to_owned()]).unwrap();

    assert!(config.prompt.unwrap().contains("smoke-test"));
}

#[test]
fn parses_mode() {
    let config = CliConfig::parse(["--mode".to_owned(), "business-cycle".to_owned()]).unwrap();

    assert_eq!(config.endpoint, Some(StreamEndpoint::BusinessCycle));
}

#[test]
fn parses_health_check() {
    let config = CliConfig::parse(["--check".to_owned()]).unwrap();

    assert!(config.health_check);
}

#[test]
fn parses_experience_hygiene_check() {
    let config = CliConfig::parse(["--hygiene".to_owned()]).unwrap();

    assert!(config.experience_hygiene);
    assert!(!config.experience_hygiene_quarantine);
    assert_eq!(config.experience_hygiene_limit, 20);
}

#[test]
fn parses_experience_hygiene_quarantine_dry_run() {
    let config = CliConfig::parse([
        "--hygiene-quarantine".to_owned(),
        "--hygiene-limit".to_owned(),
        "7".to_owned(),
    ])
    .unwrap();

    assert!(config.experience_hygiene);
    assert!(config.experience_hygiene_quarantine);
    assert_eq!(config.experience_hygiene_limit, 7);
}

#[test]
fn parses_experience_repair_dry_run() {
    let config = CliConfig::parse([
        "--repair-dry-run".to_owned(),
        "--repair-limit".to_owned(),
        "8".to_owned(),
    ])
    .unwrap();

    assert!(config.experience_repair);
    assert_eq!(config.experience_repair_limit, 8);
}

#[test]
fn parses_experience_cleanup_audit() {
    let config = CliConfig::parse([
        "--audit".to_owned(),
        "--audit-limit".to_owned(),
        "9".to_owned(),
    ])
    .unwrap();

    assert!(config.experience_cleanup_audit);
    assert_eq!(config.experience_cleanup_audit_limit, 9);
}

#[test]
fn parses_model_pool_status_and_route() {
    let status = CliConfig::parse(["--pool-status".to_owned()]).unwrap();
    assert!(status.model_pool_status);

    let manifest = CliConfig::parse(["--pool-manifest".to_owned()]).unwrap();
    assert!(manifest.model_pool_manifest);

    let apple_manifest = CliConfig::parse(["--apple-pool-manifest".to_owned()]).unwrap();
    assert!(apple_manifest.model_pool_manifest);

    let advice = CliConfig::parse(["--pool-advice".to_owned()]).unwrap();
    assert!(advice.model_pool_advice);

    let apple_advice = CliConfig::parse(["--apple-pool-advice".to_owned()]).unwrap();
    assert!(apple_advice.model_pool_advice);

    let smoke = CliConfig::parse(["--pool-smoke".to_owned()]).unwrap();
    assert!(smoke.model_pool_smoke);

    let apple_smoke = CliConfig::parse(["--apple-pool-smoke".to_owned()]).unwrap();
    assert!(apple_smoke.model_pool_smoke);

    let route = CliConfig::parse(["--pool-route".to_owned(), "test".to_owned()]).unwrap();
    assert_eq!(route.model_pool_route.as_deref(), Some("test-gate"));

    let index_route = CliConfig::parse(["--pool-route".to_owned(), "index".to_owned()]).unwrap();
    assert_eq!(index_route.model_pool_route.as_deref(), Some("index"));
    let spare_route = CliConfig::parse(["--pool-route".to_owned(), "spare".to_owned()]).unwrap();
    assert_eq!(spare_route.model_pool_route.as_deref(), Some("index"));

    let call = CliConfig::parse([
        "--pool-call".to_owned(),
        "review".to_owned(),
        "--prompt".to_owned(),
        "review this patch".to_owned(),
    ])
    .unwrap();
    assert_eq!(call.model_pool_call.as_deref(), Some("review"));
    assert_eq!(call.prompt.as_deref(), Some("review this patch"));

    let index_call = CliConfig::parse([
        "--pool-call".to_owned(),
        "REPO-INDEX".to_owned(),
        "--prompt".to_owned(),
        "refresh repository map".to_owned(),
    ])
    .unwrap();
    assert_eq!(index_call.model_pool_call.as_deref(), Some("index"));
    assert_eq!(index_call.prompt.as_deref(), Some("refresh repository map"));

    let spare_call = CliConfig::parse([
        "--pool-call".to_owned(),
        "spare".to_owned(),
        "--prompt".to_owned(),
        "refresh repository map".to_owned(),
    ])
    .unwrap();
    assert_eq!(spare_call.model_pool_call.as_deref(), Some("index"));
}

#[test]
fn parses_model_pool_watch() {
    let watch = CliConfig::parse(["--pool-watch".to_owned()]).unwrap();
    let watch = watch.model_pool_watch.unwrap();
    assert_eq!(watch.interval_secs, 5);
    assert_eq!(watch.max_iterations, None);

    let limited = CliConfig::parse([
        "--pool-watch".to_owned(),
        "2".to_owned(),
        "--pool-watch-count".to_owned(),
        "3".to_owned(),
    ])
    .unwrap();
    let limited = limited.model_pool_watch.unwrap();
    assert_eq!(limited.interval_secs, 2);
    assert_eq!(limited.max_iterations, Some(3));

    let count_first = CliConfig::parse([
        "--pool-watch-count".to_owned(),
        "4".to_owned(),
        "--pool-watch".to_owned(),
    ])
    .unwrap();
    let count_first = count_first.model_pool_watch.unwrap();
    assert_eq!(count_first.interval_secs, 5);
    assert_eq!(count_first.max_iterations, Some(4));
}

#[test]
fn parses_evolution_status_flags() {
    let status = CliConfig::parse(["--evolution-status".to_owned()]).unwrap();
    assert!(status.evolution_status);
    assert!(!status.evolution_status_json);
    assert!(!status.evolution_strict_summary);
    assert!(!status.evolution_start);
    assert!(!status.evolution_stop);
    assert!(!status.evolution_check_only);
    assert_eq!(status.evolution_work_dir, "target\\evolution\\daemon");

    let json = CliConfig::parse([
        "--daemon-status-json".to_owned(),
        "--daemon-work-dir".to_owned(),
        "target\\evolution\\daemon-live-run2-20260615".to_owned(),
    ])
    .unwrap();
    assert!(json.evolution_status);
    assert!(json.evolution_status_json);
    assert_eq!(
        json.evolution_work_dir,
        "target\\evolution\\daemon-live-run2-20260615"
    );
}

#[test]
fn parses_evolution_strict_summary_flags() {
    let summary = CliConfig::parse(["--evolution-strict-summary".to_owned()]).unwrap();
    assert!(summary.evolution_strict_summary);
    assert!(!summary.evolution_strict_summary_json);
    assert_eq!(summary.evolution_strict_summary_path, None);

    let json = CliConfig::parse([
        "--strict-summary-json".to_owned(),
        "--strict-summary-path".to_owned(),
        "target\\evolution\\strict-status-summary.json".to_owned(),
    ])
    .unwrap();
    assert!(json.evolution_strict_summary);
    assert!(json.evolution_strict_summary_json);
    assert_eq!(
        json.evolution_strict_summary_path.as_deref(),
        Some("target\\evolution\\strict-status-summary.json")
    );
}

#[test]
fn parses_evolution_daemon_control_flags() {
    let start = CliConfig::parse([
        "--evolution-start-check".to_owned(),
        "--backend".to_owned(),
        "http://127.0.0.1:7979".to_owned(),
        "--prompt".to_owned(),
        "short daemon check".to_owned(),
        "--evolution-interval-secs".to_owned(),
        "1".to_owned(),
        "--evolution-max-tokens".to_owned(),
        "64".to_owned(),
        "--evolution-max-total-tokens".to_owned(),
        "96".to_owned(),
        "--evolution-max-runtime-secs".to_owned(),
        "0".to_owned(),
        "--evolution-max-failures".to_owned(),
        "1".to_owned(),
        "--evolution-max-no-feedback-rounds".to_owned(),
        "0".to_owned(),
        "--evolution-timeout-secs".to_owned(),
        "300".to_owned(),
    ])
    .unwrap();
    assert!(start.evolution_start);
    assert!(start.evolution_check_only);
    assert!(start.backend_overridden);
    assert_eq!(start.backend, "127.0.0.1:7979");
    assert_eq!(start.prompt.as_deref(), Some("short daemon check"));
    assert_eq!(start.evolution_interval_secs, Some(1));
    assert_eq!(start.evolution_max_tokens, Some(64));
    assert_eq!(start.evolution_max_total_tokens, Some(96));
    assert_eq!(start.evolution_max_runtime_secs, Some(0));
    assert_eq!(start.evolution_max_failures, Some(1));
    assert_eq!(start.evolution_max_no_feedback_rounds, Some(0));
    assert_eq!(start.evolution_timeout_secs, Some(300));

    let start_with_backlog = CliConfig::parse([
        "--evolution-start-check".to_owned(),
        "--evolution-candidates-backlog".to_owned(),
        "target\\evolution\\candidate-backlog.jsonl".to_owned(),
    ])
    .unwrap();
    assert!(start_with_backlog.evolution_start);
    assert!(start_with_backlog.evolution_check_only);
    assert!(!start_with_backlog.evolution_candidates);
    assert!(!start_with_backlog.evolution_candidates_save);
    assert_eq!(
        start_with_backlog.evolution_candidates_backlog.as_deref(),
        Some("target\\evolution\\candidate-backlog.jsonl")
    );

    let start_check_json = CliConfig::parse([
        "--daemon-start-check-json".to_owned(),
        "--backend".to_owned(),
        "http://127.0.0.1:7979".to_owned(),
        "--evolution-max-tokens".to_owned(),
        "64".to_owned(),
        "--evolution-max-total-tokens".to_owned(),
        "96".to_owned(),
    ])
    .unwrap();
    assert!(start_check_json.evolution_start);
    assert!(start_check_json.evolution_check_only);
    assert!(start_check_json.evolution_start_check_json);
    assert_eq!(start_check_json.backend, "127.0.0.1:7979");
    assert_eq!(start_check_json.evolution_max_tokens, Some(64));
    assert_eq!(start_check_json.evolution_max_total_tokens, Some(96));

    let stop = CliConfig::parse(["--daemon-stop-check".to_owned()]).unwrap();
    assert!(stop.evolution_stop);
    assert!(stop.evolution_check_only);
    assert!(!stop.backend_overridden);

    let invalid_budget_without_start = CliConfig::parse([
        "--evolution-status".to_owned(),
        "--evolution-max-total-tokens".to_owned(),
        "96".to_owned(),
    ]);
    assert!(
        invalid_budget_without_start
            .unwrap_err()
            .contains("budget options require")
    );
}

#[test]
fn parses_evolution_watch_flags() {
    let watch = CliConfig::parse([
        "--evolution-watch".to_owned(),
        "2".to_owned(),
        "--evolution-watch-count".to_owned(),
        "3".to_owned(),
    ])
    .unwrap();
    let watch = watch.evolution_watch.unwrap();

    assert_eq!(watch.interval_secs, 2);
    assert_eq!(watch.max_iterations, Some(3));

    let count_first = CliConfig::parse([
        "--daemon-watch-count".to_owned(),
        "4".to_owned(),
        "--daemon-watch".to_owned(),
    ])
    .unwrap();
    let count_first = count_first.evolution_watch.unwrap();

    assert_eq!(count_first.interval_secs, 5);
    assert_eq!(count_first.max_iterations, Some(4));
}

#[test]
fn parses_evolution_candidates_flags() {
    let candidates = CliConfig::parse(["--evolution-candidates".to_owned()]).unwrap();
    assert!(candidates.evolution_candidates);
    assert_eq!(candidates.evolution_candidates_limit, 5);
    assert!(!candidates.evolution_candidates_save);
    assert_eq!(candidates.evolution_candidates_backlog, None);
    assert_eq!(candidates.evolution_work_dir, "target\\evolution\\daemon");

    let limited = CliConfig::parse([
        "--daemon-candidates-limit".to_owned(),
        "2".to_owned(),
        "--daemon-work-dir".to_owned(),
        "target\\evolution\\daemon-smoke".to_owned(),
    ])
    .unwrap();
    assert!(limited.evolution_candidates);
    assert_eq!(limited.evolution_candidates_limit, 2);
    assert_eq!(
        limited.evolution_work_dir,
        "target\\evolution\\daemon-smoke"
    );

    let save = CliConfig::parse(["--evolution-candidates-save".to_owned()]).unwrap();
    assert!(save.evolution_candidates);
    assert!(save.evolution_candidates_save);
    assert_eq!(save.evolution_candidates_backlog, None);

    let custom = CliConfig::parse([
        "--daemon-candidates-backlog".to_owned(),
        "target\\evolution\\custom-candidates.jsonl".to_owned(),
    ])
    .unwrap();
    assert!(custom.evolution_candidates);
    assert!(custom.evolution_candidates_save);
    assert_eq!(
        custom.evolution_candidates_backlog.as_deref(),
        Some("target\\evolution\\custom-candidates.jsonl")
    );

    let mark = CliConfig::parse([
        "--evolution-candidate-mark".to_owned(),
        "smartsteam-candidate-1".to_owned(),
        "--evolution-candidate-status".to_owned(),
        "accepted".to_owned(),
        "--evolution-candidate-note".to_owned(),
        "ready to implement".to_owned(),
        "--evolution-candidates-backlog".to_owned(),
        "target\\evolution\\candidate-backlog.jsonl".to_owned(),
    ])
    .unwrap();
    assert_eq!(
        mark.evolution_candidate_mark.as_deref(),
        Some("smartsteam-candidate-1")
    );
    assert_eq!(mark.evolution_candidate_status.as_deref(), Some("accepted"));
    assert_eq!(
        mark.evolution_candidate_note.as_deref(),
        Some("ready to implement")
    );
    assert_eq!(
        mark.evolution_candidates_backlog.as_deref(),
        Some("target\\evolution\\candidate-backlog.jsonl")
    );
    assert!(!mark.evolution_candidates);
    assert!(!mark.evolution_candidates_save);

    let list = CliConfig::parse([
        "--evolution-candidate-list".to_owned(),
        "--evolution-candidate-status".to_owned(),
        "accepted".to_owned(),
        "--evolution-candidates-limit".to_owned(),
        "3".to_owned(),
        "--evolution-candidates-backlog".to_owned(),
        "target\\evolution\\candidate-backlog.jsonl".to_owned(),
    ])
    .unwrap();
    assert!(list.evolution_candidate_list);
    assert!(!list.evolution_candidates);
    assert!(!list.evolution_candidates_save);
    assert_eq!(list.evolution_candidates_limit, 3);
    assert_eq!(list.evolution_candidate_status.as_deref(), Some("accepted"));
    assert_eq!(
        list.evolution_candidates_backlog.as_deref(),
        Some("target\\evolution\\candidate-backlog.jsonl")
    );

    let apply_check = CliConfig::parse([
        "--evolution-candidate-apply-check".to_owned(),
        "next".to_owned(),
        "--evolution-candidates-backlog".to_owned(),
        "target\\evolution\\candidate-backlog.jsonl".to_owned(),
    ])
    .unwrap();
    assert_eq!(
        apply_check.evolution_candidate_apply_check.as_deref(),
        Some("next")
    );
    assert_eq!(
        apply_check.evolution_candidates_backlog.as_deref(),
        Some("target\\evolution\\candidate-backlog.jsonl")
    );
    assert!(!apply_check.evolution_candidates);
    assert!(!apply_check.evolution_candidates_save);
    assert!(!apply_check.evolution_candidate_list);

    let gate = CliConfig::parse([
        "--evolution-candidate-gate".to_owned(),
        "--evolution-candidates-backlog".to_owned(),
        "target\\evolution\\candidate-backlog.jsonl".to_owned(),
    ])
    .unwrap();
    assert!(gate.evolution_candidate_gate);
    assert!(!gate.evolution_candidates);
    assert!(!gate.evolution_candidates_save);
    assert_eq!(
        gate.evolution_candidates_backlog.as_deref(),
        Some("target\\evolution\\candidate-backlog.jsonl")
    );

    let validate = CliConfig::parse([
        "--evolution-candidate-validate".to_owned(),
        "smartsteam-candidate-1".to_owned(),
        "--evolution-candidate-validation-command".to_owned(),
        "cargo test -q --manifest-path tools/smartsteam-forge/Cargo.toml".to_owned(),
        "--evolution-candidate-validation-status".to_owned(),
        "0".to_owned(),
        "--evolution-candidate-note".to_owned(),
        "green".to_owned(),
        "--evolution-candidates-backlog".to_owned(),
        "target\\evolution\\candidate-backlog.jsonl".to_owned(),
    ])
    .unwrap();
    assert_eq!(
        validate.evolution_candidate_validate.as_deref(),
        Some("smartsteam-candidate-1")
    );
    assert_eq!(
        validate.evolution_candidate_validation_command.as_deref(),
        Some("cargo test -q --manifest-path tools/smartsteam-forge/Cargo.toml")
    );
    assert_eq!(
        validate.evolution_candidate_validation_status.as_deref(),
        Some("0")
    );
    assert_eq!(validate.evolution_candidate_note.as_deref(), Some("green"));
    assert!(!validate.evolution_candidates);
    assert!(!validate.evolution_candidates_save);
}

#[test]
fn rejects_conflicting_evolution_daemon_actions() {
    let error = CliConfig::parse([
        "--evolution-start".to_owned(),
        "--evolution-stop".to_owned(),
    ])
    .unwrap_err();

    assert!(error.contains("choose only one evolution daemon action"));

    let strict_summary_error = CliConfig::parse([
        "--evolution-status".to_owned(),
        "--evolution-strict-summary".to_owned(),
    ])
    .unwrap_err();
    assert!(strict_summary_error.contains("strict-summary"));
}

#[test]
fn rejects_evolution_candidate_mark_without_status() {
    let error =
        CliConfig::parse(["--evolution-candidate-mark".to_owned(), "id-1".to_owned()]).unwrap_err();

    assert!(error.contains("requires --evolution-candidate-status"));
}

#[test]
fn rejects_evolution_candidate_validate_without_evidence() {
    let missing_command = CliConfig::parse([
        "--evolution-candidate-validate".to_owned(),
        "id-1".to_owned(),
        "--evolution-candidate-validation-status".to_owned(),
        "0".to_owned(),
    ])
    .unwrap_err();
    let missing_status = CliConfig::parse([
        "--evolution-candidate-validate".to_owned(),
        "id-1".to_owned(),
        "--evolution-candidate-validation-command".to_owned(),
        "cargo test".to_owned(),
    ])
    .unwrap_err();

    assert!(missing_command.contains("requires --evolution-candidate-validation-command"));
    assert!(missing_status.contains("requires --evolution-candidate-validation-status"));
}

#[test]
fn rejects_conflicting_evolution_watch_and_status_json() {
    let error = CliConfig::parse([
        "--evolution-status-json".to_owned(),
        "--evolution-watch".to_owned(),
    ])
    .unwrap_err();

    assert!(error.contains("cannot be combined"));
}

#[test]
fn rejects_evolution_check_only_without_action() {
    let error = CliConfig::parse(["--evolution-check-only".to_owned()]).unwrap_err();

    assert!(error.contains("requires --evolution-start or --evolution-stop"));
}

#[test]
fn parses_doctor_aliases() {
    let config = CliConfig::parse(["--diagnostic".to_owned()]).unwrap();

    assert!(config.doctor);
}

#[test]
fn parses_preflight_aliases() {
    let config = CliConfig::parse(["--ready".to_owned()]).unwrap();

    assert!(config.preflight_check);
}

#[test]
fn parses_session_listing_command() {
    let config = CliConfig::parse(["--sessions".to_owned()]).unwrap();

    assert_eq!(
        config.session_command,
        Some(app::SessionCliCommand::List {
            filter: SessionFilter::All,
            limit: 50
        })
    );
}

#[test]
fn parses_session_listing_filter_and_limit() {
    let config = CliConfig::parse([
        "--sessions".to_owned(),
        "failed".to_owned(),
        "--session-limit".to_owned(),
        "3".to_owned(),
    ])
    .unwrap();

    assert_eq!(
        config.session_command,
        Some(app::SessionCliCommand::List {
            filter: SessionFilter::Failed,
            limit: 3
        })
    );
}

#[test]
fn parses_summary_command_selector() {
    let config = CliConfig::parse(["--summary".to_owned(), "2".to_owned()]).unwrap();

    assert_eq!(
        config.session_command,
        Some(app::SessionCliCommand::Summary {
            selector: "2".to_owned()
        })
    );
}

#[test]
fn parses_require_health() {
    let config = CliConfig::parse(["--require-health".to_owned()]).unwrap();

    assert!(config.require_health);
}

#[test]
fn parses_require_safe_device() {
    let config = CliConfig::parse(["--require-safe-device".to_owned()]).unwrap();

    assert!(config.require_safe_device);
}

#[test]
fn parses_request_timeout() {
    let config = CliConfig::parse(["--timeout-secs".to_owned(), "12".to_owned()]).unwrap();

    assert_eq!(config.request_timeout_secs, Some(12));
    assert_eq!(
        provider_config(&config).request_timeout,
        Duration::from_secs(12)
    );
}

#[test]
fn parses_request_timeout_alias() {
    let config = CliConfig::parse(["--request-timeout-secs".to_owned(), "12".to_owned()]).unwrap();

    assert_eq!(config.request_timeout_secs, Some(12));
    assert_eq!(
        provider_config(&config).request_timeout,
        Duration::from_secs(12)
    );
}

#[test]
fn default_request_timeout_allows_slow_gemma_streams() {
    let config = CliConfig::parse([]).unwrap();

    assert_eq!(
        provider_config(&config).request_timeout,
        Duration::from_secs(900)
    );
}

#[test]
fn parses_transport_timeouts() {
    let config = CliConfig::parse([
        "--connect-timeout-ms".to_owned(),
        "250".to_owned(),
        "--read-timeout-ms".to_owned(),
        "750".to_owned(),
    ])
    .unwrap();

    let provider = provider_config(&config);

    assert_eq!(config.connect_timeout_ms, Some(250));
    assert_eq!(config.read_timeout_ms, Some(750));
    assert_eq!(provider.connect_timeout, Duration::from_millis(250));
    assert_eq!(provider.read_timeout, Duration::from_millis(750));
}

#[test]
fn help_distinguishes_read_poll_from_total_timeout() {
    let help = usage();

    assert!(help.contains("--read-timeout-ms <ms>"));
    assert!(help.contains("per-read poll/heartbeat interval"));
    assert!(help.contains("not total stream timeout"));
    assert!(help.contains("--timeout-secs <seconds>"));
    assert!(help.contains("total one-shot/stream backend request timeout"));
    assert!(help.contains("--request-timeout-secs <s>"));
    assert!(help.contains("total request window, not read polling"));
}

#[test]
fn help_documents_index_notes_clear_legacy_cleanup() {
    let help = usage();

    assert!(help.contains("TUI /index-notes clear"));
    assert!(help.contains("clears all model-pool index blocks"));
    assert!(help.contains("including legacy tails"));
}

#[test]
fn help_documents_evolution_status() {
    let help = usage();

    assert!(help.contains("--evolution-status"));
    assert!(help.contains("read-only evolution-loop daemon"));
    assert!(help.contains("--evolution-strict-summary"));
    assert!(help.contains("compact strict unattended status artifact"));
    assert!(help.contains("TUI /strict-status [path]"));
    assert!(help.contains("strict unattended evolution status artifact"));
    assert!(help.contains("--evolution-candidates"));
    assert!(help.contains("--evolution-watch"));
    assert!(help.contains("--evolution-start"));
    assert!(help.contains("--evolution-start-check"));
    assert!(help.contains("--evolution-start-check-json"));
    assert!(help.contains("machine-readable start preflight"));
    assert!(help.contains("--evolution-work-dir <dir>"));
}

#[test]
fn parses_context_window() {
    let config = CliConfig::parse(["--context-messages".to_owned(), "6".to_owned()]).unwrap();

    assert_eq!(config.context_messages, Some(6));
}

#[test]
fn parses_max_tokens() {
    let config = CliConfig::parse(["--max-tokens".to_owned(), "8192".to_owned()]).unwrap();

    assert_eq!(config.max_tokens, Some(Some(8192)));
}

#[test]
fn parses_default_max_tokens() {
    let config = CliConfig::parse(["--max-tokens".to_owned(), "DEFAULT".to_owned()]).unwrap();

    assert_eq!(config.max_tokens, Some(None));
}

#[test]
fn parses_auto_max_tokens() {
    let config = CliConfig::parse(["--max-output-tokens".to_owned(), "auto".to_owned()]).unwrap();

    assert_eq!(config.max_tokens, Some(None));
}

#[test]
fn clamps_large_max_tokens() {
    let config = CliConfig::parse(["--max-output-tokens".to_owned(), "999999".to_owned()]).unwrap();

    assert_eq!(config.max_tokens, Some(Some(262_144)));
}

#[test]
fn rejects_missing_argument_value() {
    let error = CliConfig::parse(["--prompt".to_owned()]).unwrap_err();

    assert!(error.contains("requires a value"));
}

#[test]
fn rejects_zero_request_timeout() {
    let error = CliConfig::parse(["--timeout-secs".to_owned(), "0".to_owned()]).unwrap_err();

    assert!(error.contains("positive integer"));
}

#[test]
fn rejects_zero_transport_timeout() {
    let error = CliConfig::parse(["--connect-timeout-ms".to_owned(), "0".to_owned()]).unwrap_err();

    assert!(error.contains("positive integer"));
}

#[test]
fn rejects_unknown_session_filter() {
    let error = CliConfig::parse(["--sessions".to_owned(), "maybe".to_owned()]).unwrap_err();

    assert!(error.contains("unsupported --sessions filter"));
}

#[test]
fn rejects_unknown_model_pool_route_kind() {
    let error = CliConfig::parse(["--pool-route".to_owned(), "launch".to_owned()]).unwrap_err();

    assert!(error.contains("must be one of"));
}
