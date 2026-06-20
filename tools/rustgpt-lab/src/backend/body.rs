use crate::json::json_string;
use crate::request::{ChatMessage, ChatRequest, LabEndpoint};

pub(super) fn backend_request_body(request: &ChatRequest) -> String {
    match request.endpoint {
        LabEndpoint::Chat => format!(
            "{{\"messages\":{},\"profile\":{},\"output\":{},\"max_tokens\":{},\"case\":\"rustgpt-lab-chat\"}}",
            chat_messages_json(&request.messages),
            json_string(&request.profile),
            json_string(&request.output),
            request.max_tokens
        ),
        LabEndpoint::Generate => format!(
            "{{\"prompt\":{},\"profile\":{},\"output\":{},\"max_tokens\":{},\"case\":\"rustgpt-lab-generate\"}}",
            json_string(&request.prompt),
            json_string(&request.profile),
            json_string(&request.output),
            request.max_tokens
        ),
        LabEndpoint::BusinessCycle => {
            let rust_check = request
                .rust_check_code
                .as_ref()
                .map(|code| format!(",\"rust_check_code\":{}", json_string(code)))
                .unwrap_or_default();
            format!(
                "{{\"prompt\":{},\"profile\":{},\"max_tokens\":{},\"case\":\"rustgpt-lab-business-cycle\",\"feedback_amount\":{},\"self_improve\":{},\"self_improve_limit\":1,\"gate\":\"business_cycle\",\"trace_gate\":true{}}}",
                json_string(&request.prompt),
                json_string(&request.profile),
                request.max_tokens,
                request.feedback_amount,
                request.self_improve,
                rust_check
            )
        }
    }
}

fn chat_messages_json(messages: &[ChatMessage]) -> String {
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
