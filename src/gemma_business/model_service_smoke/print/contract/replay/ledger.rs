use crate::gemma_business::model_service_smoke::print::ModelServiceSmokeReport;

pub(super) fn print_contract_replay_evidence(report: &ModelServiceSmokeReport<'_>) {
    let replay = report.replay.business_contract;
    let ledger = report.inspect.business_contract_replay_ledger;
    println!(
        "gemma_model_service_smoke_business_replay: replay_items={} replay_passed={} replay_failed={} replay_raw_passed={} replay_raw_failed={} replay_response_normalized={} replay_sanitized={} replay_canonical_fallbacks={} ledger_items={} ledger_passed={} ledger_failed={} ledger_raw_passed={} ledger_raw_failed={} ledger_response_normalized={} ledger_sanitized={} ledger_canonical_fallbacks={}",
        replay.items,
        replay.passed,
        replay.failed,
        replay.raw_passed,
        replay.raw_failed,
        replay.response_normalized,
        replay.sanitized,
        replay.canonical_fallbacks,
        ledger.items,
        ledger.passed,
        ledger.failed,
        ledger.raw_passed,
        ledger.raw_failed,
        ledger.response_normalized,
        ledger.sanitized,
        ledger.canonical_fallbacks
    );
}
