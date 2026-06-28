use rust_norion::{
    render_experience_hint, ExperienceMatch, ExperienceRetrievalReport, TaskProfile,
};

use super::super::json::{
    option_f32_service_json, option_str_service_json, service_json_string_array,
};

pub(crate) fn model_service_experience_retrieval_response_json(
    request_id: usize,
    report: &ExperienceRetrievalReport,
    index_context_used: bool,
    index_context_chars: usize,
) -> String {
    format!(
        "{{\"ok\":true,\"request_id\":{},\"retrieval\":{}}}",
        request_id,
        experience_retrieval_report_json(report, index_context_used, index_context_chars)
    )
}

fn experience_retrieval_report_json(
    report: &ExperienceRetrievalReport,
    index_context_used: bool,
    index_context_chars: usize,
) -> String {
    format!(
        "{{\"prompt_chars\":{},\"profile\":\"{}\",\"index_context_used\":{},\"index_context_chars\":{},\"total_records\":{},\"requested_limit\":{},\"matches\":{},\"match_count\":{},\"skipped_cross_task_pollution\":{},\"retrieval_noise_penalized_candidates\":{},\"retrieval_noise_filtered_candidates\":{},\"suppressed_prompt_index_candidates\":{},\"max_retrieval_noise_penalty\":{:.6},\"max_score\":{}}}",
        report.prompt.chars().count(),
        profile_name(report.profile),
        index_context_used,
        index_context_chars,
        report.total_records,
        report.requested_limit,
        experience_matches_json(&report.matches),
        report.match_count(),
        report.skipped_cross_task_pollution,
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
        "{{\"experience_id\":{},\"score\":{:.6},\"quality\":{:.6},\"process_reward\":{:.6},\"reward_action\":\"{}\",\"prompt_chars\":{},\"lesson_chars\":{},\"usable_hint_chars\":{},\"gist_hint_count\":{},\"reflection_issue_codes\":{},\"revision_actions\":{},\"runtime_model\":{},\"runtime_adapter\":{},\"runtime_device\":{},\"runtime_primary_lane\":{},\"runtime_fallback_lane\":{},\"runtime_memory_mode\":{},\"runtime_device_execution_source\":{},\"runtime_forward_energy\":{},\"runtime_kv_influence\":{},\"runtime_imported_kv_blocks\":{},\"runtime_weak_kv_imports_skipped\":{},\"runtime_budget_limited_kv_imports_skipped\":{},\"runtime_exported_kv_blocks\":{},\"runtime_kv_segments_included\":{},\"runtime_kv_segments_skipped\":{},\"runtime_kv_segments_rejected\":{},\"runtime_uncertainty_perplexity\":{},\"recursive_runtime_calls\":{},\"live_memory_feedback_reinforced\":{},\"live_memory_feedback_penalized\":{},\"live_memory_feedback_applied\":{},\"live_memory_feedback_removed\":{},\"live_memory_feedback_missing\":{},\"live_memory_feedback_strength_delta\":{:.6},\"reflection_issue_count\":{},\"critical_reflection_issues\":{},\"revision_action_count\":{}}}",
        item.id,
        item.score,
        item.quality,
        item.process_reward,
        item.reward_action.as_str(),
        item.prompt.chars().count(),
        item.lesson.chars().count(),
        render_experience_hint(item).chars().count(),
        item.gist_hints.len(),
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
        item.runtime_imported_kv_blocks,
        item.runtime_weak_kv_imports_skipped,
        item.runtime_budget_limited_kv_imports_skipped,
        item.runtime_exported_kv_blocks,
        item.runtime_kv_segments_included,
        item.runtime_kv_segments_skipped,
        item.runtime_kv_segments_rejected,
        option_f32_service_json(item.runtime_uncertainty_perplexity),
        item.recursive_runtime_calls
            .map(|calls| calls.to_string())
            .unwrap_or_else(|| "null".to_owned()),
        item.live_memory_feedback_reinforced,
        item.live_memory_feedback_penalized,
        item.live_memory_feedback_applied,
        item.live_memory_feedback_removed,
        item.live_memory_feedback_missing,
        item.live_memory_feedback_strength_delta,
        item.reflection_issue_codes.len(),
        item.critical_reflection_issues,
        item.revision_actions.len()
    )
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
            retrieval_noise_penalized_candidates: 2,
            retrieval_noise_filtered_candidates: 1,
            suppressed_prompt_index_candidates: 2,
            max_retrieval_noise_penalty: 0.44,
            matches: Vec::new(),
        };

        let json = model_service_experience_retrieval_response_json(9, &report, true, 128);

        assert!(json.contains("\"prompt_chars\":9"));
        assert!(!json.contains("\"prompt\":\"rust loop\""));
        assert!(json.contains("\"index_context_used\":true"));
        assert!(json.contains("\"index_context_chars\":128"));
        assert!(json.contains("\"retrieval_noise_penalized_candidates\":2"));
        assert!(json.contains("\"retrieval_noise_filtered_candidates\":1"));
        assert!(json.contains("\"suppressed_prompt_index_candidates\":2"));
        assert!(json.contains("\"max_retrieval_noise_penalty\":0.440000"));
    }

    #[test]
    fn retrieval_response_does_not_expose_prompt_lesson_or_hint_previews() {
        let report = ExperienceRetrievalReport {
            prompt: "rust loop".to_owned(),
            profile: TaskProfile::Coding,
            total_records: 1,
            requested_limit: 1,
            skipped_cross_task_pollution: 0,
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
                    "gist title summary=Use a Rust for loop with println output".to_owned()
                ],
                reflection_issue_codes: Vec::new(),
                revision_actions: Vec::new(),
                process_reward: 0.72,
                reward_action: RewardAction::Reinforce,
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
                runtime_imported_kv_blocks: 2,
                runtime_weak_kv_imports_skipped: 3,
                runtime_budget_limited_kv_imports_skipped: 4,
                runtime_exported_kv_blocks: 5,
                runtime_kv_segments_included: 6,
                runtime_kv_segments_skipped: 1,
                runtime_kv_segments_rejected: 2,
                live_memory_feedback_reinforced: 2,
                live_memory_feedback_penalized: 1,
                live_memory_feedback_applied: 2,
                live_memory_feedback_removed: 0,
                live_memory_feedback_missing: 1,
                live_memory_feedback_strength_delta: 0.42,
                critical_reflection_issues: 1,
            }],
        };

        let json = model_service_experience_retrieval_response_json(10, &report, false, 0);

        assert!(json.contains("\"prompt_chars\":9"));
        assert!(json.contains("\"lesson_chars\":71"));
        assert!(json.contains("\"usable_hint_chars\":"));
        assert!(json.contains("\"gist_hint_count\":1"));
        assert!(!json.contains("prompt_preview"));
        assert!(!json.contains("lesson_preview"));
        assert!(!json.contains("usable_hint_preview"));
        assert!(!json.contains("Conversation transcript"));
        assert!(!json.contains("accepted_pattern quality"));
        assert!(!json.contains("Use a Rust for loop with println output"));
        assert!(json.contains("\"runtime_imported_kv_blocks\":2"));
        assert!(json.contains("\"runtime_weak_kv_imports_skipped\":3"));
        assert!(json.contains("\"runtime_budget_limited_kv_imports_skipped\":4"));
        assert!(json.contains("\"runtime_exported_kv_blocks\":5"));
        assert!(json.contains("\"runtime_kv_segments_included\":6"));
        assert!(json.contains("\"runtime_kv_segments_skipped\":1"));
        assert!(json.contains("\"runtime_kv_segments_rejected\":2"));
        assert!(json.contains("\"live_memory_feedback_reinforced\":2"));
        assert!(json.contains("\"live_memory_feedback_penalized\":1"));
        assert!(json.contains("\"live_memory_feedback_applied\":2"));
        assert!(json.contains("\"live_memory_feedback_missing\":1"));
        assert!(json.contains("\"live_memory_feedback_strength_delta\":0.420000"));
        assert!(json.contains("\"critical_reflection_issues\":1"));
    }
}
