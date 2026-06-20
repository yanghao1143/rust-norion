use rust_norion::TaskProfile;

use super::types::GemmaModelServiceBusinessCase;

pub const GEMMA_MODEL_SERVICE_BUSINESS_CASES: [GemmaModelServiceBusinessCase; 3] = [
    GemmaModelServiceBusinessCase {
        name: "gemma-service-zh-runtime",
        profile: TaskProfile::Coding,
        prompt: "只复制这一行业务回执，不要添加其他文字：runtime_model_experiences=审计遥测；RuntimeBackend=已处理；model=Gemma；task=Rust。",
        contract_line: "runtime_model_experiences=审计遥测；RuntimeBackend=已处理；model=Gemma；task=Rust。",
        required_answer_signals: &[
            "runtime_model_experiences",
            "runtimebackend",
            "gemma",
            "rust",
        ],
    },
    GemmaModelServiceBusinessCase {
        name: "gemma-service-en-routing",
        profile: TaskProfile::General,
        prompt: "Copy exactly this business receipt and add nothing else: runtime_model_experiences=audit telemetry; Noiron=routing; business=handled.",
        contract_line: "runtime_model_experiences=audit telemetry; Noiron=routing; business=handled.",
        required_answer_signals: &["runtime_model_experiences", "noiron", "routing"],
    },
    GemmaModelServiceBusinessCase {
        name: "gemma-service-rust-feedback",
        profile: TaskProfile::Coding,
        prompt: "Copy exactly this Rust business receipt and add nothing else: runtime_model_experiences=audit telemetry; apply_user_feedback=interface; feedback=applied to memory.",
        contract_line: "runtime_model_experiences=audit telemetry; apply_user_feedback=interface; feedback=applied to memory.",
        required_answer_signals: &[
            "runtime_model_experiences",
            "apply_user_feedback",
            "feedback",
            "to memory",
        ],
    },
];

pub(crate) fn gemma_model_service_business_case_by_name(
    case_name: &str,
) -> Option<&'static GemmaModelServiceBusinessCase> {
    GEMMA_MODEL_SERVICE_BUSINESS_CASES
        .iter()
        .find(|business_case| business_case.name == case_name)
}
