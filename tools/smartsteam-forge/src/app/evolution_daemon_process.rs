use std::{
    env,
    ffi::OsString,
    fs::{self, File},
    io,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

use super::evolution_daemon_args::{
    EvolutionDaemonAction, EvolutionDaemonStartOptions, daemon_control_args, evolution_action_name,
};
use super::status_json::compact_line;

pub(super) fn invoke_evolution_daemon_control(
    action: EvolutionDaemonAction,
    work_dir: &str,
    backend: Option<&str>,
    prompt: Option<&str>,
    check_only: bool,
    start_options: EvolutionDaemonStartOptions,
) -> io::Result<String> {
    let script = evolution_daemon_script()?;
    let capture = TempCommandCapture::new("smartsteam-forge-evolution-daemon")?;
    let stdout_file = File::create(&capture.stdout_path)?;
    let stderr_file = File::create(&capture.stderr_path)?;
    let mut command = Command::new("powershell.exe");
    command
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(script)
        .args(daemon_control_args(
            action,
            work_dir,
            backend,
            prompt,
            check_only,
            start_options,
        ))
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file));
    let status = command.status().map_err(|error| {
        io::Error::other(format!(
            "evolution daemon {} command failed: {error}",
            evolution_action_name(action)
        ))
    })?;
    let stdout = capture.read_stdout();
    let stderr = capture.read_stderr();
    capture.cleanup();

    if !status.success() {
        return Err(io::Error::other(format!(
            "evolution daemon {} exited with {}: {}{}",
            evolution_action_name(action),
            status,
            compact_error_text(&stderr),
            compact_error_suffix(&stdout)
        )));
    }

    Ok(stdout)
}

pub(super) fn load_evolution_status_with_backend(
    work_dir: &str,
    backend: Option<&str>,
) -> io::Result<String> {
    let script = evolution_daemon_script()?;
    let output = Command::new("powershell.exe")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(script)
        .args(daemon_status_args(work_dir, backend))
        .output()
        .map_err(|error| io::Error::other(format!("evolution status command failed: {error}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(io::Error::other(format!(
            "evolution status command exited with {}: {}{}",
            output.status,
            compact_error_text(&stderr),
            compact_error_suffix(&stdout)
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn daemon_status_args(work_dir: &str, backend: Option<&str>) -> Vec<OsString> {
    let mut args = vec![
        OsString::from("-JsonStatus"),
        OsString::from("-WorkDir"),
        OsString::from(work_dir),
    ];
    if let Some(backend) = backend.map(str::trim).filter(|value| !value.is_empty()) {
        args.push(OsString::from("-Backend"));
        args.push(OsString::from(backend));
    }
    args
}

struct TempCommandCapture {
    stdout_path: PathBuf,
    stderr_path: PathBuf,
}

impl TempCommandCapture {
    fn new(prefix: &str) -> io::Result<Self> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| io::Error::other(format!("clock error: {error}")))?
            .as_millis();
        let stem = format!("{prefix}-{}-{now}", std::process::id());
        let dir = env::temp_dir();
        Ok(Self {
            stdout_path: dir.join(format!("{stem}.out.log")),
            stderr_path: dir.join(format!("{stem}.err.log")),
        })
    }

    fn read_stdout(&self) -> String {
        fs::read_to_string(&self.stdout_path).unwrap_or_default()
    }

    fn read_stderr(&self) -> String {
        fs::read_to_string(&self.stderr_path).unwrap_or_default()
    }

    fn cleanup(&self) {
        let _ = fs::remove_file(&self.stdout_path);
        let _ = fs::remove_file(&self.stderr_path);
    }
}

fn evolution_daemon_script() -> io::Result<PathBuf> {
    evolution_repo_root()
        .map(|root| {
            root.join("tools")
                .join("evolution-loop")
                .join("daemon-evolution-loop.ps1")
        })
        .filter(|script| script.is_file())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "could not find tools\\evolution-loop\\daemon-evolution-loop.ps1",
            )
        })
}

pub(super) fn evolution_loop_start_script() -> io::Result<PathBuf> {
    evolution_repo_root()
        .map(|root| {
            root.join("tools")
                .join("evolution-loop")
                .join("start-evolution-loop.ps1")
        })
        .filter(|script| script.is_file())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "could not find tools\\evolution-loop\\start-evolution-loop.ps1",
            )
        })
}

pub(super) fn resolve_repo_path(path: &str) -> io::Result<PathBuf> {
    let path = Path::new(path);
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    evolution_repo_root()
        .map(|root| root.join(path))
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "could not find repository root"))
}

pub(super) fn evolution_repo_root() -> Option<PathBuf> {
    env::current_dir()
        .ok()
        .as_deref()
        .and_then(find_repo_root_from)
        .or_else(|| find_repo_root_from(Path::new(env!("CARGO_MANIFEST_DIR"))))
}

fn find_repo_root_from(start: &Path) -> Option<PathBuf> {
    start.ancestors().find_map(|candidate| {
        let script = candidate
            .join("tools")
            .join("evolution-loop")
            .join("daemon-evolution-loop.ps1");
        script.is_file().then(|| candidate.to_path_buf())
    })
}

fn compact_error_text(value: &str) -> String {
    let compact = compact_line(value, 240);
    if compact.is_empty() {
        "no stderr".to_owned()
    } else {
        compact
    }
}

fn compact_error_suffix(value: &str) -> String {
    let compact = compact_line(value, 240);
    if compact.is_empty() {
        String::new()
    } else {
        format!(" stdout={compact}")
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn finds_repo_root_from_tool_directory() {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let root = find_repo_root_from(manifest_dir).unwrap();

        assert!(root.join("tools").join("evolution-loop").is_dir());
    }

    #[test]
    fn script_path_points_to_daemon_status_script() {
        let path = evolution_daemon_script().unwrap();

        assert_eq!(
            path.file_name()
                .map(|name| name.to_string_lossy().to_string()),
            Some("daemon-evolution-loop.ps1".to_owned())
        );
    }

    #[test]
    fn status_args_forward_backend_when_provided() {
        let args = daemon_status_args("target\\evolution\\daemon", Some("127.0.0.1:8789"))
            .iter()
            .map(|value| value.to_string_lossy().to_string())
            .collect::<Vec<_>>();

        assert!(
            args.windows(2)
                .any(|pair| pair == ["-JsonStatus", "-WorkDir"])
        );
        assert!(
            args.windows(2)
                .any(|pair| pair == ["-Backend", "127.0.0.1:8789"])
        );
    }
}
