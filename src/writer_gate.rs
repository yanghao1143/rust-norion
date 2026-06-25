use crate::memory_admission::{
    MemoryAdmissionApprovalState, MemoryAdmissionDecision, MemoryAdmissionPreview,
    MemoryKvLedgerWritePlan, MemoryPrivacyClassification,
};
use crate::privacy_redaction::{contains_private_or_executable_marker, stable_redaction_digest};
use crate::reasoning_genome::{
    DNA_EVOLUTION_CONTROLLER_SCHEMA_VERSION, DnaEvolutionCandidateDecision,
    DnaEvolutionControllerReport, DnaEvolutionValidationStatus,
    GENE_SCISSORS_TRANSACTION_SCHEMA_VERSION, GeneScissorsOperatorDecision,
    GeneScissorsTransactionJournal, GeneValidationStatus,
};
use crate::self_evolution::SelfEvolutionPromotionPreflightReport;
use crate::self_goal_proposal::{
    SELF_GOAL_QUEUE_PREVIEW_SCHEMA_VERSION, SELF_GOAL_QUEUE_PREVIEW_TRACE_SCHEMA,
    SelfGoalQueuePreviewDecision, SelfGoalQueuePreviewReport,
};

pub const UNIFIED_WRITER_GATE_SCHEMA_VERSION: &str = "unified_writer_gate_v1";
pub const UNIFIED_WRITER_GATE_TRACE_SCHEMA: &str = "rust-norion-unified-writer-gate-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum UnifiedWriterGateDomain {
    Memory,
    Genome,
    ExperimentLedger,
    EvolutionGoalQueue,
}

impl UnifiedWriterGateDomain {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Memory => "memory",
            Self::Genome => "genome",
            Self::ExperimentLedger => "experiment_ledger",
            Self::EvolutionGoalQueue => "evolution_goal_queue",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum UnifiedWriterGateWriteScope {
    DurableMemory,
    Genome,
    ExperimentLedger,
    EvolutionGoalQueue,
}

impl UnifiedWriterGateWriteScope {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DurableMemory => "durable_memory",
            Self::Genome => "genome",
            Self::ExperimentLedger => "experiment_ledger",
            Self::EvolutionGoalQueue => "evolution_goal_queue",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnifiedWriterGateDecision {
    PreviewOnly,
    Hold,
    Reject,
    ReadyForExplicitApply,
}

impl UnifiedWriterGateDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PreviewOnly => "preview_only",
            Self::Hold => "hold",
            Self::Reject => "reject",
            Self::ReadyForExplicitApply => "ready_for_explicit_apply",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnifiedWriterGatePolicy {
    pub durable_writes_enabled: bool,
    pub require_review_packet_refs: bool,
    pub require_validation_evidence: bool,
    pub require_trace_or_benchmark_evidence: bool,
    pub require_rollback_anchor: bool,
    pub require_privacy_gate: bool,
    pub require_license_gate: bool,
    pub require_operator_approval: bool,
    pub require_approval_refs_match: bool,
    pub reject_source_write_flags: bool,
}

impl Default for UnifiedWriterGatePolicy {
    fn default() -> Self {
        Self {
            durable_writes_enabled: false,
            require_review_packet_refs: true,
            require_validation_evidence: true,
            require_trace_or_benchmark_evidence: true,
            require_rollback_anchor: true,
            require_privacy_gate: true,
            require_license_gate: true,
            require_operator_approval: true,
            require_approval_refs_match: true,
            reject_source_write_flags: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnifiedWriterGateCandidate {
    pub schema_version: &'static str,
    pub domain: UnifiedWriterGateDomain,
    pub candidate_id: String,
    pub requested_writes: Vec<UnifiedWriterGateWriteScope>,
    pub review_packet_ids: Vec<String>,
    pub evidence_ids: Vec<String>,
    pub rollback_anchor_ids: Vec<String>,
    pub content_digests: Vec<String>,
    pub source_report_schemas: Vec<String>,
    pub validation_passed: bool,
    pub trace_or_benchmark_passed: bool,
    pub rollback_ready: bool,
    pub privacy_checked: bool,
    pub license_checked: bool,
    pub operator_approved: bool,
    pub approval_refs_match: bool,
    pub source_read_only: bool,
    pub source_preview_only: bool,
    pub source_write_allowed: bool,
    pub source_applied: bool,
    pub source_active: bool,
    pub raw_payload_redacted: bool,
}

impl UnifiedWriterGateCandidate {
    pub fn new(
        domain: UnifiedWriterGateDomain,
        candidate_id: impl Into<String>,
        requested_writes: impl IntoIterator<Item = UnifiedWriterGateWriteScope>,
    ) -> Self {
        let mut requested_writes = requested_writes.into_iter().collect::<Vec<_>>();
        requested_writes.sort();
        requested_writes.dedup();
        Self {
            schema_version: UNIFIED_WRITER_GATE_SCHEMA_VERSION,
            domain,
            candidate_id: safe_ref(candidate_id.into()),
            requested_writes,
            review_packet_ids: Vec::new(),
            evidence_ids: Vec::new(),
            rollback_anchor_ids: Vec::new(),
            content_digests: Vec::new(),
            source_report_schemas: Vec::new(),
            validation_passed: false,
            trace_or_benchmark_passed: false,
            rollback_ready: false,
            privacy_checked: false,
            license_checked: true,
            operator_approved: false,
            approval_refs_match: false,
            source_read_only: true,
            source_preview_only: true,
            source_write_allowed: false,
            source_applied: false,
            source_active: false,
            raw_payload_redacted: true,
        }
    }

    pub fn memory_admission_preview(preview: &MemoryAdmissionPreview) -> Self {
        let candidate_id = format!("memory-admission-preview:{}", preview.candidates.len());
        let review_packet_ids = unique_sanitized(
            preview
                .review_packets
                .iter()
                .map(|packet| packet.packet_id.as_str()),
        );
        let evidence_ids = preview
            .candidates
            .iter()
            .map(|candidate| {
                safe_ref(format!(
                    "memory-candidate:{}:evidence-{}:validation-{}",
                    candidate.id,
                    candidate.evidence.len(),
                    candidate.validation_evidence.len()
                ))
            })
            .collect::<Vec<_>>();
        let rollback_anchor_ids = unique_sanitized(
            preview
                .candidates
                .iter()
                .map(|candidate| candidate.rollback_anchor_id.as_str()),
        );
        let content_digests = unique_sanitized(
            preview
                .candidates
                .iter()
                .map(|candidate| candidate.source_hash.as_str()),
        );
        let source_report_schemas = vec![
            "rust-norion-memory-admission-preview-v1".to_owned(),
            "memory_kv_ledger_v1".to_owned(),
        ];
        let has_candidates = !preview.candidates.is_empty();
        let all_ready = preview
            .candidates
            .iter()
            .all(|candidate| candidate.decision == MemoryAdmissionDecision::Ready);
        let all_validation = preview
            .candidates
            .iter()
            .all(|candidate| !candidate.validation_evidence.is_empty());
        let all_privacy = preview.candidates.iter().all(|candidate| {
            candidate.privacy_checked
                && candidate.privacy_classification != MemoryPrivacyClassification::SensitiveBlocked
        });
        let packets_match_candidates = !review_packet_ids.is_empty()
            && preview.candidates.iter().all(|candidate| {
                preview.review_packets.iter().any(|packet| {
                    packet.candidate_id == candidate.id
                        && packet.rollback_anchor_id == candidate.rollback_anchor_id
                        && packet.source_hash == candidate.source_hash
                        && packet.approval_state != MemoryAdmissionApprovalState::Rejected
                        && packet.approval_state != MemoryAdmissionApprovalState::Quarantined
                        && packet.is_read_only_preview()
                })
            });

        Self::new(
            UnifiedWriterGateDomain::Memory,
            candidate_id,
            [UnifiedWriterGateWriteScope::DurableMemory],
        )
        .with_refs(
            review_packet_ids,
            evidence_ids,
            rollback_anchor_ids,
            content_digests,
            source_report_schemas,
        )
        .with_evidence(
            has_candidates && all_ready && all_validation,
            preview.ledger_plan.record_count() > 0,
            has_candidates && all_rollback_anchors(&preview.ledger_plan),
            all_privacy,
            true,
        )
        .with_operator_approval(
            preview.ledger_plan.authorized_count() > 0,
            packets_match_candidates,
        )
        .with_source_flags(
            preview.read_only && preview.ledger_plan.read_only,
            preview.is_read_only_preview(),
            preview.write_allowed || preview.ledger_plan.write_allowed,
            preview.applied || preview.ledger_plan.applied,
            false,
        )
        .with_raw_payload_redacted(memory_preview_redacted(preview))
    }

    pub fn genome_transaction_journal(journal: &GeneScissorsTransactionJournal) -> Self {
        let replay = journal.replay();
        let review_packet_ids = unique_sanitized(
            journal
                .transactions
                .iter()
                .map(|transaction| transaction.transaction_id.as_str()),
        );
        let evidence_ids = unique_sanitized(
            journal
                .transactions
                .iter()
                .map(|transaction| transaction.evidence_digest.as_str()),
        );
        let rollback_anchor_ids = unique_sanitized(
            journal
                .transactions
                .iter()
                .map(|transaction| transaction.rollback_anchor_id.as_str()),
        );
        let content_digests = journal
            .transactions
            .iter()
            .flat_map(|transaction| {
                [
                    transaction.before_digest.as_str(),
                    transaction.after_digest.as_str(),
                    transaction.forensic_copy_digest.as_str(),
                ]
            })
            .map(safe_ref)
            .collect::<Vec<_>>();
        let all_validation = !journal.transactions.is_empty()
            && journal
                .transactions
                .iter()
                .all(|transaction| transaction.validation_status == GeneValidationStatus::Passed);
        let operator_approved = !journal.transactions.is_empty()
            && journal.transactions.iter().all(|transaction| {
                transaction.operator_decision == GeneScissorsOperatorDecision::Approved
            });
        let source_active = journal.transactions.iter().any(|transaction| {
            transaction.active_expression_allowed || transaction.memory_admission_allowed
        });

        Self::new(
            UnifiedWriterGateDomain::Genome,
            format!("gene-scissors-journal:{}", replay.transaction_count),
            [UnifiedWriterGateWriteScope::Genome],
        )
        .with_refs(
            review_packet_ids,
            evidence_ids,
            rollback_anchor_ids,
            content_digests,
            vec![GENE_SCISSORS_TRANSACTION_SCHEMA_VERSION.to_owned()],
        )
        .with_evidence(
            all_validation,
            replay.transaction_count > 0 && replay.passed_preview_gate(),
            !journal.transactions.is_empty()
                && journal
                    .transactions
                    .iter()
                    .all(|transaction| !transaction.rollback_anchor_id.trim().is_empty()),
            journal.exports_are_redacted(),
            true,
        )
        .with_operator_approval(operator_approved, replay.duplicate_suppressed_count == 0)
        .with_source_flags(
            journal.read_only,
            journal.is_read_only_preview(),
            journal.write_allowed,
            journal.applied,
            source_active,
        )
        .with_raw_payload_redacted(journal.exports_are_redacted())
    }

    pub fn dna_evolution_controller_report(report: &DnaEvolutionControllerReport) -> Self {
        let preview_candidates = report
            .candidates
            .iter()
            .filter(|candidate| {
                candidate.decision == DnaEvolutionCandidateDecision::CandidatePreview
            })
            .collect::<Vec<_>>();
        let review_packet_ids = unique_sanitized(
            preview_candidates
                .iter()
                .map(|candidate| candidate.candidate_id.as_str()),
        );
        let evidence_ids = unique_owned(
            preview_candidates
                .iter()
                .flat_map(|candidate| candidate.validation_artifact_digests.iter().cloned())
                .chain([stable_redaction_digest([
                    "dna-evolution-controller-trace",
                    &report.redacted_trace_line(),
                ])])
                .collect(),
        );
        let rollback_anchor_ids = unique_sanitized(
            preview_candidates
                .iter()
                .map(|candidate| candidate.rollback_anchor_id.as_str()),
        );
        let content_digests = unique_owned(
            preview_candidates
                .iter()
                .flat_map(|candidate| {
                    [
                        candidate.source_plan_id.clone(),
                        candidate.target_gene_id.clone(),
                        candidate
                            .replacement_gene_id
                            .clone()
                            .unwrap_or_else(|| "no-replacement".to_owned()),
                    ]
                })
                .chain([
                    report.generation_id.clone(),
                    report.stable_anchor_id.clone(),
                    report.fitness_delta_summary(),
                ])
                .collect(),
        );
        let candidate_previews =
            report.decision_count(DnaEvolutionCandidateDecision::CandidatePreview);
        let activation_eligible = report.activation_eligible_count();
        let validation_passed = report.validation_status == DnaEvolutionValidationStatus::Passed
            && candidate_previews > 0;
        let replay_passed = report.transaction_replay_passed
            && report.transaction_replay_count >= report.candidate_count();
        let rollback_ready = replay_passed
            && !rollback_anchor_ids.is_empty()
            && preview_candidates
                .iter()
                .all(|candidate| !candidate.rollback_anchor_id.trim().is_empty());
        let operator_approved = report.operator_decision == GeneScissorsOperatorDecision::Approved
            && activation_eligible > 0;
        let approval_refs_match = operator_approved
            && activation_eligible == candidate_previews
            && report.decision_count(DnaEvolutionCandidateDecision::Hold) == 0
            && report.decision_count(DnaEvolutionCandidateDecision::Reject) == 0
            && report.decision_count(DnaEvolutionCandidateDecision::Rollback) == 0;
        let redacted = dna_evolution_report_redacted(report);

        Self::new(
            UnifiedWriterGateDomain::Genome,
            format!("dna-evolution-controller:{}", report.generation_id),
            [UnifiedWriterGateWriteScope::Genome],
        )
        .with_refs(
            review_packet_ids,
            evidence_ids,
            rollback_anchor_ids,
            content_digests,
            vec![DNA_EVOLUTION_CONTROLLER_SCHEMA_VERSION.to_owned()],
        )
        .with_evidence(
            validation_passed,
            replay_passed,
            rollback_ready,
            redacted,
            true,
        )
        .with_operator_approval(operator_approved, approval_refs_match)
        .with_source_flags(
            report.read_only,
            report.is_read_only_preview(),
            report.write_allowed,
            report.applied,
            false,
        )
        .with_raw_payload_redacted(redacted)
    }

    pub fn experiment_promotion_preflight(report: &SelfEvolutionPromotionPreflightReport) -> Self {
        let review_packet_ids =
            counted_refs("self-evolution-review-packet", report.review_packet_count);
        let evidence_ids = counted_refs("self-evolution-evidence", report.evidence_id_count);
        let rollback_anchor_ids = counted_refs(
            "self-evolution-rollback-anchor",
            report.rollback_anchor_count,
        );
        let content_digests =
            counted_refs("self-evolution-content-digest", report.content_digest_count);
        let source_report_schemas = counted_refs(
            "self-evolution-source-report-schema",
            report.source_report_schema_count,
        );

        Self::new(
            UnifiedWriterGateDomain::ExperimentLedger,
            report.candidate_id.clone(),
            [UnifiedWriterGateWriteScope::ExperimentLedger],
        )
        .with_refs(
            review_packet_ids,
            evidence_ids,
            rollback_anchor_ids,
            content_digests,
            source_report_schemas,
        )
        .with_evidence(
            report.rust_validation_passed && report.validation_passed,
            report.benchmark_gate_passed && report.adaptive_preview_evidence_present,
            report.rollback_anchor_count > 0,
            true,
            true,
        )
        .with_operator_approval(
            report.operator_approved,
            report.ready_for_explicit_promotion && report.blocked_reasons.is_empty(),
        )
        .with_source_flags(
            report.read_only && report.report_only,
            report.preview_only,
            report.write_allowed || report.activation_write_allowed,
            report.applied,
            report.active_candidate,
        )
        .with_raw_payload_redacted(!contains_private_or_executable_marker(
            &report.summary_line(),
        ))
    }

    pub fn self_goal_queue_preview(report: &SelfGoalQueuePreviewReport) -> Self {
        let append_records = report
            .records
            .iter()
            .filter(|record| record.decision == SelfGoalQueuePreviewDecision::AppendPreview)
            .collect::<Vec<_>>();
        let record_count = report.record_count.to_string();
        let append_preview_count = report.append_preview_count.to_string();
        let candidate_id = stable_redaction_digest([
            "self-goal-queue-preview",
            report.existing_queue_digest.as_str(),
            record_count.as_str(),
            append_preview_count.as_str(),
        ]);
        let review_packet_ids = append_records
            .iter()
            .map(|record| {
                safe_ref(format!(
                    "self-goal-queue-preview:{}:{}",
                    record.candidate_id, record.proposed_goal_id
                ))
            })
            .collect::<Vec<_>>();
        let evidence_ids = append_records
            .iter()
            .flat_map(|record| {
                [
                    record.existing_queue_digest.clone(),
                    record
                        .append_record_digest
                        .clone()
                        .unwrap_or_else(|| "missing-append-record-digest".to_owned()),
                    record
                        .resulting_queue_preview_digest
                        .clone()
                        .unwrap_or_else(|| "missing-resulting-queue-digest".to_owned()),
                ]
            })
            .collect::<Vec<_>>();
        let rollback_anchor_ids = if append_records.is_empty() {
            Vec::new()
        } else {
            vec![report.existing_queue_digest.clone()]
        };
        let content_digests = append_records
            .iter()
            .flat_map(|record| {
                [
                    record.append_record_digest.clone(),
                    record.resulting_queue_preview_digest.clone(),
                ]
            })
            .flatten()
            .collect::<Vec<_>>();
        let source_report_schemas = vec![
            SELF_GOAL_QUEUE_PREVIEW_SCHEMA_VERSION.to_owned(),
            SELF_GOAL_QUEUE_PREVIEW_TRACE_SCHEMA.to_owned(),
            report.admission_schema_version.to_owned(),
        ];
        let append_packet_ready = !append_records.is_empty()
            && append_records.iter().all(|record| {
                record.append_record_digest.is_some()
                    && record.resulting_queue_preview_digest.is_some()
                    && record.append_record_line.is_some()
                    && record.evidence_is_redacted()
            });
        let redacted = report.evidence_is_redacted()
            && !contains_private_or_executable_marker(&report.summary_line())
            && report
                .record_lines
                .iter()
                .all(|line| !contains_private_or_executable_marker(line));

        Self::new(
            UnifiedWriterGateDomain::EvolutionGoalQueue,
            candidate_id,
            [UnifiedWriterGateWriteScope::EvolutionGoalQueue],
        )
        .with_refs(
            review_packet_ids,
            evidence_ids,
            rollback_anchor_ids,
            content_digests,
            source_report_schemas,
        )
        .with_evidence(
            report.passed() && append_packet_ready,
            report.passed() && append_packet_ready,
            append_packet_ready,
            redacted,
            true,
        )
        .with_operator_approval(report.passed() && append_packet_ready, append_packet_ready)
        .with_source_flags(
            report.read_only,
            report.is_preview_only(),
            report.write_allowed,
            report.applied,
            false,
        )
        .with_raw_payload_redacted(redacted)
    }

    pub fn with_refs(
        mut self,
        review_packet_ids: Vec<String>,
        evidence_ids: Vec<String>,
        rollback_anchor_ids: Vec<String>,
        content_digests: Vec<String>,
        source_report_schemas: Vec<String>,
    ) -> Self {
        self.review_packet_ids = unique_owned(review_packet_ids);
        self.evidence_ids = unique_owned(evidence_ids);
        self.rollback_anchor_ids = unique_owned(rollback_anchor_ids);
        self.content_digests = unique_owned(content_digests);
        self.source_report_schemas = unique_owned(source_report_schemas);
        self
    }

    pub fn with_evidence(
        mut self,
        validation_passed: bool,
        trace_or_benchmark_passed: bool,
        rollback_ready: bool,
        privacy_checked: bool,
        license_checked: bool,
    ) -> Self {
        self.validation_passed = validation_passed;
        self.trace_or_benchmark_passed = trace_or_benchmark_passed;
        self.rollback_ready = rollback_ready;
        self.privacy_checked = privacy_checked;
        self.license_checked = license_checked;
        self
    }

    pub fn with_operator_approval(
        mut self,
        operator_approved: bool,
        approval_refs_match: bool,
    ) -> Self {
        self.operator_approved = operator_approved;
        self.approval_refs_match = approval_refs_match;
        self
    }

    pub fn with_source_flags(
        mut self,
        read_only: bool,
        preview_only: bool,
        write_allowed: bool,
        applied: bool,
        active: bool,
    ) -> Self {
        self.source_read_only = read_only;
        self.source_preview_only = preview_only;
        self.source_write_allowed = write_allowed;
        self.source_applied = applied;
        self.source_active = active;
        self
    }

    pub fn with_raw_payload_redacted(mut self, raw_payload_redacted: bool) -> Self {
        self.raw_payload_redacted = raw_payload_redacted;
        self
    }

    pub fn refs_digest(&self) -> String {
        let mut parts = vec![
            self.candidate_id.clone(),
            self.domain.as_str().to_owned(),
            write_scopes(&self.requested_writes),
        ];
        parts.extend(self.review_packet_ids.iter().cloned());
        parts.extend(self.evidence_ids.iter().cloned());
        parts.extend(self.rollback_anchor_ids.iter().cloned());
        parts.extend(self.content_digests.iter().cloned());
        parts.extend(self.source_report_schemas.iter().cloned());
        stable_redaction_digest(parts.iter().map(String::as_str))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnifiedWriterGateRecord {
    pub schema_version: &'static str,
    pub domain: UnifiedWriterGateDomain,
    pub candidate_id: String,
    pub requested_writes: Vec<UnifiedWriterGateWriteScope>,
    pub decision: UnifiedWriterGateDecision,
    pub durable_write_allowed: bool,
    pub explicit_apply_required: bool,
    pub reason_codes: Vec<String>,
    pub review_packet_count: usize,
    pub evidence_id_count: usize,
    pub rollback_anchor_count: usize,
    pub content_digest_count: usize,
    pub source_report_schema_count: usize,
    pub refs_digest: String,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl UnifiedWriterGateRecord {
    pub fn summary_line(&self) -> String {
        format!(
            "unified_writer_gate_record schema={} domain={} candidate={} requested_writes={} decision={} durable_write_allowed={} explicit_apply_required={} refs={} review_packets={} evidence_ids={} rollback_anchors={} content_digests={} source_report_schemas={} read_only={} write_allowed={} applied={} reasons={}",
            self.schema_version,
            self.domain.as_str(),
            self.candidate_id,
            write_scopes(&self.requested_writes),
            self.decision.as_str(),
            self.durable_write_allowed,
            self.explicit_apply_required,
            self.refs_digest,
            self.review_packet_count,
            self.evidence_id_count,
            self.rollback_anchor_count,
            self.content_digest_count,
            self.source_report_schema_count,
            self.read_only,
            self.write_allowed,
            self.applied,
            self.reason_codes.join("|")
        )
    }

    pub fn is_preview_only(&self) -> bool {
        self.decision == UnifiedWriterGateDecision::PreviewOnly
            && self.read_only
            && !self.write_allowed
            && !self.durable_write_allowed
            && !self.applied
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnifiedWriterGateReport {
    pub schema_version: &'static str,
    pub records: Vec<UnifiedWriterGateRecord>,
    pub decision: UnifiedWriterGateDecision,
    pub durable_write_allowed: bool,
    pub explicit_apply_required: bool,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
    pub memory_records: usize,
    pub genome_records: usize,
    pub experiment_ledger_records: usize,
    pub evolution_goal_queue_records: usize,
    pub ready_records: usize,
    pub held_records: usize,
    pub rejected_records: usize,
    pub preview_only_records: usize,
    pub evidence_digest: String,
}

impl UnifiedWriterGateReport {
    pub fn is_preview_only(&self) -> bool {
        self.decision == UnifiedWriterGateDecision::PreviewOnly
            && self.read_only
            && !self.write_allowed
            && !self.durable_write_allowed
            && !self.applied
            && self
                .records
                .iter()
                .all(UnifiedWriterGateRecord::is_preview_only)
    }

    pub fn summary_line(&self) -> String {
        format!(
            "unified_writer_gate schema={} decision={} records={} memory={} genome={} experiment_ledger={} evolution_goal_queue={} ready={} held={} rejected={} preview_only={} durable_write_allowed={} explicit_apply_required={} read_only={} write_allowed={} applied={} evidence={}",
            self.schema_version,
            self.decision.as_str(),
            self.records.len(),
            self.memory_records,
            self.genome_records,
            self.experiment_ledger_records,
            self.evolution_goal_queue_records,
            self.ready_records,
            self.held_records,
            self.rejected_records,
            self.preview_only_records,
            self.durable_write_allowed,
            self.explicit_apply_required,
            self.read_only,
            self.write_allowed,
            self.applied,
            self.evidence_digest
        )
    }

    pub fn json_line(&self) -> String {
        let reason_code_count = self
            .records
            .iter()
            .map(|record| record.reason_codes.len())
            .sum::<usize>();
        format!(
            "{{\"schema\":\"{}\",\"gate_schema\":\"{}\",\"decision\":\"{}\",\"records\":{},\"memory_records\":{},\"genome_records\":{},\"experiment_ledger_records\":{},\"evolution_goal_queue_records\":{},\"ready_records\":{},\"held_records\":{},\"rejected_records\":{},\"preview_only_records\":{},\"reason_code_count\":{},\"durable_write_allowed\":{},\"explicit_apply_required\":{},\"read_only\":{},\"write_allowed\":{},\"applied\":{},\"evidence_digest\":\"{}\",\"summary\":\"{}\"}}",
            json_escape(UNIFIED_WRITER_GATE_TRACE_SCHEMA),
            json_escape(self.schema_version),
            json_escape(self.decision.as_str()),
            self.records.len(),
            self.memory_records,
            self.genome_records,
            self.experiment_ledger_records,
            self.evolution_goal_queue_records,
            self.ready_records,
            self.held_records,
            self.rejected_records,
            self.preview_only_records,
            reason_code_count,
            self.durable_write_allowed,
            self.explicit_apply_required,
            self.read_only,
            self.write_allowed,
            self.applied,
            json_escape(&self.evidence_digest),
            json_escape(&self.summary_line())
        )
    }
}

#[derive(Debug, Clone)]
pub struct UnifiedWriterGate {
    pub policy: UnifiedWriterGatePolicy,
}

impl Default for UnifiedWriterGate {
    fn default() -> Self {
        Self {
            policy: UnifiedWriterGatePolicy::default(),
        }
    }
}

impl UnifiedWriterGate {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_policy(mut self, policy: UnifiedWriterGatePolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn evaluate(
        &self,
        candidates: impl IntoIterator<Item = UnifiedWriterGateCandidate>,
    ) -> UnifiedWriterGateReport {
        let records = candidates
            .into_iter()
            .map(|candidate| self.evaluate_candidate(&candidate))
            .collect::<Vec<_>>();
        let ready_records = records
            .iter()
            .filter(|record| record.decision == UnifiedWriterGateDecision::ReadyForExplicitApply)
            .count();
        let held_records = records
            .iter()
            .filter(|record| record.decision == UnifiedWriterGateDecision::Hold)
            .count();
        let rejected_records = records
            .iter()
            .filter(|record| record.decision == UnifiedWriterGateDecision::Reject)
            .count();
        let preview_only_records = records
            .iter()
            .filter(|record| record.decision == UnifiedWriterGateDecision::PreviewOnly)
            .count();
        let decision = if rejected_records > 0 {
            UnifiedWriterGateDecision::Reject
        } else if held_records > 0 {
            UnifiedWriterGateDecision::Hold
        } else if ready_records > 0 {
            UnifiedWriterGateDecision::ReadyForExplicitApply
        } else {
            UnifiedWriterGateDecision::PreviewOnly
        };
        let durable_write_allowed =
            decision == UnifiedWriterGateDecision::ReadyForExplicitApply && ready_records > 0;
        let explicit_apply_required =
            durable_write_allowed || records.iter().any(|record| record.explicit_apply_required);
        let read_only = !durable_write_allowed;
        let write_allowed = durable_write_allowed;
        let applied = false;
        let memory_records = records
            .iter()
            .filter(|record| record.domain == UnifiedWriterGateDomain::Memory)
            .count();
        let genome_records = records
            .iter()
            .filter(|record| record.domain == UnifiedWriterGateDomain::Genome)
            .count();
        let experiment_ledger_records = records
            .iter()
            .filter(|record| record.domain == UnifiedWriterGateDomain::ExperimentLedger)
            .count();
        let evolution_goal_queue_records = records
            .iter()
            .filter(|record| record.domain == UnifiedWriterGateDomain::EvolutionGoalQueue)
            .count();
        let evidence_digest = report_digest(&records, decision, durable_write_allowed);

        UnifiedWriterGateReport {
            schema_version: UNIFIED_WRITER_GATE_SCHEMA_VERSION,
            records,
            decision,
            durable_write_allowed,
            explicit_apply_required,
            read_only,
            write_allowed,
            applied,
            memory_records,
            genome_records,
            experiment_ledger_records,
            evolution_goal_queue_records,
            ready_records,
            held_records,
            rejected_records,
            preview_only_records,
            evidence_digest,
        }
    }

    fn evaluate_candidate(
        &self,
        candidate: &UnifiedWriterGateCandidate,
    ) -> UnifiedWriterGateRecord {
        let mut reason_codes = Vec::new();
        if candidate.candidate_id.trim().is_empty() {
            reason_codes.push("candidate_id_empty".to_owned());
        }
        if candidate.requested_writes.is_empty() {
            reason_codes.push("requested_write_scope_missing".to_owned());
        }
        if self.policy.require_review_packet_refs && candidate.review_packet_ids.is_empty() {
            reason_codes.push("review_packet_refs_missing".to_owned());
        }
        if self.policy.require_validation_evidence && !candidate.validation_passed {
            reason_codes.push("validation_evidence_missing_or_failed".to_owned());
        }
        if self.policy.require_trace_or_benchmark_evidence && !candidate.trace_or_benchmark_passed {
            reason_codes.push("trace_or_benchmark_evidence_missing_or_failed".to_owned());
        }
        if self.policy.require_rollback_anchor
            && (!candidate.rollback_ready || candidate.rollback_anchor_ids.is_empty())
        {
            reason_codes.push("rollback_anchor_missing_or_not_ready".to_owned());
        }
        if self.policy.require_privacy_gate && !candidate.privacy_checked {
            reason_codes.push("privacy_gate_missing_or_failed".to_owned());
        }
        if !candidate.raw_payload_redacted {
            reason_codes.push("raw_payload_redaction_failed".to_owned());
        }
        if self.policy.require_license_gate && !candidate.license_checked {
            reason_codes.push("license_gate_missing_or_failed".to_owned());
        }
        if self.policy.require_operator_approval && !candidate.operator_approved {
            reason_codes.push("operator_approval_missing".to_owned());
        }
        if self.policy.require_approval_refs_match && !candidate.approval_refs_match {
            reason_codes.push("approval_refs_missing_or_mismatched".to_owned());
        }
        if !candidate.source_read_only {
            reason_codes.push("source_not_read_only".to_owned());
        }
        if !candidate.source_preview_only {
            reason_codes.push("source_not_preview_only".to_owned());
        }
        if self.policy.reject_source_write_flags && candidate.source_write_allowed {
            reason_codes.push("source_write_allowed_before_unified_gate".to_owned());
        }
        if candidate.source_applied {
            reason_codes.push("source_already_applied".to_owned());
        }
        if candidate.source_active {
            reason_codes.push("source_active_before_unified_gate".to_owned());
        }
        if !self.policy.durable_writes_enabled {
            reason_codes.push("durable_writes_disabled".to_owned());
        }

        let non_policy_blockers = reason_codes
            .iter()
            .filter(|reason| reason.as_str() != "durable_writes_disabled")
            .count();
        let decision = if reason_codes.iter().any(|reason| {
            matches!(
                reason.as_str(),
                "privacy_gate_missing_or_failed"
                    | "raw_payload_redaction_failed"
                    | "license_gate_missing_or_failed"
                    | "source_write_allowed_before_unified_gate"
                    | "source_already_applied"
                    | "source_active_before_unified_gate"
            )
        }) {
            UnifiedWriterGateDecision::Reject
        } else if non_policy_blockers > 0 {
            UnifiedWriterGateDecision::Hold
        } else if self.policy.durable_writes_enabled {
            UnifiedWriterGateDecision::ReadyForExplicitApply
        } else {
            UnifiedWriterGateDecision::PreviewOnly
        };
        let durable_write_allowed = decision == UnifiedWriterGateDecision::ReadyForExplicitApply;
        let write_allowed = durable_write_allowed;
        let read_only = !durable_write_allowed;

        UnifiedWriterGateRecord {
            schema_version: UNIFIED_WRITER_GATE_SCHEMA_VERSION,
            domain: candidate.domain,
            candidate_id: candidate.candidate_id.clone(),
            requested_writes: candidate.requested_writes.clone(),
            decision,
            durable_write_allowed,
            explicit_apply_required: true,
            reason_codes,
            review_packet_count: candidate.review_packet_ids.len(),
            evidence_id_count: candidate.evidence_ids.len(),
            rollback_anchor_count: candidate.rollback_anchor_ids.len(),
            content_digest_count: candidate.content_digests.len(),
            source_report_schema_count: candidate.source_report_schemas.len(),
            refs_digest: candidate.refs_digest(),
            read_only,
            write_allowed,
            applied: false,
        }
    }
}

fn all_rollback_anchors(plan: &MemoryKvLedgerWritePlan) -> bool {
    !plan.records.is_empty()
        && plan
            .records
            .iter()
            .all(|record| !record.rollback_anchor_id.trim().is_empty())
}

fn memory_preview_redacted(preview: &MemoryAdmissionPreview) -> bool {
    preview
        .candidates
        .iter()
        .map(|candidate| candidate.summary())
        .chain(preview.review_packets.iter().map(|packet| packet.summary()))
        .chain(preview.ledger_plan.summary_lines())
        .all(|line| !contains_private_or_executable_marker(&line))
}

fn dna_evolution_report_redacted(report: &DnaEvolutionControllerReport) -> bool {
    !contains_private_or_executable_marker(&report.summary_line())
        && !contains_private_or_executable_marker(&report.redacted_trace_line())
        && report.candidates.iter().all(|candidate| {
            !contains_private_or_executable_marker(&candidate.candidate_id)
                && !contains_private_or_executable_marker(&candidate.generation_id)
                && !contains_private_or_executable_marker(&candidate.stable_anchor_id)
                && !contains_private_or_executable_marker(&candidate.rollback_anchor_id)
                && !contains_private_or_executable_marker(&candidate.source_plan_id)
                && !contains_private_or_executable_marker(&candidate.target_gene_id)
                && candidate
                    .replacement_gene_id
                    .as_deref()
                    .is_none_or(|value| !contains_private_or_executable_marker(value))
                && candidate
                    .reason_codes
                    .iter()
                    .all(|reason| !contains_private_or_executable_marker(reason))
                && candidate
                    .validation_artifact_digests
                    .iter()
                    .all(|digest| !contains_private_or_executable_marker(digest))
        })
}

fn write_scopes(scopes: &[UnifiedWriterGateWriteScope]) -> String {
    scopes
        .iter()
        .map(|scope| scope.as_str())
        .collect::<Vec<_>>()
        .join("|")
}

fn report_digest(
    records: &[UnifiedWriterGateRecord],
    decision: UnifiedWriterGateDecision,
    durable_write_allowed: bool,
) -> String {
    let mut parts = vec![
        UNIFIED_WRITER_GATE_SCHEMA_VERSION.to_owned(),
        decision.as_str().to_owned(),
        durable_write_allowed.to_string(),
    ];
    parts.extend(records.iter().map(UnifiedWriterGateRecord::summary_line));
    stable_redaction_digest(parts.iter().map(String::as_str))
}

fn counted_refs(prefix: &str, count: usize) -> Vec<String> {
    (0..count)
        .map(|index| format!("{prefix}:{index}"))
        .collect()
}

fn unique_sanitized<'a>(values: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    unique_owned(values.into_iter().map(safe_ref).collect())
}

fn unique_owned(values: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    for value in values {
        let value = safe_ref(value);
        if !value.trim().is_empty() && !out.iter().any(|existing| existing == &value) {
            out.push(value);
        }
    }
    out
}

fn safe_ref(value: impl Into<String>) -> String {
    let value = value.into();
    if contains_private_or_executable_marker(&value) {
        stable_redaction_digest(["unified-writer-gate-redacted-ref", value.trim()])
    } else {
        value.trim().replace(['\t', '\n', '\r', '|'], "_")
    }
}

fn json_escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            character if character.is_control() => {
                out.push_str(&format!("\\u{:04x}", character as u32))
            }
            character => out.push(character),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evolution_goal::{
        EvolutionGoalEvidence, EvolutionGoalEvidenceKind, EvolutionGoalQueue,
        EvolutionGoalRunEvidence,
    };
    use crate::hierarchy::TaskProfile;
    use crate::memory_admission::{
        MemoryAdmissionCandidate, MemoryAdmissionKind, MemoryAdmissionReviewPacket,
        MemoryKvLedgerRecord, MemoryKvLedgerWriteDecision, ReinforcedKvFusionPlan,
    };
    use crate::reasoning_genome::{
        DnaEvolutionController, DnaEvolutionValidationEvidence, GeneScissorsIntent,
        GeneScissorsTransaction, GeneScissorsTransactionJournal, GeneScissorsTransactionState,
        GeneValidationStatus, MutationPlan,
    };
    use crate::self_evolution::SelfEvolutionPromotionPreflightDecision;
    use crate::self_goal_proposal::{
        SelfGoalProposalCandidate, SelfGoalQueuePreviewReport,
        default_noiron_self_goal_queue_preview_report, default_self_goal_admission_report,
        default_self_goal_proposal_report, default_self_goal_queue_preview_report,
    };

    #[test]
    fn default_gate_keeps_all_domains_preview_only_even_when_evidence_is_ready() {
        let candidates = [
            ready_candidate(UnifiedWriterGateDomain::Memory, "memory:candidate"),
            ready_candidate(UnifiedWriterGateDomain::Genome, "genome:candidate"),
            ready_candidate(
                UnifiedWriterGateDomain::ExperimentLedger,
                "experiment:candidate",
            ),
            ready_candidate(
                UnifiedWriterGateDomain::EvolutionGoalQueue,
                "goal-queue:candidate",
            ),
        ];

        let report = UnifiedWriterGate::new().evaluate(candidates);

        assert_eq!(report.decision, UnifiedWriterGateDecision::PreviewOnly);
        assert_eq!(report.preview_only_records, 4);
        assert_eq!(report.ready_records, 0);
        assert!(report.is_preview_only());
        assert!(
            report
                .records
                .iter()
                .all(|record| record.reason_codes == ["durable_writes_disabled"])
        );
    }

    #[test]
    fn gate_authorizes_only_after_every_required_gate_and_policy_pass() {
        let policy = UnifiedWriterGatePolicy {
            durable_writes_enabled: true,
            ..UnifiedWriterGatePolicy::default()
        };

        let report = UnifiedWriterGate::new()
            .with_policy(policy)
            .evaluate([ready_candidate(
                UnifiedWriterGateDomain::ExperimentLedger,
                "experiment:approved",
            )]);

        assert_eq!(
            report.decision,
            UnifiedWriterGateDecision::ReadyForExplicitApply
        );
        assert_eq!(report.ready_records, 1);
        assert!(report.durable_write_allowed);
        assert!(report.write_allowed);
        assert!(!report.applied);
        assert!(report.explicit_apply_required);
        assert!(report.records[0].reason_codes.is_empty());
    }

    #[test]
    fn gate_holds_missing_evidence_without_granting_writes() {
        let candidate = UnifiedWriterGateCandidate::new(
            UnifiedWriterGateDomain::Memory,
            "memory:missing",
            [UnifiedWriterGateWriteScope::DurableMemory],
        )
        .with_refs(Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new())
        .with_evidence(false, false, false, true, true)
        .with_operator_approval(false, false);

        let report = UnifiedWriterGate::new().evaluate([candidate]);

        assert_eq!(report.decision, UnifiedWriterGateDecision::Hold);
        assert_eq!(report.held_records, 1);
        assert!(!report.write_allowed);
        for reason in [
            "review_packet_refs_missing",
            "validation_evidence_missing_or_failed",
            "trace_or_benchmark_evidence_missing_or_failed",
            "rollback_anchor_missing_or_not_ready",
            "operator_approval_missing",
            "approval_refs_missing_or_mismatched",
            "durable_writes_disabled",
        ] {
            assert!(
                report.records[0].reason_codes.contains(&reason.to_owned()),
                "{:?}",
                report.records[0].reason_codes
            );
        }
    }

    #[test]
    fn gate_rejects_privacy_license_source_write_and_active_flags() {
        let candidate = ready_candidate(UnifiedWriterGateDomain::Genome, "genome:unsafe")
            .with_evidence(true, true, true, false, false)
            .with_source_flags(false, false, true, true, true);

        let report = UnifiedWriterGate::new().evaluate([candidate]);

        assert_eq!(report.decision, UnifiedWriterGateDecision::Reject);
        assert_eq!(report.rejected_records, 1);
        assert!(!report.write_allowed);
        for reason in [
            "privacy_gate_missing_or_failed",
            "license_gate_missing_or_failed",
            "source_not_read_only",
            "source_not_preview_only",
            "source_write_allowed_before_unified_gate",
            "source_already_applied",
            "source_active_before_unified_gate",
        ] {
            assert!(
                report.records[0].reason_codes.contains(&reason.to_owned()),
                "{:?}",
                report.records[0].reason_codes
            );
        }
    }

    #[test]
    fn memory_preview_constructor_maps_review_validation_and_preview_flags() {
        let preview = memory_preview_fixture(true);
        let candidate = UnifiedWriterGateCandidate::memory_admission_preview(&preview);

        assert_eq!(candidate.domain, UnifiedWriterGateDomain::Memory);
        assert_eq!(
            candidate.requested_writes,
            vec![UnifiedWriterGateWriteScope::DurableMemory]
        );
        assert!(candidate.validation_passed);
        assert!(candidate.trace_or_benchmark_passed);
        assert!(candidate.rollback_ready);
        assert!(candidate.privacy_checked);
        assert!(!candidate.operator_approved);
        assert!(candidate.approval_refs_match);
        assert!(candidate.source_preview_only);
        assert!(candidate.raw_payload_redacted);

        let report = UnifiedWriterGate::new().evaluate([candidate]);
        assert_eq!(report.decision, UnifiedWriterGateDecision::Hold);
        assert!(
            report.records[0]
                .reason_codes
                .contains(&"operator_approval_missing".to_owned())
        );
    }

    #[test]
    fn genome_journal_constructor_blocks_unvalidated_or_active_transactions() {
        let mut journal = genome_journal_fixture(false, false);
        journal.transactions[0].active_expression_allowed = true;

        let candidate = UnifiedWriterGateCandidate::genome_transaction_journal(&journal);
        let report = UnifiedWriterGate::new().evaluate([candidate]);

        assert_eq!(report.decision, UnifiedWriterGateDecision::Reject);
        assert!(
            report.records[0]
                .reason_codes
                .contains(&"validation_evidence_missing_or_failed".to_owned())
        );
        assert!(
            report.records[0]
                .reason_codes
                .contains(&"source_active_before_unified_gate".to_owned())
        );
    }

    #[test]
    fn dna_evolution_constructor_holds_until_operator_approval() {
        let report = dna_evolution_report_fixture(GeneScissorsOperatorDecision::Pending);
        let candidate = UnifiedWriterGateCandidate::dna_evolution_controller_report(&report);
        let policy = UnifiedWriterGatePolicy {
            durable_writes_enabled: true,
            ..UnifiedWriterGatePolicy::default()
        };

        assert_eq!(candidate.domain, UnifiedWriterGateDomain::Genome);
        assert_eq!(
            candidate.requested_writes,
            vec![UnifiedWriterGateWriteScope::Genome]
        );
        assert!(candidate.validation_passed);
        assert!(candidate.trace_or_benchmark_passed);
        assert!(candidate.rollback_ready);
        assert!(candidate.privacy_checked);
        assert!(!candidate.operator_approved);
        assert!(candidate.source_preview_only);
        assert!(candidate.raw_payload_redacted);

        let gate = UnifiedWriterGate::new()
            .with_policy(policy)
            .evaluate([candidate]);

        assert_eq!(gate.decision, UnifiedWriterGateDecision::Hold);
        assert_eq!(gate.held_records, 1);
        assert!(!gate.durable_write_allowed);
        assert!(!gate.applied);
        assert!(
            gate.records[0]
                .reason_codes
                .contains(&"operator_approval_missing".to_owned())
        );
        assert!(
            gate.records[0]
                .reason_codes
                .contains(&"approval_refs_missing_or_mismatched".to_owned())
        );
    }

    #[test]
    fn dna_evolution_constructor_ready_for_explicit_apply_after_approval() {
        let report = dna_evolution_report_fixture(GeneScissorsOperatorDecision::Approved);
        let candidate = UnifiedWriterGateCandidate::dna_evolution_controller_report(&report);
        let policy = UnifiedWriterGatePolicy {
            durable_writes_enabled: true,
            ..UnifiedWriterGatePolicy::default()
        };

        assert_eq!(report.activation_eligible_count(), 2);
        assert!(candidate.operator_approved);
        assert!(candidate.approval_refs_match);
        assert!(!candidate.source_write_allowed);
        assert!(!candidate.source_applied);

        let gate = UnifiedWriterGate::new()
            .with_policy(policy)
            .evaluate([candidate]);

        assert_eq!(
            gate.decision,
            UnifiedWriterGateDecision::ReadyForExplicitApply
        );
        assert_eq!(gate.ready_records, 1);
        assert_eq!(gate.genome_records, 1);
        assert!(gate.durable_write_allowed);
        assert!(gate.explicit_apply_required);
        assert!(!gate.applied);
        assert!(gate.records[0].reason_codes.is_empty());
    }

    #[test]
    fn dna_evolution_constructor_blocks_failed_validation() {
        let plans = vec![dna_plan(GeneScissorsIntent::Cut, "malignant-cut")];
        let journal = GeneScissorsTransactionJournal::from_mutation_plans(
            TaskProfile::Coding,
            "stable",
            &plans,
        );
        let report = DnaEvolutionController::default().preview_plans(
            TaskProfile::Coding,
            "parent",
            "stable",
            &plans,
            &DnaEvolutionValidationEvidence::failed_tests(),
            GeneScissorsOperatorDecision::Approved,
            Some(&journal),
        );
        let candidate = UnifiedWriterGateCandidate::dna_evolution_controller_report(&report);
        let policy = UnifiedWriterGatePolicy {
            durable_writes_enabled: true,
            ..UnifiedWriterGatePolicy::default()
        };

        let gate = UnifiedWriterGate::new()
            .with_policy(policy)
            .evaluate([candidate]);

        assert_eq!(gate.decision, UnifiedWriterGateDecision::Hold);
        assert!(!gate.durable_write_allowed);
        assert!(
            gate.records[0]
                .reason_codes
                .contains(&"review_packet_refs_missing".to_owned())
        );
        assert!(
            gate.records[0]
                .reason_codes
                .contains(&"validation_evidence_missing_or_failed".to_owned())
        );
    }

    #[test]
    fn self_evolution_preflight_constructor_preserves_ready_but_read_only_apply_boundary() {
        let preflight = self_evolution_preflight_fixture(true);
        let candidate = UnifiedWriterGateCandidate::experiment_promotion_preflight(&preflight);
        let policy = UnifiedWriterGatePolicy {
            durable_writes_enabled: true,
            ..UnifiedWriterGatePolicy::default()
        };

        let report = UnifiedWriterGate::new()
            .with_policy(policy)
            .evaluate([candidate]);

        assert_eq!(
            report.decision,
            UnifiedWriterGateDecision::ReadyForExplicitApply
        );
        assert_eq!(report.experiment_ledger_records, 1);
        assert!(report.durable_write_allowed);
        assert!(report.explicit_apply_required);
        assert!(!report.applied);
    }

    #[test]
    fn self_goal_queue_preview_constructor_maps_append_packet_to_goal_queue_domain() {
        let preview = self_goal_queue_preview_fixture(true);
        let candidate = UnifiedWriterGateCandidate::self_goal_queue_preview(&preview);

        assert_eq!(
            candidate.domain,
            UnifiedWriterGateDomain::EvolutionGoalQueue
        );
        assert_eq!(
            candidate.requested_writes,
            vec![UnifiedWriterGateWriteScope::EvolutionGoalQueue]
        );
        assert!(candidate.validation_passed);
        assert!(candidate.trace_or_benchmark_passed);
        assert!(candidate.rollback_ready);
        assert!(candidate.privacy_checked);
        assert!(candidate.operator_approved);
        assert!(candidate.approval_refs_match);
        assert!(candidate.source_preview_only);
        assert!(candidate.raw_payload_redacted);

        let report = UnifiedWriterGate::new().evaluate([candidate]);
        assert_eq!(report.decision, UnifiedWriterGateDecision::PreviewOnly);
        assert_eq!(report.evolution_goal_queue_records, 1);
        assert!(report.is_preview_only());
        assert_eq!(report.records[0].reason_codes, ["durable_writes_disabled"]);
    }

    #[test]
    fn self_goal_queue_preview_can_reach_ready_for_explicit_apply_without_applying() {
        let preview = self_goal_queue_preview_fixture(true);
        let candidate = UnifiedWriterGateCandidate::self_goal_queue_preview(&preview);
        let policy = UnifiedWriterGatePolicy {
            durable_writes_enabled: true,
            ..UnifiedWriterGatePolicy::default()
        };

        let report = UnifiedWriterGate::new()
            .with_policy(policy)
            .evaluate([candidate]);

        assert_eq!(
            report.decision,
            UnifiedWriterGateDecision::ReadyForExplicitApply
        );
        assert_eq!(report.evolution_goal_queue_records, 1);
        assert!(report.durable_write_allowed);
        assert!(report.write_allowed);
        assert!(report.explicit_apply_required);
        assert!(!report.applied);
    }

    #[test]
    fn self_goal_queue_preview_holds_without_append_packet() {
        let preview = default_noiron_self_goal_queue_preview_report();
        let candidate = UnifiedWriterGateCandidate::self_goal_queue_preview(&preview);

        let report = UnifiedWriterGate::new().evaluate([candidate]);

        assert_eq!(report.decision, UnifiedWriterGateDecision::Hold);
        assert_eq!(report.evolution_goal_queue_records, 1);
        assert!(!report.write_allowed);
        for reason in [
            "review_packet_refs_missing",
            "validation_evidence_missing_or_failed",
            "trace_or_benchmark_evidence_missing_or_failed",
            "rollback_anchor_missing_or_not_ready",
            "operator_approval_missing",
            "approval_refs_missing_or_mismatched",
            "durable_writes_disabled",
        ] {
            assert!(
                report.records[0].reason_codes.contains(&reason.to_owned()),
                "{:?}",
                report.records[0].reason_codes
            );
        }
    }

    #[test]
    fn self_goal_queue_preview_rejects_source_write_flags() {
        let mut preview = self_goal_queue_preview_fixture(true);
        preview.write_allowed = true;
        let candidate = UnifiedWriterGateCandidate::self_goal_queue_preview(&preview);

        let report = UnifiedWriterGate::new().evaluate([candidate]);

        assert_eq!(report.decision, UnifiedWriterGateDecision::Reject);
        assert_eq!(report.evolution_goal_queue_records, 1);
        assert!(
            report.records[0]
                .reason_codes
                .contains(&"source_write_allowed_before_unified_gate".to_owned())
        );
    }

    fn ready_candidate(
        domain: UnifiedWriterGateDomain,
        candidate_id: &str,
    ) -> UnifiedWriterGateCandidate {
        let scope = match domain {
            UnifiedWriterGateDomain::Memory => UnifiedWriterGateWriteScope::DurableMemory,
            UnifiedWriterGateDomain::Genome => UnifiedWriterGateWriteScope::Genome,
            UnifiedWriterGateDomain::ExperimentLedger => {
                UnifiedWriterGateWriteScope::ExperimentLedger
            }
            UnifiedWriterGateDomain::EvolutionGoalQueue => {
                UnifiedWriterGateWriteScope::EvolutionGoalQueue
            }
        };
        UnifiedWriterGateCandidate::new(domain, candidate_id, [scope])
            .with_refs(
                vec!["review:1".to_owned()],
                vec!["evidence:1".to_owned()],
                vec!["rollback:1".to_owned()],
                vec!["fnv64:content".to_owned()],
                vec!["schema:v1".to_owned()],
            )
            .with_evidence(true, true, true, true, true)
            .with_operator_approval(true, true)
    }

    fn self_goal_queue_preview_fixture(ready: bool) -> SelfGoalQueuePreviewReport {
        if !ready {
            return default_noiron_self_goal_queue_preview_report();
        }

        let queue = EvolutionGoalQueue::new(Vec::new());
        let proposal = default_self_goal_proposal_report(&queue);
        let runs = [passing_run_for_self_goal_candidate(&proposal.candidates[0]).with_approval()];
        let admission = default_self_goal_admission_report(&proposal, &runs);
        default_self_goal_queue_preview_report(&queue, &proposal, &admission)
    }

    fn passing_run_for_self_goal_candidate(
        candidate: &SelfGoalProposalCandidate,
    ) -> EvolutionGoalRunEvidence {
        let evidence = candidate
            .proposed_goal
            .success_gate
            .required_evidence
            .iter()
            .map(|kind| match kind {
                EvolutionGoalEvidenceKind::CargoCheck => EvolutionGoalEvidence::cargo_check(true),
                EvolutionGoalEvidenceKind::FocusedTests => {
                    EvolutionGoalEvidence::focused_tests(true, 3, 0)
                }
                EvolutionGoalEvidenceKind::BenchmarkGate => {
                    EvolutionGoalEvidence::benchmark_gate(true)
                }
                EvolutionGoalEvidenceKind::TraceSchemaGate => {
                    EvolutionGoalEvidence::trace_schema_gate(true)
                }
                EvolutionGoalEvidenceKind::ExperimentLedger => {
                    EvolutionGoalEvidence::experiment_ledger(true)
                }
                EvolutionGoalEvidenceKind::OperatorApproval => {
                    EvolutionGoalEvidence::operator_approval(true)
                }
            })
            .collect::<Vec<_>>();

        EvolutionGoalRunEvidence::new(candidate.proposed_goal.stable_id.clone())
            .with_evidence(evidence)
    }

    fn memory_preview_fixture(read_only_plan: bool) -> MemoryAdmissionPreview {
        let candidate = MemoryAdmissionCandidate {
            id: "memory:candidate".to_owned(),
            kind: MemoryAdmissionKind::RetrospectiveEpisode,
            decision: MemoryAdmissionDecision::Ready,
            profile: TaskProfile::Coding,
            prompt_digest: "fnv64:prompt".to_owned(),
            source_hash: "sha256:source".to_owned(),
            privacy_classification: MemoryPrivacyClassification::DigestOnly,
            prompt_chars: 32,
            quality: 0.90,
            process_reward: 0.88,
            critical_reflection_issues: 0,
            revision_actions: 1,
            runtime_kv_influence: None,
            rollback_anchor_id: "rollback:memory".to_owned(),
            evidence: vec!["quality=0.900".to_owned()],
            validation_evidence: vec!["focused-tests".to_owned()],
            privacy_checked: true,
            durable_write_authorized: false,
            applied: false,
        };
        let packet = MemoryAdmissionReviewPacket {
            packet_id: "review:memory".to_owned(),
            candidate_id: candidate.id.clone(),
            kind: candidate.kind,
            decision: candidate.decision,
            approval_state: MemoryAdmissionApprovalState::PendingApproval,
            rollback_anchor_id: candidate.rollback_anchor_id.clone(),
            source_hash: candidate.source_hash.clone(),
            privacy_classification: candidate.privacy_classification,
            evidence: candidate.evidence.clone(),
            validation_evidence: candidate.validation_evidence.clone(),
            risk_flags: Vec::new(),
            next_action: "review_for_durable_write_gate".to_owned(),
            read_only: true,
            write_allowed: false,
            applied: false,
        };
        let record = MemoryKvLedgerRecord {
            ledger_key: "memory-ledger/episode/source/candidate".to_owned(),
            candidate_id: candidate.id.clone(),
            kind: candidate.kind,
            admission_decision: candidate.decision,
            approval_state: MemoryAdmissionApprovalState::PendingApproval,
            write_decision: MemoryKvLedgerWriteDecision::PreviewOnly,
            source_hash: candidate.source_hash.clone(),
            privacy_classification: candidate.privacy_classification,
            rollback_anchor_id: candidate.rollback_anchor_id.clone(),
            validation_evidence: candidate.validation_evidence.clone(),
            rejection_reasons: vec!["durable_writes_disabled".to_owned()],
            duplicate_of: None,
            merged_into: None,
            append_only: true,
            durable_write_authorized: false,
            applied: false,
        };
        MemoryAdmissionPreview {
            candidates: vec![candidate],
            review_packets: vec![packet],
            ledger_plan: MemoryKvLedgerWritePlan {
                records: vec![record],
                read_only: read_only_plan,
                write_allowed: false,
                applied: false,
            },
            fusion_plan: ReinforcedKvFusionPlan::default(),
            read_only: true,
            write_allowed: false,
            applied: false,
        }
    }

    fn genome_journal_fixture(
        validation_passed: bool,
        operator_approved: bool,
    ) -> GeneScissorsTransactionJournal {
        let transaction = GeneScissorsTransaction {
            schema_version: GENE_SCISSORS_TRANSACTION_SCHEMA_VERSION,
            transaction_id: "tx:gene".to_owned(),
            profile: TaskProfile::Coding,
            state: GeneScissorsTransactionState::CutPreview,
            source_plan_id: "plan:gene".to_owned(),
            target_segment_id: "gene:old".to_owned(),
            replacement_segment_id: Some("gene:new".to_owned()),
            parent_transaction_id: None,
            before_digest: "fnv64:before".to_owned(),
            after_digest: "fnv64:after".to_owned(),
            reason_class: "reversible_cut_candidate".to_owned(),
            evidence_digest: "fnv64:evidence".to_owned(),
            operator_decision: if operator_approved {
                GeneScissorsOperatorDecision::Approved
            } else {
                GeneScissorsOperatorDecision::Pending
            },
            validation_status: if validation_passed {
                GeneValidationStatus::Passed
            } else {
                GeneValidationStatus::Pending
            },
            rollback_anchor_id: "rollback:gene".to_owned(),
            stable_anchor_sources: vec!["stable:gene".to_owned()],
            forensic_copy_digest: "fnv64:forensic".to_owned(),
            lineage_parent_id: None,
            child_lineage_id: Some("lineage:new".to_owned()),
            child_generation: 1,
            active_expression_allowed: false,
            memory_admission_allowed: false,
            read_only: true,
            write_allowed: false,
            applied: false,
        };
        let mut journal = GeneScissorsTransactionJournal::new(TaskProfile::Coding, "stable:gene");
        assert!(journal.append(transaction));
        journal
    }

    fn dna_evolution_report_fixture(
        operator_decision: GeneScissorsOperatorDecision,
    ) -> DnaEvolutionControllerReport {
        let plans = vec![
            dna_plan(GeneScissorsIntent::Relabel, "aged-purpose"),
            dna_plan(GeneScissorsIntent::Repair, "repair-format"),
        ];
        let journal = GeneScissorsTransactionJournal::from_mutation_plans(
            TaskProfile::Coding,
            "stable",
            &plans,
        );
        DnaEvolutionController::default().preview_plans(
            TaskProfile::Coding,
            "parent",
            "stable",
            &plans,
            &DnaEvolutionValidationEvidence::passing(),
            operator_decision,
            Some(&journal),
        )
    }

    fn dna_plan(intent: GeneScissorsIntent, target: &str) -> MutationPlan {
        MutationPlan::preview(
            format!("plan-{target}-{}", intent.as_str()),
            intent,
            target,
            "digest-only reason",
            "digest-only expected effect",
            "stable",
        )
    }

    fn self_evolution_preflight_fixture(ready: bool) -> SelfEvolutionPromotionPreflightReport {
        SelfEvolutionPromotionPreflightReport {
            decision: if ready {
                SelfEvolutionPromotionPreflightDecision::ReadyForExplicitPromotion
            } else {
                SelfEvolutionPromotionPreflightDecision::Hold
            },
            ready_for_explicit_promotion: ready,
            explicit_promotion_required: true,
            candidate_id: "self-evolution:candidate".to_owned(),
            admission_admitted_for_human_review: ready,
            experiment_admitted_for_human_review: ready,
            operator_approved: ready,
            rust_validation_passed: ready,
            validation_passed: ready,
            benchmark_gate_passed: ready,
            adaptive_preview_evidence_present: ready,
            review_packet_count: usize::from(ready),
            evidence_id_count: usize::from(ready),
            rollback_anchor_count: usize::from(ready),
            content_digest_count: usize::from(ready),
            source_report_schema_count: usize::from(ready),
            read_only: true,
            report_only: true,
            preview_only: true,
            activation_write_allowed: false,
            active_candidate: false,
            write_allowed: false,
            applied: false,
            blocked_reasons: Vec::new(),
            content_digest: "fnv64:preflight".to_owned(),
        }
    }
}
