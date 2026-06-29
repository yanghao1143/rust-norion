use super::super::http::split_http_head_body;
use super::super::json::json_bool_field;
use super::business_cycle::{ModelServiceBusinessCycleRequest, parse_business_cycle_request};
use super::chat::{ModelServiceChatRequest, parse_chat_request};
use super::experience_cleanup_audit::{
    ModelServiceExperienceCleanupAuditRequest, parse_experience_cleanup_audit_request,
};
use super::experience_hygiene::{
    ModelServiceExperienceHygieneQuarantineRequest, parse_experience_hygiene_quarantine_request,
};
use super::experience_repair::{
    ModelServiceExperienceRepairRequest, parse_experience_repair_request,
};
use super::experience_retrieval::{
    ModelServiceExperienceRetrievalRequest, parse_experience_retrieval_request,
};
use super::feedback::{ModelServiceFeedbackRequest, parse_feedback_request};
use super::generate::{
    ModelServiceOpenAiCompletionRequest, ModelServiceRequest, parse_generate_request,
    parse_openai_completion_request,
};
use super::inspect::{ModelServiceInspectRequest, parse_model_service_gate_request};
use super::model_pool::{
    ModelServiceModelPoolCallRequest, ModelServiceModelPoolRouteRequest,
    parse_model_pool_call_request, parse_model_pool_route_request,
};
use super::replay::{
    ModelServiceReplayRequest, ModelServiceSelfImproveRequest, parse_replay_request,
    parse_self_improve_request,
};
use super::request_control::{ModelServiceRequestCancelRequest, parse_request_cancel_request};
use super::rust_check::{ModelServiceRustCheckRequest, parse_rust_check_request};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ModelServiceHttpRequest {
    Health,
    State,
    ExperienceHygiene,
    ExperienceHygieneQuarantine(ModelServiceExperienceHygieneQuarantineRequest),
    ExperienceCleanupAudit(ModelServiceExperienceCleanupAuditRequest),
    ExperienceRepair(ModelServiceExperienceRepairRequest),
    ExperienceRetrieval(ModelServiceExperienceRetrievalRequest),
    ModelPoolManifest,
    ModelPoolStatus,
    ModelPoolRoute(ModelServiceModelPoolRouteRequest),
    ModelPoolCall(ModelServiceModelPoolCallRequest),
    ModelCapabilities,
    Info(&'static str),
    Generate(ModelServiceRequest),
    GenerateStream(ModelServiceRequest),
    OpenAiCompletions(ModelServiceOpenAiCompletionRequest),
    Chat(ModelServiceChatRequest),
    OpenAiChatCompletions(ModelServiceChatRequest),
    OpenAiChatCompletionsStream(ModelServiceChatRequest),
    ChatStream(ModelServiceChatRequest),
    Replay(ModelServiceReplayRequest),
    SelfImprove(ModelServiceSelfImproveRequest),
    BusinessCycle(ModelServiceBusinessCycleRequest),
    BusinessCycleStream(ModelServiceBusinessCycleRequest),
    RequestCancel(ModelServiceRequestCancelRequest),
    Inspect(ModelServiceInspectRequest),
    Feedback(ModelServiceFeedbackRequest),
    RustCheck(ModelServiceRustCheckRequest),
}

pub(crate) fn parse_model_service_http_request(
    raw: &str,
) -> Result<ModelServiceHttpRequest, String> {
    let (head, body) = split_http_head_body(raw);
    let request_line = head
        .lines()
        .next()
        .ok_or_else(|| "missing HTTP request line".to_owned())?;
    let mut parts = request_line.split_whitespace();
    let method = parts
        .next()
        .ok_or_else(|| "missing HTTP method".to_owned())?;
    let path = parts.next().ok_or_else(|| "missing HTTP path".to_owned())?;

    if method.eq_ignore_ascii_case("GET") {
        return match path {
            "/health" => Ok(ModelServiceHttpRequest::Health),
            "/state" | "/v1/state" => Ok(ModelServiceHttpRequest::State),
            "/experience-hygiene" | "/v1/experience-hygiene" => {
                Ok(ModelServiceHttpRequest::ExperienceHygiene)
            }
            "/experience-cleanup-audit"
            | "/v1/experience-cleanup-audit"
            | "/cleanup-audit"
            | "/v1/cleanup-audit" => Ok(ModelServiceHttpRequest::Info("experience-cleanup-audit")),
            "/experience-hygiene/quarantine" | "/v1/experience-hygiene/quarantine" => Ok(
                ModelServiceHttpRequest::Info("experience-hygiene-quarantine"),
            ),
            "/experience-repair" | "/v1/experience-repair" => {
                Ok(ModelServiceHttpRequest::Info("experience-repair"))
            }
            "/experience-retrieval" | "/v1/experience-retrieval" => {
                Ok(ModelServiceHttpRequest::Info("experience-retrieval"))
            }
            "/model-pool/manifest" | "/v1/model-pool/manifest" => {
                Ok(ModelServiceHttpRequest::ModelPoolManifest)
            }
            "/model-pool/status" | "/v1/model-pool/status" => {
                Ok(ModelServiceHttpRequest::ModelPoolStatus)
            }
            "/model-pool/route-plan" | "/v1/model-pool/route-plan" => {
                Ok(ModelServiceHttpRequest::Info("model-pool-route-plan"))
            }
            "/model-pool/call" | "/v1/model-pool/call" => {
                Ok(ModelServiceHttpRequest::Info("model-pool-call"))
            }
            "/models" | "/v1/models" => Ok(ModelServiceHttpRequest::ModelCapabilities),
            "/diagnostics" | "/v1/diagnostics" => Ok(ModelServiceHttpRequest::Health),
            "/generate" | "/v1/generate" => Ok(ModelServiceHttpRequest::Info("generate")),
            "/v1/completions" | "/completions" => Ok(ModelServiceHttpRequest::Info("completions")),
            "/chat" | "/v1/chat" => Ok(ModelServiceHttpRequest::Info("chat")),
            "/v1/chat/completions" | "/chat/completions" => {
                Ok(ModelServiceHttpRequest::Info("chat-completions"))
            }
            "/generate-stream" | "/v1/generate-stream" => {
                Ok(ModelServiceHttpRequest::Info("generate-stream"))
            }
            "/chat-stream" | "/v1/chat-stream" => Ok(ModelServiceHttpRequest::Info("chat-stream")),
            "/business-cycle" | "/v1/business-cycle" => {
                Ok(ModelServiceHttpRequest::Info("business-cycle"))
            }
            "/business-cycle-stream" | "/v1/business-cycle-stream" => {
                Ok(ModelServiceHttpRequest::Info("business-cycle-stream"))
            }
            "/replay" | "/v1/replay" => Ok(ModelServiceHttpRequest::Info("replay")),
            "/self-improve" | "/v1/self-improve" => {
                Ok(ModelServiceHttpRequest::Info("self-improve"))
            }
            "/feedback" | "/v1/feedback" => Ok(ModelServiceHttpRequest::Info("feedback")),
            "/rust-check" | "/v1/rust-check" => Ok(ModelServiceHttpRequest::Info("rust-check")),
            "/requests/cancel" | "/v1/requests/cancel" => {
                Ok(ModelServiceHttpRequest::Info("requests-cancel"))
            }
            "/inspect" | "/v1/inspect" => Ok(ModelServiceHttpRequest::Inspect(
                ModelServiceInspectRequest::default(),
            )),
            _ => Err(format!("unsupported HTTP path: {path}")),
        };
    }

    if !method.eq_ignore_ascii_case("POST") {
        return Err(format!("unsupported HTTP method: {method}"));
    }

    match path {
        "/v1/generate" | "/generate" => {
            parse_generate_request(body).map(ModelServiceHttpRequest::Generate)
        }
        "/v1/generate-stream" | "/generate-stream" => {
            parse_generate_request(body).map(ModelServiceHttpRequest::GenerateStream)
        }
        "/v1/completions" | "/completions" => {
            if json_bool_field(body, "stream").unwrap_or(false) {
                return Err(
                    "OpenAI completions stream=true is not supported; use /v1/chat/completions stream=true"
                        .to_owned(),
                );
            }
            parse_openai_completion_request(body).map(ModelServiceHttpRequest::OpenAiCompletions)
        }
        "/v1/chat" | "/chat" => parse_chat_request(body).map(ModelServiceHttpRequest::Chat),
        "/v1/chat/completions" | "/chat/completions" => {
            if json_bool_field(body, "stream").unwrap_or(false) {
                return parse_chat_request(body)
                    .map(ModelServiceHttpRequest::OpenAiChatCompletionsStream);
            }
            parse_chat_request(body).map(ModelServiceHttpRequest::OpenAiChatCompletions)
        }
        "/v1/chat-stream" | "/chat-stream" => {
            parse_chat_request(body).map(ModelServiceHttpRequest::ChatStream)
        }
        "/v1/replay" | "/replay" => Ok(ModelServiceHttpRequest::Replay(parse_replay_request(body))),
        "/v1/self-improve" | "/self-improve" => {
            parse_self_improve_request(body).map(ModelServiceHttpRequest::SelfImprove)
        }
        "/v1/business-cycle" | "/business-cycle" => {
            parse_business_cycle_request(body).map(ModelServiceHttpRequest::BusinessCycle)
        }
        "/v1/business-cycle-stream" | "/business-cycle-stream" => {
            parse_business_cycle_request(body).map(ModelServiceHttpRequest::BusinessCycleStream)
        }
        "/v1/requests/cancel" | "/requests/cancel" => {
            parse_request_cancel_request(body).map(ModelServiceHttpRequest::RequestCancel)
        }
        "/v1/feedback" | "/feedback" => {
            parse_feedback_request(body).map(ModelServiceHttpRequest::Feedback)
        }
        "/v1/rust-check" | "/rust-check" => {
            parse_rust_check_request(body).map(ModelServiceHttpRequest::RustCheck)
        }
        "/v1/experience-hygiene/quarantine" | "/experience-hygiene/quarantine" => {
            Ok(ModelServiceHttpRequest::ExperienceHygieneQuarantine(
                parse_experience_hygiene_quarantine_request(body),
            ))
        }
        "/v1/experience-cleanup-audit"
        | "/experience-cleanup-audit"
        | "/v1/cleanup-audit"
        | "/cleanup-audit" => Ok(ModelServiceHttpRequest::ExperienceCleanupAudit(
            parse_experience_cleanup_audit_request(body),
        )),
        "/v1/experience-repair" | "/experience-repair" => Ok(
            ModelServiceHttpRequest::ExperienceRepair(parse_experience_repair_request(body)),
        ),
        "/v1/experience-retrieval" | "/experience-retrieval" => {
            parse_experience_retrieval_request(body)
                .map(ModelServiceHttpRequest::ExperienceRetrieval)
        }
        "/v1/model-pool/status" | "/model-pool/status" => {
            Ok(ModelServiceHttpRequest::ModelPoolStatus)
        }
        "/v1/model-pool/route-plan" | "/model-pool/route-plan" => {
            parse_model_pool_route_request(body).map(ModelServiceHttpRequest::ModelPoolRoute)
        }
        "/v1/model-pool/call" | "/model-pool/call" => {
            parse_model_pool_call_request(body).map(ModelServiceHttpRequest::ModelPoolCall)
        }
        "/v1/inspect" | "/inspect" => {
            let request = parse_model_service_gate_request(body, "inspect")?;
            Ok(ModelServiceHttpRequest::Inspect(request))
        }
        _ => Err(format!("unsupported HTTP path: {path}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_chat_stream_route() {
        let raw = "POST /v1/chat-stream HTTP/1.1\r\ncontent-length: 79\r\n\r\n{\"messages\":[{\"role\":\"user\",\"content\":\"你好\"}],\"profile\":\"coding\",\"output\":\"raw\"}";

        let request = parse_model_service_http_request(raw).unwrap();

        assert!(matches!(request, ModelServiceHttpRequest::ChatStream(_)));
    }

    #[test]
    fn parses_openai_chat_completions_route() {
        let raw = "POST /v1/chat/completions HTTP/1.1\r\ncontent-length: 87\r\n\r\n{\"model\":\"norion-local\",\"messages\":[{\"role\":\"user\",\"content\":\"你好\"}],\"max_tokens\":8}";

        let request = parse_model_service_http_request(raw).unwrap();

        let ModelServiceHttpRequest::OpenAiChatCompletions(request) = request else {
            panic!("expected OpenAI chat completions request");
        };
        assert_eq!(request.model.as_deref(), Some("norion-local"));
        assert_eq!(request.max_tokens, Some(8));
    }

    #[test]
    fn parses_openai_completions_route() {
        let raw = "POST /v1/completions HTTP/1.1\r\ncontent-length: 79\r\n\r\n{\"model\":\"norion-local\",\"prompt\":\"用中文解释 Rust 所有权\",\"max_tokens\":8}";

        let request = parse_model_service_http_request(raw).unwrap();

        let ModelServiceHttpRequest::OpenAiCompletions(request) = request else {
            panic!("expected OpenAI completions request");
        };
        assert_eq!(request.model.as_deref(), Some("norion-local"));
        assert_eq!(request.generate.prompt, "用中文解释 Rust 所有权");
        assert_eq!(request.generate.max_tokens, Some(8));
    }

    #[test]
    fn parses_openai_models_route() {
        let request = parse_model_service_http_request("GET /v1/models HTTP/1.1\r\n\r\n").unwrap();

        assert_eq!(request, ModelServiceHttpRequest::ModelCapabilities);
    }

    #[test]
    fn parses_versioned_diagnostics_route_as_health() {
        let request =
            parse_model_service_http_request("GET /v1/diagnostics HTTP/1.1\r\n\r\n").unwrap();

        assert_eq!(request, ModelServiceHttpRequest::Health);
    }

    #[test]
    fn parses_openai_chat_completions_stream_true_route() {
        let raw = "POST /v1/chat/completions HTTP/1.1\r\ncontent-length: 72\r\n\r\n{\"messages\":[{\"role\":\"user\",\"content\":\"你好\"}],\"stream\":true}";

        let request = parse_model_service_http_request(raw).unwrap();

        assert!(matches!(
            request,
            ModelServiceHttpRequest::OpenAiChatCompletionsStream(_)
        ));
    }

    #[test]
    fn parses_generate_stream_route() {
        let raw = "POST /v1/generate-stream HTTP/1.1\r\ncontent-length: 49\r\n\r\n{\"prompt\":\"你好\",\"profile\":\"general\",\"output\":\"raw\"}";

        let request = parse_model_service_http_request(raw).unwrap();

        assert!(matches!(
            request,
            ModelServiceHttpRequest::GenerateStream(_)
        ));
    }

    #[test]
    fn parses_experience_hygiene_routes() {
        let report =
            parse_model_service_http_request("GET /v1/experience-hygiene HTTP/1.1\r\n\r\n")
                .unwrap();
        assert_eq!(report, ModelServiceHttpRequest::ExperienceHygiene);

        let quarantine = parse_model_service_http_request(
            "POST /v1/experience-hygiene/quarantine HTTP/1.1\r\n\r\n{\"apply\":true,\"limit\":2}",
        )
        .unwrap();

        assert_eq!(
            quarantine,
            ModelServiceHttpRequest::ExperienceHygieneQuarantine(
                ModelServiceExperienceHygieneQuarantineRequest {
                    apply: true,
                    limit: Some(2),
                    backup_path: None,
                    quarantine_path: None,
                }
            )
        );
    }

    #[test]
    fn parses_experience_retrieval_route() {
        let info =
            parse_model_service_http_request("GET /v1/experience-retrieval HTTP/1.1\r\n\r\n")
                .unwrap();
        assert_eq!(info, ModelServiceHttpRequest::Info("experience-retrieval"));

        let retrieval = parse_model_service_http_request(
            "POST /v1/experience-retrieval HTTP/1.1\r\n\r\n{\"prompt\":\"rust loop\",\"profile\":\"coding\",\"limit\":2}",
        )
        .unwrap();

        assert_eq!(
            retrieval,
            ModelServiceHttpRequest::ExperienceRetrieval(ModelServiceExperienceRetrievalRequest {
                prompt: "rust loop".to_owned(),
                profile: Some(rust_norion::TaskProfile::Coding),
                limit: Some(2),
                index_context: None,
            })
        );

        let retrieval_with_index = parse_model_service_http_request(
            "POST /v1/experience-retrieval HTTP/1.1\r\n\r\n{\"prompt\":\"route code\",\"profile\":\"coding\",\"limit\":2,\"index_context\":\"src/model_service\"}",
        )
        .unwrap();

        assert_eq!(
            retrieval_with_index,
            ModelServiceHttpRequest::ExperienceRetrieval(ModelServiceExperienceRetrievalRequest {
                prompt: "route code".to_owned(),
                profile: Some(rust_norion::TaskProfile::Coding),
                limit: Some(2),
                index_context: Some("src/model_service".to_owned()),
            })
        );
    }

    #[test]
    fn parses_experience_repair_route() {
        let info =
            parse_model_service_http_request("GET /v1/experience-repair HTTP/1.1\r\n\r\n").unwrap();
        assert_eq!(info, ModelServiceHttpRequest::Info("experience-repair"));

        let repair = parse_model_service_http_request(
            "POST /v1/experience-repair HTTP/1.1\r\n\r\n{\"apply\":true,\"limit\":2,\"backup_path\":\"repair-backup.ndkv\"}",
        )
        .unwrap();

        assert_eq!(
            repair,
            ModelServiceHttpRequest::ExperienceRepair(ModelServiceExperienceRepairRequest {
                apply: true,
                limit: Some(2),
                backup_path: Some(std::path::PathBuf::from("repair-backup.ndkv")),
            })
        );
    }

    #[test]
    fn parses_experience_cleanup_audit_route() {
        let get =
            parse_model_service_http_request("GET /v1/experience-cleanup-audit HTTP/1.1\r\n\r\n")
                .unwrap();
        assert_eq!(
            get,
            ModelServiceHttpRequest::Info("experience-cleanup-audit")
        );

        let post = parse_model_service_http_request(
            "POST /v1/experience-cleanup-audit HTTP/1.1\r\n\r\n{\"limit\":7}",
        )
        .unwrap();

        assert_eq!(
            post,
            ModelServiceHttpRequest::ExperienceCleanupAudit(
                ModelServiceExperienceCleanupAuditRequest { limit: Some(7) }
            )
        );
    }

    #[test]
    fn parses_business_cycle_stream_route() {
        let info =
            parse_model_service_http_request("GET /v1/business-cycle HTTP/1.1\r\n\r\n").unwrap();
        assert_eq!(info, ModelServiceHttpRequest::Info("business-cycle"));

        let raw = "POST /v1/business-cycle-stream HTTP/1.1\r\ncontent-length: 69\r\n\r\n{\"prompt\":\"业务联调\",\"feedback_amount\":0.4,\"self_improve\":false}";

        let request = parse_model_service_http_request(raw).unwrap();

        assert!(matches!(
            request,
            ModelServiceHttpRequest::BusinessCycleStream(_)
        ));
    }

    #[test]
    fn parses_request_cancel_route() {
        let info =
            parse_model_service_http_request("GET /v1/requests/cancel HTTP/1.1\r\n\r\n").unwrap();
        assert_eq!(info, ModelServiceHttpRequest::Info("requests-cancel"));

        let request = parse_model_service_http_request(
            "POST /v1/requests/cancel HTTP/1.1\r\n\r\n{\"request_id\":42,\"reason\":\"stalled_generation\",\"retag_label\":\"repair_factor:runtime_splice\"}",
        )
        .unwrap();

        assert_eq!(
            request,
            ModelServiceHttpRequest::RequestCancel(ModelServiceRequestCancelRequest {
                request_id: 42,
                reason: "stalled_generation".to_owned(),
                retag_label: "repair_factor:runtime_splice".to_owned(),
            })
        );
    }

    #[test]
    fn parses_evolution_endpoint_info_routes() {
        let replay = parse_model_service_http_request("GET /v1/replay HTTP/1.1\r\n\r\n").unwrap();
        assert_eq!(replay, ModelServiceHttpRequest::Info("replay"));

        let self_improve =
            parse_model_service_http_request("GET /v1/self-improve HTTP/1.1\r\n\r\n").unwrap();
        assert_eq!(self_improve, ModelServiceHttpRequest::Info("self-improve"));

        let feedback =
            parse_model_service_http_request("GET /v1/feedback HTTP/1.1\r\n\r\n").unwrap();
        assert_eq!(feedback, ModelServiceHttpRequest::Info("feedback"));

        let rust_check =
            parse_model_service_http_request("GET /v1/rust-check HTTP/1.1\r\n\r\n").unwrap();
        assert_eq!(rust_check, ModelServiceHttpRequest::Info("rust-check"));
    }

    #[test]
    fn parses_model_pool_routes() {
        let manifest =
            parse_model_service_http_request("GET /v1/model-pool/manifest HTTP/1.1\r\n\r\n")
                .unwrap();
        assert_eq!(manifest, ModelServiceHttpRequest::ModelPoolManifest);

        let status =
            parse_model_service_http_request("GET /v1/model-pool/status HTTP/1.1\r\n\r\n").unwrap();
        assert_eq!(status, ModelServiceHttpRequest::ModelPoolStatus);

        let route = parse_model_service_http_request(
            "POST /v1/model-pool/route-plan HTTP/1.1\r\n\r\n{\"task_kind\":\"review\"}",
        )
        .unwrap();
        assert_eq!(
            route,
            ModelServiceHttpRequest::ModelPoolRoute(ModelServiceModelPoolRouteRequest {
                task_kind: "review".to_owned(),
                max_tokens: None,
                prompt: None,
                completed_roles: None,
            })
        );

        let call = parse_model_service_http_request(
            "POST /v1/model-pool/call HTTP/1.1\r\n\r\n{\"task_kind\":\"summary\",\"prompt\":\"summarize logs\"}",
        )
        .unwrap();
        assert_eq!(
            call,
            ModelServiceHttpRequest::ModelPoolCall(ModelServiceModelPoolCallRequest {
                task_kind: "summary".to_owned(),
                prompt: "summarize logs".to_owned(),
                max_tokens: None,
                completed_roles: None,
            })
        );

        let dependency_route = parse_model_service_http_request(
            "POST /v1/model-pool/route-plan HTTP/1.1\r\n\r\n{\"task_kind\":\"index\",\"completed_roles\":[\"quality\",\"summary\",\"route\"]}",
        )
        .unwrap();
        assert_eq!(
            dependency_route,
            ModelServiceHttpRequest::ModelPoolRoute(ModelServiceModelPoolRouteRequest {
                task_kind: "index".to_owned(),
                max_tokens: None,
                prompt: None,
                completed_roles: Some(vec![
                    "quality".to_owned(),
                    "summary".to_owned(),
                    "router".to_owned()
                ]),
            })
        );
    }
}
