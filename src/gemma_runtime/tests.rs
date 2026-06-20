use std::path::PathBuf;

use crate::runtime::{CommandTextOutputFilter, RuntimeMetadata};

use super::*;

#[test]
fn gemma4_12b_defaults_build_mistralrs_quantized_command() {
    let config = GemmaRuntimeConfig::default();

    assert_eq!(config.program, PathBuf::from("mistralrs"));
    assert_eq!(config.model_id, GEMMA4_12B_MODEL_ID);
    assert_eq!(
        config.command_args(),
        vec![
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
}

#[test]
fn gemma4_12b_local_snapshot_uses_explicit_isq_without_network_token() {
    let config = GemmaRuntimeConfig::default()
        .with_model_id("D:\\hf-cache\\gemma")
        .with_isq(GEMMA4_12B_DEFAULT_LOCAL_ISQ)
        .with_token_source("none")
        .with_paged_attn(GEMMA4_12B_DEFAULT_PAGED_ATTN);

    assert_eq!(
        config.command_args(),
        vec![
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
            "D:\\hf-cache\\gemma",
            "--max-seq-len",
            "4096",
            "--paged-attn",
            "off",
        ]
    );

    assert_eq!(
        config.command_runtime().command_text_output_filter(),
        CommandTextOutputFilter::MistralRsCli
    );
}

#[test]
fn gemma4_12b_local_snapshot_builds_persistent_serve_args() {
    let config = GemmaRuntimeConfig::default()
        .with_model_id("D:\\hf-cache\\gemma")
        .with_isq(GEMMA4_12B_DEFAULT_LOCAL_ISQ)
        .with_token_source("none")
        .with_paged_attn(GEMMA4_12B_DEFAULT_PAGED_ATTN);

    assert_eq!(
        config.serve_args("127.0.0.1", 8686),
        vec![
            "serve",
            "--host",
            "127.0.0.1",
            "--port",
            "8686",
            "--no-ui",
            "auto",
            "--token-source",
            "none",
            "--isq",
            "4",
            "-m",
            "D:\\hf-cache\\gemma",
            "--max-seq-len",
            "4096",
            "--paged-attn",
            "off",
        ]
    );
}

#[test]
fn gemma4_12b_metadata_matches_public_config_and_disables_cli_kv_exchange() {
    let config = GemmaRuntimeConfig::default();
    let metadata = config.metadata();
    let architecture = config.architecture();

    assert_eq!(metadata.model_id, GEMMA4_12B_MODEL_ID);
    assert_eq!(
        metadata.native_context_window,
        GEMMA4_12B_DEFAULT_RUNTIME_WINDOW
    );
    assert_eq!(metadata.embedding_dimensions, GEMMA4_12B_HIDDEN_SIZE);
    assert!(!metadata.supports_kv_import);
    assert!(!metadata.supports_kv_export);
    assert_eq!(architecture.layer_count, GEMMA4_12B_LAYER_COUNT);
    assert_eq!(architecture.hidden_size, GEMMA4_12B_HIDDEN_SIZE);
    assert_eq!(architecture.attention_heads, GEMMA4_12B_ATTENTION_HEADS);
    assert_eq!(architecture.kv_heads, GEMMA4_12B_KV_HEADS);
    assert_eq!(
        architecture.local_window_tokens,
        GEMMA4_12B_LOCAL_WINDOW_TOKENS
    );
}

#[test]
fn gemma4_12b_fit_summary_prefers_four_bit_on_sixteen_gb_gpu() {
    let summary = GemmaRuntimeFitSummary::for_vram(16_376);

    assert!(!summary.fits_bf16_weights);
    assert!(summary.fits_q4_weights);
    assert_eq!(summary.recommended_quantization, "4");
    assert!(summary.summary().contains("fits_bf16_weights=false"));
}

#[test]
fn gemma4_12b_defaults_can_populate_cli_runtime_fields_once() {
    let mut enabled = false;
    let mut metadata = RuntimeMetadata::default();
    let mut layer_count = None;
    let mut hidden_size = None;
    let mut attention_heads = None;
    let mut kv_heads = None;
    let mut local_window = None;

    ensure_gemma4_12b_runtime_defaults(
        &mut enabled,
        &mut metadata,
        &mut layer_count,
        &mut hidden_size,
        &mut attention_heads,
        &mut kv_heads,
        &mut local_window,
    );

    assert!(enabled);
    assert_eq!(metadata.model_id, GEMMA4_12B_MODEL_ID);
    assert_eq!(layer_count, Some(GEMMA4_12B_LAYER_COUNT));
    assert_eq!(hidden_size, Some(GEMMA4_12B_HIDDEN_SIZE));
    assert_eq!(attention_heads, Some(GEMMA4_12B_ATTENTION_HEADS));
    assert_eq!(kv_heads, Some(GEMMA4_12B_KV_HEADS));
    assert_eq!(local_window, Some(GEMMA4_12B_LOCAL_WINDOW_TOKENS));

    metadata = RuntimeMetadata::new("custom", "tok", 128, 32);
    layer_count = Some(2);
    ensure_gemma4_12b_runtime_defaults(
        &mut enabled,
        &mut metadata,
        &mut layer_count,
        &mut hidden_size,
        &mut attention_heads,
        &mut kv_heads,
        &mut local_window,
    );

    assert_eq!(metadata.model_id, "custom");
    assert_eq!(layer_count, Some(2));
}

#[test]
fn local_snapshot_path_infers_hf_cache_root() {
    let snapshot = "D:\\hf-cache\\hub\\models--google--gemma-4-12B-it\\snapshots\\5926caa";

    assert_eq!(
        infer_hf_cache_from_local_snapshot(snapshot),
        Some(PathBuf::from("D:\\hf-cache"))
    );
    assert_eq!(
        infer_hf_cache_from_local_snapshot(GEMMA4_12B_MODEL_ID),
        None
    );
}

#[test]
fn normalize_runtime_metadata_respects_kv_capabilities() {
    let no_kv = RuntimeMetadata::new("m", "t", 128, 32)
        .with_kv_limits(99, 99)
        .with_kv_precision(8, 4);
    let normalized_no_kv = normalize_runtime_metadata(no_kv);

    assert_eq!(normalized_no_kv.max_kv_import_blocks, 0);
    assert_eq!(normalized_no_kv.max_kv_export_blocks, 0);
    assert_eq!(normalized_no_kv.hot_kv_precision_bits, 8);
    assert_eq!(normalized_no_kv.cold_kv_precision_bits, 4);

    let kv = RuntimeMetadata::new("m", "t", 128, 32)
        .with_kv_exchange(true, true)
        .with_kv_limits(1, 1)
        .with_kv_precision(4, 8);
    let normalized_kv = normalize_runtime_metadata(kv);

    assert_eq!(normalized_kv.max_kv_import_blocks, 8);
    assert_eq!(normalized_kv.max_kv_export_blocks, 4);
    assert_eq!(normalized_kv.hot_kv_precision_bits, 4);
    assert_eq!(normalized_kv.cold_kv_precision_bits, 4);
}
