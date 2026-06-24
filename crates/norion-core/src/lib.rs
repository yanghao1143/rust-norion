//! Pure Rust kernel contracts for Norion.
//!
//! This crate intentionally has no dependency on Candle, service code, or the
//! root `rust-norion` crate. It provides stable ports that higher layers can
//! implement or adapt while the existing monolith is gradually split.

pub mod acceptance;
pub mod adapter;
pub mod attention;
pub mod diagnostics;
pub mod engine;
pub mod experiment;
pub mod failure_return;
pub mod fht_dke;
pub mod fusion;
pub mod hardware;
pub mod kv;
pub mod manifest;
pub mod memory;
pub mod planning;
pub mod profile;
pub mod quant;
pub mod recursive;
pub mod request;
pub mod response;
pub mod router;
pub mod runtime;
pub mod tiered;
pub mod transformer;

pub use acceptance::{
    RuntimeAcceptanceContext, RuntimeBoundaryAcceptanceSummary, RuntimeBoundaryAdapterSummary,
    RuntimeBoundaryCommitAction, RuntimeBoundaryCommitReadinessSummary, RuntimeBoundaryCommitStage,
    RuntimeBoundaryCommitSummary, RuntimeBoundaryEnvelopeSummary, RuntimeBoundaryGateSummary,
    RuntimeBoundaryKvSummary, RuntimeFailureReturnReport, RuntimeFailureReturnSource,
    RuntimeFailureReturnSummary, RuntimeKvSideEffectCommitAction, RuntimeKvSideEffectCommitSummary,
    RuntimeKvSideEffectProblemKind, RuntimeKvSideEffectReadinessSummary, RuntimeKvSideEffectStage,
    RuntimeManifestBoundaryCommitAction, RuntimeManifestBoundaryCommitProblemKind,
    RuntimeManifestBoundaryCommitReadinessSummary, RuntimeManifestBoundaryCommitStage,
    RuntimeManifestBoundaryCommitSummary, RuntimeManifestBoundaryKvStage,
    RuntimeManifestBoundaryKvSummary,
};
pub use adapter::{
    AdapterExecutionContext, AdapterExecutionContextCommitAction,
    AdapterExecutionContextCommitSummary, AdapterExecutionContextSummary,
    AdapterFailureReturnReport, AdapterFailureReturnSource, AdapterFailureReturnSummary,
    AdapterFallbackReason, AdapterObservation, AdapterRuntimeClampCommitAction,
    AdapterRuntimeClampCommitSummary, AdapterRuntimeClampSummary, AdapterSelection,
    AdapterSelectionCommitAction, AdapterSelectionCommitSummary, AdapterSelectionReport,
    AdapterSelectionRuntimeCommitAction, AdapterSelectionRuntimeCommitSummary,
    AdapterSelectionRuntimeSummary, RuntimeAdapter,
};
pub use attention::{
    AttentionCandidate, AttentionCandidateBatchSummary, AttentionCandidateSummary,
    AttentionDecision, AttentionDecisionSummary, AttentionPolicy, AttentionSelectionReadinessStage,
    AttentionSelectionReadinessSummary, ThresholdAttentionAdjustmentAction,
    ThresholdAttentionAdjustmentReport, ThresholdAttentionPolicy, ThresholdAttentionPolicySummary,
};
pub use diagnostics::{
    DeviceExecutionSource, DiagnosticsPressureBand, EmbeddingCallDiagnostics, EmbeddingDiagnostics,
    EmbeddingDiagnosticsSummary, EmbeddingSource, InferenceDiagnostics,
    InferenceDiagnosticsRequestParitySummary, InferenceDiagnosticsSummary, RuntimeDiagnostics,
    RuntimeDiagnosticsContractSummary, RuntimeDiagnosticsRequestParitySummary,
    RuntimeDiagnosticsSummary, RuntimeHardwareDiagnosticsContractSummary,
    RuntimeHardwareDiagnosticsReport, RuntimeHardwareDiagnosticsSummary,
};
pub use engine::{
    GeneratedToken, GeneratedTokenMetrics, InferenceEngine, InferenceError, InferenceOutcome,
    InferenceOutcomeSummary, InferenceRequest, RuntimeFailureBatchSummary, RuntimeFailureKind,
    RuntimeFailureReport, RuntimeFailureSummary,
};
pub use experiment::{ExperimentSwitches, ExperimentSwitchesSummary};
pub use failure_return::{
    FailureReturnFamily, FailureReturnRoutingBatchSummary, FailureReturnRoutingDecision,
    FailureReturnRoutingKey, FailureReturnRoutingSelection, FailureReturnRoutingSummary,
    RuntimeKvPersistenceFailureReturnSelection,
};
pub use fht_dke::{
    DeterministicFhtDkeBudgeter, FhtDkeBudget, FhtDkeBudgetSummary, FhtDkeBudgeter, FhtDkeInput,
    FhtDkePlanningCommitAction, FhtDkePlanningCommitSummary, FhtDkePlanningReadinessStage,
    FhtDkePlanningReadinessSummary,
};
pub use fusion::{
    KvFusionCommitAction, KvFusionCommitSummary, KvFusionMerge, KvFusionMergeSummary, KvFusionPair,
    KvFusionPolicy, ReinforcedKvFusionPolicy,
};
pub use hardware::{
    ComputeLane, DeviceClass, DeviceExecutionAdapterCommitAction,
    DeviceExecutionAdapterCommitSummary, DeviceExecutionAdapterSummary, DeviceExecutionPlan,
    DeviceExecutionPlanCommitAction, DeviceExecutionPlanCommitSummary, DeviceExecutionPlanSummary,
    DeviceMemoryMode, DeviceProfileDescriptor, DeviceProfileDescriptorSummary, DeviceTier,
    HardwareAdapterBridgeCommitAction, HardwareAdapterBridgeCommitSummary,
    HardwareAdapterBridgeSummary, HardwareAllocator, HardwareFailureReturnReport,
    HardwareFailureReturnSource, HardwareFailureReturnSummary, HardwareLoadKind,
    HardwareLoadSnapshot, HardwareLoadSnapshotCommitAction, HardwareLoadSnapshotCommitSummary,
    HardwareLoadSnapshotSummary, HardwarePlan, HardwarePlanCommitAction, HardwarePlanCommitSummary,
    HardwarePlanSummary, HardwarePressureBand, HardwareRuntimeCommitAction,
    HardwareRuntimeCommitSummary, HardwareRuntimeReadinessStage, HardwareRuntimeReadinessSummary,
};
pub use kv::{
    InMemoryKvCache, KvBlock, KvBlockShapeSummary, KvCachePort, KvNamespace,
    KvNamespaceCountDriftCommitAction, KvNamespaceCountDriftCommitSummary,
    KvNamespaceCountDriftSummary, KvNamespaceCounts, RuntimeKvBlockContract,
    RuntimeKvBlockContractCheckSummary, RuntimeKvBlockContractSummary, RuntimeKvCandidate,
    RuntimeKvDirection, RuntimeKvExchangeFailureReturnReport, RuntimeKvExchangeFailureReturnSource,
    RuntimeKvExchangeFailureReturnSummary, RuntimeKvImportBlockSummary,
    RuntimeKvImportManifestPlanSummary, RuntimeKvImportPlan, RuntimeKvImportReadinessCommitAction,
    RuntimeKvImportReadinessCommitSummary, RuntimeKvImportReadinessStage,
    RuntimeKvImportReadinessSummary, RuntimeKvImportSummary,
    RuntimeKvPersistenceFailureReturnReport, RuntimeKvPersistenceFailureReturnSource,
    RuntimeKvPersistenceFailureReturnSummary, RuntimeKvValidationBoundarySummary,
    RuntimeKvValidationReport, RuntimeKvValidationSummary,
};
pub use manifest::{
    ManifestFailureReturnReport, ManifestFailureReturnSource, ManifestFailureReturnSummary,
    QuantizationBits, RuntimeDeviceHandoffCommitAction, RuntimeDeviceHandoffCommitSummary,
    RuntimeDeviceHandoffReadinessSummary, RuntimeDeviceHandoffStage, RuntimeKvPolicy,
    RuntimeKvPolicySummary, RuntimeManifestAbiSummary, RuntimeManifestAdapterCompatibilitySummary,
    RuntimeManifestDigest, RuntimeManifestExecutionCompatibilitySummary, RuntimeManifestValidation,
    RuntimeManifestValidationCommitAction, RuntimeManifestValidationCommitSummary,
    RuntimeManifestValidationSummary, RuntimeQuantizationPolicy, RuntimeQuantizationPolicySummary,
    TransformerRuntimeArchitecture, TransformerRuntimeArchitectureSummary,
    default_transformer_runtime_architecture,
};
pub use memory::{
    MemoryCompactionMerge, MemoryCompactionPolicy, MemoryCompactionReport, MemoryGovernancePolicy,
    MemoryGovernanceReport, MemoryGovernanceSummary, MemoryRecord, MemoryRecordSummary,
    MemoryRetentionDecision, MemoryRetentionPolicy, MemoryUpdateAction, MemoryUpdateBatchSummary,
    MemoryUpdateReport, MemoryUpdateSummary, RetentionReport, plan_compaction,
    plan_memory_governance, preview_retention,
};
pub use planning::{
    RuntimePlanningAcceptanceCommitAction, RuntimePlanningAcceptanceCommitSummary,
    RuntimePlanningAcceptanceReport, RuntimePlanningAcceptanceSummary, RuntimePlanningDigest,
    RuntimePlanningFailureReturnReport, RuntimePlanningFailureReturnSource,
    RuntimePlanningFailureReturnSummary, RuntimePlanningKvClampReason,
    RuntimePlanningKvClampSummary, RuntimePlanningKvExchange,
    RuntimePlanningManifestKvBridgeSummary, RuntimePlanningReadinessStage,
    RuntimePlanningReadinessSummary, RuntimePlanningSummary,
};
pub use profile::{
    HierarchyAdjustmentFeedback, HierarchyAdjustmentFeedbackSummary, HierarchyMutationKind,
    HierarchyWeightFocus, HierarchyWeights, HierarchyWeightsSummary, ProfileHierarchyObservations,
    ProfileHierarchyWeights, ProfileHierarchyWeightsSummary, TaskAwareHierarchyAdjustmentPolicy,
    TaskAwareHierarchyAdjustmentReport, TaskAwareHierarchyMutationHistory,
    TaskAwareHierarchyMutationPlan, TaskAwareHierarchyMutationRecord, TaskProfile,
};
pub use quant::{
    KvQuantizationPlan, QuantizationError, QuantizedKvBlock, QuantizedKvPayloadSummary,
    QuantizedVector,
};
pub use recursive::{
    RecursiveChunk, RecursiveExecutionWave, RecursiveMergeRound, RecursiveScheduleDigest,
    RecursiveScheduleSummary, RecursiveScheduleValidationSummary, RecursiveSchedulerConfig,
    estimate_prompt_tokens,
};
pub use request::{
    RUNTIME_REQUEST_SCHEMA, RuntimeRequestAcceptanceReport, RuntimeRequestAcceptanceSummary,
    RuntimeRequestEnvelope, RuntimeRequestEnvelopeSummary, RuntimeRequestFailureReturnReport,
    RuntimeRequestFailureReturnSource, RuntimeRequestFailureReturnSummary,
    RuntimeRequestGateSummary, RuntimeRequestManifestPlanningReadinessStage,
    RuntimeRequestManifestPlanningReadinessSummary, RuntimeRequestPlanningParitySummary,
    RuntimeRequestPlanningReadinessStage, RuntimeRequestPlanningReadinessSummary,
};
pub use response::{
    RUNTIME_RESPONSE_SCHEMA, RuntimeResponseAcceptanceReport, RuntimeResponseAcceptanceSummary,
    RuntimeResponseEnvelope, RuntimeResponseEnvelopeSummary, RuntimeResponseFailureReturnReport,
    RuntimeResponseFailureReturnSource, RuntimeResponseFailureReturnSummary,
    RuntimeResponseGateSummary, RuntimeResponseManifestKvSummary, RuntimeResponsePlannedKvSummary,
    RuntimeResponseReadinessStage, RuntimeResponseReadinessSummary,
    RuntimeResponseRequestParitySummary,
};
pub use router::{
    DefaultHierarchicalRouter, GenerationMetrics, HierarchicalRouter, ProfileObservations,
    ProfileThresholds, RouteBudget, RouteBudgetReadinessCommitAction,
    RouteBudgetReadinessCommitSummary, RouteBudgetReadinessStage, RouteBudgetReadinessSummary,
    RouteLayer, RouteLayerCounts, RouterState, RoutingContext, RoutingDecision,
    RoutingDecisionSummary, RoutingFeedback, RoutingFeedbackBatchSummary, RoutingFeedbackSummary,
    TokenFeatures,
};
pub use runtime::{
    RuntimeBackendMaxTokensCommitAction, RuntimeBackendMaxTokensCommitSummary,
    RuntimeGenerationBudget, RuntimeMetadata, RuntimeMetadataShapeSummary,
};
pub use tiered::{
    MemoryPlacement, MemoryTier, TierCounts, TierMigration, TierMigrationAction,
    TierMigrationSummary, TieredCachePlan, TieredCacheSummary, TieredMemoryCandidate,
    TieredMemoryCandidateSummary, TieredMemoryScheduler,
};
pub use transformer::{
    DefaultTransformerPlanner, RuntimeKvExportBlockSummary, RuntimeKvExportManifestPlanSummary,
    RuntimeKvExportPlan, RuntimeKvExportPlanningSummary, RuntimeKvExportReadinessCommitAction,
    RuntimeKvExportReadinessCommitSummary, RuntimeKvExportReadinessStage,
    RuntimeKvExportReadinessSummary, RuntimeKvExportSummary, TransformerAttentionKind,
    TransformerForwardBatchSummary, TransformerForwardSummary, TransformerLayerBudget,
    TransformerLayerBudgetBatchSummary, TransformerLayerBudgetSummary, TransformerPlanCounts,
    TransformerPlanDigest, TransformerPlanReadinessStage, TransformerPlanReadinessSummary,
    TransformerPlanSummary, TransformerPlannerContract, TransformerPlanningInput,
    TransformerPlanningPressureSummary, TransformerPlanningReadinessStage,
    TransformerPlanningReadinessSummary, TransformerTemplate, TransformerTemplateKind,
};
