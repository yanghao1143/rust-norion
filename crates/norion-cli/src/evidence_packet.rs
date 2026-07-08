use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidencePacketConfig {
    pub issue: String,
    pub commit: String,
    pub command: String,
    pub gate: String,
    pub input: PathBuf,
    pub output: Option<PathBuf>,
    pub git_worktree: Option<PathBuf>,
    pub release_review_input: Option<PathBuf>,
    pub issue_state_input: Option<PathBuf>,
    pub demo_proof_input: Option<PathBuf>,
    pub roundtrip_proof_input: Option<PathBuf>,
    pub trace_report_input: Option<PathBuf>,
    pub state_gate_input: Option<PathBuf>,
    pub research_sandbox_input: Option<PathBuf>,
    pub issue30_context_input: Option<PathBuf>,
    pub issue243_fixture_matrix_input: Option<PathBuf>,
    pub state_files_input: Option<PathBuf>,
    pub required: Vec<String>,
    pub rejected: Vec<String>,
}

pub fn parse_evidence_packet_args<I, S>(args: I) -> Result<EvidencePacketConfig, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut issue = None;
    let mut commit = None;
    let mut command = None;
    let mut gate = None;
    let mut input = None;
    let mut output = None;
    let mut git_worktree = None;
    let mut release_review_input = None;
    let mut issue_state_input = None;
    let mut demo_proof_input = None;
    let mut roundtrip_proof_input = None;
    let mut trace_report_input = None;
    let mut state_gate_input = None;
    let mut research_sandbox_input = None;
    let mut issue30_context_input = None;
    let mut issue243_fixture_matrix_input = None;
    let mut state_files_input = None;
    let mut required_fields = Vec::new();
    let mut rejected_fields = Vec::new();
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        let arg = arg.as_ref().to_owned();
        let (name, inline_value) = split_option(&arg)?;
        match name {
            "--issue" => issue = Some(option_value(name, inline_value, &mut args)?),
            "--commit" => commit = Some(option_value(name, inline_value, &mut args)?),
            "--command" => command = Some(option_value(name, inline_value, &mut args)?),
            "--gate" => gate = Some(parse_gate(&option_value(name, inline_value, &mut args)?)?),
            "--input" => input = Some(PathBuf::from(option_value(name, inline_value, &mut args)?)),
            "--output" => {
                output = Some(PathBuf::from(option_value(name, inline_value, &mut args)?))
            }
            "--git-worktree" => {
                git_worktree = Some(PathBuf::from(option_value(name, inline_value, &mut args)?))
            }
            "--release-review-input" => {
                release_review_input =
                    Some(PathBuf::from(option_value(name, inline_value, &mut args)?))
            }
            "--issue-state-input" => {
                issue_state_input =
                    Some(PathBuf::from(option_value(name, inline_value, &mut args)?))
            }
            "--demo-proof-input" => {
                demo_proof_input = Some(PathBuf::from(option_value(name, inline_value, &mut args)?))
            }
            "--roundtrip-proof-input" => {
                roundtrip_proof_input =
                    Some(PathBuf::from(option_value(name, inline_value, &mut args)?))
            }
            "--trace-report-input" => {
                trace_report_input =
                    Some(PathBuf::from(option_value(name, inline_value, &mut args)?))
            }
            "--state-gate-input" => {
                state_gate_input = Some(PathBuf::from(option_value(name, inline_value, &mut args)?))
            }
            "--research-sandbox-input" => {
                research_sandbox_input =
                    Some(PathBuf::from(option_value(name, inline_value, &mut args)?))
            }
            "--issue30-context-input" => {
                issue30_context_input =
                    Some(PathBuf::from(option_value(name, inline_value, &mut args)?))
            }
            "--issue243-fixture-matrix-input" => {
                issue243_fixture_matrix_input =
                    Some(PathBuf::from(option_value(name, inline_value, &mut args)?))
            }
            "--state-files-input" => {
                state_files_input =
                    Some(PathBuf::from(option_value(name, inline_value, &mut args)?))
            }
            "--require" => required_fields.push(option_value(name, inline_value, &mut args)?),
            "--reject" => rejected_fields.push(option_value(name, inline_value, &mut args)?),
            _ => return Err(format!("unknown evidence-packet option: {name}")),
        }
    }

    Ok(EvidencePacketConfig {
        issue: required("--issue", issue)?,
        commit: required("--commit", commit)?,
        command: required("--command", command)?,
        gate: required("--gate", gate)?,
        input: input.ok_or_else(|| "missing --input".to_owned())?,
        output,
        git_worktree,
        release_review_input,
        issue_state_input,
        demo_proof_input,
        roundtrip_proof_input,
        trace_report_input,
        state_gate_input,
        research_sandbox_input,
        issue30_context_input,
        issue243_fixture_matrix_input,
        state_files_input,
        required: required_fields,
        rejected: rejected_fields,
    })
}

pub fn run_evidence_packet(config: &EvidencePacketConfig) -> Result<String, String> {
    let raw = fs::read_to_string(&config.input)
        .map_err(|error| format!("failed to read {}: {error}", config.input.display()))?;
    let mut generated = Vec::new();
    if let Some(worktree) = config.git_worktree.as_deref() {
        generated.push(git_worktree_statement(worktree)?);
    }
    if let Some(path) = config.release_review_input.as_deref() {
        generated.push(release_review_statement(path)?);
    }
    if let Some(path) = config.issue_state_input.as_deref() {
        generated.push(issue_state_statement(path)?);
    }
    if let Some(path) = config.demo_proof_input.as_deref() {
        generated.push(demo_proof_statement(path)?);
    }
    if let Some(path) = config.roundtrip_proof_input.as_deref() {
        generated.push(roundtrip_proof_statement(path)?);
    }
    if let Some(path) = config.trace_report_input.as_deref() {
        generated.push(trace_report_statement(path)?);
    }
    if let Some(path) = config.state_gate_input.as_deref() {
        generated.push(state_gate_statement(path)?);
    }
    if let Some(path) = config.research_sandbox_input.as_deref() {
        generated.push(research_sandbox_statement(path)?);
    }
    if let Some(path) = config.issue30_context_input.as_deref() {
        generated.push(issue30_context_statement(path)?);
    }
    if let Some(path) = config.issue243_fixture_matrix_input.as_deref() {
        generated.push(issue243_fixture_matrix_statement(path)?);
    }
    if let Some(path) = config.state_files_input.as_deref() {
        generated.push(state_files_statement(path)?);
    }
    let packet = render_evidence_packet(config, &raw, &generated);
    validate_packet(config, &packet)?;
    if let Some(path) = &config.output {
        fs::write(path, &packet)
            .map_err(|error| format!("failed to write {}: {error}", path.display()))?;
    }
    Ok(packet)
}

fn render_evidence_packet(
    config: &EvidencePacketConfig,
    raw: &str,
    generated: &[String],
) -> String {
    let generated = if generated.is_empty() {
        String::new()
    } else {
        format!("{}\n", generated.join("\n"))
    };
    format!(
        "## Evidence packet for #{}\n- commit: {}\n- command: {}\n- gate: {}\n\n```text\n{}{}\n```\n",
        config.issue.trim_start_matches('#'),
        config.commit,
        redact(&config.command),
        config.gate,
        generated,
        redact(raw).trim_end()
    )
}

fn git_worktree_statement(worktree: &Path) -> Result<String, String> {
    let rc_sha = git_trimmed_output(worktree, &["rev-parse", "HEAD"], "git rev-parse HEAD")?;
    let rc_branch = git_trimmed_output(
        worktree,
        &["branch", "--show-current"],
        "git branch --show-current",
    )?;
    let rc_branch = if rc_branch.is_empty() {
        "detached".to_owned()
    } else {
        rc_branch
    };
    let status = git_trimmed_output(worktree, &["status", "--short"], "git status")?;
    let dirty = !status.is_empty();
    let snapshot_ready = !dirty && !rc_sha.is_empty() && !rc_branch.is_empty();
    Ok(format!(
        "rc_sha={rc_sha} rc_sha_source=git_rev_parse rc_branch={rc_branch} rc_branch_source=git_branch dirty_worktree={dirty} dirty_worktree_source=git_status rc_snapshot_ready={snapshot_ready} rc_snapshot_ready_source=git_status_derived"
    ))
}

fn git_trimmed_output(worktree: &Path, args: &[&str], context: &str) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(worktree)
        .args(args)
        .output()
        .map_err(|error| {
            format!(
                "failed to run {context} for {}: {error}",
                worktree.display()
            )
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "{context} failed for {}: {}",
            worktree.display(),
            stderr.trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn release_review_statement(path: &Path) -> Result<String, String> {
    let raw = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let mut prs = Vec::new();
    let mut blockers = Vec::new();

    for (index, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let pr = release_field(line, "pr")
            .ok_or_else(|| format!("{}:{} missing pr field", path.display(), index + 1))?;
        let pr = normalize_pr(pr);
        let review = release_field(line, "review")
            .or_else(|| release_field(line, "review_decision"))
            .unwrap_or("MISSING_REVIEW_EVIDENCE");
        let checks = release_field(line, "checks").unwrap_or("missing");
        let branch_protection = release_field(line, "branch_protection").unwrap_or("missing");

        prs.push(pr.clone());
        if review == "REVIEW_REQUIRED" {
            blockers.push(format!("{pr}:REVIEW_REQUIRED"));
        } else if review != "APPROVED" && review != "MERGED" {
            blockers.push(format!("{pr}:REVIEW_{review}"));
        }
        if checks != "passed" && checks != "pass" {
            blockers.push(format!("{pr}:CHECKS_{}", checks.to_ascii_uppercase()));
        }
        if branch_protection != "present" && branch_protection != "not_required" {
            blockers.push(format!("{pr}:MISSING_BRANCH_PROTECTION_EVIDENCE"));
        }
    }

    if prs.is_empty() {
        return Err(format!(
            "{} has no release review evidence rows",
            path.display()
        ));
    }

    let ready = blockers.is_empty();
    let blockers = if blockers.is_empty() {
        "none".to_owned()
    } else {
        blockers.join(",")
    };
    Ok(format!(
        "rc_prs={} rc_prs_source=release_review_input release_relevant_prs={} release_review_ready={ready} release_review_blockers={blockers} release_review_source=release_review_input",
        prs.join(","),
        prs.join(",")
    ))
}

fn release_field<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    line.split_whitespace().find_map(|field| {
        let (key, value) = field.split_once('=')?;
        (key == name).then_some(value)
    })
}

fn normalize_pr(value: &str) -> String {
    if value.starts_with('#') {
        value.to_owned()
    } else {
        format!("#{value}")
    }
}

fn issue_state_statement(path: &Path) -> Result<String, String> {
    let raw = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let mut issue31_final_signoff = None;
    let mut issue19_runtime_surface_closed = None;
    let mut issue19_runtime_surface_merged_prs = None;
    let mut issue19_runtime_counters_pr = None;
    let mut issue19_runtime_counters_ready = None;
    let mut issue19_runtime_counters_state = None;
    let mut issue19_runtime_surface_blocker = None;
    let mut issue30_close_allowed = None;

    for (index, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let issue = release_field(line, "issue")
            .ok_or_else(|| format!("{}:{} missing issue field", path.display(), index + 1))?;
        match issue.trim_start_matches('#') {
            "31" => {
                issue31_final_signoff = Some(
                    release_field(line, "final_signoff")
                        .or_else(|| release_field(line, "issue31_final_signoff_present"))
                        .ok_or_else(|| {
                            format!(
                                "{}:{} missing issue31 final_signoff",
                                path.display(),
                                index + 1
                            )
                        })?
                        .to_owned(),
                );
            }
            "19" => {
                issue19_runtime_surface_closed = Some(required_issue_field(
                    path,
                    index,
                    line,
                    "runtime_surface_closed",
                )?);
                issue19_runtime_surface_merged_prs = Some(required_issue_field(
                    path,
                    index,
                    line,
                    "runtime_surface_merged_prs",
                )?);
                issue19_runtime_counters_pr = Some(required_issue_field(
                    path,
                    index,
                    line,
                    "runtime_counters_pr",
                )?);
                let runtime_counters_head =
                    required_issue_field(path, index, line, "runtime_counters_head")?;
                let runtime_counters_checks =
                    required_issue_field(path, index, line, "runtime_counters_checks")?;
                let runtime_counters_review =
                    required_issue_field(path, index, line, "runtime_counters_review")?;
                let runtime_counters_merged =
                    required_issue_field(path, index, line, "runtime_counters_merged")?;
                issue19_runtime_counters_ready = Some(derive_issue19_runtime_counters_ready(
                    path,
                    index,
                    line,
                    &runtime_counters_checks,
                    &runtime_counters_review,
                    &runtime_counters_merged,
                )?);
                issue19_runtime_counters_state = Some(format!(
                    "head_{}_checks_{}_{}_{}",
                    short_head(&runtime_counters_head),
                    state_token(&runtime_counters_checks),
                    state_token(&runtime_counters_review),
                    if runtime_counters_merged == "true" {
                        "merged"
                    } else {
                        "unmerged"
                    }
                ));
                issue19_runtime_surface_blocker = Some(required_issue_field(
                    path,
                    index,
                    line,
                    "runtime_surface_blocker",
                )?);
            }
            "30" => {
                issue30_close_allowed = Some(
                    release_field(line, "close_allowed")
                        .or_else(|| release_field(line, "issue30_close_allowed"))
                        .ok_or_else(|| {
                            format!(
                                "{}:{} missing issue30 close_allowed",
                                path.display(),
                                index + 1
                            )
                        })?
                        .to_owned(),
                );
            }
            _ => {}
        }
    }

    Ok(format!(
        "issue31_final_signoff_present={} issue31_final_signoff_source=issue_state_input issue19_runtime_surface_closed={} issue19_runtime_surface_merged_prs={} issue19_runtime_counters_pr={} issue19_runtime_counters_ready={} issue19_runtime_counters_ready_source=issue_state_input_derived issue19_runtime_counters_state={} issue19_runtime_counters_state_source=issue_state_input_derived issue19_runtime_surface_blocker={} issue19_runtime_surface_source=issue_state_input issue30_close_allowed={} issue30_close_allowed_source=issue_state_input",
        required_state(&issue31_final_signoff, path, "issue31 final_signoff")?,
        required_state(
            &issue19_runtime_surface_closed,
            path,
            "issue19 runtime_surface_closed"
        )?,
        required_state(
            &issue19_runtime_surface_merged_prs,
            path,
            "issue19 runtime_surface_merged_prs"
        )?,
        required_state(
            &issue19_runtime_counters_pr,
            path,
            "issue19 runtime_counters_pr"
        )?,
        required_state(
            &issue19_runtime_counters_ready,
            path,
            "issue19 runtime_counters_ready"
        )?,
        required_state(
            &issue19_runtime_counters_state,
            path,
            "issue19 runtime_counters_state"
        )?,
        required_state(
            &issue19_runtime_surface_blocker,
            path,
            "issue19 runtime_surface_blocker"
        )?,
        required_state(&issue30_close_allowed, path, "issue30 close_allowed")?,
    ))
}

fn short_head(value: &str) -> &str {
    value.get(..7).unwrap_or(value)
}

fn state_token(value: &str) -> String {
    value.to_ascii_lowercase().replace('-', "_")
}

fn derive_issue19_runtime_counters_ready(
    path: &Path,
    index: usize,
    line: &str,
    checks: &str,
    review: &str,
    merged: &str,
) -> Result<String, String> {
    let derived = matches!(state_token(checks).as_str(), "green" | "pass" | "passed")
        && matches!(state_token(review).as_str(), "approved" | "merged")
        && merged == "true";
    let derived = derived.to_string();

    if let Some(raw_value) = release_field(line, "runtime_counters_ready") {
        if raw_value != derived {
            return Err(format!(
                "{}:{} runtime_counters_ready conflicts with checks/review/merged fields",
                path.display(),
                index + 1
            ));
        }
    }

    Ok(derived)
}

fn required_issue_field(
    path: &Path,
    index: usize,
    line: &str,
    field: &str,
) -> Result<String, String> {
    release_field(line, field)
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("{}:{} missing {field}", path.display(), index + 1))
}

fn required_state<'a>(
    value: &'a Option<String>,
    path: &Path,
    field: &str,
) -> Result<&'a str, String> {
    value
        .as_deref()
        .ok_or_else(|| format!("{} missing {field}", path.display()))
}

fn demo_proof_statement(path: &Path) -> Result<String, String> {
    let raw = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    for (index, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let clean_checkout = required_issue_field(path, index, line, "clean_checkout")?;
        let live_model_required = required_issue_field(path, index, line, "live_model_required")?;
        let private_state_required =
            required_issue_field(path, index, line, "private_state_required")?;
        let prompt_digest_ref = required_issue_field(path, index, line, "prompt_digest_ref")?;
        let integration_test = required_issue_field(path, index, line, "integration_test")?;
        let dispatch_test = required_issue_field(path, index, line, "dispatch_test")?;
        let dispatch_path = required_issue_field(path, index, line, "dispatch_path")?;
        let trace_schema_gate_executed =
            required_issue_field(path, index, line, "trace_schema_gate_executed")?;
        let clean_checkout_demo_ready = issue30_clean_checkout_demo_ready(path, index, line)?;
        return Ok(format!(
            "clean_checkout={clean_checkout} live_model_required={live_model_required} private_state_required={private_state_required} prompt_digest_ref={prompt_digest_ref} issue30_demo_integration_test={integration_test} issue30_demo_dispatch_test={dispatch_test} issue30_demo_dispatch_path={dispatch_path} issue30_demo_trace_schema_gate_executed={trace_schema_gate_executed}{clean_checkout_demo_ready} issue30_demo_source=demo_proof_input"
        ));
    }
    Err(format!("{} has no demo proof rows", path.display()))
}

fn issue30_clean_checkout_demo_ready(
    path: &Path,
    index: usize,
    line: &str,
) -> Result<String, String> {
    let derived = release_field(line, "clean_checkout") == Some("true")
        && release_field(line, "live_model_required") == Some("false")
        && release_field(line, "private_state_required") == Some("false")
        && release_field(line, "prompt_digest_ref")
            .is_some_and(|value| value.starts_with("redaction-digest:"))
        && release_field(line, "integration_test")
            == Some("issue30_clean_checkout_demo_writes_digest_only_evidence_packet")
        && release_field(line, "dispatch_test")
            == Some("issue30_dispatch_roundtrip_inspect_runs_trace_schema_gate")
        && release_field(line, "dispatch_path") == Some("dispatch::run")
        && release_field(line, "trace_schema_gate_executed") == Some("true");
    if let Some(raw_value) = release_field(line, "issue30_clean_checkout_demo_ready") {
        if raw_value != derived.to_string() {
            return Err(format!(
                "{}:{} issue30_clean_checkout_demo_ready conflicts with demo proof fields",
                path.display(),
                index + 1
            ));
        }
        Ok(" issue30_clean_checkout_demo_ready_source=demo_proof_input_derived".to_owned())
    } else {
        Ok(format!(
            " issue30_clean_checkout_demo_ready={derived} issue30_clean_checkout_demo_ready_source=demo_proof_input_derived"
        ))
    }
}

fn roundtrip_proof_statement(path: &Path) -> Result<String, String> {
    let raw = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    for (index, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if !line.starts_with("persistent_roundtrip: ") {
            return Err(format!(
                "{}:{} expected persistent_roundtrip summary line",
                path.display(),
                index + 1
            ));
        }
        let compute_budget_reduced = roundtrip_compute_budget_reduced(path, index, line)?;
        let planning_dense_compute_reduced =
            roundtrip_planning_dense_compute_reduced(path, index, line)?;
        let compute_anchors_preserved = roundtrip_compute_anchors_preserved(path, index, line)?;
        let second_task_benefit_ready = roundtrip_second_task_benefit_ready(path, index, line)?;
        let disk_kv_roundtrip_ready = roundtrip_disk_kv_roundtrip_ready(path, index, line)?;
        let negative_all_writes_denied = roundtrip_negative_all_writes_denied(path, index, line)?;
        let polluted_evidence_contained =
            roundtrip_negative_polluted_evidence_contained(path, index, line)?;
        let tenant_scope_boundary_ok =
            roundtrip_negative_tenant_scope_boundary_ok(path, index, line)?;
        let negative_gates_ready = roundtrip_negative_gates_ready(path, index, line)?;
        let durable_write_allowed = if let Some(unauthorized_write_allowed) =
            release_field(line, "negative_unauthorized_write_allowed")
        {
            if let Some(raw_value) = release_field(line, "negative_durable_write_allowed") {
                if raw_value != unauthorized_write_allowed {
                    return Err(format!(
                        "{}:{} negative_durable_write_allowed conflicts with unauthorized write gate",
                        path.display(),
                        index + 1
                    ));
                }
                " negative_durable_write_allowed_source=roundtrip_proof_input_derived".to_owned()
            } else {
                format!(
                    " negative_durable_write_allowed={unauthorized_write_allowed} negative_durable_write_allowed_source=roundtrip_proof_input_derived"
                )
            }
        } else {
            String::new()
        };
        let single_tenant_preview = if let Some(tenant_scope_mode) =
            release_field(line, "negative_tenant_scope_mode")
        {
            let derived = tenant_scope_mode == "local_single_user_preview";
            if let Some(raw_value) = release_field(line, "negative_single_tenant_preview") {
                if raw_value != derived.to_string() {
                    return Err(format!(
                        "{}:{} negative_single_tenant_preview conflicts with tenant scope mode",
                        path.display(),
                        index + 1
                    ));
                }
                " negative_single_tenant_preview_source=roundtrip_proof_input_derived".to_owned()
            } else {
                format!(
                    " negative_single_tenant_preview={derived} negative_single_tenant_preview_source=roundtrip_proof_input_derived"
                )
            }
        } else {
            String::new()
        };
        let held_or_rolled_back = match (
            release_field(line, "negative_bad_candidate_decision"),
            release_field(line, "negative_rollback_anchor_present"),
        ) {
            (Some(decision), Some(rollback_anchor_present)) => {
                let derived = decision == "hold_then_rollback" && rollback_anchor_present == "true";
                if let Some(raw_value) =
                    release_field(line, "negative_bad_candidate_held_or_rolled_back")
                {
                    if raw_value != derived.to_string() {
                        return Err(format!(
                            "{}:{} negative_bad_candidate_held_or_rolled_back conflicts with decision and rollback anchor",
                            path.display(),
                            index + 1
                        ));
                    }
                    " negative_bad_candidate_held_or_rolled_back_source=roundtrip_proof_input_derived".to_owned()
                } else {
                    format!(
                        " negative_bad_candidate_held_or_rolled_back={derived} negative_bad_candidate_held_or_rolled_back_source=roundtrip_proof_input_derived"
                    )
                }
            }
            _ => String::new(),
        };
        let digest_only = if let Some(derived) = roundtrip_digest_only(line) {
            if let Some(raw_value) = release_field(line, "negative_digest_only") {
                if raw_value != derived.to_string() {
                    return Err(format!(
                        "{}:{} negative_digest_only conflicts with digest proof inputs",
                        path.display(),
                        index + 1
                    ));
                }
                " negative_digest_only_source=roundtrip_proof_input_derived".to_owned()
            } else {
                format!(
                    " negative_digest_only={derived} negative_digest_only_source=roundtrip_proof_input_derived"
                )
            }
        } else {
            String::new()
        };
        return Ok(format!(
            "{line}{compute_budget_reduced}{planning_dense_compute_reduced}{compute_anchors_preserved}{second_task_benefit_ready}{disk_kv_roundtrip_ready}{durable_write_allowed}{negative_all_writes_denied}{polluted_evidence_contained}{tenant_scope_boundary_ok}{single_tenant_preview}{held_or_rolled_back}{digest_only}{negative_gates_ready} issue30_roundtrip_source=roundtrip_proof_input"
        ));
    }
    Err(format!("{} has no roundtrip proof rows", path.display()))
}

fn roundtrip_compute_budget_reduced(
    path: &Path,
    index: usize,
    line: &str,
) -> Result<String, String> {
    let Some(saved_tokens) = release_field(line, "second_compute_budget_saved_tokens") else {
        return Ok(String::new());
    };
    let Some(avoided_tokens) = release_field(line, "second_compute_budget_avoided_tokens") else {
        return Ok(String::new());
    };
    let Some(kv_lookups_skipped) = release_field(line, "second_compute_budget_kv_lookups_skipped")
    else {
        return Ok(String::new());
    };
    let saved_tokens = roundtrip_usize_field(
        path,
        index,
        "second_compute_budget_saved_tokens",
        saved_tokens,
    )?;
    let avoided_tokens = roundtrip_usize_field(
        path,
        index,
        "second_compute_budget_avoided_tokens",
        avoided_tokens,
    )?;
    let kv_lookups_skipped = roundtrip_usize_field(
        path,
        index,
        "second_compute_budget_kv_lookups_skipped",
        kv_lookups_skipped,
    )?;
    let derived = saved_tokens > 0 || avoided_tokens > 0 || kv_lookups_skipped > 0;
    if let Some(raw_value) = release_field(line, "second_compute_budget_reduced") {
        if raw_value != derived.to_string() {
            return Err(format!(
                "{}:{} second_compute_budget_reduced conflicts with compute budget counters",
                path.display(),
                index + 1
            ));
        }
        Ok(" second_compute_budget_reduced_source=roundtrip_proof_input_derived".to_owned())
    } else {
        Ok(format!(
            " second_compute_budget_reduced={derived} second_compute_budget_reduced_source=roundtrip_proof_input_derived"
        ))
    }
}

fn roundtrip_planning_dense_compute_reduced(
    path: &Path,
    index: usize,
    line: &str,
) -> Result<String, String> {
    let Some(avoided_tokens) = release_field(line, "second_planning_dense_compute_avoided_tokens")
    else {
        return Ok(String::new());
    };
    let avoided_tokens = roundtrip_usize_field(
        path,
        index,
        "second_planning_dense_compute_avoided_tokens",
        avoided_tokens,
    )?;
    let derived = avoided_tokens > 0;
    if let Some(raw_value) = release_field(line, "second_planning_dense_compute_reduced") {
        if raw_value != derived.to_string() {
            return Err(format!(
                "{}:{} second_planning_dense_compute_reduced conflicts with planning dense compute counter",
                path.display(),
                index + 1
            ));
        }
        Ok(
            " second_planning_dense_compute_reduced_source=roundtrip_proof_input_derived"
                .to_owned(),
        )
    } else {
        Ok(format!(
            " second_planning_dense_compute_reduced={derived} second_planning_dense_compute_reduced_source=roundtrip_proof_input_derived"
        ))
    }
}

fn roundtrip_negative_all_writes_denied(
    path: &Path,
    index: usize,
    line: &str,
) -> Result<String, String> {
    let Some(unauthorized) = release_field(line, "negative_unauthorized_write_allowed") else {
        return Ok(String::new());
    };
    let Some(memory) = release_field(line, "negative_memory_write_allowed") else {
        return Ok(String::new());
    };
    let Some(genome) = release_field(line, "negative_genome_write_allowed") else {
        return Ok(String::new());
    };
    let Some(self_evolution) = release_field(line, "negative_self_evolution_write_allowed") else {
        return Ok(String::new());
    };
    let derived = unauthorized == "false"
        && memory == "false"
        && genome == "false"
        && self_evolution == "false";
    if let Some(raw_value) = release_field(line, "negative_all_writes_denied") {
        if raw_value != derived.to_string() {
            return Err(format!(
                "{}:{} negative_all_writes_denied conflicts with write gate fields",
                path.display(),
                index + 1
            ));
        }
        Ok(" negative_all_writes_denied_source=roundtrip_proof_input_derived".to_owned())
    } else {
        Ok(format!(
            " negative_all_writes_denied={derived} negative_all_writes_denied_source=roundtrip_proof_input_derived"
        ))
    }
}

fn roundtrip_negative_polluted_evidence_contained(
    path: &Path,
    index: usize,
    line: &str,
) -> Result<String, String> {
    let blocked = release_field(line, "negative_polluted_evidence_blocked");
    let quarantined = release_field(line, "negative_polluted_evidence_quarantined");
    if blocked.is_none() && quarantined.is_none() {
        return Ok(String::new());
    }
    let derived = blocked == Some("true") || quarantined == Some("true");
    if let Some(raw_value) = release_field(line, "negative_polluted_evidence_contained") {
        if raw_value != derived.to_string() {
            return Err(format!(
                "{}:{} negative_polluted_evidence_contained conflicts with polluted evidence gates",
                path.display(),
                index + 1
            ));
        }
        Ok(" negative_polluted_evidence_contained_source=roundtrip_proof_input_derived".to_owned())
    } else {
        Ok(format!(
            " negative_polluted_evidence_contained={derived} negative_polluted_evidence_contained_source=roundtrip_proof_input_derived"
        ))
    }
}

fn roundtrip_negative_tenant_scope_boundary_ok(
    path: &Path,
    index: usize,
    line: &str,
) -> Result<String, String> {
    let write_denied = release_field(line, "negative_tenant_scope_write_denied");
    let mode = release_field(line, "negative_tenant_scope_mode");
    if write_denied.is_none() && mode.is_none() {
        return Ok(String::new());
    }
    let derived = roundtrip_tenant_scope_boundary_bound(line);
    if let Some(raw_value) = release_field(line, "negative_tenant_scope_boundary_ok") {
        if raw_value != derived.to_string() {
            return Err(format!(
                "{}:{} negative_tenant_scope_boundary_ok conflicts with tenant scope fields",
                path.display(),
                index + 1
            ));
        }
        Ok(" negative_tenant_scope_boundary_ok_source=roundtrip_proof_input_derived".to_owned())
    } else {
        Ok(format!(
            " negative_tenant_scope_boundary_ok={derived} negative_tenant_scope_boundary_ok_source=roundtrip_proof_input_derived"
        ))
    }
}

fn roundtrip_tenant_scope_boundary_bound(line: &str) -> bool {
    let actor = release_field(line, "negative_tenant_scope_actor");
    let target = release_field(line, "negative_tenant_scope_target");
    release_field(line, "negative_tenant_scope_write_denied") == Some("true")
        && release_field(line, "negative_tenant_scope_mode") == Some("local_single_user_preview")
        && actor.is_some_and(|value| value.starts_with("fnv64:"))
        && target.is_some_and(|value| value.starts_with("fnv64:"))
        && actor != target
        && release_field(line, "negative_tenant_scope_denial_lane") == Some("self_evolving_memory")
        && release_field(line, "negative_tenant_scope_denial_reason")
            == Some("cross_tenant_scope_rejected")
}

fn roundtrip_compute_anchors_preserved(
    path: &Path,
    index: usize,
    line: &str,
) -> Result<String, String> {
    let Some(anchor_count) = release_field(line, "second_compute_budget_anchor_count") else {
        return Ok(String::new());
    };
    let Some(preserved_count) =
        release_field(line, "second_compute_budget_anchors_preserved_count")
    else {
        return Ok(String::new());
    };
    let anchor_count = roundtrip_usize_field(
        path,
        index,
        "second_compute_budget_anchor_count",
        anchor_count,
    )?;
    let preserved_count = roundtrip_usize_field(
        path,
        index,
        "second_compute_budget_anchors_preserved_count",
        preserved_count,
    )?;
    let derived = anchor_count > 0 && preserved_count == anchor_count;
    if let Some(raw_value) = release_field(line, "second_compute_budget_anchors_preserved") {
        if raw_value != derived.to_string() {
            return Err(format!(
                "{}:{} second_compute_budget_anchors_preserved conflicts with anchor counts",
                path.display(),
                index + 1
            ));
        }
        Ok(
            " second_compute_budget_anchors_preserved_source=roundtrip_proof_input_derived"
                .to_owned(),
        )
    } else {
        Ok(format!(
            " second_compute_budget_anchors_preserved={derived} second_compute_budget_anchors_preserved_source=roundtrip_proof_input_derived"
        ))
    }
}

fn roundtrip_second_task_benefit_ready(
    path: &Path,
    index: usize,
    line: &str,
) -> Result<String, String> {
    let Some(saved_tokens) = release_field(line, "second_compute_budget_saved_tokens") else {
        return Ok(String::new());
    };
    let Some(avoided_tokens) = release_field(line, "second_compute_budget_avoided_tokens") else {
        return Ok(String::new());
    };
    let Some(kv_lookups_skipped) = release_field(line, "second_compute_budget_kv_lookups_skipped")
    else {
        return Ok(String::new());
    };
    let Some(anchor_count) = release_field(line, "second_compute_budget_anchor_count") else {
        return Ok(String::new());
    };
    let Some(preserved_count) =
        release_field(line, "second_compute_budget_anchors_preserved_count")
    else {
        return Ok(String::new());
    };
    let saved_tokens = roundtrip_usize_field(
        path,
        index,
        "second_compute_budget_saved_tokens",
        saved_tokens,
    )?;
    let avoided_tokens = roundtrip_usize_field(
        path,
        index,
        "second_compute_budget_avoided_tokens",
        avoided_tokens,
    )?;
    let kv_lookups_skipped = roundtrip_usize_field(
        path,
        index,
        "second_compute_budget_kv_lookups_skipped",
        kv_lookups_skipped,
    )?;
    let anchor_count = roundtrip_usize_field(
        path,
        index,
        "second_compute_budget_anchor_count",
        anchor_count,
    )?;
    let preserved_count = roundtrip_usize_field(
        path,
        index,
        "second_compute_budget_anchors_preserved_count",
        preserved_count,
    )?;
    let derived = line.starts_with("persistent_roundtrip: passed=true")
        && (saved_tokens > 0 || avoided_tokens > 0 || kv_lookups_skipped > 0)
        && anchor_count > 0
        && preserved_count == anchor_count
        && release_field(line, "second_approved_experience_reuse_digest")
            .is_some_and(|value| value.starts_with("redaction-digest:"))
        && release_field(line, "failures") == Some("0");
    if let Some(raw_value) = release_field(line, "issue30_second_task_benefit_ready") {
        if raw_value != derived.to_string() {
            return Err(format!(
                "{}:{} issue30_second_task_benefit_ready conflicts with second-task proof fields",
                path.display(),
                index + 1
            ));
        }
        Ok(" issue30_second_task_benefit_ready_source=roundtrip_proof_input_derived".to_owned())
    } else {
        Ok(format!(
            " issue30_second_task_benefit_ready={derived} issue30_second_task_benefit_ready_source=roundtrip_proof_input_derived"
        ))
    }
}

fn roundtrip_disk_kv_roundtrip_ready(
    path: &Path,
    index: usize,
    line: &str,
) -> Result<String, String> {
    let Some(first_reopen) = release_field(line, "first_disk_kv_reopen_verified") else {
        return Ok(String::new());
    };
    let Some(disk_rehydrated) = release_field(line, "second_runtime_kv_disk_rehydrated") else {
        return Ok(String::new());
    };
    let Some(kvswap_boundary) = release_field(line, "second_kvswap_boundary_verified") else {
        return Ok(String::new());
    };
    let imported_blocks = release_field(line, "second_imported_runtime_kv_blocks")
        .map(|value| roundtrip_usize_field(path, index, "second_imported_runtime_kv_blocks", value))
        .transpose()?
        .unwrap_or(0);
    let derived = line.starts_with("persistent_roundtrip: passed=true")
        && first_reopen == "true"
        && disk_rehydrated == "true"
        && kvswap_boundary == "true"
        && imported_blocks > 0
        && release_field(line, "second_imported_runtime_kv_from_namespace") == Some("true");
    if let Some(raw_value) = release_field(line, "issue30_disk_kv_roundtrip_ready") {
        if raw_value != derived.to_string() {
            return Err(format!(
                "{}:{} issue30_disk_kv_roundtrip_ready conflicts with disk KV roundtrip fields",
                path.display(),
                index + 1
            ));
        }
        Ok(" issue30_disk_kv_roundtrip_ready_source=roundtrip_proof_input_derived".to_owned())
    } else {
        Ok(format!(
            " issue30_disk_kv_roundtrip_ready={derived} issue30_disk_kv_roundtrip_ready_source=roundtrip_proof_input_derived"
        ))
    }
}

fn roundtrip_usize_field(
    path: &Path,
    index: usize,
    field: &str,
    value: &str,
) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map_err(|_| format!("{}:{} invalid {field}", path.display(), index + 1))
}

fn roundtrip_digest_only(line: &str) -> Option<bool> {
    Some(
        release_field(line, "second_approved_experience_reuse_digest")?
            .starts_with("redaction-digest:")
            && release_field(line, "negative_bad_candidate_digest")?
                .starts_with("redaction-digest:")
            && release_field(line, "negative_rollback_anchor_digest")?
                .starts_with("redaction-digest:")
            && release_field(line, "negative_tenant_scope_actor")?.starts_with("fnv64:")
            && release_field(line, "negative_tenant_scope_target")?.starts_with("fnv64:")
            && release_field(line, "negative_polluted_evidence_quarantined")? == "true"
            && release_field(line, "negative_provenance_license_redaction_passed")? == "true",
    )
}

fn roundtrip_negative_gates_ready(path: &Path, index: usize, line: &str) -> Result<String, String> {
    let derived = release_field(line, "negative_unauthorized_write_allowed") == Some("false")
        && release_field(line, "negative_memory_write_allowed") == Some("false")
        && release_field(line, "negative_genome_write_allowed") == Some("false")
        && release_field(line, "negative_self_evolution_write_allowed") == Some("false")
        && (release_field(line, "negative_polluted_evidence_blocked") == Some("true")
            || release_field(line, "negative_polluted_evidence_quarantined") == Some("true"))
        && release_field(line, "negative_bad_candidate_decision") == Some("hold_then_rollback")
        && release_field(line, "negative_rollback_anchor_present") == Some("true")
        && roundtrip_tenant_scope_boundary_bound(line)
        && release_field(line, "negative_provenance_license_redaction_passed") == Some("true")
        && roundtrip_digest_only(line) == Some(true);
    if let Some(raw_value) = release_field(line, "issue30_negative_gates_ready") {
        if raw_value != derived.to_string() {
            return Err(format!(
                "{}:{} issue30_negative_gates_ready conflicts with negative gate fields",
                path.display(),
                index + 1
            ));
        }
        Ok(" issue30_negative_gates_ready_source=roundtrip_proof_input_derived".to_owned())
    } else {
        Ok(format!(
            " issue30_negative_gates_ready={derived} issue30_negative_gates_ready_source=roundtrip_proof_input_derived"
        ))
    }
}

fn trace_report_statement(path: &Path) -> Result<String, String> {
    let raw = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    for (index, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if !line.starts_with("trace_schema_gate: ") {
            return Err(format!(
                "{}:{} expected trace_schema_gate summary line",
                path.display(),
                index + 1
            ));
        }
        let passed = required_issue_field(path, index, line, "passed")?;
        let reasoning_genome_events =
            required_issue_field(path, index, line, "reasoning_genome_events")?;
        let reasoning_genome_write_allowed =
            required_issue_field(path, index, line, "reasoning_genome_write_allowed")?;
        let reasoning_genome_splice_write_allowed =
            required_issue_field(path, index, line, "reasoning_genome_splice_write_allowed")?;
        let self_evolution_admission_events =
            required_issue_field(path, index, line, "self_evolution_admission_events")?;
        let self_evolution_admission_review_packets =
            required_issue_field(path, index, line, "self_evolution_admission_review_packets")?;
        let self_evolution_admission_evidence_ids =
            required_issue_field(path, index, line, "self_evolution_admission_evidence_ids")?;
        let self_evolution_admission_missing_review_packet_refs = required_issue_field(
            path,
            index,
            line,
            "self_evolution_admission_missing_review_packet_refs",
        )?;
        let memory_admission_events =
            required_trace_count_field(path, index, line, "memory_admission_events")?;
        let memory_admission_candidates =
            trace_count_field_or_zero(path, index, line, "memory_admission_candidates")?;
        let memory_admission_ledger_records =
            required_issue_field(path, index, line, "memory_admission_ledger_records")?;
        let memory_admission_ledger_authorized =
            required_trace_count_field(path, index, line, "memory_admission_ledger_authorized")?;
        let memory_admission_ledger_applied =
            required_trace_count_field(path, index, line, "memory_admission_ledger_applied")?;
        let memory_admission_ledger_preview_only =
            required_issue_field(path, index, line, "memory_admission_ledger_preview_only")?;
        let memory_admission_admitted =
            required_trace_count_field(path, index, line, "memory_admission_admitted")?;
        let memory_admission_hold =
            required_trace_count_field(path, index, line, "memory_admission_hold")?;
        let memory_admission_reject =
            required_trace_count_field(path, index, line, "memory_admission_reject")?;
        let memory_admission_ledger_held =
            required_trace_count_field(path, index, line, "memory_admission_ledger_held")?;
        let memory_admission_ledger_rejected =
            required_trace_count_field(path, index, line, "memory_admission_ledger_rejected")?;
        let memory_admission_ledger_duplicate =
            required_trace_count_field(path, index, line, "memory_admission_ledger_duplicate")?;
        let memory_admission_ledger_decayed =
            required_trace_count_field(path, index, line, "memory_admission_ledger_decayed")?;
        let memory_admission_ledger_merged =
            required_trace_count_field(path, index, line, "memory_admission_ledger_merged")?;
        let memory_admission_ledger_rollback =
            required_trace_count_field(path, index, line, "memory_admission_ledger_rollback")?;
        let memory_admission_source_semantic =
            trace_count_field_or_zero(path, index, line, "memory_admission_source_semantic")?;
        let memory_admission_source_gist =
            trace_count_field_or_zero(path, index, line, "memory_admission_source_gist")?;
        let memory_admission_source_runtime_kv =
            trace_count_field_or_zero(path, index, line, "memory_admission_source_runtime_kv")?;
        let memory_admission_source_cold =
            trace_count_field_or_zero(path, index, line, "memory_admission_source_cold")?;
        let memory_admission_source_gene_segment =
            trace_count_field_or_zero(path, index, line, "memory_admission_source_gene_segment")?;
        let memory_admission_gene_segment_metadata =
            trace_count_field_or_zero(path, index, line, "memory_admission_gene_segment_metadata")?;
        let memory_admission_read_only =
            required_trace_count_field(path, index, line, "memory_admission_read_only")?;
        let memory_admission_write_allowed =
            required_trace_count_field(path, index, line, "memory_admission_write_allowed")?;
        let memory_admission_applied =
            required_trace_count_field(path, index, line, "memory_admission_applied")?;
        let disk_kv_compact_reopen_verified =
            required_issue_field(path, index, line, "disk_kv_compact_reopen_verified")?;
        let disk_kv_compact_reopen_test =
            required_issue_field(path, index, line, "disk_kv_compact_reopen_test")?;
        let memory_admission_ledger_reopen_verified =
            required_issue_field(path, index, line, "memory_admission_ledger_reopen_verified")?;
        let memory_admission_ledger_reopen_test =
            required_issue_field(path, index, line, "memory_admission_ledger_reopen_test")?;
        let admission_review_complete = trace_admission_review_complete(path, index, line)?;
        let memory_admission_preview_apply_proof =
            trace_memory_admission_preview_apply_proof(path, index, line)?;
        let memory_authorized_fixture_apply_proof =
            trace_memory_authorized_fixture_apply_statement(path, index, line)?;
        let memory_runtime_preview_apply_proof =
            trace_memory_runtime_preview_apply_statement(path, index, line)?;
        let memory_read_only_authorized_append_denial_proof =
            trace_memory_read_only_authorized_append_denial_statement(path, index, line)?;
        let memory_invalid_shape_rejection_proof =
            trace_memory_invalid_shape_rejection_statement(path, index, line)?;
        let memory_review_scope_required_proof =
            trace_memory_review_scope_required_statement(path, index, line)?;
        let memory_ledger_apply_proof = trace_memory_ledger_apply_proof(path, index, line)?;
        let memory_ledger_lifecycle_retention_proof =
            trace_memory_ledger_lifecycle_retention_proof(path, index, line)?;
        let memory_admission_source_mix_proof =
            trace_memory_admission_source_mix_proof(path, index, line)?;
        let memory_gene_segment_metadata_proof =
            trace_memory_gene_segment_metadata_proof(path, index, line)?;
        let memory_residency_retention_compaction_proof =
            trace_memory_residency_retention_compaction_proof(path, index, line)?;
        let memory_autophagy_preview_proof =
            trace_memory_autophagy_preview_proof(path, index, line)?;
        let memory_ledger_trace_ready = trace_memory_ledger_ready(path, index, line)?;
        let trace_validation_ready = trace_validation_ready(path, index, line)?;
        let agent_team_layer_b_route_ready =
            trace_agent_team_layer_b_route_ready(path, index, line)?;
        let agent_team_contract_ready =
            trace_issue185_agent_team_contract_ready(path, index, line)?;
        let coding_service_eval_self_validation_ready =
            trace_issue185_coding_service_eval_self_validation_ready(path, index, line)?;
        let issue185_agent_tooling_mvp_ready =
            trace_issue185_agent_tooling_mvp_ready(path, index, line)?;
        let chaperone_fold_guard_ready = trace_chaperone_fold_guard_ready(path, index, line)?;
        let issue37_runtime_recall_scope_ready =
            trace_issue37_runtime_recall_scope_ready(path, index, line)?;
        let control_expression_gate_ready = trace_control_expression_gate_ready(path, index, line)?;
        return Ok(format!(
            "trace_schema_gate: passed={passed} reasoning_genome_events={reasoning_genome_events} reasoning_genome_write_allowed={reasoning_genome_write_allowed} reasoning_genome_splice_write_allowed={reasoning_genome_splice_write_allowed} self_evolution_admission_events={self_evolution_admission_events} self_evolution_admission_review_packets={self_evolution_admission_review_packets} self_evolution_admission_evidence_ids={self_evolution_admission_evidence_ids} self_evolution_admission_missing_review_packet_refs={self_evolution_admission_missing_review_packet_refs} memory_admission_events={memory_admission_events} memory_admission_candidates={memory_admission_candidates} memory_admission_ledger_records={memory_admission_ledger_records} memory_admission_ledger_authorized={memory_admission_ledger_authorized} memory_admission_ledger_applied={memory_admission_ledger_applied} memory_admission_ledger_preview_only={memory_admission_ledger_preview_only} memory_admission_admitted={memory_admission_admitted} memory_admission_hold={memory_admission_hold} memory_admission_reject={memory_admission_reject} memory_admission_ledger_held={memory_admission_ledger_held} memory_admission_ledger_rejected={memory_admission_ledger_rejected} memory_admission_ledger_duplicate={memory_admission_ledger_duplicate} memory_admission_ledger_decayed={memory_admission_ledger_decayed} memory_admission_ledger_merged={memory_admission_ledger_merged} memory_admission_ledger_rollback={memory_admission_ledger_rollback} memory_admission_source_semantic={memory_admission_source_semantic} memory_admission_source_gist={memory_admission_source_gist} memory_admission_source_runtime_kv={memory_admission_source_runtime_kv} memory_admission_source_cold={memory_admission_source_cold} memory_admission_source_gene_segment={memory_admission_source_gene_segment} memory_admission_gene_segment_metadata={memory_admission_gene_segment_metadata} memory_admission_read_only={memory_admission_read_only} memory_admission_write_allowed={memory_admission_write_allowed} memory_admission_applied={memory_admission_applied} disk_kv_compact_reopen_verified={disk_kv_compact_reopen_verified} disk_kv_compact_reopen_test={disk_kv_compact_reopen_test} memory_admission_ledger_reopen_verified={memory_admission_ledger_reopen_verified} memory_admission_ledger_reopen_test={memory_admission_ledger_reopen_test}{admission_review_complete}{memory_admission_preview_apply_proof}{memory_authorized_fixture_apply_proof}{memory_runtime_preview_apply_proof}{memory_read_only_authorized_append_denial_proof}{memory_invalid_shape_rejection_proof}{memory_review_scope_required_proof}{memory_ledger_apply_proof}{memory_ledger_lifecycle_retention_proof}{memory_admission_source_mix_proof}{memory_gene_segment_metadata_proof}{memory_residency_retention_compaction_proof}{memory_autophagy_preview_proof}{memory_ledger_trace_ready}{trace_validation_ready}{agent_team_layer_b_route_ready}{agent_team_contract_ready}{coding_service_eval_self_validation_ready}{issue185_agent_tooling_mvp_ready}{chaperone_fold_guard_ready}{issue37_runtime_recall_scope_ready}{control_expression_gate_ready} trace_report_source=trace_report_input"
        ));
    }
    Err(format!("{} has no trace report rows", path.display()))
}

fn required_trace_count_field(
    path: &Path,
    index: usize,
    line: &str,
    field: &str,
) -> Result<String, String> {
    let value = required_issue_field(path, index, line, field)?;
    roundtrip_usize_field(path, index, field, &value)?;
    Ok(value)
}

fn trace_count_field_or_zero(
    path: &Path,
    index: usize,
    line: &str,
    field: &str,
) -> Result<String, String> {
    let Some(value) = release_field(line, field) else {
        return Ok("0".to_owned());
    };
    roundtrip_usize_field(path, index, field, value)?;
    Ok(value.to_owned())
}

fn trace_control_expression_gate_ready(
    path: &Path,
    index: usize,
    line: &str,
) -> Result<String, String> {
    let Some(knobs) = release_field(line, "control_expression_active_control_knobs") else {
        if release_field(line, "issue243_control_expression_gate_ready").is_some() {
            return Err(format!(
                "{}:{} issue243_control_expression_gate_ready requires control_expression_active_control_knobs",
                path.display(),
                index + 1
            ));
        }
        return Ok(String::new());
    };
    let has_knobs = [
        "routing",
        "context_anchor",
        "suppression",
        "checkpoint",
        "memory_maintenance",
    ]
    .iter()
    .all(|knob| knobs.split('|').any(|candidate| candidate == *knob));
    let derived = has_knobs
        && release_field(line, "control_expression_evidence_digest")
            .is_some_and(|value| value.starts_with("redaction-digest:"))
        && release_field(line, "control_expression_policy_version")
            == Some("control_expression_gate_v1")
        && release_field(line, "control_expression_decision_reason")
            == Some("no_weight_runtime_control_preview")
        && trace_count_field_or_zero(path, index, line, "control_expression_profile_selected")?
            != "0"
        && trace_count_field_or_zero(
            path,
            index,
            line,
            "control_expression_context_anchor_promoted",
        )? != "0"
        && trace_count_field_or_zero(
            path,
            index,
            line,
            "control_expression_suppression_gate_triggered",
        )? != "0"
        && trace_count_field_or_zero(
            path,
            index,
            line,
            "control_expression_checkpoint_repair_requested",
        )? != "0"
        && trace_count_field_or_zero(path, index, line, "control_expression_checkpoint_rejected")?
            != "0"
        && trace_count_field_or_zero(
            path,
            index,
            line,
            "control_expression_memory_refresh_candidate",
        )? != "0"
        && trace_count_field_or_zero(
            path,
            index,
            line,
            "control_expression_memory_tombstone_candidate",
        )? != "0"
        && trace_count_field_or_zero(path, index, line, "control_expression_preview_admission")?
            != "0"
        && trace_count_field_or_zero(path, index, line, "control_expression_write_allowed")? == "0"
        && trace_count_field_or_zero(path, index, line, "control_expression_applied")? == "0"
        && trace_count_field_or_zero(
            path,
            index,
            line,
            "control_expression_operator_approval_required",
        )? != "0";
    if let Some(raw_value) = release_field(line, "issue243_control_expression_gate_ready") {
        if raw_value != derived.to_string() {
            return Err(format!(
                "{}:{} issue243_control_expression_gate_ready conflicts with trace control expression fields",
                path.display(),
                index + 1
            ));
        }
        Ok(" issue243_control_expression_gate_ready_source=trace_report_input_derived".to_owned())
    } else {
        Ok(format!(
            " issue243_control_expression_gate_ready={derived} issue243_control_expression_gate_ready_source=trace_report_input_derived"
        ))
    }
}

fn trace_admission_review_complete(
    path: &Path,
    index: usize,
    line: &str,
) -> Result<String, String> {
    let events = roundtrip_usize_field(
        path,
        index,
        "self_evolution_admission_events",
        release_field(line, "self_evolution_admission_events").unwrap_or(""),
    )?;
    let review_packets = roundtrip_usize_field(
        path,
        index,
        "self_evolution_admission_review_packets",
        release_field(line, "self_evolution_admission_review_packets").unwrap_or(""),
    )?;
    let evidence_ids = roundtrip_usize_field(
        path,
        index,
        "self_evolution_admission_evidence_ids",
        release_field(line, "self_evolution_admission_evidence_ids").unwrap_or(""),
    )?;
    let missing_refs = roundtrip_usize_field(
        path,
        index,
        "self_evolution_admission_missing_review_packet_refs",
        release_field(line, "self_evolution_admission_missing_review_packet_refs").unwrap_or(""),
    )?;
    let derived = events > 0 && review_packets > 0 && evidence_ids > 0 && missing_refs == 0;
    if let Some(raw_value) = release_field(line, "self_evolution_admission_review_complete") {
        if raw_value != derived.to_string() {
            return Err(format!(
                "{}:{} self_evolution_admission_review_complete conflicts with admission counters",
                path.display(),
                index + 1
            ));
        }
        Ok(
            " self_evolution_admission_review_complete_source=trace_report_input_derived"
                .to_owned(),
        )
    } else {
        Ok(format!(
            " self_evolution_admission_review_complete={derived} self_evolution_admission_review_complete_source=trace_report_input_derived"
        ))
    }
}

fn trace_memory_ledger_ready(path: &Path, index: usize, line: &str) -> Result<String, String> {
    let records = roundtrip_usize_field(
        path,
        index,
        "memory_admission_ledger_records",
        release_field(line, "memory_admission_ledger_records").unwrap_or(""),
    )?;
    let authorized = roundtrip_usize_field(
        path,
        index,
        "memory_admission_ledger_authorized",
        release_field(line, "memory_admission_ledger_authorized").unwrap_or(""),
    )?;
    let applied = roundtrip_usize_field(
        path,
        index,
        "memory_admission_ledger_applied",
        release_field(line, "memory_admission_ledger_applied").unwrap_or(""),
    )?;
    let preview_only = roundtrip_usize_field(
        path,
        index,
        "memory_admission_ledger_preview_only",
        release_field(line, "memory_admission_ledger_preview_only").unwrap_or(""),
    )?;
    let disk_kv_compact_reopen_verified =
        required_issue_field(path, index, line, "disk_kv_compact_reopen_verified")?;
    let disk_kv_compact_reopen_test =
        required_issue_field(path, index, line, "disk_kv_compact_reopen_test")?;
    let memory_admission_ledger_reopen_verified =
        required_issue_field(path, index, line, "memory_admission_ledger_reopen_verified")?;
    let memory_admission_ledger_reopen_test =
        required_issue_field(path, index, line, "memory_admission_ledger_reopen_test")?;
    let derived = release_field(line, "passed") == Some("true")
        && records > 0
        && authorized == 0
        && applied == 0
        && preview_only > 0
        && disk_kv_compact_reopen_verified == "true"
        && disk_kv_compact_reopen_test == "disk_kv::tests::compact_keeps_latest_values"
        && memory_admission_ledger_reopen_verified == "true"
        && memory_admission_ledger_reopen_test
            == "memory_admission::tests::writer_gate_append_is_idempotent_after_store_reopen";
    if let Some(raw_value) = release_field(line, "issue30_memory_ledger_trace_ready") {
        if raw_value != derived.to_string() {
            return Err(format!(
                "{}:{} issue30_memory_ledger_trace_ready conflicts with memory ledger reopen proof fields",
                path.display(),
                index + 1
            ));
        }
        Ok(" issue30_memory_ledger_trace_ready_source=trace_report_input_derived".to_owned())
    } else {
        Ok(format!(
            " issue30_memory_ledger_trace_ready={derived} issue30_memory_ledger_trace_ready_source=trace_report_input_derived"
        ))
    }
}

fn trace_memory_admission_preview_apply_proof(
    path: &Path,
    index: usize,
    line: &str,
) -> Result<String, String> {
    let events = roundtrip_usize_field(
        path,
        index,
        "memory_admission_events",
        release_field(line, "memory_admission_events").unwrap_or(""),
    )?;
    let read_only = roundtrip_usize_field(
        path,
        index,
        "memory_admission_read_only",
        release_field(line, "memory_admission_read_only").unwrap_or(""),
    )?;
    let write_allowed = roundtrip_usize_field(
        path,
        index,
        "memory_admission_write_allowed",
        release_field(line, "memory_admission_write_allowed").unwrap_or(""),
    )?;
    let applied = roundtrip_usize_field(
        path,
        index,
        "memory_admission_applied",
        release_field(line, "memory_admission_applied").unwrap_or(""),
    )?;
    let derived = events > 0 && read_only == events && write_allowed == 0 && applied == 0;
    if let Some(raw_value) = release_field(line, "issue2_memory_admission_preview_apply_proof") {
        if raw_value != derived.to_string() {
            return Err(format!(
                "{}:{} issue2_memory_admission_preview_apply_proof conflicts with memory admission preview apply fields",
                path.display(),
                index + 1
            ));
        }
        Ok(
            " issue2_memory_admission_preview_apply_proof_source=trace_report_input_derived"
                .to_owned(),
        )
    } else {
        Ok(format!(
            " issue2_memory_admission_preview_apply_proof={derived} issue2_memory_admission_preview_apply_proof_source=trace_report_input_derived"
        ))
    }
}

fn trace_memory_authorized_fixture_apply_statement(
    path: &Path,
    index: usize,
    line: &str,
) -> Result<String, String> {
    let fixture_fields_present = [
        "memory_admission_authorized_fixture_apply_verified",
        "memory_admission_authorized_fixture_apply_test",
        "memory_admission_authorized_fixture_authorized",
        "memory_admission_authorized_fixture_applied",
        "memory_admission_authorized_fixture_admitted",
        "memory_admission_authorized_fixture_rehydrated",
        "memory_admission_authorized_fixture_reopened_records",
        "memory_admission_authorized_fixture_ledger_bytes_nonzero",
        "issue2_memory_authorized_fixture_apply_proof",
    ]
    .iter()
    .any(|field| release_field(line, field).is_some());
    if !fixture_fields_present {
        return Ok(String::new());
    }

    let runtime_ledger_authorized = roundtrip_usize_field(
        path,
        index,
        "memory_admission_ledger_authorized",
        release_field(line, "memory_admission_ledger_authorized").unwrap_or(""),
    )?;
    let runtime_ledger_applied = roundtrip_usize_field(
        path,
        index,
        "memory_admission_ledger_applied",
        release_field(line, "memory_admission_ledger_applied").unwrap_or(""),
    )?;
    let runtime_write_allowed = roundtrip_usize_field(
        path,
        index,
        "memory_admission_write_allowed",
        release_field(line, "memory_admission_write_allowed").unwrap_or(""),
    )?;
    let runtime_applied = roundtrip_usize_field(
        path,
        index,
        "memory_admission_applied",
        release_field(line, "memory_admission_applied").unwrap_or(""),
    )?;
    let fixture_verified = required_issue_field(
        path,
        index,
        line,
        "memory_admission_authorized_fixture_apply_verified",
    )?;
    let fixture_test = required_issue_field(
        path,
        index,
        line,
        "memory_admission_authorized_fixture_apply_test",
    )?;
    let fixture_authorized = roundtrip_usize_field(
        path,
        index,
        "memory_admission_authorized_fixture_authorized",
        release_field(line, "memory_admission_authorized_fixture_authorized").unwrap_or(""),
    )?;
    let fixture_applied = roundtrip_usize_field(
        path,
        index,
        "memory_admission_authorized_fixture_applied",
        release_field(line, "memory_admission_authorized_fixture_applied").unwrap_or(""),
    )?;
    let fixture_admitted = roundtrip_usize_field(
        path,
        index,
        "memory_admission_authorized_fixture_admitted",
        release_field(line, "memory_admission_authorized_fixture_admitted").unwrap_or(""),
    )?;
    let fixture_rehydrated = roundtrip_usize_field(
        path,
        index,
        "memory_admission_authorized_fixture_rehydrated",
        release_field(line, "memory_admission_authorized_fixture_rehydrated").unwrap_or(""),
    )?;
    let fixture_reopened_records = roundtrip_usize_field(
        path,
        index,
        "memory_admission_authorized_fixture_reopened_records",
        release_field(line, "memory_admission_authorized_fixture_reopened_records").unwrap_or(""),
    )?;
    let fixture_ledger_bytes_nonzero = required_issue_field(
        path,
        index,
        line,
        "memory_admission_authorized_fixture_ledger_bytes_nonzero",
    )?;
    let derived = runtime_ledger_authorized == 0
        && runtime_ledger_applied == 0
        && runtime_write_allowed == 0
        && runtime_applied == 0
        && fixture_verified == "true"
        && fixture_test
            == "memory_admission::tests::writer_gate_rehydrates_applied_authorized_records_from_existing_ledger"
        && fixture_authorized > 0
        && fixture_applied == fixture_authorized
        && fixture_admitted == fixture_authorized
        && fixture_rehydrated == fixture_authorized
        && fixture_reopened_records == fixture_authorized
        && fixture_ledger_bytes_nonzero == "true";
    let fixture_fields = format!(
        " memory_admission_authorized_fixture_apply_verified={fixture_verified} memory_admission_authorized_fixture_apply_test={fixture_test} memory_admission_authorized_fixture_authorized={fixture_authorized} memory_admission_authorized_fixture_applied={fixture_applied} memory_admission_authorized_fixture_admitted={fixture_admitted} memory_admission_authorized_fixture_rehydrated={fixture_rehydrated} memory_admission_authorized_fixture_reopened_records={fixture_reopened_records} memory_admission_authorized_fixture_ledger_bytes_nonzero={fixture_ledger_bytes_nonzero}"
    );
    if let Some(raw_value) = release_field(line, "issue2_memory_authorized_fixture_apply_proof") {
        if raw_value != derived.to_string() {
            return Err(format!(
                "{}:{} issue2_memory_authorized_fixture_apply_proof conflicts with authorized fixture apply fields",
                path.display(),
                index + 1
            ));
        }
        Ok(format!(
            "{fixture_fields} issue2_memory_authorized_fixture_apply_proof_source=trace_report_input_derived"
        ))
    } else {
        Ok(format!(
            "{fixture_fields} issue2_memory_authorized_fixture_apply_proof={derived} issue2_memory_authorized_fixture_apply_proof_source=trace_report_input_derived"
        ))
    }
}

fn trace_memory_runtime_preview_apply_statement(
    path: &Path,
    index: usize,
    line: &str,
) -> Result<String, String> {
    let fixture_fields_present = [
        "memory_admission_runtime_preview_apply_verified",
        "memory_admission_runtime_preview_apply_test",
        "memory_admission_runtime_preview_authorized",
        "memory_admission_runtime_preview_applied",
        "memory_admission_runtime_preview_admitted",
        "memory_admission_runtime_preview_rehydrated",
        "issue2_memory_runtime_preview_apply_proof",
    ]
    .iter()
    .any(|field| release_field(line, field).is_some());
    if !fixture_fields_present {
        return Ok(String::new());
    }

    let runtime_ledger_authorized = roundtrip_usize_field(
        path,
        index,
        "memory_admission_ledger_authorized",
        release_field(line, "memory_admission_ledger_authorized").unwrap_or(""),
    )?;
    let runtime_ledger_applied = roundtrip_usize_field(
        path,
        index,
        "memory_admission_ledger_applied",
        release_field(line, "memory_admission_ledger_applied").unwrap_or(""),
    )?;
    let runtime_write_allowed = roundtrip_usize_field(
        path,
        index,
        "memory_admission_write_allowed",
        release_field(line, "memory_admission_write_allowed").unwrap_or(""),
    )?;
    let runtime_applied = roundtrip_usize_field(
        path,
        index,
        "memory_admission_applied",
        release_field(line, "memory_admission_applied").unwrap_or(""),
    )?;
    let verified = required_issue_field(
        path,
        index,
        line,
        "memory_admission_runtime_preview_apply_verified",
    )?;
    let test = required_issue_field(
        path,
        index,
        line,
        "memory_admission_runtime_preview_apply_test",
    )?;
    let preview_authorized = roundtrip_usize_field(
        path,
        index,
        "memory_admission_runtime_preview_authorized",
        release_field(line, "memory_admission_runtime_preview_authorized").unwrap_or(""),
    )?;
    let preview_applied = roundtrip_usize_field(
        path,
        index,
        "memory_admission_runtime_preview_applied",
        release_field(line, "memory_admission_runtime_preview_applied").unwrap_or(""),
    )?;
    let preview_admitted = roundtrip_usize_field(
        path,
        index,
        "memory_admission_runtime_preview_admitted",
        release_field(line, "memory_admission_runtime_preview_admitted").unwrap_or(""),
    )?;
    let preview_rehydrated = roundtrip_usize_field(
        path,
        index,
        "memory_admission_runtime_preview_rehydrated",
        release_field(line, "memory_admission_runtime_preview_rehydrated").unwrap_or(""),
    )?;
    let derived = runtime_ledger_authorized == 0
        && runtime_ledger_applied == 0
        && runtime_write_allowed == 0
        && runtime_applied == 0
        && verified == "true"
        && test
            == "tests::benchmark_state::runtime_memory_admission_preview_applies_after_approved_writer_policy"
        && preview_authorized > 0
        && preview_applied == preview_authorized
        && preview_admitted == preview_authorized
        && preview_rehydrated == preview_authorized;
    let fields = format!(
        " memory_admission_runtime_preview_apply_verified={verified} memory_admission_runtime_preview_apply_test={test} memory_admission_runtime_preview_authorized={preview_authorized} memory_admission_runtime_preview_applied={preview_applied} memory_admission_runtime_preview_admitted={preview_admitted} memory_admission_runtime_preview_rehydrated={preview_rehydrated}"
    );
    if let Some(raw_value) = release_field(line, "issue2_memory_runtime_preview_apply_proof") {
        if raw_value != derived.to_string() {
            return Err(format!(
                "{}:{} issue2_memory_runtime_preview_apply_proof conflicts with runtime preview apply fields",
                path.display(),
                index + 1
            ));
        }
        Ok(format!(
            "{fields} issue2_memory_runtime_preview_apply_proof_source=trace_report_input_derived"
        ))
    } else {
        Ok(format!(
            "{fields} issue2_memory_runtime_preview_apply_proof={derived} issue2_memory_runtime_preview_apply_proof_source=trace_report_input_derived"
        ))
    }
}

fn trace_memory_read_only_authorized_append_denial_statement(
    path: &Path,
    index: usize,
    line: &str,
) -> Result<String, String> {
    let fields_present = [
        "memory_admission_read_only_authorized_append_denied",
        "memory_admission_read_only_authorized_append_test",
        "memory_admission_read_only_authorized_append_preserved_existing_bytes",
        "issue2_memory_read_only_authorized_append_denial_proof",
    ]
    .iter()
    .any(|field| release_field(line, field).is_some());
    if !fields_present {
        return Ok(String::new());
    }

    let runtime_ledger_authorized = roundtrip_usize_field(
        path,
        index,
        "memory_admission_ledger_authorized",
        release_field(line, "memory_admission_ledger_authorized").unwrap_or(""),
    )?;
    let runtime_ledger_applied = roundtrip_usize_field(
        path,
        index,
        "memory_admission_ledger_applied",
        release_field(line, "memory_admission_ledger_applied").unwrap_or(""),
    )?;
    let runtime_write_allowed = roundtrip_usize_field(
        path,
        index,
        "memory_admission_write_allowed",
        release_field(line, "memory_admission_write_allowed").unwrap_or(""),
    )?;
    let runtime_applied = roundtrip_usize_field(
        path,
        index,
        "memory_admission_applied",
        release_field(line, "memory_admission_applied").unwrap_or(""),
    )?;
    let denied = required_issue_field(
        path,
        index,
        line,
        "memory_admission_read_only_authorized_append_denied",
    )?;
    let test = required_issue_field(
        path,
        index,
        line,
        "memory_admission_read_only_authorized_append_test",
    )?;
    let preserved_existing_bytes = required_issue_field(
        path,
        index,
        line,
        "memory_admission_read_only_authorized_append_preserved_existing_bytes",
    )?;
    let derived = runtime_ledger_authorized == 0
        && runtime_ledger_applied == 0
        && runtime_write_allowed == 0
        && runtime_applied == 0
        && denied == "true"
        && test
            == "memory_admission::tests::writer_gate_refuses_authorized_append_on_read_only_store"
        && preserved_existing_bytes == "true";
    let fields = format!(
        " memory_admission_read_only_authorized_append_denied={denied} memory_admission_read_only_authorized_append_test={test} memory_admission_read_only_authorized_append_preserved_existing_bytes={preserved_existing_bytes}"
    );
    if let Some(raw_value) = release_field(
        line,
        "issue2_memory_read_only_authorized_append_denial_proof",
    ) {
        if raw_value != derived.to_string() {
            return Err(format!(
                "{}:{} issue2_memory_read_only_authorized_append_denial_proof conflicts with read-only authorized append fields",
                path.display(),
                index + 1
            ));
        }
        Ok(format!(
            "{fields} issue2_memory_read_only_authorized_append_denial_proof_source=trace_report_input_derived"
        ))
    } else {
        Ok(format!(
            "{fields} issue2_memory_read_only_authorized_append_denial_proof={derived} issue2_memory_read_only_authorized_append_denial_proof_source=trace_report_input_derived"
        ))
    }
}

fn trace_memory_review_scope_required_statement(
    path: &Path,
    index: usize,
    line: &str,
) -> Result<String, String> {
    let fields_present = [
        "memory_admission_review_scope_required_verified",
        "memory_admission_review_scope_required_test",
        "memory_admission_review_scope_required_tenant_rejection",
        "memory_admission_review_scope_required_session_rejection",
        "memory_admission_review_scope_required_authorized",
        "memory_admission_review_scope_required_appended",
        "issue2_memory_review_scope_required_proof",
    ]
    .iter()
    .any(|field| release_field(line, field).is_some());
    if !fields_present {
        return Ok(String::new());
    }

    let verified = required_issue_field(
        path,
        index,
        line,
        "memory_admission_review_scope_required_verified",
    )?;
    let test = required_issue_field(
        path,
        index,
        line,
        "memory_admission_review_scope_required_test",
    )?;
    let tenant_rejection = required_issue_field(
        path,
        index,
        line,
        "memory_admission_review_scope_required_tenant_rejection",
    )?;
    let session_rejection = required_issue_field(
        path,
        index,
        line,
        "memory_admission_review_scope_required_session_rejection",
    )?;
    let authorized = roundtrip_usize_field(
        path,
        index,
        "memory_admission_review_scope_required_authorized",
        release_field(line, "memory_admission_review_scope_required_authorized").unwrap_or(""),
    )?;
    let appended = roundtrip_usize_field(
        path,
        index,
        "memory_admission_review_scope_required_appended",
        release_field(line, "memory_admission_review_scope_required_appended").unwrap_or(""),
    )?;
    let derived = verified == "true"
        && test
            == "memory_admission::tests::gene_segment_kv_writer_gate_rejects_missing_review_scope_digests"
        && tenant_rejection == "review_packet_tenant_scope_digest_missing"
        && session_rejection == "review_packet_session_scope_digest_missing"
        && authorized == 0
        && appended == 0;
    let fields = format!(
        " memory_admission_review_scope_required_verified={verified} memory_admission_review_scope_required_test={test} memory_admission_review_scope_required_tenant_rejection={tenant_rejection} memory_admission_review_scope_required_session_rejection={session_rejection} memory_admission_review_scope_required_authorized={authorized} memory_admission_review_scope_required_appended={appended}"
    );
    if let Some(raw_value) = release_field(line, "issue2_memory_review_scope_required_proof") {
        if raw_value != derived.to_string() {
            return Err(format!(
                "{}:{} issue2_memory_review_scope_required_proof conflicts with review scope fields",
                path.display(),
                index + 1
            ));
        }
        Ok(format!(
            "{fields} issue2_memory_review_scope_required_proof_source=trace_report_input_derived"
        ))
    } else {
        Ok(format!(
            "{fields} issue2_memory_review_scope_required_proof={derived} issue2_memory_review_scope_required_proof_source=trace_report_input_derived"
        ))
    }
}

fn trace_memory_invalid_shape_rejection_statement(
    path: &Path,
    index: usize,
    line: &str,
) -> Result<String, String> {
    let fields_present = [
        "memory_admission_invalid_shape_rejection_verified",
        "memory_admission_invalid_shape_rejection_test",
        "memory_admission_invalid_shape_source_hash_present",
        "memory_admission_invalid_shape_kv_shape_valid",
        "memory_admission_invalid_shape_ledger_rejected",
        "memory_admission_invalid_shape_ledger_authorized",
        "memory_admission_invalid_shape_preview_read_only",
        "memory_admission_invalid_shape_preview_write_allowed",
        "issue2_memory_invalid_shape_rejection_proof",
    ]
    .iter()
    .any(|field| release_field(line, field).is_some());
    if !fields_present {
        return Ok(String::new());
    }

    let verified = required_issue_field(
        path,
        index,
        line,
        "memory_admission_invalid_shape_rejection_verified",
    )?;
    let test = required_issue_field(
        path,
        index,
        line,
        "memory_admission_invalid_shape_rejection_test",
    )?;
    let source_hash_present = required_issue_field(
        path,
        index,
        line,
        "memory_admission_invalid_shape_source_hash_present",
    )?;
    let kv_shape_valid = required_issue_field(
        path,
        index,
        line,
        "memory_admission_invalid_shape_kv_shape_valid",
    )?;
    let ledger_rejected = roundtrip_usize_field(
        path,
        index,
        "memory_admission_invalid_shape_ledger_rejected",
        release_field(line, "memory_admission_invalid_shape_ledger_rejected").unwrap_or(""),
    )?;
    let ledger_authorized = roundtrip_usize_field(
        path,
        index,
        "memory_admission_invalid_shape_ledger_authorized",
        release_field(line, "memory_admission_invalid_shape_ledger_authorized").unwrap_or(""),
    )?;
    let preview_read_only = required_issue_field(
        path,
        index,
        line,
        "memory_admission_invalid_shape_preview_read_only",
    )?;
    let preview_write_allowed = required_issue_field(
        path,
        index,
        line,
        "memory_admission_invalid_shape_preview_write_allowed",
    )?;
    let derived = verified == "true"
        && test
            == "memory_admission::tests::gene_segment_kv_records_reject_invalid_shape_without_write"
        && source_hash_present == "false"
        && kv_shape_valid == "false"
        && ledger_rejected > 0
        && ledger_authorized == 0
        && preview_read_only == "true"
        && preview_write_allowed == "false";
    let fields = format!(
        " memory_admission_invalid_shape_rejection_verified={verified} memory_admission_invalid_shape_rejection_test={test} memory_admission_invalid_shape_source_hash_present={source_hash_present} memory_admission_invalid_shape_kv_shape_valid={kv_shape_valid} memory_admission_invalid_shape_ledger_rejected={ledger_rejected} memory_admission_invalid_shape_ledger_authorized={ledger_authorized} memory_admission_invalid_shape_preview_read_only={preview_read_only} memory_admission_invalid_shape_preview_write_allowed={preview_write_allowed}"
    );
    if let Some(raw_value) = release_field(line, "issue2_memory_invalid_shape_rejection_proof") {
        if raw_value != derived.to_string() {
            return Err(format!(
                "{}:{} issue2_memory_invalid_shape_rejection_proof conflicts with invalid-shape fields",
                path.display(),
                index + 1
            ));
        }
        Ok(format!(
            "{fields} issue2_memory_invalid_shape_rejection_proof_source=trace_report_input_derived"
        ))
    } else {
        Ok(format!(
            "{fields} issue2_memory_invalid_shape_rejection_proof={derived} issue2_memory_invalid_shape_rejection_proof_source=trace_report_input_derived"
        ))
    }
}

fn trace_issue37_runtime_recall_scope_ready(
    path: &Path,
    index: usize,
    line: &str,
) -> Result<String, String> {
    let fields_present = [
        "issue37_runtime_recall_scope_verified",
        "issue37_runtime_recall_scope_test",
        "issue37_runtime_recall_scope_ready",
    ]
    .iter()
    .any(|field| release_field(line, field).is_some());
    if !fields_present {
        return Ok(String::new());
    }

    let verified =
        required_issue_field(path, index, line, "issue37_runtime_recall_scope_verified")?;
    let test = required_issue_field(path, index, line, "issue37_runtime_recall_scope_test")?;
    let derived = verified == "true"
        && test
            == "engine::tests::runtime_memory::inference_request_default_scope_isolates_runtime_memory_and_experience";
    let fields = format!(
        " issue37_runtime_recall_scope_verified={verified} issue37_runtime_recall_scope_test={test}"
    );
    if let Some(raw_value) = release_field(line, "issue37_runtime_recall_scope_ready") {
        if raw_value != derived.to_string() {
            return Err(format!(
                "{}:{} issue37_runtime_recall_scope_ready conflicts with runtime recall scope fields",
                path.display(),
                index + 1
            ));
        }
        Ok(format!(
            "{fields} issue37_runtime_recall_scope_ready={raw_value} issue37_runtime_recall_scope_ready_source=trace_report_input_derived"
        ))
    } else {
        Ok(format!(
            "{fields} issue37_runtime_recall_scope_ready={derived} issue37_runtime_recall_scope_ready_source=trace_report_input_derived"
        ))
    }
}

fn trace_memory_ledger_apply_proof(
    path: &Path,
    index: usize,
    line: &str,
) -> Result<String, String> {
    let authorized = roundtrip_usize_field(
        path,
        index,
        "memory_admission_ledger_authorized",
        release_field(line, "memory_admission_ledger_authorized").unwrap_or(""),
    )?;
    let applied = roundtrip_usize_field(
        path,
        index,
        "memory_admission_ledger_applied",
        release_field(line, "memory_admission_ledger_applied").unwrap_or(""),
    )?;
    let derived = authorized == 0 && applied == 0 && applied <= authorized;
    if let Some(raw_value) = release_field(line, "issue2_memory_ledger_apply_proof") {
        if raw_value != derived.to_string() {
            return Err(format!(
                "{}:{} issue2_memory_ledger_apply_proof conflicts with authorized/applied counters",
                path.display(),
                index + 1
            ));
        }
        Ok(" issue2_memory_ledger_apply_proof_source=trace_report_input_derived".to_owned())
    } else {
        Ok(format!(
            " issue2_memory_ledger_apply_proof={derived} issue2_memory_ledger_apply_proof_source=trace_report_input_derived"
        ))
    }
}

fn trace_memory_ledger_lifecycle_retention_proof(
    path: &Path,
    index: usize,
    line: &str,
) -> Result<String, String> {
    let records = roundtrip_usize_field(
        path,
        index,
        "memory_admission_ledger_records",
        release_field(line, "memory_admission_ledger_records").unwrap_or(""),
    )?;
    let duplicate = roundtrip_usize_field(
        path,
        index,
        "memory_admission_ledger_duplicate",
        release_field(line, "memory_admission_ledger_duplicate").unwrap_or(""),
    )?;
    let decayed = roundtrip_usize_field(
        path,
        index,
        "memory_admission_ledger_decayed",
        release_field(line, "memory_admission_ledger_decayed").unwrap_or(""),
    )?;
    let merged = roundtrip_usize_field(
        path,
        index,
        "memory_admission_ledger_merged",
        release_field(line, "memory_admission_ledger_merged").unwrap_or(""),
    )?;
    let rollback = roundtrip_usize_field(
        path,
        index,
        "memory_admission_ledger_rollback",
        release_field(line, "memory_admission_ledger_rollback").unwrap_or(""),
    )?;
    let lifecycle_total = duplicate + decayed + merged + rollback;
    let derived =
        release_field(line, "passed") == Some("true") && records > 0 && lifecycle_total <= records;
    if let Some(raw_value) = release_field(line, "issue2_memory_ledger_lifecycle_retention_proof") {
        if raw_value != derived.to_string() {
            return Err(format!(
                "{}:{} issue2_memory_ledger_lifecycle_retention_proof conflicts with lifecycle counters",
                path.display(),
                index + 1
            ));
        }
        Ok(
            " issue2_memory_ledger_lifecycle_retention_proof_source=trace_report_input_derived"
                .to_owned(),
        )
    } else {
        Ok(format!(
            " issue2_memory_ledger_lifecycle_retention_proof={derived} issue2_memory_ledger_lifecycle_retention_proof_source=trace_report_input_derived"
        ))
    }
}

fn trace_memory_admission_source_mix_proof(
    path: &Path,
    index: usize,
    line: &str,
) -> Result<String, String> {
    let fields_present = [
        "memory_admission_source_semantic",
        "memory_admission_source_gist",
        "memory_admission_source_runtime_kv",
        "memory_admission_source_cold",
        "memory_admission_source_gene_segment",
        "issue2_memory_admission_source_mix_proof",
    ]
    .iter()
    .any(|field| release_field(line, field).is_some());
    if !fields_present {
        return Ok(String::new());
    }

    let count =
        |field| roundtrip_usize_field(path, index, field, release_field(line, field).unwrap_or(""));
    let candidates = count("memory_admission_candidates")?;
    let ledger_records = count("memory_admission_ledger_records")?;
    let source_semantic = count("memory_admission_source_semantic")?;
    let source_gist = count("memory_admission_source_gist")?;
    let source_runtime_kv = count("memory_admission_source_runtime_kv")?;
    let source_cold = count("memory_admission_source_cold")?;
    let source_gene_segment = count("memory_admission_source_gene_segment")?;
    let source_total = source_semantic
        .saturating_add(source_gist)
        .saturating_add(source_runtime_kv)
        .saturating_add(source_cold)
        .saturating_add(source_gene_segment);
    let derived = release_field(line, "passed") == Some("true")
        && source_semantic > 0
        && source_gist > 0
        && source_runtime_kv > 0
        && source_cold > 0
        && source_gene_segment > 0
        && source_total == candidates
        && source_total == ledger_records;
    let fields = format!(" memory_admission_source_total={source_total}");
    if let Some(raw_value) = release_field(line, "issue2_memory_admission_source_mix_proof") {
        if raw_value != derived.to_string() {
            return Err(format!(
                "{}:{} issue2_memory_admission_source_mix_proof conflicts with source mix counters",
                path.display(),
                index + 1
            ));
        }
        Ok(format!(
            "{fields} issue2_memory_admission_source_mix_proof_source=trace_report_input_derived"
        ))
    } else {
        Ok(format!(
            "{fields} issue2_memory_admission_source_mix_proof={derived} issue2_memory_admission_source_mix_proof_source=trace_report_input_derived"
        ))
    }
}

fn trace_memory_gene_segment_metadata_proof(
    path: &Path,
    index: usize,
    line: &str,
) -> Result<String, String> {
    let fields_present = [
        "memory_admission_gene_segment_metadata",
        "issue2_memory_gene_segment_metadata_proof",
    ]
    .iter()
    .any(|field| release_field(line, field).is_some());
    if !fields_present {
        return Ok(String::new());
    }

    let count =
        |field| roundtrip_usize_field(path, index, field, release_field(line, field).unwrap_or(""));
    let source_gene_segment = count("memory_admission_source_gene_segment")?;
    let metadata = count("memory_admission_gene_segment_metadata")?;
    let derived = release_field(line, "passed") == Some("true")
        && source_gene_segment > 0
        && metadata == source_gene_segment;
    if let Some(raw_value) = release_field(line, "issue2_memory_gene_segment_metadata_proof") {
        if raw_value != derived.to_string() {
            return Err(format!(
                "{}:{} issue2_memory_gene_segment_metadata_proof conflicts with GeneSegment metadata counters",
                path.display(),
                index + 1
            ));
        }
        Ok(
            " issue2_memory_gene_segment_metadata_proof_source=trace_report_input_derived"
                .to_owned(),
        )
    } else {
        Ok(format!(
            " issue2_memory_gene_segment_metadata_proof={derived} issue2_memory_gene_segment_metadata_proof_source=trace_report_input_derived"
        ))
    }
}

fn trace_memory_residency_retention_compaction_proof(
    path: &Path,
    index: usize,
    line: &str,
) -> Result<String, String> {
    let fields_present = [
        "memory_retention_activity_cases",
        "memory_retention_decayed",
        "memory_retention_removed",
        "memory_compaction_activity_cases",
        "memory_compaction_merged",
        "memory_compaction_removed",
        "memory_compaction_pair_evidence",
        "memory_storage_samples",
        "memory_storage_entries_before",
        "memory_storage_entries_after",
        "memory_storage_entries_removed",
        "memory_storage_reduction_entries",
        "memory_retained_usefulness_abs_delta_milli",
        "issue2_memory_residency_retention_compaction_proof",
    ]
    .iter()
    .any(|field| release_field(line, field).is_some());
    if !fields_present {
        return Ok(String::new());
    }

    let count =
        |field| roundtrip_usize_field(path, index, field, release_field(line, field).unwrap_or(""));
    let retention_cases = count("memory_retention_activity_cases")?;
    let retention_decayed = count("memory_retention_decayed")?;
    let retention_removed = count("memory_retention_removed")?;
    let compaction_cases = count("memory_compaction_activity_cases")?;
    let compaction_merged = count("memory_compaction_merged")?;
    let compaction_removed = count("memory_compaction_removed")?;
    let compaction_pair_evidence = count("memory_compaction_pair_evidence")?;
    let storage_samples = count("memory_storage_samples")?;
    let storage_before = count("memory_storage_entries_before")?;
    let storage_after = count("memory_storage_entries_after")?;
    let storage_removed = count("memory_storage_entries_removed")?;
    let storage_reduction = count("memory_storage_reduction_entries")?;
    let retained_usefulness_abs_delta = count("memory_retained_usefulness_abs_delta_milli")?;
    let derived = release_field(line, "passed") == Some("true")
        && retention_cases > 0
        && retention_decayed.saturating_add(retention_removed) > 0
        && compaction_cases > 0
        && compaction_merged.saturating_add(compaction_removed) > 0
        && compaction_pair_evidence > 0
        && storage_samples > 0
        && storage_before > storage_after
        && storage_removed > 0
        && storage_reduction > 0
        && retained_usefulness_abs_delta > 0;
    let fields = format!(
        " memory_retention_activity_cases={retention_cases} memory_retention_decayed={retention_decayed} memory_retention_removed={retention_removed} memory_compaction_activity_cases={compaction_cases} memory_compaction_merged={compaction_merged} memory_compaction_removed={compaction_removed} memory_compaction_pair_evidence={compaction_pair_evidence} memory_storage_samples={storage_samples} memory_storage_entries_before={storage_before} memory_storage_entries_after={storage_after} memory_storage_entries_removed={storage_removed} memory_storage_reduction_entries={storage_reduction} memory_retained_usefulness_abs_delta_milli={retained_usefulness_abs_delta}"
    );
    if let Some(raw_value) =
        release_field(line, "issue2_memory_residency_retention_compaction_proof")
    {
        if raw_value != derived.to_string() {
            return Err(format!(
                "{}:{} issue2_memory_residency_retention_compaction_proof conflicts with memory residency counters",
                path.display(),
                index + 1
            ));
        }
        Ok(format!(
            "{fields} issue2_memory_residency_retention_compaction_proof_source=trace_report_input_derived"
        ))
    } else {
        Ok(format!(
            "{fields} issue2_memory_residency_retention_compaction_proof={derived} issue2_memory_residency_retention_compaction_proof_source=trace_report_input_derived"
        ))
    }
}

fn trace_memory_autophagy_preview_proof(
    path: &Path,
    index: usize,
    line: &str,
) -> Result<String, String> {
    if release_field(line, "memory_autophagy_detail_codes").is_some() {
        return Err(format!(
            "{}:{} memory_autophagy_detail_codes not allowed in trace summary",
            path.display(),
            index + 1
        ));
    }

    let fields_present = [
        "memory_autophagy_context_pressure_score",
        "memory_autophagy_retrieval_noise_score",
        "memory_autophagy_stale_decay_candidates",
        "memory_autophagy_duplicate_merge_candidates",
        "memory_autophagy_gist_recomposition_candidates",
        "memory_autophagy_active_recall_prune_candidates",
        "memory_autophagy_quarantine_candidates",
        "memory_autophagy_live_delete_allowed",
        "memory_autophagy_durable_mutation_allowed",
        "memory_autophagy_reason_codes",
        "issue499_memory_autophagy_preview_proof",
    ]
    .iter()
    .any(|field| release_field(line, field).is_some());
    if !fields_present {
        return Ok(String::new());
    }

    let count =
        |field| roundtrip_usize_field(path, index, field, release_field(line, field).unwrap_or(""));
    let context_pressure = count("memory_autophagy_context_pressure_score")?;
    let retrieval_noise = count("memory_autophagy_retrieval_noise_score")?;
    let stale_decay = count("memory_autophagy_stale_decay_candidates")?;
    let duplicate_merge = count("memory_autophagy_duplicate_merge_candidates")?;
    let gist_recomposition = count("memory_autophagy_gist_recomposition_candidates")?;
    let active_recall_prune = count("memory_autophagy_active_recall_prune_candidates")?;
    let quarantine = count("memory_autophagy_quarantine_candidates")?;
    let live_delete_allowed =
        required_issue_field(path, index, line, "memory_autophagy_live_delete_allowed")?;
    let durable_mutation_allowed = required_issue_field(
        path,
        index,
        line,
        "memory_autophagy_durable_mutation_allowed",
    )?;
    let reason_codes = required_issue_field(path, index, line, "memory_autophagy_reason_codes")?;

    if gist_recomposition != stale_decay.saturating_add(duplicate_merge) {
        return Err(format!(
            "{}:{} memory_autophagy_gist_recomposition_candidates conflicts with stale/duplicate counts",
            path.display(),
            index + 1
        ));
    }
    if live_delete_allowed != "false" {
        return Err(format!(
            "{}:{} memory_autophagy_live_delete_allowed must stay false",
            path.display(),
            index + 1
        ));
    }
    if durable_mutation_allowed != "false" {
        return Err(format!(
            "{}:{} memory_autophagy_durable_mutation_allowed must stay false",
            path.display(),
            index + 1
        ));
    }

    let candidate_total = stale_decay
        .saturating_add(duplicate_merge)
        .saturating_add(active_recall_prune)
        .saturating_add(quarantine);
    let derived = release_field(line, "passed") == Some("true")
        && context_pressure.saturating_add(retrieval_noise) > 0
        && candidate_total > 0
        && reason_codes != "none";
    let fields = format!(
        " memory_autophagy_context_pressure_score={context_pressure} memory_autophagy_retrieval_noise_score={retrieval_noise} memory_autophagy_stale_decay_candidates={stale_decay} memory_autophagy_duplicate_merge_candidates={duplicate_merge} memory_autophagy_gist_recomposition_candidates={gist_recomposition} memory_autophagy_active_recall_prune_candidates={active_recall_prune} memory_autophagy_quarantine_candidates={quarantine} memory_autophagy_live_delete_allowed={live_delete_allowed} memory_autophagy_durable_mutation_allowed={durable_mutation_allowed} memory_autophagy_reason_codes={reason_codes}"
    );
    if let Some(raw_value) = release_field(line, "issue499_memory_autophagy_preview_proof") {
        if raw_value != derived.to_string() {
            return Err(format!(
                "{}:{} issue499_memory_autophagy_preview_proof conflicts with autophagy counters",
                path.display(),
                index + 1
            ));
        }
        Ok(format!(
            "{fields} issue499_memory_autophagy_preview_proof_source=trace_report_input_derived"
        ))
    } else {
        Ok(format!(
            "{fields} issue499_memory_autophagy_preview_proof={derived} issue499_memory_autophagy_preview_proof_source=trace_report_input_derived"
        ))
    }
}

fn trace_validation_ready(path: &Path, index: usize, line: &str) -> Result<String, String> {
    let genome_events = roundtrip_usize_field(
        path,
        index,
        "reasoning_genome_events",
        release_field(line, "reasoning_genome_events").unwrap_or(""),
    )?;
    let genome_write_allowed = roundtrip_usize_field(
        path,
        index,
        "reasoning_genome_write_allowed",
        release_field(line, "reasoning_genome_write_allowed").unwrap_or(""),
    )?;
    let splice_write_allowed = roundtrip_usize_field(
        path,
        index,
        "reasoning_genome_splice_write_allowed",
        release_field(line, "reasoning_genome_splice_write_allowed").unwrap_or(""),
    )?;
    let admission_events = roundtrip_usize_field(
        path,
        index,
        "self_evolution_admission_events",
        release_field(line, "self_evolution_admission_events").unwrap_or(""),
    )?;
    let review_packets = roundtrip_usize_field(
        path,
        index,
        "self_evolution_admission_review_packets",
        release_field(line, "self_evolution_admission_review_packets").unwrap_or(""),
    )?;
    let evidence_ids = roundtrip_usize_field(
        path,
        index,
        "self_evolution_admission_evidence_ids",
        release_field(line, "self_evolution_admission_evidence_ids").unwrap_or(""),
    )?;
    let missing_refs = roundtrip_usize_field(
        path,
        index,
        "self_evolution_admission_missing_review_packet_refs",
        release_field(line, "self_evolution_admission_missing_review_packet_refs").unwrap_or(""),
    )?;
    let derived = release_field(line, "passed") == Some("true")
        && genome_events > 0
        && genome_write_allowed == 0
        && splice_write_allowed == 0
        && admission_events > 0
        && review_packets > 0
        && evidence_ids > 0
        && missing_refs == 0;
    if let Some(raw_value) = release_field(line, "issue30_trace_validation_ready") {
        if raw_value != derived.to_string() {
            return Err(format!(
                "{}:{} issue30_trace_validation_ready conflicts with trace fields",
                path.display(),
                index + 1
            ));
        }
        Ok(" issue30_trace_validation_ready_source=trace_report_input_derived".to_owned())
    } else {
        Ok(format!(
            " issue30_trace_validation_ready={derived} issue30_trace_validation_ready_source=trace_report_input_derived"
        ))
    }
}

fn trace_agent_team_layer_b_route_ready(
    path: &Path,
    index: usize,
    line: &str,
) -> Result<String, String> {
    let Some((fields, derived)) = trace_agent_team_layer_b_route_ready_fields(path, index, line)?
    else {
        return Ok(String::new());
    };
    if let Some(raw_value) = release_field(line, "issue185_agent_team_layer_b_route_ready") {
        if raw_value != derived.to_string() {
            return Err(format!(
                "{}:{} issue185_agent_team_layer_b_route_ready conflicts with agent_team route fields",
                path.display(),
                index + 1
            ));
        }
        Ok(format!(
            "{fields} issue185_agent_team_layer_b_route_ready_source=trace_report_input_derived"
        ))
    } else {
        Ok(format!(
            "{fields} issue185_agent_team_layer_b_route_ready={derived} issue185_agent_team_layer_b_route_ready_source=trace_report_input_derived"
        ))
    }
}

fn trace_agent_team_layer_b_route_ready_fields(
    path: &Path,
    index: usize,
    line: &str,
) -> Result<Option<(String, bool)>, String> {
    let Some(events) = release_field(line, "agent_team_events") else {
        if release_field(line, "issue185_agent_team_layer_b_route_ready").is_some() {
            return Err(format!(
                "{}:{} issue185_agent_team_layer_b_route_ready requires agent_team_events",
                path.display(),
                index + 1
            ));
        }
        return Ok(None);
    };
    let events = roundtrip_usize_field(path, index, "agent_team_events", events)?;
    let enabled = roundtrip_usize_field(
        path,
        index,
        "agent_team_enabled",
        &required_issue_field(path, index, line, "agent_team_enabled")?,
    )?;
    let proof_ready = roundtrip_usize_field(
        path,
        index,
        "agent_team_layer_b_route_proof_ready",
        &required_issue_field(path, index, line, "agent_team_layer_b_route_proof_ready")?,
    )?;
    let route_complete = roundtrip_usize_field(
        path,
        index,
        "agent_team_layer_b_route_complete",
        &required_issue_field(path, index, line, "agent_team_layer_b_route_complete")?,
    )?;
    let derived = release_field(line, "passed") == Some("true")
        && events > 0
        && enabled > 0
        && proof_ready == enabled
        && route_complete == enabled;
    let fields = format!(
        " agent_team_events={events} agent_team_enabled={enabled} agent_team_layer_b_route_proof_ready={proof_ready} agent_team_layer_b_route_complete={route_complete}"
    );
    Ok(Some((fields, derived)))
}

fn trace_issue185_agent_team_contract_ready(
    path: &Path,
    index: usize,
    line: &str,
) -> Result<String, String> {
    let Some((fields, derived)) =
        trace_issue185_agent_team_contract_ready_fields(path, index, line)?
    else {
        return Ok(String::new());
    };
    if let Some(raw_value) = release_field(line, "issue185_agent_team_contract_ready") {
        if raw_value != derived.to_string() {
            return Err(format!(
                "{}:{} issue185_agent_team_contract_ready conflicts with agent_team contract fields",
                path.display(),
                index + 1
            ));
        }
        Ok(format!(
            "{fields} issue185_agent_team_contract_ready_source=trace_report_input_derived"
        ))
    } else {
        Ok(format!(
            "{fields} issue185_agent_team_contract_ready={derived} issue185_agent_team_contract_ready_source=trace_report_input_derived"
        ))
    }
}

fn trace_issue185_agent_team_contract_ready_fields(
    path: &Path,
    index: usize,
    line: &str,
) -> Result<Option<(String, bool)>, String> {
    let Some(agents) = release_field(line, "agent_team_agents") else {
        if release_field(line, "issue185_agent_team_contract_ready").is_some() {
            return Err(format!(
                "{}:{} issue185_agent_team_contract_ready requires agent_team_agents",
                path.display(),
                index + 1
            ));
        }
        return Ok(None);
    };
    let events = roundtrip_usize_field(
        path,
        index,
        "agent_team_events",
        &required_issue_field(path, index, line, "agent_team_events")?,
    )?;
    let enabled = roundtrip_usize_field(
        path,
        index,
        "agent_team_enabled",
        &required_issue_field(path, index, line, "agent_team_enabled")?,
    )?;
    let agents = roundtrip_usize_field(path, index, "agent_team_agents", agents)?;
    let messages = roundtrip_usize_field(
        path,
        index,
        "agent_team_messages",
        &required_issue_field(path, index, line, "agent_team_messages")?,
    )?;
    let aggregation_lanes = roundtrip_usize_field(
        path,
        index,
        "agent_team_aggregation_lanes",
        &required_issue_field(path, index, line, "agent_team_aggregation_lanes")?,
    )?;
    let aggregation_messages = roundtrip_usize_field(
        path,
        index,
        "agent_team_aggregation_messages",
        &required_issue_field(path, index, line, "agent_team_aggregation_messages")?,
    )?;
    let conflicts = roundtrip_usize_field(
        path,
        index,
        "agent_team_conflicts",
        &required_issue_field(path, index, line, "agent_team_conflicts")?,
    )?;
    let unresolved_conflicts = roundtrip_usize_field(
        path,
        index,
        "agent_team_unresolved_conflicts",
        &required_issue_field(path, index, line, "agent_team_unresolved_conflicts")?,
    )?;
    let collision_free = roundtrip_usize_field(
        path,
        index,
        "agent_team_collision_free",
        &required_issue_field(path, index, line, "agent_team_collision_free")?,
    )?;
    let single_writer = roundtrip_usize_field(
        path,
        index,
        "agent_team_single_writer",
        &required_issue_field(path, index, line, "agent_team_single_writer")?,
    )?;
    let read_only_subagents = roundtrip_usize_field(
        path,
        index,
        "agent_team_read_only_subagents",
        &required_issue_field(path, index, line, "agent_team_read_only_subagents")?,
    )?;
    let budget_isolated = roundtrip_usize_field(
        path,
        index,
        "agent_team_budget_isolated",
        &required_issue_field(path, index, line, "agent_team_budget_isolated")?,
    )?;
    let main_thread_writer = roundtrip_usize_field(
        path,
        index,
        "agent_team_main_thread_writer",
        &required_issue_field(path, index, line, "agent_team_main_thread_writer")?,
    )?;
    let derived = release_field(line, "passed") == Some("true")
        && events > 0
        && enabled > 0
        && agents >= 2
        && messages >= agents
        && aggregation_lanes >= 2
        && aggregation_messages >= 2
        && conflicts > 0
        && unresolved_conflicts == 0
        && collision_free == enabled
        && single_writer == enabled
        && read_only_subagents == enabled
        && budget_isolated == enabled
        && main_thread_writer == enabled;
    let fields = format!(
        " agent_team_agents={agents} agent_team_messages={messages} agent_team_aggregation_lanes={aggregation_lanes} agent_team_aggregation_messages={aggregation_messages} agent_team_conflicts={conflicts} agent_team_unresolved_conflicts={unresolved_conflicts} agent_team_collision_free={collision_free} agent_team_single_writer={single_writer} agent_team_read_only_subagents={read_only_subagents} agent_team_budget_isolated={budget_isolated} agent_team_main_thread_writer={main_thread_writer}"
    );
    Ok(Some((fields, derived)))
}

fn trace_issue185_coding_service_eval_self_validation_ready(
    path: &Path,
    index: usize,
    line: &str,
) -> Result<String, String> {
    let Some((fields, derived)) =
        trace_issue185_coding_service_eval_self_validation_ready_fields(path, index, line)?
    else {
        return Ok(String::new());
    };
    if let Some(raw_value) =
        release_field(line, "issue185_coding_service_eval_self_validation_ready")
    {
        if raw_value != derived.to_string() {
            return Err(format!(
                "{}:{} issue185_coding_service_eval_self_validation_ready conflicts with coding service eval fields",
                path.display(),
                index + 1
            ));
        }
        Ok(format!(
            "{fields} issue185_coding_service_eval_self_validation_ready_source=trace_report_input_derived"
        ))
    } else {
        Ok(format!(
            "{fields} issue185_coding_service_eval_self_validation_ready={derived} issue185_coding_service_eval_self_validation_ready_source=trace_report_input_derived"
        ))
    }
}

fn trace_issue185_coding_service_eval_self_validation_ready_fields(
    path: &Path,
    index: usize,
    line: &str,
) -> Result<Option<(String, bool)>, String> {
    let Some(events) = release_field(line, "coding_service_eval_events") else {
        if release_field(line, "issue185_coding_service_eval_self_validation_ready").is_some() {
            return Err(format!(
                "{}:{} issue185_coding_service_eval_self_validation_ready requires coding_service_eval_events",
                path.display(),
                index + 1
            ));
        }
        return Ok(None);
    };
    let events = roundtrip_usize_field(path, index, "coding_service_eval_events", events)?;
    let readiness_events = roundtrip_usize_field(
        path,
        index,
        "coding_service_eval_readiness_events",
        &required_issue_field(path, index, line, "coding_service_eval_readiness_events")?,
    )?;
    let runner_events = roundtrip_usize_field(
        path,
        index,
        "coding_service_eval_runner_events",
        &required_issue_field(path, index, line, "coding_service_eval_runner_events")?,
    )?;
    let passed_events = roundtrip_usize_field(
        path,
        index,
        "coding_service_eval_passed",
        &required_issue_field(path, index, line, "coding_service_eval_passed")?,
    )?;
    let requests = roundtrip_usize_field(
        path,
        index,
        "coding_service_eval_requests",
        &required_issue_field(path, index, line, "coding_service_eval_requests")?,
    )?;
    let completed = roundtrip_usize_field(
        path,
        index,
        "coding_service_eval_completed",
        &required_issue_field(path, index, line, "coding_service_eval_completed")?,
    )?;
    let evidence_packets = roundtrip_usize_field(
        path,
        index,
        "coding_service_eval_evidence_packets",
        &required_issue_field(path, index, line, "coding_service_eval_evidence_packets")?,
    )?;
    let rust_validation_checked = roundtrip_usize_field(
        path,
        index,
        "coding_service_eval_rust_validation_checked",
        &required_issue_field(
            path,
            index,
            line,
            "coding_service_eval_rust_validation_checked",
        )?,
    )?;
    let compile_checked = roundtrip_usize_field(
        path,
        index,
        "coding_service_eval_compile_checked",
        &required_issue_field(path, index, line, "coding_service_eval_compile_checked")?,
    )?;
    let unit_test_checked = roundtrip_usize_field(
        path,
        index,
        "coding_service_eval_unit_test_checked",
        &required_issue_field(path, index, line, "coding_service_eval_unit_test_checked")?,
    )?;
    let benchmark_checked = roundtrip_usize_field(
        path,
        index,
        "coding_service_eval_benchmark_checked",
        &required_issue_field(path, index, line, "coding_service_eval_benchmark_checked")?,
    )?;
    let benchmark_passed = roundtrip_usize_field(
        path,
        index,
        "coding_service_eval_benchmark_passed",
        &required_issue_field(path, index, line, "coding_service_eval_benchmark_passed")?,
    )?;
    let layer_b_route_proof_ready = roundtrip_usize_field(
        path,
        index,
        "coding_service_eval_layer_b_route_proof_ready",
        &required_issue_field(
            path,
            index,
            line,
            "coding_service_eval_layer_b_route_proof_ready",
        )?,
    )?;
    let rust_validation_layer_b_route_ready = roundtrip_usize_field(
        path,
        index,
        "coding_service_eval_rust_validation_layer_b_route_ready",
        &required_issue_field(
            path,
            index,
            line,
            "coding_service_eval_rust_validation_layer_b_route_ready",
        )?,
    )?;
    let write_allowed = roundtrip_usize_field(
        path,
        index,
        "coding_service_eval_write_allowed",
        &required_issue_field(path, index, line, "coding_service_eval_write_allowed")?,
    )?;
    let applied = roundtrip_usize_field(
        path,
        index,
        "coding_service_eval_applied",
        &required_issue_field(path, index, line, "coding_service_eval_applied")?,
    )?;
    let derived = release_field(line, "passed") == Some("true")
        && events > 0
        && readiness_events.checked_add(runner_events) == Some(events)
        && runner_events > 0
        && passed_events == events
        && requests > 0
        && completed == requests
        && evidence_packets == requests
        && rust_validation_checked > 0
        && compile_checked == rust_validation_checked
        && unit_test_checked == rust_validation_checked
        && benchmark_checked == requests
        && benchmark_passed == benchmark_checked
        && layer_b_route_proof_ready == requests
        && rust_validation_layer_b_route_ready == rust_validation_checked
        && write_allowed == 0
        && applied == 0;
    let fields = format!(
        " coding_service_eval_events={events} coding_service_eval_readiness_events={readiness_events} coding_service_eval_runner_events={runner_events} coding_service_eval_passed={passed_events} coding_service_eval_requests={requests} coding_service_eval_completed={completed} coding_service_eval_evidence_packets={evidence_packets} coding_service_eval_rust_validation_checked={rust_validation_checked} coding_service_eval_compile_checked={compile_checked} coding_service_eval_unit_test_checked={unit_test_checked} coding_service_eval_benchmark_checked={benchmark_checked} coding_service_eval_benchmark_passed={benchmark_passed} coding_service_eval_layer_b_route_proof_ready={layer_b_route_proof_ready} coding_service_eval_rust_validation_layer_b_route_ready={rust_validation_layer_b_route_ready} coding_service_eval_write_allowed={write_allowed} coding_service_eval_applied={applied}"
    );
    Ok(Some((fields, derived)))
}

fn trace_issue185_agent_tooling_mvp_ready(
    path: &Path,
    index: usize,
    line: &str,
) -> Result<String, String> {
    let Some((_, agent_team_ready)) =
        trace_agent_team_layer_b_route_ready_fields(path, index, line)?
    else {
        if release_field(line, "issue185_agent_tooling_mvp_ready").is_some() {
            return Err(format!(
                "{}:{} issue185_agent_tooling_mvp_ready requires agent_team_events",
                path.display(),
                index + 1
            ));
        }
        return Ok(String::new());
    };
    let Some((_, agent_team_contract_ready)) =
        trace_issue185_agent_team_contract_ready_fields(path, index, line)?
    else {
        if release_field(line, "issue185_agent_tooling_mvp_ready").is_some() {
            return Err(format!(
                "{}:{} issue185_agent_tooling_mvp_ready requires agent_team contract fields",
                path.display(),
                index + 1
            ));
        }
        return Ok(String::new());
    };
    let Some((_, coding_service_ready)) =
        trace_issue185_coding_service_eval_self_validation_ready_fields(path, index, line)?
    else {
        if release_field(line, "issue185_agent_tooling_mvp_ready").is_some() {
            return Err(format!(
                "{}:{} issue185_agent_tooling_mvp_ready requires coding_service_eval_events",
                path.display(),
                index + 1
            ));
        }
        return Ok(String::new());
    };
    let derived = release_field(line, "passed") == Some("true")
        && agent_team_ready
        && agent_team_contract_ready
        && coding_service_ready;
    if let Some(raw_value) = release_field(line, "issue185_agent_tooling_mvp_ready") {
        if raw_value != derived.to_string() {
            return Err(format!(
                "{}:{} issue185_agent_tooling_mvp_ready conflicts with issue185 agent/tooling fields",
                path.display(),
                index + 1
            ));
        }
        Ok(" issue185_agent_tooling_mvp_ready_source=trace_report_input_derived".to_owned())
    } else {
        Ok(format!(
            " issue185_agent_tooling_mvp_ready={derived} issue185_agent_tooling_mvp_ready_source=trace_report_input_derived"
        ))
    }
}

fn trace_chaperone_fold_guard_ready(
    path: &Path,
    index: usize,
    line: &str,
) -> Result<String, String> {
    let Some(fold_status) =
        release_field(line, "issue503_fold_status").or_else(|| release_field(line, "fold_status"))
    else {
        return Ok(String::new());
    };
    if !matches!(fold_status, "stable" | "watch" | "repair") {
        return Err(format!(
            "{}:{} issue503_fold_status must be stable, watch, or repair",
            path.display(),
            index + 1
        ));
    }

    let undefined_capability_count = trace_chaperone_count_field(
        path,
        index,
        line,
        "issue503_undefined_capability_count",
        "undefined_capability_count",
    )?;
    let contradiction_count = trace_chaperone_count_field(
        path,
        index,
        line,
        "issue503_contradiction_count",
        "contradiction_count",
    )?;
    let ungated_side_effect_count = trace_chaperone_count_field(
        path,
        index,
        line,
        "issue503_ungated_side_effect_count",
        "ungated_side_effect_count",
    )?;
    let missing_evidence_count = trace_chaperone_count_field(
        path,
        index,
        line,
        "issue503_missing_evidence_count",
        "missing_evidence_count",
    )?;
    let repair_task_count = trace_chaperone_count_field(
        path,
        index,
        line,
        "issue503_repair_task_count",
        "repair_task_count",
    )?;
    let raw_cot_captured = release_field(line, "issue503_raw_cot_captured")
        .or_else(|| release_field(line, "raw_cot_captured"))
        .ok_or_else(|| {
            format!(
                "{}:{} missing issue503_raw_cot_captured",
                path.display(),
                index + 1
            )
        })?;
    let raw_prompt_captured = release_field(line, "issue503_raw_prompt_captured")
        .or_else(|| release_field(line, "raw_prompt_captured"))
        .unwrap_or("false");
    let verified = release_field(line, "issue503_chaperone_fold_guard_verified")
        .or_else(|| release_field(line, "reasoning_chaperone_fold_guard_verified"))
        .unwrap_or("true");

    let blocking_count = undefined_capability_count
        + contradiction_count
        + ungated_side_effect_count
        + missing_evidence_count;
    let count_shape_valid = match fold_status {
        "stable" => blocking_count == 0 && repair_task_count == 0,
        "watch" => repair_task_count == 0,
        "repair" => blocking_count > 0 && repair_task_count == 1,
        _ => false,
    };
    let derived = verified == "true"
        && raw_cot_captured == "false"
        && raw_prompt_captured == "false"
        && repair_task_count <= 1
        && count_shape_valid;

    if let Some(raw_value) = release_field(line, "issue503_chaperone_fold_guard_ready") {
        if raw_value != derived.to_string() {
            return Err(format!(
                "{}:{} issue503_chaperone_fold_guard_ready conflicts with fold guard fields",
                path.display(),
                index + 1
            ));
        }
    }

    Ok(format!(
        " issue503_fold_status={fold_status} issue503_undefined_capability_count={undefined_capability_count} issue503_contradiction_count={contradiction_count} issue503_ungated_side_effect_count={ungated_side_effect_count} issue503_missing_evidence_count={missing_evidence_count} issue503_repair_task_count={repair_task_count} issue503_raw_cot_captured={raw_cot_captured} issue503_chaperone_fold_guard_ready={derived} issue503_chaperone_fold_guard_ready_source=trace_report_input_derived"
    ))
}

fn trace_chaperone_count_field(
    path: &Path,
    index: usize,
    line: &str,
    preferred: &str,
    fallback: &str,
) -> Result<usize, String> {
    let value = release_field(line, preferred)
        .or_else(|| release_field(line, fallback))
        .ok_or_else(|| format!("{}:{} missing {preferred}", path.display(), index + 1))?;
    roundtrip_usize_field(path, index, preferred, value)
}

fn state_gate_statement(path: &Path) -> Result<String, String> {
    let raw = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    for (index, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if !line.starts_with("state_inspection_gate: ") {
            return Err(format!(
                "{}:{} expected state_inspection_gate summary line",
                path.display(),
                index + 1
            ));
        }
        let passed = required_issue_field(path, index, line, "passed")?;
        let failures = required_issue_field(path, index, line, "failures")?;
        let state_inspection_ready = passed == "true" && failures == "0";
        if let Some(raw_value) = release_field(line, "issue30_state_inspection_ready") {
            if raw_value != state_inspection_ready.to_string() {
                return Err(format!(
                    "{}:{} issue30_state_inspection_ready conflicts with state gate fields",
                    path.display(),
                    index + 1
                ));
            }
        }
        return Ok(format!(
            "state_inspection_gate: passed={passed} failures={failures} issue30_state_inspection_ready={state_inspection_ready} issue30_state_inspection_ready_source=state_gate_input_derived state_gate_source=state_gate_input"
        ));
    }
    Err(format!("{} has no state gate rows", path.display()))
}

fn research_sandbox_statement(path: &Path) -> Result<String, String> {
    let raw = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    for (index, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields = line
            .strip_prefix("research_sandbox_evidence ")
            .unwrap_or(line);
        let schema = required_issue_field(path, index, fields, "schema")?;
        if schema != "research_sandbox_evidence_v1" {
            return Err(format!(
                "{}:{} unsupported research sandbox schema",
                path.display(),
                index + 1
            ));
        }
        let target = required_issue_field(path, index, fields, "target")?;
        if !matches!(target.as_str(), "local" | "wsl" | "container" | "small-vps") {
            return Err(format!(
                "{}:{} unsupported research sandbox target",
                path.display(),
                index + 1
            ));
        }
        let profile = required_issue_field(path, index, fields, "profile")?;
        if !matches!(
            profile.as_str(),
            "cpu-only" | "single-gpu" | "low-memory" | "benchmark-replay"
        ) {
            return Err(format!(
                "{}:{} unsupported research sandbox profile",
                path.display(),
                index + 1
            ));
        }
        for (field, expected) in [
            ("noncommercial_only", "true"),
            ("contributor_pr_only", "true"),
            ("maintainer_approval_required", "true"),
            ("private_trace_publish_allowed", "false"),
            ("redacted_issue_comment_ready", "true"),
            ("wipe_test_state_supported", "true"),
            ("preview_only", "true"),
            ("write_allowed", "false"),
            ("durable_write_allowed", "false"),
            ("applied", "false"),
        ] {
            let value = required_issue_field(path, index, fields, field)?;
            if value != expected {
                return Err(format!(
                    "{}:{} {field} must be {expected}",
                    path.display(),
                    index + 1
                ));
            }
        }
        let persistent_state = required_issue_field(path, index, fields, "persistent_state")?;
        for token in ["disk_kv_cache", "redacted_evidence_packets"] {
            if !field_has_token(&persistent_state, token) {
                return Err(format!(
                    "{}:{} persistent_state missing {token}",
                    path.display(),
                    index + 1
                ));
            }
        }
        let local_only_data = required_issue_field(path, index, fields, "local_only_data")?;
        for token in ["raw_traces", "secrets"] {
            if !field_has_token(&local_only_data, token) {
                return Err(format!(
                    "{}:{} local_only_data missing {token}",
                    path.display(),
                    index + 1
                ));
            }
        }
        let safe = true;
        if let Some(raw_value) = release_field(fields, "research_sandbox_issue_comment_safe") {
            if raw_value != safe.to_string() {
                return Err(format!(
                    "{}:{} research_sandbox_issue_comment_safe conflicts with sandbox fields",
                    path.display(),
                    index + 1
                ));
            }
        }
        return Ok(format!(
            "research_sandbox_evidence schema={schema} target={target} profile={profile} noncommercial_only=true contributor_pr_only=true maintainer_approval_required=true persistent_state={persistent_state} local_only_data={local_only_data} private_trace_publish_allowed=false redacted_issue_comment_ready=true wipe_test_state_supported=true preview_only=true write_allowed=false durable_write_allowed=false applied=false research_sandbox_issue_comment_safe={safe} research_sandbox_issue_comment_safe_source=research_sandbox_input_derived research_sandbox_source=research_sandbox_input"
        ));
    }
    Err(format!("{} has no research sandbox rows", path.display()))
}

fn field_has_token(value: &str, token: &str) -> bool {
    value
        .split(|ch| ch == '|' || ch == ',')
        .any(|part| part == token)
}

const ISSUE30_ENTRY_CHAIN_REQUIRED_FIELDS: &[&str] = &[
    "issue30_environment_pressure_present",
    "issue30_pollution_event_id",
    "issue385_self_ontology_body_present",
    "issue385_body_state_id",
    "issue385_pheromone_signal_marker_present",
    "issue385_pheromone_signal_marker_id",
    "issue385_pheromone_signal_surface",
    "issue385_pheromone_signal_digest_gate_allowed",
    "issue385_pheromone_signal_preview_only",
    "issue375_pre_reasoning_genome_isa_present",
    "issue375_reasoning_frame_id",
    "issue375_reasoning_frame_environment_signals_present",
    "issue375_reasoning_frame_allowed_observations",
    "issue375_reasoning_frame_action_vocab",
    "issue375_reasoning_frame_suppressed_capabilities",
    "issue375_reasoning_frame_risk_limits",
    "issue375_expression_vm_side_effect",
    "issue375_genome_isa_apply_allowed",
    "issue30_backend_action",
    "issue4_dna_candidate_ledger_present",
    "issue4_dna_candidate_ledger_schema",
    "issue4_dna_candidate_ledger_records",
    "issue4_dna_candidate_ledger_candidate_count",
    "issue4_dna_candidate_ledger_candidate_only",
    "issue4_dna_candidate_ledger_digest",
    "issue4_dna_candidate_ledger_raw_records_allowed",
    "issue4_dna_candidate_ledger_write_allowed",
    "issue4_dna_candidate_ledger_applied",
    "issue4_dna_candidate_ledger_preview_source",
    "issue243_active_control_knobs",
    "issue243_evidence_digest",
    "issue243_policy_version",
    "issue243_decision_reason",
    "issue243_control_expression_profile_selected",
    "issue243_context_anchor_promoted",
    "issue243_suppression_gate_triggered",
    "issue243_checkpoint_repair_requested",
    "issue243_checkpoint_rejected",
    "issue243_memory_refresh_candidate",
    "issue243_memory_tombstone_candidate",
    "issue243_control_expression_preview_admission",
    "issue243_write_allowed",
    "issue243_applied",
    "issue243_operator_approval_required",
    "issue379_control_candidate_preview_only",
    "issue379_action_vocab_mask_preview",
    "issue379_signal_saliency_bias_preview",
    "issue379_zero_beat_primitive_decision_present",
    "issue379_primitive_authority",
    "issue379_primitive_side_effect",
    "issue379_primitive_reversibility",
    "issue379_primitive_evidence",
    "issue379_primitive_uncertainty",
    "issue379_primitive_attention",
    "issue379_zero_beat_output",
    "issue379_generation_bias_apply_allowed",
    "issue493_tool_organ_registry_present",
    "issue493_tool_organ_registry_id",
    "issue493_tool_organ_registry_preview_only",
    "issue493_tool_organ_registry_side_effect",
    "issue493_tool_organ_registry_apply_allowed",
    "issue493_tool_organ_capability_matrix_digest",
    "issue493_preview_bundle_protocol",
    "issue493_preview_bundle_digest",
    "issue493_preview_bundle_refs_digest_only",
    "issue493_preview_bundle_raw_artifacts_allowed",
    "issue493_tool_install_allowed",
    "issue493_tool_execution_allowed",
    "bio_epigenetic_expression_marker_present",
    "bio_epigenetic_expression_marker_id",
    "bio_mrna_cache_candidate_digest",
    "bio_expression_cache_protocol",
    "bio_expression_cache_key_digest",
    "bio_hot_path_observation_window",
    "bio_hot_path_min_success_rate",
    "bio_gate_relaxation_allowed",
    "bio_cache_materialization_allowed",
    "bio_raw_payload_or_kv_cached",
    "bio_negative_evidence_overrides",
    "issue501_telomere_state_present",
    "issue501_remaining_tokens",
    "issue501_remaining_steps",
    "issue501_remaining_messages",
    "issue501_repair_streak_count",
    "issue501_loop_risk_signal_count",
    "issue501_senescent",
    "issue501_apoptosis_required",
    "issue501_new_external_call_allowed",
    "issue501_new_file_write_allowed",
    "issue501_new_memory_write_allowed",
    "issue501_new_adaptive_state_write_allowed",
    "issue501_memory_promotion_allowed",
    "issue501_genome_mutation_allowed",
    "issue501_takeover_packet_digest",
    "issue501_rollback_anchor_digest",
    "issue501_handoff_next_owner",
    "issue501_raw_payload_present",
    "issue501_preview_side_effect_allowed",
    "issue502_pheromone_blackboard_present",
    "issue502_signal_count",
    "issue502_ranked_action_count",
    "issue502_top_signal_kind",
    "issue502_top_action",
    "issue502_blackboard_digest",
    "issue502_source_digest",
    "issue502_payload_digest",
    "issue502_raw_payload_present",
    "issue502_side_effect_allowed",
    "issue502_ttl_decay_present",
    "issue502_conflict_routes_to_repair",
    "issue502_ranked_actions_from_state_only",
    "issue509_quorum_sensing_present",
    "issue509_decision_id",
    "issue509_quorum_report_digest",
    "issue509_risk_class",
    "issue509_required_quorum_milli",
    "issue509_evaluator_count",
    "issue509_independent_model_count",
    "issue509_independent_lane_count",
    "issue509_approve_signal_count",
    "issue509_reject_signal_count",
    "issue509_abstain_signal_count",
    "issue509_approval_concentration_milli",
    "issue509_conflict_count",
    "issue509_quorum_reached",
    "issue509_apply_allowed",
    "issue509_raw_evaluator_payload_present",
    "issue509_duplicate_sources_count_once",
    "issue509_conflict_routes_to_repair",
    "issue509_writer_gate_bypass_allowed",
];

const ISSUE30_ENTRY_CHAIN_FALSE_FIELDS: &[&str] = &[
    "issue375_genome_isa_apply_allowed",
    "issue4_dna_candidate_ledger_raw_records_allowed",
    "issue4_dna_candidate_ledger_write_allowed",
    "issue4_dna_candidate_ledger_applied",
    "issue243_write_allowed",
    "issue243_applied",
    "issue379_generation_bias_apply_allowed",
    "issue493_tool_organ_registry_apply_allowed",
    "issue493_preview_bundle_raw_artifacts_allowed",
    "issue493_tool_install_allowed",
    "issue493_tool_execution_allowed",
    "bio_gate_relaxation_allowed",
    "bio_cache_materialization_allowed",
    "bio_raw_payload_or_kv_cached",
    "issue501_new_external_call_allowed",
    "issue501_new_file_write_allowed",
    "issue501_new_memory_write_allowed",
    "issue501_new_adaptive_state_write_allowed",
    "issue501_memory_promotion_allowed",
    "issue501_genome_mutation_allowed",
    "issue501_raw_payload_present",
    "issue501_preview_side_effect_allowed",
    "issue502_raw_payload_present",
    "issue502_side_effect_allowed",
    "issue509_apply_allowed",
    "issue509_raw_evaluator_payload_present",
    "issue509_writer_gate_bypass_allowed",
];

#[derive(Debug, Clone, Copy)]
struct Issue30EntryChainContext<'a> {
    line: &'a str,
}

impl<'a> Issue30EntryChainContext<'a> {
    fn parse(path: &Path, index: usize, line: &'a str) -> Result<Self, String> {
        require_issue_fields(path, index, line, ISSUE30_ENTRY_CHAIN_REQUIRED_FIELDS)?;
        let record = Self { line };
        record.validate_preview_only(path)?;
        Ok(record)
    }

    fn field(&self, name: &str) -> Option<&'a str> {
        release_field(self.line, name)
    }

    fn validate_preview_only(&self, path: &Path) -> Result<(), String> {
        for field in ISSUE30_ENTRY_CHAIN_FALSE_FIELDS {
            if self.field(field) != Some("false") {
                return Err(format!(
                    "{} {field} must stay false for issue30 entry-chain preview",
                    path.display()
                ));
            }
        }
        Ok(())
    }
}

fn issue30_context_statement(path: &Path) -> Result<String, String> {
    let raw = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let mut entry_chain = None;
    let mut problem_hypothesis = None;
    for (index, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with("issue30_environment_pressure_present=") {
            let entry_record = Issue30EntryChainContext::parse(path, index, line)?;
            entry_chain = Some(entry_record.line.to_owned());
        } else if line.starts_with("issue377_problem_finding_present=") {
            require_issue_fields(
                path,
                index,
                line,
                &[
                    "issue377_problem_finding_present",
                    "issue377_problem_finding_id",
                    "issue377_problem_finding_kind",
                    "issue377_problem_finding_severity",
                    "issue377_problem_finding_confidence_milli",
                    "issue377_problem_finding_evidence_digest",
                    "issue377_problem_finding_source_digest",
                    "issue377_problem_finding_affected_surface",
                    "issue377_problem_finding_next_step",
                    "issue377_problem_finding_raw_payload_present",
                    "issue377_self_observation_present",
                    "issue377_self_observation_id",
                    "issue377_self_observation_schema",
                    "issue377_self_observation_signal_source",
                    "issue377_self_observation_source_digest",
                    "issue377_self_observation_window",
                    "issue377_self_observation_current_truth_digest",
                    "issue377_self_observation_digest_only",
                    "issue377_self_observation_raw_payload_present",
                    "issue377_self_observation_write_allowed",
                    "issue377_self_observation_applied",
                    "issue377_self_model_present",
                    "issue377_self_model_id",
                    "issue377_self_model_schema",
                    "issue377_self_model_scope",
                    "issue377_self_model_claims_consciousness",
                    "issue377_self_model_digest_only",
                    "issue377_self_model_raw_payload_present",
                    "issue377_self_model_write_allowed",
                    "issue377_self_model_applied",
                    "issue377_hypothesis_candidate_present",
                    "issue377_hypothesis_candidate_id",
                    "issue377_hypothesis_candidate_kind",
                    "issue377_hypothesis_candidate_status",
                    "issue377_hypothesis_candidate_target_surface",
                    "issue377_hypothesis_candidate_expected_metric",
                    "issue377_hypothesis_candidate_expected_direction",
                    "issue377_hypothesis_candidate_required_gates",
                    "issue377_hypothesis_candidate_rollback_anchor",
                    "issue377_hypothesis_candidate_raw_payload_present",
                    "issue377_hypothesis_candidate_write_allowed",
                    "issue377_hypothesis_candidate_applied",
                    "issue377_hypothesis_candidate_operator_approval_required",
                    "issue377_problem_hypothesis_link",
                    "issue377_admission_decision",
                    "issue377_lexicographic_admission_present",
                    "issue377_lexicographic_admission_order",
                    "issue377_user_intent_preserved",
                    "issue377_safety_gate_passed",
                    "issue377_digest_only_evidence_gate_passed",
                    "issue377_rollback_anchor_gate_passed",
                    "issue377_quality_delta_milli",
                    "issue377_cost_delta_milli",
                    "issue377_latency_delta_milli",
                    "issue377_performance_tiebreaker_only",
                    "issue377_hard_gate_failure_action",
                    "issue377_risk_override_action",
                    "issue377_negative_evidence_count",
                    "issue377_privacy_risk",
                    "issue377_license_risk",
                    "issue377_unsupported_capability_requested",
                    "issue377_unsafe_side_effect_allowed",
                    "issue377_risk_override_clear",
                    "issue377_lexicographic_admission_apply_allowed",
                    "issue377_best_next_state",
                    "issue377_best_next_state_id",
                    "issue377_best_next_state_selected",
                    "issue377_predicament_signal_present",
                    "issue377_predicament_id",
                    "issue377_predicament_progress_delta",
                    "issue377_predicament_repeat_count",
                    "issue377_predicament_evidence_gap_count",
                    "issue377_predicament_action_novelty",
                    "issue377_predicament_stuck",
                    "issue377_self_trigger_stage",
                    "issue377_evolution_apply_allowed",
                    "issue377_experiment_plan_present",
                    "issue377_experiment_plan_id",
                    "issue377_experiment_plan_mode",
                    "issue377_experiment_plan_level_path",
                    "issue377_validation_skipped_levels",
                    "issue377_validation_skipped_reason",
                    "issue377_human_apply_level",
                    "issue377_human_apply_inside_engine",
                    "issue377_validation_level_apply_allowed",
                    "issue377_experiment_plan_required_gates",
                    "issue377_experiment_plan_budget_tokens",
                    "issue377_experiment_plan_stop_on_fail",
                    "issue377_experiment_plan_rollback_anchor",
                    "issue377_experiment_plan_raw_payload_present",
                    "issue377_experiment_plan_write_allowed",
                    "issue377_experiment_plan_applied",
                    "issue377_evidence_bundle_present",
                    "issue377_evidence_bundle_id",
                    "issue377_evidence_bundle_schema",
                    "issue377_evidence_bundle_metric",
                    "issue377_evidence_bundle_direction",
                    "issue377_evidence_bundle_pass_count",
                    "issue377_evidence_bundle_fail_count",
                    "issue377_evidence_bundle_command_label",
                    "issue377_evidence_bundle_refs_digest_only",
                    "issue377_evidence_bundle_raw_payload_present",
                    "issue377_evidence_bundle_write_allowed",
                    "issue377_evidence_bundle_applied",
                    "issue377_experiment_decision",
                    "issue377_experiment_decision_schema",
                    "issue377_experiment_decision_reason",
                    "issue377_experiment_decision_evidence_bundle_id",
                    "issue377_experiment_decision_target",
                    "issue377_experiment_decision_manual_approval_required",
                    "issue377_experiment_decision_apply_allowed",
                    "issue377_experiment_runner_allowed",
                    "issue377_experiment_apply_allowed",
                    "issue377_mutation_candidate_emitter_present",
                    "issue377_mutation_candidate_emitter_id",
                    "issue377_mutation_candidate_id",
                    "issue377_mutation_candidate_evidence_digest",
                    "issue377_mutation_candidate_rollback_anchor",
                    "issue377_mutation_candidate_requested_write_scope",
                    "issue377_mutation_candidate_kind",
                    "issue377_mutation_candidate_preview_only",
                    "issue377_mutation_candidate_refs_digest_only",
                    "issue377_mutation_candidate_writer_gate_preflight",
                    "issue377_mutation_candidate_write_allowed",
                    "issue377_mutation_candidate_applied",
                    "issue377_mutation_candidate_apply_allowed",
                    "issue377_mutation_candidate_manual_review_required",
                    "issue377_candidate_emitter_lane_coverage",
                    "issue377_candidate_emitter_kind_coverage",
                    "issue377_candidate_emitter_coverage_count",
                    "issue377_candidate_emitter_all_preview_only",
                    "issue377_candidate_emitter_all_write_allowed",
                    "issue377_candidate_emitter_all_apply_allowed",
                    "issue377_candidate_emitter_all_manual_review_required",
                    "issue377_candidate_emitter_durable_preflight_owner",
                    "issue377_candidate_emitter_writer_gate_bypass_allowed",
                    "issue377_candidate_emitter_direct_durable_write_allowed",
                    "issue377_candidate_emitter_ready_for_explicit_apply",
                    "issue377_related_issue_refs",
                    "issue377_related_issue_scope_map",
                    "issue377_related_issue_owner_scope",
                    "issue377_related_issue_non_duplicate_count",
                    "issue377_related_issue_all_non_duplicate",
                    "issue377_related_issue_apply_allowed",
                    "issue377_clean_room_reference_mode",
                    "issue377_external_code_copied",
                    "issue377_external_prompt_or_schema_copied",
                    "issue377_restricted_license_material_copied",
                    "issue377_license_provenance_posture",
                    "issue377_clean_room_apply_allowed",
                    "issue377_manual_approval_binding_present",
                    "issue377_manual_approval_candidate_id",
                    "issue377_manual_approval_evidence_digest",
                    "issue377_manual_approval_rollback_anchor",
                    "issue377_manual_approval_requested_write_scope",
                    "issue377_manual_approval_ref",
                    "issue377_manual_approval_expiration",
                    "issue377_manual_approval_apply_allowed",
                    "issue377_manual_approval_applied",
                ],
            )?;
            problem_hypothesis = Some(line.to_owned());
        } else {
            return Err(format!(
                "{}:{} expected issue30 context evidence row",
                path.display(),
                index + 1
            ));
        }
    }
    let entry_chain = required_state(&entry_chain, path, "issue30 entry chain row")?;
    let problem_hypothesis =
        required_state(&problem_hypothesis, path, "issue377 problem hypothesis row")?;
    let positive_context_loop_ready =
        issue30_positive_context_loop_ready(path, entry_chain, problem_hypothesis)?;
    let control_expression_gate_ready = issue243_control_expression_gate_ready(path, entry_chain)?;
    let dna_candidate_ledger_packet_proof =
        issue4_dna_candidate_ledger_packet_proof(path, entry_chain)?;
    Ok(format!(
        "{entry_chain}\n{problem_hypothesis}\n{positive_context_loop_ready} {control_expression_gate_ready} {dna_candidate_ledger_packet_proof} issue30_context_source=issue30_context_input"
    ))
}

fn issue4_dna_candidate_ledger_packet_proof(
    path: &Path,
    entry_chain: &str,
) -> Result<String, String> {
    let records = release_field(entry_chain, "issue4_dna_candidate_ledger_records")
        .unwrap_or_default()
        .parse::<usize>()
        .map_err(|_| {
            format!(
                "{} issue4_dna_candidate_ledger_records must be numeric",
                path.display()
            )
        })?;
    let candidate_count = release_field(entry_chain, "issue4_dna_candidate_ledger_candidate_count")
        .unwrap_or_default()
        .parse::<usize>()
        .map_err(|_| {
            format!(
                "{} issue4_dna_candidate_ledger_candidate_count must be numeric",
                path.display()
            )
        })?;
    let derived = release_field(entry_chain, "issue4_dna_candidate_ledger_present") == Some("true")
        && release_field(entry_chain, "issue4_dna_candidate_ledger_schema")
            == Some("dna_evolution_candidate_ledger_v1")
        && records > 0
        && records == candidate_count
        && release_field(entry_chain, "issue4_dna_candidate_ledger_candidate_only") == Some("true")
        && release_field(entry_chain, "issue4_dna_candidate_ledger_digest")
            .is_some_and(|value| value.starts_with("redaction-digest:"))
        && release_field(
            entry_chain,
            "issue4_dna_candidate_ledger_raw_records_allowed",
        ) == Some("false")
        && release_field(entry_chain, "issue4_dna_candidate_ledger_write_allowed") == Some("false")
        && release_field(entry_chain, "issue4_dna_candidate_ledger_applied") == Some("false")
        && release_field(entry_chain, "issue4_dna_candidate_ledger_preview_source")
            == Some("entry_chain_dna_evolution_controller");
    if let Some(raw_value) = release_field(entry_chain, "issue4_dna_candidate_ledger_packet_proof")
    {
        if raw_value != derived.to_string() {
            return Err(format!(
                "{} issue4_dna_candidate_ledger_packet_proof conflicts with candidate-ledger fields",
                path.display()
            ));
        }
        Ok(
            "issue4_dna_candidate_ledger_packet_proof_source=issue30_context_input_derived"
                .to_owned(),
        )
    } else {
        Ok(format!(
            "issue4_dna_candidate_ledger_packet_proof={derived} issue4_dna_candidate_ledger_packet_proof_source=issue30_context_input_derived"
        ))
    }
}

fn issue243_control_expression_gate_ready(
    path: &Path,
    entry_chain: &str,
) -> Result<String, String> {
    let knobs = release_field(entry_chain, "issue243_active_control_knobs").unwrap_or_default();
    let has_knobs = [
        "routing",
        "context_anchor",
        "suppression",
        "checkpoint",
        "memory_maintenance",
    ]
    .iter()
    .all(|knob| knobs.split('|').any(|candidate| candidate == *knob));
    let derived = has_knobs
        && release_field(entry_chain, "issue243_evidence_digest")
            .is_some_and(|value| value.starts_with("redaction-digest:"))
        && release_field(entry_chain, "issue243_policy_version")
            == Some("control_expression_gate_v1")
        && release_field(entry_chain, "issue243_decision_reason")
            == Some("no_weight_runtime_control_preview")
        && release_field(entry_chain, "issue243_control_expression_profile_selected") == Some("1")
        && release_field(entry_chain, "issue243_context_anchor_promoted") == Some("1")
        && release_field(entry_chain, "issue243_suppression_gate_triggered") == Some("1")
        && release_field(entry_chain, "issue243_checkpoint_repair_requested") == Some("1")
        && release_field(entry_chain, "issue243_checkpoint_rejected") == Some("1")
        && release_field(entry_chain, "issue243_memory_refresh_candidate") == Some("1")
        && release_field(entry_chain, "issue243_memory_tombstone_candidate") == Some("1")
        && release_field(entry_chain, "issue243_control_expression_preview_admission") == Some("1")
        && release_field(entry_chain, "issue243_write_allowed") == Some("false")
        && release_field(entry_chain, "issue243_applied") == Some("false")
        && release_field(entry_chain, "issue243_operator_approval_required") == Some("true");
    if let Some(raw_value) = release_field(entry_chain, "issue243_control_expression_gate_ready") {
        if raw_value != derived.to_string() {
            return Err(format!(
                "{} issue243_control_expression_gate_ready conflicts with control expression fields",
                path.display()
            ));
        }
        Ok(
            "issue243_control_expression_gate_ready_source=issue30_context_input_derived"
                .to_owned(),
        )
    } else {
        Ok(format!(
            "issue243_control_expression_gate_ready={derived} issue243_control_expression_gate_ready_source=issue30_context_input_derived"
        ))
    }
}

fn issue243_fixture_matrix_statement(path: &Path) -> Result<String, String> {
    const CASES: [(&str, &[(&str, &str)]); 9] = [
        (
            "no_weight_control_accepted",
            &[
                ("issue243_no_weight_control_accepted", "true"),
                ("issue243_control_expression_profile_selected", "1"),
            ],
        ),
        (
            "adapter_handoff_held",
            &[("issue243_adapter_handoff_held", "true")],
        ),
        (
            "long_context_anchor_promoted",
            &[("issue243_context_anchor_promoted", "1")],
        ),
        (
            "polluted_candidate_suppressed",
            &[("issue243_suppression_gate_triggered", "1")],
        ),
        (
            "verifier_checkpoint_failure",
            &[("issue243_checkpoint_rejected", "1")],
        ),
        (
            "successful_repair_retry",
            &[
                ("issue243_checkpoint_repair_requested", "1"),
                ("issue243_repair_retry_succeeded", "true"),
            ],
        ),
        (
            "memory_refresh_candidate",
            &[("issue243_memory_refresh_candidate", "1")],
        ),
        (
            "tombstone_held_for_approval",
            &[
                ("issue243_memory_tombstone_candidate", "1"),
                ("issue243_tombstone_held_for_approval", "true"),
            ],
        ),
        (
            "writer_gate_denial",
            &[
                ("issue243_write_allowed", "false"),
                ("issue243_applied", "false"),
            ],
        ),
    ];

    let raw = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let mut seen: Vec<String> = Vec::new();
    for (index, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fixture = required_issue_field(path, index, line, "fixture")?;
        let Some((_, required_fields)) = CASES
            .iter()
            .find(|(expected_fixture, _)| *expected_fixture == fixture.as_str())
        else {
            return Err(format!(
                "{}:{} unknown issue243 fixture {fixture}",
                path.display(),
                index + 1
            ));
        };
        if seen.iter().any(|value| value == &fixture) {
            return Err(format!(
                "{}:{} duplicate issue243 fixture {fixture}",
                path.display(),
                index + 1
            ));
        }
        seen.push(fixture.to_owned());
        require_issue_fields(
            path,
            index,
            line,
            &[
                "fixture",
                "issue243_active_control_knobs",
                "issue243_evidence_digest",
                "issue243_policy_version",
                "issue243_decision_reason",
                "issue243_write_allowed",
                "issue243_applied",
                "issue243_operator_approval_required",
            ],
        )?;
        if !release_field(line, "issue243_evidence_digest")
            .is_some_and(|value| value.starts_with("redaction-digest:"))
        {
            return Err(format!(
                "{}:{} issue243_evidence_digest must be redaction-digest",
                path.display(),
                index + 1
            ));
        }
        if release_field(line, "issue243_policy_version") != Some("control_expression_gate_v1") {
            return Err(format!(
                "{}:{} issue243_policy_version must be control_expression_gate_v1",
                path.display(),
                index + 1
            ));
        }
        if release_field(line, "issue243_decision_reason")
            != Some("no_weight_runtime_control_preview")
        {
            return Err(format!(
                "{}:{} issue243_decision_reason must be no_weight_runtime_control_preview",
                path.display(),
                index + 1
            ));
        }
        if release_field(line, "issue243_write_allowed") != Some("false")
            || release_field(line, "issue243_applied") != Some("false")
            || release_field(line, "issue243_operator_approval_required") != Some("true")
        {
            return Err(format!(
                "{}:{} issue243 fixture must stay preview-only with operator approval",
                path.display(),
                index + 1
            ));
        }
        for knob in [
            "routing",
            "context_anchor",
            "suppression",
            "checkpoint",
            "memory_maintenance",
        ] {
            if !release_field(line, "issue243_active_control_knobs")
                .unwrap_or_default()
                .split('|')
                .any(|candidate| candidate == knob)
            {
                return Err(format!(
                    "{}:{} issue243_active_control_knobs missing {knob}",
                    path.display(),
                    index + 1
                ));
            }
        }
        for (field, expected) in *required_fields {
            if release_field(line, field) != Some(*expected) {
                return Err(format!(
                    "{}:{} {field} must be {expected}",
                    path.display(),
                    index + 1
                ));
            }
        }
    }

    for (fixture, _) in CASES {
        if !seen.iter().any(|value| value.as_str() == fixture) {
            return Err(format!(
                "{} missing issue243 fixture {fixture}",
                path.display()
            ));
        }
    }

    Ok(format!(
        "issue243_control_fixture_matrix_ready=true issue243_control_fixture_matrix_cases={} issue243_control_fixture_matrix_source=issue243_fixture_matrix_input",
        seen.join("|")
    ))
}

fn issue30_positive_context_loop_ready(
    path: &Path,
    entry_chain: &str,
    problem_hypothesis: &str,
) -> Result<String, String> {
    let issue377_problem_hypothesis_ready =
        issue377_problem_hypothesis_ready(path, problem_hypothesis)?;
    let issue385_body_state_marker_ready = issue385_body_state_marker_ready(path, entry_chain)?;
    let issue375_reasoning_frame_ready = issue375_reasoning_frame_ready(path, entry_chain)?;
    let issue493_tool_organ_ready = issue493_tool_organ_registry_ready(path, entry_chain)?;
    let issue379_primitive_ready = issue379_zero_beat_primitive_ready(path, entry_chain)?;
    if release_field(entry_chain, "issue501_apoptosis_required") == Some("true") {
        for field in [
            "issue501_new_external_call_allowed",
            "issue501_new_file_write_allowed",
            "issue501_new_memory_write_allowed",
            "issue501_new_adaptive_state_write_allowed",
            "issue501_memory_promotion_allowed",
            "issue501_genome_mutation_allowed",
            "issue501_preview_side_effect_allowed",
        ] {
            if release_field(entry_chain, field) == Some("true") {
                return Err(format!(
                    "{} issue501 apoptosis_required conflicts with {field}=true",
                    path.display()
                ));
            }
        }
    }
    if release_field(entry_chain, "issue502_raw_payload_present") == Some("true") {
        return Err(format!(
            "{} issue502 pheromone blackboard conflicts with raw payload presence",
            path.display()
        ));
    }
    if release_field(entry_chain, "issue502_side_effect_allowed") == Some("true") {
        return Err(format!(
            "{} issue502 pheromone blackboard conflicts with side-effect permission",
            path.display()
        ));
    }
    if release_field(entry_chain, "issue509_raw_evaluator_payload_present") == Some("true") {
        return Err(format!(
            "{} issue509 quorum sensing conflicts with raw evaluator payload presence",
            path.display()
        ));
    }
    if release_field(entry_chain, "issue509_writer_gate_bypass_allowed") == Some("true") {
        return Err(format!(
            "{} issue509 quorum sensing conflicts with writer-gate bypass",
            path.display()
        ));
    }
    if release_field(entry_chain, "issue509_apply_allowed") == Some("true")
        && (release_field(entry_chain, "issue509_quorum_reached") != Some("true")
            || release_field(entry_chain, "issue509_raw_evaluator_payload_present")
                != Some("false"))
    {
        return Err(format!(
            "{} issue509 apply_allowed conflicts with quorum/raw-payload fields",
            path.display()
        ));
    }
    if release_field(entry_chain, "issue509_conflict_count")
        .is_some_and(|value| value.parse::<usize>().is_ok_and(|count| count > 0))
        && release_field(entry_chain, "issue509_conflict_routes_to_repair") != Some("true")
    {
        return Err(format!(
            "{} issue509 conflicts must route to repair/review",
            path.display()
        ));
    }

    let derived = release_field(entry_chain, "issue30_environment_pressure_present")
        == Some("true")
        && release_field(entry_chain, "issue30_pollution_event_id")
            .is_some_and(|value| value.starts_with("redaction-digest:"))
        && issue385_body_state_marker_ready
        && issue375_reasoning_frame_ready
        && release_field(entry_chain, "issue30_backend_action")
            .is_some_and(|value| !value.is_empty() && value != "none")
        && issue379_primitive_ready
        && issue493_tool_organ_ready
        && release_field(entry_chain, "bio_epigenetic_expression_marker_present") == Some("true")
        && release_field(entry_chain, "bio_epigenetic_expression_marker_id")
            .is_some_and(|value| value.starts_with("redaction-digest:"))
        && release_field(entry_chain, "bio_mrna_cache_candidate_digest")
            .is_some_and(|value| value.starts_with("redaction-digest:"))
        && release_field(entry_chain, "bio_expression_cache_protocol") == Some("mrna_preview_v1")
        && release_field(entry_chain, "bio_expression_cache_key_digest")
            .is_some_and(|value| value.starts_with("redaction-digest:"))
        && release_field(entry_chain, "bio_hot_path_observation_window") == Some("100")
        && release_field(entry_chain, "bio_hot_path_min_success_rate") == Some("0.98")
        && release_field(entry_chain, "bio_gate_relaxation_allowed") == Some("false")
        && release_field(entry_chain, "bio_cache_materialization_allowed") == Some("false")
        && release_field(entry_chain, "bio_raw_payload_or_kv_cached") == Some("false")
        && release_field(entry_chain, "bio_negative_evidence_overrides") == Some("true")
        && release_field(entry_chain, "issue501_telomere_state_present") == Some("true")
        && release_field(entry_chain, "issue501_remaining_tokens") == Some("0")
        && release_field(entry_chain, "issue501_remaining_steps") == Some("0")
        && release_field(entry_chain, "issue501_remaining_messages") == Some("0")
        && release_field(entry_chain, "issue501_repair_streak_count") == Some("2")
        && release_field(entry_chain, "issue501_loop_risk_signal_count")
            .is_some_and(|value| value.parse::<usize>().is_ok_and(|count| count >= 2))
        && release_field(entry_chain, "issue501_senescent") == Some("true")
        && release_field(entry_chain, "issue501_apoptosis_required") == Some("true")
        && release_field(entry_chain, "issue501_new_external_call_allowed") == Some("false")
        && release_field(entry_chain, "issue501_new_file_write_allowed") == Some("false")
        && release_field(entry_chain, "issue501_new_memory_write_allowed") == Some("false")
        && release_field(entry_chain, "issue501_new_adaptive_state_write_allowed") == Some("false")
        && release_field(entry_chain, "issue501_memory_promotion_allowed") == Some("false")
        && release_field(entry_chain, "issue501_genome_mutation_allowed") == Some("false")
        && release_field(entry_chain, "issue501_takeover_packet_digest")
            .is_some_and(|value| value.starts_with("redaction-digest:"))
        && release_field(entry_chain, "issue501_rollback_anchor_digest")
            .is_some_and(|value| value.starts_with("redaction-digest:"))
        && release_field(entry_chain, "issue501_handoff_next_owner") == Some("scheduler")
        && release_field(entry_chain, "issue501_raw_payload_present") == Some("false")
        && release_field(entry_chain, "issue501_preview_side_effect_allowed") == Some("false")
        && release_field(entry_chain, "issue502_pheromone_blackboard_present") == Some("true")
        && release_field(entry_chain, "issue502_signal_count")
            .is_some_and(|value| value.parse::<usize>().is_ok_and(|count| count >= 2))
        && release_field(entry_chain, "issue502_ranked_action_count")
            .is_some_and(|value| value.parse::<usize>().is_ok_and(|count| count >= 1))
        && release_field(entry_chain, "issue502_top_signal_kind") == Some("repair_first")
        && release_field(entry_chain, "issue502_top_action") == Some("repair_review")
        && release_field(entry_chain, "issue502_blackboard_digest")
            .is_some_and(|value| value.starts_with("redaction-digest:"))
        && release_field(entry_chain, "issue502_source_digest")
            .is_some_and(|value| value.starts_with("redaction-digest:"))
        && release_field(entry_chain, "issue502_payload_digest")
            .is_some_and(|value| value.starts_with("redaction-digest:"))
        && release_field(entry_chain, "issue502_raw_payload_present") == Some("false")
        && release_field(entry_chain, "issue502_side_effect_allowed") == Some("false")
        && release_field(entry_chain, "issue502_ttl_decay_present") == Some("true")
        && release_field(entry_chain, "issue502_conflict_routes_to_repair") == Some("true")
        && release_field(entry_chain, "issue502_ranked_actions_from_state_only") == Some("true")
        && release_field(entry_chain, "issue509_quorum_sensing_present") == Some("true")
        && release_field(entry_chain, "issue509_decision_id")
            .is_some_and(|value| value.starts_with("redaction-digest:"))
        && release_field(entry_chain, "issue509_quorum_report_digest")
            .is_some_and(|value| value.starts_with("redaction-digest:"))
        && release_field(entry_chain, "issue509_risk_class") == Some("irreversible")
        && release_field(entry_chain, "issue509_required_quorum_milli") == Some("700")
        && release_field(entry_chain, "issue509_evaluator_count")
            .is_some_and(|value| value.parse::<usize>().is_ok_and(|count| count >= 2))
        && release_field(entry_chain, "issue509_independent_model_count")
            .is_some_and(|value| value.parse::<usize>().is_ok_and(|count| count >= 2))
        && release_field(entry_chain, "issue509_independent_lane_count")
            .is_some_and(|value| value.parse::<usize>().is_ok_and(|count| count >= 2))
        && release_field(entry_chain, "issue509_approve_signal_count")
            .is_some_and(|value| value.parse::<usize>().is_ok_and(|count| count >= 1))
        && release_field(entry_chain, "issue509_reject_signal_count")
            .is_some_and(|value| value.parse::<usize>().is_ok_and(|count| count >= 1))
        && release_field(entry_chain, "issue509_abstain_signal_count")
            .is_some_and(|value| value.parse::<usize>().is_ok())
        && release_field(entry_chain, "issue509_approval_concentration_milli")
            .is_some_and(|value| value.parse::<usize>().is_ok())
        && release_field(entry_chain, "issue509_conflict_count")
            .is_some_and(|value| value.parse::<usize>().is_ok_and(|count| count >= 1))
        && release_field(entry_chain, "issue509_quorum_reached") == Some("false")
        && release_field(entry_chain, "issue509_apply_allowed") == Some("false")
        && release_field(entry_chain, "issue509_raw_evaluator_payload_present") == Some("false")
        && release_field(entry_chain, "issue509_duplicate_sources_count_once") == Some("true")
        && release_field(entry_chain, "issue509_conflict_routes_to_repair") == Some("true")
        && release_field(entry_chain, "issue509_writer_gate_bypass_allowed") == Some("false")
        && issue377_problem_hypothesis_ready;
    if let Some(raw_value) = release_field(entry_chain, "issue30_positive_context_loop_ready")
        .or_else(|| release_field(problem_hypothesis, "issue30_positive_context_loop_ready"))
    {
        if raw_value != derived.to_string() {
            return Err(format!(
                "{} issue30_positive_context_loop_ready conflicts with context rows",
                path.display()
            ));
        }
        Ok("issue30_positive_context_loop_ready_source=issue30_context_input_derived".to_owned())
    } else {
        Ok(format!(
            "issue30_positive_context_loop_ready={derived} issue30_positive_context_loop_ready_source=issue30_context_input_derived"
        ))
    }
}

fn issue493_tool_organ_registry_ready(path: &Path, line: &str) -> Result<bool, String> {
    let registry_present = issue493_bool_field(path, line, "issue493_tool_organ_registry_present")?;
    let registry_id = issue493_required_field(path, line, "issue493_tool_organ_registry_id")?;
    let preview_only =
        issue493_bool_field(path, line, "issue493_tool_organ_registry_preview_only")?;
    let side_effect =
        issue493_required_field(path, line, "issue493_tool_organ_registry_side_effect")?;
    let apply_allowed =
        issue493_bool_field(path, line, "issue493_tool_organ_registry_apply_allowed")?;
    let capability_matrix_digest =
        issue493_required_field(path, line, "issue493_tool_organ_capability_matrix_digest")?;
    let preview_bundle_protocol =
        issue493_required_field(path, line, "issue493_preview_bundle_protocol")?;
    let preview_bundle_digest =
        issue493_required_field(path, line, "issue493_preview_bundle_digest")?;
    let preview_bundle_refs_digest_only =
        issue493_bool_field(path, line, "issue493_preview_bundle_refs_digest_only")?;
    let preview_bundle_raw_artifacts_allowed =
        issue493_bool_field(path, line, "issue493_preview_bundle_raw_artifacts_allowed")?;
    let tool_install_allowed = issue493_bool_field(path, line, "issue493_tool_install_allowed")?;
    let tool_execution_allowed =
        issue493_bool_field(path, line, "issue493_tool_execution_allowed")?;
    let registry_digest = registry_id.starts_with("redaction-digest:");
    let capability_matrix_digest_only = capability_matrix_digest.starts_with("redaction-digest:");
    let preview_bundle_digest_only = preview_bundle_digest.starts_with("redaction-digest:");

    if preview_only && !registry_present {
        return Err(format!(
            "{} issue493 ToolOrganRegistry preview conflicts with missing registry",
            path.display()
        ));
    }
    if registry_present && !registry_digest {
        return Err(format!(
            "{} issue493 ToolOrganRegistry must use digest-only registry id",
            path.display()
        ));
    }
    if registry_present && !preview_only {
        return Err(format!(
            "{} issue493 ToolOrganRegistry must remain preview-only",
            path.display()
        ));
    }
    if registry_present && side_effect != "read_only" {
        return Err(format!(
            "{} issue493 ToolOrganRegistry must remain read-only",
            path.display()
        ));
    }
    if registry_present && apply_allowed {
        return Err(format!(
            "{} issue493 ToolOrganRegistry conflicts with apply permission",
            path.display()
        ));
    }
    if registry_present && !capability_matrix_digest_only {
        return Err(format!(
            "{} issue493 capability matrix must use digest-only evidence",
            path.display()
        ));
    }
    if registry_present && preview_bundle_protocol != "bundle_v1" {
        return Err(format!(
            "{} issue493 preview bundle protocol is not bounded",
            path.display()
        ));
    }
    if registry_present && !preview_bundle_digest_only {
        return Err(format!(
            "{} issue493 preview bundle must use digest-only evidence",
            path.display()
        ));
    }
    if registry_present && !preview_bundle_refs_digest_only {
        return Err(format!(
            "{} issue493 preview bundle refs must remain digest-only",
            path.display()
        ));
    }
    if registry_present && preview_bundle_raw_artifacts_allowed {
        return Err(format!(
            "{} issue493 preview bundle conflicts with raw artifact permission",
            path.display()
        ));
    }
    if registry_present && tool_install_allowed {
        return Err(format!(
            "{} issue493 ToolOrganRegistry conflicts with install permission",
            path.display()
        ));
    }
    if registry_present && tool_execution_allowed {
        return Err(format!(
            "{} issue493 ToolOrganRegistry conflicts with execution permission",
            path.display()
        ));
    }

    Ok(registry_present
        && registry_digest
        && preview_only
        && side_effect == "read_only"
        && !apply_allowed
        && capability_matrix_digest_only
        && preview_bundle_protocol == "bundle_v1"
        && preview_bundle_digest_only
        && preview_bundle_refs_digest_only
        && !preview_bundle_raw_artifacts_allowed
        && !tool_install_allowed
        && !tool_execution_allowed)
}

fn issue493_required_field<'a>(path: &Path, line: &'a str, field: &str) -> Result<&'a str, String> {
    release_field(line, field)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{} missing {field}", path.display()))
}

fn issue493_bool_field(path: &Path, line: &str, field: &str) -> Result<bool, String> {
    match issue493_required_field(path, line, field)? {
        "true" => Ok(true),
        "false" => Ok(false),
        value => Err(format!(
            "{} {field} is not boolean: {value}",
            path.display()
        )),
    }
}

fn issue375_reasoning_frame_ready(path: &Path, line: &str) -> Result<bool, String> {
    let pre_reasoning_present =
        issue375_bool_field(path, line, "issue375_pre_reasoning_genome_isa_present")?;
    let frame_id = issue375_required_field(path, line, "issue375_reasoning_frame_id")?;
    let environment_signals_present = issue375_bool_field(
        path,
        line,
        "issue375_reasoning_frame_environment_signals_present",
    )?;
    let allowed_observations =
        issue375_required_field(path, line, "issue375_reasoning_frame_allowed_observations")?;
    let action_vocab =
        issue375_required_field(path, line, "issue375_reasoning_frame_action_vocab")?;
    let suppressed_capabilities = issue375_required_field(
        path,
        line,
        "issue375_reasoning_frame_suppressed_capabilities",
    )?;
    let risk_limits = issue375_required_field(path, line, "issue375_reasoning_frame_risk_limits")?;
    let side_effect = issue375_required_field(path, line, "issue375_expression_vm_side_effect")?;
    let apply_allowed = issue375_bool_field(path, line, "issue375_genome_isa_apply_allowed")?;
    let frame_digest = frame_id.starts_with("redaction-digest:");

    if pre_reasoning_present && !frame_digest {
        return Err(format!(
            "{} issue375 ReasoningFrame must use digest-only frame id",
            path.display()
        ));
    }
    if pre_reasoning_present && !environment_signals_present {
        return Err(format!(
            "{} issue375 ReasoningFrame conflicts with missing environment signals",
            path.display()
        ));
    }
    if pre_reasoning_present && allowed_observations != "repo_issue_terminal_runtime_state" {
        return Err(format!(
            "{} issue375 ReasoningFrame allowed observations are not bounded",
            path.display()
        ));
    }
    if pre_reasoning_present
        && action_vocab != "observe_inspect_compare_summarize_verify_quarantine"
    {
        return Err(format!(
            "{} issue375 ReasoningFrame action vocabulary is not bounded",
            path.display()
        ));
    }
    if pre_reasoning_present
        && suppressed_capabilities != "write_process_browser_network_memory_genome_runtime"
    {
        return Err(format!(
            "{} issue375 ReasoningFrame suppressed capabilities are incomplete",
            path.display()
        ));
    }
    if pre_reasoning_present && risk_limits != "preview_only_digest_only" {
        return Err(format!(
            "{} issue375 ReasoningFrame risk limits are not preview/digest-only",
            path.display()
        ));
    }
    if pre_reasoning_present && side_effect != "read_only" {
        return Err(format!(
            "{} issue375 ExpressionVM must remain read-only",
            path.display()
        ));
    }
    if pre_reasoning_present && apply_allowed {
        return Err(format!(
            "{} issue375 Genome ISA preview conflicts with apply permission",
            path.display()
        ));
    }

    Ok(pre_reasoning_present
        && frame_digest
        && environment_signals_present
        && allowed_observations == "repo_issue_terminal_runtime_state"
        && action_vocab == "observe_inspect_compare_summarize_verify_quarantine"
        && suppressed_capabilities == "write_process_browser_network_memory_genome_runtime"
        && risk_limits == "preview_only_digest_only"
        && side_effect == "read_only"
        && !apply_allowed)
}

fn issue375_required_field<'a>(path: &Path, line: &'a str, field: &str) -> Result<&'a str, String> {
    release_field(line, field)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{} missing {field}", path.display()))
}

fn issue375_bool_field(path: &Path, line: &str, field: &str) -> Result<bool, String> {
    match issue375_required_field(path, line, field)? {
        "true" => Ok(true),
        "false" => Ok(false),
        value => Err(format!(
            "{} {field} is not boolean: {value}",
            path.display()
        )),
    }
}

fn issue385_body_state_marker_ready(path: &Path, line: &str) -> Result<bool, String> {
    let body_present = issue385_bool_field(path, line, "issue385_self_ontology_body_present")?;
    let body_state_id = issue385_required_field(path, line, "issue385_body_state_id")?;
    let marker_present =
        issue385_bool_field(path, line, "issue385_pheromone_signal_marker_present")?;
    let marker_id = issue385_required_field(path, line, "issue385_pheromone_signal_marker_id")?;
    let surface = issue385_required_field(path, line, "issue385_pheromone_signal_surface")?;
    let digest_gate_allowed =
        issue385_bool_field(path, line, "issue385_pheromone_signal_digest_gate_allowed")?;
    let preview_only = issue385_bool_field(path, line, "issue385_pheromone_signal_preview_only")?;
    let body_digest = body_state_id.starts_with("redaction-digest:");
    let marker_digest = marker_id.starts_with("redaction-digest:");

    if !body_digest && (body_present || preview_only) {
        return Err(format!(
            "{} issue385 SelfOntology.body must use digest-only body_state_id",
            path.display()
        ));
    }
    if !marker_digest && (marker_present || preview_only) {
        return Err(format!(
            "{} issue385 pheromone signal marker must use digest-only marker id",
            path.display()
        ));
    }
    if preview_only && !body_present {
        return Err(format!(
            "{} issue385 preview marker conflicts with missing SelfOntology.body",
            path.display()
        ));
    }
    if preview_only && !marker_present {
        return Err(format!(
            "{} issue385 preview marker conflicts with missing pheromone signal marker",
            path.display()
        ));
    }
    if preview_only && surface != "digest_marker" {
        return Err(format!(
            "{} issue385 preview marker requires digest_marker surface",
            path.display()
        ));
    }
    if preview_only && !digest_gate_allowed {
        return Err(format!(
            "{} issue385 preview marker conflicts with digest gate",
            path.display()
        ));
    }
    if digest_gate_allowed && surface != "digest_marker" {
        return Err(format!(
            "{} issue385 digest gate conflicts with signal surface",
            path.display()
        ));
    }

    Ok(body_present
        && body_digest
        && marker_present
        && marker_digest
        && surface == "digest_marker"
        && digest_gate_allowed
        && preview_only)
}

fn issue385_required_field<'a>(path: &Path, line: &'a str, field: &str) -> Result<&'a str, String> {
    release_field(line, field)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{} missing {field}", path.display()))
}

fn issue385_bool_field(path: &Path, line: &str, field: &str) -> Result<bool, String> {
    match issue385_required_field(path, line, field)? {
        "true" => Ok(true),
        "false" => Ok(false),
        value => Err(format!(
            "{} {field} is not boolean: {value}",
            path.display()
        )),
    }
}

fn issue379_zero_beat_primitive_ready(path: &Path, line: &str) -> Result<bool, String> {
    let control_preview =
        issue379_bool_field(path, line, "issue379_control_candidate_preview_only")?;
    let action_mask_preview =
        issue379_bool_field(path, line, "issue379_action_vocab_mask_preview")?;
    let saliency_preview =
        issue379_bool_field(path, line, "issue379_signal_saliency_bias_preview")?;
    let primitive_present =
        issue379_bool_field(path, line, "issue379_zero_beat_primitive_decision_present")?;
    let authority = issue379_required_field(path, line, "issue379_primitive_authority")?;
    let side_effect = issue379_required_field(path, line, "issue379_primitive_side_effect")?;
    let reversibility = issue379_required_field(path, line, "issue379_primitive_reversibility")?;
    let evidence = issue379_required_field(path, line, "issue379_primitive_evidence")?;
    let uncertainty = issue379_required_field(path, line, "issue379_primitive_uncertainty")?;
    let attention = issue379_required_field(path, line, "issue379_primitive_attention")?;
    let output = issue379_required_field(path, line, "issue379_zero_beat_output")?;
    let generation_bias_apply_allowed =
        issue379_bool_field(path, line, "issue379_generation_bias_apply_allowed")?;

    let dimensions_ready = primitive_present
        && authority == "preview_only"
        && side_effect == "read_only"
        && reversibility == "rollback_required"
        && evidence == "digest_only"
        && uncertainty == "hold_on_gap"
        && attention == "focus_or_mask_preview"
        && !generation_bias_apply_allowed;
    let ready = dimensions_ready
        && control_preview
        && action_mask_preview
        && saliency_preview
        && output == "action_vocab_mask_and_signal_saliency_bias";

    if !dimensions_ready
        && (control_preview
            || action_mask_preview
            || saliency_preview
            || output == "action_vocab_mask_and_signal_saliency_bias")
    {
        return Err(format!(
            "{} issue379 zero-beat output conflicts with primitive dimensions",
            path.display()
        ));
    }
    if dimensions_ready && output != "action_vocab_mask_and_signal_saliency_bias" {
        return Err(format!(
            "{} issue379 primitive dimensions conflict with zero-beat output",
            path.display()
        ));
    }
    Ok(ready)
}

fn issue379_required_field<'a>(path: &Path, line: &'a str, field: &str) -> Result<&'a str, String> {
    release_field(line, field)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{} missing {field}", path.display()))
}

fn issue379_bool_field(path: &Path, line: &str, field: &str) -> Result<bool, String> {
    match issue379_required_field(path, line, field)? {
        "true" => Ok(true),
        "false" => Ok(false),
        value => Err(format!(
            "{} {field} is not boolean: {value}",
            path.display()
        )),
    }
}

fn issue377_predicament_fields(
    path: &Path,
    line: &str,
) -> Result<(i32, usize, usize, usize), String> {
    let progress_delta = issue377_i32_field(path, line, "issue377_predicament_progress_delta")?;
    let repeat_count = issue377_usize_field(path, line, "issue377_predicament_repeat_count")?;
    let evidence_gap_count =
        issue377_usize_field(path, line, "issue377_predicament_evidence_gap_count")?;
    let action_novelty = issue377_usize_field(path, line, "issue377_predicament_action_novelty")?;
    Ok((
        progress_delta,
        repeat_count,
        evidence_gap_count,
        action_novelty,
    ))
}

fn issue377_predicament_best_next_state(path: &Path, line: &str) -> Result<&'static str, String> {
    let (progress_delta, repeat_count, evidence_gap_count, action_novelty) =
        issue377_predicament_fields(path, line)?;
    if evidence_gap_count > 0 {
        Ok("hold_for_evidence")
    } else if progress_delta == 0 && repeat_count >= 2 && action_novelty == 0 {
        Ok("problem_finding_preview")
    } else {
        Ok("watch")
    }
}

fn issue377_predicament_signal_ready(path: &Path, line: &str) -> Result<bool, String> {
    let (progress_delta, repeat_count, evidence_gap_count, action_novelty) =
        issue377_predicament_fields(path, line)?;
    let derived_stuck = progress_delta == 0 && repeat_count >= 2 && action_novelty == 0;

    if release_field(line, "issue377_predicament_stuck") != Some(derived_stuck.to_string().as_str())
    {
        return Err(format!(
            "{} issue377_predicament_stuck conflicts with predicament fields",
            path.display()
        ));
    }

    Ok(derived_stuck && evidence_gap_count == 0)
}

fn issue377_problem_hypothesis_ready(path: &Path, line: &str) -> Result<bool, String> {
    let problem_present = issue377_bool_field(path, line, "issue377_problem_finding_present")?;
    let problem_id = issue377_required_field(path, line, "issue377_problem_finding_id")?;
    let problem_kind = issue377_required_field(path, line, "issue377_problem_finding_kind")?;
    let problem_severity =
        issue377_required_field(path, line, "issue377_problem_finding_severity")?;
    let problem_confidence_milli =
        issue377_usize_field(path, line, "issue377_problem_finding_confidence_milli")?;
    let problem_evidence_digest =
        issue377_required_field(path, line, "issue377_problem_finding_evidence_digest")?;
    let problem_source_digest =
        issue377_required_field(path, line, "issue377_problem_finding_source_digest")?;
    let problem_affected_surface =
        issue377_required_field(path, line, "issue377_problem_finding_affected_surface")?;
    let problem_next_step =
        issue377_required_field(path, line, "issue377_problem_finding_next_step")?;
    let problem_raw_payload_present =
        issue377_bool_field(path, line, "issue377_problem_finding_raw_payload_present")?;
    let self_observation_present =
        issue377_bool_field(path, line, "issue377_self_observation_present")?;
    let self_observation_id = issue377_required_field(path, line, "issue377_self_observation_id")?;
    let self_observation_schema =
        issue377_required_field(path, line, "issue377_self_observation_schema")?;
    let self_observation_signal_source =
        issue377_required_field(path, line, "issue377_self_observation_signal_source")?;
    let self_observation_source_digest =
        issue377_required_field(path, line, "issue377_self_observation_source_digest")?;
    let self_observation_window =
        issue377_required_field(path, line, "issue377_self_observation_window")?;
    let self_observation_current_truth_digest =
        issue377_required_field(path, line, "issue377_self_observation_current_truth_digest")?;
    let self_observation_digest_only =
        issue377_bool_field(path, line, "issue377_self_observation_digest_only")?;
    let self_observation_raw_payload_present =
        issue377_bool_field(path, line, "issue377_self_observation_raw_payload_present")?;
    let self_observation_write_allowed =
        issue377_bool_field(path, line, "issue377_self_observation_write_allowed")?;
    let self_observation_applied =
        issue377_bool_field(path, line, "issue377_self_observation_applied")?;
    let self_model_present = issue377_bool_field(path, line, "issue377_self_model_present")?;
    let self_model_id = issue377_required_field(path, line, "issue377_self_model_id")?;
    let self_model_schema = issue377_required_field(path, line, "issue377_self_model_schema")?;
    let self_model_scope = issue377_required_field(path, line, "issue377_self_model_scope")?;
    let self_model_claims_consciousness =
        issue377_bool_field(path, line, "issue377_self_model_claims_consciousness")?;
    let self_model_digest_only =
        issue377_bool_field(path, line, "issue377_self_model_digest_only")?;
    let self_model_raw_payload_present =
        issue377_bool_field(path, line, "issue377_self_model_raw_payload_present")?;
    let self_model_write_allowed =
        issue377_bool_field(path, line, "issue377_self_model_write_allowed")?;
    let self_model_applied = issue377_bool_field(path, line, "issue377_self_model_applied")?;
    let hypothesis_present =
        issue377_bool_field(path, line, "issue377_hypothesis_candidate_present")?;
    let hypothesis_id = issue377_required_field(path, line, "issue377_hypothesis_candidate_id")?;
    let hypothesis_kind =
        issue377_required_field(path, line, "issue377_hypothesis_candidate_kind")?;
    let hypothesis_status =
        issue377_required_field(path, line, "issue377_hypothesis_candidate_status")?;
    let hypothesis_target_surface =
        issue377_required_field(path, line, "issue377_hypothesis_candidate_target_surface")?;
    let hypothesis_expected_metric =
        issue377_required_field(path, line, "issue377_hypothesis_candidate_expected_metric")?;
    let hypothesis_expected_direction = issue377_required_field(
        path,
        line,
        "issue377_hypothesis_candidate_expected_direction",
    )?;
    let hypothesis_required_gates =
        issue377_required_field(path, line, "issue377_hypothesis_candidate_required_gates")?;
    let hypothesis_rollback_anchor =
        issue377_required_field(path, line, "issue377_hypothesis_candidate_rollback_anchor")?;
    let hypothesis_raw_payload_present = issue377_bool_field(
        path,
        line,
        "issue377_hypothesis_candidate_raw_payload_present",
    )?;
    let hypothesis_write_allowed =
        issue377_bool_field(path, line, "issue377_hypothesis_candidate_write_allowed")?;
    let hypothesis_applied =
        issue377_bool_field(path, line, "issue377_hypothesis_candidate_applied")?;
    let hypothesis_operator_approval_required = issue377_bool_field(
        path,
        line,
        "issue377_hypothesis_candidate_operator_approval_required",
    )?;
    let link_id = issue377_required_field(path, line, "issue377_problem_hypothesis_link")?;
    let admission_decision = issue377_required_field(path, line, "issue377_admission_decision")?;
    let lexicographic_admission_present =
        issue377_bool_field(path, line, "issue377_lexicographic_admission_present")?;
    let lexicographic_admission_order =
        issue377_required_field(path, line, "issue377_lexicographic_admission_order")?;
    let user_intent_preserved = issue377_bool_field(path, line, "issue377_user_intent_preserved")?;
    let safety_gate_passed = issue377_bool_field(path, line, "issue377_safety_gate_passed")?;
    let digest_only_evidence_gate_passed =
        issue377_bool_field(path, line, "issue377_digest_only_evidence_gate_passed")?;
    let rollback_anchor_gate_passed =
        issue377_bool_field(path, line, "issue377_rollback_anchor_gate_passed")?;
    let _quality_delta_milli = issue377_i32_field(path, line, "issue377_quality_delta_milli")?;
    let _cost_delta_milli = issue377_i32_field(path, line, "issue377_cost_delta_milli")?;
    let _latency_delta_milli = issue377_i32_field(path, line, "issue377_latency_delta_milli")?;
    let performance_tiebreaker_only =
        issue377_bool_field(path, line, "issue377_performance_tiebreaker_only")?;
    let hard_gate_failure_action =
        issue377_required_field(path, line, "issue377_hard_gate_failure_action")?;
    let risk_override_action =
        issue377_required_field(path, line, "issue377_risk_override_action")?;
    let negative_evidence_count =
        issue377_usize_field(path, line, "issue377_negative_evidence_count")?;
    let privacy_risk = issue377_required_field(path, line, "issue377_privacy_risk")?;
    let license_risk = issue377_required_field(path, line, "issue377_license_risk")?;
    let unsupported_capability_requested =
        issue377_bool_field(path, line, "issue377_unsupported_capability_requested")?;
    let unsafe_side_effect_allowed =
        issue377_bool_field(path, line, "issue377_unsafe_side_effect_allowed")?;
    let risk_override_clear = issue377_bool_field(path, line, "issue377_risk_override_clear")?;
    let lexicographic_admission_apply_allowed =
        issue377_bool_field(path, line, "issue377_lexicographic_admission_apply_allowed")?;
    let best_next_state = issue377_required_field(path, line, "issue377_best_next_state")?;
    let best_next_state_id = issue377_required_field(path, line, "issue377_best_next_state_id")?;
    let best_next_state_selected =
        issue377_bool_field(path, line, "issue377_best_next_state_selected")?;
    let predicament_present =
        issue377_bool_field(path, line, "issue377_predicament_signal_present")?;
    let predicament_id = issue377_required_field(path, line, "issue377_predicament_id")?;
    let predicament_ready = issue377_predicament_signal_ready(path, line)?;
    let self_trigger_stage = issue377_required_field(path, line, "issue377_self_trigger_stage")?;
    let apply_allowed = issue377_bool_field(path, line, "issue377_evolution_apply_allowed")?;
    let experiment_plan_present =
        issue377_bool_field(path, line, "issue377_experiment_plan_present")?;
    let experiment_plan_id = issue377_required_field(path, line, "issue377_experiment_plan_id")?;
    let experiment_plan_mode =
        issue377_required_field(path, line, "issue377_experiment_plan_mode")?;
    let experiment_plan_level_path =
        issue377_required_field(path, line, "issue377_experiment_plan_level_path")?;
    let validation_skipped_levels =
        issue377_required_field(path, line, "issue377_validation_skipped_levels")?;
    let validation_skipped_reason =
        issue377_required_field(path, line, "issue377_validation_skipped_reason")?;
    let human_apply_level = issue377_required_field(path, line, "issue377_human_apply_level")?;
    let human_apply_inside_engine =
        issue377_bool_field(path, line, "issue377_human_apply_inside_engine")?;
    let validation_level_apply_allowed =
        issue377_bool_field(path, line, "issue377_validation_level_apply_allowed")?;
    let experiment_plan_required_gates =
        issue377_required_field(path, line, "issue377_experiment_plan_required_gates")?;
    let experiment_plan_budget_tokens =
        issue377_usize_field(path, line, "issue377_experiment_plan_budget_tokens")?;
    let experiment_plan_stop_on_fail =
        issue377_bool_field(path, line, "issue377_experiment_plan_stop_on_fail")?;
    let experiment_plan_rollback_anchor =
        issue377_required_field(path, line, "issue377_experiment_plan_rollback_anchor")?;
    let experiment_plan_raw_payload_present =
        issue377_bool_field(path, line, "issue377_experiment_plan_raw_payload_present")?;
    let experiment_plan_write_allowed =
        issue377_bool_field(path, line, "issue377_experiment_plan_write_allowed")?;
    let experiment_plan_applied =
        issue377_bool_field(path, line, "issue377_experiment_plan_applied")?;
    let evidence_bundle_present =
        issue377_bool_field(path, line, "issue377_evidence_bundle_present")?;
    let evidence_bundle_id = issue377_required_field(path, line, "issue377_evidence_bundle_id")?;
    let evidence_bundle_schema =
        issue377_required_field(path, line, "issue377_evidence_bundle_schema")?;
    let evidence_bundle_metric =
        issue377_required_field(path, line, "issue377_evidence_bundle_metric")?;
    let evidence_bundle_direction =
        issue377_required_field(path, line, "issue377_evidence_bundle_direction")?;
    let evidence_bundle_pass_count =
        issue377_usize_field(path, line, "issue377_evidence_bundle_pass_count")?;
    let evidence_bundle_fail_count =
        issue377_usize_field(path, line, "issue377_evidence_bundle_fail_count")?;
    let evidence_bundle_command_label =
        issue377_required_field(path, line, "issue377_evidence_bundle_command_label")?;
    let evidence_bundle_refs_digest_only =
        issue377_bool_field(path, line, "issue377_evidence_bundle_refs_digest_only")?;
    let evidence_bundle_raw_payload_present =
        issue377_bool_field(path, line, "issue377_evidence_bundle_raw_payload_present")?;
    let evidence_bundle_write_allowed =
        issue377_bool_field(path, line, "issue377_evidence_bundle_write_allowed")?;
    let evidence_bundle_applied =
        issue377_bool_field(path, line, "issue377_evidence_bundle_applied")?;
    let experiment_decision = issue377_required_field(path, line, "issue377_experiment_decision")?;
    let experiment_decision_schema =
        issue377_required_field(path, line, "issue377_experiment_decision_schema")?;
    let experiment_decision_reason =
        issue377_required_field(path, line, "issue377_experiment_decision_reason")?;
    let experiment_decision_evidence_bundle_id = issue377_required_field(
        path,
        line,
        "issue377_experiment_decision_evidence_bundle_id",
    )?;
    let experiment_decision_target =
        issue377_required_field(path, line, "issue377_experiment_decision_target")?;
    let experiment_decision_manual_approval_required = issue377_bool_field(
        path,
        line,
        "issue377_experiment_decision_manual_approval_required",
    )?;
    let experiment_decision_apply_allowed =
        issue377_bool_field(path, line, "issue377_experiment_decision_apply_allowed")?;
    let experiment_runner_allowed =
        issue377_bool_field(path, line, "issue377_experiment_runner_allowed")?;
    let experiment_apply_allowed =
        issue377_bool_field(path, line, "issue377_experiment_apply_allowed")?;
    let mutation_candidate_emitter_present =
        issue377_bool_field(path, line, "issue377_mutation_candidate_emitter_present")?;
    let mutation_candidate_emitter_id =
        issue377_required_field(path, line, "issue377_mutation_candidate_emitter_id")?;
    let mutation_candidate_id =
        issue377_required_field(path, line, "issue377_mutation_candidate_id")?;
    let mutation_candidate_evidence_digest =
        issue377_required_field(path, line, "issue377_mutation_candidate_evidence_digest")?;
    let mutation_candidate_rollback_anchor =
        issue377_required_field(path, line, "issue377_mutation_candidate_rollback_anchor")?;
    let mutation_candidate_requested_write_scope = issue377_required_field(
        path,
        line,
        "issue377_mutation_candidate_requested_write_scope",
    )?;
    let mutation_candidate_kind =
        issue377_required_field(path, line, "issue377_mutation_candidate_kind")?;
    let mutation_candidate_preview_only =
        issue377_bool_field(path, line, "issue377_mutation_candidate_preview_only")?;
    let mutation_candidate_refs_digest_only =
        issue377_bool_field(path, line, "issue377_mutation_candidate_refs_digest_only")?;
    let mutation_candidate_writer_gate_preflight = issue377_required_field(
        path,
        line,
        "issue377_mutation_candidate_writer_gate_preflight",
    )?;
    let mutation_candidate_write_allowed =
        issue377_bool_field(path, line, "issue377_mutation_candidate_write_allowed")?;
    let mutation_candidate_applied =
        issue377_bool_field(path, line, "issue377_mutation_candidate_applied")?;
    let mutation_candidate_apply_allowed =
        issue377_bool_field(path, line, "issue377_mutation_candidate_apply_allowed")?;
    let mutation_candidate_manual_review_required = issue377_bool_field(
        path,
        line,
        "issue377_mutation_candidate_manual_review_required",
    )?;
    let candidate_emitter_lane_coverage =
        issue377_required_field(path, line, "issue377_candidate_emitter_lane_coverage")?;
    let candidate_emitter_kind_coverage =
        issue377_required_field(path, line, "issue377_candidate_emitter_kind_coverage")?;
    let candidate_emitter_coverage_count =
        issue377_usize_field(path, line, "issue377_candidate_emitter_coverage_count")?;
    let candidate_emitter_all_preview_only =
        issue377_bool_field(path, line, "issue377_candidate_emitter_all_preview_only")?;
    let candidate_emitter_all_write_allowed =
        issue377_bool_field(path, line, "issue377_candidate_emitter_all_write_allowed")?;
    let candidate_emitter_all_apply_allowed =
        issue377_bool_field(path, line, "issue377_candidate_emitter_all_apply_allowed")?;
    let candidate_emitter_all_manual_review_required = issue377_bool_field(
        path,
        line,
        "issue377_candidate_emitter_all_manual_review_required",
    )?;
    let candidate_emitter_durable_preflight_owner = issue377_required_field(
        path,
        line,
        "issue377_candidate_emitter_durable_preflight_owner",
    )?;
    let candidate_emitter_writer_gate_bypass_allowed = issue377_bool_field(
        path,
        line,
        "issue377_candidate_emitter_writer_gate_bypass_allowed",
    )?;
    let candidate_emitter_direct_durable_write_allowed = issue377_bool_field(
        path,
        line,
        "issue377_candidate_emitter_direct_durable_write_allowed",
    )?;
    let candidate_emitter_ready_for_explicit_apply = issue377_bool_field(
        path,
        line,
        "issue377_candidate_emitter_ready_for_explicit_apply",
    )?;
    let related_issue_refs = issue377_required_field(path, line, "issue377_related_issue_refs")?;
    let related_issue_scope_map =
        issue377_required_field(path, line, "issue377_related_issue_scope_map")?;
    let related_issue_owner_scope =
        issue377_required_field(path, line, "issue377_related_issue_owner_scope")?;
    let related_issue_non_duplicate_count =
        issue377_usize_field(path, line, "issue377_related_issue_non_duplicate_count")?;
    let related_issue_all_non_duplicate =
        issue377_bool_field(path, line, "issue377_related_issue_all_non_duplicate")?;
    let related_issue_apply_allowed =
        issue377_bool_field(path, line, "issue377_related_issue_apply_allowed")?;
    let clean_room_reference_mode =
        issue377_required_field(path, line, "issue377_clean_room_reference_mode")?;
    let external_code_copied = issue377_bool_field(path, line, "issue377_external_code_copied")?;
    let external_prompt_or_schema_copied =
        issue377_bool_field(path, line, "issue377_external_prompt_or_schema_copied")?;
    let restricted_license_material_copied =
        issue377_bool_field(path, line, "issue377_restricted_license_material_copied")?;
    let license_provenance_posture =
        issue377_required_field(path, line, "issue377_license_provenance_posture")?;
    let clean_room_apply_allowed =
        issue377_bool_field(path, line, "issue377_clean_room_apply_allowed")?;
    let manual_approval_binding_present =
        issue377_bool_field(path, line, "issue377_manual_approval_binding_present")?;
    let manual_approval_candidate_id =
        issue377_required_field(path, line, "issue377_manual_approval_candidate_id")?;
    let manual_approval_evidence_digest =
        issue377_required_field(path, line, "issue377_manual_approval_evidence_digest")?;
    let manual_approval_rollback_anchor =
        issue377_required_field(path, line, "issue377_manual_approval_rollback_anchor")?;
    let manual_approval_requested_write_scope =
        issue377_required_field(path, line, "issue377_manual_approval_requested_write_scope")?;
    let manual_approval_ref = issue377_required_field(path, line, "issue377_manual_approval_ref")?;
    let manual_approval_expiration =
        issue377_required_field(path, line, "issue377_manual_approval_expiration")?;
    let manual_approval_apply_allowed =
        issue377_bool_field(path, line, "issue377_manual_approval_apply_allowed")?;
    let manual_approval_applied =
        issue377_bool_field(path, line, "issue377_manual_approval_applied")?;

    if problem_present && !problem_id.starts_with("redaction-digest:") {
        return Err(format!(
            "{} issue377 ProblemFinding must use digest-only id",
            path.display()
        ));
    }
    if problem_present
        && !matches!(
            problem_kind,
            "drift"
                | "regression"
                | "wasted_compute"
                | "stale_gene"
                | "contradiction"
                | "unsafe_candidate"
                | "tool_unreliability"
                | "routing_cost"
                | "missing_evidence"
        )
    {
        return Err(format!(
            "{} issue377 ProblemFinding kind is not bounded",
            path.display()
        ));
    }
    if problem_present && !matches!(problem_severity, "low" | "medium" | "high" | "critical") {
        return Err(format!(
            "{} issue377 ProblemFinding severity is not bounded",
            path.display()
        ));
    }
    if problem_present && !(1..=1000).contains(&problem_confidence_milli) {
        return Err(format!(
            "{} issue377 ProblemFinding confidence is not scored",
            path.display()
        ));
    }
    if problem_present && !problem_evidence_digest.starts_with("redaction-digest:") {
        return Err(format!(
            "{} issue377 ProblemFinding evidence must be digest-only",
            path.display()
        ));
    }
    if problem_present && !problem_source_digest.starts_with("redaction-digest:") {
        return Err(format!(
            "{} issue377 ProblemFinding source must be digest-only",
            path.display()
        ));
    }
    if problem_present
        && !matches!(
            problem_affected_surface,
            "runtime_kv_reuse"
                | "reasoning_gene"
                | "memory_rule"
                | "router_threshold"
                | "tool_policy"
                | "goal_queue"
                | "writer_gate"
                | "trace_schema"
        )
    {
        return Err(format!(
            "{} issue377 ProblemFinding affected surface is not bounded",
            path.display()
        ));
    }
    if problem_present
        && !matches!(
            problem_next_step,
            "watch" | "reflect" | "replay" | "experiment" | "hold" | "manual_review"
        )
    {
        return Err(format!(
            "{} issue377 ProblemFinding next step is not bounded",
            path.display()
        ));
    }
    if problem_present && problem_raw_payload_present {
        return Err(format!(
            "{} issue377 ProblemFinding must not carry raw payload",
            path.display()
        ));
    }
    if problem_present && !self_observation_present {
        return Err(format!(
            "{} issue377 ProblemFinding conflicts with missing SelfObservation",
            path.display()
        ));
    }
    if self_observation_present && !self_observation_id.starts_with("redaction-digest:") {
        return Err(format!(
            "{} issue377 SelfObservation must use digest-only id",
            path.display()
        ));
    }
    if self_observation_present && self_observation_schema != "self_observation_v1" {
        return Err(format!(
            "{} issue377 SelfObservation schema is not bounded",
            path.display()
        ));
    }
    if self_observation_present
        && !matches!(
            self_observation_signal_source,
            "runtime_trace_metrics"
                | "benchmark_replay"
                | "memory_admission"
                | "tool_reliability"
                | "operator_feedback"
                | "capability_budget"
        )
    {
        return Err(format!(
            "{} issue377 SelfObservation signal source is not bounded",
            path.display()
        ));
    }
    if self_observation_present && !self_observation_source_digest.starts_with("redaction-digest:")
    {
        return Err(format!(
            "{} issue377 SelfObservation source must be digest-only",
            path.display()
        ));
    }
    if self_observation_present
        && !matches!(
            self_observation_window,
            "current_run" | "second_task_roundtrip" | "replay_window"
        )
    {
        return Err(format!(
            "{} issue377 SelfObservation window is not bounded",
            path.display()
        ));
    }
    if self_observation_present
        && !self_observation_current_truth_digest.starts_with("redaction-digest:")
    {
        return Err(format!(
            "{} issue377 SelfObservation current truth must be digest-only",
            path.display()
        ));
    }
    if self_observation_present && !self_observation_digest_only {
        return Err(format!(
            "{} issue377 SelfObservation must be digest-only",
            path.display()
        ));
    }
    if self_observation_present
        && (self_observation_raw_payload_present
            || self_observation_write_allowed
            || self_observation_applied)
    {
        return Err(format!(
            "{} issue377 SelfObservation must not carry raw payload, write, or apply",
            path.display()
        ));
    }
    if problem_present && self_observation_source_digest != problem_source_digest {
        return Err(format!(
            "{} issue377 SelfObservation source must bind ProblemFinding source",
            path.display()
        ));
    }
    if self_observation_present && !self_model_present {
        return Err(format!(
            "{} issue377 SelfObservation conflicts with missing control-plane self-model",
            path.display()
        ));
    }
    if self_model_present && !self_model_id.starts_with("redaction-digest:") {
        return Err(format!(
            "{} issue377 control-plane self-model must use digest-only id",
            path.display()
        ));
    }
    if self_model_present && self_model_schema != "control_plane_self_model_v1" {
        return Err(format!(
            "{} issue377 control-plane self-model schema is not bounded",
            path.display()
        ));
    }
    if self_model_present && self_model_scope != "auditable_control_plane" {
        return Err(format!(
            "{} issue377 self-model scope must stay in the auditable control plane",
            path.display()
        ));
    }
    if self_model_present && self_model_claims_consciousness {
        return Err(format!(
            "{} issue377 self-model must not claim consciousness",
            path.display()
        ));
    }
    if self_model_present
        && (!self_model_digest_only
            || self_model_raw_payload_present
            || self_model_write_allowed
            || self_model_applied)
    {
        return Err(format!(
            "{} issue377 self-model must be digest-only, read-only, and unapplied",
            path.display()
        ));
    }
    if problem_present && !hypothesis_present {
        return Err(format!(
            "{} issue377 ProblemFinding preview conflicts with missing HypothesisCandidate",
            path.display()
        ));
    }
    if hypothesis_present && !hypothesis_id.starts_with("redaction-digest:") {
        return Err(format!(
            "{} issue377 HypothesisCandidate must use digest-only id",
            path.display()
        ));
    }
    if hypothesis_present
        && !matches!(
            hypothesis_kind,
            "gene" | "memory" | "routing" | "tool" | "goal"
        )
    {
        return Err(format!(
            "{} issue377 HypothesisCandidate kind is not bounded",
            path.display()
        ));
    }
    if hypothesis_present
        && !matches!(
            hypothesis_status,
            "observed" | "ready_for_validation" | "blocked" | "rejected" | "promoted_for_approval"
        )
    {
        return Err(format!(
            "{} issue377 HypothesisCandidate status is not bounded",
            path.display()
        ));
    }
    if hypothesis_present
        && !matches!(
            hypothesis_target_surface,
            "reasoning_gene" | "memory_rule" | "router_threshold" | "tool_policy" | "goal_queue"
        )
    {
        return Err(format!(
            "{} issue377 HypothesisCandidate target surface is not bounded",
            path.display()
        ));
    }
    if hypothesis_present
        && !matches!(
            hypothesis_expected_metric,
            "quality" | "latency" | "safety" | "memory_reuse" | "tool_success" | "route_cost"
        )
    {
        return Err(format!(
            "{} issue377 HypothesisCandidate expected metric is not bounded",
            path.display()
        ));
    }
    if hypothesis_present
        && !matches!(
            hypothesis_expected_direction,
            "increase" | "decrease" | "stabilize"
        )
    {
        return Err(format!(
            "{} issue377 HypothesisCandidate expected direction is not bounded",
            path.display()
        ));
    }
    if hypothesis_present
        && (hypothesis_required_gates.is_empty()
            || hypothesis_required_gates.split('|').any(|gate| {
                !matches!(
                    gate,
                    "trace_schema_gate"
                        | "focused_tests"
                        | "benchmark_gate"
                        | "replay_gate"
                        | "integration_gate"
                )
            }))
    {
        return Err(format!(
            "{} issue377 HypothesisCandidate required gates are not bounded",
            path.display()
        ));
    }
    if hypothesis_present && !hypothesis_rollback_anchor.starts_with("redaction-digest:") {
        return Err(format!(
            "{} issue377 HypothesisCandidate rollback anchor must be digest-only",
            path.display()
        ));
    }
    if hypothesis_present && hypothesis_raw_payload_present {
        return Err(format!(
            "{} issue377 HypothesisCandidate must not carry raw payload",
            path.display()
        ));
    }
    if hypothesis_present && (hypothesis_write_allowed || hypothesis_applied) {
        return Err(format!(
            "{} issue377 HypothesisCandidate must not write or apply",
            path.display()
        ));
    }
    if hypothesis_present && !hypothesis_operator_approval_required {
        return Err(format!(
            "{} issue377 HypothesisCandidate must require operator approval",
            path.display()
        ));
    }
    if (problem_present || hypothesis_present) && !link_id.starts_with("redaction-digest:") {
        return Err(format!(
            "{} issue377 problem-hypothesis link must use digest-only id",
            path.display()
        ));
    }
    if (problem_present || hypothesis_present) && admission_decision != "preview_only" {
        return Err(format!(
            "{} issue377 ProblemFinding/HypothesisCandidate must remain preview-only",
            path.display()
        ));
    }
    if (problem_present || hypothesis_present) && !lexicographic_admission_present {
        return Err(format!(
            "{} issue377 lexicographic admission proof is missing",
            path.display()
        ));
    }
    if lexicographic_admission_present
        && lexicographic_admission_order
            != "user_intent_preservation>safety>digest_only_evidence>rollback_anchor>quality_delta>cost_delta>latency_delta"
    {
        return Err(format!(
            "{} issue377 lexicographic admission order is not bounded",
            path.display()
        ));
    }
    if lexicographic_admission_present
        && (!user_intent_preserved
            || !safety_gate_passed
            || !digest_only_evidence_gate_passed
            || !rollback_anchor_gate_passed)
    {
        return Err(format!(
            "{} issue377 lexicographic hard gates must pass before performance ranking",
            path.display()
        ));
    }
    if lexicographic_admission_present && !performance_tiebreaker_only {
        return Err(format!(
            "{} issue377 performance deltas must remain tie-breakers",
            path.display()
        ));
    }
    if lexicographic_admission_present
        && !matches!(
            hard_gate_failure_action,
            "hold" | "reject" | "quarantine" | "manual_review"
        )
    {
        return Err(format!(
            "{} issue377 hard-gate failure action is not fail-closed",
            path.display()
        ));
    }
    if lexicographic_admission_present
        && !matches!(
            risk_override_action,
            "hold" | "reject" | "quarantine" | "manual_review"
        )
    {
        return Err(format!(
            "{} issue377 risk override action is not fail-closed",
            path.display()
        ));
    }
    if lexicographic_admission_present && !matches!(privacy_risk, "low" | "medium" | "high") {
        return Err(format!(
            "{} issue377 privacy risk is not bounded: {privacy_risk}",
            path.display()
        ));
    }
    if lexicographic_admission_present && !matches!(license_risk, "low" | "medium" | "high") {
        return Err(format!(
            "{} issue377 license risk is not bounded: {license_risk}",
            path.display()
        ));
    }
    let risk_blocked = negative_evidence_count > 0
        || privacy_risk != "low"
        || license_risk != "low"
        || unsupported_capability_requested
        || unsafe_side_effect_allowed;
    if lexicographic_admission_present && risk_override_clear == risk_blocked {
        return Err(format!(
            "{} issue377 risk override clear conflicts with risk blockers",
            path.display()
        ));
    }
    if lexicographic_admission_present && lexicographic_admission_apply_allowed {
        return Err(format!(
            "{} issue377 lexicographic admission does not apply by itself",
            path.display()
        ));
    }
    if lexicographic_admission_present && !best_next_state_id.starts_with("redaction-digest:") {
        return Err(format!(
            "{} issue377 best-next-state id must use digest-only id",
            path.display()
        ));
    }
    let expected_best_next_state = issue377_predicament_best_next_state(path, line)?;
    if lexicographic_admission_present && best_next_state != expected_best_next_state {
        return Err(format!(
            "{} issue377 best-next-state conflicts with predicament fields",
            path.display()
        ));
    }
    if lexicographic_admission_present && !best_next_state_selected {
        return Err(format!(
            "{} issue377 best-next-state must be selected by the admission rule",
            path.display()
        ));
    }
    if (problem_present || hypothesis_present) && !predicament_present {
        return Err(format!(
            "{} issue377 problem hypothesis preview conflicts with missing predicament signal",
            path.display()
        ));
    }
    if predicament_present && !predicament_id.starts_with("redaction-digest:") {
        return Err(format!(
            "{} issue377 predicament signal must use digest-only id",
            path.display()
        ));
    }
    if (problem_present || hypothesis_present) && self_trigger_stage != "preview_only" {
        return Err(format!(
            "{} issue377 self-trigger stage must remain preview-only",
            path.display()
        ));
    }
    if (problem_present || hypothesis_present) && apply_allowed {
        return Err(format!(
            "{} issue377 evolution preview conflicts with apply permission",
            path.display()
        ));
    }
    if (problem_present || hypothesis_present) && !experiment_plan_present {
        return Err(format!(
            "{} issue377 problem hypothesis preview conflicts with missing ExperimentPlan",
            path.display()
        ));
    }
    if experiment_plan_present && !experiment_plan_id.starts_with("redaction-digest:") {
        return Err(format!(
            "{} issue377 ExperimentPlan must use digest-only id",
            path.display()
        ));
    }
    if experiment_plan_present && experiment_plan_mode != "preview_only" {
        return Err(format!(
            "{} issue377 ExperimentPlan must remain preview-only",
            path.display()
        ));
    }
    if experiment_plan_present
        && experiment_plan_level_path != "L0_schema_safety|L1_focused_validation|L3_benchmark"
    {
        return Err(format!(
            "{} issue377 ExperimentPlan validation path is not the minimal bounded path",
            path.display()
        ));
    }
    if experiment_plan_present && experiment_plan_required_gates != hypothesis_required_gates {
        return Err(format!(
            "{} issue377 ExperimentPlan gates must bind the HypothesisCandidate gates",
            path.display()
        ));
    }
    if experiment_plan_present && !(1..=4096).contains(&experiment_plan_budget_tokens) {
        return Err(format!(
            "{} issue377 ExperimentPlan budget is not bounded",
            path.display()
        ));
    }
    if experiment_plan_present && !experiment_plan_stop_on_fail {
        return Err(format!(
            "{} issue377 ExperimentPlan must stop on first failed gate",
            path.display()
        ));
    }
    if experiment_plan_present
        && (!experiment_plan_rollback_anchor.starts_with("redaction-digest:")
            || experiment_plan_rollback_anchor != hypothesis_rollback_anchor)
    {
        return Err(format!(
            "{} issue377 ExperimentPlan rollback anchor must bind the HypothesisCandidate rollback anchor",
            path.display()
        ));
    }
    if experiment_plan_present && experiment_plan_raw_payload_present {
        return Err(format!(
            "{} issue377 ExperimentPlan must not carry raw payload",
            path.display()
        ));
    }
    if experiment_plan_present && (experiment_plan_write_allowed || experiment_plan_applied) {
        return Err(format!(
            "{} issue377 ExperimentPlan must not write or apply",
            path.display()
        ));
    }
    if experiment_plan_present && !evidence_bundle_present {
        return Err(format!(
            "{} issue377 ExperimentPlan preview conflicts with missing EvidenceBundle",
            path.display()
        ));
    }
    if evidence_bundle_present && !evidence_bundle_id.starts_with("redaction-digest:") {
        return Err(format!(
            "{} issue377 EvidenceBundle must use digest-only id",
            path.display()
        ));
    }
    if evidence_bundle_present && evidence_bundle_schema != "evidence_bundle_v1" {
        return Err(format!(
            "{} issue377 EvidenceBundle schema is not bounded",
            path.display()
        ));
    }
    if evidence_bundle_present && evidence_bundle_metric != hypothesis_expected_metric {
        return Err(format!(
            "{} issue377 EvidenceBundle metric must bind the HypothesisCandidate metric",
            path.display()
        ));
    }
    if evidence_bundle_present && evidence_bundle_direction != hypothesis_expected_direction {
        return Err(format!(
            "{} issue377 EvidenceBundle direction must bind the HypothesisCandidate direction",
            path.display()
        ));
    }
    if evidence_bundle_present
        && (evidence_bundle_pass_count == 0 || evidence_bundle_fail_count > 0)
    {
        return Err(format!(
            "{} issue377 EvidenceBundle pass/fail counts must prove a clean run",
            path.display()
        ));
    }
    if evidence_bundle_present && evidence_bundle_command_label != "issue30_fresh_checkout_smoke" {
        return Err(format!(
            "{} issue377 EvidenceBundle command label is not bounded",
            path.display()
        ));
    }
    if evidence_bundle_present && !evidence_bundle_refs_digest_only {
        return Err(format!(
            "{} issue377 EvidenceBundle refs must remain digest-only",
            path.display()
        ));
    }
    if evidence_bundle_present
        && (evidence_bundle_raw_payload_present
            || evidence_bundle_write_allowed
            || evidence_bundle_applied)
    {
        return Err(format!(
            "{} issue377 EvidenceBundle must not carry raw payload, write, or apply",
            path.display()
        ));
    }
    if !matches!(
        experiment_decision,
        "hold_for_evidence" | "reject" | "quarantine" | "rollback" | "promote_for_approval"
    ) {
        return Err(format!(
            "{} issue377 ExperimentDecision is not bounded: {experiment_decision}",
            path.display()
        ));
    }
    if experiment_decision_schema != "experiment_decision_v1" {
        return Err(format!(
            "{} issue377 ExperimentDecision schema is not bounded",
            path.display()
        ));
    }
    if !matches!(
        experiment_decision_reason,
        "missing_evidence"
            | "failed_gate"
            | "risk_blocker"
            | "rollback_required"
            | "clean_evidence_bundle_promotes_preview"
    ) {
        return Err(format!(
            "{} issue377 ExperimentDecision reason is not bounded",
            path.display()
        ));
    }
    if experiment_decision == "promote_for_approval"
        && experiment_decision_reason != "clean_evidence_bundle_promotes_preview"
    {
        return Err(format!(
            "{} issue377 ExperimentDecision promotion reason conflicts with decision",
            path.display()
        ));
    }
    if !experiment_decision_evidence_bundle_id.starts_with("redaction-digest:")
        || experiment_decision_evidence_bundle_id != evidence_bundle_id
    {
        return Err(format!(
            "{} issue377 ExperimentDecision must bind the EvidenceBundle id",
            path.display()
        ));
    }
    if !matches!(
        experiment_decision_target,
        "none" | "mutation_candidate_emitter" | "manual_review" | "rollback"
    ) {
        return Err(format!(
            "{} issue377 ExperimentDecision target is not bounded",
            path.display()
        ));
    }
    if experiment_decision == "promote_for_approval"
        && experiment_decision_target != "mutation_candidate_emitter"
    {
        return Err(format!(
            "{} issue377 ExperimentDecision promotion target conflicts with decision",
            path.display()
        ));
    }
    if experiment_decision == "promote_for_approval"
        && !experiment_decision_manual_approval_required
    {
        return Err(format!(
            "{} issue377 ExperimentDecision promotion must require manual approval",
            path.display()
        ));
    }
    if experiment_decision_apply_allowed {
        return Err(format!(
            "{} issue377 ExperimentDecision must not apply",
            path.display()
        ));
    }
    if experiment_plan_present && experiment_runner_allowed {
        return Err(format!(
            "{} issue377 ExperimentPlan preview conflicts with runner permission",
            path.display()
        ));
    }
    if experiment_plan_present && experiment_apply_allowed {
        return Err(format!(
            "{} issue377 ExperimentPlan preview conflicts with apply permission",
            path.display()
        ));
    }
    if experiment_decision == "promote_for_approval" && !mutation_candidate_emitter_present {
        return Err(format!(
            "{} issue377 ExperimentDecision conflicts with missing MutationCandidateEmitter",
            path.display()
        ));
    }
    if mutation_candidate_emitter_present
        && !mutation_candidate_emitter_id.starts_with("redaction-digest:")
    {
        return Err(format!(
            "{} issue377 MutationCandidateEmitter must use digest-only id",
            path.display()
        ));
    }
    if mutation_candidate_emitter_present && !mutation_candidate_id.starts_with("redaction-digest:")
    {
        return Err(format!(
            "{} issue377 mutation candidate must use digest-only id",
            path.display()
        ));
    }
    if mutation_candidate_emitter_present
        && !mutation_candidate_evidence_digest.starts_with("redaction-digest:")
    {
        return Err(format!(
            "{} issue377 mutation candidate evidence must use digest-only id",
            path.display()
        ));
    }
    if mutation_candidate_emitter_present
        && !mutation_candidate_rollback_anchor.starts_with("redaction-digest:")
    {
        return Err(format!(
            "{} issue377 mutation candidate rollback anchor must use digest-only id",
            path.display()
        ));
    }
    if mutation_candidate_emitter_present
        && !matches!(
            mutation_candidate_requested_write_scope,
            "reasoning_genome_preview"
                | "memory_admission_preview"
                | "routing_policy_preview"
                | "tool_policy_preview"
                | "evolution_goal_preview"
        )
    {
        return Err(format!(
            "{} issue377 mutation candidate write scope is not bounded: {mutation_candidate_requested_write_scope}",
            path.display()
        ));
    }
    if mutation_candidate_emitter_present
        && !matches!(
            mutation_candidate_kind,
            "mutation_plan_preview"
                | "memory_admission_preview"
                | "routing_shadow_proposal"
                | "tool_policy_candidate"
                | "evolution_goal_preview"
        )
    {
        return Err(format!(
            "{} issue377 mutation candidate kind is not bounded: {mutation_candidate_kind}",
            path.display()
        ));
    }
    if mutation_candidate_emitter_present && !mutation_candidate_preview_only {
        return Err(format!(
            "{} issue377 mutation candidate must remain preview-only",
            path.display()
        ));
    }
    if mutation_candidate_emitter_present && !mutation_candidate_refs_digest_only {
        return Err(format!(
            "{} issue377 mutation candidate refs must remain digest-only",
            path.display()
        ));
    }
    if mutation_candidate_emitter_present
        && !matches!(
            mutation_candidate_writer_gate_preflight,
            "preview_only" | "hold" | "reject" | "ready_for_explicit_apply"
        )
    {
        return Err(format!(
            "{} issue377 mutation candidate writer-gate preflight is not bounded: {mutation_candidate_writer_gate_preflight}",
            path.display()
        ));
    }
    if mutation_candidate_emitter_present && mutation_candidate_write_allowed {
        return Err(format!(
            "{} issue377 mutation candidate preview conflicts with write permission",
            path.display()
        ));
    }
    if mutation_candidate_emitter_present && mutation_candidate_applied {
        return Err(format!(
            "{} issue377 mutation candidate preview conflicts with applied state",
            path.display()
        ));
    }
    if mutation_candidate_emitter_present && mutation_candidate_apply_allowed {
        return Err(format!(
            "{} issue377 mutation candidate preview conflicts with apply permission",
            path.display()
        ));
    }
    if mutation_candidate_emitter_present && !mutation_candidate_manual_review_required {
        return Err(format!(
            "{} issue377 mutation candidate preview must require manual review",
            path.display()
        ));
    }
    const ISSUE377_CANDIDATE_EMITTER_LANES: &str = "reasoning_genome_preview|memory_admission_preview|routing_policy_preview|tool_policy_preview|evolution_goal_preview";
    const ISSUE377_CANDIDATE_EMITTER_KINDS: &str = "mutation_plan_preview|memory_admission_preview|routing_shadow_proposal|tool_policy_candidate|evolution_goal_preview";
    const ISSUE377_RELATED_ISSUE_REFS: &str = "#6|#7|#74|#79|#375";
    const ISSUE377_RELATED_ISSUE_SCOPE_MAP: &str = "#6:experiment_gates|#7:memory_admission_pipeline|#74:thinking_scheduler|#79:evolution_goal_queue|#375:pre_reasoning_genome_isa";
    const ISSUE377_CLEAN_ROOM_REFERENCE_MODE: &str = "rust_norion_terms_only";
    const ISSUE377_LICENSE_PROVENANCE_POSTURE: &str = "project_owned_digest_only";
    const ISSUE377_VALIDATION_SKIPPED_LEVELS: &str =
        "L2_replay|L4_integration_readiness|L5_promotion_window";
    const ISSUE377_VALIDATION_SKIPPED_REASON: &str = "minimal_existing_evidence_path";
    const ISSUE377_HUMAN_APPLY_LEVEL: &str = "L6_human_apply";
    if validation_skipped_levels != ISSUE377_VALIDATION_SKIPPED_LEVELS {
        return Err(format!(
            "{} issue377 validation skipped levels are not bounded",
            path.display()
        ));
    }
    if validation_skipped_reason != ISSUE377_VALIDATION_SKIPPED_REASON {
        return Err(format!(
            "{} issue377 validation skipped reason is not minimal",
            path.display()
        ));
    }
    if human_apply_level != ISSUE377_HUMAN_APPLY_LEVEL {
        return Err(format!(
            "{} issue377 human apply level is not L6",
            path.display()
        ));
    }
    if human_apply_inside_engine || validation_level_apply_allowed {
        return Err(format!(
            "{} issue377 validation levels must keep human apply outside the engine",
            path.display()
        ));
    }
    if mutation_candidate_emitter_present
        && candidate_emitter_lane_coverage != ISSUE377_CANDIDATE_EMITTER_LANES
    {
        return Err(format!(
            "{} issue377 candidate emitter lane coverage is incomplete",
            path.display()
        ));
    }
    if mutation_candidate_emitter_present
        && candidate_emitter_kind_coverage != ISSUE377_CANDIDATE_EMITTER_KINDS
    {
        return Err(format!(
            "{} issue377 candidate emitter kind coverage is incomplete",
            path.display()
        ));
    }
    if mutation_candidate_emitter_present && candidate_emitter_coverage_count != 5 {
        return Err(format!(
            "{} issue377 candidate emitter coverage count must be 5",
            path.display()
        ));
    }
    if mutation_candidate_emitter_present && !candidate_emitter_all_preview_only {
        return Err(format!(
            "{} issue377 candidate emitter lanes must all stay preview-only",
            path.display()
        ));
    }
    if mutation_candidate_emitter_present
        && (candidate_emitter_all_write_allowed || candidate_emitter_all_apply_allowed)
    {
        return Err(format!(
            "{} issue377 candidate emitter lanes must not write or apply",
            path.display()
        ));
    }
    if mutation_candidate_emitter_present && !candidate_emitter_all_manual_review_required {
        return Err(format!(
            "{} issue377 candidate emitter lanes must require manual review",
            path.display()
        ));
    }
    if mutation_candidate_emitter_present
        && candidate_emitter_durable_preflight_owner != "unified_writer_gate"
    {
        return Err(format!(
            "{} issue377 durable preflight owner must be unified_writer_gate",
            path.display()
        ));
    }
    if mutation_candidate_emitter_present
        && (candidate_emitter_writer_gate_bypass_allowed
            || candidate_emitter_direct_durable_write_allowed)
    {
        return Err(format!(
            "{} issue377 candidate emitter must not bypass the unified writer gate",
            path.display()
        ));
    }
    if mutation_candidate_emitter_present && candidate_emitter_ready_for_explicit_apply {
        return Err(format!(
            "{} issue377 candidate emitter must not mark explicit apply ready",
            path.display()
        ));
    }
    if related_issue_refs != ISSUE377_RELATED_ISSUE_REFS {
        return Err(format!(
            "{} issue377 related issue refs must cover #6/#7/#74/#79/#375",
            path.display()
        ));
    }
    if related_issue_scope_map != ISSUE377_RELATED_ISSUE_SCOPE_MAP {
        return Err(format!(
            "{} issue377 related issue scope map must stay non-duplicated",
            path.display()
        ));
    }
    if related_issue_owner_scope != "meta_cognitive_evolution_loop" {
        return Err(format!(
            "{} issue377 related issue owner scope is not bounded",
            path.display()
        ));
    }
    if related_issue_non_duplicate_count != 5 || !related_issue_all_non_duplicate {
        return Err(format!(
            "{} issue377 related issues must all be non-duplicated",
            path.display()
        ));
    }
    if related_issue_apply_allowed {
        return Err(format!(
            "{} issue377 related issue mapping must not apply",
            path.display()
        ));
    }
    if clean_room_reference_mode != ISSUE377_CLEAN_ROOM_REFERENCE_MODE {
        return Err(format!(
            "{} issue377 clean-room reference mode is not bounded",
            path.display()
        ));
    }
    if external_code_copied
        || external_prompt_or_schema_copied
        || restricted_license_material_copied
    {
        return Err(format!(
            "{} issue377 clean-room proof must not copy external material",
            path.display()
        ));
    }
    if license_provenance_posture != ISSUE377_LICENSE_PROVENANCE_POSTURE {
        return Err(format!(
            "{} issue377 license provenance posture is not project-owned digest-only",
            path.display()
        ));
    }
    if clean_room_apply_allowed {
        return Err(format!(
            "{} issue377 clean-room proof must not apply",
            path.display()
        ));
    }
    if mutation_candidate_emitter_present && !manual_approval_binding_present {
        return Err(format!(
            "{} issue377 manual approval binding must be present",
            path.display()
        ));
    }
    if mutation_candidate_emitter_present && manual_approval_candidate_id != mutation_candidate_id {
        return Err(format!(
            "{} issue377 manual approval must bind the mutation candidate id",
            path.display()
        ));
    }
    if mutation_candidate_emitter_present
        && manual_approval_evidence_digest != mutation_candidate_evidence_digest
    {
        return Err(format!(
            "{} issue377 manual approval must bind the mutation evidence digest",
            path.display()
        ));
    }
    if mutation_candidate_emitter_present
        && manual_approval_rollback_anchor != mutation_candidate_rollback_anchor
    {
        return Err(format!(
            "{} issue377 manual approval must bind the rollback anchor",
            path.display()
        ));
    }
    if mutation_candidate_emitter_present
        && manual_approval_requested_write_scope != mutation_candidate_requested_write_scope
    {
        return Err(format!(
            "{} issue377 manual approval must bind the requested write scope",
            path.display()
        ));
    }
    if mutation_candidate_emitter_present && !manual_approval_ref.starts_with("redaction-digest:") {
        return Err(format!(
            "{} issue377 manual approval ref must use digest-only id",
            path.display()
        ));
    }
    if mutation_candidate_emitter_present && manual_approval_expiration == "none" {
        return Err(format!(
            "{} issue377 manual approval must include expiration",
            path.display()
        ));
    }
    if mutation_candidate_emitter_present && manual_approval_apply_allowed {
        return Err(format!(
            "{} issue377 manual approval binding does not apply by itself",
            path.display()
        ));
    }
    if mutation_candidate_emitter_present && manual_approval_applied {
        return Err(format!(
            "{} issue377 manual approval binding conflicts with applied state",
            path.display()
        ));
    }

    Ok(problem_present
        && problem_id.starts_with("redaction-digest:")
        && hypothesis_present
        && hypothesis_id.starts_with("redaction-digest:")
        && link_id.starts_with("redaction-digest:")
        && admission_decision == "preview_only"
        && risk_override_clear
        && predicament_present
        && predicament_id.starts_with("redaction-digest:")
        && predicament_ready
        && self_trigger_stage == "preview_only"
        && !apply_allowed
        && experiment_plan_present
        && experiment_plan_id.starts_with("redaction-digest:")
        && experiment_plan_mode == "preview_only"
        && experiment_plan_level_path == "L0_schema_safety|L1_focused_validation|L3_benchmark"
        && validation_skipped_levels == ISSUE377_VALIDATION_SKIPPED_LEVELS
        && validation_skipped_reason == ISSUE377_VALIDATION_SKIPPED_REASON
        && human_apply_level == ISSUE377_HUMAN_APPLY_LEVEL
        && !human_apply_inside_engine
        && !validation_level_apply_allowed
        && experiment_plan_required_gates == hypothesis_required_gates
        && (1..=4096).contains(&experiment_plan_budget_tokens)
        && experiment_plan_stop_on_fail
        && experiment_plan_rollback_anchor == hypothesis_rollback_anchor
        && !experiment_plan_raw_payload_present
        && !experiment_plan_write_allowed
        && !experiment_plan_applied
        && evidence_bundle_present
        && evidence_bundle_id.starts_with("redaction-digest:")
        && evidence_bundle_refs_digest_only
        && experiment_decision == "promote_for_approval"
        && !experiment_runner_allowed
        && !experiment_apply_allowed
        && mutation_candidate_emitter_present
        && mutation_candidate_emitter_id.starts_with("redaction-digest:")
        && mutation_candidate_id.starts_with("redaction-digest:")
        && mutation_candidate_evidence_digest.starts_with("redaction-digest:")
        && mutation_candidate_rollback_anchor.starts_with("redaction-digest:")
        && mutation_candidate_requested_write_scope == "reasoning_genome_preview"
        && mutation_candidate_kind == "mutation_plan_preview"
        && mutation_candidate_preview_only
        && mutation_candidate_refs_digest_only
        && mutation_candidate_writer_gate_preflight == "hold"
        && !mutation_candidate_write_allowed
        && !mutation_candidate_applied
        && !mutation_candidate_apply_allowed
        && mutation_candidate_manual_review_required
        && candidate_emitter_lane_coverage == ISSUE377_CANDIDATE_EMITTER_LANES
        && candidate_emitter_kind_coverage == ISSUE377_CANDIDATE_EMITTER_KINDS
        && candidate_emitter_coverage_count == 5
        && candidate_emitter_all_preview_only
        && !candidate_emitter_all_write_allowed
        && !candidate_emitter_all_apply_allowed
        && candidate_emitter_all_manual_review_required
        && candidate_emitter_durable_preflight_owner == "unified_writer_gate"
        && !candidate_emitter_writer_gate_bypass_allowed
        && !candidate_emitter_direct_durable_write_allowed
        && !candidate_emitter_ready_for_explicit_apply
        && related_issue_refs == ISSUE377_RELATED_ISSUE_REFS
        && related_issue_scope_map == ISSUE377_RELATED_ISSUE_SCOPE_MAP
        && related_issue_owner_scope == "meta_cognitive_evolution_loop"
        && related_issue_non_duplicate_count == 5
        && related_issue_all_non_duplicate
        && !related_issue_apply_allowed
        && clean_room_reference_mode == ISSUE377_CLEAN_ROOM_REFERENCE_MODE
        && !external_code_copied
        && !external_prompt_or_schema_copied
        && !restricted_license_material_copied
        && license_provenance_posture == ISSUE377_LICENSE_PROVENANCE_POSTURE
        && !clean_room_apply_allowed
        && manual_approval_binding_present
        && manual_approval_candidate_id == mutation_candidate_id
        && manual_approval_evidence_digest == mutation_candidate_evidence_digest
        && manual_approval_rollback_anchor == mutation_candidate_rollback_anchor
        && manual_approval_requested_write_scope == mutation_candidate_requested_write_scope
        && manual_approval_ref.starts_with("redaction-digest:")
        && manual_approval_expiration != "none"
        && !manual_approval_apply_allowed
        && !manual_approval_applied)
}

fn issue377_required_field<'a>(path: &Path, line: &'a str, field: &str) -> Result<&'a str, String> {
    release_field(line, field)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{} missing {field}", path.display()))
}

fn issue377_bool_field(path: &Path, line: &str, field: &str) -> Result<bool, String> {
    match issue377_required_field(path, line, field)? {
        "true" => Ok(true),
        "false" => Ok(false),
        value => Err(format!(
            "{} {field} is not boolean: {value}",
            path.display()
        )),
    }
}

fn issue377_i32_field(path: &Path, line: &str, field: &str) -> Result<i32, String> {
    issue377_required_field(path, line, field)?
        .parse::<i32>()
        .map_err(|_| format!("{} {field} must be numeric", path.display()))
}

fn issue377_usize_field(path: &Path, line: &str, field: &str) -> Result<usize, String> {
    issue377_required_field(path, line, field)?
        .parse::<usize>()
        .map_err(|_| format!("{} {field} must be numeric", path.display()))
}

fn require_issue_fields(
    path: &Path,
    index: usize,
    line: &str,
    fields: &[&str],
) -> Result<(), String> {
    for field in fields {
        required_issue_field(path, index, line, field)?;
    }
    Ok(())
}

fn state_files_statement(path: &Path) -> Result<String, String> {
    let raw = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    for (index, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let memory = required_issue_field(path, index, line, "memory")?;
        let experience = required_issue_field(path, index, line, "experience")?;
        let adaptive = required_issue_field(path, index, line, "adaptive")?;
        let memory_exists = Path::new(&memory).exists();
        let experience_exists = Path::new(&experience).exists();
        let adaptive_exists = Path::new(&adaptive).exists();
        let state_files_ready = memory_exists && experience_exists && adaptive_exists;
        let memory_ndkv = Path::new(&memory)
            .extension()
            .and_then(|value| value.to_str())
            == Some("ndkv");
        let experience_ndkv = Path::new(&experience)
            .extension()
            .and_then(|value| value.to_str())
            == Some("ndkv");
        let adaptive_ndkv = Path::new(&adaptive)
            .extension()
            .and_then(|value| value.to_str())
            == Some("ndkv");
        let state_files_ndkv = memory_ndkv && experience_ndkv && adaptive_ndkv;
        let ndkv_non_fixture = if let Some(raw_value) =
            release_field(line, "ndkv_non_fixture_writes")
        {
            let writes = roundtrip_usize_field(path, index, "ndkv_non_fixture_writes", raw_value)?;
            let proof = writes == 0;
            format!(
                " issue2_ndkv_non_fixture_writes={writes} issue2_ndkv_non_fixture_write_proof={proof} issue2_ndkv_non_fixture_write_proof_source=state_files_input"
            )
        } else {
            String::new()
        };
        if let Some(raw_value) = release_field(line, "issue30_state_files_ready") {
            if raw_value != state_files_ready.to_string() {
                return Err(format!(
                    "{}:{} issue30_state_files_ready conflicts with state file existence",
                    path.display(),
                    index + 1
                ));
            }
        }
        if let Some(raw_value) = release_field(line, "issue2_state_files_ndkv_proof") {
            if raw_value != state_files_ndkv.to_string() {
                return Err(format!(
                    "{}:{} issue2_state_files_ndkv_proof conflicts with state file extensions",
                    path.display(),
                    index + 1
                ));
            }
        }
        return Ok(format!(
            "memory_file_exists={memory_exists} experience_file_exists={experience_exists} adaptive_file_exists={adaptive_exists} memory_file_ndkv={memory_ndkv} experience_file_ndkv={experience_ndkv} adaptive_file_ndkv={adaptive_ndkv} issue2_state_files_ndkv_proof={state_files_ndkv} issue2_state_files_ndkv_proof_source=state_files_input_derived issue30_state_files_ready={state_files_ready} issue30_state_files_ready_source=state_files_input_derived{ndkv_non_fixture} state_files_source=state_files_input",
        ));
    }
    Err(format!("{} has no state file rows", path.display()))
}

fn validate_packet(config: &EvidencePacketConfig, packet: &str) -> Result<(), String> {
    let mut failures = Vec::new();

    for required in &config.required {
        if !packet.contains(required) {
            failures.push(format!("missing required evidence: {required}"));
        }
    }

    for rejected in &config.rejected {
        if packet.contains(rejected) {
            failures.push(format!("rejected evidence still present: {rejected}"));
        }
    }

    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("; "))
    }
}

fn redact(text: &str) -> String {
    text.lines().map(redact_line).collect::<Vec<_>>().join("\n")
}

fn redact_line(line: &str) -> String {
    let lower = line.to_ascii_lowercase();
    if let Some(redacted) = redact_payload_line(line, &lower) {
        return redacted;
    }
    if line_contains_payload_field(&lower) {
        return "payload_line=<redacted-payload>".to_owned();
    }
    if [
        "api_key",
        "apikey",
        "access_token",
        "auth_token",
        "token=",
        "secret=",
        "password=",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
    {
        if let Some((name, _)) = line.split_once('=') {
            return format!("{}=<redacted>", name.trim_end());
        }
        return "<redacted>".to_owned();
    }
    line.split_whitespace()
        .map(redact_word)
        .collect::<Vec<_>>()
        .join(" ")
}

fn redact_payload_line(line: &str, lower: &str) -> Option<String> {
    for prefix in [
        "prompt:",
        "answer:",
        "raw_prompt=",
        "raw_answer=",
        "prompt_text=",
        "answer_text=",
        "chain_of_thought:",
        "chain_of_thought=",
        "chain-of-thought:",
        "chain-of-thought=",
        "hidden_cot:",
        "hidden_cot=",
        "hidden_reasoning:",
        "hidden_reasoning=",
        "cot:",
        "cot=",
    ] {
        if lower.trim_start().starts_with(prefix) {
            let split_at = match (line.find(':'), line.find('=')) {
                (Some(left), Some(right)) => left.min(right),
                (Some(index), None) | (None, Some(index)) => index,
                (None, None) => return Some("<redacted-payload>".to_owned()),
            };
            return Some(format!(
                "{}=<redacted-payload>",
                line[..split_at].trim_end()
            ));
        }
    }
    None
}

fn line_contains_payload_field(lower: &str) -> bool {
    let trimmed = lower.trim_start();
    trimmed.starts_with("key=")
        || trimmed.starts_with("lesson=")
        || lower.contains(" key=")
        || lower.contains(" lesson=")
}

fn redact_word(word: &str) -> String {
    if looks_private_path(word) {
        return word
            .split_once('=')
            .map(|(name, _)| format!("{name}=<redacted-path>"))
            .unwrap_or_else(|| "<redacted-path>".to_owned());
    }
    if ["ghp_", "github_pat_", "sk-", "xoxb-"]
        .iter()
        .any(|prefix| word.starts_with(prefix))
    {
        "<redacted>".to_owned()
    } else {
        word.to_owned()
    }
}

fn looks_private_path(word: &str) -> bool {
    let lower = word.to_ascii_lowercase();
    lower.contains("appdata")
        || lower.contains("\\users\\")
        || lower.contains("/users/")
        || word
            .as_bytes()
            .windows(3)
            .any(|window| window[1] == b':' && (window[2] == b'\\' || window[2] == b'/'))
}

fn parse_gate(value: &str) -> Result<String, String> {
    match value {
        "passed" | "failed" | "blocked" => Ok(value.to_owned()),
        _ => Err("--gate must be passed, failed, or blocked".to_owned()),
    }
}

fn required(name: &str, value: Option<String>) -> Result<String, String> {
    value.ok_or_else(|| format!("missing {name}"))
}

fn split_option(arg: &str) -> Result<(&str, Option<String>), String> {
    if !arg.starts_with('-') {
        return Err(format!("unexpected evidence-packet argument: {arg}"));
    }
    Ok(arg
        .split_once('=')
        .map(|(name, value)| (name, Some(value.to_owned())))
        .unwrap_or((arg, None)))
}

fn option_value<I, S>(
    name: &str,
    inline_value: Option<String>,
    args: &mut I,
) -> Result<String, String>
where
    I: Iterator<Item = S>,
    S: AsRef<str>,
{
    if let Some(value) = inline_value {
        return Ok(value);
    }
    args.next()
        .map(|value| value.as_ref().to_owned())
        .filter(|value| !value.starts_with('-'))
        .ok_or_else(|| format!("missing value for {name}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evidence_packet_is_deterministic_and_redacted() {
        let config = EvidencePacketConfig {
            issue: "#48".to_owned(),
            commit: "abc123".to_owned(),
            command: "cargo test -p norion-cli -- token=ghp_leak".to_owned(),
            gate: "passed".to_owned(),
            input: PathBuf::from("unused"),
            output: None,
            git_worktree: None,
            release_review_input: None,
            issue_state_input: None,
            demo_proof_input: None,
            roundtrip_proof_input: None,
            trace_report_input: None,
            state_gate_input: None,
            research_sandbox_input: None,
            issue30_context_input: None,
            issue243_fixture_matrix_input: None,
            state_files_input: None,
            required: vec![
                "OPENAI_API_KEY=<redacted>".to_owned(),
                "payload_line=<redacted-payload>".to_owned(),
            ],
            rejected: vec!["C:\\Users".to_owned(), "private raw prompt".to_owned()],
        };

        let packet = render_evidence_packet(
            &config,
            "ok\nOPENAI_API_KEY=sk-leak\npath=C:\\Users\\jy\\AppData\\Local\\Temp\\run.txt\nprompt: private raw prompt\nanswer_text=raw answer\nhidden_cot=private hidden thoughts\nid=3 key=runtime_kv :: Design a Rust Noiron prototype lesson=reuse_response: raw model output\nplain ghp_alsoleak done\n",
            &[],
        );

        validate_packet(&config, &packet).expect("packet should pass required and rejected gates");
        assert!(packet.contains("## Evidence packet for #48"));
        assert!(packet.contains("- command: cargo test -p norion-cli -- token=<redacted>"));
        assert!(redact("saved_tokens=12 avoided_tokens=8").contains("saved_tokens=12"));
        assert!(packet.contains("OPENAI_API_KEY=<redacted>"));
        assert!(packet.contains("path=<redacted-path>"));
        assert!(packet.contains("prompt=<redacted-payload>"));
        assert!(packet.contains("answer_text=<redacted-payload>"));
        assert!(packet.contains("hidden_cot=<redacted-payload>"));
        assert!(packet.contains("payload_line=<redacted-payload>"));
        assert!(packet.contains("plain <redacted> done"));
        assert!(!packet.contains("sk-leak"));
        assert!(!packet.contains("C:\\Users"));
        assert!(!packet.contains("AppData"));
        assert!(!packet.contains("private raw prompt"));
        assert!(!packet.contains("private hidden thoughts"));
        assert!(!packet.contains("raw answer"));
        assert!(!packet.contains("Design a Rust Noiron prototype"));
        assert!(!packet.contains("reuse_response"));
        assert!(!packet.contains("ghp_alsoleak"));
    }

    #[test]
    fn parses_research_sandbox_input_arg() {
        let config = parse_evidence_packet_args([
            "--issue",
            "62",
            "--commit",
            "abc123",
            "--command",
            "cargo test",
            "--gate",
            "passed",
            "--input",
            "evidence.txt",
            "--research-sandbox-input",
            "sandbox.txt",
        ])
        .expect("expected research sandbox input arg");

        assert_eq!(
            config.research_sandbox_input,
            Some(PathBuf::from("sandbox.txt"))
        );
    }

    #[test]
    fn release_review_statement_derives_blockers_from_input_rows() {
        let path = std::env::temp_dir().join(format!(
            "norion-cli-release-review-{}.txt",
            std::process::id()
        ));
        fs::write(
            &path,
            "pr=428 review=REVIEW_REQUIRED checks=passed branch_protection=present\npr=#429 review=APPROVED checks=failed branch_protection=missing\n",
        )
        .unwrap();

        let statement = release_review_statement(&path).unwrap();

        assert!(statement.contains("rc_prs=#428,#429"));
        assert!(statement.contains("rc_prs_source=release_review_input"));
        assert!(statement.contains("release_relevant_prs=#428,#429"));
        assert!(statement.contains("release_review_ready=false"));
        assert!(statement.contains(
            "release_review_blockers=#428:REVIEW_REQUIRED,#429:CHECKS_FAILED,#429:MISSING_BRANCH_PROTECTION_EVIDENCE"
        ));
        assert!(statement.contains("release_review_source=release_review_input"));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn issue_state_statement_derives_closeout_blockers_from_input_rows() {
        let path =
            std::env::temp_dir().join(format!("norion-cli-issue-state-{}.txt", std::process::id()));
        fs::write(
            &path,
            "issue=31 state=open final_signoff=false\nissue=19 state=open runtime_surface_closed=false runtime_surface_merged_prs=#290,#291 runtime_counters_pr=#429 runtime_counters_head=3c471cac3f7f6b218ade3473b9b29493917e7313 runtime_counters_checks=green runtime_counters_review=merged runtime_counters_merged=true runtime_surface_blocker=#19:OPEN\nissue=30 state=open close_allowed=false\n",
        )
        .unwrap();

        let statement = issue_state_statement(&path).unwrap();

        assert!(statement.contains("issue31_final_signoff_present=false"));
        assert!(statement.contains("issue31_final_signoff_source=issue_state_input"));
        assert!(statement.contains("issue19_runtime_surface_closed=false"));
        assert!(statement.contains("issue19_runtime_surface_merged_prs=#290,#291"));
        assert!(statement.contains("issue19_runtime_counters_pr=#429"));
        assert!(statement.contains("issue19_runtime_counters_ready=true"));
        assert!(
            statement.contains("issue19_runtime_counters_ready_source=issue_state_input_derived")
        );
        assert!(
            statement
                .contains("issue19_runtime_counters_state=head_3c471ca_checks_green_merged_merged")
        );
        assert!(
            statement.contains("issue19_runtime_counters_state_source=issue_state_input_derived")
        );
        assert!(statement.contains("issue19_runtime_surface_blocker=#19:OPEN"));
        assert!(statement.contains("issue19_runtime_surface_source=issue_state_input"));
        assert!(statement.contains("issue30_close_allowed=false"));
        assert!(statement.contains("issue30_close_allowed_source=issue_state_input"));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn issue_state_statement_rejects_raw_runtime_counter_ready_conflict() {
        let path = std::env::temp_dir().join(format!(
            "norion-cli-issue-state-conflict-{}.txt",
            std::process::id()
        ));
        fs::write(
            &path,
            "issue=31 state=open final_signoff=false\nissue=19 state=open runtime_surface_closed=false runtime_surface_merged_prs=#290,#291 runtime_counters_pr=#429 runtime_counters_ready=false runtime_counters_head=3c471cac3f7f6b218ade3473b9b29493917e7313 runtime_counters_checks=green runtime_counters_review=merged runtime_counters_merged=true runtime_surface_blocker=#19:OPEN\nissue=30 state=open close_allowed=false\n",
        )
        .unwrap();

        let error = issue_state_statement(&path).unwrap_err();

        assert!(
            error.contains("runtime_counters_ready conflicts with checks/review/merged fields")
        );

        let _ = fs::remove_file(path);
    }

    #[test]
    fn demo_proof_statement_derives_issue30_demo_fields() {
        let path =
            std::env::temp_dir().join(format!("norion-cli-demo-proof-{}.txt", std::process::id()));
        fs::write(
            &path,
            "clean_checkout=true live_model_required=false private_state_required=false prompt_digest_ref=redaction-digest:issue30-default-prompt integration_test=issue30_clean_checkout_demo_writes_digest_only_evidence_packet dispatch_test=issue30_dispatch_roundtrip_inspect_runs_trace_schema_gate dispatch_path=dispatch::run trace_schema_gate_executed=true\n",
        )
        .unwrap();

        let statement = demo_proof_statement(&path).unwrap();

        assert!(statement.contains("clean_checkout=true"));
        assert!(statement.contains("live_model_required=false"));
        assert!(statement.contains("private_state_required=false"));
        assert!(statement.contains("prompt_digest_ref=redaction-digest:issue30-default-prompt"));
        assert!(statement.contains(
            "issue30_demo_integration_test=issue30_clean_checkout_demo_writes_digest_only_evidence_packet"
        ));
        assert!(statement.contains(
            "issue30_demo_dispatch_test=issue30_dispatch_roundtrip_inspect_runs_trace_schema_gate"
        ));
        assert!(statement.contains("issue30_demo_dispatch_path=dispatch::run"));
        assert!(statement.contains("issue30_demo_trace_schema_gate_executed=true"));
        assert!(statement.contains("issue30_clean_checkout_demo_ready=true"));
        assert!(
            statement.contains("issue30_clean_checkout_demo_ready_source=demo_proof_input_derived")
        );
        assert!(statement.contains("issue30_demo_source=demo_proof_input"));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn roundtrip_proof_statement_preserves_summary_line_and_marks_source() {
        let path = std::env::temp_dir().join(format!(
            "norion-cli-roundtrip-proof-{}.txt",
            std::process::id()
        ));
        fs::write(
            &path,
            "persistent_roundtrip: passed=true first_disk_kv_reopen_verified=true second_imported_runtime_kv_blocks=1 second_imported_runtime_kv_from_namespace=true second_runtime_kv_disk_rehydrated=true second_kvswap_boundary_verified=true second_compute_budget_saved_tokens=320 second_compute_budget_avoided_tokens=448 second_planning_dense_compute_avoided_tokens=448 second_compute_budget_kv_lookups_skipped=2 second_compute_budget_anchor_count=2 second_compute_budget_anchors_preserved_count=2 second_approved_experience_reuse_digest=redaction-digest:abcdef0123456789 negative_unauthorized_write_allowed=false negative_memory_write_allowed=false negative_genome_write_allowed=false negative_self_evolution_write_allowed=false negative_polluted_evidence_quarantined=true negative_bad_candidate_digest=redaction-digest:fedcba9876543210 negative_bad_candidate_decision=hold_then_rollback negative_rollback_anchor_present=true negative_rollback_anchor_digest=redaction-digest:0123456789abcdef negative_tenant_scope_write_denied=true negative_tenant_scope_mode=local_single_user_preview negative_tenant_scope_actor=fnv64:1111111111111111 negative_tenant_scope_target=fnv64:2222222222222222 negative_tenant_scope_denial_lane=self_evolving_memory negative_tenant_scope_denial_reason=cross_tenant_scope_rejected negative_provenance_license_redaction_passed=true failures=0\n",
        )
        .unwrap();

        let statement = roundtrip_proof_statement(&path).unwrap();

        assert!(statement.contains("persistent_roundtrip: passed=true"));
        assert!(statement.contains("second_compute_budget_saved_tokens=320"));
        assert!(statement.contains("second_compute_budget_reduced=true"));
        assert!(
            statement
                .contains("second_compute_budget_reduced_source=roundtrip_proof_input_derived")
        );
        assert!(statement.contains("second_planning_dense_compute_avoided_tokens=448"));
        assert!(statement.contains("second_planning_dense_compute_reduced=true"));
        assert!(statement.contains(
            "second_planning_dense_compute_reduced_source=roundtrip_proof_input_derived"
        ));
        assert!(statement.contains("second_compute_budget_anchors_preserved=true"));
        assert!(statement.contains(
            "second_compute_budget_anchors_preserved_source=roundtrip_proof_input_derived"
        ));
        assert!(statement.contains("issue30_second_task_benefit_ready=true"));
        assert!(
            statement
                .contains("issue30_second_task_benefit_ready_source=roundtrip_proof_input_derived")
        );
        assert!(statement.contains("issue30_disk_kv_roundtrip_ready=true"));
        assert!(
            statement
                .contains("issue30_disk_kv_roundtrip_ready_source=roundtrip_proof_input_derived")
        );
        assert!(statement.contains("negative_unauthorized_write_allowed=false"));
        assert!(statement.contains("negative_durable_write_allowed=false"));
        assert!(
            statement
                .contains("negative_durable_write_allowed_source=roundtrip_proof_input_derived")
        );
        assert!(statement.contains("negative_all_writes_denied=true"));
        assert!(
            statement.contains("negative_all_writes_denied_source=roundtrip_proof_input_derived")
        );
        assert!(statement.contains("negative_polluted_evidence_contained=true"));
        assert!(
            statement.contains(
                "negative_polluted_evidence_contained_source=roundtrip_proof_input_derived"
            )
        );
        assert!(statement.contains("negative_bad_candidate_held_or_rolled_back=true"));
        assert!(statement.contains(
            "negative_bad_candidate_held_or_rolled_back_source=roundtrip_proof_input_derived"
        ));
        assert!(statement.contains("negative_single_tenant_preview=true"));
        assert!(
            statement
                .contains("negative_single_tenant_preview_source=roundtrip_proof_input_derived")
        );
        assert!(statement.contains("negative_tenant_scope_boundary_ok=true"));
        assert!(
            statement
                .contains("negative_tenant_scope_boundary_ok_source=roundtrip_proof_input_derived")
        );
        assert!(statement.contains("negative_digest_only=true"));
        assert!(statement.contains("negative_digest_only_source=roundtrip_proof_input_derived"));
        assert!(statement.contains("issue30_negative_gates_ready=true"));
        assert!(
            statement.contains("issue30_negative_gates_ready_source=roundtrip_proof_input_derived")
        );
        assert!(statement.contains("failures=0"));
        assert!(statement.contains("issue30_roundtrip_source=roundtrip_proof_input"));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn roundtrip_proof_requires_bound_tenant_scope_evidence() {
        let path = std::env::temp_dir().join(format!(
            "norion-cli-weak-tenant-boundary-{}.txt",
            std::process::id()
        ));
        fs::write(
            &path,
            "persistent_roundtrip: passed=true second_compute_budget_saved_tokens=320 second_compute_budget_avoided_tokens=448 second_planning_dense_compute_avoided_tokens=448 second_compute_budget_kv_lookups_skipped=2 second_compute_budget_anchor_count=2 second_compute_budget_anchors_preserved_count=2 second_approved_experience_reuse_digest=redaction-digest:abcdef0123456789 negative_unauthorized_write_allowed=false negative_memory_write_allowed=false negative_genome_write_allowed=false negative_self_evolution_write_allowed=false negative_polluted_evidence_quarantined=true negative_bad_candidate_digest=redaction-digest:fedcba9876543210 negative_bad_candidate_decision=hold_then_rollback negative_rollback_anchor_present=true negative_rollback_anchor_digest=redaction-digest:0123456789abcdef negative_tenant_scope_write_denied=true negative_tenant_scope_mode=local_single_user_preview negative_tenant_scope_actor=fnv64:1111111111111111 negative_tenant_scope_target=fnv64:1111111111111111 negative_tenant_scope_denial_lane=kv_memory negative_tenant_scope_denial_reason=missing negative_provenance_license_redaction_passed=true failures=0\n",
        )
        .unwrap();

        let statement = roundtrip_proof_statement(&path).unwrap();

        assert!(statement.contains("negative_tenant_scope_boundary_ok=false"));
        assert!(statement.contains("issue30_negative_gates_ready=false"));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn trace_report_statement_derives_issue30_counters_from_report() {
        let path = std::env::temp_dir().join(format!(
            "norion-cli-trace-report-{}.txt",
            std::process::id()
        ));
        fs::write(
            &path,
            "trace_schema_gate: passed=true lines=12 failures=0 reasoning_genome_events=2 reasoning_genome_write_allowed=0 reasoning_genome_splice_write_allowed=0 self_evolution_admission_events=1 self_evolution_admission_review_packets=1 self_evolution_admission_evidence_ids=3 self_evolution_admission_missing_review_packet_refs=0 memory_admission_events=1 memory_admission_ledger_records=3 memory_admission_ledger_authorized=0 memory_admission_ledger_applied=0 memory_admission_ledger_preview_only=1 memory_admission_admitted=1 memory_admission_hold=1 memory_admission_reject=1 memory_admission_ledger_held=1 memory_admission_ledger_rejected=1 memory_admission_ledger_duplicate=1 memory_admission_ledger_decayed=1 memory_admission_ledger_merged=0 memory_admission_ledger_rollback=1 memory_admission_read_only=1 memory_admission_write_allowed=0 memory_admission_applied=0 disk_kv_compact_reopen_verified=true disk_kv_compact_reopen_test=disk_kv::tests::compact_keeps_latest_values memory_admission_ledger_reopen_verified=true memory_admission_ledger_reopen_test=memory_admission::tests::writer_gate_append_is_idempotent_after_store_reopen\n",
        )
        .unwrap();

        let statement = trace_report_statement(&path).unwrap();

        assert_eq!(
            statement,
            "trace_schema_gate: passed=true reasoning_genome_events=2 reasoning_genome_write_allowed=0 reasoning_genome_splice_write_allowed=0 self_evolution_admission_events=1 self_evolution_admission_review_packets=1 self_evolution_admission_evidence_ids=3 self_evolution_admission_missing_review_packet_refs=0 memory_admission_events=1 memory_admission_candidates=0 memory_admission_ledger_records=3 memory_admission_ledger_authorized=0 memory_admission_ledger_applied=0 memory_admission_ledger_preview_only=1 memory_admission_admitted=1 memory_admission_hold=1 memory_admission_reject=1 memory_admission_ledger_held=1 memory_admission_ledger_rejected=1 memory_admission_ledger_duplicate=1 memory_admission_ledger_decayed=1 memory_admission_ledger_merged=0 memory_admission_ledger_rollback=1 memory_admission_source_semantic=0 memory_admission_source_gist=0 memory_admission_source_runtime_kv=0 memory_admission_source_cold=0 memory_admission_source_gene_segment=0 memory_admission_gene_segment_metadata=0 memory_admission_read_only=1 memory_admission_write_allowed=0 memory_admission_applied=0 disk_kv_compact_reopen_verified=true disk_kv_compact_reopen_test=disk_kv::tests::compact_keeps_latest_values memory_admission_ledger_reopen_verified=true memory_admission_ledger_reopen_test=memory_admission::tests::writer_gate_append_is_idempotent_after_store_reopen self_evolution_admission_review_complete=true self_evolution_admission_review_complete_source=trace_report_input_derived issue2_memory_admission_preview_apply_proof=true issue2_memory_admission_preview_apply_proof_source=trace_report_input_derived issue2_memory_ledger_apply_proof=true issue2_memory_ledger_apply_proof_source=trace_report_input_derived issue2_memory_ledger_lifecycle_retention_proof=true issue2_memory_ledger_lifecycle_retention_proof_source=trace_report_input_derived issue30_memory_ledger_trace_ready=true issue30_memory_ledger_trace_ready_source=trace_report_input_derived issue30_trace_validation_ready=true issue30_trace_validation_ready_source=trace_report_input_derived trace_report_source=trace_report_input"
        );

        let _ = fs::remove_file(path);
    }

    #[test]
    fn trace_report_statement_derives_issue185_agent_team_route_ready() {
        let path = std::env::temp_dir().join(format!(
            "norion-cli-trace-report-agent-team-{}.txt",
            std::process::id()
        ));
        fs::write(
            &path,
            "trace_schema_gate: passed=true lines=1 failures=0 reasoning_genome_events=2 reasoning_genome_write_allowed=0 reasoning_genome_splice_write_allowed=0 self_evolution_admission_events=1 self_evolution_admission_review_packets=1 self_evolution_admission_evidence_ids=3 self_evolution_admission_missing_review_packet_refs=0 memory_admission_events=1 memory_admission_ledger_records=3 memory_admission_ledger_authorized=0 memory_admission_ledger_applied=0 memory_admission_ledger_preview_only=1 memory_admission_admitted=1 memory_admission_hold=1 memory_admission_reject=1 memory_admission_ledger_held=1 memory_admission_ledger_rejected=1 memory_admission_ledger_duplicate=1 memory_admission_ledger_decayed=1 memory_admission_ledger_merged=0 memory_admission_ledger_rollback=1 memory_admission_read_only=1 memory_admission_write_allowed=0 memory_admission_applied=0 disk_kv_compact_reopen_verified=true disk_kv_compact_reopen_test=disk_kv::tests::compact_keeps_latest_values memory_admission_ledger_reopen_verified=true memory_admission_ledger_reopen_test=memory_admission::tests::writer_gate_append_is_idempotent_after_store_reopen agent_team_events=1 agent_team_enabled=1 agent_team_layer_b_route_proof_ready=1 agent_team_layer_b_route_complete=1\n",
        )
        .unwrap();

        let statement = trace_report_statement(&path).unwrap();

        assert!(statement.contains("agent_team_events=1"));
        assert!(statement.contains("agent_team_enabled=1"));
        assert!(statement.contains("agent_team_layer_b_route_proof_ready=1"));
        assert!(statement.contains("agent_team_layer_b_route_complete=1"));
        assert!(statement.contains("issue185_agent_team_layer_b_route_ready=true"));
        assert!(
            statement.contains(
                "issue185_agent_team_layer_b_route_ready_source=trace_report_input_derived"
            )
        );

        let _ = fs::remove_file(path);
    }

    #[test]
    fn trace_report_statement_derives_issue185_coding_service_eval_self_validation_ready() {
        let path = std::env::temp_dir().join(format!(
            "norion-cli-trace-report-coding-service-eval-{}.txt",
            std::process::id()
        ));
        fs::write(
            &path,
            format!(
                "{} coding_service_eval_events=1 coding_service_eval_readiness_events=0 coding_service_eval_runner_events=1 coding_service_eval_passed=1 coding_service_eval_requests=5 coding_service_eval_completed=5 coding_service_eval_evidence_packets=5 coding_service_eval_rust_validation_checked=2 coding_service_eval_compile_checked=2 coding_service_eval_unit_test_checked=2 coding_service_eval_benchmark_checked=5 coding_service_eval_benchmark_passed=5 coding_service_eval_layer_b_route_proof_ready=5 coding_service_eval_rust_validation_layer_b_route_ready=2 coding_service_eval_write_allowed=0 coding_service_eval_applied=0\n",
                minimal_trace_report_line()
            ),
        )
        .unwrap();

        let statement = trace_report_statement(&path).unwrap();

        assert!(statement.contains("coding_service_eval_runner_events=1"));
        assert!(statement.contains("coding_service_eval_benchmark_checked=5"));
        assert!(statement.contains("coding_service_eval_benchmark_passed=5"));
        assert!(statement.contains("coding_service_eval_layer_b_route_proof_ready=5"));
        assert!(statement.contains("coding_service_eval_rust_validation_layer_b_route_ready=2"));
        assert!(statement.contains("issue185_coding_service_eval_self_validation_ready=true"));
        assert!(statement.contains(
            "issue185_coding_service_eval_self_validation_ready_source=trace_report_input_derived"
        ));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn trace_report_statement_derives_issue185_agent_team_contract_ready() {
        let path = std::env::temp_dir().join(format!(
            "norion-cli-trace-report-agent-team-contract-{}.txt",
            std::process::id()
        ));
        fs::write(
            &path,
            format!(
                "{} {}\n",
                minimal_trace_report_line(),
                issue185_agent_team_contract_fields()
            ),
        )
        .unwrap();

        let statement = trace_report_statement(&path).unwrap();

        assert!(statement.contains("agent_team_agents=7"));
        assert!(statement.contains("agent_team_aggregation_lanes=7"));
        assert!(statement.contains("agent_team_conflicts=1"));
        assert!(statement.contains("agent_team_unresolved_conflicts=0"));
        assert!(statement.contains("issue185_agent_team_contract_ready=true"));
        assert!(
            statement
                .contains("issue185_agent_team_contract_ready_source=trace_report_input_derived")
        );
        let _ = fs::remove_file(path);
    }

    #[test]
    fn trace_report_statement_rejects_issue185_agent_team_contract_conflict() {
        let path = std::env::temp_dir().join(format!(
            "norion-cli-trace-report-agent-team-contract-conflict-{}.txt",
            std::process::id()
        ));
        fs::write(
            &path,
            format!(
                "{} {} issue185_agent_team_contract_ready=false\n",
                minimal_trace_report_line(),
                issue185_agent_team_contract_fields()
            ),
        )
        .unwrap();

        let error = trace_report_statement(&path).unwrap_err();

        assert!(error.contains("issue185_agent_team_contract_ready conflicts"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn trace_report_statement_rejects_issue185_coding_service_eval_self_validation_conflict() {
        let path = std::env::temp_dir().join(format!(
            "norion-cli-trace-report-coding-service-eval-conflict-{}.txt",
            std::process::id()
        ));
        fs::write(
            &path,
            format!(
                "{} coding_service_eval_events=1 coding_service_eval_readiness_events=0 coding_service_eval_runner_events=1 coding_service_eval_passed=1 coding_service_eval_requests=5 coding_service_eval_completed=5 coding_service_eval_evidence_packets=5 coding_service_eval_rust_validation_checked=2 coding_service_eval_compile_checked=2 coding_service_eval_unit_test_checked=2 coding_service_eval_benchmark_checked=5 coding_service_eval_benchmark_passed=5 coding_service_eval_layer_b_route_proof_ready=5 coding_service_eval_rust_validation_layer_b_route_ready=2 coding_service_eval_write_allowed=0 coding_service_eval_applied=0 issue185_coding_service_eval_self_validation_ready=false\n",
                minimal_trace_report_line()
            ),
        )
        .unwrap();

        let error = trace_report_statement(&path).unwrap_err();

        assert!(error.contains("issue185_coding_service_eval_self_validation_ready conflicts"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn trace_report_statement_derives_issue185_agent_tooling_mvp_ready() {
        let path = std::env::temp_dir().join(format!(
            "norion-cli-trace-report-agent-tooling-mvp-{}.txt",
            std::process::id()
        ));
        fs::write(
            &path,
            format!(
                "{} {} coding_service_eval_events=1 coding_service_eval_readiness_events=0 coding_service_eval_runner_events=1 coding_service_eval_passed=1 coding_service_eval_requests=5 coding_service_eval_completed=5 coding_service_eval_evidence_packets=5 coding_service_eval_rust_validation_checked=2 coding_service_eval_compile_checked=2 coding_service_eval_unit_test_checked=2 coding_service_eval_benchmark_checked=5 coding_service_eval_benchmark_passed=5 coding_service_eval_layer_b_route_proof_ready=5 coding_service_eval_rust_validation_layer_b_route_ready=2 coding_service_eval_write_allowed=0 coding_service_eval_applied=0\n",
                minimal_trace_report_line(),
                issue185_agent_team_contract_fields()
            ),
        )
        .unwrap();

        let statement = trace_report_statement(&path).unwrap();

        assert!(statement.contains("issue185_agent_team_layer_b_route_ready=true"));
        assert!(statement.contains("issue185_agent_team_contract_ready=true"));
        assert!(statement.contains("issue185_coding_service_eval_self_validation_ready=true"));
        assert!(statement.contains("issue185_agent_tooling_mvp_ready=true"));
        assert!(
            statement
                .contains("issue185_agent_tooling_mvp_ready_source=trace_report_input_derived")
        );
        let _ = fs::remove_file(path);
    }

    #[test]
    fn trace_report_statement_rejects_issue185_agent_tooling_mvp_conflict() {
        let path = std::env::temp_dir().join(format!(
            "norion-cli-trace-report-agent-tooling-mvp-conflict-{}.txt",
            std::process::id()
        ));
        fs::write(
            &path,
            format!(
                "{} {} coding_service_eval_events=1 coding_service_eval_readiness_events=0 coding_service_eval_runner_events=1 coding_service_eval_passed=1 coding_service_eval_requests=5 coding_service_eval_completed=5 coding_service_eval_evidence_packets=5 coding_service_eval_rust_validation_checked=2 coding_service_eval_compile_checked=2 coding_service_eval_unit_test_checked=2 coding_service_eval_benchmark_checked=5 coding_service_eval_benchmark_passed=5 coding_service_eval_layer_b_route_proof_ready=5 coding_service_eval_rust_validation_layer_b_route_ready=2 coding_service_eval_write_allowed=0 coding_service_eval_applied=0 issue185_agent_tooling_mvp_ready=false\n",
                minimal_trace_report_line(),
                issue185_agent_team_contract_fields()
            ),
        )
        .unwrap();

        let error = trace_report_statement(&path).unwrap_err();

        assert!(error.contains("issue185_agent_tooling_mvp_ready conflicts"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn trace_report_statement_derives_issue503_chaperone_fold_guard_ready() {
        let path = std::env::temp_dir().join(format!(
            "norion-cli-trace-report-chaperone-{}.txt",
            std::process::id()
        ));
        fs::write(
            &path,
            format!(
                "{} issue503_chaperone_fold_guard_verified=true issue503_fold_status=repair issue503_undefined_capability_count=1 issue503_contradiction_count=1 issue503_ungated_side_effect_count=1 issue503_missing_evidence_count=1 issue503_repair_task_count=1 issue503_raw_cot_captured=false issue503_raw_prompt_captured=false\n",
                minimal_trace_report_line()
            ),
        )
        .unwrap();

        let statement = trace_report_statement(&path).unwrap();

        assert!(statement.contains("issue503_fold_status=repair"));
        assert!(statement.contains("issue503_raw_cot_captured=false"));
        assert!(statement.contains("issue503_chaperone_fold_guard_ready=true"));
        assert!(
            statement
                .contains("issue503_chaperone_fold_guard_ready_source=trace_report_input_derived")
        );
        let _ = fs::remove_file(path);
    }

    #[test]
    fn trace_report_statement_rejects_issue503_raw_cot_ready_conflict() {
        let path = std::env::temp_dir().join(format!(
            "norion-cli-trace-report-chaperone-conflict-{}.txt",
            std::process::id()
        ));
        fs::write(
            &path,
            format!(
                "{} issue503_fold_status=repair issue503_undefined_capability_count=1 issue503_contradiction_count=0 issue503_ungated_side_effect_count=0 issue503_missing_evidence_count=0 issue503_repair_task_count=1 issue503_raw_cot_captured=true issue503_raw_prompt_captured=false issue503_chaperone_fold_guard_ready=true\n",
                minimal_trace_report_line()
            ),
        )
        .unwrap();

        let error = trace_report_statement(&path).unwrap_err();

        assert!(error.contains("issue503_chaperone_fold_guard_ready conflicts"));
        let _ = fs::remove_file(path);
    }

    fn minimal_trace_report_line() -> &'static str {
        "trace_schema_gate: passed=true reasoning_genome_events=2 reasoning_genome_write_allowed=0 reasoning_genome_splice_write_allowed=0 self_evolution_admission_events=1 self_evolution_admission_review_packets=1 self_evolution_admission_evidence_ids=3 self_evolution_admission_missing_review_packet_refs=0 memory_admission_events=1 memory_admission_candidates=0 memory_admission_ledger_records=3 memory_admission_ledger_authorized=0 memory_admission_ledger_applied=0 memory_admission_ledger_preview_only=1 memory_admission_admitted=1 memory_admission_hold=1 memory_admission_reject=1 memory_admission_ledger_held=1 memory_admission_ledger_rejected=1 memory_admission_ledger_duplicate=1 memory_admission_ledger_decayed=1 memory_admission_ledger_merged=0 memory_admission_ledger_rollback=1 memory_admission_source_semantic=0 memory_admission_source_gist=0 memory_admission_source_runtime_kv=0 memory_admission_source_cold=0 memory_admission_source_gene_segment=0 memory_admission_read_only=1 memory_admission_write_allowed=0 memory_admission_applied=0 disk_kv_compact_reopen_verified=true disk_kv_compact_reopen_test=disk_kv::tests::compact_keeps_latest_values memory_admission_ledger_reopen_verified=true memory_admission_ledger_reopen_test=memory_admission::tests::writer_gate_append_is_idempotent_after_store_reopen"
    }

    fn issue185_agent_team_contract_fields() -> &'static str {
        "agent_team_events=1 agent_team_enabled=1 agent_team_layer_b_route_proof_ready=1 agent_team_layer_b_route_complete=1 agent_team_agents=7 agent_team_messages=7 agent_team_aggregation_lanes=7 agent_team_aggregation_messages=7 agent_team_conflicts=1 agent_team_unresolved_conflicts=0 agent_team_collision_free=1 agent_team_single_writer=1 agent_team_read_only_subagents=1 agent_team_budget_isolated=1 agent_team_main_thread_writer=1"
    }

    #[test]
    fn trace_report_statement_derives_issue243_control_expression_from_trace_report() {
        let path = std::env::temp_dir().join(format!(
            "norion-cli-trace-report-control-expression-{}.txt",
            std::process::id()
        ));
        fs::write(
            &path,
            format!(
                "{} control_expression_active_control_knobs=routing|context_anchor|suppression|checkpoint|memory_maintenance control_expression_evidence_digest=redaction-digest:control243 control_expression_policy_version=control_expression_gate_v1 control_expression_decision_reason=no_weight_runtime_control_preview control_expression_profile_selected=1 control_expression_context_anchor_promoted=1 control_expression_suppression_gate_triggered=1 control_expression_checkpoint_repair_requested=1 control_expression_checkpoint_rejected=1 control_expression_memory_refresh_candidate=1 control_expression_memory_tombstone_candidate=1 control_expression_preview_admission=1 control_expression_write_allowed=0 control_expression_applied=0 control_expression_operator_approval_required=1\n",
                minimal_trace_report_line()
            ),
        )
        .unwrap();

        let statement = trace_report_statement(&path).unwrap();

        assert!(statement.contains("issue243_control_expression_gate_ready=true"));
        assert!(
            statement.contains(
                "issue243_control_expression_gate_ready_source=trace_report_input_derived"
            )
        );

        let _ = fs::remove_file(path);
    }

    #[test]
    fn trace_report_statement_derives_memory_admission_source_mix_proof() {
        let path = std::env::temp_dir().join(format!(
            "norion-cli-trace-report-source-mix-{}.txt",
            std::process::id()
        ));
        fs::write(
            &path,
            "trace_schema_gate: passed=true lines=12 failures=0 reasoning_genome_events=2 reasoning_genome_write_allowed=0 reasoning_genome_splice_write_allowed=0 self_evolution_admission_events=1 self_evolution_admission_review_packets=1 self_evolution_admission_evidence_ids=3 self_evolution_admission_missing_review_packet_refs=0 memory_admission_events=1 memory_admission_candidates=5 memory_admission_ledger_records=5 memory_admission_ledger_authorized=0 memory_admission_ledger_applied=0 memory_admission_ledger_preview_only=1 memory_admission_admitted=0 memory_admission_hold=1 memory_admission_reject=1 memory_admission_ledger_held=1 memory_admission_ledger_rejected=1 memory_admission_ledger_duplicate=1 memory_admission_ledger_decayed=1 memory_admission_ledger_merged=1 memory_admission_ledger_rollback=1 memory_admission_source_semantic=1 memory_admission_source_gist=1 memory_admission_source_runtime_kv=1 memory_admission_source_cold=1 memory_admission_source_gene_segment=1 memory_admission_gene_segment_metadata=1 memory_admission_read_only=1 memory_admission_write_allowed=0 memory_admission_applied=0 disk_kv_compact_reopen_verified=true disk_kv_compact_reopen_test=disk_kv::tests::compact_keeps_latest_values memory_admission_ledger_reopen_verified=true memory_admission_ledger_reopen_test=memory_admission::tests::writer_gate_append_is_idempotent_after_store_reopen\n",
        )
        .unwrap();

        let statement = trace_report_statement(&path).unwrap();

        assert!(statement.contains("memory_admission_source_semantic=1"));
        assert!(statement.contains("memory_admission_source_gist=1"));
        assert!(statement.contains("memory_admission_source_runtime_kv=1"));
        assert!(statement.contains("memory_admission_source_cold=1"));
        assert!(statement.contains("memory_admission_source_gene_segment=1"));
        assert!(statement.contains("memory_admission_gene_segment_metadata=1"));
        assert!(statement.contains("memory_admission_source_total=5"));
        assert!(statement.contains("issue2_memory_admission_source_mix_proof=true"));
        assert!(statement.contains(
            "issue2_memory_admission_source_mix_proof_source=trace_report_input_derived"
        ));
        assert!(statement.contains("issue2_memory_gene_segment_metadata_proof=true"));
        assert!(statement.contains(
            "issue2_memory_gene_segment_metadata_proof_source=trace_report_input_derived"
        ));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn trace_report_statement_requires_each_memory_admission_source_for_mix_proof() {
        let path = std::env::temp_dir().join(format!(
            "norion-cli-trace-report-source-mix-missing-{}.txt",
            std::process::id()
        ));
        fs::write(
            &path,
            "trace_schema_gate: passed=true lines=12 failures=0 reasoning_genome_events=2 reasoning_genome_write_allowed=0 reasoning_genome_splice_write_allowed=0 self_evolution_admission_events=1 self_evolution_admission_review_packets=1 self_evolution_admission_evidence_ids=3 self_evolution_admission_missing_review_packet_refs=0 memory_admission_events=1 memory_admission_candidates=4 memory_admission_ledger_records=4 memory_admission_ledger_authorized=0 memory_admission_ledger_applied=0 memory_admission_ledger_preview_only=1 memory_admission_admitted=0 memory_admission_hold=1 memory_admission_reject=1 memory_admission_ledger_held=1 memory_admission_ledger_rejected=1 memory_admission_ledger_duplicate=1 memory_admission_ledger_decayed=1 memory_admission_ledger_merged=0 memory_admission_ledger_rollback=1 memory_admission_source_semantic=1 memory_admission_source_gist=1 memory_admission_source_runtime_kv=1 memory_admission_source_cold=0 memory_admission_source_gene_segment=1 memory_admission_read_only=1 memory_admission_write_allowed=0 memory_admission_applied=0 disk_kv_compact_reopen_verified=true disk_kv_compact_reopen_test=disk_kv::tests::compact_keeps_latest_values memory_admission_ledger_reopen_verified=true memory_admission_ledger_reopen_test=memory_admission::tests::writer_gate_append_is_idempotent_after_store_reopen\n",
        )
        .unwrap();

        let statement = trace_report_statement(&path).unwrap();

        assert!(statement.contains("memory_admission_source_total=4"));
        assert!(statement.contains("issue2_memory_admission_source_mix_proof=false"));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn trace_report_statement_derives_authorized_fixture_apply_proof() {
        let path = std::env::temp_dir().join(format!(
            "norion-cli-trace-report-authorized-fixture-{}.txt",
            std::process::id()
        ));
        fs::write(
            &path,
            "trace_schema_gate: passed=true lines=12 failures=0 reasoning_genome_events=2 reasoning_genome_write_allowed=0 reasoning_genome_splice_write_allowed=0 self_evolution_admission_events=1 self_evolution_admission_review_packets=1 self_evolution_admission_evidence_ids=3 self_evolution_admission_missing_review_packet_refs=0 memory_admission_events=1 memory_admission_ledger_records=3 memory_admission_ledger_authorized=0 memory_admission_ledger_applied=0 memory_admission_ledger_preview_only=1 memory_admission_admitted=1 memory_admission_hold=1 memory_admission_reject=1 memory_admission_ledger_held=1 memory_admission_ledger_rejected=1 memory_admission_ledger_duplicate=1 memory_admission_ledger_decayed=1 memory_admission_ledger_merged=0 memory_admission_ledger_rollback=1 memory_admission_read_only=1 memory_admission_write_allowed=0 memory_admission_applied=0 disk_kv_compact_reopen_verified=true disk_kv_compact_reopen_test=disk_kv::tests::compact_keeps_latest_values memory_admission_ledger_reopen_verified=true memory_admission_ledger_reopen_test=memory_admission::tests::writer_gate_append_is_idempotent_after_store_reopen memory_admission_authorized_fixture_apply_verified=true memory_admission_authorized_fixture_apply_test=memory_admission::tests::writer_gate_rehydrates_applied_authorized_records_from_existing_ledger memory_admission_authorized_fixture_authorized=1 memory_admission_authorized_fixture_applied=1 memory_admission_authorized_fixture_admitted=1 memory_admission_authorized_fixture_rehydrated=1 memory_admission_authorized_fixture_reopened_records=1 memory_admission_authorized_fixture_ledger_bytes_nonzero=true\n",
        )
        .unwrap();

        let statement = trace_report_statement(&path).unwrap();

        assert!(statement.contains("memory_admission_authorized_fixture_apply_verified=true"));
        assert!(statement.contains(
            "memory_admission_authorized_fixture_apply_test=memory_admission::tests::writer_gate_rehydrates_applied_authorized_records_from_existing_ledger"
        ));
        assert!(statement.contains("memory_admission_authorized_fixture_authorized=1"));
        assert!(statement.contains("memory_admission_authorized_fixture_applied=1"));
        assert!(statement.contains("memory_admission_authorized_fixture_admitted=1"));
        assert!(statement.contains("memory_admission_authorized_fixture_rehydrated=1"));
        assert!(statement.contains("memory_admission_authorized_fixture_reopened_records=1"));
        assert!(
            statement.contains("memory_admission_authorized_fixture_ledger_bytes_nonzero=true")
        );
        assert!(statement.contains("issue2_memory_authorized_fixture_apply_proof=true"));
        assert!(statement.contains(
            "issue2_memory_authorized_fixture_apply_proof_source=trace_report_input_derived"
        ));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn trace_report_statement_derives_memory_residency_retention_compaction_proof() {
        let path = std::env::temp_dir().join(format!(
            "norion-cli-trace-report-memory-residency-{}.txt",
            std::process::id()
        ));
        fs::write(
            &path,
            "trace_schema_gate: passed=true lines=12 failures=0 reasoning_genome_events=2 reasoning_genome_write_allowed=0 reasoning_genome_splice_write_allowed=0 self_evolution_admission_events=1 self_evolution_admission_review_packets=1 self_evolution_admission_evidence_ids=3 self_evolution_admission_missing_review_packet_refs=0 memory_admission_events=1 memory_admission_ledger_records=4 memory_admission_ledger_authorized=0 memory_admission_ledger_applied=0 memory_admission_ledger_preview_only=1 memory_admission_admitted=1 memory_admission_hold=1 memory_admission_reject=1 memory_admission_ledger_held=1 memory_admission_ledger_rejected=1 memory_admission_ledger_duplicate=1 memory_admission_ledger_decayed=1 memory_admission_ledger_merged=1 memory_admission_ledger_rollback=1 memory_admission_read_only=1 memory_admission_write_allowed=0 memory_admission_applied=0 disk_kv_compact_reopen_verified=true disk_kv_compact_reopen_test=disk_kv::tests::compact_keeps_latest_values memory_admission_ledger_reopen_verified=true memory_admission_ledger_reopen_test=memory_admission::tests::writer_gate_append_is_idempotent_after_store_reopen memory_retention_activity_cases=1 memory_retention_decayed=1 memory_retention_removed=1 memory_compaction_activity_cases=1 memory_compaction_merged=1 memory_compaction_removed=1 memory_compaction_pair_evidence=1 memory_storage_samples=1 memory_storage_entries_before=4 memory_storage_entries_after=2 memory_storage_entries_removed=2 memory_storage_reduction_entries=2 memory_retained_usefulness_abs_delta_milli=100\n",
        )
        .unwrap();

        let statement = trace_report_statement(&path).unwrap();

        assert!(statement.contains("memory_retention_activity_cases=1"));
        assert!(statement.contains("memory_storage_reduction_entries=2"));
        assert!(statement.contains("memory_retained_usefulness_abs_delta_milli=100"));
        assert!(statement.contains("issue2_memory_residency_retention_compaction_proof=true"));
        assert!(statement.contains(
            "issue2_memory_residency_retention_compaction_proof_source=trace_report_input_derived"
        ));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn trace_report_statement_derives_memory_autophagy_preview_proof() {
        let path = std::env::temp_dir().join(format!(
            "norion-cli-trace-report-memory-autophagy-{}.txt",
            std::process::id()
        ));
        fs::write(
            &path,
            "trace_schema_gate: passed=true lines=12 failures=0 reasoning_genome_events=2 reasoning_genome_write_allowed=0 reasoning_genome_splice_write_allowed=0 self_evolution_admission_events=1 self_evolution_admission_review_packets=1 self_evolution_admission_evidence_ids=3 self_evolution_admission_missing_review_packet_refs=0 memory_admission_events=1 memory_admission_ledger_records=4 memory_admission_ledger_authorized=0 memory_admission_ledger_applied=0 memory_admission_ledger_preview_only=1 memory_admission_admitted=1 memory_admission_hold=1 memory_admission_reject=1 memory_admission_ledger_held=1 memory_admission_ledger_rejected=1 memory_admission_ledger_duplicate=1 memory_admission_ledger_decayed=1 memory_admission_ledger_merged=1 memory_admission_ledger_rollback=1 memory_admission_read_only=1 memory_admission_write_allowed=0 memory_admission_applied=0 disk_kv_compact_reopen_verified=true disk_kv_compact_reopen_test=disk_kv::tests::compact_keeps_latest_values memory_admission_ledger_reopen_verified=true memory_admission_ledger_reopen_test=memory_admission::tests::writer_gate_append_is_idempotent_after_store_reopen memory_autophagy_context_pressure_score=115 memory_autophagy_retrieval_noise_score=10 memory_autophagy_stale_decay_candidates=1 memory_autophagy_duplicate_merge_candidates=1 memory_autophagy_gist_recomposition_candidates=2 memory_autophagy_active_recall_prune_candidates=5 memory_autophagy_quarantine_candidates=3 memory_autophagy_live_delete_allowed=false memory_autophagy_durable_mutation_allowed=false memory_autophagy_reason_codes=active_recall_prune_preview|gist_recomposition_preview|quarantine_preview|recycle_preview\n",
        )
        .unwrap();

        let statement = trace_report_statement(&path).unwrap();

        assert!(statement.contains("memory_autophagy_context_pressure_score=115"));
        assert!(statement.contains("memory_autophagy_gist_recomposition_candidates=2"));
        assert!(statement.contains("memory_autophagy_live_delete_allowed=false"));
        assert!(statement.contains("memory_autophagy_durable_mutation_allowed=false"));
        assert!(statement.contains("issue499_memory_autophagy_preview_proof=true"));
        assert!(
            statement.contains(
                "issue499_memory_autophagy_preview_proof_source=trace_report_input_derived"
            )
        );
        assert!(!statement.contains("memory_autophagy_detail_codes="));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn trace_report_statement_rejects_autophagy_count_mismatch() {
        let path = std::env::temp_dir().join(format!(
            "norion-cli-trace-report-memory-autophagy-mismatch-{}.txt",
            std::process::id()
        ));
        fs::write(
            &path,
            "trace_schema_gate: passed=true lines=12 failures=0 reasoning_genome_events=2 reasoning_genome_write_allowed=0 reasoning_genome_splice_write_allowed=0 self_evolution_admission_events=1 self_evolution_admission_review_packets=1 self_evolution_admission_evidence_ids=3 self_evolution_admission_missing_review_packet_refs=0 memory_admission_events=1 memory_admission_ledger_records=4 memory_admission_ledger_authorized=0 memory_admission_ledger_applied=0 memory_admission_ledger_preview_only=1 memory_admission_admitted=1 memory_admission_hold=1 memory_admission_reject=1 memory_admission_ledger_held=1 memory_admission_ledger_rejected=1 memory_admission_ledger_duplicate=1 memory_admission_ledger_decayed=1 memory_admission_ledger_merged=1 memory_admission_ledger_rollback=1 memory_admission_read_only=1 memory_admission_write_allowed=0 memory_admission_applied=0 disk_kv_compact_reopen_verified=true disk_kv_compact_reopen_test=disk_kv::tests::compact_keeps_latest_values memory_admission_ledger_reopen_verified=true memory_admission_ledger_reopen_test=memory_admission::tests::writer_gate_append_is_idempotent_after_store_reopen memory_autophagy_context_pressure_score=115 memory_autophagy_retrieval_noise_score=10 memory_autophagy_stale_decay_candidates=1 memory_autophagy_duplicate_merge_candidates=1 memory_autophagy_gist_recomposition_candidates=9 memory_autophagy_active_recall_prune_candidates=5 memory_autophagy_quarantine_candidates=3 memory_autophagy_live_delete_allowed=false memory_autophagy_durable_mutation_allowed=false memory_autophagy_reason_codes=active_recall_prune_preview|gist_recomposition_preview issue499_memory_autophagy_preview_proof=true\n",
        )
        .unwrap();

        let error = trace_report_statement(&path).unwrap_err();

        assert!(error.contains(
            "memory_autophagy_gist_recomposition_candidates conflicts with stale/duplicate counts"
        ));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn trace_report_statement_rejects_autophagy_live_delete_flag() {
        let path = std::env::temp_dir().join(format!(
            "norion-cli-trace-report-memory-autophagy-live-delete-{}.txt",
            std::process::id()
        ));
        fs::write(
            &path,
            "trace_schema_gate: passed=true lines=12 failures=0 reasoning_genome_events=2 reasoning_genome_write_allowed=0 reasoning_genome_splice_write_allowed=0 self_evolution_admission_events=1 self_evolution_admission_review_packets=1 self_evolution_admission_evidence_ids=3 self_evolution_admission_missing_review_packet_refs=0 memory_admission_events=1 memory_admission_ledger_records=4 memory_admission_ledger_authorized=0 memory_admission_ledger_applied=0 memory_admission_ledger_preview_only=1 memory_admission_admitted=1 memory_admission_hold=1 memory_admission_reject=1 memory_admission_ledger_held=1 memory_admission_ledger_rejected=1 memory_admission_ledger_duplicate=1 memory_admission_ledger_decayed=1 memory_admission_ledger_merged=1 memory_admission_ledger_rollback=1 memory_admission_read_only=1 memory_admission_write_allowed=0 memory_admission_applied=0 disk_kv_compact_reopen_verified=true disk_kv_compact_reopen_test=disk_kv::tests::compact_keeps_latest_values memory_admission_ledger_reopen_verified=true memory_admission_ledger_reopen_test=memory_admission::tests::writer_gate_append_is_idempotent_after_store_reopen memory_autophagy_context_pressure_score=115 memory_autophagy_retrieval_noise_score=10 memory_autophagy_stale_decay_candidates=1 memory_autophagy_duplicate_merge_candidates=1 memory_autophagy_gist_recomposition_candidates=2 memory_autophagy_active_recall_prune_candidates=5 memory_autophagy_quarantine_candidates=3 memory_autophagy_live_delete_allowed=true memory_autophagy_durable_mutation_allowed=false memory_autophagy_reason_codes=active_recall_prune_preview|gist_recomposition_preview issue499_memory_autophagy_preview_proof=true\n",
        )
        .unwrap();

        let error = trace_report_statement(&path).unwrap_err();

        assert!(error.contains("memory_autophagy_live_delete_allowed must stay false"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn trace_report_statement_derives_runtime_preview_apply_proof() {
        let path = std::env::temp_dir().join(format!(
            "norion-cli-trace-report-runtime-preview-apply-{}.txt",
            std::process::id()
        ));
        fs::write(
            &path,
            "trace_schema_gate: passed=true lines=12 failures=0 reasoning_genome_events=2 reasoning_genome_write_allowed=0 reasoning_genome_splice_write_allowed=0 self_evolution_admission_events=1 self_evolution_admission_review_packets=1 self_evolution_admission_evidence_ids=3 self_evolution_admission_missing_review_packet_refs=0 memory_admission_events=1 memory_admission_ledger_records=3 memory_admission_ledger_authorized=0 memory_admission_ledger_applied=0 memory_admission_ledger_preview_only=1 memory_admission_admitted=1 memory_admission_hold=1 memory_admission_reject=1 memory_admission_ledger_held=1 memory_admission_ledger_rejected=1 memory_admission_ledger_duplicate=1 memory_admission_ledger_decayed=1 memory_admission_ledger_merged=0 memory_admission_ledger_rollback=1 memory_admission_read_only=1 memory_admission_write_allowed=0 memory_admission_applied=0 disk_kv_compact_reopen_verified=true disk_kv_compact_reopen_test=disk_kv::tests::compact_keeps_latest_values memory_admission_ledger_reopen_verified=true memory_admission_ledger_reopen_test=memory_admission::tests::writer_gate_append_is_idempotent_after_store_reopen memory_admission_runtime_preview_apply_verified=true memory_admission_runtime_preview_apply_test=tests::benchmark_state::runtime_memory_admission_preview_applies_after_approved_writer_policy memory_admission_runtime_preview_authorized=10 memory_admission_runtime_preview_applied=10 memory_admission_runtime_preview_admitted=10 memory_admission_runtime_preview_rehydrated=10\n",
        )
        .unwrap();

        let statement = trace_report_statement(&path).unwrap();

        assert!(statement.contains("memory_admission_runtime_preview_apply_verified=true"));
        assert!(statement.contains(
            "memory_admission_runtime_preview_apply_test=tests::benchmark_state::runtime_memory_admission_preview_applies_after_approved_writer_policy"
        ));
        assert!(statement.contains("memory_admission_runtime_preview_authorized=10"));
        assert!(statement.contains("memory_admission_runtime_preview_applied=10"));
        assert!(statement.contains("memory_admission_runtime_preview_admitted=10"));
        assert!(statement.contains("memory_admission_runtime_preview_rehydrated=10"));
        assert!(statement.contains("issue2_memory_runtime_preview_apply_proof=true"));
        assert!(statement.contains(
            "issue2_memory_runtime_preview_apply_proof_source=trace_report_input_derived"
        ));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn trace_report_statement_derives_read_only_authorized_append_denial_proof() {
        let path = std::env::temp_dir().join(format!(
            "norion-cli-trace-report-read-only-authorized-append-{}.txt",
            std::process::id()
        ));
        fs::write(
            &path,
            "trace_schema_gate: passed=true lines=12 failures=0 reasoning_genome_events=2 reasoning_genome_write_allowed=0 reasoning_genome_splice_write_allowed=0 self_evolution_admission_events=1 self_evolution_admission_review_packets=1 self_evolution_admission_evidence_ids=3 self_evolution_admission_missing_review_packet_refs=0 memory_admission_events=1 memory_admission_ledger_records=3 memory_admission_ledger_authorized=0 memory_admission_ledger_applied=0 memory_admission_ledger_preview_only=1 memory_admission_admitted=1 memory_admission_hold=1 memory_admission_reject=1 memory_admission_ledger_held=1 memory_admission_ledger_rejected=1 memory_admission_ledger_duplicate=1 memory_admission_ledger_decayed=1 memory_admission_ledger_merged=0 memory_admission_ledger_rollback=1 memory_admission_read_only=1 memory_admission_write_allowed=0 memory_admission_applied=0 disk_kv_compact_reopen_verified=true disk_kv_compact_reopen_test=disk_kv::tests::compact_keeps_latest_values memory_admission_ledger_reopen_verified=true memory_admission_ledger_reopen_test=memory_admission::tests::writer_gate_append_is_idempotent_after_store_reopen memory_admission_read_only_authorized_append_denied=true memory_admission_read_only_authorized_append_test=memory_admission::tests::writer_gate_refuses_authorized_append_on_read_only_store memory_admission_read_only_authorized_append_preserved_existing_bytes=true\n",
        )
        .unwrap();

        let statement = trace_report_statement(&path).unwrap();

        assert!(statement.contains("memory_admission_read_only_authorized_append_denied=true"));
        assert!(statement.contains(
            "memory_admission_read_only_authorized_append_test=memory_admission::tests::writer_gate_refuses_authorized_append_on_read_only_store"
        ));
        assert!(statement.contains(
            "memory_admission_read_only_authorized_append_preserved_existing_bytes=true"
        ));
        assert!(statement.contains("issue2_memory_read_only_authorized_append_denial_proof=true"));
        assert!(statement.contains(
            "issue2_memory_read_only_authorized_append_denial_proof_source=trace_report_input_derived"
        ));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn trace_report_statement_derives_review_scope_required_proof() {
        let path = std::env::temp_dir().join(format!(
            "norion-cli-trace-report-review-scope-required-{}.txt",
            std::process::id()
        ));
        fs::write(
            &path,
            "trace_schema_gate: passed=true lines=12 failures=0 reasoning_genome_events=2 reasoning_genome_write_allowed=0 reasoning_genome_splice_write_allowed=0 self_evolution_admission_events=1 self_evolution_admission_review_packets=1 self_evolution_admission_evidence_ids=3 self_evolution_admission_missing_review_packet_refs=0 memory_admission_events=1 memory_admission_ledger_records=3 memory_admission_ledger_authorized=0 memory_admission_ledger_applied=0 memory_admission_ledger_preview_only=1 memory_admission_admitted=1 memory_admission_hold=1 memory_admission_reject=1 memory_admission_ledger_held=1 memory_admission_ledger_rejected=1 memory_admission_ledger_duplicate=1 memory_admission_ledger_decayed=1 memory_admission_ledger_merged=0 memory_admission_ledger_rollback=1 memory_admission_read_only=1 memory_admission_write_allowed=0 memory_admission_applied=0 disk_kv_compact_reopen_verified=true disk_kv_compact_reopen_test=disk_kv::tests::compact_keeps_latest_values memory_admission_ledger_reopen_verified=true memory_admission_ledger_reopen_test=memory_admission::tests::writer_gate_append_is_idempotent_after_store_reopen memory_admission_review_scope_required_verified=true memory_admission_review_scope_required_test=memory_admission::tests::gene_segment_kv_writer_gate_rejects_missing_review_scope_digests memory_admission_review_scope_required_tenant_rejection=review_packet_tenant_scope_digest_missing memory_admission_review_scope_required_session_rejection=review_packet_session_scope_digest_missing memory_admission_review_scope_required_authorized=0 memory_admission_review_scope_required_appended=0\n",
        )
        .unwrap();

        let statement = trace_report_statement(&path).unwrap();

        assert!(statement.contains("memory_admission_review_scope_required_verified=true"));
        assert!(statement.contains(
            "memory_admission_review_scope_required_test=memory_admission::tests::gene_segment_kv_writer_gate_rejects_missing_review_scope_digests"
        ));
        assert!(statement.contains(
            "memory_admission_review_scope_required_tenant_rejection=review_packet_tenant_scope_digest_missing"
        ));
        assert!(statement.contains(
            "memory_admission_review_scope_required_session_rejection=review_packet_session_scope_digest_missing"
        ));
        assert!(statement.contains("memory_admission_review_scope_required_authorized=0"));
        assert!(statement.contains("memory_admission_review_scope_required_appended=0"));
        assert!(statement.contains("issue2_memory_review_scope_required_proof=true"));
        assert!(statement.contains(
            "issue2_memory_review_scope_required_proof_source=trace_report_input_derived"
        ));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn trace_report_statement_derives_issue37_runtime_recall_scope_ready() {
        let path = std::env::temp_dir().join(format!(
            "norion-cli-trace-report-issue37-scope-{}.txt",
            std::process::id()
        ));
        fs::write(
            &path,
            "trace_schema_gate: passed=true lines=12 failures=0 reasoning_genome_events=2 reasoning_genome_write_allowed=0 reasoning_genome_splice_write_allowed=0 self_evolution_admission_events=1 self_evolution_admission_review_packets=1 self_evolution_admission_evidence_ids=3 self_evolution_admission_missing_review_packet_refs=0 memory_admission_events=1 memory_admission_ledger_records=3 memory_admission_ledger_authorized=0 memory_admission_ledger_applied=0 memory_admission_ledger_preview_only=1 memory_admission_admitted=1 memory_admission_hold=1 memory_admission_reject=1 memory_admission_ledger_held=1 memory_admission_ledger_rejected=1 memory_admission_ledger_duplicate=1 memory_admission_ledger_decayed=1 memory_admission_ledger_merged=0 memory_admission_ledger_rollback=1 memory_admission_read_only=1 memory_admission_write_allowed=0 memory_admission_applied=0 disk_kv_compact_reopen_verified=true disk_kv_compact_reopen_test=disk_kv::tests::compact_keeps_latest_values memory_admission_ledger_reopen_verified=true memory_admission_ledger_reopen_test=memory_admission::tests::writer_gate_append_is_idempotent_after_store_reopen issue37_runtime_recall_scope_verified=true issue37_runtime_recall_scope_test=engine::tests::runtime_memory::inference_request_default_scope_isolates_runtime_memory_and_experience\n",
        )
        .unwrap();

        let statement = trace_report_statement(&path).unwrap();

        assert!(statement.contains("issue37_runtime_recall_scope_verified=true"));
        assert!(statement.contains(
            "issue37_runtime_recall_scope_test=engine::tests::runtime_memory::inference_request_default_scope_isolates_runtime_memory_and_experience"
        ));
        assert!(statement.contains("issue37_runtime_recall_scope_ready=true"));
        assert!(
            statement
                .contains("issue37_runtime_recall_scope_ready_source=trace_report_input_derived")
        );

        let _ = fs::remove_file(path);
    }

    #[test]
    fn trace_report_statement_derives_invalid_shape_rejection_proof() {
        let path = std::env::temp_dir().join(format!(
            "norion-cli-trace-report-invalid-shape-{}.txt",
            std::process::id()
        ));
        fs::write(
            &path,
            "trace_schema_gate: passed=true lines=12 failures=0 reasoning_genome_events=2 reasoning_genome_write_allowed=0 reasoning_genome_splice_write_allowed=0 self_evolution_admission_events=1 self_evolution_admission_review_packets=1 self_evolution_admission_evidence_ids=3 self_evolution_admission_missing_review_packet_refs=0 memory_admission_events=1 memory_admission_ledger_records=3 memory_admission_ledger_authorized=0 memory_admission_ledger_applied=0 memory_admission_ledger_preview_only=1 memory_admission_admitted=1 memory_admission_hold=1 memory_admission_reject=1 memory_admission_ledger_held=1 memory_admission_ledger_rejected=1 memory_admission_ledger_duplicate=1 memory_admission_ledger_decayed=1 memory_admission_ledger_merged=0 memory_admission_ledger_rollback=1 memory_admission_read_only=1 memory_admission_write_allowed=0 memory_admission_applied=0 disk_kv_compact_reopen_verified=true disk_kv_compact_reopen_test=disk_kv::tests::compact_keeps_latest_values memory_admission_ledger_reopen_verified=true memory_admission_ledger_reopen_test=memory_admission::tests::writer_gate_append_is_idempotent_after_store_reopen memory_admission_invalid_shape_rejection_verified=true memory_admission_invalid_shape_rejection_test=memory_admission::tests::gene_segment_kv_records_reject_invalid_shape_without_write memory_admission_invalid_shape_source_hash_present=false memory_admission_invalid_shape_kv_shape_valid=false memory_admission_invalid_shape_ledger_rejected=1 memory_admission_invalid_shape_ledger_authorized=0 memory_admission_invalid_shape_preview_read_only=true memory_admission_invalid_shape_preview_write_allowed=false\n",
        )
        .unwrap();

        let statement = trace_report_statement(&path).unwrap();

        assert!(statement.contains("memory_admission_invalid_shape_rejection_verified=true"));
        assert!(statement.contains(
            "memory_admission_invalid_shape_rejection_test=memory_admission::tests::gene_segment_kv_records_reject_invalid_shape_without_write"
        ));
        assert!(statement.contains("memory_admission_invalid_shape_source_hash_present=false"));
        assert!(statement.contains("memory_admission_invalid_shape_kv_shape_valid=false"));
        assert!(statement.contains("memory_admission_invalid_shape_ledger_rejected=1"));
        assert!(statement.contains("memory_admission_invalid_shape_ledger_authorized=0"));
        assert!(statement.contains("memory_admission_invalid_shape_preview_read_only=true"));
        assert!(statement.contains("memory_admission_invalid_shape_preview_write_allowed=false"));
        assert!(statement.contains("issue2_memory_invalid_shape_rejection_proof=true"));
        assert!(statement.contains(
            "issue2_memory_invalid_shape_rejection_proof_source=trace_report_input_derived"
        ));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn trace_report_statement_rejects_ledger_ready_without_reopen_proof() {
        let path = std::env::temp_dir().join(format!(
            "norion-cli-trace-report-missing-reopen-proof-{}.txt",
            std::process::id()
        ));
        fs::write(
            &path,
            "trace_schema_gate: passed=true lines=12 failures=0 reasoning_genome_events=2 reasoning_genome_write_allowed=0 reasoning_genome_splice_write_allowed=0 self_evolution_admission_events=1 self_evolution_admission_review_packets=1 self_evolution_admission_evidence_ids=3 self_evolution_admission_missing_review_packet_refs=0 memory_admission_events=1 memory_admission_ledger_records=3 memory_admission_ledger_authorized=0 memory_admission_ledger_applied=0 memory_admission_ledger_preview_only=1 memory_admission_admitted=1 memory_admission_hold=1 memory_admission_reject=1 memory_admission_ledger_held=1 memory_admission_ledger_rejected=1 memory_admission_ledger_duplicate=1 memory_admission_ledger_decayed=1 memory_admission_ledger_merged=0 memory_admission_ledger_rollback=1 memory_admission_read_only=1 memory_admission_write_allowed=0 memory_admission_applied=0 disk_kv_compact_reopen_verified=false disk_kv_compact_reopen_test=disk_kv::tests::compact_keeps_latest_values memory_admission_ledger_reopen_verified=true memory_admission_ledger_reopen_test=memory_admission::tests::writer_gate_append_is_idempotent_after_store_reopen issue30_memory_ledger_trace_ready=true\n",
        )
        .unwrap();

        let error = trace_report_statement(&path).unwrap_err();

        assert!(error.contains(
            "issue30_memory_ledger_trace_ready conflicts with memory ledger reopen proof fields"
        ));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn trace_report_statement_rejects_conflicting_memory_ledger_apply_proof() {
        let path = std::env::temp_dir().join(format!(
            "norion-cli-trace-report-apply-conflict-{}.txt",
            std::process::id()
        ));
        fs::write(
            &path,
            "trace_schema_gate: passed=true lines=12 failures=0 reasoning_genome_events=2 reasoning_genome_write_allowed=0 reasoning_genome_splice_write_allowed=0 self_evolution_admission_events=1 self_evolution_admission_review_packets=1 self_evolution_admission_evidence_ids=3 self_evolution_admission_missing_review_packet_refs=0 memory_admission_events=1 memory_admission_ledger_records=3 memory_admission_ledger_authorized=1 memory_admission_ledger_applied=1 memory_admission_ledger_preview_only=1 memory_admission_admitted=1 memory_admission_hold=1 memory_admission_reject=1 memory_admission_ledger_held=1 memory_admission_ledger_rejected=1 memory_admission_ledger_duplicate=1 memory_admission_ledger_decayed=1 memory_admission_ledger_merged=0 memory_admission_ledger_rollback=1 memory_admission_read_only=1 memory_admission_write_allowed=0 memory_admission_applied=0 disk_kv_compact_reopen_verified=true disk_kv_compact_reopen_test=disk_kv::tests::compact_keeps_latest_values memory_admission_ledger_reopen_verified=true memory_admission_ledger_reopen_test=memory_admission::tests::writer_gate_append_is_idempotent_after_store_reopen issue2_memory_ledger_apply_proof=true\n",
        )
        .unwrap();

        let error = trace_report_statement(&path).unwrap_err();

        assert!(error.contains(
            "issue2_memory_ledger_apply_proof conflicts with authorized/applied counters"
        ));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn trace_report_statement_rejects_conflicting_lifecycle_retention_proof() {
        let path = std::env::temp_dir().join(format!(
            "norion-cli-trace-report-lifecycle-retention-conflict-{}.txt",
            std::process::id()
        ));
        fs::write(
            &path,
            "trace_schema_gate: passed=true lines=12 failures=0 reasoning_genome_events=2 reasoning_genome_write_allowed=0 reasoning_genome_splice_write_allowed=0 self_evolution_admission_events=1 self_evolution_admission_review_packets=1 self_evolution_admission_evidence_ids=3 self_evolution_admission_missing_review_packet_refs=0 memory_admission_events=1 memory_admission_ledger_records=1 memory_admission_ledger_authorized=0 memory_admission_ledger_applied=0 memory_admission_ledger_preview_only=1 memory_admission_admitted=1 memory_admission_hold=1 memory_admission_reject=1 memory_admission_ledger_held=1 memory_admission_ledger_rejected=1 memory_admission_ledger_duplicate=1 memory_admission_ledger_decayed=0 memory_admission_ledger_merged=0 memory_admission_ledger_rollback=1 memory_admission_read_only=1 memory_admission_write_allowed=0 memory_admission_applied=0 disk_kv_compact_reopen_verified=true disk_kv_compact_reopen_test=disk_kv::tests::compact_keeps_latest_values memory_admission_ledger_reopen_verified=true memory_admission_ledger_reopen_test=memory_admission::tests::writer_gate_append_is_idempotent_after_store_reopen issue2_memory_ledger_lifecycle_retention_proof=true\n",
        )
        .unwrap();

        let error = trace_report_statement(&path).unwrap_err();

        assert!(error.contains(
            "issue2_memory_ledger_lifecycle_retention_proof conflicts with lifecycle counters"
        ));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn trace_report_statement_rejects_conflicting_memory_residency_retention_compaction_proof() {
        let path = std::env::temp_dir().join(format!(
            "norion-cli-trace-report-memory-residency-conflict-{}.txt",
            std::process::id()
        ));
        fs::write(
            &path,
            "trace_schema_gate: passed=true lines=12 failures=0 reasoning_genome_events=2 reasoning_genome_write_allowed=0 reasoning_genome_splice_write_allowed=0 self_evolution_admission_events=1 self_evolution_admission_review_packets=1 self_evolution_admission_evidence_ids=3 self_evolution_admission_missing_review_packet_refs=0 memory_admission_events=1 memory_admission_ledger_records=4 memory_admission_ledger_authorized=0 memory_admission_ledger_applied=0 memory_admission_ledger_preview_only=1 memory_admission_admitted=1 memory_admission_hold=1 memory_admission_reject=1 memory_admission_ledger_held=1 memory_admission_ledger_rejected=1 memory_admission_ledger_duplicate=1 memory_admission_ledger_decayed=1 memory_admission_ledger_merged=1 memory_admission_ledger_rollback=1 memory_admission_read_only=1 memory_admission_write_allowed=0 memory_admission_applied=0 disk_kv_compact_reopen_verified=true disk_kv_compact_reopen_test=disk_kv::tests::compact_keeps_latest_values memory_admission_ledger_reopen_verified=true memory_admission_ledger_reopen_test=memory_admission::tests::writer_gate_append_is_idempotent_after_store_reopen memory_retention_activity_cases=1 memory_retention_decayed=1 memory_retention_removed=1 memory_compaction_activity_cases=1 memory_compaction_merged=1 memory_compaction_removed=1 memory_compaction_pair_evidence=1 memory_storage_samples=1 memory_storage_entries_before=4 memory_storage_entries_after=4 memory_storage_entries_removed=0 memory_storage_reduction_entries=0 memory_retained_usefulness_abs_delta_milli=100 issue2_memory_residency_retention_compaction_proof=true\n",
        )
        .unwrap();

        let error = trace_report_statement(&path).unwrap_err();

        assert!(error.contains(
            "issue2_memory_residency_retention_compaction_proof conflicts with memory residency counters"
        ));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn trace_report_statement_rejects_conflicting_authorized_fixture_apply_proof() {
        let path = std::env::temp_dir().join(format!(
            "norion-cli-trace-report-authorized-fixture-conflict-{}.txt",
            std::process::id()
        ));
        fs::write(
            &path,
            "trace_schema_gate: passed=true lines=12 failures=0 reasoning_genome_events=2 reasoning_genome_write_allowed=0 reasoning_genome_splice_write_allowed=0 self_evolution_admission_events=1 self_evolution_admission_review_packets=1 self_evolution_admission_evidence_ids=3 self_evolution_admission_missing_review_packet_refs=0 memory_admission_events=1 memory_admission_ledger_records=3 memory_admission_ledger_authorized=0 memory_admission_ledger_applied=0 memory_admission_ledger_preview_only=1 memory_admission_admitted=1 memory_admission_hold=1 memory_admission_reject=1 memory_admission_ledger_held=1 memory_admission_ledger_rejected=1 memory_admission_ledger_duplicate=1 memory_admission_ledger_decayed=1 memory_admission_ledger_merged=0 memory_admission_ledger_rollback=1 memory_admission_read_only=1 memory_admission_write_allowed=0 memory_admission_applied=0 disk_kv_compact_reopen_verified=true disk_kv_compact_reopen_test=disk_kv::tests::compact_keeps_latest_values memory_admission_ledger_reopen_verified=true memory_admission_ledger_reopen_test=memory_admission::tests::writer_gate_append_is_idempotent_after_store_reopen memory_admission_authorized_fixture_apply_verified=true memory_admission_authorized_fixture_apply_test=memory_admission::tests::writer_gate_rehydrates_applied_authorized_records_from_existing_ledger memory_admission_authorized_fixture_authorized=1 memory_admission_authorized_fixture_applied=1 memory_admission_authorized_fixture_admitted=1 memory_admission_authorized_fixture_rehydrated=0 memory_admission_authorized_fixture_reopened_records=1 memory_admission_authorized_fixture_ledger_bytes_nonzero=true issue2_memory_authorized_fixture_apply_proof=true\n",
        )
        .unwrap();

        let error = trace_report_statement(&path).unwrap_err();

        assert!(error.contains(
            "issue2_memory_authorized_fixture_apply_proof conflicts with authorized fixture apply fields"
        ));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn trace_report_statement_rejects_conflicting_runtime_preview_apply_proof() {
        let path = std::env::temp_dir().join(format!(
            "norion-cli-trace-report-runtime-preview-apply-conflict-{}.txt",
            std::process::id()
        ));
        fs::write(
            &path,
            "trace_schema_gate: passed=true lines=12 failures=0 reasoning_genome_events=2 reasoning_genome_write_allowed=0 reasoning_genome_splice_write_allowed=0 self_evolution_admission_events=1 self_evolution_admission_review_packets=1 self_evolution_admission_evidence_ids=3 self_evolution_admission_missing_review_packet_refs=0 memory_admission_events=1 memory_admission_ledger_records=3 memory_admission_ledger_authorized=1 memory_admission_ledger_applied=0 memory_admission_ledger_preview_only=1 memory_admission_admitted=1 memory_admission_hold=1 memory_admission_reject=1 memory_admission_ledger_held=1 memory_admission_ledger_rejected=1 memory_admission_ledger_duplicate=1 memory_admission_ledger_decayed=1 memory_admission_ledger_merged=0 memory_admission_ledger_rollback=1 memory_admission_read_only=1 memory_admission_write_allowed=0 memory_admission_applied=0 disk_kv_compact_reopen_verified=true disk_kv_compact_reopen_test=disk_kv::tests::compact_keeps_latest_values memory_admission_ledger_reopen_verified=true memory_admission_ledger_reopen_test=memory_admission::tests::writer_gate_append_is_idempotent_after_store_reopen memory_admission_runtime_preview_apply_verified=true memory_admission_runtime_preview_apply_test=tests::benchmark_state::runtime_memory_admission_preview_applies_after_approved_writer_policy memory_admission_runtime_preview_authorized=10 memory_admission_runtime_preview_applied=10 memory_admission_runtime_preview_admitted=10 memory_admission_runtime_preview_rehydrated=10 issue2_memory_runtime_preview_apply_proof=true\n",
        )
        .unwrap();

        let error = trace_report_statement(&path).unwrap_err();

        assert!(error.contains(
            "issue2_memory_runtime_preview_apply_proof conflicts with runtime preview apply fields"
        ));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn trace_report_statement_rejects_conflicting_memory_admission_preview_apply_proof() {
        let path = std::env::temp_dir().join(format!(
            "norion-cli-trace-report-preview-apply-conflict-{}.txt",
            std::process::id()
        ));
        fs::write(
            &path,
            "trace_schema_gate: passed=true lines=12 failures=0 reasoning_genome_events=2 reasoning_genome_write_allowed=0 reasoning_genome_splice_write_allowed=0 self_evolution_admission_events=1 self_evolution_admission_review_packets=1 self_evolution_admission_evidence_ids=3 self_evolution_admission_missing_review_packet_refs=0 memory_admission_events=1 memory_admission_ledger_records=3 memory_admission_ledger_authorized=0 memory_admission_ledger_applied=0 memory_admission_ledger_preview_only=1 memory_admission_admitted=1 memory_admission_hold=1 memory_admission_reject=1 memory_admission_ledger_held=1 memory_admission_ledger_rejected=1 memory_admission_ledger_duplicate=1 memory_admission_ledger_decayed=1 memory_admission_ledger_merged=0 memory_admission_ledger_rollback=1 memory_admission_read_only=0 memory_admission_write_allowed=1 memory_admission_applied=0 disk_kv_compact_reopen_verified=true disk_kv_compact_reopen_test=disk_kv::tests::compact_keeps_latest_values memory_admission_ledger_reopen_verified=true memory_admission_ledger_reopen_test=memory_admission::tests::writer_gate_append_is_idempotent_after_store_reopen issue2_memory_admission_preview_apply_proof=true\n",
        )
        .unwrap();

        let error = trace_report_statement(&path).unwrap_err();

        assert!(error.contains(
            "issue2_memory_admission_preview_apply_proof conflicts with memory admission preview apply fields"
        ));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn trace_report_statement_requires_memory_ledger_trace_proof() {
        let path = std::env::temp_dir().join(format!(
            "norion-cli-trace-report-missing-memory-ledger-{}.txt",
            std::process::id()
        ));
        fs::write(
            &path,
            "trace_schema_gate: passed=true lines=12 failures=0 reasoning_genome_events=2 reasoning_genome_write_allowed=0 reasoning_genome_splice_write_allowed=0 self_evolution_admission_events=1 self_evolution_admission_review_packets=1 self_evolution_admission_evidence_ids=3 self_evolution_admission_missing_review_packet_refs=0 memory_admission_events=1\n",
        )
        .unwrap();

        let error = trace_report_statement(&path).unwrap_err();

        assert!(error.contains("missing memory_admission_ledger_records"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn state_gate_statement_derives_gate_result_from_report() {
        let path =
            std::env::temp_dir().join(format!("norion-cli-state-gate-{}.txt", std::process::id()));
        fs::write(&path, "state_inspection_gate: passed=true failures=0\n").unwrap();

        let statement = state_gate_statement(&path).unwrap();

        assert_eq!(
            statement,
            "state_inspection_gate: passed=true failures=0 issue30_state_inspection_ready=true issue30_state_inspection_ready_source=state_gate_input_derived state_gate_source=state_gate_input"
        );

        let _ = fs::remove_file(path);
    }

    fn research_sandbox_line() -> &'static str {
        "research_sandbox_evidence schema=research_sandbox_evidence_v1 target=local profile=cpu-only noncommercial_only=true contributor_pr_only=true maintainer_approval_required=true persistent_state=disk_kv_cache|runtime_state|redacted_evidence_packets local_only_data=model_artifacts|raw_traces|secrets private_trace_publish_allowed=false redacted_issue_comment_ready=true wipe_test_state_supported=true preview_only=true write_allowed=false durable_write_allowed=false applied=false\n"
    }

    #[test]
    fn research_sandbox_statement_derives_issue_comment_safe() {
        let path = std::env::temp_dir().join(format!(
            "norion-cli-research-sandbox-{}.txt",
            std::process::id()
        ));
        fs::write(&path, research_sandbox_line()).unwrap();

        let statement = research_sandbox_statement(&path).unwrap();

        assert!(statement.contains("schema=research_sandbox_evidence_v1"));
        assert!(statement.contains("target=local"));
        assert!(statement.contains("profile=cpu-only"));
        assert!(statement.contains("research_sandbox_issue_comment_safe=true"));
        assert!(
            statement.contains(
                "research_sandbox_issue_comment_safe_source=research_sandbox_input_derived"
            )
        );
        assert!(statement.contains("research_sandbox_source=research_sandbox_input"));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn research_sandbox_statement_rejects_conflicting_safe_field() {
        let path = std::env::temp_dir().join(format!(
            "norion-cli-research-sandbox-conflict-{}.txt",
            std::process::id()
        ));
        fs::write(
            &path,
            research_sandbox_line().replace(
                "applied=false",
                "applied=false research_sandbox_issue_comment_safe=false",
            ),
        )
        .unwrap();

        let error = research_sandbox_statement(&path).unwrap_err();

        assert!(error.contains("research_sandbox_issue_comment_safe conflicts"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn evidence_packet_includes_research_sandbox_statement_and_redacts_raw_input() {
        let input = std::env::temp_dir().join(format!(
            "norion-cli-research-sandbox-packet-input-{}.txt",
            std::process::id()
        ));
        let sandbox = std::env::temp_dir().join(format!(
            "norion-cli-research-sandbox-packet-sandbox-{}.txt",
            std::process::id()
        ));
        fs::write(
            &input,
            "local_path=C:\\Users\\jy\\AppData\\Local\\Temp\\trace.txt\nsecret=sk-secret\n",
        )
        .unwrap();
        fs::write(&sandbox, research_sandbox_line()).unwrap();
        let config = EvidencePacketConfig {
            issue: "62".to_owned(),
            commit: "abc123".to_owned(),
            command: "cargo test --locked --package norion-cli research_sandbox".to_owned(),
            gate: "passed".to_owned(),
            input: input.clone(),
            output: None,
            git_worktree: None,
            release_review_input: None,
            issue_state_input: None,
            demo_proof_input: None,
            roundtrip_proof_input: None,
            trace_report_input: None,
            state_gate_input: None,
            research_sandbox_input: Some(sandbox.clone()),
            issue30_context_input: None,
            issue243_fixture_matrix_input: None,
            state_files_input: None,
            required: vec!["research_sandbox_issue_comment_safe=true".to_owned()],
            rejected: vec!["C:\\Users".to_owned(), "sk-secret".to_owned()],
        };

        let packet = run_evidence_packet(&config).unwrap();

        assert!(packet.contains("research_sandbox_evidence schema=research_sandbox_evidence_v1"));
        assert!(packet.contains("research_sandbox_issue_comment_safe=true"));
        assert!(packet.contains("local_path=<redacted-path>"));
        assert!(packet.contains("secret=<redacted>"));
        assert!(!packet.contains("C:\\Users"));
        assert!(!packet.contains("sk-secret"));

        let _ = fs::remove_file(input);
        let _ = fs::remove_file(sandbox);
    }

    #[test]
    fn issue30_context_statement_derives_context_rows_and_marks_source() {
        let path = std::env::temp_dir().join(format!(
            "norion-cli-issue30-context-{}.txt",
            std::process::id()
        ));
        fs::write(
            &path,
            "issue30_environment_pressure_present=true issue30_pollution_event_id=redaction-digest:dddddddddddddddd issue385_self_ontology_body_present=true issue385_body_state_id=redaction-digest:eeeeeeeeeeeeeeee issue385_pheromone_signal_marker_present=true issue385_pheromone_signal_marker_id=redaction-digest:9999999999999999 issue385_pheromone_signal_surface=digest_marker issue385_pheromone_signal_digest_gate_allowed=true issue385_pheromone_signal_preview_only=true issue375_pre_reasoning_genome_isa_present=true issue375_reasoning_frame_id=redaction-digest:ffffffffffffffff issue375_reasoning_frame_environment_signals_present=true issue375_reasoning_frame_allowed_observations=repo_issue_terminal_runtime_state issue375_reasoning_frame_action_vocab=observe_inspect_compare_summarize_verify_quarantine issue375_reasoning_frame_suppressed_capabilities=write_process_browser_network_memory_genome_runtime issue375_reasoning_frame_risk_limits=preview_only_digest_only issue375_expression_vm_side_effect=read_only issue375_genome_isa_apply_allowed=false issue30_backend_action=deterministic_runtime_kv_roundtrip issue4_dna_candidate_ledger_present=true issue4_dna_candidate_ledger_schema=dna_evolution_candidate_ledger_v1 issue4_dna_candidate_ledger_records=1 issue4_dna_candidate_ledger_candidate_count=1 issue4_dna_candidate_ledger_candidate_only=true issue4_dna_candidate_ledger_digest=redaction-digest:4444444444440004 issue4_dna_candidate_ledger_raw_records_allowed=false issue4_dna_candidate_ledger_write_allowed=false issue4_dna_candidate_ledger_applied=false issue4_dna_candidate_ledger_preview_source=entry_chain_dna_evolution_controller issue243_active_control_knobs=routing|context_anchor|suppression|checkpoint|memory_maintenance issue243_evidence_digest=redaction-digest:control243 issue243_policy_version=control_expression_gate_v1 issue243_decision_reason=no_weight_runtime_control_preview issue243_control_expression_profile_selected=1 issue243_context_anchor_promoted=1 issue243_suppression_gate_triggered=1 issue243_checkpoint_repair_requested=1 issue243_checkpoint_rejected=1 issue243_memory_refresh_candidate=1 issue243_memory_tombstone_candidate=1 issue243_control_expression_preview_admission=1 issue243_write_allowed=false issue243_applied=false issue243_operator_approval_required=true issue379_control_candidate_preview_only=true issue379_action_vocab_mask_preview=true issue379_signal_saliency_bias_preview=true issue379_zero_beat_primitive_decision_present=true issue379_primitive_authority=preview_only issue379_primitive_side_effect=read_only issue379_primitive_reversibility=rollback_required issue379_primitive_evidence=digest_only issue379_primitive_uncertainty=hold_on_gap issue379_primitive_attention=focus_or_mask_preview issue379_zero_beat_output=action_vocab_mask_and_signal_saliency_bias issue379_generation_bias_apply_allowed=false issue493_tool_organ_registry_present=true issue493_tool_organ_registry_id=redaction-digest:1111111111111111 issue493_tool_organ_registry_preview_only=true issue493_tool_organ_registry_side_effect=read_only issue493_tool_organ_registry_apply_allowed=false issue493_tool_organ_capability_matrix_digest=redaction-digest:2222222222222222 issue493_preview_bundle_protocol=bundle_v1 issue493_preview_bundle_digest=redaction-digest:3333333333333333 issue493_preview_bundle_refs_digest_only=true issue493_preview_bundle_raw_artifacts_allowed=false issue493_tool_install_allowed=false issue493_tool_execution_allowed=false bio_epigenetic_expression_marker_present=true bio_epigenetic_expression_marker_id=redaction-digest:4444444444444444 bio_mrna_cache_candidate_digest=redaction-digest:5555555555555555 bio_expression_cache_protocol=mrna_preview_v1 bio_expression_cache_key_digest=redaction-digest:6666666666666666 bio_hot_path_observation_window=100 bio_hot_path_min_success_rate=0.98 bio_gate_relaxation_allowed=false bio_cache_materialization_allowed=false bio_raw_payload_or_kv_cached=false bio_negative_evidence_overrides=true issue501_telomere_state_present=true issue501_remaining_tokens=0 issue501_remaining_steps=0 issue501_remaining_messages=0 issue501_repair_streak_count=2 issue501_loop_risk_signal_count=4 issue501_senescent=true issue501_apoptosis_required=true issue501_new_external_call_allowed=false issue501_new_file_write_allowed=false issue501_new_memory_write_allowed=false issue501_new_adaptive_state_write_allowed=false issue501_memory_promotion_allowed=false issue501_genome_mutation_allowed=false issue501_takeover_packet_digest=redaction-digest:7777777777777777 issue501_rollback_anchor_digest=redaction-digest:8888888888888888 issue501_handoff_next_owner=scheduler issue501_raw_payload_present=false issue501_preview_side_effect_allowed=false issue502_pheromone_blackboard_present=true issue502_signal_count=3 issue502_ranked_action_count=3 issue502_top_signal_kind=repair_first issue502_top_action=repair_review issue502_blackboard_digest=redaction-digest:9999999999999999 issue502_source_digest=redaction-digest:aaaaaaaaaaaaaaaa issue502_payload_digest=redaction-digest:bbbbbbbbbbbbbbbb issue502_raw_payload_present=false issue502_side_effect_allowed=false issue502_ttl_decay_present=true issue502_conflict_routes_to_repair=true issue502_ranked_actions_from_state_only=true issue509_quorum_sensing_present=true issue509_decision_id=redaction-digest:9999999999999509 issue509_quorum_report_digest=redaction-digest:aaaaaaaaaaaa0509 issue509_risk_class=irreversible issue509_required_quorum_milli=700 issue509_evaluator_count=3 issue509_independent_model_count=3 issue509_independent_lane_count=3 issue509_approve_signal_count=2 issue509_reject_signal_count=1 issue509_abstain_signal_count=0 issue509_approval_concentration_milli=666 issue509_conflict_count=1 issue509_quorum_reached=false issue509_apply_allowed=false issue509_raw_evaluator_payload_present=false issue509_duplicate_sources_count_once=true issue509_conflict_routes_to_repair=true issue509_writer_gate_bypass_allowed=false\nissue377_problem_finding_present=true issue377_problem_finding_id=redaction-digest:aaaaaaaaaaaaaaaa issue377_problem_finding_kind=wasted_compute issue377_problem_finding_severity=medium issue377_problem_finding_confidence_milli=850 issue377_problem_finding_evidence_digest=redaction-digest:7777777777773770 issue377_problem_finding_source_digest=redaction-digest:8888888888883770 issue377_problem_finding_affected_surface=runtime_kv_reuse issue377_problem_finding_next_step=experiment issue377_problem_finding_raw_payload_present=false issue377_self_observation_present=true issue377_self_observation_id=redaction-digest:1010101010103770 issue377_self_observation_schema=self_observation_v1 issue377_self_observation_signal_source=runtime_trace_metrics issue377_self_observation_source_digest=redaction-digest:8888888888883770 issue377_self_observation_window=second_task_roundtrip issue377_self_observation_current_truth_digest=redaction-digest:1212121212123770 issue377_self_observation_digest_only=true issue377_self_observation_raw_payload_present=false issue377_self_observation_write_allowed=false issue377_self_observation_applied=false issue377_self_model_present=true issue377_self_model_id=redaction-digest:1313131313133770 issue377_self_model_schema=control_plane_self_model_v1 issue377_self_model_scope=auditable_control_plane issue377_self_model_claims_consciousness=false issue377_self_model_digest_only=true issue377_self_model_raw_payload_present=false issue377_self_model_write_allowed=false issue377_self_model_applied=false issue377_hypothesis_candidate_present=true issue377_hypothesis_candidate_id=redaction-digest:bbbbbbbbbbbbbbbb issue377_hypothesis_candidate_kind=gene issue377_hypothesis_candidate_status=promoted_for_approval issue377_hypothesis_candidate_target_surface=reasoning_gene issue377_hypothesis_candidate_expected_metric=memory_reuse issue377_hypothesis_candidate_expected_direction=increase issue377_hypothesis_candidate_required_gates=trace_schema_gate|focused_tests|benchmark_gate issue377_hypothesis_candidate_rollback_anchor=redaction-digest:9999999999993770 issue377_hypothesis_candidate_raw_payload_present=false issue377_hypothesis_candidate_write_allowed=false issue377_hypothesis_candidate_applied=false issue377_hypothesis_candidate_operator_approval_required=true issue377_problem_hypothesis_link=redaction-digest:cccccccccccccccc issue377_admission_decision=preview_only issue377_lexicographic_admission_present=true issue377_lexicographic_admission_order=user_intent_preservation>safety>digest_only_evidence>rollback_anchor>quality_delta>cost_delta>latency_delta issue377_user_intent_preserved=true issue377_safety_gate_passed=true issue377_digest_only_evidence_gate_passed=true issue377_rollback_anchor_gate_passed=true issue377_quality_delta_milli=125 issue377_cost_delta_milli=-80 issue377_latency_delta_milli=-35 issue377_performance_tiebreaker_only=true issue377_hard_gate_failure_action=hold issue377_risk_override_action=hold issue377_negative_evidence_count=0 issue377_privacy_risk=low issue377_license_risk=low issue377_unsupported_capability_requested=false issue377_unsafe_side_effect_allowed=false issue377_risk_override_clear=true issue377_lexicographic_admission_apply_allowed=false issue377_best_next_state=problem_finding_preview issue377_best_next_state_id=redaction-digest:6666666666663770 issue377_best_next_state_selected=true issue377_predicament_signal_present=true issue377_predicament_id=redaction-digest:dddddddddddddddd issue377_predicament_progress_delta=0 issue377_predicament_repeat_count=2 issue377_predicament_evidence_gap_count=0 issue377_predicament_action_novelty=0 issue377_predicament_stuck=true issue377_self_trigger_stage=preview_only issue377_evolution_apply_allowed=false issue377_experiment_plan_present=true issue377_experiment_plan_id=redaction-digest:eeeeeeeeeeeeeeee issue377_experiment_plan_mode=preview_only issue377_experiment_plan_level_path=L0_schema_safety|L1_focused_validation|L3_benchmark issue377_validation_skipped_levels=L2_replay|L4_integration_readiness|L5_promotion_window issue377_validation_skipped_reason=minimal_existing_evidence_path issue377_human_apply_level=L6_human_apply issue377_human_apply_inside_engine=false issue377_validation_level_apply_allowed=false issue377_experiment_plan_required_gates=trace_schema_gate|focused_tests|benchmark_gate issue377_experiment_plan_budget_tokens=2048 issue377_experiment_plan_stop_on_fail=true issue377_experiment_plan_rollback_anchor=redaction-digest:9999999999993770 issue377_experiment_plan_raw_payload_present=false issue377_experiment_plan_write_allowed=false issue377_experiment_plan_applied=false issue377_evidence_bundle_present=true issue377_evidence_bundle_id=redaction-digest:ffffffffffffffff issue377_evidence_bundle_schema=evidence_bundle_v1 issue377_evidence_bundle_metric=memory_reuse issue377_evidence_bundle_direction=increase issue377_evidence_bundle_pass_count=3 issue377_evidence_bundle_fail_count=0 issue377_evidence_bundle_command_label=issue30_fresh_checkout_smoke issue377_evidence_bundle_refs_digest_only=true issue377_evidence_bundle_raw_payload_present=false issue377_evidence_bundle_write_allowed=false issue377_evidence_bundle_applied=false issue377_experiment_decision=promote_for_approval issue377_experiment_decision_schema=experiment_decision_v1 issue377_experiment_decision_reason=clean_evidence_bundle_promotes_preview issue377_experiment_decision_evidence_bundle_id=redaction-digest:ffffffffffffffff issue377_experiment_decision_target=mutation_candidate_emitter issue377_experiment_decision_manual_approval_required=true issue377_experiment_decision_apply_allowed=false issue377_experiment_runner_allowed=false issue377_experiment_apply_allowed=false issue377_mutation_candidate_emitter_present=true issue377_mutation_candidate_emitter_id=redaction-digest:1111111111113770 issue377_mutation_candidate_id=redaction-digest:2222222222223770 issue377_mutation_candidate_evidence_digest=redaction-digest:3333333333333770 issue377_mutation_candidate_rollback_anchor=redaction-digest:4444444444443770 issue377_mutation_candidate_requested_write_scope=reasoning_genome_preview issue377_mutation_candidate_kind=mutation_plan_preview issue377_mutation_candidate_preview_only=true issue377_mutation_candidate_refs_digest_only=true issue377_mutation_candidate_writer_gate_preflight=hold issue377_mutation_candidate_write_allowed=false issue377_mutation_candidate_applied=false issue377_mutation_candidate_apply_allowed=false issue377_mutation_candidate_manual_review_required=true issue377_candidate_emitter_lane_coverage=reasoning_genome_preview|memory_admission_preview|routing_policy_preview|tool_policy_preview|evolution_goal_preview issue377_candidate_emitter_kind_coverage=mutation_plan_preview|memory_admission_preview|routing_shadow_proposal|tool_policy_candidate|evolution_goal_preview issue377_candidate_emitter_coverage_count=5 issue377_candidate_emitter_all_preview_only=true issue377_candidate_emitter_all_write_allowed=false issue377_candidate_emitter_all_apply_allowed=false issue377_candidate_emitter_all_manual_review_required=true issue377_candidate_emitter_durable_preflight_owner=unified_writer_gate issue377_candidate_emitter_writer_gate_bypass_allowed=false issue377_candidate_emitter_direct_durable_write_allowed=false issue377_candidate_emitter_ready_for_explicit_apply=false issue377_related_issue_refs=#6|#7|#74|#79|#375 issue377_related_issue_scope_map=#6:experiment_gates|#7:memory_admission_pipeline|#74:thinking_scheduler|#79:evolution_goal_queue|#375:pre_reasoning_genome_isa issue377_related_issue_owner_scope=meta_cognitive_evolution_loop issue377_related_issue_non_duplicate_count=5 issue377_related_issue_all_non_duplicate=true issue377_related_issue_apply_allowed=false issue377_clean_room_reference_mode=rust_norion_terms_only issue377_external_code_copied=false issue377_external_prompt_or_schema_copied=false issue377_restricted_license_material_copied=false issue377_license_provenance_posture=project_owned_digest_only issue377_clean_room_apply_allowed=false issue377_manual_approval_binding_present=true issue377_manual_approval_candidate_id=redaction-digest:2222222222223770 issue377_manual_approval_evidence_digest=redaction-digest:3333333333333770 issue377_manual_approval_rollback_anchor=redaction-digest:4444444444443770 issue377_manual_approval_requested_write_scope=reasoning_genome_preview issue377_manual_approval_ref=redaction-digest:5555555555553770 issue377_manual_approval_expiration=1970-01-01T00:00:00Z issue377_manual_approval_apply_allowed=false issue377_manual_approval_applied=false\n",
        )
        .unwrap();

        let statement = issue30_context_statement(&path).unwrap();

        assert!(statement.contains("issue30_environment_pressure_present=true"));
        assert!(statement.contains("issue30_backend_action=deterministic_runtime_kv_roundtrip"));
        assert!(statement.contains("issue385_self_ontology_body_present=true"));
        assert!(statement.contains("issue385_body_state_id=redaction-digest:"));
        assert!(statement.contains("issue385_pheromone_signal_marker_present=true"));
        assert!(statement.contains("issue385_pheromone_signal_marker_id=redaction-digest:"));
        assert!(statement.contains("issue385_pheromone_signal_surface=digest_marker"));
        assert!(statement.contains("issue385_pheromone_signal_digest_gate_allowed=true"));
        assert!(statement.contains("issue385_pheromone_signal_preview_only=true"));
        assert!(statement.contains("issue375_reasoning_frame_environment_signals_present=true"));
        assert!(statement.contains(
            "issue375_reasoning_frame_allowed_observations=repo_issue_terminal_runtime_state"
        ));
        assert!(statement.contains(
            "issue375_reasoning_frame_action_vocab=observe_inspect_compare_summarize_verify_quarantine"
        ));
        assert!(statement.contains(
            "issue375_reasoning_frame_suppressed_capabilities=write_process_browser_network_memory_genome_runtime"
        ));
        assert!(
            statement.contains("issue375_reasoning_frame_risk_limits=preview_only_digest_only")
        );
        assert!(statement.contains("issue375_expression_vm_side_effect=read_only"));
        assert!(statement.contains("issue375_genome_isa_apply_allowed=false"));
        assert!(statement.contains("issue4_dna_candidate_ledger_packet_proof=true"));
        assert!(statement.contains("issue243_active_control_knobs=routing|context_anchor|suppression|checkpoint|memory_maintenance"));
        assert!(statement.contains("issue243_control_expression_gate_ready=true"));
        assert!(statement.contains(
            "issue243_control_expression_gate_ready_source=issue30_context_input_derived"
        ));
        assert!(statement.contains("issue379_zero_beat_primitive_decision_present=true"));
        assert!(statement.contains("issue379_primitive_authority=preview_only"));
        assert!(statement.contains("issue379_primitive_side_effect=read_only"));
        assert!(statement.contains("issue379_primitive_reversibility=rollback_required"));
        assert!(statement.contains("issue379_primitive_evidence=digest_only"));
        assert!(statement.contains("issue379_primitive_uncertainty=hold_on_gap"));
        assert!(statement.contains("issue379_primitive_attention=focus_or_mask_preview"));
        assert!(
            statement
                .contains("issue379_zero_beat_output=action_vocab_mask_and_signal_saliency_bias")
        );
        assert!(statement.contains("issue379_generation_bias_apply_allowed=false"));
        assert!(statement.contains("issue493_tool_organ_registry_present=true"));
        assert!(statement.contains("issue493_tool_organ_registry_id=redaction-digest:"));
        assert!(statement.contains("issue493_tool_organ_registry_preview_only=true"));
        assert!(statement.contains("issue493_tool_organ_registry_side_effect=read_only"));
        assert!(statement.contains("issue493_tool_organ_registry_apply_allowed=false"));
        assert!(
            statement.contains("issue493_tool_organ_capability_matrix_digest=redaction-digest:")
        );
        assert!(statement.contains("issue493_preview_bundle_protocol=bundle_v1"));
        assert!(statement.contains("issue493_preview_bundle_digest=redaction-digest:"));
        assert!(statement.contains("issue493_preview_bundle_refs_digest_only=true"));
        assert!(statement.contains("issue493_preview_bundle_raw_artifacts_allowed=false"));
        assert!(statement.contains("issue493_tool_install_allowed=false"));
        assert!(statement.contains("issue493_tool_execution_allowed=false"));
        assert!(statement.contains("bio_epigenetic_expression_marker_present=true"));
        assert!(statement.contains("bio_epigenetic_expression_marker_id=redaction-digest:"));
        assert!(statement.contains("bio_mrna_cache_candidate_digest=redaction-digest:"));
        assert!(statement.contains("bio_expression_cache_protocol=mrna_preview_v1"));
        assert!(statement.contains("bio_expression_cache_key_digest=redaction-digest:"));
        assert!(statement.contains("bio_hot_path_observation_window=100"));
        assert!(statement.contains("bio_hot_path_min_success_rate=0.98"));
        assert!(statement.contains("bio_gate_relaxation_allowed=false"));
        assert!(statement.contains("bio_cache_materialization_allowed=false"));
        assert!(statement.contains("bio_raw_payload_or_kv_cached=false"));
        assert!(statement.contains("bio_negative_evidence_overrides=true"));
        assert!(statement.contains("issue501_telomere_state_present=true"));
        assert!(statement.contains("issue501_remaining_tokens=0"));
        assert!(statement.contains("issue501_remaining_steps=0"));
        assert!(statement.contains("issue501_remaining_messages=0"));
        assert!(statement.contains("issue501_repair_streak_count=2"));
        assert!(statement.contains("issue501_loop_risk_signal_count=4"));
        assert!(statement.contains("issue501_senescent=true"));
        assert!(statement.contains("issue501_apoptosis_required=true"));
        assert!(statement.contains("issue501_new_external_call_allowed=false"));
        assert!(statement.contains("issue501_new_file_write_allowed=false"));
        assert!(statement.contains("issue501_new_memory_write_allowed=false"));
        assert!(statement.contains("issue501_new_adaptive_state_write_allowed=false"));
        assert!(statement.contains("issue501_memory_promotion_allowed=false"));
        assert!(statement.contains("issue501_genome_mutation_allowed=false"));
        assert!(statement.contains("issue501_takeover_packet_digest=redaction-digest:"));
        assert!(statement.contains("issue501_rollback_anchor_digest=redaction-digest:"));
        assert!(statement.contains("issue501_handoff_next_owner=scheduler"));
        assert!(statement.contains("issue501_raw_payload_present=false"));
        assert!(statement.contains("issue501_preview_side_effect_allowed=false"));
        assert!(statement.contains("issue502_pheromone_blackboard_present=true"));
        assert!(statement.contains("issue502_signal_count=3"));
        assert!(statement.contains("issue502_ranked_action_count=3"));
        assert!(statement.contains("issue502_top_signal_kind=repair_first"));
        assert!(statement.contains("issue502_top_action=repair_review"));
        assert!(statement.contains("issue502_blackboard_digest=redaction-digest:"));
        assert!(statement.contains("issue502_source_digest=redaction-digest:"));
        assert!(statement.contains("issue502_payload_digest=redaction-digest:"));
        assert!(statement.contains("issue502_raw_payload_present=false"));
        assert!(statement.contains("issue502_side_effect_allowed=false"));
        assert!(statement.contains("issue502_ttl_decay_present=true"));
        assert!(statement.contains("issue502_conflict_routes_to_repair=true"));
        assert!(statement.contains("issue502_ranked_actions_from_state_only=true"));
        assert!(statement.contains("issue509_quorum_sensing_present=true"));
        assert!(statement.contains("issue509_decision_id=redaction-digest:"));
        assert!(statement.contains("issue509_quorum_report_digest=redaction-digest:"));
        assert!(statement.contains("issue509_risk_class=irreversible"));
        assert!(statement.contains("issue509_required_quorum_milli=700"));
        assert!(statement.contains("issue509_evaluator_count=3"));
        assert!(statement.contains("issue509_independent_model_count=3"));
        assert!(statement.contains("issue509_independent_lane_count=3"));
        assert!(statement.contains("issue509_approve_signal_count=2"));
        assert!(statement.contains("issue509_reject_signal_count=1"));
        assert!(statement.contains("issue509_abstain_signal_count=0"));
        assert!(statement.contains("issue509_approval_concentration_milli=666"));
        assert!(statement.contains("issue509_conflict_count=1"));
        assert!(statement.contains("issue509_quorum_reached=false"));
        assert!(statement.contains("issue509_apply_allowed=false"));
        assert!(statement.contains("issue509_raw_evaluator_payload_present=false"));
        assert!(statement.contains("issue509_duplicate_sources_count_once=true"));
        assert!(statement.contains("issue509_conflict_routes_to_repair=true"));
        assert!(statement.contains("issue509_writer_gate_bypass_allowed=false"));
        assert!(statement.contains("issue377_problem_finding_present=true"));
        assert!(statement.contains("issue377_problem_finding_kind=wasted_compute"));
        assert!(statement.contains("issue377_problem_finding_severity=medium"));
        assert!(statement.contains("issue377_problem_finding_confidence_milli=850"));
        assert!(statement.contains("issue377_problem_finding_evidence_digest=redaction-digest:"));
        assert!(statement.contains("issue377_problem_finding_source_digest=redaction-digest:"));
        assert!(statement.contains("issue377_problem_finding_affected_surface=runtime_kv_reuse"));
        assert!(statement.contains("issue377_problem_finding_next_step=experiment"));
        assert!(statement.contains("issue377_problem_finding_raw_payload_present=false"));
        assert!(statement.contains("issue377_self_observation_present=true"));
        assert!(statement.contains("issue377_self_observation_id=redaction-digest:"));
        assert!(statement.contains("issue377_self_observation_schema=self_observation_v1"));
        assert!(
            statement.contains("issue377_self_observation_signal_source=runtime_trace_metrics")
        );
        assert!(statement.contains("issue377_self_observation_source_digest=redaction-digest:"));
        assert!(statement.contains("issue377_self_observation_window=second_task_roundtrip"));
        assert!(
            statement.contains("issue377_self_observation_current_truth_digest=redaction-digest:")
        );
        assert!(statement.contains("issue377_self_observation_digest_only=true"));
        assert!(statement.contains("issue377_self_observation_raw_payload_present=false"));
        assert!(statement.contains("issue377_self_observation_write_allowed=false"));
        assert!(statement.contains("issue377_self_observation_applied=false"));
        assert!(statement.contains("issue377_admission_decision=preview_only"));
        assert!(statement.contains("issue377_lexicographic_admission_present=true"));
        assert!(statement.contains("issue377_lexicographic_admission_order=user_intent_preservation>safety>digest_only_evidence>rollback_anchor>quality_delta>cost_delta>latency_delta"));
        assert!(statement.contains("issue377_user_intent_preserved=true"));
        assert!(statement.contains("issue377_safety_gate_passed=true"));
        assert!(statement.contains("issue377_digest_only_evidence_gate_passed=true"));
        assert!(statement.contains("issue377_rollback_anchor_gate_passed=true"));
        assert!(statement.contains("issue377_quality_delta_milli=125"));
        assert!(statement.contains("issue377_cost_delta_milli=-80"));
        assert!(statement.contains("issue377_latency_delta_milli=-35"));
        assert!(statement.contains("issue377_performance_tiebreaker_only=true"));
        assert!(statement.contains("issue377_hard_gate_failure_action=hold"));
        assert!(statement.contains("issue377_risk_override_action=hold"));
        assert!(statement.contains("issue377_negative_evidence_count=0"));
        assert!(statement.contains("issue377_privacy_risk=low"));
        assert!(statement.contains("issue377_license_risk=low"));
        assert!(statement.contains("issue377_unsupported_capability_requested=false"));
        assert!(statement.contains("issue377_unsafe_side_effect_allowed=false"));
        assert!(statement.contains("issue377_risk_override_clear=true"));
        assert!(statement.contains("issue377_lexicographic_admission_apply_allowed=false"));
        assert!(statement.contains("issue377_best_next_state=problem_finding_preview"));
        assert!(statement.contains("issue377_best_next_state_id=redaction-digest:"));
        assert!(statement.contains("issue377_best_next_state_selected=true"));
        assert!(statement.contains("issue377_predicament_signal_present=true"));
        assert!(statement.contains("issue377_predicament_id=redaction-digest:"));
        assert!(statement.contains("issue377_predicament_progress_delta=0"));
        assert!(statement.contains("issue377_predicament_repeat_count=2"));
        assert!(statement.contains("issue377_predicament_evidence_gap_count=0"));
        assert!(statement.contains("issue377_predicament_action_novelty=0"));
        assert!(statement.contains("issue377_predicament_stuck=true"));
        assert!(statement.contains("issue377_self_trigger_stage=preview_only"));
        assert!(statement.contains("issue377_evolution_apply_allowed=false"));
        assert!(statement.contains("issue377_experiment_plan_present=true"));
        assert!(statement.contains("issue377_experiment_plan_id=redaction-digest:"));
        assert!(statement.contains("issue377_experiment_plan_mode=preview_only"));
        assert!(statement.contains(
            "issue377_experiment_plan_level_path=L0_schema_safety|L1_focused_validation|L3_benchmark"
        ));
        assert!(statement.contains(
            "issue377_validation_skipped_levels=L2_replay|L4_integration_readiness|L5_promotion_window"
        ));
        assert!(
            statement.contains("issue377_validation_skipped_reason=minimal_existing_evidence_path")
        );
        assert!(statement.contains("issue377_human_apply_level=L6_human_apply"));
        assert!(statement.contains("issue377_human_apply_inside_engine=false"));
        assert!(statement.contains("issue377_validation_level_apply_allowed=false"));
        assert!(statement.contains(
            "issue377_experiment_plan_required_gates=trace_schema_gate|focused_tests|benchmark_gate"
        ));
        assert!(statement.contains("issue377_experiment_plan_budget_tokens=2048"));
        assert!(statement.contains("issue377_experiment_plan_stop_on_fail=true"));
        assert!(statement.contains("issue377_experiment_plan_rollback_anchor=redaction-digest:"));
        assert!(statement.contains("issue377_experiment_plan_raw_payload_present=false"));
        assert!(statement.contains("issue377_experiment_plan_write_allowed=false"));
        assert!(statement.contains("issue377_experiment_plan_applied=false"));
        assert!(statement.contains("issue377_evidence_bundle_present=true"));
        assert!(statement.contains("issue377_evidence_bundle_id=redaction-digest:"));
        assert!(statement.contains("issue377_evidence_bundle_schema=evidence_bundle_v1"));
        assert!(statement.contains("issue377_evidence_bundle_metric=memory_reuse"));
        assert!(statement.contains("issue377_evidence_bundle_direction=increase"));
        assert!(statement.contains("issue377_evidence_bundle_pass_count=3"));
        assert!(statement.contains("issue377_evidence_bundle_fail_count=0"));
        assert!(
            statement
                .contains("issue377_evidence_bundle_command_label=issue30_fresh_checkout_smoke")
        );
        assert!(statement.contains("issue377_evidence_bundle_refs_digest_only=true"));
        assert!(statement.contains("issue377_evidence_bundle_raw_payload_present=false"));
        assert!(statement.contains("issue377_evidence_bundle_write_allowed=false"));
        assert!(statement.contains("issue377_evidence_bundle_applied=false"));
        assert!(statement.contains("issue377_experiment_decision=promote_for_approval"));
        assert!(statement.contains("issue377_experiment_decision_schema=experiment_decision_v1"));
        assert!(statement.contains(
            "issue377_experiment_decision_reason=clean_evidence_bundle_promotes_preview"
        ));
        assert!(
            statement.contains("issue377_experiment_decision_evidence_bundle_id=redaction-digest:")
        );
        assert!(
            statement.contains("issue377_experiment_decision_target=mutation_candidate_emitter")
        );
        assert!(statement.contains("issue377_experiment_decision_manual_approval_required=true"));
        assert!(statement.contains("issue377_experiment_decision_apply_allowed=false"));
        assert!(statement.contains("issue377_experiment_runner_allowed=false"));
        assert!(statement.contains("issue377_experiment_apply_allowed=false"));
        assert!(statement.contains("issue377_mutation_candidate_emitter_present=true"));
        assert!(statement.contains("issue377_mutation_candidate_emitter_id=redaction-digest:"));
        assert!(statement.contains("issue377_mutation_candidate_id=redaction-digest:"));
        assert!(
            statement.contains("issue377_mutation_candidate_evidence_digest=redaction-digest:")
        );
        assert!(
            statement.contains("issue377_mutation_candidate_rollback_anchor=redaction-digest:")
        );
        assert!(statement.contains(
            "issue377_mutation_candidate_requested_write_scope=reasoning_genome_preview"
        ));
        assert!(statement.contains("issue377_mutation_candidate_kind=mutation_plan_preview"));
        assert!(statement.contains("issue377_mutation_candidate_preview_only=true"));
        assert!(statement.contains("issue377_mutation_candidate_refs_digest_only=true"));
        assert!(statement.contains("issue377_mutation_candidate_writer_gate_preflight=hold"));
        assert!(statement.contains("issue377_mutation_candidate_write_allowed=false"));
        assert!(statement.contains("issue377_mutation_candidate_applied=false"));
        assert!(statement.contains("issue377_mutation_candidate_apply_allowed=false"));
        assert!(statement.contains("issue377_mutation_candidate_manual_review_required=true"));
        assert!(statement.contains("issue377_candidate_emitter_lane_coverage=reasoning_genome_preview|memory_admission_preview|routing_policy_preview|tool_policy_preview|evolution_goal_preview"));
        assert!(statement.contains("issue377_candidate_emitter_kind_coverage=mutation_plan_preview|memory_admission_preview|routing_shadow_proposal|tool_policy_candidate|evolution_goal_preview"));
        assert!(statement.contains("issue377_candidate_emitter_coverage_count=5"));
        assert!(statement.contains("issue377_candidate_emitter_all_preview_only=true"));
        assert!(statement.contains("issue377_candidate_emitter_all_write_allowed=false"));
        assert!(statement.contains("issue377_candidate_emitter_all_apply_allowed=false"));
        assert!(statement.contains("issue377_candidate_emitter_all_manual_review_required=true"));
        assert!(
            statement
                .contains("issue377_candidate_emitter_durable_preflight_owner=unified_writer_gate")
        );
        assert!(statement.contains("issue377_candidate_emitter_writer_gate_bypass_allowed=false"));
        assert!(
            statement.contains("issue377_candidate_emitter_direct_durable_write_allowed=false")
        );
        assert!(statement.contains("issue377_candidate_emitter_ready_for_explicit_apply=false"));
        assert!(statement.contains("issue377_related_issue_refs=#6|#7|#74|#79|#375"));
        assert!(statement.contains("issue377_related_issue_scope_map=#6:experiment_gates|#7:memory_admission_pipeline|#74:thinking_scheduler|#79:evolution_goal_queue|#375:pre_reasoning_genome_isa"));
        assert!(
            statement.contains("issue377_related_issue_owner_scope=meta_cognitive_evolution_loop")
        );
        assert!(statement.contains("issue377_related_issue_non_duplicate_count=5"));
        assert!(statement.contains("issue377_related_issue_all_non_duplicate=true"));
        assert!(statement.contains("issue377_related_issue_apply_allowed=false"));
        assert!(statement.contains("issue377_clean_room_reference_mode=rust_norion_terms_only"));
        assert!(statement.contains("issue377_external_code_copied=false"));
        assert!(statement.contains("issue377_external_prompt_or_schema_copied=false"));
        assert!(statement.contains("issue377_restricted_license_material_copied=false"));
        assert!(
            statement.contains("issue377_license_provenance_posture=project_owned_digest_only")
        );
        assert!(statement.contains("issue377_clean_room_apply_allowed=false"));
        assert!(statement.contains("issue377_manual_approval_binding_present=true"));
        assert!(statement.contains("issue377_manual_approval_candidate_id=redaction-digest:"));
        assert!(statement.contains("issue377_manual_approval_evidence_digest=redaction-digest:"));
        assert!(statement.contains("issue377_manual_approval_rollback_anchor=redaction-digest:"));
        assert!(
            statement.contains(
                "issue377_manual_approval_requested_write_scope=reasoning_genome_preview"
            )
        );
        assert!(statement.contains("issue377_manual_approval_ref=redaction-digest:"));
        assert!(statement.contains("issue377_manual_approval_expiration=1970-01-01T00:00:00Z"));
        assert!(statement.contains("issue377_manual_approval_apply_allowed=false"));
        assert!(statement.contains("issue377_manual_approval_applied=false"));
        assert!(statement.contains("issue30_positive_context_loop_ready=true"));
        assert!(
            statement.contains(
                "issue30_positive_context_loop_ready_source=issue30_context_input_derived"
            )
        );
        assert!(statement.contains("issue30_context_source=issue30_context_input"));

        let mut context_rows = statement.lines();
        let entry_chain = context_rows.next().expect("entry chain row");
        let bad_entry_chain = entry_chain.replace(
            "issue501_new_external_call_allowed=false",
            "issue501_new_external_call_allowed=true",
        );
        let problem_hypothesis = context_rows.next().expect("problem hypothesis row");
        let err = issue30_positive_context_loop_ready(&path, &bad_entry_chain, problem_hypothesis)
            .unwrap_err();
        assert!(err.contains(
            "issue501 apoptosis_required conflicts with issue501_new_external_call_allowed=true"
        ));

        let bad_entry_chain = entry_chain.replace(
            "issue502_raw_payload_present=false",
            "issue502_raw_payload_present=true",
        );
        let err = issue30_positive_context_loop_ready(&path, &bad_entry_chain, problem_hypothesis)
            .unwrap_err();
        assert!(err.contains("issue502 pheromone blackboard conflicts with raw payload presence"));

        let bad_entry_chain = entry_chain.replace(
            "issue509_raw_evaluator_payload_present=false",
            "issue509_raw_evaluator_payload_present=true",
        );
        let err = issue30_positive_context_loop_ready(&path, &bad_entry_chain, problem_hypothesis)
            .unwrap_err();
        assert!(
            err.contains("issue509 quorum sensing conflicts with raw evaluator payload presence")
        );

        let bad_entry_chain = entry_chain.replace(
            "issue509_apply_allowed=false",
            "issue509_apply_allowed=true",
        );
        let err = issue30_positive_context_loop_ready(&path, &bad_entry_chain, problem_hypothesis)
            .unwrap_err();
        assert!(err.contains("issue509 apply_allowed conflicts with quorum/raw-payload fields"));

        let bad_entry_chain = entry_chain.replace(
            "issue379_primitive_side_effect=read_only",
            "issue379_primitive_side_effect=write",
        );
        let err = issue30_positive_context_loop_ready(&path, &bad_entry_chain, problem_hypothesis)
            .unwrap_err();
        assert!(err.contains("issue379 zero-beat output conflicts with primitive dimensions"));

        let bad_entry_chain = entry_chain.replace("issue379_primitive_evidence=digest_only ", "");
        let err = issue30_positive_context_loop_ready(&path, &bad_entry_chain, problem_hypothesis)
            .unwrap_err();
        assert!(err.contains("missing issue379_primitive_evidence"));

        let bad_problem_hypothesis = problem_hypothesis.replace(
            "issue377_predicament_stuck=true",
            "issue377_predicament_stuck=false",
        );
        let err = issue30_positive_context_loop_ready(&path, entry_chain, &bad_problem_hypothesis)
            .unwrap_err();
        assert!(err.contains("issue377_predicament_stuck conflicts with predicament fields"));

        let hold_problem_hypothesis = problem_hypothesis
            .replace(
                "issue377_predicament_evidence_gap_count=0",
                "issue377_predicament_evidence_gap_count=1",
            )
            .replace(
                "issue377_best_next_state=problem_finding_preview",
                "issue377_best_next_state=hold_for_evidence",
            );
        let ready =
            issue30_positive_context_loop_ready(&path, entry_chain, &hold_problem_hypothesis)
                .unwrap();
        assert!(ready.contains("issue30_positive_context_loop_ready=false"));

        let bad_entry_chain = entry_chain
            .replace("issue243_applied=false", "issue243_applied=true")
            + " issue243_control_expression_gate_ready=true";
        let err = issue243_control_expression_gate_ready(&path, &bad_entry_chain).unwrap_err();
        assert!(err.contains("issue243_control_expression_gate_ready conflicts"));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn issue30_entry_chain_context_requires_typed_fields() {
        assert!(ISSUE30_ENTRY_CHAIN_REQUIRED_FIELDS.contains(&"issue375_reasoning_frame_id"));
        let line = issue30_entry_chain_test_line(&[])
            .split_whitespace()
            .filter(|field| !field.starts_with("issue375_reasoning_frame_id="))
            .collect::<Vec<_>>()
            .join(" ");

        let error = Issue30EntryChainContext::parse(Path::new("issue30-context"), 0, &line)
            .expect_err("missing required entry-chain field must fail");

        assert!(error.contains("missing issue375_reasoning_frame_id"));
    }

    #[test]
    fn issue30_entry_chain_context_rejects_unsafe_preview_permissions() {
        for field in [
            "issue4_dna_candidate_ledger_write_allowed",
            "issue243_applied",
            "issue502_raw_payload_present",
        ] {
            assert!(ISSUE30_ENTRY_CHAIN_FALSE_FIELDS.contains(&field));
            let line = issue30_entry_chain_test_line(&[(field, "true")]);

            let error = Issue30EntryChainContext::parse(Path::new("issue30-context"), 0, &line)
                .expect_err("unsafe entry-chain permission must fail");

            assert!(error.contains(field));
            assert!(error.contains("must stay false"));
        }
    }

    fn issue30_entry_chain_test_line(overrides: &[(&str, &str)]) -> String {
        ISSUE30_ENTRY_CHAIN_REQUIRED_FIELDS
            .iter()
            .map(|field| {
                let value = overrides
                    .iter()
                    .find_map(|(name, value)| (*name == *field).then_some(*value))
                    .unwrap_or_else(|| {
                        if ISSUE30_ENTRY_CHAIN_FALSE_FIELDS.contains(field) {
                            "false"
                        } else {
                            "value"
                        }
                    });
                format!("{field}={value}")
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn issue385_body_state_marker_ready_rejects_digest_gate_conflict() {
        let line = "issue385_self_ontology_body_present=true issue385_body_state_id=redaction-digest:eeeeeeeeeeeeeeee issue385_pheromone_signal_marker_present=true issue385_pheromone_signal_marker_id=redaction-digest:9999999999999999 issue385_pheromone_signal_surface=digest_marker issue385_pheromone_signal_digest_gate_allowed=false issue385_pheromone_signal_preview_only=true";

        let error = issue385_body_state_marker_ready(Path::new("issue385-context"), line)
            .expect_err("conflicting digest gate must fail");

        assert!(error.contains("issue385 preview marker conflicts with digest gate"));
    }

    #[test]
    fn issue385_body_state_marker_ready_rejects_raw_marker_id() {
        let line = "issue385_self_ontology_body_present=true issue385_body_state_id=redaction-digest:eeeeeeeeeeeeeeee issue385_pheromone_signal_marker_present=true issue385_pheromone_signal_marker_id=raw-dna-sequence issue385_pheromone_signal_surface=digest_marker issue385_pheromone_signal_digest_gate_allowed=true issue385_pheromone_signal_preview_only=true";

        let error = issue385_body_state_marker_ready(Path::new("issue385-context"), line)
            .expect_err("raw marker id must fail");

        assert!(error.contains("issue385 pheromone signal marker must use digest-only marker id"));
    }

    #[test]
    fn issue385_body_state_marker_ready_rejects_preview_surface_conflict() {
        let line = "issue385_self_ontology_body_present=true issue385_body_state_id=redaction-digest:eeeeeeeeeeeeeeee issue385_pheromone_signal_marker_present=true issue385_pheromone_signal_marker_id=redaction-digest:9999999999999999 issue385_pheromone_signal_surface=prompt issue385_pheromone_signal_digest_gate_allowed=true issue385_pheromone_signal_preview_only=true";

        let error = issue385_body_state_marker_ready(Path::new("issue385-context"), line)
            .expect_err("preview surface conflict must fail");

        assert!(error.contains("issue385 preview marker requires digest_marker surface"));
    }

    #[test]
    fn issue375_reasoning_frame_ready_rejects_raw_frame_id() {
        let line = "issue375_pre_reasoning_genome_isa_present=true issue375_reasoning_frame_id=raw-frame issue375_reasoning_frame_environment_signals_present=true issue375_reasoning_frame_allowed_observations=repo_issue_terminal_runtime_state issue375_reasoning_frame_action_vocab=observe_inspect_compare_summarize_verify_quarantine issue375_reasoning_frame_suppressed_capabilities=write_process_browser_network_memory_genome_runtime issue375_reasoning_frame_risk_limits=preview_only_digest_only issue375_expression_vm_side_effect=read_only issue375_genome_isa_apply_allowed=false";

        let error = issue375_reasoning_frame_ready(Path::new("issue375-context"), line)
            .expect_err("raw frame id must fail");

        assert!(error.contains("issue375 ReasoningFrame must use digest-only frame id"));
    }

    #[test]
    fn issue375_reasoning_frame_ready_rejects_write_side_effect() {
        let line = "issue375_pre_reasoning_genome_isa_present=true issue375_reasoning_frame_id=redaction-digest:ffffffffffffffff issue375_reasoning_frame_environment_signals_present=true issue375_reasoning_frame_allowed_observations=repo_issue_terminal_runtime_state issue375_reasoning_frame_action_vocab=observe_inspect_compare_summarize_verify_quarantine issue375_reasoning_frame_suppressed_capabilities=write_process_browser_network_memory_genome_runtime issue375_reasoning_frame_risk_limits=preview_only_digest_only issue375_expression_vm_side_effect=write issue375_genome_isa_apply_allowed=false";

        let error = issue375_reasoning_frame_ready(Path::new("issue375-context"), line)
            .expect_err("write side effect must fail");

        assert!(error.contains("issue375 ExpressionVM must remain read-only"));
    }

    #[test]
    fn issue375_reasoning_frame_ready_rejects_apply_permission() {
        let line = "issue375_pre_reasoning_genome_isa_present=true issue375_reasoning_frame_id=redaction-digest:ffffffffffffffff issue375_reasoning_frame_environment_signals_present=true issue375_reasoning_frame_allowed_observations=repo_issue_terminal_runtime_state issue375_reasoning_frame_action_vocab=observe_inspect_compare_summarize_verify_quarantine issue375_reasoning_frame_suppressed_capabilities=write_process_browser_network_memory_genome_runtime issue375_reasoning_frame_risk_limits=preview_only_digest_only issue375_expression_vm_side_effect=read_only issue375_genome_isa_apply_allowed=true";

        let error = issue375_reasoning_frame_ready(Path::new("issue375-context"), line)
            .expect_err("apply permission must fail");

        assert!(error.contains("issue375 Genome ISA preview conflicts with apply permission"));
    }

    #[test]
    fn issue493_tool_organ_registry_ready_rejects_raw_registry_id() {
        let line = "issue493_tool_organ_registry_present=true issue493_tool_organ_registry_id=raw-tool-registry issue493_tool_organ_registry_preview_only=true issue493_tool_organ_registry_side_effect=read_only issue493_tool_organ_registry_apply_allowed=false issue493_tool_organ_capability_matrix_digest=redaction-digest:2222222222222222 issue493_preview_bundle_protocol=bundle_v1 issue493_preview_bundle_digest=redaction-digest:3333333333333333 issue493_preview_bundle_refs_digest_only=true issue493_preview_bundle_raw_artifacts_allowed=false issue493_tool_install_allowed=false issue493_tool_execution_allowed=false";

        let error = issue493_tool_organ_registry_ready(Path::new("issue493-context"), line)
            .expect_err("raw registry id must fail");

        assert!(error.contains("issue493 ToolOrganRegistry must use digest-only registry id"));
    }

    #[test]
    fn issue493_tool_organ_registry_ready_rejects_raw_artifacts() {
        let line = "issue493_tool_organ_registry_present=true issue493_tool_organ_registry_id=redaction-digest:1111111111111111 issue493_tool_organ_registry_preview_only=true issue493_tool_organ_registry_side_effect=read_only issue493_tool_organ_registry_apply_allowed=false issue493_tool_organ_capability_matrix_digest=redaction-digest:2222222222222222 issue493_preview_bundle_protocol=bundle_v1 issue493_preview_bundle_digest=redaction-digest:3333333333333333 issue493_preview_bundle_refs_digest_only=true issue493_preview_bundle_raw_artifacts_allowed=true issue493_tool_install_allowed=false issue493_tool_execution_allowed=false";

        let error = issue493_tool_organ_registry_ready(Path::new("issue493-context"), line)
            .expect_err("raw artifacts must fail");

        assert!(error.contains("issue493 preview bundle conflicts with raw artifact permission"));
    }

    #[test]
    fn issue493_tool_organ_registry_ready_rejects_execution_permission() {
        let line = "issue493_tool_organ_registry_present=true issue493_tool_organ_registry_id=redaction-digest:1111111111111111 issue493_tool_organ_registry_preview_only=true issue493_tool_organ_registry_side_effect=read_only issue493_tool_organ_registry_apply_allowed=false issue493_tool_organ_capability_matrix_digest=redaction-digest:2222222222222222 issue493_preview_bundle_protocol=bundle_v1 issue493_preview_bundle_digest=redaction-digest:3333333333333333 issue493_preview_bundle_refs_digest_only=true issue493_preview_bundle_raw_artifacts_allowed=false issue493_tool_install_allowed=false issue493_tool_execution_allowed=true";

        let error = issue493_tool_organ_registry_ready(Path::new("issue493-context"), line)
            .expect_err("execution permission must fail");

        assert!(error.contains("issue493 ToolOrganRegistry conflicts with execution permission"));
    }

    fn issue377_problem_hypothesis_line() -> &'static str {
        "issue377_problem_finding_present=true issue377_problem_finding_id=redaction-digest:aaaaaaaaaaaaaaaa issue377_problem_finding_kind=wasted_compute issue377_problem_finding_severity=medium issue377_problem_finding_confidence_milli=850 issue377_problem_finding_evidence_digest=redaction-digest:7777777777773770 issue377_problem_finding_source_digest=redaction-digest:8888888888883770 issue377_problem_finding_affected_surface=runtime_kv_reuse issue377_problem_finding_next_step=experiment issue377_problem_finding_raw_payload_present=false issue377_self_observation_present=true issue377_self_observation_id=redaction-digest:1010101010103770 issue377_self_observation_schema=self_observation_v1 issue377_self_observation_signal_source=runtime_trace_metrics issue377_self_observation_source_digest=redaction-digest:8888888888883770 issue377_self_observation_window=second_task_roundtrip issue377_self_observation_current_truth_digest=redaction-digest:1212121212123770 issue377_self_observation_digest_only=true issue377_self_observation_raw_payload_present=false issue377_self_observation_write_allowed=false issue377_self_observation_applied=false issue377_self_model_present=true issue377_self_model_id=redaction-digest:1313131313133770 issue377_self_model_schema=control_plane_self_model_v1 issue377_self_model_scope=auditable_control_plane issue377_self_model_claims_consciousness=false issue377_self_model_digest_only=true issue377_self_model_raw_payload_present=false issue377_self_model_write_allowed=false issue377_self_model_applied=false issue377_hypothesis_candidate_present=true issue377_hypothesis_candidate_id=redaction-digest:bbbbbbbbbbbbbbbb issue377_hypothesis_candidate_kind=gene issue377_hypothesis_candidate_status=promoted_for_approval issue377_hypothesis_candidate_target_surface=reasoning_gene issue377_hypothesis_candidate_expected_metric=memory_reuse issue377_hypothesis_candidate_expected_direction=increase issue377_hypothesis_candidate_required_gates=trace_schema_gate|focused_tests|benchmark_gate issue377_hypothesis_candidate_rollback_anchor=redaction-digest:9999999999993770 issue377_hypothesis_candidate_raw_payload_present=false issue377_hypothesis_candidate_write_allowed=false issue377_hypothesis_candidate_applied=false issue377_hypothesis_candidate_operator_approval_required=true issue377_problem_hypothesis_link=redaction-digest:cccccccccccccccc issue377_admission_decision=preview_only issue377_lexicographic_admission_present=true issue377_lexicographic_admission_order=user_intent_preservation>safety>digest_only_evidence>rollback_anchor>quality_delta>cost_delta>latency_delta issue377_user_intent_preserved=true issue377_safety_gate_passed=true issue377_digest_only_evidence_gate_passed=true issue377_rollback_anchor_gate_passed=true issue377_quality_delta_milli=125 issue377_cost_delta_milli=-80 issue377_latency_delta_milli=-35 issue377_performance_tiebreaker_only=true issue377_hard_gate_failure_action=hold issue377_risk_override_action=hold issue377_negative_evidence_count=0 issue377_privacy_risk=low issue377_license_risk=low issue377_unsupported_capability_requested=false issue377_unsafe_side_effect_allowed=false issue377_risk_override_clear=true issue377_lexicographic_admission_apply_allowed=false issue377_best_next_state=problem_finding_preview issue377_best_next_state_id=redaction-digest:6666666666663770 issue377_best_next_state_selected=true issue377_predicament_signal_present=true issue377_predicament_id=redaction-digest:dddddddddddddddd issue377_predicament_progress_delta=0 issue377_predicament_repeat_count=2 issue377_predicament_evidence_gap_count=0 issue377_predicament_action_novelty=0 issue377_predicament_stuck=true issue377_self_trigger_stage=preview_only issue377_evolution_apply_allowed=false issue377_experiment_plan_present=true issue377_experiment_plan_id=redaction-digest:eeeeeeeeeeeeeeee issue377_experiment_plan_mode=preview_only issue377_experiment_plan_level_path=L0_schema_safety|L1_focused_validation|L3_benchmark issue377_validation_skipped_levels=L2_replay|L4_integration_readiness|L5_promotion_window issue377_validation_skipped_reason=minimal_existing_evidence_path issue377_human_apply_level=L6_human_apply issue377_human_apply_inside_engine=false issue377_validation_level_apply_allowed=false issue377_experiment_plan_required_gates=trace_schema_gate|focused_tests|benchmark_gate issue377_experiment_plan_budget_tokens=2048 issue377_experiment_plan_stop_on_fail=true issue377_experiment_plan_rollback_anchor=redaction-digest:9999999999993770 issue377_experiment_plan_raw_payload_present=false issue377_experiment_plan_write_allowed=false issue377_experiment_plan_applied=false issue377_evidence_bundle_present=true issue377_evidence_bundle_id=redaction-digest:ffffffffffffffff issue377_evidence_bundle_schema=evidence_bundle_v1 issue377_evidence_bundle_metric=memory_reuse issue377_evidence_bundle_direction=increase issue377_evidence_bundle_pass_count=3 issue377_evidence_bundle_fail_count=0 issue377_evidence_bundle_command_label=issue30_fresh_checkout_smoke issue377_evidence_bundle_refs_digest_only=true issue377_evidence_bundle_raw_payload_present=false issue377_evidence_bundle_write_allowed=false issue377_evidence_bundle_applied=false issue377_experiment_decision=promote_for_approval issue377_experiment_decision_schema=experiment_decision_v1 issue377_experiment_decision_reason=clean_evidence_bundle_promotes_preview issue377_experiment_decision_evidence_bundle_id=redaction-digest:ffffffffffffffff issue377_experiment_decision_target=mutation_candidate_emitter issue377_experiment_decision_manual_approval_required=true issue377_experiment_decision_apply_allowed=false issue377_experiment_runner_allowed=false issue377_experiment_apply_allowed=false issue377_mutation_candidate_emitter_present=true issue377_mutation_candidate_emitter_id=redaction-digest:1111111111113770 issue377_mutation_candidate_id=redaction-digest:2222222222223770 issue377_mutation_candidate_evidence_digest=redaction-digest:3333333333333770 issue377_mutation_candidate_rollback_anchor=redaction-digest:4444444444443770 issue377_mutation_candidate_requested_write_scope=reasoning_genome_preview issue377_mutation_candidate_kind=mutation_plan_preview issue377_mutation_candidate_preview_only=true issue377_mutation_candidate_refs_digest_only=true issue377_mutation_candidate_writer_gate_preflight=hold issue377_mutation_candidate_write_allowed=false issue377_mutation_candidate_applied=false issue377_mutation_candidate_apply_allowed=false issue377_mutation_candidate_manual_review_required=true issue377_candidate_emitter_lane_coverage=reasoning_genome_preview|memory_admission_preview|routing_policy_preview|tool_policy_preview|evolution_goal_preview issue377_candidate_emitter_kind_coverage=mutation_plan_preview|memory_admission_preview|routing_shadow_proposal|tool_policy_candidate|evolution_goal_preview issue377_candidate_emitter_coverage_count=5 issue377_candidate_emitter_all_preview_only=true issue377_candidate_emitter_all_write_allowed=false issue377_candidate_emitter_all_apply_allowed=false issue377_candidate_emitter_all_manual_review_required=true issue377_candidate_emitter_durable_preflight_owner=unified_writer_gate issue377_candidate_emitter_writer_gate_bypass_allowed=false issue377_candidate_emitter_direct_durable_write_allowed=false issue377_candidate_emitter_ready_for_explicit_apply=false issue377_related_issue_refs=#6|#7|#74|#79|#375 issue377_related_issue_scope_map=#6:experiment_gates|#7:memory_admission_pipeline|#74:thinking_scheduler|#79:evolution_goal_queue|#375:pre_reasoning_genome_isa issue377_related_issue_owner_scope=meta_cognitive_evolution_loop issue377_related_issue_non_duplicate_count=5 issue377_related_issue_all_non_duplicate=true issue377_related_issue_apply_allowed=false issue377_clean_room_reference_mode=rust_norion_terms_only issue377_external_code_copied=false issue377_external_prompt_or_schema_copied=false issue377_restricted_license_material_copied=false issue377_license_provenance_posture=project_owned_digest_only issue377_clean_room_apply_allowed=false issue377_manual_approval_binding_present=true issue377_manual_approval_candidate_id=redaction-digest:2222222222223770 issue377_manual_approval_evidence_digest=redaction-digest:3333333333333770 issue377_manual_approval_rollback_anchor=redaction-digest:4444444444443770 issue377_manual_approval_requested_write_scope=reasoning_genome_preview issue377_manual_approval_ref=redaction-digest:5555555555553770 issue377_manual_approval_expiration=1970-01-01T00:00:00Z issue377_manual_approval_apply_allowed=false issue377_manual_approval_applied=false"
    }

    #[test]
    fn issue377_problem_hypothesis_ready_rejects_raw_problem_id() {
        let line = issue377_problem_hypothesis_line().replace(
            "issue377_problem_finding_id=redaction-digest:aaaaaaaaaaaaaaaa",
            "issue377_problem_finding_id=raw-problem",
        );

        let error = issue377_problem_hypothesis_ready(Path::new("issue377-context"), &line)
            .expect_err("raw problem id must fail");

        assert!(error.contains("issue377 ProblemFinding must use digest-only id"));
    }

    #[test]
    fn issue377_problem_hypothesis_ready_rejects_self_observation_source_drift() {
        let line = issue377_problem_hypothesis_line().replace(
            "issue377_self_observation_source_digest=redaction-digest:8888888888883770",
            "issue377_self_observation_source_digest=redaction-digest:drifted3770000000",
        );

        let error = issue377_problem_hypothesis_ready(Path::new("issue377-context"), &line)
            .expect_err("self observation source must bind problem source");

        assert!(error.contains("issue377 SelfObservation source must bind ProblemFinding source"));
    }

    #[test]
    fn issue377_problem_hypothesis_ready_rejects_consciousness_claim() {
        let line = issue377_problem_hypothesis_line().replace(
            "issue377_self_model_claims_consciousness=false",
            "issue377_self_model_claims_consciousness=true",
        );

        let error = issue377_problem_hypothesis_ready(Path::new("issue377-context"), &line)
            .expect_err("self-model consciousness claim must fail");

        assert!(error.contains("issue377 self-model must not claim consciousness"));
    }

    #[test]
    fn issue377_problem_hypothesis_ready_rejects_self_model_side_effects() {
        for (from, to) in [
            (
                "issue377_self_model_digest_only=true",
                "issue377_self_model_digest_only=false",
            ),
            (
                "issue377_self_model_raw_payload_present=false",
                "issue377_self_model_raw_payload_present=true",
            ),
            (
                "issue377_self_model_write_allowed=false",
                "issue377_self_model_write_allowed=true",
            ),
            (
                "issue377_self_model_applied=false",
                "issue377_self_model_applied=true",
            ),
        ] {
            let line = issue377_problem_hypothesis_line().replace(from, to);
            let error = issue377_problem_hypothesis_ready(Path::new("issue377-context"), &line)
                .expect_err("self-model side effect must fail");

            assert!(
                error.contains("issue377 self-model must be digest-only, read-only, and unapplied")
            );
        }
    }

    #[test]
    fn issue377_problem_hypothesis_ready_rejects_incomplete_candidate_emitter_lanes() {
        let line = issue377_problem_hypothesis_line().replace(
            "issue377_candidate_emitter_lane_coverage=reasoning_genome_preview|memory_admission_preview|routing_policy_preview|tool_policy_preview|evolution_goal_preview",
            "issue377_candidate_emitter_lane_coverage=reasoning_genome_preview",
        );

        let error = issue377_problem_hypothesis_ready(Path::new("issue377-context"), &line)
            .expect_err("single-lane emitter proof must fail");

        assert!(error.contains("issue377 candidate emitter lane coverage is incomplete"));
    }

    #[test]
    fn issue377_problem_hypothesis_ready_rejects_candidate_emitter_side_effects() {
        for (from, to, expected) in [
            (
                "issue377_candidate_emitter_all_preview_only=true",
                "issue377_candidate_emitter_all_preview_only=false",
                "issue377 candidate emitter lanes must all stay preview-only",
            ),
            (
                "issue377_candidate_emitter_all_write_allowed=false",
                "issue377_candidate_emitter_all_write_allowed=true",
                "issue377 candidate emitter lanes must not write or apply",
            ),
            (
                "issue377_candidate_emitter_all_apply_allowed=false",
                "issue377_candidate_emitter_all_apply_allowed=true",
                "issue377 candidate emitter lanes must not write or apply",
            ),
            (
                "issue377_candidate_emitter_all_manual_review_required=true",
                "issue377_candidate_emitter_all_manual_review_required=false",
                "issue377 candidate emitter lanes must require manual review",
            ),
            (
                "issue377_candidate_emitter_durable_preflight_owner=unified_writer_gate",
                "issue377_candidate_emitter_durable_preflight_owner=mutation_candidate_emitter",
                "issue377 durable preflight owner must be unified_writer_gate",
            ),
            (
                "issue377_candidate_emitter_writer_gate_bypass_allowed=false",
                "issue377_candidate_emitter_writer_gate_bypass_allowed=true",
                "issue377 candidate emitter must not bypass the unified writer gate",
            ),
            (
                "issue377_candidate_emitter_direct_durable_write_allowed=false",
                "issue377_candidate_emitter_direct_durable_write_allowed=true",
                "issue377 candidate emitter must not bypass the unified writer gate",
            ),
            (
                "issue377_candidate_emitter_ready_for_explicit_apply=false",
                "issue377_candidate_emitter_ready_for_explicit_apply=true",
                "issue377 candidate emitter must not mark explicit apply ready",
            ),
        ] {
            let line = issue377_problem_hypothesis_line().replace(from, to);
            let error = issue377_problem_hypothesis_ready(Path::new("issue377-context"), &line)
                .expect_err("candidate emitter side effect must fail");

            assert!(error.contains(expected));
        }
    }

    #[test]
    fn issue377_problem_hypothesis_ready_rejects_related_issue_duplication() {
        for (from, to, expected) in [
            (
                "issue377_related_issue_refs=#6|#7|#74|#79|#375",
                "issue377_related_issue_refs=#6|#7",
                "issue377 related issue refs must cover #6/#7/#74/#79/#375",
            ),
            (
                "issue377_related_issue_scope_map=#6:experiment_gates|#7:memory_admission_pipeline|#74:thinking_scheduler|#79:evolution_goal_queue|#375:pre_reasoning_genome_isa",
                "issue377_related_issue_scope_map=#377:everything",
                "issue377 related issue scope map must stay non-duplicated",
            ),
            (
                "issue377_related_issue_owner_scope=meta_cognitive_evolution_loop",
                "issue377_related_issue_owner_scope=evolution_goal_queue",
                "issue377 related issue owner scope is not bounded",
            ),
            (
                "issue377_related_issue_non_duplicate_count=5",
                "issue377_related_issue_non_duplicate_count=4",
                "issue377 related issues must all be non-duplicated",
            ),
            (
                "issue377_related_issue_all_non_duplicate=true",
                "issue377_related_issue_all_non_duplicate=false",
                "issue377 related issues must all be non-duplicated",
            ),
            (
                "issue377_related_issue_apply_allowed=false",
                "issue377_related_issue_apply_allowed=true",
                "issue377 related issue mapping must not apply",
            ),
        ] {
            let line = issue377_problem_hypothesis_line().replace(from, to);
            let error = issue377_problem_hypothesis_ready(Path::new("issue377-context"), &line)
                .expect_err("related issue duplication must fail");

            assert!(error.contains(expected));
        }
    }

    #[test]
    fn issue377_problem_hypothesis_ready_rejects_clean_room_provenance_violations() {
        for (from, to, expected) in [
            (
                "issue377_clean_room_reference_mode=rust_norion_terms_only",
                "issue377_clean_room_reference_mode=external_terms",
                "issue377 clean-room reference mode is not bounded",
            ),
            (
                "issue377_external_code_copied=false",
                "issue377_external_code_copied=true",
                "issue377 clean-room proof must not copy external material",
            ),
            (
                "issue377_external_prompt_or_schema_copied=false",
                "issue377_external_prompt_or_schema_copied=true",
                "issue377 clean-room proof must not copy external material",
            ),
            (
                "issue377_restricted_license_material_copied=false",
                "issue377_restricted_license_material_copied=true",
                "issue377 clean-room proof must not copy external material",
            ),
            (
                "issue377_license_provenance_posture=project_owned_digest_only",
                "issue377_license_provenance_posture=unknown",
                "issue377 license provenance posture is not project-owned digest-only",
            ),
            (
                "issue377_clean_room_apply_allowed=false",
                "issue377_clean_room_apply_allowed=true",
                "issue377 clean-room proof must not apply",
            ),
        ] {
            let line = issue377_problem_hypothesis_line().replace(from, to);
            let error = issue377_problem_hypothesis_ready(Path::new("issue377-context"), &line)
                .expect_err("clean-room provenance violation must fail");

            assert!(error.contains(expected));
        }
    }

    #[test]
    fn issue377_problem_hypothesis_ready_rejects_unbounded_problem_finding_fields() {
        for (from, to, expected) in [
            (
                "issue377_problem_finding_kind=wasted_compute",
                "issue377_problem_finding_kind=magic",
                "issue377 ProblemFinding kind is not bounded",
            ),
            (
                "issue377_problem_finding_severity=medium",
                "issue377_problem_finding_severity=unknown",
                "issue377 ProblemFinding severity is not bounded",
            ),
            (
                "issue377_problem_finding_confidence_milli=850",
                "issue377_problem_finding_confidence_milli=0",
                "issue377 ProblemFinding confidence is not scored",
            ),
            (
                "issue377_problem_finding_affected_surface=runtime_kv_reuse",
                "issue377_problem_finding_affected_surface=filesystem",
                "issue377 ProblemFinding affected surface is not bounded",
            ),
            (
                "issue377_problem_finding_next_step=experiment",
                "issue377_problem_finding_next_step=apply",
                "issue377 ProblemFinding next step is not bounded",
            ),
        ] {
            let line = issue377_problem_hypothesis_line().replace(from, to);

            let error = issue377_problem_hypothesis_ready(Path::new("issue377-context"), &line)
                .expect_err("unbounded ProblemFinding field must fail");

            assert!(error.contains(expected), "{from} -> {to} returned {error}");
        }
    }

    #[test]
    fn issue377_problem_hypothesis_ready_rejects_raw_problem_finding_payloads() {
        for (from, to, expected) in [
            (
                "issue377_problem_finding_evidence_digest=redaction-digest:7777777777773770",
                "issue377_problem_finding_evidence_digest=raw-evidence",
                "issue377 ProblemFinding evidence must be digest-only",
            ),
            (
                "issue377_problem_finding_source_digest=redaction-digest:8888888888883770",
                "issue377_problem_finding_source_digest=raw-source",
                "issue377 ProblemFinding source must be digest-only",
            ),
            (
                "issue377_problem_finding_raw_payload_present=false",
                "issue377_problem_finding_raw_payload_present=true",
                "issue377 ProblemFinding must not carry raw payload",
            ),
        ] {
            let line = issue377_problem_hypothesis_line().replace(from, to);

            let error = issue377_problem_hypothesis_ready(Path::new("issue377-context"), &line)
                .expect_err("raw ProblemFinding payload must fail");

            assert!(error.contains(expected), "{from} -> {to} returned {error}");
        }
    }

    #[test]
    fn issue377_problem_hypothesis_ready_rejects_unbounded_hypothesis_candidate_fields() {
        for (from, to, expected) in [
            (
                "issue377_hypothesis_candidate_kind=gene",
                "issue377_hypothesis_candidate_kind=daemon",
                "issue377 HypothesisCandidate kind is not bounded",
            ),
            (
                "issue377_hypothesis_candidate_status=promoted_for_approval",
                "issue377_hypothesis_candidate_status=applied",
                "issue377 HypothesisCandidate status is not bounded",
            ),
            (
                "issue377_hypothesis_candidate_target_surface=reasoning_gene",
                "issue377_hypothesis_candidate_target_surface=filesystem",
                "issue377 HypothesisCandidate target surface is not bounded",
            ),
            (
                "issue377_hypothesis_candidate_expected_metric=memory_reuse",
                "issue377_hypothesis_candidate_expected_metric=downloads",
                "issue377 HypothesisCandidate expected metric is not bounded",
            ),
            (
                "issue377_hypothesis_candidate_expected_direction=increase",
                "issue377_hypothesis_candidate_expected_direction=explode",
                "issue377 HypothesisCandidate expected direction is not bounded",
            ),
            (
                "issue377_hypothesis_candidate_required_gates=trace_schema_gate|focused_tests|benchmark_gate",
                "issue377_hypothesis_candidate_required_gates=trace_schema_gate|shell_apply",
                "issue377 HypothesisCandidate required gates are not bounded",
            ),
        ] {
            let line = issue377_problem_hypothesis_line().replace(from, to);

            let error = issue377_problem_hypothesis_ready(Path::new("issue377-context"), &line)
                .expect_err("unbounded HypothesisCandidate field must fail");

            assert!(error.contains(expected), "{from} -> {to} returned {error}");
        }
    }

    #[test]
    fn issue377_problem_hypothesis_ready_rejects_hypothesis_candidate_side_effects() {
        for (from, to, expected) in [
            (
                "issue377_hypothesis_candidate_rollback_anchor=redaction-digest:9999999999993770",
                "issue377_hypothesis_candidate_rollback_anchor=raw-anchor",
                "issue377 HypothesisCandidate rollback anchor must be digest-only",
            ),
            (
                "issue377_hypothesis_candidate_raw_payload_present=false",
                "issue377_hypothesis_candidate_raw_payload_present=true",
                "issue377 HypothesisCandidate must not carry raw payload",
            ),
            (
                "issue377_hypothesis_candidate_write_allowed=false",
                "issue377_hypothesis_candidate_write_allowed=true",
                "issue377 HypothesisCandidate must not write or apply",
            ),
            (
                "issue377_hypothesis_candidate_applied=false",
                "issue377_hypothesis_candidate_applied=true",
                "issue377 HypothesisCandidate must not write or apply",
            ),
            (
                "issue377_hypothesis_candidate_operator_approval_required=true",
                "issue377_hypothesis_candidate_operator_approval_required=false",
                "issue377 HypothesisCandidate must require operator approval",
            ),
        ] {
            let line = issue377_problem_hypothesis_line().replace(from, to);

            let error = issue377_problem_hypothesis_ready(Path::new("issue377-context"), &line)
                .expect_err("unsafe HypothesisCandidate field must fail");

            assert!(error.contains(expected), "{from} -> {to} returned {error}");
        }
    }

    #[test]
    fn issue377_problem_hypothesis_ready_rejects_unbounded_experiment_plan_fields() {
        for (from, to, expected) in [
            (
                "issue377_experiment_plan_level_path=L0_schema_safety|L1_focused_validation|L3_benchmark",
                "issue377_experiment_plan_level_path=L6_human_apply",
                "issue377 ExperimentPlan validation path is not the minimal bounded path",
            ),
            (
                "issue377_experiment_plan_required_gates=trace_schema_gate|focused_tests|benchmark_gate",
                "issue377_experiment_plan_required_gates=trace_schema_gate|shell_apply",
                "issue377 ExperimentPlan gates must bind the HypothesisCandidate gates",
            ),
            (
                "issue377_experiment_plan_budget_tokens=2048",
                "issue377_experiment_plan_budget_tokens=0",
                "issue377 ExperimentPlan budget is not bounded",
            ),
            (
                "issue377_experiment_plan_stop_on_fail=true",
                "issue377_experiment_plan_stop_on_fail=false",
                "issue377 ExperimentPlan must stop on first failed gate",
            ),
        ] {
            let line = issue377_problem_hypothesis_line().replace(from, to);

            let error = issue377_problem_hypothesis_ready(Path::new("issue377-context"), &line)
                .expect_err("unbounded ExperimentPlan field must fail");

            assert!(error.contains(expected), "{from} -> {to} returned {error}");
        }
    }

    #[test]
    fn issue377_problem_hypothesis_ready_rejects_experiment_plan_side_effects() {
        for (from, to, expected) in [
            (
                "issue377_experiment_plan_rollback_anchor=redaction-digest:9999999999993770",
                "issue377_experiment_plan_rollback_anchor=raw-anchor",
                "issue377 ExperimentPlan rollback anchor must bind the HypothesisCandidate rollback anchor",
            ),
            (
                "issue377_experiment_plan_raw_payload_present=false",
                "issue377_experiment_plan_raw_payload_present=true",
                "issue377 ExperimentPlan must not carry raw payload",
            ),
            (
                "issue377_experiment_plan_write_allowed=false",
                "issue377_experiment_plan_write_allowed=true",
                "issue377 ExperimentPlan must not write or apply",
            ),
            (
                "issue377_experiment_plan_applied=false",
                "issue377_experiment_plan_applied=true",
                "issue377 ExperimentPlan must not write or apply",
            ),
        ] {
            let line = issue377_problem_hypothesis_line().replace(from, to);

            let error = issue377_problem_hypothesis_ready(Path::new("issue377-context"), &line)
                .expect_err("unsafe ExperimentPlan field must fail");

            assert!(error.contains(expected), "{from} -> {to} returned {error}");
        }
    }

    #[test]
    fn issue377_problem_hypothesis_ready_rejects_non_preview_admission() {
        let line = issue377_problem_hypothesis_line().replace(
            "issue377_admission_decision=preview_only",
            "issue377_admission_decision=manual_review",
        );

        let error = issue377_problem_hypothesis_ready(Path::new("issue377-context"), &line)
            .expect_err("non-preview admission must fail");

        assert!(
            error.contains("issue377 ProblemFinding/HypothesisCandidate must remain preview-only")
        );
    }

    #[test]
    fn issue377_problem_hypothesis_ready_rejects_wrong_lexicographic_order() {
        let line = issue377_problem_hypothesis_line().replace(
            "issue377_lexicographic_admission_order=user_intent_preservation>safety>digest_only_evidence>rollback_anchor>quality_delta>cost_delta>latency_delta",
            "issue377_lexicographic_admission_order=quality_delta>safety",
        );

        let error = issue377_problem_hypothesis_ready(Path::new("issue377-context"), &line)
            .expect_err("wrong admission order must fail");

        assert!(error.contains("issue377 lexicographic admission order is not bounded"));
    }

    #[test]
    fn issue377_problem_hypothesis_ready_rejects_failed_hard_gate() {
        let line = issue377_problem_hypothesis_line().replace(
            "issue377_safety_gate_passed=true",
            "issue377_safety_gate_passed=false",
        );

        let error = issue377_problem_hypothesis_ready(Path::new("issue377-context"), &line)
            .expect_err("failed hard gate must fail");

        assert!(
            error
                .contains("issue377 lexicographic hard gates must pass before performance ranking")
        );
    }

    #[test]
    fn issue377_problem_hypothesis_ready_rejects_performance_override() {
        let line = issue377_problem_hypothesis_line().replace(
            "issue377_performance_tiebreaker_only=true",
            "issue377_performance_tiebreaker_only=false",
        );

        let error = issue377_problem_hypothesis_ready(Path::new("issue377-context"), &line)
            .expect_err("performance override must fail");

        assert!(error.contains("issue377 performance deltas must remain tie-breakers"));
    }

    #[test]
    fn issue377_problem_hypothesis_ready_rejects_unbounded_risk_override_action() {
        let line = issue377_problem_hypothesis_line().replace(
            "issue377_risk_override_action=hold",
            "issue377_risk_override_action=apply",
        );

        let error = issue377_problem_hypothesis_ready(Path::new("issue377-context"), &line)
            .expect_err("unbounded risk override action must fail");

        assert!(error.contains("issue377 risk override action is not fail-closed"));
    }

    #[test]
    fn issue377_problem_hypothesis_ready_rejects_unbounded_privacy_risk() {
        let line = issue377_problem_hypothesis_line()
            .replace("issue377_privacy_risk=low", "issue377_privacy_risk=unknown");

        let error = issue377_problem_hypothesis_ready(Path::new("issue377-context"), &line)
            .expect_err("unbounded privacy risk must fail");

        assert!(error.contains("issue377 privacy risk is not bounded"));
    }

    #[test]
    fn issue377_problem_hypothesis_ready_rejects_risk_blockers_with_clear_true() {
        for (from, to) in [
            (
                "issue377_negative_evidence_count=0",
                "issue377_negative_evidence_count=1",
            ),
            ("issue377_privacy_risk=low", "issue377_privacy_risk=medium"),
            ("issue377_license_risk=low", "issue377_license_risk=medium"),
            (
                "issue377_unsupported_capability_requested=false",
                "issue377_unsupported_capability_requested=true",
            ),
            (
                "issue377_unsafe_side_effect_allowed=false",
                "issue377_unsafe_side_effect_allowed=true",
            ),
        ] {
            let line = issue377_problem_hypothesis_line().replace(from, to);

            let error = issue377_problem_hypothesis_ready(Path::new("issue377-context"), &line)
                .expect_err("risk blocker with clear=true must fail");

            assert!(
                error.contains("issue377 risk override clear conflicts with risk blockers"),
                "{from} -> {to} returned {error}"
            );
        }
    }

    #[test]
    fn issue377_problem_hypothesis_ready_rejects_wrong_best_next_state() {
        let line = issue377_problem_hypothesis_line().replace(
            "issue377_best_next_state=problem_finding_preview",
            "issue377_best_next_state=watch",
        );

        let error = issue377_problem_hypothesis_ready(Path::new("issue377-context"), &line)
            .expect_err("wrong best-next-state must fail");

        assert!(error.contains("issue377 best-next-state conflicts with predicament fields"));
    }

    #[test]
    fn issue377_problem_hypothesis_ready_rejects_raw_best_next_state_id() {
        let line = issue377_problem_hypothesis_line().replace(
            "issue377_best_next_state_id=redaction-digest:6666666666663770",
            "issue377_best_next_state_id=raw-state",
        );

        let error = issue377_problem_hypothesis_ready(Path::new("issue377-context"), &line)
            .expect_err("raw best-next-state id must fail");

        assert!(error.contains("issue377 best-next-state id must use digest-only id"));
    }

    #[test]
    fn issue377_problem_hypothesis_ready_rejects_lexicographic_apply_permission() {
        let line = issue377_problem_hypothesis_line().replace(
            "issue377_lexicographic_admission_apply_allowed=false",
            "issue377_lexicographic_admission_apply_allowed=true",
        );

        let error = issue377_problem_hypothesis_ready(Path::new("issue377-context"), &line)
            .expect_err("lexicographic admission apply permission must fail");

        assert!(error.contains("issue377 lexicographic admission does not apply by itself"));
    }

    #[test]
    fn issue377_problem_hypothesis_ready_rejects_apply_permission() {
        let line = issue377_problem_hypothesis_line().replace(
            "issue377_evolution_apply_allowed=false",
            "issue377_evolution_apply_allowed=true",
        );

        let error = issue377_problem_hypothesis_ready(Path::new("issue377-context"), &line)
            .expect_err("apply permission must fail");

        assert!(error.contains("issue377 evolution preview conflicts with apply permission"));
    }

    #[test]
    fn issue377_problem_hypothesis_ready_rejects_raw_experiment_plan_id() {
        let line = issue377_problem_hypothesis_line().replace(
            "issue377_experiment_plan_id=redaction-digest:eeeeeeeeeeeeeeee",
            "issue377_experiment_plan_id=raw-plan",
        );

        let error = issue377_problem_hypothesis_ready(Path::new("issue377-context"), &line)
            .expect_err("raw experiment plan id must fail");

        assert!(error.contains("issue377 ExperimentPlan must use digest-only id"));
    }

    #[test]
    fn issue377_problem_hypothesis_ready_rejects_validation_level_escape() {
        for (from, to, expected) in [
            (
                "issue377_validation_skipped_levels=L2_replay|L4_integration_readiness|L5_promotion_window",
                "issue377_validation_skipped_levels=L2_replay",
                "issue377 validation skipped levels are not bounded",
            ),
            (
                "issue377_validation_skipped_reason=minimal_existing_evidence_path",
                "issue377_validation_skipped_reason=promotion_window_pending",
                "issue377 validation skipped reason is not minimal",
            ),
            (
                "issue377_human_apply_level=L6_human_apply",
                "issue377_human_apply_level=L5_promotion_window",
                "issue377 human apply level is not L6",
            ),
            (
                "issue377_human_apply_inside_engine=false",
                "issue377_human_apply_inside_engine=true",
                "issue377 validation levels must keep human apply outside the engine",
            ),
            (
                "issue377_validation_level_apply_allowed=false",
                "issue377_validation_level_apply_allowed=true",
                "issue377 validation levels must keep human apply outside the engine",
            ),
        ] {
            let line = issue377_problem_hypothesis_line().replace(from, to);

            let error = issue377_problem_hypothesis_ready(Path::new("issue377-context"), &line)
                .expect_err("validation level escape must fail");

            assert!(error.contains(expected), "{from} -> {to} returned {error}");
        }
    }

    #[test]
    fn issue377_meta_cognitive_design_note_covers_runtime_boundaries() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("docs")
            .join("architecture")
            .join("meta-cognitive-evolution-loop.md");
        let body = std::fs::read_to_string(&path).expect("issue #377 design note must exist");

        for required in [
            "# Meta-Cognitive Evolution Loop",
            "SelfObservation",
            "ProblemFinding",
            "HypothesisCandidate",
            "ExperimentPlan",
            "EvidenceBundle",
            "ExperimentDecision",
            "MutationCandidateEmitter",
            "issue377_predicament_progress_delta",
            "issue377_validation_skipped_levels=L2_replay|L4_integration_readiness|L5_promotion_window",
            "issue377_human_apply_level=L6_human_apply",
            "issue377_candidate_emitter_durable_preflight_owner=unified_writer_gate",
            "issue377_manual_approval_candidate_id",
            "issue377_related_issue_refs=#6|#7|#74|#79|#375",
            "issue377_clean_room_reference_mode=rust_norion_terms_only",
            "write_allowed=false",
            "applied=false",
        ] {
            assert!(
                body.contains(required),
                "{} missing required #377 design-note marker: {required}",
                path.display()
            );
        }
    }

    #[test]
    fn issue375_pre_reasoning_genome_isa_note_covers_runtime_boundaries() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("docs")
            .join("architecture")
            .join("pre-reasoning-genome-isa.md");
        let body = std::fs::read_to_string(&path).expect("issue #375 design note must exist");

        for required in [
            "# Pre-Reasoning Genome ISA",
            "PreReasoningGenomeIsa",
            "GenomeOpcode",
            "ExpressionVM",
            "ReasoningFrame",
            "BIND_STIMULUS",
            "EMIT_FRAME",
            "read-only",
            "no-side-effect",
            "environment stimulus",
            "GenomeExpression",
            "TaskExpressionGene",
            "ThinkingScheduler",
            "#243",
            "#304",
            "Tool/action gates",
            "writer gates",
            "computer-use adapters",
            "shell",
            "browser",
            "network",
            "file write",
            "memory write",
            "process launch",
            "issue/PR creation",
            "genome mutation",
            "issue375_expression_vm_side_effect=read_only",
            "issue375_genome_isa_apply_allowed=false",
            "clean-room",
            "No implementation is required",
        ] {
            assert!(
                body.contains(required),
                "{} missing required #375 design-note marker: {required}",
                path.display()
            );
        }
    }

    #[test]
    fn issue377_problem_hypothesis_ready_rejects_raw_evidence_bundle_refs() {
        let line = issue377_problem_hypothesis_line().replace(
            "issue377_evidence_bundle_refs_digest_only=true",
            "issue377_evidence_bundle_refs_digest_only=false",
        );

        let error = issue377_problem_hypothesis_ready(Path::new("issue377-context"), &line)
            .expect_err("raw evidence bundle refs must fail");

        assert!(error.contains("issue377 EvidenceBundle refs must remain digest-only"));
    }

    #[test]
    fn issue377_problem_hypothesis_ready_rejects_unbounded_evidence_bundle_fields() {
        let line = issue377_problem_hypothesis_line().replace(
            "issue377_evidence_bundle_schema=evidence_bundle_v1",
            "issue377_evidence_bundle_schema=raw_bundle",
        );

        let error = issue377_problem_hypothesis_ready(Path::new("issue377-context"), &line)
            .expect_err("unbounded EvidenceBundle schema must fail");

        assert!(error.contains("issue377 EvidenceBundle schema is not bounded"));
    }

    #[test]
    fn issue377_problem_hypothesis_ready_rejects_unbound_evidence_bundle_metric() {
        let line = issue377_problem_hypothesis_line().replace(
            "issue377_evidence_bundle_metric=memory_reuse",
            "issue377_evidence_bundle_metric=latency",
        );

        let error = issue377_problem_hypothesis_ready(Path::new("issue377-context"), &line)
            .expect_err("EvidenceBundle metric mismatch must fail");

        assert!(
            error.contains(
                "issue377 EvidenceBundle metric must bind the HypothesisCandidate metric"
            )
        );
    }

    #[test]
    fn issue377_problem_hypothesis_ready_rejects_failed_evidence_bundle_run() {
        let line = issue377_problem_hypothesis_line()
            .replace(
                "issue377_evidence_bundle_pass_count=3",
                "issue377_evidence_bundle_pass_count=0",
            )
            .replace(
                "issue377_evidence_bundle_fail_count=0",
                "issue377_evidence_bundle_fail_count=1",
            );

        let error = issue377_problem_hypothesis_ready(Path::new("issue377-context"), &line)
            .expect_err("failed EvidenceBundle run must fail");

        assert!(error.contains("issue377 EvidenceBundle pass/fail counts must prove a clean run"));
    }

    #[test]
    fn issue377_problem_hypothesis_ready_rejects_evidence_bundle_side_effects() {
        let line = issue377_problem_hypothesis_line()
            .replace(
                "issue377_evidence_bundle_raw_payload_present=false",
                "issue377_evidence_bundle_raw_payload_present=true",
            )
            .replace(
                "issue377_evidence_bundle_write_allowed=false",
                "issue377_evidence_bundle_write_allowed=true",
            )
            .replace(
                "issue377_evidence_bundle_applied=false",
                "issue377_evidence_bundle_applied=true",
            );

        let error = issue377_problem_hypothesis_ready(Path::new("issue377-context"), &line)
            .expect_err("EvidenceBundle side effects must fail");

        assert!(
            error.contains("issue377 EvidenceBundle must not carry raw payload, write, or apply")
        );
    }

    #[test]
    fn issue377_problem_hypothesis_ready_rejects_unbounded_experiment_decision_fields() {
        let line = issue377_problem_hypothesis_line().replace(
            "issue377_experiment_decision_schema=experiment_decision_v1",
            "issue377_experiment_decision_schema=raw_decision",
        );

        let error = issue377_problem_hypothesis_ready(Path::new("issue377-context"), &line)
            .expect_err("unbounded ExperimentDecision schema must fail");

        assert!(error.contains("issue377 ExperimentDecision schema is not bounded"));
    }

    #[test]
    fn issue377_problem_hypothesis_ready_rejects_unbound_experiment_decision_bundle() {
        let line = issue377_problem_hypothesis_line().replace(
            "issue377_experiment_decision_evidence_bundle_id=redaction-digest:ffffffffffffffff",
            "issue377_experiment_decision_evidence_bundle_id=redaction-digest:9999999999999999",
        );

        let error = issue377_problem_hypothesis_ready(Path::new("issue377-context"), &line)
            .expect_err("ExperimentDecision bundle mismatch must fail");

        assert!(error.contains("issue377 ExperimentDecision must bind the EvidenceBundle id"));
    }

    #[test]
    fn issue377_problem_hypothesis_ready_rejects_experiment_decision_without_manual_approval() {
        let line = issue377_problem_hypothesis_line().replace(
            "issue377_experiment_decision_manual_approval_required=true",
            "issue377_experiment_decision_manual_approval_required=false",
        );

        let error = issue377_problem_hypothesis_ready(Path::new("issue377-context"), &line)
            .expect_err("ExperimentDecision without manual approval must fail");

        assert!(
            error.contains("issue377 ExperimentDecision promotion must require manual approval")
        );
    }

    #[test]
    fn issue377_problem_hypothesis_ready_rejects_experiment_decision_apply_permission() {
        let line = issue377_problem_hypothesis_line().replace(
            "issue377_experiment_decision_apply_allowed=false",
            "issue377_experiment_decision_apply_allowed=true",
        );

        let error = issue377_problem_hypothesis_ready(Path::new("issue377-context"), &line)
            .expect_err("ExperimentDecision apply permission must fail");

        assert!(error.contains("issue377 ExperimentDecision must not apply"));
    }

    #[test]
    fn issue377_problem_hypothesis_ready_rejects_experiment_runner_permission() {
        let line = issue377_problem_hypothesis_line().replace(
            "issue377_experiment_runner_allowed=false",
            "issue377_experiment_runner_allowed=true",
        );

        let error = issue377_problem_hypothesis_ready(Path::new("issue377-context"), &line)
            .expect_err("experiment runner permission must fail");

        assert!(error.contains("issue377 ExperimentPlan preview conflicts with runner permission"));
    }

    #[test]
    fn issue377_problem_hypothesis_ready_rejects_raw_mutation_candidate_id() {
        let line = issue377_problem_hypothesis_line().replace(
            "issue377_mutation_candidate_id=redaction-digest:2222222222223770",
            "issue377_mutation_candidate_id=raw-candidate",
        );

        let error = issue377_problem_hypothesis_ready(Path::new("issue377-context"), &line)
            .expect_err("raw mutation candidate id must fail");

        assert!(error.contains("issue377 mutation candidate must use digest-only id"));
    }

    #[test]
    fn issue377_problem_hypothesis_ready_rejects_applied_mutation_candidate() {
        let line = issue377_problem_hypothesis_line().replace(
            "issue377_mutation_candidate_applied=false",
            "issue377_mutation_candidate_applied=true",
        );

        let error = issue377_problem_hypothesis_ready(Path::new("issue377-context"), &line)
            .expect_err("applied mutation candidate must fail");

        assert!(error.contains("issue377 mutation candidate preview conflicts with applied state"));
    }

    #[test]
    fn issue377_problem_hypothesis_ready_rejects_missing_mutation_manual_review() {
        let line = issue377_problem_hypothesis_line().replace(
            "issue377_mutation_candidate_manual_review_required=true",
            "issue377_mutation_candidate_manual_review_required=false",
        );

        let error = issue377_problem_hypothesis_ready(Path::new("issue377-context"), &line)
            .expect_err("mutation candidate without manual review must fail");

        assert!(error.contains("issue377 mutation candidate preview must require manual review"));
    }

    #[test]
    fn issue377_problem_hypothesis_ready_rejects_raw_manual_approval_ref() {
        let line = issue377_problem_hypothesis_line().replace(
            "issue377_manual_approval_ref=redaction-digest:5555555555553770",
            "issue377_manual_approval_ref=raw-approval",
        );

        let error = issue377_problem_hypothesis_ready(Path::new("issue377-context"), &line)
            .expect_err("raw manual approval ref must fail");

        assert!(error.contains("issue377 manual approval ref must use digest-only id"));
    }

    #[test]
    fn issue377_problem_hypothesis_ready_rejects_unbound_manual_approval_candidate() {
        let line = issue377_problem_hypothesis_line().replace(
            "issue377_manual_approval_candidate_id=redaction-digest:2222222222223770",
            "issue377_manual_approval_candidate_id=redaction-digest:9999999999993770",
        );

        let error = issue377_problem_hypothesis_ready(Path::new("issue377-context"), &line)
            .expect_err("manual approval candidate mismatch must fail");

        assert!(error.contains("issue377 manual approval must bind the mutation candidate id"));
    }

    #[test]
    fn issue377_problem_hypothesis_ready_rejects_manual_approval_without_expiration() {
        let line = issue377_problem_hypothesis_line().replace(
            "issue377_manual_approval_expiration=1970-01-01T00:00:00Z",
            "issue377_manual_approval_expiration=none",
        );

        let error = issue377_problem_hypothesis_ready(Path::new("issue377-context"), &line)
            .expect_err("manual approval without expiration must fail");

        assert!(error.contains("issue377 manual approval must include expiration"));
    }

    #[test]
    fn issue377_problem_hypothesis_ready_rejects_manual_approval_apply_permission() {
        let line = issue377_problem_hypothesis_line().replace(
            "issue377_manual_approval_apply_allowed=false",
            "issue377_manual_approval_apply_allowed=true",
        );

        let error = issue377_problem_hypothesis_ready(Path::new("issue377-context"), &line)
            .expect_err("manual approval apply permission must fail");

        assert!(error.contains("issue377 manual approval binding does not apply by itself"));
    }

    fn issue243_fixture_matrix_rows() -> String {
        let base = "issue243_active_control_knobs=routing|context_anchor|suppression|checkpoint|memory_maintenance issue243_evidence_digest=redaction-digest:control243 issue243_policy_version=control_expression_gate_v1 issue243_decision_reason=no_weight_runtime_control_preview issue243_write_allowed=false issue243_applied=false issue243_operator_approval_required=true";
        [
            (
                "no_weight_control_accepted",
                "issue243_no_weight_control_accepted=true issue243_control_expression_profile_selected=1",
            ),
            ("adapter_handoff_held", "issue243_adapter_handoff_held=true"),
            (
                "long_context_anchor_promoted",
                "issue243_context_anchor_promoted=1",
            ),
            (
                "polluted_candidate_suppressed",
                "issue243_suppression_gate_triggered=1",
            ),
            (
                "verifier_checkpoint_failure",
                "issue243_checkpoint_rejected=1",
            ),
            (
                "successful_repair_retry",
                "issue243_checkpoint_repair_requested=1 issue243_repair_retry_succeeded=true",
            ),
            (
                "memory_refresh_candidate",
                "issue243_memory_refresh_candidate=1",
            ),
            (
                "tombstone_held_for_approval",
                "issue243_memory_tombstone_candidate=1 issue243_tombstone_held_for_approval=true",
            ),
            ("writer_gate_denial", ""),
        ]
        .into_iter()
        .map(|(fixture, extra)| format!("fixture={fixture} {base} {extra}\n"))
        .collect()
    }

    #[test]
    fn issue243_fixture_matrix_statement_derives_ready_from_all_cases() {
        let path = std::env::temp_dir().join(format!(
            "norion-cli-issue243-fixture-matrix-{}.txt",
            std::process::id()
        ));
        fs::write(&path, issue243_fixture_matrix_rows()).unwrap();

        let statement = issue243_fixture_matrix_statement(&path).unwrap();

        assert_eq!(
            statement,
            "issue243_control_fixture_matrix_ready=true issue243_control_fixture_matrix_cases=no_weight_control_accepted|adapter_handoff_held|long_context_anchor_promoted|polluted_candidate_suppressed|verifier_checkpoint_failure|successful_repair_retry|memory_refresh_candidate|tombstone_held_for_approval|writer_gate_denial issue243_control_fixture_matrix_source=issue243_fixture_matrix_input"
        );

        let _ = fs::remove_file(path);
    }

    #[test]
    fn issue243_fixture_matrix_statement_requires_all_cases() {
        let path = std::env::temp_dir().join(format!(
            "norion-cli-issue243-fixture-matrix-missing-{}.txt",
            std::process::id()
        ));
        let rows = issue243_fixture_matrix_rows()
            .lines()
            .filter(|line| !line.starts_with("fixture=writer_gate_denial "))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        fs::write(&path, rows).unwrap();

        let err = issue243_fixture_matrix_statement(&path).unwrap_err();

        assert!(err.contains("missing issue243 fixture writer_gate_denial"));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn state_files_statement_derives_presence_without_paths() {
        let dir =
            std::env::temp_dir().join(format!("norion-cli-state-files-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let memory = dir.join("memory.ndkv");
        let experience = dir.join("experience.ndkv");
        let adaptive = dir.join("adaptive.ndkv");
        let input = dir.join("state-files.txt");
        fs::write(&memory, "memory").unwrap();
        fs::write(&experience, "experience").unwrap();
        fs::write(&adaptive, "adaptive").unwrap();
        fs::write(
            &input,
            format!(
                "memory={} experience={} adaptive={} ndkv_non_fixture_writes=0\n",
                memory.display(),
                experience.display(),
                adaptive.display()
            ),
        )
        .unwrap();

        let statement = state_files_statement(&input).unwrap();

        assert!(statement.contains("memory_file_exists=true"));
        assert!(statement.contains("experience_file_exists=true"));
        assert!(statement.contains("adaptive_file_exists=true"));
        assert!(statement.contains("memory_file_ndkv=true"));
        assert!(statement.contains("experience_file_ndkv=true"));
        assert!(statement.contains("adaptive_file_ndkv=true"));
        assert!(statement.contains("issue2_state_files_ndkv_proof=true"));
        assert!(
            statement.contains("issue2_state_files_ndkv_proof_source=state_files_input_derived")
        );
        assert!(statement.contains("issue30_state_files_ready=true"));
        assert!(statement.contains("issue30_state_files_ready_source=state_files_input_derived"));
        assert!(statement.contains("issue2_ndkv_non_fixture_writes=0"));
        assert!(statement.contains("issue2_ndkv_non_fixture_write_proof=true"));
        assert!(statement.contains("issue2_ndkv_non_fixture_write_proof_source=state_files_input"));
        assert!(statement.contains("state_files_source=state_files_input"));
        assert!(!statement.contains(&dir.display().to_string()));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn state_files_statement_rejects_conflicting_ndkv_proof() {
        let dir = std::env::temp_dir().join(format!(
            "norion-cli-state-files-conflict-{}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        let memory = dir.join("memory.txt");
        let experience = dir.join("experience.ndkv");
        let adaptive = dir.join("adaptive.ndkv");
        let input = dir.join("state-files.txt");
        fs::write(&memory, "memory").unwrap();
        fs::write(&experience, "experience").unwrap();
        fs::write(&adaptive, "adaptive").unwrap();
        fs::write(
            &input,
            format!(
                "memory={} experience={} adaptive={} issue2_state_files_ndkv_proof=true\n",
                memory.display(),
                experience.display(),
                adaptive.display()
            ),
        )
        .unwrap();

        let error = state_files_statement(&input).unwrap_err();

        assert!(
            error.contains("issue2_state_files_ndkv_proof conflicts with state file extensions")
        );
        let _ = fs::remove_dir_all(dir);
    }
}
