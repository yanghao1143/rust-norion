use super::*;
use crate::ReasoningGenomeStrategy;
use crate::hardware::{DeviceClass, HardwareSnapshot};
use crate::kv_cache::MemoryResidencyState;
use norion_agent::AgentModelRouteProof;

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
    assert!(outcome.runtime_token_metrics.token_count > 0);
    assert_eq!(
        outcome.runtime_diagnostics.model_id.as_deref(),
        Some("rust-norion-heuristic-local")
    );
    assert!(
        (engine.experience.records()[0].process_reward.total - outcome.process_reward.total).abs()
            < 0.0001
    );
    assert!(!outcome.transformer_plan.is_empty());
    assert!(!engine.cache.is_empty());
}

#[test]
fn inference_caps_parallel_fanout_at_dna_confidence_prefix() {
    let mut engine = NoironEngine::new();
    engine.hardware_snapshot =
        HardwareSnapshot::new(DeviceClass::DiscreteGpu, 0.05, 0.05, 0.10, 0.05);
    let genome = &mut engine
        .genome_runtime_state
        .profile_mut(TaskProfile::General)
        .active;
    genome.genes[0].fitness = 0.95;
    genome.genes[1].fitness = 0.80;
    genome.genes[2].fitness = 0.20;

    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new(
            "benchmark DSpark paper throughput and verification scheduling",
            TaskProfile::General,
        ),
        &mut backend,
    );
    let routing_bias = &outcome.reasoning_frame.routing_bias;

    assert_eq!(routing_bias.confidence_prefix_max, 4);
    assert_eq!(routing_bias.confidence_prefix_required, 1);
    assert_eq!(routing_bias.confidence_prefix_selected, 2);
    assert_eq!(routing_bias.confidence_prefix_survival_milli, 760);
    assert!(routing_bias.confidence_prefix_early_stopped);
    assert!(routing_bias.confidence_prefix_evidence_complete);
    assert_eq!(outcome.recursive_schedule.max_parallel_chunks, 2);
}

#[test]
fn inference_squeezes_failed_dna_out_of_the_next_borrowed_prefix() {
    let mut engine = NoironEngine::new();
    engine.hardware_snapshot =
        HardwareSnapshot::new(DeviceClass::DiscreteGpu, 0.05, 0.05, 0.10, 0.05);
    let profile = TaskProfile::General;
    let gene_id = "gene:general:routing";
    let record = engine
        .genome_runtime_state
        .profile_mut(profile)
        .gene_residency
        .records
        .iter_mut()
        .find(|record| record.gene_id == gene_id)
        .unwrap();
    record.residency = MemoryResidencyState::Warm;
    record.opportunities = 12;
    record.failures = 12;

    let mut backend = HeuristicBackend;
    let first = engine.infer(
        InferenceRequest::new("audit a repeatedly failing DNA route", profile),
        &mut backend,
    );
    assert!(
        first
            .pre_reasoning_genome
            .active_gene_ids
            .contains(&gene_id.to_owned())
    );
    assert_eq!(
        first
            .reasoning_frame
            .routing_bias
            .confidence_prefix_selected,
        1
    );
    assert!(
        first
            .reasoning_frame
            .routing_bias
            .confidence_prefix_early_stopped
    );
    assert_eq!(
        engine
            .genome_runtime_state
            .profile(profile)
            .gene_residency
            .records
            .iter()
            .find(|record| record.gene_id == gene_id)
            .unwrap()
            .residency,
        MemoryResidencyState::Cold
    );

    let second = engine.infer(
        InferenceRequest::new("verify the cold DNA route stays dormant", profile),
        &mut backend,
    );
    assert!(
        !second
            .pre_reasoning_genome
            .active_gene_ids
            .contains(&gene_id.to_owned())
    );
    assert!(
        second
            .pre_reasoning_genome_chain
            .express_chain
            .iter()
            .all(|record| record.gene_id != gene_id)
    );
}

#[test]
fn inference_borrows_only_hot_and_warm_genes() {
    let mut engine = NoironEngine::new();
    let cold_gene_id = "gene:coding:routing";
    let record = engine
        .genome_runtime_state
        .profile_mut(TaskProfile::Coding)
        .gene_residency
        .records
        .iter_mut()
        .find(|record| record.gene_id == cold_gene_id)
        .unwrap();
    record.residency = MemoryResidencyState::Cold;
    record.consumed_evidence_digest = "redaction-digest:cold-routing".to_owned();

    let mut backend = HeuristicBackend;
    let outcome = engine.infer(
        InferenceRequest::new("build a bounded Rust routing cache", TaskProfile::Coding),
        &mut backend,
    );

    assert!(
        !outcome
            .pre_reasoning_genome
            .active_gene_ids
            .contains(&cold_gene_id.to_owned())
    );
    assert!(
        outcome
            .pre_reasoning_genome_chain
            .express_chain
            .iter()
            .all(|record| record.gene_id != cold_gene_id)
    );
    assert!(
        outcome
            .reasoning_genome_chain
            .express_chain
            .iter()
            .all(|record| record.gene_id != cold_gene_id)
    );
    assert!(
        outcome
            .pre_reasoning_genome
            .lifecycle_records
            .iter()
            .all(|record| record.gene_id != cold_gene_id)
    );
    assert!(
        !outcome
            .reasoning_frame
            .selected_gene_ids
            .contains(&cold_gene_id.to_owned())
    );
    assert!(
        outcome
            .reasoning_genome
            .lifecycle_records
            .iter()
            .all(|record| record.gene_id != cold_gene_id)
    );
    assert_eq!(outcome.gene_residency.cold, 1);
    assert_eq!(outcome.gene_residency.borrowed_expression_count, 6);
    assert_eq!(
        outcome.pre_reasoning_genome.expression_gene_count,
        outcome.pre_reasoning_genome.lifecycle_record_count()
    );
    assert_eq!(
        outcome.reasoning_genome.expression_gene_count,
        outcome.gene_residency.borrowed_expression_count
    );
    assert_eq!(
        outcome
            .adaptive_route_plan
            .decisions
            .iter()
            .filter(|decision| decision.candidate_id.starts_with("gene:record:"))
            .count(),
        outcome.reasoning_genome.lifecycle_record_count()
    );
    assert_eq!(
        outcome.genome_evolution_preview.residency_revision_before,
        engine
            .genome_runtime_state
            .residency_revision(TaskProfile::Coding)
    );
}

#[test]
fn all_cold_genome_stays_empty_and_passes_trace_and_benchmark_gates() {
    let mut engine = NoironEngine::new();
    let profile = TaskProfile::Coding;
    let cold_gene_ids = engine
        .genome_runtime_state
        .active(profile)
        .genes
        .iter()
        .map(|gene| gene.id.clone())
        .collect::<Vec<_>>();
    let residency = &mut engine
        .genome_runtime_state
        .profile_mut(profile)
        .gene_residency;
    residency.step = 64;
    for record in &mut residency.records {
        record.residency = MemoryResidencyState::Cold;
        record.last_used_step = 0;
        record.consumed_evidence_digest = format!("redaction-digest:cold:{}", record.gene_id);
    }

    let prompt = "audit a fully cold Rust reasoning genome without waking payloads";
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(InferenceRequest::new(prompt, profile), &mut backend);

    assert_eq!(outcome.gene_residency.borrowed_expression_count, 0);
    assert_eq!(outcome.reasoning_genome.expression_gene_count, 0);
    assert!(outcome.reasoning_genome.lifecycle_records.is_empty());
    assert!(outcome.reasoning_genome_chain.express_chain.is_empty());
    assert!(
        outcome
            .reasoning_frame
            .selected_gene_ids
            .iter()
            .all(|gene_id| !cold_gene_ids.contains(gene_id))
    );
    assert!(
        outcome
            .adaptive_route_plan
            .decisions
            .iter()
            .all(|decision| !decision.candidate_id.starts_with("gene:record:"))
    );

    let trace = crate::trace::trace_json_line(prompt, profile, 1, &outcome);
    let trace_failures = crate::trace::evaluate_trace_schema_line(&trace);
    assert!(trace_failures.is_empty(), "{trace_failures:?}");

    let case = crate::benchmark::BenchmarkCase::new("all-cold-genome", profile, prompt);
    let mut summary = crate::benchmark::BenchmarkSummary::new();
    summary.record(&case, 1, &outcome);
    assert_eq!(summary.reasoning_genome_expression_cases(), 0);
    assert_eq!(summary.total_reasoning_genome_failures(), 0);
}

#[test]
fn gene_scoped_new_evidence_previews_then_applies_cold_readmission() {
    let mut engine = NoironEngine::new();
    let profile = TaskProfile::Coding;
    let cold_gene_id = "gene:coding:routing";
    let residency = &mut engine
        .genome_runtime_state
        .profile_mut(profile)
        .gene_residency;
    residency.step = 64;
    let record = residency
        .records
        .iter_mut()
        .find(|record| record.gene_id == cold_gene_id)
        .unwrap();
    record.residency = MemoryResidencyState::Cold;
    record.last_used_step = 0;
    record.consumed_evidence_digest = "redaction-digest:older-routing-evidence".to_owned();

    let mut backend = HeuristicBackend;
    let preview = engine.infer(
        InferenceRequest::new("build a bounded Rust routing cache", profile),
        &mut backend,
    );
    assert!(preview.genome_evolution_preview.plans.iter().any(|plan| {
        plan.id == "mutation:gene:coding:routing:cold-readmission"
            && plan.validation_status == crate::reasoning_genome::GeneValidationStatus::Pending
    }));
    assert!(!preview.dna_apply_receipt.applied);
    assert_eq!(
        engine
            .genome_runtime_state
            .profile(profile)
            .gene_residency
            .records
            .iter()
            .find(|record| record.gene_id == cold_gene_id)
            .unwrap()
            .residency,
        MemoryResidencyState::Cold
    );

    let applied = engine.infer(
        InferenceRequest::new("build a bounded Rust routing cache", profile)
            .with_genome_evolution_authorization(GenomeEvolutionAuthorization::apply(
                crate::reasoning_genome::DnaEvolutionValidationEvidence::passing(),
                "operator:cold-readmission",
            )),
        &mut backend,
    );
    assert!(
        applied.dna_apply_receipt.applied,
        "{} controller={:?} writer={:?} apply={:?} plans={:?}",
        applied.dna_apply_receipt.reason,
        applied.dna_evolution_controller,
        applied.dna_writer_gate,
        applied.dna_apply_plan,
        applied.genome_evolution_preview.plans
    );
    assert_eq!(
        engine
            .genome_runtime_state
            .profile(profile)
            .gene_residency
            .records
            .iter()
            .find(|record| record.gene_id == cold_gene_id)
            .unwrap()
            .residency,
        MemoryResidencyState::Warm
    );

    let residency = &mut engine
        .genome_runtime_state
        .profile_mut(profile)
        .gene_residency;
    residency.step = residency.step.saturating_add(64);
    let record = residency
        .records
        .iter_mut()
        .find(|record| record.gene_id == cold_gene_id)
        .unwrap();
    assert!(
        record
            .consumed_evidence_digest
            .starts_with("redaction-digest:")
    );
    record.residency = MemoryResidencyState::Cold;
    record.last_used_step = 0;

    let duplicate = engine.infer(
        InferenceRequest::new("build a bounded Rust routing cache", profile),
        &mut backend,
    );
    assert!(
        duplicate
            .genome_evolution_preview
            .plans
            .iter()
            .all(|plan| { plan.id != "mutation:gene:coding:routing:cold-readmission" })
    );
    let newer = engine.infer(
        InferenceRequest::new("optimize routing thresholds for a bounded cache", profile),
        &mut backend,
    );
    assert!(
        newer
            .genome_evolution_preview
            .plans
            .iter()
            .any(|plan| { plan.id == "mutation:gene:coding:routing:cold-readmission" })
    );
}

#[test]
fn cancelled_inference_discards_post_generation_state() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let mut polls = 0usize;
    let outcome = engine.infer_cancelable(
        InferenceRequest::new("build a Rust Noiron routing cache", TaskProfile::Coding),
        &mut backend,
        &mut || {
            polls = polls.saturating_add(1);
            polls >= 2
        },
    );

    assert!(outcome.raw_answer.contains("cancelled"));
    assert_eq!(engine.router.observations(), 0);
    assert!(engine.cache.is_empty());
    assert!(engine.experience.is_empty());
}

#[test]
fn inference_selects_independent_task_strategy_genomes_before_generation() {
    let cases = [
        (
            "Explain bounded reasoning clearly",
            TaskProfile::General,
            ReasoningGenomeStrategy::English,
        ),
        (
            "请用中文总结证据边界",
            TaskProfile::Writing,
            ReasoningGenomeStrategy::Chinese,
        ),
        (
            "Build a Rust CLI tool that explains borrowing and lifetimes",
            TaskProfile::Coding,
            ReasoningGenomeStrategy::RustCoding,
        ),
        (
            "Summarize this long document about an agent tool workflow with stable anchors",
            TaskProfile::LongDocument,
            ReasoningGenomeStrategy::LongContext,
        ),
        (
            "build a local Rust cli tool for state inspection",
            TaskProfile::General,
            ReasoningGenomeStrategy::LocalTool,
        ),
    ];

    for (prompt, profile, expected) in cases {
        let mut engine = NoironEngine::new();
        let mut backend = HeuristicBackend;
        let outcome = engine.infer(InferenceRequest::new(prompt, profile), &mut backend);

        assert_eq!(outcome.genome_strategy, expected);
        assert_eq!(outcome.strategy_genome.active_gene_count(), 3);
        assert!(
            outcome
                .strategy_genome
                .active_gene_ids
                .iter()
                .all(|gene_id| gene_id.contains(expected.as_str()))
        );
        assert!(
            outcome
                .strategy_genome
                .active_gene_ids
                .iter()
                .all(|gene_id| outcome.reasoning_frame.selected_gene_ids.contains(gene_id))
        );
        assert!(outcome.reasoning_frame_valid);
    }
}

#[test]
fn inference_enables_agent_team_only_with_layer_b_route_proof() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let request = InferenceRequest::new(
        "agent team coordinate Rust implementation lanes",
        TaskProfile::Coding,
    )
    .with_agent_team_route_proof(
        AgentModelRouteProof::new(
            "model-registry-v1",
            "qwen-local-fast",
            "deterministic-inference-backend",
            "default-model-pool",
        )
        .with_selected_role("planner"),
    );

    let outcome = engine.infer(request, &mut backend);

    assert!(outcome.agent_team_plan.enabled);
    assert_eq!(
        outcome
            .agent_team_plan
            .layer_b_route_proof
            .as_ref()
            .map(|proof| proof.model_profile_id.as_str()),
        Some("qwen-local-fast")
    );
    assert!(
        outcome
            .agent_team_plan
            .notes
            .iter()
            .any(|note| note.starts_with("agent_team_layer_b_route_proof=ready "))
    );
    assert!(
        outcome
            .process_reward
            .notes
            .iter()
            .any(|note| { note == "agent_team:layer_b_route_proof=ready" })
    );
}

#[test]
fn inference_request_derives_agent_team_route_proof_from_route_plan_json() {
    let route_plan_json = r#"{
        "ok": true,
        "read_only": true,
        "launches_process": false,
        "sends_prompt": false,
        "route_allowed": true,
        "reason": "ready",
        "selected_role": "review",
        "agent_model_route_source": {
            "route_allowed": true,
            "proof_ready": true,
            "selected_role": "review",
            "model_registry_id": "registry.review",
            "model_profile_id": "profile.review",
            "inference_backend_id": "backend.review",
            "model_pool_id": "pool.main"
        }
    }"#;
    let request = InferenceRequest::new("agent team review Rust route plan", TaskProfile::Coding)
        .try_with_agent_team_route_plan_json(route_plan_json)
        .unwrap();

    assert_eq!(
        request
            .agent_team_route_proof
            .as_ref()
            .and_then(|proof| proof.selected_role.as_deref()),
        Some("review")
    );

    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let outcome = engine.infer(request, &mut backend);

    assert!(outcome.agent_team_plan.enabled);
    assert_eq!(
        outcome
            .agent_team_plan
            .layer_b_route_proof
            .as_ref()
            .and_then(|proof| proof.selected_role.as_deref()),
        Some("review")
    );
    assert!(
        outcome
            .process_reward
            .notes
            .iter()
            .any(|note| note == "agent_team:layer_b_route_proof=ready")
    );
}

#[test]
fn route_plan_json_rejects_missing_agent_route_source_proof() {
    let error = InferenceRequest::new("agent team review Rust route plan", TaskProfile::Coding)
        .try_with_agent_team_route_plan_json(
            r#"{
                    "ok": true,
                    "read_only": true,
                    "launches_process": false,
                    "sends_prompt": false,
                    "route_allowed": true,
                    "reason": "ready",
                    "selected_role": "review"
                }"#,
        )
        .unwrap_err();

    assert!(error.contains("missing agent_model_route_source"));
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
    store_local_memory(
        &mut engine.cache,
        "seed orchestration memory",
        vec![0.9, 0.2, 0.1],
        0.92,
    );
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
        "live_feedback_loop",
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
    assert_eq!(trace.schema_version, 2);
    assert!(trace.control_expression.ready());
    assert_eq!(
        trace.control_expression.active_control_knobs,
        vec![
            "routing".to_owned(),
            "context_anchor".to_owned(),
            "suppression".to_owned(),
            "checkpoint".to_owned(),
            "memory_maintenance".to_owned(),
        ]
    );
    assert_eq!(
        trace.control_expression.policy_version,
        "control_expression_gate_v1"
    );
    assert_eq!(
        trace.control_expression.decision_reason,
        "no_weight_runtime_control_preview"
    );
    assert_eq!(trace.control_expression.write_allowed, false);
    assert_eq!(trace.control_expression.applied, false);
    assert!(trace.control_expression.operator_approval_required);
    assert_eq!(
        trace.control_expression.control_expression_profile_selected,
        1
    );
    assert_eq!(trace.control_expression.context_anchor_promoted, 1);
    assert_eq!(trace.control_expression.suppression_gate_triggered, 1);
    assert_eq!(trace.control_expression.memory_refresh_candidate, 1);
    assert_eq!(
        trace
            .control_expression
            .control_expression_preview_admission,
        1
    );
    let mut missing_counter = trace.control_expression.clone();
    missing_counter.memory_refresh_candidate = 0;
    assert!(!missing_counter.ready());
    let live_feedback_stage = trace.stage("live_feedback_loop").unwrap();
    assert_eq!(
        live_feedback_stage.status,
        NoironOrchestrationStageStatus::Completed
    );
    assert!(
        live_feedback_stage
            .evidence
            .iter()
            .any(|item| item.starts_with("hierarchy_weight_delta="))
    );
    assert!(
        live_feedback_stage
            .evidence
            .iter()
            .any(|item| item.starts_with("memory_feedback=updates:"))
    );
    assert!(trace.summary_line().contains("writes_gated=true"));
    assert!(trace.summary_line().contains("live_feedback_closed=true"));
    assert!(
        trace
            .summary_line()
            .contains("control_expression_ready=true")
    );
    assert!(trace.control_expression.summary_line().contains(
        "active_control_knobs=routing|context_anchor|suppression|checkpoint|memory_maintenance"
    ));
    assert!(
        trace
            .control_expression
            .summary_line()
            .contains("write_allowed=false")
    );

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
    assert_eq!(trace.control_expression.checkpoint_rejected, 1);
    assert_eq!(trace.control_expression.write_allowed, false);
    assert_eq!(trace.control_expression.applied, false);
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

struct RustValidationRetryBackend {
    calls: usize,
    always_invalid: bool,
}

impl InferenceBackend for RustValidationRetryBackend {
    fn generate(&mut self, _context: GenerationContext<'_>) -> InferenceDraft {
        self.calls += 1;
        let answer = if self.always_invalid || self.calls == 1 {
            "修复后的代码：\nfn first(items: Vec<String>) -> &str {\n    &items[0]\n}\n原因：原代码正确，无需修改。"
        } else {
            "修复后的代码：\n```rust\nfn first(items: &[String]) -> &str {\n    &items[0]\n}\n```\n原因：借用切片后返回值的生命周期由输入借用约束。"
        };
        InferenceDraft::new(
            answer,
            vec![ReasoningStep::new("draft", "Rust correction", 0.94)],
        )
    }
}

#[test]
fn coding_inference_retries_after_rustc_failure_before_memory_admission() {
    let mut engine = NoironEngine::new();
    let mut backend = RustValidationRetryBackend {
        calls: 0,
        always_invalid: false,
    };
    let outcome = engine.infer(
        InferenceRequest::new(
            "修复 Rust 函数并避免不必要 clone: fn first(items: Vec<String>) -> &str",
            TaskProfile::Coding,
        ),
        &mut backend,
    );

    assert_eq!(backend.calls, 2);
    assert!(outcome.answer.contains("items: &[String]"));
    assert!(outcome.stored_memory_id.is_some());
    assert!(
        !outcome
            .report
            .issues
            .iter()
            .any(|issue| issue.code.starts_with("rust_validation_"))
    );
}

#[test]
fn coding_stream_blocks_unvalidated_answer_from_memory() {
    let mut engine = NoironEngine::new();
    let mut backend = RustValidationRetryBackend {
        calls: 0,
        always_invalid: false,
    };
    let outcome = engine.infer_stream(
        InferenceRequest::new(
            "修复 Rust 函数并避免不必要 clone: fn first(items: Vec<String>) -> &str",
            TaskProfile::Coding,
        ),
        &mut backend,
        &mut |_token| {},
    );

    assert_eq!(backend.calls, 1);
    assert!(outcome.stored_memory_id.is_none());
    assert_eq!(outcome.memory_feedback.reinforced, 0);
    assert!(outcome.report.critical_issue_count() > 0);
    assert!(
        outcome
            .raw_answer
            .contains("Validation failed: rust_validation_failed")
    );
}

#[test]
fn coding_inference_blocks_uncompilable_answer_and_memory_reinforcement() {
    let mut engine = NoironEngine::new();
    let memory_id = store_local_memory(
        &mut engine.cache,
        "Rust first function lifetime evidence",
        vec![1.0; 32],
        0.8,
    );
    let strength_before = memory_strength(&engine, memory_id);
    let mut backend = RustValidationRetryBackend {
        calls: 0,
        always_invalid: true,
    };
    let outcome = engine.infer(
        InferenceRequest::new("修复 Rust first 函数的返回生命周期", TaskProfile::Coding),
        &mut backend,
    );

    assert_eq!(backend.calls, 2);
    assert!(outcome.stored_memory_id.is_none());
    assert!(outcome.report.critical_issue_count() > 0);
    assert!(
        outcome
            .answer
            .contains("Validation failed: rust_validation_failed")
    );
    assert!(
        outcome
            .raw_answer
            .contains("Validation failed: rust_validation_failed")
    );
    assert_eq!(outcome.memory_feedback.reinforced, 0);
    if outcome
        .used_memories
        .iter()
        .any(|memory| memory.id == memory_id)
    {
        assert!(memory_strength(&engine, memory_id) <= strength_before);
    }
}

struct HtmlGenerationBackend;

impl InferenceBackend for HtmlGenerationBackend {
    fn generate(&mut self, _context: GenerationContext<'_>) -> InferenceDraft {
        InferenceDraft::new(
            "<!doctype html><html><body><button id=\"move\">落子</button><script>const board=[];function move(){board.push(1);}</script></body></html>",
            vec![ReasoningStep::new("draft", "complete HTML source", 0.94)],
        )
    }
}

#[test]
fn generated_html_without_behavior_evidence_is_not_treated_as_achieved() {
    let mut engine = NoironEngine::new();
    let outcome = engine.infer(
        InferenceRequest::new("生成一个完整的单文件 HTML 五子棋", TaskProfile::Coding),
        &mut HtmlGenerationBackend,
    );

    assert!(outcome.raw_answer.ends_with("</html>"));
    assert!(outcome.stored_memory_id.is_none());
    assert_eq!(outcome.memory_feedback.reinforced, 0);
    assert!(
        outcome
            .report
            .issues
            .iter()
            .any(|issue| issue.code == "generated_code_behavior_unverified")
    );
    assert!(!outcome.genome_evolution_preview.is_eligible());
    assert!(outcome.genome_evolution_preview.critical_reflection_issues > 0);
}

struct MemoryRecallBackend {
    calls: usize,
}

impl InferenceBackend for MemoryRecallBackend {
    fn generate(&mut self, _context: GenerationContext<'_>) -> InferenceDraft {
        self.calls += 1;
        let answer = if self.calls == 1 {
            "Polaris-17 的发布门槛是延迟低于 120ms 且回归测试全绿；该规则应作为会话事实持久化。"
        } else {
            "Polaris-17 的发布门槛是完成 17 个核心模块并达到 99.9% 测试覆盖率。"
        };
        InferenceDraft::new(
            answer,
            vec![ReasoningStep::new("draft", "memory fact recall", 0.94)],
        )
    }
}

#[test]
fn fact_recall_contradiction_is_not_stored_or_reinforced() {
    let mut engine = NoironEngine::new();
    let mut backend = MemoryRecallBackend { calls: 0 };
    let first = engine.infer(
        InferenceRequest::new(
            "记住规则：项目代号 Polaris-17 的发布门槛是延迟低于 120ms 且回归测试全绿。然后复述发布门槛并说明你会如何持久化这条经验。",
            TaskProfile::General,
        ),
        &mut backend,
    );
    assert!(first.stored_memory_id.is_some());

    let second = engine.infer(
        InferenceRequest::new(
            "项目 Polaris-17 的发布门槛是什么？只回答门槛。",
            TaskProfile::General,
        ),
        &mut backend,
    );

    assert!(!second.used_memories.is_empty());
    assert!(second.stored_memory_id.is_none());
    assert_eq!(second.memory_feedback.reinforced, 0);
    assert!(second.memory_feedback.penalized > 0);
    assert_eq!(
        second.raw_answer,
        "延迟低于 120ms 且回归测试全绿",
        "{:?}",
        second
            .used_memories
            .iter()
            .map(|memory| memory.key.as_str())
            .collect::<Vec<_>>()
    );
    assert!(second.report.issues.iter().any(|issue| {
        issue.code == "memory_grounding_contradiction"
            && issue.severity == ReflectionSeverity::Critical
    }));
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
