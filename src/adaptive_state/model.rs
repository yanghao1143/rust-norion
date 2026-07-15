mod ledger;
mod live_inference;
mod state;

pub use ledger::EvolutionLedger;
pub use live_inference::LiveInferenceEvolution;
pub use state::{
    AdaptiveState, GENOME_EXPRESSED_GENE_CAPACITY, GENOME_PERSISTED_GENE_CAPACITY,
    GeneResidencyReport, GeneUsageRecord, GenomeEvolutionApplyReceipt, GenomeGeneResidency,
    GenomeProfileState, GenomeRuntimeState,
};
pub(crate) use state::{all_profiles, profile_slug};

pub(super) fn nonnegative_f32(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}
