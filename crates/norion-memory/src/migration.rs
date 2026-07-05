use std::collections::BTreeSet;

use crate::{
    AdapterProjectionIssueSeverity, AdapterWriteMode, DiskKvCatalogVerification,
    MemoryInspectionRiskSeverity, MemoryServiceShadowPlan,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemoryMigrationPhase {
    ReadOnlyShadow,
    CopiedFixtureWrite,
    IsolatedWrite,
    LiveWrite,
}

impl MemoryMigrationPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ReadOnlyShadow => "read_only_shadow",
            Self::CopiedFixtureWrite => "copied_fixture_write",
            Self::IsolatedWrite => "isolated_write",
            Self::LiveWrite => "live_write",
        }
    }

    pub fn required_write_mode(self) -> AdapterWriteMode {
        match self {
            Self::ReadOnlyShadow => AdapterWriteMode::ReadOnly,
            Self::CopiedFixtureWrite | Self::IsolatedWrite => AdapterWriteMode::IsolatedWrite,
            Self::LiveWrite => AdapterWriteMode::LiveWrite,
        }
    }

    pub fn mutates_state(self) -> bool {
        self != Self::ReadOnlyShadow
    }

    pub fn requires_copied_fixture(self) -> bool {
        matches!(self, Self::CopiedFixtureWrite | Self::IsolatedWrite)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryMigrationEvidence {
    pub source_opened_read_only: bool,
    pub copied_fixture_available: bool,
    pub isolated_write_root: bool,
    pub catalog_verified: bool,
    pub checksum_verified: bool,
    pub live_store_targeted: bool,
    pub record_count: Option<usize>,
    pub fixture_detail_codes: Vec<String>,
}

impl MemoryMigrationEvidence {
    pub fn read_only_source(record_count: Option<usize>) -> Self {
        Self {
            source_opened_read_only: true,
            copied_fixture_available: false,
            isolated_write_root: false,
            catalog_verified: false,
            checksum_verified: false,
            live_store_targeted: false,
            record_count,
            fixture_detail_codes: Vec::new(),
        }
    }

    pub fn copied_fixture(record_count: usize) -> Self {
        Self {
            source_opened_read_only: true,
            copied_fixture_available: true,
            isolated_write_root: true,
            catalog_verified: true,
            checksum_verified: true,
            live_store_targeted: false,
            record_count: Some(record_count),
            fixture_detail_codes: Vec::new(),
        }
    }

    pub fn copied_disk_kv_fixture(verification: &DiskKvCatalogVerification) -> Self {
        Self {
            source_opened_read_only: true,
            copied_fixture_available: true,
            isolated_write_root: true,
            catalog_verified: verification.catalog_verified(),
            checksum_verified: verification.checksum_verified(),
            live_store_targeted: false,
            record_count: Some(verification.record_count()),
            fixture_detail_codes: verification
                .detail_codes()
                .into_iter()
                .map(|code| format!("disk_kv_catalog:{code}"))
                .collect(),
        }
    }

    pub fn with_catalog_verified(mut self, catalog_verified: bool) -> Self {
        self.catalog_verified = catalog_verified;
        self
    }

    pub fn with_checksum_verified(mut self, checksum_verified: bool) -> Self {
        self.checksum_verified = checksum_verified;
        self
    }

    pub fn with_live_store_targeted(mut self, live_store_targeted: bool) -> Self {
        self.live_store_targeted = live_store_targeted;
        self
    }

    pub fn with_fixture_detail_codes(mut self, detail_codes: Vec<String>) -> Self {
        self.fixture_detail_codes = detail_codes;
        self
    }

    pub fn guard_codes(&self) -> Vec<String> {
        let mut codes = BTreeSet::new();
        if !self.source_opened_read_only {
            codes.insert("source_not_read_only".to_owned());
        }
        if !self.copied_fixture_available {
            codes.insert("copied_fixture_missing".to_owned());
        }
        if !self.isolated_write_root {
            codes.insert("isolated_write_root_missing".to_owned());
        }
        if !self.catalog_verified {
            codes.insert("fixture_catalog_not_verified".to_owned());
        }
        if !self.checksum_verified {
            codes.insert("fixture_checksum_not_verified".to_owned());
        }
        if self.live_store_targeted {
            codes.insert("live_store_targeted".to_owned());
        }
        if self.record_count.is_none() {
            codes.insert("records_unknown".to_owned());
        }
        codes.into_iter().collect()
    }

    pub fn detail_codes(&self) -> Vec<String> {
        self.fixture_detail_codes
            .iter()
            .cloned()
            .chain(
                self.guard_codes()
                    .into_iter()
                    .map(|code| format!("guard:{code}")),
            )
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn checklist_guard_codes(&self, copied_fixture_required: bool) -> Vec<String> {
        if copied_fixture_required {
            self.guard_codes()
        } else {
            Vec::new()
        }
    }

    pub fn checklist_detail(&self, copied_fixture_required: bool) -> String {
        let guard_codes = self.checklist_guard_codes(copied_fixture_required);
        let detail_codes = if copied_fixture_required {
            self.detail_codes()
        } else {
            Vec::new()
        };
        format!(
            "guard_codes={} detail_codes={}",
            join_codes(&guard_codes),
            join_codes(&detail_codes)
        )
    }

    pub fn summary_line(&self) -> String {
        format!(
            "memory_migration_evidence source_read_only={} copied_fixture={} isolated_write_root={} catalog_verified={} checksum_verified={} live_store_targeted={} records={} guard_codes={} detail_codes={}",
            self.source_opened_read_only,
            self.copied_fixture_available,
            self.isolated_write_root,
            self.catalog_verified,
            self.checksum_verified,
            self.live_store_targeted,
            self.record_count
                .map(|count| count.to_string())
                .unwrap_or_else(|| "unknown".to_owned()),
            join_codes(&self.guard_codes()),
            join_codes(&self.detail_codes()),
        )
    }
}

impl Default for MemoryMigrationEvidence {
    fn default() -> Self {
        Self::read_only_source(None)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryMigrationApprovalPolicy {
    pub allow_live_write: bool,
    pub require_operator_ack_for_live_write: bool,
    pub block_isolated_write_on_review: bool,
}

impl Default for MemoryMigrationApprovalPolicy {
    fn default() -> Self {
        Self {
            allow_live_write: false,
            require_operator_ack_for_live_write: true,
            block_isolated_write_on_review: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryMigrationApproval {
    pub phase: MemoryMigrationPhase,
    pub required_write_mode: AdapterWriteMode,
    pub approved: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
}

impl MemoryMigrationApproval {
    pub fn requires_operator_review(&self) -> bool {
        !self.approved || !self.warnings.is_empty()
    }

    pub fn blocker_codes(&self) -> Vec<String> {
        migration_approval_codes(&self.blockers)
    }

    pub fn warning_codes(&self) -> Vec<String> {
        migration_approval_codes(&self.warnings)
    }

    pub fn blocker_detail_codes(&self) -> Vec<String> {
        migration_approval_detail_codes(&self.blockers)
    }

    pub fn warning_detail_codes(&self) -> Vec<String> {
        migration_approval_detail_codes(&self.warnings)
    }

    pub fn checklist_detail(&self) -> String {
        format!(
            "blockers={} warnings={}",
            self.blockers.len(),
            self.warnings.len()
        )
    }

    pub fn summary_line(&self) -> String {
        format!(
            "memory_migration phase={} approved={} required_write_mode={} blockers={} warnings={} blocker_codes={} warning_codes={} blocker_detail_codes={} warning_detail_codes={} blocker_details={} warning_details={}",
            self.phase.as_str(),
            self.approved,
            self.required_write_mode.as_str(),
            self.blockers.len(),
            self.warnings.len(),
            join_codes(&self.blocker_codes()),
            join_codes(&self.warning_codes()),
            join_codes(&self.blocker_detail_codes()),
            join_codes(&self.warning_detail_codes()),
            if self.blockers.is_empty() {
                "none".to_owned()
            } else {
                self.blockers.join("|")
            },
            if self.warnings.is_empty() {
                "none".to_owned()
            } else {
                self.warnings.join("|")
            },
        )
    }
}

fn migration_approval_codes(items: &[String]) -> Vec<String> {
    let mut codes = BTreeSet::new();
    for item in items {
        let parts = item.split(':').collect::<Vec<_>>();
        let code = match parts.as_slice() {
            ["projection_contract_blocker", _adapter, issue, ..] => {
                format!("projection_contract_blocker:{issue}")
            }
            ["projection_contract", _adapter, issue, ..] => format!("projection_contract:{issue}"),
            ["inspection", severity, issue, ..] => format!("inspection:{severity}:{issue}"),
            ["projection_parity", issue, ..] => format!("projection_parity:{issue}"),
            ["projection", issue, ..] => format!("projection:{issue}"),
            [prefix, detail, ..] => {
                let detail_code = detail
                    .split_once('=')
                    .map_or(*detail, |(code, _value)| code);
                format!("{prefix}:{detail_code}")
            }
            [single] => single
                .split_once('=')
                .map_or(*single, |(code, _value)| code)
                .to_owned(),
            [] => continue,
        };
        if !code.is_empty() {
            codes.insert(code);
        }
    }
    codes.into_iter().collect()
}

fn migration_approval_detail_codes(items: &[String]) -> Vec<String> {
    let mut codes = BTreeSet::new();
    for item in items {
        let parts = item.split(':').collect::<Vec<_>>();
        let code = match parts.as_slice() {
            ["projection_contract_blocker", adapter, issue, detail, ..] => format!(
                "projection_contract_blocker:{}:{}:{}",
                detail_token(adapter),
                detail_token(issue),
                detail_token(detail)
            ),
            ["projection_contract_blocker", adapter, issue] => format!(
                "projection_contract_blocker:{}:{}",
                detail_token(adapter),
                detail_token(issue)
            ),
            ["projection_contract", adapter, issue, detail, ..] => format!(
                "projection_contract:{}:{}:{}",
                detail_token(adapter),
                detail_token(issue),
                detail_token(detail)
            ),
            ["projection_contract", adapter, issue] => format!(
                "projection_contract:{}:{}",
                detail_token(adapter),
                detail_token(issue)
            ),
            ["projection", issue, source_id, ..] => format!(
                "projection:{}:source_id_hex:{}",
                detail_token(issue),
                hex_id(source_id)
            ),
            ["projection", issue] => format!("projection:{}", detail_token(issue)),
            ["projection_blocker", issue, ..] => {
                format!("projection_blocker:{}", detail_token(issue))
            }
            ["projection_parity", field, ..] => {
                format!("projection_parity:{}", detail_token(field))
            }
            ["inspection", severity, issue, ..] => format!(
                "inspection:{}:{}",
                detail_token(severity),
                detail_token(issue)
            ),
            ["disk_kv_catalog", issue, id_hex, ..] => format!(
                "disk_kv_catalog:{}:{}",
                detail_token(issue),
                detail_token(id_hex)
            ),
            [prefix, detail, ..] => format!("{}:{}", detail_token(prefix), metric_detail(detail)),
            [single] => detail_token(single),
            [] => continue,
        };
        if !code.is_empty() {
            codes.insert(code);
        }
    }
    codes.into_iter().collect()
}

fn detail_token(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_owned()
}

fn metric_detail(value: &str) -> String {
    match value.split_once('=') {
        Some((key, metric)) if metric.chars().all(|ch| ch.is_ascii_digit() || ch == '.') => {
            format!("{}={metric}", detail_token(key))
        }
        Some((key, _)) => detail_token(key),
        None => detail_token(value),
    }
}

fn hex_id(id: &str) -> String {
    id.as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join("")
}

fn join_codes(codes: &[String]) -> String {
    if codes.is_empty() {
        "none".to_owned()
    } else {
        codes.join("|")
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DefaultMemoryMigrationGate {
    pub policy: MemoryMigrationApprovalPolicy,
}

impl DefaultMemoryMigrationGate {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_policy(policy: MemoryMigrationApprovalPolicy) -> Self {
        Self { policy }
    }

    pub fn evaluate(
        &self,
        phase: MemoryMigrationPhase,
        plan: &MemoryServiceShadowPlan,
        evidence: &MemoryMigrationEvidence,
        operator_ack: bool,
    ) -> MemoryMigrationApproval {
        let mut blockers = Vec::new();
        let mut warnings = plan_warnings(plan);

        self.push_shadow_blockers(plan, &mut blockers);
        self.push_evidence_blockers(phase, evidence, &mut blockers);

        match phase {
            MemoryMigrationPhase::ReadOnlyShadow => {}
            MemoryMigrationPhase::CopiedFixtureWrite => {
                if !plan.projection_audit.is_ready_for_isolated_write() {
                    blockers.push("projection_not_ready_for_isolated_write".to_owned());
                }
                if !plan.migration_readiness.ready_for_isolated_write {
                    blockers.push("migration_readiness_not_ready_for_isolated_write".to_owned());
                }
            }
            MemoryMigrationPhase::IsolatedWrite => {
                self.push_isolated_write_blockers(plan, &mut blockers);
            }
            MemoryMigrationPhase::LiveWrite => {
                self.push_isolated_write_blockers(plan, &mut blockers);
                if !self.policy.allow_live_write {
                    blockers.push("live_write_disabled_by_policy".to_owned());
                }
                if self.policy.require_operator_ack_for_live_write && !operator_ack {
                    blockers.push("live_write_operator_ack_missing".to_owned());
                }
            }
        }

        if evidence.record_count == Some(0) {
            warnings.push("migration_evidence_empty_fixture".to_owned());
        }

        blockers.sort();
        blockers.dedup();
        warnings.sort();
        warnings.dedup();

        MemoryMigrationApproval {
            phase,
            required_write_mode: phase.required_write_mode(),
            approved: blockers.is_empty(),
            blockers,
            warnings,
        }
    }

    fn push_shadow_blockers(&self, plan: &MemoryServiceShadowPlan, blockers: &mut Vec<String>) {
        if !plan.readiness.ready {
            for capability in &plan.readiness.missing_capabilities {
                blockers.push(format!("readiness_missing:{}", capability.as_str()));
            }
            for capability in &plan.readiness.write_mode_blockers {
                blockers.push(format!("readiness_write_blocker:{}", capability.as_str()));
            }
        }
        for issue in plan.projection_audit.blockers() {
            blockers.push(format!("projection_blocker:{}", issue.code));
        }
        for report in &plan.projection_coverage {
            for blocker in &report.blockers {
                blockers.push(format!(
                    "projection_contract_blocker:{}:{blocker}",
                    report.adapter_name
                ));
            }
        }
    }

    fn push_evidence_blockers(
        &self,
        phase: MemoryMigrationPhase,
        evidence: &MemoryMigrationEvidence,
        blockers: &mut Vec<String>,
    ) {
        if !evidence.source_opened_read_only {
            blockers.push("source_not_opened_read_only".to_owned());
        }
        if evidence.live_store_targeted {
            blockers.push("live_store_targeted_during_migration".to_owned());
        }
        if phase.requires_copied_fixture() {
            if !evidence.copied_fixture_available {
                blockers.push("copied_fixture_missing".to_owned());
            }
            if !evidence.isolated_write_root {
                blockers.push("isolated_write_root_missing".to_owned());
            }
            if !evidence.catalog_verified {
                blockers.push("fixture_catalog_not_verified".to_owned());
            }
            if !evidence.checksum_verified {
                blockers.push("fixture_checksum_not_verified".to_owned());
            }
            blockers.extend(evidence.fixture_detail_codes.iter().cloned());
        }
    }

    fn push_isolated_write_blockers(
        &self,
        plan: &MemoryServiceShadowPlan,
        blockers: &mut Vec<String>,
    ) {
        if !plan.projection_audit.is_ready_for_isolated_write() {
            blockers.push("projection_not_ready_for_isolated_write".to_owned());
        }
        if !plan.migration_readiness.ready_for_isolated_write {
            blockers.push("migration_readiness_not_ready_for_isolated_write".to_owned());
        }
        if !plan.evolution_assessment.allow_isolated_write {
            blockers.extend(
                plan.evolution_assessment
                    .blockers
                    .iter()
                    .map(|blocker| format!("evolution:{blocker}")),
            );
        }
        if plan.inspection.has_blockers() {
            blockers.push("inspection_has_blockers".to_owned());
        }
        if plan.projection_parity_audit.requires_operator_review() {
            blockers.push("projection_parity_requires_operator_review".to_owned());
        }
        if self.policy.block_isolated_write_on_review && plan.requires_operator_review() {
            blockers.push("shadow_plan_requires_operator_review".to_owned());
        }
    }
}

fn plan_warnings(plan: &MemoryServiceShadowPlan) -> Vec<String> {
    let mut warnings = Vec::new();

    warnings.extend(
        plan.readiness
            .warnings
            .iter()
            .map(|warning| format!("readiness:{warning}")),
    );
    warnings.extend(plan.projection_audit.warnings().iter().map(|issue| {
        let id = issue.source_id.as_deref().unwrap_or("unknown");
        format!("projection:{}:{id}", issue.code)
    }));
    for report in &plan.projection_coverage {
        warnings.extend(
            report
                .warnings
                .iter()
                .map(|warning| format!("projection_contract:{}:{warning}", report.adapter_name)),
        );
    }
    warnings.extend(
        plan.migration_readiness
            .warnings
            .iter()
            .map(|warning| format!("migration:{warning}")),
    );
    warnings.extend(
        plan.evolution_assessment
            .warnings
            .iter()
            .map(|warning| format!("evolution:{warning}")),
    );
    warnings.extend(
        plan.inspection
            .risks
            .iter()
            .filter_map(|risk| match risk.severity {
                MemoryInspectionRiskSeverity::Info | MemoryInspectionRiskSeverity::Warning => Some(
                    format!("inspection:{}:{}", risk.severity.as_str(), risk.code),
                ),
                MemoryInspectionRiskSeverity::Blocker => None,
            }),
    );
    warnings.extend(
        plan.projection_parity_audit
            .mismatches
            .iter()
            .map(|mismatch| {
                format!(
                    "projection_parity:{}:{}!={}",
                    mismatch.field, mismatch.expected, mismatch.actual
                )
            }),
    );
    warnings.extend(
        plan.projection_parity_audit
            .warnings
            .iter()
            .map(|warning| format!("projection_parity:{warning}")),
    );
    if plan.read_only.requires_operator_review() {
        warnings.push("read_only_plan_requires_operator_review".to_owned());
    }
    if plan.migration_readiness.operator_review_required {
        warnings.push("migration_operator_review_required".to_owned());
    }
    if plan
        .projection_audit
        .issues
        .iter()
        .any(|issue| issue.severity == AdapterProjectionIssueSeverity::Warning)
    {
        warnings.push("projection_warnings_present".to_owned());
    }
    if plan
        .projection_coverage
        .iter()
        .any(|report| !report.warnings.is_empty())
    {
        warnings.push("projection_contract_warnings_present".to_owned());
    }

    warnings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AdapterProjectionContract, AdapterProjectionField, AdapterProjectionTarget,
        AdapterWriteMode, ExperienceEnvelope, KvTier, MemoryAdapterCapability,
        MemoryAdapterDescriptor, MemoryAdapterHealth, MemoryAdapterStatus, MemoryEvolutionLedger,
        MemoryProjectionMismatch, MemoryScope, MemoryServiceShadowPlanInputs, RetentionMemoryEntry,
    };

    fn status(
        name: &str,
        capabilities: Vec<MemoryAdapterCapability>,
        read_only: bool,
    ) -> MemoryAdapterStatus {
        let mut descriptor = MemoryAdapterDescriptor::new(name, capabilities);
        if read_only {
            descriptor = descriptor.read_only();
        }
        MemoryAdapterStatus::new(
            descriptor,
            MemoryAdapterHealth::ready(Some(1)),
            AdapterWriteMode::ReadOnly,
        )
    }

    fn all_shadow_status() -> MemoryAdapterStatus {
        status(
            "all_shadow",
            vec![
                MemoryAdapterCapability::ExperienceGovernance,
                MemoryAdapterCapability::MemoryIndex,
                MemoryAdapterCapability::ContextInjection,
                MemoryAdapterCapability::RepairPlanning,
                MemoryAdapterCapability::TieredPlacement,
                MemoryAdapterCapability::InfiniMemoryPlanning,
                MemoryAdapterCapability::RetentionPlanning,
                MemoryAdapterCapability::CompactionPlanning,
                MemoryAdapterCapability::MemoryEvolution,
                MemoryAdapterCapability::StateInspection,
                MemoryAdapterCapability::DiskKvOffload,
                MemoryAdapterCapability::KvSwap,
            ],
            true,
        )
    }

    fn clean_plan() -> MemoryServiceShadowPlan {
        let experiences = vec![
            ExperienceEnvelope::new("clean", "prompt", "Stable clean lesson")
                .with_scope(MemoryScope::for_task("runtime"))
                .with_clean_gist("A stable memory summary with enough useful detail.")
                .with_quality(0.9)
                .with_tags(vec!["adapter:test".to_owned()]),
        ];
        let entries = vec![
            RetentionMemoryEntry::new("mem", "semantic durable memory", vec![0.1], 2.4)
                .with_feedback(4, 0)
                .with_access(1, 10),
        ];
        let adapters = vec![all_shadow_status()];
        let seed_ledger = MemoryEvolutionLedger {
            replay_runs: 1,
            replay_items: 1,
            replay_memory_updates: 1,
            ..MemoryEvolutionLedger::default()
        };

        MemoryServiceShadowPlan::for_inputs(
            MemoryServiceShadowPlanInputs::new("clean_shadow", &experiences, &[], &entries)
                .with_adapters(&adapters)
                .with_scope(&MemoryScope::for_task("runtime"))
                .with_evolution_ledger(seed_ledger),
        )
    }

    fn risky_plan() -> MemoryServiceShadowPlan {
        let experiences = vec![ExperienceEnvelope::new(
            "risky",
            "Conversation Transcript:\nUser: inspect prod store\nAssistant: ok",
            "accepted_pattern quality=0.2 max_severity=critical",
        )];
        let metadata = vec![crate::KvShardMetadata {
            id: "cold".to_owned(),
            byte_len: 4,
            checksum: 0,
            tier: KvTier::Cold,
            priority: 0.8,
            last_access: 1,
        }];
        let adapters = vec![all_shadow_status()];

        MemoryServiceShadowPlan::for_inputs(
            MemoryServiceShadowPlanInputs::new("risky_shadow", &experiences, &metadata, &[])
                .with_adapters(&adapters),
        )
    }

    fn plan_with_projection_contract(
        contract: &AdapterProjectionContract,
        target: AdapterProjectionTarget,
    ) -> MemoryServiceShadowPlan {
        let experiences = vec![
            ExperienceEnvelope::new("clean", "prompt", "Stable clean lesson")
                .with_scope(MemoryScope::for_task("runtime"))
                .with_clean_gist("A stable memory summary with enough useful detail.")
                .with_quality(0.9)
                .with_tags(vec!["adapter:test".to_owned()]),
        ];
        let entries = vec![
            RetentionMemoryEntry::new("mem", "semantic durable memory", vec![0.1], 2.4)
                .with_feedback(4, 0)
                .with_access(1, 10),
        ];
        let adapters = vec![all_shadow_status()];
        let seed_ledger = MemoryEvolutionLedger {
            replay_runs: 1,
            replay_items: 1,
            replay_memory_updates: 1,
            ..MemoryEvolutionLedger::default()
        };

        MemoryServiceShadowPlan::for_inputs(
            MemoryServiceShadowPlanInputs::new("contract_shadow", &experiences, &[], &entries)
                .with_adapters(&adapters)
                .with_scope(&MemoryScope::for_task("runtime"))
                .with_evolution_ledger(seed_ledger)
                .with_projection_contracts(std::slice::from_ref(contract), target),
        )
    }

    #[test]
    fn read_only_shadow_allows_warning_only_projection_for_review() {
        let plan = risky_plan();
        let approval = DefaultMemoryMigrationGate::new().evaluate(
            MemoryMigrationPhase::ReadOnlyShadow,
            &plan,
            &MemoryMigrationEvidence::read_only_source(Some(2)),
            false,
        );

        assert!(approval.approved);
        assert!(approval.requires_operator_review());
        assert_eq!(approval.required_write_mode, AdapterWriteMode::ReadOnly);
        assert_eq!(approval.checklist_detail(), "blockers=0 warnings=23");
        assert!(
            approval
                .warnings
                .iter()
                .any(|warning| warning == "projection_warnings_present")
        );
    }

    #[test]
    fn read_only_shadow_surfaces_projection_contract_warnings() {
        let contract = AdapterProjectionContract::experience_store_read_only(
            "experience_shadow",
            vec![
                AdapterProjectionField::ExperienceId,
                AdapterProjectionField::ExperiencePrompt,
                AdapterProjectionField::ExperienceLesson,
                AdapterProjectionField::ExperienceQuality,
            ],
        );
        let plan = plan_with_projection_contract(&contract, AdapterProjectionTarget::ShadowRead);

        let approval = DefaultMemoryMigrationGate::new().evaluate(
            MemoryMigrationPhase::ReadOnlyShadow,
            &plan,
            &MemoryMigrationEvidence::read_only_source(Some(1)),
            false,
        );

        assert!(approval.approved);
        assert!(approval.requires_operator_review());
        assert!(
            approval
                .warnings
                .contains(&"projection_contract_warnings_present".to_owned())
        );
        assert!(
            approval
                .warning_codes()
                .contains(&"projection_contract:missing_recommended".to_owned())
        );
        assert!(
            approval.warning_detail_codes().contains(
                &"projection_contract:experience_shadow:missing_recommended:experience_clean_gist"
                    .to_owned()
            )
        );
        assert!(
            approval
                .summary_line()
                .contains("warning_codes=projection_contract:missing_recommended")
        );
        assert!(approval.summary_line().contains(
            "warning_detail_codes=projection_contract:experience_shadow:missing_recommended:experience_clean_gist"
        ));
        assert!(
            approval
                .summary_line()
                .contains("warning_details=projection_contract:experience_shadow:missing_recommended:experience_clean_gist")
        );
        assert!(approval.warnings.iter().any(|warning| {
            warning.starts_with(
                "projection_contract:experience_shadow:missing_recommended:experience_clean_gist",
            )
        }));
    }

    #[test]
    fn isolated_write_surfaces_projection_contract_blockers() {
        let contract = AdapterProjectionContract::experience_store_read_only(
            "experience_shadow",
            vec![
                AdapterProjectionField::ExperienceId,
                AdapterProjectionField::ExperiencePrompt,
                AdapterProjectionField::ExperienceLesson,
                AdapterProjectionField::ExperienceQuality,
            ],
        );
        let plan = plan_with_projection_contract(&contract, AdapterProjectionTarget::IsolatedWrite);

        let approval = DefaultMemoryMigrationGate::new().evaluate(
            MemoryMigrationPhase::IsolatedWrite,
            &plan,
            &MemoryMigrationEvidence::copied_fixture(1),
            false,
        );

        assert!(!approval.approved);
        assert!(approval.blockers.iter().any(|blocker| {
            blocker.starts_with(
                "projection_contract_blocker:experience_shadow:missing_required:experience_projection_tags",
            )
        }));
        assert!(approval.blockers.iter().any(|blocker| {
            blocker.starts_with(
                "projection_contract_blocker:experience_shadow:write_mode_not_isolated:read_only",
            )
        }));
        assert!(
            approval
                .blocker_codes()
                .contains(&"projection_contract_blocker:missing_required".to_owned())
        );
        assert!(
            approval
                .blocker_codes()
                .contains(&"projection_contract_blocker:write_mode_not_isolated".to_owned())
        );
        assert!(approval.blocker_detail_codes().contains(
            &"projection_contract_blocker:experience_shadow:missing_required:experience_projection_tags"
                .to_owned()
        ));
        assert!(
            approval.blocker_detail_codes().contains(
                &"projection_contract_blocker:experience_shadow:write_mode_not_isolated:read_only"
                    .to_owned()
            )
        );
        assert!(
            approval
                .summary_line()
                .contains("blocker_codes=projection_contract_blocker:missing_required")
        );
        assert!(approval.summary_line().contains(
            "blocker_detail_codes=projection_contract_blocker:experience_shadow:missing_required:experience_projection_tags"
        ));
        assert!(
            approval
                .summary_line()
                .contains("blocker_details=projection_contract_blocker:experience_shadow:missing_required:experience_projection_tags")
        );
    }

    #[test]
    fn copied_fixture_write_requires_verified_isolated_fixture() {
        let plan = clean_plan();
        let evidence = MemoryMigrationEvidence::read_only_source(Some(1));

        let approval = DefaultMemoryMigrationGate::new().evaluate(
            MemoryMigrationPhase::CopiedFixtureWrite,
            &plan,
            &evidence,
            false,
        );

        assert!(!approval.approved);
        assert!(
            approval
                .blockers
                .contains(&"copied_fixture_missing".to_owned())
        );
        assert!(
            approval
                .blockers
                .contains(&"isolated_write_root_missing".to_owned())
        );
    }

    #[test]
    fn migration_evidence_summary_line_reports_fixture_guards() {
        let read_only = MemoryMigrationEvidence::read_only_source(Some(7));
        assert_eq!(
            read_only.guard_codes(),
            vec![
                "copied_fixture_missing".to_owned(),
                "fixture_catalog_not_verified".to_owned(),
                "fixture_checksum_not_verified".to_owned(),
                "isolated_write_root_missing".to_owned(),
            ]
        );
        assert_eq!(
            read_only.detail_codes(),
            vec![
                "guard:copied_fixture_missing".to_owned(),
                "guard:fixture_catalog_not_verified".to_owned(),
                "guard:fixture_checksum_not_verified".to_owned(),
                "guard:isolated_write_root_missing".to_owned(),
            ]
        );
        assert_eq!(read_only.checklist_guard_codes(false), Vec::<String>::new());
        assert_eq!(
            read_only.checklist_detail(false),
            "guard_codes=none detail_codes=none"
        );
        assert_eq!(
            read_only.checklist_guard_codes(true),
            read_only.guard_codes()
        );
        assert_eq!(
            read_only.checklist_detail(true),
            "guard_codes=copied_fixture_missing|fixture_catalog_not_verified|fixture_checksum_not_verified|isolated_write_root_missing detail_codes=guard:copied_fixture_missing|guard:fixture_catalog_not_verified|guard:fixture_checksum_not_verified|guard:isolated_write_root_missing"
        );
        assert_eq!(
            read_only.summary_line(),
            "memory_migration_evidence source_read_only=true copied_fixture=false isolated_write_root=false catalog_verified=false checksum_verified=false live_store_targeted=false records=7 guard_codes=copied_fixture_missing|fixture_catalog_not_verified|fixture_checksum_not_verified|isolated_write_root_missing detail_codes=guard:copied_fixture_missing|guard:fixture_catalog_not_verified|guard:fixture_checksum_not_verified|guard:isolated_write_root_missing"
        );

        let copied = MemoryMigrationEvidence::copied_fixture(3);
        assert_eq!(copied.guard_codes(), Vec::<String>::new());
        assert_eq!(copied.detail_codes(), Vec::<String>::new());
        assert_eq!(
            copied.summary_line(),
            "memory_migration_evidence source_read_only=true copied_fixture=true isolated_write_root=true catalog_verified=true checksum_verified=true live_store_targeted=false records=3 guard_codes=none detail_codes=none"
        );

        let unknown = MemoryMigrationEvidence::default();
        assert_eq!(
            unknown.guard_codes(),
            vec![
                "copied_fixture_missing".to_owned(),
                "fixture_catalog_not_verified".to_owned(),
                "fixture_checksum_not_verified".to_owned(),
                "isolated_write_root_missing".to_owned(),
                "records_unknown".to_owned(),
            ]
        );
        assert_eq!(
            unknown.detail_codes(),
            vec![
                "guard:copied_fixture_missing".to_owned(),
                "guard:fixture_catalog_not_verified".to_owned(),
                "guard:fixture_checksum_not_verified".to_owned(),
                "guard:isolated_write_root_missing".to_owned(),
                "guard:records_unknown".to_owned(),
            ]
        );
        assert_eq!(
            unknown.summary_line(),
            "memory_migration_evidence source_read_only=true copied_fixture=false isolated_write_root=false catalog_verified=false checksum_verified=false live_store_targeted=false records=unknown guard_codes=copied_fixture_missing|fixture_catalog_not_verified|fixture_checksum_not_verified|isolated_write_root_missing|records_unknown detail_codes=guard:copied_fixture_missing|guard:fixture_catalog_not_verified|guard:fixture_checksum_not_verified|guard:isolated_write_root_missing|guard:records_unknown"
        );
    }

    #[test]
    fn isolated_write_passes_with_clean_plan_and_verified_fixture() {
        let plan = clean_plan();
        let approval = DefaultMemoryMigrationGate::new().evaluate(
            MemoryMigrationPhase::IsolatedWrite,
            &plan,
            &MemoryMigrationEvidence::copied_fixture(1),
            false,
        );

        assert!(approval.approved);
        assert!(!approval.requires_operator_review());
        assert_eq!(
            approval.required_write_mode,
            AdapterWriteMode::IsolatedWrite
        );
    }

    #[test]
    fn isolated_write_blocks_context_rot_and_missing_replay_evidence() {
        let plan = risky_plan();
        let approval = DefaultMemoryMigrationGate::new().evaluate(
            MemoryMigrationPhase::IsolatedWrite,
            &plan,
            &MemoryMigrationEvidence::copied_fixture(2),
            false,
        );

        assert!(!approval.approved);
        assert!(
            approval
                .blockers
                .contains(&"projection_not_ready_for_isolated_write".to_owned())
        );
        assert!(
            approval
                .blockers
                .contains(&"evolution:missing_replay_evidence".to_owned())
        );
    }

    #[test]
    fn live_write_is_disabled_by_default_even_after_clean_shadow() {
        let plan = clean_plan();
        let approval = DefaultMemoryMigrationGate::new().evaluate(
            MemoryMigrationPhase::LiveWrite,
            &plan,
            &MemoryMigrationEvidence::copied_fixture(1),
            true,
        );

        assert!(!approval.approved);
        assert!(
            approval
                .blockers
                .contains(&"live_write_disabled_by_policy".to_owned())
        );
    }

    #[test]
    fn disk_kv_verification_evidence_blocks_incomplete_fixture() {
        let plan = clean_plan();
        let verification = DiskKvCatalogVerification {
            missing_byte_ids: vec!["missing".to_owned()],
            byte_len_mismatch_ids: vec!["stale-len".to_owned()],
            checksum_mismatch_ids: vec!["corrupt".to_owned()],
            ..DiskKvCatalogVerification::default()
        };
        let evidence = MemoryMigrationEvidence::copied_disk_kv_fixture(&verification);
        assert_eq!(
            evidence.guard_codes(),
            vec![
                "fixture_catalog_not_verified".to_owned(),
                "fixture_checksum_not_verified".to_owned(),
            ]
        );
        assert_eq!(
            evidence.detail_codes(),
            vec![
                "disk_kv_catalog:byte_len_mismatch:7374616c652d6c656e".to_owned(),
                "disk_kv_catalog:checksum_mismatch:636f7272757074".to_owned(),
                "disk_kv_catalog:missing_bytes:6d697373696e67".to_owned(),
                "guard:fixture_catalog_not_verified".to_owned(),
                "guard:fixture_checksum_not_verified".to_owned(),
            ]
        );
        assert_eq!(
            evidence.summary_line(),
            "memory_migration_evidence source_read_only=true copied_fixture=true isolated_write_root=true catalog_verified=false checksum_verified=false live_store_targeted=false records=0 guard_codes=fixture_catalog_not_verified|fixture_checksum_not_verified detail_codes=disk_kv_catalog:byte_len_mismatch:7374616c652d6c656e|disk_kv_catalog:checksum_mismatch:636f7272757074|disk_kv_catalog:missing_bytes:6d697373696e67|guard:fixture_catalog_not_verified|guard:fixture_checksum_not_verified"
        );

        let approval = DefaultMemoryMigrationGate::new().evaluate(
            MemoryMigrationPhase::CopiedFixtureWrite,
            &plan,
            &evidence,
            false,
        );

        assert!(!approval.approved);
        assert!(
            approval
                .blockers
                .contains(&"fixture_catalog_not_verified".to_owned())
        );
        assert!(
            approval
                .blockers
                .contains(&"fixture_checksum_not_verified".to_owned())
        );
        assert!(
            approval
                .blockers
                .contains(&"disk_kv_catalog:byte_len_mismatch:7374616c652d6c656e".to_owned())
        );
        assert!(
            approval
                .blockers
                .contains(&"disk_kv_catalog:checksum_mismatch:636f7272757074".to_owned())
        );
        assert!(
            approval
                .blockers
                .contains(&"disk_kv_catalog:missing_bytes:6d697373696e67".to_owned())
        );
        assert!(
            approval
                .blocker_detail_codes()
                .contains(&"disk_kv_catalog:byte_len_mismatch:7374616c652d6c656e".to_owned())
        );
        assert!(
            approval
                .blocker_detail_codes()
                .contains(&"disk_kv_catalog:checksum_mismatch:636f7272757074".to_owned())
        );
        assert!(
            approval
                .blocker_detail_codes()
                .contains(&"disk_kv_catalog:missing_bytes:6d697373696e67".to_owned())
        );
        assert!(
            approval.summary_line().contains(
                "blocker_detail_codes=disk_kv_catalog:byte_len_mismatch:7374616c652d6c656e"
            )
        );
    }

    #[test]
    fn isolated_write_blocks_projection_parity_drift() {
        let mut plan = clean_plan();
        plan.projection_parity_audit
            .mismatches
            .push(MemoryProjectionMismatch::new("replay_runs", 2, 1));

        let approval = DefaultMemoryMigrationGate::new().evaluate(
            MemoryMigrationPhase::IsolatedWrite,
            &plan,
            &MemoryMigrationEvidence::copied_fixture(1),
            false,
        );

        assert!(!approval.approved);
        assert!(
            approval
                .blockers
                .contains(&"projection_parity_requires_operator_review".to_owned())
        );
        assert!(
            approval
                .warnings
                .iter()
                .any(|warning| warning.starts_with("projection_parity:replay_runs:"))
        );
        assert!(
            approval
                .warning_detail_codes()
                .contains(&"projection_parity:replay_runs".to_owned())
        );
    }

    #[test]
    fn migration_approval_detail_codes_hex_encode_projection_source_ids() {
        let approval = MemoryMigrationApproval {
            phase: MemoryMigrationPhase::ReadOnlyShadow,
            required_write_mode: AdapterWriteMode::ReadOnly,
            approved: true,
            blockers: Vec::new(),
            warnings: vec!["projection:missing_clean_gist:task/demo transcript".to_owned()],
        };

        assert_eq!(
            approval.warning_detail_codes(),
            vec!["projection:missing_clean_gist:source_id_hex:7461736b2f64656d6f207472616e736372697074".to_owned()]
        );
        assert!(approval.summary_line().contains(
            "warning_detail_codes=projection:missing_clean_gist:source_id_hex:7461736b2f64656d6f207472616e736372697074"
        ));
    }
}
