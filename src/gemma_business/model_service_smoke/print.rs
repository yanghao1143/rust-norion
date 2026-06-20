mod contract;
mod evidence;
mod summary;

use super::case_flow::ModelServiceCaseRun;
use super::evidence::{InspectEvidence, ReplayEvidence};
use crate::Args;
use contract::{print_business_contract_evidence, print_case_summaries, print_contract_summary};
use evidence::{
    print_runtime_audit, print_rust_check_evidence, print_self_improve_evidence,
    print_state_evidence,
};
use summary::{print_failures, print_gate_summary, print_http_summary};

pub(super) struct ModelServiceSmokeReport<'a> {
    pub(super) bind: &'a str,
    pub(super) service_args: &'a Args,
    pub(super) failures: &'a [String],
    pub(super) health_body: &'a str,
    pub(super) self_improve_body: &'a str,
    pub(super) inspect_body: &'a str,
    pub(super) case_run: &'a ModelServiceCaseRun,
    pub(super) replay: &'a ReplayEvidence,
    pub(super) inspect: &'a InspectEvidence,
}

pub(super) fn print_model_service_smoke_report(report: ModelServiceSmokeReport<'_>) {
    print_http_summary(&report);
    print_case_summaries(&report);
    print_contract_summary(&report);
    print_business_contract_evidence(&report);
    print_rust_check_evidence(&report);
    print_self_improve_evidence(&report);
    print_state_evidence(&report);
    print_runtime_audit(&report);
    print_failures(&report);
    print_gate_summary(&report);
}
