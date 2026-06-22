use crate::hierarchy::TaskProfile;

use super::model::{ReasoningGene, ReasoningGeneKind, ReasoningGeneStatus, ReasoningGenome};

const DNA_CHAIN_RECORD_VERSION: &str = "dna_chain_v2";
const DNA_CHAIN_RECORD_FIELD_COUNT: usize = 34;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DnaChainKind {
    Express,
    Memory,
}

impl DnaChainKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Express => "express_chain",
            Self::Memory => "memory_chain",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DnaGeneEvidenceKind {
    SyntheticDefault,
    Reflection,
    MemoryAdmission,
    RuntimeKv,
    ToolReliability,
    OperatorApproved,
}

impl DnaGeneEvidenceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SyntheticDefault => "synthetic_default",
            Self::Reflection => "reflection",
            Self::MemoryAdmission => "memory_admission",
            Self::RuntimeKv => "runtime_kv",
            Self::ToolReliability => "tool_reliability",
            Self::OperatorApproved => "operator_approved",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnaGeneLineage {
    pub tenant_scope: String,
    pub session_id: String,
    pub parent_gene_id: Option<String>,
    pub inherited_from: Option<String>,
    pub generation: u32,
}

impl DnaGeneLineage {
    pub fn new(tenant_scope: impl Into<String>, session_id: impl Into<String>) -> Self {
        Self {
            tenant_scope: tenant_scope.into(),
            session_id: session_id.into(),
            parent_gene_id: None,
            inherited_from: None,
            generation: 0,
        }
    }

    pub fn with_parent(mut self, parent_gene_id: impl Into<String>) -> Self {
        self.parent_gene_id = Some(parent_gene_id.into());
        self
    }

    pub fn with_inheritance(mut self, inherited_from: impl Into<String>, generation: u32) -> Self {
        self.inherited_from = Some(inherited_from.into());
        self.generation = generation;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnaGeneSourceEvidence {
    pub kind: DnaGeneEvidenceKind,
    pub source_hash: String,
    pub source_summary: String,
    pub prompt_digest: Option<String>,
    pub privacy_checked: bool,
    pub raw_prompt_included: bool,
}

impl DnaGeneSourceEvidence {
    pub fn new(
        kind: DnaGeneEvidenceKind,
        source_hash: impl Into<String>,
        source_summary: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            source_hash: source_hash.into(),
            source_summary: source_summary.into(),
            prompt_digest: None,
            privacy_checked: false,
            raw_prompt_included: false,
        }
    }

    pub fn with_prompt_digest(mut self, prompt_digest: impl Into<String>) -> Self {
        self.prompt_digest = Some(prompt_digest.into());
        self
    }

    pub fn with_privacy_gate(mut self) -> Self {
        self.privacy_checked = true;
        self
    }

    pub fn with_raw_prompt_marker(mut self) -> Self {
        self.raw_prompt_included = true;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DnaGeneRecord {
    pub chain_kind: DnaChainKind,
    pub profile: TaskProfile,
    pub gene_id: String,
    pub gene_kind: ReasoningGeneKind,
    pub status: ReasoningGeneStatus,
    pub label: String,
    pub purpose: String,
    pub last_confirmed_purpose: String,
    pub tags: Vec<String>,
    pub lineage: DnaGeneLineage,
    pub age: u32,
    pub decay_score: f32,
    pub fitness_score: f32,
    pub trust_score: f32,
    pub drift_score: f32,
    pub source_evidence: DnaGeneSourceEvidence,
    pub rollback_anchor_id: String,
    pub operator_approval_required: bool,
    pub admission_write_authorized: bool,
    pub applied: bool,
}

impl DnaGeneRecord {
    pub fn from_reasoning_gene(
        chain_kind: DnaChainKind,
        profile: TaskProfile,
        stable_anchor_id: impl Into<String>,
        lineage: DnaGeneLineage,
        source_evidence: DnaGeneSourceEvidence,
        gene: &ReasoningGene,
    ) -> Self {
        let trust_score = (gene.fitness * (1.0 - gene.drift_score)).clamp(0.0, 1.0);
        Self {
            chain_kind,
            profile,
            gene_id: gene.id.clone(),
            gene_kind: gene.kind,
            status: gene.derived_status(),
            label: gene.label.clone(),
            purpose: gene.purpose.clone(),
            last_confirmed_purpose: gene.purpose.clone(),
            tags: gene.tags.clone(),
            lineage,
            age: gene.age,
            decay_score: gene.decay_score(),
            fitness_score: gene.fitness,
            trust_score,
            drift_score: gene.drift_score,
            source_evidence,
            rollback_anchor_id: stable_anchor_id.into(),
            operator_approval_required: true,
            admission_write_authorized: false,
            applied: false,
        }
    }

    pub fn with_chain_kind(mut self, chain_kind: DnaChainKind) -> Self {
        self.chain_kind = chain_kind;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DnaGeneChain {
    pub schema_version: &'static str,
    pub genome_id: String,
    pub stable_anchor_id: String,
    pub profile: TaskProfile,
    pub express_chain: Vec<DnaGeneRecord>,
    pub memory_chain: Vec<DnaGeneRecord>,
    pub read_only: bool,
    pub write_allowed: bool,
}

impl DnaGeneChain {
    pub fn preview_from_genome(
        genome: &ReasoningGenome,
        tenant_scope: impl Into<String>,
        session_id: impl Into<String>,
        source_evidence: DnaGeneSourceEvidence,
    ) -> Self {
        let tenant_scope = tenant_scope.into();
        let session_id = session_id.into();
        let express_chain = genome
            .genes
            .iter()
            .map(|gene| {
                DnaGeneRecord::from_reasoning_gene(
                    DnaChainKind::Express,
                    genome.profile,
                    genome.stable_anchor_id.clone(),
                    DnaGeneLineage::new(tenant_scope.clone(), session_id.clone())
                        .with_inheritance(genome.stable_anchor_id.clone(), 0),
                    source_evidence.clone(),
                    gene,
                )
            })
            .collect();

        Self {
            schema_version: DNA_CHAIN_RECORD_VERSION,
            genome_id: genome.id.clone(),
            stable_anchor_id: genome.stable_anchor_id.clone(),
            profile: genome.profile,
            express_chain,
            memory_chain: Vec::new(),
            read_only: true,
            write_allowed: false,
        }
    }

    pub fn push_memory_record(&mut self, mut record: DnaGeneRecord) {
        record.chain_kind = DnaChainKind::Memory;
        self.memory_chain.push(record);
    }

    pub fn total_gene_count(&self) -> usize {
        self.express_chain.len() + self.memory_chain.len()
    }

    pub fn validate(&self) -> Result<(), DnaGeneSchemaError> {
        if self.schema_version != DNA_CHAIN_RECORD_VERSION {
            return Err(DnaGeneSchemaError::UnsupportedSchemaVersion);
        }
        if self.genome_id.trim().is_empty() {
            return Err(DnaGeneSchemaError::EmptyGenomeId);
        }
        if self.stable_anchor_id.trim().is_empty() {
            return Err(DnaGeneSchemaError::EmptyStableAnchorId);
        }
        if self.express_chain.is_empty() {
            return Err(DnaGeneSchemaError::EmptyExpressionChain);
        }
        if !self.read_only {
            return Err(DnaGeneSchemaError::ReadOnlyPreviewRequired);
        }
        if self.write_allowed {
            return Err(DnaGeneSchemaError::WriteGateOpenInPreview);
        }

        for record in &self.express_chain {
            self.validate_record(record, DnaChainKind::Express)?;
        }
        for record in &self.memory_chain {
            self.validate_record(record, DnaChainKind::Memory)?;
        }

        Ok(())
    }

    pub fn to_kv_lines(&self) -> Result<Vec<String>, DnaGeneSchemaError> {
        self.validate()?;
        Ok(self
            .express_chain
            .iter()
            .chain(self.memory_chain.iter())
            .map(|record| serialize_record(self, record))
            .collect())
    }

    pub fn from_kv_lines(lines: &[String]) -> Result<Self, DnaGeneSchemaError> {
        if lines.is_empty() {
            return Err(DnaGeneSchemaError::MalformedRecord);
        }

        let mut parsed = Vec::with_capacity(lines.len());
        for line in lines {
            parsed.push(deserialize_record(line)?);
        }

        let first = &parsed[0];
        let schema_version = first.schema_version;
        let genome_id = first.genome_id.clone();
        let stable_anchor_id = first.stable_anchor_id.clone();
        let profile = first.profile;
        let read_only = first.read_only;
        let write_allowed = first.write_allowed;
        let mut express_chain = Vec::new();
        let mut memory_chain = Vec::new();

        for record in parsed {
            if record.schema_version != schema_version
                || record.genome_id != genome_id
                || record.stable_anchor_id != stable_anchor_id
                || record.profile != profile
                || record.read_only != read_only
                || record.write_allowed != write_allowed
            {
                return Err(DnaGeneSchemaError::MalformedRecord);
            }
            match record.record.chain_kind {
                DnaChainKind::Express => express_chain.push(record.record),
                DnaChainKind::Memory => memory_chain.push(record.record),
            }
        }

        let chain = Self {
            schema_version,
            genome_id,
            stable_anchor_id,
            profile,
            express_chain,
            memory_chain,
            read_only,
            write_allowed,
        };
        chain.validate()?;
        Ok(chain)
    }

    fn validate_record(
        &self,
        record: &DnaGeneRecord,
        expected_chain_kind: DnaChainKind,
    ) -> Result<(), DnaGeneSchemaError> {
        if record.chain_kind != expected_chain_kind {
            return Err(DnaGeneSchemaError::ChainKindMismatch {
                gene_id: record.gene_id.clone(),
                expected: expected_chain_kind,
                actual: record.chain_kind,
            });
        }
        if record.profile != self.profile {
            return Err(DnaGeneSchemaError::ProfileMismatch {
                gene_id: record.gene_id.clone(),
            });
        }
        if record.gene_id.trim().is_empty() {
            return Err(DnaGeneSchemaError::GeneIdMissing);
        }
        if record.label.trim().is_empty() {
            return Err(DnaGeneSchemaError::LabelMissing {
                gene_id: record.gene_id.clone(),
            });
        }
        if record.purpose.trim().is_empty() {
            return Err(DnaGeneSchemaError::PurposeMissing {
                gene_id: record.gene_id.clone(),
            });
        }
        if record.last_confirmed_purpose.trim().is_empty() {
            return Err(DnaGeneSchemaError::LastConfirmedPurposeMissing {
                gene_id: record.gene_id.clone(),
            });
        }
        if record.lineage.tenant_scope.trim().is_empty()
            || record.lineage.session_id.trim().is_empty()
        {
            return Err(DnaGeneSchemaError::LineageMissing {
                gene_id: record.gene_id.clone(),
            });
        }
        if !is_unit(record.decay_score)
            || !is_unit(record.fitness_score)
            || !is_unit(record.trust_score)
            || !is_unit(record.drift_score)
        {
            return Err(DnaGeneSchemaError::TrustOutOfRange {
                gene_id: record.gene_id.clone(),
            });
        }
        if record.source_evidence.source_hash.trim().is_empty()
            || record.source_evidence.source_summary.trim().is_empty()
        {
            return Err(DnaGeneSchemaError::SourceEvidenceMissing {
                gene_id: record.gene_id.clone(),
            });
        }
        if record.source_evidence.raw_prompt_included && !record.source_evidence.privacy_checked {
            return Err(DnaGeneSchemaError::PrivacyGateRequired {
                gene_id: record.gene_id.clone(),
            });
        }
        if record.source_evidence.raw_prompt_included
            && record
                .source_evidence
                .prompt_digest
                .as_deref()
                .is_none_or(|digest| digest.trim().is_empty())
        {
            return Err(DnaGeneSchemaError::PromptDigestMissing {
                gene_id: record.gene_id.clone(),
            });
        }
        if record.rollback_anchor_id.trim().is_empty() {
            return Err(DnaGeneSchemaError::RollbackAnchorMissing {
                gene_id: record.gene_id.clone(),
            });
        }
        if !record.operator_approval_required {
            return Err(DnaGeneSchemaError::ApprovalGateMissing {
                gene_id: record.gene_id.clone(),
            });
        }
        if self.read_only && record.admission_write_authorized {
            return Err(DnaGeneSchemaError::WriteGateOpenInPreview);
        }
        if self.read_only && record.applied {
            return Err(DnaGeneSchemaError::AppliedPreviewMutation);
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DnaGeneSchemaError {
    EmptyGenomeId,
    EmptyStableAnchorId,
    EmptyExpressionChain,
    UnsupportedSchemaVersion,
    MalformedRecord,
    GeneIdMissing,
    ChainKindMismatch {
        gene_id: String,
        expected: DnaChainKind,
        actual: DnaChainKind,
    },
    ProfileMismatch {
        gene_id: String,
    },
    LabelMissing {
        gene_id: String,
    },
    PurposeMissing {
        gene_id: String,
    },
    LastConfirmedPurposeMissing {
        gene_id: String,
    },
    LineageMissing {
        gene_id: String,
    },
    TrustOutOfRange {
        gene_id: String,
    },
    SourceEvidenceMissing {
        gene_id: String,
    },
    PrivacyGateRequired {
        gene_id: String,
    },
    PromptDigestMissing {
        gene_id: String,
    },
    RollbackAnchorMissing {
        gene_id: String,
    },
    ApprovalGateMissing {
        gene_id: String,
    },
    ReadOnlyPreviewRequired,
    WriteGateOpenInPreview,
    AppliedPreviewMutation,
}

struct ParsedDnaRecord {
    schema_version: &'static str,
    genome_id: String,
    stable_anchor_id: String,
    profile: TaskProfile,
    record: DnaGeneRecord,
    read_only: bool,
    write_allowed: bool,
}

fn serialize_record(chain: &DnaGeneChain, record: &DnaGeneRecord) -> String {
    let fields = [
        DNA_CHAIN_RECORD_VERSION.to_owned(),
        chain.genome_id.clone(),
        chain.stable_anchor_id.clone(),
        profile_to_str(chain.profile).to_owned(),
        record.chain_kind.as_str().to_owned(),
        record.gene_id.clone(),
        record.gene_kind.as_str().to_owned(),
        record.status.as_str().to_owned(),
        record.label.clone(),
        record.purpose.clone(),
        record.last_confirmed_purpose.clone(),
        serialize_tags(&record.tags),
        record.lineage.tenant_scope.clone(),
        record.lineage.session_id.clone(),
        record.lineage.parent_gene_id.clone().unwrap_or_default(),
        record.lineage.inherited_from.clone().unwrap_or_default(),
        record.lineage.generation.to_string(),
        record.age.to_string(),
        finite_f32_to_field(record.decay_score),
        finite_f32_to_field(record.fitness_score),
        finite_f32_to_field(record.trust_score),
        finite_f32_to_field(record.drift_score),
        record.source_evidence.kind.as_str().to_owned(),
        record.source_evidence.source_hash.clone(),
        record.source_evidence.source_summary.clone(),
        record
            .source_evidence
            .prompt_digest
            .clone()
            .unwrap_or_default(),
        bool_to_field(record.source_evidence.privacy_checked).to_owned(),
        bool_to_field(record.source_evidence.raw_prompt_included).to_owned(),
        record.rollback_anchor_id.clone(),
        bool_to_field(record.operator_approval_required).to_owned(),
        bool_to_field(record.admission_write_authorized).to_owned(),
        bool_to_field(record.applied).to_owned(),
        bool_to_field(chain.read_only).to_owned(),
        bool_to_field(chain.write_allowed).to_owned(),
    ];
    fields
        .iter()
        .map(|field| escape_field(field))
        .collect::<Vec<_>>()
        .join("\t")
}

fn deserialize_record(line: &str) -> Result<ParsedDnaRecord, DnaGeneSchemaError> {
    let fields = line.split('\t').map(unescape_field).collect::<Vec<_>>();
    if fields.len() != DNA_CHAIN_RECORD_FIELD_COUNT || fields[0] != DNA_CHAIN_RECORD_VERSION {
        return Err(DnaGeneSchemaError::MalformedRecord);
    }

    let profile = str_to_profile(&fields[3])?;
    let chain_kind = str_to_chain_kind(&fields[4])?;
    let gene_kind = str_to_gene_kind(&fields[6])?;
    let status = str_to_gene_status(&fields[7])?;
    let generation = fields[16]
        .parse::<u32>()
        .map_err(|_| DnaGeneSchemaError::MalformedRecord)?;
    let age = fields[17]
        .parse::<u32>()
        .map_err(|_| DnaGeneSchemaError::MalformedRecord)?;
    let decay_score = field_to_finite_f32(&fields[18])?;
    let fitness_score = field_to_finite_f32(&fields[19])?;
    let trust_score = field_to_finite_f32(&fields[20])?;
    let drift_score = field_to_finite_f32(&fields[21])?;
    let evidence_kind = str_to_evidence_kind(&fields[22])?;
    let privacy_checked = field_to_bool(&fields[26])?;
    let raw_prompt_included = field_to_bool(&fields[27])?;
    let operator_approval_required = field_to_bool(&fields[29])?;
    let admission_write_authorized = field_to_bool(&fields[30])?;
    let applied = field_to_bool(&fields[31])?;
    let read_only = field_to_bool(&fields[32])?;
    let write_allowed = field_to_bool(&fields[33])?;

    Ok(ParsedDnaRecord {
        schema_version: DNA_CHAIN_RECORD_VERSION,
        genome_id: fields[1].clone(),
        stable_anchor_id: fields[2].clone(),
        profile,
        record: DnaGeneRecord {
            chain_kind,
            profile,
            gene_id: fields[5].clone(),
            gene_kind,
            status,
            label: fields[8].clone(),
            purpose: fields[9].clone(),
            last_confirmed_purpose: fields[10].clone(),
            tags: deserialize_tags(&fields[11]),
            lineage: DnaGeneLineage {
                tenant_scope: fields[12].clone(),
                session_id: fields[13].clone(),
                parent_gene_id: non_empty_string(&fields[14]),
                inherited_from: non_empty_string(&fields[15]),
                generation,
            },
            age,
            decay_score,
            fitness_score,
            trust_score,
            drift_score,
            source_evidence: DnaGeneSourceEvidence {
                kind: evidence_kind,
                source_hash: fields[23].clone(),
                source_summary: fields[24].clone(),
                prompt_digest: non_empty_string(&fields[25]),
                privacy_checked,
                raw_prompt_included,
            },
            rollback_anchor_id: fields[28].clone(),
            operator_approval_required,
            admission_write_authorized,
            applied,
        },
        read_only,
        write_allowed,
    })
}

fn str_to_chain_kind(value: &str) -> Result<DnaChainKind, DnaGeneSchemaError> {
    match value {
        "express_chain" => Ok(DnaChainKind::Express),
        "memory_chain" => Ok(DnaChainKind::Memory),
        _ => Err(DnaGeneSchemaError::MalformedRecord),
    }
}

fn str_to_evidence_kind(value: &str) -> Result<DnaGeneEvidenceKind, DnaGeneSchemaError> {
    match value {
        "synthetic_default" => Ok(DnaGeneEvidenceKind::SyntheticDefault),
        "reflection" => Ok(DnaGeneEvidenceKind::Reflection),
        "memory_admission" => Ok(DnaGeneEvidenceKind::MemoryAdmission),
        "runtime_kv" => Ok(DnaGeneEvidenceKind::RuntimeKv),
        "tool_reliability" => Ok(DnaGeneEvidenceKind::ToolReliability),
        "operator_approved" => Ok(DnaGeneEvidenceKind::OperatorApproved),
        _ => Err(DnaGeneSchemaError::MalformedRecord),
    }
}

fn str_to_gene_kind(value: &str) -> Result<ReasoningGeneKind, DnaGeneSchemaError> {
    match value {
        "retrieval" => Ok(ReasoningGeneKind::Retrieval),
        "routing" => Ok(ReasoningGeneKind::Routing),
        "reflection" => Ok(ReasoningGeneKind::Reflection),
        "language" => Ok(ReasoningGeneKind::Language),
        "safety" => Ok(ReasoningGeneKind::Safety),
        "tool_use" => Ok(ReasoningGeneKind::ToolUse),
        "budget" => Ok(ReasoningGeneKind::Budget),
        _ => Err(DnaGeneSchemaError::MalformedRecord),
    }
}

fn str_to_gene_status(value: &str) -> Result<ReasoningGeneStatus, DnaGeneSchemaError> {
    match value {
        "active" => Ok(ReasoningGeneStatus::Active),
        "aging" => Ok(ReasoningGeneStatus::Aging),
        "malignant" => Ok(ReasoningGeneStatus::Malignant),
        "quarantined" => Ok(ReasoningGeneStatus::Quarantined),
        "regenerating" => Ok(ReasoningGeneStatus::Regenerating),
        _ => Err(DnaGeneSchemaError::MalformedRecord),
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

fn str_to_profile(value: &str) -> Result<TaskProfile, DnaGeneSchemaError> {
    match value {
        "general" => Ok(TaskProfile::General),
        "coding" => Ok(TaskProfile::Coding),
        "writing" => Ok(TaskProfile::Writing),
        "long_document" => Ok(TaskProfile::LongDocument),
        _ => Err(DnaGeneSchemaError::MalformedRecord),
    }
}

fn serialize_tags(tags: &[String]) -> String {
    tags.iter()
        .map(|tag| escape_field(tag))
        .collect::<Vec<_>>()
        .join("|")
}

fn deserialize_tags(value: &str) -> Vec<String> {
    if value.is_empty() {
        return Vec::new();
    }
    value.split('|').map(unescape_field).collect()
}

fn is_unit(value: f32) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

fn finite_f32_to_field(value: f32) -> String {
    if value.is_finite() {
        format!("{:.6}", value)
    } else {
        "nan".to_owned()
    }
}

fn field_to_finite_f32(value: &str) -> Result<f32, DnaGeneSchemaError> {
    value
        .parse::<f32>()
        .ok()
        .filter(|value| value.is_finite())
        .ok_or(DnaGeneSchemaError::MalformedRecord)
}

fn bool_to_field(value: bool) -> &'static str {
    if value { "1" } else { "0" }
}

fn field_to_bool(value: &str) -> Result<bool, DnaGeneSchemaError> {
    match value {
        "1" | "true" => Ok(true),
        "0" | "false" => Ok(false),
        _ => Err(DnaGeneSchemaError::MalformedRecord),
    }
}

fn non_empty_string(value: &str) -> Option<String> {
    (!value.trim().is_empty()).then(|| value.to_owned())
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
