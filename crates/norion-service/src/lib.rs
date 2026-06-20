//! Protocol and session primitives for Norion service frontends.

mod gate;
mod protocol;
mod session;
mod stream;

pub use gate::{
    FrontendGateSnapshot, GateAdvice, GateAdviceAction, GateDecision, GateSendControl,
    ModelPoolGateSnapshot, ModelPoolRouteSnapshot, ModelPoolRouteStatus, ModelPoolStatus,
    ModelRouteWorkerPickerAction, ModelRouteWorkerSnapshot, ModelWorkerSnapshot,
    SmartSteamCleanRoomHandoffStatusSnapshot, SmartSteamCleanRoomHandoffStatusSource,
    SmartSteamContextHygieneStatusSnapshot, SmartSteamDaemonRoundTransitionStatusSnapshot,
    SmartSteamDaemonRoundTransitionStatusSource, SmartSteamHelperStageRepairState,
    SmartSteamHelperStageRepairStatusSnapshot, SmartSteamHelperStageRepairStatusSource,
    SmartSteamMemoryStartupAdmissionStatusSnapshot,
    SmartSteamMissingHelperRoleRepairProposalStatusSnapshot,
    SmartSteamMissingHelperRoleRepairProposalStatusSource,
    SmartSteamNextRoundDecisionReportStatusSource, SmartSteamNextRoundDecisionStatusSnapshot,
    SmartSteamNextRoundDecisionStatusSource, SmartSteamNextRoundDownstreamConsumerStatusSnapshot,
    SmartSteamNextRoundDownstreamConsumerStatusSource, SmartSteamNextRoundRoundIdEvidenceSnapshot,
    SmartSteamNextRoundRoundIdEvidenceSource, SmartSteamSelfImproveProposalLifecycle,
    SmartSteamSelfImproveProposalMemoryAdmissionStatusSnapshot,
    SmartSteamSelfImproveProposalMemoryAdmissionStatusSource,
    SmartSteamSelfImproveProposalPromptGuidanceSnapshot,
    SmartSteamSelfImproveProposalPromptGuidanceSource, SmartSteamSelfImproveProposalSnapshot,
    SmartSteamSelfImproveProposalStatusSnapshot, SmartSteamSelfImproveProposalStatusSource,
    SmartSteamSelfImproveProposalValidationStatusSnapshot,
    SmartSteamSelfImproveProposalValidationStatusSource, SmartSteamStatusSnapshot,
    SmartSteamStatusSource, SmartSteamWorkerWindowStatusSnapshot,
    SmartSteamWorkerWindowStatusSource,
};
pub use norion_memory::MemoryStartupAdmissionEvidence;
pub use protocol::{
    ChatChunk, ChatChunkDisplaySnapshot, ChatChunkKind, ChatMessage, ChatRequest,
    ChatRequestContextKind, ChatRequestSubmissionSnapshot, ChatRequestWireSnapshot, ChatRole,
    ModelEndpoint, ModelEndpointSelectionKind, ModelRole, RoutingIntent, RoutingIntentWireSnapshot,
    RoutingPreference, StreamState,
};
pub use session::{
    ChatSession, ChatSessionConfig, StartedChatTurn, StreamOutcome, StreamOutcomeSnapshot,
};
pub use stream::{
    BackendSseEvent, CliAdapter, SseAdapter, SseFrameBuffer, StreamAdapter, StreamFrame,
    WebSocketAdapter, apply_backend_event, apply_backend_final_answer, apply_sse_frame, chunk_json,
    close_incomplete_stream, parse_sse_frame, request_json,
};
