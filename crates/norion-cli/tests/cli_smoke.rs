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
        "--require",
        "OPENAI_API_KEY=<redacted>",
        "--require",
        "plain <redacted> done",
        "--reject",
        "sk-secret",
        "--reject",
        "ghp_leak",
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
    let git_worktree = env::temp_dir().join(format!(
        "norion-cli-evidence-git-{}-{}",
        std::process::id(),
        "issue30"
    ));
    fs::create_dir_all(&git_worktree).expect("create git worktree fixture");
    let git_init = Command::new("git")
        .arg("-C")
        .arg(&git_worktree)
        .args(["init", "--quiet"])
        .output()
        .expect("git init should run for evidence packet fixture");
    assert!(git_init.status.success());
    let git_branch = Command::new("git")
        .arg("-C")
        .arg(&git_worktree)
        .args(["checkout", "-b", "issue30-fixture"])
        .output()
        .expect("git branch should run for evidence packet fixture");
    assert!(git_branch.status.success());
    fs::write(git_worktree.join("fixture.txt"), "issue30 fixture\n")
        .expect("write git fixture file");
    for args in [
        ["config", "user.email", "issue30@example.invalid"].as_slice(),
        ["config", "user.name", "Issue 30 Fixture"].as_slice(),
        ["add", "fixture.txt"].as_slice(),
        ["commit", "--quiet", "-m", "fixture"].as_slice(),
    ] {
        let output = Command::new("git")
            .arg("-C")
            .arg(&git_worktree)
            .args(args)
            .output()
            .expect("git fixture command should run");
        assert!(
            output.status.success(),
            "git fixture command failed: {args:?}"
        );
    }
    let git_sha = Command::new("git")
        .arg("-C")
        .arg(&git_worktree)
        .args(["rev-parse", "HEAD"])
        .output()
        .expect("git rev-parse should run for evidence packet fixture");
    assert!(git_sha.status.success());
    let git_sha = String::from_utf8(git_sha.stdout)
        .expect("git sha should be utf-8")
        .trim()
        .to_owned();
    let git_sha_field = format!("rc_sha={git_sha}");
    let release_review = env::temp_dir().join(format!(
        "norion-cli-release-review-{}-{}.txt",
        std::process::id(),
        "issue30"
    ));
    fs::write(
        &release_review,
        "pr=428 review=REVIEW_REQUIRED checks=passed branch_protection=present\npr=429 review=REVIEW_REQUIRED checks=passed branch_protection=present\n",
    )
    .expect("write release review fixture");
    let issue_state = env::temp_dir().join(format!(
        "norion-cli-issue-state-{}-{}.txt",
        std::process::id(),
        "issue30"
    ));
    fs::write(
        &issue_state,
        "issue=31 state=open final_signoff=false\nissue=19 state=open runtime_surface_closed=false runtime_surface_merged_prs=#290,#291,#292,#293,#296,#307,#308,#309 runtime_counters_pr=#429 runtime_counters_ready=false runtime_counters_head=6f049dd02f1c8352939f9a9356f2b2f90ce07569 runtime_counters_checks=green runtime_counters_review=review_required runtime_counters_merged=false runtime_surface_blocker=#429:REVIEW_REQUIRED\nissue=30 state=open close_allowed=false\n",
    )
    .expect("write issue state fixture");
    let demo_proof = env::temp_dir().join(format!(
        "norion-cli-demo-proof-{}-{}.txt",
        std::process::id(),
        "issue30"
    ));
    fs::write(
        &demo_proof,
        "clean_checkout=true live_model_required=false private_state_required=false prompt_digest_ref=redaction-digest:issue30-default-prompt integration_test=issue30_clean_checkout_demo_writes_digest_only_evidence_packet dispatch_test=issue30_dispatch_roundtrip_inspect_runs_trace_schema_gate dispatch_path=dispatch::run trace_schema_gate_executed=true\n",
    )
    .expect("write demo proof fixture");
    let roundtrip_proof = env::temp_dir().join(format!(
        "norion-cli-roundtrip-proof-{}-{}.txt",
        std::process::id(),
        "issue30"
    ));
    fs::write(
        &roundtrip_proof,
        "persistent_roundtrip: passed=true first_stored_memory=true first_runtime_kv_stored=true first_runtime_kv_namespace_preserved=true second_used_memories=1 second_used_runtime_kv_memory=true second_used_experiences=2 second_approved_experience_reuse_digest=redaction-digest:abcdef0123456789 second_imported_runtime_kv_blocks=1 second_imported_runtime_kv_from_namespace=true second_runtime_adapter_observations=1 second_runtime_adapter_best_score=0.750 second_runtime_adapter_best_adapter=deterministic-roundtrip-adapter second_runtime_selected_adapter=deterministic-roundtrip-adapter second_compute_budget_saved_tokens=320 second_compute_budget_avoided_tokens=448 second_compute_budget_kv_lookups_skipped=2 second_compute_budget_anchor_count=2 second_compute_budget_anchors_preserved_count=2 negative_unauthorized_write_allowed=false negative_memory_write_allowed=false negative_genome_write_allowed=false negative_self_evolution_write_allowed=false negative_polluted_evidence_blocked=true negative_polluted_evidence_quarantined=true negative_bad_candidate_digest=redaction-digest:fedcba9876543210 negative_bad_candidate_decision=hold_then_rollback negative_rollback_anchor_present=true negative_rollback_anchor_evidence_id=issue-30-roundtrip-negative-gate-hold negative_rollback_anchor_digest=redaction-digest:0123456789abcdef negative_tenant_scope_write_denied=true negative_tenant_scope_mode=local_single_user_preview negative_tenant_scope_actor=fnv64:1111111111111111 negative_tenant_scope_target=fnv64:2222222222222222 negative_tenant_scope_denial_lane=self_evolving_memory negative_tenant_scope_denial_reason=cross_tenant_scope_rejected negative_provenance_license_redaction_passed=true second_quality=0.820 first_drift=watch second_drift=watch failures=0\n",
    )
    .expect("write roundtrip proof fixture");
    let trace_report = env::temp_dir().join(format!(
        "norion-cli-trace-report-{}-{}.txt",
        std::process::id(),
        "issue30"
    ));
    fs::write(
        &trace_report,
        "trace_schema_gate: passed=true lines=12 failures=0 reasoning_genome_events=2 reasoning_genome_write_allowed=0 reasoning_genome_splice_write_allowed=0 self_evolution_admission_events=1 self_evolution_admission_review_packets=1 self_evolution_admission_evidence_ids=3 self_evolution_admission_missing_review_packet_refs=0\n",
    )
    .expect("write trace report fixture");
    let state_gate = env::temp_dir().join(format!(
        "norion-cli-state-gate-{}-{}.txt",
        std::process::id(),
        "issue30"
    ));
    fs::write(
        &state_gate,
        "state_inspection_gate: passed=true failures=0\n",
    )
    .expect("write state gate fixture");
    let issue30_context = env::temp_dir().join(format!(
        "norion-cli-issue30-context-{}-{}.txt",
        std::process::id(),
        "issue30"
    ));
    fs::write(
        &issue30_context,
        concat!(
            "issue30_environment_pressure_present=true issue30_pollution_event_id=redaction-digest:dddddddddddddddd issue385_self_ontology_body_present=true issue385_body_state_id=redaction-digest:eeeeeeeeeeeeeeee issue375_pre_reasoning_genome_isa_present=true issue375_reasoning_frame_id=redaction-digest:ffffffffffffffff issue30_backend_action=deterministic_runtime_kv_roundtrip issue379_control_candidate_preview_only=true issue379_action_vocab_mask_preview=true issue379_signal_saliency_bias_preview=true\n",
            "issue377_problem_finding_present=true issue377_problem_finding_id=redaction-digest:aaaaaaaaaaaaaaaa issue377_hypothesis_candidate_present=true issue377_hypothesis_candidate_id=redaction-digest:bbbbbbbbbbbbbbbb issue377_problem_hypothesis_link=redaction-digest:cccccccccccccccc issue377_admission_decision=preview_only\n",
        ),
    )
    .expect("write issue30 context fixture");
    fs::write(
        &input,
        concat!(
            "local_path=C:\\Users\\jy\\AppData\\Local\\Temp\\issue30.txt\n",
            "prompt: private raw prompt\n",
            "answer_text=raw answer\n",
            "hidden_cot=private chain-of-thought\n",
            "id=3 key=runtime_kv :: Design a Rust Noiron prototype lesson=reuse_response: raw model output\n",
            "OPENAI_API_KEY=sk-secret\n",
        ),
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
        "--git-worktree",
        git_worktree.to_str().expect("temp path should be utf-8"),
        "--release-review-input",
        release_review.to_str().expect("temp path should be utf-8"),
        "--issue-state-input",
        issue_state.to_str().expect("temp path should be utf-8"),
        "--demo-proof-input",
        demo_proof.to_str().expect("temp path should be utf-8"),
        "--roundtrip-proof-input",
        roundtrip_proof.to_str().expect("temp path should be utf-8"),
        "--trace-report-input",
        trace_report.to_str().expect("temp path should be utf-8"),
        "--state-gate-input",
        state_gate.to_str().expect("temp path should be utf-8"),
        "--issue30-context-input",
        issue30_context.to_str().expect("temp path should be utf-8"),
        "--require",
        "clean_checkout=true",
        "--require",
        git_sha_field.as_str(),
        "--require",
        "rc_sha_source=git_rev_parse",
        "--require",
        "rc_branch=issue30-fixture",
        "--require",
        "rc_branch_source=git_branch",
        "--require",
        "dirty_worktree=false",
        "--require",
        "dirty_worktree_source=git_status",
        "--require",
        "rc_snapshot_ready=true",
        "--require",
        "rc_snapshot_ready_source=git_status_derived",
        "--require",
        "rc_prs=#428,#429",
        "--require",
        "rc_prs_source=release_review_input",
        "--require",
        "live_model_required=false",
        "--require",
        "private_state_required=false",
        "--require",
        "trace_schema_gate: passed=true",
        "--require",
        "reasoning_genome_events=2",
        "--require",
        "reasoning_genome_write_allowed=0",
        "--require",
        "reasoning_genome_splice_write_allowed=0",
        "--require",
        "self_evolution_admission_events=1",
        "--require",
        "self_evolution_admission_review_packets=1",
        "--require",
        "self_evolution_admission_evidence_ids=3",
        "--require",
        "self_evolution_admission_missing_review_packet_refs=0",
        "--require",
        "self_evolution_admission_review_complete=true",
        "--require",
        "self_evolution_admission_review_complete_source=trace_report_input_derived",
        "--require",
        "trace_report_source=trace_report_input",
        "--require",
        "state_inspection_gate: passed=true",
        "--require",
        "state_gate_source=state_gate_input",
        "--require",
        "release_review_ready=false",
        "--require",
        "release_relevant_prs=#428,#429",
        "--require",
        "release_review_blockers=#428:REVIEW_REQUIRED,#429:REVIEW_REQUIRED",
        "--require",
        "release_review_source=release_review_input",
        "--require",
        "issue31_final_signoff_present=false",
        "--require",
        "issue31_final_signoff_source=issue_state_input",
        "--require",
        "issue19_runtime_surface_closed=false",
        "--require",
        "issue19_runtime_surface_merged_prs=#290,#291,#292,#293,#296,#307,#308,#309",
        "--require",
        "issue19_runtime_counters_pr=#429",
        "--require",
        "issue19_runtime_counters_ready=false",
        "--require",
        "issue19_runtime_counters_state=head_6f049dd_checks_green_review_required_unmerged",
        "--require",
        "issue19_runtime_counters_state_source=issue_state_input_derived",
        "--require",
        "issue19_runtime_surface_blocker=#429:REVIEW_REQUIRED",
        "--require",
        "issue19_runtime_surface_source=issue_state_input",
        "--require",
        "issue30_close_allowed=false",
        "--require",
        "issue30_close_allowed_source=issue_state_input",
        "--require",
        "issue30_demo_integration_test=issue30_clean_checkout_demo_writes_digest_only_evidence_packet",
        "--require",
        "issue30_demo_dispatch_test=issue30_dispatch_roundtrip_inspect_runs_trace_schema_gate",
        "--require",
        "issue30_demo_dispatch_path=dispatch::run",
        "--require",
        "issue30_demo_trace_schema_gate_executed=true",
        "--require",
        "issue30_clean_checkout_demo_ready=true",
        "--require",
        "issue30_clean_checkout_demo_ready_source=demo_proof_input_derived",
        "--require",
        "issue30_demo_source=demo_proof_input",
        "--require",
        "issue30_environment_pressure_present=true",
        "--require",
        "issue30_pollution_event_id=redaction-digest:",
        "--require",
        "issue385_self_ontology_body_present=true",
        "--require",
        "issue385_body_state_id=redaction-digest:",
        "--require",
        "issue375_pre_reasoning_genome_isa_present=true",
        "--require",
        "issue375_reasoning_frame_id=redaction-digest:",
        "--require",
        "issue30_backend_action=deterministic_runtime_kv_roundtrip",
        "--require",
        "issue379_control_candidate_preview_only=true",
        "--require",
        "issue379_action_vocab_mask_preview=true",
        "--require",
        "issue379_signal_saliency_bias_preview=true",
        "--require",
        "issue377_problem_finding_present=true",
        "--require",
        "issue377_problem_finding_id=redaction-digest:",
        "--require",
        "issue377_hypothesis_candidate_present=true",
        "--require",
        "issue377_hypothesis_candidate_id=redaction-digest:",
        "--require",
        "issue377_problem_hypothesis_link=redaction-digest:",
        "--require",
        "issue377_admission_decision=preview_only",
        "--require",
        "issue30_positive_context_loop_ready=true",
        "--require",
        "issue30_positive_context_loop_ready_source=issue30_context_input_derived",
        "--require",
        "issue30_context_source=issue30_context_input",
        "--require",
        "persistent_roundtrip: passed=true",
        "--require",
        "second_compute_budget_saved_tokens=320",
        "--require",
        "second_compute_budget_avoided_tokens=448",
        "--require",
        "second_compute_budget_kv_lookups_skipped=2",
        "--require",
        "second_compute_budget_reduced=true",
        "--require",
        "second_compute_budget_reduced_source=roundtrip_proof_input_derived",
        "--require",
        "second_approved_experience_reuse_digest=redaction-digest:",
        "--require",
        "second_compute_budget_anchor_count=2",
        "--require",
        "second_compute_budget_anchors_preserved=true",
        "--require",
        "second_compute_budget_anchors_preserved_source=roundtrip_proof_input_derived",
        "--require",
        "second_compute_budget_anchors_preserved_count=2",
        "--require",
        "second_quality=0.820",
        "--require",
        "first_drift=watch",
        "--require",
        "second_drift=watch",
        "--require",
        "failures=0",
        "--require",
        "negative_unauthorized_write_allowed=false",
        "--require",
        "negative_durable_write_allowed=false",
        "--require",
        "negative_durable_write_allowed_source=roundtrip_proof_input_derived",
        "--require",
        "negative_memory_write_allowed=false",
        "--require",
        "negative_genome_write_allowed=false",
        "--require",
        "negative_self_evolution_write_allowed=false",
        "--require",
        "negative_all_writes_denied=true",
        "--require",
        "negative_all_writes_denied_source=roundtrip_proof_input_derived",
        "--require",
        "negative_polluted_evidence_blocked=true",
        "--require",
        "negative_polluted_evidence_quarantined=true",
        "--require",
        "negative_polluted_evidence_contained=true",
        "--require",
        "negative_polluted_evidence_contained_source=roundtrip_proof_input_derived",
        "--require",
        "negative_bad_candidate_held_or_rolled_back=true",
        "--require",
        "negative_bad_candidate_held_or_rolled_back_source=roundtrip_proof_input_derived",
        "--require",
        "negative_bad_candidate_digest=redaction-digest:",
        "--require",
        "negative_bad_candidate_decision=hold_then_rollback",
        "--require",
        "negative_rollback_anchor_present=true",
        "--require",
        "negative_rollback_anchor_evidence_id=issue-30-roundtrip-negative-gate-hold",
        "--require",
        "negative_rollback_anchor_digest=redaction-digest:0123456789abcdef",
        "--require",
        "negative_tenant_scope_write_denied=true",
        "--require",
        "negative_tenant_scope_mode=local_single_user_preview",
        "--require",
        "negative_tenant_scope_actor=fnv64:",
        "--require",
        "negative_tenant_scope_target=fnv64:",
        "--require",
        "negative_tenant_scope_denial_lane=self_evolving_memory",
        "--require",
        "negative_tenant_scope_denial_reason=cross_tenant_scope_rejected",
        "--require",
        "negative_single_tenant_preview=true",
        "--require",
        "negative_single_tenant_preview_source=roundtrip_proof_input_derived",
        "--require",
        "negative_tenant_scope_boundary_ok=true",
        "--require",
        "negative_tenant_scope_boundary_ok_source=roundtrip_proof_input_derived",
        "--require",
        "negative_provenance_license_redaction_passed=true",
        "--require",
        "negative_digest_only=true",
        "--require",
        "negative_digest_only_source=roundtrip_proof_input_derived",
        "--require",
        "issue30_roundtrip_source=roundtrip_proof_input",
        "--reject",
        "C:\\Users",
        "--reject",
        "private raw prompt",
        "--reject",
        "chain-of-thought",
        "--reject",
        "reuse_response",
    ]);

    assert!(output.status.success());
    let out = stdout(&output);
    assert!(out.contains("## Evidence packet for #30"));
    assert!(out.contains("--trace-schema-gate"));
    assert!(out.contains("clean_checkout=true"));
    assert!(out.contains(&git_sha_field));
    assert!(out.contains("rc_sha_source=git_rev_parse"));
    assert!(out.contains("rc_branch=issue30-fixture"));
    assert!(out.contains("rc_branch_source=git_branch"));
    assert!(out.contains("dirty_worktree=false dirty_worktree_source=git_status"));
    assert!(out.contains("rc_snapshot_ready=true"));
    assert!(out.contains("rc_snapshot_ready_source=git_status_derived"));
    assert!(out.contains("rc_prs=#428,#429"));
    assert!(out.contains("rc_prs_source=release_review_input"));
    assert!(out.contains("release_review_ready=false"));
    assert!(out.contains("release_relevant_prs=#428,#429"));
    assert!(out.contains("release_review_blockers=#428:REVIEW_REQUIRED,#429:REVIEW_REQUIRED"));
    assert!(out.contains("release_review_source=release_review_input"));
    assert!(out.contains("issue31_final_signoff_present=false"));
    assert!(out.contains("issue31_final_signoff_source=issue_state_input"));
    assert!(out.contains("issue19_runtime_surface_closed=false"));
    assert!(
        out.contains("issue19_runtime_surface_merged_prs=#290,#291,#292,#293,#296,#307,#308,#309")
    );
    assert!(out.contains("issue19_runtime_counters_pr=#429"));
    assert!(out.contains("issue19_runtime_counters_ready=false"));
    assert!(out.contains(
        "issue19_runtime_counters_state=head_6f049dd_checks_green_review_required_unmerged"
    ));
    assert!(out.contains("issue19_runtime_counters_state_source=issue_state_input_derived"));
    assert!(out.contains("issue19_runtime_surface_blocker=#429:REVIEW_REQUIRED"));
    assert!(out.contains("issue19_runtime_surface_source=issue_state_input"));
    assert!(out.contains("issue30_close_allowed=false"));
    assert!(out.contains("issue30_close_allowed_source=issue_state_input"));
    assert!(out.contains(
        "issue30_demo_integration_test=issue30_clean_checkout_demo_writes_digest_only_evidence_packet"
    ));
    assert!(out.contains(
        "issue30_demo_dispatch_test=issue30_dispatch_roundtrip_inspect_runs_trace_schema_gate"
    ));
    assert!(out.contains("issue30_demo_dispatch_path=dispatch::run"));
    assert!(out.contains("issue30_demo_trace_schema_gate_executed=true"));
    assert!(out.contains("issue30_clean_checkout_demo_ready=true"));
    assert!(out.contains("issue30_clean_checkout_demo_ready_source=demo_proof_input_derived"));
    assert!(out.contains("issue30_demo_source=demo_proof_input"));
    assert!(out.contains("trace_schema_gate: passed=true"));
    assert!(out.contains("reasoning_genome_events=2"));
    assert!(out.contains("reasoning_genome_write_allowed=0"));
    assert!(out.contains("reasoning_genome_splice_write_allowed=0"));
    assert!(out.contains("self_evolution_admission_events=1"));
    assert!(out.contains("self_evolution_admission_review_packets=1"));
    assert!(out.contains("self_evolution_admission_evidence_ids=3"));
    assert!(out.contains("self_evolution_admission_missing_review_packet_refs=0"));
    assert!(out.contains("self_evolution_admission_review_complete=true"));
    assert!(
        out.contains("self_evolution_admission_review_complete_source=trace_report_input_derived")
    );
    assert!(out.contains("trace_report_source=trace_report_input"));
    assert!(out.contains("state_inspection_gate: passed=true failures=0"));
    assert!(out.contains("state_gate_source=state_gate_input"));
    assert!(out.contains("issue30_environment_pressure_present=true"));
    assert!(out.contains("issue30_pollution_event_id=redaction-digest:"));
    assert!(out.contains("issue385_self_ontology_body_present=true"));
    assert!(out.contains("issue385_body_state_id=redaction-digest:"));
    assert!(out.contains("issue375_pre_reasoning_genome_isa_present=true"));
    assert!(out.contains("issue375_reasoning_frame_id=redaction-digest:"));
    assert!(out.contains("issue30_backend_action=deterministic_runtime_kv_roundtrip"));
    assert!(out.contains("issue379_control_candidate_preview_only=true"));
    assert!(out.contains("issue379_action_vocab_mask_preview=true"));
    assert!(out.contains("issue379_signal_saliency_bias_preview=true"));
    assert!(out.contains("issue377_problem_finding_present=true"));
    assert!(out.contains("issue377_problem_finding_id=redaction-digest:"));
    assert!(out.contains("issue377_hypothesis_candidate_present=true"));
    assert!(out.contains("issue377_hypothesis_candidate_id=redaction-digest:"));
    assert!(out.contains("issue377_problem_hypothesis_link=redaction-digest:"));
    assert!(out.contains("issue377_admission_decision=preview_only"));
    assert!(out.contains("issue30_positive_context_loop_ready=true"));
    assert!(
        out.contains("issue30_positive_context_loop_ready_source=issue30_context_input_derived")
    );
    assert!(out.contains("issue30_context_source=issue30_context_input"));
    assert!(out.contains("persistent_roundtrip: passed=true"));
    assert!(out.contains("second_compute_budget_saved_tokens=320"));
    assert!(out.contains("second_compute_budget_avoided_tokens=448"));
    assert!(out.contains("second_compute_budget_kv_lookups_skipped=2"));
    assert!(out.contains("second_compute_budget_reduced=true"));
    assert!(out.contains("second_compute_budget_reduced_source=roundtrip_proof_input_derived"));
    assert!(out.contains("second_approved_experience_reuse_digest=redaction-digest:"));
    assert!(out.contains("second_compute_budget_anchor_count=2"));
    assert!(out.contains("second_compute_budget_anchors_preserved=true"));
    assert!(
        out.contains(
            "second_compute_budget_anchors_preserved_source=roundtrip_proof_input_derived"
        )
    );
    assert!(out.contains("second_compute_budget_anchors_preserved_count=2"));
    assert!(out.contains("second_quality=0.820"));
    assert!(out.contains("first_drift=watch"));
    assert!(out.contains("second_drift=watch"));
    assert!(out.contains("failures=0"));
    assert!(out.contains("negative_unauthorized_write_allowed=false"));
    assert!(out.contains("negative_durable_write_allowed=false"));
    assert!(out.contains("negative_durable_write_allowed_source=roundtrip_proof_input_derived"));
    assert!(out.contains("negative_memory_write_allowed=false"));
    assert!(out.contains("negative_genome_write_allowed=false"));
    assert!(out.contains("negative_self_evolution_write_allowed=false"));
    assert!(out.contains("negative_all_writes_denied=true"));
    assert!(out.contains("negative_all_writes_denied_source=roundtrip_proof_input_derived"));
    assert!(out.contains("negative_polluted_evidence_blocked=true"));
    assert!(out.contains("negative_polluted_evidence_quarantined=true"));
    assert!(out.contains("negative_polluted_evidence_contained=true"));
    assert!(
        out.contains("negative_polluted_evidence_contained_source=roundtrip_proof_input_derived")
    );
    assert!(out.contains("negative_bad_candidate_held_or_rolled_back=true"));
    assert!(out.contains(
        "negative_bad_candidate_held_or_rolled_back_source=roundtrip_proof_input_derived"
    ));
    assert!(out.contains("negative_bad_candidate_digest=redaction-digest:"));
    assert!(out.contains("negative_bad_candidate_decision=hold_then_rollback"));
    assert!(out.contains("negative_rollback_anchor_present=true"));
    assert!(
        out.contains("negative_rollback_anchor_evidence_id=issue-30-roundtrip-negative-gate-hold")
    );
    assert!(out.contains("negative_rollback_anchor_digest=redaction-digest:0123456789abcdef"));
    assert!(out.contains("negative_tenant_scope_write_denied=true"));
    assert!(out.contains("negative_tenant_scope_mode=local_single_user_preview"));
    assert!(out.contains("negative_tenant_scope_actor=fnv64:"));
    assert!(out.contains("negative_tenant_scope_target=fnv64:"));
    assert!(out.contains("negative_tenant_scope_denial_lane=self_evolving_memory"));
    assert!(out.contains("negative_tenant_scope_denial_reason=cross_tenant_scope_rejected"));
    assert!(out.contains("negative_single_tenant_preview=true"));
    assert!(out.contains("negative_single_tenant_preview_source=roundtrip_proof_input_derived"));
    assert!(out.contains("negative_tenant_scope_boundary_ok=true"));
    assert!(out.contains("negative_tenant_scope_boundary_ok_source=roundtrip_proof_input_derived"));
    assert!(out.contains("negative_provenance_license_redaction_passed=true"));
    assert!(out.contains("negative_digest_only=true"));
    assert!(out.contains("negative_digest_only_source=roundtrip_proof_input_derived"));
    assert!(out.contains("issue30_roundtrip_source=roundtrip_proof_input"));
    assert!(out.contains("local_path=<redacted-path>"));
    assert!(out.contains("prompt=<redacted-payload>"));
    assert!(out.contains("answer_text=<redacted-payload>"));
    assert!(out.contains("hidden_cot=<redacted-payload>"));
    assert!(out.contains("payload_line=<redacted-payload>"));
    assert!(out.contains("OPENAI_API_KEY=<redacted>"));
    assert!(!out.contains("C:\\Users"));
    assert!(!out.contains("AppData"));
    assert!(!out.contains("private raw prompt"));
    assert!(!out.contains("chain-of-thought"));
    assert!(!out.contains("raw answer"));
    assert!(!out.contains("Design a Rust Noiron prototype"));
    assert!(!out.contains("reuse_response"));
    assert!(!out.contains("sk-secret"));
    assert!(stderr(&output).is_empty());

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(release_review);
    let _ = fs::remove_file(issue_state);
    let _ = fs::remove_file(demo_proof);
    let _ = fs::remove_file(roundtrip_proof);
    let _ = fs::remove_file(trace_report);
    let _ = fs::remove_file(state_gate);
    let _ = fs::remove_file(issue30_context);
    let _ = fs::remove_dir_all(git_worktree);
}

#[test]
fn evidence_packet_fails_when_required_issue30_field_is_missing() {
    let input = env::temp_dir().join(format!(
        "norion-cli-evidence-{}-{}.txt",
        std::process::id(),
        "issue30-missing"
    ));
    fs::write(&input, "trace_schema_gate: passed=true\n").expect("write issue30 evidence input");

    let output = run_cli(&[
        "evidence-packet",
        "--issue",
        "30",
        "--commit",
        "missing-field",
        "--command",
        "cargo run -- --benchmark-roundtrip",
        "--gate",
        "passed",
        "--input",
        input.to_str().expect("temp path should be utf-8"),
        "--require",
        "clean_checkout=true",
    ]);

    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("missing required evidence: clean_checkout=true"));
    assert!(stdout(&output).is_empty());

    let _ = fs::remove_file(input);
}
