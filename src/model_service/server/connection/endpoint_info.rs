use std::net::TcpStream;

use super::super::super::json::write_http_json;

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
    format!(
        "{{\"ok\":true,\"request_id\":{},\"endpoint\":\"{}\",\"method\":\"POST\",\"content_type\":\"application/json\",\"example\":{}}}",
        request_id, spec.path, spec.example
    )
}

struct EndpointInfoSpec {
    path: &'static str,
    example: &'static str,
}

impl EndpointInfoSpec {
    fn for_endpoint(endpoint: &str) -> Self {
        match endpoint {
            "chat" => Self {
                path: "/v1/chat",
                example: "{\"messages\":[{\"role\":\"user\",\"content\":\"用中文给一个 rust-norion 业务联调建议。\"}],\"profile\":\"coding\",\"case\":\"manual-chat\",\"output\":\"raw\"}",
            },
            "chat-stream" => Self {
                path: "/v1/chat-stream",
                example: "{\"messages\":[{\"role\":\"user\",\"content\":\"用中文流式测试 SmartSteam Forge。\"}],\"profile\":\"coding\",\"case\":\"manual-chat-stream\",\"output\":\"raw\"}",
            },
            "generate-stream" => Self {
                path: "/v1/generate-stream",
                example: "{\"prompt\":\"用中文流式测试 rust-norion 本地模型服务。\",\"profile\":\"coding\",\"case\":\"manual-generate-stream\",\"output\":\"raw\"}",
            },
            "business-cycle" => Self {
                path: "/v1/business-cycle",
                example: "{\"prompt\":\"用中文完成一次 SmartSteam 业务联调自检。\",\"feedback_amount\":0.4,\"self_improve\":true,\"rust_check_code\":\"fn main() {}\"}",
            },
            "business-cycle-stream" => Self {
                path: "/v1/business-cycle-stream",
                example: "{\"prompt\":\"用中文流式完成一次 SmartSteam 业务联调自检。\",\"feedback_amount\":0.4,\"self_improve\":true,\"rust_check_code\":\"fn main() {}\"}",
            },
            "experience-hygiene-quarantine" => Self {
                path: "/v1/experience-hygiene/quarantine",
                example: "{\"apply\":false,\"limit\":20}",
            },
            "experience-cleanup-audit" => Self {
                path: "/v1/experience-cleanup-audit",
                example: "{\"limit\":20}",
            },
            "experience-repair" => Self {
                path: "/v1/experience-repair",
                example: "{\"apply\":false,\"limit\":20}",
            },
            "experience-retrieval" => Self {
                path: "/v1/experience-retrieval",
                example: "{\"prompt\":\"帮我用rust输出一段for循环代码\",\"profile\":\"coding\",\"limit\":5}",
            },
            "model-pool-route-plan" => Self {
                path: "/v1/model-pool/route-plan",
                example: "{\"task_kind\":\"review\"}",
            },
            _ => Self {
                path: "/v1/generate",
                example: "{\"prompt\":\"用中文给一个 rust-norion 业务联调建议。\",\"profile\":\"coding\",\"case\":\"manual-generate\",\"output\":\"raw\"}",
            },
        }
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
