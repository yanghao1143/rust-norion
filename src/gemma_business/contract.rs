mod case;
mod experience;
mod note;
mod outcome;
mod record;
mod trace;

use std::path::PathBuf;

use rust_norion::{InferenceOutcome, NoironEngine};

use crate::model_service::types::TimedOutcome;

use super::GemmaModelServiceBusinessCase;
use super::audit::{GemmaModelServiceAnswerAudit, gemma_model_service_business_normalization};
use case::{business_case_by_name, gemma_business_smoke_contract_case};
use outcome::apply_business_contract_normalization_to_timed_outcome;
use record::record_business_contract_evidence;

pub(crate) fn annotate_model_service_business_case_for_timed(
    engine: &mut NoironEngine,
    timed: &mut TimedOutcome,
    case_name: Option<&str>,
    trace_path: Option<&PathBuf>,
) -> std::io::Result<()> {
    let Some(business_case) = case_name.and_then(business_case_by_name) else {
        return Ok(());
    };

    let normalization =
        gemma_model_service_business_normalization(business_case, &timed.outcome.answer);
    apply_business_contract_normalization_to_timed_outcome(timed, &normalization);
    let audit = GemmaModelServiceAnswerAudit::from_case(business_case, &timed.outcome.answer);
    record_business_contract_evidence(
        engine,
        business_case,
        timed.outcome.experience_id,
        &audit,
        &normalization,
        trace_path,
    )?;
    Ok(())
}

pub(crate) fn record_gemma_business_smoke_contract(
    engine: &mut NoironEngine,
    outcome: &InferenceOutcome,
    trace_path: Option<&PathBuf>,
) -> std::io::Result<GemmaModelServiceAnswerAudit> {
    let business_case = gemma_business_smoke_contract_case();
    record_business_contract_audit(
        engine,
        &business_case,
        outcome.experience_id,
        &outcome.answer,
        trace_path,
    )
}

pub(crate) fn record_business_contract_audit(
    engine: &mut NoironEngine,
    business_case: &GemmaModelServiceBusinessCase,
    experience_id: u64,
    answer: &str,
    trace_path: Option<&PathBuf>,
) -> std::io::Result<GemmaModelServiceAnswerAudit> {
    let normalization = gemma_model_service_business_normalization(business_case, answer);
    let audit = GemmaModelServiceAnswerAudit::from_case(business_case, &normalization.answer);
    record_business_contract_evidence(
        engine,
        business_case,
        experience_id,
        &audit,
        &normalization,
        trace_path,
    )?;
    Ok(audit)
}
