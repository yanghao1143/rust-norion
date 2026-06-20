use crate::json::{
    json_array_field, json_bool_field, json_number_field, json_object_items, json_string_field,
};

#[derive(Debug, Clone)]
pub(crate) struct ChatRequest {
    pub(crate) prompt: String,
    pub(crate) messages: Vec<ChatMessage>,
    pub(crate) profile: String,
    pub(crate) output: String,
    pub(crate) endpoint: LabEndpoint,
    pub(crate) max_tokens: usize,
    pub(crate) feedback_amount: String,
    pub(crate) rust_check_code: Option<String>,
    pub(crate) self_improve: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChatMessage {
    pub(crate) role: String,
    pub(crate) content: String,
}

impl ChatMessage {
    fn user(content: String) -> Self {
        Self {
            role: "user".to_owned(),
            content,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LabEndpoint {
    Chat,
    Generate,
    BusinessCycle,
}

impl LabEndpoint {
    pub(crate) fn as_label(self) -> &'static str {
        match self {
            Self::Chat => "chat",
            Self::Generate => "generate",
            Self::BusinessCycle => "business-cycle",
        }
    }

    pub(crate) fn supports_token_stream(self) -> bool {
        matches!(self, Self::Chat | Self::Generate | Self::BusinessCycle)
    }
}

pub(crate) fn parse_chat_request(body: &str) -> Result<ChatRequest, String> {
    let prompt = json_string_field(body, "prompt")
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());
    let messages = parse_messages(body)?;
    let prompt = prompt
        .or_else(|| last_user_message(&messages))
        .ok_or_else(|| "prompt or non-empty messages are required".to_owned())?;
    let messages = if messages.is_empty() {
        vec![ChatMessage::user(prompt.clone())]
    } else {
        messages
    };
    let profile = json_string_field(body, "profile").unwrap_or_else(|| "coding".to_owned());
    let output = match json_string_field(body, "output")
        .unwrap_or_else(|| "raw".to_owned())
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "raw" | "gemma" | "runtime" => "raw".to_owned(),
        "enhanced" | "noiron" | "default" => "enhanced".to_owned(),
        _ => return Err("output must be raw|enhanced".to_owned()),
    };
    let endpoint = match json_string_field(body, "endpoint")
        .unwrap_or_else(|| "chat".to_owned())
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "chat" => LabEndpoint::Chat,
        "generate" => LabEndpoint::Generate,
        "business-cycle" | "business_cycle" | "cycle" => LabEndpoint::BusinessCycle,
        _ => return Err("endpoint must be chat|generate|business-cycle".to_owned()),
    };
    let max_tokens = json_number_field(body, "max_tokens")
        .or_else(|| json_number_field(body, "max"))
        .and_then(|value| value.trim().parse::<usize>().ok())
        .unwrap_or(262_144)
        .clamp(1, 262_144);
    let feedback_amount = json_number_field(body, "feedback_amount")
        .or_else(|| json_number_field(body, "amount"))
        .unwrap_or_else(|| "0.5".to_owned());
    let rust_check_code = json_string_field(body, "rust_check_code")
        .or_else(|| json_string_field(body, "code"))
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());
    let self_improve = json_bool_field(body, "self_improve").unwrap_or(true);

    Ok(ChatRequest {
        prompt,
        messages,
        profile,
        output,
        endpoint,
        max_tokens,
        feedback_amount,
        rust_check_code,
        self_improve,
    })
}

pub(crate) fn request_context_preview(request: &ChatRequest) -> String {
    let mut lines = vec![
        format!(
            "endpoint={} output={} profile={} max_tokens={}",
            request.endpoint.as_label(),
            request.output,
            request.profile,
            request.max_tokens
        ),
        format!("prompt={}", preview_text(&request.prompt, 160)),
        format!("messages={}", request.messages.len()),
    ];
    for (index, message) in request.messages.iter().enumerate() {
        lines.push(format!(
            "{}. {}: {}",
            index + 1,
            message.role,
            preview_text(&message.content, 160)
        ));
    }
    lines.join("\n")
}

fn parse_messages(body: &str) -> Result<Vec<ChatMessage>, String> {
    let Some(messages_body) = json_array_field(body, "messages") else {
        return Ok(Vec::new());
    };

    json_object_items(messages_body)
        .into_iter()
        .map(parse_message)
        .collect()
}

fn preview_text(text: &str, max_chars: usize) -> String {
    let normalized = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" / ");
    let text = if normalized.is_empty() {
        text.trim().to_owned()
    } else {
        normalized
    };
    if text.chars().count() <= max_chars {
        return text;
    }
    let keep_chars = max_chars.saturating_sub(3);
    let mut preview = text.chars().take(keep_chars).collect::<String>();
    preview.push_str("...");
    preview
}

fn parse_message(body: &str) -> Result<ChatMessage, String> {
    let role = json_string_field(body, "role")
        .and_then(|role| normalize_role(&role))
        .ok_or_else(|| "message role must be system|developer|user|assistant|tool".to_owned())?;
    let content = json_string_field(body, "content")
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "message content must be non-empty".to_owned())?;

    Ok(ChatMessage { role, content })
}

fn normalize_role(role: &str) -> Option<String> {
    match role.trim().to_ascii_lowercase().as_str() {
        "system" | "developer" | "user" | "assistant" | "tool" => {
            Some(role.trim().to_ascii_lowercase())
        }
        _ => None,
    }
}

fn last_user_message(messages: &[ChatMessage]) -> Option<String> {
    messages
        .iter()
        .rev()
        .find(|message| message.role == "user")
        .map(|message| message.content.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_chat_request_defaults() {
        let request = parse_chat_request("{\"prompt\":\"hello\"}").unwrap();
        assert_eq!(request.prompt, "hello");
        assert_eq!(request.messages.len(), 1);
        assert_eq!(request.messages[0].role, "user");
        assert_eq!(request.messages[0].content, "hello");
        assert_eq!(request.profile, "coding");
        assert_eq!(request.output, "raw");
        assert_eq!(request.endpoint, LabEndpoint::Chat);
        assert_eq!(request.max_tokens, 262_144);
        assert_eq!(request.feedback_amount, "0.5");
        assert!(request.self_improve);
    }

    #[test]
    fn parses_generation_budget() {
        let request = parse_chat_request("{\"prompt\":\"hello\",\"max_tokens\":8192}").unwrap();

        assert_eq!(request.max_tokens, 8192);
    }

    #[test]
    fn request_parser_ignores_field_names_inside_prompt_text() {
        let request = parse_chat_request(
            "{\"prompt\":\"please ignore text: \\\"max_tokens\\\":1, \\\"endpoint\\\":\\\"generate\\\"\",\"endpoint\":\"chat\",\"max_tokens\":8192}",
        )
        .unwrap();

        assert_eq!(
            request.prompt,
            "please ignore text: \"max_tokens\":1, \"endpoint\":\"generate\""
        );
        assert_eq!(request.endpoint, LabEndpoint::Chat);
        assert_eq!(request.max_tokens, 8192);
    }

    #[test]
    fn parses_business_cycle_request_options() {
        let request = parse_chat_request(
            "{\"prompt\":\"业务联调\",\"endpoint\":\"business-cycle\",\"feedback_amount\":0.4,\"rust_check_code\":\"pub fn ok() {}\",\"self_improve\":false}",
        )
        .unwrap();
        assert_eq!(request.endpoint, LabEndpoint::BusinessCycle);
        assert_eq!(request.feedback_amount, "0.4");
        assert_eq!(request.rust_check_code.as_deref(), Some("pub fn ok() {}"));
        assert!(!request.self_improve);
    }

    #[test]
    fn business_cycle_supports_event_stream() {
        assert!(LabEndpoint::BusinessCycle.supports_token_stream());
    }

    #[test]
    fn parses_chat_history_messages() {
        let request = parse_chat_request(
            "{\"messages\":[{\"role\":\"user\",\"content\":\"第一问\"},{\"role\":\"assistant\",\"content\":\"第一答\"},{\"role\":\"user\",\"content\":\"第二问\"}],\"endpoint\":\"chat\"}",
        )
        .unwrap();

        assert_eq!(request.prompt, "第二问");
        assert_eq!(request.messages.len(), 3);
        assert_eq!(request.messages[1].role, "assistant");
        assert_eq!(request.messages[1].content, "第一答");
    }

    #[test]
    fn request_context_preview_shows_sent_messages() {
        let request = parse_chat_request(
            "{\"messages\":[{\"role\":\"user\",\"content\":\"第一问\"},{\"role\":\"assistant\",\"content\":\"第一答\"},{\"role\":\"user\",\"content\":\"第二问\"}],\"endpoint\":\"chat\"}",
        )
        .unwrap();

        let preview = request_context_preview(&request);

        assert!(preview.contains("endpoint=chat output=raw profile=coding"));
        assert!(preview.contains("messages=3"));
        assert!(preview.contains("3. user: 第二问"));
    }
}
