use super::*;
use rust_norion::RuntimeError;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

#[derive(Clone)]
struct BlockingBackend {
    started: Arc<AtomicBool>,
    release: Arc<AtomicBool>,
}

impl InferenceBackend for BlockingBackend {
    fn generate(&mut self, _context: GenerationContext<'_>) -> InferenceDraft {
        self.started.store(true, Ordering::SeqCst);
        for _ in 0..200 {
            if self.release.load(Ordering::SeqCst) {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        InferenceDraft::new(
            "blocking backend released",
            vec![ReasoningStep::new("blocking_backend", "released", 0.8)],
        )
    }

    fn generate_stream_checked(
        &mut self,
        _context: GenerationContext<'_>,
        on_token: &mut dyn FnMut(&DraftToken) -> Result<(), RuntimeError>,
    ) -> InferenceDraft {
        let draft = InferenceDraft::new(
            "blocking backend stream released",
            vec![ReasoningStep::new(
                "blocking_backend",
                "stream released",
                0.8,
            )],
        )
        .with_tokens(vec![DraftToken::new("partial "), DraftToken::new("done")]);
        self.started.store(true, Ordering::SeqCst);
        if on_token(&draft.tokens[0]).is_err() {
            return draft;
        }
        for _ in 0..200 {
            if self.release.load(Ordering::SeqCst) {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        let _ = on_token(&draft.tokens[1]);
        draft
    }
}

#[derive(Clone)]
struct RuntimeErrorBackend;

impl InferenceBackend for RuntimeErrorBackend {
    fn generate(&mut self, _context: GenerationContext<'_>) -> InferenceDraft {
        InferenceDraft::new(
            "Runtime backend error: runtime command mistralrs timed out after 1000 ms",
            vec![ReasoningStep::new(
                "runtime_error",
                "runtime command mistralrs timed out after 1000 ms",
                0.0,
            )],
        )
    }
}

#[derive(Debug, PartialEq)]
struct ObservedGeneration {
    prompt: String,
    profile: TaskProfile,
    max_tokens: Option<usize>,
    tenant_scope: Option<rust_norion::TenantScope>,
}

#[derive(Clone)]
struct RecordingBackend {
    calls: Arc<Mutex<Vec<ObservedGeneration>>>,
    max_tokens: Option<usize>,
}

impl RecordingBackend {
    fn new(calls: Arc<Mutex<Vec<ObservedGeneration>>>) -> Self {
        Self {
            calls,
            max_tokens: None,
        }
    }

    fn record_context(&self, context: &GenerationContext<'_>) {
        self.calls.lock().unwrap().push(ObservedGeneration {
            prompt: context.prompt.to_owned(),
            profile: context.profile,
            max_tokens: self.max_tokens,
            tenant_scope: context.tenant_scope.cloned(),
        });
    }
}

fn assert_json_string_fields(body: &str, fields: &[&str]) {
    for field in fields {
        assert!(body.contains(&format!("\"{field}\"")), "{body}");
    }
}

impl InferenceBackend for RecordingBackend {
    fn configure_generation(&mut self, max_tokens: Option<usize>) {
        self.max_tokens = max_tokens;
    }

    fn generate(&mut self, context: GenerationContext<'_>) -> InferenceDraft {
        self.record_context(&context);
        InferenceDraft::new(
            format!("recorded service prompt: {}", context.prompt),
            vec![ReasoningStep::new("recording_backend", "recorded", 0.9)],
        )
    }

    fn generate_stream_checked(
        &mut self,
        context: GenerationContext<'_>,
        on_token: &mut dyn FnMut(&DraftToken) -> Result<(), RuntimeError>,
    ) -> InferenceDraft {
        self.record_context(&context);
        let draft = InferenceDraft::new(
            format!("recorded service stream prompt: {}", context.prompt),
            vec![ReasoningStep::new(
                "recording_backend",
                "stream recorded",
                0.9,
            )],
        )
        .with_tokens(vec![DraftToken::new("partial "), DraftToken::new("done")]);
        for index in 0..draft.tokens.len() {
            if on_token(&draft.tokens[index]).is_err() {
                return draft;
            }
        }
        draft
    }
}

#[derive(Clone)]
struct ShortRawBackend;

impl InferenceBackend for ShortRawBackend {
    fn generate(&mut self, _context: GenerationContext<'_>) -> InferenceDraft {
        InferenceDraft::new(
            "Rust routes.",
            vec![ReasoningStep::new("draft", "short but grounded", 0.86)],
        )
    }
}

fn write_dirty_experience_store(path: &Path) {
    let mut store = rust_norion::ExperienceStore::new();
    store.record(experience_input(
        "Conversation transcript:\nuser: audit GitLab merge_requests over ssh\nassistant: ok",
        "ssh -o ConnectTimeout=8 gitlab.local merge_requests bash command",
        0.94,
    ));
    store.record(experience_input(
        "用中文解释 Rust 所有权",
        "回答要保持中文、聚焦所有权、给出短示例。",
        0.82,
    ));
    store.save_to_disk_kv(path).unwrap();
}

fn experience_input(prompt: &str, lesson: &str, quality: f32) -> rust_norion::ExperienceInput {
    rust_norion::ExperienceInput {
        prompt: prompt.to_owned(),
        profile: rust_norion::TaskProfile::Coding,
        lesson: lesson.to_owned(),
        quality,
        contradictions: Vec::new(),
        reflection_issues: Vec::new(),
        revision_actions: Vec::new(),
        stored_memory_id: None,
        router_threshold_after: 0.5,
        stream_windows: 1,
        route_budget: rust_norion::RouteBudget {
            threshold: 0.5,
            attention_tokens: 1,
            fast_tokens: 1,
            attention_fraction: 0.5,
        },
        hierarchy: rust_norion::HierarchyWeights::new(0.33, 0.34, 0.33),
        used_memory_ids: Vec::new(),
        gist_records: Vec::new(),
        gist_memory_ids: Vec::new(),
        stored_runtime_kv_memory_ids: Vec::new(),
        runtime_diagnostics: rust_norion::RuntimeDiagnostics::default(),
        runtime_token_metrics: rust_norion::ExperienceRuntimeTokenMetrics::default(),
        process_reward: rust_norion::ProcessRewardReport::default(),
        live_evolution: rust_norion::LiveInferenceEvolution::default(),
    }
}

#[test]
fn model_service_openai_models_reports_capabilities() {
    let asset_dir = target_asset_dir("model-service-openai-models");
    fs::create_dir_all(&asset_dir).unwrap();
    let bind = reserve_loopback_addr();
    let args = Args::parse(vec![
        "--serve-bind".to_owned(),
        bind.clone(),
        "--serve-max-requests".to_owned(),
        "25".to_owned(),
        "--memory".to_owned(),
        asset_dir.join("memory.ndkv").display().to_string(),
        "--experience".to_owned(),
        asset_dir.join("experience.ndkv").display().to_string(),
        "--adaptive".to_owned(),
        asset_dir.join("adaptive.ndkv").display().to_string(),
        "service openai models prompt".to_owned(),
    ]);
    let service_args = args.clone();
    let handle = thread::spawn(move || {
        let mut engine = NoironEngine::new();
        configure_engine(&mut engine, &service_args);
        let mut backend = HeuristicBackend;
        run_model_service_for_args(&mut engine, &mut backend, &service_args)
    });

    let health = wait_for_http_response(&bind, "GET", "/health", None);
    let models = service_http_request(&bind, "GET", "/v1/models", None);
    let business_cycle_contract = service_http_request(&bind, "GET", "/v1/business-cycle", None);
    let business_cycle_stream_contract =
        service_http_request(&bind, "GET", "/v1/business-cycle-stream", None);
    let generate_stream_contract = service_http_request(&bind, "GET", "/v1/generate-stream", None);
    let experience_retrieval_contract =
        service_http_request(&bind, "GET", "/v1/experience-retrieval", None);
    let experience_hygiene_quarantine_contract =
        service_http_request(&bind, "GET", "/v1/experience-hygiene/quarantine", None);
    let experience_cleanup_audit_contract =
        service_http_request(&bind, "GET", "/v1/experience-cleanup-audit", None);
    let experience_repair_contract =
        service_http_request(&bind, "GET", "/v1/experience-repair", None);
    let chat_contract = service_http_request(&bind, "GET", "/v1/chat/completions", None);
    let completion_contract = service_http_request(&bind, "GET", "/v1/completions", None);
    let route_contract = service_http_request(&bind, "GET", "/v1/model-pool/route-plan", None);
    let call_contract = service_http_request(&bind, "GET", "/v1/model-pool/call", None);
    let cancel_contract = service_http_request(&bind, "GET", "/v1/requests/cancel", None);
    let feedback_contract = service_http_request(&bind, "GET", "/v1/feedback", None);
    let rust_check_contract = service_http_request(&bind, "GET", "/v1/rust-check", None);
    let replay_contract = service_http_request(&bind, "GET", "/v1/replay", None);
    let self_improve_contract = service_http_request(&bind, "GET", "/v1/self-improve", None);
    let unsupported_completion_stream = service_http_request(
        &bind,
        "POST",
        "/v1/completions",
        Some("{\"model\":\"rust-norion-local\",\"prompt\":\"stream me\",\"stream\":true}"),
    );
    let unsupported_chat_tools = service_http_request(
        &bind,
        "POST",
        "/v1/chat/completions",
        Some(
            "{\"model\":\"rust-norion-local\",\"messages\":[{\"role\":\"user\",\"content\":\"call a tool\"}],\"tools\":[]}",
        ),
    );
    let unsupported_chat_n = service_http_request(
        &bind,
        "POST",
        "/v1/chat/completions",
        Some(
            "{\"model\":\"rust-norion-local\",\"messages\":[{\"role\":\"user\",\"content\":\"give options\"}],\"n\":2}",
        ),
    );
    let unsupported_completion_n = service_http_request(
        &bind,
        "POST",
        "/v1/completions",
        Some("{\"model\":\"rust-norion-local\",\"prompt\":\"give options\",\"n\":2}"),
    );
    let unsupported_chat_temperature = service_http_request(
        &bind,
        "POST",
        "/v1/chat/completions",
        Some(
            "{\"model\":\"rust-norion-local\",\"messages\":[{\"role\":\"user\",\"content\":\"sample this\"}],\"temperature\":0.7}",
        ),
    );
    let unsupported_completion_stop = service_http_request(
        &bind,
        "POST",
        "/v1/completions",
        Some("{\"model\":\"rust-norion-local\",\"prompt\":\"stop me\",\"stop\":\"END\"}"),
    );
    let diagnostics = service_http_request(&bind, "GET", "/v1/diagnostics", None);
    handle.join().unwrap().unwrap();

    let health_body = http_body(&health);
    let models_body = http_body(&models);
    let business_cycle_contract_body = http_body(&business_cycle_contract);
    let business_cycle_stream_contract_body = http_body(&business_cycle_stream_contract);
    let generate_stream_contract_body = http_body(&generate_stream_contract);
    let experience_retrieval_contract_body = http_body(&experience_retrieval_contract);
    let experience_hygiene_quarantine_contract_body =
        http_body(&experience_hygiene_quarantine_contract);
    let experience_cleanup_audit_contract_body = http_body(&experience_cleanup_audit_contract);
    let experience_repair_contract_body = http_body(&experience_repair_contract);
    let chat_contract_body = http_body(&chat_contract);
    let completion_contract_body = http_body(&completion_contract);
    let route_contract_body = http_body(&route_contract);
    let call_contract_body = http_body(&call_contract);
    let cancel_contract_body = http_body(&cancel_contract);
    let feedback_contract_body = http_body(&feedback_contract);
    let rust_check_contract_body = http_body(&rust_check_contract);
    let replay_contract_body = http_body(&replay_contract);
    let self_improve_contract_body = http_body(&self_improve_contract);
    let unsupported_completion_stream_body = http_body(&unsupported_completion_stream);
    let unsupported_chat_tools_body = http_body(&unsupported_chat_tools);
    let unsupported_chat_n_body = http_body(&unsupported_chat_n);
    let unsupported_completion_n_body = http_body(&unsupported_completion_n);
    let unsupported_chat_temperature_body = http_body(&unsupported_chat_temperature);
    let unsupported_completion_stop_body = http_body(&unsupported_completion_stop);
    let diagnostics_body = http_body(&diagnostics);
    assert!(health_body.contains("\"ok\":true"), "{health_body}");
    assert!(models.contains("HTTP/1.1 200 OK"), "{models}");
    assert!(models_body.contains("\"object\":\"list\""), "{models_body}");
    assert!(
        models_body.contains("\"id\":\"rust-norion-local\""),
        "{models_body}"
    );
    assert!(models_body.contains("\"/v1/models\""), "{models_body}");
    assert!(
        models_body.contains("\"/v1/chat/completions\""),
        "{models_body}"
    );
    assert!(models_body.contains("\"streaming\":true"), "{models_body}");
    assert!(
        models_body.contains("\"cancellation\":true"),
        "{models_body}"
    );
    assert!(models_body.contains("\"max_tokens\":true"), "{models_body}");
    assert!(
        models_body.contains(
            "\"supported_request_fields\":[\"model\",\"messages\",\"prompt\",\"stream\",\"max_tokens\",\"max_completion_tokens\",\"n\""
        ),
        "{models_body}"
    );
    assert!(
        models_body.contains("\"/v1/model-pool/route-plan\""),
        "{models_body}"
    );
    assert!(
        models_body.contains("\"/v1/model-pool/call\""),
        "{models_body}"
    );
    assert!(
        models_body.contains("\"/v1/business-cycle\""),
        "{models_body}"
    );
    assert!(
        models_body.contains("\"/v1/business-cycle-stream\""),
        "{models_body}"
    );
    assert!(
        models_body.contains("\"/v1/experience-retrieval\""),
        "{models_body}"
    );
    assert!(
        models_body.contains("\"/v1/experience-hygiene/quarantine\""),
        "{models_body}"
    );
    assert!(
        models_body.contains("\"/v1/experience-cleanup-audit\""),
        "{models_body}"
    );
    assert!(
        models_body.contains("\"/v1/experience-repair\""),
        "{models_body}"
    );
    assert!(models_body.contains("\"/v1/state\""), "{models_body}");
    assert!(models_body.contains("\"/v1/inspect\""), "{models_body}");
    assert!(models_body.contains("\"/v1/feedback\""), "{models_body}");
    assert!(models_body.contains("\"/v1/rust-check\""), "{models_body}");
    assert!(models_body.contains("\"/v1/replay\""), "{models_body}");
    assert!(
        models_body.contains("\"/v1/self-improve\""),
        "{models_body}"
    );
    assert!(
        models_body.contains("\"experience_retrieval\":true"),
        "{models_body}"
    );
    assert!(
        models_body.contains("\"experience_hygiene_quarantine\":true"),
        "{models_body}"
    );
    assert!(
        models_body.contains("\"experience_cleanup_audit\":true"),
        "{models_body}"
    );
    assert!(
        models_body.contains("\"experience_repair\":true"),
        "{models_body}"
    );
    assert!(
        models_body.contains("\"state_inspection\":true"),
        "{models_body}"
    );
    assert!(models_body.contains("\"feedback\":true"), "{models_body}");
    assert!(models_body.contains("\"rust_check\":true"), "{models_body}");
    assert!(
        models_body.contains("\"experience_replay\":true"),
        "{models_body}"
    );
    assert!(
        models_body.contains("\"hierarchical_routing\":true"),
        "{models_body}"
    );
    assert!(
        models_body.contains(
            "\"unsupported_features\":[\"tools\",\"tool_choice\",\"response_format\",\"logprobs\",\"multiple_choices\",\"sampling_controls\",\"stop_sequences\"]"
        ),
        "{models_body}"
    );
    assert!(
        models_body.contains("\"diagnostics_endpoint\":\"/v1/diagnostics\""),
        "{models_body}"
    );
    assert!(
        models_body.contains("\"health_response_fields\":["),
        "{models_body}"
    );
    assert!(
        models_body.contains("\"diagnostics_response_fields\":["),
        "{models_body}"
    );
    assert_json_string_fields(
        models_body,
        &[
            "active_requests.request_id",
            "active_requests.cancel_reason",
            "experience_hygiene.checked",
            "experience_hygiene.repair.repairable_legacy_metadata_lessons",
            "experience_hygiene.index.quality_score",
            "last_inference.runtime_model",
            "last_inference.runtime_token_count",
        ],
    );
    assert!(
        models_body.contains("\"weight_retraining_required\":false"),
        "{models_body}"
    );
    assert!(
        business_cycle_contract.contains("HTTP/1.1 200 OK"),
        "{business_cycle_contract}"
    );
    assert!(
        business_cycle_contract_body.contains("\"endpoint\":\"/v1/business-cycle\""),
        "{business_cycle_contract_body}"
    );
    assert!(
        business_cycle_contract_body.contains("\"supported_fields\":[\"prompt\",\"profile\",\"case\",\"max_tokens\",\"max\",\"feedback_action\",\"action\",\"feedback_amount\",\"amount\",\"rust_check_code\",\"code\",\"rust_check_edition\",\"edition\",\"rust_check_case\",\"rust_case\",\"self_improve\",\"self_improve_limit\",\"limit\",\"pool_dispatch\",\"pool_stage_dispatch\",\"gate\",\"trace_gate\",\"tenant_id\",\"workspace_id\",\"session_id\"]"),
        "{business_cycle_contract_body}"
    );
    assert!(
        business_cycle_contract_body.contains("\"response_fields\":[\"ok\",\"request_id\",\"pool_dispatch\",\"pool_stage_dispatch\",\"business_cycle\",\"generate\",\"feedback\",\"rust_check\",\"self_improve\",\"replay\",\"state\",\"state_gate\",\"trace_gate\",\"eval\",\"error\"]"),
        "{business_cycle_contract_body}"
    );
    assert!(
        business_cycle_stream_contract.contains("HTTP/1.1 200 OK"),
        "{business_cycle_stream_contract}"
    );
    assert!(
        business_cycle_stream_contract_body.contains("\"endpoint\":\"/v1/business-cycle-stream\""),
        "{business_cycle_stream_contract_body}"
    );
    assert!(
        business_cycle_stream_contract_body.contains("\"response_fields\":[\"event:status\",\"event:stage\",\"event:delta\",\"event:meta\",\"event:final\",\"event:done\",\"event:error\"]"),
        "{business_cycle_stream_contract_body}"
    );
    assert!(
        generate_stream_contract_body.contains("\"endpoint\":\"/v1/generate-stream\""),
        "{generate_stream_contract_body}"
    );
    assert!(
        generate_stream_contract_body.contains("\"event:final.task_mode\""),
        "{generate_stream_contract_body}"
    );
    assert!(
        generate_stream_contract_body.contains("\"event:final.retryable\""),
        "{generate_stream_contract_body}"
    );
    assert!(
        generate_stream_contract_body.contains("\"event:final.runtime_error_note\""),
        "{generate_stream_contract_body}"
    );
    assert!(
        generate_stream_contract_body.contains("\"event:final.compute_budget_summary\""),
        "{generate_stream_contract_body}"
    );
    for field in [
        "\"event:final.compute_budget_saved_tokens\"",
        "\"event:final.compute_budget_avoided_tokens\"",
        "\"event:final.compute_budget_kv_lookups_skipped\"",
        "\"event:final.compute_budget_fanout_reduction\"",
        "\"event:final.compute_budget_read_only\"",
        "\"event:final.compute_budget_write_allowed\"",
        "\"event:final.compute_budget_applied\"",
    ] {
        assert!(
            generate_stream_contract_body.contains(field),
            "{generate_stream_contract_body}"
        );
    }
    for field in [
        "\"event:final.elapsed_ms\"",
        "\"event:final.output_mode\"",
        "\"event:final.quality\"",
        "\"event:final.process_reward\"",
        "\"event:final.action\"",
        "\"event:final.memory_stored\"",
        "\"event:final.stored_memory_id\"",
        "\"event:final.used_memory_ids\"",
        "\"event:final.stored_gist_memory_ids\"",
        "\"event:final.stored_runtime_kv_memory_ids\"",
        "\"event:final.feedback_memory_ids\"",
        "\"event:final.experience_id\"",
        "\"event:final.runtime_model\"",
        "\"event:final.runtime_entropy_count\"",
        "\"event:final.runtime_logprob_count\"",
        "\"event:final.runtime_uncertainty_token_count\"",
        "\"event:final.runtime_average_entropy\"",
        "\"event:final.runtime_average_neg_logprob\"",
        "\"event:final.runtime_uncertainty_perplexity\"",
        "\"event:final.runtime_architecture_signal\"",
        "\"event:final.runtime_kv_precision_signal\"",
        "\"event:final.runtime_device_execution_source\"",
        "\"event:final.traceable\"",
    ] {
        assert!(
            generate_stream_contract_body.contains(field),
            "{generate_stream_contract_body}"
        );
    }
    assert!(
        generate_stream_contract_body.contains("\"event:final.memory_write_allowed\""),
        "{generate_stream_contract_body}"
    );
    assert!(
        experience_retrieval_contract.contains("HTTP/1.1 200 OK"),
        "{experience_retrieval_contract}"
    );
    assert!(
        experience_retrieval_contract_body.contains("\"endpoint\":\"/v1/experience-retrieval\""),
        "{experience_retrieval_contract_body}"
    );
    assert!(
        experience_retrieval_contract_body
            .contains("\"supported_fields\":[\"prompt\",\"profile\",\"limit\",\"index_context\",\"tenant_id\",\"workspace_id\",\"session_id\"]"),
        "{experience_retrieval_contract_body}"
    );
    assert!(
        experience_retrieval_contract_body.contains("\"response_fields\":[\"ok\",\"request_id\",\"retrieval\",\"prompt\",\"profile\",\"index_context_used\",\"index_context_chars\",\"total_records\",\"requested_limit\",\"matches\",\"match_count\",\"skipped_cross_task_pollution\",\"retrieval_noise_penalized_candidates\",\"retrieval_noise_filtered_candidates\",\"suppressed_prompt_index_candidates\",\"max_retrieval_noise_penalty\",\"max_score\",\"experience_id\",\"score\",\"quality\",\"process_reward\",\"reward_action\",\"prompt_preview\",\"lesson_preview\",\"usable_hint_preview\",\"gist_hints\",\"reflection_issue_codes\",\"revision_actions\",\"runtime_model\",\"runtime_adapter\",\"runtime_device\",\"runtime_primary_lane\",\"runtime_fallback_lane\",\"runtime_memory_mode\",\"runtime_device_execution_source\",\"runtime_forward_energy\",\"runtime_kv_influence\",\"runtime_uncertainty_perplexity\",\"recursive_runtime_calls\"]"),
        "{experience_retrieval_contract_body}"
    );
    assert!(
        experience_retrieval_contract_body.contains("\"unsupported_fields\":[\"model\",\"messages\",\"stream\",\"max_tokens\",\"tools\",\"tool_choice\",\"response_format\",\"logprobs\"]"),
        "{experience_retrieval_contract_body}"
    );
    assert!(
        experience_hygiene_quarantine_contract.contains("HTTP/1.1 200 OK"),
        "{experience_hygiene_quarantine_contract}"
    );
    assert!(
        experience_hygiene_quarantine_contract_body
            .contains("\"endpoint\":\"/v1/experience-hygiene/quarantine\""),
        "{experience_hygiene_quarantine_contract_body}"
    );
    assert!(
        experience_hygiene_quarantine_contract_body.contains(
            "\"supported_fields\":[\"apply\",\"limit\",\"backup_path\",\"quarantine_path\"]"
        ),
        "{experience_hygiene_quarantine_contract_body}"
    );
    assert!(
        experience_hygiene_quarantine_contract_body.contains("\"response_fields\":[\"ok\",\"request_id\",\"experience_file\",\"applied\",\"backup_file\",\"quarantine_file\",\"plan\",\"total_records\",\"retained_records\",\"quarantine_candidates\",\"candidate_ids\",\"listed_findings\",\"experience_id\",\"severity\",\"reason\",\"markers\",\"prompt_preview\",\"lesson_preview\"]"),
        "{experience_hygiene_quarantine_contract_body}"
    );
    assert!(
        experience_hygiene_quarantine_contract_body.contains("\"unsupported_fields\":[\"prompt\",\"profile\",\"model\",\"messages\",\"stream\",\"max_tokens\",\"tools\",\"tool_choice\",\"response_format\",\"logprobs\",\"tenant_id\",\"workspace_id\",\"session_id\"]"),
        "{experience_hygiene_quarantine_contract_body}"
    );
    assert!(
        experience_cleanup_audit_contract.contains("HTTP/1.1 200 OK"),
        "{experience_cleanup_audit_contract}"
    );
    assert!(
        experience_cleanup_audit_contract_body
            .contains("\"endpoint\":\"/v1/experience-cleanup-audit\""),
        "{experience_cleanup_audit_contract_body}"
    );
    assert!(
        experience_cleanup_audit_contract_body.contains("\"supported_fields\":[\"limit\"]"),
        "{experience_cleanup_audit_contract_body}"
    );
    assert!(
        experience_cleanup_audit_contract_body.contains("\"response_fields\":[\"ok\",\"request_id\",\"experience_file\",\"checked\",\"writes_experience_state\",\"sample_limit\",\"error\",\"report\",\"index_report\",\"quarantine_plan\",\"repair_plan\",\"next_step\",\"total_records\",\"findings\",\"watch\",\"quarantine_candidates\",\"legacy_metadata_lessons\",\"legacy_metadata_without_clean_gist\",\"clean\",\"listed_findings\",\"compacted_records\",\"overlong_records\",\"overlong_without_clean_gist\",\"max_record_chars\",\"noisy_records\",\"duplicate_outputs\",\"max_noise_penalty\",\"quality_score\",\"retrieval_ready\",\"risk_level\",\"recommended_action\",\"retained_records\",\"candidate_ids\",\"experience_id\",\"severity\",\"reason\",\"markers\",\"prompt_preview\",\"lesson_preview\",\"repairable_legacy_metadata_lessons\",\"repairable_index_records\",\"remaining_legacy_metadata_lessons_after_repair\",\"remaining_watch_after_repair\",\"remaining_quarantine_candidates_after_repair\",\"skipped_quarantine_candidates\",\"skipped_missing_clean_gist\",\"projected_hygiene_after_repair\",\"listed_repairs\",\"listed_skipped_quarantine_candidates\",\"listed_skipped_missing_clean_gist\"]"),
        "{experience_cleanup_audit_contract_body}"
    );
    assert!(
        experience_cleanup_audit_contract_body.contains("\"unsupported_fields\":[\"apply\",\"backup_path\",\"quarantine_path\",\"prompt\",\"profile\",\"model\",\"messages\",\"stream\",\"max_tokens\",\"tools\",\"tool_choice\",\"response_format\",\"logprobs\",\"tenant_id\",\"workspace_id\",\"session_id\"]"),
        "{experience_cleanup_audit_contract_body}"
    );
    assert!(
        experience_repair_contract.contains("HTTP/1.1 200 OK"),
        "{experience_repair_contract}"
    );
    assert!(
        experience_repair_contract_body.contains("\"endpoint\":\"/v1/experience-repair\""),
        "{experience_repair_contract_body}"
    );
    assert!(
        experience_repair_contract_body
            .contains("\"supported_fields\":[\"apply\",\"limit\",\"backup_path\"]"),
        "{experience_repair_contract_body}"
    );
    assert!(
        experience_repair_contract_body.contains("\"response_fields\":[\"ok\",\"request_id\",\"experience_file\",\"applied\",\"backup_file\",\"plan\",\"total_records\",\"legacy_metadata_lessons\",\"repairable_legacy_metadata_lessons\",\"index_noisy_records\",\"index_duplicate_outputs\",\"repairable_index_records\",\"remaining_legacy_metadata_lessons_after_repair\",\"remaining_watch_after_repair\",\"remaining_quarantine_candidates_after_repair\",\"skipped_quarantine_candidates\",\"skipped_missing_clean_gist\",\"projected_hygiene_after_repair\",\"legacy_metadata_without_clean_gist\",\"index_quality_score\",\"index_retrieval_ready\",\"index_risk_level\",\"listed_repairs\",\"listed_skipped_quarantine_candidates\",\"listed_skipped_missing_clean_gist\",\"experience_id\",\"action\",\"source\",\"old_lesson_preview\",\"proposed_lesson_preview\",\"source_gist_preview\",\"reason\",\"prompt_preview\",\"gist_count\"]"),
        "{experience_repair_contract_body}"
    );
    assert!(
        experience_repair_contract_body.contains("\"unsupported_fields\":[\"prompt\",\"profile\",\"model\",\"messages\",\"stream\",\"max_tokens\",\"tools\",\"tool_choice\",\"response_format\",\"logprobs\",\"quarantine_path\",\"tenant_id\",\"workspace_id\",\"session_id\"]"),
        "{experience_repair_contract_body}"
    );
    assert!(chat_contract.contains("HTTP/1.1 200 OK"), "{chat_contract}");
    assert!(
        chat_contract_body.contains("\"endpoint\":\"/v1/chat/completions\""),
        "{chat_contract_body}"
    );
    assert!(
        chat_contract_body.contains(
            "\"supported_fields\":[\"model\",\"messages\",\"max_tokens\",\"max_completion_tokens\",\"n\",\"stream\",\"tenant_id\",\"workspace_id\",\"session_id\"]"
        ),
        "{chat_contract_body}"
    );
    assert!(
        chat_contract_body.contains("\"norion.runtime_model\"")
            && chat_contract_body.contains("\"norion.runtime_uncertainty_signal\"")
            && chat_contract_body.contains("\"norion.runtime_device_execution_source\""),
        "{chat_contract_body}"
    );
    assert_json_string_fields(
        chat_contract_body,
        &[
            "error.message",
            "error.type",
            "error.param",
            "error.code",
            "norion.endpoint",
            "norion.model",
            "norion.cancelled",
            "norion.timeout",
            "norion.retryable",
            "norion.runtime_error_note",
            "norion.memory_write_allowed",
            "norion.genome_write_allowed",
            "norion.self_evolution_write_allowed",
        ],
    );
    assert!(
        chat_contract_body.contains("\"stream_response_fields\"")
            && chat_contract_body.contains("\"data:chunk\"")
            && chat_contract_body.contains("\"object:chat.completion.chunk\"")
            && chat_contract_body.contains("\"norion.model\"")
            && chat_contract_body.contains("\"norion.compute_budget\",\"norion.compute_budget_summary\",\"norion.compute_budget_saved_tokens\",\"norion.compute_budget_avoided_tokens\",\"norion.compute_budget_kv_lookups_skipped\",\"norion.compute_budget_fanout_reduction\",\"norion.compute_budget_read_only\",\"norion.compute_budget_write_allowed\",\"norion.compute_budget_applied\",\"norion.stream_state\"")
            && chat_contract_body.contains("\"norion.stream_state\"")
            && chat_contract_body.contains("\"norion.streamed_tokens\"")
            && chat_contract_body.contains("\"norion.runtime_model\",\"norion.runtime_token_count\",\"norion.runtime_entropy_count\",\"norion.runtime_logprob_count\",\"norion.runtime_uncertainty_token_count\",\"norion.runtime_uncertainty_signal\",\"norion.runtime_average_entropy\",\"norion.runtime_average_neg_logprob\",\"norion.runtime_uncertainty_perplexity\",\"norion.runtime_architecture_signal\",\"norion.runtime_kv_precision_signal\",\"norion.runtime_device_execution_source\"")
            && chat_contract_body.contains("\"norion.retryable\"")
            && chat_contract_body.contains("\"norion.runtime_error_note\"")
            && chat_contract_body.contains("\"norion.memory_write_allowed\",\"norion.genome_write_allowed\",\"norion.self_evolution_write_allowed\""),
        "{chat_contract_body}"
    );
    assert!(
        chat_contract_body.contains(
            "\"unsupported_fields\":[\"tools\",\"tool_choice\",\"response_format\",\"logprobs\",\"temperature\",\"top_p\",\"presence_penalty\",\"frequency_penalty\",\"stop\",\"seed\",\"logit_bias\"]"
        ),
        "{chat_contract_body}"
    );
    assert!(
        completion_contract.contains("HTTP/1.1 200 OK"),
        "{completion_contract}"
    );
    assert!(
        completion_contract_body.contains("\"endpoint\":\"/v1/completions\""),
        "{completion_contract_body}"
    );
    assert_json_string_fields(
        completion_contract_body,
        &[
            "error.message",
            "error.type",
            "error.param",
            "error.code",
            "norion.endpoint",
            "norion.model",
            "norion.cancelled",
            "norion.timeout",
            "norion.retryable",
            "norion.runtime_error_note",
            "norion.memory_write_allowed",
            "norion.genome_write_allowed",
            "norion.self_evolution_write_allowed",
        ],
    );
    assert!(
        completion_contract_body
            .contains("\"supported_fields\":[\"model\",\"prompt\",\"max_tokens\",\"n\",\"tenant_id\",\"workspace_id\",\"session_id\"]"),
        "{completion_contract_body}"
    );
    assert!(
        completion_contract_body.contains(
            "\"unsupported_fields\":[\"stream\",\"logprobs\",\"suffix\",\"temperature\",\"top_p\",\"presence_penalty\",\"frequency_penalty\",\"stop\",\"seed\",\"logit_bias\"]"
        ),
        "{completion_contract_body}"
    );
    assert!(
        !completion_contract_body.contains("\"stream_response_fields\""),
        "{completion_contract_body}"
    );
    assert!(
        route_contract.contains("HTTP/1.1 200 OK"),
        "{route_contract}"
    );
    assert!(
        route_contract_body.contains("\"endpoint\":\"/v1/model-pool/route-plan\""),
        "{route_contract_body}"
    );
    assert!(
        route_contract_body.contains("\"supported_fields\":[\"task_kind\",\"task\",\"max_tokens\",\"max\",\"prompt\",\"content\",\"completed_roles\",\"completed_stage_roles\"]"),
        "{route_contract_body}"
    );
    assert!(
        route_contract_body.contains("\"response_fields\":[\"ok\",\"request_id\",\"schema_version\",\"contract_version\",\"task_kind\",\"read_only\",\"launches_process\",\"sends_prompt\",\"route_allowed\",\"reason\",\"route_block_reason\",\"role_candidates\",\"routing_weights\",\"service_backpressure\",\"dependency_precheck\",\"quality_context_tokens\",\"quality_context_required_tokens\",\"quality_context_sufficient\",\"quality_block_reason\",\"selected_role\",\"selected_base_url\",\"selected_port\",\"selected_default_max_tokens\",\"selected_context_window\",\"selected_context_required_tokens\",\"selected_context_buffer_tokens\",\"selected_context_buffer_policy\",\"selected_context_sufficient\",\"selected_context_block_reason\",\"configured_max_tokens\",\"effective_max_tokens\",\"max_tokens_clamped\",\"max_tokens_clamp_reason\",\"compute_budget_summary\",\"compute_budget_configured_max_tokens\",\"compute_budget_effective_max_tokens\",\"compute_budget_saved_tokens\",\"compute_budget_avoided_tokens\",\"compute_budget_max_tokens_clamped\",\"pool_dispatch\",\"route_metrics\",\"route_metrics.success_rate_milli\",\"route_metrics.failure_rate_milli\",\"worker_metrics\",\"worker_metrics.success_rate_milli\",\"worker_metrics.failure_rate_milli\",\"candidate_workers\"]"),
        "{route_contract_body}"
    );
    assert!(call_contract.contains("HTTP/1.1 200 OK"), "{call_contract}");
    assert!(
        call_contract_body.contains("\"endpoint\":\"/v1/model-pool/call\""),
        "{call_contract_body}"
    );
    assert!(
        call_contract_body.contains("\"supported_fields\":[\"task_kind\",\"task\",\"prompt\",\"content\",\"max_tokens\",\"max\",\"completed_roles\",\"completed_stage_roles\"]"),
        "{call_contract_body}"
    );
    assert!(
        call_contract_body.contains("\"response_fields\":[\"ok\",\"request_id\",\"schema_version\",\"contract_version\",\"task_kind\",\"read_only\",\"launches_process\",\"sends_prompt\",\"route_allowed\",\"reason\",\"route_block_reason\",\"role_candidates\",\"dependency_precheck\",\"quality_context_tokens\",\"quality_context_required_tokens\",\"quality_context_sufficient\",\"quality_block_reason\",\"selected_role\",\"selected_base_url\",\"selected_port\",\"selected_default_max_tokens\",\"configured_max_tokens\",\"effective_max_tokens\",\"max_tokens_clamped\",\"max_tokens_clamp_reason\",\"pool_dispatch\",\"route_metrics\",\"route_metrics.success_rate_milli\",\"route_metrics.failure_rate_milli\",\"worker_metrics\",\"worker_metrics.success_rate_milli\",\"worker_metrics.failure_rate_milli\",\"candidate_workers\",\"elapsed_ms\",\"answer_chars\",\"answer_bytes\",\"answer_approx_tokens\",\"answer\",\"endpoint\",\"call_state\",\"cancelled\",\"timeout\",\"partial_result\",\"partial_finalized\",\"queue_time_ms\",\"compute_budget_summary\",\"compute_budget_configured_max_tokens\",\"compute_budget_effective_max_tokens\",\"compute_budget_saved_tokens\",\"compute_budget_avoided_tokens\",\"compute_budget_max_tokens_clamped\",\"error\",\"retryable\",\"dispatch_attempted\",\"persistent_writes\",\"memory_write_allowed\",\"genome_write_allowed\",\"self_evolution_write_allowed\"]"),
        "{call_contract_body}"
    );
    assert!(
        cancel_contract.contains("HTTP/1.1 200 OK"),
        "{cancel_contract}"
    );
    assert!(
        cancel_contract_body.contains("\"endpoint\":\"/v1/requests/cancel\""),
        "{cancel_contract_body}"
    );
    assert!(
        cancel_contract_body
            .contains("\"supported_fields\":[\"request_id\",\"reason\",\"retag_label\"]"),
        "{cancel_contract_body}"
    );
    assert!(
        cancel_contract_body.contains("\"response_fields\":[\"ok\",\"request_id\",\"target_request_id\",\"target_active\",\"target_endpoint\",\"repair_factor_released\",\"repair_factor\",\"retag_applied\",\"retag_label\",\"reason\",\"cooperative_only\",\"persistent_writes\",\"next_step\"]"),
        "{cancel_contract_body}"
    );
    assert!(
        feedback_contract.contains("HTTP/1.1 200 OK"),
        "{feedback_contract}"
    );
    assert!(
        feedback_contract_body.contains("\"endpoint\":\"/v1/feedback\""),
        "{feedback_contract_body}"
    );
    assert!(
        feedback_contract_body.contains(
            "\"supported_fields\":[\"experience_id\",\"memory_id\",\"action\",\"amount\"]"
        ),
        "{feedback_contract_body}"
    );
    assert!(
        feedback_contract_body.contains("\"state.evolution_external_feedbacks\""),
        "{feedback_contract_body}"
    );
    assert!(
        rust_check_contract.contains("HTTP/1.1 200 OK"),
        "{rust_check_contract}"
    );
    assert!(
        rust_check_contract_body.contains("\"endpoint\":\"/v1/rust-check\""),
        "{rust_check_contract_body}"
    );
    assert!(
        rust_check_contract_body.contains("\"rust_check.passed\""),
        "{rust_check_contract_body}"
    );
    assert!(
        rust_check_contract_body.contains("\"feedback.memory_ids\""),
        "{rust_check_contract_body}"
    );
    assert!(
        replay_contract.contains("HTTP/1.1 200 OK"),
        "{replay_contract}"
    );
    assert!(
        replay_contract_body.contains("\"endpoint\":\"/v1/replay\""),
        "{replay_contract_body}"
    );
    assert!(
        replay_contract_body.contains("\"replay.live_evolution_items\""),
        "{replay_contract_body}"
    );
    assert!(
        self_improve_contract.contains("HTTP/1.1 200 OK"),
        "{self_improve_contract}"
    );
    assert!(
        self_improve_contract_body.contains("\"endpoint\":\"/v1/self-improve\""),
        "{self_improve_contract_body}"
    );
    assert!(
        self_improve_contract_body.contains("\"self_improve.self_evolution_admission_checked\""),
        "{self_improve_contract_body}"
    );
    assert!(
        self_improve_contract_body.contains("\"self_evolution_admission.git_write_allowed\""),
        "{self_improve_contract_body}"
    );
    assert!(
        unsupported_completion_stream.contains("HTTP/1.1 400 Bad Request"),
        "{unsupported_completion_stream}"
    );
    assert!(
        unsupported_completion_stream_body.contains("stream=true is not supported"),
        "{unsupported_completion_stream_body}"
    );
    assert!(
        unsupported_chat_tools.contains("HTTP/1.1 400 Bad Request"),
        "{unsupported_chat_tools}"
    );
    assert!(
        unsupported_chat_tools_body
            .contains("OpenAI chat completions does not support request field: tools"),
        "{unsupported_chat_tools_body}"
    );
    assert!(
        unsupported_chat_n.contains("HTTP/1.1 400 Bad Request"),
        "{unsupported_chat_n}"
    );
    assert!(
        unsupported_chat_n_body.contains("OpenAI chat completions only supports request field n=1"),
        "{unsupported_chat_n_body}"
    );
    assert!(
        unsupported_completion_n.contains("HTTP/1.1 400 Bad Request"),
        "{unsupported_completion_n}"
    );
    assert!(
        unsupported_completion_n_body
            .contains("OpenAI completions only supports request field n=1"),
        "{unsupported_completion_n_body}"
    );
    assert!(
        unsupported_chat_temperature.contains("HTTP/1.1 400 Bad Request"),
        "{unsupported_chat_temperature}"
    );
    assert!(
        unsupported_chat_temperature_body
            .contains("OpenAI chat completions does not support request field: temperature"),
        "{unsupported_chat_temperature_body}"
    );
    assert!(
        unsupported_completion_stop.contains("HTTP/1.1 400 Bad Request"),
        "{unsupported_completion_stop}"
    );
    assert!(
        unsupported_completion_stop_body
            .contains("OpenAI completions does not support request field: stop"),
        "{unsupported_completion_stop_body}"
    );
    assert!(diagnostics.contains("HTTP/1.1 200 OK"), "{diagnostics}");
    assert!(
        diagnostics_body.contains("\"ok\":true"),
        "{diagnostics_body}"
    );
    assert!(
        diagnostics_body.contains("\"runtime_mode\":\"built-in\""),
        "{diagnostics_body}"
    );
    assert!(
        diagnostics_body.contains("\"readiness_ok\":true"),
        "{diagnostics_body}"
    );
    assert!(
        diagnostics_body.contains("\"last_inference\":null"),
        "{diagnostics_body}"
    );

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn model_service_http_smoke_covers_english_chinese_and_rust_prompts() {
    let asset_dir = target_asset_dir("model-service-english-rust-prompts");
    fs::create_dir_all(&asset_dir).unwrap();
    let bind = reserve_loopback_addr();
    let args = Args::parse(vec![
        "--serve-bind".to_owned(),
        bind.clone(),
        "--serve-max-requests".to_owned(),
        "8".to_owned(),
        "--memory".to_owned(),
        asset_dir.join("memory.ndkv").display().to_string(),
        "--experience".to_owned(),
        asset_dir.join("experience.ndkv").display().to_string(),
        "--adaptive".to_owned(),
        asset_dir.join("adaptive.ndkv").display().to_string(),
        "service multilingual smoke prompt".to_owned(),
    ]);
    let calls = Arc::new(Mutex::new(Vec::new()));
    let service_backend = RecordingBackend::new(Arc::clone(&calls));
    let service_args = args.clone();
    let handle = thread::spawn(move || {
        let mut engine = NoironEngine::new();
        configure_engine(&mut engine, &service_args);
        let mut backend = service_backend;
        run_model_service_for_args(&mut engine, &mut backend, &service_args)
    });

    let health = wait_for_http_response(&bind, "GET", "/health", None);
    let english_chat = service_http_request(
        &bind,
        "POST",
        "/v1/chat/completions",
        Some(
            "{\"model\":\"rust-norion-local\",\"messages\":[{\"role\":\"user\",\"content\":\"Explain how persistent KV memory reduces wasted compute.\"}],\"max_tokens\":12}",
        ),
    );
    let chinese_chat = service_http_request(
        &bind,
        "POST",
        "/v1/chat/completions",
        Some(
            "{\"model\":\"rust-norion-local\",\"messages\":[{\"role\":\"user\",\"content\":\"用中文解释持久 KV 记忆如何减少重复计算。\"}],\"max_tokens\":16}",
        ),
    );
    let rust_intent_chat = service_http_request(
        &bind,
        "POST",
        "/v1/chat/completions",
        Some(
            "{\"model\":\"rust-norion-local\",\"messages\":[{\"role\":\"user\",\"content\":\"Explain ownership and lifetime rules for checked add.\"}],\"max_tokens\":14}",
        ),
    );
    let scoped_chat = service_http_request(
        &bind,
        "POST",
        "/v1/chat/completions",
        Some(
            "{\"model\":\"rust-norion-local\",\"messages\":[{\"role\":\"user\",\"content\":\"Keep this memory in scoped session.\"}],\"tenant_id\":\"tenant-a\",\"workspace_id\":\"workspace\",\"session_id\":\"chat-1\",\"max_tokens\":8}",
        ),
    );
    let alias_chat = service_http_request(
        &bind,
        "POST",
        "/v1/chat/completions",
        Some(
            "{\"model\":\"rust-norion-local\",\"messages\":[{\"role\":\"user\",\"content\":\"Limit this OpenAI chat with the new token field.\"}],\"max_completion_tokens\":9}",
        ),
    );
    let rust_completion = service_http_request(
        &bind,
        "POST",
        "/v1/completions",
        Some(
            "{\"model\":\"rust-norion-local\",\"prompt\":\"Write Rust code for a checked add helper.\",\"max_tokens\":24}",
        ),
    );
    let scoped_completion = service_http_request(
        &bind,
        "POST",
        "/v1/completions",
        Some(
            "{\"model\":\"rust-norion-local\",\"prompt\":\"Keep this completion in scoped memory.\",\"tenant_id\":\"tenant-b\",\"workspace_id\":\"workspace\",\"session_id\":\"completion-1\",\"max_tokens\":10}",
        ),
    );
    handle.join().unwrap().unwrap();

    let health_body = http_body(&health);
    let english_body = http_body(&english_chat);
    let chinese_body = http_body(&chinese_chat);
    let rust_intent_body = http_body(&rust_intent_chat);
    let scoped_body = http_body(&scoped_chat);
    let alias_body = http_body(&alias_chat);
    let rust_body = http_body(&rust_completion);
    let scoped_completion_body = http_body(&scoped_completion);
    assert!(health_body.contains("\"ok\":true"), "{health_body}");
    assert!(
        english_body.contains("\"object\":\"chat.completion\""),
        "{english_body}"
    );
    assert!(
        chinese_body.contains("\"object\":\"chat.completion\""),
        "{chinese_body}"
    );
    assert!(
        rust_intent_body.contains("\"object\":\"chat.completion\""),
        "{rust_intent_body}"
    );
    assert!(
        rust_intent_body.contains("\"profile\":\"coding\"")
            && rust_intent_body.contains("\"coding_language\":\"rust\"")
            && rust_intent_body.contains("\"rust_coding\":true"),
        "{rust_intent_body}"
    );
    assert!(
        scoped_body.contains("\"object\":\"chat.completion\""),
        "{scoped_body}"
    );
    assert!(
        alias_body.contains("\"object\":\"chat.completion\""),
        "{alias_body}"
    );
    assert!(
        rust_body.contains("\"object\":\"text_completion\""),
        "{rust_body}"
    );
    assert!(
        scoped_completion_body.contains("\"object\":\"text_completion\""),
        "{scoped_completion_body}"
    );

    let calls = calls.lock().unwrap();
    assert_eq!(calls.len(), 7, "{calls:?}");
    assert!(
        calls[0]
            .prompt
            .contains("Explain how persistent KV memory reduces wasted compute."),
        "{calls:?}"
    );
    assert_eq!(calls[0].max_tokens, Some(12), "{calls:?}");
    assert!(
        calls[1]
            .prompt
            .contains("用中文解释持久 KV 记忆如何减少重复计算。"),
        "{calls:?}"
    );
    assert_eq!(calls[1].max_tokens, Some(16), "{calls:?}");
    assert!(
        calls[2]
            .prompt
            .contains("Explain ownership and lifetime rules for checked add."),
        "{calls:?}"
    );
    assert_eq!(calls[2].profile, TaskProfile::Coding, "{calls:?}");
    assert_eq!(calls[2].max_tokens, Some(14), "{calls:?}");
    assert!(
        calls[3]
            .prompt
            .contains("Keep this memory in scoped session."),
        "{calls:?}"
    );
    assert_eq!(calls[3].max_tokens, Some(8), "{calls:?}");
    assert_eq!(
        calls[3].tenant_scope,
        Some(rust_norion::TenantScope::new(
            "tenant-a",
            "workspace",
            "chat-1"
        )),
        "{calls:?}"
    );
    assert!(
        calls[4]
            .prompt
            .contains("Limit this OpenAI chat with the new token field."),
        "{calls:?}"
    );
    assert_eq!(calls[4].max_tokens, Some(9), "{calls:?}");
    assert!(
        calls[5]
            .prompt
            .contains("Write Rust code for a checked add helper."),
        "{calls:?}"
    );
    assert_eq!(calls[5].profile, TaskProfile::Coding, "{calls:?}");
    assert_eq!(calls[5].max_tokens, Some(24), "{calls:?}");
    assert!(
        calls[6]
            .prompt
            .contains("Keep this completion in scoped memory."),
        "{calls:?}"
    );
    assert_eq!(calls[6].max_tokens, Some(10), "{calls:?}");
    assert_eq!(
        calls[6].tenant_scope,
        Some(rust_norion::TenantScope::new(
            "tenant-b",
            "workspace",
            "completion-1"
        )),
        "{calls:?}"
    );

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn model_service_generate_raw_output_keeps_unrepaired_answer_for_strict_consumers() {
    let asset_dir = target_asset_dir("model-service-raw-output-smoke");
    fs::create_dir_all(&asset_dir).unwrap();
    let bind = reserve_loopback_addr();
    let args = Args::parse(vec![
        "--serve-bind".to_owned(),
        bind.clone(),
        "--serve-max-requests".to_owned(),
        "2".to_owned(),
        "--memory".to_owned(),
        asset_dir.join("memory.ndkv").display().to_string(),
        "--experience".to_owned(),
        asset_dir.join("experience.ndkv").display().to_string(),
        "--adaptive".to_owned(),
        asset_dir.join("adaptive.ndkv").display().to_string(),
        "service raw output smoke prompt".to_owned(),
    ]);
    let service_args = args.clone();
    let handle = thread::spawn(move || {
        let mut engine = NoironEngine::new();
        configure_engine(&mut engine, &service_args);
        let mut backend = ShortRawBackend;
        run_model_service_for_args(&mut engine, &mut backend, &service_args)
    });

    let health = wait_for_http_response(&bind, "GET", "/health", None);
    let generate = service_http_request(
        &bind,
        "POST",
        "/v1/generate",
        Some(
            "{\"prompt\":\"Explain Rust Noiron adaptive routing decisions\",\"profile\":\"coding\",\"output\":\"raw\"}",
        ),
    );
    handle.join().unwrap().unwrap();

    let health_body = http_body(&health);
    let generate_body = http_body(&generate);
    assert!(health_body.contains("\"ok\":true"), "{health_body}");
    assert!(
        generate_body.contains("\"output_mode\":\"raw\""),
        "{generate_body}"
    );
    assert!(
        generate_body.contains("\"answer\":\"Rust routes.\""),
        "{generate_body}"
    );
    assert!(
        generate_body.contains("\"raw_answer\":\"Rust routes.\""),
        "{generate_body}"
    );
    assert!(
        generate_body.contains("\"enhanced_answer\":\"Rust routes.\\n\\nReflection repair:"),
        "{generate_body}"
    );
    assert!(
        !generate_body.contains("\"answer\":\"Rust routes.\\n\\nReflection repair:"),
        "{generate_body}"
    );

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn model_service_generation_runtime_errors_return_structured_json() {
    let asset_dir = target_asset_dir("model-service-runtime-error-json");
    fs::create_dir_all(&asset_dir).unwrap();
    let bind = reserve_loopback_addr();
    let args = Args::parse(vec![
        "--serve-bind".to_owned(),
        bind.clone(),
        "--serve-max-requests".to_owned(),
        "4".to_owned(),
        "--memory".to_owned(),
        asset_dir.join("memory.ndkv").display().to_string(),
        "--experience".to_owned(),
        asset_dir.join("experience.ndkv").display().to_string(),
        "--adaptive".to_owned(),
        asset_dir.join("adaptive.ndkv").display().to_string(),
        "service runtime error prompt".to_owned(),
    ]);
    let service_args = args.clone();
    let handle = thread::spawn(move || {
        let mut engine = NoironEngine::new();
        configure_engine(&mut engine, &service_args);
        let mut backend = RuntimeErrorBackend;
        run_model_service_for_args(&mut engine, &mut backend, &service_args)
    });

    let health = wait_for_http_response(&bind, "GET", "/health", None);
    let generate = service_http_request(
        &bind,
        "POST",
        "/v1/generate",
        Some("{\"prompt\":\"trigger runtime timeout\",\"profile\":\"coding\"}"),
    );
    let openai_chat = service_http_request(
        &bind,
        "POST",
        "/v1/chat/completions",
        Some(
            "{\"model\":\"rust-norion-local\",\"messages\":[{\"role\":\"user\",\"content\":\"trigger runtime timeout\"}]}",
        ),
    );
    let openai_completion = service_http_request(
        &bind,
        "POST",
        "/v1/completions",
        Some("{\"model\":\"rust-norion-local\",\"prompt\":\"trigger runtime timeout\"}"),
    );
    handle.join().unwrap().unwrap();

    let health_body = http_body(&health);
    let generate_body = http_body(&generate);
    let chat_body = http_body(&openai_chat);
    let completion_body = http_body(&openai_completion);

    assert!(health_body.contains("\"ok\":true"), "{health_body}");
    assert!(
        generate.contains("HTTP/1.1 504 Gateway Timeout"),
        "{generate}"
    );
    assert!(generate_body.contains("\"ok\":false"), "{generate_body}");
    assert!(
        generate_body.contains("\"endpoint\":\"generate\""),
        "{generate_body}"
    );
    assert!(
        generate_body.contains("\"error_type\":\"runtime_error\""),
        "{generate_body}"
    );
    assert!(
        generate_body.contains("\"timeout\":true"),
        "{generate_body}"
    );
    assert!(
        generate_body.contains("\"retryable\":true"),
        "{generate_body}"
    );
    assert!(
        generate_body
            .contains("\"runtime_error_note\":\"runtime_error:label=runtime_error:timeout=true"),
        "{generate_body}"
    );
    assert_generation_error_compute_budget_fields(generate_body);
    assert!(
        generate_body.contains("\"persistent_writes\":false"),
        "{generate_body}"
    );
    assert!(
        generate_body.contains("\"memory_write_allowed\":false"),
        "{generate_body}"
    );
    assert!(
        generate_body.contains("\"genome_write_allowed\":false"),
        "{generate_body}"
    );
    assert!(
        generate_body.contains("\"self_evolution_write_allowed\":false"),
        "{generate_body}"
    );

    assert!(
        openai_chat.contains("HTTP/1.1 504 Gateway Timeout"),
        "{openai_chat}"
    );
    assert!(
        chat_body.contains("\"error\":{\"message\":\"Runtime backend error:"),
        "{chat_body}"
    );
    assert!(
        chat_body.contains("\"type\":\"runtime_error\""),
        "{chat_body}"
    );
    assert!(
        chat_body.contains("\"endpoint\":\"chat-completions\""),
        "{chat_body}"
    );
    assert!(
        chat_body.contains("\"model\":\"rust-norion-local\""),
        "{chat_body}"
    );
    assert_generation_error_compute_budget_fields(chat_body);
    assert!(
        chat_body.contains("\"persistent_writes\":false"),
        "{chat_body}"
    );

    assert!(
        openai_completion.contains("HTTP/1.1 504 Gateway Timeout"),
        "{openai_completion}"
    );
    assert!(
        completion_body.contains("\"endpoint\":\"completions\""),
        "{completion_body}"
    );
    assert!(
        completion_body.contains("\"type\":\"runtime_error\""),
        "{completion_body}"
    );
    assert!(
        completion_body.contains("\"self_evolution_write_allowed\":false"),
        "{completion_body}"
    );
    assert_generation_error_compute_budget_fields(completion_body);

    fs::remove_dir_all(asset_dir).unwrap();
}

fn assert_generation_error_compute_budget_fields(body: &str) {
    assert!(body.contains("\"compute_budget\":null"), "{body}");
    assert!(
        body.contains("\"compute_budget_summary\":\"unavailable_failed_before_final_outcome\""),
        "{body}"
    );
    assert!(body.contains("\"compute_budget_saved_tokens\":0"), "{body}");
    assert!(
        body.contains("\"compute_budget_avoided_tokens\":0"),
        "{body}"
    );
    assert!(
        body.contains("\"compute_budget_kv_lookups_skipped\":0"),
        "{body}"
    );
    assert!(
        body.contains("\"compute_budget_fanout_reduction\":0"),
        "{body}"
    );
    assert!(body.contains("\"compute_budget_read_only\":true"), "{body}");
    assert!(
        body.contains("\"compute_budget_write_allowed\":false"),
        "{body}"
    );
    assert!(body.contains("\"compute_budget_applied\":false"), "{body}");
}

#[test]
fn model_service_health_responds_while_generate_is_running() {
    let asset_dir = target_asset_dir("model-service-concurrent-health");
    fs::create_dir_all(&asset_dir).unwrap();
    let bind = reserve_loopback_addr();
    let args = Args::parse(vec![
        "--serve-bind".to_owned(),
        bind.clone(),
        "--serve-max-requests".to_owned(),
        "4".to_owned(),
        "--memory".to_owned(),
        asset_dir.join("memory.ndkv").display().to_string(),
        "--experience".to_owned(),
        asset_dir.join("experience.ndkv").display().to_string(),
        "--adaptive".to_owned(),
        asset_dir.join("adaptive.ndkv").display().to_string(),
        "service concurrent health prompt".to_owned(),
    ]);
    let started = Arc::new(AtomicBool::new(false));
    let release = Arc::new(AtomicBool::new(false));
    let service_backend = BlockingBackend {
        started: Arc::clone(&started),
        release: Arc::clone(&release),
    };
    let service_args = args.clone();
    let handle = thread::spawn(move || {
        let mut engine = NoironEngine::new();
        configure_engine(&mut engine, &service_args);
        let mut backend = service_backend;
        run_model_service_for_args(&mut engine, &mut backend, &service_args)
    });

    let first_health = wait_for_http_response(&bind, "GET", "/health", None);
    assert!(http_body(&first_health).contains("\"ok\":true"));
    assert!(http_body(&first_health).contains("\"runtime_mode\":\"built-in\""));
    assert!(http_body(&first_health).contains("\"gemma_runtime_server\":null"));
    assert!(http_body(&first_health).contains("\"gemma_runtime_reachable\":null"));
    assert!(http_body(&first_health).contains("\"last_inference\":null"));

    let generate_bind = bind.clone();
    let generate_handle = thread::spawn(move || {
        service_http_request(
            &generate_bind,
            "POST",
            "/v1/generate",
            Some("{\"prompt\":\"hold generate\",\"profile\":\"coding\"}"),
        )
    });
    for _ in 0..100 {
        if started.load(Ordering::SeqCst) {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    assert!(
        started.load(Ordering::SeqCst),
        "generate request should enter the blocking backend"
    );

    let health_during_generate = service_http_request(&bind, "GET", "/health", None);
    assert!(
        http_body(&health_during_generate).contains("\"ok\":true"),
        "{health_during_generate}"
    );
    assert!(
        http_body(&health_during_generate).contains("\"active_engine_requests\":1"),
        "{health_during_generate}"
    );
    assert!(
        http_body(&health_during_generate).contains("\"engine_busy\":true"),
        "{health_during_generate}"
    );

    release.store(true, Ordering::SeqCst);
    let generate = generate_handle.join().unwrap();
    assert!(http_body(&generate).contains("\"ok\":true"), "{generate}");
    let final_health = service_http_request(&bind, "GET", "/health", None);
    let final_health_body = http_body(&final_health);
    assert!(
        final_health_body.contains("\"last_inference\":{"),
        "{final_health_body}"
    );
    assert!(
        final_health_body.contains("\"endpoint\":\"generate\""),
        "{final_health_body}"
    );
    assert!(
        final_health_body.contains("\"elapsed_ms\":"),
        "{final_health_body}"
    );
    assert!(
        final_health_body.contains("\"runtime_token_count\":"),
        "{final_health_body}"
    );
    handle.join().unwrap().unwrap();
    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn model_service_request_cancel_releases_repair_factor_for_active_generate() {
    let asset_dir = target_asset_dir("model-service-request-cancel");
    fs::create_dir_all(&asset_dir).unwrap();
    let bind = reserve_loopback_addr();
    let args = Args::parse(vec![
        "--serve-bind".to_owned(),
        bind.clone(),
        "--serve-max-requests".to_owned(),
        "5".to_owned(),
        "--memory".to_owned(),
        asset_dir.join("memory.ndkv").display().to_string(),
        "--experience".to_owned(),
        asset_dir.join("experience.ndkv").display().to_string(),
        "--adaptive".to_owned(),
        asset_dir.join("adaptive.ndkv").display().to_string(),
        "service request cancel prompt".to_owned(),
    ]);
    let started = Arc::new(AtomicBool::new(false));
    let release = Arc::new(AtomicBool::new(false));
    let service_backend = BlockingBackend {
        started: Arc::clone(&started),
        release: Arc::clone(&release),
    };
    let service_args = args.clone();
    let handle = thread::spawn(move || {
        let mut engine = NoironEngine::new();
        configure_engine(&mut engine, &service_args);
        let mut backend = service_backend;
        run_model_service_for_args(&mut engine, &mut backend, &service_args)
    });

    let first_health = wait_for_http_response(&bind, "GET", "/health", None);
    assert!(http_body(&first_health).contains("\"ok\":true"));

    let generate_bind = bind.clone();
    let generate_handle = thread::spawn(move || {
        service_http_request(
            &generate_bind,
            "POST",
            "/v1/generate",
            Some("{\"prompt\":\"hold generate for repair\",\"profile\":\"coding\"}"),
        )
    });
    for _ in 0..100 {
        if started.load(Ordering::SeqCst) {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    assert!(
        started.load(Ordering::SeqCst),
        "generate request should enter the blocking backend"
    );

    let cancel = service_http_request(
        &bind,
        "POST",
        "/v1/requests/cancel",
        Some(
            "{\"request_id\":2,\"reason\":\"operator_runtime_splice\",\"retag_label\":\"repair_factor:runtime_splice\"}",
        ),
    );
    let cancel_body = http_body(&cancel);
    assert!(
        cancel_body.contains("\"target_request_id\":2"),
        "{cancel_body}"
    );
    assert!(
        cancel_body.contains("\"target_active\":true"),
        "{cancel_body}"
    );
    assert!(
        cancel_body.contains("\"repair_factor_released\":true"),
        "{cancel_body}"
    );
    assert!(
        cancel_body.contains("\"repair_factor\":\"runtime_request_splice\""),
        "{cancel_body}"
    );
    assert!(
        cancel_body.contains("\"retag_applied\":true"),
        "{cancel_body}"
    );
    assert!(
        cancel_body.contains("\"persistent_writes\":false"),
        "{cancel_body}"
    );

    let health_after_cancel = service_http_request(&bind, "GET", "/health", None);
    let health_after_cancel_body = http_body(&health_after_cancel);
    assert!(
        health_after_cancel_body.contains("\"cancel_requested\":true"),
        "{health_after_cancel_body}"
    );
    assert!(
        health_after_cancel_body.contains("\"retag_label\":\"repair_factor:runtime_splice\""),
        "{health_after_cancel_body}"
    );

    release.store(true, Ordering::SeqCst);
    let generate = generate_handle.join().unwrap();
    let generate_body = http_body(&generate);
    assert!(generate.contains("HTTP/1.1 409 Conflict"), "{generate}");
    assert!(
        generate_body.contains("\"request_id\":2"),
        "{generate_body}"
    );
    assert!(
        generate_body.contains("\"endpoint\":\"generate\""),
        "{generate_body}"
    );
    assert!(
        generate_body.contains("request cancelled by runtime_request_splice"),
        "{generate_body}"
    );
    assert!(
        generate_body.contains("\"error_type\":\"cancelled\""),
        "{generate_body}"
    );
    assert!(
        generate_body.contains("\"cancelled\":true"),
        "{generate_body}"
    );
    assert!(
        generate_body.contains("\"compute_budget\":\""),
        "{generate_body}"
    );
    assert!(
        generate_body.contains("\"compute_budget_summary\":\"compute_budget_schedule"),
        "{generate_body}"
    );
    assert_cancelled_generate_compute_budget_fields(generate_body);
    assert!(
        generate_body.contains("\"persistent_writes\":false"),
        "{generate_body}"
    );

    let final_health = service_http_request(&bind, "GET", "/health", None);
    let final_health_body = http_body(&final_health);
    assert!(
        final_health_body.contains("\"active_engine_requests\":0"),
        "{final_health_body}"
    );
    handle.join().unwrap().unwrap();
    fs::remove_dir_all(asset_dir).unwrap();
}

fn assert_cancelled_generate_compute_budget_fields(body: &str) {
    assert!(body.contains("\"compute_budget_saved_tokens\":"), "{body}");
    assert!(
        body.contains("\"compute_budget_avoided_tokens\":"),
        "{body}"
    );
    assert!(
        body.contains("\"compute_budget_kv_lookups_skipped\":"),
        "{body}"
    );
    assert!(
        body.contains("\"compute_budget_fanout_reduction\":"),
        "{body}"
    );
    assert!(body.contains("\"compute_budget_read_only\":true"), "{body}");
    assert!(
        body.contains("\"compute_budget_write_allowed\":false"),
        "{body}"
    );
    assert!(body.contains("\"compute_budget_applied\":false"), "{body}");
}

#[test]
fn model_service_openai_chat_completions_cancel_returns_openai_error_json() {
    let asset_dir = target_asset_dir("model-service-openai-chat-cancel");
    fs::create_dir_all(&asset_dir).unwrap();
    let bind = reserve_loopback_addr();
    let args = Args::parse(vec![
        "--serve-bind".to_owned(),
        bind.clone(),
        "--serve-max-requests".to_owned(),
        "4".to_owned(),
        "--memory".to_owned(),
        asset_dir.join("memory.ndkv").display().to_string(),
        "--experience".to_owned(),
        asset_dir.join("experience.ndkv").display().to_string(),
        "--adaptive".to_owned(),
        asset_dir.join("adaptive.ndkv").display().to_string(),
        "service openai chat cancel prompt".to_owned(),
    ]);
    let started = Arc::new(AtomicBool::new(false));
    let release = Arc::new(AtomicBool::new(false));
    let service_backend = BlockingBackend {
        started: Arc::clone(&started),
        release: Arc::clone(&release),
    };
    let service_args = args.clone();
    let handle = thread::spawn(move || {
        let mut engine = NoironEngine::new();
        configure_engine(&mut engine, &service_args);
        let mut backend = service_backend;
        run_model_service_for_args(&mut engine, &mut backend, &service_args)
    });

    let first_health = wait_for_http_response(&bind, "GET", "/health", None);
    assert!(http_body(&first_health).contains("\"ok\":true"));

    let chat_bind = bind.clone();
    let chat_handle = thread::spawn(move || {
        service_http_request(
            &chat_bind,
            "POST",
            "/v1/chat/completions",
            Some(
                "{\"model\":\"rust-norion-local\",\"messages\":[{\"role\":\"user\",\"content\":\"hold OpenAI chat for cancel\"}],\"max_tokens\":8}",
            ),
        )
    });
    for _ in 0..100 {
        if started.load(Ordering::SeqCst) {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    assert!(
        started.load(Ordering::SeqCst),
        "OpenAI chat request should enter the blocking backend"
    );

    let cancel = service_http_request(
        &bind,
        "POST",
        "/v1/requests/cancel",
        Some(
            "{\"request_id\":2,\"reason\":\"operator_openai_chat_cancel\",\"retag_label\":\"repair_factor:runtime_splice\"}",
        ),
    );
    let cancel_body = http_body(&cancel);
    assert!(
        cancel_body.contains("\"target_request_id\":2"),
        "{cancel_body}"
    );
    assert!(
        cancel_body.contains("\"target_active\":true"),
        "{cancel_body}"
    );
    assert!(
        cancel_body.contains("\"persistent_writes\":false"),
        "{cancel_body}"
    );

    release.store(true, Ordering::SeqCst);
    let chat = chat_handle.join().unwrap();
    let chat_body = http_body(&chat);
    assert!(chat.contains("HTTP/1.1 409 Conflict"), "{chat}");
    assert!(
        chat_body.contains("\"error\":{\"message\":\"request cancelled by runtime_request_splice\",\"type\":\"cancelled\",\"param\":null,\"code\":null}"),
        "{chat_body}"
    );
    assert!(chat_body.contains("\"norion\":{"), "{chat_body}");
    assert!(chat_body.contains("\"request_id\":2"), "{chat_body}");
    assert!(
        chat_body.contains("\"endpoint\":\"chat-completions\""),
        "{chat_body}"
    );
    assert!(
        chat_body.contains("\"model\":\"rust-norion-local\""),
        "{chat_body}"
    );
    assert!(chat_body.contains("\"cancelled\":true"), "{chat_body}");
    assert!(
        chat_body.contains("\"runtime_error_note\":null"),
        "{chat_body}"
    );
    assert_cancelled_generate_compute_budget_fields(chat_body);
    assert!(
        chat_body.contains("\"persistent_writes\":false"),
        "{chat_body}"
    );

    let final_health = service_http_request(&bind, "GET", "/health", None);
    let final_health_body = http_body(&final_health);
    assert!(
        final_health_body.contains("\"active_engine_requests\":0"),
        "{final_health_body}"
    );
    handle.join().unwrap().unwrap();
    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn model_service_openai_completions_cancel_returns_openai_error_json() {
    let asset_dir = target_asset_dir("model-service-openai-completions-cancel");
    fs::create_dir_all(&asset_dir).unwrap();
    let bind = reserve_loopback_addr();
    let args = Args::parse(vec![
        "--serve-bind".to_owned(),
        bind.clone(),
        "--serve-max-requests".to_owned(),
        "4".to_owned(),
        "--memory".to_owned(),
        asset_dir.join("memory.ndkv").display().to_string(),
        "--experience".to_owned(),
        asset_dir.join("experience.ndkv").display().to_string(),
        "--adaptive".to_owned(),
        asset_dir.join("adaptive.ndkv").display().to_string(),
        "service openai completions cancel prompt".to_owned(),
    ]);
    let started = Arc::new(AtomicBool::new(false));
    let release = Arc::new(AtomicBool::new(false));
    let service_backend = BlockingBackend {
        started: Arc::clone(&started),
        release: Arc::clone(&release),
    };
    let service_args = args.clone();
    let handle = thread::spawn(move || {
        let mut engine = NoironEngine::new();
        configure_engine(&mut engine, &service_args);
        let mut backend = service_backend;
        run_model_service_for_args(&mut engine, &mut backend, &service_args)
    });

    let first_health = wait_for_http_response(&bind, "GET", "/health", None);
    assert!(http_body(&first_health).contains("\"ok\":true"));

    let completions_bind = bind.clone();
    let completions_handle = thread::spawn(move || {
        service_http_request(
            &completions_bind,
            "POST",
            "/v1/completions",
            Some(
                "{\"model\":\"rust-norion-local\",\"prompt\":\"hold OpenAI completions for cancel\",\"max_tokens\":8}",
            ),
        )
    });
    for _ in 0..100 {
        if started.load(Ordering::SeqCst) {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    assert!(
        started.load(Ordering::SeqCst),
        "OpenAI completions request should enter the blocking backend"
    );

    let cancel = service_http_request(
        &bind,
        "POST",
        "/v1/requests/cancel",
        Some(
            "{\"request_id\":2,\"reason\":\"operator_openai_completions_cancel\",\"retag_label\":\"repair_factor:runtime_splice\"}",
        ),
    );
    let cancel_body = http_body(&cancel);
    assert!(
        cancel_body.contains("\"target_request_id\":2"),
        "{cancel_body}"
    );
    assert!(
        cancel_body.contains("\"target_active\":true"),
        "{cancel_body}"
    );
    assert!(
        cancel_body.contains("\"persistent_writes\":false"),
        "{cancel_body}"
    );

    release.store(true, Ordering::SeqCst);
    let completions = completions_handle.join().unwrap();
    let completions_body = http_body(&completions);
    assert!(
        completions.contains("HTTP/1.1 409 Conflict"),
        "{completions}"
    );
    assert!(
        completions_body.contains("\"error\":{\"message\":\"request cancelled by runtime_request_splice\",\"type\":\"cancelled\",\"param\":null,\"code\":null}"),
        "{completions_body}"
    );
    assert!(
        completions_body.contains("\"norion\":{"),
        "{completions_body}"
    );
    assert!(
        completions_body.contains("\"request_id\":2"),
        "{completions_body}"
    );
    assert!(
        completions_body.contains("\"endpoint\":\"completions\""),
        "{completions_body}"
    );
    assert!(
        completions_body.contains("\"model\":\"rust-norion-local\""),
        "{completions_body}"
    );
    assert!(
        completions_body.contains("\"cancelled\":true"),
        "{completions_body}"
    );
    assert!(
        completions_body.contains("\"runtime_error_note\":null"),
        "{completions_body}"
    );
    assert_cancelled_generate_compute_budget_fields(completions_body);
    assert!(
        completions_body.contains("\"persistent_writes\":false"),
        "{completions_body}"
    );

    let final_health = service_http_request(&bind, "GET", "/health", None);
    let final_health_body = http_body(&final_health);
    assert!(
        final_health_body.contains("\"active_engine_requests\":0"),
        "{final_health_body}"
    );
    handle.join().unwrap().unwrap();
    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn model_service_generate_stream_cancel_emits_interrupted_final() {
    let asset_dir = target_asset_dir("model-service-stream-cancel");
    fs::create_dir_all(&asset_dir).unwrap();
    let trace = asset_dir.join("trace.jsonl");
    let bind = reserve_loopback_addr();
    let args = Args::parse(vec![
        "--serve-bind".to_owned(),
        bind.clone(),
        "--serve-max-requests".to_owned(),
        "4".to_owned(),
        "--memory".to_owned(),
        asset_dir.join("memory.ndkv").display().to_string(),
        "--experience".to_owned(),
        asset_dir.join("experience.ndkv").display().to_string(),
        "--adaptive".to_owned(),
        asset_dir.join("adaptive.ndkv").display().to_string(),
        "--trace".to_owned(),
        trace.display().to_string(),
        "service stream cancel prompt".to_owned(),
    ]);
    let started = Arc::new(AtomicBool::new(false));
    let release = Arc::new(AtomicBool::new(false));
    let service_backend = BlockingBackend {
        started: Arc::clone(&started),
        release: Arc::clone(&release),
    };
    let service_args = args.clone();
    let handle = thread::spawn(move || {
        let mut engine = NoironEngine::new();
        configure_engine(&mut engine, &service_args);
        let mut backend = service_backend;
        run_model_service_for_args(&mut engine, &mut backend, &service_args)
    });

    let first_health = wait_for_http_response(&bind, "GET", "/health", None);
    assert!(http_body(&first_health).contains("\"ok\":true"));

    let stream_bind = bind.clone();
    let stream_handle = thread::spawn(move || {
        service_http_request(
            &stream_bind,
            "POST",
            "/v1/generate-stream",
            Some("{\"prompt\":\"hold stream for cancel\",\"profile\":\"coding\"}"),
        )
    });
    for _ in 0..100 {
        if started.load(Ordering::SeqCst) {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    assert!(
        started.load(Ordering::SeqCst),
        "stream request should enter the streaming backend"
    );

    let cancel = service_http_request(
        &bind,
        "POST",
        "/v1/requests/cancel",
        Some(
            "{\"request_id\":2,\"reason\":\"operator_stream_cancel\",\"retag_label\":\"repair_factor:runtime_splice\"}",
        ),
    );
    let cancel_body = http_body(&cancel);
    assert!(
        cancel_body.contains("\"target_request_id\":2"),
        "{cancel_body}"
    );
    assert!(
        cancel_body.contains("\"target_active\":true"),
        "{cancel_body}"
    );
    assert!(
        cancel_body.contains("\"persistent_writes\":false"),
        "{cancel_body}"
    );

    release.store(true, Ordering::SeqCst);
    let stream = stream_handle.join().unwrap();
    assert!(stream.contains("event: delta"), "{stream}");
    assert!(stream.contains("data: partial "), "{stream}");
    assert!(stream.contains("event: final"), "{stream}");
    assert!(
        stream.contains("\"stream_state\":\"interrupted\""),
        "{stream}"
    );
    assert!(stream.contains("\"cancelled\":true"), "{stream}");
    assert!(stream.contains("\"retryable\":false"), "{stream}");
    assert!(stream.contains("\"runtime_error_note\":null"), "{stream}");
    assert!(stream.contains("\"partial_result\":true"), "{stream}");
    assert!(stream.contains("\"partial_finalized\":true"), "{stream}");
    assert!(stream.contains("\"streamed_tokens\":1"), "{stream}");
    assert!(stream.contains("\"persistent_writes\":false"), "{stream}");
    assert!(
        stream.contains("\"memory_write_allowed\":false"),
        "{stream}"
    );
    assert!(
        stream.contains("\"genome_write_allowed\":false"),
        "{stream}"
    );
    assert!(
        stream.contains("\"self_evolution_write_allowed\":false"),
        "{stream}"
    );
    assert!(stream.contains("event: done"), "{stream}");

    let final_health = service_http_request(&bind, "GET", "/health", None);
    let final_health_body = http_body(&final_health);
    assert!(
        final_health_body.contains("\"active_engine_requests\":0"),
        "{final_health_body}"
    );
    let trace_report = evaluate_trace_schema_jsonl(&trace).unwrap();
    assert!(trace_report.passed, "{:?}", trace_report.failures);
    assert_eq!(trace_report.checked_lines, 1);
    assert_eq!(trace_report.runtime_error_events, 1);
    handle.join().unwrap().unwrap();
    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn model_service_openai_chat_completions_stream_cancel_emits_error_chunk() {
    let asset_dir = target_asset_dir("model-service-openai-chat-stream-cancel");
    fs::create_dir_all(&asset_dir).unwrap();
    let bind = reserve_loopback_addr();
    let args = Args::parse(vec![
        "--serve-bind".to_owned(),
        bind.clone(),
        "--serve-max-requests".to_owned(),
        "4".to_owned(),
        "--memory".to_owned(),
        asset_dir.join("memory.ndkv").display().to_string(),
        "--experience".to_owned(),
        asset_dir.join("experience.ndkv").display().to_string(),
        "--adaptive".to_owned(),
        asset_dir.join("adaptive.ndkv").display().to_string(),
        "service openai stream cancel prompt".to_owned(),
    ]);
    let started = Arc::new(AtomicBool::new(false));
    let release = Arc::new(AtomicBool::new(false));
    let service_backend = BlockingBackend {
        started: Arc::clone(&started),
        release: Arc::clone(&release),
    };
    let service_args = args.clone();
    let handle = thread::spawn(move || {
        let mut engine = NoironEngine::new();
        configure_engine(&mut engine, &service_args);
        let mut backend = service_backend;
        run_model_service_for_args(&mut engine, &mut backend, &service_args)
    });

    let first_health = wait_for_http_response(&bind, "GET", "/health", None);
    assert!(http_body(&first_health).contains("\"ok\":true"));

    let stream_bind = bind.clone();
    let stream_handle = thread::spawn(move || {
        service_http_request(
            &stream_bind,
            "POST",
            "/v1/chat/completions",
            Some(
                "{\"model\":\"rust-norion-local\",\"messages\":[{\"role\":\"user\",\"content\":\"hold OpenAI stream for cancel\"}],\"stream\":true,\"max_tokens\":8}",
            ),
        )
    });
    for _ in 0..100 {
        if started.load(Ordering::SeqCst) {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    assert!(
        started.load(Ordering::SeqCst),
        "OpenAI stream request should enter the streaming backend"
    );

    let cancel = service_http_request(
        &bind,
        "POST",
        "/v1/requests/cancel",
        Some(
            "{\"request_id\":2,\"reason\":\"operator_openai_stream_cancel\",\"retag_label\":\"repair_factor:runtime_splice\"}",
        ),
    );
    let cancel_body = http_body(&cancel);
    assert!(
        cancel_body.contains("\"target_request_id\":2"),
        "{cancel_body}"
    );
    assert!(
        cancel_body.contains("\"target_active\":true"),
        "{cancel_body}"
    );
    assert!(
        cancel_body.contains("\"persistent_writes\":false"),
        "{cancel_body}"
    );

    release.store(true, Ordering::SeqCst);
    let stream = stream_handle.join().unwrap();
    assert!(
        stream.contains("content-type: text/event-stream"),
        "{stream}"
    );
    assert!(
        stream.contains("\"delta\":{\"role\":\"assistant\",\"content\":\"partial \"}"),
        "{stream}"
    );
    assert!(stream.contains("\"type\":\"cancelled\""), "{stream}");
    assert!(
        stream.contains("\"endpoint\":\"chat-completions-stream\""),
        "{stream}"
    );
    assert!(stream.contains("\"stream_state\":\"failed\""), "{stream}");
    assert!(stream.contains("\"cancelled\":true"), "{stream}");
    assert!(stream.contains("\"streamed_tokens\":1"), "{stream}");
    assert!(stream.contains("\"persistent_writes\":false"), "{stream}");
    assert!(
        stream.contains("\"memory_write_allowed\":false"),
        "{stream}"
    );
    assert!(
        stream.contains("\"genome_write_allowed\":false"),
        "{stream}"
    );
    assert!(
        stream.contains("\"self_evolution_write_allowed\":false"),
        "{stream}"
    );
    assert!(stream.contains("data: [DONE]"), "{stream}");
    assert!(!stream.contains("event: delta"), "{stream}");

    let final_health = service_http_request(&bind, "GET", "/health", None);
    let final_health_body = http_body(&final_health);
    assert!(
        final_health_body.contains("\"active_engine_requests\":0"),
        "{final_health_body}"
    );
    handle.join().unwrap().unwrap();
    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn model_service_openai_chat_completions_stream_emits_chunks() {
    let asset_dir = target_asset_dir("model-service-openai-chat-stream");
    fs::create_dir_all(&asset_dir).unwrap();
    let bind = reserve_loopback_addr();
    let args = Args::parse(vec![
        "--serve-bind".to_owned(),
        bind.clone(),
        "--serve-max-requests".to_owned(),
        "2".to_owned(),
        "--memory".to_owned(),
        asset_dir.join("memory.ndkv").display().to_string(),
        "--experience".to_owned(),
        asset_dir.join("experience.ndkv").display().to_string(),
        "--adaptive".to_owned(),
        asset_dir.join("adaptive.ndkv").display().to_string(),
        "service openai chat stream prompt".to_owned(),
    ]);
    let calls = Arc::new(Mutex::new(Vec::new()));
    let service_backend = RecordingBackend::new(Arc::clone(&calls));
    let service_args = args.clone();
    let handle = thread::spawn(move || {
        let mut engine = NoironEngine::new();
        configure_engine(&mut engine, &service_args);
        let mut backend = service_backend;
        run_model_service_for_args(&mut engine, &mut backend, &service_args)
    });

    let health = wait_for_http_response(&bind, "GET", "/health", None);
    assert!(http_body(&health).contains("\"ok\":true"));

    let stream = service_http_request(
        &bind,
        "POST",
        "/v1/chat/completions",
        Some(
            "{\"model\":\"rust-norion-local\",\"messages\":[{\"role\":\"user\",\"content\":\"stream please\"}],\"stream\":true,\"tenant_id\":\"tenant-stream\",\"workspace_id\":\"workspace\",\"session_id\":\"stream-1\",\"max_tokens\":8}",
        ),
    );
    handle.join().unwrap().unwrap();

    assert!(
        stream.contains("content-type: text/event-stream"),
        "{stream}"
    );
    assert!(
        stream.contains("data: {\"id\":\"chatcmpl-norion-"),
        "{stream}"
    );
    assert!(
        stream.contains("\"object\":\"chat.completion.chunk\""),
        "{stream}"
    );
    assert!(
        stream.contains("\"model\":\"rust-norion-local\""),
        "{stream}"
    );
    assert!(
        stream.contains("\"delta\":{\"role\":\"assistant\",\"content\":\"partial \"}"),
        "{stream}"
    );
    assert!(stream.contains("\"finish_reason\":\"stop\""), "{stream}");
    assert!(
        stream.contains("\"stream_state\":\"completed\""),
        "{stream}"
    );
    assert!(
        stream.contains("\"endpoint\":\"chat-completions-stream\""),
        "{stream}"
    );
    assert!(
        stream.contains("\"model\":\"rust-norion-local\""),
        "{stream}"
    );
    assert!(stream.contains("\"cancelled\":false"), "{stream}");
    assert!(stream.contains("\"timeout\":false"), "{stream}");
    assert!(stream.contains("\"retryable\":false"), "{stream}");
    assert!(stream.contains("\"runtime_error_note\":null"), "{stream}");
    assert!(stream.contains("\"elapsed_ms\":"), "{stream}");
    assert!(stream.contains("\"language_mode\":\"english\""), "{stream}");
    assert!(stream.contains("\"coding_language\":\"none\""), "{stream}");
    assert!(stream.contains("\"rust_coding\":false"), "{stream}");
    assert!(stream.contains("\"task_mode\":\"low_budget\""), "{stream}");
    assert!(stream.contains("\"task_language\":\"english\""), "{stream}");
    assert!(stream.contains("\"compute_budget\":\"low\""), "{stream}");
    assert!(stream.contains("\"runtime_model\":"), "{stream}");
    assert!(stream.contains("\"runtime_entropy_count\":"), "{stream}");
    assert!(stream.contains("\"runtime_logprob_count\":"), "{stream}");
    assert!(
        stream.contains("\"runtime_uncertainty_token_count\":"),
        "{stream}"
    );
    assert!(
        stream.contains("\"runtime_uncertainty_signal\":"),
        "{stream}"
    );
    assert!(
        stream.contains("\"runtime_device_execution_source\":"),
        "{stream}"
    );
    assert!(stream.contains("\"persistent_writes\":true"), "{stream}");
    assert!(stream.contains("\"memory_write_allowed\":true"), "{stream}");
    assert!(stream.contains("\"genome_write_allowed\":true"), "{stream}");
    assert!(
        stream.contains("\"self_evolution_write_allowed\":true"),
        "{stream}"
    );
    assert!(stream.contains("data: [DONE]"), "{stream}");
    assert!(!stream.contains("event: delta"), "{stream}");
    let calls = calls.lock().unwrap();
    assert_eq!(calls.len(), 1, "{calls:?}");
    assert!(calls[0].prompt.contains("stream please"), "{calls:?}");
    assert_eq!(calls[0].max_tokens, Some(8), "{calls:?}");
    assert_eq!(
        calls[0].tenant_scope,
        Some(rust_norion::TenantScope::new(
            "tenant-stream",
            "workspace",
            "stream-1"
        )),
        "{calls:?}"
    );

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn model_service_stream_backpressure_rejects_queue_overflow() {
    let asset_dir = target_asset_dir("model-service-stream-backpressure");
    fs::create_dir_all(&asset_dir).unwrap();
    let bind = reserve_loopback_addr();
    let args = Args::parse(vec![
        "--serve-bind".to_owned(),
        bind.clone(),
        "--serve-max-requests".to_owned(),
        "9".to_owned(),
        "--memory".to_owned(),
        asset_dir.join("memory.ndkv").display().to_string(),
        "--experience".to_owned(),
        asset_dir.join("experience.ndkv").display().to_string(),
        "--adaptive".to_owned(),
        asset_dir.join("adaptive.ndkv").display().to_string(),
        "service stream backpressure prompt".to_owned(),
    ]);
    let started = Arc::new(AtomicBool::new(false));
    let release = Arc::new(AtomicBool::new(false));
    let service_backend = BlockingBackend {
        started: Arc::clone(&started),
        release: Arc::clone(&release),
    };
    let service_args = args.clone();
    let handle = thread::spawn(move || {
        let mut engine = NoironEngine::new();
        configure_engine(&mut engine, &service_args);
        let mut backend = service_backend;
        run_model_service_for_args(&mut engine, &mut backend, &service_args)
    });

    let first_health = wait_for_http_response(&bind, "GET", "/health", None);
    assert!(http_body(&first_health).contains("\"ok\":true"));

    let mut stream_handles = Vec::new();
    for index in 0..4 {
        let stream_bind = bind.clone();
        stream_handles.push(thread::spawn(move || {
            service_http_request(
                &stream_bind,
                "POST",
                "/v1/generate-stream",
                Some(&format!(
                    "{{\"prompt\":\"hold stream {index}\",\"profile\":\"coding\"}}"
                )),
            )
        }));
    }
    for _ in 0..100 {
        if started.load(Ordering::SeqCst) {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    assert!(
        started.load(Ordering::SeqCst),
        "first stream request should enter the blocking backend"
    );
    thread::sleep(Duration::from_millis(100));
    let active_health = service_http_request(&bind, "GET", "/health", None);
    assert!(
        http_body(&active_health).contains("\"active_engine_requests\":4"),
        "{active_health}"
    );

    let overflow = service_http_request(
        &bind,
        "POST",
        "/v1/generate-stream",
        Some("{\"prompt\":\"overflow stream\",\"profile\":\"coding\"}"),
    );
    let overflow_body = http_body(&overflow);
    assert!(overflow.starts_with("HTTP/1.1 429"), "{overflow}");
    assert!(
        overflow_body.contains("\"error\":\"model service backpressure: active_engine_requests=4 max_active_engine_requests=4\""),
        "{overflow_body}"
    );
    assert!(
        overflow_body.contains("\"retryable\":true"),
        "{overflow_body}"
    );
    assert!(
        overflow_body.contains("\"persistent_writes\":false"),
        "{overflow_body}"
    );
    let backpressure_health = service_http_request(&bind, "GET", "/health", None);
    assert!(
        http_body(&backpressure_health).contains("\"stream_backpressure_rejections\":1"),
        "{backpressure_health}"
    );

    release.store(true, Ordering::SeqCst);
    for stream_handle in stream_handles {
        let response = stream_handle.join().unwrap();
        assert!(
            response.contains("event: done") || http_body(&response).contains("\"ok\":true"),
            "{response}"
        );
        assert!(
            response.contains("\"stream_state\":\"completed\""),
            "{response}"
        );
        assert!(response.contains("\"queue_time_ms\":0"), "{response}");
        assert!(
            response.contains("\"compute_budget_summary\":"),
            "{response}"
        );
        for field in [
            "\"compute_budget_saved_tokens\":",
            "\"compute_budget_avoided_tokens\":",
            "\"compute_budget_kv_lookups_skipped\":",
            "\"compute_budget_fanout_reduction\":",
            "\"compute_budget_read_only\":true",
            "\"compute_budget_write_allowed\":false",
            "\"compute_budget_applied\":false",
            "\"retryable\":false",
            "\"runtime_error_note\":null",
            "\"elapsed_ms\":",
            "\"quality\":",
            "\"process_reward\":",
            "\"stored_runtime_kv_memory_ids\":[",
            "\"runtime_entropy_count\":",
            "\"runtime_logprob_count\":",
            "\"runtime_uncertainty_token_count\":",
            "\"runtime_average_entropy\":",
            "\"runtime_architecture_signal\":",
            "\"runtime_kv_precision_signal\":",
            "\"runtime_device_execution_source\":",
            "\"persistent_writes\":true",
            "\"memory_write_allowed\":true",
            "\"genome_write_allowed\":true",
            "\"self_evolution_write_allowed\":true",
        ] {
            assert!(response.contains(field), "{response}");
        }
    }
    let final_health = service_http_request(&bind, "GET", "/health", None);
    assert!(
        http_body(&final_health).contains("\"active_engine_requests\":0"),
        "{final_health}"
    );
    handle.join().unwrap().unwrap();
    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn model_service_experience_hygiene_quarantine_is_explicit_apply_only() {
    let asset_dir = target_asset_dir("model-service-experience-hygiene");
    fs::create_dir_all(&asset_dir).unwrap();
    let memory = asset_dir.join("memory.ndkv");
    let experience = asset_dir.join("experience.ndkv");
    let adaptive = asset_dir.join("adaptive.ndkv");
    let backup = asset_dir.join("experience.backup.ndkv");
    let quarantine = asset_dir.join("experience.quarantine.ndkv");
    write_dirty_experience_store(&experience);

    let bind = reserve_loopback_addr();
    let args = Args::parse(vec![
        "--serve-bind".to_owned(),
        bind.clone(),
        "--serve-max-requests".to_owned(),
        "4".to_owned(),
        "--memory".to_owned(),
        memory.display().to_string(),
        "--experience".to_owned(),
        experience.display().to_string(),
        "--adaptive".to_owned(),
        adaptive.display().to_string(),
        "experience hygiene service prompt".to_owned(),
    ]);
    let service_args = args.clone();
    let handle = thread::spawn(move || {
        let mut engine = NoironEngine::new();
        configure_engine(&mut engine, &service_args);
        let mut backend = HeuristicBackend;
        run_model_service_for_args(&mut engine, &mut backend, &service_args)
    });

    let report = wait_for_http_response(&bind, "GET", "/v1/experience-hygiene", None);
    let dry_run = service_http_request(
        &bind,
        "POST",
        "/v1/experience-hygiene/quarantine",
        Some("{\"limit\":10}"),
    );
    let after_dry_run = rust_norion::ExperienceStore::load_from_disk_kv(&experience).unwrap();
    assert_eq!(after_dry_run.len(), 2);
    assert!(!backup.exists());
    assert!(!quarantine.exists());
    let apply_body = format!(
        "{{\"apply\":true,\"limit\":10,\"backup_path\":{},\"quarantine_path\":{}}}",
        service_json_string(&backup.display().to_string()),
        service_json_string(&quarantine.display().to_string())
    );
    let apply = service_http_request(
        &bind,
        "POST",
        "/v1/experience-hygiene/quarantine",
        Some(&apply_body),
    );
    let clean_report = service_http_request(&bind, "GET", "/v1/experience-hygiene", None);
    handle.join().unwrap().unwrap();

    let report_body = http_body(&report);
    assert!(report_body.contains("\"checked\":true"), "{report_body}");
    assert!(
        report_body.contains("\"quarantine_candidates\":1"),
        "{report_body}"
    );
    assert!(
        report_body.contains("\"candidate_ids\":[1]"),
        "{report_body}"
    );

    let dry_run_body = http_body(&dry_run);
    assert!(dry_run_body.contains("\"applied\":false"), "{dry_run_body}");

    let apply_body = http_body(&apply);
    assert!(apply_body.contains("\"applied\":true"), "{apply_body}");
    assert!(backup.exists());
    assert!(quarantine.exists());
    let retained = rust_norion::ExperienceStore::load_from_disk_kv(&experience).unwrap();
    let quarantined = rust_norion::ExperienceStore::load_from_disk_kv(&quarantine).unwrap();
    assert_eq!(retained.len(), 1);
    assert_eq!(quarantined.len(), 1);

    let clean_report_body = http_body(&clean_report);
    assert!(
        clean_report_body.contains("\"quarantine_candidates\":0"),
        "{clean_report_body}"
    );
    assert!(
        clean_report_body.contains("\"clean\":true"),
        "{clean_report_body}"
    );

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn model_service_experience_cleanup_audit_is_read_only() {
    let asset_dir = target_asset_dir("model-service-experience-cleanup-audit");
    fs::create_dir_all(&asset_dir).unwrap();
    let bind = reserve_loopback_addr();
    let experience = asset_dir.join("experience.ndkv");
    write_dirty_experience_store(&experience);
    let before = rust_norion::ExperienceStore::load_from_disk_kv(&experience)
        .unwrap()
        .len();
    let args = Args::parse(vec![
        "--serve-bind".to_owned(),
        bind.clone(),
        "--serve-max-requests".to_owned(),
        "1".to_owned(),
        "--memory".to_owned(),
        asset_dir.join("memory.ndkv").display().to_string(),
        "--experience".to_owned(),
        experience.display().to_string(),
        "--adaptive".to_owned(),
        asset_dir.join("adaptive.ndkv").display().to_string(),
        "experience cleanup audit service prompt".to_owned(),
    ]);
    let service_args = args.clone();
    let handle = thread::spawn(move || {
        let mut engine = NoironEngine::new();
        configure_engine(&mut engine, &service_args);
        let mut backend = HeuristicBackend;
        run_model_service_for_args(&mut engine, &mut backend, &service_args)
    });

    let response = wait_for_http_response(
        &bind,
        "POST",
        "/v1/experience-cleanup-audit",
        Some("{\"limit\":7}"),
    );
    handle.join().unwrap().unwrap();

    let after = rust_norion::ExperienceStore::load_from_disk_kv(&experience)
        .unwrap()
        .len();
    let body = http_body(&response);
    assert!(body.contains("\"ok\":true"), "{body}");
    assert!(body.contains("\"writes_experience_state\":false"), "{body}");
    assert!(body.contains("\"sample_limit\":7"), "{body}");
    assert!(body.contains("\"report\":{"), "{body}");
    assert!(body.contains("\"index_report\":{"), "{body}");
    assert!(body.contains("\"quarantine_plan\":{"), "{body}");
    assert!(body.contains("\"repair_plan\":{"), "{body}");
    assert!(body.contains("\"quarantine_candidates\":1"), "{body}");
    assert_eq!(before, after);
    assert!(!asset_dir.join("experience.backup.ndkv").exists());
    assert!(!asset_dir.join("experience.quarantine.ndkv").exists());

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn model_service_experience_cleanup_audit_defers_large_store() {
    let asset_dir = target_asset_dir("model-service-experience-cleanup-audit-large-store");
    fs::create_dir_all(&asset_dir).unwrap();
    let bind = reserve_loopback_addr();
    let experience = asset_dir.join("experience.ndkv");
    fs::write(&experience, vec![b'x'; 1_000_001]).unwrap();
    let args = Args::parse(vec![
        "--serve-bind".to_owned(),
        bind.clone(),
        "--serve-max-requests".to_owned(),
        "1".to_owned(),
        "--memory".to_owned(),
        asset_dir.join("memory.ndkv").display().to_string(),
        "--experience".to_owned(),
        experience.display().to_string(),
        "--adaptive".to_owned(),
        asset_dir.join("adaptive.ndkv").display().to_string(),
        "experience cleanup audit large store prompt".to_owned(),
    ]);
    let service_args = args.clone();
    let handle = thread::spawn(move || {
        let mut engine = NoironEngine::new();
        configure_engine(&mut engine, &service_args);
        let mut backend = HeuristicBackend;
        run_model_service_for_args(&mut engine, &mut backend, &service_args)
    });

    let response = wait_for_http_response(
        &bind,
        "POST",
        "/v1/experience-cleanup-audit",
        Some("{\"limit\":7}"),
    );
    handle.join().unwrap().unwrap();

    let body = http_body(&response);
    assert!(body.contains("\"ok\":true"), "{body}");
    assert!(body.contains("\"checked\":false"), "{body}");
    assert!(
        body.contains("experience_hygiene_deferred_large_file"),
        "{body}"
    );
    assert!(body.contains("\"writes_experience_state\":false"), "{body}");

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn model_service_experience_retrieval_previews_matches_without_generation() {
    let asset_dir = target_asset_dir("model-service-experience-retrieval");
    fs::create_dir_all(&asset_dir).unwrap();
    let bind = reserve_loopback_addr();
    let args = Args::parse(vec![
        "--serve-bind".to_owned(),
        bind.clone(),
        "--serve-max-requests".to_owned(),
        "1".to_owned(),
        "--memory".to_owned(),
        asset_dir.join("memory.ndkv").display().to_string(),
        "--experience".to_owned(),
        asset_dir.join("experience.ndkv").display().to_string(),
        "--adaptive".to_owned(),
        asset_dir.join("adaptive.ndkv").display().to_string(),
        "experience retrieval service prompt".to_owned(),
    ]);
    let service_args = args.clone();
    let handle = thread::spawn(move || {
        let mut engine = NoironEngine::new();
        engine.experience.record(experience_input(
            "Conversation transcript:\nuser: 帮我用rust输出一段for循环代码\nassistant: Rust loop\nuser: Bash command\nssh -o ConnectTimeout=8 gitlab.local merge_requests",
            "polluted shell transcript should be skipped",
            0.99,
        ));
        engine.experience.record(experience_input(
            "Rust for loop examples",
            "show a clean Rust range loop with for i in 0..10",
            0.82,
        ));
        configure_engine(&mut engine, &service_args);
        let mut backend = HeuristicBackend;
        run_model_service_for_args(&mut engine, &mut backend, &service_args)
    });

    let response = wait_for_http_response(
        &bind,
        "POST",
        "/v1/experience-retrieval",
        Some(
            "{\"prompt\":\"帮我用rust输出一段for循环代码\",\"profile\":\"coding\",\"limit\":5,\"index_context\":\"model_pool_index: src/experience contains clean Rust loop examples\"}",
        ),
    );
    let body = http_body(&response);

    assert!(body.contains("\"ok\":true"), "{body}");
    assert!(
        body.contains("\"prompt\":\"帮我用rust输出一段for循环代码\""),
        "{body}"
    );
    assert!(body.contains("\"index_context_used\":true"), "{body}");
    assert!(body.contains("\"index_context_chars\":66"), "{body}");
    assert!(body.contains("\"total_records\":2"), "{body}");
    assert!(body.contains("\"match_count\":1"), "{body}");
    assert!(
        body.contains("\"skipped_cross_task_pollution\":1"),
        "{body}"
    );
    assert!(
        body.contains("\"lesson_preview\":\"show a clean Rust range loop"),
        "{body}"
    );
    assert!(!body.contains("gitlab.local"), "{body}");

    handle.join().unwrap().unwrap();
    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn model_service_experience_retrieval_respects_tenant_scope() {
    let asset_dir = target_asset_dir("model-service-experience-retrieval-scope");
    fs::create_dir_all(&asset_dir).unwrap();
    let bind = reserve_loopback_addr();
    let args = Args::parse(vec![
        "--serve-bind".to_owned(),
        bind.clone(),
        "--serve-max-requests".to_owned(),
        "1".to_owned(),
        "--memory".to_owned(),
        asset_dir.join("memory.ndkv").display().to_string(),
        "--experience".to_owned(),
        asset_dir.join("experience.ndkv").display().to_string(),
        "--adaptive".to_owned(),
        asset_dir.join("adaptive.ndkv").display().to_string(),
        "scoped experience retrieval service prompt".to_owned(),
    ]);
    let service_args = args.clone();
    let handle = thread::spawn(move || {
        let mut engine = NoironEngine::new();
        let tenant_a = rust_norion::TenantScope::new("tenant-a", "workspace", "retrieval-1");
        let tenant_b = rust_norion::TenantScope::new("tenant-b", "workspace", "retrieval-1");
        let memory_a = engine.cache.store_scoped_or_fuse(
            &tenant_a,
            rust_norion::TenantResourceLane::KvMemory,
            "rust-loop-a",
            vec![1.0, 0.0],
            0.82,
        );
        let memory_b = engine.cache.store_scoped_or_fuse(
            &tenant_b,
            rust_norion::TenantResourceLane::KvMemory,
            "rust-loop-b",
            vec![1.0, 0.0],
            0.82,
        );
        let mut tenant_a_input = experience_input(
            "Rust loop scoped tenant A",
            "tenant-a scoped Rust loop lesson",
            0.82,
        );
        tenant_a_input.stored_memory_id = Some(memory_a);
        let mut tenant_b_input = experience_input(
            "Rust loop scoped tenant B",
            "tenant-b scoped Rust loop lesson should stay hidden",
            0.82,
        );
        tenant_b_input.stored_memory_id = Some(memory_b);
        engine.experience.record(tenant_a_input);
        engine.experience.record(tenant_b_input);
        configure_engine(&mut engine, &service_args);
        let mut backend = HeuristicBackend;
        run_model_service_for_args(&mut engine, &mut backend, &service_args)
    });

    let response = wait_for_http_response(
        &bind,
        "POST",
        "/v1/experience-retrieval",
        Some(
            "{\"prompt\":\"Rust loop scoped\",\"profile\":\"coding\",\"limit\":5,\"tenant_id\":\"tenant-a\",\"workspace_id\":\"workspace\",\"session_id\":\"retrieval-1\"}",
        ),
    );
    let body = http_body(&response);

    assert!(body.contains("\"ok\":true"), "{body}");
    assert!(body.contains("\"total_records\":1"), "{body}");
    assert!(body.contains("\"match_count\":1"), "{body}");
    assert!(body.contains("tenant-a scoped Rust loop lesson"), "{body}");
    assert!(!body.contains("tenant-b scoped Rust loop lesson"), "{body}");

    handle.join().unwrap().unwrap();
    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn model_service_health_reports_gemma_runtime_reachability() {
    let asset_dir = target_asset_dir("model-service-gemma-runtime-reachability");
    fs::create_dir_all(&asset_dir).unwrap();
    let bind = reserve_loopback_addr();
    let runtime_listener = TcpListener::bind("127.0.0.1:0").unwrap();
    runtime_listener.set_nonblocking(true).unwrap();
    let runtime_addr = runtime_listener.local_addr().unwrap();
    let args = Args::parse(vec![
        "--serve-bind".to_owned(),
        bind.clone(),
        "--serve-max-requests".to_owned(),
        "2".to_owned(),
        "--gemma-runtime-server".to_owned(),
        format!("http://{runtime_addr}"),
        "--memory".to_owned(),
        asset_dir.join("memory.ndkv").display().to_string(),
        "--experience".to_owned(),
        asset_dir.join("experience.ndkv").display().to_string(),
        "--adaptive".to_owned(),
        asset_dir.join("adaptive.ndkv").display().to_string(),
        "service gemma runtime reachability prompt".to_owned(),
    ]);
    let service_args = args.clone();
    let handle = thread::spawn(move || {
        let mut engine = NoironEngine::new();
        configure_engine(&mut engine, &service_args);
        let mut backend = HeuristicBackend;
        run_model_service_for_args(&mut engine, &mut backend, &service_args)
    });

    let reachable_health = wait_for_http_response(&bind, "GET", "/health", None);
    let reachable_body = http_body(&reachable_health);
    assert!(
        reachable_body.contains("\"runtime_mode\":\"gemma-http\""),
        "{reachable_body}"
    );
    assert!(
        reachable_body.contains("\"gemma_runtime_reachable\":true"),
        "{reachable_body}"
    );
    drop(runtime_listener);

    let unreachable_health = service_http_request(&bind, "GET", "/health", None);
    let unreachable_body = http_body(&unreachable_health);
    assert!(
        unreachable_body.contains("\"gemma_runtime_reachable\":false"),
        "{unreachable_body}"
    );

    handle.join().unwrap().unwrap();
    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn model_service_runs_generate_replay_and_inspect_http_smoke() {
    let asset_dir = target_asset_dir("model-service-http-smoke");
    fs::create_dir_all(&asset_dir).unwrap();
    let memory = asset_dir.join("memory.ndkv");
    let experience = asset_dir.join("experience.ndkv");
    let adaptive = asset_dir.join("adaptive.ndkv");
    let trace = asset_dir.join("trace.jsonl");
    let bind = reserve_loopback_addr();
    let args = Args::parse(vec![
        "--serve-bind".to_owned(),
        bind.clone(),
        "--serve-max-requests".to_owned(),
        "10".to_owned(),
        "--memory".to_owned(),
        memory.display().to_string(),
        "--experience".to_owned(),
        experience.display().to_string(),
        "--adaptive".to_owned(),
        adaptive.display().to_string(),
        "--trace".to_owned(),
        trace.display().to_string(),
        "--trace-schema-gate".to_owned(),
        trace.display().to_string(),
        "--inspect-min-memories".to_owned(),
        "1".to_owned(),
        "--inspect-min-experiences".to_owned(),
        "1".to_owned(),
        "--inspect-min-evolution-live-inference-runs".to_owned(),
        "1".to_owned(),
        "--inspect-min-evolution-replay-runs".to_owned(),
        "1".to_owned(),
        "--inspect-min-evolution-replay-items".to_owned(),
        "1".to_owned(),
        "--inspect-min-evolution-external-feedbacks".to_owned(),
        "1".to_owned(),
        "--inspect-min-evolution-external-feedback-memory-updates".to_owned(),
        "1".to_owned(),
        "--inspect-min-evolution-external-feedback-strength-delta".to_owned(),
        "0.08".to_owned(),
        "service smoke prompt".to_owned(),
    ]);
    let service_args = args.clone();
    let handle = thread::spawn(move || {
        let mut engine = NoironEngine::new();
        configure_engine(&mut engine, &service_args);
        let mut backend = HeuristicBackend;
        run_model_service_for_args(&mut engine, &mut backend, &service_args)
    });

    let health = wait_for_http_response(&bind, "GET", "/health", None);
    let generate_info = service_http_request(&bind, "GET", "/v1/generate", None);
    let generate_body = "{\"prompt\":\"用中文解释 Rust 所有权，并给出一个 rust-norion 业务联调测试建议。\",\"profile\":\"coding\",\"case\":\"service-http-smoke\"}";
    let generate = service_http_request(&bind, "POST", "/v1/generate", Some(generate_body));
    let generate_json = http_body(&generate).to_owned();
    let experience_id = json_u64_field(&generate_json, "experience_id")
        .expect("generate response must expose experience_id");
    let feedback_memory_ids = json_u64_array_field(&generate_json, "feedback_memory_ids")
        .expect("generate response must expose feedback_memory_ids");
    assert!(
        !feedback_memory_ids.is_empty(),
        "generate response must include at least one feedback memory id: {generate_json}"
    );
    let feedback_request = format!(
        "{{\"experience_id\":{},\"action\":\"reinforce\",\"amount\":0.5}}",
        experience_id
    );
    let feedback = service_http_request(&bind, "POST", "/v1/feedback", Some(&feedback_request));
    let chat_body = "{\"messages\":[{\"role\":\"system\",\"content\":\"你是 rust-norion 的本地模型服务。\"},{\"role\":\"user\",\"content\":\"继续用中文给一个业务联调建议。\"}],\"profile\":\"coding\",\"case\":\"chat-http-smoke\"}";
    let chat = service_http_request(&bind, "POST", "/v1/chat", Some(chat_body));
    let openai_chat_body = "{\"model\":\"rust-norion-local\",\"messages\":[{\"role\":\"user\",\"content\":\"继续用中文给一个 OpenAI 兼容业务联调建议。\"}],\"max_tokens\":64}";
    let openai_chat = service_http_request(
        &bind,
        "POST",
        "/v1/chat/completions",
        Some(openai_chat_body),
    );
    let completion_info = service_http_request(&bind, "GET", "/v1/completions", None);
    let openai_completion_body = "{\"model\":\"rust-norion-local\",\"prompt\":\"继续用中文给一个 OpenAI completion 兼容业务联调建议。\",\"max_tokens\":64}";
    let openai_completion = service_http_request(
        &bind,
        "POST",
        "/v1/completions",
        Some(openai_completion_body),
    );
    let replay = service_http_request(&bind, "POST", "/v1/replay", Some("{\"limit\":1}"));
    let inspect = service_http_request(&bind, "POST", "/v1/inspect", Some("{\"trace_gate\":true}"));
    handle.join().unwrap().unwrap();

    let health_body = http_body(&health);
    let generate_info_body = http_body(&generate_info);
    let generate_body = http_body(&generate);
    let chat_body = http_body(&chat);
    let openai_chat_body = http_body(&openai_chat);
    let completion_info_body = http_body(&completion_info);
    let openai_completion_body = http_body(&openai_completion);
    let feedback_body = http_body(&feedback);
    let replay_body = http_body(&replay);
    let inspect_body = http_body(&inspect);

    assert!(health_body.contains("\"ok\":true"));
    assert!(generate_info_body.contains("\"endpoint\":\"/v1/generate\""));
    assert!(generate_info_body.contains("\"method\":\"POST\""));
    assert_json_string_fields(
        generate_info_body,
        &[
            "elapsed_ms",
            "output_mode",
            "quality",
            "process_reward",
            "action",
            "memory_stored",
            "stored_memory_id",
            "used_memory_ids",
            "stored_gist_memory_ids",
            "stored_runtime_kv_memory_ids",
            "feedback_memory_ids",
            "runtime_entropy_count",
            "runtime_logprob_count",
            "runtime_uncertainty_token_count",
            "runtime_average_entropy",
            "runtime_average_neg_logprob",
            "runtime_uncertainty_perplexity",
            "runtime_architecture_signal",
            "runtime_kv_precision_signal",
            "runtime_device_execution_source",
        ],
    );
    assert!(generate_body.contains("\"ok\":true"));
    assert!(generate_body.contains("\"profile\":\"coding\""));
    assert!(generate_body.contains("\"language_mode\":\"chinese\""));
    assert!(generate_body.contains("\"coding_language\":\"rust\""));
    assert!(generate_body.contains("\"rust_coding\":true"));
    assert!(generate_body.contains("\"task_mode\":\"rust_coding\""));
    assert!(generate_body.contains("\"task_language\":\"mixed\""));
    assert!(generate_body.contains("\"coding_intent\":true"));
    assert!(generate_body.contains("\"validation_mode\":"));
    assert!(generate_body.contains("\"memory_need\":"));
    assert!(generate_body.contains("\"compute_budget\":"));
    assert!(generate_body.contains("\"compute_budget_summary\":"));
    assert!(generate_body.contains("\"compute_budget_saved_tokens\":"));
    assert!(generate_body.contains("\"compute_budget_avoided_tokens\":"));
    assert!(generate_body.contains("\"compute_budget_kv_lookups_skipped\":"));
    assert!(generate_body.contains("\"compute_budget_fanout_reduction\":"));
    assert!(generate_body.contains("\"compute_budget_read_only\":true"));
    assert!(generate_body.contains("\"compute_budget_write_allowed\":false"));
    assert!(generate_body.contains("\"compute_budget_applied\":false"));
    assert!(generate_body.contains("\"traceable\":true"));
    assert!(generate_body.contains("\"stored_memory_id\":"));
    assert!(generate_body.contains("\"used_memory_ids\":["));
    assert!(generate_body.contains("\"stored_gist_memory_ids\":["));
    assert!(generate_body.contains("\"stored_runtime_kv_memory_ids\":["));
    assert!(generate_body.contains("\"feedback_memory_ids\":["));
    assert!(generate_body.contains("\"runtime_token_count\":"));
    assert!(generate_body.contains("\"runtime_entropy_count\":"));
    assert!(generate_body.contains("\"runtime_logprob_count\":"));
    assert!(generate_body.contains("\"runtime_uncertainty_token_count\":"));
    assert!(generate_body.contains("\"runtime_uncertainty_signal\":"));
    assert!(json_u64_field(generate_body, "runtime_token_count").is_some());
    assert!(json_u64_field(generate_body, "runtime_uncertainty_token_count").is_some());
    assert!(json_bool_field(generate_body, "runtime_uncertainty_signal").is_some());
    assert!(chat_body.contains("\"ok\":true"));
    assert!(chat_body.contains("\"profile\":\"coding\""));
    assert!(chat_body.contains("\"language_mode\":\"chinese\""));
    assert!(chat_body.contains("\"coding_language\":\"rust\""));
    assert!(chat_body.contains("\"task_mode\":\"rust_coding\""));
    assert!(chat_body.contains("\"traceable\":true"));
    assert!(chat_body.contains("\"experience_id\":"));
    for (body, endpoint) in [(generate_body, "generate"), (chat_body, "chat")] {
        assert!(
            body.contains(&format!("\"endpoint\":\"{endpoint}\"")),
            "{body}"
        );
        assert!(body.contains("\"error\":null"), "{body}");
        assert!(body.contains("\"error_type\":null"), "{body}");
        assert!(body.contains("\"cancelled\":false"), "{body}");
        assert!(body.contains("\"timeout\":false"), "{body}");
        assert!(body.contains("\"retryable\":false"), "{body}");
        assert!(body.contains("\"runtime_error_note\":null"), "{body}");
        assert!(body.contains("\"persistent_writes\":true"), "{body}");
        assert!(body.contains("\"memory_write_allowed\":true"), "{body}");
        assert!(body.contains("\"genome_write_allowed\":true"), "{body}");
        assert!(
            body.contains("\"self_evolution_write_allowed\":true"),
            "{body}"
        );
    }
    assert!(openai_chat_body.contains("\"object\":\"chat.completion\""));
    assert!(openai_chat_body.contains("\"model\":\"rust-norion-local\""));
    assert!(openai_chat_body.contains("\"choices\":[{"));
    assert!(openai_chat_body.contains("\"message\":{\"role\":\"assistant\""));
    assert!(openai_chat_body.contains("\"usage\":{\"prompt_tokens\":0"));
    assert!(openai_chat_body.contains("\"norion\":{\"request_id\":"));
    assert!(openai_chat_body.contains("\"endpoint\":\"chat-completions\""));
    assert!(openai_chat_body.contains("\"model\":\"rust-norion-local\""));
    assert!(openai_chat_body.contains("\"language_mode\":\"chinese\""));
    assert!(openai_chat_body.contains("\"coding_language\":\"none\""));
    assert!(openai_chat_body.contains("\"rust_coding\":false"));
    assert!(openai_chat_body.contains("\"task_mode\":\"low_budget\""));
    assert!(openai_chat_body.contains("\"task_language\":\"mixed\""));
    assert!(openai_chat_body.contains("\"compute_budget\":\"low\""));
    assert!(openai_chat_body.contains("\"compute_budget_summary\":"));
    assert!(openai_chat_body.contains("\"compute_budget_saved_tokens\":"));
    assert!(openai_chat_body.contains("\"compute_budget_avoided_tokens\":"));
    assert!(openai_chat_body.contains("\"compute_budget_kv_lookups_skipped\":"));
    assert!(openai_chat_body.contains("\"compute_budget_fanout_reduction\":"));
    assert!(openai_chat_body.contains("\"compute_budget_read_only\":true"));
    assert!(openai_chat_body.contains("\"compute_budget_write_allowed\":false"));
    assert!(openai_chat_body.contains("\"compute_budget_applied\":false"));
    assert!(openai_chat_body.contains("\"runtime_model\":"));
    assert!(openai_chat_body.contains("\"runtime_entropy_count\":"));
    assert!(openai_chat_body.contains("\"runtime_logprob_count\":"));
    assert!(openai_chat_body.contains("\"runtime_uncertainty_token_count\":"));
    assert!(openai_chat_body.contains("\"runtime_uncertainty_signal\":"));
    assert!(openai_chat_body.contains("\"runtime_device_execution_source\":"));
    assert!(openai_chat_body.contains("\"cancelled\":false"));
    assert!(openai_chat_body.contains("\"timeout\":false"));
    assert!(openai_chat_body.contains("\"retryable\":false"));
    assert!(openai_chat_body.contains("\"runtime_error_note\":null"));
    assert!(openai_chat_body.contains("\"persistent_writes\":true"));
    assert!(openai_chat_body.contains("\"memory_write_allowed\":true"));
    assert!(openai_chat_body.contains("\"genome_write_allowed\":true"));
    assert!(openai_chat_body.contains("\"self_evolution_write_allowed\":true"));
    assert!(completion_info_body.contains("\"endpoint\":\"/v1/completions\""));
    assert!(
        completion_info_body.contains(
            "\"supported_fields\":[\"model\",\"prompt\",\"max_tokens\",\"n\",\"tenant_id\",\"workspace_id\",\"session_id\"]"
        )
    );
    assert!(completion_info_body.contains("\"norion.runtime_model\""));
    assert!(completion_info_body.contains("\"norion.runtime_uncertainty_signal\""));
    assert!(completion_info_body.contains("\"norion.runtime_device_execution_source\""));
    assert!(completion_info_body.contains("\"norion.language_mode\""));
    assert!(completion_info_body.contains("\"norion.coding_language\""));
    assert!(completion_info_body.contains("\"norion.task_mode\""));
    assert!(openai_completion_body.contains("\"object\":\"text_completion\""));
    assert!(openai_completion_body.contains("\"model\":\"rust-norion-local\""));
    assert!(openai_completion_body.contains("\"choices\":[{"));
    assert!(openai_completion_body.contains("\"text\":"));
    assert!(openai_completion_body.contains("\"usage\":{\"prompt_tokens\":0"));
    assert!(openai_completion_body.contains("\"norion\":{\"request_id\":"));
    assert!(openai_completion_body.contains("\"endpoint\":\"completions\""));
    assert!(openai_completion_body.contains("\"model\":\"rust-norion-local\""));
    assert!(openai_completion_body.contains("\"language_mode\":\"chinese\""));
    assert!(openai_completion_body.contains("\"coding_language\":\"none\""));
    assert!(openai_completion_body.contains("\"task_mode\":\"low_budget\""));
    assert!(openai_completion_body.contains("\"compute_budget\":\"low\""));
    assert!(openai_completion_body.contains("\"compute_budget_summary\":"));
    assert!(openai_completion_body.contains("\"compute_budget_saved_tokens\":"));
    assert!(openai_completion_body.contains("\"compute_budget_avoided_tokens\":"));
    assert!(openai_completion_body.contains("\"compute_budget_kv_lookups_skipped\":"));
    assert!(openai_completion_body.contains("\"compute_budget_fanout_reduction\":"));
    assert!(openai_completion_body.contains("\"compute_budget_read_only\":true"));
    assert!(openai_completion_body.contains("\"compute_budget_write_allowed\":false"));
    assert!(openai_completion_body.contains("\"compute_budget_applied\":false"));
    assert!(openai_completion_body.contains("\"runtime_model\":"));
    assert!(openai_completion_body.contains("\"runtime_entropy_count\":"));
    assert!(openai_completion_body.contains("\"runtime_logprob_count\":"));
    assert!(openai_completion_body.contains("\"runtime_uncertainty_token_count\":"));
    assert!(openai_completion_body.contains("\"runtime_uncertainty_signal\":"));
    assert!(openai_completion_body.contains("\"runtime_device_execution_source\":"));
    assert!(openai_completion_body.contains("\"cancelled\":false"));
    assert!(openai_completion_body.contains("\"timeout\":false"));
    assert!(openai_completion_body.contains("\"retryable\":false"));
    assert!(openai_completion_body.contains("\"runtime_error_note\":null"));
    assert!(openai_completion_body.contains("\"persistent_writes\":true"));
    assert!(openai_completion_body.contains("\"memory_write_allowed\":true"));
    assert!(openai_completion_body.contains("\"genome_write_allowed\":true"));
    assert!(openai_completion_body.contains("\"self_evolution_write_allowed\":true"));
    assert!(feedback_body.contains("\"ok\":true"));
    assert!(feedback_body.contains("\"action\":\"reinforce\""));
    assert!(feedback_body.contains(&format!("\"experience_id\":{experience_id}")));
    assert!(feedback_body.contains(&format!(
        "\"memory_ids\":{}",
        service_u64_array(&feedback_memory_ids)
    )));
    assert_eq!(
        json_u64_field(feedback_body, "applied"),
        Some(feedback_memory_ids.len() as u64)
    );
    assert!(
        json_f32_field(feedback_body, "strength_delta").unwrap_or_default() >= 0.08,
        "{feedback_body}"
    );
    assert!(feedback_body.contains("\"evolution_external_feedbacks\":1"));
    assert!(feedback_body.contains(&format!(
        "\"evolution_external_feedback_memory_updates\":{}",
        feedback_memory_ids.len()
    )));
    assert!(replay_body.contains("\"ok\":true"));
    assert!(replay_body.contains("\"applied\":1"));
    assert!(
        json_u64_field(replay_body, "live_memory_feedback_updates").unwrap_or_default() >= 1,
        "{replay_body}"
    );
    assert!(
        json_u64_field(replay_body, "live_memory_feedback_applied").unwrap_or_default() >= 1,
        "{replay_body}"
    );
    assert!(
        json_u64_field(replay_body, "live_evolution_items").unwrap_or_default() >= 1,
        "{replay_body}"
    );
    assert!(
        inspect_body.contains("\"state_gate\":{\"passed\":true"),
        "{inspect_body}"
    );
    assert!(
        inspect_body.contains("\"trace_gate\":{\"passed\":true"),
        "{inspect_body}"
    );
    let runtime_audit = GemmaModelServiceRuntimeAudit::from_inspect_body(inspect_body);
    assert!(runtime_audit.passed(), "{runtime_audit:?} {inspect_body}");
    assert_eq!(runtime_audit, GemmaModelServiceRuntimeAudit::default());
    assert!(
        json_u64_field(inspect_body, "evolution_live_inference_runs").unwrap_or_default() >= 2,
        "{inspect_body}"
    );
    assert!(
        json_u64_field(inspect_body, "evolution_replay_runs").unwrap_or_default() >= 1,
        "{inspect_body}"
    );
    assert!(inspect_body.contains("\"evolution_external_feedbacks\":1"));
    assert!(inspect_body.contains(&format!(
        "\"evolution_external_feedback_memory_updates\":{}",
        feedback_memory_ids.len()
    )));

    let trace_report = evaluate_trace_schema_jsonl(&trace).unwrap();
    let state_report = run_state_inspection(&args).unwrap();
    assert!(trace_report.passed, "{:?}", trace_report.failures);
    assert!(trace_report.checked_lines >= 2);
    assert!(state_report.experience_count >= 2);
    assert!(state_report.memory_count >= 1);
    assert!(state_report.evolution_ledger.live_inference_runs >= 2);
    assert!(state_report.evolution_ledger.replay_runs >= 1);
    assert_eq!(state_report.evolution_ledger.external_feedbacks, 1);
    assert_eq!(
        state_report
            .evolution_ledger
            .external_feedback_memory_updates,
        feedback_memory_ids.len() as u64
    );

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn model_service_runs_self_improve_http_smoke() {
    let asset_dir = target_asset_dir("model-service-self-improve-smoke");
    fs::create_dir_all(&asset_dir).unwrap();
    let memory = asset_dir.join("memory.ndkv");
    let experience = asset_dir.join("experience.ndkv");
    let adaptive = asset_dir.join("adaptive.ndkv");
    let trace = asset_dir.join("trace.jsonl");
    let bind = reserve_loopback_addr();
    let args = Args::parse(vec![
        "--serve-bind".to_owned(),
        bind.clone(),
        "--serve-max-requests".to_owned(),
        "4".to_owned(),
        "--memory".to_owned(),
        memory.display().to_string(),
        "--experience".to_owned(),
        experience.display().to_string(),
        "--adaptive".to_owned(),
        adaptive.display().to_string(),
        "--trace".to_owned(),
        trace.display().to_string(),
        "--trace-schema-gate".to_owned(),
        trace.display().to_string(),
        "--inspect-min-memories".to_owned(),
        "1".to_owned(),
        "--inspect-min-experiences".to_owned(),
        "1".to_owned(),
        "--inspect-min-evolution-live-inference-runs".to_owned(),
        "1".to_owned(),
        "--inspect-min-evolution-replay-runs".to_owned(),
        "1".to_owned(),
        "--inspect-min-evolution-replay-items".to_owned(),
        "1".to_owned(),
        "--inspect-min-evolution-external-feedbacks".to_owned(),
        "1".to_owned(),
        "--inspect-min-evolution-external-feedback-memory-updates".to_owned(),
        "1".to_owned(),
        "--inspect-min-evolution-external-feedback-strength-delta".to_owned(),
        "0.08".to_owned(),
        "service self improve prompt".to_owned(),
    ]);
    let service_args = args.clone();
    let handle = thread::spawn(move || {
        let mut engine = NoironEngine::new();
        configure_engine(&mut engine, &service_args);
        let mut backend = HeuristicBackend;
        run_model_service_for_args(&mut engine, &mut backend, &service_args)
    });

    let health = wait_for_http_response(&bind, "GET", "/health", None);
    let generate_body = "{\"prompt\":\"用中文给出 rust-norion 本地模型业务联调建议。\",\"profile\":\"coding\",\"case\":\"self-improve-http-smoke\"}";
    let generate = service_http_request(&bind, "POST", "/v1/generate", Some(generate_body));
    let generate_json = http_body(&generate).to_owned();
    let experience_id = json_u64_field(&generate_json, "experience_id")
        .expect("generate response must expose experience_id");
    let feedback_memory_ids = json_u64_array_field(&generate_json, "feedback_memory_ids")
        .expect("generate response must expose feedback_memory_ids");
    assert!(!feedback_memory_ids.is_empty(), "{generate_json}");
    let feedback_request = format!(
        "{{\"experience_id\":{},\"action\":\"reinforce\",\"amount\":0.5}}",
        experience_id
    );
    let feedback = service_http_request(&bind, "POST", "/v1/feedback", Some(&feedback_request));
    let self_improve = service_http_request(
        &bind,
        "POST",
        "/v1/self-improve",
        Some("{\"limit\":1,\"trace_gate\":true}"),
    );
    handle.join().unwrap().unwrap();

    let health_body = http_body(&health);
    let feedback_body = http_body(&feedback);
    let self_improve_body = http_body(&self_improve);

    assert!(health_body.contains("\"ok\":true"));
    assert!(feedback_body.contains("\"ok\":true"), "{feedback_body}");
    assert!(self_improve_body.contains("\"ok\":true"));
    assert!(
        self_improve_body.contains("\"self_improve\":{\"passed\":true"),
        "{self_improve_body}"
    );
    assert!(
        self_improve_body.contains("\"replay_passed\":true"),
        "{self_improve_body}"
    );
    assert!(
        self_improve_body.contains("\"business_cycle_gate\":false"),
        "{self_improve_body}"
    );
    assert!(
        self_improve_body.contains("\"state_gate\":{\"passed\":true"),
        "{self_improve_body}"
    );
    assert!(
        self_improve_body.contains("\"trace_gate\":{\"passed\":true"),
        "{self_improve_body}"
    );
    assert!(
        self_improve_body.contains("\"self_evolution_admission_events\":1"),
        "{self_improve_body}"
    );
    assert!(
        self_improve_body.contains("\"self_evolution_admission_blocked\":1"),
        "{self_improve_body}"
    );
    assert!(
        self_improve_body.contains("\"self_evolution_admission_review_packets\":1"),
        "{self_improve_body}"
    );
    assert!(
        self_improve_body.contains("\"self_evolution_admission_evidence_ids\":2"),
        "{self_improve_body}"
    );
    assert!(
        self_improve_body.contains("\"self_evolution_admission\":{"),
        "{self_improve_body}"
    );
    assert!(
        self_improve_body.contains("\"self_evolution_admission_checked\":true"),
        "{self_improve_body}"
    );
    assert!(
        self_improve_body.contains("\"self_evolution_admission_blocked\":true"),
        "{self_improve_body}"
    );
    assert!(
        self_improve_body
            .contains("\"self_evolution_admission_model_service_benchmark_gate_evidence_missing\""),
        "{self_improve_body}"
    );
    assert!(
        self_improve_body.contains("\"memory_store_write_allowed\":false"),
        "{self_improve_body}"
    );
    assert!(
        self_improve_body.contains("\"ndkv_write_allowed\":false"),
        "{self_improve_body}"
    );
    assert!(
        self_improve_body.contains("\"model_weight_write_allowed\":false"),
        "{self_improve_body}"
    );
    assert!(
        self_improve_body.contains("\"git_write_allowed\":false"),
        "{self_improve_body}"
    );
    assert_eq!(
        json_u64_field(self_improve_body, "replay_applied"),
        Some(1),
        "{self_improve_body}"
    );
    assert!(
        json_u64_field(self_improve_body, "live_memory_feedback_updates").unwrap_or_default() >= 1,
        "{self_improve_body}"
    );
    assert!(
        json_u64_field(self_improve_body, "evolution_replay_runs").unwrap_or_default() >= 1,
        "{self_improve_body}"
    );
    assert!(
        json_u64_field(self_improve_body, "evolution_replay_items").unwrap_or_default() >= 1,
        "{self_improve_body}"
    );

    let trace_report = evaluate_trace_schema_jsonl(&trace).unwrap();
    let state_report = run_state_inspection(&args).unwrap();
    assert!(trace_report.passed, "{:?}", trace_report.failures);
    assert_eq!(trace_report.checked_lines, 2);
    assert_eq!(trace_report.self_evolution_admission_events, 1);
    assert_eq!(trace_report.self_evolution_admission_admitted, 0);
    assert_eq!(trace_report.self_evolution_admission_blocked, 1);
    assert_eq!(trace_report.self_evolution_admission_review_packets, 1);
    assert_eq!(trace_report.self_evolution_admission_evidence_ids, 2);
    assert_eq!(
        trace_report.self_evolution_admission_missing_review_packet_refs,
        0
    );
    assert_eq!(state_report.evolution_ledger.replay_runs, 1);
    assert!(state_report.evolution_ledger.replay_items >= 1);

    fs::remove_dir_all(asset_dir).unwrap();
}

#[test]
fn model_service_business_cycle_runs_feedback_rust_check_and_self_improve() {
    let asset_dir = target_asset_dir("model-service-business-cycle-smoke");
    fs::create_dir_all(&asset_dir).unwrap();
    let memory = asset_dir.join("memory.ndkv");
    let experience = asset_dir.join("experience.ndkv");
    let adaptive = asset_dir.join("adaptive.ndkv");
    let trace = asset_dir.join("trace.jsonl");
    let bind = reserve_loopback_addr();
    let args = Args::parse(vec![
        "--serve-bind".to_owned(),
        bind.clone(),
        "--serve-max-requests".to_owned(),
        "2".to_owned(),
        "--memory".to_owned(),
        memory.display().to_string(),
        "--experience".to_owned(),
        experience.display().to_string(),
        "--adaptive".to_owned(),
        adaptive.display().to_string(),
        "--trace".to_owned(),
        trace.display().to_string(),
        "--trace-schema-gate".to_owned(),
        trace.display().to_string(),
        "--inspect-min-memories".to_owned(),
        "1".to_owned(),
        "--inspect-min-experiences".to_owned(),
        "1".to_owned(),
        "--inspect-min-evolution-live-inference-runs".to_owned(),
        "1".to_owned(),
        "--inspect-min-evolution-replay-runs".to_owned(),
        "1".to_owned(),
        "--inspect-min-evolution-replay-items".to_owned(),
        "1".to_owned(),
        "--inspect-min-evolution-external-feedbacks".to_owned(),
        "2".to_owned(),
        "--inspect-min-evolution-external-feedback-memory-updates".to_owned(),
        "2".to_owned(),
        "--inspect-min-evolution-external-feedback-strength-delta".to_owned(),
        "0.08".to_owned(),
        "--inspect-min-rust-check-experiences".to_owned(),
        "1".to_owned(),
        "--inspect-min-rust-check-passed".to_owned(),
        "1".to_owned(),
        "--inspect-max-rust-check-failed".to_owned(),
        "0".to_owned(),
        "service business cycle prompt".to_owned(),
    ]);
    let service_args = args.clone();
    let handle = thread::spawn(move || {
        let mut engine = NoironEngine::new();
        configure_engine(&mut engine, &service_args);
        let mut backend = HeuristicBackend;
        run_model_service_for_args(&mut engine, &mut backend, &service_args)
    });

    let health = wait_for_http_response(&bind, "GET", "/health", None);
    let code = r#"pub fn apply_user_feedback(memory_id: u64) -> bool { memory_id > 0 }"#;
    let request = format!(
        "{{\"prompt\":\"用中文给出 rust-norion 业务联调和反馈建议。\",\"profile\":\"coding\",\"case\":\"gemma-service-rust-feedback\",\"feedback_amount\":0.5,\"rust_check_code\":{},\"rust_check_case\":\"business-cycle-rust-check\",\"self_improve\":true,\"self_improve_limit\":1,\"gate\":\"business_cycle\",\"trace_gate\":true}}",
        service_json_string(code)
    );
    let business_cycle = service_http_request(&bind, "POST", "/v1/business-cycle", Some(&request));
    handle.join().unwrap().unwrap();

    let health_body = http_body(&health);
    let cycle_body = http_body(&business_cycle);

    assert!(health_body.contains("\"ok\":true"));
    assert!(cycle_body.contains("\"ok\":true"), "{cycle_body}");
    assert!(
        cycle_body.contains("\"business_cycle\":{\"passed\":true"),
        "{cycle_body}"
    );
    assert!(
        cycle_body.contains("\"feedback_passed\":true"),
        "{cycle_body}"
    );
    assert!(
        cycle_body.contains("\"rust_check_checked\":true"),
        "{cycle_body}"
    );
    assert!(
        cycle_body.contains("\"rust_check_passed\":true"),
        "{cycle_body}"
    );
    assert!(
        cycle_body.contains("\"self_improve_passed\":true"),
        "{cycle_body}"
    );
    assert!(
        cycle_body.contains("\"state_gate\":{\"passed\":true"),
        "{cycle_body}"
    );
    assert!(
        cycle_body.contains("\"trace_gate\":{\"passed\":true"),
        "{cycle_body}"
    );
    assert!(
        json_u64_field(cycle_body, "feedback_applied").unwrap_or_default() >= 1,
        "{cycle_body}"
    );
    assert!(
        json_u64_field(cycle_body, "runtime_token_count").is_some(),
        "{cycle_body}"
    );
    assert!(
        json_u64_field(cycle_body, "rust_check_passed").unwrap_or_default() >= 1
            || cycle_body.contains("\"rust_check_passed\":true"),
        "{cycle_body}"
    );
    assert!(
        json_u64_field(cycle_body, "evolution_external_feedbacks").unwrap_or_default() >= 2,
        "{cycle_body}"
    );
    assert!(
        json_u64_field(cycle_body, "evolution_replay_runs").unwrap_or_default() >= 1,
        "{cycle_body}"
    );

    let trace_report = evaluate_trace_schema_jsonl(&trace).unwrap();
    let trace_content = fs::read_to_string(&trace).unwrap();
    let state_report = run_state_inspection(&args).unwrap();
    assert!(trace_report.passed, "{:?}", trace_report.failures);
    assert_eq!(trace_report.checked_lines, 3);
    assert_eq!(trace_report.business_contract_events, 1);
    assert_eq!(trace_report.business_contract_event_passed, 1);
    assert_eq!(trace_report.business_contract_event_failed, 0);
    assert_eq!(trace_report.rust_check_events, 1);
    assert_eq!(trace_report.rust_check_passed, 1);
    assert!(
        trace_content.contains("\"schema\":\"rust-norion-rust-check-v1\""),
        "{trace_content}"
    );
    assert!(
        trace_content.contains("\"schema\":\"rust-norion-business-contract-v1\""),
        "{trace_content}"
    );
    assert_eq!(state_report.evolution_ledger.live_inference_runs, 1);
    assert_eq!(state_report.evolution_ledger.external_feedbacks, 2);
    assert!(state_report.evolution_ledger.replay_runs >= 1);
    assert!(state_report.evolution_ledger.replay_rust_check_items >= 1);
    assert!(state_report.business_contract_passed_count >= 1);
    assert!(
        state_report
            .evolution_ledger
            .replay_business_contract_passed
            >= 1
    );

    if let Some(source_path) = json_string_field(cycle_body, "source_path")
        && let Some(parent) = PathBuf::from(source_path).parent()
    {
        let _ = fs::remove_dir_all(parent);
    }
    fs::remove_dir_all(asset_dir).unwrap();
}
