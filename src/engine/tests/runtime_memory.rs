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
fn rust_native_adapter_self_learning_evidence_is_sanitized() {
    let private_prompt = "Rust runtime KV reuse memory prompt: tenant_id=prod-42 secret=sk-test-noiron answer: raw adapter output";
    let mut engine = NoironEngine::new();
    engine.cache.store_or_fuse(
        "runtime_kv:l0h0:0-1 :: Rust runtime KV reuse memory",
        vec![1.0, 0.0, 0.0],
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
        stored_runtime_kv_memory_ids: Vec::new(),
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
            && candidate.source_hash.starts_with("sha256:")
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
fn low_yield_runtime_kv_feedback_prevents_next_runtime_import() {
    let mut engine = NoironEngine::new();
    let runtime_kv_memory_id = engine.cache.store_or_fuse(
        "runtime_kv:l0h0:0-1 :: reusable runtime kv",
        vec![1.0, 0.0, 0.0],
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
