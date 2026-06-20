use crate::gemma_business::GEMMA_MODEL_SERVICE_BUSINESS_CASES;
use crate::gemma_business::model_service_smoke::print::ModelServiceSmokeReport;

pub(in crate::gemma_business::model_service_smoke::print) fn print_state_evidence(
    report: &ModelServiceSmokeReport<'_>,
) {
    println!(
        "gemma_model_service_smoke_state: cases={} runtime_tokens={} external_feedbacks={} feedback_memory_updates={} replay_runs={} replay_items={}",
        GEMMA_MODEL_SERVICE_BUSINESS_CASES.len(),
        report.inspect.runtime_tokens,
        report.inspect.evolution_external_feedbacks,
        report.inspect.evolution_external_feedback_memory_updates,
        report.inspect.evolution_replay_runs,
        report.inspect.evolution_replay_items
    );
}
