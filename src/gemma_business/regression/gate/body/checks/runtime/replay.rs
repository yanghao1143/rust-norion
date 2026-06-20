use crate::gemma_business::regression::gate::body::evidence::ReportBodyEvidence;
use crate::gemma_business::regression::report_checks::require_report_min_u64;

pub(super) fn require_replay_evidence(evidence: &ReportBodyEvidence, failures: &mut Vec<String>) {
    require_report_min_u64(
        failures,
        "replay_rust_check_passed",
        evidence.replay_rust_check_passed,
        evidence.case_count,
    );
    require_report_min_u64(
        failures,
        "live_memory_feedback_applied",
        evidence.replay_live_memory_feedback_applied,
        1,
    );
    require_report_min_u64(
        failures,
        "live_evolution_items",
        evidence.replay_live_evolution_items,
        1,
    );
    require_report_min_u64(
        failures,
        "checked_lines",
        evidence.checked_trace_lines,
        evidence.case_count.saturating_mul(3),
    );
}
