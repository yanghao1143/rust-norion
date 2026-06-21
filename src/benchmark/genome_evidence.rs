use crate::engine::InferenceOutcome;
use crate::hardware::DeviceClass;

use super::{BenchmarkCase, explicit_device_count, push_unique_device};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BenchmarkGenomeEvidence {
    pub expression_cases: usize,
    pub gene_scissors_proposal_cases: usize,
    pub total_genes: usize,
    pub total_active_genes: usize,
    pub total_aged_genes: usize,
    pub total_malignant_genes: usize,
    pub total_relabel_candidates: usize,
    pub total_regeneration_candidates: usize,
    pub total_gene_scissors_proposals: usize,
    pub failures: Vec<String>,
    pub(super) expression_devices: Vec<DeviceClass>,
    pub(super) proposal_devices: Vec<DeviceClass>,
}

impl BenchmarkGenomeEvidence {
    pub(super) fn record(&mut self, case: &BenchmarkCase, outcome: &InferenceOutcome) {
        let device = outcome.hardware_plan.device;
        let expression = &outcome.reasoning_genome;

        if expression.expression_gene_count > 0 {
            self.expression_cases += 1;
            push_unique_device(&mut self.expression_devices, device);
        }
        if expression.scissors_proposal_count() > 0 {
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
        if expression.malignant_gene_count() > expression.regeneration_candidate_count() {
            self.failures.push(format!(
                "{}:{} reasoning_genome malignant genes require regeneration candidates",
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
        if !(0.0..=1.0).contains(&expression.youth_pressure) {
            self.failures.push(format!(
                "{}:{} reasoning_genome youth_pressure {:.6} outside 0.0..=1.0",
                device.as_str(),
                case.name,
                expression.youth_pressure
            ));
        }
    }

    pub fn expression_device_profiles(&self) -> usize {
        explicit_device_count(&self.expression_devices)
    }

    pub fn gene_scissors_proposal_device_profiles(&self) -> usize {
        explicit_device_count(&self.proposal_devices)
    }
}
