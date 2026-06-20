use rust_norion::KvQuantBenchmarkGate;

use crate::cli::args::Args;

impl Args {
    pub(crate) fn kv_quant_gate(&self) -> KvQuantBenchmarkGate {
        let mut gate = KvQuantBenchmarkGate::default();

        if let Some(value) = self.kv_quant_max_total_us {
            gate.max_total_elapsed_us = Some(value);
        }

        gate
    }
}
