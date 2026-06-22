use super::*;
use rust_norion::{DraftToken, GenerationContext, InferenceDraft, ReasoningStep};
use std::fs::{self, File};
use std::path::Path;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[path = "tests/args.rs"]
mod args;

#[path = "tests/benchmark_state.rs"]
mod benchmark_state;

#[path = "tests/coding_service_eval_cli.rs"]
mod coding_service_eval_cli;

#[path = "tests/device_config.rs"]
mod device_config;

#[path = "tests/gemma_service.rs"]
mod gemma_service;

#[path = "tests/production_runtime_cli.rs"]
mod production_runtime_cli;

#[path = "tests/self_goal_queue_cli.rs"]
mod self_goal_queue_cli;

#[path = "tests/state_gate_cli.rs"]
mod state_gate_cli;

fn assert_production_kernel_all_devices_gate_recursive_runtime_coverage(
    kernel_flag: &str,
    asset_name: &str,
) {
    let asset_dir = temp_asset_dir(asset_name);
    fs::create_dir_all(&asset_dir).unwrap();
    let weights = asset_dir.join("weights.noiron");
    let tokenizer = asset_dir.join("tokenizer.noiron");
    File::create(&weights).unwrap();
    File::create(&tokenizer).unwrap();
    let trace_path = asset_dir.join("benchmark.jsonl");
    let device_count = DeviceClass::explicit_profiles().len();
    let case_count = default_benchmark_cases().len();
    let min_adapter_kinds = if kernel_flag == "--production-reference-kernel" {
        6
    } else {
        1
    };
    let mut raw_args = vec![
        kernel_flag.to_owned(),
        "--benchmark".to_owned(),
        trace_path.display().to_string(),
        "--benchmark-all-devices".to_owned(),
        "--benchmark-gate".to_owned(),
        "--benchmark-min-quality".to_owned(),
        "0.45".to_owned(),
        "--benchmark-min-reward".to_owned(),
        "0.30".to_owned(),
        "--benchmark-min-device-profiles".to_owned(),
        device_count.to_string(),
        "--benchmark-min-recursive-device-profiles".to_owned(),
        device_count.to_string(),
        "--benchmark-min-recursive-cases".to_owned(),
        device_count.to_string(),
        "--benchmark-min-runtime-forward-cases".to_owned(),
        (device_count * case_count).to_string(),
        "--benchmark-min-runtime-forward-energy-cases".to_owned(),
        (device_count * case_count).to_string(),
        "--benchmark-min-runtime-kv-influence-cases".to_owned(),
        (device_count * case_count).to_string(),
        "--benchmark-min-runtime-architecture-cases".to_owned(),
        (device_count * case_count).to_string(),
        "--benchmark-min-runtime-architecture-device-profiles".to_owned(),
        device_count.to_string(),
        "--benchmark-min-runtime-kv-precision-cases".to_owned(),
        (device_count * case_count).to_string(),
        "--benchmark-min-runtime-kv-precision-device-profiles".to_owned(),
        device_count.to_string(),
        "--benchmark-min-runtime-layer-mode-cases".to_owned(),
        (device_count * case_count).to_string(),
        "--benchmark-min-runtime-all-layer-mode-cases".to_owned(),
        (device_count * case_count).to_string(),
        "--benchmark-min-runtime-global-layers".to_owned(),
        (device_count * case_count).to_string(),
        "--benchmark-min-runtime-local-window-layers".to_owned(),
        (device_count * case_count).to_string(),
        "--benchmark-min-runtime-convolutional-fusion-layers".to_owned(),
        (device_count * case_count).to_string(),
        "--benchmark-min-runtime-uncertainty-cases".to_owned(),
        (device_count * case_count).to_string(),
        "--benchmark-min-runtime-uncertainty-tokens".to_owned(),
        (device_count * case_count).to_string(),
        "--benchmark-min-runtime-uncertainty-device-profiles".to_owned(),
        device_count.to_string(),
        "--benchmark-min-runtime-uncertainty-token-device-profiles".to_owned(),
        device_count.to_string(),
        "--benchmark-min-runtime-kv-import-cases".to_owned(),
        (device_count * case_count).to_string(),
        "--benchmark-min-runtime-kv-imported".to_owned(),
        (device_count * case_count).to_string(),
        "--benchmark-min-runtime-kv-import-device-profiles".to_owned(),
        device_count.to_string(),
        "--benchmark-min-runtime-kv-exported".to_owned(),
        (device_count * case_count).to_string(),
        "--benchmark-min-runtime-kv-export-device-profiles".to_owned(),
        device_count.to_string(),
        "--benchmark-min-runtime-kv-stored".to_owned(),
        "1".to_owned(),
        "--benchmark-min-runtime-kv-stored-device-profiles".to_owned(),
        "1".to_owned(),
        "--benchmark-min-runtime-adapter-contract-cases".to_owned(),
        (device_count * case_count).to_string(),
        "--benchmark-min-runtime-adapter-kinds".to_owned(),
        min_adapter_kinds.to_string(),
        "--benchmark-min-runtime-adapter-observations".to_owned(),
        "1".to_owned(),
        "--benchmark-min-runtime-adapter-best-score".to_owned(),
        "0.05".to_owned(),
        "--benchmark-max-runtime-adapter-contract-violations".to_owned(),
        "0".to_owned(),
        "--benchmark-min-runtime-embedding-cases".to_owned(),
        (device_count * case_count).to_string(),
        "--benchmark-min-runtime-embedding-device-profiles".to_owned(),
        device_count.to_string(),
        "--benchmark-max-embedding-fallback-cases".to_owned(),
        "0".to_owned(),
        "--benchmark-max-embedding-evidence-failures".to_owned(),
        "0".to_owned(),
        "--benchmark-min-runtime-device-execution-cases".to_owned(),
        (device_count * case_count).to_string(),
        "--benchmark-min-runtime-device-execution-device-profiles".to_owned(),
        device_count.to_string(),
        "--benchmark-max-runtime-device-execution-violations".to_owned(),
        "0".to_owned(),
        "--benchmark-max-memory-governance-failures".to_owned(),
        "0".to_owned(),
        "--benchmark-min-memory-governance-cases".to_owned(),
        (device_count * case_count).to_string(),
        "--benchmark-min-memory-governance-device-profiles".to_owned(),
        device_count.to_string(),
        "--benchmark-min-auto-replay-router-threshold-mutations".to_owned(),
        "1".to_owned(),
        "--benchmark-min-auto-replay-hierarchy-weight-mutations".to_owned(),
        "1".to_owned(),
        "--benchmark-min-auto-replay-router-threshold-delta".to_owned(),
        "0.001".to_owned(),
        "--benchmark-min-auto-replay-hierarchy-weight-delta".to_owned(),
        "0.001".to_owned(),
        "--benchmark-min-auto-replay-memory-updates".to_owned(),
        "1".to_owned(),
        "--benchmark-min-evolution-replay-runs".to_owned(),
        "1".to_owned(),
        "--benchmark-min-evolution-replay-items".to_owned(),
        "1".to_owned(),
        "--benchmark-min-evolution-router-threshold-mutations".to_owned(),
        "1".to_owned(),
        "--benchmark-min-evolution-hierarchy-weight-mutations".to_owned(),
        "1".to_owned(),
        "--benchmark-min-evolution-router-threshold-delta".to_owned(),
        "0.001".to_owned(),
        "--benchmark-min-evolution-hierarchy-weight-delta".to_owned(),
        "0.001".to_owned(),
        "--benchmark-min-evolution-memory-updates".to_owned(),
        "1".to_owned(),
        "--benchmark-min-evolution-recursive-replay-items".to_owned(),
        "1".to_owned(),
        "--benchmark-min-evolution-recursive-runtime-calls".to_owned(),
        "1".to_owned(),
        "--benchmark-max-drift-blocks".to_owned(),
        "0".to_owned(),
        "--benchmark-max-drift-rollbacks".to_owned(),
        "0".to_owned(),
        "--runtime-model-id".to_owned(),
        "self-owned-transformer".to_owned(),
        "--runtime-tokenizer".to_owned(),
        "self-bpe".to_owned(),
        "--runtime-native-window".to_owned(),
        "64".to_owned(),
        "--runtime-embedding-dims".to_owned(),
        "64".to_owned(),
        "--runtime-layers".to_owned(),
        "6".to_owned(),
        "--runtime-hidden-size".to_owned(),
        "64".to_owned(),
        "--runtime-attention-heads".to_owned(),
        "4".to_owned(),
        "--runtime-kv-heads".to_owned(),
        "2".to_owned(),
        "--runtime-local-window".to_owned(),
        "32".to_owned(),
        "--runtime-kv-exchange".to_owned(),
        "--runtime-weights".to_owned(),
        weights.display().to_string(),
        "--runtime-tokenizer-path".to_owned(),
        tokenizer.display().to_string(),
        "--chunk-tokens".to_owned(),
        "32".to_owned(),
        "--chunk-overlap".to_owned(),
        "8".to_owned(),
        "--device".to_owned(),
        "cpu".to_owned(),
    ];
    if kernel_flag == "--production-local-kernel" {
        raw_args.extend([
            "--benchmark-max-runtime-adapter-selection-mismatches".to_owned(),
            "0".to_owned(),
        ]);
    }
    let args = Args::parse(raw_args);
    let mut engine = NoironEngine::new();
    configure_engine(&mut engine, &args);

    let summary = run_production_benchmark_all_devices(&mut engine, &args, &trace_path).unwrap();
    let gate_report = summary.evaluate(&args.benchmark_gate());
    let trace_report = evaluate_trace_schema_jsonl(&trace_path).unwrap();

    assert!(args.production_runtime);
    match kernel_flag {
        "--production-reference-kernel" => assert!(args.production_reference_kernel),
        "--production-local-kernel" => assert!(args.production_local_kernel),
        _ => panic!("unexpected production kernel flag {kernel_flag}"),
    }
    assert_eq!(summary.len(), device_count * case_count);
    assert_eq!(summary.explicit_device_profiles_covered(), device_count);
    assert_eq!(summary.recursive_device_profiles_covered(), device_count);
    assert_eq!(summary.recursive_cases(), device_count);
    assert_eq!(summary.runtime_forward_cases(), device_count * case_count);
    assert_eq!(
        summary.runtime_forward_energy_cases(),
        device_count * case_count
    );
    assert_eq!(
        summary.runtime_kv_influence_cases(),
        device_count * case_count
    );
    assert_eq!(
        summary.runtime_architecture_cases(),
        device_count * case_count
    );
    assert_eq!(summary.runtime_architecture_device_profiles(), device_count);
    assert_eq!(
        summary.runtime_kv_precision_cases(),
        device_count * case_count
    );
    assert_eq!(summary.runtime_kv_precision_device_profiles(), device_count);
    assert_eq!(
        summary.runtime_layer_mode_cases(),
        device_count * case_count
    );
    assert_eq!(
        summary.runtime_all_layer_mode_cases(),
        device_count * case_count
    );
    assert!(summary.total_runtime_global_layers() >= device_count * case_count);
    assert!(summary.total_runtime_local_window_layers() >= device_count * case_count);
    assert!(summary.total_runtime_convolutional_fusion_layers() >= device_count * case_count);
    assert_eq!(
        summary.runtime_uncertainty_cases(),
        device_count * case_count
    );
    assert!(summary.total_runtime_uncertainty_tokens() >= device_count * case_count);
    assert_eq!(summary.runtime_uncertainty_device_profiles(), device_count);
    assert_eq!(
        summary.runtime_uncertainty_token_device_profiles(),
        device_count
    );
    assert_eq!(
        args.benchmark_gate()
            .min_runtime_uncertainty_token_device_profiles,
        Some(device_count)
    );
    assert_eq!(summary.runtime_kv_import_cases(), device_count * case_count);
    assert!(summary.total_runtime_kv_imported() >= device_count * case_count);
    assert_eq!(
        summary.runtime_adapter_contract_cases(),
        device_count * case_count
    );
    assert!(summary.runtime_adapter_kinds() >= min_adapter_kinds);
    assert!(summary.total_runtime_adapter_observations() >= 1);
    assert!(summary.max_runtime_adapter_score().unwrap_or(0.0) >= 0.05);
    assert_eq!(summary.total_runtime_adapter_contract_violations(), 0);
    if kernel_flag == "--production-local-kernel" {
        assert_eq!(
            args.benchmark_gate()
                .max_runtime_adapter_selection_mismatches,
            Some(0)
        );
        assert_eq!(summary.total_runtime_adapter_selection_mismatches(), 0);
    } else {
        assert_eq!(
            args.benchmark_gate()
                .max_runtime_adapter_selection_mismatches,
            None
        );
    }
    assert_eq!(summary.runtime_embedding_cases(), device_count * case_count);
    assert_eq!(summary.runtime_embedding_device_profiles(), device_count);
    assert!(summary.total_runtime_embedding_calls() >= device_count * case_count);
    assert_eq!(summary.embedding_fallback_cases(), 0);
    assert_eq!(summary.total_fallback_embedding_calls(), 0);
    assert_eq!(summary.total_embedding_evidence_failures(), 0);
    assert_eq!(
        summary.runtime_device_execution_matched_cases(),
        device_count * case_count
    );
    assert_eq!(
        summary.runtime_device_execution_device_profiles(),
        device_count
    );
    assert_eq!(summary.total_runtime_device_execution_violations(), 0);
    assert_eq!(summary.memory_governance_cases(), device_count * case_count);
    assert_eq!(summary.memory_governance_device_profiles(), device_count);
    assert_eq!(summary.memory_governance_evidence().failures.len(), 0);
    assert!(summary.total_runtime_kv_exported() >= device_count * case_count);
    assert!(summary.total_runtime_kv_stored() >= 1);
    assert!(summary.total_auto_replay_router_threshold_mutations() >= 1);
    assert!(summary.total_auto_replay_hierarchy_weight_mutations() >= 1);
    assert!(summary.total_auto_replay_router_threshold_delta() >= 0.001);
    assert!(summary.total_auto_replay_hierarchy_weight_delta() >= 0.001);
    assert!(summary.total_auto_replay_memory_updates() >= 1);
    assert!(summary.evolution_ledger().replay_runs >= 1);
    assert!(summary.evolution_ledger().replay_items >= 1);
    assert!(summary.evolution_ledger().router_threshold_mutations >= 1);
    assert!(summary.evolution_ledger().hierarchy_weight_mutations >= 1);
    assert!(summary.evolution_ledger().router_threshold_delta >= 0.001);
    assert!(summary.evolution_ledger().hierarchy_weight_delta >= 0.001);
    assert!(summary.evolution_ledger().memory_updates() >= 1);
    assert!(summary.evolution_ledger().recursive_replay_items >= 1);
    assert!(summary.evolution_ledger().recursive_runtime_calls >= 1);
    assert!(summary.results().iter().any(|result| {
        result.device == DeviceClass::Microcontroller
            && result.name.starts_with("microcontroller_")
            && result.runtime_forward_signal
            && result.runtime_selected_adapter.as_deref() == Some("portable-rust")
            && result.runtime_adapter_contract_ok
    }));
    let trace = fs::read_to_string(&trace_path).unwrap();
    let microcontroller_line = trace
        .lines()
        .find(|line| line.contains("\"case\":\"microcontroller_long_context_scheduler\""))
        .unwrap();
    assert!(microcontroller_line.contains("\"runtime_device_contract\":\"device=microcontroller"));
    assert!(microcontroller_line.contains("\"device_profile\":\"microcontroller\""));
    assert!(microcontroller_line.contains("\"selected_adapter\":\"portable-rust\""));
    assert!(microcontroller_line.contains("\"embedding\":{"));
    assert!(microcontroller_line.contains("\"query_source\":\"runtime\""));
    assert!(microcontroller_line.contains("\"fallback_embedding_calls\":0"));
    assert!(microcontroller_line.contains("\"max_parallel_chunks\":1"));

    let discrete_line = trace
        .lines()
        .find(|line| line.contains("\"case\":\"discrete_long_context_scheduler\""))
        .unwrap();
    assert!(discrete_line.contains("\"runtime_device_contract\":\"device=discrete"));
    if kernel_flag == "--production-reference-kernel" {
        assert!(discrete_line.contains("\"selected_adapter\":\"cuda\""));
    } else {
        assert_trace_selected_adapter_allowed(
            discrete_line,
            &[
                "cuda",
                "rocm",
                "vulkan",
                "wgpu",
                "oneapi",
                "directml",
                "portable-rust",
            ],
        );
    }
    assert!(discrete_line.contains("\"execution_waves\":23"));

    let multi_gpu_line = trace
        .lines()
        .find(|line| line.contains("\"case\":\"multi-gpu_long_context_scheduler\""))
        .unwrap();
    assert!(multi_gpu_line.contains("\"runtime_device_contract\":\"device=multi-gpu"));
    if kernel_flag == "--production-reference-kernel" {
        assert!(multi_gpu_line.contains("\"selected_adapter\":\"multi-device\""));
    } else {
        assert_trace_selected_adapter_allowed(
            multi_gpu_line,
            &[
                "multi-device",
                "cuda",
                "rocm",
                "oneapi",
                "vulkan",
                "wgpu",
                "custom-accelerator",
                "portable-rust",
            ],
        );
    }
    assert!(multi_gpu_line.contains("\"execution_waves\":12"));
    assert!(
        summary
            .summary_line()
            .contains("recursive_device_profiles=12")
    );
    assert!(
        summary
            .summary_line()
            .contains("runtime_forward_energy_cases=48")
    );
    assert!(
        summary
            .summary_line()
            .contains("runtime_kv_influence_cases=48")
    );
    assert!(
        summary
            .summary_line()
            .contains("runtime_architecture_cases=48")
    );
    assert!(
        summary
            .summary_line()
            .contains("runtime_architecture_device_profiles=12")
    );
    assert!(
        summary
            .summary_line()
            .contains("runtime_layer_mode_cases=48")
    );
    assert!(
        summary
            .summary_line()
            .contains("runtime_all_layer_mode_cases=48")
    );
    assert!(summary.summary_line().contains("runtime_global_layers="));
    assert!(
        summary
            .summary_line()
            .contains("runtime_local_window_layers=")
    );
    assert!(
        summary
            .summary_line()
            .contains("runtime_convolutional_fusion_layers=")
    );
    assert!(
        summary
            .summary_line()
            .contains("runtime_adapter_contract_cases=48")
    );
    assert!(
        summary
            .summary_line()
            .contains("runtime_device_execution_matched_cases=48")
    );
    assert!(
        summary
            .summary_line()
            .contains("runtime_device_execution_device_profiles=12")
    );
    assert!(
        summary
            .summary_line()
            .contains("runtime_device_execution_violations=0")
    );
    assert!(summary.summary_line().contains("runtime_adapter_kinds="));
    assert!(
        summary
            .summary_line()
            .contains("runtime_adapter_observations=")
    );
    assert!(
        summary
            .summary_line()
            .contains("runtime_adapter_best_score=")
    );
    assert!(
        summary
            .summary_line()
            .contains("runtime_embedding_cases=48")
    );
    assert!(
        summary
            .summary_line()
            .contains("runtime_embedding_device_profiles=12")
    );
    assert!(
        summary
            .summary_line()
            .contains("embedding_fallback_cases=0")
    );
    assert!(
        summary
            .summary_line()
            .contains("embedding_evidence_failures=0")
    );
    assert!(
        summary
            .summary_line()
            .contains("runtime_uncertainty_cases=48")
    );
    assert!(
        summary
            .summary_line()
            .contains("runtime_kv_import_cases=48")
    );
    assert!(summary.summary_line().contains("runtime_kv_stored="));
    assert!(
        summary
            .summary_line()
            .contains("auto_replay_router_threshold_mutations=")
    );
    assert!(
        summary
            .summary_line()
            .contains("auto_replay_hierarchy_weight_mutations=")
    );
    assert!(
        summary
            .summary_line()
            .contains("auto_replay_router_threshold_delta=")
    );
    assert!(
        summary
            .summary_line()
            .contains("auto_replay_hierarchy_weight_delta=")
    );
    assert!(
        summary
            .summary_line()
            .contains("runtime_adapter_contract_violations=0")
    );
    assert!(
        summary
            .summary_line()
            .contains("memory_governance_cases=48")
    );
    assert!(
        summary
            .summary_line()
            .contains("memory_governance_device_profiles=12")
    );
    assert!(
        summary
            .summary_line()
            .contains("memory_governance_failures=0")
    );
    assert!(gate_report.passed, "{:?}", gate_report.failures);
    assert!(trace_report.passed, "{:?}", trace_report.failures);
    assert_eq!(trace_report.checked_lines, summary.len());

    fs::remove_dir_all(asset_dir).unwrap();
}

fn assert_production_kernel_conformance_all_devices_cli_passes(
    kernel_flag: &str,
    asset_name: &str,
) {
    let asset_dir = temp_asset_dir(asset_name);
    fs::create_dir_all(&asset_dir).unwrap();
    let weights = asset_dir.join("weights.noiron");
    let tokenizer = asset_dir.join("tokenizer.noiron");
    File::create(&weights).unwrap();
    File::create(&tokenizer).unwrap();
    let args = Args::parse(production_conformance_all_devices_args(
        kernel_flag,
        &weights,
        &tokenizer,
    ));

    let report = run_production_kernel_conformance_all_devices(&args);

    assert!(args.production_runtime);
    assert!(args.production_kernel_conformance_gate);
    assert!(args.production_kernel_conformance_all_devices_gate);
    match kernel_flag {
        "--production-reference-kernel" => assert!(args.production_reference_kernel),
        "--production-local-kernel" => assert!(args.production_local_kernel),
        _ => panic!("unexpected production kernel flag {kernel_flag}"),
    }
    assert!(report.passed, "{report:?}");
    assert_eq!(
        report.covered_devices(),
        DeviceClass::explicit_profiles().len()
    );
    assert!(report.missing_devices().is_empty());
    assert!(report.failed_devices().is_empty());
    assert!(report.device_reports.iter().all(|device_report| {
        device_report.report.kernel_connected
            && device_report.report.token_count > 0
            && device_report.report.trace_steps > 0
            && device_report.report.imported_kv_blocks > 0
            && device_report.report.exported_kv_blocks > 0
    }));
    assert!(report.summary_line().contains("devices=12"));

    fs::remove_dir_all(asset_dir).unwrap();
}

fn production_conformance_all_devices_args(
    kernel_flag: &str,
    weights: &Path,
    tokenizer: &Path,
) -> Vec<String> {
    vec![
        kernel_flag.to_owned(),
        "--production-kernel-conformance-all-devices-gate".to_owned(),
        "--runtime-model-id".to_owned(),
        "self-owned-transformer".to_owned(),
        "--runtime-tokenizer".to_owned(),
        "self-bpe".to_owned(),
        "--runtime-native-window".to_owned(),
        "4096".to_owned(),
        "--runtime-embedding-dims".to_owned(),
        "64".to_owned(),
        "--runtime-layers".to_owned(),
        "6".to_owned(),
        "--runtime-hidden-size".to_owned(),
        "64".to_owned(),
        "--runtime-attention-heads".to_owned(),
        "4".to_owned(),
        "--runtime-kv-heads".to_owned(),
        "2".to_owned(),
        "--runtime-local-window".to_owned(),
        "1024".to_owned(),
        "--runtime-kv-exchange".to_owned(),
        "--runtime-weights".to_owned(),
        weights.display().to_string(),
        "--runtime-tokenizer-path".to_owned(),
        tokenizer.display().to_string(),
        "--device".to_owned(),
        "cpu".to_owned(),
    ]
}

#[cfg(windows)]
fn command_runtime_json_import_probe_args() -> (String, Vec<String>) {
    let escaped = command_runtime_json_import_probe_response().replace('\'', "''");
    (
        "powershell.exe".to_owned(),
        vec![
            "-NoProfile".to_owned(),
            "-NonInteractive".to_owned(),
            "-Command".to_owned(),
            format!(
                "$payload = [Console]::In.ReadToEnd(); if (-not $payload.Contains('\"imported_kv_blocks\":[') -or -not $payload.Contains('\"layer\":0')) {{ exit 7 }}; Write-Output '{escaped}'"
            ),
        ],
    )
}

#[cfg(not(windows))]
fn command_runtime_json_import_probe_args() -> (String, Vec<String>) {
    let escaped = command_runtime_json_import_probe_response().replace('\'', "'\\''");
    (
        "sh".to_owned(),
        vec![
            "-c".to_owned(),
            format!(
                "payload=$(cat); case \"$payload\" in *'\"imported_kv_blocks\":['*'\"layer\":0'*) printf '%s' '{escaped}' ;; *) exit 7 ;; esac"
            ),
        ],
    )
}

fn command_runtime_json_import_probe_response() -> &'static str {
    "{\"schema\":\"rust-norion-runtime-response-v1\",\"answer\":\"command runtime production answer\",\"tokens\":[{\"text\":\"command\",\"logprob\":-0.1,\"entropy\":0.2}],\"trace\":[{\"label\":\"command_kernel\",\"content\":\"external self-developed runtime received imported KV through production ABI\",\"confidence\":0.92}],\"exported_kv_blocks\":[{\"layer\":1,\"head\":0,\"token_start\":0,\"token_end\":1,\"key\":[0.1,0.2],\"value\":[0.3,0.4]}],\"diagnostics\":{\"model_id\":\"self-owned-transformer\",\"selected_adapter\":\"portable-rust\",\"global_layers\":2,\"local_window_layers\":3,\"convolutional_fusion_layers\":1,\"forward_energy\":0.42,\"kv_influence\":0.25,\"hot_kv_precision_bits\":8,\"cold_kv_precision_bits\":4,\"imported_kv_blocks\":1}}"
}

fn assert_trace_selected_adapter_allowed(line: &str, allowed: &[&str]) {
    assert!(
        allowed
            .iter()
            .any(|adapter| line.contains(&format!("\"selected_adapter\":\"{adapter}\""))),
        "selected adapter was outside allowed set {:?}: {}",
        allowed,
        line
    );
}

fn assert_gemma_smoke_paths_are_isolated(args: &Args, base_dir: &str) {
    let smoke_dir = args
        .memory_path
        .parent()
        .expect("memory path should have smoke dir");
    let base = PathBuf::from(base_dir);
    assert_eq!(
        smoke_dir.parent().map(|path| path.to_path_buf()),
        base.parent().map(|path| path.to_path_buf())
    );
    let base_name = base
        .file_name()
        .expect("base dir should have name")
        .to_string_lossy();
    let smoke_dir_name = smoke_dir
        .file_name()
        .expect("smoke dir should have name")
        .to_string_lossy();
    assert!(
        smoke_dir_name.starts_with(&format!("{base_name}-")),
        "smoke dir {smoke_dir:?} should start with {base_name}-"
    );
    assert_eq!(args.memory_path, smoke_dir.join("memory.ndkv"));
    assert_eq!(args.experience_path, smoke_dir.join("experience.ndkv"));
    assert_eq!(args.adaptive_path, smoke_dir.join("adaptive.ndkv"));
    let trace_path = smoke_dir.join("trace.jsonl");
    assert_eq!(args.trace_path.as_ref(), Some(&trace_path));
}

fn temp_asset_dir(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("{name}-{unique}"))
}

fn target_asset_dir(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    PathBuf::from("target").join(format!("{name}-{unique}"))
}

fn reserve_loopback_addr() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);
    addr.to_string()
}

fn wait_for_http_response(addr: &str, method: &str, path: &str, body: Option<&str>) -> String {
    let mut last_error = None;
    for _ in 0..100 {
        match try_service_http_request(addr, method, path, body) {
            Ok(response) => return response,
            Err(error) => {
                last_error = Some(error);
                thread::sleep(Duration::from_millis(20));
            }
        }
    }
    panic!(
        "service did not respond at {addr}: {:?}",
        last_error.map(|error| error.to_string())
    );
}

fn service_http_request(addr: &str, method: &str, path: &str, body: Option<&str>) -> String {
    try_service_http_request(addr, method, path, body).unwrap()
}

fn try_service_http_request(
    addr: &str,
    method: &str,
    path: &str,
    body: Option<&str>,
) -> std::io::Result<String> {
    let body = body.unwrap_or("");
    let request = if method.eq_ignore_ascii_case("GET") {
        format!("GET {path} HTTP/1.1\r\nhost: {addr}\r\nconnection: close\r\n\r\n")
    } else {
        format!(
            "{method} {path} HTTP/1.1\r\nhost: {addr}\r\ncontent-type: application/json; charset=utf-8\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        )
    };
    let mut stream = TcpStream::connect(addr)?;
    stream.write_all(request.as_bytes())?;
    let mut response = String::new();
    stream.read_to_string(&mut response)?;
    Ok(response)
}

fn http_body(response: &str) -> &str {
    split_http_head_body(response).1
}
