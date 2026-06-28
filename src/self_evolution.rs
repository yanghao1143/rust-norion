use crate::adaptive_state::EvolutionLedger;
use crate::benchmark::{BenchmarkGate, BenchmarkGateReport, BenchmarkSummary};
use crate::hierarchy::HierarchyAdjustmentPreviewReport;
use crate::privacy_redaction::contains_private_or_executable_marker;
use crate::router::RouterThresholdAdjustmentPreviewReport;
use crate::split::bridge::KvFusionPolicyObservationDryRunReport;
use crate::tenant_scope::{
    TenantAccessKind, TenantIsolationGate, TenantResourceLane, TenantScope, TenantScopedKey,
};

#[derive(Debug, Clone, Copy)]
pub struct SelfEvolutionAdmissionPolicy {
    pub min_rust_check_items: u64,
    pub min_compiler_validation_items: u64,
    pub min_test_validation_items: u64,
    pub min_benchmark_validation_items: u64,
    pub min_experiment_validation_items: u64,
    pub require_all_rust_checks_passed: bool,
    pub require_all_validation_lanes_passed: bool,
    pub require_benchmark_gate_passed: bool,
    pub require_adaptive_preview_evidence: bool,
    pub max_drift_rollbacks: u64,
    pub max_rollback_router_threshold_delta: f32,
    pub max_rollback_hierarchy_weight_delta: f32,
}

impl Default for SelfEvolutionAdmissionPolicy {
    fn default() -> Self {
        Self {
            min_rust_check_items: 1,
            min_compiler_validation_items: 1,
            min_test_validation_items: 1,
            min_benchmark_validation_items: 1,
            min_experiment_validation_items: 1,
            require_all_rust_checks_passed: true,
            require_all_validation_lanes_passed: true,
            require_benchmark_gate_passed: true,
            require_adaptive_preview_evidence: true,
            max_drift_rollbacks: 0,
            max_rollback_router_threshold_delta: 0.0,
            max_rollback_hierarchy_weight_delta: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SelfEvolutionValidationLane {
    pub items: u64,
    pub passed: u64,
    pub failed: u64,
}

impl SelfEvolutionValidationLane {
    pub fn new(items: u64, passed: u64, failed: u64) -> Self {
        Self {
            items,
            passed,
            failed,
        }
    }

    pub fn passed_at_least(self, minimum: u64, require_all_passed: bool) -> bool {
        self.items >= minimum
            && self.passed >= minimum
            && (!require_all_passed || self.failed == 0)
            && self.passed.saturating_add(self.failed) <= self.items
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SelfEvolutionValidationEvidence {
    pub compiler: SelfEvolutionValidationLane,
    pub tests: SelfEvolutionValidationLane,
    pub benchmarks: SelfEvolutionValidationLane,
    pub experiments: SelfEvolutionValidationLane,
}

impl SelfEvolutionValidationEvidence {
    pub fn from_lanes(
        compiler: SelfEvolutionValidationLane,
        tests: SelfEvolutionValidationLane,
        benchmarks: SelfEvolutionValidationLane,
        experiments: SelfEvolutionValidationLane,
    ) -> Self {
        Self {
            compiler,
            tests,
            benchmarks,
            experiments,
        }
    }

    pub fn add_artifact(&mut self, artifact: &SelfEvolutionValidationArtifact) {
        let lane = artifact.validation_lane();
        match artifact.kind.lane() {
            SelfEvolutionValidationArtifactLane::Compiler => {
                self.compiler = self.compiler.saturating_add(lane);
            }
            SelfEvolutionValidationArtifactLane::Tests => {
                self.tests = self.tests.saturating_add(lane);
            }
            SelfEvolutionValidationArtifactLane::Benchmarks => {
                self.benchmarks = self.benchmarks.saturating_add(lane);
            }
            SelfEvolutionValidationArtifactLane::Experiments => {
                self.experiments = self.experiments.saturating_add(lane);
            }
        }
    }
}

impl SelfEvolutionValidationLane {
    fn saturating_add(self, other: Self) -> Self {
        Self {
            items: self.items.saturating_add(other.items),
            passed: self.passed.saturating_add(other.passed),
            failed: self.failed.saturating_add(other.failed),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelfEvolutionValidationArtifactLane {
    Compiler,
    Tests,
    Benchmarks,
    Experiments,
}

impl SelfEvolutionValidationArtifactLane {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Compiler => "compiler",
            Self::Tests => "tests",
            Self::Benchmarks => "benchmarks",
            Self::Experiments => "experiments",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelfEvolutionValidationArtifactKind {
    CargoCheck,
    FocusedTests,
    BenchmarkGate,
    TraceSchemaGate,
}

impl SelfEvolutionValidationArtifactKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CargoCheck => "cargo-check",
            Self::FocusedTests => "focused-tests",
            Self::BenchmarkGate => "benchmark-gate",
            Self::TraceSchemaGate => "trace-schema-gate",
        }
    }

    pub fn lane(self) -> SelfEvolutionValidationArtifactLane {
        match self {
            Self::CargoCheck => SelfEvolutionValidationArtifactLane::Compiler,
            Self::FocusedTests => SelfEvolutionValidationArtifactLane::Tests,
            Self::BenchmarkGate => SelfEvolutionValidationArtifactLane::Benchmarks,
            Self::TraceSchemaGate => SelfEvolutionValidationArtifactLane::Experiments,
        }
    }

    pub fn source_report_schema(self) -> &'static str {
        match self {
            Self::CargoCheck => "rust-norion-cargo-check-v1",
            Self::FocusedTests => "rust-norion-focused-test-v1",
            Self::BenchmarkGate => "rust-norion-benchmark-gate-v1",
            Self::TraceSchemaGate => "rust-norion-trace-schema-gate-v1",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfEvolutionValidationArtifact {
    pub kind: SelfEvolutionValidationArtifactKind,
    pub label: String,
    pub items: u64,
    pub passed: u64,
    pub failed: u64,
}

impl SelfEvolutionValidationArtifact {
    pub fn new(
        kind: SelfEvolutionValidationArtifactKind,
        label: impl Into<String>,
        items: u64,
        passed: u64,
        failed: u64,
    ) -> Self {
        Self {
            kind,
            label: label.into(),
            items,
            passed,
            failed,
        }
    }

    pub fn cargo_check(label: impl Into<String>, passed: bool) -> Self {
        Self::single(
            SelfEvolutionValidationArtifactKind::CargoCheck,
            label,
            passed,
        )
    }

    pub fn focused_tests(label: impl Into<String>, items: u64, passed: u64, failed: u64) -> Self {
        Self::new(
            SelfEvolutionValidationArtifactKind::FocusedTests,
            label,
            items,
            passed,
            failed,
        )
    }

    pub fn benchmark_gate(label: impl Into<String>, passed: bool, failure_count: usize) -> Self {
        let failed = if passed {
            0
        } else {
            failure_count.max(1) as u64
        };
        Self::new(
            SelfEvolutionValidationArtifactKind::BenchmarkGate,
            label,
            1_u64.max(u64::from(passed).saturating_add(failed)),
            u64::from(passed),
            failed,
        )
    }

    pub fn trace_schema_gate(
        label: impl Into<String>,
        passed: bool,
        checked_lines: usize,
        failure_count: usize,
    ) -> Self {
        let failed = if passed {
            0
        } else {
            failure_count.max(1) as u64
        };
        Self::new(
            SelfEvolutionValidationArtifactKind::TraceSchemaGate,
            label,
            (checked_lines as u64).max(u64::from(passed).saturating_add(failed)),
            u64::from(passed),
            failed,
        )
    }

    fn single(
        kind: SelfEvolutionValidationArtifactKind,
        label: impl Into<String>,
        passed: bool,
    ) -> Self {
        Self::new(kind, label, 1, u64::from(passed), u64::from(!passed))
    }

    fn validation_lane(&self) -> SelfEvolutionValidationLane {
        SelfEvolutionValidationLane::new(self.items, self.passed, self.failed)
    }

    fn evidence_id(&self, candidate_id: &str) -> String {
        let candidate = self_evolution_review_id_component(candidate_id);
        let label = self_evolution_review_id_component(&self.label);
        format!(
            "validation-artifact:{}:{}:{}:items-{}:passed-{}:failed-{}",
            self.kind.as_str(),
            candidate,
            label,
            self.items,
            self.passed,
            self.failed
        )
    }

    fn content_digest(&self, candidate_id: &str) -> String {
        self_evolution_stable_digest(&format!(
            "validation_artifact;candidate={candidate_id};kind={};lane={};label={};items={};passed={};failed={}",
            self.kind.as_str(),
            self.kind.lane().as_str(),
            self.label,
            self.items,
            self.passed,
            self.failed
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfEvolutionAdmissionReviewPacketRefs {
    pub approval_review_packet_ids: Vec<String>,
    pub evidence_ids: Vec<String>,
    pub rollback_anchor_ids: Vec<String>,
    pub content_digests: Vec<String>,
    pub source_report_schemas: Vec<String>,
}

impl Default for SelfEvolutionAdmissionReviewPacketRefs {
    fn default() -> Self {
        Self {
            approval_review_packet_ids: Vec::new(),
            evidence_ids: Vec::new(),
            rollback_anchor_ids: Vec::new(),
            content_digests: Vec::new(),
            source_report_schemas: Vec::new(),
        }
    }
}

impl SelfEvolutionAdmissionReviewPacketRefs {
    fn derived(
        candidate_id: &str,
        evolution_ledger: EvolutionLedger,
        benchmark_gate: &BenchmarkGateReport,
    ) -> Self {
        let candidate = self_evolution_review_id_component(candidate_id);
        let mut refs = Self::default();
        refs.push_approval_review_packet_id(format!("approval-review:{candidate}"));
        refs.push_evidence_id(format!(
            "rust-check:{candidate}:items-{}:passed-{}:failed-{}",
            evolution_ledger.replay_rust_check_items,
            evolution_ledger.replay_rust_check_passed,
            evolution_ledger.replay_rust_check_failed
        ));
        refs.push_evidence_id(format!(
            "benchmark-gate:{candidate}:passed-{}:failures-{}",
            benchmark_gate.passed,
            benchmark_gate.failures.len()
        ));
        refs.push_rollback_anchor_id(format!(
            "rollback-budget:{candidate}:drift-{}",
            evolution_ledger.drift_rollbacks
        ));
        refs.push_content_digest(self_evolution_stable_digest(&format!(
            "candidate={candidate_id};rust_check_items={};rust_check_passed={};rust_check_failed={};benchmark_gate_passed={};benchmark_gate_failures={};drift_rollbacks={};router_delta={:.6};hierarchy_delta={:.6}",
            evolution_ledger.replay_rust_check_items,
            evolution_ledger.replay_rust_check_passed,
            evolution_ledger.replay_rust_check_failed,
            benchmark_gate.passed,
            benchmark_gate.failures.len(),
            evolution_ledger.drift_rollbacks,
            evolution_ledger.rollback_router_threshold_delta,
            evolution_ledger.rollback_hierarchy_weight_delta
        )));
        refs.push_source_report_schema("rust-norion-self-evolution-admission-v1");
        refs.push_source_report_schema("rust-norion-benchmark-gate-v1");
        refs
    }

    fn rollback_replay_gate(plan: &SelfEvolutionRollbackReplayPlan, content_digest: &str) -> Self {
        let review = self_evolution_review_id_component(content_digest);
        let mut refs = Self::default();
        refs.push_approval_review_packet_id(format!("rollback-replay-review:{review}"));
        refs.push_evidence_id(format!(
            "rollback-replay-plan:items-{}:replayable-{}:blocked-{}",
            plan.item_count(),
            plan.replayable(),
            plan.blocked()
        ));
        for evidence_id in plan.evidence_ids() {
            refs.push_evidence_id(evidence_id);
        }
        for anchor_id in plan.rollback_anchor_ids() {
            refs.push_rollback_anchor_id(anchor_id);
        }
        refs.push_content_digest(content_digest.to_owned());
        refs.push_content_digest(self_evolution_stable_digest(&format!(
            "rollback_replay_plan_summary={}",
            plan.summary_line()
        )));
        for item in &plan.items {
            refs.push_content_digest(item.content_digest.clone());
        }
        refs.push_source_report_schema("rust-norion-self-evolution-rollback-replay-gate-v1");
        refs.push_source_report_schema("rust-norion-self-evolution-rollback-replay-plan-v1");
        refs.push_source_report_schema("rust-norion-self-evolution-experiment-v1");
        refs
    }

    pub fn push_approval_review_packet_id(&mut self, value: impl Into<String>) {
        self.push_scoped_approval_review_packet_id(&TenantScope::local_single_user(), value);
    }

    pub fn push_scoped_approval_review_packet_id(
        &mut self,
        scope: &TenantScope,
        value: impl Into<String>,
    ) {
        let value = value.into();
        let scoped = scope.scoped_key(TenantResourceLane::ApprovalPacket, value);
        push_unique_string(
            &mut self.approval_review_packet_ids,
            scoped.as_str().to_owned(),
        );
    }

    pub fn push_evidence_id(&mut self, value: impl Into<String>) {
        push_unique_string(&mut self.evidence_ids, value);
    }

    pub fn push_rollback_anchor_id(&mut self, value: impl Into<String>) {
        self.push_scoped_rollback_anchor_id(&TenantScope::local_single_user(), value);
    }

    pub fn push_scoped_rollback_anchor_id(
        &mut self,
        scope: &TenantScope,
        value: impl Into<String>,
    ) {
        let value = value.into();
        let scoped = scope.scoped_key(TenantResourceLane::SessionState, value);
        push_unique_string(&mut self.rollback_anchor_ids, scoped.as_str().to_owned());
    }

    pub fn push_content_digest(&mut self, value: impl Into<String>) {
        push_unique_string(&mut self.content_digests, value);
    }

    pub fn push_source_report_schema(&mut self, value: impl Into<String>) {
        push_unique_string(&mut self.source_report_schemas, value);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SelfEvolutionPromotionLane {
    Memory,
    Genome,
    Routing,
    RuntimeAdapter,
    TaskSkillGene,
    ToolPolicy,
}

impl SelfEvolutionPromotionLane {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Memory => "memory",
            Self::Genome => "genome",
            Self::Routing => "routing",
            Self::RuntimeAdapter => "runtime_adapter",
            Self::TaskSkillGene => "task_skill_gene",
            Self::ToolPolicy => "tool_policy",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelfEvolutionPromotionDecision {
    PromoteForApproval,
    HoldForEvidence,
    InsufficientEvidence,
    Reject,
    Rollback,
}

impl SelfEvolutionPromotionDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PromoteForApproval => "promote_for_approval",
            Self::HoldForEvidence => "hold_for_evidence",
            Self::InsufficientEvidence => "insufficient_evidence",
            Self::Reject => "reject",
            Self::Rollback => "rollback",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SelfEvolutionRegressionBudget {
    pub max_correctness_regression: f32,
    pub max_latency_regression_ms: i64,
    pub max_wasted_compute_regression: f32,
    pub max_privacy_risk: f32,
    pub max_cross_task_regression: f32,
    pub max_flaky_runs: u64,
}

impl SelfEvolutionRegressionBudget {
    pub const fn strict() -> Self {
        Self {
            max_correctness_regression: 0.005,
            max_latency_regression_ms: 5,
            max_wasted_compute_regression: 0.01,
            max_privacy_risk: 0.0,
            max_cross_task_regression: 0.005,
            max_flaky_runs: 0,
        }
    }

    pub const fn balanced() -> Self {
        Self {
            max_correctness_regression: 0.01,
            max_latency_regression_ms: 25,
            max_wasted_compute_regression: 0.03,
            max_privacy_risk: 0.0,
            max_cross_task_regression: 0.02,
            max_flaky_runs: 0,
        }
    }

    pub const fn runtime_adapter() -> Self {
        Self {
            max_correctness_regression: 0.005,
            max_latency_regression_ms: 10,
            max_wasted_compute_regression: 0.02,
            max_privacy_risk: 0.0,
            max_cross_task_regression: 0.01,
            max_flaky_runs: 0,
        }
    }

    fn normalized(self) -> Self {
        Self {
            max_correctness_regression: finite_or_zero(self.max_correctness_regression).max(0.0),
            max_latency_regression_ms: self.max_latency_regression_ms.max(0),
            max_wasted_compute_regression: finite_or_zero(self.max_wasted_compute_regression)
                .max(0.0),
            max_privacy_risk: finite_or_zero(self.max_privacy_risk).clamp(0.0, 1.0),
            max_cross_task_regression: finite_or_zero(self.max_cross_task_regression).max(0.0),
            max_flaky_runs: self.max_flaky_runs,
        }
    }
}

impl Default for SelfEvolutionRegressionBudget {
    fn default() -> Self {
        Self::balanced()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SelfEvolutionPromotionPolicy {
    pub min_correctness_delta: f32,
    pub min_reproducible_runs: u64,
    pub require_validation_passed: bool,
    pub require_rollback_ready: bool,
    pub memory_budget: SelfEvolutionRegressionBudget,
    pub genome_budget: SelfEvolutionRegressionBudget,
    pub routing_budget: SelfEvolutionRegressionBudget,
    pub runtime_adapter_budget: SelfEvolutionRegressionBudget,
    pub task_skill_gene_budget: SelfEvolutionRegressionBudget,
    pub tool_policy_budget: SelfEvolutionRegressionBudget,
}

impl Default for SelfEvolutionPromotionPolicy {
    fn default() -> Self {
        Self {
            min_correctness_delta: 0.0,
            min_reproducible_runs: 2,
            require_validation_passed: true,
            require_rollback_ready: true,
            memory_budget: SelfEvolutionRegressionBudget::balanced(),
            genome_budget: SelfEvolutionRegressionBudget::strict(),
            routing_budget: SelfEvolutionRegressionBudget::balanced(),
            runtime_adapter_budget: SelfEvolutionRegressionBudget::runtime_adapter(),
            task_skill_gene_budget: SelfEvolutionRegressionBudget::strict(),
            tool_policy_budget: SelfEvolutionRegressionBudget::balanced(),
        }
    }
}

impl SelfEvolutionPromotionPolicy {
    pub fn budget_for(self, lane: SelfEvolutionPromotionLane) -> SelfEvolutionRegressionBudget {
        match lane {
            SelfEvolutionPromotionLane::Memory => self.memory_budget,
            SelfEvolutionPromotionLane::Genome => self.genome_budget,
            SelfEvolutionPromotionLane::Routing => self.routing_budget,
            SelfEvolutionPromotionLane::RuntimeAdapter => self.runtime_adapter_budget,
            SelfEvolutionPromotionLane::TaskSkillGene => self.task_skill_gene_budget,
            SelfEvolutionPromotionLane::ToolPolicy => self.tool_policy_budget,
        }
        .normalized()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfEvolutionPromotionArtifactRef {
    pub label: String,
    pub content_digest: String,
    pub trace_id: Option<String>,
    pub source_schema: String,
}

impl SelfEvolutionPromotionArtifactRef {
    pub fn digest(label: impl Into<String>, content_digest: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            content_digest: content_digest.into(),
            trace_id: None,
            source_schema: "rust-norion-promotion-artifact-v1".to_owned(),
        }
    }

    pub fn trace(
        label: impl Into<String>,
        trace_id: impl Into<String>,
        content_digest: impl Into<String>,
    ) -> Self {
        Self {
            label: label.into(),
            content_digest: content_digest.into(),
            trace_id: Some(trace_id.into()),
            source_schema: "rust-norion-trace-artifact-v1".to_owned(),
        }
    }

    pub fn with_source_schema(mut self, source_schema: impl Into<String>) -> Self {
        self.source_schema = source_schema.into();
        self
    }

    fn redacted(&self) -> Self {
        Self {
            label: self_evolution_review_id_component(&self.label),
            content_digest: digest_like_or_redacted(&self.content_digest),
            trace_id: self
                .trace_id
                .as_deref()
                .map(self_evolution_review_id_component),
            source_schema: self_evolution_review_id_component(&self.source_schema),
        }
    }

    fn is_safe_ref(&self) -> bool {
        !self.label.trim().is_empty()
            && !self.content_digest.trim().is_empty()
            && !contains_private_or_executable_marker(&self.label)
            && !contains_private_or_executable_marker(&self.content_digest)
            && !self
                .trace_id
                .as_deref()
                .is_some_and(contains_private_or_executable_marker)
            && !contains_private_or_executable_marker(&self.source_schema)
            && digest_or_trace_like(&self.content_digest)
            && self.trace_id.as_deref().map_or(true, trace_id_like)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelfEvolutionPromotionCandidate {
    pub candidate_id: String,
    pub lane: SelfEvolutionPromotionLane,
    pub correctness_delta: f32,
    pub latency_delta_ms: i64,
    pub wasted_compute_delta: f32,
    pub privacy_risk: f32,
    pub reproducible_runs: u64,
    pub cross_task_regression: f32,
    pub flaky_runs: u64,
    pub rollback_ready: bool,
    pub rollback_anchor_id: String,
    pub validation: SelfEvolutionValidationEvidence,
    pub artifact_refs: Vec<SelfEvolutionPromotionArtifactRef>,
}

impl SelfEvolutionPromotionCandidate {
    pub fn new(candidate_id: impl Into<String>, lane: SelfEvolutionPromotionLane) -> Self {
        Self {
            candidate_id: candidate_id.into(),
            lane,
            correctness_delta: 0.0,
            latency_delta_ms: 0,
            wasted_compute_delta: 0.0,
            privacy_risk: 0.0,
            reproducible_runs: 0,
            cross_task_regression: 0.0,
            flaky_runs: 0,
            rollback_ready: false,
            rollback_anchor_id: String::new(),
            validation: SelfEvolutionValidationEvidence::default(),
            artifact_refs: Vec::new(),
        }
    }

    pub fn with_correctness_delta(mut self, correctness_delta: f32) -> Self {
        self.correctness_delta = finite_or_zero(correctness_delta);
        self
    }

    pub fn with_latency_delta_ms(mut self, latency_delta_ms: i64) -> Self {
        self.latency_delta_ms = latency_delta_ms;
        self
    }

    pub fn with_wasted_compute_delta(mut self, wasted_compute_delta: f32) -> Self {
        self.wasted_compute_delta = finite_or_zero(wasted_compute_delta);
        self
    }

    pub fn with_privacy_risk(mut self, privacy_risk: f32) -> Self {
        self.privacy_risk = finite_or_zero(privacy_risk).clamp(0.0, 1.0);
        self
    }

    pub fn with_reproducible_runs(mut self, reproducible_runs: u64) -> Self {
        self.reproducible_runs = reproducible_runs;
        self
    }

    pub fn with_cross_task_regression(mut self, cross_task_regression: f32) -> Self {
        self.cross_task_regression = finite_or_zero(cross_task_regression).max(0.0);
        self
    }

    pub fn with_flaky_runs(mut self, flaky_runs: u64) -> Self {
        self.flaky_runs = flaky_runs;
        self
    }

    pub fn with_rollback(mut self, rollback_anchor_id: impl Into<String>) -> Self {
        self.rollback_ready = true;
        self.rollback_anchor_id = rollback_anchor_id.into();
        self
    }

    pub fn with_validation(mut self, validation: SelfEvolutionValidationEvidence) -> Self {
        self.validation = validation;
        self
    }

    pub fn with_artifact_ref(mut self, artifact_ref: SelfEvolutionPromotionArtifactRef) -> Self {
        self.artifact_refs.push(artifact_ref);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelfEvolutionPromotionScorecard {
    pub candidate_id: String,
    pub lane: SelfEvolutionPromotionLane,
    pub decision: SelfEvolutionPromotionDecision,
    pub ready_for_human_approval: bool,
    pub human_approval_required: bool,
    pub correctness_delta: f32,
    pub latency_delta_ms: i64,
    pub wasted_compute_delta: f32,
    pub privacy_risk: f32,
    pub reproducible_runs: u64,
    pub cross_task_regression: f32,
    pub flaky_runs: u64,
    pub rollback_ready: bool,
    pub rollback_anchor_id: String,
    pub validation_passed: bool,
    pub artifact_refs: Vec<SelfEvolutionPromotionArtifactRef>,
    pub evidence_digest: String,
    pub blocked_reasons: Vec<String>,
    pub read_only: bool,
    pub report_only: bool,
    pub preview_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl SelfEvolutionPromotionScorecard {
    pub fn summary_line(&self) -> String {
        format!(
            "self_evolution_promotion_scorecard candidate={} lane={} decision={} ready_for_human_approval={} human_approval_required={} correctness_delta={:.6} latency_delta_ms={} wasted_compute_delta={:.6} privacy_risk={:.6} reproducible_runs={} cross_task_regression={:.6} flaky_runs={} rollback_ready={} validation_passed={} artifacts={} blocked_reasons={} read_only={} report_only={} preview_only={} write_allowed={} applied={} evidence_digest={}",
            self.candidate_id,
            self.lane.as_str(),
            self.decision.as_str(),
            self.ready_for_human_approval,
            self.human_approval_required,
            self.correctness_delta,
            self.latency_delta_ms,
            self.wasted_compute_delta,
            self.privacy_risk,
            self.reproducible_runs,
            self.cross_task_regression,
            self.flaky_runs,
            self.rollback_ready,
            self.validation_passed,
            self.artifact_refs.len(),
            self.blocked_reasons.len(),
            self.read_only,
            self.report_only,
            self.preview_only,
            self.write_allowed,
            self.applied,
            self.evidence_digest
        )
    }

    pub fn review_packet_line(&self) -> String {
        let artifact_digests = self
            .artifact_refs
            .iter()
            .map(|artifact| artifact.content_digest.clone())
            .collect::<Vec<_>>();
        let trace_ids = self
            .artifact_refs
            .iter()
            .filter_map(|artifact| artifact.trace_id.clone())
            .collect::<Vec<_>>();
        format!(
            "self_evolution_promotion_packet candidate={} lane={} decision={} artifact_digests={} trace_ids={} rollback_anchor={} evidence_digest={} blocked={}",
            self.candidate_id,
            self.lane.as_str(),
            self.decision.as_str(),
            artifact_digests.join(","),
            trace_ids.join(","),
            self.rollback_anchor_id,
            self.evidence_digest,
            self.blocked_reasons.join("|")
        )
    }

    pub fn json_line(&self) -> String {
        let candidate_id = self_evolution_json_escape(&self.candidate_id);
        let lane = self.lane.as_str();
        let decision = self.decision.as_str();
        let rollback_anchor_id = self_evolution_json_escape(&self.rollback_anchor_id);
        let evidence_digest = self_evolution_json_escape(&self.evidence_digest);
        let blocked_reasons = self_evolution_string_array_json(&self.blocked_reasons);
        let artifact_digests = self_evolution_string_array_json(
            &self
                .artifact_refs
                .iter()
                .map(|artifact| artifact.content_digest.clone())
                .collect::<Vec<_>>(),
        );
        let trace_ids = self_evolution_string_array_json(
            &self
                .artifact_refs
                .iter()
                .filter_map(|artifact| artifact.trace_id.clone())
                .collect::<Vec<_>>(),
        );
        let correctness_delta = self_evolution_f32_json(self.correctness_delta);
        let wasted_compute_delta = self_evolution_f32_json(self.wasted_compute_delta);
        let privacy_risk = self_evolution_f32_json(self.privacy_risk);
        let cross_task_regression = self_evolution_f32_json(self.cross_task_regression);
        format!(
            "{{\"schema\":\"rust-norion-self-evolution-promotion-scorecard-v1\",\"candidate_id\":\"{candidate_id}\",\"lane\":\"{lane}\",\"decision\":\"{decision}\",\"ready_for_human_approval\":{},\"human_approval_required\":{},\"correctness_delta\":{correctness_delta},\"latency_delta_ms\":{},\"wasted_compute_delta\":{wasted_compute_delta},\"privacy_risk\":{privacy_risk},\"reproducible_runs\":{},\"cross_task_regression\":{cross_task_regression},\"flaky_runs\":{},\"rollback_ready\":{},\"rollback_anchor_id\":\"{rollback_anchor_id}\",\"validation_passed\":{},\"artifact_digests\":{artifact_digests},\"trace_ids\":{trace_ids},\"blocked_reasons\":{blocked_reasons},\"read_only\":{},\"report_only\":{},\"preview_only\":{},\"write_allowed\":{},\"applied\":{},\"evidence_digest\":\"{evidence_digest}\"}}",
            self.ready_for_human_approval,
            self.human_approval_required,
            self.latency_delta_ms,
            self.reproducible_runs,
            self.flaky_runs,
            self.rollback_ready,
            self.validation_passed,
            self.read_only,
            self.report_only,
            self.preview_only,
            self.write_allowed,
            self.applied
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SelfEvolutionPromotionScorecardGate {
    pub policy: SelfEvolutionPromotionPolicy,
}

impl Default for SelfEvolutionPromotionScorecardGate {
    fn default() -> Self {
        Self {
            policy: SelfEvolutionPromotionPolicy::default(),
        }
    }
}

impl SelfEvolutionPromotionScorecardGate {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_policy(mut self, policy: SelfEvolutionPromotionPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn evaluate(
        &self,
        candidate: &SelfEvolutionPromotionCandidate,
    ) -> SelfEvolutionPromotionScorecard {
        let budget = self.policy.budget_for(candidate.lane);
        let validation_passed = self_evolution_promotion_validation_passed(candidate.validation);
        let mut blocked_reasons = Vec::new();
        let mut rollback_required = false;
        let mut rejection_required = false;
        let mut insufficient_evidence = false;

        let candidate_id = self_evolution_review_id_component(&candidate.candidate_id);
        if candidate.candidate_id.trim().is_empty() {
            insufficient_evidence = true;
            blocked_reasons.push("promotion_candidate_id_missing".to_owned());
        }
        if candidate.artifact_refs.is_empty() {
            insufficient_evidence = true;
            blocked_reasons.push("promotion_artifact_refs_missing".to_owned());
        }
        for artifact in &candidate.artifact_refs {
            if !artifact.is_safe_ref() {
                rejection_required = true;
                blocked_reasons.push("promotion_artifact_ref_not_digest_or_trace_only".to_owned());
                break;
            }
        }
        if candidate.reproducible_runs < self.policy.min_reproducible_runs {
            insufficient_evidence = true;
            blocked_reasons.push(format!(
                "promotion_reproducible_runs={}<{}",
                candidate.reproducible_runs, self.policy.min_reproducible_runs
            ));
        }
        if self.policy.require_validation_passed && !validation_passed {
            blocked_reasons.extend(self_evolution_promotion_validation_blockers(
                candidate.validation,
            ));
        }
        if candidate.correctness_delta < -budget.max_correctness_regression {
            rollback_required = true;
            blocked_reasons.push(format!(
                "promotion_correctness_regression={:.6}>{:.6}",
                -candidate.correctness_delta, budget.max_correctness_regression
            ));
        }
        if candidate.correctness_delta < self.policy.min_correctness_delta {
            blocked_reasons.push(format!(
                "promotion_correctness_delta={:.6}<{}",
                candidate.correctness_delta, self.policy.min_correctness_delta
            ));
        }
        if candidate.latency_delta_ms > budget.max_latency_regression_ms {
            blocked_reasons.push(format!(
                "promotion_latency_regression_ms={}>{}",
                candidate.latency_delta_ms, budget.max_latency_regression_ms
            ));
        }
        if candidate.wasted_compute_delta > budget.max_wasted_compute_regression {
            blocked_reasons.push(format!(
                "promotion_wasted_compute_regression={:.6}>{:.6}",
                candidate.wasted_compute_delta, budget.max_wasted_compute_regression
            ));
        }
        if candidate.privacy_risk > budget.max_privacy_risk {
            rejection_required = true;
            blocked_reasons.push(format!(
                "promotion_privacy_risk={:.6}>{:.6}",
                candidate.privacy_risk, budget.max_privacy_risk
            ));
        }
        if candidate.cross_task_regression > budget.max_cross_task_regression {
            rollback_required = true;
            blocked_reasons.push(format!(
                "promotion_cross_task_regression={:.6}>{:.6}",
                candidate.cross_task_regression, budget.max_cross_task_regression
            ));
        }
        if candidate.flaky_runs > budget.max_flaky_runs {
            blocked_reasons.push(format!(
                "promotion_flaky_runs={}>{}",
                candidate.flaky_runs, budget.max_flaky_runs
            ));
        }
        if self.policy.require_rollback_ready
            && (!candidate.rollback_ready || candidate.rollback_anchor_id.trim().is_empty())
        {
            rollback_required = true;
            blocked_reasons.push("promotion_rollback_not_ready".to_owned());
        }

        let decision = if rejection_required {
            SelfEvolutionPromotionDecision::Reject
        } else if rollback_required {
            SelfEvolutionPromotionDecision::Rollback
        } else if insufficient_evidence {
            SelfEvolutionPromotionDecision::InsufficientEvidence
        } else if blocked_reasons.is_empty() {
            SelfEvolutionPromotionDecision::PromoteForApproval
        } else {
            SelfEvolutionPromotionDecision::HoldForEvidence
        };

        let artifact_refs = candidate
            .artifact_refs
            .iter()
            .map(SelfEvolutionPromotionArtifactRef::redacted)
            .collect::<Vec<_>>();
        let evidence_digest =
            self_evolution_promotion_digest(candidate, &artifact_refs, decision, &blocked_reasons);

        SelfEvolutionPromotionScorecard {
            candidate_id,
            lane: candidate.lane,
            decision,
            ready_for_human_approval: decision
                == SelfEvolutionPromotionDecision::PromoteForApproval,
            human_approval_required: true,
            correctness_delta: finite_or_zero(candidate.correctness_delta),
            latency_delta_ms: candidate.latency_delta_ms,
            wasted_compute_delta: finite_or_zero(candidate.wasted_compute_delta),
            privacy_risk: finite_or_zero(candidate.privacy_risk).clamp(0.0, 1.0),
            reproducible_runs: candidate.reproducible_runs,
            cross_task_regression: finite_or_zero(candidate.cross_task_regression).max(0.0),
            flaky_runs: candidate.flaky_runs,
            rollback_ready: candidate.rollback_ready,
            rollback_anchor_id: self_evolution_review_id_component(&candidate.rollback_anchor_id),
            validation_passed,
            artifact_refs,
            evidence_digest,
            blocked_reasons,
            read_only: true,
            report_only: true,
            preview_only: true,
            write_allowed: false,
            applied: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SelfEvolutionAdmissionEvidence {
    pub candidate_id: String,
    pub evolution_ledger: EvolutionLedger,
    pub benchmark_gate_passed: bool,
    pub benchmark_gate_failures: Vec<String>,
    pub router_threshold_preview_ready: bool,
    pub hierarchy_adjustment_preview_ready: bool,
    pub kv_fusion_policy_observation_preview_ready: bool,
    pub adaptive_preview_source_count: usize,
    pub adaptive_preview_read_only: bool,
    pub adaptive_preview_report_only: bool,
    pub adaptive_preview_preview_only: bool,
    pub adaptive_preview_write_allowed: bool,
    pub adaptive_preview_applied: bool,
    pub adaptive_preview_blocked_reasons: Vec<String>,
    pub validation: SelfEvolutionValidationEvidence,
    pub review_packet: SelfEvolutionAdmissionReviewPacketRefs,
}

impl SelfEvolutionAdmissionEvidence {
    pub fn from_benchmark_gate(
        candidate_id: impl Into<String>,
        evolution_ledger: EvolutionLedger,
        benchmark_gate: &BenchmarkGateReport,
    ) -> Self {
        let candidate_id = candidate_id.into();
        Self {
            review_packet: SelfEvolutionAdmissionReviewPacketRefs::derived(
                &candidate_id,
                evolution_ledger,
                benchmark_gate,
            ),
            candidate_id,
            evolution_ledger,
            benchmark_gate_passed: benchmark_gate.passed,
            benchmark_gate_failures: benchmark_gate.failures.clone(),
            router_threshold_preview_ready: false,
            hierarchy_adjustment_preview_ready: false,
            kv_fusion_policy_observation_preview_ready: false,
            adaptive_preview_source_count: 0,
            adaptive_preview_read_only: true,
            adaptive_preview_report_only: true,
            adaptive_preview_preview_only: true,
            adaptive_preview_write_allowed: false,
            adaptive_preview_applied: false,
            adaptive_preview_blocked_reasons: Vec::new(),
            validation: SelfEvolutionValidationEvidence {
                compiler: SelfEvolutionValidationLane::new(
                    evolution_ledger.replay_rust_check_items,
                    evolution_ledger.replay_rust_check_passed,
                    evolution_ledger.replay_rust_check_failed,
                ),
                benchmarks: SelfEvolutionValidationLane::new(
                    u64::from(!benchmark_gate.failures.is_empty() || benchmark_gate.passed),
                    u64::from(benchmark_gate.passed),
                    u64::from(!benchmark_gate.passed),
                ),
                ..SelfEvolutionValidationEvidence::default()
            },
        }
    }

    pub fn from_benchmark_summary(
        candidate_id: impl Into<String>,
        summary: &BenchmarkSummary,
        gate: &BenchmarkGate,
    ) -> Self {
        let benchmark_gate = summary.evaluate(gate);
        Self::from_benchmark_gate(candidate_id, summary.evolution_ledger(), &benchmark_gate)
    }

    pub fn with_validation_evidence(mut self, validation: SelfEvolutionValidationEvidence) -> Self {
        self.validation = validation;
        let candidate = self_evolution_review_id_component(&self.candidate_id);
        self.review_packet.push_evidence_id(format!(
            "validation:{candidate}:compiler-{}/{}:{}:tests-{}/{}:{}:benchmarks-{}/{}:{}:experiments-{}/{}:{}",
            validation.compiler.passed,
            validation.compiler.items,
            validation.compiler.failed,
            validation.tests.passed,
            validation.tests.items,
            validation.tests.failed,
            validation.benchmarks.passed,
            validation.benchmarks.items,
            validation.benchmarks.failed,
            validation.experiments.passed,
            validation.experiments.items,
            validation.experiments.failed,
        ));
        self.review_packet
            .push_content_digest(self_evolution_stable_digest(&format!(
                "candidate={};compiler={:?};tests={:?};benchmarks={:?};experiments={:?}",
                self.candidate_id,
                validation.compiler,
                validation.tests,
                validation.benchmarks,
                validation.experiments
            )));
        self.review_packet
            .push_source_report_schema("rust-norion-self-evolution-validation-v1");
        self
    }

    pub fn with_validation_artifact(mut self, artifact: SelfEvolutionValidationArtifact) -> Self {
        if artifact.kind == SelfEvolutionValidationArtifactKind::CargoCheck {
            self.evolution_ledger.replay_rust_check_items = self
                .evolution_ledger
                .replay_rust_check_items
                .saturating_add(artifact.items);
            self.evolution_ledger.replay_rust_check_passed = self
                .evolution_ledger
                .replay_rust_check_passed
                .saturating_add(artifact.passed);
            self.evolution_ledger.replay_rust_check_failed = self
                .evolution_ledger
                .replay_rust_check_failed
                .saturating_add(artifact.failed);
        }
        self.validation.add_artifact(&artifact);
        self.review_packet
            .push_evidence_id(artifact.evidence_id(&self.candidate_id));
        self.review_packet
            .push_content_digest(artifact.content_digest(&self.candidate_id));
        self.review_packet
            .push_source_report_schema("rust-norion-self-evolution-validation-artifact-v1");
        self.review_packet
            .push_source_report_schema(artifact.kind.source_report_schema());
        self
    }

    pub fn with_validation_artifacts(
        mut self,
        artifacts: impl IntoIterator<Item = SelfEvolutionValidationArtifact>,
    ) -> Self {
        for artifact in artifacts {
            self = self.with_validation_artifact(artifact);
        }
        self
    }

    pub fn with_router_threshold_preview_report(
        mut self,
        report: &RouterThresholdAdjustmentPreviewReport,
    ) -> Self {
        self.record_adaptive_preview_safety(AdaptivePreviewSafety {
            read_only: report.read_only,
            report_only: report.report_only,
            preview_only: report.preview_only,
            write_allowed: report.router_state_write_allowed
                || report.adaptive_state_write_allowed
                || report.ndkv_write_allowed,
            applied: report.router_observation_applied,
        });
        let ready = router_threshold_preview_admissible(report);
        self.router_threshold_preview_ready = ready;
        if !ready {
            self.adaptive_preview_blocked_reasons
                .extend(router_threshold_preview_blocked_reasons(report));
        }
        let candidate = self_evolution_review_id_component(&self.candidate_id);
        self.review_packet.push_evidence_id(format!(
            "adaptive-preview:router-threshold:{candidate}:ready-{ready}"
        ));
        self.review_packet.push_content_digest(self_evolution_stable_digest(&format!(
            "candidate={};router_threshold_ready={ready};source_count={};read_only={};report_only={};preview_only={};write_allowed={};applied={}",
            self.candidate_id,
            self.adaptive_preview_source_count,
            report.read_only,
            report.report_only,
            report.preview_only,
            report.router_state_write_allowed
                || report.adaptive_state_write_allowed
                || report.ndkv_write_allowed,
            report.router_observation_applied
        )));
        self.review_packet
            .push_source_report_schema("rust-norion-router-threshold-preview-v1");
        self
    }

    pub fn with_hierarchy_adjustment_preview_report(
        mut self,
        report: &HierarchyAdjustmentPreviewReport,
    ) -> Self {
        self.record_adaptive_preview_safety(AdaptivePreviewSafety {
            read_only: report.read_only,
            report_only: report.report_only,
            preview_only: report.preview_only,
            write_allowed: report.state_write_allowed
                || report.adaptive_state_write_allowed
                || report.ndkv_write_allowed,
            applied: report.controller_observation_applied,
        });
        let ready = hierarchy_adjustment_preview_admissible(report);
        self.hierarchy_adjustment_preview_ready = ready;
        if !ready {
            self.adaptive_preview_blocked_reasons
                .extend(hierarchy_adjustment_preview_blocked_reasons(report));
        }
        let candidate = self_evolution_review_id_component(&self.candidate_id);
        self.review_packet.push_evidence_id(format!(
            "adaptive-preview:hierarchy-adjustment:{candidate}:ready-{ready}"
        ));
        self.review_packet.push_content_digest(self_evolution_stable_digest(&format!(
            "candidate={};hierarchy_adjustment_ready={ready};source_count={};read_only={};report_only={};preview_only={};write_allowed={};applied={}",
            self.candidate_id,
            self.adaptive_preview_source_count,
            report.read_only,
            report.report_only,
            report.preview_only,
            report.state_write_allowed
                || report.adaptive_state_write_allowed
                || report.ndkv_write_allowed,
            report.controller_observation_applied
        )));
        self.review_packet
            .push_source_report_schema("rust-norion-hierarchy-adjustment-preview-v1");
        self
    }

    pub fn with_kv_fusion_policy_observation_preview_report(
        mut self,
        report: &KvFusionPolicyObservationDryRunReport,
    ) -> Self {
        self.record_adaptive_preview_safety(AdaptivePreviewSafety {
            read_only: report.reward_preview_source_read_only,
            report_only: true,
            preview_only: report.preview_only,
            write_allowed: report.policy_write_allowed
                || report.reward_preview_source_memory_store_write_allowed
                || report.reward_preview_memory_store_write_allowed
                || report.reward_preview_kv_cache_write_allowed,
            applied: report.policy_observation_applied,
        });
        let ready = report.can_use_policy_observation_preview();
        self.kv_fusion_policy_observation_preview_ready = ready;
        if !ready {
            self.adaptive_preview_blocked_reasons
                .extend(kv_fusion_policy_observation_preview_blocked_reasons(report));
        }
        let candidate = self_evolution_review_id_component(&self.candidate_id);
        self.review_packet.push_evidence_id(format!(
            "adaptive-preview:kv-fusion-policy-observation:{candidate}:ready-{ready}"
        ));
        self.review_packet.push_content_digest(self_evolution_stable_digest(&format!(
            "candidate={};kv_fusion_policy_observation_ready={ready};source_count={};source_read_only={};source_memory_store_write_allowed={};preview_only={};memory_store_write_allowed={};kv_cache_write_allowed={};policy_write_allowed={};applied={}",
            self.candidate_id,
            self.adaptive_preview_source_count,
            report.reward_preview_source_read_only,
            report.reward_preview_source_memory_store_write_allowed,
            report.preview_only,
            report.reward_preview_memory_store_write_allowed,
            report.reward_preview_kv_cache_write_allowed,
            report.policy_write_allowed,
            report.policy_observation_applied
        )));
        self.review_packet
            .push_source_report_schema("rust-norion-kv-fusion-policy-observation-preview-v1");
        self
    }

    pub fn with_review_packet_refs(
        mut self,
        review_packet: SelfEvolutionAdmissionReviewPacketRefs,
    ) -> Self {
        self.review_packet = review_packet;
        self
    }

    pub fn adaptive_preview_evidence_present(&self) -> bool {
        self.router_threshold_preview_ready
            || self.hierarchy_adjustment_preview_ready
            || self.kv_fusion_policy_observation_preview_ready
    }

    fn record_adaptive_preview_safety(&mut self, safety: AdaptivePreviewSafety) {
        self.adaptive_preview_source_count = self.adaptive_preview_source_count.saturating_add(1);
        self.adaptive_preview_read_only &= safety.read_only;
        self.adaptive_preview_report_only &= safety.report_only;
        self.adaptive_preview_preview_only &= safety.preview_only;
        self.adaptive_preview_write_allowed |= safety.write_allowed;
        self.adaptive_preview_applied |= safety.applied;
    }
}

#[derive(Debug, Clone, Copy)]
struct AdaptivePreviewSafety {
    read_only: bool,
    report_only: bool,
    preview_only: bool,
    write_allowed: bool,
    applied: bool,
}

#[derive(Debug, Clone)]
pub struct SelfEvolutionAdmissionReport {
    pub candidate_id: String,
    pub read_only: bool,
    pub report_only: bool,
    pub preview_only: bool,
    pub policy_valid: bool,
    pub mutation_write_allowed: bool,
    pub memory_store_write_allowed: bool,
    pub ndkv_write_allowed: bool,
    pub model_weight_write_allowed: bool,
    pub git_write_allowed: bool,
    pub human_approval_required: bool,
    pub admitted_for_human_review: bool,
    pub rust_check_items: u64,
    pub rust_check_passed: u64,
    pub rust_check_failed: u64,
    pub rust_validation_passed: bool,
    pub validation: SelfEvolutionValidationEvidence,
    pub validation_passed: bool,
    pub benchmark_gate_passed: bool,
    pub benchmark_gate_failures: Vec<String>,
    pub rollback_budget_clean: bool,
    pub drift_rollbacks: u64,
    pub rollback_router_threshold_delta: f32,
    pub rollback_hierarchy_weight_delta: f32,
    pub adaptive_preview_evidence_present: bool,
    pub router_threshold_preview_ready: bool,
    pub hierarchy_adjustment_preview_ready: bool,
    pub kv_fusion_policy_observation_preview_ready: bool,
    pub adaptive_preview_source_count: usize,
    pub adaptive_preview_read_only: bool,
    pub adaptive_preview_report_only: bool,
    pub adaptive_preview_preview_only: bool,
    pub adaptive_preview_write_allowed: bool,
    pub adaptive_preview_applied: bool,
    pub adaptive_preview_blocked_reasons: Vec<String>,
    pub review_packet: SelfEvolutionAdmissionReviewPacketRefs,
    pub blocked_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SelfEvolutionAdmissionGate {
    pub policy: SelfEvolutionAdmissionPolicy,
}

impl Default for SelfEvolutionAdmissionGate {
    fn default() -> Self {
        Self {
            policy: SelfEvolutionAdmissionPolicy::default(),
        }
    }
}

impl SelfEvolutionAdmissionGate {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_policy(mut self, policy: SelfEvolutionAdmissionPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn evaluate(
        &self,
        evidence: &SelfEvolutionAdmissionEvidence,
    ) -> SelfEvolutionAdmissionReport {
        let ledger = evidence.evolution_ledger;
        let mut blocked_reasons = Vec::new();
        let mut policy_valid = true;

        if evidence.candidate_id.trim().is_empty() {
            blocked_reasons.push("self_evolution_admission_candidate_id_empty".to_owned());
        }
        let max_rollback_router_threshold_delta =
            match normalized_rollback_delta(self.policy.max_rollback_router_threshold_delta) {
                Some(delta) => delta,
                None => {
                    policy_valid = false;
                    blocked_reasons.push(
                        "self_evolution_admission_max_rollback_router_threshold_delta_invalid"
                            .to_owned(),
                    );
                    0.0
                }
            };
        let max_rollback_hierarchy_weight_delta =
            match normalized_rollback_delta(self.policy.max_rollback_hierarchy_weight_delta) {
                Some(delta) => delta,
                None => {
                    policy_valid = false;
                    blocked_reasons.push(
                        "self_evolution_admission_max_rollback_hierarchy_weight_delta_invalid"
                            .to_owned(),
                    );
                    0.0
                }
            };

        let rust_check_items = ledger.replay_rust_check_items;
        let rust_check_passed = ledger.replay_rust_check_passed;
        let rust_check_failed = ledger.replay_rust_check_failed;
        let rust_validation_passed = rust_check_items >= self.policy.min_rust_check_items
            && rust_check_passed >= self.policy.min_rust_check_items
            && (!self.policy.require_all_rust_checks_passed || rust_check_failed == 0);
        let validation = evidence.validation;
        let validation_passed = validation.compiler.passed_at_least(
            self.policy.min_compiler_validation_items,
            self.policy.require_all_validation_lanes_passed,
        ) && validation.tests.passed_at_least(
            self.policy.min_test_validation_items,
            self.policy.require_all_validation_lanes_passed,
        ) && validation.benchmarks.passed_at_least(
            self.policy.min_benchmark_validation_items,
            self.policy.require_all_validation_lanes_passed,
        ) && validation.experiments.passed_at_least(
            self.policy.min_experiment_validation_items,
            self.policy.require_all_validation_lanes_passed,
        );

        if rust_check_items < self.policy.min_rust_check_items {
            blocked_reasons.push(format!(
                "self_evolution_admission_rust_check_items={}<{}",
                rust_check_items, self.policy.min_rust_check_items
            ));
        }
        if rust_check_passed < self.policy.min_rust_check_items {
            blocked_reasons.push(format!(
                "self_evolution_admission_rust_check_passed={}<{}",
                rust_check_passed, self.policy.min_rust_check_items
            ));
        }
        if self.policy.require_all_rust_checks_passed && rust_check_failed > 0 {
            blocked_reasons.push(format!(
                "self_evolution_admission_rust_check_failed={}>0",
                rust_check_failed
            ));
        }
        if self.policy.require_benchmark_gate_passed && !evidence.benchmark_gate_passed {
            blocked_reasons.push("self_evolution_admission_benchmark_gate_failed".to_owned());
        }
        push_validation_lane_blocked_reasons(
            &mut blocked_reasons,
            "compiler",
            validation.compiler,
            self.policy.min_compiler_validation_items,
            self.policy.require_all_validation_lanes_passed,
        );
        push_validation_lane_blocked_reasons(
            &mut blocked_reasons,
            "tests",
            validation.tests,
            self.policy.min_test_validation_items,
            self.policy.require_all_validation_lanes_passed,
        );
        push_validation_lane_blocked_reasons(
            &mut blocked_reasons,
            "benchmarks",
            validation.benchmarks,
            self.policy.min_benchmark_validation_items,
            self.policy.require_all_validation_lanes_passed,
        );
        push_validation_lane_blocked_reasons(
            &mut blocked_reasons,
            "experiments",
            validation.experiments,
            self.policy.min_experiment_validation_items,
            self.policy.require_all_validation_lanes_passed,
        );

        let rollback_budget_clean = rollback_budget_clean(
            ledger,
            self.policy.max_drift_rollbacks,
            max_rollback_router_threshold_delta,
            max_rollback_hierarchy_weight_delta,
        );
        if ledger.drift_rollbacks > self.policy.max_drift_rollbacks {
            blocked_reasons.push(format!(
                "self_evolution_admission_drift_rollbacks={}>{}",
                ledger.drift_rollbacks, self.policy.max_drift_rollbacks
            ));
        }
        if ledger.rollback_router_threshold_delta > max_rollback_router_threshold_delta {
            blocked_reasons.push(format!(
                "self_evolution_admission_rollback_router_threshold_delta={:.6}>{:.6}",
                ledger.rollback_router_threshold_delta, max_rollback_router_threshold_delta
            ));
        }
        if ledger.rollback_hierarchy_weight_delta > max_rollback_hierarchy_weight_delta {
            blocked_reasons.push(format!(
                "self_evolution_admission_rollback_hierarchy_weight_delta={:.6}>{:.6}",
                ledger.rollback_hierarchy_weight_delta, max_rollback_hierarchy_weight_delta
            ));
        }

        let adaptive_preview_evidence_present = evidence.adaptive_preview_evidence_present();
        if self.policy.require_adaptive_preview_evidence && !adaptive_preview_evidence_present {
            blocked_reasons
                .push("self_evolution_admission_adaptive_preview_evidence_missing".to_owned());
        }
        blocked_reasons.extend(evidence.adaptive_preview_blocked_reasons.iter().cloned());

        let admitted_for_human_review = blocked_reasons.is_empty();
        let report = SelfEvolutionAdmissionReport {
            candidate_id: evidence.candidate_id.clone(),
            read_only: true,
            report_only: true,
            preview_only: true,
            policy_valid,
            mutation_write_allowed: false,
            memory_store_write_allowed: false,
            ndkv_write_allowed: false,
            model_weight_write_allowed: false,
            git_write_allowed: false,
            human_approval_required: true,
            admitted_for_human_review,
            rust_check_items,
            rust_check_passed,
            rust_check_failed,
            rust_validation_passed,
            validation,
            validation_passed,
            benchmark_gate_passed: evidence.benchmark_gate_passed,
            benchmark_gate_failures: evidence.benchmark_gate_failures.clone(),
            rollback_budget_clean,
            drift_rollbacks: ledger.drift_rollbacks,
            rollback_router_threshold_delta: ledger.rollback_router_threshold_delta,
            rollback_hierarchy_weight_delta: ledger.rollback_hierarchy_weight_delta,
            adaptive_preview_evidence_present,
            router_threshold_preview_ready: evidence.router_threshold_preview_ready,
            hierarchy_adjustment_preview_ready: evidence.hierarchy_adjustment_preview_ready,
            kv_fusion_policy_observation_preview_ready: evidence
                .kv_fusion_policy_observation_preview_ready,
            adaptive_preview_source_count: evidence.adaptive_preview_source_count,
            adaptive_preview_read_only: evidence.adaptive_preview_read_only,
            adaptive_preview_report_only: evidence.adaptive_preview_report_only,
            adaptive_preview_preview_only: evidence.adaptive_preview_preview_only,
            adaptive_preview_write_allowed: evidence.adaptive_preview_write_allowed,
            adaptive_preview_applied: evidence.adaptive_preview_applied,
            adaptive_preview_blocked_reasons: evidence.adaptive_preview_blocked_reasons.clone(),
            review_packet: evidence.review_packet.clone(),
            blocked_reasons,
            telemetry: Vec::new(),
        };

        report.with_telemetry()
    }
}

impl SelfEvolutionAdmissionReport {
    pub fn summary_line(&self) -> String {
        format!(
            "self_evolution_admission candidate={} read_only={} report_only={} preview_only={} admitted_for_human_review={} human_approval_required={} rust_checks={}/{} rust_failed={} benchmark_gate_passed={} rollback_budget_clean={} adaptive_preview_evidence={} blocked_reasons={}",
            self.candidate_id,
            self.read_only,
            self.report_only,
            self.preview_only,
            self.admitted_for_human_review,
            self.human_approval_required,
            self.rust_check_passed,
            self.rust_check_items,
            self.rust_check_failed,
            self.benchmark_gate_passed,
            self.rollback_budget_clean,
            self.adaptive_preview_evidence_present,
            self.blocked_reasons.len(),
        )
    }

    pub fn json_line(&self) -> String {
        let candidate_id = self_evolution_json_escape(&self.candidate_id);
        let benchmark_gate_failures =
            self_evolution_string_array_json(&self.benchmark_gate_failures);
        let adaptive_preview_blocked_reasons =
            self_evolution_string_array_json(&self.adaptive_preview_blocked_reasons);
        let blocked_reasons = self_evolution_string_array_json(&self.blocked_reasons);
        let telemetry = self_evolution_string_array_json(&self.telemetry);
        let approval_review_packet_ids =
            self_evolution_string_array_json(&self.review_packet.approval_review_packet_ids);
        let evidence_ids = self_evolution_string_array_json(&self.review_packet.evidence_ids);
        let rollback_anchor_ids =
            self_evolution_string_array_json(&self.review_packet.rollback_anchor_ids);
        let content_digests = self_evolution_string_array_json(&self.review_packet.content_digests);
        let source_report_schemas =
            self_evolution_string_array_json(&self.review_packet.source_report_schemas);
        let rollback_router_threshold_delta =
            self_evolution_f32_json(self.rollback_router_threshold_delta);
        let rollback_hierarchy_weight_delta =
            self_evolution_f32_json(self.rollback_hierarchy_weight_delta);

        format!(
            "{{\
             \"schema\":\"rust-norion-self-evolution-admission-v1\",\
             \"candidate_id\":\"{candidate_id}\",\
             \"read_only\":{},\
             \"report_only\":{},\
             \"preview_only\":{},\
             \"policy_valid\":{},\
             \"admitted_for_human_review\":{},\
             \"human_approval_required\":{},\
             \"review_packet\":{{\"approval_review_packet_ids\":{approval_review_packet_ids},\"evidence_ids\":{evidence_ids},\"rollback_anchor_ids\":{rollback_anchor_ids},\"content_digests\":{content_digests},\"source_report_schemas\":{source_report_schemas},\"approval_review_packet_count\":{},\"evidence_count\":{},\"rollback_anchor_count\":{},\"content_digest_count\":{},\"source_report_schema_count\":{},\"read_only\":true,\"approval_tokens_included\":false}},\
             \"rust_check\":{{\"items\":{},\"passed\":{},\"failed\":{},\"validation_passed\":{}}},\
             \"validation\":{{\"passed\":{},\"compiler\":{{\"items\":{},\"passed\":{},\"failed\":{},\"validation_passed\":{}}},\"tests\":{{\"items\":{},\"passed\":{},\"failed\":{},\"validation_passed\":{}}},\"benchmarks\":{{\"items\":{},\"passed\":{},\"failed\":{},\"validation_passed\":{}}},\"experiments\":{{\"items\":{},\"passed\":{},\"failed\":{},\"validation_passed\":{}}}}},\
             \"benchmark_gate\":{{\"passed\":{},\"failures\":{benchmark_gate_failures}}},\
             \"rollback\":{{\"budget_clean\":{},\"drift_rollbacks\":{},\"router_threshold_delta\":{rollback_router_threshold_delta},\"hierarchy_weight_delta\":{rollback_hierarchy_weight_delta}}},\
             \"adaptive_preview\":{{\"evidence_present\":{},\"source_count\":{},\"router_threshold_ready\":{},\"hierarchy_adjustment_ready\":{},\"kv_fusion_policy_observation_ready\":{},\"read_only\":{},\"report_only\":{},\"preview_only\":{},\"write_allowed\":{},\"applied\":{},\"blocked_reasons\":{adaptive_preview_blocked_reasons}}},\
             \"writes\":{{\"mutation_allowed\":{},\"memory_store_allowed\":{},\"ndkv_allowed\":{},\"model_weight_allowed\":{},\"git_allowed\":{}}},\
             \"blocked_reasons\":{blocked_reasons},\
             \"telemetry\":{telemetry}\
             }}",
            self.read_only,
            self.report_only,
            self.preview_only,
            self.policy_valid,
            self.admitted_for_human_review,
            self.human_approval_required,
            self.review_packet.approval_review_packet_ids.len(),
            self.review_packet.evidence_ids.len(),
            self.review_packet.rollback_anchor_ids.len(),
            self.review_packet.content_digests.len(),
            self.review_packet.source_report_schemas.len(),
            self.rust_check_items,
            self.rust_check_passed,
            self.rust_check_failed,
            self.rust_validation_passed,
            self.validation_passed,
            self.validation.compiler.items,
            self.validation.compiler.passed,
            self.validation.compiler.failed,
            self.validation.compiler.passed_at_least(1, true),
            self.validation.tests.items,
            self.validation.tests.passed,
            self.validation.tests.failed,
            self.validation.tests.passed_at_least(1, true),
            self.validation.benchmarks.items,
            self.validation.benchmarks.passed,
            self.validation.benchmarks.failed,
            self.validation.benchmarks.passed_at_least(1, true),
            self.validation.experiments.items,
            self.validation.experiments.passed,
            self.validation.experiments.failed,
            self.validation.experiments.passed_at_least(1, true),
            self.benchmark_gate_passed,
            self.rollback_budget_clean,
            self.drift_rollbacks,
            self.adaptive_preview_evidence_present,
            self.adaptive_preview_source_count,
            self.router_threshold_preview_ready,
            self.hierarchy_adjustment_preview_ready,
            self.kv_fusion_policy_observation_preview_ready,
            self.adaptive_preview_read_only,
            self.adaptive_preview_report_only,
            self.adaptive_preview_preview_only,
            self.adaptive_preview_write_allowed,
            self.adaptive_preview_applied,
            self.mutation_write_allowed,
            self.memory_store_write_allowed,
            self.ndkv_write_allowed,
            self.model_weight_write_allowed,
            self.git_write_allowed,
        )
    }

    fn with_telemetry(mut self) -> Self {
        self.telemetry = self_evolution_admission_telemetry(&self);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelfEvolutionExperimentDecision {
    AdmitForHumanReview,
    Hold,
    Reject,
    Rollback,
}

impl SelfEvolutionExperimentDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AdmitForHumanReview => "admit_for_human_review",
            Self::Hold => "hold",
            Self::Reject => "reject",
            Self::Rollback => "rollback",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfEvolutionExperimentRecord {
    pub sequence: u64,
    pub experiment_id: String,
    pub candidate_id: String,
    pub decision: SelfEvolutionExperimentDecision,
    pub repeated_experiment: bool,
    pub conflicting_evidence: bool,
    pub rollback_required: bool,
    pub rollback_replayable: bool,
    pub human_approval_required: bool,
    pub active_candidate: bool,
    pub read_only: bool,
    pub report_only: bool,
    pub preview_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
    pub evidence_ids: Vec<String>,
    pub rollback_anchor_ids: Vec<String>,
    pub blocked_reasons: Vec<String>,
    pub content_digest: String,
}

impl SelfEvolutionExperimentRecord {
    pub fn summary_line(&self) -> String {
        format!(
            "self_evolution_experiment sequence={} experiment={} candidate={} decision={} repeated={} conflict={} rollback_required={} rollback_replayable={} human_approval_required={} active_candidate={} write_allowed={} applied={} evidence_ids={} rollback_anchors={} blocked_reasons={} digest={}",
            self.sequence,
            self.experiment_id,
            self.candidate_id,
            self.decision.as_str(),
            self.repeated_experiment,
            self.conflicting_evidence,
            self.rollback_required,
            self.rollback_replayable,
            self.human_approval_required,
            self.active_candidate,
            self.write_allowed,
            self.applied,
            self.evidence_ids.len(),
            self.rollback_anchor_ids.len(),
            self.blocked_reasons.len(),
            self.content_digest,
        )
    }

    pub fn json_line(&self) -> String {
        let experiment_id = self_evolution_json_escape(&self.experiment_id);
        let candidate_id = self_evolution_json_escape(&self.candidate_id);
        let content_digest = self_evolution_json_escape(&self.content_digest);
        let evidence_ids = self_evolution_string_array_json(&self.evidence_ids);
        let rollback_anchor_ids = self_evolution_string_array_json(&self.rollback_anchor_ids);
        let blocked_reasons = self_evolution_string_array_json(&self.blocked_reasons);

        format!(
            "{{\
             \"schema\":\"rust-norion-self-evolution-experiment-v1\",\
             \"sequence\":{},\
             \"experiment_id\":\"{experiment_id}\",\
             \"candidate_id\":\"{candidate_id}\",\
             \"decision\":\"{}\",\
             \"repeated_experiment\":{},\
             \"conflicting_evidence\":{},\
             \"rollback_required\":{},\
             \"rollback_replayable\":{},\
             \"human_approval_required\":{},\
             \"active_candidate\":{},\
             \"read_only\":{},\
             \"report_only\":{},\
             \"preview_only\":{},\
             \"write_allowed\":{},\
             \"applied\":{},\
             \"evidence_ids\":{evidence_ids},\
             \"rollback_anchor_ids\":{rollback_anchor_ids},\
             \"blocked_reasons\":{blocked_reasons},\
             \"content_digest\":\"{content_digest}\"\
             }}",
            self.sequence,
            self.decision.as_str(),
            self.repeated_experiment,
            self.conflicting_evidence,
            self.rollback_required,
            self.rollback_replayable,
            self.human_approval_required,
            self.active_candidate,
            self.read_only,
            self.report_only,
            self.preview_only,
            self.write_allowed,
            self.applied,
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SelfEvolutionExperimentLedger {
    records: Vec<SelfEvolutionExperimentRecord>,
}

impl SelfEvolutionExperimentLedger {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn records(&self) -> &[SelfEvolutionExperimentRecord] {
        &self.records
    }

    pub fn append_admission_report(
        &mut self,
        experiment_id: impl Into<String>,
        report: &SelfEvolutionAdmissionReport,
    ) -> SelfEvolutionExperimentRecord {
        let experiment_id = self_evolution_review_id_component(&experiment_id.into());
        let repeated_experiment = self
            .records
            .iter()
            .any(|record| record.experiment_id == experiment_id);
        let conflicting_evidence = self_evolution_experiment_conflicting_evidence(report);
        let decision = self_evolution_experiment_decision(report, conflicting_evidence);
        let write_allowed = report.mutation_write_allowed
            || report.memory_store_write_allowed
            || report.ndkv_write_allowed
            || report.model_weight_write_allowed
            || report.git_write_allowed
            || report.adaptive_preview_write_allowed;
        let applied = report.adaptive_preview_applied;
        let rollback_required = decision == SelfEvolutionExperimentDecision::Rollback;
        let rollback_replayable =
            rollback_required && !report.review_packet.rollback_anchor_ids.is_empty();
        let sequence = self.records.len() as u64 + 1;
        let content_digest = self_evolution_stable_digest(&format!(
            "sequence={sequence};experiment={experiment_id};candidate={};decision={};repeated={repeated_experiment};conflict={conflicting_evidence};rollback={rollback_required};blocked={:?};evidence={:?};anchors={:?}",
            report.candidate_id,
            decision.as_str(),
            report.blocked_reasons,
            report.review_packet.evidence_ids,
            report.review_packet.rollback_anchor_ids
        ));
        let record = SelfEvolutionExperimentRecord {
            sequence,
            experiment_id,
            candidate_id: report.candidate_id.clone(),
            decision,
            repeated_experiment,
            conflicting_evidence,
            rollback_required,
            rollback_replayable,
            human_approval_required: true,
            active_candidate: false,
            read_only: report.read_only,
            report_only: report.report_only,
            preview_only: report.preview_only,
            write_allowed,
            applied,
            evidence_ids: report.review_packet.evidence_ids.clone(),
            rollback_anchor_ids: report.review_packet.rollback_anchor_ids.clone(),
            blocked_reasons: report.blocked_reasons.clone(),
            content_digest,
        };
        self.records.push(record.clone());
        record
    }

    pub fn admitted_for_review(&self) -> usize {
        self.count_decision(SelfEvolutionExperimentDecision::AdmitForHumanReview)
    }

    pub fn held(&self) -> usize {
        self.count_decision(SelfEvolutionExperimentDecision::Hold)
    }

    pub fn rejected(&self) -> usize {
        self.count_decision(SelfEvolutionExperimentDecision::Reject)
    }

    pub fn rollback_required(&self) -> usize {
        self.count_decision(SelfEvolutionExperimentDecision::Rollback)
    }

    pub fn repeated_experiments(&self) -> usize {
        self.records
            .iter()
            .filter(|record| record.repeated_experiment)
            .count()
    }

    pub fn conflicting_evidence(&self) -> usize {
        self.records
            .iter()
            .filter(|record| record.conflicting_evidence)
            .count()
    }

    pub fn write_allowed_records(&self) -> usize {
        self.records
            .iter()
            .filter(|record| record.write_allowed)
            .count()
    }

    pub fn applied_records(&self) -> usize {
        self.records.iter().filter(|record| record.applied).count()
    }

    pub fn summary_line(&self) -> String {
        format!(
            "self_evolution_experiment_ledger records={} admitted_for_review={} held={} rejected={} rollback_required={} repeated_experiments={} conflicting_evidence={} active_candidates={} write_allowed_records={} applied_records={}",
            self.records.len(),
            self.admitted_for_review(),
            self.held(),
            self.rejected(),
            self.rollback_required(),
            self.repeated_experiments(),
            self.conflicting_evidence(),
            self.records
                .iter()
                .filter(|record| record.active_candidate)
                .count(),
            self.write_allowed_records(),
            self.applied_records(),
        )
    }

    pub fn rollback_replay_plan(&self) -> SelfEvolutionRollbackReplayPlan {
        SelfEvolutionRollbackReplayPlan::new(
            self.records
                .iter()
                .filter(|record| {
                    record.rollback_required
                        || record.decision == SelfEvolutionExperimentDecision::Rollback
                })
                .map(SelfEvolutionRollbackReplayItem::from_record)
                .collect(),
        )
    }

    fn count_decision(&self, decision: SelfEvolutionExperimentDecision) -> usize {
        self.records
            .iter()
            .filter(|record| record.decision == decision)
            .count()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfEvolutionRollbackReplayItem {
    pub sequence: u64,
    pub experiment_id: String,
    pub candidate_id: String,
    pub decision: SelfEvolutionExperimentDecision,
    pub rollback_required: bool,
    pub rollback_replayable: bool,
    pub replayable: bool,
    pub active_candidate: bool,
    pub read_only: bool,
    pub report_only: bool,
    pub preview_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
    pub evidence_ids: Vec<String>,
    pub rollback_anchor_ids: Vec<String>,
    pub blocked_reasons: Vec<String>,
    pub content_digest: String,
}

impl SelfEvolutionRollbackReplayItem {
    fn from_record(record: &SelfEvolutionExperimentRecord) -> Self {
        let mut blocked_reasons = Vec::new();

        if record.decision != SelfEvolutionExperimentDecision::Rollback {
            blocked_reasons.push("self_evolution_rollback_replay_decision_not_rollback".to_owned());
        }
        if !record.rollback_required {
            blocked_reasons.push("self_evolution_rollback_replay_rollback_not_required".to_owned());
        }
        if !record.rollback_replayable {
            blocked_reasons.push("self_evolution_rollback_replay_record_not_replayable".to_owned());
        }
        if record.evidence_ids.is_empty() {
            blocked_reasons.push("self_evolution_rollback_replay_evidence_missing".to_owned());
        }
        if record.rollback_anchor_ids.is_empty() {
            blocked_reasons.push("self_evolution_rollback_replay_anchor_missing".to_owned());
        }
        if record.active_candidate {
            blocked_reasons.push("self_evolution_rollback_replay_active_candidate".to_owned());
        }
        if !record.read_only {
            blocked_reasons.push("self_evolution_rollback_replay_not_read_only".to_owned());
        }
        if !record.report_only {
            blocked_reasons.push("self_evolution_rollback_replay_not_report_only".to_owned());
        }
        if !record.preview_only {
            blocked_reasons.push("self_evolution_rollback_replay_not_preview_only".to_owned());
        }
        if record.write_allowed {
            blocked_reasons.push("self_evolution_rollback_replay_write_allowed".to_owned());
        }
        if record.applied {
            blocked_reasons.push("self_evolution_rollback_replay_already_applied".to_owned());
        }

        Self {
            sequence: record.sequence,
            experiment_id: record.experiment_id.clone(),
            candidate_id: record.candidate_id.clone(),
            decision: record.decision,
            rollback_required: record.rollback_required,
            rollback_replayable: record.rollback_replayable,
            replayable: blocked_reasons.is_empty(),
            active_candidate: record.active_candidate,
            read_only: record.read_only,
            report_only: record.report_only,
            preview_only: record.preview_only,
            write_allowed: record.write_allowed,
            applied: record.applied,
            evidence_ids: record.evidence_ids.clone(),
            rollback_anchor_ids: record.rollback_anchor_ids.clone(),
            blocked_reasons,
            content_digest: record.content_digest.clone(),
        }
    }

    pub fn summary_line(&self) -> String {
        format!(
            "self_evolution_rollback_replay_item sequence={} experiment={} candidate={} decision={} replayable={} rollback_required={} rollback_replayable={} active_candidate={} write_allowed={} applied={} evidence_ids={} rollback_anchors={} blocked_reasons={} digest={}",
            self.sequence,
            self.experiment_id,
            self.candidate_id,
            self.decision.as_str(),
            self.replayable,
            self.rollback_required,
            self.rollback_replayable,
            self.active_candidate,
            self.write_allowed,
            self.applied,
            self.evidence_ids.len(),
            self.rollback_anchor_ids.len(),
            self.blocked_reasons.len(),
            self.content_digest,
        )
    }

    pub fn json_line(&self) -> String {
        format!(
            "{{\"schema\":\"rust-norion-self-evolution-rollback-replay-item-v1\",{}}}",
            self.json_fields()
        )
    }

    fn json_fields(&self) -> String {
        let experiment_id = self_evolution_json_escape(&self.experiment_id);
        let candidate_id = self_evolution_json_escape(&self.candidate_id);
        let content_digest = self_evolution_json_escape(&self.content_digest);
        let evidence_ids = self_evolution_string_array_json(&self.evidence_ids);
        let rollback_anchor_ids = self_evolution_string_array_json(&self.rollback_anchor_ids);
        let blocked_reasons = self_evolution_string_array_json(&self.blocked_reasons);

        format!(
            "\"sequence\":{},\
             \"experiment_id\":\"{experiment_id}\",\
             \"candidate_id\":\"{candidate_id}\",\
             \"decision\":\"{}\",\
             \"rollback_required\":{},\
             \"rollback_replayable\":{},\
             \"replayable\":{},\
             \"active_candidate\":{},\
             \"read_only\":{},\
             \"report_only\":{},\
             \"preview_only\":{},\
             \"write_allowed\":{},\
             \"applied\":{},\
             \"evidence_ids\":{evidence_ids},\
             \"rollback_anchor_ids\":{rollback_anchor_ids},\
             \"blocked_reasons\":{blocked_reasons},\
             \"content_digest\":\"{content_digest}\"",
            self.sequence,
            self.decision.as_str(),
            self.rollback_required,
            self.rollback_replayable,
            self.replayable,
            self.active_candidate,
            self.read_only,
            self.report_only,
            self.preview_only,
            self.write_allowed,
            self.applied,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfEvolutionRollbackReplayPlan {
    pub items: Vec<SelfEvolutionRollbackReplayItem>,
    pub read_only: bool,
    pub report_only: bool,
    pub preview_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl SelfEvolutionRollbackReplayPlan {
    pub fn new(items: Vec<SelfEvolutionRollbackReplayItem>) -> Self {
        Self {
            items,
            read_only: true,
            report_only: true,
            preview_only: true,
            write_allowed: false,
            applied: false,
        }
    }

    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    pub fn replayable(&self) -> usize {
        self.items.iter().filter(|item| item.replayable).count()
    }

    pub fn blocked(&self) -> usize {
        self.items.iter().filter(|item| !item.replayable).count()
    }

    pub fn all_replayable(&self) -> bool {
        self.blocked() == 0
    }

    pub fn active_candidates(&self) -> usize {
        self.items
            .iter()
            .filter(|item| item.active_candidate)
            .count()
    }

    pub fn write_allowed_items(&self) -> usize {
        self.items.iter().filter(|item| item.write_allowed).count()
    }

    pub fn applied_items(&self) -> usize {
        self.items.iter().filter(|item| item.applied).count()
    }

    pub fn rollback_anchor_ids(&self) -> Vec<String> {
        let mut ids = Vec::new();
        for item in &self.items {
            for anchor_id in &item.rollback_anchor_ids {
                push_unique_string(&mut ids, anchor_id.clone());
            }
        }
        ids
    }

    pub fn evidence_ids(&self) -> Vec<String> {
        let mut ids = Vec::new();
        for item in &self.items {
            for evidence_id in &item.evidence_ids {
                push_unique_string(&mut ids, evidence_id.clone());
            }
        }
        ids
    }

    pub fn summary_line(&self) -> String {
        format!(
            "self_evolution_rollback_replay_plan items={} replayable={} blocked={} active_candidates={} item_write_allowed={} item_applied={} rollback_anchors={} evidence_ids={} read_only={} report_only={} preview_only={} write_allowed={} applied={}",
            self.item_count(),
            self.replayable(),
            self.blocked(),
            self.active_candidates(),
            self.write_allowed_items(),
            self.applied_items(),
            self.rollback_anchor_ids().len(),
            self.evidence_ids().len(),
            self.read_only,
            self.report_only,
            self.preview_only,
            self.write_allowed,
            self.applied,
        )
    }

    pub fn json_line(&self) -> String {
        let items = self
            .items
            .iter()
            .map(|item| format!("{{{}}}", item.json_fields()))
            .collect::<Vec<_>>()
            .join(",");
        let rollback_anchor_ids = self_evolution_string_array_json(&self.rollback_anchor_ids());
        let evidence_ids = self_evolution_string_array_json(&self.evidence_ids());

        format!(
            "{{\
             \"schema\":\"rust-norion-self-evolution-rollback-replay-plan-v1\",\
             \"item_count\":{},\
             \"replayable\":{},\
             \"blocked\":{},\
             \"all_replayable\":{},\
             \"active_candidates\":{},\
             \"item_write_allowed\":{},\
             \"item_applied\":{},\
             \"read_only\":{},\
             \"report_only\":{},\
             \"preview_only\":{},\
             \"write_allowed\":{},\
             \"applied\":{},\
             \"rollback_anchor_ids\":{rollback_anchor_ids},\
             \"evidence_ids\":{evidence_ids},\
             \"items\":[{items}]\
             }}",
            self.item_count(),
            self.replayable(),
            self.blocked(),
            self.all_replayable(),
            self.active_candidates(),
            self.write_allowed_items(),
            self.applied_items(),
            self.read_only,
            self.report_only,
            self.preview_only,
            self.write_allowed,
            self.applied,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelfEvolutionRollbackReplayDecision {
    AdmitForHumanReview,
    Hold,
}

impl SelfEvolutionRollbackReplayDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AdmitForHumanReview => "admit_for_human_review",
            Self::Hold => "hold",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelfEvolutionRollbackReplayPolicy {
    pub require_non_empty_plan: bool,
    pub require_all_items_replayable: bool,
    pub require_rollback_anchor_ids: bool,
    pub require_evidence_ids: bool,
}

impl Default for SelfEvolutionRollbackReplayPolicy {
    fn default() -> Self {
        Self {
            require_non_empty_plan: true,
            require_all_items_replayable: true,
            require_rollback_anchor_ids: true,
            require_evidence_ids: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SelfEvolutionRollbackReplayGate {
    pub policy: SelfEvolutionRollbackReplayPolicy,
}

impl Default for SelfEvolutionRollbackReplayGate {
    fn default() -> Self {
        Self {
            policy: SelfEvolutionRollbackReplayPolicy::default(),
        }
    }
}

impl SelfEvolutionRollbackReplayGate {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_policy(mut self, policy: SelfEvolutionRollbackReplayPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn evaluate(
        &self,
        plan: &SelfEvolutionRollbackReplayPlan,
    ) -> SelfEvolutionRollbackReplayGateReport {
        let item_count = plan.item_count();
        let replayable = plan.replayable();
        let blocked = plan.blocked();
        let all_replayable = plan.all_replayable();
        let rollback_anchor_ids = plan.rollback_anchor_ids();
        let evidence_ids = plan.evidence_ids();
        let active_candidates = plan.active_candidates();
        let item_write_allowed = plan.write_allowed_items();
        let item_applied = plan.applied_items();
        let mut blocked_reasons = Vec::new();

        if self.policy.require_non_empty_plan && item_count == 0 {
            blocked_reasons.push("self_evolution_rollback_replay_gate_empty_plan".to_owned());
        }
        if self.policy.require_all_items_replayable && (!all_replayable || blocked > 0) {
            blocked_reasons.push("self_evolution_rollback_replay_gate_blocked_items".to_owned());
        }
        if self.policy.require_rollback_anchor_ids
            && item_count > 0
            && rollback_anchor_ids.is_empty()
        {
            blocked_reasons
                .push("self_evolution_rollback_replay_gate_rollback_anchor_ids_missing".to_owned());
        }
        if self.policy.require_evidence_ids && item_count > 0 && evidence_ids.is_empty() {
            blocked_reasons
                .push("self_evolution_rollback_replay_gate_evidence_ids_missing".to_owned());
        }
        if active_candidates > 0 {
            blocked_reasons.push(format!(
                "self_evolution_rollback_replay_gate_active_candidates={active_candidates}>0"
            ));
        }
        if item_write_allowed > 0 {
            blocked_reasons.push(format!(
                "self_evolution_rollback_replay_gate_item_write_allowed={item_write_allowed}>0"
            ));
        }
        if item_applied > 0 {
            blocked_reasons.push(format!(
                "self_evolution_rollback_replay_gate_item_applied={item_applied}>0"
            ));
        }
        if !plan.read_only {
            blocked_reasons
                .push("self_evolution_rollback_replay_gate_plan_not_read_only".to_owned());
        }
        if !plan.report_only {
            blocked_reasons
                .push("self_evolution_rollback_replay_gate_plan_not_report_only".to_owned());
        }
        if !plan.preview_only {
            blocked_reasons
                .push("self_evolution_rollback_replay_gate_plan_not_preview_only".to_owned());
        }
        if plan.write_allowed {
            blocked_reasons
                .push("self_evolution_rollback_replay_gate_plan_write_allowed".to_owned());
        }
        if plan.applied {
            blocked_reasons.push("self_evolution_rollback_replay_gate_plan_applied".to_owned());
        }

        let admitted_for_human_review = blocked_reasons.is_empty();
        let decision = if admitted_for_human_review {
            SelfEvolutionRollbackReplayDecision::AdmitForHumanReview
        } else {
            SelfEvolutionRollbackReplayDecision::Hold
        };
        let item_digests = plan
            .items
            .iter()
            .map(|item| item.content_digest.as_str())
            .collect::<Vec<_>>()
            .join(",");
        let content_digest = self_evolution_stable_digest(&format!(
            "rollback_replay_gate;items={item_count};replayable={replayable};blocked={blocked};all_replayable={all_replayable};anchors={rollback_anchor_ids:?};evidence={evidence_ids:?};digests={item_digests}"
        ));
        let review_packet =
            SelfEvolutionAdmissionReviewPacketRefs::rollback_replay_gate(plan, &content_digest);

        SelfEvolutionRollbackReplayGateReport {
            decision,
            item_count,
            replayable,
            blocked,
            all_replayable,
            active_candidates,
            item_write_allowed,
            item_applied,
            rollback_anchor_ids,
            evidence_ids,
            read_only: true,
            report_only: true,
            preview_only: true,
            write_allowed: false,
            applied: false,
            plan_read_only: plan.read_only,
            plan_report_only: plan.report_only,
            plan_preview_only: plan.preview_only,
            plan_write_allowed: plan.write_allowed,
            plan_applied: plan.applied,
            human_approval_required: true,
            admitted_for_human_review,
            review_packet,
            blocked_reasons,
            content_digest,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfEvolutionRollbackReplayGateReport {
    pub decision: SelfEvolutionRollbackReplayDecision,
    pub item_count: usize,
    pub replayable: usize,
    pub blocked: usize,
    pub all_replayable: bool,
    pub active_candidates: usize,
    pub item_write_allowed: usize,
    pub item_applied: usize,
    pub rollback_anchor_ids: Vec<String>,
    pub evidence_ids: Vec<String>,
    pub read_only: bool,
    pub report_only: bool,
    pub preview_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
    pub plan_read_only: bool,
    pub plan_report_only: bool,
    pub plan_preview_only: bool,
    pub plan_write_allowed: bool,
    pub plan_applied: bool,
    pub human_approval_required: bool,
    pub admitted_for_human_review: bool,
    pub review_packet: SelfEvolutionAdmissionReviewPacketRefs,
    pub blocked_reasons: Vec<String>,
    pub content_digest: String,
}

impl SelfEvolutionRollbackReplayGateReport {
    pub fn summary_line(&self) -> String {
        format!(
            "self_evolution_rollback_replay_gate decision={} admitted_for_human_review={} human_approval_required={} review_packets={} review_evidence_ids={} items={} replayable={} blocked={} all_replayable={} active_candidates={} item_write_allowed={} item_applied={} rollback_anchors={} evidence_ids={} read_only={} report_only={} preview_only={} write_allowed={} applied={} plan_read_only={} plan_report_only={} plan_preview_only={} plan_write_allowed={} plan_applied={} blocked_reasons={} digest={}",
            self.decision.as_str(),
            self.admitted_for_human_review,
            self.human_approval_required,
            self.review_packet.approval_review_packet_ids.len(),
            self.review_packet.evidence_ids.len(),
            self.item_count,
            self.replayable,
            self.blocked,
            self.all_replayable,
            self.active_candidates,
            self.item_write_allowed,
            self.item_applied,
            self.rollback_anchor_ids.len(),
            self.evidence_ids.len(),
            self.read_only,
            self.report_only,
            self.preview_only,
            self.write_allowed,
            self.applied,
            self.plan_read_only,
            self.plan_report_only,
            self.plan_preview_only,
            self.plan_write_allowed,
            self.plan_applied,
            self.blocked_reasons.len(),
            self.content_digest,
        )
    }

    pub fn json_line(&self) -> String {
        let rollback_anchor_ids = self_evolution_string_array_json(&self.rollback_anchor_ids);
        let evidence_ids = self_evolution_string_array_json(&self.evidence_ids);
        let blocked_reasons = self_evolution_string_array_json(&self.blocked_reasons);
        let content_digest = self_evolution_json_escape(&self.content_digest);
        let approval_review_packet_ids =
            self_evolution_string_array_json(&self.review_packet.approval_review_packet_ids);
        let review_evidence_ids =
            self_evolution_string_array_json(&self.review_packet.evidence_ids);
        let review_rollback_anchor_ids =
            self_evolution_string_array_json(&self.review_packet.rollback_anchor_ids);
        let review_content_digests =
            self_evolution_string_array_json(&self.review_packet.content_digests);
        let review_source_report_schemas =
            self_evolution_string_array_json(&self.review_packet.source_report_schemas);

        format!(
            "{{\
             \"schema\":\"rust-norion-self-evolution-rollback-replay-gate-v1\",\
             \"decision\":\"{}\",\
             \"admitted_for_human_review\":{},\
             \"human_approval_required\":{},\
             \"item_count\":{},\
             \"replayable\":{},\
             \"blocked\":{},\
             \"all_replayable\":{},\
             \"active_candidates\":{},\
             \"item_write_allowed\":{},\
             \"item_applied\":{},\
             \"read_only\":{},\
             \"report_only\":{},\
             \"preview_only\":{},\
             \"write_allowed\":{},\
             \"applied\":{},\
             \"plan_read_only\":{},\
             \"plan_report_only\":{},\
             \"plan_preview_only\":{},\
             \"plan_write_allowed\":{},\
             \"plan_applied\":{},\
             \"rollback_anchor_ids\":{rollback_anchor_ids},\
             \"evidence_ids\":{evidence_ids},\
             \"blocked_reasons\":{blocked_reasons},\
             \"review_packet\":{{\"approval_review_packet_ids\":{approval_review_packet_ids},\"evidence_ids\":{review_evidence_ids},\"rollback_anchor_ids\":{review_rollback_anchor_ids},\"content_digests\":{review_content_digests},\"source_report_schemas\":{review_source_report_schemas},\"approval_review_packet_count\":{},\"evidence_count\":{},\"rollback_anchor_count\":{},\"content_digest_count\":{},\"source_report_schema_count\":{},\"read_only\":true,\"approval_tokens_included\":false}},\
             \"content_digest\":\"{content_digest}\"\
             }}",
            self.decision.as_str(),
            self.admitted_for_human_review,
            self.human_approval_required,
            self.item_count,
            self.replayable,
            self.blocked,
            self.all_replayable,
            self.active_candidates,
            self.item_write_allowed,
            self.item_applied,
            self.read_only,
            self.report_only,
            self.preview_only,
            self.write_allowed,
            self.applied,
            self.plan_read_only,
            self.plan_report_only,
            self.plan_preview_only,
            self.plan_write_allowed,
            self.plan_applied,
            self.review_packet.approval_review_packet_ids.len(),
            self.review_packet.evidence_ids.len(),
            self.review_packet.rollback_anchor_ids.len(),
            self.review_packet.content_digests.len(),
            self.review_packet.source_report_schemas.len(),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfEvolutionOperatorApprovalEvidence {
    pub operator_id: String,
    pub approval_ticket_id: String,
    pub approved_review_packet_ids: Vec<String>,
    pub approved_evidence_ids: Vec<String>,
    pub approved_rollback_anchor_ids: Vec<String>,
    pub approved_content_digests: Vec<String>,
    pub approved_source_report_schemas: Vec<String>,
    pub approval_reason: String,
    pub approval_attestation_digest: String,
}

impl SelfEvolutionOperatorApprovalEvidence {
    pub fn from_review_packet(
        operator_id: impl Into<String>,
        approval_ticket_id: impl Into<String>,
        review_packet: &SelfEvolutionAdmissionReviewPacketRefs,
        approval_reason: impl Into<String>,
    ) -> Self {
        let operator_id = operator_id.into();
        let approval_ticket_id = approval_ticket_id.into();
        let approval_reason = approval_reason.into();
        let approval_attestation_digest = self_evolution_operator_approval_attestation_digest(
            &operator_id,
            &approval_ticket_id,
            &review_packet.approval_review_packet_ids,
            &review_packet.evidence_ids,
            &review_packet.rollback_anchor_ids,
            &review_packet.content_digests,
            &review_packet.source_report_schemas,
            &approval_reason,
        );

        Self {
            operator_id,
            approval_ticket_id,
            approved_review_packet_ids: review_packet.approval_review_packet_ids.clone(),
            approved_evidence_ids: review_packet.evidence_ids.clone(),
            approved_rollback_anchor_ids: review_packet.rollback_anchor_ids.clone(),
            approved_content_digests: review_packet.content_digests.clone(),
            approved_source_report_schemas: review_packet.source_report_schemas.clone(),
            approval_reason,
            approval_attestation_digest,
        }
    }

    fn expected_attestation_digest(&self) -> String {
        self_evolution_operator_approval_attestation_digest(
            &self.operator_id,
            &self.approval_ticket_id,
            &self.approved_review_packet_ids,
            &self.approved_evidence_ids,
            &self.approved_rollback_anchor_ids,
            &self.approved_content_digests,
            &self.approved_source_report_schemas,
            &self.approval_reason,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelfEvolutionOperatorApprovalDecision {
    Approved,
    Hold,
}

impl SelfEvolutionOperatorApprovalDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Approved => "approved",
            Self::Hold => "hold",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelfEvolutionOperatorApprovalPolicy {
    pub require_review_packet_ids: bool,
    pub require_evidence_ids: bool,
    pub require_rollback_anchor_ids: bool,
    pub require_content_digests: bool,
    pub require_source_report_schemas: bool,
}

impl Default for SelfEvolutionOperatorApprovalPolicy {
    fn default() -> Self {
        Self {
            require_review_packet_ids: true,
            require_evidence_ids: true,
            require_rollback_anchor_ids: true,
            require_content_digests: true,
            require_source_report_schemas: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SelfEvolutionOperatorApprovalGate {
    pub policy: SelfEvolutionOperatorApprovalPolicy,
}

impl Default for SelfEvolutionOperatorApprovalGate {
    fn default() -> Self {
        Self {
            policy: SelfEvolutionOperatorApprovalPolicy::default(),
        }
    }
}

impl SelfEvolutionOperatorApprovalGate {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_policy(mut self, policy: SelfEvolutionOperatorApprovalPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn evaluate(
        &self,
        review_packet: &SelfEvolutionAdmissionReviewPacketRefs,
        evidence: &SelfEvolutionOperatorApprovalEvidence,
    ) -> SelfEvolutionOperatorApprovalReport {
        self.evaluate_for_scope(&TenantScope::local_single_user(), review_packet, evidence)
    }

    pub fn evaluate_for_scope(
        &self,
        actor_scope: &TenantScope,
        review_packet: &SelfEvolutionAdmissionReviewPacketRefs,
        evidence: &SelfEvolutionOperatorApprovalEvidence,
    ) -> SelfEvolutionOperatorApprovalReport {
        let mut blocked_reasons = Vec::new();

        if evidence.operator_id.trim().is_empty() {
            blocked_reasons.push("self_evolution_operator_approval_operator_id_empty".to_owned());
        }
        if evidence.approval_ticket_id.trim().is_empty() {
            blocked_reasons.push("self_evolution_operator_approval_ticket_id_empty".to_owned());
        }
        if evidence.approval_reason.trim().is_empty() {
            blocked_reasons.push("self_evolution_operator_approval_reason_empty".to_owned());
        }
        let expected_attestation_digest = evidence.expected_attestation_digest();
        if !evidence.approval_attestation_digest.starts_with("fnv64:") {
            blocked_reasons
                .push("self_evolution_operator_approval_attestation_digest_invalid".to_owned());
        } else if evidence.approval_attestation_digest != expected_attestation_digest {
            blocked_reasons
                .push("self_evolution_operator_approval_attestation_digest_mismatch".to_owned());
        }

        push_operator_approval_presence_reason(
            &mut blocked_reasons,
            self.policy.require_review_packet_ids,
            "review_packet_ids",
            &review_packet.approval_review_packet_ids,
            &evidence.approved_review_packet_ids,
        );
        push_operator_approval_presence_reason(
            &mut blocked_reasons,
            self.policy.require_evidence_ids,
            "evidence_ids",
            &review_packet.evidence_ids,
            &evidence.approved_evidence_ids,
        );
        push_operator_approval_presence_reason(
            &mut blocked_reasons,
            self.policy.require_rollback_anchor_ids,
            "rollback_anchor_ids",
            &review_packet.rollback_anchor_ids,
            &evidence.approved_rollback_anchor_ids,
        );
        push_operator_approval_presence_reason(
            &mut blocked_reasons,
            self.policy.require_content_digests,
            "content_digests",
            &review_packet.content_digests,
            &evidence.approved_content_digests,
        );
        push_operator_approval_presence_reason(
            &mut blocked_reasons,
            self.policy.require_source_report_schemas,
            "source_report_schemas",
            &review_packet.source_report_schemas,
            &evidence.approved_source_report_schemas,
        );
        push_operator_approval_packet_scope_reason(
            &mut blocked_reasons,
            "review_packet_ids",
            actor_scope,
            &review_packet.approval_review_packet_ids,
        );
        push_operator_approval_packet_scope_reason(
            &mut blocked_reasons,
            "approved_review_packet_ids",
            actor_scope,
            &evidence.approved_review_packet_ids,
        );

        push_operator_approval_ref_mismatch(
            &mut blocked_reasons,
            self.policy.require_review_packet_ids,
            "review_packet_ids",
            &review_packet.approval_review_packet_ids,
            &evidence.approved_review_packet_ids,
        );
        push_operator_approval_ref_mismatch(
            &mut blocked_reasons,
            self.policy.require_evidence_ids,
            "evidence_ids",
            &review_packet.evidence_ids,
            &evidence.approved_evidence_ids,
        );
        push_operator_approval_ref_mismatch(
            &mut blocked_reasons,
            self.policy.require_rollback_anchor_ids,
            "rollback_anchor_ids",
            &review_packet.rollback_anchor_ids,
            &evidence.approved_rollback_anchor_ids,
        );
        push_operator_approval_ref_mismatch(
            &mut blocked_reasons,
            self.policy.require_content_digests,
            "content_digests",
            &review_packet.content_digests,
            &evidence.approved_content_digests,
        );
        push_operator_approval_ref_mismatch(
            &mut blocked_reasons,
            self.policy.require_source_report_schemas,
            "source_report_schemas",
            &review_packet.source_report_schemas,
            &evidence.approved_source_report_schemas,
        );

        let operator_approved = blocked_reasons.is_empty();
        let decision = if operator_approved {
            SelfEvolutionOperatorApprovalDecision::Approved
        } else {
            SelfEvolutionOperatorApprovalDecision::Hold
        };
        let content_digest = self_evolution_stable_digest(&format!(
            "operator_approval;operator={};ticket={};decision={};review_packets={:?};content_digests={:?};attestation={}",
            evidence.operator_id,
            evidence.approval_ticket_id,
            decision.as_str(),
            evidence.approved_review_packet_ids,
            evidence.approved_content_digests,
            evidence.approval_attestation_digest,
        ));

        SelfEvolutionOperatorApprovalReport {
            decision,
            operator_approved,
            operator_id: evidence.operator_id.clone(),
            approval_ticket_id: evidence.approval_ticket_id.clone(),
            approved_review_packet_ids: evidence.approved_review_packet_ids.clone(),
            approved_evidence_ids: evidence.approved_evidence_ids.clone(),
            approved_rollback_anchor_ids: evidence.approved_rollback_anchor_ids.clone(),
            approved_content_digests: evidence.approved_content_digests.clone(),
            approved_source_report_schemas: evidence.approved_source_report_schemas.clone(),
            approval_reason: evidence.approval_reason.clone(),
            approval_attestation_digest: evidence.approval_attestation_digest.clone(),
            read_only: true,
            report_only: true,
            preview_only: true,
            activation_write_allowed: false,
            active_candidate: false,
            write_allowed: false,
            applied: false,
            blocked_reasons,
            content_digest,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfEvolutionOperatorApprovalReport {
    pub decision: SelfEvolutionOperatorApprovalDecision,
    pub operator_approved: bool,
    pub operator_id: String,
    pub approval_ticket_id: String,
    pub approved_review_packet_ids: Vec<String>,
    pub approved_evidence_ids: Vec<String>,
    pub approved_rollback_anchor_ids: Vec<String>,
    pub approved_content_digests: Vec<String>,
    pub approved_source_report_schemas: Vec<String>,
    pub approval_reason: String,
    pub approval_attestation_digest: String,
    pub read_only: bool,
    pub report_only: bool,
    pub preview_only: bool,
    pub activation_write_allowed: bool,
    pub active_candidate: bool,
    pub write_allowed: bool,
    pub applied: bool,
    pub blocked_reasons: Vec<String>,
    pub content_digest: String,
}

impl SelfEvolutionOperatorApprovalReport {
    pub fn summary_line(&self) -> String {
        format!(
            "self_evolution_operator_approval decision={} operator_approved={} operator_digest={} ticket_digest={} review_packets={} evidence_ids={} rollback_anchors={} content_digests={} schemas={} approved_refs_digest={} approval_reason_digest={} read_only={} report_only={} preview_only={} activation_write_allowed={} active_candidate={} write_allowed={} applied={} blocked_reasons={} blocked_reasons_digest={} digest={}",
            self.decision.as_str(),
            self.operator_approved,
            self.operator_digest(),
            self.approval_ticket_digest(),
            self.approved_review_packet_ids.len(),
            self.approved_evidence_ids.len(),
            self.approved_rollback_anchor_ids.len(),
            self.approved_content_digests.len(),
            self.approved_source_report_schemas.len(),
            self.approved_refs_digest(),
            self.approval_reason_digest(),
            self.read_only,
            self.report_only,
            self.preview_only,
            self.activation_write_allowed,
            self.active_candidate,
            self.write_allowed,
            self.applied,
            self.blocked_reasons.len(),
            self.blocked_reasons_digest(),
            self.content_digest,
        )
    }

    pub fn json_line(&self) -> String {
        let operator_digest = self_evolution_json_escape(&self.operator_digest());
        let approval_ticket_digest = self_evolution_json_escape(&self.approval_ticket_digest());
        let approved_refs_digest = self_evolution_json_escape(&self.approved_refs_digest());
        let approval_reason_digest = self_evolution_json_escape(&self.approval_reason_digest());
        let approval_attestation_digest =
            self_evolution_json_escape(&self.approval_attestation_digest);
        let blocked_reasons_digest = self_evolution_json_escape(&self.blocked_reasons_digest());
        let content_digest = self_evolution_json_escape(&self.content_digest);

        format!(
            "{{\
             \"schema\":\"rust-norion-self-evolution-operator-approval-v1\",\
             \"decision\":\"{}\",\
             \"operator_approved\":{},\
             \"operator_digest\":\"{operator_digest}\",\
             \"approval_ticket_digest\":\"{approval_ticket_digest}\",\
             \"approved_review_packet_count\":{},\
             \"approved_evidence_count\":{},\
             \"approved_rollback_anchor_count\":{},\
             \"approved_content_digest_count\":{},\
             \"approved_source_report_schema_count\":{},\
             \"approved_refs_digest\":\"{approved_refs_digest}\",\
             \"approval_reason_digest\":\"{approval_reason_digest}\",\
             \"approval_attestation_digest\":\"{approval_attestation_digest}\",\
             \"read_only\":{},\
             \"report_only\":{},\
             \"preview_only\":{},\
             \"activation_write_allowed\":{},\
             \"active_candidate\":{},\
             \"write_allowed\":{},\
             \"applied\":{},\
             \"blocked_reasons_count\":{},\
             \"blocked_reasons_digest\":\"{blocked_reasons_digest}\",\
             \"content_digest\":\"{content_digest}\"\
             }}",
            self.decision.as_str(),
            self.operator_approved,
            self.approved_review_packet_ids.len(),
            self.approved_evidence_ids.len(),
            self.approved_rollback_anchor_ids.len(),
            self.approved_content_digests.len(),
            self.approved_source_report_schemas.len(),
            self.read_only,
            self.report_only,
            self.preview_only,
            self.activation_write_allowed,
            self.active_candidate,
            self.write_allowed,
            self.applied,
            self.blocked_reasons.len(),
        )
    }

    fn operator_digest(&self) -> String {
        self_evolution_stable_digest(&format!("operator={}", self.operator_id))
    }

    fn approval_ticket_digest(&self) -> String {
        self_evolution_stable_digest(&format!("approval_ticket={}", self.approval_ticket_id))
    }

    fn approval_reason_digest(&self) -> String {
        self_evolution_stable_digest(&format!("approval_reason={}", self.approval_reason))
    }

    fn approved_refs_digest(&self) -> String {
        self_evolution_stable_digest(&format!(
            "review_packets={:?};evidence={:?};rollback_anchors={:?};content_digests={:?};schemas={:?}",
            self.approved_review_packet_ids,
            self.approved_evidence_ids,
            self.approved_rollback_anchor_ids,
            self.approved_content_digests,
            self.approved_source_report_schemas,
        ))
    }

    fn blocked_reasons_digest(&self) -> String {
        self_evolution_stable_digest(&format!("blocked_reasons={:?}", self.blocked_reasons))
    }

    fn normalize_for_ledger_append(&self) -> Self {
        let mut report = self.clone();
        let unsafe_flags = !report.read_only
            || !report.report_only
            || !report.preview_only
            || report.activation_write_allowed
            || report.active_candidate
            || report.write_allowed
            || report.applied;
        if unsafe_flags {
            report.decision = SelfEvolutionOperatorApprovalDecision::Hold;
            report.operator_approved = false;
            if !report.blocked_reasons.iter().any(|reason| {
                reason == "self_evolution_operator_approval_ledger_rejected_write_active_report"
            }) {
                report.blocked_reasons.push(
                    "self_evolution_operator_approval_ledger_rejected_write_active_report"
                        .to_owned(),
                );
            }
        }
        report.read_only = true;
        report.report_only = true;
        report.preview_only = true;
        report.activation_write_allowed = false;
        report.active_candidate = false;
        report.write_allowed = false;
        report.applied = false;
        report.content_digest = self_evolution_stable_digest(&format!(
            "operator_approval_ledger_record;decision={};operator_approved={};operator_digest={};ticket_digest={};approved_refs_digest={};attestation={};blocked_reasons_digest={};read_only={};report_only={};preview_only={};activation_write_allowed={};active_candidate={};write_allowed={};applied={}",
            report.decision.as_str(),
            report.operator_approved,
            report.operator_digest(),
            report.approval_ticket_digest(),
            report.approved_refs_digest(),
            report.approval_attestation_digest,
            report.blocked_reasons_digest(),
            report.read_only,
            report.report_only,
            report.preview_only,
            report.activation_write_allowed,
            report.active_candidate,
            report.write_allowed,
            report.applied,
        ));
        report
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfEvolutionOperatorApprovalRecord {
    pub sequence: u64,
    pub report: SelfEvolutionOperatorApprovalReport,
}

impl SelfEvolutionOperatorApprovalRecord {
    pub fn summary_line(&self) -> String {
        format!(
            "self_evolution_operator_approval_record sequence={} {}",
            self.sequence,
            self.report.summary_line()
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SelfEvolutionOperatorApprovalLedger {
    records: Vec<SelfEvolutionOperatorApprovalRecord>,
}

impl SelfEvolutionOperatorApprovalLedger {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn records(&self) -> &[SelfEvolutionOperatorApprovalRecord] {
        &self.records
    }

    pub fn append_report(
        &mut self,
        report: &SelfEvolutionOperatorApprovalReport,
    ) -> SelfEvolutionOperatorApprovalRecord {
        let record = SelfEvolutionOperatorApprovalRecord {
            sequence: self.records.len() as u64 + 1,
            report: report.normalize_for_ledger_append(),
        };
        self.records.push(record.clone());
        record
    }

    pub fn approved(&self) -> usize {
        self.records
            .iter()
            .filter(|record| record.report.operator_approved)
            .count()
    }

    pub fn held(&self) -> usize {
        self.records
            .iter()
            .filter(|record| !record.report.operator_approved)
            .count()
    }

    pub fn write_allowed_records(&self) -> usize {
        self.records
            .iter()
            .filter(|record| record.report.write_allowed || record.report.activation_write_allowed)
            .count()
    }

    pub fn applied_records(&self) -> usize {
        self.records
            .iter()
            .filter(|record| record.report.applied || record.report.active_candidate)
            .count()
    }

    pub fn summary_line(&self) -> String {
        format!(
            "self_evolution_operator_approval_ledger records={} approved={} held={} write_allowed_records={} applied_records={}",
            self.records.len(),
            self.approved(),
            self.held(),
            self.write_allowed_records(),
            self.applied_records(),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelfEvolutionPromotionPreflightDecision {
    ReadyForExplicitPromotion,
    Hold,
}

impl SelfEvolutionPromotionPreflightDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ReadyForExplicitPromotion => "ready_for_explicit_promotion",
            Self::Hold => "hold",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SelfEvolutionPromotionPreflightGate;

impl SelfEvolutionPromotionPreflightGate {
    pub fn new() -> Self {
        Self
    }

    pub fn evaluate(
        &self,
        admission: &SelfEvolutionAdmissionReport,
        experiment: &SelfEvolutionExperimentRecord,
        approval: &SelfEvolutionOperatorApprovalReport,
    ) -> SelfEvolutionPromotionPreflightReport {
        let mut blocked_reasons = Vec::new();

        if !admission.policy_valid {
            blocked_reasons.push("self_evolution_promotion_preflight_policy_invalid".to_owned());
        }
        if !admission.admitted_for_human_review || !admission.human_approval_required {
            blocked_reasons
                .push("self_evolution_promotion_preflight_admission_not_admitted".to_owned());
        }
        if !admission.read_only || !admission.report_only || !admission.preview_only {
            blocked_reasons.push(
                "self_evolution_promotion_preflight_admission_not_read_only_preview".to_owned(),
            );
        }
        if !admission.rust_validation_passed {
            blocked_reasons
                .push("self_evolution_promotion_preflight_rust_validation_failed".to_owned());
        }
        if !admission.validation_passed {
            blocked_reasons.push("self_evolution_promotion_preflight_validation_failed".to_owned());
        }
        if !admission.benchmark_gate_passed {
            blocked_reasons
                .push("self_evolution_promotion_preflight_benchmark_gate_failed".to_owned());
        }
        if !admission.rollback_budget_clean {
            blocked_reasons
                .push("self_evolution_promotion_preflight_rollback_budget_dirty".to_owned());
        }
        if !admission.adaptive_preview_evidence_present {
            blocked_reasons
                .push("self_evolution_promotion_preflight_adaptive_preview_missing".to_owned());
        }
        if admission.adaptive_preview_write_allowed
            || admission.adaptive_preview_applied
            || admission.mutation_write_allowed
            || admission.memory_store_write_allowed
            || admission.ndkv_write_allowed
            || admission.model_weight_write_allowed
            || admission.git_write_allowed
        {
            blocked_reasons
                .push("self_evolution_promotion_preflight_admission_write_or_applied".to_owned());
        }
        if !admission.blocked_reasons.is_empty() {
            blocked_reasons.push(
                "self_evolution_promotion_preflight_admission_blocked_reasons_present".to_owned(),
            );
        }

        if experiment.candidate_id != admission.candidate_id {
            blocked_reasons.push(
                "self_evolution_promotion_preflight_experiment_candidate_mismatch".to_owned(),
            );
        }
        if experiment.decision != SelfEvolutionExperimentDecision::AdmitForHumanReview {
            blocked_reasons
                .push("self_evolution_promotion_preflight_experiment_not_admitted".to_owned());
        }
        if !experiment.human_approval_required {
            blocked_reasons.push(
                "self_evolution_promotion_preflight_experiment_human_approval_not_required"
                    .to_owned(),
            );
        }
        if experiment.repeated_experiment {
            blocked_reasons
                .push("self_evolution_promotion_preflight_experiment_repeated".to_owned());
        }
        if experiment.conflicting_evidence {
            blocked_reasons.push(
                "self_evolution_promotion_preflight_experiment_conflicting_evidence".to_owned(),
            );
        }
        if experiment.rollback_required || experiment.rollback_replayable {
            blocked_reasons
                .push("self_evolution_promotion_preflight_experiment_rollback_required".to_owned());
        }
        if !experiment.read_only || !experiment.report_only || !experiment.preview_only {
            blocked_reasons.push(
                "self_evolution_promotion_preflight_experiment_not_read_only_preview".to_owned(),
            );
        }
        if experiment.active_candidate || experiment.write_allowed || experiment.applied {
            blocked_reasons
                .push("self_evolution_promotion_preflight_experiment_write_or_applied".to_owned());
        }
        if experiment.evidence_ids.is_empty() {
            blocked_reasons
                .push("self_evolution_promotion_preflight_experiment_evidence_missing".to_owned());
        }
        if experiment.rollback_anchor_ids.is_empty() {
            blocked_reasons.push(
                "self_evolution_promotion_preflight_experiment_rollback_anchor_missing".to_owned(),
            );
        }

        if approval.decision != SelfEvolutionOperatorApprovalDecision::Approved
            || !approval.operator_approved
        {
            blocked_reasons.push(
                "self_evolution_promotion_preflight_operator_approval_not_approved".to_owned(),
            );
        }
        if !approval.read_only || !approval.report_only || !approval.preview_only {
            blocked_reasons.push(
                "self_evolution_promotion_preflight_operator_approval_not_read_only_preview"
                    .to_owned(),
            );
        }
        if approval.activation_write_allowed
            || approval.active_candidate
            || approval.write_allowed
            || approval.applied
        {
            blocked_reasons.push(
                "self_evolution_promotion_preflight_operator_approval_write_or_applied".to_owned(),
            );
        }

        push_promotion_preflight_ref_mismatch(
            &mut blocked_reasons,
            "approval_review_packet_ids",
            &admission.review_packet.approval_review_packet_ids,
            &approval.approved_review_packet_ids,
        );
        push_promotion_preflight_ref_mismatch(
            &mut blocked_reasons,
            "evidence_ids",
            &admission.review_packet.evidence_ids,
            &approval.approved_evidence_ids,
        );
        push_promotion_preflight_ref_mismatch(
            &mut blocked_reasons,
            "rollback_anchor_ids",
            &admission.review_packet.rollback_anchor_ids,
            &approval.approved_rollback_anchor_ids,
        );
        push_promotion_preflight_ref_mismatch(
            &mut blocked_reasons,
            "content_digests",
            &admission.review_packet.content_digests,
            &approval.approved_content_digests,
        );
        push_promotion_preflight_ref_mismatch(
            &mut blocked_reasons,
            "source_report_schemas",
            &admission.review_packet.source_report_schemas,
            &approval.approved_source_report_schemas,
        );

        let ready_for_explicit_promotion = blocked_reasons.is_empty();
        let decision = if ready_for_explicit_promotion {
            SelfEvolutionPromotionPreflightDecision::ReadyForExplicitPromotion
        } else {
            SelfEvolutionPromotionPreflightDecision::Hold
        };
        let content_digest = self_evolution_stable_digest(&format!(
            "promotion_preflight;candidate={};decision={};ready={};admission_admitted={};experiment={};approval={};review_packets={:?};evidence={:?};rollback_anchors={:?};content_digests={:?};schemas={:?};blocked={:?}",
            admission.candidate_id,
            decision.as_str(),
            ready_for_explicit_promotion,
            admission.admitted_for_human_review,
            experiment.content_digest,
            approval.content_digest,
            admission.review_packet.approval_review_packet_ids,
            admission.review_packet.evidence_ids,
            admission.review_packet.rollback_anchor_ids,
            admission.review_packet.content_digests,
            admission.review_packet.source_report_schemas,
            blocked_reasons,
        ));

        SelfEvolutionPromotionPreflightReport {
            decision,
            ready_for_explicit_promotion,
            explicit_promotion_required: true,
            candidate_id: admission.candidate_id.clone(),
            admission_admitted_for_human_review: admission.admitted_for_human_review,
            experiment_admitted_for_human_review: experiment.decision
                == SelfEvolutionExperimentDecision::AdmitForHumanReview,
            operator_approved: approval.operator_approved,
            rust_validation_passed: admission.rust_validation_passed,
            validation_passed: admission.validation_passed,
            benchmark_gate_passed: admission.benchmark_gate_passed,
            adaptive_preview_evidence_present: admission.adaptive_preview_evidence_present,
            review_packet_count: admission.review_packet.approval_review_packet_ids.len(),
            evidence_id_count: admission.review_packet.evidence_ids.len(),
            rollback_anchor_count: admission.review_packet.rollback_anchor_ids.len(),
            content_digest_count: admission.review_packet.content_digests.len(),
            source_report_schema_count: admission.review_packet.source_report_schemas.len(),
            read_only: true,
            report_only: true,
            preview_only: true,
            activation_write_allowed: false,
            active_candidate: false,
            write_allowed: false,
            applied: false,
            blocked_reasons,
            content_digest,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfEvolutionPromotionPreflightReport {
    pub decision: SelfEvolutionPromotionPreflightDecision,
    pub ready_for_explicit_promotion: bool,
    pub explicit_promotion_required: bool,
    pub candidate_id: String,
    pub admission_admitted_for_human_review: bool,
    pub experiment_admitted_for_human_review: bool,
    pub operator_approved: bool,
    pub rust_validation_passed: bool,
    pub validation_passed: bool,
    pub benchmark_gate_passed: bool,
    pub adaptive_preview_evidence_present: bool,
    pub review_packet_count: usize,
    pub evidence_id_count: usize,
    pub rollback_anchor_count: usize,
    pub content_digest_count: usize,
    pub source_report_schema_count: usize,
    pub read_only: bool,
    pub report_only: bool,
    pub preview_only: bool,
    pub activation_write_allowed: bool,
    pub active_candidate: bool,
    pub write_allowed: bool,
    pub applied: bool,
    pub blocked_reasons: Vec<String>,
    pub content_digest: String,
}

impl SelfEvolutionPromotionPreflightReport {
    pub fn summary_line(&self) -> String {
        format!(
            "self_evolution_promotion_preflight decision={} ready_for_explicit_promotion={} explicit_promotion_required={} candidate={} admission_admitted={} experiment_admitted={} operator_approved={} rust_validation_passed={} validation_passed={} benchmark_gate_passed={} adaptive_preview_evidence={} review_packets={} evidence_ids={} rollback_anchors={} content_digests={} source_report_schemas={} read_only={} report_only={} preview_only={} activation_write_allowed={} active_candidate={} write_allowed={} applied={} blocked_reasons={} digest={}",
            self.decision.as_str(),
            self.ready_for_explicit_promotion,
            self.explicit_promotion_required,
            self.candidate_id,
            self.admission_admitted_for_human_review,
            self.experiment_admitted_for_human_review,
            self.operator_approved,
            self.rust_validation_passed,
            self.validation_passed,
            self.benchmark_gate_passed,
            self.adaptive_preview_evidence_present,
            self.review_packet_count,
            self.evidence_id_count,
            self.rollback_anchor_count,
            self.content_digest_count,
            self.source_report_schema_count,
            self.read_only,
            self.report_only,
            self.preview_only,
            self.activation_write_allowed,
            self.active_candidate,
            self.write_allowed,
            self.applied,
            self.blocked_reasons.len(),
            self.content_digest,
        )
    }

    pub fn json_line(&self) -> String {
        let candidate_id = self_evolution_json_escape(&self.candidate_id);
        let blocked_reasons_digest = self_evolution_json_escape(&self.blocked_reasons_digest());
        let content_digest = self_evolution_json_escape(&self.content_digest);

        format!(
            "{{\
             \"schema\":\"rust-norion-self-evolution-promotion-preflight-v1\",\
             \"decision\":\"{}\",\
             \"ready_for_explicit_promotion\":{},\
             \"explicit_promotion_required\":{},\
             \"candidate_id\":\"{candidate_id}\",\
             \"admission_admitted_for_human_review\":{},\
             \"experiment_admitted_for_human_review\":{},\
             \"operator_approved\":{},\
             \"rust_validation_passed\":{},\
             \"validation_passed\":{},\
             \"benchmark_gate_passed\":{},\
             \"adaptive_preview_evidence_present\":{},\
             \"review_packet_count\":{},\
             \"evidence_id_count\":{},\
             \"rollback_anchor_count\":{},\
             \"content_digest_count\":{},\
             \"source_report_schema_count\":{},\
             \"read_only\":{},\
             \"report_only\":{},\
             \"preview_only\":{},\
             \"activation_write_allowed\":{},\
             \"active_candidate\":{},\
             \"write_allowed\":{},\
             \"applied\":{},\
             \"blocked_reasons_count\":{},\
             \"blocked_reasons_digest\":\"{blocked_reasons_digest}\",\
             \"content_digest\":\"{content_digest}\"\
             }}",
            self.decision.as_str(),
            self.ready_for_explicit_promotion,
            self.explicit_promotion_required,
            self.admission_admitted_for_human_review,
            self.experiment_admitted_for_human_review,
            self.operator_approved,
            self.rust_validation_passed,
            self.validation_passed,
            self.benchmark_gate_passed,
            self.adaptive_preview_evidence_present,
            self.review_packet_count,
            self.evidence_id_count,
            self.rollback_anchor_count,
            self.content_digest_count,
            self.source_report_schema_count,
            self.read_only,
            self.report_only,
            self.preview_only,
            self.activation_write_allowed,
            self.active_candidate,
            self.write_allowed,
            self.applied,
            self.blocked_reasons.len(),
        )
    }

    pub fn is_read_only_preflight(&self) -> bool {
        self.read_only
            && self.report_only
            && self.preview_only
            && !self.activation_write_allowed
            && !self.active_candidate
            && !self.write_allowed
            && !self.applied
    }

    fn blocked_reasons_digest(&self) -> String {
        self_evolution_stable_digest(&format!("blocked_reasons={:?}", self.blocked_reasons))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelfEvolutionRollbackReplayApplyDecision {
    ReadyForOperatorApply,
    Hold,
}

impl SelfEvolutionRollbackReplayApplyDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ReadyForOperatorApply => "ready_for_operator_apply",
            Self::Hold => "hold",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SelfEvolutionRollbackReplayApplyGate;

impl SelfEvolutionRollbackReplayApplyGate {
    pub fn new() -> Self {
        Self
    }

    pub fn evaluate(
        &self,
        rollback_gate: &SelfEvolutionRollbackReplayGateReport,
        approval: &SelfEvolutionOperatorApprovalReport,
    ) -> SelfEvolutionRollbackReplayApplyReport {
        self.evaluate_for_scope(&TenantScope::local_single_user(), rollback_gate, approval)
    }

    pub fn evaluate_for_scope(
        &self,
        actor_scope: &TenantScope,
        rollback_gate: &SelfEvolutionRollbackReplayGateReport,
        approval: &SelfEvolutionOperatorApprovalReport,
    ) -> SelfEvolutionRollbackReplayApplyReport {
        let mut blocked_reasons = Vec::new();

        if rollback_gate.decision != SelfEvolutionRollbackReplayDecision::AdmitForHumanReview
            || !rollback_gate.admitted_for_human_review
        {
            blocked_reasons.push(
                "self_evolution_rollback_replay_apply_gate_not_admitted_for_review".to_owned(),
            );
        }
        if !rollback_gate.human_approval_required {
            blocked_reasons.push(
                "self_evolution_rollback_replay_apply_gate_human_approval_not_required".to_owned(),
            );
        }
        if rollback_gate.item_count == 0 {
            blocked_reasons.push("self_evolution_rollback_replay_apply_gate_empty_plan".to_owned());
        }
        if !rollback_gate.all_replayable
            || rollback_gate.blocked > 0
            || rollback_gate.replayable != rollback_gate.item_count
        {
            blocked_reasons.push(
                "self_evolution_rollback_replay_apply_gate_plan_not_all_replayable".to_owned(),
            );
        }
        if rollback_gate.active_candidates > 0 {
            blocked_reasons.push(format!(
                "self_evolution_rollback_replay_apply_gate_active_candidates={}>0",
                rollback_gate.active_candidates
            ));
        }
        if rollback_gate.item_write_allowed > 0 || rollback_gate.item_applied > 0 {
            blocked_reasons
                .push("self_evolution_rollback_replay_apply_gate_item_write_or_applied".to_owned());
        }
        if !rollback_gate.read_only || !rollback_gate.report_only || !rollback_gate.preview_only {
            blocked_reasons
                .push("self_evolution_rollback_replay_apply_gate_not_read_only_preview".to_owned());
        }
        if rollback_gate.write_allowed || rollback_gate.applied {
            blocked_reasons
                .push("self_evolution_rollback_replay_apply_gate_write_or_applied".to_owned());
        }
        if !rollback_gate.plan_read_only
            || !rollback_gate.plan_report_only
            || !rollback_gate.plan_preview_only
        {
            blocked_reasons.push(
                "self_evolution_rollback_replay_apply_gate_plan_not_read_only_preview".to_owned(),
            );
        }
        if rollback_gate.plan_write_allowed || rollback_gate.plan_applied {
            blocked_reasons
                .push("self_evolution_rollback_replay_apply_gate_plan_write_or_applied".to_owned());
        }

        if approval.decision != SelfEvolutionOperatorApprovalDecision::Approved
            || !approval.operator_approved
        {
            blocked_reasons.push(
                "self_evolution_rollback_replay_apply_operator_approval_not_approved".to_owned(),
            );
        }
        if !approval.read_only || !approval.report_only || !approval.preview_only {
            blocked_reasons.push(
                "self_evolution_rollback_replay_apply_operator_approval_not_read_only_preview"
                    .to_owned(),
            );
        }
        if approval.activation_write_allowed
            || approval.active_candidate
            || approval.write_allowed
            || approval.applied
        {
            blocked_reasons.push(
                "self_evolution_rollback_replay_apply_operator_approval_write_or_applied"
                    .to_owned(),
            );
        }

        push_rollback_replay_apply_ref_mismatch(
            &mut blocked_reasons,
            "approval_review_packet_ids",
            &rollback_gate.review_packet.approval_review_packet_ids,
            &approval.approved_review_packet_ids,
        );
        push_rollback_replay_apply_ref_mismatch(
            &mut blocked_reasons,
            "evidence_ids",
            &rollback_gate.review_packet.evidence_ids,
            &approval.approved_evidence_ids,
        );
        push_rollback_replay_apply_ref_mismatch(
            &mut blocked_reasons,
            "rollback_anchor_ids",
            &rollback_gate.review_packet.rollback_anchor_ids,
            &approval.approved_rollback_anchor_ids,
        );
        push_rollback_replay_apply_anchor_scope_reason(
            &mut blocked_reasons,
            "rollback_anchor_ids",
            actor_scope,
            &rollback_gate.review_packet.rollback_anchor_ids,
        );
        push_rollback_replay_apply_anchor_scope_reason(
            &mut blocked_reasons,
            "approved_rollback_anchor_ids",
            actor_scope,
            &approval.approved_rollback_anchor_ids,
        );
        push_rollback_replay_apply_ref_mismatch(
            &mut blocked_reasons,
            "content_digests",
            &rollback_gate.review_packet.content_digests,
            &approval.approved_content_digests,
        );
        push_rollback_replay_apply_ref_mismatch(
            &mut blocked_reasons,
            "source_report_schemas",
            &rollback_gate.review_packet.source_report_schemas,
            &approval.approved_source_report_schemas,
        );

        let ready_for_operator_apply = blocked_reasons.is_empty();
        let decision = if ready_for_operator_apply {
            SelfEvolutionRollbackReplayApplyDecision::ReadyForOperatorApply
        } else {
            SelfEvolutionRollbackReplayApplyDecision::Hold
        };
        let content_digest = self_evolution_stable_digest(&format!(
            "rollback_replay_apply;decision={};ready={};rollback_gate={};approval={};items={};review_packets={:?};evidence={:?};rollback_anchors={:?};content_digests={:?};schemas={:?};blocked={:?}",
            decision.as_str(),
            ready_for_operator_apply,
            rollback_gate.content_digest,
            approval.content_digest,
            rollback_gate.item_count,
            rollback_gate.review_packet.approval_review_packet_ids,
            rollback_gate.review_packet.evidence_ids,
            rollback_gate.review_packet.rollback_anchor_ids,
            rollback_gate.review_packet.content_digests,
            rollback_gate.review_packet.source_report_schemas,
            blocked_reasons,
        ));

        SelfEvolutionRollbackReplayApplyReport {
            decision,
            ready_for_operator_apply,
            explicit_apply_required: true,
            rollback_gate_admitted_for_human_review: rollback_gate.admitted_for_human_review,
            operator_approved: approval.operator_approved,
            item_count: rollback_gate.item_count,
            replayable: rollback_gate.replayable,
            blocked: rollback_gate.blocked,
            review_packet_count: rollback_gate.review_packet.approval_review_packet_ids.len(),
            evidence_id_count: rollback_gate.review_packet.evidence_ids.len(),
            rollback_anchor_count: rollback_gate.review_packet.rollback_anchor_ids.len(),
            content_digest_count: rollback_gate.review_packet.content_digests.len(),
            source_report_schema_count: rollback_gate.review_packet.source_report_schemas.len(),
            read_only: true,
            report_only: true,
            preview_only: true,
            activation_write_allowed: false,
            active_candidate: false,
            write_allowed: false,
            applied: false,
            blocked_reasons,
            content_digest,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfEvolutionRollbackReplayApplyReport {
    pub decision: SelfEvolutionRollbackReplayApplyDecision,
    pub ready_for_operator_apply: bool,
    pub explicit_apply_required: bool,
    pub rollback_gate_admitted_for_human_review: bool,
    pub operator_approved: bool,
    pub item_count: usize,
    pub replayable: usize,
    pub blocked: usize,
    pub review_packet_count: usize,
    pub evidence_id_count: usize,
    pub rollback_anchor_count: usize,
    pub content_digest_count: usize,
    pub source_report_schema_count: usize,
    pub read_only: bool,
    pub report_only: bool,
    pub preview_only: bool,
    pub activation_write_allowed: bool,
    pub active_candidate: bool,
    pub write_allowed: bool,
    pub applied: bool,
    pub blocked_reasons: Vec<String>,
    pub content_digest: String,
}

impl SelfEvolutionRollbackReplayApplyReport {
    pub fn summary_line(&self) -> String {
        format!(
            "self_evolution_rollback_replay_apply decision={} ready_for_operator_apply={} explicit_apply_required={} rollback_gate_admitted={} operator_approved={} items={} replayable={} blocked={} review_packets={} evidence_ids={} rollback_anchors={} content_digests={} schemas={} read_only={} report_only={} preview_only={} activation_write_allowed={} active_candidate={} write_allowed={} applied={} blocked_reasons={} digest={}",
            self.decision.as_str(),
            self.ready_for_operator_apply,
            self.explicit_apply_required,
            self.rollback_gate_admitted_for_human_review,
            self.operator_approved,
            self.item_count,
            self.replayable,
            self.blocked,
            self.review_packet_count,
            self.evidence_id_count,
            self.rollback_anchor_count,
            self.content_digest_count,
            self.source_report_schema_count,
            self.read_only,
            self.report_only,
            self.preview_only,
            self.activation_write_allowed,
            self.active_candidate,
            self.write_allowed,
            self.applied,
            self.blocked_reasons.len(),
            self.content_digest,
        )
    }

    pub fn json_line(&self) -> String {
        let blocked_reasons_digest = self_evolution_json_escape(&self.blocked_reasons_digest());
        let content_digest = self_evolution_json_escape(&self.content_digest);

        format!(
            "{{\
             \"schema\":\"rust-norion-self-evolution-rollback-replay-apply-v1\",\
             \"decision\":\"{}\",\
             \"ready_for_operator_apply\":{},\
             \"explicit_apply_required\":{},\
             \"rollback_gate_admitted_for_human_review\":{},\
             \"operator_approved\":{},\
             \"item_count\":{},\
             \"replayable\":{},\
             \"blocked\":{},\
             \"review_packet_count\":{},\
             \"evidence_id_count\":{},\
             \"rollback_anchor_count\":{},\
             \"content_digest_count\":{},\
             \"source_report_schema_count\":{},\
             \"read_only\":{},\
             \"report_only\":{},\
             \"preview_only\":{},\
             \"activation_write_allowed\":{},\
             \"active_candidate\":{},\
             \"write_allowed\":{},\
             \"applied\":{},\
             \"blocked_reasons_count\":{},\
             \"blocked_reasons_digest\":\"{blocked_reasons_digest}\",\
             \"content_digest\":\"{content_digest}\"\
             }}",
            self.decision.as_str(),
            self.ready_for_operator_apply,
            self.explicit_apply_required,
            self.rollback_gate_admitted_for_human_review,
            self.operator_approved,
            self.item_count,
            self.replayable,
            self.blocked,
            self.review_packet_count,
            self.evidence_id_count,
            self.rollback_anchor_count,
            self.content_digest_count,
            self.source_report_schema_count,
            self.read_only,
            self.report_only,
            self.preview_only,
            self.activation_write_allowed,
            self.active_candidate,
            self.write_allowed,
            self.applied,
            self.blocked_reasons.len(),
        )
    }

    pub fn is_read_only_preflight(&self) -> bool {
        self.read_only
            && self.report_only
            && self.preview_only
            && !self.activation_write_allowed
            && !self.active_candidate
            && !self.write_allowed
            && !self.applied
    }

    fn blocked_reasons_digest(&self) -> String {
        self_evolution_stable_digest(&format!("blocked_reasons={:?}", self.blocked_reasons))
    }
}

fn push_operator_approval_presence_reason(
    blocked_reasons: &mut Vec<String>,
    required: bool,
    field: &str,
    expected: &[String],
    approved: &[String],
) {
    if !required {
        return;
    }
    if expected.is_empty() {
        blocked_reasons.push(format!(
            "self_evolution_operator_approval_review_packet_{field}_empty"
        ));
    }
    if approved.is_empty() {
        blocked_reasons.push(format!(
            "self_evolution_operator_approval_approved_{field}_empty"
        ));
    }
}

fn push_operator_approval_packet_scope_reason(
    blocked_reasons: &mut Vec<String>,
    field: &str,
    actor_scope: &TenantScope,
    packet_ids: &[String],
) {
    let gate = TenantIsolationGate::new();
    for packet_id in packet_ids {
        let Some(packet_key) = TenantScopedKey::parse(packet_id) else {
            push_unique_string(
                blocked_reasons,
                format!("self_evolution_operator_approval_{field}_unscoped"),
            );
            continue;
        };
        if packet_key.lane != TenantResourceLane::ApprovalPacket {
            push_unique_string(
                blocked_reasons,
                format!("self_evolution_operator_approval_{field}_wrong_lane"),
            );
            continue;
        }
        let report = gate.check_key_access(actor_scope, &packet_key, TenantAccessKind::Read);
        if !report.allowed {
            push_unique_string(
                blocked_reasons,
                format!("self_evolution_operator_approval_{field}_scope_rejected"),
            );
        }
    }
}

fn push_rollback_replay_apply_ref_mismatch(
    blocked_reasons: &mut Vec<String>,
    field: &str,
    expected: &[String],
    approved: &[String],
) {
    if expected.is_empty() || approved.is_empty() || expected != approved {
        blocked_reasons.push(format!(
            "self_evolution_rollback_replay_apply_{field}_mismatch"
        ));
    }
}

fn push_rollback_replay_apply_anchor_scope_reason(
    blocked_reasons: &mut Vec<String>,
    field: &str,
    actor_scope: &TenantScope,
    rollback_anchor_ids: &[String],
) {
    let gate = TenantIsolationGate::new();
    for anchor_id in rollback_anchor_ids {
        let Some(anchor_key) = TenantScopedKey::parse(anchor_id) else {
            push_unique_string(
                blocked_reasons,
                format!("self_evolution_rollback_replay_apply_{field}_unscoped"),
            );
            continue;
        };
        if anchor_key.lane != TenantResourceLane::SessionState {
            push_unique_string(
                blocked_reasons,
                format!("self_evolution_rollback_replay_apply_{field}_wrong_lane"),
            );
            continue;
        }
        let report =
            gate.check_key_access(actor_scope, &anchor_key, TenantAccessKind::RollbackReplay);
        if !report.allowed {
            push_unique_string(
                blocked_reasons,
                format!("self_evolution_rollback_replay_apply_{field}_scope_rejected"),
            );
        }
    }
}

fn push_promotion_preflight_ref_mismatch(
    blocked_reasons: &mut Vec<String>,
    field: &str,
    expected: &[String],
    approved: &[String],
) {
    if expected.is_empty() || approved.is_empty() || expected != approved {
        blocked_reasons.push(format!(
            "self_evolution_promotion_preflight_{field}_mismatch"
        ));
    }
}

fn push_operator_approval_ref_mismatch(
    blocked_reasons: &mut Vec<String>,
    required: bool,
    field: &str,
    expected: &[String],
    approved: &[String],
) {
    if !required {
        return;
    }
    for value in expected {
        if value.trim().is_empty() {
            blocked_reasons.push(format!(
                "self_evolution_operator_approval_review_packet_{field}_contains_empty"
            ));
        } else if !approved.iter().any(|approved| approved == value) {
            blocked_reasons.push(format!(
                "self_evolution_operator_approval_missing_{field}={value}"
            ));
        }
    }
    for value in approved {
        if value.trim().is_empty() {
            blocked_reasons.push(format!(
                "self_evolution_operator_approval_approved_{field}_contains_empty"
            ));
        } else if !expected.iter().any(|expected| expected == value) {
            blocked_reasons.push(format!(
                "self_evolution_operator_approval_unexpected_{field}={value}"
            ));
        }
    }
    if expected != approved
        && expected
            .iter()
            .filter(|value| !value.trim().is_empty())
            .all(|value| approved.iter().any(|approved| approved == value))
        && approved
            .iter()
            .filter(|value| !value.trim().is_empty())
            .all(|value| expected.iter().any(|expected| expected == value))
    {
        blocked_reasons.push(format!(
            "self_evolution_operator_approval_{field}_order_or_duplicate_mismatch"
        ));
    }
}

fn self_evolution_operator_approval_attestation_digest(
    operator_id: &str,
    approval_ticket_id: &str,
    approved_review_packet_ids: &[String],
    approved_evidence_ids: &[String],
    approved_rollback_anchor_ids: &[String],
    approved_content_digests: &[String],
    approved_source_report_schemas: &[String],
    approval_reason: &str,
) -> String {
    self_evolution_stable_digest(&format!(
        "operator={operator_id};ticket={approval_ticket_id};review_packets={approved_review_packet_ids:?};evidence={approved_evidence_ids:?};rollback_anchors={approved_rollback_anchor_ids:?};content_digests={approved_content_digests:?};schemas={approved_source_report_schemas:?};reason={approval_reason}",
    ))
}

fn self_evolution_f32_json(value: f32) -> String {
    if value.is_finite() {
        format!("{value:.6}")
    } else {
        "null".to_owned()
    }
}

fn self_evolution_string_array_json(items: &[String]) -> String {
    let values = items
        .iter()
        .map(|item| format!("\"{}\"", self_evolution_json_escape(item)))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{values}]")
}

fn self_evolution_json_escape(value: &str) -> String {
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

fn finite_or_zero(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

fn self_evolution_promotion_validation_passed(validation: SelfEvolutionValidationEvidence) -> bool {
    self_evolution_promotion_lane_passed(validation.compiler)
        && self_evolution_promotion_lane_passed(validation.tests)
        && self_evolution_promotion_lane_passed(validation.benchmarks)
        && validation.experiments.failed == 0
        && validation
            .experiments
            .passed
            .saturating_add(validation.experiments.failed)
            <= validation.experiments.items
}

fn self_evolution_promotion_lane_passed(lane: SelfEvolutionValidationLane) -> bool {
    lane.items > 0
        && lane.passed > 0
        && lane.failed == 0
        && lane.passed.saturating_add(lane.failed) <= lane.items
}

fn self_evolution_promotion_validation_blockers(
    validation: SelfEvolutionValidationEvidence,
) -> Vec<String> {
    let mut blocked = Vec::new();
    self_evolution_promotion_push_lane_blocker(&mut blocked, "compiler", validation.compiler);
    self_evolution_promotion_push_lane_blocker(&mut blocked, "tests", validation.tests);
    self_evolution_promotion_push_lane_blocker(&mut blocked, "benchmarks", validation.benchmarks);
    if validation.experiments.failed > 0
        || validation
            .experiments
            .passed
            .saturating_add(validation.experiments.failed)
            > validation.experiments.items
    {
        self_evolution_promotion_push_lane_blocker(
            &mut blocked,
            "experiments",
            validation.experiments,
        );
    }
    blocked
}

fn self_evolution_promotion_push_lane_blocker(
    blocked: &mut Vec<String>,
    lane_name: &str,
    lane: SelfEvolutionValidationLane,
) {
    if lane.items == 0 || lane.passed == 0 {
        blocked.push(format!("promotion_{lane_name}_validation_missing"));
    }
    if lane.failed > 0 {
        blocked.push(format!(
            "promotion_{lane_name}_validation_failed:{}",
            lane.failed
        ));
    }
    if lane.passed.saturating_add(lane.failed) > lane.items {
        blocked.push(format!(
            "promotion_{lane_name}_validation_accounting_invalid"
        ));
    }
}

fn digest_or_trace_like(value: &str) -> bool {
    let value = value.trim();
    value.starts_with("fnv64:")
        || value.starts_with("sha256:")
        || value.starts_with("digest:")
        || value.starts_with("redaction-digest:")
}

fn trace_id_like(value: &str) -> bool {
    let value = value.trim();
    (value.starts_with("trace:")
        || value.starts_with("run:")
        || value.starts_with("gh-run:")
        || value.starts_with("artifact:"))
        && !value.contains(char::is_whitespace)
}

fn digest_like_or_redacted(value: &str) -> String {
    if digest_or_trace_like(value) && !contains_private_or_executable_marker(value) {
        self_evolution_review_id_component(value)
    } else {
        self_evolution_stable_digest(value)
    }
}

fn self_evolution_promotion_digest(
    candidate: &SelfEvolutionPromotionCandidate,
    artifacts: &[SelfEvolutionPromotionArtifactRef],
    decision: SelfEvolutionPromotionDecision,
    blocked_reasons: &[String],
) -> String {
    let artifact_digests = artifacts
        .iter()
        .map(|artifact| artifact.content_digest.as_str())
        .collect::<Vec<_>>()
        .join("|");
    let trace_ids = artifacts
        .iter()
        .filter_map(|artifact| artifact.trace_id.as_deref())
        .collect::<Vec<_>>()
        .join("|");
    self_evolution_stable_digest(&format!(
        "candidate={};lane={};decision={};correctness_delta={:.6};latency_delta_ms={};wasted_compute_delta={:.6};privacy_risk={:.6};reproducible_runs={};cross_task_regression={:.6};flaky_runs={};rollback_ready={};rollback_anchor={};validation={:?};artifacts={};traces={};blocked={}",
        candidate.candidate_id,
        candidate.lane.as_str(),
        decision.as_str(),
        finite_or_zero(candidate.correctness_delta),
        candidate.latency_delta_ms,
        finite_or_zero(candidate.wasted_compute_delta),
        finite_or_zero(candidate.privacy_risk).clamp(0.0, 1.0),
        candidate.reproducible_runs,
        finite_or_zero(candidate.cross_task_regression).max(0.0),
        candidate.flaky_runs,
        candidate.rollback_ready,
        candidate.rollback_anchor_id,
        candidate.validation,
        artifact_digests,
        trace_ids,
        blocked_reasons.join("|")
    ))
}

fn self_evolution_experiment_decision(
    report: &SelfEvolutionAdmissionReport,
    conflicting_evidence: bool,
) -> SelfEvolutionExperimentDecision {
    if !report.rollback_budget_clean
        || report.drift_rollbacks > 0
        || report.rollback_router_threshold_delta > 0.0
        || report.rollback_hierarchy_weight_delta > 0.0
    {
        return SelfEvolutionExperimentDecision::Rollback;
    }

    if conflicting_evidence {
        return SelfEvolutionExperimentDecision::Hold;
    }

    if report.admitted_for_human_review {
        return SelfEvolutionExperimentDecision::AdmitForHumanReview;
    }

    if self_evolution_experiment_failed_evidence(report) {
        SelfEvolutionExperimentDecision::Reject
    } else {
        SelfEvolutionExperimentDecision::Hold
    }
}

fn self_evolution_experiment_failed_evidence(report: &SelfEvolutionAdmissionReport) -> bool {
    !report.policy_valid
        || report.rust_check_failed > 0
        || report.validation.compiler.failed > 0
        || report.validation.tests.failed > 0
        || report.validation.benchmarks.failed > 0
        || report.validation.experiments.failed > 0
        || (!report.benchmark_gate_passed && !report.benchmark_gate_failures.is_empty())
        || report.adaptive_preview_write_allowed
        || report.adaptive_preview_applied
        || report.mutation_write_allowed
        || report.memory_store_write_allowed
        || report.ndkv_write_allowed
        || report.model_weight_write_allowed
        || report.git_write_allowed
}

fn self_evolution_experiment_conflicting_evidence(report: &SelfEvolutionAdmissionReport) -> bool {
    (report.benchmark_gate_passed && !report.benchmark_gate_failures.is_empty())
        || (report.rust_check_passed > 0 && report.rust_check_failed > 0)
        || self_evolution_validation_lane_conflicting(report.validation.compiler)
        || self_evolution_validation_lane_conflicting(report.validation.tests)
        || self_evolution_validation_lane_conflicting(report.validation.benchmarks)
        || self_evolution_validation_lane_conflicting(report.validation.experiments)
        || (report.admitted_for_human_review && !report.blocked_reasons.is_empty())
}

fn self_evolution_validation_lane_conflicting(lane: SelfEvolutionValidationLane) -> bool {
    (lane.passed > 0 && lane.failed > 0) || lane.passed.saturating_add(lane.failed) > lane.items
}

fn self_evolution_review_id_component(value: &str) -> String {
    let component = value
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.') {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_owned();
    if component.is_empty() {
        "candidate-missing".to_owned()
    } else {
        component
    }
}

fn self_evolution_stable_digest(value: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("fnv64:{hash:016x}")
}

fn push_unique_string(items: &mut Vec<String>, value: impl Into<String>) {
    let value = value.into();
    if value.trim().is_empty() || items.iter().any(|item| item == &value) {
        return;
    }
    items.push(value);
}

fn rollback_budget_clean(
    ledger: EvolutionLedger,
    max_drift_rollbacks: u64,
    max_rollback_router_threshold_delta: f32,
    max_rollback_hierarchy_weight_delta: f32,
) -> bool {
    ledger.drift_rollbacks <= max_drift_rollbacks
        && ledger.rollback_router_threshold_delta <= max_rollback_router_threshold_delta
        && ledger.rollback_hierarchy_weight_delta <= max_rollback_hierarchy_weight_delta
}

fn normalized_rollback_delta(delta: f32) -> Option<f32> {
    (delta.is_finite() && delta >= 0.0).then_some(delta)
}

fn push_validation_lane_blocked_reasons(
    blocked_reasons: &mut Vec<String>,
    name: &str,
    lane: SelfEvolutionValidationLane,
    minimum: u64,
    require_all_passed: bool,
) {
    if lane.items < minimum {
        blocked_reasons.push(format!(
            "self_evolution_admission_{name}_validation_items={}<{}",
            lane.items, minimum
        ));
    }
    if lane.passed < minimum {
        blocked_reasons.push(format!(
            "self_evolution_admission_{name}_validation_passed={}<{}",
            lane.passed, minimum
        ));
    }
    if require_all_passed && lane.failed > 0 {
        blocked_reasons.push(format!(
            "self_evolution_admission_{name}_validation_failed={}>0",
            lane.failed
        ));
    }
    if lane.passed.saturating_add(lane.failed) > lane.items {
        blocked_reasons.push(format!(
            "self_evolution_admission_{name}_validation_passed_failed_exceeds_items"
        ));
    }
}

fn router_threshold_preview_admissible(report: &RouterThresholdAdjustmentPreviewReport) -> bool {
    report.read_only
        && report.report_only
        && report.preview_only
        && report.adjustment_ready
        && !report.router_state_write_allowed
        && !report.adaptive_state_write_allowed
        && !report.ndkv_write_allowed
        && !report.router_observation_applied
        && report.blocked_reasons.is_empty()
}

fn hierarchy_adjustment_preview_admissible(report: &HierarchyAdjustmentPreviewReport) -> bool {
    report.read_only
        && report.report_only
        && report.preview_only
        && report.adjustment_ready
        && !report.state_write_allowed
        && !report.adaptive_state_write_allowed
        && !report.ndkv_write_allowed
        && !report.controller_observation_applied
        && report.blocked_reasons.is_empty()
}

fn router_threshold_preview_blocked_reasons(
    report: &RouterThresholdAdjustmentPreviewReport,
) -> Vec<String> {
    let mut reasons = Vec::new();
    if !report.read_only {
        reasons.push("self_evolution_admission_router_preview_not_read_only".to_owned());
    }
    if !report.report_only {
        reasons.push("self_evolution_admission_router_preview_not_report_only".to_owned());
    }
    if !report.preview_only {
        reasons.push("self_evolution_admission_router_preview_not_preview_only".to_owned());
    }
    if report.router_state_write_allowed
        || report.adaptive_state_write_allowed
        || report.ndkv_write_allowed
    {
        reasons.push("self_evolution_admission_router_preview_write_allowed".to_owned());
    }
    if report.router_observation_applied {
        reasons.push("self_evolution_admission_router_preview_already_applied".to_owned());
    }
    if !report.adjustment_ready {
        reasons.push("self_evolution_admission_router_preview_not_ready".to_owned());
    }
    reasons.extend(
        report
            .blocked_reasons
            .iter()
            .map(|reason| format!("self_evolution_admission_router_preview_blocked={reason}")),
    );
    reasons
}

fn hierarchy_adjustment_preview_blocked_reasons(
    report: &HierarchyAdjustmentPreviewReport,
) -> Vec<String> {
    let mut reasons = Vec::new();
    if !report.read_only {
        reasons.push("self_evolution_admission_hierarchy_preview_not_read_only".to_owned());
    }
    if !report.report_only {
        reasons.push("self_evolution_admission_hierarchy_preview_not_report_only".to_owned());
    }
    if !report.preview_only {
        reasons.push("self_evolution_admission_hierarchy_preview_not_preview_only".to_owned());
    }
    if report.state_write_allowed
        || report.adaptive_state_write_allowed
        || report.ndkv_write_allowed
    {
        reasons.push("self_evolution_admission_hierarchy_preview_write_allowed".to_owned());
    }
    if report.controller_observation_applied {
        reasons.push("self_evolution_admission_hierarchy_preview_already_applied".to_owned());
    }
    if !report.adjustment_ready {
        reasons.push("self_evolution_admission_hierarchy_preview_not_ready".to_owned());
    }
    reasons.extend(
        report
            .blocked_reasons
            .iter()
            .map(|reason| format!("self_evolution_admission_hierarchy_preview_blocked={reason}")),
    );
    reasons
}

fn kv_fusion_policy_observation_preview_blocked_reasons(
    report: &KvFusionPolicyObservationDryRunReport,
) -> Vec<String> {
    let mut reasons = Vec::new();
    if !report.reward_preview_source_read_only {
        reasons.push("self_evolution_admission_kv_fusion_preview_source_not_read_only".to_owned());
    }
    if report.reward_preview_source_memory_store_write_allowed {
        reasons.push(
            "self_evolution_admission_kv_fusion_preview_source_memory_store_write_allowed"
                .to_owned(),
        );
    }
    if !report.preview_only {
        reasons.push("self_evolution_admission_kv_fusion_preview_not_preview_only".to_owned());
    }
    if report.reward_preview_memory_store_write_allowed {
        reasons.push(
            "self_evolution_admission_kv_fusion_preview_memory_store_write_allowed".to_owned(),
        );
    }
    if report.reward_preview_kv_cache_write_allowed {
        reasons
            .push("self_evolution_admission_kv_fusion_preview_kv_cache_write_allowed".to_owned());
    }
    if report.policy_write_allowed {
        reasons.push("self_evolution_admission_kv_fusion_preview_write_allowed".to_owned());
    }
    if report.policy_observation_applied {
        reasons.push("self_evolution_admission_kv_fusion_preview_already_applied".to_owned());
    }
    if !report.policy_observation_ready {
        reasons.push("self_evolution_admission_kv_fusion_preview_not_ready".to_owned());
    }
    if !report.threshold_within_bounds {
        reasons
            .push("self_evolution_admission_kv_fusion_preview_threshold_out_of_bounds".to_owned());
    }
    reasons.extend(
        report
            .blocked_reasons
            .iter()
            .map(|reason| format!("self_evolution_admission_kv_fusion_preview_blocked={reason}")),
    );
    reasons
}

fn self_evolution_admission_telemetry(report: &SelfEvolutionAdmissionReport) -> Vec<String> {
    let mut telemetry = vec![
        "self_evolution_admission=true".to_owned(),
        format!("self_evolution_admission_candidate={}", report.candidate_id),
        format!("self_evolution_admission_read_only={}", report.read_only),
        format!(
            "self_evolution_admission_report_only={}",
            report.report_only
        ),
        format!(
            "self_evolution_admission_preview_only={}",
            report.preview_only
        ),
        format!(
            "self_evolution_admission_policy_valid={}",
            report.policy_valid
        ),
        format!(
            "self_evolution_admission_mutation_write_allowed={}",
            report.mutation_write_allowed
        ),
        format!(
            "self_evolution_admission_memory_store_write_allowed={}",
            report.memory_store_write_allowed
        ),
        format!(
            "self_evolution_admission_ndkv_write_allowed={}",
            report.ndkv_write_allowed
        ),
        format!(
            "self_evolution_admission_model_weight_write_allowed={}",
            report.model_weight_write_allowed
        ),
        format!(
            "self_evolution_admission_git_write_allowed={}",
            report.git_write_allowed
        ),
        format!(
            "self_evolution_admission_human_approval_required={}",
            report.human_approval_required
        ),
        format!(
            "self_evolution_admission_admitted_for_human_review={}",
            report.admitted_for_human_review
        ),
        format!(
            "self_evolution_admission_review_packet_ids={}",
            report.review_packet.approval_review_packet_ids.len()
        ),
        format!(
            "self_evolution_admission_review_packet_evidence_ids={}",
            report.review_packet.evidence_ids.len()
        ),
        format!(
            "self_evolution_admission_review_packet_rollback_anchor_ids={}",
            report.review_packet.rollback_anchor_ids.len()
        ),
        format!(
            "self_evolution_admission_review_packet_content_digests={}",
            report.review_packet.content_digests.len()
        ),
        format!(
            "self_evolution_admission_review_packet_source_report_schemas={}",
            report.review_packet.source_report_schemas.len()
        ),
        format!(
            "self_evolution_admission_rust_validation_passed={}",
            report.rust_validation_passed
        ),
        format!(
            "self_evolution_admission_rust_check_items={}",
            report.rust_check_items
        ),
        format!(
            "self_evolution_admission_rust_check_passed={}",
            report.rust_check_passed
        ),
        format!(
            "self_evolution_admission_rust_check_failed={}",
            report.rust_check_failed
        ),
        format!(
            "self_evolution_admission_validation_passed={}",
            report.validation_passed
        ),
        format!(
            "self_evolution_admission_compiler_validation={}/{}:{}",
            report.validation.compiler.passed,
            report.validation.compiler.items,
            report.validation.compiler.failed
        ),
        format!(
            "self_evolution_admission_test_validation={}/{}:{}",
            report.validation.tests.passed,
            report.validation.tests.items,
            report.validation.tests.failed
        ),
        format!(
            "self_evolution_admission_benchmark_validation={}/{}:{}",
            report.validation.benchmarks.passed,
            report.validation.benchmarks.items,
            report.validation.benchmarks.failed
        ),
        format!(
            "self_evolution_admission_experiment_validation={}/{}:{}",
            report.validation.experiments.passed,
            report.validation.experiments.items,
            report.validation.experiments.failed
        ),
        format!(
            "self_evolution_admission_benchmark_gate_passed={}",
            report.benchmark_gate_passed
        ),
        format!(
            "self_evolution_admission_benchmark_gate_failures={}",
            report.benchmark_gate_failures.len()
        ),
        format!(
            "self_evolution_admission_rollback_budget_clean={}",
            report.rollback_budget_clean
        ),
        format!(
            "self_evolution_admission_drift_rollbacks={}",
            report.drift_rollbacks
        ),
        format!(
            "self_evolution_admission_rollback_router_threshold_delta={:.6}",
            report.rollback_router_threshold_delta
        ),
        format!(
            "self_evolution_admission_rollback_hierarchy_weight_delta={:.6}",
            report.rollback_hierarchy_weight_delta
        ),
        format!(
            "self_evolution_admission_adaptive_preview_evidence={}",
            report.adaptive_preview_evidence_present
        ),
        format!(
            "self_evolution_admission_adaptive_preview_source_count={}",
            report.adaptive_preview_source_count
        ),
        format!(
            "self_evolution_admission_adaptive_preview_read_only={}",
            report.adaptive_preview_read_only
        ),
        format!(
            "self_evolution_admission_adaptive_preview_report_only={}",
            report.adaptive_preview_report_only
        ),
        format!(
            "self_evolution_admission_adaptive_preview_preview_only={}",
            report.adaptive_preview_preview_only
        ),
        format!(
            "self_evolution_admission_adaptive_preview_write_allowed={}",
            report.adaptive_preview_write_allowed
        ),
        format!(
            "self_evolution_admission_adaptive_preview_applied={}",
            report.adaptive_preview_applied
        ),
        format!(
            "self_evolution_admission_router_threshold_preview_ready={}",
            report.router_threshold_preview_ready
        ),
        format!(
            "self_evolution_admission_hierarchy_adjustment_preview_ready={}",
            report.hierarchy_adjustment_preview_ready
        ),
        format!(
            "self_evolution_admission_kv_fusion_policy_observation_preview_ready={}",
            report.kv_fusion_policy_observation_preview_ready
        ),
        format!(
            "self_evolution_admission_adaptive_preview_blocked_reasons={}",
            report.adaptive_preview_blocked_reasons.len()
        ),
        format!(
            "self_evolution_admission_blocked_reasons={}",
            report.blocked_reasons.len()
        ),
    ];
    telemetry.extend(
        report
            .blocked_reasons
            .iter()
            .map(|reason| format!("self_evolution_admission_blocked_reason={reason}")),
    );
    telemetry
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hierarchy::{HierarchyAdjustmentPreviewPlanner, HierarchyController, TaskProfile};
    use crate::router::{
        GenerationMetrics, NoironRouter, RouterThresholdAdjustmentPreviewPlanner,
        RouterThresholdAdjustmentPreviewReport,
    };

    fn passing_benchmark_gate() -> BenchmarkGateReport {
        BenchmarkGateReport {
            passed: true,
            failures: Vec::new(),
        }
    }

    fn passing_evolution_ledger() -> EvolutionLedger {
        EvolutionLedger {
            replay_rust_check_items: 2,
            replay_rust_check_passed: 2,
            replay_rust_check_failed: 0,
            ..EvolutionLedger::default()
        }
    }

    fn passing_validation_evidence() -> SelfEvolutionValidationEvidence {
        SelfEvolutionValidationEvidence::from_lanes(
            SelfEvolutionValidationLane::new(2, 2, 0),
            SelfEvolutionValidationLane::new(3, 3, 0),
            SelfEvolutionValidationLane::new(1, 1, 0),
            SelfEvolutionValidationLane::new(1, 1, 0),
        )
    }

    fn promotion_artifact(label: &str) -> SelfEvolutionPromotionArtifactRef {
        SelfEvolutionPromotionArtifactRef::trace(
            label,
            format!("trace:{label}"),
            self_evolution_stable_digest(label),
        )
    }

    fn passing_promotion_candidate(
        lane: SelfEvolutionPromotionLane,
    ) -> SelfEvolutionPromotionCandidate {
        SelfEvolutionPromotionCandidate::new("promotion-candidate", lane)
            .with_correctness_delta(0.042)
            .with_latency_delta_ms(-8)
            .with_wasted_compute_delta(-0.03)
            .with_privacy_risk(0.0)
            .with_reproducible_runs(3)
            .with_cross_task_regression(0.0)
            .with_flaky_runs(0)
            .with_rollback("rollback:promotion-candidate")
            .with_validation(passing_validation_evidence())
            .with_artifact_ref(promotion_artifact("cargo-check"))
            .with_artifact_ref(promotion_artifact("focused-tests"))
            .with_artifact_ref(promotion_artifact("benchmark-gate"))
    }

    fn safe_router_threshold_preview() -> RouterThresholdAdjustmentPreviewReport {
        RouterThresholdAdjustmentPreviewPlanner::new().preview(
            NoironRouter::new().state(),
            TaskProfile::Coding,
            GenerationMetrics {
                perplexity: 36.0,
                semantic_consistency: 0.20,
                contradiction_count: 2,
                token_count: 64,
            },
        )
    }

    fn passing_admission_report(candidate_id: &str) -> SelfEvolutionAdmissionReport {
        let router_preview = safe_router_threshold_preview();
        let evidence = SelfEvolutionAdmissionEvidence::from_benchmark_gate(
            candidate_id,
            passing_evolution_ledger(),
            &passing_benchmark_gate(),
        )
        .with_validation_evidence(passing_validation_evidence())
        .with_router_threshold_preview_report(&router_preview);

        SelfEvolutionAdmissionGate::new().evaluate(&evidence)
    }

    fn hold_admission_report(candidate_id: &str) -> SelfEvolutionAdmissionReport {
        let evidence = SelfEvolutionAdmissionEvidence::from_benchmark_gate(
            candidate_id,
            passing_evolution_ledger(),
            &passing_benchmark_gate(),
        )
        .with_validation_evidence(passing_validation_evidence());

        SelfEvolutionAdmissionGate::new().evaluate(&evidence)
    }

    fn reject_admission_report(candidate_id: &str) -> SelfEvolutionAdmissionReport {
        let evidence = SelfEvolutionAdmissionEvidence::from_benchmark_gate(
            candidate_id,
            EvolutionLedger {
                replay_rust_check_items: 1,
                replay_rust_check_passed: 0,
                replay_rust_check_failed: 1,
                ..EvolutionLedger::default()
            },
            &BenchmarkGateReport {
                passed: false,
                failures: vec!["compiler validation failed".to_owned()],
            },
        )
        .with_validation_evidence(SelfEvolutionValidationEvidence::from_lanes(
            SelfEvolutionValidationLane::new(1, 0, 1),
            SelfEvolutionValidationLane::new(1, 0, 1),
            SelfEvolutionValidationLane::new(1, 0, 1),
            SelfEvolutionValidationLane::new(1, 0, 1),
        ));

        SelfEvolutionAdmissionGate::new().evaluate(&evidence)
    }

    fn rollback_admission_report(candidate_id: &str) -> SelfEvolutionAdmissionReport {
        let router_preview = safe_router_threshold_preview();
        let evidence = SelfEvolutionAdmissionEvidence::from_benchmark_gate(
            candidate_id,
            EvolutionLedger {
                replay_rust_check_items: 2,
                replay_rust_check_passed: 2,
                replay_rust_check_failed: 0,
                drift_rollbacks: 1,
                rollback_router_threshold_delta: 0.02,
                rollback_hierarchy_weight_delta: 0.03,
                ..EvolutionLedger::default()
            },
            &passing_benchmark_gate(),
        )
        .with_validation_evidence(passing_validation_evidence())
        .with_router_threshold_preview_report(&router_preview);

        SelfEvolutionAdmissionGate::new().evaluate(&evidence)
    }

    fn approved_rollback_replay_gate_and_approval() -> (
        SelfEvolutionRollbackReplayGateReport,
        SelfEvolutionOperatorApprovalReport,
    ) {
        let mut experiment_ledger = SelfEvolutionExperimentLedger::new();
        experiment_ledger.append_admission_report(
            "experiment-rollback",
            &rollback_admission_report("candidate-rollback"),
        );
        let plan = experiment_ledger.rollback_replay_plan();
        let rollback_gate = SelfEvolutionRollbackReplayGate::new().evaluate(&plan);
        let evidence = SelfEvolutionOperatorApprovalEvidence::from_review_packet(
            "maintainer-jy",
            "approval-ticket-rollback-apply",
            &rollback_gate.review_packet,
            "approved for rollback replay apply preflight",
        );
        let approval = SelfEvolutionOperatorApprovalGate::new()
            .evaluate(&rollback_gate.review_packet, &evidence);

        assert!(rollback_gate.admitted_for_human_review);
        assert!(approval.operator_approved);
        (rollback_gate, approval)
    }

    fn approved_promotion_preflight_inputs(
        candidate_id: &str,
    ) -> (
        SelfEvolutionAdmissionReport,
        SelfEvolutionExperimentRecord,
        SelfEvolutionOperatorApprovalReport,
    ) {
        let admission = passing_admission_report(candidate_id);
        let mut experiment_ledger = SelfEvolutionExperimentLedger::new();
        let experiment =
            experiment_ledger.append_admission_report("promotion-experiment", &admission);
        let approval_evidence = SelfEvolutionOperatorApprovalEvidence::from_review_packet(
            "maintainer-jy",
            "approval-ticket-promotion",
            &admission.review_packet,
            "approved admission packet for promotion preflight",
        );
        let approval = SelfEvolutionOperatorApprovalGate::new()
            .evaluate(&admission.review_packet, &approval_evidence);

        assert!(admission.admitted_for_human_review);
        assert_eq!(
            experiment.decision,
            SelfEvolutionExperimentDecision::AdmitForHumanReview
        );
        assert!(approval.operator_approved);
        (admission, experiment, approval)
    }

    #[test]
    fn promotion_scorecard_promotes_digest_only_candidate_for_human_approval() {
        let candidate = passing_promotion_candidate(SelfEvolutionPromotionLane::Memory);

        let scorecard = SelfEvolutionPromotionScorecardGate::new().evaluate(&candidate);

        assert_eq!(
            scorecard.decision,
            SelfEvolutionPromotionDecision::PromoteForApproval
        );
        assert!(scorecard.ready_for_human_approval);
        assert!(scorecard.human_approval_required);
        assert!(scorecard.validation_passed);
        assert_eq!(scorecard.artifact_refs.len(), 3);
        assert!(scorecard.artifact_refs.iter().all(|artifact| {
            digest_or_trace_like(&artifact.content_digest)
                && artifact.trace_id.as_deref().is_some_and(trace_id_like)
        }));
        assert!(scorecard.read_only);
        assert!(scorecard.report_only);
        assert!(scorecard.preview_only);
        assert!(!scorecard.write_allowed);
        assert!(!scorecard.applied);
        assert!(scorecard.summary_line().contains("write_allowed=false"));
        assert!(scorecard.review_packet_line().contains("artifact_digests="));
        assert!(!contains_private_or_executable_marker(
            &scorecard.review_packet_line()
        ));
    }

    #[test]
    fn promotion_scorecard_holds_failed_validation_with_artifacts_present() {
        let candidate = passing_promotion_candidate(SelfEvolutionPromotionLane::ToolPolicy)
            .with_validation(SelfEvolutionValidationEvidence::from_lanes(
                SelfEvolutionValidationLane::new(1, 1, 0),
                SelfEvolutionValidationLane::new(2, 1, 1),
                SelfEvolutionValidationLane::new(1, 1, 0),
                SelfEvolutionValidationLane::default(),
            ));

        let scorecard = SelfEvolutionPromotionScorecardGate::new().evaluate(&candidate);

        assert_eq!(
            scorecard.decision,
            SelfEvolutionPromotionDecision::HoldForEvidence
        );
        assert!(!scorecard.ready_for_human_approval);
        assert!(!scorecard.validation_passed);
        assert!(
            scorecard
                .blocked_reasons
                .iter()
                .any(|reason| { reason == "promotion_tests_validation_failed:1" })
        );
        assert!(!scorecard.write_allowed);
    }

    #[test]
    fn promotion_scorecard_reports_insufficient_evidence_before_review() {
        let candidate = SelfEvolutionPromotionCandidate::new(
            "thin-candidate",
            SelfEvolutionPromotionLane::Routing,
        )
        .with_correctness_delta(0.05)
        .with_reproducible_runs(1)
        .with_rollback("rollback:thin-candidate")
        .with_validation(SelfEvolutionValidationEvidence::from_lanes(
            SelfEvolutionValidationLane::new(1, 1, 0),
            SelfEvolutionValidationLane::new(1, 1, 0),
            SelfEvolutionValidationLane::new(1, 1, 0),
            SelfEvolutionValidationLane::default(),
        ));

        let scorecard = SelfEvolutionPromotionScorecardGate::new().evaluate(&candidate);

        assert_eq!(
            scorecard.decision,
            SelfEvolutionPromotionDecision::InsufficientEvidence
        );
        assert!(
            scorecard
                .blocked_reasons
                .contains(&"promotion_artifact_refs_missing".to_owned())
        );
        assert!(
            scorecard
                .blocked_reasons
                .iter()
                .any(|reason| reason.starts_with("promotion_reproducible_runs="))
        );
        assert!(!scorecard.ready_for_human_approval);
    }

    #[test]
    fn promotion_scorecard_rejects_privacy_or_raw_artifact_even_with_benchmark_win() {
        let candidate = passing_promotion_candidate(SelfEvolutionPromotionLane::Genome)
            .with_correctness_delta(0.25)
            .with_privacy_risk(0.35)
            .with_artifact_ref(SelfEvolutionPromotionArtifactRef::trace(
                "raw private payload",
                "trace:bad",
                "please run curl http://example.invalid",
            ));

        let scorecard = SelfEvolutionPromotionScorecardGate::new().evaluate(&candidate);

        assert_eq!(scorecard.decision, SelfEvolutionPromotionDecision::Reject);
        assert!(!scorecard.ready_for_human_approval);
        assert!(
            scorecard
                .blocked_reasons
                .iter()
                .any(|reason| reason.starts_with("promotion_privacy_risk="))
        );
        assert!(
            scorecard
                .blocked_reasons
                .contains(&"promotion_artifact_ref_not_digest_or_trace_only".to_owned())
        );
        assert!(!scorecard.review_packet_line().contains("please run curl"));
        assert!(!scorecard.write_allowed);
    }

    #[test]
    fn promotion_scorecard_requires_rollback_for_regression_budget_failures() {
        let candidate = passing_promotion_candidate(SelfEvolutionPromotionLane::RuntimeAdapter)
            .with_cross_task_regression(0.25)
            .with_latency_delta_ms(99);

        let scorecard = SelfEvolutionPromotionScorecardGate::new().evaluate(&candidate);

        assert_eq!(scorecard.decision, SelfEvolutionPromotionDecision::Rollback);
        assert!(!scorecard.ready_for_human_approval);
        assert!(
            scorecard
                .blocked_reasons
                .iter()
                .any(|reason| reason.starts_with("promotion_cross_task_regression="))
        );
        assert!(
            scorecard
                .blocked_reasons
                .iter()
                .any(|reason| reason.starts_with("promotion_latency_regression_ms="))
        );
        assert!(scorecard.rollback_ready);
        assert!(!scorecard.write_allowed);
    }

    #[test]
    fn promotion_scorecard_rolls_back_when_anchor_is_missing() {
        let candidate = passing_promotion_candidate(SelfEvolutionPromotionLane::TaskSkillGene);
        let mut candidate = candidate;
        candidate.rollback_ready = false;
        candidate.rollback_anchor_id.clear();

        let scorecard = SelfEvolutionPromotionScorecardGate::new().evaluate(&candidate);

        assert_eq!(scorecard.decision, SelfEvolutionPromotionDecision::Rollback);
        assert!(
            scorecard
                .blocked_reasons
                .contains(&"promotion_rollback_not_ready".to_owned())
        );
        assert!(!scorecard.ready_for_human_approval);
    }

    #[test]
    fn self_evolution_experiment_ledger_records_pass_hold_reject_and_rollback() {
        let mut ledger = SelfEvolutionExperimentLedger::new();

        let admitted = ledger.append_admission_report(
            "experiment-pass",
            &passing_admission_report("candidate-pass"),
        );
        let held =
            ledger.append_admission_report("experiment-hold", &hold_admission_report("hold"));
        let rejected =
            ledger.append_admission_report("experiment-reject", &reject_admission_report("reject"));
        let rollback = ledger.append_admission_report(
            "experiment-rollback",
            &rollback_admission_report("rollback"),
        );

        assert_eq!(
            admitted.decision,
            SelfEvolutionExperimentDecision::AdmitForHumanReview
        );
        assert_eq!(held.decision, SelfEvolutionExperimentDecision::Hold);
        assert_eq!(rejected.decision, SelfEvolutionExperimentDecision::Reject);
        assert_eq!(rollback.decision, SelfEvolutionExperimentDecision::Rollback);
        assert_eq!(admitted.sequence, 1);
        assert_eq!(held.sequence, 2);
        assert_eq!(rejected.sequence, 3);
        assert_eq!(rollback.sequence, 4);
        assert_eq!(ledger.records().len(), 4);
        assert_eq!(ledger.admitted_for_review(), 1);
        assert_eq!(ledger.held(), 1);
        assert_eq!(ledger.rejected(), 1);
        assert_eq!(ledger.rollback_required(), 1);
        assert!(rollback.rollback_required);
        assert!(rollback.rollback_replayable);
        assert!(!admitted.active_candidate);
        assert!(!admitted.write_allowed);
        assert!(!admitted.applied);
        assert!(admitted.human_approval_required);
        assert!(admitted.read_only);
        assert!(admitted.report_only);
        assert!(admitted.preview_only);
        assert!(!admitted.evidence_ids.is_empty());
        assert!(!admitted.rollback_anchor_ids.is_empty());
        assert!(
            held.blocked_reasons
                .contains(&"self_evolution_admission_adaptive_preview_evidence_missing".to_owned())
        );
        assert!(
            rejected
                .blocked_reasons
                .iter()
                .any(|reason| reason.contains("validation_failed"))
        );
        assert!(
            rollback
                .blocked_reasons
                .iter()
                .any(|reason| reason.contains("rollback_router_threshold_delta"))
        );
        assert!(
            ledger
                .summary_line()
                .contains("self_evolution_experiment_ledger records=4")
        );
        assert!(
            admitted
                .summary_line()
                .contains("decision=admit_for_human_review")
        );
        assert!(
            admitted
                .json_line()
                .contains("\"schema\":\"rust-norion-self-evolution-experiment-v1\"")
        );
        assert!(admitted.json_line().contains("\"active_candidate\":false"));
        assert!(admitted.json_line().contains("\"write_allowed\":false"));
    }

    #[test]
    fn self_evolution_experiment_ledger_marks_repeated_experiments_append_only() {
        let mut ledger = SelfEvolutionExperimentLedger::new();
        let first =
            ledger.append_admission_report("repeat-me", &passing_admission_report("repeat-a"));
        let first_snapshot = first.clone();
        let second =
            ledger.append_admission_report("repeat-me", &passing_admission_report("repeat-b"));

        assert!(!first.repeated_experiment);
        assert!(second.repeated_experiment);
        assert_eq!(second.sequence, 2);
        assert_eq!(ledger.records().len(), 2);
        assert_eq!(ledger.repeated_experiments(), 1);
        assert_eq!(ledger.records()[0], first_snapshot);
        assert_ne!(
            ledger.records()[0].content_digest,
            ledger.records()[1].content_digest
        );
        assert!(ledger.summary_line().contains("repeated_experiments=1"));
    }

    #[test]
    fn self_evolution_experiment_ledger_holds_conflicting_evidence() {
        let mut report = passing_admission_report("conflicting");
        report
            .benchmark_gate_failures
            .push("benchmark regression despite passed gate".to_owned());
        report.validation.compiler = SelfEvolutionValidationLane::new(1, 1, 1);

        let mut ledger = SelfEvolutionExperimentLedger::new();
        let record = ledger.append_admission_report("conflicting", &report);

        assert!(record.conflicting_evidence);
        assert_eq!(record.decision, SelfEvolutionExperimentDecision::Hold);
        assert!(!record.active_candidate);
        assert!(!record.write_allowed);
        assert_eq!(ledger.conflicting_evidence(), 1);
        assert!(ledger.summary_line().contains("conflicting_evidence=1"));
    }

    #[test]
    fn self_evolution_rollback_replay_plan_extracts_safe_rollback_records() {
        let mut ledger = SelfEvolutionExperimentLedger::new();
        ledger.append_admission_report(
            "experiment-pass",
            &passing_admission_report("candidate-pass"),
        );
        let rollback = ledger.append_admission_report(
            "experiment-rollback",
            &rollback_admission_report("candidate-rollback"),
        );
        let before = ledger.records().to_vec();

        let plan = ledger.rollback_replay_plan();

        assert_eq!(ledger.records(), before.as_slice());
        assert_eq!(plan.item_count(), 1);
        assert_eq!(plan.replayable(), 1);
        assert_eq!(plan.blocked(), 0);
        assert!(plan.all_replayable());
        assert!(plan.read_only);
        assert!(plan.report_only);
        assert!(plan.preview_only);
        assert!(!plan.write_allowed);
        assert!(!plan.applied);
        assert_eq!(plan.rollback_anchor_ids(), rollback.rollback_anchor_ids);
        assert_eq!(plan.evidence_ids(), rollback.evidence_ids);

        let item = &plan.items[0];
        assert_eq!(item.sequence, rollback.sequence);
        assert_eq!(item.decision, SelfEvolutionExperimentDecision::Rollback);
        assert!(item.rollback_required);
        assert!(item.rollback_replayable);
        assert!(item.replayable);
        assert!(item.blocked_reasons.is_empty());
        assert!(item.summary_line().contains("replayable=true"));
        assert!(
            item.json_line()
                .contains("\"schema\":\"rust-norion-self-evolution-rollback-replay-item-v1\"")
        );
        assert!(
            plan.summary_line()
                .contains("self_evolution_rollback_replay_plan items=1")
        );
        assert!(
            plan.json_line()
                .contains("\"schema\":\"rust-norion-self-evolution-rollback-replay-plan-v1\"")
        );
        assert!(plan.json_line().contains("\"write_allowed\":false"));
        assert!(plan.json_line().contains("\"applied\":false"));
    }

    #[test]
    fn self_evolution_rollback_replay_plan_blocks_missing_evidence_or_anchor() {
        let mut ledger = SelfEvolutionExperimentLedger::new();
        ledger.append_admission_report(
            "experiment-rollback",
            &rollback_admission_report("candidate-rollback"),
        );
        ledger.records[0].evidence_ids.clear();
        ledger.records[0].rollback_anchor_ids.clear();

        let plan = ledger.rollback_replay_plan();

        assert_eq!(plan.item_count(), 1);
        assert_eq!(plan.replayable(), 0);
        assert_eq!(plan.blocked(), 1);
        assert!(!plan.all_replayable());
        assert!(plan.rollback_anchor_ids().is_empty());
        assert!(plan.evidence_ids().is_empty());
        assert!(
            plan.items[0]
                .blocked_reasons
                .contains(&"self_evolution_rollback_replay_evidence_missing".to_owned())
        );
        assert!(
            plan.items[0]
                .blocked_reasons
                .contains(&"self_evolution_rollback_replay_anchor_missing".to_owned())
        );
    }

    #[test]
    fn self_evolution_rollback_replay_plan_blocks_unsafe_record_state() {
        let mut ledger = SelfEvolutionExperimentLedger::new();
        ledger.append_admission_report(
            "experiment-rollback",
            &rollback_admission_report("candidate-rollback"),
        );
        ledger.records[0].active_candidate = true;
        ledger.records[0].read_only = false;
        ledger.records[0].report_only = false;
        ledger.records[0].preview_only = false;
        ledger.records[0].write_allowed = true;
        ledger.records[0].applied = true;

        let plan = ledger.rollback_replay_plan();

        assert_eq!(plan.item_count(), 1);
        assert_eq!(plan.replayable(), 0);
        assert_eq!(plan.blocked(), 1);
        for reason in [
            "self_evolution_rollback_replay_active_candidate",
            "self_evolution_rollback_replay_not_read_only",
            "self_evolution_rollback_replay_not_report_only",
            "self_evolution_rollback_replay_not_preview_only",
            "self_evolution_rollback_replay_write_allowed",
            "self_evolution_rollback_replay_already_applied",
        ] {
            assert!(
                plan.items[0].blocked_reasons.contains(&reason.to_owned()),
                "missing rollback replay blocked reason {reason}: {:?}",
                plan.items[0].blocked_reasons
            );
        }
        assert!(plan.read_only);
        assert!(plan.report_only);
        assert!(plan.preview_only);
        assert!(!plan.write_allowed);
        assert!(!plan.applied);
    }

    #[test]
    fn self_evolution_rollback_replay_plan_ignores_non_rollback_records() {
        let mut ledger = SelfEvolutionExperimentLedger::new();
        ledger.append_admission_report(
            "experiment-pass",
            &passing_admission_report("candidate-pass"),
        );
        ledger.append_admission_report("experiment-hold", &hold_admission_report("candidate-hold"));
        ledger.append_admission_report(
            "experiment-reject",
            &reject_admission_report("candidate-reject"),
        );

        let plan = ledger.rollback_replay_plan();

        assert_eq!(ledger.records().len(), 3);
        assert_eq!(plan.item_count(), 0);
        assert_eq!(plan.replayable(), 0);
        assert_eq!(plan.blocked(), 0);
        assert!(plan.all_replayable());
        assert!(plan.rollback_anchor_ids().is_empty());
        assert!(plan.evidence_ids().is_empty());
        assert!(
            plan.summary_line()
                .contains("self_evolution_rollback_replay_plan items=0")
        );
    }

    #[test]
    fn self_evolution_rollback_replay_gate_admits_safe_plan_for_human_review() {
        let mut ledger = SelfEvolutionExperimentLedger::new();
        let rollback = ledger.append_admission_report(
            "experiment-rollback",
            &rollback_admission_report("candidate-rollback"),
        );
        let plan = ledger.rollback_replay_plan();
        let plan_before = plan.clone();

        let report = SelfEvolutionRollbackReplayGate::new().evaluate(&plan);

        assert_eq!(plan, plan_before);
        assert_eq!(
            report.decision,
            SelfEvolutionRollbackReplayDecision::AdmitForHumanReview
        );
        assert!(report.admitted_for_human_review);
        assert!(report.human_approval_required);
        assert!(report.read_only);
        assert!(report.report_only);
        assert!(report.preview_only);
        assert!(!report.write_allowed);
        assert!(!report.applied);
        assert!(report.plan_read_only);
        assert!(report.plan_report_only);
        assert!(report.plan_preview_only);
        assert!(!report.plan_write_allowed);
        assert!(!report.plan_applied);
        assert_eq!(report.item_count, 1);
        assert_eq!(report.replayable, 1);
        assert_eq!(report.blocked, 0);
        assert!(report.all_replayable);
        assert_eq!(report.rollback_anchor_ids, rollback.rollback_anchor_ids);
        assert_eq!(report.evidence_ids, rollback.evidence_ids);
        assert!(!report.review_packet.approval_review_packet_ids.is_empty());
        assert!(!report.review_packet.evidence_ids.is_empty());
        assert_eq!(
            report.review_packet.rollback_anchor_ids,
            report.rollback_anchor_ids
        );
        assert!(
            report
                .review_packet
                .content_digests
                .iter()
                .any(|digest| digest == &report.content_digest)
        );
        assert!(
            report
                .review_packet
                .source_report_schemas
                .iter()
                .any(|schema| schema == "rust-norion-self-evolution-rollback-replay-gate-v1")
        );
        assert!(report.summary_line().contains("review_packets=1"));
        assert!(report.blocked_reasons.is_empty());
        assert!(report.content_digest.starts_with("fnv64:"));
        assert!(
            report
                .summary_line()
                .contains("decision=admit_for_human_review")
        );
        assert!(
            report
                .json_line()
                .contains("\"schema\":\"rust-norion-self-evolution-rollback-replay-gate-v1\"")
        );
        assert!(
            report
                .json_line()
                .contains("\"admitted_for_human_review\":true")
        );
        assert!(report.json_line().contains("\"review_packet\":"));
        assert!(
            report
                .json_line()
                .contains("\"approval_tokens_included\":false")
        );
        assert!(plan.read_only);
        assert!(plan.report_only);
        assert!(plan.preview_only);
        assert!(!plan.write_allowed);
        assert!(!plan.applied);
    }

    #[test]
    fn self_evolution_rollback_replay_gate_holds_empty_plan() {
        let plan = SelfEvolutionRollbackReplayPlan::new(Vec::new());

        let report = SelfEvolutionRollbackReplayGate::new().evaluate(&plan);

        assert_eq!(report.decision, SelfEvolutionRollbackReplayDecision::Hold);
        assert!(!report.admitted_for_human_review);
        assert!(report.human_approval_required);
        assert_eq!(report.item_count, 0);
        assert_eq!(report.replayable, 0);
        assert_eq!(report.blocked, 0);
        assert!(report.all_replayable);
        assert!(report.rollback_anchor_ids.is_empty());
        assert!(report.evidence_ids.is_empty());
        assert!(!report.review_packet.approval_review_packet_ids.is_empty());
        assert!(report.review_packet.rollback_anchor_ids.is_empty());
        assert!(
            report
                .review_packet
                .content_digests
                .iter()
                .any(|digest| digest == &report.content_digest)
        );
        assert!(
            report
                .json_line()
                .contains("\"approval_tokens_included\":false")
        );
        assert!(
            report
                .blocked_reasons
                .contains(&"self_evolution_rollback_replay_gate_empty_plan".to_owned())
        );
        assert!(report.read_only);
        assert!(report.report_only);
        assert!(report.preview_only);
        assert!(!report.write_allowed);
        assert!(!report.applied);
    }

    #[test]
    fn self_evolution_rollback_replay_gate_blocks_missing_evidence_or_anchor() {
        let mut ledger = SelfEvolutionExperimentLedger::new();
        ledger.append_admission_report(
            "experiment-rollback",
            &rollback_admission_report("candidate-rollback"),
        );
        ledger.records[0].evidence_ids.clear();
        ledger.records[0].rollback_anchor_ids.clear();
        let plan = ledger.rollback_replay_plan();

        let report = SelfEvolutionRollbackReplayGate::new().evaluate(&plan);

        assert_eq!(report.decision, SelfEvolutionRollbackReplayDecision::Hold);
        assert!(!report.admitted_for_human_review);
        assert_eq!(report.item_count, 1);
        assert_eq!(report.replayable, 0);
        assert_eq!(report.blocked, 1);
        assert!(!report.all_replayable);
        assert!(report.rollback_anchor_ids.is_empty());
        assert!(report.evidence_ids.is_empty());
        for reason in [
            "self_evolution_rollback_replay_gate_blocked_items",
            "self_evolution_rollback_replay_gate_rollback_anchor_ids_missing",
            "self_evolution_rollback_replay_gate_evidence_ids_missing",
        ] {
            assert!(
                report.blocked_reasons.contains(&reason.to_owned()),
                "missing rollback replay gate blocked reason {reason}: {:?}",
                report.blocked_reasons
            );
        }
        assert!(report.json_line().contains("\"decision\":\"hold\""));
    }

    #[test]
    fn self_evolution_rollback_replay_gate_blocks_active_write_or_applied_state() {
        let mut ledger = SelfEvolutionExperimentLedger::new();
        ledger.append_admission_report(
            "experiment-rollback",
            &rollback_admission_report("candidate-rollback"),
        );
        ledger.records[0].active_candidate = true;
        ledger.records[0].write_allowed = true;
        ledger.records[0].applied = true;
        let mut plan = ledger.rollback_replay_plan();
        plan.read_only = false;
        plan.report_only = false;
        plan.preview_only = false;
        plan.write_allowed = true;
        plan.applied = true;

        let report = SelfEvolutionRollbackReplayGate::new().evaluate(&plan);

        assert_eq!(report.decision, SelfEvolutionRollbackReplayDecision::Hold);
        assert!(!report.admitted_for_human_review);
        assert_eq!(report.active_candidates, 1);
        assert_eq!(report.item_write_allowed, 1);
        assert_eq!(report.item_applied, 1);
        assert!(!report.plan_read_only);
        assert!(!report.plan_report_only);
        assert!(!report.plan_preview_only);
        assert!(report.plan_write_allowed);
        assert!(report.plan_applied);
        for reason in [
            "self_evolution_rollback_replay_gate_blocked_items",
            "self_evolution_rollback_replay_gate_active_candidates=1>0",
            "self_evolution_rollback_replay_gate_item_write_allowed=1>0",
            "self_evolution_rollback_replay_gate_item_applied=1>0",
            "self_evolution_rollback_replay_gate_plan_not_read_only",
            "self_evolution_rollback_replay_gate_plan_not_report_only",
            "self_evolution_rollback_replay_gate_plan_not_preview_only",
            "self_evolution_rollback_replay_gate_plan_write_allowed",
            "self_evolution_rollback_replay_gate_plan_applied",
        ] {
            assert!(
                report.blocked_reasons.contains(&reason.to_owned()),
                "missing rollback replay gate blocked reason {reason}: {:?}",
                report.blocked_reasons
            );
        }
        assert!(report.read_only);
        assert!(report.report_only);
        assert!(report.preview_only);
        assert!(!report.write_allowed);
        assert!(!report.applied);
    }

    #[test]
    fn self_evolution_operator_approval_gate_approves_review_packet_without_activation() {
        let mut ledger = SelfEvolutionExperimentLedger::new();
        ledger.append_admission_report(
            "experiment-rollback",
            &rollback_admission_report("candidate-rollback"),
        );
        let plan = ledger.rollback_replay_plan();
        let gate_report = SelfEvolutionRollbackReplayGate::new().evaluate(&plan);
        let evidence = SelfEvolutionOperatorApprovalEvidence::from_review_packet(
            "maintainer-jy",
            "approval-ticket-001",
            &gate_report.review_packet,
            "reviewed rollback replay packet and approved candidate for the next gated stage",
        );

        let report = SelfEvolutionOperatorApprovalGate::new()
            .evaluate(&gate_report.review_packet, &evidence);

        assert_eq!(
            report.decision,
            SelfEvolutionOperatorApprovalDecision::Approved
        );
        assert!(report.operator_approved);
        assert_eq!(report.operator_id, "maintainer-jy");
        assert_eq!(report.approval_ticket_id, "approval-ticket-001");
        assert_eq!(
            report.approved_review_packet_ids,
            gate_report.review_packet.approval_review_packet_ids
        );
        let approval_packet = TenantScopedKey::parse(&report.approved_review_packet_ids[0])
            .expect("scoped approval packet id");
        assert_eq!(approval_packet.lane, TenantResourceLane::ApprovalPacket);
        assert_eq!(approval_packet.scope, TenantScope::local_single_user());
        assert_eq!(
            report.approved_evidence_ids,
            gate_report.review_packet.evidence_ids
        );
        assert_eq!(
            report.approved_rollback_anchor_ids,
            gate_report.review_packet.rollback_anchor_ids
        );
        assert!(report.approval_attestation_digest.starts_with("fnv64:"));
        assert!(report.read_only);
        assert!(report.report_only);
        assert!(report.preview_only);
        assert!(!report.activation_write_allowed);
        assert!(!report.active_candidate);
        assert!(!report.write_allowed);
        assert!(!report.applied);
        assert!(report.blocked_reasons.is_empty());
        assert!(report.content_digest.starts_with("fnv64:"));
        assert!(
            report
                .summary_line()
                .contains("self_evolution_operator_approval decision=approved")
        );
        assert!(
            report
                .json_line()
                .contains("\"schema\":\"rust-norion-self-evolution-operator-approval-v1\"")
        );
        assert!(report.json_line().contains("\"operator_approved\":true"));
        assert!(
            report
                .json_line()
                .contains("\"activation_write_allowed\":false")
        );
        assert!(!report.summary_line().contains("maintainer-jy"));
        assert!(!report.summary_line().contains("approval-ticket-001"));
        assert!(!report.json_line().contains("maintainer-jy"));
        assert!(!report.json_line().contains("approval-ticket-001"));
        assert!(
            !report
                .json_line()
                .contains("reviewed rollback replay packet")
        );
        assert!(report.json_line().contains("\"operator_digest\":\"fnv64:"));
        assert!(
            report
                .json_line()
                .contains("\"approval_reason_digest\":\"fnv64:")
        );
    }

    #[test]
    fn self_evolution_operator_approval_gate_holds_missing_or_mismatched_refs() {
        let mut ledger = SelfEvolutionExperimentLedger::new();
        ledger.append_admission_report(
            "experiment-rollback",
            &rollback_admission_report("candidate-rollback"),
        );
        let plan = ledger.rollback_replay_plan();
        let gate_report = SelfEvolutionRollbackReplayGate::new().evaluate(&plan);
        let mut evidence = SelfEvolutionOperatorApprovalEvidence::from_review_packet(
            "",
            "",
            &gate_report.review_packet,
            "",
        );
        evidence.approved_review_packet_ids.clear();
        evidence.approved_evidence_ids.clear();
        evidence.approved_content_digests = vec!["wrong-digest".to_owned()];
        evidence.approval_attestation_digest = "not-a-digest".to_owned();

        let report = SelfEvolutionOperatorApprovalGate::new()
            .evaluate(&gate_report.review_packet, &evidence);

        assert_eq!(report.decision, SelfEvolutionOperatorApprovalDecision::Hold);
        assert!(!report.operator_approved);
        assert!(!report.activation_write_allowed);
        assert!(!report.active_candidate);
        assert!(!report.write_allowed);
        assert!(!report.applied);
        for reason in [
            "self_evolution_operator_approval_operator_id_empty",
            "self_evolution_operator_approval_ticket_id_empty",
            "self_evolution_operator_approval_reason_empty",
            "self_evolution_operator_approval_attestation_digest_invalid",
            "self_evolution_operator_approval_approved_review_packet_ids_empty",
            "self_evolution_operator_approval_approved_evidence_ids_empty",
        ] {
            assert!(
                report.blocked_reasons.contains(&reason.to_owned()),
                "missing operator approval blocked reason {reason}: {:?}",
                report.blocked_reasons
            );
        }
        assert!(report.blocked_reasons.iter().any(|reason| {
            reason.starts_with("self_evolution_operator_approval_missing_content_digests=")
        }));
        assert!(report.json_line().contains("\"decision\":\"hold\""));
    }

    #[test]
    fn self_evolution_operator_approval_gate_rejects_cross_tenant_approval_packet_scope() {
        let tenant_a = TenantScope::new("tenant-a", "workspace", "session-a");
        let tenant_b = TenantScope::new("tenant-b", "workspace", "session-b");
        let mut review_packet = SelfEvolutionAdmissionReviewPacketRefs::default();
        review_packet.push_scoped_approval_review_packet_id(&tenant_a, "approval-review:tenant-a");
        review_packet.push_evidence_id("evidence:tenant-a");
        review_packet.push_rollback_anchor_id("rollback:tenant-a");
        review_packet.push_content_digest("fnv64:tenant-a");
        review_packet.push_source_report_schema("rust-norion-self-evolution-admission-v1");
        let evidence = SelfEvolutionOperatorApprovalEvidence::from_review_packet(
            "maintainer-jy",
            "approval-ticket-tenant-b",
            &review_packet,
            "approved packet from the wrong tenant scope",
        );

        let report = SelfEvolutionOperatorApprovalGate::new().evaluate_for_scope(
            &tenant_b,
            &review_packet,
            &evidence,
        );

        assert_eq!(report.decision, SelfEvolutionOperatorApprovalDecision::Hold);
        assert!(!report.operator_approved);
        assert!(report.blocked_reasons.contains(
            &"self_evolution_operator_approval_review_packet_ids_scope_rejected".to_owned()
        ));
        assert!(
            report.blocked_reasons.contains(
                &"self_evolution_operator_approval_approved_review_packet_ids_scope_rejected"
                    .to_owned()
            )
        );
        assert!(!report.summary_line().contains("tenant-a"));
        assert!(!report.json_line().contains("tenant-a"));
    }

    #[test]
    fn self_evolution_operator_approval_gate_holds_unexpected_extra_refs() {
        let mut ledger = SelfEvolutionExperimentLedger::new();
        ledger.append_admission_report(
            "experiment-rollback",
            &rollback_admission_report("candidate-rollback"),
        );
        let plan = ledger.rollback_replay_plan();
        let gate_report = SelfEvolutionRollbackReplayGate::new().evaluate(&plan);
        let mut evidence = SelfEvolutionOperatorApprovalEvidence::from_review_packet(
            "maintainer-jy",
            "approval-ticket-extra-refs",
            &gate_report.review_packet,
            "approved packet with extra unrelated refs should not pass",
        );
        evidence
            .approved_review_packet_ids
            .push("review-packet:unrelated".to_owned());
        evidence
            .approved_evidence_ids
            .push("evidence:unrelated".to_owned());
        evidence
            .approved_rollback_anchor_ids
            .push("rollback-anchor:unrelated".to_owned());
        evidence
            .approved_content_digests
            .push("fnv64:unrelated".to_owned());
        evidence
            .approved_source_report_schemas
            .push("rust-norion-unrelated-schema-v1".to_owned());
        evidence.approval_attestation_digest = evidence.expected_attestation_digest();

        let report = SelfEvolutionOperatorApprovalGate::new()
            .evaluate(&gate_report.review_packet, &evidence);

        assert_eq!(report.decision, SelfEvolutionOperatorApprovalDecision::Hold);
        assert!(!report.operator_approved);
        for reason in [
            "self_evolution_operator_approval_unexpected_review_packet_ids=review-packet:unrelated",
            "self_evolution_operator_approval_unexpected_evidence_ids=evidence:unrelated",
            "self_evolution_operator_approval_unexpected_rollback_anchor_ids=rollback-anchor:unrelated",
            "self_evolution_operator_approval_unexpected_content_digests=fnv64:unrelated",
            "self_evolution_operator_approval_unexpected_source_report_schemas=rust-norion-unrelated-schema-v1",
        ] {
            assert!(
                report.blocked_reasons.contains(&reason.to_owned()),
                "missing unexpected-ref blocked reason {reason}: {:?}",
                report.blocked_reasons
            );
        }
        assert!(!report.activation_write_allowed);
        assert!(!report.write_allowed);
        assert!(!report.applied);
    }

    #[test]
    fn self_evolution_operator_approval_gate_holds_tampered_attestation() {
        let mut ledger = SelfEvolutionExperimentLedger::new();
        ledger.append_admission_report(
            "experiment-rollback",
            &rollback_admission_report("candidate-rollback"),
        );
        let plan = ledger.rollback_replay_plan();
        let gate_report = SelfEvolutionRollbackReplayGate::new().evaluate(&plan);
        let mut evidence = SelfEvolutionOperatorApprovalEvidence::from_review_packet(
            "maintainer-jy",
            "approval-ticket-tampered",
            &gate_report.review_packet,
            "original approval reason",
        );
        evidence.approval_reason = "tampered approval reason".to_owned();
        evidence.approval_attestation_digest = "fnv64:syntactically-valid-but-wrong".to_owned();

        let report = SelfEvolutionOperatorApprovalGate::new()
            .evaluate(&gate_report.review_packet, &evidence);

        assert_eq!(report.decision, SelfEvolutionOperatorApprovalDecision::Hold);
        assert!(!report.operator_approved);
        assert!(
            report.blocked_reasons.contains(
                &"self_evolution_operator_approval_attestation_digest_mismatch".to_owned()
            )
        );
        assert!(!report.write_allowed);
        assert!(!report.applied);
    }

    #[test]
    fn self_evolution_operator_approval_ledger_is_append_only_and_read_only() {
        let mut experiment_ledger = SelfEvolutionExperimentLedger::new();
        experiment_ledger.append_admission_report(
            "experiment-rollback",
            &rollback_admission_report("candidate-rollback"),
        );
        let plan = experiment_ledger.rollback_replay_plan();
        let gate_report = SelfEvolutionRollbackReplayGate::new().evaluate(&plan);
        let evidence = SelfEvolutionOperatorApprovalEvidence::from_review_packet(
            "maintainer-jy",
            "approval-ticket-append-only",
            &gate_report.review_packet,
            "approved for audit trail only",
        );
        let approved = SelfEvolutionOperatorApprovalGate::new()
            .evaluate(&gate_report.review_packet, &evidence);
        let mut held_evidence = evidence.clone();
        held_evidence.approval_ticket_id.clear();
        let held = SelfEvolutionOperatorApprovalGate::new()
            .evaluate(&gate_report.review_packet, &held_evidence);
        let mut approval_ledger = SelfEvolutionOperatorApprovalLedger::new();

        let first = approval_ledger.append_report(&approved);
        let first_snapshot = first.clone();
        let second = approval_ledger.append_report(&held);

        assert_eq!(first.sequence, 1);
        assert_eq!(second.sequence, 2);
        assert_eq!(approval_ledger.records().len(), 2);
        assert_eq!(approval_ledger.records()[0], first_snapshot);
        assert_eq!(approval_ledger.approved(), 1);
        assert_eq!(approval_ledger.held(), 1);
        assert_eq!(approval_ledger.write_allowed_records(), 0);
        assert_eq!(approval_ledger.applied_records(), 0);
        assert!(approval_ledger.summary_line().contains("records=2"));
        assert!(approval_ledger.summary_line().contains("approved=1"));
        assert!(
            first
                .summary_line()
                .contains("self_evolution_operator_approval_record sequence=1")
        );
        assert!(!approved.write_allowed);
        assert!(!approved.activation_write_allowed);
        assert!(!approved.applied);
        assert!(!approved.active_candidate);
    }

    #[test]
    fn self_evolution_operator_approval_ledger_normalizes_forged_write_active_report() {
        let mut experiment_ledger = SelfEvolutionExperimentLedger::new();
        experiment_ledger.append_admission_report(
            "experiment-rollback",
            &rollback_admission_report("candidate-rollback"),
        );
        let plan = experiment_ledger.rollback_replay_plan();
        let gate_report = SelfEvolutionRollbackReplayGate::new().evaluate(&plan);
        let evidence = SelfEvolutionOperatorApprovalEvidence::from_review_packet(
            "maintainer-jy",
            "approval-ticket-forged",
            &gate_report.review_packet,
            "approved for audit trail only",
        );
        let mut forged = SelfEvolutionOperatorApprovalGate::new()
            .evaluate(&gate_report.review_packet, &evidence);
        forged.read_only = false;
        forged.report_only = false;
        forged.preview_only = false;
        forged.activation_write_allowed = true;
        forged.active_candidate = true;
        forged.write_allowed = true;
        forged.applied = true;
        let mut approval_ledger = SelfEvolutionOperatorApprovalLedger::new();

        let record = approval_ledger.append_report(&forged);

        assert_eq!(record.sequence, 1);
        assert_eq!(
            record.report.decision,
            SelfEvolutionOperatorApprovalDecision::Hold
        );
        assert!(!record.report.operator_approved);
        assert!(record.report.read_only);
        assert!(record.report.report_only);
        assert!(record.report.preview_only);
        assert!(!record.report.activation_write_allowed);
        assert!(!record.report.active_candidate);
        assert!(!record.report.write_allowed);
        assert!(!record.report.applied);
        assert!(record.report.blocked_reasons.contains(
            &"self_evolution_operator_approval_ledger_rejected_write_active_report".to_owned()
        ));
        assert_eq!(approval_ledger.approved(), 0);
        assert_eq!(approval_ledger.held(), 1);
        assert_eq!(approval_ledger.write_allowed_records(), 0);
        assert_eq!(approval_ledger.applied_records(), 0);
    }

    #[test]
    fn self_evolution_promotion_preflight_requires_admission_experiment_and_operator_approval() {
        let (admission, experiment, approval) =
            approved_promotion_preflight_inputs("candidate-promotion");

        let report =
            SelfEvolutionPromotionPreflightGate::new().evaluate(&admission, &experiment, &approval);

        assert_eq!(
            report.decision,
            SelfEvolutionPromotionPreflightDecision::ReadyForExplicitPromotion
        );
        assert!(report.ready_for_explicit_promotion);
        assert!(report.explicit_promotion_required);
        assert!(report.admission_admitted_for_human_review);
        assert!(report.experiment_admitted_for_human_review);
        assert!(report.operator_approved);
        assert!(report.rust_validation_passed);
        assert!(report.validation_passed);
        assert!(report.benchmark_gate_passed);
        assert!(report.adaptive_preview_evidence_present);
        assert!(report.review_packet_count > 0);
        assert!(report.evidence_id_count > 0);
        assert!(report.rollback_anchor_count > 0);
        assert!(report.content_digest_count > 0);
        assert!(report.source_report_schema_count > 0);
        assert!(report.blocked_reasons.is_empty());
        assert!(report.is_read_only_preflight());
        assert!(!report.activation_write_allowed);
        assert!(!report.active_candidate);
        assert!(!report.write_allowed);
        assert!(!report.applied);
        assert!(report.content_digest.starts_with("fnv64:"));
        assert!(
            report
                .summary_line()
                .contains("ready_for_explicit_promotion=true")
        );
        assert!(
            report
                .json_line()
                .contains("\"schema\":\"rust-norion-self-evolution-promotion-preflight-v1\"")
        );
        assert!(report.json_line().contains("\"write_allowed\":false"));
        assert!(!report.json_line().contains("maintainer-jy"));
        assert!(!report.json_line().contains("approval-ticket-promotion"));
        assert!(
            !report
                .json_line()
                .contains("approved admission packet for promotion preflight")
        );
    }

    #[test]
    fn self_evolution_promotion_preflight_holds_without_operator_approval_or_matching_refs() {
        let (admission, experiment, mut approval) =
            approved_promotion_preflight_inputs("candidate-promotion-hold");
        approval.decision = SelfEvolutionOperatorApprovalDecision::Hold;
        approval.operator_approved = false;
        approval
            .approved_evidence_ids
            .push("evidence:unexpected-promotion-ref".to_owned());

        let report =
            SelfEvolutionPromotionPreflightGate::new().evaluate(&admission, &experiment, &approval);

        assert_eq!(
            report.decision,
            SelfEvolutionPromotionPreflightDecision::Hold
        );
        assert!(!report.ready_for_explicit_promotion);
        assert!(report.is_read_only_preflight());
        assert!(report.blocked_reasons.contains(
            &"self_evolution_promotion_preflight_operator_approval_not_approved".to_owned()
        ));
        assert!(
            report
                .blocked_reasons
                .contains(&"self_evolution_promotion_preflight_evidence_ids_mismatch".to_owned())
        );
        assert!(!report.activation_write_allowed);
        assert!(!report.active_candidate);
        assert!(!report.write_allowed);
        assert!(!report.applied);
    }

    #[test]
    fn self_evolution_promotion_preflight_holds_unsafe_experiment_record() {
        let (admission, mut experiment, approval) =
            approved_promotion_preflight_inputs("candidate-promotion-unsafe");
        experiment.active_candidate = true;
        experiment.write_allowed = true;
        experiment.applied = true;
        experiment.read_only = false;
        experiment.report_only = false;
        experiment.preview_only = false;

        let report =
            SelfEvolutionPromotionPreflightGate::new().evaluate(&admission, &experiment, &approval);

        assert_eq!(
            report.decision,
            SelfEvolutionPromotionPreflightDecision::Hold
        );
        assert!(!report.ready_for_explicit_promotion);
        assert!(report.is_read_only_preflight());
        assert!(report.blocked_reasons.contains(
            &"self_evolution_promotion_preflight_experiment_not_read_only_preview".to_owned()
        ));
        assert!(report.blocked_reasons.contains(
            &"self_evolution_promotion_preflight_experiment_write_or_applied".to_owned()
        ));
        assert!(!report.activation_write_allowed);
        assert!(!report.active_candidate);
        assert!(!report.write_allowed);
        assert!(!report.applied);
    }

    #[test]
    fn self_evolution_rollback_replay_apply_preflight_requires_gate_and_approval() {
        let (rollback_gate, approval) = approved_rollback_replay_gate_and_approval();

        let report =
            SelfEvolutionRollbackReplayApplyGate::new().evaluate(&rollback_gate, &approval);

        assert_eq!(
            report.decision,
            SelfEvolutionRollbackReplayApplyDecision::ReadyForOperatorApply
        );
        assert!(report.ready_for_operator_apply);
        assert!(report.explicit_apply_required);
        assert!(report.rollback_gate_admitted_for_human_review);
        assert!(report.operator_approved);
        assert_eq!(report.item_count, 1);
        assert_eq!(report.replayable, report.item_count);
        assert_eq!(report.blocked, 0);
        assert_eq!(report.review_packet_count, 1);
        assert!(report.evidence_id_count > 0);
        assert!(report.rollback_anchor_count > 0);
        let rollback_anchor =
            TenantScopedKey::parse(&rollback_gate.review_packet.rollback_anchor_ids[0])
                .expect("scoped rollback anchor id");
        assert_eq!(rollback_anchor.lane, TenantResourceLane::SessionState);
        assert_eq!(rollback_anchor.scope, TenantScope::local_single_user());
        assert!(report.content_digest_count > 0);
        assert!(report.source_report_schema_count > 0);
        assert!(report.blocked_reasons.is_empty());
        assert!(report.is_read_only_preflight());
        assert!(!report.activation_write_allowed);
        assert!(!report.active_candidate);
        assert!(!report.write_allowed);
        assert!(!report.applied);
        assert!(
            report
                .summary_line()
                .contains("ready_for_operator_apply=true")
        );
    }

    #[test]
    fn self_evolution_rollback_replay_apply_preflight_rejects_cross_tenant_rollback_anchor_scope() {
        let (rollback_gate, approval) = approved_rollback_replay_gate_and_approval();
        let tenant_b = TenantScope::new("tenant-b", "workspace", "session-b");

        let report = SelfEvolutionRollbackReplayApplyGate::new().evaluate_for_scope(
            &tenant_b,
            &rollback_gate,
            &approval,
        );

        assert_eq!(
            report.decision,
            SelfEvolutionRollbackReplayApplyDecision::Hold
        );
        assert!(!report.ready_for_operator_apply);
        assert!(report.blocked_reasons.contains(
            &"self_evolution_rollback_replay_apply_rollback_anchor_ids_scope_rejected".to_owned()
        ));
        assert!(
            report.blocked_reasons.contains(
                &"self_evolution_rollback_replay_apply_approved_rollback_anchor_ids_scope_rejected"
                    .to_owned()
            )
        );
        assert!(!report.summary_line().contains("tenant=local"));
        assert!(!report.json_line().contains("tenant=local"));
        assert!(!report.json_line().contains("rollback-budget"));
    }

    #[test]
    fn self_evolution_rollback_replay_apply_preflight_holds_without_operator_approval() {
        let (rollback_gate, mut approval) = approved_rollback_replay_gate_and_approval();
        approval.decision = SelfEvolutionOperatorApprovalDecision::Hold;
        approval.operator_approved = false;
        approval
            .blocked_reasons
            .push("operator approval deliberately missing".to_owned());

        let report =
            SelfEvolutionRollbackReplayApplyGate::new().evaluate(&rollback_gate, &approval);

        assert_eq!(
            report.decision,
            SelfEvolutionRollbackReplayApplyDecision::Hold
        );
        assert!(!report.ready_for_operator_apply);
        assert!(report.is_read_only_preflight());
        assert!(report.blocked_reasons.contains(
            &"self_evolution_rollback_replay_apply_operator_approval_not_approved".to_owned()
        ));
        assert!(!report.activation_write_allowed);
        assert!(!report.active_candidate);
        assert!(!report.write_allowed);
        assert!(!report.applied);
    }

    #[test]
    fn self_evolution_rollback_replay_apply_preflight_holds_mismatched_approval_refs() {
        let (rollback_gate, mut approval) = approved_rollback_replay_gate_and_approval();
        approval
            .approved_content_digests
            .push("fnv64:unexpected-extra-digest".to_owned());

        let report =
            SelfEvolutionRollbackReplayApplyGate::new().evaluate(&rollback_gate, &approval);

        assert_eq!(
            report.decision,
            SelfEvolutionRollbackReplayApplyDecision::Hold
        );
        assert!(!report.ready_for_operator_apply);
        assert!(report.is_read_only_preflight());
        assert!(
            report.blocked_reasons.contains(
                &"self_evolution_rollback_replay_apply_content_digests_mismatch".to_owned()
            )
        );
    }

    #[test]
    fn self_evolution_rollback_replay_apply_preflight_holds_unsafe_replay_gate() {
        let (mut rollback_gate, approval) = approved_rollback_replay_gate_and_approval();
        rollback_gate.write_allowed = true;
        rollback_gate.plan_applied = true;

        let report =
            SelfEvolutionRollbackReplayApplyGate::new().evaluate(&rollback_gate, &approval);

        assert_eq!(
            report.decision,
            SelfEvolutionRollbackReplayApplyDecision::Hold
        );
        assert!(!report.ready_for_operator_apply);
        assert!(report.is_read_only_preflight());
        assert!(
            report
                .blocked_reasons
                .contains(&"self_evolution_rollback_replay_apply_gate_write_or_applied".to_owned())
        );
        assert!(report.blocked_reasons.contains(
            &"self_evolution_rollback_replay_apply_gate_plan_write_or_applied".to_owned()
        ));
    }

    #[test]
    fn self_evolution_admission_allows_read_only_human_review_packet() {
        let router_preview = safe_router_threshold_preview();
        let evidence = SelfEvolutionAdmissionEvidence::from_benchmark_gate(
            "router-preview-round",
            passing_evolution_ledger(),
            &passing_benchmark_gate(),
        )
        .with_validation_evidence(passing_validation_evidence())
        .with_router_threshold_preview_report(&router_preview);

        let report = SelfEvolutionAdmissionGate::new().evaluate(&evidence);

        assert!(report.read_only);
        assert!(report.report_only);
        assert!(report.preview_only);
        assert!(report.policy_valid);
        assert!(!report.mutation_write_allowed);
        assert!(!report.memory_store_write_allowed);
        assert!(!report.ndkv_write_allowed);
        assert!(!report.model_weight_write_allowed);
        assert!(!report.git_write_allowed);
        assert!(report.human_approval_required);
        assert!(report.admitted_for_human_review);
        assert!(report.rust_validation_passed);
        assert!(report.validation_passed);
        assert!(report.benchmark_gate_passed);
        assert!(report.rollback_budget_clean);
        assert!(report.adaptive_preview_evidence_present);
        assert_eq!(report.adaptive_preview_source_count, 1);
        assert!(report.adaptive_preview_read_only);
        assert!(report.adaptive_preview_report_only);
        assert!(report.adaptive_preview_preview_only);
        assert!(!report.adaptive_preview_write_allowed);
        assert!(!report.adaptive_preview_applied);
        assert!(report.blocked_reasons.is_empty());
        assert!(!report.review_packet.approval_review_packet_ids.is_empty());
        assert!(report.review_packet.evidence_ids.iter().any(|id| {
            id.starts_with("adaptive-preview:router-threshold:router-preview-round")
        }));
        assert!(!report.review_packet.content_digests.is_empty());
        assert!(
            report
                .review_packet
                .source_report_schemas
                .iter()
                .any(|schema| schema == "rust-norion-self-evolution-admission-v1")
        );
        assert_eq!(report.rust_check_items, 2);
        assert_eq!(report.rust_check_passed, 2);
        assert_eq!(report.rust_check_failed, 0);
        assert!(report.summary_line().contains("self_evolution_admission"));
        assert!(
            report
                .telemetry
                .iter()
                .any(|line| { line == "self_evolution_admission_admitted_for_human_review=true" })
        );
        assert!(
            report
                .telemetry
                .iter()
                .any(|line| { line == "self_evolution_admission_human_approval_required=true" })
        );
        assert!(
            report
                .telemetry
                .iter()
                .any(|line| line == "self_evolution_admission_review_packet_ids=1")
        );
    }

    #[test]
    fn self_evolution_admission_derives_benchmark_gate_from_summary() {
        let summary = BenchmarkSummary::new();
        let evidence = SelfEvolutionAdmissionEvidence::from_benchmark_summary(
            "empty-summary",
            &summary,
            &BenchmarkGate::default(),
        );

        assert_eq!(evidence.evolution_ledger, EvolutionLedger::default());
        assert!(!evidence.benchmark_gate_passed);
        assert!(
            evidence
                .benchmark_gate_failures
                .iter()
                .any(|failure| failure == "no benchmark cases were recorded")
        );
    }

    #[test]
    fn self_evolution_admission_attaches_command_and_gate_validation_artifacts() {
        let router_preview = safe_router_threshold_preview();
        let evidence = SelfEvolutionAdmissionEvidence::from_benchmark_gate(
            "artifact-candidate",
            EvolutionLedger::default(),
            &passing_benchmark_gate(),
        )
        .with_validation_artifacts([
            SelfEvolutionValidationArtifact::cargo_check("cargo-check-rust-norion", true),
            SelfEvolutionValidationArtifact::focused_tests(
                "operator-approval-focused-tests",
                2,
                2,
                0,
            ),
            SelfEvolutionValidationArtifact::benchmark_gate("benchmark-gate-full", true, 0),
            SelfEvolutionValidationArtifact::trace_schema_gate("trace-schema-jsonl", true, 3, 0),
        ])
        .with_router_threshold_preview_report(&router_preview);

        let report = SelfEvolutionAdmissionGate::new().evaluate(&evidence);

        assert!(
            report.admitted_for_human_review,
            "{:?}",
            report.blocked_reasons
        );
        assert!(report.rust_validation_passed);
        assert!(report.validation_passed);
        assert_eq!(report.rust_check_items, 1);
        assert_eq!(report.rust_check_passed, 1);
        assert_eq!(report.rust_check_failed, 0);
        assert_eq!(
            report.validation.compiler,
            SelfEvolutionValidationLane::new(1, 1, 0)
        );
        assert_eq!(
            report.validation.tests,
            SelfEvolutionValidationLane::new(2, 2, 0)
        );
        assert_eq!(
            report.validation.experiments,
            SelfEvolutionValidationLane::new(3, 1, 0)
        );
        for prefix in [
            "validation-artifact:cargo-check:artifact-candidate:cargo-check-rust-norion",
            "validation-artifact:focused-tests:artifact-candidate:operator-approval-focused-tests",
            "validation-artifact:benchmark-gate:artifact-candidate:benchmark-gate-full",
            "validation-artifact:trace-schema-gate:artifact-candidate:trace-schema-jsonl",
        ] {
            assert!(
                report
                    .review_packet
                    .evidence_ids
                    .iter()
                    .any(|evidence_id| evidence_id.starts_with(prefix)),
                "missing validation artifact evidence prefix {prefix}: {:?}",
                report.review_packet.evidence_ids
            );
        }
        for schema in [
            "rust-norion-self-evolution-validation-artifact-v1",
            "rust-norion-cargo-check-v1",
            "rust-norion-focused-test-v1",
            "rust-norion-benchmark-gate-v1",
            "rust-norion-trace-schema-gate-v1",
        ] {
            assert!(
                report
                    .review_packet
                    .source_report_schemas
                    .iter()
                    .any(|source_schema| source_schema == schema),
                "missing validation artifact schema {schema}: {:?}",
                report.review_packet.source_report_schemas
            );
        }
        assert!(!report.mutation_write_allowed);
        assert!(!report.memory_store_write_allowed);
        assert!(!report.ndkv_write_allowed);
        assert!(!report.model_weight_write_allowed);
        assert!(!report.git_write_allowed);
    }

    #[test]
    fn self_evolution_admission_blocks_failed_validation_artifact_without_writes() {
        let router_preview = safe_router_threshold_preview();
        let evidence = SelfEvolutionAdmissionEvidence::from_benchmark_gate(
            "failed-artifact",
            EvolutionLedger::default(),
            &passing_benchmark_gate(),
        )
        .with_validation_artifacts([
            SelfEvolutionValidationArtifact::cargo_check("cargo-check-rust-norion", false),
            SelfEvolutionValidationArtifact::focused_tests("focused-tests", 1, 1, 0),
            SelfEvolutionValidationArtifact::benchmark_gate("benchmark-gate", true, 0),
            SelfEvolutionValidationArtifact::trace_schema_gate("trace-schema", true, 1, 0),
        ])
        .with_router_threshold_preview_report(&router_preview);

        let report = SelfEvolutionAdmissionGate::new().evaluate(&evidence);

        assert!(!report.admitted_for_human_review);
        assert!(!report.rust_validation_passed);
        assert!(!report.validation_passed);
        assert_eq!(report.rust_check_items, 1);
        assert_eq!(report.rust_check_passed, 0);
        assert_eq!(report.rust_check_failed, 1);
        assert_eq!(
            report.validation.compiler,
            SelfEvolutionValidationLane::new(1, 0, 1)
        );
        assert!(
            report
                .blocked_reasons
                .contains(&"self_evolution_admission_rust_check_passed=0<1".to_owned())
        );
        assert!(
            report
                .blocked_reasons
                .contains(&"self_evolution_admission_rust_check_failed=1>0".to_owned())
        );
        assert!(
            report
                .blocked_reasons
                .contains(&"self_evolution_admission_compiler_validation_passed=0<1".to_owned())
        );
        assert!(
            report
                .blocked_reasons
                .contains(&"self_evolution_admission_compiler_validation_failed=1>0".to_owned())
        );
        assert!(!report.mutation_write_allowed);
        assert!(!report.memory_store_write_allowed);
        assert!(!report.ndkv_write_allowed);
        assert!(!report.model_weight_write_allowed);
        assert!(!report.git_write_allowed);
    }

    #[test]
    fn self_evolution_admission_derives_preview_readiness_from_reports() {
        let metrics = GenerationMetrics {
            perplexity: 36.0,
            semantic_consistency: 0.20,
            contradiction_count: 2,
            token_count: 64,
        };
        let router_preview = safe_router_threshold_preview();
        let hierarchy_preview = HierarchyAdjustmentPreviewPlanner::new().preview(
            HierarchyController::new().state(),
            TaskProfile::Coding,
            metrics,
        );
        let recall_report = crate::split::agent::AgentRecallOutcomeAttributionReport {
            attributions: vec![crate::split::agent::AgentRecallOutcomeAttribution {
                task_id: "runtime-recall".to_owned(),
                record_id: "runtime_kv:l0h0:0-8".to_owned(),
                source: "runtime_kv".to_owned(),
                action: crate::split::agent::AgentRecallOutcomeAttributionAction::Reinforce,
                amount: 0.24,
                reason_codes: vec!["result_accepted".to_owned()],
            }],
            reinforced_count: 1,
            penalized_count: 0,
            skipped_rejected_recall_count: 0,
            skipped_missing_outcome_task_ids: Vec::new(),
            read_only: true,
            memory_store_write_allowed: false,
            telemetry: Vec::new(),
        };
        let kv_reward_preview =
            crate::split::bridge::recall_outcome_attribution_to_kv_fusion_reward_preview(
                &recall_report,
            );
        let kv_policy_preview = crate::split::bridge::kv_fusion_reward_policy_observation_dry_run(
            &kv_reward_preview,
            crate::split::core::ReinforcedKvFusionPolicy::new(0.92, 64),
        );
        let evidence = SelfEvolutionAdmissionEvidence::from_benchmark_gate(
            "report-derived-preview",
            passing_evolution_ledger(),
            &passing_benchmark_gate(),
        )
        .with_validation_evidence(passing_validation_evidence())
        .with_router_threshold_preview_report(&router_preview)
        .with_hierarchy_adjustment_preview_report(&hierarchy_preview)
        .with_kv_fusion_policy_observation_preview_report(&kv_policy_preview);

        assert!(evidence.router_threshold_preview_ready);
        assert!(evidence.hierarchy_adjustment_preview_ready);
        assert!(evidence.kv_fusion_policy_observation_preview_ready);
        assert_eq!(evidence.adaptive_preview_source_count, 3);
        assert!(evidence.adaptive_preview_read_only);
        assert!(evidence.adaptive_preview_report_only);
        assert!(evidence.adaptive_preview_preview_only);
        assert!(!evidence.adaptive_preview_write_allowed);
        assert!(!evidence.adaptive_preview_applied);
        assert!(evidence.adaptive_preview_blocked_reasons.is_empty());

        let report = SelfEvolutionAdmissionGate::new().evaluate(&evidence);

        assert!(report.admitted_for_human_review);
        assert!(report.adaptive_preview_evidence_present);
        assert_eq!(report.adaptive_preview_source_count, 3);
        assert!(report.adaptive_preview_read_only);
        assert!(report.adaptive_preview_report_only);
        assert!(report.adaptive_preview_preview_only);
        assert!(!report.adaptive_preview_write_allowed);
        assert!(!report.adaptive_preview_applied);
        assert!(report.adaptive_preview_blocked_reasons.is_empty());
        assert!(
            report
                .telemetry
                .iter()
                .any(|line| line == "self_evolution_admission_adaptive_preview_blocked_reasons=0")
        );
    }

    #[test]
    fn self_evolution_admission_blocks_preview_reports_with_write_or_blocked_flags() {
        let mut router_preview = RouterThresholdAdjustmentPreviewPlanner::new().preview(
            NoironRouter::new().state(),
            TaskProfile::Coding,
            GenerationMetrics {
                perplexity: 36.0,
                semantic_consistency: 0.20,
                contradiction_count: 2,
                token_count: 64,
            },
        );
        router_preview.router_state_write_allowed = true;
        router_preview.report_only = false;
        router_preview.router_observation_applied = true;

        let blocked_router_preview = RouterThresholdAdjustmentPreviewPlanner::new().preview(
            NoironRouter::new().state(),
            TaskProfile::Coding,
            GenerationMetrics {
                perplexity: f32::NAN,
                semantic_consistency: 0.20,
                contradiction_count: 0,
                token_count: 64,
            },
        );
        let evidence = SelfEvolutionAdmissionEvidence::from_benchmark_gate(
            "unsafe-preview",
            passing_evolution_ledger(),
            &passing_benchmark_gate(),
        )
        .with_router_threshold_preview_report(&router_preview)
        .with_router_threshold_preview_report(&blocked_router_preview);

        assert!(!evidence.router_threshold_preview_ready);
        assert_eq!(evidence.adaptive_preview_source_count, 2);
        assert!(evidence.adaptive_preview_read_only);
        assert!(!evidence.adaptive_preview_report_only);
        assert!(evidence.adaptive_preview_preview_only);
        assert!(evidence.adaptive_preview_write_allowed);
        assert!(evidence.adaptive_preview_applied);
        assert!(
            evidence
                .adaptive_preview_blocked_reasons
                .contains(&"self_evolution_admission_router_preview_not_report_only".to_owned())
        );
        assert!(
            evidence
                .adaptive_preview_blocked_reasons
                .contains(&"self_evolution_admission_router_preview_write_allowed".to_owned())
        );
        assert!(
            evidence
                .adaptive_preview_blocked_reasons
                .contains(&"self_evolution_admission_router_preview_already_applied".to_owned())
        );
        assert!(evidence.adaptive_preview_blocked_reasons.iter().any(|reason| {
            reason
                == "self_evolution_admission_router_preview_blocked=router_threshold_adjustment_generation_metrics_not_finite"
        }));

        let report = SelfEvolutionAdmissionGate::new().evaluate(&evidence);

        assert!(!report.admitted_for_human_review);
        assert!(!report.adaptive_preview_evidence_present);
        assert_eq!(report.adaptive_preview_source_count, 2);
        assert!(!report.adaptive_preview_report_only);
        assert!(report.adaptive_preview_write_allowed);
        assert!(report.adaptive_preview_applied);
        assert!(
            report
                .blocked_reasons
                .contains(&"self_evolution_admission_adaptive_preview_evidence_missing".to_owned())
        );
        assert!(
            report
                .blocked_reasons
                .contains(&"self_evolution_admission_router_preview_write_allowed".to_owned())
        );
    }

    #[test]
    fn self_evolution_admission_blocks_hierarchy_and_kv_preview_report_failures() {
        let hierarchy_preview = HierarchyAdjustmentPreviewPlanner::new().preview(
            HierarchyController::new().state(),
            TaskProfile::Coding,
            GenerationMetrics {
                perplexity: 12.0,
                semantic_consistency: 0.90,
                contradiction_count: 0,
                token_count: 0,
            },
        );
        let recall_report = crate::split::agent::AgentRecallOutcomeAttributionReport {
            attributions: vec![crate::split::agent::AgentRecallOutcomeAttribution {
                task_id: "runtime-recall".to_owned(),
                record_id: "runtime_kv:l0h0:0-8".to_owned(),
                source: "runtime_kv".to_owned(),
                action: crate::split::agent::AgentRecallOutcomeAttributionAction::Penalize,
                amount: 0.32,
                reason_codes: vec!["execution_failed".to_owned()],
            }],
            reinforced_count: 0,
            penalized_count: 1,
            skipped_rejected_recall_count: 0,
            skipped_missing_outcome_task_ids: Vec::new(),
            read_only: false,
            memory_store_write_allowed: true,
            telemetry: Vec::new(),
        };
        let kv_reward_preview =
            crate::split::bridge::recall_outcome_attribution_to_kv_fusion_reward_preview(
                &recall_report,
            );
        let kv_policy_preview = crate::split::bridge::kv_fusion_reward_policy_observation_dry_run(
            &kv_reward_preview,
            crate::split::core::ReinforcedKvFusionPolicy::new(0.92, 64),
        );
        let evidence = SelfEvolutionAdmissionEvidence::from_benchmark_gate(
            "blocked-hierarchy-kv-preview",
            passing_evolution_ledger(),
            &passing_benchmark_gate(),
        )
        .with_hierarchy_adjustment_preview_report(&hierarchy_preview)
        .with_kv_fusion_policy_observation_preview_report(&kv_policy_preview);

        assert!(!evidence.hierarchy_adjustment_preview_ready);
        assert!(!evidence.kv_fusion_policy_observation_preview_ready);
        assert_eq!(evidence.adaptive_preview_source_count, 2);
        assert!(!evidence.adaptive_preview_read_only);
        assert!(evidence.adaptive_preview_report_only);
        assert!(evidence.adaptive_preview_preview_only);
        assert!(evidence.adaptive_preview_write_allowed);
        assert!(!evidence.adaptive_preview_applied);
        assert!(
            evidence
                .adaptive_preview_blocked_reasons
                .contains(&"self_evolution_admission_hierarchy_preview_not_ready".to_owned())
        );
        assert!(evidence.adaptive_preview_blocked_reasons.iter().any(|reason| {
            reason
                == "self_evolution_admission_hierarchy_preview_blocked=hierarchy_adjustment_token_count=0<1"
        }));
        assert!(
            evidence
                .adaptive_preview_blocked_reasons
                .contains(&"self_evolution_admission_kv_fusion_preview_not_ready".to_owned())
        );
        assert!(
            evidence
                .adaptive_preview_blocked_reasons
                .iter()
                .any(|reason| {
                    reason == "self_evolution_admission_kv_fusion_preview_source_not_read_only"
                })
        );
        assert!(evidence.adaptive_preview_blocked_reasons.iter().any(|reason| {
            reason
                == "self_evolution_admission_kv_fusion_preview_source_memory_store_write_allowed"
        }));
        assert!(evidence.adaptive_preview_blocked_reasons.iter().any(|reason| {
            reason
                == "self_evolution_admission_kv_fusion_preview_blocked=recall_attribution_not_read_only"
        }));

        let report = SelfEvolutionAdmissionGate::new().evaluate(&evidence);

        assert!(!report.admitted_for_human_review);
        assert!(!report.adaptive_preview_evidence_present);
        assert!(!report.adaptive_preview_read_only);
        assert!(report.adaptive_preview_write_allowed);
        assert!(
            report
                .blocked_reasons
                .contains(&"self_evolution_admission_adaptive_preview_evidence_missing".to_owned())
        );
        assert!(
            report
                .blocked_reasons
                .contains(&"self_evolution_admission_hierarchy_preview_not_ready".to_owned())
        );
        assert!(
            report
                .blocked_reasons
                .contains(&"self_evolution_admission_kv_fusion_preview_not_ready".to_owned())
        );
    }

    #[test]
    fn self_evolution_admission_blocks_missing_rust_benchmark_and_preview_evidence() {
        let benchmark_gate = BenchmarkGateReport {
            passed: false,
            failures: vec!["evolution_replay_rust_check_passed 0 below minimum 1".to_owned()],
        };
        let evidence = SelfEvolutionAdmissionEvidence::from_benchmark_gate(
            "empty-candidate",
            EvolutionLedger::default(),
            &benchmark_gate,
        );

        let report = SelfEvolutionAdmissionGate::new().evaluate(&evidence);

        assert!(!report.admitted_for_human_review);
        assert!(!report.rust_validation_passed);
        assert!(!report.validation_passed);
        assert!(!report.benchmark_gate_passed);
        assert!(!report.adaptive_preview_evidence_present);
        assert_eq!(report.benchmark_gate_failures, benchmark_gate.failures);
        assert!(
            report
                .blocked_reasons
                .contains(&"self_evolution_admission_rust_check_items=0<1".to_owned())
        );
        assert!(
            report
                .blocked_reasons
                .contains(&"self_evolution_admission_rust_check_passed=0<1".to_owned())
        );
        assert!(
            report
                .blocked_reasons
                .contains(&"self_evolution_admission_benchmark_gate_failed".to_owned())
        );
        assert!(
            report
                .blocked_reasons
                .iter()
                .any(|reason| { reason == "self_evolution_admission_tests_validation_items=0<1" })
        );
        assert!(report.blocked_reasons.iter().any(|reason| {
            reason == "self_evolution_admission_experiments_validation_items=0<1"
        }));
        assert!(
            report
                .blocked_reasons
                .contains(&"self_evolution_admission_adaptive_preview_evidence_missing".to_owned())
        );
    }

    #[test]
    fn self_evolution_admission_blocks_rollback_budget_regression() {
        let router_preview = safe_router_threshold_preview();
        let evidence = SelfEvolutionAdmissionEvidence::from_benchmark_gate(
            "rollback-candidate",
            EvolutionLedger {
                replay_rust_check_items: 1,
                replay_rust_check_passed: 1,
                replay_rust_check_failed: 0,
                drift_rollbacks: 1,
                rollback_router_threshold_delta: 0.02,
                rollback_hierarchy_weight_delta: 0.03,
                ..EvolutionLedger::default()
            },
            &passing_benchmark_gate(),
        )
        .with_validation_evidence(passing_validation_evidence())
        .with_router_threshold_preview_report(&router_preview);

        let report = SelfEvolutionAdmissionGate::new().evaluate(&evidence);

        assert!(!report.admitted_for_human_review);
        assert!(report.rust_validation_passed);
        assert!(report.benchmark_gate_passed);
        assert!(!report.rollback_budget_clean);
        assert!(
            report
                .blocked_reasons
                .contains(&"self_evolution_admission_drift_rollbacks=1>0".to_owned())
        );
        assert!(report.blocked_reasons.iter().any(|reason| {
            reason.starts_with("self_evolution_admission_rollback_router_threshold_delta=")
        }));
        assert!(report.blocked_reasons.iter().any(|reason| {
            reason.starts_with("self_evolution_admission_rollback_hierarchy_weight_delta=")
        }));
        assert!(
            report
                .telemetry
                .iter()
                .any(|line| { line == "self_evolution_admission_rollback_budget_clean=false" })
        );
    }

    #[test]
    fn self_evolution_admission_blocks_invalid_policy_and_empty_candidate() {
        let router_preview = safe_router_threshold_preview();
        let evidence = SelfEvolutionAdmissionEvidence::from_benchmark_gate(
            " ",
            EvolutionLedger {
                replay_rust_check_items: 1,
                replay_rust_check_passed: 1,
                replay_rust_check_failed: 0,
                ..EvolutionLedger::default()
            },
            &passing_benchmark_gate(),
        )
        .with_validation_evidence(passing_validation_evidence())
        .with_router_threshold_preview_report(&router_preview);

        let report = SelfEvolutionAdmissionGate::new()
            .with_policy(SelfEvolutionAdmissionPolicy {
                max_rollback_router_threshold_delta: f32::NAN,
                max_rollback_hierarchy_weight_delta: -0.01,
                ..SelfEvolutionAdmissionPolicy::default()
            })
            .evaluate(&evidence);

        assert!(!report.admitted_for_human_review);
        assert!(!report.policy_valid);
        assert!(
            report
                .blocked_reasons
                .contains(&"self_evolution_admission_candidate_id_empty".to_owned())
        );
        assert!(report.blocked_reasons.contains(
            &"self_evolution_admission_max_rollback_router_threshold_delta_invalid".to_owned()
        ));
        assert!(report.blocked_reasons.contains(
            &"self_evolution_admission_max_rollback_hierarchy_weight_delta_invalid".to_owned()
        ));
        assert!(
            report
                .telemetry
                .iter()
                .any(|line| line == "self_evolution_admission_policy_valid=false")
        );
    }

    #[test]
    fn self_evolution_admission_keeps_inputs_unchanged() {
        let router_preview = safe_router_threshold_preview();
        let ledger = EvolutionLedger {
            replay_rust_check_items: 1,
            replay_rust_check_passed: 1,
            replay_rust_check_failed: 0,
            ..EvolutionLedger::default()
        };
        let benchmark_gate = passing_benchmark_gate();
        let evidence = SelfEvolutionAdmissionEvidence::from_benchmark_gate(
            "immutable-evidence",
            ledger,
            &benchmark_gate,
        )
        .with_validation_evidence(passing_validation_evidence())
        .with_router_threshold_preview_report(&router_preview);
        let evidence_before = evidence.clone();

        let report = SelfEvolutionAdmissionGate::new().evaluate(&evidence);

        assert!(report.admitted_for_human_review);
        assert_eq!(evidence.candidate_id, evidence_before.candidate_id);
        assert_eq!(evidence.evolution_ledger, evidence_before.evolution_ledger);
        assert_eq!(benchmark_gate, passing_benchmark_gate());
    }
}
