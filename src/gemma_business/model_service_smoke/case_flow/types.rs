use crate::gemma_business::smoke_report::GemmaModelServiceCaseResult;

#[derive(Debug, Default)]
pub(in crate::gemma_business::model_service_smoke) struct ModelServiceCaseRun {
    pub(in crate::gemma_business::model_service_smoke) case_results:
        Vec<GemmaModelServiceCaseResult>,
    pub(in crate::gemma_business::model_service_smoke) total_runtime_token_count: u64,
    pub(in crate::gemma_business::model_service_smoke) total_feedback_memory_ids: u64,
    pub(in crate::gemma_business::model_service_smoke) generate_ok_count: usize,
    pub(in crate::gemma_business::model_service_smoke) feedback_ok_count: usize,
    pub(in crate::gemma_business::model_service_smoke) rust_check_expected_count: usize,
    pub(in crate::gemma_business::model_service_smoke) rust_check_ok_count: usize,
}
