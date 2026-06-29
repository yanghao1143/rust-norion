use rust_norion::{TaskProfile, TenantScope};

use super::super::json::{json_string_field, json_usize_field};
use super::generate::ModelServiceRequest;
use super::output::ModelServiceOutputMode;
use super::scope::parse_tenant_scope;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelServiceChatMessage {
    pub(crate) role: String,
    pub(crate) content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelServiceChatRequest {
    pub(crate) messages: Vec<ModelServiceChatMessage>,
    pub(crate) model: Option<String>,
    pub(crate) profile: Option<TaskProfile>,
    pub(crate) case_name: Option<String>,
    pub(crate) output_mode: ModelServiceOutputMode,
    pub(crate) max_tokens: Option<usize>,
    pub(crate) tenant_scope: Option<TenantScope>,
}

impl ModelServiceChatRequest {
    pub(crate) fn into_generate_request(self) -> ModelServiceRequest {
        ModelServiceRequest {
            prompt: render_chat_prompt(&self.messages),
            profile: self.profile,
            case_name: self.case_name,
            output_mode: self.output_mode,
            max_tokens: self.max_tokens,
            tenant_scope: self.tenant_scope,
        }
    }
}

pub(super) fn parse_chat_request(body: &str) -> Result<ModelServiceChatRequest, String> {
    let messages_body = json_array_field(body, "messages")
        .ok_or_else(|| "chat request requires a non-empty messages array".to_owned())?;
    let message_objects = json_object_items(messages_body);
    if message_objects.is_empty() {
        return Err("chat request requires at least one message".to_owned());
    }

    let messages = message_objects
        .iter()
        .map(|message| parse_chat_message(message))
        .collect::<Result<Vec<_>, _>>()?;
    let model = json_string_field(body, "model").filter(|model| !model.trim().is_empty());
    let profile = json_string_field(body, "profile")
        .map(|value| value.parse::<TaskProfile>())
        .transpose()
        .map_err(|error| error.to_string())?;
    let case_name = json_string_field(body, "case").filter(|case| !case.trim().is_empty());
    let output_mode = ModelServiceOutputMode::parse_from_body(body)?;
    let max_tokens = json_usize_field(body, "max_tokens")
        .or_else(|| json_usize_field(body, "max"))
        .map(|value| value.max(1));
    let tenant_scope = parse_tenant_scope(body);

    Ok(ModelServiceChatRequest {
        messages,
        model,
        profile,
        case_name,
        output_mode,
        max_tokens,
        tenant_scope,
    })
}

fn parse_chat_message(body: &str) -> Result<ModelServiceChatMessage, String> {
    let role = json_string_field(body, "role")
        .and_then(|role| normalize_chat_role(&role))
        .ok_or_else(|| {
            "chat message requires role system|developer|user|assistant|tool".to_owned()
        })?;
    let content = json_string_field(body, "content")
        .map(|content| content.trim().to_owned())
        .filter(|content| !content.is_empty())
        .ok_or_else(|| "chat message requires non-empty content".to_owned())?;

    Ok(ModelServiceChatMessage { role, content })
}

fn normalize_chat_role(role: &str) -> Option<String> {
    match role.trim().to_ascii_lowercase().as_str() {
        "system" | "developer" | "user" | "assistant" | "tool" => {
            Some(role.trim().to_ascii_lowercase())
        }
        _ => None,
    }
}

fn render_chat_prompt(messages: &[ModelServiceChatMessage]) -> String {
    let mut prompt = String::from("Conversation transcript:\n");
    for message in messages {
        prompt.push_str(&message.role);
        prompt.push_str(": ");
        prompt.push_str(&message.content);
        prompt.push('\n');
    }
    prompt.push_str("assistant:");
    prompt
}

fn json_array_field<'a>(body: &'a str, field: &str) -> Option<&'a str> {
    let needle = format!("\"{field}\"");
    let after_field = body.get(body.find(&needle)? + needle.len()..)?;
    let after_colon = after_field.get(after_field.find(':')? + 1..)?;
    let trimmed = after_colon.trim_start();
    let close = find_matching_json_close(trimmed, '[', ']')?;
    trimmed.get(1..close)
}

fn json_object_items(input: &str) -> Vec<&str> {
    let mut items = Vec::new();
    let mut start = None;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (index, character) in input.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }

        match character {
            '"' => in_string = true,
            '{' => {
                if depth == 0 {
                    start = Some(index);
                }
                depth = depth.saturating_add(1);
            }
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0
                    && let Some(start_index) = start.take()
                    && let Some(item) = input.get(start_index..=index)
                {
                    items.push(item);
                }
            }
            _ => {}
        }
    }

    items
}

fn find_matching_json_close(input: &str, open: char, close: char) -> Option<usize> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (index, character) in input.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }

        match character {
            '"' => in_string = true,
            value if value == open => depth = depth.saturating_add(1),
            value if value == close => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_request_preserves_tenant_scope_when_rendered_for_generation() {
        let request = parse_chat_request(
            "{\"messages\":[{\"role\":\"user\",\"content\":\"hi\"}],\"tenant_id\":\"tenant-a\",\"workspace_id\":\"workspace\",\"session_id\":\"chat-1\"}",
        )
        .unwrap()
        .into_generate_request();

        assert_eq!(
            request.tenant_scope,
            Some(TenantScope::new("tenant-a", "workspace", "chat-1"))
        );
    }
}
