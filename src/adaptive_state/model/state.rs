use crate::hierarchy::HierarchyState;
use crate::hierarchy::TaskProfile;
use crate::kv_cache::{MemoryCompactionPolicy, MemoryRetentionPolicy};
use crate::reasoning_genome::{
    DnaChainKind, DnaGeneChain, DnaGeneEvidenceKind, DnaGeneLineage, DnaGeneRecord,
    DnaGeneSourceEvidence, GeneScissorsIntent, GeneValidationStatus, MutationPlan, ReasoningGene,
    ReasoningGeneStatus, ReasoningGenome,
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
    pub active_chain: DnaGeneChain,
    pub previous_chain: Option<DnaGeneChain>,
    pub generation: u64,
    pub journal_lines: Vec<String>,
}

impl GenomeProfileState {
    fn new(profile: TaskProfile) -> Self {
        let active = ReasoningGenome::default_for_profile(profile);
        let active_chain = initial_chain(&active);
        Self {
            profile,
            active,
            previous: None,
            active_chain,
            previous_chain: None,
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
        self.apply_with_lineage(
            profile,
            candidate,
            plans,
            journal_lines,
            approval_ref,
            "local-single-user",
            "genome-runtime",
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn apply_with_lineage(
        &mut self,
        profile: TaskProfile,
        candidate: &ReasoningGenome,
        plans: &[MutationPlan],
        journal_lines: &[String],
        approval_ref: &str,
        tenant_scope: &str,
        session_id: &str,
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
        let next_chain = match applied_chain(
            &next,
            candidate,
            plans,
            tenant_scope,
            session_id,
            generation_after,
            approval_ref,
        ) {
            Ok(chain) => chain,
            Err(reason) => {
                return GenomeEvolutionApplyReceipt::held(
                    profile,
                    generation_before,
                    genome_id_before,
                    reason,
                );
            }
        };
        let express_chain_records = next_chain.express_chain.len();
        let memory_chain_records = next_chain.memory_chain.len();
        let state = self.profile_mut(profile);
        state.previous = Some(state.active.clone());
        state.previous_chain = Some(state.active_chain.clone());
        state.active = next;
        state.active_chain = next_chain;
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
            express_chain_records,
            memory_chain_records,
            dual_chain_committed: true,
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
        let (Some(previous), Some(previous_chain)) =
            (state.previous.clone(), state.previous_chain.clone())
        else {
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
        state.previous_chain = Some(std::mem::replace(&mut state.active_chain, previous_chain));
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
            express_chain_records: state.active_chain.express_chain.len(),
            memory_chain_records: state.active_chain.memory_chain.len(),
            dual_chain_committed: true,
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
    pub express_chain_records: usize,
    pub memory_chain_records: usize,
    pub dual_chain_committed: bool,
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
            express_chain_records: 0,
            memory_chain_records: 0,
            dual_chain_committed: false,
            reason: reason.into(),
        }
    }
}

fn initial_chain(genome: &ReasoningGenome) -> DnaGeneChain {
    DnaGeneChain::preview_from_genome(
        genome,
        "local-single-user",
        "genome-runtime",
        DnaGeneSourceEvidence::new(
            DnaGeneEvidenceKind::SyntheticDefault,
            format!("initial:{}", genome.id),
            "initial profile genome",
        )
        .with_privacy_gate(),
    )
}

#[allow(clippy::too_many_arguments)]
fn applied_chain(
    genome: &ReasoningGenome,
    previous: &ReasoningGenome,
    plans: &[MutationPlan],
    tenant_scope: &str,
    session_id: &str,
    generation: u64,
    approval_ref: &str,
) -> Result<DnaGeneChain, &'static str> {
    let generation = u32::try_from(generation).unwrap_or(u32::MAX);
    let source_hash = crate::privacy_redaction::stable_redaction_digest([
        "genome-dual-chain-apply",
        approval_ref,
        genome.id.as_str(),
    ]);
    let source_evidence = DnaGeneSourceEvidence::new(
        DnaGeneEvidenceKind::OperatorApproved,
        source_hash,
        "operator-approved genome mutation",
    )
    .with_privacy_gate();
    let mut chain = DnaGeneChain::preview_from_genome(
        genome,
        tenant_scope,
        session_id,
        source_evidence.clone(),
    );
    chain.read_only = false;
    chain.write_allowed = true;
    for record in &mut chain.express_chain {
        record.lineage.generation = generation;
        record.admission_write_authorized = true;
        record.applied = true;
    }

    for plan in plans {
        let record_gene = plan
            .replacement_gene_id
            .as_deref()
            .and_then(|replacement| genome.genes.iter().find(|gene| gene.id == replacement))
            .or_else(|| {
                genome
                    .genes
                    .iter()
                    .find(|gene| gene.id == plan.target_gene_id)
            })
            .or_else(|| {
                previous
                    .genes
                    .iter()
                    .find(|gene| gene.id == plan.target_gene_id)
            })
            .ok_or("dual_chain_mutation_gene_missing")?;
        let mut record = DnaGeneRecord::from_reasoning_gene(
            DnaChainKind::Memory,
            genome.profile,
            plan.rollback_anchor_id.clone(),
            DnaGeneLineage::new(tenant_scope, session_id)
                .with_parent(plan.target_gene_id.clone())
                .with_inheritance(plan.rollback_anchor_id.clone(), generation),
            DnaGeneSourceEvidence::new(
                DnaGeneEvidenceKind::OperatorApproved,
                crate::privacy_redaction::stable_redaction_digest([
                    "genome-memory-chain-apply",
                    approval_ref,
                    plan.id.as_str(),
                ]),
                format!("approved {} mutation audit record", plan.intent.as_str()),
            )
            .with_privacy_gate(),
            record_gene,
        );
        record.admission_write_authorized = true;
        record.applied = true;
        chain.push_memory_record(record);
    }
    chain
        .validate()
        .map_err(|_| "dual_chain_validation_failed")?;
    Ok(chain)
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
        GeneScissorsIntent::Splice => {
            let replacement = replacement_gene_from_plan(genome, plan, target)?;
            genome.genes.insert(target.saturating_add(1), replacement);
        }
        GeneScissorsIntent::Crossover => {
            let replacement = crossover_gene_from_plan(genome, plan)?;
            genome.genes.insert(target.saturating_add(1), replacement);
        }
    }
    Ok(())
}

fn replacement_gene_from_plan(
    genome: &ReasoningGenome,
    plan: &MutationPlan,
    target: usize,
) -> Result<ReasoningGene, &'static str> {
    if plan.source_gene_ids.is_empty() || !plan.has_source_evidence() {
        return Err("splice_source_evidence_missing");
    }
    let replacement_id = replacement_id(genome, plan)?;
    let (label, purpose) = replacement_payload(plan, "splice_payload_missing")?;
    let source = &genome.genes[target];
    let mut replacement = ReasoningGene::new(replacement_id, source.kind, label, purpose)
        .with_tags(
            plan.proposed_tags
                .iter()
                .cloned()
                .chain(["splice".to_owned()]),
        );
    replacement.fitness = source.fitness.max(0.60);
    replacement.drift_score = source.drift_score.min(0.20);
    replacement.status = ReasoningGeneStatus::Active;
    Ok(replacement)
}

fn crossover_gene_from_plan(
    genome: &ReasoningGenome,
    plan: &MutationPlan,
) -> Result<ReasoningGene, &'static str> {
    let mut source_indexes = Vec::new();
    for source_id in &plan.source_gene_ids {
        let index = genome
            .genes
            .iter()
            .position(|gene| gene.id == *source_id)
            .ok_or("crossover_source_gene_missing")?;
        if !source_indexes.contains(&index) {
            source_indexes.push(index);
        }
    }
    if source_indexes.len() < 2 || !plan.has_source_evidence() {
        return Err("crossover_sources_missing");
    }
    let first = &genome.genes[source_indexes[0]];
    if source_indexes.iter().any(|index| {
        let gene = &genome.genes[*index];
        gene.kind != first.kind
            || gene.derived_status() != ReasoningGeneStatus::Active
            || gene.fitness < 0.60
    }) {
        return Err("crossover_sources_incompatible");
    }
    let replacement_id = replacement_id(genome, plan)?;
    let (label, purpose) = replacement_payload(plan, "crossover_payload_missing")?;
    let source_count = source_indexes.len() as f32;
    let average_fitness = source_indexes
        .iter()
        .map(|index| genome.genes[*index].fitness)
        .sum::<f32>()
        / source_count;
    let average_drift = source_indexes
        .iter()
        .map(|index| genome.genes[*index].drift_score)
        .sum::<f32>()
        / source_count;
    let mut replacement = ReasoningGene::new(replacement_id, first.kind, label, purpose).with_tags(
        plan.proposed_tags
            .iter()
            .cloned()
            .chain(["crossover".to_owned()]),
    );
    replacement.fitness = average_fitness.clamp(0.0, 1.0);
    replacement.drift_score = average_drift.clamp(0.0, 1.0);
    replacement.status = ReasoningGeneStatus::Active;
    Ok(replacement)
}

fn replacement_id<'a>(
    genome: &ReasoningGenome,
    plan: &'a MutationPlan,
) -> Result<&'a str, &'static str> {
    let replacement_id = plan
        .replacement_gene_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or("replacement_gene_missing")?;
    if genome.genes.iter().any(|gene| gene.id == replacement_id) {
        return Err("replacement_gene_duplicate");
    }
    Ok(replacement_id)
}

fn replacement_payload<'a>(
    plan: &'a MutationPlan,
    error: &'static str,
) -> Result<(&'a str, &'a str), &'static str> {
    let label = plan
        .proposed_label
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or(error)?;
    let purpose = plan
        .proposed_purpose
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or(error)?;
    Ok((label, purpose))
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
