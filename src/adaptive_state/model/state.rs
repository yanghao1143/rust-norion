use crate::hierarchy::HierarchyState;
use crate::hierarchy::TaskProfile;
use crate::kv_cache::{MemoryCompactionPolicy, MemoryRetentionPolicy};
use crate::reasoning_genome::{
    GeneScissorsIntent, GeneValidationStatus, MutationPlan, ReasoningGene, ReasoningGeneStatus,
    ReasoningGenome,
};
use crate::router::RouterState;
use crate::tiered_cache::TieredCachePlan;

use super::EvolutionLedger;

#[derive(Debug, Clone)]
pub struct AdaptiveState {
    pub router: RouterState,
    pub hierarchy: HierarchyState,
    pub tier_plan: TieredCachePlan,
    pub memory_retention_policy: MemoryRetentionPolicy,
    pub memory_compaction_policy: MemoryCompactionPolicy,
    pub evolution_ledger: EvolutionLedger,
    pub genome_runtime: GenomeRuntimeState,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GenomeProfileState {
    pub profile: TaskProfile,
    pub active: ReasoningGenome,
    pub previous: Option<ReasoningGenome>,
    pub generation: u64,
    pub journal_lines: Vec<String>,
}

impl GenomeProfileState {
    fn new(profile: TaskProfile) -> Self {
        Self {
            profile,
            active: ReasoningGenome::default_for_profile(profile),
            previous: None,
            generation: 0,
            journal_lines: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GenomeRuntimeState {
    pub profiles: Vec<GenomeProfileState>,
}

impl Default for GenomeRuntimeState {
    fn default() -> Self {
        Self {
            profiles: all_profiles()
                .into_iter()
                .map(GenomeProfileState::new)
                .collect(),
        }
    }
}

impl GenomeRuntimeState {
    pub fn profile(&self, profile: TaskProfile) -> &GenomeProfileState {
        self.profiles
            .iter()
            .find(|state| state.profile == profile)
            .expect("all task profiles have genome runtime state")
    }

    pub fn profile_mut(&mut self, profile: TaskProfile) -> &mut GenomeProfileState {
        self.profiles
            .iter_mut()
            .find(|state| state.profile == profile)
            .expect("all task profiles have genome runtime state")
    }

    pub fn active(&self, profile: TaskProfile) -> &ReasoningGenome {
        &self.profile(profile).active
    }

    pub fn generation(&self, profile: TaskProfile) -> u64 {
        self.profile(profile).generation
    }

    pub fn apply(
        &mut self,
        profile: TaskProfile,
        candidate: &ReasoningGenome,
        plans: &[MutationPlan],
        journal_lines: &[String],
        approval_ref: &str,
    ) -> GenomeEvolutionApplyReceipt {
        let current = self.profile(profile);
        let generation_before = current.generation;
        let genome_id_before = current.active.id.clone();
        if approval_ref.trim().is_empty() {
            return GenomeEvolutionApplyReceipt::held(
                profile,
                generation_before,
                genome_id_before,
                "approval_ref_missing",
            );
        }
        if candidate.profile != profile || candidate.genes.is_empty() || plans.is_empty() {
            return GenomeEvolutionApplyReceipt::held(
                profile,
                generation_before,
                genome_id_before,
                "candidate_or_plan_invalid",
            );
        }
        if plans
            .iter()
            .any(|plan| plan.validation_status != GeneValidationStatus::Passed)
        {
            return GenomeEvolutionApplyReceipt::held(
                profile,
                generation_before,
                genome_id_before,
                "plan_validation_not_passed",
            );
        }
        if plans
            .iter()
            .any(|plan| plan.intent == GeneScissorsIntent::Rollback)
        {
            return self.rollback(profile, journal_lines, approval_ref);
        }

        let mut next = candidate.clone();
        for plan in plans {
            if let Err(reason) = apply_plan(&mut next, plan) {
                return GenomeEvolutionApplyReceipt::held(
                    profile,
                    generation_before,
                    genome_id_before,
                    reason,
                );
            }
        }
        if !valid_genome(&next) {
            return GenomeEvolutionApplyReceipt::held(
                profile,
                generation_before,
                genome_id_before,
                "candidate_genome_failed_validation",
            );
        }

        let generation_after = generation_before.saturating_add(1);
        next.id = format!(
            "genome:{}:generation:{}",
            profile_slug(profile),
            generation_after
        );
        next.stable_anchor_id = format!(
            "genome:{}:stable:generation:{}",
            profile_slug(profile),
            generation_before
        );
        let genome_id_after = next.id.clone();
        let state = self.profile_mut(profile);
        state.previous = Some(state.active.clone());
        state.active = next;
        state.generation = generation_after;
        append_journal(&mut state.journal_lines, journal_lines);

        GenomeEvolutionApplyReceipt {
            profile,
            generation_before,
            generation_after,
            genome_id_before,
            genome_id_after,
            mutation_count: plans.len(),
            applied: true,
            rolled_back: false,
            reason: "mutation_applied".to_owned(),
        }
    }

    pub fn rollback(
        &mut self,
        profile: TaskProfile,
        journal_lines: &[String],
        approval_ref: &str,
    ) -> GenomeEvolutionApplyReceipt {
        let state = self.profile(profile);
        let generation_before = state.generation;
        let genome_id_before = state.active.id.clone();
        if approval_ref.trim().is_empty() {
            return GenomeEvolutionApplyReceipt::held(
                profile,
                generation_before,
                genome_id_before,
                "approval_ref_missing",
            );
        }
        let Some(previous) = state.previous.clone() else {
            return GenomeEvolutionApplyReceipt::held(
                profile,
                generation_before,
                genome_id_before,
                "rollback_snapshot_missing",
            );
        };

        let generation_after = generation_before.saturating_add(1);
        let genome_id_after = previous.id.clone();
        let state = self.profile_mut(profile);
        state.previous = Some(std::mem::replace(&mut state.active, previous));
        state.generation = generation_after;
        append_journal(&mut state.journal_lines, journal_lines);

        GenomeEvolutionApplyReceipt {
            profile,
            generation_before,
            generation_after,
            genome_id_before,
            genome_id_after,
            mutation_count: 1,
            applied: true,
            rolled_back: true,
            reason: "rollback_applied".to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenomeEvolutionApplyReceipt {
    pub profile: TaskProfile,
    pub generation_before: u64,
    pub generation_after: u64,
    pub genome_id_before: String,
    pub genome_id_after: String,
    pub mutation_count: usize,
    pub applied: bool,
    pub rolled_back: bool,
    pub reason: String,
}

impl GenomeEvolutionApplyReceipt {
    pub fn held(
        profile: TaskProfile,
        generation: u64,
        genome_id: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        let genome_id = genome_id.into();
        Self {
            profile,
            generation_before: generation,
            generation_after: generation,
            genome_id_before: genome_id.clone(),
            genome_id_after: genome_id,
            mutation_count: 0,
            applied: false,
            rolled_back: false,
            reason: reason.into(),
        }
    }
}

fn apply_plan(genome: &mut ReasoningGenome, plan: &MutationPlan) -> Result<(), &'static str> {
    let target = genome
        .genes
        .iter()
        .position(|gene| gene.id == plan.target_gene_id)
        .ok_or("target_gene_missing")?;
    match plan.intent {
        GeneScissorsIntent::Relabel | GeneScissorsIntent::Repair => {
            let label = plan
                .proposed_label
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .ok_or("relabel_payload_missing")?;
            let purpose = plan
                .proposed_purpose
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .ok_or("relabel_payload_missing")?;
            let gene = &mut genome.genes[target];
            gene.label = label.to_owned();
            gene.purpose = purpose.to_owned();
            gene.tags.clone_from(&plan.proposed_tags);
            gene.age = 0;
            gene.fitness = gene.fitness.max(0.60);
            gene.drift_score = gene.drift_score.min(0.20);
            gene.status = ReasoningGeneStatus::Active;
        }
        GeneScissorsIntent::Quarantine => {
            genome.genes[target].status = ReasoningGeneStatus::Quarantined;
        }
        GeneScissorsIntent::Regenerate => {
            let replacement_id = plan
                .replacement_gene_id
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .ok_or("replacement_gene_missing")?;
            let label = plan
                .proposed_label
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .ok_or("regeneration_payload_missing")?;
            let purpose = plan
                .proposed_purpose
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .ok_or("regeneration_payload_missing")?;
            if genome.genes.iter().any(|gene| gene.id == replacement_id) {
                return Err("replacement_gene_duplicate");
            }
            let kind = genome.genes[target].kind;
            genome.genes.push(
                ReasoningGene::new(replacement_id, kind, label, purpose)
                    .with_tags(plan.proposed_tags.clone()),
            );
        }
        GeneScissorsIntent::Cut => {
            genome.genes.remove(target);
        }
        GeneScissorsIntent::Rollback => return Err("rollback_requires_snapshot"),
        GeneScissorsIntent::Splice | GeneScissorsIntent::Crossover => {
            return Err("unsupported_genome_mutation_intent");
        }
    }
    Ok(())
}

fn valid_genome(genome: &ReasoningGenome) -> bool {
    !genome.id.trim().is_empty()
        && !genome.stable_anchor_id.trim().is_empty()
        && !genome.genes.is_empty()
        && genome.genes.iter().all(|gene| {
            !gene.id.trim().is_empty()
                && !gene.label.trim().is_empty()
                && !gene.purpose.trim().is_empty()
                && gene.fitness.is_finite()
                && (0.0..=1.0).contains(&gene.fitness)
                && gene.drift_score.is_finite()
                && (0.0..=1.0).contains(&gene.drift_score)
        })
        && genome.genes.iter().enumerate().all(|(index, gene)| {
            genome.genes[index + 1..]
                .iter()
                .all(|other| other.id != gene.id)
        })
}

fn append_journal(target: &mut Vec<String>, lines: &[String]) {
    target.extend(lines.iter().filter(|line| !line.trim().is_empty()).cloned());
    if target.len() > 256 {
        target.drain(..target.len() - 256);
    }
}

pub(crate) fn all_profiles() -> [TaskProfile; 4] {
    [
        TaskProfile::General,
        TaskProfile::Coding,
        TaskProfile::Writing,
        TaskProfile::LongDocument,
    ]
}

pub(crate) fn profile_slug(profile: TaskProfile) -> &'static str {
    match profile {
        TaskProfile::General => "general",
        TaskProfile::Coding => "coding",
        TaskProfile::Writing => "writing",
        TaskProfile::LongDocument => "long_document",
    }
}
