mod adapter;
mod device_execution;
mod embedding;
mod forward;
mod kv;
mod sparse;
mod uncertainty;

use super::super::BenchmarkGate;
use super::super::summary::BenchmarkSummary;
use super::GateFailures;

pub(super) fn evaluate(
    summary: &BenchmarkSummary,
    gate: &BenchmarkGate,
    failures: &mut GateFailures,
) {
    sparse::evaluate(summary, gate, failures);
    forward::evaluate(summary, gate, failures);
    uncertainty::evaluate(summary, gate, failures);
    kv::evaluate(summary, gate, failures);
    adapter::evaluate(summary, gate, failures);
    embedding::evaluate(summary, gate, failures);
    device_execution::evaluate(summary, gate, failures);
}
