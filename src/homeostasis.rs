use crate::privacy_redaction::stable_redaction_digest;
use crate::reasoning_genome::{MobileGeneMovementDecision, MobileGeneMovementReview};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HomeostaticGateDecision {
    Normal,
    DownshiftParallelism,
    PauseSelfEvolution,
    RejectNewSpawn,
    RequireOperatorReview,
    EmergencyQuarantine,
}

impl HomeostaticGateDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::DownshiftParallelism => "downshift_parallelism",
            Self::PauseSelfEvolution => "pause_self_evolution",
            Self::RejectNewSpawn => "reject_new_spawn",
            Self::RequireOperatorReview => "require_operator_review",
            Self::EmergencyQuarantine => "emergency_quarantine",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HomeostaticSetpoints {
    pub max_runtime_memory_pressure_milli: u16,
    pub max_device_pressure_milli: u16,
    pub max_model_pool_saturation_milli: u16,
    pub max_failed_model_workers: usize,
    pub max_trace_failure_rate_milli: u16,
    pub max_benchmark_failure_rate_milli: u16,
    pub max_memory_candidate_backlog: usize,
    pub max_genome_candidate_backlog: usize,
    pub max_verifier_rejection_rate_milli: u16,
    pub max_rollback_rate_milli: u16,
    pub max_quarantine_rate_milli: u16,
    pub max_operator_approval_backlog: usize,
    pub sustained_high_load_windows: usize,
    pub min_recovery_stable_windows: usize,
    pub emergency_rollback_rate_milli: u16,
    pub emergency_quarantine_rate_milli: u16,
}

impl Default for HomeostaticSetpoints {
    fn default() -> Self {
        Self {
            max_runtime_memory_pressure_milli: 760,
            max_device_pressure_milli: 820,
            max_model_pool_saturation_milli: 780,
            max_failed_model_workers: 0,
            max_trace_failure_rate_milli: 120,
            max_benchmark_failure_rate_milli: 120,
            max_memory_candidate_backlog: 16,
            max_genome_candidate_backlog: 8,
            max_verifier_rejection_rate_milli: 180,
            max_rollback_rate_milli: 160,
            max_quarantine_rate_milli: 180,
            max_operator_approval_backlog: 4,
            sustained_high_load_windows: 3,
            min_recovery_stable_windows: 2,
            emergency_rollback_rate_milli: 700,
            emergency_quarantine_rate_milli: 700,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AllostaticLoadCounters {
    pub runtime_memory_pressure_milli: u16,
    pub device_pressure_milli: u16,
    pub model_pool_saturation_milli: u16,
    pub failed_model_workers: usize,
    pub trace_schema_failure_rate_milli: u16,
    pub benchmark_failure_rate_milli: u16,
    pub memory_candidate_backlog: usize,
    pub genome_candidate_backlog: usize,
    pub verifier_rejection_rate_milli: u16,
    pub rollback_rate_milli: u16,
    pub quarantine_rate_milli: u16,
    pub operator_approval_backlog: usize,
    pub consecutive_high_load_windows: usize,
    pub recovery_stable_windows: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HomeostaticGateReport {
    pub decision: HomeostaticGateDecision,
    pub reason_codes: Vec<&'static str>,
    pub load_score_milli: u16,
    pub evidence_digest: String,
    pub model_cell_policy_movement_review_digest: Option<String>,
    pub durable_write_allowed: bool,
    pub recursive_spawn_allowed: bool,
    pub model_cell_expansion_allowed: bool,
    pub memory_admission_allowed: bool,
    pub genome_mutation_allowed: bool,
}

impl HomeostaticGateReport {
    pub fn trace_line(&self) -> String {
        format!(
            "homeostatic_gate decision={} load_score_milli={} reason_count={} evidence_digest={} model_cell_policy_movement_review={} durable_write_allowed={} recursive_spawn_allowed={} model_cell_expansion_allowed={} memory_admission_allowed={} genome_mutation_allowed={} read_only=true",
            self.decision.as_str(),
            self.load_score_milli,
            self.reason_codes.len(),
            self.evidence_digest,
            self.model_cell_policy_movement_review_digest
                .as_deref()
                .unwrap_or("none"),
            self.durable_write_allowed,
            self.recursive_spawn_allowed,
            self.model_cell_expansion_allowed,
            self.memory_admission_allowed,
            self.genome_mutation_allowed
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelCellPolicyMovement {
    pub policy_id: String,
    pub policy_digest: String,
    pub source_scope: String,
    pub target_scope: String,
    pub movement_review: Option<MobileGeneMovementReview>,
}

impl ModelCellPolicyMovement {
    pub fn new(
        policy_id: impl Into<String>,
        policy_digest: impl Into<String>,
        source_scope: impl Into<String>,
        target_scope: impl Into<String>,
    ) -> Self {
        Self {
            policy_id: policy_id.into(),
            policy_digest: policy_digest.into(),
            source_scope: source_scope.into(),
            target_scope: target_scope.into(),
            movement_review: None,
        }
    }

    pub fn with_movement_review(mut self, review: MobileGeneMovementReview) -> Self {
        self.movement_review = Some(review);
        self
    }
}

impl HomeostaticSetpoints {
    pub fn evaluate(self, counters: AllostaticLoadCounters) -> HomeostaticGateReport {
        self.evaluate_with_model_cell_policy_movement(counters, None)
    }

    pub fn evaluate_with_model_cell_policy_movement(
        self,
        counters: AllostaticLoadCounters,
        model_cell_policy_movement: Option<&ModelCellPolicyMovement>,
    ) -> HomeostaticGateReport {
        let mut reason_codes = Vec::new();
        let mut recursive_spawn_allowed = true;
        let mut model_cell_expansion_allowed = true;
        let mut memory_admission_allowed = true;
        let mut genome_mutation_allowed = true;

        if counters.runtime_memory_pressure_milli > self.max_runtime_memory_pressure_milli {
            reason_codes.push("runtime_memory_pressure_high");
            recursive_spawn_allowed = false;
            memory_admission_allowed = false;
            genome_mutation_allowed = false;
        }
        if counters.device_pressure_milli > self.max_device_pressure_milli {
            reason_codes.push("device_pressure_high");
            recursive_spawn_allowed = false;
        }
        if counters.model_pool_saturation_milli > self.max_model_pool_saturation_milli {
            reason_codes.push("model_pool_saturated");
            recursive_spawn_allowed = false;
            model_cell_expansion_allowed = false;
        }
        if counters.failed_model_workers > self.max_failed_model_workers {
            reason_codes.push("model_worker_health_failed");
            recursive_spawn_allowed = false;
            model_cell_expansion_allowed = false;
        }
        if counters.trace_schema_failure_rate_milli > self.max_trace_failure_rate_milli {
            reason_codes.push("trace_schema_failure_rate_high");
            recursive_spawn_allowed = false;
        }
        if counters.benchmark_failure_rate_milli > self.max_benchmark_failure_rate_milli {
            reason_codes.push("benchmark_failure_rate_high");
            recursive_spawn_allowed = false;
        }
        if counters.memory_candidate_backlog > self.max_memory_candidate_backlog {
            reason_codes.push("memory_candidate_backlog_high");
            memory_admission_allowed = false;
        }
        if counters.genome_candidate_backlog > self.max_genome_candidate_backlog {
            reason_codes.push("genome_candidate_backlog_high");
            genome_mutation_allowed = false;
        }
        if counters.verifier_rejection_rate_milli > self.max_verifier_rejection_rate_milli {
            reason_codes.push("verifier_rejection_rate_high");
            memory_admission_allowed = false;
            genome_mutation_allowed = false;
        }
        if counters.rollback_rate_milli > self.max_rollback_rate_milli {
            reason_codes.push("rollback_rate_high");
            genome_mutation_allowed = false;
        }
        if counters.quarantine_rate_milli > self.max_quarantine_rate_milli {
            reason_codes.push("quarantine_rate_high");
            genome_mutation_allowed = false;
        }
        if counters.operator_approval_backlog > self.max_operator_approval_backlog {
            reason_codes.push("operator_approval_backlog_high");
            memory_admission_allowed = false;
            genome_mutation_allowed = false;
        }

        let load_score_milli = [
            counters.runtime_memory_pressure_milli,
            counters.device_pressure_milli,
            counters.model_pool_saturation_milli,
            counters.trace_schema_failure_rate_milli,
            counters.benchmark_failure_rate_milli,
            counters.verifier_rejection_rate_milli,
            counters.rollback_rate_milli,
            counters.quarantine_rate_milli,
        ]
        .into_iter()
        .max()
        .unwrap_or(0);

        let current_overloaded = !reason_codes.is_empty();
        let recovery_pending = !current_overloaded
            && counters.consecutive_high_load_windows > 0
            && counters.recovery_stable_windows < self.min_recovery_stable_windows;
        if recovery_pending {
            reason_codes.push("recovery_window_pending");
            recursive_spawn_allowed = false;
        }
        let model_cell_policy_movement_review_digest = model_cell_policy_movement
            .and_then(|movement| movement.movement_review.as_ref())
            .map(model_cell_policy_movement_review_digest);
        let model_cell_policy_requires_review =
            if let Some(reason) = model_cell_policy_movement_blocker(model_cell_policy_movement) {
                reason_codes.push(reason);
                model_cell_expansion_allowed = false;
                true
            } else {
                false
            };
        let decision = if counters.rollback_rate_milli >= self.emergency_rollback_rate_milli
            || counters.quarantine_rate_milli >= self.emergency_quarantine_rate_milli
        {
            recursive_spawn_allowed = false;
            model_cell_expansion_allowed = false;
            memory_admission_allowed = false;
            genome_mutation_allowed = false;
            HomeostaticGateDecision::EmergencyQuarantine
        } else if counters.operator_approval_backlog > self.max_operator_approval_backlog
            || counters.verifier_rejection_rate_milli > self.max_verifier_rejection_rate_milli
            || model_cell_policy_requires_review
        {
            HomeostaticGateDecision::RequireOperatorReview
        } else if current_overloaded
            && counters.consecutive_high_load_windows >= self.sustained_high_load_windows
        {
            recursive_spawn_allowed = false;
            model_cell_expansion_allowed = false;
            memory_admission_allowed = false;
            genome_mutation_allowed = false;
            HomeostaticGateDecision::PauseSelfEvolution
        } else if counters.failed_model_workers > self.max_failed_model_workers
            || counters.model_pool_saturation_milli > self.max_model_pool_saturation_milli
        {
            HomeostaticGateDecision::RejectNewSpawn
        } else if current_overloaded || recovery_pending {
            HomeostaticGateDecision::DownshiftParallelism
        } else {
            HomeostaticGateDecision::Normal
        };

        HomeostaticGateReport {
            decision,
            reason_codes,
            load_score_milli,
            evidence_digest: homeostatic_evidence_digest(&self, &counters, decision),
            model_cell_policy_movement_review_digest,
            durable_write_allowed: false,
            recursive_spawn_allowed,
            model_cell_expansion_allowed,
            memory_admission_allowed,
            genome_mutation_allowed,
        }
    }
}

fn model_cell_policy_movement_blocker(
    movement: Option<&ModelCellPolicyMovement>,
) -> Option<&'static str> {
    let movement = movement?;
    let moved = movement.source_scope != movement.target_scope;
    let Some(review) = &movement.movement_review else {
        return moved.then_some("model_cell_policy_movement_review_missing");
    };

    if !review.is_preview_only() {
        return Some("model_cell_policy_movement_write_violation");
    }
    if review.source_record_id != movement.policy_id
        || review.source_digest != movement.policy_digest
        || review.source_scope != movement.source_scope
        || review.target_scope != movement.target_scope
    {
        return Some("model_cell_policy_movement_evidence_stale");
    }
    if review
        .forbidden_scope_tags
        .iter()
        .any(|tag| tag == "*" || tag == &movement.target_scope)
    {
        return Some("model_cell_policy_forbidden_target_scope");
    }
    if review.collision_risk {
        return Some("model_cell_policy_neighbor_collision_risk");
    }

    match review.decision {
        MobileGeneMovementDecision::AllowPreviewMove => {
            if moved
                && !review
                    .allowed_scope_tags
                    .iter()
                    .any(|tag| tag == &movement.target_scope)
            {
                Some("model_cell_policy_target_scope_not_allowed")
            } else {
                None
            }
        }
        MobileGeneMovementDecision::HoldForScopeReview => {
            Some("model_cell_policy_hold_for_scope_review")
        }
        MobileGeneMovementDecision::QuarantineMobileElement => {
            Some("model_cell_policy_quarantine_requested")
        }
        MobileGeneMovementDecision::RejectContextJump => {
            Some("model_cell_policy_context_jump_rejected")
        }
    }
}

fn model_cell_policy_movement_review_digest(review: &MobileGeneMovementReview) -> String {
    stable_redaction_digest([
        review.schema_version,
        review.source_record_id.as_str(),
        review.source_digest.as_str(),
        review.source_scope.as_str(),
        review.target_scope.as_str(),
        review.neighbor_context_digest.as_str(),
        review.decision.as_str(),
    ])
}

fn homeostatic_evidence_digest(
    setpoints: &HomeostaticSetpoints,
    counters: &AllostaticLoadCounters,
    decision: HomeostaticGateDecision,
) -> String {
    let parts = [
        "homeostatic-gate-v1".to_owned(),
        decision.as_str().to_owned(),
        setpoints.max_runtime_memory_pressure_milli.to_string(),
        setpoints.max_device_pressure_milli.to_string(),
        setpoints.max_model_pool_saturation_milli.to_string(),
        setpoints.max_failed_model_workers.to_string(),
        setpoints.max_trace_failure_rate_milli.to_string(),
        setpoints.max_benchmark_failure_rate_milli.to_string(),
        setpoints.max_memory_candidate_backlog.to_string(),
        setpoints.max_genome_candidate_backlog.to_string(),
        setpoints.max_verifier_rejection_rate_milli.to_string(),
        setpoints.max_rollback_rate_milli.to_string(),
        setpoints.max_quarantine_rate_milli.to_string(),
        setpoints.max_operator_approval_backlog.to_string(),
        setpoints.sustained_high_load_windows.to_string(),
        setpoints.min_recovery_stable_windows.to_string(),
        setpoints.emergency_rollback_rate_milli.to_string(),
        setpoints.emergency_quarantine_rate_milli.to_string(),
        counters.runtime_memory_pressure_milli.to_string(),
        counters.device_pressure_milli.to_string(),
        counters.model_pool_saturation_milli.to_string(),
        counters.failed_model_workers.to_string(),
        counters.trace_schema_failure_rate_milli.to_string(),
        counters.benchmark_failure_rate_milli.to_string(),
        counters.memory_candidate_backlog.to_string(),
        counters.genome_candidate_backlog.to_string(),
        counters.verifier_rejection_rate_milli.to_string(),
        counters.rollback_rate_milli.to_string(),
        counters.quarantine_rate_milli.to_string(),
        counters.operator_approval_backlog.to_string(),
        counters.consecutive_high_load_windows.to_string(),
        counters.recovery_stable_windows.to_string(),
    ];
    stable_redaction_digest(parts.iter().map(String::as_str))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_load_passes_without_write_authority() {
        let report = HomeostaticSetpoints::default().evaluate(AllostaticLoadCounters {
            recovery_stable_windows: 2,
            ..AllostaticLoadCounters::default()
        });

        assert_eq!(report.decision, HomeostaticGateDecision::Normal);
        assert!(report.reason_codes.is_empty());
        assert!(!report.durable_write_allowed);
        assert!(report.recursive_spawn_allowed);
        assert!(report.model_cell_expansion_allowed);
        assert!(report.memory_admission_allowed);
        assert!(report.genome_mutation_allowed);
        assert!(
            report
                .trace_line()
                .contains("evidence_digest=redaction-digest:")
        );
        assert!(report.trace_line().contains("read_only=true"));
    }

    #[test]
    fn sustained_high_load_pauses_self_evolution() {
        let report = HomeostaticSetpoints::default().evaluate(AllostaticLoadCounters {
            runtime_memory_pressure_milli: 900,
            device_pressure_milli: 870,
            consecutive_high_load_windows: 3,
            ..AllostaticLoadCounters::default()
        });

        assert_eq!(report.decision, HomeostaticGateDecision::PauseSelfEvolution);
        assert!(
            report
                .reason_codes
                .contains(&"runtime_memory_pressure_high")
        );
        assert!(report.reason_codes.contains(&"device_pressure_high"));
        assert!(!report.recursive_spawn_allowed);
        assert!(!report.model_cell_expansion_allowed);
        assert!(!report.memory_admission_allowed);
        assert!(!report.genome_mutation_allowed);
    }

    #[test]
    fn rollback_storm_triggers_emergency_quarantine() {
        let report = HomeostaticSetpoints::default().evaluate(AllostaticLoadCounters {
            rollback_rate_milli: 900,
            quarantine_rate_milli: 200,
            ..AllostaticLoadCounters::default()
        });

        assert_eq!(
            report.decision,
            HomeostaticGateDecision::EmergencyQuarantine
        );
        assert!(report.reason_codes.contains(&"rollback_rate_high"));
        assert!(!report.recursive_spawn_allowed);
        assert!(!report.model_cell_expansion_allowed);
        assert!(!report.memory_admission_allowed);
        assert!(!report.genome_mutation_allowed);
    }

    #[test]
    fn verifier_rejection_spike_requires_operator_review() {
        let report = HomeostaticSetpoints::default().evaluate(AllostaticLoadCounters {
            verifier_rejection_rate_milli: 450,
            ..AllostaticLoadCounters::default()
        });

        assert_eq!(
            report.decision,
            HomeostaticGateDecision::RequireOperatorReview
        );
        assert!(
            report
                .reason_codes
                .contains(&"verifier_rejection_rate_high")
        );
        assert!(report.recursive_spawn_allowed);
        assert!(report.model_cell_expansion_allowed);
        assert!(!report.memory_admission_allowed);
        assert!(!report.genome_mutation_allowed);
    }

    #[test]
    fn model_pool_pressure_rejects_new_spawn() {
        let report = HomeostaticSetpoints::default().evaluate(AllostaticLoadCounters {
            model_pool_saturation_milli: 900,
            failed_model_workers: 1,
            ..AllostaticLoadCounters::default()
        });

        assert_eq!(report.decision, HomeostaticGateDecision::RejectNewSpawn);
        assert!(!report.recursive_spawn_allowed);
        assert!(!report.model_cell_expansion_allowed);
        assert!(report.memory_admission_allowed);
        assert!(report.genome_mutation_allowed);
    }

    #[test]
    fn moved_model_cell_policy_requires_preview_movement_review() {
        let movement = ModelCellPolicyMovement::new(
            "model-cell-policy",
            "redaction-digest:policy",
            "scope-a",
            "scope-b",
        );

        let report = HomeostaticSetpoints::default().evaluate_with_model_cell_policy_movement(
            AllostaticLoadCounters {
                recovery_stable_windows: 2,
                ..AllostaticLoadCounters::default()
            },
            Some(&movement),
        );

        assert_eq!(
            report.decision,
            HomeostaticGateDecision::RequireOperatorReview
        );
        assert!(
            report
                .reason_codes
                .contains(&"model_cell_policy_movement_review_missing")
        );
        assert!(!report.model_cell_expansion_allowed);
        assert_eq!(report.model_cell_policy_movement_review_digest, None);
        assert!(
            report
                .trace_line()
                .contains("model_cell_policy_movement_review=none")
        );
    }

    #[test]
    fn moved_model_cell_policy_with_preview_review_can_expand() {
        let target_scope = "scope-b".to_owned();
        let movement = ModelCellPolicyMovement::new(
            "model-cell-policy",
            "redaction-digest:policy",
            "scope-a",
            target_scope.clone(),
        );
        let review = MobileGeneMovementReview::new(
            movement.policy_id.clone(),
            movement.policy_digest.clone(),
            movement.source_scope.clone(),
            movement.target_scope.clone(),
            "model-cell-policy",
        )
        .with_allowed_scope_tags(vec![target_scope])
        .with_decision(MobileGeneMovementDecision::AllowPreviewMove);

        let report = HomeostaticSetpoints::default().evaluate_with_model_cell_policy_movement(
            AllostaticLoadCounters {
                recovery_stable_windows: 2,
                ..AllostaticLoadCounters::default()
            },
            Some(&movement.with_movement_review(review)),
        );

        assert_eq!(report.decision, HomeostaticGateDecision::Normal);
        assert!(report.reason_codes.is_empty());
        assert!(report.model_cell_expansion_allowed);
        assert!(
            report
                .model_cell_policy_movement_review_digest
                .as_deref()
                .is_some_and(|digest| digest.starts_with("redaction-digest:"))
        );
        assert!(
            report
                .trace_line()
                .contains("model_cell_policy_movement_review=redaction-digest:")
        );
    }

    #[test]
    fn recovery_returns_to_normal() {
        let setpoints = HomeostaticSetpoints::default();
        let stressed = setpoints.evaluate(AllostaticLoadCounters {
            runtime_memory_pressure_milli: 900,
            ..AllostaticLoadCounters::default()
        });
        let pending = setpoints.evaluate(AllostaticLoadCounters {
            consecutive_high_load_windows: 3,
            recovery_stable_windows: 1,
            ..AllostaticLoadCounters::default()
        });
        let recovered = setpoints.evaluate(AllostaticLoadCounters {
            recovery_stable_windows: 2,
            ..AllostaticLoadCounters::default()
        });

        assert_eq!(
            stressed.decision,
            HomeostaticGateDecision::DownshiftParallelism
        );
        assert_eq!(
            pending.decision,
            HomeostaticGateDecision::DownshiftParallelism
        );
        assert!(pending.reason_codes.contains(&"recovery_window_pending"));
        assert_eq!(recovered.decision, HomeostaticGateDecision::Normal);
        assert!(recovered.reason_codes.is_empty());
    }
}
