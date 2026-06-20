use crate::adaptive_state::EvolutionLedger;

use super::BenchmarkSummary;

impl BenchmarkSummary {
    pub fn evolution_ledger(&self) -> EvolutionLedger {
        self.evolution_ledger
    }
}
