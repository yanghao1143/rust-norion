use std::path::PathBuf;

use rust_norion::append_business_contract_trace_jsonl;

use crate::gemma_business::GemmaModelServiceBusinessCase;
use crate::gemma_business::audit::{
    GemmaModelServiceAnswerAudit, GemmaModelServiceBusinessNormalization,
};

pub(super) fn append_business_contract_trace(
    trace_path: Option<&PathBuf>,
    business_case: &GemmaModelServiceBusinessCase,
    experience_id: Option<u64>,
    audit: &GemmaModelServiceAnswerAudit,
    normalization: &GemmaModelServiceBusinessNormalization,
) -> std::io::Result<()> {
    let Some(trace_path) = trace_path else {
        return Ok(());
    };
    append_business_contract_trace_jsonl(
        trace_path,
        business_case.name,
        experience_id,
        audit.required_signals,
        audit.matched_signals,
        &audit.missing_signals,
        audit.has_runtime_model_experiences,
        audit.protocol_leak,
        audit.substituted_runtime_model_experiences,
        audit.evasive_denial,
        audit.handling_signal,
        normalization.raw_audit.passed(),
        normalization.kind.as_str(),
        normalization.kind.response_normalized(),
        normalization.kind.canonical_fallback(),
    )
}
