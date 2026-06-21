use std::collections::BTreeSet;

use crate::{
    ContextCandidate, ContextDecisionKind, ContextInjectionGate, ContextInjectionPlan,
    DefaultContextInjectionGate, DefaultExperienceGovernance, DefaultMemoryIndexPlanner,
    DefaultMemoryRepairPlanner, DefaultMemorySemanticRetriever, DefaultTieredMemoryPlanner,
    DiskKvOffload, ExperienceEnvelope, ExperienceGovernance, ExperienceIndexQualityGate,
    GovernanceReport, IndexRebuildPlan, KvShardMetadata, KvSwapIntent, MemoryAdapter,
    MemoryIndexDocument, MemoryIndexPlan, MemoryIndexPlanner, MemoryPlacementCandidate,
    MemoryRepairPlan, MemoryRepairPlanner, MemoryResult, MemoryScope, MemorySemanticQuery,
    MemorySemanticRetrievalPlan, MemorySemanticRetriever, TierBudgets, TieredMemoryPlan,
    TieredMemoryPlanner,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterWriteMode {
    ReadOnly,
    IsolatedWrite,
    LiveWrite,
}

impl AdapterWriteMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ReadOnly => "read_only",
            Self::IsolatedWrite => "isolated_write",
            Self::LiveWrite => "live_write",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterSnapshotSummary {
    pub adapter_name: String,
    pub write_mode: AdapterWriteMode,
    pub experience_count: usize,
    pub kv_shard_count: usize,
    pub warnings: Vec<String>,
}

impl AdapterSnapshotSummary {
    pub fn read_only(
        adapter_name: impl Into<String>,
        experience_count: usize,
        kv_shard_count: usize,
    ) -> Self {
        Self {
            adapter_name: adapter_name.into(),
            write_mode: AdapterWriteMode::ReadOnly,
            experience_count,
            kv_shard_count,
            warnings: Vec::new(),
        }
    }

    pub fn total_records(&self) -> usize {
        self.experience_count.saturating_add(self.kv_shard_count)
    }

    pub fn warning_codes(&self) -> Vec<String> {
        normalized_codes(&self.warnings)
    }

    pub fn detail_codes(&self) -> Vec<String> {
        self.warning_codes()
            .into_iter()
            .map(|code| format!("warning:{code}"))
            .collect()
    }

    pub fn status_codes(&self) -> Vec<String> {
        let mut codes = BTreeSet::new();
        if self.total_records() == 0 {
            codes.insert("empty_snapshot".to_owned());
        }
        if !self.warnings.is_empty() {
            codes.insert("snapshot_warnings".to_owned());
        }
        match self.write_mode {
            AdapterWriteMode::ReadOnly => {
                codes.insert("read_only".to_owned());
            }
            AdapterWriteMode::IsolatedWrite => {
                codes.insert("isolated_write".to_owned());
            }
            AdapterWriteMode::LiveWrite => {
                codes.insert("live_write".to_owned());
            }
        }
        codes.into_iter().collect()
    }

    pub fn summary_line(&self) -> String {
        format!(
            "adapter_snapshot adapter={} write_mode={} experiences={} kv_shards={} total_records={} warnings={} status_codes={} warning_codes={} detail_codes={}",
            self.adapter_name,
            self.write_mode.as_str(),
            self.experience_count,
            self.kv_shard_count,
            self.total_records(),
            self.warnings.len(),
            join_codes(self.status_codes()),
            join_codes(self.warning_codes()),
            join_codes(self.detail_codes()),
        )
    }
}

pub trait ExperienceSnapshotAdapter: MemoryAdapter {
    fn write_mode(&self) -> AdapterWriteMode {
        AdapterWriteMode::ReadOnly
    }

    fn snapshot(&self) -> MemoryResult<Vec<ExperienceEnvelope>>;

    fn snapshot_summary(&self) -> MemoryResult<AdapterSnapshotSummary> {
        let experiences = self.snapshot()?;
        let warnings = adapter_snapshot_warnings(self)?;
        Ok(AdapterSnapshotSummary {
            adapter_name: self.descriptor().name,
            write_mode: self.write_mode(),
            experience_count: experiences.len(),
            kv_shard_count: 0,
            warnings,
        })
    }

    fn snapshot_for_scope(&self, scope: &MemoryScope) -> MemoryResult<Vec<ExperienceEnvelope>> {
        Ok(self
            .snapshot()?
            .into_iter()
            .filter(|envelope| scope.same_task_as(&envelope.scope).unwrap_or(true))
            .collect())
    }

    fn index_documents(&self) -> MemoryResult<Vec<MemoryIndexDocument>> {
        Ok(self
            .snapshot()?
            .iter()
            .map(MemoryIndexDocument::from_experience)
            .collect())
    }
}

pub trait KvShardCatalogAdapter: MemoryAdapter {
    fn write_mode(&self) -> AdapterWriteMode {
        AdapterWriteMode::ReadOnly
    }

    fn kv_metadata(&self) -> MemoryResult<Vec<KvShardMetadata>>;

    fn catalog_summary(&self) -> MemoryResult<AdapterSnapshotSummary> {
        let metadata = self.kv_metadata()?;
        let warnings = adapter_snapshot_warnings(self)?;
        Ok(AdapterSnapshotSummary {
            adapter_name: self.descriptor().name,
            write_mode: self.write_mode(),
            experience_count: 0,
            kv_shard_count: metadata.len(),
            warnings,
        })
    }

    fn placement_candidates(&self) -> MemoryResult<Vec<MemoryPlacementCandidate>> {
        Ok(self
            .kv_metadata()?
            .iter()
            .map(MemoryPlacementCandidate::from_kv_metadata)
            .collect())
    }
}

impl<T> KvShardCatalogAdapter for T
where
    T: DiskKvOffload + MemoryAdapter,
{
    fn kv_metadata(&self) -> MemoryResult<Vec<KvShardMetadata>> {
        Ok(self.list_cold_metadata())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReadOnlyMemoryPlan {
    pub summary: AdapterSnapshotSummary,
    pub governance: GovernanceReport,
    pub rebuild: IndexRebuildPlan,
    pub quality_gate: ExperienceIndexQualityGate,
    pub repair: MemoryRepairPlan,
    pub index: MemoryIndexPlan,
    pub semantic: MemorySemanticRetrievalPlan,
    pub context: ContextInjectionPlan,
    pub placement: TieredMemoryPlan,
    pub kvswap: KvSwapIntent,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MigrationReadinessReport {
    pub ready_for_isolated_write: bool,
    pub operator_review_required: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
}

impl MigrationReadinessReport {
    pub fn from_plan(plan: &ReadOnlyMemoryPlan) -> Self {
        let mut blockers = Vec::new();
        let mut warnings = plan.summary.warnings.clone();

        if plan.summary.write_mode == AdapterWriteMode::LiveWrite {
            blockers.push("live_write_mode_not_allowed_for_initial_migration".to_owned());
        }
        if !plan.context.rejected_ids().is_empty() {
            warnings.push(format!(
                "context_rejections={}",
                plan.context.rejected_ids().len()
            ));
        }
        if plan.rebuild.rebuild_required {
            warnings.push("governance_rebuild_required".to_owned());
        }
        if plan.quality_gate.blocker_count > 0 {
            warnings.push(format!(
                "quality_gate_blockers={}",
                plan.quality_gate.blocker_count
            ));
        }
        if plan.quality_gate.warning_count > 0 {
            warnings.push(format!(
                "quality_gate_warnings={}",
                plan.quality_gate.warning_count
            ));
        }
        if !plan.repair.items.is_empty() {
            warnings.push(format!("repair_items={}", plan.repair.items.len()));
        }
        if !plan.repair.skipped.is_empty() {
            warnings.push(format!("repair_skipped={}", plan.repair.skipped.len()));
        }
        if !plan.kvswap.is_empty() {
            warnings.push("kvswap_intent_pending".to_owned());
        }
        warnings.sort();
        warnings.dedup();

        Self {
            ready_for_isolated_write: blockers.is_empty(),
            operator_review_required: !warnings.is_empty(),
            blockers,
            warnings,
        }
    }

    pub fn blocker_codes(&self) -> Vec<String> {
        normalized_codes(&self.blockers)
    }

    pub fn warning_codes(&self) -> Vec<String> {
        normalized_codes(&self.warnings)
    }

    pub fn blocker_details(&self) -> Vec<String> {
        self.blockers.clone()
    }

    pub fn warning_details(&self) -> Vec<String> {
        self.warnings.clone()
    }

    pub fn blocker_detail_codes(&self) -> Vec<String> {
        readiness_detail_codes(&self.blockers)
    }

    pub fn warning_detail_codes(&self) -> Vec<String> {
        readiness_detail_codes(&self.warnings)
    }

    pub fn summary_line(&self) -> String {
        format!(
            "memory_migration_readiness isolated_write_ready={} review={} blockers={} warnings={} blocker_codes={} warning_codes={} blocker_detail_codes={} warning_detail_codes={} blocker_details={} warning_details={}",
            self.ready_for_isolated_write,
            self.operator_review_required,
            self.blockers.len(),
            self.warnings.len(),
            join_codes(self.blocker_codes()),
            join_codes(self.warning_codes()),
            join_codes(self.blocker_detail_codes()),
            join_codes(self.warning_detail_codes()),
            join_details(&self.blockers),
            join_details(&self.warnings),
        )
    }
}

fn readiness_detail_codes(items: &[String]) -> Vec<String> {
    items
        .iter()
        .map(|item| match item.split_once('=') {
            Some((key, value)) if value.chars().all(|ch| ch.is_ascii_digit() || ch == '.') => {
                format!("{}={value}", detail_token(key))
            }
            Some((key, _)) => detail_token(key),
            None => detail_token(item),
        })
        .filter(|code| !code.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn adapter_snapshot_warnings<A: MemoryAdapter + ?Sized>(adapter: &A) -> MemoryResult<Vec<String>> {
    let health = adapter.health()?;
    let mut warnings = health.warnings;
    if !health.ready {
        warnings.push("adapter_unhealthy".to_owned());
    }
    warnings.sort();
    warnings.dedup();
    Ok(warnings)
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

fn normalized_codes(items: &[String]) -> Vec<String> {
    items
        .iter()
        .map(|item| {
            item.split_once('=')
                .or_else(|| item.split_once(':'))
                .map_or(item.as_str(), |(code, _)| code)
        })
        .filter(|code| !code.is_empty())
        .map(str::to_owned)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn join_codes(codes: Vec<String>) -> String {
    if codes.is_empty() {
        "none".to_owned()
    } else {
        codes.join("|")
    }
}

fn join_static_codes(codes: Vec<&'static str>) -> String {
    if codes.is_empty() {
        "none".to_owned()
    } else {
        codes.join("|")
    }
}

fn join_details(details: &[String]) -> String {
    if details.is_empty() {
        "none".to_owned()
    } else {
        details.join("|")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExperienceProjectionHints {
    pub adapter: String,
    pub task_profile: Option<String>,
    pub runtime_model: Option<String>,
    pub device_profile: Option<String>,
    pub memory_mode: Option<String>,
}

impl ExperienceProjectionHints {
    pub fn new(adapter: impl Into<String>) -> Self {
        Self {
            adapter: adapter.into(),
            task_profile: None,
            runtime_model: None,
            device_profile: None,
            memory_mode: None,
        }
    }

    pub fn with_task_profile(mut self, task_profile: impl Into<String>) -> Self {
        self.task_profile = Some(task_profile.into());
        self
    }

    pub fn with_runtime_model(mut self, runtime_model: impl Into<String>) -> Self {
        self.runtime_model = Some(runtime_model.into());
        self
    }

    pub fn with_device_profile(mut self, device_profile: impl Into<String>) -> Self {
        self.device_profile = Some(device_profile.into());
        self
    }

    pub fn with_memory_mode(mut self, memory_mode: impl Into<String>) -> Self {
        self.memory_mode = Some(memory_mode.into());
        self
    }

    pub fn tags(&self) -> Vec<String> {
        let mut tags = vec![format!("adapter:{}", self.adapter)];
        if let Some(task_profile) = &self.task_profile {
            tags.push(format!("task_profile:{task_profile}"));
        }
        if let Some(runtime_model) = &self.runtime_model {
            tags.push(format!("runtime_model:{runtime_model}"));
        }
        if let Some(device_profile) = &self.device_profile {
            tags.push(format!("device_profile:{device_profile}"));
        }
        if let Some(memory_mode) = &self.memory_mode {
            tags.push(format!("memory_mode:{memory_mode}"));
        }
        tags
    }

    pub fn apply_to(&self, mut envelope: ExperienceEnvelope) -> ExperienceEnvelope {
        envelope.tags.extend(self.tags());
        envelope.tags.sort();
        envelope.tags.dedup();
        if envelope.scope.task_id.is_none() {
            envelope.scope.task_id = self.task_profile.clone();
        }
        envelope
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AdapterProjectionKind {
    ExperienceStore,
    DiskKvStore,
    GistMemory,
    InfiniMemory,
    KvCache,
    ServiceMemory,
    TieredCache,
}

impl AdapterProjectionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ExperienceStore => "experience_store",
            Self::DiskKvStore => "disk_kv_store",
            Self::GistMemory => "gist_memory",
            Self::InfiniMemory => "infini_memory",
            Self::KvCache => "kv_cache",
            Self::ServiceMemory => "service_memory",
            Self::TieredCache => "tiered_cache",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AdapterProjectionTarget {
    ShadowRead,
    IsolatedWrite,
}

impl AdapterProjectionTarget {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ShadowRead => "shadow_read",
            Self::IsolatedWrite => "isolated_write",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AdapterProjectionField {
    ExperienceId,
    ExperiencePrompt,
    ExperienceLesson,
    ExperienceQuality,
    ExperienceCleanGist,
    ExperienceProjectionTags,
    ExperienceTaskScope,
    ExperienceSessionScope,
    ExperienceAgentScope,
    KvShardId,
    KvShardBytes,
    KvShardMetadata,
    KvShardChecksum,
    KvShardTier,
    KvShardPriority,
    KvShardLastAccess,
    KvDeleteTombstone,
    KvCompactionIsolation,
    GistId,
    GistText,
    GistImportance,
    GistSourceExperienceId,
    InfiniItemId,
    InfiniScope,
    InfiniScore,
    InfiniTokenEstimate,
    KvCacheEntryId,
    KvCacheVector,
    KvCacheStrength,
    KvCacheLastAccess,
    KvCacheNamespace,
    TierPlacementId,
    TierPlacementBytes,
    TierPlacementPriority,
    ServiceDescriptor,
    ServiceHealth,
}

impl AdapterProjectionField {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ExperienceId => "experience_id",
            Self::ExperiencePrompt => "experience_prompt",
            Self::ExperienceLesson => "experience_lesson",
            Self::ExperienceQuality => "experience_quality",
            Self::ExperienceCleanGist => "experience_clean_gist",
            Self::ExperienceProjectionTags => "experience_projection_tags",
            Self::ExperienceTaskScope => "experience_task_scope",
            Self::ExperienceSessionScope => "experience_session_scope",
            Self::ExperienceAgentScope => "experience_agent_scope",
            Self::KvShardId => "kv_shard_id",
            Self::KvShardBytes => "kv_shard_bytes",
            Self::KvShardMetadata => "kv_shard_metadata",
            Self::KvShardChecksum => "kv_shard_checksum",
            Self::KvShardTier => "kv_shard_tier",
            Self::KvShardPriority => "kv_shard_priority",
            Self::KvShardLastAccess => "kv_shard_last_access",
            Self::KvDeleteTombstone => "kv_delete_tombstone",
            Self::KvCompactionIsolation => "kv_compaction_isolation",
            Self::GistId => "gist_id",
            Self::GistText => "gist_text",
            Self::GistImportance => "gist_importance",
            Self::GistSourceExperienceId => "gist_source_experience_id",
            Self::InfiniItemId => "infini_item_id",
            Self::InfiniScope => "infini_scope",
            Self::InfiniScore => "infini_score",
            Self::InfiniTokenEstimate => "infini_token_estimate",
            Self::KvCacheEntryId => "kv_cache_entry_id",
            Self::KvCacheVector => "kv_cache_vector",
            Self::KvCacheStrength => "kv_cache_strength",
            Self::KvCacheLastAccess => "kv_cache_last_access",
            Self::KvCacheNamespace => "kv_cache_namespace",
            Self::TierPlacementId => "tier_placement_id",
            Self::TierPlacementBytes => "tier_placement_bytes",
            Self::TierPlacementPriority => "tier_placement_priority",
            Self::ServiceDescriptor => "service_descriptor",
            Self::ServiceHealth => "service_health",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterProjectionContract {
    pub adapter_name: String,
    pub kind: AdapterProjectionKind,
    pub write_mode: AdapterWriteMode,
    pub fields: Vec<AdapterProjectionField>,
    pub notes: Vec<String>,
}

impl AdapterProjectionContract {
    pub fn new(
        adapter_name: impl Into<String>,
        kind: AdapterProjectionKind,
        write_mode: AdapterWriteMode,
        fields: Vec<AdapterProjectionField>,
    ) -> Self {
        Self {
            adapter_name: adapter_name.into(),
            kind,
            write_mode,
            fields: sorted_unique_fields(fields),
            notes: Vec::new(),
        }
    }

    pub fn experience_store_read_only(
        adapter_name: impl Into<String>,
        fields: Vec<AdapterProjectionField>,
    ) -> Self {
        Self::new(
            adapter_name,
            AdapterProjectionKind::ExperienceStore,
            AdapterWriteMode::ReadOnly,
            fields,
        )
    }

    pub fn disk_kv_store_read_only(
        adapter_name: impl Into<String>,
        fields: Vec<AdapterProjectionField>,
    ) -> Self {
        Self::new(
            adapter_name,
            AdapterProjectionKind::DiskKvStore,
            AdapterWriteMode::ReadOnly,
            fields,
        )
    }

    pub fn for_target(
        adapter_name: impl Into<String>,
        kind: AdapterProjectionKind,
        target: AdapterProjectionTarget,
        write_mode: AdapterWriteMode,
    ) -> Self {
        let fields = required_projection_fields(kind, target)
            .iter()
            .chain(recommended_projection_fields(kind, target).iter())
            .copied()
            .collect::<Vec<_>>();
        Self::new(adapter_name, kind, write_mode, fields)
    }

    pub fn experience_store_shadow(adapter_name: impl Into<String>) -> Self {
        Self::for_target(
            adapter_name,
            AdapterProjectionKind::ExperienceStore,
            AdapterProjectionTarget::ShadowRead,
            AdapterWriteMode::ReadOnly,
        )
    }

    pub fn experience_store_isolated_write(adapter_name: impl Into<String>) -> Self {
        Self::for_target(
            adapter_name,
            AdapterProjectionKind::ExperienceStore,
            AdapterProjectionTarget::IsolatedWrite,
            AdapterWriteMode::IsolatedWrite,
        )
    }

    pub fn disk_kv_store_shadow(adapter_name: impl Into<String>) -> Self {
        Self::for_target(
            adapter_name,
            AdapterProjectionKind::DiskKvStore,
            AdapterProjectionTarget::ShadowRead,
            AdapterWriteMode::ReadOnly,
        )
    }

    pub fn disk_kv_copied_fixture(adapter_name: impl Into<String>) -> Self {
        Self::for_target(
            adapter_name,
            AdapterProjectionKind::DiskKvStore,
            AdapterProjectionTarget::IsolatedWrite,
            AdapterWriteMode::IsolatedWrite,
        )
    }

    pub fn gist_memory_shadow(adapter_name: impl Into<String>) -> Self {
        Self::for_target(
            adapter_name,
            AdapterProjectionKind::GistMemory,
            AdapterProjectionTarget::ShadowRead,
            AdapterWriteMode::ReadOnly,
        )
    }

    pub fn infini_memory_shadow(adapter_name: impl Into<String>) -> Self {
        Self::for_target(
            adapter_name,
            AdapterProjectionKind::InfiniMemory,
            AdapterProjectionTarget::ShadowRead,
            AdapterWriteMode::ReadOnly,
        )
    }

    pub fn kv_cache_shadow(adapter_name: impl Into<String>) -> Self {
        Self::for_target(
            adapter_name,
            AdapterProjectionKind::KvCache,
            AdapterProjectionTarget::ShadowRead,
            AdapterWriteMode::ReadOnly,
        )
    }

    pub fn tiered_cache_shadow(adapter_name: impl Into<String>) -> Self {
        Self::for_target(
            adapter_name,
            AdapterProjectionKind::TieredCache,
            AdapterProjectionTarget::ShadowRead,
            AdapterWriteMode::ReadOnly,
        )
    }

    pub fn service_memory_shadow(adapter_name: impl Into<String>) -> Self {
        Self::for_target(
            adapter_name,
            AdapterProjectionKind::ServiceMemory,
            AdapterProjectionTarget::ShadowRead,
            AdapterWriteMode::ReadOnly,
        )
    }

    pub fn with_write_mode(mut self, write_mode: AdapterWriteMode) -> Self {
        self.write_mode = write_mode;
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn has_field(&self, field: AdapterProjectionField) -> bool {
        self.fields.binary_search(&field).is_ok()
    }

    pub fn mapped_field_codes(&self) -> Vec<&'static str> {
        self.fields.iter().map(|field| field.as_str()).collect()
    }

    pub fn required_fields_for(
        &self,
        target: AdapterProjectionTarget,
    ) -> Vec<AdapterProjectionField> {
        required_projection_fields(self.kind, target).to_vec()
    }

    pub fn recommended_fields_for(
        &self,
        target: AdapterProjectionTarget,
    ) -> Vec<AdapterProjectionField> {
        recommended_projection_fields(self.kind, target).to_vec()
    }

    pub fn required_field_codes_for(&self, target: AdapterProjectionTarget) -> Vec<&'static str> {
        required_projection_fields(self.kind, target)
            .iter()
            .map(|field| field.as_str())
            .collect()
    }

    pub fn recommended_field_codes_for(
        &self,
        target: AdapterProjectionTarget,
    ) -> Vec<&'static str> {
        recommended_projection_fields(self.kind, target)
            .iter()
            .map(|field| field.as_str())
            .collect()
    }

    pub fn manifest_line(&self, target: AdapterProjectionTarget) -> String {
        format!(
            "adapter_projection_contract adapter={} kind={} target={} write_mode={} mapped_fields={} required_fields={} recommended_fields={} notes={}",
            self.adapter_name,
            self.kind.as_str(),
            target.as_str(),
            self.write_mode.as_str(),
            join_static_codes(self.mapped_field_codes()),
            join_static_codes(self.required_field_codes_for(target)),
            join_static_codes(self.recommended_field_codes_for(target)),
            self.notes.len(),
        )
    }

    pub fn coverage_report(
        &self,
        target: AdapterProjectionTarget,
    ) -> AdapterProjectionCoverageReport {
        let missing_required_fields = required_projection_fields(self.kind, target)
            .iter()
            .copied()
            .filter(|field| !self.has_field(*field))
            .collect::<Vec<_>>();
        let missing_recommended_fields = recommended_projection_fields(self.kind, target)
            .iter()
            .copied()
            .filter(|field| !self.has_field(*field))
            .collect::<Vec<_>>();
        let mut blockers = missing_required_fields
            .iter()
            .map(|field| format!("missing_required:{}", field.as_str()))
            .collect::<Vec<_>>();
        let mut warnings = missing_recommended_fields
            .iter()
            .map(|field| format!("missing_recommended:{}", field.as_str()))
            .collect::<Vec<_>>();

        match target {
            AdapterProjectionTarget::ShadowRead => {
                if self.write_mode == AdapterWriteMode::LiveWrite {
                    blockers.push("live_write_not_allowed_for_shadow_read".to_owned());
                }
            }
            AdapterProjectionTarget::IsolatedWrite => {
                if self.write_mode != AdapterWriteMode::IsolatedWrite {
                    blockers.push(format!(
                        "write_mode_not_isolated:{}",
                        self.write_mode.as_str()
                    ));
                }
            }
        }

        if self.notes.is_empty() {
            warnings.sort();
        } else {
            warnings.extend(self.notes.iter().map(|note| format!("note:{note}")));
            warnings.sort();
        }
        blockers.sort();
        blockers.dedup();
        warnings.dedup();

        AdapterProjectionCoverageReport {
            adapter_name: self.adapter_name.clone(),
            kind: self.kind,
            target,
            write_mode: self.write_mode,
            ready: blockers.is_empty(),
            missing_required_fields,
            missing_recommended_fields,
            blockers,
            warnings,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterProjectionCoverageReport {
    pub adapter_name: String,
    pub kind: AdapterProjectionKind,
    pub target: AdapterProjectionTarget,
    pub write_mode: AdapterWriteMode,
    pub ready: bool,
    pub missing_required_fields: Vec<AdapterProjectionField>,
    pub missing_recommended_fields: Vec<AdapterProjectionField>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
}

impl AdapterProjectionCoverageReport {
    pub fn requires_operator_review(&self) -> bool {
        !self.ready || !self.warnings.is_empty()
    }

    pub fn missing_required_codes(&self) -> Vec<&'static str> {
        self.missing_required_fields
            .iter()
            .map(|field| field.as_str())
            .collect()
    }

    pub fn missing_recommended_codes(&self) -> Vec<&'static str> {
        self.missing_recommended_fields
            .iter()
            .map(|field| field.as_str())
            .collect()
    }

    pub fn blocker_codes(&self) -> Vec<String> {
        normalized_codes(&self.blockers)
    }

    pub fn warning_codes(&self) -> Vec<String> {
        normalized_codes(&self.warnings)
    }

    pub fn blocker_detail_codes(&self) -> Vec<String> {
        prefixed_projection_details(&self.adapter_name, &self.blockers)
    }

    pub fn warning_detail_codes(&self) -> Vec<String> {
        prefixed_projection_details(&self.adapter_name, &self.warnings)
    }

    pub fn blocker_details(&self) -> Vec<String> {
        self.blockers.clone()
    }

    pub fn warning_details(&self) -> Vec<String> {
        self.warnings.clone()
    }

    pub fn summary_line(&self) -> String {
        format!(
            "adapter_projection adapter={} kind={} target={} ready={} write_mode={} missing_required={} missing_recommended={} blockers={} warnings={} blocker_codes={} warning_codes={} blocker_detail_codes={} warning_detail_codes={}",
            self.adapter_name,
            self.kind.as_str(),
            self.target.as_str(),
            self.ready,
            self.write_mode.as_str(),
            self.missing_required_fields.len(),
            self.missing_recommended_fields.len(),
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
            join_codes(self.blocker_codes()),
            join_codes(self.warning_codes()),
            join_codes(self.blocker_detail_codes()),
            join_codes(self.warning_detail_codes()),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterProjectionBundleReport {
    pub name: String,
    pub target: AdapterProjectionTarget,
    pub ready: bool,
    pub requires_operator_review: bool,
    pub contract_count: usize,
    pub ready_contract_count: usize,
    pub blocker_count: usize,
    pub warning_count: usize,
    pub reports: Vec<AdapterProjectionCoverageReport>,
}

impl AdapterProjectionBundleReport {
    pub fn from_reports(
        name: impl Into<String>,
        target: AdapterProjectionTarget,
        reports: Vec<AdapterProjectionCoverageReport>,
    ) -> Self {
        let ready_contract_count = reports.iter().filter(|report| report.ready).count();
        let blocker_count = reports.iter().map(|report| report.blockers.len()).sum();
        let warning_count = reports.iter().map(|report| report.warnings.len()).sum();
        let contract_count = reports.len();
        let ready = blocker_count == 0 && ready_contract_count == contract_count;

        Self {
            name: name.into(),
            target,
            ready,
            requires_operator_review: !ready || warning_count > 0,
            contract_count,
            ready_contract_count,
            blocker_count,
            warning_count,
            reports,
        }
    }

    pub fn summary_line(&self) -> String {
        format!(
            "adapter_projection_bundle name={} target={} ready={} review={} contracts={} ready_contracts={} blockers={} warnings={} blocker_codes={} warning_codes={} blocker_detail_codes={} warning_detail_codes={}",
            self.name,
            self.target.as_str(),
            self.ready,
            self.requires_operator_review,
            self.contract_count,
            self.ready_contract_count,
            self.blocker_count,
            self.warning_count,
            join_codes(self.blocker_codes()),
            join_codes(self.warning_codes()),
            join_codes(self.blocker_detail_codes()),
            join_codes(self.warning_detail_codes()),
        )
    }

    pub fn blocker_codes(&self) -> Vec<String> {
        self.reports
            .iter()
            .flat_map(AdapterProjectionCoverageReport::blocker_codes)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn warning_codes(&self) -> Vec<String> {
        self.reports
            .iter()
            .flat_map(AdapterProjectionCoverageReport::warning_codes)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn blocker_detail_codes(&self) -> Vec<String> {
        self.reports
            .iter()
            .flat_map(AdapterProjectionCoverageReport::blocker_detail_codes)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn warning_detail_codes(&self) -> Vec<String> {
        self.reports
            .iter()
            .flat_map(AdapterProjectionCoverageReport::warning_detail_codes)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn blocker_details(&self) -> Vec<String> {
        self.reports
            .iter()
            .flat_map(AdapterProjectionCoverageReport::blocker_detail_codes)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn warning_details(&self) -> Vec<String> {
        self.reports
            .iter()
            .flat_map(AdapterProjectionCoverageReport::warning_detail_codes)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }
}

fn prefixed_projection_details(adapter_name: &str, details: &[String]) -> Vec<String> {
    details
        .iter()
        .map(|detail| format!("{adapter_name}:{detail}"))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterProjectionContractBundle {
    pub name: String,
    pub target: AdapterProjectionTarget,
    pub contracts: Vec<AdapterProjectionContract>,
}

impl AdapterProjectionContractBundle {
    pub fn new(
        name: impl Into<String>,
        target: AdapterProjectionTarget,
        contracts: Vec<AdapterProjectionContract>,
    ) -> Self {
        Self {
            name: name.into(),
            target,
            contracts,
        }
    }

    pub fn standard_shadow() -> Self {
        Self::new(
            "standard_shadow",
            AdapterProjectionTarget::ShadowRead,
            vec![
                AdapterProjectionContract::experience_store_shadow("experience_store"),
                AdapterProjectionContract::disk_kv_store_shadow("disk_kv_store"),
                AdapterProjectionContract::gist_memory_shadow("gist_memory"),
                AdapterProjectionContract::infini_memory_shadow("infini_memory"),
                AdapterProjectionContract::kv_cache_shadow("kv_cache"),
                AdapterProjectionContract::tiered_cache_shadow("tiered_cache"),
                AdapterProjectionContract::service_memory_shadow("service_memory"),
            ],
        )
    }

    pub fn copied_fixture_isolated_write() -> Self {
        Self::new(
            "copied_fixture_isolated_write",
            AdapterProjectionTarget::IsolatedWrite,
            vec![
                AdapterProjectionContract::experience_store_isolated_write(
                    "experience_store_fixture",
                ),
                AdapterProjectionContract::disk_kv_copied_fixture("disk_kv_fixture"),
            ],
        )
    }

    pub fn with_contract(mut self, contract: AdapterProjectionContract) -> Self {
        self.contracts.push(contract);
        self
    }

    pub fn is_empty(&self) -> bool {
        self.contracts.is_empty()
    }

    pub fn coverage_reports(&self) -> Vec<AdapterProjectionCoverageReport> {
        self.contracts
            .iter()
            .map(|contract| contract.coverage_report(self.target))
            .collect()
    }

    pub fn coverage_summary(&self) -> AdapterProjectionBundleReport {
        AdapterProjectionBundleReport::from_reports(
            self.name.clone(),
            self.target,
            self.coverage_reports(),
        )
    }

    pub fn manifest_lines(&self) -> Vec<String> {
        self.contracts
            .iter()
            .map(|contract| contract.manifest_line(self.target))
            .collect()
    }

    pub fn adapter_codes(&self) -> Vec<String> {
        self.contracts
            .iter()
            .map(|contract| normalized_detail_token(&contract.adapter_name))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn manifest_summary_line(&self) -> String {
        let mapped_field_count = self
            .contracts
            .iter()
            .map(|contract| contract.mapped_field_codes().len())
            .sum::<usize>();
        let required_field_count = self
            .contracts
            .iter()
            .map(|contract| contract.required_field_codes_for(self.target).len())
            .sum::<usize>();
        let recommended_field_count = self
            .contracts
            .iter()
            .map(|contract| contract.recommended_field_codes_for(self.target).len())
            .sum::<usize>();
        let note_count = self
            .contracts
            .iter()
            .map(|contract| contract.notes.len())
            .sum::<usize>();
        format!(
            "adapter_projection_contract_bundle_manifest name={} target={} contracts={} adapters={} mapped_fields={} required_fields={} recommended_fields={} notes={}",
            self.name,
            self.target.as_str(),
            self.contracts.len(),
            join_codes(self.adapter_codes()),
            mapped_field_count,
            required_field_count,
            recommended_field_count,
            note_count,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AdapterProjectionIssueSeverity {
    Warning,
    Blocker,
}

impl AdapterProjectionIssueSeverity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Warning => "warning",
            Self::Blocker => "blocker",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterProjectionIssue {
    pub severity: AdapterProjectionIssueSeverity,
    pub source_id: Option<String>,
    pub code: String,
    pub message: String,
}

impl AdapterProjectionIssue {
    pub fn warning(
        source_id: impl Into<Option<String>>,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity: AdapterProjectionIssueSeverity::Warning,
            source_id: source_id.into(),
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn blocker(
        source_id: impl Into<Option<String>>,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity: AdapterProjectionIssueSeverity::Blocker,
            source_id: source_id.into(),
            code: code.into(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AdapterProjectionAudit {
    pub experience_count: usize,
    pub kv_shard_count: usize,
    pub issues: Vec<AdapterProjectionIssue>,
}

impl AdapterProjectionAudit {
    pub fn blockers(&self) -> Vec<&AdapterProjectionIssue> {
        self.issues
            .iter()
            .filter(|issue| issue.severity == AdapterProjectionIssueSeverity::Blocker)
            .collect()
    }

    pub fn warnings(&self) -> Vec<&AdapterProjectionIssue> {
        self.issues
            .iter()
            .filter(|issue| issue.severity == AdapterProjectionIssueSeverity::Warning)
            .collect()
    }

    pub fn is_ready_for_shadow_read(&self) -> bool {
        self.blockers().is_empty()
    }

    pub fn is_ready_for_isolated_write(&self) -> bool {
        self.is_ready_for_shadow_read()
            && !self
                .issues
                .iter()
                .any(|issue| issue.code == "missing_task_scope")
    }

    pub fn shadow_read_checklist_detail(&self) -> String {
        format!("projection_blockers={}", self.blockers().len())
    }

    pub fn isolated_write_checklist_detail(&self) -> String {
        format!("projection_issues={}", self.issues.len())
    }

    pub fn issue_codes(&self) -> Vec<String> {
        let mut codes = self
            .issues
            .iter()
            .map(|issue| issue.code.clone())
            .collect::<Vec<_>>();
        codes.sort();
        codes.dedup();
        codes
    }

    pub fn detail_codes(&self) -> Vec<String> {
        self.issues
            .iter()
            .map(|issue| {
                let mut code = format!(
                    "{}:{}",
                    issue.severity.as_str(),
                    normalized_detail_token(&issue.code)
                );
                if let Some(source_id) = issue.source_id.as_deref() {
                    code.push_str(":source_id_hex:");
                    code.push_str(&hex_id(source_id));
                }
                code
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn summary_line(&self) -> String {
        let issue_codes = self.issue_codes();
        format!(
            "adapter_projection_audit shadow_ready={} isolated_write_ready={} experiences={} kv_shards={} issues={} blockers={} warnings={} issue_codes={} detail_codes={}",
            self.is_ready_for_shadow_read(),
            self.is_ready_for_isolated_write(),
            self.experience_count,
            self.kv_shard_count,
            self.issues.len(),
            self.blockers().len(),
            self.warnings().len(),
            if issue_codes.is_empty() {
                "none".to_owned()
            } else {
                issue_codes.join("|")
            },
            join_codes(self.detail_codes()),
        )
    }
}

fn normalized_detail_token(value: &str) -> String {
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

fn hex_id(id: &str) -> String {
    id.as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join("")
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdapterProjectionAuditPolicy {
    pub require_task_scope_for_isolated_write: bool,
    pub require_clean_gist_for_risky_records: bool,
    pub max_clean_gist_chars: usize,
}

impl Default for AdapterProjectionAuditPolicy {
    fn default() -> Self {
        Self {
            require_task_scope_for_isolated_write: true,
            require_clean_gist_for_risky_records: true,
            max_clean_gist_chars: 420,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultAdapterProjectionAuditor {
    pub policy: AdapterProjectionAuditPolicy,
}

impl DefaultAdapterProjectionAuditor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn audit(
        &self,
        experiences: &[ExperienceEnvelope],
        kv_metadata: &[KvShardMetadata],
    ) -> AdapterProjectionAudit {
        let mut issues = Vec::new();
        audit_experiences(experiences, self.policy, &mut issues);
        audit_kv_metadata(kv_metadata, &mut issues);
        issues.sort_by(|left, right| {
            left.severity
                .cmp(&right.severity)
                .then_with(|| left.code.cmp(&right.code))
                .then_with(|| left.source_id.cmp(&right.source_id))
        });

        AdapterProjectionAudit {
            experience_count: experiences.len(),
            kv_shard_count: kv_metadata.len(),
            issues,
        }
    }
}

impl MemoryAdapter for DefaultAdapterProjectionAuditor {
    fn descriptor(&self) -> crate::MemoryAdapterDescriptor {
        crate::MemoryAdapterDescriptor::new(
            "default_adapter_projection_auditor",
            vec![
                crate::MemoryAdapterCapability::ExperienceGovernance,
                crate::MemoryAdapterCapability::DiskKvOffload,
            ],
        )
        .read_only()
    }

    fn health(&self) -> MemoryResult<crate::MemoryAdapterHealth> {
        Ok(crate::MemoryAdapterHealth::ready(None))
    }
}

impl ReadOnlyMemoryPlan {
    pub fn for_inputs(
        adapter_name: impl Into<String>,
        experiences: &[ExperienceEnvelope],
        kv_metadata: &[KvShardMetadata],
        scope: Option<&MemoryScope>,
        budgets: TierBudgets,
        previous_placement: Option<&TieredMemoryPlan>,
        target_hot_bytes: usize,
    ) -> Self {
        let governance = DefaultExperienceGovernance::default();
        let report = match scope {
            Some(scope) => governance.assess_for_scope(experiences, scope),
            None => governance.assess(experiences),
        };
        let rebuild = match scope {
            Some(scope) => governance.rebuild_plan_for_scope(experiences, scope),
            None => governance.rebuild_plan(experiences),
        };
        let quality_gate = report.quality_gate(&rebuild);
        let repair = DefaultMemoryRepairPlanner.plan(experiences, &rebuild);
        let documents = experiences
            .iter()
            .map(MemoryIndexDocument::from_experience)
            .collect::<Vec<_>>();
        let index = DefaultMemoryIndexPlanner.plan(&documents, &rebuild);
        let context_request_scope = scope.cloned().unwrap_or_default();
        let context_request = crate::MemoryRequestContext::new(
            context_request_scope.clone(),
            crate::MemoryAccessPurpose::Recall,
        );
        let semantic_query = MemorySemanticQuery::new(
            read_only_semantic_query_text(scope, &context_request_scope),
            context_request.limit,
        )
        .with_scope(context_request_scope.clone())
        .with_token_budget(context_request.limit.saturating_mul(128).max(1));
        let semantic = DefaultMemorySemanticRetriever.retrieve(&documents, &semantic_query);
        let semantic_documents = semantic
            .matches
            .iter()
            .map(|item| {
                MemoryIndexDocument::new(item.id.clone(), item.source, item.content.clone())
                    .with_scope(item.scope.clone())
                    .with_metadata(item.metadata.clone())
                    .with_strength(item.score.max(item.strength))
            })
            .collect::<Vec<_>>();
        let context_candidates = semantic_documents
            .iter()
            .map(ContextCandidate::from_index_document)
            .collect::<Vec<_>>();
        let context =
            DefaultContextInjectionGate::new().plan(&context_candidates, &context_request);
        let candidates = kv_metadata
            .iter()
            .map(MemoryPlacementCandidate::from_kv_metadata)
            .collect::<Vec<_>>();
        let placement = DefaultTieredMemoryPlanner.plan(&candidates, budgets);
        let kvswap = previous_placement
            .map(|previous| {
                let migrations = placement.migrations_from(previous);
                KvSwapIntent::from_migrations(&migrations, target_hot_bytes)
            })
            .unwrap_or_default();

        Self {
            summary: AdapterSnapshotSummary::read_only(
                adapter_name,
                experiences.len(),
                kv_metadata.len(),
            ),
            governance: report,
            rebuild,
            quality_gate,
            repair,
            index,
            semantic,
            context,
            placement,
            kvswap,
        }
    }

    pub fn requires_operator_review(&self) -> bool {
        self.rebuild.rebuild_required
            || !self.quality_gate.ready_for_context_injection
            || self.index.requires_rebuild()
            || !self.repair.items.is_empty()
            || !self.repair.skipped.is_empty()
            || !self.context.rejected_ids().is_empty()
            || !self.kvswap.is_empty()
            || !self.summary.warnings.is_empty()
    }

    pub fn reason_codes(&self) -> Vec<String> {
        let mut codes = BTreeSet::new();
        insert_prefixed_codes(
            &mut codes,
            "adapter",
            normalized_codes(&self.summary.warnings),
        );
        insert_prefixed_codes(&mut codes, "governance", self.governance.reason_codes());
        insert_prefixed_codes(&mut codes, "rebuild", self.rebuild.reason_codes());
        insert_prefixed_codes(&mut codes, "quality_gate", self.quality_gate.reason_codes());
        insert_prefixed_codes(&mut codes, "repair", self.repair.reason_codes());
        insert_prefixed_codes(
            &mut codes,
            "repair_skipped",
            self.repair.skipped_reason_codes(),
        );
        insert_prefixed_codes(&mut codes, "index", self.index.reason_codes());
        insert_prefixed_codes(&mut codes, "semantic", self.semantic.reason_codes());
        insert_prefixed_codes(&mut codes, "context", self.context.reason_codes());
        insert_prefixed_codes(&mut codes, "kvswap", self.kvswap.reason_codes());
        codes.into_iter().collect()
    }

    pub fn detail_codes(&self) -> Vec<String> {
        let mut codes = BTreeSet::new();
        insert_prefixed_codes(&mut codes, "governance", self.governance.detail_codes());
        insert_prefixed_codes(&mut codes, "rebuild", self.rebuild.detail_codes());
        insert_prefixed_codes(&mut codes, "quality_gate", self.quality_gate.detail_codes());
        insert_prefixed_codes(&mut codes, "repair", self.repair.detail_codes());
        insert_prefixed_codes(&mut codes, "index", self.index.detail_codes());
        insert_prefixed_codes(&mut codes, "semantic", self.semantic.skip_detail_codes());
        insert_prefixed_codes(&mut codes, "context", self.context.detail_codes());
        insert_prefixed_codes(&mut codes, "kvswap", self.kvswap.detail_codes());
        codes.into_iter().collect()
    }

    pub fn summary_line(&self) -> String {
        let tier_counts = self.placement.counts();
        let context_admit = self
            .context
            .decisions
            .iter()
            .filter(|decision| decision.kind == ContextDecisionKind::Admit)
            .count();
        let context_summarize = self
            .context
            .decisions
            .iter()
            .filter(|decision| decision.kind == ContextDecisionKind::Summarize)
            .count();
        let context_reject = self.context.decisions.len() - context_admit - context_summarize;

        format!(
            "memory_read_only_plan adapter={} write_mode={} review={} experiences={} kv_shards={} noisy={} context_rot={} rebuild_required={} rebuild_reasons={} quality_gate_ready={} quality_gate_blockers={} quality_gate_warnings={} repair_items={} repair_skipped={} index_ops={} index_skipped={} semantic_matches={} semantic_skipped={} semantic_tokens={} context_admit={} context_summarize={} context_reject={} context_tokens={} hot_gpu={} warm_ram={} cold_disk={} kvswap_empty={} reason_codes={} detail_codes={}",
            self.summary.adapter_name,
            self.summary.write_mode.as_str(),
            self.requires_operator_review(),
            self.summary.experience_count,
            self.summary.kv_shard_count,
            self.governance.noisy_records.len(),
            self.governance.context_rot_risks.len(),
            self.rebuild.rebuild_required,
            self.rebuild.reasons.len(),
            self.quality_gate.ready_for_context_injection,
            self.quality_gate.blocker_count,
            self.quality_gate.warning_count,
            self.repair.items.len(),
            self.repair.skipped.len(),
            self.index.operations.len(),
            self.index.skipped_ids.len(),
            self.semantic.matches.len(),
            self.semantic.skipped.len(),
            self.semantic.used_tokens,
            context_admit,
            context_summarize,
            context_reject,
            self.context.used_tokens,
            tier_counts.hot_gpu,
            tier_counts.warm_ram,
            tier_counts.cold_disk,
            self.kvswap.is_empty(),
            join_codes(self.reason_codes()),
            join_codes(self.detail_codes()),
        )
    }
}

fn read_only_semantic_query_text(
    scope: Option<&MemoryScope>,
    fallback_scope: &MemoryScope,
) -> String {
    scope
        .and_then(|scope| scope.task_id.as_deref())
        .or(fallback_scope.task_id.as_deref())
        .unwrap_or("memory recall")
        .to_owned()
}

fn insert_prefixed_codes(codes: &mut BTreeSet<String>, prefix: &str, source: Vec<String>) {
    for code in source {
        codes.insert(format!("{prefix}:{code}"));
    }
}

fn audit_experiences(
    experiences: &[ExperienceEnvelope],
    policy: AdapterProjectionAuditPolicy,
    issues: &mut Vec<AdapterProjectionIssue>,
) {
    let mut ids = BTreeSet::new();
    for envelope in experiences {
        let id = envelope.id.trim();
        if id.is_empty() {
            issues.push(AdapterProjectionIssue::blocker(
                None,
                "empty_experience_id",
                "projected experience id is empty",
            ));
            continue;
        }
        if !ids.insert(id.to_owned()) {
            issues.push(AdapterProjectionIssue::blocker(
                Some(envelope.id.clone()),
                "duplicate_experience_id",
                "projected experience id is duplicated",
            ));
        }
        if envelope.prompt.trim().is_empty() && envelope.lesson.trim().is_empty() {
            issues.push(AdapterProjectionIssue::blocker(
                Some(envelope.id.clone()),
                "empty_experience_content",
                "projected experience has no prompt or lesson content",
            ));
        }
        if policy.require_task_scope_for_isolated_write && envelope.scope.task_id.is_none() {
            issues.push(AdapterProjectionIssue::warning(
                Some(envelope.id.clone()),
                "missing_task_scope",
                "projected experience has no task scope for isolated-write gating",
            ));
        }
        if envelope.tags.is_empty() {
            issues.push(AdapterProjectionIssue::warning(
                Some(envelope.id.clone()),
                "missing_projection_tags",
                "projected experience has no adapter/profile/runtime tags",
            ));
        }
        if let Some(gist) = envelope.clean_gist.as_deref() {
            if gist.trim().is_empty() || gist.chars().count() > policy.max_clean_gist_chars {
                issues.push(AdapterProjectionIssue::warning(
                    Some(envelope.id.clone()),
                    "dirty_clean_gist_projection",
                    "projected clean gist is empty or too long",
                ));
            }
        } else if policy.require_clean_gist_for_risky_records
            && (has_transcript_shape(&envelope.prompt)
                || has_metadata_lesson_shape(&envelope.lesson))
        {
            issues.push(AdapterProjectionIssue::warning(
                Some(envelope.id.clone()),
                "missing_clean_gist_for_risky_record",
                "risky projected experience is missing a clean gist",
            ));
        }
    }
}

fn audit_kv_metadata(kv_metadata: &[KvShardMetadata], issues: &mut Vec<AdapterProjectionIssue>) {
    let mut ids = BTreeSet::new();
    for metadata in kv_metadata {
        let id = metadata.id.trim();
        if id.is_empty() {
            issues.push(AdapterProjectionIssue::blocker(
                None,
                "empty_kv_shard_id",
                "projected KV shard id is empty",
            ));
            continue;
        }
        if !ids.insert(id.to_owned()) {
            issues.push(AdapterProjectionIssue::blocker(
                Some(metadata.id.clone()),
                "duplicate_kv_shard_id",
                "projected KV shard id is duplicated",
            ));
        }
        if metadata.byte_len == 0 {
            issues.push(AdapterProjectionIssue::warning(
                Some(metadata.id.clone()),
                "empty_kv_shard",
                "projected KV shard has zero byte length",
            ));
        }
        if metadata.checksum == 0 && metadata.byte_len > 0 {
            issues.push(AdapterProjectionIssue::warning(
                Some(metadata.id.clone()),
                "missing_kv_checksum",
                "projected KV shard has bytes but no checksum",
            ));
        }
        if !metadata.priority.is_finite() {
            issues.push(AdapterProjectionIssue::blocker(
                Some(metadata.id.clone()),
                "invalid_kv_priority",
                "projected KV shard priority is not finite",
            ));
        } else if !(0.0..=1.0).contains(&metadata.priority) {
            issues.push(AdapterProjectionIssue::warning(
                Some(metadata.id.clone()),
                "kv_priority_out_of_range",
                "projected KV shard priority should be normalized to 0..=1",
            ));
        }
    }
}

fn has_transcript_shape(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    value.contains("conversation transcript:")
        || (value.contains("user:") && value.contains("assistant:"))
}

fn has_metadata_lesson_shape(value: &str) -> bool {
    let value = value.trim().to_ascii_lowercase();
    value.starts_with("accepted_pattern ")
        || value.starts_with("rejected_pattern ")
        || ((value.contains("quality=") || value.contains("overlap="))
            && value.contains("max_severity="))
}

fn sorted_unique_fields(mut fields: Vec<AdapterProjectionField>) -> Vec<AdapterProjectionField> {
    fields.sort();
    fields.dedup();
    fields
}

fn required_projection_fields(
    kind: AdapterProjectionKind,
    target: AdapterProjectionTarget,
) -> &'static [AdapterProjectionField] {
    match (kind, target) {
        (AdapterProjectionKind::ExperienceStore, AdapterProjectionTarget::ShadowRead) => &[
            AdapterProjectionField::ExperienceId,
            AdapterProjectionField::ExperiencePrompt,
            AdapterProjectionField::ExperienceLesson,
            AdapterProjectionField::ExperienceQuality,
        ],
        (AdapterProjectionKind::ExperienceStore, AdapterProjectionTarget::IsolatedWrite) => &[
            AdapterProjectionField::ExperienceId,
            AdapterProjectionField::ExperiencePrompt,
            AdapterProjectionField::ExperienceLesson,
            AdapterProjectionField::ExperienceQuality,
            AdapterProjectionField::ExperienceProjectionTags,
            AdapterProjectionField::ExperienceTaskScope,
        ],
        (AdapterProjectionKind::DiskKvStore, AdapterProjectionTarget::ShadowRead) => &[
            AdapterProjectionField::KvShardId,
            AdapterProjectionField::KvShardBytes,
            AdapterProjectionField::KvShardMetadata,
            AdapterProjectionField::KvShardChecksum,
        ],
        (AdapterProjectionKind::DiskKvStore, AdapterProjectionTarget::IsolatedWrite) => &[
            AdapterProjectionField::KvShardId,
            AdapterProjectionField::KvShardBytes,
            AdapterProjectionField::KvShardMetadata,
            AdapterProjectionField::KvShardChecksum,
            AdapterProjectionField::KvShardTier,
            AdapterProjectionField::KvShardPriority,
            AdapterProjectionField::KvShardLastAccess,
            AdapterProjectionField::KvDeleteTombstone,
            AdapterProjectionField::KvCompactionIsolation,
        ],
        (AdapterProjectionKind::ServiceMemory, _) => &[
            AdapterProjectionField::ServiceDescriptor,
            AdapterProjectionField::ServiceHealth,
        ],
        (AdapterProjectionKind::GistMemory, _) => &[
            AdapterProjectionField::GistId,
            AdapterProjectionField::GistText,
            AdapterProjectionField::GistSourceExperienceId,
        ],
        (AdapterProjectionKind::InfiniMemory, _) => &[
            AdapterProjectionField::InfiniItemId,
            AdapterProjectionField::InfiniScope,
            AdapterProjectionField::InfiniScore,
        ],
        (AdapterProjectionKind::KvCache, _) => &[
            AdapterProjectionField::KvCacheEntryId,
            AdapterProjectionField::KvCacheVector,
            AdapterProjectionField::KvCacheStrength,
        ],
        (AdapterProjectionKind::TieredCache, _) => &[
            AdapterProjectionField::TierPlacementId,
            AdapterProjectionField::TierPlacementBytes,
            AdapterProjectionField::TierPlacementPriority,
        ],
    }
}

fn recommended_projection_fields(
    kind: AdapterProjectionKind,
    target: AdapterProjectionTarget,
) -> &'static [AdapterProjectionField] {
    match (kind, target) {
        (AdapterProjectionKind::ExperienceStore, AdapterProjectionTarget::ShadowRead) => &[
            AdapterProjectionField::ExperienceCleanGist,
            AdapterProjectionField::ExperienceProjectionTags,
            AdapterProjectionField::ExperienceTaskScope,
        ],
        (AdapterProjectionKind::ExperienceStore, AdapterProjectionTarget::IsolatedWrite) => &[
            AdapterProjectionField::ExperienceCleanGist,
            AdapterProjectionField::ExperienceSessionScope,
            AdapterProjectionField::ExperienceAgentScope,
        ],
        (AdapterProjectionKind::DiskKvStore, AdapterProjectionTarget::ShadowRead) => &[
            AdapterProjectionField::KvShardTier,
            AdapterProjectionField::KvShardPriority,
            AdapterProjectionField::KvShardLastAccess,
        ],
        (AdapterProjectionKind::DiskKvStore, AdapterProjectionTarget::IsolatedWrite) => &[],
        (AdapterProjectionKind::GistMemory, _) => &[AdapterProjectionField::GistImportance],
        (AdapterProjectionKind::InfiniMemory, _) => &[AdapterProjectionField::InfiniTokenEstimate],
        (AdapterProjectionKind::KvCache, _) => &[
            AdapterProjectionField::KvCacheLastAccess,
            AdapterProjectionField::KvCacheNamespace,
        ],
        (AdapterProjectionKind::ServiceMemory, _) | (AdapterProjectionKind::TieredCache, _) => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ColdKvShard, DiskKvOffload, InMemoryDiskKvOffload, KvTier, MemoryAdapterCapability,
        MemoryAdapterDescriptor, MemoryAdapterHealth, MemoryIndexOperationKind, MemoryPlacement,
        MemoryTier,
    };

    #[derive(Debug, Clone)]
    struct FakeExperienceAdapter {
        records: Vec<ExperienceEnvelope>,
    }

    impl MemoryAdapter for FakeExperienceAdapter {
        fn descriptor(&self) -> MemoryAdapterDescriptor {
            MemoryAdapterDescriptor::new(
                "fake_experience",
                vec![MemoryAdapterCapability::ExperienceGovernance],
            )
            .read_only()
        }

        fn health(&self) -> MemoryResult<MemoryAdapterHealth> {
            Ok(MemoryAdapterHealth::ready(Some(self.records.len())))
        }
    }

    impl ExperienceSnapshotAdapter for FakeExperienceAdapter {
        fn snapshot(&self) -> MemoryResult<Vec<ExperienceEnvelope>> {
            Ok(self.records.clone())
        }
    }

    #[derive(Debug, Clone)]
    struct WarningExperienceAdapter {
        records: Vec<ExperienceEnvelope>,
    }

    impl MemoryAdapter for WarningExperienceAdapter {
        fn descriptor(&self) -> MemoryAdapterDescriptor {
            MemoryAdapterDescriptor::new(
                "warning_experience",
                vec![MemoryAdapterCapability::ExperienceGovernance],
            )
            .read_only()
        }

        fn health(&self) -> MemoryResult<MemoryAdapterHealth> {
            Ok(MemoryAdapterHealth {
                ready: false,
                record_count: Some(self.records.len()),
                warnings: vec!["store_lag=2".to_owned()],
            })
        }
    }

    impl ExperienceSnapshotAdapter for WarningExperienceAdapter {
        fn snapshot(&self) -> MemoryResult<Vec<ExperienceEnvelope>> {
            Ok(self.records.clone())
        }
    }

    fn assert_has_code(codes: &[String], expected: &str) {
        assert!(
            codes.iter().any(|code| code == expected),
            "missing code {expected}; actual={codes:?}"
        );
    }

    #[test]
    fn adapter_snapshot_summary_exposes_status_and_warning_codes() {
        let clean = AdapterSnapshotSummary::read_only("experience_shadow", 2, 1);

        assert_eq!(clean.total_records(), 3);
        assert_eq!(clean.warning_codes(), Vec::<String>::new());
        assert_eq!(clean.detail_codes(), Vec::<String>::new());
        assert_eq!(clean.status_codes(), vec!["read_only".to_owned()]);
        assert_eq!(
            clean.summary_line(),
            "adapter_snapshot adapter=experience_shadow write_mode=read_only experiences=2 kv_shards=1 total_records=3 warnings=0 status_codes=read_only warning_codes=none detail_codes=none"
        );

        let empty = AdapterSnapshotSummary::read_only("empty_shadow", 0, 0);
        assert_eq!(
            empty.status_codes(),
            vec!["empty_snapshot".to_owned(), "read_only".to_owned()]
        );

        let live = AdapterSnapshotSummary {
            adapter_name: "live_experience".to_owned(),
            write_mode: AdapterWriteMode::LiveWrite,
            experience_count: 1,
            kv_shard_count: 0,
            warnings: vec![
                "missing_scope=2".to_owned(),
                "dirty_clean_gist:legacy".to_owned(),
            ],
        };

        assert_eq!(
            live.warning_codes(),
            vec!["dirty_clean_gist".to_owned(), "missing_scope".to_owned()]
        );
        assert_eq!(
            live.detail_codes(),
            vec![
                "warning:dirty_clean_gist".to_owned(),
                "warning:missing_scope".to_owned()
            ]
        );
        assert_eq!(
            live.status_codes(),
            vec!["live_write".to_owned(), "snapshot_warnings".to_owned()]
        );
        assert_eq!(
            live.summary_line(),
            "adapter_snapshot adapter=live_experience write_mode=live_write experiences=1 kv_shards=0 total_records=1 warnings=2 status_codes=live_write|snapshot_warnings warning_codes=dirty_clean_gist|missing_scope detail_codes=warning:dirty_clean_gist|warning:missing_scope"
        );
    }

    #[test]
    fn adapter_snapshot_summary_carries_adapter_health_warnings() {
        let adapter = WarningExperienceAdapter {
            records: vec![ExperienceEnvelope::new("lagging", "prompt", "lesson")],
        };
        let summary = adapter.snapshot_summary().unwrap();

        assert_eq!(
            summary.warnings,
            vec!["adapter_unhealthy".to_owned(), "store_lag=2".to_owned()]
        );
        assert_eq!(
            summary.warning_codes(),
            vec!["adapter_unhealthy".to_owned(), "store_lag".to_owned()]
        );
        assert_eq!(
            summary.detail_codes(),
            vec![
                "warning:adapter_unhealthy".to_owned(),
                "warning:store_lag".to_owned()
            ]
        );
        assert_eq!(
            summary.status_codes(),
            vec!["read_only".to_owned(), "snapshot_warnings".to_owned()]
        );
        assert_eq!(
            summary.summary_line(),
            "adapter_snapshot adapter=warning_experience write_mode=read_only experiences=1 kv_shards=0 total_records=1 warnings=2 status_codes=read_only|snapshot_warnings warning_codes=adapter_unhealthy|store_lag detail_codes=warning:adapter_unhealthy|warning:store_lag"
        );
    }

    #[test]
    fn experience_snapshot_filters_by_scope_and_projects_index_documents() {
        let adapter = FakeExperienceAdapter {
            records: vec![
                ExperienceEnvelope::new("runtime", "prompt", "lesson")
                    .with_scope(MemoryScope::for_task("runtime")),
                ExperienceEnvelope::new("global", "prompt", "lesson"),
                ExperienceEnvelope::new("ops", "prompt", "lesson")
                    .with_scope(MemoryScope::for_task("ops")),
            ],
        };

        let scoped = adapter
            .snapshot_for_scope(&MemoryScope::for_task("runtime"))
            .unwrap();
        let ids = scoped
            .iter()
            .map(|item| item.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(ids, vec!["runtime", "global"]);
        assert_eq!(adapter.index_documents().unwrap().len(), 3);
        assert_eq!(
            adapter.snapshot_summary().unwrap().summary_line(),
            "adapter_snapshot adapter=fake_experience write_mode=read_only experiences=3 kv_shards=0 total_records=3 warnings=0 status_codes=read_only warning_codes=none detail_codes=none"
        );
    }

    #[test]
    fn kv_catalog_projects_backend_metadata_to_placement_candidates() {
        let mut backend = InMemoryDiskKvOffload::new();
        backend
            .write_cold_shard(ColdKvShard {
                metadata: KvShardMetadata {
                    id: "cold".to_owned(),
                    byte_len: 0,
                    checksum: 0,
                    tier: KvTier::Hot,
                    priority: 0.7,
                    last_access: 9,
                },
                bytes: vec![1, 2, 3],
            })
            .unwrap();

        let candidates = backend.placement_candidates().unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].id, "cold");
        assert_eq!(candidates[0].current_tier, Some(MemoryTier::ColdDisk));
        assert_eq!(candidates[0].byte_len, 3);
        assert_eq!(
            backend.catalog_summary().unwrap().summary_line(),
            "adapter_snapshot adapter=in_memory_disk_kv_offload write_mode=read_only experiences=0 kv_shards=1 total_records=1 warnings=0 status_codes=read_only warning_codes=none detail_codes=none"
        );
    }

    #[test]
    fn read_only_plan_combines_governance_index_placement_and_kvswap() {
        let experiences = vec![
            ExperienceEnvelope::new("keep", "runtime prompt", "runtime lesson")
                .with_scope(MemoryScope::for_task("runtime")),
            ExperienceEnvelope::new(
                "polluted",
                "Conversation Transcript:\nUser: ssh -o ConnectTimeout=1 gitlab.local\nAssistant: ok",
                "accepted_pattern quality=0.2 max_severity=critical",
            )
            .with_scope(MemoryScope::for_task("ops")),
        ];
        let kv_metadata = vec![
            KvShardMetadata {
                id: "promote".to_owned(),
                byte_len: 4,
                checksum: 1,
                tier: KvTier::Cold,
                priority: 0.95,
                last_access: 20,
            },
            KvShardMetadata {
                id: "demote".to_owned(),
                byte_len: 4,
                checksum: 2,
                tier: KvTier::Hot,
                priority: 0.1,
                last_access: 1,
            },
        ];
        let previous = TieredMemoryPlan::new(vec![
            MemoryPlacement {
                id: "promote".to_owned(),
                tier: MemoryTier::ColdDisk,
                score: 0.1,
                reason: "old".to_owned(),
            },
            MemoryPlacement {
                id: "demote".to_owned(),
                tier: MemoryTier::WarmRam,
                score: 0.8,
                reason: "old".to_owned(),
            },
        ]);

        let plan = ReadOnlyMemoryPlan::for_inputs(
            "fake_service",
            &experiences,
            &kv_metadata,
            Some(&MemoryScope::for_task("runtime")),
            TierBudgets::new(4, 0),
            Some(&previous),
            4,
        );

        assert_eq!(plan.summary.experience_count, 2);
        assert!(plan.requires_operator_review());
        assert!(
            plan.governance
                .noisy_records
                .iter()
                .any(|item| item.experience_id == "polluted")
        );
        assert!(
            plan.index
                .operations_by_kind(MemoryIndexOperationKind::Quarantine)
                .iter()
                .any(|operation| operation.source_id == "polluted")
        );
        assert!(!plan.repair.items.is_empty());
        assert_eq!(
            plan.semantic.skipped_ids_for_reason("cross_task_scope"),
            vec!["polluted".to_owned()]
        );
        assert!(plan.context.rejected_ids().is_empty());
        assert_eq!(plan.kvswap.prefetch.promote_ids, vec!["promote".to_owned()]);
        assert_eq!(plan.kvswap.evict.demote_ids, vec!["demote".to_owned()]);
        let reason_codes = plan.reason_codes();
        assert!(
            reason_codes
                .iter()
                .any(|code| code == "semantic:cross_task_scope")
        );
        assert!(
            reason_codes
                .iter()
                .any(|code| code == "repair:governance_quarantine_candidate")
        );
        assert!(
            reason_codes
                .iter()
                .any(|code| code == "kvswap:prefetch_promote")
        );
        let detail_codes = plan.detail_codes();
        assert!(detail_codes.iter().any(|code| {
            code == "governance:context_rot:polluted:cross_task_transcript_pollution"
        }));
        assert!(
            detail_codes.iter().any(|code| {
                code == "semantic:skip:experience:cross_task_scope:706f6c6c75746564"
            })
        );
        assert!(detail_codes.iter().any(|code| {
            code == "repair:quarantine:governance_quarantine_candidate:706f6c6c75746564"
        }));
        assert!(detail_codes.iter().any(|code| {
            code == "index:quarantine:governance_quarantine_candidate:706f6c6c75746564"
        }));
        assert!(detail_codes.iter().any(|code| {
            code == "kvswap:prefetch:promote:tiered_memory_promotions:70726f6d6f7465"
        }));
        assert!(
            detail_codes.iter().any(|code| {
                code == "kvswap:eviction:demote:tiered_memory_demotions:64656d6f7465"
            })
        );
        let summary = plan.summary_line();
        assert!(summary.contains("memory_read_only_plan adapter=fake_service"));
        assert!(summary.contains("review=true"));
        assert!(summary.contains("experiences=2 kv_shards=2"));
        assert!(summary.contains("semantic_skipped=1"));
        assert!(summary.contains("context_reject=0"));
        assert!(summary.contains("kvswap_empty=false"));
        assert!(summary.contains("reason_codes="));
        assert!(summary.contains("semantic:cross_task_scope"));
        assert!(summary.contains("detail_codes="));
        assert!(summary.contains("semantic:skip:experience:cross_task_scope:706f6c6c75746564"));
        assert!(
            summary.contains("governance:context_rot:polluted:cross_task_transcript_pollution")
        );
        assert!(
            summary.contains("kvswap:prefetch:promote:tiered_memory_promotions:70726f6d6f7465")
        );

        let readiness = MigrationReadinessReport::from_plan(&plan);
        assert!(readiness.ready_for_isolated_write);
        assert!(readiness.operator_review_required);
        assert_eq!(
            readiness.summary_line(),
            "memory_migration_readiness isolated_write_ready=true review=true blockers=0 warnings=6 blocker_codes=none warning_codes=governance_rebuild_required|kvswap_intent_pending|quality_gate_blockers|quality_gate_warnings|repair_items|repair_skipped blocker_detail_codes=none warning_detail_codes=governance_rebuild_required|kvswap_intent_pending|quality_gate_blockers=1|quality_gate_warnings=2|repair_items=1|repair_skipped=1 blocker_details=none warning_details=governance_rebuild_required|kvswap_intent_pending|quality_gate_blockers=1|quality_gate_warnings=2|repair_items=1|repair_skipped=1"
        );
        assert_eq!(readiness.blocker_details(), Vec::<String>::new());
        assert_eq!(readiness.blocker_detail_codes(), Vec::<String>::new());
        assert_eq!(
            readiness.warning_details(),
            vec![
                "governance_rebuild_required".to_owned(),
                "kvswap_intent_pending".to_owned(),
                "quality_gate_blockers=1".to_owned(),
                "quality_gate_warnings=2".to_owned(),
                "repair_items=1".to_owned(),
                "repair_skipped=1".to_owned()
            ]
        );
        assert_eq!(
            readiness.warning_detail_codes(),
            vec![
                "governance_rebuild_required".to_owned(),
                "kvswap_intent_pending".to_owned(),
                "quality_gate_blockers=1".to_owned(),
                "quality_gate_warnings=2".to_owned(),
                "repair_items=1".to_owned(),
                "repair_skipped=1".to_owned()
            ]
        );
        assert_eq!(
            readiness.warning_codes(),
            vec![
                "governance_rebuild_required".to_owned(),
                "kvswap_intent_pending".to_owned(),
                "quality_gate_blockers".to_owned(),
                "quality_gate_warnings".to_owned(),
                "repair_items".to_owned(),
                "repair_skipped".to_owned()
            ]
        );
        assert!(
            readiness
                .warnings
                .iter()
                .any(|warning| warning == "governance_rebuild_required")
        );
    }

    #[test]
    fn read_only_plan_context_reuses_index_projection_quality_metadata() {
        let experiences = vec![
            ExperienceEnvelope::new(
                "raw",
                format!("runtime prompt {}", "p".repeat(2_000)),
                format!("runtime lesson {}", "l".repeat(2_000)),
            )
            .with_quality(0.9)
            .with_scope(MemoryScope::for_task("runtime")),
        ];

        let plan = ReadOnlyMemoryPlan::for_inputs(
            "fake_service",
            &experiences,
            &[],
            Some(&MemoryScope::for_task("runtime")),
            TierBudgets::new(0, 0),
            None,
            0,
        );

        assert_eq!(plan.context.accepted_ids(), Vec::<String>::new());
        assert_eq!(plan.context.rejected_ids(), vec!["raw".to_owned()]);
        assert!(
            plan.context
                .reason_codes()
                .contains(&"missing_clean_gist".to_owned())
        );
        assert!(
            plan.context
                .reason_codes()
                .contains(&"raw_fallback_index_content".to_owned())
        );
        assert!(
            plan.context
                .reason_codes()
                .contains(&"truncated_index_content".to_owned())
        );
        assert!(
            plan.detail_codes()
                .contains(&"context:reject_risk:missing_clean_gist:726177".to_owned())
        );
        assert!(
            plan.detail_codes()
                .contains(&"context:reject_risk:raw_fallback_index_content:726177".to_owned())
        );
        assert!(
            plan.detail_codes()
                .contains(&"context:reject_risk:truncated_index_content:726177".to_owned())
        );
    }

    #[test]
    fn adapter_facing_index_report_parity_preserves_rebuild_and_context_codes() {
        let clean_gist = "clean operational lesson keeps concise adapter parity evidence";
        let duplicate_prompt = "Normalized prompt for duplicate fixture";
        let duplicate_lesson = "Normalized lesson for duplicate fixture";
        let raw_prompt = "runtime prompt observed during copied fixture parity";
        let raw_lesson = "accepted_pattern quality=0.9 max_severity=low detail=raw fallback";
        let dirty_prompt = "DIRTY_PROMPT_SECRET_DO_NOT_LOG";
        let dirty_lesson = "DIRTY_LESSON_SECRET_DO_NOT_LOG";
        let dirty_gist =
            "DIRTY_GIST_SECRET_DO_NOT_LOG Conversation Transcript:\nUser: raw\nAssistant: raw";
        let noisy_prompt = "NOISY_PROMPT_SECRET_DO_NOT_LOG Conversation Transcript:\nUser: ssh -o ConnectTimeout=1 gitlab.local\nAssistant: ok";
        let noisy_lesson =
            "NOISY_LESSON_SECRET_DO_NOT_LOG accepted_pattern quality=0.1 max_severity=critical";
        let experiences = vec![
            ExperienceEnvelope::new("dupe1", duplicate_prompt, duplicate_lesson)
                .with_clean_gist(clean_gist)
                .with_scope(MemoryScope::for_task("runtime")),
            ExperienceEnvelope::new(
                "dupe2",
                format!("  {duplicate_prompt}  "),
                format!(" {duplicate_lesson} "),
            )
            .with_clean_gist(clean_gist)
            .with_scope(MemoryScope::for_task("runtime")),
            ExperienceEnvelope::new("missing", raw_prompt, raw_lesson)
                .with_quality(0.92)
                .with_scope(MemoryScope::for_task("runtime")),
            ExperienceEnvelope::new("dirty", dirty_prompt, dirty_lesson)
                .with_clean_gist(dirty_gist)
                .with_quality(0.88)
                .with_scope(MemoryScope::for_task("runtime")),
            ExperienceEnvelope::new(
                "long",
                format!("runtime prompt {}", "p".repeat(2_000)),
                format!("runtime lesson {}", "l".repeat(2_000)),
            )
            .with_quality(0.86)
            .with_scope(MemoryScope::for_task("runtime")),
            ExperienceEnvelope::new("noisy", noisy_prompt, noisy_lesson)
                .with_quality(0.05)
                .with_scope(MemoryScope::for_task("runtime")),
        ];

        let plan = ReadOnlyMemoryPlan::for_inputs(
            "copied_experience_store",
            &experiences,
            &[],
            Some(&MemoryScope::for_task("runtime")),
            TierBudgets::new(0, 0),
            None,
            0,
        );

        let reason_codes = plan.reason_codes();
        assert_has_code(&reason_codes, "governance:missing_clean_gist");
        assert_has_code(&reason_codes, "governance:dirty_clean_gist");
        assert_has_code(&reason_codes, "rebuild:deduplicate_exact_fingerprints");
        assert_has_code(&reason_codes, "rebuild:repair_missing_or_dirty_clean_gist");
        assert_has_code(&reason_codes, "rebuild:compact_long_context_without_gist");
        assert_has_code(&reason_codes, "rebuild:quarantine_high_noise_records");
        assert_has_code(&reason_codes, "index:deduplicate_exact_fingerprints");
        assert_has_code(&reason_codes, "index:repair_missing_or_dirty_clean_gist");
        assert_has_code(&reason_codes, "context:missing_clean_gist");
        assert_has_code(&reason_codes, "context:dirty_clean_gist");

        let detail_codes = plan.detail_codes();
        assert_has_code(&detail_codes, "governance:duplicate:dupe1:dupe2");
        assert_has_code(&detail_codes, "governance:noise:missing:missing_clean_gist");
        assert_has_code(&detail_codes, "governance:noise:dirty:dirty_clean_gist");
        assert_has_code(&detail_codes, "rebuild:missing_clean_gist:missing");
        assert_has_code(&detail_codes, "rebuild:dirty_clean_gist:dirty");
        assert_has_code(
            &detail_codes,
            "index:delete_duplicate:deduplicate_exact_fingerprint:6475706532",
        );
        assert_has_code(
            &detail_codes,
            "index:refresh_embedding:refresh_missing_clean_gist:6d697373696e67",
        );
        assert_has_code(
            &detail_codes,
            "index:refresh_embedding:refresh_dirty_clean_gist:6469727479",
        );
        assert_has_code(
            &detail_codes,
            "index:compact:compact_long_context_without_gist:6c6f6e67",
        );
        assert_has_code(
            &detail_codes,
            "index:quarantine:governance_quarantine_candidate:6e6f697379",
        );
        assert_has_code(
            &detail_codes,
            "context:reject_risk:missing_clean_gist:6d697373696e67",
        );
        assert_has_code(
            &detail_codes,
            "context:reject_risk:raw_fallback_index_content:6d697373696e67",
        );
        assert_has_code(
            &detail_codes,
            "context:reject_risk:dirty_clean_gist:6469727479",
        );

        let summary = plan.summary_line();
        assert!(summary.contains("adapter=copied_experience_store"));
        assert!(summary.contains("rebuild_required=true"));
        assert!(summary.contains("reason_codes="));
        assert!(summary.contains("index:repair_missing_or_dirty_clean_gist"));
        assert!(summary.contains("context:missing_clean_gist"));
        assert!(summary.contains("context:dirty_clean_gist"));
        assert!(summary.contains("detail_codes="));
        assert!(summary.contains("context:missing_clean_gist"));
        assert!(
            summary.contains("index:refresh_embedding:refresh_missing_clean_gist:6d697373696e67")
        );
        assert!(summary.contains("context:reject_risk:raw_fallback_index_content:6d697373696e67"));
        for forbidden in [
            clean_gist,
            duplicate_prompt,
            duplicate_lesson,
            raw_prompt,
            raw_lesson,
            dirty_prompt,
            dirty_lesson,
            dirty_gist,
            noisy_prompt,
            noisy_lesson,
        ] {
            assert!(
                !detail_codes.iter().any(|code| code.contains(forbidden)),
                "adapter-facing detail codes leaked root fixture payload: {forbidden}"
            );
            assert!(
                !summary.contains(forbidden),
                "adapter-facing summary leaked root fixture payload: {forbidden}"
            );
        }
        assert!(!plan.index.summary_line().contains(raw_prompt));
        assert!(!plan.index.summary_line().contains(raw_lesson));
        assert!(!plan.context.summary_line().contains(raw_prompt));
        assert!(!plan.context.summary_line().contains(raw_lesson));
    }

    #[test]
    fn projection_hints_attach_stable_tags_and_task_scope() {
        let hints = ExperienceProjectionHints::new("experience_store")
            .with_task_profile("coding")
            .with_runtime_model("gemma")
            .with_device_profile("cpu")
            .with_memory_mode("agentic");
        let envelope = hints.apply_to(ExperienceEnvelope::new("id", "prompt", "lesson"));

        assert_eq!(envelope.scope.task_id.as_deref(), Some("coding"));
        assert!(
            envelope
                .tags
                .contains(&"adapter:experience_store".to_owned())
        );
        assert!(envelope.tags.contains(&"runtime_model:gemma".to_owned()));
        assert!(envelope.tags.contains(&"device_profile:cpu".to_owned()));
        assert!(envelope.tags.contains(&"memory_mode:agentic".to_owned()));
    }

    #[test]
    fn experience_projection_contract_allows_read_only_shadow_with_review_warnings() {
        let contract = AdapterProjectionContract::experience_store_read_only(
            "root_experience_store",
            vec![
                AdapterProjectionField::ExperienceQuality,
                AdapterProjectionField::ExperienceLesson,
                AdapterProjectionField::ExperiencePrompt,
                AdapterProjectionField::ExperienceId,
            ],
        )
        .with_note("legacy_gist_projection_pending");

        let report = contract.coverage_report(AdapterProjectionTarget::ShadowRead);

        assert!(report.ready);
        assert!(report.requires_operator_review());
        assert!(report.missing_required_fields.is_empty());
        assert_eq!(
            report.missing_recommended_fields,
            vec![
                AdapterProjectionField::ExperienceCleanGist,
                AdapterProjectionField::ExperienceProjectionTags,
                AdapterProjectionField::ExperienceTaskScope,
            ]
        );
        assert!(
            report
                .summary_line()
                .contains("adapter_projection adapter=root_experience_store")
        );
    }

    #[test]
    fn projection_contract_exposes_expected_field_codes_for_root_adapters() {
        let experience = AdapterProjectionContract::experience_store_shadow("experience_shadow");
        let disk = AdapterProjectionContract::disk_kv_copied_fixture("disk_fixture");

        assert_eq!(
            experience.mapped_field_codes(),
            vec![
                "experience_id",
                "experience_prompt",
                "experience_lesson",
                "experience_quality",
                "experience_clean_gist",
                "experience_projection_tags",
                "experience_task_scope",
            ]
        );
        assert_eq!(
            experience.required_field_codes_for(AdapterProjectionTarget::ShadowRead),
            vec![
                "experience_id",
                "experience_prompt",
                "experience_lesson",
                "experience_quality",
            ]
        );
        assert_eq!(
            experience.recommended_field_codes_for(AdapterProjectionTarget::ShadowRead),
            vec![
                "experience_clean_gist",
                "experience_projection_tags",
                "experience_task_scope",
            ]
        );
        assert_eq!(
            disk.required_fields_for(AdapterProjectionTarget::IsolatedWrite),
            vec![
                AdapterProjectionField::KvShardId,
                AdapterProjectionField::KvShardBytes,
                AdapterProjectionField::KvShardMetadata,
                AdapterProjectionField::KvShardChecksum,
                AdapterProjectionField::KvShardTier,
                AdapterProjectionField::KvShardPriority,
                AdapterProjectionField::KvShardLastAccess,
                AdapterProjectionField::KvDeleteTombstone,
                AdapterProjectionField::KvCompactionIsolation,
            ]
        );
        assert!(
            disk.recommended_fields_for(AdapterProjectionTarget::IsolatedWrite)
                .is_empty()
        );
    }

    #[test]
    fn projection_contract_manifest_line_explains_root_wiring_without_note_text() {
        let contract = AdapterProjectionContract::disk_kv_store_shadow("disk_shadow")
            .with_note("production path must stay read-only");

        let line = contract.manifest_line(AdapterProjectionTarget::ShadowRead);

        assert_eq!(
            line,
            "adapter_projection_contract adapter=disk_shadow kind=disk_kv_store target=shadow_read write_mode=read_only mapped_fields=kv_shard_id|kv_shard_bytes|kv_shard_metadata|kv_shard_checksum|kv_shard_tier|kv_shard_priority|kv_shard_last_access required_fields=kv_shard_id|kv_shard_bytes|kv_shard_metadata|kv_shard_checksum recommended_fields=kv_shard_tier|kv_shard_priority|kv_shard_last_access notes=1"
        );
        assert!(!line.contains("production path"));
    }

    #[test]
    fn experience_projection_contract_blocks_isolated_write_until_scope_tags_and_mode_exist() {
        let contract = AdapterProjectionContract::experience_store_read_only(
            "root_experience_store",
            vec![
                AdapterProjectionField::ExperienceId,
                AdapterProjectionField::ExperiencePrompt,
                AdapterProjectionField::ExperienceLesson,
                AdapterProjectionField::ExperienceQuality,
            ],
        );

        let report = contract.coverage_report(AdapterProjectionTarget::IsolatedWrite);

        assert!(!report.ready);
        assert_eq!(
            report.missing_required_codes(),
            vec!["experience_projection_tags", "experience_task_scope"]
        );
        assert_eq!(
            report.missing_recommended_codes(),
            vec![
                "experience_clean_gist",
                "experience_session_scope",
                "experience_agent_scope"
            ]
        );
        assert!(
            report
                .blockers
                .contains(&"write_mode_not_isolated:read_only".to_owned())
        );
    }

    #[test]
    fn disk_kv_projection_contract_requires_verified_fixture_fields_for_isolated_write() {
        let contract = AdapterProjectionContract::disk_kv_store_read_only(
            "copied_disk_kv",
            vec![
                AdapterProjectionField::KvShardId,
                AdapterProjectionField::KvShardBytes,
                AdapterProjectionField::KvShardMetadata,
                AdapterProjectionField::KvShardChecksum,
                AdapterProjectionField::KvShardTier,
            ],
        )
        .with_write_mode(AdapterWriteMode::IsolatedWrite);

        let report = contract.coverage_report(AdapterProjectionTarget::IsolatedWrite);

        assert!(!report.ready);
        assert_eq!(
            report.missing_required_codes(),
            vec![
                "kv_shard_priority",
                "kv_shard_last_access",
                "kv_delete_tombstone",
                "kv_compaction_isolation",
            ]
        );
        assert!(
            report
                .blockers
                .iter()
                .any(|blocker| blocker == "missing_required:kv_compaction_isolation")
        );
    }

    #[test]
    fn disk_kv_projection_contract_passes_complete_copied_fixture() {
        let contract = AdapterProjectionContract::disk_kv_store_read_only(
            "copied_disk_kv",
            vec![
                AdapterProjectionField::KvShardId,
                AdapterProjectionField::KvShardBytes,
                AdapterProjectionField::KvShardMetadata,
                AdapterProjectionField::KvShardChecksum,
                AdapterProjectionField::KvShardTier,
                AdapterProjectionField::KvShardPriority,
                AdapterProjectionField::KvShardLastAccess,
                AdapterProjectionField::KvDeleteTombstone,
                AdapterProjectionField::KvCompactionIsolation,
            ],
        )
        .with_write_mode(AdapterWriteMode::IsolatedWrite);

        let report = contract.coverage_report(AdapterProjectionTarget::IsolatedWrite);

        assert!(report.ready);
        assert!(!report.requires_operator_review());
        assert_eq!(report.blocker_codes(), Vec::<String>::new());
        assert_eq!(report.warning_codes(), Vec::<String>::new());
        assert_eq!(report.blocker_details(), Vec::<String>::new());
        assert_eq!(report.warning_details(), Vec::<String>::new());
        assert_eq!(
            report.summary_line(),
            "adapter_projection adapter=copied_disk_kv kind=disk_kv_store target=isolated_write ready=true write_mode=isolated_write missing_required=0 missing_recommended=0 blockers=none warnings=none blocker_codes=none warning_codes=none blocker_detail_codes=none warning_detail_codes=none"
        );
    }

    #[test]
    fn projection_contract_presets_cover_common_root_adapter_targets() {
        let experience_shadow =
            AdapterProjectionContract::experience_store_shadow("experience_shadow");
        let experience_isolated =
            AdapterProjectionContract::experience_store_isolated_write("experience_fixture");
        let disk_shadow = AdapterProjectionContract::disk_kv_store_shadow("disk_shadow");
        let disk_fixture = AdapterProjectionContract::disk_kv_copied_fixture("disk_fixture");
        let gist_shadow = AdapterProjectionContract::gist_memory_shadow("gist_shadow");
        let infini_shadow = AdapterProjectionContract::infini_memory_shadow("infini_shadow");
        let kv_cache_shadow = AdapterProjectionContract::kv_cache_shadow("kv_cache_shadow");
        let tiered_shadow = AdapterProjectionContract::tiered_cache_shadow("tiered_shadow");
        let service_shadow = AdapterProjectionContract::service_memory_shadow("service_shadow");

        for contract in [
            &experience_shadow,
            &disk_shadow,
            &gist_shadow,
            &infini_shadow,
            &kv_cache_shadow,
            &tiered_shadow,
            &service_shadow,
        ] {
            let report = contract.coverage_report(AdapterProjectionTarget::ShadowRead);
            assert!(report.ready, "{}", report.summary_line());
            assert!(report.warnings.is_empty(), "{}", report.summary_line());
        }

        for contract in [&experience_isolated, &disk_fixture] {
            let report = contract.coverage_report(AdapterProjectionTarget::IsolatedWrite);
            assert!(report.ready, "{}", report.summary_line());
            assert!(report.warnings.is_empty(), "{}", report.summary_line());
        }

        let blocked = experience_shadow.coverage_report(AdapterProjectionTarget::IsolatedWrite);
        assert!(!blocked.ready);
        assert!(
            blocked
                .blockers
                .contains(&"write_mode_not_isolated:read_only".to_owned())
        );
    }

    #[test]
    fn projection_contract_bundles_cover_shadow_and_fixture_targets() {
        let shadow_bundle = AdapterProjectionContractBundle::standard_shadow();

        assert_eq!(shadow_bundle.name, "standard_shadow");
        assert_eq!(shadow_bundle.target, AdapterProjectionTarget::ShadowRead);
        assert!(!shadow_bundle.is_empty());
        assert_eq!(shadow_bundle.contracts.len(), 7);
        let shadow_reports = shadow_bundle.coverage_reports();
        assert_eq!(shadow_reports.len(), 7);
        assert!(shadow_reports.iter().all(|report| report.ready));
        assert!(
            shadow_reports
                .iter()
                .all(|report| report.target == AdapterProjectionTarget::ShadowRead)
        );
        assert!(
            shadow_reports
                .iter()
                .all(|report| report.warnings.is_empty())
        );
        assert!(
            shadow_reports
                .iter()
                .any(|report| report.adapter_name == "experience_store")
        );
        assert!(
            shadow_reports
                .iter()
                .any(|report| report.adapter_name == "disk_kv_store")
        );
        assert!(
            shadow_reports
                .iter()
                .any(|report| report.adapter_name == "gist_memory")
        );
        assert!(
            shadow_reports
                .iter()
                .any(|report| report.adapter_name == "infini_memory")
        );
        assert!(
            shadow_reports
                .iter()
                .any(|report| report.adapter_name == "kv_cache")
        );
        assert!(
            shadow_reports
                .iter()
                .any(|report| report.adapter_name == "tiered_cache")
        );
        assert!(
            shadow_reports
                .iter()
                .any(|report| report.adapter_name == "service_memory")
        );
        let shadow_summary = shadow_bundle.coverage_summary();
        assert!(shadow_summary.ready);
        assert!(!shadow_summary.requires_operator_review);
        assert_eq!(shadow_summary.contract_count, 7);
        assert_eq!(shadow_summary.ready_contract_count, 7);
        assert_eq!(shadow_summary.blocker_count, 0);
        assert_eq!(shadow_summary.warning_count, 0);
        assert_eq!(shadow_summary.blocker_codes(), Vec::<String>::new());
        assert_eq!(shadow_summary.warning_codes(), Vec::<String>::new());
        assert_eq!(
            shadow_summary.summary_line(),
            "adapter_projection_bundle name=standard_shadow target=shadow_read ready=true review=false contracts=7 ready_contracts=7 blockers=0 warnings=0 blocker_codes=none warning_codes=none blocker_detail_codes=none warning_detail_codes=none"
        );
        assert_eq!(shadow_bundle.manifest_lines().len(), 7);
        assert!(
            shadow_bundle
                .manifest_lines()
                .iter()
                .any(|line| line.contains("adapter=experience_store kind=experience_store"))
        );
        assert_eq!(
            shadow_bundle.manifest_summary_line(),
            "adapter_projection_contract_bundle_manifest name=standard_shadow target=shadow_read contracts=7 adapters=disk_kv_store|experience_store|gist_memory|infini_memory|kv_cache|service_memory|tiered_cache mapped_fields=32 required_fields=22 recommended_fields=10 notes=0"
        );

        let fixture_bundle = AdapterProjectionContractBundle::copied_fixture_isolated_write();

        assert_eq!(fixture_bundle.name, "copied_fixture_isolated_write");
        assert_eq!(
            fixture_bundle.target,
            AdapterProjectionTarget::IsolatedWrite
        );
        assert_eq!(fixture_bundle.contracts.len(), 2);
        let fixture_reports = fixture_bundle.coverage_reports();
        assert_eq!(fixture_reports.len(), 2);
        assert!(fixture_reports.iter().all(|report| report.ready));
        assert!(
            fixture_reports
                .iter()
                .all(|report| report.target == AdapterProjectionTarget::IsolatedWrite)
        );
        assert!(
            fixture_reports
                .iter()
                .all(|report| report.warnings.is_empty())
        );
        assert!(
            fixture_reports
                .iter()
                .any(|report| report.adapter_name == "experience_store_fixture")
        );
        assert!(
            fixture_reports
                .iter()
                .any(|report| report.adapter_name == "disk_kv_fixture")
        );
        let fixture_summary = fixture_bundle.coverage_summary();
        assert!(fixture_summary.ready);
        assert!(!fixture_summary.requires_operator_review);
        assert_eq!(fixture_summary.contract_count, 2);
        assert_eq!(fixture_bundle.manifest_lines().len(), 2);
        assert_eq!(
            fixture_bundle.manifest_summary_line(),
            "adapter_projection_contract_bundle_manifest name=copied_fixture_isolated_write target=isolated_write contracts=2 adapters=disk_kv_fixture|experience_store_fixture mapped_fields=18 required_fields=15 recommended_fields=3 notes=0"
        );

        let extended = AdapterProjectionContractBundle::new(
            "custom_shadow",
            AdapterProjectionTarget::ShadowRead,
            Vec::new(),
        )
        .with_contract(AdapterProjectionContract::service_memory_shadow(
            "service_shadow",
        ));
        assert!(!extended.is_empty());
        assert_eq!(extended.coverage_reports().len(), 1);
    }

    #[test]
    fn projection_contract_bundle_manifest_normalizes_adapter_codes() {
        let bundle = AdapterProjectionContractBundle::new(
            "custom_shadow",
            AdapterProjectionTarget::ShadowRead,
            vec![
                AdapterProjectionContract::service_memory_shadow("Service Memory"),
                AdapterProjectionContract::disk_kv_store_shadow("disk/kv:prod"),
            ],
        );

        assert_eq!(
            bundle.adapter_codes(),
            vec!["Service_Memory".to_owned(), "disk_kv_prod".to_owned()]
        );
        assert!(
            bundle
                .manifest_summary_line()
                .contains("adapters=Service_Memory|disk_kv_prod")
        );
    }

    #[test]
    fn projection_contract_bundle_summary_aggregates_review_risks() {
        let bundle = AdapterProjectionContractBundle::new(
            "partial_fixture",
            AdapterProjectionTarget::IsolatedWrite,
            vec![
                AdapterProjectionContract::experience_store_read_only(
                    "experience_partial",
                    vec![
                        AdapterProjectionField::ExperienceId,
                        AdapterProjectionField::ExperiencePrompt,
                        AdapterProjectionField::ExperienceLesson,
                        AdapterProjectionField::ExperienceQuality,
                    ],
                ),
                AdapterProjectionContract::disk_kv_store_shadow("disk_shadow_only"),
            ],
        );

        let summary = bundle.coverage_summary();

        assert!(!summary.ready);
        assert!(summary.requires_operator_review);
        assert_eq!(summary.contract_count, 2);
        assert_eq!(summary.ready_contract_count, 0);
        assert_eq!(summary.blocker_count, 6);
        assert_eq!(summary.warning_count, 3);
        assert_eq!(
            summary.blocker_codes(),
            vec![
                "missing_required".to_owned(),
                "write_mode_not_isolated".to_owned(),
            ]
        );
        assert_eq!(
            summary.warning_codes(),
            vec!["missing_recommended".to_owned()]
        );
        assert!(
            summary
                .blocker_details()
                .iter()
                .any(|detail| detail.starts_with("experience_partial:missing_required:"))
        );
        assert!(
            summary
                .blocker_detail_codes()
                .iter()
                .any(|detail| detail.starts_with("experience_partial:missing_required:"))
        );
        assert!(
            summary
                .blocker_details()
                .contains(&"experience_partial:write_mode_not_isolated:read_only".to_owned())
        );
        assert!(
            summary
                .warning_details()
                .iter()
                .any(|detail| detail.contains(":missing_recommended:"))
        );
        assert!(
            summary
                .warning_detail_codes()
                .iter()
                .any(|detail| detail.contains(":missing_recommended:"))
        );
        assert!(
            summary
                .reports
                .iter()
                .any(|report| report.adapter_name == "experience_partial")
        );
        let summary_line = summary.summary_line();
        assert!(summary_line.contains(
            "adapter_projection_bundle name=partial_fixture target=isolated_write ready=false review=true contracts=2 ready_contracts=0 blockers=6 warnings=3"
        ));
        assert!(summary_line.contains("blocker_codes=missing_required|write_mode_not_isolated"));
        assert!(summary_line.contains("warning_codes=missing_recommended"));
        assert!(summary_line.contains("blocker_detail_codes="));
        assert!(summary_line.contains("disk_shadow_only:missing_required:kv_delete_tombstone"));
        assert!(summary_line.contains("experience_partial:write_mode_not_isolated:read_only"));
        assert!(summary_line.contains("warning_detail_codes="));
        assert!(
            summary_line.contains("experience_partial:missing_recommended:experience_clean_gist")
        );
    }

    #[test]
    fn projection_contracts_cover_gist_infini_and_kv_cache_requirements() {
        let gist = AdapterProjectionContract::new(
            "gist_partial",
            AdapterProjectionKind::GistMemory,
            AdapterWriteMode::ReadOnly,
            vec![
                AdapterProjectionField::GistId,
                AdapterProjectionField::GistText,
            ],
        );
        let infini = AdapterProjectionContract::new(
            "infini_partial",
            AdapterProjectionKind::InfiniMemory,
            AdapterWriteMode::ReadOnly,
            vec![
                AdapterProjectionField::InfiniItemId,
                AdapterProjectionField::InfiniScope,
                AdapterProjectionField::InfiniScore,
            ],
        );
        let kv_cache = AdapterProjectionContract::new(
            "kv_cache_partial",
            AdapterProjectionKind::KvCache,
            AdapterWriteMode::ReadOnly,
            vec![
                AdapterProjectionField::KvCacheEntryId,
                AdapterProjectionField::KvCacheVector,
                AdapterProjectionField::KvCacheStrength,
            ],
        );

        let gist_report = gist.coverage_report(AdapterProjectionTarget::ShadowRead);
        assert!(!gist_report.ready);
        assert_eq!(
            gist_report.blocker_codes(),
            vec!["missing_required".to_owned()]
        );
        assert!(
            gist_report
                .blocker_details()
                .contains(&"missing_required:gist_source_experience_id".to_owned())
        );

        let infini_report = infini.coverage_report(AdapterProjectionTarget::ShadowRead);
        assert!(infini_report.ready);
        assert_eq!(
            infini_report.warning_codes(),
            vec!["missing_recommended".to_owned()]
        );
        assert!(
            infini_report
                .warning_details()
                .contains(&"missing_recommended:infini_token_estimate".to_owned())
        );

        let kv_cache_report = kv_cache.coverage_report(AdapterProjectionTarget::ShadowRead);
        assert!(kv_cache_report.ready);
        assert_eq!(
            kv_cache_report.warning_codes(),
            vec!["missing_recommended".to_owned()]
        );
        assert!(
            kv_cache_report
                .warning_details()
                .contains(&"missing_recommended:kv_cache_namespace".to_owned())
        );
    }

    #[test]
    fn projection_audit_blocks_duplicate_ids_and_empty_content() {
        let records = vec![
            ExperienceEnvelope::new("dup", "", "")
                .with_scope(MemoryScope::for_task("task"))
                .with_tags(vec!["adapter:test".to_owned()]),
            ExperienceEnvelope::new("dup", "prompt", "lesson")
                .with_scope(MemoryScope::for_task("task"))
                .with_tags(vec!["adapter:test".to_owned()]),
        ];

        let audit = DefaultAdapterProjectionAuditor::new().audit(&records, &[]);

        assert!(!audit.is_ready_for_shadow_read());
        assert!(
            audit
                .issue_codes()
                .contains(&"duplicate_experience_id".to_owned())
        );
        assert!(
            audit
                .issue_codes()
                .contains(&"empty_experience_content".to_owned())
        );
        assert_eq!(audit.blockers().len(), 2);
        assert_eq!(
            audit.shadow_read_checklist_detail(),
            "projection_blockers=2"
        );
        assert_eq!(
            audit.isolated_write_checklist_detail(),
            "projection_issues=2"
        );
        assert!(
            audit
                .detail_codes()
                .contains(&"blocker:duplicate_experience_id:source_id_hex:647570".to_owned())
        );
        assert!(
            audit
                .detail_codes()
                .contains(&"blocker:empty_experience_content:source_id_hex:647570".to_owned())
        );
        assert_eq!(
            audit.summary_line(),
            "adapter_projection_audit shadow_ready=false isolated_write_ready=false experiences=2 kv_shards=0 issues=2 blockers=2 warnings=0 issue_codes=duplicate_experience_id|empty_experience_content detail_codes=blocker:duplicate_experience_id:source_id_hex:647570|blocker:empty_experience_content:source_id_hex:647570"
        );
    }

    #[test]
    fn projection_audit_warns_on_missing_scope_tags_and_clean_gist() {
        let record = ExperienceEnvelope::new(
            "risky",
            "Conversation Transcript:\nUser: run bash command\nAssistant: ok",
            "accepted_pattern quality=0.2 max_severity=critical",
        );

        let audit = DefaultAdapterProjectionAuditor::new().audit(&[record], &[]);

        assert!(audit.is_ready_for_shadow_read());
        assert!(!audit.is_ready_for_isolated_write());
        assert_eq!(
            audit.shadow_read_checklist_detail(),
            "projection_blockers=0"
        );
        assert_eq!(
            audit.isolated_write_checklist_detail(),
            "projection_issues=3"
        );
        assert!(
            audit
                .issue_codes()
                .contains(&"missing_task_scope".to_owned())
        );
        assert!(
            audit
                .issue_codes()
                .contains(&"missing_projection_tags".to_owned())
        );
        assert!(
            audit
                .issue_codes()
                .contains(&"missing_clean_gist_for_risky_record".to_owned())
        );
        assert!(
            audit
                .detail_codes()
                .contains(&"warning:missing_task_scope:source_id_hex:7269736b79".to_owned())
        );
        assert!(
            audit
                .detail_codes()
                .contains(&"warning:missing_projection_tags:source_id_hex:7269736b79".to_owned())
        );
        assert!(audit.detail_codes().contains(
            &"warning:missing_clean_gist_for_risky_record:source_id_hex:7269736b79".to_owned()
        ));
        assert_eq!(
            audit.summary_line(),
            "adapter_projection_audit shadow_ready=true isolated_write_ready=false experiences=1 kv_shards=0 issues=3 blockers=0 warnings=3 issue_codes=missing_clean_gist_for_risky_record|missing_projection_tags|missing_task_scope detail_codes=warning:missing_clean_gist_for_risky_record:source_id_hex:7269736b79|warning:missing_projection_tags:source_id_hex:7269736b79|warning:missing_task_scope:source_id_hex:7269736b79"
        );
    }

    #[test]
    fn projection_audit_checks_kv_metadata_quality() {
        let metadata = vec![
            KvShardMetadata {
                id: "kv".to_owned(),
                byte_len: 4,
                checksum: 0,
                tier: KvTier::Cold,
                priority: 1.4,
                last_access: 1,
            },
            KvShardMetadata {
                id: "kv".to_owned(),
                byte_len: 0,
                checksum: 0,
                tier: KvTier::Cold,
                priority: f32::NAN,
                last_access: 1,
            },
        ];

        let audit = DefaultAdapterProjectionAuditor::new().audit(&[], &metadata);

        assert!(!audit.is_ready_for_shadow_read());
        assert!(
            audit
                .issue_codes()
                .contains(&"duplicate_kv_shard_id".to_owned())
        );
        assert!(
            audit
                .issue_codes()
                .contains(&"missing_kv_checksum".to_owned())
        );
        assert!(
            audit
                .issue_codes()
                .contains(&"kv_priority_out_of_range".to_owned())
        );
        assert!(
            audit
                .issue_codes()
                .contains(&"invalid_kv_priority".to_owned())
        );
        assert!(audit.issue_codes().contains(&"empty_kv_shard".to_owned()));
        assert!(
            audit
                .detail_codes()
                .contains(&"blocker:duplicate_kv_shard_id:source_id_hex:6b76".to_owned())
        );
        assert!(
            audit
                .detail_codes()
                .contains(&"blocker:invalid_kv_priority:source_id_hex:6b76".to_owned())
        );
        assert!(
            audit
                .detail_codes()
                .contains(&"warning:empty_kv_shard:source_id_hex:6b76".to_owned())
        );
        assert!(
            audit
                .summary_line()
                .contains("detail_codes=blocker:duplicate_kv_shard_id:source_id_hex:6b76")
        );
    }

    #[test]
    fn projection_auditor_reports_read_only_adapter_health() {
        let auditor = DefaultAdapterProjectionAuditor::new();
        let descriptor = auditor.descriptor();

        assert!(descriptor.read_only);
        assert!(
            descriptor
                .capabilities
                .contains(&MemoryAdapterCapability::ExperienceGovernance)
        );
        assert!(
            descriptor
                .capabilities
                .contains(&MemoryAdapterCapability::DiskKvOffload)
        );
        assert!(auditor.health().unwrap().ready);
    }
}
