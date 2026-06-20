mod base;
mod live_evolution;
mod live_memory_feedback;
mod recursive;
mod rollback;
mod rust_check;

use super::super::BenchmarkGate;
use super::super::summary::BenchmarkSummary;
use super::GateFailures;

pub(super) fn evaluate(
    summary: &BenchmarkSummary,
    gate: &BenchmarkGate,
    failures: &mut GateFailures,
) {
    base::evaluate(summary, gate, failures);
    live_memory_feedback::evaluate(summary, gate, failures);
    rust_check::evaluate(summary, gate, failures);
    live_evolution::evaluate(summary, gate, failures);
    recursive::evaluate(summary, gate, failures);
    rollback::evaluate(summary, gate, failures);
}
