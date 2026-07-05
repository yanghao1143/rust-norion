use super::super::http::split_http_head_body;
use super::super::json::{json_bool_field, json_usize_field};
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

const OPENAI_SAMPLING_UNSUPPORTED_FIELDS: &[&str] = &[
    "temperature",
    "top_p",
    "presence_penalty",
    "frequency_penalty",
    "stop",
    "seed",
    "logit_bias",
];

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
            "/inspect" | "/v1/inspect" => Ok(ModelServiceHttpRequest::Info("inspect")),
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
            reject_unsupported_fields(body, "OpenAI completions", &["stream_options"])?;
            if json_bool_field(body, "stream").unwrap_or(false) {
                return Err(
                    "OpenAI completions stream=true is not supported; use /v1/chat/completions stream=true"
                        .to_owned(),
                );
            }
            reject_unsupported_choice_count(body, "OpenAI completions")?;
            reject_unsupported_fields(
                body,
                "OpenAI completions",
                OPENAI_SAMPLING_UNSUPPORTED_FIELDS,
            )?;
            reject_unsupported_fields(body, "OpenAI completions", &["logprobs", "suffix"])?;
            parse_openai_completion_request(body).map(ModelServiceHttpRequest::OpenAiCompletions)
        }
        "/v1/chat" | "/chat" => parse_chat_request(body).map(ModelServiceHttpRequest::Chat),
        "/v1/chat/completions" | "/chat/completions" => {
            reject_unsupported_fields(body, "OpenAI chat completions", &["stream_options"])?;
            reject_unsupported_choice_count(body, "OpenAI chat completions")?;
            reject_unsupported_fields(
                body,
                "OpenAI chat completions",
                OPENAI_SAMPLING_UNSUPPORTED_FIELDS,
            )?;
            reject_unsupported_fields(
                body,
                "OpenAI chat completions",
                &["tools", "tool_choice", "response_format", "logprobs"],
            )?;
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

fn reject_unsupported_fields(body: &str, endpoint: &str, fields: &[&str]) -> Result<(), String> {
    for field in fields {
        if json_top_level_key_present(body, field) {
            return Err(format!(
                "{endpoint} does not support request field: {field}"
            ));
        }
    }
    Ok(())
}

fn reject_unsupported_choice_count(body: &str, endpoint: &str) -> Result<(), String> {
    if !json_top_level_key_present(body, "n") {
        return Ok(());
    }
    if json_usize_field(body, "n") == Some(1) {
        return Ok(());
    }
    Err(format!("{endpoint} only supports request field n=1"))
}

fn json_top_level_key_present(body: &str, field: &str) -> bool {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    let mut string_start = None;

    for (index, character) in body.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
                if depth == 1
                    && let Some(start) = string_start.take()
                    && body.get(start..index) == Some(field)
                    && body
                        .get(index + character.len_utf8()..)
                        .and_then(|rest| rest.trim_start().chars().next())
                        == Some(':')
                {
                    return true;
                }
            }
            continue;
        }

        match character {
            '"' => {
                in_string = true;
                string_start = Some(index + character.len_utf8());
            }
            '{' => depth = depth.saturating_add(1),
            '}' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_chat_stream_route() {
        let raw = "POST /v1/chat-stream HTTP/1.1\r\ncontent-length: 148\r\n\r\n{\"messages\":[{\"role\":\"user\",\"content\":\"你好\"}],\"profile\":\"coding\",\"output\":\"raw\",\"tenant_id\":\"tenant-a\",\"workspace_id\":\"workspace\",\"session_id\":\"chat-stream-1\"}";

        let request = parse_model_service_http_request(raw).unwrap();

        assert!(matches!(request, ModelServiceHttpRequest::ChatStream(_)));
    }

    #[test]
    fn parses_openai_chat_completions_route() {
        let raw = "POST /v1/chat/completions HTTP/1.1\r\ncontent-length: 157\r\n\r\n{\"model\":\"norion-local\",\"messages\":[{\"role\":\"user\",\"content\":\"你好\"}],\"max_tokens\":8,\"tenant_id\":\"tenant-a\",\"workspace_id\":\"workspace\",\"session_id\":\"chat-1\"}";

        let request = parse_model_service_http_request(raw).unwrap();

        let ModelServiceHttpRequest::OpenAiChatCompletions(request) = request else {
            panic!("expected OpenAI chat completions request");
        };
        assert_eq!(request.model.as_deref(), Some("norion-local"));
        assert_eq!(request.max_tokens, Some(8));
    }

    #[test]
    fn parses_openai_chat_max_completion_tokens_alias() {
        let raw = "POST /v1/chat/completions HTTP/1.1\r\n\r\n{\"model\":\"norion-local\",\"messages\":[{\"role\":\"user\",\"content\":\"你好\"}],\"max_completion_tokens\":9,\"tenant_id\":\"tenant-a\",\"workspace_id\":\"workspace\",\"session_id\":\"chat-2\"}";

        let request = parse_model_service_http_request(raw).unwrap();

        let ModelServiceHttpRequest::OpenAiChatCompletions(request) = request else {
            panic!("expected OpenAI chat completions request");
        };
        assert_eq!(request.max_tokens, Some(9));
    }

    #[test]
    fn validates_openai_choice_count() {
        parse_model_service_http_request(
            "POST /v1/chat/completions HTTP/1.1\r\n\r\n{\"messages\":[{\"role\":\"user\",\"content\":\"hi\"}],\"n\":1,\"tenant_id\":\"tenant-a\",\"workspace_id\":\"workspace\",\"session_id\":\"chat-n\"}",
        )
        .unwrap();
        parse_model_service_http_request(
            "POST /v1/completions HTTP/1.1\r\n\r\n{\"model\":\"norion-local\",\"prompt\":\"hi\",\"n\":\"1\",\"tenant_id\":\"tenant-a\",\"workspace_id\":\"workspace\",\"session_id\":\"completion-n\"}",
        )
        .unwrap();

        let chat_error = parse_model_service_http_request(
            "POST /v1/chat/completions HTTP/1.1\r\n\r\n{\"messages\":[{\"role\":\"user\",\"content\":\"hi\"}],\"n\":2}",
        )
        .unwrap_err();
        assert_eq!(
            chat_error,
            "OpenAI chat completions only supports request field n=1"
        );

        let completion_error = parse_model_service_http_request(
            "POST /v1/completions HTTP/1.1\r\n\r\n{\"model\":\"norion-local\",\"prompt\":\"hi\",\"n\":2}",
        )
        .unwrap_err();
        assert_eq!(
            completion_error,
            "OpenAI completions only supports request field n=1"
        );

        parse_model_service_http_request(
            "POST /v1/chat/completions HTTP/1.1\r\n\r\n{\"messages\":[{\"role\":\"user\",\"content\":\"mention \\\"n\\\" as text\"}],\"tenant_id\":\"tenant-a\",\"workspace_id\":\"workspace\",\"session_id\":\"chat-mention-n\"}",
        )
        .unwrap();
    }

    #[test]
    fn parses_openai_completions_route() {
        let raw = "POST /v1/completions HTTP/1.1\r\ncontent-length: 155\r\n\r\n{\"model\":\"norion-local\",\"prompt\":\"用中文解释 Rust 所有权\",\"max_tokens\":8,\"tenant_id\":\"tenant-a\",\"workspace_id\":\"workspace\",\"session_id\":\"completion-1\"}";

        let request = parse_model_service_http_request(raw).unwrap();

        let ModelServiceHttpRequest::OpenAiCompletions(request) = request else {
            panic!("expected OpenAI completions request");
        };
        assert_eq!(request.model.as_deref(), Some("norion-local"));
        assert_eq!(request.generate.prompt, "用中文解释 Rust 所有权");
        assert_eq!(request.generate.max_tokens, Some(8));
    }

    #[test]
    fn rejects_unsupported_openai_request_fields() {
        let chat_error = parse_model_service_http_request(
            "POST /v1/chat/completions HTTP/1.1\r\n\r\n{\"messages\":[{\"role\":\"user\",\"content\":\"hi\"}],\"tools\":[]}",
        )
        .unwrap_err();
        assert_eq!(
            chat_error,
            "OpenAI chat completions does not support request field: tools"
        );

        let completion_error = parse_model_service_http_request(
            "POST /v1/completions HTTP/1.1\r\n\r\n{\"model\":\"norion-local\",\"prompt\":\"hi\",\"logprobs\":1}",
        )
        .unwrap_err();
        assert_eq!(
            completion_error,
            "OpenAI completions does not support request field: logprobs"
        );

        let sampling_error = parse_model_service_http_request(
            "POST /v1/chat/completions HTTP/1.1\r\n\r\n{\"messages\":[{\"role\":\"user\",\"content\":\"hi\"}],\"temperature\":0.7}",
        )
        .unwrap_err();
        assert_eq!(
            sampling_error,
            "OpenAI chat completions does not support request field: temperature"
        );

        let stop_error = parse_model_service_http_request(
            "POST /v1/completions HTTP/1.1\r\n\r\n{\"model\":\"norion-local\",\"prompt\":\"hi\",\"stop\":\"END\"}",
        )
        .unwrap_err();
        assert_eq!(
            stop_error,
            "OpenAI completions does not support request field: stop"
        );

        let stream_options_error = parse_model_service_http_request(
            "POST /v1/chat/completions HTTP/1.1\r\n\r\n{\"messages\":[{\"role\":\"user\",\"content\":\"hi\"}],\"stream\":true,\"stream_options\":{\"include_usage\":true}}",
        )
        .unwrap_err();
        assert_eq!(
            stream_options_error,
            "OpenAI chat completions does not support request field: stream_options"
        );

        let completion_stream_options_error = parse_model_service_http_request(
            "POST /v1/completions HTTP/1.1\r\n\r\n{\"model\":\"norion-local\",\"prompt\":\"hi\",\"stream_options\":{\"include_usage\":true}}",
        )
        .unwrap_err();
        assert_eq!(
            completion_stream_options_error,
            "OpenAI completions does not support request field: stream_options"
        );

        parse_model_service_http_request(
            "POST /v1/chat/completions HTTP/1.1\r\n\r\n{\"messages\":[{\"role\":\"user\",\"content\":\"mention \\\"tools\\\" as text\"}],\"tenant_id\":\"tenant-a\",\"workspace_id\":\"workspace\",\"session_id\":\"chat-mention-tools\"}",
        )
        .unwrap();
        parse_model_service_http_request(
            "POST /v1/chat/completions HTTP/1.1\r\n\r\n{\"messages\":[{\"role\":\"user\",\"content\":\"mention \\\"temperature\\\" as text\"}],\"tenant_id\":\"tenant-a\",\"workspace_id\":\"workspace\",\"session_id\":\"chat-mention-temperature\"}",
        )
        .unwrap();
        parse_model_service_http_request(
            "POST /v1/chat/completions HTTP/1.1\r\n\r\n{\"messages\":[{\"role\":\"user\",\"content\":\"mention \\\"stream_options\\\" as text\"}],\"tenant_id\":\"tenant-a\",\"workspace_id\":\"workspace\",\"session_id\":\"chat-mention-stream-options\"}",
        )
        .unwrap();
        parse_model_service_http_request(
            "POST /v1/completions HTTP/1.1\r\n\r\n{\"model\":\"norion-local\",\"prompt\":\"mention \\\"logprobs\\\" as text\",\"tenant_id\":\"tenant-a\",\"workspace_id\":\"workspace\",\"session_id\":\"completion-mention-logprobs\"}",
        )
        .unwrap();
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
        let raw = "POST /v1/chat/completions HTTP/1.1\r\ncontent-length: 153\r\n\r\n{\"messages\":[{\"role\":\"user\",\"content\":\"你好\"}],\"stream\":true,\"tenant_id\":\"tenant-a\",\"workspace_id\":\"workspace\",\"session_id\":\"chat-stream-openai\"}";

        let request = parse_model_service_http_request(raw).unwrap();

        assert!(matches!(
            request,
            ModelServiceHttpRequest::OpenAiChatCompletionsStream(_)
        ));
    }

    #[test]
    fn parses_generate_stream_route() {
        let raw = "POST /v1/generate-stream HTTP/1.1\r\ncontent-length: 130\r\n\r\n{\"prompt\":\"你好\",\"profile\":\"general\",\"output\":\"raw\",\"tenant_id\":\"tenant-a\",\"workspace_id\":\"workspace\",\"session_id\":\"generate-stream-1\"}";

        let request = parse_model_service_http_request(raw).unwrap();

        assert!(matches!(
            request,
            ModelServiceHttpRequest::GenerateStream(_)
        ));
    }

    #[test]
    fn rejects_hot_inference_routes_without_tenant_scope() {
        for raw in [
            "POST /v1/generate HTTP/1.1\r\n\r\n{\"prompt\":\"hi\"}",
            "POST /v1/generate-stream HTTP/1.1\r\n\r\n{\"prompt\":\"hi\"}",
            "POST /v1/chat HTTP/1.1\r\n\r\n{\"messages\":[{\"role\":\"user\",\"content\":\"hi\"}]}",
            "POST /v1/chat-stream HTTP/1.1\r\n\r\n{\"messages\":[{\"role\":\"user\",\"content\":\"hi\"}]}",
            "POST /v1/chat/completions HTTP/1.1\r\n\r\n{\"messages\":[{\"role\":\"user\",\"content\":\"hi\"}]}",
            "POST /v1/chat/completions HTTP/1.1\r\n\r\n{\"messages\":[{\"role\":\"user\",\"content\":\"hi\"}],\"stream\":true}",
            "POST /v1/completions HTTP/1.1\r\n\r\n{\"model\":\"norion-local\",\"prompt\":\"hi\"}",
            "POST /v1/business-cycle HTTP/1.1\r\n\r\n{\"prompt\":\"hi\"}",
            "POST /v1/business-cycle-stream HTTP/1.1\r\n\r\n{\"prompt\":\"hi\"}",
        ] {
            assert_eq!(
                parse_model_service_http_request(raw).unwrap_err(),
                "tenant scope requires tenant_id, workspace_id, and session_id",
                "{raw}"
            );
        }
    }

    #[test]
    fn rejects_feedback_write_routes_without_tenant_scope() {
        assert_eq!(
            parse_model_service_http_request(
                "POST /v1/feedback HTTP/1.1\r\n\r\n{\"experience_id\":7,\"action\":\"reinforce\"}",
            )
            .unwrap_err(),
            "tenant scope requires tenant_id, workspace_id, and session_id"
        );
        assert_eq!(
            parse_model_service_http_request(
                "POST /v1/rust-check HTTP/1.1\r\n\r\n{\"experience_id\":7,\"code\":\"pub fn ok() {}\"}",
            )
            .unwrap_err(),
            "tenant scope requires tenant_id, workspace_id, and session_id"
        );

        let pure_check = parse_model_service_http_request(
            "POST /v1/rust-check HTTP/1.1\r\n\r\n{\"code\":\"pub fn ok() {}\"}",
        )
        .unwrap();
        let ModelServiceHttpRequest::RustCheck(request) = pure_check else {
            panic!("expected rust-check request");
        };
        assert_eq!(request.tenant_scope, None);
    }

    #[test]
    fn parses_get_inspect_as_contract_info_and_post_inspect_as_execution() {
        let info = parse_model_service_http_request("GET /v1/inspect HTTP/1.1\r\n\r\n").unwrap();
        assert_eq!(info, ModelServiceHttpRequest::Info("inspect"));

        let execution =
            parse_model_service_http_request("POST /v1/inspect HTTP/1.1\r\n\r\n{}").unwrap();
        assert!(matches!(execution, ModelServiceHttpRequest::Inspect(_)));
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
                tenant_scope: None,
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
                tenant_scope: None,
            })
        );

        let scoped_retrieval = parse_model_service_http_request(
            "POST /v1/experience-retrieval HTTP/1.1\r\n\r\n{\"prompt\":\"route code\",\"tenant_id\":\"tenant-a\",\"workspace_id\":\"workspace\",\"session_id\":\"retrieval-1\"}",
        )
        .unwrap();

        assert_eq!(
            scoped_retrieval,
            ModelServiceHttpRequest::ExperienceRetrieval(ModelServiceExperienceRetrievalRequest {
                prompt: "route code".to_owned(),
                profile: None,
                limit: None,
                index_context: None,
                tenant_scope: Some(rust_norion::TenantScope::new(
                    "tenant-a",
                    "workspace",
                    "retrieval-1"
                )),
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

        let raw = "POST /v1/business-cycle-stream HTTP/1.1\r\ncontent-length: 149\r\n\r\n{\"prompt\":\"业务联调\",\"feedback_amount\":0.4,\"self_improve\":false,\"tenant_id\":\"tenant-a\",\"workspace_id\":\"workspace\",\"session_id\":\"business-cycle-stream-1\"}";

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
