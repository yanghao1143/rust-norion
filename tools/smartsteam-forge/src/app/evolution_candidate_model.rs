use std::path::{Path, PathBuf};

pub(super) const REPORT_FILE: &str = "report.json";
pub(super) const LEDGER_FILE: &str = "evolution-ledger.jsonl";
pub(super) const BACKLOG_FILE: &str = "evolution-candidates.jsonl";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct EvolutionCandidate {
    pub(super) source: String,
    pub(super) round: String,
    pub(super) case_name: String,
    pub(super) model: String,
    pub(super) tokens: String,
    pub(super) elapsed_ms: String,
    pub(super) feedback: String,
    pub(super) self_improve: String,
    pub(super) answer_preview: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct EvolutionCandidateBatch {
    pub(super) source: String,
    pub(super) source_path: PathBuf,
    pub(super) candidates: Vec<EvolutionCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct EvolutionCandidateBacklogItem {
    pub(super) candidate_id: String,
    pub(super) status: String,
    pub(super) source: String,
    pub(super) round: String,
    pub(super) case_name: String,
    pub(super) model: String,
    pub(super) tokens: String,
    pub(super) elapsed_ms: String,
    pub(super) feedback: String,
    pub(super) self_improve: String,
    pub(super) answer_preview: String,
    pub(super) note: String,
    pub(super) changed_unix: String,
    pub(super) validation_command: String,
    pub(super) validation_status_code: String,
    pub(super) validation_passed: String,
    pub(super) validation_note: String,
    pub(super) validation_unix: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CandidateLifecycleGate {
    pub(super) path: PathBuf,
    pub(super) exists: bool,
    pub(super) total: usize,
    pub(super) invalid_count: usize,
    pub(super) implemented_validated_count: usize,
    pub(super) accepted_pending_ids: Vec<String>,
    pub(super) implemented_unvalidated_ids: Vec<String>,
    pub(super) implemented_failed_ids: Vec<String>,
}

impl CandidateLifecycleGate {
    pub(super) fn ready(&self) -> bool {
        self.invalid_count == 0
            && self.accepted_pending_ids.is_empty()
            && self.implemented_unvalidated_ids.is_empty()
            && self.implemented_failed_ids.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct BacklogAppendResult {
    pub(super) path: PathBuf,
    pub(super) existing: usize,
    pub(super) appended: usize,
    pub(super) skipped: usize,
}

impl BacklogAppendResult {
    pub(super) fn summary_line(&self) -> String {
        format!(
            "backlog path={} existing={} appended={} skipped_duplicate={}",
            self.path.display(),
            self.existing,
            self.appended,
            self.skipped
        )
    }
}

pub(super) struct EvolutionCandidatePaths {
    pub(super) report: PathBuf,
    pub(super) ledger: PathBuf,
}

impl EvolutionCandidatePaths {
    pub(super) fn new(work_dir: &str) -> Self {
        let work_dir = Path::new(work_dir);
        Self {
            report: work_dir.join(REPORT_FILE),
            ledger: work_dir.join(LEDGER_FILE),
        }
    }

    pub(super) fn backlog(&self) -> PathBuf {
        self.report
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(BACKLOG_FILE)
    }
}

pub(super) fn candidate_backlog_path(
    paths: &EvolutionCandidatePaths,
    path: Option<&str>,
) -> PathBuf {
    match path.map(str::trim).filter(|value| !value.is_empty()) {
        Some(path) => PathBuf::from(path),
        None => paths.backlog(),
    }
}
