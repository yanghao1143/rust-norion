use std::path::{Path, PathBuf};

#[cfg(test)]
const VALIDATION_COMMAND_STRICT_COVERAGE_SCHEMA_FIELDS: [&str; 4] = [
    "validation_command.strict_coverage_requested",
    "validation_command.coverage_tooling_evidence",
    "validation_command.coverage_report_evidence",
    "validation_command.coverage_tooling_or_report_evidence_present",
];

#[cfg(test)]
const VALIDATION_COMMAND_STRICT_COVERAGE_BOUNDARY_SOURCES: [(&str, &str); 4] = [
    (
        "validation_command.strict_coverage_requested",
        "ValidationCommandCoverageEvidence::strict_coverage_is_requested",
    ),
    (
        "validation_command.coverage_tooling_evidence",
        "ValidationCommandCoverageEvidence::with_coverage_tooling_evidence",
    ),
    (
        "validation_command.coverage_report_evidence",
        "ValidationCommandCoverageEvidence::with_coverage_report_evidence",
    ),
    (
        "validation_command.coverage_tooling_or_report_evidence_present",
        "ValidationCommandCoverageEvidence::coverage_tooling_or_report_evidence_present",
    ),
];

#[cfg(test)]
fn validation_command_strict_coverage_schema_fields() -> &'static [&'static str] {
    &VALIDATION_COMMAND_STRICT_COVERAGE_SCHEMA_FIELDS
}

#[cfg(test)]
fn validation_command_strict_coverage_boundary_sources() -> &'static [(&'static str, &'static str)]
{
    &VALIDATION_COMMAND_STRICT_COVERAGE_BOUNDARY_SOURCES
}

#[cfg(test)]
fn has_validation_command_strict_coverage_schema_bundle<F>(mut contains_field: F) -> bool
where
    F: FnMut(&str) -> bool,
{
    validation_command_strict_coverage_schema_fields()
        .iter()
        .all(|field| contains_field(*field))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerificationPhase {
    PreRound,
    PostRound,
    Both,
    ReportOnly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerificationTool {
    Cargo,
    Rustc,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationCommand {
    pub tool: VerificationTool,
    pub program: String,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
    pub timeout_secs: u64,
    pub phase: VerificationPhase,
}

impl VerificationCommand {
    pub fn cargo<I, S>(args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            tool: VerificationTool::Cargo,
            program: "cargo".to_owned(),
            args: args.into_iter().map(Into::into).collect(),
            cwd: None,
            timeout_secs: 300,
            phase: VerificationPhase::PreRound,
        }
    }

    pub fn rustc<I, S>(args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            tool: VerificationTool::Rustc,
            program: "rustc".to_owned(),
            args: args.into_iter().map(Into::into).collect(),
            cwd: None,
            timeout_secs: 120,
            phase: VerificationPhase::PreRound,
        }
    }

    pub fn with_cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    pub fn with_timeout_secs(mut self, timeout_secs: u64) -> Self {
        self.timeout_secs = timeout_secs.max(1);
        self
    }

    pub fn with_phase(mut self, phase: VerificationPhase) -> Self {
        self.phase = phase;
        self
    }

    pub fn display_line(&self) -> String {
        let args = self
            .args
            .iter()
            .map(|arg| shell_quote(arg))
            .collect::<Vec<_>>()
            .join(" ");
        if args.is_empty() {
            self.program.clone()
        } else {
            format!("{} {args}", self.program)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationPlan {
    pub name: String,
    pub commands: Vec<VerificationCommand>,
}

impl VerificationPlan {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            commands: Vec::new(),
        }
    }

    pub fn add_command(mut self, command: VerificationCommand) -> Self {
        self.commands.push(command);
        self
    }

    pub fn cargo_test_manifest(manifest_path: impl AsRef<Path>) -> Self {
        let manifest = manifest_path.as_ref().to_string_lossy().into_owned();
        Self::new("cargo-test-manifest").add_command(VerificationCommand::cargo([
            "test".to_owned(),
            "--manifest-path".to_owned(),
            manifest,
        ]))
    }

    pub fn cargo_check_manifest(manifest_path: impl AsRef<Path>) -> Self {
        let manifest = manifest_path.as_ref().to_string_lossy().into_owned();
        Self::new("cargo-check-manifest").add_command(VerificationCommand::cargo([
            "check".to_owned(),
            "--manifest-path".to_owned(),
            manifest,
        ]))
    }

    pub fn commands_for_phase(&self, phase: VerificationPhase) -> Vec<&VerificationCommand> {
        self.commands
            .iter()
            .filter(|command| {
                command.phase == phase
                    || command.phase == VerificationPhase::Both
                    || phase == VerificationPhase::Both
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutcome {
    pub command_line: String,
    pub status_code: Option<i32>,
    pub elapsed_ms: u64,
    pub stdout_tail: String,
    pub stderr_tail: String,
}

impl CommandOutcome {
    pub fn passed(&self) -> bool {
        self.status_code == Some(0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationObservation {
    pub checked: bool,
    pub passed: bool,
    pub phase: VerificationPhase,
    pub outcome: Option<CommandOutcome>,
}

impl ValidationObservation {
    pub fn skipped(phase: VerificationPhase) -> Self {
        Self {
            checked: false,
            passed: false,
            phase,
            outcome: None,
        }
    }

    pub fn from_outcome(phase: VerificationPhase, outcome: CommandOutcome) -> Self {
        Self {
            checked: true,
            passed: outcome.passed(),
            phase,
            outcome: Some(outcome),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmartSteamCase {
    pub id: String,
    pub prompt: String,
    pub endpoint: String,
    pub max_tokens: u64,
    pub require_feedback: bool,
    pub require_self_improve: bool,
    pub rust_check_code: Option<String>,
}

impl SmartSteamCase {
    pub fn business_cycle(id: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            prompt: prompt.into(),
            endpoint: "/v1/business-cycle-stream".to_owned(),
            max_tokens: 4096,
            require_feedback: true,
            require_self_improve: true,
            rust_check_code: None,
        }
    }

    pub fn with_max_tokens(mut self, max_tokens: u64) -> Self {
        self.max_tokens = max_tokens.max(1);
        self
    }

    pub fn with_rust_check_code(mut self, code: impl Into<String>) -> Self {
        self.rust_check_code = Some(code.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamContinuityCheck {
    pub saw_done: bool,
    pub saw_error: bool,
    pub saw_final: bool,
    pub incomplete_buffer_bytes: usize,
    pub delta_chars: usize,
}

impl StreamContinuityCheck {
    pub fn passed(&self) -> bool {
        self.saw_done && !self.saw_error && self.saw_final && self.incomplete_buffer_bytes == 0
    }

    pub fn failure_reason(&self) -> Option<String> {
        if self.saw_error {
            Some("stream emitted error event".to_owned())
        } else if !self.saw_done {
            Some("stream truncated before done event".to_owned())
        } else if !self.saw_final {
            Some("stream ended without final event".to_owned())
        } else if self.incomplete_buffer_bytes > 0 {
            Some(format!(
                "stream ended with {} buffered byte(s)",
                self.incomplete_buffer_bytes
            ))
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendHealthCheckPlan {
    pub backend_url: String,
    pub require_model_ready: bool,
    pub require_safe_device: bool,
    pub require_experience_hygiene: bool,
    pub min_runtime_context_tokens: Option<u64>,
}

impl BackendHealthCheckPlan {
    pub fn local(backend_url: impl Into<String>) -> Self {
        Self {
            backend_url: backend_url.into(),
            require_model_ready: true,
            require_safe_device: true,
            require_experience_hygiene: true,
            min_runtime_context_tokens: None,
        }
    }

    pub fn with_min_runtime_context_tokens(mut self, tokens: u64) -> Self {
        self.min_runtime_context_tokens = Some(tokens);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExperienceAuditPlan {
    pub endpoint: String,
    pub limit: u64,
    pub max_noisy_records: u64,
    pub max_noise_penalty: f64,
    pub max_quarantine_candidates: u64,
    pub max_repairable_legacy_metadata_lessons: u64,
    pub max_legacy_metadata_without_clean_gist: u64,
}

impl ExperienceAuditPlan {
    pub fn cleanup_audit(limit: u64) -> Self {
        Self {
            endpoint: "/v1/experience-cleanup-audit".to_owned(),
            limit: limit.max(1),
            max_noisy_records: 0,
            max_noise_penalty: 0.0,
            max_quarantine_candidates: 0,
            max_repairable_legacy_metadata_lessons: 0,
            max_legacy_metadata_without_clean_gist: 0,
        }
    }

    pub fn with_noise_thresholds(mut self, max_noisy_records: u64, max_noise_penalty: f64) -> Self {
        self.max_noisy_records = max_noisy_records;
        self.max_noise_penalty = max_noise_penalty.max(0.0);
        self
    }

    pub fn with_legacy_metadata_thresholds(
        mut self,
        max_repairable: u64,
        max_without_clean_gist: u64,
    ) -> Self {
        self.max_repairable_legacy_metadata_lessons = max_repairable;
        self.max_legacy_metadata_without_clean_gist = max_without_clean_gist;
        self
    }

    pub fn with_quarantine_candidates(mut self, max_quarantine_candidates: u64) -> Self {
        self.max_quarantine_candidates = max_quarantine_candidates;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliUiSmokePlan {
    pub name: String,
    pub backend: BackendHealthCheckPlan,
    pub case: SmartSteamCase,
    pub cli_plan: VerificationPlan,
    pub web_lab_url: Option<String>,
}

impl CliUiSmokePlan {
    pub fn new(
        name: impl Into<String>,
        backend: BackendHealthCheckPlan,
        case: SmartSteamCase,
        cli_plan: VerificationPlan,
    ) -> Self {
        Self {
            name: name.into(),
            backend,
            case,
            cli_plan,
            web_lab_url: None,
        }
    }

    pub fn with_web_lab_url(mut self, url: impl Into<String>) -> Self {
        self.web_lab_url = Some(url.into());
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelRole {
    Planner,
    Reviewer,
    Tester,
    Summarizer,
    HighQuality,
}

impl ModelRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Planner => "planner",
            Self::Reviewer => "reviewer",
            Self::Tester => "tester",
            Self::Summarizer => "summarizer",
            Self::HighQuality => "high_quality",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelWorkerPlan {
    pub id: String,
    pub role: ModelRole,
    pub model: String,
    pub endpoint: String,
    pub max_tokens: u64,
    pub timeout_ms: u64,
    pub may_block_primary_12b: bool,
    pub validation_plan: VerificationPlan,
}

impl ModelWorkerPlan {
    pub fn new(id: impl Into<String>, role: ModelRole, model: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            role,
            model: model.into(),
            endpoint: "/v1/business-cycle-stream".to_owned(),
            max_tokens: 2048,
            timeout_ms: 120_000,
            may_block_primary_12b: false,
            validation_plan: VerificationPlan::new("model-worker-validation"),
        }
    }

    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = endpoint.into();
        self
    }

    pub fn with_max_tokens(mut self, max_tokens: u64) -> Self {
        self.max_tokens = max_tokens.max(1);
        self
    }

    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms.max(1);
        self
    }

    pub fn with_primary_blocking(mut self, may_block: bool) -> Self {
        self.may_block_primary_12b = may_block;
        self
    }

    pub fn with_validation_plan(mut self, validation_plan: VerificationPlan) -> Self {
        self.validation_plan = validation_plan;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelPoolSmokePlan {
    pub name: String,
    pub primary_model: String,
    pub workers: Vec<ModelWorkerPlan>,
    pub merge_validation_plan: VerificationPlan,
}

impl ModelPoolSmokePlan {
    pub fn new(name: impl Into<String>, primary_model: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            primary_model: primary_model.into(),
            workers: Vec::new(),
            merge_validation_plan: VerificationPlan::new("model-pool-merge-validation"),
        }
    }

    pub fn add_worker(mut self, worker: ModelWorkerPlan) -> Self {
        self.workers.push(worker);
        self
    }

    pub fn with_merge_validation_plan(mut self, validation_plan: VerificationPlan) -> Self {
        self.merge_validation_plan = validation_plan;
        self
    }

    pub fn worker_count(&self) -> usize {
        self.workers.len()
    }

    pub fn primary_blocking_worker_count(&self) -> usize {
        self.workers
            .iter()
            .filter(|worker| worker.may_block_primary_12b)
            .count()
    }

    pub fn has_role(&self, role: ModelRole) -> bool {
        self.workers.iter().any(|worker| worker.role == role)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterAcceptanceStage {
    ShadowOnly,
    ReportOnly,
    Enforced,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterAcceptancePlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub root_business_cycle_endpoint: String,
    pub model_pool_plan: ModelPoolSmokePlan,
    pub report_schema_name: String,
    pub worker_event_name: String,
    pub outage_attribution_required: bool,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl AdapterAcceptancePlan {
    pub fn root_business_cycle(
        name: impl Into<String>,
        stage: AdapterAcceptanceStage,
        model_pool_plan: ModelPoolSmokePlan,
    ) -> Self {
        Self {
            name: name.into(),
            stage,
            root_business_cycle_endpoint: "/v1/business-cycle-stream".to_owned(),
            model_pool_plan,
            report_schema_name: "model_pool_report_v1".to_owned(),
            worker_event_name: "model_worker_v1".to_owned(),
            outage_attribution_required: stage != AdapterAcceptanceStage::ShadowOnly,
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn with_verification_plan(mut self, verification_plan: VerificationPlan) -> Self {
        self.verification_plan = verification_plan;
        self
    }

    pub fn may_block_current_runner(&self) -> bool {
        self.stage == AdapterAcceptanceStage::Enforced
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelPoolEffectivenessPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub worker_event_name: String,
    pub report_schema_name: String,
    pub required_worker_fields: Vec<String>,
    pub enforced_worker_fields: Vec<String>,
    pub non_quality_failure_kinds: Vec<String>,
    pub preserves_primary_12b: bool,
}

impl ModelPoolEffectivenessPlan {
    pub fn apple_silicon(stage: AdapterAcceptanceStage) -> Self {
        Self {
            name: "apple-silicon-model-pool-effectiveness".to_owned(),
            stage,
            worker_event_name: "model_worker_v1".to_owned(),
            report_schema_name: "model_worker_gate_report_v1".to_owned(),
            required_worker_fields: vec![
                "worker_id".to_owned(),
                "role".to_owned(),
                "model".to_owned(),
                "latency_ms".to_owned(),
                "runtime_tokens".to_owned(),
                "success".to_owned(),
                "feedback_applied".to_owned(),
                "validation_checked".to_owned(),
                "validation_passed".to_owned(),
            ],
            enforced_worker_fields: vec![
                "duplicate_output".to_owned(),
                "noisy_output".to_owned(),
                "blocked_primary_12b".to_owned(),
                "failure_kind".to_owned(),
                "development_claim_allowed".to_owned(),
                "claim_blockers".to_owned(),
            ],
            non_quality_failure_kinds: vec![
                "chain_not_ready".to_owned(),
                "model_unavailable".to_owned(),
            ],
            preserves_primary_12b: true,
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        self.stage == AdapterAcceptanceStage::Enforced
    }

    pub fn requires_field(&self, field: &str) -> bool {
        self.required_worker_fields
            .iter()
            .chain(self.enforced_worker_fields.iter())
            .any(|required| required == field)
    }

    pub fn treats_as_operational(&self, failure_kind: &str) -> bool {
        self.non_quality_failure_kinds
            .iter()
            .any(|kind| kind == failure_kind)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModelPoolBudgetFairnessPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_name: String,
    pub required_roles: Vec<ModelRole>,
    pub required_report_fields: Vec<String>,
    pub max_role_runtime_token_share: f64,
    pub require_role_feedback: bool,
    pub require_no_primary_12b_blockers: bool,
    pub preserves_legacy_runner: bool,
}

impl ModelPoolBudgetFairnessPlan {
    pub fn apple_silicon(stage: AdapterAcceptanceStage) -> Self {
        Self {
            name: "apple-silicon-model-pool-budget-fairness".to_owned(),
            stage,
            report_schema_name: "model_pool_budget_fairness_report_v1".to_owned(),
            required_roles: vec![ModelRole::Planner, ModelRole::Reviewer, ModelRole::Tester],
            required_report_fields: vec![
                "model_pool_budget.roles".to_owned(),
                "model_pool_budget.workers_by_role".to_owned(),
                "model_pool_budget.runtime_tokens_by_role".to_owned(),
                "model_pool_budget.runtime_token_share_by_role".to_owned(),
                "model_pool_budget.dominant_runtime_token_roles".to_owned(),
                "model_pool_budget.missing_required_roles".to_owned(),
                "model_pool_budget.max_role_runtime_token_share".to_owned(),
                "model_pool_budget.allow_pool_expansion".to_owned(),
            ],
            max_role_runtime_token_share: 0.60,
            require_role_feedback: true,
            require_no_primary_12b_blockers: true,
            preserves_legacy_runner: true,
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        self.stage == AdapterAcceptanceStage::Enforced
    }

    pub fn requires_role(&self, role: ModelRole) -> bool {
        self.required_roles.contains(&role)
    }

    pub fn requires_report_field(&self, field: &str) -> bool {
        self.required_report_fields
            .iter()
            .any(|required| required == field)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModelPoolDevelopmentWindowPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_name: String,
    pub min_rounds: usize,
    pub min_development_claim_rate: f64,
    pub min_feedback_delta_total: i64,
    pub max_latency_multiplier: f64,
    pub max_token_multiplier: f64,
    pub require_no_duplicate_or_noisy_output: bool,
    pub require_no_primary_12b_blockers: bool,
    pub keep_operational_failures_out_of_quality: bool,
    pub verification_plan: VerificationPlan,
}

impl ModelPoolDevelopmentWindowPlan {
    pub fn apple_silicon(stage: AdapterAcceptanceStage) -> Self {
        Self {
            name: "apple-silicon-model-pool-development-window".to_owned(),
            stage,
            report_schema_name: "model_pool_development_window_report_v1".to_owned(),
            min_rounds: 3,
            min_development_claim_rate: 0.67,
            min_feedback_delta_total: 3,
            max_latency_multiplier: 1.5,
            max_token_multiplier: 2.0,
            require_no_duplicate_or_noisy_output: true,
            require_no_primary_12b_blockers: true,
            keep_operational_failures_out_of_quality: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        self.stage == AdapterAcceptanceStage::Enforced
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AppleSiliconBaselineComparisonPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_name: String,
    pub min_paired_rounds: usize,
    pub min_feedback_gain_rounds: usize,
    pub min_feedback_delta_total: i64,
    pub max_success_regression_rounds: usize,
    pub max_validation_regression_rounds: usize,
    pub require_latency_budget: bool,
    pub require_token_budget: bool,
    pub require_no_duplicate_or_noisy_output: bool,
    pub require_no_primary_12b_blockers: bool,
    pub keep_operational_failures_out_of_quality: bool,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl AppleSiliconBaselineComparisonPlan {
    pub fn apple_silicon(stage: AdapterAcceptanceStage) -> Self {
        let enforced = stage == AdapterAcceptanceStage::Enforced;
        Self {
            name: "apple-silicon-baseline-comparison".to_owned(),
            stage,
            report_schema_name: "apple_silicon_baseline_comparison_report_v1".to_owned(),
            min_paired_rounds: 3,
            min_feedback_gain_rounds: 2,
            min_feedback_delta_total: 3,
            max_success_regression_rounds: 0,
            max_validation_regression_rounds: 0,
            require_latency_budget: enforced,
            require_token_budget: enforced,
            require_no_duplicate_or_noisy_output: enforced,
            require_no_primary_12b_blockers: enforced,
            keep_operational_failures_out_of_quality: true,
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        self.stage == AdapterAcceptanceStage::Enforced
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppleSiliconBaselineAdapterWiringPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub eval_contract_name: String,
    pub current_projection_fields: Vec<String>,
    pub required_future_events: Vec<String>,
    pub rollout_steps: Vec<String>,
    pub require_paired_baseline_events_before_enforcement: bool,
    pub require_outage_attribution_before_quality: bool,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl AppleSiliconBaselineAdapterWiringPlan {
    pub fn root_business_cycle(stage: AdapterAcceptanceStage) -> Self {
        let enforced = stage == AdapterAcceptanceStage::Enforced;
        Self {
            name: "apple-silicon-baseline-adapter-wiring".to_owned(),
            stage,
            eval_contract_name: "AppleSiliconBaselineAdapterPlan::root_business_cycle_json"
                .to_owned(),
            current_projection_fields: vec![
                "round".to_owned(),
                "pool_feedback_applied_total".to_owned(),
                "pool_latency_ms_total".to_owned(),
                "pool_runtime_tokens_total".to_owned(),
                "pool_success".to_owned(),
                "pool_validation_passed".to_owned(),
                "duplicate_outputs".to_owned(),
                "noisy_outputs".to_owned(),
                "primary_12b_blockers".to_owned(),
                "root_adapter_failure_kind".to_owned(),
            ],
            required_future_events: vec![
                "baseline_12b_feedback_applied".to_owned(),
                "baseline_12b_latency_ms".to_owned(),
                "baseline_12b_runtime_tokens".to_owned(),
                "baseline_12b_success".to_owned(),
                "baseline_12b_validation_passed".to_owned(),
            ],
            rollout_steps: vec![
                "shadow-project-pool-side-baseline-fields".to_owned(),
                "report-only-paired-baseline-coverage".to_owned(),
                "report-only-apple-silicon-baseline-comparison".to_owned(),
                "enforced-apple-silicon-baseline-comparison".to_owned(),
            ],
            require_paired_baseline_events_before_enforcement: enforced,
            require_outage_attribution_before_quality: true,
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        self.stage == AdapterAcceptanceStage::Enforced
    }

    pub fn requires_future_event(&self, event: &str) -> bool {
        self.required_future_events
            .iter()
            .any(|required| required == event)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModelPoolDevelopmentAttributionPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_name: String,
    pub required_worker_fields: Vec<String>,
    pub required_failure_kinds: Vec<String>,
    pub required_roles: Vec<ModelRole>,
    pub require_runtime_metrics_for_success: bool,
    pub require_validation_checked: bool,
    pub require_feedback_applied: bool,
    pub require_no_duplicate_or_noisy_output: bool,
    pub require_no_primary_12b_blockers: bool,
    pub keep_operational_failures_out_of_quality: bool,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl ModelPoolDevelopmentAttributionPlan {
    pub fn apple_silicon(stage: AdapterAcceptanceStage) -> Self {
        Self {
            name: "apple-silicon-model-pool-development-attribution".to_owned(),
            stage,
            report_schema_name: "model_pool_development_attribution_report_v1".to_owned(),
            required_worker_fields: vec![
                "worker_id".to_owned(),
                "role".to_owned(),
                "model".to_owned(),
                "latency_ms".to_owned(),
                "runtime_tokens".to_owned(),
                "success".to_owned(),
                "feedback_applied".to_owned(),
                "validation_checked".to_owned(),
                "validation_passed".to_owned(),
                "duplicate_output".to_owned(),
                "noisy_output".to_owned(),
                "blocked_primary_12b".to_owned(),
                "failure_kind".to_owned(),
                "worker_development_claim_allowed".to_owned(),
                "worker_claim_blockers".to_owned(),
            ],
            required_failure_kinds: vec![
                "none".to_owned(),
                "chain_not_ready".to_owned(),
                "model_unavailable".to_owned(),
                "model_quality_failure".to_owned(),
            ],
            required_roles: vec![ModelRole::Planner, ModelRole::Reviewer, ModelRole::Tester],
            require_runtime_metrics_for_success: stage == AdapterAcceptanceStage::Enforced,
            require_validation_checked: stage == AdapterAcceptanceStage::Enforced,
            require_feedback_applied: stage == AdapterAcceptanceStage::Enforced,
            require_no_duplicate_or_noisy_output: stage == AdapterAcceptanceStage::Enforced,
            require_no_primary_12b_blockers: stage == AdapterAcceptanceStage::Enforced,
            keep_operational_failures_out_of_quality: true,
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        self.stage == AdapterAcceptanceStage::Enforced
    }

    pub fn requires_worker_field(&self, field: &str) -> bool {
        self.required_worker_fields
            .iter()
            .any(|required| required == field)
    }

    pub fn requires_failure_kind(&self, failure_kind: &str) -> bool {
        self.required_failure_kinds
            .iter()
            .any(|required| required == failure_kind)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerRootFailureConsistencyPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_name: String,
    pub required_input_reports: Vec<String>,
    pub require_single_worker_agreement_before_enforcement: bool,
    pub keep_operational_failures_out_of_quality: bool,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl WorkerRootFailureConsistencyPlan {
    pub fn legacy_single_worker_projection(stage: AdapterAcceptanceStage) -> Self {
        Self {
            name: "worker-root-failure-consistency".to_owned(),
            stage,
            report_schema_name: "worker_root_failure_consistency_report_v1".to_owned(),
            required_input_reports: vec![
                "model_worker_v1".to_owned(),
                "root_adapter_attribution_report_v1".to_owned(),
            ],
            require_single_worker_agreement_before_enforcement: stage
                == AdapterAcceptanceStage::Enforced,
            keep_operational_failures_out_of_quality: true,
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        self.stage == AdapterAcceptanceStage::Enforced
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AppleSiliconDevelopmentEffectPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_name: String,
    pub required_input_reports: Vec<String>,
    pub required_worker_fields: Vec<String>,
    pub required_effect_report_fields: Vec<String>,
    pub required_attribution_rules: Vec<String>,
    pub require_worker_metric_coverage: bool,
    pub require_budget_fairness: bool,
    pub require_development_window: bool,
    pub forbid_operational_quality_confusion: bool,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl AppleSiliconDevelopmentEffectPlan {
    pub fn apple_silicon(stage: AdapterAcceptanceStage) -> Self {
        let enforced = stage == AdapterAcceptanceStage::Enforced;
        Self {
            name: "apple-silicon-development-effect".to_owned(),
            stage,
            report_schema_name: "apple_silicon_development_effect_report_v1".to_owned(),
            required_input_reports: vec![
                "model_pool_development_attribution_report_v1".to_owned(),
                "model_pool_budget_fairness_report_v1".to_owned(),
                "model_pool_development_window_report_v1".to_owned(),
                "apple_silicon_baseline_comparison_report_v1".to_owned(),
                "root_adapter_attribution_report_v1".to_owned(),
            ],
            required_worker_fields: vec![
                "latency_ms".to_owned(),
                "runtime_tokens".to_owned(),
                "success".to_owned(),
                "feedback_applied".to_owned(),
                "validation_checked".to_owned(),
                "validation_passed".to_owned(),
                "duplicate_output".to_owned(),
                "noisy_output".to_owned(),
                "blocked_primary_12b".to_owned(),
                "failure_kind".to_owned(),
            ],
            required_effect_report_fields: vec![
                "apple_silicon_effect.worker_ids".to_owned(),
                "apple_silicon_effect.roles".to_owned(),
                "apple_silicon_effect.latency_ms".to_owned(),
                "apple_silicon_effect.runtime_tokens".to_owned(),
                "apple_silicon_effect.success".to_owned(),
                "apple_silicon_effect.feedback_applied".to_owned(),
                "apple_silicon_effect.validation_checked".to_owned(),
                "apple_silicon_effect.validation_passed".to_owned(),
                "apple_silicon_effect.duplicate_output".to_owned(),
                "apple_silicon_effect.noisy_output".to_owned(),
                "apple_silicon_effect.blocked_primary_12b".to_owned(),
                "apple_silicon_effect.failure_kinds".to_owned(),
                "apple_silicon_effect.worker_development_claim_allowed".to_owned(),
                "apple_silicon_effect.worker_claim_blockers".to_owned(),
                "apple_silicon_effect.operational_readiness_failures".to_owned(),
                "apple_silicon_effect.chain_not_ready_count".to_owned(),
                "apple_silicon_effect.model_unavailable_count".to_owned(),
                "apple_silicon_effect.model_quality_failure_count".to_owned(),
                "apple_silicon_effect.reported_model_quality_failures".to_owned(),
                "apple_silicon_effect.operational_readiness_failure_kind".to_owned(),
                "apple_silicon_effect.model_quality_failure_allowed".to_owned(),
                "apple_silicon_effect.quality_failure_blocked_by_readiness_order".to_owned(),
                "apple_silicon_effect.operational_failure_counted_as_quality".to_owned(),
                "apple_silicon_effect.duplicate_outputs".to_owned(),
                "apple_silicon_effect.noisy_outputs".to_owned(),
                "apple_silicon_effect.primary_12b_blockers".to_owned(),
                "apple_silicon_effect.worker_metric_rows_consistent".to_owned(),
                "apple_silicon_effect.worker_metric_coverage_passed".to_owned(),
                "apple_silicon_effect.validation_unchecked_workers".to_owned(),
                "apple_silicon_effect.validation_failed_workers".to_owned(),
                "apple_silicon_effect.successful_workers_missing_runtime_metrics".to_owned(),
                "apple_silicon_effect.allow_development_effect_claim".to_owned(),
            ],
            required_attribution_rules: vec![
                "prompt_gate_blocked_and_8686_down=chain_not_ready".to_owned(),
                "prompt_gate_blocked_and_8686_down!=model_quality_failure".to_owned(),
                "prompt_gate_passed_and_8686_down=model_unavailable".to_owned(),
                "prompt_gate_passed_and_8686_down!=model_quality_failure".to_owned(),
                "chain_not_ready_or_model_unavailable_blocks_claim_not_quality".to_owned(),
                "model_quality_failure_requires_final_json_runtime_model_tokens_failed_business_cycle"
                    .to_owned(),
            ],
            require_worker_metric_coverage: enforced,
            require_budget_fairness: enforced,
            require_development_window: enforced,
            forbid_operational_quality_confusion: true,
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        self.stage == AdapterAcceptanceStage::Enforced
    }

    pub fn requires_input_report(&self, report_schema_name: &str) -> bool {
        self.required_input_reports
            .iter()
            .any(|required| required == report_schema_name)
    }

    pub fn requires_attribution_rule(&self, rule: &str) -> bool {
        self.required_attribution_rules
            .iter()
            .any(|required| required == rule)
    }

    pub fn requires_effect_report_field(&self, field: &str) -> bool {
        self.required_effect_report_fields
            .iter()
            .any(|required| required == field)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterReportEmissionPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_names: Vec<String>,
    pub required_future_events: Vec<String>,
    pub required_report_fields: Vec<String>,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl AdapterReportEmissionPlan {
    pub fn apple_silicon_development_effect(stage: AdapterAcceptanceStage) -> Self {
        Self {
            name: "apple-silicon-development-effect-emission".to_owned(),
            stage,
            report_schema_names: vec![
                "model_worker_v1".to_owned(),
                "root_adapter_attribution_report_v1".to_owned(),
                "ledger_gate_report_v1".to_owned(),
                "report_freshness_report_v1".to_owned(),
                "remote_runtime_acceleration_report_v1".to_owned(),
                "model_worker_gate_report_v1".to_owned(),
                "worker_root_failure_consistency_report_v1".to_owned(),
                "model_pool_development_attribution_report_v1".to_owned(),
                "model_pool_budget_fairness_report_v1".to_owned(),
                "model_pool_development_window_report_v1".to_owned(),
                "apple_silicon_baseline_comparison_report_v1".to_owned(),
                "apple_silicon_development_effect_report_v1".to_owned(),
                "context_rot_report_v1".to_owned(),
                "context_rot_trend_report_v1".to_owned(),
                "context_rot_remediation_report_v1".to_owned(),
                "steam_case_matrix_report_v1".to_owned(),
                "validation_command_coverage_report_v1".to_owned(),
                "self_evolution_continuity_report_v1".to_owned(),
                "self_evolution_regression_report_v1".to_owned(),
                "rollback_resume_report_v1".to_owned(),
                "readiness_next_round_v1".to_owned(),
                "rollback_report_v1".to_owned(),
                "adapter_closure_report_v1".to_owned(),
                "adapter_report_emission_report_v1".to_owned(),
                "adapter_future_event_coverage_report_v1".to_owned(),
                "report_bundle_gate_report_v1".to_owned(),
                "run_mode_report_refresh_acceptance_report_v1".to_owned(),
                "adapter_promotion_window_report_v1".to_owned(),
                "self_evolution_unattended_prerequisites_report_v1".to_owned(),
            ],
            required_future_events: vec![
                "worker_output_fingerprint".to_owned(),
                "worker_noise_score".to_owned(),
                "worker_primary_wait_ms".to_owned(),
                "worker_failure_kind".to_owned(),
                "backend_8686_reachable".to_owned(),
                "prompt_gate_blocked".to_owned(),
                "final_json_present".to_owned(),
                "runtime_model_present".to_owned(),
                "runtime_tokens".to_owned(),
                "business_cycle_passed".to_owned(),
                "baseline_12b_feedback_applied".to_owned(),
                "baseline_12b_latency_ms".to_owned(),
                "baseline_12b_runtime_tokens".to_owned(),
                "baseline_12b_success".to_owned(),
                "baseline_12b_validation_passed".to_owned(),
                "context_rot_noisy_records".to_owned(),
                "context_rot_noise_penalty".to_owned(),
                "context_rot_duplicate_outputs".to_owned(),
                "context_rot_quarantine_candidates".to_owned(),
                "context_rot_trend_window_rounds".to_owned(),
                "context_rot_consecutive_noisy_rounds".to_owned(),
                "context_rot_consecutive_duplicate_rounds".to_owned(),
                "context_rot_remediation_applied".to_owned(),
                "context_rot_clean_gist_backfilled".to_owned(),
                "context_rot_legacy_metadata_repaired".to_owned(),
                "steam_case_id".to_owned(),
                "steam_case_endpoint".to_owned(),
                "steam_case_kind".to_owned(),
                "validation_command_phase".to_owned(),
                "validation_command_line".to_owned(),
                "validation_command_status_code".to_owned(),
                "validation_output_tail".to_owned(),
            ],
            required_report_fields: vec![
                "apple_silicon_effect.worker_ids".to_owned(),
                "apple_silicon_effect.roles".to_owned(),
                "apple_silicon_effect.latency_ms".to_owned(),
                "apple_silicon_effect.runtime_tokens".to_owned(),
                "apple_silicon_effect.success".to_owned(),
                "apple_silicon_effect.feedback_applied".to_owned(),
                "apple_silicon_effect.validation_checked".to_owned(),
                "apple_silicon_effect.validation_passed".to_owned(),
                "apple_silicon_effect.duplicate_output".to_owned(),
                "apple_silicon_effect.noisy_output".to_owned(),
                "apple_silicon_effect.blocked_primary_12b".to_owned(),
                "apple_silicon_effect.failure_kinds".to_owned(),
                "apple_silicon_effect.worker_development_claim_allowed".to_owned(),
                "apple_silicon_effect.worker_claim_blockers".to_owned(),
                "apple_silicon_effect.operational_readiness_failure_kind".to_owned(),
                "apple_silicon_effect.model_quality_failure_allowed".to_owned(),
                "apple_silicon_effect.quality_failure_blocked_by_readiness_order".to_owned(),
                "apple_silicon_effect.operational_failure_counted_as_quality".to_owned(),
                "model_pool_attribution.worker_ids".to_owned(),
                "model_pool_attribution.roles".to_owned(),
                "model_pool_attribution.latency_ms".to_owned(),
                "model_pool_attribution.runtime_tokens".to_owned(),
                "model_pool_attribution.success".to_owned(),
                "model_pool_attribution.feedback_applied".to_owned(),
                "model_pool_attribution.validation_checked".to_owned(),
                "model_pool_attribution.validation_passed".to_owned(),
                "model_pool_attribution.duplicate_output".to_owned(),
                "model_pool_attribution.noisy_output".to_owned(),
                "model_pool_attribution.blocked_primary_12b".to_owned(),
                "model_pool_attribution.failure_kinds".to_owned(),
                "model_pool_attribution.worker_development_claim_allowed".to_owned(),
                "model_pool_attribution.worker_claim_blockers".to_owned(),
                "model_pool_attribution.chain_not_ready_count".to_owned(),
                "model_pool_attribution.model_unavailable_count".to_owned(),
                "model_pool_budget.roles".to_owned(),
                "model_pool_budget.workers_by_role".to_owned(),
                "model_pool_budget.successful_workers_by_role".to_owned(),
                "model_pool_budget.feedback_by_role".to_owned(),
                "model_pool_budget.runtime_tokens_by_role".to_owned(),
                "model_pool_budget.runtime_token_share_by_role".to_owned(),
                "model_pool_budget.dominant_runtime_token_roles".to_owned(),
                "model_pool_budget.latency_ms_by_role".to_owned(),
                "model_pool_budget.missing_required_roles".to_owned(),
                "model_pool_budget.total_runtime_tokens".to_owned(),
                "model_pool_budget.total_latency_ms".to_owned(),
                "model_pool_budget.max_role_runtime_token_share".to_owned(),
                "model_pool_budget.fairness_blocked".to_owned(),
                "model_pool_budget.allow_pool_expansion".to_owned(),
                "model_pool_budget.failure_reasons".to_owned(),
                "context_rot.noisy_records".to_owned(),
                "context_rot.max_noise_penalty".to_owned(),
                "context_rot.duplicate_outputs".to_owned(),
                "context_rot.gate_blocked".to_owned(),
                "context_rot_trend.latest_noisy_records".to_owned(),
                "context_rot_trend.latest_duplicate_outputs".to_owned(),
                "context_rot_trend.noisy_records_delta".to_owned(),
                "context_rot_trend.remediation_improved_noise".to_owned(),
                "context_rot_trend.remediation_improved_duplicates".to_owned(),
                "context_rot_trend.allow_unattended_continuation".to_owned(),
                "context_rot_remediation.quarantine_candidates".to_owned(),
                "context_rot_remediation.quarantined_records".to_owned(),
                "context_rot_remediation.clean_gists_backfilled".to_owned(),
                "context_rot_remediation.duplicate_outputs_removed".to_owned(),
                "context_rot_remediation.allow_experiment_rollout".to_owned(),
                "report_freshness.rounds".to_owned(),
                "report_freshness.ledger_lag".to_owned(),
                "report_freshness.stale".to_owned(),
                "report_freshness.gate_failures".to_owned(),
                "report_freshness.ledger_gate_blocked".to_owned(),
                "report_freshness.fresh".to_owned(),
                "report_freshness.allow_next_round".to_owned(),
                "remote_runtime.total_workers".to_owned(),
                "remote_runtime.healthy_workers".to_owned(),
                "remote_runtime.metal_workers".to_owned(),
                "remote_runtime.quality_model".to_owned(),
                "remote_runtime.all_workers_healthy".to_owned(),
                "remote_runtime.all_workers_metal".to_owned(),
                "remote_runtime.quality_model_present".to_owned(),
                "remote_runtime.acceleration_ready".to_owned(),
                "remote_runtime.failure_reasons".to_owned(),
                "run_mode_report_refresh.report_refresh_allowed".to_owned(),
                "run_mode_report_refresh.ledger_gate_allow_next_round".to_owned(),
                "run_mode_report_refresh.remote_runtime_acceleration_ready".to_owned(),
                "run_mode_report_refresh.report_bundle_complete".to_owned(),
                "run_mode_report_refresh.allow_next_round".to_owned(),
                "run_mode_report_refresh.failure_reasons".to_owned(),
                "validation_command.strict_coverage_requested".to_owned(),
                "validation_command.coverage_tooling_evidence".to_owned(),
                "validation_command.coverage_report_evidence".to_owned(),
                "validation_command.coverage_tooling_or_report_evidence_present".to_owned(),
                "ledger.allow_next_round".to_owned(),
                "rollback.resume_gate".to_owned(),
                "adapter_closure.allow_next_round".to_owned(),
            ],
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        false
    }

    pub fn report_index(&self, report_schema_name: &str) -> Option<usize> {
        self.report_schema_names
            .iter()
            .position(|name| name == report_schema_name)
    }

    pub fn emits_after(&self, report_schema_name: &str, prior_report_schema_name: &str) -> bool {
        match (
            self.report_index(report_schema_name),
            self.report_index(prior_report_schema_name),
        ) {
            (Some(report), Some(prior)) => prior < report,
            _ => false,
        }
    }

    pub fn requires_report_field(&self, field: &str) -> bool {
        self.required_report_fields
            .iter()
            .any(|required| required == field)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterFutureEventCoveragePlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_name: String,
    pub source_contracts: Vec<String>,
    pub required_future_events: Vec<String>,
    pub require_required_events_planned_before_enforcement: bool,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl AdapterFutureEventCoveragePlan {
    pub fn apple_silicon_contracts(stage: AdapterAcceptanceStage) -> Self {
        Self {
            name: "adapter-future-event-coverage".to_owned(),
            stage,
            report_schema_name: "adapter_future_event_coverage_report_v1".to_owned(),
            source_contracts: vec![
                "ModelWorkerLedgerAdapterPlan::evolution_loop_ledger".to_owned(),
                "AppleSiliconBaselineAdapterPlan::root_business_cycle_json".to_owned(),
                "RootBusinessCycleAdapterPlan::root_business_cycle_json".to_owned(),
                "ContextRotAcceptanceContract".to_owned(),
                "ContextRotTrendGate".to_owned(),
                "ContextRotRemediationGate".to_owned(),
                "SteamCaseCoverageGate".to_owned(),
                "ValidationCommandCoverageGate".to_owned(),
            ],
            required_future_events: vec![
                "worker_output_fingerprint".to_owned(),
                "worker_noise_score".to_owned(),
                "worker_primary_wait_ms".to_owned(),
                "worker_failure_kind".to_owned(),
                "backend_8686_reachable".to_owned(),
                "prompt_gate_blocked".to_owned(),
                "final_json_present".to_owned(),
                "runtime_model_present".to_owned(),
                "runtime_tokens".to_owned(),
                "business_cycle_passed".to_owned(),
                "baseline_12b_feedback_applied".to_owned(),
                "baseline_12b_latency_ms".to_owned(),
                "baseline_12b_runtime_tokens".to_owned(),
                "baseline_12b_success".to_owned(),
                "baseline_12b_validation_passed".to_owned(),
                "context_rot_noisy_records".to_owned(),
                "context_rot_noise_penalty".to_owned(),
                "context_rot_duplicate_outputs".to_owned(),
                "context_rot_quarantine_candidates".to_owned(),
                "context_rot_trend_window_rounds".to_owned(),
                "context_rot_consecutive_noisy_rounds".to_owned(),
                "context_rot_consecutive_duplicate_rounds".to_owned(),
                "context_rot_remediation_applied".to_owned(),
                "context_rot_clean_gist_backfilled".to_owned(),
                "context_rot_legacy_metadata_repaired".to_owned(),
                "steam_case_id".to_owned(),
                "steam_case_endpoint".to_owned(),
                "steam_case_kind".to_owned(),
                "validation_command_phase".to_owned(),
                "validation_command_line".to_owned(),
                "validation_command_status_code".to_owned(),
                "validation_output_tail".to_owned(),
            ],
            require_required_events_planned_before_enforcement: stage
                == AdapterAcceptanceStage::Enforced,
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        self.stage == AdapterAcceptanceStage::Enforced
    }

    pub fn requires_future_event(&self, event: &str) -> bool {
        self.required_future_events
            .iter()
            .any(|required| required == event)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContextRotAcceptancePlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub audit_plan: ExperienceAuditPlan,
    pub report_schema_name: String,
    pub blocks_experiment_rollout: bool,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl ContextRotAcceptancePlan {
    pub fn experience_audit(name: impl Into<String>, stage: AdapterAcceptanceStage) -> Self {
        Self {
            name: name.into(),
            stage,
            audit_plan: ExperienceAuditPlan::cleanup_audit(500),
            report_schema_name: "context_rot_report_v1".to_owned(),
            blocks_experiment_rollout: stage == AdapterAcceptanceStage::Enforced,
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn with_audit_plan(mut self, audit_plan: ExperienceAuditPlan) -> Self {
        self.audit_plan = audit_plan;
        self
    }

    pub fn may_block_current_runner(&self) -> bool {
        self.stage == AdapterAcceptanceStage::Enforced
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContextRotTrendWindowPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_name: String,
    pub min_window_rounds: usize,
    pub max_consecutive_noisy_rounds: usize,
    pub max_consecutive_duplicate_rounds: usize,
    pub require_remediation_improves_noise: bool,
    pub require_remediation_improves_duplicates: bool,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl ContextRotTrendWindowPlan {
    pub fn trend_window(name: impl Into<String>, stage: AdapterAcceptanceStage) -> Self {
        Self {
            name: name.into(),
            stage,
            report_schema_name: "context_rot_trend_report_v1".to_owned(),
            min_window_rounds: 3,
            max_consecutive_noisy_rounds: 1,
            max_consecutive_duplicate_rounds: 0,
            require_remediation_improves_noise: stage == AdapterAcceptanceStage::Enforced,
            require_remediation_improves_duplicates: stage == AdapterAcceptanceStage::Enforced,
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        self.stage == AdapterAcceptanceStage::Enforced
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterContextRotTrendBoundaryPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub eval_entrypoints: Vec<String>,
    pub allowed_inputs: Vec<String>,
    pub produced_outputs: Vec<String>,
    pub forbidden_capabilities: Vec<String>,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl AdapterContextRotTrendBoundaryPlan {
    pub fn context_rot_trend_boundary(
        name: impl Into<String>,
        stage: AdapterAcceptanceStage,
    ) -> Self {
        Self {
            name: name.into(),
            stage,
            eval_entrypoints: vec![
                "ContextRotTrendWindowSummary::from_points".to_owned(),
                "ContextRotTrendGate::evaluate".to_owned(),
                "ContextRotTrendReport::from_points_and_gate".to_owned(),
            ],
            allowed_inputs: vec![
                "ContextRotSignal".to_owned(),
                "ContextRotTrendPoint".to_owned(),
                "ContextRotTrendGate".to_owned(),
            ],
            produced_outputs: vec![
                "ContextRotTrendWindowSummary".to_owned(),
                "GateDecision".to_owned(),
                "ContextRotTrendReport".to_owned(),
            ],
            forbidden_capabilities: vec![
                "jsonl_io".to_owned(),
                "filesystem_scan".to_owned(),
                "quarantine_action_execution".to_owned(),
                "clean_gist_write".to_owned(),
                "duplicate_output_delete".to_owned(),
                "http_sse".to_owned(),
                "process_spawn".to_owned(),
                "validation_command_spawn".to_owned(),
                "model_call".to_owned(),
                "runner_state".to_owned(),
            ],
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        false
    }

    pub fn exposes_entrypoint(&self, entrypoint: &str) -> bool {
        self.eval_entrypoints
            .iter()
            .any(|exposed| exposed == entrypoint)
    }

    pub fn allows_input(&self, input: &str) -> bool {
        self.allowed_inputs.iter().any(|allowed| allowed == input)
    }

    pub fn produces_output(&self, output: &str) -> bool {
        self.produced_outputs
            .iter()
            .any(|produced| produced == output)
    }

    pub fn forbids_capability(&self, capability: &str) -> bool {
        self.forbidden_capabilities
            .iter()
            .any(|forbidden| forbidden == capability)
    }

    pub fn stays_pure_data_boundary(&self) -> bool {
        self.exposes_entrypoint("ContextRotTrendWindowSummary::from_points")
            && self.exposes_entrypoint("ContextRotTrendGate::evaluate")
            && self.exposes_entrypoint("ContextRotTrendReport::from_points_and_gate")
            && self.allows_input("ContextRotSignal")
            && self.allows_input("ContextRotTrendPoint")
            && self.allows_input("ContextRotTrendGate")
            && !self.allows_input("FilesystemScanner")
            && !self.allows_input("JsonlReader")
            && self.produces_output("ContextRotTrendWindowSummary")
            && self.produces_output("GateDecision")
            && self.produces_output("ContextRotTrendReport")
            && [
                "jsonl_io",
                "filesystem_scan",
                "quarantine_action_execution",
                "clean_gist_write",
                "duplicate_output_delete",
                "http_sse",
                "process_spawn",
                "validation_command_spawn",
                "model_call",
                "runner_state",
            ]
            .iter()
            .all(|capability| self.forbids_capability(capability))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextRotRemediationPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_name: String,
    pub require_quarantine_complete: bool,
    pub require_legacy_metadata_repaired: bool,
    pub require_clean_gist_backfilled: bool,
    pub require_duplicates_removed: bool,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl ContextRotRemediationPlan {
    pub fn remediation(name: impl Into<String>, stage: AdapterAcceptanceStage) -> Self {
        Self {
            name: name.into(),
            stage,
            report_schema_name: "context_rot_remediation_report_v1".to_owned(),
            require_quarantine_complete: stage == AdapterAcceptanceStage::Enforced,
            require_legacy_metadata_repaired: stage == AdapterAcceptanceStage::Enforced,
            require_clean_gist_backfilled: stage == AdapterAcceptanceStage::Enforced,
            require_duplicates_removed: stage == AdapterAcceptanceStage::Enforced,
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        self.stage == AdapterAcceptanceStage::Enforced
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterContextRotRemediationBoundaryPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub eval_entrypoint: String,
    pub allowed_inputs: Vec<String>,
    pub produced_outputs: Vec<String>,
    pub forbidden_capabilities: Vec<String>,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl AdapterContextRotRemediationBoundaryPlan {
    pub fn context_rot_remediation_boundary(
        name: impl Into<String>,
        stage: AdapterAcceptanceStage,
    ) -> Self {
        Self {
            name: name.into(),
            stage,
            eval_entrypoint: "ContextRotRemediationReport::from_gate_and_evidence".to_owned(),
            allowed_inputs: vec![
                "ContextRotSignal".to_owned(),
                "ContextRotRemediationEvidence".to_owned(),
                "ContextRotRemediationGate".to_owned(),
            ],
            produced_outputs: vec!["ContextRotRemediationReport".to_owned()],
            forbidden_capabilities: vec![
                "jsonl_io".to_owned(),
                "filesystem_scan".to_owned(),
                "quarantine_action_execution".to_owned(),
                "clean_gist_write".to_owned(),
                "duplicate_output_delete".to_owned(),
                "http_sse".to_owned(),
                "process_spawn".to_owned(),
                "validation_command_spawn".to_owned(),
                "model_call".to_owned(),
                "runner_state".to_owned(),
            ],
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        false
    }

    pub fn allows_input(&self, input: &str) -> bool {
        self.allowed_inputs.iter().any(|allowed| allowed == input)
    }

    pub fn produces_output(&self, output: &str) -> bool {
        self.produced_outputs
            .iter()
            .any(|produced| produced == output)
    }

    pub fn forbids_capability(&self, capability: &str) -> bool {
        self.forbidden_capabilities
            .iter()
            .any(|forbidden| forbidden == capability)
    }

    pub fn stays_pure_data_boundary(&self) -> bool {
        self.eval_entrypoint == "ContextRotRemediationReport::from_gate_and_evidence"
            && self.allows_input("ContextRotSignal")
            && self.allows_input("ContextRotRemediationEvidence")
            && self.allows_input("ContextRotRemediationGate")
            && !self.allows_input("FilesystemScanner")
            && !self.allows_input("QuarantineActionExecutor")
            && self.produces_output("ContextRotRemediationReport")
            && [
                "jsonl_io",
                "filesystem_scan",
                "quarantine_action_execution",
                "clean_gist_write",
                "duplicate_output_delete",
                "http_sse",
                "process_spawn",
                "validation_command_spawn",
                "model_call",
                "runner_state",
            ]
            .iter()
            .all(|capability| self.forbids_capability(capability))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfEvolutionReadinessPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_name: String,
    pub required_gate_inputs: Vec<String>,
    pub report_only_inputs: Vec<String>,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl SelfEvolutionReadinessPlan {
    pub fn next_round(name: impl Into<String>, stage: AdapterAcceptanceStage) -> Self {
        Self {
            name: name.into(),
            stage,
            report_schema_name: "readiness_next_round_v1".to_owned(),
            required_gate_inputs: vec![
                "ReportGate".to_owned(),
                "LedgerGateReport::gate_blocked".to_owned(),
                "ContextRotAcceptanceContract".to_owned(),
                "ContextRotReport::gate_blocked".to_owned(),
                "ContextRotTrendGate".to_owned(),
                "ContextRotTrendReport::trend_blocked".to_owned(),
                "ContextRotRemediationGate".to_owned(),
                "ContextRotRemediationReport::remediation_blocked".to_owned(),
                "ModelPoolGate".to_owned(),
                "ExperimentRolloutGate".to_owned(),
                "ExperimentKillSwitchGate".to_owned(),
                "ExperimentExpansionSafetyGate".to_owned(),
                "ExperimentExpansionSafetyReport::allow_experiment_expansion".to_owned(),
                "AdapterReportEmissionGate".to_owned(),
                "AdapterReportEmissionReport::field_coverage_passed".to_owned(),
                "AppleSiliconDevelopmentEffectGate".to_owned(),
                "RollbackResumeGate".to_owned(),
                "RollbackResumeReport::resume_blocked".to_owned(),
                "SteamCaseCoverageGate".to_owned(),
                "SteamCaseCoverageReport::coverage_blocked".to_owned(),
                "ValidationCommandCoverageGate".to_owned(),
                "ValidationCommandCoverageReport::coverage_blocked".to_owned(),
                "RootAdapterFailureKind".to_owned(),
            ],
            report_only_inputs: vec![
                "AdviceContinuationReport::allow_continuation".to_owned(),
                "AdviceContinuationReport::continuation_blocked".to_owned(),
            ],
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        self.stage == AdapterAcceptanceStage::Enforced
    }

    pub fn uses_report_only_input(&self, input: &str) -> bool {
        self.report_only_inputs
            .iter()
            .any(|report_only| report_only == input)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyLedgerReplayPlan {
    pub name: String,
    pub ledger_glob: String,
    pub report_schema_name: String,
    pub required_existing_fields: Vec<String>,
    pub optional_additive_reports: Vec<String>,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl LegacyLedgerReplayPlan {
    pub fn evolution_loop_jsonl(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ledger_glob: r"target\evolution\*.jsonl".to_owned(),
            report_schema_name: "legacy_ledger_replay_report_v1".to_owned(),
            required_existing_fields: vec![
                "round".to_owned(),
                "success".to_owned(),
                "runtime_tokens".to_owned(),
                "runtime_model".to_owned(),
                "validation_checked".to_owned(),
                "validation_passed".to_owned(),
                "feedback_applied".to_owned(),
            ],
            optional_additive_reports: vec![
                "model_worker_v1".to_owned(),
                "model_worker_gate_report_v1".to_owned(),
                "worker_root_failure_consistency_report_v1".to_owned(),
                "model_pool_budget_fairness_report_v1".to_owned(),
                "model_pool_development_attribution_report_v1".to_owned(),
                "model_pool_development_window_report_v1".to_owned(),
                "apple_silicon_baseline_comparison_report_v1".to_owned(),
                "apple_silicon_development_effect_report_v1".to_owned(),
                "adapter_report_emission_report_v1".to_owned(),
                "adapter_future_event_coverage_report_v1".to_owned(),
                "ledger_gate_report_v1".to_owned(),
                "report_freshness_report_v1".to_owned(),
                "remote_runtime_acceleration_report_v1".to_owned(),
                "run_mode_report_refresh_acceptance_report_v1".to_owned(),
                "context_rot_report_v1".to_owned(),
                "context_rot_trend_report_v1".to_owned(),
                "context_rot_remediation_report_v1".to_owned(),
                "experiment_rollout_report_v1".to_owned(),
                "experiment_kill_switch_report_v1".to_owned(),
                "experiment_expansion_safety_report_v1".to_owned(),
                "experiment_switch_matrix_report_v1".to_owned(),
                "root_adapter_attribution_report_v1".to_owned(),
                "adapter_fixture_contract_report_v1".to_owned(),
                "current_runner_compatibility_report_v1".to_owned(),
                "feedback_self_improve_report_v1".to_owned(),
                "self_evolution_continuity_report_v1".to_owned(),
                "self_evolution_regression_report_v1".to_owned(),
                "self_evolution_unattended_prerequisites_report_v1".to_owned(),
                "readiness_next_round_v1".to_owned(),
                "advice_continuation_report_v1".to_owned(),
                "steam_round_report_v1".to_owned(),
                "steam_case_matrix_report_v1".to_owned(),
                "validation_command_coverage_report_v1".to_owned(),
                "rollback_report_v1".to_owned(),
                "adapter_closure_report_v1".to_owned(),
                "rollback_drill_matrix_report_v1".to_owned(),
                "adapter_handoff_report_v1".to_owned(),
                "report_bundle_gate_report_v1".to_owned(),
                "schema_drift_report_v1".to_owned(),
                "adapter_promotion_window_report_v1".to_owned(),
                "rollback_resume_report_v1".to_owned(),
            ],
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterLegacyLedgerReplayBoundaryPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub eval_entrypoints: Vec<String>,
    pub allowed_inputs: Vec<String>,
    pub produced_outputs: Vec<String>,
    pub forbidden_capabilities: Vec<String>,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl AdapterLegacyLedgerReplayBoundaryPlan {
    pub fn legacy_ledger_replay_boundary(
        name: impl Into<String>,
        stage: AdapterAcceptanceStage,
    ) -> Self {
        Self {
            name: name.into(),
            stage,
            eval_entrypoints: vec![
                "LegacyLedgerReplayEvidence::from_ledger_summary".to_owned(),
                "LegacyLedgerReplayEvidence::from_ledger_records".to_owned(),
                "LegacyLedgerReplayEvidence::from_ledger_summary_and_report_names".to_owned(),
                "LegacyLedgerReplayEvidence::from_ledger_records_and_report_names".to_owned(),
                "LegacyLedgerReplayEvidence::with_observed_additive_reports".to_owned(),
                "LegacyLedgerReplayCompatibility::evaluate_replay".to_owned(),
                "LegacyLedgerReplayReport::from_summary_and_contract".to_owned(),
                "LegacyLedgerReplayReport::from_records_and_contract".to_owned(),
            ],
            allowed_inputs: vec![
                "LedgerRecord".to_owned(),
                "LedgerSummary".to_owned(),
                "LegacyLedgerReplayCompatibility".to_owned(),
                "LegacyLedgerReplayEvidence".to_owned(),
                "GateDecision".to_owned(),
                "observed_report_schema_names".to_owned(),
            ],
            produced_outputs: vec![
                "LegacyLedgerReplayEvidence".to_owned(),
                "GateDecision".to_owned(),
                "LegacyLedgerReplayReport".to_owned(),
            ],
            forbidden_capabilities: vec![
                "jsonl_io".to_owned(),
                "file_io".to_owned(),
                "report_directory_scan".to_owned(),
                "http_sse".to_owned(),
                "process_spawn".to_owned(),
                "validation_command_spawn".to_owned(),
                "model_call".to_owned(),
                "runner_state".to_owned(),
            ],
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        false
    }

    pub fn exposes_entrypoint(&self, entrypoint: &str) -> bool {
        self.eval_entrypoints
            .iter()
            .any(|exposed| exposed == entrypoint)
    }

    pub fn allows_input(&self, input: &str) -> bool {
        self.allowed_inputs.iter().any(|allowed| allowed == input)
    }

    pub fn produces_output(&self, output: &str) -> bool {
        self.produced_outputs
            .iter()
            .any(|produced| produced == output)
    }

    pub fn forbids_capability(&self, capability: &str) -> bool {
        self.forbidden_capabilities
            .iter()
            .any(|forbidden| forbidden == capability)
    }

    pub fn stays_pure_data_boundary(&self) -> bool {
        self.exposes_entrypoint("LegacyLedgerReplayEvidence::from_ledger_summary")
            && self.exposes_entrypoint("LegacyLedgerReplayEvidence::from_ledger_records")
            && self.exposes_entrypoint(
                "LegacyLedgerReplayEvidence::from_ledger_summary_and_report_names",
            )
            && self.exposes_entrypoint(
                "LegacyLedgerReplayEvidence::from_ledger_records_and_report_names",
            )
            && self.exposes_entrypoint("LegacyLedgerReplayEvidence::with_observed_additive_reports")
            && self.exposes_entrypoint("LegacyLedgerReplayCompatibility::evaluate_replay")
            && self.exposes_entrypoint("LegacyLedgerReplayReport::from_summary_and_contract")
            && self.exposes_entrypoint("LegacyLedgerReplayReport::from_records_and_contract")
            && self.allows_input("LedgerRecord")
            && self.allows_input("LedgerSummary")
            && self.allows_input("LegacyLedgerReplayCompatibility")
            && self.allows_input("LegacyLedgerReplayEvidence")
            && self.allows_input("GateDecision")
            && self.allows_input("observed_report_schema_names")
            && !self.allows_input("JsonlLedgerReader")
            && !self.allows_input("ReportDirectoryScanner")
            && !self.allows_input("EvolutionLoopRunner")
            && self.produces_output("LegacyLedgerReplayEvidence")
            && self.produces_output("GateDecision")
            && self.produces_output("LegacyLedgerReplayReport")
            && [
                "jsonl_io",
                "file_io",
                "report_directory_scan",
                "http_sse",
                "process_spawn",
                "validation_command_spawn",
                "model_call",
                "runner_state",
            ]
            .iter()
            .all(|capability| self.forbids_capability(capability))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SteamRoundAcceptancePlan {
    pub name: String,
    pub case: SmartSteamCase,
    pub report_schema_name: String,
    pub require_stream_continuity: bool,
    pub validation_plan: VerificationPlan,
    pub readiness_plan: SelfEvolutionReadinessPlan,
    pub preserves_legacy_runner: bool,
}

impl SteamRoundAcceptancePlan {
    pub fn business_cycle(
        name: impl Into<String>,
        case: SmartSteamCase,
        validation_plan: VerificationPlan,
        readiness_plan: SelfEvolutionReadinessPlan,
    ) -> Self {
        Self {
            name: name.into(),
            case,
            report_schema_name: "steam_round_report_v1".to_owned(),
            require_stream_continuity: true,
            validation_plan,
            readiness_plan,
            preserves_legacy_runner: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SteamCaseMatrixPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_name: String,
    pub min_cases: usize,
    pub required_endpoint: String,
    pub required_case_kinds: Vec<String>,
    pub required_final_json_fields: Vec<String>,
    pub require_unique_case_ids: bool,
    pub require_stream_continuity: bool,
    pub require_validation_passed: bool,
    pub require_business_cycle_passed: bool,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl SteamCaseMatrixPlan {
    pub fn business_cycle_matrix(name: impl Into<String>, stage: AdapterAcceptanceStage) -> Self {
        Self {
            name: name.into(),
            stage,
            report_schema_name: "steam_case_matrix_report_v1".to_owned(),
            min_cases: 4,
            required_endpoint: "/v1/business-cycle-stream".to_owned(),
            required_case_kinds: vec![
                "planning".to_owned(),
                "validation".to_owned(),
                "rollback".to_owned(),
                "apple_silicon_model_pool".to_owned(),
            ],
            required_final_json_fields: vec![
                "business_cycle.passed".to_owned(),
                "business_cycle.feedback_applied".to_owned(),
                "generate.runtime_model".to_owned(),
                "generate.runtime_tokens".to_owned(),
                "validation.checked".to_owned(),
                "validation.passed".to_owned(),
                "self_improve.checked".to_owned(),
                "self_improve.passed".to_owned(),
            ],
            require_unique_case_ids: true,
            require_stream_continuity: true,
            require_validation_passed: stage == AdapterAcceptanceStage::Enforced,
            require_business_cycle_passed: stage == AdapterAcceptanceStage::Enforced,
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        self.stage == AdapterAcceptanceStage::Enforced
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationCommandCoveragePlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_name: String,
    pub required_phase: VerificationPhase,
    pub min_commands: usize,
    pub require_status_code: bool,
    pub require_output_tail: bool,
    pub require_all_commands_passed: bool,
    pub require_rust_check_passed: bool,
    pub require_strict_coverage_evidence: bool,
    pub required_report_fields: Vec<String>,
    pub failure_attribution_rules: Vec<String>,
    pub forbid_model_quality_failure_counting: bool,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl ValidationCommandCoveragePlan {
    pub fn post_round(name: impl Into<String>, stage: AdapterAcceptanceStage) -> Self {
        Self {
            name: name.into(),
            stage,
            report_schema_name: "validation_command_coverage_report_v1".to_owned(),
            required_phase: VerificationPhase::PostRound,
            min_commands: 1,
            require_status_code: true,
            require_output_tail: true,
            require_all_commands_passed: true,
            require_rust_check_passed: stage == AdapterAcceptanceStage::Enforced,
            require_strict_coverage_evidence: stage == AdapterAcceptanceStage::Enforced,
            required_report_fields: vec![
                "validation_command.strict_coverage_requested".to_owned(),
                "validation_command.coverage_tooling_evidence".to_owned(),
                "validation_command.coverage_report_evidence".to_owned(),
                "validation_command.coverage_tooling_or_report_evidence_present".to_owned(),
                "validation_command.coverage_failure_kind".to_owned(),
                "validation_command.model_quality_failure_counted".to_owned(),
                "validation_command.allow_next_round".to_owned(),
            ],
            failure_attribution_rules: vec![
                "validation_command_failure=validation_command_coverage".to_owned(),
                "validation_command_failure!=model_quality_failure".to_owned(),
            ],
            forbid_model_quality_failure_counting: true,
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        self.stage == AdapterAcceptanceStage::Enforced
    }

    pub fn requires_report_field(&self, field: &str) -> bool {
        self.required_report_fields
            .iter()
            .any(|required| required == field)
    }

    pub fn requires_failure_attribution_rule(&self, rule: &str) -> bool {
        self.failure_attribution_rules
            .iter()
            .any(|required| required == rule)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterValidationCommandCoverageBoundaryPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub eval_entrypoints: Vec<String>,
    pub allowed_inputs: Vec<String>,
    pub produced_outputs: Vec<String>,
    pub forbidden_capabilities: Vec<String>,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl AdapterValidationCommandCoverageBoundaryPlan {
    pub fn validation_command_coverage(
        name: impl Into<String>,
        stage: AdapterAcceptanceStage,
    ) -> Self {
        Self {
            name: name.into(),
            stage,
            eval_entrypoints: vec![
                "ValidationCommandCoverageEvidence::from_observations".to_owned(),
                "ValidationCommandCoverageEvidence::with_rust_check".to_owned(),
                "ValidationCommandCoverageEvidence::with_strict_coverage_requested".to_owned(),
                "ValidationCommandCoverageEvidence::with_strict_coverage_request_from_helper_stage"
                    .to_owned(),
                "ValidationCommandCoverageEvidence::with_coverage_tooling_evidence".to_owned(),
                "ValidationCommandCoverageEvidence::with_coverage_report_evidence".to_owned(),
                "ValidationCommandCoverageEvidence::strict_coverage_is_requested".to_owned(),
                "ValidationCommandCoverageEvidence::coverage_tooling_or_report_evidence_present"
                    .to_owned(),
                "ValidationCommandCoverageGate::evaluate".to_owned(),
                "ValidationCommandCoverageReport::from_gate_and_evidence".to_owned(),
                "ValidationCommandCoverageReport::from_observations_and_gate".to_owned(),
            ],
            allowed_inputs: vec![
                "ValidationObservation".to_owned(),
                "CommandOutcome".to_owned(),
                "VerificationPhase".to_owned(),
                "ValidationCommandCoverageEvidence".to_owned(),
                "ValidationCommandCoverageGate".to_owned(),
                "rust_check_checked".to_owned(),
                "rust_check_passed".to_owned(),
                "strict_coverage_requested".to_owned(),
                "coverage_tooling_evidence".to_owned(),
                "coverage_report_evidence".to_owned(),
                "HelperStageContractSummary".to_owned(),
            ],
            produced_outputs: vec![
                "ValidationCommandCoverageEvidence".to_owned(),
                "GateDecision".to_owned(),
                "ValidationCommandCoverageReport".to_owned(),
            ],
            forbidden_capabilities: vec![
                "jsonl_io".to_owned(),
                "file_io".to_owned(),
                "http_sse".to_owned(),
                "process_spawn".to_owned(),
                "validation_command_spawn".to_owned(),
                "validation_command_execution".to_owned(),
                "model_call".to_owned(),
                "runner_state".to_owned(),
            ],
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        false
    }

    pub fn exposes_entrypoint(&self, entrypoint: &str) -> bool {
        self.eval_entrypoints
            .iter()
            .any(|exposed| exposed == entrypoint)
    }

    pub fn allows_input(&self, input: &str) -> bool {
        self.allowed_inputs.iter().any(|allowed| allowed == input)
    }

    pub fn produces_output(&self, output: &str) -> bool {
        self.produced_outputs
            .iter()
            .any(|produced| produced == output)
    }

    pub fn forbids_capability(&self, capability: &str) -> bool {
        self.forbidden_capabilities
            .iter()
            .any(|forbidden| forbidden == capability)
    }

    pub fn stays_pure_data_boundary(&self) -> bool {
        self.exposes_entrypoint("ValidationCommandCoverageEvidence::from_observations")
            && self.exposes_entrypoint("ValidationCommandCoverageEvidence::with_rust_check")
            && self.exposes_entrypoint(
                "ValidationCommandCoverageEvidence::with_strict_coverage_requested",
            )
            && self.exposes_entrypoint(
                "ValidationCommandCoverageEvidence::with_strict_coverage_request_from_helper_stage",
            )
            && self.exposes_entrypoint(
                "ValidationCommandCoverageEvidence::with_coverage_tooling_evidence",
            )
            && self.exposes_entrypoint(
                "ValidationCommandCoverageEvidence::with_coverage_report_evidence",
            )
            && self.exposes_entrypoint(
                "ValidationCommandCoverageEvidence::strict_coverage_is_requested",
            )
            && self.exposes_entrypoint(
                "ValidationCommandCoverageEvidence::coverage_tooling_or_report_evidence_present",
            )
            && self.exposes_entrypoint("ValidationCommandCoverageGate::evaluate")
            && self.exposes_entrypoint("ValidationCommandCoverageReport::from_gate_and_evidence")
            && self
                .exposes_entrypoint("ValidationCommandCoverageReport::from_observations_and_gate")
            && self.allows_input("ValidationObservation")
            && self.allows_input("CommandOutcome")
            && self.allows_input("VerificationPhase")
            && self.allows_input("ValidationCommandCoverageEvidence")
            && self.allows_input("ValidationCommandCoverageGate")
            && self.allows_input("strict_coverage_requested")
            && self.allows_input("coverage_tooling_evidence")
            && self.allows_input("coverage_report_evidence")
            && self.allows_input("HelperStageContractSummary")
            && !self.allows_input("ValidationCommandExecutor")
            && !self.allows_input("EvolutionLoopRunner")
            && !self.allows_input("ProcessHandle")
            && self.produces_output("ValidationCommandCoverageEvidence")
            && self.produces_output("GateDecision")
            && self.produces_output("ValidationCommandCoverageReport")
            && [
                "jsonl_io",
                "file_io",
                "http_sse",
                "process_spawn",
                "validation_command_spawn",
                "validation_command_execution",
                "model_call",
                "runner_state",
            ]
            .iter()
            .all(|capability| self.forbids_capability(capability))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RollbackReportPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_name: String,
    pub require_resume_gate: bool,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl RollbackReportPlan {
    pub fn rollback_report(name: impl Into<String>, stage: AdapterAcceptanceStage) -> Self {
        Self {
            name: name.into(),
            stage,
            report_schema_name: "rollback_report_v1".to_owned(),
            require_resume_gate: stage == AdapterAcceptanceStage::Enforced,
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        self.stage == AdapterAcceptanceStage::Enforced
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterNormalizedEvidenceProjectionPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub eval_entrypoint: String,
    pub allowed_inputs: Vec<String>,
    pub produced_outputs: Vec<String>,
    pub forbidden_capabilities: Vec<String>,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl AdapterNormalizedEvidenceProjectionPlan {
    pub fn from_runner_projection(name: impl Into<String>, stage: AdapterAcceptanceStage) -> Self {
        Self {
            name: name.into(),
            stage,
            eval_entrypoint: "AdapterEvidenceProjection::from_normalized_evidence".to_owned(),
            allowed_inputs: vec![
                "LedgerRecord".to_owned(),
                "ReportGate".to_owned(),
                "HelperStageContractSummary".to_owned(),
                "ValidationCommandCoverageEvidence".to_owned(),
                "ValidationCommandCoverageGate".to_owned(),
            ],
            produced_outputs: vec![
                "LedgerSummary".to_owned(),
                "LedgerGateReport".to_owned(),
                "HelperStageContractSummary".to_owned(),
                "ValidationCommandCoverageReport".to_owned(),
            ],
            forbidden_capabilities: vec![
                "jsonl_io".to_owned(),
                "http_sse".to_owned(),
                "process_spawn".to_owned(),
                "validation_command_spawn".to_owned(),
                "model_call".to_owned(),
                "runner_state".to_owned(),
            ],
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        false
    }

    pub fn allows_input(&self, input: &str) -> bool {
        self.allowed_inputs.iter().any(|allowed| allowed == input)
    }

    pub fn produces_output(&self, output: &str) -> bool {
        self.produced_outputs
            .iter()
            .any(|produced| produced == output)
    }

    pub fn forbids_capability(&self, capability: &str) -> bool {
        self.forbidden_capabilities
            .iter()
            .any(|forbidden| forbidden == capability)
    }

    pub fn stays_pure_data_boundary(&self) -> bool {
        self.eval_entrypoint == "AdapterEvidenceProjection::from_normalized_evidence"
            && self.allows_input("LedgerRecord")
            && self.allows_input("ReportGate")
            && self.allows_input("HelperStageContractSummary")
            && self.allows_input("ValidationCommandCoverageEvidence")
            && self.allows_input("ValidationCommandCoverageGate")
            && !self.allows_input("EvolutionLoopRunner")
            && !self.allows_input("ValidationCommandExecutor")
            && self.produces_output("LedgerSummary")
            && self.produces_output("LedgerGateReport")
            && self.produces_output("HelperStageContractSummary")
            && self.produces_output("ValidationCommandCoverageReport")
            && [
                "jsonl_io",
                "http_sse",
                "process_spawn",
                "validation_command_spawn",
                "model_call",
                "runner_state",
            ]
            .iter()
            .all(|capability| self.forbids_capability(capability))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterReadinessReportsInputPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub eval_entrypoint: String,
    pub allowed_report_inputs: Vec<String>,
    pub produced_outputs: Vec<String>,
    pub forbidden_capabilities: Vec<String>,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl AdapterReadinessReportsInputPlan {
    pub fn readiness_reports(name: impl Into<String>, stage: AdapterAcceptanceStage) -> Self {
        Self {
            name: name.into(),
            stage,
            eval_entrypoint: "AdapterEvidenceProjection::readiness_snapshot_with_reports"
                .to_owned(),
            allowed_report_inputs: vec![
                "SteamRoundAcceptanceReport".to_owned(),
                "SteamCaseCoverageReport".to_owned(),
                "ContextRotReport".to_owned(),
                "ContextRotTrendReport".to_owned(),
                "ContextRotRemediationReport".to_owned(),
                "AdviceContinuationReport".to_owned(),
            ],
            produced_outputs: vec![
                "SelfEvolutionReadinessSnapshot".to_owned(),
                "SelfEvolutionReadinessReport".to_owned(),
                "RollbackReport".to_owned(),
                "AdapterClosureReport".to_owned(),
            ],
            forbidden_capabilities: vec![
                "jsonl_io".to_owned(),
                "http_sse".to_owned(),
                "process_spawn".to_owned(),
                "validation_command_spawn".to_owned(),
                "model_call".to_owned(),
                "runner_state".to_owned(),
            ],
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        false
    }

    pub fn allows_report_input(&self, input: &str) -> bool {
        self.allowed_report_inputs
            .iter()
            .any(|allowed| allowed == input)
    }

    pub fn produces_output(&self, output: &str) -> bool {
        self.produced_outputs
            .iter()
            .any(|produced| produced == output)
    }

    pub fn forbids_capability(&self, capability: &str) -> bool {
        self.forbidden_capabilities
            .iter()
            .any(|forbidden| forbidden == capability)
    }

    pub fn stays_pure_data_boundary(&self) -> bool {
        self.eval_entrypoint == "AdapterEvidenceProjection::readiness_snapshot_with_reports"
            && self.allows_report_input("SteamRoundAcceptanceReport")
            && self.allows_report_input("SteamCaseCoverageReport")
            && self.allows_report_input("ContextRotReport")
            && self.allows_report_input("ContextRotTrendReport")
            && self.allows_report_input("ContextRotRemediationReport")
            && !self.allows_report_input("SteamHttpClient")
            && !self.allows_report_input("FilesystemScanner")
            && !self.allows_report_input("EvolutionLoopRunner")
            && self.produces_output("SelfEvolutionReadinessSnapshot")
            && self.produces_output("SelfEvolutionReadinessReport")
            && self.produces_output("RollbackReport")
            && self.produces_output("AdapterClosureReport")
            && [
                "jsonl_io",
                "http_sse",
                "process_spawn",
                "validation_command_spawn",
                "model_call",
                "runner_state",
            ]
            .iter()
            .all(|capability| self.forbids_capability(capability))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterSteamEvidenceBoundaryPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub eval_entrypoints: Vec<String>,
    pub allowed_inputs: Vec<String>,
    pub produced_outputs: Vec<String>,
    pub forbidden_capabilities: Vec<String>,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl AdapterSteamEvidenceBoundaryPlan {
    pub fn steam_reports(name: impl Into<String>, stage: AdapterAcceptanceStage) -> Self {
        Self {
            name: name.into(),
            stage,
            eval_entrypoints: vec![
                "SteamRoundAcceptanceReport::from_evidence".to_owned(),
                "SteamCaseCoverageReport::from_rows_and_gate".to_owned(),
                "SteamCaseCoverageReport::from_gate_and_evidence".to_owned(),
            ],
            allowed_inputs: vec![
                "StreamContinuityCheck".to_owned(),
                "ValidationObservation".to_owned(),
                "LedgerRecord".to_owned(),
                "SelfEvolutionReadinessSnapshot".to_owned(),
                "SteamRoundAcceptanceEvidence".to_owned(),
                "SteamRoundAcceptanceGate".to_owned(),
                "SteamCaseCoverageRow".to_owned(),
                "SteamCaseCoverageEvidence".to_owned(),
                "SteamCaseCoverageGate".to_owned(),
            ],
            produced_outputs: vec![
                "SteamRoundAcceptanceReport".to_owned(),
                "SteamCaseCoverageReport".to_owned(),
            ],
            forbidden_capabilities: vec![
                "jsonl_io".to_owned(),
                "http_sse".to_owned(),
                "steam_http_execution".to_owned(),
                "stream_process_execution".to_owned(),
                "process_spawn".to_owned(),
                "validation_command_spawn".to_owned(),
                "model_call".to_owned(),
                "runner_state".to_owned(),
            ],
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        false
    }

    pub fn exposes_entrypoint(&self, entrypoint: &str) -> bool {
        self.eval_entrypoints
            .iter()
            .any(|exposed| exposed == entrypoint)
    }

    pub fn allows_input(&self, input: &str) -> bool {
        self.allowed_inputs.iter().any(|allowed| allowed == input)
    }

    pub fn produces_output(&self, output: &str) -> bool {
        self.produced_outputs
            .iter()
            .any(|produced| produced == output)
    }

    pub fn forbids_capability(&self, capability: &str) -> bool {
        self.forbidden_capabilities
            .iter()
            .any(|forbidden| forbidden == capability)
    }

    pub fn stays_pure_data_boundary(&self) -> bool {
        self.exposes_entrypoint("SteamRoundAcceptanceReport::from_evidence")
            && self.exposes_entrypoint("SteamCaseCoverageReport::from_rows_and_gate")
            && self.exposes_entrypoint("SteamCaseCoverageReport::from_gate_and_evidence")
            && self.allows_input("StreamContinuityCheck")
            && self.allows_input("ValidationObservation")
            && self.allows_input("LedgerRecord")
            && self.allows_input("SelfEvolutionReadinessSnapshot")
            && self.allows_input("SteamRoundAcceptanceEvidence")
            && self.allows_input("SteamRoundAcceptanceGate")
            && self.allows_input("SteamCaseCoverageRow")
            && self.allows_input("SteamCaseCoverageEvidence")
            && self.allows_input("SteamCaseCoverageGate")
            && !self.allows_input("SteamHttpClient")
            && !self.allows_input("EvolutionLoopRunner")
            && !self.allows_input("StreamProcess")
            && self.produces_output("SteamRoundAcceptanceReport")
            && self.produces_output("SteamCaseCoverageReport")
            && [
                "jsonl_io",
                "http_sse",
                "steam_http_execution",
                "stream_process_execution",
                "process_spawn",
                "validation_command_spawn",
                "model_call",
                "runner_state",
            ]
            .iter()
            .all(|capability| self.forbids_capability(capability))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterClosurePureDataPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_name: String,
    pub eval_entrypoints: Vec<String>,
    pub allowed_inputs: Vec<String>,
    pub required_input_reports: Vec<String>,
    pub forbidden_capabilities: Vec<String>,
    pub required_report_fields: Vec<String>,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl AdapterClosurePureDataPlan {
    pub fn adapter_closure(name: impl Into<String>, stage: AdapterAcceptanceStage) -> Self {
        Self {
            name: name.into(),
            stage,
            report_schema_name: "adapter_closure_report_v1".to_owned(),
            eval_entrypoints: vec![
                "AdapterEvidenceProjection::closure_report_with_reports".to_owned(),
                "AdapterClosureReport::from_reports".to_owned(),
                "RollbackReport::from_readiness_report".to_owned(),
            ],
            allowed_inputs: vec![
                "LedgerRecord".to_owned(),
                "ReportGate".to_owned(),
                "HelperStageContractSummary".to_owned(),
                "ValidationCommandCoverageEvidence".to_owned(),
                "ValidationCommandCoverageGate".to_owned(),
                "AdapterReadinessReports".to_owned(),
                "RootAdapterFailureKind".to_owned(),
                "LedgerGateReport".to_owned(),
                "ValidationCommandCoverageReport".to_owned(),
                "SelfEvolutionReadinessReport".to_owned(),
                "helper_stage_useful".to_owned(),
            ],
            required_input_reports: vec![
                "ledger_gate_report_v1".to_owned(),
                "validation_command_coverage_report_v1".to_owned(),
                "readiness_next_round_v1".to_owned(),
                "rollback_report_v1".to_owned(),
            ],
            forbidden_capabilities: vec![
                "jsonl_io".to_owned(),
                "http_sse".to_owned(),
                "process_spawn".to_owned(),
                "validation_command_spawn".to_owned(),
                "model_call".to_owned(),
                "runner_state".to_owned(),
            ],
            required_report_fields: vec![
                "adapter_closure.stage".to_owned(),
                "adapter_closure.helper_stage_useful".to_owned(),
                "adapter_closure.ledger_gate_blocked".to_owned(),
                "adapter_closure.validation_command_coverage_blocked".to_owned(),
                "adapter_closure.readiness_can_schedule_next_round".to_owned(),
                "adapter_closure.rollback_required".to_owned(),
                "adapter_closure.rollback_resume_gate".to_owned(),
                "adapter_closure.allow_next_round".to_owned(),
            ],
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        false
    }

    pub fn exposes_entrypoint(&self, entrypoint: &str) -> bool {
        self.eval_entrypoints
            .iter()
            .any(|exposed| exposed == entrypoint)
    }

    pub fn allows_input(&self, input: &str) -> bool {
        self.allowed_inputs.iter().any(|allowed| allowed == input)
    }

    pub fn requires_input_report(&self, report_schema_name: &str) -> bool {
        self.required_input_reports
            .iter()
            .any(|required| required == report_schema_name)
    }

    pub fn forbids_capability(&self, capability: &str) -> bool {
        self.forbidden_capabilities
            .iter()
            .any(|forbidden| forbidden == capability)
    }

    pub fn requires_report_field(&self, field: &str) -> bool {
        self.required_report_fields
            .iter()
            .any(|required| required == field)
    }

    pub fn stays_pure_data_boundary(&self) -> bool {
        self.report_schema_name == "adapter_closure_report_v1"
            && self.exposes_entrypoint("AdapterEvidenceProjection::closure_report_with_reports")
            && self.exposes_entrypoint("AdapterClosureReport::from_reports")
            && self.exposes_entrypoint("RollbackReport::from_readiness_report")
            && self.allows_input("LedgerRecord")
            && self.allows_input("ReportGate")
            && self.allows_input("HelperStageContractSummary")
            && self.allows_input("ValidationCommandCoverageEvidence")
            && self.allows_input("ValidationCommandCoverageGate")
            && self.allows_input("AdapterReadinessReports")
            && self.allows_input("RootAdapterFailureKind")
            && self.allows_input("LedgerGateReport")
            && self.allows_input("ValidationCommandCoverageReport")
            && self.allows_input("SelfEvolutionReadinessReport")
            && self.allows_input("helper_stage_useful")
            && !self.allows_input("EvolutionLoopRunner")
            && !self.allows_input("JsonlLedgerReader")
            && !self.allows_input("ValidationCommandExecutor")
            && !self.allows_input("RollbackReport")
            && self.requires_input_report("ledger_gate_report_v1")
            && self.requires_input_report("validation_command_coverage_report_v1")
            && self.requires_input_report("readiness_next_round_v1")
            && self.requires_input_report("rollback_report_v1")
            && self.requires_report_field("adapter_closure.stage")
            && self.requires_report_field("adapter_closure.helper_stage_useful")
            && self.requires_report_field("adapter_closure.ledger_gate_blocked")
            && self.requires_report_field("adapter_closure.validation_command_coverage_blocked")
            && self.requires_report_field("adapter_closure.readiness_can_schedule_next_round")
            && self.requires_report_field("adapter_closure.rollback_required")
            && self.requires_report_field("adapter_closure.rollback_resume_gate")
            && self.requires_report_field("adapter_closure.allow_next_round")
            && [
                "jsonl_io",
                "http_sse",
                "process_spawn",
                "validation_command_spawn",
                "model_call",
                "runner_state",
            ]
            .iter()
            .all(|capability| self.forbids_capability(capability))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterClosureSchemaDocumentPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_name: String,
    pub schema_document_source: String,
    pub required_entrypoints: Vec<String>,
    pub required_document_fields: Vec<String>,
    pub required_report_only_fields: Vec<String>,
    pub required_enforced_fields: Vec<String>,
    pub required_allowed_inputs: Vec<String>,
    pub required_boundary_sources: Vec<String>,
    pub forbidden_capabilities: Vec<String>,
    pub require_emission_field_coverage: bool,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl AdapterClosureSchemaDocumentPlan {
    pub fn adapter_closure_schema_document(
        name: impl Into<String>,
        stage: AdapterAcceptanceStage,
    ) -> Self {
        Self {
            name: name.into(),
            stage,
            report_schema_name: "adapter_closure_report_v1".to_owned(),
            schema_document_source: "AdapterClosurePureDataContract::schema_document".to_owned(),
            required_entrypoints: vec![
                "AdapterEvidenceProjection::closure_report_with_reports".to_owned(),
                "AdapterClosureReport::from_reports".to_owned(),
                "RollbackReport::from_readiness_report".to_owned(),
            ],
            required_document_fields: vec![
                "adapter_closure.stage".to_owned(),
                "adapter_closure.helper_stage_useful".to_owned(),
                "adapter_closure.ledger_gate_blocked".to_owned(),
                "adapter_closure.validation_command_coverage_blocked".to_owned(),
                "adapter_closure.readiness_can_schedule_next_round".to_owned(),
                "adapter_closure.rollback_required".to_owned(),
                "adapter_closure.rollback_resume_gate".to_owned(),
                "adapter_closure.allow_next_round".to_owned(),
            ],
            required_report_only_fields: vec![
                "adapter_closure.stage".to_owned(),
                "adapter_closure.helper_stage_useful".to_owned(),
                "adapter_closure.ledger_gate_blocked".to_owned(),
                "adapter_closure.validation_command_coverage_blocked".to_owned(),
            ],
            required_enforced_fields: vec![
                "adapter_closure.readiness_can_schedule_next_round".to_owned(),
                "adapter_closure.rollback_required".to_owned(),
                "adapter_closure.rollback_resume_gate".to_owned(),
                "adapter_closure.allow_next_round".to_owned(),
            ],
            required_allowed_inputs: vec![
                "LedgerRecord".to_owned(),
                "ReportGate".to_owned(),
                "HelperStageContractSummary".to_owned(),
                "ValidationCommandCoverageEvidence".to_owned(),
                "ValidationCommandCoverageGate".to_owned(),
                "AdapterReadinessReports".to_owned(),
                "RootAdapterFailureKind".to_owned(),
                "LedgerGateReport".to_owned(),
                "ValidationCommandCoverageReport".to_owned(),
                "SelfEvolutionReadinessReport".to_owned(),
                "helper_stage_useful".to_owned(),
            ],
            required_boundary_sources: vec![
                "AdapterClosureReportSchema::adapter_closure_v1".to_owned(),
                "AdapterReportEmissionPlan::required_report_fields".to_owned(),
                "AdapterClosurePureDataContract::adapter_closure_v1".to_owned(),
            ],
            forbidden_capabilities: vec![
                "jsonl_io".to_owned(),
                "http_sse".to_owned(),
                "process_spawn".to_owned(),
                "validation_command_spawn".to_owned(),
                "model_call".to_owned(),
                "runner_state".to_owned(),
            ],
            require_emission_field_coverage: stage == AdapterAcceptanceStage::Enforced,
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        false
    }

    pub fn requires_document_field(&self, field: &str) -> bool {
        self.required_document_fields
            .iter()
            .any(|required| required == field)
    }

    pub fn requires_entrypoint(&self, entrypoint: &str) -> bool {
        self.required_entrypoints
            .iter()
            .any(|required| required == entrypoint)
    }

    pub fn requires_boundary_source(&self, source: &str) -> bool {
        self.required_boundary_sources
            .iter()
            .any(|required| required == source)
    }

    pub fn requires_allowed_input(&self, input: &str) -> bool {
        self.required_allowed_inputs
            .iter()
            .any(|required| required == input)
    }

    pub fn forbids_capability(&self, capability: &str) -> bool {
        self.forbidden_capabilities
            .iter()
            .any(|forbidden| forbidden == capability)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RollbackDrillMatrixReportPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_name: String,
    pub require_all_root_adapter_failure_kinds: bool,
    pub require_stable_rollback_reasons: bool,
    pub require_stable_resume_gates: bool,
    pub require_actions_for_required_rollbacks: bool,
    pub forbid_clean_case_rollback: bool,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl RollbackDrillMatrixReportPlan {
    pub fn rollback_drill_matrix(name: impl Into<String>, stage: AdapterAcceptanceStage) -> Self {
        let enforced = stage == AdapterAcceptanceStage::Enforced;
        Self {
            name: name.into(),
            stage,
            report_schema_name: "rollback_drill_matrix_report_v1".to_owned(),
            require_all_root_adapter_failure_kinds: enforced,
            require_stable_rollback_reasons: enforced,
            require_stable_resume_gates: enforced,
            require_actions_for_required_rollbacks: enforced,
            forbid_clean_case_rollback: enforced,
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        self.stage == AdapterAcceptanceStage::Enforced
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterRollbackDrillMatrixBoundaryPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub eval_entrypoints: Vec<String>,
    pub allowed_inputs: Vec<String>,
    pub produced_outputs: Vec<String>,
    pub forbidden_capabilities: Vec<String>,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl AdapterRollbackDrillMatrixBoundaryPlan {
    pub fn rollback_drill_matrix_boundary(
        name: impl Into<String>,
        stage: AdapterAcceptanceStage,
    ) -> Self {
        Self {
            name: name.into(),
            stage,
            eval_entrypoints: vec![
                "RollbackDrillCase::from_failure_kind".to_owned(),
                "RollbackDrillMatrixEvidence::root_adapter_policy_matrix".to_owned(),
                "RollbackDrillMatrixGate::evaluate".to_owned(),
                "RollbackDrillMatrixReport::from_gate_and_evidence".to_owned(),
            ],
            allowed_inputs: vec![
                "RootAdapterFailureKind".to_owned(),
                "RollbackDrillCase".to_owned(),
                "RollbackDrillMatrixEvidence".to_owned(),
                "RollbackDrillMatrixGate".to_owned(),
            ],
            produced_outputs: vec![
                "RollbackDrillCase".to_owned(),
                "RollbackDrillMatrixEvidence".to_owned(),
                "GateDecision".to_owned(),
                "RollbackDrillMatrixReport".to_owned(),
            ],
            forbidden_capabilities: vec![
                "jsonl_io".to_owned(),
                "http_sse".to_owned(),
                "process_spawn".to_owned(),
                "validation_command_spawn".to_owned(),
                "model_call".to_owned(),
                "rollback_action_execution".to_owned(),
                "resume_action_execution".to_owned(),
                "runner_state".to_owned(),
            ],
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        false
    }

    pub fn exposes_entrypoint(&self, entrypoint: &str) -> bool {
        self.eval_entrypoints
            .iter()
            .any(|exposed| exposed == entrypoint)
    }

    pub fn allows_input(&self, input: &str) -> bool {
        self.allowed_inputs.iter().any(|allowed| allowed == input)
    }

    pub fn produces_output(&self, output: &str) -> bool {
        self.produced_outputs
            .iter()
            .any(|produced| produced == output)
    }

    pub fn forbids_capability(&self, capability: &str) -> bool {
        self.forbidden_capabilities
            .iter()
            .any(|forbidden| forbidden == capability)
    }

    pub fn stays_pure_data_boundary(&self) -> bool {
        self.exposes_entrypoint("RollbackDrillCase::from_failure_kind")
            && self.exposes_entrypoint("RollbackDrillMatrixEvidence::root_adapter_policy_matrix")
            && self.exposes_entrypoint("RollbackDrillMatrixGate::evaluate")
            && self.exposes_entrypoint("RollbackDrillMatrixReport::from_gate_and_evidence")
            && self.allows_input("RootAdapterFailureKind")
            && self.allows_input("RollbackDrillCase")
            && self.allows_input("RollbackDrillMatrixEvidence")
            && self.allows_input("RollbackDrillMatrixGate")
            && !self.allows_input("RollbackActionExecutor")
            && !self.allows_input("ResumeActionExecutor")
            && !self.allows_input("EvolutionLoopRunner")
            && self.produces_output("RollbackDrillCase")
            && self.produces_output("RollbackDrillMatrixEvidence")
            && self.produces_output("GateDecision")
            && self.produces_output("RollbackDrillMatrixReport")
            && [
                "jsonl_io",
                "http_sse",
                "process_spawn",
                "validation_command_spawn",
                "model_call",
                "rollback_action_execution",
                "resume_action_execution",
                "runner_state",
            ]
            .iter()
            .all(|capability| self.forbids_capability(capability))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RollbackResumeReportPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_name: String,
    pub require_resume_evidence: bool,
    pub require_steam_case_matrix: bool,
    pub require_validation_command_coverage: bool,
    pub require_adapter_report_field_coverage: bool,
    pub require_operational_failures_not_quality: bool,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl RollbackResumeReportPlan {
    pub fn rollback_resume(name: impl Into<String>, stage: AdapterAcceptanceStage) -> Self {
        Self {
            name: name.into(),
            stage,
            report_schema_name: "rollback_resume_report_v1".to_owned(),
            require_resume_evidence: stage == AdapterAcceptanceStage::Enforced,
            require_steam_case_matrix: stage == AdapterAcceptanceStage::Enforced,
            require_validation_command_coverage: stage == AdapterAcceptanceStage::Enforced,
            require_adapter_report_field_coverage: stage == AdapterAcceptanceStage::Enforced,
            require_operational_failures_not_quality: stage == AdapterAcceptanceStage::Enforced,
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        self.stage == AdapterAcceptanceStage::Enforced
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterRollbackResumeBoundaryPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub eval_entrypoint: String,
    pub allowed_inputs: Vec<String>,
    pub allowed_resume_gates: Vec<String>,
    pub produced_outputs: Vec<String>,
    pub forbidden_capabilities: Vec<String>,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl AdapterRollbackResumeBoundaryPlan {
    pub fn rollback_resume_boundary(
        name: impl Into<String>,
        stage: AdapterAcceptanceStage,
    ) -> Self {
        Self {
            name: name.into(),
            stage,
            eval_entrypoint: "RollbackResumeReport::from_gate_and_evidence".to_owned(),
            allowed_inputs: vec![
                "RollbackReport.resume_gate".to_owned(),
                "RollbackResumeEvidence".to_owned(),
                "RollbackResumeGate".to_owned(),
                "AdapterReportEmissionReport::field_coverage_passed".to_owned(),
            ],
            allowed_resume_gates: vec![
                "none".to_owned(),
                "chain_readiness_gate".to_owned(),
                "runtime_backend_health_check".to_owned(),
                "stream_continuity_validation".to_owned(),
                "runtime_response_gate".to_owned(),
                "model_quality_review".to_owned(),
                "planned_validation_command".to_owned(),
                "manual_failure_classification".to_owned(),
            ],
            produced_outputs: vec!["RollbackResumeReport".to_owned()],
            forbidden_capabilities: vec![
                "jsonl_io".to_owned(),
                "http_sse".to_owned(),
                "process_spawn".to_owned(),
                "validation_command_spawn".to_owned(),
                "model_call".to_owned(),
                "rollback_action_execution".to_owned(),
                "resume_action_execution".to_owned(),
                "runner_state".to_owned(),
            ],
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        false
    }

    pub fn allows_input(&self, input: &str) -> bool {
        self.allowed_inputs.iter().any(|allowed| allowed == input)
    }

    pub fn allows_resume_gate(&self, resume_gate: &str) -> bool {
        self.allowed_resume_gates
            .iter()
            .any(|allowed| allowed == resume_gate)
    }

    pub fn produces_output(&self, output: &str) -> bool {
        self.produced_outputs
            .iter()
            .any(|produced| produced == output)
    }

    pub fn forbids_capability(&self, capability: &str) -> bool {
        self.forbidden_capabilities
            .iter()
            .any(|forbidden| forbidden == capability)
    }

    pub fn stays_pure_data_boundary(&self) -> bool {
        self.eval_entrypoint == "RollbackResumeReport::from_gate_and_evidence"
            && self.allows_input("RollbackReport.resume_gate")
            && self.allows_input("RollbackResumeEvidence")
            && self.allows_input("RollbackResumeGate")
            && self.allows_input("AdapterReportEmissionReport::field_coverage_passed")
            && !self.allows_input("EvolutionLoopRunner")
            && self.allows_resume_gate("chain_readiness_gate")
            && self.allows_resume_gate("runtime_backend_health_check")
            && self.allows_resume_gate("planned_validation_command")
            && !self.allows_resume_gate("spawn_validation_command")
            && self.produces_output("RollbackResumeReport")
            && [
                "jsonl_io",
                "http_sse",
                "process_spawn",
                "validation_command_spawn",
                "model_call",
                "rollback_action_execution",
                "resume_action_execution",
                "runner_state",
            ]
            .iter()
            .all(|capability| self.forbids_capability(capability))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExperimentRolloutReportPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_name: String,
    pub require_clean_context_rot: bool,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl ExperimentRolloutReportPlan {
    pub fn experiment_rollout(name: impl Into<String>, stage: AdapterAcceptanceStage) -> Self {
        Self {
            name: name.into(),
            stage,
            report_schema_name: "experiment_rollout_report_v1".to_owned(),
            require_clean_context_rot: stage == AdapterAcceptanceStage::Enforced,
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        self.stage == AdapterAcceptanceStage::Enforced
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExperimentKillSwitchReportPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_name: String,
    pub require_kill_switch: bool,
    pub require_rollback_report: bool,
    pub require_rollback_resume_gate: bool,
    pub require_clean_context_rot: bool,
    pub require_owner_acknowledgement: bool,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl ExperimentKillSwitchReportPlan {
    pub fn experiment_kill_switch(name: impl Into<String>, stage: AdapterAcceptanceStage) -> Self {
        let enforced = stage == AdapterAcceptanceStage::Enforced;
        Self {
            name: name.into(),
            stage,
            report_schema_name: "experiment_kill_switch_report_v1".to_owned(),
            require_kill_switch: enforced,
            require_rollback_report: enforced,
            require_rollback_resume_gate: enforced,
            require_clean_context_rot: enforced,
            require_owner_acknowledgement: enforced,
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        self.stage == AdapterAcceptanceStage::Enforced
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExperimentExpansionSafetyReportPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_name: String,
    pub require_rollout_report: bool,
    pub require_kill_switch_report: bool,
    pub require_clean_context_rot: bool,
    pub require_rollback_resume: bool,
    pub require_model_pool_attribution: bool,
    pub require_adapter_report_emission: bool,
    pub require_adapter_report_field_coverage: bool,
    pub require_apple_silicon_development_effect: bool,
    pub require_promotion_window: bool,
    pub require_readiness: bool,
    pub require_steam_case_matrix: bool,
    pub require_validation_command_coverage: bool,
    pub require_root_adapter_ready: bool,
    pub keep_operational_failures_out_of_quality: bool,
    pub operational_report_sources: Vec<String>,
    pub forbidden_capabilities: Vec<String>,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl ExperimentExpansionSafetyReportPlan {
    pub fn experiment_expansion_safety(
        name: impl Into<String>,
        stage: AdapterAcceptanceStage,
    ) -> Self {
        let enforced = stage == AdapterAcceptanceStage::Enforced;
        Self {
            name: name.into(),
            stage,
            report_schema_name: "experiment_expansion_safety_report_v1".to_owned(),
            require_rollout_report: enforced,
            require_kill_switch_report: enforced,
            require_clean_context_rot: enforced,
            require_rollback_resume: enforced,
            require_model_pool_attribution: enforced,
            require_adapter_report_emission: enforced,
            require_adapter_report_field_coverage: enforced,
            require_apple_silicon_development_effect: enforced,
            require_promotion_window: enforced,
            require_readiness: enforced,
            require_steam_case_matrix: enforced,
            require_validation_command_coverage: enforced,
            require_root_adapter_ready: enforced,
            keep_operational_failures_out_of_quality: true,
            operational_report_sources: vec![
                "SelfEvolutionReadinessReport::can_schedule_next_round".to_owned(),
                "RollbackReport::present".to_owned(),
                "RollbackResumeReport::allow_unattended_rounds".to_owned(),
                "SteamCaseCoverageReport::allow_enforced_adapter".to_owned(),
                "ValidationCommandCoverageReport::allow_next_round".to_owned(),
            ],
            forbidden_capabilities: vec![
                "jsonl_io".to_owned(),
                "file_io".to_owned(),
                "http_sse".to_owned(),
                "process_spawn".to_owned(),
                "validation_command_spawn".to_owned(),
                "model_call".to_owned(),
                "runner_state".to_owned(),
            ],
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        self.stage == AdapterAcceptanceStage::Enforced
    }

    pub fn uses_operational_report_source(&self, source: &str) -> bool {
        self.operational_report_sources
            .iter()
            .any(|allowed| allowed == source)
    }

    pub fn forbids_capability(&self, capability: &str) -> bool {
        self.forbidden_capabilities
            .iter()
            .any(|forbidden| forbidden == capability)
    }

    pub fn stays_pure_data_boundary(&self) -> bool {
        self.uses_operational_report_source("SelfEvolutionReadinessReport::can_schedule_next_round")
            && self.uses_operational_report_source("RollbackReport::present")
            && self.uses_operational_report_source("RollbackResumeReport::allow_unattended_rounds")
            && self
                .uses_operational_report_source("SteamCaseCoverageReport::allow_enforced_adapter")
            && self
                .uses_operational_report_source("ValidationCommandCoverageReport::allow_next_round")
            && !self.uses_operational_report_source("JsonlLedgerReader::scan")
            && !self.uses_operational_report_source("ValidationCommandExecutor::spawn")
            && [
                "jsonl_io",
                "file_io",
                "http_sse",
                "process_spawn",
                "validation_command_spawn",
                "model_call",
                "runner_state",
            ]
            .iter()
            .all(|capability| self.forbids_capability(capability))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExperimentSwitchMatrixReportPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_name: String,
    pub required_input_reports: Vec<String>,
    pub required_report_fields: Vec<String>,
    pub require_enabled_flags_reported: bool,
    pub require_exactly_one_report_per_enabled_flag: bool,
    pub require_expansion_safety_passed: bool,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl ExperimentSwitchMatrixReportPlan {
    pub fn experiment_switch_matrix(
        name: impl Into<String>,
        stage: AdapterAcceptanceStage,
    ) -> Self {
        let enforced = stage == AdapterAcceptanceStage::Enforced;
        Self {
            name: name.into(),
            stage,
            report_schema_name: "experiment_switch_matrix_report_v1".to_owned(),
            required_input_reports: vec![
                "experiment_rollout_report_v1".to_owned(),
                "experiment_kill_switch_report_v1".to_owned(),
                "experiment_expansion_safety_report_v1".to_owned(),
            ],
            required_report_fields: vec![
                "experiment_switch.enabled_flag_names".to_owned(),
                "experiment_switch.reported_enabled_flag_names".to_owned(),
                "experiment_switch.missing_enabled_flag_reports".to_owned(),
                "experiment_switch.duplicate_enabled_flag_reports".to_owned(),
                "experiment_switch.unknown_enabled_flag_reports".to_owned(),
                "experiment_switch.exactly_one_report_per_enabled_flag".to_owned(),
                "experiment_switch.all_expansion_reports_passed".to_owned(),
                "experiment_switch.allow_experiment_switch_expansion".to_owned(),
            ],
            require_enabled_flags_reported: enforced,
            require_exactly_one_report_per_enabled_flag: enforced,
            require_expansion_safety_passed: enforced,
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn requires_input_report(&self, report: &str) -> bool {
        self.required_input_reports
            .iter()
            .any(|required| required == report)
    }

    pub fn requires_report_field(&self, field: &str) -> bool {
        self.required_report_fields
            .iter()
            .any(|required| required == field)
    }

    pub fn may_block_current_runner(&self) -> bool {
        self.stage == AdapterAcceptanceStage::Enforced
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedbackSelfImproveReportPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_name: String,
    pub require_closed_loop_gate: bool,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl FeedbackSelfImproveReportPlan {
    pub fn feedback_self_improve(name: impl Into<String>, stage: AdapterAcceptanceStage) -> Self {
        Self {
            name: name.into(),
            stage,
            report_schema_name: "feedback_self_improve_report_v1".to_owned(),
            require_closed_loop_gate: stage == AdapterAcceptanceStage::Enforced,
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        self.stage == AdapterAcceptanceStage::Enforced
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterFeedbackSelfImproveBoundaryPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub eval_entrypoints: Vec<String>,
    pub allowed_inputs: Vec<String>,
    pub produced_outputs: Vec<String>,
    pub forbidden_capabilities: Vec<String>,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl AdapterFeedbackSelfImproveBoundaryPlan {
    pub fn feedback_self_improve_boundary(
        name: impl Into<String>,
        stage: AdapterAcceptanceStage,
    ) -> Self {
        Self {
            name: name.into(),
            stage,
            eval_entrypoints: vec![
                "LedgerSummary::from_records".to_owned(),
                "ReportGate::evaluate".to_owned(),
                "FeedbackSelfImproveReport::from_summary_and_gate".to_owned(),
                "FeedbackSelfImproveReport::from_records_and_gate".to_owned(),
            ],
            allowed_inputs: vec![
                "LedgerRecord".to_owned(),
                "LedgerSummary".to_owned(),
                "ReportGate".to_owned(),
                "GateDecision".to_owned(),
            ],
            produced_outputs: vec![
                "LedgerSummary".to_owned(),
                "GateDecision".to_owned(),
                "FeedbackSelfImproveReport".to_owned(),
            ],
            forbidden_capabilities: vec![
                "jsonl_io".to_owned(),
                "file_io".to_owned(),
                "http_sse".to_owned(),
                "process_spawn".to_owned(),
                "validation_command_spawn".to_owned(),
                "model_call".to_owned(),
                "runner_state".to_owned(),
            ],
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        false
    }

    pub fn exposes_entrypoint(&self, entrypoint: &str) -> bool {
        self.eval_entrypoints
            .iter()
            .any(|exposed| exposed == entrypoint)
    }

    pub fn allows_input(&self, input: &str) -> bool {
        self.allowed_inputs.iter().any(|allowed| allowed == input)
    }

    pub fn produces_output(&self, output: &str) -> bool {
        self.produced_outputs
            .iter()
            .any(|produced| produced == output)
    }

    pub fn forbids_capability(&self, capability: &str) -> bool {
        self.forbidden_capabilities
            .iter()
            .any(|forbidden| forbidden == capability)
    }

    pub fn stays_pure_data_boundary(&self) -> bool {
        self.exposes_entrypoint("LedgerSummary::from_records")
            && self.exposes_entrypoint("ReportGate::evaluate")
            && self.exposes_entrypoint("FeedbackSelfImproveReport::from_summary_and_gate")
            && self.exposes_entrypoint("FeedbackSelfImproveReport::from_records_and_gate")
            && self.allows_input("LedgerRecord")
            && self.allows_input("LedgerSummary")
            && self.allows_input("ReportGate")
            && self.allows_input("GateDecision")
            && !self.allows_input("JsonlReader")
            && !self.allows_input("RunnerLedger")
            && self.produces_output("LedgerSummary")
            && self.produces_output("GateDecision")
            && self.produces_output("FeedbackSelfImproveReport")
            && [
                "jsonl_io",
                "file_io",
                "http_sse",
                "process_spawn",
                "validation_command_spawn",
                "model_call",
                "runner_state",
            ]
            .iter()
            .all(|capability| self.forbids_capability(capability))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdviceContinuationReportPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_name: String,
    pub max_repeated_advice: usize,
    pub max_invalid_advice: usize,
    pub max_invalid_commands: usize,
    pub require_latest_round_success: bool,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl AdviceContinuationReportPlan {
    pub fn advice_continuation(name: impl Into<String>, stage: AdapterAcceptanceStage) -> Self {
        let enforced = stage == AdapterAcceptanceStage::Enforced;
        Self {
            name: name.into(),
            stage,
            report_schema_name: "advice_continuation_report_v1".to_owned(),
            max_repeated_advice: usize::from(!enforced),
            max_invalid_advice: usize::from(!enforced),
            max_invalid_commands: usize::from(!enforced),
            require_latest_round_success: enforced,
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        self.stage == AdapterAcceptanceStage::Enforced
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdviceContinuationBoundaryPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub eval_entrypoints: Vec<String>,
    pub allowed_inputs: Vec<String>,
    pub produced_outputs: Vec<String>,
    pub forbidden_capabilities: Vec<String>,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl AdviceContinuationBoundaryPlan {
    pub fn advice_continuation_boundary(
        name: impl Into<String>,
        stage: AdapterAcceptanceStage,
    ) -> Self {
        Self {
            name: name.into(),
            stage,
            eval_entrypoints: vec![
                "AdviceContinuationEvidence::from_observations_and_summary".to_owned(),
                "AdviceContinuationGate::evaluate".to_owned(),
                "AdviceContinuationReport::from_gate_and_evidence".to_owned(),
                "AdviceContinuationReport::from_observations_summary_and_gate".to_owned(),
            ],
            allowed_inputs: vec![
                "AdviceContinuationObservation".to_owned(),
                "LedgerSummary".to_owned(),
                "AdviceContinuationEvidence".to_owned(),
                "AdviceContinuationGate".to_owned(),
                "GateDecision".to_owned(),
            ],
            produced_outputs: vec![
                "AdviceContinuationEvidence".to_owned(),
                "GateDecision".to_owned(),
                "AdviceContinuationReport".to_owned(),
            ],
            forbidden_capabilities: vec![
                "jsonl_io".to_owned(),
                "file_io".to_owned(),
                "report_directory_scan".to_owned(),
                "http_sse".to_owned(),
                "process_spawn".to_owned(),
                "validation_command_spawn".to_owned(),
                "model_call".to_owned(),
                "daemon_control".to_owned(),
                "runner_state".to_owned(),
            ],
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        false
    }

    pub fn exposes_entrypoint(&self, entrypoint: &str) -> bool {
        self.eval_entrypoints
            .iter()
            .any(|exposed| exposed == entrypoint)
    }

    pub fn allows_input(&self, input: &str) -> bool {
        self.allowed_inputs.iter().any(|allowed| allowed == input)
    }

    pub fn produces_output(&self, output: &str) -> bool {
        self.produced_outputs
            .iter()
            .any(|produced| produced == output)
    }

    pub fn forbids_capability(&self, capability: &str) -> bool {
        self.forbidden_capabilities
            .iter()
            .any(|forbidden| forbidden == capability)
    }

    pub fn stays_pure_data_boundary(&self) -> bool {
        self.exposes_entrypoint("AdviceContinuationEvidence::from_observations_and_summary")
            && self.exposes_entrypoint("AdviceContinuationGate::evaluate")
            && self.exposes_entrypoint("AdviceContinuationReport::from_gate_and_evidence")
            && self
                .exposes_entrypoint("AdviceContinuationReport::from_observations_summary_and_gate")
            && self.allows_input("AdviceContinuationObservation")
            && self.allows_input("LedgerSummary")
            && self.allows_input("AdviceContinuationEvidence")
            && self.allows_input("AdviceContinuationGate")
            && self.allows_input("GateDecision")
            && !self.allows_input("JsonlReader")
            && !self.allows_input("EvolutionLoopRunner")
            && self.produces_output("AdviceContinuationEvidence")
            && self.produces_output("GateDecision")
            && self.produces_output("AdviceContinuationReport")
            && [
                "jsonl_io",
                "file_io",
                "report_directory_scan",
                "http_sse",
                "process_spawn",
                "validation_command_spawn",
                "model_call",
                "daemon_control",
                "runner_state",
            ]
            .iter()
            .all(|capability| self.forbids_capability(capability))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfEvolutionContinuityReportPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_name: String,
    pub require_adjacent_rounds: bool,
    pub require_feedback_carryover: bool,
    pub require_self_improve_passed: bool,
    pub require_validation_passed: bool,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl SelfEvolutionContinuityReportPlan {
    pub fn self_evolution_continuity(
        name: impl Into<String>,
        stage: AdapterAcceptanceStage,
    ) -> Self {
        Self {
            name: name.into(),
            stage,
            report_schema_name: "self_evolution_continuity_report_v1".to_owned(),
            require_adjacent_rounds: stage == AdapterAcceptanceStage::Enforced,
            require_feedback_carryover: stage == AdapterAcceptanceStage::Enforced,
            require_self_improve_passed: stage == AdapterAcceptanceStage::Enforced,
            require_validation_passed: stage == AdapterAcceptanceStage::Enforced,
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        self.stage == AdapterAcceptanceStage::Enforced
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerGateReportPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_name: String,
    pub require_strict_ledger_hygiene: bool,
    pub require_last_success_policy: bool,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl LedgerGateReportPlan {
    pub fn ledger_gate(name: impl Into<String>, stage: AdapterAcceptanceStage) -> Self {
        Self {
            name: name.into(),
            stage,
            report_schema_name: "ledger_gate_report_v1".to_owned(),
            require_strict_ledger_hygiene: stage == AdapterAcceptanceStage::Enforced,
            require_last_success_policy: stage == AdapterAcceptanceStage::Enforced,
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        self.stage == AdapterAcceptanceStage::Enforced
    }

    pub fn downstream_projection_field_mappings() -> Vec<(&'static str, &'static str)> {
        vec![
            (
                "ledger.total_rounds",
                "report_freshness.ledger_gate_total_rounds",
            ),
            (
                "ledger.gate_blocked",
                "report_freshness.ledger_gate_blocked",
            ),
            (
                "ledger.allow_next_round",
                "report_freshness.ledger_gate_allow_next_round",
            ),
            (
                "ledger.allow_next_round",
                "run_mode_report_refresh.ledger_gate_allow_next_round",
            ),
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportFreshnessReportPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_name: String,
    pub required_report_fields: Vec<String>,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl ReportFreshnessReportPlan {
    pub fn report_freshness(name: impl Into<String>, stage: AdapterAcceptanceStage) -> Self {
        Self {
            name: name.into(),
            stage,
            report_schema_name: "report_freshness_report_v1".to_owned(),
            required_report_fields: vec![
                "report_freshness.rounds".to_owned(),
                "report_freshness.ledger_lag".to_owned(),
                "report_freshness.stale".to_owned(),
                "report_freshness.gate_failures".to_owned(),
                "report_freshness.ledger_gate_total_rounds".to_owned(),
                "report_freshness.ledger_gate_blocked".to_owned(),
                "report_freshness.ledger_gate_allow_next_round".to_owned(),
                "report_freshness.fresh".to_owned(),
                "report_freshness.freshness_blocked".to_owned(),
                "report_freshness.allow_next_round".to_owned(),
                "report_freshness.failure_reasons".to_owned(),
            ],
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        false
    }

    pub fn runner_status_field_mappings() -> Vec<(&'static str, &'static str)> {
        vec![
            ("rounds", "report_freshness.rounds"),
            ("ledger_lag", "report_freshness.ledger_lag"),
            ("stale", "report_freshness.stale"),
            ("gate_failures", "report_freshness.gate_failures"),
        ]
    }

    pub fn requires_report_field(&self, field: &str) -> bool {
        self.required_report_fields
            .iter()
            .any(|required| required == field)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterReportFreshnessBoundaryPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub eval_entrypoints: Vec<String>,
    pub allowed_inputs: Vec<String>,
    pub produced_outputs: Vec<String>,
    pub forbidden_capabilities: Vec<String>,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl AdapterReportFreshnessBoundaryPlan {
    pub fn report_freshness(name: impl Into<String>, stage: AdapterAcceptanceStage) -> Self {
        Self {
            name: name.into(),
            stage,
            eval_entrypoints: vec![
                "ReportFreshnessStatus::from_runner_status".to_owned(),
                "ReportFreshnessReport::from_status_and_ledger_gate".to_owned(),
            ],
            allowed_inputs: vec![
                "runner_report_rounds".to_owned(),
                "runner_report_ledger_lag".to_owned(),
                "runner_report_stale".to_owned(),
                "runner_report_gate_failures".to_owned(),
                "ReportFreshnessStatus".to_owned(),
                "LedgerGateReport".to_owned(),
            ],
            produced_outputs: vec![
                "ReportFreshnessStatus".to_owned(),
                "ReportFreshnessReport".to_owned(),
            ],
            forbidden_capabilities: vec![
                "jsonl_io".to_owned(),
                "file_io".to_owned(),
                "http_sse".to_owned(),
                "process_spawn".to_owned(),
                "validation_command_spawn".to_owned(),
                "daemon_control".to_owned(),
                "runner_state".to_owned(),
                "remote_mac_call".to_owned(),
                "model_call".to_owned(),
            ],
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        false
    }

    pub fn exposes_entrypoint(&self, entrypoint: &str) -> bool {
        self.eval_entrypoints
            .iter()
            .any(|exposed| exposed == entrypoint)
    }

    pub fn allows_input(&self, input: &str) -> bool {
        self.allowed_inputs.iter().any(|allowed| allowed == input)
    }

    pub fn produces_output(&self, output: &str) -> bool {
        self.produced_outputs
            .iter()
            .any(|produced| produced == output)
    }

    pub fn forbids_capability(&self, capability: &str) -> bool {
        self.forbidden_capabilities
            .iter()
            .any(|forbidden| forbidden == capability)
    }

    pub fn stays_pure_data_boundary(&self) -> bool {
        self.exposes_entrypoint("ReportFreshnessStatus::from_runner_status")
            && self.exposes_entrypoint("ReportFreshnessReport::from_status_and_ledger_gate")
            && self.allows_input("runner_report_rounds")
            && self.allows_input("runner_report_ledger_lag")
            && self.allows_input("runner_report_stale")
            && self.allows_input("runner_report_gate_failures")
            && self.allows_input("ReportFreshnessStatus")
            && self.allows_input("LedgerGateReport")
            && !self.allows_input("JsonlLedgerReader")
            && !self.allows_input("EvolutionLoopRunner")
            && !self.allows_input("DaemonHandle")
            && !self.allows_input("RemoteModelClient")
            && self.produces_output("ReportFreshnessStatus")
            && self.produces_output("ReportFreshnessReport")
            && [
                "jsonl_io",
                "file_io",
                "http_sse",
                "process_spawn",
                "validation_command_spawn",
                "daemon_control",
                "runner_state",
                "remote_mac_call",
                "model_call",
            ]
            .iter()
            .all(|capability| self.forbids_capability(capability))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteRuntimeAccelerationReportPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_name: String,
    pub required_report_fields: Vec<String>,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl RemoteRuntimeAccelerationReportPlan {
    pub fn remote_runtime_acceleration(
        name: impl Into<String>,
        stage: AdapterAcceptanceStage,
    ) -> Self {
        Self {
            name: name.into(),
            stage,
            report_schema_name: "remote_runtime_acceleration_report_v1".to_owned(),
            required_report_fields: vec![
                "remote_runtime.total_workers".to_owned(),
                "remote_runtime.healthy_workers".to_owned(),
                "remote_runtime.metal_workers".to_owned(),
                "remote_runtime.quality_model".to_owned(),
                "remote_runtime.all_workers_healthy".to_owned(),
                "remote_runtime.all_workers_metal".to_owned(),
                "remote_runtime.quality_model_present".to_owned(),
                "remote_runtime.acceleration_ready".to_owned(),
                "remote_runtime.failure_reasons".to_owned(),
            ],
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        false
    }

    pub fn requires_report_field(&self, field: &str) -> bool {
        self.required_report_fields
            .iter()
            .any(|required| required == field)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunModeReportRefreshAcceptancePlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_name: String,
    pub eval_entrypoints: Vec<String>,
    pub required_input_reports: Vec<String>,
    pub produced_report_fields: Vec<String>,
    pub forbidden_capabilities: Vec<String>,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl RunModeReportRefreshAcceptancePlan {
    pub fn run_mode_report_refresh(name: impl Into<String>, stage: AdapterAcceptanceStage) -> Self {
        Self {
            name: name.into(),
            stage,
            report_schema_name: "run_mode_report_refresh_acceptance_report_v1".to_owned(),
            eval_entrypoints: vec!["RunModeReportRefreshAcceptanceReport::from_reports".to_owned()],
            required_input_reports: vec![
                "ReportFreshnessReport".to_owned(),
                "RemoteRuntimeAccelerationReport".to_owned(),
                "EvalReportBundleGateReport".to_owned(),
            ],
            produced_report_fields: vec![
                "run_mode_report_refresh.report_refresh_allowed".to_owned(),
                "run_mode_report_refresh.ledger_gate_allow_next_round".to_owned(),
                "run_mode_report_refresh.remote_runtime_acceleration_ready".to_owned(),
                "run_mode_report_refresh.report_bundle_complete".to_owned(),
                "run_mode_report_refresh.allow_next_round".to_owned(),
                "run_mode_report_refresh.failure_reasons".to_owned(),
            ],
            forbidden_capabilities: vec![
                "jsonl_io".to_owned(),
                "file_io".to_owned(),
                "http_sse".to_owned(),
                "process_spawn".to_owned(),
                "validation_command_spawn".to_owned(),
                "daemon_control".to_owned(),
                "runner_state".to_owned(),
                "remote_mac_call".to_owned(),
                "model_call".to_owned(),
            ],
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        false
    }

    pub fn exposes_entrypoint(&self, entrypoint: &str) -> bool {
        self.eval_entrypoints
            .iter()
            .any(|exposed| exposed == entrypoint)
    }

    pub fn requires_input_report(&self, report: &str) -> bool {
        self.required_input_reports
            .iter()
            .any(|required| required == report)
    }

    pub fn produces_report_field(&self, field: &str) -> bool {
        self.produced_report_fields
            .iter()
            .any(|produced| produced == field)
    }

    pub fn forbids_capability(&self, capability: &str) -> bool {
        self.forbidden_capabilities
            .iter()
            .any(|forbidden| forbidden == capability)
    }

    pub fn stays_pure_data_boundary(&self) -> bool {
        self.eval_entrypoints
            .contains(&"RunModeReportRefreshAcceptanceReport::from_reports".to_owned())
            && self.requires_input_report("ReportFreshnessReport")
            && self.requires_input_report("RemoteRuntimeAccelerationReport")
            && self.requires_input_report("EvalReportBundleGateReport")
            && self.produces_report_field("run_mode_report_refresh.allow_next_round")
            && [
                "jsonl_io",
                "file_io",
                "http_sse",
                "process_spawn",
                "validation_command_spawn",
                "daemon_control",
                "runner_state",
                "remote_mac_call",
                "model_call",
            ]
            .iter()
            .all(|capability| self.forbids_capability(capability))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterRemoteRuntimeBoundaryPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub eval_entrypoints: Vec<String>,
    pub allowed_inputs: Vec<String>,
    pub produced_outputs: Vec<String>,
    pub forbidden_capabilities: Vec<String>,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl AdapterRemoteRuntimeBoundaryPlan {
    pub fn remote_runtime_acceleration(
        name: impl Into<String>,
        stage: AdapterAcceptanceStage,
    ) -> Self {
        Self {
            name: name.into(),
            stage,
            eval_entrypoints: vec![
                "RemoteRuntimeAccelerationStatus::from_runner_pool_status".to_owned(),
                "RemoteRuntimeAccelerationReport::from_status".to_owned(),
            ],
            allowed_inputs: vec![
                "runner_remote_total_workers".to_owned(),
                "runner_remote_healthy_workers".to_owned(),
                "runner_remote_metal_workers".to_owned(),
                "runner_remote_quality_model".to_owned(),
                "RemoteRuntimeAccelerationStatus".to_owned(),
            ],
            produced_outputs: vec![
                "RemoteRuntimeAccelerationStatus".to_owned(),
                "RemoteRuntimeAccelerationReport".to_owned(),
            ],
            forbidden_capabilities: vec![
                "jsonl_io".to_owned(),
                "file_io".to_owned(),
                "http_sse".to_owned(),
                "process_spawn".to_owned(),
                "validation_command_spawn".to_owned(),
                "daemon_control".to_owned(),
                "runner_state".to_owned(),
                "remote_mac_call".to_owned(),
                "model_call".to_owned(),
            ],
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        false
    }

    pub fn exposes_entrypoint(&self, entrypoint: &str) -> bool {
        self.eval_entrypoints
            .iter()
            .any(|exposed| exposed == entrypoint)
    }

    pub fn allows_input(&self, input: &str) -> bool {
        self.allowed_inputs.iter().any(|allowed| allowed == input)
    }

    pub fn produces_output(&self, output: &str) -> bool {
        self.produced_outputs
            .iter()
            .any(|produced| produced == output)
    }

    pub fn forbids_capability(&self, capability: &str) -> bool {
        self.forbidden_capabilities
            .iter()
            .any(|forbidden| forbidden == capability)
    }

    pub fn stays_pure_data_boundary(&self) -> bool {
        self.exposes_entrypoint("RemoteRuntimeAccelerationStatus::from_runner_pool_status")
            && self.exposes_entrypoint("RemoteRuntimeAccelerationReport::from_status")
            && self.allows_input("runner_remote_total_workers")
            && self.allows_input("runner_remote_healthy_workers")
            && self.allows_input("runner_remote_metal_workers")
            && self.allows_input("runner_remote_quality_model")
            && self.allows_input("RemoteRuntimeAccelerationStatus")
            && !self.allows_input("RemoteModelClient")
            && !self.allows_input("RemoteMacSession")
            && !self.allows_input("EvolutionLoopRunner")
            && self.produces_output("RemoteRuntimeAccelerationStatus")
            && self.produces_output("RemoteRuntimeAccelerationReport")
            && [
                "jsonl_io",
                "file_io",
                "http_sse",
                "process_spawn",
                "validation_command_spawn",
                "daemon_control",
                "runner_state",
                "remote_mac_call",
                "model_call",
            ]
            .iter()
            .all(|capability| self.forbids_capability(capability))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterLedgerGateBoundaryPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub eval_entrypoints: Vec<String>,
    pub allowed_inputs: Vec<String>,
    pub produced_outputs: Vec<String>,
    pub forbidden_capabilities: Vec<String>,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl AdapterLedgerGateBoundaryPlan {
    pub fn ledger_gate(name: impl Into<String>, stage: AdapterAcceptanceStage) -> Self {
        Self {
            name: name.into(),
            stage,
            eval_entrypoints: vec![
                "LedgerSummary::from_records".to_owned(),
                "ReportGate::evaluate".to_owned(),
                "LedgerGateReport::from_summary_and_gate".to_owned(),
                "LedgerGateReport::from_records_and_gate".to_owned(),
            ],
            allowed_inputs: vec![
                "LedgerRecord".to_owned(),
                "LedgerSummary".to_owned(),
                "ReportGate".to_owned(),
                "GateDecision".to_owned(),
            ],
            produced_outputs: vec![
                "LedgerSummary".to_owned(),
                "GateDecision".to_owned(),
                "LedgerGateReport".to_owned(),
            ],
            forbidden_capabilities: vec![
                "jsonl_io".to_owned(),
                "file_io".to_owned(),
                "http_sse".to_owned(),
                "process_spawn".to_owned(),
                "validation_command_spawn".to_owned(),
                "model_call".to_owned(),
                "runner_state".to_owned(),
            ],
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        false
    }

    pub fn exposes_entrypoint(&self, entrypoint: &str) -> bool {
        self.eval_entrypoints
            .iter()
            .any(|exposed| exposed == entrypoint)
    }

    pub fn allows_input(&self, input: &str) -> bool {
        self.allowed_inputs.iter().any(|allowed| allowed == input)
    }

    pub fn produces_output(&self, output: &str) -> bool {
        self.produced_outputs
            .iter()
            .any(|produced| produced == output)
    }

    pub fn forbids_capability(&self, capability: &str) -> bool {
        self.forbidden_capabilities
            .iter()
            .any(|forbidden| forbidden == capability)
    }

    pub fn stays_pure_data_boundary(&self) -> bool {
        self.exposes_entrypoint("LedgerSummary::from_records")
            && self.exposes_entrypoint("ReportGate::evaluate")
            && self.exposes_entrypoint("LedgerGateReport::from_summary_and_gate")
            && self.exposes_entrypoint("LedgerGateReport::from_records_and_gate")
            && self.allows_input("LedgerRecord")
            && self.allows_input("LedgerSummary")
            && self.allows_input("ReportGate")
            && self.allows_input("GateDecision")
            && !self.allows_input("JsonlLedgerReader")
            && !self.allows_input("EvolutionLoopRunner")
            && !self.allows_input("ValidationCommandExecutor")
            && self.produces_output("LedgerSummary")
            && self.produces_output("GateDecision")
            && self.produces_output("LedgerGateReport")
            && [
                "jsonl_io",
                "file_io",
                "http_sse",
                "process_spawn",
                "validation_command_spawn",
                "model_call",
                "runner_state",
            ]
            .iter()
            .all(|capability| self.forbids_capability(capability))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootAdapterAttributionReportPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_name: String,
    pub require_outage_attribution: bool,
    pub require_quality_failure_guard: bool,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl RootAdapterAttributionReportPlan {
    pub fn root_adapter_attribution(
        name: impl Into<String>,
        stage: AdapterAcceptanceStage,
    ) -> Self {
        Self {
            name: name.into(),
            stage,
            report_schema_name: "root_adapter_attribution_report_v1".to_owned(),
            require_outage_attribution: stage != AdapterAcceptanceStage::ShadowOnly,
            require_quality_failure_guard: stage == AdapterAcceptanceStage::Enforced,
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        self.stage == AdapterAcceptanceStage::Enforced
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfEvolutionRegressionReportPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_name: String,
    pub min_window_rounds: usize,
    pub require_validation_not_regressed: bool,
    pub require_self_improve_not_regressed: bool,
    pub require_feedback_not_regressed: bool,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl SelfEvolutionRegressionReportPlan {
    pub fn self_evolution_regression(
        name: impl Into<String>,
        stage: AdapterAcceptanceStage,
    ) -> Self {
        Self {
            name: name.into(),
            stage,
            report_schema_name: "self_evolution_regression_report_v1".to_owned(),
            min_window_rounds: 3,
            require_validation_not_regressed: stage == AdapterAcceptanceStage::Enforced,
            require_self_improve_not_regressed: stage == AdapterAcceptanceStage::Enforced,
            require_feedback_not_regressed: stage == AdapterAcceptanceStage::Enforced,
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        self.stage == AdapterAcceptanceStage::Enforced
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfEvolutionUnattendedPrerequisiteReportPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_name: String,
    pub require_continuity: bool,
    pub require_regression: bool,
    pub require_readiness_next_round: bool,
    pub require_context_rot_trend: bool,
    pub require_context_rot_remediation: bool,
    pub require_rollback_resume: bool,
    pub require_steam_case_matrix: bool,
    pub require_validation_command_coverage: bool,
    pub require_promotion_window: bool,
    pub require_adapter_report_field_coverage: bool,
    pub require_apple_silicon_development_effect: bool,
    pub operational_report_sources: Vec<String>,
    pub forbidden_capabilities: Vec<String>,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl SelfEvolutionUnattendedPrerequisiteReportPlan {
    pub fn unattended_prerequisites(
        name: impl Into<String>,
        stage: AdapterAcceptanceStage,
    ) -> Self {
        let enforced = stage == AdapterAcceptanceStage::Enforced;
        Self {
            name: name.into(),
            stage,
            report_schema_name: "self_evolution_unattended_prerequisites_report_v1".to_owned(),
            require_continuity: enforced,
            require_regression: enforced,
            require_readiness_next_round: enforced,
            require_context_rot_trend: enforced,
            require_context_rot_remediation: enforced,
            require_rollback_resume: enforced,
            require_steam_case_matrix: enforced,
            require_validation_command_coverage: enforced,
            require_promotion_window: enforced,
            require_adapter_report_field_coverage: enforced,
            require_apple_silicon_development_effect: enforced,
            operational_report_sources: vec![
                "SelfEvolutionReadinessReport::can_schedule_next_round".to_owned(),
                "AdviceContinuationReport::allow_continuation".to_owned(),
                "ContextRotTrendReport::allow_unattended_continuation".to_owned(),
                "ContextRotRemediationReport::allow_experiment_rollout".to_owned(),
                "RollbackResumeReport::allow_unattended_rounds".to_owned(),
                "SteamCaseCoverageReport::allow_enforced_adapter".to_owned(),
                "ValidationCommandCoverageReport::allow_next_round".to_owned(),
                "AdapterPromotionWindowReport::allow_enforcement".to_owned(),
                "AdapterReportEmissionReport::field_coverage_passed".to_owned(),
                "AppleSiliconDevelopmentEffectReport::allow_development_effect_claim".to_owned(),
            ],
            forbidden_capabilities: vec![
                "jsonl_io".to_owned(),
                "http_sse".to_owned(),
                "process_spawn".to_owned(),
                "validation_command_spawn".to_owned(),
                "model_call".to_owned(),
                "runner_state".to_owned(),
            ],
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        self.stage == AdapterAcceptanceStage::Enforced
    }

    pub fn uses_operational_report_source(&self, source: &str) -> bool {
        self.operational_report_sources
            .iter()
            .any(|required| required == source)
    }

    pub fn forbids_capability(&self, capability: &str) -> bool {
        self.forbidden_capabilities
            .iter()
            .any(|forbidden| forbidden == capability)
    }

    pub fn stays_pure_data_boundary(&self) -> bool {
        self.uses_operational_report_source("SelfEvolutionReadinessReport::can_schedule_next_round")
            && self.uses_operational_report_source("AdviceContinuationReport::allow_continuation")
            && self.uses_operational_report_source(
                "ContextRotTrendReport::allow_unattended_continuation",
            )
            && self.uses_operational_report_source(
                "ContextRotRemediationReport::allow_experiment_rollout",
            )
            && self.uses_operational_report_source("RollbackResumeReport::allow_unattended_rounds")
            && self
                .uses_operational_report_source("SteamCaseCoverageReport::allow_enforced_adapter")
            && self
                .uses_operational_report_source("ValidationCommandCoverageReport::allow_next_round")
            && self
                .uses_operational_report_source("AdapterPromotionWindowReport::allow_enforcement")
            && self.uses_operational_report_source(
                "AdapterReportEmissionReport::field_coverage_passed",
            )
            && self.uses_operational_report_source(
                "AppleSiliconDevelopmentEffectReport::allow_development_effect_claim",
            )
            && !self.uses_operational_report_source("JsonlLedgerReader::scan")
            && !self.uses_operational_report_source("ValidationCommandExecutor::run")
            && !self.uses_operational_report_source("EvolutionLoopRunner::state")
            && [
                "jsonl_io",
                "http_sse",
                "process_spawn",
                "validation_command_spawn",
                "model_call",
                "runner_state",
            ]
            .iter()
            .all(|capability| self.forbids_capability(capability))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrictUnattendedAcceptancePlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_name: String,
    pub eval_entrypoints: Vec<String>,
    pub allowed_inputs: Vec<String>,
    pub required_input_reports: Vec<String>,
    pub produced_outputs: Vec<String>,
    pub produced_report_fields: Vec<String>,
    pub forbidden_capabilities: Vec<String>,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl StrictUnattendedAcceptancePlan {
    pub fn strict_unattended_acceptance(
        name: impl Into<String>,
        stage: AdapterAcceptanceStage,
    ) -> Self {
        Self {
            name: name.into(),
            stage,
            report_schema_name: "strict_unattended_acceptance_report_v1".to_owned(),
            eval_entrypoints: vec![
                "StrictUnattendedSupervisorEvidence::from_status".to_owned(),
                "StrictUnattendedSupervisorEvidence::with_stale_pid_detected".to_owned(),
                "StrictUnattendedSupervisorEvidence::with_side_effect_flags".to_owned(),
                "StrictUnattendedSupervisorEvidence::allow_next_round".to_owned(),
                "StrictUnattendedAcceptanceReport::from_reports".to_owned(),
            ],
            allowed_inputs: vec![
                "StrictUnattendedSupervisorEvidence".to_owned(),
                "RunModeReportRefreshAcceptanceReport".to_owned(),
                "AdapterClosureReport".to_owned(),
                "ValidationCommandCoverageReport".to_owned(),
                "RollbackResumeReport".to_owned(),
                "SelfEvolutionRegressionReport".to_owned(),
                "SelfEvolutionReadinessReport".to_owned(),
                "SelfEvolutionUnattendedPrerequisiteReport".to_owned(),
            ],
            required_input_reports: vec![
                "run_mode_report_refresh_acceptance_report_v1".to_owned(),
                "adapter_closure_report_v1".to_owned(),
                "validation_command_coverage_report_v1".to_owned(),
                "rollback_resume_report_v1".to_owned(),
                "self_evolution_regression_report_v1".to_owned(),
                "readiness_next_round_v1".to_owned(),
                "self_evolution_unattended_prerequisites_report_v1".to_owned(),
            ],
            produced_outputs: vec![
                "StrictUnattendedSupervisorEvidence".to_owned(),
                "StrictUnattendedAcceptanceReport".to_owned(),
            ],
            produced_report_fields: vec![
                "strict_unattended_acceptance.supervisor_status_observed".to_owned(),
                "strict_unattended_acceptance.daemon_running".to_owned(),
                "strict_unattended_acceptance.strict_unattended_evolution_enabled".to_owned(),
                "strict_unattended_acceptance.configured_validation_enabled".to_owned(),
                "strict_unattended_acceptance.supervisor_check_only".to_owned(),
                "strict_unattended_acceptance.stale_pid_detected".to_owned(),
                "strict_unattended_acceptance.starts_process".to_owned(),
                "strict_unattended_acceptance.sends_prompt".to_owned(),
                "strict_unattended_acceptance.touches_remote".to_owned(),
                "strict_unattended_acceptance.supervisor_allow_next_round".to_owned(),
                "strict_unattended_acceptance.report_refresh_allow_next_round".to_owned(),
                "strict_unattended_acceptance.adapter_closure_allow_next_round".to_owned(),
                "strict_unattended_acceptance.validation_allow_next_round".to_owned(),
                "strict_unattended_acceptance.rollback_resume_allow_unattended_rounds".to_owned(),
                "strict_unattended_acceptance.self_improve_regression_allow_unattended_continuation".to_owned(),
                "strict_unattended_acceptance.readiness_can_schedule_next_round".to_owned(),
                "strict_unattended_acceptance.self_evolution_allow_unattended_claim".to_owned(),
                "strict_unattended_acceptance.allow_next_round".to_owned(),
                "strict_unattended_acceptance.failure_reasons".to_owned(),
            ],
            forbidden_capabilities: vec![
                "jsonl_io".to_owned(),
                "file_io".to_owned(),
                "http_sse".to_owned(),
                "process_spawn".to_owned(),
                "validation_command_spawn".to_owned(),
                "daemon_control".to_owned(),
                "runner_state".to_owned(),
                "remote_mac_call".to_owned(),
                "model_call".to_owned(),
                "helper_prose_parse".to_owned(),
                "chat_stream".to_owned(),
            ],
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        false
    }

    pub fn exposes_entrypoint(&self, entrypoint: &str) -> bool {
        self.eval_entrypoints
            .iter()
            .any(|exposed| exposed == entrypoint)
    }

    pub fn allows_input(&self, input: &str) -> bool {
        self.allowed_inputs.iter().any(|allowed| allowed == input)
    }

    pub fn requires_input_report(&self, report_schema_name: &str) -> bool {
        self.required_input_reports
            .iter()
            .any(|required| required == report_schema_name)
    }

    pub fn produces_output(&self, output: &str) -> bool {
        self.produced_outputs
            .iter()
            .any(|produced| produced == output)
    }

    pub fn produces_report_field(&self, field: &str) -> bool {
        self.produced_report_fields
            .iter()
            .any(|produced| produced == field)
    }

    pub fn forbids_capability(&self, capability: &str) -> bool {
        self.forbidden_capabilities
            .iter()
            .any(|forbidden| forbidden == capability)
    }

    pub fn stays_pure_data_boundary(&self) -> bool {
        self.exposes_entrypoint("StrictUnattendedSupervisorEvidence::from_status")
            && self.exposes_entrypoint("StrictUnattendedAcceptanceReport::from_reports")
            && self.allows_input("StrictUnattendedSupervisorEvidence")
            && self.allows_input("RunModeReportRefreshAcceptanceReport")
            && self.allows_input("AdapterClosureReport")
            && self.allows_input("ValidationCommandCoverageReport")
            && self.allows_input("RollbackResumeReport")
            && self.allows_input("SelfEvolutionRegressionReport")
            && self.allows_input("SelfEvolutionReadinessReport")
            && self.allows_input("SelfEvolutionUnattendedPrerequisiteReport")
            && !self.allows_input("EvolutionLoopRunner")
            && !self.allows_input("DaemonHandle")
            && !self.allows_input("ValidationCommandExecutor")
            && !self.allows_input("HelperStageContractSummary")
            && self.requires_input_report("run_mode_report_refresh_acceptance_report_v1")
            && self.requires_input_report("adapter_closure_report_v1")
            && self.requires_input_report("validation_command_coverage_report_v1")
            && self.requires_input_report("rollback_resume_report_v1")
            && self.requires_input_report("self_evolution_regression_report_v1")
            && self.requires_input_report("readiness_next_round_v1")
            && self.requires_input_report("self_evolution_unattended_prerequisites_report_v1")
            && self.produces_output("StrictUnattendedAcceptanceReport")
            && self.produces_report_field("strict_unattended_acceptance.allow_next_round")
            && self.produces_report_field("strict_unattended_acceptance.failure_reasons")
            && [
                "jsonl_io",
                "file_io",
                "http_sse",
                "process_spawn",
                "validation_command_spawn",
                "daemon_control",
                "runner_state",
                "remote_mac_call",
                "model_call",
                "helper_prose_parse",
                "chat_stream",
            ]
            .iter()
            .all(|capability| self.forbids_capability(capability))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerWindowReplacementPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_name: String,
    pub eval_entrypoints: Vec<String>,
    pub allowed_inputs: Vec<String>,
    pub produced_outputs: Vec<String>,
    pub produced_report_fields: Vec<String>,
    pub forbidden_capabilities: Vec<String>,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl WorkerWindowReplacementPlan {
    pub fn worker_window_replacement(
        name: impl Into<String>,
        stage: AdapterAcceptanceStage,
    ) -> Self {
        Self {
            name: name.into(),
            stage,
            report_schema_name: "worker_window_replacement_report_v1".to_owned(),
            eval_entrypoints: vec![
                "WorkerWindowReplacementEvidence::clean".to_owned(),
                "WorkerWindowReplacementEvidence::with_evidence_ids".to_owned(),
                "WorkerWindowReplacementEvidence::with_status".to_owned(),
                "WorkerWindowReplacementEvidence::no_old_thread_reads".to_owned(),
                "WorkerWindowReplacementEvidence::no_side_effects".to_owned(),
                "WorkerWindowReplacementEvidence::evidence_ids_only".to_owned(),
                "WorkerWindowReplacementGate::evaluate".to_owned(),
                "WorkerWindowReplacementReport::from_gate_and_evidence".to_owned(),
            ],
            allowed_inputs: vec![
                "WorkerWindowReplacementEvidence".to_owned(),
                "WorkerWindowReplacementGate".to_owned(),
                "worker_window_id".to_owned(),
                "evidence_ids".to_owned(),
                "paused".to_owned(),
                "polluted".to_owned(),
                "stale".to_owned(),
                "clean_room_replacement_required".to_owned(),
                "old_thread_read_attempted".to_owned(),
                "side_effects_observed".to_owned(),
            ],
            produced_outputs: vec![
                "WorkerWindowReplacementEvidence".to_owned(),
                "GateDecision".to_owned(),
                "WorkerWindowReplacementReport".to_owned(),
            ],
            produced_report_fields: vec![
                "worker_window_replacement.worker_window_id".to_owned(),
                "worker_window_replacement.evidence_ids".to_owned(),
                "worker_window_replacement.paused".to_owned(),
                "worker_window_replacement.polluted".to_owned(),
                "worker_window_replacement.stale".to_owned(),
                "worker_window_replacement.clean_room_replacement_required".to_owned(),
                "worker_window_replacement.no_old_thread_reads".to_owned(),
                "worker_window_replacement.no_side_effects".to_owned(),
                "worker_window_replacement.evidence_ids_only".to_owned(),
                "worker_window_replacement.allow_worker_continuation".to_owned(),
                "worker_window_replacement.failure_reasons".to_owned(),
            ],
            forbidden_capabilities: vec![
                "old_thread_read".to_owned(),
                "old_window_read".to_owned(),
                "chat_transcript_read".to_owned(),
                "jsonl_io".to_owned(),
                "file_io".to_owned(),
                "http_sse".to_owned(),
                "process_spawn".to_owned(),
                "validation_command_spawn".to_owned(),
                "daemon_control".to_owned(),
                "runner_state".to_owned(),
                "remote_mac_call".to_owned(),
                "model_call".to_owned(),
                "helper_prose_parse".to_owned(),
                "chat_stream".to_owned(),
                "side_effect_execution".to_owned(),
            ],
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        false
    }

    pub fn exposes_entrypoint(&self, entrypoint: &str) -> bool {
        self.eval_entrypoints
            .iter()
            .any(|exposed| exposed == entrypoint)
    }

    pub fn allows_input(&self, input: &str) -> bool {
        self.allowed_inputs.iter().any(|allowed| allowed == input)
    }

    pub fn produces_output(&self, output: &str) -> bool {
        self.produced_outputs
            .iter()
            .any(|produced| produced == output)
    }

    pub fn produces_report_field(&self, field: &str) -> bool {
        self.produced_report_fields
            .iter()
            .any(|produced| produced == field)
    }

    pub fn forbids_capability(&self, capability: &str) -> bool {
        self.forbidden_capabilities
            .iter()
            .any(|forbidden| forbidden == capability)
    }

    pub fn stays_pure_data_boundary(&self) -> bool {
        self.exposes_entrypoint("WorkerWindowReplacementEvidence::clean")
            && self.exposes_entrypoint("WorkerWindowReplacementEvidence::with_evidence_ids")
            && self.exposes_entrypoint("WorkerWindowReplacementEvidence::with_status")
            && self.exposes_entrypoint("WorkerWindowReplacementGate::evaluate")
            && self.exposes_entrypoint("WorkerWindowReplacementReport::from_gate_and_evidence")
            && self.allows_input("WorkerWindowReplacementEvidence")
            && self.allows_input("WorkerWindowReplacementGate")
            && self.allows_input("evidence_ids")
            && self.allows_input("paused")
            && self.allows_input("polluted")
            && self.allows_input("stale")
            && self.allows_input("clean_room_replacement_required")
            && !self.allows_input("OldThreadReader")
            && !self.allows_input("WorkerWindowMutator")
            && !self.allows_input("EvolutionLoopRunner")
            && self.produces_output("WorkerWindowReplacementReport")
            && self.produces_report_field("worker_window_replacement.paused")
            && self.produces_report_field("worker_window_replacement.polluted")
            && self.produces_report_field("worker_window_replacement.stale")
            && self
                .produces_report_field("worker_window_replacement.clean_room_replacement_required")
            && self.produces_report_field("worker_window_replacement.no_old_thread_reads")
            && self.produces_report_field("worker_window_replacement.no_side_effects")
            && self.produces_report_field("worker_window_replacement.evidence_ids_only")
            && self.produces_report_field("worker_window_replacement.allow_worker_continuation")
            && [
                "old_thread_read",
                "old_window_read",
                "chat_transcript_read",
                "jsonl_io",
                "file_io",
                "http_sse",
                "process_spawn",
                "validation_command_spawn",
                "daemon_control",
                "runner_state",
                "remote_mac_call",
                "model_call",
                "helper_prose_parse",
                "chat_stream",
                "side_effect_execution",
            ]
            .iter()
            .all(|capability| self.forbids_capability(capability))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusDrivenSelfEvolutionClosurePlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_name: String,
    pub eval_entrypoints: Vec<String>,
    pub allowed_inputs: Vec<String>,
    pub produced_outputs: Vec<String>,
    pub produced_report_fields: Vec<String>,
    pub forbidden_capabilities: Vec<String>,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl StatusDrivenSelfEvolutionClosurePlan {
    pub fn status_driven_self_evolution_closure(
        name: impl Into<String>,
        stage: AdapterAcceptanceStage,
    ) -> Self {
        Self {
            name: name.into(),
            stage,
            report_schema_name: "status_driven_self_evolution_closure_report_v1".to_owned(),
            eval_entrypoints: vec![
                "StatusDrivenSelfEvolutionClosureEvidence::r24_clean_room".to_owned(),
                "StatusDrivenSelfEvolutionClosureEvidence::with_memory_startup_admission_safe"
                    .to_owned(),
                "StatusDrivenSelfEvolutionClosureEvidence::with_worker_replacement_required"
                    .to_owned(),
                "StatusDrivenSelfEvolutionClosureEvidence::with_clean_room_assignment_allowed"
                    .to_owned(),
                "StatusDrivenSelfEvolutionClosureEvidence::no_old_thread_reads".to_owned(),
                "StatusDrivenSelfEvolutionClosureEvidence::no_live_writes".to_owned(),
                "StatusDrivenSelfEvolutionClosureEvidence::no_runtime_side_effects".to_owned(),
                "StatusDrivenSelfEvolutionClosureEvidence::evidence_ids_only".to_owned(),
                "StatusDrivenSelfEvolutionClosureGate::evaluate".to_owned(),
                "StatusDrivenSelfEvolutionClosureReport::from_gate_and_evidence".to_owned(),
            ],
            allowed_inputs: vec![
                "StatusDrivenSelfEvolutionClosureEvidence".to_owned(),
                "StatusDrivenSelfEvolutionClosureGate".to_owned(),
                "evidence_ids".to_owned(),
                "memory_startup_admission_safe".to_owned(),
                "worker_replacement_required".to_owned(),
                "clean_room_assignment_allowed".to_owned(),
                "old_thread_read_attempted".to_owned(),
                "live_write_attempted".to_owned(),
                "runtime_side_effects_observed".to_owned(),
                "report_only_continuation".to_owned(),
            ],
            produced_outputs: vec![
                "StatusDrivenSelfEvolutionClosureEvidence".to_owned(),
                "GateDecision".to_owned(),
                "StatusDrivenSelfEvolutionClosureReport".to_owned(),
            ],
            produced_report_fields: vec![
                "status_driven_self_evolution_closure.evidence_ids".to_owned(),
                "status_driven_self_evolution_closure.memory_startup_admission_safe".to_owned(),
                "status_driven_self_evolution_closure.worker_replacement_required".to_owned(),
                "status_driven_self_evolution_closure.clean_room_assignment_allowed".to_owned(),
                "status_driven_self_evolution_closure.no_old_thread_reads".to_owned(),
                "status_driven_self_evolution_closure.no_live_writes".to_owned(),
                "status_driven_self_evolution_closure.no_runtime_side_effects".to_owned(),
                "status_driven_self_evolution_closure.report_only_continuation".to_owned(),
                "status_driven_self_evolution_closure.evidence_ids_only".to_owned(),
                "status_driven_self_evolution_closure.allow_report_only_continuation".to_owned(),
                "status_driven_self_evolution_closure.allow_runtime_continuation".to_owned(),
                "status_driven_self_evolution_closure.failure_reasons".to_owned(),
            ],
            forbidden_capabilities: vec![
                "old_thread_read".to_owned(),
                "old_window_read".to_owned(),
                "chat_transcript_read".to_owned(),
                "jsonl_io".to_owned(),
                "file_io".to_owned(),
                "http_sse".to_owned(),
                "process_spawn".to_owned(),
                "validation_command_spawn".to_owned(),
                "daemon_control".to_owned(),
                "runner_state".to_owned(),
                "remote_mac_call".to_owned(),
                "model_call".to_owned(),
                "helper_prose_parse".to_owned(),
                "chat_stream".to_owned(),
                "live_write".to_owned(),
                "memory_store_write".to_owned(),
                "ndkv_write".to_owned(),
                "runtime_side_effect_execution".to_owned(),
            ],
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        false
    }

    pub fn exposes_entrypoint(&self, entrypoint: &str) -> bool {
        self.eval_entrypoints
            .iter()
            .any(|exposed| exposed == entrypoint)
    }

    pub fn allows_input(&self, input: &str) -> bool {
        self.allowed_inputs.iter().any(|allowed| allowed == input)
    }

    pub fn produces_output(&self, output: &str) -> bool {
        self.produced_outputs
            .iter()
            .any(|produced| produced == output)
    }

    pub fn produces_report_field(&self, field: &str) -> bool {
        self.produced_report_fields
            .iter()
            .any(|produced| produced == field)
    }

    pub fn forbids_capability(&self, capability: &str) -> bool {
        self.forbidden_capabilities
            .iter()
            .any(|forbidden| forbidden == capability)
    }

    pub fn stays_pure_data_boundary(&self) -> bool {
        self.exposes_entrypoint("StatusDrivenSelfEvolutionClosureEvidence::r24_clean_room")
            && self.exposes_entrypoint("StatusDrivenSelfEvolutionClosureGate::evaluate")
            && self.exposes_entrypoint(
                "StatusDrivenSelfEvolutionClosureReport::from_gate_and_evidence",
            )
            && self.allows_input("StatusDrivenSelfEvolutionClosureEvidence")
            && self.allows_input("StatusDrivenSelfEvolutionClosureGate")
            && self.allows_input("evidence_ids")
            && self.allows_input("memory_startup_admission_safe")
            && self.allows_input("worker_replacement_required")
            && self.allows_input("clean_room_assignment_allowed")
            && self.allows_input("report_only_continuation")
            && !self.allows_input("OldThreadReader")
            && !self.allows_input("LiveWriteExecutor")
            && !self.allows_input("RuntimeSideEffectExecutor")
            && !self.allows_input("EvolutionLoopRunner")
            && self.produces_output("StatusDrivenSelfEvolutionClosureReport")
            && self.produces_report_field("status_driven_self_evolution_closure.evidence_ids")
            && self.produces_report_field(
                "status_driven_self_evolution_closure.memory_startup_admission_safe",
            )
            && self.produces_report_field(
                "status_driven_self_evolution_closure.worker_replacement_required",
            )
            && self.produces_report_field(
                "status_driven_self_evolution_closure.clean_room_assignment_allowed",
            )
            && self
                .produces_report_field("status_driven_self_evolution_closure.no_old_thread_reads")
            && self.produces_report_field("status_driven_self_evolution_closure.no_live_writes")
            && self.produces_report_field(
                "status_driven_self_evolution_closure.no_runtime_side_effects",
            )
            && self.produces_report_field(
                "status_driven_self_evolution_closure.report_only_continuation",
            )
            && self.produces_report_field(
                "status_driven_self_evolution_closure.allow_report_only_continuation",
            )
            && self.produces_report_field(
                "status_driven_self_evolution_closure.allow_runtime_continuation",
            )
            && [
                "old_thread_read",
                "old_window_read",
                "chat_transcript_read",
                "jsonl_io",
                "file_io",
                "http_sse",
                "process_spawn",
                "validation_command_spawn",
                "daemon_control",
                "runner_state",
                "remote_mac_call",
                "model_call",
                "helper_prose_parse",
                "chat_stream",
                "live_write",
                "memory_store_write",
                "ndkv_write",
                "runtime_side_effect_execution",
            ]
            .iter()
            .all(|capability| self.forbids_capability(capability))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanRoomHandoffReportPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_name: String,
    pub eval_entrypoints: Vec<String>,
    pub allowed_inputs: Vec<String>,
    pub produced_outputs: Vec<String>,
    pub produced_report_fields: Vec<String>,
    pub forbidden_capabilities: Vec<String>,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl CleanRoomHandoffReportPlan {
    pub fn clean_room_handoff_report(
        name: impl Into<String>,
        stage: AdapterAcceptanceStage,
    ) -> Self {
        Self {
            name: name.into(),
            stage,
            report_schema_name: "clean_room_handoff_report_v1".to_owned(),
            eval_entrypoints: vec![
                "CleanRoomHandoffEvidence::r25_clean_room_handoff".to_owned(),
                "CleanRoomHandoffEvidence::with_memory_startup_admission".to_owned(),
                "CleanRoomHandoffEvidence::with_agent_replacement_plan".to_owned(),
                "CleanRoomHandoffEvidence::with_source_json_parse_flags".to_owned(),
                "CleanRoomHandoffEvidence::with_side_effect_attempts".to_owned(),
                "CleanRoomHandoffEvidence::source_json_retained".to_owned(),
                "CleanRoomHandoffEvidence::source_json_not_parsed_as_prompt_or_live_write"
                    .to_owned(),
                "CleanRoomHandoffEvidence::side_effects_all_false".to_owned(),
                "CleanRoomHandoffGate::evaluate".to_owned(),
                "CleanRoomHandoffReport::from_gate_and_evidence".to_owned(),
            ],
            allowed_inputs: vec![
                "CleanRoomHandoffEvidence".to_owned(),
                "CleanRoomHandoffGate".to_owned(),
                "evidence_ids".to_owned(),
                "memory_startup_admission_json".to_owned(),
                "agent_clean_room_replacement_plan_json".to_owned(),
                "memory_startup_admission_input_present".to_owned(),
                "memory_startup_admission_safe".to_owned(),
                "agent_replacement_plan_input_present".to_owned(),
                "agent_replacement_plan_clean_room_required".to_owned(),
                "source_json".to_owned(),
                "source_json_parsed_as_prompt".to_owned(),
                "source_json_parsed_as_live_write".to_owned(),
                "report_only_continuation".to_owned(),
            ],
            produced_outputs: vec![
                "CleanRoomHandoffSourceJson".to_owned(),
                "CleanRoomHandoffEvidence".to_owned(),
                "GateDecision".to_owned(),
                "CleanRoomHandoffReport".to_owned(),
            ],
            produced_report_fields: vec![
                "clean_room_handoff.evidence_ids".to_owned(),
                "clean_room_handoff.memory_startup_admission_input_present".to_owned(),
                "clean_room_handoff.memory_startup_admission_safe".to_owned(),
                "clean_room_handoff.agent_replacement_plan_input_present".to_owned(),
                "clean_room_handoff.agent_replacement_plan_clean_room_required".to_owned(),
                "clean_room_handoff.source_json_input_names".to_owned(),
                "clean_room_handoff.source_json_retained".to_owned(),
                "clean_room_handoff.source_json_not_parsed_as_prompt_or_live_write".to_owned(),
                "clean_room_handoff.side_effects_all_false".to_owned(),
                "clean_room_handoff.no_old_thread_reads".to_owned(),
                "clean_room_handoff.evidence_ids_only".to_owned(),
                "clean_room_handoff.report_only_continuation".to_owned(),
                "clean_room_handoff.allow_report_only_continuation".to_owned(),
                "clean_room_handoff.allow_runtime_continuation".to_owned(),
                "clean_room_handoff.failure_reasons".to_owned(),
            ],
            forbidden_capabilities: vec![
                "old_thread_read".to_owned(),
                "old_window_read".to_owned(),
                "chat_transcript_read".to_owned(),
                "jsonl_io".to_owned(),
                "file_io".to_owned(),
                "http_sse".to_owned(),
                "process_spawn".to_owned(),
                "validation_command_spawn".to_owned(),
                "daemon_control".to_owned(),
                "runner_state".to_owned(),
                "remote_mac_call".to_owned(),
                "model_call".to_owned(),
                "helper_prose_parse".to_owned(),
                "chat_stream".to_owned(),
                "prompt_parse".to_owned(),
                "live_write".to_owned(),
                "memory_store_write".to_owned(),
                "ndkv_write".to_owned(),
                "runtime_side_effect_execution".to_owned(),
            ],
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        false
    }

    pub fn exposes_entrypoint(&self, entrypoint: &str) -> bool {
        self.eval_entrypoints
            .iter()
            .any(|exposed| exposed == entrypoint)
    }

    pub fn allows_input(&self, input: &str) -> bool {
        self.allowed_inputs.iter().any(|allowed| allowed == input)
    }

    pub fn produces_output(&self, output: &str) -> bool {
        self.produced_outputs
            .iter()
            .any(|produced| produced == output)
    }

    pub fn produces_report_field(&self, field: &str) -> bool {
        self.produced_report_fields
            .iter()
            .any(|produced| produced == field)
    }

    pub fn forbids_capability(&self, capability: &str) -> bool {
        self.forbidden_capabilities
            .iter()
            .any(|forbidden| forbidden == capability)
    }

    pub fn stays_pure_data_boundary(&self) -> bool {
        self.exposes_entrypoint("CleanRoomHandoffEvidence::r25_clean_room_handoff")
            && self.exposes_entrypoint("CleanRoomHandoffGate::evaluate")
            && self.exposes_entrypoint("CleanRoomHandoffReport::from_gate_and_evidence")
            && self.allows_input("CleanRoomHandoffEvidence")
            && self.allows_input("CleanRoomHandoffGate")
            && self.allows_input("memory_startup_admission_json")
            && self.allows_input("agent_clean_room_replacement_plan_json")
            && self.allows_input("source_json")
            && self.allows_input("report_only_continuation")
            && !self.allows_input("OldThreadReader")
            && !self.allows_input("SourceJsonPromptParser")
            && !self.allows_input("LiveWriteExecutor")
            && !self.allows_input("EvolutionLoopRunner")
            && self.produces_output("CleanRoomHandoffReport")
            && self
                .produces_report_field("clean_room_handoff.memory_startup_admission_input_present")
            && self.produces_report_field("clean_room_handoff.memory_startup_admission_safe")
            && self.produces_report_field("clean_room_handoff.agent_replacement_plan_input_present")
            && self.produces_report_field(
                "clean_room_handoff.agent_replacement_plan_clean_room_required",
            )
            && self.produces_report_field("clean_room_handoff.source_json_retained")
            && self.produces_report_field(
                "clean_room_handoff.source_json_not_parsed_as_prompt_or_live_write",
            )
            && self.produces_report_field("clean_room_handoff.side_effects_all_false")
            && self.produces_report_field("clean_room_handoff.allow_report_only_continuation")
            && self.produces_report_field("clean_room_handoff.allow_runtime_continuation")
            && [
                "old_thread_read",
                "old_window_read",
                "chat_transcript_read",
                "jsonl_io",
                "file_io",
                "http_sse",
                "process_spawn",
                "validation_command_spawn",
                "daemon_control",
                "runner_state",
                "remote_mac_call",
                "model_call",
                "helper_prose_parse",
                "chat_stream",
                "prompt_parse",
                "live_write",
                "memory_store_write",
                "ndkv_write",
                "runtime_side_effect_execution",
            ]
            .iter()
            .all(|capability| self.forbids_capability(capability))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfImproveProposalAcceptancePlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_name: String,
    pub eval_entrypoints: Vec<String>,
    pub allowed_inputs: Vec<String>,
    pub produced_outputs: Vec<String>,
    pub produced_report_fields: Vec<String>,
    pub forbidden_capabilities: Vec<String>,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl SelfImproveProposalAcceptancePlan {
    pub fn self_improve_proposal_acceptance(
        name: impl Into<String>,
        stage: AdapterAcceptanceStage,
    ) -> Self {
        Self {
            name: name.into(),
            stage,
            report_schema_name: "self_improve_proposal_acceptance_v1".to_owned(),
            eval_entrypoints: vec![
                "SelfImproveProposalEvidence::clean_candidate".to_owned(),
                "SelfImproveProposalEvidence::with_validation".to_owned(),
                "SelfImproveProposalEvidence::safe_command_source".to_owned(),
                "SelfImproveProposalEvidence::clean_gist_without_raw_old_window_payload".to_owned(),
                "SelfImproveProposalEvidence::runtime_side_effects_all_false".to_owned(),
                "SelfImproveProposalEvidence::memory_admission_accepted".to_owned(),
                "SelfImproveProposalEvidence::evidence_backed_business_improvement".to_owned(),
                "SelfImproveMemoryAdmissionCandidate::accepted".to_owned(),
                "SelfImproveMemoryAdmissionCandidate::quarantined".to_owned(),
                "SelfImproveMemoryAdmissionCandidate::decided_with_reasons".to_owned(),
                "SelfImproveProposalAcceptanceGate::evaluate".to_owned(),
                "SelfImproveProposalAcceptanceReport::from_gate_and_evidence".to_owned(),
            ],
            allowed_inputs: vec![
                "SelfImproveProposalEvidence".to_owned(),
                "SelfImproveProposalAcceptanceGate".to_owned(),
                "SelfImproveMemoryAdmissionCandidate".to_owned(),
                "source_round".to_owned(),
                "evidence_ids".to_owned(),
                "validation_checked".to_owned(),
                "validation_passed".to_owned(),
                "validation_command_source".to_owned(),
                "validation_command_safe".to_owned(),
                "clean_gist".to_owned(),
                "raw_old_window_payload_present".to_owned(),
                "side_effect_attempts".to_owned(),
                "memory_admission_candidate".to_owned(),
            ],
            produced_outputs: vec![
                "SelfImproveMemoryAdmissionCandidate".to_owned(),
                "SelfImproveProposalEvidence".to_owned(),
                "GateDecision".to_owned(),
                "SelfImproveProposalAcceptanceReport".to_owned(),
            ],
            produced_report_fields: vec![
                "self_improve_proposal.source_round".to_owned(),
                "self_improve_proposal.evidence_ids".to_owned(),
                "self_improve_proposal.validation_checked".to_owned(),
                "self_improve_proposal.validation_passed".to_owned(),
                "self_improve_proposal.validation_command_source".to_owned(),
                "self_improve_proposal.clean_gist".to_owned(),
                "self_improve_proposal.memory_admission_candidate_id".to_owned(),
                "self_improve_proposal.memory_admission_decision".to_owned(),
                "self_improve_proposal.memory_admission_reasons".to_owned(),
                "self_improve_proposal.validation_passed_for_promotion".to_owned(),
                "self_improve_proposal.safe_command_source".to_owned(),
                "self_improve_proposal.clean_gist_without_raw_old_window_payload".to_owned(),
                "self_improve_proposal.no_raw_old_window_payload".to_owned(),
                "self_improve_proposal.runtime_side_effects_all_false".to_owned(),
                "self_improve_proposal.memory_admission_candidate_decided_with_reasons".to_owned(),
                "self_improve_proposal.memory_admission_accepted".to_owned(),
                "self_improve_proposal.evidence_backed_business_improvement".to_owned(),
                "self_improve_proposal.advisory_only".to_owned(),
                "self_improve_proposal.allow_promotion".to_owned(),
                "self_improve_proposal.require_repair".to_owned(),
                "self_improve_proposal.failure_reasons".to_owned(),
            ],
            forbidden_capabilities: vec![
                "old_thread_read".to_owned(),
                "old_window_read".to_owned(),
                "raw_old_window_payload".to_owned(),
                "chat_transcript_read".to_owned(),
                "jsonl_io".to_owned(),
                "file_io".to_owned(),
                "http_sse".to_owned(),
                "process_spawn".to_owned(),
                "validation_command_spawn".to_owned(),
                "daemon_control".to_owned(),
                "runner_state".to_owned(),
                "remote_mac_call".to_owned(),
                "model_download".to_owned(),
                "model_call".to_owned(),
                "helper_prose_parse".to_owned(),
                "chat_stream".to_owned(),
                "prompt_parse".to_owned(),
                "live_write".to_owned(),
                "memory_store_write".to_owned(),
                "ndkv_write".to_owned(),
                "runtime_side_effect_execution".to_owned(),
                "promotion_action_execution".to_owned(),
            ],
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        false
    }

    pub fn exposes_entrypoint(&self, entrypoint: &str) -> bool {
        self.eval_entrypoints
            .iter()
            .any(|exposed| exposed == entrypoint)
    }

    pub fn allows_input(&self, input: &str) -> bool {
        self.allowed_inputs.iter().any(|allowed| allowed == input)
    }

    pub fn produces_output(&self, output: &str) -> bool {
        self.produced_outputs
            .iter()
            .any(|produced| produced == output)
    }

    pub fn produces_report_field(&self, field: &str) -> bool {
        self.produced_report_fields
            .iter()
            .any(|produced| produced == field)
    }

    pub fn forbids_capability(&self, capability: &str) -> bool {
        self.forbidden_capabilities
            .iter()
            .any(|forbidden| forbidden == capability)
    }

    pub fn stays_pure_data_boundary(&self) -> bool {
        self.exposes_entrypoint("SelfImproveProposalEvidence::clean_candidate")
            && self.exposes_entrypoint("SelfImproveProposalAcceptanceGate::evaluate")
            && self
                .exposes_entrypoint("SelfImproveProposalAcceptanceReport::from_gate_and_evidence")
            && self.allows_input("SelfImproveProposalEvidence")
            && self.allows_input("SelfImproveProposalAcceptanceGate")
            && self.allows_input("source_round")
            && self.allows_input("evidence_ids")
            && self.allows_input("validation_passed")
            && self.allows_input("validation_command_source")
            && self.allows_input("clean_gist")
            && self.allows_input("memory_admission_candidate")
            && !self.allows_input("OldThreadReader")
            && !self.allows_input("OldWindowPayload")
            && !self.allows_input("ValidationCommandExecutor")
            && !self.allows_input("PromotionExecutor")
            && !self.allows_input("EvolutionLoopRunner")
            && self.produces_output("SelfImproveProposalAcceptanceReport")
            && self.produces_report_field("self_improve_proposal.source_round")
            && self.produces_report_field("self_improve_proposal.evidence_ids")
            && self.produces_report_field("self_improve_proposal.validation_passed")
            && self.produces_report_field("self_improve_proposal.safe_command_source")
            && self.produces_report_field(
                "self_improve_proposal.clean_gist_without_raw_old_window_payload",
            )
            && self.produces_report_field("self_improve_proposal.no_raw_old_window_payload")
            && self.produces_report_field("self_improve_proposal.runtime_side_effects_all_false")
            && self.produces_report_field(
                "self_improve_proposal.memory_admission_candidate_decided_with_reasons",
            )
            && self.produces_report_field("self_improve_proposal.allow_promotion")
            && self.produces_report_field("self_improve_proposal.require_repair")
            && [
                "old_thread_read",
                "old_window_read",
                "raw_old_window_payload",
                "chat_transcript_read",
                "jsonl_io",
                "file_io",
                "http_sse",
                "process_spawn",
                "validation_command_spawn",
                "daemon_control",
                "runner_state",
                "remote_mac_call",
                "model_download",
                "model_call",
                "helper_prose_parse",
                "chat_stream",
                "prompt_parse",
                "live_write",
                "memory_store_write",
                "ndkv_write",
                "runtime_side_effect_execution",
                "promotion_action_execution",
            ]
            .iter()
            .all(|capability| self.forbids_capability(capability))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfImproveProposalActionAssignmentPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_name: String,
    pub eval_entrypoints: Vec<String>,
    pub allowed_inputs: Vec<String>,
    pub produced_outputs: Vec<String>,
    pub produced_report_fields: Vec<String>,
    pub forbidden_capabilities: Vec<String>,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl SelfImproveProposalActionAssignmentPlan {
    pub fn self_improve_proposal_action_assignment(
        name: impl Into<String>,
        stage: AdapterAcceptanceStage,
    ) -> Self {
        Self {
            name: name.into(),
            stage,
            report_schema_name: "self_improve_proposal_action_assignment_v1".to_owned(),
            eval_entrypoints: vec![
                "SelfImproveProposalActionPlan::from_summary".to_owned(),
                "SelfImproveProposalActionAssignment::from_reports_and_plan".to_owned(),
                "SelfImproveProposalActionAssignment::first_target_digest".to_owned(),
                "SelfImproveProposalActionAssignmentFirstTargetDigest::from_target".to_owned(),
            ],
            allowed_inputs: vec![
                "SelfImproveProposalAcceptanceReport".to_owned(),
                "SelfImproveProposalAcceptanceSummaryReport".to_owned(),
                "SelfImproveProposalActionPlan".to_owned(),
                "SelfImproveProposalActionAssignmentTarget".to_owned(),
            ],
            produced_outputs: vec![
                "SelfImproveProposalActionPlan".to_owned(),
                "SelfImproveProposalActionAssignment".to_owned(),
                "SelfImproveProposalActionAssignmentTarget".to_owned(),
                "SelfImproveProposalActionAssignmentFirstTargetDigest".to_owned(),
            ],
            produced_report_fields: vec![
                "self_improve_proposal.action_assignment.action_required".to_owned(),
                "self_improve_proposal.action_assignment.primary_action".to_owned(),
                "self_improve_proposal.action_assignment.actions".to_owned(),
                "self_improve_proposal.action_assignment.target_count".to_owned(),
                "self_improve_proposal.action_assignment.requires_checked_passed_validation_and_accepted_memory_admission".to_owned(),
                "self_improve_proposal.action_assignment.first_target.proposal_id".to_owned(),
                "self_improve_proposal.action_assignment.first_target.source_round".to_owned(),
                "self_improve_proposal.action_assignment.first_target.evidence_ids".to_owned(),
                "self_improve_proposal.action_assignment.first_target.current_memory_admission_decision".to_owned(),
                "self_improve_proposal.action_assignment.first_target.validation_checked".to_owned(),
                "self_improve_proposal.action_assignment.first_target.validation_passed".to_owned(),
                "self_improve_proposal.action_assignment.first_target.memory_admission_accepted".to_owned(),
                "self_improve_proposal.action_assignment.first_target.evidence_backed_business_improvement".to_owned(),
                "self_improve_proposal.action_assignment.first_target.advisory_only".to_owned(),
                "self_improve_proposal.action_assignment.first_target.require_repair".to_owned(),
                "self_improve_proposal.action_assignment.first_target.missing_requirements".to_owned(),
            ],
            forbidden_capabilities: vec![
                "old_thread_read".to_owned(),
                "old_window_read".to_owned(),
                "raw_old_window_payload".to_owned(),
                "jsonl_io".to_owned(),
                "file_io".to_owned(),
                "http_sse".to_owned(),
                "process_spawn".to_owned(),
                "validation_command_spawn".to_owned(),
                "daemon_control".to_owned(),
                "runner_state".to_owned(),
                "remote_mac_call".to_owned(),
                "model_download".to_owned(),
                "model_call".to_owned(),
                "helper_prose_parse".to_owned(),
                "chat_stream".to_owned(),
                "prompt_parse".to_owned(),
                "live_write".to_owned(),
                "memory_store_write".to_owned(),
                "ndkv_write".to_owned(),
                "runtime_side_effect_execution".to_owned(),
                "promotion_action_execution".to_owned(),
            ],
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        false
    }

    pub fn exposes_entrypoint(&self, entrypoint: &str) -> bool {
        self.eval_entrypoints
            .iter()
            .any(|exposed| exposed == entrypoint)
    }

    pub fn allows_input(&self, input: &str) -> bool {
        self.allowed_inputs.iter().any(|allowed| allowed == input)
    }

    pub fn produces_output(&self, output: &str) -> bool {
        self.produced_outputs
            .iter()
            .any(|produced| produced == output)
    }

    pub fn produces_report_field(&self, field: &str) -> bool {
        self.produced_report_fields
            .iter()
            .any(|produced| produced == field)
    }

    pub fn forbids_capability(&self, capability: &str) -> bool {
        self.forbidden_capabilities
            .iter()
            .any(|forbidden| forbidden == capability)
    }

    pub fn stays_pure_data_boundary(&self) -> bool {
        self.exposes_entrypoint("SelfImproveProposalActionPlan::from_summary")
            && self.exposes_entrypoint("SelfImproveProposalActionAssignment::from_reports_and_plan")
            && self.exposes_entrypoint("SelfImproveProposalActionAssignment::first_target_digest")
            && self.allows_input("SelfImproveProposalAcceptanceReport")
            && self.allows_input("SelfImproveProposalActionPlan")
            && !self.allows_input("EvolutionLoopRunner")
            && !self.allows_input("ValidationCommandExecutor")
            && !self.allows_input("PromotionExecutor")
            && self.produces_output("SelfImproveProposalActionAssignment")
            && self.produces_output("SelfImproveProposalActionAssignmentFirstTargetDigest")
            && self.produces_report_field(
                "self_improve_proposal.action_assignment.first_target.proposal_id",
            )
            && self.produces_report_field(
                "self_improve_proposal.action_assignment.first_target.missing_requirements",
            )
            && [
                "old_thread_read",
                "old_window_read",
                "raw_old_window_payload",
                "jsonl_io",
                "file_io",
                "http_sse",
                "process_spawn",
                "validation_command_spawn",
                "daemon_control",
                "runner_state",
                "remote_mac_call",
                "model_download",
                "model_call",
                "helper_prose_parse",
                "chat_stream",
                "prompt_parse",
                "live_write",
                "memory_store_write",
                "ndkv_write",
                "runtime_side_effect_execution",
                "promotion_action_execution",
            ]
            .iter()
            .all(|capability| self.forbids_capability(capability))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelperStageRepairPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_name: String,
    pub eval_entrypoints: Vec<String>,
    pub allowed_inputs: Vec<String>,
    pub produced_outputs: Vec<String>,
    pub produced_report_fields: Vec<String>,
    pub forbidden_capabilities: Vec<String>,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl HelperStageRepairPlan {
    pub fn helper_stage_repair(name: impl Into<String>, stage: AdapterAcceptanceStage) -> Self {
        Self {
            name: name.into(),
            stage,
            report_schema_name: "helper_stage_repair_report_v1".to_owned(),
            eval_entrypoints: vec![
                "HelperStageRepairRoleEvidence::from_summary".to_owned(),
                "HelperStageRepairEvidence::from_role_summaries".to_owned(),
                "HelperStageRepairEvidence::with_required_roles".to_owned(),
                "HelperStageRepairGate::evaluate".to_owned(),
                "HelperStageRepairReport::from_gate_and_evidence".to_owned(),
            ],
            allowed_inputs: vec![
                "HelperStageContractSummary".to_owned(),
                "HelperStageRepairGate".to_owned(),
                "role".to_owned(),
                "required_roles".to_owned(),
                "fields".to_owned(),
                "matched_markers".to_owned(),
                "expected_markers".to_owned(),
                "latest_preview".to_owned(),
            ],
            produced_outputs: vec![
                "HelperStageRepairRoleEvidence".to_owned(),
                "HelperStageRepairEvidence".to_owned(),
                "GateDecision".to_owned(),
                "HelperStageRepairReport".to_owned(),
            ],
            produced_report_fields: vec![
                "helper_stage_repair.role_count".to_owned(),
                "helper_stage_repair.roles".to_owned(),
                "helper_stage_repair.required_roles".to_owned(),
                "helper_stage_repair.missing_required_roles".to_owned(),
                "helper_stage_repair.incomplete_roles".to_owned(),
                "helper_stage_repair.present_but_incomplete_roles".to_owned(),
                "helper_stage_repair.missing_fields_by_role".to_owned(),
                "helper_stage_repair.placeholder_fields_by_role".to_owned(),
                "helper_stage_repair.missing_markers_by_role".to_owned(),
                "helper_stage_repair.repair_actions".to_owned(),
                "helper_stage_repair.helper_stage_contract_complete".to_owned(),
                "helper_stage_repair.allow_helper_stage_acceptance".to_owned(),
                "helper_stage_repair.require_repair".to_owned(),
                "helper_stage_repair.failure_reasons".to_owned(),
            ],
            forbidden_capabilities: vec![
                "old_thread_read".to_owned(),
                "old_window_read".to_owned(),
                "chat_transcript_read".to_owned(),
                "jsonl_io".to_owned(),
                "file_io".to_owned(),
                "http_sse".to_owned(),
                "process_spawn".to_owned(),
                "validation_command_spawn".to_owned(),
                "daemon_control".to_owned(),
                "runner_state".to_owned(),
                "remote_mac_call".to_owned(),
                "model_download".to_owned(),
                "model_call".to_owned(),
                "helper_prose_parse".to_owned(),
                "chat_stream".to_owned(),
                "prompt_parse".to_owned(),
                "live_write".to_owned(),
                "memory_store_write".to_owned(),
                "ndkv_write".to_owned(),
                "runtime_side_effect_execution".to_owned(),
                "repair_action_execution".to_owned(),
            ],
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        false
    }

    pub fn exposes_entrypoint(&self, entrypoint: &str) -> bool {
        self.eval_entrypoints
            .iter()
            .any(|exposed| exposed == entrypoint)
    }

    pub fn allows_input(&self, input: &str) -> bool {
        self.allowed_inputs.iter().any(|allowed| allowed == input)
    }

    pub fn produces_output(&self, output: &str) -> bool {
        self.produced_outputs
            .iter()
            .any(|produced| produced == output)
    }

    pub fn produces_report_field(&self, field: &str) -> bool {
        self.produced_report_fields
            .iter()
            .any(|produced| produced == field)
    }

    pub fn forbids_capability(&self, capability: &str) -> bool {
        self.forbidden_capabilities
            .iter()
            .any(|forbidden| forbidden == capability)
    }

    pub fn stays_pure_data_boundary(&self) -> bool {
        self.exposes_entrypoint("HelperStageRepairRoleEvidence::from_summary")
            && self.exposes_entrypoint("HelperStageRepairEvidence::from_role_summaries")
            && self.exposes_entrypoint("HelperStageRepairEvidence::with_required_roles")
            && self.exposes_entrypoint("HelperStageRepairGate::evaluate")
            && self.exposes_entrypoint("HelperStageRepairReport::from_gate_and_evidence")
            && self.allows_input("HelperStageContractSummary")
            && self.allows_input("HelperStageRepairGate")
            && self.allows_input("role")
            && self.allows_input("required_roles")
            && self.allows_input("fields")
            && self.allows_input("matched_markers")
            && self.allows_input("expected_markers")
            && !self.allows_input("OldThreadReader")
            && !self.allows_input("OldWindowPayload")
            && !self.allows_input("ValidationCommandExecutor")
            && !self.allows_input("RepairActionExecutor")
            && !self.allows_input("EvolutionLoopRunner")
            && self.produces_output("HelperStageRepairReport")
            && self.produces_report_field("helper_stage_repair.roles")
            && self.produces_report_field("helper_stage_repair.required_roles")
            && self.produces_report_field("helper_stage_repair.missing_required_roles")
            && self.produces_report_field("helper_stage_repair.incomplete_roles")
            && self.produces_report_field("helper_stage_repair.present_but_incomplete_roles")
            && self.produces_report_field("helper_stage_repair.repair_actions")
            && self.produces_report_field("helper_stage_repair.helper_stage_contract_complete")
            && self.produces_report_field("helper_stage_repair.allow_helper_stage_acceptance")
            && self.produces_report_field("helper_stage_repair.require_repair")
            && [
                "old_thread_read",
                "old_window_read",
                "chat_transcript_read",
                "jsonl_io",
                "file_io",
                "http_sse",
                "process_spawn",
                "validation_command_spawn",
                "daemon_control",
                "runner_state",
                "remote_mac_call",
                "model_download",
                "model_call",
                "helper_prose_parse",
                "chat_stream",
                "prompt_parse",
                "live_write",
                "memory_store_write",
                "ndkv_write",
                "runtime_side_effect_execution",
                "repair_action_execution",
            ]
            .iter()
            .all(|capability| self.forbids_capability(capability))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterFixtureContractPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_name: String,
    pub required_fixture_kinds: Vec<String>,
    pub require_root_fixture: bool,
    pub require_ledger_projection: bool,
    pub require_model_worker_projection: bool,
    pub require_report_bundle_projection: bool,
    pub forbid_operational_quality_confusion: bool,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl AdapterFixtureContractPlan {
    pub fn root_adapter_fixtures(name: impl Into<String>, stage: AdapterAcceptanceStage) -> Self {
        Self {
            name: name.into(),
            stage,
            report_schema_name: "adapter_fixture_contract_report_v1".to_owned(),
            required_fixture_kinds: vec![
                "none".to_owned(),
                "chain_not_ready".to_owned(),
                "model_unavailable".to_owned(),
                "stream_or_final_missing".to_owned(),
                "runtime_response_missing".to_owned(),
                "model_quality_failure".to_owned(),
            ],
            require_root_fixture: true,
            require_ledger_projection: true,
            require_model_worker_projection: true,
            require_report_bundle_projection: true,
            forbid_operational_quality_confusion: true,
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        self.stage == AdapterAcceptanceStage::Enforced
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentRunnerCompatibilityPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_name: String,
    pub require_legacy_replay: bool,
    pub require_report_bundle: bool,
    pub require_schema_drift: bool,
    pub require_adapter_report_emission: bool,
    pub require_adapter_report_field_coverage: bool,
    pub require_adapter_future_event_coverage: bool,
    pub require_model_pool_development_window: bool,
    pub require_apple_silicon_development_effect: bool,
    pub require_feedback_self_improve: bool,
    pub require_self_evolution_continuity: bool,
    pub require_self_evolution_regression: bool,
    pub require_readiness_next_round: bool,
    pub require_self_evolution_unattended_prerequisites: bool,
    pub require_context_rot_trend: bool,
    pub require_context_rot_remediation: bool,
    pub require_rollback_resume: bool,
    pub require_adapter_fixture: bool,
    pub require_steam_case_matrix: bool,
    pub require_validation_command_coverage: bool,
    pub require_promotion_window: bool,
    pub require_handoff: bool,
    pub require_evolution_loop_tests: bool,
    pub require_workspace_tests: bool,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl CurrentRunnerCompatibilityPlan {
    pub fn before_enforced_wiring(name: impl Into<String>, stage: AdapterAcceptanceStage) -> Self {
        let enforced = stage == AdapterAcceptanceStage::Enforced;
        Self {
            name: name.into(),
            stage,
            report_schema_name: "current_runner_compatibility_report_v1".to_owned(),
            require_legacy_replay: enforced,
            require_report_bundle: enforced,
            require_schema_drift: enforced,
            require_adapter_report_emission: enforced,
            require_adapter_report_field_coverage: enforced,
            require_adapter_future_event_coverage: enforced,
            require_model_pool_development_window: enforced,
            require_apple_silicon_development_effect: enforced,
            require_feedback_self_improve: enforced,
            require_self_evolution_continuity: enforced,
            require_self_evolution_regression: enforced,
            require_readiness_next_round: enforced,
            require_self_evolution_unattended_prerequisites: enforced,
            require_context_rot_trend: enforced,
            require_context_rot_remediation: enforced,
            require_rollback_resume: enforced,
            require_adapter_fixture: enforced,
            require_steam_case_matrix: enforced,
            require_validation_command_coverage: enforced,
            require_promotion_window: enforced,
            require_handoff: enforced,
            require_evolution_loop_tests: enforced,
            require_workspace_tests: enforced,
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        self.stage == AdapterAcceptanceStage::Enforced
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterCurrentRunnerCompatibilityBoundaryPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub eval_entrypoints: Vec<String>,
    pub allowed_inputs: Vec<String>,
    pub produced_outputs: Vec<String>,
    pub forbidden_capabilities: Vec<String>,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl AdapterCurrentRunnerCompatibilityBoundaryPlan {
    pub fn current_runner_compatibility_boundary(
        name: impl Into<String>,
        stage: AdapterAcceptanceStage,
    ) -> Self {
        Self {
            name: name.into(),
            stage,
            eval_entrypoints: vec![
                "CurrentRunnerCompatibilityEvidence::all_passed".to_owned(),
                "CurrentRunnerCompatibilityEvidence::crate_only".to_owned(),
                "CurrentRunnerCompatibilityEvidence::with_adapter_report_emission_report"
                    .to_owned(),
                "CurrentRunnerCompatibilityEvidence::with_adapter_report_field_coverage_from_report"
                    .to_owned(),
                "CurrentRunnerCompatibilityEvidence::with_adapter_future_event_coverage_report"
                    .to_owned(),
                "CurrentRunnerCompatibilityEvidence::with_report_bundle_gate_report".to_owned(),
                "CurrentRunnerCompatibilityEvidence::with_schema_drift_report".to_owned(),
                "CurrentRunnerCompatibilityEvidence::with_adapter_fixture_report".to_owned(),
                "CurrentRunnerCompatibilityEvidence::with_handoff_report_pass_bits".to_owned(),
                "CurrentRunnerCompatibilityGate::for_stage".to_owned(),
                "CurrentRunnerCompatibilityGate::evaluate".to_owned(),
                "CurrentRunnerCompatibilityReport::from_gate_and_evidence".to_owned(),
            ],
            allowed_inputs: vec![
                "RootAdapterRolloutStage".to_owned(),
                "CurrentRunnerCompatibilityGate".to_owned(),
                "CurrentRunnerCompatibilityEvidence".to_owned(),
                "AdapterReportEmissionReport".to_owned(),
                "AdapterFutureEventCoverageReport".to_owned(),
                "EvalReportBundleGateReport".to_owned(),
                "EvalSchemaDriftReport".to_owned(),
                "AdapterFixtureReport".to_owned(),
                "AdapterHandoffReport".to_owned(),
                "upstream_gate_pass_bits".to_owned(),
                "test_result_pass_bits".to_owned(),
            ],
            produced_outputs: vec![
                "CurrentRunnerCompatibilityEvidence".to_owned(),
                "GateDecision".to_owned(),
                "CurrentRunnerCompatibilityReport".to_owned(),
            ],
            forbidden_capabilities: vec![
                "jsonl_io".to_owned(),
                "file_io".to_owned(),
                "http_sse".to_owned(),
                "process_spawn".to_owned(),
                "cargo_test_execution".to_owned(),
                "workspace_test_execution".to_owned(),
                "evolution_loop_test_execution".to_owned(),
                "model_call".to_owned(),
                "runner_switch_execution".to_owned(),
                "runner_wiring_execution".to_owned(),
                "runner_state_mutation".to_owned(),
                "remote_mac_call".to_owned(),
            ],
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        false
    }

    pub fn exposes_entrypoint(&self, entrypoint: &str) -> bool {
        self.eval_entrypoints
            .iter()
            .any(|exposed| exposed == entrypoint)
    }

    pub fn allows_input(&self, input: &str) -> bool {
        self.allowed_inputs.iter().any(|allowed| allowed == input)
    }

    pub fn produces_output(&self, output: &str) -> bool {
        self.produced_outputs
            .iter()
            .any(|produced| produced == output)
    }

    pub fn forbids_capability(&self, capability: &str) -> bool {
        self.forbidden_capabilities
            .iter()
            .any(|forbidden| forbidden == capability)
    }

    pub fn stays_pure_data_boundary(&self) -> bool {
        self.exposes_entrypoint("CurrentRunnerCompatibilityEvidence::all_passed")
            && self.exposes_entrypoint("CurrentRunnerCompatibilityEvidence::crate_only")
            && self.exposes_entrypoint(
                "CurrentRunnerCompatibilityEvidence::with_adapter_report_field_coverage_from_report",
            )
            && self.exposes_entrypoint(
                "CurrentRunnerCompatibilityEvidence::with_adapter_fixture_report",
            )
            && self.exposes_entrypoint(
                "CurrentRunnerCompatibilityEvidence::with_handoff_report_pass_bits",
            )
            && self.exposes_entrypoint("CurrentRunnerCompatibilityGate::for_stage")
            && self.exposes_entrypoint("CurrentRunnerCompatibilityGate::evaluate")
            && self
                .exposes_entrypoint("CurrentRunnerCompatibilityReport::from_gate_and_evidence")
            && self.allows_input("RootAdapterRolloutStage")
            && self.allows_input("CurrentRunnerCompatibilityGate")
            && self.allows_input("CurrentRunnerCompatibilityEvidence")
            && self.allows_input("AdapterReportEmissionReport")
            && self.allows_input("AdapterFixtureReport")
            && self.allows_input("AdapterHandoffReport")
            && self.allows_input("upstream_gate_pass_bits")
            && self.allows_input("test_result_pass_bits")
            && !self.allows_input("CommandExecutor")
            && !self.allows_input("EvolutionLoopRunner")
            && !self.allows_input("RunnerSwitcher")
            && !self.allows_input("JsonlReader")
            && self.produces_output("CurrentRunnerCompatibilityEvidence")
            && self.produces_output("GateDecision")
            && self.produces_output("CurrentRunnerCompatibilityReport")
            && [
                "jsonl_io",
                "file_io",
                "http_sse",
                "process_spawn",
                "cargo_test_execution",
                "workspace_test_execution",
                "evolution_loop_test_execution",
                "model_call",
                "runner_switch_execution",
                "runner_wiring_execution",
                "runner_state_mutation",
                "remote_mac_call",
            ]
            .iter()
            .all(|capability| self.forbids_capability(capability))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentRunnerCompatibilitySchemaDocumentPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_name: String,
    pub schema_document_source: String,
    pub required_document_fields: Vec<String>,
    pub required_report_only_fields: Vec<String>,
    pub required_enforced_fields: Vec<String>,
    pub required_boundary_sources: Vec<String>,
    pub forbidden_capabilities: Vec<String>,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl CurrentRunnerCompatibilitySchemaDocumentPlan {
    pub fn current_runner_compatibility_schema_document(
        name: impl Into<String>,
        stage: AdapterAcceptanceStage,
    ) -> Self {
        let report_only_fields = vec![
            "current_runner.stage".to_owned(),
            "current_runner.legacy_replay_passed".to_owned(),
            "current_runner.report_bundle_complete".to_owned(),
            "current_runner.schema_drift_passed".to_owned(),
            "current_runner.adapter_report_emission_passed".to_owned(),
            "current_runner.adapter_report_field_coverage_passed".to_owned(),
            "current_runner.adapter_future_event_coverage_passed".to_owned(),
            "current_runner.model_pool_development_window_passed".to_owned(),
            "current_runner.apple_silicon_development_effect_passed".to_owned(),
            "current_runner.feedback_self_improve_passed".to_owned(),
            "current_runner.self_evolution_continuity_passed".to_owned(),
            "current_runner.self_evolution_regression_passed".to_owned(),
            "current_runner.readiness_next_round_passed".to_owned(),
            "current_runner.self_evolution_unattended_prerequisites_passed".to_owned(),
            "current_runner.context_rot_trend_passed".to_owned(),
            "current_runner.context_rot_remediation_passed".to_owned(),
            "current_runner.rollback_resume_passed".to_owned(),
            "current_runner.adapter_fixture_passed".to_owned(),
            "current_runner.steam_case_matrix_passed".to_owned(),
            "current_runner.validation_command_coverage_passed".to_owned(),
            "current_runner.promotion_window_passed".to_owned(),
            "current_runner.handoff_passed".to_owned(),
            "current_runner.evolution_loop_tests_passed".to_owned(),
            "current_runner.workspace_tests_passed".to_owned(),
        ];
        let enforced_fields = vec![
            "current_runner.compatibility_blocked".to_owned(),
            "current_runner.failure_reasons".to_owned(),
            "current_runner.allow_enforced_wiring".to_owned(),
        ];
        let mut required_document_fields = report_only_fields.clone();
        required_document_fields.extend(enforced_fields.clone());

        Self {
            name: name.into(),
            stage,
            report_schema_name: "current_runner_compatibility_report_v1".to_owned(),
            schema_document_source:
                "CurrentRunnerCompatibilityReportSchema::current_runner_compatibility_v1".to_owned(),
            required_document_fields,
            required_report_only_fields: report_only_fields,
            required_enforced_fields: enforced_fields,
            required_boundary_sources: vec![
                "CurrentRunnerCompatibilityReportSchema::current_runner_compatibility_v1"
                    .to_owned(),
                "AdapterCurrentRunnerCompatibilityBoundaryContract::current_runner_compatibility_v1"
                    .to_owned(),
                "CurrentRunnerCompatibilityEvidence::with_handoff_report_pass_bits".to_owned(),
            ],
            forbidden_capabilities: vec![
                "jsonl_io".to_owned(),
                "file_io".to_owned(),
                "http_sse".to_owned(),
                "process_spawn".to_owned(),
                "cargo_test_execution".to_owned(),
                "workspace_test_execution".to_owned(),
                "evolution_loop_test_execution".to_owned(),
                "model_call".to_owned(),
                "runner_switch_execution".to_owned(),
                "runner_wiring_execution".to_owned(),
                "runner_state_mutation".to_owned(),
                "remote_mac_call".to_owned(),
            ],
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        false
    }

    pub fn requires_document_field(&self, field: &str) -> bool {
        self.required_document_fields
            .iter()
            .any(|required| required == field)
    }

    pub fn requires_boundary_source(&self, source: &str) -> bool {
        self.required_boundary_sources
            .iter()
            .any(|required| required == source)
    }

    pub fn forbids_capability(&self, capability: &str) -> bool {
        self.forbidden_capabilities
            .iter()
            .any(|forbidden| forbidden == capability)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterHandoffReportPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_name: String,
    pub require_runner_workspace_replay_before_enforcement: bool,
    pub require_report_only_observation_before_enforcement: bool,
    pub require_report_bundle_complete_before_enforcement: bool,
    pub require_schema_drift_before_enforcement: bool,
    pub require_adapter_report_emission_before_enforcement: bool,
    pub require_adapter_report_field_coverage_before_enforcement: bool,
    pub require_adapter_future_event_coverage_before_enforcement: bool,
    pub require_model_pool_development_window_before_enforcement: bool,
    pub require_apple_silicon_development_effect_before_enforcement: bool,
    pub require_feedback_self_improve_before_enforcement: bool,
    pub require_self_evolution_continuity_before_enforcement: bool,
    pub require_self_evolution_regression_before_enforcement: bool,
    pub require_readiness_next_round_before_enforcement: bool,
    pub require_self_evolution_unattended_prerequisites_before_enforcement: bool,
    pub require_context_rot_trend_before_enforcement: bool,
    pub require_context_rot_remediation_before_enforcement: bool,
    pub require_rollback_resume_before_enforcement: bool,
    pub require_steam_case_matrix_before_enforcement: bool,
    pub require_validation_command_coverage_before_enforcement: bool,
    pub require_promotion_window_before_enforcement: bool,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl AdapterHandoffReportPlan {
    pub fn adapter_handoff(name: impl Into<String>, stage: AdapterAcceptanceStage) -> Self {
        Self {
            name: name.into(),
            stage,
            report_schema_name: "adapter_handoff_report_v1".to_owned(),
            require_runner_workspace_replay_before_enforcement: stage
                == AdapterAcceptanceStage::Enforced,
            require_report_only_observation_before_enforcement: stage
                == AdapterAcceptanceStage::Enforced,
            require_report_bundle_complete_before_enforcement: stage
                == AdapterAcceptanceStage::Enforced,
            require_schema_drift_before_enforcement: stage == AdapterAcceptanceStage::Enforced,
            require_adapter_report_emission_before_enforcement: stage
                == AdapterAcceptanceStage::Enforced,
            require_adapter_report_field_coverage_before_enforcement: stage
                == AdapterAcceptanceStage::Enforced,
            require_adapter_future_event_coverage_before_enforcement: stage
                == AdapterAcceptanceStage::Enforced,
            require_model_pool_development_window_before_enforcement: stage
                == AdapterAcceptanceStage::Enforced,
            require_apple_silicon_development_effect_before_enforcement: stage
                == AdapterAcceptanceStage::Enforced,
            require_feedback_self_improve_before_enforcement: stage
                == AdapterAcceptanceStage::Enforced,
            require_self_evolution_continuity_before_enforcement: stage
                == AdapterAcceptanceStage::Enforced,
            require_self_evolution_regression_before_enforcement: stage
                == AdapterAcceptanceStage::Enforced,
            require_readiness_next_round_before_enforcement: stage
                == AdapterAcceptanceStage::Enforced,
            require_self_evolution_unattended_prerequisites_before_enforcement: stage
                == AdapterAcceptanceStage::Enforced,
            require_context_rot_trend_before_enforcement: stage == AdapterAcceptanceStage::Enforced,
            require_context_rot_remediation_before_enforcement: stage
                == AdapterAcceptanceStage::Enforced,
            require_rollback_resume_before_enforcement: stage == AdapterAcceptanceStage::Enforced,
            require_steam_case_matrix_before_enforcement: stage == AdapterAcceptanceStage::Enforced,
            require_validation_command_coverage_before_enforcement: stage
                == AdapterAcceptanceStage::Enforced,
            require_promotion_window_before_enforcement: stage == AdapterAcceptanceStage::Enforced,
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        self.stage == AdapterAcceptanceStage::Enforced
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterHandoffBoundaryPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub eval_entrypoints: Vec<String>,
    pub allowed_inputs: Vec<String>,
    pub produced_outputs: Vec<String>,
    pub forbidden_capabilities: Vec<String>,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl AdapterHandoffBoundaryPlan {
    pub fn handoff_boundary(name: impl Into<String>, stage: AdapterAcceptanceStage) -> Self {
        Self {
            name: name.into(),
            stage,
            eval_entrypoints: vec![
                "AdapterHandoffChecklist::before_runner_wiring".to_owned(),
                "AdapterHandoffChecklist::evaluate".to_owned(),
                "AdapterHandoffEvidence::crate_only_passed".to_owned(),
                "AdapterHandoffEvidence::full_handoff_passed".to_owned(),
                "AdapterHandoffEvidence::with_adapter_report_emission_report".to_owned(),
                "AdapterHandoffEvidence::with_adapter_report_field_coverage_from_report".to_owned(),
                "AdapterHandoffEvidence::with_adapter_future_event_coverage_report".to_owned(),
                "AdapterHandoffEvidence::with_report_bundle_gate_report".to_owned(),
                "AdapterHandoffEvidence::with_schema_drift_report".to_owned(),
                "AdapterHandoffEvidence::with_operational_gate_reports".to_owned(),
                "AdapterHandoffEvidence::with_unattended_prerequisite_report".to_owned(),
                "AdapterHandoffReport::from_checklist_and_evidence".to_owned(),
            ],
            allowed_inputs: vec![
                "RootAdapterRolloutStage".to_owned(),
                "AdapterHandoffChecklist".to_owned(),
                "AdapterHandoffEvidence".to_owned(),
                "AdapterTestGate".to_owned(),
                "AdapterReportEmissionReport".to_owned(),
                "AdapterFutureEventCoverageReport".to_owned(),
                "EvalReportBundleGateReport".to_owned(),
                "EvalSchemaDriftReport".to_owned(),
                "ContextRotTrendReport".to_owned(),
                "ContextRotRemediationReport".to_owned(),
                "RollbackResumeReport".to_owned(),
                "SteamCaseCoverageReport".to_owned(),
                "ValidationCommandCoverageReport".to_owned(),
                "SelfEvolutionUnattendedPrerequisiteReport".to_owned(),
                "test_gate_command_text".to_owned(),
                "upstream_report_pass_bits".to_owned(),
            ],
            produced_outputs: vec![
                "AdapterHandoffChecklist".to_owned(),
                "GateDecision".to_owned(),
                "AdapterHandoffReport".to_owned(),
            ],
            forbidden_capabilities: vec![
                "jsonl_io".to_owned(),
                "file_io".to_owned(),
                "http_sse".to_owned(),
                "process_spawn".to_owned(),
                "cargo_test_execution".to_owned(),
                "workspace_test_execution".to_owned(),
                "evolution_loop_test_execution".to_owned(),
                "model_call".to_owned(),
                "runner_handoff_execution".to_owned(),
                "runner_state_mutation".to_owned(),
                "remote_mac_call".to_owned(),
            ],
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        false
    }

    pub fn exposes_entrypoint(&self, entrypoint: &str) -> bool {
        self.eval_entrypoints
            .iter()
            .any(|exposed| exposed == entrypoint)
    }

    pub fn allows_input(&self, input: &str) -> bool {
        self.allowed_inputs.iter().any(|allowed| allowed == input)
    }

    pub fn produces_output(&self, output: &str) -> bool {
        self.produced_outputs
            .iter()
            .any(|produced| produced == output)
    }

    pub fn forbids_capability(&self, capability: &str) -> bool {
        self.forbidden_capabilities
            .iter()
            .any(|forbidden| forbidden == capability)
    }

    pub fn stays_pure_data_boundary(&self) -> bool {
        self.exposes_entrypoint("AdapterHandoffChecklist::before_runner_wiring")
            && self.exposes_entrypoint("AdapterHandoffChecklist::evaluate")
            && self.exposes_entrypoint("AdapterHandoffEvidence::crate_only_passed")
            && self.exposes_entrypoint("AdapterHandoffEvidence::full_handoff_passed")
            && self.exposes_entrypoint(
                "AdapterHandoffEvidence::with_adapter_report_field_coverage_from_report",
            )
            && self.exposes_entrypoint("AdapterHandoffEvidence::with_report_bundle_gate_report")
            && self.exposes_entrypoint("AdapterHandoffEvidence::with_operational_gate_reports")
            && self
                .exposes_entrypoint("AdapterHandoffEvidence::with_unattended_prerequisite_report")
            && self.exposes_entrypoint("AdapterHandoffReport::from_checklist_and_evidence")
            && self.allows_input("RootAdapterRolloutStage")
            && self.allows_input("AdapterHandoffChecklist")
            && self.allows_input("AdapterHandoffEvidence")
            && self.allows_input("AdapterTestGate")
            && self.allows_input("AdapterReportEmissionReport")
            && self.allows_input("EvalReportBundleGateReport")
            && self.allows_input("ContextRotTrendReport")
            && self.allows_input("ContextRotRemediationReport")
            && self.allows_input("RollbackResumeReport")
            && self.allows_input("SteamCaseCoverageReport")
            && self.allows_input("ValidationCommandCoverageReport")
            && self.allows_input("SelfEvolutionUnattendedPrerequisiteReport")
            && self.allows_input("test_gate_command_text")
            && self.allows_input("upstream_report_pass_bits")
            && !self.allows_input("CommandExecutor")
            && !self.allows_input("EvolutionLoopRunner")
            && !self.allows_input("RunnerSwitcher")
            && self.produces_output("AdapterHandoffChecklist")
            && self.produces_output("GateDecision")
            && self.produces_output("AdapterHandoffReport")
            && [
                "jsonl_io",
                "file_io",
                "http_sse",
                "process_spawn",
                "cargo_test_execution",
                "workspace_test_execution",
                "evolution_loop_test_execution",
                "model_call",
                "runner_handoff_execution",
                "runner_state_mutation",
                "remote_mac_call",
            ]
            .iter()
            .all(|capability| self.forbids_capability(capability))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalSchemaManifestPlan {
    pub name: String,
    pub required_schema_names: Vec<String>,
    pub verification_plan: VerificationPlan,
}

impl EvalSchemaManifestPlan {
    pub fn evolution_loop_handoff(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            required_schema_names: vec![
                "model_worker_v1".to_owned(),
                "model_worker_gate_report_v1".to_owned(),
                "worker_root_failure_consistency_report_v1".to_owned(),
                "model_pool_budget_fairness_report_v1".to_owned(),
                "model_pool_development_attribution_report_v1".to_owned(),
                "model_pool_development_window_report_v1".to_owned(),
                "apple_silicon_baseline_comparison_report_v1".to_owned(),
                "apple_silicon_development_effect_report_v1".to_owned(),
                "adapter_report_emission_report_v1".to_owned(),
                "adapter_future_event_coverage_report_v1".to_owned(),
                "ledger_gate_report_v1".to_owned(),
                "report_freshness_report_v1".to_owned(),
                "remote_runtime_acceleration_report_v1".to_owned(),
                "run_mode_report_refresh_acceptance_report_v1".to_owned(),
                "context_rot_report_v1".to_owned(),
                "context_rot_trend_report_v1".to_owned(),
                "context_rot_remediation_report_v1".to_owned(),
                "experiment_rollout_report_v1".to_owned(),
                "experiment_kill_switch_report_v1".to_owned(),
                "experiment_expansion_safety_report_v1".to_owned(),
                "experiment_switch_matrix_report_v1".to_owned(),
                "root_adapter_attribution_report_v1".to_owned(),
                "adapter_fixture_contract_report_v1".to_owned(),
                "current_runner_compatibility_report_v1".to_owned(),
                "legacy_ledger_replay_report_v1".to_owned(),
                "feedback_self_improve_report_v1".to_owned(),
                "self_evolution_continuity_report_v1".to_owned(),
                "self_evolution_regression_report_v1".to_owned(),
                "self_evolution_unattended_prerequisites_report_v1".to_owned(),
                "readiness_next_round_v1".to_owned(),
                "steam_round_report_v1".to_owned(),
                "steam_case_matrix_report_v1".to_owned(),
                "validation_command_coverage_report_v1".to_owned(),
                "rollback_report_v1".to_owned(),
                "adapter_closure_report_v1".to_owned(),
                "rollback_drill_matrix_report_v1".to_owned(),
                "adapter_handoff_report_v1".to_owned(),
                "report_bundle_gate_report_v1".to_owned(),
                "schema_drift_report_v1".to_owned(),
                "adapter_promotion_window_report_v1".to_owned(),
                "rollback_resume_report_v1".to_owned(),
            ],
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalReportBundlePlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_names: Vec<String>,
    pub verification_plan: VerificationPlan,
}

impl EvalReportBundlePlan {
    pub fn for_stage(name: impl Into<String>, stage: AdapterAcceptanceStage) -> Self {
        let report_schema_names = match stage {
            AdapterAcceptanceStage::ShadowOnly => Vec::new(),
            AdapterAcceptanceStage::ReportOnly => vec![
                "model_worker_v1".to_owned(),
                "worker_root_failure_consistency_report_v1".to_owned(),
                "apple_silicon_baseline_comparison_report_v1".to_owned(),
                "experiment_switch_matrix_report_v1".to_owned(),
                "rollback_drill_matrix_report_v1".to_owned(),
                "context_rot_report_v1".to_owned(),
                "adapter_promotion_window_report_v1".to_owned(),
            ],
            AdapterAcceptanceStage::Enforced => vec![
                "model_worker_v1".to_owned(),
                "model_worker_gate_report_v1".to_owned(),
                "worker_root_failure_consistency_report_v1".to_owned(),
                "model_pool_budget_fairness_report_v1".to_owned(),
                "model_pool_development_attribution_report_v1".to_owned(),
                "model_pool_development_window_report_v1".to_owned(),
                "apple_silicon_baseline_comparison_report_v1".to_owned(),
                "ledger_gate_report_v1".to_owned(),
                "report_freshness_report_v1".to_owned(),
                "remote_runtime_acceleration_report_v1".to_owned(),
                "run_mode_report_refresh_acceptance_report_v1".to_owned(),
                "context_rot_report_v1".to_owned(),
                "context_rot_trend_report_v1".to_owned(),
                "context_rot_remediation_report_v1".to_owned(),
                "experiment_rollout_report_v1".to_owned(),
                "experiment_kill_switch_report_v1".to_owned(),
                "experiment_expansion_safety_report_v1".to_owned(),
                "experiment_switch_matrix_report_v1".to_owned(),
                "root_adapter_attribution_report_v1".to_owned(),
                "adapter_fixture_contract_report_v1".to_owned(),
                "current_runner_compatibility_report_v1".to_owned(),
                "legacy_ledger_replay_report_v1".to_owned(),
                "feedback_self_improve_report_v1".to_owned(),
                "self_evolution_continuity_report_v1".to_owned(),
                "self_evolution_regression_report_v1".to_owned(),
                "readiness_next_round_v1".to_owned(),
                "steam_round_report_v1".to_owned(),
                "steam_case_matrix_report_v1".to_owned(),
                "validation_command_coverage_report_v1".to_owned(),
                "rollback_report_v1".to_owned(),
                "adapter_closure_report_v1".to_owned(),
                "rollback_drill_matrix_report_v1".to_owned(),
                "adapter_handoff_report_v1".to_owned(),
                "report_bundle_gate_report_v1".to_owned(),
                "schema_drift_report_v1".to_owned(),
                "adapter_promotion_window_report_v1".to_owned(),
                "rollback_resume_report_v1".to_owned(),
            ],
        };

        Self {
            name: name.into(),
            stage,
            report_schema_names,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalReportBundleGateReportPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_name: String,
    pub require_complete_bundle_before_enforcement: bool,
    pub require_adapter_report_field_coverage_before_enforcement: bool,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl EvalReportBundleGateReportPlan {
    pub fn report_bundle_gate(name: impl Into<String>, stage: AdapterAcceptanceStage) -> Self {
        Self {
            name: name.into(),
            stage,
            report_schema_name: "report_bundle_gate_report_v1".to_owned(),
            require_complete_bundle_before_enforcement: stage == AdapterAcceptanceStage::Enforced,
            require_adapter_report_field_coverage_before_enforcement: stage
                == AdapterAcceptanceStage::Enforced,
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        self.stage == AdapterAcceptanceStage::Enforced
    }

    pub fn downstream_projection_field_mappings() -> Vec<(&'static str, &'static str)> {
        vec![(
            "report_bundle.complete",
            "run_mode_report_refresh.report_bundle_complete",
        )]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterReportBundleGateBoundaryPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub eval_entrypoints: Vec<String>,
    pub allowed_inputs: Vec<String>,
    pub produced_outputs: Vec<String>,
    pub forbidden_capabilities: Vec<String>,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl AdapterReportBundleGateBoundaryPlan {
    pub fn report_bundle_gate_boundary(
        name: impl Into<String>,
        stage: AdapterAcceptanceStage,
    ) -> Self {
        Self {
            name: name.into(),
            stage,
            eval_entrypoints: vec![
                "EvalReportBundleManifest::for_stage".to_owned(),
                "EvalReportBundleEvidence::from_schema_names".to_owned(),
                "EvalReportBundleEvidence::with_adapter_report_field_coverage_from_report"
                    .to_owned(),
                "EvalReportBundleEvidence::from_adapter_report_emission_report".to_owned(),
                "EvalReportBundleManifest::evaluate_bundle".to_owned(),
                "EvalReportBundleGateReport::from_manifest_and_evidence".to_owned(),
                "EvalReportBundleGateReport::from_manifest_and_adapter_report_emission_report"
                    .to_owned(),
            ],
            allowed_inputs: vec![
                "RootAdapterRolloutStage".to_owned(),
                "observed_schema_names".to_owned(),
                "EvalReportBundleManifest".to_owned(),
                "EvalReportBundleEvidence".to_owned(),
                "adapter_report_field_coverage_passed".to_owned(),
                "AdapterReportEmissionReport::field_coverage_passed".to_owned(),
                "AdapterReportEmissionReport".to_owned(),
            ],
            produced_outputs: vec![
                "EvalReportBundleManifest".to_owned(),
                "EvalReportBundleEvidence".to_owned(),
                "GateDecision".to_owned(),
                "EvalReportBundleGateReport".to_owned(),
            ],
            forbidden_capabilities: vec![
                "jsonl_io".to_owned(),
                "file_io".to_owned(),
                "report_directory_scan".to_owned(),
                "http_sse".to_owned(),
                "process_spawn".to_owned(),
                "validation_command_spawn".to_owned(),
                "model_call".to_owned(),
                "runner_state".to_owned(),
            ],
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        false
    }

    pub fn exposes_entrypoint(&self, entrypoint: &str) -> bool {
        self.eval_entrypoints
            .iter()
            .any(|exposed| exposed == entrypoint)
    }

    pub fn allows_input(&self, input: &str) -> bool {
        self.allowed_inputs.iter().any(|allowed| allowed == input)
    }

    pub fn produces_output(&self, output: &str) -> bool {
        self.produced_outputs
            .iter()
            .any(|produced| produced == output)
    }

    pub fn forbids_capability(&self, capability: &str) -> bool {
        self.forbidden_capabilities
            .iter()
            .any(|forbidden| forbidden == capability)
    }

    pub fn stays_pure_data_boundary(&self) -> bool {
        self.exposes_entrypoint("EvalReportBundleManifest::for_stage")
            && self.exposes_entrypoint("EvalReportBundleEvidence::from_schema_names")
            && self.exposes_entrypoint(
                "EvalReportBundleEvidence::with_adapter_report_field_coverage_from_report",
            )
            && self
                .exposes_entrypoint("EvalReportBundleEvidence::from_adapter_report_emission_report")
            && self.exposes_entrypoint("EvalReportBundleManifest::evaluate_bundle")
            && self.exposes_entrypoint("EvalReportBundleGateReport::from_manifest_and_evidence")
            && self.exposes_entrypoint(
                "EvalReportBundleGateReport::from_manifest_and_adapter_report_emission_report",
            )
            && self.allows_input("RootAdapterRolloutStage")
            && self.allows_input("observed_schema_names")
            && self.allows_input("EvalReportBundleManifest")
            && self.allows_input("EvalReportBundleEvidence")
            && self.allows_input("adapter_report_field_coverage_passed")
            && self.allows_input("AdapterReportEmissionReport::field_coverage_passed")
            && self.allows_input("AdapterReportEmissionReport")
            && !self.allows_input("ReportDirectoryScanner")
            && !self.allows_input("JsonlReader")
            && !self.allows_input("EvolutionLoopRunner")
            && self.produces_output("EvalReportBundleManifest")
            && self.produces_output("EvalReportBundleEvidence")
            && self.produces_output("GateDecision")
            && self.produces_output("EvalReportBundleGateReport")
            && [
                "jsonl_io",
                "file_io",
                "report_directory_scan",
                "http_sse",
                "process_spawn",
                "validation_command_spawn",
                "model_call",
                "runner_state",
            ]
            .iter()
            .all(|capability| self.forbids_capability(capability))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalSchemaDriftReportPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_name: String,
    pub compared_contract_sources: Vec<String>,
    pub required_report_field_contract_examples: Vec<String>,
    pub require_matching_checksums_before_wiring: bool,
    pub require_matching_report_field_contract_before_wiring: bool,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl EvalSchemaDriftReportPlan {
    pub fn schema_drift(name: impl Into<String>, stage: AdapterAcceptanceStage) -> Self {
        Self {
            name: name.into(),
            stage,
            report_schema_name: "schema_drift_report_v1".to_owned(),
            compared_contract_sources: vec![
                "EvalSchemaManifest::evolution_loop_handoff_v1".to_owned(),
                "EvalReportBundleManifest::for_stage".to_owned(),
                "AdapterHandoffChecklist::before_runner_wiring".to_owned(),
                "AdapterReportEmissionPlan::required_report_fields".to_owned(),
                "AdapterClosurePureDataContract::schema_document".to_owned(),
            ],
            required_report_field_contract_examples: vec![
                "apple_silicon_effect.feedback_applied".to_owned(),
                "model_pool_attribution.validation_checked".to_owned(),
                "model_pool_budget.missing_required_roles".to_owned(),
                "context_rot_trend.latest_noisy_records".to_owned(),
                "context_rot_remediation.allow_experiment_rollout".to_owned(),
                "validation_command.strict_coverage_requested".to_owned(),
                "validation_command.coverage_tooling_evidence".to_owned(),
                "validation_command.coverage_report_evidence".to_owned(),
                "validation_command.coverage_tooling_or_report_evidence_present".to_owned(),
                "adapter_closure.allow_next_round".to_owned(),
            ],
            require_matching_checksums_before_wiring: stage == AdapterAcceptanceStage::Enforced,
            require_matching_report_field_contract_before_wiring: stage
                == AdapterAcceptanceStage::Enforced,
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        self.stage == AdapterAcceptanceStage::Enforced
    }

    pub fn requires_report_field_contract_example(&self, field: &str) -> bool {
        self.required_report_field_contract_examples
            .iter()
            .any(|required| required == field)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterSchemaDriftBoundaryPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub eval_entrypoints: Vec<String>,
    pub allowed_inputs: Vec<String>,
    pub produced_outputs: Vec<String>,
    pub forbidden_capabilities: Vec<String>,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl AdapterSchemaDriftBoundaryPlan {
    pub fn schema_drift_boundary(name: impl Into<String>, stage: AdapterAcceptanceStage) -> Self {
        Self {
            name: name.into(),
            stage,
            eval_entrypoints: vec![
                "EvalSchemaDriftEvidence::from_current_contracts".to_owned(),
                "EvalSchemaDriftEvidence::from_adapter_closure_schema_document".to_owned(),
                "EvalSchemaDriftEvidence::with_adapter_report_field_coverage_from_report"
                    .to_owned(),
                "EvalSchemaFingerprint::from_schema_names".to_owned(),
                "EvalSchemaDriftGate::evaluate".to_owned(),
                "EvalSchemaDriftReport::from_gate_and_evidence".to_owned(),
            ],
            allowed_inputs: vec![
                "RootAdapterRolloutStage".to_owned(),
                "EvalSchemaManifest".to_owned(),
                "EvalReportBundleManifest".to_owned(),
                "AdapterHandoffChecklist".to_owned(),
                "AdapterReportEmissionPlan::required_report_fields".to_owned(),
                "AdapterClosureSchemaDocument".to_owned(),
                "AdapterReportEmissionReport".to_owned(),
                "EvalSchemaDriftEvidence".to_owned(),
                "EvalSchemaDriftGate".to_owned(),
            ],
            produced_outputs: vec![
                "EvalSchemaFingerprint".to_owned(),
                "EvalSchemaDriftEvidence".to_owned(),
                "GateDecision".to_owned(),
                "EvalSchemaDriftReport".to_owned(),
            ],
            forbidden_capabilities: vec![
                "jsonl_io".to_owned(),
                "file_io".to_owned(),
                "schema_file_read".to_owned(),
                "report_directory_scan".to_owned(),
                "http_sse".to_owned(),
                "process_spawn".to_owned(),
                "validation_command_spawn".to_owned(),
                "model_call".to_owned(),
                "runner_state".to_owned(),
            ],
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        false
    }

    pub fn exposes_entrypoint(&self, entrypoint: &str) -> bool {
        self.eval_entrypoints
            .iter()
            .any(|exposed| exposed == entrypoint)
    }

    pub fn allows_input(&self, input: &str) -> bool {
        self.allowed_inputs.iter().any(|allowed| allowed == input)
    }

    pub fn produces_output(&self, output: &str) -> bool {
        self.produced_outputs
            .iter()
            .any(|produced| produced == output)
    }

    pub fn forbids_capability(&self, capability: &str) -> bool {
        self.forbidden_capabilities
            .iter()
            .any(|forbidden| forbidden == capability)
    }

    pub fn stays_pure_data_boundary(&self) -> bool {
        self.exposes_entrypoint("EvalSchemaDriftEvidence::from_current_contracts")
            && self
                .exposes_entrypoint("EvalSchemaDriftEvidence::from_adapter_closure_schema_document")
            && self.exposes_entrypoint(
                "EvalSchemaDriftEvidence::with_adapter_report_field_coverage_from_report",
            )
            && self.exposes_entrypoint("EvalSchemaFingerprint::from_schema_names")
            && self.exposes_entrypoint("EvalSchemaDriftGate::evaluate")
            && self.exposes_entrypoint("EvalSchemaDriftReport::from_gate_and_evidence")
            && self.allows_input("RootAdapterRolloutStage")
            && self.allows_input("EvalSchemaManifest")
            && self.allows_input("EvalReportBundleManifest")
            && self.allows_input("AdapterHandoffChecklist")
            && self.allows_input("AdapterReportEmissionPlan::required_report_fields")
            && self.allows_input("AdapterClosureSchemaDocument")
            && self.allows_input("AdapterReportEmissionReport")
            && self.allows_input("EvalSchemaDriftEvidence")
            && self.allows_input("EvalSchemaDriftGate")
            && !self.allows_input("SchemaFileReader")
            && !self.allows_input("ReportDirectoryScanner")
            && !self.allows_input("EvolutionLoopRunner")
            && self.produces_output("EvalSchemaFingerprint")
            && self.produces_output("EvalSchemaDriftEvidence")
            && self.produces_output("GateDecision")
            && self.produces_output("EvalSchemaDriftReport")
            && [
                "jsonl_io",
                "file_io",
                "schema_file_read",
                "report_directory_scan",
                "http_sse",
                "process_spawn",
                "validation_command_spawn",
                "model_call",
                "runner_state",
            ]
            .iter()
            .all(|capability| self.forbids_capability(capability))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterPromotionWindowReportPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_name: String,
    pub min_report_only_runs: u64,
    pub min_complete_bundle_runs: u64,
    pub min_adapter_report_emission_passed_runs: u64,
    pub min_adapter_report_field_coverage_passed_runs: u64,
    pub min_adapter_future_event_coverage_passed_runs: u64,
    pub min_apple_silicon_development_effect_passed_runs: u64,
    pub min_apple_silicon_baseline_comparison_passed_runs: u64,
    pub min_experiment_switch_matrix_passed_runs: u64,
    pub min_readiness_passed_runs: u64,
    pub min_context_rot_trend_passed_runs: u64,
    pub min_context_rot_remediation_passed_runs: u64,
    pub min_rollback_resume_passed_runs: u64,
    pub min_steam_case_matrix_passed_runs: u64,
    pub min_validation_command_coverage_passed_runs: u64,
    pub require_no_quality_failures_before_enforcement: bool,
    pub require_no_worker_quality_failures_before_enforcement: bool,
    pub require_no_worker_claim_blockers_before_enforcement: bool,
    pub require_chain_and_model_available_before_enforcement: bool,
    pub require_worker_operational_readiness_clear_before_enforcement: bool,
    pub require_adapter_report_emission_stable_before_enforcement: bool,
    pub require_adapter_report_field_coverage_stable_before_enforcement: bool,
    pub require_adapter_future_event_coverage_stable_before_enforcement: bool,
    pub require_apple_silicon_effect_stable_before_enforcement: bool,
    pub require_apple_silicon_baseline_comparison_stable_before_enforcement: bool,
    pub require_experiment_switch_matrix_stable_before_enforcement: bool,
    pub require_readiness_stable_before_enforcement: bool,
    pub require_context_rot_trend_stable_before_enforcement: bool,
    pub require_context_rot_remediation_stable_before_enforcement: bool,
    pub require_rollback_resume_stable_before_enforcement: bool,
    pub require_steam_case_matrix_stable_before_enforcement: bool,
    pub require_validation_command_coverage_stable_before_enforcement: bool,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl AdapterPromotionWindowReportPlan {
    pub fn adapter_promotion_window(
        name: impl Into<String>,
        stage: AdapterAcceptanceStage,
    ) -> Self {
        Self {
            name: name.into(),
            stage,
            report_schema_name: "adapter_promotion_window_report_v1".to_owned(),
            min_report_only_runs: 3,
            min_complete_bundle_runs: 3,
            min_adapter_report_emission_passed_runs: 3,
            min_adapter_report_field_coverage_passed_runs: 3,
            min_adapter_future_event_coverage_passed_runs: 3,
            min_apple_silicon_development_effect_passed_runs: 3,
            min_apple_silicon_baseline_comparison_passed_runs: 3,
            min_experiment_switch_matrix_passed_runs: 3,
            min_readiness_passed_runs: 3,
            min_context_rot_trend_passed_runs: 3,
            min_context_rot_remediation_passed_runs: 3,
            min_rollback_resume_passed_runs: 3,
            min_steam_case_matrix_passed_runs: 3,
            min_validation_command_coverage_passed_runs: 3,
            require_no_quality_failures_before_enforcement: stage
                == AdapterAcceptanceStage::Enforced,
            require_no_worker_quality_failures_before_enforcement: stage
                == AdapterAcceptanceStage::Enforced,
            require_no_worker_claim_blockers_before_enforcement: stage
                == AdapterAcceptanceStage::Enforced,
            require_chain_and_model_available_before_enforcement: stage
                == AdapterAcceptanceStage::Enforced,
            require_worker_operational_readiness_clear_before_enforcement: stage
                == AdapterAcceptanceStage::Enforced,
            require_adapter_report_emission_stable_before_enforcement: stage
                == AdapterAcceptanceStage::Enforced,
            require_adapter_report_field_coverage_stable_before_enforcement: stage
                == AdapterAcceptanceStage::Enforced,
            require_adapter_future_event_coverage_stable_before_enforcement: stage
                == AdapterAcceptanceStage::Enforced,
            require_apple_silicon_effect_stable_before_enforcement: stage
                == AdapterAcceptanceStage::Enforced,
            require_apple_silicon_baseline_comparison_stable_before_enforcement: stage
                == AdapterAcceptanceStage::Enforced,
            require_experiment_switch_matrix_stable_before_enforcement: stage
                == AdapterAcceptanceStage::Enforced,
            require_readiness_stable_before_enforcement: stage == AdapterAcceptanceStage::Enforced,
            require_context_rot_trend_stable_before_enforcement: stage
                == AdapterAcceptanceStage::Enforced,
            require_context_rot_remediation_stable_before_enforcement: stage
                == AdapterAcceptanceStage::Enforced,
            require_rollback_resume_stable_before_enforcement: stage
                == AdapterAcceptanceStage::Enforced,
            require_steam_case_matrix_stable_before_enforcement: stage
                == AdapterAcceptanceStage::Enforced,
            require_validation_command_coverage_stable_before_enforcement: stage
                == AdapterAcceptanceStage::Enforced,
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        self.stage == AdapterAcceptanceStage::Enforced
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterPromotionWindowBoundaryPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub eval_entrypoints: Vec<String>,
    pub allowed_inputs: Vec<String>,
    pub produced_outputs: Vec<String>,
    pub forbidden_capabilities: Vec<String>,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl AdapterPromotionWindowBoundaryPlan {
    pub fn promotion_window_boundary(
        name: impl Into<String>,
        stage: AdapterAcceptanceStage,
    ) -> Self {
        Self {
            name: name.into(),
            stage,
            eval_entrypoints: vec![
                "AdapterPromotionWindowEvidence::stable_report_only_window".to_owned(),
                "AdapterPromotionWindowEvidence::with_adapter_report_field_coverage_passed_runs_from_reports".to_owned(),
                "AdapterPromotionWindowGate::evaluate".to_owned(),
                "AdapterPromotionWindowReport::from_gate_and_evidence".to_owned(),
            ],
            allowed_inputs: vec![
                "AdapterPromotionWindowEvidence".to_owned(),
                "AdapterPromotionWindowGate".to_owned(),
                "AdapterReportEmissionReport::field_coverage_passed".to_owned(),
                "report_only_observation_counts".to_owned(),
                "gate_passed_run_counts".to_owned(),
                "root_adapter_failure_counts".to_owned(),
            ],
            produced_outputs: vec![
                "AdapterPromotionWindowEvidence".to_owned(),
                "GateDecision".to_owned(),
                "AdapterPromotionWindowReport".to_owned(),
            ],
            forbidden_capabilities: vec![
                "jsonl_io".to_owned(),
                "file_io".to_owned(),
                "report_directory_scan".to_owned(),
                "http_sse".to_owned(),
                "process_spawn".to_owned(),
                "validation_command_spawn".to_owned(),
                "model_call".to_owned(),
                "promotion_action_execution".to_owned(),
                "runner_state".to_owned(),
            ],
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn may_block_current_runner(&self) -> bool {
        false
    }

    pub fn exposes_entrypoint(&self, entrypoint: &str) -> bool {
        self.eval_entrypoints
            .iter()
            .any(|exposed| exposed == entrypoint)
    }

    pub fn allows_input(&self, input: &str) -> bool {
        self.allowed_inputs.iter().any(|allowed| allowed == input)
    }

    pub fn produces_output(&self, output: &str) -> bool {
        self.produced_outputs
            .iter()
            .any(|produced| produced == output)
    }

    pub fn forbids_capability(&self, capability: &str) -> bool {
        self.forbidden_capabilities
            .iter()
            .any(|forbidden| forbidden == capability)
    }

    pub fn stays_pure_data_boundary(&self) -> bool {
        self.exposes_entrypoint("AdapterPromotionWindowEvidence::stable_report_only_window")
            && self.exposes_entrypoint("AdapterPromotionWindowEvidence::with_adapter_report_field_coverage_passed_runs_from_reports")
            && self.exposes_entrypoint("AdapterPromotionWindowGate::evaluate")
            && self.exposes_entrypoint("AdapterPromotionWindowReport::from_gate_and_evidence")
            && self.allows_input("AdapterPromotionWindowEvidence")
            && self.allows_input("AdapterPromotionWindowGate")
            && self.allows_input("AdapterReportEmissionReport::field_coverage_passed")
            && self.allows_input("report_only_observation_counts")
            && self.allows_input("gate_passed_run_counts")
            && self.allows_input("root_adapter_failure_counts")
            && !self.allows_input("ReportDirectoryScanner")
            && !self.allows_input("JsonlReader")
            && !self.allows_input("EvolutionLoopRunner")
            && self.produces_output("AdapterPromotionWindowEvidence")
            && self.produces_output("GateDecision")
            && self.produces_output("AdapterPromotionWindowReport")
            && [
                "jsonl_io",
                "file_io",
                "report_directory_scan",
                "http_sse",
                "process_spawn",
                "validation_command_spawn",
                "model_call",
                "promotion_action_execution",
                "runner_state",
            ]
            .iter()
            .all(|capability| self.forbids_capability(capability))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanRoomContextBoundaryPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_name: String,
    pub eval_entrypoints: Vec<String>,
    pub allowed_inputs: Vec<String>,
    pub produced_outputs: Vec<String>,
    pub forbidden_capabilities: Vec<String>,
    pub required_report_fields: Vec<String>,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl CleanRoomContextBoundaryPlan {
    pub fn clean_room_context_hygiene(
        name: impl Into<String>,
        stage: AdapterAcceptanceStage,
    ) -> Self {
        Self {
            name: name.into(),
            stage,
            report_schema_name: "clean_room_context_hygiene_report_v1".to_owned(),
            eval_entrypoints: vec![
                "CleanRoomContextObservation::current_file".to_owned(),
                "CleanRoomContextObservation::coordination_tail".to_owned(),
                "CleanRoomReportOnlyContextHygieneEvidence::to_clean_room_context_evidence"
                    .to_owned(),
                "CleanRoomContextEvidence::from_observations".to_owned(),
                "CleanRoomContextGate::evaluate".to_owned(),
                "CleanRoomContextReport::from_gate_and_evidence".to_owned(),
            ],
            allowed_inputs: vec![
                "CleanRoomContextObservation".to_owned(),
                "CleanRoomReportOnlyContextHygieneEvidence".to_owned(),
                "CleanRoomContextEvidence".to_owned(),
                "CleanRoomContextGate".to_owned(),
                "current_file_evidence".to_owned(),
                "coordination_tail_evidence".to_owned(),
            ],
            produced_outputs: vec![
                "CleanRoomContextEvidence".to_owned(),
                "GateDecision".to_owned(),
                "CleanRoomContextReport".to_owned(),
            ],
            forbidden_capabilities: vec![
                "old_thread_read".to_owned(),
                "old_window_read".to_owned(),
                "raw_dialog_payload_read".to_owned(),
                "completed_window_follow_up_reuse".to_owned(),
                "chat_transcript_read".to_owned(),
                "jsonl_io".to_owned(),
                "file_io".to_owned(),
                "http_sse".to_owned(),
                "process_spawn".to_owned(),
                "validation_command_spawn".to_owned(),
                "daemon_control".to_owned(),
                "runner_state".to_owned(),
                "remote_mac_call".to_owned(),
                "model_download".to_owned(),
                "model_call".to_owned(),
                "chat_stream".to_owned(),
                "live_write".to_owned(),
                "memory_store_write".to_owned(),
                "ndkv_write".to_owned(),
                "runtime_side_effect_execution".to_owned(),
            ],
            required_report_fields: vec![
                "clean_room_context.evidence_count".to_owned(),
                "clean_room_context.allowed_evidence_labels".to_owned(),
                "clean_room_context.polluted_evidence_labels".to_owned(),
                "clean_room_context.completed_window_follow_up_labels".to_owned(),
                "clean_room_context.context_hygiene_passed".to_owned(),
                "clean_room_context.allow_clean_room_eval".to_owned(),
                "clean_room_context.failure_reasons".to_owned(),
            ],
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn exposes_entrypoint(&self, entrypoint: &str) -> bool {
        self.eval_entrypoints
            .iter()
            .any(|exposed| exposed == entrypoint)
    }

    pub fn allows_input(&self, input: &str) -> bool {
        self.allowed_inputs.iter().any(|allowed| allowed == input)
    }

    pub fn produces_output(&self, output: &str) -> bool {
        self.produced_outputs
            .iter()
            .any(|produced| produced == output)
    }

    pub fn forbids_capability(&self, capability: &str) -> bool {
        self.forbidden_capabilities
            .iter()
            .any(|forbidden| forbidden == capability)
    }

    pub fn requires_report_field(&self, field: &str) -> bool {
        self.required_report_fields
            .iter()
            .any(|required| required == field)
    }

    pub fn may_block_current_runner(&self) -> bool {
        false
    }

    pub fn stays_pure_data_boundary(&self) -> bool {
        self.report_schema_name == "clean_room_context_hygiene_report_v1"
            && self.exposes_entrypoint(
                "CleanRoomReportOnlyContextHygieneEvidence::to_clean_room_context_evidence",
            )
            && self.exposes_entrypoint("CleanRoomContextEvidence::from_observations")
            && self.exposes_entrypoint("CleanRoomContextGate::evaluate")
            && self.exposes_entrypoint("CleanRoomContextReport::from_gate_and_evidence")
            && self.allows_input("CleanRoomReportOnlyContextHygieneEvidence")
            && self.allows_input("current_file_evidence")
            && self.allows_input("coordination_tail_evidence")
            && !self.allows_input("OldThreadReader")
            && !self.allows_input("RawDialogPayload")
            && !self.allows_input("CompletedWindowFollowUp")
            && self.produces_output("CleanRoomContextReport")
            && self.requires_report_field("clean_room_context.allow_clean_room_eval")
            && [
                "old_thread_read",
                "raw_dialog_payload_read",
                "completed_window_follow_up_reuse",
                "chat_transcript_read",
                "jsonl_io",
                "file_io",
                "http_sse",
                "process_spawn",
                "daemon_control",
                "model_call",
                "chat_stream",
                "runtime_side_effect_execution",
            ]
            .iter()
            .all(|capability| self.forbids_capability(capability))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonRoundTransitionPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_name: String,
    pub eval_entrypoints: Vec<String>,
    pub allowed_inputs: Vec<String>,
    pub produced_outputs: Vec<String>,
    pub forbidden_capabilities: Vec<String>,
    pub required_report_fields: Vec<String>,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl DaemonRoundTransitionPlan {
    pub fn daemon_round_transition(name: impl Into<String>, stage: AdapterAcceptanceStage) -> Self {
        Self {
            name: name.into(),
            stage,
            report_schema_name: "daemon_round_transition_report_v1".to_owned(),
            eval_entrypoints: vec![
                "DaemonRoundTransitionEvidence::round_done_waiting_ledger_commit".to_owned(),
                "DaemonRoundTransitionEvidence::no_side_effects".to_owned(),
                "DaemonRoundTransitionGate::evaluate".to_owned(),
                "DaemonRoundTransitionReport::from_gate_and_evidence".to_owned(),
            ],
            allowed_inputs: vec![
                "DaemonRoundTransitionEvidence".to_owned(),
                "DaemonRoundTransitionGate".to_owned(),
                "latest_round_state".to_owned(),
                "done_round".to_owned(),
                "ledger_round".to_owned(),
                "round_in_progress".to_owned(),
                "reason_codes".to_owned(),
                "side_effects".to_owned(),
            ],
            produced_outputs: vec![
                "DaemonRoundTransitionEvidence".to_owned(),
                "GateDecision".to_owned(),
                "DaemonRoundTransitionReport".to_owned(),
            ],
            forbidden_capabilities: vec![
                "jsonl_io".to_owned(),
                "file_io".to_owned(),
                "http_sse".to_owned(),
                "process_spawn".to_owned(),
                "validation_command_spawn".to_owned(),
                "daemon_control".to_owned(),
                "runner_state".to_owned(),
                "remote_mac_call".to_owned(),
                "model_call".to_owned(),
                "chat_stream".to_owned(),
                "ndkv_write".to_owned(),
                "runtime_side_effect_execution".to_owned(),
            ],
            required_report_fields: vec![
                "daemon_round_transition.latest_round_state".to_owned(),
                "daemon_round_transition.done_round".to_owned(),
                "daemon_round_transition.ledger_round".to_owned(),
                "daemon_round_transition.round_in_progress".to_owned(),
                "daemon_round_transition.reason_codes".to_owned(),
                "daemon_round_transition.report_only".to_owned(),
                "daemon_round_transition.display_only".to_owned(),
                "daemon_round_transition.side_effects".to_owned(),
                "daemon_round_transition.allow_runtime_transition_action".to_owned(),
                "daemon_round_transition.failure_reasons".to_owned(),
            ],
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn exposes_entrypoint(&self, entrypoint: &str) -> bool {
        self.eval_entrypoints
            .iter()
            .any(|exposed| exposed == entrypoint)
    }

    pub fn allows_input(&self, input: &str) -> bool {
        self.allowed_inputs.iter().any(|allowed| allowed == input)
    }

    pub fn produces_output(&self, output: &str) -> bool {
        self.produced_outputs
            .iter()
            .any(|produced| produced == output)
    }

    pub fn forbids_capability(&self, capability: &str) -> bool {
        self.forbidden_capabilities
            .iter()
            .any(|forbidden| forbidden == capability)
    }

    pub fn requires_report_field(&self, field: &str) -> bool {
        self.required_report_fields
            .iter()
            .any(|required| required == field)
    }

    pub fn may_block_current_runner(&self) -> bool {
        false
    }

    pub fn stays_pure_data_boundary(&self) -> bool {
        self.report_schema_name == "daemon_round_transition_report_v1"
            && self.exposes_entrypoint(
                "DaemonRoundTransitionEvidence::round_done_waiting_ledger_commit",
            )
            && self.exposes_entrypoint("DaemonRoundTransitionGate::evaluate")
            && self.exposes_entrypoint("DaemonRoundTransitionReport::from_gate_and_evidence")
            && self.allows_input("DaemonRoundTransitionEvidence")
            && self.allows_input("latest_round_state")
            && self.allows_input("done_round")
            && self.allows_input("ledger_round")
            && self.allows_input("side_effects")
            && !self.allows_input("DaemonHandle")
            && !self.allows_input("LedgerReader")
            && !self.allows_input("EvolutionLoopRunner")
            && self.produces_output("DaemonRoundTransitionReport")
            && self.requires_report_field("daemon_round_transition.report_only")
            && self.requires_report_field("daemon_round_transition.display_only")
            && self.requires_report_field("daemon_round_transition.side_effects")
            && self.requires_report_field("daemon_round_transition.allow_runtime_transition_action")
            && [
                "jsonl_io",
                "file_io",
                "http_sse",
                "process_spawn",
                "daemon_control",
                "runner_state",
                "remote_mac_call",
                "model_call",
                "chat_stream",
                "ndkv_write",
                "runtime_side_effect_execution",
            ]
            .iter()
            .all(|capability| self.forbids_capability(capability))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveStatusBundlePlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_name: String,
    pub eval_entrypoints: Vec<String>,
    pub allowed_inputs: Vec<String>,
    pub produced_outputs: Vec<String>,
    pub forbidden_capabilities: Vec<String>,
    pub required_report_fields: Vec<String>,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl LiveStatusBundlePlan {
    pub fn live_status_bundle(name: impl Into<String>, stage: AdapterAcceptanceStage) -> Self {
        Self {
            name: name.into(),
            stage,
            report_schema_name: "live_status_bundle_report_v1".to_owned(),
            eval_entrypoints: vec![
                "LiveStatusBundleDaemonState::normal_in_progress".to_owned(),
                "LiveStatusBundleDaemonState::round_done_waiting_ledger_commit".to_owned(),
                "LiveStatusBundleDaemonState::display_state".to_owned(),
                "LiveStatusBundleReportGateReadiness::ready".to_owned(),
                "LiveStatusBundleReportGateReadiness::is_ready".to_owned(),
                "LiveStatusBundleEvidence::from_reports".to_owned(),
                "LiveStatusBundleGate::evaluate".to_owned(),
                "LiveStatusBundleReport::from_gate_and_evidence".to_owned(),
            ],
            allowed_inputs: vec![
                "LiveStatusBundleDaemonState".to_owned(),
                "CleanRoomContextReport".to_owned(),
                "LiveStatusBundleReportGateReadiness".to_owned(),
                "LiveStatusBundleGate".to_owned(),
                "transition_kind".to_owned(),
                "active_round".to_owned(),
                "ledger_latest_round".to_owned(),
                "latest_done_round".to_owned(),
                "round_in_progress".to_owned(),
                "service_cli_context_read_only".to_owned(),
                "report_gate_failure_count".to_owned(),
            ],
            produced_outputs: vec![
                "GateDecision".to_owned(),
                "LiveStatusBundleReport".to_owned(),
                "live_status_bundle_report_v1".to_owned(),
            ],
            forbidden_capabilities: vec![
                "dispatch_work".to_owned(),
                "prompt_replay".to_owned(),
                "process_start".to_owned(),
                "validation_command_spawn".to_owned(),
                "daemon_control".to_owned(),
                "runner_state".to_owned(),
                "service_call".to_owned(),
                "cli_execution".to_owned(),
                "http_sse".to_owned(),
                "jsonl_io".to_owned(),
                "file_io".to_owned(),
                "old_thread_read".to_owned(),
                "raw_dialog_payload_read".to_owned(),
                "completed_window_follow_up_reuse".to_owned(),
                "mark_polluted_window_actionable".to_owned(),
                "mark_completed_window_actionable".to_owned(),
                "remote_mac_call".to_owned(),
                "model_download".to_owned(),
                "model_call".to_owned(),
                "chat_stream".to_owned(),
                "live_write".to_owned(),
                "memory_store_write".to_owned(),
                "ndkv_write".to_owned(),
                "runtime_side_effect_execution".to_owned(),
            ],
            required_report_fields: vec![
                "live_status_bundle.display_state".to_owned(),
                "live_status_bundle.transition_kind".to_owned(),
                "live_status_bundle.active_round".to_owned(),
                "live_status_bundle.ledger_latest_round".to_owned(),
                "live_status_bundle.latest_done_round".to_owned(),
                "live_status_bundle.round_in_progress".to_owned(),
                "live_status_bundle.context_hygiene_passed".to_owned(),
                "live_status_bundle.polluted_evidence_labels".to_owned(),
                "live_status_bundle.completed_window_follow_up_labels".to_owned(),
                "live_status_bundle.report_gate_ready".to_owned(),
                "live_status_bundle.report_gate_failure_count".to_owned(),
                "live_status_bundle.service_cli_context_read_only".to_owned(),
                "live_status_bundle.allow_downstream_display".to_owned(),
                "live_status_bundle.dispatch_work_allowed".to_owned(),
                "live_status_bundle.prompt_replay_allowed".to_owned(),
                "live_status_bundle.process_start_allowed".to_owned(),
                "live_status_bundle.memory_write_allowed".to_owned(),
                "live_status_bundle.ndkv_write_allowed".to_owned(),
                "live_status_bundle.polluted_or_completed_windows_actionable".to_owned(),
                "live_status_bundle.failure_reasons".to_owned(),
            ],
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn exposes_entrypoint(&self, entrypoint: &str) -> bool {
        self.eval_entrypoints
            .iter()
            .any(|exposed| exposed == entrypoint)
    }

    pub fn allows_input(&self, input: &str) -> bool {
        self.allowed_inputs.iter().any(|allowed| allowed == input)
    }

    pub fn produces_output(&self, output: &str) -> bool {
        self.produced_outputs
            .iter()
            .any(|produced| produced == output)
    }

    pub fn forbids_capability(&self, capability: &str) -> bool {
        self.forbidden_capabilities
            .iter()
            .any(|forbidden| forbidden == capability)
    }

    pub fn requires_report_field(&self, field: &str) -> bool {
        self.required_report_fields
            .iter()
            .any(|required| required == field)
    }

    pub fn may_dispatch_work(&self) -> bool {
        false
    }

    pub fn stays_pure_data_boundary(&self) -> bool {
        self.report_schema_name == "live_status_bundle_report_v1"
            && self.exposes_entrypoint("LiveStatusBundleEvidence::from_reports")
            && self.exposes_entrypoint("LiveStatusBundleGate::evaluate")
            && self.exposes_entrypoint("LiveStatusBundleReport::from_gate_and_evidence")
            && self.allows_input("LiveStatusBundleDaemonState")
            && self.allows_input("CleanRoomContextReport")
            && self.allows_input("LiveStatusBundleReportGateReadiness")
            && !self.allows_input("DaemonHandle")
            && !self.allows_input("ServiceClient")
            && !self.allows_input("CliExecutor")
            && !self.allows_input("PromptReplayer")
            && !self.allows_input("MemoryStore")
            && self.produces_output("LiveStatusBundleReport")
            && self.requires_report_field("live_status_bundle.display_state")
            && self.requires_report_field("live_status_bundle.allow_downstream_display")
            && self.requires_report_field("live_status_bundle.dispatch_work_allowed")
            && self.requires_report_field("live_status_bundle.prompt_replay_allowed")
            && self.requires_report_field("live_status_bundle.process_start_allowed")
            && self.requires_report_field("live_status_bundle.memory_write_allowed")
            && self.requires_report_field("live_status_bundle.ndkv_write_allowed")
            && self.requires_report_field(
                "live_status_bundle.polluted_or_completed_windows_actionable",
            )
            && [
                "dispatch_work",
                "prompt_replay",
                "process_start",
                "daemon_control",
                "service_call",
                "cli_execution",
                "old_thread_read",
                "completed_window_follow_up_reuse",
                "mark_polluted_window_actionable",
                "mark_completed_window_actionable",
                "model_call",
                "chat_stream",
                "memory_store_write",
                "ndkv_write",
                "runtime_side_effect_execution",
            ]
            .iter()
            .all(|capability| self.forbids_capability(capability))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NextRoundDecisionPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_name: String,
    pub decision_statuses: Vec<String>,
    pub eval_entrypoints: Vec<String>,
    pub allowed_inputs: Vec<String>,
    pub produced_outputs: Vec<String>,
    pub forbidden_capabilities: Vec<String>,
    pub required_report_fields: Vec<String>,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl NextRoundDecisionPlan {
    pub fn next_round_decision(name: impl Into<String>, stage: AdapterAcceptanceStage) -> Self {
        Self {
            name: name.into(),
            stage,
            report_schema_name: "next_round_decision_report_v1".to_owned(),
            decision_statuses: vec![
                "safe_to_wait_current_round_active".to_owned(),
                "safe_to_continue_after_current_round".to_owned(),
                "operator_attention_blocked".to_owned(),
            ],
            eval_entrypoints: vec![
                "NextRoundDecisionEvidence::from_reports".to_owned(),
                "NextRoundDecisionEvidence::current_round_active".to_owned(),
                "NextRoundDecisionEvidence::no_side_effects".to_owned(),
                "NextRoundDecisionGate::evaluate".to_owned(),
                "NextRoundDecisionReport::from_gate_and_evidence".to_owned(),
            ],
            allowed_inputs: vec![
                "LiveStatusBundleReport".to_owned(),
                "SelfEvolutionReadinessReport".to_owned(),
                "transition_kind".to_owned(),
                "active_round".to_owned(),
                "ledger_latest_round".to_owned(),
                "latest_done_round".to_owned(),
                "readiness_can_schedule_next_round".to_owned(),
                "report_gate_ready".to_owned(),
                "context_hygiene_passed".to_owned(),
            ],
            produced_outputs: vec![
                "GateDecision".to_owned(),
                "NextRoundDecisionReport".to_owned(),
                "next_round_decision_report_v1".to_owned(),
            ],
            forbidden_capabilities: vec![
                "dispatch_work".to_owned(),
                "prompt_replay".to_owned(),
                "process_start".to_owned(),
                "validation_command_spawn".to_owned(),
                "daemon_control".to_owned(),
                "runner_state".to_owned(),
                "service_call".to_owned(),
                "cli_execution".to_owned(),
                "http_sse".to_owned(),
                "jsonl_io".to_owned(),
                "file_io".to_owned(),
                "old_thread_read".to_owned(),
                "raw_dialog_payload_read".to_owned(),
                "completed_window_follow_up_reuse".to_owned(),
                "remote_mac_call".to_owned(),
                "model_download".to_owned(),
                "model_call".to_owned(),
                "chat_stream".to_owned(),
                "memory_store_write".to_owned(),
                "ndkv_write".to_owned(),
                "runtime_side_effect_execution".to_owned(),
            ],
            required_report_fields: vec![
                "next_round_decision.decision_status".to_owned(),
                "next_round_decision.current_round_active".to_owned(),
                "next_round_decision.live_status_display_state".to_owned(),
                "next_round_decision.transition_kind".to_owned(),
                "next_round_decision.active_round".to_owned(),
                "next_round_decision.ledger_latest_round".to_owned(),
                "next_round_decision.latest_done_round".to_owned(),
                "next_round_decision.readiness_can_schedule_next_round".to_owned(),
                "next_round_decision.report_gate_ready".to_owned(),
                "next_round_decision.context_hygiene_passed".to_owned(),
                "next_round_decision.read_only".to_owned(),
                "next_round_decision.report_only".to_owned(),
                "next_round_decision.no_side_effects".to_owned(),
                "next_round_decision.dispatch_work_allowed".to_owned(),
                "next_round_decision.prompt_replay_allowed".to_owned(),
                "next_round_decision.process_start_allowed".to_owned(),
                "next_round_decision.memory_write_allowed".to_owned(),
                "next_round_decision.ndkv_write_allowed".to_owned(),
                "next_round_decision.operator_attention_required".to_owned(),
                "next_round_decision.failure_reasons".to_owned(),
            ],
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn exposes_entrypoint(&self, entrypoint: &str) -> bool {
        self.eval_entrypoints
            .iter()
            .any(|exposed| exposed == entrypoint)
    }

    pub fn allows_input(&self, input: &str) -> bool {
        self.allowed_inputs.iter().any(|allowed| allowed == input)
    }

    pub fn produces_output(&self, output: &str) -> bool {
        self.produced_outputs
            .iter()
            .any(|produced| produced == output)
    }

    pub fn forbids_capability(&self, capability: &str) -> bool {
        self.forbidden_capabilities
            .iter()
            .any(|forbidden| forbidden == capability)
    }

    pub fn requires_report_field(&self, field: &str) -> bool {
        self.required_report_fields
            .iter()
            .any(|required| required == field)
    }

    pub fn has_decision_status(&self, status: &str) -> bool {
        self.decision_statuses.iter().any(|known| known == status)
    }

    pub fn may_dispatch_work(&self) -> bool {
        false
    }

    pub fn stays_pure_data_boundary(&self) -> bool {
        self.report_schema_name == "next_round_decision_report_v1"
            && self.has_decision_status("safe_to_wait_current_round_active")
            && self.has_decision_status("safe_to_continue_after_current_round")
            && self.has_decision_status("operator_attention_blocked")
            && self.exposes_entrypoint("NextRoundDecisionEvidence::from_reports")
            && self.exposes_entrypoint("NextRoundDecisionGate::evaluate")
            && self.exposes_entrypoint("NextRoundDecisionReport::from_gate_and_evidence")
            && self.allows_input("LiveStatusBundleReport")
            && self.allows_input("SelfEvolutionReadinessReport")
            && !self.allows_input("DaemonHandle")
            && !self.allows_input("ServiceClient")
            && !self.allows_input("CliExecutor")
            && !self.allows_input("PromptReplayer")
            && !self.allows_input("MemoryStore")
            && self.produces_output("NextRoundDecisionReport")
            && self.requires_report_field("next_round_decision.decision_status")
            && self.requires_report_field("next_round_decision.read_only")
            && self.requires_report_field("next_round_decision.report_only")
            && self.requires_report_field("next_round_decision.no_side_effects")
            && self.requires_report_field("next_round_decision.dispatch_work_allowed")
            && self.requires_report_field("next_round_decision.prompt_replay_allowed")
            && self.requires_report_field("next_round_decision.process_start_allowed")
            && self.requires_report_field("next_round_decision.memory_write_allowed")
            && self.requires_report_field("next_round_decision.ndkv_write_allowed")
            && [
                "dispatch_work",
                "prompt_replay",
                "process_start",
                "daemon_control",
                "service_call",
                "cli_execution",
                "old_thread_read",
                "completed_window_follow_up_reuse",
                "model_call",
                "chat_stream",
                "memory_store_write",
                "ndkv_write",
                "runtime_side_effect_execution",
            ]
            .iter()
            .all(|capability| self.forbids_capability(capability))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NextRoundDownstreamStatusConsumersPlan {
    pub name: String,
    pub stage: AdapterAcceptanceStage,
    pub report_schema_name: String,
    pub consumers: Vec<String>,
    pub eval_entrypoints: Vec<String>,
    pub allowed_inputs: Vec<String>,
    pub produced_outputs: Vec<String>,
    pub forbidden_capabilities: Vec<String>,
    pub required_report_fields: Vec<String>,
    pub optional_report_fields: Vec<String>,
    pub preserves_legacy_runner: bool,
    pub verification_plan: VerificationPlan,
}

impl NextRoundDownstreamStatusConsumersPlan {
    pub fn downstream_status_consumers(
        name: impl Into<String>,
        stage: AdapterAcceptanceStage,
    ) -> Self {
        Self {
            name: name.into(),
            stage,
            report_schema_name: "next_round_downstream_status_consumers_v1".to_owned(),
            consumers: vec![
                "service_cli_display_status".to_owned(),
                "forge_operator_display".to_owned(),
                "agent_assignment_acceptance".to_owned(),
                "memory_self_improve_admission_visibility".to_owned(),
            ],
            eval_entrypoints: vec![
                "NextRoundDownstreamStatusEvidence::from_report".to_owned(),
                "NextRoundDownstreamStatusEvidence::no_side_effects".to_owned(),
                "NextRoundDownstreamStatusGate::evaluate".to_owned(),
                "NextRoundDownstreamStatusReport::from_gate_and_evidence".to_owned(),
                "project_next_round_decision_report_to_downstream_status".to_owned(),
            ],
            allowed_inputs: vec![
                "NextRoundDecisionReport".to_owned(),
                "normalized_next_round_status_facts".to_owned(),
                "consumer_presence_bits".to_owned(),
                "read_only".to_owned(),
                "report_only".to_owned(),
            ],
            produced_outputs: vec![
                "GateDecision".to_owned(),
                "NextRoundDownstreamStatusReport".to_owned(),
                "next_round_downstream_status_consumers_v1".to_owned(),
            ],
            forbidden_capabilities: vec![
                "dispatch_work".to_owned(),
                "prompt_replay".to_owned(),
                "process_start".to_owned(),
                "validation_command_spawn".to_owned(),
                "daemon_control".to_owned(),
                "runner_state".to_owned(),
                "service_call".to_owned(),
                "cli_execution".to_owned(),
                "forge_call".to_owned(),
                "agent_dispatch".to_owned(),
                "assignment_write".to_owned(),
                "http_sse".to_owned(),
                "jsonl_io".to_owned(),
                "file_io".to_owned(),
                "old_thread_read".to_owned(),
                "raw_dialog_payload_read".to_owned(),
                "completed_window_follow_up_reuse".to_owned(),
                "remote_mac_call".to_owned(),
                "model_download".to_owned(),
                "model_call".to_owned(),
                "chat_stream".to_owned(),
                "memory_store_write".to_owned(),
                "memory_admission_write".to_owned(),
                "ndkv_write".to_owned(),
                "runtime_side_effect_execution".to_owned(),
            ],
            required_report_fields: vec![
                "next_round_downstream.source_decision_status".to_owned(),
                "next_round_downstream.effective_decision_status".to_owned(),
                "next_round_downstream.service_cli_display_status".to_owned(),
                "next_round_downstream.forge_operator_display_status".to_owned(),
                "next_round_downstream.agent_assignment_acceptance".to_owned(),
                "next_round_downstream.memory_self_improve_admission_visibility".to_owned(),
                "next_round_downstream.operator_attention_required".to_owned(),
                "next_round_downstream.read_only".to_owned(),
                "next_round_downstream.report_only".to_owned(),
                "next_round_downstream.no_side_effects".to_owned(),
                "next_round_downstream.dispatch_work_allowed".to_owned(),
                "next_round_downstream.prompt_replay_allowed".to_owned(),
                "next_round_downstream.process_start_allowed".to_owned(),
                "next_round_downstream.memory_write_allowed".to_owned(),
                "next_round_downstream.ndkv_write_allowed".to_owned(),
            ],
            optional_report_fields: vec![
                "next_round_downstream.current_round_active".to_owned(),
                "next_round_downstream.live_status_display_state".to_owned(),
                "next_round_downstream.active_round".to_owned(),
                "next_round_downstream.ledger_latest_round".to_owned(),
                "next_round_downstream.latest_done_round".to_owned(),
                "next_round_downstream.readiness_can_schedule_next_round".to_owned(),
                "next_round_downstream.failure_reasons".to_owned(),
            ],
            preserves_legacy_runner: true,
            verification_plan: VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ),
        }
    }

    pub fn exposes_entrypoint(&self, entrypoint: &str) -> bool {
        self.eval_entrypoints
            .iter()
            .any(|exposed| exposed == entrypoint)
    }

    pub fn allows_input(&self, input: &str) -> bool {
        self.allowed_inputs.iter().any(|allowed| allowed == input)
    }

    pub fn produces_output(&self, output: &str) -> bool {
        self.produced_outputs
            .iter()
            .any(|produced| produced == output)
    }

    pub fn forbids_capability(&self, capability: &str) -> bool {
        self.forbidden_capabilities
            .iter()
            .any(|forbidden| forbidden == capability)
    }

    pub fn has_consumer(&self, consumer: &str) -> bool {
        self.consumers.iter().any(|known| known == consumer)
    }

    pub fn requires_report_field(&self, field: &str) -> bool {
        self.required_report_fields
            .iter()
            .any(|required| required == field)
    }

    pub fn optionally_reports_field(&self, field: &str) -> bool {
        self.optional_report_fields
            .iter()
            .any(|optional| optional == field)
    }

    pub fn may_dispatch_work(&self) -> bool {
        false
    }

    pub fn stays_pure_data_boundary(&self) -> bool {
        self.report_schema_name == "next_round_downstream_status_consumers_v1"
            && self.has_consumer("service_cli_display_status")
            && self.has_consumer("forge_operator_display")
            && self.has_consumer("agent_assignment_acceptance")
            && self.has_consumer("memory_self_improve_admission_visibility")
            && self.exposes_entrypoint("NextRoundDownstreamStatusEvidence::from_report")
            && self.exposes_entrypoint("NextRoundDownstreamStatusGate::evaluate")
            && self.exposes_entrypoint("NextRoundDownstreamStatusReport::from_gate_and_evidence")
            && self.exposes_entrypoint("project_next_round_decision_report_to_downstream_status")
            && self.allows_input("NextRoundDecisionReport")
            && self.allows_input("normalized_next_round_status_facts")
            && !self.allows_input("ServiceClient")
            && !self.allows_input("CliExecutor")
            && !self.allows_input("ForgeOperatorClient")
            && !self.allows_input("AgentDispatcher")
            && !self.allows_input("MemoryStore")
            && self.produces_output("NextRoundDownstreamStatusReport")
            && self.requires_report_field("next_round_downstream.service_cli_display_status")
            && self.requires_report_field("next_round_downstream.forge_operator_display_status")
            && self.requires_report_field("next_round_downstream.agent_assignment_acceptance")
            && self.requires_report_field(
                "next_round_downstream.memory_self_improve_admission_visibility",
            )
            && self.requires_report_field("next_round_downstream.no_side_effects")
            && self.requires_report_field("next_round_downstream.dispatch_work_allowed")
            && self.requires_report_field("next_round_downstream.prompt_replay_allowed")
            && self.requires_report_field("next_round_downstream.process_start_allowed")
            && self.requires_report_field("next_round_downstream.memory_write_allowed")
            && self.requires_report_field("next_round_downstream.ndkv_write_allowed")
            && self.optionally_reports_field("next_round_downstream.active_round")
            && self.optionally_reports_field("next_round_downstream.failure_reasons")
            && [
                "dispatch_work",
                "prompt_replay",
                "process_start",
                "daemon_control",
                "service_call",
                "cli_execution",
                "forge_call",
                "agent_dispatch",
                "assignment_write",
                "model_call",
                "chat_stream",
                "memory_store_write",
                "memory_admission_write",
                "ndkv_write",
                "runtime_side_effect_execution",
            ]
            .iter()
            .all(|capability| self.forbids_capability(capability))
    }
}

fn shell_quote(value: &str) -> String {
    if value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || "-_./\\:=".contains(character))
    {
        value.to_owned()
    } else {
        format!("\"{}\"", value.replace('"', "\\\""))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_room_context_boundary_plan_is_pure_data() {
        let report = CleanRoomContextBoundaryPlan::clean_room_context_hygiene(
            "clean-room-context-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = CleanRoomContextBoundaryPlan::clean_room_context_hygiene(
            "clean-room-context-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert_eq!(
            enforced.report_schema_name,
            "clean_room_context_hygiene_report_v1"
        );
        assert!(enforced.exposes_entrypoint("CleanRoomContextObservation::current_file"));
        assert!(enforced.exposes_entrypoint("CleanRoomContextObservation::coordination_tail"));
        assert!(enforced.exposes_entrypoint(
            "CleanRoomReportOnlyContextHygieneEvidence::to_clean_room_context_evidence"
        ));
        assert!(enforced.exposes_entrypoint("CleanRoomContextEvidence::from_observations"));
        assert!(enforced.exposes_entrypoint("CleanRoomContextGate::evaluate"));
        assert!(enforced.exposes_entrypoint("CleanRoomContextReport::from_gate_and_evidence"));
        assert!(enforced.allows_input("CleanRoomReportOnlyContextHygieneEvidence"));
        assert!(enforced.allows_input("current_file_evidence"));
        assert!(enforced.allows_input("coordination_tail_evidence"));
        assert!(!enforced.allows_input("OldThreadReader"));
        assert!(!enforced.allows_input("RawDialogPayload"));
        assert!(!enforced.allows_input("CompletedWindowFollowUp"));
        assert!(enforced.produces_output("CleanRoomContextEvidence"));
        assert!(enforced.produces_output("GateDecision"));
        assert!(enforced.produces_output("CleanRoomContextReport"));
        assert!(enforced.requires_report_field("clean_room_context.allowed_evidence_labels"));
        assert!(enforced.requires_report_field("clean_room_context.polluted_evidence_labels"));
        assert!(
            enforced.requires_report_field("clean_room_context.completed_window_follow_up_labels")
        );
        assert!(enforced.requires_report_field("clean_room_context.allow_clean_room_eval"));
        assert!(enforced.forbids_capability("old_thread_read"));
        assert!(enforced.forbids_capability("raw_dialog_payload_read"));
        assert!(enforced.forbids_capability("completed_window_follow_up_reuse"));
        assert!(enforced.forbids_capability("chat_transcript_read"));
        assert!(enforced.forbids_capability("jsonl_io"));
        assert!(enforced.forbids_capability("file_io"));
        assert!(enforced.forbids_capability("http_sse"));
        assert!(enforced.forbids_capability("process_spawn"));
        assert!(enforced.forbids_capability("daemon_control"));
        assert!(enforced.forbids_capability("model_call"));
        assert!(enforced.forbids_capability("chat_stream"));
        assert!(enforced.stays_pure_data_boundary());
        assert!(!report.may_block_current_runner());
        assert!(!enforced.may_block_current_runner());
        assert!(enforced.preserves_legacy_runner);
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn daemon_round_transition_plan_is_report_only_without_runtime_actions() {
        let report = DaemonRoundTransitionPlan::daemon_round_transition(
            "daemon-round-transition-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = DaemonRoundTransitionPlan::daemon_round_transition(
            "daemon-round-transition-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert_eq!(
            enforced.report_schema_name,
            "daemon_round_transition_report_v1"
        );
        assert!(
            enforced.exposes_entrypoint(
                "DaemonRoundTransitionEvidence::round_done_waiting_ledger_commit"
            )
        );
        assert!(enforced.exposes_entrypoint("DaemonRoundTransitionEvidence::no_side_effects"));
        assert!(enforced.exposes_entrypoint("DaemonRoundTransitionGate::evaluate"));
        assert!(enforced.exposes_entrypoint("DaemonRoundTransitionReport::from_gate_and_evidence"));
        assert!(enforced.allows_input("DaemonRoundTransitionEvidence"));
        assert!(enforced.allows_input("latest_round_state"));
        assert!(enforced.allows_input("done_round"));
        assert!(enforced.allows_input("ledger_round"));
        assert!(enforced.allows_input("side_effects"));
        assert!(!enforced.allows_input("DaemonHandle"));
        assert!(!enforced.allows_input("LedgerReader"));
        assert!(!enforced.allows_input("EvolutionLoopRunner"));
        assert!(enforced.produces_output("DaemonRoundTransitionReport"));
        assert!(enforced.requires_report_field("daemon_round_transition.latest_round_state"));
        assert!(enforced.requires_report_field("daemon_round_transition.report_only"));
        assert!(enforced.requires_report_field("daemon_round_transition.display_only"));
        assert!(enforced.requires_report_field("daemon_round_transition.side_effects"));
        assert!(
            enforced
                .requires_report_field("daemon_round_transition.allow_runtime_transition_action")
        );
        assert!(enforced.forbids_capability("jsonl_io"));
        assert!(enforced.forbids_capability("daemon_control"));
        assert!(enforced.forbids_capability("ndkv_write"));
        assert!(enforced.forbids_capability("runtime_side_effect_execution"));
        assert!(enforced.stays_pure_data_boundary());
        assert!(!report.may_block_current_runner());
        assert!(!enforced.may_block_current_runner());
        assert!(enforced.preserves_legacy_runner);
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn live_status_bundle_plan_is_display_only_and_never_dispatches() {
        let report = LiveStatusBundlePlan::live_status_bundle(
            "live-status-bundle",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = LiveStatusBundlePlan::live_status_bundle(
            "live-status-bundle-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert_eq!(enforced.report_schema_name, "live_status_bundle_report_v1");
        assert!(enforced.exposes_entrypoint("LiveStatusBundleDaemonState::normal_in_progress"));
        assert!(
            enforced.exposes_entrypoint(
                "LiveStatusBundleDaemonState::round_done_waiting_ledger_commit"
            )
        );
        assert!(enforced.exposes_entrypoint("LiveStatusBundleDaemonState::display_state"));
        assert!(enforced.exposes_entrypoint("LiveStatusBundleReportGateReadiness::ready"));
        assert!(enforced.exposes_entrypoint("LiveStatusBundleReportGateReadiness::is_ready"));
        assert!(enforced.exposes_entrypoint("LiveStatusBundleEvidence::from_reports"));
        assert!(enforced.exposes_entrypoint("LiveStatusBundleGate::evaluate"));
        assert!(enforced.exposes_entrypoint("LiveStatusBundleReport::from_gate_and_evidence"));
        assert!(enforced.allows_input("LiveStatusBundleDaemonState"));
        assert!(enforced.allows_input("CleanRoomContextReport"));
        assert!(enforced.allows_input("LiveStatusBundleReportGateReadiness"));
        assert!(enforced.allows_input("service_cli_context_read_only"));
        assert!(!enforced.allows_input("DaemonHandle"));
        assert!(!enforced.allows_input("ServiceClient"));
        assert!(!enforced.allows_input("CliExecutor"));
        assert!(!enforced.allows_input("PromptReplayer"));
        assert!(!enforced.allows_input("MemoryStore"));
        assert!(enforced.produces_output("LiveStatusBundleReport"));
        assert!(enforced.requires_report_field("live_status_bundle.display_state"));
        assert!(enforced.requires_report_field("live_status_bundle.transition_kind"));
        assert!(enforced.requires_report_field("live_status_bundle.active_round"));
        assert!(enforced.requires_report_field("live_status_bundle.ledger_latest_round"));
        assert!(enforced.requires_report_field("live_status_bundle.latest_done_round"));
        assert!(enforced.requires_report_field("live_status_bundle.round_in_progress"));
        assert!(enforced.requires_report_field("live_status_bundle.context_hygiene_passed"));
        assert!(enforced.requires_report_field("live_status_bundle.report_gate_ready"));
        assert!(enforced.requires_report_field("live_status_bundle.report_gate_failure_count"));
        assert!(enforced.requires_report_field("live_status_bundle.service_cli_context_read_only"));
        assert!(enforced.requires_report_field("live_status_bundle.allow_downstream_display"));
        assert!(enforced.requires_report_field("live_status_bundle.dispatch_work_allowed"));
        assert!(enforced.requires_report_field("live_status_bundle.prompt_replay_allowed"));
        assert!(enforced.requires_report_field("live_status_bundle.process_start_allowed"));
        assert!(enforced.requires_report_field("live_status_bundle.memory_write_allowed"));
        assert!(enforced.requires_report_field("live_status_bundle.ndkv_write_allowed"));
        assert!(
            enforced.requires_report_field(
                "live_status_bundle.polluted_or_completed_windows_actionable"
            )
        );
        assert!(enforced.forbids_capability("dispatch_work"));
        assert!(enforced.forbids_capability("prompt_replay"));
        assert!(enforced.forbids_capability("process_start"));
        assert!(enforced.forbids_capability("daemon_control"));
        assert!(enforced.forbids_capability("service_call"));
        assert!(enforced.forbids_capability("cli_execution"));
        assert!(enforced.forbids_capability("old_thread_read"));
        assert!(enforced.forbids_capability("completed_window_follow_up_reuse"));
        assert!(enforced.forbids_capability("mark_polluted_window_actionable"));
        assert!(enforced.forbids_capability("mark_completed_window_actionable"));
        assert!(enforced.forbids_capability("model_call"));
        assert!(enforced.forbids_capability("chat_stream"));
        assert!(enforced.forbids_capability("memory_store_write"));
        assert!(enforced.forbids_capability("ndkv_write"));
        assert!(enforced.stays_pure_data_boundary());
        assert!(!report.may_dispatch_work());
        assert!(!enforced.may_dispatch_work());
        assert!(enforced.preserves_legacy_runner);
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn next_round_decision_plan_is_report_only_and_never_dispatches() {
        let report = NextRoundDecisionPlan::next_round_decision(
            "next-round-decision",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = NextRoundDecisionPlan::next_round_decision(
            "next-round-decision-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert_eq!(enforced.report_schema_name, "next_round_decision_report_v1");
        assert!(enforced.has_decision_status("safe_to_wait_current_round_active"));
        assert!(enforced.has_decision_status("safe_to_continue_after_current_round"));
        assert!(enforced.has_decision_status("operator_attention_blocked"));
        assert!(enforced.exposes_entrypoint("NextRoundDecisionEvidence::from_reports"));
        assert!(enforced.exposes_entrypoint("NextRoundDecisionEvidence::current_round_active"));
        assert!(enforced.exposes_entrypoint("NextRoundDecisionEvidence::no_side_effects"));
        assert!(enforced.exposes_entrypoint("NextRoundDecisionGate::evaluate"));
        assert!(enforced.exposes_entrypoint("NextRoundDecisionReport::from_gate_and_evidence"));
        assert!(enforced.allows_input("LiveStatusBundleReport"));
        assert!(enforced.allows_input("SelfEvolutionReadinessReport"));
        assert!(enforced.allows_input("readiness_can_schedule_next_round"));
        assert!(enforced.allows_input("report_gate_ready"));
        assert!(enforced.allows_input("context_hygiene_passed"));
        assert!(!enforced.allows_input("DaemonHandle"));
        assert!(!enforced.allows_input("ServiceClient"));
        assert!(!enforced.allows_input("CliExecutor"));
        assert!(!enforced.allows_input("PromptReplayer"));
        assert!(!enforced.allows_input("MemoryStore"));
        assert!(enforced.produces_output("NextRoundDecisionReport"));
        assert!(enforced.requires_report_field("next_round_decision.decision_status"));
        assert!(enforced.requires_report_field("next_round_decision.current_round_active"));
        assert!(
            enforced.requires_report_field("next_round_decision.readiness_can_schedule_next_round")
        );
        assert!(enforced.requires_report_field("next_round_decision.read_only"));
        assert!(enforced.requires_report_field("next_round_decision.report_only"));
        assert!(enforced.requires_report_field("next_round_decision.no_side_effects"));
        assert!(enforced.requires_report_field("next_round_decision.dispatch_work_allowed"));
        assert!(enforced.requires_report_field("next_round_decision.prompt_replay_allowed"));
        assert!(enforced.requires_report_field("next_round_decision.process_start_allowed"));
        assert!(enforced.requires_report_field("next_round_decision.memory_write_allowed"));
        assert!(enforced.requires_report_field("next_round_decision.ndkv_write_allowed"));
        assert!(enforced.requires_report_field("next_round_decision.operator_attention_required"));
        assert!(enforced.forbids_capability("dispatch_work"));
        assert!(enforced.forbids_capability("prompt_replay"));
        assert!(enforced.forbids_capability("process_start"));
        assert!(enforced.forbids_capability("daemon_control"));
        assert!(enforced.forbids_capability("service_call"));
        assert!(enforced.forbids_capability("cli_execution"));
        assert!(enforced.forbids_capability("old_thread_read"));
        assert!(enforced.forbids_capability("completed_window_follow_up_reuse"));
        assert!(enforced.forbids_capability("model_call"));
        assert!(enforced.forbids_capability("chat_stream"));
        assert!(enforced.forbids_capability("memory_store_write"));
        assert!(enforced.forbids_capability("ndkv_write"));
        assert!(enforced.stays_pure_data_boundary());
        assert!(!report.may_dispatch_work());
        assert!(!enforced.may_dispatch_work());
        assert!(enforced.preserves_legacy_runner);
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn downstream_next_round_status_consumers_plan_names_required_and_optional_fields() {
        let report = NextRoundDownstreamStatusConsumersPlan::downstream_status_consumers(
            "next-round-downstream-status-consumers",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = NextRoundDownstreamStatusConsumersPlan::downstream_status_consumers(
            "next-round-downstream-status-consumers-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert_eq!(
            enforced.report_schema_name,
            "next_round_downstream_status_consumers_v1"
        );
        assert!(enforced.has_consumer("service_cli_display_status"));
        assert!(enforced.has_consumer("forge_operator_display"));
        assert!(enforced.has_consumer("agent_assignment_acceptance"));
        assert!(enforced.has_consumer("memory_self_improve_admission_visibility"));
        assert!(enforced.exposes_entrypoint("NextRoundDownstreamStatusEvidence::from_report"));
        assert!(enforced.exposes_entrypoint("NextRoundDownstreamStatusEvidence::no_side_effects"));
        assert!(enforced.exposes_entrypoint("NextRoundDownstreamStatusGate::evaluate"));
        assert!(
            enforced.exposes_entrypoint("NextRoundDownstreamStatusReport::from_gate_and_evidence")
        );
        assert!(
            enforced.exposes_entrypoint("project_next_round_decision_report_to_downstream_status")
        );
        assert!(enforced.allows_input("NextRoundDecisionReport"));
        assert!(enforced.allows_input("normalized_next_round_status_facts"));
        assert!(enforced.allows_input("consumer_presence_bits"));
        assert!(!enforced.allows_input("ServiceClient"));
        assert!(!enforced.allows_input("CliExecutor"));
        assert!(!enforced.allows_input("ForgeOperatorClient"));
        assert!(!enforced.allows_input("AgentDispatcher"));
        assert!(!enforced.allows_input("MemoryStore"));
        assert!(enforced.produces_output("NextRoundDownstreamStatusReport"));
        assert!(enforced.requires_report_field("next_round_downstream.source_decision_status"));
        assert!(enforced.requires_report_field("next_round_downstream.effective_decision_status"));
        assert!(enforced.requires_report_field("next_round_downstream.service_cli_display_status"));
        assert!(
            enforced.requires_report_field("next_round_downstream.forge_operator_display_status")
        );
        assert!(
            enforced.requires_report_field("next_round_downstream.agent_assignment_acceptance")
        );
        assert!(enforced.requires_report_field(
            "next_round_downstream.memory_self_improve_admission_visibility"
        ));
        assert!(enforced.requires_report_field("next_round_downstream.read_only"));
        assert!(enforced.requires_report_field("next_round_downstream.report_only"));
        assert!(enforced.requires_report_field("next_round_downstream.no_side_effects"));
        assert!(enforced.requires_report_field("next_round_downstream.dispatch_work_allowed"));
        assert!(enforced.requires_report_field("next_round_downstream.prompt_replay_allowed"));
        assert!(enforced.requires_report_field("next_round_downstream.process_start_allowed"));
        assert!(enforced.requires_report_field("next_round_downstream.memory_write_allowed"));
        assert!(enforced.requires_report_field("next_round_downstream.ndkv_write_allowed"));
        assert!(enforced.optionally_reports_field("next_round_downstream.current_round_active"));
        assert!(enforced.optionally_reports_field("next_round_downstream.active_round"));
        assert!(enforced.optionally_reports_field("next_round_downstream.ledger_latest_round"));
        assert!(enforced.optionally_reports_field("next_round_downstream.latest_done_round"));
        assert!(enforced.optionally_reports_field("next_round_downstream.failure_reasons"));
        assert!(enforced.forbids_capability("service_call"));
        assert!(enforced.forbids_capability("cli_execution"));
        assert!(enforced.forbids_capability("forge_call"));
        assert!(enforced.forbids_capability("agent_dispatch"));
        assert!(enforced.forbids_capability("assignment_write"));
        assert!(enforced.forbids_capability("memory_store_write"));
        assert!(enforced.forbids_capability("memory_admission_write"));
        assert!(enforced.forbids_capability("ndkv_write"));
        assert!(enforced.stays_pure_data_boundary());
        assert!(!report.may_dispatch_work());
        assert!(!enforced.may_dispatch_work());
        assert!(enforced.preserves_legacy_runner);
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn cargo_manifest_plan_is_data_only() {
        let plan = VerificationPlan::cargo_test_manifest(r".\crates\norion-eval\Cargo.toml");

        assert_eq!(plan.commands.len(), 1);
        assert_eq!(plan.commands[0].program, "cargo");
        assert_eq!(plan.commands[0].args[0], "test");
        assert_eq!(
            plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn smartsteam_case_defaults_to_business_cycle_stream() {
        let case = SmartSteamCase::business_cycle("steam-eval", "validate loop")
            .with_rust_check_code("pub fn ok() -> bool { true }");

        assert_eq!(case.endpoint, "/v1/business-cycle-stream");
        assert_eq!(case.max_tokens, 4096);
        assert!(case.require_feedback);
        assert!(case.rust_check_code.as_deref().unwrap().contains("ok"));
    }

    #[test]
    fn stream_continuity_requires_done_final_and_clean_buffer() {
        let check = StreamContinuityCheck {
            saw_done: false,
            saw_error: false,
            saw_final: true,
            incomplete_buffer_bytes: 0,
            delta_chars: 12,
        };

        assert!(!check.passed());
        assert_eq!(
            check.failure_reason().as_deref(),
            Some("stream truncated before done event")
        );
    }

    #[test]
    fn cli_ui_smoke_plan_links_backend_case_and_command_plan() {
        let plan = CliUiSmokePlan::new(
            "runtime-model-gated-loop",
            BackendHealthCheckPlan::local("http://127.0.0.1:7979")
                .with_min_runtime_context_tokens(262144),
            SmartSteamCase::business_cycle("case-1", "prompt"),
            VerificationPlan::cargo_check_manifest(r".\tools\evolution-loop\Cargo.toml"),
        )
        .with_web_lab_url("http://127.0.0.1:8789/");

        assert_eq!(plan.backend.min_runtime_context_tokens, Some(262144));
        assert_eq!(plan.case.endpoint, "/v1/business-cycle-stream");
        assert_eq!(plan.cli_plan.commands[0].program, "cargo");
        assert_eq!(plan.web_lab_url.as_deref(), Some("http://127.0.0.1:8789/"));
    }

    #[test]
    fn experience_audit_plan_defaults_to_strict_cleanup_gate() {
        let plan = ExperienceAuditPlan::cleanup_audit(0);

        assert_eq!(plan.endpoint, "/v1/experience-cleanup-audit");
        assert_eq!(plan.limit, 1);
        assert_eq!(plan.max_noisy_records, 0);
        assert_eq!(plan.max_noise_penalty, 0.0);
        assert_eq!(plan.max_quarantine_candidates, 0);
        assert_eq!(plan.max_repairable_legacy_metadata_lessons, 0);
        assert_eq!(plan.max_legacy_metadata_without_clean_gist, 0);
    }

    #[test]
    fn experience_audit_plan_can_describe_advisory_thresholds() {
        let plan = ExperienceAuditPlan::cleanup_audit(500)
            .with_noise_thresholds(2, 0.15)
            .with_quarantine_candidates(1)
            .with_legacy_metadata_thresholds(3, 4);

        assert_eq!(plan.limit, 500);
        assert_eq!(plan.max_noisy_records, 2);
        assert_eq!(plan.max_noise_penalty, 0.15);
        assert_eq!(plan.max_quarantine_candidates, 1);
        assert_eq!(plan.max_repairable_legacy_metadata_lessons, 3);
        assert_eq!(plan.max_legacy_metadata_without_clean_gist, 4);
    }

    #[test]
    fn validation_observation_tracks_exit_status() {
        let observation = ValidationObservation::from_outcome(
            VerificationPhase::PostRound,
            CommandOutcome {
                command_line: "cargo test".to_owned(),
                status_code: Some(101),
                elapsed_ms: 9,
                stdout_tail: String::new(),
                stderr_tail: "failed".to_owned(),
            },
        );

        assert!(observation.checked);
        assert!(!observation.passed);
    }

    #[test]
    fn model_role_names_are_stable_for_worker_adapters() {
        assert_eq!(ModelRole::Planner.as_str(), "planner");
        assert_eq!(ModelRole::Reviewer.as_str(), "reviewer");
        assert_eq!(ModelRole::Tester.as_str(), "tester");
        assert_eq!(ModelRole::Summarizer.as_str(), "summarizer");
        assert_eq!(ModelRole::HighQuality.as_str(), "high_quality");
    }

    #[test]
    fn model_worker_plan_defaults_to_non_blocking_business_cycle_worker() {
        let worker = ModelWorkerPlan::new("planner-1", ModelRole::Planner, "gemma-small")
            .with_max_tokens(512)
            .with_timeout_ms(30_000)
            .with_validation_plan(VerificationPlan::cargo_check_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ));

        assert_eq!(worker.endpoint, "/v1/business-cycle-stream");
        assert_eq!(worker.max_tokens, 512);
        assert_eq!(worker.timeout_ms, 30_000);
        assert!(!worker.may_block_primary_12b);
        assert_eq!(worker.validation_plan.commands[0].program, "cargo");
    }

    #[test]
    fn model_pool_smoke_plan_tracks_roles_and_primary_blockers() {
        let plan = ModelPoolSmokePlan::new("parallel-review", "google/gemma-4-12B-it")
            .add_worker(ModelWorkerPlan::new(
                "planner-1",
                ModelRole::Planner,
                "gemma-small",
            ))
            .add_worker(
                ModelWorkerPlan::new("high-quality-1", ModelRole::HighQuality, "gemma-12b")
                    .with_primary_blocking(true),
            )
            .with_merge_validation_plan(VerificationPlan::cargo_test_manifest(
                r".\crates\norion-eval\Cargo.toml",
            ));

        assert_eq!(plan.worker_count(), 2);
        assert!(plan.has_role(ModelRole::Planner));
        assert!(plan.has_role(ModelRole::HighQuality));
        assert_eq!(plan.primary_blocking_worker_count(), 1);
        assert_eq!(plan.merge_validation_plan.commands[0].args[0], "test");
    }

    #[test]
    fn adapter_acceptance_plan_keeps_shadow_and_report_non_blocking() {
        let model_pool = ModelPoolSmokePlan::new("single-worker-shadow", "google/gemma-4-12B-it")
            .add_worker(ModelWorkerPlan::new(
                "high-quality-1",
                ModelRole::HighQuality,
                "google/gemma-4-12B-it",
            ));
        let shadow = AdapterAcceptancePlan::root_business_cycle(
            "root-adapter-shadow",
            AdapterAcceptanceStage::ShadowOnly,
            model_pool.clone(),
        );
        let report = AdapterAcceptancePlan::root_business_cycle(
            "root-adapter-report",
            AdapterAcceptanceStage::ReportOnly,
            model_pool,
        );

        assert_eq!(
            shadow.root_business_cycle_endpoint,
            "/v1/business-cycle-stream"
        );
        assert_eq!(shadow.worker_event_name, "model_worker_v1");
        assert!(!shadow.outage_attribution_required);
        assert!(!shadow.may_block_current_runner());
        assert!(report.outage_attribution_required);
        assert!(!report.may_block_current_runner());
        assert!(report.preserves_legacy_runner);
    }

    #[test]
    fn adapter_acceptance_plan_enforced_stage_names_eval_verification() {
        let plan = AdapterAcceptancePlan::root_business_cycle(
            "root-adapter-enforced",
            AdapterAcceptanceStage::Enforced,
            ModelPoolSmokePlan::new("parallel-dev", "google/gemma-4-12B-it"),
        )
        .with_verification_plan(VerificationPlan::cargo_test_manifest(
            r".\crates\norion-test\Cargo.toml",
        ));

        assert!(plan.may_block_current_runner());
        assert!(plan.outage_attribution_required);
        assert_eq!(plan.report_schema_name, "model_pool_report_v1");
        assert_eq!(
            plan.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-test\Cargo.toml"
        );
    }

    #[test]
    fn model_pool_effectiveness_plan_names_per_worker_gate_fields() {
        let plan = ModelPoolEffectivenessPlan::apple_silicon(AdapterAcceptanceStage::ReportOnly);

        assert_eq!(plan.worker_event_name, "model_worker_v1");
        assert_eq!(plan.report_schema_name, "model_worker_gate_report_v1");
        assert!(plan.requires_field("latency_ms"));
        assert!(plan.requires_field("runtime_tokens"));
        assert!(plan.requires_field("success"));
        assert!(plan.requires_field("feedback_applied"));
        assert!(plan.requires_field("validation_checked"));
        assert!(plan.requires_field("validation_passed"));
        assert!(plan.requires_field("duplicate_output"));
        assert!(plan.requires_field("noisy_output"));
        assert!(plan.requires_field("blocked_primary_12b"));
        assert!(plan.requires_field("development_claim_allowed"));
        assert!(plan.requires_field("claim_blockers"));
        assert!(!plan.may_block_current_runner());
        assert!(plan.preserves_primary_12b);
    }

    #[test]
    fn model_pool_effectiveness_plan_keeps_operational_failures_out_of_quality() {
        let plan = ModelPoolEffectivenessPlan::apple_silicon(AdapterAcceptanceStage::Enforced);

        assert!(plan.may_block_current_runner());
        assert!(plan.treats_as_operational("chain_not_ready"));
        assert!(plan.treats_as_operational("model_unavailable"));
        assert!(!plan.treats_as_operational("model_quality_failure"));
        assert!(
            plan.enforced_worker_fields
                .contains(&"failure_kind".to_owned())
        );
    }

    #[test]
    fn model_pool_budget_fairness_plan_requires_role_contribution_before_expansion() {
        let report = ModelPoolBudgetFairnessPlan::apple_silicon(AdapterAcceptanceStage::ReportOnly);
        let enforced = ModelPoolBudgetFairnessPlan::apple_silicon(AdapterAcceptanceStage::Enforced);

        assert_eq!(
            report.report_schema_name,
            "model_pool_budget_fairness_report_v1"
        );
        assert!(report.requires_role(ModelRole::Planner));
        assert!(report.requires_role(ModelRole::Reviewer));
        assert!(report.requires_role(ModelRole::Tester));
        assert!(!report.requires_role(ModelRole::Summarizer));
        assert!(enforced.requires_report_field("model_pool_budget.roles"));
        assert!(enforced.requires_report_field("model_pool_budget.runtime_token_share_by_role"));
        assert!(enforced.requires_report_field("model_pool_budget.dominant_runtime_token_roles"));
        assert!(enforced.requires_report_field("model_pool_budget.missing_required_roles"));
        assert_eq!(report.max_role_runtime_token_share, 0.60);
        assert!(report.require_role_feedback);
        assert!(report.require_no_primary_12b_blockers);
        assert!(report.preserves_legacy_runner);
        assert!(!report.may_block_current_runner());
        assert!(enforced.may_block_current_runner());
    }

    #[test]
    fn model_pool_development_window_plan_requires_sustained_apple_silicon_gain() {
        let report =
            ModelPoolDevelopmentWindowPlan::apple_silicon(AdapterAcceptanceStage::ReportOnly);
        let enforced =
            ModelPoolDevelopmentWindowPlan::apple_silicon(AdapterAcceptanceStage::Enforced);

        assert_eq!(
            enforced.report_schema_name,
            "model_pool_development_window_report_v1"
        );
        assert_eq!(enforced.min_rounds, 3);
        assert_eq!(enforced.min_feedback_delta_total, 3);
        assert!(enforced.min_development_claim_rate >= 0.67);
        assert!(enforced.require_no_duplicate_or_noisy_output);
        assert!(enforced.require_no_primary_12b_blockers);
        assert!(enforced.keep_operational_failures_out_of_quality);
        assert!(!report.may_block_current_runner());
        assert!(enforced.may_block_current_runner());
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn apple_silicon_baseline_comparison_plan_requires_paired_gain_before_claim() {
        let report =
            AppleSiliconBaselineComparisonPlan::apple_silicon(AdapterAcceptanceStage::ReportOnly);
        let enforced =
            AppleSiliconBaselineComparisonPlan::apple_silicon(AdapterAcceptanceStage::Enforced);

        assert_eq!(
            enforced.report_schema_name,
            "apple_silicon_baseline_comparison_report_v1"
        );
        assert_eq!(enforced.min_paired_rounds, 3);
        assert_eq!(enforced.min_feedback_gain_rounds, 2);
        assert_eq!(enforced.min_feedback_delta_total, 3);
        assert_eq!(enforced.max_success_regression_rounds, 0);
        assert_eq!(enforced.max_validation_regression_rounds, 0);
        assert!(!report.require_latency_budget);
        assert!(!report.require_token_budget);
        assert!(!report.require_no_duplicate_or_noisy_output);
        assert!(!report.require_no_primary_12b_blockers);
        assert!(!report.may_block_current_runner());
        assert!(enforced.require_latency_budget);
        assert!(enforced.require_token_budget);
        assert!(enforced.require_no_duplicate_or_noisy_output);
        assert!(enforced.require_no_primary_12b_blockers);
        assert!(enforced.keep_operational_failures_out_of_quality);
        assert!(enforced.preserves_legacy_runner);
        assert!(enforced.may_block_current_runner());
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn apple_silicon_baseline_adapter_wiring_plan_keeps_baseline_events_future_gated() {
        let report = AppleSiliconBaselineAdapterWiringPlan::root_business_cycle(
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = AppleSiliconBaselineAdapterWiringPlan::root_business_cycle(
            AdapterAcceptanceStage::Enforced,
        );

        assert_eq!(
            enforced.eval_contract_name,
            "AppleSiliconBaselineAdapterPlan::root_business_cycle_json"
        );
        assert!(
            enforced
                .current_projection_fields
                .contains(&"pool_runtime_tokens_total".to_owned())
        );
        assert!(
            enforced
                .current_projection_fields
                .contains(&"root_adapter_failure_kind".to_owned())
        );
        assert!(enforced.requires_future_event("baseline_12b_feedback_applied"));
        assert!(enforced.requires_future_event("baseline_12b_latency_ms"));
        assert!(enforced.requires_future_event("baseline_12b_runtime_tokens"));
        assert!(enforced.requires_future_event("baseline_12b_success"));
        assert!(enforced.requires_future_event("baseline_12b_validation_passed"));
        assert!(!report.require_paired_baseline_events_before_enforcement);
        assert!(enforced.require_paired_baseline_events_before_enforcement);
    }

    #[test]
    fn apple_silicon_baseline_adapter_wiring_rollout_is_non_blocking_until_enforced() {
        let report = AppleSiliconBaselineAdapterWiringPlan::root_business_cycle(
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = AppleSiliconBaselineAdapterWiringPlan::root_business_cycle(
            AdapterAcceptanceStage::Enforced,
        );

        assert_eq!(
            report.rollout_steps[0],
            "shadow-project-pool-side-baseline-fields"
        );
        assert_eq!(
            report.rollout_steps[1],
            "report-only-paired-baseline-coverage"
        );
        assert!(!report.may_block_current_runner());
        assert!(enforced.may_block_current_runner());
        assert!(enforced.require_outage_attribution_before_quality);
        assert!(enforced.preserves_legacy_runner);
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn model_pool_development_attribution_plan_names_worker_metrics_and_failures() {
        let report =
            ModelPoolDevelopmentAttributionPlan::apple_silicon(AdapterAcceptanceStage::ReportOnly);
        let enforced =
            ModelPoolDevelopmentAttributionPlan::apple_silicon(AdapterAcceptanceStage::Enforced);

        assert_eq!(
            enforced.report_schema_name,
            "model_pool_development_attribution_report_v1"
        );
        assert!(enforced.requires_worker_field("latency_ms"));
        assert!(enforced.requires_worker_field("runtime_tokens"));
        assert!(enforced.requires_worker_field("success"));
        assert!(enforced.requires_worker_field("feedback_applied"));
        assert!(enforced.requires_worker_field("validation_checked"));
        assert!(enforced.requires_worker_field("validation_passed"));
        assert!(enforced.requires_worker_field("duplicate_output"));
        assert!(enforced.requires_worker_field("noisy_output"));
        assert!(enforced.requires_worker_field("blocked_primary_12b"));
        assert!(enforced.requires_worker_field("failure_kind"));
        assert!(enforced.requires_worker_field("worker_development_claim_allowed"));
        assert!(enforced.requires_worker_field("worker_claim_blockers"));
        assert!(enforced.requires_failure_kind("chain_not_ready"));
        assert!(enforced.requires_failure_kind("model_unavailable"));
        assert!(enforced.requires_failure_kind("model_quality_failure"));
        assert!(enforced.required_roles.contains(&ModelRole::Planner));
        assert!(enforced.required_roles.contains(&ModelRole::Reviewer));
        assert!(enforced.required_roles.contains(&ModelRole::Tester));
        assert!(!report.may_block_current_runner());
        assert!(enforced.may_block_current_runner());
        assert!(enforced.preserves_legacy_runner);
    }

    #[test]
    fn model_pool_development_attribution_plan_keeps_outages_out_of_quality() {
        let enforced =
            ModelPoolDevelopmentAttributionPlan::apple_silicon(AdapterAcceptanceStage::Enforced);

        assert!(enforced.require_runtime_metrics_for_success);
        assert!(enforced.require_validation_checked);
        assert!(enforced.require_feedback_applied);
        assert!(enforced.require_no_duplicate_or_noisy_output);
        assert!(enforced.require_no_primary_12b_blockers);
        assert!(enforced.keep_operational_failures_out_of_quality);
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn worker_root_failure_consistency_plan_requires_single_worker_agreement() {
        let report = WorkerRootFailureConsistencyPlan::legacy_single_worker_projection(
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = WorkerRootFailureConsistencyPlan::legacy_single_worker_projection(
            AdapterAcceptanceStage::Enforced,
        );

        assert_eq!(
            enforced.report_schema_name,
            "worker_root_failure_consistency_report_v1"
        );
        assert!(
            enforced
                .required_input_reports
                .contains(&"model_worker_v1".to_owned())
        );
        assert!(
            enforced
                .required_input_reports
                .contains(&"root_adapter_attribution_report_v1".to_owned())
        );
        assert!(!report.require_single_worker_agreement_before_enforcement);
        assert!(enforced.require_single_worker_agreement_before_enforcement);
        assert!(enforced.keep_operational_failures_out_of_quality);
        assert!(enforced.preserves_legacy_runner);
        assert!(!report.may_block_current_runner());
        assert!(enforced.may_block_current_runner());
    }

    #[test]
    fn apple_silicon_development_effect_plan_requires_inputs_before_claim() {
        let report =
            AppleSiliconDevelopmentEffectPlan::apple_silicon(AdapterAcceptanceStage::ReportOnly);
        let enforced =
            AppleSiliconDevelopmentEffectPlan::apple_silicon(AdapterAcceptanceStage::Enforced);

        assert_eq!(
            enforced.report_schema_name,
            "apple_silicon_development_effect_report_v1"
        );
        assert!(enforced.requires_input_report("model_pool_development_attribution_report_v1"));
        assert!(enforced.requires_input_report("model_pool_budget_fairness_report_v1"));
        assert!(enforced.requires_input_report("model_pool_development_window_report_v1"));
        assert!(enforced.requires_input_report("apple_silicon_baseline_comparison_report_v1"));
        assert!(enforced.requires_input_report("root_adapter_attribution_report_v1"));
        assert!(
            enforced
                .required_worker_fields
                .contains(&"latency_ms".to_owned())
        );
        assert!(
            enforced
                .required_worker_fields
                .contains(&"runtime_tokens".to_owned())
        );
        assert!(
            enforced
                .required_worker_fields
                .contains(&"duplicate_output".to_owned())
        );
        assert!(
            enforced
                .required_worker_fields
                .contains(&"noisy_output".to_owned())
        );
        assert!(
            enforced
                .required_worker_fields
                .contains(&"blocked_primary_12b".to_owned())
        );
        assert!(
            enforced
                .required_worker_fields
                .contains(&"failure_kind".to_owned())
        );
        assert!(enforced.requires_effect_report_field("apple_silicon_effect.worker_ids"));
        assert!(enforced.requires_effect_report_field("apple_silicon_effect.latency_ms"));
        assert!(enforced.requires_effect_report_field("apple_silicon_effect.runtime_tokens"));
        assert!(enforced.requires_effect_report_field("apple_silicon_effect.success"));
        assert!(enforced.requires_effect_report_field("apple_silicon_effect.feedback_applied"));
        assert!(enforced.requires_effect_report_field("apple_silicon_effect.validation_checked"));
        assert!(enforced.requires_effect_report_field("apple_silicon_effect.validation_passed"));
        assert!(enforced.requires_effect_report_field("apple_silicon_effect.duplicate_output"));
        assert!(enforced.requires_effect_report_field("apple_silicon_effect.noisy_output"));
        assert!(enforced.requires_effect_report_field("apple_silicon_effect.blocked_primary_12b"));
        assert!(enforced.requires_effect_report_field("apple_silicon_effect.failure_kinds"));
        assert!(
            enforced.requires_effect_report_field(
                "apple_silicon_effect.worker_development_claim_allowed"
            )
        );
        assert!(
            enforced.requires_effect_report_field("apple_silicon_effect.worker_claim_blockers")
        );
        assert!(
            enforced.requires_effect_report_field(
                "apple_silicon_effect.operational_readiness_failures"
            )
        );
        assert!(
            enforced.requires_effect_report_field("apple_silicon_effect.chain_not_ready_count")
        );
        assert!(
            enforced.requires_effect_report_field("apple_silicon_effect.model_unavailable_count")
        );
        assert!(
            enforced
                .requires_effect_report_field("apple_silicon_effect.model_quality_failure_count")
        );
        assert!(
            enforced.requires_effect_report_field(
                "apple_silicon_effect.reported_model_quality_failures"
            )
        );
        assert!(
            enforced
                .requires_effect_report_field("apple_silicon_effect.model_quality_failure_allowed")
        );
        assert!(enforced.requires_effect_report_field(
            "apple_silicon_effect.operational_readiness_failure_kind"
        ));
        assert!(enforced.requires_effect_report_field(
            "apple_silicon_effect.quality_failure_blocked_by_readiness_order"
        ));
        assert!(enforced.requires_effect_report_field(
            "apple_silicon_effect.operational_failure_counted_as_quality"
        ));
        assert!(enforced.requires_effect_report_field("apple_silicon_effect.duplicate_outputs"));
        assert!(enforced.requires_effect_report_field("apple_silicon_effect.noisy_outputs"));
        assert!(enforced.requires_effect_report_field("apple_silicon_effect.primary_12b_blockers"));
        assert!(
            enforced
                .requires_effect_report_field("apple_silicon_effect.worker_metric_rows_consistent")
        );
        assert!(
            enforced
                .requires_effect_report_field("apple_silicon_effect.worker_metric_coverage_passed")
        );
        assert!(
            enforced
                .requires_effect_report_field("apple_silicon_effect.validation_unchecked_workers")
        );
        assert!(
            enforced.requires_effect_report_field("apple_silicon_effect.validation_failed_workers")
        );
        assert!(enforced.requires_effect_report_field(
            "apple_silicon_effect.successful_workers_missing_runtime_metrics"
        ));
        assert!(
            enforced.requires_effect_report_field(
                "apple_silicon_effect.allow_development_effect_claim"
            )
        );
        assert!(!report.require_worker_metric_coverage);
        assert!(enforced.require_worker_metric_coverage);
        assert!(
            enforced.requires_attribution_rule("prompt_gate_blocked_and_8686_down=chain_not_ready")
        );
        assert!(
            enforced
                .requires_attribution_rule("prompt_gate_passed_and_8686_down=model_unavailable")
        );
        assert!(
            enforced.requires_attribution_rule(
                "prompt_gate_blocked_and_8686_down!=model_quality_failure"
            )
        );
        assert!(
            enforced.requires_attribution_rule(
                "prompt_gate_passed_and_8686_down!=model_quality_failure"
            )
        );
        assert!(enforced.requires_attribution_rule(
            "model_quality_failure_requires_final_json_runtime_model_tokens_failed_business_cycle"
        ));
        assert!(enforced.requires_attribution_rule(
            "chain_not_ready_or_model_unavailable_blocks_claim_not_quality"
        ));
        assert!(!report.require_budget_fairness);
        assert!(!report.require_development_window);
        assert!(enforced.require_budget_fairness);
        assert!(enforced.require_development_window);
        assert!(enforced.forbid_operational_quality_confusion);
        assert!(!report.may_block_current_runner());
        assert!(enforced.may_block_current_runner());
        assert!(enforced.preserves_legacy_runner);
    }

    #[test]
    fn adapter_report_emission_plan_orders_effect_reports_without_blocking_runner() {
        let report = AdapterReportEmissionPlan::apple_silicon_development_effect(
            AdapterAcceptanceStage::ReportOnly,
        );
        let plan = AdapterReportEmissionPlan::apple_silicon_development_effect(
            AdapterAcceptanceStage::Enforced,
        );

        assert_eq!(report.stage, AdapterAcceptanceStage::ReportOnly);
        assert!(report.requires_report_field("apple_silicon_effect.latency_ms"));
        assert!(report.requires_report_field("apple_silicon_effect.runtime_tokens"));
        assert!(report.requires_report_field("apple_silicon_effect.validation_passed"));
        assert!(report.requires_report_field("apple_silicon_effect.duplicate_output"));
        assert!(report.requires_report_field("apple_silicon_effect.noisy_output"));
        assert!(report.requires_report_field("apple_silicon_effect.blocked_primary_12b"));
        assert!(report.requires_report_field("context_rot_trend.latest_noisy_records"));
        assert!(report.requires_report_field("context_rot_remediation.allow_experiment_rollout"));
        assert!(report.requires_report_field("report_freshness.rounds"));
        assert!(report.requires_report_field("report_freshness.ledger_lag"));
        assert!(report.requires_report_field("report_freshness.fresh"));
        assert!(report.requires_report_field("report_freshness.allow_next_round"));
        assert!(report.requires_report_field("remote_runtime.acceleration_ready"));
        assert!(report.requires_report_field("remote_runtime.all_workers_metal"));
        assert!(report.requires_report_field("validation_command.strict_coverage_requested"));
        assert!(report.requires_report_field("validation_command.coverage_tooling_evidence"));
        assert!(report.requires_report_field("validation_command.coverage_report_evidence"));
        assert!(report.requires_report_field(
            "validation_command.coverage_tooling_or_report_evidence_present"
        ));
        assert!(report.requires_report_field("adapter_closure.allow_next_round"));
        assert!(!report.may_block_current_runner());
        assert!(report.preserves_legacy_runner);

        assert_eq!(plan.stage, AdapterAcceptanceStage::Enforced);
        assert!(plan.emits_after(
            "ledger_gate_report_v1",
            "root_adapter_attribution_report_v1"
        ));
        assert!(plan.emits_after("report_freshness_report_v1", "ledger_gate_report_v1"));
        assert!(plan.emits_after("remote_runtime_acceleration_report_v1", "model_worker_v1"));
        assert!(plan.emits_after(
            "run_mode_report_refresh_acceptance_report_v1",
            "report_bundle_gate_report_v1"
        ));
        assert!(plan.emits_after(
            "apple_silicon_development_effect_report_v1",
            "model_pool_development_attribution_report_v1"
        ));
        assert!(plan.emits_after(
            "apple_silicon_development_effect_report_v1",
            "model_pool_budget_fairness_report_v1"
        ));
        assert!(plan.emits_after(
            "apple_silicon_development_effect_report_v1",
            "model_pool_development_window_report_v1"
        ));
        assert!(plan.emits_after(
            "apple_silicon_baseline_comparison_report_v1",
            "model_pool_development_window_report_v1"
        ));
        assert!(plan.emits_after(
            "apple_silicon_development_effect_report_v1",
            "apple_silicon_baseline_comparison_report_v1"
        ));
        assert!(plan.requires_report_field("apple_silicon_effect.feedback_applied"));
        assert!(
            plan.requires_report_field("apple_silicon_effect.operational_readiness_failure_kind")
        );
        assert!(plan.requires_report_field("model_pool_attribution.validation_checked"));
        assert!(plan.requires_report_field("model_pool_attribution.validation_passed"));
        assert!(plan.requires_report_field("model_pool_attribution.blocked_primary_12b"));
        assert!(plan.requires_report_field("model_pool_attribution.chain_not_ready_count"));
        assert!(plan.requires_report_field("model_pool_attribution.model_unavailable_count"));
        assert!(plan.requires_report_field("model_pool_budget.missing_required_roles"));
        assert!(plan.requires_report_field("model_pool_budget.dominant_runtime_token_roles"));
        assert!(plan.requires_report_field("model_pool_budget.runtime_token_share_by_role"));
        assert!(plan.requires_report_field("ledger.allow_next_round"));
        assert!(plan.requires_report_field("report_freshness.ledger_gate_blocked"));
        assert!(plan.requires_report_field("report_freshness.fresh"));
        assert!(plan.requires_report_field("report_freshness.allow_next_round"));
        assert!(plan.requires_report_field("remote_runtime.acceleration_ready"));
        assert!(plan.requires_report_field("remote_runtime.all_workers_metal"));
        assert!(plan.requires_report_field("run_mode_report_refresh.allow_next_round"));
        assert!(plan.requires_report_field("validation_command.strict_coverage_requested"));
        assert!(plan.requires_report_field("validation_command.coverage_tooling_evidence"));
        assert!(plan.requires_report_field("validation_command.coverage_report_evidence"));
        assert!(plan.requires_report_field(
            "validation_command.coverage_tooling_or_report_evidence_present"
        ));
        assert!(plan.requires_report_field("rollback.resume_gate"));
        assert!(plan.requires_report_field("adapter_closure.allow_next_round"));
        assert!(plan.emits_after("readiness_next_round_v1", "steam_case_matrix_report_v1"));
        assert!(plan.emits_after(
            "readiness_next_round_v1",
            "validation_command_coverage_report_v1"
        ));
        assert!(plan.emits_after("readiness_next_round_v1", "rollback_resume_report_v1"));
        assert!(plan.emits_after(
            "readiness_next_round_v1",
            "apple_silicon_development_effect_report_v1"
        ));
        assert!(plan.emits_after("readiness_next_round_v1", "context_rot_report_v1"));
        assert!(plan.emits_after("readiness_next_round_v1", "context_rot_trend_report_v1"));
        assert!(plan.emits_after(
            "readiness_next_round_v1",
            "context_rot_remediation_report_v1"
        ));
        assert!(plan.emits_after("rollback_report_v1", "readiness_next_round_v1"));
        assert!(plan.emits_after("adapter_closure_report_v1", "ledger_gate_report_v1"));
        assert!(plan.emits_after(
            "adapter_closure_report_v1",
            "validation_command_coverage_report_v1"
        ));
        assert!(plan.emits_after("adapter_closure_report_v1", "readiness_next_round_v1"));
        assert!(plan.emits_after("adapter_closure_report_v1", "rollback_report_v1"));
        assert!(plan.emits_after(
            "adapter_report_emission_report_v1",
            "adapter_closure_report_v1"
        ));
        assert!(plan.emits_after(
            "adapter_promotion_window_report_v1",
            "apple_silicon_development_effect_report_v1"
        ));
        assert!(plan.emits_after(
            "adapter_promotion_window_report_v1",
            "readiness_next_round_v1"
        ));
        assert!(plan.emits_after(
            "adapter_promotion_window_report_v1",
            "report_bundle_gate_report_v1"
        ));
        assert!(plan.emits_after(
            "self_evolution_unattended_prerequisites_report_v1",
            "self_evolution_continuity_report_v1"
        ));
        assert!(plan.emits_after(
            "self_evolution_unattended_prerequisites_report_v1",
            "self_evolution_regression_report_v1"
        ));
        assert!(plan.emits_after(
            "self_evolution_unattended_prerequisites_report_v1",
            "context_rot_trend_report_v1"
        ));
        assert!(plan.emits_after(
            "self_evolution_unattended_prerequisites_report_v1",
            "context_rot_remediation_report_v1"
        ));
        assert!(plan.emits_after(
            "self_evolution_unattended_prerequisites_report_v1",
            "rollback_resume_report_v1"
        ));
        assert!(plan.emits_after(
            "self_evolution_unattended_prerequisites_report_v1",
            "adapter_promotion_window_report_v1"
        ));
        assert!(
            plan.report_schema_names
                .contains(&"adapter_report_emission_report_v1".to_owned())
        );
        assert!(
            plan.report_schema_names
                .contains(&"adapter_future_event_coverage_report_v1".to_owned())
        );
        assert!(plan.emits_after(
            "report_bundle_gate_report_v1",
            "adapter_future_event_coverage_report_v1"
        ));
        assert!(
            plan.required_future_events
                .contains(&"worker_output_fingerprint".to_owned())
        );
        assert!(
            plan.required_future_events
                .contains(&"worker_failure_kind".to_owned())
        );
        assert!(
            plan.required_future_events
                .contains(&"backend_8686_reachable".to_owned())
        );
        assert!(
            plan.required_future_events
                .contains(&"baseline_12b_feedback_applied".to_owned())
        );
        assert!(
            plan.required_future_events
                .contains(&"baseline_12b_success".to_owned())
        );
        assert!(
            plan.required_future_events
                .contains(&"baseline_12b_validation_passed".to_owned())
        );
        assert!(
            plan.required_future_events
                .contains(&"context_rot_noisy_records".to_owned())
        );
        assert!(
            plan.required_future_events
                .contains(&"context_rot_duplicate_outputs".to_owned())
        );
        assert!(
            plan.required_future_events
                .contains(&"context_rot_trend_window_rounds".to_owned())
        );
        assert!(
            plan.required_future_events
                .contains(&"context_rot_remediation_applied".to_owned())
        );
        assert!(
            plan.required_future_events
                .contains(&"steam_case_id".to_owned())
        );
        assert!(
            plan.required_future_events
                .contains(&"steam_case_endpoint".to_owned())
        );
        assert!(
            plan.required_future_events
                .contains(&"steam_case_kind".to_owned())
        );
        assert!(
            plan.required_future_events
                .contains(&"validation_command_phase".to_owned())
        );
        assert!(
            plan.required_future_events
                .contains(&"validation_command_line".to_owned())
        );
        assert!(
            plan.required_future_events
                .contains(&"validation_command_status_code".to_owned())
        );
        assert!(
            plan.required_future_events
                .contains(&"validation_output_tail".to_owned())
        );
        assert!(plan.requires_report_field("context_rot.noisy_records"));
        assert!(plan.requires_report_field("context_rot_trend.latest_noisy_records"));
        assert!(plan.requires_report_field("context_rot_trend.remediation_improved_noise"));
        assert!(plan.requires_report_field("context_rot_remediation.quarantine_candidates"));
        assert!(plan.requires_report_field("context_rot_remediation.clean_gists_backfilled"));
        assert!(plan.requires_report_field("context_rot_remediation.allow_experiment_rollout"));
        assert!(!plan.may_block_current_runner());
        assert!(plan.preserves_legacy_runner);
        assert_eq!(
            plan.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn adapter_future_event_coverage_plan_tracks_cross_contract_events() {
        let report = AdapterFutureEventCoveragePlan::apple_silicon_contracts(
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = AdapterFutureEventCoveragePlan::apple_silicon_contracts(
            AdapterAcceptanceStage::Enforced,
        );

        assert_eq!(
            enforced.report_schema_name,
            "adapter_future_event_coverage_report_v1"
        );
        assert!(
            enforced
                .source_contracts
                .contains(&"ModelWorkerLedgerAdapterPlan::evolution_loop_ledger".to_owned())
        );
        assert!(
            enforced
                .source_contracts
                .contains(&"RootBusinessCycleAdapterPlan::root_business_cycle_json".to_owned())
        );
        assert!(
            enforced
                .source_contracts
                .contains(&"SteamCaseCoverageGate".to_owned())
        );
        assert!(
            enforced
                .source_contracts
                .contains(&"ValidationCommandCoverageGate".to_owned())
        );
        assert!(enforced.requires_future_event("worker_noise_score"));
        assert!(enforced.requires_future_event("worker_failure_kind"));
        assert!(enforced.requires_future_event("backend_8686_reachable"));
        assert!(enforced.requires_future_event("baseline_12b_feedback_applied"));
        assert!(enforced.requires_future_event("baseline_12b_success"));
        assert!(enforced.requires_future_event("baseline_12b_validation_passed"));
        assert!(enforced.requires_future_event("context_rot_remediation_applied"));
        assert!(enforced.requires_future_event("steam_case_id"));
        assert!(enforced.requires_future_event("steam_case_endpoint"));
        assert!(enforced.requires_future_event("steam_case_kind"));
        assert!(enforced.requires_future_event("validation_command_phase"));
        assert!(enforced.requires_future_event("validation_command_line"));
        assert!(enforced.requires_future_event("validation_command_status_code"));
        assert!(enforced.requires_future_event("validation_output_tail"));
        assert!(!report.require_required_events_planned_before_enforcement);
        assert!(!report.may_block_current_runner());
        assert!(enforced.require_required_events_planned_before_enforcement);
        assert!(enforced.may_block_current_runner());
        assert!(enforced.preserves_legacy_runner);
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn context_rot_acceptance_plan_reports_before_it_blocks() {
        let report = ContextRotAcceptancePlan::experience_audit(
            "context-rot-report",
            AdapterAcceptanceStage::ReportOnly,
        )
        .with_audit_plan(
            ExperienceAuditPlan::cleanup_audit(250)
                .with_noise_thresholds(2, 0.05)
                .with_quarantine_candidates(1),
        );

        assert_eq!(report.audit_plan.endpoint, "/v1/experience-cleanup-audit");
        assert_eq!(report.audit_plan.limit, 250);
        assert_eq!(report.report_schema_name, "context_rot_report_v1");
        assert!(!report.blocks_experiment_rollout);
        assert!(!report.may_block_current_runner());
        assert!(report.preserves_legacy_runner);
    }

    #[test]
    fn context_rot_acceptance_plan_enforced_stage_can_block_rollout() {
        let enforced = ContextRotAcceptancePlan::experience_audit(
            "context-rot-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert!(enforced.blocks_experiment_rollout);
        assert!(enforced.may_block_current_runner());
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn context_rot_trend_window_plan_tracks_consecutive_noise_before_enforcement() {
        let report = ContextRotTrendWindowPlan::trend_window(
            "context-rot-trend",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = ContextRotTrendWindowPlan::trend_window(
            "context-rot-trend-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert_eq!(report.report_schema_name, "context_rot_trend_report_v1");
        assert_eq!(report.min_window_rounds, 3);
        assert_eq!(report.max_consecutive_noisy_rounds, 1);
        assert_eq!(report.max_consecutive_duplicate_rounds, 0);
        assert!(!report.require_remediation_improves_noise);
        assert!(!report.require_remediation_improves_duplicates);
        assert!(!report.may_block_current_runner());
        assert!(report.preserves_legacy_runner);
        assert!(enforced.require_remediation_improves_noise);
        assert!(enforced.require_remediation_improves_duplicates);
        assert!(enforced.may_block_current_runner());
    }

    #[test]
    fn adapter_context_rot_trend_boundary_plan_does_not_collect_runner_signals() {
        let report = AdapterContextRotTrendBoundaryPlan::context_rot_trend_boundary(
            "context-rot-trend-boundary-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = AdapterContextRotTrendBoundaryPlan::context_rot_trend_boundary(
            "context-rot-trend-boundary-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert!(enforced.exposes_entrypoint("ContextRotTrendWindowSummary::from_points"));
        assert!(enforced.exposes_entrypoint("ContextRotTrendGate::evaluate"));
        assert!(enforced.exposes_entrypoint("ContextRotTrendReport::from_points_and_gate"));
        assert!(enforced.allows_input("ContextRotSignal"));
        assert!(enforced.allows_input("ContextRotTrendPoint"));
        assert!(enforced.allows_input("ContextRotTrendGate"));
        assert!(!enforced.allows_input("FilesystemScanner"));
        assert!(!enforced.allows_input("JsonlReader"));
        assert!(enforced.produces_output("ContextRotTrendWindowSummary"));
        assert!(enforced.produces_output("GateDecision"));
        assert!(enforced.produces_output("ContextRotTrendReport"));
        assert!(enforced.forbids_capability("jsonl_io"));
        assert!(enforced.forbids_capability("filesystem_scan"));
        assert!(enforced.forbids_capability("quarantine_action_execution"));
        assert!(enforced.forbids_capability("clean_gist_write"));
        assert!(enforced.forbids_capability("duplicate_output_delete"));
        assert!(enforced.forbids_capability("http_sse"));
        assert!(enforced.forbids_capability("process_spawn"));
        assert!(enforced.forbids_capability("validation_command_spawn"));
        assert!(enforced.forbids_capability("model_call"));
        assert!(enforced.forbids_capability("runner_state"));
        assert!(enforced.stays_pure_data_boundary());
        assert!(!report.may_block_current_runner());
        assert!(!enforced.may_block_current_runner());
        assert!(enforced.preserves_legacy_runner);
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn context_rot_remediation_plan_requires_cleanup_before_enforcement() {
        let report = ContextRotRemediationPlan::remediation(
            "context-rot-remediation-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = ContextRotRemediationPlan::remediation(
            "context-rot-remediation-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert_eq!(
            enforced.report_schema_name,
            "context_rot_remediation_report_v1"
        );
        assert!(!report.require_quarantine_complete);
        assert!(!report.require_legacy_metadata_repaired);
        assert!(!report.require_clean_gist_backfilled);
        assert!(!report.require_duplicates_removed);
        assert!(!report.may_block_current_runner());
        assert!(report.preserves_legacy_runner);
        assert!(enforced.require_quarantine_complete);
        assert!(enforced.require_legacy_metadata_repaired);
        assert!(enforced.require_clean_gist_backfilled);
        assert!(enforced.require_duplicates_removed);
        assert!(enforced.may_block_current_runner());
    }

    #[test]
    fn adapter_context_rot_remediation_boundary_plan_does_not_run_cleanup_actions() {
        let report = AdapterContextRotRemediationBoundaryPlan::context_rot_remediation_boundary(
            "context-rot-remediation-boundary-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = AdapterContextRotRemediationBoundaryPlan::context_rot_remediation_boundary(
            "context-rot-remediation-boundary-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert_eq!(
            enforced.eval_entrypoint,
            "ContextRotRemediationReport::from_gate_and_evidence"
        );
        assert!(enforced.allows_input("ContextRotSignal"));
        assert!(enforced.allows_input("ContextRotRemediationEvidence"));
        assert!(enforced.allows_input("ContextRotRemediationGate"));
        assert!(!enforced.allows_input("FilesystemScanner"));
        assert!(enforced.produces_output("ContextRotRemediationReport"));
        assert!(enforced.forbids_capability("jsonl_io"));
        assert!(enforced.forbids_capability("filesystem_scan"));
        assert!(enforced.forbids_capability("quarantine_action_execution"));
        assert!(enforced.forbids_capability("clean_gist_write"));
        assert!(enforced.forbids_capability("duplicate_output_delete"));
        assert!(enforced.forbids_capability("http_sse"));
        assert!(enforced.forbids_capability("process_spawn"));
        assert!(enforced.forbids_capability("validation_command_spawn"));
        assert!(enforced.forbids_capability("model_call"));
        assert!(enforced.forbids_capability("runner_state"));
        assert!(enforced.stays_pure_data_boundary());
        assert!(!report.may_block_current_runner());
        assert!(!enforced.may_block_current_runner());
        assert!(enforced.preserves_legacy_runner);
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn self_evolution_readiness_plan_names_required_gate_inputs() {
        let plan = SelfEvolutionReadinessPlan::next_round(
            "next-round-readiness",
            AdapterAcceptanceStage::ReportOnly,
        );

        assert_eq!(plan.report_schema_name, "readiness_next_round_v1");
        assert!(plan.required_gate_inputs.contains(&"ReportGate".to_owned()));
        assert!(
            plan.required_gate_inputs
                .contains(&"LedgerGateReport::gate_blocked".to_owned())
        );
        assert!(
            plan.required_gate_inputs
                .contains(&"ContextRotAcceptanceContract".to_owned())
        );
        assert!(
            plan.required_gate_inputs
                .contains(&"ContextRotReport::gate_blocked".to_owned())
        );
        assert!(
            plan.required_gate_inputs
                .contains(&"ContextRotTrendGate".to_owned())
        );
        assert!(
            plan.required_gate_inputs
                .contains(&"ContextRotTrendReport::trend_blocked".to_owned())
        );
        assert!(
            plan.required_gate_inputs
                .contains(&"ContextRotRemediationGate".to_owned())
        );
        assert!(
            plan.required_gate_inputs
                .contains(&"ContextRotRemediationReport::remediation_blocked".to_owned())
        );
        assert!(
            plan.required_gate_inputs
                .contains(&"RootAdapterFailureKind".to_owned())
        );
        assert!(
            plan.required_gate_inputs
                .contains(&"ExperimentExpansionSafetyGate".to_owned())
        );
        assert!(
            plan.required_gate_inputs.contains(
                &"ExperimentExpansionSafetyReport::allow_experiment_expansion".to_owned()
            )
        );
        assert!(
            plan.required_gate_inputs
                .contains(&"AdapterReportEmissionGate".to_owned())
        );
        assert!(
            plan.required_gate_inputs
                .contains(&"AdapterReportEmissionReport::field_coverage_passed".to_owned())
        );
        assert!(
            plan.required_gate_inputs
                .contains(&"AppleSiliconDevelopmentEffectGate".to_owned())
        );
        assert!(
            plan.required_gate_inputs
                .contains(&"RollbackResumeGate".to_owned())
        );
        assert!(
            plan.required_gate_inputs
                .contains(&"RollbackResumeReport::resume_blocked".to_owned())
        );
        assert!(
            plan.required_gate_inputs
                .contains(&"SteamCaseCoverageGate".to_owned())
        );
        assert!(
            plan.required_gate_inputs
                .contains(&"SteamCaseCoverageReport::coverage_blocked".to_owned())
        );
        assert!(
            plan.required_gate_inputs
                .contains(&"ValidationCommandCoverageGate".to_owned())
        );
        assert!(
            plan.required_gate_inputs
                .contains(&"ValidationCommandCoverageReport::coverage_blocked".to_owned())
        );
        assert!(plan.preserves_legacy_runner);
        assert!(!plan.may_block_current_runner());
    }

    #[test]
    fn self_evolution_readiness_plan_enforced_stage_can_block_runner() {
        let plan = SelfEvolutionReadinessPlan::next_round(
            "next-round-readiness-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert!(plan.may_block_current_runner());
        assert_eq!(
            plan.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn legacy_ledger_replay_plan_marks_new_reports_optional() {
        let plan = LegacyLedgerReplayPlan::evolution_loop_jsonl("legacy-ledger-replay");

        assert_eq!(plan.ledger_glob, r"target\evolution\*.jsonl");
        assert_eq!(plan.report_schema_name, "legacy_ledger_replay_report_v1");
        assert!(
            plan.required_existing_fields
                .contains(&"runtime_model".to_owned())
        );
        assert!(
            plan.required_existing_fields
                .contains(&"feedback_applied".to_owned())
        );
        assert!(
            plan.optional_additive_reports
                .contains(&"model_worker_v1".to_owned())
        );
        assert!(
            plan.optional_additive_reports
                .contains(&"worker_root_failure_consistency_report_v1".to_owned())
        );
        assert!(
            plan.optional_additive_reports
                .contains(&"model_pool_budget_fairness_report_v1".to_owned())
        );
        assert!(
            plan.optional_additive_reports
                .contains(&"model_pool_development_attribution_report_v1".to_owned())
        );
        assert!(
            plan.optional_additive_reports
                .contains(&"model_pool_development_window_report_v1".to_owned())
        );
        assert!(
            plan.optional_additive_reports
                .contains(&"apple_silicon_baseline_comparison_report_v1".to_owned())
        );
        assert!(
            plan.optional_additive_reports
                .contains(&"ledger_gate_report_v1".to_owned())
        );
        assert!(
            plan.optional_additive_reports
                .contains(&"report_freshness_report_v1".to_owned())
        );
        assert!(
            plan.optional_additive_reports
                .contains(&"remote_runtime_acceleration_report_v1".to_owned())
        );
        assert!(
            plan.optional_additive_reports
                .contains(&"run_mode_report_refresh_acceptance_report_v1".to_owned())
        );
        assert!(
            plan.optional_additive_reports
                .contains(&"context_rot_trend_report_v1".to_owned())
        );
        assert!(
            plan.optional_additive_reports
                .contains(&"context_rot_remediation_report_v1".to_owned())
        );
        assert!(
            plan.optional_additive_reports
                .contains(&"experiment_rollout_report_v1".to_owned())
        );
        assert!(
            plan.optional_additive_reports
                .contains(&"experiment_kill_switch_report_v1".to_owned())
        );
        assert!(
            plan.optional_additive_reports
                .contains(&"experiment_expansion_safety_report_v1".to_owned())
        );
        assert!(
            plan.optional_additive_reports
                .contains(&"experiment_switch_matrix_report_v1".to_owned())
        );
        assert!(
            plan.optional_additive_reports
                .contains(&"root_adapter_attribution_report_v1".to_owned())
        );
        assert!(
            plan.optional_additive_reports
                .contains(&"adapter_fixture_contract_report_v1".to_owned())
        );
        assert!(
            plan.optional_additive_reports
                .contains(&"current_runner_compatibility_report_v1".to_owned())
        );
        assert!(
            !plan
                .optional_additive_reports
                .contains(&"legacy_ledger_replay_report_v1".to_owned())
        );
        assert!(
            plan.optional_additive_reports
                .contains(&"feedback_self_improve_report_v1".to_owned())
        );
        assert!(
            plan.optional_additive_reports
                .contains(&"self_evolution_continuity_report_v1".to_owned())
        );
        assert!(
            plan.optional_additive_reports
                .contains(&"self_evolution_regression_report_v1".to_owned())
        );
        assert!(
            plan.optional_additive_reports
                .contains(&"readiness_next_round_v1".to_owned())
        );
        assert!(
            plan.optional_additive_reports
                .contains(&"advice_continuation_report_v1".to_owned())
        );
        assert!(
            plan.optional_additive_reports
                .contains(&"steam_round_report_v1".to_owned())
        );
        assert!(
            plan.optional_additive_reports
                .contains(&"steam_case_matrix_report_v1".to_owned())
        );
        assert!(
            plan.optional_additive_reports
                .contains(&"validation_command_coverage_report_v1".to_owned())
        );
        assert!(
            plan.optional_additive_reports
                .contains(&"rollback_report_v1".to_owned())
        );
        assert!(
            plan.optional_additive_reports
                .contains(&"adapter_closure_report_v1".to_owned())
        );
        assert!(
            plan.optional_additive_reports
                .contains(&"rollback_drill_matrix_report_v1".to_owned())
        );
        assert!(
            plan.optional_additive_reports
                .contains(&"adapter_handoff_report_v1".to_owned())
        );
        assert!(
            plan.optional_additive_reports
                .contains(&"report_bundle_gate_report_v1".to_owned())
        );
        assert!(
            plan.optional_additive_reports
                .contains(&"adapter_promotion_window_report_v1".to_owned())
        );
        assert!(
            plan.optional_additive_reports
                .contains(&"rollback_resume_report_v1".to_owned())
        );
        assert!(plan.preserves_legacy_runner);
    }

    #[test]
    fn adapter_legacy_ledger_replay_boundary_plan_does_not_read_jsonl_or_runner_state() {
        let report = AdapterLegacyLedgerReplayBoundaryPlan::legacy_ledger_replay_boundary(
            "legacy-replay-boundary-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = AdapterLegacyLedgerReplayBoundaryPlan::legacy_ledger_replay_boundary(
            "legacy-replay-boundary-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert!(enforced.exposes_entrypoint("LegacyLedgerReplayEvidence::from_ledger_summary"));
        assert!(enforced.exposes_entrypoint("LegacyLedgerReplayEvidence::from_ledger_records"));
        assert!(enforced.exposes_entrypoint(
            "LegacyLedgerReplayEvidence::from_ledger_summary_and_report_names"
        ));
        assert!(enforced.exposes_entrypoint(
            "LegacyLedgerReplayEvidence::from_ledger_records_and_report_names"
        ));
        assert!(
            enforced
                .exposes_entrypoint("LegacyLedgerReplayEvidence::with_observed_additive_reports")
        );
        assert!(enforced.exposes_entrypoint("LegacyLedgerReplayCompatibility::evaluate_replay"));
        assert!(enforced.exposes_entrypoint("LegacyLedgerReplayReport::from_summary_and_contract"));
        assert!(enforced.exposes_entrypoint("LegacyLedgerReplayReport::from_records_and_contract"));
        assert!(enforced.allows_input("LedgerRecord"));
        assert!(enforced.allows_input("LedgerSummary"));
        assert!(enforced.allows_input("LegacyLedgerReplayCompatibility"));
        assert!(enforced.allows_input("LegacyLedgerReplayEvidence"));
        assert!(enforced.allows_input("GateDecision"));
        assert!(enforced.allows_input("observed_report_schema_names"));
        assert!(!enforced.allows_input("JsonlLedgerReader"));
        assert!(!enforced.allows_input("ReportDirectoryScanner"));
        assert!(!enforced.allows_input("EvolutionLoopRunner"));
        assert!(enforced.produces_output("LegacyLedgerReplayEvidence"));
        assert!(enforced.produces_output("GateDecision"));
        assert!(enforced.produces_output("LegacyLedgerReplayReport"));
        assert!(enforced.forbids_capability("jsonl_io"));
        assert!(enforced.forbids_capability("file_io"));
        assert!(enforced.forbids_capability("report_directory_scan"));
        assert!(enforced.forbids_capability("http_sse"));
        assert!(enforced.forbids_capability("process_spawn"));
        assert!(enforced.forbids_capability("validation_command_spawn"));
        assert!(enforced.forbids_capability("model_call"));
        assert!(enforced.forbids_capability("runner_state"));
        assert!(enforced.stays_pure_data_boundary());
        assert!(!report.may_block_current_runner());
        assert!(!enforced.may_block_current_runner());
        assert!(enforced.preserves_legacy_runner);
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn steam_round_acceptance_plan_ties_case_validation_and_readiness() {
        let plan = SteamRoundAcceptancePlan::business_cycle(
            "steam-round",
            SmartSteamCase::business_cycle("case-1", "prove readiness"),
            VerificationPlan::cargo_test_manifest(r".\crates\norion-eval\Cargo.toml"),
            SelfEvolutionReadinessPlan::next_round("readiness", AdapterAcceptanceStage::ReportOnly),
        );

        assert_eq!(plan.case.endpoint, "/v1/business-cycle-stream");
        assert_eq!(plan.report_schema_name, "steam_round_report_v1");
        assert!(plan.require_stream_continuity);
        assert_eq!(plan.validation_plan.commands[0].args[0], "test");
        assert_eq!(
            plan.readiness_plan.report_schema_name,
            "readiness_next_round_v1"
        );
        assert!(plan.preserves_legacy_runner);
    }

    #[test]
    fn steam_case_matrix_plan_requires_multi_case_coverage_before_enforcement() {
        let report = SteamCaseMatrixPlan::business_cycle_matrix(
            "steam-case-matrix-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = SteamCaseMatrixPlan::business_cycle_matrix(
            "steam-case-matrix-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert_eq!(enforced.report_schema_name, "steam_case_matrix_report_v1");
        assert_eq!(enforced.min_cases, 4);
        assert_eq!(enforced.required_endpoint, "/v1/business-cycle-stream");
        assert!(
            enforced
                .required_case_kinds
                .contains(&"planning".to_owned())
        );
        assert!(
            enforced
                .required_case_kinds
                .contains(&"validation".to_owned())
        );
        assert!(
            enforced
                .required_case_kinds
                .contains(&"rollback".to_owned())
        );
        assert!(
            enforced
                .required_case_kinds
                .contains(&"apple_silicon_model_pool".to_owned())
        );
        assert!(
            enforced
                .required_final_json_fields
                .contains(&"business_cycle.passed".to_owned())
        );
        assert!(
            enforced
                .required_final_json_fields
                .contains(&"generate.runtime_tokens".to_owned())
        );
        assert!(enforced.require_unique_case_ids);
        assert!(enforced.require_stream_continuity);
        assert!(!report.require_validation_passed);
        assert!(!report.require_business_cycle_passed);
        assert!(!report.may_block_current_runner());
        assert!(report.preserves_legacy_runner);
        assert!(enforced.require_validation_passed);
        assert!(enforced.require_business_cycle_passed);
        assert!(enforced.may_block_current_runner());
        assert!(enforced.preserves_legacy_runner);
        assert_eq!(
            report.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn validation_command_coverage_plan_requires_command_evidence_before_enforcement() {
        let report = ValidationCommandCoveragePlan::post_round(
            "validation-command-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = ValidationCommandCoveragePlan::post_round(
            "validation-command-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert_eq!(
            enforced.report_schema_name,
            "validation_command_coverage_report_v1"
        );
        assert_eq!(enforced.required_phase, VerificationPhase::PostRound);
        assert_eq!(enforced.min_commands, 1);
        assert!(enforced.require_status_code);
        assert!(enforced.require_output_tail);
        assert!(enforced.require_all_commands_passed);
        assert!(!report.require_rust_check_passed);
        assert!(!report.require_strict_coverage_evidence);
        assert!(!report.may_block_current_runner());
        assert!(report.requires_report_field("validation_command.strict_coverage_requested"));
        assert!(report.requires_report_field("validation_command.coverage_tooling_evidence"));
        assert!(report.requires_report_field("validation_command.coverage_report_evidence"));
        assert!(report.requires_report_field(
            "validation_command.coverage_tooling_or_report_evidence_present"
        ));
        assert!(report.requires_report_field("validation_command.coverage_failure_kind"));
        assert!(report.requires_report_field("validation_command.model_quality_failure_counted"));
        assert!(report.requires_failure_attribution_rule(
            "validation_command_failure=validation_command_coverage"
        ));
        assert!(report.requires_failure_attribution_rule(
            "validation_command_failure!=model_quality_failure"
        ));
        assert!(report.forbid_model_quality_failure_counting);
        assert!(enforced.require_rust_check_passed);
        assert!(enforced.require_strict_coverage_evidence);
        assert!(enforced.requires_report_field("validation_command.strict_coverage_requested"));
        assert!(enforced.requires_report_field("validation_command.coverage_tooling_evidence"));
        assert!(enforced.requires_report_field("validation_command.coverage_report_evidence"));
        assert!(enforced.requires_report_field(
            "validation_command.coverage_tooling_or_report_evidence_present"
        ));
        assert!(enforced.requires_report_field("validation_command.coverage_failure_kind"));
        assert!(enforced.requires_report_field("validation_command.model_quality_failure_counted"));
        assert!(enforced.requires_report_field("validation_command.allow_next_round"));
        assert!(enforced.requires_failure_attribution_rule(
            "validation_command_failure=validation_command_coverage"
        ));
        assert!(enforced.requires_failure_attribution_rule(
            "validation_command_failure!=model_quality_failure"
        ));
        assert!(enforced.forbid_model_quality_failure_counting);
        assert!(enforced.may_block_current_runner());
        assert!(enforced.preserves_legacy_runner);
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn adapter_validation_command_coverage_boundary_plan_does_not_execute_commands() {
        let report = AdapterValidationCommandCoverageBoundaryPlan::validation_command_coverage(
            "validation-command-boundary-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = AdapterValidationCommandCoverageBoundaryPlan::validation_command_coverage(
            "validation-command-boundary-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert!(
            enforced.exposes_entrypoint("ValidationCommandCoverageEvidence::from_observations")
        );
        assert!(enforced.exposes_entrypoint(
            "ValidationCommandCoverageEvidence::with_strict_coverage_requested"
        ));
        assert!(enforced.exposes_entrypoint(
            "ValidationCommandCoverageEvidence::with_strict_coverage_request_from_helper_stage"
        ));
        assert!(enforced.exposes_entrypoint(
            "ValidationCommandCoverageEvidence::with_coverage_tooling_evidence"
        ));
        assert!(enforced.exposes_entrypoint(
            "ValidationCommandCoverageEvidence::with_coverage_report_evidence"
        ));
        assert!(
            enforced.exposes_entrypoint(
                "ValidationCommandCoverageEvidence::strict_coverage_is_requested"
            )
        );
        assert!(enforced.exposes_entrypoint(
            "ValidationCommandCoverageEvidence::coverage_tooling_or_report_evidence_present"
        ));
        assert!(enforced.exposes_entrypoint("ValidationCommandCoverageGate::evaluate"));
        assert!(
            enforced.exposes_entrypoint("ValidationCommandCoverageReport::from_gate_and_evidence")
        );
        assert!(
            enforced
                .exposes_entrypoint("ValidationCommandCoverageReport::from_observations_and_gate")
        );
        assert!(enforced.allows_input("ValidationObservation"));
        assert!(enforced.allows_input("CommandOutcome"));
        assert!(enforced.allows_input("VerificationPhase"));
        assert!(enforced.allows_input("ValidationCommandCoverageGate"));
        assert!(enforced.allows_input("strict_coverage_requested"));
        assert!(enforced.allows_input("coverage_tooling_evidence"));
        assert!(enforced.allows_input("coverage_report_evidence"));
        assert!(enforced.allows_input("HelperStageContractSummary"));
        assert!(!enforced.allows_input("ValidationCommandExecutor"));
        assert!(!enforced.allows_input("EvolutionLoopRunner"));
        assert!(!enforced.allows_input("ProcessHandle"));
        assert!(enforced.produces_output("ValidationCommandCoverageEvidence"));
        assert!(enforced.produces_output("GateDecision"));
        assert!(enforced.produces_output("ValidationCommandCoverageReport"));
        assert!(enforced.forbids_capability("jsonl_io"));
        assert!(enforced.forbids_capability("file_io"));
        assert!(enforced.forbids_capability("process_spawn"));
        assert!(enforced.forbids_capability("validation_command_spawn"));
        assert!(enforced.forbids_capability("validation_command_execution"));
        assert!(enforced.forbids_capability("model_call"));
        assert!(enforced.forbids_capability("runner_state"));
        assert!(enforced.stays_pure_data_boundary());
        assert!(!report.may_block_current_runner());
        assert!(!enforced.may_block_current_runner());
        assert!(enforced.preserves_legacy_runner);
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn validation_command_strict_coverage_schema_bundle_spans_contracts() {
        let coverage_plan = ValidationCommandCoveragePlan::post_round(
            "validation-command-bundle-plan",
            AdapterAcceptanceStage::Enforced,
        );
        let emission_plan = AdapterReportEmissionPlan::apple_silicon_development_effect(
            AdapterAcceptanceStage::Enforced,
        );
        let schema_drift = EvalSchemaDriftReportPlan::schema_drift(
            "validation-command-bundle-schema",
            AdapterAcceptanceStage::Enforced,
        );
        let boundary = AdapterValidationCommandCoverageBoundaryPlan::validation_command_coverage(
            "validation-command-bundle-boundary",
            AdapterAcceptanceStage::Enforced,
        );

        assert_eq!(validation_command_strict_coverage_schema_fields().len(), 4);
        assert_eq!(
            validation_command_strict_coverage_boundary_sources()
                .iter()
                .map(|(field, _)| *field)
                .collect::<Vec<_>>(),
            validation_command_strict_coverage_schema_fields().to_vec()
        );
        assert!(
            validation_command_strict_coverage_schema_fields()
                .iter()
                .all(|field| field.starts_with("validation_command.") && field.contains("coverage"))
        );

        assert!(has_validation_command_strict_coverage_schema_bundle(
            |field| { coverage_plan.requires_report_field(field) }
        ));
        assert!(has_validation_command_strict_coverage_schema_bundle(
            |field| { emission_plan.requires_report_field(field) }
        ));
        assert!(has_validation_command_strict_coverage_schema_bundle(
            |field| { schema_drift.requires_report_field_contract_example(field) }
        ));
        assert!(
            validation_command_strict_coverage_boundary_sources()
                .iter()
                .all(|(_, source)| boundary.exposes_entrypoint(source))
        );
        assert!(boundary.allows_input("strict_coverage_requested"));
        assert!(boundary.allows_input("coverage_tooling_evidence"));
        assert!(boundary.allows_input("coverage_report_evidence"));
    }

    #[test]
    fn rollback_report_plan_is_report_only_before_enforcement() {
        let plan = RollbackReportPlan::rollback_report(
            "rollback-report",
            AdapterAcceptanceStage::ReportOnly,
        );

        assert_eq!(plan.report_schema_name, "rollback_report_v1");
        assert!(!plan.require_resume_gate);
        assert!(!plan.may_block_current_runner());
        assert!(plan.preserves_legacy_runner);
    }

    #[test]
    fn rollback_report_plan_enforced_stage_requires_resume_gate() {
        let plan = RollbackReportPlan::rollback_report(
            "rollback-report-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert!(plan.require_resume_gate);
        assert!(plan.may_block_current_runner());
        assert_eq!(
            plan.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn adapter_normalized_evidence_projection_plan_keeps_runner_thin() {
        let report = AdapterNormalizedEvidenceProjectionPlan::from_runner_projection(
            "normalized-projection-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = AdapterNormalizedEvidenceProjectionPlan::from_runner_projection(
            "normalized-projection-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert_eq!(
            enforced.eval_entrypoint,
            "AdapterEvidenceProjection::from_normalized_evidence"
        );
        assert!(enforced.allows_input("LedgerRecord"));
        assert!(enforced.allows_input("ReportGate"));
        assert!(enforced.allows_input("HelperStageContractSummary"));
        assert!(enforced.allows_input("ValidationCommandCoverageEvidence"));
        assert!(enforced.allows_input("ValidationCommandCoverageGate"));
        assert!(!enforced.allows_input("EvolutionLoopRunner"));
        assert!(enforced.produces_output("LedgerSummary"));
        assert!(enforced.produces_output("LedgerGateReport"));
        assert!(enforced.produces_output("ValidationCommandCoverageReport"));
        assert!(enforced.forbids_capability("jsonl_io"));
        assert!(enforced.forbids_capability("http_sse"));
        assert!(enforced.forbids_capability("process_spawn"));
        assert!(enforced.forbids_capability("validation_command_spawn"));
        assert!(enforced.forbids_capability("model_call"));
        assert!(enforced.forbids_capability("runner_state"));
        assert!(!report.may_block_current_runner());
        assert!(!enforced.may_block_current_runner());
        assert!(enforced.preserves_legacy_runner);
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn validation_coverage_gate_bridge_uses_test_plan_evidence_before_runner_gates() {
        let coverage_plan = ValidationCommandCoveragePlan::post_round(
            "validation-command-bridge",
            AdapterAcceptanceStage::Enforced,
        );
        let coverage_boundary =
            AdapterValidationCommandCoverageBoundaryPlan::validation_command_coverage(
                "validation-command-bridge-boundary",
                AdapterAcceptanceStage::Enforced,
            );
        let projection = AdapterNormalizedEvidenceProjectionPlan::from_runner_projection(
            "normalized-validation-bridge",
            AdapterAcceptanceStage::Enforced,
        );
        let self_evolution =
            SelfEvolutionUnattendedPrerequisiteReportPlan::unattended_prerequisites(
                "self-evolution-validation-bridge",
                AdapterAcceptanceStage::Enforced,
            );
        let handoff = AdapterHandoffReportPlan::adapter_handoff(
            "handoff-validation-bridge",
            AdapterAcceptanceStage::Enforced,
        );
        let handoff_boundary = AdapterHandoffBoundaryPlan::handoff_boundary(
            "handoff-validation-bridge-boundary",
            AdapterAcceptanceStage::Enforced,
        );
        let current_runner = CurrentRunnerCompatibilityPlan::before_enforced_wiring(
            "current-runner-validation-bridge",
            AdapterAcceptanceStage::Enforced,
        );
        let current_runner_schema =
            CurrentRunnerCompatibilitySchemaDocumentPlan::current_runner_compatibility_schema_document(
                "current-runner-validation-bridge-schema",
                AdapterAcceptanceStage::Enforced,
            );
        let current_runner_boundary =
            AdapterCurrentRunnerCompatibilityBoundaryPlan::current_runner_compatibility_boundary(
                "current-runner-validation-bridge-boundary",
                AdapterAcceptanceStage::Enforced,
            );

        assert!(coverage_plan.require_strict_coverage_evidence);
        assert!(coverage_plan.requires_report_field(
            "validation_command.coverage_tooling_or_report_evidence_present"
        ));
        assert!(coverage_boundary.allows_input("ValidationObservation"));
        assert!(coverage_boundary.allows_input("CommandOutcome"));
        assert!(coverage_boundary.allows_input("coverage_tooling_evidence"));
        assert!(coverage_boundary.allows_input("coverage_report_evidence"));
        assert!(coverage_boundary.exposes_entrypoint("ValidationCommandCoverageGate::evaluate"));
        assert!(
            coverage_boundary
                .exposes_entrypoint("ValidationCommandCoverageReport::from_gate_and_evidence")
        );
        assert!(!coverage_boundary.allows_input("ValidationCommandExecutor"));
        assert!(coverage_boundary.forbids_capability("validation_command_execution"));

        assert!(projection.allows_input("ValidationCommandCoverageEvidence"));
        assert!(projection.allows_input("ValidationCommandCoverageGate"));
        assert!(projection.produces_output("ValidationCommandCoverageReport"));
        assert!(!projection.allows_input("ValidationCommandExecutor"));
        assert!(projection.forbids_capability("validation_command_spawn"));

        assert!(self_evolution.require_validation_command_coverage);
        assert!(
            self_evolution.uses_operational_report_source(
                "ValidationCommandCoverageReport::allow_next_round"
            )
        );
        assert!(
            !self_evolution
                .uses_operational_report_source("HelperStageContractSummary::coverage_text")
        );

        assert!(handoff.require_validation_command_coverage_before_enforcement);
        assert!(handoff_boundary.allows_input("ValidationCommandCoverageReport"));
        assert!(
            handoff_boundary
                .exposes_entrypoint("AdapterHandoffEvidence::with_operational_gate_reports")
        );
        assert!(!handoff_boundary.allows_input("HelperStageContractSummary"));
        assert!(!handoff_boundary.allows_input("ValidationCommandExecutor"));

        assert!(current_runner.require_validation_command_coverage);
        assert!(
            current_runner_schema
                .requires_document_field("current_runner.validation_command_coverage_passed")
        );
        assert!(current_runner_boundary.allows_input("test_result_pass_bits"));
        assert!(!current_runner_boundary.allows_input("HelperStageContractSummary"));
        assert!(!current_runner_boundary.allows_input("ValidationCommandExecutor"));
        assert!(!current_runner_boundary.allows_input("CommandExecutor"));
    }

    #[test]
    fn unattended_evolution_acceptance_matrix_requires_evidence_backed_gates() {
        let coverage_plan = ValidationCommandCoveragePlan::post_round(
            "validation-command-matrix",
            AdapterAcceptanceStage::Enforced,
        );
        let coverage_boundary =
            AdapterValidationCommandCoverageBoundaryPlan::validation_command_coverage(
                "validation-command-matrix-boundary",
                AdapterAcceptanceStage::Enforced,
            );
        let projection = AdapterNormalizedEvidenceProjectionPlan::from_runner_projection(
            "normalized-matrix",
            AdapterAcceptanceStage::Enforced,
        );
        let rollback_resume = RollbackResumeReportPlan::rollback_resume(
            "rollback-resume-matrix",
            AdapterAcceptanceStage::Enforced,
        );
        let rollback_boundary = AdapterRollbackResumeBoundaryPlan::rollback_resume_boundary(
            "rollback-resume-matrix-boundary",
            AdapterAcceptanceStage::Enforced,
        );
        let unattended = SelfEvolutionUnattendedPrerequisiteReportPlan::unattended_prerequisites(
            "unattended-matrix",
            AdapterAcceptanceStage::Enforced,
        );
        let handoff = AdapterHandoffReportPlan::adapter_handoff(
            "handoff-matrix",
            AdapterAcceptanceStage::Enforced,
        );
        let handoff_boundary = AdapterHandoffBoundaryPlan::handoff_boundary(
            "handoff-matrix-boundary",
            AdapterAcceptanceStage::Enforced,
        );
        let current_runner = CurrentRunnerCompatibilityPlan::before_enforced_wiring(
            "current-runner-matrix",
            AdapterAcceptanceStage::Enforced,
        );
        let current_runner_schema =
            CurrentRunnerCompatibilitySchemaDocumentPlan::current_runner_compatibility_schema_document(
                "current-runner-matrix-schema",
                AdapterAcceptanceStage::Enforced,
            );
        let current_runner_boundary =
            AdapterCurrentRunnerCompatibilityBoundaryPlan::current_runner_compatibility_boundary(
                "current-runner-matrix-boundary",
                AdapterAcceptanceStage::Enforced,
            );

        let rows = [
            (
                "validation coverage",
                coverage_plan.require_strict_coverage_evidence
                    && coverage_boundary.allows_input("ValidationObservation")
                    && coverage_boundary.allows_input("CommandOutcome")
                    && coverage_boundary
                        .exposes_entrypoint("ValidationCommandCoverageGate::evaluate")
                    && coverage_boundary.exposes_entrypoint(
                        "ValidationCommandCoverageReport::from_gate_and_evidence",
                    )
                    && projection.allows_input("ValidationCommandCoverageEvidence")
                    && projection.allows_input("ValidationCommandCoverageGate")
                    && projection.produces_output("ValidationCommandCoverageReport"),
                !coverage_boundary.allows_input("ValidationCommandExecutor")
                    && !projection.allows_input("ValidationCommandExecutor")
                    && coverage_boundary.forbids_capability("validation_command_execution")
                    && projection.forbids_capability("validation_command_spawn"),
            ),
            (
                "rollback resume",
                rollback_resume.require_resume_evidence
                    && rollback_resume.require_validation_command_coverage
                    && rollback_boundary.allows_input("RollbackReport.resume_gate")
                    && rollback_boundary.allows_input("RollbackResumeEvidence")
                    && rollback_boundary.allows_input("RollbackResumeGate")
                    && rollback_boundary.allows_resume_gate("planned_validation_command")
                    && rollback_boundary.produces_output("RollbackResumeReport"),
                !rollback_boundary.allows_input("helper_prose")
                    && !rollback_boundary.allows_resume_gate("spawn_validation_command")
                    && rollback_boundary.forbids_capability("resume_action_execution")
                    && rollback_boundary.forbids_capability("validation_command_spawn"),
            ),
            (
                "self evolution unattended",
                unattended.require_validation_command_coverage
                    && unattended.require_rollback_resume
                    && unattended.uses_operational_report_source(
                        "ValidationCommandCoverageReport::allow_next_round",
                    )
                    && unattended.uses_operational_report_source(
                        "RollbackResumeReport::allow_unattended_rounds",
                    )
                    && unattended.uses_operational_report_source(
                        "AdapterReportEmissionReport::field_coverage_passed",
                    ),
                !unattended
                    .uses_operational_report_source("HelperStageContractSummary::coverage_text")
                    && !unattended.uses_operational_report_source("ValidationCommandExecutor::run")
                    && unattended.forbids_capability("validation_command_spawn")
                    && unattended.forbids_capability("runner_state"),
            ),
            (
                "handoff test gate",
                handoff.require_validation_command_coverage_before_enforcement
                    && handoff.require_rollback_resume_before_enforcement
                    && handoff.require_self_evolution_unattended_prerequisites_before_enforcement
                    && handoff_boundary.allows_input("AdapterTestGate")
                    && handoff_boundary.allows_input("ValidationCommandCoverageReport")
                    && handoff_boundary.allows_input("RollbackResumeReport")
                    && handoff_boundary.allows_input("SelfEvolutionUnattendedPrerequisiteReport")
                    && handoff_boundary.allows_input("upstream_report_pass_bits"),
                !handoff_boundary.allows_input("HelperStageContractSummary")
                    && !handoff_boundary.allows_input("CommandExecutor")
                    && handoff_boundary.forbids_capability("cargo_test_execution")
                    && handoff_boundary.forbids_capability("runner_handoff_execution"),
            ),
            (
                "current runner",
                current_runner.require_validation_command_coverage
                    && current_runner.require_rollback_resume
                    && current_runner.require_self_evolution_unattended_prerequisites
                    && current_runner.require_evolution_loop_tests
                    && current_runner.require_workspace_tests
                    && current_runner_schema.requires_document_field(
                        "current_runner.validation_command_coverage_passed",
                    )
                    && current_runner_schema
                        .requires_document_field("current_runner.rollback_resume_passed")
                    && current_runner_schema.requires_document_field(
                        "current_runner.self_evolution_unattended_prerequisites_passed",
                    )
                    && current_runner_boundary.allows_input("test_result_pass_bits"),
                !current_runner_boundary.allows_input("HelperStageContractSummary")
                    && !current_runner_boundary.allows_input("CommandExecutor")
                    && current_runner_boundary.forbids_capability("cargo_test_execution")
                    && current_runner_boundary.forbids_capability("runner_wiring_execution"),
            ),
        ];

        for (name, requires_evidence_gate, rejects_helper_prose) in rows {
            assert!(
                requires_evidence_gate,
                "{name} row must require evidence-backed gates"
            );
            assert!(
                rejects_helper_prose,
                "{name} row must reject helper prose or execution shortcuts"
            );
        }
    }

    #[test]
    fn adapter_closure_acceptance_bundle_requires_report_object_sources() {
        let closure = AdapterClosurePureDataPlan::adapter_closure(
            "adapter-closure-acceptance-bundle",
            AdapterAcceptanceStage::Enforced,
        );
        let unattended = SelfEvolutionUnattendedPrerequisiteReportPlan::unattended_prerequisites(
            "unattended-acceptance-bundle",
            AdapterAcceptanceStage::Enforced,
        );
        let handoff = AdapterHandoffBoundaryPlan::handoff_boundary(
            "handoff-acceptance-bundle",
            AdapterAcceptanceStage::Enforced,
        );
        let current_runner = CurrentRunnerCompatibilityPlan::before_enforced_wiring(
            "current-runner-acceptance-bundle",
            AdapterAcceptanceStage::Enforced,
        );
        let current_runner_boundary =
            AdapterCurrentRunnerCompatibilityBoundaryPlan::current_runner_compatibility_boundary(
                "current-runner-acceptance-bundle-boundary",
                AdapterAcceptanceStage::Enforced,
            );

        let rows = [
            (
                "adapter closure",
                closure.report_schema_name == "adapter_closure_report_v1"
                    && closure.exposes_entrypoint("AdapterClosureReport::from_reports")
                    && closure.requires_input_report("ledger_gate_report_v1")
                    && closure.requires_input_report("validation_command_coverage_report_v1")
                    && closure.requires_input_report("readiness_next_round_v1")
                    && closure.requires_input_report("rollback_report_v1")
                    && closure.allows_input("LedgerGateReport")
                    && closure.allows_input("ValidationCommandCoverageReport")
                    && closure.allows_input("SelfEvolutionReadinessReport")
                    && !closure.allows_input("RollbackReport")
                    && closure.requires_report_field("adapter_closure.allow_next_round"),
                !closure.allows_input("helper_prose")
                    && !closure.allows_input("raw_logs")
                    && !closure.allows_input("RunnerSideEffects")
                    && closure.forbids_capability("jsonl_io")
                    && closure.forbids_capability("validation_command_spawn")
                    && closure.forbids_capability("runner_state"),
            ),
            (
                "unattended prerequisite",
                unattended.report_schema_name
                    == "self_evolution_unattended_prerequisites_report_v1"
                    && unattended.require_validation_command_coverage
                    && unattended.require_rollback_resume
                    && unattended.require_adapter_report_field_coverage
                    && unattended.uses_operational_report_source(
                        "ValidationCommandCoverageReport::allow_next_round",
                    )
                    && unattended.uses_operational_report_source(
                        "RollbackResumeReport::allow_unattended_rounds",
                    )
                    && unattended.uses_operational_report_source(
                        "AdapterReportEmissionReport::field_coverage_passed",
                    )
                    && unattended.uses_operational_report_source(
                        "AppleSiliconDevelopmentEffectReport::allow_development_effect_claim",
                    ),
                !unattended.uses_operational_report_source("helper_prose")
                    && !unattended.uses_operational_report_source("raw_logs")
                    && !unattended.uses_operational_report_source("RunnerSideEffects")
                    && !unattended.uses_operational_report_source(
                        "HelperStageContractSummary::coverage_text",
                    )
                    && !unattended.uses_operational_report_source("EvolutionLoopRunner::state")
                    && unattended.forbids_capability("jsonl_io")
                    && unattended.forbids_capability("validation_command_spawn")
                    && unattended.forbids_capability("runner_state"),
            ),
            (
                "current runner compatibility",
                current_runner.report_schema_name == "current_runner_compatibility_report_v1"
                    && current_runner.require_report_bundle
                    && current_runner.require_self_evolution_unattended_prerequisites
                    && current_runner.require_handoff
                    && handoff.allows_input("SelfEvolutionUnattendedPrerequisiteReport")
                    && handoff.produces_output("AdapterHandoffReport")
                    && current_runner_boundary.exposes_entrypoint(
                        "CurrentRunnerCompatibilityReport::from_gate_and_evidence",
                    )
                    && current_runner_boundary.allows_input("AdapterReportEmissionReport")
                    && current_runner_boundary.allows_input("EvalReportBundleGateReport")
                    && current_runner_boundary.allows_input("EvalSchemaDriftReport")
                    && current_runner_boundary.allows_input("AdapterFixtureReport")
                    && current_runner_boundary.allows_input("AdapterHandoffReport")
                    && current_runner_boundary.produces_output("CurrentRunnerCompatibilityReport"),
                !current_runner_boundary.allows_input("helper_prose")
                    && !current_runner_boundary.allows_input("raw_logs")
                    && !current_runner_boundary.allows_input("RunnerSideEffects")
                    && !current_runner_boundary.allows_input("CommandExecutor")
                    && !current_runner_boundary.allows_input("EvolutionLoopRunner")
                    && current_runner_boundary.forbids_capability("file_io")
                    && current_runner_boundary.forbids_capability("cargo_test_execution")
                    && current_runner_boundary.forbids_capability("runner_wiring_execution")
                    && current_runner_boundary.forbids_capability("runner_state_mutation"),
            ),
        ];

        for (name, requires_report_objects, rejects_non_report_sources) in rows {
            assert!(
                requires_report_objects,
                "{name} acceptance must be sourced from evidence-backed report objects"
            );
            assert!(
                rejects_non_report_sources,
                "{name} acceptance must reject helper prose, raw logs, and runner side effects"
            );
        }
    }

    #[test]
    fn adapter_readiness_reports_input_plan_accepts_only_computed_reports() {
        let report = AdapterReadinessReportsInputPlan::readiness_reports(
            "readiness-reports-input-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = AdapterReadinessReportsInputPlan::readiness_reports(
            "readiness-reports-input-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert_eq!(
            enforced.eval_entrypoint,
            "AdapterEvidenceProjection::readiness_snapshot_with_reports"
        );
        assert!(enforced.allows_report_input("SteamRoundAcceptanceReport"));
        assert!(enforced.allows_report_input("SteamCaseCoverageReport"));
        assert!(enforced.allows_report_input("ContextRotReport"));
        assert!(enforced.allows_report_input("ContextRotTrendReport"));
        assert!(enforced.allows_report_input("ContextRotRemediationReport"));
        assert!(enforced.allows_report_input("AdviceContinuationReport"));
        assert!(!enforced.allows_report_input("SteamHttpClient"));
        assert!(enforced.produces_output("SelfEvolutionReadinessSnapshot"));
        assert!(enforced.produces_output("SelfEvolutionReadinessReport"));
        assert!(enforced.produces_output("RollbackReport"));
        assert!(enforced.produces_output("AdapterClosureReport"));
        assert!(enforced.forbids_capability("jsonl_io"));
        assert!(enforced.forbids_capability("http_sse"));
        assert!(enforced.forbids_capability("process_spawn"));
        assert!(enforced.forbids_capability("validation_command_spawn"));
        assert!(enforced.forbids_capability("model_call"));
        assert!(enforced.forbids_capability("runner_state"));
        assert!(!report.may_block_current_runner());
        assert!(!enforced.may_block_current_runner());
        assert!(enforced.preserves_legacy_runner);
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn adapter_steam_evidence_boundary_plan_keeps_http_sse_outside_eval() {
        let report = AdapterSteamEvidenceBoundaryPlan::steam_reports(
            "steam-evidence-boundary-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = AdapterSteamEvidenceBoundaryPlan::steam_reports(
            "steam-evidence-boundary-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert!(enforced.exposes_entrypoint("SteamRoundAcceptanceReport::from_evidence"));
        assert!(enforced.exposes_entrypoint("SteamCaseCoverageReport::from_rows_and_gate"));
        assert!(enforced.exposes_entrypoint("SteamCaseCoverageReport::from_gate_and_evidence"));
        assert!(enforced.allows_input("StreamContinuityCheck"));
        assert!(enforced.allows_input("ValidationObservation"));
        assert!(enforced.allows_input("LedgerRecord"));
        assert!(enforced.allows_input("SelfEvolutionReadinessSnapshot"));
        assert!(enforced.allows_input("SteamCaseCoverageRow"));
        assert!(enforced.allows_input("SteamCaseCoverageEvidence"));
        assert!(!enforced.allows_input("SteamHttpClient"));
        assert!(!enforced.allows_input("EvolutionLoopRunner"));
        assert!(enforced.produces_output("SteamRoundAcceptanceReport"));
        assert!(enforced.produces_output("SteamCaseCoverageReport"));
        assert!(enforced.forbids_capability("jsonl_io"));
        assert!(enforced.forbids_capability("http_sse"));
        assert!(enforced.forbids_capability("steam_http_execution"));
        assert!(enforced.forbids_capability("stream_process_execution"));
        assert!(enforced.forbids_capability("process_spawn"));
        assert!(enforced.forbids_capability("validation_command_spawn"));
        assert!(enforced.forbids_capability("model_call"));
        assert!(enforced.forbids_capability("runner_state"));
        assert!(enforced.stays_pure_data_boundary());
        assert!(!report.may_block_current_runner());
        assert!(!enforced.may_block_current_runner());
        assert!(enforced.preserves_legacy_runner);
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn adapter_closure_pure_data_plan_forbids_runner_capabilities() {
        let report = AdapterClosurePureDataPlan::adapter_closure(
            "adapter-closure-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = AdapterClosurePureDataPlan::adapter_closure(
            "adapter-closure-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert_eq!(enforced.report_schema_name, "adapter_closure_report_v1");
        assert!(
            enforced.exposes_entrypoint("AdapterEvidenceProjection::closure_report_with_reports",)
        );
        assert!(enforced.exposes_entrypoint("AdapterClosureReport::from_reports"));
        assert!(enforced.exposes_entrypoint("RollbackReport::from_readiness_report"));
        assert!(enforced.allows_input("LedgerRecord"));
        assert!(enforced.allows_input("ValidationCommandCoverageEvidence"));
        assert!(enforced.allows_input("AdapterReadinessReports"));
        assert!(enforced.allows_input("LedgerGateReport"));
        assert!(enforced.allows_input("ValidationCommandCoverageReport"));
        assert!(enforced.allows_input("SelfEvolutionReadinessReport"));
        assert!(enforced.allows_input("helper_stage_useful"));
        assert!(!enforced.allows_input("EvolutionLoopRunner"));
        assert!(enforced.requires_input_report("ledger_gate_report_v1"));
        assert!(enforced.requires_input_report("validation_command_coverage_report_v1"));
        assert!(enforced.requires_input_report("readiness_next_round_v1"));
        assert!(enforced.requires_input_report("rollback_report_v1"));
        assert!(enforced.forbids_capability("jsonl_io"));
        assert!(enforced.forbids_capability("http_sse"));
        assert!(enforced.forbids_capability("process_spawn"));
        assert!(enforced.forbids_capability("validation_command_spawn"));
        assert!(enforced.forbids_capability("model_call"));
        assert!(enforced.forbids_capability("runner_state"));
        assert!(enforced.requires_report_field("adapter_closure.stage"));
        assert!(enforced.requires_report_field("adapter_closure.rollback_resume_gate"));
        assert!(enforced.requires_report_field("adapter_closure.allow_next_round"));
        assert!(enforced.stays_pure_data_boundary());
        assert!(!report.may_block_current_runner());
        assert!(!enforced.may_block_current_runner());
        assert!(enforced.preserves_legacy_runner);
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn coverage_gate_requirements_stay_on_evidence_backed_plan_items() {
        let coverage_plan = ValidationCommandCoveragePlan::post_round(
            "validation-command-coverage",
            AdapterAcceptanceStage::Enforced,
        );
        let coverage_boundary =
            AdapterValidationCommandCoverageBoundaryPlan::validation_command_coverage(
                "validation-command-coverage-boundary",
                AdapterAcceptanceStage::Enforced,
            );
        let helper_projection = AdapterClosurePureDataPlan::adapter_closure(
            "adapter-closure-helper",
            AdapterAcceptanceStage::Enforced,
        );
        let helper_schema = AdapterClosureSchemaDocumentPlan::adapter_closure_schema_document(
            "adapter-closure-schema",
            AdapterAcceptanceStage::Enforced,
        );

        assert!(coverage_plan.may_block_current_runner());
        assert!(coverage_plan.require_rust_check_passed);
        assert!(coverage_plan.require_strict_coverage_evidence);
        assert!(
            coverage_plan.requires_report_field("validation_command.strict_coverage_requested")
        );
        assert!(
            coverage_plan.requires_report_field("validation_command.coverage_tooling_evidence")
        );
        assert!(coverage_plan.requires_report_field("validation_command.coverage_report_evidence"));
        assert!(coverage_plan.requires_report_field(
            "validation_command.coverage_tooling_or_report_evidence_present"
        ));
        assert!(coverage_plan.requires_report_field("validation_command.coverage_failure_kind"));
        assert!(coverage_plan.requires_report_field("validation_command.allow_next_round"));
        assert!(coverage_boundary.exposes_entrypoint(
            "ValidationCommandCoverageEvidence::coverage_tooling_or_report_evidence_present"
        ));
        assert!(coverage_boundary.exposes_entrypoint("ValidationCommandCoverageGate::evaluate"));
        assert!(!coverage_boundary.may_block_current_runner());

        assert!(helper_projection.requires_input_report(&coverage_plan.report_schema_name));
        assert!(helper_projection.allows_input("ValidationCommandCoverageReport"));
        assert!(!helper_projection.allows_input("RollbackReport"));
        assert!(
            helper_projection
                .requires_report_field("adapter_closure.validation_command_coverage_blocked")
        );
        assert!(!helper_projection.exposes_entrypoint("ValidationCommandCoverageGate::evaluate"));
        assert!(!helper_projection.allows_input("ValidationCommandExecutor"));
        assert!(helper_projection.forbids_capability("validation_command_spawn"));
        assert!(!helper_projection.may_block_current_runner());

        assert!(helper_schema.requires_allowed_input("ValidationCommandCoverageReport"));
        assert!(
            helper_schema
                .requires_document_field("adapter_closure.validation_command_coverage_blocked")
        );
        assert!(
            helper_schema
                .requires_boundary_source("AdapterReportEmissionPlan::required_report_fields")
        );
        assert!(helper_schema.require_emission_field_coverage);
        assert!(!helper_schema.may_block_current_runner());
    }

    #[test]
    fn adapter_closure_schema_document_plan_guards_field_and_boundary_drift() {
        let report = AdapterClosureSchemaDocumentPlan::adapter_closure_schema_document(
            "adapter-closure-schema-doc-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = AdapterClosureSchemaDocumentPlan::adapter_closure_schema_document(
            "adapter-closure-schema-doc-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert_eq!(enforced.report_schema_name, "adapter_closure_report_v1");
        assert_eq!(
            enforced.schema_document_source,
            "AdapterClosurePureDataContract::schema_document"
        );
        assert!(
            enforced.requires_entrypoint("AdapterEvidenceProjection::closure_report_with_reports")
        );
        assert!(enforced.requires_entrypoint("AdapterClosureReport::from_reports"));
        assert!(enforced.requires_entrypoint("RollbackReport::from_readiness_report"));
        assert!(enforced.requires_document_field("adapter_closure.stage"));
        assert!(enforced.requires_document_field("adapter_closure.helper_stage_useful"));
        assert!(
            enforced.requires_document_field("adapter_closure.validation_command_coverage_blocked")
        );
        assert!(enforced.requires_document_field("adapter_closure.rollback_resume_gate"));
        assert!(enforced.requires_document_field("adapter_closure.allow_next_round"));
        assert!(
            enforced
                .required_report_only_fields
                .contains(&"adapter_closure.ledger_gate_blocked".to_owned())
        );
        assert!(
            enforced
                .required_enforced_fields
                .contains(&"adapter_closure.allow_next_round".to_owned())
        );
        assert!(
            enforced.requires_boundary_source("AdapterClosureReportSchema::adapter_closure_v1")
        );
        assert!(
            enforced.requires_boundary_source("AdapterReportEmissionPlan::required_report_fields")
        );
        assert!(
            enforced.requires_boundary_source("AdapterClosurePureDataContract::adapter_closure_v1")
        );
        assert!(enforced.requires_allowed_input("AdapterReadinessReports"));
        assert!(enforced.requires_allowed_input("LedgerGateReport"));
        assert!(enforced.requires_allowed_input("ValidationCommandCoverageReport"));
        assert!(enforced.requires_allowed_input("SelfEvolutionReadinessReport"));
        assert!(enforced.requires_allowed_input("helper_stage_useful"));
        assert!(enforced.forbids_capability("jsonl_io"));
        assert!(enforced.forbids_capability("http_sse"));
        assert!(enforced.forbids_capability("process_spawn"));
        assert!(enforced.forbids_capability("validation_command_spawn"));
        assert!(enforced.forbids_capability("model_call"));
        assert!(enforced.forbids_capability("runner_state"));
        assert!(!report.require_emission_field_coverage);
        assert!(enforced.require_emission_field_coverage);
        assert!(!report.may_block_current_runner());
        assert!(!enforced.may_block_current_runner());
        assert!(enforced.preserves_legacy_runner);
    }

    #[test]
    fn rollback_drill_matrix_plan_is_observational_until_enforced() {
        let report = RollbackDrillMatrixReportPlan::rollback_drill_matrix(
            "rollback-drill-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = RollbackDrillMatrixReportPlan::rollback_drill_matrix(
            "rollback-drill-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert_eq!(report.report_schema_name, "rollback_drill_matrix_report_v1");
        assert!(!report.require_all_root_adapter_failure_kinds);
        assert!(!report.require_stable_rollback_reasons);
        assert!(!report.require_stable_resume_gates);
        assert!(!report.require_actions_for_required_rollbacks);
        assert!(!report.forbid_clean_case_rollback);
        assert!(!report.may_block_current_runner());
        assert!(report.preserves_legacy_runner);
        assert!(enforced.require_all_root_adapter_failure_kinds);
        assert!(enforced.require_stable_rollback_reasons);
        assert!(enforced.require_stable_resume_gates);
        assert!(enforced.require_actions_for_required_rollbacks);
        assert!(enforced.forbid_clean_case_rollback);
        assert!(enforced.may_block_current_runner());
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn adapter_rollback_drill_matrix_boundary_plan_does_not_execute_actions() {
        let report = AdapterRollbackDrillMatrixBoundaryPlan::rollback_drill_matrix_boundary(
            "rollback-drill-boundary-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = AdapterRollbackDrillMatrixBoundaryPlan::rollback_drill_matrix_boundary(
            "rollback-drill-boundary-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert!(enforced.exposes_entrypoint("RollbackDrillCase::from_failure_kind"));
        assert!(
            enforced.exposes_entrypoint("RollbackDrillMatrixEvidence::root_adapter_policy_matrix")
        );
        assert!(enforced.exposes_entrypoint("RollbackDrillMatrixGate::evaluate"));
        assert!(enforced.exposes_entrypoint("RollbackDrillMatrixReport::from_gate_and_evidence"));
        assert!(enforced.allows_input("RootAdapterFailureKind"));
        assert!(enforced.allows_input("RollbackDrillCase"));
        assert!(enforced.allows_input("RollbackDrillMatrixEvidence"));
        assert!(enforced.allows_input("RollbackDrillMatrixGate"));
        assert!(!enforced.allows_input("RollbackActionExecutor"));
        assert!(!enforced.allows_input("ResumeActionExecutor"));
        assert!(!enforced.allows_input("EvolutionLoopRunner"));
        assert!(enforced.produces_output("RollbackDrillCase"));
        assert!(enforced.produces_output("RollbackDrillMatrixEvidence"));
        assert!(enforced.produces_output("GateDecision"));
        assert!(enforced.produces_output("RollbackDrillMatrixReport"));
        assert!(enforced.forbids_capability("jsonl_io"));
        assert!(enforced.forbids_capability("http_sse"));
        assert!(enforced.forbids_capability("process_spawn"));
        assert!(enforced.forbids_capability("validation_command_spawn"));
        assert!(enforced.forbids_capability("model_call"));
        assert!(enforced.forbids_capability("rollback_action_execution"));
        assert!(enforced.forbids_capability("resume_action_execution"));
        assert!(enforced.forbids_capability("runner_state"));
        assert!(enforced.stays_pure_data_boundary());
        assert!(!report.may_block_current_runner());
        assert!(!enforced.may_block_current_runner());
        assert!(enforced.preserves_legacy_runner);
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn rollback_resume_report_plan_requires_resume_evidence_before_unattended_rounds() {
        let report = RollbackResumeReportPlan::rollback_resume(
            "rollback-resume-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = RollbackResumeReportPlan::rollback_resume(
            "rollback-resume-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert_eq!(enforced.report_schema_name, "rollback_resume_report_v1");
        assert!(!report.require_resume_evidence);
        assert!(!report.require_steam_case_matrix);
        assert!(!report.require_validation_command_coverage);
        assert!(!report.require_adapter_report_field_coverage);
        assert!(!report.require_operational_failures_not_quality);
        assert!(!report.may_block_current_runner());
        assert!(report.preserves_legacy_runner);
        assert!(enforced.require_resume_evidence);
        assert!(enforced.require_steam_case_matrix);
        assert!(enforced.require_validation_command_coverage);
        assert!(enforced.require_adapter_report_field_coverage);
        assert!(enforced.require_operational_failures_not_quality);
        assert!(enforced.may_block_current_runner());
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn adapter_rollback_resume_boundary_plan_does_not_execute_resume_actions() {
        let report = AdapterRollbackResumeBoundaryPlan::rollback_resume_boundary(
            "rollback-resume-boundary-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = AdapterRollbackResumeBoundaryPlan::rollback_resume_boundary(
            "rollback-resume-boundary-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert_eq!(
            enforced.eval_entrypoint,
            "RollbackResumeReport::from_gate_and_evidence"
        );
        assert!(enforced.allows_input("RollbackReport.resume_gate"));
        assert!(enforced.allows_input("RollbackResumeEvidence"));
        assert!(enforced.allows_input("RollbackResumeGate"));
        assert!(enforced.allows_input("AdapterReportEmissionReport::field_coverage_passed"));
        assert!(enforced.allows_resume_gate("none"));
        assert!(enforced.allows_resume_gate("chain_readiness_gate"));
        assert!(enforced.allows_resume_gate("runtime_backend_health_check"));
        assert!(enforced.allows_resume_gate("planned_validation_command"));
        assert!(!enforced.allows_resume_gate("spawn_validation_command"));
        assert!(enforced.produces_output("RollbackResumeReport"));
        assert!(enforced.forbids_capability("jsonl_io"));
        assert!(enforced.forbids_capability("http_sse"));
        assert!(enforced.forbids_capability("process_spawn"));
        assert!(enforced.forbids_capability("validation_command_spawn"));
        assert!(enforced.forbids_capability("model_call"));
        assert!(enforced.forbids_capability("rollback_action_execution"));
        assert!(enforced.forbids_capability("resume_action_execution"));
        assert!(enforced.forbids_capability("runner_state"));
        assert!(enforced.stays_pure_data_boundary());
        assert!(!report.may_block_current_runner());
        assert!(!enforced.may_block_current_runner());
        assert!(enforced.preserves_legacy_runner);
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn experiment_rollout_report_plan_is_advisory_until_enforced() {
        let report = ExperimentRolloutReportPlan::experiment_rollout(
            "experiment-rollout-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = ExperimentRolloutReportPlan::experiment_rollout(
            "experiment-rollout-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert_eq!(report.report_schema_name, "experiment_rollout_report_v1");
        assert!(!report.require_clean_context_rot);
        assert!(!report.may_block_current_runner());
        assert!(report.preserves_legacy_runner);
        assert!(enforced.require_clean_context_rot);
        assert!(enforced.may_block_current_runner());
    }

    #[test]
    fn experiment_kill_switch_report_plan_requires_escape_hatch_before_enforcement() {
        let report = ExperimentKillSwitchReportPlan::experiment_kill_switch(
            "experiment-kill-switch-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = ExperimentKillSwitchReportPlan::experiment_kill_switch(
            "experiment-kill-switch-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert_eq!(
            enforced.report_schema_name,
            "experiment_kill_switch_report_v1"
        );
        assert!(!report.require_kill_switch);
        assert!(!report.require_rollback_report);
        assert!(!report.require_rollback_resume_gate);
        assert!(!report.require_clean_context_rot);
        assert!(!report.require_owner_acknowledgement);
        assert!(!report.may_block_current_runner());
        assert!(enforced.require_kill_switch);
        assert!(enforced.require_rollback_report);
        assert!(enforced.require_rollback_resume_gate);
        assert!(enforced.require_clean_context_rot);
        assert!(enforced.require_owner_acknowledgement);
        assert!(enforced.may_block_current_runner());
        assert!(enforced.preserves_legacy_runner);
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn experiment_expansion_safety_report_plan_aggregates_pre_expansion_gates() {
        let report = ExperimentExpansionSafetyReportPlan::experiment_expansion_safety(
            "experiment-expansion-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = ExperimentExpansionSafetyReportPlan::experiment_expansion_safety(
            "experiment-expansion-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert_eq!(
            enforced.report_schema_name,
            "experiment_expansion_safety_report_v1"
        );
        assert!(!report.require_rollout_report);
        assert!(!report.require_kill_switch_report);
        assert!(!report.require_clean_context_rot);
        assert!(!report.require_rollback_resume);
        assert!(!report.require_model_pool_attribution);
        assert!(!report.require_adapter_report_emission);
        assert!(!report.require_adapter_report_field_coverage);
        assert!(!report.require_apple_silicon_development_effect);
        assert!(!report.require_promotion_window);
        assert!(!report.require_readiness);
        assert!(!report.require_steam_case_matrix);
        assert!(!report.require_validation_command_coverage);
        assert!(!report.require_root_adapter_ready);
        assert!(!report.may_block_current_runner());
        assert!(enforced.require_rollout_report);
        assert!(enforced.require_kill_switch_report);
        assert!(enforced.require_clean_context_rot);
        assert!(enforced.require_rollback_resume);
        assert!(enforced.require_model_pool_attribution);
        assert!(enforced.require_adapter_report_emission);
        assert!(enforced.require_adapter_report_field_coverage);
        assert!(enforced.require_apple_silicon_development_effect);
        assert!(enforced.require_promotion_window);
        assert!(enforced.require_readiness);
        assert!(enforced.require_steam_case_matrix);
        assert!(enforced.require_validation_command_coverage);
        assert!(enforced.require_root_adapter_ready);
        assert!(enforced.keep_operational_failures_out_of_quality);
        assert!(enforced.uses_operational_report_source(
            "SelfEvolutionReadinessReport::can_schedule_next_round"
        ));
        assert!(enforced.uses_operational_report_source("RollbackReport::present"));
        assert!(
            enforced
                .uses_operational_report_source("RollbackResumeReport::allow_unattended_rounds")
        );
        assert!(
            enforced
                .uses_operational_report_source("SteamCaseCoverageReport::allow_enforced_adapter")
        );
        assert!(
            enforced.uses_operational_report_source(
                "ValidationCommandCoverageReport::allow_next_round"
            )
        );
        assert!(!enforced.uses_operational_report_source("JsonlLedgerReader::scan"));
        assert!(!enforced.uses_operational_report_source("ValidationCommandExecutor::spawn"));
        assert!(enforced.forbids_capability("jsonl_io"));
        assert!(enforced.forbids_capability("file_io"));
        assert!(enforced.forbids_capability("http_sse"));
        assert!(enforced.forbids_capability("process_spawn"));
        assert!(enforced.forbids_capability("validation_command_spawn"));
        assert!(enforced.forbids_capability("model_call"));
        assert!(enforced.forbids_capability("runner_state"));
        assert!(enforced.stays_pure_data_boundary());
        assert!(enforced.may_block_current_runner());
        assert!(enforced.preserves_legacy_runner);
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn experiment_switch_matrix_report_plan_requires_enabled_flag_coverage() {
        let report = ExperimentSwitchMatrixReportPlan::experiment_switch_matrix(
            "experiment-switch-matrix-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = ExperimentSwitchMatrixReportPlan::experiment_switch_matrix(
            "experiment-switch-matrix-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert_eq!(
            enforced.report_schema_name,
            "experiment_switch_matrix_report_v1"
        );
        assert!(enforced.requires_input_report("experiment_rollout_report_v1"));
        assert!(enforced.requires_input_report("experiment_kill_switch_report_v1"));
        assert!(enforced.requires_input_report("experiment_expansion_safety_report_v1"));
        assert!(enforced.requires_report_field("experiment_switch.enabled_flag_names"));
        assert!(enforced.requires_report_field("experiment_switch.reported_enabled_flag_names"));
        assert!(enforced.requires_report_field("experiment_switch.missing_enabled_flag_reports"));
        assert!(enforced.requires_report_field("experiment_switch.duplicate_enabled_flag_reports"));
        assert!(enforced.requires_report_field("experiment_switch.unknown_enabled_flag_reports"));
        assert!(
            enforced.requires_report_field("experiment_switch.exactly_one_report_per_enabled_flag")
        );
        assert!(!report.require_enabled_flags_reported);
        assert!(!report.require_exactly_one_report_per_enabled_flag);
        assert!(!report.require_expansion_safety_passed);
        assert!(enforced.require_enabled_flags_reported);
        assert!(enforced.require_exactly_one_report_per_enabled_flag);
        assert!(enforced.require_expansion_safety_passed);
        assert!(enforced.preserves_legacy_runner);
        assert!(!report.may_block_current_runner());
        assert!(enforced.may_block_current_runner());
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn feedback_self_improve_report_plan_is_observational_until_enforced() {
        let report = FeedbackSelfImproveReportPlan::feedback_self_improve(
            "feedback-self-improve-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = FeedbackSelfImproveReportPlan::feedback_self_improve(
            "feedback-self-improve-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert_eq!(report.report_schema_name, "feedback_self_improve_report_v1");
        assert!(!report.require_closed_loop_gate);
        assert!(!report.may_block_current_runner());
        assert!(report.preserves_legacy_runner);
        assert!(enforced.require_closed_loop_gate);
        assert!(enforced.may_block_current_runner());
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn adapter_feedback_self_improve_boundary_plan_does_not_read_ledger_jsonl() {
        let report = AdapterFeedbackSelfImproveBoundaryPlan::feedback_self_improve_boundary(
            "feedback-self-improve-boundary-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = AdapterFeedbackSelfImproveBoundaryPlan::feedback_self_improve_boundary(
            "feedback-self-improve-boundary-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert!(enforced.exposes_entrypoint("LedgerSummary::from_records"));
        assert!(enforced.exposes_entrypoint("ReportGate::evaluate"));
        assert!(enforced.exposes_entrypoint("FeedbackSelfImproveReport::from_summary_and_gate"));
        assert!(enforced.exposes_entrypoint("FeedbackSelfImproveReport::from_records_and_gate"));
        assert!(enforced.allows_input("LedgerRecord"));
        assert!(enforced.allows_input("LedgerSummary"));
        assert!(enforced.allows_input("ReportGate"));
        assert!(enforced.allows_input("GateDecision"));
        assert!(!enforced.allows_input("JsonlReader"));
        assert!(!enforced.allows_input("RunnerLedger"));
        assert!(enforced.produces_output("LedgerSummary"));
        assert!(enforced.produces_output("GateDecision"));
        assert!(enforced.produces_output("FeedbackSelfImproveReport"));
        assert!(enforced.forbids_capability("jsonl_io"));
        assert!(enforced.forbids_capability("file_io"));
        assert!(enforced.forbids_capability("http_sse"));
        assert!(enforced.forbids_capability("process_spawn"));
        assert!(enforced.forbids_capability("validation_command_spawn"));
        assert!(enforced.forbids_capability("model_call"));
        assert!(enforced.forbids_capability("runner_state"));
        assert!(enforced.stays_pure_data_boundary());
        assert!(!report.may_block_current_runner());
        assert!(!enforced.may_block_current_runner());
        assert!(enforced.preserves_legacy_runner);
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn advice_continuation_report_plan_blocks_only_when_enforced() {
        let report = AdviceContinuationReportPlan::advice_continuation(
            "advice-continuation-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = AdviceContinuationReportPlan::advice_continuation(
            "advice-continuation-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert_eq!(enforced.report_schema_name, "advice_continuation_report_v1");
        assert_eq!(report.max_repeated_advice, 1);
        assert_eq!(report.max_invalid_advice, 1);
        assert_eq!(report.max_invalid_commands, 1);
        assert!(!report.require_latest_round_success);
        assert!(!report.may_block_current_runner());
        assert!(report.preserves_legacy_runner);
        assert_eq!(enforced.max_repeated_advice, 0);
        assert_eq!(enforced.max_invalid_advice, 0);
        assert_eq!(enforced.max_invalid_commands, 0);
        assert!(enforced.require_latest_round_success);
        assert!(enforced.may_block_current_runner());
        assert!(enforced.preserves_legacy_runner);
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn advice_continuation_boundary_plan_keeps_scanning_and_runner_outside_eval() {
        let report = AdviceContinuationBoundaryPlan::advice_continuation_boundary(
            "advice-continuation-boundary-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = AdviceContinuationBoundaryPlan::advice_continuation_boundary(
            "advice-continuation-boundary-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert!(
            enforced
                .exposes_entrypoint("AdviceContinuationEvidence::from_observations_and_summary")
        );
        assert!(enforced.exposes_entrypoint("AdviceContinuationGate::evaluate"));
        assert!(enforced.exposes_entrypoint("AdviceContinuationReport::from_gate_and_evidence"));
        assert!(
            enforced
                .exposes_entrypoint("AdviceContinuationReport::from_observations_summary_and_gate")
        );
        assert!(enforced.allows_input("AdviceContinuationObservation"));
        assert!(enforced.allows_input("LedgerSummary"));
        assert!(enforced.allows_input("AdviceContinuationEvidence"));
        assert!(enforced.allows_input("AdviceContinuationGate"));
        assert!(enforced.allows_input("GateDecision"));
        assert!(!enforced.allows_input("JsonlReader"));
        assert!(!enforced.allows_input("EvolutionLoopRunner"));
        assert!(enforced.produces_output("AdviceContinuationEvidence"));
        assert!(enforced.produces_output("GateDecision"));
        assert!(enforced.produces_output("AdviceContinuationReport"));
        assert!(enforced.forbids_capability("jsonl_io"));
        assert!(enforced.forbids_capability("file_io"));
        assert!(enforced.forbids_capability("report_directory_scan"));
        assert!(enforced.forbids_capability("http_sse"));
        assert!(enforced.forbids_capability("process_spawn"));
        assert!(enforced.forbids_capability("validation_command_spawn"));
        assert!(enforced.forbids_capability("model_call"));
        assert!(enforced.forbids_capability("daemon_control"));
        assert!(enforced.forbids_capability("runner_state"));
        assert!(enforced.stays_pure_data_boundary());
        assert!(!report.may_block_current_runner());
        assert!(!enforced.may_block_current_runner());
        assert!(enforced.preserves_legacy_runner);
    }

    #[test]
    fn self_evolution_continuity_report_plan_requires_feedback_carryover_before_enforcement() {
        let report = SelfEvolutionContinuityReportPlan::self_evolution_continuity(
            "self-evolution-continuity-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = SelfEvolutionContinuityReportPlan::self_evolution_continuity(
            "self-evolution-continuity-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert_eq!(
            enforced.report_schema_name,
            "self_evolution_continuity_report_v1"
        );
        assert!(!report.require_adjacent_rounds);
        assert!(!report.require_feedback_carryover);
        assert!(!report.require_self_improve_passed);
        assert!(!report.require_validation_passed);
        assert!(!report.may_block_current_runner());
        assert!(report.preserves_legacy_runner);
        assert!(enforced.require_adjacent_rounds);
        assert!(enforced.require_feedback_carryover);
        assert!(enforced.require_self_improve_passed);
        assert!(enforced.require_validation_passed);
        assert!(enforced.may_block_current_runner());
    }

    #[test]
    fn self_evolution_regression_report_plan_requires_stable_window_before_enforcement() {
        let report = SelfEvolutionRegressionReportPlan::self_evolution_regression(
            "self-evolution-regression-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = SelfEvolutionRegressionReportPlan::self_evolution_regression(
            "self-evolution-regression-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert_eq!(
            enforced.report_schema_name,
            "self_evolution_regression_report_v1"
        );
        assert_eq!(enforced.min_window_rounds, 3);
        assert!(!report.require_validation_not_regressed);
        assert!(!report.require_self_improve_not_regressed);
        assert!(!report.require_feedback_not_regressed);
        assert!(!report.may_block_current_runner());
        assert!(report.preserves_legacy_runner);
        assert!(enforced.require_validation_not_regressed);
        assert!(enforced.require_self_improve_not_regressed);
        assert!(enforced.require_feedback_not_regressed);
        assert!(enforced.may_block_current_runner());
    }

    #[test]
    fn self_evolution_unattended_prerequisite_plan_requires_recovery_and_coverage() {
        let report = SelfEvolutionUnattendedPrerequisiteReportPlan::unattended_prerequisites(
            "self-evolution-unattended-prerequisites-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = SelfEvolutionUnattendedPrerequisiteReportPlan::unattended_prerequisites(
            "self-evolution-unattended-prerequisites-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert_eq!(
            enforced.report_schema_name,
            "self_evolution_unattended_prerequisites_report_v1"
        );
        assert!(!report.require_continuity);
        assert!(!report.require_regression);
        assert!(!report.require_readiness_next_round);
        assert!(!report.require_context_rot_trend);
        assert!(!report.require_context_rot_remediation);
        assert!(!report.require_rollback_resume);
        assert!(!report.require_steam_case_matrix);
        assert!(!report.require_validation_command_coverage);
        assert!(!report.require_promotion_window);
        assert!(!report.require_adapter_report_field_coverage);
        assert!(!report.require_apple_silicon_development_effect);
        assert!(!report.may_block_current_runner());
        assert!(report.preserves_legacy_runner);
        assert!(enforced.require_continuity);
        assert!(enforced.require_regression);
        assert!(enforced.require_readiness_next_round);
        assert!(enforced.require_context_rot_trend);
        assert!(enforced.require_context_rot_remediation);
        assert!(enforced.require_rollback_resume);
        assert!(enforced.require_steam_case_matrix);
        assert!(enforced.require_validation_command_coverage);
        assert!(enforced.require_promotion_window);
        assert!(enforced.require_adapter_report_field_coverage);
        assert!(enforced.require_apple_silicon_development_effect);
        assert!(enforced.may_block_current_runner());
    }

    #[test]
    fn self_evolution_unattended_prerequisite_plan_maps_pure_report_sources() {
        let enforced = SelfEvolutionUnattendedPrerequisiteReportPlan::unattended_prerequisites(
            "self-evolution-unattended-prerequisites-sources",
            AdapterAcceptanceStage::Enforced,
        );

        assert!(enforced.uses_operational_report_source(
            "SelfEvolutionReadinessReport::can_schedule_next_round"
        ));
        assert!(enforced.uses_operational_report_source(
            "ContextRotTrendReport::allow_unattended_continuation"
        ));
        assert!(enforced.uses_operational_report_source(
            "ContextRotRemediationReport::allow_experiment_rollout"
        ));
        assert!(
            enforced
                .uses_operational_report_source("RollbackResumeReport::allow_unattended_rounds")
        );
        assert!(
            enforced
                .uses_operational_report_source("SteamCaseCoverageReport::allow_enforced_adapter")
        );
        assert!(
            enforced.uses_operational_report_source(
                "ValidationCommandCoverageReport::allow_next_round"
            )
        );
        assert!(!enforced.uses_operational_report_source(
            "ValidationCommandCoverageReport::allow_enforced_adapter"
        ));
        assert!(
            enforced
                .uses_operational_report_source("AdapterPromotionWindowReport::allow_enforcement")
        );
        assert!(!enforced.uses_operational_report_source(
            "AdapterPromotionWindowReport::allow_enforcement_promotion"
        ));
        assert!(
            enforced.uses_operational_report_source(
                "AdapterReportEmissionReport::field_coverage_passed"
            )
        );
        assert!(enforced.uses_operational_report_source(
            "AppleSiliconDevelopmentEffectReport::allow_development_effect_claim"
        ));
        assert!(!enforced.uses_operational_report_source("JsonlLedgerReader::scan"));
        assert!(!enforced.uses_operational_report_source("ValidationCommandExecutor::run"));
        assert!(!enforced.uses_operational_report_source("EvolutionLoopRunner::state"));
        assert!(enforced.forbids_capability("jsonl_io"));
        assert!(enforced.forbids_capability("http_sse"));
        assert!(enforced.forbids_capability("process_spawn"));
        assert!(enforced.forbids_capability("validation_command_spawn"));
        assert!(enforced.forbids_capability("model_call"));
        assert!(enforced.forbids_capability("runner_state"));
        assert!(enforced.stays_pure_data_boundary());
    }

    #[test]
    fn strict_unattended_acceptance_plan_collects_allow_flags_without_runner_wiring() {
        let report = StrictUnattendedAcceptancePlan::strict_unattended_acceptance(
            "strict-unattended-acceptance-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = StrictUnattendedAcceptancePlan::strict_unattended_acceptance(
            "strict-unattended-acceptance-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert_eq!(
            enforced.report_schema_name,
            "strict_unattended_acceptance_report_v1"
        );
        assert!(enforced.exposes_entrypoint("StrictUnattendedSupervisorEvidence::from_status"));
        assert!(enforced.exposes_entrypoint("StrictUnattendedAcceptanceReport::from_reports"));
        assert!(enforced.allows_input("StrictUnattendedSupervisorEvidence"));
        assert!(enforced.allows_input("RunModeReportRefreshAcceptanceReport"));
        assert!(enforced.allows_input("AdapterClosureReport"));
        assert!(enforced.allows_input("ValidationCommandCoverageReport"));
        assert!(enforced.allows_input("RollbackResumeReport"));
        assert!(enforced.allows_input("SelfEvolutionRegressionReport"));
        assert!(enforced.allows_input("SelfEvolutionReadinessReport"));
        assert!(enforced.allows_input("SelfEvolutionUnattendedPrerequisiteReport"));
        assert!(!enforced.allows_input("EvolutionLoopRunner"));
        assert!(!enforced.allows_input("DaemonHandle"));
        assert!(!enforced.allows_input("ValidationCommandExecutor"));
        assert!(!enforced.allows_input("HelperStageContractSummary"));
        assert!(enforced.requires_input_report("run_mode_report_refresh_acceptance_report_v1"));
        assert!(enforced.requires_input_report("adapter_closure_report_v1"));
        assert!(enforced.requires_input_report("validation_command_coverage_report_v1"));
        assert!(enforced.requires_input_report("rollback_resume_report_v1"));
        assert!(enforced.requires_input_report("self_evolution_regression_report_v1"));
        assert!(enforced.requires_input_report("readiness_next_round_v1"));
        assert!(
            enforced.requires_input_report("self_evolution_unattended_prerequisites_report_v1")
        );
        assert!(enforced.produces_output("StrictUnattendedAcceptanceReport"));
        assert!(
            enforced
                .produces_report_field("strict_unattended_acceptance.supervisor_allow_next_round")
        );
        assert!(enforced.produces_report_field("strict_unattended_acceptance.stale_pid_detected"));
        assert!(enforced.produces_report_field("strict_unattended_acceptance.starts_process"));
        assert!(enforced.produces_report_field("strict_unattended_acceptance.sends_prompt"));
        assert!(enforced.produces_report_field("strict_unattended_acceptance.touches_remote"));
        assert!(enforced.produces_report_field(
            "strict_unattended_acceptance.adapter_closure_allow_next_round"
        ));
        assert!(
            enforced
                .produces_report_field("strict_unattended_acceptance.validation_allow_next_round")
        );
        assert!(enforced.produces_report_field(
            "strict_unattended_acceptance.rollback_resume_allow_unattended_rounds"
        ));
        assert!(enforced.produces_report_field(
            "strict_unattended_acceptance.self_improve_regression_allow_unattended_continuation"
        ));
        assert!(enforced.produces_report_field(
            "strict_unattended_acceptance.readiness_can_schedule_next_round"
        ));
        assert!(enforced.produces_report_field(
            "strict_unattended_acceptance.self_evolution_allow_unattended_claim"
        ));
        assert!(enforced.produces_report_field("strict_unattended_acceptance.allow_next_round"));
        assert!(enforced.produces_report_field("strict_unattended_acceptance.failure_reasons"));
        for forbidden in [
            "jsonl_io",
            "file_io",
            "http_sse",
            "process_spawn",
            "validation_command_spawn",
            "daemon_control",
            "runner_state",
            "remote_mac_call",
            "model_call",
            "helper_prose_parse",
            "chat_stream",
        ] {
            assert!(
                enforced.forbids_capability(forbidden),
                "missing forbidden capability {forbidden}"
            );
        }
        assert!(enforced.stays_pure_data_boundary());
        assert!(!report.may_block_current_runner());
        assert!(!enforced.may_block_current_runner());
        assert!(enforced.preserves_legacy_runner);
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn worker_window_replacement_plan_tracks_clean_room_contract_fields() {
        let report = WorkerWindowReplacementPlan::worker_window_replacement(
            "worker-window-replacement-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = WorkerWindowReplacementPlan::worker_window_replacement(
            "worker-window-replacement-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert_eq!(
            enforced.report_schema_name,
            "worker_window_replacement_report_v1"
        );
        assert!(enforced.exposes_entrypoint("WorkerWindowReplacementEvidence::clean"));
        assert!(enforced.exposes_entrypoint("WorkerWindowReplacementEvidence::with_evidence_ids"));
        assert!(enforced.exposes_entrypoint("WorkerWindowReplacementEvidence::with_status"));
        assert!(enforced.exposes_entrypoint("WorkerWindowReplacementGate::evaluate"));
        assert!(
            enforced.exposes_entrypoint("WorkerWindowReplacementReport::from_gate_and_evidence")
        );
        assert!(enforced.allows_input("WorkerWindowReplacementEvidence"));
        assert!(enforced.allows_input("WorkerWindowReplacementGate"));
        assert!(enforced.allows_input("evidence_ids"));
        assert!(enforced.allows_input("paused"));
        assert!(enforced.allows_input("polluted"));
        assert!(enforced.allows_input("stale"));
        assert!(enforced.allows_input("clean_room_replacement_required"));
        assert!(!enforced.allows_input("OldThreadReader"));
        assert!(!enforced.allows_input("WorkerWindowMutator"));
        assert!(!enforced.allows_input("EvolutionLoopRunner"));
        assert!(enforced.produces_output("WorkerWindowReplacementReport"));
        assert!(enforced.produces_report_field("worker_window_replacement.evidence_ids"));
        assert!(enforced.produces_report_field("worker_window_replacement.paused"));
        assert!(enforced.produces_report_field("worker_window_replacement.polluted"));
        assert!(enforced.produces_report_field("worker_window_replacement.stale"));
        assert!(
            enforced
                .produces_report_field("worker_window_replacement.clean_room_replacement_required")
        );
        assert!(enforced.produces_report_field("worker_window_replacement.no_old_thread_reads"));
        assert!(enforced.produces_report_field("worker_window_replacement.no_side_effects"));
        assert!(enforced.produces_report_field("worker_window_replacement.evidence_ids_only"));
        assert!(
            enforced.produces_report_field("worker_window_replacement.allow_worker_continuation")
        );
        for forbidden in [
            "old_thread_read",
            "old_window_read",
            "chat_transcript_read",
            "jsonl_io",
            "file_io",
            "http_sse",
            "process_spawn",
            "validation_command_spawn",
            "daemon_control",
            "runner_state",
            "remote_mac_call",
            "model_call",
            "helper_prose_parse",
            "chat_stream",
            "side_effect_execution",
        ] {
            assert!(
                enforced.forbids_capability(forbidden),
                "missing forbidden capability {forbidden}"
            );
        }
        assert!(enforced.stays_pure_data_boundary());
        assert!(!report.may_block_current_runner());
        assert!(!enforced.may_block_current_runner());
        assert!(enforced.preserves_legacy_runner);
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn status_driven_self_evolution_closure_plan_tracks_r24_clean_room_contract_fields() {
        let report = StatusDrivenSelfEvolutionClosurePlan::status_driven_self_evolution_closure(
            "status-driven-self-evolution-closure-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = StatusDrivenSelfEvolutionClosurePlan::status_driven_self_evolution_closure(
            "status-driven-self-evolution-closure-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert_eq!(
            enforced.report_schema_name,
            "status_driven_self_evolution_closure_report_v1"
        );
        assert!(
            enforced.exposes_entrypoint("StatusDrivenSelfEvolutionClosureEvidence::r24_clean_room")
        );
        assert!(enforced.exposes_entrypoint(
            "StatusDrivenSelfEvolutionClosureEvidence::with_memory_startup_admission_safe"
        ));
        assert!(enforced.exposes_entrypoint(
            "StatusDrivenSelfEvolutionClosureEvidence::with_worker_replacement_required"
        ));
        assert!(enforced.exposes_entrypoint(
            "StatusDrivenSelfEvolutionClosureEvidence::with_clean_room_assignment_allowed"
        ));
        assert!(enforced.exposes_entrypoint("StatusDrivenSelfEvolutionClosureGate::evaluate"));
        assert!(
            enforced.exposes_entrypoint(
                "StatusDrivenSelfEvolutionClosureReport::from_gate_and_evidence"
            )
        );
        assert!(enforced.allows_input("StatusDrivenSelfEvolutionClosureEvidence"));
        assert!(enforced.allows_input("StatusDrivenSelfEvolutionClosureGate"));
        assert!(enforced.allows_input("evidence_ids"));
        assert!(enforced.allows_input("memory_startup_admission_safe"));
        assert!(enforced.allows_input("worker_replacement_required"));
        assert!(enforced.allows_input("clean_room_assignment_allowed"));
        assert!(enforced.allows_input("report_only_continuation"));
        assert!(!enforced.allows_input("OldThreadReader"));
        assert!(!enforced.allows_input("LiveWriteExecutor"));
        assert!(!enforced.allows_input("RuntimeSideEffectExecutor"));
        assert!(!enforced.allows_input("EvolutionLoopRunner"));
        assert!(enforced.produces_output("StatusDrivenSelfEvolutionClosureReport"));
        assert!(enforced.produces_report_field(
            "status_driven_self_evolution_closure.memory_startup_admission_safe"
        ));
        assert!(enforced.produces_report_field(
            "status_driven_self_evolution_closure.worker_replacement_required"
        ));
        assert!(enforced.produces_report_field(
            "status_driven_self_evolution_closure.clean_room_assignment_allowed"
        ));
        assert!(
            enforced
                .produces_report_field("status_driven_self_evolution_closure.no_old_thread_reads")
        );
        assert!(
            enforced.produces_report_field("status_driven_self_evolution_closure.no_live_writes")
        );
        assert!(
            enforced.produces_report_field(
                "status_driven_self_evolution_closure.no_runtime_side_effects"
            )
        );
        assert!(enforced.produces_report_field(
            "status_driven_self_evolution_closure.allow_report_only_continuation"
        ));
        assert!(enforced.produces_report_field(
            "status_driven_self_evolution_closure.allow_runtime_continuation"
        ));
        for forbidden in [
            "old_thread_read",
            "old_window_read",
            "chat_transcript_read",
            "jsonl_io",
            "file_io",
            "http_sse",
            "process_spawn",
            "validation_command_spawn",
            "daemon_control",
            "runner_state",
            "remote_mac_call",
            "model_call",
            "helper_prose_parse",
            "chat_stream",
            "live_write",
            "memory_store_write",
            "ndkv_write",
            "runtime_side_effect_execution",
        ] {
            assert!(
                enforced.forbids_capability(forbidden),
                "missing forbidden capability {forbidden}"
            );
        }
        assert!(enforced.stays_pure_data_boundary());
        assert!(!report.may_block_current_runner());
        assert!(!enforced.may_block_current_runner());
        assert!(enforced.preserves_legacy_runner);
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn clean_room_handoff_report_plan_tracks_r25_contract_fields() {
        let report = CleanRoomHandoffReportPlan::clean_room_handoff_report(
            "clean-room-handoff-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = CleanRoomHandoffReportPlan::clean_room_handoff_report(
            "clean-room-handoff-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert_eq!(enforced.report_schema_name, "clean_room_handoff_report_v1");
        assert!(enforced.exposes_entrypoint("CleanRoomHandoffEvidence::r25_clean_room_handoff"));
        assert!(
            enforced.exposes_entrypoint("CleanRoomHandoffEvidence::with_memory_startup_admission")
        );
        assert!(
            enforced.exposes_entrypoint("CleanRoomHandoffEvidence::with_agent_replacement_plan")
        );
        assert!(
            enforced.exposes_entrypoint("CleanRoomHandoffEvidence::with_source_json_parse_flags")
        );
        assert!(enforced.exposes_entrypoint("CleanRoomHandoffEvidence::side_effects_all_false"));
        assert!(enforced.exposes_entrypoint("CleanRoomHandoffGate::evaluate"));
        assert!(enforced.exposes_entrypoint("CleanRoomHandoffReport::from_gate_and_evidence"));
        assert!(enforced.allows_input("CleanRoomHandoffEvidence"));
        assert!(enforced.allows_input("CleanRoomHandoffGate"));
        assert!(enforced.allows_input("memory_startup_admission_json"));
        assert!(enforced.allows_input("agent_clean_room_replacement_plan_json"));
        assert!(enforced.allows_input("memory_startup_admission_input_present"));
        assert!(enforced.allows_input("memory_startup_admission_safe"));
        assert!(enforced.allows_input("agent_replacement_plan_input_present"));
        assert!(enforced.allows_input("agent_replacement_plan_clean_room_required"));
        assert!(enforced.allows_input("source_json"));
        assert!(enforced.allows_input("report_only_continuation"));
        assert!(!enforced.allows_input("OldThreadReader"));
        assert!(!enforced.allows_input("SourceJsonPromptParser"));
        assert!(!enforced.allows_input("LiveWriteExecutor"));
        assert!(!enforced.allows_input("EvolutionLoopRunner"));
        assert!(enforced.produces_output("CleanRoomHandoffReport"));
        assert!(
            enforced
                .produces_report_field("clean_room_handoff.memory_startup_admission_input_present")
        );
        assert!(enforced.produces_report_field("clean_room_handoff.memory_startup_admission_safe"));
        assert!(
            enforced
                .produces_report_field("clean_room_handoff.agent_replacement_plan_input_present")
        );
        assert!(enforced.produces_report_field(
            "clean_room_handoff.agent_replacement_plan_clean_room_required"
        ));
        assert!(enforced.produces_report_field("clean_room_handoff.source_json_retained"));
        assert!(enforced.produces_report_field(
            "clean_room_handoff.source_json_not_parsed_as_prompt_or_live_write"
        ));
        assert!(enforced.produces_report_field("clean_room_handoff.side_effects_all_false"));
        assert!(
            enforced.produces_report_field("clean_room_handoff.allow_report_only_continuation")
        );
        assert!(enforced.produces_report_field("clean_room_handoff.allow_runtime_continuation"));
        for forbidden in [
            "old_thread_read",
            "old_window_read",
            "chat_transcript_read",
            "jsonl_io",
            "file_io",
            "http_sse",
            "process_spawn",
            "validation_command_spawn",
            "daemon_control",
            "runner_state",
            "remote_mac_call",
            "model_call",
            "helper_prose_parse",
            "chat_stream",
            "prompt_parse",
            "live_write",
            "memory_store_write",
            "ndkv_write",
            "runtime_side_effect_execution",
        ] {
            assert!(
                enforced.forbids_capability(forbidden),
                "missing forbidden capability {forbidden}"
            );
        }
        assert!(enforced.stays_pure_data_boundary());
        assert!(!report.may_block_current_runner());
        assert!(!enforced.may_block_current_runner());
        assert!(enforced.preserves_legacy_runner);
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn self_improve_proposal_acceptance_plan_tracks_r26_contract_fields() {
        let report = SelfImproveProposalAcceptancePlan::self_improve_proposal_acceptance(
            "self-improve-proposal-acceptance-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = SelfImproveProposalAcceptancePlan::self_improve_proposal_acceptance(
            "self-improve-proposal-acceptance-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert_eq!(
            enforced.report_schema_name,
            "self_improve_proposal_acceptance_v1"
        );
        assert!(enforced.exposes_entrypoint("SelfImproveProposalEvidence::clean_candidate"));
        assert!(enforced.exposes_entrypoint("SelfImproveProposalEvidence::with_validation"));
        assert!(enforced.exposes_entrypoint("SelfImproveProposalEvidence::safe_command_source"));
        assert!(enforced.exposes_entrypoint(
            "SelfImproveProposalEvidence::clean_gist_without_raw_old_window_payload"
        ));
        assert!(
            enforced.exposes_entrypoint("SelfImproveProposalEvidence::memory_admission_accepted")
        );
        assert!(enforced.exposes_entrypoint(
            "SelfImproveProposalEvidence::evidence_backed_business_improvement"
        ));
        assert!(enforced.exposes_entrypoint("SelfImproveMemoryAdmissionCandidate::accepted"));
        assert!(enforced.exposes_entrypoint("SelfImproveMemoryAdmissionCandidate::quarantined"));
        assert!(enforced.exposes_entrypoint("SelfImproveProposalAcceptanceGate::evaluate"));
        assert!(
            enforced
                .exposes_entrypoint("SelfImproveProposalAcceptanceReport::from_gate_and_evidence")
        );
        assert!(enforced.allows_input("SelfImproveProposalEvidence"));
        assert!(enforced.allows_input("SelfImproveProposalAcceptanceGate"));
        assert!(enforced.allows_input("SelfImproveMemoryAdmissionCandidate"));
        assert!(enforced.allows_input("source_round"));
        assert!(enforced.allows_input("evidence_ids"));
        assert!(enforced.allows_input("validation_checked"));
        assert!(enforced.allows_input("validation_passed"));
        assert!(enforced.allows_input("validation_command_source"));
        assert!(enforced.allows_input("validation_command_safe"));
        assert!(enforced.allows_input("clean_gist"));
        assert!(enforced.allows_input("raw_old_window_payload_present"));
        assert!(enforced.allows_input("side_effect_attempts"));
        assert!(enforced.allows_input("memory_admission_candidate"));
        assert!(!enforced.allows_input("OldThreadReader"));
        assert!(!enforced.allows_input("OldWindowPayload"));
        assert!(!enforced.allows_input("ValidationCommandExecutor"));
        assert!(!enforced.allows_input("PromotionExecutor"));
        assert!(!enforced.allows_input("EvolutionLoopRunner"));
        assert!(enforced.produces_output("SelfImproveProposalAcceptanceReport"));
        assert!(enforced.produces_report_field("self_improve_proposal.source_round"));
        assert!(enforced.produces_report_field("self_improve_proposal.evidence_ids"));
        assert!(enforced.produces_report_field("self_improve_proposal.validation_checked"));
        assert!(enforced.produces_report_field("self_improve_proposal.validation_passed"));
        assert!(enforced.produces_report_field("self_improve_proposal.validation_command_source"));
        assert!(enforced.produces_report_field("self_improve_proposal.clean_gist"));
        assert!(enforced.produces_report_field("self_improve_proposal.memory_admission_decision"));
        assert!(enforced.produces_report_field("self_improve_proposal.memory_admission_reasons"));
        assert!(enforced.produces_report_field("self_improve_proposal.safe_command_source"));
        assert!(enforced.produces_report_field(
            "self_improve_proposal.clean_gist_without_raw_old_window_payload"
        ));
        assert!(enforced.produces_report_field("self_improve_proposal.no_raw_old_window_payload"));
        assert!(
            enforced.produces_report_field("self_improve_proposal.runtime_side_effects_all_false")
        );
        assert!(enforced.produces_report_field(
            "self_improve_proposal.memory_admission_candidate_decided_with_reasons"
        ));
        assert!(enforced.produces_report_field("self_improve_proposal.memory_admission_accepted"));
        assert!(
            enforced.produces_report_field(
                "self_improve_proposal.evidence_backed_business_improvement"
            )
        );
        assert!(enforced.produces_report_field("self_improve_proposal.advisory_only"));
        assert!(enforced.produces_report_field("self_improve_proposal.allow_promotion"));
        assert!(enforced.produces_report_field("self_improve_proposal.require_repair"));
        assert!(enforced.produces_report_field("self_improve_proposal.failure_reasons"));
        for forbidden in [
            "old_thread_read",
            "old_window_read",
            "raw_old_window_payload",
            "chat_transcript_read",
            "jsonl_io",
            "file_io",
            "http_sse",
            "process_spawn",
            "validation_command_spawn",
            "daemon_control",
            "runner_state",
            "remote_mac_call",
            "model_download",
            "model_call",
            "helper_prose_parse",
            "chat_stream",
            "prompt_parse",
            "live_write",
            "memory_store_write",
            "ndkv_write",
            "runtime_side_effect_execution",
            "promotion_action_execution",
        ] {
            assert!(
                enforced.forbids_capability(forbidden),
                "missing forbidden capability {forbidden}"
            );
        }
        assert!(enforced.stays_pure_data_boundary());
        assert!(!report.may_block_current_runner());
        assert!(!enforced.may_block_current_runner());
        assert!(enforced.preserves_legacy_runner);
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn self_improve_proposal_action_assignment_plan_tracks_first_target_digest_contract() {
        let report =
            SelfImproveProposalActionAssignmentPlan::self_improve_proposal_action_assignment(
                "self-improve-proposal-action-assignment-report",
                AdapterAcceptanceStage::ReportOnly,
            );
        let enforced =
            SelfImproveProposalActionAssignmentPlan::self_improve_proposal_action_assignment(
                "self-improve-proposal-action-assignment-enforced",
                AdapterAcceptanceStage::Enforced,
            );

        assert_eq!(
            enforced.report_schema_name,
            "self_improve_proposal_action_assignment_v1"
        );
        assert!(enforced.exposes_entrypoint("SelfImproveProposalActionPlan::from_summary"));
        assert!(
            enforced
                .exposes_entrypoint("SelfImproveProposalActionAssignment::from_reports_and_plan")
        );
        assert!(
            enforced.exposes_entrypoint("SelfImproveProposalActionAssignment::first_target_digest")
        );
        assert!(enforced.exposes_entrypoint(
            "SelfImproveProposalActionAssignmentFirstTargetDigest::from_target"
        ));
        assert!(enforced.allows_input("SelfImproveProposalAcceptanceReport"));
        assert!(enforced.allows_input("SelfImproveProposalAcceptanceSummaryReport"));
        assert!(enforced.allows_input("SelfImproveProposalActionPlan"));
        assert!(enforced.allows_input("SelfImproveProposalActionAssignmentTarget"));
        assert!(!enforced.allows_input("EvolutionLoopRunner"));
        assert!(!enforced.allows_input("ValidationCommandExecutor"));
        assert!(!enforced.allows_input("PromotionExecutor"));
        assert!(enforced.produces_output("SelfImproveProposalActionPlan"));
        assert!(enforced.produces_output("SelfImproveProposalActionAssignment"));
        assert!(enforced.produces_output("SelfImproveProposalActionAssignmentTarget"));
        assert!(enforced.produces_output("SelfImproveProposalActionAssignmentFirstTargetDigest"));
        assert!(
            enforced
                .produces_report_field("self_improve_proposal.action_assignment.action_required")
        );
        assert!(
            enforced
                .produces_report_field("self_improve_proposal.action_assignment.primary_action")
        );
        assert!(
            enforced.produces_report_field("self_improve_proposal.action_assignment.target_count")
        );
        assert!(enforced.produces_report_field(
            "self_improve_proposal.action_assignment.first_target.proposal_id"
        ));
        assert!(enforced.produces_report_field(
            "self_improve_proposal.action_assignment.first_target.source_round"
        ));
        assert!(enforced.produces_report_field(
            "self_improve_proposal.action_assignment.first_target.evidence_ids"
        ));
        assert!(enforced.produces_report_field(
            "self_improve_proposal.action_assignment.first_target.current_memory_admission_decision"
        ));
        assert!(enforced.produces_report_field(
            "self_improve_proposal.action_assignment.first_target.validation_checked"
        ));
        assert!(enforced.produces_report_field(
            "self_improve_proposal.action_assignment.first_target.validation_passed"
        ));
        assert!(enforced.produces_report_field(
            "self_improve_proposal.action_assignment.first_target.memory_admission_accepted"
        ));
        assert!(enforced.produces_report_field(
            "self_improve_proposal.action_assignment.first_target.evidence_backed_business_improvement"
        ));
        assert!(enforced.produces_report_field(
            "self_improve_proposal.action_assignment.first_target.advisory_only"
        ));
        assert!(enforced.produces_report_field(
            "self_improve_proposal.action_assignment.first_target.require_repair"
        ));
        assert!(enforced.produces_report_field(
            "self_improve_proposal.action_assignment.first_target.missing_requirements"
        ));
        for forbidden in [
            "old_thread_read",
            "old_window_read",
            "raw_old_window_payload",
            "jsonl_io",
            "file_io",
            "http_sse",
            "process_spawn",
            "validation_command_spawn",
            "daemon_control",
            "runner_state",
            "remote_mac_call",
            "model_download",
            "model_call",
            "helper_prose_parse",
            "chat_stream",
            "prompt_parse",
            "live_write",
            "memory_store_write",
            "ndkv_write",
            "runtime_side_effect_execution",
            "promotion_action_execution",
        ] {
            assert!(
                enforced.forbids_capability(forbidden),
                "missing forbidden capability {forbidden}"
            );
        }
        assert!(enforced.stays_pure_data_boundary());
        assert!(!report.may_block_current_runner());
        assert!(!enforced.may_block_current_runner());
        assert!(enforced.preserves_legacy_runner);
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
        assert!(report.stays_pure_data_boundary());
    }

    #[test]
    fn helper_stage_repair_plan_tracks_r28_contract_fields() {
        let report = HelperStageRepairPlan::helper_stage_repair(
            "helper-stage-repair-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = HelperStageRepairPlan::helper_stage_repair(
            "helper-stage-repair-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert_eq!(enforced.report_schema_name, "helper_stage_repair_report_v1");
        assert!(enforced.exposes_entrypoint("HelperStageRepairRoleEvidence::from_summary"));
        assert!(enforced.exposes_entrypoint("HelperStageRepairEvidence::from_role_summaries"));
        assert!(enforced.exposes_entrypoint("HelperStageRepairEvidence::with_required_roles"));
        assert!(enforced.exposes_entrypoint("HelperStageRepairGate::evaluate"));
        assert!(enforced.exposes_entrypoint("HelperStageRepairReport::from_gate_and_evidence"));
        assert!(enforced.allows_input("HelperStageContractSummary"));
        assert!(enforced.allows_input("HelperStageRepairGate"));
        assert!(enforced.allows_input("role"));
        assert!(enforced.allows_input("required_roles"));
        assert!(enforced.allows_input("fields"));
        assert!(enforced.allows_input("matched_markers"));
        assert!(enforced.allows_input("expected_markers"));
        assert!(enforced.allows_input("latest_preview"));
        assert!(!enforced.allows_input("OldThreadReader"));
        assert!(!enforced.allows_input("OldWindowPayload"));
        assert!(!enforced.allows_input("ValidationCommandExecutor"));
        assert!(!enforced.allows_input("RepairActionExecutor"));
        assert!(!enforced.allows_input("EvolutionLoopRunner"));
        assert!(enforced.produces_output("HelperStageRepairRoleEvidence"));
        assert!(enforced.produces_output("HelperStageRepairEvidence"));
        assert!(enforced.produces_output("GateDecision"));
        assert!(enforced.produces_output("HelperStageRepairReport"));
        assert!(enforced.produces_report_field("helper_stage_repair.role_count"));
        assert!(enforced.produces_report_field("helper_stage_repair.roles"));
        assert!(enforced.produces_report_field("helper_stage_repair.required_roles"));
        assert!(enforced.produces_report_field("helper_stage_repair.missing_required_roles"));
        assert!(enforced.produces_report_field("helper_stage_repair.incomplete_roles"));
        assert!(enforced.produces_report_field("helper_stage_repair.present_but_incomplete_roles"));
        assert!(enforced.produces_report_field("helper_stage_repair.missing_fields_by_role"));
        assert!(enforced.produces_report_field("helper_stage_repair.placeholder_fields_by_role"));
        assert!(enforced.produces_report_field("helper_stage_repair.missing_markers_by_role"));
        assert!(enforced.produces_report_field("helper_stage_repair.repair_actions"));
        assert!(
            enforced.produces_report_field("helper_stage_repair.helper_stage_contract_complete")
        );
        assert!(
            enforced.produces_report_field("helper_stage_repair.allow_helper_stage_acceptance")
        );
        assert!(enforced.produces_report_field("helper_stage_repair.require_repair"));
        assert!(enforced.produces_report_field("helper_stage_repair.failure_reasons"));
        for forbidden in [
            "old_thread_read",
            "old_window_read",
            "chat_transcript_read",
            "jsonl_io",
            "file_io",
            "http_sse",
            "process_spawn",
            "validation_command_spawn",
            "daemon_control",
            "runner_state",
            "remote_mac_call",
            "model_download",
            "model_call",
            "helper_prose_parse",
            "chat_stream",
            "prompt_parse",
            "live_write",
            "memory_store_write",
            "ndkv_write",
            "runtime_side_effect_execution",
            "repair_action_execution",
        ] {
            assert!(
                enforced.forbids_capability(forbidden),
                "missing forbidden capability {forbidden}"
            );
        }
        assert!(enforced.stays_pure_data_boundary());
        assert!(!report.may_block_current_runner());
        assert!(!enforced.may_block_current_runner());
        assert!(enforced.preserves_legacy_runner);
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn ledger_gate_report_plan_is_observational_until_enforced() {
        let report = LedgerGateReportPlan::ledger_gate(
            "ledger-gate-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = LedgerGateReportPlan::ledger_gate(
            "ledger-gate-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert_eq!(report.report_schema_name, "ledger_gate_report_v1");
        assert!(!report.require_strict_ledger_hygiene);
        assert!(!report.require_last_success_policy);
        assert!(!report.may_block_current_runner());
        assert!(report.preserves_legacy_runner);
        assert!(enforced.require_strict_ledger_hygiene);
        assert!(enforced.require_last_success_policy);
        assert!(enforced.may_block_current_runner());
        assert_eq!(
            LedgerGateReportPlan::downstream_projection_field_mappings(),
            vec![
                (
                    "ledger.total_rounds",
                    "report_freshness.ledger_gate_total_rounds"
                ),
                (
                    "ledger.gate_blocked",
                    "report_freshness.ledger_gate_blocked"
                ),
                (
                    "ledger.allow_next_round",
                    "report_freshness.ledger_gate_allow_next_round",
                ),
                (
                    "ledger.allow_next_round",
                    "run_mode_report_refresh.ledger_gate_allow_next_round",
                ),
            ]
        );
        assert!(
            !LedgerGateReportPlan::downstream_projection_field_mappings()
                .iter()
                .any(|(ledger_field, downstream_field)| {
                    *ledger_field == "ledger.failure_reasons"
                        || *downstream_field == "report_freshness.ledger_gate_failure_reasons"
                        || *downstream_field == "run_mode_report_refresh.ledger_failure_reasons"
                })
        );
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn adapter_ledger_gate_boundary_plan_keeps_jsonl_reader_outside_eval() {
        let report = AdapterLedgerGateBoundaryPlan::ledger_gate(
            "ledger-gate-boundary-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = AdapterLedgerGateBoundaryPlan::ledger_gate(
            "ledger-gate-boundary-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert!(enforced.exposes_entrypoint("LedgerSummary::from_records"));
        assert!(enforced.exposes_entrypoint("ReportGate::evaluate"));
        assert!(enforced.exposes_entrypoint("LedgerGateReport::from_summary_and_gate"));
        assert!(enforced.exposes_entrypoint("LedgerGateReport::from_records_and_gate"));
        assert!(enforced.allows_input("LedgerRecord"));
        assert!(enforced.allows_input("LedgerSummary"));
        assert!(enforced.allows_input("ReportGate"));
        assert!(!enforced.allows_input("JsonlLedgerReader"));
        assert!(!enforced.allows_input("EvolutionLoopRunner"));
        assert!(!enforced.allows_input("ValidationCommandExecutor"));
        assert!(enforced.produces_output("LedgerSummary"));
        assert!(enforced.produces_output("GateDecision"));
        assert!(enforced.produces_output("LedgerGateReport"));
        assert!(enforced.forbids_capability("jsonl_io"));
        assert!(enforced.forbids_capability("file_io"));
        assert!(enforced.forbids_capability("http_sse"));
        assert!(enforced.forbids_capability("process_spawn"));
        assert!(enforced.forbids_capability("validation_command_spawn"));
        assert!(enforced.forbids_capability("model_call"));
        assert!(enforced.forbids_capability("runner_state"));
        assert!(enforced.stays_pure_data_boundary());
        assert!(!report.may_block_current_runner());
        assert!(!enforced.may_block_current_runner());
        assert!(enforced.preserves_legacy_runner);
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn report_freshness_plan_requires_refresh_and_gate_fields() {
        let report = ReportFreshnessReportPlan::report_freshness(
            "report-freshness-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = ReportFreshnessReportPlan::report_freshness(
            "report-freshness-enforced",
            AdapterAcceptanceStage::Enforced,
        );
        let boundary = AdapterReportFreshnessBoundaryPlan::report_freshness(
            "report-freshness-boundary",
            AdapterAcceptanceStage::ReportOnly,
        );

        assert_eq!(report.report_schema_name, "report_freshness_report_v1");
        assert_eq!(
            ReportFreshnessReportPlan::runner_status_field_mappings(),
            vec![
                ("rounds", "report_freshness.rounds"),
                ("ledger_lag", "report_freshness.ledger_lag"),
                ("stale", "report_freshness.stale"),
                ("gate_failures", "report_freshness.gate_failures"),
            ]
        );
        for (runner_status_key, report_field) in
            ReportFreshnessReportPlan::runner_status_field_mappings()
        {
            assert!(
                report.requires_report_field(report_field),
                "missing report field for runner status {runner_status_key}"
            );
            assert!(
                boundary.allows_input(&format!("runner_report_{runner_status_key}")),
                "missing boundary input for runner status {runner_status_key}"
            );
        }
        for required in [
            "report_freshness.rounds",
            "report_freshness.ledger_lag",
            "report_freshness.stale",
            "report_freshness.gate_failures",
            "report_freshness.fresh",
            "report_freshness.ledger_gate_blocked",
            "report_freshness.allow_next_round",
        ] {
            assert!(report.requires_report_field(required), "missing {required}");
        }
        assert!(!report.requires_report_field("ledger_gate.failure_reasons"));
        assert!(!report.requires_report_field("report_freshness.ledger_gate_failure_reasons"));
        assert!(
            !ReportFreshnessReportPlan::runner_status_field_mappings()
                .iter()
                .any(|(_, field)| *field == "report_freshness.failure_reasons")
        );
        assert!(!report.may_block_current_runner());
        assert!(!enforced.may_block_current_runner());
        assert!(report.preserves_legacy_runner);
        assert!(enforced.preserves_legacy_runner);
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn adapter_report_freshness_boundary_plan_forbids_runner_capabilities() {
        let report = AdapterReportFreshnessBoundaryPlan::report_freshness(
            "report-freshness-boundary-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = AdapterReportFreshnessBoundaryPlan::report_freshness(
            "report-freshness-boundary-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert!(enforced.exposes_entrypoint("ReportFreshnessStatus::from_runner_status"));
        assert!(enforced.exposes_entrypoint("ReportFreshnessReport::from_status_and_ledger_gate"));
        assert!(enforced.allows_input("runner_report_rounds"));
        assert!(enforced.allows_input("runner_report_ledger_lag"));
        assert!(enforced.allows_input("runner_report_stale"));
        assert!(enforced.allows_input("runner_report_gate_failures"));
        assert!(enforced.allows_input("ReportFreshnessStatus"));
        assert!(enforced.allows_input("LedgerGateReport"));
        assert!(!enforced.allows_input("JsonlLedgerReader"));
        assert!(!enforced.allows_input("EvolutionLoopRunner"));
        assert!(!enforced.allows_input("DaemonHandle"));
        assert!(!enforced.allows_input("RemoteModelClient"));
        assert!(!enforced.allows_input("LedgerGateFailureReasons"));
        assert!(enforced.produces_output("ReportFreshnessStatus"));
        assert!(enforced.produces_output("ReportFreshnessReport"));
        assert!(enforced.forbids_capability("jsonl_io"));
        assert!(enforced.forbids_capability("file_io"));
        assert!(enforced.forbids_capability("http_sse"));
        assert!(enforced.forbids_capability("process_spawn"));
        assert!(enforced.forbids_capability("validation_command_spawn"));
        assert!(enforced.forbids_capability("daemon_control"));
        assert!(enforced.forbids_capability("runner_state"));
        assert!(enforced.forbids_capability("remote_mac_call"));
        assert!(enforced.forbids_capability("model_call"));
        assert!(enforced.stays_pure_data_boundary());
        assert!(!report.may_block_current_runner());
        assert!(!enforced.may_block_current_runner());
        assert!(enforced.preserves_legacy_runner);
    }

    #[test]
    fn remote_runtime_acceleration_plan_requires_health_metal_and_model_fields() {
        let report = RemoteRuntimeAccelerationReportPlan::remote_runtime_acceleration(
            "remote-runtime-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = RemoteRuntimeAccelerationReportPlan::remote_runtime_acceleration(
            "remote-runtime-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert_eq!(
            report.report_schema_name,
            "remote_runtime_acceleration_report_v1"
        );
        for required in [
            "remote_runtime.total_workers",
            "remote_runtime.healthy_workers",
            "remote_runtime.metal_workers",
            "remote_runtime.quality_model",
            "remote_runtime.all_workers_healthy",
            "remote_runtime.all_workers_metal",
            "remote_runtime.quality_model_present",
            "remote_runtime.acceleration_ready",
        ] {
            assert!(report.requires_report_field(required), "missing {required}");
        }
        assert!(!report.requires_report_field("remote_runtime.remote_mac_host"));
        assert!(!report.requires_report_field("remote_runtime.model_probe_output"));
        assert!(!report.may_block_current_runner());
        assert!(!enforced.may_block_current_runner());
        assert!(report.preserves_legacy_runner);
        assert!(enforced.preserves_legacy_runner);
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn adapter_remote_runtime_boundary_plan_forbids_remote_and_model_calls() {
        let report = AdapterRemoteRuntimeBoundaryPlan::remote_runtime_acceleration(
            "remote-runtime-boundary-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = AdapterRemoteRuntimeBoundaryPlan::remote_runtime_acceleration(
            "remote-runtime-boundary-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert!(
            enforced.exposes_entrypoint("RemoteRuntimeAccelerationStatus::from_runner_pool_status")
        );
        assert!(enforced.exposes_entrypoint("RemoteRuntimeAccelerationReport::from_status"));
        assert!(enforced.allows_input("runner_remote_total_workers"));
        assert!(enforced.allows_input("runner_remote_healthy_workers"));
        assert!(enforced.allows_input("runner_remote_metal_workers"));
        assert!(enforced.allows_input("runner_remote_quality_model"));
        assert!(enforced.allows_input("RemoteRuntimeAccelerationStatus"));
        assert!(!enforced.allows_input("RemoteModelClient"));
        assert!(!enforced.allows_input("RemoteMacSession"));
        assert!(!enforced.allows_input("EvolutionLoopRunner"));
        assert!(enforced.produces_output("RemoteRuntimeAccelerationStatus"));
        assert!(enforced.produces_output("RemoteRuntimeAccelerationReport"));
        assert!(enforced.forbids_capability("jsonl_io"));
        assert!(enforced.forbids_capability("file_io"));
        assert!(enforced.forbids_capability("http_sse"));
        assert!(enforced.forbids_capability("process_spawn"));
        assert!(enforced.forbids_capability("validation_command_spawn"));
        assert!(enforced.forbids_capability("daemon_control"));
        assert!(enforced.forbids_capability("runner_state"));
        assert!(enforced.forbids_capability("remote_mac_call"));
        assert!(enforced.forbids_capability("model_call"));
        assert!(enforced.stays_pure_data_boundary());
        assert!(!report.may_block_current_runner());
        assert!(!enforced.may_block_current_runner());
        assert!(enforced.preserves_legacy_runner);
    }

    #[test]
    fn run_mode_report_refresh_acceptance_plan_combines_reports_without_runner_io() {
        let report = RunModeReportRefreshAcceptancePlan::run_mode_report_refresh(
            "run-mode-report-refresh-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = RunModeReportRefreshAcceptancePlan::run_mode_report_refresh(
            "run-mode-report-refresh-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert_eq!(
            enforced.report_schema_name,
            "run_mode_report_refresh_acceptance_report_v1"
        );
        assert!(enforced.exposes_entrypoint("RunModeReportRefreshAcceptanceReport::from_reports"));
        assert!(enforced.requires_input_report("ReportFreshnessReport"));
        assert!(enforced.requires_input_report("RemoteRuntimeAccelerationReport"));
        assert!(enforced.requires_input_report("EvalReportBundleGateReport"));
        assert!(enforced.produces_report_field("run_mode_report_refresh.report_refresh_allowed"));
        assert!(
            enforced.produces_report_field("run_mode_report_refresh.ledger_gate_allow_next_round")
        );
        assert!(
            enforced
                .produces_report_field("run_mode_report_refresh.remote_runtime_acceleration_ready")
        );
        assert!(enforced.produces_report_field("run_mode_report_refresh.report_bundle_complete"));
        assert!(enforced.produces_report_field("run_mode_report_refresh.allow_next_round"));
        assert!(enforced.produces_report_field("run_mode_report_refresh.failure_reasons"));
        for forbidden in [
            "jsonl_io",
            "file_io",
            "http_sse",
            "process_spawn",
            "validation_command_spawn",
            "daemon_control",
            "runner_state",
            "remote_mac_call",
            "model_call",
        ] {
            assert!(
                enforced.forbids_capability(forbidden),
                "missing forbidden capability {forbidden}"
            );
        }
        assert!(enforced.stays_pure_data_boundary());
        assert!(!report.may_block_current_runner());
        assert!(!enforced.may_block_current_runner());
        assert!(enforced.preserves_legacy_runner);
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn root_adapter_attribution_report_plan_requires_guard_before_enforcement() {
        let shadow = RootAdapterAttributionReportPlan::root_adapter_attribution(
            "root-attribution-shadow",
            AdapterAcceptanceStage::ShadowOnly,
        );
        let report = RootAdapterAttributionReportPlan::root_adapter_attribution(
            "root-attribution-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = RootAdapterAttributionReportPlan::root_adapter_attribution(
            "root-attribution-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert_eq!(
            enforced.report_schema_name,
            "root_adapter_attribution_report_v1"
        );
        assert!(!shadow.require_outage_attribution);
        assert!(report.require_outage_attribution);
        assert!(!report.require_quality_failure_guard);
        assert!(!report.may_block_current_runner());
        assert!(enforced.require_outage_attribution);
        assert!(enforced.require_quality_failure_guard);
        assert!(enforced.may_block_current_runner());
        assert!(enforced.preserves_legacy_runner);
    }

    #[test]
    fn adapter_fixture_contract_plan_requires_pre_wiring_fixture_set() {
        let report = AdapterFixtureContractPlan::root_adapter_fixtures(
            "adapter-fixtures-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = AdapterFixtureContractPlan::root_adapter_fixtures(
            "adapter-fixtures-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert_eq!(
            enforced.report_schema_name,
            "adapter_fixture_contract_report_v1"
        );
        assert!(
            enforced
                .required_fixture_kinds
                .contains(&"chain_not_ready".to_owned())
        );
        assert!(
            enforced
                .required_fixture_kinds
                .contains(&"model_unavailable".to_owned())
        );
        assert!(
            enforced
                .required_fixture_kinds
                .contains(&"model_quality_failure".to_owned())
        );
        assert!(enforced.require_root_fixture);
        assert!(enforced.require_ledger_projection);
        assert!(enforced.require_model_worker_projection);
        assert!(enforced.require_report_bundle_projection);
        assert!(enforced.forbid_operational_quality_confusion);
        assert!(!report.may_block_current_runner());
        assert!(enforced.may_block_current_runner());
        assert!(enforced.preserves_legacy_runner);
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn current_runner_compatibility_plan_aggregates_pre_wiring_gates() {
        let report = CurrentRunnerCompatibilityPlan::before_enforced_wiring(
            "current-runner-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = CurrentRunnerCompatibilityPlan::before_enforced_wiring(
            "current-runner-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert_eq!(
            enforced.report_schema_name,
            "current_runner_compatibility_report_v1"
        );
        assert!(!report.require_legacy_replay);
        assert!(!report.require_report_bundle);
        assert!(!report.require_schema_drift);
        assert!(!report.require_adapter_report_emission);
        assert!(!report.require_adapter_report_field_coverage);
        assert!(!report.require_adapter_future_event_coverage);
        assert!(!report.require_model_pool_development_window);
        assert!(!report.require_apple_silicon_development_effect);
        assert!(!report.require_feedback_self_improve);
        assert!(!report.require_self_evolution_continuity);
        assert!(!report.require_self_evolution_regression);
        assert!(!report.require_readiness_next_round);
        assert!(!report.require_self_evolution_unattended_prerequisites);
        assert!(!report.require_context_rot_trend);
        assert!(!report.require_context_rot_remediation);
        assert!(!report.require_rollback_resume);
        assert!(!report.may_block_current_runner());
        assert!(report.preserves_legacy_runner);
        assert!(enforced.require_legacy_replay);
        assert!(enforced.require_report_bundle);
        assert!(enforced.require_schema_drift);
        assert!(enforced.require_adapter_report_emission);
        assert!(enforced.require_adapter_report_field_coverage);
        assert!(enforced.require_adapter_future_event_coverage);
        assert!(enforced.require_model_pool_development_window);
        assert!(enforced.require_apple_silicon_development_effect);
        assert!(enforced.require_feedback_self_improve);
        assert!(enforced.require_self_evolution_continuity);
        assert!(enforced.require_self_evolution_regression);
        assert!(enforced.require_readiness_next_round);
        assert!(enforced.require_self_evolution_unattended_prerequisites);
        assert!(enforced.require_context_rot_trend);
        assert!(enforced.require_context_rot_remediation);
        assert!(enforced.require_rollback_resume);
        assert!(enforced.require_adapter_fixture);
        assert!(enforced.require_steam_case_matrix);
        assert!(enforced.require_validation_command_coverage);
        assert!(enforced.require_promotion_window);
        assert!(enforced.require_handoff);
        assert!(enforced.require_evolution_loop_tests);
        assert!(enforced.require_workspace_tests);
        assert!(enforced.may_block_current_runner());
        assert!(enforced.preserves_legacy_runner);
        assert_eq!(
            report.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn adapter_current_runner_compatibility_boundary_plan_stays_pure_data() {
        let report =
            AdapterCurrentRunnerCompatibilityBoundaryPlan::current_runner_compatibility_boundary(
                "current-runner-boundary-report",
                AdapterAcceptanceStage::ReportOnly,
            );
        let enforced =
            AdapterCurrentRunnerCompatibilityBoundaryPlan::current_runner_compatibility_boundary(
                "current-runner-boundary-enforced",
                AdapterAcceptanceStage::Enforced,
            );

        assert!(enforced.exposes_entrypoint("CurrentRunnerCompatibilityEvidence::all_passed"));
        assert!(enforced.exposes_entrypoint("CurrentRunnerCompatibilityEvidence::crate_only"));
        assert!(enforced.exposes_entrypoint(
            "CurrentRunnerCompatibilityEvidence::with_adapter_report_emission_report"
        ));
        assert!(enforced.exposes_entrypoint(
            "CurrentRunnerCompatibilityEvidence::with_adapter_report_field_coverage_from_report"
        ));
        assert!(enforced.exposes_entrypoint(
            "CurrentRunnerCompatibilityEvidence::with_adapter_future_event_coverage_report"
        ));
        assert!(enforced.exposes_entrypoint(
            "CurrentRunnerCompatibilityEvidence::with_report_bundle_gate_report"
        ));
        assert!(
            enforced
                .exposes_entrypoint("CurrentRunnerCompatibilityEvidence::with_schema_drift_report")
        );
        assert!(
            enforced.exposes_entrypoint(
                "CurrentRunnerCompatibilityEvidence::with_adapter_fixture_report"
            )
        );
        assert!(enforced.exposes_entrypoint(
            "CurrentRunnerCompatibilityEvidence::with_handoff_report_pass_bits"
        ));
        assert!(enforced.exposes_entrypoint("CurrentRunnerCompatibilityGate::for_stage"));
        assert!(enforced.exposes_entrypoint("CurrentRunnerCompatibilityGate::evaluate"));
        assert!(
            enforced.exposes_entrypoint("CurrentRunnerCompatibilityReport::from_gate_and_evidence")
        );
        assert!(enforced.allows_input("RootAdapterRolloutStage"));
        assert!(enforced.allows_input("CurrentRunnerCompatibilityGate"));
        assert!(enforced.allows_input("CurrentRunnerCompatibilityEvidence"));
        assert!(enforced.allows_input("AdapterReportEmissionReport"));
        assert!(enforced.allows_input("AdapterFutureEventCoverageReport"));
        assert!(enforced.allows_input("EvalReportBundleGateReport"));
        assert!(enforced.allows_input("EvalSchemaDriftReport"));
        assert!(enforced.allows_input("AdapterFixtureReport"));
        assert!(enforced.allows_input("AdapterHandoffReport"));
        assert!(!enforced.allows_input("AdviceContinuationReport"));
        assert!(enforced.allows_input("upstream_gate_pass_bits"));
        assert!(enforced.allows_input("test_result_pass_bits"));
        assert!(!enforced.allows_input("CommandExecutor"));
        assert!(!enforced.allows_input("EvolutionLoopRunner"));
        assert!(!enforced.allows_input("RunnerSwitcher"));
        assert!(!enforced.allows_input("JsonlReader"));
        assert!(enforced.produces_output("CurrentRunnerCompatibilityEvidence"));
        assert!(enforced.produces_output("GateDecision"));
        assert!(enforced.produces_output("CurrentRunnerCompatibilityReport"));
        assert!(enforced.forbids_capability("jsonl_io"));
        assert!(enforced.forbids_capability("file_io"));
        assert!(enforced.forbids_capability("http_sse"));
        assert!(enforced.forbids_capability("process_spawn"));
        assert!(enforced.forbids_capability("cargo_test_execution"));
        assert!(enforced.forbids_capability("workspace_test_execution"));
        assert!(enforced.forbids_capability("evolution_loop_test_execution"));
        assert!(enforced.forbids_capability("model_call"));
        assert!(enforced.forbids_capability("runner_switch_execution"));
        assert!(enforced.forbids_capability("runner_wiring_execution"));
        assert!(enforced.forbids_capability("runner_state_mutation"));
        assert!(enforced.forbids_capability("remote_mac_call"));
        assert!(enforced.stays_pure_data_boundary());
        assert!(!report.may_block_current_runner());
        assert!(!enforced.may_block_current_runner());
        assert!(enforced.preserves_legacy_runner);
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn current_runner_compatibility_schema_document_plan_guards_report_fields() {
        let report =
            CurrentRunnerCompatibilitySchemaDocumentPlan::current_runner_compatibility_schema_document(
                "current-runner-schema-doc-report",
                AdapterAcceptanceStage::ReportOnly,
            );
        let enforced =
            CurrentRunnerCompatibilitySchemaDocumentPlan::current_runner_compatibility_schema_document(
                "current-runner-schema-doc-enforced",
                AdapterAcceptanceStage::Enforced,
            );

        assert_eq!(
            enforced.report_schema_name,
            "current_runner_compatibility_report_v1"
        );
        assert_eq!(
            enforced.schema_document_source,
            "CurrentRunnerCompatibilityReportSchema::current_runner_compatibility_v1"
        );
        assert!(enforced.requires_document_field("current_runner.legacy_replay_passed"));
        assert!(enforced.requires_document_field("current_runner.report_bundle_complete"));
        assert!(enforced.requires_document_field("current_runner.schema_drift_passed"));
        assert!(
            enforced.requires_document_field("current_runner.adapter_report_field_coverage_passed")
        );
        assert!(enforced.requires_document_field("current_runner.rollback_resume_passed"));
        assert!(enforced.requires_document_field("current_runner.steam_case_matrix_passed"));
        assert!(
            enforced.requires_document_field("current_runner.validation_command_coverage_passed")
        );
        assert!(enforced.requires_document_field("current_runner.compatibility_blocked"));
        assert!(enforced.requires_document_field("current_runner.allow_enforced_wiring"));
        assert!(
            enforced
                .required_report_only_fields
                .contains(&"current_runner.handoff_passed".to_owned())
        );
        assert!(
            enforced
                .required_enforced_fields
                .contains(&"current_runner.failure_reasons".to_owned())
        );
        assert!(enforced.requires_boundary_source(
            "CurrentRunnerCompatibilityReportSchema::current_runner_compatibility_v1"
        ));
        assert!(enforced.requires_boundary_source(
            "AdapterCurrentRunnerCompatibilityBoundaryContract::current_runner_compatibility_v1"
        ));
        assert!(enforced.requires_boundary_source(
            "CurrentRunnerCompatibilityEvidence::with_handoff_report_pass_bits"
        ));
        assert!(enforced.forbids_capability("jsonl_io"));
        assert!(enforced.forbids_capability("file_io"));
        assert!(enforced.forbids_capability("http_sse"));
        assert!(enforced.forbids_capability("process_spawn"));
        assert!(enforced.forbids_capability("cargo_test_execution"));
        assert!(enforced.forbids_capability("workspace_test_execution"));
        assert!(enforced.forbids_capability("evolution_loop_test_execution"));
        assert!(enforced.forbids_capability("model_call"));
        assert!(enforced.forbids_capability("runner_switch_execution"));
        assert!(enforced.forbids_capability("runner_wiring_execution"));
        assert!(enforced.forbids_capability("runner_state_mutation"));
        assert!(enforced.forbids_capability("remote_mac_call"));
        assert!(!report.may_block_current_runner());
        assert!(!enforced.may_block_current_runner());
        assert!(enforced.preserves_legacy_runner);
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn adapter_handoff_report_plan_requires_runner_evidence_before_enforcement() {
        let report = AdapterHandoffReportPlan::adapter_handoff(
            "handoff-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = AdapterHandoffReportPlan::adapter_handoff(
            "handoff-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert_eq!(enforced.report_schema_name, "adapter_handoff_report_v1");
        assert!(!report.require_runner_workspace_replay_before_enforcement);
        assert!(!report.require_report_only_observation_before_enforcement);
        assert!(!report.require_report_bundle_complete_before_enforcement);
        assert!(!report.require_schema_drift_before_enforcement);
        assert!(!report.require_adapter_report_emission_before_enforcement);
        assert!(!report.require_adapter_report_field_coverage_before_enforcement);
        assert!(!report.require_adapter_future_event_coverage_before_enforcement);
        assert!(!report.require_model_pool_development_window_before_enforcement);
        assert!(!report.require_apple_silicon_development_effect_before_enforcement);
        assert!(!report.require_feedback_self_improve_before_enforcement);
        assert!(!report.require_self_evolution_continuity_before_enforcement);
        assert!(!report.require_self_evolution_regression_before_enforcement);
        assert!(!report.require_readiness_next_round_before_enforcement);
        assert!(!report.require_self_evolution_unattended_prerequisites_before_enforcement);
        assert!(!report.require_context_rot_trend_before_enforcement);
        assert!(!report.require_context_rot_remediation_before_enforcement);
        assert!(!report.require_rollback_resume_before_enforcement);
        assert!(!report.require_steam_case_matrix_before_enforcement);
        assert!(!report.require_validation_command_coverage_before_enforcement);
        assert!(!report.require_promotion_window_before_enforcement);
        assert!(!report.may_block_current_runner());
        assert!(report.preserves_legacy_runner);
        assert!(enforced.require_runner_workspace_replay_before_enforcement);
        assert!(enforced.require_report_only_observation_before_enforcement);
        assert!(enforced.require_report_bundle_complete_before_enforcement);
        assert!(enforced.require_schema_drift_before_enforcement);
        assert!(enforced.require_adapter_report_emission_before_enforcement);
        assert!(enforced.require_adapter_report_field_coverage_before_enforcement);
        assert!(enforced.require_adapter_future_event_coverage_before_enforcement);
        assert!(enforced.require_model_pool_development_window_before_enforcement);
        assert!(enforced.require_apple_silicon_development_effect_before_enforcement);
        assert!(enforced.require_feedback_self_improve_before_enforcement);
        assert!(enforced.require_self_evolution_continuity_before_enforcement);
        assert!(enforced.require_self_evolution_regression_before_enforcement);
        assert!(enforced.require_readiness_next_round_before_enforcement);
        assert!(enforced.require_self_evolution_unattended_prerequisites_before_enforcement);
        assert!(enforced.require_context_rot_trend_before_enforcement);
        assert!(enforced.require_context_rot_remediation_before_enforcement);
        assert!(enforced.require_rollback_resume_before_enforcement);
        assert!(enforced.require_steam_case_matrix_before_enforcement);
        assert!(enforced.require_validation_command_coverage_before_enforcement);
        assert!(enforced.require_promotion_window_before_enforcement);
        assert!(enforced.may_block_current_runner());
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn adapter_handoff_boundary_plan_does_not_execute_runner_handoff() {
        let report = AdapterHandoffBoundaryPlan::handoff_boundary(
            "handoff-boundary-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = AdapterHandoffBoundaryPlan::handoff_boundary(
            "handoff-boundary-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert!(enforced.exposes_entrypoint("AdapterHandoffChecklist::before_runner_wiring"));
        assert!(enforced.exposes_entrypoint("AdapterHandoffChecklist::evaluate"));
        assert!(enforced.exposes_entrypoint("AdapterHandoffEvidence::crate_only_passed"));
        assert!(enforced.exposes_entrypoint("AdapterHandoffEvidence::full_handoff_passed"));
        assert!(
            enforced
                .exposes_entrypoint("AdapterHandoffEvidence::with_adapter_report_emission_report")
        );
        assert!(enforced.exposes_entrypoint(
            "AdapterHandoffEvidence::with_adapter_report_field_coverage_from_report"
        ));
        assert!(enforced.exposes_entrypoint(
            "AdapterHandoffEvidence::with_adapter_future_event_coverage_report"
        ));
        assert!(
            enforced.exposes_entrypoint("AdapterHandoffEvidence::with_report_bundle_gate_report")
        );
        assert!(enforced.exposes_entrypoint("AdapterHandoffEvidence::with_schema_drift_report"));
        assert!(
            enforced.exposes_entrypoint("AdapterHandoffEvidence::with_operational_gate_reports")
        );
        assert!(
            enforced
                .exposes_entrypoint("AdapterHandoffEvidence::with_unattended_prerequisite_report")
        );
        assert!(enforced.exposes_entrypoint("AdapterHandoffReport::from_checklist_and_evidence"));
        assert!(enforced.allows_input("RootAdapterRolloutStage"));
        assert!(enforced.allows_input("AdapterHandoffChecklist"));
        assert!(enforced.allows_input("AdapterHandoffEvidence"));
        assert!(enforced.allows_input("AdapterTestGate"));
        assert!(enforced.allows_input("AdapterReportEmissionReport"));
        assert!(enforced.allows_input("AdapterFutureEventCoverageReport"));
        assert!(enforced.allows_input("EvalReportBundleGateReport"));
        assert!(enforced.allows_input("EvalSchemaDriftReport"));
        assert!(enforced.allows_input("ContextRotTrendReport"));
        assert!(enforced.allows_input("ContextRotRemediationReport"));
        assert!(enforced.allows_input("RollbackResumeReport"));
        assert!(enforced.allows_input("SteamCaseCoverageReport"));
        assert!(enforced.allows_input("ValidationCommandCoverageReport"));
        assert!(enforced.allows_input("SelfEvolutionUnattendedPrerequisiteReport"));
        assert!(!enforced.allows_input("AdviceContinuationReport"));
        assert!(enforced.allows_input("test_gate_command_text"));
        assert!(enforced.allows_input("upstream_report_pass_bits"));
        assert!(!enforced.allows_input("CommandExecutor"));
        assert!(!enforced.allows_input("EvolutionLoopRunner"));
        assert!(!enforced.allows_input("RunnerSwitcher"));
        assert!(enforced.produces_output("AdapterHandoffChecklist"));
        assert!(enforced.produces_output("GateDecision"));
        assert!(enforced.produces_output("AdapterHandoffReport"));
        assert!(enforced.forbids_capability("jsonl_io"));
        assert!(enforced.forbids_capability("file_io"));
        assert!(enforced.forbids_capability("http_sse"));
        assert!(enforced.forbids_capability("process_spawn"));
        assert!(enforced.forbids_capability("cargo_test_execution"));
        assert!(enforced.forbids_capability("workspace_test_execution"));
        assert!(enforced.forbids_capability("evolution_loop_test_execution"));
        assert!(enforced.forbids_capability("model_call"));
        assert!(enforced.forbids_capability("runner_handoff_execution"));
        assert!(enforced.forbids_capability("runner_state_mutation"));
        assert!(enforced.forbids_capability("remote_mac_call"));
        assert!(enforced.stays_pure_data_boundary());
        assert!(!report.may_block_current_runner());
        assert!(!enforced.may_block_current_runner());
        assert!(enforced.preserves_legacy_runner);
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn eval_schema_manifest_plan_names_handoff_schemas() {
        let plan = EvalSchemaManifestPlan::evolution_loop_handoff("schema-manifest");

        assert!(
            plan.required_schema_names
                .contains(&"model_worker_v1".to_owned())
        );
        assert!(
            plan.required_schema_names
                .contains(&"model_worker_gate_report_v1".to_owned())
        );
        assert!(
            plan.required_schema_names
                .contains(&"worker_root_failure_consistency_report_v1".to_owned())
        );
        assert!(
            plan.required_schema_names
                .contains(&"model_pool_budget_fairness_report_v1".to_owned())
        );
        assert!(
            plan.required_schema_names
                .contains(&"model_pool_development_attribution_report_v1".to_owned())
        );
        assert!(
            plan.required_schema_names
                .contains(&"model_pool_development_window_report_v1".to_owned())
        );
        assert!(
            plan.required_schema_names
                .contains(&"apple_silicon_baseline_comparison_report_v1".to_owned())
        );
        assert!(
            plan.required_schema_names
                .contains(&"adapter_report_emission_report_v1".to_owned())
        );
        assert!(
            plan.required_schema_names
                .contains(&"adapter_future_event_coverage_report_v1".to_owned())
        );
        assert!(
            plan.required_schema_names
                .contains(&"ledger_gate_report_v1".to_owned())
        );
        assert!(
            plan.required_schema_names
                .contains(&"report_freshness_report_v1".to_owned())
        );
        assert!(
            plan.required_schema_names
                .contains(&"remote_runtime_acceleration_report_v1".to_owned())
        );
        assert!(
            plan.required_schema_names
                .contains(&"run_mode_report_refresh_acceptance_report_v1".to_owned())
        );
        assert!(
            plan.required_schema_names
                .contains(&"context_rot_report_v1".to_owned())
        );
        assert!(
            plan.required_schema_names
                .contains(&"context_rot_trend_report_v1".to_owned())
        );
        assert!(
            plan.required_schema_names
                .contains(&"context_rot_remediation_report_v1".to_owned())
        );
        assert!(
            plan.required_schema_names
                .contains(&"experiment_rollout_report_v1".to_owned())
        );
        assert!(
            plan.required_schema_names
                .contains(&"experiment_kill_switch_report_v1".to_owned())
        );
        assert!(
            plan.required_schema_names
                .contains(&"experiment_expansion_safety_report_v1".to_owned())
        );
        assert!(
            plan.required_schema_names
                .contains(&"experiment_switch_matrix_report_v1".to_owned())
        );
        assert!(
            plan.required_schema_names
                .contains(&"root_adapter_attribution_report_v1".to_owned())
        );
        assert!(
            plan.required_schema_names
                .contains(&"adapter_fixture_contract_report_v1".to_owned())
        );
        assert!(
            plan.required_schema_names
                .contains(&"current_runner_compatibility_report_v1".to_owned())
        );
        assert!(
            plan.required_schema_names
                .contains(&"legacy_ledger_replay_report_v1".to_owned())
        );
        assert!(
            plan.required_schema_names
                .contains(&"feedback_self_improve_report_v1".to_owned())
        );
        assert!(
            !plan
                .required_schema_names
                .contains(&"advice_continuation_report_v1".to_owned())
        );
        assert!(
            plan.required_schema_names
                .contains(&"self_evolution_continuity_report_v1".to_owned())
        );
        assert!(
            plan.required_schema_names
                .contains(&"self_evolution_regression_report_v1".to_owned())
        );
        assert!(
            plan.required_schema_names
                .contains(&"readiness_next_round_v1".to_owned())
        );
        assert!(
            plan.required_schema_names
                .contains(&"steam_round_report_v1".to_owned())
        );
        assert!(
            plan.required_schema_names
                .contains(&"steam_case_matrix_report_v1".to_owned())
        );
        assert!(
            plan.required_schema_names
                .contains(&"validation_command_coverage_report_v1".to_owned())
        );
        assert!(
            plan.required_schema_names
                .contains(&"rollback_report_v1".to_owned())
        );
        assert!(
            plan.required_schema_names
                .contains(&"adapter_closure_report_v1".to_owned())
        );
        assert!(
            plan.required_schema_names
                .contains(&"rollback_drill_matrix_report_v1".to_owned())
        );
        assert!(
            plan.required_schema_names
                .contains(&"adapter_handoff_report_v1".to_owned())
        );
        assert!(
            plan.required_schema_names
                .contains(&"report_bundle_gate_report_v1".to_owned())
        );
        assert!(
            plan.required_schema_names
                .contains(&"schema_drift_report_v1".to_owned())
        );
        assert!(
            plan.required_schema_names
                .contains(&"adapter_promotion_window_report_v1".to_owned())
        );
        assert!(
            plan.required_schema_names
                .contains(&"rollback_resume_report_v1".to_owned())
        );
        assert_eq!(
            plan.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn eval_report_bundle_plan_tracks_stage_schema_sets() {
        let shadow =
            EvalReportBundlePlan::for_stage("bundle-shadow", AdapterAcceptanceStage::ShadowOnly);
        let report =
            EvalReportBundlePlan::for_stage("bundle-report", AdapterAcceptanceStage::ReportOnly);
        let enforced =
            EvalReportBundlePlan::for_stage("bundle-enforced", AdapterAcceptanceStage::Enforced);

        assert!(shadow.report_schema_names.is_empty());
        assert!(
            report
                .report_schema_names
                .contains(&"model_worker_v1".to_owned())
        );
        assert!(
            report
                .report_schema_names
                .contains(&"worker_root_failure_consistency_report_v1".to_owned())
        );
        assert!(
            report
                .report_schema_names
                .contains(&"apple_silicon_baseline_comparison_report_v1".to_owned())
        );
        assert!(
            report
                .report_schema_names
                .contains(&"experiment_switch_matrix_report_v1".to_owned())
        );
        assert!(
            report
                .report_schema_names
                .contains(&"rollback_drill_matrix_report_v1".to_owned())
        );
        assert!(
            report
                .report_schema_names
                .contains(&"adapter_promotion_window_report_v1".to_owned())
        );
        assert!(
            !report
                .report_schema_names
                .contains(&"experiment_rollout_report_v1".to_owned())
        );
        assert!(
            !report
                .report_schema_names
                .contains(&"remote_runtime_acceleration_report_v1".to_owned())
        );
        assert!(
            !report
                .report_schema_names
                .contains(&"run_mode_report_refresh_acceptance_report_v1".to_owned())
        );
        assert!(
            enforced
                .report_schema_names
                .contains(&"model_worker_gate_report_v1".to_owned())
        );
        assert!(
            enforced
                .report_schema_names
                .contains(&"worker_root_failure_consistency_report_v1".to_owned())
        );
        assert!(
            enforced
                .report_schema_names
                .contains(&"model_pool_budget_fairness_report_v1".to_owned())
        );
        assert!(
            enforced
                .report_schema_names
                .contains(&"model_pool_development_attribution_report_v1".to_owned())
        );
        assert!(
            enforced
                .report_schema_names
                .contains(&"model_pool_development_window_report_v1".to_owned())
        );
        assert!(
            enforced
                .report_schema_names
                .contains(&"apple_silicon_baseline_comparison_report_v1".to_owned())
        );
        assert!(
            enforced
                .report_schema_names
                .contains(&"ledger_gate_report_v1".to_owned())
        );
        assert!(
            enforced
                .report_schema_names
                .contains(&"report_freshness_report_v1".to_owned())
        );
        assert!(
            enforced
                .report_schema_names
                .contains(&"remote_runtime_acceleration_report_v1".to_owned())
        );
        assert!(
            enforced
                .report_schema_names
                .contains(&"run_mode_report_refresh_acceptance_report_v1".to_owned())
        );
        assert!(
            enforced
                .report_schema_names
                .contains(&"context_rot_trend_report_v1".to_owned())
        );
        assert!(
            enforced
                .report_schema_names
                .contains(&"context_rot_remediation_report_v1".to_owned())
        );
        assert!(
            enforced
                .report_schema_names
                .contains(&"experiment_rollout_report_v1".to_owned())
        );
        assert!(
            enforced
                .report_schema_names
                .contains(&"experiment_kill_switch_report_v1".to_owned())
        );
        assert!(
            enforced
                .report_schema_names
                .contains(&"experiment_expansion_safety_report_v1".to_owned())
        );
        assert!(
            enforced
                .report_schema_names
                .contains(&"experiment_switch_matrix_report_v1".to_owned())
        );
        assert!(
            enforced
                .report_schema_names
                .contains(&"root_adapter_attribution_report_v1".to_owned())
        );
        assert!(
            enforced
                .report_schema_names
                .contains(&"adapter_fixture_contract_report_v1".to_owned())
        );
        assert!(
            enforced
                .report_schema_names
                .contains(&"current_runner_compatibility_report_v1".to_owned())
        );
        assert!(
            enforced
                .report_schema_names
                .contains(&"legacy_ledger_replay_report_v1".to_owned())
        );
        assert!(
            !enforced
                .report_schema_names
                .contains(&"advice_continuation_report_v1".to_owned())
        );
        assert!(
            enforced
                .report_schema_names
                .contains(&"feedback_self_improve_report_v1".to_owned())
        );
        assert!(
            enforced
                .report_schema_names
                .contains(&"self_evolution_continuity_report_v1".to_owned())
        );
        assert!(
            enforced
                .report_schema_names
                .contains(&"self_evolution_regression_report_v1".to_owned())
        );
        assert!(
            enforced
                .report_schema_names
                .contains(&"steam_case_matrix_report_v1".to_owned())
        );
        assert!(
            enforced
                .report_schema_names
                .contains(&"validation_command_coverage_report_v1".to_owned())
        );
        assert!(
            enforced
                .report_schema_names
                .contains(&"rollback_report_v1".to_owned())
        );
        assert!(
            enforced
                .report_schema_names
                .contains(&"adapter_closure_report_v1".to_owned())
        );
        assert!(
            enforced
                .report_schema_names
                .contains(&"rollback_drill_matrix_report_v1".to_owned())
        );
        assert!(
            enforced
                .report_schema_names
                .contains(&"adapter_handoff_report_v1".to_owned())
        );
        assert!(
            enforced
                .report_schema_names
                .contains(&"report_bundle_gate_report_v1".to_owned())
        );
        assert!(
            enforced
                .report_schema_names
                .contains(&"schema_drift_report_v1".to_owned())
        );
        assert!(
            enforced
                .report_schema_names
                .contains(&"adapter_promotion_window_report_v1".to_owned())
        );
        assert!(
            enforced
                .report_schema_names
                .contains(&"rollback_resume_report_v1".to_owned())
        );
    }

    #[test]
    fn eval_report_bundle_gate_report_plan_requires_complete_bundle_before_enforcement() {
        let report = EvalReportBundleGateReportPlan::report_bundle_gate(
            "bundle-gate-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = EvalReportBundleGateReportPlan::report_bundle_gate(
            "bundle-gate-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert_eq!(enforced.report_schema_name, "report_bundle_gate_report_v1");
        assert!(!report.require_complete_bundle_before_enforcement);
        assert!(!report.require_adapter_report_field_coverage_before_enforcement);
        assert!(!report.may_block_current_runner());
        assert!(report.preserves_legacy_runner);
        assert!(enforced.require_complete_bundle_before_enforcement);
        assert!(enforced.require_adapter_report_field_coverage_before_enforcement);
        assert!(enforced.may_block_current_runner());
        assert_eq!(
            EvalReportBundleGateReportPlan::downstream_projection_field_mappings(),
            vec![(
                "report_bundle.complete",
                "run_mode_report_refresh.report_bundle_complete"
            )]
        );
        assert!(
            !EvalReportBundleGateReportPlan::downstream_projection_field_mappings()
                .iter()
                .any(|(bundle_field, downstream_field)| {
                    *bundle_field == "report_bundle.failure_reasons"
                        || *downstream_field
                            == "run_mode_report_refresh.report_bundle_failure_reasons"
                })
        );
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn adapter_report_bundle_gate_boundary_plan_does_not_scan_report_files() {
        let report = AdapterReportBundleGateBoundaryPlan::report_bundle_gate_boundary(
            "report-bundle-boundary-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = AdapterReportBundleGateBoundaryPlan::report_bundle_gate_boundary(
            "report-bundle-boundary-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert!(enforced.exposes_entrypoint("EvalReportBundleManifest::for_stage"));
        assert!(enforced.exposes_entrypoint("EvalReportBundleEvidence::from_schema_names"));
        assert!(enforced.exposes_entrypoint(
            "EvalReportBundleEvidence::with_adapter_report_field_coverage_from_report"
        ));
        assert!(
            enforced.exposes_entrypoint(
                "EvalReportBundleEvidence::from_adapter_report_emission_report"
            )
        );
        assert!(enforced.exposes_entrypoint("EvalReportBundleManifest::evaluate_bundle"));
        assert!(
            enforced.exposes_entrypoint("EvalReportBundleGateReport::from_manifest_and_evidence")
        );
        assert!(enforced.exposes_entrypoint(
            "EvalReportBundleGateReport::from_manifest_and_adapter_report_emission_report"
        ));
        assert!(enforced.allows_input("RootAdapterRolloutStage"));
        assert!(enforced.allows_input("observed_schema_names"));
        assert!(enforced.allows_input("EvalReportBundleManifest"));
        assert!(enforced.allows_input("EvalReportBundleEvidence"));
        assert!(enforced.allows_input("adapter_report_field_coverage_passed"));
        assert!(enforced.allows_input("AdapterReportEmissionReport::field_coverage_passed"));
        assert!(enforced.allows_input("AdapterReportEmissionReport"));
        assert!(!enforced.allows_input("ReportDirectoryScanner"));
        assert!(!enforced.allows_input("JsonlReader"));
        assert!(!enforced.allows_input("EvolutionLoopRunner"));
        assert!(enforced.produces_output("EvalReportBundleManifest"));
        assert!(enforced.produces_output("EvalReportBundleEvidence"));
        assert!(enforced.produces_output("GateDecision"));
        assert!(enforced.produces_output("EvalReportBundleGateReport"));
        assert!(enforced.forbids_capability("jsonl_io"));
        assert!(enforced.forbids_capability("file_io"));
        assert!(enforced.forbids_capability("report_directory_scan"));
        assert!(enforced.forbids_capability("http_sse"));
        assert!(enforced.forbids_capability("process_spawn"));
        assert!(enforced.forbids_capability("validation_command_spawn"));
        assert!(enforced.forbids_capability("model_call"));
        assert!(enforced.forbids_capability("runner_state"));
        assert!(enforced.stays_pure_data_boundary());
        assert!(!report.may_block_current_runner());
        assert!(!enforced.may_block_current_runner());
        assert!(enforced.preserves_legacy_runner);
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn eval_schema_drift_report_plan_requires_matching_contracts_before_wiring() {
        let report = EvalSchemaDriftReportPlan::schema_drift(
            "schema-drift-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = EvalSchemaDriftReportPlan::schema_drift(
            "schema-drift-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert_eq!(enforced.report_schema_name, "schema_drift_report_v1");
        assert!(
            enforced
                .compared_contract_sources
                .contains(&"EvalSchemaManifest::evolution_loop_handoff_v1".to_owned())
        );
        assert!(
            enforced
                .compared_contract_sources
                .contains(&"EvalReportBundleManifest::for_stage".to_owned())
        );
        assert!(
            enforced
                .compared_contract_sources
                .contains(&"AdapterHandoffChecklist::before_runner_wiring".to_owned())
        );
        assert!(
            enforced
                .compared_contract_sources
                .contains(&"AdapterReportEmissionPlan::required_report_fields".to_owned())
        );
        assert!(
            enforced
                .compared_contract_sources
                .contains(&"AdapterClosurePureDataContract::schema_document".to_owned())
        );
        assert!(
            enforced
                .requires_report_field_contract_example("apple_silicon_effect.feedback_applied")
        );
        assert!(
            enforced.requires_report_field_contract_example(
                "model_pool_attribution.validation_checked"
            )
        );
        assert!(
            enforced
                .requires_report_field_contract_example("context_rot_trend.latest_noisy_records")
        );
        assert!(enforced.requires_report_field_contract_example(
            "context_rot_remediation.allow_experiment_rollout"
        ));
        assert!(enforced.requires_report_field_contract_example(
            "validation_command.strict_coverage_requested"
        ));
        assert!(enforced.requires_report_field_contract_example(
            "validation_command.coverage_tooling_evidence"
        ));
        assert!(
            enforced.requires_report_field_contract_example(
                "validation_command.coverage_report_evidence"
            )
        );
        assert!(enforced.requires_report_field_contract_example(
            "validation_command.coverage_tooling_or_report_evidence_present"
        ));
        assert!(
            enforced.requires_report_field_contract_example("adapter_closure.allow_next_round")
        );
        assert!(!report.require_matching_checksums_before_wiring);
        assert!(!report.require_matching_report_field_contract_before_wiring);
        assert!(!report.may_block_current_runner());
        assert!(report.preserves_legacy_runner);
        assert!(enforced.require_matching_checksums_before_wiring);
        assert!(enforced.require_matching_report_field_contract_before_wiring);
        assert!(enforced.may_block_current_runner());
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn adapter_schema_drift_boundary_plan_does_not_read_schema_files() {
        let report = AdapterSchemaDriftBoundaryPlan::schema_drift_boundary(
            "schema-drift-boundary-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = AdapterSchemaDriftBoundaryPlan::schema_drift_boundary(
            "schema-drift-boundary-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert!(enforced.exposes_entrypoint("EvalSchemaDriftEvidence::from_current_contracts"));
        assert!(
            enforced.exposes_entrypoint(
                "EvalSchemaDriftEvidence::from_adapter_closure_schema_document"
            )
        );
        assert!(enforced.exposes_entrypoint(
            "EvalSchemaDriftEvidence::with_adapter_report_field_coverage_from_report"
        ));
        assert!(enforced.exposes_entrypoint("EvalSchemaFingerprint::from_schema_names"));
        assert!(enforced.exposes_entrypoint("EvalSchemaDriftGate::evaluate"));
        assert!(enforced.exposes_entrypoint("EvalSchemaDriftReport::from_gate_and_evidence"));
        assert!(enforced.allows_input("RootAdapterRolloutStage"));
        assert!(enforced.allows_input("EvalSchemaManifest"));
        assert!(enforced.allows_input("EvalReportBundleManifest"));
        assert!(enforced.allows_input("AdapterHandoffChecklist"));
        assert!(enforced.allows_input("AdapterReportEmissionPlan::required_report_fields"));
        assert!(enforced.allows_input("AdapterClosureSchemaDocument"));
        assert!(enforced.allows_input("AdapterReportEmissionReport"));
        assert!(enforced.allows_input("EvalSchemaDriftEvidence"));
        assert!(enforced.allows_input("EvalSchemaDriftGate"));
        assert!(!enforced.allows_input("SchemaFileReader"));
        assert!(!enforced.allows_input("ReportDirectoryScanner"));
        assert!(!enforced.allows_input("EvolutionLoopRunner"));
        assert!(enforced.produces_output("EvalSchemaFingerprint"));
        assert!(enforced.produces_output("EvalSchemaDriftEvidence"));
        assert!(enforced.produces_output("GateDecision"));
        assert!(enforced.produces_output("EvalSchemaDriftReport"));
        assert!(enforced.forbids_capability("jsonl_io"));
        assert!(enforced.forbids_capability("file_io"));
        assert!(enforced.forbids_capability("schema_file_read"));
        assert!(enforced.forbids_capability("report_directory_scan"));
        assert!(enforced.forbids_capability("http_sse"));
        assert!(enforced.forbids_capability("process_spawn"));
        assert!(enforced.forbids_capability("validation_command_spawn"));
        assert!(enforced.forbids_capability("model_call"));
        assert!(enforced.forbids_capability("runner_state"));
        assert!(enforced.stays_pure_data_boundary());
        assert!(!report.may_block_current_runner());
        assert!(!enforced.may_block_current_runner());
        assert!(enforced.preserves_legacy_runner);
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn adapter_promotion_window_report_plan_requires_stable_window_before_enforcement() {
        let report = AdapterPromotionWindowReportPlan::adapter_promotion_window(
            "promotion-window-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = AdapterPromotionWindowReportPlan::adapter_promotion_window(
            "promotion-window-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert_eq!(
            enforced.report_schema_name,
            "adapter_promotion_window_report_v1"
        );
        assert_eq!(report.min_report_only_runs, 3);
        assert_eq!(report.min_complete_bundle_runs, 3);
        assert_eq!(report.min_adapter_report_emission_passed_runs, 3);
        assert_eq!(report.min_adapter_report_field_coverage_passed_runs, 3);
        assert_eq!(report.min_adapter_future_event_coverage_passed_runs, 3);
        assert_eq!(report.min_apple_silicon_development_effect_passed_runs, 3);
        assert_eq!(report.min_context_rot_trend_passed_runs, 3);
        assert_eq!(report.min_context_rot_remediation_passed_runs, 3);
        assert_eq!(report.min_steam_case_matrix_passed_runs, 3);
        assert_eq!(report.min_validation_command_coverage_passed_runs, 3);
        assert_eq!(enforced.min_report_only_runs, 3);
        assert_eq!(enforced.min_complete_bundle_runs, 3);
        assert_eq!(enforced.min_adapter_report_emission_passed_runs, 3);
        assert_eq!(enforced.min_adapter_report_field_coverage_passed_runs, 3);
        assert_eq!(enforced.min_adapter_future_event_coverage_passed_runs, 3);
        assert_eq!(enforced.min_apple_silicon_development_effect_passed_runs, 3);
        assert_eq!(
            enforced.min_apple_silicon_baseline_comparison_passed_runs,
            3
        );
        assert_eq!(enforced.min_experiment_switch_matrix_passed_runs, 3);
        assert_eq!(enforced.min_readiness_passed_runs, 3);
        assert_eq!(enforced.min_context_rot_trend_passed_runs, 3);
        assert_eq!(enforced.min_context_rot_remediation_passed_runs, 3);
        assert_eq!(enforced.min_rollback_resume_passed_runs, 3);
        assert_eq!(enforced.min_steam_case_matrix_passed_runs, 3);
        assert_eq!(enforced.min_validation_command_coverage_passed_runs, 3);
        assert!(!report.require_no_quality_failures_before_enforcement);
        assert!(!report.require_no_worker_quality_failures_before_enforcement);
        assert!(!report.require_no_worker_claim_blockers_before_enforcement);
        assert!(!report.require_chain_and_model_available_before_enforcement);
        assert!(!report.require_worker_operational_readiness_clear_before_enforcement);
        assert!(!report.require_adapter_report_emission_stable_before_enforcement);
        assert!(!report.require_adapter_report_field_coverage_stable_before_enforcement);
        assert!(!report.require_adapter_future_event_coverage_stable_before_enforcement);
        assert!(!report.require_apple_silicon_effect_stable_before_enforcement);
        assert!(!report.require_apple_silicon_baseline_comparison_stable_before_enforcement);
        assert!(!report.require_experiment_switch_matrix_stable_before_enforcement);
        assert!(!report.require_readiness_stable_before_enforcement);
        assert!(!report.require_context_rot_trend_stable_before_enforcement);
        assert!(!report.require_context_rot_remediation_stable_before_enforcement);
        assert!(!report.require_rollback_resume_stable_before_enforcement);
        assert!(!report.require_steam_case_matrix_stable_before_enforcement);
        assert!(!report.require_validation_command_coverage_stable_before_enforcement);
        assert!(!report.may_block_current_runner());
        assert!(report.preserves_legacy_runner);
        assert!(enforced.require_no_quality_failures_before_enforcement);
        assert!(enforced.require_no_worker_quality_failures_before_enforcement);
        assert!(enforced.require_no_worker_claim_blockers_before_enforcement);
        assert!(enforced.require_chain_and_model_available_before_enforcement);
        assert!(enforced.require_worker_operational_readiness_clear_before_enforcement);
        assert!(enforced.require_adapter_report_emission_stable_before_enforcement);
        assert!(enforced.require_adapter_report_field_coverage_stable_before_enforcement);
        assert!(enforced.require_adapter_future_event_coverage_stable_before_enforcement);
        assert!(enforced.require_apple_silicon_effect_stable_before_enforcement);
        assert!(enforced.require_apple_silicon_baseline_comparison_stable_before_enforcement);
        assert!(enforced.require_experiment_switch_matrix_stable_before_enforcement);
        assert!(enforced.require_readiness_stable_before_enforcement);
        assert!(enforced.require_context_rot_trend_stable_before_enforcement);
        assert!(enforced.require_context_rot_remediation_stable_before_enforcement);
        assert!(enforced.require_rollback_resume_stable_before_enforcement);
        assert!(enforced.require_steam_case_matrix_stable_before_enforcement);
        assert!(enforced.require_validation_command_coverage_stable_before_enforcement);
        assert!(enforced.may_block_current_runner());
        assert!(enforced.preserves_legacy_runner);
        assert_eq!(
            report.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }

    #[test]
    fn advice_continuation_report_stays_additive_and_out_of_handoff_plans() {
        let replay = LegacyLedgerReplayPlan::evolution_loop_jsonl("legacy-ledger-replay");
        let manifest = EvalSchemaManifestPlan::evolution_loop_handoff("schema-manifest");
        let report_bundle =
            EvalReportBundlePlan::for_stage("bundle-report", AdapterAcceptanceStage::ReportOnly);
        let enforced_bundle =
            EvalReportBundlePlan::for_stage("bundle-enforced", AdapterAcceptanceStage::Enforced);
        let handoff = AdapterHandoffBoundaryPlan::handoff_boundary(
            "handoff-boundary",
            AdapterAcceptanceStage::Enforced,
        );
        let current_runner =
            AdapterCurrentRunnerCompatibilityBoundaryPlan::current_runner_compatibility_boundary(
                "current-runner-boundary",
                AdapterAcceptanceStage::Enforced,
            );

        assert!(
            replay
                .optional_additive_reports
                .contains(&"advice_continuation_report_v1".to_owned())
        );
        assert!(
            !manifest
                .required_schema_names
                .contains(&"advice_continuation_report_v1".to_owned())
        );
        assert!(
            !report_bundle
                .report_schema_names
                .contains(&"advice_continuation_report_v1".to_owned())
        );
        assert!(
            !enforced_bundle
                .report_schema_names
                .contains(&"advice_continuation_report_v1".to_owned())
        );
        assert!(!handoff.allows_input("AdviceContinuationReport"));
        assert!(!current_runner.allows_input("AdviceContinuationReport"));
    }

    #[test]
    fn context_rot_report_stays_report_only_but_in_handoff_plans() {
        let manifest = EvalSchemaManifestPlan::evolution_loop_handoff("schema-manifest");
        let report_bundle =
            EvalReportBundlePlan::for_stage("bundle-report", AdapterAcceptanceStage::ReportOnly);
        let enforced_bundle =
            EvalReportBundlePlan::for_stage("bundle-enforced", AdapterAcceptanceStage::Enforced);
        let handoff = AdapterHandoffBoundaryPlan::handoff_boundary(
            "handoff-boundary",
            AdapterAcceptanceStage::Enforced,
        );

        assert!(
            manifest
                .required_schema_names
                .contains(&"context_rot_report_v1".to_owned())
        );
        assert!(
            report_bundle
                .report_schema_names
                .contains(&"context_rot_report_v1".to_owned())
        );
        assert!(
            enforced_bundle
                .report_schema_names
                .contains(&"context_rot_report_v1".to_owned())
        );
        assert!(handoff.allows_input("ContextRotTrendReport"));
        assert!(handoff.allows_input("ContextRotRemediationReport"));
    }

    #[test]
    fn adapter_promotion_window_boundary_plan_does_not_promote_runner() {
        let report = AdapterPromotionWindowBoundaryPlan::promotion_window_boundary(
            "promotion-window-boundary-report",
            AdapterAcceptanceStage::ReportOnly,
        );
        let enforced = AdapterPromotionWindowBoundaryPlan::promotion_window_boundary(
            "promotion-window-boundary-enforced",
            AdapterAcceptanceStage::Enforced,
        );

        assert!(
            enforced
                .exposes_entrypoint("AdapterPromotionWindowEvidence::stable_report_only_window")
        );
        assert!(enforced.exposes_entrypoint(
            "AdapterPromotionWindowEvidence::with_adapter_report_field_coverage_passed_runs_from_reports"
        ));
        assert!(enforced.exposes_entrypoint("AdapterPromotionWindowGate::evaluate"));
        assert!(
            enforced.exposes_entrypoint("AdapterPromotionWindowReport::from_gate_and_evidence")
        );
        assert!(enforced.allows_input("AdapterPromotionWindowEvidence"));
        assert!(enforced.allows_input("AdapterPromotionWindowGate"));
        assert!(enforced.allows_input("AdapterReportEmissionReport::field_coverage_passed"));
        assert!(enforced.allows_input("report_only_observation_counts"));
        assert!(enforced.allows_input("gate_passed_run_counts"));
        assert!(enforced.allows_input("root_adapter_failure_counts"));
        assert!(!enforced.allows_input("ReportDirectoryScanner"));
        assert!(!enforced.allows_input("JsonlReader"));
        assert!(!enforced.allows_input("EvolutionLoopRunner"));
        assert!(enforced.produces_output("AdapterPromotionWindowEvidence"));
        assert!(enforced.produces_output("GateDecision"));
        assert!(enforced.produces_output("AdapterPromotionWindowReport"));
        assert!(enforced.forbids_capability("jsonl_io"));
        assert!(enforced.forbids_capability("file_io"));
        assert!(enforced.forbids_capability("report_directory_scan"));
        assert!(enforced.forbids_capability("http_sse"));
        assert!(enforced.forbids_capability("process_spawn"));
        assert!(enforced.forbids_capability("validation_command_spawn"));
        assert!(enforced.forbids_capability("model_call"));
        assert!(enforced.forbids_capability("promotion_action_execution"));
        assert!(enforced.forbids_capability("runner_state"));
        assert!(enforced.stays_pure_data_boundary());
        assert!(!report.may_block_current_runner());
        assert!(!enforced.may_block_current_runner());
        assert!(enforced.preserves_legacy_runner);
        assert_eq!(
            enforced.verification_plan.commands[0].display_line(),
            r"cargo test --manifest-path .\crates\norion-eval\Cargo.toml"
        );
    }
}
