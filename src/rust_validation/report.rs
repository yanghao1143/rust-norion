use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustSnippetCheckReport {
    pub passed: bool,
    pub edition: String,
    pub status_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub source_path: PathBuf,
    pub metadata_path: PathBuf,
}

impl RustSnippetCheckReport {
    pub fn diagnostic_chars(&self) -> usize {
        self.stdout
            .chars()
            .count()
            .saturating_add(self.stderr.chars().count())
    }

    pub fn feedback_label(&self) -> &'static str {
        if self.passed {
            "rustc_passed"
        } else {
            "rustc_failed"
        }
    }
}
