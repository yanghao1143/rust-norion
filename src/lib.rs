pub mod adaptive_state;
pub mod benchmark;
pub mod disk_kv;
pub mod drift;
pub mod engine;
pub mod experience;
pub mod experience_replay;
pub mod gist_memory;
pub mod hardware;
pub mod hierarchy;
pub mod infini_memory;
pub mod kv_cache;
pub mod kv_exchange;
pub mod kv_quant;
pub mod local_runtime;
pub mod process_reward;
pub mod recursive_scheduler;
pub mod reflection;
pub mod router;
pub mod runtime;
pub mod runtime_manifest;
pub mod state_inspect;
pub mod tiered_cache;
pub mod token_stream;
pub mod trace;
pub mod transformer;

pub use adaptive_state::AdaptiveState;
pub use benchmark::{
    BenchmarkCase, BenchmarkCaseResult, BenchmarkGate, BenchmarkGateReport, BenchmarkSummary,
    KvQuantBenchmarkCaseResult, KvQuantBenchmarkGate, KvQuantBenchmarkGateReport,
    KvQuantBenchmarkSummary, PersistentRoundtripInput, PersistentRoundtripReport,
    default_benchmark_cases,
};
pub use disk_kv::DiskKvStore;
pub use drift::{DriftGuard, DriftInput, DriftReport, DriftSeverity};
pub use engine::{
    GenerationContext, HeuristicBackend, InferenceBackend, InferenceOutcome, InferenceRequest,
    NoironEngine,
};
pub use experience::{ExperienceInput, ExperienceMatch, ExperienceRecord, ExperienceStore};
pub use experience_replay::{
    ExperienceReplayItem, ExperienceReplayPlan, ExperienceReplayPlanner, ExperienceReplayReport,
};
pub use gist_memory::{GistGenerator, GistLevel, GistRecord};
pub use hardware::{
    ComputeLane, DeviceClass, DeviceExecutionPlan, DeviceMemoryMode, DevicePlanGateReport,
    DevicePlanGateRow, DeviceProfileDescriptor, DeviceTier, HardwareAllocator, HardwarePlan,
    HardwareProbe, HardwareSnapshot, MemoryGovernancePlan, RuntimeAdapterHint,
};
pub use hierarchy::{
    HierarchyController, HierarchyState, HierarchyWeights, ProfileHierarchyObservations,
    ProfileHierarchyWeights, TaskProfile,
};
pub use infini_memory::{
    InfiniMemoryCounts, InfiniMemoryItem, InfiniMemoryPlan, InfiniMemoryPlanner, InfiniMemoryScope,
};
pub use kv_cache::{
    KvFusionCache, MemoryCompactionMerge, MemoryCompactionPolicy, MemoryCompactionReport,
    MemoryEntry, MemoryMatch, MemoryRetentionPolicy, RetentionReport,
};
pub use kv_exchange::RuntimeKvBlock;
pub use kv_quant::{QuantizationBits, QuantizationError, QuantizedVector};
pub use local_runtime::LocalTransformerRuntime;
pub use process_reward::{
    ProcessRewardComponents, ProcessRewardInput, ProcessRewardReport, ProcessRewarder, RewardAction,
};
pub use recursive_scheduler::{
    RecursiveChunk, RecursiveExecutionWave, RecursiveMergeRound, RecursiveSchedule,
    RecursiveScheduler,
};
pub use reflection::{
    DraftToken, InferenceDraft, ReasoningStep, ReflectionReport, Reflector, RuntimeDiagnostics,
};
pub use router::{
    GenerationMetrics, NoironRouter, ProfileObservations, ProfileThresholds, Route, RouteBudget,
    RouterState, RoutingContext, RoutingDecision,
};
pub use runtime::{
    CommandPromptMode, CommandRuntime, CommandWireFormat, ModelRuntime, RuntimeAdapterObservation,
    RuntimeBackend, RuntimeEmbedding, RuntimeError, RuntimeMetadata, RuntimeRequest,
    RuntimeResponse, RuntimeToken, RuntimeTokenId, parse_runtime_response_json,
    runtime_request_json,
};
pub use runtime_manifest::{
    RuntimeAssetPaths, RuntimeKvPolicy, RuntimeManifest, RuntimeManifestValidation,
    RuntimeQuantizationPolicy, TransformerRuntimeArchitecture,
};
pub use state_inspect::{
    StateExperienceSummary, StateInspectionReport, StateMemorySummary, StateMemoryVectorDimensions,
};
pub use tiered_cache::{
    MemoryPlacement, MemoryTier, TierCounts, TierMigration, TierMigrationAction, TieredCachePlan,
    TieredCacheScheduler,
};
pub use token_stream::{TokenObservation, TokenStreamMonitor, TokenWindowReport};
pub use trace::{
    TraceSchemaGateReport, append_trace_jsonl, append_trace_jsonl_with_case,
    evaluate_trace_schema_jsonl, evaluate_trace_schema_line, trace_json_line,
    trace_json_line_with_case,
};
pub use transformer::{
    AttentionKind, TransformerLayerPlan, TransformerPlanCounts, TransformerPlanner,
    TransformerRefactorPlan, TransformerTemplate, TransformerTemplateKind,
};
