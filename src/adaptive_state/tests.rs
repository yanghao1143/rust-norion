use super::*;
use crate::disk_kv::DiskKvStore;
use crate::hierarchy::{
    HierarchyState, HierarchyWeights, ProfileHierarchyObservations, ProfileHierarchyWeights,
    TaskProfile,
};
use crate::kv_cache::{MemoryCompactionPolicy, MemoryResidencyState, MemoryRetentionPolicy};
use crate::reasoning_genome::{
    DnaEvolutionCandidateDecision, DnaGeneChain, GeneLifecycleSourceEvidence,
    GeneLifecycleSourceKind, GeneScissorsIntent, GeneScissorsTransactionJournal,
    GeneValidationStatus, MutationPlan, ReasoningGene, ReasoningGeneKind,
};
use crate::router::{ProfileObservations, ProfileThresholds, RouterState};
use crate::tiered_cache::{MemoryPlacement, MemoryTier, TieredCachePlan};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn genome_runtime_applies_splice_and_rolls_back_snapshot() {
    let mut runtime = GenomeRuntimeState::default();
    let candidate = runtime.active(TaskProfile::Coding).clone();
    let plan = MutationPlan::preview(
        "mutation:coding:splice",
        GeneScissorsIntent::Splice,
        "gene:coding:routing",
        "validated replay produced a reusable routing gene",
        "insert the validated routing strategy after its anchor",
        candidate.stable_anchor_id.clone(),
    )
    .with_sources(["replay:validated:routing"])
    .with_replacement("gene:coding:routing:spliced")
    .with_repair_payload(
        "spliced routing strategy",
        "reuse compiler-validated routing evidence",
        ["routing", "validated"],
    )
    .with_validation_status(GeneValidationStatus::Passed);

    let bounded = runtime
        .bounded_mutation_plans(TaskProfile::Coding, &candidate, std::slice::from_ref(&plan))
        .unwrap();
    let receipt = runtime.apply(
        TaskProfile::Coding,
        &candidate,
        &bounded,
        &["splice-preview-journal".to_owned()],
        "approval:splice",
    );

    assert!(receipt.applied, "{}", receipt.reason);
    assert_eq!(receipt.reason, "mutation_applied");
    assert!(receipt.dual_chain_committed);
    assert_eq!(receipt.memory_chain_records, 1);
    let active_chain = &runtime.profile(TaskProfile::Coding).active_chain;
    assert_eq!(
        active_chain.express_chain.len(),
        receipt.express_chain_records
    );
    assert_eq!(active_chain.memory_chain.len(), 1);
    assert!(active_chain.memory_chain[0].applied);
    assert!(active_chain.memory_chain[0].admission_write_authorized);
    let persisted = DnaGeneChain::from_kv_lines(
        &active_chain
            .to_kv_lines()
            .expect("serialize applied dual chain"),
    )
    .expect("reload applied dual chain");
    assert_eq!(persisted, *active_chain);
    assert!(
        runtime
            .active(TaskProfile::Coding)
            .genes
            .iter()
            .any(|gene| gene.id == "gene:coding:routing:spliced")
    );

    let rollback = runtime.rollback(
        TaskProfile::Coding,
        &["splice-rollback-journal".to_owned()],
        "approval:rollback",
    );

    assert!(rollback.applied);
    assert!(rollback.rolled_back);
    assert!(rollback.dual_chain_committed);
    assert!(
        runtime
            .profile(TaskProfile::Coding)
            .active_chain
            .memory_chain
            .is_empty()
    );
    assert!(
        runtime
            .active(TaskProfile::Coding)
            .genes
            .iter()
            .all(|gene| gene.id != "gene:coding:routing:spliced")
    );
}

#[test]
fn genome_runtime_applies_compatible_crossover_and_holds_incompatible_sources() {
    let mut runtime = GenomeRuntimeState::default();
    let mut candidate = runtime.active(TaskProfile::Coding).clone();
    candidate.genes.push(
        ReasoningGene::new(
            "gene:coding:routing:sibling",
            ReasoningGeneKind::Routing,
            "routing sibling",
            "second high-fitness routing strategy",
        )
        .with_tags(["routing", "sibling"]),
    );
    runtime.profile_mut(TaskProfile::Coding).active = candidate.clone();
    let plan = MutationPlan::preview(
        "mutation:coding:crossover",
        GeneScissorsIntent::Crossover,
        "gene:coding:routing",
        "two compatible high-fitness routing genes passed validation",
        "insert one bounded crossover child",
        candidate.stable_anchor_id.clone(),
    )
    .with_sources(["gene:coding:routing", "gene:coding:routing:sibling"])
    .with_replacement("gene:coding:routing:crossover")
    .with_repair_payload(
        "crossover routing strategy",
        "combine compatible routing strengths under rollback",
        ["routing", "validated"],
    )
    .with_validation_status(GeneValidationStatus::Passed);

    let bounded = runtime
        .bounded_mutation_plans(TaskProfile::Coding, &candidate, std::slice::from_ref(&plan))
        .unwrap();
    let receipt = runtime.apply(
        TaskProfile::Coding,
        &candidate,
        &bounded,
        &["crossover-preview-journal".to_owned()],
        "approval:crossover",
    );

    assert!(receipt.applied, "{}", receipt.reason);
    assert!(
        runtime
            .active(TaskProfile::Coding)
            .genes
            .iter()
            .any(|gene| gene.id == "gene:coding:routing:crossover")
    );

    let mut incompatible = plan;
    incompatible.id = "mutation:coding:crossover:incompatible".to_owned();
    incompatible.replacement_gene_id = Some("gene:coding:bad-crossover".to_owned());
    incompatible.source_gene_ids = vec![
        "gene:coding:routing".to_owned(),
        "gene:coding:reflection".to_owned(),
    ];
    let current = runtime.active(TaskProfile::Coding).clone();
    let held = runtime.apply(
        TaskProfile::Coding,
        &current,
        &[incompatible],
        &[],
        "approval:incompatible",
    );
    assert!(!held.applied);
    assert_eq!(held.reason, "crossover_sources_incompatible");
}

#[test]
fn genome_runtime_forces_bounded_gene_phase_transition() {
    let mut runtime = GenomeRuntimeState::default();
    let mut candidate = runtime.active(TaskProfile::Coding).clone();
    while candidate.genes.len() < GENOME_PERSISTED_GENE_CAPACITY {
        let index = candidate.genes.len();
        let fitness = if index == 7 { 0.20 } else { 0.90 };
        candidate.genes.push(
            ReasoningGene::new(
                format!("gene:coding:resident:{index}"),
                ReasoningGeneKind::Routing,
                format!("resident routing {index}"),
                "bounded resident routing evidence",
            )
            .with_health(0, fitness, 0.0),
        );
    }
    let weak_gene_id = "gene:coding:resident:7".to_owned();
    let state = runtime.profile_mut(TaskProfile::Coding);
    state.active = candidate.clone();
    state.gene_residency = GenomeGeneResidency::for_genome(&candidate);

    let plan = MutationPlan::preview(
        "mutation:coding:forced-splice",
        GeneScissorsIntent::Splice,
        "gene:coding:routing",
        "new validated routing evidence must beat the weakest resident",
        "admit one warm candidate and retire one cold payload",
        candidate.stable_anchor_id.clone(),
    )
    .with_sources(["replay:validated:forced-splice"])
    .with_replacement("gene:coding:routing:forced")
    .with_repair_payload(
        "forced routing candidate",
        "reuse stronger validated routing evidence",
        ["routing", "validated"],
    )
    .with_validation_status(GeneValidationStatus::Passed);

    let plans = runtime
        .bounded_mutation_plans(TaskProfile::Coding, &candidate, &[plan])
        .unwrap();
    assert!(
        plans
            .iter()
            .any(|plan| plan.intent == GeneScissorsIntent::Cut)
    );
    let admission = plans
        .iter()
        .find(|plan| plan.id == "mutation:coding:forced-splice")
        .unwrap();
    let phase_victim_id = admission
        .source_evidence
        .iter()
        .find(|evidence| evidence.summary.starts_with("forced_expression_demote="))
        .map(|evidence| evidence.source_id.clone())
        .expect("admission must name its exact forced-expression victim");
    assert!(admission.source_gene_ids.contains(&phase_victim_id));
    assert!(plans.iter().any(|plan| {
        plan.intent == GeneScissorsIntent::Cut && plan.target_gene_id == weak_gene_id
    }));
    let journal = GeneScissorsTransactionJournal::from_mutation_plans(
        TaskProfile::Coding,
        candidate.stable_anchor_id.clone(),
        &plans,
    );
    assert!(journal.transactions.iter().any(|transaction| {
        transaction.source_plan_id == admission.id
            && transaction.stable_anchor_sources.contains(&phase_victim_id)
    }));
    let mut replay = runtime.clone();
    let receipt = runtime.apply(
        TaskProfile::Coding,
        &candidate,
        &plans,
        &["forced-phase-transition".to_owned()],
        "approval:forced-phase-transition",
    );
    let replay_receipt = replay.apply(
        TaskProfile::Coding,
        &candidate,
        &plans,
        &["forced-phase-transition".to_owned()],
        "approval:forced-phase-transition",
    );

    assert!(receipt.applied, "{}", receipt.reason);
    assert_eq!(replay_receipt, receipt);
    assert_eq!(
        replay.profile(TaskProfile::Coding),
        runtime.profile(TaskProfile::Coding)
    );
    let active = runtime.active(TaskProfile::Coding);
    assert_eq!(active.genes.len(), GENOME_PERSISTED_GENE_CAPACITY);
    assert!(
        active
            .genes
            .iter()
            .any(|gene| gene.id == "gene:coding:routing:forced")
    );
    assert!(active.genes.iter().all(|gene| gene.id != weak_gene_id));
    assert!(
        active
            .genes
            .iter()
            .any(|gene| gene.kind == ReasoningGeneKind::Safety)
    );
    let report = runtime.gene_residency_report(TaskProfile::Coding);
    assert_eq!(
        report.borrowed_expression_count,
        GENOME_EXPRESSED_GENE_CAPACITY
    );
    assert_eq!(report.persisted_gene_count, GENOME_PERSISTED_GENE_CAPACITY);
    assert_eq!(report.retired, 1);
    assert_eq!(
        report.last_transition_reason,
        "forced_admission_replaced_weakest"
    );
    let duplicate = runtime.apply(
        TaskProfile::Coding,
        &candidate,
        &plans,
        &["forced-phase-transition".to_owned()],
        "approval:forced-phase-transition",
    );
    assert!(!duplicate.applied);
    assert_eq!(duplicate.reason, "candidate_genome_stale");
    assert_eq!(
        runtime.gene_residency_report(TaskProfile::Coding).retired,
        1
    );
    let borrowed = runtime.borrowed_gene_ids(TaskProfile::Coding);
    runtime.record_gene_expression(TaskProfile::Coding, &borrowed, 1, true);
    runtime.record_gene_expression(TaskProfile::Coding, &borrowed, 2, true);
    let revision_before_rollback = runtime.residency_revision(TaskProfile::Coding);

    let rollback = runtime.rollback(
        TaskProfile::Coding,
        &["forced-phase-rollback".to_owned()],
        "approval:forced-phase-rollback",
    );
    assert!(rollback.applied);
    assert!(rollback.rolled_back);
    assert!(runtime.residency_revision(TaskProfile::Coding) > revision_before_rollback);
    let after_rollback = runtime.profile(TaskProfile::Coding).gene_residency.clone();
    runtime.record_gene_expression(TaskProfile::Coding, &borrowed, 1, true);
    runtime.record_gene_expression(TaskProfile::Coding, &borrowed, 2, true);
    assert_eq!(
        runtime.profile(TaskProfile::Coding).gene_residency,
        after_rollback
    );
    assert!(
        runtime
            .active(TaskProfile::Coding)
            .genes
            .iter()
            .any(|gene| gene.id == weak_gene_id)
    );
}

#[test]
fn forced_expression_swap_binds_each_plan_to_its_own_victim() {
    let mut runtime = GenomeRuntimeState::default();
    let profile = TaskProfile::Coding;
    let mut candidate = runtime.active(profile).clone();
    candidate.genes.push(ReasoningGene::new(
        "gene:coding:resident:extra",
        ReasoningGeneKind::Language,
        "extra resident language strategy",
        "fill the eighth expression seat before two admissions",
    ));
    let state = runtime.profile_mut(profile);
    state.active = candidate.clone();
    state.gene_residency = GenomeGeneResidency::for_genome(&candidate);

    let plans = [
        MutationPlan::preview(
            "mutation:coding:two-admission:routing",
            GeneScissorsIntent::Splice,
            "gene:coding:routing",
            "first validated admission",
            "admit one routing candidate",
            candidate.stable_anchor_id.clone(),
        )
        .with_sources(["gene:coding:routing"])
        .with_replacement("gene:coding:routing:two-admission")
        .with_repair_payload(
            "two-admission routing",
            "first deterministic admission",
            ["routing", "admission"],
        )
        .with_validation_status(GeneValidationStatus::Passed),
        MutationPlan::preview(
            "mutation:coding:two-admission:reflection",
            GeneScissorsIntent::Splice,
            "gene:coding:reflection",
            "second validated admission",
            "admit one reflection candidate",
            candidate.stable_anchor_id.clone(),
        )
        .with_sources(["gene:coding:reflection"])
        .with_replacement("gene:coding:reflection:two-admission")
        .with_repair_payload(
            "two-admission reflection",
            "second deterministic admission",
            ["reflection", "admission"],
        )
        .with_validation_status(GeneValidationStatus::Passed),
    ];
    let bounded = runtime
        .bounded_mutation_plans(profile, &candidate, &plans)
        .unwrap();
    let victims = plans
        .iter()
        .map(|source_plan| {
            let bounded_plan = bounded
                .iter()
                .find(|plan| plan.id == source_plan.id)
                .unwrap();
            let victim = bounded_plan
                .source_evidence
                .iter()
                .find(|evidence| evidence.summary.starts_with("forced_expression_demote="))
                .map(|evidence| evidence.source_id.clone())
                .unwrap();
            assert!(bounded_plan.source_gene_ids.contains(&victim));
            victim
        })
        .collect::<Vec<_>>();
    assert_ne!(victims[0], victims[1]);

    let receipt = runtime.apply(
        profile,
        &candidate,
        &bounded,
        &["two-admission-forced-transition".to_owned()],
        "approval:two-admission",
    );
    assert!(receipt.applied, "{}", receipt.reason);
    assert_eq!(
        runtime
            .gene_residency_report(profile)
            .borrowed_expression_count,
        GENOME_EXPRESSED_GENE_CAPACITY
    );
}

#[test]
fn readmission_into_free_expression_seat_does_not_report_forced_swap() {
    let mut runtime = GenomeRuntimeState::default();
    let profile = TaskProfile::Coding;
    let mut candidate = runtime.active(profile).clone();
    for index in 0..2 {
        candidate.genes.push(ReasoningGene::new(
            format!("gene:coding:free-seat:{index}"),
            ReasoningGeneKind::Language,
            format!("free seat resident {index}"),
            "keep persisted payload count above expression count",
        ));
    }
    let state = runtime.profile_mut(profile);
    state.active = candidate.clone();
    state.gene_residency = GenomeGeneResidency::for_genome(&candidate);
    state.gene_residency.step = 64;
    let cold_ids = state
        .gene_residency
        .records
        .iter()
        .filter(|record| record.residency == MemoryResidencyState::Cold)
        .map(|record| record.gene_id.clone())
        .collect::<Vec<_>>();
    let target_id = cold_ids.first().cloned().unwrap();
    let second_cold_id = state
        .gene_residency
        .records
        .iter()
        .find(|record| {
            record.gene_id != target_id
                && record.residency == MemoryResidencyState::Warm
                && candidate
                    .genes
                    .iter()
                    .find(|gene| gene.id == record.gene_id)
                    .is_some_and(|gene| gene.kind != ReasoningGeneKind::Safety)
        })
        .map(|record| record.gene_id.clone())
        .unwrap();
    state
        .gene_residency
        .records
        .iter_mut()
        .find(|record| record.gene_id == second_cold_id)
        .unwrap()
        .residency = MemoryResidencyState::Cold;
    let target_record = state
        .gene_residency
        .records
        .iter_mut()
        .find(|record| record.gene_id == target_id)
        .unwrap();
    target_record.last_used_step = 0;
    target_record.consumed_evidence_digest = "redaction-digest:older-free-seat".to_owned();
    let target_gene = candidate
        .genes
        .iter()
        .find(|gene| gene.id == target_id)
        .unwrap();
    let plan = MutationPlan::preview(
        "mutation:coding:free-seat-readmission",
        GeneScissorsIntent::Repair,
        target_id.clone(),
        "new evidence may use an already free expression seat",
        "admit without demoting another resident",
        candidate.stable_anchor_id.clone(),
    )
    .with_sources([target_id.clone()])
    .with_repair_payload(
        target_gene.label.clone(),
        target_gene.purpose.clone(),
        target_gene.tags.clone(),
    )
    .with_validation_status(GeneValidationStatus::Passed);
    let bounded = runtime
        .bounded_mutation_plans(profile, &candidate, &[plan])
        .unwrap();
    assert!(bounded.iter().all(|plan| {
        plan.source_evidence
            .iter()
            .all(|evidence| !evidence.summary.starts_with("forced_expression_demote="))
    }));

    let receipt = runtime.apply(
        profile,
        &candidate,
        &bounded,
        &["free-seat-readmission".to_owned()],
        "approval:free-seat-readmission",
    );
    assert!(receipt.applied, "{}", receipt.reason);
    assert_eq!(
        runtime
            .gene_residency_report(profile)
            .last_transition_reason,
        "candidate_admitted_warm"
    );
}

#[test]
fn duplicate_readmission_evidence_stays_consumed_when_forced_victim_changes() {
    let mut runtime = GenomeRuntimeState::default();
    let profile = TaskProfile::Coding;
    let target_id = "gene:coding:routing";
    let mut candidate = runtime.active(profile).clone();
    for index in 0..2 {
        candidate.genes.push(ReasoningGene::new(
            format!("gene:coding:replay-resident:{index}"),
            ReasoningGeneKind::Language,
            format!("replay resident {index}"),
            "fill persisted payloads for victim replay",
        ));
    }
    let state = runtime.profile_mut(profile);
    state.active = candidate.clone();
    state.gene_residency = GenomeGeneResidency::for_genome(&candidate);
    state.gene_residency.step = 64;
    for record in &mut state.gene_residency.records {
        record.residency = if record.gene_id == target_id {
            MemoryResidencyState::Cold
        } else {
            MemoryResidencyState::Warm
        };
        record.last_used_step = 0;
    }
    let scoped_evidence = "redaction-digest:stable-gene-scoped-readmission";
    let target = candidate
        .genes
        .iter()
        .find(|gene| gene.id == target_id)
        .unwrap();
    let plan = MutationPlan::preview(
        "mutation:gene:coding:routing:cold-readmission",
        GeneScissorsIntent::Repair,
        target_id,
        "gene-scoped evidence may readmit dormant routing DNA",
        "preview candidate residency as Warm-phase",
        candidate.stable_anchor_id.clone(),
    )
    .with_source_evidence([GeneLifecycleSourceEvidence::new(
        GeneLifecycleSourceKind::FeedbackSignal,
        scoped_evidence,
        "tenant-scoped request evidence matched this dormant gene",
    )])
    .with_repair_payload(
        target.label.clone(),
        target.purpose.clone(),
        target.tags.clone(),
    )
    .with_validation_status(GeneValidationStatus::Passed);
    let bounded = runtime
        .bounded_mutation_plans(profile, &candidate, &[plan])
        .unwrap();
    let first_victim = bounded[0]
        .source_evidence
        .iter()
        .find(|evidence| evidence.summary.starts_with("forced_expression_demote="))
        .map(|evidence| evidence.source_id.clone())
        .unwrap();
    let receipt = runtime.apply(
        profile,
        &candidate,
        &bounded,
        &["stable-readmission-evidence".to_owned()],
        "approval:stable-readmission-evidence",
    );
    assert!(receipt.applied, "{}", receipt.reason);

    let state = runtime.profile_mut(profile);
    state.gene_residency.step = state.gene_residency.step.saturating_add(64);
    state
        .gene_residency
        .records
        .iter_mut()
        .find(|record| record.gene_id == target_id)
        .unwrap()
        .residency = MemoryResidencyState::Cold;
    state
        .gene_residency
        .records
        .iter_mut()
        .find(|record| record.gene_id == first_victim)
        .unwrap()
        .residency = MemoryResidencyState::Warm;
    let next_victim = state
        .active
        .genes
        .iter_mut()
        .find(|gene| {
            gene.id != target_id
                && gene.id != first_victim
                && gene.kind != ReasoningGeneKind::Safety
        })
        .unwrap();
    next_victim.fitness = 0.10;
    let replay_candidate = state.active.clone();
    let replay_target = replay_candidate
        .genes
        .iter()
        .find(|gene| gene.id == target_id)
        .unwrap();
    let replay_plan = MutationPlan::preview(
        "mutation:gene:coding:routing:cold-readmission",
        GeneScissorsIntent::Repair,
        target_id,
        "gene-scoped evidence may readmit dormant routing DNA",
        "preview candidate residency as Warm-phase",
        replay_candidate.stable_anchor_id.clone(),
    )
    .with_source_evidence([GeneLifecycleSourceEvidence::new(
        GeneLifecycleSourceKind::FeedbackSignal,
        scoped_evidence,
        "tenant-scoped request evidence matched this dormant gene",
    )])
    .with_repair_payload(
        replay_target.label.clone(),
        replay_target.purpose.clone(),
        replay_target.tags.clone(),
    )
    .with_validation_status(GeneValidationStatus::Passed);
    assert_eq!(
        runtime.cold_gene_plan_decision(profile, &replay_candidate, &replay_plan),
        Some(DnaEvolutionCandidateDecision::Hold)
    );
    assert_eq!(
        runtime
            .bounded_mutation_plans(profile, &replay_candidate, &[replay_plan])
            .unwrap_err(),
        "duplicate_gene_transition_evidence"
    );
}

#[test]
fn weaker_candidate_is_held_at_expression_capacity() {
    let mut runtime = GenomeRuntimeState::default();
    let mut candidate = runtime.active(TaskProfile::Coding).clone();
    candidate.genes.push(ReasoningGene::new(
        "gene:coding:routing:resident",
        ReasoningGeneKind::Routing,
        "established routing resident",
        "high-frequency routing evidence",
    ));
    let state = runtime.profile_mut(TaskProfile::Coding);
    state.active = candidate.clone();
    state.gene_residency = GenomeGeneResidency::for_genome(&candidate);
    for record in &mut state.gene_residency.records {
        record.residency = MemoryResidencyState::Hot;
        record.opportunities = 10;
        record.hits = 10;
        record.last_used_step = 10;
    }
    state.gene_residency.step = 10;
    let plan = MutationPlan::preview(
        "mutation:coding:weak-splice",
        GeneScissorsIntent::Splice,
        "gene:coding:routing",
        "candidate has validation but no usage advantage",
        "hold equal or weaker candidates to prevent churn",
        candidate.stable_anchor_id.clone(),
    )
    .with_sources(["replay:validated:weak-splice"])
    .with_replacement("gene:coding:routing:weak-candidate")
    .with_repair_payload(
        "weak routing candidate",
        "candidate must beat the weakest resident",
        ["routing", "candidate"],
    )
    .with_validation_status(GeneValidationStatus::Passed);

    let error = runtime
        .bounded_mutation_plans(TaskProfile::Coding, &candidate, &[plan])
        .unwrap_err();

    assert_eq!(error, "gene_candidate_did_not_beat_weakest_resident");
    assert_eq!(runtime.active(TaskProfile::Coding).genes.len(), 8);
}

#[test]
fn cold_duplicate_evidence_never_wakes_gene() {
    let mut runtime = GenomeRuntimeState::default();
    let profile = TaskProfile::Coding;
    let gene_id = "gene:coding:routing";
    let state = runtime.profile_mut(profile);
    state.gene_residency.step = 64;
    let record = state
        .gene_residency
        .records
        .iter_mut()
        .find(|record| record.gene_id == gene_id)
        .unwrap();
    record.residency = MemoryResidencyState::Cold;
    record.consumed_evidence_digest = "redaction-digest:duplicate".to_owned();

    assert_eq!(
        runtime
            .cold_gene_readmission_decision(profile, gene_id, "redaction-digest:duplicate", 900,),
        DnaEvolutionCandidateDecision::Hold
    );
    assert_eq!(
        runtime.cold_gene_readmission_decision(
            profile,
            gene_id,
            "redaction-digest:new-evidence",
            900,
        ),
        DnaEvolutionCandidateDecision::CandidatePreview
    );
    assert!(
        !runtime
            .borrowed_gene_ids(profile)
            .contains(&gene_id.to_owned())
    );
    assert_eq!(
        runtime
            .profile(profile)
            .gene_residency
            .records
            .iter()
            .find(|record| record.gene_id == gene_id)
            .unwrap()
            .residency,
        MemoryResidencyState::Cold
    );
}

#[test]
fn cold_new_evidence_enters_warm_only_after_explicit_apply() {
    let mut runtime = GenomeRuntimeState::default();
    let profile = TaskProfile::Coding;
    let gene_id = "gene:coding:routing";
    let state = runtime.profile_mut(profile);
    state.gene_residency.step = 64;
    let record = state
        .gene_residency
        .records
        .iter_mut()
        .find(|record| record.gene_id == gene_id)
        .unwrap();
    record.residency = MemoryResidencyState::Cold;
    record.consumed_evidence_digest = "redaction-digest:old-routing-evidence".to_owned();
    let candidate = runtime.active(profile).clone();
    let plan = MutationPlan::preview(
        "mutation:coding:routing:readmission",
        GeneScissorsIntent::Relabel,
        gene_id,
        "new validated evidence may readmit one cold gene",
        "enter Warm first and require real usage before Hot",
        candidate.stable_anchor_id.clone(),
    )
    .with_sources(["replay:new-routing-evidence"])
    .with_repair_payload(
        "readmitted routing strategy",
        "use new validated routing evidence",
        ["routing", "readmission"],
    )
    .with_validation_status(GeneValidationStatus::Passed);
    let plans = runtime
        .bounded_mutation_plans(profile, &candidate, &[plan])
        .unwrap();

    assert_eq!(
        runtime
            .profile(profile)
            .gene_residency
            .records
            .iter()
            .find(|record| record.gene_id == gene_id)
            .unwrap()
            .residency,
        MemoryResidencyState::Cold
    );
    let receipt = runtime.apply(
        profile,
        &candidate,
        &plans,
        &["cold-readmission".to_owned()],
        "approval:cold-readmission",
    );
    assert!(receipt.applied, "{}", receipt.reason);
    assert_eq!(
        runtime
            .profile(profile)
            .gene_residency
            .records
            .iter()
            .find(|record| record.gene_id == gene_id)
            .unwrap()
            .residency,
        MemoryResidencyState::Warm
    );
}

#[test]
fn interleaved_expression_replay_is_idempotent_but_distinct_live_uses_count() {
    let mut runtime = GenomeRuntimeState::default();
    let profile = TaskProfile::Coding;
    let borrowed = runtime.borrowed_gene_ids(profile);
    runtime.record_gene_expression(profile, &borrowed, 1, true);
    runtime.record_gene_expression(profile, &borrowed, 2, false);
    let after_two = runtime.profile(profile).gene_residency.clone();

    runtime.record_gene_expression(profile, &borrowed, 1, true);
    runtime.record_gene_expression(profile, &borrowed, 2, false);

    assert_eq!(runtime.profile(profile).gene_residency, after_two);

    runtime.record_gene_expression(profile, &borrowed, 3, true);
    let after_three = &runtime.profile(profile).gene_residency;
    assert_eq!(after_three.step, after_two.step + 1);
    assert_eq!(after_three.last_observation_sequence, 3);
    assert!(
        after_three
            .records
            .iter()
            .filter(|record| borrowed.contains(&record.gene_id))
            .all(|record| record.opportunities == 3)
    );
}

#[test]
fn usage_observation_does_not_erase_transition_evidence() {
    let mut runtime = GenomeRuntimeState::default();
    let profile = TaskProfile::Coding;
    let borrowed = runtime.borrowed_gene_ids(profile);
    let gene_id = borrowed.first().unwrap().clone();
    let evidence = "redaction-digest:transition-evidence";
    runtime
        .profile_mut(profile)
        .gene_residency
        .records
        .iter_mut()
        .find(|record| record.gene_id == gene_id)
        .unwrap()
        .consumed_evidence_digest = evidence.to_owned();

    runtime.record_gene_expression(profile, &borrowed, 1, true);

    assert_eq!(
        runtime
            .profile(profile)
            .gene_residency
            .records
            .iter()
            .find(|record| record.gene_id == gene_id)
            .unwrap()
            .consumed_evidence_digest,
        evidence
    );
}

#[test]
fn gene_residency_thresholds_keep_hysteresis() {
    let mut runtime = GenomeRuntimeState::default();
    let profile = TaskProfile::Coding;
    let state = runtime.profile_mut(profile);
    for (gene_id, residency) in [
        ("gene:coding:retrieval", MemoryResidencyState::Hot),
        ("gene:coding:routing", MemoryResidencyState::Warm),
        ("gene:coding:reflection", MemoryResidencyState::Cold),
    ] {
        let record = state
            .gene_residency
            .records
            .iter_mut()
            .find(|record| record.gene_id == gene_id)
            .unwrap();
        record.residency = residency;
        record.opportunities = 10;
        record.hits = 0;
        record.failures = 0;
    }

    runtime.record_gene_expression(profile, &[], 1, true);

    let records = &runtime.profile(profile).gene_residency.records;
    for (gene_id, expected) in [
        ("gene:coding:retrieval", MemoryResidencyState::Hot),
        ("gene:coding:routing", MemoryResidencyState::Warm),
        ("gene:coding:reflection", MemoryResidencyState::Cold),
    ] {
        assert_eq!(
            records
                .iter()
                .find(|record| record.gene_id == gene_id)
                .unwrap()
                .residency,
            expected
        );
    }
}

#[test]
fn gene_residency_legacy_defaults_warm_and_invalid_sidecar_fails_closed() {
    let legacy_path = temp_path("gene-residency-legacy");
    let state = crate::engine::NoironEngine::new().adaptive_state();
    state.save_to_disk_kv(&legacy_path).unwrap();
    let mut store = DiskKvStore::open(&legacy_path).unwrap();
    store.delete("adaptive/genome/residency_schema").unwrap();
    for key in store.keys_with_prefix("adaptive/genome/") {
        if key.ends_with("/residency") || key.ends_with("/previous_residency") {
            store.delete(&key).unwrap();
        }
    }
    drop(store);

    let legacy = AdaptiveState::load_from_disk_kv(&legacy_path)
        .unwrap()
        .unwrap();
    let legacy_report = legacy
        .genome_runtime
        .gene_residency_report(TaskProfile::Coding);
    assert_eq!(legacy_report.warm, 7);
    assert_eq!(legacy_report.borrowed_expression_count, 7);

    let marker_loss_path = temp_path("gene-residency-marker-loss");
    state.save_to_disk_kv(&marker_loss_path).unwrap();
    let mut store = DiskKvStore::open(&marker_loss_path).unwrap();
    store.delete("adaptive/genome/residency_schema").unwrap();
    drop(store);
    let error = AdaptiveState::load_from_disk_kv(&marker_loss_path).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);

    let invalid_path = temp_path("gene-residency-invalid");
    state.save_to_disk_kv(&invalid_path).unwrap();
    let mut store = DiskKvStore::open(&invalid_path).unwrap();
    store
        .put("adaptive/genome/coding/residency", "invalid-sidecar")
        .unwrap();
    drop(store);
    let error = AdaptiveState::load_from_disk_kv(&invalid_path).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);

    let mismatch_path = temp_path("gene-residency-generation-mismatch");
    state.save_to_disk_kv(&mismatch_path).unwrap();
    let mut store = DiskKvStore::open(&mismatch_path).unwrap();
    let bytes = store
        .get("adaptive/genome/coding/residency")
        .unwrap()
        .unwrap();
    let value = String::from_utf8(bytes).unwrap();
    let mut lines = value.lines();
    let mut header = lines.next().unwrap().split('\t').collect::<Vec<_>>();
    header[2] = "99";
    let mismatched = std::iter::once(header.join("\t"))
        .chain(lines.map(str::to_owned))
        .collect::<Vec<_>>()
        .join("\n");
    store
        .put("adaptive/genome/coding/residency", mismatched)
        .unwrap();
    drop(store);
    let error = AdaptiveState::load_from_disk_kv(&mismatch_path).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);

    cleanup(legacy_path);
    cleanup(marker_loss_path);
    cleanup(invalid_path);
    cleanup(mismatch_path);
}

#[test]
fn invalid_residency_snapshot_does_not_replace_last_committed_state() {
    let path = temp_path("gene-residency-atomic-snapshot");
    let state = crate::engine::NoironEngine::new().adaptive_state();
    state.save_to_disk_kv(&path).unwrap();
    let committed = AdaptiveState::load_from_disk_kv(&path).unwrap().unwrap();
    let mut invalid = state.clone();
    invalid
        .genome_runtime
        .profile_mut(TaskProfile::Coding)
        .gene_residency
        .records
        .pop();

    let error = invalid.save_to_disk_kv(&path).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    let reloaded = AdaptiveState::load_from_disk_kv(&path).unwrap().unwrap();
    assert_eq!(
        reloaded.genome_runtime.profile(TaskProfile::Coding),
        committed.genome_runtime.profile(TaskProfile::Coding)
    );
    cleanup(path);
}

#[test]
fn persisted_gene_capacity_fails_closed() {
    let path = temp_path("gene-residency-persisted-capacity");
    let mut state = crate::engine::NoironEngine::new().adaptive_state();
    let genome = &mut state.genome_runtime.profile_mut(TaskProfile::Coding).active;
    while genome.genes.len() <= GENOME_PERSISTED_GENE_CAPACITY {
        let index = genome.genes.len();
        genome.genes.push(ReasoningGene::new(
            format!("gene:coding:over-cap:{index}"),
            ReasoningGeneKind::Language,
            format!("over cap gene {index}"),
            "invalid persisted payload beyond the hard cap",
        ));
    }

    let error = state.save_to_disk_kv(&path).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    assert!(!path.exists());
    cleanup(path);
}

#[test]
fn previous_residency_generation_mismatch_fails_closed() {
    let path = temp_path("gene-residency-previous-generation-mismatch");
    let mut state = crate::engine::NoironEngine::new().adaptive_state();
    let profile = TaskProfile::Coding;
    let candidate = state.genome_runtime.active(profile).clone();
    let plan = MutationPlan::preview(
        "mutation:coding:previous-generation-test",
        GeneScissorsIntent::Splice,
        "gene:coding:routing",
        "create a previous snapshot for generation binding",
        "persist a rollback-bound previous residency sidecar",
        candidate.stable_anchor_id.clone(),
    )
    .with_sources(["gene:coding:routing"])
    .with_replacement("gene:coding:routing:previous-generation-test")
    .with_repair_payload(
        "previous generation routing",
        "verify previous residency generation binding",
        ["routing", "generation"],
    )
    .with_validation_status(GeneValidationStatus::Passed);
    let receipt = state.genome_runtime.apply(
        profile,
        &candidate,
        &[plan],
        &["previous-generation-test".to_owned()],
        "approval:previous-generation-test",
    );
    assert!(receipt.applied, "{}", receipt.reason);
    state.save_to_disk_kv(&path).unwrap();

    let mut store = DiskKvStore::open(&path).unwrap();
    let key = "adaptive/genome/coding/previous_residency";
    let value = String::from_utf8(store.get(key).unwrap().unwrap()).unwrap();
    let mut lines = value.lines();
    let mut header = lines.next().unwrap().split('\t').collect::<Vec<_>>();
    header[2] = "99";
    let mismatched = std::iter::once(header.join("\t"))
        .chain(lines.map(str::to_owned))
        .collect::<Vec<_>>()
        .join("\n");
    store.put(key, mismatched).unwrap();
    drop(store);

    let error = AdaptiveState::load_from_disk_kv(&path).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    cleanup(path);
}

#[test]
fn adaptive_state_roundtrips_through_disk_kv() {
    let path = temp_path("adaptive-state");
    let mut state = AdaptiveState {
        router: RouterState {
            threshold: 0.61,
            observations: 17,
            profile_thresholds: ProfileThresholds {
                general: 0.61,
                coding: 0.49,
                writing: 0.66,
                long_document: 0.42,
            },
            profile_observations: ProfileObservations {
                general: 8,
                coding: 5,
                writing: 3,
                long_document: 1,
            },
        },
        hierarchy: HierarchyState {
            current: HierarchyWeights::new(0.2, 0.6, 0.2),
            profile_weights: ProfileHierarchyWeights {
                general: HierarchyWeights::new(0.36, 0.42, 0.22),
                coding: HierarchyWeights::new(0.18, 0.68, 0.14),
                writing: HierarchyWeights::new(0.60, 0.26, 0.14),
                long_document: HierarchyWeights::new(0.24, 0.18, 0.58),
            },
            profile_observations: ProfileHierarchyObservations {
                general: 2,
                coding: 7,
                writing: 5,
                long_document: 3,
            },
        },
        tier_plan: TieredCachePlan::new(vec![MemoryPlacement {
            id: 7,
            tier: MemoryTier::WarmRam,
            score: 0.42,
            reason: "warm\tstate".to_owned(),
        }]),
        memory_retention_policy: MemoryRetentionPolicy {
            stale_after: 11,
            decay_rate: 0.12,
            remove_below_strength: 0.08,
            remove_after_failures: 7,
        },
        memory_compaction_policy: MemoryCompactionPolicy {
            similarity_threshold: 0.91,
            max_candidates: 64,
            max_merges: 4,
        },
        evolution_ledger: EvolutionLedger {
            live_inference_runs: 11,
            live_router_threshold_mutations: 8,
            live_hierarchy_weight_mutations: 6,
            live_router_threshold_delta: 0.19,
            live_hierarchy_weight_delta: 0.13,
            live_online_reward_feedbacks: 6,
            live_online_reward_reinforcements: 4,
            live_online_reward_penalties: 2,
            live_online_reward_strength: 3.25,
            live_online_reward_reinforcement_strength: 2.15,
            live_online_reward_penalty_strength: 1.10,
            live_memory_reinforcements: 9,
            live_memory_penalties: 4,
            live_stored_memories: 3,
            live_stored_gist_memories: 5,
            live_stored_runtime_kv_memories: 2,
            live_reflection_issues: 7,
            live_critical_reflection_issues: 1,
            live_revision_actions: 10,
            replay_runs: 3,
            replay_items: 9,
            router_threshold_mutations: 5,
            hierarchy_weight_mutations: 7,
            router_threshold_delta: 0.42,
            hierarchy_weight_delta: 0.21,
            memory_reinforcements: 4,
            memory_penalties: 2,
            replay_live_memory_feedback_items: 3,
            replay_live_memory_feedback_reinforcements: 5,
            replay_live_memory_feedback_penalties: 1,
            replay_live_memory_feedback_detail_items: 2,
            replay_live_memory_feedback_applied: 4,
            replay_live_memory_feedback_removed: 1,
            replay_live_memory_feedback_missing: 1,
            replay_live_memory_feedback_strength_delta: 0.72,
            replay_rust_check_items: 2,
            replay_rust_check_passed: 2,
            replay_rust_check_failed: 0,
            replay_rust_check_diagnostic_chars: 17,
            replay_rust_check_live_memory_feedback_items: 2,
            replay_rust_check_live_memory_feedback_updates: 5,
            replay_rust_check_live_memory_feedback_applied: 4,
            replay_rust_check_live_memory_feedback_strength_delta: 0.68,
            replay_business_contract_items: 3,
            replay_business_contract_passed: 3,
            replay_business_contract_failed: 0,
            replay_business_contract_raw_passed: 1,
            replay_business_contract_raw_failed: 2,
            replay_business_contract_response_normalized: 2,
            replay_business_contract_sanitized: 0,
            replay_business_contract_canonical_fallbacks: 2,
            replay_live_evolution_items: 4,
            replay_live_evolution_router_threshold_mutations: 2,
            replay_live_evolution_hierarchy_weight_mutations: 1,
            replay_live_evolution_router_threshold_delta: 0.08,
            replay_live_evolution_hierarchy_weight_delta: 0.05,
            replay_live_evolution_online_reward_feedbacks: 3,
            replay_live_evolution_online_reward_reinforcements: 2,
            replay_live_evolution_online_reward_penalties: 1,
            replay_live_evolution_online_reward_strength: 1.75,
            replay_live_evolution_online_reward_reinforcement_strength: 1.20,
            replay_live_evolution_online_reward_penalty_strength: 0.55,
            replay_live_evolution_memory_updates: 6,
            replay_live_evolution_stored_memory_updates: 3,
            replay_live_evolution_reflection_issues: 5,
            replay_live_evolution_critical_reflection_issues: 1,
            replay_live_evolution_revision_actions: 4,
            recursive_replay_items: 1,
            recursive_runtime_calls: 8,
            drift_rollbacks: 2,
            rollback_router_threshold_delta: 0.11,
            rollback_hierarchy_weight_delta: 0.09,
            external_feedbacks: 2,
            external_feedback_reinforcements: 3,
            external_feedback_penalties: 1,
            external_feedback_memory_updates: 4,
            external_feedback_removed: 1,
            external_feedback_missing: 2,
            external_feedback_strength_delta: 0.31,
        },
        genome_runtime: GenomeRuntimeState::default(),
    };

    let coding_residency = &mut state
        .genome_runtime
        .profile_mut(TaskProfile::Coding)
        .gene_residency;
    coding_residency.step = 9;
    coding_residency.last_observation_sequence = 17;
    coding_residency.last_transition_reason = "usage_promoted_hot".to_owned();
    let routing = coding_residency
        .records
        .iter_mut()
        .find(|record| record.gene_id == "gene:coding:routing")
        .unwrap();
    routing.residency = MemoryResidencyState::Hot;
    routing.opportunities = 4;
    routing.hits = 4;
    routing.last_used_step = 9;
    routing.consumed_evidence_digest = "redaction-digest:roundtrip".to_owned();

    state.save_to_disk_kv(&path).unwrap();
    let loaded = AdaptiveState::load_from_disk_kv(&path).unwrap().unwrap();

    assert!((loaded.router.threshold - 0.61).abs() < 0.0001);
    assert_eq!(loaded.router.observations, 17);
    assert!((loaded.router.profile_thresholds.coding - 0.49).abs() < 0.0001);
    assert_eq!(loaded.router.profile_observations.writing, 3);
    assert!((loaded.hierarchy.current.local - 0.6).abs() < 0.0001);
    assert!((loaded.hierarchy.profile_weights.coding.local - 0.68).abs() < 0.0001);
    assert_eq!(loaded.hierarchy.profile_observations.long_document, 3);
    let placement = loaded.tier_plan.placement_for(7).unwrap();
    assert_eq!(placement.tier, MemoryTier::WarmRam);
    assert_eq!(placement.reason, "warm\tstate");
    assert_eq!(loaded.memory_retention_policy.stale_after, 11);
    assert!((loaded.memory_retention_policy.decay_rate - 0.12).abs() < 0.0001);
    assert!((loaded.memory_retention_policy.remove_below_strength - 0.08).abs() < 0.0001);
    assert_eq!(loaded.memory_retention_policy.remove_after_failures, 7);
    assert!((loaded.memory_compaction_policy.similarity_threshold - 0.91).abs() < 0.0001);
    assert_eq!(loaded.memory_compaction_policy.max_candidates, 64);
    assert_eq!(loaded.memory_compaction_policy.max_merges, 4);
    assert_eq!(loaded.evolution_ledger.replay_runs, 3);
    assert_eq!(loaded.evolution_ledger.live_inference_runs, 11);
    assert_eq!(loaded.evolution_ledger.live_router_threshold_mutations, 8);
    assert_eq!(loaded.evolution_ledger.live_hierarchy_weight_mutations, 6);
    assert!((loaded.evolution_ledger.live_router_threshold_delta - 0.19).abs() < 0.0001);
    assert!((loaded.evolution_ledger.live_hierarchy_weight_delta - 0.13).abs() < 0.0001);
    assert_eq!(loaded.evolution_ledger.live_online_reward_feedbacks, 6);
    assert_eq!(loaded.evolution_ledger.live_online_reward_reinforcements, 4);
    assert_eq!(loaded.evolution_ledger.live_online_reward_penalties, 2);
    assert!((loaded.evolution_ledger.live_online_reward_strength - 3.25).abs() < 0.0001);
    assert!(
        (loaded
            .evolution_ledger
            .live_online_reward_reinforcement_strength
            - 2.15)
            .abs()
            < 0.0001
    );
    assert!((loaded.evolution_ledger.live_online_reward_penalty_strength - 1.10).abs() < 0.0001);
    assert_eq!(loaded.evolution_ledger.live_memory_updates(), 13);
    assert_eq!(loaded.evolution_ledger.live_stored_memory_updates(), 10);
    assert_eq!(loaded.evolution_ledger.live_reflection_issues, 7);
    assert_eq!(loaded.evolution_ledger.live_critical_reflection_issues, 1);
    assert_eq!(loaded.evolution_ledger.live_revision_actions, 10);
    assert_eq!(
        loaded
            .genome_runtime
            .profile(TaskProfile::Coding)
            .gene_residency,
        state
            .genome_runtime
            .profile(TaskProfile::Coding)
            .gene_residency
    );
    assert_eq!(loaded.evolution_ledger.replay_items, 9);
    assert_eq!(loaded.evolution_ledger.router_threshold_mutations, 5);
    assert_eq!(loaded.evolution_ledger.hierarchy_weight_mutations, 7);
    assert!((loaded.evolution_ledger.router_threshold_delta - 0.42).abs() < 0.0001);
    assert!((loaded.evolution_ledger.hierarchy_weight_delta - 0.21).abs() < 0.0001);
    assert_eq!(loaded.evolution_ledger.memory_updates(), 6);
    assert_eq!(loaded.evolution_ledger.replay_live_memory_feedback_items, 3);
    assert_eq!(
        loaded
            .evolution_ledger
            .replay_live_memory_feedback_updates(),
        6
    );
    assert_eq!(
        loaded
            .evolution_ledger
            .replay_live_memory_feedback_reinforcements,
        5
    );
    assert_eq!(
        loaded
            .evolution_ledger
            .replay_live_memory_feedback_penalties,
        1
    );
    assert_eq!(
        loaded
            .evolution_ledger
            .replay_live_memory_feedback_detail_items,
        2
    );
    assert_eq!(
        loaded.evolution_ledger.replay_live_memory_feedback_applied,
        4
    );
    assert_eq!(
        loaded.evolution_ledger.replay_live_memory_feedback_removed,
        1
    );
    assert_eq!(
        loaded.evolution_ledger.replay_live_memory_feedback_missing,
        1
    );
    assert_eq!(
        loaded
            .evolution_ledger
            .replay_live_memory_feedback_detail_updates(),
        5
    );
    assert!(
        (loaded
            .evolution_ledger
            .replay_live_memory_feedback_strength_delta
            - 0.72)
            .abs()
            < 0.0001
    );
    assert_eq!(loaded.evolution_ledger.replay_rust_check_items, 2);
    assert_eq!(loaded.evolution_ledger.replay_rust_check_passed, 2);
    assert_eq!(loaded.evolution_ledger.replay_rust_check_failed, 0);
    assert_eq!(loaded.evolution_ledger.replay_rust_check_total(), 2);
    assert_eq!(
        loaded.evolution_ledger.replay_rust_check_diagnostic_chars,
        17
    );
    assert_eq!(
        loaded
            .evolution_ledger
            .replay_rust_check_live_memory_feedback_items,
        2
    );
    assert_eq!(
        loaded
            .evolution_ledger
            .replay_rust_check_live_memory_feedback_updates,
        5
    );
    assert_eq!(
        loaded
            .evolution_ledger
            .replay_rust_check_live_memory_feedback_applied,
        4
    );
    assert!(
        (loaded
            .evolution_ledger
            .replay_rust_check_live_memory_feedback_strength_delta
            - 0.68)
            .abs()
            < 0.0001
    );
    assert_eq!(loaded.evolution_ledger.replay_business_contract_items, 3);
    assert_eq!(loaded.evolution_ledger.replay_business_contract_passed, 3);
    assert_eq!(loaded.evolution_ledger.replay_business_contract_failed, 0);
    assert_eq!(loaded.evolution_ledger.replay_business_contract_total(), 3);
    assert_eq!(
        loaded.evolution_ledger.replay_business_contract_raw_passed,
        1
    );
    assert_eq!(
        loaded.evolution_ledger.replay_business_contract_raw_failed,
        2
    );
    assert_eq!(
        loaded
            .evolution_ledger
            .replay_business_contract_response_normalized,
        2
    );
    assert_eq!(
        loaded.evolution_ledger.replay_business_contract_sanitized,
        0
    );
    assert_eq!(
        loaded
            .evolution_ledger
            .replay_business_contract_canonical_fallbacks,
        2
    );
    assert_eq!(loaded.evolution_ledger.replay_live_evolution_items, 4);
    assert_eq!(
        loaded
            .evolution_ledger
            .replay_live_evolution_router_threshold_mutations,
        2
    );
    assert_eq!(
        loaded
            .evolution_ledger
            .replay_live_evolution_hierarchy_weight_mutations,
        1
    );
    assert!(
        (loaded
            .evolution_ledger
            .replay_live_evolution_router_threshold_delta
            - 0.08)
            .abs()
            < 0.0001
    );
    assert!(
        (loaded
            .evolution_ledger
            .replay_live_evolution_hierarchy_weight_delta
            - 0.05)
            .abs()
            < 0.0001
    );
    assert_eq!(
        loaded
            .evolution_ledger
            .replay_live_evolution_online_reward_feedbacks,
        3
    );
    assert_eq!(
        loaded
            .evolution_ledger
            .replay_live_evolution_online_reward_reinforcements,
        2
    );
    assert_eq!(
        loaded
            .evolution_ledger
            .replay_live_evolution_online_reward_penalties,
        1
    );
    assert!(
        (loaded
            .evolution_ledger
            .replay_live_evolution_online_reward_strength
            - 1.75)
            .abs()
            < 0.0001
    );
    assert!(
        (loaded
            .evolution_ledger
            .replay_live_evolution_online_reward_reinforcement_strength
            - 1.20)
            .abs()
            < 0.0001
    );
    assert!(
        (loaded
            .evolution_ledger
            .replay_live_evolution_online_reward_penalty_strength
            - 0.55)
            .abs()
            < 0.0001
    );
    assert_eq!(
        loaded.evolution_ledger.replay_live_evolution_memory_updates,
        6
    );
    assert_eq!(
        loaded
            .evolution_ledger
            .replay_live_evolution_stored_memory_updates,
        3
    );
    assert_eq!(
        loaded
            .evolution_ledger
            .replay_live_evolution_reflection_issues,
        5
    );
    assert_eq!(
        loaded
            .evolution_ledger
            .replay_live_evolution_critical_reflection_issues,
        1
    );
    assert_eq!(
        loaded
            .evolution_ledger
            .replay_live_evolution_revision_actions,
        4
    );
    assert_eq!(loaded.evolution_ledger.recursive_replay_items, 1);
    assert_eq!(loaded.evolution_ledger.recursive_runtime_calls, 8);
    assert_eq!(loaded.evolution_ledger.drift_rollbacks, 2);
    assert!((loaded.evolution_ledger.rollback_router_threshold_delta - 0.11).abs() < 0.0001);
    assert!((loaded.evolution_ledger.rollback_hierarchy_weight_delta - 0.09).abs() < 0.0001);
    assert_eq!(loaded.evolution_ledger.external_feedbacks, 2);
    assert_eq!(loaded.evolution_ledger.external_feedback_reinforcements, 3);
    assert_eq!(loaded.evolution_ledger.external_feedback_penalties, 1);
    assert_eq!(loaded.evolution_ledger.external_feedback_memory_updates, 4);
    assert_eq!(loaded.evolution_ledger.external_feedback_removed, 1);
    assert_eq!(loaded.evolution_ledger.external_feedback_missing, 2);
    assert!((loaded.evolution_ledger.external_feedback_strength_delta - 0.31).abs() < 0.0001);
    cleanup(path);
}

#[test]
fn evolution_ledger_loads_legacy_without_rollback_fields() {
    let legacy = "3\t9\t5\t7\t0.420000\t0.210000\t4\t2\t1\t8";
    let ledger = parse_evolution_ledger(legacy).unwrap();

    assert_eq!(ledger.replay_runs, 3);
    assert_eq!(ledger.memory_updates(), 6);
    assert_eq!(ledger.replay_live_memory_feedback_items, 0);
    assert_eq!(ledger.replay_live_memory_feedback_updates(), 0);
    assert_eq!(ledger.replay_live_memory_feedback_detail_items, 0);
    assert_eq!(ledger.replay_live_memory_feedback_detail_updates(), 0);
    assert_eq!(ledger.live_online_reward_strength, 0.0);
    assert_eq!(ledger.live_online_reward_reinforcement_strength, 0.0);
    assert_eq!(ledger.live_online_reward_penalty_strength, 0.0);
    assert_eq!(ledger.replay_live_evolution_online_reward_strength, 0.0);
    assert_eq!(
        ledger.replay_live_evolution_online_reward_reinforcement_strength,
        0.0
    );
    assert_eq!(
        ledger.replay_live_evolution_online_reward_penalty_strength,
        0.0
    );
    assert_eq!(ledger.recursive_runtime_calls, 8);
    assert_eq!(ledger.drift_rollbacks, 0);
    assert_eq!(ledger.rollback_router_threshold_delta, 0.0);
    assert_eq!(ledger.rollback_hierarchy_weight_delta, 0.0);
    assert_eq!(ledger.external_feedbacks, 0);
    assert_eq!(ledger.external_feedback_memory_updates, 0);
    assert_eq!(ledger.external_feedback_strength_delta, 0.0);
    assert_eq!(ledger.replay_rust_check_items, 0);
    assert_eq!(ledger.replay_rust_check_total(), 0);
    assert_eq!(ledger.replay_rust_check_diagnostic_chars, 0);
    assert_eq!(ledger.replay_rust_check_live_memory_feedback_updates, 0);
    assert_eq!(
        ledger.replay_rust_check_live_memory_feedback_strength_delta,
        0.0
    );
    assert_eq!(ledger.replay_business_contract_items, 0);
    assert_eq!(ledger.replay_business_contract_total(), 0);
    assert_eq!(ledger.replay_business_contract_raw_failed, 0);
    assert_eq!(ledger.replay_business_contract_canonical_fallbacks, 0);
}

#[test]
fn adaptive_state_loads_legacy_files_without_memory_policies() {
    let path = temp_path("adaptive-state-legacy");
    {
        let mut store = DiskKvStore::open(&path).unwrap();
        store.put("adaptive/router", b"0.610000\t17").unwrap();
        store
            .put("adaptive/hierarchy", b"0.200000\t0.600000\t0.200000")
            .unwrap();
        store.compact().unwrap();
    }

    let loaded = AdaptiveState::load_from_disk_kv(&path).unwrap().unwrap();

    assert!((loaded.router.threshold - 0.61).abs() < 0.0001);
    assert_eq!(loaded.router.observations, 17);
    assert!((loaded.hierarchy.current.local - 0.6).abs() < 0.0001);
    assert_eq!(
        loaded.memory_retention_policy.stale_after,
        MemoryRetentionPolicy::default().stale_after
    );
    assert!(
        (loaded.memory_compaction_policy.similarity_threshold
            - MemoryCompactionPolicy::default().similarity_threshold)
            .abs()
            < 0.0001
    );
    assert_eq!(loaded.evolution_ledger, EvolutionLedger::default());
    cleanup(path);
}

#[test]
fn adaptive_state_load_recovers_interrupted_compaction_backup() {
    let path = temp_path("adaptive-state-interrupted-compact");
    let compact_path = path.with_extension("compact");
    let backup_path = path.with_extension("compact.bak");
    let mut state = AdaptiveState {
        router: RouterState {
            threshold: 0.61,
            observations: 17,
            profile_thresholds: ProfileThresholds::from_single(0.61),
            profile_observations: ProfileObservations::default(),
        },
        hierarchy: HierarchyState {
            current: HierarchyWeights::new(0.2, 0.6, 0.2),
            profile_weights: ProfileHierarchyWeights::from_single(HierarchyWeights::new(
                0.2, 0.6, 0.2,
            )),
            profile_observations: ProfileHierarchyObservations::default(),
        },
        tier_plan: TieredCachePlan::default(),
        memory_retention_policy: MemoryRetentionPolicy::default(),
        memory_compaction_policy: MemoryCompactionPolicy::default(),
        evolution_ledger: EvolutionLedger::default(),
        genome_runtime: GenomeRuntimeState::default(),
    };
    state
        .genome_runtime
        .profile_mut(TaskProfile::Coding)
        .generation = 7;
    state.save_to_disk_kv(&path).unwrap();
    fs::copy(&path, &compact_path).unwrap();
    fs::rename(&path, &backup_path).unwrap();

    let loaded = AdaptiveState::load_from_disk_kv(&path).unwrap().unwrap();

    assert!((loaded.router.threshold - 0.61).abs() < 0.0001);
    assert!((loaded.hierarchy.current.local - 0.6).abs() < 0.0001);
    assert_eq!(
        loaded
            .genome_runtime
            .profile(TaskProfile::Coding)
            .generation,
        7
    );
    cleanup(path);
}

fn temp_path(label: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "rust-norion-{label}-{}-{nanos}.ndkv",
        std::process::id()
    ))
}

fn cleanup(path: std::path::PathBuf) {
    let _ = fs::remove_file(path);
}
