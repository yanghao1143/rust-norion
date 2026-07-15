use crate::hierarchy::HierarchyState;
use crate::hierarchy::TaskProfile;
use crate::kv_cache::{
    MemoryCompactionPolicy, MemoryResidencyPolicy, MemoryResidencyState, MemoryRetentionPolicy,
};
use crate::reasoning_genome::{
    DnaChainKind, DnaEvolutionCandidateDecision, DnaGeneChain, DnaGeneEvidenceKind, DnaGeneLineage,
    DnaGeneRecord, DnaGeneSourceEvidence, GeneLifecycleSourceEvidence, GeneLifecycleSourceKind,
    GeneScissorsIntent, GeneScissorsTransaction, GeneValidationStatus, MutationPlan, ReasoningGene,
    ReasoningGeneKind, ReasoningGeneStatus, ReasoningGenome,
};
use crate::router::RouterState;
use crate::tiered_cache::TieredCachePlan;

use super::EvolutionLedger;

pub const GENOME_PERSISTED_GENE_CAPACITY: usize = 16;
pub const GENOME_EXPRESSED_GENE_CAPACITY: usize = 8;
const MAX_RETIRED_GENE_FINGERPRINTS: usize = 16;
const NEW_EVIDENCE_SCORE_BONUS_MILLI: u16 = 50;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneUsageRecord {
    pub gene_id: String,
    pub residency: MemoryResidencyState,
    pub opportunities: u64,
    pub hits: u64,
    pub failures: u64,
    pub last_used_step: u64,
    pub consumed_evidence_digest: String,
}

impl GeneUsageRecord {
    fn warm(gene_id: impl Into<String>) -> Self {
        Self {
            gene_id: gene_id.into(),
            residency: MemoryResidencyState::Warm,
            opportunities: 0,
            hits: 0,
            failures: 0,
            last_used_step: 0,
            consumed_evidence_digest: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenomeGeneResidency {
    pub step: u64,
    pub records: Vec<GeneUsageRecord>,
    pub last_observation_sequence: u64,
    pub last_transition_reason: String,
}

impl GenomeGeneResidency {
    pub fn for_genome(genome: &ReasoningGenome) -> Self {
        let mut residency = Self {
            step: 0,
            records: genome
                .genes
                .iter()
                .map(|gene| {
                    let mut record = GeneUsageRecord::warm(gene.id.clone());
                    if gene_health_blocks_expression(gene) {
                        record.residency = MemoryResidencyState::Quarantined;
                    }
                    record
                })
                .collect(),
            last_observation_sequence: 0,
            last_transition_reason: "legacy_default_warm".to_owned(),
        };
        let mut squeezed = false;
        while resident_count(&residency) > GENOME_EXPRESSED_GENE_CAPACITY {
            let Some(victim) = weakest_resident_gene(genome, &residency, None) else {
                break;
            };
            let victim_id = genome.genes[victim].id.clone();
            residency
                .record_mut(&victim_id)
                .expect("resident gene has usage record")
                .residency = MemoryResidencyState::Cold;
            squeezed = true;
        }
        if squeezed {
            residency.last_transition_reason = "legacy_capacity_squeezed".to_owned();
        }
        residency
    }

    pub(crate) fn normalized_for_genome(&self, genome: &ReasoningGenome) -> Self {
        let mut normalized = self.clone();
        let retired = normalized
            .records
            .iter()
            .filter(|record| record.residency == MemoryResidencyState::Retired)
            .cloned()
            .collect::<Vec<_>>();
        normalized.records = genome
            .genes
            .iter()
            .map(|gene| {
                let mut record = self
                    .records
                    .iter()
                    .find(|record| record.gene_id == gene.id)
                    .cloned()
                    .unwrap_or_else(|| GeneUsageRecord::warm(gene.id.clone()));
                if gene_health_blocks_expression(gene) {
                    record.residency = MemoryResidencyState::Quarantined;
                } else if gene.derived_status() == ReasoningGeneStatus::Aging
                    && record.residency == MemoryResidencyState::Hot
                {
                    record.residency = MemoryResidencyState::Warm;
                }
                record
            })
            .chain(retired)
            .collect();
        normalized.trim_retired_fingerprints();
        normalized
    }

    fn trim_retired_fingerprints(&mut self) {
        let retired_count = self
            .records
            .iter()
            .filter(|record| record.residency == MemoryResidencyState::Retired)
            .count();
        let mut excess = retired_count.saturating_sub(MAX_RETIRED_GENE_FINGERPRINTS);
        self.records.retain(|record| {
            if excess > 0 && record.residency == MemoryResidencyState::Retired {
                excess -= 1;
                false
            } else {
                true
            }
        });
    }

    fn record(&self, gene_id: &str) -> Option<&GeneUsageRecord> {
        self.records.iter().find(|record| record.gene_id == gene_id)
    }

    fn record_mut(&mut self, gene_id: &str) -> Option<&mut GeneUsageRecord> {
        self.records
            .iter_mut()
            .find(|record| record.gene_id == gene_id)
    }

    fn cold_readmission_decision(
        &self,
        gene_id: &str,
        evidence_digest: &str,
        score_milli: u16,
    ) -> DnaEvolutionCandidateDecision {
        if evidence_digest.trim().is_empty() || !evidence_digest.starts_with("redaction-digest:") {
            return DnaEvolutionCandidateDecision::Reject;
        }
        let Some(record) = self.record(gene_id) else {
            return DnaEvolutionCandidateDecision::Reject;
        };
        if record.residency != MemoryResidencyState::Cold
            || record.consumed_evidence_digest == evidence_digest
        {
            return DnaEvolutionCandidateDecision::Hold;
        }
        let policy = MemoryResidencyPolicy::default();
        let cooled = self.step.saturating_sub(record.last_used_step) >= policy.stale_after_steps;
        if cooled && score_milli >= unit_milli(policy.warm_score_threshold) {
            DnaEvolutionCandidateDecision::CandidatePreview
        } else {
            DnaEvolutionCandidateDecision::Hold
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneResidencyReport {
    pub hot: usize,
    pub warm: usize,
    pub cold: usize,
    pub quarantined: usize,
    pub retired: usize,
    pub borrowed_expression_count: usize,
    pub persisted_gene_count: usize,
    pub persisted_capacity: usize,
    pub expressed_capacity: usize,
    pub last_transition_reason: String,
}

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
    pub gene_residency: GenomeGeneResidency,
    pub previous_gene_residency: Option<GenomeGeneResidency>,
    pub generation: u64,
    pub journal_lines: Vec<String>,
}

impl GenomeProfileState {
    fn new(profile: TaskProfile) -> Self {
        let active = ReasoningGenome::default_for_profile(profile);
        let active_chain = initial_chain(&active);
        let gene_residency = GenomeGeneResidency::for_genome(&active);
        Self {
            profile,
            active,
            previous: None,
            active_chain,
            previous_chain: None,
            gene_residency,
            previous_gene_residency: None,
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
    pub fn profiles(&self) -> &[GenomeProfileState] {
        &self.profiles
    }

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

    pub fn residency_revision(&self, profile: TaskProfile) -> u64 {
        self.profile(profile).gene_residency.step
    }

    pub fn borrowed_gene_ids(&self, profile: TaskProfile) -> Vec<String> {
        let state = self.profile(profile);
        borrowed_gene_ids(&state.active, &state.gene_residency)
    }

    pub fn gene_residency_report(&self, profile: TaskProfile) -> GeneResidencyReport {
        let state = self.profile(profile);
        gene_residency_report(&state.active, &state.gene_residency)
    }

    pub fn cold_gene_readmission_decision(
        &self,
        profile: TaskProfile,
        gene_id: &str,
        evidence_digest: &str,
        score_milli: u16,
    ) -> DnaEvolutionCandidateDecision {
        self.profile(profile)
            .gene_residency
            .cold_readmission_decision(gene_id, evidence_digest, score_milli)
    }

    pub(crate) fn cold_gene_plan_decision(
        &self,
        profile: TaskProfile,
        candidate: &ReasoningGenome,
        plan: &MutationPlan,
    ) -> Option<DnaEvolutionCandidateDecision> {
        if candidate.profile != profile
            || !matches!(
                plan.intent,
                GeneScissorsIntent::Relabel | GeneScissorsIntent::Repair
            )
        {
            return None;
        }
        let state = self.profile(profile);
        let record = state.gene_residency.record(&plan.target_gene_id)?;
        if record.residency != MemoryResidencyState::Cold {
            return None;
        }
        let gene = candidate
            .genes
            .iter()
            .find(|gene| gene.id == plan.target_gene_id)?;
        Some(state.gene_residency.cold_readmission_decision(
            &gene.id,
            &gene_transition_evidence_digest(profile, &candidate.stable_anchor_id, plan),
            gene_phase_score_milli(gene, record, NEW_EVIDENCE_SCORE_BONUS_MILLI),
        ))
    }

    pub fn record_gene_expression(
        &mut self,
        profile: TaskProfile,
        borrowed_gene_ids: &[String],
        observation_sequence: u64,
        success: bool,
    ) -> GeneResidencyReport {
        let state = self.profile_mut(profile);
        if observation_sequence == 0
            || observation_sequence <= state.gene_residency.last_observation_sequence
        {
            return gene_residency_report(&state.active, &state.gene_residency);
        }
        let mut residency = state.gene_residency.normalized_for_genome(&state.active);
        residency.step = residency.step.saturating_add(1);
        let step = residency.step;
        let policy = MemoryResidencyPolicy::default();
        let mut transition_reason = residency.last_transition_reason.clone();
        for gene in &state.active.genes {
            let Some(record) = residency.record_mut(&gene.id) else {
                continue;
            };
            record.opportunities = record.opportunities.saturating_add(1);
            if borrowed_gene_ids.iter().any(|gene_id| gene_id == &gene.id) {
                if success {
                    record.hits = record.hits.saturating_add(1);
                } else {
                    record.failures = record.failures.saturating_add(1);
                }
                record.last_used_step = step;
            }

            let previous = record.residency;
            record.residency = next_residency(gene, record, &policy);
            if record.residency != previous {
                transition_reason = match record.residency {
                    MemoryResidencyState::Hot => "usage_promoted_hot",
                    MemoryResidencyState::Warm => "usage_cooled_warm",
                    MemoryResidencyState::Cold => "usage_squeezed_cold",
                    MemoryResidencyState::Quarantined => "health_quarantined",
                    MemoryResidencyState::Retired => "payload_retired",
                }
                .to_owned();
            }
        }
        residency.last_transition_reason = transition_reason;
        residency.last_observation_sequence = observation_sequence;
        state.gene_residency = residency;
        gene_residency_report(&state.active, &state.gene_residency)
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

    pub fn bounded_mutation_plans(
        &self,
        profile: TaskProfile,
        candidate: &ReasoningGenome,
        plans: &[MutationPlan],
    ) -> Result<Vec<MutationPlan>, &'static str> {
        if candidate.profile != profile
            || candidate.genes.is_empty()
            || candidate.genes.len() > GENOME_PERSISTED_GENE_CAPACITY
        {
            return Err("candidate_or_plan_invalid");
        }
        if plans
            .iter()
            .any(|plan| plan.intent == GeneScissorsIntent::Rollback)
        {
            return Ok(plans.to_vec());
        }
        let state = self.profile(profile);
        let mut next = candidate.clone();
        for plan in plans {
            apply_plan(&mut next, plan)?;
        }
        let (_, forced_victims) = transition_gene_residency(
            profile,
            candidate,
            &mut next,
            &state.gene_residency,
            plans,
            true,
            false,
        )?;
        let mut bounded = plans.to_vec();
        if !forced_victims.is_empty() {
            for (plan_id, victim_id) in forced_victims {
                let plan = bounded
                    .iter_mut()
                    .find(|plan| plan.id == plan_id)
                    .ok_or("forced_expression_victim_plan_missing")?;
                let victim = next
                    .genes
                    .iter()
                    .find(|gene| gene.id == victim_id)
                    .ok_or("forced_expression_victim_missing")?;
                bind_forced_expression_victim(plan, victim);
            }
            next = candidate.clone();
            for plan in &bounded {
                apply_plan(&mut next, plan)?;
            }
        }
        let (mut residency, _) = transition_gene_residency(
            profile,
            candidate,
            &mut next,
            &state.gene_residency,
            &bounded,
            true,
            true,
        )?;
        while next.genes.len() > GENOME_PERSISTED_GENE_CAPACITY {
            let victim = weakest_cold_gene(&next, &residency)
                .ok_or("gene_persisted_capacity_has_no_cold_victim")?;
            let victim_gene = next.genes[victim].clone();
            let cut = MutationPlan::preview(
                format!("mutation:{}:capacity-cut", victim_gene.id),
                GeneScissorsIntent::Cut,
                victim_gene.id.clone(),
                "persisted DNA capacity requires an explicit forced phase transition",
                "retire one non-protected cold payload while preserving its bounded fingerprint",
                candidate.stable_anchor_id.clone(),
            )
            .with_sources([victim_gene.id.clone()])
            .with_source_evidence([GeneLifecycleSourceEvidence::health_metadata(&victim_gene)])
            .with_validation_status(GeneValidationStatus::Passed);
            apply_plan(&mut next, &cut)?;
            retire_gene_usage(&mut residency, &victim_gene.id);
            bounded.push(cut);
        }
        Ok(bounded)
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
        let current_gene_residency = current.gene_residency.clone();
        if approval_ref.trim().is_empty() {
            return GenomeEvolutionApplyReceipt::held(
                profile,
                generation_before,
                genome_id_before,
                "approval_ref_missing",
            );
        }
        if candidate.id != current.active.id
            || candidate.stable_anchor_id != current.active.stable_anchor_id
        {
            return GenomeEvolutionApplyReceipt::held(
                profile,
                generation_before,
                genome_id_before,
                "candidate_genome_stale",
            );
        }
        if candidate.profile != profile
            || candidate.genes.is_empty()
            || candidate.genes.len() > GENOME_PERSISTED_GENE_CAPACITY
            || plans.is_empty()
        {
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
        let next_gene_residency = match transition_gene_residency(
            profile,
            candidate,
            &mut next,
            &current_gene_residency,
            plans,
            false,
            true,
        ) {
            Ok((residency, _)) => residency,
            Err(reason) => {
                return GenomeEvolutionApplyReceipt::held(
                    profile,
                    generation_before,
                    genome_id_before,
                    reason,
                );
            }
        };
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
        state.previous_gene_residency = Some(state.gene_residency.clone());
        state.active = next;
        state.active_chain = next_chain;
        state.gene_residency = next_gene_residency;
        state.generation = generation_after;
        append_journal(&mut state.journal_lines, journal_lines);
        state.journal_lines.push(format!(
            "gene_residency step={} reason={} persisted={} borrowed={}",
            state.gene_residency.step,
            state.gene_residency.last_transition_reason,
            state.active.genes.len(),
            borrowed_gene_ids(&state.active, &state.gene_residency).len()
        ));
        if state.journal_lines.len() > 256 {
            state.journal_lines.drain(..state.journal_lines.len() - 256);
        }

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
        let (Some(previous), Some(previous_chain), Some(previous_gene_residency)) = (
            state.previous.clone(),
            state.previous_chain.clone(),
            state.previous_gene_residency.clone(),
        ) else {
            return GenomeEvolutionApplyReceipt::held(
                profile,
                generation_before,
                genome_id_before,
                "rollback_snapshot_missing",
            );
        };

        let generation_after = generation_before.saturating_add(1);
        let residency_revision_after = state
            .gene_residency
            .step
            .max(previous_gene_residency.step)
            .saturating_add(1);
        let observation_sequence_after = state
            .gene_residency
            .last_observation_sequence
            .max(previous_gene_residency.last_observation_sequence);
        let genome_id_after = previous.id.clone();
        let state = self.profile_mut(profile);
        state.previous = Some(std::mem::replace(&mut state.active, previous));
        state.previous_chain = Some(std::mem::replace(&mut state.active_chain, previous_chain));
        state.previous_gene_residency = Some(std::mem::replace(
            &mut state.gene_residency,
            previous_gene_residency,
        ));
        state.gene_residency.step = residency_revision_after;
        state.gene_residency.last_observation_sequence = observation_sequence_after;
        state.gene_residency.last_transition_reason = "rollback_restored_snapshot".to_owned();
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
    for source_id in plan
        .source_gene_ids
        .iter()
        .filter(|source_id| !forced_expression_auxiliary_source(plan, source_id))
    {
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

fn transition_gene_residency(
    profile: TaskProfile,
    candidate: &ReasoningGenome,
    next: &mut ReasoningGenome,
    current_residency: &GenomeGeneResidency,
    plans: &[MutationPlan],
    allow_persisted_overflow: bool,
    enforce_forced_victim_binding: bool,
) -> Result<(GenomeGeneResidency, Vec<(String, String)>), &'static str> {
    let mut residency = current_residency.normalized_for_genome(candidate);
    let mut forced_victims = Vec::new();
    residency.step = residency.step.saturating_add(1);
    let transition_evidence_digests = plans
        .iter()
        .map(|plan| gene_transition_evidence_digest(profile, &candidate.stable_anchor_id, plan))
        .collect::<Vec<_>>();
    if transition_evidence_digests.iter().any(|evidence_digest| {
        residency.records.iter().any(|record| {
            !record.consumed_evidence_digest.is_empty()
                && record.consumed_evidence_digest == evidence_digest.as_str()
        })
    }) {
        return Err("duplicate_gene_transition_evidence");
    }

    let removed_gene_ids = residency
        .records
        .iter()
        .filter(|record| {
            record.residency != MemoryResidencyState::Retired
                && next.genes.iter().all(|gene| gene.id != record.gene_id)
        })
        .map(|record| record.gene_id.clone())
        .collect::<Vec<_>>();
    let retired_payload = !removed_gene_ids.is_empty();
    for gene_id in removed_gene_ids {
        retire_gene_usage(&mut residency, &gene_id);
    }

    let planned_replacements = plans
        .iter()
        .filter_map(|plan| plan.replacement_gene_id.as_deref())
        .collect::<Vec<_>>();
    for gene in &next.genes {
        if residency.record(&gene.id).is_none() {
            let mut record = GeneUsageRecord::warm(gene.id.clone());
            if planned_replacements.contains(&gene.id.as_str()) {
                record.residency = MemoryResidencyState::Cold;
            }
            if gene_health_blocks_expression(gene) {
                record.residency = MemoryResidencyState::Quarantined;
            }
            residency.records.push(record);
        }
    }

    for gene in &next.genes {
        if gene_health_blocks_expression(gene) {
            if let Some(record) = residency.record_mut(&gene.id) {
                record.residency = MemoryResidencyState::Quarantined;
            }
        }
    }

    let mut transition_reason = if retired_payload {
        "payload_retired"
    } else {
        "mutation_preserved_residency"
    };
    if resident_count(&residency) > GENOME_EXPRESSED_GENE_CAPACITY {
        return Err("gene_expression_capacity_state_invalid");
    }

    for (plan, evidence_digest) in plans.iter().zip(&transition_evidence_digests) {
        let replacement_candidate = plan.replacement_gene_id.as_deref().filter(|gene_id| {
            candidate.genes.iter().all(|gene| gene.id != **gene_id)
                && next.genes.iter().any(|gene| gene.id == **gene_id)
        });
        let target_candidate = matches!(
            plan.intent,
            GeneScissorsIntent::Relabel | GeneScissorsIntent::Repair
        )
        .then_some(plan.target_gene_id.as_str())
        .filter(|gene_id| {
            residency.record(gene_id).is_some_and(|record| {
                matches!(
                    record.residency,
                    MemoryResidencyState::Cold | MemoryResidencyState::Quarantined
                )
            })
        });
        let admission_candidate = replacement_candidate.or(target_candidate);

        if let Some(gene_id) = admission_candidate {
            let residents_before = residency
                .records
                .iter()
                .filter(|record| {
                    matches!(
                        record.residency,
                        MemoryResidencyState::Hot | MemoryResidencyState::Warm
                    )
                })
                .map(|record| record.gene_id.clone())
                .collect::<Vec<_>>();
            let gene = next
                .genes
                .iter()
                .find(|gene| gene.id == gene_id)
                .ok_or("gene_residency_candidate_missing")?;
            let record = residency
                .record(gene_id)
                .ok_or("gene_residency_candidate_record_missing")?;
            if record.residency == MemoryResidencyState::Cold
                && target_candidate == Some(gene_id)
                && residency.cold_readmission_decision(
                    gene_id,
                    evidence_digest,
                    gene_phase_score_milli(gene, record, NEW_EVIDENCE_SCORE_BONUS_MILLI),
                ) != DnaEvolutionCandidateDecision::CandidatePreview
            {
                return Err("cold_gene_readmission_not_ready");
            }
            admit_gene_candidate(next, &mut residency, gene_id, evidence_digest)?;
            let plan_victims = residents_before
                .into_iter()
                .filter(|resident_id| {
                    residency
                        .record(resident_id)
                        .is_some_and(|record| record.residency == MemoryResidencyState::Cold)
                })
                .collect::<Vec<_>>();
            for victim_id in &plan_victims {
                if enforce_forced_victim_binding
                    && !forced_expression_victim_is_bound(plan, victim_id)
                {
                    return Err("forced_expression_victim_not_bound");
                }
                forced_victims.push((plan.id.clone(), victim_id.clone()));
            }
            transition_reason = if plan_victims.is_empty() {
                "candidate_admitted_warm"
            } else {
                "forced_admission_replaced_weakest"
            };
        } else if let Some(record) = residency.record_mut(&plan.target_gene_id) {
            record.consumed_evidence_digest = evidence_digest.clone();
        }
    }

    if !allow_persisted_overflow && next.genes.len() > GENOME_PERSISTED_GENE_CAPACITY {
        return Err("gene_persisted_capacity_requires_explicit_cut");
    }

    residency.last_transition_reason = transition_reason.to_owned();
    residency = residency.normalized_for_genome(next);
    Ok((residency, forced_victims))
}

fn bind_forced_expression_victim(plan: &mut MutationPlan, victim: &ReasoningGene) {
    let appended_source = !plan.source_gene_ids.contains(&victim.id);
    if appended_source {
        plan.source_gene_ids.push(victim.id.clone());
    }
    plan.source_evidence.push(GeneLifecycleSourceEvidence::new(
        GeneLifecycleSourceKind::HealthMetadata,
        victim.id.clone(),
        format!(
            "forced_expression_demote={} age={} fitness={:.3} drift={:.3}",
            if appended_source { "aux" } else { "existing" },
            victim.age,
            victim.fitness,
            victim.drift_score
        ),
    ));
}

fn forced_expression_victim_is_bound(plan: &MutationPlan, victim_id: &str) -> bool {
    plan.source_gene_ids
        .iter()
        .any(|source| source == victim_id)
        && plan.source_evidence.iter().any(|evidence| {
            evidence.source_id == victim_id
                && evidence.summary.starts_with("forced_expression_demote=")
        })
}

fn forced_expression_auxiliary_source(plan: &MutationPlan, source_id: &str) -> bool {
    plan.source_evidence.iter().any(|evidence| {
        evidence.source_id == source_id
            && evidence.summary.starts_with("forced_expression_demote=aux")
    })
}

fn gene_transition_evidence_digest(
    profile: TaskProfile,
    stable_anchor_id: &str,
    plan: &MutationPlan,
) -> String {
    if matches!(
        plan.intent,
        GeneScissorsIntent::Relabel | GeneScissorsIntent::Repair
    ) && let Some(evidence) = plan.source_evidence.iter().find(|evidence| {
        evidence.source_id.starts_with("redaction-digest:")
            && evidence.summary == "tenant-scoped request evidence matched this dormant gene"
    }) {
        return evidence.source_id.clone();
    }
    let mut transition_plan = plan.clone();
    transition_plan
        .source_evidence
        .retain(|evidence| !evidence.summary.starts_with("forced_expression_demote="));
    GeneScissorsTransaction::from_plan(profile, stable_anchor_id, &transition_plan).evidence_digest
}

fn admit_gene_candidate(
    genome: &ReasoningGenome,
    residency: &mut GenomeGeneResidency,
    candidate_id: &str,
    evidence_digest: &str,
) -> Result<(), &'static str> {
    let candidate = genome
        .genes
        .iter()
        .find(|gene| gene.id == candidate_id)
        .ok_or("gene_residency_candidate_missing")?;
    if gene_health_blocks_expression(candidate) {
        return Err("gene_residency_candidate_health_blocked");
    }
    if resident_count(residency) >= GENOME_EXPRESSED_GENE_CAPACITY {
        let victim = weakest_resident_gene(genome, residency, Some(candidate_id))
            .ok_or("gene_expression_capacity_has_no_eligible_victim")?;
        let victim_gene = &genome.genes[victim];
        let candidate_record = residency
            .record(candidate_id)
            .ok_or("gene_residency_candidate_record_missing")?;
        let victim_record = residency
            .record(&victim_gene.id)
            .ok_or("gene_residency_victim_record_missing")?;
        let candidate_score =
            gene_phase_score_milli(candidate, candidate_record, NEW_EVIDENCE_SCORE_BONUS_MILLI);
        let victim_score = gene_phase_score_milli(victim_gene, victim_record, 0);
        if candidate_score <= victim_score {
            return Err("gene_candidate_did_not_beat_weakest_resident");
        }
        residency
            .record_mut(&victim_gene.id)
            .expect("victim record exists")
            .residency = MemoryResidencyState::Cold;
    }
    let candidate_record = residency
        .record_mut(candidate_id)
        .ok_or("gene_residency_candidate_record_missing")?;
    candidate_record.residency = MemoryResidencyState::Warm;
    candidate_record.consumed_evidence_digest = evidence_digest.to_owned();
    Ok(())
}

fn retire_gene_usage(residency: &mut GenomeGeneResidency, gene_id: &str) {
    let Some(index) = residency
        .records
        .iter()
        .position(|record| record.gene_id == gene_id)
    else {
        return;
    };
    let mut record = residency.records.remove(index);
    let step = residency.step.to_string();
    record.gene_id = format!(
        "retired:{}",
        crate::privacy_redaction::stable_redaction_digest([
            "retired-gene-fingerprint",
            gene_id,
            step.as_str(),
        ])
    );
    record.residency = MemoryResidencyState::Retired;
    record.consumed_evidence_digest = crate::privacy_redaction::stable_redaction_digest([
        "retired-gene-evidence",
        gene_id,
        step.as_str(),
    ]);
    residency.records.push(record);
    residency.trim_retired_fingerprints();
}

fn borrowed_gene_ids(genome: &ReasoningGenome, residency: &GenomeGeneResidency) -> Vec<String> {
    let normalized = residency.normalized_for_genome(genome);
    let mut candidates = genome
        .genes
        .iter()
        .enumerate()
        .filter(|(_, gene)| !gene_health_blocks_expression(gene))
        .filter_map(|(index, gene)| {
            normalized.record(&gene.id).and_then(|record| {
                matches!(
                    record.residency,
                    MemoryResidencyState::Hot | MemoryResidencyState::Warm
                )
                .then(|| {
                    (
                        index,
                        gene.kind == ReasoningGeneKind::Safety,
                        record.residency == MemoryResidencyState::Hot,
                        gene_phase_score_milli(gene, record, 0),
                        gene.id.as_str(),
                    )
                })
            })
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        right
            .1
            .cmp(&left.1)
            .then_with(|| right.2.cmp(&left.2))
            .then_with(|| right.3.cmp(&left.3))
            .then_with(|| left.4.cmp(right.4))
    });
    let mut selected_indexes = candidates
        .into_iter()
        .take(GENOME_EXPRESSED_GENE_CAPACITY)
        .map(|candidate| candidate.0)
        .collect::<Vec<_>>();
    selected_indexes.sort_unstable();
    selected_indexes
        .into_iter()
        .map(|index| genome.genes[index].id.clone())
        .collect()
}

fn weakest_resident_gene(
    genome: &ReasoningGenome,
    residency: &GenomeGeneResidency,
    excluded_gene_id: Option<&str>,
) -> Option<usize> {
    genome
        .genes
        .iter()
        .enumerate()
        .filter(|(_, gene)| !gene_is_protected(gene))
        .filter(|(_, gene)| excluded_gene_id != Some(gene.id.as_str()))
        .filter_map(|(index, gene)| {
            residency.record(&gene.id).and_then(|record| {
                matches!(
                    record.residency,
                    MemoryResidencyState::Hot | MemoryResidencyState::Warm
                )
                .then(|| (index, gene_phase_score_milli(gene, record, 0), &gene.id))
            })
        })
        .min_by(|left, right| left.1.cmp(&right.1).then_with(|| left.2.cmp(right.2)))
        .map(|candidate| candidate.0)
}

fn weakest_cold_gene(genome: &ReasoningGenome, residency: &GenomeGeneResidency) -> Option<usize> {
    genome
        .genes
        .iter()
        .enumerate()
        .filter(|(_, gene)| !gene_is_protected(gene))
        .filter_map(|(index, gene)| {
            residency.record(&gene.id).and_then(|record| {
                (record.residency == MemoryResidencyState::Cold)
                    .then(|| (index, gene_phase_score_milli(gene, record, 0), &gene.id))
            })
        })
        .min_by(|left, right| left.1.cmp(&right.1).then_with(|| left.2.cmp(right.2)))
        .map(|candidate| candidate.0)
}

fn gene_residency_report(
    genome: &ReasoningGenome,
    residency: &GenomeGeneResidency,
) -> GeneResidencyReport {
    let normalized = residency.normalized_for_genome(genome);
    let count = |state| {
        normalized
            .records
            .iter()
            .filter(|record| record.residency == state)
            .count()
    };
    GeneResidencyReport {
        hot: count(MemoryResidencyState::Hot),
        warm: count(MemoryResidencyState::Warm),
        cold: count(MemoryResidencyState::Cold),
        quarantined: count(MemoryResidencyState::Quarantined),
        retired: count(MemoryResidencyState::Retired),
        borrowed_expression_count: borrowed_gene_ids(genome, &normalized).len(),
        persisted_gene_count: genome.genes.len(),
        persisted_capacity: GENOME_PERSISTED_GENE_CAPACITY,
        expressed_capacity: GENOME_EXPRESSED_GENE_CAPACITY,
        last_transition_reason: normalized.last_transition_reason,
    }
}

fn resident_count(residency: &GenomeGeneResidency) -> usize {
    residency
        .records
        .iter()
        .filter(|record| {
            matches!(
                record.residency,
                MemoryResidencyState::Hot | MemoryResidencyState::Warm
            )
        })
        .count()
}

fn next_residency(
    gene: &ReasoningGene,
    record: &GeneUsageRecord,
    policy: &MemoryResidencyPolicy,
) -> MemoryResidencyState {
    if gene_health_blocks_expression(gene) {
        return MemoryResidencyState::Quarantined;
    }
    if matches!(
        record.residency,
        MemoryResidencyState::Cold
            | MemoryResidencyState::Quarantined
            | MemoryResidencyState::Retired
    ) {
        return record.residency;
    }
    let score = gene_phase_score_milli(gene, record, 0);
    let hot = unit_milli(policy.hot_score_threshold);
    let warm = unit_milli(policy.warm_score_threshold);
    let cold = unit_milli(policy.cold_score_threshold);
    if gene.derived_status() == ReasoningGeneStatus::Aging {
        return if score >= cold {
            MemoryResidencyState::Warm
        } else {
            MemoryResidencyState::Cold
        };
    }
    match record.residency {
        MemoryResidencyState::Hot if score >= warm => MemoryResidencyState::Hot,
        MemoryResidencyState::Hot if score >= cold => MemoryResidencyState::Warm,
        MemoryResidencyState::Hot => MemoryResidencyState::Cold,
        MemoryResidencyState::Warm if score >= hot => MemoryResidencyState::Hot,
        MemoryResidencyState::Warm if score >= cold => MemoryResidencyState::Warm,
        MemoryResidencyState::Warm => MemoryResidencyState::Cold,
        state => state,
    }
}

fn gene_phase_score_milli(
    gene: &ReasoningGene,
    record: &GeneUsageRecord,
    evidence_bonus_milli: u16,
) -> u16 {
    let trust = u64::from(unit_milli(gene.trust_score()));
    let heat = if record.opportunities == 0 {
        500
    } else {
        record.hits.saturating_mul(1000) / record.opportunities.max(1)
    };
    let momentum = record.hits.saturating_add(1).saturating_mul(1000)
        / record
            .hits
            .saturating_add(record.failures)
            .saturating_add(2);
    let weighted = trust
        .saturating_mul(600)
        .saturating_add(heat.min(1000).saturating_mul(250))
        .saturating_add(momentum.min(1000).saturating_mul(100))
        / 1000;
    u16::try_from(weighted.min(950))
        .unwrap_or(950)
        .saturating_add(evidence_bonus_milli)
        .min(1000)
}

fn gene_health_blocks_expression(gene: &ReasoningGene) -> bool {
    matches!(
        gene.derived_status(),
        ReasoningGeneStatus::Malignant
            | ReasoningGeneStatus::Quarantined
            | ReasoningGeneStatus::Regenerating
    )
}

fn gene_is_protected(gene: &ReasoningGene) -> bool {
    gene.kind == ReasoningGeneKind::Safety
}

fn unit_milli(value: f32) -> u16 {
    if value.is_finite() {
        (value.clamp(0.0, 1.0) * 1000.0).round() as u16
    } else {
        0
    }
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
