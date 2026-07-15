use crate::engine::InferenceOutcome;
use crate::hardware::DeviceClass;
use crate::privacy_redaction::contains_private_or_executable_marker;
use crate::reasoning_genome::{
    DnaEvolutionCandidateDecision, DnaEvolutionController, DnaEvolutionControllerReport,
    DnaEvolutionValidationEvidence, DnaEvolutionValidationStatus, GeneScissorsOperatorDecision,
    MalignantGeneRecoveryDrillReport, MutationRepairFixtureReport,
    default_malignant_gene_recovery_drill_corpus, default_mutation_repair_fixture_corpus,
};
use crate::writer_gate::{
    UnifiedWriterGate, UnifiedWriterGateCandidate, UnifiedWriterGateDecision,
};

use super::{BenchmarkCase, explicit_device_count, push_unique_device};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchmarkGenomeEvidence {
    pub expression_cases: usize,
    pub splice_cases: usize,
    pub gene_scissors_proposal_cases: usize,
    pub mutation_repair_fixtures: usize,
    pub mutation_repair_fixture_kinds: usize,
    pub mutation_repair_candidates: usize,
    pub mutation_repair_review_packets: usize,
    pub malignant_gene_recovery_drills: usize,
    pub malignant_gene_quarantines: usize,
    pub malignant_gene_cut_candidates: usize,
    pub malignant_gene_regeneration_candidates: usize,
    pub malignant_gene_failed_replay: usize,
    pub dna_evolution_reports: usize,
    pub dna_evolution_candidates: usize,
    pub dna_evolution_candidate_previews: usize,
    pub dna_evolution_candidate_ledger_records: usize,
    pub dna_evolution_candidate_ledger_preview_only: usize,
    pub dna_evolution_holds: usize,
    pub dna_evolution_rejects: usize,
    pub dna_evolution_rollbacks: usize,
    pub dna_evolution_activation_eligible: usize,
    pub dna_evolution_transaction_replays: usize,
    pub dna_evolution_replay_passed: usize,
    pub dna_evolution_validation_passed: usize,
    pub dna_evolution_fitness_delta_milli: i64,
    pub dna_evolution_writer_gate_reports: usize,
    pub dna_evolution_writer_gate_preview_only: usize,
    pub dna_evolution_writer_gate_holds: usize,
    pub dna_evolution_writer_gate_rejects: usize,
    pub dna_evolution_writer_gate_ready: usize,
    pub dna_evolution_writer_gate_explicit_apply_required: usize,
    pub dna_evolution_writer_gate_durable_write_allowed: usize,
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

impl Default for BenchmarkGenomeEvidence {
    fn default() -> Self {
        let mut evidence = Self {
            expression_cases: 0,
            splice_cases: 0,
            gene_scissors_proposal_cases: 0,
            mutation_repair_fixtures: 0,
            mutation_repair_fixture_kinds: 0,
            mutation_repair_candidates: 0,
            mutation_repair_review_packets: 0,
            malignant_gene_recovery_drills: 0,
            malignant_gene_quarantines: 0,
            malignant_gene_cut_candidates: 0,
            malignant_gene_regeneration_candidates: 0,
            malignant_gene_failed_replay: 0,
            dna_evolution_reports: 0,
            dna_evolution_candidates: 0,
            dna_evolution_candidate_previews: 0,
            dna_evolution_candidate_ledger_records: 0,
            dna_evolution_candidate_ledger_preview_only: 0,
            dna_evolution_holds: 0,
            dna_evolution_rejects: 0,
            dna_evolution_rollbacks: 0,
            dna_evolution_activation_eligible: 0,
            dna_evolution_transaction_replays: 0,
            dna_evolution_replay_passed: 0,
            dna_evolution_validation_passed: 0,
            dna_evolution_fitness_delta_milli: 0,
            dna_evolution_writer_gate_reports: 0,
            dna_evolution_writer_gate_preview_only: 0,
            dna_evolution_writer_gate_holds: 0,
            dna_evolution_writer_gate_rejects: 0,
            dna_evolution_writer_gate_ready: 0,
            dna_evolution_writer_gate_explicit_apply_required: 0,
            dna_evolution_writer_gate_durable_write_allowed: 0,
            total_genes: 0,
            total_active_genes: 0,
            total_aged_genes: 0,
            total_malignant_genes: 0,
            total_relabel_candidates: 0,
            total_regeneration_candidates: 0,
            total_gene_scissors_proposals: 0,
            total_repair_payloads: 0,
            total_regeneration_payloads: 0,
            total_lifecycle_records: 0,
            total_lifecycle_tombstone_candidates: 0,
            total_lifecycle_pending_validations: 0,
            total_lifecycle_source_evidence: 0,
            total_splice_segments: 0,
            total_splice_exons: 0,
            total_splice_introns: 0,
            total_splice_variants: 0,
            total_splice_retained: 0,
            total_splice_skipped: 0,
            total_splice_quarantined: 0,
            total_splice_repair_candidates: 0,
            total_splice_input_tokens: 0,
            total_splice_retained_tokens: 0,
            total_splice_lifecycle_records: 0,
            total_splice_lifecycle_quarantined: 0,
            total_splice_lifecycle_held: 0,
            total_splice_lifecycle_rejected: 0,
            total_splice_findings: 0,
            total_splice_proposals: 0,
            failures: Vec::new(),
            expression_devices: Vec::new(),
            splice_devices: Vec::new(),
            proposal_devices: Vec::new(),
        };
        evidence.record_mutation_repair_fixture_report(
            &default_mutation_repair_fixture_corpus().evaluate(),
        );
        evidence.record_malignant_gene_recovery_drill_report(
            &default_malignant_gene_recovery_drill_corpus().evaluate(),
        );
        evidence
    }
}

impl BenchmarkGenomeEvidence {
    fn record_mutation_repair_fixture_report(&mut self, report: &MutationRepairFixtureReport) {
        self.mutation_repair_fixtures += report.results.len();
        self.mutation_repair_fixture_kinds += report.covered_fixture_kinds.len();
        self.mutation_repair_candidates += report.total_repair_candidate_count;
        self.mutation_repair_review_packets += report.total_review_packet_line_count;
        if !report.preview_only {
            self.failures
                .push("mutation_repair_fixture_corpus must remain preview-only".to_owned());
        }
        for failure in &report.failures {
            self.failures
                .push(format!("mutation_repair_fixture:{failure}"));
        }
    }

    fn record_malignant_gene_recovery_drill_report(
        &mut self,
        report: &MalignantGeneRecoveryDrillReport,
    ) {
        self.malignant_gene_recovery_drills += report.results.len();
        self.malignant_gene_quarantines += report.quarantined_count;
        self.malignant_gene_cut_candidates += report.cut_candidate_count;
        self.malignant_gene_regeneration_candidates += report.regeneration_candidate_count;
        self.malignant_gene_failed_replay += report.failed_replay_count;
        if !report.preview_only {
            self.failures
                .push("malignant_gene_recovery_drills must remain preview-only".to_owned());
        }
        for failure in &report.failures {
            self.failures
                .push(format!("malignant_gene_recovery_drill:{failure}"));
        }
    }

    fn record_dna_evolution_report(
        &mut self,
        case: &BenchmarkCase,
        device: DeviceClass,
        lane: &str,
        report: &DnaEvolutionControllerReport,
    ) {
        self.dna_evolution_reports += 1;
        self.dna_evolution_candidates += report.candidate_count();
        self.dna_evolution_candidate_previews +=
            report.decision_count(DnaEvolutionCandidateDecision::CandidatePreview);
        let candidate_ledger_lines = report.candidate_ledger_lines();
        if !candidate_ledger_lines.is_empty() {
            match DnaEvolutionControllerReport::replay_candidate_ledger_lines(
                &candidate_ledger_lines,
            ) {
                Ok(replay) => {
                    self.dna_evolution_candidate_ledger_records += replay.candidate_count;
                    if replay.passed_candidate_only_gate() {
                        self.dna_evolution_candidate_ledger_preview_only += replay.candidate_count;
                    }
                }
                Err(_) => self.failures.push(format!(
                    "{}:{} dna_evolution_controller {lane} candidate ledger replay failed",
                    device.as_str(),
                    case.name
                )),
            }
        }
        self.dna_evolution_holds += report.decision_count(DnaEvolutionCandidateDecision::Hold);
        self.dna_evolution_rejects += report.decision_count(DnaEvolutionCandidateDecision::Reject);
        self.dna_evolution_rollbacks +=
            report.decision_count(DnaEvolutionCandidateDecision::Rollback);
        self.dna_evolution_activation_eligible += report.activation_eligible_count();
        self.dna_evolution_transaction_replays += report.transaction_replay_count;
        self.dna_evolution_fitness_delta_milli += i64::from(report.total_fitness_delta_milli);
        if report.transaction_replay_passed {
            self.dna_evolution_replay_passed += 1;
        }
        if report.validation_status == DnaEvolutionValidationStatus::Passed {
            self.dna_evolution_validation_passed += 1;
        }
        if !report.is_read_only_preview() {
            self.failures.push(format!(
                "{}:{} dna_evolution_controller {lane} must remain read-only preview",
                device.as_str(),
                case.name
            ));
        }
        if report.write_allowed || report.applied {
            self.failures.push(format!(
                "{}:{} dna_evolution_controller {lane} cannot write or apply during benchmark",
                device.as_str(),
                case.name
            ));
        }
        if report.activation_eligible_count() > 0 {
            self.failures.push(format!(
                "{}:{} dna_evolution_controller {lane} cannot auto-activate without operator approval",
                device.as_str(),
                case.name
            ));
        }
        if !report.transaction_replay_passed {
            self.failures.push(format!(
                "{}:{} dna_evolution_controller {lane} requires passing transaction replay",
                device.as_str(),
                case.name
            ));
        }
        if report.transaction_replay_count < report.candidate_count() {
            self.failures.push(format!(
                "{}:{} dna_evolution_controller {lane} transaction replay must cover candidates",
                device.as_str(),
                case.name
            ));
        }
        if report.validation_status != DnaEvolutionValidationStatus::Passed {
            self.failures.push(format!(
                "{}:{} dna_evolution_controller {lane} requires passing validation evidence",
                device.as_str(),
                case.name
            ));
        }
        let trace_line = report.redacted_trace_line();
        if contains_private_or_executable_marker(&trace_line) {
            self.failures.push(format!(
                "{}:{} dna_evolution_controller {lane} trace leaked blocked marker",
                device.as_str(),
                case.name
            ));
        }
        if !trace_line.contains("\"raw_payload_included\":false") {
            self.failures.push(format!(
                "{}:{} dna_evolution_controller {lane} trace must declare raw payload exclusion",
                device.as_str(),
                case.name
            ));
        }
        let writer_gate = UnifiedWriterGate::new().evaluate([
            UnifiedWriterGateCandidate::dna_evolution_controller_report(report),
        ]);
        self.dna_evolution_writer_gate_reports += 1;
        self.dna_evolution_writer_gate_preview_only += writer_gate.preview_only_records;
        self.dna_evolution_writer_gate_holds += writer_gate.held_records;
        self.dna_evolution_writer_gate_rejects += writer_gate.rejected_records;
        self.dna_evolution_writer_gate_ready += writer_gate.ready_records;
        if writer_gate.explicit_apply_required {
            self.dna_evolution_writer_gate_explicit_apply_required += 1;
        }
        if writer_gate.durable_write_allowed {
            self.dna_evolution_writer_gate_durable_write_allowed += 1;
        }
        if writer_gate.decision == UnifiedWriterGateDecision::ReadyForExplicitApply {
            self.failures.push(format!(
                "{}:{} dna_evolution_controller {lane} writer gate cannot become ready during benchmark",
                device.as_str(),
                case.name
            ));
        }
        if writer_gate.durable_write_allowed || writer_gate.write_allowed || writer_gate.applied {
            self.failures.push(format!(
                "{}:{} dna_evolution_controller {lane} writer gate cannot allow/apply durable writes during benchmark",
                device.as_str(),
                case.name
            ));
        }
        if !writer_gate.explicit_apply_required {
            self.failures.push(format!(
                "{}:{} dna_evolution_controller {lane} writer gate must preserve explicit apply boundary",
                device.as_str(),
                case.name
            ));
        }
        if contains_private_or_executable_marker(&writer_gate.summary_line()) {
            self.failures.push(format!(
                "{}:{} dna_evolution_controller {lane} writer gate summary leaked blocked marker",
                device.as_str(),
                case.name
            ));
        }
    }

    pub(super) fn record(&mut self, case: &BenchmarkCase, outcome: &InferenceOutcome) {
        let device = outcome.hardware_plan.device;
        let expression = &outcome.reasoning_genome;
        let splice = &outcome.reasoning_genome_splice;
        let dna_evolution_controller = DnaEvolutionController::default();
        let dna_evolution_validation = DnaEvolutionValidationEvidence::passing();
        let dna_evolution_operator = GeneScissorsOperatorDecision::Pending;
        let dna_evolution_expression = dna_evolution_controller.preview_expression(
            expression,
            &dna_evolution_validation,
            dna_evolution_operator,
        );
        let dna_evolution_splice = dna_evolution_controller.preview_splice(
            splice,
            &dna_evolution_validation,
            dna_evolution_operator,
        );

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
        self.record_dna_evolution_report(case, device, "expression", &dna_evolution_expression);
        self.record_dna_evolution_report(case, device, "splice", &dna_evolution_splice);

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
