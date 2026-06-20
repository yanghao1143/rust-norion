use super::*;

#[test]
fn replay_metrics_penalize_excessive_recursive_runtime_calls() {
    let cheap = replay_item_with_recursive_calls(Some(2));
    let expensive = replay_item_with_recursive_calls(Some(96));

    let cheap_metrics = replay_metrics(&cheap);
    let expensive_metrics = replay_metrics(&expensive);

    assert!(expensive_metrics.perplexity > cheap_metrics.perplexity);
    assert!(expensive_metrics.semantic_consistency < cheap_metrics.semantic_consistency);
    assert!(expensive_metrics.quality_score() < cheap_metrics.quality_score());
}

#[test]
fn auto_replay_skips_when_hardware_pressure_is_high() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;

    engine.infer(
        InferenceRequest::new("build a Rust Noiron replay loop", TaskProfile::Coding),
        &mut backend,
    );
    engine.set_hardware_snapshot(HardwareSnapshot::new(
        DeviceClass::CpuOnly,
        0.98,
        0.90,
        0.96,
        0.80,
    ));
    let second = engine.infer(
        InferenceRequest::new("build a Rust Noiron replay loop", TaskProfile::Coding),
        &mut backend,
    );

    assert!(second.auto_replay_report.is_none());
}

#[test]
fn inference_exposes_tiered_cache_plan() {
    let mut cache = KvFusionCache::new();
    let vector = TextEmbedder::default().embed("Rust Noiron tiered memory");
    cache.store_or_fuse("Rust Noiron tiered memory", vector, 1.0);
    let mut engine = NoironEngine::with_cache(cache);
    let mut backend = HeuristicBackend;

    let outcome = engine.infer(
        InferenceRequest::new("Rust Noiron tiered memory", TaskProfile::Coding),
        &mut backend,
    );

    assert_eq!(outcome.tier_plan.placements().len(), 1);
    assert_eq!(outcome.tier_migrations.len(), 1);
    assert_eq!(outcome.infini_memory_plan.counts().local_window, 1);
    assert!(outcome.answer.contains("Tier plan"));
    assert!(outcome.answer.contains("Infini memory"));
}

#[test]
fn inference_exposes_recursive_schedule_for_long_prompt() {
    let mut engine = NoironEngine::new();
    engine.recursive_scheduler = RecursiveScheduler::new(8, 6, 2, 2);
    let prompt = (0..14)
        .map(|index| format!("chunk_token_{index}"))
        .collect::<Vec<_>>()
        .join(" ");
    let mut backend = HeuristicBackend;

    let outcome = engine.infer(
        InferenceRequest::new(prompt, TaskProfile::LongDocument),
        &mut backend,
    );

    assert!(outcome.recursive_schedule.requires_recursion);
    assert_eq!(outcome.recursive_schedule.chunk_count(), 3);
    assert_eq!(outcome.recursive_schedule.merge_round_count(), 2);
    assert_eq!(
        outcome.recursive_schedule.max_parallel_chunks,
        outcome.hardware_plan.execution.max_parallel_chunks
    );
    assert_eq!(outcome.recursive_schedule.execution_wave_count(), 2);
    assert_eq!(outcome.recursive_runtime_calls, 6);
    assert!(outcome.answer.contains("Recursive Noiron merged answer"));
    assert!(outcome.answer.contains("Recursive schedule"));
}

#[test]
fn recursive_inference_calls_backend_for_chunks_and_merges() {
    struct CountingBackend {
        prompts: Vec<String>,
    }

    impl InferenceBackend for CountingBackend {
        fn generate(&mut self, context: GenerationContext<'_>) -> InferenceDraft {
            self.prompts.push(context.prompt.to_owned());
            InferenceDraft::new(
                format!("draft {}", self.prompts.len()),
                vec![ReasoningStep::new("count", "counted recursive call", 0.9)],
            )
        }
    }

    let mut engine = NoironEngine::new();
    engine.recursive_scheduler = RecursiveScheduler::new(8, 6, 2, 2);
    let prompt = (0..14)
        .map(|index| format!("recursive_call_{index}"))
        .collect::<Vec<_>>()
        .join(" ");
    let mut backend = CountingBackend {
        prompts: Vec::new(),
    };

    let outcome = engine.infer(
        InferenceRequest::new(prompt, TaskProfile::LongDocument),
        &mut backend,
    );

    assert_eq!(outcome.recursive_schedule.chunk_count(), 3);
    assert_eq!(outcome.recursive_schedule.merge_round_count(), 2);
    assert_eq!(outcome.recursive_runtime_calls, 6);
    assert_eq!(backend.prompts.len(), outcome.recursive_runtime_calls);
    assert!(
        backend
            .prompts
            .iter()
            .filter(|prompt| prompt.contains("Noiron recursive chunk"))
            .count()
            >= 3
    );
    assert!(
        backend
            .prompts
            .iter()
            .filter(|prompt| prompt.contains("Noiron recursive merge round"))
            .count()
            >= 2
    );
}

#[test]
fn hardware_parallel_budget_limits_recursive_execution_waves() {
    let mut engine = NoironEngine::new();
    engine.recursive_scheduler = RecursiveScheduler::new(8, 6, 2, 2);
    engine.set_hardware_snapshot(HardwareSnapshot::new(
        DeviceClass::Embedded,
        0.82,
        0.0,
        0.82,
        0.55,
    ));
    let prompt = (0..14)
        .map(|index| format!("edge_chunk_{index}"))
        .collect::<Vec<_>>()
        .join(" ");
    let mut backend = HeuristicBackend;

    let outcome = engine.infer(
        InferenceRequest::new(prompt, TaskProfile::LongDocument),
        &mut backend,
    );

    assert_eq!(outcome.hardware_plan.execution.max_parallel_chunks, 1);
    assert_eq!(outcome.recursive_schedule.max_parallel_chunks, 1);
    assert_eq!(
        outcome.recursive_schedule.execution_wave_count(),
        outcome.recursive_schedule.chunk_count()
    );
}

#[test]
fn inference_uses_backend_native_window_for_recursive_schedule() {
    struct SmallWindowBackend;

    impl InferenceBackend for SmallWindowBackend {
        fn runtime_native_context_window(&self) -> Option<usize> {
            Some(4)
        }

        fn generate(&mut self, context: GenerationContext<'_>) -> InferenceDraft {
            InferenceDraft::new(
                format!(
                    "native window {} chunks {}",
                    context.recursive_schedule.native_window_tokens,
                    context.recursive_schedule.chunk_count()
                ),
                vec![ReasoningStep::new("runtime", "used native window", 0.9)],
            )
        }
    }

    let mut engine = NoironEngine::new();
    let mut backend = SmallWindowBackend;

    let outcome = engine.infer(
        InferenceRequest::new("one two three four five six", TaskProfile::LongDocument),
        &mut backend,
    );

    assert!(outcome.recursive_schedule.requires_recursion);
    assert_eq!(outcome.recursive_schedule.native_window_tokens, 4);
    assert!(outcome.recursive_schedule.chunk_count() > 1);
    assert!(outcome.answer.contains("native window 4"));
}

#[test]
fn recursive_inference_preserves_runtime_device_execution_diagnostics() {
    struct DeviceDiagnosedBackend;

    impl InferenceBackend for DeviceDiagnosedBackend {
        fn runtime_native_context_window(&self) -> Option<usize> {
            Some(4)
        }

        fn generate(&mut self, context: GenerationContext<'_>) -> InferenceDraft {
            let execution = &context.hardware_plan.execution;
            InferenceDraft::new(
                "recursive runtime device execution diagnostics",
                vec![ReasoningStep::new(
                    "runtime",
                    "preserved device execution diagnostics",
                    0.91,
                )],
            )
            .with_runtime_diagnostics(RuntimeDiagnostics {
                model_id: Some("recursive-device-diagnostics-test".to_owned()),
                selected_adapter: execution
                    .adapter_hints
                    .first()
                    .map(|adapter| adapter.as_str().to_owned()),
                layer_count: 6,
                global_layers: 2,
                local_window_layers: 2,
                convolutional_fusion_layers: 2,
                hidden_size: 64,
                local_window_tokens: 4,
                forward_energy: Some(0.25),
                kv_influence: Some(0.33),
                ..RuntimeDiagnostics::default().with_device_execution(
                    context.hardware_plan.device.as_str(),
                    execution.primary_lane.as_str(),
                    execution.fallback_lane.as_str(),
                    execution.memory_mode.as_str(),
                )
            })
        }
    }

    let mut engine = NoironEngine::new();
    engine.set_hardware_snapshot(HardwareSnapshot::new(
        DeviceClass::Microcontroller,
        0.62,
        0.0,
        0.72,
        0.55,
    ));
    let mut backend = DeviceDiagnosedBackend;

    let outcome = engine.infer(
        InferenceRequest::new("one two three four five six", TaskProfile::LongDocument),
        &mut backend,
    );

    assert!(outcome.recursive_schedule.requires_recursion);
    assert_eq!(
        outcome.runtime_diagnostics.device_profile.as_deref(),
        Some(outcome.hardware_plan.device.as_str())
    );
    assert_eq!(
        outcome.runtime_diagnostics.primary_lane.as_deref(),
        Some(outcome.hardware_plan.execution.primary_lane.as_str())
    );
    assert_eq!(
        outcome.runtime_diagnostics.fallback_lane.as_deref(),
        Some(outcome.hardware_plan.execution.fallback_lane.as_str())
    );
    assert_eq!(
        outcome.runtime_diagnostics.memory_mode.as_deref(),
        Some(outcome.hardware_plan.execution.memory_mode.as_str())
    );
}
