#[derive(Debug, Clone)]
pub(crate) struct BackendResult {
    pub(crate) ok: bool,
    pub(crate) answer: String,
    pub(crate) raw_answer: Option<String>,
    pub(crate) enhanced_answer: Option<String>,
    pub(crate) runtime_model: Option<String>,
    pub(crate) elapsed_ms: Option<String>,
    pub(crate) business_cycle_passed: Option<bool>,
    pub(crate) feedback_applied: Option<String>,
    pub(crate) rust_check_passed: Option<bool>,
    pub(crate) self_improve_passed: Option<bool>,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct BackendHealth {
    pub(crate) ok: bool,
    pub(crate) service: Option<String>,
    pub(crate) requests_seen: Option<String>,
    pub(crate) active_engine_requests: Option<String>,
    pub(crate) engine_busy: Option<bool>,
    pub(crate) runtime_mode: Option<String>,
    pub(crate) gemma_runtime_server: Option<String>,
    pub(crate) gemma_runtime_reachable: Option<bool>,
    pub(crate) gemma_runtime_model: Option<String>,
    pub(crate) gemma_runtime_context_window: Option<String>,
    pub(crate) gemma_runtime_train_context_window: Option<String>,
    pub(crate) gemma_runtime_vocab_size: Option<String>,
    pub(crate) gemma_runtime_metadata_error: Option<String>,
    pub(crate) readiness_ok: Option<bool>,
    pub(crate) safe_device_ok: Option<bool>,
    pub(crate) readiness_failures: Vec<String>,
    pub(crate) safe_device_failures: Vec<String>,
    pub(crate) device_primary_lane: Option<String>,
    pub(crate) device_memory_mode: Option<String>,
    pub(crate) experience_hygiene: Option<BackendExperienceHygiene>,
    pub(crate) active_requests: Vec<BackendActiveRequest>,
    pub(crate) last_inference: Option<BackendLastInference>,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct BackendExperienceHygiene {
    pub(crate) experience_file: Option<String>,
    pub(crate) checked: Option<bool>,
    pub(crate) clean: Option<bool>,
    pub(crate) findings: Option<String>,
    pub(crate) quarantine_candidates: Option<String>,
    pub(crate) repairable_legacy_metadata_lessons: Option<String>,
    pub(crate) repairable_index_records: Option<String>,
    pub(crate) index: Option<BackendExperienceIndex>,
}

#[derive(Debug, Clone)]
pub(crate) struct BackendExperienceIndex {
    pub(crate) total_records: Option<String>,
    pub(crate) noisy_records: Option<String>,
    pub(crate) duplicate_outputs: Option<String>,
    pub(crate) quality_score: Option<String>,
    pub(crate) retrieval_ready: Option<bool>,
    pub(crate) risk_level: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct BackendActiveRequest {
    pub(crate) request_id: Option<String>,
    pub(crate) endpoint: Option<String>,
    pub(crate) elapsed_ms: Option<String>,
    pub(crate) prompt_preview: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct BackendLastInference {
    pub(crate) request_id: Option<String>,
    pub(crate) endpoint: Option<String>,
    pub(crate) elapsed_ms: Option<String>,
    pub(crate) runtime_model: Option<String>,
    pub(crate) runtime_token_count: Option<String>,
    pub(crate) quality: Option<String>,
    pub(crate) process_reward: Option<String>,
    pub(crate) action: Option<String>,
    pub(crate) error: Option<String>,
}
