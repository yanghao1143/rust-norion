use super::*;

#[test]
fn local_learning_smoke_cli_writes_state_and_trace_gate_evidence() {
    let dir = temp_asset_dir("local-learning-smoke-cli");
    fs::create_dir_all(&dir).unwrap();
    let memory_path = dir.join("memory.ndkv");
    let experience_path = dir.join("experience.ndkv");
    let adaptive_path = dir.join("adaptive.ndkv");
    let trace_path = dir.join("trace.jsonl");
    let args = Args::parse(vec![
        "--local-learning-smoke".to_owned(),
        "--memory".to_owned(),
        memory_path.display().to_string(),
        "--experience".to_owned(),
        experience_path.display().to_string(),
        "--adaptive".to_owned(),
        adaptive_path.display().to_string(),
        "--trace-schema-gate".to_owned(),
        trace_path.display().to_string(),
        "--max-tokens".to_owned(),
        "16".to_owned(),
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
        "--device".to_owned(),
        "cpu".to_owned(),
        "safe local learning smoke prompt".to_owned(),
    ]);

    let passed = crate::cli::local_learning_smoke::run_local_learning_smoke_cli(&args)
        .expect("local learning smoke");
    let trace_report = evaluate_trace_schema_jsonl(&trace_path).unwrap();
    let trace = fs::read_to_string(&trace_path).unwrap();

    assert!(passed);
    assert!(trace_report.passed, "{:?}", trace_report.failures);
    assert_eq!(trace_report.checked_lines, 1);
    assert!(trace.contains("\"case\":\"local_learning_smoke\""));
    assert!(trace.contains("\"runtime_tokens\":{"));
    assert!(trace.contains("\"memory\":{"));
    assert!(trace.contains("\"live_evolution\":{"));
    assert!(trace.contains("\"evolution_ledger\":{"));
    assert!(
        NoironEngine::full_state_files_exist(&memory_path, &experience_path, &adaptive_path)
            .unwrap()
    );
    let restored =
        NoironEngine::load_full_state(&memory_path, &experience_path, &adaptive_path).unwrap();
    assert!(!restored.cache.is_empty());
    assert!(!restored.experience.is_empty());

    fs::remove_dir_all(dir).unwrap();
}
