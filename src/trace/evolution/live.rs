mod context;
mod cumulative;
mod current;

use context::LiveEvolutionTrace;

pub(in crate::trace) fn evaluate_trace_live_evolution(line: &str) -> Vec<String> {
    let trace = LiveEvolutionTrace::from_line(line);
    let mut failures = Vec::new();

    current::evaluate_current_trace(&mut failures, &trace);
    cumulative::evaluate_cumulative_trace(&mut failures, &trace);

    failures
}
