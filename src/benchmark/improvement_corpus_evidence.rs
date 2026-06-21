use crate::improvement_corpus::ImprovementCorpusReport;

#[derive(Debug, Clone, Default)]
pub struct BenchmarkImprovementCorpusEvidence {
    pub reports: usize,
    pub episodes: usize,
    pub accepted: usize,
    pub failed: usize,
    pub flaky: usize,
    pub privacy_blocked: usize,
    pub research_only: usize,
    pub active_adaptation: usize,
    pub blocked_adaptation: usize,
    pub compiler_items: u64,
    pub compiler_passed: u64,
    pub compiler_failed: u64,
    pub test_items: u64,
    pub test_passed: u64,
    pub test_failed: u64,
    pub benchmark_items: u64,
    pub benchmark_passed: u64,
    pub benchmark_failed: u64,
    pub rollback_replayed: usize,
    pub approval_approved: usize,
    pub validation_passed: usize,
    pub privacy_rejected: usize,
    pub privacy_redactions: usize,
    pub raw_prompt_payloads_stored: usize,
    pub raw_response_payloads_stored: usize,
    pub secret_leaks: usize,
    pub dataset_export_enabled: usize,
    pub evidence_ids: usize,
    pub failures: Vec<String>,
}

impl BenchmarkImprovementCorpusEvidence {
    pub fn record_report(&mut self, report: &ImprovementCorpusReport) {
        self.reports = self.reports.saturating_add(1);
        self.episodes = self.episodes.saturating_add(report.total_episodes);
        self.accepted = self.accepted.saturating_add(report.accepted_episodes);
        self.failed = self.failed.saturating_add(report.failed_episodes);
        self.flaky = self.flaky.saturating_add(report.flaky_episodes);
        self.privacy_blocked = self
            .privacy_blocked
            .saturating_add(report.privacy_blocked_episodes);
        self.research_only = self
            .research_only
            .saturating_add(report.research_only_episodes);
        self.active_adaptation = self
            .active_adaptation
            .saturating_add(report.active_adaptation_evidence);
        self.blocked_adaptation = self
            .blocked_adaptation
            .saturating_add(report.blocked_adaptation_evidence);
        self.compiler_items = self.compiler_items.saturating_add(report.compiler.items);
        self.compiler_passed = self.compiler_passed.saturating_add(report.compiler.passed);
        self.compiler_failed = self.compiler_failed.saturating_add(report.compiler.failed);
        self.test_items = self.test_items.saturating_add(report.tests.items);
        self.test_passed = self.test_passed.saturating_add(report.tests.passed);
        self.test_failed = self.test_failed.saturating_add(report.tests.failed);
        self.benchmark_items = self.benchmark_items.saturating_add(report.benchmarks.items);
        self.benchmark_passed = self
            .benchmark_passed
            .saturating_add(report.benchmarks.passed);
        self.benchmark_failed = self
            .benchmark_failed
            .saturating_add(report.benchmarks.failed);
        self.rollback_replayed = self
            .rollback_replayed
            .saturating_add(report.rollback_replayed);
        self.approval_approved = self
            .approval_approved
            .saturating_add(report.approval_approved);
        self.validation_passed = self
            .validation_passed
            .saturating_add(report.validation_passed);
        self.privacy_rejected = self
            .privacy_rejected
            .saturating_add(report.privacy_rejected);
        self.privacy_redactions = self
            .privacy_redactions
            .saturating_add(report.privacy_redactions);
        self.raw_prompt_payloads_stored = self
            .raw_prompt_payloads_stored
            .saturating_add(report.raw_prompt_payloads_stored);
        self.raw_response_payloads_stored = self
            .raw_response_payloads_stored
            .saturating_add(report.raw_response_payloads_stored);
        self.secret_leaks = self.secret_leaks.saturating_add(report.secret_leaks);
        self.dataset_export_enabled = self
            .dataset_export_enabled
            .saturating_add(usize::from(report.dataset_export_enabled));
        self.evidence_ids = self.evidence_ids.saturating_add(report.evidence_ids);
        self.failures.extend(
            report
                .blocked_reasons
                .iter()
                .map(|reason| format!("improvement_corpus_report:{}:{reason}", report.corpus_id)),
        );
    }
}
