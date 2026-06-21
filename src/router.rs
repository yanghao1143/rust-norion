mod adaptive;
mod adjustment;
mod core;
mod scoring;
mod types;

pub use adaptive::{AdaptiveRoutingPlanner, AdaptiveRoutingPolicy};
pub use adjustment::{
    RouterThresholdAdjustmentPreviewPlanner, RouterThresholdAdjustmentPreviewPolicy,
    RouterThresholdAdjustmentPreviewReport,
};
pub use core::NoironRouter;
pub use types::{
    AdaptiveRouteAction, AdaptiveRouteCandidate, AdaptiveRouteDecision,
    AdaptiveRouteScoreComponents, AdaptiveRouteSource, AdaptiveRoutingPlan, GenerationMetrics,
    ProfileObservations, ProfileThresholds, Route, RouteBudget, RouterState, RoutingContext,
    RoutingDecision,
};

#[cfg(test)]
mod tests;
