use std::process::{Command, Output};
use std::{env, fs};

fn run_cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_norion-cli"))
        .args(args)
        .output()
        .expect("failed to run norion-cli")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout should be utf-8")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr should be utf-8")
}

#[test]
fn default_startup_is_local_unpinned_protocol_shell() {
    let output = run_cli(&[]);

    assert!(output.status.success());
    let out = stdout(&output);
    assert!(out.contains("norion-cli protocol shell"));
    assert!(out.contains("role=assistant preference=balanced endpoint=auto pinned=false"));
    assert!(out.contains("history_limit=64 max_tokens=backend-default"));
    assert!(out.contains("/role ROLE"));
    assert!(out.contains("/prefer fast|quality|balanced"));
    assert!(out.contains("/max-tokens N|auto"));
    assert!(out.contains("/history-limit N"));
    assert!(stderr(&output).is_empty());
}

#[test]
fn explicit_worker_flag_is_visible_as_operator_pin_without_prompt_send() {
    let output = run_cli(&[
        "--role",
        "reviewer",
        "--prefer",
        "fast",
        "--worker",
        "mlx-reviewer-8b",
        "--max-tokens",
        "8192",
        "--history-limit",
        "16",
    ]);

    assert!(output.status.success());
    let out = stdout(&output);
    assert!(
        out.contains("role=reviewer preference=prefer_fast endpoint=mlx-reviewer-8b pinned=true")
    );
    assert!(out.contains("history_limit=16 max_tokens=8192"));
    assert!(!out.contains("reviewer fast mlx-reviewer-8b"));
    assert!(stderr(&output).is_empty());
}

#[test]
fn endpoint_auto_keeps_role_preference_as_scheduler_hints_without_worker_pin() {
    let output = run_cli(&[
        "--role",
        "reviewer",
        "--prefer",
        "fast",
        "--endpoint",
        "auto",
        "--max-tokens",
        "auto",
    ]);

    assert!(output.status.success());
    let out = stdout(&output);
    assert!(out.contains("norion-cli protocol shell"));
    assert!(out.contains("role=reviewer preference=prefer_fast endpoint=auto pinned=false"));
    assert!(out.contains("history_limit=64 max_tokens=backend-default"));
    assert!(!out.contains("endpoint=fast-reviewer pinned=true"));
    assert!(!out.contains("endpoint=mlx-reviewer-8b pinned=true"));
    assert!(stderr(&output).is_empty());
}

#[test]
fn help_exits_successfully_without_starting_protocol_shell() {
    let output = run_cli(&["--help"]);

    assert!(output.status.success());
    let out = stdout(&output);
    assert!(out.contains("usage: norion-cli"));
    assert!(out.contains("no-backend mode"));
    assert!(out.contains("does not start Gemma"));
    assert!(out.contains("connect to a backend"));
    assert!(out.contains("submit prompts"));
    assert!(out.contains("endpoint=auto pinned=false"));
    assert!(out.contains("roles: assistant|reviewer|summarizer|tester"));
    assert!(out.contains("preferences: balanced|fast|quality"));
    assert!(out.contains("endpoint auto|default|none clears a worker pin"));
    assert!(!out.contains("norion-cli protocol shell"));
    assert!(stderr(&output).is_empty());
}

#[test]
fn invalid_argument_exits_with_help_and_no_startup_lines() {
    let output = run_cli(&["--bogus"]);

    assert_eq!(output.status.code(), Some(2));
    let err = stderr(&output);
    assert!(err.contains("unknown option: --bogus"));
    assert!(err.contains("usage: norion-cli"));
    assert!(!err.contains("norion-cli protocol shell"));
    assert!(stdout(&output).is_empty());
}

#[test]
fn evidence_packet_prints_redacted_issue_comment() {
    let input = env::temp_dir().join(format!(
        "norion-cli-evidence-{}-{}.txt",
        std::process::id(),
        "issue48"
    ));
    fs::write(
        &input,
        "cargo test ok\nOPENAI_API_KEY=sk-secret\nplain ghp_leak done\n",
    )
    .expect("write evidence input");

    let output = run_cli(&[
        "evidence-packet",
        "--issue",
        "48",
        "--commit",
        "abc123",
        "--command",
        "cargo test -p norion-cli",
        "--gate",
        "passed",
        "--input",
        input.to_str().expect("temp path should be utf-8"),
    ]);

    assert!(output.status.success());
    let out = stdout(&output);
    assert!(out.contains("## Evidence packet for #48"));
    assert!(out.contains("- commit: abc123"));
    assert!(out.contains("- gate: passed"));
    assert!(out.contains("OPENAI_API_KEY=<redacted>"));
    assert!(out.contains("plain <redacted> done"));
    assert!(!out.contains("sk-secret"));
    assert!(!out.contains("ghp_leak"));
    assert!(stderr(&output).is_empty());

    let _ = fs::remove_file(input);
}

#[test]
fn issue30_evidence_packet_cli_keeps_trace_gate_command_and_redacts_payload() {
    let input = env::temp_dir().join(format!(
        "norion-cli-evidence-{}-{}.txt",
        std::process::id(),
        "issue30"
    ));
    fs::write(
        &input,
        "issue30_clean_checkout_demo clean_checkout=true live_model_required=false private_state_required=false prompt_digest_ref=redaction-digest:issue30-default-prompt\ntrace_schema_gate: passed=true\nreasoning_genome_events=2 reasoning_genome_write_allowed=0 reasoning_genome_splice_write_allowed=0 self_evolution_admission_events=1\nsecond_compute_budget_avoided_tokens=448\nnegative_unauthorized_write_allowed=false negative_polluted_evidence_blocked=true negative_polluted_evidence_quarantined=true negative_bad_candidate_held_or_rolled_back=true negative_rollback_anchor_present=true negative_rollback_anchor_evidence_id=issue-30-roundtrip-negative-gate-hold negative_rollback_anchor_digest=redaction-digest:0123456789abcdef negative_tenant_scope_write_denied=true negative_single_tenant_preview=true negative_provenance_license_redaction_passed=true negative_digest_only=true\nlocal_path=C:\\Users\\jy\\AppData\\Local\\Temp\\issue30.txt\nprompt: private raw prompt\nanswer_text=raw answer\nOPENAI_API_KEY=sk-secret\n",
    )
    .expect("write issue30 evidence input");

    let command = "cargo run -- --benchmark-roundtrip --inspect-state --inspect-gate --trace \"$STATE_DIR/issue30-trace.jsonl\" --trace-schema-gate \"$STATE_DIR/issue30-trace.jsonl\"";
    let output = run_cli(&[
        "evidence-packet",
        "--issue",
        "30",
        "--commit",
        "clean-checkout-demo",
        "--command",
        command,
        "--gate",
        "passed",
        "--input",
        input.to_str().expect("temp path should be utf-8"),
    ]);

    assert!(output.status.success());
    let out = stdout(&output);
    assert!(out.contains("## Evidence packet for #30"));
    assert!(out.contains("--trace-schema-gate"));
    assert!(out.contains("clean_checkout=true"));
    assert!(out.contains("trace_schema_gate: passed=true"));
    assert!(out.contains("reasoning_genome_events=2"));
    assert!(out.contains("reasoning_genome_write_allowed=0"));
    assert!(out.contains("reasoning_genome_splice_write_allowed=0"));
    assert!(out.contains("self_evolution_admission_events=1"));
    assert!(out.contains("second_compute_budget_avoided_tokens=448"));
    assert!(out.contains("negative_unauthorized_write_allowed=false"));
    assert!(out.contains("negative_polluted_evidence_blocked=true"));
    assert!(out.contains("negative_polluted_evidence_quarantined=true"));
    assert!(out.contains("negative_bad_candidate_held_or_rolled_back=true"));
    assert!(out.contains("negative_rollback_anchor_present=true"));
    assert!(
        out.contains("negative_rollback_anchor_evidence_id=issue-30-roundtrip-negative-gate-hold")
    );
    assert!(out.contains("negative_rollback_anchor_digest=redaction-digest:0123456789abcdef"));
    assert!(out.contains("negative_tenant_scope_write_denied=true"));
    assert!(out.contains("negative_single_tenant_preview=true"));
    assert!(out.contains("negative_provenance_license_redaction_passed=true"));
    assert!(out.contains("negative_digest_only=true"));
    assert!(out.contains("local_path=<redacted-path>"));
    assert!(out.contains("prompt=<redacted-payload>"));
    assert!(out.contains("answer_text=<redacted-payload>"));
    assert!(out.contains("OPENAI_API_KEY=<redacted>"));
    assert!(!out.contains("C:\\Users"));
    assert!(!out.contains("AppData"));
    assert!(!out.contains("private raw prompt"));
    assert!(!out.contains("raw answer"));
    assert!(!out.contains("sk-secret"));
    assert!(stderr(&output).is_empty());

    let _ = fs::remove_file(input);
}
