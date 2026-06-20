use crate::gemma_business::regression::gate::body::evidence::ReportBodyEvidence;
use crate::gemma_business::regression::report_checks::require_report_min_u64;

pub(super) fn require_feedback_evidence(evidence: &ReportBodyEvidence, failures: &mut Vec<String>) {
    require_report_min_u64(
        failures,
        "feedback.applied",
        evidence.feedback_applied,
        evidence.case_count,
    );
    require_report_min_u64(
        failures,
        "rust_check_feedback_applied",
        evidence.rust_check_feedback_applied,
        evidence.case_count,
    );
    require_report_min_u64(
        failures,
        "external_feedbacks",
        evidence.external_feedbacks,
        evidence.case_count.saturating_mul(2),
    );
    require_report_min_u64(
        failures,
        "feedback_memory_updates",
        evidence.feedback_memory_updates,
        evidence.case_count.saturating_mul(2),
    );
}
