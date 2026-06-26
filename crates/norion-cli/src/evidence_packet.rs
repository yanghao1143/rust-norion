use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidencePacketConfig {
    pub issue: String,
    pub commit: String,
    pub command: String,
    pub gate: String,
    pub input: PathBuf,
    pub output: Option<PathBuf>,
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
    })
}

pub fn run_evidence_packet(config: &EvidencePacketConfig) -> Result<String, String> {
    let raw = fs::read_to_string(&config.input)
        .map_err(|error| format!("failed to read {}: {error}", config.input.display()))?;
    let packet = render_evidence_packet(config, &raw);
    if let Some(path) = &config.output {
        fs::write(path, &packet)
            .map_err(|error| format!("failed to write {}: {error}", path.display()))?;
    }
    Ok(packet)
}

fn render_evidence_packet(config: &EvidencePacketConfig, raw: &str) -> String {
    format!(
        "## Evidence packet for #{}\n- commit: {}\n- command: {}\n- gate: {}\n\n```text\n{}\n```\n",
        config.issue.trim_start_matches('#'),
        config.commit,
        redact(&config.command),
        config.gate,
        redact(raw).trim_end()
    )
}

fn redact(text: &str) -> String {
    text.lines().map(redact_line).collect::<Vec<_>>().join("\n")
}

fn redact_line(line: &str) -> String {
    let lower = line.to_ascii_lowercase();
    if ["api_key", "apikey", "token", "secret", "password"]
        .iter()
        .any(|marker| lower.contains(marker))
    {
        if let Some((name, _)) = line.split_once('=') {
            return format!("{}=<redacted>", name.trim_end());
        }
        return "<redacted>".to_owned();
    }
    line.split_whitespace()
        .map(|word| {
            if ["ghp_", "github_pat_", "sk-", "xoxb-"]
                .iter()
                .any(|prefix| word.starts_with(prefix))
            {
                "<redacted>"
            } else {
                word
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
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
        };

        let packet = render_evidence_packet(
            &config,
            "ok\nOPENAI_API_KEY=sk-leak\nplain ghp_alsoleak done\n",
        );

        assert!(packet.contains("## Evidence packet for #48"));
        assert!(packet.contains("- command: cargo test -p norion-cli -- token=<redacted>"));
        assert!(packet.contains("OPENAI_API_KEY=<redacted>"));
        assert!(packet.contains("plain <redacted> done"));
        assert!(!packet.contains("sk-leak"));
        assert!(!packet.contains("ghp_alsoleak"));
    }
}
