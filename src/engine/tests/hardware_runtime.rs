use super::*;

#[test]
fn inference_uses_hardware_pressure_for_latency_and_kv_budget() {
    let mut cache = KvFusionCache::new();
    cache.store_or_fuse("hardware constrained memory", vec![1.0, 0.0, 0.0], 1.0);
    let mut engine = NoironEngine::with_cache(cache);
    engine.set_hardware_snapshot(HardwareSnapshot::new(
        DeviceClass::CpuOnly,
        0.95,
        0.0,
        0.90,
        0.50,
    ));
    let mut backend = HeuristicBackend;

    let outcome = engine.infer(
        InferenceRequest::new("hardware constrained memory", TaskProfile::LongDocument),
        &mut backend,
    );

    assert!(outcome.hardware_plan.latency_budget_ms.is_some());
    assert!(outcome.hardware_plan.local_kv_token_budget < 512);
    assert!(outcome.hardware_plan.global_kv_token_budget < 4096);
    assert!(outcome.answer.contains("Hardware plan"));
}

#[test]
fn hardware_pressure_flows_into_route_budget() {
    let prompt = (0..8)
        .map(|index| format!("ComputeA{index}B{index}C{index}D"))
        .collect::<Vec<_>>()
        .join(" ");
    let mut roomy_engine = NoironEngine::new();
    roomy_engine.set_hardware_snapshot(HardwareSnapshot::new(
        DeviceClass::Server,
        0.10,
        0.15,
        0.20,
        0.10,
    ));
    let mut constrained_engine = NoironEngine::new();
    constrained_engine.set_hardware_snapshot(HardwareSnapshot::new(
        DeviceClass::Embedded,
        0.95,
        0.0,
        0.92,
        0.70,
    ));
    let mut roomy_backend = HeuristicBackend;
    let mut constrained_backend = HeuristicBackend;

    let roomy = roomy_engine.infer(
        InferenceRequest::new(prompt.clone(), TaskProfile::Coding),
        &mut roomy_backend,
    );
    let constrained = constrained_engine.infer(
        InferenceRequest::new(prompt, TaskProfile::Coding),
        &mut constrained_backend,
    );

    assert!(roomy.hardware_plan.compute_headroom() > constrained.hardware_plan.compute_headroom());
    assert!(roomy.route_budget.attention_fraction > constrained.route_budget.attention_fraction);
}

#[test]
fn cache_hits_flow_into_route_budget_to_reduce_attention() {
    let prompt = (0..8)
        .map(|index| format!("CacheHitA{index}B{index}C{index}D"))
        .collect::<Vec<_>>()
        .join(" ");
    let mut uncached_engine = NoironEngine::new();
    let mut cached_cache = KvFusionCache::with_limits(0.99, 16);
    let query = TextEmbedder::default().embed(&prompt);
    store_local_memory(&mut cached_cache, "cache hit memory 1", query.clone(), 1.0);
    store_local_memory(
        &mut cached_cache,
        "cache hit memory 2",
        perturbed_vector(&query, 1),
        1.0,
    );
    store_local_memory(
        &mut cached_cache,
        "cache hit memory 3",
        perturbed_vector(&query, 2),
        1.0,
    );
    store_local_memory(
        &mut cached_cache,
        "cache hit memory 4",
        perturbed_vector(&query, 3),
        1.0,
    );
    let mut cached_engine = NoironEngine::with_cache(cached_cache);
    let mut uncached_backend = HeuristicBackend;
    let mut cached_backend = HeuristicBackend;

    let uncached = uncached_engine.infer(
        InferenceRequest::new(prompt.clone(), TaskProfile::Coding),
        &mut uncached_backend,
    );
    let cached = cached_engine.infer(
        InferenceRequest::new(prompt, TaskProfile::Coding),
        &mut cached_backend,
    );

    assert!(uncached.used_memories.is_empty());
    assert_eq!(cached.used_memories.len(), 4);
    assert!(uncached.route_budget.attention_tokens > cached.route_budget.attention_tokens);
    assert!(
        uncached.route_budget.attention_fraction > cached.route_budget.attention_fraction,
        "uncached={:?} cached={:?}",
        uncached.route_budget,
        cached.route_budget
    );
}

#[test]
fn runtime_token_uncertainty_raises_generation_perplexity() {
    let low_entropy = InferenceDraft::new(
        "A stable local runtime answer with enough detail to pass reflection.",
        vec![],
    )
    .with_tokens(vec![
        DraftToken {
            text: "stable".to_owned(),
            logprob: Some(-0.05),
            entropy: Some(0.05),
        },
        DraftToken {
            text: "answer".to_owned(),
            logprob: Some(-0.08),
            entropy: Some(0.08),
        },
    ]);
    let high_entropy = InferenceDraft::new(
        "A stable local runtime answer with enough detail to pass reflection.",
        vec![],
    )
    .with_tokens(vec![
        DraftToken {
            text: "unstable".to_owned(),
            logprob: Some(-2.5),
            entropy: Some(0.95),
        },
        DraftToken {
            text: "answer".to_owned(),
            logprob: Some(-1.8),
            entropy: Some(0.85),
        },
    ]);
    let report = ReflectionReport {
        quality: 0.88,
        contradictions: Vec::new(),
        issues: Vec::new(),
        revision_actions: Vec::new(),
        revision_passes: 0,
        revised_answer: low_entropy.answer.clone(),
        store_as_memory: true,
        lesson: "runtime token metrics should affect Noiron feedback".to_owned(),
    };
    let budget = RouteBudget {
        threshold: 0.5,
        attention_tokens: 1,
        fast_tokens: 3,
        attention_fraction: 0.25,
    };

    let low_token_metrics = RuntimeTokenMetrics::from_draft(&low_entropy);
    let high_token_metrics = RuntimeTokenMetrics::from_draft(&high_entropy);
    let low = metrics_from_report(&low_entropy, &report, budget, low_token_metrics);
    let high = metrics_from_report(&high_entropy, &report, budget, high_token_metrics);

    assert!(
        high_token_metrics.average_entropy.unwrap() > low_token_metrics.average_entropy.unwrap()
    );
    assert!(
        high_token_metrics.uncertainty_perplexity.unwrap()
            > low_token_metrics.uncertainty_perplexity.unwrap()
    );
    assert!(high.perplexity > low.perplexity + 2.0);
    assert_eq!(high.semantic_consistency, low.semantic_consistency);
}

#[test]
fn runtime_token_metrics_ignore_non_finite_runtime_values() {
    let draft =
        InferenceDraft::new("runtime returned partial token metadata", vec![]).with_tokens(vec![
            DraftToken {
                text: "bad-entropy".to_owned(),
                logprob: Some(f32::NAN),
                entropy: Some(f32::INFINITY),
            },
            DraftToken {
                text: "valid".to_owned(),
                logprob: Some(-0.5),
                entropy: Some(0.25),
            },
        ]);

    let metrics = RuntimeTokenMetrics::from_draft(&draft);

    assert_eq!(metrics.token_count, 2);
    assert_eq!(metrics.entropy_count, 1);
    assert_eq!(metrics.logprob_count, 1);
    assert_eq!(metrics.average_entropy, Some(0.25));
    assert_eq!(metrics.average_neg_logprob, Some(0.5));
    assert_eq!(metrics.uncertainty_perplexity, Some(3.5));
}

#[test]
fn inference_outcome_exposes_runtime_adapter_observations() {
    struct DiagnosedBackend;

    impl InferenceBackend for DiagnosedBackend {
        fn generate(&mut self, _context: GenerationContext<'_>) -> InferenceDraft {
            InferenceDraft::new(
                "A stable adapter-aware runtime answer with useful control detail.",
                vec![ReasoningStep::new(
                    "runtime",
                    "selected a historically useful adapter",
                    0.91,
                )],
            )
            .with_runtime_diagnostics(RuntimeDiagnostics {
                model_id: Some("self-transformer-test".to_owned()),
                selected_adapter: Some(RuntimeAdapterHint::CpuSimd.as_str().to_owned()),
                layer_count: 6,
                hidden_size: 128,
                local_window_tokens: 4096,
                forward_energy: Some(0.20),
                kv_influence: Some(0.46),
                imported_kv_blocks: 1,
                exported_kv_blocks: 1,
                ..RuntimeDiagnostics::default()
            })
        }
    }

    let mut engine = NoironEngine::new();
    let memory_id = store_local_memory(
        &mut engine.cache,
        "adapter observation history",
        TextEmbedder::default().embed("adapter observation history"),
        0.9,
    );
    engine.experience.record(ExperienceInput {
        prompt: "adapter observation history".to_owned(),
        profile: TaskProfile::Coding,
        lesson: "prefer cpu SIMD when prior self-developed runtime reward is strong".to_owned(),
        quality: 0.92,
        contradictions: Vec::new(),
        reflection_issues: Vec::new(),
        revision_actions: Vec::new(),
        stored_memory_id: None,
        router_threshold_after: 0.55,
        stream_windows: 1,
        route_budget: RouteBudget {
            threshold: 0.55,
            attention_tokens: 2,
            fast_tokens: 2,
            attention_fraction: 0.5,
        },
        hierarchy: HierarchyWeights::new(0.2, 0.6, 0.2),
        used_memory_ids: vec![memory_id],
        gist_records: Vec::new(),
        gist_memory_ids: Vec::new(),
        stored_runtime_kv_memory_ids: Vec::new(),
        runtime_diagnostics: RuntimeDiagnostics {
            model_id: Some("self-transformer-test".to_owned()),
            selected_adapter: Some(RuntimeAdapterHint::CpuSimd.as_str().to_owned()),
            layer_count: 6,
            hidden_size: 128,
            local_window_tokens: 4096,
            forward_energy: Some(0.18),
            kv_influence: Some(0.50),
            imported_kv_blocks: 1,
            exported_kv_blocks: 2,
            ..RuntimeDiagnostics::default()
        },
        runtime_token_metrics: Default::default(),
        process_reward: ProcessRewardReport {
            total: 0.90,
            action: RewardAction::Reinforce,
            components: ProcessRewardComponents::default(),
            notes: Vec::new(),
        },
        live_evolution: Default::default(),
    });
    let mut backend = DiagnosedBackend;

    let outcome = engine.infer(
        InferenceRequest::new("adapter observation history", TaskProfile::Coding),
        &mut backend,
    );

    assert_eq!(outcome.runtime_adapter_observations.len(), 1);
    assert_eq!(
        outcome.runtime_adapter_observations[0].adapter,
        RuntimeAdapterHint::CpuSimd
    );
    assert!(outcome.runtime_adapter_observations[0].score > 0.80);
}

#[test]
fn inference_outcome_filters_adapter_observations_to_device_plan() {
    struct DiagnosedBackend;

    impl InferenceBackend for DiagnosedBackend {
        fn generate(&mut self, _context: GenerationContext<'_>) -> InferenceDraft {
            InferenceDraft::new(
                "A stable CPU runtime answer that should ignore unavailable CUDA history.",
                vec![ReasoningStep::new(
                    "runtime",
                    "selected a device-valid adapter",
                    0.91,
                )],
            )
            .with_runtime_diagnostics(RuntimeDiagnostics {
                model_id: Some("self-transformer-test".to_owned()),
                selected_adapter: Some(RuntimeAdapterHint::CpuSimd.as_str().to_owned()),
                layer_count: 6,
                hidden_size: 128,
                local_window_tokens: 4096,
                forward_energy: Some(0.20),
                kv_influence: Some(0.46),
                imported_kv_blocks: 1,
                exported_kv_blocks: 1,
                ..RuntimeDiagnostics::default()
            })
        }
    }

    let mut engine = NoironEngine::new();
    engine.set_hardware_snapshot(HardwareSnapshot::new(
        DeviceClass::CpuOnly,
        0.35,
        0.0,
        0.45,
        0.20,
    ));
    let memory_id = store_local_memory(
        &mut engine.cache,
        "adapter observation history",
        TextEmbedder::default().embed("adapter observation history"),
        0.9,
    );
    engine.experience.record(ExperienceInput {
        prompt: "adapter observation history".to_owned(),
        profile: TaskProfile::Coding,
        lesson: "prefer unavailable cuda when prior score is high".to_owned(),
        quality: 0.99,
        contradictions: Vec::new(),
        reflection_issues: Vec::new(),
        revision_actions: Vec::new(),
        stored_memory_id: None,
        router_threshold_after: 0.55,
        stream_windows: 1,
        route_budget: RouteBudget {
            threshold: 0.55,
            attention_tokens: 2,
            fast_tokens: 2,
            attention_fraction: 0.5,
        },
        hierarchy: HierarchyWeights::new(0.2, 0.6, 0.2),
        used_memory_ids: vec![memory_id],
        gist_records: Vec::new(),
        gist_memory_ids: Vec::new(),
        stored_runtime_kv_memory_ids: Vec::new(),
        runtime_diagnostics: RuntimeDiagnostics {
            model_id: Some("self-transformer-test".to_owned()),
            selected_adapter: Some(RuntimeAdapterHint::Cuda.as_str().to_owned()),
            layer_count: 6,
            hidden_size: 128,
            local_window_tokens: 4096,
            forward_energy: Some(0.05),
            kv_influence: Some(0.90),
            imported_kv_blocks: 2,
            exported_kv_blocks: 2,
            ..RuntimeDiagnostics::default()
        },
        runtime_token_metrics: Default::default(),
        process_reward: ProcessRewardReport {
            total: 0.99,
            action: RewardAction::Reinforce,
            components: ProcessRewardComponents::default(),
            notes: Vec::new(),
        },
        live_evolution: Default::default(),
    });
    engine.experience.record(ExperienceInput {
        prompt: "adapter observation history".to_owned(),
        profile: TaskProfile::Coding,
        lesson: "prefer cpu SIMD when current CPU plan allows it".to_owned(),
        quality: 0.88,
        contradictions: Vec::new(),
        reflection_issues: Vec::new(),
        revision_actions: Vec::new(),
        stored_memory_id: None,
        router_threshold_after: 0.55,
        stream_windows: 1,
        route_budget: RouteBudget {
            threshold: 0.55,
            attention_tokens: 2,
            fast_tokens: 2,
            attention_fraction: 0.5,
        },
        hierarchy: HierarchyWeights::new(0.2, 0.6, 0.2),
        used_memory_ids: vec![memory_id],
        gist_records: Vec::new(),
        gist_memory_ids: Vec::new(),
        stored_runtime_kv_memory_ids: Vec::new(),
        runtime_diagnostics: RuntimeDiagnostics {
            model_id: Some("self-transformer-test".to_owned()),
            selected_adapter: Some(RuntimeAdapterHint::CpuSimd.as_str().to_owned()),
            layer_count: 6,
            hidden_size: 128,
            local_window_tokens: 4096,
            forward_energy: Some(0.18),
            kv_influence: Some(0.40),
            imported_kv_blocks: 1,
            exported_kv_blocks: 1,
            ..RuntimeDiagnostics::default()
        },
        runtime_token_metrics: Default::default(),
        process_reward: ProcessRewardReport {
            total: 0.86,
            action: RewardAction::Reinforce,
            components: ProcessRewardComponents::default(),
            notes: Vec::new(),
        },
        live_evolution: Default::default(),
    });
    let mut backend = DiagnosedBackend;

    let outcome = engine.infer(
        InferenceRequest::new("adapter observation history", TaskProfile::Coding),
        &mut backend,
    );

    assert_eq!(outcome.runtime_adapter_observations.len(), 1);
    assert_eq!(
        outcome.runtime_adapter_observations[0].adapter,
        RuntimeAdapterHint::CpuSimd
    );
    assert!(
        !outcome
            .runtime_adapter_observations
            .iter()
            .any(|observation| observation.adapter == RuntimeAdapterHint::Cuda)
    );
}
