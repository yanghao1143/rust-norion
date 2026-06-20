mod case;
mod contract;
mod state;
mod summary;

use std::path::PathBuf;

use crate::Args;
use crate::gemma_business::smoke_report::GemmaBusinessCycleCaseResult;
use case::print_case_summaries;
use contract::print_contract_summary;
use state::print_state_summary;
use summary::{print_failures, print_gate_summary, print_http_summary};

pub(super) struct BusinessCycleSmokePrintReport<'a> {
    pub(super) passed: bool,
    pub(super) bind: &'a str,
    pub(super) health_body: &'a str,
    pub(super) case_results: &'a [GemmaBusinessCycleCaseResult],
    pub(super) failures: &'a [String],
    pub(super) service_args: &'a Args,
    pub(super) report_path: Option<&'a PathBuf>,
    pub(super) runtime_token_count: u64,
    pub(super) feedback_applied: u64,
    pub(super) rust_check_feedback_applied: u64,
    pub(super) checked_trace_lines: u64,
    pub(super) passed_cases: usize,
    pub(super) expected_case_count: usize,
    pub(super) final_cycle_body: &'a str,
}

pub(super) fn print_gemma_business_cycle_smoke_report(report: BusinessCycleSmokePrintReport<'_>) {
    print_http_summary(&report);
    print_case_summaries(&report);
    print_contract_summary(&report);
    print_state_summary(&report);
    print_failures(&report);
    print_gate_summary(&report);
}
