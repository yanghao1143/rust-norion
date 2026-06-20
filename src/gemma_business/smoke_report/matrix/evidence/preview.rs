use crate::gemma_business::smoke_report::preview::compact_business_answer_preview;
use crate::gemma_business::smoke_report::types::GemmaBusinessCycleCaseResult;

pub(super) fn matrix_answer_preview(case_results: &[GemmaBusinessCycleCaseResult]) -> String {
    case_results
        .iter()
        .map(|result| {
            format!(
                "{}: {}",
                result.name,
                compact_business_answer_preview(&result.answer, 80)
            )
        })
        .collect::<Vec<_>>()
        .join(" | ")
}
