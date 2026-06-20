use crate::gemma_business::model_service_smoke::print::ModelServiceSmokeReport;
use crate::model_service::json::service_json_string_array;

pub(super) fn print_contract_summary(report: &ModelServiceSmokeReport<'_>) {
    let contract_matched_signals = report
        .case_run
        .case_results
        .iter()
        .map(|result| result.answer_audit.matched_signals)
        .sum::<usize>();
    let contract_required_signals = report
        .case_run
        .case_results
        .iter()
        .map(|result| result.answer_audit.required_signals)
        .sum::<usize>();
    let contract_passed = report
        .case_run
        .case_results
        .iter()
        .filter(|result| result.answer_audit.passed())
        .count();
    println!(
        "gemma_model_service_smoke_contract_summary: passed={}/{} matched_signals={}/{}",
        contract_passed,
        report.case_run.case_results.len(),
        contract_matched_signals,
        contract_required_signals
    );
    for result in &report.case_run.case_results {
        println!(
            "gemma_model_service_smoke_contract: name={} passed={} required_signals={} matched_signals={} missing_signals={} runtime_model_experiences={} protocol_leak={} substituted_runtime_model_experiences={} evasive_denial={} handling_signal={}",
            result.name,
            result.answer_audit.passed(),
            result.answer_audit.required_signals,
            result.answer_audit.matched_signals,
            service_json_string_array(&result.answer_audit.missing_signals),
            result.answer_audit.has_runtime_model_experiences,
            result.answer_audit.protocol_leak,
            result.answer_audit.substituted_runtime_model_experiences,
            result.answer_audit.evasive_denial,
            result.answer_audit.handling_signal
        );
    }
}
