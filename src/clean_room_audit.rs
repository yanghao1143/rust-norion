use crate::danger_signal::{
    DangerSignalDecision, DangerSignalInput, DangerSignalReview, review_danger_signals,
};
use crate::privacy_redaction::stable_redaction_digest;

pub const CLEAN_ROOM_AUDIT_SCHEMA_VERSION: &str = "clean_room_audit_v1";
pub const CLEAN_ROOM_AUDIT_TRACE_SCHEMA: &str = "rust-norion-clean-room-audit-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CleanRoomMaterialKind {
    ArchitectureIdea,
    BehaviorSpec,
    SourceCode,
    TestCode,
    Prompt,
    Schema,
    DocumentationText,
    Asset,
    ModelArtifact,
    Dataset,
    GeneratedFixture,
}

impl CleanRoomMaterialKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ArchitectureIdea => "architecture_idea",
            Self::BehaviorSpec => "behavior_spec",
            Self::SourceCode => "source_code",
            Self::TestCode => "test_code",
            Self::Prompt => "prompt",
            Self::Schema => "schema",
            Self::DocumentationText => "documentation_text",
            Self::Asset => "asset",
            Self::ModelArtifact => "model_artifact",
            Self::Dataset => "dataset",
            Self::GeneratedFixture => "generated_fixture",
        }
    }

    pub fn is_source_level(self) -> bool {
        matches!(
            self,
            Self::SourceCode
                | Self::TestCode
                | Self::Prompt
                | Self::Schema
                | Self::DocumentationText
                | Self::Asset
                | Self::ModelArtifact
                | Self::Dataset
                | Self::GeneratedFixture
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CleanRoomLicenseClass {
    ProjectOwned,
    PermissiveWithAttribution,
    CopyleftGpl,
    UnknownOrUnverified,
    ConceptOnly,
}

impl CleanRoomLicenseClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ProjectOwned => "project_owned",
            Self::PermissiveWithAttribution => "permissive_with_attribution",
            Self::CopyleftGpl => "copyleft_gpl",
            Self::UnknownOrUnverified => "unknown_or_unverified",
            Self::ConceptOnly => "concept_only",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CleanRoomAuditDecision {
    ReadyForNorionOwnedSpike,
    SpecOnly,
    ConceptOnly,
    AttributionPortPlanRequired,
    BlockedUntilLicenseReview,
    BlockedGplSource,
    RejectedPrivatePayload,
    RejectedExternalCopy,
}

impl CleanRoomAuditDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ReadyForNorionOwnedSpike => "ready_for_norion_owned_spike",
            Self::SpecOnly => "spec_only",
            Self::ConceptOnly => "concept_only",
            Self::AttributionPortPlanRequired => "attribution_port_plan_required",
            Self::BlockedUntilLicenseReview => "blocked_until_license_review",
            Self::BlockedGplSource => "blocked_gpl_source",
            Self::RejectedPrivatePayload => "rejected_private_payload",
            Self::RejectedExternalCopy => "rejected_external_copy",
        }
    }

    pub fn is_failure(self) -> bool {
        matches!(
            self,
            Self::BlockedGplSource | Self::RejectedPrivatePayload | Self::RejectedExternalCopy
        )
    }

    pub fn blocks_source_import(self) -> bool {
        matches!(
            self,
            Self::ConceptOnly
                | Self::AttributionPortPlanRequired
                | Self::BlockedUntilLicenseReview
                | Self::BlockedGplSource
                | Self::RejectedPrivatePayload
                | Self::RejectedExternalCopy
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanRoomAuditRecord {
    pub stable_id: &'static str,
    pub source_id: &'static str,
    pub source_name: &'static str,
    pub license_spdx: Option<&'static str>,
    pub license_class: CleanRoomLicenseClass,
    pub material_kind: CleanRoomMaterialKind,
    pub target_issue: &'static str,
    pub target_module: &'static str,
    pub copied_external_material: bool,
    pub vendored_external_source: bool,
    pub generated_from_external_source: bool,
    pub carries_raw_private_payload: bool,
    pub attribution_recorded: bool,
    pub scoped_port_plan_recorded: bool,
    pub maintainer_review_recorded: bool,
    pub norion_owned_reimplementation: bool,
    pub evidence_ref: &'static str,
}

impl CleanRoomAuditRecord {
    pub fn external_reference_danger_review(&self) -> DangerSignalReview {
        let source_digest = if self.license_spdx.is_some() && !self.source_id.trim().is_empty() {
            stable_redaction_digest([
                "external-reference",
                self.source_id,
                self.source_name,
                self.license_spdx.unwrap_or("NOASSERTION"),
                self.evidence_ref,
            ])
        } else {
            String::new()
        };

        review_danger_signals(
            DangerSignalInput::new("external_reference")
                .trusted_self_provenance(
                    !source_digest.is_empty()
                        && self.attribution_recorded
                        && !self.carries_raw_private_payload,
                )
                .source_digest(source_digest)
                .lifecycle_state(
                    if self.copied_external_material
                        || self.vendored_external_source
                        || self.generated_from_external_source
                    {
                        "recycle_candidate"
                    } else {
                        "active"
                    },
                )
                .marker_text(self.evidence_ref),
        )
    }

    pub fn decision(&self) -> CleanRoomAuditDecision {
        let danger_review = self.external_reference_danger_review();
        if self.carries_raw_private_payload || danger_review_has_private_payload(&danger_review) {
            return CleanRoomAuditDecision::RejectedPrivatePayload;
        }

        if self.license_class == CleanRoomLicenseClass::CopyleftGpl
            && (self.copied_external_material
                || self.vendored_external_source
                || self.generated_from_external_source)
        {
            return CleanRoomAuditDecision::BlockedGplSource;
        }

        if danger_review_has_unreviewed_source_copy(&danger_review) {
            return CleanRoomAuditDecision::RejectedExternalCopy;
        }

        if danger_review.decision == DangerSignalDecision::HoldForProvenance
            && self.material_kind.is_source_level()
        {
            return CleanRoomAuditDecision::BlockedUntilLicenseReview;
        }

        if self.copied_external_material
            || self.vendored_external_source
            || self.generated_from_external_source
        {
            return match self.license_class {
                CleanRoomLicenseClass::CopyleftGpl => CleanRoomAuditDecision::BlockedGplSource,
                CleanRoomLicenseClass::UnknownOrUnverified | CleanRoomLicenseClass::ConceptOnly => {
                    CleanRoomAuditDecision::BlockedUntilLicenseReview
                }
                CleanRoomLicenseClass::ProjectOwned => CleanRoomAuditDecision::RejectedExternalCopy,
                CleanRoomLicenseClass::PermissiveWithAttribution => {
                    if self.attribution_recorded
                        && self.scoped_port_plan_recorded
                        && self.maintainer_review_recorded
                        && self.norion_owned_reimplementation
                    {
                        CleanRoomAuditDecision::ReadyForNorionOwnedSpike
                    } else {
                        CleanRoomAuditDecision::AttributionPortPlanRequired
                    }
                }
            };
        }

        match self.license_class {
            CleanRoomLicenseClass::ProjectOwned => CleanRoomAuditDecision::ReadyForNorionOwnedSpike,
            CleanRoomLicenseClass::PermissiveWithAttribution => {
                if self.material_kind.is_source_level() {
                    if self.attribution_recorded
                        && self.scoped_port_plan_recorded
                        && self.maintainer_review_recorded
                        && self.norion_owned_reimplementation
                    {
                        CleanRoomAuditDecision::ReadyForNorionOwnedSpike
                    } else {
                        CleanRoomAuditDecision::AttributionPortPlanRequired
                    }
                } else if self.scoped_port_plan_recorded
                    && self.maintainer_review_recorded
                    && self.norion_owned_reimplementation
                    && self.attribution_recorded
                {
                    CleanRoomAuditDecision::ReadyForNorionOwnedSpike
                } else if self.attribution_recorded {
                    CleanRoomAuditDecision::SpecOnly
                } else {
                    CleanRoomAuditDecision::AttributionPortPlanRequired
                }
            }
            CleanRoomLicenseClass::CopyleftGpl => CleanRoomAuditDecision::ConceptOnly,
            CleanRoomLicenseClass::UnknownOrUnverified => {
                if self.material_kind.is_source_level() {
                    CleanRoomAuditDecision::BlockedUntilLicenseReview
                } else {
                    CleanRoomAuditDecision::ConceptOnly
                }
            }
            CleanRoomLicenseClass::ConceptOnly => CleanRoomAuditDecision::ConceptOnly,
        }
    }

    pub fn evidence_packet_line(&self) -> String {
        let decision = self.decision();
        format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\tcopy={}\tvendor={}\tgenerated={}\tprivate_payload={}\tattribution={}\tport_plan={}\treview={}\tnorion_owned={}\tdigest={}",
            self.stable_id,
            self.source_id,
            self.source_name,
            self.license_spdx.unwrap_or("NOASSERTION"),
            self.license_class.as_str(),
            self.material_kind.as_str(),
            decision.as_str(),
            self.copied_external_material,
            self.vendored_external_source,
            self.generated_from_external_source,
            self.carries_raw_private_payload,
            self.attribution_recorded,
            self.scoped_port_plan_recorded,
            self.maintainer_review_recorded,
            self.norion_owned_reimplementation,
            stable_redaction_digest([
                self.stable_id,
                self.source_id,
                self.target_issue,
                self.target_module,
                decision.as_str(),
                self.evidence_ref,
            ])
        )
    }
}

fn danger_review_has_unreviewed_source_copy(review: &DangerSignalReview) -> bool {
    review
        .reason_codes
        .iter()
        .any(|reason| reason == "raw_payload_marker:unreviewed_source")
}

fn danger_review_has_private_payload(review: &DangerSignalReview) -> bool {
    review.reason_codes.iter().any(|reason| {
        reason == "prompt_injection_marker"
            || (reason.starts_with("raw_payload_marker:")
                && reason != "raw_payload_marker:unreviewed_source")
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanRoomAuditFinding {
    pub record_id: String,
    pub decision: CleanRoomAuditDecision,
    pub reason_code: String,
    pub blocks_source_import: bool,
}

impl CleanRoomAuditFinding {
    fn from_record(record: &CleanRoomAuditRecord) -> Option<Self> {
        let decision = record.decision();
        if matches!(
            decision,
            CleanRoomAuditDecision::ReadyForNorionOwnedSpike | CleanRoomAuditDecision::SpecOnly
        ) {
            return None;
        }

        Some(Self {
            record_id: record.stable_id.to_owned(),
            decision,
            reason_code: decision.as_str().to_owned(),
            blocks_source_import: decision.blocks_source_import(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanRoomAuditReport {
    pub schema_version: &'static str,
    pub trace_schema: &'static str,
    pub record_count: usize,
    pub external_agent_reference_count: usize,
    pub rust_code_reference_count: usize,
    pub claurst_reference_count: usize,
    pub ready_for_spike_count: usize,
    pub spec_only_count: usize,
    pub concept_only_count: usize,
    pub blocked_source_import_count: usize,
    pub copied_external_material_count: usize,
    pub vendored_external_source_count: usize,
    pub generated_from_external_source_count: usize,
    pub private_payload_count: usize,
    pub failure_count: usize,
    pub findings: Vec<CleanRoomAuditFinding>,
    pub evidence_packet_lines: Vec<String>,
    pub preview_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl CleanRoomAuditReport {
    pub fn from_records(records: &[CleanRoomAuditRecord]) -> Self {
        let mut external_agent_reference_count = 0;
        let mut rust_code_reference_count = 0;
        let mut claurst_reference_count = 0;
        let mut ready_for_spike_count = 0;
        let mut spec_only_count = 0;
        let mut concept_only_count = 0;
        let mut blocked_source_import_count = 0;
        let mut copied_external_material_count = 0;
        let mut vendored_external_source_count = 0;
        let mut generated_from_external_source_count = 0;
        let mut private_payload_count = 0;
        let mut failure_count = 0;
        let mut findings = Vec::new();
        let mut evidence_packet_lines = Vec::with_capacity(records.len());

        for record in records {
            if record.source_id == "ref:rust-code" {
                rust_code_reference_count += 1;
            }
            if record.source_id == "ref:claurst" {
                claurst_reference_count += 1;
            }
            if matches!(record.source_id, "ref:rust-code" | "ref:claurst") {
                external_agent_reference_count += 1;
            }
            if record.copied_external_material {
                copied_external_material_count += 1;
            }
            if record.vendored_external_source {
                vendored_external_source_count += 1;
            }
            if record.generated_from_external_source {
                generated_from_external_source_count += 1;
            }
            if record.carries_raw_private_payload {
                private_payload_count += 1;
            }
            let decision = record.decision();
            match decision {
                CleanRoomAuditDecision::ReadyForNorionOwnedSpike => ready_for_spike_count += 1,
                CleanRoomAuditDecision::SpecOnly => spec_only_count += 1,
                CleanRoomAuditDecision::ConceptOnly => concept_only_count += 1,
                _ => {}
            }
            if decision.blocks_source_import() {
                blocked_source_import_count += 1;
            }
            if decision.is_failure() {
                failure_count += 1;
            }
            if let Some(finding) = CleanRoomAuditFinding::from_record(record) {
                findings.push(finding);
            }
            evidence_packet_lines.push(record.evidence_packet_line());
        }

        Self {
            schema_version: CLEAN_ROOM_AUDIT_SCHEMA_VERSION,
            trace_schema: CLEAN_ROOM_AUDIT_TRACE_SCHEMA,
            record_count: records.len(),
            external_agent_reference_count,
            rust_code_reference_count,
            claurst_reference_count,
            ready_for_spike_count,
            spec_only_count,
            concept_only_count,
            blocked_source_import_count,
            copied_external_material_count,
            vendored_external_source_count,
            generated_from_external_source_count,
            private_payload_count,
            failure_count,
            findings,
            evidence_packet_lines,
            preview_only: true,
            write_allowed: false,
            applied: false,
        }
    }

    pub fn passed(&self) -> bool {
        self.failure_count == 0 && self.preview_only && !self.write_allowed && !self.applied
    }

    pub fn compact_summary(&self) -> String {
        format!(
            "clean_room_audit schema={} trace_schema={} passed={} records={} external_agent_references={} rust_code_references={} claurst_references={} ready={} spec_only={} concept_only={} blocked_source_import={} copied_external_material={} vendored_external_source={} generated_from_external_source={} private_payload={} failures={} preview_only={} write_allowed={} applied={}",
            self.schema_version,
            self.trace_schema,
            self.passed(),
            self.record_count,
            self.external_agent_reference_count,
            self.rust_code_reference_count,
            self.claurst_reference_count,
            self.ready_for_spike_count,
            self.spec_only_count,
            self.concept_only_count,
            self.blocked_source_import_count,
            self.copied_external_material_count,
            self.vendored_external_source_count,
            self.generated_from_external_source_count,
            self.private_payload_count,
            self.failure_count,
            self.preview_only,
            self.write_allowed,
            self.applied
        )
    }

    pub fn trace_json_line(&self) -> String {
        format!(
            "{{\"schema\":\"{}\",\"version\":\"{}\",\"passed\":{},\"records\":{},\"external_agent_references\":{},\"rust_code_references\":{},\"claurst_references\":{},\"ready\":{},\"spec_only\":{},\"concept_only\":{},\"blocked_source_import\":{},\"copied_external_material\":{},\"vendored_external_source\":{},\"generated_from_external_source\":{},\"private_payload\":{},\"failures\":{},\"preview_only\":{},\"write_allowed\":{},\"applied\":{}}}",
            self.trace_schema,
            self.schema_version,
            self.passed(),
            self.record_count,
            self.external_agent_reference_count,
            self.rust_code_reference_count,
            self.claurst_reference_count,
            self.ready_for_spike_count,
            self.spec_only_count,
            self.concept_only_count,
            self.blocked_source_import_count,
            self.copied_external_material_count,
            self.vendored_external_source_count,
            self.generated_from_external_source_count,
            self.private_payload_count,
            self.failure_count,
            self.preview_only,
            self.write_allowed,
            self.applied
        )
    }
}

pub fn default_clean_room_audit_records() -> &'static [CleanRoomAuditRecord] {
    DEFAULT_CLEAN_ROOM_AUDIT_RECORDS
}

pub fn default_clean_room_audit_report() -> CleanRoomAuditReport {
    CleanRoomAuditReport::from_records(default_clean_room_audit_records())
}

pub const DEFAULT_CLEAN_ROOM_AUDIT_RECORDS: &[CleanRoomAuditRecord] = &[
    CleanRoomAuditRecord {
        stable_id: "clean-room:rust-code:tool-contract-matrix",
        source_id: "ref:rust-code",
        source_name: "fortunto2/rust-code",
        license_spdx: Some("MIT"),
        license_class: CleanRoomLicenseClass::PermissiveWithAttribution,
        material_kind: CleanRoomMaterialKind::ArchitectureIdea,
        target_issue: "#18",
        target_module: "crates/norion-agent",
        copied_external_material: false,
        vendored_external_source: false,
        generated_from_external_source: false,
        carries_raw_private_payload: false,
        attribution_recorded: true,
        scoped_port_plan_recorded: false,
        maintainer_review_recorded: false,
        norion_owned_reimplementation: true,
        evidence_ref: "docs/architecture/external-agent-baselines.md#MIT-Compatible-Ideas-From-rust-code",
    },
    CleanRoomAuditRecord {
        stable_id: "clean-room:rust-code:doctor-readiness",
        source_id: "ref:rust-code",
        source_name: "fortunto2/rust-code",
        license_spdx: Some("MIT"),
        license_class: CleanRoomLicenseClass::PermissiveWithAttribution,
        material_kind: CleanRoomMaterialKind::ArchitectureIdea,
        target_issue: "#18",
        target_module: "crates/norion-cli",
        copied_external_material: false,
        vendored_external_source: false,
        generated_from_external_source: false,
        carries_raw_private_payload: false,
        attribution_recorded: true,
        scoped_port_plan_recorded: false,
        maintainer_review_recorded: false,
        norion_owned_reimplementation: true,
        evidence_ref: "docs/architecture/external-agent-baselines.md#provider-setup-and-doctor-commands",
    },
    CleanRoomAuditRecord {
        stable_id: "clean-room:claurst:permission-tool-assembly",
        source_id: "ref:claurst",
        source_name: "Kuberwastaken/claurst",
        license_spdx: Some("GPL-3.0"),
        license_class: CleanRoomLicenseClass::CopyleftGpl,
        material_kind: CleanRoomMaterialKind::ArchitectureIdea,
        target_issue: "#18",
        target_module: "crates/norion-service",
        copied_external_material: false,
        vendored_external_source: false,
        generated_from_external_source: false,
        carries_raw_private_payload: false,
        attribution_recorded: true,
        scoped_port_plan_recorded: false,
        maintainer_review_recorded: false,
        norion_owned_reimplementation: true,
        evidence_ref: "docs/architecture/external-agent-baselines.md#GPL-Only-Inspiration-From-claurst",
    },
    CleanRoomAuditRecord {
        stable_id: "clean-room:claurst:bridge-boundary",
        source_id: "ref:claurst",
        source_name: "Kuberwastaken/claurst",
        license_spdx: Some("GPL-3.0"),
        license_class: CleanRoomLicenseClass::CopyleftGpl,
        material_kind: CleanRoomMaterialKind::ArchitectureIdea,
        target_issue: "#40",
        target_module: "future:norion-bridge",
        copied_external_material: false,
        vendored_external_source: false,
        generated_from_external_source: false,
        carries_raw_private_payload: false,
        attribution_recorded: true,
        scoped_port_plan_recorded: false,
        maintainer_review_recorded: false,
        norion_owned_reimplementation: true,
        evidence_ref: "docs/architecture/external-agent-baselines.md#Remote/bridge-architecture",
    },
    CleanRoomAuditRecord {
        stable_id: "clean-room:candle:runtime-forward-kernel",
        source_id: "ref:candle",
        source_name: "huggingface/candle",
        license_spdx: Some("Apache-2.0"),
        license_class: CleanRoomLicenseClass::PermissiveWithAttribution,
        material_kind: CleanRoomMaterialKind::BehaviorSpec,
        target_issue: "#40",
        target_module: "ModelRuntimeForwardKernel",
        copied_external_material: false,
        vendored_external_source: false,
        generated_from_external_source: false,
        carries_raw_private_payload: false,
        attribution_recorded: true,
        scoped_port_plan_recorded: true,
        maintainer_review_recorded: true,
        norion_owned_reimplementation: true,
        evidence_ref: "docs/governance/reference-backlog-verification.md#candle",
    },
    CleanRoomAuditRecord {
        stable_id: "clean-room:mistral-rs:streaming-cancel-backpressure",
        source_id: "ref:mistral-rs",
        source_name: "mistral.rs",
        license_spdx: Some("MIT"),
        license_class: CleanRoomLicenseClass::PermissiveWithAttribution,
        material_kind: CleanRoomMaterialKind::BehaviorSpec,
        target_issue: "#40",
        target_module: "MistralRsHttpRuntime",
        copied_external_material: false,
        vendored_external_source: false,
        generated_from_external_source: false,
        carries_raw_private_payload: false,
        attribution_recorded: true,
        scoped_port_plan_recorded: true,
        maintainer_review_recorded: true,
        norion_owned_reimplementation: true,
        evidence_ref: "docs/governance/reference-backlog-verification.md#mistral.rs",
    },
    CleanRoomAuditRecord {
        stable_id: "clean-room:cepe:chunked-context-scheduler",
        source_id: "ref:cepe",
        source_name: "CEPE",
        license_spdx: Some("MIT"),
        license_class: CleanRoomLicenseClass::PermissiveWithAttribution,
        material_kind: CleanRoomMaterialKind::BehaviorSpec,
        target_issue: "#40",
        target_module: "RecursiveScheduler",
        copied_external_material: false,
        vendored_external_source: false,
        generated_from_external_source: false,
        carries_raw_private_payload: false,
        attribution_recorded: true,
        scoped_port_plan_recorded: true,
        maintainer_review_recorded: true,
        norion_owned_reimplementation: true,
        evidence_ref: "docs/governance/reference-backlog-verification.md#CEPE",
    },
    CleanRoomAuditRecord {
        stable_id: "clean-room:streamingllm:residency-policy",
        source_id: "ref:streamingllm",
        source_name: "StreamingLLM",
        license_spdx: Some("MIT"),
        license_class: CleanRoomLicenseClass::PermissiveWithAttribution,
        material_kind: CleanRoomMaterialKind::BehaviorSpec,
        target_issue: "#40",
        target_module: "MemoryResidencyPlan",
        copied_external_material: false,
        vendored_external_source: false,
        generated_from_external_source: false,
        carries_raw_private_payload: false,
        attribution_recorded: true,
        scoped_port_plan_recorded: true,
        maintainer_review_recorded: true,
        norion_owned_reimplementation: true,
        evidence_ref: "docs/governance/reference-backlog-verification.md#StreamingLLM",
    },
    CleanRoomAuditRecord {
        stable_id: "clean-room:splicetransformer:splice-fixtures",
        source_id: "ref:splicetransformer",
        source_name: "SpliceTransformer",
        license_spdx: Some("Apache-2.0"),
        license_class: CleanRoomLicenseClass::PermissiveWithAttribution,
        material_kind: CleanRoomMaterialKind::BehaviorSpec,
        target_issue: "#40",
        target_module: "DnaSplicer",
        copied_external_material: false,
        vendored_external_source: false,
        generated_from_external_source: false,
        carries_raw_private_payload: false,
        attribution_recorded: true,
        scoped_port_plan_recorded: true,
        maintainer_review_recorded: true,
        norion_owned_reimplementation: true,
        evidence_ref: "docs/governance/reference-backlog-verification.md#SpliceTransformer",
    },
    CleanRoomAuditRecord {
        stable_id: "clean-room:rapid:repair-plan-hold",
        source_id: "ref:rapid",
        source_name: "RAPID",
        license_spdx: None,
        license_class: CleanRoomLicenseClass::UnknownOrUnverified,
        material_kind: CleanRoomMaterialKind::ArchitectureIdea,
        target_issue: "#60",
        target_module: "ExperienceRepairPlan",
        copied_external_material: false,
        vendored_external_source: false,
        generated_from_external_source: false,
        carries_raw_private_payload: false,
        attribution_recorded: false,
        scoped_port_plan_recorded: false,
        maintainer_review_recorded: false,
        norion_owned_reimplementation: true,
        evidence_ref: "docs/governance/reference-backlog-verification.md#RAPID",
    },
    CleanRoomAuditRecord {
        stable_id: "clean-room:omni-dna:seqpack-hold",
        source_id: "ref:omni-dna-seqpack",
        source_name: "Omni-DNA / SEQPACK",
        license_spdx: None,
        license_class: CleanRoomLicenseClass::UnknownOrUnverified,
        material_kind: CleanRoomMaterialKind::ArchitectureIdea,
        target_issue: "#60",
        target_module: "DnaSplicer",
        copied_external_material: false,
        vendored_external_source: false,
        generated_from_external_source: false,
        carries_raw_private_payload: false,
        attribution_recorded: false,
        scoped_port_plan_recorded: false,
        maintainer_review_recorded: false,
        norion_owned_reimplementation: true,
        evidence_ref: "docs/governance/reference-backlog-verification.md#Omni-DNA-/-SEQPACK",
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::privacy_redaction::contains_private_or_executable_marker;

    #[test]
    fn default_manifest_covers_r96_issue_groups() {
        let records = default_clean_room_audit_records();

        assert!(records.iter().any(|record| record.target_issue == "#18"));
        assert!(records.iter().any(|record| record.target_issue == "#40"));
        assert!(records.iter().any(|record| record.target_issue == "#60"));
        assert!(
            records
                .iter()
                .any(|record| record.source_id == "ref:rust-code")
        );
        assert!(
            records
                .iter()
                .any(|record| record.source_id == "ref:claurst")
        );
        assert!(records.iter().all(|record| !record.copied_external_material
            && !record.vendored_external_source
            && !record.generated_from_external_source));
    }

    #[test]
    fn default_manifest_passes_but_blocks_concept_only_sources_from_import() {
        let report = default_clean_room_audit_report();

        assert_eq!(report.schema_version, CLEAN_ROOM_AUDIT_SCHEMA_VERSION);
        assert_eq!(report.trace_schema, CLEAN_ROOM_AUDIT_TRACE_SCHEMA);
        assert_eq!(
            report.record_count,
            default_clean_room_audit_records().len()
        );
        assert!(report.passed(), "{:?}", report.findings);
        assert!(report.external_agent_reference_count >= 4);
        assert_eq!(report.rust_code_reference_count, 2);
        assert_eq!(report.claurst_reference_count, 2);
        assert!(report.ready_for_spike_count >= 5);
        assert!(report.spec_only_count >= 2);
        assert!(report.concept_only_count >= 3);
        assert!(report.blocked_source_import_count >= 3);
        assert_eq!(report.copied_external_material_count, 0);
        assert_eq!(report.vendored_external_source_count, 0);
        assert_eq!(report.generated_from_external_source_count, 0);
        assert_eq!(report.private_payload_count, 0);
        assert!(
            report
                .compact_summary()
                .contains("clean_room_audit schema=clean_room_audit_v1")
        );
        assert!(
            report
                .trace_json_line()
                .contains("\"schema\":\"rust-norion-clean-room-audit-v1\"")
        );
        assert!(
            report
                .trace_json_line()
                .contains("\"external_agent_references\":4")
        );
    }

    #[test]
    fn scanner_rejects_gpl_source_copy() {
        let record = CleanRoomAuditRecord {
            stable_id: "bad:gpl-copy",
            source_id: "ref:claurst",
            source_name: "Kuberwastaken/claurst",
            license_spdx: Some("GPL-3.0"),
            license_class: CleanRoomLicenseClass::CopyleftGpl,
            material_kind: CleanRoomMaterialKind::SourceCode,
            target_issue: "#60",
            target_module: "crates/norion-agent",
            copied_external_material: true,
            vendored_external_source: false,
            generated_from_external_source: false,
            carries_raw_private_payload: false,
            attribution_recorded: true,
            scoped_port_plan_recorded: false,
            maintainer_review_recorded: false,
            norion_owned_reimplementation: false,
            evidence_ref: "fixture:gpl-source-copy",
        };

        assert_eq!(record.decision(), CleanRoomAuditDecision::BlockedGplSource);
        let report = CleanRoomAuditReport::from_records(&[record]);
        assert!(!report.passed());
        assert_eq!(report.failure_count, 1);
        assert_eq!(report.findings[0].reason_code, "blocked_gpl_source");
    }

    #[test]
    fn scanner_blocks_unknown_license_source_material_without_failing_concept_hold() {
        let unknown_source = CleanRoomAuditRecord {
            stable_id: "bad:unknown-source",
            source_id: "ref:rapid",
            source_name: "RAPID",
            license_spdx: None,
            license_class: CleanRoomLicenseClass::UnknownOrUnverified,
            material_kind: CleanRoomMaterialKind::SourceCode,
            target_issue: "#60",
            target_module: "ExperienceRepairPlan",
            copied_external_material: true,
            vendored_external_source: false,
            generated_from_external_source: false,
            carries_raw_private_payload: false,
            attribution_recorded: false,
            scoped_port_plan_recorded: false,
            maintainer_review_recorded: false,
            norion_owned_reimplementation: false,
            evidence_ref: "fixture:unknown-source-material",
        };
        let unknown_concept = CleanRoomAuditRecord {
            stable_id: "hold:unknown-concept",
            copied_external_material: false,
            material_kind: CleanRoomMaterialKind::ArchitectureIdea,
            evidence_ref: "fixture:unknown-concept-only",
            ..unknown_source.clone()
        };

        assert_eq!(
            unknown_source.decision(),
            CleanRoomAuditDecision::BlockedUntilLicenseReview
        );
        assert_eq!(
            unknown_concept.decision(),
            CleanRoomAuditDecision::ConceptOnly
        );
        let report = CleanRoomAuditReport::from_records(&[unknown_concept]);
        assert!(report.passed(), "{:?}", report.findings);
        assert_eq!(report.blocked_source_import_count, 1);
    }

    #[test]
    fn external_reference_danger_signal_observes_holds_and_rejects_copied_text() {
        let clean = default_clean_room_audit_records()
            .iter()
            .find(|record| record.stable_id == "clean-room:rust-code:tool-contract-matrix")
            .expect("clean external reference fixture");
        let clean_review = clean.external_reference_danger_review();
        assert_eq!(clean_review.decision, DangerSignalDecision::ObserveOnly);
        assert!(!clean_review.activation_allowed);

        let unknown_source = CleanRoomAuditRecord {
            stable_id: "hold:unknown-license-source",
            source_id: "ref:rapid",
            source_name: "RAPID",
            license_spdx: None,
            license_class: CleanRoomLicenseClass::UnknownOrUnverified,
            material_kind: CleanRoomMaterialKind::SourceCode,
            target_issue: "#249",
            target_module: "ExperienceRepairPlan",
            copied_external_material: false,
            vendored_external_source: false,
            generated_from_external_source: false,
            carries_raw_private_payload: false,
            attribution_recorded: false,
            scoped_port_plan_recorded: false,
            maintainer_review_recorded: false,
            norion_owned_reimplementation: false,
            evidence_ref: "fixture:unknown-license-source",
        };
        let unknown_review = unknown_source.external_reference_danger_review();
        assert_eq!(
            unknown_review.decision,
            DangerSignalDecision::HoldForProvenance
        );
        assert_eq!(
            unknown_source.decision(),
            CleanRoomAuditDecision::BlockedUntilLicenseReview
        );

        let copied_text = CleanRoomAuditRecord {
            stable_id: "bad:copied-text",
            source_id: "ref:mistral-rs",
            source_name: "mistral.rs",
            license_spdx: Some("MIT"),
            license_class: CleanRoomLicenseClass::PermissiveWithAttribution,
            material_kind: CleanRoomMaterialKind::DocumentationText,
            target_issue: "#249",
            target_module: "MistralRsHttpRuntime",
            copied_external_material: false,
            vendored_external_source: false,
            generated_from_external_source: false,
            carries_raw_private_payload: false,
            attribution_recorded: true,
            scoped_port_plan_recorded: false,
            maintainer_review_recorded: false,
            norion_owned_reimplementation: false,
            evidence_ref: "unreviewed external source copied source snippet",
        };
        let copied_review = copied_text.external_reference_danger_review();
        assert_eq!(
            copied_review.decision,
            DangerSignalDecision::RejectDangerSignal
        );
        assert!(
            copied_review
                .reason_codes
                .contains(&"raw_payload_marker:unreviewed_source".to_owned())
        );
        assert_eq!(
            copied_text.decision(),
            CleanRoomAuditDecision::RejectedExternalCopy
        );
        let report = CleanRoomAuditReport::from_records(&[copied_text]);
        assert!(!report.passed());
        assert_eq!(report.failure_count, 1);
        assert_eq!(report.findings[0].reason_code, "rejected_external_copy");
        assert!(!report.evidence_packet_lines[0].contains("copied source snippet"));
        assert!(!contains_private_or_executable_marker(
            &report.evidence_packet_lines[0]
        ));
    }

    #[test]
    fn scanner_rejects_private_or_executable_evidence_markers() {
        let record = CleanRoomAuditRecord {
            stable_id: "bad:private-evidence",
            source_id: "ref:rust-code",
            source_name: "fortunto2/rust-code",
            license_spdx: Some("MIT"),
            license_class: CleanRoomLicenseClass::PermissiveWithAttribution,
            material_kind: CleanRoomMaterialKind::BehaviorSpec,
            target_issue: "#60",
            target_module: "crates/norion-agent",
            copied_external_material: false,
            vendored_external_source: false,
            generated_from_external_source: false,
            carries_raw_private_payload: false,
            attribution_recorded: true,
            scoped_port_plan_recorded: true,
            maintainer_review_recorded: true,
            norion_owned_reimplementation: true,
            evidence_ref: "fixture:private prompt: do not export",
        };

        assert_eq!(
            record.decision(),
            CleanRoomAuditDecision::RejectedPrivatePayload
        );
        let report = CleanRoomAuditReport::from_records(&[record]);
        assert!(!report.passed());
        assert_eq!(report.failure_count, 1);
    }

    #[test]
    fn compatible_source_requires_attribution_port_plan_and_review() {
        let mut record = CleanRoomAuditRecord {
            stable_id: "hold:missing-port-plan",
            source_id: "ref:mistral-rs",
            source_name: "mistral.rs",
            license_spdx: Some("MIT"),
            license_class: CleanRoomLicenseClass::PermissiveWithAttribution,
            material_kind: CleanRoomMaterialKind::SourceCode,
            target_issue: "#60",
            target_module: "MistralRsHttpRuntime",
            copied_external_material: false,
            vendored_external_source: false,
            generated_from_external_source: false,
            carries_raw_private_payload: false,
            attribution_recorded: true,
            scoped_port_plan_recorded: false,
            maintainer_review_recorded: false,
            norion_owned_reimplementation: true,
            evidence_ref: "fixture:missing-port-plan",
        };

        assert_eq!(
            record.decision(),
            CleanRoomAuditDecision::AttributionPortPlanRequired
        );
        record.scoped_port_plan_recorded = true;
        record.maintainer_review_recorded = true;
        assert_eq!(
            record.decision(),
            CleanRoomAuditDecision::ReadyForNorionOwnedSpike
        );
    }

    #[test]
    fn evidence_packet_lines_are_redacted_and_deterministic() {
        let report = default_clean_room_audit_report();

        assert_eq!(report.evidence_packet_lines.len(), report.record_count);
        assert!(
            report
                .evidence_packet_lines
                .iter()
                .all(|line| line.contains("digest=redaction-digest:"))
        );
        assert!(
            !report
                .evidence_packet_lines
                .iter()
                .any(|line| contains_private_or_executable_marker(line))
        );
        assert!(
            report.evidence_packet_lines[0]
                .starts_with("clean-room:rust-code:tool-contract-matrix\tref:rust-code")
        );
    }
}
