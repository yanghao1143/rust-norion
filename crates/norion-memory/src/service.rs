use crate::{
    AdapterProjectionAudit, AdapterProjectionBundleReport, AdapterProjectionContract,
    AdapterProjectionContractBundle, AdapterProjectionCoverageReport, AdapterProjectionTarget,
    AdapterSnapshotSummary, AdapterWriteMode, CleanGistSelectionReport,
    DefaultAdapterProjectionAuditor, DefaultInfiniMemoryPlanner, DefaultMemoryEvolutionGate,
    DefaultMemoryInspectionBuilder, DefaultMemoryMigrationGate, DefaultMemoryRetentionPlanner,
    ExperienceReplayPlanner, InfiniMemoryActiveMatch, InfiniMemoryPlan, InfiniMemoryPlanner,
    KvSwapBoundaryAudit, KvSwapStateSnapshot, MemoryAdapter, MemoryAdapterCapability,
    MemoryAdapterDescriptor, MemoryAdapterHealth, MemoryCompactionPlan, MemoryCompactionPlanner,
    MemoryCompactionPolicy, MemoryEvolutionAssessment, MemoryEvolutionGate, MemoryEvolutionLedger,
    MemoryHygieneWorkItem, MemoryInspectionBuilder, MemoryInspectionSnapshot,
    MemoryMigrationApproval, MemoryMigrationEvidence, MemoryMigrationPhase, MemoryProjectionAudit,
    MemoryResult, MemoryRetentionPlan, MemoryRetentionPlanner, MemoryRetentionPolicy,
    MigrationReadinessReport, ReadOnlyMemoryPlan, ReplayCandidate, ReplayPlan, ReplayReport,
    RetentionMemoryEntry, StateInspectionMemoryProjection, TierBudgets, TieredMemoryPlan,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryConsumerProfile {
    Core,
    Agent,
    Service,
    ShadowMigration,
}

impl MemoryConsumerProfile {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Core => "core",
            Self::Agent => "agent",
            Self::Service => "service",
            Self::ShadowMigration => "shadow_migration",
        }
    }

    pub fn required_capabilities(self) -> &'static [MemoryAdapterCapability] {
        match self {
            Self::Core => &[
                MemoryAdapterCapability::ShortTermKv,
                MemoryAdapterCapability::LongTermMemory,
                MemoryAdapterCapability::SkillLibrary,
            ],
            Self::Agent => &[
                MemoryAdapterCapability::ShortTermKv,
                MemoryAdapterCapability::LongTermMemory,
                MemoryAdapterCapability::SkillLibrary,
                MemoryAdapterCapability::ExperienceGovernance,
                MemoryAdapterCapability::MemoryIndex,
                MemoryAdapterCapability::ContextInjection,
                MemoryAdapterCapability::ExperienceReplay,
            ],
            Self::Service => &[
                MemoryAdapterCapability::ShortTermKv,
                MemoryAdapterCapability::LongTermMemory,
                MemoryAdapterCapability::SkillLibrary,
                MemoryAdapterCapability::ExperienceGovernance,
                MemoryAdapterCapability::MemoryIndex,
                MemoryAdapterCapability::ContextInjection,
                MemoryAdapterCapability::RepairPlanning,
                MemoryAdapterCapability::TieredPlacement,
                MemoryAdapterCapability::InfiniMemoryPlanning,
                MemoryAdapterCapability::ExperienceReplay,
                MemoryAdapterCapability::RetentionPlanning,
                MemoryAdapterCapability::CompactionPlanning,
                MemoryAdapterCapability::MemoryEvolution,
                MemoryAdapterCapability::StateInspection,
                MemoryAdapterCapability::DiskKvOffload,
                MemoryAdapterCapability::KvSwap,
            ],
            Self::ShadowMigration => &[
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
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryServiceRequirement {
    pub profile: MemoryConsumerProfile,
    pub capabilities: Vec<MemoryAdapterCapability>,
    pub minimum_write_mode: AdapterWriteMode,
}

impl MemoryServiceRequirement {
    pub fn for_profile(
        profile: MemoryConsumerProfile,
        minimum_write_mode: AdapterWriteMode,
    ) -> Self {
        Self {
            profile,
            capabilities: profile.required_capabilities().to_vec(),
            minimum_write_mode,
        }
    }

    pub fn with_capabilities(mut self, capabilities: Vec<MemoryAdapterCapability>) -> Self {
        self.capabilities = sorted_unique_capabilities(capabilities);
        self
    }

    pub fn capability_codes(&self) -> Vec<String> {
        sorted_unique_capabilities(self.capabilities.clone())
            .into_iter()
            .map(|capability| capability.as_str().to_owned())
            .collect()
    }

    pub fn summary_line(&self) -> String {
        format!(
            "memory_service_requirement profile={} minimum_write_mode={} capabilities={} capability_count={}",
            self.profile.as_str(),
            self.minimum_write_mode.as_str(),
            join_codes(self.capability_codes()),
            self.capabilities.len(),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryAdapterStatus {
    pub descriptor: MemoryAdapterDescriptor,
    pub health: MemoryAdapterHealth,
    pub write_mode: AdapterWriteMode,
}

impl MemoryAdapterStatus {
    pub fn new(
        descriptor: MemoryAdapterDescriptor,
        health: MemoryAdapterHealth,
        write_mode: AdapterWriteMode,
    ) -> Self {
        let write_mode = if descriptor.read_only {
            AdapterWriteMode::ReadOnly
        } else {
            write_mode
        };
        Self {
            descriptor,
            health,
            write_mode,
        }
    }

    pub fn inspect<A: MemoryAdapter>(
        adapter: &A,
        write_mode: AdapterWriteMode,
    ) -> MemoryResult<Self> {
        Ok(Self::new(
            adapter.descriptor(),
            adapter.health()?,
            write_mode,
        ))
    }

    pub fn capability_codes(&self) -> Vec<String> {
        sorted_unique_capabilities(self.descriptor.capabilities.clone())
            .into_iter()
            .map(|capability| capability.as_str().to_owned())
            .collect()
    }

    pub fn warning_codes(&self) -> Vec<String> {
        self.health
            .warnings
            .iter()
            .map(|warning| {
                warning
                    .split_once('=')
                    .or_else(|| warning.split_once(':'))
                    .map_or(warning.as_str(), |(code, _value)| code)
                    .to_owned()
            })
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn status_codes(&self) -> Vec<String> {
        let mut codes = Vec::new();
        if !self.health.ready {
            codes.push("unhealthy".to_owned());
        }
        if self.descriptor.read_only {
            codes.push("read_only".to_owned());
        }
        if self.descriptor.capabilities.is_empty() {
            codes.push("empty_capabilities".to_owned());
        }
        if !self.health.warnings.is_empty() {
            codes.push("health_warnings".to_owned());
        }
        if self.write_mode == AdapterWriteMode::LiveWrite {
            codes.push("live_write_enabled".to_owned());
        }
        codes.sort();
        codes.dedup();
        codes
    }

    pub fn summary_line(&self) -> String {
        format!(
            "memory_adapter_status name={} ready={} read_only={} write_mode={} capabilities={} records={} warnings={} status_codes={} warning_codes={}",
            self.descriptor.name,
            self.health.ready,
            self.descriptor.read_only,
            self.write_mode.as_str(),
            join_codes(self.capability_codes()),
            self.health
                .record_count
                .map(|count| count.to_string())
                .unwrap_or_else(|| "unknown".to_owned()),
            self.health.warnings.len(),
            join_codes(self.status_codes()),
            join_codes(self.warning_codes()),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryCapabilityCoverage {
    pub capability: MemoryAdapterCapability,
    pub providers: Vec<String>,
    pub healthy_providers: Vec<String>,
    pub writable_providers: Vec<String>,
    pub read_only_providers: Vec<String>,
    pub record_count: Option<usize>,
}

impl MemoryCapabilityCoverage {
    pub fn has_healthy_provider(&self) -> bool {
        !self.healthy_providers.is_empty()
    }

    pub fn has_writable_provider(&self) -> bool {
        !self.writable_providers.is_empty()
    }

    pub fn status_codes(&self) -> Vec<String> {
        let mut codes = Vec::new();
        if self.providers.is_empty() {
            codes.push("missing_provider".to_owned());
        } else if self.healthy_providers.is_empty() {
            codes.push("no_healthy_provider".to_owned());
        } else if self.writable_providers.is_empty() {
            codes.push("write_mode_blocked".to_owned());
        }
        if self.providers.len() > 1 {
            codes.push("multiple_providers".to_owned());
        }
        codes
    }

    pub fn summary_line(&self) -> String {
        format!(
            "memory_capability_coverage capability={} providers={} healthy={} writable={} read_only={} records={} status_codes={}",
            self.capability.as_str(),
            join_names(&self.providers),
            join_names(&self.healthy_providers),
            join_names(&self.writable_providers),
            join_names(&self.read_only_providers),
            self.record_count
                .map(|count| count.to_string())
                .unwrap_or_else(|| "unknown".to_owned()),
            join_codes(self.status_codes()),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryReadinessReport {
    pub profile: MemoryConsumerProfile,
    pub required_write_mode: AdapterWriteMode,
    pub ready: bool,
    pub adapter_statuses: Vec<MemoryAdapterStatus>,
    pub missing_capabilities: Vec<MemoryAdapterCapability>,
    pub write_mode_blockers: Vec<MemoryAdapterCapability>,
    pub unhealthy_adapters: Vec<String>,
    pub warnings: Vec<String>,
    pub coverage: Vec<MemoryCapabilityCoverage>,
}

impl MemoryReadinessReport {
    pub fn requires_operator_review(&self) -> bool {
        !self.ready || !self.warnings.is_empty()
    }

    pub fn missing_capability_codes(&self) -> Vec<String> {
        self.missing_capabilities
            .iter()
            .map(|capability| capability.as_str().to_owned())
            .collect()
    }

    pub fn write_mode_blocker_codes(&self) -> Vec<String> {
        self.write_mode_blockers
            .iter()
            .map(|capability| capability.as_str().to_owned())
            .collect()
    }

    pub fn warning_codes(&self) -> Vec<String> {
        let mut codes = self
            .warnings
            .iter()
            .map(|warning| {
                if warning.starts_with("capability:") && warning.contains("has multiple providers")
                {
                    "multiple_providers".to_owned()
                } else if warning.contains(':') {
                    "adapter_health_warning".to_owned()
                } else {
                    warning
                        .split_once('=')
                        .map_or(warning.as_str(), |(code, _)| code)
                        .to_owned()
                }
            })
            .collect::<Vec<_>>();
        codes.sort();
        codes.dedup();
        codes
    }

    pub fn capability_manifest_checklist_detail(&self) -> String {
        format!(
            "missing={} write_blockers={}",
            self.missing_capabilities.len(),
            self.write_mode_blockers.len()
        )
    }

    pub fn summary_line(&self) -> String {
        format!(
            "memory_readiness profile={} required_write_mode={} ready={} review={} missing={} write_blockers={} unhealthy={} warnings={} missing_codes={} write_blocker_codes={} warning_codes={}",
            self.profile.as_str(),
            self.required_write_mode.as_str(),
            self.ready,
            self.requires_operator_review(),
            self.missing_capabilities.len(),
            self.write_mode_blockers.len(),
            self.unhealthy_adapters.len(),
            self.warnings.len(),
            join_codes(self.missing_capability_codes()),
            join_codes(self.write_mode_blocker_codes()),
            join_codes(self.warning_codes()),
        )
    }

    pub fn coverage_summary_lines(&self) -> Vec<String> {
        self.coverage
            .iter()
            .map(MemoryCapabilityCoverage::summary_line)
            .collect()
    }

    pub fn adapter_summary_lines(&self) -> Vec<String> {
        self.adapter_statuses
            .iter()
            .map(MemoryAdapterStatus::summary_line)
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct MemoryServiceShadowPlanInputs<'a> {
    pub adapter_name: String,
    pub experiences: &'a [crate::ExperienceEnvelope],
    pub kv_metadata: &'a [crate::KvShardMetadata],
    pub memory_entries: &'a [RetentionMemoryEntry],
    pub replay_candidates: &'a [ReplayCandidate],
    pub active_matches: &'a [InfiniMemoryActiveMatch],
    pub adapters: &'a [MemoryAdapterStatus],
    pub adapter_snapshots: &'a [AdapterSnapshotSummary],
    pub clean_gist_selection_reports: &'a [CleanGistSelectionReport],
    pub requirement: MemoryServiceRequirement,
    pub projection_contracts: &'a [AdapterProjectionContract],
    pub projection_contract_target: AdapterProjectionTarget,
    pub projection_contract_bundle_name: Option<String>,
    pub scope: Option<&'a crate::MemoryScope>,
    pub tier_budgets: TierBudgets,
    pub previous_placement: Option<&'a TieredMemoryPlan>,
    pub target_hot_bytes: usize,
    pub now: u64,
    pub retention_policy: MemoryRetentionPolicy,
    pub compaction_policy: MemoryCompactionPolicy,
    pub protected_memory_ids: Vec<String>,
    pub kvswap_state: Option<KvSwapStateSnapshot>,
    pub kvswap_boundary: Option<KvSwapBoundaryAudit>,
    pub seed_evolution_ledger: Option<MemoryEvolutionLedger>,
    pub adaptive_state_projection: Option<&'a crate::AdaptiveStateMemoryProjection>,
    pub state_inspection_projection: Option<&'a StateInspectionMemoryProjection>,
    pub replay_limit: usize,
    pub inspection_limit: usize,
}

impl<'a> MemoryServiceShadowPlanInputs<'a> {
    pub fn new(
        adapter_name: impl Into<String>,
        experiences: &'a [crate::ExperienceEnvelope],
        kv_metadata: &'a [crate::KvShardMetadata],
        memory_entries: &'a [RetentionMemoryEntry],
    ) -> Self {
        Self {
            adapter_name: adapter_name.into(),
            experiences,
            kv_metadata,
            memory_entries,
            replay_candidates: &[],
            active_matches: &[],
            adapters: &[],
            adapter_snapshots: &[],
            clean_gist_selection_reports: &[],
            requirement: MemoryServiceRequirement::for_profile(
                MemoryConsumerProfile::ShadowMigration,
                AdapterWriteMode::ReadOnly,
            ),
            projection_contracts: &[],
            projection_contract_target: AdapterProjectionTarget::ShadowRead,
            projection_contract_bundle_name: None,
            scope: None,
            tier_budgets: TierBudgets::new(4, 16),
            previous_placement: None,
            target_hot_bytes: 0,
            now: 0,
            retention_policy: MemoryRetentionPolicy::default(),
            compaction_policy: MemoryCompactionPolicy::default(),
            protected_memory_ids: Vec::new(),
            kvswap_state: None,
            kvswap_boundary: None,
            seed_evolution_ledger: None,
            adaptive_state_projection: None,
            state_inspection_projection: None,
            replay_limit: 8,
            inspection_limit: 8,
        }
    }

    pub fn with_active_matches(mut self, active_matches: &'a [InfiniMemoryActiveMatch]) -> Self {
        self.active_matches = active_matches;
        self
    }

    pub fn with_replay_candidates(mut self, replay_candidates: &'a [ReplayCandidate]) -> Self {
        self.replay_candidates = replay_candidates;
        self
    }

    pub fn with_adapters(mut self, adapters: &'a [MemoryAdapterStatus]) -> Self {
        self.adapters = adapters;
        self
    }

    pub fn with_adapter_snapshots(
        mut self,
        adapter_snapshots: &'a [AdapterSnapshotSummary],
    ) -> Self {
        self.adapter_snapshots = adapter_snapshots;
        self
    }

    pub fn with_clean_gist_selection_reports(
        mut self,
        reports: &'a [CleanGistSelectionReport],
    ) -> Self {
        self.clean_gist_selection_reports = reports;
        self
    }

    pub fn with_requirement(mut self, requirement: MemoryServiceRequirement) -> Self {
        self.requirement = requirement;
        self
    }

    pub fn with_projection_contracts(
        mut self,
        contracts: &'a [AdapterProjectionContract],
        target: AdapterProjectionTarget,
    ) -> Self {
        self.projection_contracts = contracts;
        self.projection_contract_target = target;
        self.projection_contract_bundle_name = None;
        self
    }

    pub fn with_projection_contract_bundle(
        mut self,
        bundle: &'a AdapterProjectionContractBundle,
    ) -> Self {
        self.projection_contracts = &bundle.contracts;
        self.projection_contract_target = bundle.target;
        self.projection_contract_bundle_name = Some(bundle.name.clone());
        self
    }

    pub fn with_scope(mut self, scope: &'a crate::MemoryScope) -> Self {
        self.scope = Some(scope);
        self
    }

    pub fn with_tier_plan(
        mut self,
        tier_budgets: TierBudgets,
        previous_placement: Option<&'a TieredMemoryPlan>,
        target_hot_bytes: usize,
    ) -> Self {
        self.tier_budgets = tier_budgets;
        self.previous_placement = previous_placement;
        self.target_hot_bytes = target_hot_bytes;
        self
    }

    pub fn with_maintenance(
        mut self,
        now: u64,
        retention_policy: MemoryRetentionPolicy,
        compaction_policy: MemoryCompactionPolicy,
        protected_memory_ids: Vec<String>,
    ) -> Self {
        self.now = now;
        self.retention_policy = retention_policy;
        self.compaction_policy = compaction_policy;
        self.protected_memory_ids = protected_memory_ids;
        self
    }

    pub fn with_kvswap_state(mut self, snapshot: KvSwapStateSnapshot) -> Self {
        self.kvswap_state = Some(snapshot);
        self
    }

    pub fn with_kvswap_boundary(mut self, audit: KvSwapBoundaryAudit) -> Self {
        self.kvswap_boundary = Some(audit);
        self
    }

    pub fn with_evolution_ledger(mut self, ledger: MemoryEvolutionLedger) -> Self {
        self.seed_evolution_ledger = Some(ledger);
        self
    }

    pub fn with_adaptive_state_projection(
        mut self,
        projection: &'a crate::AdaptiveStateMemoryProjection,
    ) -> Self {
        self.adaptive_state_projection = Some(projection);
        self
    }

    pub fn with_state_inspection_projection(
        mut self,
        projection: &'a StateInspectionMemoryProjection,
    ) -> Self {
        self.state_inspection_projection = Some(projection);
        self
    }

    pub fn with_replay_limit(mut self, replay_limit: usize) -> Self {
        self.replay_limit = replay_limit.max(1);
        self
    }

    pub fn with_inspection_limit(mut self, inspection_limit: usize) -> Self {
        self.inspection_limit = inspection_limit.max(1);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryServiceShadowPlan {
    pub requirement: MemoryServiceRequirement,
    pub readiness: MemoryReadinessReport,
    pub adapter_snapshots: Vec<AdapterSnapshotSummary>,
    pub clean_gist_selection_reports: Vec<CleanGistSelectionReport>,
    pub projection_contract_bundle_manifest: Option<String>,
    pub projection_contract_manifests: Vec<String>,
    pub projection_coverage: Vec<AdapterProjectionCoverageReport>,
    pub projection_bundle_summary: Option<AdapterProjectionBundleReport>,
    pub projection_audit: AdapterProjectionAudit,
    pub read_only: ReadOnlyMemoryPlan,
    pub request_scope_missing: bool,
    pub migration_readiness: MigrationReadinessReport,
    pub replay: ReplayPlan,
    pub replay_report: ReplayReport,
    pub infini: InfiniMemoryPlan,
    pub retention: MemoryRetentionPlan,
    pub compaction: MemoryCompactionPlan,
    pub kvswap_state: Option<KvSwapStateSnapshot>,
    pub kvswap_boundary: Option<KvSwapBoundaryAudit>,
    pub evolution_ledger: MemoryEvolutionLedger,
    pub evolution_assessment: MemoryEvolutionAssessment,
    pub inspection: MemoryInspectionSnapshot,
    pub projection_parity_audit: MemoryProjectionAudit,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MemoryServiceShadowSummary {
    pub ready: bool,
    pub requires_operator_review: bool,
    pub experience_count: usize,
    pub kv_shard_count: usize,
    pub memory_count: usize,
    pub runtime_kv_memory_count: usize,
    pub projection_issue_count: usize,
    pub projection_blocker_count: usize,
    pub adapter_snapshot_count: usize,
    pub adapter_snapshot_warning_count: usize,
    pub projection_contract_count: usize,
    pub projection_contract_manifest_count: usize,
    pub projection_contract_blocker_count: usize,
    pub projection_contract_warning_count: usize,
    pub projection_parity_mismatch_count: usize,
    pub projection_parity_warning_count: usize,
    pub readiness_missing_capability_count: usize,
    pub readiness_write_blocker_count: usize,
    pub context_rejection_count: usize,
    pub context_rot_risk_count: usize,
    pub context_rot_risk_reason_codes: Vec<String>,
    pub context_rot_blocker_reason_codes: Vec<String>,
    pub context_rot_risk_detail_codes: Vec<String>,
    pub clean_gist_repair_missing_clean_gist_count: usize,
    pub clean_gist_repair_dirty_clean_gist_count: usize,
    pub clean_gist_repair_dirty_gist_count: usize,
    pub clean_gist_repair_detail_codes: Vec<String>,
    pub repair_item_count: usize,
    pub repair_skipped_count: usize,
    pub replay_planned_count: usize,
    pub replay_memory_update_count: usize,
    pub replay_context_rot_count: usize,
    pub kvswap_prefetch_count: usize,
    pub kvswap_evict_count: usize,
    pub kvswap_state_present: bool,
    pub kvswap_hot_shard_count: usize,
    pub kvswap_cold_shard_count: usize,
    pub kvswap_metadata_count: usize,
    pub kvswap_total_byte_len: usize,
    pub kvswap_shape_codes: Vec<String>,
    pub kvswap_boundary_present: bool,
    pub kvswap_boundary_issue_count: usize,
    pub kvswap_boundary_reason_codes: Vec<String>,
    pub kvswap_boundary_detail_codes: Vec<String>,
    pub kvswap_boundary_overlap_count: usize,
    pub kvswap_boundary_missing_hot_metadata_count: usize,
    pub kvswap_boundary_stale_metadata_count: usize,
    pub kvswap_boundary_hot_tier_mismatch_count: usize,
    pub kvswap_boundary_cold_tier_mismatch_count: usize,
    pub retention_decay_count: usize,
    pub retention_removal_count: usize,
    pub compaction_merge_count: usize,
    pub compaction_removal_count: usize,
    pub evolution_blocker_count: usize,
    pub evolution_warning_count: usize,
    pub hygiene_pressure_score: u64,
    pub hygiene_pressure_priority: String,
    pub hygiene_pressure_action_lanes: Vec<String>,
    pub hygiene_pressure_action_lane_details: Vec<String>,
    pub hygiene_pressure_reason_codes: Vec<String>,
    pub hygiene_pressure_detail_codes: Vec<String>,
    pub hygiene_work_next_action: String,
    pub hygiene_work_operator_review_required: bool,
    pub hygiene_work_isolation_recommended: bool,
    pub hygiene_work_queue_item_count: usize,
    pub hygiene_work_queue_operator_review_count: usize,
    pub hygiene_work_queue_isolation_count: usize,
    pub hygiene_work_queue_next_dispatch: String,
    pub hygiene_work_queue_lane_codes: Vec<String>,
    pub hygiene_work_queue_priority_codes: Vec<String>,
    pub hygiene_work_queue_dispatch_codes: Vec<String>,
    pub hygiene_work_queue_detail_codes: Vec<String>,
    pub hygiene_work_queue_reason_codes: Vec<String>,
    pub inspection_risk_count: usize,
    pub review_reasons: Vec<String>,
    pub review_detail_codes: Vec<String>,
}

impl MemoryServiceShadowSummary {
    pub fn from_plan(plan: &MemoryServiceShadowPlan) -> Self {
        let mut review_reasons = Vec::new();
        if !plan.readiness.ready {
            if !plan.readiness.missing_capabilities.is_empty() {
                review_reasons.push("readiness_missing_capabilities".to_owned());
            }
            if !plan.readiness.write_mode_blockers.is_empty() {
                review_reasons.push("readiness_write_blockers".to_owned());
            }
        }
        if !plan.readiness.warnings.is_empty() {
            review_reasons.push("readiness_warnings".to_owned());
        }
        if !plan.projection_audit.blockers().is_empty() {
            review_reasons.push("projection_blockers".to_owned());
        }
        if !plan.projection_audit.warnings().is_empty() {
            review_reasons.push("projection_warnings".to_owned());
        }
        if plan
            .adapter_snapshots
            .iter()
            .any(|snapshot| !snapshot.warnings.is_empty())
        {
            review_reasons.push("adapter_snapshot_warnings".to_owned());
        }
        if plan.projection_coverage.iter().any(|report| !report.ready) {
            review_reasons.push("projection_contract_blockers".to_owned());
        }
        if plan
            .projection_coverage
            .iter()
            .any(|report| !report.warnings.is_empty())
        {
            review_reasons.push("projection_contract_warnings".to_owned());
        }
        if plan.read_only.requires_operator_review() {
            review_reasons.push("read_only_plan_review".to_owned());
        }
        if plan.request_scope_missing {
            review_reasons.push("missing_request_scope".to_owned());
        }
        if plan.migration_readiness.operator_review_required {
            review_reasons.push("migration_readiness_review".to_owned());
        }
        if plan.evolution_assessment.requires_operator_review() {
            review_reasons.push("evolution_review".to_owned());
        }
        if plan.inspection.has_blockers() {
            review_reasons.push("inspection_blockers".to_owned());
        }
        if plan.projection_parity_audit.requires_operator_review() {
            review_reasons.push("projection_parity_review".to_owned());
        }
        if plan
            .kvswap_boundary
            .as_ref()
            .is_some_and(|audit| !audit.is_clean())
        {
            review_reasons.push("kvswap_boundary_review".to_owned());
        }
        review_reasons.sort();
        review_reasons.dedup();
        let review_detail_codes = collect_shadow_review_detail_codes(plan);
        let kvswap_state = plan.kvswap_state.unwrap_or_default();
        let kvswap_shape_codes = plan
            .kvswap_state
            .map(|snapshot| snapshot.shape_codes())
            .unwrap_or_default();
        let kvswap_boundary = plan.kvswap_boundary.clone().unwrap_or_default();
        let hygiene_pressure = plan.evolution_ledger.hygiene_pressure();
        let hygiene_work_plan = hygiene_pressure.work_plan();
        let hygiene_work_queue = hygiene_work_plan.work_queue();
        let hygiene_work_queue_reason_codes = hygiene_work_queue.reason_codes();

        Self {
            ready: plan.readiness.ready
                && plan.projection_audit.is_ready_for_shadow_read()
                && !plan.inspection.has_blockers(),
            requires_operator_review: plan.requires_operator_review(),
            experience_count: plan.projection_audit.experience_count,
            kv_shard_count: plan.projection_audit.kv_shard_count,
            memory_count: plan.inspection.memory_count,
            runtime_kv_memory_count: plan.inspection.runtime_kv_memory_count,
            projection_issue_count: plan.projection_audit.issues.len(),
            projection_blocker_count: plan.projection_audit.blockers().len(),
            adapter_snapshot_count: plan.adapter_snapshots.len(),
            adapter_snapshot_warning_count: plan
                .adapter_snapshots
                .iter()
                .map(|snapshot| snapshot.warnings.len())
                .sum(),
            projection_contract_count: plan.projection_coverage.len(),
            projection_contract_manifest_count: plan.projection_contract_manifests.len(),
            projection_contract_blocker_count: plan
                .projection_coverage
                .iter()
                .map(|report| report.blockers.len())
                .sum(),
            projection_contract_warning_count: plan
                .projection_coverage
                .iter()
                .map(|report| report.warnings.len())
                .sum(),
            projection_parity_mismatch_count: plan.projection_parity_audit.mismatches.len(),
            projection_parity_warning_count: plan.projection_parity_audit.warnings.len(),
            readiness_missing_capability_count: plan.readiness.missing_capabilities.len(),
            readiness_write_blocker_count: plan.readiness.write_mode_blockers.len(),
            context_rejection_count: plan.read_only.context.rejected_ids().len(),
            context_rot_risk_count: plan.read_only.governance.context_rot_risks.len(),
            context_rot_risk_reason_codes: context_rot_risk_reason_codes(plan),
            context_rot_blocker_reason_codes: plan
                .read_only
                .governance
                .context_rot_blocker_reason_codes(),
            context_rot_risk_detail_codes: context_rot_risk_detail_codes(plan),
            clean_gist_repair_missing_clean_gist_count: plan
                .read_only
                .rebuild
                .missing_clean_gist_ids
                .len(),
            clean_gist_repair_dirty_clean_gist_count: plan
                .read_only
                .rebuild
                .dirty_clean_gist_ids
                .len(),
            clean_gist_repair_dirty_gist_count: plan.read_only.rebuild.dirty_gist_ids.len(),
            clean_gist_repair_detail_codes: plan.read_only.rebuild.clean_gist_repair_detail_codes(),
            repair_item_count: plan.read_only.repair.items.len(),
            repair_skipped_count: plan.read_only.repair.skipped.len(),
            replay_planned_count: plan.replay_report.planned,
            replay_memory_update_count: plan.replay_report.memory_reinforcements
                + plan.replay_report.memory_penalties,
            replay_context_rot_count: plan.replay_report.context_rot_items,
            kvswap_prefetch_count: plan.read_only.kvswap.prefetch.promote_ids.len(),
            kvswap_evict_count: plan.read_only.kvswap.evict.demote_ids.len(),
            kvswap_state_present: plan.kvswap_state.is_some(),
            kvswap_hot_shard_count: kvswap_state.hot_shard_count,
            kvswap_cold_shard_count: kvswap_state.cold_shard_count,
            kvswap_metadata_count: kvswap_state.metadata_count,
            kvswap_total_byte_len: kvswap_state.total_byte_len(),
            kvswap_shape_codes,
            kvswap_boundary_present: plan.kvswap_boundary.is_some(),
            kvswap_boundary_issue_count: kvswap_boundary.issue_count(),
            kvswap_boundary_reason_codes: plan
                .kvswap_boundary
                .as_ref()
                .map(KvSwapBoundaryAudit::reason_codes)
                .unwrap_or_default(),
            kvswap_boundary_detail_codes: plan
                .kvswap_boundary
                .as_ref()
                .map(KvSwapBoundaryAudit::detail_codes)
                .unwrap_or_default(),
            kvswap_boundary_overlap_count: kvswap_boundary.overlapping_hot_cold_ids.len(),
            kvswap_boundary_missing_hot_metadata_count: kvswap_boundary
                .missing_hot_metadata_ids
                .len(),
            kvswap_boundary_stale_metadata_count: kvswap_boundary.stale_metadata_ids.len(),
            kvswap_boundary_hot_tier_mismatch_count: kvswap_boundary.hot_tier_mismatch_ids.len(),
            kvswap_boundary_cold_tier_mismatch_count: kvswap_boundary.cold_tier_mismatch_ids.len(),
            retention_decay_count: plan.retention.decays.len(),
            retention_removal_count: plan.retention.removals.len(),
            compaction_merge_count: plan.compaction.merges.len(),
            compaction_removal_count: plan.compaction.removed_ids.len(),
            evolution_blocker_count: plan.evolution_assessment.blockers.len(),
            evolution_warning_count: plan.evolution_assessment.warnings.len(),
            hygiene_pressure_score: hygiene_pressure.score,
            hygiene_pressure_priority: hygiene_pressure.priority_code().to_owned(),
            hygiene_pressure_action_lanes: hygiene_pressure.action_lane_codes(),
            hygiene_pressure_action_lane_details: hygiene_pressure.action_lane_detail_codes(),
            hygiene_pressure_reason_codes: hygiene_pressure.reason_codes(),
            hygiene_pressure_detail_codes: hygiene_pressure.detail_codes(),
            hygiene_work_next_action: hygiene_work_plan.next_action_code,
            hygiene_work_operator_review_required: hygiene_work_plan.operator_review_required,
            hygiene_work_isolation_recommended: hygiene_work_plan.isolation_recommended,
            hygiene_work_queue_item_count: hygiene_work_queue.item_count,
            hygiene_work_queue_operator_review_count: hygiene_work_queue.operator_review_count,
            hygiene_work_queue_isolation_count: hygiene_work_queue.isolation_count,
            hygiene_work_queue_next_dispatch: hygiene_work_queue.next_dispatch_code,
            hygiene_work_queue_lane_codes: hygiene_work_queue.lane_codes,
            hygiene_work_queue_priority_codes: hygiene_work_queue.priority_codes,
            hygiene_work_queue_dispatch_codes: hygiene_work_queue.dispatch_codes,
            hygiene_work_queue_detail_codes: hygiene_work_queue.detail_codes,
            hygiene_work_queue_reason_codes,
            inspection_risk_count: plan.inspection.risks.len(),
            review_reasons,
            review_detail_codes,
        }
    }

    pub fn clean_gist_repair_issue_count(&self) -> usize {
        self.clean_gist_repair_missing_clean_gist_count
            .saturating_add(self.clean_gist_repair_dirty_clean_gist_count)
            .saturating_add(self.clean_gist_repair_dirty_gist_count)
    }

    pub fn clean_gist_repair_is_clean(&self) -> bool {
        self.clean_gist_repair_issue_count() == 0 && self.clean_gist_repair_detail_codes.is_empty()
    }

    pub fn adapter_snapshot_checklist_detail(&self) -> String {
        if self.adapter_snapshot_warning_count == 0 {
            format!("warnings={}", self.adapter_snapshot_warning_count)
        } else {
            format!(
                "snapshots={} warnings={}",
                self.adapter_snapshot_count, self.adapter_snapshot_warning_count
            )
        }
    }

    pub fn context_gate_checklist_detail(&self) -> String {
        format!("context_rejections={}", self.context_rejection_count)
    }

    pub fn projection_contracts_ready_checklist_detail(&self) -> String {
        format!(
            "contracts={} blockers={}",
            self.projection_contract_count, self.projection_contract_blocker_count
        )
    }

    pub fn projection_contract_warnings_checklist_detail(&self) -> String {
        format!(
            "contracts={} warnings={}",
            self.projection_contract_count, self.projection_contract_warning_count
        )
    }

    pub fn repair_plan_checklist_detail(&self) -> String {
        format!(
            "repair_items={} repair_skipped={}",
            self.repair_item_count, self.repair_skipped_count
        )
    }

    pub fn kvswap_intent_checklist_detail(&self) -> String {
        format!(
            "prefetch={} evict={}",
            self.kvswap_prefetch_count, self.kvswap_evict_count
        )
    }

    pub fn clean_gist_repair_checklist_detail(&self) -> String {
        format!(
            "missing_clean_gist={} dirty_clean_gist={} dirty_gist={} clean_gist_repair_detail_codes={}",
            self.clean_gist_repair_missing_clean_gist_count,
            self.clean_gist_repair_dirty_clean_gist_count,
            self.clean_gist_repair_dirty_gist_count,
            join_codes(self.clean_gist_repair_detail_codes.clone())
        )
    }

    pub fn context_rot_risk_is_clean(&self) -> bool {
        self.context_rot_risk_count == 0
            && self.context_rot_risk_reason_codes.is_empty()
            && self.context_rot_blocker_reason_codes.is_empty()
            && self.context_rot_risk_detail_codes.is_empty()
    }

    pub fn context_rot_risk_checklist_detail(&self) -> String {
        format!(
            "context_rot_risks={} context_rot_risk_reason_codes={} context_rot_blocker_reason_codes={} context_rot_risk_detail_codes={}",
            self.context_rot_risk_count,
            join_codes(self.context_rot_risk_reason_codes.clone()),
            join_codes(self.context_rot_blocker_reason_codes.clone()),
            join_codes(self.context_rot_risk_detail_codes.clone())
        )
    }

    pub fn kvswap_boundary_blocker_count(&self) -> usize {
        self.kvswap_boundary_overlap_count
            .saturating_add(self.kvswap_boundary_missing_hot_metadata_count)
    }

    pub fn kvswap_boundary_warning_count(&self) -> usize {
        self.kvswap_boundary_stale_metadata_count
            .saturating_add(self.kvswap_boundary_hot_tier_mismatch_count)
            .saturating_add(self.kvswap_boundary_cold_tier_mismatch_count)
    }

    pub fn kvswap_boundary_blocker_reason_codes(&self) -> Vec<String> {
        kvswap_boundary_blocker_reason_codes_from_counts(
            self.kvswap_boundary_overlap_count,
            self.kvswap_boundary_missing_hot_metadata_count,
        )
    }

    pub fn kvswap_boundary_warning_reason_codes(&self) -> Vec<String> {
        kvswap_boundary_warning_reason_codes_from_counts(
            self.kvswap_boundary_stale_metadata_count,
            self.kvswap_boundary_hot_tier_mismatch_count,
            self.kvswap_boundary_cold_tier_mismatch_count,
        )
    }

    pub fn kvswap_boundary_readiness_detail_codes(&self) -> Vec<String> {
        kvswap_boundary_readiness_detail_codes_from_boundary(&self.kvswap_boundary_detail_codes)
    }

    pub fn kvswap_boundary_checklist_detail(&self) -> String {
        format!(
            "boundary_issues={} boundary_blockers={} boundary_warnings={} boundary_reason_codes={} boundary_blocker_reason_codes={} boundary_warning_reason_codes={} boundary_detail_codes={}",
            self.kvswap_boundary_issue_count,
            self.kvswap_boundary_blocker_count(),
            self.kvswap_boundary_warning_count(),
            join_codes(self.kvswap_boundary_reason_codes.clone()),
            join_codes(self.kvswap_boundary_blocker_reason_codes()),
            join_codes(self.kvswap_boundary_warning_reason_codes()),
            join_codes(self.kvswap_boundary_readiness_detail_codes())
        )
    }

    pub fn summary_line(&self) -> String {
        format!(
            "memory_shadow ready={} review={} experiences={} kv_shards={} memories={} runtime_kv_memories={} projection_issues={} projection_blockers={} adapter_snapshots={} adapter_snapshot_warnings={} projection_contracts={} projection_contract_manifests={} projection_contract_blockers={} projection_contract_warnings={} parity_mismatches={} readiness_missing={} readiness_write_blockers={} context_rejections={} context_rot_risks={} context_rot_risk_reason_codes={} context_rot_blocker_reason_codes={} context_rot_risk_detail_codes={} clean_gist_repair_missing_clean_gist={} clean_gist_repair_dirty_clean_gist={} clean_gist_repair_dirty_gist={} clean_gist_repair_detail_codes={} repair_items={} repair_skipped={} replay_planned={} replay_updates={} replay_context_rot={} kvswap_prefetch={} kvswap_evict={} kvswap_state={} kvswap_hot={} kvswap_cold={} kvswap_metadata={} kvswap_bytes={} kvswap_shape_codes={} kvswap_boundary={} kvswap_boundary_issues={} kvswap_boundary_reason_codes={} kvswap_boundary_detail_codes={} kvswap_boundary_overlap={} kvswap_boundary_missing_hot_metadata={} kvswap_boundary_stale_metadata={} kvswap_boundary_hot_tier_mismatch={} kvswap_boundary_cold_tier_mismatch={} retention_decays={} retention_removals={} compaction_merges={} compaction_removals={} evolution_blockers={} evolution_warnings={} hygiene_pressure_score={} hygiene_pressure_priority={} hygiene_pressure_action_lanes={} hygiene_pressure_action_lane_details={} hygiene_work_next_action={} hygiene_work_operator_review={} hygiene_work_isolation={} hygiene_work_queue_items={} hygiene_work_queue_operator_review={} hygiene_work_queue_isolation={} hygiene_work_queue_next_dispatch={} hygiene_work_queue_lanes={} hygiene_work_queue_priorities={} hygiene_work_queue_dispatch_codes={} hygiene_work_queue_detail_codes={} hygiene_work_queue_reason_codes={} hygiene_pressure_reason_codes={} hygiene_pressure_detail_codes={} inspection_risks={} reasons={} reason_codes={} detail_codes={}",
            self.ready,
            self.requires_operator_review,
            self.experience_count,
            self.kv_shard_count,
            self.memory_count,
            self.runtime_kv_memory_count,
            self.projection_issue_count,
            self.projection_blocker_count,
            self.adapter_snapshot_count,
            self.adapter_snapshot_warning_count,
            self.projection_contract_count,
            self.projection_contract_manifest_count,
            self.projection_contract_blocker_count,
            self.projection_contract_warning_count,
            self.projection_parity_mismatch_count,
            self.readiness_missing_capability_count,
            self.readiness_write_blocker_count,
            self.context_rejection_count,
            self.context_rot_risk_count,
            join_codes(self.context_rot_risk_reason_codes.clone()),
            join_codes(self.context_rot_blocker_reason_codes.clone()),
            join_codes(self.context_rot_risk_detail_codes.clone()),
            self.clean_gist_repair_missing_clean_gist_count,
            self.clean_gist_repair_dirty_clean_gist_count,
            self.clean_gist_repair_dirty_gist_count,
            join_codes(self.clean_gist_repair_detail_codes.clone()),
            self.repair_item_count,
            self.repair_skipped_count,
            self.replay_planned_count,
            self.replay_memory_update_count,
            self.replay_context_rot_count,
            self.kvswap_prefetch_count,
            self.kvswap_evict_count,
            self.kvswap_state_present,
            self.kvswap_hot_shard_count,
            self.kvswap_cold_shard_count,
            self.kvswap_metadata_count,
            self.kvswap_total_byte_len,
            join_codes(self.kvswap_shape_codes.clone()),
            self.kvswap_boundary_present,
            self.kvswap_boundary_issue_count,
            join_codes(self.kvswap_boundary_reason_codes.clone()),
            join_codes(self.kvswap_boundary_detail_codes.clone()),
            self.kvswap_boundary_overlap_count,
            self.kvswap_boundary_missing_hot_metadata_count,
            self.kvswap_boundary_stale_metadata_count,
            self.kvswap_boundary_hot_tier_mismatch_count,
            self.kvswap_boundary_cold_tier_mismatch_count,
            self.retention_decay_count,
            self.retention_removal_count,
            self.compaction_merge_count,
            self.compaction_removal_count,
            self.evolution_blocker_count,
            self.evolution_warning_count,
            self.hygiene_pressure_score,
            self.hygiene_pressure_priority,
            join_codes(self.hygiene_pressure_action_lanes.clone()),
            join_codes(self.hygiene_pressure_action_lane_details.clone()),
            self.hygiene_work_next_action,
            self.hygiene_work_operator_review_required,
            self.hygiene_work_isolation_recommended,
            self.hygiene_work_queue_item_count,
            self.hygiene_work_queue_operator_review_count,
            self.hygiene_work_queue_isolation_count,
            self.hygiene_work_queue_next_dispatch,
            join_codes(self.hygiene_work_queue_lane_codes.clone()),
            join_codes(self.hygiene_work_queue_priority_codes.clone()),
            join_codes(self.hygiene_work_queue_dispatch_codes.clone()),
            join_codes(self.hygiene_work_queue_detail_codes.clone()),
            join_codes(self.hygiene_work_queue_reason_codes.clone()),
            join_codes(self.hygiene_pressure_reason_codes.clone()),
            join_codes(self.hygiene_pressure_detail_codes.clone()),
            self.inspection_risk_count,
            if self.review_reasons.is_empty() {
                "none".to_owned()
            } else {
                self.review_reasons.join("|")
            },
            join_codes(self.reason_codes()),
            join_codes(self.detail_codes()),
        )
    }

    pub fn reason_codes(&self) -> Vec<String> {
        self.review_reasons.clone()
    }

    pub fn detail_codes(&self) -> Vec<String> {
        self.review_detail_codes.clone()
    }

    pub fn context_rot_risk_detail_codes_for_reason(&self, reason: &str) -> Vec<String> {
        let suffix = format!(":{reason}");
        self.context_rot_risk_detail_codes
            .iter()
            .filter(|code| code.ends_with(&suffix))
            .cloned()
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn context_rot_risk_reason_count(&self, reason: &str) -> usize {
        self.context_rot_risk_detail_codes_for_reason(reason).len()
    }
}

fn kvswap_boundary_blocker_reason_codes_from_counts(
    overlap_count: usize,
    missing_hot_metadata_count: usize,
) -> Vec<String> {
    let mut codes = Vec::new();
    if overlap_count > 0 {
        codes.push("overlapping_hot_cold".to_owned());
    }
    if missing_hot_metadata_count > 0 {
        codes.push("missing_hot_metadata".to_owned());
    }
    codes.sort();
    codes
}

fn kvswap_boundary_warning_reason_codes_from_counts(
    stale_metadata_count: usize,
    hot_tier_mismatch_count: usize,
    cold_tier_mismatch_count: usize,
) -> Vec<String> {
    let mut codes = Vec::new();
    if stale_metadata_count > 0 {
        codes.push("stale_metadata".to_owned());
    }
    if hot_tier_mismatch_count > 0 {
        codes.push("hot_tier_mismatch".to_owned());
    }
    if cold_tier_mismatch_count > 0 {
        codes.push("cold_tier_mismatch".to_owned());
    }
    codes.sort();
    codes
}

fn kvswap_boundary_readiness_detail_codes_from_boundary(detail_codes: &[String]) -> Vec<String> {
    detail_codes
        .iter()
        .filter_map(|code| {
            if code.starts_with("overlap:") || code.starts_with("missing_hot_metadata:") {
                Some(format!("blocker:{code}"))
            } else if code.starts_with("stale_metadata:")
                || code.starts_with("hot_tier_mismatch:")
                || code.starts_with("cold_tier_mismatch:")
            {
                Some(format!("warning:{code}"))
            } else {
                None
            }
        })
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn context_rot_risk_reason_codes(plan: &MemoryServiceShadowPlan) -> Vec<String> {
    plan.read_only
        .governance
        .context_rot_risks
        .iter()
        .flat_map(|risk| risk.reason_codes())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn context_rot_risk_detail_codes(plan: &MemoryServiceShadowPlan) -> Vec<String> {
    plan.read_only
        .governance
        .context_rot_risks
        .iter()
        .flat_map(|risk| risk.detail_codes())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn collect_shadow_review_detail_codes(plan: &MemoryServiceShadowPlan) -> Vec<String> {
    let mut codes = std::collections::BTreeSet::new();

    for capability in &plan.readiness.missing_capabilities {
        codes.insert(format!("readiness:missing:{}", capability.as_str()));
    }
    for capability in &plan.readiness.write_mode_blockers {
        codes.insert(format!(
            "readiness:write_mode_blocked:{}",
            capability.as_str()
        ));
    }
    insert_prefixed_detail_codes(&mut codes, "readiness", plan.readiness.warning_codes());

    for status in &plan.readiness.adapter_statuses {
        for code in status.status_codes() {
            if code != "read_only" {
                codes.insert(format!("adapter_status:{code}"));
            }
        }
        insert_prefixed_detail_codes(&mut codes, "adapter_warning", status.warning_codes());
    }
    for coverage in &plan.readiness.coverage {
        for code in coverage.status_codes() {
            if code != "write_mode_blocked" || !coverage.read_only_providers.is_empty() {
                codes.insert(format!(
                    "capability:{}:{code}",
                    coverage.capability.as_str()
                ));
            }
        }
    }

    insert_prefixed_detail_codes(
        &mut codes,
        "projection",
        plan.projection_audit.issue_codes(),
    );
    insert_prefixed_detail_codes(
        &mut codes,
        "projection_detail",
        plan.projection_audit.detail_codes(),
    );
    for snapshot in &plan.adapter_snapshots {
        for warning_code in snapshot.warning_codes() {
            codes.insert(format!(
                "adapter_snapshot:{}:{}",
                detail_code_from_text(&snapshot.adapter_name),
                warning_code
            ));
        }
    }
    for report in &plan.projection_coverage {
        insert_prefixed_detail_codes(
            &mut codes,
            "projection_contract_blocker",
            report.blocker_codes(),
        );
        insert_prefixed_detail_codes(
            &mut codes,
            "projection_contract_blocker_detail",
            report.blocker_detail_codes(),
        );
        insert_prefixed_detail_codes(
            &mut codes,
            "projection_contract_warning",
            report.warning_codes(),
        );
        insert_prefixed_detail_codes(
            &mut codes,
            "projection_contract_warning_detail",
            report.warning_detail_codes(),
        );
    }
    insert_prefixed_detail_codes(&mut codes, "read_only", plan.read_only.reason_codes());
    insert_prefixed_detail_codes(
        &mut codes,
        "read_only_detail",
        plan.read_only.detail_codes(),
    );
    if plan.request_scope_missing {
        codes.insert("request_scope:missing".to_owned());
    }
    insert_prefixed_detail_codes(&mut codes, "replay", plan.replay_report.detail_codes());
    if let Some(boundary) = &plan.kvswap_boundary {
        insert_prefixed_detail_codes(&mut codes, "kvswap_boundary", boundary.reason_codes());
        insert_prefixed_detail_codes(
            &mut codes,
            "kvswap_boundary_detail",
            boundary.detail_codes(),
        );
    }
    insert_prefixed_detail_codes(
        &mut codes,
        "migration_readiness_blocker",
        plan.migration_readiness.blocker_codes(),
    );
    insert_prefixed_detail_codes(
        &mut codes,
        "migration_readiness_blocker_detail",
        plan.migration_readiness.blocker_detail_codes(),
    );
    insert_prefixed_detail_codes(
        &mut codes,
        "migration_readiness_warning",
        plan.migration_readiness.warning_codes(),
    );
    insert_prefixed_detail_codes(
        &mut codes,
        "migration_readiness_warning_detail",
        plan.migration_readiness.warning_detail_codes(),
    );
    insert_prefixed_detail_codes(
        &mut codes,
        "evolution_blocker",
        plan.evolution_assessment.blocker_codes(),
    );
    insert_prefixed_detail_codes(
        &mut codes,
        "evolution_blocker_detail",
        plan.evolution_assessment.blocker_detail_codes(),
    );
    insert_prefixed_detail_codes(
        &mut codes,
        "evolution_warning",
        plan.evolution_assessment.warning_codes(),
    );
    insert_prefixed_detail_codes(
        &mut codes,
        "evolution_warning_detail",
        plan.evolution_assessment.warning_detail_codes(),
    );
    insert_prefixed_detail_codes(&mut codes, "inspection", plan.inspection.risk_codes());
    insert_prefixed_detail_codes(
        &mut codes,
        "inspection_detail",
        plan.inspection.risk_detail_codes(),
    );
    if !plan.retention.is_empty() {
        insert_prefixed_detail_codes(&mut codes, "retention", plan.retention.detail_codes());
    }
    if !plan.compaction.is_empty()
        || plan.compaction.skipped_reason.as_deref() == Some("policy_disabled")
    {
        insert_prefixed_detail_codes(&mut codes, "compaction", plan.compaction.detail_codes());
    }

    for mismatch in &plan.projection_parity_audit.mismatches {
        codes.insert(format!("projection_parity:mismatch:{}", mismatch.field));
    }
    for warning in &plan.projection_parity_audit.warnings {
        codes.insert(format!(
            "projection_parity:warning:{}",
            detail_code_from_text(warning)
        ));
    }

    codes.into_iter().collect()
}

fn insert_prefixed_detail_codes(
    codes: &mut std::collections::BTreeSet<String>,
    prefix: &str,
    source: Vec<String>,
) {
    for code in source {
        codes.insert(format!("{prefix}:{code}"));
    }
}

fn detail_code_from_text(detail: &str) -> String {
    detail
        .split_once('=')
        .or_else(|| detail.split_once(':'))
        .map_or(detail, |(code, _)| code)
        .to_owned()
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryServiceDryRun {
    pub plan: MemoryServiceShadowPlan,
    pub summary: MemoryServiceShadowSummary,
    pub migration_evidence: MemoryMigrationEvidence,
    pub approvals: Vec<MemoryMigrationApproval>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryServiceStartupEvidence {
    pub requires_operator_review: bool,
    pub approved_phases: Vec<MemoryMigrationPhase>,
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryStartupAdmissionEvidence {
    pub read_only_review_required: bool,
    pub index_quality_blocker_count: usize,
    pub index_quality_warning_count: usize,
    pub index_operation_count: usize,
    pub index_refresh_count: usize,
    pub index_detail_codes: Vec<String>,
    pub context_rot_risk_count: usize,
    pub context_rot_blocker_reason_codes: Vec<String>,
    pub admission_decision_count: usize,
    pub admission_accepted_count: usize,
    pub admission_risk_rejection_count: usize,
    pub migration_live_store_targeted_count: usize,
    pub adapter_live_write_count: usize,
    pub live_write_phase_request_count: usize,
    pub store_mutation_count: usize,
    pub helper_prose_line_count: usize,
    pub non_contract_line_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryHygieneDispatchPressureSummary {
    pub pressure_score: usize,
    pub queue_items: usize,
    pub operator_review_items: usize,
    pub isolation_items: usize,
    pub kvswap_boundary_repair_lanes: usize,
    pub context_rot_review_lanes: usize,
    pub experience_index_rebuild_lanes: usize,
    pub quarantine_priorities: usize,
    pub repair_priorities: usize,
    pub context_rot_risks: usize,
    pub missing_clean_gist_pressure: usize,
    pub kvswap_boundary_blockers: usize,
    pub kvswap_boundary_warnings: usize,
}

impl MemoryHygieneDispatchPressureSummary {
    pub fn has_pressure(&self) -> bool {
        self.pressure_score > 0
            || self.queue_items > 0
            || self.context_rot_risks > 0
            || self.missing_clean_gist_pressure > 0
            || self.kvswap_boundary_blockers > 0
            || self.kvswap_boundary_warnings > 0
    }

    pub fn requires_operator_review(&self) -> bool {
        self.operator_review_items > 0
            || self.quarantine_priorities > 0
            || self.context_rot_risks > 0
            || self.kvswap_boundary_blockers > 0
    }

    pub fn requires_isolation(&self) -> bool {
        self.isolation_items > 0
            || self.quarantine_priorities > 0
            || self.kvswap_boundary_blockers > 0
    }

    pub fn priority_code(&self) -> &'static str {
        if self.requires_isolation() {
            "quarantine"
        } else if self.requires_operator_review() {
            "review"
        } else if self.has_pressure() {
            "repair"
        } else {
            "clean"
        }
    }

    pub fn dispatch_rank(&self) -> u8 {
        match self.priority_code() {
            "quarantine" => 3,
            "review" => 2,
            "repair" => 1,
            _ => 0,
        }
    }

    pub fn reason_codes(&self) -> Vec<String> {
        let mut codes = Vec::new();
        if self.queue_items > 0 {
            codes.push("queue_items");
        }
        if self.operator_review_items > 0 {
            codes.push("operator_review_items");
        }
        if self.isolation_items > 0 {
            codes.push("isolation_items");
        }
        if self.kvswap_boundary_repair_lanes > 0 {
            codes.push("kvswap_boundary_repair");
        }
        if self.context_rot_review_lanes > 0 {
            codes.push("context_rot_review");
        }
        if self.experience_index_rebuild_lanes > 0 {
            codes.push("experience_index_rebuild");
        }
        if self.quarantine_priorities > 0 {
            codes.push("quarantine_priority");
        }
        if self.repair_priorities > 0 {
            codes.push("repair_priority");
        }
        if self.context_rot_risks > 0 {
            codes.push("context_rot_risk");
        }
        if self.missing_clean_gist_pressure > 0 {
            codes.push("missing_clean_gist");
        }
        if self.kvswap_boundary_blockers > 0 {
            codes.push("kvswap_boundary_blocker");
        }
        if self.kvswap_boundary_warnings > 0 {
            codes.push("kvswap_boundary_warning");
        }
        codes.into_iter().map(str::to_owned).collect()
    }

    pub fn summary_line(&self) -> String {
        format!(
            "memory_hygiene_dispatch_pressure rank={} priority={} pressure_score={} queue_items={} operator_review_items={} isolation_items={} context_rot_risks={} missing_clean_gist_pressure={} kvswap_boundary_blockers={} kvswap_boundary_warnings={} reason_codes={}",
            self.dispatch_rank(),
            self.priority_code(),
            self.pressure_score,
            self.queue_items,
            self.operator_review_items,
            self.isolation_items,
            self.context_rot_risks,
            self.missing_clean_gist_pressure,
            self.kvswap_boundary_blockers,
            self.kvswap_boundary_warnings,
            join_codes(self.reason_codes())
        )
    }
}

impl MemoryServiceStartupEvidence {
    pub fn required_line_prefixes() -> &'static [&'static str] {
        &[
            "memory_shadow ",
            "memory_service_requirement ",
            "memory_readiness ",
            "memory_adapter_status ",
            "memory_capability_coverage ",
            "memory_read_only_plan ",
            "adapter_projection_audit ",
            "memory_governance ",
            "memory_rebuild ",
            "experience_index_quality_gate ",
            "memory_repair_plan ",
            "memory_index_plan ",
            "memory_context_injection ",
            "context_rot_risk ",
            "memory_adapter_checklist ",
            "memory_adapter_checklist_item ",
            "memory_replay ",
            "memory_evolution ",
            "memory_hygiene_work_plan ",
            "memory_hygiene_work_queue ",
            "memory_hygiene_dispatch_pressure ",
            "infini_memory ",
            "memory_retention ",
            "memory_compaction ",
            "memory_inspection ",
            "memory_projection_parity ",
            "memory_migration_readiness ",
            "memory_migration_evidence ",
            "kvswap_intent ",
            "kvswap_prefetch ",
            "kvswap_eviction ",
        ]
    }

    pub fn summary_text(&self) -> String {
        self.lines.join("\n")
    }

    pub fn has_line_prefix(&self, prefix: &str) -> bool {
        self.lines.iter().any(|line| line.starts_with(prefix))
    }

    pub fn missing_required_line_prefixes(&self) -> Vec<&'static str> {
        Self::required_line_prefixes()
            .iter()
            .copied()
            .filter(|prefix| !self.has_line_prefix(prefix))
            .collect()
    }

    pub fn missing_required_codes(&self) -> Vec<String> {
        let mut codes = self
            .missing_required_line_prefixes()
            .into_iter()
            .map(|prefix| prefix.trim().replace(' ', "_"))
            .collect::<Vec<_>>();
        if self.projection_contract_manifest_gap() > 0 {
            codes.push("adapter_projection_contract".to_owned());
        }
        codes.sort();
        codes.dedup();
        codes
    }

    pub fn line_count_with_prefix(&self, prefix: &str) -> usize {
        self.lines
            .iter()
            .filter(|line| line.starts_with(prefix))
            .count()
    }

    pub fn projection_contract_count(&self) -> usize {
        self.line_count_with_prefix("adapter_projection adapter=")
    }

    pub fn projection_contract_manifest_count(&self) -> usize {
        self.line_count_with_prefix("adapter_projection_contract adapter=")
    }

    pub fn projection_contract_manifest_gap(&self) -> usize {
        self.projection_contract_count()
            .saturating_sub(self.projection_contract_manifest_count())
    }

    pub fn projection_contract_manifests_complete(&self) -> bool {
        self.projection_contract_manifest_gap() == 0
    }

    pub fn projection_contract_ready_count(&self) -> usize {
        startup_bool_count_from_line_prefix(
            &self.lines,
            "adapter_projection adapter=",
            "ready=",
            true,
        )
    }

    pub fn projection_contract_missing_required_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(
            &self.lines,
            "adapter_projection adapter=",
            "missing_required=",
        )
    }

    pub fn projection_contract_missing_recommended_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(
            &self.lines,
            "adapter_projection adapter=",
            "missing_recommended=",
        )
    }

    pub fn projection_contract_blocker_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "adapter_projection adapter=", "blockers=")
    }

    pub fn projection_contract_warning_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "adapter_projection adapter=", "warnings=")
    }

    pub fn projection_contract_blocker_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "adapter_projection adapter=", "blocker_codes=")
    }

    pub fn projection_contract_warning_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "adapter_projection adapter=", "warning_codes=")
    }

    pub fn projection_contract_blocker_detail_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(
            &self.lines,
            "adapter_projection adapter=",
            "blocker_detail_codes=",
        )
    }

    pub fn projection_contract_warning_detail_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(
            &self.lines,
            "adapter_projection adapter=",
            "warning_detail_codes=",
        )
    }

    pub fn projection_contract_manifest_mapped_field_count(&self) -> usize {
        startup_sum_code_count_from_line_prefix(
            &self.lines,
            "adapter_projection_contract adapter=",
            "mapped_fields=",
        )
    }

    pub fn projection_contract_manifest_required_field_count(&self) -> usize {
        startup_sum_code_count_from_line_prefix(
            &self.lines,
            "adapter_projection_contract adapter=",
            "required_fields=",
        )
    }

    pub fn projection_contract_manifest_recommended_field_count(&self) -> usize {
        startup_sum_code_count_from_line_prefix(
            &self.lines,
            "adapter_projection_contract adapter=",
            "recommended_fields=",
        )
    }

    pub fn projection_contract_manifest_note_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(
            &self.lines,
            "adapter_projection_contract adapter=",
            "notes=",
        )
    }

    pub fn projection_bundle_report_count(&self) -> usize {
        self.line_count_with_prefix("adapter_projection_bundle ")
    }

    pub fn projection_bundle_ready_count(&self) -> usize {
        startup_bool_count_from_line_prefix(
            &self.lines,
            "adapter_projection_bundle ",
            "ready=",
            true,
        )
    }

    pub fn projection_bundle_review_count(&self) -> usize {
        startup_bool_count_from_line_prefix(
            &self.lines,
            "adapter_projection_bundle ",
            "review=",
            true,
        )
    }

    pub fn projection_bundle_contract_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "adapter_projection_bundle ", "contracts=")
    }

    pub fn projection_bundle_ready_contract_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(
            &self.lines,
            "adapter_projection_bundle ",
            "ready_contracts=",
        )
    }

    pub fn projection_bundle_blocker_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "adapter_projection_bundle ", "blockers=")
    }

    pub fn projection_bundle_warning_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "adapter_projection_bundle ", "warnings=")
    }

    pub fn projection_bundle_blocker_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "adapter_projection_bundle ", "blocker_codes=")
    }

    pub fn projection_bundle_warning_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "adapter_projection_bundle ", "warning_codes=")
    }

    pub fn projection_bundle_blocker_detail_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(
            &self.lines,
            "adapter_projection_bundle ",
            "blocker_detail_codes=",
        )
    }

    pub fn projection_bundle_warning_detail_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(
            &self.lines,
            "adapter_projection_bundle ",
            "warning_detail_codes=",
        )
    }

    pub fn projection_bundle_manifest_count(&self) -> usize {
        self.line_count_with_prefix("adapter_projection_contract_bundle_manifest ")
    }

    pub fn projection_bundle_manifest_contract_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(
            &self.lines,
            "adapter_projection_contract_bundle_manifest ",
            "contracts=",
        )
    }

    pub fn projection_bundle_manifest_mapped_field_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(
            &self.lines,
            "adapter_projection_contract_bundle_manifest ",
            "mapped_fields=",
        )
    }

    pub fn projection_bundle_manifest_required_field_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(
            &self.lines,
            "adapter_projection_contract_bundle_manifest ",
            "required_fields=",
        )
    }

    pub fn projection_bundle_manifest_recommended_field_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(
            &self.lines,
            "adapter_projection_contract_bundle_manifest ",
            "recommended_fields=",
        )
    }

    pub fn projection_bundle_manifest_note_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(
            &self.lines,
            "adapter_projection_contract_bundle_manifest ",
            "notes=",
        )
    }

    pub fn projection_audit_issue_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "adapter_projection_audit ", "issues=")
    }

    pub fn projection_audit_blocker_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "adapter_projection_audit ", "blockers=")
    }

    pub fn projection_audit_warning_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "adapter_projection_audit ", "warnings=")
    }

    pub fn projection_audit_issue_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "adapter_projection_audit ", "issue_codes=")
    }

    pub fn projection_audit_detail_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "adapter_projection_audit ", "detail_codes=")
    }

    pub fn adapter_status_count(&self) -> usize {
        self.line_count_with_prefix("memory_adapter_status ")
    }

    pub fn adapter_status_ready_count(&self) -> usize {
        startup_bool_count_from_line_prefix(&self.lines, "memory_adapter_status ", "ready=", true)
    }

    pub fn adapter_status_unhealthy_count(&self) -> usize {
        startup_bool_count_from_line_prefix(&self.lines, "memory_adapter_status ", "ready=", false)
    }

    pub fn adapter_status_read_only_count(&self) -> usize {
        startup_bool_count_from_line_prefix(
            &self.lines,
            "memory_adapter_status ",
            "read_only=",
            true,
        )
    }

    pub fn adapter_status_live_write_count(&self) -> usize {
        startup_field_value_count_from_line_prefix(
            &self.lines,
            "memory_adapter_status ",
            "write_mode=",
            "live_write",
        )
    }

    pub fn adapter_status_isolated_write_count(&self) -> usize {
        startup_field_value_count_from_line_prefix(
            &self.lines,
            "memory_adapter_status ",
            "write_mode=",
            "isolated_write",
        )
    }

    pub fn adapter_status_capability_count(&self) -> usize {
        startup_sum_code_count_from_line_prefix(
            &self.lines,
            "memory_adapter_status ",
            "capabilities=",
        )
    }

    pub fn adapter_status_record_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_adapter_status ", "records=")
    }

    pub fn adapter_status_warning_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_adapter_status ", "warnings=")
    }

    pub fn adapter_status_status_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "memory_adapter_status ", "status_codes=")
    }

    pub fn adapter_status_warning_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "memory_adapter_status ", "warning_codes=")
    }

    pub fn adapter_status_status_code_count(&self, code: &str) -> usize {
        startup_code_occurrence_count_from_line_prefix(
            &self.lines,
            "memory_adapter_status ",
            "status_codes=",
            code,
        )
    }

    pub fn capability_coverage_count(&self) -> usize {
        self.line_count_with_prefix("memory_capability_coverage ")
    }

    pub fn capability_coverage_provider_count(&self) -> usize {
        startup_sum_name_count_from_line_prefix(
            &self.lines,
            "memory_capability_coverage ",
            "providers=",
        )
    }

    pub fn capability_coverage_healthy_provider_count(&self) -> usize {
        startup_sum_name_count_from_line_prefix(
            &self.lines,
            "memory_capability_coverage ",
            "healthy=",
        )
    }

    pub fn capability_coverage_writable_provider_count(&self) -> usize {
        startup_sum_name_count_from_line_prefix(
            &self.lines,
            "memory_capability_coverage ",
            "writable=",
        )
    }

    pub fn capability_coverage_read_only_provider_count(&self) -> usize {
        startup_sum_name_count_from_line_prefix(
            &self.lines,
            "memory_capability_coverage ",
            "read_only=",
        )
    }

    pub fn capability_coverage_record_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_capability_coverage ", "records=")
    }

    pub fn capability_coverage_status_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "memory_capability_coverage ", "status_codes=")
    }

    pub fn capability_coverage_missing_provider_count(&self) -> usize {
        self.capability_coverage_status_code_count("missing_provider")
    }

    pub fn capability_coverage_no_healthy_provider_count(&self) -> usize {
        self.capability_coverage_status_code_count("no_healthy_provider")
    }

    pub fn capability_coverage_write_mode_blocked_count(&self) -> usize {
        self.capability_coverage_status_code_count("write_mode_blocked")
    }

    pub fn capability_coverage_multiple_provider_count(&self) -> usize {
        self.capability_coverage_status_code_count("multiple_providers")
    }

    pub fn capability_coverage_status_code_count(&self, code: &str) -> usize {
        startup_code_occurrence_count_from_line_prefix(
            &self.lines,
            "memory_capability_coverage ",
            "status_codes=",
            code,
        )
    }

    pub fn adapter_checklist_report_count(&self) -> usize {
        self.line_count_with_prefix("memory_adapter_checklist ")
    }

    pub fn adapter_checklist_summary_item_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_adapter_checklist ", "items=")
    }

    pub fn adapter_checklist_blocker_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_adapter_checklist ", "blockers=")
    }

    pub fn adapter_checklist_warning_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_adapter_checklist ", "warnings=")
    }

    pub fn adapter_checklist_satisfied_report_count(&self) -> usize {
        self.lines
            .iter()
            .filter(|line| line.starts_with("memory_adapter_checklist "))
            .filter(|line| {
                startup_line_field_value(line, "satisfied=")
                    .and_then(|value| value.parse::<bool>().ok())
                    .unwrap_or_default()
            })
            .count()
    }

    pub fn adapter_checklist_blocker_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "memory_adapter_checklist ", "blocker_codes=")
    }

    pub fn adapter_checklist_warning_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "memory_adapter_checklist ", "warning_codes=")
    }

    pub fn adapter_checklist_blocker_detail_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(
            &self.lines,
            "memory_adapter_checklist ",
            "blocker_detail_codes=",
        )
    }

    pub fn adapter_checklist_warning_detail_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(
            &self.lines,
            "memory_adapter_checklist ",
            "warning_detail_codes=",
        )
    }

    pub fn adapter_checklist_item_line_count(&self) -> usize {
        self.line_count_with_prefix("memory_adapter_checklist_item ")
    }

    pub fn adapter_checklist_failed_item_count(&self) -> usize {
        self.adapter_checklist_item_codes_matching(None, Some(false))
            .len()
    }

    pub fn adapter_checklist_failed_blocker_count(&self) -> usize {
        self.adapter_checklist_item_codes_matching(Some("blocker"), Some(false))
            .len()
    }

    pub fn adapter_checklist_failed_warning_count(&self) -> usize {
        self.adapter_checklist_item_codes_matching(Some("warning"), Some(false))
            .len()
    }

    pub fn adapter_checklist_failed_info_count(&self) -> usize {
        self.adapter_checklist_item_codes_matching(Some("info"), Some(false))
            .len()
    }

    pub fn adapter_checklist_failed_item_codes(&self) -> Vec<String> {
        self.adapter_checklist_item_codes_matching(None, Some(false))
    }

    pub fn adapter_checklist_failed_item_detail_codes(&self) -> Vec<String> {
        self.adapter_checklist_item_detail_codes_matching(None, Some(false))
    }

    pub fn adapter_checklist_item_detail_codes_for(&self, code: &str) -> Vec<String> {
        self.adapter_checklist_item_detail_codes_for_matching(code, None)
    }

    pub fn adapter_checklist_failed_item_detail_codes_for(&self, code: &str) -> Vec<String> {
        self.adapter_checklist_item_detail_codes_for_matching(code, Some(false))
    }

    pub fn migration_readiness_report_count(&self) -> usize {
        self.line_count_with_prefix("memory_migration_readiness ")
    }

    pub fn migration_readiness_isolated_write_ready_count(&self) -> usize {
        startup_bool_count_from_line_prefix(
            &self.lines,
            "memory_migration_readiness ",
            "isolated_write_ready=",
            true,
        )
    }

    pub fn migration_readiness_review_count(&self) -> usize {
        startup_bool_count_from_line_prefix(
            &self.lines,
            "memory_migration_readiness ",
            "review=",
            true,
        )
    }

    pub fn migration_readiness_blocker_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_migration_readiness ", "blockers=")
    }

    pub fn migration_readiness_warning_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_migration_readiness ", "warnings=")
    }

    pub fn migration_readiness_blocker_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "memory_migration_readiness ", "blocker_codes=")
    }

    pub fn migration_readiness_warning_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "memory_migration_readiness ", "warning_codes=")
    }

    pub fn migration_readiness_blocker_detail_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(
            &self.lines,
            "memory_migration_readiness ",
            "blocker_detail_codes=",
        )
    }

    pub fn migration_readiness_warning_detail_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(
            &self.lines,
            "memory_migration_readiness ",
            "warning_detail_codes=",
        )
    }

    pub fn migration_approval_count(&self) -> usize {
        self.line_count_with_prefix("memory_migration phase=")
    }

    pub fn migration_approval_approved_count(&self) -> usize {
        startup_bool_count_from_line_prefix(
            &self.lines,
            "memory_migration phase=",
            "approved=",
            true,
        )
    }

    pub fn migration_approval_blocked_count(&self) -> usize {
        startup_bool_count_from_line_prefix(
            &self.lines,
            "memory_migration phase=",
            "approved=",
            false,
        )
    }

    pub fn migration_approval_read_only_required_count(&self) -> usize {
        startup_field_value_count_from_line_prefix(
            &self.lines,
            "memory_migration phase=",
            "required_write_mode=",
            "read_only",
        )
    }

    pub fn migration_approval_isolated_write_required_count(&self) -> usize {
        startup_field_value_count_from_line_prefix(
            &self.lines,
            "memory_migration phase=",
            "required_write_mode=",
            "isolated_write",
        )
    }

    pub fn migration_approval_live_write_required_count(&self) -> usize {
        startup_field_value_count_from_line_prefix(
            &self.lines,
            "memory_migration phase=",
            "required_write_mode=",
            "live_write",
        )
    }

    pub fn migration_approval_blocker_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_migration phase=", "blockers=")
    }

    pub fn migration_approval_warning_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_migration phase=", "warnings=")
    }

    pub fn migration_approval_blocker_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "memory_migration phase=", "blocker_codes=")
    }

    pub fn migration_approval_warning_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "memory_migration phase=", "warning_codes=")
    }

    pub fn migration_approval_blocker_detail_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(
            &self.lines,
            "memory_migration phase=",
            "blocker_detail_codes=",
        )
    }

    pub fn migration_approval_warning_detail_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(
            &self.lines,
            "memory_migration phase=",
            "warning_detail_codes=",
        )
    }

    fn adapter_checklist_item_codes_matching(
        &self,
        severity: Option<&str>,
        satisfied: Option<bool>,
    ) -> Vec<String> {
        self.lines
            .iter()
            .filter(|line| line.starts_with("memory_adapter_checklist_item "))
            .filter(|line| {
                severity.is_none_or(|expected| {
                    startup_line_field_value(line, "severity=") == Some(expected)
                })
            })
            .filter(|line| {
                satisfied.is_none_or(|expected| {
                    startup_line_field_value(line, "satisfied=")
                        .and_then(|value| value.parse::<bool>().ok())
                        == Some(expected)
                })
            })
            .filter_map(|line| startup_line_field_value(line, "code="))
            .map(str::to_owned)
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    fn adapter_checklist_item_detail_codes_matching(
        &self,
        severity: Option<&str>,
        satisfied: Option<bool>,
    ) -> Vec<String> {
        self.lines
            .iter()
            .filter(|line| line.starts_with("memory_adapter_checklist_item "))
            .filter(|line| {
                severity.is_none_or(|expected| {
                    startup_line_field_value(line, "severity=") == Some(expected)
                })
            })
            .filter(|line| {
                satisfied.is_none_or(|expected| {
                    startup_line_field_value(line, "satisfied=")
                        .and_then(|value| value.parse::<bool>().ok())
                        == Some(expected)
                })
            })
            .flat_map(|line| {
                let code = startup_line_field_value(line, "code=").unwrap_or("unknown");
                startup_line_field_value(line, "detail_codes=")
                    .into_iter()
                    .flat_map(split_startup_codes)
                    .map(move |detail| format!("{code}:{detail}"))
            })
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    fn adapter_checklist_item_detail_codes_for_matching(
        &self,
        code: &str,
        satisfied: Option<bool>,
    ) -> Vec<String> {
        self.lines
            .iter()
            .filter(|line| line.starts_with("memory_adapter_checklist_item "))
            .filter(|line| startup_line_field_value(line, "code=") == Some(code))
            .filter(|line| {
                satisfied.is_none_or(|expected| {
                    startup_line_field_value(line, "satisfied=")
                        .and_then(|value| value.parse::<bool>().ok())
                        == Some(expected)
                })
            })
            .flat_map(|line| {
                startup_line_field_value(line, "detail_codes=")
                    .into_iter()
                    .flat_map(split_startup_codes)
            })
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn context_rot_risk_count(&self) -> usize {
        startup_sum_usize_from_primary_or_shadow(
            &self.lines,
            "context_rot_risk ",
            "risks=",
            "memory_shadow ",
            "context_rot_risks=",
        )
    }

    pub fn context_rot_risk_report_count(&self) -> usize {
        self.line_count_with_prefix("context_rot_risk ")
    }

    pub fn context_rot_risk_reason_codes(&self) -> Vec<String> {
        startup_codes_from_primary_or_shadow(
            &self.lines,
            "context_rot_risk ",
            "reason_codes=",
            "memory_shadow ",
            "context_rot_risk_reason_codes=",
        )
    }

    pub fn context_rot_blocker_reason_codes(&self) -> Vec<String> {
        startup_codes_from_primary_or_shadow_preserve_order(
            &self.lines,
            "experience_index_quality_gate ",
            "context_rot_blocker_reason_codes=",
            "memory_shadow ",
            "context_rot_blocker_reason_codes=",
        )
    }

    pub fn context_rot_risk_detail_codes(&self) -> Vec<String> {
        startup_codes_from_primary_or_shadow(
            &self.lines,
            "context_rot_risk ",
            "detail_codes=",
            "memory_shadow ",
            "context_rot_risk_detail_codes=",
        )
    }

    pub fn context_rot_risk_detail_codes_for_reason(&self, reason: &str) -> Vec<String> {
        let suffix = format!(":{reason}");
        self.context_rot_risk_detail_codes()
            .into_iter()
            .filter(|code| code.ends_with(&suffix))
            .collect()
    }

    pub fn context_rot_risk_reason_count(&self, reason: &str) -> usize {
        self.context_rot_risk_detail_codes_for_reason(reason).len()
    }

    pub fn clean_gist_selection_report_count(&self) -> usize {
        self.line_count_with_prefix("clean_gist_selection ")
    }

    pub fn clean_gist_selection_candidate_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "clean_gist_selection ", "candidates=")
    }

    pub fn clean_gist_selection_selected_count(&self) -> usize {
        self.lines
            .iter()
            .filter(|line| line.starts_with("clean_gist_selection "))
            .filter(|line| {
                startup_line_field_value(line, "selected=")
                    .and_then(|value| value.parse::<bool>().ok())
                    .unwrap_or_default()
            })
            .count()
    }

    pub fn clean_gist_selection_no_selection_count(&self) -> usize {
        startup_code_occurrence_count_from_line_prefix(
            &self.lines,
            "clean_gist_selection ",
            "reason_codes=",
            "no_selection",
        )
    }

    pub fn clean_gist_selection_selected_level_codes(&self) -> Vec<String> {
        self.lines
            .iter()
            .filter(|line| line.starts_with("clean_gist_selection "))
            .filter_map(|line| startup_line_field_value(line, "selected_level="))
            .map(str::to_owned)
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn clean_gist_selection_selected_level_count(&self, level: &str) -> usize {
        startup_field_value_count_from_line_prefix(
            &self.lines,
            "clean_gist_selection ",
            "selected_level=",
            level,
        )
    }

    pub fn clean_gist_selection_rejected_empty_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "clean_gist_selection ", "rejected_empty=")
    }

    pub fn clean_gist_selection_rejected_transcript_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(
            &self.lines,
            "clean_gist_selection ",
            "rejected_transcript=",
        )
    }

    pub fn clean_gist_selection_rejected_metadata_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(
            &self.lines,
            "clean_gist_selection ",
            "rejected_metadata=",
        )
    }

    pub fn clean_gist_selection_rejected_low_signal_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(
            &self.lines,
            "clean_gist_selection ",
            "rejected_low_signal=",
        )
    }

    pub fn clean_gist_selection_reason_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "clean_gist_selection ", "reason_codes=")
    }

    pub fn clean_gist_selection_reason_count(&self, reason: &str) -> usize {
        startup_code_occurrence_count_from_line_prefix(
            &self.lines,
            "clean_gist_selection ",
            "reason_codes=",
            reason,
        )
    }

    pub fn clean_gist_selection_detail_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "clean_gist_selection ", "detail_codes=")
    }

    pub fn clean_gist_selection_detail_codes_for(&self, prefix: &str) -> Vec<String> {
        let prefix = format!("{prefix}:");
        self.clean_gist_selection_detail_codes()
            .into_iter()
            .filter(|code| code.starts_with(&prefix))
            .collect()
    }

    pub fn governance_record_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_governance ", "records=")
    }

    pub fn governance_duplicate_group_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_governance ", "duplicate_groups=")
    }

    pub fn governance_duplicate_record_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_governance ", "duplicate_records=")
    }

    pub fn governance_noisy_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_governance ", "noisy=")
    }

    pub fn governance_context_rot_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_governance ", "context_rot=")
    }

    pub fn governance_reason_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "memory_governance ", "reason_codes=")
    }

    pub fn governance_reason_count(&self, reason: &str) -> usize {
        startup_code_occurrence_count_from_line_prefix(
            &self.lines,
            "memory_governance ",
            "reason_codes=",
            reason,
        )
    }

    pub fn governance_detail_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "memory_governance ", "detail_codes=")
    }

    pub fn memory_rebuild_required(&self) -> bool {
        self.lines
            .iter()
            .filter(|line| line.starts_with("memory_rebuild "))
            .any(|line| {
                startup_line_field_value(line, "required=")
                    .and_then(|value| value.parse::<bool>().ok())
                    .unwrap_or_default()
            })
    }

    pub fn memory_rebuild_duplicate_group_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_rebuild ", "duplicate_groups=")
    }

    pub fn memory_rebuild_refresh_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_rebuild ", "refresh=")
    }

    pub fn memory_rebuild_compact_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_rebuild ", "compact=")
    }

    pub fn memory_rebuild_quarantine_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_rebuild ", "quarantine=")
    }

    pub fn memory_rebuild_missing_clean_gist_count(&self) -> usize {
        self.clean_gist_repair_missing_clean_gist_count()
    }

    pub fn memory_rebuild_dirty_clean_gist_count(&self) -> usize {
        self.clean_gist_repair_dirty_clean_gist_count()
    }

    pub fn memory_rebuild_dirty_gist_count(&self) -> usize {
        self.clean_gist_repair_dirty_gist_count()
    }

    pub fn memory_rebuild_reason_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "memory_rebuild ", "reason_codes=")
    }

    pub fn memory_rebuild_reason_count(&self, reason: &str) -> usize {
        startup_code_occurrence_count_from_line_prefix(
            &self.lines,
            "memory_rebuild ",
            "reason_codes=",
            reason,
        )
    }

    pub fn memory_rebuild_detail_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "memory_rebuild ", "detail_codes=")
    }

    pub fn clean_gist_repair_missing_clean_gist_count(&self) -> usize {
        startup_sum_usize_from_projection_chain(
            &self.lines,
            "clean_gist_repair ",
            "missing_clean_gist=",
            "memory_shadow ",
            "clean_gist_repair_missing_clean_gist=",
            "memory_rebuild ",
            "missing_clean_gist=",
        )
    }

    pub fn clean_gist_repair_dirty_clean_gist_count(&self) -> usize {
        startup_sum_usize_from_projection_chain(
            &self.lines,
            "clean_gist_repair ",
            "dirty_clean_gist=",
            "memory_shadow ",
            "clean_gist_repair_dirty_clean_gist=",
            "memory_rebuild ",
            "dirty_clean_gist=",
        )
    }

    pub fn clean_gist_repair_dirty_gist_count(&self) -> usize {
        startup_sum_usize_from_projection_chain(
            &self.lines,
            "clean_gist_repair ",
            "dirty_gist=",
            "memory_shadow ",
            "clean_gist_repair_dirty_gist=",
            "memory_rebuild ",
            "dirty_gist=",
        )
    }

    pub fn clean_gist_repair_detail_codes(&self) -> Vec<String> {
        if startup_has_field_from_line_prefix(&self.lines, "clean_gist_repair ", "detail_codes=") {
            return startup_codes_from_line_prefix(
                &self.lines,
                "clean_gist_repair ",
                "detail_codes=",
            );
        }
        if startup_has_field_from_line_prefix(
            &self.lines,
            "memory_shadow ",
            "clean_gist_repair_detail_codes=",
        ) {
            return startup_codes_from_line_prefix(
                &self.lines,
                "memory_shadow ",
                "clean_gist_repair_detail_codes=",
            );
        }
        self.memory_rebuild_detail_codes()
            .into_iter()
            .filter(|code| is_clean_gist_repair_detail_code(code))
            .collect()
    }

    pub fn clean_gist_repair_issue_count(&self) -> usize {
        self.clean_gist_repair_missing_clean_gist_count()
            .saturating_add(self.clean_gist_repair_dirty_clean_gist_count())
            .saturating_add(self.clean_gist_repair_dirty_gist_count())
    }

    pub fn experience_index_quality_gate_ready(&self) -> bool {
        self.lines
            .iter()
            .filter(|line| line.starts_with("experience_index_quality_gate "))
            .all(|line| {
                startup_line_field_value(line, "ready_for_context_injection=")
                    .and_then(|value| value.parse::<bool>().ok())
                    .unwrap_or_default()
            })
    }

    pub fn experience_index_quality_gate_blocker_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(
            &self.lines,
            "experience_index_quality_gate ",
            "blockers=",
        )
    }

    pub fn experience_index_quality_gate_warning_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(
            &self.lines,
            "experience_index_quality_gate ",
            "warnings=",
        )
    }

    pub fn experience_index_quality_gate_context_rot_blocker_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(
            &self.lines,
            "experience_index_quality_gate ",
            "context_rot_blockers=",
        )
    }

    pub fn experience_index_quality_gate_missing_clean_gist_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(
            &self.lines,
            "experience_index_quality_gate ",
            "missing_clean_gist=",
        )
    }

    pub fn experience_index_quality_gate_dirty_gist_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(
            &self.lines,
            "experience_index_quality_gate ",
            "dirty_gist=",
        )
    }

    pub fn experience_index_quality_gate_reason_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(
            &self.lines,
            "experience_index_quality_gate ",
            "reason_codes=",
        )
    }

    pub fn experience_index_quality_gate_context_rot_blocker_reason_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix_preserve_order(
            &self.lines,
            "experience_index_quality_gate ",
            "context_rot_blocker_reason_codes=",
        )
    }

    pub fn experience_index_quality_gate_detail_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(
            &self.lines,
            "experience_index_quality_gate ",
            "detail_codes=",
        )
    }

    pub fn experience_index_quality_gate_detail_codes_for_reason(
        &self,
        reason: &str,
    ) -> Vec<String> {
        let needle = format!(":{reason}:");
        self.experience_index_quality_gate_detail_codes()
            .into_iter()
            .filter(|code| code.contains(&needle))
            .collect()
    }

    pub fn memory_repair_item_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_repair_plan ", "items=")
    }

    pub fn memory_repair_skipped_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_repair_plan ", "skipped=")
    }

    pub fn memory_repair_clean_gist_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_repair_plan ", "repair_clean_gist=")
    }

    pub fn memory_repair_compact_context_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_repair_plan ", "compact_context=")
    }

    pub fn memory_repair_quarantine_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_repair_plan ", "quarantine=")
    }

    pub fn memory_repair_delete_duplicate_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_repair_plan ", "delete_duplicate=")
    }

    pub fn memory_repair_skipped_clean_gist_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(
            &self.lines,
            "memory_repair_plan ",
            "skipped_repair_clean_gist=",
        )
    }

    pub fn memory_repair_skipped_compact_context_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(
            &self.lines,
            "memory_repair_plan ",
            "skipped_compact_context=",
        )
    }

    pub fn memory_repair_skipped_quarantine_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(
            &self.lines,
            "memory_repair_plan ",
            "skipped_quarantine=",
        )
    }

    pub fn memory_repair_skipped_delete_duplicate_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(
            &self.lines,
            "memory_repair_plan ",
            "skipped_delete_duplicate=",
        )
    }

    pub fn memory_repair_reason_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "memory_repair_plan ", "reason_codes=")
    }

    pub fn memory_repair_reason_count(&self, reason: &str) -> usize {
        startup_code_occurrence_count_from_line_prefix(
            &self.lines,
            "memory_repair_plan ",
            "reason_codes=",
            reason,
        )
    }

    pub fn memory_repair_skipped_reason_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "memory_repair_plan ", "skipped_reason_codes=")
    }

    pub fn memory_repair_skipped_reason_count(&self, reason: &str) -> usize {
        startup_code_occurrence_count_from_line_prefix(
            &self.lines,
            "memory_repair_plan ",
            "skipped_reason_codes=",
            reason,
        )
    }

    pub fn memory_repair_detail_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "memory_repair_plan ", "detail_codes=")
    }

    pub fn memory_repair_detail_codes_for_action(&self, action: &str) -> Vec<String> {
        let prefix = format!("{action}:");
        self.memory_repair_detail_codes()
            .into_iter()
            .filter(|code| code.starts_with(&prefix))
            .collect()
    }

    pub fn memory_repair_detail_codes_for_reason(&self, reason: &str) -> Vec<String> {
        let needle = format!(":{reason}:");
        self.memory_repair_detail_codes()
            .into_iter()
            .filter(|code| code.contains(&needle))
            .collect()
    }

    pub fn memory_repair_skipped_detail_codes(&self) -> Vec<String> {
        self.memory_repair_detail_codes()
            .into_iter()
            .filter(|code| code.starts_with("skipped:"))
            .collect()
    }

    pub fn memory_repair_skipped_detail_codes_for_action(&self, action: &str) -> Vec<String> {
        let prefix = format!("skipped:{action}:");
        self.memory_repair_detail_codes()
            .into_iter()
            .filter(|code| code.starts_with(&prefix))
            .collect()
    }

    pub fn memory_repair_skipped_detail_codes_for_reason(&self, reason: &str) -> Vec<String> {
        let needle = format!(":{reason}:");
        self.memory_repair_skipped_detail_codes()
            .into_iter()
            .filter(|code| code.contains(&needle))
            .collect()
    }

    pub fn migration_evidence_guard_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "memory_migration_evidence ", "guard_codes=")
    }

    pub fn migration_evidence_detail_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "memory_migration_evidence ", "detail_codes=")
    }

    pub fn migration_evidence_live_store_targeted_count(&self) -> usize {
        startup_bool_count_from_line_prefix(
            &self.lines,
            "memory_migration_evidence ",
            "live_store_targeted=",
            true,
        )
    }

    pub fn disk_kv_catalog_missing_bytes_count(&self) -> usize {
        self.disk_kv_catalog_detail_codes_for("missing_bytes").len()
    }

    pub fn disk_kv_catalog_byte_len_mismatch_count(&self) -> usize {
        self.disk_kv_catalog_detail_codes_for("byte_len_mismatch")
            .len()
    }

    pub fn disk_kv_catalog_checksum_mismatch_count(&self) -> usize {
        self.disk_kv_catalog_detail_codes_for("checksum_mismatch")
            .len()
    }

    pub fn disk_kv_catalog_detail_codes_for(&self, issue: &str) -> Vec<String> {
        let prefix = format!("disk_kv_catalog:{issue}:");
        self.migration_evidence_detail_codes()
            .into_iter()
            .filter(|code| code.starts_with(&prefix))
            .collect()
    }

    pub fn memory_index_reason_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "memory_index_plan ", "reason_codes=")
    }

    pub fn memory_index_operation_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_index_plan ", "operations=")
    }

    pub fn memory_index_upsert_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_index_plan ", "upsert=")
    }

    pub fn memory_index_refresh_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_index_plan ", "refresh=")
    }

    pub fn memory_index_compact_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_index_plan ", "compact=")
    }

    pub fn memory_index_quarantine_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_index_plan ", "quarantine=")
    }

    pub fn memory_index_delete_duplicate_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_index_plan ", "delete_duplicate=")
    }

    pub fn memory_index_skipped_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_index_plan ", "skipped=")
    }

    pub fn memory_index_detail_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "memory_index_plan ", "detail_codes=")
    }

    pub fn memory_index_detail_codes_for_kind(&self, kind: &str) -> Vec<String> {
        let prefix = format!("{kind}:");
        self.memory_index_detail_codes()
            .into_iter()
            .filter(|code| code.starts_with(&prefix))
            .collect()
    }

    pub fn memory_index_detail_codes_for_reason(&self, reason: &str) -> Vec<String> {
        let needle = format!(":{reason}:");
        self.memory_index_detail_codes()
            .into_iter()
            .filter(|code| code.contains(&needle))
            .collect()
    }

    pub fn memory_index_skipped_detail_codes(&self) -> Vec<String> {
        self.memory_index_detail_codes_for_kind("skipped")
    }

    pub fn context_injection_decision_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_context_injection ", "decisions=")
    }

    pub fn context_injection_admit_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_context_injection ", "admit=")
    }

    pub fn context_injection_summarize_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_context_injection ", "summarize=")
    }

    pub fn context_injection_reject_budget_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(
            &self.lines,
            "memory_context_injection ",
            "reject_budget=",
        )
    }

    pub fn context_injection_reject_risk_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_context_injection ", "reject_risk=")
    }

    pub fn context_injection_reject_scope_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(
            &self.lines,
            "memory_context_injection ",
            "reject_scope=",
        )
    }

    pub fn context_injection_reject_score_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(
            &self.lines,
            "memory_context_injection ",
            "reject_score=",
        )
    }

    pub fn context_injection_accepted_risk_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(
            &self.lines,
            "memory_context_injection ",
            "accepted_risk=",
        )
    }

    pub fn context_injection_used_tokens(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_context_injection ", "used_tokens=")
    }

    pub fn context_injection_reason_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "memory_context_injection ", "reason_codes=")
    }

    pub fn context_injection_detail_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "memory_context_injection ", "detail_codes=")
    }

    pub fn context_injection_detail_codes_for_reason(&self, reason: &str) -> Vec<String> {
        let needle = format!(":{reason}:");
        self.context_injection_detail_codes()
            .into_iter()
            .filter(|code| code.contains(&needle))
            .collect()
    }

    pub fn context_injection_reject_risk_detail_codes(&self) -> Vec<String> {
        self.context_injection_detail_codes()
            .into_iter()
            .filter(|code| code.starts_with("reject_risk:"))
            .collect()
    }

    pub fn context_injection_reject_risk_detail_codes_for_reason(
        &self,
        reason: &str,
    ) -> Vec<String> {
        let needle = format!("reject_risk:{reason}:");
        self.context_injection_reject_risk_detail_codes()
            .into_iter()
            .filter(|code| code.starts_with(&needle))
            .collect()
    }

    pub fn context_injection_missing_clean_gist_count(&self) -> usize {
        self.context_injection_detail_codes_for_reason("missing_clean_gist")
            .len()
    }

    pub fn context_injection_raw_fallback_count(&self) -> usize {
        self.context_injection_detail_codes_for_reason("raw_fallback_index_content")
            .len()
    }

    pub fn context_injection_truncated_index_content_count(&self) -> usize {
        self.context_injection_detail_codes_for_reason("truncated_index_content")
            .len()
    }

    pub fn memory_replay_planned_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_replay ", "planned=")
    }

    pub fn memory_replay_reinforced_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_replay ", "reinforced=")
    }

    pub fn memory_replay_penalized_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_replay ", "penalized=")
    }

    pub fn memory_replay_held_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_replay ", "held=")
    }

    pub fn memory_replay_touched_memory_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_replay ", "touched_memories=")
    }

    pub fn memory_replay_reinforcement_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_replay ", "memory_reinforcements=")
    }

    pub fn memory_replay_penalty_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_replay ", "memory_penalties=")
    }

    pub fn memory_replay_feedback_item_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_replay ", "feedback_items=")
    }

    pub fn memory_replay_feedback_update_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_replay ", "feedback_updates=")
    }

    pub fn memory_replay_feedback_applied_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_replay ", "feedback_applied=")
    }

    pub fn memory_replay_feedback_removed_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_replay ", "feedback_removed=")
    }

    pub fn memory_replay_feedback_missing_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_replay ", "feedback_missing=")
    }

    pub fn memory_replay_recursive_runtime_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_replay ", "recursive_runtime=")
    }

    pub fn memory_replay_live_memory_feedback_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_replay ", "live_memory_feedback=")
    }

    pub fn memory_replay_rust_check_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_replay ", "rust_check=")
    }

    pub fn memory_replay_business_contract_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_replay ", "business_contract=")
    }

    pub fn memory_replay_context_rot_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_replay ", "context_rot=")
    }

    pub fn memory_replay_reason_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "memory_replay ", "reason_codes=")
    }

    pub fn memory_replay_detail_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "memory_replay ", "detail_codes=")
    }

    pub fn memory_evolution_replay_run_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_evolution ", "replay_runs=")
    }

    pub fn memory_evolution_replay_item_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_evolution ", "replay_items=")
    }

    pub fn memory_evolution_replay_update_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_evolution ", "replay_updates=")
    }

    pub fn memory_evolution_replay_missing_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_evolution ", "replay_missing=")
    }

    pub fn memory_evolution_invalid_memory_id_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_evolution ", "invalid_memory_ids=")
    }

    pub fn memory_evolution_context_rot_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_evolution ", "context_rot_items=")
    }

    pub fn memory_evolution_live_feedback_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_evolution ", "live_feedback_items=")
    }

    pub fn memory_evolution_retention_decay_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_evolution ", "retention_decays=")
    }

    pub fn memory_evolution_retention_removal_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_evolution ", "retention_removals=")
    }

    pub fn memory_evolution_compaction_merge_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_evolution ", "compaction_merges=")
    }

    pub fn memory_evolution_compaction_removal_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_evolution ", "compaction_removals=")
    }

    pub fn memory_evolution_external_applied_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_evolution ", "external_applied=")
    }

    pub fn memory_evolution_external_missing_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_evolution ", "external_missing=")
    }

    pub fn memory_evolution_drift_rollback_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_evolution ", "drift_rollbacks=")
    }

    pub fn memory_evolution_hygiene_pressure_score(&self) -> usize {
        startup_sum_usize_from_line_prefix(
            &self.lines,
            "memory_evolution ",
            "hygiene_pressure_score=",
        )
        .saturating_add(startup_sum_usize_from_line_prefix(
            &self.lines,
            "memory_shadow ",
            "hygiene_pressure_score=",
        ))
    }

    pub fn memory_evolution_hygiene_pressure_priority_codes(&self) -> Vec<String> {
        let mut codes = startup_codes_from_line_prefix(
            &self.lines,
            "memory_evolution ",
            "hygiene_pressure_priority=",
        );
        codes.extend(startup_codes_from_line_prefix(
            &self.lines,
            "memory_shadow ",
            "hygiene_pressure_priority=",
        ));
        codes.sort();
        codes.dedup();
        codes
    }

    pub fn memory_evolution_hygiene_pressure_action_lanes(&self) -> Vec<String> {
        let mut codes = startup_codes_from_line_prefix(
            &self.lines,
            "memory_evolution ",
            "hygiene_pressure_action_lanes=",
        );
        codes.extend(startup_codes_from_line_prefix(
            &self.lines,
            "memory_shadow ",
            "hygiene_pressure_action_lanes=",
        ));
        codes.sort();
        codes.dedup();
        codes
    }

    pub fn memory_evolution_hygiene_pressure_action_lane_details(&self) -> Vec<String> {
        let mut codes = startup_codes_from_line_prefix(
            &self.lines,
            "memory_evolution ",
            "hygiene_pressure_action_lane_details=",
        );
        codes.extend(startup_codes_from_line_prefix(
            &self.lines,
            "memory_shadow ",
            "hygiene_pressure_action_lane_details=",
        ));
        codes.sort();
        codes.dedup();
        codes
    }

    pub fn memory_hygiene_work_plan_count(&self) -> usize {
        self.line_count_with_prefix("memory_hygiene_work_plan ")
    }

    pub fn memory_hygiene_work_next_action_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "memory_hygiene_work_plan ", "next_action=")
    }

    pub fn memory_hygiene_work_operator_review_count(&self) -> usize {
        startup_bool_count_from_line_prefix(
            &self.lines,
            "memory_hygiene_work_plan ",
            "operator_review=",
            true,
        )
    }

    pub fn memory_hygiene_work_isolation_count(&self) -> usize {
        startup_bool_count_from_line_prefix(
            &self.lines,
            "memory_hygiene_work_plan ",
            "isolation=",
            true,
        )
    }

    pub fn memory_hygiene_work_action_lanes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "memory_hygiene_work_plan ", "action_lanes=")
    }

    pub fn memory_hygiene_work_action_lane_details(&self) -> Vec<String> {
        startup_codes_from_line_prefix(
            &self.lines,
            "memory_hygiene_work_plan ",
            "action_lane_details=",
        )
    }

    pub fn memory_hygiene_work_next_dispatch_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "memory_hygiene_work_plan ", "dispatch_next=")
    }

    pub fn memory_hygiene_work_dispatch_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "memory_hygiene_work_plan ", "dispatch_codes=")
    }

    pub fn memory_hygiene_work_queue_count(&self) -> usize {
        self.line_count_with_prefix("memory_hygiene_work_queue ")
    }

    pub fn memory_hygiene_work_queue_item_count(&self) -> usize {
        startup_sum_usize_from_primary_or_shadow(
            &self.lines,
            "memory_hygiene_work_queue ",
            "items=",
            "memory_shadow ",
            "hygiene_work_queue_items=",
        )
    }

    pub fn memory_hygiene_work_queue_operator_review_count(&self) -> usize {
        startup_sum_usize_from_primary_or_shadow(
            &self.lines,
            "memory_hygiene_work_queue ",
            "operator_review=",
            "memory_shadow ",
            "hygiene_work_queue_operator_review=",
        )
    }

    pub fn memory_hygiene_work_queue_isolation_count(&self) -> usize {
        startup_sum_usize_from_primary_or_shadow(
            &self.lines,
            "memory_hygiene_work_queue ",
            "isolation=",
            "memory_shadow ",
            "hygiene_work_queue_isolation=",
        )
    }

    pub fn memory_hygiene_work_queue_next_dispatch_codes(&self) -> Vec<String> {
        startup_codes_from_primary_or_shadow(
            &self.lines,
            "memory_hygiene_work_queue ",
            "next_dispatch=",
            "memory_shadow ",
            "hygiene_work_queue_next_dispatch=",
        )
    }

    pub fn memory_hygiene_work_queue_lane_codes(&self) -> Vec<String> {
        startup_codes_from_primary_or_shadow(
            &self.lines,
            "memory_hygiene_work_queue ",
            "lanes=",
            "memory_shadow ",
            "hygiene_work_queue_lanes=",
        )
    }

    pub fn memory_hygiene_work_queue_lane_count(&self, lane: &str) -> usize {
        startup_code_occurrence_count_from_primary_or_shadow(
            &self.lines,
            "memory_hygiene_work_queue ",
            "lanes=",
            "memory_shadow ",
            "hygiene_work_queue_lanes=",
            lane,
        )
    }

    pub fn memory_hygiene_work_queue_priority_codes(&self) -> Vec<String> {
        startup_codes_from_primary_or_shadow(
            &self.lines,
            "memory_hygiene_work_queue ",
            "priorities=",
            "memory_shadow ",
            "hygiene_work_queue_priorities=",
        )
    }

    pub fn memory_hygiene_work_queue_priority_count(&self, priority: &str) -> usize {
        startup_code_occurrence_count_from_primary_or_shadow(
            &self.lines,
            "memory_hygiene_work_queue ",
            "priorities=",
            "memory_shadow ",
            "hygiene_work_queue_priorities=",
            priority,
        )
    }

    pub fn memory_hygiene_work_queue_dispatch_codes(&self) -> Vec<String> {
        startup_codes_from_primary_or_shadow(
            &self.lines,
            "memory_hygiene_work_queue ",
            "dispatch_codes=",
            "memory_shadow ",
            "hygiene_work_queue_dispatch_codes=",
        )
    }

    pub fn memory_hygiene_work_queue_detail_codes(&self) -> Vec<String> {
        startup_codes_from_primary_or_shadow(
            &self.lines,
            "memory_hygiene_work_queue ",
            "detail_codes=",
            "memory_shadow ",
            "hygiene_work_queue_detail_codes=",
        )
    }

    pub fn memory_hygiene_work_queue_reason_codes(&self) -> Vec<String> {
        startup_codes_from_primary_or_shadow(
            &self.lines,
            "memory_hygiene_work_queue ",
            "reason_codes=",
            "memory_shadow ",
            "hygiene_work_queue_reason_codes=",
        )
    }

    pub fn memory_hygiene_work_queue_reason_count(&self, reason: &str) -> usize {
        startup_code_occurrence_count_from_primary_or_shadow(
            &self.lines,
            "memory_hygiene_work_queue ",
            "reason_codes=",
            "memory_shadow ",
            "hygiene_work_queue_reason_codes=",
            reason,
        )
    }

    pub fn memory_hygiene_dispatch_pressure_rank(&self) -> usize {
        startup_sum_usize_from_line_prefix(
            &self.lines,
            "memory_hygiene_dispatch_pressure ",
            "rank=",
        )
    }

    pub fn memory_hygiene_dispatch_pressure_priority_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(
            &self.lines,
            "memory_hygiene_dispatch_pressure ",
            "priority=",
        )
    }

    pub fn memory_hygiene_dispatch_pressure_reason_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(
            &self.lines,
            "memory_hygiene_dispatch_pressure ",
            "reason_codes=",
        )
    }

    pub fn memory_hygiene_dispatch_pressure_reason_count(&self, reason: &str) -> usize {
        startup_code_occurrence_count_from_line_prefix(
            &self.lines,
            "memory_hygiene_dispatch_pressure ",
            "reason_codes=",
            reason,
        )
    }

    pub fn hygiene_dispatch_pressure_summary(&self) -> MemoryHygieneDispatchPressureSummary {
        let evolution_kvswap_boundary_blockers = startup_sum_usize_from_line_prefix(
            &self.lines,
            "memory_evolution ",
            "kvswap_boundary_blockers=",
        );
        let evolution_kvswap_boundary_warnings = startup_sum_usize_from_line_prefix(
            &self.lines,
            "memory_evolution ",
            "kvswap_boundary_warnings=",
        );
        MemoryHygieneDispatchPressureSummary {
            pressure_score: self.memory_evolution_hygiene_pressure_score(),
            queue_items: self.memory_hygiene_work_queue_item_count(),
            operator_review_items: self.memory_hygiene_work_queue_operator_review_count(),
            isolation_items: self.memory_hygiene_work_queue_isolation_count(),
            kvswap_boundary_repair_lanes: self
                .memory_hygiene_work_queue_lane_count("kvswap_boundary_repair"),
            context_rot_review_lanes: self
                .memory_hygiene_work_queue_lane_count("context_rot_review"),
            experience_index_rebuild_lanes: self
                .memory_hygiene_work_queue_lane_count("experience_index_rebuild"),
            quarantine_priorities: self.memory_hygiene_work_queue_priority_count("quarantine"),
            repair_priorities: self.memory_hygiene_work_queue_priority_count("repair"),
            context_rot_risks: self.context_rot_risk_count(),
            missing_clean_gist_pressure: self
                .context_rot_risk_reason_count("missing_clean_gist")
                .max(self.clean_gist_selection_reason_count("no_selection"))
                .max(self.governance_reason_count("missing_clean_gist"))
                .max(self.clean_gist_repair_missing_clean_gist_count())
                .max(self.experience_index_quality_gate_missing_clean_gist_count())
                .max(self.context_injection_missing_clean_gist_count())
                .max(self.memory_repair_skipped_reason_count("missing_clean_gist")),
            kvswap_boundary_blockers: evolution_kvswap_boundary_blockers
                .max(self.kvswap_boundary_blocker_count()),
            kvswap_boundary_warnings: evolution_kvswap_boundary_warnings
                .max(self.kvswap_boundary_warning_count()),
        }
    }

    pub fn memory_hygiene_work_item_count(&self) -> usize {
        self.line_count_with_prefix("memory_hygiene_work_item ")
    }

    pub fn memory_hygiene_work_item_lane_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "memory_hygiene_work_item ", "lane=")
    }

    pub fn memory_hygiene_work_item_priority_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "memory_hygiene_work_item ", "priority=")
    }

    pub fn memory_hygiene_work_item_dispatch_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "memory_hygiene_work_item ", "dispatch_code=")
    }

    pub fn memory_hygiene_work_item_detail_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "memory_hygiene_work_item ", "detail_code=")
    }

    pub fn memory_hygiene_work_reason_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "memory_hygiene_work_plan ", "reason_codes=")
    }

    pub fn memory_hygiene_work_detail_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "memory_hygiene_work_plan ", "detail_codes=")
    }

    pub fn memory_evolution_hygiene_work_next_action_codes(&self) -> Vec<String> {
        let direct_codes = self.memory_hygiene_work_next_action_codes();
        if !direct_codes.is_empty() {
            return direct_codes;
        }

        let mut codes = startup_codes_from_line_prefix(
            &self.lines,
            "memory_evolution ",
            "hygiene_work_next_action=",
        );
        codes.extend(startup_codes_from_line_prefix(
            &self.lines,
            "memory_shadow ",
            "hygiene_work_next_action=",
        ));
        codes.sort();
        codes.dedup();
        codes
    }

    pub fn memory_evolution_hygiene_work_operator_review_count(&self) -> usize {
        if self.memory_hygiene_work_plan_count() > 0 {
            return self.memory_hygiene_work_operator_review_count();
        }

        startup_bool_count_from_line_prefix(
            &self.lines,
            "memory_evolution ",
            "hygiene_work_operator_review=",
            true,
        )
        .saturating_add(startup_bool_count_from_line_prefix(
            &self.lines,
            "memory_shadow ",
            "hygiene_work_operator_review=",
            true,
        ))
    }

    pub fn memory_evolution_hygiene_work_isolation_count(&self) -> usize {
        if self.memory_hygiene_work_plan_count() > 0 {
            return self.memory_hygiene_work_isolation_count();
        }

        startup_bool_count_from_line_prefix(
            &self.lines,
            "memory_evolution ",
            "hygiene_work_isolation=",
            true,
        )
        .saturating_add(startup_bool_count_from_line_prefix(
            &self.lines,
            "memory_shadow ",
            "hygiene_work_isolation=",
            true,
        ))
    }

    pub fn memory_evolution_hygiene_pressure_reason_codes(&self) -> Vec<String> {
        let mut codes = startup_codes_from_line_prefix(
            &self.lines,
            "memory_evolution ",
            "hygiene_pressure_reason_codes=",
        );
        codes.extend(startup_codes_from_line_prefix(
            &self.lines,
            "memory_shadow ",
            "hygiene_pressure_reason_codes=",
        ));
        codes.sort();
        codes.dedup();
        codes
    }

    pub fn memory_evolution_hygiene_pressure_detail_codes(&self) -> Vec<String> {
        let mut codes = startup_codes_from_line_prefix(
            &self.lines,
            "memory_evolution ",
            "hygiene_pressure_detail_codes=",
        );
        codes.extend(startup_codes_from_line_prefix(
            &self.lines,
            "memory_shadow ",
            "hygiene_pressure_detail_codes=",
        ));
        codes.sort();
        codes.dedup();
        codes
    }

    pub fn memory_evolution_reason_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "memory_evolution ", "reason_codes=")
    }

    pub fn infini_memory_reason_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "infini_memory ", "reason_codes=")
    }

    pub fn infini_memory_local_window_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "infini_memory ", "local_window=")
    }

    pub fn infini_memory_global_memory_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "infini_memory ", "global_memory=")
    }

    pub fn infini_memory_skipped_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "infini_memory ", "skipped=")
    }

    pub fn infini_memory_local_token_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "infini_memory ", "local_tokens=")
    }

    pub fn infini_memory_global_token_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "infini_memory ", "global_tokens=")
    }

    pub fn infini_memory_skipped_token_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "infini_memory ", "skipped_tokens=")
    }

    pub fn infini_memory_selected_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "infini_memory ", "selected=")
    }

    pub fn infini_memory_detail_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "infini_memory ", "detail_codes=")
    }

    pub fn infini_memory_detail_codes_for_scope(&self, scope: &str) -> Vec<String> {
        let prefix = format!("{scope}:");
        self.infini_memory_detail_codes()
            .into_iter()
            .filter(|code| code.starts_with(&prefix))
            .collect()
    }

    pub fn infini_memory_skipped_detail_codes(&self) -> Vec<String> {
        self.infini_memory_detail_codes_for_scope("skipped")
    }

    pub fn infini_memory_skipped_detail_codes_for_reason(&self, reason: &str) -> Vec<String> {
        let prefix = format!("skipped:{reason}:");
        self.infini_memory_skipped_detail_codes()
            .into_iter()
            .filter(|code| code.starts_with(&prefix))
            .collect()
    }

    pub fn memory_retention_before_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_retention ", "before=")
    }

    pub fn memory_retention_after_estimate_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_retention ", "after_estimate=")
    }

    pub fn memory_retention_decay_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_retention ", "decays=")
    }

    pub fn memory_retention_removal_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_retention ", "removals=")
    }

    pub fn memory_retention_reason_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "memory_retention ", "reason_codes=")
    }

    pub fn memory_retention_detail_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "memory_retention ", "detail_codes=")
    }

    pub fn memory_compaction_before_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_compaction ", "before=")
    }

    pub fn memory_compaction_after_estimate_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_compaction ", "after_estimate=")
    }

    pub fn memory_compaction_merge_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_compaction ", "merges=")
    }

    pub fn memory_compaction_removal_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "memory_compaction ", "removals=")
    }

    pub fn memory_compaction_skipped_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "memory_compaction ", "skipped=")
    }

    pub fn memory_compaction_reason_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "memory_compaction ", "reason_codes=")
    }

    pub fn memory_compaction_detail_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "memory_compaction ", "detail_codes=")
    }

    pub fn kvswap_intent_reason_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "kvswap_intent ", "reason_codes=")
    }

    pub fn kvswap_prefetch_promote_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "kvswap_prefetch ", "promote=")
    }

    pub fn kvswap_prefetch_missing_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "kvswap_prefetch ", "missing=")
    }

    pub fn kvswap_prefetch_already_hot_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "kvswap_prefetch ", "hot=")
    }

    pub fn kvswap_prefetch_duplicate_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "kvswap_prefetch ", "duplicate=")
    }

    pub fn kvswap_prefetch_reason_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "kvswap_prefetch ", "reason_codes=")
    }

    pub fn kvswap_prefetch_detail_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "kvswap_prefetch ", "detail_codes=")
    }

    pub fn kvswap_prefetch_detail_codes_for_action(&self, action: &str) -> Vec<String> {
        let prefix = format!("{action}:");
        self.kvswap_prefetch_detail_codes()
            .into_iter()
            .filter(|code| code.starts_with(&prefix))
            .collect()
    }

    pub fn kvswap_prefetch_detail_codes_for_reason(&self, reason: &str) -> Vec<String> {
        let needle = format!(":{reason}:");
        self.kvswap_prefetch_detail_codes()
            .into_iter()
            .filter(|code| code.contains(&needle))
            .collect()
    }

    pub fn kvswap_eviction_target_hot_bytes(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "kvswap_eviction ", "target_hot_bytes=")
    }

    pub fn kvswap_eviction_demote_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "kvswap_eviction ", "demote=")
    }

    pub fn kvswap_eviction_keep_hot_count(&self) -> usize {
        startup_sum_usize_from_line_prefix(&self.lines, "kvswap_eviction ", "keep_hot=")
    }

    pub fn kvswap_eviction_reason_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "kvswap_eviction ", "reason_codes=")
    }

    pub fn kvswap_eviction_detail_codes(&self) -> Vec<String> {
        startup_codes_from_line_prefix(&self.lines, "kvswap_eviction ", "detail_codes=")
    }

    pub fn kvswap_eviction_detail_codes_for_action(&self, action: &str) -> Vec<String> {
        let prefix = format!("{action}:");
        self.kvswap_eviction_detail_codes()
            .into_iter()
            .filter(|code| code.starts_with(&prefix))
            .collect()
    }

    pub fn kvswap_eviction_detail_codes_for_reason(&self, reason: &str) -> Vec<String> {
        let needle = format!(":{reason}:");
        self.kvswap_eviction_detail_codes()
            .into_iter()
            .filter(|code| code.contains(&needle))
            .collect()
    }

    pub fn kvswap_state_present(&self) -> bool {
        self.has_line_prefix("kvswap_state ")
            || startup_bool_from_line_prefix(&self.lines, "memory_shadow ", "kvswap_state=")
                .unwrap_or_default()
    }

    pub fn kvswap_state_hot_shard_count(&self) -> usize {
        startup_usize_from_primary_or_shadow(
            &self.lines,
            "kvswap_state ",
            "hot=",
            "memory_shadow ",
            "kvswap_hot=",
        )
        .unwrap_or_default()
    }

    pub fn kvswap_state_cold_shard_count(&self) -> usize {
        startup_usize_from_primary_or_shadow(
            &self.lines,
            "kvswap_state ",
            "cold=",
            "memory_shadow ",
            "kvswap_cold=",
        )
        .unwrap_or_default()
    }

    pub fn kvswap_state_metadata_count(&self) -> usize {
        startup_usize_from_primary_or_shadow(
            &self.lines,
            "kvswap_state ",
            "metadata=",
            "memory_shadow ",
            "kvswap_metadata=",
        )
        .unwrap_or_default()
    }

    pub fn kvswap_state_total_byte_len(&self) -> usize {
        startup_usize_from_primary_or_shadow(
            &self.lines,
            "kvswap_state ",
            "total_bytes=",
            "memory_shadow ",
            "kvswap_bytes=",
        )
        .unwrap_or_default()
    }

    pub fn kvswap_state_shape_codes(&self) -> Vec<String> {
        startup_codes_from_primary_or_shadow(
            &self.lines,
            "kvswap_state ",
            "shape_codes=",
            "memory_shadow ",
            "kvswap_shape_codes=",
        )
    }

    pub fn kvswap_action_detail_codes(&self) -> Vec<String> {
        let mut codes =
            startup_codes_from_line_prefix(&self.lines, "kvswap_prefetch ", "detail_codes=")
                .into_iter()
                .map(|code| format!("prefetch:{code}"))
                .chain(
                    startup_codes_from_line_prefix(
                        &self.lines,
                        "kvswap_eviction ",
                        "detail_codes=",
                    )
                    .into_iter()
                    .map(|code| format!("eviction:{code}")),
                )
                .collect::<Vec<_>>();
        codes.sort();
        codes.dedup();
        codes
    }

    pub fn kvswap_action_detail_codes_for_stage(&self, stage: &str) -> Vec<String> {
        let prefix = format!("{stage}:");
        self.kvswap_action_detail_codes()
            .into_iter()
            .filter(|code| code.starts_with(&prefix))
            .collect()
    }

    pub fn kvswap_action_detail_codes_for_action(&self, action: &str) -> Vec<String> {
        let needle = format!(":{action}:");
        self.kvswap_action_detail_codes()
            .into_iter()
            .filter(|code| code.contains(&needle))
            .collect()
    }

    pub fn kvswap_action_detail_codes_for_reason(&self, reason: &str) -> Vec<String> {
        let needle = format!(":{reason}:");
        self.kvswap_action_detail_codes()
            .into_iter()
            .filter(|code| code.contains(&needle))
            .collect()
    }

    pub fn kvswap_boundary_report_count(&self) -> usize {
        self.line_count_with_prefix("kvswap_boundary ")
    }

    pub fn kvswap_boundary_clean_report_count(&self) -> usize {
        startup_bool_count_from_line_prefix(&self.lines, "kvswap_boundary ", "clean=", true)
    }

    pub fn kvswap_boundary_review_count(&self) -> usize {
        startup_bool_count_from_line_prefix(&self.lines, "kvswap_boundary ", "clean=", false)
    }

    pub fn kvswap_boundary_issue_count(&self) -> usize {
        startup_sum_usize_from_primary_or_shadow(
            &self.lines,
            "kvswap_boundary ",
            "issues=",
            "memory_shadow ",
            "kvswap_boundary_issues=",
        )
    }

    pub fn kvswap_boundary_reason_codes(&self) -> Vec<String> {
        startup_codes_from_primary_or_shadow(
            &self.lines,
            "kvswap_boundary ",
            "reason_codes=",
            "memory_shadow ",
            "kvswap_boundary_reason_codes=",
        )
    }

    pub fn kvswap_boundary_detail_codes(&self) -> Vec<String> {
        startup_codes_from_primary_or_shadow(
            &self.lines,
            "kvswap_boundary ",
            "detail_codes=",
            "memory_shadow ",
            "kvswap_boundary_detail_codes=",
        )
    }

    pub fn kvswap_boundary_overlap_count(&self) -> usize {
        startup_sum_usize_from_primary_or_shadow(
            &self.lines,
            "kvswap_boundary ",
            "overlap=",
            "memory_shadow ",
            "kvswap_boundary_overlap=",
        )
    }

    pub fn kvswap_boundary_missing_hot_metadata_count(&self) -> usize {
        startup_sum_usize_from_primary_or_shadow(
            &self.lines,
            "kvswap_boundary ",
            "missing_hot_metadata=",
            "memory_shadow ",
            "kvswap_boundary_missing_hot_metadata=",
        )
    }

    pub fn kvswap_boundary_stale_metadata_count(&self) -> usize {
        startup_sum_usize_from_primary_or_shadow(
            &self.lines,
            "kvswap_boundary ",
            "stale_metadata=",
            "memory_shadow ",
            "kvswap_boundary_stale_metadata=",
        )
    }

    pub fn kvswap_boundary_hot_tier_mismatch_count(&self) -> usize {
        startup_sum_usize_from_primary_or_shadow(
            &self.lines,
            "kvswap_boundary ",
            "hot_tier_mismatch=",
            "memory_shadow ",
            "kvswap_boundary_hot_tier_mismatch=",
        )
    }

    pub fn kvswap_boundary_cold_tier_mismatch_count(&self) -> usize {
        startup_sum_usize_from_primary_or_shadow(
            &self.lines,
            "kvswap_boundary ",
            "cold_tier_mismatch=",
            "memory_shadow ",
            "kvswap_boundary_cold_tier_mismatch=",
        )
    }

    pub fn kvswap_boundary_detail_codes_for(&self, issue: &str) -> Vec<String> {
        let prefix = format!("{issue}:");
        self.kvswap_boundary_detail_codes()
            .into_iter()
            .filter(|code| code.starts_with(&prefix))
            .collect()
    }

    pub fn kvswap_boundary_readiness_report_count(&self) -> usize {
        self.line_count_with_prefix("kvswap_boundary_readiness ")
    }

    pub fn kvswap_boundary_ready_count(&self) -> usize {
        startup_bool_count_from_line_prefix(
            &self.lines,
            "kvswap_boundary_readiness ",
            "ready=",
            true,
        )
    }

    pub fn kvswap_boundary_blocker_count(&self) -> usize {
        if self.kvswap_boundary_readiness_report_count() == 0 {
            return self
                .kvswap_boundary_overlap_count()
                .saturating_add(self.kvswap_boundary_missing_hot_metadata_count());
        }
        startup_sum_usize_from_line_prefix(&self.lines, "kvswap_boundary_readiness ", "blockers=")
    }

    pub fn kvswap_boundary_warning_count(&self) -> usize {
        if self.kvswap_boundary_readiness_report_count() == 0 {
            return self
                .kvswap_boundary_stale_metadata_count()
                .saturating_add(self.kvswap_boundary_hot_tier_mismatch_count())
                .saturating_add(self.kvswap_boundary_cold_tier_mismatch_count());
        }
        startup_sum_usize_from_line_prefix(&self.lines, "kvswap_boundary_readiness ", "warnings=")
    }

    pub fn kvswap_boundary_blocker_reason_codes(&self) -> Vec<String> {
        if self.kvswap_boundary_readiness_report_count() == 0 {
            return kvswap_boundary_blocker_reason_codes_from_counts(
                self.kvswap_boundary_overlap_count(),
                self.kvswap_boundary_missing_hot_metadata_count(),
            );
        }
        startup_codes_from_line_prefix(
            &self.lines,
            "kvswap_boundary_readiness ",
            "blocker_reason_codes=",
        )
    }

    pub fn kvswap_boundary_blocker_reason_count(&self, reason: &str) -> usize {
        startup_code_occurrence_count_from_line_prefix(
            &self.lines,
            "kvswap_boundary_readiness ",
            "blocker_reason_codes=",
            reason,
        )
    }

    pub fn kvswap_boundary_warning_reason_codes(&self) -> Vec<String> {
        if self.kvswap_boundary_readiness_report_count() == 0 {
            return kvswap_boundary_warning_reason_codes_from_counts(
                self.kvswap_boundary_stale_metadata_count(),
                self.kvswap_boundary_hot_tier_mismatch_count(),
                self.kvswap_boundary_cold_tier_mismatch_count(),
            );
        }
        startup_codes_from_line_prefix(
            &self.lines,
            "kvswap_boundary_readiness ",
            "warning_reason_codes=",
        )
    }

    pub fn kvswap_boundary_warning_reason_count(&self, reason: &str) -> usize {
        startup_code_occurrence_count_from_line_prefix(
            &self.lines,
            "kvswap_boundary_readiness ",
            "warning_reason_codes=",
            reason,
        )
    }

    pub fn kvswap_boundary_readiness_detail_codes(&self) -> Vec<String> {
        if self.kvswap_boundary_readiness_report_count() == 0 {
            return kvswap_boundary_readiness_detail_codes_from_boundary(
                &self.kvswap_boundary_detail_codes(),
            );
        }
        startup_codes_from_line_prefix(&self.lines, "kvswap_boundary_readiness ", "detail_codes=")
    }

    pub fn detail_codes(&self) -> Vec<String> {
        let mut codes = self
            .missing_required_codes()
            .into_iter()
            .map(|code| format!("missing_line:{code}"))
            .collect::<Vec<_>>();
        if self.requires_operator_review {
            for line in &self.lines {
                codes.extend(startup_line_detail_codes(line));
            }
        }
        if self.requires_operator_review {
            codes.push("operator_review_required".to_owned());
        }
        if self.approved_phases.is_empty() {
            codes.push("approved_phases:none".to_owned());
        }
        if self.projection_contract_manifest_gap() > 0 {
            codes.push(format!(
                "projection_contract_manifest_gap:{}",
                self.projection_contract_manifest_gap()
            ));
        }
        if !codes.iter().all(|code| !code.starts_with("missing_line:")) {
            codes.push("incomplete_evidence".to_owned());
        }
        codes.sort();
        codes.dedup();
        codes
    }

    pub fn status_codes(&self) -> Vec<String> {
        let mut codes = std::collections::BTreeSet::new();
        if self.is_complete() {
            codes.insert("complete".to_owned());
        } else {
            codes.insert("incomplete_evidence".to_owned());
        }
        if self.requires_operator_review {
            codes.insert("operator_review_required".to_owned());
        } else {
            codes.insert("review_clear".to_owned());
        }
        if self.approved_phases.is_empty() {
            codes.insert("no_approved_phases".to_owned());
        } else {
            codes.insert("phases_approved".to_owned());
        }
        if self.clean_gist_repair_issue_count() > 0 {
            codes.insert("clean_gist_repair_required".to_owned());
        }
        codes.into_iter().collect()
    }

    pub fn is_complete(&self) -> bool {
        self.missing_required_line_prefixes().is_empty()
            && self.projection_contract_manifests_complete()
    }

    pub fn summary_line(&self) -> String {
        let missing = self.missing_required_line_prefixes();
        let missing_codes = self.missing_required_codes();
        format!(
            "memory_startup_evidence complete={} review={} approved_phases={} lines={} missing_required={} projection_contracts={} projection_contract_manifests={} projection_contract_manifest_gap={} missing_prefixes={} missing_codes={} context_rot_risks={} context_rot_risk_reason_codes={} context_rot_blocker_reason_codes={} context_rot_risk_detail_codes={} migration_guard_codes={} migration_detail_codes={} kvswap_boundary_issues={} kvswap_boundary_reason_codes={} kvswap_boundary_detail_codes={} status_codes={} detail_codes={}",
            self.is_complete(),
            self.requires_operator_review,
            self.approved_phases.len(),
            self.lines.len(),
            missing.len(),
            self.projection_contract_count(),
            self.projection_contract_manifest_count(),
            self.projection_contract_manifest_gap(),
            if missing.is_empty() {
                "none".to_owned()
            } else {
                missing.join("|")
            },
            join_codes(missing_codes),
            self.context_rot_risk_count(),
            join_codes(self.context_rot_risk_reason_codes()),
            join_codes(self.context_rot_blocker_reason_codes()),
            join_codes(self.context_rot_risk_detail_codes()),
            join_codes(self.migration_evidence_guard_codes()),
            join_codes(self.migration_evidence_detail_codes()),
            self.kvswap_boundary_issue_count(),
            join_codes(self.kvswap_boundary_reason_codes()),
            join_codes(self.kvswap_boundary_detail_codes()),
            join_codes(self.status_codes()),
            join_codes(self.detail_codes()),
        )
    }

    pub fn emit_to<S: MemoryStartupEvidenceSink>(&self, sink: &mut S) -> MemoryResult<()> {
        sink.record_startup_evidence(self)
    }
}

impl MemoryStartupAdmissionEvidence {
    pub fn from_startup_evidence(evidence: &MemoryServiceStartupEvidence) -> Self {
        let context_admit = evidence.context_injection_admit_count();
        let context_summarize = evidence.context_injection_summarize_count();

        Self {
            read_only_review_required: startup_bool_count_from_line_prefix(
                &evidence.lines,
                "memory_read_only_plan ",
                "review=",
                true,
            ) > 0,
            index_quality_blocker_count: evidence.experience_index_quality_gate_blocker_count(),
            index_quality_warning_count: evidence.experience_index_quality_gate_warning_count(),
            index_operation_count: evidence.memory_index_operation_count(),
            index_refresh_count: evidence.memory_index_refresh_count(),
            index_detail_codes: evidence.memory_index_detail_codes(),
            context_rot_risk_count: evidence.context_rot_risk_count(),
            context_rot_blocker_reason_codes: evidence.context_rot_blocker_reason_codes(),
            admission_decision_count: evidence.context_injection_decision_count(),
            admission_accepted_count: context_admit.saturating_add(context_summarize),
            admission_risk_rejection_count: evidence.context_injection_reject_risk_count(),
            migration_live_store_targeted_count: evidence
                .migration_evidence_live_store_targeted_count(),
            adapter_live_write_count: evidence.adapter_status_live_write_count(),
            live_write_phase_request_count: evidence.migration_approval_live_write_required_count(),
            store_mutation_count: 0,
            helper_prose_line_count: evidence.line_count_with_prefix("helper_prose "),
            non_contract_line_count: evidence
                .lines
                .iter()
                .filter(|line| !startup_admission_contract_prefix(line))
                .count(),
        }
    }

    pub fn live_store_mutation_requested(&self) -> bool {
        self.migration_live_store_targeted_count > 0 || self.adapter_live_write_count > 0
    }

    pub fn ndkv_write_allowed(&self) -> bool {
        false
    }

    pub fn admission_expanded_by_non_contract_evidence(&self) -> bool {
        false
    }

    pub fn read_only_contract_holds(&self) -> bool {
        !self.live_store_mutation_requested()
            && !self.ndkv_write_allowed()
            && self.store_mutation_count == 0
            && self.admission_accepted_count <= self.admission_decision_count
            && !self.admission_expanded_by_non_contract_evidence()
    }

    pub fn summary_line(&self) -> String {
        format!(
            "memory_startup_admission_evidence read_only_contract={} read_only_review={} index_quality_blockers={} index_quality_warnings={} index_ops={} index_refresh={} context_rot_risks={} context_rot_blocker_reason_codes={} admission_decisions={} admission_accepted={} admission_risk_rejections={} live_store_targeted={} adapter_live_write={} live_write_phase_requests={} live_store_mutation_requested={} store_mutations={} ndkv_write_allowed={} helper_prose_lines={} non_contract_lines={} admission_expanded_by_non_contract={} index_detail_codes={}",
            self.read_only_contract_holds(),
            self.read_only_review_required,
            self.index_quality_blocker_count,
            self.index_quality_warning_count,
            self.index_operation_count,
            self.index_refresh_count,
            self.context_rot_risk_count,
            join_codes(self.context_rot_blocker_reason_codes.clone()),
            self.admission_decision_count,
            self.admission_accepted_count,
            self.admission_risk_rejection_count,
            self.migration_live_store_targeted_count,
            self.adapter_live_write_count,
            self.live_write_phase_request_count,
            self.live_store_mutation_requested(),
            self.store_mutation_count,
            self.ndkv_write_allowed(),
            self.helper_prose_line_count,
            self.non_contract_line_count,
            self.admission_expanded_by_non_contract_evidence(),
            join_codes(self.index_detail_codes.clone()),
        )
    }
}

fn startup_admission_contract_prefix(line: &str) -> bool {
    MemoryServiceStartupEvidence::required_line_prefixes()
        .iter()
        .any(|prefix| line.starts_with(prefix))
        || line.starts_with("memory_migration ")
        || line.starts_with("memory_adapter_status ")
        || line.starts_with("memory_startup_evidence ")
}

pub trait MemoryStartupEvidenceSink {
    fn record_evidence_line(&mut self, line: &str) -> MemoryResult<()>;

    fn record_startup_evidence(
        &mut self,
        evidence: &MemoryServiceStartupEvidence,
    ) -> MemoryResult<()> {
        for line in &evidence.lines {
            self.record_evidence_line(line)?;
        }
        self.record_evidence_line(&evidence.summary_line())
    }
}

impl MemoryStartupEvidenceSink for Vec<String> {
    fn record_evidence_line(&mut self, line: &str) -> MemoryResult<()> {
        self.push(line.to_owned());
        Ok(())
    }
}

fn startup_line_detail_codes(line: &str) -> Vec<String> {
    let fields: &[&str] = if line.starts_with("memory_shadow ") {
        &[
            "context_rot_risk_reason_codes=",
            "context_rot_risk_detail_codes=",
            "detail_codes=",
        ]
    } else if line.starts_with("memory_migration_evidence ")
        || line.starts_with("memory_projection_parity ")
        || line.starts_with("context_rot_risk ")
        || line.starts_with("memory_repair_plan ")
        || line.starts_with("memory_index_plan ")
        || line.starts_with("clean_gist_selection ")
        || line.starts_with("clean_gist_repair ")
        || line.starts_with("kvswap_boundary ")
        || line.starts_with("kvswap_boundary_readiness ")
        || line.starts_with("kvswap_prefetch ")
        || line.starts_with("kvswap_eviction ")
    {
        &["detail_codes="]
    } else if line.starts_with("adapter_projection")
        || line.starts_with("memory_adapter_checklist ")
        || line.starts_with("memory_migration_readiness ")
        || line.starts_with("memory_migration phase=")
    {
        &["blocker_detail_codes=", "warning_detail_codes="]
    } else if line.starts_with("memory_adapter_checklist_item ") {
        &["detail_codes="]
    } else {
        &[]
    };

    line.split_whitespace()
        .filter_map(|part| fields.iter().find_map(|prefix| part.strip_prefix(prefix)))
        .filter(|value| *value != "none")
        .flat_map(|value| value.split('|').filter(|code| !code.is_empty()))
        .map(str::to_owned)
        .collect()
}

fn is_clean_gist_repair_detail_code(code: &str) -> bool {
    code.starts_with("missing_clean_gist:")
        || code.starts_with("dirty_clean_gist:")
        || code.starts_with("dirty_gist:")
}

fn startup_usize_from_line_prefix(
    lines: &[String],
    line_prefix: &str,
    field_prefix: &str,
) -> Option<usize> {
    lines
        .iter()
        .find(|line| line.starts_with(line_prefix))
        .and_then(|line| startup_line_field_value(line, field_prefix))
        .and_then(|value| value.parse::<usize>().ok())
}

fn startup_usize_from_primary_or_shadow(
    lines: &[String],
    primary_line_prefix: &str,
    primary_field_prefix: &str,
    shadow_line_prefix: &str,
    shadow_field_prefix: &str,
) -> Option<usize> {
    startup_usize_from_line_prefix(lines, primary_line_prefix, primary_field_prefix)
        .or_else(|| startup_usize_from_line_prefix(lines, shadow_line_prefix, shadow_field_prefix))
}

fn startup_sum_usize_from_line_prefix(
    lines: &[String],
    line_prefix: &str,
    field_prefix: &str,
) -> usize {
    lines
        .iter()
        .filter(|line| line.starts_with(line_prefix))
        .filter_map(|line| startup_line_field_value(line, field_prefix))
        .filter_map(|value| value.parse::<usize>().ok())
        .sum()
}

fn startup_sum_usize_from_primary_or_shadow(
    lines: &[String],
    primary_line_prefix: &str,
    primary_field_prefix: &str,
    shadow_line_prefix: &str,
    shadow_field_prefix: &str,
) -> usize {
    if lines
        .iter()
        .any(|line| line.starts_with(primary_line_prefix))
    {
        return startup_sum_usize_from_line_prefix(
            lines,
            primary_line_prefix,
            primary_field_prefix,
        );
    }
    startup_sum_usize_from_line_prefix(lines, shadow_line_prefix, shadow_field_prefix)
}

fn startup_has_field_from_line_prefix(
    lines: &[String],
    line_prefix: &str,
    field_prefix: &str,
) -> bool {
    lines
        .iter()
        .filter(|line| line.starts_with(line_prefix))
        .any(|line| startup_line_field_value(line, field_prefix).is_some())
}

fn startup_sum_usize_from_projection_chain(
    lines: &[String],
    primary_line_prefix: &str,
    primary_field_prefix: &str,
    shadow_line_prefix: &str,
    shadow_field_prefix: &str,
    legacy_line_prefix: &str,
    legacy_field_prefix: &str,
) -> usize {
    if startup_has_field_from_line_prefix(lines, primary_line_prefix, primary_field_prefix) {
        return startup_sum_usize_from_line_prefix(
            lines,
            primary_line_prefix,
            primary_field_prefix,
        );
    }
    if startup_has_field_from_line_prefix(lines, shadow_line_prefix, shadow_field_prefix) {
        return startup_sum_usize_from_line_prefix(lines, shadow_line_prefix, shadow_field_prefix);
    }
    startup_sum_usize_from_line_prefix(lines, legacy_line_prefix, legacy_field_prefix)
}

fn startup_sum_code_count_from_line_prefix(
    lines: &[String],
    line_prefix: &str,
    field_prefix: &str,
) -> usize {
    lines
        .iter()
        .filter(|line| line.starts_with(line_prefix))
        .filter_map(|line| startup_line_field_value(line, field_prefix))
        .map(|value| split_startup_codes(value).count())
        .sum()
}

fn startup_sum_name_count_from_line_prefix(
    lines: &[String],
    line_prefix: &str,
    field_prefix: &str,
) -> usize {
    lines
        .iter()
        .filter(|line| line.starts_with(line_prefix))
        .filter_map(|line| startup_line_field_value(line, field_prefix))
        .map(|value| {
            value
                .split(',')
                .filter(|name| {
                    let name = name.trim();
                    !name.is_empty() && name != "none"
                })
                .count()
        })
        .sum()
}

fn startup_bool_from_line_prefix(
    lines: &[String],
    line_prefix: &str,
    field_prefix: &str,
) -> Option<bool> {
    lines
        .iter()
        .find(|line| line.starts_with(line_prefix))
        .and_then(|line| startup_line_field_value(line, field_prefix))
        .and_then(|value| value.parse::<bool>().ok())
}

fn startup_bool_count_from_line_prefix(
    lines: &[String],
    line_prefix: &str,
    field_prefix: &str,
    expected: bool,
) -> usize {
    lines
        .iter()
        .filter(|line| line.starts_with(line_prefix))
        .filter(|line| {
            startup_line_field_value(line, field_prefix)
                .and_then(|value| value.parse::<bool>().ok())
                == Some(expected)
        })
        .count()
}

fn startup_field_value_count_from_line_prefix(
    lines: &[String],
    line_prefix: &str,
    field_prefix: &str,
    expected: &str,
) -> usize {
    lines
        .iter()
        .filter(|line| line.starts_with(line_prefix))
        .filter(|line| startup_line_field_value(line, field_prefix) == Some(expected))
        .count()
}

fn startup_code_occurrence_count_from_line_prefix(
    lines: &[String],
    line_prefix: &str,
    field_prefix: &str,
    expected: &str,
) -> usize {
    lines
        .iter()
        .filter(|line| line.starts_with(line_prefix))
        .filter_map(|line| startup_line_field_value(line, field_prefix))
        .filter(|value| split_startup_codes(value).any(|code| code == expected))
        .count()
}

fn startup_codes_from_line_prefix(
    lines: &[String],
    line_prefix: &str,
    field_prefix: &str,
) -> Vec<String> {
    lines
        .iter()
        .filter(|line| line.starts_with(line_prefix))
        .filter_map(|line| startup_line_field_value(line, field_prefix))
        .flat_map(split_startup_codes)
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn startup_codes_from_primary_or_shadow(
    lines: &[String],
    primary_line_prefix: &str,
    primary_field_prefix: &str,
    shadow_line_prefix: &str,
    shadow_field_prefix: &str,
) -> Vec<String> {
    let codes = startup_codes_from_line_prefix(lines, primary_line_prefix, primary_field_prefix);
    if !codes.is_empty() {
        return codes;
    }
    startup_codes_from_line_prefix(lines, shadow_line_prefix, shadow_field_prefix)
}

fn startup_codes_from_line_prefix_preserve_order(
    lines: &[String],
    line_prefix: &str,
    field_prefix: &str,
) -> Vec<String> {
    let mut seen = std::collections::BTreeSet::new();
    let mut codes = Vec::new();
    for code in lines
        .iter()
        .filter(|line| line.starts_with(line_prefix))
        .filter_map(|line| startup_line_field_value(line, field_prefix))
        .flat_map(split_startup_codes)
    {
        if seen.insert(code.clone()) {
            codes.push(code);
        }
    }
    codes
}

fn startup_codes_from_primary_or_shadow_preserve_order(
    lines: &[String],
    primary_line_prefix: &str,
    primary_field_prefix: &str,
    shadow_line_prefix: &str,
    shadow_field_prefix: &str,
) -> Vec<String> {
    let codes = startup_codes_from_line_prefix_preserve_order(
        lines,
        primary_line_prefix,
        primary_field_prefix,
    );
    if !codes.is_empty() {
        return codes;
    }
    startup_codes_from_line_prefix_preserve_order(lines, shadow_line_prefix, shadow_field_prefix)
}

fn startup_code_occurrence_count_from_primary_or_shadow(
    lines: &[String],
    primary_line_prefix: &str,
    primary_field_prefix: &str,
    shadow_line_prefix: &str,
    shadow_field_prefix: &str,
    expected: &str,
) -> usize {
    let primary_codes =
        startup_codes_from_line_prefix(lines, primary_line_prefix, primary_field_prefix);
    let count = startup_code_occurrence_count_from_line_prefix(
        lines,
        primary_line_prefix,
        primary_field_prefix,
        expected,
    );
    if !primary_codes.is_empty() {
        return count;
    }
    startup_code_occurrence_count_from_line_prefix(
        lines,
        shadow_line_prefix,
        shadow_field_prefix,
        expected,
    )
}

fn startup_line_field_value<'a>(line: &'a str, field_prefix: &str) -> Option<&'a str> {
    line.split_whitespace()
        .find_map(|part| part.strip_prefix(field_prefix))
        .filter(|value| *value != "none")
}

fn split_startup_codes(value: &str) -> impl Iterator<Item = String> + '_ {
    value
        .split('|')
        .filter(|code| !code.is_empty() && *code != "none")
        .map(str::to_owned)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryServiceChecklistSeverity {
    Info,
    Warning,
    Blocker,
}

impl MemoryServiceChecklistSeverity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Blocker => "blocker",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryServiceChecklistItem {
    pub code: String,
    pub satisfied: bool,
    pub severity: MemoryServiceChecklistSeverity,
    pub detail: String,
}

impl MemoryServiceChecklistItem {
    pub fn new(
        code: impl Into<String>,
        satisfied: bool,
        severity: MemoryServiceChecklistSeverity,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            satisfied,
            severity,
            detail: detail.into(),
        }
    }

    pub fn detail_codes(&self) -> Vec<String> {
        detail_codes_from_text(&self.detail)
    }

    pub fn summary_line(&self) -> String {
        format!(
            "memory_adapter_checklist_item code={} satisfied={} severity={} detail_codes={}",
            self.code,
            self.satisfied,
            self.severity.as_str(),
            join_codes(self.detail_codes()),
        )
    }
}

fn detail_codes_from_text(detail: &str) -> Vec<String> {
    let mut codes = Vec::new();
    for token in detail.split_whitespace() {
        let Some((key, value)) = token.split_once('=') else {
            continue;
        };
        codes.extend(detail_field_codes(key, value));
    }
    codes.sort();
    codes.dedup();
    codes
}

fn detail_field_codes(key: &str, value: &str) -> Vec<String> {
    let key = key.trim();
    let value = value.trim().trim_matches(|ch: char| ch == ',' || ch == ';');
    if key.is_empty() || detail_value_is_empty(value) {
        return Vec::new();
    }
    if key.ends_with("_codes") {
        return value
            .split('|')
            .map(|code| code.trim().trim_matches(|ch: char| ch == ',' || ch == ';'))
            .filter(|code| !detail_value_is_empty(code))
            .map(|code| format!("{key}:{code}"))
            .collect();
    }
    vec![key.to_owned()]
}

fn detail_value_is_empty(value: &str) -> bool {
    let value = value.trim();
    value.is_empty() || value == "0" || value.eq_ignore_ascii_case("false") || value == "none"
}

fn join_codes(codes: Vec<String>) -> String {
    if codes.is_empty() {
        "none".to_owned()
    } else {
        codes.join("|")
    }
}

fn join_names(names: &[String]) -> String {
    if names.is_empty() {
        "none".to_owned()
    } else {
        names.join(",")
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MemoryServiceAdapterChecklist {
    pub items: Vec<MemoryServiceChecklistItem>,
}

impl MemoryServiceAdapterChecklist {
    pub fn from_dry_run(dry_run: &MemoryServiceDryRun) -> Self {
        let plan = &dry_run.plan;
        let summary = &dry_run.summary;
        let mut items = vec![
            MemoryServiceChecklistItem::new(
                "capability_manifest_ready",
                plan.readiness.ready,
                MemoryServiceChecklistSeverity::Blocker,
                plan.readiness.capability_manifest_checklist_detail(),
            ),
            MemoryServiceChecklistItem::new(
                "projection_shadow_read_ready",
                plan.projection_audit.is_ready_for_shadow_read(),
                MemoryServiceChecklistSeverity::Blocker,
                plan.projection_audit.shadow_read_checklist_detail(),
            ),
            MemoryServiceChecklistItem::new(
                "projection_isolated_write_ready",
                plan.projection_audit.is_ready_for_isolated_write(),
                MemoryServiceChecklistSeverity::Blocker,
                plan.projection_audit.isolated_write_checklist_detail(),
            ),
            MemoryServiceChecklistItem::new(
                "projection_contracts_ready",
                plan.projection_coverage.iter().all(|report| report.ready),
                MemoryServiceChecklistSeverity::Blocker,
                summary.projection_contracts_ready_checklist_detail(),
            ),
            MemoryServiceChecklistItem::new(
                "projection_contract_warnings_clean",
                plan.projection_coverage
                    .iter()
                    .all(|report| report.warnings.is_empty()),
                MemoryServiceChecklistSeverity::Warning,
                summary.projection_contract_warnings_checklist_detail(),
            ),
            MemoryServiceChecklistItem::new(
                "adapter_snapshots_clean",
                summary.adapter_snapshot_warning_count == 0,
                MemoryServiceChecklistSeverity::Warning,
                summary.adapter_snapshot_checklist_detail(),
            ),
            MemoryServiceChecklistItem::new(
                "context_gate_clean",
                summary.context_rejection_count == 0,
                MemoryServiceChecklistSeverity::Warning,
                summary.context_gate_checklist_detail(),
            ),
            MemoryServiceChecklistItem::new(
                "context_rot_risks_clean",
                summary.context_rot_risk_is_clean(),
                MemoryServiceChecklistSeverity::Warning,
                summary.context_rot_risk_checklist_detail(),
            ),
            MemoryServiceChecklistItem::new(
                "experience_index_quality_gate_ready",
                plan.read_only.quality_gate.ready_for_context_injection,
                MemoryServiceChecklistSeverity::Warning,
                plan.read_only.quality_gate.checklist_detail(),
            ),
            MemoryServiceChecklistItem::new(
                "clean_gist_repair_clean",
                summary.clean_gist_repair_is_clean(),
                MemoryServiceChecklistSeverity::Warning,
                summary.clean_gist_repair_checklist_detail(),
            ),
            MemoryServiceChecklistItem::new(
                "repair_plan_clean",
                summary.repair_item_count == 0 && summary.repair_skipped_count == 0,
                MemoryServiceChecklistSeverity::Warning,
                summary.repair_plan_checklist_detail(),
            ),
            MemoryServiceChecklistItem::new(
                "replay_evidence_ready",
                plan.evolution_ledger.replay_runs > 0,
                MemoryServiceChecklistSeverity::Blocker,
                plan.evolution_ledger.replay_evidence_checklist_detail(),
            ),
            MemoryServiceChecklistItem::new(
                "kvswap_intent_clean",
                summary.kvswap_prefetch_count == 0 && summary.kvswap_evict_count == 0,
                MemoryServiceChecklistSeverity::Warning,
                summary.kvswap_intent_checklist_detail(),
            ),
            MemoryServiceChecklistItem::new(
                "kvswap_boundary_clean",
                summary.kvswap_boundary_issue_count == 0,
                MemoryServiceChecklistSeverity::Warning,
                summary.kvswap_boundary_checklist_detail(),
            ),
            MemoryServiceChecklistItem::new(
                "evolution_gate_ready",
                plan.evolution_assessment.allow_isolated_write,
                MemoryServiceChecklistSeverity::Blocker,
                plan.evolution_assessment.checklist_detail(),
            ),
            MemoryServiceChecklistItem::new(
                "inspection_ready",
                !plan.inspection.has_blockers(),
                MemoryServiceChecklistSeverity::Blocker,
                plan.inspection.checklist_detail(),
            ),
            MemoryServiceChecklistItem::new(
                "projection_parity_clean",
                !plan.projection_parity_audit.requires_operator_review(),
                MemoryServiceChecklistSeverity::Blocker,
                plan.projection_parity_audit.checklist_detail(),
            ),
            {
                let copied_fixture_required = dry_run
                    .approvals
                    .iter()
                    .any(|approval| approval.phase.requires_copied_fixture());
                MemoryServiceChecklistItem::new(
                    "migration_evidence_ready",
                    dry_run
                        .migration_evidence
                        .checklist_guard_codes(copied_fixture_required)
                        .is_empty(),
                    MemoryServiceChecklistSeverity::Blocker,
                    dry_run
                        .migration_evidence
                        .checklist_detail(copied_fixture_required),
                )
            },
        ];

        items.extend(dry_run.approvals.iter().map(|approval| {
            MemoryServiceChecklistItem::new(
                format!("migration_phase:{}", approval.phase.as_str()),
                approval.approved,
                MemoryServiceChecklistSeverity::Blocker,
                approval.checklist_detail(),
            )
        }));

        Self { items }
    }

    pub fn blockers(&self) -> Vec<&MemoryServiceChecklistItem> {
        self.items
            .iter()
            .filter(|item| {
                !item.satisfied && item.severity == MemoryServiceChecklistSeverity::Blocker
            })
            .collect()
    }

    pub fn warnings(&self) -> Vec<&MemoryServiceChecklistItem> {
        self.items
            .iter()
            .filter(|item| {
                !item.satisfied && item.severity == MemoryServiceChecklistSeverity::Warning
            })
            .collect()
    }

    pub fn is_satisfied(&self) -> bool {
        self.blockers().is_empty() && self.warnings().is_empty()
    }

    pub fn blocker_detail_codes(&self) -> Vec<String> {
        prefixed_item_detail_codes(self.blockers())
    }

    pub fn warning_detail_codes(&self) -> Vec<String> {
        prefixed_item_detail_codes(self.warnings())
    }

    pub fn summary_line(&self) -> String {
        let blockers = self
            .blockers()
            .iter()
            .map(|item| item.code.as_str())
            .collect::<Vec<_>>();
        let warnings = self
            .warnings()
            .iter()
            .map(|item| item.code.as_str())
            .collect::<Vec<_>>();
        format!(
            "memory_adapter_checklist satisfied={} items={} blockers={} warnings={} blocker_codes={} warning_codes={} blocker_detail_codes={} warning_detail_codes={}",
            self.is_satisfied(),
            self.items.len(),
            blockers.len(),
            warnings.len(),
            if blockers.is_empty() {
                "none".to_owned()
            } else {
                blockers.join("|")
            },
            if warnings.is_empty() {
                "none".to_owned()
            } else {
                warnings.join("|")
            },
            join_codes(self.blocker_detail_codes()),
            join_codes(self.warning_detail_codes()),
        )
    }
}

fn prefixed_item_detail_codes(items: Vec<&MemoryServiceChecklistItem>) -> Vec<String> {
    let mut codes = items
        .iter()
        .flat_map(|item| {
            item.detail_codes()
                .into_iter()
                .map(|code| format!("{}:{code}", item.code))
        })
        .collect::<Vec<_>>();
    codes.sort();
    codes.dedup();
    codes
}

fn context_rot_risk_summary_line(summary: &MemoryServiceShadowSummary) -> String {
    format!(
        "context_rot_risk risks={} reason_codes={} detail_codes={}",
        summary.context_rot_risk_count,
        join_codes(summary.context_rot_risk_reason_codes.clone()),
        join_codes(summary.context_rot_risk_detail_codes.clone()),
    )
}

impl MemoryServiceDryRun {
    pub fn for_inputs(
        inputs: MemoryServiceShadowPlanInputs<'_>,
        evidence: &MemoryMigrationEvidence,
        phases: &[MemoryMigrationPhase],
        operator_ack: bool,
    ) -> Self {
        let plan = MemoryServiceShadowPlan::for_inputs(inputs);
        let summary = plan.summary();
        let approvals = phases
            .iter()
            .map(|phase| plan.migration_approval(*phase, evidence, operator_ack))
            .collect::<Vec<_>>();

        Self {
            plan,
            summary,
            migration_evidence: evidence.clone(),
            approvals,
        }
    }

    pub fn approval_for(&self, phase: MemoryMigrationPhase) -> Option<&MemoryMigrationApproval> {
        self.approvals
            .iter()
            .find(|approval| approval.phase == phase)
    }

    pub fn requires_operator_review(&self) -> bool {
        self.summary.requires_operator_review
            || self
                .approvals
                .iter()
                .any(MemoryMigrationApproval::requires_operator_review)
    }

    pub fn approved_phases(&self) -> Vec<MemoryMigrationPhase> {
        self.approvals
            .iter()
            .filter(|approval| approval.approved)
            .map(|approval| approval.phase)
            .collect()
    }

    pub fn adapter_checklist(&self) -> MemoryServiceAdapterChecklist {
        MemoryServiceAdapterChecklist::from_dry_run(self)
    }

    pub fn startup_evidence(&self) -> MemoryServiceStartupEvidence {
        let checklist = self.adapter_checklist();
        let hygiene_work_plan = self.plan.evolution_ledger.hygiene_pressure().work_plan();
        let mut lines = vec![
            self.summary.summary_line(),
            self.plan.requirement.summary_line(),
            self.plan.readiness.summary_line(),
            self.plan.read_only.summary_line(),
            self.plan.projection_audit.summary_line(),
            self.plan.read_only.governance.summary_line(),
            self.plan.read_only.rebuild.summary_line(),
            self.plan.read_only.rebuild.clean_gist_repair_summary_line(),
            self.plan.read_only.quality_gate.summary_line(),
            self.plan.read_only.repair.summary_line(),
            self.plan
                .read_only
                .repair
                .genome_repair_factor_plan()
                .summary_line(),
            self.plan.read_only.index.summary_line(),
            self.plan.read_only.context.summary_line(),
            context_rot_risk_summary_line(&self.summary),
            checklist.summary_line(),
            self.plan.replay_report.summary_line(),
            self.plan.evolution_ledger.summary_line(),
            hygiene_work_plan.summary_line(),
            hygiene_work_plan.work_queue().summary_line(),
            self.plan.infini.summary_line(),
            self.plan.retention.summary_line(),
            self.plan.compaction.summary_line(),
            self.plan.inspection.summary_line(),
            self.plan.projection_parity_audit.summary_line(),
            self.plan.migration_readiness.summary_line(),
            self.migration_evidence.summary_line(),
        ];
        let dispatch_pressure_evidence = MemoryServiceStartupEvidence {
            requires_operator_review: self.requires_operator_review(),
            approved_phases: self.approved_phases(),
            lines: lines.clone(),
        };
        lines.push(
            dispatch_pressure_evidence
                .hygiene_dispatch_pressure_summary()
                .summary_line(),
        );
        lines.extend(
            hygiene_work_plan
                .work_items()
                .iter()
                .map(MemoryHygieneWorkItem::summary_line),
        );
        lines.extend(self.plan.readiness.adapter_summary_lines());
        lines.extend(
            self.plan
                .adapter_snapshots
                .iter()
                .map(AdapterSnapshotSummary::summary_line),
        );
        lines.extend(
            self.plan
                .clean_gist_selection_reports
                .iter()
                .map(CleanGistSelectionReport::summary_line),
        );
        lines.extend(self.plan.readiness.coverage_summary_lines());
        lines.extend(
            checklist
                .items
                .iter()
                .map(MemoryServiceChecklistItem::summary_line),
        );
        lines.extend(self.plan.read_only.kvswap.summary_lines());
        if let Some(snapshot) = &self.plan.kvswap_state {
            lines.push(snapshot.summary_line());
        }
        if let Some(boundary) = &self.plan.kvswap_boundary {
            lines.push(boundary.summary_line());
            lines.push(boundary.readiness().summary_line());
        }
        if let Some(bundle_summary) = &self.plan.projection_bundle_summary {
            lines.push(bundle_summary.summary_line());
        }
        if let Some(bundle_manifest) = &self.plan.projection_contract_bundle_manifest {
            lines.push(bundle_manifest.clone());
        }
        lines.extend(self.plan.projection_contract_manifests.iter().cloned());
        lines.extend(
            self.plan
                .projection_coverage
                .iter()
                .map(AdapterProjectionCoverageReport::summary_line),
        );
        lines.extend(
            self.approvals
                .iter()
                .map(MemoryMigrationApproval::summary_line),
        );

        MemoryServiceStartupEvidence {
            requires_operator_review: self.requires_operator_review(),
            approved_phases: self.approved_phases(),
            lines,
        }
    }
}

impl MemoryServiceShadowPlan {
    pub fn for_inputs(inputs: MemoryServiceShadowPlanInputs<'_>) -> Self {
        let request_scope_missing = inputs.scope.is_none();
        let manifest = MemoryServiceManifest::new(inputs.adapters.to_vec());
        let requirement = inputs.requirement;
        let readiness = manifest.readiness(&requirement);
        let projection_coverage = inputs
            .projection_contracts
            .iter()
            .map(|contract| contract.coverage_report(inputs.projection_contract_target))
            .collect::<Vec<_>>();
        let projection_contract_manifests = inputs
            .projection_contracts
            .iter()
            .map(|contract| contract.manifest_line(inputs.projection_contract_target))
            .collect::<Vec<_>>();
        let projection_contract_bundle_manifest =
            inputs.projection_contract_bundle_name.as_ref().map(|name| {
                AdapterProjectionContractBundle::new(
                    name.clone(),
                    inputs.projection_contract_target,
                    inputs.projection_contracts.to_vec(),
                )
                .manifest_summary_line()
            });
        let projection_bundle_summary =
            inputs.projection_contract_bundle_name.as_ref().map(|name| {
                AdapterProjectionBundleReport::from_reports(
                    name.clone(),
                    inputs.projection_contract_target,
                    projection_coverage.clone(),
                )
            });
        let projection_audit =
            DefaultAdapterProjectionAuditor::new().audit(inputs.experiences, inputs.kv_metadata);
        let read_only = ReadOnlyMemoryPlan::for_inputs(
            inputs.adapter_name,
            inputs.experiences,
            inputs.kv_metadata,
            inputs.scope,
            inputs.tier_budgets,
            inputs.previous_placement,
            inputs.target_hot_bytes,
        );
        let migration_readiness = MigrationReadinessReport::from_plan(&read_only);
        let replay = crate::DefaultExperienceReplayPlanner::default().plan(
            inputs.replay_candidates,
            inputs.scope,
            inputs.replay_limit,
        );
        let replay_report = ReplayReport::from_plan(&replay);
        let infini = DefaultInfiniMemoryPlanner::default()
            .plan(inputs.memory_entries, inputs.active_matches);
        let retention = DefaultMemoryRetentionPlanner.plan_retention(
            inputs.memory_entries,
            inputs.now,
            inputs.retention_policy,
        );
        let compaction = DefaultMemoryRetentionPlanner.plan_compaction(
            inputs.memory_entries,
            &inputs.protected_memory_ids,
            inputs.now,
            inputs.compaction_policy,
        );
        let mut evolution_ledger = inputs.seed_evolution_ledger.unwrap_or_default();
        evolution_ledger.record_replay_report(&replay_report);
        evolution_ledger.record_retention_plan(&retention);
        evolution_ledger.record_compaction_plan(&compaction);
        evolution_ledger.context_rot_items = evolution_ledger
            .context_rot_items
            .saturating_add(read_only.context.rejected_ids().len() as u64)
            .saturating_add(read_only.governance.context_rot_risks.len() as u64);
        evolution_ledger.record_index_quality_gate(&read_only.quality_gate);
        if let Some(boundary) = &inputs.kvswap_boundary {
            evolution_ledger.record_kvswap_boundary_readiness(&boundary.readiness());
        }
        let evolution_assessment = DefaultMemoryEvolutionGate::default().assess(&evolution_ledger);
        let inspection = DefaultMemoryInspectionBuilder.build(
            inputs.memory_entries,
            inputs.experiences.len(),
            inputs.kv_metadata.len(),
            inputs.adapters,
            Some(&projection_audit),
            Some(&readiness),
            Some(&evolution_ledger),
            Some(&evolution_assessment),
            inputs.inspection_limit,
        );
        let mut projection_parity_audit = MemoryProjectionAudit::default();
        if let Some(projection) = inputs.adaptive_state_projection {
            projection_parity_audit.merge(projection.audit_ledger(&evolution_ledger));
        }
        if let Some(projection) = inputs.state_inspection_projection {
            projection_parity_audit.merge(projection.audit_snapshot(&inspection));
        }

        Self {
            requirement,
            readiness,
            adapter_snapshots: inputs.adapter_snapshots.to_vec(),
            clean_gist_selection_reports: inputs.clean_gist_selection_reports.to_vec(),
            projection_contract_bundle_manifest,
            projection_contract_manifests,
            projection_coverage,
            projection_bundle_summary,
            projection_audit,
            read_only,
            request_scope_missing,
            migration_readiness,
            replay,
            replay_report,
            infini,
            retention,
            compaction,
            kvswap_state: inputs.kvswap_state,
            kvswap_boundary: inputs.kvswap_boundary,
            evolution_ledger,
            evolution_assessment,
            inspection,
            projection_parity_audit,
        }
    }

    pub fn requires_operator_review(&self) -> bool {
        self.readiness.requires_operator_review()
            || self.request_scope_missing
            || self
                .projection_coverage
                .iter()
                .any(AdapterProjectionCoverageReport::requires_operator_review)
            || !self.projection_audit.is_ready_for_isolated_write()
            || self
                .adapter_snapshots
                .iter()
                .any(|snapshot| !snapshot.warnings.is_empty())
            || self.read_only.requires_operator_review()
            || self.migration_readiness.operator_review_required
            || self
                .kvswap_boundary
                .as_ref()
                .is_some_and(|audit| !audit.is_clean())
            || self.evolution_assessment.requires_operator_review()
            || self.inspection.has_blockers()
            || self.projection_parity_audit.requires_operator_review()
    }

    pub fn migration_approval(
        &self,
        phase: MemoryMigrationPhase,
        evidence: &MemoryMigrationEvidence,
        operator_ack: bool,
    ) -> MemoryMigrationApproval {
        DefaultMemoryMigrationGate::new().evaluate(phase, self, evidence, operator_ack)
    }

    pub fn summary(&self) -> MemoryServiceShadowSummary {
        MemoryServiceShadowSummary::from_plan(self)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MemoryServiceManifest {
    pub adapters: Vec<MemoryAdapterStatus>,
}

impl MemoryServiceManifest {
    pub fn new(adapters: Vec<MemoryAdapterStatus>) -> Self {
        Self { adapters }
    }

    pub fn push(&mut self, status: MemoryAdapterStatus) {
        self.adapters.push(status);
    }

    pub fn covered_capabilities(&self) -> Vec<MemoryAdapterCapability> {
        sorted_unique_capabilities(
            self.adapters
                .iter()
                .flat_map(|status| status.descriptor.capabilities.iter().copied())
                .collect(),
        )
    }

    pub fn coverage_for(
        &self,
        capability: MemoryAdapterCapability,
        minimum_write_mode: AdapterWriteMode,
    ) -> MemoryCapabilityCoverage {
        let mut providers = Vec::new();
        let mut healthy_providers = Vec::new();
        let mut writable_providers = Vec::new();
        let mut read_only_providers = Vec::new();
        let mut record_count = Some(0_usize);

        for status in &self.adapters {
            if !status.descriptor.capabilities.contains(&capability) {
                continue;
            }
            providers.push(status.descriptor.name.clone());
            if status.write_mode == AdapterWriteMode::ReadOnly {
                read_only_providers.push(status.descriptor.name.clone());
            }
            if let Some(count) = status.health.record_count {
                if let Some(total) = &mut record_count {
                    *total = total.saturating_add(count);
                }
            } else {
                record_count = None;
            }
            if !status.health.ready {
                continue;
            }
            healthy_providers.push(status.descriptor.name.clone());
            if write_mode_satisfies(status.write_mode, minimum_write_mode) {
                writable_providers.push(status.descriptor.name.clone());
            }
        }

        MemoryCapabilityCoverage {
            capability,
            providers,
            healthy_providers,
            writable_providers,
            read_only_providers,
            record_count,
        }
    }

    pub fn readiness(&self, requirement: &MemoryServiceRequirement) -> MemoryReadinessReport {
        let capabilities = sorted_unique_capabilities(requirement.capabilities.clone());
        let mut coverage = Vec::with_capacity(capabilities.len());
        let mut missing_capabilities = Vec::new();
        let mut write_mode_blockers = Vec::new();

        for capability in capabilities {
            let item = self.coverage_for(capability, requirement.minimum_write_mode);
            if !item.has_healthy_provider() {
                missing_capabilities.push(capability);
            } else if !item.has_writable_provider() {
                write_mode_blockers.push(capability);
            }
            coverage.push(item);
        }

        let unhealthy_adapters = self
            .adapters
            .iter()
            .filter(|status| !status.health.ready)
            .map(|status| status.descriptor.name.clone())
            .collect::<Vec<_>>();
        let mut warnings = Vec::new();
        for status in &self.adapters {
            warnings.extend(
                status
                    .health
                    .warnings
                    .iter()
                    .map(|warning| format!("{}:{warning}", status.descriptor.name)),
            );
        }
        for item in &coverage {
            if item.providers.len() > 1 {
                warnings.push(format!(
                    "capability:{} has multiple providers: {}",
                    item.capability.as_str(),
                    item.providers.join(",")
                ));
            }
        }
        warnings.sort();
        warnings.dedup();

        MemoryReadinessReport {
            profile: requirement.profile,
            required_write_mode: requirement.minimum_write_mode,
            ready: missing_capabilities.is_empty() && write_mode_blockers.is_empty(),
            adapter_statuses: self.adapters.clone(),
            missing_capabilities,
            write_mode_blockers,
            unhealthy_adapters,
            warnings,
            coverage,
        }
    }
}

fn sorted_unique_capabilities(
    mut capabilities: Vec<MemoryAdapterCapability>,
) -> Vec<MemoryAdapterCapability> {
    capabilities.sort();
    capabilities.dedup();
    capabilities
}

fn write_mode_satisfies(provider: AdapterWriteMode, required: AdapterWriteMode) -> bool {
    match required {
        AdapterWriteMode::ReadOnly => true,
        AdapterWriteMode::IsolatedWrite => {
            matches!(
                provider,
                AdapterWriteMode::IsolatedWrite | AdapterWriteMode::LiveWrite
            )
        }
        AdapterWriteMode::LiveWrite => provider == AdapterWriteMode::LiveWrite,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AdaptiveStateMemoryProjection, DefaultCleanGistSelector, DiskKvCatalogVerification,
        ExperienceEnvelope, GistLevel, InfiniMemoryActiveMatch, KvEvictionPlan, KvPrefetchPlan,
        KvShardMetadata, KvTier, LongTermMemory, MemoryDocumentInput, MemoryGist,
        MemoryProjectionMismatch, MemoryScope, ReplaySignal, RetentionMemoryEntry, ShortTermKv,
        SkillLibrary, SkillRecordInput, StateInspectionMemoryProjection, in_memory_ports,
    };

    #[test]
    fn hygiene_dispatch_pressure_summary_clean_priority_when_empty() {
        let summary = MemoryHygieneDispatchPressureSummary {
            pressure_score: 0,
            queue_items: 0,
            operator_review_items: 0,
            isolation_items: 0,
            kvswap_boundary_repair_lanes: 0,
            context_rot_review_lanes: 0,
            experience_index_rebuild_lanes: 0,
            quarantine_priorities: 0,
            repair_priorities: 0,
            context_rot_risks: 0,
            missing_clean_gist_pressure: 0,
            kvswap_boundary_blockers: 0,
            kvswap_boundary_warnings: 0,
        };

        assert!(!summary.has_pressure());
        assert!(!summary.requires_operator_review());
        assert!(!summary.requires_isolation());
        assert_eq!(summary.priority_code(), "clean");
        assert_eq!(summary.dispatch_rank(), 0);
        assert_eq!(summary.reason_codes(), Vec::<String>::new());
        assert_eq!(
            summary.summary_line(),
            "memory_hygiene_dispatch_pressure rank=0 priority=clean pressure_score=0 queue_items=0 operator_review_items=0 isolation_items=0 context_rot_risks=0 missing_clean_gist_pressure=0 kvswap_boundary_blockers=0 kvswap_boundary_warnings=0 reason_codes=none"
        );

        let repair_summary = MemoryHygieneDispatchPressureSummary {
            kvswap_boundary_warnings: 1,
            ..summary
        };
        assert!(repair_summary.has_pressure());
        assert!(!repair_summary.requires_operator_review());
        assert!(!repair_summary.requires_isolation());
        assert_eq!(repair_summary.priority_code(), "repair");
        assert_eq!(repair_summary.dispatch_rank(), 1);
        assert_eq!(
            repair_summary.reason_codes(),
            vec!["kvswap_boundary_warning".to_owned()]
        );
        assert_eq!(
            repair_summary.summary_line(),
            "memory_hygiene_dispatch_pressure rank=1 priority=repair pressure_score=0 queue_items=0 operator_review_items=0 isolation_items=0 context_rot_risks=0 missing_clean_gist_pressure=0 kvswap_boundary_blockers=0 kvswap_boundary_warnings=1 reason_codes=kvswap_boundary_warning"
        );
    }

    fn status(
        name: &str,
        capabilities: Vec<MemoryAdapterCapability>,
        ready: bool,
        read_only: bool,
        write_mode: AdapterWriteMode,
    ) -> MemoryAdapterStatus {
        let mut descriptor = MemoryAdapterDescriptor::new(name, capabilities);
        if read_only {
            descriptor = descriptor.read_only();
        }
        MemoryAdapterStatus::new(
            descriptor,
            MemoryAdapterHealth {
                ready,
                record_count: Some(1),
                warnings: Vec::new(),
            },
            write_mode,
        )
    }

    #[test]
    fn service_requirement_summarizes_profile_capabilities_for_startup() {
        let requirement = MemoryServiceRequirement::for_profile(
            MemoryConsumerProfile::Service,
            AdapterWriteMode::IsolatedWrite,
        );

        assert!(
            requirement
                .capability_codes()
                .contains(&"experience_governance".to_owned())
        );
        assert!(
            requirement
                .capability_codes()
                .contains(&"kv_swap".to_owned())
        );
        assert_eq!(
            requirement.summary_line(),
            "memory_service_requirement profile=service minimum_write_mode=isolated_write capabilities=short_term_kv|long_term_memory|skill_library|experience_governance|memory_index|tiered_placement|experience_replay|context_injection|repair_planning|disk_kv_offload|kv_swap|retention_planning|compaction_planning|memory_evolution|state_inspection|infini_memory_planning capability_count=16"
        );

        let custom = MemoryServiceRequirement::for_profile(
            MemoryConsumerProfile::Core,
            AdapterWriteMode::ReadOnly,
        )
        .with_capabilities(vec![
            MemoryAdapterCapability::SkillLibrary,
            MemoryAdapterCapability::ShortTermKv,
            MemoryAdapterCapability::SkillLibrary,
        ]);

        assert_eq!(
            custom.capability_codes(),
            vec!["short_term_kv".to_owned(), "skill_library".to_owned()]
        );
        assert!(custom.summary_line().contains("capability_count=2"));
    }

    #[test]
    fn core_profile_is_satisfied_by_agentic_memory_ports() {
        let mut ports = in_memory_ports();
        ports
            .short_term
            .put("focus".to_owned(), b"task".to_vec(), crate::Metadata::new())
            .unwrap();
        ports
            .long_term
            .remember(MemoryDocumentInput::new("lesson", vec![1.0]))
            .unwrap();
        ports
            .skills
            .add_skill(SkillRecordInput::new("skill", "do work"))
            .unwrap();

        let manifest = MemoryServiceManifest::new(vec![
            MemoryAdapterStatus::inspect(&ports, AdapterWriteMode::IsolatedWrite).unwrap(),
        ]);
        let report = manifest.readiness(&MemoryServiceRequirement::for_profile(
            MemoryConsumerProfile::Core,
            AdapterWriteMode::IsolatedWrite,
        ));

        assert!(report.ready);
        assert!(report.missing_capabilities.is_empty());
        assert_eq!(
            report.capability_manifest_checklist_detail(),
            "missing=0 write_blockers=0"
        );
        assert_eq!(report.coverage.len(), 3);
        assert_eq!(
            report.summary_line(),
            "memory_readiness profile=core required_write_mode=isolated_write ready=true review=false missing=0 write_blockers=0 unhealthy=0 warnings=0 missing_codes=none write_blocker_codes=none warning_codes=none"
        );
    }

    #[test]
    fn agent_profile_reports_missing_policy_layers() {
        let ports = in_memory_ports();
        let manifest = MemoryServiceManifest::new(vec![
            MemoryAdapterStatus::inspect(&ports, AdapterWriteMode::IsolatedWrite).unwrap(),
        ]);
        let report = manifest.readiness(&MemoryServiceRequirement::for_profile(
            MemoryConsumerProfile::Agent,
            AdapterWriteMode::ReadOnly,
        ));

        assert!(!report.ready);
        assert_eq!(
            report.capability_manifest_checklist_detail(),
            "missing=4 write_blockers=0"
        );
        assert!(
            report
                .missing_capabilities
                .contains(&MemoryAdapterCapability::ExperienceGovernance)
        );
        assert!(
            report
                .missing_capabilities
                .contains(&MemoryAdapterCapability::ContextInjection)
        );
        assert!(
            report
                .missing_capability_codes()
                .contains(&"context_injection".to_owned())
        );
        assert!(
            report
                .summary_line()
                .contains("profile=agent required_write_mode=read_only ready=false")
        );
    }

    #[test]
    fn explicit_clean_gist_requirement_is_satisfied_by_selector_adapter() {
        let selector = DefaultCleanGistSelector::new();
        let manifest = MemoryServiceManifest::new(vec![
            MemoryAdapterStatus::inspect(&selector, AdapterWriteMode::ReadOnly).unwrap(),
        ]);
        let requirement = MemoryServiceRequirement::for_profile(
            MemoryConsumerProfile::ShadowMigration,
            AdapterWriteMode::ReadOnly,
        )
        .with_capabilities(vec![MemoryAdapterCapability::CleanGistSelection]);

        let report = manifest.readiness(&requirement);

        assert!(report.ready);
        assert_eq!(report.coverage.len(), 1);
        assert_eq!(
            report.coverage[0].capability,
            MemoryAdapterCapability::CleanGistSelection
        );
        assert_eq!(
            report.coverage[0].providers,
            vec!["default_clean_gist_selector".to_owned()]
        );
        assert_eq!(
            report.coverage_summary_lines(),
            vec![
                "memory_capability_coverage capability=clean_gist_selection providers=default_clean_gist_selector healthy=default_clean_gist_selector writable=default_clean_gist_selector read_only=default_clean_gist_selector records=unknown status_codes=none".to_owned()
            ]
        );
    }

    #[test]
    fn service_profile_requires_experience_replay_provider() {
        let ports = in_memory_ports();
        let manifest = MemoryServiceManifest::new(vec![
            MemoryAdapterStatus::inspect(&ports, AdapterWriteMode::IsolatedWrite).unwrap(),
            status(
                "policy_without_replay",
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
                false,
                AdapterWriteMode::IsolatedWrite,
            ),
        ]);

        let report = manifest.readiness(&MemoryServiceRequirement::for_profile(
            MemoryConsumerProfile::Service,
            AdapterWriteMode::IsolatedWrite,
        ));

        assert!(!report.ready);
        assert_eq!(
            report.missing_capabilities,
            vec![MemoryAdapterCapability::ExperienceReplay]
        );
        assert!(report.write_mode_blockers.is_empty());
        assert_eq!(
            report.missing_capability_codes(),
            vec!["experience_replay".to_owned()]
        );
    }

    #[test]
    fn shadow_migration_accepts_read_only_policy_and_kv_adapters() {
        let manifest = MemoryServiceManifest::new(vec![
            status(
                "experience_shadow",
                vec![
                    MemoryAdapterCapability::ExperienceGovernance,
                    MemoryAdapterCapability::MemoryIndex,
                    MemoryAdapterCapability::ContextInjection,
                    MemoryAdapterCapability::RepairPlanning,
                    MemoryAdapterCapability::InfiniMemoryPlanning,
                    MemoryAdapterCapability::RetentionPlanning,
                    MemoryAdapterCapability::CompactionPlanning,
                    MemoryAdapterCapability::MemoryEvolution,
                    MemoryAdapterCapability::StateInspection,
                ],
                true,
                true,
                AdapterWriteMode::ReadOnly,
            ),
            status(
                "kv_shadow",
                vec![
                    MemoryAdapterCapability::TieredPlacement,
                    MemoryAdapterCapability::DiskKvOffload,
                    MemoryAdapterCapability::KvSwap,
                ],
                true,
                true,
                AdapterWriteMode::ReadOnly,
            ),
        ]);
        let report = manifest.readiness(&MemoryServiceRequirement::for_profile(
            MemoryConsumerProfile::ShadowMigration,
            AdapterWriteMode::ReadOnly,
        ));

        assert!(report.ready);
        assert!(report.write_mode_blockers.is_empty());
        assert!(report.coverage.iter().any(|item| item.capability
            == MemoryAdapterCapability::DiskKvOffload
            && item.read_only_providers == vec!["kv_shadow".to_owned()]));
    }

    #[test]
    fn service_isolated_write_blocks_read_only_only_capabilities() {
        let manifest = MemoryServiceManifest::new(vec![
            status(
                "ports",
                vec![
                    MemoryAdapterCapability::ShortTermKv,
                    MemoryAdapterCapability::LongTermMemory,
                    MemoryAdapterCapability::SkillLibrary,
                ],
                true,
                false,
                AdapterWriteMode::IsolatedWrite,
            ),
            status(
                "policy_shadow",
                vec![
                    MemoryAdapterCapability::ExperienceGovernance,
                    MemoryAdapterCapability::MemoryIndex,
                    MemoryAdapterCapability::ContextInjection,
                    MemoryAdapterCapability::RepairPlanning,
                    MemoryAdapterCapability::TieredPlacement,
                    MemoryAdapterCapability::InfiniMemoryPlanning,
                    MemoryAdapterCapability::ExperienceReplay,
                    MemoryAdapterCapability::RetentionPlanning,
                    MemoryAdapterCapability::CompactionPlanning,
                    MemoryAdapterCapability::MemoryEvolution,
                    MemoryAdapterCapability::StateInspection,
                    MemoryAdapterCapability::DiskKvOffload,
                    MemoryAdapterCapability::KvSwap,
                ],
                true,
                true,
                AdapterWriteMode::ReadOnly,
            ),
        ]);
        let report = manifest.readiness(&MemoryServiceRequirement::for_profile(
            MemoryConsumerProfile::Service,
            AdapterWriteMode::IsolatedWrite,
        ));

        assert!(!report.ready);
        assert!(report.missing_capabilities.is_empty());
        assert!(
            report
                .write_mode_blockers
                .contains(&MemoryAdapterCapability::DiskKvOffload)
        );
        assert!(
            report
                .write_mode_blockers
                .contains(&MemoryAdapterCapability::ExperienceGovernance)
        );
        assert!(
            report
                .write_mode_blocker_codes()
                .contains(&"disk_kv_offload".to_owned())
        );
        let disk_coverage = report
            .coverage
            .iter()
            .find(|item| item.capability == MemoryAdapterCapability::DiskKvOffload)
            .unwrap();
        assert_eq!(
            disk_coverage.status_codes(),
            vec!["write_mode_blocked".to_owned()]
        );
        assert_eq!(
            disk_coverage.summary_line(),
            "memory_capability_coverage capability=disk_kv_offload providers=policy_shadow healthy=policy_shadow writable=none read_only=policy_shadow records=1 status_codes=write_mode_blocked"
        );
        assert!(report.summary_line().contains("write_blocker_codes="));
    }

    #[test]
    fn shadow_plan_inputs_can_override_requirement_for_service_preflight() {
        let adapters = vec![status(
            "service_shadow_all",
            MemoryConsumerProfile::Service
                .required_capabilities()
                .to_vec(),
            true,
            true,
            AdapterWriteMode::ReadOnly,
        )];
        let requirement = MemoryServiceRequirement::for_profile(
            MemoryConsumerProfile::Service,
            AdapterWriteMode::IsolatedWrite,
        );

        let dry_run = MemoryServiceDryRun::for_inputs(
            MemoryServiceShadowPlanInputs::new("service_preflight", &[], &[], &[])
                .with_adapters(&adapters)
                .with_requirement(requirement.clone()),
            &MemoryMigrationEvidence::read_only_source(Some(0)),
            &[],
            false,
        );
        let evidence = dry_run.startup_evidence();
        let text = evidence.summary_text();

        assert_eq!(dry_run.plan.requirement, requirement);
        assert_eq!(
            dry_run.plan.readiness.profile,
            MemoryConsumerProfile::Service
        );
        assert_eq!(
            dry_run.plan.readiness.required_write_mode,
            AdapterWriteMode::IsolatedWrite
        );
        assert!(dry_run.plan.readiness.missing_capabilities.is_empty());
        assert!(
            dry_run
                .plan
                .readiness
                .write_mode_blockers
                .contains(&MemoryAdapterCapability::KvSwap)
        );
        assert!(text.contains(
            "memory_service_requirement profile=service minimum_write_mode=isolated_write"
        ));
        assert!(text.contains(
            "memory_readiness profile=service required_write_mode=isolated_write ready=false"
        ));
        assert!(evidence.is_complete());
    }

    #[test]
    fn unhealthy_provider_does_not_satisfy_required_capability() {
        let manifest = MemoryServiceManifest::new(vec![status(
            "unhealthy_index",
            vec![MemoryAdapterCapability::MemoryIndex],
            false,
            false,
            AdapterWriteMode::IsolatedWrite,
        )]);
        let requirement = MemoryServiceRequirement::for_profile(
            MemoryConsumerProfile::ShadowMigration,
            AdapterWriteMode::ReadOnly,
        )
        .with_capabilities(vec![MemoryAdapterCapability::MemoryIndex]);
        let report = manifest.readiness(&requirement);

        assert!(!report.ready);
        assert_eq!(
            report.unhealthy_adapters,
            vec!["unhealthy_index".to_owned()]
        );
        assert_eq!(
            report.missing_capabilities,
            vec![MemoryAdapterCapability::MemoryIndex]
        );
        assert_eq!(
            report.adapter_statuses[0].status_codes(),
            vec!["unhealthy".to_owned()]
        );
        assert_eq!(
            report.adapter_statuses[0].summary_line(),
            "memory_adapter_status name=unhealthy_index ready=false read_only=false write_mode=isolated_write capabilities=memory_index records=1 warnings=0 status_codes=unhealthy warning_codes=none"
        );
    }

    #[test]
    fn readiness_report_normalizes_health_and_provider_warning_codes() {
        let mut warning_descriptor =
            MemoryAdapterDescriptor::new("index_a", vec![MemoryAdapterCapability::MemoryIndex]);
        warning_descriptor = warning_descriptor.read_only();
        let manifest = MemoryServiceManifest::new(vec![
            MemoryAdapterStatus::new(
                warning_descriptor,
                MemoryAdapterHealth {
                    ready: true,
                    record_count: Some(4),
                    warnings: vec!["lagging_snapshot".to_owned()],
                },
                AdapterWriteMode::ReadOnly,
            ),
            status(
                "index_b",
                vec![MemoryAdapterCapability::MemoryIndex],
                true,
                true,
                AdapterWriteMode::ReadOnly,
            ),
        ]);
        let requirement = MemoryServiceRequirement::for_profile(
            MemoryConsumerProfile::ShadowMigration,
            AdapterWriteMode::ReadOnly,
        )
        .with_capabilities(vec![MemoryAdapterCapability::MemoryIndex]);

        let report = manifest.readiness(&requirement);

        assert!(report.ready);
        assert!(report.requires_operator_review());
        assert_eq!(report.warnings.len(), 2);
        assert_eq!(
            report.warning_codes(),
            vec![
                "adapter_health_warning".to_owned(),
                "multiple_providers".to_owned(),
            ]
        );
        assert_eq!(report.adapter_statuses.len(), 2);
        assert_eq!(
            report.adapter_statuses[0].capability_codes(),
            vec!["memory_index".to_owned()]
        );
        assert_eq!(
            report.adapter_statuses[0].warning_codes(),
            vec!["lagging_snapshot".to_owned()]
        );
        assert_eq!(
            report.adapter_statuses[0].status_codes(),
            vec!["health_warnings".to_owned(), "read_only".to_owned()]
        );
        assert_eq!(
            report.adapter_statuses[0].summary_line(),
            "memory_adapter_status name=index_a ready=true read_only=true write_mode=read_only capabilities=memory_index records=4 warnings=1 status_codes=health_warnings|read_only warning_codes=lagging_snapshot"
        );
        assert_eq!(
            report.coverage[0].status_codes(),
            vec!["multiple_providers".to_owned()]
        );
        assert_eq!(
            report.coverage_summary_lines(),
            vec![
                "memory_capability_coverage capability=memory_index providers=index_a,index_b healthy=index_a,index_b writable=index_a,index_b read_only=index_a,index_b records=5 status_codes=multiple_providers".to_owned()
            ]
        );
        assert_eq!(
            report.summary_line(),
            "memory_readiness profile=shadow_migration required_write_mode=read_only ready=true review=true missing=0 write_blockers=0 unhealthy=0 warnings=2 missing_codes=none write_blocker_codes=none warning_codes=adapter_health_warning|multiple_providers"
        );
    }

    #[test]
    fn shadow_plan_combines_readiness_projection_infini_and_inspection() {
        let experiences = vec![
            ExperienceEnvelope::new("keep", "runtime prompt", "runtime lesson")
                .with_scope(MemoryScope::for_task("runtime"))
                .with_tags(vec!["adapter:test".to_owned()]),
            ExperienceEnvelope::new(
                "rot",
                "Conversation Transcript:\nUser: ssh -o ConnectTimeout=1\nAssistant: ok",
                "accepted_pattern quality=0.1 max_severity=critical",
            )
            .with_scope(MemoryScope::for_task("ops"))
            .with_tags(vec!["adapter:test".to_owned()]),
        ];
        let kv_metadata = vec![KvShardMetadata {
            id: "kv".to_owned(),
            byte_len: 4,
            checksum: 1,
            tier: KvTier::Cold,
            priority: 0.7,
            last_access: 5,
        }];
        let memory_entries = vec![
            RetentionMemoryEntry::new("global", "semantic durable memory", vec![0.1], 2.4)
                .with_feedback(4, 0)
                .with_access(1, 10),
            RetentionMemoryEntry::new("runtime", "runtime_kv:block", vec![0.2], 1.2)
                .with_feedback(1, 0)
                .with_access(1, 9),
        ];
        let active = vec![InfiniMemoryActiveMatch::new(
            "local",
            "semantic active prompt",
            vec![0.3],
            0.9,
            1.5,
        )];
        let adapters = vec![
            status(
                "policy_shadow",
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
                ],
                true,
                true,
                AdapterWriteMode::ReadOnly,
            ),
            status(
                "kv_shadow",
                vec![
                    MemoryAdapterCapability::DiskKvOffload,
                    MemoryAdapterCapability::KvSwap,
                ],
                true,
                true,
                AdapterWriteMode::ReadOnly,
            ),
        ];
        let seed_ledger = MemoryEvolutionLedger {
            replay_runs: 1,
            replay_items: 2,
            replay_memory_updates: 2,
            ..MemoryEvolutionLedger::default()
        };

        let plan = MemoryServiceShadowPlan::for_inputs(
            MemoryServiceShadowPlanInputs::new(
                "shadow",
                &experiences,
                &kv_metadata,
                &memory_entries,
            )
            .with_active_matches(&active)
            .with_adapters(&adapters)
            .with_scope(&MemoryScope::for_task("runtime"))
            .with_evolution_ledger(seed_ledger)
            .with_inspection_limit(2),
        );

        assert!(plan.readiness.ready);
        assert_eq!(plan.projection_audit.experience_count, 2);
        assert_eq!(plan.read_only.summary.experience_count, 2);
        assert_eq!(plan.infini.local_window.len(), 1);
        assert_eq!(plan.inspection.memory_count, 2);
        assert_eq!(plan.inspection.runtime_kv_memory_count, 1);
        assert!(plan.evolution_ledger.context_rot_items > 0);
        assert!(plan.summary().detail_codes().iter().any(|code| {
            code == "projection_detail:warning:missing_clean_gist_for_risky_record:source_id_hex:726f74"
        }));
        assert!(plan.requires_operator_review());
    }

    #[test]
    fn startup_evidence_carries_read_only_context_rot_detail_codes() {
        let clean_prompt_secret = "SERVICE_CONTEXT_ROT_CLEAN_PROMPT_SECRET_DO_NOT_LOG";
        let clean_lesson_secret = "SERVICE_CONTEXT_ROT_CLEAN_LESSON_SECRET_DO_NOT_LOG";
        let clean_gist_secret = "SERVICE_CONTEXT_ROT_CLEAN_GIST_SECRET_DO_NOT_LOG";
        let polluted_prompt_secret = "SERVICE_CONTEXT_ROT_POLLUTED_PROMPT_SECRET_DO_NOT_LOG Conversation Transcript:\nUser: ssh -o ConnectTimeout=1\nAssistant: ok";
        let polluted_lesson_secret = "SERVICE_CONTEXT_ROT_POLLUTED_LESSON_SECRET_DO_NOT_LOG accepted_pattern quality=0.1 max_severity=critical";
        let experiences = vec![
            ExperienceEnvelope::new("clean", clean_prompt_secret, clean_lesson_secret)
                .with_scope(MemoryScope::for_task("runtime"))
                .with_clean_gist(clean_gist_secret)
                .with_quality(0.9)
                .with_tags(vec!["adapter:test".to_owned()]),
            ExperienceEnvelope::new("polluted", polluted_prompt_secret, polluted_lesson_secret)
                .with_scope(MemoryScope::for_task("ops"))
                .with_tags(vec!["adapter:test".to_owned()]),
        ];
        let memory_entries = vec![
            RetentionMemoryEntry::new("mem", "semantic durable memory", vec![0.1], 2.4)
                .with_feedback(4, 0)
                .with_access(1, 10),
        ];
        let adapters = vec![status(
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
            true,
            AdapterWriteMode::ReadOnly,
        )];
        let seed_ledger = MemoryEvolutionLedger {
            replay_runs: 1,
            replay_items: 2,
            replay_memory_updates: 2,
            ..MemoryEvolutionLedger::default()
        };

        let dry_run = MemoryServiceDryRun::for_inputs(
            MemoryServiceShadowPlanInputs::new("shadow", &experiences, &[], &memory_entries)
                .with_adapters(&adapters)
                .with_scope(&MemoryScope::for_task("runtime"))
                .with_evolution_ledger(seed_ledger),
            &MemoryMigrationEvidence::read_only_source(Some(2)),
            &[MemoryMigrationPhase::ReadOnlyShadow],
            false,
        );
        let evidence = dry_run.startup_evidence();
        let detail_codes = evidence.detail_codes();
        let shadow_summary = dry_run.summary.summary_line();
        let evidence_summary = evidence.summary_line();
        let evidence_text = evidence.summary_text();

        assert!(dry_run.requires_operator_review());
        assert_eq!(dry_run.summary.context_rot_risk_count, 1);
        assert!(
            dry_run
                .summary
                .context_rot_risk_reason_codes
                .contains(&"cross_task_transcript_pollution".to_owned())
        );
        assert!(
            dry_run
                .summary
                .context_rot_risk_detail_codes
                .contains(&"context_rot:polluted:cross_task_transcript_pollution".to_owned())
        );
        assert_eq!(
            dry_run
                .summary
                .context_rot_risk_detail_codes_for_reason("cross_task_transcript_pollution"),
            vec!["context_rot:polluted:cross_task_transcript_pollution".to_owned()]
        );
        assert_eq!(
            dry_run
                .summary
                .context_rot_risk_reason_count("cross_task_transcript_pollution"),
            1
        );
        assert_eq!(
            dry_run
                .summary
                .context_rot_risk_detail_codes_for_reason("missing_clean_gist"),
            vec!["context_rot:polluted:missing_clean_gist".to_owned()]
        );
        assert_eq!(
            dry_run
                .summary
                .context_rot_risk_reason_count("missing_clean_gist"),
            1
        );
        let risk_reason_codes = dry_run
            .plan
            .read_only
            .governance
            .context_rot_risks
            .iter()
            .flat_map(|risk| risk.reason_codes())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let risk_detail_codes = dry_run
            .plan
            .read_only
            .governance
            .context_rot_risks
            .iter()
            .flat_map(|risk| risk.detail_codes())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();

        assert_eq!(
            dry_run.summary.context_rot_risk_reason_codes,
            risk_reason_codes
        );
        assert_eq!(
            dry_run.summary.context_rot_risk_detail_codes,
            risk_detail_codes
        );
        assert_eq!(
            dry_run.summary.clean_gist_repair_missing_clean_gist_count,
            dry_run.plan.read_only.rebuild.missing_clean_gist_ids.len()
        );
        assert_eq!(
            dry_run.summary.clean_gist_repair_dirty_clean_gist_count,
            dry_run.plan.read_only.rebuild.dirty_clean_gist_ids.len()
        );
        assert_eq!(
            dry_run.summary.clean_gist_repair_dirty_gist_count,
            dry_run.plan.read_only.rebuild.dirty_gist_ids.len()
        );
        assert_eq!(
            dry_run.summary.clean_gist_repair_detail_codes,
            dry_run
                .plan
                .read_only
                .rebuild
                .clean_gist_repair_detail_codes()
        );
        assert_eq!(
            dry_run.summary.clean_gist_repair_issue_count(),
            dry_run.plan.read_only.rebuild.missing_clean_gist_ids.len()
                + dry_run.plan.read_only.rebuild.dirty_clean_gist_ids.len()
                + dry_run.plan.read_only.rebuild.dirty_gist_ids.len()
        );
        assert_eq!(
            dry_run.summary.clean_gist_repair_is_clean(),
            dry_run
                .plan
                .read_only
                .rebuild
                .clean_gist_repair_detail_codes()
                .is_empty()
        );
        assert!(
            dry_run
                .plan
                .read_only
                .governance
                .context_rot_risks
                .iter()
                .any(|risk| risk.summary_line().contains("context_rot_risk "))
        );
        assert!(shadow_summary.contains("context_rot_risks=1"));
        assert!(
            shadow_summary
                .contains("context_rot_risk_reason_codes=cross_task_transcript_pollution")
        );
        assert!(shadow_summary.contains("context_rot_risk_detail_codes=context_rot:polluted:"));
        assert!(shadow_summary.contains("clean_gist_repair_missing_clean_gist="));
        assert!(shadow_summary.contains("clean_gist_repair_detail_codes="));
        assert!(
            dry_run
                .summary
                .clean_gist_repair_detail_codes
                .iter()
                .any(|code| code.starts_with("missing_clean_gist:"))
        );
        assert!(
            dry_run.summary.detail_codes().contains(
                &"read_only_detail:semantic:skip:experience:cross_task_scope:706f6c6c75746564"
                    .to_owned()
            )
        );
        assert!(
            detail_codes
                .contains(&"context_rot:polluted:cross_task_transcript_pollution".to_owned())
        );
        assert!(
            detail_codes.contains(
                &"read_only_detail:semantic:skip:experience:cross_task_scope:706f6c6c75746564"
                    .to_owned()
            )
        );
        assert!(
            detail_codes.contains(
                &"read_only_detail:governance:context_rot:polluted:cross_task_transcript_pollution"
                    .to_owned()
            )
        );
        assert!(
            evidence_summary.contains("read_only_detail:semantic:skip:experience:cross_task_scope")
        );
        assert!(
            evidence_text
                .contains("memory_read_only_plan adapter=shadow write_mode=read_only review=true")
        );
        assert!(evidence_text.contains("detail_codes="));
        for forbidden in [
            clean_prompt_secret,
            clean_lesson_secret,
            clean_gist_secret,
            polluted_prompt_secret,
            polluted_lesson_secret,
        ] {
            assert!(
                !shadow_summary.contains(forbidden),
                "memory shadow summary leaked context rot payload: {forbidden}"
            );
            assert!(
                !evidence_summary.contains(forbidden),
                "startup evidence summary leaked context rot payload: {forbidden}"
            );
            assert!(
                !evidence_text.contains(forbidden),
                "startup evidence text leaked context rot payload: {forbidden}"
            );
            assert!(
                !detail_codes.iter().any(|code| code.contains(forbidden)),
                "startup detail codes leaked context rot payload: {forbidden}"
            );
        }
    }

    #[test]
    fn dry_run_startup_evidence_exposes_structured_review_fields() {
        let experiences = vec![
            ExperienceEnvelope::new("clean", "runtime prompt", "Stable runtime lesson")
                .with_scope(MemoryScope::for_task("runtime"))
                .with_clean_gist("A stable runtime summary with useful detail.")
                .with_quality(0.9)
                .with_tags(vec!["adapter:test".to_owned()]),
            ExperienceEnvelope::new(
                "polluted",
                "Conversation Transcript:\nUser: ssh -o ConnectTimeout=1\nAssistant: ok",
                "accepted_pattern quality=0.1 max_severity=critical",
            )
            .with_scope(MemoryScope::for_task("ops"))
            .with_tags(vec!["adapter:test".to_owned()]),
        ];
        let memory_entries = vec![
            RetentionMemoryEntry::new("mem", "semantic durable memory", vec![0.1], 2.4)
                .with_feedback(4, 0)
                .with_access(1, 10),
        ];
        let adapters = vec![status(
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
            true,
            AdapterWriteMode::ReadOnly,
        )];
        let seed_ledger = MemoryEvolutionLedger {
            replay_runs: 1,
            replay_items: 2,
            replay_memory_updates: 2,
            ..MemoryEvolutionLedger::default()
        };
        let boundary = KvSwapBoundaryAudit {
            overlapping_hot_cold_ids: vec!["hot/cold".to_owned()],
            ..KvSwapBoundaryAudit::default()
        };

        let dry_run = MemoryServiceDryRun::for_inputs(
            MemoryServiceShadowPlanInputs::new("shadow", &experiences, &[], &memory_entries)
                .with_adapters(&adapters)
                .with_scope(&MemoryScope::for_task("runtime"))
                .with_evolution_ledger(seed_ledger)
                .with_kvswap_boundary(boundary),
            &MemoryMigrationEvidence::read_only_source(Some(2)),
            &[MemoryMigrationPhase::IsolatedWrite],
            false,
        );
        let evidence = dry_run.startup_evidence();
        let summary_line = evidence.summary_line();

        assert!(dry_run.requires_operator_review());
        assert_eq!(evidence.context_rot_risk_count(), 1);
        assert_eq!(
            evidence.context_rot_risk_reason_codes(),
            vec![
                "cross_task_transcript_pollution".to_owned(),
                "missing_clean_gist".to_owned(),
                "transcript_anchor_risk".to_owned(),
            ]
        );
        assert!(
            evidence
                .context_rot_risk_detail_codes()
                .contains(&"context_rot:polluted:cross_task_transcript_pollution".to_owned())
        );
        assert_eq!(evidence.kvswap_boundary_issue_count(), 1);
        assert_eq!(
            evidence.kvswap_boundary_reason_codes(),
            vec!["overlapping_hot_cold".to_owned()]
        );
        assert_eq!(
            evidence.kvswap_boundary_detail_codes(),
            vec!["overlap:686f742f636f6c64".to_owned()]
        );
        assert!(
            evidence
                .migration_evidence_guard_codes()
                .contains(&"copied_fixture_missing".to_owned())
        );
        assert!(
            evidence
                .migration_evidence_guard_codes()
                .contains(&"isolated_write_root_missing".to_owned())
        );
        assert!(
            evidence
                .migration_evidence_detail_codes()
                .contains(&"guard:copied_fixture_missing".to_owned())
        );
        assert!(summary_line.contains("context_rot_risks=1"));
        assert!(
            summary_line.contains("migration_guard_codes=copied_fixture_missing")
                || summary_line.contains("migration_guard_codes=fixture_catalog_not_verified")
        );
        assert!(summary_line.contains("kvswap_boundary_detail_codes=overlap:686f742f636f6c64"));
    }

    #[test]
    fn raw_fallback_missing_clean_gist_rejects_risk_but_preserves_rot_evidence() {
        let raw_prompt =
            "Conversation Transcript:\nUser: cargo test\nAssistant: copied raw fallback";
        let raw_lesson = "accepted_pattern quality=0.1 max_severity=critical";
        let replay_lesson = "REPLAY_RAW_FALLBACK_PAYLOAD_DO_NOT_LOG";
        let experiences = vec![
            ExperienceEnvelope::new("clean", "runtime prompt", "Stable runtime lesson")
                .with_scope(MemoryScope::for_task("runtime"))
                .with_clean_gist("A stable runtime summary with useful detail.")
                .with_quality(0.9)
                .with_tags(vec!["adapter:test".to_owned()]),
            ExperienceEnvelope::new("raw", raw_prompt, raw_lesson)
                .with_scope(MemoryScope::for_task("runtime"))
                .with_quality(0.8)
                .with_tags(vec!["adapter:test".to_owned()]),
        ];
        let replay_candidates = vec![
            ReplayCandidate::new("raw", replay_lesson, 0.1)
                .with_scope(MemoryScope::for_task("runtime"))
                .with_signals(vec![ReplaySignal::ContextRot]),
        ];
        let memory_entries = vec![
            RetentionMemoryEntry::new("mem", "semantic durable memory", vec![0.1], 2.4)
                .with_feedback(4, 0)
                .with_access(1, 10),
        ];
        let adapters = vec![status(
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
            true,
            AdapterWriteMode::ReadOnly,
        )];
        let seed_ledger = MemoryEvolutionLedger {
            replay_runs: 1,
            replay_items: 2,
            replay_memory_updates: 2,
            ..MemoryEvolutionLedger::default()
        };

        let dry_run = MemoryServiceDryRun::for_inputs(
            MemoryServiceShadowPlanInputs::new("shadow", &experiences, &[], &memory_entries)
                .with_adapters(&adapters)
                .with_scope(&MemoryScope::for_task("runtime"))
                .with_replay_candidates(&replay_candidates)
                .with_evolution_ledger(seed_ledger),
            &MemoryMigrationEvidence::read_only_source(Some(2)),
            &[MemoryMigrationPhase::ReadOnlyShadow],
            false,
        );
        let evidence = dry_run.startup_evidence();
        let detail_codes = evidence.detail_codes();
        let evidence_text = evidence.summary_text();

        assert!(dry_run.requires_operator_review());
        assert_eq!(dry_run.summary.context_rejection_count, 1);
        assert_eq!(dry_run.summary.context_rot_risk_count, 1);
        assert_eq!(dry_run.plan.replay_report.context_rot_items, 1);
        assert_eq!(dry_run.summary.replay_context_rot_count, 1);
        assert_eq!(dry_run.plan.evolution_ledger.context_rot_items, 3);
        assert_eq!(
            evidence.context_rot_risk_reason_codes(),
            vec![
                "missing_clean_gist".to_owned(),
                "transcript_anchor_risk".to_owned(),
            ]
        );
        assert!(
            evidence
                .context_rot_risk_detail_codes()
                .contains(&"context_rot:raw:missing_clean_gist".to_owned())
        );
        assert!(
            evidence
                .context_rot_risk_detail_codes()
                .contains(&"context_rot:raw:transcript_anchor_risk".to_owned())
        );
        assert!(
            dry_run
                .plan
                .read_only
                .context
                .summary_line()
                .contains("reject_risk=1")
        );
        assert!(
            dry_run
                .plan
                .read_only
                .context
                .summary_line()
                .contains("reason_codes=missing_clean_gist|raw_fallback_index_content")
        );
        assert_eq!(
            evidence.context_injection_reason_codes(),
            vec![
                "missing_clean_gist".to_owned(),
                "raw_fallback_index_content".to_owned(),
            ]
        );
        assert!(
            evidence
                .context_injection_detail_codes()
                .contains(&"reject_risk:missing_clean_gist:726177".to_owned())
        );
        assert!(
            evidence
                .context_injection_detail_codes()
                .contains(&"reject_risk:raw_fallback_index_content:726177".to_owned())
        );
        assert!(detail_codes.contains(
            &"read_only_detail:context:reject_risk:missing_clean_gist:726177".to_owned()
        ));
        assert!(detail_codes.contains(
            &"read_only_detail:context:reject_risk:raw_fallback_index_content:726177".to_owned()
        ));
        assert!(
            evidence.summary_line().contains(
                "context_rot_risk_reason_codes=missing_clean_gist|transcript_anchor_risk"
            )
        );
        assert_eq!(evidence.context_rot_risk_report_count(), 1);
        assert!(evidence.has_line_prefix("context_rot_risk "));
        assert!(evidence_text.contains(
            "context_rot_risk risks=1 reason_codes=missing_clean_gist|transcript_anchor_risk"
        ));
        assert!(
            dry_run
                .plan
                .replay_report
                .detail_codes()
                .contains(&"signal:context_rot:726177".to_owned())
        );
        assert!(
            dry_run
                .summary
                .detail_codes()
                .contains(&"replay:signal:context_rot:726177".to_owned())
        );
        assert!(
            dry_run
                .plan
                .replay_report
                .summary_line()
                .contains("context_rot=1")
        );
        assert!(
            dry_run
                .plan
                .evolution_ledger
                .summary_line()
                .contains("context_rot_items=3")
        );
        assert_eq!(
            evidence.memory_hygiene_work_action_lanes(),
            vec![
                "context_rot_review".to_owned(),
                "experience_index_rebuild".to_owned()
            ]
        );
        assert!(
            evidence
                .memory_hygiene_work_action_lane_details()
                .contains(&"context_rot_review:repair:15:3".to_owned())
        );
        assert!(evidence.memory_hygiene_work_dispatch_codes().contains(
            &"dispatch:operator_review:isolated:context_rot_review:repair:15:3".to_owned()
        ));
        assert!(evidence.memory_hygiene_work_item_dispatch_codes().contains(
            &"dispatch:operator_review:isolated:context_rot_review:repair:15:3".to_owned()
        ));
        for forbidden in [raw_prompt, raw_lesson, replay_lesson] {
            assert!(
                !evidence_text.contains(forbidden),
                "startup evidence leaked replay/context payload: {forbidden}"
            );
            assert!(
                !dry_run
                    .plan
                    .replay_report
                    .summary_line()
                    .contains(forbidden),
                "replay summary leaked payload: {forbidden}"
            );
            assert!(
                !dry_run
                    .plan
                    .evolution_ledger
                    .summary_line()
                    .contains(forbidden),
                "evolution summary leaked payload: {forbidden}"
            );
        }
    }

    #[test]
    fn shadow_plan_can_pass_review_with_clean_seeded_inputs() {
        let experiences = vec![
            ExperienceEnvelope::new("clean", "prompt", "Stable clean lesson")
                .with_scope(MemoryScope::for_task("runtime"))
                .with_clean_gist("A stable memory summary with enough useful detail.")
                .with_quality(0.9)
                .with_tags(vec!["adapter:test".to_owned()]),
        ];
        let memory_entries = vec![
            RetentionMemoryEntry::new("mem", "semantic durable memory", vec![0.1], 2.4)
                .with_feedback(4, 0)
                .with_access(1, 10),
        ];
        let adapters = vec![status(
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
            true,
            AdapterWriteMode::ReadOnly,
        )];
        let seed_ledger = MemoryEvolutionLedger {
            replay_runs: 1,
            replay_items: 1,
            replay_memory_updates: 1,
            ..MemoryEvolutionLedger::default()
        };

        let plan = MemoryServiceShadowPlan::for_inputs(
            MemoryServiceShadowPlanInputs::new("clean_shadow", &experiences, &[], &memory_entries)
                .with_adapters(&adapters)
                .with_scope(&MemoryScope::for_task("runtime"))
                .with_evolution_ledger(seed_ledger),
        );

        assert!(plan.readiness.ready);
        assert!(plan.projection_audit.is_ready_for_isolated_write());
        assert!(!plan.read_only.requires_operator_review());
        assert!(plan.evolution_assessment.allow_isolated_write);
        assert!(!plan.inspection.has_blockers());
        assert!(!plan.requires_operator_review());
    }

    #[test]
    fn shadow_plan_missing_request_scope_requires_operator_review() {
        let experiences = vec![
            ExperienceEnvelope::new("clean", "prompt", "Stable clean lesson")
                .with_scope(MemoryScope::for_task("runtime"))
                .with_clean_gist("A stable memory summary with enough useful detail.")
                .with_quality(0.9)
                .with_tags(vec!["adapter:test".to_owned()]),
        ];
        let memory_entries = vec![
            RetentionMemoryEntry::new("mem", "semantic durable memory", vec![0.1], 2.4)
                .with_feedback(4, 0)
                .with_access(1, 10),
        ];
        let adapters = vec![status(
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
            true,
            AdapterWriteMode::ReadOnly,
        )];

        let plan = MemoryServiceShadowPlan::for_inputs(
            MemoryServiceShadowPlanInputs::new("clean_shadow", &experiences, &[], &memory_entries)
                .with_adapters(&adapters),
        );
        let summary = plan.summary();

        assert!(plan.request_scope_missing);
        assert!(plan.requires_operator_review());
        assert!(summary.requires_operator_review);
        assert!(
            summary
                .review_reasons
                .contains(&"missing_request_scope".to_owned())
        );
        assert!(
            summary
                .review_detail_codes
                .contains(&"request_scope:missing".to_owned())
        );
    }

    #[test]
    fn shadow_plan_records_replay_evidence_from_candidates() {
        let experiences = vec![
            ExperienceEnvelope::new("clean", "prompt", "Stable clean lesson")
                .with_scope(MemoryScope::for_task("runtime"))
                .with_clean_gist("A stable memory summary with enough useful detail.")
                .with_quality(0.9)
                .with_tags(vec!["adapter:test".to_owned()]),
        ];
        let replay_candidates = vec![
            ReplayCandidate::new("good", "reinforce useful memory", 0.9)
                .with_scope(MemoryScope::for_task("runtime"))
                .with_memory_ids(vec!["1".to_owned()])
                .with_signals(vec![ReplaySignal::RecursiveRuntime]),
            ReplayCandidate::new("rot", "penalize context rot", 0.1)
                .with_scope(MemoryScope::for_task("runtime"))
                .with_signals(vec![ReplaySignal::ContextRot]),
        ];
        let memory_entries = vec![
            RetentionMemoryEntry::new("mem", "semantic durable memory", vec![0.1], 2.4)
                .with_feedback(4, 0)
                .with_access(1, 10),
        ];
        let adapters = vec![status(
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
            true,
            AdapterWriteMode::ReadOnly,
        )];

        let plan = MemoryServiceShadowPlan::for_inputs(
            MemoryServiceShadowPlanInputs::new("replay_shadow", &experiences, &[], &memory_entries)
                .with_replay_candidates(&replay_candidates)
                .with_adapters(&adapters)
                .with_scope(&MemoryScope::for_task("runtime")),
        );
        let summary = plan.summary();

        assert_eq!(plan.replay_report.planned, 2);
        assert_eq!(plan.replay_report.memory_reinforcements, 1);
        assert_eq!(plan.replay_report.context_rot_items, 1);
        assert_eq!(plan.evolution_ledger.replay_runs, 1);
        assert_eq!(plan.evolution_ledger.context_rot_items, 1);
        assert!(plan.evolution_assessment.allow_isolated_write);
        assert!(!summary.requires_operator_review);
        assert_eq!(summary.replay_planned_count, 2);
        assert_eq!(summary.replay_memory_update_count, 1);
        assert_eq!(summary.replay_context_rot_count, 1);
        assert!(
            summary
                .detail_codes()
                .contains(&"replay:item:reinforce:676f6f64".to_owned())
        );
        assert!(
            summary
                .detail_codes()
                .contains(&"replay:item:penalize:726f74".to_owned())
        );
        assert!(
            summary
                .detail_codes()
                .contains(&"replay:memory_update:reinforce:31:676f6f64".to_owned())
        );
        assert!(
            summary
                .detail_codes()
                .contains(&"replay:signal:context_rot:726f74".to_owned())
        );
        assert!(
            summary
                .detail_codes()
                .contains(&"replay:signal:recursive_runtime:676f6f64".to_owned())
        );
        assert!(summary.summary_line().contains("replay_planned=2"));
    }

    #[test]
    fn shadow_plan_surfaces_projection_contract_review_warnings() {
        let experiences = vec![
            ExperienceEnvelope::new("clean", "prompt", "Stable clean lesson")
                .with_scope(MemoryScope::for_task("runtime"))
                .with_clean_gist("A stable memory summary with enough useful detail.")
                .with_quality(0.9)
                .with_tags(vec!["adapter:test".to_owned()]),
        ];
        let memory_entries = vec![
            RetentionMemoryEntry::new("mem", "semantic durable memory", vec![0.1], 2.4)
                .with_feedback(4, 0)
                .with_access(1, 10),
        ];
        let adapters = vec![status(
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
            true,
            AdapterWriteMode::ReadOnly,
        )];
        let seed_ledger = MemoryEvolutionLedger {
            replay_runs: 1,
            replay_items: 1,
            replay_memory_updates: 1,
            ..MemoryEvolutionLedger::default()
        };
        let contracts = vec![AdapterProjectionContract::experience_store_read_only(
            "experience_shadow",
            vec![
                crate::AdapterProjectionField::ExperienceId,
                crate::AdapterProjectionField::ExperiencePrompt,
                crate::AdapterProjectionField::ExperienceLesson,
                crate::AdapterProjectionField::ExperienceQuality,
            ],
        )];

        let dry_run = MemoryServiceDryRun::for_inputs(
            MemoryServiceShadowPlanInputs::new(
                "contract_shadow",
                &experiences,
                &[],
                &memory_entries,
            )
            .with_adapters(&adapters)
            .with_scope(&MemoryScope::for_task("runtime"))
            .with_evolution_ledger(seed_ledger)
            .with_projection_contracts(&contracts, AdapterProjectionTarget::ShadowRead),
            &MemoryMigrationEvidence::read_only_source(Some(1)),
            &[MemoryMigrationPhase::ReadOnlyShadow],
            false,
        );
        let checklist = dry_run.adapter_checklist();

        assert!(dry_run.summary.ready);
        assert!(dry_run.requires_operator_review());
        assert_eq!(dry_run.summary.projection_contract_count, 1);
        assert_eq!(dry_run.summary.projection_contract_manifest_count, 1);
        assert_eq!(dry_run.summary.projection_contract_blocker_count, 0);
        assert_eq!(dry_run.summary.projection_contract_warning_count, 3);
        assert!(dry_run.summary.detail_codes().contains(
            &"projection_contract_warning_detail:experience_shadow:missing_recommended:experience_clean_gist"
                .to_owned()
        ));
        assert!(
            dry_run
                .summary
                .review_reasons
                .contains(&"projection_contract_warnings".to_owned())
        );
        assert!(checklist.warnings().iter().any(|item| {
            item.code == "projection_contract_warnings_clean"
                && item.detail
                    == dry_run
                        .summary
                        .projection_contract_warnings_checklist_detail()
        }));
    }

    #[test]
    fn shadow_plan_blocks_isolated_write_when_projection_contract_is_incomplete() {
        let experiences = vec![
            ExperienceEnvelope::new("clean", "prompt", "Stable clean lesson")
                .with_scope(MemoryScope::for_task("runtime"))
                .with_clean_gist("A stable memory summary with enough useful detail.")
                .with_quality(0.9)
                .with_tags(vec!["adapter:test".to_owned()]),
        ];
        let memory_entries = vec![
            RetentionMemoryEntry::new("mem", "semantic durable memory", vec![0.1], 2.4)
                .with_feedback(4, 0)
                .with_access(1, 10),
        ];
        let adapters = vec![status(
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
            true,
            AdapterWriteMode::ReadOnly,
        )];
        let seed_ledger = MemoryEvolutionLedger {
            replay_runs: 1,
            replay_items: 1,
            replay_memory_updates: 1,
            ..MemoryEvolutionLedger::default()
        };
        let contracts = vec![AdapterProjectionContract::experience_store_read_only(
            "experience_shadow",
            vec![
                crate::AdapterProjectionField::ExperienceId,
                crate::AdapterProjectionField::ExperiencePrompt,
                crate::AdapterProjectionField::ExperienceLesson,
                crate::AdapterProjectionField::ExperienceQuality,
            ],
        )];

        let dry_run = MemoryServiceDryRun::for_inputs(
            MemoryServiceShadowPlanInputs::new(
                "contract_shadow",
                &experiences,
                &[],
                &memory_entries,
            )
            .with_adapters(&adapters)
            .with_scope(&MemoryScope::for_task("runtime"))
            .with_evolution_ledger(seed_ledger)
            .with_projection_contracts(&contracts, AdapterProjectionTarget::IsolatedWrite),
            &MemoryMigrationEvidence::copied_fixture(1),
            &[MemoryMigrationPhase::IsolatedWrite],
            false,
        );
        let checklist = dry_run.adapter_checklist();
        let approval = dry_run
            .approval_for(MemoryMigrationPhase::IsolatedWrite)
            .unwrap();

        assert!(dry_run.requires_operator_review());
        assert_eq!(dry_run.summary.projection_contract_blocker_count, 3);
        assert!(dry_run.summary.detail_codes().contains(
            &"projection_contract_blocker_detail:experience_shadow:missing_required:experience_projection_tags"
                .to_owned()
        ));
        assert!(
            dry_run
                .summary
                .review_reasons
                .contains(&"projection_contract_blockers".to_owned())
        );
        assert!(checklist.blockers().iter().any(|item| {
            item.code == "projection_contracts_ready"
                && item.detail
                    == dry_run
                        .summary
                        .projection_contracts_ready_checklist_detail()
        }));
        assert!(!approval.approved);
        assert!(
            approval
                .blockers
                .contains(&"shadow_plan_requires_operator_review".to_owned())
        );
    }

    #[test]
    fn shadow_plan_exposes_default_migration_approval_gate() {
        let experiences = vec![
            ExperienceEnvelope::new("clean", "prompt", "Stable clean lesson")
                .with_scope(MemoryScope::for_task("runtime"))
                .with_clean_gist("A stable memory summary with enough useful detail.")
                .with_quality(0.9)
                .with_tags(vec!["adapter:test".to_owned()]),
        ];
        let memory_entries = vec![
            RetentionMemoryEntry::new("mem", "semantic durable memory", vec![0.1], 2.4)
                .with_feedback(4, 0)
                .with_access(1, 10),
        ];
        let adapters = vec![status(
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
            true,
            AdapterWriteMode::ReadOnly,
        )];
        let seed_ledger = MemoryEvolutionLedger {
            replay_runs: 1,
            replay_items: 1,
            replay_memory_updates: 1,
            ..MemoryEvolutionLedger::default()
        };

        let plan = MemoryServiceShadowPlan::for_inputs(
            MemoryServiceShadowPlanInputs::new("clean_shadow", &experiences, &[], &memory_entries)
                .with_adapters(&adapters)
                .with_scope(&MemoryScope::for_task("runtime"))
                .with_evolution_ledger(seed_ledger),
        );
        let approval = plan.migration_approval(
            MemoryMigrationPhase::IsolatedWrite,
            &MemoryMigrationEvidence::copied_fixture(1),
            false,
        );

        assert!(approval.approved);
        assert_eq!(
            approval.required_write_mode,
            AdapterWriteMode::IsolatedWrite
        );
    }

    #[test]
    fn shadow_plan_records_clean_projection_parity() {
        let experiences = vec![
            ExperienceEnvelope::new("clean", "prompt", "Stable clean lesson")
                .with_scope(MemoryScope::for_task("runtime"))
                .with_clean_gist("A stable memory summary with enough useful detail.")
                .with_quality(0.9)
                .with_tags(vec!["adapter:test".to_owned()]),
        ];
        let memory_entries = vec![
            RetentionMemoryEntry::new("mem", "semantic durable memory", vec![0.1], 2.4)
                .with_feedback(4, 0)
                .with_access(1, 10),
        ];
        let adapters = vec![status(
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
            true,
            AdapterWriteMode::ReadOnly,
        )];
        let adaptive_projection = AdaptiveStateMemoryProjection {
            replay_runs: 1,
            replay_items: 1,
            replay_memory_updates: 1,
            ..AdaptiveStateMemoryProjection::default()
        };
        let state_projection = StateInspectionMemoryProjection {
            memory_count: Some(1),
            runtime_kv_memory_count: Some(0),
            experience_count: Some(1),
            kv_shard_count: Some(0),
            adapter_count: Some(1),
            projection_blocker_count: Some(0),
            evolution_blocker_count: Some(0),
            evolution_warning_count: Some(0),
            ..StateInspectionMemoryProjection::default()
        };
        let seed_ledger = adaptive_projection.to_ledger();

        let plan = MemoryServiceShadowPlan::for_inputs(
            MemoryServiceShadowPlanInputs::new("clean_shadow", &experiences, &[], &memory_entries)
                .with_adapters(&adapters)
                .with_scope(&MemoryScope::for_task("runtime"))
                .with_evolution_ledger(seed_ledger)
                .with_adaptive_state_projection(&adaptive_projection)
                .with_state_inspection_projection(&state_projection),
        );

        assert!(plan.projection_parity_audit.is_clean());
        assert!(!plan.requires_operator_review());
    }

    #[test]
    fn shadow_plan_requires_review_when_root_projection_drifts() {
        let experiences = vec![
            ExperienceEnvelope::new("clean", "prompt", "Stable clean lesson")
                .with_scope(MemoryScope::for_task("runtime"))
                .with_clean_gist("A stable memory summary with enough useful detail.")
                .with_quality(0.9)
                .with_tags(vec!["adapter:test".to_owned()]),
        ];
        let memory_entries = vec![
            RetentionMemoryEntry::new("mem", "semantic durable memory", vec![0.1], 2.4)
                .with_feedback(4, 0)
                .with_access(1, 10),
        ];
        let adapters = vec![status(
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
            true,
            AdapterWriteMode::ReadOnly,
        )];
        let seed_ledger = MemoryEvolutionLedger {
            replay_runs: 1,
            replay_items: 1,
            replay_memory_updates: 1,
            ..MemoryEvolutionLedger::default()
        };
        let stale_projection = AdaptiveStateMemoryProjection {
            replay_runs: 2,
            replay_items: 1,
            replay_memory_updates: 1,
            ..AdaptiveStateMemoryProjection::default()
        };

        let plan = MemoryServiceShadowPlan::for_inputs(
            MemoryServiceShadowPlanInputs::new("clean_shadow", &experiences, &[], &memory_entries)
                .with_adapters(&adapters)
                .with_scope(&MemoryScope::for_task("runtime"))
                .with_evolution_ledger(seed_ledger)
                .with_adaptive_state_projection(&stale_projection),
        );

        assert!(plan.projection_parity_audit.requires_operator_review());
        assert_eq!(
            plan.projection_parity_audit.mismatches[0].field,
            "replay_runs"
        );
        assert!(plan.requires_operator_review());
    }

    #[test]
    fn shadow_summary_reports_clean_dry_run_counts() {
        let experiences = vec![
            ExperienceEnvelope::new("clean", "prompt", "Stable clean lesson")
                .with_scope(MemoryScope::for_task("runtime"))
                .with_clean_gist("A stable memory summary with enough useful detail.")
                .with_quality(0.9)
                .with_tags(vec!["adapter:test".to_owned()]),
        ];
        let memory_entries = vec![
            RetentionMemoryEntry::new("mem", "semantic durable memory", vec![0.1], 2.4)
                .with_feedback(4, 0)
                .with_access(1, 10),
        ];
        let adapters = vec![status(
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
            true,
            AdapterWriteMode::ReadOnly,
        )];
        let seed_ledger = MemoryEvolutionLedger {
            replay_runs: 1,
            replay_items: 1,
            replay_memory_updates: 1,
            ..MemoryEvolutionLedger::default()
        };

        let plan = MemoryServiceShadowPlan::for_inputs(
            MemoryServiceShadowPlanInputs::new("clean_shadow", &experiences, &[], &memory_entries)
                .with_adapters(&adapters)
                .with_scope(&MemoryScope::for_task("runtime"))
                .with_evolution_ledger(seed_ledger),
        );
        let summary = plan.summary();

        assert!(summary.ready);
        assert!(!summary.requires_operator_review);
        assert_eq!(summary.experience_count, 1);
        assert_eq!(summary.memory_count, 1);
        assert!(!summary.kvswap_state_present);
        assert_eq!(summary.kvswap_hot_shard_count, 0);
        assert_eq!(summary.kvswap_cold_shard_count, 0);
        assert_eq!(summary.kvswap_metadata_count, 0);
        assert_eq!(summary.kvswap_total_byte_len, 0);
        assert_eq!(summary.kvswap_shape_codes, Vec::<String>::new());
        assert!(summary.review_reasons.is_empty());
        assert_eq!(summary.reason_codes(), Vec::<String>::new());
        assert_eq!(summary.detail_codes(), Vec::<String>::new());
        assert!(summary.summary_line().contains("memory_shadow ready=true"));
        assert!(
            summary
                .summary_line()
                .contains("kvswap_state=false kvswap_hot=0 kvswap_cold=0 kvswap_metadata=0 kvswap_bytes=0 kvswap_shape_codes=none")
        );
        assert!(summary.summary_line().contains("reasons=none"));
        assert!(summary.summary_line().contains("reason_codes=none"));
        assert!(summary.summary_line().contains("detail_codes=none"));
    }

    #[test]
    fn context_rot_blocker_reason_codes_are_report_only_for_shadow_readiness() {
        let runtime_scope = MemoryScope::for_task("runtime");
        let experiences = vec![
            ExperienceEnvelope::new("clean", "runtime prompt", "Stable clean lesson")
                .with_scope(runtime_scope.clone())
                .with_clean_gist("A stable memory summary with enough useful detail.")
                .with_quality(0.9)
                .with_tags(vec!["adapter:test".to_owned()]),
            ExperienceEnvelope::new(
                "rot",
                format!(
                    "Conversation Transcript:\nUser: ssh -o ConnectTimeout=1 {}\nAssistant: ok",
                    "x".repeat(2_700)
                ),
                "accepted_pattern quality=0.1 max_severity=critical",
            )
            .with_scope(runtime_scope.clone())
            .with_tags(vec!["adapter:test".to_owned()]),
        ];
        let adapters = vec![status(
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
            true,
            AdapterWriteMode::ReadOnly,
        )];
        let seed_ledger = MemoryEvolutionLedger {
            replay_runs: 1,
            replay_items: 1,
            replay_memory_updates: 1,
            ..MemoryEvolutionLedger::default()
        };
        let evidence = MemoryMigrationEvidence::read_only_source(Some(experiences.len()));

        let dry_run = MemoryServiceDryRun::for_inputs(
            MemoryServiceShadowPlanInputs::new("shadow", &experiences, &[], &[])
                .with_adapters(&adapters)
                .with_scope(&runtime_scope)
                .with_evolution_ledger(seed_ledger),
            &evidence,
            &[MemoryMigrationPhase::ReadOnlyShadow],
            false,
        );
        let summary = &dry_run.summary;
        let startup = dry_run.startup_evidence();

        assert_eq!(
            summary.context_rot_blocker_reason_codes,
            vec!["long_without_clean_gist".to_owned()]
        );
        assert_eq!(
            dry_run
                .plan
                .read_only
                .quality_gate
                .context_rot_blocker_reason_codes,
            summary.context_rot_blocker_reason_codes
        );
        assert!(dry_run.plan.readiness.ready);
        assert!(dry_run.plan.readiness.write_mode_blockers.is_empty());
        assert_eq!(summary.readiness_write_blocker_count, 0);
        assert!(summary.ready);
        assert!(dry_run.plan.projection_audit.is_ready_for_shadow_read());
        assert_eq!(
            dry_run.plan.read_only.summary.write_mode,
            AdapterWriteMode::ReadOnly
        );
        assert!(!dry_run.migration_evidence.live_store_targeted);
        assert_eq!(
            dry_run.approved_phases(),
            vec![MemoryMigrationPhase::ReadOnlyShadow]
        );
        assert_eq!(summary.context_rejection_count, 1);
        assert_eq!(
            dry_run.plan.read_only.context.rejected_ids(),
            vec!["rot".to_owned()]
        );
        assert!(
            !dry_run
                .plan
                .read_only
                .context
                .reason_codes()
                .contains(&"long_without_clean_gist".to_owned())
        );
        assert!(
            dry_run
                .plan
                .read_only
                .summary_line()
                .contains("context_reject=1")
        );
        assert!(
            summary
                .summary_line()
                .contains("context_rot_blocker_reason_codes=long_without_clean_gist")
        );
        assert_eq!(
            startup.context_rot_blocker_reason_codes(),
            vec!["long_without_clean_gist".to_owned()]
        );
        assert!(
            startup
                .summary_line()
                .contains("context_rot_blocker_reason_codes=long_without_clean_gist")
        );
    }

    #[test]
    fn shadow_summary_requires_review_for_kvswap_boundary_issues() {
        let experiences = vec![
            ExperienceEnvelope::new("clean", "prompt", "Stable clean lesson")
                .with_scope(MemoryScope::for_task("runtime"))
                .with_clean_gist("A stable memory summary with enough useful detail.")
                .with_quality(0.9)
                .with_tags(vec!["adapter:test".to_owned()]),
        ];
        let memory_entries = vec![
            RetentionMemoryEntry::new("mem", "semantic durable memory", vec![0.1], 2.4)
                .with_feedback(4, 0)
                .with_access(1, 10),
        ];
        let adapters = vec![status(
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
            true,
            AdapterWriteMode::ReadOnly,
        )];
        let seed_ledger = MemoryEvolutionLedger {
            replay_runs: 1,
            replay_items: 1,
            replay_memory_updates: 1,
            ..MemoryEvolutionLedger::default()
        };
        let boundary = KvSwapBoundaryAudit {
            overlapping_hot_cold_ids: vec!["shard-a".to_owned()],
            ..KvSwapBoundaryAudit::default()
        };

        let dry_run = MemoryServiceDryRun::for_inputs(
            MemoryServiceShadowPlanInputs::new("clean_shadow", &experiences, &[], &memory_entries)
                .with_adapters(&adapters)
                .with_scope(&MemoryScope::for_task("runtime"))
                .with_evolution_ledger(seed_ledger)
                .with_kvswap_state(KvSwapStateSnapshot {
                    hot_shard_count: 1,
                    cold_shard_count: 1,
                    metadata_count: 2,
                    hot_byte_len: 4,
                    cold_byte_len: 8,
                })
                .with_kvswap_boundary(boundary),
            &MemoryMigrationEvidence::read_only_source(Some(1)),
            &[MemoryMigrationPhase::ReadOnlyShadow],
            false,
        );
        let summary = &dry_run.summary;
        let evidence = dry_run.startup_evidence();

        assert!(summary.ready);
        assert!(dry_run.plan.requires_operator_review());
        assert!(dry_run.requires_operator_review());
        assert!(summary.requires_operator_review);
        assert!(summary.kvswap_state_present);
        assert_eq!(summary.kvswap_hot_shard_count, 1);
        assert_eq!(summary.kvswap_cold_shard_count, 1);
        assert_eq!(summary.kvswap_metadata_count, 2);
        assert_eq!(summary.kvswap_total_byte_len, 12);
        assert_eq!(
            summary.kvswap_shape_codes,
            vec![
                "cold_catalog".to_owned(),
                "hot_metadata".to_owned(),
                "metadata_index".to_owned(),
                "mixed_tiers".to_owned(),
            ]
        );
        assert!(summary.kvswap_boundary_present);
        assert_eq!(summary.kvswap_boundary_issue_count, 1);
        assert_eq!(
            summary.kvswap_boundary_reason_codes,
            vec!["overlapping_hot_cold".to_owned()]
        );
        assert_eq!(summary.hygiene_pressure_score, 100);
        assert_eq!(summary.hygiene_pressure_priority, "quarantine");
        assert_eq!(
            summary.hygiene_pressure_reason_codes,
            vec!["kvswap_boundary_blocker".to_owned()]
        );
        assert_eq!(
            summary.hygiene_pressure_detail_codes,
            vec!["kvswap_boundary_blockers:1".to_owned()]
        );
        assert_eq!(
            summary.hygiene_pressure_action_lanes,
            vec!["kvswap_boundary_repair".to_owned()]
        );
        assert_eq!(
            summary.hygiene_pressure_action_lane_details,
            vec!["kvswap_boundary_repair:quarantine:100:1".to_owned()]
        );
        assert_eq!(summary.hygiene_work_next_action, "kvswap_boundary_repair");
        assert!(summary.hygiene_work_operator_review_required);
        assert!(summary.hygiene_work_isolation_recommended);
        assert_eq!(summary.hygiene_work_queue_item_count, 1);
        assert_eq!(summary.hygiene_work_queue_operator_review_count, 1);
        assert_eq!(summary.hygiene_work_queue_isolation_count, 1);
        assert_eq!(
            summary.hygiene_work_queue_next_dispatch,
            "dispatch:operator_review:isolated:kvswap_boundary_repair:quarantine:100:1"
        );
        assert_eq!(
            summary.hygiene_work_queue_lane_codes,
            vec!["kvswap_boundary_repair".to_owned()]
        );
        assert_eq!(
            summary.hygiene_work_queue_priority_codes,
            vec!["quarantine".to_owned()]
        );
        assert_eq!(
            summary.hygiene_work_queue_dispatch_codes,
            vec![
                "dispatch:operator_review:isolated:kvswap_boundary_repair:quarantine:100:1"
                    .to_owned()
            ]
        );
        assert_eq!(
            summary.hygiene_work_queue_detail_codes,
            vec!["kvswap_boundary_repair:quarantine:100:1".to_owned()]
        );
        assert_eq!(
            summary.hygiene_work_queue_reason_codes,
            vec![
                "isolation_recommended".to_owned(),
                "items_present".to_owned(),
                "operator_review_required".to_owned()
            ]
        );
        assert_eq!(
            summary.reason_codes(),
            vec![
                "evolution_review".to_owned(),
                "kvswap_boundary_review".to_owned()
            ]
        );
        assert!(
            summary
                .detail_codes()
                .contains(&"kvswap_boundary:overlapping_hot_cold".to_owned())
        );
        assert!(
            summary
                .detail_codes()
                .contains(&"kvswap_boundary_detail:overlap:73686172642d61".to_owned())
        );
        assert!(summary.summary_line().contains(
            "kvswap_state=true kvswap_hot=1 kvswap_cold=1 kvswap_metadata=2 kvswap_bytes=12 kvswap_shape_codes=cold_catalog|hot_metadata|metadata_index|mixed_tiers"
        ));
        assert!(summary.summary_line().contains(
            "kvswap_boundary=true kvswap_boundary_issues=1 kvswap_boundary_reason_codes=overlapping_hot_cold"
        ));
        assert!(summary.summary_line().contains(
            "hygiene_pressure_score=100 hygiene_pressure_priority=quarantine hygiene_pressure_action_lanes=kvswap_boundary_repair hygiene_pressure_action_lane_details=kvswap_boundary_repair:quarantine:100:1 hygiene_work_next_action=kvswap_boundary_repair hygiene_work_operator_review=true hygiene_work_isolation=true hygiene_work_queue_items=1 hygiene_work_queue_operator_review=1 hygiene_work_queue_isolation=1 hygiene_work_queue_next_dispatch=dispatch:operator_review:isolated:kvswap_boundary_repair:quarantine:100:1 hygiene_work_queue_lanes=kvswap_boundary_repair hygiene_work_queue_priorities=quarantine hygiene_work_queue_dispatch_codes=dispatch:operator_review:isolated:kvswap_boundary_repair:quarantine:100:1 hygiene_work_queue_detail_codes=kvswap_boundary_repair:quarantine:100:1 hygiene_work_queue_reason_codes=isolation_recommended|items_present|operator_review_required hygiene_pressure_reason_codes=kvswap_boundary_blocker hygiene_pressure_detail_codes=kvswap_boundary_blockers:1"
        ));
        assert!(
            evidence
                .summary_text()
                .contains("memory_hygiene_work_plan ")
        );
        assert_eq!(evidence.memory_hygiene_work_plan_count(), 1);
        assert_eq!(
            evidence.memory_hygiene_work_next_action_codes(),
            vec!["kvswap_boundary_repair".to_owned()]
        );
        assert_eq!(evidence.memory_hygiene_work_operator_review_count(), 1);
        assert_eq!(evidence.memory_hygiene_work_isolation_count(), 1);
        assert_eq!(
            evidence.memory_hygiene_work_action_lanes(),
            vec!["kvswap_boundary_repair".to_owned()]
        );
        assert_eq!(
            evidence.memory_hygiene_work_action_lane_details(),
            vec!["kvswap_boundary_repair:quarantine:100:1".to_owned()]
        );
        assert_eq!(
            evidence.memory_hygiene_work_next_dispatch_codes(),
            vec![
                "dispatch:operator_review:isolated:kvswap_boundary_repair:quarantine:100:1"
                    .to_owned()
            ]
        );
        assert_eq!(
            evidence.memory_hygiene_work_dispatch_codes(),
            vec![
                "dispatch:operator_review:isolated:kvswap_boundary_repair:quarantine:100:1"
                    .to_owned()
            ]
        );
        assert_eq!(evidence.memory_hygiene_work_queue_count(), 1);
        assert_eq!(evidence.memory_hygiene_work_queue_item_count(), 1);
        assert_eq!(
            evidence.memory_hygiene_work_queue_operator_review_count(),
            1
        );
        assert_eq!(evidence.memory_hygiene_work_queue_isolation_count(), 1);
        assert_eq!(
            evidence.memory_hygiene_work_queue_next_dispatch_codes(),
            vec![
                "dispatch:operator_review:isolated:kvswap_boundary_repair:quarantine:100:1"
                    .to_owned()
            ]
        );
        assert_eq!(
            evidence.memory_hygiene_work_queue_lane_codes(),
            vec!["kvswap_boundary_repair".to_owned()]
        );
        assert_eq!(
            evidence.memory_hygiene_work_queue_priority_codes(),
            vec!["quarantine".to_owned()]
        );
        assert_eq!(
            evidence.memory_hygiene_work_queue_dispatch_codes(),
            vec![
                "dispatch:operator_review:isolated:kvswap_boundary_repair:quarantine:100:1"
                    .to_owned()
            ]
        );
        assert_eq!(
            evidence.memory_hygiene_work_queue_detail_codes(),
            vec!["kvswap_boundary_repair:quarantine:100:1".to_owned()]
        );
        assert_eq!(
            evidence.memory_hygiene_work_queue_reason_codes(),
            vec![
                "isolation_recommended".to_owned(),
                "items_present".to_owned(),
                "operator_review_required".to_owned()
            ]
        );
        assert_eq!(evidence.memory_hygiene_work_item_count(), 1);
        assert_eq!(
            evidence.memory_hygiene_work_item_lane_codes(),
            vec!["kvswap_boundary_repair".to_owned()]
        );
        assert_eq!(
            evidence.memory_hygiene_work_item_priority_codes(),
            vec!["quarantine".to_owned()]
        );
        assert_eq!(
            evidence.memory_hygiene_work_item_dispatch_codes(),
            vec![
                "dispatch:operator_review:isolated:kvswap_boundary_repair:quarantine:100:1"
                    .to_owned()
            ]
        );
        assert_eq!(
            evidence.memory_hygiene_work_item_detail_codes(),
            vec!["kvswap_boundary_repair:quarantine:100:1".to_owned()]
        );
        assert!(evidence.kvswap_state_present());
        assert_eq!(evidence.kvswap_state_hot_shard_count(), 1);
        assert_eq!(evidence.kvswap_state_cold_shard_count(), 1);
        assert_eq!(evidence.kvswap_state_metadata_count(), 2);
        assert_eq!(evidence.kvswap_state_total_byte_len(), 12);
        assert_eq!(
            evidence.kvswap_state_shape_codes(),
            vec![
                "cold_catalog".to_owned(),
                "hot_metadata".to_owned(),
                "metadata_index".to_owned(),
                "mixed_tiers".to_owned(),
            ]
        );
        assert_eq!(evidence.kvswap_boundary_issue_count(), 1);
        assert_eq!(
            evidence.kvswap_boundary_reason_codes(),
            vec!["overlapping_hot_cold".to_owned()]
        );
        assert_eq!(
            evidence.kvswap_boundary_detail_codes(),
            vec!["overlap:73686172642d61".to_owned()]
        );
        assert!(evidence.summary_line().contains("kvswap_boundary_issues=1"));
    }

    #[test]
    fn shadow_summary_carries_retention_and_compaction_detail_codes() {
        let experiences = vec![
            ExperienceEnvelope::new("clean", "prompt", "Stable clean lesson")
                .with_scope(MemoryScope::for_task("runtime"))
                .with_clean_gist("A stable memory summary with enough useful detail.")
                .with_quality(0.9)
                .with_tags(vec!["adapter:test".to_owned()]),
        ];
        let memory_entries = vec![
            RetentionMemoryEntry::new("failed", "semantic:bad", vec![0.1, 0.2], 0.02)
                .with_feedback(0, 4)
                .with_access(1, 1),
            RetentionMemoryEntry::new("strong", "runtime_kv:route", vec![1.0, 0.0], 0.9)
                .with_feedback(2, 0)
                .with_access(1, 10),
            RetentionMemoryEntry::new("weak", "runtime_kv:route copy", vec![0.99, 0.01], 0.2)
                .with_access(1, 10),
        ];
        let adapters = vec![status(
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
            true,
            AdapterWriteMode::ReadOnly,
        )];
        let seed_ledger = MemoryEvolutionLedger {
            replay_runs: 1,
            replay_items: 1,
            replay_memory_updates: 1,
            ..MemoryEvolutionLedger::default()
        };

        let plan = MemoryServiceShadowPlan::for_inputs(
            MemoryServiceShadowPlanInputs::new(
                "maintenance_shadow",
                &experiences,
                &[],
                &memory_entries,
            )
            .with_adapters(&adapters)
            .with_scope(&MemoryScope::for_task("runtime"))
            .with_evolution_ledger(seed_ledger)
            .with_maintenance(
                10,
                MemoryRetentionPolicy {
                    stale_after: 4,
                    decay_rate: 0.10,
                    remove_below_strength: 0.04,
                    remove_after_failures: 4,
                },
                MemoryCompactionPolicy::default(),
                Vec::new(),
            ),
        );
        let summary = plan.summary();

        assert_eq!(summary.retention_decay_count, 1);
        assert_eq!(summary.retention_removal_count, 1);
        assert_eq!(summary.compaction_merge_count, 1);
        assert_eq!(summary.compaction_removal_count, 1);
        assert!(
            summary
                .detail_codes()
                .contains(&"retention:decay:stale_decay:6661696c6564".to_owned())
        );
        assert!(summary.detail_codes().contains(
            &"retention:remove:weak_stale_and_repeated_failures:6661696c6564".to_owned()
        ));
        assert!(summary.detail_codes().contains(
            &"compaction:merge:same_namespace_high_similarity:7374726f6e67:7765616b".to_owned()
        ));
        assert!(
            summary
                .detail_codes()
                .contains(&"compaction:remove:7765616b".to_owned())
        );
        assert!(
            summary
                .summary_line()
                .contains("retention:remove:weak_stale_and_repeated_failures:6661696c6564")
        );
    }

    #[test]
    fn shadow_summary_carries_evolution_assessment_detail_codes() {
        let experiences = vec![
            ExperienceEnvelope::new("clean", "prompt", "Stable clean lesson")
                .with_scope(MemoryScope::for_task("runtime"))
                .with_clean_gist("A stable memory summary with enough useful detail.")
                .with_quality(0.9)
                .with_tags(vec!["adapter:test".to_owned()]),
        ];
        let memory_entries = vec![
            RetentionMemoryEntry::new("memory", "durable semantic memory", vec![0.2], 1.0)
                .with_feedback(1, 0)
                .with_access(1, 1),
        ];
        let adapters = vec![status(
            "all_shadow",
            vec![
                MemoryAdapterCapability::ExperienceGovernance,
                MemoryAdapterCapability::CleanGistSelection,
                MemoryAdapterCapability::MemoryIndex,
                MemoryAdapterCapability::ExperienceReplay,
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
            true,
            AdapterWriteMode::ReadOnly,
        )];
        let seed_ledger = MemoryEvolutionLedger {
            replay_runs: 1,
            replay_items: 2,
            replay_memory_updates: 4,
            context_rot_items: 9,
            drift_rollbacks: 1,
            ..MemoryEvolutionLedger::default()
        };

        let plan = MemoryServiceShadowPlan::for_inputs(
            MemoryServiceShadowPlanInputs::new("evolution", &experiences, &[], &memory_entries)
                .with_adapters(&adapters)
                .with_scope(&MemoryScope::for_task("runtime"))
                .with_evolution_ledger(seed_ledger),
        );
        let summary = plan.summary();

        assert!(summary.requires_operator_review);
        assert!(summary.evolution_warning_count >= 2);
        assert!(
            summary
                .detail_codes()
                .contains(&"evolution_warning:context_rot_items".to_owned())
        );
        assert!(
            summary
                .detail_codes()
                .contains(&"evolution_warning_detail:context_rot_items:9".to_owned())
        );
        assert!(
            summary
                .detail_codes()
                .contains(&"evolution_warning_detail:drift_rollbacks:1".to_owned())
        );
        assert!(
            summary
                .summary_line()
                .contains("evolution_warning_detail:context_rot_items:9")
        );
    }

    #[test]
    fn shadow_summary_carries_adapter_snapshot_warning_evidence() {
        let experiences = vec![
            ExperienceEnvelope::new("clean", "prompt", "Stable clean lesson")
                .with_scope(MemoryScope::for_task("runtime"))
                .with_clean_gist("A stable memory summary with enough useful detail.")
                .with_quality(0.9)
                .with_tags(vec!["adapter:test".to_owned()]),
        ];
        let memory_entries = vec![
            RetentionMemoryEntry::new("mem", "semantic durable memory", vec![0.1], 2.4)
                .with_feedback(4, 0)
                .with_access(1, 10),
        ];
        let adapters = vec![status(
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
            true,
            AdapterWriteMode::ReadOnly,
        )];
        let adapter_snapshots = vec![AdapterSnapshotSummary {
            adapter_name: "experience_shadow".to_owned(),
            write_mode: AdapterWriteMode::ReadOnly,
            experience_count: 1,
            kv_shard_count: 0,
            warnings: vec!["adapter_unhealthy".to_owned(), "store_lag=2".to_owned()],
        }];
        let seed_ledger = MemoryEvolutionLedger {
            replay_runs: 1,
            replay_items: 1,
            replay_memory_updates: 1,
            ..MemoryEvolutionLedger::default()
        };

        let plan = MemoryServiceShadowPlan::for_inputs(
            MemoryServiceShadowPlanInputs::new(
                "adapter_snapshot_warning",
                &experiences,
                &[],
                &memory_entries,
            )
            .with_adapters(&adapters)
            .with_adapter_snapshots(&adapter_snapshots)
            .with_scope(&MemoryScope::for_task("runtime"))
            .with_evolution_ledger(seed_ledger),
        );
        let summary = plan.summary();

        assert!(plan.requires_operator_review());
        assert!(summary.requires_operator_review);
        assert_eq!(summary.adapter_snapshot_count, 1);
        assert_eq!(summary.adapter_snapshot_warning_count, 2);
        assert!(
            summary
                .review_reasons
                .contains(&"adapter_snapshot_warnings".to_owned())
        );
        let checklist = MemoryServiceDryRun {
            plan: plan.clone(),
            summary: summary.clone(),
            migration_evidence: MemoryMigrationEvidence::read_only_source(Some(1)),
            approvals: Vec::new(),
        }
        .adapter_checklist();
        assert!(checklist.warnings().iter().any(|item| {
            item.code == "adapter_snapshots_clean"
                && item.detail == summary.adapter_snapshot_checklist_detail()
                && item.detail_codes().contains(&"warnings".to_owned())
        }));
        assert!(
            checklist
                .warning_detail_codes()
                .contains(&"adapter_snapshots_clean:warnings".to_owned())
        );
        assert!(
            checklist
                .summary_line()
                .contains("warning_codes=adapter_snapshots_clean")
        );
        assert!(
            summary
                .detail_codes()
                .contains(&"adapter_snapshot:experience_shadow:adapter_unhealthy".to_owned())
        );
        assert!(
            summary
                .detail_codes()
                .contains(&"adapter_snapshot:experience_shadow:store_lag".to_owned())
        );
        assert!(
            summary
                .summary_line()
                .contains("adapter_snapshots=1 adapter_snapshot_warnings=2")
        );
    }

    #[test]
    fn shadow_summary_collects_review_reasons_from_rot_and_parity() {
        let experiences = vec![
            ExperienceEnvelope::new("keep", "runtime prompt", "runtime lesson")
                .with_scope(MemoryScope::for_task("runtime"))
                .with_tags(vec!["adapter:test".to_owned()]),
            ExperienceEnvelope::new(
                "rot",
                "Conversation Transcript:\nUser: ssh -o ConnectTimeout=1\nAssistant: ok",
                "accepted_pattern quality=0.1 max_severity=critical",
            )
            .with_scope(MemoryScope::for_task("ops"))
            .with_tags(vec!["adapter:test".to_owned()]),
        ];
        let memory_entries = vec![
            RetentionMemoryEntry::new("runtime", "runtime_kv:block", vec![0.2], 1.2)
                .with_feedback(1, 0)
                .with_access(1, 9),
        ];
        let adapters = vec![status(
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
            true,
            AdapterWriteMode::ReadOnly,
        )];
        let seed_ledger = MemoryEvolutionLedger {
            replay_runs: 1,
            replay_items: 1,
            replay_memory_updates: 1,
            ..MemoryEvolutionLedger::default()
        };
        let stale_projection = AdaptiveStateMemoryProjection {
            replay_runs: 2,
            replay_items: 1,
            replay_memory_updates: 1,
            ..AdaptiveStateMemoryProjection::default()
        };

        let plan = MemoryServiceShadowPlan::for_inputs(
            MemoryServiceShadowPlanInputs::new("shadow", &experiences, &[], &memory_entries)
                .with_adapters(&adapters)
                .with_scope(&MemoryScope::for_task("runtime"))
                .with_evolution_ledger(seed_ledger)
                .with_adaptive_state_projection(&stale_projection),
        );
        let summary = plan.summary();

        assert!(summary.ready);
        assert!(summary.requires_operator_review);
        assert_eq!(summary.context_rejection_count, 0);
        assert_eq!(summary.projection_parity_mismatch_count, 4);
        assert!(
            summary
                .review_reasons
                .contains(&"projection_parity_review".to_owned())
        );
        assert!(
            summary
                .review_reasons
                .contains(&"read_only_plan_review".to_owned())
        );
        assert_eq!(summary.reason_codes(), summary.review_reasons);
        assert!(
            summary
                .detail_codes()
                .contains(&"read_only:semantic:cross_task_scope".to_owned())
        );
        assert!(summary.detail_codes().contains(
            &"read_only_detail:semantic:skip:experience:cross_task_scope:726f74".to_owned()
        ));
        assert!(
            summary
                .detail_codes()
                .contains(&"projection_parity:mismatch:replay_runs".to_owned())
        );
        assert!(
            summary
                .detail_codes()
                .contains(&"projection_parity:mismatch:index_quality_blockers".to_owned())
        );
        assert!(
            summary
                .detail_codes()
                .contains(&"projection_parity:mismatch:index_quality_warnings".to_owned())
        );
        assert!(
            summary
                .detail_codes()
                .contains(&"inspection_detail:info:evolution:context_rot_seen:1".to_owned())
        );
        assert!(summary.summary_line().contains("reason_codes="));
        assert!(summary.summary_line().contains("detail_codes="));
        assert!(summary.summary_line().contains("projection_parity_review"));
        assert!(
            summary
                .summary_line()
                .contains("read_only:semantic:cross_task_scope")
        );
        assert!(summary.summary_line().contains("parity_mismatches=4"));
    }

    #[test]
    fn service_dry_run_combines_plan_summary_and_phase_approvals() {
        let experiences = vec![
            ExperienceEnvelope::new("clean", "prompt", "Stable clean lesson")
                .with_scope(MemoryScope::for_task("runtime"))
                .with_clean_gist("A stable memory summary with enough useful detail.")
                .with_quality(0.9)
                .with_tags(vec!["adapter:test".to_owned()]),
        ];
        let memory_entries = vec![
            RetentionMemoryEntry::new("mem", "semantic durable memory", vec![0.1], 2.4)
                .with_feedback(4, 0)
                .with_access(1, 10),
        ];
        let adapters = vec![status(
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
            true,
            AdapterWriteMode::ReadOnly,
        )];
        let seed_ledger = MemoryEvolutionLedger {
            replay_runs: 1,
            replay_items: 1,
            replay_memory_updates: 1,
            ..MemoryEvolutionLedger::default()
        };
        let phases = [
            MemoryMigrationPhase::ReadOnlyShadow,
            MemoryMigrationPhase::IsolatedWrite,
        ];

        let dry_run = MemoryServiceDryRun::for_inputs(
            MemoryServiceShadowPlanInputs::new("clean_shadow", &experiences, &[], &memory_entries)
                .with_adapters(&adapters)
                .with_scope(&MemoryScope::for_task("runtime"))
                .with_evolution_ledger(seed_ledger),
            &MemoryMigrationEvidence::copied_fixture(1),
            &phases,
            false,
        );

        assert!(dry_run.summary.ready);
        assert!(!dry_run.requires_operator_review());
        assert_eq!(
            dry_run.approved_phases(),
            vec![
                MemoryMigrationPhase::ReadOnlyShadow,
                MemoryMigrationPhase::IsolatedWrite,
            ]
        );
        assert!(
            dry_run
                .approval_for(MemoryMigrationPhase::IsolatedWrite)
                .is_some_and(|approval| approval.approved)
        );
    }

    #[test]
    fn service_startup_evidence_collects_stable_summary_lines() {
        let experiences = vec![
            ExperienceEnvelope::new("clean", "prompt", "Stable clean lesson")
                .with_scope(MemoryScope::for_task("runtime"))
                .with_clean_gist("A stable memory summary with enough useful detail.")
                .with_quality(0.9)
                .with_tags(vec!["adapter:test".to_owned()]),
        ];
        let memory_entries = vec![
            RetentionMemoryEntry::new("mem", "semantic durable memory", vec![0.1], 2.4)
                .with_feedback(4, 0)
                .with_access(1, 10),
        ];
        let adapters = vec![status(
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
            true,
            AdapterWriteMode::ReadOnly,
        )];
        let seed_ledger = MemoryEvolutionLedger {
            replay_runs: 1,
            replay_items: 1,
            replay_memory_updates: 1,
            ..MemoryEvolutionLedger::default()
        };
        let contracts = vec![AdapterProjectionContract::experience_store_read_only(
            "experience_shadow",
            vec![
                crate::AdapterProjectionField::ExperienceId,
                crate::AdapterProjectionField::ExperiencePrompt,
                crate::AdapterProjectionField::ExperienceLesson,
                crate::AdapterProjectionField::ExperienceQuality,
                crate::AdapterProjectionField::ExperienceCleanGist,
                crate::AdapterProjectionField::ExperienceProjectionTags,
                crate::AdapterProjectionField::ExperienceTaskScope,
            ],
        )];

        let dry_run = MemoryServiceDryRun::for_inputs(
            MemoryServiceShadowPlanInputs::new("clean_shadow", &experiences, &[], &memory_entries)
                .with_adapters(&adapters)
                .with_scope(&MemoryScope::for_task("runtime"))
                .with_evolution_ledger(seed_ledger)
                .with_projection_contracts(&contracts, AdapterProjectionTarget::ShadowRead),
            &MemoryMigrationEvidence::copied_fixture(1),
            &[
                MemoryMigrationPhase::ReadOnlyShadow,
                MemoryMigrationPhase::IsolatedWrite,
            ],
            false,
        );
        let evidence = dry_run.startup_evidence();
        let text = evidence.summary_text();

        assert!(!evidence.requires_operator_review);
        assert_eq!(
            evidence.approved_phases,
            vec![
                MemoryMigrationPhase::ReadOnlyShadow,
                MemoryMigrationPhase::IsolatedWrite,
            ]
        );
        assert!(evidence.lines.len() >= 7);
        assert_eq!(
            dry_run.migration_evidence,
            MemoryMigrationEvidence::copied_fixture(1)
        );
        assert!(text.contains("memory_shadow ready=true"));
        assert!(text.contains(
            "memory_service_requirement profile=shadow_migration minimum_write_mode=read_only"
        ));
        assert!(text.contains(
            "memory_readiness profile=shadow_migration required_write_mode=read_only ready=true review=false missing=0 write_blockers=0 unhealthy=0 warnings=0 missing_codes=none write_blocker_codes=none warning_codes=none"
        ));
        assert!(text.contains(
            "memory_adapter_status name=all_shadow ready=true read_only=true write_mode=read_only"
        ));
        assert!(text.contains(
            "memory_capability_coverage capability=disk_kv_offload providers=all_shadow healthy=all_shadow writable=all_shadow read_only=all_shadow records=1 status_codes=none"
        ));
        assert!(text.contains("memory_read_only_plan adapter=clean_shadow"));
        assert!(text.contains("memory_read_only_plan adapter=clean_shadow write_mode=read_only review=false experiences=1 kv_shards=0"));
        assert!(text.contains("memory_read_only_plan adapter=clean_shadow write_mode=read_only review=false experiences=1 kv_shards=0 noisy=0 context_rot=0 rebuild_required=false rebuild_reasons=0 quality_gate_ready=true quality_gate_blockers=0 quality_gate_warnings=0 repair_items=0 repair_skipped=0 index_ops=1 index_skipped=0 semantic_matches=1 semantic_skipped=0"));
        assert!(text.contains("context_admit=1 context_summarize=0 context_reject=0 context_tokens=8 hot_gpu=0 warm_ram=0 cold_disk=0 kvswap_empty=true reason_codes=none"));
        assert!(text.contains("adapter_projection_audit shadow_ready=true isolated_write_ready=true experiences=1 kv_shards=0 issues=0 blockers=0 warnings=0 issue_codes=none"));
        assert!(text.contains(
            "memory_governance records=1 duplicate_groups=0 duplicate_records=0 noisy=0 context_rot=0 reason_codes=none detail_codes=none"
        ));
        assert!(text.contains(
            "memory_rebuild required=false duplicate_groups=0 refresh=0 compact=0 quarantine=0 missing_clean_gist=0 dirty_clean_gist=0 dirty_gist=0 reasons=0 reason_codes=none detail_codes=none"
        ));
        assert!(text.contains(
            "clean_gist_repair missing_clean_gist=0 dirty_clean_gist=0 dirty_gist=0 detail_codes=none"
        ));
        assert!(text.contains(
            "experience_index_quality_gate ready_for_context_injection=true records=1 blockers=0 warnings=0 duplicates=0 refresh=0 compact=0 quarantine=0 missing_clean_gist=0 dirty_clean_gist=0 dirty_gist=0 context_rot_blockers=0 reason_codes=none context_rot_blocker_reason_codes=none detail_codes=none"
        ));
        assert_eq!(evidence.clean_gist_repair_missing_clean_gist_count(), 0);
        assert_eq!(evidence.clean_gist_repair_dirty_clean_gist_count(), 0);
        assert_eq!(evidence.clean_gist_repair_dirty_gist_count(), 0);
        assert_eq!(
            evidence.clean_gist_repair_detail_codes(),
            Vec::<String>::new()
        );
        assert!(evidence.experience_index_quality_gate_ready());
        assert_eq!(evidence.experience_index_quality_gate_blocker_count(), 0);
        assert_eq!(evidence.experience_index_quality_gate_warning_count(), 0);
        assert_eq!(
            evidence.experience_index_quality_gate_reason_codes(),
            Vec::<String>::new()
        );
        assert!(text.contains(
            "memory_repair_plan empty=true items=0 skipped=0 repair_clean_gist=0 compact_context=0 quarantine=0 delete_duplicate=0 skipped_repair_clean_gist=0 skipped_compact_context=0 skipped_quarantine=0 skipped_delete_duplicate=0 reason_codes=none skipped_reason_codes=none"
        ));
        assert!(text.contains(
            "memory_index_plan rebuild=false operations=1 upsert=1 refresh=0 compact=0 quarantine=0 delete_duplicate=0 skipped=0 reasons=0 reason_codes=none"
        ));
        assert!(text.contains(
            "memory_context_injection decisions=1 admit=1 summarize=0 reject_budget=0 reject_risk=0 reject_scope=0 reject_score=0 accepted_risk=0 used_tokens=8 reason_codes=none detail_codes=none"
        ));
        assert!(text.contains("context_rot_risk risks=0 reason_codes=none detail_codes=none"));
        assert!(text.contains("memory_adapter_checklist satisfied=true"));
        assert!(text.contains(
            "memory_adapter_checklist_item code=capability_manifest_ready satisfied=true"
        ));
        assert!(text.contains(
            "memory_adapter_checklist_item code=evolution_gate_ready satisfied=true severity=blocker detail_codes=none"
        ));
        assert!(text.contains("memory_replay planned=0 reinforced=0 penalized=0 held=0"));
        assert!(text.contains("memory_evolution replay_runs=1"));
        assert!(text.contains("infini_memory local_window=0 global_memory=1 skipped=0 local_tokens=0 global_tokens=3 skipped_tokens=0 selected=1 reason_codes=global_memory"));
        assert!(
            text.contains(
                "memory_retention before=1 after_estimate=1 decays=0 removals=0 empty=true reason_codes=none detail_codes=none"
            )
        );
        assert!(text.contains(
            "memory_compaction before=1 after_estimate=1 merges=0 removals=0 skipped=not_enough_entries empty=true reason_codes=not_enough_entries detail_codes=skipped:not_enough_entries"
        ));
        assert!(text.contains("memory_inspection memories=1"));
        assert!(text.contains(
            "memory_projection_parity clean=true review=false mismatches=0 warnings=0 mismatch_fields=none warning_codes=none detail_codes=none"
        ));
        assert!(text.contains(
            "memory_migration_readiness isolated_write_ready=true review=false blockers=0 warnings=0 blocker_codes=none warning_codes=none"
        ));
        assert!(text.contains("memory_migration_evidence source_read_only=true copied_fixture=true isolated_write_root=true catalog_verified=true checksum_verified=true live_store_targeted=false records=1 guard_codes=none"));
        assert!(text.contains("kvswap_intent empty=true prefetch_promote=0 prefetch_missing=0 evict_demote=0 evict_keep_hot=0 target_hot_bytes=0 reason_codes=none"));
        assert!(text.contains("kvswap_prefetch promote=0 missing=0"));
        assert!(text.contains("kvswap_eviction target_hot_bytes=0 demote=0"));
        assert!(text.contains("adapter_projection_contract adapter=experience_shadow"));
        assert!(text.contains("adapter_projection adapter=experience_shadow"));
        assert!(text.contains("memory_migration phase=isolated_write approved=true"));
        assert!(evidence.is_complete());
        assert!(evidence.missing_required_line_prefixes().is_empty());
        assert!(evidence.has_line_prefix("memory_service_requirement "));
        assert!(evidence.has_line_prefix("memory_read_only_plan "));
        assert!(evidence.has_line_prefix("adapter_projection_contract "));
        assert!(evidence.has_line_prefix("memory_adapter_status "));
        assert!(evidence.has_line_prefix("memory_capability_coverage "));
        assert!(evidence.has_line_prefix("adapter_projection_audit "));
        assert!(evidence.has_line_prefix("clean_gist_repair "));
        assert!(evidence.has_line_prefix("experience_index_quality_gate "));
        assert!(evidence.has_line_prefix("memory_adapter_checklist_item "));
        assert!(evidence.has_line_prefix("memory_projection_parity "));
        assert!(evidence.has_line_prefix("memory_hygiene_work_plan "));
        assert!(evidence.has_line_prefix("memory_hygiene_work_queue "));
        assert!(evidence.has_line_prefix("memory_hygiene_dispatch_pressure "));
        assert!(evidence.has_line_prefix("context_rot_risk "));
        assert!(evidence.has_line_prefix("genome_repair_factor_plan "));
        assert_eq!(evidence.memory_hygiene_dispatch_pressure_rank(), 0);
        assert_eq!(
            evidence.memory_hygiene_dispatch_pressure_priority_codes(),
            vec!["clean".to_owned()]
        );
        assert_eq!(
            evidence.memory_hygiene_dispatch_pressure_reason_codes(),
            Vec::<String>::new()
        );
        assert_eq!(
            evidence.memory_hygiene_dispatch_pressure_reason_count("missing_clean_gist"),
            0
        );
        assert_eq!(
            evidence.memory_hygiene_dispatch_pressure_reason_count("kvswap_boundary_blocker"),
            0
        );
        assert_eq!(
            evidence.status_codes(),
            vec![
                "complete".to_owned(),
                "phases_approved".to_owned(),
                "review_clear".to_owned(),
            ]
        );
        assert_eq!(evidence.detail_codes(), Vec::<String>::new());
        assert_eq!(
            evidence.summary_line(),
            "memory_startup_evidence complete=true review=false approved_phases=2 lines=67 missing_required=0 projection_contracts=1 projection_contract_manifests=1 projection_contract_manifest_gap=0 missing_prefixes=none missing_codes=none context_rot_risks=0 context_rot_risk_reason_codes=none context_rot_blocker_reason_codes=none context_rot_risk_detail_codes=none migration_guard_codes=none migration_detail_codes=none kvswap_boundary_issues=0 kvswap_boundary_reason_codes=none kvswap_boundary_detail_codes=none status_codes=complete|phases_approved|review_clear detail_codes=none"
        );
    }

    #[test]
    fn service_startup_evidence_includes_clean_gist_selection_rows() {
        let prompt_secret = "GIST_SERVICE_PROMPT_SECRET_DO_NOT_LOG";
        let lesson_secret = "GIST_SERVICE_LESSON_SECRET_DO_NOT_LOG";
        let selected_gist_secret = "GIST_SERVICE_SELECTED_SECRET_DO_NOT_LOG";
        let transcript_gist_secret = "GIST_SERVICE_TRANSCRIPT_SECRET_DO_NOT_LOG";
        let metadata_gist_secret = "GIST_SERVICE_METADATA_SECRET_DO_NOT_LOG";
        let experiences = vec![
            ExperienceEnvelope::new("clean", prompt_secret, lesson_secret)
                .with_scope(MemoryScope::for_task("runtime"))
                .with_clean_gist("A stable runtime summary with enough useful detail.")
                .with_quality(0.9)
                .with_tags(vec!["adapter:test".to_owned()]),
        ];
        let memory_entries = vec![
            RetentionMemoryEntry::new("mem", "semantic durable memory", vec![0.1], 2.4)
                .with_feedback(4, 0)
                .with_access(1, 10),
        ];
        let gists = vec![
            MemoryGist::new(
                GistLevel::Section,
                "selected",
                format!("Use scoped clean gist selection. {selected_gist_secret}"),
            )
            .with_importance(0.7)
            .with_source_tokens(24),
            MemoryGist::new(
                GistLevel::Document,
                "transcript",
                format!("Conversation Transcript:\nUser: {transcript_gist_secret}\nAssistant: ok"),
            )
            .with_importance(0.9),
        ];
        let rejected_gists = vec![
            MemoryGist::new(GistLevel::Document, "empty", "   "),
            MemoryGist::new(
                GistLevel::Section,
                "metadata",
                format!("accepted_pattern quality=0.9 max_severity=watch {metadata_gist_secret}"),
            ),
            MemoryGist::new(GistLevel::Paragraph, "low-signal", "tiny"),
        ];
        let gist_reports = vec![
            DefaultCleanGistSelector::new().selection_report(&gists),
            DefaultCleanGistSelector::new().selection_report(&rejected_gists),
        ];

        let dry_run = MemoryServiceDryRun::for_inputs(
            MemoryServiceShadowPlanInputs::new("gist_shadow", &experiences, &[], &memory_entries)
                .with_scope(&MemoryScope::for_task("runtime"))
                .with_clean_gist_selection_reports(&gist_reports),
            &MemoryMigrationEvidence::read_only_source(Some(1)),
            &[MemoryMigrationPhase::ReadOnlyShadow],
            false,
        );
        let evidence = dry_run.startup_evidence();
        let text = evidence.summary_text();

        assert!(evidence.has_line_prefix("clean_gist_selection "));
        assert_eq!(evidence.clean_gist_selection_report_count(), 2);
        assert_eq!(evidence.clean_gist_selection_candidate_count(), 5);
        assert_eq!(evidence.clean_gist_selection_selected_count(), 1);
        assert_eq!(evidence.clean_gist_selection_no_selection_count(), 1);
        assert_eq!(
            evidence.clean_gist_selection_selected_level_codes(),
            vec!["section".to_owned()]
        );
        assert_eq!(
            evidence.clean_gist_selection_selected_level_count("section"),
            1
        );
        assert_eq!(
            evidence.clean_gist_selection_selected_level_count("document"),
            0
        );
        assert_eq!(evidence.clean_gist_selection_rejected_empty_count(), 1);
        assert_eq!(evidence.clean_gist_selection_rejected_transcript_count(), 1);
        assert_eq!(evidence.clean_gist_selection_rejected_metadata_count(), 1);
        assert_eq!(evidence.clean_gist_selection_rejected_low_signal_count(), 1);
        assert_eq!(evidence.clean_gist_selection_reason_count("selected"), 1);
        assert_eq!(
            evidence.clean_gist_selection_reason_count("no_selection"),
            1
        );
        assert_eq!(evidence.clean_gist_selection_reason_count("missing"), 0);
        assert_eq!(
            evidence.clean_gist_selection_reason_codes(),
            vec![
                "no_selection".to_owned(),
                "rejected_empty".to_owned(),
                "rejected_low_signal".to_owned(),
                "rejected_metadata".to_owned(),
                "rejected_transcript".to_owned(),
                "selected".to_owned(),
            ]
        );
        assert_eq!(
            evidence.clean_gist_selection_detail_codes(),
            vec![
                "rejected_empty".to_owned(),
                "rejected_low_signal".to_owned(),
                "rejected_metadata".to_owned(),
                "rejected_transcript".to_owned(),
                "selected:none".to_owned(),
                "selected_level:section".to_owned(),
            ]
        );
        assert_eq!(
            evidence.clean_gist_selection_detail_codes_for("selected_level"),
            vec!["selected_level:section".to_owned()]
        );
        assert_eq!(
            evidence.clean_gist_selection_detail_codes_for("selected"),
            vec!["selected:none".to_owned()]
        );
        assert!(text.contains("clean_gist_selection candidates=2 selected=true"));
        assert!(text.contains("clean_gist_selection candidates=3 selected=false"));
        assert!(text.contains("reason_codes=rejected_transcript|selected"));
        assert!(text.contains(
            "reason_codes=no_selection|rejected_empty|rejected_low_signal|rejected_metadata"
        ));
        assert!(text.contains("detail_codes=rejected_transcript|selected_level:section"));
        assert!(text.contains(
            "detail_codes=rejected_empty|rejected_low_signal|rejected_metadata|selected:none"
        ));
        let startup_detail_codes = evidence.detail_codes();
        assert!(startup_detail_codes.contains(&"rejected_empty".to_owned()));
        assert!(startup_detail_codes.contains(&"rejected_low_signal".to_owned()));
        assert!(startup_detail_codes.contains(&"rejected_metadata".to_owned()));
        assert!(startup_detail_codes.contains(&"rejected_transcript".to_owned()));
        assert!(startup_detail_codes.contains(&"selected:none".to_owned()));
        assert!(startup_detail_codes.contains(&"selected_level:section".to_owned()));
        for forbidden in [
            prompt_secret,
            lesson_secret,
            selected_gist_secret,
            transcript_gist_secret,
            metadata_gist_secret,
        ] {
            assert!(
                !text.contains(forbidden),
                "startup evidence leaked clean gist payload: {forbidden}"
            );
        }
    }

    #[test]
    fn service_startup_evidence_keeps_infini_and_kvswap_payload_safe() {
        let prompt_secret = "SERVICE_PROMPT_SECRET_DO_NOT_LOG";
        let lesson_secret = "SERVICE_LESSON_SECRET_DO_NOT_LOG";
        let local_key_secret = "SERVICE_INFINI_ACTIVE_KEY_DO_NOT_LOG";
        let global_key_secret = "SERVICE_INFINI_GLOBAL_KEY_DO_NOT_LOG";
        let promote_shard_id = "SERVICE_KVSWAP_PROMOTE_SHARD_DO_NOT_LOG";
        let demote_shard_id = "SERVICE_KVSWAP_DEMOTE_SHARD_DO_NOT_LOG";
        let experiences = vec![
            ExperienceEnvelope::new("clean", prompt_secret, lesson_secret)
                .with_scope(MemoryScope::for_task("runtime"))
                .with_clean_gist("A stable runtime summary with enough useful detail.")
                .with_quality(0.9)
                .with_tags(vec!["adapter:test".to_owned()]),
        ];
        let kv_metadata = vec![
            KvShardMetadata {
                id: promote_shard_id.to_owned(),
                byte_len: 4,
                checksum: 7,
                tier: KvTier::Cold,
                priority: 0.95,
                last_access: 20,
            },
            KvShardMetadata {
                id: demote_shard_id.to_owned(),
                byte_len: 4,
                checksum: 11,
                tier: KvTier::Hot,
                priority: 0.10,
                last_access: 1,
            },
        ];
        let memory_entries = vec![
            RetentionMemoryEntry::new("global-safe-id", global_key_secret, vec![0.1], 2.4)
                .with_feedback(4, 0)
                .with_access(1, 10),
        ];
        let active_matches = vec![InfiniMemoryActiveMatch::new(
            "local-safe-id",
            local_key_secret,
            vec![0.3, 0.4],
            0.9,
            1.5,
        )];
        let previous_placement = TieredMemoryPlan::new(vec![
            crate::MemoryPlacement {
                id: promote_shard_id.to_owned(),
                tier: crate::MemoryTier::ColdDisk,
                score: 0.2,
                reason: "previous_cold".to_owned(),
            },
            crate::MemoryPlacement {
                id: demote_shard_id.to_owned(),
                tier: crate::MemoryTier::HotGpu,
                score: 0.9,
                reason: "previous_hot".to_owned(),
            },
        ]);
        let adapters = vec![status(
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
            true,
            AdapterWriteMode::ReadOnly,
        )];
        let seed_ledger = MemoryEvolutionLedger {
            replay_runs: 1,
            replay_items: 1,
            replay_memory_updates: 1,
            ..MemoryEvolutionLedger::default()
        };

        let dry_run = MemoryServiceDryRun::for_inputs(
            MemoryServiceShadowPlanInputs::new(
                "payload_safe_shadow",
                &experiences,
                &kv_metadata,
                &memory_entries,
            )
            .with_active_matches(&active_matches)
            .with_adapters(&adapters)
            .with_scope(&MemoryScope::for_task("runtime"))
            .with_tier_plan(TierBudgets::new(4, 0), Some(&previous_placement), 2048)
            .with_evolution_ledger(seed_ledger),
            &MemoryMigrationEvidence::read_only_source(Some(1)),
            &[MemoryMigrationPhase::ReadOnlyShadow],
            false,
        );
        let evidence = dry_run.startup_evidence();
        let text = evidence.summary_text();
        let summary_line = evidence.summary_line();
        let detail_codes = evidence.detail_codes();

        assert!(dry_run.requires_operator_review());
        assert!(text.contains("infini_memory local_window=1 global_memory=1"));
        assert!(text.contains("local_window:6c6f63616c2d736166652d6964"));
        assert!(text.contains("global_memory:676c6f62616c2d736166652d6964"));
        assert_eq!(
            evidence.infini_memory_reason_codes(),
            vec!["global_memory".to_owned(), "local_window".to_owned()]
        );
        assert_eq!(
            evidence.infini_memory_detail_codes(),
            vec![
                "global_memory:676c6f62616c2d736166652d6964".to_owned(),
                "local_window:6c6f63616c2d736166652d6964".to_owned(),
            ]
        );
        assert!(text.contains("kvswap_intent empty=false prefetch_promote=1"));
        assert!(text.contains("evict_demote=1"));
        assert_eq!(
            evidence.kvswap_intent_reason_codes(),
            vec![
                "evict_demote".to_owned(),
                "prefetch_promote".to_owned(),
                "tiered_memory_demotions".to_owned(),
                "tiered_memory_promotions".to_owned(),
            ]
        );
        assert!(text.contains(
            "promote_id_hex=534552564943455f4b56535741505f50524f4d4f54455f53484152445f444f5f4e4f545f4c4f47"
        ));
        assert!(text.contains(
            "demote_id_hex=534552564943455f4b56535741505f44454d4f54455f53484152445f444f5f4e4f545f4c4f47"
        ));
        assert_eq!(
            evidence.kvswap_action_detail_codes(),
            vec![
                "eviction:demote:tiered_memory_demotions:534552564943455f4b56535741505f44454d4f54455f53484152445f444f5f4e4f545f4c4f47".to_owned(),
                "prefetch:promote:tiered_memory_promotions:534552564943455f4b56535741505f50524f4d4f54455f53484152445f444f5f4e4f545f4c4f47".to_owned(),
            ]
        );
        assert!(detail_codes.contains(
            &"read_only_detail:kvswap:prefetch:promote:tiered_memory_promotions:534552564943455f4b56535741505f50524f4d4f54455f53484152445f444f5f4e4f545f4c4f47"
                .to_owned()
        ));
        assert!(detail_codes.contains(
            &"read_only_detail:kvswap:eviction:demote:tiered_memory_demotions:534552564943455f4b56535741505f44454d4f54455f53484152445f444f5f4e4f545f4c4f47"
                .to_owned()
        ));
        assert!(summary_line.contains("kvswap:prefetch_promote"));
        assert!(summary_line.contains("kvswap:evict_demote"));
        for forbidden in [
            prompt_secret,
            lesson_secret,
            local_key_secret,
            global_key_secret,
            promote_shard_id,
            demote_shard_id,
        ] {
            assert!(
                !text.contains(forbidden),
                "startup evidence leaked payload text: {forbidden}"
            );
            assert!(
                !summary_line.contains(forbidden),
                "startup summary leaked payload text: {forbidden}"
            );
            assert!(
                !detail_codes.iter().any(|code| code.contains(forbidden)),
                "startup detail codes leaked payload text: {forbidden}"
            );
        }
    }

    #[test]
    fn startup_evidence_aggregates_infini_retention_and_compaction_rows() {
        let memory_payload_secret = "LONG_TERM_MEMORY_PAYLOAD_SECRET_DO_NOT_LOG";
        let evidence = MemoryServiceStartupEvidence {
            requires_operator_review: true,
            approved_phases: vec![MemoryMigrationPhase::ReadOnlyShadow],
            lines: vec![
                "infini_memory local_window=1 global_memory=1 skipped=1 local_tokens=3 global_tokens=5 skipped_tokens=2 selected=2 reason_codes=global_memory|local_window|sparse_filter:low_score detail_codes=global_memory:676c6f62616c|local_window:6c6f63616c|skipped:sparse_filter:low_score:7765616b".to_owned(),
                "infini_memory local_window=0 global_memory=1 skipped=1 local_tokens=0 global_tokens=4 skipped_tokens=2 selected=1 reason_codes=global_memory|sparse_filter:missing_entry detail_codes=global_memory:676c6f62616c2d32|skipped:sparse_filter:missing_entry:6d697373696e67".to_owned(),
                "memory_retention before=2 after_estimate=1 decays=1 removals=1 empty=false reason_codes=stale_decay|weak_stale_and_repeated_failures detail_codes=decay:stale_decay:7374616c65|remove:weak_stale_and_repeated_failures:6661696c6564".to_owned(),
                "memory_retention before=1 after_estimate=1 decays=1 removals=0 empty=false reason_codes=stale_decay detail_codes=decay:stale_decay:7374616c652d32".to_owned(),
                "memory_compaction before=3 after_estimate=2 merges=1 removals=1 skipped=none empty=false reason_codes=same_namespace_high_similarity detail_codes=merge:same_namespace_high_similarity:7374726f6e67:7765616b|remove:7765616b".to_owned(),
                "memory_compaction before=2 after_estimate=2 merges=0 removals=0 skipped=policy_disabled empty=true reason_codes=policy_disabled detail_codes=skipped:policy_disabled".to_owned(),
            ],
        };

        assert_eq!(evidence.infini_memory_local_window_count(), 1);
        assert_eq!(evidence.infini_memory_global_memory_count(), 2);
        assert_eq!(evidence.infini_memory_skipped_count(), 2);
        assert_eq!(evidence.infini_memory_local_token_count(), 3);
        assert_eq!(evidence.infini_memory_global_token_count(), 9);
        assert_eq!(evidence.infini_memory_skipped_token_count(), 4);
        assert_eq!(evidence.infini_memory_selected_count(), 3);
        assert_eq!(
            evidence.infini_memory_reason_codes(),
            vec![
                "global_memory".to_owned(),
                "local_window".to_owned(),
                "sparse_filter:low_score".to_owned(),
                "sparse_filter:missing_entry".to_owned(),
            ]
        );
        assert_eq!(
            evidence.infini_memory_detail_codes(),
            vec![
                "global_memory:676c6f62616c".to_owned(),
                "global_memory:676c6f62616c2d32".to_owned(),
                "local_window:6c6f63616c".to_owned(),
                "skipped:sparse_filter:low_score:7765616b".to_owned(),
                "skipped:sparse_filter:missing_entry:6d697373696e67".to_owned(),
            ]
        );
        assert_eq!(
            evidence.infini_memory_detail_codes_for_scope("local_window"),
            vec!["local_window:6c6f63616c".to_owned()]
        );
        assert_eq!(
            evidence.infini_memory_detail_codes_for_scope("global_memory"),
            vec![
                "global_memory:676c6f62616c".to_owned(),
                "global_memory:676c6f62616c2d32".to_owned(),
            ]
        );
        assert_eq!(
            evidence.infini_memory_skipped_detail_codes(),
            vec![
                "skipped:sparse_filter:low_score:7765616b".to_owned(),
                "skipped:sparse_filter:missing_entry:6d697373696e67".to_owned(),
            ]
        );
        assert_eq!(
            evidence.infini_memory_skipped_detail_codes_for_reason("sparse_filter:missing_entry"),
            vec!["skipped:sparse_filter:missing_entry:6d697373696e67".to_owned()]
        );
        assert_eq!(
            evidence.infini_memory_skipped_detail_codes_for_reason("sparse_filter:token_budget"),
            Vec::<String>::new()
        );

        assert_eq!(evidence.memory_retention_before_count(), 3);
        assert_eq!(evidence.memory_retention_after_estimate_count(), 2);
        assert_eq!(evidence.memory_retention_decay_count(), 2);
        assert_eq!(evidence.memory_retention_removal_count(), 1);
        assert_eq!(
            evidence.memory_retention_reason_codes(),
            vec![
                "stale_decay".to_owned(),
                "weak_stale_and_repeated_failures".to_owned(),
            ]
        );
        assert_eq!(
            evidence.memory_retention_detail_codes(),
            vec![
                "decay:stale_decay:7374616c65".to_owned(),
                "decay:stale_decay:7374616c652d32".to_owned(),
                "remove:weak_stale_and_repeated_failures:6661696c6564".to_owned(),
            ]
        );

        assert_eq!(evidence.memory_compaction_before_count(), 5);
        assert_eq!(evidence.memory_compaction_after_estimate_count(), 4);
        assert_eq!(evidence.memory_compaction_merge_count(), 1);
        assert_eq!(evidence.memory_compaction_removal_count(), 1);
        assert_eq!(
            evidence.memory_compaction_skipped_codes(),
            vec!["policy_disabled".to_owned()]
        );
        assert_eq!(
            evidence.memory_compaction_reason_codes(),
            vec![
                "policy_disabled".to_owned(),
                "same_namespace_high_similarity".to_owned(),
            ]
        );
        assert_eq!(
            evidence.memory_compaction_detail_codes(),
            vec![
                "merge:same_namespace_high_similarity:7374726f6e67:7765616b".to_owned(),
                "remove:7765616b".to_owned(),
                "skipped:policy_disabled".to_owned(),
            ]
        );
        assert!(!evidence.summary_text().contains(memory_payload_secret));
        assert!(!evidence.summary_line().contains(memory_payload_secret));
    }

    #[test]
    fn service_startup_evidence_keeps_kvswap_boundary_payload_safe() {
        let prompt_secret = "BOUNDARY_PROMPT_SECRET_DO_NOT_LOG";
        let lesson_secret = "BOUNDARY_LESSON_SECRET_DO_NOT_LOG";
        let overlap_id = "BOUNDARY_OVERLAP_SHARD_DO_NOT_LOG";
        let missing_metadata_id = "BOUNDARY_MISSING_HOT_METADATA_DO_NOT_LOG";
        let stale_metadata_id = "BOUNDARY_STALE_METADATA_DO_NOT_LOG";
        let hot_mismatch_id = "BOUNDARY_HOT_TIER_MISMATCH_DO_NOT_LOG";
        let cold_mismatch_id = "BOUNDARY_COLD_TIER_MISMATCH_DO_NOT_LOG";
        let experiences = vec![
            ExperienceEnvelope::new("clean", prompt_secret, lesson_secret)
                .with_scope(MemoryScope::for_task("runtime"))
                .with_clean_gist("A stable runtime summary with enough useful detail.")
                .with_quality(0.9)
                .with_tags(vec!["adapter:test".to_owned()]),
        ];
        let memory_entries = vec![
            RetentionMemoryEntry::new("mem", "semantic durable memory", vec![0.1], 2.4)
                .with_feedback(4, 0)
                .with_access(1, 10),
        ];
        let adapters = vec![status(
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
            true,
            AdapterWriteMode::ReadOnly,
        )];
        let seed_ledger = MemoryEvolutionLedger {
            replay_runs: 1,
            replay_items: 1,
            replay_memory_updates: 1,
            ..MemoryEvolutionLedger::default()
        };
        let boundary = KvSwapBoundaryAudit {
            overlapping_hot_cold_ids: vec![overlap_id.to_owned()],
            missing_hot_metadata_ids: vec![missing_metadata_id.to_owned()],
            stale_metadata_ids: vec![stale_metadata_id.to_owned()],
            hot_tier_mismatch_ids: vec![hot_mismatch_id.to_owned()],
            cold_tier_mismatch_ids: vec![cold_mismatch_id.to_owned()],
        };

        let dry_run = MemoryServiceDryRun::for_inputs(
            MemoryServiceShadowPlanInputs::new(
                "boundary_payload_safe_shadow",
                &experiences,
                &[],
                &memory_entries,
            )
            .with_adapters(&adapters)
            .with_scope(&MemoryScope::for_task("runtime"))
            .with_evolution_ledger(seed_ledger)
            .with_kvswap_boundary(boundary),
            &MemoryMigrationEvidence::read_only_source(Some(1)),
            &[MemoryMigrationPhase::ReadOnlyShadow],
            false,
        );
        let evidence = dry_run.startup_evidence();
        let text = evidence.summary_text();
        let summary_line = evidence.summary_line();
        let detail_codes = evidence.detail_codes();
        let hex = |id: &str| {
            id.as_bytes()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        };
        let expected_detail_codes = vec![
            format!("cold_tier_mismatch:{}", hex(cold_mismatch_id)),
            format!("hot_tier_mismatch:{}", hex(hot_mismatch_id)),
            format!("missing_hot_metadata:{}", hex(missing_metadata_id)),
            format!("overlap:{}", hex(overlap_id)),
            format!("stale_metadata:{}", hex(stale_metadata_id)),
        ];

        assert!(dry_run.requires_operator_review());
        assert_eq!(evidence.kvswap_boundary_issue_count(), 5);
        assert_eq!(
            evidence.kvswap_boundary_reason_codes(),
            vec![
                "cold_tier_mismatch".to_owned(),
                "hot_tier_mismatch".to_owned(),
                "missing_hot_metadata".to_owned(),
                "overlapping_hot_cold".to_owned(),
                "stale_metadata".to_owned(),
            ]
        );
        assert_eq!(
            evidence.kvswap_boundary_detail_codes(),
            expected_detail_codes
        );
        assert_eq!(evidence.kvswap_boundary_readiness_report_count(), 1);
        assert_eq!(evidence.kvswap_boundary_ready_count(), 0);
        assert_eq!(evidence.kvswap_boundary_blocker_count(), 2);
        assert_eq!(evidence.kvswap_boundary_warning_count(), 3);
        assert_eq!(
            evidence.kvswap_boundary_blocker_reason_codes(),
            vec![
                "missing_hot_metadata".to_owned(),
                "overlapping_hot_cold".to_owned()
            ]
        );
        assert_eq!(
            evidence.kvswap_boundary_warning_reason_codes(),
            vec![
                "cold_tier_mismatch".to_owned(),
                "hot_tier_mismatch".to_owned(),
                "stale_metadata".to_owned()
            ]
        );
        assert!(
            evidence
                .kvswap_boundary_readiness_detail_codes()
                .contains(&format!(
                    "blocker:missing_hot_metadata:{}",
                    hex(missing_metadata_id)
                ))
        );
        assert!(
            evidence
                .kvswap_boundary_readiness_detail_codes()
                .contains(&format!("blocker:overlap:{}", hex(overlap_id)))
        );
        assert!(
            evidence
                .kvswap_boundary_readiness_detail_codes()
                .contains(&format!(
                    "warning:cold_tier_mismatch:{}",
                    hex(cold_mismatch_id)
                ))
        );
        assert!(summary_line.contains("kvswap_boundary_issues=5"));
        assert!(summary_line.contains(
            "kvswap_boundary_reason_codes=cold_tier_mismatch|hot_tier_mismatch|missing_hot_metadata|overlapping_hot_cold|stale_metadata"
        ));
        for expected in evidence.kvswap_boundary_detail_codes() {
            assert!(
                detail_codes.contains(&expected),
                "startup detail codes omitted boundary detail: {expected}"
            );
            assert!(
                summary_line.contains(&expected),
                "startup summary omitted boundary detail: {expected}"
            );
        }
        for forbidden in [
            prompt_secret,
            lesson_secret,
            overlap_id,
            missing_metadata_id,
            stale_metadata_id,
            hot_mismatch_id,
            cold_mismatch_id,
        ] {
            assert!(
                !text.contains(forbidden),
                "startup evidence leaked boundary payload: {forbidden}"
            );
            assert!(
                !summary_line.contains(forbidden),
                "startup summary leaked boundary payload: {forbidden}"
            );
            assert!(
                !detail_codes.iter().any(|code| code.contains(forbidden)),
                "startup detail codes leaked boundary payload: {forbidden}"
            );
        }
    }

    #[test]
    fn startup_evidence_reports_missing_core_lines() {
        let evidence = MemoryServiceStartupEvidence {
            requires_operator_review: true,
            approved_phases: Vec::new(),
            lines: vec![
                "memory_shadow ready=false".to_owned(),
                "memory_adapter_checklist satisfied=false".to_owned(),
            ],
        };

        let missing = evidence.missing_required_line_prefixes();

        assert!(!evidence.is_complete());
        assert!(evidence.has_line_prefix("memory_shadow "));
        assert!(missing.contains(&"memory_service_requirement "));
        assert!(missing.contains(&"memory_read_only_plan "));
        assert!(missing.contains(&"memory_adapter_status "));
        assert!(missing.contains(&"memory_capability_coverage "));
        assert!(missing.contains(&"adapter_projection_audit "));
        assert!(missing.contains(&"memory_adapter_checklist_item "));
        assert!(missing.contains(&"memory_projection_parity "));
        assert!(missing.contains(&"memory_migration_evidence "));
        assert!(missing.contains(&"memory_hygiene_work_plan "));
        assert!(missing.contains(&"memory_hygiene_work_queue "));
        assert!(missing.contains(&"context_rot_risk "));
        assert!(evidence.summary_line().contains("complete=false"));
        assert!(evidence.summary_line().contains("missing_required=29"));
        assert_eq!(
            evidence.status_codes(),
            vec![
                "incomplete_evidence".to_owned(),
                "no_approved_phases".to_owned(),
                "operator_review_required".to_owned(),
            ]
        );
        assert!(
            evidence
                .missing_required_codes()
                .contains(&"memory_readiness".to_owned())
        );
        assert!(
            evidence
                .detail_codes()
                .contains(&"missing_line:memory_readiness".to_owned())
        );
        assert!(
            evidence
                .detail_codes()
                .contains(&"incomplete_evidence".to_owned())
        );
        assert!(
            evidence
                .detail_codes()
                .contains(&"operator_review_required".to_owned())
        );
        assert!(
            evidence
                .detail_codes()
                .contains(&"approved_phases:none".to_owned())
        );
        assert!(evidence.summary_line().contains("missing_codes="));
        assert!(evidence.summary_line().contains(
            "status_codes=incomplete_evidence|no_approved_phases|operator_review_required"
        ));
        assert!(evidence.summary_line().contains("memory_readiness"));
        assert!(evidence.summary_line().contains("detail_codes="));
        assert!(
            evidence
                .summary_line()
                .contains("missing_line:memory_readiness")
        );
    }

    #[test]
    fn startup_evidence_lifts_migration_evidence_guard_detail_codes() {
        let evidence = MemoryServiceStartupEvidence {
            requires_operator_review: true,
            approved_phases: vec![MemoryMigrationPhase::ReadOnlyShadow],
            lines: vec![MemoryMigrationEvidence::read_only_source(Some(7)).summary_line()],
        };

        assert!(
            evidence
                .detail_codes()
                .contains(&"guard:copied_fixture_missing".to_owned())
        );
        assert!(
            evidence
                .detail_codes()
                .contains(&"guard:fixture_catalog_not_verified".to_owned())
        );
        assert!(
            evidence
                .detail_codes()
                .contains(&"guard:fixture_checksum_not_verified".to_owned())
        );
        assert!(
            evidence
                .detail_codes()
                .contains(&"guard:isolated_write_root_missing".to_owned())
        );
        assert!(
            evidence
                .summary_line()
                .contains("guard:copied_fixture_missing")
        );

        let verification = DiskKvCatalogVerification {
            missing_byte_ids: vec!["missing".to_owned()],
            byte_len_mismatch_ids: vec!["stale-len".to_owned()],
            checksum_mismatch_ids: vec!["corrupt".to_owned()],
            ..DiskKvCatalogVerification::default()
        };
        let evidence = MemoryServiceStartupEvidence {
            requires_operator_review: true,
            approved_phases: vec![MemoryMigrationPhase::ReadOnlyShadow],
            lines: vec![
                MemoryMigrationEvidence::copied_disk_kv_fixture(&verification).summary_line(),
            ],
        };

        assert!(
            evidence
                .detail_codes()
                .contains(&"disk_kv_catalog:missing_bytes:6d697373696e67".to_owned())
        );
        assert!(
            evidence
                .detail_codes()
                .contains(&"disk_kv_catalog:checksum_mismatch:636f7272757074".to_owned())
        );
        assert!(
            evidence
                .summary_line()
                .contains("disk_kv_catalog:missing_bytes:6d697373696e67")
        );
    }

    #[test]
    fn startup_evidence_aggregates_replay_and_evolution_rows() {
        let replay_payload_secret = "REPLAY_PAYLOAD_SECRET_DO_NOT_LOG";
        let evidence = MemoryServiceStartupEvidence {
            requires_operator_review: true,
            approved_phases: vec![MemoryMigrationPhase::ReadOnlyShadow],
            lines: vec![
                "memory_replay planned=2 reinforced=1 penalized=1 held=0 touched_memories=2 memory_reinforcements=2 memory_penalties=1 feedback_items=1 feedback_updates=1 feedback_applied=1 feedback_removed=0 feedback_missing=0 average_reward=0.500 recursive_runtime=1 live_memory_feedback=0 rust_check=1 business_contract=0 context_rot=1 reason_codes=action_penalize|action_reinforce|context_rot|memory_penalty|memory_reinforcement|recursive_runtime|rust_check detail_codes=item:penalize:70656e616c697a65|item:reinforce:7265696e666f726365|memory_update:penalize:6d32:70656e616c697a65|memory_update:reinforce:6d31:7265696e666f726365|memory_update:reinforce:6d32:7265696e666f726365|signal:context_rot:70656e616c697a65|signal:recursive_runtime:7265696e666f726365|signal:rust_check:7265696e666f726365".to_owned(),
                "memory_replay planned=1 reinforced=0 penalized=0 held=1 touched_memories=1 memory_reinforcements=0 memory_penalties=0 feedback_items=2 feedback_updates=1 feedback_applied=0 feedback_removed=1 feedback_missing=1 average_reward=0.000 recursive_runtime=0 live_memory_feedback=1 rust_check=0 business_contract=1 context_rot=0 reason_codes=action_hold|business_contract|feedback_missing|feedback_removed|live_memory_feedback detail_codes=feedback:missing:6d697373696e67|feedback:removed:72656d6f766564|item:hold:68656c64|signal:business_contract:68656c64|signal:live_memory_feedback:68656c64".to_owned(),
                "memory_evolution replay_runs=1 replay_items=3 replay_updates=3 replay_missing=1 invalid_memory_ids=1 context_rot_items=1 live_feedback_items=1 retention_decays=2 retention_removals=1 compaction_merges=1 compaction_removals=1 external_applied=1 external_missing=0 drift_rollbacks=0 index_quality_blockers=1 index_quality_warnings=2 kvswap_boundary_blockers=1 kvswap_boundary_warnings=1 hygiene_pressure_score=235 hygiene_pressure_priority=quarantine hygiene_pressure_action_lanes=experience_index_rebuild|kvswap_boundary_repair|context_rot_review hygiene_pressure_action_lane_details=experience_index_rebuild:quarantine:120:3|kvswap_boundary_repair:quarantine:110:2|context_rot_review:repair:5:1 hygiene_work_next_action=experience_index_rebuild hygiene_work_operator_review=true hygiene_work_isolation=true hygiene_pressure_reason_codes=context_rot_pressure|index_quality_blocker|index_quality_warning|kvswap_boundary_blocker|kvswap_boundary_warning hygiene_pressure_detail_codes=context_rot_items:1|index_quality_blockers:1|index_quality_warnings:2|kvswap_boundary_blockers:1|kvswap_boundary_warnings:1 reason_codes=context_rot|external_feedback_applied|invalid_memory_id|live_memory_feedback|replay_evidence|replay_memory_update|replay_missing_memory|retention_decay|retention_removal|compaction_merge|compaction_removal|index_quality_blocker|index_quality_warning|kvswap_boundary_blocker|kvswap_boundary_warning".to_owned(),
                "memory_evolution replay_runs=1 replay_items=1 replay_updates=0 replay_missing=1 invalid_memory_ids=0 context_rot_items=0 live_feedback_items=0 retention_decays=0 retention_removals=0 compaction_merges=0 compaction_removals=0 external_applied=0 external_missing=1 drift_rollbacks=1 index_quality_blockers=0 index_quality_warnings=0 kvswap_boundary_blockers=0 kvswap_boundary_warnings=0 hygiene_pressure_score=0 hygiene_pressure_priority=clean hygiene_pressure_action_lanes=none hygiene_pressure_action_lane_details=none hygiene_work_next_action=none hygiene_work_operator_review=false hygiene_work_isolation=false hygiene_pressure_reason_codes=none hygiene_pressure_detail_codes=none reason_codes=drift_rollback|external_feedback_missing|replay_evidence|replay_missing_memory".to_owned(),
                "memory_hygiene_work_plan clean=false total_score=235 lanes=3 next_action=experience_index_rebuild operator_review=true isolation=true action_lanes=experience_index_rebuild|kvswap_boundary_repair|context_rot_review action_lane_details=experience_index_rebuild:quarantine:120:3|kvswap_boundary_repair:quarantine:110:2|context_rot_review:repair:5:1 dispatch_next=dispatch:operator_review:isolated:experience_index_rebuild:quarantine:120:3 dispatch_codes=dispatch:operator_review:isolated:experience_index_rebuild:quarantine:120:3|dispatch:operator_review:isolated:kvswap_boundary_repair:quarantine:110:2|dispatch:operator_review:isolated:context_rot_review:repair:5:1 reason_codes=operator_review|isolation_recommended|experience_index_rebuild|kvswap_boundary_repair|context_rot_review detail_codes=next:experience_index_rebuild|score:235|lanes:3|lane:experience_index_rebuild:quarantine:120:3|lane:kvswap_boundary_repair:quarantine:110:2|lane:context_rot_review:repair:5:1".to_owned(),
                "memory_hygiene_work_queue clean=false total_score=235 items=3 operator_review=3 isolation=3 next_dispatch=dispatch:operator_review:isolated:experience_index_rebuild:quarantine:120:3 lanes=experience_index_rebuild|kvswap_boundary_repair|context_rot_review priorities=quarantine|repair dispatch_codes=dispatch:operator_review:isolated:experience_index_rebuild:quarantine:120:3|dispatch:operator_review:isolated:kvswap_boundary_repair:quarantine:110:2|dispatch:operator_review:isolated:context_rot_review:repair:5:1 detail_codes=experience_index_rebuild:quarantine:120:3|kvswap_boundary_repair:quarantine:110:2|context_rot_review:repair:5:1 reason_codes=isolation_recommended|items_present|operator_review_required".to_owned(),
                "memory_hygiene_work_item lane=experience_index_rebuild priority=quarantine score=120 items=3 operator_review=true isolation=true dispatch_code=dispatch:operator_review:isolated:experience_index_rebuild:quarantine:120:3 detail_code=experience_index_rebuild:quarantine:120:3".to_owned(),
                "memory_hygiene_work_item lane=kvswap_boundary_repair priority=quarantine score=110 items=2 operator_review=true isolation=true dispatch_code=dispatch:operator_review:isolated:kvswap_boundary_repair:quarantine:110:2 detail_code=kvswap_boundary_repair:quarantine:110:2".to_owned(),
                "memory_hygiene_work_item lane=context_rot_review priority=repair score=5 items=1 operator_review=true isolation=true dispatch_code=dispatch:operator_review:isolated:context_rot_review:repair:5:1 detail_code=context_rot_review:repair:5:1".to_owned(),
            ],
        };

        assert_eq!(evidence.memory_replay_planned_count(), 3);
        assert_eq!(evidence.memory_replay_reinforced_count(), 1);
        assert_eq!(evidence.memory_replay_penalized_count(), 1);
        assert_eq!(evidence.memory_replay_held_count(), 1);
        assert_eq!(evidence.memory_replay_touched_memory_count(), 3);
        assert_eq!(evidence.memory_replay_reinforcement_count(), 2);
        assert_eq!(evidence.memory_replay_penalty_count(), 1);
        assert_eq!(evidence.memory_replay_feedback_item_count(), 3);
        assert_eq!(evidence.memory_replay_feedback_update_count(), 2);
        assert_eq!(evidence.memory_replay_feedback_applied_count(), 1);
        assert_eq!(evidence.memory_replay_feedback_removed_count(), 1);
        assert_eq!(evidence.memory_replay_feedback_missing_count(), 1);
        assert_eq!(evidence.memory_replay_recursive_runtime_count(), 1);
        assert_eq!(evidence.memory_replay_live_memory_feedback_count(), 1);
        assert_eq!(evidence.memory_replay_rust_check_count(), 1);
        assert_eq!(evidence.memory_replay_business_contract_count(), 1);
        assert_eq!(evidence.memory_replay_context_rot_count(), 1);
        assert_eq!(
            evidence.memory_replay_reason_codes(),
            vec![
                "action_hold".to_owned(),
                "action_penalize".to_owned(),
                "action_reinforce".to_owned(),
                "business_contract".to_owned(),
                "context_rot".to_owned(),
                "feedback_missing".to_owned(),
                "feedback_removed".to_owned(),
                "live_memory_feedback".to_owned(),
                "memory_penalty".to_owned(),
                "memory_reinforcement".to_owned(),
                "recursive_runtime".to_owned(),
                "rust_check".to_owned(),
            ]
        );
        assert!(
            evidence
                .memory_replay_detail_codes()
                .contains(&"signal:context_rot:70656e616c697a65".to_owned())
        );
        assert!(
            evidence
                .memory_replay_detail_codes()
                .contains(&"feedback:missing:6d697373696e67".to_owned())
        );

        assert_eq!(evidence.memory_evolution_replay_run_count(), 2);
        assert_eq!(evidence.memory_evolution_replay_item_count(), 4);
        assert_eq!(evidence.memory_evolution_replay_update_count(), 3);
        assert_eq!(evidence.memory_evolution_replay_missing_count(), 2);
        assert_eq!(evidence.memory_evolution_invalid_memory_id_count(), 1);
        assert_eq!(evidence.memory_evolution_context_rot_count(), 1);
        assert_eq!(evidence.memory_evolution_live_feedback_count(), 1);
        assert_eq!(evidence.memory_evolution_retention_decay_count(), 2);
        assert_eq!(evidence.memory_evolution_retention_removal_count(), 1);
        assert_eq!(evidence.memory_evolution_compaction_merge_count(), 1);
        assert_eq!(evidence.memory_evolution_compaction_removal_count(), 1);
        assert_eq!(evidence.memory_evolution_external_applied_count(), 1);
        assert_eq!(evidence.memory_evolution_external_missing_count(), 1);
        assert_eq!(evidence.memory_evolution_drift_rollback_count(), 1);
        assert_eq!(evidence.memory_evolution_hygiene_pressure_score(), 235);
        assert_eq!(
            evidence.memory_evolution_hygiene_pressure_priority_codes(),
            vec!["clean".to_owned(), "quarantine".to_owned()]
        );
        assert_eq!(
            evidence.memory_evolution_hygiene_pressure_action_lanes(),
            vec![
                "context_rot_review".to_owned(),
                "experience_index_rebuild".to_owned(),
                "kvswap_boundary_repair".to_owned()
            ]
        );
        assert_eq!(
            evidence.memory_evolution_hygiene_pressure_action_lane_details(),
            vec![
                "context_rot_review:repair:5:1".to_owned(),
                "experience_index_rebuild:quarantine:120:3".to_owned(),
                "kvswap_boundary_repair:quarantine:110:2".to_owned()
            ]
        );
        assert_eq!(
            evidence.memory_evolution_hygiene_work_next_action_codes(),
            vec!["experience_index_rebuild".to_owned()]
        );
        assert_eq!(
            evidence.memory_evolution_hygiene_work_operator_review_count(),
            1
        );
        assert_eq!(evidence.memory_evolution_hygiene_work_isolation_count(), 1);
        assert_eq!(evidence.memory_hygiene_work_plan_count(), 1);
        assert_eq!(
            evidence.memory_hygiene_work_action_lanes(),
            vec![
                "context_rot_review".to_owned(),
                "experience_index_rebuild".to_owned(),
                "kvswap_boundary_repair".to_owned()
            ]
        );
        assert_eq!(
            evidence.memory_hygiene_work_action_lane_details(),
            vec![
                "context_rot_review:repair:5:1".to_owned(),
                "experience_index_rebuild:quarantine:120:3".to_owned(),
                "kvswap_boundary_repair:quarantine:110:2".to_owned()
            ]
        );
        assert_eq!(
            evidence.memory_hygiene_work_next_dispatch_codes(),
            vec![
                "dispatch:operator_review:isolated:experience_index_rebuild:quarantine:120:3"
                    .to_owned()
            ]
        );
        assert_eq!(
            evidence.memory_hygiene_work_dispatch_codes(),
            vec![
                "dispatch:operator_review:isolated:context_rot_review:repair:5:1".to_owned(),
                "dispatch:operator_review:isolated:experience_index_rebuild:quarantine:120:3"
                    .to_owned(),
                "dispatch:operator_review:isolated:kvswap_boundary_repair:quarantine:110:2"
                    .to_owned()
            ]
        );
        assert_eq!(evidence.memory_hygiene_work_queue_count(), 1);
        assert_eq!(evidence.memory_hygiene_work_queue_item_count(), 3);
        assert_eq!(
            evidence.memory_hygiene_work_queue_operator_review_count(),
            3
        );
        assert_eq!(evidence.memory_hygiene_work_queue_isolation_count(), 3);
        assert_eq!(
            evidence.memory_hygiene_work_queue_next_dispatch_codes(),
            vec![
                "dispatch:operator_review:isolated:experience_index_rebuild:quarantine:120:3"
                    .to_owned()
            ]
        );
        assert_eq!(
            evidence.memory_hygiene_work_queue_lane_codes(),
            vec![
                "context_rot_review".to_owned(),
                "experience_index_rebuild".to_owned(),
                "kvswap_boundary_repair".to_owned()
            ]
        );
        assert_eq!(
            evidence.memory_hygiene_work_queue_priority_codes(),
            vec!["quarantine".to_owned(), "repair".to_owned()]
        );
        assert_eq!(
            evidence.memory_hygiene_work_queue_dispatch_codes(),
            vec![
                "dispatch:operator_review:isolated:context_rot_review:repair:5:1".to_owned(),
                "dispatch:operator_review:isolated:experience_index_rebuild:quarantine:120:3"
                    .to_owned(),
                "dispatch:operator_review:isolated:kvswap_boundary_repair:quarantine:110:2"
                    .to_owned()
            ]
        );
        assert_eq!(
            evidence.memory_hygiene_work_queue_detail_codes(),
            vec![
                "context_rot_review:repair:5:1".to_owned(),
                "experience_index_rebuild:quarantine:120:3".to_owned(),
                "kvswap_boundary_repair:quarantine:110:2".to_owned()
            ]
        );
        assert_eq!(
            evidence.memory_hygiene_work_queue_reason_codes(),
            vec![
                "isolation_recommended".to_owned(),
                "items_present".to_owned(),
                "operator_review_required".to_owned()
            ]
        );
        let dispatch_pressure = evidence.hygiene_dispatch_pressure_summary();
        assert_eq!(
            dispatch_pressure,
            MemoryHygieneDispatchPressureSummary {
                pressure_score: 235,
                queue_items: 3,
                operator_review_items: 3,
                isolation_items: 3,
                kvswap_boundary_repair_lanes: 1,
                context_rot_review_lanes: 1,
                experience_index_rebuild_lanes: 1,
                quarantine_priorities: 1,
                repair_priorities: 1,
                context_rot_risks: 0,
                missing_clean_gist_pressure: 0,
                kvswap_boundary_blockers: 1,
                kvswap_boundary_warnings: 1,
            }
        );
        assert!(dispatch_pressure.has_pressure());
        assert!(dispatch_pressure.requires_operator_review());
        assert!(dispatch_pressure.requires_isolation());
        assert_eq!(dispatch_pressure.priority_code(), "quarantine");
        assert_eq!(dispatch_pressure.dispatch_rank(), 3);
        assert_eq!(
            dispatch_pressure.reason_codes(),
            vec![
                "queue_items".to_owned(),
                "operator_review_items".to_owned(),
                "isolation_items".to_owned(),
                "kvswap_boundary_repair".to_owned(),
                "context_rot_review".to_owned(),
                "experience_index_rebuild".to_owned(),
                "quarantine_priority".to_owned(),
                "repair_priority".to_owned(),
                "kvswap_boundary_blocker".to_owned(),
                "kvswap_boundary_warning".to_owned(),
            ]
        );
        assert_eq!(
            evidence.memory_hygiene_dispatch_pressure_reason_count("missing_clean_gist"),
            0
        );
        let dispatch_pressure_line = dispatch_pressure.summary_line();
        assert!(dispatch_pressure_line.contains("rank=3 priority=quarantine"));
        assert!(dispatch_pressure_line.contains("pressure_score=235 queue_items=3"));
        assert!(dispatch_pressure_line.contains(
            "reason_codes=queue_items|operator_review_items|isolation_items|kvswap_boundary_repair|context_rot_review|experience_index_rebuild|quarantine_priority|repair_priority|kvswap_boundary_blocker|kvswap_boundary_warning"
        ));
        assert_eq!(evidence.memory_hygiene_work_item_count(), 3);
        assert_eq!(
            evidence.memory_hygiene_work_item_lane_codes(),
            vec![
                "context_rot_review".to_owned(),
                "experience_index_rebuild".to_owned(),
                "kvswap_boundary_repair".to_owned()
            ]
        );
        assert_eq!(
            evidence.memory_hygiene_work_item_priority_codes(),
            vec!["quarantine".to_owned(), "repair".to_owned()]
        );
        assert_eq!(
            evidence.memory_hygiene_work_item_dispatch_codes(),
            vec![
                "dispatch:operator_review:isolated:context_rot_review:repair:5:1".to_owned(),
                "dispatch:operator_review:isolated:experience_index_rebuild:quarantine:120:3"
                    .to_owned(),
                "dispatch:operator_review:isolated:kvswap_boundary_repair:quarantine:110:2"
                    .to_owned()
            ]
        );
        assert_eq!(
            evidence.memory_hygiene_work_item_detail_codes(),
            vec![
                "context_rot_review:repair:5:1".to_owned(),
                "experience_index_rebuild:quarantine:120:3".to_owned(),
                "kvswap_boundary_repair:quarantine:110:2".to_owned()
            ]
        );
        assert_eq!(
            evidence.memory_evolution_hygiene_pressure_reason_codes(),
            vec![
                "context_rot_pressure".to_owned(),
                "index_quality_blocker".to_owned(),
                "index_quality_warning".to_owned(),
                "kvswap_boundary_blocker".to_owned(),
                "kvswap_boundary_warning".to_owned()
            ]
        );
        assert_eq!(
            evidence.memory_evolution_hygiene_pressure_detail_codes(),
            vec![
                "context_rot_items:1".to_owned(),
                "index_quality_blockers:1".to_owned(),
                "index_quality_warnings:2".to_owned(),
                "kvswap_boundary_blockers:1".to_owned(),
                "kvswap_boundary_warnings:1".to_owned()
            ]
        );
        assert_eq!(
            evidence.memory_evolution_reason_codes(),
            vec![
                "compaction_merge".to_owned(),
                "compaction_removal".to_owned(),
                "context_rot".to_owned(),
                "drift_rollback".to_owned(),
                "external_feedback_applied".to_owned(),
                "external_feedback_missing".to_owned(),
                "index_quality_blocker".to_owned(),
                "index_quality_warning".to_owned(),
                "invalid_memory_id".to_owned(),
                "kvswap_boundary_blocker".to_owned(),
                "kvswap_boundary_warning".to_owned(),
                "live_memory_feedback".to_owned(),
                "replay_evidence".to_owned(),
                "replay_memory_update".to_owned(),
                "replay_missing_memory".to_owned(),
                "retention_decay".to_owned(),
                "retention_removal".to_owned(),
            ]
        );
        assert!(!evidence.summary_text().contains(replay_payload_secret));
        assert!(!evidence.summary_line().contains(replay_payload_secret));
    }

    #[test]
    fn startup_evidence_aggregates_adapter_checklist_counts_and_details() {
        let checklist_payload_secret = "CHECKLIST_PAYLOAD_SECRET_DO_NOT_LOG";
        let evidence = MemoryServiceStartupEvidence {
            requires_operator_review: true,
            approved_phases: vec![MemoryMigrationPhase::ReadOnlyShadow],
            lines: vec![
                "memory_adapter_checklist satisfied=false items=5 blockers=1 warnings=3 blocker_codes=capability_manifest_ready warning_codes=adapter_snapshots_clean|context_rot_risks_clean|kvswap_boundary_clean blocker_detail_codes=capability_manifest_ready:missing|capability_manifest_ready:write_blockers warning_detail_codes=adapter_snapshots_clean:warnings|context_rot_risks_clean:context_rot_risks|kvswap_boundary_clean:boundary_issues|kvswap_boundary_clean:boundary_reason_codes:overlapping_hot_cold".to_owned(),
                "memory_adapter_checklist satisfied=true items=1 blockers=0 warnings=0 blocker_codes=none warning_codes=none blocker_detail_codes=none warning_detail_codes=none".to_owned(),
                "memory_adapter_checklist_item code=capability_manifest_ready satisfied=false severity=blocker detail_codes=missing|write_blockers".to_owned(),
                "memory_adapter_checklist_item code=adapter_snapshots_clean satisfied=false severity=warning detail_codes=warnings".to_owned(),
                "memory_adapter_checklist_item code=context_rot_risks_clean satisfied=false severity=warning detail_codes=context_rot_risks".to_owned(),
                "memory_adapter_checklist_item code=kvswap_boundary_clean satisfied=false severity=warning detail_codes=boundary_issues|boundary_reason_codes:overlapping_hot_cold".to_owned(),
                "memory_adapter_checklist_item code=projection_contract_manifest_ready satisfied=false severity=info detail_codes=manifest_gap".to_owned(),
                "memory_adapter_checklist_item code=evolution_gate_ready satisfied=true severity=blocker detail_codes=none".to_owned(),
            ],
        };

        assert_eq!(evidence.adapter_checklist_report_count(), 2);
        assert_eq!(evidence.adapter_checklist_summary_item_count(), 6);
        assert_eq!(evidence.adapter_checklist_blocker_count(), 1);
        assert_eq!(evidence.adapter_checklist_warning_count(), 3);
        assert_eq!(evidence.adapter_checklist_satisfied_report_count(), 1);
        assert_eq!(evidence.adapter_checklist_item_line_count(), 6);
        assert_eq!(evidence.adapter_checklist_failed_item_count(), 5);
        assert_eq!(evidence.adapter_checklist_failed_blocker_count(), 1);
        assert_eq!(evidence.adapter_checklist_failed_warning_count(), 3);
        assert_eq!(evidence.adapter_checklist_failed_info_count(), 1);
        assert_eq!(
            evidence.adapter_checklist_blocker_codes(),
            vec!["capability_manifest_ready".to_owned()]
        );
        assert_eq!(
            evidence.adapter_checklist_warning_codes(),
            vec![
                "adapter_snapshots_clean".to_owned(),
                "context_rot_risks_clean".to_owned(),
                "kvswap_boundary_clean".to_owned(),
            ]
        );
        assert_eq!(
            evidence.adapter_checklist_failed_item_codes(),
            vec![
                "adapter_snapshots_clean".to_owned(),
                "capability_manifest_ready".to_owned(),
                "context_rot_risks_clean".to_owned(),
                "kvswap_boundary_clean".to_owned(),
                "projection_contract_manifest_ready".to_owned(),
            ]
        );
        assert_eq!(
            evidence.adapter_checklist_failed_item_detail_codes(),
            vec![
                "adapter_snapshots_clean:warnings".to_owned(),
                "capability_manifest_ready:missing".to_owned(),
                "capability_manifest_ready:write_blockers".to_owned(),
                "context_rot_risks_clean:context_rot_risks".to_owned(),
                "kvswap_boundary_clean:boundary_issues".to_owned(),
                "kvswap_boundary_clean:boundary_reason_codes:overlapping_hot_cold".to_owned(),
                "projection_contract_manifest_ready:manifest_gap".to_owned(),
            ]
        );
        assert_eq!(
            evidence.adapter_checklist_item_detail_codes_for("kvswap_boundary_clean"),
            vec![
                "boundary_issues".to_owned(),
                "boundary_reason_codes:overlapping_hot_cold".to_owned(),
            ]
        );
        assert_eq!(
            evidence.adapter_checklist_failed_item_detail_codes_for("kvswap_boundary_clean"),
            vec![
                "boundary_issues".to_owned(),
                "boundary_reason_codes:overlapping_hot_cold".to_owned(),
            ]
        );
        assert_eq!(
            evidence.adapter_checklist_failed_item_detail_codes_for("evolution_gate_ready"),
            Vec::<String>::new()
        );
        let detail_codes = evidence.detail_codes();
        assert!(detail_codes.contains(&"capability_manifest_ready:missing".to_owned()));
        assert!(detail_codes.contains(&"context_rot_risks_clean:context_rot_risks".to_owned()));
        assert!(detail_codes.contains(
            &"kvswap_boundary_clean:boundary_reason_codes:overlapping_hot_cold".to_owned()
        ));
        assert!(detail_codes.contains(&"manifest_gap".to_owned()));
        assert!(!evidence.summary_text().contains(checklist_payload_secret));
        assert!(!evidence.summary_line().contains(checklist_payload_secret));
    }

    #[test]
    fn checklist_detail_codes_project_structured_detail_fields() {
        let item = MemoryServiceChecklistItem::new(
            "projection_probe",
            false,
            MemoryServiceChecklistSeverity::Warning,
            "guard_codes=fixture_catalog_not_verified|fixture_checksum_not_verified \
             detail_codes=disk_kv_catalog:missing_bytes:6d697373696e67 \
             boundary_detail_codes=blocker:overlap:73686172642d61 \
             blocker_codes=none warning_codes=none \
             replay_runs=1 replay_items=1 replay_updates=1",
        );

        assert_eq!(
            item.detail_codes(),
            vec![
                "boundary_detail_codes:blocker:overlap:73686172642d61".to_owned(),
                "detail_codes:disk_kv_catalog:missing_bytes:6d697373696e67".to_owned(),
                "guard_codes:fixture_catalog_not_verified".to_owned(),
                "guard_codes:fixture_checksum_not_verified".to_owned(),
                "replay_items".to_owned(),
                "replay_runs".to_owned(),
                "replay_updates".to_owned(),
            ]
        );
        assert_eq!(
            item.summary_line(),
            "memory_adapter_checklist_item code=projection_probe satisfied=false severity=warning detail_codes=boundary_detail_codes:blocker:overlap:73686172642d61|detail_codes:disk_kv_catalog:missing_bytes:6d697373696e67|guard_codes:fixture_catalog_not_verified|guard_codes:fixture_checksum_not_verified|replay_items|replay_runs|replay_updates"
        );
    }

    #[test]
    fn checklist_detail_codes_ignore_empty_and_false_detail_fields() {
        let item = MemoryServiceChecklistItem::new(
            "clean_projection_probe",
            true,
            MemoryServiceChecklistSeverity::Blocker,
            "missing=0 dirty=false detail_codes=none blocker_codes=none warning_codes=none",
        );

        assert!(item.detail_codes().is_empty());
        assert_eq!(
            item.summary_line(),
            "memory_adapter_checklist_item code=clean_projection_probe satisfied=true severity=blocker detail_codes=none"
        );
    }

    #[test]
    fn startup_evidence_aggregates_projection_contract_and_bundle_readiness() {
        let contract_payload_secret = "PROJECTION_CONTRACT_PAYLOAD_SECRET_DO_NOT_LOG";
        let evidence = MemoryServiceStartupEvidence {
            requires_operator_review: true,
            approved_phases: vec![MemoryMigrationPhase::ReadOnlyShadow],
            lines: vec![
                "adapter_projection adapter=experience_shadow kind=experience_store target=isolated_write ready=false write_mode=read_only missing_required=2 missing_recommended=1 blockers=3 warnings=1 blocker_codes=missing_required|write_mode_not_isolated warning_codes=missing_recommended blocker_detail_codes=experience_shadow:missing_required:experience_projection_tags|experience_shadow:missing_required:experience_task_scope|experience_shadow:write_mode_not_isolated:read_only warning_detail_codes=experience_shadow:missing_recommended:experience_clean_gist".to_owned(),
                "adapter_projection adapter=disk_shadow kind=disk_kv_store target=isolated_write ready=true write_mode=isolated_write missing_required=0 missing_recommended=0 blockers=0 warnings=0 blocker_codes=none warning_codes=none blocker_detail_codes=none warning_detail_codes=none".to_owned(),
                "adapter_projection_contract adapter=experience_shadow kind=experience_store target=isolated_write write_mode=read_only mapped_fields=experience_id|experience_prompt|experience_lesson|experience_quality required_fields=experience_id|experience_prompt|experience_lesson|experience_quality|experience_projection_tags|experience_task_scope recommended_fields=experience_clean_gist notes=1".to_owned(),
                "adapter_projection_contract adapter=disk_shadow kind=disk_kv_store target=isolated_write write_mode=isolated_write mapped_fields=kv_shard_id|kv_shard_bytes|kv_shard_metadata|kv_shard_checksum|kv_shard_tier required_fields=kv_shard_id|kv_shard_bytes|kv_shard_metadata|kv_shard_checksum recommended_fields=kv_shard_tier|kv_shard_priority notes=0".to_owned(),
                "adapter_projection_bundle name=fixture target=isolated_write ready=false review=true contracts=2 ready_contracts=1 blockers=3 warnings=1 blocker_codes=missing_required|write_mode_not_isolated warning_codes=missing_recommended blocker_detail_codes=experience_shadow:missing_required:experience_projection_tags|experience_shadow:write_mode_not_isolated:read_only warning_detail_codes=experience_shadow:missing_recommended:experience_clean_gist".to_owned(),
                "adapter_projection_bundle name=clean_shadow target=shadow_read ready=true review=false contracts=1 ready_contracts=1 blockers=0 warnings=0 blocker_codes=none warning_codes=none blocker_detail_codes=none warning_detail_codes=none".to_owned(),
                "adapter_projection_contract_bundle_manifest name=fixture target=isolated_write contracts=2 adapters=experience_shadow|disk_shadow mapped_fields=9 required_fields=10 recommended_fields=3 notes=1".to_owned(),
                "adapter_projection_contract_bundle_manifest name=clean_shadow target=shadow_read contracts=1 adapters=experience_shadow mapped_fields=4 required_fields=4 recommended_fields=0 notes=0".to_owned(),
            ],
        };

        assert_eq!(evidence.projection_contract_count(), 2);
        assert_eq!(evidence.projection_contract_ready_count(), 1);
        assert_eq!(evidence.projection_contract_missing_required_count(), 2);
        assert_eq!(evidence.projection_contract_missing_recommended_count(), 1);
        assert_eq!(evidence.projection_contract_blocker_count(), 3);
        assert_eq!(evidence.projection_contract_warning_count(), 1);
        assert_eq!(
            evidence.projection_contract_blocker_codes(),
            vec![
                "missing_required".to_owned(),
                "write_mode_not_isolated".to_owned(),
            ]
        );
        assert_eq!(
            evidence.projection_contract_warning_codes(),
            vec!["missing_recommended".to_owned()]
        );
        assert!(
            evidence
                .projection_contract_blocker_detail_codes()
                .contains(
                    &"experience_shadow:missing_required:experience_projection_tags".to_owned()
                )
        );
        assert_eq!(evidence.projection_contract_manifest_count(), 2);
        assert_eq!(
            evidence.projection_contract_manifest_mapped_field_count(),
            9
        );
        assert_eq!(
            evidence.projection_contract_manifest_required_field_count(),
            10
        );
        assert_eq!(
            evidence.projection_contract_manifest_recommended_field_count(),
            3
        );
        assert_eq!(evidence.projection_contract_manifest_note_count(), 1);

        assert_eq!(evidence.projection_bundle_report_count(), 2);
        assert_eq!(evidence.projection_bundle_ready_count(), 1);
        assert_eq!(evidence.projection_bundle_review_count(), 1);
        assert_eq!(evidence.projection_bundle_contract_count(), 3);
        assert_eq!(evidence.projection_bundle_ready_contract_count(), 2);
        assert_eq!(evidence.projection_bundle_blocker_count(), 3);
        assert_eq!(evidence.projection_bundle_warning_count(), 1);
        assert_eq!(
            evidence.projection_bundle_blocker_codes(),
            vec![
                "missing_required".to_owned(),
                "write_mode_not_isolated".to_owned(),
            ]
        );
        assert_eq!(
            evidence.projection_bundle_warning_detail_codes(),
            vec!["experience_shadow:missing_recommended:experience_clean_gist".to_owned()]
        );
        assert_eq!(evidence.projection_bundle_manifest_count(), 2);
        assert_eq!(evidence.projection_bundle_manifest_contract_count(), 3);
        assert_eq!(evidence.projection_bundle_manifest_mapped_field_count(), 13);
        assert_eq!(
            evidence.projection_bundle_manifest_required_field_count(),
            14
        );
        assert_eq!(
            evidence.projection_bundle_manifest_recommended_field_count(),
            3
        );
        assert_eq!(evidence.projection_bundle_manifest_note_count(), 1);

        let detail_codes = evidence.detail_codes();
        assert!(
            detail_codes.contains(
                &"experience_shadow:missing_required:experience_projection_tags".to_owned()
            )
        );
        assert!(
            detail_codes.contains(
                &"experience_shadow:missing_recommended:experience_clean_gist".to_owned()
            )
        );
        assert!(!evidence.summary_text().contains(contract_payload_secret));
        assert!(!evidence.summary_line().contains(contract_payload_secret));
    }

    #[test]
    fn startup_evidence_aggregates_adapter_status_and_capability_coverage() {
        let adapter_payload_secret = "ADAPTER_STATUS_PAYLOAD_SECRET_DO_NOT_LOG";
        let evidence = MemoryServiceStartupEvidence {
            requires_operator_review: true,
            approved_phases: vec![MemoryMigrationPhase::ReadOnlyShadow],
            lines: vec![
                "memory_adapter_status name=unhealthy_index ready=false read_only=false write_mode=isolated_write capabilities=memory_index|context_injection records=1 warnings=1 status_codes=health_warnings|unhealthy warning_codes=lagging_snapshot".to_owned(),
                "memory_adapter_status name=readonly_disk ready=true read_only=true write_mode=read_only capabilities=disk_kv_offload records=unknown warnings=0 status_codes=read_only warning_codes=none".to_owned(),
                "memory_adapter_status name=live_service ready=true read_only=false write_mode=live_write capabilities=memory_evolution records=3 warnings=1 status_codes=health_warnings|live_write_enabled warning_codes=operator_ack_missing".to_owned(),
                "memory_capability_coverage capability=memory_index providers=unhealthy_index healthy=none writable=none read_only=none records=1 status_codes=no_healthy_provider".to_owned(),
                "memory_capability_coverage capability=disk_kv_offload providers=readonly_disk healthy=readonly_disk writable=none read_only=readonly_disk records=unknown status_codes=write_mode_blocked".to_owned(),
                "memory_capability_coverage capability=context_injection providers=unhealthy_index,live_service healthy=live_service writable=live_service read_only=none records=4 status_codes=multiple_providers".to_owned(),
                "memory_capability_coverage capability=kv_swap providers=none healthy=none writable=none read_only=none records=unknown status_codes=missing_provider".to_owned(),
            ],
        };

        assert_eq!(evidence.adapter_status_count(), 3);
        assert_eq!(evidence.adapter_status_ready_count(), 2);
        assert_eq!(evidence.adapter_status_unhealthy_count(), 1);
        assert_eq!(evidence.adapter_status_read_only_count(), 1);
        assert_eq!(evidence.adapter_status_isolated_write_count(), 1);
        assert_eq!(evidence.adapter_status_live_write_count(), 1);
        assert_eq!(evidence.adapter_status_capability_count(), 4);
        assert_eq!(evidence.adapter_status_record_count(), 4);
        assert_eq!(evidence.adapter_status_warning_count(), 2);
        assert_eq!(
            evidence.adapter_status_status_codes(),
            vec![
                "health_warnings".to_owned(),
                "live_write_enabled".to_owned(),
                "read_only".to_owned(),
                "unhealthy".to_owned(),
            ]
        );
        assert_eq!(
            evidence.adapter_status_warning_codes(),
            vec![
                "lagging_snapshot".to_owned(),
                "operator_ack_missing".to_owned(),
            ]
        );
        assert_eq!(
            evidence.adapter_status_status_code_count("health_warnings"),
            2
        );
        assert_eq!(evidence.adapter_status_status_code_count("unhealthy"), 1);

        assert_eq!(evidence.capability_coverage_count(), 4);
        assert_eq!(evidence.capability_coverage_provider_count(), 4);
        assert_eq!(evidence.capability_coverage_healthy_provider_count(), 2);
        assert_eq!(evidence.capability_coverage_writable_provider_count(), 1);
        assert_eq!(evidence.capability_coverage_read_only_provider_count(), 1);
        assert_eq!(evidence.capability_coverage_record_count(), 5);
        assert_eq!(
            evidence.capability_coverage_status_codes(),
            vec![
                "missing_provider".to_owned(),
                "multiple_providers".to_owned(),
                "no_healthy_provider".to_owned(),
                "write_mode_blocked".to_owned(),
            ]
        );
        assert_eq!(evidence.capability_coverage_missing_provider_count(), 1);
        assert_eq!(evidence.capability_coverage_no_healthy_provider_count(), 1);
        assert_eq!(evidence.capability_coverage_write_mode_blocked_count(), 1);
        assert_eq!(evidence.capability_coverage_multiple_provider_count(), 1);
        assert!(!evidence.summary_text().contains(adapter_payload_secret));
        assert!(!evidence.summary_line().contains(adapter_payload_secret));
    }

    #[test]
    fn startup_evidence_aggregates_migration_readiness_and_approval_rows() {
        let migration_payload_secret = "MIGRATION_PAYLOAD_SECRET_DO_NOT_LOG";
        let evidence = MemoryServiceStartupEvidence {
            requires_operator_review: true,
            approved_phases: vec![MemoryMigrationPhase::ReadOnlyShadow],
            lines: vec![
                "memory_migration_readiness isolated_write_ready=false review=true blockers=1 warnings=2 blocker_codes=write_mode_blocked warning_codes=context_rejections|kvswap_intent_pending blocker_detail_codes=write_mode_blocked:disk_kv_offload warning_detail_codes=context_rejections=2|kvswap_intent_pending blocker_details=write_mode_blocked:disk_kv_offload warning_details=context_rejections=2|kvswap_intent_pending".to_owned(),
                "memory_migration_readiness isolated_write_ready=true review=true blockers=0 warnings=1 blocker_codes=none warning_codes=repair_items blocker_detail_codes=none warning_detail_codes=repair_items=1 blocker_details=none warning_details=repair_items=1".to_owned(),
                "memory_migration phase=read_only_shadow approved=true required_write_mode=read_only blockers=0 warnings=1 blocker_codes=none warning_codes=operator_review_required blocker_detail_codes=none warning_detail_codes=operator_review_required blocker_details=none warning_details=operator_review_required".to_owned(),
                "memory_migration phase=isolated_write approved=false required_write_mode=isolated_write blockers=2 warnings=1 blocker_codes=projection_contract_blocker:missing_required|shadow_plan_requires_operator_review warning_codes=migration:repair_items blocker_detail_codes=projection_contract_blocker:experience_shadow:missing_required:experience_projection_tags|shadow_plan_requires_operator_review warning_detail_codes=migration:repair_items=1 blocker_details=projection_contract_blocker:experience_shadow:missing_required:experience_projection_tags|shadow_plan_requires_operator_review warning_details=migration:repair_items=1".to_owned(),
                "memory_migration phase=live_write approved=false required_write_mode=live_write blockers=1 warnings=0 blocker_codes=operator_ack_required warning_codes=none blocker_detail_codes=operator_ack_required warning_detail_codes=none blocker_details=operator_ack_required warning_details=none".to_owned(),
            ],
        };

        assert_eq!(evidence.migration_readiness_report_count(), 2);
        assert_eq!(evidence.migration_readiness_isolated_write_ready_count(), 1);
        assert_eq!(evidence.migration_readiness_review_count(), 2);
        assert_eq!(evidence.migration_readiness_blocker_count(), 1);
        assert_eq!(evidence.migration_readiness_warning_count(), 3);
        assert_eq!(
            evidence.migration_readiness_blocker_codes(),
            vec!["write_mode_blocked".to_owned()]
        );
        assert_eq!(
            evidence.migration_readiness_warning_codes(),
            vec![
                "context_rejections".to_owned(),
                "kvswap_intent_pending".to_owned(),
                "repair_items".to_owned(),
            ]
        );
        assert_eq!(
            evidence.migration_readiness_blocker_detail_codes(),
            vec!["write_mode_blocked:disk_kv_offload".to_owned()]
        );
        assert_eq!(
            evidence.migration_readiness_warning_detail_codes(),
            vec![
                "context_rejections=2".to_owned(),
                "kvswap_intent_pending".to_owned(),
                "repair_items=1".to_owned(),
            ]
        );

        assert_eq!(evidence.migration_approval_count(), 3);
        assert_eq!(evidence.migration_approval_approved_count(), 1);
        assert_eq!(evidence.migration_approval_blocked_count(), 2);
        assert_eq!(evidence.migration_approval_read_only_required_count(), 1);
        assert_eq!(
            evidence.migration_approval_isolated_write_required_count(),
            1
        );
        assert_eq!(evidence.migration_approval_live_write_required_count(), 1);
        assert_eq!(evidence.migration_approval_blocker_count(), 3);
        assert_eq!(evidence.migration_approval_warning_count(), 2);
        assert_eq!(
            evidence.migration_approval_blocker_codes(),
            vec![
                "operator_ack_required".to_owned(),
                "projection_contract_blocker:missing_required".to_owned(),
                "shadow_plan_requires_operator_review".to_owned(),
            ]
        );
        assert_eq!(
            evidence.migration_approval_warning_codes(),
            vec![
                "migration:repair_items".to_owned(),
                "operator_review_required".to_owned(),
            ]
        );
        assert!(
            evidence
                .migration_approval_blocker_detail_codes()
                .contains(&"projection_contract_blocker:experience_shadow:missing_required:experience_projection_tags".to_owned())
        );
        assert!(
            evidence
                .migration_approval_warning_detail_codes()
                .contains(&"migration:repair_items=1".to_owned())
        );
        assert!(!evidence.summary_text().contains(migration_payload_secret));
        assert!(!evidence.summary_line().contains(migration_payload_secret));
    }

    #[test]
    fn startup_evidence_aggregates_projection_audit_codes() {
        let leaked_source_id = "DUPLICATE_SOURCE_SECRET_DO_NOT_LOG";
        let evidence = MemoryServiceStartupEvidence {
            requires_operator_review: true,
            approved_phases: vec![MemoryMigrationPhase::ReadOnlyShadow],
            lines: vec![
                "adapter_projection_audit shadow_ready=false isolated_write_ready=false experiences=2 kv_shards=0 issues=2 blockers=2 warnings=0 issue_codes=duplicate_experience_id|empty_experience_content detail_codes=blocker:duplicate_experience_id:source_id_hex:647570|blocker:empty_experience_content:source_id_hex:647570".to_owned(),
                "adapter_projection_audit shadow_ready=true isolated_write_ready=false experiences=1 kv_shards=0 issues=2 blockers=0 warnings=2 issue_codes=duplicate_experience_id|missing_clean_gist_for_risky_record detail_codes=blocker:duplicate_experience_id:source_id_hex:647570|warning:missing_clean_gist_for_risky_record:source_id_hex:7269736b79".to_owned(),
            ],
        };

        assert_eq!(evidence.projection_audit_issue_count(), 4);
        assert_eq!(evidence.projection_audit_blocker_count(), 2);
        assert_eq!(evidence.projection_audit_warning_count(), 2);
        assert_eq!(
            evidence.projection_audit_issue_codes(),
            vec![
                "duplicate_experience_id".to_owned(),
                "empty_experience_content".to_owned(),
                "missing_clean_gist_for_risky_record".to_owned(),
            ]
        );
        assert_eq!(
            evidence.projection_audit_detail_codes(),
            vec![
                "blocker:duplicate_experience_id:source_id_hex:647570".to_owned(),
                "blocker:empty_experience_content:source_id_hex:647570".to_owned(),
                "warning:missing_clean_gist_for_risky_record:source_id_hex:7269736b79".to_owned(),
            ]
        );
        assert!(!evidence.summary_text().contains(leaked_source_id));
        assert!(!evidence.summary_line().contains(leaked_source_id));
        assert!(
            !evidence
                .projection_audit_detail_codes()
                .iter()
                .any(|code| code.contains(leaked_source_id))
        );
    }

    #[test]
    fn startup_evidence_aggregates_disk_kv_catalog_detail_codes_across_rows() {
        let evidence = MemoryServiceStartupEvidence {
            requires_operator_review: true,
            approved_phases: vec![MemoryMigrationPhase::ReadOnlyShadow],
            lines: vec![
                "memory_migration_evidence source_read_only=true copied_fixture=true isolated_write_root=true catalog_verified=false checksum_verified=false live_store_targeted=false records=1 guard_codes=fixture_catalog_not_verified detail_codes=disk_kv_catalog:missing_bytes:6d697373696e67|disk_kv_catalog:byte_len_mismatch:7374616c652d61|guard:fixture_catalog_not_verified".to_owned(),
                "memory_migration_evidence source_read_only=true copied_fixture=true isolated_write_root=true catalog_verified=false checksum_verified=false live_store_targeted=false records=1 guard_codes=fixture_catalog_not_verified detail_codes=disk_kv_catalog:missing_bytes:6d697373696e67|disk_kv_catalog:byte_len_mismatch:7374616c652d62|disk_kv_catalog:checksum_mismatch:636f7272757074".to_owned(),
            ],
        };

        assert_eq!(evidence.disk_kv_catalog_missing_bytes_count(), 1);
        assert_eq!(evidence.disk_kv_catalog_byte_len_mismatch_count(), 2);
        assert_eq!(evidence.disk_kv_catalog_checksum_mismatch_count(), 1);
        assert_eq!(
            evidence.disk_kv_catalog_detail_codes_for("byte_len_mismatch"),
            vec![
                "disk_kv_catalog:byte_len_mismatch:7374616c652d61".to_owned(),
                "disk_kv_catalog:byte_len_mismatch:7374616c652d62".to_owned(),
            ]
        );
        assert_eq!(
            evidence.disk_kv_catalog_detail_codes_for("missing_bytes"),
            vec!["disk_kv_catalog:missing_bytes:6d697373696e67".to_owned()]
        );
        assert!(
            evidence
                .migration_evidence_detail_codes()
                .contains(&"guard:fixture_catalog_not_verified".to_owned())
        );
        assert!(
            !evidence
                .disk_kv_catalog_detail_codes_for("fixture_catalog_not_verified")
                .iter()
                .any(|code| code.starts_with("guard:"))
        );
    }

    #[test]
    fn startup_evidence_aggregates_index_and_context_counts_across_rows() {
        let prompt_secret = "INDEX_CONTEXT_PROMPT_SECRET_DO_NOT_LOG";
        let evidence = MemoryServiceStartupEvidence {
            requires_operator_review: true,
            approved_phases: vec![MemoryMigrationPhase::ReadOnlyShadow],
            lines: vec![
                "memory_index_plan rebuild=true operations=3 upsert=1 refresh=0 compact=0 quarantine=1 delete_duplicate=1 skipped=1 reasons=3 reason_codes=deduplicate_exact_fingerprints|quarantine_high_noise_records|repair_missing_or_dirty_clean_gist detail_codes=delete_duplicate:deduplicate_exact_fingerprint:647570|quarantine:governance_quarantine_candidate:6e6f697379|skipped:6d697373696e67".to_owned(),
                "memory_index_plan rebuild=true operations=2 upsert=0 refresh=1 compact=1 quarantine=0 delete_duplicate=0 skipped=1 reasons=2 reason_codes=compact_long_context_without_gist|quarantine_high_noise_records detail_codes=compact:compact_long_context_without_gist:6c6f6e67|refresh:refresh_noisy_or_rotting_index:726f74|skipped:6d697373696e67".to_owned(),
                "memory_context_injection decisions=3 admit=1 summarize=0 reject_budget=0 reject_risk=1 reject_scope=1 reject_score=0 accepted_risk=0 used_tokens=7 reason_codes=cross_task_scope|missing_clean_gist detail_codes=reject_risk:missing_clean_gist:726177|reject_scope:cross_task_scope:6f7073".to_owned(),
                "memory_context_injection decisions=2 admit=0 summarize=1 reject_budget=1 reject_risk=0 reject_scope=0 reject_score=0 accepted_risk=1 used_tokens=11 reason_codes=max_tokens|raw_fallback_index_content detail_codes=reject_budget:max_tokens:6f766572666c6f77|summarize:raw_fallback_index_content:73756d6d617279".to_owned(),
            ],
        };

        assert_eq!(evidence.memory_index_operation_count(), 5);
        assert_eq!(evidence.memory_index_upsert_count(), 1);
        assert_eq!(evidence.memory_index_refresh_count(), 1);
        assert_eq!(evidence.memory_index_compact_count(), 1);
        assert_eq!(evidence.memory_index_quarantine_count(), 1);
        assert_eq!(evidence.memory_index_delete_duplicate_count(), 1);
        assert_eq!(evidence.memory_index_skipped_count(), 2);
        assert_eq!(
            evidence.memory_index_reason_codes(),
            vec![
                "compact_long_context_without_gist".to_owned(),
                "deduplicate_exact_fingerprints".to_owned(),
                "quarantine_high_noise_records".to_owned(),
                "repair_missing_or_dirty_clean_gist".to_owned(),
            ]
        );
        assert_eq!(
            evidence.memory_index_detail_codes(),
            vec![
                "compact:compact_long_context_without_gist:6c6f6e67".to_owned(),
                "delete_duplicate:deduplicate_exact_fingerprint:647570".to_owned(),
                "quarantine:governance_quarantine_candidate:6e6f697379".to_owned(),
                "refresh:refresh_noisy_or_rotting_index:726f74".to_owned(),
                "skipped:6d697373696e67".to_owned(),
            ]
        );
        assert_eq!(
            evidence.memory_index_detail_codes_for_kind("refresh"),
            vec!["refresh:refresh_noisy_or_rotting_index:726f74".to_owned()]
        );
        assert_eq!(
            evidence.memory_index_detail_codes_for_reason("deduplicate_exact_fingerprint"),
            vec!["delete_duplicate:deduplicate_exact_fingerprint:647570".to_owned()]
        );
        assert_eq!(
            evidence.memory_index_detail_codes_for_reason("compact_long_context_without_gist"),
            vec!["compact:compact_long_context_without_gist:6c6f6e67".to_owned()]
        );
        assert_eq!(
            evidence.memory_index_skipped_detail_codes(),
            vec!["skipped:6d697373696e67".to_owned()]
        );

        assert_eq!(evidence.context_injection_decision_count(), 5);
        assert_eq!(evidence.context_injection_admit_count(), 1);
        assert_eq!(evidence.context_injection_summarize_count(), 1);
        assert_eq!(evidence.context_injection_reject_budget_count(), 1);
        assert_eq!(evidence.context_injection_reject_risk_count(), 1);
        assert_eq!(evidence.context_injection_reject_scope_count(), 1);
        assert_eq!(evidence.context_injection_reject_score_count(), 0);
        assert_eq!(evidence.context_injection_accepted_risk_count(), 1);
        assert_eq!(evidence.context_injection_used_tokens(), 18);
        assert_eq!(
            evidence.context_injection_reason_codes(),
            vec![
                "cross_task_scope".to_owned(),
                "max_tokens".to_owned(),
                "missing_clean_gist".to_owned(),
                "raw_fallback_index_content".to_owned(),
            ]
        );
        assert_eq!(
            evidence.context_injection_detail_codes(),
            vec![
                "reject_budget:max_tokens:6f766572666c6f77".to_owned(),
                "reject_risk:missing_clean_gist:726177".to_owned(),
                "reject_scope:cross_task_scope:6f7073".to_owned(),
                "summarize:raw_fallback_index_content:73756d6d617279".to_owned(),
            ]
        );
        assert!(!evidence.summary_text().contains(prompt_secret));
        assert!(!evidence.summary_line().contains(prompt_secret));
    }

    #[test]
    fn startup_evidence_counts_context_rot_quality_reasons_without_payloads() {
        let payload_secret = "RAW_FALLBACK_CONTEXT_SECRET_DO_NOT_LOG";
        let evidence = MemoryServiceStartupEvidence {
            requires_operator_review: true,
            approved_phases: vec![MemoryMigrationPhase::ReadOnlyShadow],
            lines: vec![
                "memory_context_injection decisions=2 admit=0 summarize=0 reject_budget=0 reject_risk=1 reject_scope=0 reject_score=1 accepted_risk=0 used_tokens=0 reason_codes=missing_clean_gist|raw_fallback_index_content|truncated_index_content detail_codes=reject_risk:missing_clean_gist:726177|reject_risk:raw_fallback_index_content:726177|reject_risk:truncated_index_content:726177|reject_score:below_min_score:7765616b".to_owned(),
                "memory_context_injection decisions=2 admit=1 summarize=1 reject_budget=0 reject_risk=0 reject_scope=0 reject_score=0 accepted_risk=2 used_tokens=12 reason_codes=raw_fallback_index_content|truncated_index_content detail_codes=admit:raw_fallback_index_content:726177|admit:truncated_index_content:726177|summarize:raw_fallback_index_content:73756d6d617279|summarize:truncated_index_content:73756d6d617279".to_owned(),
                "memory_context_injection decisions=1 admit=0 summarize=0 reject_budget=0 reject_risk=1 reject_scope=0 reject_score=0 accepted_risk=0 used_tokens=0 reason_codes=missing_clean_gist|raw_fallback_index_content detail_codes=reject_risk:missing_clean_gist:726177|reject_risk:raw_fallback_index_content:726177".to_owned(),
            ],
        };

        assert_eq!(evidence.context_injection_missing_clean_gist_count(), 1);
        assert_eq!(evidence.context_injection_raw_fallback_count(), 3);
        assert_eq!(
            evidence.context_injection_truncated_index_content_count(),
            3
        );
        assert_eq!(
            evidence.context_injection_detail_codes_for_reason("raw_fallback_index_content"),
            vec![
                "admit:raw_fallback_index_content:726177".to_owned(),
                "reject_risk:raw_fallback_index_content:726177".to_owned(),
                "summarize:raw_fallback_index_content:73756d6d617279".to_owned(),
            ]
        );
        assert_eq!(
            evidence.context_injection_detail_codes_for_reason("truncated_index_content"),
            vec![
                "admit:truncated_index_content:726177".to_owned(),
                "reject_risk:truncated_index_content:726177".to_owned(),
                "summarize:truncated_index_content:73756d6d617279".to_owned(),
            ]
        );
        assert_eq!(
            evidence.context_injection_detail_codes_for_reason("missing_clean_gist"),
            vec!["reject_risk:missing_clean_gist:726177".to_owned()]
        );
        assert_eq!(
            evidence.context_injection_reject_risk_detail_codes(),
            vec![
                "reject_risk:missing_clean_gist:726177".to_owned(),
                "reject_risk:raw_fallback_index_content:726177".to_owned(),
                "reject_risk:truncated_index_content:726177".to_owned(),
            ]
        );
        assert_eq!(
            evidence.context_injection_reject_risk_detail_codes_for_reason(
                "raw_fallback_index_content"
            ),
            vec!["reject_risk:raw_fallback_index_content:726177".to_owned()]
        );
        assert_eq!(
            evidence.context_injection_reject_risk_detail_codes_for_reason("missing_clean_gist"),
            vec!["reject_risk:missing_clean_gist:726177".to_owned()]
        );
        assert!(!evidence.summary_text().contains(payload_secret));
        assert!(!evidence.summary_line().contains(payload_secret));
        assert!(
            !evidence
                .context_injection_detail_codes()
                .iter()
                .any(|code| code.contains(payload_secret))
        );
    }

    #[test]
    fn startup_evidence_emits_lines_and_summary_to_sink() {
        let evidence = MemoryServiceStartupEvidence {
            requires_operator_review: true,
            approved_phases: Vec::new(),
            lines: vec![
                "memory_shadow ready=false detail_codes=read_only:context:reject_risk".to_owned(),
                MemoryMigrationEvidence::read_only_source(Some(1)).summary_line(),
            ],
        };
        let mut sink = Vec::new();

        evidence.emit_to(&mut sink).unwrap();

        assert_eq!(sink.len(), 3);
        assert_eq!(sink[0], evidence.lines[0]);
        assert!(sink[1].starts_with("memory_migration_evidence "));
        assert!(sink[2].starts_with("memory_startup_evidence "));
        assert!(sink[2].contains("operator_review_required"));
        assert!(sink[2].contains("guard:copied_fixture_missing"));
    }

    #[test]
    fn startup_evidence_lifts_kvswap_detail_codes() {
        let prefetch = KvPrefetchPlan {
            promote_ids: vec!["cold shard".to_owned()],
            missing_ids: vec!["missing shard".to_owned()],
            already_hot_ids: Vec::new(),
            duplicate_ids: Vec::new(),
            reason: "requested_ids".to_owned(),
        };
        let eviction = KvEvictionPlan {
            demote_ids: vec!["demote shard".to_owned()],
            keep_hot_ids: vec!["hot shard".to_owned()],
            target_hot_bytes: 16,
            reason: "target_hot_bytes".to_owned(),
        };
        let evidence = MemoryServiceStartupEvidence {
            requires_operator_review: true,
            approved_phases: vec![MemoryMigrationPhase::ReadOnlyShadow],
            lines: vec![prefetch.summary_line(), eviction.summary_line()],
        };
        let detail_codes = evidence.detail_codes();

        assert!(detail_codes.contains(&"promote:requested_ids:636f6c64207368617264".to_owned()));
        assert!(
            detail_codes.contains(&"missing:requested_ids:6d697373696e67207368617264".to_owned())
        );
        assert!(
            detail_codes.contains(&"demote:target_hot_bytes:64656d6f7465207368617264".to_owned())
        );
        assert!(detail_codes.contains(&"keep_hot:686f74207368617264".to_owned()));
        assert!(
            evidence
                .summary_line()
                .contains("promote:requested_ids:636f6c64207368617264")
        );
    }

    #[test]
    fn startup_evidence_aggregates_kvswap_action_counters_across_rows() {
        let shard_payload_secret = "KVSWAP_ACTION_SHARD_SECRET_DO_NOT_LOG";
        let evidence = MemoryServiceStartupEvidence {
            requires_operator_review: true,
            approved_phases: vec![MemoryMigrationPhase::ReadOnlyShadow],
            lines: vec![
                "kvswap_prefetch promote=1 missing=1 hot=1 duplicate=0 reason=requested_ids promote_id_hex=636f6c64 missing_id_hex=6d697373696e67 hot_id_hex=686f74 duplicate_id_hex=none reason_codes=prefetch_already_hot|prefetch_missing|prefetch_promote|requested_ids detail_codes=already_hot:686f74|missing:requested_ids:6d697373696e67|promote:requested_ids:636f6c64".to_owned(),
                "kvswap_prefetch promote=1 missing=0 hot=0 duplicate=1 reason=tiered_memory_promotions promote_id_hex=636f6c642d32 missing_id_hex=none hot_id_hex=none duplicate_id_hex=636f6c642d32 reason_codes=prefetch_duplicate|prefetch_promote|tiered_memory_promotions detail_codes=duplicate:tiered_memory_promotions:636f6c642d32|promote:tiered_memory_promotions:636f6c642d32".to_owned(),
                "kvswap_eviction target_hot_bytes=12 demote=1 keep_hot=2 reason=target_hot_bytes demote_id_hex=6f6c64 keep_hot_id_hex=686f74|6e6577 reason_codes=evict_demote|evict_keep_hot|target_hot_bytes detail_codes=demote:target_hot_bytes:6f6c64|keep_hot:686f74|keep_hot:6e6577".to_owned(),
                "kvswap_eviction target_hot_bytes=4 demote=1 keep_hot=0 reason=tiered_memory_demotions demote_id_hex=636f6c64 keep_hot_id_hex=none reason_codes=evict_demote|tiered_memory_demotions detail_codes=demote:tiered_memory_demotions:636f6c64".to_owned(),
            ],
        };

        assert_eq!(evidence.kvswap_prefetch_promote_count(), 2);
        assert_eq!(evidence.kvswap_prefetch_missing_count(), 1);
        assert_eq!(evidence.kvswap_prefetch_already_hot_count(), 1);
        assert_eq!(evidence.kvswap_prefetch_duplicate_count(), 1);
        assert_eq!(
            evidence.kvswap_prefetch_reason_codes(),
            vec![
                "prefetch_already_hot".to_owned(),
                "prefetch_duplicate".to_owned(),
                "prefetch_missing".to_owned(),
                "prefetch_promote".to_owned(),
                "requested_ids".to_owned(),
                "tiered_memory_promotions".to_owned(),
            ]
        );
        assert_eq!(
            evidence.kvswap_prefetch_detail_codes(),
            vec![
                "already_hot:686f74".to_owned(),
                "duplicate:tiered_memory_promotions:636f6c642d32".to_owned(),
                "missing:requested_ids:6d697373696e67".to_owned(),
                "promote:requested_ids:636f6c64".to_owned(),
                "promote:tiered_memory_promotions:636f6c642d32".to_owned(),
            ]
        );
        assert_eq!(
            evidence.kvswap_prefetch_detail_codes_for_action("promote"),
            vec![
                "promote:requested_ids:636f6c64".to_owned(),
                "promote:tiered_memory_promotions:636f6c642d32".to_owned(),
            ]
        );
        assert_eq!(
            evidence.kvswap_prefetch_detail_codes_for_action("missing"),
            vec!["missing:requested_ids:6d697373696e67".to_owned()]
        );
        assert_eq!(
            evidence.kvswap_prefetch_detail_codes_for_reason("tiered_memory_promotions"),
            vec![
                "duplicate:tiered_memory_promotions:636f6c642d32".to_owned(),
                "promote:tiered_memory_promotions:636f6c642d32".to_owned(),
            ]
        );

        assert_eq!(evidence.kvswap_eviction_target_hot_bytes(), 16);
        assert_eq!(evidence.kvswap_eviction_demote_count(), 2);
        assert_eq!(evidence.kvswap_eviction_keep_hot_count(), 2);
        assert_eq!(
            evidence.kvswap_eviction_reason_codes(),
            vec![
                "evict_demote".to_owned(),
                "evict_keep_hot".to_owned(),
                "target_hot_bytes".to_owned(),
                "tiered_memory_demotions".to_owned(),
            ]
        );
        assert_eq!(
            evidence.kvswap_eviction_detail_codes(),
            vec![
                "demote:target_hot_bytes:6f6c64".to_owned(),
                "demote:tiered_memory_demotions:636f6c64".to_owned(),
                "keep_hot:686f74".to_owned(),
                "keep_hot:6e6577".to_owned(),
            ]
        );
        assert_eq!(
            evidence.kvswap_eviction_detail_codes_for_action("demote"),
            vec![
                "demote:target_hot_bytes:6f6c64".to_owned(),
                "demote:tiered_memory_demotions:636f6c64".to_owned(),
            ]
        );
        assert_eq!(
            evidence.kvswap_eviction_detail_codes_for_action("keep_hot"),
            vec!["keep_hot:686f74".to_owned(), "keep_hot:6e6577".to_owned()]
        );
        assert_eq!(
            evidence.kvswap_eviction_detail_codes_for_reason("target_hot_bytes"),
            vec!["demote:target_hot_bytes:6f6c64".to_owned()]
        );
        assert_eq!(
            evidence.kvswap_action_detail_codes(),
            vec![
                "eviction:demote:target_hot_bytes:6f6c64".to_owned(),
                "eviction:demote:tiered_memory_demotions:636f6c64".to_owned(),
                "eviction:keep_hot:686f74".to_owned(),
                "eviction:keep_hot:6e6577".to_owned(),
                "prefetch:already_hot:686f74".to_owned(),
                "prefetch:duplicate:tiered_memory_promotions:636f6c642d32".to_owned(),
                "prefetch:missing:requested_ids:6d697373696e67".to_owned(),
                "prefetch:promote:requested_ids:636f6c64".to_owned(),
                "prefetch:promote:tiered_memory_promotions:636f6c642d32".to_owned(),
            ]
        );
        assert_eq!(
            evidence.kvswap_action_detail_codes_for_stage("prefetch"),
            vec![
                "prefetch:already_hot:686f74".to_owned(),
                "prefetch:duplicate:tiered_memory_promotions:636f6c642d32".to_owned(),
                "prefetch:missing:requested_ids:6d697373696e67".to_owned(),
                "prefetch:promote:requested_ids:636f6c64".to_owned(),
                "prefetch:promote:tiered_memory_promotions:636f6c642d32".to_owned(),
            ]
        );
        assert_eq!(
            evidence.kvswap_action_detail_codes_for_action("demote"),
            vec![
                "eviction:demote:target_hot_bytes:6f6c64".to_owned(),
                "eviction:demote:tiered_memory_demotions:636f6c64".to_owned(),
            ]
        );
        assert_eq!(
            evidence.kvswap_action_detail_codes_for_reason("requested_ids"),
            vec![
                "prefetch:missing:requested_ids:6d697373696e67".to_owned(),
                "prefetch:promote:requested_ids:636f6c64".to_owned(),
            ]
        );
        assert!(!evidence.summary_text().contains(shard_payload_secret));
        assert!(!evidence.summary_line().contains(shard_payload_secret));
    }

    #[test]
    fn startup_evidence_lifts_kvswap_boundary_detail_codes() {
        let boundary = KvSwapBoundaryAudit {
            overlapping_hot_cold_ids: vec!["hot/cold".to_owned()],
            stale_metadata_ids: vec!["stale".to_owned()],
            ..KvSwapBoundaryAudit::default()
        };
        let evidence = MemoryServiceStartupEvidence {
            requires_operator_review: true,
            approved_phases: vec![MemoryMigrationPhase::ReadOnlyShadow],
            lines: vec![boundary.summary_line(), boundary.readiness().summary_line()],
        };
        let detail_codes = evidence.detail_codes();

        assert!(detail_codes.contains(&"overlap:686f742f636f6c64".to_owned()));
        assert!(detail_codes.contains(&"stale_metadata:7374616c65".to_owned()));
        assert!(detail_codes.contains(&"blocker:overlap:686f742f636f6c64".to_owned()));
        assert!(detail_codes.contains(&"warning:stale_metadata:7374616c65".to_owned()));
        assert_eq!(evidence.kvswap_boundary_readiness_report_count(), 1);
        assert_eq!(evidence.kvswap_boundary_ready_count(), 0);
        assert_eq!(evidence.kvswap_boundary_blocker_count(), 1);
        assert_eq!(evidence.kvswap_boundary_warning_count(), 1);
        assert_eq!(
            evidence.kvswap_boundary_blocker_reason_codes(),
            vec!["overlapping_hot_cold".to_owned()]
        );
        assert_eq!(
            evidence.kvswap_boundary_warning_reason_codes(),
            vec!["stale_metadata".to_owned()]
        );
        assert_eq!(
            evidence.kvswap_boundary_readiness_detail_codes(),
            vec![
                "blocker:overlap:686f742f636f6c64".to_owned(),
                "warning:stale_metadata:7374616c65".to_owned()
            ]
        );
        assert!(evidence.summary_line().contains("overlap:686f742f636f6c64"));
        assert!(
            evidence
                .summary_line()
                .contains("kvswap_boundary_detail_codes=overlap:686f742f636f6c64")
        );
    }

    #[test]
    fn startup_evidence_aggregates_kvswap_boundary_issue_counters_across_rows() {
        let shard_payload_secret = "KVSWAP_BOUNDARY_SECRET_DO_NOT_LOG";
        let evidence = MemoryServiceStartupEvidence {
            requires_operator_review: true,
            approved_phases: vec![MemoryMigrationPhase::ReadOnlyShadow],
            lines: vec![
                "kvswap_boundary clean=false issues=3 overlap=1 missing_hot_metadata=1 stale_metadata=1 hot_tier_mismatch=0 cold_tier_mismatch=0 reason_codes=missing_hot_metadata|overlapping_hot_cold|stale_metadata detail_codes=missing_hot_metadata:6d697373696e672d6d657461|overlap:6f7665726c6170|stale_metadata:7374616c65".to_owned(),
                "kvswap_boundary clean=false issues=4 overlap=1 missing_hot_metadata=0 stale_metadata=1 hot_tier_mismatch=1 cold_tier_mismatch=1 reason_codes=cold_tier_mismatch|hot_tier_mismatch|overlapping_hot_cold|stale_metadata detail_codes=cold_tier_mismatch:636f6c64|hot_tier_mismatch:686f74|overlap:6f7665726c6170|stale_metadata:7374616c652d32".to_owned(),
                "kvswap_boundary clean=true issues=0 overlap=0 missing_hot_metadata=0 stale_metadata=0 hot_tier_mismatch=0 cold_tier_mismatch=0 reason_codes=none detail_codes=none".to_owned(),
            ],
        };

        assert_eq!(evidence.kvswap_boundary_report_count(), 3);
        assert_eq!(evidence.kvswap_boundary_clean_report_count(), 1);
        assert_eq!(evidence.kvswap_boundary_review_count(), 2);
        assert_eq!(evidence.kvswap_boundary_issue_count(), 7);
        assert_eq!(evidence.kvswap_boundary_overlap_count(), 2);
        assert_eq!(evidence.kvswap_boundary_missing_hot_metadata_count(), 1);
        assert_eq!(evidence.kvswap_boundary_stale_metadata_count(), 2);
        assert_eq!(evidence.kvswap_boundary_hot_tier_mismatch_count(), 1);
        assert_eq!(evidence.kvswap_boundary_cold_tier_mismatch_count(), 1);
        assert_eq!(
            evidence.kvswap_boundary_reason_codes(),
            vec![
                "cold_tier_mismatch".to_owned(),
                "hot_tier_mismatch".to_owned(),
                "missing_hot_metadata".to_owned(),
                "overlapping_hot_cold".to_owned(),
                "stale_metadata".to_owned(),
            ]
        );
        assert_eq!(
            evidence.kvswap_boundary_detail_codes_for("overlap"),
            vec!["overlap:6f7665726c6170".to_owned()]
        );
        assert_eq!(
            evidence.kvswap_boundary_detail_codes_for("stale_metadata"),
            vec![
                "stale_metadata:7374616c65".to_owned(),
                "stale_metadata:7374616c652d32".to_owned(),
            ]
        );
        assert_eq!(
            evidence.kvswap_boundary_detail_codes_for("hot_tier_mismatch"),
            vec!["hot_tier_mismatch:686f74".to_owned()]
        );
        assert!(!evidence.summary_text().contains(shard_payload_secret));
        assert!(!evidence.summary_line().contains(shard_payload_secret));
    }

    #[test]
    fn startup_evidence_exposes_structured_review_fields() {
        let boundary = KvSwapBoundaryAudit {
            overlapping_hot_cold_ids: vec!["hot/cold".to_owned()],
            ..KvSwapBoundaryAudit::default()
        };
        let evidence = MemoryServiceStartupEvidence {
            requires_operator_review: true,
            approved_phases: vec![MemoryMigrationPhase::ReadOnlyShadow],
            lines: vec![
                "memory_shadow ready=false review=true context_rot_risks=1 context_rot_risk_reason_codes=cross_task_transcript_pollution context_rot_blocker_reason_codes=long_without_clean_gist context_rot_risk_detail_codes=context_rot:rot:cross_task_transcript_pollution kvswap_boundary_issues=1 kvswap_boundary_reason_codes=overlapping_hot_cold detail_codes=none".to_owned(),
                MemoryMigrationEvidence::read_only_source(Some(1)).summary_line(),
                boundary.summary_line(),
            ],
        };

        assert_eq!(evidence.context_rot_risk_count(), 1);
        assert_eq!(
            evidence.context_rot_risk_reason_codes(),
            vec!["cross_task_transcript_pollution".to_owned()]
        );
        assert_eq!(
            evidence.context_rot_risk_detail_codes(),
            vec!["context_rot:rot:cross_task_transcript_pollution".to_owned()]
        );
        assert_eq!(
            evidence.context_rot_blocker_reason_codes(),
            vec!["long_without_clean_gist".to_owned()]
        );
        assert_eq!(evidence.kvswap_boundary_issue_count(), 1);
        assert_eq!(
            evidence.kvswap_boundary_reason_codes(),
            vec!["overlapping_hot_cold".to_owned()]
        );
        assert_eq!(
            evidence.kvswap_boundary_detail_codes(),
            vec!["overlap:686f742f636f6c64".to_owned()]
        );
        assert!(
            evidence
                .migration_evidence_guard_codes()
                .contains(&"copied_fixture_missing".to_owned())
        );
        assert!(
            evidence
                .migration_evidence_detail_codes()
                .contains(&"guard:copied_fixture_missing".to_owned())
        );
        assert!(
            evidence
                .summary_line()
                .contains("context_rot_risk_reason_codes=cross_task_transcript_pollution")
        );
        assert!(
            evidence
                .summary_line()
                .contains("context_rot_blocker_reason_codes=long_without_clean_gist")
        );
        assert!(
            evidence
                .summary_line()
                .contains("migration_guard_codes=copied_fixture_missing")
        );
        assert!(
            evidence
                .summary_line()
                .contains("kvswap_boundary_detail_codes=overlap:686f742f636f6c64")
        );
    }

    #[test]
    fn startup_evidence_preserves_context_rot_blocker_reason_code_order_without_admission_effect() {
        let evidence = MemoryServiceStartupEvidence {
            requires_operator_review: true,
            approved_phases: vec![MemoryMigrationPhase::ReadOnlyShadow],
            lines: vec![
                "memory_shadow ready=true review=false context_rot_risks=2 context_rot_risk_reason_codes=long_without_clean_gist|duplicate_experience context_rot_blocker_reason_codes=shadow_only|duplicate_experience context_rot_risk_detail_codes=context_rot:rot:long_without_clean_gist detail_codes=none".to_owned(),
                "memory_readiness profile=shadow_migration required_write_mode=read_only ready=true review=false missing=0 write_blockers=0 unhealthy=0 warnings=0 missing_codes=none write_blocker_codes=none warning_codes=none".to_owned(),
                "memory_migration_readiness isolated_write_ready=true review=false blockers=0 warnings=0 blocker_codes=none warning_codes=none blocker_detail_codes=none warning_detail_codes=none blocker_details=none warning_details=none".to_owned(),
                "memory_migration_evidence source_read_only=true copied_fixture=false isolated_write_root=false catalog_verified=false checksum_verified=false live_store_targeted=false records=2 guard_codes=none detail_codes=none".to_owned(),
                "memory_context_injection decisions=2 admit=1 summarize=0 reject_budget=0 reject_risk=1 reject_scope=0 reject_score=0 accepted_risk=0 used_tokens=8 reason_codes=missing_clean_gist detail_codes=reject_risk:missing_clean_gist:726f74".to_owned(),
                "context_rot_risk risks=2 reason_codes=long_without_clean_gist|duplicate_experience detail_codes=context_rot:rot:long_without_clean_gist|context_rot:dupe:duplicate_experience".to_owned(),
                "experience_index_quality_gate ready_for_context_injection=false records=2 blockers=2 warnings=0 duplicates=1 refresh=0 compact=1 quarantine=0 missing_clean_gist=0 dirty_clean_gist=0 dirty_gist=0 context_rot_blockers=2 reason_codes=compact_context_rot|duplicate_experience context_rot_blocker_reason_codes=long_without_clean_gist|duplicate_experience|long_without_clean_gist detail_codes=blocker:compact:rot|blocker:duplicate:clean:dupe".to_owned(),
            ],
        };

        assert_eq!(
            evidence.context_rot_blocker_reason_codes(),
            vec![
                "long_without_clean_gist".to_owned(),
                "duplicate_experience".to_owned(),
            ]
        );
        assert_eq!(
            evidence.experience_index_quality_gate_context_rot_blocker_reason_codes(),
            vec![
                "long_without_clean_gist".to_owned(),
                "duplicate_experience".to_owned(),
            ]
        );
        assert!(
            !evidence
                .context_rot_blocker_reason_codes()
                .contains(&"shadow_only".to_owned())
        );
        assert_eq!(evidence.context_injection_decision_count(), 2);
        assert_eq!(evidence.context_injection_admit_count(), 1);
        assert_eq!(evidence.context_injection_reject_risk_count(), 1);
        assert_eq!(
            evidence.context_injection_reason_codes(),
            vec!["missing_clean_gist".to_owned()]
        );
        assert_eq!(evidence.migration_readiness_isolated_write_ready_count(), 1);
        assert_eq!(evidence.migration_readiness_blocker_count(), 0);
        assert!(evidence.migration_readiness_blocker_codes().is_empty());
        assert!(evidence.summary_line().contains(
            "context_rot_blocker_reason_codes=long_without_clean_gist|duplicate_experience"
        ));
        assert!(
            evidence
                .summary_text()
                .contains("live_store_targeted=false")
        );
    }

    fn report_only_context_rot_startup_evidence() -> MemoryServiceStartupEvidence {
        MemoryServiceStartupEvidence {
            requires_operator_review: true,
            approved_phases: vec![MemoryMigrationPhase::ReadOnlyShadow],
            lines: vec![
                "memory_shadow ready=true review=true context_rot_risks=3 context_rot_risk_reason_codes=missing_clean_gist|transcript_anchor_risk|long_without_clean_gist context_rot_blocker_reason_codes=long_without_clean_gist|duplicate_experience context_rot_risk_detail_codes=context_rot:legacy:missing_clean_gist|context_rot:anchor:transcript_anchor_risk|context_rot:long:long_without_clean_gist detail_codes=none".to_owned(),
                "context_rot_risk risks=3 reason_codes=missing_clean_gist|transcript_anchor_risk|long_without_clean_gist detail_codes=context_rot:legacy:missing_clean_gist|context_rot:anchor:transcript_anchor_risk|context_rot:long:long_without_clean_gist".to_owned(),
                "experience_index_quality_gate ready_for_context_injection=false records=3 blockers=2 warnings=1 duplicates=0 refresh=1 compact=1 quarantine=1 missing_clean_gist=1 dirty_clean_gist=0 dirty_gist=0 context_rot_blockers=2 reason_codes=compact_context_rot|missing_clean_gist|quarantine_context_rot|trend_context_rot context_rot_blocker_reason_codes=long_without_clean_gist|duplicate_experience detail_codes=blocker:compact:long|blocker:quarantine:dupe|warning:missing_clean_gist:legacy|warning:trend:context_rot".to_owned(),
                "memory_hygiene_work_plan clean=false total_score=5 lanes=1 next_action=context_rot_review operator_review=true isolation=false action_lanes=context_rot_review action_lane_details=context_rot_review:repair:5:1 dispatch_next=dispatch:operator_review:review:context_rot_review:repair:5:1 dispatch_codes=dispatch:operator_review:review:context_rot_review:repair:5:1 reason_codes=operator_review|context_rot_review|remediation_evidence|trend_evidence detail_codes=next:context_rot_review|lane:context_rot_review:repair:5:1".to_owned(),
                "memory_hygiene_work_queue clean=false total_score=5 items=1 operator_review=1 isolation=0 next_dispatch=dispatch:operator_review:review:context_rot_review:repair:5:1 lanes=context_rot_review priorities=repair dispatch_codes=dispatch:operator_review:review:context_rot_review:repair:5:1 detail_codes=context_rot_review:repair:5:1 reason_codes=items_present|operator_review_required|remediation_evidence|trend_evidence".to_owned(),
                "memory_evolution replay_runs=1 replay_items=1 replay_updates=0 replay_missing=0 invalid_memory_ids=0 context_rot_items=3 live_feedback_items=0 retention_decays=0 retention_removals=0 compaction_merges=0 compaction_removals=0 external_applied=0 external_missing=0 drift_rollbacks=0 index_quality_blockers=2 index_quality_warnings=1 kvswap_boundary_blockers=0 kvswap_boundary_warnings=0 hygiene_pressure_score=5 hygiene_pressure_priority=repair hygiene_pressure_action_lanes=context_rot_review hygiene_pressure_action_lane_details=context_rot_review:repair:5:1 hygiene_work_next_action=context_rot_review hygiene_work_operator_review=true hygiene_work_isolation=false hygiene_pressure_reason_codes=context_rot_pressure hygiene_pressure_detail_codes=context_rot_items:3 reason_codes=context_rot|index_quality_blocker|index_quality_warning|trend_evidence".to_owned(),
                "memory_context_injection decisions=3 admit=2 summarize=1 reject_budget=0 reject_risk=0 reject_scope=0 reject_score=0 accepted_risk=0 used_tokens=12 reason_codes=none detail_codes=none".to_owned(),
                "memory_readiness profile=shadow_migration required_write_mode=read_only ready=true review=false missing=0 write_blockers=0 unhealthy=0 warnings=0 missing_codes=none write_blocker_codes=none warning_codes=none".to_owned(),
                "memory_migration_readiness isolated_write_ready=true review=false blockers=0 warnings=1 blocker_codes=none warning_codes=context_rot_report_only blocker_detail_codes=none warning_detail_codes=context_rot_report_only blocker_details=none warning_details=context_rot_report_only".to_owned(),
                "memory_migration_evidence source_read_only=true copied_fixture=false isolated_write_root=false catalog_verified=false checksum_verified=false live_store_targeted=false records=3 guard_codes=none detail_codes=none".to_owned(),
                "memory_adapter_status name=shadow_report ready=true read_only=true write_mode=read_only capabilities=experience_governance|memory_index|context_injection|memory_evolution records=3 warnings=0 status_codes=read_only warning_codes=none".to_owned(),
                "memory_migration phase=read_only_shadow approved=true required_write_mode=read_only blockers=0 warnings=1 blocker_codes=none warning_codes=context_rot_report_only blocker_detail_codes=none warning_detail_codes=context_rot_report_only".to_owned(),
            ],
        }
    }

    #[test]
    fn context_rot_report_only_evidence_does_not_expand_admission_or_live_write() {
        let evidence = report_only_context_rot_startup_evidence();
        assert_eq!(
            evidence.context_rot_blocker_reason_codes(),
            vec![
                "long_without_clean_gist".to_owned(),
                "duplicate_experience".to_owned(),
            ]
        );
        assert_eq!(
            evidence.experience_index_quality_gate_context_rot_blocker_reason_codes(),
            evidence.context_rot_blocker_reason_codes()
        );
        assert_eq!(
            evidence.memory_hygiene_work_queue_reason_codes(),
            vec![
                "items_present".to_owned(),
                "operator_review_required".to_owned(),
                "remediation_evidence".to_owned(),
                "trend_evidence".to_owned(),
            ]
        );
        assert_eq!(evidence.memory_evolution_context_rot_count(), 3);
        assert_eq!(evidence.context_injection_decision_count(), 3);
        assert_eq!(evidence.context_injection_admit_count(), 2);
        assert_eq!(evidence.context_injection_reject_risk_count(), 0);
        assert_eq!(
            evidence.context_injection_reason_codes(),
            Vec::<String>::new()
        );
        assert_eq!(evidence.migration_readiness_blocker_count(), 0);
        assert_eq!(
            evidence.migration_readiness_blocker_codes(),
            Vec::<String>::new()
        );
        assert_eq!(evidence.migration_approval_approved_count(), 1);
        assert_eq!(evidence.migration_approval_blocker_count(), 0);
        assert_eq!(evidence.migration_approval_live_write_required_count(), 0);
        assert_eq!(evidence.adapter_status_live_write_count(), 0);
        assert_eq!(evidence.adapter_status_read_only_count(), 1);
        assert!(
            evidence
                .summary_text()
                .contains("live_store_targeted=false")
        );
        assert!(!evidence.summary_text().contains("live_store_targeted=true"));
    }

    #[test]
    fn context_rot_report_only_startup_schema_bundle_keeps_all_guard_fields() {
        let evidence = report_only_context_rot_startup_evidence();
        let mut bundle = Vec::new();

        evidence.emit_to(&mut bundle).unwrap();

        let bundle_text = bundle.join("\n");
        let schema_line = bundle.last().expect("startup schema summary line");
        assert!(schema_line.starts_with("memory_startup_evidence "));
        assert!(schema_line.contains(
            "context_rot_blocker_reason_codes=long_without_clean_gist|duplicate_experience"
        ));
        assert!(schema_line.contains(
            "context_rot_risk_detail_codes=context_rot:anchor:transcript_anchor_risk|context_rot:legacy:missing_clean_gist|context_rot:long:long_without_clean_gist"
        ));
        assert!(bundle_text.contains("reason_codes=items_present|operator_review_required|remediation_evidence|trend_evidence"));
        assert!(bundle_text.contains(
            "reason_codes=context_rot|index_quality_blocker|index_quality_warning|trend_evidence"
        ));
        assert!(bundle_text.contains(
            "memory_context_injection decisions=3 admit=2 summarize=1 reject_budget=0 reject_risk=0"
        ));
        assert!(bundle_text.contains(
            "memory_readiness profile=shadow_migration required_write_mode=read_only ready=true review=false missing=0 write_blockers=0"
        ));
        assert!(bundle_text.contains(
            "memory_migration_readiness isolated_write_ready=true review=false blockers=0"
        ));
        assert!(bundle_text.contains(
            "memory_migration phase=read_only_shadow approved=true required_write_mode=read_only blockers=0"
        ));
        assert!(bundle_text.contains("live_store_targeted=false"));
        assert!(!bundle_text.contains("live_store_targeted=true"));
        assert!(!bundle_text.contains("write_mode=live_write"));
        assert!(!bundle_text.contains("required_write_mode=live_write"));
        assert_eq!(
            evidence.context_injection_reason_codes(),
            Vec::<String>::new()
        );
        assert_eq!(evidence.migration_readiness_blocker_count(), 0);
        assert_eq!(evidence.migration_approval_blocker_count(), 0);
        assert_eq!(evidence.adapter_status_live_write_count(), 0);
    }

    #[test]
    fn broader_startup_admission_readiness_bundle_keeps_contract_boundaries() {
        let mut evidence = report_only_context_rot_startup_evidence();
        evidence.lines.extend([
            "kvswap_boundary_readiness ready=false blockers=1 warnings=0 blocker_reason_codes=overlapping_hot_cold warning_reason_codes=none detail_codes=blocker:overlap:6469736b2d6b76".to_owned(),
            "memory_migration_evidence source_read_only=true copied_fixture=true isolated_write_root=true catalog_verified=true checksum_verified=true live_store_targeted=false records=4 guard_codes=none detail_codes=none".to_owned(),
            "memory_migration phase=copied_fixture_write approved=true required_write_mode=isolated_write blockers=0 warnings=0 blocker_codes=none warning_codes=none blocker_detail_codes=none warning_detail_codes=none blocker_details=none warning_details=none".to_owned(),
            "memory_migration phase=live_write approved=false required_write_mode=live_write blockers=1 warnings=0 blocker_codes=live_write_disabled_by_policy warning_codes=none blocker_detail_codes=live_write_disabled_by_policy warning_detail_codes=none blocker_details=live_write_disabled_by_policy warning_details=none".to_owned(),
            "memory_adapter_status name=disk_kv_fixture ready=true read_only=false write_mode=isolated_write capabilities=disk_kv_offload|kv_swap records=4 warnings=0 status_codes=none warning_codes=none".to_owned(),
        ]);
        let mut bundle = Vec::new();

        evidence.emit_to(&mut bundle).unwrap();

        let bundle_text = bundle.join("\n");
        let schema_line = bundle.last().expect("startup schema summary line");
        assert!(schema_line.starts_with("memory_startup_evidence "));
        assert!(schema_line.contains(
            "context_rot_blocker_reason_codes=long_without_clean_gist|duplicate_experience"
        ));
        assert!(bundle_text.contains(
            "kvswap_boundary_readiness ready=false blockers=1 warnings=0 blocker_reason_codes=overlapping_hot_cold"
        ));
        assert_eq!(evidence.kvswap_boundary_readiness_report_count(), 1);
        assert_eq!(evidence.kvswap_boundary_blocker_count(), 1);
        assert_eq!(
            evidence.kvswap_boundary_blocker_reason_codes(),
            vec!["overlapping_hot_cold".to_owned()]
        );
        assert_eq!(
            evidence.kvswap_boundary_readiness_detail_codes(),
            vec!["blocker:overlap:6469736b2d6b76".to_owned()]
        );
        assert_eq!(evidence.context_injection_decision_count(), 3);
        assert_eq!(evidence.context_injection_admit_count(), 2);
        assert_eq!(evidence.context_injection_reject_risk_count(), 0);
        assert_eq!(
            evidence.context_injection_reason_codes(),
            Vec::<String>::new()
        );
        assert_eq!(evidence.migration_readiness_report_count(), 1);
        assert_eq!(evidence.migration_readiness_blocker_count(), 0);
        assert_eq!(
            evidence.migration_readiness_blocker_codes(),
            Vec::<String>::new()
        );
        assert_eq!(evidence.migration_approval_count(), 3);
        assert_eq!(evidence.migration_approval_approved_count(), 2);
        assert_eq!(evidence.migration_approval_blocked_count(), 1);
        assert_eq!(evidence.migration_approval_live_write_required_count(), 1);
        assert_eq!(
            evidence.migration_approval_blocker_codes(),
            vec!["live_write_disabled_by_policy".to_owned()]
        );
        assert_eq!(evidence.adapter_status_live_write_count(), 0);
        assert_eq!(evidence.adapter_status_isolated_write_count(), 1);
        assert!(bundle_text.contains("catalog_verified=true checksum_verified=true"));
        assert!(bundle_text.contains("live_store_targeted=false"));
        assert!(bundle_text.contains("phase=live_write approved=false"));
        assert!(bundle_text.contains("blocker_codes=live_write_disabled_by_policy"));
        assert!(!bundle_text.contains("live_store_targeted=true"));
        assert!(!bundle_text.contains("memory_adapter_status name=disk_kv_fixture ready=true read_only=false write_mode=live_write"));
        assert!(
            evidence
                .status_codes()
                .contains(&"incomplete_evidence".to_owned())
        );
        assert!(
            evidence
                .status_codes()
                .contains(&"phases_approved".to_owned())
        );
        assert!(
            evidence
                .status_codes()
                .contains(&"operator_review_required".to_owned())
        );
    }

    #[derive(Debug, Default, PartialEq, Eq)]
    struct StartupBundleConsumerView {
        consumed_fields: std::collections::BTreeSet<&'static str>,
        readiness_ready: bool,
        readiness_write_blockers: usize,
        admission_decisions: usize,
        admission_accepted: usize,
        admission_risk_rejections: usize,
        report_only_context_rot: bool,
        migration_isolated_write_ready: bool,
        migration_blockers: usize,
        live_store_targeted: bool,
        adapter_live_write_statuses: usize,
        live_write_requests: usize,
        store_mutations: usize,
    }

    impl StartupBundleConsumerView {
        fn from_startup_bundle(lines: &[String]) -> Self {
            let mut view = Self::default();

            for line in lines {
                if line.starts_with("memory_readiness ") {
                    view.consumed_fields.insert("memory_readiness.ready");
                    view.consumed_fields
                        .insert("memory_readiness.write_blockers");
                    view.readiness_ready |= startup_bool_from_line(line, "ready=");
                    view.readiness_write_blockers = view
                        .readiness_write_blockers
                        .saturating_add(startup_usize_from_line(line, "write_blockers="));
                } else if line.starts_with("memory_context_injection ") {
                    view.consumed_fields
                        .insert("memory_context_injection.decisions");
                    view.consumed_fields
                        .insert("memory_context_injection.admit");
                    view.consumed_fields
                        .insert("memory_context_injection.reject_risk");
                    view.admission_decisions = view
                        .admission_decisions
                        .saturating_add(startup_usize_from_line(line, "decisions="));
                    view.admission_accepted = view
                        .admission_accepted
                        .saturating_add(startup_usize_from_line(line, "admit="));
                    view.admission_risk_rejections = view
                        .admission_risk_rejections
                        .saturating_add(startup_usize_from_line(line, "reject_risk="));
                } else if line.starts_with("memory_migration_readiness ") {
                    view.consumed_fields
                        .insert("memory_migration_readiness.isolated_write_ready");
                    view.consumed_fields
                        .insert("memory_migration_readiness.blockers");
                    view.consumed_fields
                        .insert("memory_migration_readiness.warning_codes");
                    view.migration_isolated_write_ready |=
                        startup_bool_from_line(line, "isolated_write_ready=");
                    view.migration_blockers = view
                        .migration_blockers
                        .saturating_add(startup_usize_from_line(line, "blockers="));
                    view.report_only_context_rot |=
                        startup_line_field_value(line, "warning_codes=").is_some_and(|codes| {
                            split_startup_codes(codes).any(|code| code == "context_rot_report_only")
                        });
                } else if line.starts_with("memory_migration_evidence ") {
                    view.consumed_fields
                        .insert("memory_migration_evidence.live_store_targeted");
                    view.live_store_targeted |=
                        startup_bool_from_line(line, "live_store_targeted=");
                } else if line.starts_with("memory_adapter_status ") {
                    view.consumed_fields
                        .insert("memory_adapter_status.write_mode");
                    if startup_line_field_value(line, "write_mode=") == Some("live_write") {
                        view.adapter_live_write_statuses =
                            view.adapter_live_write_statuses.saturating_add(1);
                    }
                }
            }

            if view.live_store_targeted || view.adapter_live_write_statuses > 0 {
                view.live_write_requests = 1;
            }

            view
        }
    }

    fn startup_bool_from_line(line: &str, field: &str) -> bool {
        startup_line_field_value(line, field)
            .and_then(|value| value.parse::<bool>().ok())
            .unwrap_or_default()
    }

    fn startup_usize_from_line(line: &str, field: &str) -> usize {
        startup_line_field_value(line, field)
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or_default()
    }

    #[test]
    fn startup_bundle_consumer_contract_reads_stable_fields_without_live_write() {
        let mut evidence = report_only_context_rot_startup_evidence();
        evidence.lines.extend([
            "kvswap_boundary_readiness ready=false blockers=1 warnings=0 blocker_reason_codes=overlapping_hot_cold warning_reason_codes=none detail_codes=blocker:overlap:6469736b2d6b76".to_owned(),
            "memory_migration_evidence source_read_only=true copied_fixture=true isolated_write_root=true catalog_verified=true checksum_verified=true live_store_targeted=false records=4 guard_codes=none detail_codes=none".to_owned(),
            "memory_migration phase=copied_fixture_write approved=true required_write_mode=isolated_write blockers=0 warnings=0 blocker_codes=none warning_codes=none blocker_detail_codes=none warning_detail_codes=none blocker_details=none warning_details=none".to_owned(),
            "memory_migration phase=live_write approved=false required_write_mode=live_write blockers=1 warnings=0 blocker_codes=live_write_disabled_by_policy warning_codes=none blocker_detail_codes=live_write_disabled_by_policy warning_detail_codes=none blocker_details=live_write_disabled_by_policy warning_details=none".to_owned(),
            "memory_adapter_status name=disk_kv_fixture ready=true read_only=false write_mode=isolated_write capabilities=disk_kv_offload|kv_swap records=4 warnings=0 status_codes=none warning_codes=none".to_owned(),
        ]);
        let mut bundle = Vec::new();

        evidence.emit_to(&mut bundle).unwrap();
        let consumer = StartupBundleConsumerView::from_startup_bundle(&bundle);

        assert_eq!(
            consumer.consumed_fields,
            [
                "memory_adapter_status.write_mode",
                "memory_context_injection.admit",
                "memory_context_injection.decisions",
                "memory_context_injection.reject_risk",
                "memory_migration_evidence.live_store_targeted",
                "memory_migration_readiness.blockers",
                "memory_migration_readiness.isolated_write_ready",
                "memory_migration_readiness.warning_codes",
                "memory_readiness.ready",
                "memory_readiness.write_blockers",
            ]
            .into_iter()
            .collect()
        );
        assert!(consumer.readiness_ready);
        assert_eq!(consumer.readiness_write_blockers, 0);
        assert_eq!(consumer.admission_decisions, 3);
        assert_eq!(consumer.admission_accepted, 2);
        assert_eq!(consumer.admission_risk_rejections, 0);
        assert!(consumer.report_only_context_rot);
        assert!(consumer.migration_isolated_write_ready);
        assert_eq!(consumer.migration_blockers, 0);
        assert!(!consumer.live_store_targeted);
        assert_eq!(consumer.adapter_live_write_statuses, 0);
        assert_eq!(consumer.live_write_requests, 0);
        assert_eq!(consumer.store_mutations, 0);
        assert!(bundle.iter().any(|line| line.contains("phase=live_write")));
        assert!(
            bundle
                .iter()
                .any(|line| line.contains("blocker_codes=live_write_disabled_by_policy"))
        );
    }

    #[derive(Debug, Default, PartialEq, Eq)]
    struct IndexQualityBundleConsumerView {
        consumed_fields: std::collections::BTreeSet<&'static str>,
        read_only_review: bool,
        quality_blockers: usize,
        quality_warnings: usize,
        index_operations: usize,
        index_refreshes: usize,
        index_detail_codes: Vec<String>,
        admission_decisions: usize,
        admission_risk_rejections: usize,
        migration_live_store_targeted: bool,
        adapter_live_write_statuses: usize,
        helper_prose_lines_seen: usize,
        store_mutations: usize,
        live_write_requests: usize,
    }

    impl IndexQualityBundleConsumerView {
        fn from_startup_bundle(lines: &[String]) -> Self {
            let mut view = Self::default();

            for line in lines {
                if line.starts_with("memory_read_only_plan ") {
                    view.consumed_fields.insert("memory_read_only_plan.review");
                    view.read_only_review |= startup_bool_from_line(line, "review=");
                } else if line.starts_with("experience_index_quality_gate ") {
                    view.consumed_fields
                        .insert("experience_index_quality_gate.blockers");
                    view.consumed_fields
                        .insert("experience_index_quality_gate.warnings");
                    view.quality_blockers = view
                        .quality_blockers
                        .saturating_add(startup_usize_from_line(line, "blockers="));
                    view.quality_warnings = view
                        .quality_warnings
                        .saturating_add(startup_usize_from_line(line, "warnings="));
                } else if line.starts_with("memory_index_plan ") {
                    view.consumed_fields.insert("memory_index_plan.operations");
                    view.consumed_fields.insert("memory_index_plan.refresh");
                    view.consumed_fields
                        .insert("memory_index_plan.detail_codes");
                    view.index_operations = view
                        .index_operations
                        .saturating_add(startup_usize_from_line(line, "operations="));
                    view.index_refreshes = view
                        .index_refreshes
                        .saturating_add(startup_usize_from_line(line, "refresh="));
                    if let Some(codes) = startup_line_field_value(line, "detail_codes=") {
                        view.index_detail_codes.extend(split_startup_codes(codes));
                    }
                } else if line.starts_with("memory_context_injection ") {
                    view.consumed_fields
                        .insert("memory_context_injection.decisions");
                    view.consumed_fields
                        .insert("memory_context_injection.reject_risk");
                    view.admission_decisions = view
                        .admission_decisions
                        .saturating_add(startup_usize_from_line(line, "decisions="));
                    view.admission_risk_rejections = view
                        .admission_risk_rejections
                        .saturating_add(startup_usize_from_line(line, "reject_risk="));
                } else if line.starts_with("memory_migration_evidence ") {
                    view.consumed_fields
                        .insert("memory_migration_evidence.live_store_targeted");
                    view.migration_live_store_targeted |=
                        startup_bool_from_line(line, "live_store_targeted=");
                } else if line.starts_with("memory_adapter_status ") {
                    view.consumed_fields
                        .insert("memory_adapter_status.write_mode");
                    if startup_line_field_value(line, "write_mode=") == Some("live_write") {
                        view.adapter_live_write_statuses =
                            view.adapter_live_write_statuses.saturating_add(1);
                    }
                } else if line.starts_with("helper_prose ") {
                    view.helper_prose_lines_seen = view.helper_prose_lines_seen.saturating_add(1);
                }
            }

            view.index_detail_codes.sort();
            view.index_detail_codes.dedup();
            if view.migration_live_store_targeted || view.adapter_live_write_statuses > 0 {
                view.live_write_requests = 1;
            }

            view
        }
    }

    #[test]
    fn startup_index_quality_bundle_consumer_ignores_helper_prose_and_live_write_advice() {
        let mut evidence = report_only_context_rot_startup_evidence();
        evidence.lines.extend([
            "memory_read_only_plan adapter=shadow_report write_mode=read_only review=true governance_noisy=1 context_rot=3 rebuild=true repair_items=1 repair_skipped=0 index_operations=2 context_rejections=0 tiers_hot=0 tiers_warm=0 tiers_cold=0 kvswap_prefetch=0 kvswap_evict=0 reason_codes=quality_gate:missing_clean_gist|index:repair_missing_or_dirty_clean_gist detail_codes=index:refresh_embedding:refresh_missing_clean_gist:6c6567616379".to_owned(),
            "memory_index_plan rebuild=true operations=2 upsert=1 refresh=1 compact=0 quarantine=0 delete_duplicate=0 skipped=0 reasons=2 reason_codes=full_rebuild_requested|repair_missing_or_dirty_clean_gist detail_codes=refresh_embedding:refresh_missing_clean_gist:6c6567616379".to_owned(),
            "helper_prose suggestion=enable_live_write_and_rewrite_real_ndkv write_mode=live_write live_store_targeted=true store_mutations=9".to_owned(),
        ]);
        let mut bundle = Vec::new();

        evidence.emit_to(&mut bundle).unwrap();
        let consumer = IndexQualityBundleConsumerView::from_startup_bundle(&bundle);

        assert_eq!(
            consumer.consumed_fields,
            [
                "experience_index_quality_gate.blockers",
                "experience_index_quality_gate.warnings",
                "memory_adapter_status.write_mode",
                "memory_context_injection.decisions",
                "memory_context_injection.reject_risk",
                "memory_index_plan.detail_codes",
                "memory_index_plan.operations",
                "memory_index_plan.refresh",
                "memory_migration_evidence.live_store_targeted",
                "memory_read_only_plan.review",
            ]
            .into_iter()
            .collect()
        );
        assert!(consumer.read_only_review);
        assert_eq!(consumer.quality_blockers, 2);
        assert_eq!(consumer.quality_warnings, 1);
        assert_eq!(consumer.index_operations, 2);
        assert_eq!(consumer.index_refreshes, 1);
        assert_eq!(
            consumer.index_detail_codes,
            vec!["refresh_embedding:refresh_missing_clean_gist:6c6567616379".to_owned()]
        );
        assert_eq!(consumer.admission_decisions, 3);
        assert_eq!(consumer.admission_risk_rejections, 0);
        assert_eq!(consumer.helper_prose_lines_seen, 1);
        assert!(!consumer.migration_live_store_targeted);
        assert_eq!(consumer.adapter_live_write_statuses, 0);
        assert_eq!(consumer.live_write_requests, 0);
        assert_eq!(consumer.store_mutations, 0);
        assert_eq!(evidence.adapter_status_live_write_count(), 0);
        assert_eq!(evidence.migration_evidence_live_store_targeted_count(), 0);
        assert_eq!(evidence.memory_index_operation_count(), 2);
        assert_eq!(evidence.memory_index_refresh_count(), 1);
        assert_eq!(
            evidence.memory_index_detail_codes_for_reason("refresh_missing_clean_gist"),
            vec!["refresh_embedding:refresh_missing_clean_gist:6c6567616379".to_owned()]
        );
        assert!(bundle.iter().any(|line| line.starts_with("helper_prose ")));
        assert!(
            bundle
                .iter()
                .any(|line| line.contains("write_mode=live_write"))
        );
    }

    #[test]
    fn startup_admission_evidence_ignores_helper_and_old_window_payloads() {
        let mut evidence = report_only_context_rot_startup_evidence();
        evidence.lines.extend([
            "memory_read_only_plan adapter=shadow_report write_mode=read_only review=true governance_noisy=1 context_rot=3 rebuild=true repair_items=1 repair_skipped=0 index_operations=2 context_rejections=0 tiers_hot=0 tiers_warm=0 tiers_cold=0 kvswap_prefetch=0 kvswap_evict=0 reason_codes=quality_gate:missing_clean_gist|index:repair_missing_or_dirty_clean_gist detail_codes=index:refresh_embedding:refresh_missing_clean_gist:6c6567616379".to_owned(),
            "memory_index_plan rebuild=true operations=2 upsert=1 refresh=1 compact=0 quarantine=0 delete_duplicate=0 skipped=0 reasons=2 reason_codes=full_rebuild_requested|repair_missing_or_dirty_clean_gist detail_codes=refresh_embedding:refresh_missing_clean_gist:6c6567616379".to_owned(),
            "helper_prose suggestion=rewrite_real_ndkv write_mode=live_write live_store_targeted=true store_mutations=9 admission_decisions=99".to_owned(),
            "old_window_payload thread=stale write_mode=live_write live_store_targeted=true store_mutations=42 ndkv_path=prod.ndkv admission_decisions=88".to_owned(),
        ]);
        let mut bundle = Vec::new();

        evidence.emit_to(&mut bundle).unwrap();
        let report = MemoryStartupAdmissionEvidence::from_startup_evidence(&evidence);
        let report_from_bundle =
            MemoryStartupAdmissionEvidence::from_startup_evidence(&MemoryServiceStartupEvidence {
                requires_operator_review: evidence.requires_operator_review,
                approved_phases: evidence.approved_phases.clone(),
                lines: bundle,
            });

        assert_eq!(report, report_from_bundle);
        assert!(report.read_only_contract_holds());
        assert!(report.read_only_review_required);
        assert_eq!(report.index_quality_blocker_count, 2);
        assert_eq!(report.index_quality_warning_count, 1);
        assert_eq!(report.index_operation_count, 2);
        assert_eq!(report.index_refresh_count, 1);
        assert_eq!(
            report.index_detail_codes,
            vec!["refresh_embedding:refresh_missing_clean_gist:6c6567616379".to_owned()]
        );
        assert_eq!(report.context_rot_risk_count, 3);
        assert_eq!(report.admission_decision_count, 3);
        assert_eq!(report.admission_accepted_count, 3);
        assert_eq!(report.admission_risk_rejection_count, 0);
        assert_eq!(report.migration_live_store_targeted_count, 0);
        assert_eq!(report.adapter_live_write_count, 0);
        assert_eq!(report.live_write_phase_request_count, 0);
        assert_eq!(report.store_mutation_count, 0);
        assert_eq!(report.helper_prose_line_count, 1);
        assert_eq!(report.non_contract_line_count, 2);
        assert!(!report.live_store_mutation_requested());
        assert!(!report.ndkv_write_allowed());
        assert!(!report.admission_expanded_by_non_contract_evidence());

        let summary = report.summary_line();
        assert!(summary.contains("read_only_contract=true"));
        assert!(summary.contains("live_store_mutation_requested=false"));
        assert!(summary.contains("store_mutations=0"));
        assert!(summary.contains("ndkv_write_allowed=false"));
        assert!(summary.contains("helper_prose_lines=1"));
        assert!(summary.contains("non_contract_lines=2"));
        assert!(summary.contains("admission_decisions=3"));
        assert!(!summary.contains("prod.ndkv"));
        assert!(!summary.contains("admission_decisions=99"));
        assert!(!summary.contains("store_mutations=9"));
        assert!(!summary.contains("store_mutations=42"));
    }

    #[derive(Debug, PartialEq, Eq)]
    struct CleanRoomAdmissionConsumerView {
        read_only_safe: bool,
        index_safe: bool,
        admission_safe: bool,
        side_effect_safe: bool,
        observed_non_contract_lines: usize,
        observed_helper_lines: usize,
        observed_payload_lines: usize,
        index_detail_codes: Vec<String>,
        admission_decisions: usize,
    }

    impl CleanRoomAdmissionConsumerView {
        fn from_startup_evidence(evidence: &MemoryServiceStartupEvidence) -> Self {
            let admission = MemoryStartupAdmissionEvidence::from_startup_evidence(evidence);
            let observed_payload_lines = evidence
                .lines
                .iter()
                .filter(|line| {
                    line.starts_with("old_window_payload ")
                        || line.starts_with("polluted_context_payload ")
                })
                .count();

            Self {
                read_only_safe: admission.read_only_contract_holds(),
                index_safe: admission
                    .index_detail_codes
                    .iter()
                    .all(|code| !code.contains("OLD_WINDOW") && !code.contains("prod.ndkv")),
                admission_safe: admission.admission_accepted_count
                    <= admission.admission_decision_count
                    && !admission.admission_expanded_by_non_contract_evidence(),
                side_effect_safe: !admission.live_store_mutation_requested()
                    && admission.store_mutation_count == 0
                    && !admission.ndkv_write_allowed(),
                observed_non_contract_lines: admission.non_contract_line_count,
                observed_helper_lines: admission.helper_prose_line_count,
                observed_payload_lines,
                index_detail_codes: admission.index_detail_codes,
                admission_decisions: admission.admission_decision_count,
            }
        }
    }

    #[test]
    fn clean_room_handoff_admission_consumer_keeps_payloads_out_of_index_and_writes() {
        let mut evidence = report_only_context_rot_startup_evidence();
        evidence.lines.extend([
            "memory_read_only_plan adapter=shadow_report write_mode=read_only review=true governance_noisy=1 context_rot=3 rebuild=true repair_items=1 repair_skipped=0 index_operations=2 context_rejections=0 tiers_hot=0 tiers_warm=0 tiers_cold=0 kvswap_prefetch=0 kvswap_evict=0 reason_codes=quality_gate:missing_clean_gist|index:repair_missing_or_dirty_clean_gist detail_codes=index:refresh_embedding:refresh_missing_clean_gist:6c6567616379".to_owned(),
            "memory_index_plan rebuild=true operations=2 upsert=1 refresh=1 compact=0 quarantine=0 delete_duplicate=0 skipped=0 reasons=2 reason_codes=full_rebuild_requested|repair_missing_or_dirty_clean_gist detail_codes=refresh_embedding:refresh_missing_clean_gist:6c6567616379".to_owned(),
            "clean_room_handoff_report_v1 source=clean-room evidence_ids=mem-admission-r23|agent-clean-room-r24 expands_memory_admission=false mutates_memory_store=false writes_ndkv=false side_effects_allowed=false".to_owned(),
            "memory_startup_admission_status schema=memory_startup_admission_status_v1 read_only_contract=true store_mutations=0 ndkv_write_allowed=false admission_expanded_by_non_contract=false".to_owned(),
            "helper_prose suggestion=please_reindex_prod_ndkv write_mode=live_write live_store_targeted=true store_mutations=9 index_detail_codes=OLD_WINDOW_PAYLOAD".to_owned(),
            "old_window_payload thread=stale write_mode=live_write live_store_targeted=true store_mutations=42 ndkv_path=prod.ndkv admission_decisions=88 index_detail_codes=OLD_WINDOW_PAYLOAD".to_owned(),
            "polluted_context_payload source=legacy_chat write_mode=live_write live_store_targeted=true store_mutations=7 ndkv_path=prod.ndkv admission_decisions=77 memory_index_plan=poison".to_owned(),
        ]);
        let mut bundle = Vec::new();

        evidence.emit_to(&mut bundle).unwrap();
        let bundle_evidence = MemoryServiceStartupEvidence {
            requires_operator_review: evidence.requires_operator_review,
            approved_phases: evidence.approved_phases.clone(),
            lines: bundle,
        };
        let view = CleanRoomAdmissionConsumerView::from_startup_evidence(&bundle_evidence);
        let admission = MemoryStartupAdmissionEvidence::from_startup_evidence(&bundle_evidence);

        assert!(view.read_only_safe);
        assert!(view.index_safe);
        assert!(view.admission_safe);
        assert!(view.side_effect_safe);
        assert_eq!(view.observed_helper_lines, 1);
        assert_eq!(view.observed_payload_lines, 2);
        assert_eq!(view.observed_non_contract_lines, 5);
        assert_eq!(view.admission_decisions, 3);
        assert_eq!(
            view.index_detail_codes,
            vec!["refresh_embedding:refresh_missing_clean_gist:6c6567616379".to_owned()]
        );
        assert_eq!(admission.index_operation_count, 2);
        assert_eq!(admission.index_refresh_count, 1);
        assert_eq!(admission.migration_live_store_targeted_count, 0);
        assert_eq!(admission.adapter_live_write_count, 0);
        assert_eq!(admission.live_write_phase_request_count, 0);
        assert_eq!(admission.store_mutation_count, 0);
        assert!(!admission.live_store_mutation_requested());
        assert!(!admission.ndkv_write_allowed());
        assert!(!admission.admission_expanded_by_non_contract_evidence());

        let summary = admission.summary_line();
        assert!(summary.contains("read_only_contract=true"));
        assert!(summary.contains("non_contract_lines=5"));
        assert!(summary.contains("admission_decisions=3"));
        assert!(summary.contains("store_mutations=0"));
        assert!(summary.contains("ndkv_write_allowed=false"));
        assert!(!summary.contains("prod.ndkv"));
        assert!(!summary.contains("OLD_WINDOW_PAYLOAD"));
        assert!(!summary.contains("admission_decisions=77"));
        assert!(!summary.contains("admission_decisions=88"));
    }

    #[test]
    fn startup_evidence_prefers_direct_context_rot_risk_over_shadow_summary() {
        let evidence = MemoryServiceStartupEvidence {
            requires_operator_review: true,
            approved_phases: vec![MemoryMigrationPhase::ReadOnlyShadow],
            lines: vec![
                "memory_shadow ready=false review=true context_rot_risks=9 context_rot_risk_reason_codes=shadow_reason context_rot_blocker_reason_codes=shadow_blocker context_rot_risk_detail_codes=context_rot:shadow:shadow_reason detail_codes=none".to_owned(),
                "context_rot_risk risks=2 reason_codes=missing_clean_gist|transcript_anchor_risk detail_codes=context_rot:gist:missing_clean_gist|context_rot:anchor:transcript_anchor_risk".to_owned(),
                "experience_index_quality_gate ready_for_context_injection=false records=2 blockers=1 warnings=1 duplicates=0 refresh=0 compact=1 quarantine=0 missing_clean_gist=0 dirty_clean_gist=0 dirty_gist=0 context_rot_blockers=1 reason_codes=compact_context_rot context_rot_blocker_reason_codes=long_without_clean_gist detail_codes=blocker:compact:gist".to_owned(),
            ],
        };

        assert_eq!(evidence.context_rot_risk_count(), 2);
        assert_eq!(
            evidence.context_rot_risk_reason_codes(),
            vec![
                "missing_clean_gist".to_owned(),
                "transcript_anchor_risk".to_owned()
            ]
        );
        assert_eq!(
            evidence.context_rot_risk_detail_codes(),
            vec![
                "context_rot:anchor:transcript_anchor_risk".to_owned(),
                "context_rot:gist:missing_clean_gist".to_owned()
            ]
        );
        assert_eq!(
            evidence.context_rot_risk_detail_codes_for_reason("missing_clean_gist"),
            vec!["context_rot:gist:missing_clean_gist".to_owned()]
        );
        assert_eq!(
            evidence.context_rot_blocker_reason_codes(),
            vec!["long_without_clean_gist".to_owned()]
        );
        assert_eq!(
            evidence.context_rot_risk_reason_count("transcript_anchor_risk"),
            1
        );
        let dispatch_pressure = evidence.hygiene_dispatch_pressure_summary();
        assert_eq!(dispatch_pressure.context_rot_risks, 2);
        assert_eq!(dispatch_pressure.missing_clean_gist_pressure, 1);
        assert!(dispatch_pressure.has_pressure());
        assert!(dispatch_pressure.requires_operator_review());
        assert!(!dispatch_pressure.requires_isolation());
        assert_eq!(dispatch_pressure.priority_code(), "review");
        assert_eq!(dispatch_pressure.dispatch_rank(), 2);
        assert_eq!(
            dispatch_pressure.reason_codes(),
            vec![
                "context_rot_risk".to_owned(),
                "missing_clean_gist".to_owned(),
            ]
        );
        assert!(
            evidence
                .detail_codes()
                .contains(&"context_rot:gist:missing_clean_gist".to_owned())
        );
        assert!(
            !evidence
                .summary_line()
                .contains("context_rot_risk_reason_codes=shadow_reason")
        );
        assert!(
            !evidence
                .summary_line()
                .contains("context_rot_blocker_reason_codes=shadow_blocker")
        );
    }

    #[test]
    fn startup_evidence_lifts_repair_and_index_detail_codes() {
        let evidence = MemoryServiceStartupEvidence {
            requires_operator_review: true,
            approved_phases: vec![MemoryMigrationPhase::ReadOnlyShadow],
            lines: vec![
                "memory_repair_plan empty=false items=1 skipped=1 repair_clean_gist=1 compact_context=0 quarantine=0 delete_duplicate=0 skipped_repair_clean_gist=1 skipped_compact_context=0 skipped_quarantine=0 skipped_delete_duplicate=0 reason_codes=repair_legacy_metadata_lesson skipped_reason_codes=missing_clean_gist detail_codes=repair_clean_gist:repair_legacy_metadata_lesson:726570616972|skipped:repair_clean_gist:missing_clean_gist:6c6567616379".to_owned(),
                "memory_index_plan rebuild=true operations=1 upsert=0 refresh=0 compact=0 quarantine=0 delete_duplicate=1 skipped=1 reasons=1 reason_codes=deduplicate_exact_fingerprints detail_codes=delete_duplicate:deduplicate_exact_fingerprint:647570|skipped:6d697373696e67".to_owned(),
            ],
        };
        let detail_codes = evidence.detail_codes();

        assert!(
            detail_codes.contains(
                &"repair_clean_gist:repair_legacy_metadata_lesson:726570616972".to_owned()
            )
        );
        assert!(
            detail_codes
                .contains(&"skipped:repair_clean_gist:missing_clean_gist:6c6567616379".to_owned())
        );
        assert!(
            detail_codes
                .contains(&"delete_duplicate:deduplicate_exact_fingerprint:647570".to_owned())
        );
        assert!(detail_codes.contains(&"skipped:6d697373696e67".to_owned()));
        assert_eq!(
            evidence.memory_index_reason_codes(),
            vec!["deduplicate_exact_fingerprints".to_owned()]
        );
        assert_eq!(
            evidence.memory_index_detail_codes(),
            vec![
                "delete_duplicate:deduplicate_exact_fingerprint:647570".to_owned(),
                "skipped:6d697373696e67".to_owned(),
            ]
        );
        assert!(
            evidence
                .summary_line()
                .contains("delete_duplicate:deduplicate_exact_fingerprint:647570")
        );
    }

    #[test]
    fn startup_evidence_reads_clean_gist_repair_from_shadow_before_rebuild_line() {
        let evidence = MemoryServiceStartupEvidence {
            requires_operator_review: true,
            approved_phases: vec![MemoryMigrationPhase::ReadOnlyShadow],
            lines: vec![
                "memory_shadow ready=false review=true clean_gist_repair_missing_clean_gist=2 clean_gist_repair_dirty_clean_gist=1 clean_gist_repair_dirty_gist=1 clean_gist_repair_detail_codes=dirty_clean_gist:shadow_dirty|dirty_gist:shadow_legacy|missing_clean_gist:shadow_missing detail_codes=none".to_owned(),
                "memory_rebuild required=true duplicate_groups=0 refresh=0 compact=0 quarantine=0 missing_clean_gist=9 dirty_clean_gist=9 dirty_gist=9 reasons=1 reason_codes=repair_missing_or_dirty_clean_gist detail_codes=missing_clean_gist:stale|dirty_clean_gist:stale|dirty_gist:stale".to_owned(),
            ],
        };

        assert_eq!(evidence.clean_gist_repair_missing_clean_gist_count(), 2);
        assert_eq!(evidence.clean_gist_repair_dirty_clean_gist_count(), 1);
        assert_eq!(evidence.clean_gist_repair_dirty_gist_count(), 1);
        assert_eq!(evidence.clean_gist_repair_issue_count(), 4);
        assert_eq!(
            evidence.clean_gist_repair_detail_codes(),
            vec![
                "dirty_clean_gist:shadow_dirty".to_owned(),
                "dirty_gist:shadow_legacy".to_owned(),
                "missing_clean_gist:shadow_missing".to_owned(),
            ]
        );
        assert!(
            !evidence
                .clean_gist_repair_detail_codes()
                .contains(&"missing_clean_gist:stale".to_owned())
        );
        assert!(
            evidence
                .status_codes()
                .contains(&"clean_gist_repair_required".to_owned())
        );
    }

    #[test]
    fn startup_evidence_prefers_clean_gist_repair_projection_over_rebuild_line() {
        let evidence = MemoryServiceStartupEvidence {
            requires_operator_review: true,
            approved_phases: vec![MemoryMigrationPhase::ReadOnlyShadow],
            lines: vec![
                "memory_rebuild required=true duplicate_groups=0 refresh=0 compact=0 quarantine=0 missing_clean_gist=9 dirty_clean_gist=9 dirty_gist=9 reasons=1 reason_codes=repair_missing_or_dirty_clean_gist detail_codes=missing_clean_gist:stale|dirty_clean_gist:stale|dirty_gist:stale".to_owned(),
                "clean_gist_repair missing_clean_gist=2 dirty_clean_gist=1 dirty_gist=1 detail_codes=dirty_clean_gist:dirty|dirty_gist:legacy|missing_clean_gist:missing".to_owned(),
            ],
        };

        assert_eq!(evidence.memory_rebuild_missing_clean_gist_count(), 2);
        assert_eq!(evidence.memory_rebuild_dirty_clean_gist_count(), 1);
        assert_eq!(evidence.memory_rebuild_dirty_gist_count(), 1);
        assert_eq!(evidence.clean_gist_repair_issue_count(), 4);
        assert_eq!(
            evidence.clean_gist_repair_detail_codes(),
            vec![
                "dirty_clean_gist:dirty".to_owned(),
                "dirty_gist:legacy".to_owned(),
                "missing_clean_gist:missing".to_owned(),
            ]
        );
        assert!(
            !evidence
                .clean_gist_repair_detail_codes()
                .contains(&"missing_clean_gist:stale".to_owned())
        );
        assert!(
            evidence
                .detail_codes()
                .contains(&"missing_clean_gist:missing".to_owned())
        );
        let dispatch_pressure = evidence.hygiene_dispatch_pressure_summary();
        assert_eq!(dispatch_pressure.missing_clean_gist_pressure, 2);
        assert!(
            dispatch_pressure
                .reason_codes()
                .contains(&"missing_clean_gist".to_owned())
        );
        assert!(
            evidence
                .status_codes()
                .contains(&"clean_gist_repair_required".to_owned())
        );
    }

    #[test]
    fn startup_evidence_aggregates_governance_rebuild_and_repair_counts() {
        let prompt_secret = "GOVERNANCE_PROMPT_SECRET_DO_NOT_LOG";
        let evidence = MemoryServiceStartupEvidence {
            requires_operator_review: true,
            approved_phases: vec![MemoryMigrationPhase::ReadOnlyShadow],
            lines: vec![
                "memory_governance records=3 duplicate_groups=1 duplicate_records=1 noisy=1 context_rot=1 reason_codes=missing_clean_gist|transcript_shape detail_codes=context_rot:rot:missing_clean_gist|noise:rot:transcript_shape".to_owned(),
                "memory_governance records=2 duplicate_groups=0 duplicate_records=0 noisy=1 context_rot=1 reason_codes=dirty_clean_gist|transcript_shape detail_codes=context_rot:dirty:transcript_anchor_risk|noise:dirty:dirty_clean_gist".to_owned(),
                "memory_rebuild required=true duplicate_groups=1 refresh=1 compact=1 quarantine=1 missing_clean_gist=1 dirty_clean_gist=0 dirty_gist=1 reasons=4 reason_codes=compact_long_context_without_gist|deduplicate_exact_fingerprints|quarantine_high_noise_records|repair_missing_or_dirty_clean_gist detail_codes=compact:long|deduplicate:clean:dupe|dirty_gist:long|missing_clean_gist:long|quarantine:long|refresh:long".to_owned(),
                "memory_rebuild required=false duplicate_groups=0 refresh=1 compact=0 quarantine=0 missing_clean_gist=0 dirty_clean_gist=1 dirty_gist=0 reasons=2 reason_codes=refresh_noisy_or_rotting_index|repair_missing_or_dirty_clean_gist detail_codes=dirty_clean_gist:dirty|refresh:dirty".to_owned(),
                "experience_index_quality_gate ready_for_context_injection=false records=3 blockers=3 warnings=2 duplicates=1 refresh=1 compact=1 quarantine=1 missing_clean_gist=1 dirty_clean_gist=0 dirty_gist=1 context_rot_blockers=2 reason_codes=compact_context_rot|dirty_gist|duplicate_experience|missing_clean_gist|quarantine_context_rot|refresh_noisy_or_rotting_index context_rot_blocker_reason_codes=duplicate_experience|long_without_clean_gist detail_codes=blocker:compact:long|blocker:duplicate:clean:dupe|blocker:quarantine:long|warning:dirty_gist:long|warning:missing_clean_gist:long|warning:refresh:long".to_owned(),
                "experience_index_quality_gate ready_for_context_injection=false records=2 blockers=0 warnings=2 duplicates=0 refresh=1 compact=0 quarantine=0 missing_clean_gist=0 dirty_clean_gist=1 dirty_gist=0 context_rot_blockers=0 reason_codes=dirty_clean_gist|refresh_noisy_or_rotting_index context_rot_blocker_reason_codes=none detail_codes=warning:dirty_clean_gist:dirty|warning:refresh:dirty".to_owned(),
                "memory_repair_plan empty=false items=3 skipped=1 repair_clean_gist=1 compact_context=0 quarantine=1 delete_duplicate=1 skipped_repair_clean_gist=1 skipped_compact_context=0 skipped_quarantine=0 skipped_delete_duplicate=0 reason_codes=deduplicate_exact_fingerprint|governance_quarantine_candidate|repair_legacy_metadata_lesson skipped_reason_codes=missing_clean_gist detail_codes=delete_duplicate:deduplicate_exact_fingerprint:647570|quarantine:governance_quarantine_candidate:6e6f697379|repair_clean_gist:repair_legacy_metadata_lesson:726570616972|skipped:repair_clean_gist:missing_clean_gist:6c6567616379".to_owned(),
                "memory_repair_plan empty=false items=1 skipped=2 repair_clean_gist=0 compact_context=1 quarantine=0 delete_duplicate=0 skipped_repair_clean_gist=0 skipped_compact_context=1 skipped_quarantine=1 skipped_delete_duplicate=0 reason_codes=compact_long_context_without_gist skipped_reason_codes=dirty_clean_gist|policy_disabled detail_codes=compact_context:compact_long_context_without_gist:6c6f6e67|skipped:compact_context:policy_disabled:6c6f6e67|skipped:quarantine:dirty_clean_gist:6469727479".to_owned(),
            ],
        };

        assert_eq!(evidence.governance_record_count(), 5);
        assert_eq!(evidence.governance_duplicate_group_count(), 1);
        assert_eq!(evidence.governance_duplicate_record_count(), 1);
        assert_eq!(evidence.governance_noisy_count(), 2);
        assert_eq!(evidence.governance_context_rot_count(), 2);
        assert_eq!(
            evidence.governance_reason_codes(),
            vec![
                "dirty_clean_gist".to_owned(),
                "missing_clean_gist".to_owned(),
                "transcript_shape".to_owned(),
            ]
        );
        assert_eq!(evidence.governance_reason_count("transcript_shape"), 2);
        assert_eq!(evidence.governance_reason_count("missing_clean_gist"), 1);
        assert_eq!(evidence.governance_reason_count("dirty_clean_gist"), 1);
        assert_eq!(
            evidence.governance_detail_codes(),
            vec![
                "context_rot:dirty:transcript_anchor_risk".to_owned(),
                "context_rot:rot:missing_clean_gist".to_owned(),
                "noise:dirty:dirty_clean_gist".to_owned(),
                "noise:rot:transcript_shape".to_owned(),
            ]
        );

        assert!(evidence.memory_rebuild_required());
        assert_eq!(evidence.memory_rebuild_duplicate_group_count(), 1);
        assert_eq!(evidence.memory_rebuild_refresh_count(), 2);
        assert_eq!(evidence.memory_rebuild_compact_count(), 1);
        assert_eq!(evidence.memory_rebuild_quarantine_count(), 1);
        assert_eq!(evidence.memory_rebuild_missing_clean_gist_count(), 1);
        assert_eq!(evidence.memory_rebuild_dirty_clean_gist_count(), 1);
        assert_eq!(evidence.memory_rebuild_dirty_gist_count(), 1);
        assert!(!evidence.experience_index_quality_gate_ready());
        assert_eq!(evidence.experience_index_quality_gate_blocker_count(), 3);
        assert_eq!(evidence.experience_index_quality_gate_warning_count(), 4);
        assert_eq!(
            evidence.experience_index_quality_gate_context_rot_blocker_count(),
            2
        );
        assert_eq!(
            evidence.experience_index_quality_gate_context_rot_blocker_reason_codes(),
            vec![
                "duplicate_experience".to_owned(),
                "long_without_clean_gist".to_owned(),
            ]
        );
        assert_eq!(
            evidence.experience_index_quality_gate_missing_clean_gist_count(),
            1
        );
        assert_eq!(evidence.experience_index_quality_gate_dirty_gist_count(), 1);
        assert_eq!(
            evidence.memory_rebuild_reason_codes(),
            vec![
                "compact_long_context_without_gist".to_owned(),
                "deduplicate_exact_fingerprints".to_owned(),
                "quarantine_high_noise_records".to_owned(),
                "refresh_noisy_or_rotting_index".to_owned(),
                "repair_missing_or_dirty_clean_gist".to_owned(),
            ]
        );
        assert_eq!(
            evidence.memory_rebuild_reason_count("repair_missing_or_dirty_clean_gist"),
            2
        );
        assert_eq!(
            evidence.memory_rebuild_reason_count("deduplicate_exact_fingerprints"),
            1
        );
        assert!(
            evidence
                .memory_rebuild_detail_codes()
                .contains(&"dirty_clean_gist:dirty".to_owned())
        );

        assert_eq!(evidence.memory_repair_item_count(), 4);
        assert_eq!(evidence.memory_repair_skipped_count(), 3);
        assert_eq!(evidence.memory_repair_clean_gist_count(), 1);
        assert_eq!(evidence.memory_repair_compact_context_count(), 1);
        assert_eq!(evidence.memory_repair_quarantine_count(), 1);
        assert_eq!(evidence.memory_repair_delete_duplicate_count(), 1);
        assert_eq!(evidence.memory_repair_skipped_clean_gist_count(), 1);
        assert_eq!(evidence.memory_repair_skipped_compact_context_count(), 1);
        assert_eq!(evidence.memory_repair_skipped_quarantine_count(), 1);
        assert_eq!(evidence.memory_repair_skipped_delete_duplicate_count(), 0);
        assert_eq!(
            evidence.memory_repair_reason_codes(),
            vec![
                "compact_long_context_without_gist".to_owned(),
                "deduplicate_exact_fingerprint".to_owned(),
                "governance_quarantine_candidate".to_owned(),
                "repair_legacy_metadata_lesson".to_owned(),
            ]
        );
        assert_eq!(
            evidence.memory_repair_reason_count("repair_legacy_metadata_lesson"),
            1
        );
        assert_eq!(
            evidence.memory_repair_reason_count("compact_long_context_without_gist"),
            1
        );
        assert_eq!(
            evidence.memory_repair_skipped_reason_codes(),
            vec![
                "dirty_clean_gist".to_owned(),
                "missing_clean_gist".to_owned(),
                "policy_disabled".to_owned(),
            ]
        );
        assert_eq!(
            evidence.memory_repair_skipped_reason_count("missing_clean_gist"),
            1
        );
        assert_eq!(
            evidence.memory_repair_skipped_reason_count("policy_disabled"),
            1
        );
        assert!(
            evidence
                .memory_repair_detail_codes()
                .contains(&"skipped:quarantine:dirty_clean_gist:6469727479".to_owned())
        );
        assert_eq!(
            evidence.memory_repair_detail_codes_for_action("repair_clean_gist"),
            vec!["repair_clean_gist:repair_legacy_metadata_lesson:726570616972".to_owned()]
        );
        assert_eq!(
            evidence.memory_repair_detail_codes_for_action("compact_context"),
            vec!["compact_context:compact_long_context_without_gist:6c6f6e67".to_owned()]
        );
        assert_eq!(
            evidence.memory_repair_detail_codes_for_reason("missing_clean_gist"),
            vec!["skipped:repair_clean_gist:missing_clean_gist:6c6567616379".to_owned()]
        );
        assert_eq!(
            evidence.experience_index_quality_gate_detail_codes_for_reason("missing_clean_gist"),
            vec!["warning:missing_clean_gist:long".to_owned()]
        );
        assert_eq!(
            evidence.memory_repair_skipped_detail_codes(),
            vec![
                "skipped:compact_context:policy_disabled:6c6f6e67".to_owned(),
                "skipped:quarantine:dirty_clean_gist:6469727479".to_owned(),
                "skipped:repair_clean_gist:missing_clean_gist:6c6567616379".to_owned(),
            ]
        );
        assert_eq!(
            evidence.memory_repair_skipped_detail_codes_for_action("quarantine"),
            vec!["skipped:quarantine:dirty_clean_gist:6469727479".to_owned()]
        );
        assert_eq!(
            evidence.memory_repair_skipped_detail_codes_for_reason("policy_disabled"),
            vec!["skipped:compact_context:policy_disabled:6c6f6e67".to_owned()]
        );
        assert!(!evidence.summary_text().contains(prompt_secret));
        assert!(!evidence.summary_line().contains(prompt_secret));
    }

    #[test]
    fn startup_evidence_lifts_projection_parity_detail_codes() {
        let mut audit = MemoryProjectionAudit::default();
        audit
            .mismatches
            .push(MemoryProjectionMismatch::new("replay_runs", 2, 1));
        audit
            .warnings
            .push("state_inspection_projection_missing_runtime_kv_count".to_owned());
        let evidence = MemoryServiceStartupEvidence {
            requires_operator_review: true,
            approved_phases: vec![MemoryMigrationPhase::ReadOnlyShadow],
            lines: vec![audit.summary_line()],
        };
        let detail_codes = evidence.detail_codes();

        assert!(detail_codes.contains(&"mismatch:replay_runs".to_owned()));
        assert!(
            detail_codes.contains(
                &"warning:state_inspection_projection_missing_runtime_kv_count".to_owned()
            )
        );
        assert!(evidence.summary_line().contains("mismatch:replay_runs"));
    }

    #[test]
    fn service_dry_run_accepts_projection_contract_bundle_for_startup_evidence() {
        let experiences = vec![
            ExperienceEnvelope::new("clean", "prompt", "Stable clean lesson")
                .with_scope(MemoryScope::for_task("runtime"))
                .with_clean_gist("A stable memory summary with enough useful detail.")
                .with_quality(0.9)
                .with_tags(vec!["adapter:test".to_owned()]),
        ];
        let memory_entries = vec![
            RetentionMemoryEntry::new("mem", "semantic durable memory", vec![0.1], 2.4)
                .with_feedback(4, 0)
                .with_access(1, 10),
        ];
        let adapters = vec![status(
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
            true,
            AdapterWriteMode::ReadOnly,
        )];
        let seed_ledger = MemoryEvolutionLedger {
            replay_runs: 1,
            replay_items: 1,
            replay_memory_updates: 1,
            ..MemoryEvolutionLedger::default()
        };
        let bundle = AdapterProjectionContractBundle::standard_shadow();
        let adapter_snapshots = vec![
            AdapterSnapshotSummary::read_only("experience_shadow", 1, 0),
            AdapterSnapshotSummary::read_only("disk_kv_shadow", 0, 1),
        ];

        let dry_run = MemoryServiceDryRun::for_inputs(
            MemoryServiceShadowPlanInputs::new("bundle_shadow", &experiences, &[], &memory_entries)
                .with_adapters(&adapters)
                .with_adapter_snapshots(&adapter_snapshots)
                .with_scope(&MemoryScope::for_task("runtime"))
                .with_evolution_ledger(seed_ledger)
                .with_kvswap_state(KvSwapStateSnapshot {
                    hot_shard_count: 1,
                    cold_shard_count: 1,
                    metadata_count: 2,
                    hot_byte_len: 4,
                    cold_byte_len: 8,
                })
                .with_projection_contract_bundle(&bundle),
            &MemoryMigrationEvidence::read_only_source(Some(1)),
            &[MemoryMigrationPhase::ReadOnlyShadow],
            false,
        );
        let evidence = dry_run.startup_evidence();
        let text = evidence.summary_text();

        assert!(dry_run.summary.ready);
        assert!(!dry_run.requires_operator_review());
        assert_eq!(dry_run.summary.adapter_snapshot_count, 2);
        assert_eq!(dry_run.summary.adapter_snapshot_warning_count, 0);
        assert!(dry_run.adapter_checklist().items.iter().any(|item| {
            item.code == "adapter_snapshots_clean"
                && item.satisfied
                && item.detail == dry_run.summary.adapter_snapshot_checklist_detail()
                && item.detail_codes().is_empty()
        }));
        assert_eq!(dry_run.summary.projection_contract_count, 7);
        assert_eq!(dry_run.summary.projection_contract_manifest_count, 7);
        assert_eq!(dry_run.summary.projection_contract_blocker_count, 0);
        assert_eq!(dry_run.summary.projection_contract_warning_count, 0);
        assert!(dry_run.adapter_checklist().items.iter().any(|item| {
            item.code == "projection_contracts_ready"
                && item.satisfied
                && item.detail
                    == dry_run
                        .summary
                        .projection_contracts_ready_checklist_detail()
        }));
        assert!(dry_run.adapter_checklist().items.iter().any(|item| {
            item.code == "projection_contract_warnings_clean"
                && item.satisfied
                && item.detail
                    == dry_run
                        .summary
                        .projection_contract_warnings_checklist_detail()
        }));
        assert_eq!(dry_run.plan.projection_coverage, bundle.coverage_reports());
        assert_eq!(
            dry_run.plan.projection_bundle_summary,
            Some(bundle.coverage_summary())
        );
        assert_eq!(
            dry_run.plan.projection_contract_bundle_manifest,
            Some(bundle.manifest_summary_line())
        );
        assert_eq!(
            dry_run.migration_evidence,
            MemoryMigrationEvidence::read_only_source(Some(1))
        );
        assert_eq!(
            dry_run.plan.kvswap_state,
            Some(KvSwapStateSnapshot {
                hot_shard_count: 1,
                cold_shard_count: 1,
                metadata_count: 2,
                hot_byte_len: 4,
                cold_byte_len: 8,
            })
        );
        assert!(dry_run.summary.kvswap_state_present);
        assert_eq!(dry_run.summary.kvswap_hot_shard_count, 1);
        assert_eq!(dry_run.summary.kvswap_cold_shard_count, 1);
        assert_eq!(dry_run.summary.kvswap_metadata_count, 2);
        assert_eq!(dry_run.summary.kvswap_total_byte_len, 12);
        assert_eq!(
            dry_run.summary.kvswap_shape_codes,
            vec![
                "cold_catalog".to_owned(),
                "hot_metadata".to_owned(),
                "metadata_index".to_owned(),
                "mixed_tiers".to_owned(),
            ]
        );
        assert!(evidence.kvswap_state_present());
        assert_eq!(evidence.kvswap_state_hot_shard_count(), 1);
        assert_eq!(evidence.kvswap_state_cold_shard_count(), 1);
        assert_eq!(evidence.kvswap_state_metadata_count(), 2);
        assert_eq!(evidence.kvswap_state_total_byte_len(), 12);
        assert_eq!(
            evidence.kvswap_state_shape_codes(),
            vec![
                "cold_catalog".to_owned(),
                "hot_metadata".to_owned(),
                "metadata_index".to_owned(),
                "mixed_tiers".to_owned(),
            ]
        );
        assert!(text.contains("projection_contracts=7 projection_contract_manifests=7"));
        assert!(text.contains("adapter_snapshots=2 adapter_snapshot_warnings=0"));
        assert!(text.contains(
            "adapter_snapshot adapter=experience_shadow write_mode=read_only experiences=1 kv_shards=0 total_records=1 warnings=0 status_codes=read_only warning_codes=none"
        ));
        assert!(text.contains(
            "adapter_snapshot adapter=disk_kv_shadow write_mode=read_only experiences=0 kv_shards=1 total_records=1 warnings=0 status_codes=read_only warning_codes=none"
        ));
        assert!(text.contains("kvswap_state=true kvswap_hot=1 kvswap_cold=1 kvswap_metadata=2 kvswap_bytes=12 kvswap_shape_codes=cold_catalog|hot_metadata|metadata_index|mixed_tiers"));
        assert!(text.contains(
            "adapter_projection_bundle name=standard_shadow target=shadow_read ready=true"
        ));
        assert!(text.contains(
            "adapter_projection_contract_bundle_manifest name=standard_shadow target=shadow_read contracts=7 adapters=disk_kv_store|experience_store|gist_memory|infini_memory|kv_cache|service_memory|tiered_cache mapped_fields=32 required_fields=22 recommended_fields=10 notes=0"
        ));
        assert!(text.contains(
            "adapter_projection_contract adapter=experience_store kind=experience_store target=shadow_read"
        ));
        assert!(text.contains(
            "adapter_projection_contract adapter=disk_kv_store kind=disk_kv_store target=shadow_read"
        ));
        assert!(text.contains(
            "kvswap_state empty=false hot=1 cold=1 metadata=2 hot_bytes=4 cold_bytes=8 total_bytes=12 shape_codes=cold_catalog|hot_metadata|metadata_index|mixed_tiers"
        ));
        assert!(text.contains("memory_migration_evidence source_read_only=true copied_fixture=false isolated_write_root=false catalog_verified=false checksum_verified=false live_store_targeted=false records=1"));
        assert!(text.contains("adapter_projection adapter=experience_store"));
        assert!(text.contains("adapter_projection adapter=disk_kv_store"));
        assert!(text.contains("adapter_projection adapter=gist_memory"));
        assert!(text.contains("adapter_projection adapter=infini_memory"));
        assert!(text.contains("adapter_projection adapter=kv_cache"));
        assert!(text.contains("adapter_projection adapter=tiered_cache"));
        assert!(text.contains("adapter_projection adapter=service_memory"));
        assert!(text.contains("memory_migration phase=read_only_shadow approved=true"));
    }

    #[test]
    fn startup_evidence_reads_kvswap_state_from_memory_shadow_fallback() {
        let evidence = MemoryServiceStartupEvidence {
            requires_operator_review: false,
            approved_phases: vec![MemoryMigrationPhase::ReadOnlyShadow],
            lines: vec![
                "memory_shadow ready=true review=false kvswap_state=true kvswap_hot=2 kvswap_cold=0 kvswap_metadata=2 kvswap_bytes=7 kvswap_shape_codes=hot_metadata|hot_only|metadata_index detail_codes=none".to_owned(),
            ],
        };

        assert!(evidence.kvswap_state_present());
        assert_eq!(evidence.kvswap_state_hot_shard_count(), 2);
        assert_eq!(evidence.kvswap_state_cold_shard_count(), 0);
        assert_eq!(evidence.kvswap_state_metadata_count(), 2);
        assert_eq!(evidence.kvswap_state_total_byte_len(), 7);
        assert_eq!(
            evidence.kvswap_state_shape_codes(),
            vec![
                "hot_metadata".to_owned(),
                "hot_only".to_owned(),
                "metadata_index".to_owned(),
            ]
        );
    }

    #[test]
    fn startup_evidence_prefers_direct_kvswap_state_and_boundary_over_shadow_summary() {
        let evidence = MemoryServiceStartupEvidence {
            requires_operator_review: true,
            approved_phases: vec![MemoryMigrationPhase::ReadOnlyShadow],
            lines: vec![
                "memory_shadow ready=false review=true kvswap_state=true kvswap_hot=9 kvswap_cold=9 kvswap_metadata=9 kvswap_bytes=999 kvswap_shape_codes=shadow_only kvswap_boundary=true kvswap_boundary_issues=9 kvswap_boundary_reason_codes=stale_metadata kvswap_boundary_detail_codes=stale_metadata:736861646f77 kvswap_boundary_overlap=9 kvswap_boundary_missing_hot_metadata=9 kvswap_boundary_stale_metadata=9 kvswap_boundary_hot_tier_mismatch=9 kvswap_boundary_cold_tier_mismatch=9 detail_codes=none".to_owned(),
                "kvswap_state hot=1 cold=2 metadata=3 total_bytes=42 shape_codes=cold_catalog|hot_metadata|metadata_index".to_owned(),
                "kvswap_boundary clean=false issues=1 overlap=1 missing_hot_metadata=0 stale_metadata=0 hot_tier_mismatch=0 cold_tier_mismatch=0 reason_codes=overlapping_hot_cold detail_codes=overlap:73686172642d61".to_owned(),
            ],
        };

        assert!(evidence.kvswap_state_present());
        assert_eq!(evidence.kvswap_state_hot_shard_count(), 1);
        assert_eq!(evidence.kvswap_state_cold_shard_count(), 2);
        assert_eq!(evidence.kvswap_state_metadata_count(), 3);
        assert_eq!(evidence.kvswap_state_total_byte_len(), 42);
        assert_eq!(
            evidence.kvswap_state_shape_codes(),
            vec![
                "cold_catalog".to_owned(),
                "hot_metadata".to_owned(),
                "metadata_index".to_owned()
            ]
        );
        assert_eq!(evidence.kvswap_boundary_issue_count(), 1);
        assert_eq!(
            evidence.kvswap_boundary_reason_codes(),
            vec!["overlapping_hot_cold".to_owned()]
        );
        assert_eq!(
            evidence.kvswap_boundary_detail_codes(),
            vec!["overlap:73686172642d61".to_owned()]
        );
        assert_eq!(evidence.kvswap_boundary_overlap_count(), 1);
        assert_eq!(evidence.kvswap_boundary_missing_hot_metadata_count(), 0);
        assert_eq!(evidence.kvswap_boundary_stale_metadata_count(), 0);
        assert_eq!(evidence.kvswap_boundary_hot_tier_mismatch_count(), 0);
        assert_eq!(evidence.kvswap_boundary_cold_tier_mismatch_count(), 0);
        assert_eq!(evidence.kvswap_boundary_readiness_report_count(), 0);
        assert_eq!(evidence.kvswap_boundary_blocker_count(), 1);
        assert_eq!(evidence.kvswap_boundary_warning_count(), 0);
        assert_eq!(
            evidence.kvswap_boundary_blocker_reason_codes(),
            vec!["overlapping_hot_cold".to_owned()]
        );
        assert_eq!(
            evidence.kvswap_boundary_warning_reason_codes(),
            Vec::<String>::new()
        );
        assert_eq!(
            evidence.kvswap_boundary_readiness_detail_codes(),
            vec!["blocker:overlap:73686172642d61".to_owned()]
        );
    }

    #[test]
    fn startup_evidence_reads_kvswap_boundary_detail_and_counts_from_shadow_fallback() {
        let evidence = MemoryServiceStartupEvidence {
            requires_operator_review: true,
            approved_phases: vec![MemoryMigrationPhase::ReadOnlyShadow],
            lines: vec![
                "memory_shadow ready=false review=true kvswap_boundary=true kvswap_boundary_issues=6 kvswap_boundary_reason_codes=cold_tier_mismatch|hot_tier_mismatch|missing_hot_metadata|overlapping_hot_cold|stale_metadata kvswap_boundary_detail_codes=cold_tier_mismatch:736861646f772d636f6c64|hot_tier_mismatch:736861646f772d686f74|missing_hot_metadata:736861646f772d6d657461|overlap:736861646f772d686f74|stale_metadata:736861646f772d7374616c65 kvswap_boundary_overlap=1 kvswap_boundary_missing_hot_metadata=1 kvswap_boundary_stale_metadata=2 kvswap_boundary_hot_tier_mismatch=1 kvswap_boundary_cold_tier_mismatch=1 detail_codes=none".to_owned(),
            ],
        };

        assert_eq!(evidence.kvswap_boundary_issue_count(), 6);
        assert_eq!(
            evidence.kvswap_boundary_reason_codes(),
            vec![
                "cold_tier_mismatch".to_owned(),
                "hot_tier_mismatch".to_owned(),
                "missing_hot_metadata".to_owned(),
                "overlapping_hot_cold".to_owned(),
                "stale_metadata".to_owned()
            ]
        );
        assert_eq!(
            evidence.kvswap_boundary_detail_codes(),
            vec![
                "cold_tier_mismatch:736861646f772d636f6c64".to_owned(),
                "hot_tier_mismatch:736861646f772d686f74".to_owned(),
                "missing_hot_metadata:736861646f772d6d657461".to_owned(),
                "overlap:736861646f772d686f74".to_owned(),
                "stale_metadata:736861646f772d7374616c65".to_owned(),
            ]
        );
        assert_eq!(evidence.kvswap_boundary_overlap_count(), 1);
        assert_eq!(evidence.kvswap_boundary_missing_hot_metadata_count(), 1);
        assert_eq!(evidence.kvswap_boundary_stale_metadata_count(), 2);
        assert_eq!(evidence.kvswap_boundary_hot_tier_mismatch_count(), 1);
        assert_eq!(evidence.kvswap_boundary_cold_tier_mismatch_count(), 1);
        assert_eq!(evidence.kvswap_boundary_readiness_report_count(), 0);
        assert_eq!(evidence.kvswap_boundary_blocker_count(), 2);
        assert_eq!(evidence.kvswap_boundary_warning_count(), 4);
        assert_eq!(
            evidence.kvswap_boundary_blocker_reason_codes(),
            vec![
                "missing_hot_metadata".to_owned(),
                "overlapping_hot_cold".to_owned()
            ]
        );
        assert_eq!(
            evidence.kvswap_boundary_warning_reason_codes(),
            vec![
                "cold_tier_mismatch".to_owned(),
                "hot_tier_mismatch".to_owned(),
                "stale_metadata".to_owned()
            ]
        );
        assert_eq!(
            evidence.kvswap_boundary_readiness_detail_codes(),
            vec![
                "blocker:missing_hot_metadata:736861646f772d6d657461".to_owned(),
                "blocker:overlap:736861646f772d686f74".to_owned(),
                "warning:cold_tier_mismatch:736861646f772d636f6c64".to_owned(),
                "warning:hot_tier_mismatch:736861646f772d686f74".to_owned(),
                "warning:stale_metadata:736861646f772d7374616c65".to_owned(),
            ]
        );
        assert_eq!(
            evidence.kvswap_boundary_detail_codes_for("overlap"),
            vec!["overlap:736861646f772d686f74".to_owned()]
        );
    }

    #[test]
    fn startup_evidence_counts_kvswap_boundary_readiness_reasons() {
        let evidence = MemoryServiceStartupEvidence {
            requires_operator_review: true,
            approved_phases: vec![MemoryMigrationPhase::ReadOnlyShadow],
            lines: vec![
                "kvswap_boundary_readiness ready=false blockers=2 warnings=1 blocker_reason_codes=overlapping_hot_cold|missing_hot_metadata warning_reason_codes=stale_metadata detail_codes=blocker:overlap:686f74|blocker:missing_hot_metadata:636f6c64|warning:stale_metadata:6d657461".to_owned(),
                "kvswap_boundary_readiness ready=false blockers=1 warnings=1 blocker_reason_codes=overlapping_hot_cold warning_reason_codes=stale_metadata detail_codes=blocker:overlap:686f7432|warning:stale_metadata:6d65746132".to_owned(),
            ],
        };

        assert_eq!(evidence.kvswap_boundary_readiness_report_count(), 2);
        assert_eq!(evidence.kvswap_boundary_blocker_count(), 3);
        assert_eq!(evidence.kvswap_boundary_warning_count(), 2);
        assert_eq!(
            evidence.kvswap_boundary_blocker_reason_codes(),
            vec![
                "missing_hot_metadata".to_owned(),
                "overlapping_hot_cold".to_owned(),
            ]
        );
        assert_eq!(
            evidence.kvswap_boundary_warning_reason_codes(),
            vec!["stale_metadata".to_owned()]
        );
        assert_eq!(
            evidence.kvswap_boundary_blocker_reason_count("overlapping_hot_cold"),
            2
        );
        assert_eq!(
            evidence.kvswap_boundary_blocker_reason_count("missing_hot_metadata"),
            1
        );
        assert_eq!(
            evidence.kvswap_boundary_warning_reason_count("stale_metadata"),
            2
        );
        assert_eq!(
            evidence.kvswap_boundary_warning_reason_count("missing_hot_metadata"),
            0
        );
        assert_eq!(
            evidence.hygiene_dispatch_pressure_summary(),
            MemoryHygieneDispatchPressureSummary {
                pressure_score: 0,
                queue_items: 0,
                operator_review_items: 0,
                isolation_items: 0,
                kvswap_boundary_repair_lanes: 0,
                context_rot_review_lanes: 0,
                experience_index_rebuild_lanes: 0,
                quarantine_priorities: 0,
                repair_priorities: 0,
                context_rot_risks: 0,
                missing_clean_gist_pressure: 0,
                kvswap_boundary_blockers: 3,
                kvswap_boundary_warnings: 2,
            }
        );
        let dispatch_pressure = evidence.hygiene_dispatch_pressure_summary();
        assert!(dispatch_pressure.has_pressure());
        assert!(dispatch_pressure.requires_operator_review());
        assert!(dispatch_pressure.requires_isolation());
        assert_eq!(dispatch_pressure.priority_code(), "quarantine");
        assert_eq!(dispatch_pressure.dispatch_rank(), 3);
        assert_eq!(
            dispatch_pressure.reason_codes(),
            vec![
                "kvswap_boundary_blocker".to_owned(),
                "kvswap_boundary_warning".to_owned(),
            ]
        );
    }

    #[test]
    fn startup_evidence_reads_hygiene_work_queue_from_memory_shadow_fallback() {
        let evidence = MemoryServiceStartupEvidence {
            requires_operator_review: true,
            approved_phases: vec![MemoryMigrationPhase::ReadOnlyShadow],
            lines: vec![
                "memory_shadow ready=false review=true hygiene_work_queue_items=2 hygiene_work_queue_operator_review=1 hygiene_work_queue_isolation=1 hygiene_work_queue_next_dispatch=dispatch:operator_review:isolated:kvswap_boundary_repair:quarantine:100:1 hygiene_work_queue_lanes=kvswap_boundary_repair|context_rot_review hygiene_work_queue_priorities=quarantine|repair hygiene_work_queue_dispatch_codes=dispatch:operator_review:isolated:kvswap_boundary_repair:quarantine:100:1|dispatch:auto:shared:context_rot_review:repair:5:1 hygiene_work_queue_detail_codes=kvswap_boundary_repair:quarantine:100:1|context_rot_review:repair:5:1 hygiene_work_queue_reason_codes=items_present|operator_review_required|isolation_recommended detail_codes=none".to_owned(),
            ],
        };

        assert_eq!(evidence.memory_hygiene_work_queue_count(), 0);
        assert_eq!(evidence.memory_hygiene_work_queue_item_count(), 2);
        assert_eq!(
            evidence.memory_hygiene_work_queue_operator_review_count(),
            1
        );
        assert_eq!(evidence.memory_hygiene_work_queue_isolation_count(), 1);
        assert_eq!(
            evidence.memory_hygiene_work_queue_next_dispatch_codes(),
            vec![
                "dispatch:operator_review:isolated:kvswap_boundary_repair:quarantine:100:1"
                    .to_owned()
            ]
        );
        assert_eq!(
            evidence.memory_hygiene_work_queue_lane_codes(),
            vec![
                "context_rot_review".to_owned(),
                "kvswap_boundary_repair".to_owned()
            ]
        );
        assert_eq!(
            evidence.memory_hygiene_work_queue_lane_count("kvswap_boundary_repair"),
            1
        );
        assert_eq!(
            evidence.memory_hygiene_work_queue_lane_count("context_rot_review"),
            1
        );
        assert_eq!(
            evidence.memory_hygiene_work_queue_priority_codes(),
            vec!["quarantine".to_owned(), "repair".to_owned()]
        );
        assert_eq!(
            evidence.memory_hygiene_work_queue_priority_count("quarantine"),
            1
        );
        assert_eq!(
            evidence.memory_hygiene_work_queue_dispatch_codes(),
            vec![
                "dispatch:auto:shared:context_rot_review:repair:5:1".to_owned(),
                "dispatch:operator_review:isolated:kvswap_boundary_repair:quarantine:100:1"
                    .to_owned()
            ]
        );
        assert_eq!(
            evidence.memory_hygiene_work_queue_detail_codes(),
            vec![
                "context_rot_review:repair:5:1".to_owned(),
                "kvswap_boundary_repair:quarantine:100:1".to_owned()
            ]
        );
        assert_eq!(
            evidence.memory_hygiene_work_queue_reason_codes(),
            vec![
                "isolation_recommended".to_owned(),
                "items_present".to_owned(),
                "operator_review_required".to_owned()
            ]
        );
        assert_eq!(
            evidence.memory_hygiene_work_queue_reason_count("operator_review_required"),
            1
        );
    }

    #[test]
    fn startup_evidence_counts_hygiene_dispatch_pressure_reasons() {
        let evidence = MemoryServiceStartupEvidence {
            requires_operator_review: true,
            approved_phases: vec![MemoryMigrationPhase::ReadOnlyShadow],
            lines: vec![
                "memory_hygiene_dispatch_pressure rank=3 priority=quarantine pressure_score=235 queue_items=3 operator_review_items=3 isolation_items=3 context_rot_risks=2 missing_clean_gist_pressure=1 kvswap_boundary_blockers=1 kvswap_boundary_warnings=1 reason_codes=queue_items|operator_review_items|isolation_items|context_rot_review|missing_clean_gist|kvswap_boundary_blocker|kvswap_boundary_warning".to_owned(),
            ],
        };

        assert_eq!(evidence.memory_hygiene_dispatch_pressure_rank(), 3);
        assert_eq!(
            evidence.memory_hygiene_dispatch_pressure_priority_codes(),
            vec!["quarantine".to_owned()]
        );
        assert_eq!(
            evidence.memory_hygiene_dispatch_pressure_reason_count("missing_clean_gist"),
            1
        );
        assert_eq!(
            evidence.memory_hygiene_dispatch_pressure_reason_count("kvswap_boundary_blocker"),
            1
        );
        assert_eq!(
            evidence.memory_hygiene_dispatch_pressure_reason_count("stale_context_rot"),
            0
        );
    }

    #[test]
    fn startup_evidence_prefers_direct_hygiene_work_queue_over_shadow_summary() {
        let evidence = MemoryServiceStartupEvidence {
            requires_operator_review: true,
            approved_phases: vec![MemoryMigrationPhase::ReadOnlyShadow],
            lines: vec![
                "memory_shadow ready=false review=true hygiene_work_queue_items=9 hygiene_work_queue_operator_review=9 hygiene_work_queue_isolation=9 hygiene_work_queue_next_dispatch=dispatch:shadow:shared:context_rot_review:repair:5:9 hygiene_work_queue_lanes=context_rot_review hygiene_work_queue_priorities=repair hygiene_work_queue_dispatch_codes=dispatch:shadow:shared:context_rot_review:repair:5:9 hygiene_work_queue_detail_codes=context_rot_review:repair:5:9 hygiene_work_queue_reason_codes=items_present detail_codes=none".to_owned(),
                "memory_hygiene_work_queue clean=false total_score=100 items=1 operator_review=1 isolation=1 next_dispatch=dispatch:operator_review:isolated:kvswap_boundary_repair:quarantine:100:1 lanes=kvswap_boundary_repair priorities=quarantine dispatch_codes=dispatch:operator_review:isolated:kvswap_boundary_repair:quarantine:100:1 detail_codes=kvswap_boundary_repair:quarantine:100:1 reason_codes=isolation_recommended|items_present|operator_review_required".to_owned(),
            ],
        };

        assert_eq!(evidence.memory_hygiene_work_queue_count(), 1);
        assert_eq!(evidence.memory_hygiene_work_queue_item_count(), 1);
        assert_eq!(
            evidence.memory_hygiene_work_queue_operator_review_count(),
            1
        );
        assert_eq!(evidence.memory_hygiene_work_queue_isolation_count(), 1);
        assert_eq!(
            evidence.memory_hygiene_work_queue_next_dispatch_codes(),
            vec![
                "dispatch:operator_review:isolated:kvswap_boundary_repair:quarantine:100:1"
                    .to_owned()
            ]
        );
        assert_eq!(
            evidence.memory_hygiene_work_queue_lane_codes(),
            vec!["kvswap_boundary_repair".to_owned()]
        );
        assert_eq!(
            evidence.memory_hygiene_work_queue_lane_count("kvswap_boundary_repair"),
            1
        );
        assert_eq!(
            evidence.memory_hygiene_work_queue_lane_count("context_rot_review"),
            0
        );
        assert_eq!(
            evidence.memory_hygiene_work_queue_priority_codes(),
            vec!["quarantine".to_owned()]
        );
        assert_eq!(
            evidence.memory_hygiene_work_queue_priority_count("repair"),
            0
        );
        assert_eq!(
            evidence.memory_hygiene_work_queue_dispatch_codes(),
            vec![
                "dispatch:operator_review:isolated:kvswap_boundary_repair:quarantine:100:1"
                    .to_owned()
            ]
        );
        assert_eq!(
            evidence.memory_hygiene_work_queue_detail_codes(),
            vec!["kvswap_boundary_repair:quarantine:100:1".to_owned()]
        );
        assert_eq!(
            evidence.memory_hygiene_work_queue_reason_codes(),
            vec![
                "isolation_recommended".to_owned(),
                "items_present".to_owned(),
                "operator_review_required".to_owned()
            ]
        );
        assert_eq!(
            evidence.memory_hygiene_work_queue_reason_count("items_present"),
            1
        );
    }

    #[test]
    fn projection_bundle_startup_evidence_uses_stable_codes_not_payload_text() {
        let prompt_secret = "PROMPT_SECRET_DO_NOT_LOG";
        let lesson_secret = "LESSON_SECRET_DO_NOT_LOG";
        let gist_secret = "GIST_SECRET_DO_NOT_LOG";
        let memory_secret = "MEMORY_SECRET_DO_NOT_LOG";
        let experiences = vec![
            ExperienceEnvelope::new("clean", prompt_secret, lesson_secret)
                .with_scope(MemoryScope::for_task("runtime"))
                .with_clean_gist(gist_secret)
                .with_quality(0.9)
                .with_tags(vec!["adapter:test".to_owned()]),
        ];
        let memory_entries = vec![
            RetentionMemoryEntry::new("mem", memory_secret, vec![0.1], 2.4)
                .with_feedback(4, 0)
                .with_access(1, 10),
        ];
        let adapters = vec![status(
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
            true,
            AdapterWriteMode::ReadOnly,
        )];
        let seed_ledger = MemoryEvolutionLedger {
            replay_runs: 1,
            replay_items: 1,
            replay_memory_updates: 1,
            ..MemoryEvolutionLedger::default()
        };
        let bundle = AdapterProjectionContractBundle::standard_shadow();

        let dry_run = MemoryServiceDryRun::for_inputs(
            MemoryServiceShadowPlanInputs::new("bundle_shadow", &experiences, &[], &memory_entries)
                .with_adapters(&adapters)
                .with_scope(&MemoryScope::for_task("runtime"))
                .with_evolution_ledger(seed_ledger)
                .with_projection_contract_bundle(&bundle),
            &MemoryMigrationEvidence::copied_fixture(1),
            &[MemoryMigrationPhase::ReadOnlyShadow],
            false,
        );
        let evidence = dry_run.startup_evidence();
        let text = evidence.summary_text();
        let summary_line = evidence.summary_line();

        assert!(!dry_run.requires_operator_review());
        assert!(evidence.is_complete());
        assert!(text.contains("adapter_projection_bundle name=standard_shadow"));
        assert!(text.contains(
            "adapter_projection_contract_bundle_manifest name=standard_shadow target=shadow_read"
        ));
        assert!(summary_line.contains("projection_contracts=7"));
        assert!(summary_line.contains("projection_contract_manifests=7"));
        assert!(summary_line.contains("context_rot_risks=0"));
        assert!(summary_line.contains("migration_guard_codes=none"));
        assert!(summary_line.contains("kvswap_boundary_issues=0"));
        assert_eq!(evidence.context_rot_risk_count(), 0);
        assert_eq!(
            evidence.migration_evidence_guard_codes(),
            Vec::<String>::new()
        );
        assert_eq!(evidence.kvswap_boundary_issue_count(), 0);
        assert_eq!(
            evidence.status_codes(),
            vec![
                "complete".to_owned(),
                "phases_approved".to_owned(),
                "review_clear".to_owned(),
            ]
        );
        for forbidden in [prompt_secret, lesson_secret, gist_secret, memory_secret] {
            assert!(
                !text.contains(forbidden),
                "startup evidence leaked payload text: {forbidden}"
            );
            assert!(
                !summary_line.contains(forbidden),
                "startup summary leaked payload text: {forbidden}"
            );
        }
    }

    #[test]
    fn startup_evidence_requires_manifest_for_each_projection_contract() {
        let experiences = vec![
            ExperienceEnvelope::new("clean", "prompt", "Stable clean lesson")
                .with_scope(MemoryScope::for_task("runtime"))
                .with_clean_gist("A stable memory summary with enough useful detail.")
                .with_quality(0.9)
                .with_tags(vec!["adapter:test".to_owned()]),
        ];
        let memory_entries = vec![
            RetentionMemoryEntry::new("mem", "semantic durable memory", vec![0.1], 2.4)
                .with_feedback(4, 0)
                .with_access(1, 10),
        ];
        let adapters = vec![status(
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
            true,
            AdapterWriteMode::ReadOnly,
        )];
        let bundle = AdapterProjectionContractBundle::standard_shadow();
        let dry_run = MemoryServiceDryRun::for_inputs(
            MemoryServiceShadowPlanInputs::new("bundle_shadow", &experiences, &[], &memory_entries)
                .with_adapters(&adapters)
                .with_scope(&MemoryScope::for_task("runtime"))
                .with_kvswap_state(KvSwapStateSnapshot {
                    hot_shard_count: 1,
                    cold_shard_count: 1,
                    metadata_count: 2,
                    hot_byte_len: 4,
                    cold_byte_len: 8,
                })
                .with_projection_contract_bundle(&bundle),
            &MemoryMigrationEvidence::read_only_source(Some(1)),
            &[MemoryMigrationPhase::ReadOnlyShadow],
            false,
        );
        let mut evidence = dry_run.startup_evidence();
        evidence
            .lines
            .retain(|line| !line.starts_with("adapter_projection_contract adapter="));

        assert!(!evidence.is_complete());
        assert_eq!(evidence.projection_contract_count(), 7);
        assert_eq!(evidence.projection_contract_manifest_count(), 0);
        assert_eq!(evidence.projection_contract_manifest_gap(), 7);
        assert!(
            evidence
                .missing_required_codes()
                .contains(&"adapter_projection_contract".to_owned())
        );
        assert!(
            evidence
                .detail_codes()
                .contains(&"projection_contract_manifest_gap:7".to_owned())
        );
        assert!(
            evidence
                .status_codes()
                .contains(&"incomplete_evidence".to_owned())
        );
        assert!(
            evidence
                .summary_line()
                .contains("projection_contract_manifest_gap=7")
        );
    }

    #[test]
    fn startup_evidence_carries_blocked_projection_bundle_detail_codes() {
        let experiences = vec![
            ExperienceEnvelope::new("clean", "prompt", "Stable clean lesson")
                .with_scope(MemoryScope::for_task("runtime"))
                .with_clean_gist("A stable memory summary with enough useful detail.")
                .with_quality(0.9)
                .with_tags(vec!["adapter:test".to_owned()]),
        ];
        let memory_entries = vec![
            RetentionMemoryEntry::new("mem", "semantic durable memory", vec![0.1], 2.4)
                .with_feedback(4, 0)
                .with_access(1, 10),
        ];
        let adapters = vec![status(
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
            true,
            AdapterWriteMode::ReadOnly,
        )];
        let seed_ledger = MemoryEvolutionLedger {
            replay_runs: 1,
            replay_items: 1,
            replay_memory_updates: 1,
            ..MemoryEvolutionLedger::default()
        };
        let bundle = AdapterProjectionContractBundle::new(
            "blocked_fixture",
            AdapterProjectionTarget::IsolatedWrite,
            vec![
                AdapterProjectionContract::experience_store_read_only(
                    "experience_shadow",
                    vec![
                        crate::AdapterProjectionField::ExperienceId,
                        crate::AdapterProjectionField::ExperiencePrompt,
                        crate::AdapterProjectionField::ExperienceLesson,
                        crate::AdapterProjectionField::ExperienceQuality,
                        crate::AdapterProjectionField::ExperienceCleanGist,
                    ],
                ),
                AdapterProjectionContract::disk_kv_store_shadow("disk_shadow"),
            ],
        );

        let dry_run = MemoryServiceDryRun::for_inputs(
            MemoryServiceShadowPlanInputs::new(
                "blocked_bundle_shadow",
                &experiences,
                &[],
                &memory_entries,
            )
            .with_adapters(&adapters)
            .with_scope(&MemoryScope::for_task("runtime"))
            .with_evolution_ledger(seed_ledger)
            .with_projection_contract_bundle(&bundle),
            &MemoryMigrationEvidence::read_only_source(Some(1)),
            &[MemoryMigrationPhase::ReadOnlyShadow],
            false,
        );
        let evidence = dry_run.startup_evidence();
        let text = evidence.summary_text();
        let detail_codes = evidence.detail_codes();

        assert!(dry_run.requires_operator_review());
        assert_eq!(dry_run.summary.projection_contract_count, 2);
        assert_eq!(dry_run.summary.projection_contract_manifest_count, 2);
        assert_eq!(dry_run.summary.projection_contract_blocker_count, 6);
        assert_eq!(dry_run.summary.projection_contract_warning_count, 2);
        assert_eq!(
            dry_run.plan.projection_bundle_summary,
            Some(bundle.coverage_summary())
        );
        assert!(text.contains(
            "adapter_projection_bundle name=blocked_fixture target=isolated_write ready=false review=true contracts=2 ready_contracts=0 blockers=6 warnings=2"
        ));
        assert!(text.contains(
            "adapter_projection_contract_bundle_manifest name=blocked_fixture target=isolated_write contracts=2"
        ));
        assert!(
            text.contains(
                "blocker_detail_codes=disk_shadow:missing_required:kv_compaction_isolation"
            )
        );
        assert!(text.contains("experience_shadow:write_mode_not_isolated:read_only"));
        assert!(text.contains(
            "adapter_projection adapter=experience_shadow kind=experience_store target=isolated_write ready=false"
        ));
        assert!(text.contains(
            "adapter_projection adapter=disk_shadow kind=disk_kv_store target=isolated_write ready=false"
        ));
        assert!(
            detail_codes.contains(
                &"experience_shadow:missing_required:experience_projection_tags".to_owned()
            )
        );
        assert!(
            detail_codes
                .contains(&"experience_shadow:write_mode_not_isolated:read_only".to_owned())
        );
        assert!(
            detail_codes.contains(&"disk_shadow:missing_required:kv_delete_tombstone".to_owned())
        );
        assert!(
            detail_codes.contains(
                &"experience_shadow:missing_recommended:experience_agent_scope".to_owned()
            )
        );
        assert!(
            evidence
                .summary_line()
                .contains("disk_shadow:missing_required:kv_compaction_isolation")
        );
        assert!(evidence.summary_line().contains("operator_review_required"));
    }

    #[test]
    fn copied_fixture_startup_evidence_keeps_catalog_and_projection_payload_safe() {
        let prompt_secret = "PROMPT_FIXTURE_SECRET_DO_NOT_LOG";
        let lesson_secret = "LESSON_FIXTURE_SECRET_DO_NOT_LOG";
        let gist_secret = "GIST_FIXTURE_SECRET_DO_NOT_LOG";
        let memory_secret = "MEMORY_FIXTURE_SECRET_DO_NOT_LOG";
        let missing_shard_id = "kv-byte-secret";
        let length_shard_id = "kv-byte-len-secret";
        let corrupt_shard_id = "kv-checksum-secret";
        let experiences = vec![
            ExperienceEnvelope::new("clean", prompt_secret, lesson_secret)
                .with_scope(MemoryScope::for_task("runtime"))
                .with_clean_gist(format!(
                    "A stable {gist_secret} memory summary with enough useful detail."
                ))
                .with_quality(0.9)
                .with_tags(vec!["adapter:test".to_owned()]),
        ];
        let memory_entries = vec![
            RetentionMemoryEntry::new("mem", memory_secret, vec![0.1], 2.4)
                .with_feedback(4, 0)
                .with_access(1, 10),
        ];
        let adapters = vec![status(
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
            true,
            AdapterWriteMode::ReadOnly,
        )];
        let seed_ledger = MemoryEvolutionLedger {
            replay_runs: 1,
            replay_items: 1,
            replay_memory_updates: 1,
            ..MemoryEvolutionLedger::default()
        };
        let bundle = AdapterProjectionContractBundle::new(
            "blocked_fixture",
            AdapterProjectionTarget::IsolatedWrite,
            vec![
                AdapterProjectionContract::experience_store_read_only(
                    "experience_shadow",
                    vec![
                        crate::AdapterProjectionField::ExperienceId,
                        crate::AdapterProjectionField::ExperiencePrompt,
                        crate::AdapterProjectionField::ExperienceLesson,
                        crate::AdapterProjectionField::ExperienceQuality,
                        crate::AdapterProjectionField::ExperienceCleanGist,
                    ],
                ),
                AdapterProjectionContract::disk_kv_store_shadow("disk_shadow"),
            ],
        );
        let verification = DiskKvCatalogVerification {
            missing_byte_ids: vec![missing_shard_id.to_owned()],
            byte_len_mismatch_ids: vec![length_shard_id.to_owned()],
            checksum_mismatch_ids: vec![corrupt_shard_id.to_owned()],
            ..DiskKvCatalogVerification::default()
        };
        let migration_evidence = MemoryMigrationEvidence::copied_disk_kv_fixture(&verification);

        let dry_run = MemoryServiceDryRun::for_inputs(
            MemoryServiceShadowPlanInputs::new(
                "blocked_fixture_shadow",
                &experiences,
                &[],
                &memory_entries,
            )
            .with_adapters(&adapters)
            .with_scope(&MemoryScope::for_task("runtime"))
            .with_evolution_ledger(seed_ledger)
            .with_projection_contract_bundle(&bundle),
            &migration_evidence,
            &[MemoryMigrationPhase::IsolatedWrite],
            false,
        );
        let evidence = dry_run.startup_evidence();
        let text = evidence.summary_text();
        let summary_line = evidence.summary_line();
        let detail_codes = evidence.detail_codes();

        assert!(dry_run.requires_operator_review());
        assert_eq!(
            migration_evidence.guard_codes(),
            vec![
                "fixture_catalog_not_verified".to_owned(),
                "fixture_checksum_not_verified".to_owned(),
            ]
        );
        assert!(
            detail_codes
                .contains(&"disk_kv_catalog:missing_bytes:6b762d627974652d736563726574".to_owned())
        );
        assert!(detail_codes.contains(
            &"disk_kv_catalog:byte_len_mismatch:6b762d627974652d6c656e2d736563726574".to_owned()
        ));
        assert!(detail_codes.contains(
            &"disk_kv_catalog:checksum_mismatch:6b762d636865636b73756d2d736563726574".to_owned()
        ));
        assert!(
            detail_codes.contains(
                &"experience_shadow:missing_required:experience_projection_tags".to_owned()
            )
        );
        assert!(
            evidence
                .migration_evidence_guard_codes()
                .contains(&"fixture_catalog_not_verified".to_owned())
        );
        assert!(
            evidence
                .migration_evidence_guard_codes()
                .contains(&"fixture_checksum_not_verified".to_owned())
        );
        let migration_detail_codes = evidence.migration_evidence_detail_codes();
        assert!(
            migration_detail_codes
                .contains(&"disk_kv_catalog:missing_bytes:6b762d627974652d736563726574".to_owned())
        );
        assert!(migration_detail_codes.contains(
            &"disk_kv_catalog:byte_len_mismatch:6b762d627974652d6c656e2d736563726574".to_owned()
        ));
        assert!(migration_detail_codes.contains(
            &"disk_kv_catalog:checksum_mismatch:6b762d636865636b73756d2d736563726574".to_owned()
        ));
        assert_eq!(evidence.disk_kv_catalog_missing_bytes_count(), 1);
        assert_eq!(evidence.disk_kv_catalog_byte_len_mismatch_count(), 1);
        assert_eq!(evidence.disk_kv_catalog_checksum_mismatch_count(), 1);
        assert_eq!(
            evidence.disk_kv_catalog_detail_codes_for("missing_bytes"),
            vec!["disk_kv_catalog:missing_bytes:6b762d627974652d736563726574".to_owned()]
        );
        assert_eq!(
            evidence.disk_kv_catalog_detail_codes_for("byte_len_mismatch"),
            vec![
                "disk_kv_catalog:byte_len_mismatch:6b762d627974652d6c656e2d736563726574".to_owned()
            ]
        );
        assert_eq!(
            evidence.disk_kv_catalog_detail_codes_for("checksum_mismatch"),
            vec![
                "disk_kv_catalog:checksum_mismatch:6b762d636865636b73756d2d736563726574".to_owned()
            ]
        );
        assert!(summary_line.contains(
            "migration_detail_codes=disk_kv_catalog:byte_len_mismatch:6b762d627974652d6c656e2d736563726574"
        ));
        assert!(
            summary_line
                .contains("disk_kv_catalog:checksum_mismatch:6b762d636865636b73756d2d736563726574")
        );
        assert!(
            summary_line.contains("disk_kv_catalog:missing_bytes:6b762d627974652d736563726574")
        );
        assert!(summary_line.contains("disk_shadow:missing_required:kv_compaction_isolation"));
        for forbidden in [
            prompt_secret,
            lesson_secret,
            gist_secret,
            memory_secret,
            missing_shard_id,
            length_shard_id,
            corrupt_shard_id,
        ] {
            assert!(
                !text.contains(forbidden),
                "startup evidence leaked payload text: {forbidden}"
            );
            assert!(
                !summary_line.contains(forbidden),
                "startup summary leaked payload text: {forbidden}"
            );
        }
    }

    #[test]
    fn startup_summary_separates_clean_fixture_dirty_boundary_and_projection_blockers() {
        let experiences = vec![
            ExperienceEnvelope::new("clean", "prompt", "Stable clean lesson")
                .with_scope(MemoryScope::for_task("runtime"))
                .with_clean_gist("A stable memory summary with enough useful detail.")
                .with_quality(0.9)
                .with_tags(vec!["adapter:test".to_owned()]),
        ];
        let memory_entries = vec![
            RetentionMemoryEntry::new("mem", "semantic durable memory", vec![0.1], 2.4)
                .with_feedback(4, 0)
                .with_access(1, 10),
        ];
        let adapters = vec![status(
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
            true,
            AdapterWriteMode::ReadOnly,
        )];
        let seed_ledger = MemoryEvolutionLedger {
            replay_runs: 1,
            replay_items: 1,
            replay_memory_updates: 1,
            ..MemoryEvolutionLedger::default()
        };
        let bundle = AdapterProjectionContractBundle::new(
            "blocked_fixture",
            AdapterProjectionTarget::IsolatedWrite,
            vec![
                AdapterProjectionContract::experience_store_read_only(
                    "experience_shadow",
                    vec![
                        crate::AdapterProjectionField::ExperienceId,
                        crate::AdapterProjectionField::ExperiencePrompt,
                        crate::AdapterProjectionField::ExperienceLesson,
                        crate::AdapterProjectionField::ExperienceQuality,
                        crate::AdapterProjectionField::ExperienceCleanGist,
                    ],
                ),
                AdapterProjectionContract::disk_kv_store_shadow("disk_shadow"),
            ],
        );
        let migration_evidence =
            MemoryMigrationEvidence::copied_disk_kv_fixture(&DiskKvCatalogVerification::default());
        let boundary = KvSwapBoundaryAudit {
            overlapping_hot_cold_ids: vec!["hot/cold".to_owned()],
            ..KvSwapBoundaryAudit::default()
        };

        let dry_run = MemoryServiceDryRun::for_inputs(
            MemoryServiceShadowPlanInputs::new(
                "blocked_boundary_shadow",
                &experiences,
                &[],
                &memory_entries,
            )
            .with_adapters(&adapters)
            .with_scope(&MemoryScope::for_task("runtime"))
            .with_evolution_ledger(seed_ledger)
            .with_projection_contract_bundle(&bundle)
            .with_kvswap_boundary(boundary),
            &migration_evidence,
            &[MemoryMigrationPhase::IsolatedWrite],
            false,
        );
        let evidence = dry_run.startup_evidence();
        let summary_line = evidence.summary_line();
        let detail_codes = evidence.detail_codes();

        assert!(dry_run.requires_operator_review());
        assert_eq!(migration_evidence.guard_codes(), Vec::<String>::new());
        assert_eq!(migration_evidence.detail_codes(), Vec::<String>::new());
        assert_eq!(
            evidence.migration_evidence_guard_codes(),
            Vec::<String>::new()
        );
        assert_eq!(
            evidence.migration_evidence_detail_codes(),
            Vec::<String>::new()
        );
        assert_eq!(evidence.kvswap_boundary_issue_count(), 1);
        assert_eq!(
            evidence.kvswap_boundary_reason_codes(),
            vec!["overlapping_hot_cold".to_owned()]
        );
        assert_eq!(
            evidence.kvswap_boundary_detail_codes(),
            vec!["overlap:686f742f636f6c64".to_owned()]
        );
        assert_eq!(dry_run.summary.projection_contract_blocker_count, 6);
        assert_eq!(dry_run.summary.kvswap_boundary_issue_count, 1);
        assert_eq!(
            dry_run.summary.kvswap_boundary_detail_codes,
            vec!["overlap:686f742f636f6c64".to_owned()]
        );
        assert_eq!(dry_run.summary.kvswap_boundary_overlap_count, 1);
        assert_eq!(
            dry_run.summary.kvswap_boundary_missing_hot_metadata_count,
            0
        );
        assert_eq!(dry_run.summary.kvswap_boundary_stale_metadata_count, 0);
        assert_eq!(dry_run.summary.kvswap_boundary_hot_tier_mismatch_count, 0);
        assert_eq!(dry_run.summary.kvswap_boundary_cold_tier_mismatch_count, 0);
        assert!(
            dry_run
                .summary
                .reason_codes()
                .contains(&"projection_contract_blockers".to_owned())
        );
        assert!(
            dry_run
                .summary
                .reason_codes()
                .contains(&"kvswap_boundary_review".to_owned())
        );
        assert!(
            detail_codes.contains(&"disk_shadow:missing_required:kv_delete_tombstone".to_owned())
        );
        assert!(detail_codes.contains(&"overlap:686f742f636f6c64".to_owned()));
        assert!(summary_line.contains("migration_guard_codes=none"));
        assert!(summary_line.contains("migration_detail_codes=none"));
        assert!(summary_line.contains("kvswap_boundary_issues=1"));
        assert!(summary_line.contains("kvswap_boundary_reason_codes=overlapping_hot_cold"));
        assert!(summary_line.contains("kvswap_boundary_detail_codes=overlap:686f742f636f6c64"));
        assert!(summary_line.contains("disk_shadow:missing_required:kv_compaction_isolation"));
    }

    #[test]
    fn service_dry_run_blocks_live_write_by_default_across_multiple_phases() {
        let experiences = vec![
            ExperienceEnvelope::new("clean", "prompt", "Stable clean lesson")
                .with_scope(MemoryScope::for_task("runtime"))
                .with_clean_gist("A stable memory summary with enough useful detail.")
                .with_quality(0.9)
                .with_tags(vec!["adapter:test".to_owned()]),
        ];
        let memory_entries = vec![
            RetentionMemoryEntry::new("mem", "semantic durable memory", vec![0.1], 2.4)
                .with_feedback(4, 0)
                .with_access(1, 10),
        ];
        let adapters = vec![status(
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
            true,
            AdapterWriteMode::ReadOnly,
        )];
        let seed_ledger = MemoryEvolutionLedger {
            replay_runs: 1,
            replay_items: 1,
            replay_memory_updates: 1,
            ..MemoryEvolutionLedger::default()
        };
        let phases = [
            MemoryMigrationPhase::ReadOnlyShadow,
            MemoryMigrationPhase::CopiedFixtureWrite,
            MemoryMigrationPhase::IsolatedWrite,
            MemoryMigrationPhase::LiveWrite,
        ];

        let dry_run = MemoryServiceDryRun::for_inputs(
            MemoryServiceShadowPlanInputs::new("clean_shadow", &experiences, &[], &memory_entries)
                .with_adapters(&adapters)
                .with_scope(&MemoryScope::for_task("runtime"))
                .with_evolution_ledger(seed_ledger),
            &MemoryMigrationEvidence::copied_fixture(1),
            &phases,
            true,
        );

        assert_eq!(
            dry_run.approved_phases(),
            vec![
                MemoryMigrationPhase::ReadOnlyShadow,
                MemoryMigrationPhase::CopiedFixtureWrite,
                MemoryMigrationPhase::IsolatedWrite,
            ]
        );
        let live = dry_run
            .approval_for(MemoryMigrationPhase::LiveWrite)
            .unwrap();
        assert!(!live.approved);
        assert!(
            live.blockers
                .contains(&"live_write_disabled_by_policy".to_owned())
        );
        assert!(dry_run.requires_operator_review());
    }

    #[test]
    fn service_dry_run_surfaces_isolated_write_blockers() {
        let experiences = vec![
            ExperienceEnvelope::new("clean", "prompt", "Stable clean lesson")
                .with_scope(MemoryScope::for_task("runtime"))
                .with_clean_gist("A stable memory summary with enough useful detail.")
                .with_quality(0.9)
                .with_tags(vec!["adapter:test".to_owned()]),
        ];
        let memory_entries = vec![
            RetentionMemoryEntry::new("mem", "semantic durable memory", vec![0.1], 2.4)
                .with_feedback(4, 0)
                .with_access(1, 10),
        ];
        let adapters = vec![status(
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
            true,
            AdapterWriteMode::ReadOnly,
        )];
        let seed_ledger = MemoryEvolutionLedger {
            replay_runs: 1,
            replay_items: 1,
            replay_memory_updates: 1,
            ..MemoryEvolutionLedger::default()
        };
        let stale_projection = AdaptiveStateMemoryProjection {
            replay_runs: 2,
            replay_items: 1,
            replay_memory_updates: 1,
            ..AdaptiveStateMemoryProjection::default()
        };

        let dry_run = MemoryServiceDryRun::for_inputs(
            MemoryServiceShadowPlanInputs::new("clean_shadow", &experiences, &[], &memory_entries)
                .with_adapters(&adapters)
                .with_scope(&MemoryScope::for_task("runtime"))
                .with_evolution_ledger(seed_ledger)
                .with_adaptive_state_projection(&stale_projection),
            &MemoryMigrationEvidence::copied_fixture(1),
            &[MemoryMigrationPhase::IsolatedWrite],
            false,
        );

        let approval = dry_run
            .approval_for(MemoryMigrationPhase::IsolatedWrite)
            .unwrap();
        assert!(dry_run.requires_operator_review());
        assert!(!approval.approved);
        assert!(
            approval
                .blockers
                .contains(&"projection_parity_requires_operator_review".to_owned())
        );
        assert_eq!(
            dry_run.approved_phases(),
            Vec::<MemoryMigrationPhase>::new()
        );
    }

    #[test]
    fn adapter_checklist_is_satisfied_for_clean_dry_run() {
        let experiences = vec![
            ExperienceEnvelope::new("clean", "prompt", "Stable clean lesson")
                .with_scope(MemoryScope::for_task("runtime"))
                .with_clean_gist("A stable memory summary with enough useful detail.")
                .with_quality(0.9)
                .with_tags(vec!["adapter:test".to_owned()]),
        ];
        let memory_entries = vec![
            RetentionMemoryEntry::new("mem", "semantic durable memory", vec![0.1], 2.4)
                .with_feedback(4, 0)
                .with_access(1, 10),
        ];
        let adapters = vec![status(
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
            true,
            AdapterWriteMode::ReadOnly,
        )];
        let seed_ledger = MemoryEvolutionLedger {
            replay_runs: 1,
            replay_items: 1,
            replay_memory_updates: 1,
            ..MemoryEvolutionLedger::default()
        };

        let dry_run = MemoryServiceDryRun::for_inputs(
            MemoryServiceShadowPlanInputs::new("clean_shadow", &experiences, &[], &memory_entries)
                .with_adapters(&adapters)
                .with_scope(&MemoryScope::for_task("runtime"))
                .with_evolution_ledger(seed_ledger),
            &MemoryMigrationEvidence::copied_fixture(1),
            &[MemoryMigrationPhase::IsolatedWrite],
            false,
        );
        let checklist = dry_run.adapter_checklist();

        assert!(dry_run.summary.clean_gist_repair_is_clean());
        assert_eq!(dry_run.summary.clean_gist_repair_issue_count(), 0);
        assert!(dry_run.summary.context_rot_risk_is_clean());
        assert!(checklist.is_satisfied());
        assert!(checklist.blockers().is_empty());
        assert!(checklist.warnings().is_empty());
        assert!(
            checklist
                .summary_line()
                .contains("memory_adapter_checklist satisfied=true")
        );
        assert!(checklist.items.iter().any(|item| {
            item.code == "capability_manifest_ready"
                && item.satisfied
                && item.detail
                    == dry_run
                        .plan
                        .readiness
                        .capability_manifest_checklist_detail()
                && item.detail_codes().is_empty()
        }));
        assert!(checklist.items.iter().any(|item| {
            item.code == "projection_shadow_read_ready"
                && item.satisfied
                && item.detail == dry_run.plan.projection_audit.shadow_read_checklist_detail()
                && item.detail_codes().is_empty()
        }));
        assert!(checklist.items.iter().any(|item| {
            item.code == "projection_isolated_write_ready"
                && item.satisfied
                && item.detail
                    == dry_run
                        .plan
                        .projection_audit
                        .isolated_write_checklist_detail()
                && item.detail_codes().is_empty()
        }));
        assert!(checklist.items.iter().any(|item| {
            item.code == "evolution_gate_ready"
                && item.satisfied
                && item.detail == dry_run.plan.evolution_assessment.checklist_detail()
                && item
                    .detail
                    .contains("blocker_codes=none warning_codes=none")
                && item.detail_codes().is_empty()
        }));
        assert!(checklist.items.iter().any(|item| {
            item.code == "inspection_ready"
                && item.satisfied
                && item.detail == dry_run.plan.inspection.checklist_detail()
                && item.detail_codes().is_empty()
        }));
        assert!(checklist.items.iter().any(|item| {
            item.code == "projection_parity_clean"
                && item.satisfied
                && item.detail == dry_run.plan.projection_parity_audit.checklist_detail()
                && item.detail_codes().is_empty()
        }));
        assert!(checklist.items.iter().any(|item| {
            item.code == "context_rot_risks_clean"
                && item.satisfied
                && item.detail == dry_run.summary.context_rot_risk_checklist_detail()
                && item.detail_codes().is_empty()
        }));
        assert!(checklist.items.iter().any(|item| {
            item.code == "clean_gist_repair_clean"
                && item.satisfied
                && item.detail == dry_run.summary.clean_gist_repair_checklist_detail()
                && item.detail_codes().is_empty()
        }));
        assert!(checklist.items.iter().any(|item| {
            item.code == "experience_index_quality_gate_ready"
                && item.satisfied
                && item.detail == dry_run.plan.read_only.quality_gate.checklist_detail()
                && item.detail_codes().is_empty()
        }));
        assert!(checklist.items.iter().any(|item| {
            item.code == "context_gate_clean"
                && item.satisfied
                && item.detail == dry_run.summary.context_gate_checklist_detail()
                && item.detail_codes().is_empty()
        }));
        assert!(checklist.items.iter().any(|item| {
            item.code == "repair_plan_clean"
                && item.satisfied
                && item.detail == dry_run.summary.repair_plan_checklist_detail()
                && item.detail_codes().is_empty()
        }));
        assert!(checklist.items.iter().any(|item| {
            item.code == "replay_evidence_ready"
                && item.satisfied
                && item.detail
                    == dry_run
                        .plan
                        .evolution_ledger
                        .replay_evidence_checklist_detail()
        }));
        assert!(checklist.items.iter().any(|item| {
            item.code == "kvswap_intent_clean"
                && item.satisfied
                && item.detail == dry_run.summary.kvswap_intent_checklist_detail()
                && item.detail_codes().is_empty()
        }));
        assert!(checklist.items.iter().any(|item| {
            item.code == "kvswap_boundary_clean" && item.satisfied && item.detail_codes().is_empty()
        }));
        assert!(checklist.items.iter().any(|item| {
            item.code == "migration_evidence_ready"
                && item.satisfied
                && item.detail_codes().is_empty()
        }));
        let evolution_item = checklist
            .items
            .iter()
            .find(|item| item.code == "evolution_gate_ready")
            .unwrap();
        assert_eq!(
            evolution_item.summary_line(),
            "memory_adapter_checklist_item code=evolution_gate_ready satisfied=true severity=blocker detail_codes=none"
        );
    }

    #[test]
    fn adapter_checklist_reports_blockers_and_warnings_for_dirty_dry_run() {
        let experiences = vec![
            ExperienceEnvelope::new("keep", "runtime prompt", "runtime lesson")
                .with_scope(MemoryScope::for_task("runtime"))
                .with_tags(vec!["adapter:test".to_owned()]),
            ExperienceEnvelope::new(
                "rot",
                "Conversation Transcript:\nUser: ssh -o ConnectTimeout=1\nAssistant: ok",
                "accepted_pattern quality=0.1 max_severity=critical",
            )
            .with_scope(MemoryScope::for_task("ops"))
            .with_tags(vec!["adapter:test".to_owned()]),
        ];
        let memory_entries = vec![
            RetentionMemoryEntry::new("runtime", "runtime_kv:block", vec![0.2], 1.2)
                .with_feedback(1, 0)
                .with_access(1, 9),
        ];
        let adapters = vec![status(
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
            true,
            AdapterWriteMode::ReadOnly,
        )];
        let seed_ledger = MemoryEvolutionLedger {
            replay_runs: 1,
            replay_items: 1,
            replay_memory_updates: 1,
            ..MemoryEvolutionLedger::default()
        };
        let stale_projection = AdaptiveStateMemoryProjection {
            replay_runs: 2,
            replay_items: 1,
            replay_memory_updates: 1,
            ..AdaptiveStateMemoryProjection::default()
        };

        let dry_run = MemoryServiceDryRun::for_inputs(
            MemoryServiceShadowPlanInputs::new("dirty_shadow", &experiences, &[], &memory_entries)
                .with_adapters(&adapters)
                .with_scope(&MemoryScope::for_task("runtime"))
                .with_evolution_ledger(seed_ledger)
                .with_adaptive_state_projection(&stale_projection),
            &MemoryMigrationEvidence::copied_fixture(1),
            &[MemoryMigrationPhase::IsolatedWrite],
            false,
        );
        let checklist = dry_run.adapter_checklist();
        let blocker_codes = checklist
            .blockers()
            .iter()
            .map(|item| item.code.as_str())
            .collect::<Vec<_>>();
        let warning_codes = checklist
            .warnings()
            .iter()
            .map(|item| item.code.as_str())
            .collect::<Vec<_>>();
        let blocker_detail_codes = checklist.blocker_detail_codes();
        let warning_detail_codes = checklist.warning_detail_codes();
        let context_rot_item = checklist
            .items
            .iter()
            .find(|item| item.code == "context_rot_risks_clean")
            .unwrap();
        let clean_gist_item = checklist
            .items
            .iter()
            .find(|item| item.code == "clean_gist_repair_clean")
            .unwrap();
        let quality_gate_item = checklist
            .items
            .iter()
            .find(|item| item.code == "experience_index_quality_gate_ready")
            .unwrap();
        let context_gate_item = checklist
            .items
            .iter()
            .find(|item| item.code == "context_gate_clean")
            .unwrap();
        let repair_plan_item = checklist
            .items
            .iter()
            .find(|item| item.code == "repair_plan_clean")
            .unwrap();
        let replay_evidence_item = checklist
            .items
            .iter()
            .find(|item| item.code == "replay_evidence_ready")
            .unwrap();
        let kvswap_intent_item = checklist
            .items
            .iter()
            .find(|item| item.code == "kvswap_intent_clean")
            .unwrap();
        let projection_parity_item = checklist
            .items
            .iter()
            .find(|item| item.code == "projection_parity_clean")
            .unwrap();
        let migration_phase_item = checklist
            .items
            .iter()
            .find(|item| item.code == "migration_phase:isolated_write")
            .unwrap();

        assert!(!dry_run.summary.clean_gist_repair_is_clean());
        assert!(dry_run.summary.clean_gist_repair_issue_count() > 0);
        assert_eq!(
            clean_gist_item.detail,
            dry_run.summary.clean_gist_repair_checklist_detail()
        );
        assert_eq!(
            quality_gate_item.detail,
            dry_run.plan.read_only.quality_gate.checklist_detail()
        );
        assert_eq!(
            context_gate_item.detail,
            dry_run.summary.context_gate_checklist_detail()
        );
        assert_eq!(
            repair_plan_item.detail,
            dry_run.summary.repair_plan_checklist_detail()
        );
        assert_eq!(
            replay_evidence_item.detail,
            dry_run
                .plan
                .evolution_ledger
                .replay_evidence_checklist_detail()
        );
        assert_eq!(
            kvswap_intent_item.detail,
            dry_run.summary.kvswap_intent_checklist_detail()
        );
        assert_eq!(
            projection_parity_item.detail,
            dry_run.plan.projection_parity_audit.checklist_detail()
        );
        assert_eq!(
            migration_phase_item.detail,
            dry_run
                .approval_for(MemoryMigrationPhase::IsolatedWrite)
                .unwrap()
                .checklist_detail()
        );
        assert!(!dry_run.summary.context_rot_risk_is_clean());
        assert_eq!(
            context_rot_item.detail,
            dry_run.summary.context_rot_risk_checklist_detail()
        );
        assert!(!checklist.is_satisfied());
        assert!(blocker_codes.contains(&"projection_parity_clean"));
        assert!(blocker_codes.contains(&"migration_phase:isolated_write"));
        assert!(context_gate_item.satisfied);
        assert!(warning_codes.contains(&"context_rot_risks_clean"));
        assert!(warning_codes.contains(&"experience_index_quality_gate_ready"));
        assert!(warning_codes.contains(&"clean_gist_repair_clean"));
        assert!(
            blocker_detail_codes
                .iter()
                .any(|code| code == "projection_parity_clean:parity_mismatches")
        );
        assert!(
            blocker_detail_codes
                .iter()
                .any(|code| code == "migration_phase:isolated_write:blockers")
        );
        assert!(
            !warning_detail_codes
                .iter()
                .any(|code| code == "context_gate_clean:context_rejections")
        );
        assert!(
            warning_detail_codes
                .iter()
                .any(|code| code == "context_rot_risks_clean:context_rot_risks")
        );
        assert!(warning_detail_codes.iter().any(|code| {
            code == "context_rot_risks_clean:context_rot_risk_reason_codes:cross_task_transcript_pollution"
        }));
        assert!(warning_detail_codes.iter().any(|code| {
            code.starts_with(
                "context_rot_risks_clean:context_rot_risk_detail_codes:context_rot:rot:",
            )
        }));
        assert!(
            warning_detail_codes.iter().any(|code| {
                code == "experience_index_quality_gate_ready:quality_gate_blockers"
            })
        );
        assert!(warning_detail_codes.iter().any(|code| {
            code == "experience_index_quality_gate_ready:quality_gate_reason_codes:missing_clean_gist"
        }));
        assert!(warning_detail_codes.iter().any(|code| {
            code.starts_with(
                "experience_index_quality_gate_ready:quality_gate_detail_codes:warning:missing_clean_gist:",
            )
        }));
        assert!(
            warning_detail_codes
                .iter()
                .any(|code| code == "clean_gist_repair_clean:missing_clean_gist")
        );
        assert!(warning_detail_codes.iter().any(|code| {
            code.starts_with(
                "clean_gist_repair_clean:clean_gist_repair_detail_codes:missing_clean_gist:",
            )
        }));
        assert!(
            checklist
                .summary_line()
                .contains("blocker_codes=projection_parity_clean|migration_phase:isolated_write")
        );
        assert!(checklist.summary_line().contains("blocker_detail_codes="));
        assert!(checklist.summary_line().contains("warning_detail_codes="));
    }

    #[test]
    fn adapter_checklist_reports_disk_kv_fixture_evidence_guards() {
        let experiences = vec![
            ExperienceEnvelope::new("clean", "prompt", "Stable clean lesson")
                .with_scope(MemoryScope::for_task("runtime"))
                .with_clean_gist("A stable memory summary with enough useful detail.")
                .with_quality(0.9)
                .with_tags(vec!["adapter:test".to_owned()]),
        ];
        let memory_entries = vec![
            RetentionMemoryEntry::new("mem", "semantic durable memory", vec![0.1], 2.4)
                .with_feedback(4, 0)
                .with_access(1, 10),
        ];
        let adapters = vec![status(
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
            true,
            AdapterWriteMode::ReadOnly,
        )];
        let seed_ledger = MemoryEvolutionLedger {
            replay_runs: 1,
            replay_items: 1,
            replay_memory_updates: 1,
            ..MemoryEvolutionLedger::default()
        };
        let verification = DiskKvCatalogVerification {
            missing_byte_ids: vec!["missing".to_owned()],
            byte_len_mismatch_ids: vec!["stale-len".to_owned()],
            checksum_mismatch_ids: vec!["corrupt".to_owned()],
            ..DiskKvCatalogVerification::default()
        };
        let evidence = MemoryMigrationEvidence::copied_disk_kv_fixture(&verification);

        let dry_run = MemoryServiceDryRun::for_inputs(
            MemoryServiceShadowPlanInputs::new(
                "fixture_shadow",
                &experiences,
                &[],
                &memory_entries,
            )
            .with_adapters(&adapters)
            .with_scope(&MemoryScope::for_task("runtime"))
            .with_evolution_ledger(seed_ledger),
            &evidence,
            &[MemoryMigrationPhase::IsolatedWrite],
            false,
        );
        let checklist = dry_run.adapter_checklist();
        let blocker_codes = checklist
            .blockers()
            .iter()
            .map(|item| item.code.as_str())
            .collect::<Vec<_>>();
        let blocker_detail_codes = checklist.blocker_detail_codes();
        let evidence_item = checklist
            .items
            .iter()
            .find(|item| item.code == "migration_evidence_ready")
            .unwrap();

        assert!(!checklist.is_satisfied());
        assert!(blocker_codes.contains(&"migration_evidence_ready"));
        assert_eq!(evidence_item.detail, evidence.checklist_detail(true));
        assert!(blocker_detail_codes.iter().any(|code| {
            code == "migration_evidence_ready:guard_codes:fixture_catalog_not_verified"
        }));
        assert!(blocker_detail_codes.iter().any(|code| {
            code == "migration_evidence_ready:guard_codes:fixture_checksum_not_verified"
        }));
        assert!(blocker_detail_codes.iter().any(|code| {
            code == "migration_evidence_ready:detail_codes:disk_kv_catalog:missing_bytes:6d697373696e67"
        }));
        assert!(blocker_detail_codes.iter().any(|code| {
            code == "migration_evidence_ready:detail_codes:disk_kv_catalog:byte_len_mismatch:7374616c652d6c656e"
        }));
        assert!(blocker_detail_codes.iter().any(|code| {
            code == "migration_evidence_ready:detail_codes:disk_kv_catalog:checksum_mismatch:636f7272757074"
        }));
        assert!(
            checklist
                .summary_line()
                .contains("migration_evidence_ready")
        );
    }

    #[test]
    fn disk_kv_fixture_can_verify_while_kvswap_boundary_still_requires_review() {
        let experiences = vec![
            ExperienceEnvelope::new("clean", "prompt", "Stable clean lesson")
                .with_scope(MemoryScope::for_task("runtime"))
                .with_clean_gist("A stable memory summary with enough useful detail.")
                .with_quality(0.9)
                .with_tags(vec!["adapter:test".to_owned()]),
        ];
        let memory_entries = vec![
            RetentionMemoryEntry::new("mem", "semantic durable memory", vec![0.1], 2.4)
                .with_feedback(4, 0)
                .with_access(1, 10),
        ];
        let adapters = vec![status(
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
            true,
            AdapterWriteMode::ReadOnly,
        )];
        let seed_ledger = MemoryEvolutionLedger {
            replay_runs: 1,
            replay_items: 1,
            replay_memory_updates: 1,
            ..MemoryEvolutionLedger::default()
        };
        let verification = DiskKvCatalogVerification::default();
        let evidence = MemoryMigrationEvidence::copied_disk_kv_fixture(&verification);
        let boundary = KvSwapBoundaryAudit {
            overlapping_hot_cold_ids: vec!["shard-a".to_owned()],
            ..KvSwapBoundaryAudit::default()
        };

        let dry_run = MemoryServiceDryRun::for_inputs(
            MemoryServiceShadowPlanInputs::new(
                "fixture_boundary_shadow",
                &experiences,
                &[],
                &memory_entries,
            )
            .with_adapters(&adapters)
            .with_scope(&MemoryScope::for_task("runtime"))
            .with_evolution_ledger(seed_ledger)
            .with_kvswap_boundary(boundary),
            &evidence,
            &[MemoryMigrationPhase::IsolatedWrite],
            false,
        );
        let summary = dry_run.plan.summary();
        let checklist = dry_run.adapter_checklist();
        let blocker_codes = checklist
            .blockers()
            .iter()
            .map(|item| item.code.as_str())
            .collect::<Vec<_>>();
        let warning_codes = checklist
            .warnings()
            .iter()
            .map(|item| item.code.as_str())
            .collect::<Vec<_>>();
        let warning_detail_codes = checklist.warning_detail_codes();

        assert!(dry_run.requires_operator_review());
        assert_eq!(evidence.guard_codes(), Vec::<String>::new());
        assert!(summary.requires_operator_review);
        assert_eq!(
            summary.reason_codes(),
            vec![
                "evolution_review".to_owned(),
                "kvswap_boundary_review".to_owned()
            ]
        );
        assert_eq!(
            summary.kvswap_boundary_reason_codes,
            vec!["overlapping_hot_cold".to_owned()]
        );
        assert_eq!(summary.kvswap_boundary_blocker_count(), 1);
        assert_eq!(summary.kvswap_boundary_warning_count(), 0);
        assert_eq!(
            summary.kvswap_boundary_blocker_reason_codes(),
            vec!["overlapping_hot_cold".to_owned()]
        );
        assert_eq!(
            summary.kvswap_boundary_warning_reason_codes(),
            Vec::<String>::new()
        );
        assert_eq!(
            summary.kvswap_boundary_readiness_detail_codes(),
            vec!["blocker:overlap:73686172642d61".to_owned()]
        );
        assert!(
            summary
                .detail_codes()
                .contains(&"kvswap_boundary:overlapping_hot_cold".to_owned())
        );
        assert!(
            summary
                .detail_codes()
                .contains(&"kvswap_boundary_detail:overlap:73686172642d61".to_owned())
        );
        assert!(blocker_codes.contains(&"migration_phase:isolated_write"));
        assert!(!blocker_codes.contains(&"migration_evidence_ready"));
        assert!(warning_codes.contains(&"kvswap_boundary_clean"));
        let boundary_item = checklist
            .items
            .iter()
            .find(|item| item.code == "kvswap_boundary_clean")
            .unwrap();
        assert_eq!(
            boundary_item.detail,
            summary.kvswap_boundary_checklist_detail()
        );
        assert!(
            warning_detail_codes
                .iter()
                .any(|code| code == "kvswap_boundary_clean:boundary_issues")
        );
        assert!(warning_detail_codes.iter().any(|code| {
            code == "kvswap_boundary_clean:boundary_reason_codes:overlapping_hot_cold"
        }));
        assert!(
            warning_detail_codes
                .iter()
                .any(|code| code == "kvswap_boundary_clean:boundary_blockers")
        );
        assert!(warning_detail_codes.iter().any(|code| {
            code == "kvswap_boundary_clean:boundary_blocker_reason_codes:overlapping_hot_cold"
        }));
        assert!(warning_detail_codes.iter().any(|code| {
            code == "kvswap_boundary_clean:boundary_detail_codes:blocker:overlap:73686172642d61"
        }));
        assert!(checklist.items.iter().any(|item| {
            item.code == "migration_evidence_ready"
                && item.satisfied
                && item.detail == evidence.checklist_detail(true)
                && item.detail_codes().is_empty()
        }));
    }
}
