mod parse;
mod readiness;
mod summary;

#[cfg(test)]
mod tests;

pub(crate) use parse::parse_provider_health;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderHealth {
    pub ok: bool,
    pub service: Option<String>,
    pub requests_seen: Option<String>,
    pub active_engine_requests: Option<String>,
    pub engine_busy: Option<bool>,
    pub active_requests: Vec<ActiveRequest>,
    pub runtime_mode: Option<String>,
    pub gemma_runtime_server: Option<String>,
    pub gemma_runtime_reachable: Option<bool>,
    pub readiness_ok: Option<bool>,
    pub safe_device_ok: Option<bool>,
    pub readiness_failures: Vec<String>,
    pub safe_device_failures: Vec<String>,
    pub device_profile: Option<String>,
    pub device_accelerators: Option<String>,
    pub device_pressure: Option<String>,
    pub device_primary_lane: Option<String>,
    pub device_memory_mode: Option<String>,
    pub device_plan_summary: Option<String>,
    pub device_probe_summary: Option<String>,
    pub readiness_warnings: Vec<String>,
    pub experience_hygiene: ExperienceHygieneHealth,
    pub last_inference: Option<LastInference>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExperienceHygieneHealth {
    pub checked: Option<bool>,
    pub clean: Option<bool>,
    pub findings: Option<String>,
    pub watch: Option<String>,
    pub quarantine_candidates: Option<String>,
    pub legacy_metadata_lessons: Option<String>,
    pub legacy_metadata_without_clean_gist: Option<String>,
    pub repair: ExperienceHygieneRepairHealth,
    pub index: ExperienceIndexHealth,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExperienceIndexHealth {
    pub total_records: Option<String>,
    pub noisy_records: Option<String>,
    pub duplicate_outputs: Option<String>,
    pub quality_score: Option<String>,
    pub retrieval_ready: Option<bool>,
    pub risk_level: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExperienceHygieneRepairHealth {
    pub repairable_legacy_metadata_lessons: Option<String>,
    pub repairable_index_records: Option<String>,
    pub projected_findings_after_repair: Option<String>,
    pub projected_watch_after_repair: Option<String>,
    pub projected_quarantine_candidates_after_repair: Option<String>,
    pub projected_legacy_metadata_lessons_after_repair: Option<String>,
    pub projected_legacy_metadata_without_clean_gist_after_repair: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveRequest {
    pub request_id: Option<String>,
    pub endpoint: Option<String>,
    pub elapsed_ms: Option<String>,
    pub prompt_preview: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LastInference {
    pub endpoint: Option<String>,
    pub elapsed_ms: Option<String>,
    pub runtime_model: Option<String>,
    pub runtime_token_count: Option<String>,
    pub error: Option<String>,
}
