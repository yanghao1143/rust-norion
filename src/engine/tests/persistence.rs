use super::super::state::FullStateSaveStage;
use super::*;
use crate::kv_quant::{QuantizationBits, QuantizedVector};
use crate::runtime::ModelRuntime;

const FULL_STATE_LOCK_CHILD_MEMORY: &str = "RUST_NORION_FULL_STATE_LOCK_CHILD_MEMORY";
const FULL_STATE_LOCK_CHILD_EXPERIENCE: &str = "RUST_NORION_FULL_STATE_LOCK_CHILD_EXPERIENCE";
const FULL_STATE_LOCK_CHILD_ADAPTIVE: &str = "RUST_NORION_FULL_STATE_LOCK_CHILD_ADAPTIVE";
const FULL_STATE_LOCK_READY: &str = "RUST_NORION_FULL_STATE_LOCK_READY";
const FULL_STATE_LOCK_RELEASE: &str = "RUST_NORION_FULL_STATE_LOCK_RELEASE";

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
    let original_runtime_kv = first_backend
        .runtime_mut()
        .export_kv()
        .unwrap()
        .into_iter()
        .next()
        .expect("runtime should retain the exported KV block before persistence");
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
        restored_runtime_kv_entry.vector,
        QuantizedVector::quantize(&runtime_kv_entry.vector, QuantizationBits::Four).dequantize()
    );
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
        .find(|block| {
            block.layer == original_runtime_kv.layer
                && block.head == original_runtime_kv.head
                && block.token_start == original_runtime_kv.token_start
                && block.token_end == original_runtime_kv.token_end
        })
        .expect("persisted runtime KV vector should be reconstructed as imported KV");
    let split = restored_runtime_kv_vector.len() / 2;
    assert_eq!(
        &imported_runtime_kv.key[..split],
        &restored_runtime_kv_vector[..split]
    );
    let runtime_kv_weight = second
        .infini_memory_plan
        .local_window()
        .iter()
        .chain(second.infini_memory_plan.global_memory())
        .find(|item| item.id == runtime_kv_memory_id)
        .map(|item| item.score.max(0.05))
        .expect("persisted runtime KV should retain its reuse weight");
    for (actual, stored) in imported_runtime_kv.value[..split]
        .iter()
        .zip(&restored_runtime_kv_vector[split..])
    {
        assert!((actual - stored * runtime_kv_weight).abs() < 0.0001);
    }
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

fn add_full_state_revision(engine: &mut NoironEngine, revision: u64) {
    let memory_id = store_local_memory(
        &mut engine.cache,
        &format!("atomic full-state memory revision {revision}"),
        vec![1.0; revision as usize + 2],
        0.7 + revision.min(2) as f32 * 0.05,
    );
    engine.experience.record(replay_memory_input(
        &format!("atomic full-state prompt revision {revision}"),
        &format!("atomic full-state lesson revision {revision}"),
        0.8,
        memory_id,
        Vec::new(),
        Vec::new(),
        RuntimeDiagnostics::default(),
        Vec::new(),
    ));
    engine.router.observe(GenerationMetrics {
        perplexity: 8.0 + revision as f32,
        semantic_consistency: 0.8,
        contradiction_count: 0,
        token_count: 32,
    });
    engine.hierarchy.adapt_to_profile(TaskProfile::Coding);
    engine.set_memory_retention_policy(MemoryRetentionPolicy {
        stale_after: 20 + revision,
        decay_rate: 0.05,
        remove_below_strength: 0.04,
        remove_after_failures: 4,
    });
}

fn assert_full_state_matches(actual: &NoironEngine, expected: &NoironEngine) {
    let actual_memory = actual
        .cache
        .entries()
        .iter()
        .map(|entry| (entry.id, entry.key.as_str()))
        .collect::<Vec<_>>();
    let expected_memory = expected
        .cache
        .entries()
        .iter()
        .map(|entry| (entry.id, entry.key.as_str()))
        .collect::<Vec<_>>();
    assert_eq!(actual_memory, expected_memory);

    let actual_experience = actual
        .experience
        .records()
        .iter()
        .map(|record| (record.id, record.prompt.as_str(), record.lesson.as_str()))
        .collect::<Vec<_>>();
    let expected_experience = expected
        .experience
        .records()
        .iter()
        .map(|record| (record.id, record.prompt.as_str(), record.lesson.as_str()))
        .collect::<Vec<_>>();
    assert_eq!(actual_experience, expected_experience);

    assert_eq!(
        actual.memory_retention_policy,
        expected.memory_retention_policy
    );
    assert_eq!(
        actual.memory_compaction_policy,
        expected.memory_compaction_policy
    );
    assert_eq!(actual.evolution_ledger, expected.evolution_ledger);
    assert_eq!(actual.genome_runtime_state, expected.genome_runtime_state);
    assert_eq!(actual.router.observations(), expected.router.observations());
    assert!((actual.router.threshold() - expected.router.threshold()).abs() < 0.0001);
    let actual_hierarchy = actual.hierarchy.current();
    let expected_hierarchy = expected.hierarchy.current();
    assert!((actual_hierarchy.global - expected_hierarchy.global).abs() < 0.0001);
    assert!((actual_hierarchy.local - expected_hierarchy.local).abs() < 0.0001);
    assert!((actual_hierarchy.convolution - expected_hierarchy.convolution).abs() < 0.0001);
}

fn full_state_generation_paths(
    memory_path: &Path,
    experience_path: &Path,
    adaptive_path: &Path,
    generation: u64,
) -> [PathBuf; 3] {
    [
        NoironEngine::full_state_generation_path_for_test(memory_path, generation).unwrap(),
        NoironEngine::full_state_generation_path_for_test(experience_path, generation).unwrap(),
        NoironEngine::full_state_generation_path_for_test(adaptive_path, generation).unwrap(),
    ]
}

fn appended_test_path(path: &Path, suffix: &str) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(suffix);
    PathBuf::from(value)
}

fn wait_for_test_marker(path: &Path) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    while !path.is_file() {
        assert!(
            std::time::Instant::now() < deadline,
            "timed out waiting for test marker: {}",
            path.display()
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

#[test]
fn full_state_writer_lock_child_process() {
    let Some(memory_path) = std::env::var_os(FULL_STATE_LOCK_CHILD_MEMORY).map(PathBuf::from)
    else {
        return;
    };
    let experience_path =
        PathBuf::from(std::env::var_os(FULL_STATE_LOCK_CHILD_EXPERIENCE).unwrap());
    let adaptive_path = PathBuf::from(std::env::var_os(FULL_STATE_LOCK_CHILD_ADAPTIVE).unwrap());
    let mut engine =
        NoironEngine::load_full_state(&memory_path, &experience_path, &adaptive_path).unwrap();
    add_full_state_revision(&mut engine, 2);
    engine
        .save_full_state(&memory_path, &experience_path, &adaptive_path)
        .unwrap();
}

#[test]
fn full_state_cross_process_writer_lock_allows_only_one_publisher() {
    let root = temp_path("atomic-writer-lock", "dir");
    let memory_path = root.join("memory.ndkv");
    let experience_path = root.join("experience.ndkv");
    let adaptive_path = root.join("adaptive.ndkv");
    let ready_path = temp_path("atomic-writer-lock-ready", "marker");
    let release_path = temp_path("atomic-writer-lock-release", "marker");
    let mut seed = NoironEngine::new();
    add_full_state_revision(&mut seed, 1);
    seed.save_full_state(&memory_path, &experience_path, &adaptive_path)
        .unwrap();
    let mut loser =
        NoironEngine::load_full_state(&memory_path, &experience_path, &adaptive_path).unwrap();
    add_full_state_revision(&mut loser, 3);

    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .arg("engine::tests::persistence::full_state_writer_lock_child_process")
        .arg("--exact")
        .arg("--nocapture")
        .env(FULL_STATE_LOCK_CHILD_MEMORY, &memory_path)
        .env(FULL_STATE_LOCK_CHILD_EXPERIENCE, &experience_path)
        .env(FULL_STATE_LOCK_CHILD_ADAPTIVE, &adaptive_path)
        .env(FULL_STATE_LOCK_READY, &ready_path)
        .env(FULL_STATE_LOCK_RELEASE, &release_path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    wait_for_test_marker(&ready_path);
    let loser_result = loser.save_full_state(&memory_path, &experience_path, &adaptive_path);
    File::create(&release_path).unwrap().sync_all().unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "child stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let error = loser_result.unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
    assert!(error.to_string().contains("full-state writer is busy"));
    assert_eq!(
        NoironEngine::read_full_state_manifest_for_test(&adaptive_path).unwrap(),
        (2, Some(1))
    );
    assert!(
        appended_test_path(
            &NoironEngine::full_state_manifest_path_for_test(&adaptive_path),
            ".lock"
        )
        .is_file()
    );
    let restarted =
        NoironEngine::load_full_state(&memory_path, &experience_path, &adaptive_path).unwrap();
    let prompts = restarted
        .experience
        .records()
        .iter()
        .map(|record| record.prompt.as_str())
        .collect::<Vec<_>>();
    assert!(prompts.contains(&"atomic full-state prompt revision 2"));
    assert!(!prompts.contains(&"atomic full-state prompt revision 3"));

    std::fs::remove_dir_all(root).unwrap();
    let _ = std::fs::remove_file(ready_path);
    let _ = std::fs::remove_file(release_path);
}

#[test]
fn full_state_stage_failures_restore_the_last_committed_generation() {
    let memory_path = temp_path("atomic-stage-memory", "ndkv");
    let experience_path = temp_path("atomic-stage-experience", "ndkv");
    let adaptive_path = temp_path("atomic-stage-adaptive", "ndkv");
    let mut engine = NoironEngine::new();
    add_full_state_revision(&mut engine, 1);
    engine
        .save_full_state(&memory_path, &experience_path, &adaptive_path)
        .unwrap();
    let baseline =
        NoironEngine::load_full_state(&memory_path, &experience_path, &adaptive_path).unwrap();

    for (index, stage) in [
        FullStateSaveStage::MemoryStaged,
        FullStateSaveStage::ExperienceStaged,
        FullStateSaveStage::AdaptiveStaged,
        FullStateSaveStage::ManifestStaged,
        FullStateSaveStage::CurrentBackedUp,
    ]
    .into_iter()
    .enumerate()
    {
        add_full_state_revision(&mut engine, index as u64 + 2);
        let error = engine
            .save_full_state_failing_after(&memory_path, &experience_path, &adaptive_path, stage)
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("injected full-state save failure")
        );
        assert_full_state_matches(&engine, &baseline);
        let restarted =
            NoironEngine::load_full_state(&memory_path, &experience_path, &adaptive_path).unwrap();
        assert_full_state_matches(&restarted, &baseline);
        assert_eq!(
            NoironEngine::read_full_state_manifest_for_test(&adaptive_path).unwrap(),
            (1, Some(0))
        );
        assert!(
            full_state_generation_paths(&memory_path, &experience_path, &adaptive_path, 2,)
                .iter()
                .all(|path| !path.exists())
        );
    }

    cleanup(memory_path);
    cleanup(experience_path);
    cleanup(adaptive_path);
}

#[test]
fn full_state_failure_after_manifest_rename_keeps_the_complete_new_generation() {
    let memory_path = temp_path("atomic-published-memory", "ndkv");
    let experience_path = temp_path("atomic-published-experience", "ndkv");
    let adaptive_path = temp_path("atomic-published-adaptive", "ndkv");
    let mut engine = NoironEngine::new();
    add_full_state_revision(&mut engine, 1);
    engine
        .save_full_state(&memory_path, &experience_path, &adaptive_path)
        .unwrap();
    add_full_state_revision(&mut engine, 2);
    let expected = engine.clone();

    let error = engine
        .save_full_state_failing_after(
            &memory_path,
            &experience_path,
            &adaptive_path,
            FullStateSaveStage::ManifestPublished,
        )
        .unwrap_err();

    assert!(
        error
            .to_string()
            .contains("injected full-state save failure")
    );
    assert_full_state_matches(&engine, &expected);
    assert_eq!(
        NoironEngine::read_full_state_manifest_for_test(&adaptive_path).unwrap(),
        (2, Some(1))
    );
    assert!(
        full_state_generation_paths(&memory_path, &experience_path, &adaptive_path, 2)
            .iter()
            .all(|path| path.is_file())
    );
    let restarted =
        NoironEngine::load_full_state(&memory_path, &experience_path, &adaptive_path).unwrap();
    assert_full_state_matches(&restarted, &expected);

    cleanup(memory_path);
    cleanup(experience_path);
    cleanup(adaptive_path);
}

#[test]
fn generation_transaction_keeps_published_state_after_commit_uncertain_error() {
    let memory_path = temp_path("transaction-published-memory", "ndkv");
    let experience_path = temp_path("transaction-published-experience", "ndkv");
    let adaptive_path = temp_path("transaction-published-adaptive", "ndkv");
    let mut engine = NoironEngine::new();
    add_full_state_revision(&mut engine, 1);
    engine
        .save_full_state(&memory_path, &experience_path, &adaptive_path)
        .unwrap();

    let transaction = engine.begin_generation_state_transaction();
    add_full_state_revision(&mut engine, 2);
    let expected = engine.clone();
    let error = engine
        .save_full_state_in_generation_transaction_failing_after(
            &transaction,
            &memory_path,
            &experience_path,
            &adaptive_path,
            FullStateSaveStage::ManifestPublished,
        )
        .unwrap_err();

    assert!(error.committed());
    assert!(
        error
            .into_inner()
            .to_string()
            .contains("injected full-state save failure")
    );
    engine.commit_generation_state_transaction(transaction);
    assert_full_state_matches(&engine, &expected);
    assert_eq!(
        NoironEngine::read_full_state_manifest_for_test(&adaptive_path).unwrap(),
        (2, Some(1))
    );
    let restarted =
        NoironEngine::load_full_state(&memory_path, &experience_path, &adaptive_path).unwrap();
    assert_full_state_matches(&restarted, &expected);

    add_full_state_revision(&mut engine, 3);
    engine
        .save_full_state(&memory_path, &experience_path, &adaptive_path)
        .unwrap();
    assert_eq!(
        NoironEngine::read_full_state_manifest_for_test(&adaptive_path).unwrap(),
        (3, Some(2))
    );

    cleanup(memory_path);
    cleanup(experience_path);
    cleanup(adaptive_path);
}

#[test]
fn full_state_success_advances_once_and_retains_only_current_and_previous() {
    let memory_path = temp_path("atomic-retention-memory", "ndkv");
    let experience_path = temp_path("atomic-retention-experience", "ndkv");
    let adaptive_path = temp_path("atomic-retention-adaptive", "ndkv");
    let mut engine = NoironEngine::new();

    for generation in 1..=3 {
        add_full_state_revision(&mut engine, generation);
        engine
            .save_full_state(&memory_path, &experience_path, &adaptive_path)
            .unwrap();
        assert_eq!(
            NoironEngine::read_full_state_manifest_for_test(&adaptive_path).unwrap(),
            (
                generation,
                Some(if generation == 1 { 0 } else { generation - 1 })
            )
        );
        let restarted =
            NoironEngine::load_full_state(&memory_path, &experience_path, &adaptive_path).unwrap();
        assert_full_state_matches(&restarted, &engine);
    }

    assert!(
        full_state_generation_paths(&memory_path, &experience_path, &adaptive_path, 1)
            .iter()
            .all(|path| !path.exists())
    );
    for generation in [2, 3] {
        assert!(
            full_state_generation_paths(
                &memory_path,
                &experience_path,
                &adaptive_path,
                generation,
            )
            .iter()
            .all(|path| path.is_file())
        );
    }

    let generation_two =
        full_state_generation_paths(&memory_path, &experience_path, &adaptive_path, 2);
    let orphan_generation_one =
        full_state_generation_paths(&memory_path, &experience_path, &adaptive_path, 1);
    for (source, orphan) in generation_two.iter().zip(&orphan_generation_one) {
        std::fs::copy(source, orphan).unwrap();
    }
    add_full_state_revision(&mut engine, 4);
    engine
        .save_full_state(&memory_path, &experience_path, &adaptive_path)
        .unwrap();
    assert_eq!(
        NoironEngine::read_full_state_manifest_for_test(&adaptive_path).unwrap(),
        (4, Some(3))
    );
    for generation in [1, 2] {
        assert!(
            full_state_generation_paths(
                &memory_path,
                &experience_path,
                &adaptive_path,
                generation,
            )
            .iter()
            .all(|path| !path.exists())
        );
    }

    cleanup(memory_path);
    cleanup(experience_path);
    cleanup(adaptive_path);
}

#[test]
fn full_state_publication_syncs_distinct_parent_directories() {
    let root = temp_path("atomic-directory-sync", "dir");
    let memory_path = root.join("memory").join("memory.ndkv");
    let experience_path = root.join("experience").join("experience.ndkv");
    let adaptive_path = root.join("adaptive").join("adaptive.ndkv");
    let mut engine = NoironEngine::new();
    add_full_state_revision(&mut engine, 1);

    engine
        .save_full_state(&memory_path, &experience_path, &adaptive_path)
        .unwrap();

    let restarted =
        NoironEngine::load_full_state(&memory_path, &experience_path, &adaptive_path).unwrap();
    assert_full_state_matches(&restarted, &engine);
    assert_eq!(
        NoironEngine::read_full_state_manifest_for_test(&adaptive_path).unwrap(),
        (1, Some(0))
    );
    assert!(
        [&memory_path, &experience_path, &adaptive_path]
            .iter()
            .all(|path| path.parent().unwrap().is_dir())
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn full_state_explicitly_migrates_legacy_three_file_state() {
    let memory_path = temp_path("atomic-legacy-memory", "ndkv");
    let experience_path = temp_path("atomic-legacy-experience", "ndkv");
    let adaptive_path = temp_path("atomic-legacy-adaptive", "ndkv");
    let mut legacy = NoironEngine::new();
    add_full_state_revision(&mut legacy, 1);
    legacy.save_memory(&memory_path).unwrap();
    legacy.save_experience(&experience_path).unwrap();
    legacy.save_adaptive_state(&adaptive_path).unwrap();
    let manifest_path = NoironEngine::full_state_manifest_path_for_test(&adaptive_path);
    assert!(!manifest_path.exists());

    let mut migrated =
        NoironEngine::load_full_state(&memory_path, &experience_path, &adaptive_path).unwrap();
    assert_full_state_matches(&migrated, &legacy);
    migrated
        .save_full_state(&memory_path, &experience_path, &adaptive_path)
        .unwrap();
    assert_eq!(
        NoironEngine::read_full_state_manifest_for_test(&adaptive_path).unwrap(),
        (1, Some(0))
    );
    let restarted =
        NoironEngine::load_full_state(&memory_path, &experience_path, &adaptive_path).unwrap();
    assert_full_state_matches(&restarted, &migrated);

    cleanup(memory_path);
    cleanup(experience_path);
    cleanup(adaptive_path);
}

#[test]
fn full_state_recovers_windows_manifest_backup_window() {
    let memory_path = temp_path("atomic-windows-memory", "ndkv");
    let experience_path = temp_path("atomic-windows-experience", "ndkv");
    let adaptive_path = temp_path("atomic-windows-adaptive", "ndkv");
    let mut engine = NoironEngine::new();
    add_full_state_revision(&mut engine, 1);
    engine
        .save_full_state(&memory_path, &experience_path, &adaptive_path)
        .unwrap();
    add_full_state_revision(&mut engine, 2);
    engine
        .save_full_state(&memory_path, &experience_path, &adaptive_path)
        .unwrap();
    let committed =
        NoironEngine::load_full_state(&memory_path, &experience_path, &adaptive_path).unwrap();
    let manifest = NoironEngine::full_state_manifest_path_for_test(&adaptive_path);
    let backup = appended_test_path(&manifest, ".bak");
    let next = appended_test_path(&manifest, ".next");
    std::fs::remove_file(&backup).unwrap();
    std::fs::rename(&manifest, &backup).unwrap();
    std::fs::copy(&backup, &next).unwrap();

    let mut recovered =
        NoironEngine::load_full_state(&memory_path, &experience_path, &adaptive_path).unwrap();
    assert_full_state_matches(&recovered, &committed);
    assert!(!manifest.exists());
    assert!(backup.is_file());
    assert!(next.is_file());

    add_full_state_revision(&mut recovered, 3);
    recovered
        .save_full_state(&memory_path, &experience_path, &adaptive_path)
        .unwrap();
    assert_eq!(
        NoironEngine::read_full_state_manifest_for_test(&adaptive_path).unwrap(),
        (3, Some(2))
    );
    assert!(
        full_state_generation_paths(&memory_path, &experience_path, &adaptive_path, 1)
            .iter()
            .all(|path| !path.exists())
    );

    cleanup(memory_path);
    cleanup(experience_path);
    cleanup(adaptive_path);
}

#[test]
fn full_state_falls_back_as_one_generation_without_creating_missing_files() {
    let memory_path = temp_path("atomic-fallback-memory", "ndkv");
    let experience_path = temp_path("atomic-fallback-experience", "ndkv");
    let adaptive_path = temp_path("atomic-fallback-adaptive", "ndkv");
    let mut engine = NoironEngine::new();
    add_full_state_revision(&mut engine, 1);
    engine
        .save_full_state(&memory_path, &experience_path, &adaptive_path)
        .unwrap();
    let previous =
        NoironEngine::load_full_state(&memory_path, &experience_path, &adaptive_path).unwrap();
    add_full_state_revision(&mut engine, 2);
    engine
        .save_full_state(&memory_path, &experience_path, &adaptive_path)
        .unwrap();
    let missing_experience =
        NoironEngine::full_state_generation_path_for_test(&experience_path, 2).unwrap();
    std::fs::remove_file(&missing_experience).unwrap();

    let mut recovered =
        NoironEngine::load_full_state(&memory_path, &experience_path, &adaptive_path).unwrap();
    assert_full_state_matches(&recovered, &previous);
    assert!(!missing_experience.exists());
    assert_eq!(
        NoironEngine::read_full_state_manifest_for_test(&adaptive_path).unwrap(),
        (2, Some(1))
    );
    recovered
        .save_full_state(&memory_path, &experience_path, &adaptive_path)
        .unwrap();
    assert!(missing_experience.is_file());
    assert_eq!(
        NoironEngine::read_full_state_manifest_for_test(&adaptive_path).unwrap(),
        (2, Some(1))
    );

    cleanup(memory_path);
    cleanup(experience_path);
    cleanup(adaptive_path);
}

#[test]
fn full_state_rejects_reserved_or_colliding_paths_before_deleting_files() {
    let root = temp_path("atomic-path-collision", "dir");
    std::fs::create_dir_all(&root).unwrap();
    let adaptive_path = root.join("adaptive.ndkv");
    let cases = [
        (
            root.join("state.ndkv"),
            root.join("state.full-state-1.ndkv"),
        ),
        (root.join("same-stem"), root.join("same-stem.ndkv")),
    ];

    for (index, (memory_path, experience_path)) in cases.into_iter().enumerate() {
        let sentinel = format!("preserve-existing-experience-{index}");
        std::fs::write(&experience_path, &sentinel).unwrap();
        let mut engine = NoironEngine::new();
        add_full_state_revision(&mut engine, index as u64 + 1);

        let error = engine
            .save_full_state(&memory_path, &experience_path, &adaptive_path)
            .unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
        assert_eq!(std::fs::read_to_string(&experience_path).unwrap(), sentinel);
        assert!(!NoironEngine::full_state_manifest_path_for_test(&adaptive_path).exists());
        let _ = std::fs::remove_file(experience_path);
    }

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn full_state_rejects_a_stale_engine_generation_without_publication() {
    let memory_path = temp_path("atomic-cas-memory", "ndkv");
    let experience_path = temp_path("atomic-cas-experience", "ndkv");
    let adaptive_path = temp_path("atomic-cas-adaptive", "ndkv");
    let mut engine = NoironEngine::new();
    add_full_state_revision(&mut engine, 1);
    engine
        .save_full_state(&memory_path, &experience_path, &adaptive_path)
        .unwrap();
    let mut stale =
        NoironEngine::load_full_state(&memory_path, &experience_path, &adaptive_path).unwrap();
    let mut current = stale.clone();
    add_full_state_revision(&mut current, 2);
    current
        .save_full_state(&memory_path, &experience_path, &adaptive_path)
        .unwrap();
    add_full_state_revision(&mut stale, 3);

    let error = stale
        .save_full_state(&memory_path, &experience_path, &adaptive_path)
        .unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
    assert_eq!(
        NoironEngine::read_full_state_manifest_for_test(&adaptive_path).unwrap(),
        (2, Some(1))
    );
    let restarted =
        NoironEngine::load_full_state(&memory_path, &experience_path, &adaptive_path).unwrap();
    assert_full_state_matches(&restarted, &current);

    cleanup(memory_path);
    cleanup(experience_path);
    cleanup(adaptive_path);
}

#[test]
fn full_state_rejects_unbound_or_foreign_engines_over_an_existing_generation() {
    let memory_path = temp_path("atomic-unbound-memory", "ndkv");
    let experience_path = temp_path("atomic-unbound-experience", "ndkv");
    let adaptive_path = temp_path("atomic-unbound-adaptive", "ndkv");
    let foreign_memory_path = temp_path("atomic-foreign-memory", "ndkv");
    let foreign_experience_path = temp_path("atomic-foreign-experience", "ndkv");
    let foreign_adaptive_path = temp_path("atomic-foreign-adaptive", "ndkv");
    let mut current = NoironEngine::new();
    add_full_state_revision(&mut current, 1);
    current
        .save_full_state(&memory_path, &experience_path, &adaptive_path)
        .unwrap();
    let mut foreign = NoironEngine::new();
    add_full_state_revision(&mut foreign, 2);
    foreign
        .save_full_state(
            &foreign_memory_path,
            &foreign_experience_path,
            &foreign_adaptive_path,
        )
        .unwrap();
    let foreign = NoironEngine::load_full_state(
        &foreign_memory_path,
        &foreign_experience_path,
        &foreign_adaptive_path,
    )
    .unwrap();

    for mut candidate in [NoironEngine::new(), foreign] {
        add_full_state_revision(&mut candidate, 3);
        let error = candidate
            .save_full_state(&memory_path, &experience_path, &adaptive_path)
            .unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
        let restarted =
            NoironEngine::load_full_state(&memory_path, &experience_path, &adaptive_path).unwrap();
        assert_full_state_matches(&restarted, &current);
    }

    cleanup(memory_path);
    cleanup(experience_path);
    cleanup(adaptive_path);
    cleanup(foreign_memory_path);
    cleanup(foreign_experience_path);
    cleanup(foreign_adaptive_path);
}
