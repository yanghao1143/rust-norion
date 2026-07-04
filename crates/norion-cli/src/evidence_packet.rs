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
    pub issue30_context_input: Option<PathBuf>,
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
    let mut issue30_context_input = None;
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
            "--issue30-context-input" => {
                issue30_context_input =
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
        issue30_context_input,
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
    if let Some(path) = config.issue30_context_input.as_deref() {
        generated.push(issue30_context_statement(path)?);
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
        let compute_anchors_preserved = roundtrip_compute_anchors_preserved(path, index, line)?;
        let second_task_benefit_ready = roundtrip_second_task_benefit_ready(path, index, line)?;
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
            "{line}{compute_budget_reduced}{compute_anchors_preserved}{second_task_benefit_ready}{durable_write_allowed}{negative_all_writes_denied}{polluted_evidence_contained}{tenant_scope_boundary_ok}{single_tenant_preview}{held_or_rolled_back}{digest_only}{negative_gates_ready} issue30_roundtrip_source=roundtrip_proof_input"
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
        let memory_admission_ledger_records =
            required_issue_field(path, index, line, "memory_admission_ledger_records")?;
        let memory_admission_ledger_preview_only =
            required_issue_field(path, index, line, "memory_admission_ledger_preview_only")?;
        let disk_kv_compact_reopen_verified =
            required_issue_field(path, index, line, "disk_kv_compact_reopen_verified")?;
        let disk_kv_compact_reopen_test =
            required_issue_field(path, index, line, "disk_kv_compact_reopen_test")?;
        let memory_admission_ledger_reopen_verified =
            required_issue_field(path, index, line, "memory_admission_ledger_reopen_verified")?;
        let memory_admission_ledger_reopen_test =
            required_issue_field(path, index, line, "memory_admission_ledger_reopen_test")?;
        let admission_review_complete = trace_admission_review_complete(path, index, line)?;
        let memory_ledger_trace_ready = trace_memory_ledger_ready(path, index, line)?;
        let trace_validation_ready = trace_validation_ready(path, index, line)?;
        return Ok(format!(
            "trace_schema_gate: passed={passed} reasoning_genome_events={reasoning_genome_events} reasoning_genome_write_allowed={reasoning_genome_write_allowed} reasoning_genome_splice_write_allowed={reasoning_genome_splice_write_allowed} self_evolution_admission_events={self_evolution_admission_events} self_evolution_admission_review_packets={self_evolution_admission_review_packets} self_evolution_admission_evidence_ids={self_evolution_admission_evidence_ids} self_evolution_admission_missing_review_packet_refs={self_evolution_admission_missing_review_packet_refs} memory_admission_ledger_records={memory_admission_ledger_records} memory_admission_ledger_preview_only={memory_admission_ledger_preview_only} disk_kv_compact_reopen_verified={disk_kv_compact_reopen_verified} disk_kv_compact_reopen_test={disk_kv_compact_reopen_test} memory_admission_ledger_reopen_verified={memory_admission_ledger_reopen_verified} memory_admission_ledger_reopen_test={memory_admission_ledger_reopen_test}{admission_review_complete}{memory_ledger_trace_ready}{trace_validation_ready} trace_report_source=trace_report_input"
        ));
    }
    Err(format!("{} has no trace report rows", path.display()))
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
            require_issue_fields(
                path,
                index,
                line,
                &[
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
                ],
            )?;
            entry_chain = Some(line.to_owned());
        } else if line.starts_with("issue377_problem_finding_present=") {
            require_issue_fields(
                path,
                index,
                line,
                &[
                    "issue377_problem_finding_present",
                    "issue377_problem_finding_id",
                    "issue377_hypothesis_candidate_present",
                    "issue377_hypothesis_candidate_id",
                    "issue377_problem_hypothesis_link",
                    "issue377_admission_decision",
                    "issue377_predicament_signal_present",
                    "issue377_predicament_id",
                    "issue377_predicament_progress_delta",
                    "issue377_predicament_repeat_count",
                    "issue377_predicament_evidence_gap_count",
                    "issue377_predicament_action_novelty",
                    "issue377_predicament_stuck",
                    "issue377_self_trigger_stage",
                    "issue377_evolution_apply_allowed",
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
    Ok(format!(
        "{entry_chain}\n{problem_hypothesis}\n{positive_context_loop_ready} issue30_context_source=issue30_context_input"
    ))
}

fn issue30_positive_context_loop_ready(
    path: &Path,
    entry_chain: &str,
    problem_hypothesis: &str,
) -> Result<String, String> {
    let derived = release_field(entry_chain, "issue30_environment_pressure_present")
        == Some("true")
        && release_field(entry_chain, "issue30_pollution_event_id")
            .is_some_and(|value| value.starts_with("redaction-digest:"))
        && release_field(entry_chain, "issue385_self_ontology_body_present") == Some("true")
        && release_field(entry_chain, "issue385_body_state_id")
            .is_some_and(|value| value.starts_with("redaction-digest:"))
        && release_field(entry_chain, "issue385_pheromone_signal_marker_present") == Some("true")
        && release_field(entry_chain, "issue385_pheromone_signal_marker_id")
            .is_some_and(|value| value.starts_with("redaction-digest:"))
        && release_field(entry_chain, "issue385_pheromone_signal_surface") == Some("digest_marker")
        && release_field(entry_chain, "issue385_pheromone_signal_digest_gate_allowed")
            == Some("true")
        && release_field(entry_chain, "issue385_pheromone_signal_preview_only") == Some("true")
        && release_field(entry_chain, "issue375_pre_reasoning_genome_isa_present") == Some("true")
        && release_field(entry_chain, "issue375_reasoning_frame_id")
            .is_some_and(|value| value.starts_with("redaction-digest:"))
        && release_field(
            entry_chain,
            "issue375_reasoning_frame_environment_signals_present",
        ) == Some("true")
        && release_field(entry_chain, "issue375_reasoning_frame_allowed_observations")
            == Some("repo_issue_terminal_runtime_state")
        && release_field(entry_chain, "issue375_reasoning_frame_action_vocab")
            == Some("observe_inspect_compare_summarize_verify_quarantine")
        && release_field(
            entry_chain,
            "issue375_reasoning_frame_suppressed_capabilities",
        ) == Some("write_process_browser_network_memory_genome_runtime")
        && release_field(entry_chain, "issue375_reasoning_frame_risk_limits")
            == Some("preview_only_digest_only")
        && release_field(entry_chain, "issue375_expression_vm_side_effect") == Some("read_only")
        && release_field(entry_chain, "issue375_genome_isa_apply_allowed") == Some("false")
        && release_field(entry_chain, "issue30_backend_action")
            .is_some_and(|value| !value.is_empty() && value != "none")
        && release_field(entry_chain, "issue379_control_candidate_preview_only") == Some("true")
        && release_field(entry_chain, "issue379_action_vocab_mask_preview") == Some("true")
        && release_field(entry_chain, "issue379_signal_saliency_bias_preview") == Some("true")
        && release_field(entry_chain, "issue379_zero_beat_primitive_decision_present")
            == Some("true")
        && release_field(entry_chain, "issue379_primitive_authority") == Some("preview_only")
        && release_field(entry_chain, "issue379_primitive_side_effect") == Some("read_only")
        && release_field(entry_chain, "issue379_primitive_reversibility")
            == Some("rollback_required")
        && release_field(entry_chain, "issue379_primitive_evidence") == Some("digest_only")
        && release_field(entry_chain, "issue379_primitive_uncertainty") == Some("hold_on_gap")
        && release_field(entry_chain, "issue379_primitive_attention")
            == Some("focus_or_mask_preview")
        && release_field(entry_chain, "issue379_zero_beat_output")
            == Some("action_vocab_mask_and_signal_saliency_bias")
        && release_field(entry_chain, "issue379_generation_bias_apply_allowed") == Some("false")
        && release_field(problem_hypothesis, "issue377_problem_finding_present") == Some("true")
        && release_field(problem_hypothesis, "issue377_problem_finding_id")
            .is_some_and(|value| value.starts_with("redaction-digest:"))
        && release_field(problem_hypothesis, "issue377_hypothesis_candidate_present")
            == Some("true")
        && release_field(problem_hypothesis, "issue377_hypothesis_candidate_id")
            .is_some_and(|value| value.starts_with("redaction-digest:"))
        && release_field(problem_hypothesis, "issue377_problem_hypothesis_link")
            .is_some_and(|value| value.starts_with("redaction-digest:"))
        && release_field(problem_hypothesis, "issue377_admission_decision") == Some("preview_only")
        && release_field(problem_hypothesis, "issue377_predicament_signal_present") == Some("true")
        && release_field(problem_hypothesis, "issue377_predicament_id")
            .is_some_and(|value| value.starts_with("redaction-digest:"))
        && release_field(problem_hypothesis, "issue377_predicament_progress_delta") == Some("0")
        && release_field(problem_hypothesis, "issue377_predicament_repeat_count") == Some("2")
        && release_field(
            problem_hypothesis,
            "issue377_predicament_evidence_gap_count",
        ) == Some("0")
        && release_field(problem_hypothesis, "issue377_predicament_action_novelty") == Some("0")
        && release_field(problem_hypothesis, "issue377_predicament_stuck") == Some("true")
        && release_field(problem_hypothesis, "issue377_self_trigger_stage") == Some("preview_only")
        && release_field(problem_hypothesis, "issue377_evolution_apply_allowed") == Some("false");
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
        if let Some(raw_value) = release_field(line, "issue30_state_files_ready") {
            if raw_value != state_files_ready.to_string() {
                return Err(format!(
                    "{}:{} issue30_state_files_ready conflicts with state file existence",
                    path.display(),
                    index + 1
                ));
            }
        }
        return Ok(format!(
            "memory_file_exists={memory_exists} experience_file_exists={experience_exists} adaptive_file_exists={adaptive_exists} issue30_state_files_ready={state_files_ready} issue30_state_files_ready_source=state_files_input_derived state_files_source=state_files_input",
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
            issue30_context_input: None,
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
            "persistent_roundtrip: passed=true second_compute_budget_saved_tokens=320 second_compute_budget_avoided_tokens=448 second_compute_budget_kv_lookups_skipped=2 second_compute_budget_anchor_count=2 second_compute_budget_anchors_preserved_count=2 second_approved_experience_reuse_digest=redaction-digest:abcdef0123456789 negative_unauthorized_write_allowed=false negative_memory_write_allowed=false negative_genome_write_allowed=false negative_self_evolution_write_allowed=false negative_polluted_evidence_quarantined=true negative_bad_candidate_digest=redaction-digest:fedcba9876543210 negative_bad_candidate_decision=hold_then_rollback negative_rollback_anchor_present=true negative_rollback_anchor_digest=redaction-digest:0123456789abcdef negative_tenant_scope_write_denied=true negative_tenant_scope_mode=local_single_user_preview negative_tenant_scope_actor=fnv64:1111111111111111 negative_tenant_scope_target=fnv64:2222222222222222 negative_tenant_scope_denial_lane=self_evolving_memory negative_tenant_scope_denial_reason=cross_tenant_scope_rejected negative_provenance_license_redaction_passed=true failures=0\n",
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
        assert!(statement.contains("second_compute_budget_anchors_preserved=true"));
        assert!(statement.contains(
            "second_compute_budget_anchors_preserved_source=roundtrip_proof_input_derived"
        ));
        assert!(statement.contains("issue30_second_task_benefit_ready=true"));
        assert!(
            statement
                .contains("issue30_second_task_benefit_ready_source=roundtrip_proof_input_derived")
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
            "persistent_roundtrip: passed=true second_compute_budget_saved_tokens=320 second_compute_budget_avoided_tokens=448 second_compute_budget_kv_lookups_skipped=2 second_compute_budget_anchor_count=2 second_compute_budget_anchors_preserved_count=2 second_approved_experience_reuse_digest=redaction-digest:abcdef0123456789 negative_unauthorized_write_allowed=false negative_memory_write_allowed=false negative_genome_write_allowed=false negative_self_evolution_write_allowed=false negative_polluted_evidence_quarantined=true negative_bad_candidate_digest=redaction-digest:fedcba9876543210 negative_bad_candidate_decision=hold_then_rollback negative_rollback_anchor_present=true negative_rollback_anchor_digest=redaction-digest:0123456789abcdef negative_tenant_scope_write_denied=true negative_tenant_scope_mode=local_single_user_preview negative_tenant_scope_actor=fnv64:1111111111111111 negative_tenant_scope_target=fnv64:1111111111111111 negative_tenant_scope_denial_lane=kv_memory negative_tenant_scope_denial_reason=missing negative_provenance_license_redaction_passed=true failures=0\n",
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
            "trace_schema_gate: passed=true lines=12 failures=0 reasoning_genome_events=2 reasoning_genome_write_allowed=0 reasoning_genome_splice_write_allowed=0 self_evolution_admission_events=1 self_evolution_admission_review_packets=1 self_evolution_admission_evidence_ids=3 self_evolution_admission_missing_review_packet_refs=0 memory_admission_ledger_records=3 memory_admission_ledger_preview_only=1 disk_kv_compact_reopen_verified=true disk_kv_compact_reopen_test=disk_kv::tests::compact_keeps_latest_values memory_admission_ledger_reopen_verified=true memory_admission_ledger_reopen_test=memory_admission::tests::writer_gate_append_is_idempotent_after_store_reopen\n",
        )
        .unwrap();

        let statement = trace_report_statement(&path).unwrap();

        assert_eq!(
            statement,
            "trace_schema_gate: passed=true reasoning_genome_events=2 reasoning_genome_write_allowed=0 reasoning_genome_splice_write_allowed=0 self_evolution_admission_events=1 self_evolution_admission_review_packets=1 self_evolution_admission_evidence_ids=3 self_evolution_admission_missing_review_packet_refs=0 memory_admission_ledger_records=3 memory_admission_ledger_preview_only=1 disk_kv_compact_reopen_verified=true disk_kv_compact_reopen_test=disk_kv::tests::compact_keeps_latest_values memory_admission_ledger_reopen_verified=true memory_admission_ledger_reopen_test=memory_admission::tests::writer_gate_append_is_idempotent_after_store_reopen self_evolution_admission_review_complete=true self_evolution_admission_review_complete_source=trace_report_input_derived issue30_memory_ledger_trace_ready=true issue30_memory_ledger_trace_ready_source=trace_report_input_derived issue30_trace_validation_ready=true issue30_trace_validation_ready_source=trace_report_input_derived trace_report_source=trace_report_input"
        );

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
            "trace_schema_gate: passed=true lines=12 failures=0 reasoning_genome_events=2 reasoning_genome_write_allowed=0 reasoning_genome_splice_write_allowed=0 self_evolution_admission_events=1 self_evolution_admission_review_packets=1 self_evolution_admission_evidence_ids=3 self_evolution_admission_missing_review_packet_refs=0 memory_admission_ledger_records=3 memory_admission_ledger_preview_only=1 disk_kv_compact_reopen_verified=false disk_kv_compact_reopen_test=disk_kv::tests::compact_keeps_latest_values memory_admission_ledger_reopen_verified=true memory_admission_ledger_reopen_test=memory_admission::tests::writer_gate_append_is_idempotent_after_store_reopen issue30_memory_ledger_trace_ready=true\n",
        )
        .unwrap();

        let error = trace_report_statement(&path).unwrap_err();

        assert!(error.contains(
            "issue30_memory_ledger_trace_ready conflicts with memory ledger reopen proof fields"
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
            "trace_schema_gate: passed=true lines=12 failures=0 reasoning_genome_events=2 reasoning_genome_write_allowed=0 reasoning_genome_splice_write_allowed=0 self_evolution_admission_events=1 self_evolution_admission_review_packets=1 self_evolution_admission_evidence_ids=3 self_evolution_admission_missing_review_packet_refs=0\n",
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

    #[test]
    fn issue30_context_statement_derives_context_rows_and_marks_source() {
        let path = std::env::temp_dir().join(format!(
            "norion-cli-issue30-context-{}.txt",
            std::process::id()
        ));
        fs::write(
            &path,
            "issue30_environment_pressure_present=true issue30_pollution_event_id=redaction-digest:dddddddddddddddd issue385_self_ontology_body_present=true issue385_body_state_id=redaction-digest:eeeeeeeeeeeeeeee issue385_pheromone_signal_marker_present=true issue385_pheromone_signal_marker_id=redaction-digest:9999999999999999 issue385_pheromone_signal_surface=digest_marker issue385_pheromone_signal_digest_gate_allowed=true issue385_pheromone_signal_preview_only=true issue375_pre_reasoning_genome_isa_present=true issue375_reasoning_frame_id=redaction-digest:ffffffffffffffff issue375_reasoning_frame_environment_signals_present=true issue375_reasoning_frame_allowed_observations=repo_issue_terminal_runtime_state issue375_reasoning_frame_action_vocab=observe_inspect_compare_summarize_verify_quarantine issue375_reasoning_frame_suppressed_capabilities=write_process_browser_network_memory_genome_runtime issue375_reasoning_frame_risk_limits=preview_only_digest_only issue375_expression_vm_side_effect=read_only issue375_genome_isa_apply_allowed=false issue30_backend_action=deterministic_runtime_kv_roundtrip issue379_control_candidate_preview_only=true issue379_action_vocab_mask_preview=true issue379_signal_saliency_bias_preview=true issue379_zero_beat_primitive_decision_present=true issue379_primitive_authority=preview_only issue379_primitive_side_effect=read_only issue379_primitive_reversibility=rollback_required issue379_primitive_evidence=digest_only issue379_primitive_uncertainty=hold_on_gap issue379_primitive_attention=focus_or_mask_preview issue379_zero_beat_output=action_vocab_mask_and_signal_saliency_bias issue379_generation_bias_apply_allowed=false\nissue377_problem_finding_present=true issue377_problem_finding_id=redaction-digest:aaaaaaaaaaaaaaaa issue377_hypothesis_candidate_present=true issue377_hypothesis_candidate_id=redaction-digest:bbbbbbbbbbbbbbbb issue377_problem_hypothesis_link=redaction-digest:cccccccccccccccc issue377_admission_decision=preview_only issue377_predicament_signal_present=true issue377_predicament_id=redaction-digest:dddddddddddddddd issue377_predicament_progress_delta=0 issue377_predicament_repeat_count=2 issue377_predicament_evidence_gap_count=0 issue377_predicament_action_novelty=0 issue377_predicament_stuck=true issue377_self_trigger_stage=preview_only issue377_evolution_apply_allowed=false\n",
        )
        .unwrap();

        let statement = issue30_context_statement(&path).unwrap();

        assert!(statement.contains("issue30_environment_pressure_present=true"));
        assert!(statement.contains("issue30_backend_action=deterministic_runtime_kv_roundtrip"));
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
        assert!(statement.contains("issue377_problem_finding_present=true"));
        assert!(statement.contains("issue377_admission_decision=preview_only"));
        assert!(statement.contains("issue377_predicament_signal_present=true"));
        assert!(statement.contains("issue377_predicament_id=redaction-digest:"));
        assert!(statement.contains("issue377_predicament_progress_delta=0"));
        assert!(statement.contains("issue377_predicament_repeat_count=2"));
        assert!(statement.contains("issue377_predicament_evidence_gap_count=0"));
        assert!(statement.contains("issue377_predicament_action_novelty=0"));
        assert!(statement.contains("issue377_predicament_stuck=true"));
        assert!(statement.contains("issue377_self_trigger_stage=preview_only"));
        assert!(statement.contains("issue377_evolution_apply_allowed=false"));
        assert!(statement.contains("issue30_positive_context_loop_ready=true"));
        assert!(
            statement.contains(
                "issue30_positive_context_loop_ready_source=issue30_context_input_derived"
            )
        );
        assert!(statement.contains("issue30_context_source=issue30_context_input"));

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
                "memory={} experience={} adaptive={}\n",
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
        assert!(statement.contains("issue30_state_files_ready=true"));
        assert!(statement.contains("issue30_state_files_ready_source=state_files_input_derived"));
        assert!(statement.contains("state_files_source=state_files_input"));
        assert!(!statement.contains(&dir.display().to_string()));

        let _ = fs::remove_dir_all(dir);
    }
}
