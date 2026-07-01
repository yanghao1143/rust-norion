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
        required: required_fields,
        rejected: rejected_fields,
    })
}

pub fn run_evidence_packet(config: &EvidencePacketConfig) -> Result<String, String> {
    let raw = fs::read_to_string(&config.input)
        .map_err(|error| format!("failed to read {}: {error}", config.input.display()))?;
    let git_statement = config
        .git_worktree
        .as_deref()
        .map(git_dirty_statement)
        .transpose()?;
    let packet = render_evidence_packet(config, &raw, git_statement.as_deref());
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
    git_statement: Option<&str>,
) -> String {
    let git_statement = git_statement
        .map(|statement| format!("{statement}\n"))
        .unwrap_or_default();
    format!(
        "## Evidence packet for #{}\n- commit: {}\n- command: {}\n- gate: {}\n\n```text\n{}{}\n```\n",
        config.issue.trim_start_matches('#'),
        config.commit,
        redact(&config.command),
        config.gate,
        git_statement,
        redact(raw).trim_end()
    )
}

fn git_dirty_statement(worktree: &Path) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(worktree)
        .args(["status", "--short"])
        .output()
        .map_err(|error| {
            format!(
                "failed to run git status for {}: {error}",
                worktree.display()
            )
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "git status failed for {}: {}",
            worktree.display(),
            stderr.trim()
        ));
    }
    let dirty = !String::from_utf8_lossy(&output.stdout).trim().is_empty();
    Ok(format!(
        "dirty_worktree={dirty} dirty_worktree_source=git_status"
    ))
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
            required: vec![
                "OPENAI_API_KEY=<redacted>".to_owned(),
                "payload_line=<redacted-payload>".to_owned(),
            ],
            rejected: vec!["C:\\Users".to_owned(), "private raw prompt".to_owned()],
        };

        let packet = render_evidence_packet(
            &config,
            "ok\nOPENAI_API_KEY=sk-leak\npath=C:\\Users\\jy\\AppData\\Local\\Temp\\run.txt\nprompt: private raw prompt\nanswer_text=raw answer\nid=3 key=runtime_kv :: Design a Rust Noiron prototype lesson=reuse_response: raw model output\nplain ghp_alsoleak done\n",
            None,
        );

        validate_packet(&config, &packet).expect("packet should pass required and rejected gates");
        assert!(packet.contains("## Evidence packet for #48"));
        assert!(packet.contains("- command: cargo test -p norion-cli -- token=<redacted>"));
        assert!(redact("saved_tokens=12 avoided_tokens=8").contains("saved_tokens=12"));
        assert!(packet.contains("OPENAI_API_KEY=<redacted>"));
        assert!(packet.contains("path=<redacted-path>"));
        assert!(packet.contains("prompt=<redacted-payload>"));
        assert!(packet.contains("answer_text=<redacted-payload>"));
        assert!(packet.contains("payload_line=<redacted-payload>"));
        assert!(packet.contains("plain <redacted> done"));
        assert!(!packet.contains("sk-leak"));
        assert!(!packet.contains("C:\\Users"));
        assert!(!packet.contains("AppData"));
        assert!(!packet.contains("private raw prompt"));
        assert!(!packet.contains("raw answer"));
        assert!(!packet.contains("Design a Rust Noiron prototype"));
        assert!(!packet.contains("reuse_response"));
        assert!(!packet.contains("ghp_alsoleak"));
    }
}
