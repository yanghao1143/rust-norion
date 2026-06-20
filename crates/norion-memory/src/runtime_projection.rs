use crate::{
    MemoryAdapter, MemoryAdapterCapability, MemoryAdapterDescriptor, MemoryAdapterHealth,
    MemoryEvolutionLedger, MemoryInspectionSnapshot, MemoryResult,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryProjectionMismatch {
    pub field: String,
    pub expected: String,
    pub actual: String,
}

impl MemoryProjectionMismatch {
    pub fn new(field: impl Into<String>, expected: impl ToString, actual: impl ToString) -> Self {
        Self {
            field: field.into(),
            expected: expected.to_string(),
            actual: actual.to_string(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MemoryProjectionAudit {
    pub mismatches: Vec<MemoryProjectionMismatch>,
    pub warnings: Vec<String>,
}

impl MemoryProjectionAudit {
    pub fn is_clean(&self) -> bool {
        self.mismatches.is_empty() && self.warnings.is_empty()
    }

    pub fn requires_operator_review(&self) -> bool {
        !self.is_clean()
    }

    pub fn merge(&mut self, mut other: Self) {
        self.mismatches.append(&mut other.mismatches);
        self.warnings.append(&mut other.warnings);
        self.mismatches
            .sort_by(|left, right| left.field.cmp(&right.field));
        self.warnings.sort();
        self.warnings.dedup();
    }

    pub fn mismatch_fields(&self) -> Vec<String> {
        self.mismatches
            .iter()
            .map(|mismatch| projection_detail_part(&mismatch.field))
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn warning_codes(&self) -> Vec<String> {
        self.warnings
            .iter()
            .map(|warning| {
                warning
                    .split_once('=')
                    .or_else(|| warning.split_once(':'))
                    .map_or(warning.as_str(), |(code, _)| code)
            })
            .map(projection_detail_part)
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn detail_codes(&self) -> Vec<String> {
        self.mismatch_fields()
            .into_iter()
            .map(|field| format!("mismatch:{field}"))
            .chain(
                self.warning_codes()
                    .into_iter()
                    .map(|warning| format!("warning:{warning}")),
            )
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn checklist_detail(&self) -> String {
        format!(
            "parity_mismatches={} parity_warnings={}",
            self.mismatches.len(),
            self.warnings.len()
        )
    }

    pub fn summary_line(&self) -> String {
        format!(
            "memory_projection_parity clean={} review={} mismatches={} warnings={} mismatch_fields={} warning_codes={} detail_codes={}",
            self.is_clean(),
            self.requires_operator_review(),
            self.mismatches.len(),
            self.warnings.len(),
            join_codes(&self.mismatch_fields()),
            join_codes(&self.warning_codes()),
            join_codes(&self.detail_codes()),
        )
    }

    fn push_mismatch(
        &mut self,
        field: impl Into<String>,
        expected: impl ToString,
        actual: impl ToString,
    ) {
        self.mismatches
            .push(MemoryProjectionMismatch::new(field, expected, actual));
    }
}

fn projection_detail_part(value: &str) -> String {
    let normalized = value
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == ':' {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_owned();
    if normalized.is_empty() {
        "unknown".to_owned()
    } else {
        normalized
    }
}

fn join_codes(codes: &[String]) -> String {
    if codes.is_empty() {
        "none".to_owned()
    } else {
        codes.join("|")
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AdaptiveStateMemoryProjection {
    pub replay_runs: u64,
    pub replay_items: u64,
    pub replay_memory_updates: u64,
    pub replay_memory_missing: u64,
    pub replay_invalid_memory_ids: u64,
    pub recursive_runtime_items: u64,
    pub live_memory_feedback_items: u64,
    pub context_rot_items: u64,
    pub retention_decays: u64,
    pub retention_removals: u64,
    pub compaction_merges: u64,
    pub compaction_removals: u64,
    pub external_feedback_applied: u64,
    pub external_feedback_missing: u64,
    pub drift_rollbacks: u64,
    pub index_quality_blockers: u64,
    pub index_quality_warnings: u64,
    pub kvswap_boundary_blockers: u64,
    pub kvswap_boundary_warnings: u64,
}

impl AdaptiveStateMemoryProjection {
    pub fn from_ledger(ledger: &MemoryEvolutionLedger) -> Self {
        Self {
            replay_runs: ledger.replay_runs,
            replay_items: ledger.replay_items,
            replay_memory_updates: ledger.replay_memory_updates,
            replay_memory_missing: ledger.replay_memory_missing,
            replay_invalid_memory_ids: ledger.replay_invalid_memory_ids,
            recursive_runtime_items: ledger.recursive_runtime_items,
            live_memory_feedback_items: ledger.live_memory_feedback_items,
            context_rot_items: ledger.context_rot_items,
            retention_decays: ledger.retention_decays,
            retention_removals: ledger.retention_removals,
            compaction_merges: ledger.compaction_merges,
            compaction_removals: ledger.compaction_removals,
            external_feedback_applied: ledger.external_feedback_applied,
            external_feedback_missing: ledger.external_feedback_missing,
            drift_rollbacks: ledger.drift_rollbacks,
            index_quality_blockers: ledger.index_quality_blockers,
            index_quality_warnings: ledger.index_quality_warnings,
            kvswap_boundary_blockers: ledger.kvswap_boundary_blockers,
            kvswap_boundary_warnings: ledger.kvswap_boundary_warnings,
        }
    }

    pub fn to_ledger(&self) -> MemoryEvolutionLedger {
        MemoryEvolutionLedger {
            replay_runs: self.replay_runs,
            replay_items: self.replay_items,
            replay_memory_updates: self.replay_memory_updates,
            replay_memory_missing: self.replay_memory_missing,
            replay_invalid_memory_ids: self.replay_invalid_memory_ids,
            recursive_runtime_items: self.recursive_runtime_items,
            live_memory_feedback_items: self.live_memory_feedback_items,
            context_rot_items: self.context_rot_items,
            retention_decays: self.retention_decays,
            retention_removals: self.retention_removals,
            compaction_merges: self.compaction_merges,
            compaction_removals: self.compaction_removals,
            external_feedback_applied: self.external_feedback_applied,
            external_feedback_missing: self.external_feedback_missing,
            drift_rollbacks: self.drift_rollbacks,
            index_quality_blockers: self.index_quality_blockers,
            index_quality_warnings: self.index_quality_warnings,
            kvswap_boundary_blockers: self.kvswap_boundary_blockers,
            kvswap_boundary_warnings: self.kvswap_boundary_warnings,
            ..MemoryEvolutionLedger::default()
        }
    }

    pub fn audit_ledger(&self, ledger: &MemoryEvolutionLedger) -> MemoryProjectionAudit {
        let actual = Self::from_ledger(ledger);
        let mut audit = MemoryProjectionAudit::default();

        compare_u64(
            &mut audit,
            "replay_runs",
            self.replay_runs,
            actual.replay_runs,
        );
        compare_u64(
            &mut audit,
            "replay_items",
            self.replay_items,
            actual.replay_items,
        );
        compare_u64(
            &mut audit,
            "replay_memory_updates",
            self.replay_memory_updates,
            actual.replay_memory_updates,
        );
        compare_u64(
            &mut audit,
            "replay_memory_missing",
            self.replay_memory_missing,
            actual.replay_memory_missing,
        );
        compare_u64(
            &mut audit,
            "replay_invalid_memory_ids",
            self.replay_invalid_memory_ids,
            actual.replay_invalid_memory_ids,
        );
        compare_u64(
            &mut audit,
            "recursive_runtime_items",
            self.recursive_runtime_items,
            actual.recursive_runtime_items,
        );
        compare_u64(
            &mut audit,
            "live_memory_feedback_items",
            self.live_memory_feedback_items,
            actual.live_memory_feedback_items,
        );
        compare_u64(
            &mut audit,
            "context_rot_items",
            self.context_rot_items,
            actual.context_rot_items,
        );
        compare_u64(
            &mut audit,
            "retention_decays",
            self.retention_decays,
            actual.retention_decays,
        );
        compare_u64(
            &mut audit,
            "retention_removals",
            self.retention_removals,
            actual.retention_removals,
        );
        compare_u64(
            &mut audit,
            "compaction_merges",
            self.compaction_merges,
            actual.compaction_merges,
        );
        compare_u64(
            &mut audit,
            "compaction_removals",
            self.compaction_removals,
            actual.compaction_removals,
        );
        compare_u64(
            &mut audit,
            "external_feedback_applied",
            self.external_feedback_applied,
            actual.external_feedback_applied,
        );
        compare_u64(
            &mut audit,
            "external_feedback_missing",
            self.external_feedback_missing,
            actual.external_feedback_missing,
        );
        compare_u64(
            &mut audit,
            "drift_rollbacks",
            self.drift_rollbacks,
            actual.drift_rollbacks,
        );
        compare_u64(
            &mut audit,
            "index_quality_blockers",
            self.index_quality_blockers,
            actual.index_quality_blockers,
        );
        compare_u64(
            &mut audit,
            "index_quality_warnings",
            self.index_quality_warnings,
            actual.index_quality_warnings,
        );
        compare_u64(
            &mut audit,
            "kvswap_boundary_blockers",
            self.kvswap_boundary_blockers,
            actual.kvswap_boundary_blockers,
        );
        compare_u64(
            &mut audit,
            "kvswap_boundary_warnings",
            self.kvswap_boundary_warnings,
            actual.kvswap_boundary_warnings,
        );

        audit
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StateInspectionMemoryProjection {
    pub memory_count: Option<usize>,
    pub runtime_kv_memory_count: Option<usize>,
    pub experience_count: Option<usize>,
    pub kv_shard_count: Option<usize>,
    pub adapter_count: Option<usize>,
    pub unhealthy_adapter_count: Option<usize>,
    pub projection_blocker_count: Option<usize>,
    pub readiness_missing_capability_count: Option<usize>,
    pub readiness_write_blocker_count: Option<usize>,
    pub evolution_blocker_count: Option<usize>,
    pub evolution_warning_count: Option<usize>,
}

impl StateInspectionMemoryProjection {
    pub fn from_snapshot(snapshot: &MemoryInspectionSnapshot) -> Self {
        Self {
            memory_count: Some(snapshot.memory_count),
            runtime_kv_memory_count: Some(snapshot.runtime_kv_memory_count),
            experience_count: Some(snapshot.experience_count),
            kv_shard_count: Some(snapshot.kv_shard_count),
            adapter_count: Some(snapshot.adapter_count),
            unhealthy_adapter_count: Some(snapshot.unhealthy_adapter_count),
            projection_blocker_count: Some(snapshot.projection_blocker_count),
            readiness_missing_capability_count: Some(snapshot.readiness_missing_capability_count),
            readiness_write_blocker_count: Some(snapshot.readiness_write_blocker_count),
            evolution_blocker_count: Some(snapshot.evolution_blocker_count),
            evolution_warning_count: Some(snapshot.evolution_warning_count),
        }
    }

    pub fn audit_snapshot(&self, snapshot: &MemoryInspectionSnapshot) -> MemoryProjectionAudit {
        let mut audit = MemoryProjectionAudit::default();

        compare_optional_usize(
            &mut audit,
            "memory_count",
            self.memory_count,
            snapshot.memory_count,
        );
        compare_optional_usize(
            &mut audit,
            "runtime_kv_memory_count",
            self.runtime_kv_memory_count,
            snapshot.runtime_kv_memory_count,
        );
        compare_optional_usize(
            &mut audit,
            "experience_count",
            self.experience_count,
            snapshot.experience_count,
        );
        compare_optional_usize(
            &mut audit,
            "kv_shard_count",
            self.kv_shard_count,
            snapshot.kv_shard_count,
        );
        compare_optional_usize(
            &mut audit,
            "adapter_count",
            self.adapter_count,
            snapshot.adapter_count,
        );
        compare_optional_usize(
            &mut audit,
            "unhealthy_adapter_count",
            self.unhealthy_adapter_count,
            snapshot.unhealthy_adapter_count,
        );
        compare_optional_usize(
            &mut audit,
            "projection_blocker_count",
            self.projection_blocker_count,
            snapshot.projection_blocker_count,
        );
        compare_optional_usize(
            &mut audit,
            "readiness_missing_capability_count",
            self.readiness_missing_capability_count,
            snapshot.readiness_missing_capability_count,
        );
        compare_optional_usize(
            &mut audit,
            "readiness_write_blocker_count",
            self.readiness_write_blocker_count,
            snapshot.readiness_write_blocker_count,
        );
        compare_optional_usize(
            &mut audit,
            "evolution_blocker_count",
            self.evolution_blocker_count,
            snapshot.evolution_blocker_count,
        );
        compare_optional_usize(
            &mut audit,
            "evolution_warning_count",
            self.evolution_warning_count,
            snapshot.evolution_warning_count,
        );

        if self.memory_count.is_none() {
            audit
                .warnings
                .push("state_inspection_projection_missing_memory_count".to_owned());
        }
        if self.runtime_kv_memory_count.is_none() {
            audit
                .warnings
                .push("state_inspection_projection_missing_runtime_kv_count".to_owned());
        }

        audit
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultRuntimeStateProjector;

impl MemoryAdapter for DefaultRuntimeStateProjector {
    fn descriptor(&self) -> MemoryAdapterDescriptor {
        MemoryAdapterDescriptor::new(
            "default_runtime_state_projector",
            vec![
                MemoryAdapterCapability::MemoryEvolution,
                MemoryAdapterCapability::StateInspection,
            ],
        )
        .read_only()
    }

    fn health(&self) -> MemoryResult<MemoryAdapterHealth> {
        Ok(MemoryAdapterHealth::ready(None))
    }
}

impl DefaultRuntimeStateProjector {
    pub fn projection_from_ledger(
        &self,
        ledger: &MemoryEvolutionLedger,
    ) -> AdaptiveStateMemoryProjection {
        AdaptiveStateMemoryProjection::from_ledger(ledger)
    }

    pub fn projection_from_snapshot(
        &self,
        snapshot: &MemoryInspectionSnapshot,
    ) -> StateInspectionMemoryProjection {
        StateInspectionMemoryProjection::from_snapshot(snapshot)
    }
}

fn compare_u64(audit: &mut MemoryProjectionAudit, field: &str, expected: u64, actual: u64) {
    if expected != actual {
        audit.push_mismatch(field, expected, actual);
    }
}

fn compare_optional_usize(
    audit: &mut MemoryProjectionAudit,
    field: &str,
    expected: Option<usize>,
    actual: usize,
) {
    if let Some(expected) = expected
        && expected != actual
    {
        audit.push_mismatch(field, expected, actual);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AdapterProjectionAudit, AdapterProjectionIssue, DefaultMemoryInspectionBuilder,
        MemoryAdapterCapability, MemoryEvolutionAssessment, MemoryInspectionBuilder,
        RetentionMemoryEntry,
    };

    #[test]
    fn adaptive_state_projection_round_trips_memory_ledger_subset() {
        let ledger = MemoryEvolutionLedger {
            replay_runs: 2,
            replay_items: 5,
            replay_memory_updates: 4,
            replay_memory_missing: 1,
            replay_invalid_memory_ids: 1,
            recursive_runtime_items: 2,
            live_memory_feedback_items: 3,
            context_rot_items: 1,
            retention_decays: 6,
            retention_removals: 2,
            compaction_merges: 1,
            compaction_removals: 1,
            external_feedback_applied: 3,
            external_feedback_missing: 1,
            drift_rollbacks: 1,
            index_quality_blockers: 2,
            index_quality_warnings: 3,
            kvswap_boundary_blockers: 1,
            kvswap_boundary_warnings: 4,
            ..MemoryEvolutionLedger::default()
        };

        let projection = AdaptiveStateMemoryProjection::from_ledger(&ledger);
        let round_trip = projection.to_ledger();

        assert_eq!(round_trip.replay_runs, ledger.replay_runs);
        assert_eq!(
            round_trip.replay_memory_missing,
            ledger.replay_memory_missing
        );
        assert_eq!(round_trip.context_rot_items, ledger.context_rot_items);
        assert_eq!(round_trip.compaction_removals, ledger.compaction_removals);
        assert_eq!(
            round_trip.index_quality_blockers,
            ledger.index_quality_blockers
        );
        assert_eq!(
            round_trip.kvswap_boundary_warnings,
            ledger.kvswap_boundary_warnings
        );
        assert!(projection.audit_ledger(&ledger).is_clean());
    }

    #[test]
    fn adaptive_state_projection_audit_reports_counter_drift() {
        let ledger = MemoryEvolutionLedger {
            replay_runs: 1,
            context_rot_items: 2,
            ..MemoryEvolutionLedger::default()
        };
        let projection = AdaptiveStateMemoryProjection {
            replay_runs: 2,
            context_rot_items: 2,
            ..AdaptiveStateMemoryProjection::default()
        };

        let audit = projection.audit_ledger(&ledger);

        assert!(audit.requires_operator_review());
        assert_eq!(audit.mismatches.len(), 1);
        assert_eq!(audit.mismatches[0].field, "replay_runs");
        assert_eq!(audit.mismatches[0].expected, "2");
        assert_eq!(audit.mismatches[0].actual, "1");
    }

    #[test]
    fn projection_audit_summary_keeps_parity_details_machine_readable() {
        let mut audit = MemoryProjectionAudit::default();
        audit.push_mismatch("Replay Runs", 2, 1);
        audit
            .warnings
            .push("state_inspection_projection_missing_runtime_kv_count".to_owned());

        assert_eq!(audit.mismatch_fields(), vec!["replay_runs".to_owned()]);
        assert_eq!(
            audit.warning_codes(),
            vec!["state_inspection_projection_missing_runtime_kv_count".to_owned()]
        );
        assert_eq!(
            audit.detail_codes(),
            vec![
                "mismatch:replay_runs".to_owned(),
                "warning:state_inspection_projection_missing_runtime_kv_count".to_owned(),
            ]
        );
        assert_eq!(
            audit.summary_line(),
            "memory_projection_parity clean=false review=true mismatches=1 warnings=1 mismatch_fields=replay_runs warning_codes=state_inspection_projection_missing_runtime_kv_count detail_codes=mismatch:replay_runs|warning:state_inspection_projection_missing_runtime_kv_count"
        );
        assert!(!audit.summary_line().contains("2!=1"));
    }

    #[test]
    fn state_inspection_projection_audits_known_snapshot_counts() {
        let entries = vec![
            RetentionMemoryEntry::new("semantic", "semantic:lesson", vec![0.1], 0.8),
            RetentionMemoryEntry::new("runtime", "runtime_kv:block", vec![0.2], 0.7),
        ];
        let projection_audit = AdapterProjectionAudit {
            experience_count: 1,
            kv_shard_count: 0,
            issues: vec![AdapterProjectionIssue::blocker(
                Some("exp".to_owned()),
                "duplicate_experience_id",
                "duplicate",
            )],
        };
        let evolution = MemoryEvolutionAssessment {
            allow_isolated_write: false,
            blockers: vec!["missing_replay_evidence".to_owned()],
            warnings: vec!["context_rot_items=9".to_owned()],
            rollback_recommended: false,
        };
        let snapshot = DefaultMemoryInspectionBuilder.build(
            &entries,
            1,
            0,
            &[],
            Some(&projection_audit),
            None,
            None,
            Some(&evolution),
            4,
        );
        let projection = StateInspectionMemoryProjection {
            memory_count: Some(2),
            runtime_kv_memory_count: Some(1),
            projection_blocker_count: Some(1),
            evolution_blocker_count: Some(1),
            evolution_warning_count: Some(1),
            ..StateInspectionMemoryProjection::default()
        };

        let audit = projection.audit_snapshot(&snapshot);

        assert!(audit.warnings.is_empty());
        assert!(audit.mismatches.is_empty());
    }

    #[test]
    fn state_inspection_projection_audit_reports_drift_and_missing_core_counts() {
        let snapshot = MemoryInspectionSnapshot {
            memory_count: 3,
            runtime_kv_memory_count: 1,
            experience_count: 2,
            ..MemoryInspectionSnapshot::default()
        };
        let projection = StateInspectionMemoryProjection {
            memory_count: Some(2),
            experience_count: Some(2),
            ..StateInspectionMemoryProjection::default()
        };

        let audit = projection.audit_snapshot(&snapshot);

        assert!(audit.requires_operator_review());
        assert!(
            audit
                .warnings
                .contains(&"state_inspection_projection_missing_runtime_kv_count".to_owned())
        );
        assert_eq!(audit.mismatches.len(), 1);
        assert_eq!(audit.mismatches[0].field, "memory_count");
    }

    #[test]
    fn runtime_state_projector_reports_adapter_capabilities() {
        let projector = DefaultRuntimeStateProjector;
        let descriptor = projector.descriptor();

        assert!(descriptor.read_only);
        assert!(
            descriptor
                .capabilities
                .contains(&MemoryAdapterCapability::MemoryEvolution)
        );
        assert!(
            descriptor
                .capabilities
                .contains(&MemoryAdapterCapability::StateInspection)
        );
        assert!(projector.health().unwrap().ready);
    }
}
