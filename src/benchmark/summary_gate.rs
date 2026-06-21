mod auto_replay;
mod basic;
mod device_drift;
mod evolution_live;
mod evolution_replay;
mod genome;
mod improvement_corpus;
mod memory;
mod routing;
mod runtime;

use super::summary::BenchmarkSummary;
use super::{BenchmarkGate, BenchmarkGateReport};

type GateFailures = Vec<String>;

impl BenchmarkSummary {
    pub fn evaluate(&self, gate: &BenchmarkGate) -> BenchmarkGateReport {
        let mut failures = Vec::new();

        basic::evaluate(self, gate, &mut failures);
        auto_replay::evaluate(self, gate, &mut failures);
        evolution_live::evaluate(self, gate, &mut failures);
        evolution_replay::evaluate(self, gate, &mut failures);
        runtime::evaluate(self, gate, &mut failures);
        routing::evaluate(self, gate, &mut failures);
        genome::evaluate(self, gate, &mut failures);
        improvement_corpus::evaluate(self, gate, &mut failures);
        memory::evaluate(self, gate, &mut failures);
        device_drift::evaluate(self, gate, &mut failures);

        BenchmarkGateReport {
            passed: failures.is_empty(),
            failures,
        }
    }
}
