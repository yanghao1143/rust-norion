pub mod engine;
pub mod hierarchy;
pub mod kv_cache;
pub mod reflection;
pub mod router;

pub use engine::{
    GenerationContext, HeuristicBackend, InferenceBackend, InferenceOutcome, InferenceRequest,
    NoironEngine,
};
pub use hierarchy::{HierarchyController, HierarchyWeights, TaskProfile};
pub use kv_cache::{KvFusionCache, MemoryEntry, MemoryMatch};
pub use reflection::{InferenceDraft, ReasoningStep, ReflectionReport, Reflector};
pub use router::{GenerationMetrics, NoironRouter, Route, RouteBudget, RoutingDecision};
