use std::fs;
use std::io;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use super::{
    RustSnippetCheck, RustSnippetCheckReport,
    output::{
        DEFAULT_RUSTC_PROGRAM, MAX_DIAGNOSTIC_CHARS, bounded_output, normalize_edition,
        sanitize_path_segment, validate_snippet_input,
    },
};

#[derive(Debug, Clone)]
pub struct RustSnippetValidator {
    rustc_program: PathBuf,
    work_dir: PathBuf,
    max_diagnostic_chars: usize,
}

impl RustSnippetValidator {
    pub fn new(work_dir: impl Into<PathBuf>) -> Self {
        Self {
            rustc_program: PathBuf::from(DEFAULT_RUSTC_PROGRAM),
            work_dir: work_dir.into(),
            max_diagnostic_chars: MAX_DIAGNOSTIC_CHARS,
        }
    }

    pub fn with_rustc_program(mut self, rustc_program: impl Into<PathBuf>) -> Self {
        self.rustc_program = rustc_program.into();
        self
    }

    pub fn check(&self, request: &RustSnippetCheck) -> io::Result<RustSnippetCheckReport> {
        validate_snippet_input(request)?;
        let edition = normalize_edition(&request.edition)?;
        let case_dir = self.case_dir(request.case_name.as_deref())?;
        fs::create_dir_all(&case_dir)?;

        let source_path = case_dir.join("lib.rs");
        let metadata_path = case_dir.join("check.rmeta");
        fs::write(&source_path, request.code.as_bytes())?;

        let output = Command::new(&self.rustc_program)
            .arg("--edition")
            .arg(&edition)
            .arg("--crate-type")
            .arg("lib")
            .arg("--emit=metadata")
            .arg("-o")
            .arg("check.rmeta")
            .arg("lib.rs")
            .current_dir(&case_dir)
            .output()?;

        let passed = output.status.success() && metadata_path.is_file();
        Ok(RustSnippetCheckReport {
            passed,
            edition,
            status_code: output.status.code(),
            stdout: bounded_output(&output.stdout, self.max_diagnostic_chars),
            stderr: bounded_output(&output.stderr, self.max_diagnostic_chars),
            source_path,
            metadata_path,
        })
    }

    fn case_dir(&self, case_name: Option<&str>) -> io::Result<PathBuf> {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_nanos();
        let case = sanitize_path_segment(case_name.unwrap_or("rust-check"));
        Ok(self.work_dir.join(format!("{case}-{stamp}")))
    }
}
