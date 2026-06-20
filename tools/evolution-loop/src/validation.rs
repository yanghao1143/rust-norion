use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ValidationResult {
    pub(crate) status_code: Option<i32>,
    pub(crate) elapsed_ms: u64,
    pub(crate) stdout_tail: String,
    pub(crate) stderr_tail: String,
}

pub(crate) fn test_gate_validation_command_safety(command: Option<&str>) -> &'static str {
    let Some(command) = command.map(str::trim).filter(|command| !command.is_empty()) else {
        return "missing";
    };
    if !is_safe_test_gate_validation_command(command) {
        return "unsafe";
    }
    "safe"
}

pub(crate) fn is_safe_test_gate_validation_command(command: &str) -> bool {
    let command = command.trim();
    if command.is_empty() || command.chars().count() > 240 {
        return false;
    }
    if command
        .chars()
        .any(|character| matches!(character, '\n' | '\r'))
    {
        return false;
    }
    if command
        .chars()
        .any(|character| matches!(character, ';' | '&' | '|' | '>' | '<' | '`'))
    {
        return false;
    }
    let tokens = command.split_whitespace().collect::<Vec<_>>();
    let [binary, subcommand, rest @ ..] = tokens.as_slice() else {
        return false;
    };
    let binary = binary.trim_matches('"').to_ascii_lowercase();
    if !matches!(binary.as_str(), "cargo" | "cargo.exe") {
        return false;
    }
    let subcommand = subcommand.to_ascii_lowercase();
    if !matches!(subcommand.as_str(), "test" | "check" | "clippy" | "fmt") {
        return false;
    }
    if rest.iter().any(|token| {
        let token = token.to_ascii_lowercase();
        matches!(
            token.as_str(),
            "--fix"
                | "fix"
                | "run"
                | "install"
                | "publish"
                | "clean"
                | "remove"
                | "rm"
                | "del"
                | "erase"
        ) || token.starts_with('@')
    }) {
        return false;
    }
    subcommand != "fmt"
        || rest
            .iter()
            .any(|token| token.eq_ignore_ascii_case("--check"))
}

pub(crate) fn run_command(
    command: &str,
    workdir: Option<&Path>,
    timeout_secs: u64,
) -> Result<ValidationResult, String> {
    let mut process = shell_command(command);
    if let Some(workdir) = workdir {
        process.current_dir(workdir);
    }
    let mut child = process
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("spawn validation command failed: {error}"))?;

    let stdout_handle = child.stdout.take().map(read_pipe_async);
    let stderr_handle = child.stderr.take().map(read_pipe_async);
    let started = Instant::now();
    let timeout = Duration::from_secs(timeout_secs.max(1));

    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if started.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    let stdout = join_pipe(stdout_handle);
                    let stderr = join_pipe(stderr_handle);
                    return Err(format!(
                        "validation command timed out after {}s stdout_tail={} stderr_tail={}",
                        timeout.as_secs(),
                        preview_tail(&stdout, 320),
                        preview_tail(&stderr, 320)
                    ));
                }
                thread::sleep(Duration::from_millis(100));
            }
            Err(error) => return Err(format!("poll validation command failed: {error}")),
        }
    };

    let elapsed_ms = started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
    let stdout = join_pipe(stdout_handle);
    let stderr = join_pipe(stderr_handle);
    Ok(ValidationResult {
        status_code: status.code(),
        elapsed_ms,
        stdout_tail: preview_tail(&stdout, 640),
        stderr_tail: preview_tail(&stderr, 640),
    })
}

pub(crate) fn failure_message(phase: &str, command: &str, result: &ValidationResult) -> String {
    format!(
        "validation gate {phase} failed: command={} status={} elapsed_ms={} stdout_tail={} stderr_tail={}",
        command,
        option_i32_text(result.status_code),
        result.elapsed_ms,
        empty_as_dash(&result.stdout_tail),
        empty_as_dash(&result.stderr_tail)
    )
}

fn shell_command(command: &str) -> Command {
    if cfg!(windows) {
        let mut process = Command::new("cmd.exe");
        process.arg("/C").arg(command);
        process
    } else {
        let mut process = Command::new("sh");
        process.arg("-c").arg(command);
        process
    }
}

fn read_pipe_async<R>(mut pipe: R) -> thread::JoinHandle<String>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut text = String::new();
        let _ = pipe.read_to_string(&mut text);
        text
    })
}

fn join_pipe(handle: Option<thread::JoinHandle<String>>) -> String {
    handle
        .and_then(|handle| handle.join().ok())
        .unwrap_or_default()
}

fn preview_tail(text: &str, max_chars: usize) -> String {
    let compact = text
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .split('\n')
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" / ");
    let count = compact.chars().count();
    if count <= max_chars {
        return compact;
    }
    let start = count.saturating_sub(max_chars);
    format!("...{}", compact.chars().skip(start).collect::<String>())
}

fn option_i32_text(value: Option<i32>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "unknown".to_owned())
}

fn empty_as_dash(value: &str) -> &str {
    if value.is_empty() { "-" } else { value }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_command_captures_success() {
        let result = run_command("echo validation-ok", None, 5).unwrap();

        assert_eq!(result.status_code, Some(0));
        assert!(result.stdout_tail.contains("validation-ok"));
    }

    #[test]
    fn validation_failure_message_includes_status_and_stderr() {
        let result = ValidationResult {
            status_code: Some(7),
            elapsed_ms: 12,
            stdout_tail: String::new(),
            stderr_tail: "boom".to_owned(),
        };
        let message = failure_message("pre", "exit 7", &result);

        assert!(message.contains("validation gate pre failed"));
        assert!(message.contains("status=7"));
        assert!(message.contains("stderr_tail=boom"));
        assert!(message.contains("stdout_tail=-"));
    }

    #[test]
    fn validation_command_reports_non_zero_status() {
        let result = run_command("exit 7", None, 5).unwrap();

        assert_eq!(result.status_code, Some(7));
    }

    #[test]
    fn test_gate_validation_command_safety_is_conservative() {
        assert_eq!(
            test_gate_validation_command_safety(Some(
                "cargo test -q --manifest-path tools/evolution-loop/Cargo.toml"
            )),
            "safe"
        );
        assert_eq!(
            test_gate_validation_command_safety(Some(
                "cargo test -q --manifest-path tools/evolution-loop/Cargo.toml --no-fail-fast"
            )),
            "safe"
        );
        assert_eq!(
            test_gate_validation_command_safety(Some("cargo check --workspace")),
            "safe"
        );
        assert_eq!(
            test_gate_validation_command_safety(Some("cargo clippy --all-targets -- -D warnings")),
            "safe"
        );
        assert_eq!(
            test_gate_validation_command_safety(Some("cargo fmt --check")),
            "safe"
        );
        assert_eq!(
            test_gate_validation_command_safety(Some("cargo fmt")),
            "unsafe"
        );
        assert_eq!(
            test_gate_validation_command_safety(Some("cargo test; Remove-Item target")),
            "unsafe"
        );
        assert_eq!(
            test_gate_validation_command_safety(Some("cargo clippy --fix")),
            "unsafe"
        );
        assert_eq!(test_gate_validation_command_safety(None), "missing");
    }
}
