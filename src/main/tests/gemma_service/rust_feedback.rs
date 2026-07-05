use super::*;

#[test]
fn model_service_rust_check_feedback_flows_into_replay() {
    let asset_dir = target_asset_dir("model-service-rust-check-smoke");
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
        "5".to_owned(),
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
        "--inspect-min-rust-check-experiences".to_owned(),
        "1".to_owned(),
        "--inspect-min-rust-check-passed".to_owned(),
        "1".to_owned(),
        "--inspect-max-rust-check-failed".to_owned(),
        "0".to_owned(),
        "service rust check prompt".to_owned(),
    ]);
    let service_args = args.clone();
    let handle = thread::spawn(move || {
        let mut engine = NoironEngine::new();
        configure_engine(&mut engine, &service_args);
        let mut backend = HeuristicBackend;
        run_model_service_for_args(&mut engine, &mut backend, &service_args)
    });

    let health = wait_for_http_response(&bind, "GET", "/health", None);
    let generate_body = "{\"prompt\":\"Generate a compact Rust helper for validating ownership hints in rust-norion.\",\"profile\":\"coding\",\"case\":\"rust-check-feedback\",\"tenant_id\":\"tenant-a\",\"workspace_id\":\"workspace\",\"session_id\":\"rust-feedback-generate\"}";
    let generate = service_http_request(&bind, "POST", "/v1/generate", Some(generate_body));
    let generate_json = http_body(&generate).to_owned();
    let experience_id = json_u64_field(&generate_json, "experience_id")
        .expect("generate response must expose experience_id");
    let feedback_memory_ids = json_u64_array_field(&generate_json, "feedback_memory_ids")
        .expect("generate response must expose feedback_memory_ids");
    assert!(!feedback_memory_ids.is_empty(), "{generate_json}");
    let code = r#"pub fn ownership_hint(input: String) -> usize { input.len() }"#;
    let rust_check_request = format!(
        "{{\"experience_id\":{},\"edition\":\"2021\",\"case\":\"rust-check-feedback\",\"code\":{}}}",
        experience_id,
        service_json_string(code)
    );
    let rust_check =
        service_http_request(&bind, "POST", "/v1/rust-check", Some(&rust_check_request));
    let replay = service_http_request(&bind, "POST", "/v1/replay", Some("{\"limit\":1}"));
    let inspect = service_http_request(&bind, "POST", "/v1/inspect", Some("{\"trace_gate\":true}"));
    handle.join().unwrap().unwrap();

    let health_body = http_body(&health);
    let rust_check_body = http_body(&rust_check);
    let replay_body = http_body(&replay);
    let inspect_body = http_body(&inspect);

    assert!(health_body.contains("\"ok\":true"));
    assert!(rust_check_body.contains("\"ok\":true"), "{rust_check_body}");
    assert!(
        rust_check_body.contains("\"passed\":true"),
        "{rust_check_body}"
    );
    assert!(
        rust_check_body.contains("\"label\":\"rustc_passed\""),
        "{rust_check_body}"
    );
    assert!(
        rust_check_body.contains("\"action\":\"reinforce\""),
        "{rust_check_body}"
    );
    assert_eq!(
        json_u64_field(rust_check_body, "applied"),
        Some(feedback_memory_ids.len() as u64)
    );
    assert!(
        json_f32_field(rust_check_body, "strength_delta").unwrap_or_default() >= 0.08,
        "{rust_check_body}"
    );
    assert!(replay_body.contains("\"ok\":true"));
    assert!(replay_body.contains("\"applied\":1"), "{replay_body}");
    assert!(
        json_u64_field(replay_body, "live_memory_feedback_updates").unwrap_or_default() >= 1,
        "{replay_body}"
    );
    assert!(
        json_u64_field(replay_body, "live_memory_feedback_applied").unwrap_or_default() >= 1,
        "{replay_body}"
    );
    assert!(
        json_u64_field(replay_body, "rust_check_items").unwrap_or_default() >= 1,
        "{replay_body}"
    );
    assert!(
        json_u64_field(replay_body, "rust_check_passed").unwrap_or_default() >= 1,
        "{replay_body}"
    );
    assert_eq!(
        json_u64_field(replay_body, "rust_check_failed").unwrap_or_default(),
        0,
        "{replay_body}"
    );
    assert!(
        json_u64_field(replay_body, "rust_check_live_memory_feedback_updates").unwrap_or_default()
            >= 1,
        "{replay_body}"
    );
    assert!(
        json_u64_field(replay_body, "rust_check_live_memory_feedback_applied").unwrap_or_default()
            >= 1,
        "{replay_body}"
    );
    assert_eq!(
        json_u64_field(replay_body, "business_contract_items").unwrap_or_default(),
        0,
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
    assert!(
        json_u64_field(inspect_body, "rust_check_events").unwrap_or_default() >= 1,
        "{inspect_body}"
    );
    assert!(
        json_u64_field(inspect_body, "rust_check_feedback_applied").unwrap_or_default() >= 1,
        "{inspect_body}"
    );
    assert!(
        json_u64_field(inspect_body, "rust_check_experiences").unwrap_or_default() >= 1,
        "{inspect_body}"
    );
    assert!(
        json_u64_field(inspect_body, "rust_check_passed").unwrap_or_default() >= 1,
        "{inspect_body}"
    );
    assert!(
        json_u64_field(inspect_body, "evolution_replay_rust_check_items").unwrap_or_default() >= 1,
        "{inspect_body}"
    );
    assert!(
        json_u64_field(inspect_body, "evolution_replay_rust_check_passed").unwrap_or_default() >= 1,
        "{inspect_body}"
    );
    assert!(
        json_u64_field(
            inspect_body,
            "evolution_replay_rust_check_live_memory_feedback_updates"
        )
        .unwrap_or_default()
            >= 1,
        "{inspect_body}"
    );

    let trace_report = evaluate_trace_schema_jsonl(&trace).unwrap();
    let trace_content = fs::read_to_string(&trace).unwrap();
    let state_report = run_state_inspection(&args).unwrap();
    assert!(trace_report.passed, "{:?}", trace_report.failures);
    assert_eq!(trace_report.checked_lines, 2);
    assert_eq!(trace_report.rust_check_events, 1);
    assert_eq!(trace_report.rust_check_passed, 1);
    assert_eq!(trace_report.rust_check_failed, 0);
    assert!(trace_report.rust_check_feedback_updates >= 1);
    assert!(trace_report.rust_check_feedback_applied >= 1);
    assert!(
        trace_content.contains("\"schema\":\"rust-norion-rust-check-v1\""),
        "{trace_content}"
    );
    assert_eq!(state_report.evolution_ledger.external_feedbacks, 1);
    assert_eq!(
        state_report
            .evolution_ledger
            .external_feedback_memory_updates,
        feedback_memory_ids.len() as u64
    );
    assert_eq!(state_report.evolution_ledger.replay_runs, 1);
    assert!(
        state_report
            .evolution_ledger
            .replay_live_memory_feedback_updates()
            >= 1
    );
    assert!(state_report.evolution_ledger.replay_rust_check_items >= 1);
    assert!(state_report.evolution_ledger.replay_rust_check_passed >= 1);
    assert!(
        state_report
            .evolution_ledger
            .replay_rust_check_live_memory_feedback_updates
            >= 1
    );

    if let Some(source_path) = json_string_field(rust_check_body, "source_path")
        && let Some(parent) = PathBuf::from(source_path).parent()
    {
        let _ = fs::remove_dir_all(parent);
    }
    fs::remove_dir_all(asset_dir).unwrap();
}
