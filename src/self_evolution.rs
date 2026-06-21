use crate::adaptive_state::EvolutionLedger;
use crate::benchmark::{BenchmarkGate, BenchmarkGateReport, BenchmarkSummary};
use crate::hierarchy::HierarchyAdjustmentPreviewReport;
use crate::router::RouterThresholdAdjustmentPreviewReport;
use crate::split::bridge::KvFusionPolicyObservationDryRunReport;

#[derive(Debug, Clone, Copy)]
pub struct SelfEvolutionAdmissionPolicy {
    pub min_rust_check_items: u64,
    pub require_all_rust_checks_passed: bool,
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
            require_all_rust_checks_passed: true,
            require_benchmark_gate_passed: true,
            require_adaptive_preview_evidence: true,
            max_drift_rollbacks: 0,
            max_rollback_router_threshold_delta: 0.0,
            max_rollback_hierarchy_weight_delta: 0.0,
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
}

impl SelfEvolutionAdmissionEvidence {
    pub fn from_benchmark_gate(
        candidate_id: impl Into<String>,
        evolution_ledger: EvolutionLedger,
        benchmark_gate: &BenchmarkGateReport,
    ) -> Self {
        Self {
            candidate_id: candidate_id.into(),
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
        self
    }

    pub fn with_kv_fusion_policy_observation_preview_report(
        mut self,
        report: &KvFusionPolicyObservationDryRunReport,
    ) -> Self {
        self.record_adaptive_preview_safety(AdaptivePreviewSafety {
            read_only: true,
            report_only: true,
            preview_only: report.preview_only,
            write_allowed: report.policy_write_allowed,
            applied: report.policy_observation_applied,
        });
        let ready = report.can_use_policy_observation_preview();
        self.kv_fusion_policy_observation_preview_ready = ready;
        if !ready {
            self.adaptive_preview_blocked_reasons
                .extend(kv_fusion_policy_observation_preview_blocked_reasons(report));
        }
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

    fn with_telemetry(mut self) -> Self {
        self.telemetry = self_evolution_admission_telemetry(&self);
        self
    }
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
    if !report.preview_only {
        reasons.push("self_evolution_admission_kv_fusion_preview_not_preview_only".to_owned());
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

    #[test]
    fn self_evolution_admission_allows_read_only_human_review_packet() {
        let router_preview = safe_router_threshold_preview();
        let evidence = SelfEvolutionAdmissionEvidence::from_benchmark_gate(
            "router-preview-round",
            passing_evolution_ledger(),
            &passing_benchmark_gate(),
        )
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
        assert!(evidence.adaptive_preview_read_only);
        assert!(evidence.adaptive_preview_report_only);
        assert!(evidence.adaptive_preview_preview_only);
        assert!(!evidence.adaptive_preview_write_allowed);
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
        assert!(evidence.adaptive_preview_blocked_reasons.iter().any(|reason| {
            reason
                == "self_evolution_admission_kv_fusion_preview_blocked=recall_attribution_not_read_only"
        }));

        let report = SelfEvolutionAdmissionGate::new().evaluate(&evidence);

        assert!(!report.admitted_for_human_review);
        assert!(!report.adaptive_preview_evidence_present);
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
        .with_router_threshold_preview_report(&router_preview);
        let evidence_before = evidence.clone();

        let report = SelfEvolutionAdmissionGate::new().evaluate(&evidence);

        assert!(report.admitted_for_human_review);
        assert_eq!(evidence.candidate_id, evidence_before.candidate_id);
        assert_eq!(evidence.evolution_ledger, evidence_before.evolution_ledger);
        assert_eq!(benchmark_gate, passing_benchmark_gate());
    }
}
