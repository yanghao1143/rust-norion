use std::path::PathBuf;

use rust_norion::NoironEngine;

use super::experience::annotate_model_service_business_contract_experience;
use super::trace::append_business_contract_trace_to_paths;
use crate::gemma_business::GemmaModelServiceBusinessCase;
use crate::gemma_business::audit::{
    GemmaModelServiceAnswerAudit, GemmaModelServiceBusinessNormalization,
};

pub(super) fn record_business_contract_evidence(
    engine: &mut NoironEngine,
    business_case: &GemmaModelServiceBusinessCase,
    experience_id: u64,
    audit: &GemmaModelServiceAnswerAudit,
    normalization: &GemmaModelServiceBusinessNormalization,
    trace_paths: [Option<&PathBuf>; 2],
) -> std::io::Result<()> {
    annotate_model_service_business_contract_experience(
        engine,
        experience_id,
        business_case.name,
        audit,
        normalization,
    );
    append_business_contract_trace_to_paths(
        trace_paths,
        business_case,
        Some(experience_id),
        audit,
        normalization,
    )
}
