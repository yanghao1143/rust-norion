use super::*;

#[test]
fn inference_generates_gist_memory_for_high_quality_answer() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;

    let outcome = engine.infer(
        InferenceRequest::new(
            "Rust Noiron hierarchical gist memory for long context control",
            TaskProfile::LongDocument,
        ),
        &mut backend,
    );

    assert!(!outcome.gist_records.is_empty());
    assert!(!outcome.stored_gist_memory_ids.is_empty());
    assert_eq!(
        engine.experience.records()[0].gist_records.len(),
        outcome.gist_records.len()
    );
    assert_eq!(
        engine.experience.records()[0].gist_memory_ids,
        outcome.stored_gist_memory_ids
    );
    assert_eq!(outcome.evolution_ledger.live_inference_runs, 1);
    assert!(outcome.evolution_ledger.live_stored_memories >= 1);
    assert!(outcome.evolution_ledger.live_stored_gist_memories >= 1);
    assert!(outcome.evolution_ledger.live_stored_memory_updates() >= 2);
}

#[test]
fn inference_stores_high_quality_exported_runtime_kv() {
    struct ExportingBackend;

    impl InferenceBackend for ExportingBackend {
        fn generate(&mut self, _context: GenerationContext<'_>) -> InferenceDraft {
            InferenceDraft::new(
                    "Rust runtime KV export memory should be stored as useful Noiron local memory for future routing.",
                    vec![ReasoningStep::new("runtime", "exported reusable kv", 0.92)],
                )
                .with_exported_kv_blocks(vec![RuntimeKvBlock::new(
                    2,
                    1,
                    0,
                    4,
                    vec![0.1, 0.2],
                    vec![0.3, 0.4],
                )])
        }
    }

    let mut engine = NoironEngine::new();
    let mut backend = ExportingBackend;

    let outcome = engine.infer(
        InferenceRequest::new("Rust runtime KV export memory", TaskProfile::Coding),
        &mut backend,
    );

    assert_eq!(outcome.exported_runtime_kv_blocks, 1);
    assert_eq!(outcome.stored_runtime_kv_memory_ids.len(), 1);
    assert!(
        engine
            .cache
            .entries()
            .iter()
            .any(|entry| entry.key.contains("runtime_kv:l2h1"))
    );
}

#[derive(Debug, Default)]
struct TenantScopedMemoryBackend {
    seen_memory_keys: Vec<String>,
}

impl InferenceBackend for TenantScopedMemoryBackend {
    fn embed_text(&mut self, _text: &str) -> Option<Vec<f32>> {
        Some(vec![1.0, 0.0, 0.0])
    }

    fn generate(&mut self, context: GenerationContext<'_>) -> InferenceDraft {
        self.seen_memory_keys = context
            .memories
            .iter()
            .map(|memory| memory.key.clone())
            .collect();
        let infini_keys = context
            .infini_memory_plan
            .local_window()
            .iter()
            .chain(context.infini_memory_plan.global_memory())
            .chain(context.infini_memory_plan.skipped())
            .map(|item| item.key.as_str())
            .collect::<Vec<_>>();
        assert!(!infini_keys.is_empty());
        assert!(
            infini_keys
                .iter()
                .all(|key| key.contains("tenant=tenant-a"))
        );

        InferenceDraft::new(
            "Rust Noiron tenant scoped runtime KV memory stays isolated for local inference and future routing.",
            vec![ReasoningStep::new(
                "tenant_scope",
                "only same tenant KV memory should reach runtime context",
                0.93,
            )],
        )
        .with_exported_kv_blocks(vec![RuntimeKvBlock::new(
            1,
            0,
            0,
            2,
            vec![0.1, 0.2],
            vec![0.3, 0.4],
        )])
    }
}

#[test]
fn inference_request_tenant_scope_isolates_runtime_cache_reads_and_writes() {
    let tenant_a = crate::tenant_scope::TenantScope::new("tenant-a", "workspace", "session-a");
    let tenant_b = crate::tenant_scope::TenantScope::new("tenant-b", "workspace", "session-b");
    let mut engine = NoironEngine::new();
    let legacy =
        engine
            .cache
            .store_or_fuse("legacy shared runtime memory", vec![1.0, 0.0, 0.0], 0.92);
    let memory_a = engine.cache.store_scoped_or_fuse(
        &tenant_a,
        crate::tenant_scope::TenantResourceLane::KvMemory,
        "shared runtime memory",
        vec![1.0, 0.0, 0.0],
        0.92,
    );
    let memory_b = engine.cache.store_scoped_or_fuse(
        &tenant_b,
        crate::tenant_scope::TenantResourceLane::KvMemory,
        "shared runtime memory",
        vec![1.0, 0.0, 0.0],
        0.92,
    );
    let mut backend = TenantScopedMemoryBackend::default();

    let outcome = engine.infer(
        InferenceRequest::new("Rust tenant scoped runtime memory", TaskProfile::Coding)
            .with_tenant_scope(tenant_a.clone()),
        &mut backend,
    );

    assert!(
        outcome
            .used_memories
            .iter()
            .any(|memory| memory.id == memory_a)
    );
    assert!(
        outcome
            .used_memories
            .iter()
            .all(|memory| memory.id != memory_b)
    );
    assert!(
        outcome
            .used_memories
            .iter()
            .all(|memory| memory.id != legacy)
    );
    assert!(!backend.seen_memory_keys.is_empty());
    assert!(
        backend
            .seen_memory_keys
            .iter()
            .all(|key| key.contains("tenant=tenant-a"))
    );
    assert!(
        outcome
            .infini_memory_plan
            .local_window()
            .iter()
            .chain(outcome.infini_memory_plan.global_memory())
            .chain(outcome.infini_memory_plan.skipped())
            .all(|item| item.key.contains("tenant=tenant-a"))
    );

    let stored_memory_id = outcome.stored_memory_id.expect("stored tenant memory");
    let stored_memory_key = &engine
        .cache
        .entries()
        .iter()
        .find(|entry| entry.id == stored_memory_id)
        .unwrap()
        .key;
    let parsed_memory =
        crate::tenant_scope::TenantScopedKey::parse(stored_memory_key).expect("scoped memory key");
    assert_eq!(parsed_memory.scope, tenant_a);
    assert_eq!(
        parsed_memory.lane,
        crate::tenant_scope::TenantResourceLane::KvMemory
    );
    assert_eq!(
        outcome.reasoning_genome_chain.express_chain.len(),
        outcome.reasoning_genome.expression_gene_count
    );
    assert!(
        outcome
            .reasoning_genome_chain
            .express_chain
            .iter()
            .all(|record| {
                record.lineage.tenant_scope == tenant_a.lineage_tenant_scope()
                    && record.lineage.session_id == tenant_a.session_id
            })
    );
    let genome_gate = crate::tenant_scope::TenantIsolationGate::new();
    let allowed = genome_gate.check_genome_chain_access(
        &tenant_a,
        &outcome.reasoning_genome_chain,
        crate::tenant_scope::TenantAccessKind::Inherit,
    );
    let rejected = genome_gate.check_genome_chain_access(
        &tenant_b,
        &outcome.reasoning_genome_chain,
        crate::tenant_scope::TenantAccessKind::Inherit,
    );
    let score_allowed = genome_gate.check_genome_chain_access(
        &tenant_a,
        &outcome.reasoning_genome_chain,
        crate::tenant_scope::TenantAccessKind::Score,
    );
    let score_rejected = genome_gate.check_genome_chain_access(
        &tenant_b,
        &outcome.reasoning_genome_chain,
        crate::tenant_scope::TenantAccessKind::Score,
    );
    let write_rejected = genome_gate.check_genome_chain_access(
        &tenant_b,
        &outcome.reasoning_genome_chain,
        crate::tenant_scope::TenantAccessKind::Write,
    );
    assert!(allowed.allowed);
    assert!(!rejected.allowed);
    assert_eq!(rejected.audit_event.reason, "cross_tenant_scope_rejected");
    assert!(score_allowed.allowed);
    assert!(!score_rejected.allowed);
    assert_eq!(
        score_rejected.audit_event.reason,
        "cross_tenant_scope_rejected"
    );
    assert!(!write_rejected.allowed);
    assert_eq!(
        write_rejected.audit_event.reason,
        "cross_tenant_scope_rejected"
    );

    assert!(!outcome.stored_runtime_kv_memory_ids.is_empty());
    for memory_id in &outcome.stored_runtime_kv_memory_ids {
        let key = &engine
            .cache
            .entries()
            .iter()
            .find(|entry| entry.id == *memory_id)
            .unwrap()
            .key;
        let parsed =
            crate::tenant_scope::TenantScopedKey::parse(key).expect("scoped runtime kv memory key");
        assert_eq!(parsed.scope, tenant_a);
        assert_eq!(
            parsed.lane,
            crate::tenant_scope::TenantResourceLane::RuntimeKv
        );
    }
}

#[derive(Debug, Default)]
struct TenantScopedExperienceBackend {
    defer_auto_replay: bool,
    seen_tenant_scope: Option<crate::tenant_scope::TenantScope>,
    seen_experience_ids: Vec<u64>,
}

impl InferenceBackend for TenantScopedExperienceBackend {
    fn defer_auto_replay_until_generation_result(&self) -> bool {
        self.defer_auto_replay
    }

    fn embed_text(&mut self, _text: &str) -> Option<Vec<f32>> {
        Some(vec![1.0, 0.0, 0.0])
    }

    fn generate(&mut self, context: GenerationContext<'_>) -> InferenceDraft {
        self.seen_tenant_scope = context.tenant_scope.cloned();
        self.seen_experience_ids = context
            .experiences
            .iter()
            .map(|experience| experience.id)
            .collect();

        InferenceDraft::new(
            "Rust tenant scoped experience recall keeps runtime adapter learning isolated.",
            vec![ReasoningStep::new(
                "tenant_scope",
                "only same tenant experience should reach hot inference",
                0.93,
            )],
        )
        .with_runtime_diagnostics(RuntimeDiagnostics {
            model_id: Some("tenant-adapter-test".to_owned()),
            selected_adapter: Some("portable-rust".to_owned()),
            forward_energy: Some(0.22),
            kv_influence: Some(0.72),
            ..RuntimeDiagnostics::default()
        })
    }
}

#[test]
fn inference_request_default_scope_isolates_runtime_memory_and_experience() {
    let local_scope = crate::tenant_scope::TenantScope::local_single_user();
    let foreign_scope = crate::tenant_scope::TenantScope::new("tenant-b", "workspace", "session-b");
    let mut engine = NoironEngine::new();
    let legacy =
        engine
            .cache
            .store_or_fuse("legacy shared runtime memory", vec![1.0, 0.0, 0.0], 0.99);
    let local_memory = engine.cache.store_scoped_or_fuse(
        &local_scope,
        crate::tenant_scope::TenantResourceLane::KvMemory,
        "local adapter experience",
        vec![1.0, 0.0, 0.0],
        0.95,
    );
    let foreign_memory = engine.cache.store_scoped_or_fuse(
        &foreign_scope,
        crate::tenant_scope::TenantResourceLane::KvMemory,
        "foreign adapter experience",
        vec![1.0, 0.0, 0.0],
        0.99,
    );
    let local_experience = seed_runtime_adapter_experience_for_memory(
        &mut engine,
        local_memory,
        "tenant-adapter-test",
        "portable-rust",
        0.90,
    );
    let foreign_experience = seed_runtime_adapter_experience_for_memory(
        &mut engine,
        foreign_memory,
        "tenant-adapter-test",
        "cpu-simd",
        0.99,
    );
    let mut backend = TenantScopedExperienceBackend::default();

    let outcome = engine.infer(
        InferenceRequest::new(
            "Rust default tenant scoped adapter experience",
            TaskProfile::Coding,
        ),
        &mut backend,
    );

    assert_eq!(backend.seen_tenant_scope, Some(local_scope.clone()));
    assert!(
        outcome
            .used_memories
            .iter()
            .any(|memory| memory.id == local_memory)
    );
    assert!(
        outcome
            .used_memories
            .iter()
            .all(|memory| memory.id != foreign_memory)
    );
    assert!(
        outcome
            .used_memories
            .iter()
            .all(|memory| memory.id != legacy)
    );
    assert_eq!(backend.seen_experience_ids, vec![local_experience]);
    assert!(
        outcome
            .used_experiences
            .iter()
            .all(|experience| experience.id != foreign_experience)
    );
    let stored_memory_key = &engine
        .cache
        .entries()
        .iter()
        .find(|entry| Some(entry.id) == outcome.stored_memory_id)
        .expect("stored default scoped memory")
        .key;
    let parsed_memory =
        crate::tenant_scope::TenantScopedKey::parse(stored_memory_key).expect("scoped memory key");
    assert_eq!(parsed_memory.scope, local_scope);
    for memory_id in &outcome.stored_runtime_kv_memory_ids {
        let key = &engine
            .cache
            .entries()
            .iter()
            .find(|entry| entry.id == *memory_id)
            .unwrap()
            .key;
        let parsed =
            crate::tenant_scope::TenantScopedKey::parse(key).expect("scoped runtime kv memory key");
        assert_eq!(parsed.scope, local_scope);
    }
}

#[test]
fn inference_request_tenant_scope_isolates_runtime_experience_recall() {
    assert_tenant_scoped_auto_replay(false);
    assert_tenant_scoped_auto_replay(true);
}

fn assert_tenant_scoped_auto_replay(defer_auto_replay: bool) {
    let tenant_a = crate::tenant_scope::TenantScope::new("tenant-a", "workspace", "session-a");
    let tenant_b = crate::tenant_scope::TenantScope::new("tenant-b", "workspace", "session-b");
    let mut engine = NoironEngine::new();
    let memory_a = engine.cache.store_scoped_or_fuse(
        &tenant_a,
        crate::tenant_scope::TenantResourceLane::KvMemory,
        "adapter-experience-a",
        vec![1.0, 0.0, 0.0],
        0.95,
    );
    let memory_b = engine.cache.store_scoped_or_fuse(
        &tenant_b,
        crate::tenant_scope::TenantResourceLane::KvMemory,
        "adapter-experience-b",
        vec![1.0, 0.0, 0.0],
        0.99,
    );
    let experience_a = seed_runtime_adapter_experience_for_memory(
        &mut engine,
        memory_a,
        "tenant-adapter-test",
        "portable-rust",
        0.20,
    );
    let experience_b = seed_runtime_adapter_experience_for_memory(
        &mut engine,
        memory_b,
        "tenant-adapter-test",
        "cpu-simd",
        0.99,
    );
    let foreign_before = engine
        .cache
        .entries()
        .iter()
        .find(|entry| entry.id == memory_b)
        .unwrap()
        .clone();
    let mut backend = TenantScopedExperienceBackend {
        defer_auto_replay,
        ..TenantScopedExperienceBackend::default()
    };

    let outcome = engine.infer(
        InferenceRequest::new("Rust tenant scoped adapter experience", TaskProfile::Coding)
            .with_tenant_scope(tenant_a),
        &mut backend,
    );

    assert_eq!(backend.seen_experience_ids, vec![experience_a]);
    assert!(
        outcome
            .used_experiences
            .iter()
            .all(|experience| experience.id != experience_b)
    );
    assert!(
        outcome
            .runtime_adapter_observations
            .iter()
            .any(|observation| observation.experience_id == experience_a)
    );
    assert!(
        outcome
            .runtime_adapter_observations
            .iter()
            .all(|observation| observation.experience_id != experience_b)
    );
    let replay = outcome.auto_replay_report.as_ref().unwrap();
    assert_eq!(replay.touched_memories, 1);
    assert_eq!(replay.penalized, 1);
    let foreign_after = engine
        .cache
        .entries()
        .iter()
        .find(|entry| entry.id == memory_b)
        .unwrap();
    assert_eq!(foreign_after.strength, foreign_before.strength);
    assert_eq!(foreign_after.hits, foreign_before.hits);
    assert_eq!(foreign_after.failures, foreign_before.failures);
    assert_eq!(foreign_after.last_access, foreign_before.last_access);
}

fn seed_local_runtime_kv_memory(
    engine: &mut NoironEngine,
    local_key: &str,
    usefulness: f32,
) -> u64 {
    seed_local_runtime_kv_memory_with_vector(engine, local_key, vec![1.0, 0.0, 0.0], usefulness)
}

fn seed_local_runtime_kv_memory_with_vector(
    engine: &mut NoironEngine,
    local_key: &str,
    vector: Vec<f32>,
    usefulness: f32,
) -> u64 {
    engine.cache.store_scoped_or_fuse(
        &crate::tenant_scope::TenantScope::local_single_user(),
        crate::tenant_scope::TenantResourceLane::RuntimeKv,
        local_key,
        vector,
        usefulness,
    )
}

fn mock_rust_native_embedding(text: &str) -> Vec<f32> {
    let mut values = text
        .split_whitespace()
        .take(8)
        .enumerate()
        .map(|(index, token)| ((token.len() + index + 1) as f32 / 32.0).clamp(0.0, 1.0))
        .collect::<Vec<_>>();
    values.resize(8, 0.0);
    values
}

fn use_cpu_only_hardware(engine: &mut NoironEngine) {
    engine.set_hardware_snapshot(HardwareSnapshot::new(
        DeviceClass::CpuOnly,
        0.20,
        0.10,
        0.25,
        0.10,
    ));
}

fn seed_runtime_adapter_experience(engine: &mut NoironEngine, model_id: &str, adapter: &str) {
    let memory_id = engine.cache.store_scoped_or_fuse(
        &crate::tenant_scope::TenantScope::local_single_user(),
        crate::tenant_scope::TenantResourceLane::KvMemory,
        "runtime adapter seeded observation",
        vec![1.0, 0.0, 0.0],
        0.90,
    );
    engine.experience.record(ExperienceInput {
        prompt: "Rust runtime adapter seeded observation".to_owned(),
        profile: TaskProfile::Coding,
        lesson: "prefer trusted runtime adapter evidence only after canonical validation"
            .to_owned(),
        quality: 0.90,
        contradictions: Vec::new(),
        reflection_issues: Vec::new(),
        revision_actions: Vec::new(),
        stored_memory_id: Some(memory_id),
        router_threshold_after: 0.50,
        stream_windows: 1,
        route_budget: RouteBudget {
            threshold: 0.50,
            attention_tokens: 2,
            fast_tokens: 2,
            attention_fraction: 0.50,
        },
        hierarchy: HierarchyWeights::new(0.20, 0.60, 0.20),
        used_memory_ids: vec![memory_id],
        gist_records: Vec::new(),
        gist_memory_ids: Vec::new(),
        stored_runtime_kv_memory_ids: Vec::new(),
        runtime_diagnostics: RuntimeDiagnostics {
            model_id: Some(model_id.to_owned()),
            selected_adapter: Some(adapter.to_owned()),
            forward_energy: Some(0.20),
            kv_influence: Some(0.72),
            imported_kv_blocks: 1,
            exported_kv_blocks: 1,
            ..RuntimeDiagnostics::default()
        },
        runtime_token_metrics: Default::default(),
        process_reward: ProcessRewardReport {
            total: 0.88,
            action: RewardAction::Reinforce,
            components: ProcessRewardComponents::default(),
            notes: Vec::new(),
        },
        live_evolution: Default::default(),
    });
}

fn seed_runtime_adapter_experience_for_memory(
    engine: &mut NoironEngine,
    memory_id: u64,
    model_id: &str,
    adapter: &str,
    quality: f32,
) -> u64 {
    engine.experience.record(ExperienceInput {
        prompt: "Rust tenant scoped adapter experience".to_owned(),
        profile: TaskProfile::Coding,
        lesson: format!("prefer {adapter} only for matching tenant scope"),
        quality,
        contradictions: Vec::new(),
        reflection_issues: Vec::new(),
        revision_actions: Vec::new(),
        stored_memory_id: Some(memory_id),
        router_threshold_after: 0.50,
        stream_windows: 1,
        route_budget: RouteBudget {
            threshold: 0.50,
            attention_tokens: 2,
            fast_tokens: 2,
            attention_fraction: 0.50,
        },
        hierarchy: HierarchyWeights::new(0.20, 0.60, 0.20),
        used_memory_ids: vec![memory_id],
        gist_records: Vec::new(),
        gist_memory_ids: Vec::new(),
        stored_runtime_kv_memory_ids: Vec::new(),
        runtime_diagnostics: RuntimeDiagnostics {
            model_id: Some(model_id.to_owned()),
            selected_adapter: Some(adapter.to_owned()),
            forward_energy: Some(0.20),
            kv_influence: Some(0.72),
            imported_kv_blocks: 1,
            exported_kv_blocks: 1,
            ..RuntimeDiagnostics::default()
        },
        runtime_token_metrics: Default::default(),
        process_reward: ProcessRewardReport {
            total: quality,
            action: RewardAction::Reinforce,
            components: ProcessRewardComponents::default(),
            notes: Vec::new(),
        },
        live_evolution: Default::default(),
    })
}

#[test]
fn current_rust_native_adapter_run_creates_sanitized_reliability_candidate() {
    let private_prompt = "Rust native current adapter private tenant alpha credential marker";
    let mut engine = NoironEngine::new();
    let runtime =
        crate::runtime::RustNativeModelRuntime::new(crate::runtime::MockRustNativeAdapter::new())
            .with_cache_mode(crate::runtime::ChunkedKvCacheMode::NoCache);
    let mut backend = RuntimeBackend::new(runtime).with_max_tokens(32);
    use_cpu_only_hardware(&mut engine);

    let outcome = engine.infer(
        InferenceRequest::new(private_prompt, TaskProfile::Coding),
        &mut backend,
    );
    let admission = &outcome.memory_admission;
    let tool_candidate = admission
        .candidates
        .iter()
        .find(|candidate| {
            candidate.kind
                == crate::memory_admission::MemoryAdmissionKind::ToolReliabilityObservation
        })
        .expect("current adapter reliability candidate");
    let evidence_lines = admission
        .review_packet_summaries()
        .into_iter()
        .chain(admission.ledger_summaries())
        .chain(admission.fusion_plan.score_summaries(usize::MAX))
        .collect::<Vec<_>>();

    assert!(outcome.runtime_diagnostics.selected_adapter.is_some());
    assert!(outcome.runtime_adapter_observations.is_empty());
    assert_eq!(
        tool_candidate.decision,
        crate::memory_admission::MemoryAdmissionDecision::Ready
    );
    assert!(
        tool_candidate
            .evidence
            .iter()
            .any(|item| item == "runtime_adapter_current_signal=true")
    );
    assert!(
        tool_candidate
            .evidence
            .iter()
            .any(|item| item == "runtime_adapter_observations=0")
    );
    assert_eq!(
        tool_candidate.privacy_classification,
        crate::memory_admission::MemoryPrivacyClassification::DigestOnly
    );
    assert!(tool_candidate.privacy_checked);
    assert!(!tool_candidate.durable_write_authorized);
    assert!(!tool_candidate.applied);
    assert!(admission.is_read_only_preview());

    for line in evidence_lines {
        assert!(!line.contains(private_prompt), "{line}");
        for marker in ["prompt:", "tenant_id=", "secret=", "sk-current-adapter"] {
            assert!(!line.contains(marker), "{line}");
        }
        assert!(
            !crate::privacy_redaction::contains_private_or_executable_marker(&line),
            "{line}"
        );
    }
}

#[test]
fn unknown_current_adapter_does_not_create_reliability_candidate() {
    struct UnknownAdapterBackend;

    impl InferenceBackend for UnknownAdapterBackend {
        fn generate(&mut self, _context: GenerationContext<'_>) -> InferenceDraft {
            InferenceDraft::new(
                "Rust runtime answer with enough stable detail for reflection.",
                vec![ReasoningStep::new(
                    "runtime",
                    "unknown adapter name should not be trusted for self learning",
                    0.91,
                )],
            )
            .with_runtime_diagnostics(RuntimeDiagnostics {
                model_id: Some("unknown-adapter-test".to_owned()),
                selected_adapter: Some("unknown-adapter secret=sk-should-not-learn".to_owned()),
                forward_energy: Some(0.20),
                kv_influence: Some(0.60),
                ..RuntimeDiagnostics::default()
            })
        }
    }

    let private_prompt =
        "Rust native unknown adapter prompt: tenant_id=prod-42 secret=sk-unknown-adapter";
    let mut engine = NoironEngine::new();
    let mut backend = UnknownAdapterBackend;

    let outcome = engine.infer(
        InferenceRequest::new(private_prompt, TaskProfile::Coding),
        &mut backend,
    );
    let admission = &outcome.memory_admission;
    let evidence_lines = admission
        .candidate_summaries()
        .into_iter()
        .chain(admission.review_packet_summaries())
        .chain(admission.ledger_summaries())
        .chain(admission.fusion_plan.score_summaries(usize::MAX))
        .collect::<Vec<_>>();

    assert_eq!(
        outcome.runtime_diagnostics.selected_adapter.as_deref(),
        Some("unknown-adapter secret=sk-should-not-learn")
    );
    assert!(!admission.candidates.iter().any(|candidate| {
        candidate.kind == crate::memory_admission::MemoryAdmissionKind::ToolReliabilityObservation
    }));
    assert!(admission.is_read_only_preview());
    assert!(!evidence_lines.is_empty());

    for line in evidence_lines {
        assert!(!line.contains(private_prompt), "{line}");
        for marker in [
            "tenant_id=",
            "secret=",
            "sk-unknown-adapter",
            "sk-should-not-learn",
            "unknown-adapter secret",
        ] {
            assert!(!line.contains(marker), "{line}");
        }
        assert!(
            !crate::privacy_redaction::contains_private_or_executable_marker(&line),
            "{line}"
        );
    }
}

#[test]
fn unknown_current_adapter_keeps_memory_admission_mismatch_signals_false() {
    struct UnknownAdapterBackend;

    impl InferenceBackend for UnknownAdapterBackend {
        fn generate(&mut self, _context: GenerationContext<'_>) -> InferenceDraft {
            InferenceDraft::new(
                "Rust runtime answer with enough stable detail for reflection.",
                vec![ReasoningStep::new(
                    "runtime",
                    "unknown adapter name should not override trusted observations",
                    0.91,
                )],
            )
            .with_runtime_diagnostics(RuntimeDiagnostics {
                model_id: Some("unknown-adapter-test".to_owned()),
                selected_adapter: Some("unknown-adapter secret=sk-current-observed".to_owned()),
                forward_energy: Some(0.20),
                kv_influence: Some(0.60),
                ..RuntimeDiagnostics::default()
            })
        }
    }

    let mut engine = NoironEngine::new();
    seed_runtime_adapter_experience(&mut engine, "unknown-adapter-test", "portable-rust");
    let mut backend = UnknownAdapterBackend;
    let outcome = engine.infer(
        InferenceRequest::new(
            "Rust runtime adapter seeded observation current unknown",
            TaskProfile::Coding,
        ),
        &mut backend,
    );
    let admission = &outcome.memory_admission;
    let tool_candidate = admission
        .candidates
        .iter()
        .find(|candidate| {
            candidate.kind
                == crate::memory_admission::MemoryAdmissionKind::ToolReliabilityObservation
        })
        .expect("tool reliability candidate from trusted observation");
    let evidence_lines = admission
        .candidate_summaries()
        .into_iter()
        .chain(admission.review_packet_summaries())
        .chain(admission.ledger_summaries())
        .chain(admission.fusion_plan.score_summaries(usize::MAX))
        .collect::<Vec<_>>();

    assert_eq!(
        outcome.runtime_diagnostics.selected_adapter.as_deref(),
        Some("unknown-adapter secret=sk-current-observed")
    );
    assert!(!outcome.runtime_adapter_observations.is_empty());
    assert!(
        tool_candidate
            .evidence
            .iter()
            .any(|item| { item == "runtime_adapter_selection_mismatch=false" })
    );
    assert!(
        tool_candidate
            .evidence
            .iter()
            .any(|item| { item == "runtime_adapter_current_signal=false" })
    );
    assert!(admission.is_read_only_preview());

    for line in evidence_lines {
        for marker in ["secret=", "sk-current-observed", "unknown-adapter secret"] {
            assert!(!line.contains(marker), "{line}");
        }
        assert!(
            !crate::privacy_redaction::contains_private_or_executable_marker(&line),
            "{line}"
        );
    }
}

#[test]
fn rust_native_adapter_self_learning_evidence_is_sanitized() {
    let private_prompt = "Rust runtime KV reuse memory private tenant alpha adapter output marker";
    let mut engine = NoironEngine::new();
    let runtime_kv_memory_id = seed_local_runtime_kv_memory_with_vector(
        &mut engine,
        "runtime_kv:l0h0:0-1 :: Rust runtime KV reuse memory",
        mock_rust_native_embedding(private_prompt),
        0.92,
    );
    engine.experience.record(ExperienceInput {
        prompt: "runtime adapter self learning evidence".to_owned(),
        profile: TaskProfile::Coding,
        lesson: "prefer portable rust adapter when runtime reward is strong".to_owned(),
        quality: 0.90,
        contradictions: Vec::new(),
        reflection_issues: Vec::new(),
        revision_actions: Vec::new(),
        stored_memory_id: None,
        router_threshold_after: 0.50,
        stream_windows: 1,
        route_budget: RouteBudget {
            threshold: 0.50,
            attention_tokens: 2,
            fast_tokens: 2,
            attention_fraction: 0.50,
        },
        hierarchy: HierarchyWeights::new(0.20, 0.60, 0.20),
        used_memory_ids: Vec::new(),
        gist_records: Vec::new(),
        gist_memory_ids: Vec::new(),
        stored_runtime_kv_memory_ids: vec![runtime_kv_memory_id],
        runtime_diagnostics: RuntimeDiagnostics {
            model_id: Some("rust-native-mock".to_owned()),
            selected_adapter: Some("portable-rust".to_owned()),
            forward_energy: Some(0.20),
            kv_influence: Some(0.72),
            imported_kv_blocks: 1,
            exported_kv_blocks: 1,
            ..RuntimeDiagnostics::default()
        },
        runtime_token_metrics: Default::default(),
        process_reward: ProcessRewardReport {
            total: 0.88,
            action: RewardAction::Reinforce,
            components: ProcessRewardComponents::default(),
            notes: Vec::new(),
        },
        live_evolution: Default::default(),
    });
    let runtime =
        crate::runtime::RustNativeModelRuntime::new(crate::runtime::MockRustNativeAdapter::new())
            .with_cache_mode(crate::runtime::ChunkedKvCacheMode::ChunkedCache);
    let mut backend = RuntimeBackend::new(runtime).with_max_tokens(32);
    use_cpu_only_hardware(&mut engine);

    let outcome = engine.infer(
        InferenceRequest::new(private_prompt, TaskProfile::Coding),
        &mut backend,
    );
    let admission = &outcome.memory_admission;
    let evidence_lines = admission
        .candidate_summaries()
        .into_iter()
        .chain(admission.review_packet_summaries())
        .chain(admission.ledger_summaries())
        .chain(admission.fusion_plan.score_summaries(usize::MAX))
        .collect::<Vec<_>>();

    assert!(outcome.runtime_adapter_observations.len() >= 1);
    assert!(outcome.exported_runtime_kv_blocks >= 1);
    assert!(admission.candidates.iter().any(|candidate| {
        candidate.kind == crate::memory_admission::MemoryAdmissionKind::ToolReliabilityObservation
    }));
    assert!(admission.candidates.iter().any(|candidate| {
        candidate.kind == crate::memory_admission::MemoryAdmissionKind::RuntimeKvEvidence
    }));
    assert!(admission.candidates.iter().all(|candidate| {
        candidate.privacy_classification
            == crate::memory_admission::MemoryPrivacyClassification::DigestOnly
            && candidate.privacy_checked
            && !candidate.durable_write_authorized
            && !candidate.applied
            && (candidate.source_hash.starts_with("sha256:")
                || candidate.source_hash.starts_with("redaction-digest:"))
    }));
    assert!(admission.is_read_only_preview());
    assert!(!evidence_lines.is_empty());

    for line in evidence_lines {
        assert!(
            !line.contains(private_prompt),
            "raw prompt leaked in evidence line: {line}"
        );
        for marker in [
            "prompt:",
            "answer:",
            "tenant_id=",
            "secret=",
            "sk-test-noiron",
        ] {
            assert!(
                !line.contains(marker),
                "private marker {marker} leaked in evidence line: {line}"
            );
        }
        assert!(
            !crate::privacy_redaction::contains_private_or_executable_marker(&line),
            "privacy detector flagged evidence line: {line}"
        );
    }
}

#[test]
fn fast_path_watch_holds_exported_runtime_kv_admission() {
    struct FastPathExportingBackend;

    impl InferenceBackend for FastPathExportingBackend {
        fn generate(&mut self, _context: GenerationContext<'_>) -> InferenceDraft {
            InferenceDraft::new(
                    "Rust local KV cache route memory stores useful Noiron notes for replay and future routing.",
                    vec![ReasoningStep::new("runtime", "exported under fast path", 0.45)],
                )
                .with_exported_kv_blocks(vec![RuntimeKvBlock::new(
                    4,
                    2,
                    0,
                    4,
                    vec![0.2, 0.1],
                    vec![0.4, 0.3],
                )])
        }
    }

    let mut engine = NoironEngine::new();
    engine.router.restore_state(crate::router::RouterState {
        threshold: 0.88,
        observations: 0,
        profile_thresholds: crate::router::ProfileThresholds::from_single(0.88),
        profile_observations: crate::router::ProfileObservations::default(),
    });
    let mut backend = FastPathExportingBackend;

    let outcome = engine.infer(
        InferenceRequest::new("Rust local KV cache route memory", TaskProfile::Coding),
        &mut backend,
    );

    assert!(outcome.route_budget.attention_fraction < 0.10);
    assert_eq!(
        outcome.drift_report.severity,
        crate::drift::DriftSeverity::Watch
    );
    assert!(outcome.drift_report.allow_memory_write);
    assert!(!outcome.drift_report.allow_runtime_kv_write);
    assert!(
        outcome
            .drift_report
            .notes
            .iter()
            .any(|note| note == "route:fast_path_watch")
    );
    assert!(outcome.stored_memory_id.is_some());
    assert_eq!(outcome.exported_runtime_kv_blocks, 1);
    assert!(outcome.stored_runtime_kv_memory_ids.is_empty());
    assert!(
        engine
            .cache
            .entries()
            .iter()
            .all(|entry| !entry.key.starts_with("runtime_kv:"))
    );
}

#[derive(Debug, Clone)]
struct RuntimeKvMemoryFeedbackBackend {
    included: usize,
    skipped: usize,
    rejected: usize,
}

impl RuntimeKvMemoryFeedbackBackend {
    fn new(included: usize, skipped: usize, rejected: usize) -> Self {
        Self {
            included,
            skipped,
            rejected,
        }
    }
}

impl InferenceBackend for RuntimeKvMemoryFeedbackBackend {
    fn embed_text(&mut self, _text: &str) -> Option<Vec<f32>> {
        Some(vec![1.0, 0.0, 0.0])
    }

    fn generate(&mut self, _context: GenerationContext<'_>) -> InferenceDraft {
        InferenceDraft::new(
            "Rust runtime KV reuse memory keeps Noiron adaptive routing grounded with useful local cache evidence.",
            vec![ReasoningStep::new(
                "runtime_kv_feedback",
                "runtime kv memory feedback should update cache strength",
                0.92,
            )],
        )
        .with_runtime_diagnostics(RuntimeDiagnostics {
            model_id: Some("runtime-kv-feedback-test".to_owned()),
            selected_adapter: Some("portable-rust".to_owned()),
            forward_energy: Some(0.34),
            kv_influence: Some(0.62),
            imported_kv_blocks: self.included + self.skipped + self.rejected,
            runtime_kv_segments_included: self.included,
            runtime_kv_segments_skipped: self.skipped,
            runtime_kv_segments_rejected: self.rejected,
            ..RuntimeDiagnostics::default()
        })
    }
}

#[derive(Debug, Default, Clone)]
struct RuntimeKvImportFeedbackRuntime {
    request_import_counts: Vec<usize>,
}

impl crate::runtime::ModelRuntime for RuntimeKvImportFeedbackRuntime {
    fn metadata(&self) -> crate::runtime::RuntimeMetadata {
        crate::runtime::RuntimeMetadata::new("runtime-kv-import-feedback", "test-bpe", 512, 3)
            .with_kv_exchange(true, false)
            .with_kv_limits(1, 0)
    }

    fn architecture(&self) -> TransformerRuntimeArchitecture {
        TransformerRuntimeArchitecture::new(4, 3, 1, 1, 512)
    }

    fn embed_text(&self, _text: &str) -> Result<crate::runtime::RuntimeEmbedding, RuntimeError> {
        Ok(crate::runtime::RuntimeEmbedding::new(vec![1.0, 0.0, 0.0]))
    }

    fn import_kv(&mut self, blocks: &[RuntimeKvBlock]) -> Result<usize, RuntimeError> {
        Ok(blocks.len())
    }

    fn generate(
        &mut self,
        request: crate::runtime::RuntimeRequest,
    ) -> Result<crate::runtime::RuntimeResponse, RuntimeError> {
        let imported = request.imported_kv_blocks.len();
        self.request_import_counts.push(imported);

        let mut diagnostics = RuntimeDiagnostics {
            model_id: Some("runtime-kv-import-feedback".to_owned()),
            selected_adapter: Some("portable-rust".to_owned()),
            forward_energy: Some(0.34),
            kv_influence: Some(0.62),
            imported_kv_blocks: imported,
            ..RuntimeDiagnostics::default()
        };
        if imported > 0 {
            diagnostics.runtime_kv_segments_skipped = 3;
            diagnostics.runtime_kv_segments_rejected = 2;
        }

        let mut response = crate::runtime::RuntimeResponse::new(
            "Rust runtime KV reuse memory is certain but maybe unknown.",
        )
        .with_diagnostics(diagnostics);
        response.tokens = vec![RuntimeToken {
            text: "runtime".to_owned(),
            logprob: Some(-0.1),
            entropy: Some(0.2),
        }];
        Ok(response)
    }
}

#[test]
fn live_feedback_penalizes_low_yield_runtime_kv_memory() {
    let mut engine = NoironEngine::new();
    let runtime_kv_memory_id = seed_local_runtime_kv_memory(
        &mut engine,
        "runtime_kv:l0h0:0-1 :: reusable runtime kv",
        0.90,
    );
    let before = memory_strength(&engine, runtime_kv_memory_id);
    let mut backend = RuntimeKvMemoryFeedbackBackend::new(0, 3, 2);

    let outcome = engine.infer(
        InferenceRequest::new("Rust runtime KV reuse memory", TaskProfile::Coding),
        &mut backend,
    );
    let after = memory_strength(&engine, runtime_kv_memory_id);

    assert_eq!(
        outcome.runtime_diagnostics.runtime_kv_segment_yield(),
        Some(0.0)
    );
    assert_eq!(outcome.memory_feedback.penalized, 1);
    assert_eq!(outcome.memory_feedback.reinforced, 0);
    assert!(after < before);
    assert!(outcome.memory_feedback.penalty_amount >= 0.80);
}

#[test]
fn low_yield_runtime_kv_feedback_prevents_next_runtime_import() {
    let mut engine = NoironEngine::new();
    let runtime_kv_memory_id = seed_local_runtime_kv_memory(
        &mut engine,
        "runtime_kv:l0h0:0-1 :: reusable runtime kv",
        0.62,
    );
    let before = memory_strength(&engine, runtime_kv_memory_id);
    let mut backend = RuntimeBackend::new(RuntimeKvImportFeedbackRuntime::default());

    let first = engine.infer(
        InferenceRequest::new("Rust runtime KV reuse memory", TaskProfile::Coding),
        &mut backend,
    );
    let after_first = memory_strength(&engine, runtime_kv_memory_id);
    let second = engine.infer(
        InferenceRequest::new("Rust runtime KV reuse memory", TaskProfile::Coding),
        &mut backend,
    );

    assert_eq!(backend.runtime().request_import_counts, vec![1, 0]);
    assert_eq!(first.runtime_diagnostics.imported_kv_blocks, 1);
    assert_eq!(
        first.runtime_diagnostics.runtime_kv_segment_yield(),
        Some(0.0)
    );
    assert_eq!(first.memory_feedback.penalized, 1);
    assert!(first.stored_memory_id.is_none());
    assert!(after_first < before);
    assert!(after_first < 0.45);
    assert!(
        second
            .infini_memory_plan
            .local_window()
            .iter()
            .any(|item| item.id == runtime_kv_memory_id)
    );
    assert_eq!(second.runtime_diagnostics.imported_kv_blocks, 0);
    assert!(
        second
            .runtime_diagnostics
            .runtime_kv_segment_yield()
            .is_none()
    );
}

#[test]
fn live_feedback_reinforces_high_yield_runtime_kv_memory() {
    let mut engine = NoironEngine::new();
    let runtime_kv_memory_id = seed_local_runtime_kv_memory(
        &mut engine,
        "runtime_kv:l0h0:0-1 :: reusable runtime kv",
        0.90,
    );
    let before = memory_strength(&engine, runtime_kv_memory_id);
    let mut backend = RuntimeKvMemoryFeedbackBackend::new(3, 0, 0);

    let outcome = engine.infer(
        InferenceRequest::new("Rust runtime KV reuse memory", TaskProfile::Coding),
        &mut backend,
    );
    let after = memory_strength(&engine, runtime_kv_memory_id);

    assert_eq!(
        outcome.runtime_diagnostics.runtime_kv_segment_yield(),
        Some(1.0)
    );
    assert_eq!(outcome.memory_feedback.reinforced, 1);
    assert_eq!(outcome.memory_feedback.penalized, 0);
    assert!(after > before);
}

#[test]
fn production_runtime_kernel_flows_through_engine_feedback_and_runtime_kv() {
    let (asset_dir, weights, tokenizer) = create_runtime_assets("engine-production-kernel");
    let manifest = RuntimeManifest::self_developed(
        "engine-production-transformer",
        "engine-production-tokenizer",
        4096,
        64,
    )
    .with_architecture(TransformerRuntimeArchitecture::new(6, 64, 4, 2, 1024))
    .with_supported_devices(vec![DeviceClass::CpuOnly])
    .with_adapter_hints(vec![RuntimeAdapterHint::PortableRust])
    .with_kv_policy(RuntimeKvPolicy {
        import_enabled: true,
        export_enabled: true,
        max_import_blocks: 2,
        max_export_blocks: 2,
    })
    .with_assets(
        RuntimeAssetPaths::new()
            .with_weights(&weights)
            .with_tokenizer(&tokenizer),
    );
    let plan = crate::hardware::HardwareAllocator::new().plan(
        crate::hardware::HardwareSnapshot::new(DeviceClass::CpuOnly, 0.20, 0.10, 0.25, 0.10),
        TaskProfile::Coding,
        512,
        HierarchyWeights::default(),
    );
    let runtime = ProductionTransformerRuntime::from_manifest_for_plan(manifest, &plan)
        .unwrap()
        .with_kernel(EngineForwardKernel);
    let mut backend = RuntimeBackend::new(runtime);
    let mut engine = NoironEngine::new();
    engine.set_hardware_snapshot(HardwareSnapshot::new(
        DeviceClass::CpuOnly,
        0.20,
        0.10,
        0.25,
        0.10,
    ));

    let outcome = engine.infer(
        InferenceRequest::new(
            "Rust production forward kernel should export reusable KV memory",
            TaskProfile::Coding,
        ),
        &mut backend,
    );

    assert!(outcome.answer.contains("production kernel answer"));
    assert_eq!(outcome.runtime_token_metrics.token_count, 3);
    assert_eq!(outcome.runtime_token_metrics.entropy_count, 3);
    assert_eq!(
        outcome.runtime_diagnostics.model_id.as_deref(),
        Some("engine-production-transformer")
    );
    assert_eq!(
        outcome.runtime_diagnostics.selected_adapter.as_deref(),
        Some("portable-rust")
    );
    assert_eq!(outcome.runtime_diagnostics.layer_count, 6);
    assert_eq!(outcome.runtime_diagnostics.forward_energy, Some(0.31));
    assert_eq!(outcome.runtime_diagnostics.kv_influence, Some(0.22));
    assert_eq!(outcome.exported_runtime_kv_blocks, 1);
    assert_eq!(outcome.stored_runtime_kv_memory_ids.len(), 1);
    assert!(outcome.report.quality > 0.70);
    assert!(outcome.process_reward.total > 0.50);
    assert!(engine.cache.entries().iter().any(|entry| {
        TenantScopedKey::parse(&entry.key).is_some_and(|key| {
            key.lane == TenantResourceLane::RuntimeKv
                && key.local_key.starts_with("runtime_kv:l3h1")
                && key.local_key.contains("production_forward_kernel")
        })
    }));
    assert_eq!(backend.runtime().exported_kv_blocks().len(), 1);

    fs::remove_dir_all(asset_dir).unwrap();
}
