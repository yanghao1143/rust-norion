use crate::hierarchy::TaskProfile;
use crate::privacy_redaction::{contains_private_or_executable_marker, stable_redaction_digest};

use super::model::{GeneScissorsIntent, GeneValidationStatus, MutationPlan};
use super::replication::{ReplicationProofreadInput, ReplicationProofreadReview};
use super::splicing::DnaSplicePreview;

pub const GENE_SCISSORS_TRANSACTION_SCHEMA_VERSION: &str = "gene_scissors_tx_v1";

const JOURNAL_FIELD_COUNT: usize = 25;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeneScissorsTransactionState {
    Quarantine,
    Hold,
    Reject,
    CutPreview,
    RegeneratePreview,
    RollbackPreview,
    Promoted,
}

impl GeneScissorsTransactionState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Quarantine => "quarantine",
            Self::Hold => "hold",
            Self::Reject => "reject",
            Self::CutPreview => "cut_preview",
            Self::RegeneratePreview => "regenerate_preview",
            Self::RollbackPreview => "rollback_preview",
            Self::Promoted => "promoted",
        }
    }

    fn from_intent(intent: GeneScissorsIntent) -> Self {
        match intent {
            GeneScissorsIntent::Quarantine => Self::Quarantine,
            GeneScissorsIntent::Cut => Self::CutPreview,
            GeneScissorsIntent::Regenerate => Self::RegeneratePreview,
            GeneScissorsIntent::Rollback => Self::RollbackPreview,
            GeneScissorsIntent::Repair
            | GeneScissorsIntent::Relabel
            | GeneScissorsIntent::Splice
            | GeneScissorsIntent::Crossover => Self::Hold,
        }
    }

    fn blocks_active_expression(self) -> bool {
        !matches!(self, Self::Promoted)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeneScissorsOperatorDecision {
    Pending,
    Approved,
    Rejected,
}

impl GeneScissorsOperatorDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneScissorsTransaction {
    pub schema_version: &'static str,
    pub transaction_id: String,
    pub profile: TaskProfile,
    pub state: GeneScissorsTransactionState,
    pub source_plan_id: String,
    pub target_segment_id: String,
    pub replacement_segment_id: Option<String>,
    pub parent_transaction_id: Option<String>,
    pub before_digest: String,
    pub after_digest: String,
    pub reason_class: String,
    pub evidence_digest: String,
    pub operator_decision: GeneScissorsOperatorDecision,
    pub validation_status: GeneValidationStatus,
    pub rollback_anchor_id: String,
    pub stable_anchor_sources: Vec<String>,
    pub forensic_copy_digest: String,
    pub lineage_parent_id: Option<String>,
    pub child_lineage_id: Option<String>,
    pub child_generation: u32,
    pub active_expression_allowed: bool,
    pub memory_admission_allowed: bool,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl GeneScissorsTransaction {
    pub fn from_plan(
        profile: TaskProfile,
        stable_anchor_id: impl AsRef<str>,
        plan: &MutationPlan,
    ) -> Self {
        let stable_anchor_id = stable_anchor_id.as_ref();
        let source_plan_id = redacted_ref(&plan.id);
        let target_segment_id = redacted_ref(&plan.target_gene_id);
        let replacement_segment_id = plan.replacement_gene_id.as_deref().map(redacted_ref);
        let rollback_anchor_id = redacted_ref(&plan.rollback_anchor_id);
        let state = GeneScissorsTransactionState::from_intent(plan.intent);
        let mut stable_anchor_sources = normalized_refs(
            plan.source_gene_ids
                .iter()
                .map(String::as_str)
                .chain([stable_anchor_id, plan.rollback_anchor_id.as_str()]),
        );
        if stable_anchor_sources.is_empty() {
            stable_anchor_sources.push(redacted_ref(stable_anchor_id));
        }
        let before_digest = stable_redaction_digest([
            "gene-scissors-before",
            target_segment_id.as_str(),
            rollback_anchor_id.as_str(),
            plan.intent.as_str(),
        ]);
        let evidence_digest = plan_evidence_digest(plan);
        let transaction_id = stable_redaction_digest([
            "gene-scissors-transaction",
            source_plan_id.as_str(),
            target_segment_id.as_str(),
            replacement_segment_id
                .as_deref()
                .unwrap_or("replacement:none"),
            state.as_str(),
            evidence_digest.as_str(),
        ]);
        let after_digest = stable_redaction_digest([
            "gene-scissors-after",
            transaction_id.as_str(),
            state.as_str(),
            replacement_segment_id
                .as_deref()
                .unwrap_or("replacement:none"),
            plan.validation_status.as_str(),
        ]);
        let forensic_copy_digest = stable_redaction_digest([
            "forensic-copy",
            target_segment_id.as_str(),
            before_digest.as_str(),
            evidence_digest.as_str(),
        ]);
        let child_lineage_id = replacement_segment_id.as_deref().map(|replacement| {
            stable_redaction_digest([
                "child-lineage",
                target_segment_id.as_str(),
                replacement,
                transaction_id.as_str(),
            ])
        });

        Self {
            schema_version: GENE_SCISSORS_TRANSACTION_SCHEMA_VERSION,
            transaction_id,
            profile,
            state,
            source_plan_id,
            target_segment_id: target_segment_id.clone(),
            replacement_segment_id,
            parent_transaction_id: None,
            before_digest,
            after_digest,
            reason_class: reason_class_for_plan(plan).to_owned(),
            evidence_digest,
            operator_decision: GeneScissorsOperatorDecision::Pending,
            validation_status: plan.validation_status,
            rollback_anchor_id,
            stable_anchor_sources,
            forensic_copy_digest,
            lineage_parent_id: child_lineage_id.as_ref().map(|_| target_segment_id),
            child_lineage_id,
            child_generation: u32::from(plan.replacement_gene_id.is_some()),
            active_expression_allowed: false,
            memory_admission_allowed: false,
            read_only: true,
            write_allowed: false,
            applied: false,
        }
    }

    pub fn with_state(mut self, state: GeneScissorsTransactionState) -> Self {
        self.state = state;
        self.active_expression_allowed = !state.blocks_active_expression()
            && self.operator_decision == GeneScissorsOperatorDecision::Approved
            && self.validation_status == GeneValidationStatus::Passed;
        self.memory_admission_allowed = self.active_expression_allowed;
        self
    }

    pub fn with_operator_decision(
        mut self,
        operator_decision: GeneScissorsOperatorDecision,
    ) -> Self {
        self.operator_decision = operator_decision;
        if operator_decision == GeneScissorsOperatorDecision::Rejected {
            self.state = GeneScissorsTransactionState::Reject;
            self.active_expression_allowed = false;
            self.memory_admission_allowed = false;
            self.write_allowed = false;
            self.applied = false;
        }
        self
    }

    pub fn is_read_only_preview(&self) -> bool {
        self.read_only && !self.write_allowed && !self.applied
    }

    pub fn replication_proofread_review(
        &self,
        target_scope: impl Into<String>,
        expected_fields: impl IntoIterator<Item = impl Into<String>>,
        copy_fields: impl IntoIterator<Item = impl Into<String>>,
        mutation_budget_delta: i32,
    ) -> ReplicationProofreadReview {
        let parent_lineage_id = self
            .lineage_parent_id
            .as_deref()
            .unwrap_or(&self.rollback_anchor_id);
        ReplicationProofreadReview::from_input(ReplicationProofreadInput::new(
            self.transaction_id.clone(),
            self.before_digest.clone(),
            self.after_digest.clone(),
            parent_lineage_id,
            target_scope,
            self.reason_class.clone(),
            expected_fields,
            copy_fields,
            mutation_budget_delta,
        ))
    }

    pub fn to_journal_line(&self) -> String {
        let fields = [
            self.schema_version.to_owned(),
            self.transaction_id.clone(),
            profile_to_str(self.profile).to_owned(),
            self.state.as_str().to_owned(),
            self.source_plan_id.clone(),
            self.target_segment_id.clone(),
            self.replacement_segment_id.clone().unwrap_or_default(),
            self.parent_transaction_id.clone().unwrap_or_default(),
            self.before_digest.clone(),
            self.after_digest.clone(),
            self.reason_class.clone(),
            self.evidence_digest.clone(),
            self.operator_decision.as_str().to_owned(),
            self.validation_status.as_str().to_owned(),
            self.rollback_anchor_id.clone(),
            serialize_list(&self.stable_anchor_sources),
            self.forensic_copy_digest.clone(),
            self.lineage_parent_id.clone().unwrap_or_default(),
            self.child_lineage_id.clone().unwrap_or_default(),
            self.child_generation.to_string(),
            bool_to_field(self.active_expression_allowed).to_owned(),
            bool_to_field(self.memory_admission_allowed).to_owned(),
            bool_to_field(self.read_only).to_owned(),
            bool_to_field(self.write_allowed).to_owned(),
            bool_to_field(self.applied).to_owned(),
        ];

        fields
            .iter()
            .map(|field| escape_field(field))
            .collect::<Vec<_>>()
            .join("\t")
    }

    pub fn from_journal_line(line: &str) -> Result<Self, GeneScissorsTransactionJournalError> {
        let fields = line.split('\t').map(unescape_field).collect::<Vec<_>>();
        if fields.len() != JOURNAL_FIELD_COUNT
            || fields[0] != GENE_SCISSORS_TRANSACTION_SCHEMA_VERSION
        {
            return Err(GeneScissorsTransactionJournalError::MalformedLine);
        }
        let child_generation = fields[19]
            .parse::<u32>()
            .map_err(|_| GeneScissorsTransactionJournalError::MalformedLine)?;

        Ok(Self {
            schema_version: GENE_SCISSORS_TRANSACTION_SCHEMA_VERSION,
            transaction_id: fields[1].clone(),
            profile: str_to_profile(&fields[2])?,
            state: str_to_state(&fields[3])?,
            source_plan_id: fields[4].clone(),
            target_segment_id: fields[5].clone(),
            replacement_segment_id: non_empty_string(&fields[6]),
            parent_transaction_id: non_empty_string(&fields[7]),
            before_digest: fields[8].clone(),
            after_digest: fields[9].clone(),
            reason_class: fields[10].clone(),
            evidence_digest: fields[11].clone(),
            operator_decision: str_to_operator_decision(&fields[12])?,
            validation_status: str_to_validation_status(&fields[13])?,
            rollback_anchor_id: fields[14].clone(),
            stable_anchor_sources: deserialize_list(&fields[15]),
            forensic_copy_digest: fields[16].clone(),
            lineage_parent_id: non_empty_string(&fields[17]),
            child_lineage_id: non_empty_string(&fields[18]),
            child_generation,
            active_expression_allowed: field_to_bool(&fields[20])?,
            memory_admission_allowed: field_to_bool(&fields[21])?,
            read_only: field_to_bool(&fields[22])?,
            write_allowed: field_to_bool(&fields[23])?,
            applied: field_to_bool(&fields[24])?,
        })
    }

    pub fn to_redacted_trace_line(&self) -> String {
        format!(
            "{} tx={} state={} target={} replacement={} before={} after={} evidence={} reason={} operator={} validation={} rollback={} active_expression_allowed={} memory_admission_allowed={} write_allowed={} applied={}",
            self.schema_version,
            self.transaction_id,
            self.state.as_str(),
            stable_redaction_digest([self.target_segment_id.as_str()]),
            self.replacement_segment_id
                .as_deref()
                .map(|replacement| stable_redaction_digest([replacement]))
                .unwrap_or_else(|| "none".to_owned()),
            self.before_digest,
            self.after_digest,
            self.evidence_digest,
            self.reason_class,
            self.operator_decision.as_str(),
            self.validation_status.as_str(),
            stable_redaction_digest([self.rollback_anchor_id.as_str()]),
            self.active_expression_allowed,
            self.memory_admission_allowed,
            self.write_allowed,
            self.applied
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneScissorsTransactionJournal {
    pub schema_version: &'static str,
    pub profile: TaskProfile,
    pub stable_anchor_id: String,
    pub transactions: Vec<GeneScissorsTransaction>,
    pub duplicate_transaction_ids: Vec<String>,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl GeneScissorsTransactionJournal {
    pub fn new(profile: TaskProfile, stable_anchor_id: impl Into<String>) -> Self {
        Self {
            schema_version: GENE_SCISSORS_TRANSACTION_SCHEMA_VERSION,
            profile,
            stable_anchor_id: redacted_ref(&stable_anchor_id.into()),
            transactions: Vec::new(),
            duplicate_transaction_ids: Vec::new(),
            read_only: true,
            write_allowed: false,
            applied: false,
        }
    }

    pub fn from_splice_preview(preview: &DnaSplicePreview) -> Self {
        Self::from_mutation_plans(
            preview.profile,
            preview.stable_anchor_id.clone(),
            &preview.mutation_plans,
        )
    }

    pub fn from_mutation_plans(
        profile: TaskProfile,
        stable_anchor_id: impl Into<String>,
        plans: &[MutationPlan],
    ) -> Self {
        let stable_anchor_id = stable_anchor_id.into();
        let mut journal = Self::new(profile, stable_anchor_id.clone());
        for plan in plans {
            journal.append(GeneScissorsTransaction::from_plan(
                profile,
                stable_anchor_id.as_str(),
                plan,
            ));
        }
        journal
    }

    pub fn append(&mut self, transaction: GeneScissorsTransaction) -> bool {
        if self
            .transactions
            .iter()
            .any(|existing| existing.transaction_id == transaction.transaction_id)
        {
            push_once(
                &mut self.duplicate_transaction_ids,
                transaction.transaction_id,
            );
            return false;
        }
        self.transactions.push(transaction);
        true
    }

    pub fn replay(&self) -> GeneScissorsTransactionReplayReport {
        let mut active_expression_excluded_segments = Vec::new();
        let mut forensic_copy_digests = Vec::new();
        let mut child_lineage_ids = Vec::new();
        let mut quarantine_count = 0;
        let mut cut_preview_count = 0;
        let mut regenerate_preview_count = 0;
        let mut rollback_preview_count = 0;

        for transaction in &self.transactions {
            if transaction.state.blocks_active_expression()
                || !transaction.active_expression_allowed
            {
                push_once(
                    &mut active_expression_excluded_segments,
                    transaction.target_segment_id.clone(),
                );
            }
            push_once(
                &mut forensic_copy_digests,
                transaction.forensic_copy_digest.clone(),
            );
            if let Some(child_lineage_id) = &transaction.child_lineage_id {
                push_once(&mut child_lineage_ids, child_lineage_id.clone());
            }
            match transaction.state {
                GeneScissorsTransactionState::Quarantine => quarantine_count += 1,
                GeneScissorsTransactionState::CutPreview => cut_preview_count += 1,
                GeneScissorsTransactionState::RegeneratePreview => regenerate_preview_count += 1,
                GeneScissorsTransactionState::RollbackPreview => rollback_preview_count += 1,
                GeneScissorsTransactionState::Hold
                | GeneScissorsTransactionState::Reject
                | GeneScissorsTransactionState::Promoted => {}
            }
        }

        GeneScissorsTransactionReplayReport {
            transaction_count: self.transactions.len(),
            duplicate_suppressed_count: self.duplicate_transaction_ids.len(),
            quarantine_count,
            cut_preview_count,
            regenerate_preview_count,
            rollback_preview_count,
            active_expression_excluded_segments,
            forensic_copy_digests,
            child_lineage_ids,
            read_only: self.is_read_only_preview(),
            write_allowed: self.write_allowed,
            applied: self.applied,
        }
    }

    pub fn to_journal_lines(&self) -> Vec<String> {
        self.transactions
            .iter()
            .map(GeneScissorsTransaction::to_journal_line)
            .collect()
    }

    pub fn from_journal_lines(
        lines: &[String],
    ) -> Result<Self, GeneScissorsTransactionJournalError> {
        if lines.is_empty() {
            return Err(GeneScissorsTransactionJournalError::EmptyJournal);
        }
        let mut transactions = Vec::with_capacity(lines.len());
        for line in lines {
            transactions.push(GeneScissorsTransaction::from_journal_line(line)?);
        }
        let profile = transactions[0].profile;
        let stable_anchor_id = journal_stable_anchor_id(&transactions);
        let mut journal = Self::new(profile, stable_anchor_id);
        for transaction in transactions {
            if transaction.profile != profile {
                return Err(GeneScissorsTransactionJournalError::ProfileMismatch);
            }
            journal.append(transaction);
        }
        Ok(journal)
    }

    pub fn to_redacted_trace_lines(&self) -> Vec<String> {
        self.transactions
            .iter()
            .map(GeneScissorsTransaction::to_redacted_trace_line)
            .collect()
    }

    pub fn exports_are_redacted(&self) -> bool {
        self.to_redacted_trace_lines()
            .iter()
            .all(|line| !contains_private_or_executable_marker(line))
    }

    pub fn is_read_only_preview(&self) -> bool {
        self.read_only
            && !self.write_allowed
            && !self.applied
            && self
                .transactions
                .iter()
                .all(GeneScissorsTransaction::is_read_only_preview)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneScissorsTransactionReplayReport {
    pub transaction_count: usize,
    pub duplicate_suppressed_count: usize,
    pub quarantine_count: usize,
    pub cut_preview_count: usize,
    pub regenerate_preview_count: usize,
    pub rollback_preview_count: usize,
    pub active_expression_excluded_segments: Vec<String>,
    pub forensic_copy_digests: Vec<String>,
    pub child_lineage_ids: Vec<String>,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl GeneScissorsTransactionReplayReport {
    pub fn passed_preview_gate(&self) -> bool {
        self.read_only && !self.write_allowed && !self.applied
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GeneScissorsTransactionJournalError {
    EmptyJournal,
    MalformedLine,
    UnsupportedState,
    UnsupportedOperatorDecision,
    UnsupportedValidationStatus,
    UnsupportedProfile,
    ProfileMismatch,
    InvalidBool,
}

fn plan_evidence_digest(plan: &MutationPlan) -> String {
    let mut parts = vec![
        plan.id.as_str(),
        plan.target_gene_id.as_str(),
        plan.intent.as_str(),
        plan.reason.as_str(),
        plan.expected_effect.as_str(),
        plan.rollback_anchor_id.as_str(),
    ];
    for gate in &plan.validation_gates {
        parts.push(gate.as_str());
    }
    for evidence in &plan.source_evidence {
        parts.push(evidence.source_id.as_str());
        parts.push(evidence.summary.as_str());
    }
    stable_redaction_digest(parts)
}

fn reason_class_for_plan(plan: &MutationPlan) -> &'static str {
    match plan.intent {
        GeneScissorsIntent::Quarantine => "quarantine_isolation",
        GeneScissorsIntent::Cut => "reversible_cut_candidate",
        GeneScissorsIntent::Regenerate => "stable_anchor_regeneration",
        GeneScissorsIntent::Rollback => "rollback_preview",
        GeneScissorsIntent::Splice => "splice_repair",
        GeneScissorsIntent::Relabel => "purpose_relabel",
        GeneScissorsIntent::Repair => "metadata_repair",
        GeneScissorsIntent::Crossover => "crossover_preview",
    }
}

fn normalized_refs<'a>(values: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    let mut refs = Vec::new();
    for value in values {
        let value = redacted_ref(value);
        if value.trim().is_empty() {
            continue;
        }
        push_once(&mut refs, value);
    }
    refs.sort();
    refs
}

fn redacted_ref(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.len() > 96
        || contains_private_or_executable_marker(trimmed)
        || trimmed.contains('\n')
        || trimmed.contains('\r')
        || trimmed.contains('\t')
    {
        return stable_redaction_digest(["redacted-ref", trimmed]);
    }
    trimmed.to_owned()
}

fn serialize_list(values: &[String]) -> String {
    values
        .iter()
        .map(|value| escape_field(value))
        .collect::<Vec<_>>()
        .join("|")
}

fn deserialize_list(value: &str) -> Vec<String> {
    if value.trim().is_empty() {
        return Vec::new();
    }
    value.split('|').map(unescape_field).collect()
}

fn non_empty_string(value: &str) -> Option<String> {
    (!value.trim().is_empty()).then(|| value.to_owned())
}

fn journal_stable_anchor_id(transactions: &[GeneScissorsTransaction]) -> String {
    let rollback_anchors = transactions
        .iter()
        .map(|transaction| transaction.rollback_anchor_id.as_str())
        .collect::<Vec<_>>();
    let common_sources = transactions[0]
        .stable_anchor_sources
        .iter()
        .filter(|source| {
            transactions
                .iter()
                .all(|transaction| transaction.stable_anchor_sources.contains(source))
        })
        .collect::<Vec<_>>();

    if let Some(source) = common_sources.iter().copied().find(|source| {
        !rollback_anchors.contains(&source.as_str()) && looks_like_stable_anchor(source)
    }) {
        return source.clone();
    }
    if let Some(source) = common_sources.iter().copied().find(|source| {
        rollback_anchors.contains(&source.as_str()) && looks_like_stable_anchor(source)
    }) {
        return source.clone();
    }
    transactions
        .first()
        .and_then(|transaction| transaction.stable_anchor_sources.first().cloned())
        .unwrap_or_else(|| transactions[0].rollback_anchor_id.clone())
}

fn looks_like_stable_anchor(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    value.contains("stable") || value.contains("anchor")
}

fn escape_field(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('|', "\\p")
}

fn unescape_field(value: &str) -> String {
    let mut out = String::new();
    let mut chars = value.chars();

    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }

        match chars.next() {
            Some('t') => out.push('\t'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('p') => out.push('|'),
            Some('\\') => out.push('\\'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }

    out
}

fn bool_to_field(value: bool) -> &'static str {
    if value { "1" } else { "0" }
}

fn field_to_bool(value: &str) -> Result<bool, GeneScissorsTransactionJournalError> {
    match value {
        "1" | "true" => Ok(true),
        "0" | "false" => Ok(false),
        _ => Err(GeneScissorsTransactionJournalError::InvalidBool),
    }
}

fn profile_to_str(profile: TaskProfile) -> &'static str {
    match profile {
        TaskProfile::General => "general",
        TaskProfile::Coding => "coding",
        TaskProfile::Writing => "writing",
        TaskProfile::LongDocument => "long_document",
    }
}

fn str_to_profile(value: &str) -> Result<TaskProfile, GeneScissorsTransactionJournalError> {
    match value {
        "general" => Ok(TaskProfile::General),
        "coding" => Ok(TaskProfile::Coding),
        "writing" => Ok(TaskProfile::Writing),
        "long_document" => Ok(TaskProfile::LongDocument),
        _ => Err(GeneScissorsTransactionJournalError::UnsupportedProfile),
    }
}

fn str_to_state(
    value: &str,
) -> Result<GeneScissorsTransactionState, GeneScissorsTransactionJournalError> {
    match value {
        "quarantine" => Ok(GeneScissorsTransactionState::Quarantine),
        "hold" => Ok(GeneScissorsTransactionState::Hold),
        "reject" => Ok(GeneScissorsTransactionState::Reject),
        "cut_preview" => Ok(GeneScissorsTransactionState::CutPreview),
        "regenerate_preview" => Ok(GeneScissorsTransactionState::RegeneratePreview),
        "rollback_preview" => Ok(GeneScissorsTransactionState::RollbackPreview),
        "promoted" => Ok(GeneScissorsTransactionState::Promoted),
        _ => Err(GeneScissorsTransactionJournalError::UnsupportedState),
    }
}

fn str_to_operator_decision(
    value: &str,
) -> Result<GeneScissorsOperatorDecision, GeneScissorsTransactionJournalError> {
    match value {
        "pending" => Ok(GeneScissorsOperatorDecision::Pending),
        "approved" => Ok(GeneScissorsOperatorDecision::Approved),
        "rejected" => Ok(GeneScissorsOperatorDecision::Rejected),
        _ => Err(GeneScissorsTransactionJournalError::UnsupportedOperatorDecision),
    }
}

fn str_to_validation_status(
    value: &str,
) -> Result<GeneValidationStatus, GeneScissorsTransactionJournalError> {
    match value {
        "not_required" => Ok(GeneValidationStatus::NotRequired),
        "pending" => Ok(GeneValidationStatus::Pending),
        "passed" => Ok(GeneValidationStatus::Passed),
        "failed" => Ok(GeneValidationStatus::Failed),
        _ => Err(GeneScissorsTransactionJournalError::UnsupportedValidationStatus),
    }
}

fn push_once(values: &mut Vec<String>, value: String) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}
