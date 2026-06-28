use std::collections::BTreeSet;

use crate::privacy_redaction::{contains_private_or_executable_marker, stable_redaction_digest};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplicationRepairAction {
    AcceptExactCopy,
    HoldMinorMismatch,
    RejectMajorMismatch,
    QuarantineUnexplainedDrift,
}

impl ReplicationRepairAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AcceptExactCopy => "accept_exact_copy",
            Self::HoldMinorMismatch => "hold_minor_mismatch",
            Self::RejectMajorMismatch => "reject_major_mismatch",
            Self::QuarantineUnexplainedDrift => "quarantine_unexplained_drift",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplicationProofreadInput {
    pub source_record_id: String,
    pub source_digest: String,
    pub copy_digest: String,
    pub parent_lineage_id: String,
    pub expected_target_scope: String,
    pub target_scope: String,
    pub copy_reason: String,
    pub expected_fields: Vec<String>,
    pub copy_fields: Vec<String>,
    pub mutation_budget_delta: i32,
}

impl ReplicationProofreadInput {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        source_record_id: impl Into<String>,
        source_digest: impl Into<String>,
        copy_digest: impl Into<String>,
        parent_lineage_id: impl Into<String>,
        target_scope: impl Into<String>,
        copy_reason: impl Into<String>,
        expected_fields: impl IntoIterator<Item = impl Into<String>>,
        copy_fields: impl IntoIterator<Item = impl Into<String>>,
        mutation_budget_delta: i32,
    ) -> Self {
        let target_scope = target_scope.into();
        Self {
            source_record_id: source_record_id.into(),
            source_digest: source_digest.into(),
            copy_digest: copy_digest.into(),
            parent_lineage_id: parent_lineage_id.into(),
            expected_target_scope: target_scope.clone(),
            target_scope,
            copy_reason: copy_reason.into(),
            expected_fields: expected_fields.into_iter().map(Into::into).collect(),
            copy_fields: copy_fields.into_iter().map(Into::into).collect(),
            mutation_budget_delta,
        }
    }

    pub fn with_expected_target_scope(mut self, expected_target_scope: impl Into<String>) -> Self {
        self.expected_target_scope = expected_target_scope.into();
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplicationProofreadReview {
    pub source_record_id: String,
    pub source_digest: String,
    pub copy_digest: String,
    pub parent_lineage_id: String,
    pub target_scope: String,
    pub copy_reason: String,
    pub expected_fields: Vec<String>,
    pub mismatch_fields: Vec<String>,
    pub mutation_budget_delta: i32,
    pub repair_action: ReplicationRepairAction,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl ReplicationProofreadReview {
    pub fn from_input(input: ReplicationProofreadInput) -> Self {
        let source_digest = normalize_digest(&input.source_digest);
        let copy_digest = normalize_digest(&input.copy_digest);
        let parent_lineage_id = safe_atom(&input.parent_lineage_id, "lineage");
        let target_scope = safe_atom(&input.target_scope, "target_scope");
        let copy_reason = safe_atom(&input.copy_reason, "copy_reason");
        let expected_fields = unique_safe_atoms(input.expected_fields, "field");
        let copy_fields = unique_safe_atoms(input.copy_fields, "field");
        let mismatch_fields = mismatch_fields(
            &source_digest,
            &copy_digest,
            &input.expected_target_scope,
            &input.target_scope,
            &expected_fields,
            &copy_fields,
        );
        let missing_evidence = input.source_digest.trim().is_empty()
            || input.parent_lineage_id.trim().is_empty()
            || input.target_scope.trim().is_empty();
        let target_scope_mismatch = !input.expected_target_scope.trim().is_empty()
            && input.expected_target_scope.trim() != input.target_scope.trim();
        let repair_action = repair_action(
            missing_evidence,
            target_scope_mismatch,
            &copy_reason,
            &mismatch_fields,
        );

        Self {
            source_record_id: safe_atom(&input.source_record_id, "source_record"),
            source_digest,
            copy_digest,
            parent_lineage_id,
            target_scope,
            copy_reason,
            expected_fields,
            mismatch_fields,
            mutation_budget_delta: input.mutation_budget_delta,
            repair_action,
            read_only: true,
            write_allowed: false,
            applied: false,
        }
    }

    pub fn is_preview_only(&self) -> bool {
        self.read_only && !self.write_allowed && !self.applied
    }

    pub fn can_authorize_write(&self) -> bool {
        false
    }

    pub fn summary_line(&self) -> String {
        format!(
            "replication_proofread source={} source_digest_present={} copy_digest_present={} parent_lineage_present={} target_scope={} copy_reason_present={} expected_fields={} mismatch_fields={} mutation_budget_delta={} repair_action={} preview_only={} write_allowed={} applied={}",
            self.source_record_id,
            !self.source_digest.is_empty(),
            !self.copy_digest.is_empty(),
            !self.parent_lineage_id.is_empty(),
            self.target_scope,
            !self.copy_reason.is_empty(),
            self.expected_fields.len(),
            self.mismatch_fields.join("|"),
            self.mutation_budget_delta,
            self.repair_action.as_str(),
            self.is_preview_only(),
            self.write_allowed,
            self.applied
        )
    }
}

fn repair_action(
    missing_evidence: bool,
    target_scope_mismatch: bool,
    copy_reason: &str,
    mismatch_fields: &[String],
) -> ReplicationRepairAction {
    if missing_evidence || target_scope_mismatch {
        return ReplicationRepairAction::RejectMajorMismatch;
    }
    if mismatch_fields.is_empty() {
        return ReplicationRepairAction::AcceptExactCopy;
    }
    if copy_reason.trim().is_empty() {
        return ReplicationRepairAction::QuarantineUnexplainedDrift;
    }
    ReplicationRepairAction::HoldMinorMismatch
}

fn mismatch_fields(
    source_digest: &str,
    copy_digest: &str,
    expected_target_scope: &str,
    target_scope: &str,
    expected_fields: &[String],
    copy_fields: &[String],
) -> Vec<String> {
    let mut mismatches = Vec::new();
    if !source_digest.is_empty() && !copy_digest.is_empty() && source_digest != copy_digest {
        mismatches.push("copy_digest".to_owned());
    }
    if !expected_target_scope.trim().is_empty()
        && expected_target_scope.trim() != target_scope.trim()
    {
        mismatches.push("target_scope".to_owned());
    }

    let copy = copy_fields.iter().collect::<BTreeSet<_>>();
    for expected in expected_fields {
        if !copy.contains(expected) {
            mismatches.push(format!("field:{expected}"));
        }
    }
    mismatches
}

fn unique_safe_atoms(
    values: impl IntoIterator<Item = String>,
    fallback: &'static str,
) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for value in values {
        let value = safe_atom(&value, fallback);
        if seen.insert(value.clone()) {
            out.push(value);
        }
    }
    out
}

fn normalize_digest(value: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        return String::new();
    }
    if value.starts_with("fnv64:")
        || value.starts_with("sha256:")
        || value.starts_with("blake3:")
        || value.starts_with("redaction-digest:")
    {
        return safe_atom(value, "digest");
    }
    stable_redaction_digest(["digest", value])
}

fn safe_atom(value: &str, fallback: &'static str) -> String {
    let value = value.trim();
    if value.is_empty() {
        return String::new();
    }
    if value.len() > 96
        || contains_private_or_executable_marker(value)
        || value.contains('\n')
        || value.contains('\r')
        || value.contains('\t')
        || value.contains(' ')
    {
        return format!("{fallback}:{}", stable_redaction_digest([fallback, value]));
    }
    value.to_owned()
}
