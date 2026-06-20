use super::*;

#[test]
fn inference_tracks_tier_migrations_across_runs() {
    let mut cache = KvFusionCache::new();
    cache.store_or_fuse("Rust Noiron tiered memory", vec![1.0, 0.0, 0.0], 1.0);
    let mut engine = NoironEngine::with_cache(cache);
    let mut backend = HeuristicBackend;

    let first = engine.infer(
        InferenceRequest::new("Rust Noiron tiered memory", TaskProfile::Coding),
        &mut backend,
    );
    let second = engine.infer(
        InferenceRequest::new("Rust Noiron tiered memory", TaskProfile::Coding),
        &mut backend,
    );

    assert!(
        first
            .tier_migrations
            .iter()
            .any(|migration| migration.action == TierMigrationAction::New)
    );
    assert!(
        second
            .tier_migrations
            .iter()
            .any(|migration| migration.from.is_some())
    );
    assert!(
        second
            .tier_migrations
            .iter()
            .any(|migration| migration.action != TierMigrationAction::New)
    );
}

#[test]
fn inference_uses_relevant_experience() {
    let mut engine = NoironEngine::new();
    engine.experience.record(ExperienceInput {
        prompt: "Rust router feedback".to_owned(),
        profile: TaskProfile::Coding,
        lesson: "reuse token-window feedback lessons".to_owned(),
        quality: 0.9,
        contradictions: Vec::new(),
        reflection_issues: Vec::new(),
        revision_actions: Vec::new(),
        stored_memory_id: None,
        router_threshold_after: 0.55,
        stream_windows: 2,
        route_budget: RouteBudget {
            threshold: 0.55,
            attention_tokens: 1,
            fast_tokens: 3,
            attention_fraction: 0.25,
        },
        hierarchy: HierarchyWeights::new(0.2, 0.6, 0.2),
        used_memory_ids: Vec::new(),
        gist_records: Vec::new(),
        gist_memory_ids: Vec::new(),
        stored_runtime_kv_memory_ids: Vec::new(),
        runtime_diagnostics: RuntimeDiagnostics::default(),
        runtime_token_metrics: Default::default(),
        process_reward: ProcessRewardReport::default(),
        live_evolution: Default::default(),
    });
    let mut backend = HeuristicBackend;

    let outcome = engine.infer(
        InferenceRequest::new("Rust router feedback", TaskProfile::Coding),
        &mut backend,
    );

    assert_eq!(outcome.used_experiences.len(), 1);
    assert!(outcome.answer.contains("Experience hints"));
}

#[test]
fn heuristic_backend_uses_clean_gist_for_metadata_experience_hint() {
    let mut engine = NoironEngine::new();
    engine.experience.record(ExperienceInput {
        prompt: "Conversation transcript:\nuser: 帮我用rust输出一段for循环代码\nassistant:"
            .to_owned(),
        profile: TaskProfile::Coding,
        lesson: "accepted_pattern quality=0.778 overlap=0.640 issues=0 max_severity=info"
            .to_owned(),
        quality: 0.9,
        contradictions: Vec::new(),
        reflection_issues: Vec::new(),
        revision_actions: Vec::new(),
        stored_memory_id: None,
        router_threshold_after: 0.55,
        stream_windows: 2,
        route_budget: RouteBudget {
            threshold: 0.55,
            attention_tokens: 1,
            fast_tokens: 3,
            attention_fraction: 0.25,
        },
        hierarchy: HierarchyWeights::new(0.2, 0.6, 0.2),
        used_memory_ids: Vec::new(),
        gist_records: vec![crate::gist_memory::GistRecord {
            level: crate::gist_memory::GistLevel::Document,
            title: "Conversation transcript".to_owned(),
            summary: "这是一个 Rust for 循环代码示例，使用 for i in 0..10 并 println 输出"
                .to_owned(),
            source_tokens: 42,
            importance: 0.86,
        }],
        gist_memory_ids: Vec::new(),
        stored_runtime_kv_memory_ids: Vec::new(),
        runtime_diagnostics: RuntimeDiagnostics::default(),
        runtime_token_metrics: Default::default(),
        process_reward: ProcessRewardReport::default(),
        live_evolution: Default::default(),
    });
    let mut backend = HeuristicBackend;

    let outcome = engine.infer(
        InferenceRequest::new("帮我用rust输出一段for循环代码", TaskProfile::Coding),
        &mut backend,
    );

    assert_eq!(outcome.used_experiences.len(), 1);
    assert!(outcome.answer.contains("Rust for 循环代码示例"));
    assert!(!outcome.answer.contains("accepted_pattern"));
}

#[test]
fn full_state_roundtrip_reuses_memory_experience_and_runtime_kv() {
    let memory_path = temp_path("full-state-memory", "ndkv");
    let experience_path = temp_path("full-state-experience", "ndkv");
    let adaptive_path = temp_path("full-state-adaptive", "ndkv");
    let prompt = "Rust Noiron persistent runtime KV memory";

    let mut engine = NoironEngine::new();
    engine.set_memory_retention_policy(MemoryRetentionPolicy {
        stale_after: 11,
        decay_rate: 0.12,
        remove_below_strength: 0.08,
        remove_after_failures: 7,
    });
    engine.set_memory_compaction_policy(MemoryCompactionPolicy {
        similarity_threshold: 0.91,
        max_candidates: 64,
        max_merges: 4,
    });
    let mut first_backend = RuntimeBackend::new(LocalTransformerRuntime::default());
    let first = engine.infer(
        InferenceRequest::new(prompt, TaskProfile::Coding),
        &mut first_backend,
    );
    assert!(first.stored_memory_id.is_some());
    assert!(!first.stored_runtime_kv_memory_ids.is_empty());
    let runtime_kv_memory_id = first.stored_runtime_kv_memory_ids[0];
    let runtime_kv_entry = engine
        .cache
        .entries()
        .iter()
        .find(|entry| entry.id == runtime_kv_memory_id)
        .expect("stored runtime KV memory should be present before save")
        .clone();
    assert!(runtime_kv_entry.key.starts_with("runtime_kv:"));

    engine
        .save_full_state(&memory_path, &experience_path, &adaptive_path)
        .unwrap();

    let mut restored =
        NoironEngine::load_full_state(&memory_path, &experience_path, &adaptive_path).unwrap();
    assert_eq!(restored.memory_retention_policy.stale_after, 11);
    assert!((restored.memory_retention_policy.decay_rate - 0.12).abs() < 0.0001);
    assert_eq!(restored.memory_compaction_policy.max_candidates, 64);
    assert_eq!(restored.memory_compaction_policy.max_merges, 4);
    assert_eq!(
        restored.evolution_ledger.replay_runs,
        engine.evolution_ledger.replay_runs
    );
    assert_eq!(
        restored.evolution_ledger.live_inference_runs,
        engine.evolution_ledger.live_inference_runs
    );
    assert_eq!(
        restored.evolution_ledger.live_stored_memory_updates(),
        engine.evolution_ledger.live_stored_memory_updates()
    );
    let restored_runtime_kv_entry = restored
        .cache
        .entries()
        .iter()
        .find(|entry| entry.id == runtime_kv_memory_id)
        .expect("stored runtime KV memory should survive full-state reload");
    assert_eq!(restored_runtime_kv_entry.key, runtime_kv_entry.key);
    assert_eq!(
        restored_runtime_kv_entry.vector.len(),
        runtime_kv_entry.vector.len()
    );
    let restored_runtime_kv_vector = restored_runtime_kv_entry.vector.clone();
    let mut second_backend = RuntimeBackend::new(LocalTransformerRuntime::default());
    let second = restored.infer(
        InferenceRequest::new(prompt, TaskProfile::Coding),
        &mut second_backend,
    );

    assert!(!second.used_memories.is_empty());
    assert!(
        second.used_memories.iter().any(
            |memory| memory.id == runtime_kv_memory_id && memory.key.starts_with("runtime_kv:")
        )
    );
    assert!(!second.used_experiences.is_empty());
    let imported = second_backend.runtime().imported_kv_blocks();
    assert!(!imported.is_empty());
    assert_eq!(
        second.runtime_diagnostics.imported_kv_blocks,
        imported.len()
    );
    let imported_runtime_kv = imported
        .iter()
        .find(|block| restored_runtime_kv_vector.starts_with(&block.key))
        .expect("persisted runtime KV vector should be reconstructed as imported KV");
    assert_eq!(
        imported_runtime_kv.token_end,
        imported_runtime_kv.token_start + 1
    );
    assert!(second.answer.contains("imported"));

    cleanup(memory_path);
    cleanup(experience_path);
    cleanup(adaptive_path);
}

#[test]
fn inference_stream_monitor_uses_backend_tokens() {
    struct TokenBackend;

    impl InferenceBackend for TokenBackend {
        fn generate(&mut self, _context: GenerationContext<'_>) -> InferenceDraft {
            InferenceDraft::new(
                "easy hard",
                vec![ReasoningStep::new("tokens", "runtime token metadata", 0.9)],
            )
            .with_tokens(vec![
                DraftToken {
                    text: "easy".to_owned(),
                    logprob: Some(-0.1),
                    entropy: Some(0.1),
                },
                DraftToken {
                    text: "hard".to_owned(),
                    logprob: Some(-1.2),
                    entropy: Some(0.9),
                },
            ])
        }
    }

    let mut engine = NoironEngine::new();
    engine.stream_monitor = TokenStreamMonitor::new(2);
    let mut backend = TokenBackend;

    let outcome = engine.infer(
        InferenceRequest::new("runtime token metadata", TaskProfile::Coding),
        &mut backend,
    );

    assert_eq!(outcome.stream_reports.len(), 1);
    assert_eq!(outcome.stream_reports[0].observations[0].entropy, 0.1);
    assert_eq!(outcome.stream_reports[0].observations[1].entropy, 0.9);
}

#[test]
fn adaptive_state_restores_router_and_hierarchy() {
    let mut engine = NoironEngine::new();
    engine.router.observe(GenerationMetrics {
        perplexity: 4.0,
        semantic_consistency: 0.98,
        contradiction_count: 0,
        token_count: 8,
    });
    engine.hierarchy.adapt_to_profile(TaskProfile::Coding);
    engine.set_memory_retention_policy(MemoryRetentionPolicy {
        stale_after: 9,
        decay_rate: 0.18,
        remove_below_strength: 0.11,
        remove_after_failures: 6,
    });
    engine.set_memory_compaction_policy(MemoryCompactionPolicy {
        similarity_threshold: 0.89,
        max_candidates: 48,
        max_merges: 3,
    });
    let state = engine.adaptive_state();

    let mut restored = NoironEngine::new();
    restored.restore_adaptive_state(state);

    assert_eq!(restored.router.observations(), engine.router.observations());
    assert!((restored.router.threshold() - engine.router.threshold()).abs() < 0.0001);
    assert!((restored.hierarchy.current().local - engine.hierarchy.current().local).abs() < 0.0001);
    assert_eq!(restored.memory_retention_policy.stale_after, 9);
    assert!((restored.memory_retention_policy.decay_rate - 0.18).abs() < 0.0001);
    assert_eq!(restored.memory_compaction_policy.max_candidates, 48);
    assert_eq!(restored.memory_compaction_policy.max_merges, 3);
}

#[test]
fn replay_evolution_ledger_persists_through_full_state() {
    let memory_path = temp_path("ledger-memory", "ndkv");
    let experience_path = temp_path("ledger-experience", "ndkv");
    let adaptive_path = temp_path("ledger-adaptive", "ndkv");

    let mut engine = NoironEngine::new();
    let memory_id =
        engine
            .cache
            .store_or_fuse("persistent ledger memory", vec![1.0, 0.0, 0.0], 0.8);
    engine.experience.record(ExperienceInput {
        prompt: "persistent ledger replay".to_owned(),
        profile: TaskProfile::LongDocument,
        lesson: "persist control-plane evolution evidence across restarts".to_owned(),
        quality: 0.94,
        contradictions: Vec::new(),
        reflection_issues: Vec::new(),
        revision_actions: Vec::new(),
        stored_memory_id: Some(memory_id),
        router_threshold_after: 0.52,
        stream_windows: 2,
        route_budget: RouteBudget {
            threshold: 0.52,
            attention_tokens: 3,
            fast_tokens: 1,
            attention_fraction: 0.75,
        },
        hierarchy: HierarchyWeights::new(0.2, 0.2, 0.6),
        used_memory_ids: vec![memory_id],
        gist_records: Vec::new(),
        gist_memory_ids: Vec::new(),
        stored_runtime_kv_memory_ids: Vec::new(),
        runtime_diagnostics: RuntimeDiagnostics::default(),
        runtime_token_metrics: Default::default(),
        process_reward: ProcessRewardReport {
            total: 0.93,
            action: RewardAction::Reinforce,
            components: ProcessRewardComponents::default(),
            notes: vec![
                "recursive:chunks=4:merge_rounds=2:waves=2:parallel=2:runtime_calls=7".to_owned(),
            ],
        },
        live_evolution: Default::default(),
    });

    let report = engine.replay_experience(4);
    assert_eq!(report.applied, 1);
    assert_eq!(engine.evolution_ledger.replay_runs, 1);
    assert_eq!(engine.evolution_ledger.replay_items, 1);
    assert_eq!(engine.evolution_ledger.recursive_replay_items, 1);
    assert_eq!(engine.evolution_ledger.recursive_runtime_calls, 7);

    engine
        .save_full_state(&memory_path, &experience_path, &adaptive_path)
        .unwrap();
    let restored =
        NoironEngine::load_full_state(&memory_path, &experience_path, &adaptive_path).unwrap();

    assert_eq!(restored.evolution_ledger, engine.evolution_ledger);
    assert!(
        restored
            .evolution_ledger
            .summary_line()
            .contains("replay_runs=1")
    );

    cleanup(memory_path);
    cleanup(experience_path);
    cleanup(adaptive_path);
}
