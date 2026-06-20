use crate::model_service::types::TimedOutcome;

use crate::gemma_business::audit::GemmaModelServiceBusinessNormalization;

pub(super) fn apply_business_contract_normalization_to_timed_outcome(
    timed: &mut TimedOutcome,
    normalization: &GemmaModelServiceBusinessNormalization,
) {
    if normalization.rewrites_answer(&timed.outcome.answer) {
        timed.outcome.answer = normalization.answer.clone();
        timed.outcome.report.revised_answer = normalization.answer.clone();
    }
}
