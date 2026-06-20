use rust_norion::TaskProfile;

use super::constants::GEMMA_BUSINESS_SMOKE_PROMPT;
use super::types::GemmaModelServiceBusinessCase;

pub fn gemma_business_smoke_case() -> GemmaModelServiceBusinessCase {
    GemmaModelServiceBusinessCase {
        name: "gemma-business-runtime",
        profile: TaskProfile::Coding,
        prompt: GEMMA_BUSINESS_SMOKE_PROMPT,
        contract_line: "runtime_model_experiences=审计遥测；RuntimeBackend=已处理；model=Gemma；task=Rust。",
        required_answer_signals: &[
            "runtime_model_experiences",
            "runtimebackend",
            "gemma",
            "rust",
        ],
    }
}
