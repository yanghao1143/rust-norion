use self::count_contract::{
    validate_count_evidence_present, validate_declared_matches, validate_match_count,
    validate_retrieval_cardinality, validate_total_records_evidence_present,
};
#[cfg(test)]
pub(in crate::app) use self::error_json::experience_retrieval_error_json_summary;
pub(in crate::app) use self::error_json::experience_retrieval_error_status;
pub(in crate::app) use self::event_json::experience_retrieval_preview_event_status;
#[cfg(test)]
pub(in crate::app) use self::event_json::experience_retrieval_preview_json_summary;
use self::index_context_contract::{RetrievalIndexContextActive, validate_index_context_contract};
use self::parse::{
    optional_bool_line, optional_declared_matches, optional_finite_number_line,
    optional_index_context_query_active_trusted, optional_index_context_query_chars,
    optional_index_context_query_context_active, optional_index_context_query_trusted,
    optional_line_value, optional_usize_line, require_line, required_usize_line,
    retrieval_match_summary,
};
use self::quality_contract::validate_quality_metrics;
use self::score_contract::{
    validate_empty_match_score_contract, validate_max_score, validate_score_order,
};

mod count_contract;
mod error_json;
mod event_json;
mod index_context_contract;
mod parse;
mod quality_contract;
mod score_contract;

#[derive(Debug, PartialEq, Eq)]
pub(in crate::app) struct RetrievalPreviewSummary {
    pub(in crate::app) prompt: Option<String>,
    pub(in crate::app) requested_limit: usize,
    pub(in crate::app) total_records: Option<usize>,
    pub(in crate::app) skipped_cross_task_pollution: Option<usize>,
    pub(in crate::app) retrieval_noise_penalized_candidates: Option<usize>,
    pub(in crate::app) retrieval_noise_filtered_candidates: Option<usize>,
    pub(in crate::app) suppressed_prompt_index_candidates: Option<usize>,
    pub(in crate::app) max_retrieval_noise_penalty: Option<String>,
    pub(in crate::app) index_context_used: Option<bool>,
    pub(in crate::app) index_context_chars: Option<usize>,
    pub(in crate::app) index_context_query_chars: Option<usize>,
    pub(in crate::app) index_context_query_trusted: Option<bool>,
    pub(in crate::app) index_context_query_active_trusted: Option<bool>,
    pub(in crate::app) index_context_query_context_active: Option<RetrievalIndexContextActive>,
    pub(in crate::app) match_count: Option<usize>,
    pub(in crate::app) declared_matches: Option<usize>,
    pub(in crate::app) max_score: Option<String>,
    pub(in crate::app) matches: Vec<RetrievalPreviewMatch>,
}

#[derive(Debug, PartialEq, Eq)]
pub(in crate::app) struct RetrievalPreviewMatch {
    pub(in crate::app) id: String,
    pub(in crate::app) score: Option<String>,
    pub(in crate::app) runtime_model: Option<String>,
    pub(in crate::app) runtime_adapter: Option<String>,
    pub(in crate::app) runtime_device: Option<String>,
    pub(in crate::app) runtime_primary_lane: Option<String>,
    pub(in crate::app) runtime_fallback_lane: Option<String>,
    pub(in crate::app) runtime_memory_mode: Option<String>,
    pub(in crate::app) runtime_device_execution_source: Option<String>,
    pub(in crate::app) runtime_forward_energy: Option<String>,
    pub(in crate::app) runtime_kv_influence: Option<String>,
    pub(in crate::app) runtime_uncertainty_perplexity: Option<String>,
    pub(in crate::app) recursive_runtime_calls: Option<usize>,
}

pub(in crate::app) fn experience_retrieval_preview_summary(
    summary: &str,
) -> Result<RetrievalPreviewSummary, String> {
    let lines = summary.lines().collect::<Vec<_>>();
    require_line(&lines, 0, "Noiron experience retrieval preview")?;
    let prompt = optional_line_value(&lines, "prompt=").map(str::to_owned);
    let requested_limit = required_usize_line(&lines, "requested_limit=")?;
    if requested_limit == 0 {
        return Err("experience retrieval preview requested_limit must be positive".to_owned());
    }
    let total_records = optional_usize_line(&lines, "total_records=")?;
    let skipped_cross_task_pollution =
        optional_usize_line(&lines, "skipped_cross_task_pollution=")?;
    let retrieval_noise_penalized_candidates =
        optional_usize_line(&lines, "retrieval_noise_penalized_candidates=")?;
    let retrieval_noise_filtered_candidates =
        optional_usize_line(&lines, "retrieval_noise_filtered_candidates=")?;
    let suppressed_prompt_index_candidates =
        optional_usize_line(&lines, "suppressed_prompt_index_candidates=")?;
    let max_retrieval_noise_penalty =
        optional_finite_number_line(&lines, "max_retrieval_noise_penalty=")?;
    let index_context_used = optional_bool_line(&lines, "index_context_used=")?;
    let index_context_chars = optional_usize_line(&lines, "index_context_chars=")?;
    let index_context_query_chars = optional_index_context_query_chars(&lines)?;
    let index_context_query_trusted = optional_index_context_query_trusted(&lines)?;
    let index_context_query_active_trusted = optional_index_context_query_active_trusted(&lines)?;
    let index_context_query_context_active = optional_index_context_query_context_active(&lines)?;
    let match_count = optional_usize_line(&lines, "match_count=")?;
    let declared_matches = optional_declared_matches(&lines)?;
    let max_score = optional_finite_number_line(&lines, "max_score=")?;
    validate_index_context_contract(
        index_context_used,
        index_context_chars,
        index_context_query_chars,
        index_context_query_trusted,
        index_context_query_active_trusted,
        index_context_query_context_active,
    )?;
    let matches = lines
        .iter()
        .filter(|line| line.starts_with("match "))
        .map(|line| retrieval_match_summary(line))
        .collect::<Result<Vec<_>, _>>()?;
    validate_match_count(match_count, declared_matches, matches.len())?;
    validate_declared_matches(declared_matches, matches.len())?;
    validate_retrieval_cardinality(
        total_records,
        requested_limit,
        match_count.or(declared_matches),
    )?;
    validate_count_evidence_present(match_count, declared_matches)?;
    validate_total_records_evidence_present(total_records)?;
    validate_quality_metrics(
        total_records,
        skipped_cross_task_pollution,
        retrieval_noise_penalized_candidates,
        retrieval_noise_filtered_candidates,
        suppressed_prompt_index_candidates,
        max_retrieval_noise_penalty.as_deref(),
    )?;
    validate_empty_match_score_contract(match_count.or(declared_matches), max_score.as_deref())?;
    validate_max_score(max_score.as_deref(), &matches)?;
    validate_score_order(&matches)?;

    Ok(RetrievalPreviewSummary {
        prompt,
        requested_limit,
        total_records,
        skipped_cross_task_pollution,
        retrieval_noise_penalized_candidates,
        retrieval_noise_filtered_candidates,
        suppressed_prompt_index_candidates,
        max_retrieval_noise_penalty,
        index_context_used,
        index_context_chars,
        index_context_query_chars,
        index_context_query_trusted,
        index_context_query_active_trusted,
        index_context_query_context_active,
        match_count,
        declared_matches,
        max_score,
        matches,
    })
}

#[cfg(test)]
mod tests;
