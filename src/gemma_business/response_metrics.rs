mod cycle;
mod feedback;
mod report;
mod runtime;
mod trace;

pub(super) use cycle::{
    cycle_external_feedbacks, cycle_feedback_memory_updates, cycle_replay_rust_check_passed,
    cycle_rust_check_passed, live_evolution_items, live_memory_feedback_applied,
};
pub(super) use feedback::{feedback_applied, rust_check_feedback_applied};
pub(super) use report::{
    report_external_feedbacks, report_feedback_memory_updates, report_replay_rust_check_passed,
};
pub(super) use runtime::{runtime_token_count, runtime_tokens};
pub(super) use trace::checked_trace_lines;

#[cfg(test)]
mod tests {
    use super::{
        cycle_external_feedbacks, cycle_feedback_memory_updates, cycle_replay_rust_check_passed,
        cycle_rust_check_passed, live_evolution_items, live_memory_feedback_applied,
        report_external_feedbacks, report_feedback_memory_updates, report_replay_rust_check_passed,
        runtime_token_count, runtime_tokens,
    };

    #[test]
    fn cycle_and_report_metrics_keep_schema_field_names_distinct() {
        let cycle_body = "{\"runtime_tokens\":3,\"runtime_token_count\":7,\"evolution_external_feedbacks\":2,\"evolution_external_feedback_memory_updates\":4,\"evolution_replay_rust_check_passed\":1,\"rust_check_passed\":11,\"live_memory_feedback_applied\":5,\"live_evolution_items\":6}";
        let report_body = "{\"external_feedbacks\":8,\"feedback_memory_updates\":9,\"replay_rust_check_passed\":10}";

        assert_eq!(runtime_tokens(cycle_body), 3);
        assert_eq!(runtime_token_count(cycle_body), 7);
        assert_eq!(cycle_external_feedbacks(cycle_body), 2);
        assert_eq!(cycle_feedback_memory_updates(cycle_body), 4);
        assert_eq!(cycle_replay_rust_check_passed(cycle_body), 1);
        assert_eq!(cycle_rust_check_passed(cycle_body), 11);
        assert_eq!(live_memory_feedback_applied(cycle_body), 5);
        assert_eq!(live_evolution_items(cycle_body), 6);
        assert_eq!(report_external_feedbacks(report_body), 8);
        assert_eq!(report_feedback_memory_updates(report_body), 9);
        assert_eq!(report_replay_rust_check_passed(report_body), 10);
    }
}
