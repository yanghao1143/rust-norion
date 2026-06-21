use crate::adaptive_state::EvolutionLedger;
use crate::benchmark::{BenchmarkGate, BenchmarkGateReport, BenchmarkSummary};
use crate::hierarchy::HierarchyAdjustmentPreviewReport;
use crate::router::RouterThresholdAdjustmentPreviewReport;
use crate::split::bridge::KvFusionPolicyObservationDryRunReport;

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

    pub fn push_approval_review_packet_id(&mut self, value: impl Into<String>) {
        push_unique_string(&mut self.approval_review_packet_ids, value);
    }

    pub fn push_evidence_id(&mut self, value: impl Into<String>) {
        push_unique_string(&mut self.evidence_ids, value);
    }

    pub fn push_rollback_anchor_id(&mut self, value: impl Into<String>) {
        push_unique_string(&mut self.rollback_anchor_ids, value);
    }

    pub fn push_content_digest(&mut self, value: impl Into<String>) {
        push_unique_string(&mut self.content_digests, value);
    }

    pub fn push_source_report_schema(&mut self, value: impl Into<String>) {
        push_unique_string(&mut self.source_report_schemas, value);
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
