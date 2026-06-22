use crate::hierarchy::TaskProfile;

use super::model::{GeneScissorsIntent, GeneValidationStatus, MutationPlan};
use super::schema::{DnaGeneChain, DnaGeneRecord};
use super::splicing::{
    ClassifiedGeneSegment, DnaSplicePreview, GeneScissorsLifecycleRecord, GeneSegment,
    MutationFinding,
};

pub const DNA_LINEAGE_AUDIT_SCHEMA_VERSION: &str = "dna_lineage_audit_v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DnaLineageAuditNodeKind {
    RollbackAnchor,
    OriginalSegment,
    GeneRecord,
    MutationPlan,
    RepairPreview,
    ApprovedRepair,
    LifecycleRecord,
}

impl DnaLineageAuditNodeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RollbackAnchor => "rollback_anchor",
            Self::OriginalSegment => "original_segment",
            Self::GeneRecord => "gene_record",
            Self::MutationPlan => "mutation_plan",
            Self::RepairPreview => "repair_preview",
            Self::ApprovedRepair => "approved_repair",
            Self::LifecycleRecord => "lifecycle_record",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DnaLineageRepairState {
    NotApplicable,
    PreviewOnly,
    ApprovedPendingApply,
    ApprovedApplied,
    AppliedWithoutWriteGate,
}

impl DnaLineageRepairState {
    pub fn from_flags(admission_write_authorized: bool, applied: bool) -> Self {
        match (admission_write_authorized, applied) {
            (false, false) => Self::PreviewOnly,
            (true, false) => Self::ApprovedPendingApply,
            (true, true) => Self::ApprovedApplied,
            (false, true) => Self::AppliedWithoutWriteGate,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotApplicable => "not_applicable",
            Self::PreviewOnly => "preview_only",
            Self::ApprovedPendingApply => "approved_pending_apply",
            Self::ApprovedApplied => "approved_applied",
            Self::AppliedWithoutWriteGate => "applied_without_write_gate",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnaLineageAuditNode {
    pub id: String,
    pub kind: DnaLineageAuditNodeKind,
    pub source_ref: String,
    pub chain_kind: Option<String>,
    pub before_digest: String,
    pub after_digest: String,
    pub reason_codes: Vec<String>,
    pub gate_status: String,
    pub rollback_anchor_id: String,
    pub repair_state: DnaLineageRepairState,
    pub redacted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnaLineageAuditEdge {
    pub parent_id: String,
    pub child_id: String,
    pub relation: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnaLineageAuditPacket {
    pub schema_version: &'static str,
    pub profile: TaskProfile,
    pub stable_anchor_id: String,
    pub stable_anchor_digest: String,
    pub nodes: Vec<DnaLineageAuditNode>,
    pub edges: Vec<DnaLineageAuditEdge>,
    pub redacted: bool,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl DnaLineageAuditPacket {
    pub fn from_splice_preview(preview: &DnaSplicePreview) -> Self {
        let (anchor_ref, anchor_was_redacted) = redacted_ref(&preview.stable_anchor_id);
        let stable_anchor_digest = stable_digest(["rollback_anchor", &preview.stable_anchor_id]);
        let anchor_node_id = rollback_anchor_node_id(&anchor_ref);
        let mut nodes = vec![DnaLineageAuditNode {
            id: anchor_node_id.clone(),
            kind: DnaLineageAuditNodeKind::RollbackAnchor,
            source_ref: anchor_ref.clone(),
            chain_kind: None,
            before_digest: stable_anchor_digest.clone(),
            after_digest: stable_anchor_digest.clone(),
            reason_codes: vec!["rollback_anchor".to_owned()],
            gate_status: "anchor_available".to_owned(),
            rollback_anchor_id: anchor_ref.clone(),
            repair_state: DnaLineageRepairState::NotApplicable,
            redacted: anchor_was_redacted,
        }];
        let mut edges = Vec::new();

        for segment in &preview.segments {
            let node = segment_node(segment, &preview.findings, &anchor_ref);
            edges.push(DnaLineageAuditEdge {
                parent_id: anchor_node_id.clone(),
                child_id: node.id.clone(),
                relation: "anchors_original_segment".to_owned(),
            });
            nodes.push(node);
        }

        for plan in &preview.mutation_plans {
            let node = mutation_plan_node(plan, &preview.segments, &preview.findings);
            let target_node_id = segment_node_id_for_raw(&plan.target_gene_id);
            edges.push(DnaLineageAuditEdge {
                parent_id: target_node_id,
                child_id: node.id.clone(),
                relation: "proposes_mutation".to_owned(),
            });

            if let Some(replacement_gene_id) = &plan.replacement_gene_id {
                let replacement = replacement_node(plan, replacement_gene_id);
                edges.push(DnaLineageAuditEdge {
                    parent_id: node.id.clone(),
                    child_id: replacement.id.clone(),
                    relation: "produces_replacement_candidate".to_owned(),
                });
                nodes.push(replacement);
            }

            nodes.push(node);
        }

        for record in &preview.lifecycle_records {
            let node = lifecycle_node(record);
            edges.push(DnaLineageAuditEdge {
                parent_id: segment_node_id_for_raw(&record.target_segment_id),
                child_id: node.id.clone(),
                relation: "tracks_lifecycle".to_owned(),
            });
            nodes.push(node);
        }

        Self {
            schema_version: DNA_LINEAGE_AUDIT_SCHEMA_VERSION,
            profile: preview.profile,
            stable_anchor_id: anchor_ref,
            stable_anchor_digest,
            nodes,
            edges,
            redacted: true,
            read_only: preview.read_only,
            write_allowed: preview.write_allowed,
            applied: preview.applied,
        }
    }

    pub fn from_gene_chain(chain: &DnaGeneChain) -> Self {
        let (anchor_ref, anchor_was_redacted) = redacted_ref(&chain.stable_anchor_id);
        let stable_anchor_digest = stable_digest(["rollback_anchor", &chain.stable_anchor_id]);
        let anchor_node_id = rollback_anchor_node_id(&anchor_ref);
        let mut nodes = vec![DnaLineageAuditNode {
            id: anchor_node_id.clone(),
            kind: DnaLineageAuditNodeKind::RollbackAnchor,
            source_ref: anchor_ref.clone(),
            chain_kind: None,
            before_digest: stable_anchor_digest.clone(),
            after_digest: stable_anchor_digest.clone(),
            reason_codes: vec!["rollback_anchor".to_owned()],
            gate_status: "anchor_available".to_owned(),
            rollback_anchor_id: anchor_ref.clone(),
            repair_state: DnaLineageRepairState::NotApplicable,
            redacted: anchor_was_redacted,
        }];
        let mut edges = Vec::new();

        for record in chain.express_chain.iter().chain(chain.memory_chain.iter()) {
            let node = gene_record_node(record);
            let parent_id = record
                .lineage
                .parent_gene_id
                .as_deref()
                .map(gene_node_id_for_raw)
                .unwrap_or_else(|| anchor_node_id.clone());
            let relation = if record.lineage.parent_gene_id.is_some() {
                "parent_gene"
            } else {
                "inherited_from_anchor"
            };
            edges.push(DnaLineageAuditEdge {
                parent_id,
                child_id: node.id.clone(),
                relation: relation.to_owned(),
            });
            nodes.push(node);
        }

        Self {
            schema_version: DNA_LINEAGE_AUDIT_SCHEMA_VERSION,
            profile: chain.profile,
            stable_anchor_id: anchor_ref,
            stable_anchor_digest,
            nodes,
            edges,
            redacted: true,
            read_only: chain.read_only,
            write_allowed: chain.write_allowed,
            applied: chain
                .express_chain
                .iter()
                .chain(chain.memory_chain.iter())
                .any(|record| record.applied),
        }
    }

    pub fn node(&self, id: &str) -> Option<&DnaLineageAuditNode> {
        self.nodes.iter().find(|node| node.id == id)
    }

    pub fn nodes_for_source(&self, source_ref: &str) -> Vec<&DnaLineageAuditNode> {
        self.nodes
            .iter()
            .filter(|node| node.source_ref == source_ref)
            .collect()
    }

    pub fn edges_touching(&self, node_id: &str) -> Vec<&DnaLineageAuditEdge> {
        self.edges
            .iter()
            .filter(|edge| edge.parent_id == node_id || edge.child_id == node_id)
            .collect()
    }

    pub fn to_redacted_json(&self) -> String {
        let mut out = String::new();
        out.push('{');
        push_json_field(&mut out, "schema_version", self.schema_version);
        push_json_field(&mut out, "profile", profile_to_str(self.profile));
        push_json_field(&mut out, "stable_anchor_id", &self.stable_anchor_id);
        push_json_field(&mut out, "stable_anchor_digest", &self.stable_anchor_digest);
        push_json_bool_field(&mut out, "redacted", self.redacted);
        push_json_bool_field(&mut out, "read_only", self.read_only);
        push_json_bool_field(&mut out, "write_allowed", self.write_allowed);
        push_json_bool_field(&mut out, "applied", self.applied);
        out.push_str(",\"nodes\":[");
        for (index, node) in self.nodes.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            out.push_str(&node.to_redacted_json());
        }
        out.push_str("],\"edges\":[");
        for (index, edge) in self.edges.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            out.push_str(&edge.to_redacted_json());
        }
        out.push_str("]}");
        out
    }

    pub fn to_redacted_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str("# Reasoning Genome Lineage Audit\n\n");
        out.push_str(&format!(
            "- schema: {}\n- profile: {}\n- stable_anchor_digest: {}\n- redacted: {}\n\n",
            self.schema_version,
            profile_to_str(self.profile),
            self.stable_anchor_digest,
            self.redacted
        ));
        out.push_str(
            "| node | kind | before | after | reason_codes | gate | rollback | repair_state |\n",
        );
        out.push_str("| --- | --- | --- | --- | --- | --- | --- | --- |\n");
        for node in &self.nodes {
            out.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} | {} | {} |\n",
                markdown_cell(&node.id),
                node.kind.as_str(),
                node.before_digest,
                node.after_digest,
                markdown_cell(&node.reason_codes.join("|")),
                markdown_cell(&node.gate_status),
                markdown_cell(&node.rollback_anchor_id),
                node.repair_state.as_str()
            ));
        }
        out.push_str("\n| parent | child | relation |\n");
        out.push_str("| --- | --- | --- |\n");
        for edge in &self.edges {
            out.push_str(&format!(
                "| {} | {} | {} |\n",
                markdown_cell(&edge.parent_id),
                markdown_cell(&edge.child_id),
                markdown_cell(&edge.relation)
            ));
        }
        out
    }

    pub fn exports_are_redacted(&self) -> bool {
        let json = self.to_redacted_json();
        let markdown = self.to_redacted_markdown();
        !contains_blocked_payload_marker(&json) && !contains_blocked_payload_marker(&markdown)
    }
}

impl DnaLineageAuditNode {
    fn to_redacted_json(&self) -> String {
        let mut out = String::new();
        out.push('{');
        push_json_field(&mut out, "id", &self.id);
        push_json_field(&mut out, "kind", self.kind.as_str());
        push_json_field(&mut out, "source_ref", &self.source_ref);
        if let Some(chain_kind) = &self.chain_kind {
            push_json_field(&mut out, "chain_kind", chain_kind);
        } else {
            out.push_str(",\"chain_kind\":null");
        }
        push_json_field(&mut out, "before_digest", &self.before_digest);
        push_json_field(&mut out, "after_digest", &self.after_digest);
        out.push_str(",\"reason_codes\":");
        out.push_str(&json_string_array(&self.reason_codes));
        push_json_field(&mut out, "gate_status", &self.gate_status);
        push_json_field(&mut out, "rollback_anchor_id", &self.rollback_anchor_id);
        push_json_field(&mut out, "repair_state", self.repair_state.as_str());
        push_json_bool_field(&mut out, "redacted", self.redacted);
        out.push('}');
        out
    }
}

impl DnaLineageAuditEdge {
    fn to_redacted_json(&self) -> String {
        let mut out = String::new();
        out.push('{');
        push_json_field(&mut out, "parent_id", &self.parent_id);
        push_json_field(&mut out, "child_id", &self.child_id);
        push_json_field(&mut out, "relation", &self.relation);
        out.push('}');
        out
    }
}

fn segment_node(
    classified: &ClassifiedGeneSegment,
    findings: &[MutationFinding],
    rollback_anchor_id: &str,
) -> DnaLineageAuditNode {
    let (source_ref, redacted) = redacted_ref(&classified.segment.id);
    let before_digest = segment_digest(&classified.segment);
    let reason_codes = finding_codes_for_segment(findings, &classified.segment.id);
    let after_digest = stable_digest([
        before_digest.as_str(),
        classified.class.as_str(),
        classified.disposition.as_str(),
        &reason_codes.join("|"),
    ]);

    DnaLineageAuditNode {
        id: segment_node_id_for_ref(&source_ref),
        kind: DnaLineageAuditNodeKind::OriginalSegment,
        source_ref,
        chain_kind: None,
        before_digest,
        after_digest,
        reason_codes,
        gate_status: classified.disposition.as_str().to_owned(),
        rollback_anchor_id: rollback_anchor_id.to_owned(),
        repair_state: DnaLineageRepairState::NotApplicable,
        redacted,
    }
}

fn mutation_plan_node(
    plan: &MutationPlan,
    segments: &[ClassifiedGeneSegment],
    findings: &[MutationFinding],
) -> DnaLineageAuditNode {
    let (source_ref, redacted) = redacted_ref(&plan.id);
    let before_digest = segments
        .iter()
        .find(|segment| segment.segment.id == plan.target_gene_id)
        .map(|segment| segment_digest(&segment.segment))
        .unwrap_or_else(|| stable_digest(["missing_target", &plan.target_gene_id]));
    let repair_state =
        DnaLineageRepairState::from_flags(plan.admission_write_authorized, plan.applied);
    let reason_codes = plan_reason_codes(plan, findings);

    DnaLineageAuditNode {
        id: mutation_plan_node_id_for_ref(&source_ref),
        kind: DnaLineageAuditNodeKind::MutationPlan,
        source_ref,
        chain_kind: None,
        before_digest,
        after_digest: plan_digest(plan),
        reason_codes,
        gate_status: plan.validation_status.as_str().to_owned(),
        rollback_anchor_id: redacted_ref(&plan.rollback_anchor_id).0,
        repair_state,
        redacted,
    }
}

fn replacement_node(plan: &MutationPlan, replacement_gene_id: &str) -> DnaLineageAuditNode {
    let (source_ref, redacted) = redacted_ref(replacement_gene_id);
    let repair_state =
        DnaLineageRepairState::from_flags(plan.admission_write_authorized, plan.applied);
    let kind = if repair_state == DnaLineageRepairState::ApprovedApplied {
        DnaLineageAuditNodeKind::ApprovedRepair
    } else {
        DnaLineageAuditNodeKind::RepairPreview
    };
    let before_digest = plan_digest(plan);
    let after_digest = stable_digest([
        "replacement",
        replacement_gene_id,
        plan.intent.as_str(),
        plan.validation_status.as_str(),
        repair_state.as_str(),
    ]);

    DnaLineageAuditNode {
        id: replacement_node_id_for_ref(&source_ref),
        kind,
        source_ref,
        chain_kind: None,
        before_digest,
        after_digest,
        reason_codes: plan_reason_codes(plan, &[]),
        gate_status: plan.validation_status.as_str().to_owned(),
        rollback_anchor_id: redacted_ref(&plan.rollback_anchor_id).0,
        repair_state,
        redacted,
    }
}

fn lifecycle_node(record: &GeneScissorsLifecycleRecord) -> DnaLineageAuditNode {
    let (source_ref, redacted) = redacted_ref(&record.id);
    let reason_codes = record
        .finding_kinds
        .iter()
        .map(|kind| kind.as_str().to_owned())
        .collect::<Vec<_>>();
    let before_digest = stable_digest([
        "lifecycle",
        &record.target_segment_id,
        record.state.as_str(),
        &record.finding_ids.join("|"),
    ]);
    let after_digest = stable_digest([
        before_digest.as_str(),
        record.validation_status.as_str(),
        &record.mutation_plan_ids.join("|"),
        bool_field(record.admission_write_authorized),
        bool_field(record.applied),
    ]);

    DnaLineageAuditNode {
        id: lifecycle_node_id_for_ref(&source_ref),
        kind: DnaLineageAuditNodeKind::LifecycleRecord,
        source_ref,
        chain_kind: None,
        before_digest,
        after_digest,
        reason_codes,
        gate_status: record.validation_status.as_str().to_owned(),
        rollback_anchor_id: redacted_ref(&record.rollback_anchor_id).0,
        repair_state: DnaLineageRepairState::from_flags(
            record.admission_write_authorized,
            record.applied,
        ),
        redacted,
    }
}

fn gene_record_node(record: &DnaGeneRecord) -> DnaLineageAuditNode {
    let (source_ref, redacted) = redacted_ref(&record.gene_id);
    let before_digest = gene_record_digest(record);
    let repair_state =
        DnaLineageRepairState::from_flags(record.admission_write_authorized, record.applied);
    let after_digest = stable_digest([
        before_digest.as_str(),
        record.status.as_str(),
        record.chain_kind.as_str(),
        record.source_evidence.kind.as_str(),
        repair_state.as_str(),
    ]);

    DnaLineageAuditNode {
        id: gene_node_id_for_ref(&source_ref),
        kind: DnaLineageAuditNodeKind::GeneRecord,
        source_ref,
        chain_kind: Some(record.chain_kind.as_str().to_owned()),
        before_digest,
        after_digest,
        reason_codes: vec![
            record.chain_kind.as_str().to_owned(),
            record.status.as_str().to_owned(),
            record.source_evidence.kind.as_str().to_owned(),
        ],
        gate_status: gene_record_gate_status(record).to_owned(),
        rollback_anchor_id: redacted_ref(&record.rollback_anchor_id).0,
        repair_state,
        redacted,
    }
}

fn finding_codes_for_segment(findings: &[MutationFinding], segment_id: &str) -> Vec<String> {
    let mut codes = Vec::new();
    for finding in findings
        .iter()
        .filter(|finding| finding.segment_id == segment_id)
    {
        push_code_once(&mut codes, finding.kind.as_str());
    }
    if codes.is_empty() {
        codes.push("none".to_owned());
    }
    codes
}

fn plan_reason_codes(plan: &MutationPlan, findings: &[MutationFinding]) -> Vec<String> {
    let mut codes = vec![format!("intent:{}", plan.intent.as_str())];
    for finding in findings
        .iter()
        .filter(|finding| finding.segment_id == plan.target_gene_id)
    {
        push_code_once(&mut codes, finding.kind.as_str());
    }
    if plan.validation_status != GeneValidationStatus::Pending {
        push_code_once(&mut codes, plan.validation_status.as_str());
    }
    if plan.intent == GeneScissorsIntent::Regenerate && plan.replacement_gene_id.is_some() {
        push_code_once(&mut codes, "replacement_candidate");
    }
    codes
}

fn push_code_once(codes: &mut Vec<String>, code: &str) {
    if !codes.iter().any(|existing| existing == code) {
        codes.push(code.to_owned());
    }
}

fn segment_digest(segment: &GeneSegment) -> String {
    stable_digest([
        segment.id.as_str(),
        segment.source.as_str(),
        segment.source_hash.as_str(),
        &segment.start_token.to_string(),
        &segment.end_token.to_string(),
        segment.kv_residency.as_str(),
        &format!("{:.3}", segment.fitness),
        &format!("{:.3}", segment.drift_score),
        &format!("{:.3}", segment.privacy_risk),
        bool_field(segment.schema_valid),
        bool_field(segment.kv_shape_valid),
    ])
}

fn plan_digest(plan: &MutationPlan) -> String {
    stable_digest([
        plan.id.as_str(),
        plan.intent.as_str(),
        plan.target_gene_id.as_str(),
        plan.replacement_gene_id
            .as_deref()
            .unwrap_or("replacement:none"),
        plan.rollback_anchor_id.as_str(),
        plan.validation_status.as_str(),
        bool_field(plan.admission_write_authorized),
        bool_field(plan.applied),
    ])
}

fn gene_record_digest(record: &DnaGeneRecord) -> String {
    stable_digest([
        record.chain_kind.as_str(),
        record.gene_id.as_str(),
        record.gene_kind.as_str(),
        record.status.as_str(),
        record.lineage.tenant_scope.as_str(),
        record.lineage.session_id.as_str(),
        record
            .lineage
            .parent_gene_id
            .as_deref()
            .unwrap_or("parent:none"),
        record
            .lineage
            .inherited_from
            .as_deref()
            .unwrap_or("inherit:none"),
        &record.lineage.generation.to_string(),
        record.source_evidence.kind.as_str(),
        record.source_evidence.source_hash.as_str(),
        record.rollback_anchor_id.as_str(),
    ])
}

fn gene_record_gate_status(record: &DnaGeneRecord) -> &'static str {
    match (record.admission_write_authorized, record.applied) {
        (false, false) => "preview_only",
        (true, false) => "approved_pending_apply",
        (true, true) => "approved_applied",
        (false, true) => "applied_without_write_gate",
    }
}

fn redacted_ref(value: &str) -> (String, bool) {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.len() > 96
        || contains_blocked_payload_marker(trimmed)
        || trimmed.contains('\n')
        || trimmed.contains('\r')
        || trimmed.contains('\t')
    {
        return (format!("redacted-ref:{}", stable_digest([trimmed])), true);
    }
    (trimmed.to_owned(), false)
}

pub fn contains_blocked_payload_marker(value: &str) -> bool {
    let normalized = value.to_ascii_lowercase();
    [
        "prompt:",
        "answer:",
        "secret=",
        "api_key",
        "private key",
        "-----begin",
        "rm ",
        "rm -",
        "curl ",
        "wget ",
        "powershell",
        "cmd.exe",
        "sudo ",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
}

fn stable_digest<'a>(parts: impl IntoIterator<Item = &'a str>) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for part in parts {
        for byte in part.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash ^= 0xff;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("audit-digest:{hash:016x}")
}

fn segment_node_id_for_raw(segment_id: &str) -> String {
    let (segment_ref, _) = redacted_ref(segment_id);
    segment_node_id_for_ref(&segment_ref)
}

fn segment_node_id_for_ref(segment_ref: &str) -> String {
    format!("segment:{segment_ref}")
}

fn mutation_plan_node_id_for_ref(plan_ref: &str) -> String {
    format!("mutation_plan:{plan_ref}")
}

fn replacement_node_id_for_ref(replacement_ref: &str) -> String {
    format!("replacement:{replacement_ref}")
}

fn lifecycle_node_id_for_ref(lifecycle_ref: &str) -> String {
    format!("lifecycle:{lifecycle_ref}")
}

fn gene_node_id_for_raw(gene_id: &str) -> String {
    let (gene_ref, _) = redacted_ref(gene_id);
    gene_node_id_for_ref(&gene_ref)
}

fn gene_node_id_for_ref(gene_ref: &str) -> String {
    format!("gene:{gene_ref}")
}

fn rollback_anchor_node_id(anchor_ref: &str) -> String {
    format!("rollback_anchor:{anchor_ref}")
}

fn push_json_field(out: &mut String, key: &str, value: &str) {
    push_json_separator(out);
    out.push('"');
    out.push_str(key);
    out.push_str("\":");
    out.push('"');
    out.push_str(&json_escape(value));
    out.push('"');
}

fn push_json_bool_field(out: &mut String, key: &str, value: bool) {
    push_json_separator(out);
    out.push('"');
    out.push_str(key);
    out.push_str("\":");
    out.push_str(bool_field(value));
}

fn push_json_separator(out: &mut String) {
    if !out.ends_with('{') {
        out.push(',');
    }
}

fn json_string_array(values: &[String]) -> String {
    let mut out = String::from("[");
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('"');
        out.push_str(&json_escape(value));
        out.push('"');
    }
    out.push(']');
    out
}

fn json_escape(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out
}

fn markdown_cell(value: &str) -> String {
    value
        .replace('|', "\\|")
        .replace('\n', " ")
        .replace('\r', " ")
        .replace('\t', " ")
}

fn bool_field(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

fn profile_to_str(profile: TaskProfile) -> &'static str {
    match profile {
        TaskProfile::General => "general",
        TaskProfile::Coding => "coding",
        TaskProfile::Writing => "writing",
        TaskProfile::LongDocument => "long_document",
    }
}
