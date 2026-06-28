use super::*;

#[test]
fn ordinary_local_runtime_cli_loads_self_evolving_snapshot_hints() {
    let dir = temp_asset_dir("ordinary-local-runtime-sem-hints");
    fs::create_dir_all(&dir).unwrap();
    let memory_path = dir.join("memory.ndkv");
    let experience_path = dir.join("experience.ndkv");
    let adaptive_path = dir.join("adaptive.ndkv");
    let sem_path = experience_path.with_extension("self-evolving-memory.tsv");
    let trace_path = dir.join("trace.jsonl");
    let prompt = "safe dispatch runtime prompt";

    let approval = rust_norion::SelfEvolvingMemoryApproval::approved(
        "rollback:dispatch-sem".to_owned(),
        vec!["dispatch-sem-test".to_owned()],
    );
    let mut store = rust_norion::SelfEvolvingMemoryStore::new();
    store.append_episode(
        rust_norion::SelfEvolvingEpisodeInput {
            problem: "private dispatch sem prompt".to_owned(),
            solution_path: "runtime hint reuse".to_owned(),
            outcome: "positive runtime reuse".to_owned(),
            key_insights: vec!["dispatch hint enters runtime".to_owned()],
            tags: vec!["runtime".to_owned()],
            profile: TaskProfile::General,
            quality: 0.91,
            token_estimate: 8,
            source_case_id: "case:dispatch-sem".to_owned(),
        },
        &approval,
    );
    store.append_heuristic(
        rust_norion::SelfEvolvingHeuristicInput {
            rule: "prefer durable digest hints before fresh runtime compute".to_owned(),
            tags: vec!["runtime".to_owned()],
            profile: TaskProfile::General,
            priority: 0.82,
            confidence: 0.84,
            source_case_id: "case:dispatch-sem".to_owned(),
            updated_step: 1,
        },
        &approval,
    );
    store.observe_tool(
        rust_norion::ToolReliabilityObservationInput {
            tool_name: "local_transformer_runtime".to_owned(),
            profile: TaskProfile::General,
            success: true,
            quality: 0.88,
            source_case_id: "case:dispatch-sem".to_owned(),
            observed_step: 1,
        },
        &approval,
    );
    store.save_snapshot(&sem_path).unwrap();
    let sem_snapshot = fs::read_to_string(&sem_path).unwrap();
    assert!(!sem_snapshot.contains("private dispatch sem prompt"));

    let args = Args::parse(vec![
        "--local-runtime".to_owned(),
        "--memory".to_owned(),
        memory_path.display().to_string(),
        "--experience".to_owned(),
        experience_path.display().to_string(),
        "--adaptive".to_owned(),
        adaptive_path.display().to_string(),
        "--trace".to_owned(),
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
        prompt.to_owned(),
    ]);

    dispatch::run(args).unwrap();

    let saved = NoironEngine::load_full_state(&memory_path, &experience_path, &adaptive_path)
        .expect("saved dispatch state");
    let latest = saved.experience.records().last().unwrap();
    assert!(latest.lesson.contains("3 experience hints"));
    assert!(!latest.lesson.contains("private dispatch sem prompt"));
    let trace_report = evaluate_trace_schema_jsonl(&trace_path).unwrap();
    assert!(
        trace_report.compute_budget_self_evolving_memory_fusion_saved_tokens > 0,
        "{}",
        trace_report.summary_line()
    );

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn self_evolving_memory_quarantine_cli_applies_to_snapshot() {
    let dir = temp_asset_dir("self-evolving-memory-quarantine-cli");
    fs::create_dir_all(&dir).unwrap();
    let experience_path = dir.join("experience.ndkv");
    let sem_path = experience_path.with_extension("self-evolving-memory.tsv");
    let trace_path = dir.join("quarantine-trace.jsonl");
    let polluted_source_case = "case:polluted-context".to_owned();

    let approval = rust_norion::SelfEvolvingMemoryApproval::approved(
        "rollback:sem-quarantine-cli",
        vec!["sem-quarantine-cli-test".to_owned()],
    );
    let mut store = rust_norion::SelfEvolvingMemoryStore::new();
    store.append_episode(
        rust_norion::SelfEvolvingEpisodeInput {
            problem: "polluted source case prompt".to_owned(),
            solution_path: "polluted route".to_owned(),
            outcome: "polluted outcome".to_owned(),
            key_insights: vec!["polluted insight".to_owned()],
            tags: vec!["runtime".to_owned()],
            profile: TaskProfile::General,
            quality: 0.95,
            token_estimate: 8,
            source_case_id: polluted_source_case.clone(),
        },
        &approval,
    );
    store.append_heuristic(
        rust_norion::SelfEvolvingHeuristicInput {
            rule: "polluted rule".to_owned(),
            tags: vec!["runtime".to_owned()],
            profile: TaskProfile::General,
            priority: 0.90,
            confidence: 0.90,
            source_case_id: polluted_source_case.clone(),
            updated_step: 1,
        },
        &approval,
    );
    store.observe_tool(
        rust_norion::ToolReliabilityObservationInput {
            tool_name: "local_transformer_runtime".to_owned(),
            profile: TaskProfile::General,
            success: false,
            quality: 0.10,
            source_case_id: polluted_source_case.clone(),
            observed_step: 1,
        },
        &approval,
    );
    let polluted_digest = store.episodes()[0].source_case_digest.clone();
    store.save_snapshot(&sem_path).unwrap();

    let dry_run_args = Args::parse(vec![
        "--experience".to_owned(),
        experience_path.display().to_string(),
        "--trace".to_owned(),
        trace_path.display().to_string(),
        "--self-evolving-memory-quarantine-source-case".to_owned(),
        polluted_source_case.clone(),
        "--self-evolving-memory-quarantine-reason".to_owned(),
        "context_polluted".to_owned(),
    ]);

    dispatch::run(dry_run_args).unwrap();

    let dry_run_loaded = rust_norion::SelfEvolvingMemoryStore::load_snapshot(&sem_path).unwrap();
    assert!(dry_run_loaded
        .episodes()
        .iter()
        .any(|record| record.source_case_digest == polluted_digest && record.active));
    assert!(dry_run_loaded
        .tool_observations()
        .iter()
        .any(|record| record.source_case_digest == polluted_digest));
    let dry_run_report = evaluate_trace_schema_jsonl(&trace_path).unwrap();
    assert!(dry_run_report.passed, "{:?}", dry_run_report.failures);
    assert_eq!(dry_run_report.self_evolving_memory_store_events, 1);
    assert_eq!(
        dry_run_report.self_evolving_memory_store_source_quarantine_events,
        1
    );
    assert_eq!(
        dry_run_report.self_evolving_memory_store_source_quarantine_actions,
        3
    );
    assert_eq!(dry_run_report.self_evolving_memory_store_write_allowed, 0);
    assert_eq!(dry_run_report.self_evolving_memory_store_applied, 0);
    assert_eq!(dry_run_report.self_evolving_memory_store_applied_to_disk, 0);

    let args = Args::parse(vec![
        "--experience".to_owned(),
        experience_path.display().to_string(),
        "--trace".to_owned(),
        trace_path.display().to_string(),
        "--self-evolving-memory-quarantine-source-case".to_owned(),
        polluted_source_case.clone(),
        "--self-evolving-memory-quarantine-reason".to_owned(),
        "context_polluted".to_owned(),
        "--self-evolving-memory-quarantine-apply".to_owned(),
    ]);

    dispatch::run(args).unwrap();

    let saved = fs::read_to_string(&sem_path).unwrap();
    assert!(!saved.contains(&polluted_source_case));
    assert!(saved.contains(&polluted_digest));
    let loaded = rust_norion::SelfEvolvingMemoryStore::load_snapshot(&sem_path).unwrap();
    assert!(loaded
        .episodes()
        .iter()
        .any(|record| record.source_case_digest == polluted_digest && !record.active));
    assert!(loaded.heuristics().iter().any(|record| {
        record.source_case_digest == polluted_digest
            && record.quarantined
            && record.quarantine_reason.as_deref() == Some("context_polluted")
    }));
    assert!(loaded
        .tool_observations()
        .iter()
        .all(|record| record.source_case_digest != polluted_digest));
    let snapshot_digest = loaded.snapshot_digest();
    let trace = fs::read_to_string(&trace_path).unwrap();
    let trace_report = evaluate_trace_schema_jsonl(&trace_path).unwrap();
    assert!(trace_report.passed, "{:?}", trace_report.failures);
    assert_eq!(trace_report.self_evolving_memory_store_events, 2);
    assert_eq!(
        trace_report.self_evolving_memory_store_source_quarantine_events,
        2
    );
    assert_eq!(
        trace_report.self_evolving_memory_store_source_quarantine_actions,
        6
    );
    assert_eq!(trace_report.self_evolving_memory_store_write_allowed, 1);
    assert_eq!(
        trace_report.self_evolving_memory_store_durable_write_allowed,
        1
    );
    assert_eq!(trace_report.self_evolving_memory_store_applied, 1);
    assert_eq!(trace_report.self_evolving_memory_store_applied_to_disk, 1);
    assert!(trace.contains("\"operation\":\"source_quarantine\""));
    assert!(trace.contains(&format!("\"source_case_digest\":\"{polluted_digest}\"")));
    assert!(trace.contains(&format!("\"snapshot_digest\":\"{snapshot_digest}\"")));
    assert!(trace.contains(&format!("\"disk_snapshot_digest\":\"{snapshot_digest}\"")));
    assert!(!trace.contains(&polluted_source_case));
    assert!(fs::read_dir(&dir).unwrap().any(|entry| {
        entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains(".backup.")
    }));

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn ordinary_local_runtime_cli_persists_self_evolving_snapshot_from_inference() {
    let dir = temp_asset_dir("ordinary-local-runtime-sem-writeback");
    fs::create_dir_all(&dir).unwrap();
    let memory_path = dir.join("memory.ndkv");
    let experience_path = dir.join("experience.ndkv");
    let adaptive_path = dir.join("adaptive.ndkv");
    let sem_path = experience_path.with_extension("self-evolving-memory.tsv");
    let trace_path = dir.join("trace.jsonl");
    let prompt = "private ordinary runtime sem writeback prompt";
    let sem_before_digest = rust_norion::SelfEvolvingMemoryStore::new().snapshot_digest();

    let args = Args::parse(vec![
        "--local-runtime".to_owned(),
        "--memory".to_owned(),
        memory_path.display().to_string(),
        "--experience".to_owned(),
        experience_path.display().to_string(),
        "--adaptive".to_owned(),
        adaptive_path.display().to_string(),
        "--trace".to_owned(),
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
        prompt.to_owned(),
    ]);

    dispatch::run(args).unwrap();

    let trace_report = evaluate_trace_schema_jsonl(&trace_path).unwrap();
    let trace = fs::read_to_string(&trace_path).unwrap();
    assert!(trace_report.passed, "{:?}", trace_report.failures);
    assert_eq!(trace_report.checked_lines, 2);
    assert_eq!(trace_report.self_evolving_memory_writeback_events, 1);
    assert_eq!(
        trace_report.self_evolving_memory_writeback_attempted_records,
        3
    );
    assert_eq!(
        trace_report.self_evolving_memory_writeback_accepted_records,
        3
    );
    assert_eq!(
        trace_report.self_evolving_memory_writeback_records_before,
        0
    );
    assert_eq!(trace_report.self_evolving_memory_writeback_records_after, 4);
    assert_eq!(trace_report.self_evolving_memory_writeback_applied, 1);
    assert_eq!(
        trace_report.self_evolving_memory_writeback_applied_to_disk,
        1
    );
    assert!(trace.contains("\"schema\":\"rust-norion-self-evolving-memory-writeback-v1\""));
    assert!(trace.contains("\"operation\":\"runtime_writeback\""));
    let writeback_line = trace
        .lines()
        .find(|line| line.contains("rust-norion-self-evolving-memory-writeback-v1"))
        .unwrap();
    assert!(!writeback_line.contains(prompt));
    let sem_snapshot = fs::read_to_string(&sem_path).unwrap();
    assert!(!sem_snapshot.contains(prompt));
    let sem = rust_norion::SelfEvolvingMemoryStore::load_snapshot(&sem_path).unwrap();
    assert!(writeback_line.contains(&format!(
        "\"snapshot_before_digest\":\"{sem_before_digest}\""
    )));
    assert!(writeback_line.contains(&format!(
        "\"snapshot_digest\":\"{}\"",
        sem.snapshot_digest()
    )));
    assert!(writeback_line.contains(&format!(
        "\"disk_snapshot_digest\":\"{}\"",
        sem.snapshot_digest()
    )));
    assert_ne!(sem_before_digest, sem.snapshot_digest());
    assert_eq!(sem.episodes().len(), 1);
    assert_eq!(sem.heuristics().len(), 1);
    assert_eq!(sem.tool_reliability().len(), 1);
    assert_eq!(sem.tool_observations().len(), 1);
    let engine =
        rust_norion::NoironEngine::load_full_state(&memory_path, &experience_path, &adaptive_path)
            .unwrap();
    let state_report = rust_norion::StateInspectionReport::from_engine(&engine, 1);
    assert_eq!(
        state_report.self_evolving_memory_writeback_experience_count,
        1
    );
    assert_eq!(
        state_report.self_evolving_memory_writeback_attempted_records,
        3
    );
    assert_eq!(
        state_report.self_evolving_memory_writeback_accepted_records,
        3
    );
    assert_eq!(
        state_report.self_evolving_memory_writeback_applied_to_disk,
        1
    );
    assert!(state_report
        .summary_line()
        .contains("self_evolving_memory_writeback_applied_to_disk=1"));
    let source_case_digest = &sem.episodes()[0].source_case_digest;
    assert_eq!(sem.heuristics()[0].source_case_digest, *source_case_digest);
    assert_eq!(
        sem.tool_observations()[0].source_case_digest,
        *source_case_digest
    );
    assert!(writeback_line.contains(&format!("\"source_case_digest\":\"{source_case_digest}\"")));
    let sem_hints = sem
        .retrieve_context(&rust_norion::SelfEvolvingMemoryQuery {
            prompt: prompt.to_owned(),
            profile: TaskProfile::General,
            tags: Vec::new(),
            record_limit: 4,
            token_budget: 160,
        })
        .experience_hints();
    assert_eq!(sem_hints.len(), 3);
    assert!(
        sem_hints.iter().any(|hint| hint.contains("key_insights=4")),
        "{sem_hints:?}"
    );

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn ordinary_local_runtime_cli_reuses_self_evolving_writeback_on_next_run() {
    let dir = temp_asset_dir("ordinary-local-runtime-sem-loop");
    fs::create_dir_all(&dir).unwrap();
    let memory_path = dir.join("memory.ndkv");
    let experience_path = dir.join("experience.ndkv");
    let adaptive_path = dir.join("adaptive.ndkv");
    let sem_path = experience_path.with_extension("self-evolving-memory.tsv");
    let trace_path = dir.join("trace.jsonl");
    let prompt = "safe ordinary runtime sem loop prompt";
    let make_args = || {
        Args::parse(vec![
            "--local-runtime".to_owned(),
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
            prompt.to_owned(),
        ])
    };

    dispatch::run(make_args()).unwrap();
    dispatch::run(make_args()).unwrap();

    let saved = NoironEngine::load_full_state(&memory_path, &experience_path, &adaptive_path)
        .expect("saved ordinary runtime loop state");
    let latest = saved.experience.records().last().unwrap();
    let trace_report = evaluate_trace_schema_jsonl(&trace_path).unwrap();
    let trace = fs::read_to_string(&trace_path).unwrap();
    let sem = rust_norion::SelfEvolvingMemoryStore::load_snapshot(&sem_path).unwrap();
    let sem_hints = sem
        .retrieve_context(&rust_norion::SelfEvolvingMemoryQuery {
            prompt: prompt.to_owned(),
            profile: TaskProfile::General,
            tags: Vec::new(),
            record_limit: 4,
            token_budget: 160,
        })
        .experience_hints();

    assert_eq!(saved.experience.len(), 2);
    assert!(latest.lesson.contains("4 experience hints"));
    assert!(trace_report.passed, "{:?}", trace_report.failures);
    assert_eq!(trace_report.checked_lines, 4);
    assert_eq!(trace_report.self_evolving_memory_writeback_events, 2);
    assert_eq!(
        trace_report.self_evolving_memory_writeback_applied_to_disk,
        2
    );
    assert_eq!(trace_report.used_experiences, 1);
    assert!(
        trace_report.compute_budget_self_evolving_memory_fusion_saved_tokens > 0,
        "{}",
        trace_report.summary_line()
    );
    assert!(trace
        .lines()
        .filter(|line| line.contains("rust-norion-self-evolving-memory-writeback-v1"))
        .all(|line| !line.contains(prompt)));
    assert_eq!(sem.episodes().len(), 2);
    assert_eq!(sem.heuristics().len(), 2);
    assert_eq!(sem.tool_observations().len(), 2);
    assert!(sem_hints.len() >= 3, "{sem_hints:?}");

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn ordinary_local_runtime_cli_copies_writeback_to_separate_trace_gate() {
    let dir = temp_asset_dir("ordinary-local-runtime-sem-writeback-gate");
    fs::create_dir_all(&dir).unwrap();
    let memory_path = dir.join("memory.ndkv");
    let experience_path = dir.join("experience.ndkv");
    let adaptive_path = dir.join("adaptive.ndkv");
    let trace_path = dir.join("trace.jsonl");
    let trace_gate_path = dir.join("trace-gate.jsonl");

    let args = Args::parse(vec![
        "--local-runtime".to_owned(),
        "--memory".to_owned(),
        memory_path.display().to_string(),
        "--experience".to_owned(),
        experience_path.display().to_string(),
        "--adaptive".to_owned(),
        adaptive_path.display().to_string(),
        "--trace".to_owned(),
        trace_path.display().to_string(),
        "--trace-schema-gate".to_owned(),
        trace_gate_path.display().to_string(),
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
        "safe ordinary runtime separate writeback gate prompt".to_owned(),
    ]);

    dispatch::run(args).unwrap();

    let trace_report = evaluate_trace_schema_jsonl(&trace_path).unwrap();
    let gate_report = evaluate_trace_schema_jsonl(&trace_gate_path).unwrap();
    let gate_trace = fs::read_to_string(&trace_gate_path).unwrap();
    assert!(trace_report.passed, "{:?}", trace_report.failures);
    assert!(gate_report.passed, "{:?}", gate_report.failures);
    assert_eq!(trace_report.checked_lines, 2);
    assert_eq!(gate_report.checked_lines, 2);
    assert_eq!(gate_report.self_evolving_memory_writeback_events, 1);
    assert_eq!(
        gate_report.self_evolving_memory_writeback_applied_to_disk,
        1
    );
    assert!(gate_report.compute_budget_events >= 1);
    assert!(gate_trace.contains("\"schema\":\"rust-norion-trace-v1\""));
    assert!(gate_trace.contains("\"schema\":\"rust-norion-self-evolving-memory-writeback-v1\""));

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn ordinary_local_runtime_cli_merges_duplicate_self_evolving_episodes() {
    let dir = temp_asset_dir("ordinary-local-runtime-sem-merge");
    fs::create_dir_all(&dir).unwrap();
    let memory_path = dir.join("memory.ndkv");
    let experience_path = dir.join("experience.ndkv");
    let adaptive_path = dir.join("adaptive.ndkv");
    let sem_path = experience_path.with_extension("self-evolving-memory.tsv");
    let prompt = "private ordinary runtime sem duplicate prompt";

    for _ in 0..2 {
        dispatch::run(Args::parse(vec![
            "--local-runtime".to_owned(),
            "--memory".to_owned(),
            memory_path.display().to_string(),
            "--experience".to_owned(),
            experience_path.display().to_string(),
            "--adaptive".to_owned(),
            adaptive_path.display().to_string(),
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
            prompt.to_owned(),
        ]))
        .unwrap();
    }

    let sem_snapshot = fs::read_to_string(&sem_path).unwrap();
    assert!(!sem_snapshot.contains(prompt));
    let sem = rust_norion::SelfEvolvingMemoryStore::load_snapshot(&sem_path).unwrap();
    assert_eq!(sem.episodes().len(), 2);
    assert_eq!(
        sem.episodes()
            .iter()
            .filter(|episode| episode.active)
            .count(),
        1
    );
    assert!(sem
        .episodes()
        .iter()
        .any(|episode| episode.merged_into.is_some()));

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn local_learning_smoke_cli_writes_state_and_trace_gate_evidence() {
    let dir = temp_asset_dir("local-learning-smoke-cli");
    fs::create_dir_all(&dir).unwrap();
    let memory_path = dir.join("memory.ndkv");
    let experience_path = dir.join("experience.ndkv");
    let self_evolving_memory_path = experience_path.with_extension("self-evolving-memory.tsv");
    let adaptive_path = dir.join("adaptive.ndkv");
    let trace_path = dir.join("trace.jsonl");
    let prompt = "safe local learning smoke prompt";
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
        prompt.to_owned(),
    ]);

    let passed = crate::cli::local_learning_smoke::run_local_learning_smoke_cli(&args)
        .expect("local learning smoke");
    let trace_report = evaluate_trace_schema_jsonl(&trace_path).unwrap();
    let trace = fs::read_to_string(&trace_path).unwrap();

    assert!(passed);
    assert!(trace_report.passed, "{:?}", trace_report.failures);
    assert_eq!(trace_report.checked_lines, 6);
    assert_self_evolving_memory_store_trace(&trace_report, 1);
    assert_self_evolving_memory_writeback_trace(&trace_report, 1);
    assert!(trace_report.reasoning_genome_events > 0);
    assert!(trace_report.reasoning_genome_active_genes > 0);
    assert!(trace_report.reasoning_genome_splice_segments > 0);
    assert!(trace_report.reasoning_genome_splice_exons > 0);
    assert!(trace_report.adaptive_routing_saved_tokens > 0);
    assert_eq!(trace_report.task_hierarchy_events, 1);
    assert!(trace_report.task_hierarchy_mutation_records > 0);
    assert!(trace_report.task_hierarchy_route_pressure_milli > 0);
    assert!(trace_report.task_hierarchy_compute_reduction_milli > 0);
    assert_eq!(trace_report.fht_dke_events, 1);
    assert_eq!(trace_report.fht_dke_enabled, 1);
    assert!(trace_report.fht_dke_total_tokens > 0);
    assert!(trace_report.fht_dke_routed_tokens > 0);
    assert_eq!(trace_report.fht_dke_token_split_invalid, 0);
    assert!(trace_report.compute_budget_threshold_delta_milli > 0);
    assert!(trace_report.compute_budget_saved_tokens > 0);
    assert!(trace_report.compute_budget_avoided_tokens > 0);
    assert!(trace_report.kv_fusion_saved_tokens > 0);
    assert_eq!(trace_report.process_reward_events, 1);
    assert_eq!(trace_report.process_reward_positive, 1);
    assert_eq!(trace_report.process_reward_reinforce, 1);
    assert_eq!(trace_report.process_reward_hold, 0);
    assert_eq!(trace_report.process_reward_penalize, 0);
    assert!(trace_report.process_reward_total_milli > 0);
    assert_eq!(trace_report.live_evolution_events, 1);
    assert!(trace_report.live_router_threshold_delta_milli > 0);
    assert!(trace_report.live_hierarchy_weight_delta_milli > 0);
    assert_eq!(trace_report.live_online_reward_feedbacks, 1);
    assert!(trace_report.live_stored_memory_updates > 0);
    assert!(trace.contains("\"case\":\"local_learning_smoke\""));
    assert!(trace.contains("\"runtime_tokens\":{"));
    assert!(trace.contains("\"memory\":{"));
    assert!(trace.contains("\"process_reward\":{"));
    assert!(trace.contains("\"reasoning_genome\":{"));
    assert!(trace.contains("\"live_online_reward_feedbacks\":1"));
    assert!(trace.contains("\"adaptive_routing\":{"));
    assert!(trace.contains("\"task_hierarchy\":{"));
    assert!(trace.contains("\"compute_budget\":{"));
    assert!(trace.contains("\"kv_fusion\":{"));
    assert!(trace.contains("\"live_evolution\":{"));
    assert!(trace.contains("\"evolution_ledger\":{"));
    assert!(trace.contains("\"live_inference_runs\":1"));
    assert!(trace.contains("\"schema\":\"rust-norion-self-evolving-memory-store-v1\""));
    assert!(trace.contains("\"operation\":\"retrieval\""));
    assert!(trace.contains("\"operation\":\"maintenance\""));
    assert!(trace.contains("\"operation\":\"admission_preview\""));
    assert!(trace.contains("\"operation\":\"consolidation_preview\""));
    assert!(trace.contains("\"schema\":\"rust-norion-self-evolving-memory-writeback-v1\""));
    assert!(trace.contains("\"operation\":\"runtime_writeback\""));
    assert!(memory_path.exists());
    assert!(experience_path.exists());
    assert!(self_evolving_memory_path.exists());
    assert!(adaptive_path.exists());
    let self_evolving_memory =
        rust_norion::SelfEvolvingMemoryStore::load_snapshot(&self_evolving_memory_path)
            .expect("self-evolving local learning memory");
    assert_eq!(self_evolving_memory.episodes().len(), 1);
    assert_eq!(self_evolving_memory.heuristics().len(), 1);
    assert_eq!(self_evolving_memory.tool_reliability().len(), 1);
    assert_eq!(self_evolving_memory.tool_observations().len(), 1);
    let sem_hints_before_second = self_evolving_memory
        .retrieve_context(&rust_norion::SelfEvolvingMemoryQuery {
            prompt: prompt.to_owned(),
            profile: args.profile,
            tags: vec![
                "local_learning_smoke".to_owned(),
                "fht-dke".to_owned(),
                "noiron".to_owned(),
                "runtime".to_owned(),
            ],
            record_limit: 4,
            token_budget: 160,
        })
        .experience_hints();
    assert!(!sem_hints_before_second.is_empty());
    assert!(sem_hints_before_second
        .iter()
        .all(|hint| !hint.contains(prompt)));
    assert!(
        sem_hints_before_second
            .iter()
            .any(|hint| hint.contains("key_insights=4")),
        "{sem_hints_before_second:?}"
    );
    let saved = NoironEngine::load_full_state(&memory_path, &experience_path, &adaptive_path)
        .expect("saved local learning state");
    assert_eq!(saved.evolution_ledger.live_inference_runs, 1);
    assert_eq!(saved.experience.len(), 1);
    let saved_record = &saved.experience.records()[0];
    let saved_live = saved_record.live_evolution;
    let saved_adaptive = saved.adaptive_state();
    assert!(saved_adaptive.router.observations > 0);
    assert!(saved_adaptive.router.profile_observations.get(args.profile) > 0);
    assert!(
        saved_adaptive
            .hierarchy
            .profile_observations
            .get(args.profile)
            > 0
    );
    assert!((saved_adaptive.router.threshold - saved_record.router_threshold_after).abs() < 0.0001);
    assert_hierarchy_matches(saved_adaptive.hierarchy.current, saved_record.hierarchy);
    assert!(saved_live.has_evidence());
    assert!(saved_record
        .process_reward
        .notes
        .iter()
        .any(|note| note.starts_with("fht_dke_budget:")));
    let state_report = rust_norion::StateInspectionReport::from_engine(&saved, 2);
    assert!(state_report.fht_dke_budget_experience_count >= 1);
    assert!(state_report.fht_dke_enabled_experience_count >= 1);
    assert!(state_report.fht_dke_total_tokens > 0);
    assert!(state_report.fht_dke_routed_tokens > 0);
    assert_eq!(state_report.fht_dke_token_split_invalid_count, 0);
    assert!(state_report
        .summary_line()
        .contains("fht_dke_budget_experiences="));
    assert_eq!(
        trace_report.live_router_threshold_delta_milli,
        smoke_test_milli(saved_live.router_threshold_delta)
    );
    assert_eq!(
        trace_report.live_hierarchy_weight_delta_milli,
        smoke_test_milli(saved_live.hierarchy_weight_delta)
    );
    assert!(saved_record.process_reward.total > 0.0);
    assert!(saved_live.online_reward_feedbacks > 0);
    assert!(!saved_record.stored_runtime_kv_memory_ids.is_empty());
    assert_eq!(
        saved_live.stored_runtime_kv_memories,
        saved_record.stored_runtime_kv_memory_ids.len()
    );
    assert!(saved_record.stored_runtime_kv_memory_ids.iter().all(|id| {
        saved
            .cache
            .entries()
            .iter()
            .any(|entry| entry.id == *id && entry.key.starts_with("runtime_kv:"))
    }));
    assert_eq!(
        saved.evolution_ledger.live_memory_updates(),
        saved_live.memory_updates() as u64
    );
    assert_eq!(
        saved.evolution_ledger.live_stored_memory_updates(),
        saved_live.stored_memory_updates() as u64
    );

    let passed = crate::cli::local_learning_smoke::run_local_learning_smoke_cli(&args)
        .expect("second local learning smoke");
    let trace_report = evaluate_trace_schema_jsonl(&trace_path).unwrap();
    let trace = fs::read_to_string(&trace_path).unwrap();

    assert!(passed);
    assert!(trace_report.passed, "{:?}", trace_report.failures);
    assert_eq!(trace_report.checked_lines, 12);
    assert_self_evolving_memory_store_trace(&trace_report, 2);
    assert_self_evolving_memory_writeback_trace(&trace_report, 2);
    assert_eq!(trace_report.used_experiences, 1);
    assert_eq!(trace_report.imported_kv_blocks, 1);
    assert!(trace_report.reasoning_genome_events > 0);
    assert!(trace_report.reasoning_genome_active_genes > 0);
    assert!(trace_report.reasoning_genome_splice_segments > 0);
    assert!(trace_report.reasoning_genome_splice_exons > 0);
    assert!(trace_report.adaptive_routing_saved_tokens > 0);
    assert_eq!(trace_report.task_hierarchy_events, 2);
    assert!(trace_report.task_hierarchy_mutation_records > 0);
    assert!(trace_report.task_hierarchy_route_pressure_milli > 0);
    assert!(trace_report.task_hierarchy_compute_reduction_milli > 0);
    assert_eq!(trace_report.fht_dke_events, 2);
    assert_eq!(trace_report.fht_dke_enabled, 2);
    assert!(trace_report.fht_dke_total_tokens > 0);
    assert!(trace_report.fht_dke_routed_tokens > 0);
    assert_eq!(trace_report.fht_dke_token_split_invalid, 0);
    assert!(trace_report.compute_budget_threshold_delta_milli > 0);
    assert!(trace_report.compute_budget_saved_tokens > 0);
    assert!(trace_report.compute_budget_self_evolving_memory_fusion_saved_tokens > 0);
    assert!(trace_report.compute_budget_avoided_tokens > 0);
    assert!(trace_report.kv_fusion_saved_tokens > 0);
    assert_eq!(trace_report.process_reward_events, 2);
    assert_eq!(trace_report.process_reward_positive, 2);
    assert_eq!(trace_report.process_reward_reinforce, 2);
    assert_eq!(trace_report.process_reward_hold, 0);
    assert_eq!(trace_report.process_reward_penalize, 0);
    assert!(trace_report.process_reward_total_milli > 0);
    assert_eq!(trace_report.live_evolution_events, 2);
    assert!(trace_report.live_router_threshold_delta_milli > 0);
    assert!(trace_report.live_hierarchy_weight_delta_milli > 0);
    assert_eq!(trace_report.live_online_reward_feedbacks, 2);
    assert!(trace_report.live_memory_updates > 0);
    assert!(trace_report.live_stored_memory_updates > 0);
    assert!(trace.contains("\"live_inference_runs\":2"));
    assert!(trace.contains("\"replay_runs\":1"));
    assert!(trace.contains("\"live_evolution_items\":1"));
    assert!(trace.contains("\"live_evolution_online_reward_feedbacks\":1"));
    assert!(trace.contains("\"used_experiences\":1"));
    assert!(trace.contains("\"imported_kv_blocks\":1"));
    assert!(trace.contains("\"operation\":\"consolidation_preview\""));

    let saved = NoironEngine::load_full_state(&memory_path, &experience_path, &adaptive_path)
        .expect("saved local learning replay state");
    assert_eq!(saved.evolution_ledger.live_inference_runs, 2);
    assert_eq!(saved.evolution_ledger.replay_runs, 1);
    assert!(saved.evolution_ledger.replay_items >= 1);
    assert!(saved.evolution_ledger.replay_live_evolution_items >= 1);
    assert!(
        saved
            .evolution_ledger
            .replay_live_evolution_online_reward_feedbacks
            >= 1
    );
    assert!(
        saved
            .evolution_ledger
            .replay_live_evolution_online_reward_strength
            > 0.0
    );
    assert_eq!(saved.experience.len(), 2);
    assert_eq!(
        saved.evolution_ledger.live_memory_updates(),
        saved
            .experience
            .records()
            .iter()
            .map(|record| record.live_evolution.memory_updates() as u64)
            .sum()
    );
    assert_eq!(
        saved.evolution_ledger.live_stored_memory_updates(),
        saved
            .experience
            .records()
            .iter()
            .map(|record| record.live_evolution.stored_memory_updates() as u64)
            .sum()
    );
    assert_eq!(
        saved.evolution_ledger.live_stored_runtime_kv_memories,
        saved
            .experience
            .records()
            .iter()
            .map(|record| record.live_evolution.stored_runtime_kv_memories as u64)
            .sum()
    );
    assert!(saved
        .cache
        .entries()
        .iter()
        .any(|entry| entry.key.starts_with("runtime_kv:")));
    let saved_adaptive = saved.adaptive_state();
    let latest_record = saved.experience.records().last().unwrap();
    let expected_runtime_experience_hints = 1usize.saturating_add(sem_hints_before_second.len());
    assert!(latest_record.lesson.contains(&format!(
        "{expected_runtime_experience_hints} experience hints"
    )));
    assert!(saved_adaptive.router.observations >= 2);
    assert!(saved_adaptive.router.profile_observations.get(args.profile) >= 2);
    assert!(
        saved_adaptive
            .hierarchy
            .profile_observations
            .get(args.profile)
            >= 2
    );
    assert!(
        (saved_adaptive.router.threshold - latest_record.router_threshold_after).abs() < 0.0001
    );
    assert_hierarchy_matches(saved_adaptive.hierarchy.current, latest_record.hierarchy);
    assert!(
        trace_report.live_router_threshold_delta_milli
            >= smoke_test_live_router_threshold_delta_milli(saved.experience.records())
    );
    assert!(
        trace_report.live_hierarchy_weight_delta_milli
            >= smoke_test_live_hierarchy_weight_delta_milli(saved.experience.records())
    );
    let self_evolving_memory =
        rust_norion::SelfEvolvingMemoryStore::load_snapshot(&self_evolving_memory_path)
            .expect("reloaded self-evolving local learning memory");
    assert_eq!(self_evolving_memory.episodes().len(), 2);
    assert_eq!(self_evolving_memory.heuristics().len(), 2);
    assert_eq!(self_evolving_memory.tool_reliability().len(), 1);
    assert_eq!(self_evolving_memory.tool_observations().len(), 2);
    assert!(self_evolving_memory
        .episodes()
        .iter()
        .any(|episode| !episode.active));

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn local_learning_smoke_cli_copies_runtime_and_writeback_to_separate_trace_gate() {
    let dir = temp_asset_dir("local-learning-smoke-cli-separate-gate");
    fs::create_dir_all(&dir).unwrap();
    let memory_path = dir.join("memory.ndkv");
    let experience_path = dir.join("experience.ndkv");
    let adaptive_path = dir.join("adaptive.ndkv");
    let trace_path = dir.join("trace.jsonl");
    let trace_gate_path = dir.join("trace-gate.jsonl");
    let args = Args::parse(vec![
        "--local-learning-smoke".to_owned(),
        "--memory".to_owned(),
        memory_path.display().to_string(),
        "--experience".to_owned(),
        experience_path.display().to_string(),
        "--adaptive".to_owned(),
        adaptive_path.display().to_string(),
        "--trace".to_owned(),
        trace_path.display().to_string(),
        "--trace-schema-gate".to_owned(),
        trace_gate_path.display().to_string(),
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
        "safe local learning separate gate prompt".to_owned(),
    ]);

    let passed = crate::cli::local_learning_smoke::run_local_learning_smoke_cli(&args)
        .expect("local learning smoke with separate trace gate");
    let trace_report = evaluate_trace_schema_jsonl(&trace_path).unwrap();
    let gate_report = evaluate_trace_schema_jsonl(&trace_gate_path).unwrap();
    let gate_trace = fs::read_to_string(&trace_gate_path).unwrap();

    assert!(passed);
    assert!(trace_report.passed, "{:?}", trace_report.failures);
    assert!(gate_report.passed, "{:?}", gate_report.failures);
    assert_eq!(trace_report.checked_lines, 6);
    assert_eq!(gate_report.checked_lines, 6);
    assert_self_evolving_memory_store_trace(&gate_report, 1);
    assert_self_evolving_memory_writeback_trace(&gate_report, 1);
    assert!(gate_report.reasoning_genome_events > 0);
    assert!(gate_trace.contains("\"case\":\"local_learning_smoke\""));
    assert!(gate_trace.contains("\"schema\":\"rust-norion-self-evolving-memory-writeback-v1\""));

    fs::remove_dir_all(dir).unwrap();
}

fn assert_hierarchy_matches(
    left: rust_norion::HierarchyWeights,
    right: rust_norion::HierarchyWeights,
) {
    assert!((left.global - right.global).abs() < 0.0001);
    assert!((left.local - right.local).abs() < 0.0001);
    assert!((left.convolution - right.convolution).abs() < 0.0001);
}

fn assert_self_evolving_memory_store_trace(
    report: &rust_norion::TraceSchemaGateReport,
    runs: usize,
) {
    assert_eq!(report.self_evolving_memory_store_events, runs * 4);
    assert_eq!(report.self_evolving_memory_store_retrieval_events, runs);
    assert_eq!(report.self_evolving_memory_store_maintenance_events, runs);
    assert_eq!(
        report.self_evolving_memory_store_admission_preview_events,
        runs
    );
    assert_eq!(report.self_evolving_memory_store_consolidation_events, runs);
    assert!(report.self_evolving_memory_store_consolidation_actions >= runs * 3);
    assert!(report.self_evolving_memory_store_merge_previews >= runs);
    assert!(report.self_evolving_memory_store_decay_previews >= runs);
    assert!(report.self_evolving_memory_store_tombstone_previews >= runs);
    assert!(report.self_evolving_memory_store_contexts >= runs);
    assert!(report.self_evolving_memory_store_saved_tokens > 0);
    assert!(report.self_evolving_memory_store_maintenance_actions >= runs);
    assert!(report.self_evolving_memory_store_admission_candidates >= runs);
    assert_eq!(report.self_evolving_memory_store_write_allowed, 0);
    assert_eq!(report.self_evolving_memory_store_durable_write_allowed, 0);
    assert_eq!(report.self_evolving_memory_store_applied, 0);
    assert_eq!(report.self_evolving_memory_store_applied_to_disk, 0);
}

fn assert_self_evolving_memory_writeback_trace(
    report: &rust_norion::TraceSchemaGateReport,
    runs: usize,
) {
    assert_eq!(report.self_evolving_memory_writeback_events, runs);
    assert_eq!(
        report.self_evolving_memory_writeback_source_case_digests,
        runs
    );
    assert_eq!(
        report.self_evolving_memory_writeback_attempted_records,
        runs * 3
    );
    assert_eq!(
        report.self_evolving_memory_writeback_accepted_records,
        runs * 3
    );
    assert!(report.self_evolving_memory_writeback_records_after >= runs * 4);
    assert!(report.self_evolving_memory_writeback_maintenance_actions >= runs);
    assert_eq!(report.self_evolving_memory_writeback_write_allowed, runs);
    assert_eq!(
        report.self_evolving_memory_writeback_durable_write_allowed,
        runs
    );
    assert_eq!(report.self_evolving_memory_writeback_applied, runs);
    assert_eq!(report.self_evolving_memory_writeback_applied_to_disk, runs);
}

fn smoke_test_live_router_threshold_delta_milli(
    records: &[rust_norion::ExperienceRecord],
) -> usize {
    records
        .iter()
        .map(|record| smoke_test_milli(record.live_evolution.router_threshold_delta))
        .sum()
}

fn smoke_test_live_hierarchy_weight_delta_milli(
    records: &[rust_norion::ExperienceRecord],
) -> usize {
    records
        .iter()
        .map(|record| smoke_test_milli(record.live_evolution.hierarchy_weight_delta))
        .sum()
}

fn smoke_test_milli(value: f32) -> usize {
    if value.is_finite() {
        (value.clamp(0.0, 1.0) * 1000.0).round() as usize
    } else {
        0
    }
}

#[test]
fn local_learning_smoke_cli_replays_experience_without_requiring_missing_runtime_kv() {
    let dir = temp_asset_dir("local-learning-smoke-cli-no-runtime-kv");
    fs::create_dir_all(&dir).unwrap();
    let memory_path = dir.join("memory.ndkv");
    let experience_path = dir.join("experience.ndkv");
    let self_evolving_memory_path = experience_path.with_extension("self-evolving-memory.tsv");
    let adaptive_path = dir.join("adaptive.ndkv");
    let trace_path = dir.join("trace.jsonl");
    let prompt = "safe local learning smoke prompt without prior runtime kv";
    let mut engine = NoironEngine::new();
    engine.experience.record(rust_norion::ExperienceInput {
        prompt: prompt.to_owned(),
        profile: TaskProfile::Coding,
        lesson: "reuse smoke learning without runtime kv import evidence".to_owned(),
        quality: 0.92,
        contradictions: Vec::new(),
        reflection_issues: Vec::new(),
        revision_actions: Vec::new(),
        stored_memory_id: None,
        router_threshold_after: 0.55,
        stream_windows: 2,
        route_budget: rust_norion::RouteBudget {
            threshold: 0.55,
            attention_tokens: 1,
            fast_tokens: 3,
            attention_fraction: 0.25,
        },
        hierarchy: rust_norion::HierarchyWeights::new(0.2, 0.6, 0.2),
        used_memory_ids: Vec::new(),
        gist_records: Vec::new(),
        gist_memory_ids: Vec::new(),
        stored_runtime_kv_memory_ids: Vec::new(),
        runtime_diagnostics: rust_norion::RuntimeDiagnostics::default(),
        runtime_token_metrics: Default::default(),
        process_reward: rust_norion::ProcessRewardReport {
            total: 0.90,
            action: rust_norion::RewardAction::Reinforce,
            components: Default::default(),
            notes: Vec::new(),
        },
        live_evolution: Default::default(),
    });
    engine
        .save_full_state(&memory_path, &experience_path, &adaptive_path)
        .unwrap();
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
        prompt.to_owned(),
    ]);

    let passed = crate::cli::local_learning_smoke::run_local_learning_smoke_cli(&args)
        .expect("local learning smoke without prior runtime kv");
    let trace_report = evaluate_trace_schema_jsonl(&trace_path).unwrap();
    let trace = fs::read_to_string(&trace_path).unwrap();

    assert!(passed);
    assert!(trace_report.passed, "{:?}", trace_report.failures);
    assert_eq!(trace_report.checked_lines, 6);
    assert_self_evolving_memory_store_trace(&trace_report, 1);
    assert_self_evolving_memory_writeback_trace(&trace_report, 1);
    assert_eq!(trace_report.used_experiences, 1);
    assert_eq!(trace_report.imported_kv_blocks, 0);
    assert!(trace.contains("\"used_experiences\":1"));
    assert!(trace.contains("\"imported_kv_blocks\":0"));
    assert!(trace.contains("\"operation\":\"consolidation_preview\""));
    assert!(trace.contains("\"schema\":\"rust-norion-self-evolving-memory-writeback-v1\""));
    let self_evolving_memory =
        rust_norion::SelfEvolvingMemoryStore::load_snapshot(&self_evolving_memory_path)
            .expect("self-evolving local learning memory without runtime kv");
    assert_eq!(self_evolving_memory.episodes().len(), 1);
    assert_eq!(self_evolving_memory.heuristics().len(), 1);
    assert_eq!(self_evolving_memory.tool_observations().len(), 1);

    fs::remove_dir_all(dir).unwrap();
}
