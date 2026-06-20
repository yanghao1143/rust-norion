use rust_norion::{
    ExperienceReplayReport, InferenceOutcome, MemoryUpdateReport, RustSnippetCheckReport,
    StateInspectionGateReport, StateInspectionReport, TaskProfile, TraceSchemaGateReport,
};

use super::request::{
    ModelServiceFeedbackRequest, ModelServicePoolDispatchRequest,
    ModelServicePoolStageDispatchRequest, ModelServiceRustCheckRequest,
};

#[derive(Debug)]
pub(crate) struct TimedOutcome {
    pub(crate) outcome: InferenceOutcome,
    pub(crate) elapsed_ms: u128,
}

pub(crate) struct ModelServiceBusinessCycleReport {
    pub(crate) profile: TaskProfile,
    pub(crate) traceable: bool,
    pub(crate) pool_dispatch: Option<ModelServicePoolDispatchRequest>,
    pub(crate) pool_stage_dispatch: Vec<ModelServicePoolStageDispatchRequest>,
    pub(crate) pool_dispatch_forwarded: bool,
    pub(crate) timed: TimedOutcome,
    pub(crate) feedback_request: ModelServiceFeedbackRequest,
    pub(crate) feedback_memory_ids: Vec<u64>,
    pub(crate) feedback_updates: Vec<MemoryUpdateReport>,
    pub(crate) rust_check_request: Option<ModelServiceRustCheckRequest>,
    pub(crate) rust_check_report: Option<RustSnippetCheckReport>,
    pub(crate) rust_check_feedback_request: Option<ModelServiceFeedbackRequest>,
    pub(crate) rust_check_memory_ids: Vec<u64>,
    pub(crate) rust_check_updates: Vec<MemoryUpdateReport>,
    pub(crate) self_improve_enabled: bool,
    pub(crate) self_improve_limit: usize,
    pub(crate) replay_report: Option<ExperienceReplayReport>,
    pub(crate) inspection: StateInspectionReport,
    pub(crate) state_gate_report: Option<StateInspectionGateReport>,
    pub(crate) trace_gate_report: Option<TraceSchemaGateReport>,
}

pub(crate) fn profile_name(profile: TaskProfile) -> &'static str {
    match profile {
        TaskProfile::General => "general",
        TaskProfile::Coding => "coding",
        TaskProfile::Writing => "writing",
        TaskProfile::LongDocument => "long",
    }
}
