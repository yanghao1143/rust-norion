use rust_norion::{MemoryUpdateReport, StateInspectionReport};

use super::super::feedback::ModelServiceExperienceFeedbackUpdate;
use super::super::json::{
    option_u64_service_json, service_json_string, service_memory_update_array, service_u64_array,
};
use super::super::request::ModelServiceFeedbackRequest;
use super::state::model_service_state_json;
use super::update_stats::{
    memory_update_applied_count, memory_update_missing_count, memory_update_removed_count,
    memory_update_strength_delta,
};

pub(crate) fn model_service_feedback_response_json(
    request_id: usize,
    request: &ModelServiceFeedbackRequest,
    memory_ids: &[u64],
    updates: &[MemoryUpdateReport],
    experience_update: Option<&ModelServiceExperienceFeedbackUpdate>,
    inspection: &StateInspectionReport,
) -> String {
    let experience_update = experience_update
        .map(|update| {
            format!(
                "{{\"applied\":{},\"reward_before\":{:.6},\"reward_after\":{:.6},\"reward_delta\":{:.6}}}",
                update.applied,
                update.reward_before,
                update.reward_after,
                update.reward_delta,
            )
        })
        .unwrap_or_else(|| "null".to_owned());
    let source = request
        .source
        .as_deref()
        .map(service_json_string)
        .unwrap_or_else(|| "null".to_owned());
    let evidence = request
        .evidence
        .as_deref()
        .map(service_json_string)
        .unwrap_or_else(|| "null".to_owned());
    format!(
        "{{\"ok\":true,\"request_id\":{},\"feedback\":{{\"action\":\"{}\",\"amount\":{:.6},\"experience_id\":{},\"memory_id\":{},\"source\":{},\"evidence\":{},\"memory_ids\":{},\"applied\":{},\"missing\":{},\"removed\":{},\"strength_delta\":{:.6},\"updates\":{},\"experience_update\":{}}},\"state\":{}}}",
        request_id,
        request.action.as_str(),
        request.amount,
        option_u64_service_json(request.experience_id),
        option_u64_service_json(request.memory_id),
        source,
        evidence,
        service_u64_array(memory_ids),
        memory_update_applied_count(updates),
        memory_update_missing_count(updates),
        memory_update_removed_count(updates),
        memory_update_strength_delta(updates),
        service_memory_update_array(updates),
        experience_update,
        model_service_state_json(inspection)
    )
}
