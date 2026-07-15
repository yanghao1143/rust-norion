use super::*;
use crate::reasoning_genome::GeneScissorsIntent;

#[test]
fn drift_guard_blocks_contradictory_runtime_kv_memory() {
    struct ContradictingBackend;

    impl InferenceBackend for ContradictingBackend {
        fn generate(&mut self, _context: GenerationContext<'_>) -> InferenceDraft {
            InferenceDraft::new(
                    "Rust Noiron drift guard is certain about this answer, but it is also uncertain in the same claim, so the self-evolving memory path should treat it as unsafe.",
                    vec![ReasoningStep::new("runtime", "contradictory draft", 0.92)],
                )
                .with_exported_kv_blocks(vec![RuntimeKvBlock::new(
                    1,
                    0,
                    0,
                    2,
                    vec![0.2, 0.4],
                    vec![0.3, 0.5],
                )])
        }
    }

    let mut engine = NoironEngine::new();
    let mut backend = ContradictingBackend;

    let outcome = engine.infer(
        InferenceRequest::new("Rust Noiron drift guard", TaskProfile::Coding),
        &mut backend,
    );

    assert_eq!(outcome.exported_runtime_kv_blocks, 1);
    assert_eq!(
        outcome.drift_report.severity,
        crate::drift::DriftSeverity::Block
    );
    assert!(!outcome.report.store_as_memory);
    assert!(outcome.report.critical_issue_count() > 0);
    assert!(
        outcome
            .report
            .issue_codes()
            .iter()
            .any(|code| code == "conflicting_certainty_markers")
    );
    assert!(outcome.stored_memory_id.is_none());
    assert!(outcome.stored_runtime_kv_memory_ids.is_empty());
}

#[test]
fn drift_guard_penalizes_used_memory_by_reflection_severity() {
    struct ContradictingBackend;

    impl InferenceBackend for ContradictingBackend {
        fn generate(&mut self, _context: GenerationContext<'_>) -> InferenceDraft {
            InferenceDraft::new(
                "Rust Noiron cached answer is certain and guaranteed, but maybe unknown.",
                vec![ReasoningStep::new(
                    "runtime",
                    "contradictory cached path",
                    0.90,
                )],
            )
        }
    }

    let prompt = "Rust Noiron cached answer";
    let mut cache = KvFusionCache::new();
    let memory_id = store_local_memory(
        &mut cache,
        prompt,
        TextEmbedder::default().embed(prompt),
        0.82,
    );
    let mut engine = NoironEngine::with_cache(cache);
    let before_strength = memory_strength(&engine, memory_id);
    let mut backend = ContradictingBackend;

    let outcome = engine.infer(
        InferenceRequest::new(prompt, TaskProfile::Coding),
        &mut backend,
    );

    assert_eq!(
        outcome.drift_report.severity,
        crate::drift::DriftSeverity::Block
    );
    assert_eq!(outcome.used_memories.len(), 1);
    assert!(outcome.drift_report.penalize_used_memory);
    assert_eq!(outcome.memory_feedback.reinforced, 0);
    assert_eq!(outcome.memory_feedback.penalized, 1);
    assert!(outcome.memory_feedback.penalty_amount > 0.10);
    assert_eq!(outcome.memory_feedback.total_updates(), 1);
    assert_eq!(outcome.memory_feedback.applied_updates(), 1);
    assert_eq!(outcome.memory_feedback.missing_updates(), 0);
    assert_eq!(outcome.memory_feedback.removed_updates(), 0);
    assert_eq!(outcome.memory_feedback.updates.len(), 1);
    assert_eq!(outcome.memory_feedback.updates[0].id, memory_id);
    assert!(outcome.memory_feedback.updates[0].strength_delta < 0.0);
    assert!(outcome.memory_feedback.strength_delta() > 0.10);
    assert!(
        engine.experience.records()[0]
            .process_reward
            .notes
            .iter()
            .any(|note| {
                note.starts_with("memory_feedback:")
                    && note.contains("penalized=1")
                    && note.contains("applied=1")
                    && note.contains("strength_delta=")
            })
    );
    assert!(outcome.report.critical_issue_count() > 0);
    assert!(memory_strength(&engine, memory_id) < before_strength - 0.10);
}

#[test]
fn process_reward_penalty_updates_adaptive_state_in_same_inference() {
    struct PenalizedBackend;

    impl InferenceBackend for PenalizedBackend {
        fn generate(&mut self, _context: GenerationContext<'_>) -> InferenceDraft {
            let mut token = DraftToken::new("maybe");
            token.entropy = Some(3.2);
            token.logprob = Some(-3.6);
            InferenceDraft::new(
                    "Rust Noiron Python tool result is certain and guaranteed, but maybe unknown; use a Python helper script even though the Rust-only control loop must reject it.",
                    vec![ReasoningStep::new(
                        "runtime",
                        "conflicting non-rust tool request",
                        0.62,
                    )],
                )
                .with_tokens(vec![token])
        }
    }

    let mut engine = NoironEngine::new();
    let threshold_before = engine.router.threshold_for(TaskProfile::Coding);
    let hierarchy_before = engine
        .hierarchy
        .state()
        .profile_weights
        .get(TaskProfile::Coding);
    let mut backend = PenalizedBackend;

    let outcome = engine.infer(
        InferenceRequest::new(
            "build a python script tool for Rust Noiron routing",
            TaskProfile::Coding,
        ),
        &mut backend,
    );

    assert_eq!(outcome.process_reward.action, RewardAction::Penalize);
    assert!(!outcome.drift_report.rollback_adaptive);
    assert!(outcome.router_threshold_after < threshold_before);
    assert!(outcome.live_evolution.router_threshold_delta > 0.0);
    assert!(outcome.live_evolution.hierarchy_weight_delta > 0.0);
    assert_ne!(
        engine
            .hierarchy
            .state()
            .profile_weights
            .get(TaskProfile::Coding),
        hierarchy_before
    );
    assert_eq!(outcome.hierarchy, outcome.hardware_plan.hierarchy);
    assert_eq!(
        engine.experience.records()[0].hierarchy,
        outcome.hardware_plan.hierarchy
    );
    assert_ne!(
        engine
            .hierarchy
            .state()
            .profile_weights
            .get(TaskProfile::Coding),
        outcome.hierarchy
    );
    assert_eq!(outcome.live_evolution.online_reward_feedbacks, 1);
    assert_eq!(outcome.live_evolution.online_reward_reinforcements, 0);
    assert_eq!(outcome.live_evolution.online_reward_penalties, 1);
    assert!(outcome.live_evolution.online_reward_strength > 0.0);
    assert_eq!(
        outcome.live_evolution.online_reward_reinforcement_strength,
        0.0
    );
    assert!(outcome.live_evolution.online_reward_penalty_strength > 0.0);
    assert_eq!(outcome.evolution_ledger.live_online_reward_feedbacks, 1);
    assert_eq!(
        outcome.evolution_ledger.live_online_reward_reinforcements,
        0
    );
    assert_eq!(outcome.evolution_ledger.live_online_reward_penalties, 1);
    assert!(outcome.evolution_ledger.live_online_reward_strength > 0.0);
    assert_eq!(
        outcome
            .evolution_ledger
            .live_online_reward_reinforcement_strength,
        0.0
    );
    assert!(outcome.evolution_ledger.live_online_reward_penalty_strength > 0.0);
    assert_eq!(
        engine.router.observations(),
        outcome.stream_reports.len() as u64 + 2
    );
    assert!(
        outcome
            .process_reward
            .notes
            .iter()
            .any(|note| { note.starts_with("online_reward_feedback:action=penalize") })
    );
    assert!(outcome.reasoning_genome.repair_payload_count() >= 1);
    assert!(
        outcome
            .reasoning_genome
            .mutation_plans
            .iter()
            .any(|plan| plan.target_gene_id == "gene:coding:reflection"
                && plan.intent == GeneScissorsIntent::Relabel
                && plan.has_repair_payload())
    );
    assert!(
        engine.experience.records()[0]
            .process_reward
            .notes
            .iter()
            .any(|note| note.starts_with("online_reward_feedback:action=penalize"))
    );
    assert!(
        (engine.experience.records()[0]
            .live_evolution
            .online_reward_strength
            - outcome.live_evolution.online_reward_strength)
            .abs()
            < 0.0001
    );
    assert!(
        (engine.experience.records()[0]
            .live_evolution
            .online_reward_penalty_strength
            - outcome.live_evolution.online_reward_penalty_strength)
            .abs()
            < 0.0001
    );
    assert!(
        (engine.experience.records()[0]
            .live_evolution
            .router_threshold_delta
            - outcome.live_evolution.router_threshold_delta)
            .abs()
            < 0.0001
    );
}

#[test]
fn drift_guard_rolls_back_adaptive_state_for_bad_draft() {
    struct BadBackend;

    impl InferenceBackend for BadBackend {
        fn generate(&mut self, _context: GenerationContext<'_>) -> InferenceDraft {
            InferenceDraft::new("", vec![ReasoningStep::new("runtime", "empty", 0.0)])
        }
    }

    let mut engine = NoironEngine::new();
    let threshold_before = engine.router.threshold();
    let hierarchy_before = engine.hierarchy.current();
    let mut backend = BadBackend;

    let outcome = engine.infer(
        InferenceRequest::new("Rust Noiron rollback bad draft", TaskProfile::Coding),
        &mut backend,
    );

    assert_eq!(
        outcome.drift_report.severity,
        crate::drift::DriftSeverity::Rollback
    );
    assert!((outcome.router_threshold_after - threshold_before).abs() < 0.0001);
    assert!((engine.router.threshold() - threshold_before).abs() < 0.0001);
    assert!((engine.hierarchy.current().local - hierarchy_before.local).abs() < 0.0001);
    assert_eq!(engine.evolution_ledger.drift_rollbacks, 1);
    assert_eq!(outcome.evolution_ledger.drift_rollbacks, 1);
    assert!(outcome.evolution_ledger.rollback_router_threshold_delta > 0.0);
    assert!(outcome.evolution_ledger.rollback_hierarchy_weight_delta > 0.0);
    assert!(outcome.stored_memory_id.is_none());
    assert_eq!(outcome.live_evolution.router_threshold_delta, 0.0);
    assert_eq!(outcome.live_evolution.hierarchy_weight_delta, 0.0);
    assert_eq!(outcome.live_evolution.online_reward_feedbacks, 0);
    assert_eq!(outcome.evolution_ledger.live_online_reward_feedbacks, 0);
    assert!(outcome.reasoning_genome.regeneration_payload_count() >= 1);
    assert!(
        !outcome
            .reasoning_genome
            .active_gene_ids
            .contains(&"gene:coding:safety".to_owned())
    );
    assert!(
        !outcome
            .process_reward
            .notes
            .iter()
            .any(|note| note.starts_with("online_reward_feedback:"))
    );
}

#[test]
fn process_reward_reinforcement_feedback_scales_with_reward_strength() {
    let base = feedback_base_metrics();
    let reflection = clean_feedback_reflection(0.80);
    let drift_report = stable_feedback_drift_report();
    let near_threshold = feedback_reward_report(0.73, RewardAction::Reinforce);
    let strong = feedback_reward_report(0.98, RewardAction::Reinforce);

    let near_metrics =
        process_reward_feedback_metrics(&near_threshold, base, &reflection, &drift_report).unwrap();
    let strong_metrics =
        process_reward_feedback_metrics(&strong, base, &reflection, &drift_report).unwrap();

    assert!(
        process_reward_feedback_strength(&strong)
            > process_reward_feedback_strength(&near_threshold)
    );
    assert!(strong_metrics.perplexity < near_metrics.perplexity);
    assert!(strong_metrics.semantic_consistency > near_metrics.semantic_consistency);
    assert!(strong_metrics.quality_score() > near_metrics.quality_score());
    assert!(process_reward_feedback_note(&strong, strong_metrics).contains("strength="));
}

#[test]
fn process_reward_penalty_feedback_scales_with_reward_strength() {
    let base = feedback_base_metrics();
    let reflection = clean_feedback_reflection(0.45);
    let drift_report = stable_feedback_drift_report();
    let near_threshold = feedback_reward_report(0.41, RewardAction::Penalize);
    let strong = feedback_reward_report(0.05, RewardAction::Penalize);

    let near_metrics =
        process_reward_feedback_metrics(&near_threshold, base, &reflection, &drift_report).unwrap();
    let strong_metrics =
        process_reward_feedback_metrics(&strong, base, &reflection, &drift_report).unwrap();

    assert!(
        process_reward_feedback_strength(&strong)
            > process_reward_feedback_strength(&near_threshold)
    );
    assert!(strong_metrics.perplexity > near_metrics.perplexity);
    assert!(strong_metrics.semantic_consistency < near_metrics.semantic_consistency);
    assert!(strong_metrics.contradiction_count >= near_metrics.contradiction_count);
    assert!(strong_metrics.quality_score() < near_metrics.quality_score());
    assert!(process_reward_feedback_note(&strong, strong_metrics).contains("strength="));
}

#[test]
fn drift_guard_strongly_penalizes_used_memory_on_rollback() {
    struct BadBackend;

    impl InferenceBackend for BadBackend {
        fn generate(&mut self, _context: GenerationContext<'_>) -> InferenceDraft {
            InferenceDraft::new("", vec![ReasoningStep::new("runtime", "empty", 0.0)])
        }
    }

    let prompt = "Rust Noiron rollback cached memory";
    let mut cache = KvFusionCache::new();
    let memory_id = store_local_memory(
        &mut cache,
        prompt,
        TextEmbedder::default().embed(prompt),
        0.82,
    );
    let mut engine = NoironEngine::with_cache(cache);
    let before_strength = memory_strength(&engine, memory_id);
    let mut backend = BadBackend;

    let outcome = engine.infer(
        InferenceRequest::new(prompt, TaskProfile::Coding),
        &mut backend,
    );

    assert_eq!(
        outcome.drift_report.severity,
        crate::drift::DriftSeverity::Rollback
    );
    assert_eq!(outcome.used_memories.len(), 1);
    assert!(outcome.drift_report.penalize_used_memory);
    assert_eq!(outcome.memory_feedback.reinforced, 0);
    assert_eq!(outcome.memory_feedback.penalized, 1);
    assert!(outcome.memory_feedback.penalty_amount > 0.18);
    assert_eq!(outcome.memory_feedback.total_updates(), 1);
    assert!(memory_strength(&engine, memory_id) < before_strength - 0.18);
}
