mod contract;
mod coverage;
mod cycle;
mod runtime;

use contract::require_contract_fields;
use coverage::require_schema_and_case_coverage;
use cycle::{require_cycle_sections, require_empty_failure_fields, require_http_and_gate_fields};
use runtime::require_runtime_and_replay_evidence;

use super::evidence::ReportBodyEvidence;

pub(super) fn require_report_body(
    body: &str,
    evidence: &ReportBodyEvidence,
    failures: &mut Vec<String>,
) {
    require_schema_and_case_coverage(body, evidence, failures);
    require_http_and_gate_fields(body, failures);
    require_contract_fields(body, failures);
    require_cycle_sections(body, failures);
    require_runtime_and_replay_evidence(body, evidence, failures);
    require_empty_failure_fields(body, failures);
}
