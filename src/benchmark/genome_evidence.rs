use crate::engine::InferenceOutcome;
use crate::hardware::DeviceClass;

use super::{BenchmarkCase, explicit_device_count, push_unique_device};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BenchmarkGenomeEvidence {
    pub expression_cases: usize,
    pub splice_cases: usize,
    pub gene_scissors_proposal_cases: usize,
    pub total_genes: usize,
    pub total_active_genes: usize,
    pub total_aged_genes: usize,
    pub total_malignant_genes: usize,
    pub total_relabel_candidates: usize,
    pub total_regeneration_candidates: usize,
    pub total_gene_scissors_proposals: usize,
    pub total_repair_payloads: usize,
    pub total_regeneration_payloads: usize,
    pub total_lifecycle_records: usize,
    pub total_lifecycle_tombstone_candidates: usize,
    pub total_lifecycle_pending_validations: usize,
    pub total_lifecycle_source_evidence: usize,
    pub total_splice_segments: usize,
    pub total_splice_exons: usize,
    pub total_splice_introns: usize,
    pub total_splice_variants: usize,
    pub total_splice_retained: usize,
    pub total_splice_skipped: usize,
    pub total_splice_quarantined: usize,
    pub total_splice_repair_candidates: usize,
    pub total_splice_input_tokens: usize,
    pub total_splice_retained_tokens: usize,
    pub total_splice_lifecycle_records: usize,
    pub total_splice_lifecycle_quarantined: usize,
    pub total_splice_lifecycle_held: usize,
    pub total_splice_lifecycle_rejected: usize,
    pub total_splice_findings: usize,
    pub total_splice_proposals: usize,
    pub failures: Vec<String>,
    pub(super) expression_devices: Vec<DeviceClass>,
    pub(super) splice_devices: Vec<DeviceClass>,
    pub(super) proposal_devices: Vec<DeviceClass>,
}

impl BenchmarkGenomeEvidence {
    pub(super) fn record(&mut self, case: &BenchmarkCase, outcome: &InferenceOutcome) {
        let device = outcome.hardware_plan.device;
        let expression = &outcome.reasoning_genome;
        let splice = &outcome.reasoning_genome_splice;

        if expression.expression_gene_count > 0 {
            self.expression_cases += 1;
            push_unique_device(&mut self.expression_devices, device);
        }
        if !splice.segments.is_empty() {
            self.splice_cases += 1;
            push_unique_device(&mut self.splice_devices, device);
        }
        if expression.scissors_proposal_count() > 0 || !splice.mutation_plans.is_empty() {
            self.gene_scissors_proposal_cases += 1;
            push_unique_device(&mut self.proposal_devices, device);
        }

        self.total_genes += expression.expression_gene_count;
        self.total_active_genes += expression.active_gene_count();
        self.total_aged_genes += expression.aged_gene_count();
        self.total_malignant_genes += expression.malignant_gene_count();
        self.total_relabel_candidates += expression.relabel_candidate_count();
        self.total_regeneration_candidates += expression.regeneration_candidate_count();
        self.total_gene_scissors_proposals += expression.scissors_proposal_count();
        self.total_repair_payloads += expression.repair_payload_count();
        self.total_regeneration_payloads += expression.regeneration_payload_count();
        self.total_lifecycle_records += expression.lifecycle_record_count();
        self.total_lifecycle_tombstone_candidates += expression.tombstone_candidate_count();
        self.total_lifecycle_pending_validations += expression.pending_lifecycle_validation_count();
        self.total_lifecycle_source_evidence += expression.lifecycle_source_evidence_count();
        self.total_splice_segments += splice.segments.len();
        self.total_splice_exons += splice.exon_count();
        self.total_splice_introns += splice.intron_count();
        self.total_splice_variants += splice.variant_count();
        self.total_splice_retained += splice.retained_count();
        self.total_splice_skipped += splice.skipped_count();
        self.total_splice_quarantined += splice.quarantined_count();
        self.total_splice_repair_candidates += splice.repair_candidate_count();
        self.total_splice_input_tokens += splice.total_token_count();
        self.total_splice_retained_tokens += splice.retained_token_count();
        self.total_splice_lifecycle_records += splice.lifecycle_record_count();
        self.total_splice_lifecycle_quarantined += splice.quarantined_lifecycle_count();
        self.total_splice_lifecycle_held += splice.held_lifecycle_count();
        self.total_splice_lifecycle_rejected += splice.rejected_lifecycle_count();
        self.total_splice_findings += splice.findings.len();
        self.total_splice_proposals += splice.mutation_plans.len();
        self.total_gene_scissors_proposals += splice.mutation_plans.len();

        if expression.genome_id.trim().is_empty() {
            self.failures.push(format!(
                "{}:{} reasoning_genome genome_id is empty",
                device.as_str(),
                case.name
            ));
        }
        if expression.stable_anchor_id.trim().is_empty() {
            self.failures.push(format!(
                "{}:{} reasoning_genome stable_anchor_id is empty",
                device.as_str(),
                case.name
            ));
        }
        if expression.expression_gene_count == 0 {
            self.failures.push(format!(
                "{}:{} reasoning_genome gene_count must be > 0",
                device.as_str(),
                case.name
            ));
        }
        if expression.active_gene_count() > expression.expression_gene_count {
            self.failures.push(format!(
                "{}:{} reasoning_genome active genes exceed gene_count",
                device.as_str(),
                case.name
            ));
        }
        if expression.aged_gene_count() > expression.relabel_candidate_count() {
            self.failures.push(format!(
                "{}:{} reasoning_genome aging genes require relabel candidates",
                device.as_str(),
                case.name
            ));
        }
        if expression.relabel_candidate_count() > 0
            && expression.repair_payload_count() < expression.relabel_candidate_count()
        {
            self.failures.push(format!(
                "{}:{} reasoning_genome relabel candidates require repair payloads",
                device.as_str(),
                case.name
            ));
        }
        if expression.malignant_gene_count() > expression.regeneration_candidate_count() {
            self.failures.push(format!(
                "{}:{} reasoning_genome malignant genes require regeneration candidates",
                device.as_str(),
                case.name
            ));
        }
        if expression.malignant_gene_count() > 0
            && expression.regeneration_payload_count() < expression.malignant_gene_count()
        {
            self.failures.push(format!(
                "{}:{} reasoning_genome malignant genes require regeneration payloads",
                device.as_str(),
                case.name
            ));
        }
        if !expression.is_read_only_preview() {
            self.failures.push(format!(
                "{}:{} reasoning_genome expression must remain read-only preview",
                device.as_str(),
                case.name
            ));
        }
        if expression.lifecycle_record_count() < expression.expression_gene_count {
            self.failures.push(format!(
                "{}:{} reasoning_genome lifecycle records must cover every gene",
                device.as_str(),
                case.name
            ));
        }
        if expression.lifecycle_source_evidence_count() < expression.lifecycle_record_count() {
            self.failures.push(format!(
                "{}:{} reasoning_genome lifecycle records require source evidence",
                device.as_str(),
                case.name
            ));
        }
        if expression.tombstone_candidate_count() < expression.malignant_gene_count() {
            self.failures.push(format!(
                "{}:{} reasoning_genome malignant genes require tombstone candidates",
                device.as_str(),
                case.name
            ));
        }
        if expression
            .mutation_plans
            .iter()
            .any(|plan| !plan.has_source_evidence())
        {
            self.failures.push(format!(
                "{}:{} reasoning_genome mutation plans require source evidence",
                device.as_str(),
                case.name
            ));
        }
        if !(0.0..=1.0).contains(&expression.youth_pressure) {
            self.failures.push(format!(
                "{}:{} reasoning_genome youth_pressure {:.6} outside 0.0..=1.0",
                device.as_str(),
                case.name,
                expression.youth_pressure
            ));
        }
        if splice.stable_anchor_id != expression.stable_anchor_id {
            self.failures.push(format!(
                "{}:{} reasoning_genome splice stable anchor does not match expression anchor",
                device.as_str(),
                case.name
            ));
        }
        if splice.segments.len()
            != splice.exon_count() + splice.intron_count() + splice.variant_count()
        {
            self.failures.push(format!(
                "{}:{} reasoning_genome splice segment counts are inconsistent",
                device.as_str(),
                case.name
            ));
        }
        if splice.segments.len()
            != splice.retained_count()
                + splice.skipped_count()
                + splice.quarantined_count()
                + splice.repair_candidate_count()
        {
            self.failures.push(format!(
                "{}:{} reasoning_genome splice disposition counts are inconsistent",
                device.as_str(),
                case.name
            ));
        }
        if splice.exon_count() != splice.retained_count()
            || splice.intron_count() != splice.skipped_count()
            || splice.variant_count()
                != splice.quarantined_count() + splice.repair_candidate_count()
        {
            self.failures.push(format!(
                "{}:{} reasoning_genome splice class/disposition counts do not align",
                device.as_str(),
                case.name
            ));
        }
        if !splice.segments.is_empty() && splice.segment_reason_summaries(usize::MAX).is_empty() {
            self.failures.push(format!(
                "{}:{} reasoning_genome splice segments require sanitized reason summaries",
                device.as_str(),
                case.name
            ));
        }
        if !splice.findings.is_empty() && splice.lifecycle_records.is_empty() {
            self.failures.push(format!(
                "{}:{} reasoning_genome splice findings require lifecycle records",
                device.as_str(),
                case.name
            ));
        }
        if splice.lifecycle_records.len() > splice.findings.len() {
            self.failures.push(format!(
                "{}:{} reasoning_genome splice lifecycle records exceed findings",
                device.as_str(),
                case.name
            ));
        }
        if !splice
            .lifecycle_records
            .iter()
            .all(|record| record.is_read_only_preview())
        {
            self.failures.push(format!(
                "{}:{} reasoning_genome splice lifecycle must remain read-only preview",
                device.as_str(),
                case.name
            ));
        }
        if splice.variant_count() > 0 && splice.findings.is_empty() {
            self.failures.push(format!(
                "{}:{} reasoning_genome splice variants require findings",
                device.as_str(),
                case.name
            ));
        }
        if !splice.findings.is_empty() && splice.mutation_plans.is_empty() {
            self.failures.push(format!(
                "{}:{} reasoning_genome splice findings require mutation plans",
                device.as_str(),
                case.name
            ));
        }
        if !splice.is_read_only_preview() {
            self.failures.push(format!(
                "{}:{} reasoning_genome splice must remain read-only preview",
                device.as_str(),
                case.name
            ));
        }
    }

    pub fn expression_device_profiles(&self) -> usize {
        explicit_device_count(&self.expression_devices)
    }

    pub fn splice_device_profiles(&self) -> usize {
        explicit_device_count(&self.splice_devices)
    }

    pub fn gene_scissors_proposal_device_profiles(&self) -> usize {
        explicit_device_count(&self.proposal_devices)
    }

    pub fn estimated_splice_saved_tokens(&self) -> usize {
        self.total_splice_input_tokens
            .saturating_sub(self.total_splice_retained_tokens)
    }
}
