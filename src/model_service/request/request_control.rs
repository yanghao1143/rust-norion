use super::super::json::{json_string_field, json_usize_field};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelServiceRequestCancelRequest {
    pub(crate) request_id: usize,
    pub(crate) reason: String,
    pub(crate) retag_label: String,
}

pub(super) fn parse_request_cancel_request(
    body: &str,
) -> Result<ModelServiceRequestCancelRequest, String> {
    let request_id = json_usize_field(body, "request_id")
        .or_else(|| json_usize_field(body, "id"))
        .filter(|request_id| *request_id > 0)
        .ok_or_else(|| "JSON body must include a positive request_id".to_owned())?;
    let reason = json_string_field(body, "reason")
        .filter(|reason| !reason.trim().is_empty())
        .unwrap_or_else(|| "operator_runtime_splice".to_owned());
    let retag_label = json_string_field(body, "retag_label")
        .or_else(|| json_string_field(body, "label"))
        .filter(|label| !label.trim().is_empty())
        .unwrap_or_else(|| "repair_requested".to_owned());

    Ok(ModelServiceRequestCancelRequest {
        request_id,
        reason,
        retag_label,
    })
}
