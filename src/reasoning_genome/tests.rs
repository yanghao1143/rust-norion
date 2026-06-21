use super::*;
use crate::hierarchy::TaskProfile;

#[test]
fn default_genome_expresses_read_only_profile_genes() {
    let genome = ReasoningGenome::default_for_profile(TaskProfile::Coding);
    let expression = genome.express(GenomeExpressionInput {
        profile: TaskProfile::Coding,
        quality: 0.92,
        process_reward: 0.88,
        contradiction_count: 0,
        critical_reflection_issue_count: 0,
        revision_action_count: 0,
        used_memories: 2,
        memory_feedback_updates: 1,
        route_attention_fraction: 0.42,
        agent_team_collision_free: true,
        toolsmith_gate_passed: true,
        drift_memory_write_allowed: true,
        drift_rollback: false,
        runtime_kv_hold: false,
    });

    assert_eq!(expression.expression_gene_count, 7);
    assert_eq!(expression.active_gene_count(), 7);
    assert_eq!(expression.aged_gene_count(), 0);
    assert_eq!(expression.malignant_gene_count(), 0);
    assert!(expression.is_read_only_preview());
    assert_eq!(expression.scissors_proposal_count(), 0);
}

#[test]
fn aging_gene_gets_relabel_plan_without_writes() {
    let genome = ReasoningGenome::new(
        "genome:test:v1",
        TaskProfile::General,
        "genome:test:stable",
        vec![
            ReasoningGene::new(
                "gene:test:retrieval",
                ReasoningGeneKind::Retrieval,
                "",
                "retrieve useful memory",
            )
            .with_health(12, 0.72, 0.05),
        ],
    );

    let expression = genome.express(GenomeExpressionInput {
        profile: TaskProfile::General,
        quality: 0.80,
        process_reward: 0.72,
        contradiction_count: 0,
        critical_reflection_issue_count: 0,
        revision_action_count: 0,
        used_memories: 0,
        memory_feedback_updates: 0,
        route_attention_fraction: 0.20,
        agent_team_collision_free: true,
        toolsmith_gate_passed: true,
        drift_memory_write_allowed: true,
        drift_rollback: false,
        runtime_kv_hold: false,
    });

    assert_eq!(expression.aged_gene_count(), 1);
    assert_eq!(expression.relabel_candidate_count(), 1);
    assert_eq!(expression.scissors_proposal_count(), 1);
    assert_eq!(
        expression.mutation_plans[0].intent,
        GeneScissorsIntent::Relabel
    );
    assert!(expression.is_read_only_preview());
}

#[test]
fn malignant_gene_is_quarantined_and_regenerated_from_stable_anchor() {
    let genome = ReasoningGenome::new(
        "genome:test:v1",
        TaskProfile::Coding,
        "genome:test:stable",
        vec![
            ReasoningGene::new(
                "gene:test:safety",
                ReasoningGeneKind::Safety,
                "unsafe memory admission",
                "this gene drifted",
            )
            .with_health(2, 0.12, 0.91),
        ],
    );

    let expression = genome.express(GenomeExpressionInput {
        profile: TaskProfile::Coding,
        quality: 0.30,
        process_reward: 0.20,
        contradiction_count: 2,
        critical_reflection_issue_count: 1,
        revision_action_count: 1,
        used_memories: 1,
        memory_feedback_updates: 1,
        route_attention_fraction: 0.80,
        agent_team_collision_free: true,
        toolsmith_gate_passed: true,
        drift_memory_write_allowed: false,
        drift_rollback: true,
        runtime_kv_hold: true,
    });

    let intents = expression.mutation_intents();
    assert_eq!(expression.malignant_gene_count(), 1);
    assert_eq!(expression.regeneration_candidate_count(), 1);
    assert!(intents.contains(&"quarantine".to_owned()));
    assert!(intents.contains(&"regenerate".to_owned()));
    assert!(intents.contains(&"rollback".to_owned()));
    assert!(expression.youth_pressure > 0.50);
    assert!(expression.is_read_only_preview());
    assert!(
        expression
            .mutation_plans
            .iter()
            .all(|plan| plan.rollback_anchor_id == "genome:test:stable")
    );
}

#[test]
fn rollback_pressure_quarantines_and_regenerates_active_safety_gene() {
    let genome = ReasoningGenome::default_for_profile(TaskProfile::Coding);

    let expression = genome.express(GenomeExpressionInput {
        profile: TaskProfile::Coding,
        quality: 0.74,
        process_reward: 0.61,
        contradiction_count: 1,
        critical_reflection_issue_count: 0,
        revision_action_count: 1,
        used_memories: 2,
        memory_feedback_updates: 1,
        route_attention_fraction: 0.55,
        agent_team_collision_free: true,
        toolsmith_gate_passed: true,
        drift_memory_write_allowed: false,
        drift_rollback: true,
        runtime_kv_hold: false,
    });

    let intents = expression.mutation_intents();
    assert_eq!(expression.malignant_gene_count(), 1);
    assert_eq!(expression.regeneration_candidate_count(), 1);
    assert!(intents.contains(&"quarantine".to_owned()));
    assert!(intents.contains(&"regenerate".to_owned()));
    assert!(intents.contains(&"rollback".to_owned()));
    assert!(expression.is_read_only_preview());
}
