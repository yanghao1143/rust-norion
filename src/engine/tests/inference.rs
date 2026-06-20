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
