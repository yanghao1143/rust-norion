use rust_norion::{
    ExperienceMatch, ExperienceRetrievalReport, TaskProfile, render_experience_hint,
};

use super::super::json::{
    option_f32_service_json, option_str_service_json, service_json_string,
    service_json_string_array, service_u64_array,
};

pub(crate) fn model_service_experience_retrieval_response_json(
    request_id: usize,
    report: &ExperienceRetrievalReport,
    retrieval_elapsed_ms: u128,
    index_context_used: bool,
    index_context_chars: usize,
) -> String {
    format!(
        "{{\"ok\":true,\"request_id\":{},\"retrieval\":{}}}",
        request_id,
        experience_retrieval_report_json(
            report,
            retrieval_elapsed_ms,
            index_context_used,
            index_context_chars
        )
    )
}

fn experience_retrieval_report_json(
    report: &ExperienceRetrievalReport,
    retrieval_elapsed_ms: u128,
    index_context_used: bool,
    index_context_chars: usize,
) -> String {
    format!(
        "{{\"prompt\":{},\"profile\":\"{}\",\"retrieval_elapsed_ms\":{},\"index_context_used\":{},\"index_context_chars\":{},\"total_records\":{},\"requested_limit\":{},\"matches\":{},\"match_count\":{},\"skipped_cross_task_pollution\":{},\"development_evidence_surface_blocked_candidates\":{},\"retrieval_noise_penalized_candidates\":{},\"retrieval_noise_filtered_candidates\":{},\"suppressed_prompt_index_candidates\":{},\"max_retrieval_noise_penalty\":{:.6},\"max_score\":{}}}",
        service_json_string(&report.prompt),
        profile_name(report.profile),
        retrieval_elapsed_ms,
        index_context_used,
        index_context_chars,
        report.total_records,
        report.requested_limit,
        experience_matches_json(&report.matches),
        report.match_count(),
        report.skipped_cross_task_pollution,
        report.development_evidence_surface_blocked_candidates,
        report.retrieval_noise_penalized_candidates,
        report.retrieval_noise_filtered_candidates,
        report.suppressed_prompt_index_candidates,
        report.max_retrieval_noise_penalty,
        option_f32_service_json(report.max_score())
    )
}

fn experience_matches_json(matches: &[ExperienceMatch]) -> String {
    let items = matches
        .iter()
        .map(experience_match_json)
        .collect::<Vec<_>>()
        .join(",");
    format!("[{items}]")
}

fn experience_match_json(item: &ExperienceMatch) -> String {
    format!(
        "{{\"experience_id\":{},\"score\":{:.6},\"quality\":{:.6},\"process_reward\":{:.6},\"reward_action\":\"{}\",\"used_memory_count\":{},\"stored_runtime_kv_memory_ids\":{},\"route_threshold\":{:.6},\"route_attention_tokens\":{},\"route_fast_tokens\":{},\"route_attention_fraction\":{:.6},\"prompt_preview\":{},\"lesson_preview\":{},\"usable_hint_preview\":{},\"gist_hints\":{},\"reflection_issue_codes\":{},\"revision_actions\":{},\"runtime_model\":{},\"runtime_adapter\":{},\"runtime_device\":{},\"runtime_primary_lane\":{},\"runtime_fallback_lane\":{},\"runtime_memory_mode\":{},\"runtime_device_execution_source\":{},\"runtime_forward_energy\":{},\"runtime_kv_influence\":{},\"runtime_uncertainty_perplexity\":{},\"recursive_runtime_calls\":{}}}",
        item.id,
        item.score,
        item.quality,
        item.process_reward,
        item.reward_action.as_str(),
        item.used_memory_count,
        service_u64_array(&item.stored_runtime_kv_memory_ids),
        item.route_threshold,
        item.route_attention_tokens,
        item.route_fast_tokens,
        item.route_attention_fraction,
        service_json_string(&compact_preview(&item.prompt, 220)),
        service_json_string(&compact_preview(&item.lesson, 260)),
        service_json_string(&compact_preview(&render_experience_hint(item), 320)),
        service_json_string_array(&item.gist_hints),
        service_json_string_array(&item.reflection_issue_codes),
        service_json_string_array(&item.revision_actions),
        option_str_service_json(item.runtime_model_id.as_deref()),
        option_str_service_json(item.runtime_selected_adapter.as_deref()),
        option_str_service_json(item.runtime_device_profile.as_deref()),
        option_str_service_json(item.runtime_primary_lane.as_deref()),
        option_str_service_json(item.runtime_fallback_lane.as_deref()),
        option_str_service_json(item.runtime_memory_mode.as_deref()),
        option_str_service_json(item.runtime_device_execution_source.as_deref()),
        option_f32_service_json(item.runtime_forward_energy),
        option_f32_service_json(item.runtime_kv_influence),
        option_f32_service_json(item.runtime_uncertainty_perplexity),
        item.recursive_runtime_calls
            .map(|calls| calls.to_string())
            .unwrap_or_else(|| "null".to_owned())
    )
}

fn compact_preview(value: &str, max_chars: usize) -> String {
    let mut out = String::new();
    for ch in value.chars().take(max_chars) {
        if ch.is_whitespace() {
            out.push(' ');
        } else {
            out.push(ch);
        }
    }
    if value.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}

fn profile_name(profile: TaskProfile) -> &'static str {
    match profile {
        TaskProfile::General => "general",
        TaskProfile::Coding => "coding",
        TaskProfile::Writing => "writing",
        TaskProfile::LongDocument => "long",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_norion::RewardAction;

    #[test]
    fn retrieval_response_exposes_noise_accounting() {
        let report = ExperienceRetrievalReport {
            prompt: "rust loop".to_owned(),
            profile: TaskProfile::Coding,
            total_records: 4,
            requested_limit: 2,
            skipped_cross_task_pollution: 1,
            development_evidence_surface_blocked_candidates: 1,
            retrieval_noise_penalized_candidates: 2,
            retrieval_noise_filtered_candidates: 1,
            suppressed_prompt_index_candidates: 2,
            max_retrieval_noise_penalty: 0.44,
            matches: Vec::new(),
        };

        let json = model_service_experience_retrieval_response_json(9, &report, 37, true, 128);

        assert!(json.contains("\"retrieval_elapsed_ms\":37"));
        assert!(json.contains("\"index_context_used\":true"));
        assert!(json.contains("\"index_context_chars\":128"));
        assert!(json.contains("\"retrieval_noise_penalized_candidates\":2"));
        assert!(json.contains("\"development_evidence_surface_blocked_candidates\":1"));
        assert!(json.contains("\"retrieval_noise_filtered_candidates\":1"));
        assert!(json.contains("\"suppressed_prompt_index_candidates\":2"));
        assert!(json.contains("\"max_retrieval_noise_penalty\":0.440000"));
    }

    #[test]
    fn retrieval_response_uses_hint_preview_for_metadata_lessons() {
        let report = ExperienceRetrievalReport {
            prompt: "rust loop".to_owned(),
            profile: TaskProfile::Coding,
            total_records: 1,
            requested_limit: 1,
            skipped_cross_task_pollution: 0,
            development_evidence_surface_blocked_candidates: 0,
            retrieval_noise_penalized_candidates: 1,
            retrieval_noise_filtered_candidates: 0,
            suppressed_prompt_index_candidates: 1,
            max_retrieval_noise_penalty: 0.07,
            matches: vec![ExperienceMatch {
                id: 7,
                prompt: "Conversation transcript:\nuser: rust loop\nassistant:".to_owned(),
                lesson: "accepted_pattern quality=0.778 overlap=0.640 issues=0 max_severity=info"
                    .to_owned(),
                quality: 0.78,
                score: 0.66,
                gist_hints: vec![
                    "gist title summary=Use a Rust for loop with println output".to_owned(),
                ],
                reflection_issue_codes: Vec::new(),
                revision_actions: Vec::new(),
                process_reward: 0.72,
                reward_action: RewardAction::Reinforce,
                used_memory_count: 2,
                stored_runtime_kv_memory_ids: vec![11, 13],
                route_threshold: 0.42,
                route_attention_tokens: 96,
                route_fast_tokens: 288,
                route_attention_fraction: 0.25,
                runtime_model_id: None,
                runtime_selected_adapter: None,
                runtime_device_profile: None,
                runtime_primary_lane: None,
                runtime_fallback_lane: None,
                runtime_memory_mode: None,
                runtime_device_execution_source: None,
                runtime_forward_energy: None,
                runtime_kv_influence: None,
                runtime_uncertainty_perplexity: None,
                recursive_runtime_calls: None,
            }],
        };

        let json = model_service_experience_retrieval_response_json(10, &report, 3, false, 0);

        assert!(json.contains("\"retrieval_elapsed_ms\":3"));
        assert!(json.contains("\"lesson_preview\":\"accepted_pattern quality=0.778"));
        assert!(json.contains("\"used_memory_count\":2"));
        assert!(json.contains("\"stored_runtime_kv_memory_ids\":[11,13]"));
        assert!(json.contains("\"route_threshold\":0.420000"));
        assert!(json.contains("\"route_attention_tokens\":96"));
        assert!(json.contains("\"route_fast_tokens\":288"));
        assert!(json.contains("\"route_attention_fraction\":0.250000"));
        assert!(json.contains("\"usable_hint_preview\":\"Use a Rust for loop with println output"));
        assert!(!json.contains("\"usable_hint_preview\":\"accepted_pattern"));
    }
}
