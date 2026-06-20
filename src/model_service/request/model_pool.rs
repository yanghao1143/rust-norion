use super::super::json::{json_string_array_field, json_string_field, json_usize_field};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelServiceModelPoolRouteRequest {
    pub(crate) task_kind: String,
    pub(crate) max_tokens: Option<usize>,
    pub(crate) prompt: Option<String>,
    pub(crate) completed_roles: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelServiceModelPoolCallRequest {
    pub(crate) task_kind: String,
    pub(crate) prompt: String,
    pub(crate) max_tokens: Option<usize>,
    pub(crate) completed_roles: Option<Vec<String>>,
}

pub(super) fn parse_model_pool_route_request(
    body: &str,
) -> Result<ModelServiceModelPoolRouteRequest, String> {
    let task_kind = json_string_field(body, "task_kind")
        .or_else(|| json_string_field(body, "task"))
        .unwrap_or_else(|| "auto".to_owned());
    Ok(ModelServiceModelPoolRouteRequest {
        task_kind: normalize_task_kind(&task_kind)?.to_owned(),
        max_tokens: json_usize_field(body, "max_tokens")
            .or_else(|| json_usize_field(body, "max"))
            .map(|value| value.max(1)),
        prompt: json_string_field(body, "prompt")
            .or_else(|| json_string_field(body, "content"))
            .filter(|value| !value.trim().is_empty()),
        completed_roles: parse_completed_roles(body)?,
    })
}

pub(super) fn parse_model_pool_call_request(
    body: &str,
) -> Result<ModelServiceModelPoolCallRequest, String> {
    let route = parse_model_pool_route_request(body)?;
    let prompt = json_string_field(body, "prompt")
        .or_else(|| json_string_field(body, "content"))
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "model pool call requires prompt".to_owned())?;
    Ok(ModelServiceModelPoolCallRequest {
        task_kind: route.task_kind,
        prompt,
        max_tokens: route.max_tokens,
        completed_roles: route.completed_roles,
    })
}

pub(crate) fn normalize_task_kind(task_kind: &str) -> Result<&'static str, String> {
    match task_kind.trim() {
        "" | "auto" => Ok("auto"),
        "summary" => Ok("summary"),
        "router" | "route" | "intent" | "intent-classify" | "preflight" | "tool-call"
        | "tool_calls" | "function" | "function-call" | "function_call" => Ok("router"),
        "review" => Ok("review"),
        "test-gate" | "test" | "gate" => Ok("test-gate"),
        "index" | "repo-index" | "repository-index" => Ok("index"),
        "quality" | "primary" => Ok("quality"),
        "chat" | "generate" | "generation" => Ok("chat"),
        "business-cycle" | "business_cycle" | "business" => Ok("business-cycle"),
        "spare" => Ok("index"),
        other => Err(format!(
            "unsupported model pool task_kind: {other}. Use auto, summary, router, review, test-gate, index, quality, primary, chat, or business-cycle"
        )),
    }
}

fn parse_completed_roles(body: &str) -> Result<Option<Vec<String>>, String> {
    let Some(roles) = json_string_array_field(body, "completed_roles")
        .or_else(|| json_string_array_field(body, "completed_stage_roles"))
    else {
        return Ok(None);
    };
    let mut normalized = Vec::new();
    for role in roles {
        let role = role.trim();
        if role.is_empty() {
            continue;
        }
        let normalized_role = normalize_task_kind(role)?;
        if matches!(normalized_role, "auto" | "chat" | "business-cycle") {
            continue;
        }
        if !normalized
            .iter()
            .any(|existing| existing == normalized_role)
        {
            normalized.push(normalized_role.to_owned());
        }
    }
    Ok(Some(normalized))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_model_pool_route_request() {
        let request = parse_model_pool_route_request("{\"task_kind\":\"review\"}").unwrap();

        assert_eq!(request.task_kind, "review");
        assert_eq!(request.max_tokens, None);
        assert_eq!(request.prompt, None);
        assert_eq!(request.completed_roles, None);
    }

    #[test]
    fn route_request_defaults_to_auto() {
        let request = parse_model_pool_route_request("{}").unwrap();

        assert_eq!(request.task_kind, "auto");
        assert_eq!(request.max_tokens, None);
        assert_eq!(request.prompt, None);
    }

    #[test]
    fn route_request_normalizes_test_alias() {
        let request = parse_model_pool_route_request("{\"task\":\"test\"}").unwrap();

        assert_eq!(request.task_kind, "test-gate");
        assert_eq!(request.max_tokens, None);
    }

    #[test]
    fn route_request_normalizes_router_aliases() {
        let router = parse_model_pool_route_request("{\"task\":\"router\"}").unwrap();
        let preflight = parse_model_pool_route_request("{\"task\":\"preflight\"}").unwrap();
        let tool_call = parse_model_pool_route_request("{\"task\":\"tool-call\"}").unwrap();

        assert_eq!(router.task_kind, "router");
        assert_eq!(preflight.task_kind, "router");
        assert_eq!(tool_call.task_kind, "router");
    }

    #[test]
    fn route_request_accepts_index_aliases() {
        let index = parse_model_pool_route_request("{\"task\":\"index\"}").unwrap();
        let repo_index = parse_model_pool_route_request("{\"task\":\"repo-index\"}").unwrap();
        let spare = parse_model_pool_route_request("{\"task\":\"spare\"}").unwrap();

        assert_eq!(index.task_kind, "index");
        assert_eq!(repo_index.task_kind, "index");
        assert_eq!(spare.task_kind, "index");
    }

    #[test]
    fn route_request_accepts_primary_chat_and_business_cycle_tasks() {
        let primary = parse_model_pool_route_request("{\"task_kind\":\"primary\"}").unwrap();
        let chat = parse_model_pool_route_request("{\"task_kind\":\"chat\",\"max_tokens\":262144}")
            .unwrap();
        let business =
            parse_model_pool_route_request("{\"task_kind\":\"business_cycle\",\"max\":4096}")
                .unwrap();

        assert_eq!(primary.task_kind, "quality");
        assert_eq!(chat.task_kind, "chat");
        assert_eq!(chat.max_tokens, Some(262_144));
        assert_eq!(business.task_kind, "business-cycle");
        assert_eq!(business.max_tokens, Some(4096));
        assert_eq!(business.prompt, None);
    }

    #[test]
    fn route_request_accepts_optional_prompt_for_weighted_routing() {
        let request = parse_model_pool_route_request(
            "{\"task_kind\":\"auto\",\"prompt\":\"review this Rust routing trace\"}",
        )
        .unwrap();

        assert_eq!(request.task_kind, "auto");
        assert_eq!(
            request.prompt.as_deref(),
            Some("review this Rust routing trace")
        );
    }

    #[test]
    fn route_request_accepts_completed_roles_for_dependency_precheck() {
        let request = parse_model_pool_route_request(
            "{\"task_kind\":\"test\",\"completed_roles\":[\"primary\",\"summary\",\"summary\",\"route\"]}",
        )
        .unwrap();

        assert_eq!(request.task_kind, "test-gate");
        assert_eq!(
            request.completed_roles,
            Some(vec![
                "quality".to_owned(),
                "summary".to_owned(),
                "router".to_owned()
            ])
        );
    }

    #[test]
    fn route_request_rejects_unknown_task() {
        let error = parse_model_pool_route_request("{\"task_kind\":\"launch\"}").unwrap_err();

        assert!(error.contains("unsupported model pool task_kind"));
    }

    #[test]
    fn parses_model_pool_call_request() {
        let request = parse_model_pool_call_request(
            "{\"task\":\"summary\",\"prompt\":\"summarize logs\",\"max_tokens\":128}",
        )
        .unwrap();

        assert_eq!(
            request,
            ModelServiceModelPoolCallRequest {
                task_kind: "summary".to_owned(),
                prompt: "summarize logs".to_owned(),
                max_tokens: Some(128),
                completed_roles: None,
            }
        );
    }

    #[test]
    fn call_request_accepts_max_alias() {
        let request = parse_model_pool_call_request(
            "{\"task\":\"quality\",\"prompt\":\"long\",\"max\":262144}",
        )
        .unwrap();

        assert_eq!(request.task_kind, "quality");
        assert_eq!(request.max_tokens, Some(262_144));
    }

    #[test]
    fn call_request_requires_prompt() {
        let error = parse_model_pool_call_request("{\"task_kind\":\"review\"}").unwrap_err();

        assert!(error.contains("requires prompt"));
    }
}
