mod adaptive;
mod adjustment;
mod budget;
mod core;
mod scoring;
mod trace_replay;
mod types;

pub use adaptive::{AdaptiveRoutingPlanner, AdaptiveRoutingPolicy};
pub use adjustment::{
    RouterThresholdAdjustmentPreviewPlanner, RouterThresholdAdjustmentPreviewPolicy,
    RouterThresholdAdjustmentPreviewReport,
};
pub use budget::{
    BudgetedAdaptiveRoutingPlan, ComputeBudgetContext, ComputeBudgetPolicy, ComputeBudgetSchedule,
    ComputeBudgetScheduler,
};
pub use core::NoironRouter;
pub use trace_replay::{
    ROUTER_DECISION_TRACE_SCHEMA, RouterDecisionTrace, RouterDecisionTraceRow,
    RoutingTraceReplayFixture, RoutingTraceReplayReport,
};
pub use types::{
    AdaptiveRouteAction, AdaptiveRouteCandidate, AdaptiveRouteDecision,
    AdaptiveRouteScoreComponents, AdaptiveRouteSource, AdaptiveRoutingPlan, GenerationMetrics,
    ProfileObservations, ProfileThresholds, Route, RouteBudget, RouterState, RoutingContext,
    RoutingDecision,
};

#[cfg(test)]
mod tests;
