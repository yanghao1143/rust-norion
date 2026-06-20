use rust_norion::{MemoryUpdateReport, RustSnippetCheckReport, StateInspectionReport};

use super::super::json::{
    option_i32_service_json, option_u64_service_json, service_json_string,
    service_memory_update_array, service_u64_array,
};
use super::super::request::{ModelServiceFeedbackRequest, ModelServiceRustCheckRequest};
use super::state::model_service_state_json;
use super::update_stats::{
    memory_update_applied_count, memory_update_missing_count, memory_update_removed_count,
    memory_update_strength_delta,
};

pub(crate) fn model_service_rust_check_response_json(
    request_id: usize,
    request: &ModelServiceRustCheckRequest,
    report: &RustSnippetCheckReport,
    feedback_request: &ModelServiceFeedbackRequest,
    memory_ids: &[u64],
    updates: &[MemoryUpdateReport],
    inspection: &StateInspectionReport,
) -> String {
    format!(
        "{{\"ok\":true,\"request_id\":{},\"rust_check\":{{\"passed\":{},\"label\":\"{}\",\"edition\":\"{}\",\"status_code\":{},\"diagnostic_chars\":{},\"stdout\":{},\"stderr\":{},\"source_path\":{},\"metadata_path\":{}}},\"feedback\":{{\"action\":\"{}\",\"amount\":{:.6},\"experience_id\":{},\"memory_id\":{},\"memory_ids\":{},\"applied\":{},\"missing\":{},\"removed\":{},\"strength_delta\":{:.6},\"updates\":{}}},\"state\":{}}}",
        request_id,
        report.passed,
        report.feedback_label(),
        report.edition,
        option_i32_service_json(report.status_code),
        report.diagnostic_chars(),
        service_json_string(&report.stdout),
        service_json_string(&report.stderr),
        service_json_string(&report.source_path.display().to_string()),
        service_json_string(&report.metadata_path.display().to_string()),
        feedback_request.action.as_str(),
        feedback_request.amount,
        option_u64_service_json(request.experience_id),
        option_u64_service_json(request.memory_id),
        service_u64_array(memory_ids),
        memory_update_applied_count(updates),
        memory_update_missing_count(updates),
        memory_update_removed_count(updates),
        memory_update_strength_delta(updates),
        service_memory_update_array(updates),
        model_service_state_json(inspection)
    )
}
