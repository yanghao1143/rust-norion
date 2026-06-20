use crate::router::{Route, RoutingDecision};

use super::model::TokenObservation;

pub(super) fn observe_token(
    decision: RoutingDecision,
    semantic_consistency: f32,
) -> TokenObservation {
    let route_mismatch = match decision.route {
        Route::FastProjection if decision.entropy > 0.68 => 2.2,
        route if route.uses_attention_budget() && decision.entropy < 0.24 => 0.35,
        _ => 0.0,
    };
    let consistency = semantic_consistency.clamp(0.0, 1.0);
    let loss = 2.0 + decision.entropy * 4.0 + (1.0 - consistency) * 8.0 + route_mismatch;

    TokenObservation {
        token: decision.token,
        entropy: decision.entropy,
        route: decision.route,
        loss,
        consistency,
    }
}
