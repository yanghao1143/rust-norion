use rust_norion::{RewardAction, TenantScope};

use super::super::json::{json_f32_field, json_string_field, json_u64_field};
use super::scope::require_tenant_scope;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ModelServiceFeedbackRequest {
    pub(crate) action: RewardAction,
    pub(crate) amount: f32,
    pub(crate) experience_id: Option<u64>,
    pub(crate) memory_id: Option<u64>,
    pub(crate) capability_token: Option<String>,
    pub(crate) source: Option<String>,
    pub(crate) evidence: Option<String>,
    pub(crate) tenant_scope: Option<TenantScope>,
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
    let capability_token = bounded_feedback_text(body, "capability_token", 160, false)?;
    let source = bounded_feedback_text(body, "source", 64, false)?;
    let evidence = bounded_feedback_text(body, "evidence", 512, true)?;
    let tenant_scope = require_tenant_scope(body)?;

    Ok(ModelServiceFeedbackRequest {
        action,
        amount,
        experience_id,
        memory_id,
        capability_token,
        source,
        evidence,
        tenant_scope: Some(tenant_scope),
    })
}

fn bounded_feedback_text(
    body: &str,
    field: &str,
    max_chars: usize,
    allow_comma: bool,
) -> Result<Option<String>, String> {
    let Some(value) = json_string_field(body, field) else {
        return Ok(None);
    };
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.chars().count() > max_chars
        || !value.chars().all(|character| {
            character.is_ascii_alphanumeric()
                || matches!(character, '_' | '-' | '.' | ':')
                || (allow_comma && character == ',')
        })
    {
        return Err(format!("feedback {field} contains unsupported characters"));
    }
    Ok(Some(value.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feedback_accepts_bounded_behavior_capability_and_evidence() {
        let request = parse_feedback_request(
            "{\"experience_id\":7,\"action\":\"penalize\",\"capability_token\":\"redaction-digest:abc123\",\"source\":\"browser_behavior_validation\",\"evidence\":\"gomoku_wrong_winner,gomoku_reset_failed\",\"tenant_id\":\"tenant\",\"workspace_id\":\"workspace\",\"session_id\":\"session\"}",
        )
        .unwrap();

        assert_eq!(
            request.capability_token.as_deref(),
            Some("redaction-digest:abc123")
        );
        assert_eq!(
            request.source.as_deref(),
            Some("browser_behavior_validation")
        );
        assert_eq!(
            request.evidence.as_deref(),
            Some("gomoku_wrong_winner,gomoku_reset_failed")
        );
    }

    #[test]
    fn feedback_rejects_free_form_evidence() {
        let error = parse_feedback_request(
            "{\"experience_id\":7,\"evidence\":\"winner was wrong\",\"tenant_id\":\"tenant\",\"workspace_id\":\"workspace\",\"session_id\":\"session\"}",
        )
        .unwrap_err();

        assert!(error.contains("unsupported characters"));
    }
}
