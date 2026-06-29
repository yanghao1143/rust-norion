use std::net::TcpStream;

use super::super::super::json::{service_json_string, write_http_json};

pub(super) fn handle_endpoint_info(
    stream: &mut TcpStream,
    request_id: usize,
    endpoint: &str,
) -> std::io::Result<()> {
    let body = model_service_endpoint_info_json(request_id, endpoint);
    write_http_json(stream, 200, "OK", &body)
}

fn model_service_endpoint_info_json(request_id: usize, endpoint: &str) -> String {
    let spec = EndpointInfoSpec::for_endpoint(endpoint);
    let response_fields = endpoint_response_fields(endpoint);
    format!(
        "{{\"ok\":true,\"request_id\":{},\"endpoint\":\"{}\",\"method\":\"POST\",\"content_type\":\"application/json\",\"example\":{},\"supported_fields\":{},\"response_fields\":{},\"unsupported_fields\":{}}}",
        request_id,
        spec.path,
        spec.example,
        str_array_json(spec.supported_fields),
        str_array_json(response_fields),
        str_array_json(spec.unsupported_fields)
    )
}

fn str_array_json(values: &[&str]) -> String {
    let items = values
        .iter()
        .map(|value| service_json_string(value))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{items}]")
}

struct EndpointInfoSpec {
    path: &'static str,
    example: &'static str,
    supported_fields: &'static [&'static str],
    unsupported_fields: &'static [&'static str],
}

impl EndpointInfoSpec {
    fn for_endpoint(endpoint: &str) -> Self {
        match endpoint {
            "generate" => Self {
                path: "/v1/generate",
                example: "{\"prompt\":\"用中文给一个 rust-norion 业务联调建议。\",\"profile\":\"coding\",\"case\":\"manual-generate\",\"output\":\"raw\"}",
                supported_fields: &[
                    "prompt",
                    "profile",
                    "case",
                    "output",
                    "max_tokens",
                    "tenant_id",
                    "workspace_id",
                    "session_id",
                ],
                unsupported_fields: &[
                    "messages",
                    "stream",
                    "tools",
                    "tool_choice",
                    "response_format",
                ],
            },
            "chat" => Self {
                path: "/v1/chat",
                example: "{\"messages\":[{\"role\":\"user\",\"content\":\"用中文给一个 rust-norion 业务联调建议。\"}],\"profile\":\"coding\",\"case\":\"manual-chat\",\"output\":\"raw\"}",
                supported_fields: &[
                    "messages",
                    "profile",
                    "case",
                    "output",
                    "max_tokens",
                    "tenant_id",
                    "workspace_id",
                    "session_id",
                ],
                unsupported_fields: &["stream", "tools", "tool_choice", "response_format"],
            },
            "chat-completions" => Self {
                path: "/v1/chat/completions",
                example: "{\"model\":\"rust-norion-local\",\"messages\":[{\"role\":\"user\",\"content\":\"用中文给一个 rust-norion 业务联调建议。\"}],\"max_tokens\":256,\"stream\":true}",
                supported_fields: &[
                    "model",
                    "messages",
                    "max_tokens",
                    "stream",
                    "tenant_id",
                    "workspace_id",
                    "session_id",
                ],
                unsupported_fields: &["tools", "tool_choice", "response_format"],
            },
            "completions" => Self {
                path: "/v1/completions",
                example: "{\"model\":\"rust-norion-local\",\"prompt\":\"用中文给一个 rust-norion 业务联调建议。\",\"max_tokens\":256}",
                supported_fields: &[
                    "model",
                    "prompt",
                    "max_tokens",
                    "tenant_id",
                    "workspace_id",
                    "session_id",
                ],
                unsupported_fields: &["stream", "logprobs", "suffix"],
            },
            "chat-stream" => Self {
                path: "/v1/chat-stream",
                example: "{\"messages\":[{\"role\":\"user\",\"content\":\"用中文流式测试 SmartSteam Forge。\"}],\"profile\":\"coding\",\"case\":\"manual-chat-stream\",\"output\":\"raw\"}",
                supported_fields: &[
                    "messages",
                    "profile",
                    "case",
                    "output",
                    "max_tokens",
                    "tenant_id",
                    "workspace_id",
                    "session_id",
                ],
                unsupported_fields: &["tools", "tool_choice", "response_format"],
            },
            "generate-stream" => Self {
                path: "/v1/generate-stream",
                example: "{\"prompt\":\"用中文流式测试 rust-norion 本地模型服务。\",\"profile\":\"coding\",\"case\":\"manual-generate-stream\",\"output\":\"raw\"}",
                supported_fields: &[
                    "prompt",
                    "profile",
                    "case",
                    "output",
                    "max_tokens",
                    "tenant_id",
                    "workspace_id",
                    "session_id",
                ],
                unsupported_fields: &["messages", "tools", "tool_choice", "response_format"],
            },
            "business-cycle" => Self {
                path: "/v1/business-cycle",
                example: "{\"prompt\":\"用中文完成一次 SmartSteam 业务联调自检。\",\"feedback_amount\":0.4,\"self_improve\":true,\"rust_check_code\":\"fn main() {}\"}",
                supported_fields: &[],
                unsupported_fields: &[],
            },
            "business-cycle-stream" => Self {
                path: "/v1/business-cycle-stream",
                example: "{\"prompt\":\"用中文流式完成一次 SmartSteam 业务联调自检。\",\"feedback_amount\":0.4,\"self_improve\":true,\"rust_check_code\":\"fn main() {}\"}",
                supported_fields: &[],
                unsupported_fields: &[],
            },
            "experience-hygiene-quarantine" => Self {
                path: "/v1/experience-hygiene/quarantine",
                example: "{\"apply\":false,\"limit\":20}",
                supported_fields: &[],
                unsupported_fields: &[],
            },
            "experience-cleanup-audit" => Self {
                path: "/v1/experience-cleanup-audit",
                example: "{\"limit\":20}",
                supported_fields: &[],
                unsupported_fields: &[],
            },
            "experience-repair" => Self {
                path: "/v1/experience-repair",
                example: "{\"apply\":false,\"limit\":20}",
                supported_fields: &[],
                unsupported_fields: &[],
            },
            "experience-retrieval" => Self {
                path: "/v1/experience-retrieval",
                example: "{\"prompt\":\"帮我用rust输出一段for循环代码\",\"profile\":\"coding\",\"limit\":5}",
                supported_fields: &[],
                unsupported_fields: &[],
            },
            "model-pool-route-plan" => Self {
                path: "/v1/model-pool/route-plan",
                example: "{\"task_kind\":\"review\"}",
                supported_fields: &[],
                unsupported_fields: &[],
            },
            "requests-cancel" => Self {
                path: "/v1/requests/cancel",
                example: "{\"request_id\":42,\"reason\":\"operator_runtime_splice\",\"retag_label\":\"repair_factor:runtime_splice\"}",
                supported_fields: &[],
                unsupported_fields: &[],
            },
            _ => Self {
                path: "/v1/generate",
                example: "{\"prompt\":\"用中文给一个 rust-norion 业务联调建议。\",\"profile\":\"coding\",\"case\":\"manual-generate\",\"output\":\"raw\"}",
                supported_fields: &[
                    "prompt",
                    "profile",
                    "case",
                    "output",
                    "max_tokens",
                    "tenant_id",
                    "workspace_id",
                    "session_id",
                ],
                unsupported_fields: &[
                    "messages",
                    "stream",
                    "tools",
                    "tool_choice",
                    "response_format",
                ],
            },
        }
    }
}

fn endpoint_response_fields(endpoint: &str) -> &'static [&'static str] {
    match endpoint {
        "chat-completions" => &[
            "id", "object", "created", "model", "choices", "usage", "norion", "error",
        ],
        "completions" => &[
            "id", "object", "created", "model", "choices", "usage", "norion", "error",
        ],
        "chat-stream" | "generate-stream" => &[
            "event:status",
            "event:delta",
            "event:final",
            "event:done",
            "event:error",
        ],
        "generate" | "chat" => &[
            "ok",
            "request_id",
            "profile",
            "answer",
            "raw_answer",
            "enhanced_answer",
            "runtime_token_count",
            "runtime_uncertainty_signal",
            "traceable",
            "error",
        ],
        _ => &["ok", "request_id", "error"],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_info_json_reports_business_cycle_stream_route() {
        let json = model_service_endpoint_info_json(7, "business-cycle-stream");

        assert!(json.contains("\"request_id\":7"));
        assert!(json.contains("\"endpoint\":\"/v1/business-cycle-stream\""));
        assert!(json.contains("\"self_improve\":true"));
        assert!(!json.contains("\"endpoint\":\"/v1/generate\""));
    }

    #[test]
    fn endpoint_info_json_reports_chat_stream_route() {
        let json = model_service_endpoint_info_json(3, "chat-stream");

        assert!(json.contains("\"endpoint\":\"/v1/chat-stream\""));
        assert!(json.contains("\"messages\""));
        assert!(json.contains("\"manual-chat-stream\""));
    }

    #[test]
    fn endpoint_info_json_reports_openai_chat_completions_contract() {
        let json = model_service_endpoint_info_json(11, "chat-completions");

        assert!(json.contains("\"endpoint\":\"/v1/chat/completions\""));
        assert!(json.contains("\"model\":\"rust-norion-local\""));
        assert!(json.contains("\"stream\":true"));
        assert!(json.contains("\"supported_fields\":[\"model\",\"messages\",\"max_tokens\",\"stream\",\"tenant_id\",\"workspace_id\",\"session_id\"]"));
        assert!(json.contains("\"response_fields\":[\"id\",\"object\",\"created\",\"model\",\"choices\",\"usage\",\"norion\",\"error\"]"));
        assert!(
            json.contains("\"unsupported_fields\":[\"tools\",\"tool_choice\",\"response_format\"]")
        );
    }

    #[test]
    fn endpoint_info_json_reports_openai_completions_contract() {
        let json = model_service_endpoint_info_json(12, "completions");

        assert!(json.contains("\"endpoint\":\"/v1/completions\""));
        assert!(json.contains("\"model\":\"rust-norion-local\""));
        assert!(json.contains("\"prompt\":\"用中文"));
        assert!(json.contains("\"supported_fields\":[\"model\",\"prompt\",\"max_tokens\",\"tenant_id\",\"workspace_id\",\"session_id\"]"));
        assert!(json.contains("\"response_fields\":[\"id\",\"object\",\"created\",\"model\",\"choices\",\"usage\",\"norion\",\"error\"]"));
        assert!(json.contains("\"unsupported_fields\":[\"stream\",\"logprobs\",\"suffix\"]"));
    }

    #[test]
    fn endpoint_info_json_reports_generate_contract_fields() {
        let json = model_service_endpoint_info_json(13, "generate");

        assert!(json.contains("\"endpoint\":\"/v1/generate\""));
        assert!(json.contains("\"supported_fields\":[\"prompt\",\"profile\",\"case\",\"output\",\"max_tokens\",\"tenant_id\",\"workspace_id\",\"session_id\"]"));
        assert!(json.contains("\"response_fields\":[\"ok\",\"request_id\",\"profile\",\"answer\",\"raw_answer\",\"enhanced_answer\",\"runtime_token_count\",\"runtime_uncertainty_signal\",\"traceable\",\"error\"]"));
        assert!(json.contains("\"unsupported_fields\":[\"messages\",\"stream\",\"tools\",\"tool_choice\",\"response_format\"]"));
    }

    #[test]
    fn endpoint_info_json_reports_stream_contract_fields() {
        let json = model_service_endpoint_info_json(14, "generate-stream");

        assert!(json.contains("\"endpoint\":\"/v1/generate-stream\""));
        assert!(json.contains("\"supported_fields\":[\"prompt\",\"profile\",\"case\",\"output\",\"max_tokens\",\"tenant_id\",\"workspace_id\",\"session_id\"]"));
        assert!(json.contains("\"response_fields\":[\"event:status\",\"event:delta\",\"event:final\",\"event:done\",\"event:error\"]"));
        assert!(json.contains(
            "\"unsupported_fields\":[\"messages\",\"tools\",\"tool_choice\",\"response_format\"]"
        ));
    }

    #[test]
    fn endpoint_info_json_reports_experience_hygiene_quarantine_route() {
        let json = model_service_endpoint_info_json(4, "experience-hygiene-quarantine");

        assert!(json.contains("\"endpoint\":\"/v1/experience-hygiene/quarantine\""));
        assert!(json.contains("\"apply\":false"));
        assert!(json.contains("\"limit\":20"));
    }

    #[test]
    fn endpoint_info_json_reports_experience_retrieval_route() {
        let json = model_service_endpoint_info_json(5, "experience-retrieval");

        assert!(json.contains("\"endpoint\":\"/v1/experience-retrieval\""));
        assert!(json.contains("\"prompt\""));
        assert!(json.contains("\"profile\":\"coding\""));
    }

    #[test]
    fn endpoint_info_json_reports_experience_repair_route() {
        let json = model_service_endpoint_info_json(6, "experience-repair");

        assert!(json.contains("\"endpoint\":\"/v1/experience-repair\""));
        assert!(json.contains("\"apply\":false"));
        assert!(json.contains("\"limit\":20"));
    }

    #[test]
    fn endpoint_info_json_reports_model_pool_route_plan() {
        let json = model_service_endpoint_info_json(9, "model-pool-route-plan");

        assert!(json.contains("\"endpoint\":\"/v1/model-pool/route-plan\""));
        assert!(json.contains("\"task_kind\":\"review\""));
    }

    #[test]
    fn endpoint_info_json_reports_request_cancel_route() {
        let json = model_service_endpoint_info_json(10, "requests-cancel");

        assert!(json.contains("\"endpoint\":\"/v1/requests/cancel\""));
        assert!(json.contains("\"request_id\":42"));
        assert!(json.contains("operator_runtime_splice"));
    }

    #[test]
    fn endpoint_info_json_reports_experience_cleanup_audit_route() {
        let json = model_service_endpoint_info_json(8, "experience-cleanup-audit");

        assert!(json.contains("\"endpoint\":\"/v1/experience-cleanup-audit\""));
        assert!(json.contains("\"limit\":20"));
    }

    #[test]
    fn endpoint_info_json_keeps_generate_fallback() {
        let json = model_service_endpoint_info_json(1, "unknown");

        assert!(json.contains("\"endpoint\":\"/v1/generate\""));
        assert!(json.contains("\"manual-generate\""));
    }
}
