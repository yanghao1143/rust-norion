use std::path::PathBuf;

use rust_norion::append_business_contract_trace_jsonl;

use crate::gemma_business::GemmaModelServiceBusinessCase;
use crate::gemma_business::audit::{
    GemmaModelServiceAnswerAudit, GemmaModelServiceBusinessNormalization,
};

pub(super) fn append_business_contract_trace_to_paths(
    trace_paths: [Option<&PathBuf>; 2],
    business_case: &GemmaModelServiceBusinessCase,
    experience_id: Option<u64>,
    audit: &GemmaModelServiceAnswerAudit,
    normalization: &GemmaModelServiceBusinessNormalization,
) -> std::io::Result<()> {
    for trace_path in trace_paths.into_iter().flatten() {
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
        )?;
    }
    Ok(())
}
