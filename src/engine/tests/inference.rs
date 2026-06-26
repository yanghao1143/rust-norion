use super::*;

#[test]
fn inference_updates_router_and_memory() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new("build a Rust Noiron routing cache", TaskProfile::Coding),
        &mut backend,
    );

    assert!(outcome.answer.contains("Noiron"));
    assert!(outcome.stored_memory_id.is_some());
    assert!(!outcome.stream_reports.is_empty());
    let online_reward_feedback = outcome
        .process_reward
        .notes
        .iter()
        .any(|note| note.starts_with("online_reward_feedback:"));
    assert_eq!(
        engine.router.observations(),
        outcome.stream_reports.len() as u64 + 1 + u64::from(online_reward_feedback)
    );
    assert_eq!(engine.experience.len(), 1);
    assert_eq!(outcome.experience_id, 1);
    assert!(
        engine.experience.records()[0]
            .lesson
            .contains("reuse_response:")
    );
    assert!(
        !engine.experience.records()[0]
            .lesson
            .contains("accepted_pattern")
    );
    assert!(outcome.process_reward.total > 0.0);
    assert!(
        (engine.experience.records()[0].process_reward.total - outcome.process_reward.total).abs()
            < 0.0001
    );
    assert!(!outcome.transformer_plan.is_empty());
    assert!(!engine.cache.is_empty());
}

#[derive(Debug, Clone)]
struct RuntimeEmbeddingBackend;

impl InferenceBackend for RuntimeEmbeddingBackend {
    fn embed_text(&mut self, text: &str) -> Option<Vec<f32>> {
        Some(vec![
            1.0,
            text.len() as f32,
            text.bytes().fold(0_u32, |sum, byte| sum + u32::from(byte)) as f32,
        ])
    }

    fn generate(&mut self, _context: GenerationContext<'_>) -> InferenceDraft {
        InferenceDraft::new(
            "Build a Rust Noiron runtime embedding audit path that stores model-side vectors.",
            vec![ReasoningStep::new(
                "embedding",
                "runtime supplied model-side memory vector",
                0.92,
            )],
        )
    }
}

#[derive(Debug, Clone)]
struct TimeoutErrorBackend;

impl InferenceBackend for TimeoutErrorBackend {
    fn generate(&mut self, _context: GenerationContext<'_>) -> InferenceDraft {
        InferenceDraft::new(
            "Runtime backend error: runtime command mistralrs timed out after 1000 ms",
            vec![ReasoningStep::new(
                "runtime_error",
                "runtime command mistralrs timed out after 1000 ms",
                0.0,
            )],
        )
    }
}

#[derive(Debug, Clone)]
struct RuntimeKvSegmentDiagnosticsBackend;

impl InferenceBackend for RuntimeKvSegmentDiagnosticsBackend {
    fn generate(&mut self, _context: GenerationContext<'_>) -> InferenceDraft {
        let diagnostics = RuntimeDiagnostics {
            model_id: Some("native-kv-segment-test".to_owned()),
            selected_adapter: Some("portable-rust".to_owned()),
            forward_energy: Some(0.41),
            kv_influence: Some(0.50),
            imported_kv_blocks: 2,
            exported_kv_blocks: 1,
            runtime_kv_segments_included: 2,
            runtime_kv_segments_skipped: 1,
            runtime_kv_segments_rejected: 0,
            ..RuntimeDiagnostics::default()
        };

        InferenceDraft::new(
            "Runtime KV segment diagnostics should become process reward evidence.",
            vec![ReasoningStep::new(
                "runtime_kv_segments",
                "included=2 skipped=1 rejected=0",
                0.91,
            )],
        )
        .with_runtime_diagnostics(diagnostics)
    }
}

#[derive(Debug, Clone)]
struct RuntimeKvRouteYieldBackend {
    included: usize,
    skipped: usize,
    rejected: usize,
    budget_limited_skipped: usize,
}

impl RuntimeKvRouteYieldBackend {
    fn new(included: usize, skipped: usize, rejected: usize) -> Self {
        Self {
            included,
            skipped,
            rejected,
            budget_limited_skipped: 0,
        }
    }

    fn with_budget_limited_skipped(mut self, skipped: usize) -> Self {
        self.budget_limited_skipped = skipped;
        self
    }
}

impl InferenceBackend for RuntimeKvRouteYieldBackend {
    fn generate(&mut self, _context: GenerationContext<'_>) -> InferenceDraft {
        let total_segments = self
            .included
            .saturating_add(self.skipped)
            .saturating_add(self.rejected);
        let diagnostics = RuntimeDiagnostics {
            model_id: Some("native-kv-route-yield-test".to_owned()),
            selected_adapter: Some("portable-rust".to_owned()),
            forward_energy: Some(0.38),
            kv_influence: Some(0.64),
            imported_kv_blocks: total_segments,
            exported_kv_blocks: 1,
            budget_limited_runtime_kv_imports_skipped: self.budget_limited_skipped,
            runtime_kv_segments_included: self.included,
            runtime_kv_segments_skipped: self.skipped,
            runtime_kv_segments_rejected: self.rejected,
            ..RuntimeDiagnostics::default()
        };

        InferenceDraft::new(
            "Rust Noiron runtime KV evidence routes through adaptive routing and compute budget feedback.",
            vec![ReasoningStep::new(
                "runtime_kv_route_yield",
                "runtime kv segment yield should affect the next route candidate",
                0.90,
            )],
        )
        .with_exported_kv_blocks(vec![RuntimeKvBlock::new(
            0,
            0,
            0,
            1,
            vec![0.1, 0.2, 0.3],
            vec![0.3, 0.2, 0.1],
        )])
        .with_runtime_diagnostics(diagnostics)
    }
}

#[derive(Debug, Clone)]
struct OrchestrationTraceBackend;

impl InferenceBackend for OrchestrationTraceBackend {
    fn embed_text(&mut self, _text: &str) -> Option<Vec<f32>> {
        Some(vec![0.9, 0.2, 0.1])
    }

    fn generate(&mut self, _context: GenerationContext<'_>) -> InferenceDraft {
        InferenceDraft::new(
            "Rust Noiron orchestration audit routes Rust coding context through disk-backed KV memory, adaptive routing, reasoning genome splicing, runtime KV evidence, model adapter execution, reflection validation, and approval gated memory admission. The trace keeps context as digest counts and stage summaries while durable memory, genome, and experiment ledgers remain approval gated.",
            vec![
                ReasoningStep::new(
                    "route",
                    "selected adaptive routing candidates from context counters and memory hits",
                    0.91,
                ),
                ReasoningStep::new(
                    "runtime_adapter",
                    "generated with local deterministic adapter and exported bounded KV evidence",
                    0.89,
                ),
                ReasoningStep::new(
                    "genome_splice",
                    "previewed reasoning genome segments behind read-only scissors gates",
                    0.88,
                ),
                ReasoningStep::new(
                    "memory_admission",
                    "kept durable memory ledger records preview-only until approval",
                    0.90,
                ),
            ],
        )
        .with_exported_kv_blocks(vec![RuntimeKvBlock::new(
            0,
            0,
            0,
            3,
            vec![0.1, 0.2, 0.3],
            vec![0.3, 0.2, 0.1],
        )])
    }
}

#[test]
fn inference_records_runtime_error_notes_for_inspection() {
    let mut engine = NoironEngine::new();
    let mut backend = TimeoutErrorBackend;

    let outcome = engine.infer(
        InferenceRequest::new("bounded runtime error audit", TaskProfile::Coding),
        &mut backend,
    );

    assert!(
        outcome
            .process_reward
            .notes
            .iter()
            .any(|note| note.starts_with("runtime_error:")
                && note.contains("timeout=true")
                && note.contains("message_chars="))
    );
    assert!(
        engine.experience.records()[0]
            .process_reward
            .notes
            .iter()
            .any(|note| note.starts_with("runtime_error:"))
    );
}

#[test]
fn inference_records_runtime_kv_segment_reward_notes() {
    let mut engine = NoironEngine::new();
    let mut backend = RuntimeKvSegmentDiagnosticsBackend;

    let outcome = engine.infer(
        InferenceRequest::new("audit native kv segment hooks", TaskProfile::Coding),
        &mut backend,
    );

    assert!(outcome.process_reward.notes.iter().any(|note| {
        note == "runtime_kv_segments:included=2:skipped=1:rejected=0:total=3:yield=0.583"
    }));
    assert!(
        engine.experience.records()[0]
            .process_reward
            .notes
            .iter()
            .any(|note| note
                == "runtime_kv_segments:included=2:skipped=1:rejected=0:total=3:yield=0.583")
    );
}

#[test]
fn low_runtime_kv_segment_yield_downweights_adaptive_route_candidate() {
    let mut efficient_engine = NoironEngine::new();
    let mut efficient_backend = RuntimeKvRouteYieldBackend::new(3, 0, 0);
    let efficient = efficient_engine.infer(
        InferenceRequest::new("audit runtime kv routing yield", TaskProfile::Coding),
        &mut efficient_backend,
    );

    let mut wasteful_engine = NoironEngine::new();
    let mut wasteful_backend = RuntimeKvRouteYieldBackend::new(0, 3, 2);
    let wasteful = wasteful_engine.infer(
        InferenceRequest::new("audit runtime kv routing yield", TaskProfile::Coding),
        &mut wasteful_backend,
    );

    assert_eq!(
        efficient.runtime_diagnostics.runtime_kv_segment_yield(),
        Some(1.0)
    );
    assert_eq!(
        wasteful.runtime_diagnostics.runtime_kv_segment_yield(),
        Some(0.0)
    );

    let efficient_decision = runtime_kv_route_decision(&efficient);
    let wasteful_decision = runtime_kv_route_decision(&wasteful);

    assert!(efficient_decision.score > wasteful_decision.score);
    assert!(
        efficient_decision.components.memory_fitness > wasteful_decision.components.memory_fitness
    );
    assert!(efficient_decision.components.trust > wasteful_decision.components.trust);
    assert!(
        efficient_decision.components.reward_history > wasteful_decision.components.reward_history
    );
    assert!(wasteful_decision.components.compute_cost > efficient_decision.components.compute_cost);
    assert!(
        adaptive_route_action_rank(efficient_decision.action)
            >= adaptive_route_action_rank(wasteful_decision.action)
    );
    assert!(wasteful_decision.retained_tokens <= efficient_decision.retained_tokens);
}

#[test]
fn runtime_kv_budget_pressure_downweights_adaptive_route_candidate() {
    let mut unconstrained_engine = NoironEngine::new();
    let mut unconstrained_backend = RuntimeKvRouteYieldBackend::new(1, 0, 0);
    let unconstrained = unconstrained_engine.infer(
        InferenceRequest::new("audit runtime kv budget pressure", TaskProfile::Coding),
        &mut unconstrained_backend,
    );

    let mut budget_limited_engine = NoironEngine::new();
    let mut budget_limited_backend =
        RuntimeKvRouteYieldBackend::new(1, 0, 0).with_budget_limited_skipped(4);
    let budget_limited = budget_limited_engine.infer(
        InferenceRequest::new("audit runtime kv budget pressure", TaskProfile::Coding),
        &mut budget_limited_backend,
    );

    let unconstrained_decision = runtime_kv_route_decision(&unconstrained);
    let budget_limited_decision = runtime_kv_route_decision(&budget_limited);

    assert_eq!(
        budget_limited
            .runtime_diagnostics
            .budget_limited_runtime_kv_imports_skipped,
        4
    );
    assert_eq!(
        budget_limited
            .compute_budget_schedule
            .runtime_kv_budget_pressure,
        0.8
    );
    assert!(
        budget_limited.compute_budget_schedule.threshold_after
            > unconstrained.compute_budget_schedule.threshold_after
    );
    assert!(unconstrained_decision.score > budget_limited_decision.score);
    assert!(
        unconstrained_decision.components.memory_fitness
            > budget_limited_decision.components.memory_fitness
    );
    assert!(unconstrained_decision.components.trust > budget_limited_decision.components.trust);
    assert!(
        budget_limited_decision.components.compute_cost
            > unconstrained_decision.components.compute_cost
    );
    assert!(
        budget_limited
            .compute_budget_schedule
            .summary_line()
            .contains("runtime_kv_budget_pressure=0.800")
    );
    assert!(
        budget_limited
            .compute_budget_schedule
            .notes
            .iter()
            .any(|note| { note == "runtime_kv_budget_pressure=0.800" })
    );
}

fn runtime_kv_route_decision(outcome: &InferenceOutcome) -> &crate::router::AdaptiveRouteDecision {
    outcome
        .adaptive_route_plan
        .decisions
        .iter()
        .find(|decision| decision.source == crate::router::AdaptiveRouteSource::RuntimeKv)
        .expect("runtime kv adaptive route decision")
}

fn adaptive_route_action_rank(action: crate::router::AdaptiveRouteAction) -> u8 {
    match action {
        crate::router::AdaptiveRouteAction::Skip => 0,
        crate::router::AdaptiveRouteAction::Defer => 1,
        crate::router::AdaptiveRouteAction::Compress => 2,
        crate::router::AdaptiveRouteAction::Include => 3,
    }
}

#[test]
fn orchestration_trace_summarizes_full_loop_without_private_payloads() {
    let mut engine = NoironEngine::new();
    engine
        .cache
        .store_or_fuse("seed orchestration memory", vec![0.9, 0.2, 0.1], 0.92);
    let mut backend = OrchestrationTraceBackend;
    let prompt =
        "Rust Noiron orchestration audit with runtime KV genome memory gates private-sentinel-4397";

    let outcome = engine.infer(
        InferenceRequest::new(prompt, TaskProfile::Coding),
        &mut backend,
    );
    let trace = outcome.orchestration_trace();

    for stage in [
        "context",
        "memory_retrieval",
        "routing",
        "model_adapter",
        "reflection_validation",
        "reasoning_genome",
        "memory_admission",
        "evolution_ledger",
        "retention_compaction",
    ] {
        assert!(trace.has_stage(stage), "missing stage {stage}");
    }
    assert_eq!(
        trace.stage("model_adapter").unwrap().status,
        NoironOrchestrationStageStatus::Completed
    );
    assert!(trace.route.adaptive_candidates > 0);
    assert!(trace.route.decision_count_matches);
    assert!(trace.route.token_accounting_matches);
    assert!(trace.route.anchors_retained);
    assert!(trace.kv.used_memories > 0);
    assert_eq!(trace.kv.exported_runtime_kv_blocks, 1);
    assert!(trace.genome.splice_segments > 0);
    assert!(trace.genome.expression_gene_count > 0);
    assert!(trace.gates.memory_admission_read_only_preview);
    assert!(trace.gates.genome_expression_read_only_preview);
    assert!(trace.gates.genome_splice_read_only_preview);
    assert_eq!(trace.gates.durable_memory_ledger_applied, 0);
    assert_eq!(trace.gates.unauthorized_durable_memory_writes, 0);
    assert!(trace.all_writes_gated());
    assert!(trace.summary_line().contains("writes_gated=true"));

    let rendered = format!("{trace:?}");
    assert!(!rendered.contains("private-sentinel-4397"));
    assert!(!rendered.contains(&outcome.answer));
    assert!(!rendered.contains(&outcome.raw_answer));
}

#[test]
fn orchestration_trace_isolates_runtime_failure_with_rollback_record() {
    let mut engine = NoironEngine::new();
    let mut backend = TimeoutErrorBackend;

    let outcome = engine.infer(
        InferenceRequest::new("bounded runtime error audit", TaskProfile::Coding),
        &mut backend,
    );
    let trace = outcome.orchestration_trace();

    let model_stage = trace.stage("model_adapter").unwrap();
    assert_eq!(model_stage.status, NoironOrchestrationStageStatus::Failed);
    assert!(model_stage.rollback_records.iter().any(|record| {
        record.starts_with("runtime_error:")
            && record.contains("timeout=true")
            && record.contains("message_chars=")
    }));
    assert!(trace.has_actionable_rollback_record());
    assert_eq!(trace.gates.durable_memory_ledger_applied, 0);
    assert!(trace.all_writes_gated());
    assert_eq!(
        trace.stage("memory_admission").unwrap().status,
        NoironOrchestrationStageStatus::PreviewOnly
    );

    let rendered = format!("{trace:?}");
    assert!(!rendered.contains("runtime command mistralrs timed out after 1000 ms"));
}

#[test]
fn inference_records_runtime_embedding_source_for_query_and_memory() {
    let mut engine = NoironEngine::new();
    let mut backend = RuntimeEmbeddingBackend;

    let outcome = engine.infer(
        InferenceRequest::new("audit runtime embedding source", TaskProfile::Coding),
        &mut backend,
    );

    assert_eq!(
        outcome.embedding_diagnostics.query.source,
        EmbeddingSource::Runtime
    );
    assert_eq!(outcome.embedding_diagnostics.query.dimensions, 3);
    assert!(outcome.embedding_diagnostics.runtime_embedding_available());
    assert!(!outcome.embedding_diagnostics.fallback_embedding_used());
    assert_eq!(outcome.embedding_diagnostics.fallback_calls, 0);
    assert_eq!(
        outcome.embedding_diagnostics.runtime_calls,
        outcome.embedding_diagnostics.total_calls()
    );
    assert!(outcome.stored_memory_id.is_some());
    assert!(
        engine
            .cache
            .entries()
            .iter()
            .any(|entry| entry.vector.len() == 3)
    );
}

#[test]
fn inference_records_fallback_embedding_source_for_heuristic_backend() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;

    let outcome = engine.infer(
        InferenceRequest::new("audit fallback embedding source", TaskProfile::General),
        &mut backend,
    );

    assert_eq!(
        outcome.embedding_diagnostics.query.source,
        EmbeddingSource::Fallback
    );
    assert_eq!(outcome.embedding_diagnostics.query.dimensions, 64);
    assert!(!outcome.embedding_diagnostics.runtime_embedding_available());
    assert!(outcome.embedding_diagnostics.fallback_embedding_used());
    assert_eq!(outcome.embedding_diagnostics.runtime_calls, 0);
    assert_eq!(
        outcome.embedding_diagnostics.fallback_calls,
        outcome.embedding_diagnostics.total_calls()
    );
}

#[derive(Debug, Clone)]
struct ShortRepairBackend;

impl InferenceBackend for ShortRepairBackend {
    fn generate(&mut self, _context: GenerationContext<'_>) -> InferenceDraft {
        InferenceDraft::new(
            "Rust routes.",
            vec![ReasoningStep::new("draft", "short but grounded", 0.86)],
        )
        .with_exported_kv_blocks(vec![RuntimeKvBlock::new(
            0,
            0,
            0,
            1,
            vec![0.1, 0.2, 0.3],
            vec![0.3, 0.2, 0.1],
        )])
    }
}

#[test]
fn reflection_repair_rechecks_answer_without_admitting_stale_runtime_kv() {
    let mut engine = NoironEngine::new();
    let mut backend = ShortRepairBackend;
    let outcome = engine.infer(
        InferenceRequest::new(
            "Explain Rust Noiron adaptive routing decisions",
            TaskProfile::Coding,
        ),
        &mut backend,
    );

    assert_eq!(outcome.report.revision_passes, 1);
    assert_eq!(outcome.raw_answer, "Rust routes.");
    assert!(outcome.answer.contains("Reflection repair"));
    assert_ne!(outcome.raw_answer, outcome.answer);
    assert!(outcome.stored_memory_id.is_some());
    assert_eq!(outcome.exported_runtime_kv_blocks, 1);
    assert!(outcome.stored_runtime_kv_memory_ids.is_empty());
}

#[test]
fn inference_auto_replays_prior_experience_before_next_run() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;

    let first = engine.infer(
        InferenceRequest::new("build a Rust Noiron replay loop", TaskProfile::Coding),
        &mut backend,
    );
    let second = engine.infer(
        InferenceRequest::new("build a Rust Noiron replay loop", TaskProfile::Coding),
        &mut backend,
    );

    assert!(first.auto_replay_report.is_none());
    let report = second.auto_replay_report.as_ref().unwrap();
    assert!(report.applied >= 1);
    assert_eq!(report.router_updates, report.applied);
    assert_eq!(report.hierarchy_updates, report.applied);
    assert!(report.reinforced >= 1 || report.penalized >= 1);
    assert!(report.memory_reinforcements + report.memory_penalties >= 1);
    assert_eq!(engine.evolution_ledger.replay_runs, 1);
    assert_eq!(engine.evolution_ledger.replay_items, report.applied as u64);
    assert_eq!(
        engine.evolution_ledger.router_threshold_mutations,
        report.router_threshold_mutations as u64
    );
    assert_eq!(
        engine.evolution_ledger.hierarchy_weight_mutations,
        report.hierarchy_weight_mutations as u64
    );
    assert_eq!(
        engine.evolution_ledger.memory_updates(),
        (report.memory_reinforcements + report.memory_penalties) as u64
    );
}
