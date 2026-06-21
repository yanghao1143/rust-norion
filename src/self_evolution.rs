use crate::adaptive_state::EvolutionLedger;
use crate::benchmark::BenchmarkGateReport;

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
        }
    }

    pub fn with_router_threshold_preview_ready(mut self, ready: bool) -> Self {
        self.router_threshold_preview_ready = ready;
        self
    }

    pub fn with_hierarchy_adjustment_preview_ready(mut self, ready: bool) -> Self {
        self.hierarchy_adjustment_preview_ready = ready;
        self
    }

    pub fn with_kv_fusion_policy_observation_preview_ready(mut self, ready: bool) -> Self {
        self.kv_fusion_policy_observation_preview_ready = ready;
        self
    }

    pub fn adaptive_preview_evidence_present(&self) -> bool {
        self.router_threshold_preview_ready
            || self.hierarchy_adjustment_preview_ready
            || self.kv_fusion_policy_observation_preview_ready
    }
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

    fn passing_benchmark_gate() -> BenchmarkGateReport {
        BenchmarkGateReport {
            passed: true,
            failures: Vec::new(),
        }
    }

    #[test]
    fn self_evolution_admission_allows_read_only_human_review_packet() {
        let evidence = SelfEvolutionAdmissionEvidence::from_benchmark_gate(
            "router-preview-round",
            EvolutionLedger {
                replay_rust_check_items: 2,
                replay_rust_check_passed: 2,
                replay_rust_check_failed: 0,
                ..EvolutionLedger::default()
            },
            &passing_benchmark_gate(),
        )
        .with_router_threshold_preview_ready(true)
        .with_hierarchy_adjustment_preview_ready(true)
        .with_kv_fusion_policy_observation_preview_ready(true);

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
        .with_router_threshold_preview_ready(true);

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
        .with_router_threshold_preview_ready(true);

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
        .with_kv_fusion_policy_observation_preview_ready(true);
        let evidence_before = evidence.clone();

        let report = SelfEvolutionAdmissionGate::new().evaluate(&evidence);

        assert!(report.admitted_for_human_review);
        assert_eq!(evidence.candidate_id, evidence_before.candidate_id);
        assert_eq!(evidence.evolution_ledger, evidence_before.evolution_ledger);
        assert_eq!(benchmark_gate, passing_benchmark_gate());
    }
}
