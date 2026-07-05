use super::*;
use crate::model_service::request::ModelServiceOutputMode;

#[test]
fn parses_model_service_flags() {
    let args = Args::parse(vec![
        "--serve-bind".to_owned(),
        "127.0.0.1:0".to_owned(),
        "--serve-max-requests".to_owned(),
        "1".to_owned(),
        "--model-pool-manifest".to_owned(),
        "docs\\runbooks\\apple-model-pool.example.json".to_owned(),
        "--max-tokens".to_owned(),
        "8192".to_owned(),
        "--gemma-local-snapshot".to_owned(),
        "D:\\hf-cache\\hub\\models--google--gemma-4-12B-it\\snapshots\\5926caa".to_owned(),
        "--trace".to_owned(),
        "target\\service-trace.jsonl".to_owned(),
        "Rust coding request".to_owned(),
    ]);

    assert!(args.serve);
    assert_eq!(args.serve_bind, "127.0.0.1:0");
    assert_eq!(args.serve_max_requests, Some(1));
    assert_eq!(
        args.model_pool_manifest_path.as_ref().unwrap(),
        &PathBuf::from("docs\\runbooks\\apple-model-pool.example.json")
    );
    assert_eq!(args.max_tokens, Some(8192));
    assert!(args.gemma_12b_runtime);
    assert_eq!(args.gemma_runtime_token_source.as_deref(), Some("none"));
    assert_eq!(
        args.trace_path.as_ref().unwrap(),
        &PathBuf::from("target\\service-trace.jsonl")
    );
}

#[test]
fn model_service_parses_health_and_generate_http_requests() {
    let health = parse_model_service_http_request("GET /health HTTP/1.1\r\n\r\n").unwrap();
    assert_eq!(health, ModelServiceHttpRequest::Health);

    let state = parse_model_service_http_request("GET /v1/state HTTP/1.1\r\n\r\n").unwrap();
    assert_eq!(state, ModelServiceHttpRequest::State);

    let generate_info =
        parse_model_service_http_request("GET /v1/generate HTTP/1.1\r\n\r\n").unwrap();
    assert_eq!(generate_info, ModelServiceHttpRequest::Info("generate"));

    let chat_info = parse_model_service_http_request("GET /v1/chat HTTP/1.1\r\n\r\n").unwrap();
    assert_eq!(chat_info, ModelServiceHttpRequest::Info("chat"));

    let openai_chat_info =
        parse_model_service_http_request("GET /v1/chat/completions HTTP/1.1\r\n\r\n").unwrap();
    assert_eq!(
        openai_chat_info,
        ModelServiceHttpRequest::Info("chat-completions")
    );

    let openai_completion_info =
        parse_model_service_http_request("GET /v1/completions HTTP/1.1\r\n\r\n").unwrap();
    assert_eq!(
        openai_completion_info,
        ModelServiceHttpRequest::Info("completions")
    );

    let body = "{\"prompt\":\"用中文解释 Rust 所有权\",\"profile\":\"coding\",\"case\":\"service-smoke\",\"max_tokens\":2048,\"tenant_id\":\"tenant-a\",\"workspace_id\":\"workspace\",\"session_id\":\"generate-1\"}";
    let request = format!(
        "POST /v1/generate HTTP/1.1\r\ncontent-length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    let parsed = parse_model_service_http_request(&request).unwrap();
    assert_eq!(
        parsed,
        ModelServiceHttpRequest::Generate(ModelServiceRequest {
            prompt: "用中文解释 Rust 所有权".to_owned(),
            profile: Some(TaskProfile::Coding),
            case_name: Some("service-smoke".to_owned()),
            output_mode: ModelServiceOutputMode::Enhanced,
            max_tokens: Some(2048),
            tenant_scope: Some(rust_norion::TenantScope::new(
                "tenant-a",
                "workspace",
                "generate-1"
            )),
        })
    );

    let raw_body = "{\"prompt\":\"请只回答 OK\",\"profile\":\"coding\",\"case\":\"raw-smoke\",\"output\":\"raw\",\"tenant_id\":\"tenant-a\",\"workspace_id\":\"workspace\",\"session_id\":\"generate-raw\"}";
    let raw_request = format!(
        "POST /v1/generate HTTP/1.1\r\ncontent-length: {}\r\n\r\n{}",
        raw_body.len(),
        raw_body
    );
    let raw_parsed = parse_model_service_http_request(&raw_request).unwrap();
    assert_eq!(
        raw_parsed,
        ModelServiceHttpRequest::Generate(ModelServiceRequest {
            prompt: "请只回答 OK".to_owned(),
            profile: Some(TaskProfile::Coding),
            case_name: Some("raw-smoke".to_owned()),
            output_mode: ModelServiceOutputMode::Raw,
            max_tokens: None,
            tenant_scope: Some(rust_norion::TenantScope::new(
                "tenant-a",
                "workspace",
                "generate-raw"
            )),
        })
    );

    let chat_body = concat!(
        "{\"messages\":[",
        "{\"role\":\"system\",\"content\":\"你是 rust-norion 的本地 Gemma 助手。\"},",
        "{\"role\":\"USER\",\"content\":\"继续解释业务联调。\"}",
        "],\"profile\":\"coding\",\"case\":\"chat-smoke\",\"mode\":\"gemma\",\"max_tokens\":3072,",
        "\"tenant_id\":\"tenant-a\",\"workspace_id\":\"workspace\",\"session_id\":\"chat-1\"}"
    );
    let chat_request = format!(
        "POST /v1/chat HTTP/1.1\r\ncontent-length: {}\r\n\r\n{}",
        chat_body.len(),
        chat_body
    );
    let chat = parse_model_service_http_request(&chat_request).unwrap();
    assert_eq!(
        chat,
        ModelServiceHttpRequest::Chat(ModelServiceChatRequest {
            messages: vec![
                ModelServiceChatMessage {
                    role: "system".to_owned(),
                    content: "你是 rust-norion 的本地 Gemma 助手。".to_owned(),
                },
                ModelServiceChatMessage {
                    role: "user".to_owned(),
                    content: "继续解释业务联调。".to_owned(),
                },
            ],
            model: None,
            profile: Some(TaskProfile::Coding),
            case_name: Some("chat-smoke".to_owned()),
            output_mode: ModelServiceOutputMode::Raw,
            max_tokens: Some(3072),
            tenant_scope: Some(rust_norion::TenantScope::new(
                "tenant-a",
                "workspace",
                "chat-1"
            )),
        })
    );

    let openai_chat_body = concat!(
        "{\"model\":\"rust-norion-local\",\"messages\":[",
        "{\"role\":\"user\",\"content\":\"用中文解释 OpenAI 兼容路由。\"}",
        "],\"max_tokens\":64,",
        "\"tenant_id\":\"tenant-a\",\"workspace_id\":\"workspace\",\"session_id\":\"openai-chat-1\"}"
    );
    let openai_chat_request = format!(
        "POST /v1/chat/completions HTTP/1.1\r\ncontent-length: {}\r\n\r\n{}",
        openai_chat_body.len(),
        openai_chat_body
    );
    let openai_chat = parse_model_service_http_request(&openai_chat_request).unwrap();
    assert_eq!(
        openai_chat,
        ModelServiceHttpRequest::OpenAiChatCompletions(ModelServiceChatRequest {
            messages: vec![ModelServiceChatMessage {
                role: "user".to_owned(),
                content: "用中文解释 OpenAI 兼容路由。".to_owned(),
            }],
            model: Some("rust-norion-local".to_owned()),
            profile: None,
            case_name: None,
            output_mode: ModelServiceOutputMode::Enhanced,
            max_tokens: Some(64),
            tenant_scope: Some(rust_norion::TenantScope::new(
                "tenant-a",
                "workspace",
                "openai-chat-1"
            )),
        })
    );

    let openai_completion_body = concat!(
        "{\"model\":\"rust-norion-local\",",
        "\"prompt\":\"用中文解释 OpenAI completion 路由。\",",
        "\"max_tokens\":32,",
        "\"tenant_id\":\"tenant-a\",\"workspace_id\":\"workspace\",\"session_id\":\"openai-completion-1\"}"
    );
    let openai_completion_request = format!(
        "POST /v1/completions HTTP/1.1\r\ncontent-length: {}\r\n\r\n{}",
        openai_completion_body.len(),
        openai_completion_body
    );
    let openai_completion = parse_model_service_http_request(&openai_completion_request).unwrap();
    let ModelServiceHttpRequest::OpenAiCompletions(openai_completion) = openai_completion else {
        panic!("expected OpenAI completions request");
    };
    assert_eq!(
        openai_completion.model.as_deref(),
        Some("rust-norion-local")
    );
    assert_eq!(
        openai_completion.generate,
        ModelServiceRequest {
            prompt: "用中文解释 OpenAI completion 路由。".to_owned(),
            profile: None,
            case_name: None,
            output_mode: ModelServiceOutputMode::Enhanced,
            max_tokens: Some(32),
            tenant_scope: Some(rust_norion::TenantScope::new(
                "tenant-a",
                "workspace",
                "openai-completion-1"
            )),
        }
    );

    let openai_completion_stream = parse_model_service_http_request(
        "POST /v1/completions HTTP/1.1\r\n\r\n{\"prompt\":\"hi\",\"stream\":true}",
    )
    .unwrap_err();
    assert!(openai_completion_stream.contains("stream=true is not supported"));

    let openai_stream = parse_model_service_http_request(
        "POST /v1/chat/completions HTTP/1.1\r\n\r\n{\"messages\":[{\"role\":\"user\",\"content\":\"hi\"}],\"stream\":true,\"tenant_id\":\"tenant-a\",\"workspace_id\":\"workspace\",\"session_id\":\"openai-stream-1\"}",
    )
    .unwrap();
    assert!(matches!(
        openai_stream,
        ModelServiceHttpRequest::OpenAiChatCompletionsStream(_)
    ));

    let chat_prompt = match chat {
        ModelServiceHttpRequest::Chat(request) => request.into_generate_request().prompt,
        _ => unreachable!("chat request should parse as chat"),
    };
    assert_eq!(
        chat_prompt,
        "Conversation transcript:\nsystem: 你是 rust-norion 的本地 Gemma 助手。\nuser: 继续解释业务联调。\nassistant:"
    );

    let replay =
        parse_model_service_http_request("POST /v1/replay HTTP/1.1\r\n\r\n{\"limit\":2}").unwrap();
    assert_eq!(
        replay,
        ModelServiceHttpRequest::Replay(ModelServiceReplayRequest { limit: 2 })
    );

    let self_improve = parse_model_service_http_request(
            "POST /v1/self-improve HTTP/1.1\r\n\r\n{\"limit\":3,\"gate\":\"gemma_model_service_smoke\",\"trace_gate\":false}",
        )
        .unwrap();
    assert_eq!(
        self_improve,
        ModelServiceHttpRequest::SelfImprove(ModelServiceSelfImproveRequest {
            limit: 3,
            inspect: ModelServiceInspectRequest {
                state_gate: true,
                business_gate: false,
                business_cycle_gate: false,
                model_service_gate: true,
                trace_gate: Some(false),
            },
        })
    );

    let cycle_self_improve = parse_model_service_http_request(
        "POST /v1/self-improve HTTP/1.1\r\n\r\n{\"limit\":2,\"gate\":\"business_cycle\"}",
    )
    .unwrap();
    assert_eq!(
        cycle_self_improve,
        ModelServiceHttpRequest::SelfImprove(ModelServiceSelfImproveRequest {
            limit: 2,
            inspect: ModelServiceInspectRequest {
                state_gate: true,
                business_gate: false,
                business_cycle_gate: true,
                model_service_gate: false,
                trace_gate: None,
            },
        })
    );

    let business_cycle = parse_model_service_http_request(
            "POST /v1/business-cycle HTTP/1.1\r\n\r\n{\"prompt\":\"业务联调\",\"profile\":\"coding\",\"case\":\"cycle-case\",\"max_tokens\":4096,\"feedback_amount\":0.4,\"rust_check_code\":\"pub fn ok() {}\",\"self_improve_limit\":2,\"gate\":\"business_cycle\",\"trace_gate\":false,\"tenant_id\":\"tenant-a\",\"workspace_id\":\"workspace\",\"session_id\":\"business-cycle-1\"}",
        )
        .unwrap();
    assert_eq!(
        business_cycle,
        ModelServiceHttpRequest::BusinessCycle(ModelServiceBusinessCycleRequest {
            prompt: "业务联调".to_owned(),
            profile: Some(TaskProfile::Coding),
            case_name: Some("cycle-case".to_owned()),
            max_tokens: Some(4096),
            feedback_action: RewardAction::Reinforce,
            feedback_amount: 0.4,
            rust_check_code: Some("pub fn ok() {}".to_owned()),
            rust_check_edition: "2021".to_owned(),
            rust_check_case_name: None,
            self_improve: true,
            self_improve_limit: 2,
            pool_dispatch: None,
            pool_stage_dispatch: Vec::new(),
            inspect: ModelServiceInspectRequest {
                state_gate: true,
                business_gate: false,
                business_cycle_gate: true,
                model_service_gate: false,
                trace_gate: Some(false),
            },
            tenant_scope: Some(rust_norion::TenantScope::new(
                "tenant-a",
                "workspace",
                "business-cycle-1"
            )),
        })
    );

    let feedback = parse_model_service_http_request(
            "POST /v1/feedback HTTP/1.1\r\n\r\n{\"experience_id\":7,\"action\":\"penalize\",\"amount\":0.25,\"tenant_id\":\"tenant-a\",\"workspace_id\":\"workspace\",\"session_id\":\"feedback-1\"}",
        )
        .unwrap();
    assert_eq!(
        feedback,
        ModelServiceHttpRequest::Feedback(ModelServiceFeedbackRequest {
            action: RewardAction::Penalize,
            amount: 0.25,
            experience_id: Some(7),
            memory_id: None,
            tenant_scope: Some(rust_norion::TenantScope::new(
                "tenant-a",
                "workspace",
                "feedback-1"
            )),
        })
    );

    let rust_check = parse_model_service_http_request(
            "POST /v1/rust-check HTTP/1.1\r\n\r\n{\"experience_id\":7,\"code\":\"pub fn ok() -> u32 { 1 }\",\"edition\":\"2021\",\"amount\":0.4,\"case\":\"compiler-feedback\",\"tenant_id\":\"tenant-a\",\"workspace_id\":\"workspace\",\"session_id\":\"rust-check-1\"}",
        )
        .unwrap();
    assert_eq!(
        rust_check,
        ModelServiceHttpRequest::RustCheck(ModelServiceRustCheckRequest {
            code: "pub fn ok() -> u32 { 1 }".to_owned(),
            edition: "2021".to_owned(),
            case_name: Some("compiler-feedback".to_owned()),
            amount: Some(0.4),
            experience_id: Some(7),
            memory_id: None,
            tenant_scope: Some(rust_norion::TenantScope::new(
                "tenant-a",
                "workspace",
                "rust-check-1"
            )),
        })
    );

    let inspect = parse_model_service_http_request(
        "POST /v1/inspect HTTP/1.1\r\n\r\n{\"gate\":\"gemma_business_smoke\",\"trace_gate\":true}",
    )
    .unwrap();
    assert_eq!(
        inspect,
        ModelServiceHttpRequest::Inspect(ModelServiceInspectRequest {
            state_gate: true,
            business_gate: true,
            business_cycle_gate: false,
            model_service_gate: false,
            trace_gate: Some(true),
        })
    );

    let gemma_cycle_inspect = parse_model_service_http_request(
        "POST /v1/inspect HTTP/1.1\r\n\r\n{\"gate\":\"gemma_business_cycle\",\"trace_gate\":true}",
    )
    .unwrap();
    assert_eq!(
        gemma_cycle_inspect,
        ModelServiceHttpRequest::Inspect(ModelServiceInspectRequest {
            state_gate: true,
            business_gate: true,
            business_cycle_gate: true,
            model_service_gate: false,
            trace_gate: Some(true),
        })
    );

    let service_inspect = parse_model_service_http_request(
            "POST /v1/inspect HTTP/1.1\r\n\r\n{\"gate\":\"gemma_model_service_smoke\",\"trace_gate\":true}",
        )
        .unwrap();
    assert_eq!(
        service_inspect,
        ModelServiceHttpRequest::Inspect(ModelServiceInspectRequest {
            state_gate: true,
            business_gate: false,
            business_cycle_gate: false,
            model_service_gate: true,
            trace_gate: Some(true),
        })
    );
}

#[test]
fn model_service_json_helpers_escape_control_text() {
    assert_eq!(
        service_json_string("quote \" slash \\ line\n"),
        "\"quote \\\" slash \\\\ line\\n\""
    );
    assert_eq!(
        service_json_string_array(&["one".to_owned(), "two\n".to_owned()]),
        "[\"one\",\"two\\n\"]"
    );
    assert_eq!(json_u64_field("{\"id\":\"42\"}", "id"), Some(42));
    assert_eq!(
        json_u64_array_field("{\"ids\":[42,7]}", "ids"),
        Some(vec![42, 7])
    );
    assert_eq!(json_f32_field("{\"amount\":0.25}", "amount"), Some(0.25));
    let bad = parse_model_service_http_request(
        "POST /v1/generate HTTP/1.1\r\ncontent-length: 2\r\n\r\n{}",
    )
    .unwrap_err();
    assert!(bad.contains("prompt"));
}
