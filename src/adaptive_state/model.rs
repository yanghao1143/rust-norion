mod ledger;
mod live_inference;
mod state;

pub use ledger::EvolutionLedger;
pub use live_inference::LiveInferenceEvolution;
pub use state::AdaptiveState;

pub(super) fn nonnegative_f32(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}
