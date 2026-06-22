use std::collections::BTreeSet;
use std::io;

use crate::improvement_corpus::{
    ImprovementApprovalState, ImprovementCorpus, ImprovementCorpusReport, ImprovementEpisodeClass,
    ImprovementEpisodeInput, ImprovementEvidenceLane, ImprovementValidationStatus,
};
use crate::no_weight_retrain::{
    NoWeightImprovementCandidate, NoWeightImprovementLane, NoWeightRetrainGate,
    NoWeightRetrainScorecard,
};
use crate::self_evolution::{SelfEvolutionValidationEvidence, SelfEvolutionValidationLane};

use super::{RustSnippetCheck, RustSnippetCheckReport, RustSnippetValidator};

const MAX_DIAGNOSTIC_PREVIEW_CHARS: usize = 240;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RustCodingRepairCommandKind {
    Formatting,
    Compiler,
    Lint,
    Tests,
    Benchmarks,
}

impl RustCodingRepairCommandKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Formatting => "formatting",
            Self::Compiler => "compiler",
            Self::Lint => "lint",
            Self::Tests => "tests",
            Self::Benchmarks => "benchmarks",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RustCodingRepairOutcome {
    Passed,
    Failed,
    TimedOut,
    Skipped,
}

impl RustCodingRepairOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::TimedOut => "timed_out",
            Self::Skipped => "skipped",
        }
    }

    pub fn passed(self) -> bool {
        self == Self::Passed
    }

    pub fn failed(self) -> bool {
        matches!(self, Self::Failed | Self::TimedOut)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustCodingCommandEvidence {
    pub kind: RustCodingRepairCommandKind,
    pub outcome: RustCodingRepairOutcome,
    pub status_code: Option<i32>,
    pub duration_ms: u64,
    pub diagnostic_digest: String,
    pub diagnostic_preview: String,
    pub evidence_id: String,
}

impl RustCodingCommandEvidence {
    pub fn passed(kind: RustCodingRepairCommandKind) -> Self {
        Self::new(kind, RustCodingRepairOutcome::Passed)
    }

    pub fn failed(kind: RustCodingRepairCommandKind, diagnostic: impl AsRef<str>) -> Self {
        Self::new(kind, RustCodingRepairOutcome::Failed).with_diagnostic(diagnostic)
    }

    pub fn timed_out(kind: RustCodingRepairCommandKind, diagnostic: impl AsRef<str>) -> Self {
        Self::new(kind, RustCodingRepairOutcome::TimedOut).with_diagnostic(diagnostic)
    }

    pub fn skipped(kind: RustCodingRepairCommandKind, diagnostic: impl AsRef<str>) -> Self {
        Self::new(kind, RustCodingRepairOutcome::Skipped).with_diagnostic(diagnostic)
    }

    pub fn from_snippet_report(report: &RustSnippetCheckReport) -> Self {
        let diagnostic = format!("{}\n{}", report.stdout, report.stderr);
        let outcome = if report.passed {
            RustCodingRepairOutcome::Passed
        } else {
            RustCodingRepairOutcome::Failed
        };
        Self::new(RustCodingRepairCommandKind::Compiler, outcome)
            .with_status_code(report.status_code)
            .with_diagnostic(diagnostic)
    }

    pub fn with_status_code(mut self, status_code: Option<i32>) -> Self {
        self.status_code = status_code;
        self.refresh_evidence_id();
        self
    }

    pub fn with_duration_ms(mut self, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms;
        self.refresh_evidence_id();
        self
    }

    pub fn with_diagnostic(mut self, diagnostic: impl AsRef<str>) -> Self {
        let diagnostic = diagnostic.as_ref();
        self.diagnostic_digest = stable_digest(diagnostic);
        self.diagnostic_preview = sanitize_public_text(diagnostic, MAX_DIAGNOSTIC_PREVIEW_CHARS);
        self.refresh_evidence_id();
        self
    }

    fn new(kind: RustCodingRepairCommandKind, outcome: RustCodingRepairOutcome) -> Self {
        let mut evidence = Self {
            kind,
            outcome,
            status_code: None,
            duration_ms: 0,
            diagnostic_digest: stable_digest(""),
            diagnostic_preview: String::new(),
            evidence_id: String::new(),
        };
        evidence.refresh_evidence_id();
        evidence
    }

    fn refresh_evidence_id(&mut self) {
        self.evidence_id = format!(
            "rust-repair-{}:{}",
            self.kind.as_str(),
            stable_digest(&format!(
                "{}:{}:{:?}:{}:{}",
                self.kind.as_str(),
                self.outcome.as_str(),
                self.status_code,
                self.duration_ms,
                self.diagnostic_digest
            ))
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RustCodingRepairFailureClass {
    Formatting,
    Compiler,
    Lint,
    Tests,
    Benchmarks,
    Timeout,
    MissingEvidence,
    PrivacyBlocked,
}

impl RustCodingRepairFailureClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Formatting => "formatting",
            Self::Compiler => "compiler",
            Self::Lint => "lint",
            Self::Tests => "tests",
            Self::Benchmarks => "benchmarks",
            Self::Timeout => "timeout",
            Self::MissingEvidence => "missing_evidence",
            Self::PrivacyBlocked => "privacy_blocked",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RustCodingRepairDecision {
    ValidatedCandidate,
    HeldForEvidence,
    RetainedForLearning,
    PrivacyBlocked,
}

impl RustCodingRepairDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ValidatedCandidate => "validated_candidate",
            Self::HeldForEvidence => "held_for_evidence",
            Self::RetainedForLearning => "retained_for_learning",
            Self::PrivacyBlocked => "privacy_blocked",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RustCodingRepairPolicy {
    pub require_formatting: bool,
    pub require_compiler: bool,
    pub require_lint: bool,
    pub require_tests: bool,
    pub require_benchmarks: bool,
    pub require_rollback_anchor: bool,
    pub require_rollback_replay: bool,
}

impl Default for RustCodingRepairPolicy {
    fn default() -> Self {
        Self {
            require_formatting: true,
            require_compiler: true,
            require_lint: true,
            require_tests: true,
            require_benchmarks: true,
            require_rollback_anchor: true,
            require_rollback_replay: true,
        }
    }
}

impl RustCodingRepairPolicy {
    fn required_kinds(self) -> Vec<RustCodingRepairCommandKind> {
        let mut kinds = Vec::new();
        if self.require_formatting {
            kinds.push(RustCodingRepairCommandKind::Formatting);
        }
        if self.require_compiler {
            kinds.push(RustCodingRepairCommandKind::Compiler);
        }
        if self.require_lint {
            kinds.push(RustCodingRepairCommandKind::Lint);
        }
        if self.require_tests {
            kinds.push(RustCodingRepairCommandKind::Tests);
        }
        if self.require_benchmarks {
            kinds.push(RustCodingRepairCommandKind::Benchmarks);
        }
        kinds
    }
}

#[derive(Debug, Clone)]
pub struct RustCodingRepairInput {
    pub repair_id: String,
    pub task_label: String,
    pub patch_summary: String,
    pub prompt_payload: Option<String>,
    pub response_payload: Option<String>,
    pub command_evidence: Vec<RustCodingCommandEvidence>,
    pub rollback_anchor_id: String,
    pub rollback_replayed: bool,
    pub operator_approved: bool,
    pub benchmark_delta: f32,
    pub regression_budget: f32,
}

impl RustCodingRepairInput {
    pub fn new(repair_id: impl Into<String>) -> Self {
        Self {
            repair_id: repair_id.into(),
            task_label: "rust-coding-repair".to_owned(),
            patch_summary: "compiler-guided Rust repair candidate".to_owned(),
            prompt_payload: None,
            response_payload: None,
            command_evidence: Vec::new(),
            rollback_anchor_id: String::new(),
            rollback_replayed: false,
            operator_approved: false,
            benchmark_delta: 0.0,
            regression_budget: 0.0,
        }
    }

    pub fn with_task_label(mut self, task_label: impl Into<String>) -> Self {
        self.task_label = task_label.into();
        self
    }

    pub fn with_patch_summary(mut self, patch_summary: impl Into<String>) -> Self {
        self.patch_summary = patch_summary.into();
        self
    }

    pub fn with_prompt_payload(mut self, prompt_payload: impl Into<String>) -> Self {
        self.prompt_payload = Some(prompt_payload.into());
        self
    }

    pub fn with_response_payload(mut self, response_payload: impl Into<String>) -> Self {
        self.response_payload = Some(response_payload.into());
        self
    }

    pub fn with_command(mut self, evidence: RustCodingCommandEvidence) -> Self {
        self.command_evidence.push(evidence);
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

    pub fn with_operator_approval(mut self, operator_approved: bool) -> Self {
        self.operator_approved = operator_approved;
        self
    }

    pub fn with_benchmark_delta(mut self, benchmark_delta: f32) -> Self {
        self.benchmark_delta = finite_or_zero(benchmark_delta);
        self
    }

    pub fn with_regression_budget(mut self, regression_budget: f32) -> Self {
        self.regression_budget = finite_or_zero(regression_budget).max(0.0);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RustCodingRepairCandidateSummary {
    pub candidate_id: String,
    pub lane: NoWeightImprovementLane,
    pub gate_decision: String,
    pub evidence_digest: String,
    pub write_allowed: bool,
    pub summary: String,
}

#[derive(Debug, Clone)]
pub struct RustCodingRepairReport {
    pub repair_id: String,
    pub decision: RustCodingRepairDecision,
    pub task_label: String,
    pub task_digest: String,
    pub patch_summary_preview: String,
    pub patch_summary_digest: String,
    pub prompt_payload_digest: Option<String>,
    pub response_payload_digest: Option<String>,
    pub command_evidence: Vec<RustCodingCommandEvidence>,
    pub evidence_ids: Vec<String>,
    pub evidence_digest: String,
    pub failure_classes: Vec<RustCodingRepairFailureClass>,
    pub blocked_reasons: Vec<String>,
    pub improvement_corpus_report: ImprovementCorpusReport,
    pub candidate_summaries: Vec<RustCodingRepairCandidateSummary>,
    pub no_weight_scorecards: Vec<NoWeightRetrainScorecard>,
    pub read_only: bool,
    pub report_only: bool,
    pub preview_only: bool,
    pub auto_commit_allowed: bool,
    pub auto_push_allowed: bool,
    pub durable_promotion_allowed: bool,
}

impl RustCodingRepairReport {
    pub fn passed_commands(&self) -> usize {
        self.command_evidence
            .iter()
            .filter(|evidence| evidence.outcome == RustCodingRepairOutcome::Passed)
            .count()
    }

    pub fn failed_commands(&self) -> usize {
        self.command_evidence
            .iter()
            .filter(|evidence| evidence.outcome == RustCodingRepairOutcome::Failed)
            .count()
    }

    pub fn timed_out_commands(&self) -> usize {
        self.command_evidence
            .iter()
            .filter(|evidence| evidence.outcome == RustCodingRepairOutcome::TimedOut)
            .count()
    }

    pub fn summary_line(&self) -> String {
        format!(
            "rust_coding_repair repair={} decision={} commands={} passed={} failed={} timed_out={} candidates={} active_corpus={} read_only={} report_only={} preview_only={} auto_commit_allowed={} auto_push_allowed={} durable_promotion_allowed={} evidence_digest={} blocked_reasons={}",
            self.repair_id,
            self.decision.as_str(),
            self.command_evidence.len(),
            self.passed_commands(),
            self.failed_commands(),
            self.timed_out_commands(),
            self.candidate_summaries.len(),
            self.improvement_corpus_report.active_adaptation_evidence,
            self.read_only,
            self.report_only,
            self.preview_only,
            self.auto_commit_allowed,
            self.auto_push_allowed,
            self.durable_promotion_allowed,
            self.evidence_digest,
            self.blocked_reasons.len()
        )
    }

    pub fn json_line(&self) -> String {
        format!(
            "{{\
             \"schema\":\"rust-norion-rust-coding-repair-v1\",\
             \"repair_id\":\"{}\",\
             \"decision\":\"{}\",\
             \"task_digest\":\"{}\",\
             \"patch_summary_digest\":\"{}\",\
             \"prompt_payload_digest\":{},\
             \"response_payload_digest\":{},\
             \"commands\":{{\"total\":{},\"passed\":{},\"failed\":{},\"timed_out\":{}}},\
             \"evidence_ids\":{},\
             \"evidence_digest\":\"{}\",\
             \"failure_classes\":{},\
             \"blocked_reasons\":{},\
             \"candidate_summaries\":{},\
             \"read_only\":{},\
             \"report_only\":{},\
             \"preview_only\":{},\
             \"auto_commit_allowed\":{},\
             \"auto_push_allowed\":{},\
             \"durable_promotion_allowed\":{}\
             }}",
            json_escape(&self.repair_id),
            self.decision.as_str(),
            json_escape(&self.task_digest),
            json_escape(&self.patch_summary_digest),
            optional_json_string(self.prompt_payload_digest.as_deref()),
            optional_json_string(self.response_payload_digest.as_deref()),
            self.command_evidence.len(),
            self.passed_commands(),
            self.failed_commands(),
            self.timed_out_commands(),
            string_array_json(&self.evidence_ids),
            json_escape(&self.evidence_digest),
            string_array_json(
                &self
                    .failure_classes
                    .iter()
                    .map(|class| class.as_str().to_owned())
                    .collect::<Vec<_>>()
            ),
            string_array_json(&self.blocked_reasons),
            string_array_json(
                &self
                    .candidate_summaries
                    .iter()
                    .map(|candidate| candidate.summary.clone())
                    .collect::<Vec<_>>()
            ),
            self.read_only,
            self.report_only,
            self.preview_only,
            self.auto_commit_allowed,
            self.auto_push_allowed,
            self.durable_promotion_allowed,
        )
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RustCodingRepairHarness {
    pub policy: RustCodingRepairPolicy,
}

impl RustCodingRepairHarness {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_policy(mut self, policy: RustCodingRepairPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn run_snippet_fixture(
        &self,
        validator: &RustSnippetValidator,
        check: &RustSnippetCheck,
        input: RustCodingRepairInput,
    ) -> io::Result<RustCodingRepairReport> {
        let report = validator.check(check)?;
        Ok(self
            .evaluate(input.with_command(RustCodingCommandEvidence::from_snippet_report(&report))))
    }

    pub fn evaluate(&self, input: RustCodingRepairInput) -> RustCodingRepairReport {
        let repair_id = sanitize_identifier(&input.repair_id, "rust-repair");
        let task_label = sanitize_public_text(&input.task_label, 96);
        let patch_summary_preview = sanitize_public_text(&input.patch_summary, 160);
        let task_digest = stable_digest(&input.task_label);
        let patch_summary_digest = stable_digest(&input.patch_summary);
        let prompt_payload_digest = input.prompt_payload.as_deref().map(stable_digest);
        let response_payload_digest = input.response_payload.as_deref().map(stable_digest);
        let privacy_blocked = input
            .prompt_payload
            .as_deref()
            .is_some_and(contains_sensitive_payload)
            || input
                .response_payload
                .as_deref()
                .is_some_and(contains_sensitive_payload);

        let mut blocked_reasons = Vec::new();
        let mut failure_classes = BTreeSet::new();
        if privacy_blocked {
            blocked_reasons.push("rust_repair_privacy_payload_blocked".to_owned());
            failure_classes.insert(RustCodingRepairFailureClass::PrivacyBlocked);
        }

        for required in self.policy.required_kinds() {
            if !input
                .command_evidence
                .iter()
                .any(|evidence| evidence.kind == required)
            {
                blocked_reasons.push(format!("rust_repair_{}_missing", required.as_str()));
                failure_classes.insert(RustCodingRepairFailureClass::MissingEvidence);
            }
            if !input
                .command_evidence
                .iter()
                .any(|evidence| evidence.kind == required && evidence.outcome.passed())
            {
                blocked_reasons.push(format!("rust_repair_{}_not_passed", required.as_str()));
                insert_kind_failure(&mut failure_classes, required);
            }
        }

        for evidence in &input.command_evidence {
            match evidence.outcome {
                RustCodingRepairOutcome::Passed => {}
                RustCodingRepairOutcome::Failed => {
                    insert_kind_failure(&mut failure_classes, evidence.kind);
                }
                RustCodingRepairOutcome::TimedOut => {
                    failure_classes.insert(RustCodingRepairFailureClass::Timeout);
                    insert_kind_failure(&mut failure_classes, evidence.kind);
                }
                RustCodingRepairOutcome::Skipped => {
                    failure_classes.insert(RustCodingRepairFailureClass::MissingEvidence);
                    insert_kind_failure(&mut failure_classes, evidence.kind);
                }
            }
        }

        if self.policy.require_rollback_anchor && input.rollback_anchor_id.trim().is_empty() {
            blocked_reasons.push("rust_repair_rollback_anchor_missing".to_owned());
        }
        if self.policy.require_rollback_replay && !input.rollback_replayed {
            blocked_reasons.push("rust_repair_rollback_replay_missing".to_owned());
        }
        blocked_reasons.sort();
        blocked_reasons.dedup();

        let decision = if privacy_blocked {
            RustCodingRepairDecision::PrivacyBlocked
        } else if input
            .command_evidence
            .iter()
            .any(|evidence| evidence.outcome.failed())
        {
            RustCodingRepairDecision::RetainedForLearning
        } else if blocked_reasons.is_empty() {
            RustCodingRepairDecision::ValidatedCandidate
        } else {
            RustCodingRepairDecision::HeldForEvidence
        };

        let evidence_ids = redacted_evidence_ids(
            input
                .command_evidence
                .iter()
                .map(|evidence| evidence.evidence_id.as_str()),
        );
        let evidence_digest = stable_digest(&format!(
            "{}:{}:{}:{}:{}:{}",
            input.repair_id,
            task_digest,
            patch_summary_digest,
            evidence_ids.join("|"),
            decision.as_str(),
            blocked_reasons.join("|")
        ));

        let validation = validation_from_commands(&input.command_evidence);
        let corpus_report =
            self.improvement_corpus_report(&repair_id, &input, decision, &evidence_ids, validation);
        let (candidate_summaries, no_weight_scorecards) =
            if decision == RustCodingRepairDecision::ValidatedCandidate {
                candidate_scorecards(
                    &input,
                    &repair_id,
                    &evidence_digest,
                    &evidence_ids,
                    validation,
                )
            } else {
                (Vec::new(), Vec::new())
            };

        RustCodingRepairReport {
            repair_id,
            decision,
            task_label,
            task_digest,
            patch_summary_preview,
            patch_summary_digest,
            prompt_payload_digest,
            response_payload_digest,
            command_evidence: input.command_evidence,
            evidence_ids,
            evidence_digest,
            failure_classes: failure_classes.into_iter().collect(),
            blocked_reasons,
            improvement_corpus_report: corpus_report,
            candidate_summaries,
            no_weight_scorecards,
            read_only: true,
            report_only: true,
            preview_only: true,
            auto_commit_allowed: false,
            auto_push_allowed: false,
            durable_promotion_allowed: false,
        }
    }

    fn improvement_corpus_report(
        &self,
        repair_id: &str,
        input: &RustCodingRepairInput,
        decision: RustCodingRepairDecision,
        evidence_ids: &[String],
        validation: SelfEvolutionValidationEvidence,
    ) -> ImprovementCorpusReport {
        let class = match decision {
            RustCodingRepairDecision::ValidatedCandidate => ImprovementEpisodeClass::Accepted,
            RustCodingRepairDecision::PrivacyBlocked => ImprovementEpisodeClass::PrivacyBlocked,
            RustCodingRepairDecision::HeldForEvidence
            | RustCodingRepairDecision::RetainedForLearning => ImprovementEpisodeClass::Failed,
        };
        let validation_status = if decision == RustCodingRepairDecision::ValidatedCandidate {
            ImprovementValidationStatus::Passed
        } else {
            ImprovementValidationStatus::Failed
        };
        let approval_state = if input.operator_approved {
            ImprovementApprovalState::Approved
        } else {
            ImprovementApprovalState::Pending
        };

        let mut episode = ImprovementEpisodeInput::new(repair_id, class)
            .with_task_label(&input.task_label)
            .with_patch_summary(&input.patch_summary)
            .with_compiler(to_improvement_lane(validation.compiler))
            .with_tests(to_improvement_lane(validation.tests))
            .with_benchmarks(to_improvement_lane(validation.benchmarks))
            .with_rollback_anchor(&input.rollback_anchor_id)
            .with_rollback_replayed(input.rollback_replayed)
            .with_approval_state(approval_state)
            .with_validation_status(validation_status);
        if let Some(payload) = &input.prompt_payload {
            episode = episode.with_prompt_payload(payload);
        }
        if let Some(payload) = &input.response_payload {
            episode = episode.with_response_payload(payload);
        }
        for evidence_id in evidence_ids {
            episode = episode.with_evidence_id(evidence_id);
        }

        let mut corpus = ImprovementCorpus::new(format!("rust-repair:{repair_id}"));
        corpus.push_episode(episode);
        corpus.report()
    }
}

fn validation_from_commands(
    commands: &[RustCodingCommandEvidence],
) -> SelfEvolutionValidationEvidence {
    SelfEvolutionValidationEvidence::from_lanes(
        validation_lane(commands, RustCodingRepairCommandKind::Compiler),
        validation_lane(commands, RustCodingRepairCommandKind::Tests),
        validation_lane(commands, RustCodingRepairCommandKind::Benchmarks),
        SelfEvolutionValidationLane::default(),
    )
}

fn validation_lane(
    commands: &[RustCodingCommandEvidence],
    kind: RustCodingRepairCommandKind,
) -> SelfEvolutionValidationLane {
    let mut items = 0_u64;
    let mut passed = 0_u64;
    let mut failed = 0_u64;
    for command in commands.iter().filter(|command| command.kind == kind) {
        items = items.saturating_add(1);
        if command.outcome == RustCodingRepairOutcome::Passed {
            passed = passed.saturating_add(1);
        }
        if matches!(
            command.outcome,
            RustCodingRepairOutcome::Failed | RustCodingRepairOutcome::TimedOut
        ) {
            failed = failed.saturating_add(1);
        }
    }
    SelfEvolutionValidationLane::new(items, passed, failed)
}

fn to_improvement_lane(lane: SelfEvolutionValidationLane) -> ImprovementEvidenceLane {
    ImprovementEvidenceLane::new(lane.items, lane.passed, lane.failed, 0)
}

fn candidate_scorecards(
    input: &RustCodingRepairInput,
    repair_id: &str,
    evidence_digest: &str,
    evidence_ids: &[String],
    validation: SelfEvolutionValidationEvidence,
) -> (
    Vec<RustCodingRepairCandidateSummary>,
    Vec<NoWeightRetrainScorecard>,
) {
    let mut scorecards = Vec::new();
    for lane in [
        NoWeightImprovementLane::Memory,
        NoWeightImprovementLane::Gene,
    ] {
        let candidate_id = format!("rust-repair:{repair_id}:{}", lane.as_str());
        let mut candidate = NoWeightImprovementCandidate::new(&candidate_id, lane)
            .with_rationale(format!(
                "{}:{}:{}",
                input.task_label, input.patch_summary, evidence_digest
            ))
            .with_benchmark_delta(input.benchmark_delta)
            .with_regression_budget(input.regression_budget)
            .with_rollback_anchor(&input.rollback_anchor_id)
            .with_privacy_evidence(format!("privacy:{evidence_digest}"))
            .with_validation(validation)
            .with_operator_approval(input.operator_approved);
        for evidence_id in evidence_ids {
            candidate = candidate.with_evidence_id(evidence_id);
        }
        scorecards.push(NoWeightRetrainGate::new().evaluate(&candidate));
    }

    let summaries = scorecards
        .iter()
        .map(|scorecard| RustCodingRepairCandidateSummary {
            candidate_id: scorecard.candidate_id.clone(),
            lane: scorecard.lane,
            gate_decision: scorecard.decision.as_str().to_owned(),
            evidence_digest: scorecard.evidence_digest.clone(),
            write_allowed: scorecard.write_allowed,
            summary: format!(
                "candidate={} lane={} decision={} write_allowed={} evidence_digest={}",
                scorecard.candidate_id,
                scorecard.lane.as_str(),
                scorecard.decision.as_str(),
                scorecard.write_allowed,
                scorecard.evidence_digest
            ),
        })
        .collect();
    (summaries, scorecards)
}

fn insert_kind_failure(
    failure_classes: &mut BTreeSet<RustCodingRepairFailureClass>,
    kind: RustCodingRepairCommandKind,
) {
    let failure = match kind {
        RustCodingRepairCommandKind::Formatting => RustCodingRepairFailureClass::Formatting,
        RustCodingRepairCommandKind::Compiler => RustCodingRepairFailureClass::Compiler,
        RustCodingRepairCommandKind::Lint => RustCodingRepairFailureClass::Lint,
        RustCodingRepairCommandKind::Tests => RustCodingRepairFailureClass::Tests,
        RustCodingRepairCommandKind::Benchmarks => RustCodingRepairFailureClass::Benchmarks,
    };
    failure_classes.insert(failure);
}

fn redacted_evidence_ids<'a>(ids: impl Iterator<Item = &'a str>) -> Vec<String> {
    ids.map(|id| {
        if let Some((prefix, _)) = id.split_once(':') {
            format!(
                "{}:{}",
                sanitize_identifier(prefix, "evidence"),
                stable_digest(id)
            )
        } else {
            format!("evidence:{}", stable_digest(id))
        }
    })
    .collect::<BTreeSet<_>>()
    .into_iter()
    .collect()
}

fn finite_or_zero(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

fn sanitize_identifier(value: &str, fallback: &str) -> String {
    if value.trim().is_empty() {
        return fallback.to_owned();
    }
    if contains_sensitive_payload(value) {
        return format!("{fallback}:{}", stable_digest(value));
    }
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

fn stable_digest(value: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("fnv64:{hash:016x}")
}

fn optional_json_string(value: Option<&str>) -> String {
    value
        .map(|value| format!("\"{}\"", json_escape(value)))
        .unwrap_or_else(|| "null".to_owned())
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
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn passing_repair_produces_redacted_memory_and_gene_candidates() {
        let report = RustCodingRepairHarness::new().evaluate(
            valid_repair_input("repair-pass")
                .with_operator_approval(true)
                .with_benchmark_delta(0.04)
                .with_regression_budget(0.01),
        );

        assert_eq!(
            report.decision,
            RustCodingRepairDecision::ValidatedCandidate
        );
        assert!(report.failure_classes.is_empty());
        assert_eq!(report.passed_commands(), 5);
        assert_eq!(report.candidate_summaries.len(), 2);
        assert_eq!(report.no_weight_scorecards.len(), 2);
        assert!(
            report
                .candidate_summaries
                .iter()
                .any(|candidate| candidate.lane == NoWeightImprovementLane::Memory)
        );
        assert!(
            report
                .candidate_summaries
                .iter()
                .any(|candidate| candidate.lane == NoWeightImprovementLane::Gene)
        );
        assert!(
            report
                .no_weight_scorecards
                .iter()
                .all(|card| !card.write_allowed)
        );
        assert!(!report.auto_commit_allowed);
        assert!(!report.auto_push_allowed);
        assert!(!report.durable_promotion_allowed);
        assert!(report.summary_line().contains("candidates=2"));
    }

    #[test]
    fn snippet_fixture_runs_compiler_and_classifies_compile_failure() {
        let work_dir = target_test_dir("rust-repair-compile-failure");
        let validator = RustSnippetValidator::new(&work_dir);
        let report = RustCodingRepairHarness::new()
            .run_snippet_fixture(
                &validator,
                &RustSnippetCheck::new("pub fn broken() -> u32 { missing_symbol }"),
                base_repair_input("repair-compiler")
                    .with_command(RustCodingCommandEvidence::passed(
                        RustCodingRepairCommandKind::Formatting,
                    ))
                    .with_command(RustCodingCommandEvidence::passed(
                        RustCodingRepairCommandKind::Lint,
                    ))
                    .with_command(RustCodingCommandEvidence::passed(
                        RustCodingRepairCommandKind::Tests,
                    ))
                    .with_command(RustCodingCommandEvidence::passed(
                        RustCodingRepairCommandKind::Benchmarks,
                    )),
            )
            .unwrap();

        assert_eq!(
            report.decision,
            RustCodingRepairDecision::RetainedForLearning
        );
        assert!(
            report
                .failure_classes
                .contains(&RustCodingRepairFailureClass::Compiler)
        );
        assert!(report.candidate_summaries.is_empty());
        assert_eq!(report.improvement_corpus_report.failed_episodes, 1);
        fs::remove_dir_all(work_dir).unwrap();
    }

    #[test]
    fn test_failure_is_retained_without_promotion() {
        let report = RustCodingRepairHarness::new().evaluate(
            base_repair_input("repair-test-failure")
                .with_command(RustCodingCommandEvidence::passed(
                    RustCodingRepairCommandKind::Formatting,
                ))
                .with_command(RustCodingCommandEvidence::passed(
                    RustCodingRepairCommandKind::Compiler,
                ))
                .with_command(RustCodingCommandEvidence::passed(
                    RustCodingRepairCommandKind::Lint,
                ))
                .with_command(RustCodingCommandEvidence::failed(
                    RustCodingRepairCommandKind::Tests,
                    "assertion failed in regression fixture",
                ))
                .with_command(RustCodingCommandEvidence::passed(
                    RustCodingRepairCommandKind::Benchmarks,
                )),
        );

        assert_eq!(
            report.decision,
            RustCodingRepairDecision::RetainedForLearning
        );
        assert!(
            report
                .failure_classes
                .contains(&RustCodingRepairFailureClass::Tests)
        );
        assert!(report.candidate_summaries.is_empty());
        assert!(
            report
                .blocked_reasons
                .contains(&"rust_repair_tests_not_passed".to_owned())
        );
    }

    #[test]
    fn timeout_is_tagged_as_timeout_and_non_promoted() {
        let report = RustCodingRepairHarness::new().evaluate(
            base_repair_input("repair-timeout")
                .with_command(RustCodingCommandEvidence::passed(
                    RustCodingRepairCommandKind::Formatting,
                ))
                .with_command(RustCodingCommandEvidence::passed(
                    RustCodingRepairCommandKind::Compiler,
                ))
                .with_command(RustCodingCommandEvidence::timed_out(
                    RustCodingRepairCommandKind::Lint,
                    "clippy timed out after sandbox budget",
                ))
                .with_command(RustCodingCommandEvidence::passed(
                    RustCodingRepairCommandKind::Tests,
                ))
                .with_command(RustCodingCommandEvidence::passed(
                    RustCodingRepairCommandKind::Benchmarks,
                )),
        );

        assert_eq!(
            report.decision,
            RustCodingRepairDecision::RetainedForLearning
        );
        assert!(
            report
                .failure_classes
                .contains(&RustCodingRepairFailureClass::Timeout)
        );
        assert!(
            report
                .failure_classes
                .contains(&RustCodingRepairFailureClass::Lint)
        );
        assert_eq!(report.timed_out_commands(), 1);
        assert!(report.candidate_summaries.is_empty());
    }

    #[test]
    fn redaction_digests_private_payloads_without_leaking_raw_text() {
        let secret = "SECRET_TOKEN=sk-private-do-not-log";
        let report = RustCodingRepairHarness::new().evaluate(
            valid_repair_input("repair-redaction")
                .with_prompt_payload(format!("private: {secret}"))
                .with_response_payload(format!("repair notes {secret}"))
                .with_command(RustCodingCommandEvidence::failed(
                    RustCodingRepairCommandKind::Compiler,
                    format!("compiler diagnostic included {secret}"),
                )),
        );
        let json = report.json_line();
        let summary = report.summary_line();

        assert_eq!(report.decision, RustCodingRepairDecision::PrivacyBlocked);
        assert!(
            report
                .failure_classes
                .contains(&RustCodingRepairFailureClass::PrivacyBlocked)
        );
        assert_eq!(report.improvement_corpus_report.privacy_blocked_episodes, 1);
        assert!(!json.contains(secret));
        assert!(!summary.contains(secret));
        assert!(!report.evidence_digest.contains(secret));
        assert!(report.prompt_payload_digest.is_some());
        assert!(report.response_payload_digest.is_some());
        assert!(
            report
                .command_evidence
                .iter()
                .all(|evidence| !evidence.diagnostic_preview.contains(secret))
        );
    }

    #[test]
    fn missing_format_lint_or_benchmark_evidence_is_held() {
        let report = RustCodingRepairHarness::new().evaluate(
            base_repair_input("repair-missing")
                .with_command(RustCodingCommandEvidence::passed(
                    RustCodingRepairCommandKind::Compiler,
                ))
                .with_command(RustCodingCommandEvidence::passed(
                    RustCodingRepairCommandKind::Tests,
                )),
        );

        assert_eq!(report.decision, RustCodingRepairDecision::HeldForEvidence);
        assert!(
            report
                .failure_classes
                .contains(&RustCodingRepairFailureClass::MissingEvidence)
        );
        assert!(
            report
                .failure_classes
                .contains(&RustCodingRepairFailureClass::Formatting)
        );
        assert!(
            report
                .failure_classes
                .contains(&RustCodingRepairFailureClass::Lint)
        );
        assert!(
            report
                .failure_classes
                .contains(&RustCodingRepairFailureClass::Benchmarks)
        );
        assert!(report.candidate_summaries.is_empty());
    }

    fn valid_repair_input(id: &str) -> RustCodingRepairInput {
        base_repair_input(id)
            .with_command(RustCodingCommandEvidence::passed(
                RustCodingRepairCommandKind::Formatting,
            ))
            .with_command(RustCodingCommandEvidence::passed(
                RustCodingRepairCommandKind::Compiler,
            ))
            .with_command(RustCodingCommandEvidence::passed(
                RustCodingRepairCommandKind::Lint,
            ))
            .with_command(RustCodingCommandEvidence::passed(
                RustCodingRepairCommandKind::Tests,
            ))
            .with_command(RustCodingCommandEvidence::passed(
                RustCodingRepairCommandKind::Benchmarks,
            ))
    }

    fn base_repair_input(id: &str) -> RustCodingRepairInput {
        RustCodingRepairInput::new(id)
            .with_task_label("rust compile repair")
            .with_patch_summary("replace failing code path with Result and regression test")
            .with_prompt_payload("compiler diagnostic digest only")
            .with_response_payload("repair summary digest only")
            .with_rollback_anchor("rollback:rust-repair")
            .with_rollback_replayed(true)
    }

    fn target_test_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        Path::new("target").join(format!("{name}-{unique}"))
    }
}
