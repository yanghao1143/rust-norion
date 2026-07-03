use super::*;

#[test]
fn gemma4_12b_runtime_cli_builds_quantized_mistralrs_command() {
    let args = Args::parse(vec![
        "--gemma-12b-runtime".to_owned(),
        "--gemma-quant".to_owned(),
        "4".to_owned(),
        "--gemma-runtime-window".to_owned(),
        "4096".to_owned(),
        "Explain Rust ownership with one tiny example.".to_owned(),
    ]);

    assert!(args.gemma_12b_runtime);
    assert_eq!(args.gemma_runtime_program, PathBuf::from("mistralrs"));
    assert_eq!(args.gemma_runtime_quantization, "4");
    assert_eq!(
        args.gemma_runtime_thinking.as_deref(),
        Some(rust_norion::GEMMA4_12B_DEFAULT_THINKING)
    );
    assert_eq!(
        args.gemma_runtime_quantization_mode,
        GemmaRuntimeQuantizationMode::Quant
    );
    assert_eq!(
        args.runtime_metadata.model_id,
        rust_norion::GEMMA4_12B_MODEL_ID
    );
    assert_eq!(args.runtime_metadata.native_context_window, 4096);
    assert_eq!(
        args.runtime_metadata.embedding_dimensions,
        rust_norion::GEMMA4_12B_HIDDEN_SIZE
    );
    assert!(!args.runtime_metadata.supports_kv_import);
    assert!(!args.runtime_metadata.supports_kv_export);

    let runtime = args.command_runtime().unwrap();
    assert_eq!(runtime.program(), Path::new("mistralrs"));
    assert_eq!(
        runtime.command_args(),
        &[
            "run",
            "--thinking",
            "false",
            "-i",
            "{user_prompt}",
            "auto",
            "--quant",
            "4",
            "-m",
            "google/gemma-4-12B-it",
            "--max-seq-len",
            "4096",
        ]
    );
    assert_eq!(
        runtime.metadata().model_id,
        rust_norion::GEMMA4_12B_MODEL_ID
    );
    assert_eq!(runtime.metadata().native_context_window, 4096);
    assert_eq!(
        runtime.metadata().embedding_dimensions,
        rust_norion::GEMMA4_12B_HIDDEN_SIZE
    );
    assert!(!runtime.metadata().supports_kv_import);
    assert!(!runtime.metadata().supports_kv_export);
    assert_eq!(
        runtime.architecture().layer_count,
        rust_norion::GEMMA4_12B_LAYER_COUNT
    );
    assert_eq!(
        runtime.architecture().hidden_size,
        rust_norion::GEMMA4_12B_HIDDEN_SIZE
    );
    assert_eq!(
        runtime.architecture().attention_heads,
        rust_norion::GEMMA4_12B_ATTENTION_HEADS
    );
    assert_eq!(
        runtime.architecture().kv_heads,
        rust_norion::GEMMA4_12B_KV_HEADS
    );
}

#[test]
fn runtime_timeout_flag_applies_to_gemma_command_runtime() {
    let args = Args::parse(vec![
        "--gemma-12b-runtime".to_owned(),
        "--runtime-timeout-ms".to_owned(),
        "1500".to_owned(),
        "Short bounded runtime smoke.".to_owned(),
    ]);

    assert_eq!(args.runtime_timeout_ms, Some(1500));
    assert_eq!(args.command_runtime().unwrap().timeout_ms(), Some(1500));
}

#[test]
fn runtime_command_cli_attaches_spacer_gate_for_retired_marker() {
    let args = Args::parse(vec![
        "--runtime-command".to_owned(),
        "retired_version_marker:v0.305.0-runner".to_owned(),
        "--runtime-arg".to_owned(),
        "runtime_manifest_sha_mismatch".to_owned(),
        "Run through external command runtime.".to_owned(),
    ]);
    let runtime = args.command_runtime().unwrap();
    let gate = runtime.activation_gate().expect("activation gate");
    let summary = gate.summary_line();

    assert!(!gate.allowed);
    assert_eq!(gate.decision, rust_norion::DefenseSpacerDecision::Block);
    assert_eq!(gate.matched_scope, "process_start");
    assert!(summary.contains("defense_spacer_activation_gate"));
    assert!(!summary.contains("retired_version_marker"));
    assert!(!summary.contains("sha_mismatch"));
}

#[test]
fn gemma_runtime_server_cli_uses_persistent_http_runtime() {
    let args = Args::parse(vec![
        "--gemma-12b-runtime".to_owned(),
        "--gemma-runtime-server".to_owned(),
        "http://127.0.0.1:8686".to_owned(),
        "--runtime-timeout-ms".to_owned(),
        "1500".to_owned(),
        "Use persistent Gemma runtime.".to_owned(),
    ]);

    assert!(args.gemma_12b_runtime);
    assert_eq!(
        args.gemma_runtime_server.as_deref(),
        Some("http://127.0.0.1:8686")
    );
    assert!(args.command_runtime().is_none());
    assert_eq!(
        args.gemma_http_runtime().unwrap().unwrap().timeout_ms(),
        Some(1500)
    );
}

#[test]
fn gemma4_12b_local_snapshot_cli_uses_offline_q4_runtime() {
    let snapshot = "D:\\hf-cache\\hub\\models--google--gemma-4-12B-it\\snapshots\\5926caa";
    let args = Args::parse(vec![
        "--gemma-local-snapshot".to_owned(),
        snapshot.to_owned(),
        "Use the local Gemma runtime for Noiron integration.".to_owned(),
    ]);

    assert!(args.gemma_12b_runtime);
    assert_eq!(args.runtime_metadata.model_id, snapshot);
    assert_eq!(
        args.gemma_runtime_quantization_mode,
        GemmaRuntimeQuantizationMode::Isq
    );
    assert_eq!(args.gemma_runtime_quantization, "4");
    assert_eq!(args.gemma_runtime_token_source.as_deref(), Some("none"));
    assert_eq!(args.gemma_runtime_paged_attn.as_deref(), Some("off"));
    assert_eq!(args.gemma_runtime_thinking.as_deref(), Some("false"));

    let runtime = args.command_runtime().unwrap();
    assert_eq!(
        runtime.command_args(),
        &[
            "run",
            "--thinking",
            "false",
            "-i",
            "{user_prompt}",
            "auto",
            "--token-source",
            "none",
            "--isq",
            "4",
            "-m",
            snapshot,
            "--max-seq-len",
            "4096",
            "--paged-attn",
            "off",
        ]
    );
}
