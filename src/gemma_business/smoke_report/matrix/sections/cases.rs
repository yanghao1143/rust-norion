use crate::gemma_business::smoke_report::types::GemmaBusinessCycleCaseResult;

pub(super) fn all_case_bodies_pass(
    case_results: &[GemmaBusinessCycleCaseResult],
    passed: impl Fn(&str) -> bool,
) -> bool {
    case_results.iter().all(|result| passed(&result.body))
}

pub(super) fn case_bodies_passing(
    case_results: &[GemmaBusinessCycleCaseResult],
    passed: impl Fn(&str) -> bool,
) -> usize {
    case_results
        .iter()
        .filter(|result| passed(&result.body))
        .count()
}
