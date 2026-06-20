mod audit;
mod ledger;

use crate::gemma_business::model_service_smoke::print::ModelServiceSmokeReport;
use audit::print_contract_audit_evidence;
use ledger::print_contract_replay_evidence;

pub(super) fn print_business_contract_evidence(report: &ModelServiceSmokeReport<'_>) {
    print_contract_audit_evidence(report);
    print_contract_replay_evidence(report);
}
