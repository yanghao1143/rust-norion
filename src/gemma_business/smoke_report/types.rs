use crate::gemma_business::audit::GemmaModelServiceAnswerAudit;

pub(crate) struct GemmaBusinessCycleCaseResult {
    pub(crate) name: &'static str,
    pub(crate) body: String,
    pub(crate) answer: String,
    pub(crate) answer_audit: GemmaModelServiceAnswerAudit,
    pub(crate) runtime_token_count: u64,
    pub(crate) feedback_applied: u64,
    pub(crate) rust_check_feedback_applied: u64,
    pub(crate) checked_trace_lines: u64,
    pub(crate) passed: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct GemmaModelServiceCaseResult {
    pub(crate) name: &'static str,
    pub(crate) experience_id: Option<u64>,
    pub(crate) feedback_memory_ids: Vec<u64>,
    pub(crate) runtime_token_count: u64,
    pub(crate) answer_chars: usize,
    pub(crate) answer_preview: String,
    pub(crate) answer_audit: GemmaModelServiceAnswerAudit,
    pub(crate) generate_ok: bool,
    pub(crate) feedback_ok: bool,
    pub(crate) rust_check_ok: Option<bool>,
}
