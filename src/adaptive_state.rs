mod model;
mod persistence;

pub use model::{
    AdaptiveState, EvolutionLedger, GENOME_EXPRESSED_GENE_CAPACITY, GENOME_PERSISTED_GENE_CAPACITY,
    GeneResidencyReport, GeneUsageRecord, GenomeEvolutionApplyReceipt, GenomeGeneResidency,
    GenomeProfileState, GenomeRuntimeState, LiveInferenceEvolution,
};

#[cfg(test)]
use persistence::parse_evolution_ledger;

#[cfg(test)]
mod tests;
