mod core;
mod feedback;
mod replay;

use crate::gemma_business::regression::gate::body::evidence::ReportBodyEvidence;

use core::require_runtime_evidence;
use feedback::require_feedback_evidence;
use replay::require_replay_evidence;

pub(super) fn require_runtime_and_replay_evidence(
    body: &str,
    evidence: &ReportBodyEvidence,
    failures: &mut Vec<String>,
) {
    require_runtime_evidence(body, evidence, failures);
    require_feedback_evidence(evidence, failures);
    require_replay_evidence(evidence, failures);
}
