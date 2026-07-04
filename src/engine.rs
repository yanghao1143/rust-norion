#[cfg(test)]
use crate::adaptive_state::LiveInferenceEvolution;
#[cfg(test)]
use crate::drift::DriftReport;
#[cfg(test)]
use crate::experience::ExperienceInput;
#[cfg(test)]
use crate::hardware::HardwareSnapshot;
#[cfg(test)]
use crate::hierarchy::{HierarchyWeights, TaskProfile};
#[cfg(test)]
use crate::kv_cache::{KvFusionCache, MemoryCompactionPolicy, MemoryRetentionPolicy};
#[cfg(test)]
use crate::kv_exchange::RuntimeKvBlock;
#[cfg(test)]
use crate::process_reward::{ProcessRewardReport, RewardAction};
#[cfg(test)]
use crate::recursive_scheduler::RecursiveScheduler;
#[cfg(test)]
use crate::reflection::{InferenceDraft, ReasoningStep, ReflectionReport, RuntimeDiagnostics};
#[cfg(test)]
use crate::router::{GenerationMetrics, RouteBudget};
#[cfg(test)]
use crate::token_stream::TokenStreamMonitor;

mod backend;
mod core;
mod embedder;
mod inference;
mod memory_keys;
mod metrics;
mod orchestration;
mod recursive;
mod replay;
mod replay_feedback;
mod state;
mod text;
mod types;

pub use backend::HeuristicBackend;
pub use core::NoironEngine;
pub use orchestration::{
    NoironContextTrace, NoironControlLayerPhenotypeTrace, NoironGateTrace, NoironGenomeTrace,
    NoironKvTrace, NoironOrchestrationStage, NoironOrchestrationStageStatus,
    NoironOrchestrationTrace, NoironReflectionTrace, NoironRouteTrace,
};
pub use types::{
    EmbeddingCallDiagnostics, EmbeddingDiagnostics, EmbeddingSource, GenerationContext,
    InferenceBackend, InferenceOutcome, InferenceRequest, MemoryFeedbackReport,
    RuntimeTokenMetrics,
};

#[cfg(test)]
use embedder::TextEmbedder;
#[cfg(test)]
use metrics::metrics_from_report;
#[cfg(test)]
use replay_feedback::*;

#[cfg(test)]
mod tests;
