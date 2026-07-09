use crate::privacy_redaction::stable_redaction_digest;
use crate::reflection::{InferenceDraft, ReasoningStep, Reflector};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailureSeededReflectionBenchmarkReport {
    pub cases: usize,
    pub reflection_issues: usize,
    pub critical_reflection_issues: usize,
    pub revision_actions: usize,
    pub rollback_holds: usize,
    pub candidate_digest: String,
    pub read_only: bool,
    pub report_only: bool,
    pub preview_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl FailureSeededReflectionBenchmarkReport {
    pub fn passed(&self) -> bool {
        self.reflection_issues > 0
            && self.critical_reflection_issues > 0
            && self.revision_actions > 0
            && self.rollback_holds > 0
            && self.read_only
            && self.report_only
            && self.preview_only
            && !self.write_allowed
            && !self.applied
            && self.candidate_digest.starts_with("redaction-digest:")
    }

    pub fn summary_line(&self) -> String {
        format!(
            "failure_seeded_reflection_benchmark passed={} cases={} reflection_issues={} critical_reflection_issues={} revision_actions={} rollback_holds={} candidate_digest={} read_only={} report_only={} preview_only={} write_allowed={} applied={}",
            self.passed(),
            self.cases,
            self.reflection_issues,
            self.critical_reflection_issues,
            self.revision_actions,
            self.rollback_holds,
            self.candidate_digest,
            self.read_only,
            self.report_only,
            self.preview_only,
            self.write_allowed,
            self.applied
        )
    }
}

pub fn run_failure_seeded_reflection_benchmark() -> FailureSeededReflectionBenchmarkReport {
    let prompt = "Return a grounded Rust fallback plan for a failed model-pool primary call.";
    let draft = InferenceDraft::new(
        "",
        vec![ReasoningStep::new(
            "model_pool_primary_failure",
            "provider returned no usable candidate",
            0.05,
        )],
    );
    let report = Reflector::new().reflect(prompt, &draft);
    let critical = report.critical_issue_count();
    let revision_actions = report.revision_actions.len();
    let reflection_issues = report.issues.len();
    let reflection_issues_text = reflection_issues.to_string();
    let critical_text = critical.to_string();
    let revision_actions_text = revision_actions.to_string();

    FailureSeededReflectionBenchmarkReport {
        cases: 1,
        reflection_issues,
        critical_reflection_issues: critical,
        revision_actions,
        rollback_holds: usize::from(critical > 0 || revision_actions > 0),
        candidate_digest: stable_redaction_digest([
            "failure_seeded_reflection",
            &reflection_issues_text,
            &critical_text,
            &revision_actions_text,
        ]),
        read_only: true,
        report_only: true,
        preview_only: true,
        write_allowed: false,
        applied: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::privacy_redaction::contains_private_or_executable_marker;

    #[test]
    fn failure_seeded_reflection_benchmark_reports_revision_evidence_without_writes() {
        let report = run_failure_seeded_reflection_benchmark();

        assert!(report.passed(), "{}", report.summary_line());
        assert!(report.reflection_issues > 0);
        assert!(report.critical_reflection_issues > 0);
        assert!(report.revision_actions > 0);
        assert_eq!(report.rollback_holds, 1);
        assert!(report.read_only);
        assert!(report.preview_only);
        assert!(!report.write_allowed);
        assert!(!report.applied);
        assert!(!contains_private_or_executable_marker(
            &report.summary_line()
        ));
    }
}
