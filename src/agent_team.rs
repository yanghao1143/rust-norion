mod cross_window;
mod handoff;
mod planner;
mod types;
mod util;

pub use cross_window::{
    CrossWindowBudget, CrossWindowBudgetReport, CrossWindowConflictClass,
    CrossWindowExchangeAggregator, CrossWindowExchangeContext, CrossWindowExchangeReport,
    CrossWindowExperiencePacket, CrossWindowPacketDecision, CrossWindowPacketReview,
};
pub use handoff::{
    AgentHandoffAggregationReport, AgentHandoffContext, AgentHandoffInput, AgentHandoffReview,
    AgentHandoffSanitizer, AgentHandoffTrustState,
};
pub use planner::{AgentTeamInput, AgentTeamPlanner};
pub use types::{
    AgentConflict, AgentEvolutionSignal, AgentIsolationPolicy, AgentMessage, AgentMessageKind,
    AgentNode, AgentRole, AgentTeamAggregation, AgentTeamPlan,
};

#[cfg(test)]
mod tests;
