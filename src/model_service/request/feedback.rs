use rust_norion::RewardAction;

use super::super::json::{json_f32_field, json_string_field, json_u64_field};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ModelServiceFeedbackRequest {
    pub(crate) action: RewardAction,
    pub(crate) amount: f32,
    pub(crate) experience_id: Option<u64>,
    pub(crate) memory_id: Option<u64>,
}

pub(super) fn parse_feedback_request(body: &str) -> Result<ModelServiceFeedbackRequest, String> {
    let action_text = json_string_field(body, "action")
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "reinforce".to_owned());
    let action = action_text
        .trim()
        .parse::<RewardAction>()
        .map_err(|_| format!("unsupported feedback action: {action_text}"))?;
    if action == RewardAction::Hold {
        return Err("feedback action must be reinforce or penalize".to_owned());
    }
    let amount = json_f32_field(body, "amount")
        .unwrap_or(0.5)
        .clamp(0.0, 1.0);
    let experience_id = json_u64_field(body, "experience_id");
    let memory_id = json_u64_field(body, "memory_id");
    if experience_id.is_none() && memory_id.is_none() {
        return Err("feedback requires experience_id or memory_id".to_owned());
    }

    Ok(ModelServiceFeedbackRequest {
        action,
        amount,
        experience_id,
        memory_id,
    })
}
