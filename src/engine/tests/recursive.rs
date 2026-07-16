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
fn replay_metrics_penalize_weak_runtime_kv_import_pressure() {
    let clean = replay_item_with_recursive_calls(Some(2));
    let mut weak_imports = replay_item_with_recursive_calls(Some(2));
    weak_imports.runtime_diagnostics = RuntimeDiagnostics {
        imported_kv_blocks: 1,
        weak_runtime_kv_imports_skipped: 3,
        ..RuntimeDiagnostics::default()
    };

    let clean_metrics = replay_metrics(&clean);
    let weak_metrics = replay_metrics(&weak_imports);

    assert!(weak_metrics.perplexity > clean_metrics.perplexity);
    assert!(weak_metrics.semantic_consistency < clean_metrics.semantic_consistency);
    assert!(weak_metrics.quality_score() < clean_metrics.quality_score());
}

#[test]
fn replay_metrics_penalize_rust_check_failures() {
    let clean = replay_item_with_recursive_calls(Some(2));
    let mut failed = replay_item_with_recursive_calls(Some(2));
    failed.rust_check_stats = Some(RustCheckReplayStats {
        passed: 0,
        failed: 1,
        diagnostic_chars: 600,
    });

    let clean_metrics = replay_metrics(&clean);
    let failed_metrics = replay_metrics(&failed);

    assert!(failed_metrics.perplexity > clean_metrics.perplexity);
    assert!(failed_metrics.semantic_consistency < clean_metrics.semantic_consistency);
    assert!(failed_metrics.contradiction_count > clean_metrics.contradiction_count);
    assert!(failed_metrics.quality_score() < clean_metrics.quality_score());
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
    store_local_memory(&mut cache, "Rust Noiron tiered memory", vector, 1.0);
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
    let chunk_count = outcome.recursive_schedule.chunk_count();
    let merge_calls = outcome
        .recursive_schedule
        .merge_rounds
        .iter()
        .map(|round| round.output_units)
        .sum::<usize>();
    assert!(chunk_count > 1);
    assert!(!outcome.recursive_schedule.merge_rounds.is_empty());
    assert_eq!(
        outcome.recursive_schedule.max_parallel_chunks,
        outcome.hardware_plan.execution.max_parallel_chunks
    );
    assert_eq!(
        outcome.recursive_schedule.execution_wave_count(),
        chunk_count.div_ceil(outcome.recursive_schedule.max_parallel_chunks)
    );
    assert_eq!(outcome.recursive_runtime_calls, chunk_count + merge_calls);
    assert!(outcome.answer.contains("Recursive Noiron merged answer"));
    assert!(outcome.answer.contains("Recursive schedule"));
}

#[test]
fn recursive_dispatch_receipt_distinguishes_serial_backend_from_parallel_plan() {
    let mut engine = NoironEngine::new();
    engine.recursive_scheduler = RecursiveScheduler::new(45, 45, 10, 4);
    engine.set_hardware_snapshot(HardwareSnapshot::new(
        DeviceClass::DiscreteGpu,
        0.05,
        0.05,
        0.10,
        0.05,
    ));
    let genes = &mut engine
        .genome_runtime_state
        .profile_mut(TaskProfile::General)
        .active
        .genes;
    for gene in genes.iter_mut() {
        gene.fitness = 0.20;
    }
    genes[0].fitness = 0.95;
    genes[1].fitness = 0.80;
    let mut backend = HeuristicBackend;

    let outcome = engine.infer(
        InferenceRequest::new(
            "benchmark DSpark paper throughput and verification scheduling",
            TaskProfile::General,
        ),
        &mut backend,
    );

    assert_eq!(
        outcome
            .reasoning_frame
            .routing_bias
            .confidence_prefix_selected,
        2
    );
    assert_eq!(outcome.recursive_schedule.execution_wave_count(), 2);
    assert_eq!(outcome.recursive_schedule.max_parallel_chunks, 2);
    assert_eq!(outcome.recursive_schedule.dispatched_wave_count, 2);
    assert_eq!(outcome.recursive_schedule.parallel_wave_count, 0);
    assert_eq!(outcome.recursive_schedule.max_dispatch_width, 1);
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

    let chunk_count = outcome.recursive_schedule.chunk_count();
    let merge_calls = outcome
        .recursive_schedule
        .merge_rounds
        .iter()
        .map(|round| round.output_units)
        .sum::<usize>();
    assert!(chunk_count > 1);
    assert!(!outcome.recursive_schedule.merge_rounds.is_empty());
    assert_eq!(outcome.recursive_runtime_calls, chunk_count + merge_calls);
    assert_eq!(backend.prompts.len(), outcome.recursive_runtime_calls);
    assert_eq!(
        backend
            .prompts
            .iter()
            .filter(|prompt| prompt.contains("Noiron recursive chunk"))
            .count(),
        chunk_count
    );
    assert_eq!(
        backend
            .prompts
            .iter()
            .filter(|prompt| prompt.contains("Noiron recursive merge round"))
            .count(),
        merge_calls
    );
}

#[test]
fn recursive_wave_contract_fails_closed_on_missing_drafts() {
    struct ShortWaveBackend;

    impl InferenceBackend for ShortWaveBackend {
        fn generate(&mut self, _context: GenerationContext<'_>) -> InferenceDraft {
            panic!("recursive generation must use the wave seam")
        }

        fn generate_wave_cancelable(
            &mut self,
            _contexts: &[GenerationContext<'_>],
            _should_cancel: &mut dyn FnMut() -> bool,
        ) -> (Vec<InferenceDraft>, bool) {
            (Vec::new(), false)
        }
    }

    let mut engine = NoironEngine::new();
    engine.recursive_scheduler = RecursiveScheduler::new(8, 6, 2, 2);
    let prompt = (0..14)
        .map(|index| format!("short_wave_{index}"))
        .collect::<Vec<_>>()
        .join(" ");
    let mut backend = ShortWaveBackend;

    let outcome = engine.infer(
        InferenceRequest::new(prompt, TaskProfile::LongDocument),
        &mut backend,
    );

    assert_eq!(outcome.recursive_runtime_calls, 0);
    assert!(
        outcome
            .raw_answer
            .contains("recursive wave 0 returned 0 drafts")
    );
    assert!(
        outcome
            .process_reward
            .notes
            .iter()
            .any(|note| note.contains("runtime_recursive_wave_contract_error"))
    );
}

#[test]
fn recursive_wave_contract_rejects_overfull_cancelled_results() {
    struct OverfullCancelledWaveBackend;

    impl InferenceBackend for OverfullCancelledWaveBackend {
        fn generate(&mut self, _context: GenerationContext<'_>) -> InferenceDraft {
            panic!("recursive generation must use the wave seam")
        }

        fn generate_wave_cancelable(
            &mut self,
            contexts: &[GenerationContext<'_>],
            _should_cancel: &mut dyn FnMut() -> bool,
        ) -> (Vec<InferenceDraft>, bool) {
            (
                (0..=contexts.len())
                    .map(|_| InferenceDraft::new("overfull", Vec::new()))
                    .collect(),
                true,
            )
        }
    }

    let mut engine = NoironEngine::new();
    engine.recursive_scheduler = RecursiveScheduler::new(8, 6, 2, 2);
    let prompt = (0..14)
        .map(|index| format!("overfull_wave_{index}"))
        .collect::<Vec<_>>()
        .join(" ");
    let mut backend = OverfullCancelledWaveBackend;

    let outcome = engine.infer(
        InferenceRequest::new(prompt, TaskProfile::LongDocument),
        &mut backend,
    );

    assert!(outcome.raw_answer.contains("recursive wave 0 returned"));
    assert!(outcome.raw_answer.contains("drafts for"));
    assert!(
        outcome
            .process_reward
            .notes
            .iter()
            .any(|note| note.contains("runtime_recursive_wave_contract_error"))
    );
}

#[test]
fn homeostatic_gate_downshifts_recursive_spawn_under_memory_pressure() {
    struct CountingBackend {
        prompts: Vec<String>,
    }

    impl InferenceBackend for CountingBackend {
        fn generate(&mut self, context: GenerationContext<'_>) -> InferenceDraft {
            self.prompts.push(context.prompt.to_owned());
            InferenceDraft::new(
                format!("draft {}", self.prompts.len()),
                vec![ReasoningStep::new("count", "counted direct call", 0.9)],
            )
        }
    }

    let mut engine = NoironEngine::new();
    engine.recursive_scheduler = RecursiveScheduler::new(8, 6, 2, 2);
    engine.set_hardware_snapshot(HardwareSnapshot::new(
        DeviceClass::CpuOnly,
        0.40,
        0.0,
        0.98,
        0.20,
    ));
    let prompt = (0..14)
        .map(|index| format!("pressure_chunk_{index}"))
        .collect::<Vec<_>>()
        .join(" ");
    let mut backend = CountingBackend {
        prompts: Vec::new(),
    };

    let outcome = engine.infer(
        InferenceRequest::new(prompt, TaskProfile::LongDocument),
        &mut backend,
    );

    assert_eq!(outcome.recursive_runtime_calls, 1);
    assert_eq!(backend.prompts.len(), 1);
    assert!(!outcome.recursive_schedule.requires_recursion);
    assert_eq!(
        outcome.homeostatic_gate.decision.as_str(),
        "downshift_parallelism"
    );
    assert!(!outcome.homeostatic_gate.recursive_spawn_allowed);
    assert!(!outcome.homeostatic_gate.memory_admission_allowed);
    assert!(!outcome.homeostatic_gate.genome_mutation_allowed);
    assert!(
        outcome
            .homeostatic_gate
            .reason_codes
            .contains(&"runtime_memory_pressure_high")
    );
    assert!(outcome.raw_answer.contains("draft 1"));
}

#[test]
fn homeostatic_gate_blocks_genome_mutation_candidates_under_memory_pressure() {
    struct BadBackend;

    impl InferenceBackend for BadBackend {
        fn generate(&mut self, _context: GenerationContext<'_>) -> InferenceDraft {
            InferenceDraft::new("", vec![ReasoningStep::new("runtime", "empty", 0.0)])
        }
    }

    let request =
        InferenceRequest::new("Rust Noiron rollback genome pressure", TaskProfile::Coding);
    let mut low_pressure = NoironEngine::new();
    let mut low_backend = BadBackend;
    let low = low_pressure.infer(request.clone(), &mut low_backend);
    assert!(low.homeostatic_gate.genome_mutation_allowed);
    assert!(low.reasoning_genome.scissors_proposal_count() > 0);

    let mut high_pressure = NoironEngine::new();
    high_pressure.set_hardware_snapshot(HardwareSnapshot::new(
        DeviceClass::CpuOnly,
        0.40,
        0.0,
        0.98,
        0.20,
    ));
    let mut high_backend = BadBackend;
    let high = high_pressure.infer(request, &mut high_backend);

    assert!(!high.homeostatic_gate.genome_mutation_allowed);
    assert_eq!(high.reasoning_genome.scissors_proposal_count(), 0);
    assert!(high.reasoning_genome.malignant_gene_count() > 0);
}

#[test]
fn hardware_parallel_budget_limits_recursive_execution_waves() {
    let mut engine = NoironEngine::new();
    engine.recursive_scheduler = RecursiveScheduler::new(8, 6, 2, 2);
    engine.set_hardware_snapshot(HardwareSnapshot::new(
        DeviceClass::Embedded,
        0.82,
        0.0,
        0.70,
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
                weak_runtime_kv_imports_skipped: 1,
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
    assert_eq!(
        outcome.runtime_diagnostics.weak_runtime_kv_imports_skipped,
        outcome.recursive_runtime_calls
    );
}

#[test]
fn recursive_inference_preserves_model_fallback_diagnostics() {
    struct FallbackDiagnosedBackend {
        calls: usize,
        mixed_models: bool,
        saturating_attempts: bool,
    }

    impl InferenceBackend for FallbackDiagnosedBackend {
        fn runtime_native_context_window(&self) -> Option<usize> {
            Some(4)
        }

        fn generate(&mut self, _context: GenerationContext<'_>) -> InferenceDraft {
            let call = self.calls;
            let all_failed = call == 0;
            self.calls += 1;
            let selected_model = (!all_failed).then(|| {
                if self.mixed_models && self.calls == 3 {
                    "newapi/other-fallback"
                } else {
                    "newapi/recursive-fallback"
                }
                .to_owned()
            });
            InferenceDraft::new(
                "recursive runtime fallback diagnostics",
                vec![ReasoningStep::new(
                    "newapi_fallback",
                    "preserved recursive fallback diagnostics",
                    0.91,
                )],
            )
            .with_runtime_diagnostics(RuntimeDiagnostics {
                model_fallback_configured: call == 0,
                model_fallback_primary_failed: call == 0,
                model_fallback_used: call == 1,
                model_fallback_attempts: if self.saturating_attempts && call == 0 {
                    usize::MAX
                } else {
                    2
                },
                model_fallback_failures: usize::from(all_failed) + 1,
                model_fallback_quarantined: 1,
                model_fallback_cooldown_skipped: 1,
                model_fallback_selected_model: selected_model,
                model_fallback_all_failed: all_failed,
                ..RuntimeDiagnostics::default()
            })
        }
    }

    let mut engine = NoironEngine::new();
    let mut backend = FallbackDiagnosedBackend {
        calls: 0,
        mixed_models: false,
        saturating_attempts: false,
    };
    let outcome = engine.infer(
        InferenceRequest::new("one two three four five six", TaskProfile::LongDocument),
        &mut backend,
    );
    let diagnostics = &outcome.runtime_diagnostics;

    assert!(outcome.recursive_schedule.requires_recursion);
    assert!(diagnostics.model_fallback_configured);
    assert!(diagnostics.model_fallback_primary_failed);
    assert!(diagnostics.model_fallback_used);
    assert_eq!(
        diagnostics.model_fallback_attempts,
        outcome.recursive_runtime_calls * 2
    );
    assert_eq!(
        diagnostics.model_fallback_failures,
        outcome.recursive_runtime_calls + 1
    );
    assert_eq!(
        diagnostics.model_fallback_quarantined,
        outcome.recursive_runtime_calls
    );
    assert_eq!(
        diagnostics.model_fallback_cooldown_skipped,
        outcome.recursive_runtime_calls
    );
    assert_eq!(
        diagnostics.model_fallback_selected_model.as_deref(),
        Some("newapi/recursive-fallback")
    );
    assert!(diagnostics.model_fallback_all_failed);

    let mut mixed_backend = FallbackDiagnosedBackend {
        calls: 0,
        mixed_models: true,
        saturating_attempts: false,
    };
    let mixed = engine.infer(
        InferenceRequest::new("one two three four five six", TaskProfile::LongDocument),
        &mut mixed_backend,
    );
    assert_eq!(
        mixed
            .runtime_diagnostics
            .model_fallback_selected_model
            .as_deref(),
        None
    );

    let mut saturating_backend = FallbackDiagnosedBackend {
        calls: 0,
        mixed_models: false,
        saturating_attempts: true,
    };
    let saturated = engine.infer(
        InferenceRequest::new("one two three four five six", TaskProfile::LongDocument),
        &mut saturating_backend,
    );
    assert_eq!(
        saturated.runtime_diagnostics.model_fallback_attempts,
        usize::MAX
    );
}
