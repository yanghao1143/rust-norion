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
    Ok(format!(
        "rc_sha={rc_sha} rc_sha_source=git_rev_parse rc_branch={rc_branch} rc_branch_source=git_branch dirty_worktree={dirty} dirty_worktree_source=git_status"
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
                issue19_runtime_counters_ready = Some(required_issue_field(
                    path,
                    index,
                    line,
                    "runtime_counters_ready",
                )?);
                let runtime_counters_head =
                    required_issue_field(path, index, line, "runtime_counters_head")?;
                let runtime_counters_checks =
                    required_issue_field(path, index, line, "runtime_counters_checks")?;
                let runtime_counters_review =
                    required_issue_field(path, index, line, "runtime_counters_review")?;
                let runtime_counters_merged =
                    required_issue_field(path, index, line, "runtime_counters_merged")?;
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
        "issue31_final_signoff_present={} issue31_final_signoff_source=issue_state_input issue19_runtime_surface_closed={} issue19_runtime_surface_merged_prs={} issue19_runtime_counters_pr={} issue19_runtime_counters_ready={} issue19_runtime_counters_state={} issue19_runtime_counters_state_source=issue_state_input_derived issue19_runtime_surface_blocker={} issue19_runtime_surface_source=issue_state_input issue30_close_allowed={} issue30_close_allowed_source=issue_state_input",
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
        return Ok(format!(
            "clean_checkout={clean_checkout} live_model_required={live_model_required} private_state_required={private_state_required} prompt_digest_ref={prompt_digest_ref} issue30_demo_integration_test={integration_test} issue30_demo_dispatch_test={dispatch_test} issue30_demo_dispatch_path={dispatch_path} issue30_demo_trace_schema_gate_executed={trace_schema_gate_executed} issue30_demo_source=demo_proof_input"
        ));
    }
    Err(format!("{} has no demo proof rows", path.display()))
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
        return Ok(format!(
            "{line} issue30_roundtrip_source=roundtrip_proof_input"
        ));
    }
    Err(format!("{} has no roundtrip proof rows", path.display()))
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
        return Ok(format!(
            "trace_schema_gate: passed={passed} reasoning_genome_events={reasoning_genome_events} reasoning_genome_write_allowed={reasoning_genome_write_allowed} reasoning_genome_splice_write_allowed={reasoning_genome_splice_write_allowed} self_evolution_admission_events={self_evolution_admission_events} self_evolution_admission_review_packets={self_evolution_admission_review_packets} self_evolution_admission_evidence_ids={self_evolution_admission_evidence_ids} self_evolution_admission_missing_review_packet_refs={self_evolution_admission_missing_review_packet_refs} trace_report_source=trace_report_input"
        ));
    }
    Err(format!("{} has no trace report rows", path.display()))
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
        return Ok(format!(
            "state_inspection_gate: passed={passed} failures={failures} state_gate_source=state_gate_input"
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
                    "issue375_pre_reasoning_genome_isa_present",
                    "issue375_reasoning_frame_id",
                    "issue30_backend_action",
                    "issue379_control_candidate_preview_only",
                    "issue379_action_vocab_mask_preview",
                    "issue379_signal_saliency_bias_preview",
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
    Ok(format!(
        "{}\n{}\nissue30_context_source=issue30_context_input",
        required_state(&entry_chain, path, "issue30 entry chain row")?,
        required_state(&problem_hypothesis, path, "issue377 problem hypothesis row")?,
    ))
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
        return Ok(format!(
            "memory_file_exists={} experience_file_exists={} adaptive_file_exists={} state_files_source=state_files_input",
            Path::new(&memory).exists(),
            Path::new(&experience).exists(),
            Path::new(&adaptive).exists()
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
            "issue=31 state=open final_signoff=false\nissue=19 state=open runtime_surface_closed=false runtime_surface_merged_prs=#290,#291 runtime_counters_pr=#429 runtime_counters_ready=false runtime_counters_head=6f049dd02f1c8352939f9a9356f2b2f90ce07569 runtime_counters_checks=green runtime_counters_review=review_required runtime_counters_merged=false runtime_surface_blocker=#429:REVIEW_REQUIRED\nissue=30 state=open close_allowed=false\n",
        )
        .unwrap();

        let statement = issue_state_statement(&path).unwrap();

        assert!(statement.contains("issue31_final_signoff_present=false"));
        assert!(statement.contains("issue31_final_signoff_source=issue_state_input"));
        assert!(statement.contains("issue19_runtime_surface_closed=false"));
        assert!(statement.contains("issue19_runtime_surface_merged_prs=#290,#291"));
        assert!(statement.contains("issue19_runtime_counters_pr=#429"));
        assert!(statement.contains(
            "issue19_runtime_counters_state=head_6f049dd_checks_green_review_required_unmerged"
        ));
        assert!(
            statement.contains("issue19_runtime_counters_state_source=issue_state_input_derived")
        );
        assert!(statement.contains("issue19_runtime_surface_blocker=#429:REVIEW_REQUIRED"));
        assert!(statement.contains("issue19_runtime_surface_source=issue_state_input"));
        assert!(statement.contains("issue30_close_allowed=false"));
        assert!(statement.contains("issue30_close_allowed_source=issue_state_input"));

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
            "persistent_roundtrip: passed=true second_compute_budget_saved_tokens=320 negative_unauthorized_write_allowed=false failures=0\n",
        )
        .unwrap();

        let statement = roundtrip_proof_statement(&path).unwrap();

        assert!(statement.contains("persistent_roundtrip: passed=true"));
        assert!(statement.contains("second_compute_budget_saved_tokens=320"));
        assert!(statement.contains("negative_unauthorized_write_allowed=false"));
        assert!(statement.contains("failures=0"));
        assert!(statement.contains("issue30_roundtrip_source=roundtrip_proof_input"));

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
            "trace_schema_gate: passed=true lines=12 failures=0 reasoning_genome_events=2 reasoning_genome_write_allowed=0 reasoning_genome_splice_write_allowed=0 self_evolution_admission_events=1 self_evolution_admission_review_packets=1 self_evolution_admission_evidence_ids=3 self_evolution_admission_missing_review_packet_refs=0\n",
        )
        .unwrap();

        let statement = trace_report_statement(&path).unwrap();

        assert_eq!(
            statement,
            "trace_schema_gate: passed=true reasoning_genome_events=2 reasoning_genome_write_allowed=0 reasoning_genome_splice_write_allowed=0 self_evolution_admission_events=1 self_evolution_admission_review_packets=1 self_evolution_admission_evidence_ids=3 self_evolution_admission_missing_review_packet_refs=0 trace_report_source=trace_report_input"
        );

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
            "state_inspection_gate: passed=true failures=0 state_gate_source=state_gate_input"
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
            "issue30_environment_pressure_present=true issue30_pollution_event_id=redaction-digest:dddddddddddddddd issue385_self_ontology_body_present=true issue385_body_state_id=redaction-digest:eeeeeeeeeeeeeeee issue375_pre_reasoning_genome_isa_present=true issue375_reasoning_frame_id=redaction-digest:ffffffffffffffff issue30_backend_action=deterministic_runtime_kv_roundtrip issue379_control_candidate_preview_only=true issue379_action_vocab_mask_preview=true issue379_signal_saliency_bias_preview=true\nissue377_problem_finding_present=true issue377_problem_finding_id=redaction-digest:aaaaaaaaaaaaaaaa issue377_hypothesis_candidate_present=true issue377_hypothesis_candidate_id=redaction-digest:bbbbbbbbbbbbbbbb issue377_problem_hypothesis_link=redaction-digest:cccccccccccccccc issue377_admission_decision=preview_only\n",
        )
        .unwrap();

        let statement = issue30_context_statement(&path).unwrap();

        assert!(statement.contains("issue30_environment_pressure_present=true"));
        assert!(statement.contains("issue30_backend_action=deterministic_runtime_kv_roundtrip"));
        assert!(statement.contains("issue377_problem_finding_present=true"));
        assert!(statement.contains("issue377_admission_decision=preview_only"));
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

        assert_eq!(
            statement,
            "memory_file_exists=true experience_file_exists=true adaptive_file_exists=true state_files_source=state_files_input"
        );
        assert!(!statement.contains(&dir.display().to_string()));

        let _ = fs::remove_dir_all(dir);
    }
}
