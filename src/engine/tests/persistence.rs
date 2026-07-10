use super::*;

#[test]
fn inference_tracks_tier_migrations_across_runs() {
    let mut cache = KvFusionCache::new();
    store_local_memory(
        &mut cache,
        "Rust Noiron tiered memory",
        vec![1.0, 0.0, 0.0],
        1.0,
    );
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
    let prompt = "Rust router feedback";
    let memory_id = store_local_memory(
        &mut engine.cache,
        "router feedback experience anchor",
        TextEmbedder::default().embed(prompt),
        0.9,
    );
    engine.experience.record(ExperienceInput {
        prompt: prompt.to_owned(),
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
        used_memory_ids: vec![memory_id],
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
        InferenceRequest::new(prompt, TaskProfile::Coding),
        &mut backend,
    );

    assert_eq!(outcome.used_experiences.len(), 1);
    assert!(outcome.answer.contains("Experience hints"));
}

#[test]
fn heuristic_backend_uses_clean_gist_for_metadata_experience_hint() {
    let mut engine = NoironEngine::new();
    let prompt = "帮我用rust输出一段for循环代码";
    let memory_id = store_local_memory(
        &mut engine.cache,
        "rust for loop experience anchor",
        TextEmbedder::default().embed(prompt),
        0.9,
    );
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
        used_memory_ids: vec![memory_id],
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
        InferenceRequest::new(prompt, TaskProfile::Coding),
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
    assert!(
        TenantScopedKey::parse(&runtime_kv_entry.key)
            .is_some_and(|key| key.local_key.starts_with("runtime_kv:"))
    );

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
    assert!(second.used_memories.iter().any(|memory| {
        memory.id == runtime_kv_memory_id
            && TenantScopedKey::parse(&memory.key)
                .is_some_and(|key| key.local_key.starts_with("runtime_kv:"))
    }));
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

#[derive(Debug, Default)]
struct GenomeFeedbackBackend {
    prompts: Vec<String>,
    confidence: f32,
}

impl GenomeFeedbackBackend {
    fn with_confidence(confidence: f32) -> Self {
        Self {
            prompts: Vec::new(),
            confidence,
        }
    }
}

impl InferenceBackend for GenomeFeedbackBackend {
    fn generate(&mut self, context: GenerationContext<'_>) -> InferenceDraft {
        self.prompts.push(context.prompt.to_owned());
        InferenceDraft::new(
            "Rust Noiron DNA routing keeps retrieval, reflection, validation, and rollback grounded in the requested control path.",
            vec![ReasoningStep::new(
                "dna_feedback",
                "bounded feedback for genome evolution",
                self.confidence,
            )],
        )
    }
}

#[test]
fn genome_evolution_applies_persists_reloads_and_rolls_back() {
    let memory_path = temp_path("genome-loop-memory", "ndkv");
    let experience_path = temp_path("genome-loop-experience", "ndkv");
    let adaptive_path = temp_path("genome-loop-adaptive", "ndkv");
    let original_id = NoironEngine::new()
        .genome_runtime_state
        .active(TaskProfile::Coding)
        .id
        .clone();
    let authorization = GenomeEvolutionAuthorization::apply(
        crate::reasoning_genome::DnaEvolutionValidationEvidence::passing(),
        "operator:genome-loop-apply",
    );
    let mut engine = NoironEngine::new();
    let mut first_backend = GenomeFeedbackBackend::with_confidence(0.20);
    let first = engine.infer(
        InferenceRequest::new("Rust Noiron DNA routing feedback", TaskProfile::Coding)
            .with_genome_evolution_authorization(authorization),
        &mut first_backend,
    );

    assert!(first_backend.prompts[0].starts_with("[noiron-dna "));
    assert!(first.reasoning_frame_valid);
    assert!(!first.task_gene_cascade.genes.is_empty());
    assert_eq!(
        first.task_skill_gene.decision,
        crate::reasoning_genome::TaskSkillGeneDecision::AcceptPreview
    );
    assert!(first.task_skill_gene.activation_eligible);
    assert!(first.adaptive_route_plan.candidates > 0);
    assert_eq!(
        first.dna_writer_gate.decision,
        crate::writer_gate::UnifiedWriterGateDecision::ReadyForExplicitApply,
        "{} records={:?} controller={} candidates={:?} purpose={:?}",
        first.dna_writer_gate.summary_line(),
        first
            .dna_writer_gate
            .records
            .iter()
            .map(|record| record.summary_line())
            .collect::<Vec<_>>(),
        first.dna_evolution_controller.redacted_trace_line(),
        first
            .dna_evolution_controller
            .candidates
            .iter()
            .map(|candidate| (&candidate.intent, &candidate.reason_codes))
            .collect::<Vec<_>>(),
        first
            .gene_purpose_reviews
            .iter()
            .map(|review| review.summary_line())
            .collect::<Vec<_>>()
    );
    assert!(
        first.dna_apply_receipt.applied,
        "{}",
        first.dna_apply_receipt.reason
    );
    assert_eq!(first.dna_apply_receipt.generation_after, 1);
    let evolved_id = first.dna_apply_receipt.genome_id_after.clone();
    assert_ne!(evolved_id, original_id);
    let trace = crate::trace::trace_json_line(
        "Rust Noiron DNA routing feedback",
        TaskProfile::Coding,
        1,
        &first,
    );
    let trace_failures = crate::trace::evaluate_trace_schema_line(&trace);
    assert!(trace_failures.is_empty(), "{trace_failures:?}");

    engine
        .save_full_state(&memory_path, &experience_path, &adaptive_path)
        .unwrap();
    let mut restored =
        NoironEngine::load_full_state(&memory_path, &experience_path, &adaptive_path).unwrap();
    assert_eq!(
        restored
            .genome_runtime_state
            .generation(TaskProfile::Coding),
        1
    );
    assert_eq!(
        restored.genome_runtime_state.active(TaskProfile::Coding).id,
        evolved_id
    );

    let mut inherited_backend = GenomeFeedbackBackend::with_confidence(0.90);
    let inherited = restored.infer(
        InferenceRequest::new("Rust Noiron inherited DNA generation", TaskProfile::Coding),
        &mut inherited_backend,
    );
    assert_eq!(inherited.genome_generation_before, 1);
    assert_eq!(
        inherited.task_skill_gene.decision,
        crate::reasoning_genome::TaskSkillGeneDecision::HoldForEvidence
    );
    assert!(inherited_backend.prompts[0].contains("generation=1"));
    assert!(inherited_backend.prompts[0].contains(&evolved_id));

    let mut rollback_backend = GenomeFeedbackBackend::with_confidence(0.90);
    let rollback = restored.infer(
        InferenceRequest::new("Rust Noiron rollback DNA generation", TaskProfile::Coding)
            .with_genome_evolution_authorization(GenomeEvolutionAuthorization::rollback(
                crate::reasoning_genome::DnaEvolutionValidationEvidence::passing(),
                "operator:genome-loop-rollback",
            )),
        &mut rollback_backend,
    );
    assert!(
        rollback.dna_apply_receipt.applied,
        "{}",
        rollback.dna_apply_receipt.reason
    );
    assert!(rollback.dna_apply_receipt.rolled_back);
    assert_eq!(rollback.dna_apply_receipt.generation_after, 2);
    assert_eq!(
        restored.genome_runtime_state.active(TaskProfile::Coding).id,
        original_id
    );

    cleanup(memory_path);
    cleanup(experience_path);
    cleanup(adaptive_path);
}

#[test]
fn failed_genome_validation_keeps_generation_and_disk_state_unchanged() {
    let memory_path = temp_path("genome-failed-memory", "ndkv");
    let experience_path = temp_path("genome-failed-experience", "ndkv");
    let adaptive_path = temp_path("genome-failed-adaptive", "ndkv");
    let mut engine = NoironEngine::new();
    let original_id = engine
        .genome_runtime_state
        .active(TaskProfile::Coding)
        .id
        .clone();
    let mut backend = GenomeFeedbackBackend::with_confidence(0.20);
    let outcome = engine.infer(
        InferenceRequest::new("Rust Noiron rejected DNA feedback", TaskProfile::Coding)
            .with_genome_evolution_authorization(GenomeEvolutionAuthorization::apply(
                crate::reasoning_genome::DnaEvolutionValidationEvidence::failed_tests(),
                "operator:genome-loop-reject",
            )),
        &mut backend,
    );

    assert!(!outcome.dna_apply_receipt.applied);
    assert_eq!(
        engine.genome_runtime_state.generation(TaskProfile::Coding),
        0
    );
    assert_eq!(
        engine.genome_runtime_state.active(TaskProfile::Coding).id,
        original_id
    );
    engine
        .save_full_state(&memory_path, &experience_path, &adaptive_path)
        .unwrap();
    let restored =
        NoironEngine::load_full_state(&memory_path, &experience_path, &adaptive_path).unwrap();
    assert_eq!(
        restored
            .genome_runtime_state
            .generation(TaskProfile::Coding),
        0
    );
    assert_eq!(
        restored.genome_runtime_state.active(TaskProfile::Coding).id,
        original_id
    );

    cleanup(memory_path);
    cleanup(experience_path);
    cleanup(adaptive_path);
}
