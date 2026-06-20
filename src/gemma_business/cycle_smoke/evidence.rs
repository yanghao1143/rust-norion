use super::artifacts::BusinessCycleSmokeMetrics;
use crate::gemma_business::smoke_report::GemmaBusinessCycleCaseResult;
use crate::model_service::http::model_service_http_body;

pub(super) struct BusinessCycleSmokeEvidence<'a> {
    pub(super) health_body: &'a str,
    pub(super) final_cycle_body: &'a str,
    pub(super) metrics: BusinessCycleSmokeMetrics,
}

impl<'a> BusinessCycleSmokeEvidence<'a> {
    pub(super) fn from_run(
        health: &'a str,
        case_results: &'a [GemmaBusinessCycleCaseResult],
    ) -> Self {
        Self {
            health_body: model_service_http_body(health),
            final_cycle_body: case_results
                .last()
                .map(|result| result.body.as_str())
                .unwrap_or(""),
            metrics: BusinessCycleSmokeMetrics::from_cases(case_results),
        }
    }
}
