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

#[test]
fn live_feedback_penalizes_low_yield_runtime_kv_memory() {
    let mut engine = NoironEngine::new();
    let runtime_kv_memory_id = engine.cache.store_or_fuse(
        "runtime_kv:l0h0:0-1 :: reusable runtime kv",
        vec![1.0, 0.0, 0.0],
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
fn live_feedback_reinforces_high_yield_runtime_kv_memory() {
    let mut engine = NoironEngine::new();
    let runtime_kv_memory_id = engine.cache.store_or_fuse(
        "runtime_kv:l0h0:0-1 :: reusable runtime kv",
        vec![1.0, 0.0, 0.0],
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
        entry.key.contains("runtime_kv:l3h1") && entry.key.contains("production forward kernel")
    }));
    assert_eq!(backend.runtime().exported_kv_blocks().len(), 1);

    fs::remove_dir_all(asset_dir).unwrap();
}
