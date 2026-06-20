mod context;
mod cumulative;
mod current;

use context::AutoReplayTrace;

pub(in crate::trace) fn evaluate_trace_auto_replay(line: &str) -> Vec<String> {
    let trace = AutoReplayTrace::from_line(line);
    let mut failures = Vec::new();

    current::evaluate_current_trace(&mut failures, &trace);
    cumulative::evaluate_cumulative_trace(&mut failures, &trace);

    failures
}
