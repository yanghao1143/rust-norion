//! Memory ports for Norion agents.
//!
//! This crate intentionally has no external storage dependency. The first
//! implementation layer is in-memory or local-file backed so callers can wire
//! it into tests and isolated service runs before choosing redb, sled, qdrant,
//! or another production backend.

pub mod adapters;
pub mod context;
pub mod disk_kv;
pub mod evolution;
pub mod gist;
pub mod governance;
pub mod index;
pub mod infini;
pub mod inspect;
pub mod long_term;
pub mod migration;
pub mod placement;
pub mod repair;
pub mod replay;
pub mod retention;
pub mod reuse;
pub mod runtime_projection;
pub mod self_evolving;
pub mod service;
pub mod short_term;
pub mod skills;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub use adapters::{
    AdapterProjectionAudit, AdapterProjectionAuditPolicy, AdapterProjectionBundleReport,
    AdapterProjectionContract, AdapterProjectionContractBundle, AdapterProjectionCoverageReport,
    AdapterProjectionField, AdapterProjectionIssue, AdapterProjectionIssueSeverity,
    AdapterProjectionKind, AdapterProjectionTarget, AdapterSnapshotSummary, AdapterWriteMode,
    DefaultAdapterProjectionAuditor, ExperienceProjectionHints, ExperienceSnapshotAdapter,
    KvShardCatalogAdapter, MigrationReadinessReport, ReadOnlyMemoryPlan,
};
pub use context::{
    ContextCandidate, ContextDecision, ContextDecisionKind, ContextInjectionGate,
    ContextInjectionPlan, ContextInjectionPolicy, DefaultContextInjectionGate,
};
pub use disk_kv::{
    ColdKvShard, DiskKvCatalogVerification, DiskKvOffload, DiskKvShardKeys, DiskKvShardKeyspace,
    DiskKvShardManifest, FileDiskKvOffload, InMemoryDiskKvOffload, KvEvictionPlan, KvPrefetchPlan,
    KvShardMetadata, KvSwap, KvSwapBoundaryAudit, KvSwapBoundaryReadiness, KvSwapManager,
    KvSwapStateSnapshot, KvTier, deserialize_kv_metadata, serialize_kv_metadata,
};
pub use evolution::{
    DefaultMemoryEvolutionGate, MemoryEvolutionAssessment, MemoryEvolutionGate,
    MemoryEvolutionLedger, MemoryEvolutionPolicy, MemoryHygieneActionLane, MemoryHygienePressure,
    MemoryHygieneWorkItem, MemoryHygieneWorkPlan, MemoryHygieneWorkQueue,
};
pub use gist::{
    CleanGistPolicy, CleanGistSelectionReport, CleanGistSelector, DefaultCleanGistSelector,
    GistLevel, MemoryGist,
};
pub use governance::{
    ContextRotRisk, DeduplicationReport, DefaultExperienceGovernance, DuplicateGroup,
    ExperienceEnvelope, ExperienceGovernance, ExperienceIndexQualityGate, GistStatus,
    GovernanceReport, IndexRebuildPlan, NoiseAssessment, SelfImproveAdmissionDecision,
    SelfImproveAdmissionWriteMode, SelfImproveLearningAdmissionPlan, SelfImproveLearningEvidence,
    SelfImproveLearningProposal, SelfImproveNextRoundDecision, SelfImproveProposalRepairState,
    SelfImproveProposalSource, SelfImproveRoundIdEvidence, admit_self_improve_learning_candidate,
};
pub use index::{
    DefaultMemoryIndexPlanner, DefaultMemorySemanticRetriever, ExperienceIndexFindingProjection,
    MemoryIndexDocument, MemoryIndexOperation, MemoryIndexOperationKind, MemoryIndexPlan,
    MemoryIndexPlanner, MemoryIndexSource, MemorySemanticMatch, MemorySemanticQuery,
    MemorySemanticRetrievalPlan, MemorySemanticRetriever, MemorySemanticSkip, memory_index_digest,
};
pub use infini::{
    DefaultInfiniMemoryPlanner, InfiniMemoryActiveMatch, InfiniMemoryCounts, InfiniMemoryItem,
    InfiniMemoryPlan, InfiniMemoryPlanner, InfiniMemoryScope,
};
pub use inspect::{
    DefaultMemoryInspectionBuilder, MemoryInspectionBuilder, MemoryInspectionRisk,
    MemoryInspectionRiskSeverity, MemoryInspectionSnapshot, MemoryInspectionSummary,
    MemoryVectorDimensions,
};
pub use long_term::{
    InMemoryLongTermMemory, LongTermMatch, LongTermMemory, LongTermQuery, MemoryDocument,
    MemoryDocumentInput,
};
pub use migration::{
    DefaultMemoryMigrationGate, MemoryMigrationApproval, MemoryMigrationApprovalPolicy,
    MemoryMigrationEvidence, MemoryMigrationPhase,
};
pub use placement::{
    DefaultTieredMemoryPlanner, KvSwapIntent, MemoryPlacement, MemoryPlacementCandidate,
    MemoryTier, TierBudgets, TierCounts, TierMigration, TierMigrationAction, TieredMemoryPlan,
    TieredMemoryPlanner,
};
pub use repair::{
    DefaultMemoryRepairPlanner, GenomeRepairFactor, GenomeRepairFactorKind, GenomeRepairFactorPlan,
    GenomeRepairSkippedFactor, MemoryRepairAction, MemoryRepairItem, MemoryRepairPlan,
    MemoryRepairPlanner, MemoryRepairSkippedItem,
};
pub use replay::{
    DefaultExperienceReplayPlanner, ExperienceReplayPlanner, ReplayAction, ReplayApplyReport,
    ReplayCandidate, ReplayFeedbackStats, ReplayItem, ReplayMemoryUpdate, ReplayPlan, ReplayReport,
    ReplaySignal, apply_replay_updates_to_long_term,
};
pub use retention::{
    DefaultMemoryRetentionPlanner, MemoryCompactionMerge, MemoryCompactionPlan,
    MemoryCompactionPlanner, MemoryCompactionPolicy, MemoryDecay, MemoryRetentionPlan,
    MemoryRetentionPlanner, MemoryRetentionPolicy, MemoryRetentionRemoval, RetentionMemoryEntry,
};
pub use reuse::{
    DefaultMemoryReusePlanner, MemoryReuseDryRunSummary, MemoryReusePlan, MemoryReusePlanner,
    MemoryReusePolicy,
};
pub use runtime_projection::{
    AdaptiveStateMemoryProjection, DefaultRuntimeStateProjector, MemoryProjectionAudit,
    MemoryProjectionMismatch, StateInspectionMemoryProjection,
};
pub use self_evolving::{
    AdaptiveHeuristic, AdaptiveHeuristicInput, CaseOutcome, EpisodeInput, EpisodeMatch,
    EpisodeQuery, HeuristicMatch, HeuristicQuery, InMemorySelfEvolvingMemory, RetrospectiveEpisode,
    SelfEvolvingCaseReflection, SelfEvolvingMemory, SelfEvolvingMemorySnapshot,
    SelfEvolvingReflectionReport, ToolReliabilityRecord, ToolReliabilityUpdate,
};
pub use service::{
    MemoryAdapterStatus, MemoryCapabilityCoverage, MemoryConsumerProfile, MemoryReadinessReport,
    MemoryServiceAdapterChecklist, MemoryServiceChecklistItem, MemoryServiceChecklistSeverity,
    MemoryServiceDryRun, MemoryServiceManifest, MemoryServiceRequirement, MemoryServiceShadowPlan,
    MemoryServiceShadowPlanInputs, MemoryServiceShadowSummary, MemoryServiceStartupEvidence,
    MemoryStartupAdmissionEvidence, MemoryStartupEvidenceSink,
};
pub use short_term::{InMemoryShortTermKv, ShortTermEntry, ShortTermKv};
pub use skills::{InMemorySkillLibrary, SkillLibrary, SkillQuery, SkillRecord, SkillRecordInput};

pub type Metadata = BTreeMap<String, String>;
pub type MemoryResult<T> = Result<T, MemoryError>;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MemoryScope {
    pub agent_id: Option<String>,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
}

impl MemoryScope {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn for_task(task_id: impl Into<String>) -> Self {
        Self {
            task_id: Some(task_id.into()),
            ..Self::default()
        }
    }

    pub fn with_agent(mut self, agent_id: impl Into<String>) -> Self {
        self.agent_id = Some(agent_id.into());
        self
    }

    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn same_task_as(&self, other: &Self) -> Option<bool> {
        Some(self.task_id.as_deref()? == other.task_id.as_deref()?)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryAccessPurpose {
    Recall,
    Reinforce,
    Offload,
    Repair,
    Inspect,
}

impl MemoryAccessPurpose {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Recall => "recall",
            Self::Reinforce => "reinforce",
            Self::Offload => "offload",
            Self::Repair => "repair",
            Self::Inspect => "inspect",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryRequestContext {
    pub scope: MemoryScope,
    pub purpose: MemoryAccessPurpose,
    pub limit: usize,
    pub tags: Vec<String>,
}

impl MemoryRequestContext {
    pub fn new(scope: MemoryScope, purpose: MemoryAccessPurpose) -> Self {
        Self {
            scope,
            purpose,
            limit: 16,
            tags: Vec::new(),
        }
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit.max(1);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MemoryAdapterCapability {
    ShortTermKv,
    LongTermMemory,
    SkillLibrary,
    ExperienceGovernance,
    CleanGistSelection,
    MemoryIndex,
    TieredPlacement,
    ExperienceReplay,
    ContextInjection,
    RepairPlanning,
    DiskKvOffload,
    KvSwap,
    RetentionPlanning,
    CompactionPlanning,
    MemoryEvolution,
    StateInspection,
    InfiniMemoryPlanning,
    SemanticRetrieval,
}

impl MemoryAdapterCapability {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ShortTermKv => "short_term_kv",
            Self::LongTermMemory => "long_term_memory",
            Self::SkillLibrary => "skill_library",
            Self::ExperienceGovernance => "experience_governance",
            Self::CleanGistSelection => "clean_gist_selection",
            Self::MemoryIndex => "memory_index",
            Self::TieredPlacement => "tiered_placement",
            Self::ExperienceReplay => "experience_replay",
            Self::ContextInjection => "context_injection",
            Self::RepairPlanning => "repair_planning",
            Self::DiskKvOffload => "disk_kv_offload",
            Self::KvSwap => "kv_swap",
            Self::RetentionPlanning => "retention_planning",
            Self::CompactionPlanning => "compaction_planning",
            Self::MemoryEvolution => "memory_evolution",
            Self::StateInspection => "state_inspection",
            Self::InfiniMemoryPlanning => "infini_memory_planning",
            Self::SemanticRetrieval => "semantic_retrieval",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryAdapterDescriptor {
    pub name: String,
    pub capabilities: Vec<MemoryAdapterCapability>,
    pub read_only: bool,
}

impl MemoryAdapterDescriptor {
    pub fn new(name: impl Into<String>, capabilities: Vec<MemoryAdapterCapability>) -> Self {
        Self {
            name: name.into(),
            capabilities,
            read_only: false,
        }
    }

    pub fn read_only(mut self) -> Self {
        self.read_only = true;
        self
    }

    pub fn has_capability(&self, capability: MemoryAdapterCapability) -> bool {
        self.capabilities.contains(&capability)
    }

    pub fn capability_codes(&self) -> Vec<String> {
        self.capabilities
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .map(|capability| capability.as_str().to_owned())
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryAdapterHealth {
    pub ready: bool,
    pub record_count: Option<usize>,
    pub warnings: Vec<String>,
}

impl MemoryAdapterHealth {
    pub fn ready(record_count: Option<usize>) -> Self {
        Self {
            ready: true,
            record_count,
            warnings: Vec::new(),
        }
    }

    pub fn with_warning(mut self, warning: impl Into<String>) -> Self {
        self.warnings.push(warning.into());
        self
    }

    pub fn not_ready(record_count: Option<usize>, warnings: Vec<String>) -> Self {
        Self {
            ready: false,
            record_count,
            warnings,
        }
    }
}

pub trait MemoryAdapter {
    fn descriptor(&self) -> MemoryAdapterDescriptor;
    fn health(&self) -> MemoryResult<MemoryAdapterHealth>;
}

#[derive(Debug)]
pub enum MemoryError {
    Io(std::io::Error),
    InvalidInput(String),
    NotFound(String),
}

impl fmt::Display for MemoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "io error: {error}"),
            Self::InvalidInput(message) => write!(f, "invalid input: {message}"),
            Self::NotFound(message) => write!(f, "not found: {message}"),
        }
    }
}

impl std::error::Error for MemoryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::InvalidInput(_) | Self::NotFound(_) => None,
        }
    }
}

impl From<std::io::Error> for MemoryError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

pub trait AgenticMemory {
    type ShortTerm: ShortTermKv;
    type LongTerm: LongTermMemory;
    type Skills: SkillLibrary;

    fn short_term(&self) -> &Self::ShortTerm;
    fn short_term_mut(&mut self) -> &mut Self::ShortTerm;
    fn long_term(&self) -> &Self::LongTerm;
    fn long_term_mut(&mut self) -> &mut Self::LongTerm;
    fn skills(&self) -> &Self::Skills;
    fn skills_mut(&mut self) -> &mut Self::Skills;
}

#[derive(Debug, Clone)]
pub struct MemoryPorts<S, L, K> {
    pub short_term: S,
    pub long_term: L,
    pub skills: K,
}

impl<S, L, K> MemoryPorts<S, L, K> {
    pub fn new(short_term: S, long_term: L, skills: K) -> Self {
        Self {
            short_term,
            long_term,
            skills,
        }
    }
}

impl<S, L, K> MemoryPorts<S, L, K>
where
    S: ShortTermKv,
    L: LongTermMemory,
    K: SkillLibrary,
{
    pub fn adapter_status(
        &self,
        write_mode: AdapterWriteMode,
    ) -> MemoryResult<MemoryAdapterStatus> {
        MemoryAdapterStatus::inspect(self, write_mode)
    }

    pub fn service_manifest(
        &self,
        write_mode: AdapterWriteMode,
    ) -> MemoryResult<MemoryServiceManifest> {
        Ok(MemoryServiceManifest::new(vec![
            self.adapter_status(write_mode)?,
        ]))
    }

    pub fn readiness_for(
        &self,
        profile: MemoryConsumerProfile,
        write_mode: AdapterWriteMode,
    ) -> MemoryResult<MemoryReadinessReport> {
        let manifest = self.service_manifest(write_mode)?;
        Ok(manifest.readiness(&MemoryServiceRequirement::for_profile(profile, write_mode)))
    }
}

impl<S, L, K> AgenticMemory for MemoryPorts<S, L, K>
where
    S: ShortTermKv,
    L: LongTermMemory,
    K: SkillLibrary,
{
    type ShortTerm = S;
    type LongTerm = L;
    type Skills = K;

    fn short_term(&self) -> &Self::ShortTerm {
        &self.short_term
    }

    fn short_term_mut(&mut self) -> &mut Self::ShortTerm {
        &mut self.short_term
    }

    fn long_term(&self) -> &Self::LongTerm {
        &self.long_term
    }

    fn long_term_mut(&mut self) -> &mut Self::LongTerm {
        &mut self.long_term
    }

    fn skills(&self) -> &Self::Skills {
        &self.skills
    }

    fn skills_mut(&mut self) -> &mut Self::Skills {
        &mut self.skills
    }
}

impl<S, L, K> MemoryAdapter for MemoryPorts<S, L, K>
where
    S: ShortTermKv,
    L: LongTermMemory,
    K: SkillLibrary,
{
    fn descriptor(&self) -> MemoryAdapterDescriptor {
        MemoryAdapterDescriptor::new(
            "memory_ports",
            vec![
                MemoryAdapterCapability::ShortTermKv,
                MemoryAdapterCapability::LongTermMemory,
                MemoryAdapterCapability::SkillLibrary,
            ],
        )
    }

    fn health(&self) -> MemoryResult<MemoryAdapterHealth> {
        Ok(MemoryAdapterHealth::ready(Some(
            self.short_term.len() + self.long_term.len() + self.skills.len(),
        )))
    }
}

pub fn in_memory_ports()
-> MemoryPorts<InMemoryShortTermKv, InMemoryLongTermMemory, InMemorySkillLibrary> {
    MemoryPorts::new(
        InMemoryShortTermKv::default(),
        InMemoryLongTermMemory::default(),
        InMemorySkillLibrary::default(),
    )
}

pub(crate) fn clamp01(value: f32) -> f32 {
    value.clamp(0.0, 1.0)
}

pub(crate) fn stable_hash(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_ports_report_adapter_health() {
        let mut ports = in_memory_ports();
        ports
            .short_term_mut()
            .put(
                "focus".to_owned(),
                b"agent memory".to_vec(),
                Metadata::new(),
            )
            .unwrap();
        ports
            .long_term_mut()
            .remember(MemoryDocumentInput::new("long-term lesson", vec![1.0]))
            .unwrap();
        ports
            .skills_mut()
            .add_skill(SkillRecordInput::new("repair", "repair dirty memory"))
            .unwrap();

        let descriptor = ports.descriptor();
        assert_eq!(descriptor.name, "memory_ports");
        assert!(
            descriptor
                .capabilities
                .contains(&MemoryAdapterCapability::ShortTermKv)
        );
        assert!(
            descriptor
                .capabilities
                .contains(&MemoryAdapterCapability::LongTermMemory)
        );
        assert!(
            descriptor
                .capabilities
                .contains(&MemoryAdapterCapability::SkillLibrary)
        );

        let health = ports.health().unwrap();
        assert!(health.ready);
        assert_eq!(health.record_count, Some(3));
    }

    #[test]
    fn memory_ports_build_core_service_manifest() {
        let ports = in_memory_ports();
        let manifest = ports
            .service_manifest(AdapterWriteMode::IsolatedWrite)
            .unwrap();

        assert_eq!(manifest.adapters.len(), 1);
        assert_eq!(manifest.adapters[0].descriptor.name, "memory_ports");
        assert_eq!(
            manifest.adapters[0].write_mode,
            AdapterWriteMode::IsolatedWrite
        );

        let readiness = manifest.readiness(&MemoryServiceRequirement::for_profile(
            MemoryConsumerProfile::Core,
            AdapterWriteMode::IsolatedWrite,
        ));
        assert!(readiness.ready);
        assert_eq!(readiness.missing_capability_codes(), Vec::<String>::new());
        assert!(
            readiness
                .summary_line()
                .contains("profile=core required_write_mode=isolated_write ready=true")
        );
    }

    #[test]
    fn memory_ports_report_profile_readiness_without_manual_manifest() {
        let ports = in_memory_ports();

        let core = ports
            .readiness_for(MemoryConsumerProfile::Core, AdapterWriteMode::IsolatedWrite)
            .unwrap();
        assert!(core.ready);
        assert_eq!(core.missing_capability_codes(), Vec::<String>::new());

        let agent = ports
            .readiness_for(MemoryConsumerProfile::Agent, AdapterWriteMode::ReadOnly)
            .unwrap();
        assert!(!agent.ready);
        assert!(
            agent
                .missing_capability_codes()
                .contains(&"experience_governance".to_owned())
        );
        assert!(
            agent
                .missing_capability_codes()
                .contains(&"context_injection".to_owned())
        );
        assert!(
            agent
                .summary_line()
                .contains("profile=agent required_write_mode=read_only ready=false")
        );
    }

    #[test]
    fn adapter_descriptor_reports_stable_capability_codes() {
        let descriptor = MemoryAdapterDescriptor::new(
            "root_shadow",
            vec![
                MemoryAdapterCapability::KvSwap,
                MemoryAdapterCapability::ExperienceGovernance,
                MemoryAdapterCapability::KvSwap,
            ],
        )
        .read_only();

        assert!(descriptor.read_only);
        assert!(descriptor.has_capability(MemoryAdapterCapability::KvSwap));
        assert!(!descriptor.has_capability(MemoryAdapterCapability::SkillLibrary));
        assert_eq!(
            descriptor.capability_codes(),
            vec!["experience_governance".to_owned(), "kv_swap".to_owned()]
        );
    }

    #[test]
    fn adapter_health_builders_preserve_warning_evidence() {
        let ready = MemoryAdapterHealth::ready(Some(2)).with_warning("store_lag=1");
        let blocked = MemoryAdapterHealth::not_ready(
            Some(3),
            vec!["adapter_unhealthy".to_owned(), "catalog_missing".to_owned()],
        );

        assert!(ready.ready);
        assert_eq!(ready.record_count, Some(2));
        assert_eq!(ready.warnings, vec!["store_lag=1".to_owned()]);
        assert!(!blocked.ready);
        assert_eq!(blocked.record_count, Some(3));
        assert_eq!(
            blocked.warnings,
            vec!["adapter_unhealthy".to_owned(), "catalog_missing".to_owned()]
        );
    }

    #[test]
    fn memory_scope_compares_only_known_task_ids() {
        let left = MemoryScope::for_task("task-a").with_agent("agent");
        let right = MemoryScope::for_task("task-a").with_session("session");
        let missing = MemoryScope::new();

        assert_eq!(left.same_task_as(&right), Some(true));
        assert_eq!(
            left.same_task_as(&MemoryScope::for_task("task-b")),
            Some(false)
        );
        assert_eq!(left.same_task_as(&missing), None);
    }
}
