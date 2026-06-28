mod case;
mod experience;
mod note;
mod outcome;
mod record;
mod trace;

use std::path::PathBuf;

use rust_norion::{InferenceOutcome, NoironEngine};

use crate::model_service::types::TimedOutcome;

use super::audit::{gemma_model_service_business_normalization, GemmaModelServiceAnswerAudit};
use super::GemmaModelServiceBusinessCase;
use case::{business_case_by_name, gemma_business_smoke_contract_case};
use outcome::apply_business_contract_normalization_to_timed_outcome;
use record::record_business_contract_evidence;

pub(crate) fn annotate_model_service_business_case_for_timed_to_paths(
    engine: &mut NoironEngine,
    timed: &mut TimedOutcome,
    case_name: Option<&str>,
    trace_paths: [Option<&PathBuf>; 2],
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
        trace_paths,
    )?;
    Ok(())
}

#[cfg(test)]
pub(crate) fn record_gemma_business_smoke_contract(
    engine: &mut NoironEngine,
    outcome: &InferenceOutcome,
    trace_path: Option<&PathBuf>,
) -> std::io::Result<GemmaModelServiceAnswerAudit> {
    record_gemma_business_smoke_contract_to_paths(engine, outcome, [trace_path, None])
}

pub(crate) fn record_gemma_business_smoke_contract_to_paths(
    engine: &mut NoironEngine,
    outcome: &InferenceOutcome,
    trace_paths: [Option<&PathBuf>; 2],
) -> std::io::Result<GemmaModelServiceAnswerAudit> {
    let business_case = gemma_business_smoke_contract_case();
    record_business_contract_audit_to_paths(
        engine,
        &business_case,
        outcome.experience_id,
        &outcome.answer,
        trace_paths,
    )
}

pub(crate) fn record_business_contract_audit_to_paths(
    engine: &mut NoironEngine,
    business_case: &GemmaModelServiceBusinessCase,
    experience_id: u64,
    answer: &str,
    trace_paths: [Option<&PathBuf>; 2],
) -> std::io::Result<GemmaModelServiceAnswerAudit> {
    let normalization = gemma_model_service_business_normalization(business_case, answer);
    let audit = GemmaModelServiceAnswerAudit::from_case(business_case, &normalization.answer);
    record_business_contract_evidence(
        engine,
        business_case,
        experience_id,
        &audit,
        &normalization,
        trace_paths,
    )?;
    Ok(audit)
}
