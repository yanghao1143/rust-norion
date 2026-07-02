use crate::app::status_json::json_string_literal;
#[cfg(test)]
use crate::app::status_json::{
    json_bool_field, json_null_field, json_number_field, json_object_field, json_object_keys,
    json_string_field, require_json_bool_equals, require_json_string_equals, required_json_number,
};

#[cfg(test)]
use super::count_contract::{
    validate_declared_matches, validate_match_count, validate_retrieval_cardinality,
};
#[cfg(not(test))]
use super::index_context_contract::RetrievalIndexContextActive;
#[cfg(test)]
use super::index_context_contract::{RetrievalIndexContextActive, validate_index_context_contract};
#[cfg(test)]
use super::quality_contract::validate_quality_metrics;
#[cfg(test)]
use super::score_contract::{validate_empty_match_score_contract, validate_max_score};
use super::{RetrievalPreviewMatch, RetrievalPreviewSummary};

const EXPERIENCE_RETRIEVAL_PREVIEW_JSON_SCHEMA: &str =
    "smartsteam.forge.experience_retrieval_preview.v1";

#[derive(Debug, PartialEq, Eq)]
#[cfg(test)]
pub(in crate::app) struct ExperienceRetrievalPreviewJsonSummary {
    pub(in crate::app) prompt: Option<String>,
    pub(in crate::app) requested_limit: usize,
    pub(in crate::app) total_records: usize,
    pub(in crate::app) match_count: usize,
    pub(in crate::app) declared_matches: usize,
    pub(in crate::app) rendered_matches: usize,
    pub(in crate::app) skipped_cross_task_pollution: Option<usize>,
    pub(in crate::app) retrieval_noise_penalized_candidates: Option<usize>,
    pub(in crate::app) retrieval_noise_filtered_candidates: Option<usize>,
    pub(in crate::app) suppressed_prompt_index_candidates: Option<usize>,
    pub(in crate::app) max_retrieval_noise_penalty: Option<String>,
    pub(in crate::app) max_score: Option<String>,
    pub(in crate::app) index_context_used: Option<bool>,
    pub(in crate::app) index_context_chars: Option<usize>,
    pub(in crate::app) index_context_query_chars: Option<usize>,
    pub(in crate::app) index_context_query_trusted: Option<bool>,
    pub(in crate::app) index_context_query_active_trusted: Option<bool>,
    pub(in crate::app) index_context_query_context_active: Option<RetrievalIndexContextActive>,
    pub(in crate::app) top_match: Option<ExperienceRetrievalPreviewTopMatchJsonSummary>,
}

#[derive(Debug, PartialEq, Eq)]
#[cfg(test)]
pub(in crate::app) struct ExperienceRetrievalPreviewTopMatchJsonSummary {
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
    pub(in crate::app) stored_runtime_kv_memory_ids: Vec<u64>,
}

pub(in crate::app) fn experience_retrieval_preview_event_status(
    raw_summary: &str,
    summary: &RetrievalPreviewSummary,
) -> String {
    [
        raw_summary.trim_end().to_owned(),
        "section=retrieval_preview_json".to_owned(),
        experience_retrieval_preview_json(summary),
    ]
    .join("\n")
}

#[cfg(test)]
pub(in crate::app) fn experience_retrieval_preview_json_summary(
    preview_json: &str,
) -> Result<ExperienceRetrievalPreviewJsonSummary, String> {
    require_json_string_equals(
        preview_json,
        "schema",
        EXPERIENCE_RETRIEVAL_PREVIEW_JSON_SCHEMA,
        "experience retrieval preview JSON schema",
    )?;
    require_json_bool_equals(
        preview_json,
        "read_only",
        true,
        "experience retrieval preview JSON read_only",
    )?;
    require_json_bool_equals(
        preview_json,
        "writes_experience_state",
        false,
        "experience retrieval preview JSON writes_experience_state",
    )?;
    require_json_bool_equals(
        preview_json,
        "streams_model",
        false,
        "experience retrieval preview JSON streams_model",
    )?;

    let prompt = optional_json_string_or_null(preview_json, "prompt")?;
    let requested_limit = required_usize_number(
        preview_json,
        "requested_limit",
        "experience retrieval preview JSON requested_limit",
    )?;
    if requested_limit == 0 {
        return Err(
            "experience retrieval preview JSON requested_limit must be positive".to_owned(),
        );
    }
    let total_records = required_usize_number(
        preview_json,
        "total_records",
        "experience retrieval preview JSON total_records",
    )?;
    let match_count = required_usize_number(
        preview_json,
        "match_count",
        "experience retrieval preview JSON match_count",
    )?;
    let declared_matches = required_usize_number(
        preview_json,
        "declared_matches",
        "experience retrieval preview JSON declared_matches",
    )?;
    let rendered_matches = required_usize_number(
        preview_json,
        "rendered_matches",
        "experience retrieval preview JSON rendered_matches",
    )?;
    let skipped_cross_task_pollution =
        optional_usize_number_or_null(preview_json, "skipped_cross_task_pollution")?;
    let retrieval_noise_penalized_candidates =
        optional_usize_number_or_null(preview_json, "retrieval_noise_penalized_candidates")?;
    let retrieval_noise_filtered_candidates =
        optional_usize_number_or_null(preview_json, "retrieval_noise_filtered_candidates")?;
    let suppressed_prompt_index_candidates =
        optional_usize_number_or_null(preview_json, "suppressed_prompt_index_candidates")?;
    let max_retrieval_noise_penalty =
        optional_json_number_or_null(preview_json, "max_retrieval_noise_penalty")?;
    let max_score = optional_json_number_or_null(preview_json, "max_score")?;
    let index_context_used = optional_json_bool_or_null(preview_json, "index_context_used")?;
    let index_context_chars = optional_usize_number_or_null(preview_json, "index_context_chars")?;
    let index_context_query_chars =
        optional_usize_number_or_null(preview_json, "index_context_query_chars")?;
    let index_context_query_trusted =
        optional_json_bool_or_null(preview_json, "index_context_query_trusted")?;
    let index_context_query_active_trusted =
        optional_json_bool_or_null(preview_json, "index_context_query_active_trusted")?;
    let index_context_query_context_active =
        optional_index_context_active_or_null(preview_json, "index_context_query_context_active")?;
    let top_match = top_match_json_summary(preview_json)?;
    if rendered_matches == 0 && top_match.is_some() {
        return Err(
            "experience retrieval preview JSON top_match requires rendered_matches > 0".to_owned(),
        );
    }
    if rendered_matches > 0 && top_match.is_none() {
        return Err(
            "experience retrieval preview JSON rendered_matches > 0 requires top_match".to_owned(),
        );
    }
    validate_match_count(Some(match_count), Some(declared_matches), rendered_matches)?;
    validate_declared_matches(Some(declared_matches), rendered_matches)?;
    validate_retrieval_cardinality(Some(total_records), requested_limit, Some(match_count))?;
    validate_index_context_contract(
        index_context_used,
        index_context_chars,
        index_context_query_chars,
        index_context_query_trusted,
        index_context_query_active_trusted,
        index_context_query_context_active,
    )?;
    validate_quality_metrics(
        Some(total_records),
        skipped_cross_task_pollution,
        retrieval_noise_penalized_candidates,
        retrieval_noise_filtered_candidates,
        suppressed_prompt_index_candidates,
        max_retrieval_noise_penalty.as_deref(),
    )?;
    validate_empty_match_score_contract(Some(match_count), max_score.as_deref())?;
    validate_top_match_max_score(max_score.as_deref(), top_match.as_ref())?;

    Ok(ExperienceRetrievalPreviewJsonSummary {
        prompt,
        requested_limit,
        total_records,
        match_count,
        declared_matches,
        rendered_matches,
        skipped_cross_task_pollution,
        retrieval_noise_penalized_candidates,
        retrieval_noise_filtered_candidates,
        suppressed_prompt_index_candidates,
        max_retrieval_noise_penalty,
        max_score,
        index_context_used,
        index_context_chars,
        index_context_query_chars,
        index_context_query_trusted,
        index_context_query_active_trusted,
        index_context_query_context_active,
        top_match,
    })
}

fn experience_retrieval_preview_json(summary: &RetrievalPreviewSummary) -> String {
    let total_records = summary.total_records.unwrap_or_default();
    let match_count = summary
        .match_count
        .or(summary.declared_matches)
        .unwrap_or(summary.matches.len());
    let declared_matches = summary.declared_matches.unwrap_or(match_count);
    format!(
        concat!(
            "{{",
            "\"schema\":{},",
            "\"read_only\":true,",
            "\"writes_experience_state\":false,",
            "\"streams_model\":false,",
            "\"prompt\":{},",
            "\"requested_limit\":{},",
            "\"total_records\":{},",
            "\"match_count\":{},",
            "\"declared_matches\":{},",
            "\"rendered_matches\":{},",
            "\"skipped_cross_task_pollution\":{},",
            "\"retrieval_noise_penalized_candidates\":{},",
            "\"retrieval_noise_filtered_candidates\":{},",
            "\"suppressed_prompt_index_candidates\":{},",
            "\"max_retrieval_noise_penalty\":{},",
            "\"max_score\":{},",
            "\"index_context_used\":{},",
            "\"index_context_chars\":{},",
            "\"index_context_query_chars\":{},",
            "\"index_context_query_trusted\":{},",
            "\"index_context_query_active_trusted\":{},",
            "\"index_context_query_context_active\":{},",
            "\"top_match\":{}",
            "}}"
        ),
        json_string_literal(EXPERIENCE_RETRIEVAL_PREVIEW_JSON_SCHEMA),
        optional_string_json(summary.prompt.as_deref()),
        summary.requested_limit,
        total_records,
        match_count,
        declared_matches,
        summary.matches.len(),
        optional_usize_json(summary.skipped_cross_task_pollution),
        optional_usize_json(summary.retrieval_noise_penalized_candidates),
        optional_usize_json(summary.retrieval_noise_filtered_candidates),
        optional_usize_json(summary.suppressed_prompt_index_candidates),
        optional_number_json(summary.max_retrieval_noise_penalty.as_deref()),
        optional_number_json(summary.max_score.as_deref()),
        optional_bool_json(summary.index_context_used),
        optional_usize_json(summary.index_context_chars),
        optional_usize_json(summary.index_context_query_chars),
        optional_bool_json(summary.index_context_query_trusted),
        optional_bool_json(summary.index_context_query_active_trusted),
        optional_index_context_active_json(summary.index_context_query_context_active),
        optional_top_match_json(summary.matches.first()),
    )
}

fn optional_top_match_json(match_item: Option<&RetrievalPreviewMatch>) -> String {
    match_item
        .map(top_match_json)
        .unwrap_or_else(|| "null".to_owned())
}

fn top_match_json(match_item: &RetrievalPreviewMatch) -> String {
    format!(
        concat!(
            "{{",
            "\"id\":{},",
            "\"score\":{},",
            "\"runtime_model\":{},",
            "\"runtime_adapter\":{},",
            "\"runtime_device\":{},",
            "\"runtime_primary_lane\":{},",
            "\"runtime_fallback_lane\":{},",
            "\"runtime_memory_mode\":{},",
            "\"runtime_device_execution_source\":{},",
            "\"runtime_forward_energy\":{},",
            "\"runtime_kv_influence\":{},",
            "\"runtime_uncertainty_perplexity\":{},",
            "\"recursive_runtime_calls\":{},",
            "\"stored_runtime_kv_memory_ids\":{}",
            "}}"
        ),
        json_string_literal(&match_item.id),
        optional_number_json(match_item.score.as_deref()),
        optional_string_json(match_item.runtime_model.as_deref()),
        optional_string_json(match_item.runtime_adapter.as_deref()),
        optional_string_json(match_item.runtime_device.as_deref()),
        optional_string_json(match_item.runtime_primary_lane.as_deref()),
        optional_string_json(match_item.runtime_fallback_lane.as_deref()),
        optional_string_json(match_item.runtime_memory_mode.as_deref()),
        optional_string_json(match_item.runtime_device_execution_source.as_deref()),
        optional_number_json(match_item.runtime_forward_energy.as_deref()),
        optional_number_json(match_item.runtime_kv_influence.as_deref()),
        optional_number_json(match_item.runtime_uncertainty_perplexity.as_deref()),
        optional_usize_json(match_item.recursive_runtime_calls),
        u64_array_json(&match_item.stored_runtime_kv_memory_ids),
    )
}

#[cfg(test)]
fn top_match_json_summary(
    preview_json: &str,
) -> Result<Option<ExperienceRetrievalPreviewTopMatchJsonSummary>, String> {
    if json_null_field(preview_json, "top_match").is_some() {
        return Ok(None);
    }
    let Some(top_match) = json_object_field(preview_json, "top_match") else {
        return Err("experience retrieval preview JSON missing top_match".to_owned());
    };

    Ok(Some(ExperienceRetrievalPreviewTopMatchJsonSummary {
        id: json_string_field(top_match, "id")
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| "experience retrieval preview JSON top_match missing id".to_owned())?,
        score: optional_json_number_or_null(top_match, "score")?,
        runtime_model: optional_json_string_or_null(top_match, "runtime_model")?,
        runtime_adapter: optional_json_string_or_null(top_match, "runtime_adapter")?,
        runtime_device: optional_json_string_or_null(top_match, "runtime_device")?,
        runtime_primary_lane: optional_json_string_or_null(top_match, "runtime_primary_lane")?,
        runtime_fallback_lane: optional_json_string_or_null(top_match, "runtime_fallback_lane")?,
        runtime_memory_mode: optional_json_string_or_null(top_match, "runtime_memory_mode")?,
        runtime_device_execution_source: optional_json_string_or_null(
            top_match,
            "runtime_device_execution_source",
        )?,
        runtime_forward_energy: optional_json_number_or_null(top_match, "runtime_forward_energy")?,
        runtime_kv_influence: optional_json_number_or_null(top_match, "runtime_kv_influence")?,
        runtime_uncertainty_perplexity: optional_json_number_or_null(
            top_match,
            "runtime_uncertainty_perplexity",
        )?,
        recursive_runtime_calls: optional_usize_number_or_null(
            top_match,
            "recursive_runtime_calls",
        )?,
        stored_runtime_kv_memory_ids: json_u64_array_field(
            top_match,
            "stored_runtime_kv_memory_ids",
        )
        .unwrap_or_default(),
    }))
}

#[cfg(test)]
fn validate_top_match_max_score(
    max_score: Option<&str>,
    top_match: Option<&ExperienceRetrievalPreviewTopMatchJsonSummary>,
) -> Result<(), String> {
    let Some(top_match) = top_match else {
        return Ok(());
    };
    let rendered_match = RetrievalPreviewMatch {
        id: top_match.id.clone(),
        score: top_match.score.clone(),
        runtime_model: top_match.runtime_model.clone(),
        runtime_adapter: top_match.runtime_adapter.clone(),
        runtime_device: top_match.runtime_device.clone(),
        runtime_primary_lane: top_match.runtime_primary_lane.clone(),
        runtime_fallback_lane: top_match.runtime_fallback_lane.clone(),
        runtime_memory_mode: top_match.runtime_memory_mode.clone(),
        runtime_device_execution_source: top_match.runtime_device_execution_source.clone(),
        runtime_forward_energy: top_match.runtime_forward_energy.clone(),
        runtime_kv_influence: top_match.runtime_kv_influence.clone(),
        runtime_uncertainty_perplexity: top_match.runtime_uncertainty_perplexity.clone(),
        recursive_runtime_calls: top_match.recursive_runtime_calls,
        stored_runtime_kv_memory_ids: top_match.stored_runtime_kv_memory_ids.clone(),
    };
    validate_max_score(max_score, std::slice::from_ref(&rendered_match))
}

#[cfg(test)]
fn required_usize_number(object: &str, field: &str, label: &str) -> Result<usize, String> {
    required_json_number(object, field, label)?
        .parse::<usize>()
        .map_err(|_| format!("{label} must be usize"))
}

#[cfg(test)]
fn optional_usize_number_or_null(object: &str, field: &str) -> Result<Option<usize>, String> {
    optional_json_number_or_null(object, field)?
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|_| format!("experience retrieval preview JSON {field} must be usize"))
        })
        .transpose()
}

#[cfg(test)]
fn optional_json_string_or_null(object: &str, field: &str) -> Result<Option<String>, String> {
    if json_null_field(object, field).is_some() {
        return Ok(None);
    }
    if let Some(value) = json_string_field(object, field) {
        return Ok(Some(value));
    }
    if json_field_present(object, field) {
        return Err(format!(
            "experience retrieval preview JSON {field} must be string or null"
        ));
    }
    Ok(None)
}

#[cfg(test)]
fn optional_json_bool_or_null(object: &str, field: &str) -> Result<Option<bool>, String> {
    if json_null_field(object, field).is_some() {
        return Ok(None);
    }
    if let Some(value) = json_bool_field(object, field) {
        return Ok(Some(value));
    }
    if json_field_present(object, field) {
        return Err(format!(
            "experience retrieval preview JSON {field} must be bool or null"
        ));
    }
    Ok(None)
}

#[cfg(test)]
fn optional_json_number_or_null(object: &str, field: &str) -> Result<Option<String>, String> {
    if json_null_field(object, field).is_some() {
        return Ok(None);
    }
    if let Some(value) = json_number_field(object, field) {
        return Ok(Some(value));
    }
    if json_field_present(object, field) {
        return Err(format!(
            "experience retrieval preview JSON {field} must be number or null"
        ));
    }
    Ok(None)
}

#[cfg(test)]
fn optional_index_context_active_or_null(
    object: &str,
    field: &str,
) -> Result<Option<RetrievalIndexContextActive>, String> {
    let Some(value) = optional_json_string_or_null(object, field)? else {
        return Ok(None);
    };
    match value.as_str() {
        "latest_trusted_delimited" => Ok(Some(RetrievalIndexContextActive::LatestTrustedDelimited)),
        _ => Err(format!(
            "experience retrieval preview JSON {field} unknown value {value:?}"
        )),
    }
}

#[cfg(test)]
fn json_field_present(object: &str, field: &str) -> bool {
    json_object_keys(object).iter().any(|key| key == field)
}

fn optional_string_json(value: Option<&str>) -> String {
    value
        .map(json_string_literal)
        .unwrap_or_else(|| "null".to_owned())
}

fn optional_number_json(value: Option<&str>) -> String {
    value
        .map(str::to_owned)
        .unwrap_or_else(|| "null".to_owned())
}

fn u64_array_json(values: &[u64]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(u64::to_string)
            .collect::<Vec<_>>()
            .join(",")
    )
}

#[cfg(test)]
fn json_u64_array_field(body: &str, field: &str) -> Option<Vec<u64>> {
    let field = format!("\"{field}\":[");
    let start = body.find(&field)? + field.len();
    let end = body.get(start..)?.find(']')? + start;
    let array = body.get(start..end)?.trim();
    if array.is_empty() {
        return Some(Vec::new());
    }
    array
        .split(',')
        .map(|value| value.trim().parse::<u64>().ok())
        .collect()
}

fn optional_bool_json(value: Option<bool>) -> String {
    value
        .map(|value| if value { "true" } else { "false" }.to_owned())
        .unwrap_or_else(|| "null".to_owned())
}

fn optional_usize_json(value: Option<usize>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

fn optional_index_context_active_json(value: Option<RetrievalIndexContextActive>) -> String {
    value
        .map(|value| match value {
            RetrievalIndexContextActive::LatestTrustedDelimited => {
                json_string_literal("latest_trusted_delimited")
            }
        })
        .unwrap_or_else(|| "null".to_owned())
}
