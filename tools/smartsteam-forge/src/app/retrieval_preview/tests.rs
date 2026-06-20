use super::*;

#[test]
fn retrieval_preview_summary_projects_index_and_runtime_fields() {
    let summary = concat!(
        "Noiron experience retrieval preview\n",
        "prompt=rust loop\n",
        "profile=coding\n",
        "index_context_used=true\n",
        "index_context_chars=88\n",
        "total_records=10\n",
        "requested_limit=2\n",
        "skipped_cross_task_pollution=4\n",
        "match_count=1\n",
        "max_score=0.9\n",
        "matches=1\n",
        "match id=7 score=0.9 quality=0.8 runtime_model=gemma-3-12b runtime_adapter=llama.cpp runtime_device=metal runtime_primary_lane=quality runtime_fallback_lane=summary runtime_memory_mode=kv runtime_device_execution_source=metal runtime_forward_energy=0.72 runtime_kv_influence=0.61 runtime_uncertainty_perplexity=1.25 recursive_runtime_calls=2\n",
        "index_context_query=used chars=88 trusted=true active_trusted=true context_active=latest_trusted_delimited"
    );

    let parsed = experience_retrieval_preview_summary(summary).unwrap();

    assert_eq!(parsed.prompt.as_deref(), Some("rust loop"));
    assert_eq!(parsed.requested_limit, 2);
    assert_eq!(parsed.total_records, Some(10));
    assert_eq!(parsed.skipped_cross_task_pollution, Some(4));
    assert_eq!(parsed.retrieval_noise_penalized_candidates, None);
    assert_eq!(parsed.retrieval_noise_filtered_candidates, None);
    assert_eq!(parsed.suppressed_prompt_index_candidates, None);
    assert_eq!(parsed.max_retrieval_noise_penalty, None);
    assert_eq!(parsed.index_context_used, Some(true));
    assert_eq!(parsed.index_context_chars, Some(88));
    assert_eq!(parsed.index_context_query_chars, Some(88));
    assert_eq!(parsed.index_context_query_trusted, Some(true));
    assert_eq!(parsed.index_context_query_active_trusted, Some(true));
    assert_eq!(
        parsed.index_context_query_context_active,
        Some(RetrievalIndexContextActive::LatestTrustedDelimited)
    );
    assert_eq!(parsed.match_count, Some(1));
    assert_eq!(parsed.declared_matches, Some(1));
    assert_eq!(parsed.max_score.as_deref(), Some("0.9"));
    assert_eq!(
        parsed.matches,
        vec![RetrievalPreviewMatch {
            id: "7".to_owned(),
            score: Some("0.9".to_owned()),
            runtime_model: Some("gemma-3-12b".to_owned()),
            runtime_adapter: Some("llama.cpp".to_owned()),
            runtime_device: Some("metal".to_owned()),
            runtime_primary_lane: Some("quality".to_owned()),
            runtime_fallback_lane: Some("summary".to_owned()),
            runtime_memory_mode: Some("kv".to_owned()),
            runtime_device_execution_source: Some("metal".to_owned()),
            runtime_forward_energy: Some("0.72".to_owned()),
            runtime_kv_influence: Some("0.61".to_owned()),
            runtime_uncertainty_perplexity: Some("1.25".to_owned()),
            recursive_runtime_calls: Some(2),
        }]
    );
}

#[test]
fn retrieval_preview_event_status_carries_machine_readable_runtime_json() {
    let summary = concat!(
        "Noiron experience retrieval preview\n",
        "prompt=rust loop\n",
        "index_context_used=true\n",
        "index_context_chars=88\n",
        "total_records=10\n",
        "requested_limit=2\n",
        "skipped_cross_task_pollution=4\n",
        "retrieval_noise_penalized_candidates=2\n",
        "retrieval_noise_filtered_candidates=1\n",
        "suppressed_prompt_index_candidates=3\n",
        "max_retrieval_noise_penalty=0.44\n",
        "match_count=1\n",
        "max_score=0.9\n",
        "matches=1\n",
        "match id=7 score=0.9 runtime_model=gemma-3-12b runtime_adapter=llama.cpp runtime_device=metal runtime_primary_lane=quality runtime_fallback_lane=summary runtime_memory_mode=kv runtime_device_execution_source=metal runtime_forward_energy=0.72 runtime_kv_influence=0.61 runtime_uncertainty_perplexity=1.25 recursive_runtime_calls=2\n",
        "index_context_query=used chars=88 trusted=true active_trusted=true context_active=latest_trusted_delimited"
    );
    let parsed = experience_retrieval_preview_summary(summary).unwrap();

    let event_status = experience_retrieval_preview_event_status(summary, &parsed);

    assert!(event_status.starts_with("Noiron experience retrieval preview"));
    assert!(event_status.contains("section=retrieval_preview_json"));
    let preview_json = section_body(&event_status, "section=retrieval_preview_json");
    let json_summary = experience_retrieval_preview_json_summary(preview_json).unwrap();
    assert_eq!(json_summary.prompt.as_deref(), Some("rust loop"));
    assert_eq!(json_summary.requested_limit, 2);
    assert_eq!(json_summary.total_records, 10);
    assert_eq!(json_summary.match_count, 1);
    assert_eq!(json_summary.declared_matches, 1);
    assert_eq!(json_summary.rendered_matches, 1);
    assert_eq!(json_summary.skipped_cross_task_pollution, Some(4));
    assert_eq!(json_summary.retrieval_noise_penalized_candidates, Some(2));
    assert_eq!(json_summary.retrieval_noise_filtered_candidates, Some(1));
    assert_eq!(json_summary.suppressed_prompt_index_candidates, Some(3));
    assert_eq!(
        json_summary.max_retrieval_noise_penalty.as_deref(),
        Some("0.44")
    );
    assert_eq!(json_summary.max_score.as_deref(), Some("0.9"));
    assert_eq!(json_summary.index_context_used, Some(true));
    assert_eq!(json_summary.index_context_chars, Some(88));
    assert_eq!(json_summary.index_context_query_chars, Some(88));
    assert_eq!(json_summary.index_context_query_trusted, Some(true));
    assert_eq!(json_summary.index_context_query_active_trusted, Some(true));
    assert_eq!(
        json_summary.index_context_query_context_active,
        Some(RetrievalIndexContextActive::LatestTrustedDelimited)
    );
    let top_match = json_summary.top_match.unwrap();
    assert_eq!(top_match.id, "7");
    assert_eq!(top_match.score.as_deref(), Some("0.9"));
    assert_eq!(top_match.runtime_model.as_deref(), Some("gemma-3-12b"));
    assert_eq!(top_match.runtime_adapter.as_deref(), Some("llama.cpp"));
    assert_eq!(top_match.runtime_device.as_deref(), Some("metal"));
    assert_eq!(top_match.runtime_primary_lane.as_deref(), Some("quality"));
    assert_eq!(top_match.runtime_fallback_lane.as_deref(), Some("summary"));
    assert_eq!(top_match.runtime_memory_mode.as_deref(), Some("kv"));
    assert_eq!(
        top_match.runtime_device_execution_source.as_deref(),
        Some("metal")
    );
    assert_eq!(top_match.runtime_forward_energy.as_deref(), Some("0.72"));
    assert_eq!(top_match.runtime_kv_influence.as_deref(), Some("0.61"));
    assert_eq!(
        top_match.runtime_uncertainty_perplexity.as_deref(),
        Some("1.25")
    );
    assert_eq!(top_match.recursive_runtime_calls, Some(2));
}

#[test]
fn retrieval_preview_event_status_reports_empty_match_set_as_null_top_match() {
    let summary = "Noiron experience retrieval preview\nprompt=rust loop\ntotal_records=0\nrequested_limit=2\nmatch_count=0\nmatches=none";
    let parsed = experience_retrieval_preview_summary(summary).unwrap();

    let event_status = experience_retrieval_preview_event_status(summary, &parsed);
    let preview_json = section_body(&event_status, "section=retrieval_preview_json");
    let json_summary = experience_retrieval_preview_json_summary(preview_json).unwrap();

    assert_eq!(json_summary.match_count, 0);
    assert_eq!(json_summary.declared_matches, 0);
    assert_eq!(json_summary.rendered_matches, 0);
    assert_eq!(json_summary.top_match, None);
}

#[test]
fn retrieval_preview_event_status_preserves_exponent_runtime_numbers() {
    let summary = concat!(
        "Noiron experience retrieval preview\n",
        "total_records=1\n",
        "requested_limit=1\n",
        "match_count=1\n",
        "matches=1\n",
        "match id=7 score=1.25e-3 runtime_forward_energy=6E+2 runtime_kv_influence=4.2e-1 runtime_uncertainty_perplexity=3e0"
    );
    let parsed = experience_retrieval_preview_summary(summary).unwrap();

    let event_status = experience_retrieval_preview_event_status(summary, &parsed);
    let preview_json = section_body(&event_status, "section=retrieval_preview_json");
    let json_summary = experience_retrieval_preview_json_summary(preview_json).unwrap();
    let top_match = json_summary.top_match.unwrap();

    assert_eq!(top_match.score.as_deref(), Some("1.25e-3"));
    assert_eq!(top_match.runtime_forward_energy.as_deref(), Some("6E+2"));
    assert_eq!(top_match.runtime_kv_influence.as_deref(), Some("4.2e-1"));
    assert_eq!(
        top_match.runtime_uncertainty_perplexity.as_deref(),
        Some("3e0")
    );
}

#[test]
fn retrieval_preview_json_rejects_side_effect_drift() {
    let summary = "Noiron experience retrieval preview\ntotal_records=0\nrequested_limit=2\nmatch_count=0\nmatches=none";
    let parsed = experience_retrieval_preview_summary(summary).unwrap();
    let event_status = experience_retrieval_preview_event_status(summary, &parsed);
    let preview_json = section_body(&event_status, "section=retrieval_preview_json");
    let writes_state = preview_json.replace(
        "\"writes_experience_state\":false",
        "\"writes_experience_state\":true",
    );

    assert!(
        experience_retrieval_preview_json_summary(&writes_state)
            .unwrap_err()
            .contains("writes_experience_state")
    );
}

#[test]
fn retrieval_preview_json_rejects_untrusted_index_context_query() {
    let summary = concat!(
        "Noiron experience retrieval preview\n",
        "index_context_used=true\n",
        "index_context_chars=88\n",
        "total_records=0\n",
        "requested_limit=2\n",
        "match_count=0\n",
        "matches=none\n",
        "index_context_query=used chars=88 trusted=true active_trusted=true context_active=latest_trusted_delimited"
    );
    let parsed = experience_retrieval_preview_summary(summary).unwrap();
    let event_status = experience_retrieval_preview_event_status(summary, &parsed);
    let preview_json = section_body(&event_status, "section=retrieval_preview_json");
    let drifted = preview_json.replace(
        "\"index_context_query_trusted\":true",
        "\"index_context_query_trusted\":false",
    );

    assert!(
        experience_retrieval_preview_json_summary(&drifted)
            .unwrap_err()
            .contains("index_context_query requires trusted=true")
    );
}

#[test]
fn retrieval_preview_json_rejects_index_context_chars_mismatch() {
    let summary = concat!(
        "Noiron experience retrieval preview\n",
        "index_context_used=true\n",
        "index_context_chars=88\n",
        "total_records=0\n",
        "requested_limit=2\n",
        "match_count=0\n",
        "matches=none\n",
        "index_context_query=used chars=88 trusted=true active_trusted=true context_active=latest_trusted_delimited"
    );
    let parsed = experience_retrieval_preview_summary(summary).unwrap();
    let event_status = experience_retrieval_preview_event_status(summary, &parsed);
    let preview_json = section_body(&event_status, "section=retrieval_preview_json");
    let drifted = preview_json.replace(
        "\"index_context_query_chars\":88",
        "\"index_context_query_chars\":64",
    );

    assert!(
        experience_retrieval_preview_json_summary(&drifted)
            .unwrap_err()
            .contains("index context chars mismatch")
    );
}

#[test]
fn retrieval_preview_json_rejects_malformed_optional_number() {
    let summary = concat!(
        "Noiron experience retrieval preview\n",
        "total_records=1\n",
        "requested_limit=1\n",
        "match_count=1\n",
        "matches=1\n",
        "match id=7 runtime_forward_energy=0.72"
    );
    let parsed = experience_retrieval_preview_summary(summary).unwrap();
    let event_status = experience_retrieval_preview_event_status(summary, &parsed);
    let preview_json = section_body(&event_status, "section=retrieval_preview_json");
    let malformed = preview_json.replace(
        "\"runtime_forward_energy\":0.72",
        "\"runtime_forward_energy\":1e",
    );

    assert!(
        experience_retrieval_preview_json_summary(&malformed)
            .unwrap_err()
            .contains("runtime_forward_energy must be number or null")
    );
}

#[test]
fn retrieval_preview_json_rejects_cardinality_drift() {
    let summary = concat!(
        "Noiron experience retrieval preview\n",
        "total_records=10\n",
        "requested_limit=2\n",
        "match_count=1\n",
        "matches=1\n",
        "match id=7 score=0.9"
    );
    let parsed = experience_retrieval_preview_summary(summary).unwrap();
    let event_status = experience_retrieval_preview_event_status(summary, &parsed);
    let preview_json = section_body(&event_status, "section=retrieval_preview_json");
    let drifted = preview_json
        .replace("\"match_count\":1", "\"match_count\":3")
        .replace("\"declared_matches\":1", "\"declared_matches\":3")
        .replace("\"rendered_matches\":1", "\"rendered_matches\":3");

    assert!(
        experience_retrieval_preview_json_summary(&drifted)
            .unwrap_err()
            .contains("exceeds requested_limit")
    );
}

#[test]
fn retrieval_preview_json_rejects_quality_counter_drift() {
    let summary = concat!(
        "Noiron experience retrieval preview\n",
        "total_records=10\n",
        "requested_limit=2\n",
        "match_count=1\n",
        "retrieval_noise_penalized_candidates=2\n",
        "retrieval_noise_filtered_candidates=1\n",
        "matches=1\n",
        "match id=7 score=0.9"
    );
    let parsed = experience_retrieval_preview_summary(summary).unwrap();
    let event_status = experience_retrieval_preview_event_status(summary, &parsed);
    let preview_json = section_body(&event_status, "section=retrieval_preview_json");
    let drifted = preview_json.replace(
        "\"retrieval_noise_filtered_candidates\":1",
        "\"retrieval_noise_filtered_candidates\":3",
    );

    assert!(
        experience_retrieval_preview_json_summary(&drifted)
            .unwrap_err()
            .contains("retrieval_noise_filtered_candidates exceeds")
    );
}

#[test]
fn retrieval_preview_json_rejects_cross_task_pollution_counter_drift() {
    let summary = concat!(
        "Noiron experience retrieval preview\n",
        "total_records=10\n",
        "requested_limit=2\n",
        "skipped_cross_task_pollution=4\n",
        "match_count=1\n",
        "matches=1\n",
        "match id=7 score=0.9"
    );
    let parsed = experience_retrieval_preview_summary(summary).unwrap();
    let event_status = experience_retrieval_preview_event_status(summary, &parsed);
    let preview_json = section_body(&event_status, "section=retrieval_preview_json");
    let drifted = preview_json.replace(
        "\"skipped_cross_task_pollution\":4",
        "\"skipped_cross_task_pollution\":11",
    );

    assert!(
        experience_retrieval_preview_json_summary(&drifted)
            .unwrap_err()
            .contains("skipped_cross_task_pollution exceeds total_records")
    );
}

#[test]
fn retrieval_preview_json_rejects_empty_match_set_with_max_score() {
    let summary = "Noiron experience retrieval preview\ntotal_records=0\nrequested_limit=2\nmatch_count=0\nmatches=none";
    let parsed = experience_retrieval_preview_summary(summary).unwrap();
    let event_status = experience_retrieval_preview_event_status(summary, &parsed);
    let preview_json = section_body(&event_status, "section=retrieval_preview_json");
    let drifted = preview_json.replace("\"max_score\":null", "\"max_score\":0.1");

    assert!(
        experience_retrieval_preview_json_summary(&drifted)
            .unwrap_err()
            .contains("empty match set forbids max_score")
    );
}

#[test]
fn retrieval_preview_json_rejects_max_score_below_top_match() {
    let summary = concat!(
        "Noiron experience retrieval preview\n",
        "total_records=1\n",
        "requested_limit=2\n",
        "match_count=1\n",
        "max_score=0.9\n",
        "matches=1\n",
        "match id=7 score=0.9"
    );
    let parsed = experience_retrieval_preview_summary(summary).unwrap();
    let event_status = experience_retrieval_preview_event_status(summary, &parsed);
    let preview_json = section_body(&event_status, "section=retrieval_preview_json");
    let drifted = preview_json.replace("\"max_score\":0.9", "\"max_score\":0.1");

    assert!(
        experience_retrieval_preview_json_summary(&drifted)
            .unwrap_err()
            .contains("max_score below rendered match score")
    );
}

#[test]
fn retrieval_preview_summary_projects_quality_noise_metrics() {
    let summary = concat!(
        "Noiron experience retrieval preview\n",
        "total_records=10\n",
        "requested_limit=2\n",
        "match_count=1\n",
        "retrieval_noise_penalized_candidates=2\n",
        "retrieval_noise_filtered_candidates=1\n",
        "suppressed_prompt_index_candidates=3\n",
        "max_retrieval_noise_penalty=0.44\n",
        "matches=1\n",
        "match id=7 score=0.8"
    );

    let parsed = experience_retrieval_preview_summary(summary).unwrap();

    assert_eq!(parsed.retrieval_noise_penalized_candidates, Some(2));
    assert_eq!(parsed.retrieval_noise_filtered_candidates, Some(1));
    assert_eq!(parsed.suppressed_prompt_index_candidates, Some(3));
    assert_eq!(parsed.max_retrieval_noise_penalty.as_deref(), Some("0.44"));
}

fn section_body<'a>(text: &'a str, section: &str) -> &'a str {
    text.lines()
        .skip_while(|line| *line != section)
        .nth(1)
        .expect("section should include a body line")
}

#[test]
fn retrieval_preview_summary_rejects_negative_noise_penalty() {
    let summary = concat!(
        "Noiron experience retrieval preview\n",
        "total_records=0\n",
        "requested_limit=2\n",
        "match_count=0\n",
        "max_retrieval_noise_penalty=-0.01\n",
        "matches=none"
    );

    assert!(
        experience_retrieval_preview_summary(summary)
            .unwrap_err()
            .contains("max_retrieval_noise_penalty must be non-negative")
    );
}

#[test]
fn retrieval_preview_summary_rejects_noise_filtered_above_penalized() {
    let summary = concat!(
        "Noiron experience retrieval preview\n",
        "total_records=4\n",
        "requested_limit=2\n",
        "match_count=0\n",
        "retrieval_noise_penalized_candidates=1\n",
        "retrieval_noise_filtered_candidates=2\n",
        "matches=none"
    );

    assert!(
        experience_retrieval_preview_summary(summary)
            .unwrap_err()
            .contains("retrieval_noise_filtered_candidates exceeds")
    );
}

#[test]
fn retrieval_preview_summary_rejects_quality_counter_above_total_records() {
    let summary = concat!(
        "Noiron experience retrieval preview\n",
        "total_records=1\n",
        "requested_limit=2\n",
        "match_count=0\n",
        "suppressed_prompt_index_candidates=2\n",
        "matches=none"
    );

    assert!(
        experience_retrieval_preview_summary(summary)
            .unwrap_err()
            .contains("suppressed_prompt_index_candidates exceeds total_records")
    );
}

#[test]
fn retrieval_preview_summary_rejects_cross_task_pollution_above_total_records() {
    let summary = concat!(
        "Noiron experience retrieval preview\n",
        "total_records=1\n",
        "requested_limit=2\n",
        "match_count=0\n",
        "skipped_cross_task_pollution=2\n",
        "matches=none"
    );

    assert!(
        experience_retrieval_preview_summary(summary)
            .unwrap_err()
            .contains("skipped_cross_task_pollution exceeds total_records")
    );
}

#[test]
fn retrieval_preview_summary_rejects_positive_noise_penalty_without_penalized_candidates() {
    let summary = concat!(
        "Noiron experience retrieval preview\n",
        "total_records=4\n",
        "requested_limit=2\n",
        "match_count=0\n",
        "retrieval_noise_penalized_candidates=0\n",
        "max_retrieval_noise_penalty=0.12\n",
        "matches=none"
    );

    assert!(
        experience_retrieval_preview_summary(summary)
            .unwrap_err()
            .contains("requires retrieval_noise_penalized_candidates > 0")
    );
}

#[test]
fn retrieval_preview_summary_rejects_zero_noise_penalty_with_penalized_candidates() {
    let summary = concat!(
        "Noiron experience retrieval preview\n",
        "total_records=4\n",
        "requested_limit=2\n",
        "match_count=0\n",
        "retrieval_noise_penalized_candidates=1\n",
        "max_retrieval_noise_penalty=0.0\n",
        "matches=none"
    );

    assert!(
        experience_retrieval_preview_summary(summary)
            .unwrap_err()
            .contains("must be positive when retrieval_noise_penalized_candidates is positive")
    );
}

#[test]
fn retrieval_preview_summary_accepts_empty_match_set() {
    let summary = "Noiron experience retrieval preview\nprompt=rust loop\ntotal_records=0\nrequested_limit=2\nmatch_count=0\nmatches=none";

    let parsed = experience_retrieval_preview_summary(summary).unwrap();

    assert_eq!(parsed.match_count, Some(0));
    assert_eq!(parsed.declared_matches, Some(0));
    assert!(parsed.matches.is_empty());
}

#[test]
fn retrieval_preview_summary_accepts_declared_matches_capped_to_render_limit() {
    let summary = concat!(
        "Noiron experience retrieval preview\n",
        "total_records=7\n",
        "requested_limit=7\n",
        "match_count=7\n",
        "matches=7\n",
        "match id=1\n",
        "match id=2\n",
        "match id=3\n",
        "match id=4\n",
        "match id=5"
    );

    let parsed = experience_retrieval_preview_summary(summary).unwrap();

    assert_eq!(parsed.match_count, Some(7));
    assert_eq!(parsed.declared_matches, Some(7));
    assert_eq!(parsed.matches.len(), 5);
}

#[test]
fn retrieval_preview_summary_rejects_missing_limit() {
    let summary = "Noiron experience retrieval preview\nprompt=rust loop";

    assert!(
        experience_retrieval_preview_summary(summary)
            .unwrap_err()
            .contains("missing requested_limit=")
    );
}

#[test]
fn retrieval_preview_summary_rejects_missing_count_evidence() {
    let summary =
        "Noiron experience retrieval preview\ntotal_records=0\nrequested_limit=2\nmatches=none";

    assert!(
        experience_retrieval_preview_summary(summary)
            .unwrap_err()
            .contains("missing match_count=")
    );
}

#[test]
fn retrieval_preview_summary_rejects_missing_total_records_evidence() {
    let summary =
        "Noiron experience retrieval preview\nrequested_limit=2\nmatch_count=0\nmatches=none";

    assert!(
        experience_retrieval_preview_summary(summary)
            .unwrap_err()
            .contains("missing total_records=")
    );
}

#[test]
fn retrieval_preview_summary_rejects_declared_match_count_drift() {
    let summary = concat!(
        "Noiron experience retrieval preview\n",
        "requested_limit=2\n",
        "matches=2\n",
        "match id=7"
    );

    assert!(
        experience_retrieval_preview_summary(summary)
            .unwrap_err()
            .contains("matches count mismatch")
    );
}

#[test]
fn retrieval_preview_summary_rejects_match_count_total_drift() {
    let summary = concat!(
        "Noiron experience retrieval preview\n",
        "requested_limit=2\n",
        "match_count=2\n",
        "matches=1\n",
        "match id=7"
    );

    assert!(
        experience_retrieval_preview_summary(summary)
            .unwrap_err()
            .contains("match_count drift")
    );
}

#[test]
fn retrieval_preview_summary_rejects_match_count_render_drift() {
    let summary = concat!(
        "Noiron experience retrieval preview\n",
        "requested_limit=2\n",
        "match_count=2\n",
        "match id=7"
    );

    assert!(
        experience_retrieval_preview_summary(summary)
            .unwrap_err()
            .contains("match_count mismatch")
    );
}

#[test]
fn retrieval_preview_summary_rejects_match_count_above_requested_limit() {
    let summary = concat!(
        "Noiron experience retrieval preview\n",
        "requested_limit=2\n",
        "match_count=3\n",
        "matches=3\n",
        "match id=7\n",
        "match id=8\n",
        "match id=9"
    );

    assert!(
        experience_retrieval_preview_summary(summary)
            .unwrap_err()
            .contains("exceeds requested_limit")
    );
}

#[test]
fn retrieval_preview_summary_rejects_match_count_above_total_records() {
    let summary = concat!(
        "Noiron experience retrieval preview\n",
        "total_records=1\n",
        "requested_limit=5\n",
        "match_count=2\n",
        "matches=2\n",
        "match id=7\n",
        "match id=8"
    );

    assert!(
        experience_retrieval_preview_summary(summary)
            .unwrap_err()
            .contains("exceeds total_records")
    );
}

#[test]
fn retrieval_preview_summary_rejects_matches_none_with_match_lines() {
    let summary = concat!(
        "Noiron experience retrieval preview\n",
        "requested_limit=2\n",
        "matches=none\n",
        "match id=7"
    );

    assert!(
        experience_retrieval_preview_summary(summary)
            .unwrap_err()
            .contains("matches count mismatch")
    );
}

#[test]
fn retrieval_preview_summary_rejects_empty_match_set_with_max_score() {
    let summary = concat!(
        "Noiron experience retrieval preview\n",
        "total_records=0\n",
        "requested_limit=2\n",
        "match_count=0\n",
        "max_score=0.0\n",
        "matches=none"
    );

    assert!(
        experience_retrieval_preview_summary(summary)
            .unwrap_err()
            .contains("empty match set forbids max_score")
    );
}

#[test]
fn retrieval_preview_summary_rejects_max_score_below_rendered_score() {
    let summary = concat!(
        "Noiron experience retrieval preview\n",
        "total_records=1\n",
        "requested_limit=2\n",
        "match_count=1\n",
        "max_score=0.5\n",
        "matches=1\n",
        "match id=7 score=0.9"
    );

    assert!(
        experience_retrieval_preview_summary(summary)
            .unwrap_err()
            .contains("max_score below rendered match score")
    );
}

#[test]
fn retrieval_preview_summary_rejects_rendered_score_order_drift() {
    let summary = concat!(
        "Noiron experience retrieval preview\n",
        "total_records=3\n",
        "requested_limit=3\n",
        "match_count=3\n",
        "max_score=0.9\n",
        "matches=3\n",
        "match id=7 score=0.9\n",
        "match id=8 score=0.4\n",
        "match id=9 score=0.8"
    );

    assert!(
        experience_retrieval_preview_summary(summary)
            .unwrap_err()
            .contains("score order drift")
    );
}

#[test]
fn retrieval_preview_summary_rejects_non_finite_match_score() {
    let summary = concat!(
        "Noiron experience retrieval preview\n",
        "requested_limit=2\n",
        "match_count=1\n",
        "matches=1\n",
        "match id=7 score=NaN"
    );

    assert!(
        experience_retrieval_preview_summary(summary)
            .unwrap_err()
            .contains("expected finite number")
    );
}

#[test]
fn retrieval_preview_summary_rejects_non_finite_runtime_number() {
    let summary = concat!(
        "Noiron experience retrieval preview\n",
        "total_records=1\n",
        "requested_limit=1\n",
        "match_count=1\n",
        "matches=1\n",
        "match id=7 score=0.9 runtime_forward_energy=NaN"
    );

    assert!(
        experience_retrieval_preview_summary(summary)
            .unwrap_err()
            .contains("expected finite number for runtime_forward_energy")
    );
}

#[test]
fn retrieval_preview_summary_rejects_non_usize_recursive_runtime_calls() {
    let summary = concat!(
        "Noiron experience retrieval preview\n",
        "total_records=1\n",
        "requested_limit=1\n",
        "match_count=1\n",
        "matches=1\n",
        "match id=7 score=0.9 recursive_runtime_calls=NaN"
    );

    assert!(
        experience_retrieval_preview_summary(summary)
            .unwrap_err()
            .contains("expected usize for recursive_runtime_calls=")
    );
}

#[test]
fn retrieval_preview_summary_rejects_dirty_index_context_evidence() {
    let summary = concat!(
        "Noiron experience retrieval preview\n",
        "requested_limit=2\n",
        "index_context_used=true\n",
        "index_context_chars=0"
    );

    assert!(
        experience_retrieval_preview_summary(summary)
            .unwrap_err()
            .contains("index_context_used=true requires index_context_chars")
    );
}

#[test]
fn retrieval_preview_summary_rejects_unused_index_context_with_chars() {
    let summary = concat!(
        "Noiron experience retrieval preview\n",
        "requested_limit=2\n",
        "index_context_used=false\n",
        "index_context_chars=88"
    );

    assert!(
        experience_retrieval_preview_summary(summary)
            .unwrap_err()
            .contains("index_context_used=false forbids index_context_chars")
    );
}

#[test]
fn retrieval_preview_summary_rejects_index_context_query_provider_drift() {
    let summary = concat!(
        "Noiron experience retrieval preview\n",
        "requested_limit=2\n",
        "index_context_used=false\n",
        "index_context_query=used chars=88 trusted=true active_trusted=true context_active=latest_trusted_delimited"
    );

    assert!(
        experience_retrieval_preview_summary(summary)
            .unwrap_err()
            .contains("index_context_query requires index_context_used not false")
    );
}

#[test]
fn retrieval_preview_summary_rejects_index_context_chars_mismatch() {
    let summary = concat!(
        "Noiron experience retrieval preview\n",
        "requested_limit=2\n",
        "index_context_used=true\n",
        "index_context_chars=64\n",
        "index_context_query=used chars=88 trusted=true active_trusted=true context_active=latest_trusted_delimited"
    );

    assert!(
        experience_retrieval_preview_summary(summary)
            .unwrap_err()
            .contains("index context chars mismatch")
    );
}

#[test]
fn retrieval_preview_summary_rejects_untrusted_index_context_query() {
    let summary = concat!(
        "Noiron experience retrieval preview\n",
        "requested_limit=2\n",
        "index_context_query=used chars=88 trusted=false active_trusted=false context_active=latest_trusted_delimited"
    );

    assert!(
        experience_retrieval_preview_summary(summary)
            .unwrap_err()
            .contains("requires trusted=true")
    );
}

#[test]
fn retrieval_preview_summary_rejects_missing_index_context_query_trust_contract() {
    let summary = concat!(
        "Noiron experience retrieval preview\n",
        "requested_limit=2\n",
        "index_context_query=used chars=88"
    );

    assert!(
        experience_retrieval_preview_summary(summary)
            .unwrap_err()
            .contains("missing trusted")
    );
}

#[test]
fn retrieval_preview_summary_rejects_missing_index_context_query_active_trusted() {
    let summary = concat!(
        "Noiron experience retrieval preview\n",
        "requested_limit=2\n",
        "index_context_query=used chars=88 trusted=true context_active=latest_trusted_delimited"
    );

    assert!(
        experience_retrieval_preview_summary(summary)
            .unwrap_err()
            .contains("missing active_trusted")
    );
}

#[test]
fn retrieval_preview_summary_rejects_match_without_id() {
    let summary = "Noiron experience retrieval preview\nrequested_limit=2\nmatch score=0.9 runtime_model=gemma-3-12b";

    assert!(
        experience_retrieval_preview_summary(summary)
            .unwrap_err()
            .contains("match missing id")
    );
}
