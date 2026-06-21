mod adjustment;
mod core;
mod scoring;
mod types;

pub use adjustment::{
    RouterThresholdAdjustmentPreviewPlanner, RouterThresholdAdjustmentPreviewPolicy,
    RouterThresholdAdjustmentPreviewReport,
};
pub use core::NoironRouter;
pub use types::{
    GenerationMetrics, ProfileObservations, ProfileThresholds, Route, RouteBudget, RouterState,
    RoutingContext, RoutingDecision,
};

#[cfg(test)]
mod tests;
