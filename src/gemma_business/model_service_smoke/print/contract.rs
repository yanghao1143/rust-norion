mod audit;
mod cases;
mod replay;

use super::ModelServiceSmokeReport;

pub(super) fn print_case_summaries(report: &ModelServiceSmokeReport<'_>) {
    cases::print_case_summaries(report);
}

pub(super) fn print_contract_summary(report: &ModelServiceSmokeReport<'_>) {
    audit::print_contract_summary(report);
}

pub(super) fn print_business_contract_evidence(report: &ModelServiceSmokeReport<'_>) {
    replay::print_business_contract_evidence(report);
}
