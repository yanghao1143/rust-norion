use super::endpoint::StreamEndpoint;
use super::json::json_string;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_owned(),
            content: content.into(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_owned(),
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_owned(),
            content: content.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct StreamRequest {
    pub prompt: String,
    pub messages: Vec<ChatMessage>,
    pub profile: String,
    pub output: String,
    pub max_tokens: Option<usize>,
    pub endpoint: StreamEndpoint,
    pub feedback_amount: String,
    pub rust_check_code: Option<String>,
    pub rust_check_edition: String,
    pub rust_check_case: Option<String>,
    pub self_improve: bool,
}

impl StreamRequest {
    pub fn chat(prompt: impl Into<String>, messages: Vec<ChatMessage>) -> Self {
        let prompt = prompt.into();
        let messages = if messages.is_empty() {
            vec![ChatMessage::user(prompt.clone())]
        } else {
            messages
        };
        Self {
            prompt,
            messages,
            profile: "coding".to_owned(),
            output: "raw".to_owned(),
            max_tokens: None,
            endpoint: StreamEndpoint::Chat,
            feedback_amount: "0.5".to_owned(),
            rust_check_code: None,
            rust_check_edition: "2021".to_owned(),
            rust_check_case: None,
            self_improve: true,
        }
    }

    pub(crate) fn body_json(&self) -> String {
        match self.endpoint {
            StreamEndpoint::Chat => format!(
                "{{\"messages\":{},\"profile\":{},\"output\":{},\"case\":\"smartsteam-forge-chat\"{}}}",
                messages_json(&self.messages),
                json_string(&self.profile),
                json_string(&self.output),
                max_tokens_json(self.max_tokens)
            ),
            StreamEndpoint::Generate => format!(
                "{{\"prompt\":{},\"profile\":{},\"output\":{},\"case\":\"smartsteam-forge-generate\"{}}}",
                json_string(&self.prompt),
                json_string(&self.profile),
                json_string(&self.output),
                max_tokens_json(self.max_tokens)
            ),
            StreamEndpoint::BusinessCycle => {
                let rust_check = self
                    .rust_check_code
                    .as_ref()
                    .map(|code| {
                        let case_name = self
                            .rust_check_case
                            .as_ref()
                            .map(|case| format!(",\"rust_check_case\":{}", json_string(case)))
                            .unwrap_or_default();
                        format!(
                            ",\"rust_check_code\":{},\"rust_check_edition\":{}{}",
                            json_string(code),
                            json_string(&self.rust_check_edition),
                            case_name
                        )
                    })
                    .unwrap_or_default();
                format!(
                    "{{\"prompt\":{},\"profile\":{},\"case\":\"smartsteam-forge-business-cycle\",\"feedback_amount\":{},\"self_improve\":{},\"self_improve_limit\":1,\"gate\":\"business_cycle\",\"trace_gate\":true{}{}}}",
                    json_string(&self.prompt),
                    json_string(&self.profile),
                    self.feedback_amount,
                    self.self_improve,
                    max_tokens_json(self.max_tokens),
                    rust_check
                )
            }
        }
    }
}

fn max_tokens_json(max_tokens: Option<usize>) -> String {
    max_tokens
        .map(|max_tokens| format!(",\"max_tokens\":{max_tokens}"))
        .unwrap_or_default()
}

fn messages_json(messages: &[ChatMessage]) -> String {
    let items = messages
        .iter()
        .map(|message| {
            format!(
                "{{\"role\":{},\"content\":{}}}",
                json_string(&message.role),
                json_string(&message.content)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{items}]")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_request_body_preserves_context() {
        let request = StreamRequest::chat(
            "第二问",
            vec![
                ChatMessage::system("会话摘要"),
                ChatMessage::user("第一问"),
                ChatMessage::assistant("第一答"),
                ChatMessage::user("第二问"),
            ],
        );

        let body = request.body_json();

        assert!(body.contains("\"role\":\"system\",\"content\":\"会话摘要\""));
        assert!(body.contains("\"role\":\"assistant\",\"content\":\"第一答\""));
        assert!(body.contains("\"case\":\"smartsteam-forge-chat\""));
        assert!(!body.contains("\"max_tokens\""));
    }

    #[test]
    fn business_cycle_request_body_includes_gate_options() {
        let mut request = StreamRequest::chat("检查代码", vec![ChatMessage::user("检查代码")]);
        request.endpoint = StreamEndpoint::BusinessCycle;
        request.feedback_amount = "0.250".to_owned();
        request.rust_check_code = Some("pub fn ok() {}".to_owned());
        request.rust_check_edition = "2024".to_owned();
        request.rust_check_case = Some("forge-smoke".to_owned());
        request.self_improve = false;

        let body = request.body_json();

        assert!(body.contains("\"feedback_amount\":0.250"));
        assert!(body.contains("\"self_improve\":false"));
        assert!(body.contains("\"gate\":\"business_cycle\""));
        assert!(body.contains("\"rust_check_code\":\"pub fn ok() {}\""));
        assert!(body.contains("\"rust_check_edition\":\"2024\""));
        assert!(body.contains("\"rust_check_case\":\"forge-smoke\""));
    }

    #[test]
    fn request_body_includes_max_tokens_for_each_endpoint() {
        for endpoint in [
            StreamEndpoint::Chat,
            StreamEndpoint::Generate,
            StreamEndpoint::BusinessCycle,
        ] {
            let mut request = StreamRequest::chat("hello", vec![ChatMessage::user("hello")]);
            request.endpoint = endpoint;
            request.max_tokens = Some(8192);

            let body = request.body_json();

            assert!(body.contains("\"max_tokens\":8192"));
        }
    }
}
