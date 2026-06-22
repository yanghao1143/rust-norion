#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImprovementEpisodeClass {
    Accepted,
    Failed,
    Flaky,
    PrivacyBlocked,
    ResearchOnly,
}

impl ImprovementEpisodeClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Failed => "failed",
            Self::Flaky => "flaky",
            Self::PrivacyBlocked => "privacy_blocked",
            Self::ResearchOnly => "research_only",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImprovementApprovalState {
    Pending,
    Approved,
    Rejected,
}

impl ImprovementApprovalState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImprovementValidationStatus {
    Pending,
    Passed,
    Failed,
    Flaky,
}

impl ImprovementValidationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Flaky => "flaky",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImprovementPrivacyState {
    Clean,
    Sanitized,
    Rejected,
}

impl ImprovementPrivacyState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Clean => "clean",
            Self::Sanitized => "sanitized",
            Self::Rejected => "rejected",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ImprovementEvidenceLane {
    pub items: u64,
    pub passed: u64,
    pub failed: u64,
    pub flaky: u64,
}

impl ImprovementEvidenceLane {
    pub fn new(items: u64, passed: u64, failed: u64, flaky: u64) -> Self {
        Self {
            items,
            passed,
            failed,
            flaky,
        }
    }

    pub fn passed_for_adaptation(self) -> bool {
        self.items > 0
            && self.passed > 0
            && self.failed == 0
            && self.flaky == 0
            && self
                .passed
                .saturating_add(self.failed)
                .saturating_add(self.flaky)
                <= self.items
    }

    fn add(&mut self, other: Self) {
        self.items = self.items.saturating_add(other.items);
        self.passed = self.passed.saturating_add(other.passed);
        self.failed = self.failed.saturating_add(other.failed);
        self.flaky = self.flaky.saturating_add(other.flaky);
    }
}

#[derive(Debug, Clone)]
pub struct ImprovementEpisodeInput {
    pub episode_id: String,
    pub task_label: String,
    pub patch_summary: String,
    pub prompt_payload: Option<String>,
    pub response_payload: Option<String>,
    pub source_trace_ids: Vec<String>,
    pub evidence_ids: Vec<String>,
    pub compiler: ImprovementEvidenceLane,
    pub tests: ImprovementEvidenceLane,
    pub benchmarks: ImprovementEvidenceLane,
    pub reflection_quality: f32,
    pub process_reward: f32,
    pub rollback_anchor_id: String,
    pub rollback_replayed: bool,
    pub approval_state: ImprovementApprovalState,
    pub validation_status: ImprovementValidationStatus,
    pub class: ImprovementEpisodeClass,
}

impl ImprovementEpisodeInput {
    pub fn new(episode_id: impl Into<String>, class: ImprovementEpisodeClass) -> Self {
        Self {
            episode_id: episode_id.into(),
            task_label: "unspecified".to_owned(),
            patch_summary: "no patch summary supplied".to_owned(),
            prompt_payload: None,
            response_payload: None,
            source_trace_ids: Vec::new(),
            evidence_ids: Vec::new(),
            compiler: ImprovementEvidenceLane::default(),
            tests: ImprovementEvidenceLane::default(),
            benchmarks: ImprovementEvidenceLane::default(),
            reflection_quality: 0.0,
            process_reward: 0.0,
            rollback_anchor_id: String::new(),
            rollback_replayed: false,
            approval_state: ImprovementApprovalState::Pending,
            validation_status: ImprovementValidationStatus::Pending,
            class,
        }
    }

    pub fn accepted(episode_id: impl Into<String>) -> Self {
        Self::new(episode_id, ImprovementEpisodeClass::Accepted)
            .with_approval_state(ImprovementApprovalState::Approved)
            .with_validation_status(ImprovementValidationStatus::Passed)
            .with_compiler(ImprovementEvidenceLane::new(1, 1, 0, 0))
            .with_tests(ImprovementEvidenceLane::new(1, 1, 0, 0))
            .with_benchmarks(ImprovementEvidenceLane::new(1, 1, 0, 0))
            .with_rollback_anchor("rollback:accepted")
            .with_rollback_replayed(true)
    }

    pub fn with_task_label(mut self, task_label: impl Into<String>) -> Self {
        self.task_label = task_label.into();
        self
    }

    pub fn with_patch_summary(mut self, patch_summary: impl Into<String>) -> Self {
        self.patch_summary = patch_summary.into();
        self
    }

    pub fn with_prompt_payload(mut self, payload: impl Into<String>) -> Self {
        self.prompt_payload = Some(payload.into());
        self
    }

    pub fn with_response_payload(mut self, payload: impl Into<String>) -> Self {
        self.response_payload = Some(payload.into());
        self
    }

    pub fn with_source_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        push_unique_string(&mut self.source_trace_ids, trace_id);
        self
    }

    pub fn with_evidence_id(mut self, evidence_id: impl Into<String>) -> Self {
        push_unique_string(&mut self.evidence_ids, evidence_id);
        self
    }

    pub fn with_compiler(mut self, lane: ImprovementEvidenceLane) -> Self {
        self.compiler = lane;
        self
    }

    pub fn with_tests(mut self, lane: ImprovementEvidenceLane) -> Self {
        self.tests = lane;
        self
    }

    pub fn with_benchmarks(mut self, lane: ImprovementEvidenceLane) -> Self {
        self.benchmarks = lane;
        self
    }

    pub fn with_reflection_quality(mut self, reflection_quality: f32) -> Self {
        self.reflection_quality = finite_or_zero(reflection_quality);
        self
    }

    pub fn with_process_reward(mut self, process_reward: f32) -> Self {
        self.process_reward = finite_or_zero(process_reward);
        self
    }

    pub fn with_rollback_anchor(mut self, rollback_anchor_id: impl Into<String>) -> Self {
        self.rollback_anchor_id = rollback_anchor_id.into();
        self
    }

    pub fn with_rollback_replayed(mut self, rollback_replayed: bool) -> Self {
        self.rollback_replayed = rollback_replayed;
        self
    }

    pub fn with_approval_state(mut self, approval_state: ImprovementApprovalState) -> Self {
        self.approval_state = approval_state;
        self
    }

    pub fn with_validation_status(
        mut self,
        validation_status: ImprovementValidationStatus,
    ) -> Self {
        self.validation_status = validation_status;
        self
    }
}

#[derive(Debug, Clone)]
pub struct ImprovementEpisodeRecord {
    pub episode_id: String,
    pub task_label: String,
    pub task_digest: String,
    pub patch_summary_preview: String,
    pub patch_summary_digest: String,
    pub prompt_payload_digest: Option<String>,
    pub response_payload_digest: Option<String>,
    pub source_trace_ids: Vec<String>,
    pub evidence_ids: Vec<String>,
    pub compiler: ImprovementEvidenceLane,
    pub tests: ImprovementEvidenceLane,
    pub benchmarks: ImprovementEvidenceLane,
    pub reflection_quality: f32,
    pub process_reward: f32,
    pub rollback_anchor_id: String,
    pub rollback_replayed: bool,
    pub approval_state: ImprovementApprovalState,
    pub validation_status: ImprovementValidationStatus,
    pub class: ImprovementEpisodeClass,
    pub privacy_state: ImprovementPrivacyState,
    pub privacy_redactions: usize,
    pub raw_prompt_payload_stored: bool,
    pub raw_response_payload_stored: bool,
    pub preview_only: bool,
    pub dataset_export_enabled: bool,
    pub active_adaptation_evidence: bool,
    pub blocked_reasons: Vec<String>,
}

impl ImprovementEpisodeRecord {
    pub fn from_input(input: ImprovementEpisodeInput) -> Self {
        let prompt_sensitive = input
            .prompt_payload
            .as_deref()
            .is_some_and(contains_sensitive_payload);
        let response_sensitive = input
            .response_payload
            .as_deref()
            .is_some_and(contains_sensitive_payload);
        let summary_sensitive = contains_sensitive_payload(&input.patch_summary);
        let task_sensitive = contains_sensitive_payload(&input.task_label);
        let privacy_redactions = usize::from(prompt_sensitive)
            .saturating_add(usize::from(response_sensitive))
            .saturating_add(usize::from(summary_sensitive))
            .saturating_add(usize::from(task_sensitive));
        let privacy_state = if prompt_sensitive || response_sensitive {
            ImprovementPrivacyState::Rejected
        } else if summary_sensitive || task_sensitive {
            ImprovementPrivacyState::Sanitized
        } else {
            ImprovementPrivacyState::Clean
        };
        let class = if privacy_state == ImprovementPrivacyState::Rejected {
            ImprovementEpisodeClass::PrivacyBlocked
        } else {
            input.class
        };

        let task_label = sanitize_public_text(&input.task_label, 80);
        let patch_summary_preview = sanitize_public_text(&input.patch_summary, 160);
        let prompt_payload_digest = input
            .prompt_payload
            .as_deref()
            .map(improvement_stable_digest);
        let response_payload_digest = input
            .response_payload
            .as_deref()
            .map(improvement_stable_digest);

        let mut record = Self {
            episode_id: sanitize_identifier(&input.episode_id, "episode"),
            task_digest: improvement_stable_digest(&input.task_label),
            task_label,
            patch_summary_digest: improvement_stable_digest(&input.patch_summary),
            patch_summary_preview,
            prompt_payload_digest,
            response_payload_digest,
            source_trace_ids: sanitize_id_list(input.source_trace_ids),
            evidence_ids: sanitize_id_list(input.evidence_ids),
            compiler: input.compiler,
            tests: input.tests,
            benchmarks: input.benchmarks,
            reflection_quality: finite_or_zero(input.reflection_quality),
            process_reward: finite_or_zero(input.process_reward),
            rollback_anchor_id: sanitize_identifier(&input.rollback_anchor_id, "rollback"),
            rollback_replayed: input.rollback_replayed,
            approval_state: input.approval_state,
            validation_status: input.validation_status,
            class,
            privacy_state,
            privacy_redactions,
            raw_prompt_payload_stored: false,
            raw_response_payload_stored: false,
            preview_only: true,
            dataset_export_enabled: false,
            active_adaptation_evidence: false,
            blocked_reasons: Vec::new(),
        };
        record.blocked_reasons = record.derive_blocked_reasons();
        record.active_adaptation_evidence = record.blocked_reasons.is_empty();
        record
    }

    pub fn summary(&self) -> String {
        format!(
            "{}:{} approval={} validation={} active={} compiler={}/{} tests={}/{} benchmarks={}/{} rollback_replayed={} privacy={} digest={}",
            self.episode_id,
            self.class.as_str(),
            self.approval_state.as_str(),
            self.validation_status.as_str(),
            self.active_adaptation_evidence,
            self.compiler.passed,
            self.compiler.items,
            self.tests.passed,
            self.tests.items,
            self.benchmarks.passed,
            self.benchmarks.items,
            self.rollback_replayed,
            self.privacy_state.as_str(),
            self.patch_summary_digest
        )
    }

    fn derive_blocked_reasons(&self) -> Vec<String> {
        let mut reasons = Vec::new();
        if self.episode_id.trim().is_empty() {
            reasons.push("improvement_corpus_episode_id_empty".to_owned());
        }
        if self.class != ImprovementEpisodeClass::Accepted {
            reasons.push(format!(
                "improvement_corpus_class_not_active={}",
                self.class.as_str()
            ));
        }
        if self.approval_state != ImprovementApprovalState::Approved {
            reasons.push(format!(
                "improvement_corpus_approval_not_approved={}",
                self.approval_state.as_str()
            ));
        }
        if self.validation_status != ImprovementValidationStatus::Passed {
            reasons.push(format!(
                "improvement_corpus_validation_not_passed={}",
                self.validation_status.as_str()
            ));
        }
        push_lane_blocked_reasons(&mut reasons, "compiler", self.compiler);
        push_lane_blocked_reasons(&mut reasons, "tests", self.tests);
        push_lane_blocked_reasons(&mut reasons, "benchmarks", self.benchmarks);
        if self.privacy_state == ImprovementPrivacyState::Rejected {
            reasons.push("improvement_corpus_privacy_rejected".to_owned());
        }
        if self.raw_prompt_payload_stored || self.raw_response_payload_stored {
            reasons.push("improvement_corpus_raw_payload_storage_forbidden".to_owned());
        }
        if self.rollback_anchor_id.trim().is_empty() {
            reasons.push("improvement_corpus_rollback_anchor_missing".to_owned());
        }
        if !self.rollback_replayed {
            reasons.push("improvement_corpus_rollback_replay_missing".to_owned());
        }
        if !self.preview_only {
            reasons.push("improvement_corpus_record_not_preview_only".to_owned());
        }
        if self.dataset_export_enabled {
            reasons.push("improvement_corpus_dataset_export_enabled".to_owned());
        }
        reasons
    }
}

#[derive(Debug, Clone)]
pub struct ImprovementCorpus {
    pub corpus_id: String,
    pub episodes: Vec<ImprovementEpisodeRecord>,
    pub read_only: bool,
    pub report_only: bool,
    pub preview_only: bool,
    pub dataset_export_enabled: bool,
}

impl ImprovementCorpus {
    pub fn new(corpus_id: impl Into<String>) -> Self {
        Self {
            corpus_id: sanitize_identifier(&corpus_id.into(), "corpus"),
            episodes: Vec::new(),
            read_only: true,
            report_only: true,
            preview_only: true,
            dataset_export_enabled: false,
        }
    }

    pub fn push_episode(&mut self, input: ImprovementEpisodeInput) -> &ImprovementEpisodeRecord {
        self.episodes
            .push(ImprovementEpisodeRecord::from_input(input));
        self.episodes
            .last()
            .expect("just pushed improvement episode")
    }

    pub fn report(&self) -> ImprovementCorpusReport {
        ImprovementCorpusReport::from_records(
            &self.corpus_id,
            self.read_only,
            self.report_only,
            self.preview_only,
            self.dataset_export_enabled,
            &self.episodes,
        )
    }
}

#[derive(Debug, Clone)]
pub struct ImprovementCorpusReport {
    pub corpus_id: String,
    pub read_only: bool,
    pub report_only: bool,
    pub preview_only: bool,
    pub dataset_export_enabled: bool,
    pub total_episodes: usize,
    pub accepted_episodes: usize,
    pub failed_episodes: usize,
    pub flaky_episodes: usize,
    pub privacy_blocked_episodes: usize,
    pub research_only_episodes: usize,
    pub active_adaptation_evidence: usize,
    pub blocked_adaptation_evidence: usize,
    pub approval_approved: usize,
    pub approval_pending: usize,
    pub approval_rejected: usize,
    pub validation_passed: usize,
    pub validation_pending: usize,
    pub validation_failed: usize,
    pub validation_flaky: usize,
    pub compiler: ImprovementEvidenceLane,
    pub tests: ImprovementEvidenceLane,
    pub benchmarks: ImprovementEvidenceLane,
    pub rollback_anchors: usize,
    pub rollback_replayed: usize,
    pub privacy_rejected: usize,
    pub privacy_redactions: usize,
    pub raw_prompt_payloads_stored: usize,
    pub raw_response_payloads_stored: usize,
    pub secret_leaks: usize,
    pub source_trace_ids: usize,
    pub evidence_ids: usize,
    pub record_summaries: Vec<String>,
    pub blocked_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl ImprovementCorpusReport {
    fn from_records(
        corpus_id: &str,
        read_only: bool,
        report_only: bool,
        preview_only: bool,
        dataset_export_enabled: bool,
        records: &[ImprovementEpisodeRecord],
    ) -> Self {
        let mut report = Self {
            corpus_id: corpus_id.to_owned(),
            read_only,
            report_only,
            preview_only,
            dataset_export_enabled,
            total_episodes: records.len(),
            accepted_episodes: 0,
            failed_episodes: 0,
            flaky_episodes: 0,
            privacy_blocked_episodes: 0,
            research_only_episodes: 0,
            active_adaptation_evidence: 0,
            blocked_adaptation_evidence: 0,
            approval_approved: 0,
            approval_pending: 0,
            approval_rejected: 0,
            validation_passed: 0,
            validation_pending: 0,
            validation_failed: 0,
            validation_flaky: 0,
            compiler: ImprovementEvidenceLane::default(),
            tests: ImprovementEvidenceLane::default(),
            benchmarks: ImprovementEvidenceLane::default(),
            rollback_anchors: 0,
            rollback_replayed: 0,
            privacy_rejected: 0,
            privacy_redactions: 0,
            raw_prompt_payloads_stored: 0,
            raw_response_payloads_stored: 0,
            secret_leaks: 0,
            source_trace_ids: 0,
            evidence_ids: 0,
            record_summaries: Vec::new(),
            blocked_reasons: Vec::new(),
            telemetry: Vec::new(),
        };

        for record in records {
            match record.class {
                ImprovementEpisodeClass::Accepted => report.accepted_episodes += 1,
                ImprovementEpisodeClass::Failed => report.failed_episodes += 1,
                ImprovementEpisodeClass::Flaky => report.flaky_episodes += 1,
                ImprovementEpisodeClass::PrivacyBlocked => report.privacy_blocked_episodes += 1,
                ImprovementEpisodeClass::ResearchOnly => report.research_only_episodes += 1,
            }
            match record.approval_state {
                ImprovementApprovalState::Approved => report.approval_approved += 1,
                ImprovementApprovalState::Pending => report.approval_pending += 1,
                ImprovementApprovalState::Rejected => report.approval_rejected += 1,
            }
            match record.validation_status {
                ImprovementValidationStatus::Passed => report.validation_passed += 1,
                ImprovementValidationStatus::Pending => report.validation_pending += 1,
                ImprovementValidationStatus::Failed => report.validation_failed += 1,
                ImprovementValidationStatus::Flaky => report.validation_flaky += 1,
            }
            report.active_adaptation_evidence += usize::from(record.active_adaptation_evidence);
            report.blocked_adaptation_evidence += usize::from(!record.active_adaptation_evidence);
            report.compiler.add(record.compiler);
            report.tests.add(record.tests);
            report.benchmarks.add(record.benchmarks);
            report.rollback_anchors += usize::from(!record.rollback_anchor_id.trim().is_empty());
            report.rollback_replayed += usize::from(record.rollback_replayed);
            report.privacy_rejected +=
                usize::from(record.privacy_state == ImprovementPrivacyState::Rejected);
            report.privacy_redactions = report
                .privacy_redactions
                .saturating_add(record.privacy_redactions);
            report.raw_prompt_payloads_stored += usize::from(record.raw_prompt_payload_stored);
            report.raw_response_payloads_stored += usize::from(record.raw_response_payload_stored);
            report.secret_leaks += usize::from(record_has_secret_leak(record));
            report.source_trace_ids = report
                .source_trace_ids
                .saturating_add(record.source_trace_ids.len());
            report.evidence_ids = report
                .evidence_ids
                .saturating_add(record.evidence_ids.len());
            report
                .blocked_reasons
                .extend(record.blocked_reasons.iter().cloned());
            report.record_summaries.push(record.summary());
        }

        if !read_only {
            report
                .blocked_reasons
                .push("improvement_corpus_not_read_only".to_owned());
        }
        if !report_only {
            report
                .blocked_reasons
                .push("improvement_corpus_not_report_only".to_owned());
        }
        if !preview_only {
            report
                .blocked_reasons
                .push("improvement_corpus_not_preview_only".to_owned());
        }
        if dataset_export_enabled {
            report
                .blocked_reasons
                .push("improvement_corpus_dataset_export_enabled".to_owned());
        }
        if report.raw_prompt_payloads_stored > 0 || report.raw_response_payloads_stored > 0 {
            report
                .blocked_reasons
                .push("improvement_corpus_raw_payload_storage_forbidden".to_owned());
        }
        if report.secret_leaks > 0 {
            report
                .blocked_reasons
                .push("improvement_corpus_secret_leak_detected".to_owned());
        }
        report.blocked_reasons.sort();
        report.blocked_reasons.dedup();
        report.telemetry = report.telemetry();
        report
    }

    pub fn summary_line(&self) -> String {
        format!(
            "improvement_corpus corpus={} read_only={} report_only={} preview_only={} dataset_export_enabled={} episodes={} accepted={} failed={} flaky={} privacy_blocked={} research_only={} active_adaptation={} blocked_adaptation={} approved={} validation_passed={} compiler_passed={} test_passed={} benchmark_passed={} rollback_replayed={} privacy_rejected={} redactions={} secret_leaks={} evidence_ids={} blocked_reasons={}",
            self.corpus_id,
            self.read_only,
            self.report_only,
            self.preview_only,
            self.dataset_export_enabled,
            self.total_episodes,
            self.accepted_episodes,
            self.failed_episodes,
            self.flaky_episodes,
            self.privacy_blocked_episodes,
            self.research_only_episodes,
            self.active_adaptation_evidence,
            self.blocked_adaptation_evidence,
            self.approval_approved,
            self.validation_passed,
            self.compiler.passed,
            self.tests.passed,
            self.benchmarks.passed,
            self.rollback_replayed,
            self.privacy_rejected,
            self.privacy_redactions,
            self.secret_leaks,
            self.evidence_ids,
            self.blocked_reasons.len()
        )
    }

    pub fn json_line(&self) -> String {
        format!(
            "{{\
             \"schema\":\"rust-norion-improvement-corpus-v1\",\
             \"corpus_id\":\"{}\",\
             \"read_only\":{},\
             \"report_only\":{},\
             \"preview_only\":{},\
             \"dataset_export_enabled\":{},\
             \"records\":{{\"total\":{},\"accepted\":{},\"failed\":{},\"flaky\":{},\"privacy_blocked\":{},\"research_only\":{}}},\
             \"active_adaptation\":{{\"eligible\":{},\"blocked\":{}}},\
             \"approval\":{{\"approved\":{},\"pending\":{},\"rejected\":{}}},\
             \"validation\":{{\"passed\":{},\"pending\":{},\"failed\":{},\"flaky\":{}}},\
             \"evidence\":{{\"compiler_items\":{},\"compiler_passed\":{},\"compiler_failed\":{},\"compiler_flaky\":{},\"test_items\":{},\"test_passed\":{},\"test_failed\":{},\"test_flaky\":{},\"benchmark_items\":{},\"benchmark_passed\":{},\"benchmark_failed\":{},\"benchmark_flaky\":{},\"source_trace_ids\":{},\"evidence_ids\":{}}},\
             \"rollback\":{{\"anchors\":{},\"replayed\":{}}},\
             \"privacy\":{{\"rejected\":{},\"redactions\":{},\"raw_prompt_payloads_stored\":{},\"raw_response_payloads_stored\":{},\"secret_leaks\":{}}},\
             \"record_summaries\":{},\
             \"blocked_reasons\":{},\
             \"telemetry\":{}\
             }}",
            json_escape(&self.corpus_id),
            self.read_only,
            self.report_only,
            self.preview_only,
            self.dataset_export_enabled,
            self.total_episodes,
            self.accepted_episodes,
            self.failed_episodes,
            self.flaky_episodes,
            self.privacy_blocked_episodes,
            self.research_only_episodes,
            self.active_adaptation_evidence,
            self.blocked_adaptation_evidence,
            self.approval_approved,
            self.approval_pending,
            self.approval_rejected,
            self.validation_passed,
            self.validation_pending,
            self.validation_failed,
            self.validation_flaky,
            self.compiler.items,
            self.compiler.passed,
            self.compiler.failed,
            self.compiler.flaky,
            self.tests.items,
            self.tests.passed,
            self.tests.failed,
            self.tests.flaky,
            self.benchmarks.items,
            self.benchmarks.passed,
            self.benchmarks.failed,
            self.benchmarks.flaky,
            self.source_trace_ids,
            self.evidence_ids,
            self.rollback_anchors,
            self.rollback_replayed,
            self.privacy_rejected,
            self.privacy_redactions,
            self.raw_prompt_payloads_stored,
            self.raw_response_payloads_stored,
            self.secret_leaks,
            string_array_json(&self.record_summaries),
            string_array_json(&self.blocked_reasons),
            string_array_json(&self.telemetry),
        )
    }

    fn telemetry(&self) -> Vec<String> {
        vec![
            "improvement_corpus=true".to_owned(),
            format!("improvement_corpus_id={}", self.corpus_id),
            format!("improvement_corpus_episodes={}", self.total_episodes),
            format!(
                "improvement_corpus_active_adaptation={}",
                self.active_adaptation_evidence
            ),
            format!(
                "improvement_corpus_compiler_passed={}",
                self.compiler.passed
            ),
            format!("improvement_corpus_test_passed={}", self.tests.passed),
            format!(
                "improvement_corpus_benchmark_passed={}",
                self.benchmarks.passed
            ),
            format!(
                "improvement_corpus_privacy_rejected={}",
                self.privacy_rejected
            ),
            format!("improvement_corpus_secret_leaks={}", self.secret_leaks),
            format!(
                "improvement_corpus_dataset_export_enabled={}",
                self.dataset_export_enabled
            ),
        ]
    }
}

fn push_lane_blocked_reasons(
    reasons: &mut Vec<String>,
    lane_name: &str,
    lane: ImprovementEvidenceLane,
) {
    if !lane.passed_for_adaptation() {
        reasons.push(format!(
            "improvement_corpus_{lane_name}_validation_not_clean={}/{} failed={} flaky={}",
            lane.passed, lane.items, lane.failed, lane.flaky
        ));
    }
}

fn finite_or_zero(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

fn sanitize_identifier(value: &str, fallback: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.') {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>();
    let sanitized = sanitized.trim_matches('-').to_owned();
    if sanitized.is_empty() {
        fallback.to_owned()
    } else {
        sanitized.chars().take(96).collect()
    }
}

fn sanitize_id_list(values: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    for value in values {
        push_unique_string(&mut out, sanitize_identifier(&value, "id"));
    }
    out
}

fn sanitize_public_text(value: &str, max_chars: usize) -> String {
    let mut out = Vec::new();
    for word in value.split_whitespace() {
        if contains_sensitive_payload(word) {
            out.push("[redacted]");
        } else {
            out.push(word);
        }
    }
    let sanitized = out.join(" ");
    let mut preview = sanitized.chars().take(max_chars).collect::<String>();
    if sanitized.chars().count() > max_chars {
        preview.push_str("...");
    }
    preview
}

fn contains_sensitive_payload(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "api_key",
        "apikey",
        "secret",
        "password",
        "passwd",
        "token=",
        "private:",
        "private_key",
        "begin private key",
        "sk-",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn record_has_secret_leak(record: &ImprovementEpisodeRecord) -> bool {
    [
        record.episode_id.as_str(),
        record.task_label.as_str(),
        record.patch_summary_preview.as_str(),
        record.rollback_anchor_id.as_str(),
    ]
    .iter()
    .any(|value| contains_sensitive_payload(value))
}

fn push_unique_string(values: &mut Vec<String>, value: impl Into<String>) {
    let value = value.into();
    if !value.trim().is_empty() && !values.contains(&value) {
        values.push(value);
    }
}

fn improvement_stable_digest(value: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("fnv64:{hash:016x}")
}

fn string_array_json(values: &[String]) -> String {
    let values = values
        .iter()
        .map(|value| format!("\"{}\"", json_escape(value)))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{values}]")
}

fn json_escape(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn accepted_episode() -> ImprovementEpisodeInput {
        ImprovementEpisodeInput::accepted("fix-accepted")
            .with_task_label("rust coding compiler fix")
            .with_patch_summary("replace unwrap with propagated error and add regression test")
            .with_prompt_payload("compiler error E0308 in local fixture")
            .with_response_payload("use Result return type and assert cargo check passes")
            .with_source_trace_id("trace:rust-check:fix-accepted")
            .with_evidence_id("compiler:passed")
            .with_evidence_id("tests:passed")
            .with_evidence_id("benchmark:won")
            .with_reflection_quality(0.91)
            .with_process_reward(0.88)
    }

    #[test]
    fn accepted_episode_becomes_active_only_after_approval_validation_and_replay() {
        let record = ImprovementEpisodeRecord::from_input(accepted_episode());

        assert_eq!(record.class, ImprovementEpisodeClass::Accepted);
        assert!(record.active_adaptation_evidence);
        assert!(
            record.blocked_reasons.is_empty(),
            "{:?}",
            record.blocked_reasons
        );
        assert_eq!(record.approval_state, ImprovementApprovalState::Approved);
        assert_eq!(
            record.validation_status,
            ImprovementValidationStatus::Passed
        );
        assert!(record.rollback_replayed);
        assert!(!record.raw_prompt_payload_stored);
        assert!(!record.raw_response_payload_stored);
    }

    #[test]
    fn failed_flaky_and_privacy_rejected_episodes_remain_inactive() {
        let failed = ImprovementEpisodeRecord::from_input(
            ImprovementEpisodeInput::new("fix-failed", ImprovementEpisodeClass::Failed)
                .with_approval_state(ImprovementApprovalState::Approved)
                .with_validation_status(ImprovementValidationStatus::Failed)
                .with_compiler(ImprovementEvidenceLane::new(1, 0, 1, 0))
                .with_tests(ImprovementEvidenceLane::new(1, 0, 1, 0))
                .with_benchmarks(ImprovementEvidenceLane::new(1, 0, 1, 0))
                .with_rollback_anchor("rollback:failed")
                .with_rollback_replayed(true),
        );
        let flaky = ImprovementEpisodeRecord::from_input(
            ImprovementEpisodeInput::new("fix-flaky", ImprovementEpisodeClass::Flaky)
                .with_approval_state(ImprovementApprovalState::Approved)
                .with_validation_status(ImprovementValidationStatus::Flaky)
                .with_compiler(ImprovementEvidenceLane::new(1, 1, 0, 0))
                .with_tests(ImprovementEvidenceLane::new(2, 1, 0, 1))
                .with_benchmarks(ImprovementEvidenceLane::new(1, 1, 0, 0))
                .with_rollback_anchor("rollback:flaky")
                .with_rollback_replayed(true),
        );
        let privacy = ImprovementEpisodeRecord::from_input(
            ImprovementEpisodeInput::accepted("fix-privacy")
                .with_prompt_payload("private: user password=correct-horse-battery-staple"),
        );

        assert!(!failed.active_adaptation_evidence);
        assert!(
            failed
                .blocked_reasons
                .iter()
                .any(|reason| reason.contains("class_not_active=failed"))
        );
        assert!(!flaky.active_adaptation_evidence);
        assert!(
            flaky
                .blocked_reasons
                .iter()
                .any(|reason| reason.contains("validation_not_passed=flaky"))
        );
        assert_eq!(privacy.class, ImprovementEpisodeClass::PrivacyBlocked);
        assert!(!privacy.active_adaptation_evidence);
        assert!(
            privacy
                .blocked_reasons
                .contains(&"improvement_corpus_privacy_rejected".to_owned())
        );
    }

    #[test]
    fn rollback_replay_is_required_for_active_adaptation() {
        let missing_replay = ImprovementEpisodeRecord::from_input(
            accepted_episode()
                .with_rollback_anchor("rollback:present")
                .with_rollback_replayed(false),
        );

        assert!(!missing_replay.active_adaptation_evidence);
        assert!(
            missing_replay
                .blocked_reasons
                .contains(&"improvement_corpus_rollback_replay_missing".to_owned())
        );
    }

    #[test]
    fn corpus_report_sanitizes_raw_payloads_and_counts_validation_evidence() {
        let mut corpus = ImprovementCorpus::new("self-training-preview");
        corpus.push_episode(accepted_episode());
        corpus.push_episode(
            ImprovementEpisodeInput::new("research", ImprovementEpisodeClass::ResearchOnly)
                .with_patch_summary("research-only trace, not active")
                .with_compiler(ImprovementEvidenceLane::new(1, 1, 0, 0))
                .with_tests(ImprovementEvidenceLane::new(1, 1, 0, 0))
                .with_benchmarks(ImprovementEvidenceLane::new(1, 1, 0, 0))
                .with_validation_status(ImprovementValidationStatus::Passed)
                .with_approval_state(ImprovementApprovalState::Pending)
                .with_rollback_anchor("rollback:research")
                .with_rollback_replayed(true),
        );
        corpus.push_episode(
            ImprovementEpisodeInput::accepted("privacy-case")
                .with_patch_summary("remove leaked api_key from fixture")
                .with_prompt_payload("SECRET=do-not-store private: raw user request"),
        );
        let report = corpus.report();
        let json = report.json_line();

        assert_eq!(report.total_episodes, 3);
        assert_eq!(report.active_adaptation_evidence, 1);
        assert_eq!(report.research_only_episodes, 1);
        assert_eq!(report.privacy_blocked_episodes, 1);
        assert_eq!(report.privacy_rejected, 1);
        assert_eq!(report.compiler.passed, 3);
        assert_eq!(report.tests.passed, 3);
        assert_eq!(report.benchmarks.passed, 3);
        assert_eq!(report.raw_prompt_payloads_stored, 0);
        assert_eq!(report.raw_response_payloads_stored, 0);
        assert_eq!(report.secret_leaks, 0);
        assert!(!json.contains("do-not-store"));
        assert!(!json.contains("raw user request"));
        assert!(!json.contains("SECRET="));
        assert!(!json.contains("api_key"));
        assert!(json.contains("\"schema\":\"rust-norion-improvement-corpus-v1\""));
        assert!(report.summary_line().contains("active_adaptation=1"));
    }

    #[test]
    fn approval_and_validation_are_both_required_for_active_adaptation() {
        let pending = ImprovementEpisodeRecord::from_input(
            accepted_episode().with_approval_state(ImprovementApprovalState::Pending),
        );
        let unvalidated = ImprovementEpisodeRecord::from_input(
            accepted_episode().with_validation_status(ImprovementValidationStatus::Pending),
        );

        assert!(!pending.active_adaptation_evidence);
        assert!(!unvalidated.active_adaptation_evidence);
        assert!(
            pending
                .blocked_reasons
                .iter()
                .any(|reason| reason.contains("approval_not_approved=pending"))
        );
        assert!(
            unvalidated
                .blocked_reasons
                .iter()
                .any(|reason| reason.contains("validation_not_passed=pending"))
        );
    }
}
