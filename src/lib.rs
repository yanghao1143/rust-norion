pub mod adaptive_state;
pub mod agent_team;
pub mod benchmark;
pub mod disk_kv;
pub mod drift;
pub mod engine;
pub mod experience;
pub mod experience_replay;
pub mod gemma_runtime;
pub mod gist_memory;
pub mod hardware;
pub mod hierarchy;
pub mod improvement_corpus;
pub mod infini_memory;
pub mod kv_cache;
pub mod kv_exchange;
pub mod kv_quant;
pub mod local_runtime;
pub mod memory_admission;
pub mod no_weight_retrain;
pub mod process_reward;
pub mod production_runtime;
pub mod reasoning_genome;
pub mod recursive_scheduler;
pub mod reflection;
pub mod router;
pub mod runtime;
pub mod runtime_manifest;
pub mod rust_validation;
pub mod self_evolution;
pub mod self_evolving_memory;
pub mod semantic_index;
pub mod session_state;
pub mod split;
pub mod state_inspect;
pub mod tenant_scope;
pub mod tiered_cache;
pub mod token_stream;
pub mod toolsmith;
pub mod trace;
pub mod transformer;

pub use adaptive_state::{AdaptiveState, EvolutionLedger, LiveInferenceEvolution};
pub use agent_team::{
    AgentConflict, AgentEvolutionSignal, AgentHandoffAggregationReport, AgentHandoffContext,
    AgentHandoffInput, AgentHandoffReview, AgentHandoffSanitizer, AgentHandoffTrustState,
    AgentIsolationPolicy, AgentMessage, AgentMessageKind, AgentNode, AgentRole,
    AgentTeamAggregation, AgentTeamInput, AgentTeamPlan, AgentTeamPlanner, CrossWindowBudget,
    CrossWindowBudgetReport, CrossWindowConflictClass, CrossWindowExchangeAggregator,
    CrossWindowExchangeContext, CrossWindowExchangeReport, CrossWindowExperiencePacket,
    CrossWindowPacketDecision, CrossWindowPacketReview,
};
pub use benchmark::{
    BenchmarkCase, BenchmarkCaseResult, BenchmarkGate, BenchmarkGateReport,
    BenchmarkImprovementCorpusEvidence, BenchmarkLiveEvolutionEvidence,
    BenchmarkMemoryGovernanceEvidence, BenchmarkRoutingEvidence, BenchmarkSummary,
    KvQuantBenchmarkCaseResult, KvQuantBenchmarkGate, KvQuantBenchmarkGateReport,
    KvQuantBenchmarkSummary, PersistentRoundtripDeviceReport, PersistentRoundtripInput,
    PersistentRoundtripMatrixReport, PersistentRoundtripReport, SelfEvolvingMemoryAbCase,
    SelfEvolvingMemoryAbGate, SelfEvolvingMemoryAbGateReport, SelfEvolvingMemoryAbHarness,
    SelfEvolvingMemoryAbRecommendation, SelfEvolvingMemoryAbReport, SelfEvolvingMemoryAbResult,
    SelfEvolvingMemoryEvalLanguage, SelfEvolvingMemoryEvalMode,
    SelfEvolvingMemoryValidationEvidence, default_benchmark_cases,
    default_self_evolving_memory_ab_cases, run_default_self_evolving_memory_ab_suite,
    seeded_self_evolving_memory_ab_store,
};
pub use disk_kv::DiskKvStore;
pub use drift::{DriftGuard, DriftInput, DriftReport, DriftSeverity};
pub use engine::{
    EmbeddingCallDiagnostics, EmbeddingDiagnostics, EmbeddingSource, GenerationContext,
    HeuristicBackend, InferenceBackend, InferenceOutcome, InferenceRequest, MemoryFeedbackReport,
    NoironEngine,
};
pub use experience::{
    ExperienceHygieneFinding, ExperienceHygieneQuarantinePlan, ExperienceHygieneReport,
    ExperienceHygieneSeverity, ExperienceIndexFinding, ExperienceIndexReport, ExperienceInput,
    ExperienceMatch, ExperienceRecord, ExperienceRepairAction, ExperienceRepairItem,
    ExperienceRepairPlan, ExperienceRepairProjection, ExperienceRepairSkippedItem,
    ExperienceRetrievalReport, ExperienceRuntimeTokenMetrics, ExperienceStore,
    render_experience_hint,
};
pub use experience_replay::{
    ExperienceReplayItem, ExperienceReplayPlan, ExperienceReplayPlanner, ExperienceReplayReport,
};
pub use gemma_runtime::{
    GEMMA4_12B_ATTENTION_HEADS, GEMMA4_12B_DEFAULT_LOCAL_ISQ, GEMMA4_12B_DEFAULT_PAGED_ATTN,
    GEMMA4_12B_DEFAULT_QUANT, GEMMA4_12B_DEFAULT_RUNTIME_WINDOW, GEMMA4_12B_DEFAULT_THINKING,
    GEMMA4_12B_HIDDEN_SIZE, GEMMA4_12B_KV_HEADS, GEMMA4_12B_LAYER_COUNT,
    GEMMA4_12B_LOCAL_WINDOW_TOKENS, GEMMA4_12B_MODEL_ID, GEMMA4_12B_NATIVE_CONTEXT_WINDOW,
    GEMMA4_12B_TOKENIZER, GemmaRuntimeConfig, GemmaRuntimeDefaults, GemmaRuntimeFitSummary,
    GemmaRuntimeQuantizationMode, ensure_gemma4_12b_runtime_defaults,
    infer_hf_cache_from_local_snapshot, normalize_runtime_metadata,
};
pub use gist_memory::{GistGenerator, GistLevel, GistRecord};
pub use hardware::{
    ComputeLane, DeviceClass, DeviceExecutionPlan, DeviceMemoryMode, DevicePlanGateReport,
    DevicePlanGateRow, DeviceProfileDescriptor, DeviceTier, HardwareAllocator, HardwarePlan,
    HardwareProbe, HardwareProbeReport, HardwareSnapshot, KvPrecisionPolicySummary,
    MemoryGovernancePlan, RuntimeAdapterHint, RuntimeManifestDeviceGateReport,
};
pub use hierarchy::{
    HierarchyAdjustmentPreviewPlanner, HierarchyAdjustmentPreviewPolicy,
    HierarchyAdjustmentPreviewReport, HierarchyController, HierarchyState, HierarchyWeightDelta,
    HierarchyWeights, ProfileHierarchyObservations, ProfileHierarchyWeights,
    TaskAwareHierarchyInput, TaskAwareHierarchyPlan, TaskAwareHierarchyPlanner, TaskComputeBudget,
    TaskHierarchyMutationKind, TaskHierarchyMutationRecord, TaskHierarchyReplayReport,
    TaskLanguageMode, TaskMode, TaskModeSignals, TaskProfile,
};
pub use improvement_corpus::{
    ImprovementApprovalState, ImprovementCorpus, ImprovementCorpusReport, ImprovementEpisodeClass,
    ImprovementEpisodeInput, ImprovementEpisodeRecord, ImprovementEvidenceLane,
    ImprovementPrivacyState, ImprovementValidationStatus,
};
pub use infini_memory::{
    InfiniMemoryCounts, InfiniMemoryItem, InfiniMemoryPlan, InfiniMemoryPlanner, InfiniMemoryScope,
};
pub use kv_cache::{
    KvFusionCache, MemoryCompactionMerge, MemoryCompactionPolicy, MemoryCompactionReport,
    MemoryEntry, MemoryMatch, MemoryResidencyCandidate, MemoryResidencyDecisionRecord,
    MemoryResidencyPlan, MemoryResidencyPolicy, MemoryResidencyState, MemoryRetentionPolicy,
    MemoryUpdateAction, MemoryUpdateReport, RetentionReport, plan_memory_residency,
};
pub use kv_exchange::{RuntimeKvBlock, RuntimeKvBlockValidationError};
pub use kv_quant::{QuantizationBits, QuantizationError, QuantizedVector};
pub use local_runtime::LocalTransformerRuntime;
pub use memory_admission::{
    MemoryAdmissionApprovalState, MemoryAdmissionCandidate, MemoryAdmissionDecision,
    MemoryAdmissionInput, MemoryAdmissionKind, MemoryAdmissionPreview, MemoryAdmissionReviewPacket,
    MemoryKvLedgerRecord, MemoryKvLedgerWriteDecision, MemoryKvLedgerWritePlan,
    MemoryKvLedgerWritePolicy, MemoryPrivacyClassification, ReinforcedKvFusionCandidate,
    ReinforcedKvFusionDecision, ReinforcedKvFusionDecisionRecord, ReinforcedKvFusionPlan,
    ReinforcedKvFusionPolicy, ReinforcedKvFusionScoreComponents, ReinforcedKvFusionSource,
};
pub use no_weight_retrain::{
    AdapterTrainingHandoffState, NoWeightImprovementCandidate, NoWeightImprovementLane,
    NoWeightRetrainDecision, NoWeightRetrainGate, NoWeightRetrainPolicy, NoWeightRetrainScorecard,
};
pub use process_reward::{
    ProcessRewardComponents, ProcessRewardInput, ProcessRewardReport, ProcessRewarder, RewardAction,
};
pub use production_runtime::{
    ModelRuntimeForwardKernel, ProductionForwardKernel, ProductionKernelConformanceDeviceReport,
    ProductionKernelConformanceGate, ProductionKernelConformanceMatrixReport,
    ProductionKernelConformanceReport, ProductionKernelContext, ProductionKernelOutput,
    ProductionTransformerRuntime, ReferenceProductionForwardKernel, RuntimeAssetSummary,
};
pub use reasoning_genome::{
    DnaChainKind, DnaGeneChain, DnaGeneEvidenceKind, DnaGeneLineage, DnaGeneRecord,
    DnaGeneSchemaError, DnaGeneSourceEvidence, GeneLifecycleAction, GeneLifecycleRecord,
    GeneLifecycleSourceEvidence, GeneLifecycleSourceKind, GeneScissorsIntent, GeneValidationStatus,
    GenomeExpression, GenomeExpressionInput, MutationFixtureKind, MutationPlan,
    MutationRepairCandidateFixture, MutationRepairFixture, MutationRepairFixtureCorpus,
    MutationRepairFixtureGateReport, MutationRepairFixtureReport, MutationRepairFixtureResult,
    ReasoningGene, ReasoningGeneKind, ReasoningGeneStatus, ReasoningGenome,
    default_mutation_repair_fixture_corpus,
};
pub use recursive_scheduler::{
    RecursiveChunk, RecursiveExecutionWave, RecursiveMergeRound, RecursiveSchedule,
    RecursiveScheduler,
};
pub use reflection::{
    DraftToken, InferenceDraft, ReasoningStep, ReflectionIssue, ReflectionReport,
    ReflectionSeverity, Reflector, RuntimeDiagnostics,
};
pub use router::{
    AdaptiveRouteAction, AdaptiveRouteCandidate, AdaptiveRouteDecision,
    AdaptiveRouteScoreComponents, AdaptiveRouteSource, AdaptiveRoutingPlan, AdaptiveRoutingPlanner,
    AdaptiveRoutingPolicy, BudgetedAdaptiveRoutingPlan, ComputeBudgetContext, ComputeBudgetPolicy,
    ComputeBudgetSchedule, ComputeBudgetScheduler, GenerationMetrics, NoironRouter,
    ProfileObservations, ProfileThresholds, ROUTER_DECISION_TRACE_SCHEMA, Route, RouteBudget,
    RouterDecisionTrace, RouterDecisionTraceRow, RouterState,
    RouterThresholdAdjustmentPreviewPlanner, RouterThresholdAdjustmentPreviewPolicy,
    RouterThresholdAdjustmentPreviewReport, RoutingContext, RoutingDecision,
    RoutingTraceReplayFixture, RoutingTraceReplayReport,
};
pub use runtime::{
    ChunkedKvCacheMode, ChunkedKvHookDecision, ChunkedKvHookRecord, ChunkedKvSegment,
    CommandPromptMode, CommandRuntime, CommandTextOutputFilter, CommandWireFormat,
    MistralRsHttpRuntime, MockRustNativeAdapter, ModelRuntime, RuntimeAdapterObservation,
    RuntimeBackend, RuntimeEmbedding, RuntimeError, RuntimeMetadata, RuntimeRequest,
    RuntimeResponse, RuntimeToken, RuntimeTokenId, RustNativeAdapterReport,
    RustNativeAdapterRequest, RustNativeInferenceAdapter, parse_runtime_response_json,
    runtime_request_json,
};
pub use runtime_manifest::{
    RuntimeAssetPaths, RuntimeKvPolicy, RuntimeManifest, RuntimeManifestValidation,
    RuntimeQuantizationPolicy, TransformerRuntimeArchitecture,
    default_transformer_runtime_architecture,
};
pub use rust_validation::{
    RustCodingCommandEvidence, RustCodingRepairCandidateSummary, RustCodingRepairCommandKind,
    RustCodingRepairDecision, RustCodingRepairFailureClass, RustCodingRepairHarness,
    RustCodingRepairInput, RustCodingRepairOutcome, RustCodingRepairPolicy, RustCodingRepairReport,
    RustSnippetCheck, RustSnippetCheckReport, RustSnippetValidator,
};
pub use self_evolution::{
    SelfEvolutionAdmissionEvidence, SelfEvolutionAdmissionGate, SelfEvolutionAdmissionPolicy,
    SelfEvolutionAdmissionReport, SelfEvolutionAdmissionReviewPacketRefs,
    SelfEvolutionExperimentDecision, SelfEvolutionExperimentLedger, SelfEvolutionExperimentRecord,
    SelfEvolutionOperatorApprovalDecision, SelfEvolutionOperatorApprovalEvidence,
    SelfEvolutionOperatorApprovalGate, SelfEvolutionOperatorApprovalLedger,
    SelfEvolutionOperatorApprovalPolicy, SelfEvolutionOperatorApprovalRecord,
    SelfEvolutionOperatorApprovalReport, SelfEvolutionRollbackReplayApplyDecision,
    SelfEvolutionRollbackReplayApplyGate, SelfEvolutionRollbackReplayApplyReport,
    SelfEvolutionRollbackReplayDecision, SelfEvolutionRollbackReplayGate,
    SelfEvolutionRollbackReplayGateReport, SelfEvolutionRollbackReplayItem,
    SelfEvolutionRollbackReplayPlan, SelfEvolutionRollbackReplayPolicy,
    SelfEvolutionValidationEvidence, SelfEvolutionValidationLane,
};
pub use self_evolving_memory::{
    SelfEvolvingEpisodeContext, SelfEvolvingEpisodeInput, SelfEvolvingEpisodeRecord,
    SelfEvolvingHeuristicContext, SelfEvolvingHeuristicInput, SelfEvolvingHeuristicRecord,
    SelfEvolvingMemoryAdmissionCandidatePreview, SelfEvolvingMemoryAdmissionPreview,
    SelfEvolvingMemoryApproval, SelfEvolvingMemoryMaintenancePolicy,
    SelfEvolvingMemoryMaintenanceReport, SelfEvolvingMemoryQuery,
    SelfEvolvingMemoryRetrievalReport, SelfEvolvingMemoryStore, SelfEvolvingMemoryWriteReport,
    ToolReliabilityContext, ToolReliabilityObservationInput, ToolReliabilityObservationRecord,
    ToolReliabilityRecord,
};
pub use semantic_index::{
    DeterministicSemanticEmbeddingProvider, SemanticEmbeddingProvider, SemanticIndex,
    SemanticIndexLane, SemanticIndexMatch, SemanticIndexQuery, SemanticIndexRecord,
    SemanticIndexRetrievalReport, SemanticIndexSkip, SemanticVectorCache,
    SemanticVectorCacheBuildReport, SemanticVectorCacheKey, SemanticVectorCacheRecord,
    SemanticVectorCacheSkippedRecord,
};
pub use session_state::{
    SessionAnchorKind, SessionReplayPlanner, SessionReplayPreview, SessionRuntimeProfile,
    SessionStateAnchor, SessionStateDecodeError, SessionStateInput, SessionStateReadReport,
    SessionStateRecord, SessionStateStore, SessionStateWritePolicy, SessionStateWriteReport,
    SessionTurnDigest, SessionTurnRole,
};
pub use state_inspect::{
    StateExperienceHygieneFinding, StateExperienceIndexFinding, StateExperienceSummary,
    StateInspectionDeviceGateReport, StateInspectionGate, StateInspectionGateReport,
    StateInspectionMatrixGate, StateInspectionMatrixGateReport, StateInspectionReport,
    StateMemorySummary, StateMemoryVectorDimensions,
};
pub use tenant_scope::{
    TenantAccessDecision, TenantAccessKind, TenantIsolationAuditEvent, TenantIsolationGate,
    TenantIsolationReport, TenantMigrationAction, TenantMigrationPlan, TenantMigrationRecord,
    TenantResourceLane, TenantScope, TenantScopedKey, TenantScopedKvReadReport,
    TenantScopedKvWriteReport, tenant_scoped_delete, tenant_scoped_get, tenant_scoped_put,
};
pub use tiered_cache::{
    MemoryPlacement, MemoryTier, TierCounts, TierMigration, TierMigrationAction, TieredCachePlan,
    TieredCacheScheduler,
};
pub use token_stream::{TokenObservation, TokenStreamMonitor, TokenWindowReport};
pub use toolsmith::{
    ToolBlueprint, ToolBuildStatus, ToolIntent, ToolsmithInput, ToolsmithPlan, ToolsmithPlanner,
};
pub use trace::{
    SelfEvolutionOperatorApprovalServiceCounters, TraceSchemaGateReport,
    append_business_contract_trace_jsonl, append_improvement_corpus_trace_jsonl,
    append_rust_check_trace_jsonl, append_self_evolution_admission_trace_jsonl,
    append_self_evolution_experiment_trace_jsonl,
    append_self_evolution_operator_approval_trace_jsonl,
    append_self_evolution_rollback_replay_apply_trace_jsonl,
    append_self_evolution_rollback_replay_gate_trace_jsonl,
    append_self_evolution_rollback_replay_trace_jsonl, append_trace_jsonl,
    append_trace_jsonl_with_case, business_contract_trace_json_line, evaluate_trace_schema_jsonl,
    evaluate_trace_schema_line, improvement_corpus_trace_json_line, rust_check_trace_json_line,
    self_evolution_admission_trace_json_line, self_evolution_experiment_trace_json_line,
    self_evolution_operator_approval_trace_json_line,
    self_evolution_rollback_replay_apply_trace_json_line,
    self_evolution_rollback_replay_gate_trace_json_line,
    self_evolution_rollback_replay_trace_json_line, trace_json_line, trace_json_line_with_case,
};
pub use transformer::{
    AttentionKind, TransformerLayerPlan, TransformerPlanCounts, TransformerPlanner,
    TransformerRefactorPlan, TransformerTemplate, TransformerTemplateKind,
};
