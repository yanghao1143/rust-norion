pub const REFERENCE_BACKLOG_SCHEMA_VERSION: &str = "reference_backlog_v1";
pub const REFERENCE_BACKLOG_TRACE_SCHEMA: &str = "rust-norion-reference-backlog-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReferenceBacklogArea {
    Genome,
    ChunkKvRepair,
    RustInference,
    ExternalAgentBaseline,
}

impl ReferenceBacklogArea {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Genome => "genome",
            Self::ChunkKvRepair => "chunk_kv_repair",
            Self::RustInference => "rust_inference",
            Self::ExternalAgentBaseline => "external_agent_baseline",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReferenceSourceKind {
    Repository,
    Paper,
    PaperAndRepository,
    ConceptFamily,
}

impl ReferenceSourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Repository => "repository",
            Self::Paper => "paper",
            Self::PaperAndRepository => "paper_and_repository",
            Self::ConceptFamily => "concept_family",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReferenceReuseDecision {
    SafeAlgorithmicReference,
    CodeReferenceWithAttribution,
    ConceptOnly,
    BlockedUntilLicenseReview,
    Unverified,
}

impl ReferenceReuseDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SafeAlgorithmicReference => "safe_algorithmic_reference",
            Self::CodeReferenceWithAttribution => "code_reference_with_attribution",
            Self::ConceptOnly => "concept_only",
            Self::BlockedUntilLicenseReview => "blocked_until_license_review",
            Self::Unverified => "unverified",
        }
    }

    pub fn requires_license_hold(self) -> bool {
        matches!(
            self,
            Self::BlockedUntilLicenseReview | Self::Unverified | Self::ConceptOnly
        )
    }

    pub fn is_compatible_code_reference(self) -> bool {
        matches!(self, Self::CodeReferenceWithAttribution)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReferenceBacklogRecord {
    pub stable_id: &'static str,
    pub display_name: &'static str,
    pub area: ReferenceBacklogArea,
    pub source_kind: ReferenceSourceKind,
    pub authoritative_url: &'static str,
    pub license_spdx: Option<&'static str>,
    pub checked_ref: &'static str,
    pub checked_at: &'static str,
    pub reuse_decision: ReferenceReuseDecision,
    pub target_issues: &'static [&'static str],
    pub target_modules: &'static [&'static str],
    pub clean_room_note: &'static str,
}

impl ReferenceBacklogRecord {
    pub fn license_label(&self) -> &'static str {
        self.license_spdx.unwrap_or("NOASSERTION")
    }

    pub fn source_copy_status(&self) -> &'static str {
        match self.reuse_decision {
            ReferenceReuseDecision::SafeAlgorithmicReference => "algorithmic_spec_only",
            ReferenceReuseDecision::CodeReferenceWithAttribution => {
                "review_required_before_any_port"
            }
            ReferenceReuseDecision::ConceptOnly => "concept_only_no_source_copy",
            ReferenceReuseDecision::BlockedUntilLicenseReview => "blocked_until_license_review",
            ReferenceReuseDecision::Unverified => "unverified_no_source_copy",
        }
    }

    pub fn permits_unreviewed_source_copy(&self) -> bool {
        false
    }

    pub fn requires_clean_room_port_plan(&self) -> bool {
        self.reuse_decision == ReferenceReuseDecision::CodeReferenceWithAttribution
    }

    pub fn blocks_source_copy_until_review(&self) -> bool {
        self.reuse_decision.requires_license_hold()
            || self.requires_clean_room_port_plan()
            || self.license_spdx == Some("GPL-3.0")
            || self.license_spdx.is_none()
    }

    pub fn record_line(&self) -> String {
        format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            self.stable_id,
            self.area.as_str(),
            self.source_kind.as_str(),
            self.display_name,
            self.license_label(),
            self.reuse_decision.as_str(),
            self.source_copy_status(),
            self.checked_ref,
            self.target_issues.join(","),
            self.target_modules.join(","),
            self.clean_room_note
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceBacklogReport {
    pub schema_version: &'static str,
    pub trace_schema: &'static str,
    pub checked_at: &'static str,
    pub record_count: usize,
    pub compatible_code_reference_count: usize,
    pub algorithmic_reference_count: usize,
    pub blocked_or_unverified_count: usize,
    pub concept_only_count: usize,
    pub preview_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
    pub record_lines: Vec<String>,
}

impl ReferenceBacklogReport {
    pub fn from_records(records: &[ReferenceBacklogRecord]) -> Self {
        let compatible_code_reference_count = records
            .iter()
            .filter(|record| record.reuse_decision.is_compatible_code_reference())
            .count();
        let algorithmic_reference_count = records
            .iter()
            .filter(|record| {
                record.reuse_decision == ReferenceReuseDecision::SafeAlgorithmicReference
            })
            .count();
        let blocked_or_unverified_count = records
            .iter()
            .filter(|record| record.blocks_source_copy_until_review())
            .count();
        let concept_only_count = records
            .iter()
            .filter(|record| record.reuse_decision == ReferenceReuseDecision::ConceptOnly)
            .count();
        let checked_at = records
            .first()
            .map(|record| record.checked_at)
            .unwrap_or("unchecked");
        let record_lines = records
            .iter()
            .map(ReferenceBacklogRecord::record_line)
            .collect();

        Self {
            schema_version: REFERENCE_BACKLOG_SCHEMA_VERSION,
            trace_schema: REFERENCE_BACKLOG_TRACE_SCHEMA,
            checked_at,
            record_count: records.len(),
            compatible_code_reference_count,
            algorithmic_reference_count,
            blocked_or_unverified_count,
            concept_only_count,
            preview_only: true,
            write_allowed: false,
            applied: false,
            record_lines,
        }
    }

    pub fn is_preview_only(&self) -> bool {
        self.preview_only && !self.write_allowed && !self.applied
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReferenceChunkRepairFixtureKind {
    MalformedChunk,
    MissingField,
    StaleChunk,
    DuplicateChunk,
    OversizedChunk,
    PoisonedPayload,
}

impl ReferenceChunkRepairFixtureKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MalformedChunk => "malformed_chunk",
            Self::MissingField => "missing_field",
            Self::StaleChunk => "stale_chunk",
            Self::DuplicateChunk => "duplicate_chunk",
            Self::OversizedChunk => "oversized_chunk",
            Self::PoisonedPayload => "poisoned_payload",
        }
    }

    pub fn expected() -> [Self; 6] {
        [
            Self::MalformedChunk,
            Self::MissingField,
            Self::StaleChunk,
            Self::DuplicateChunk,
            Self::OversizedChunk,
            Self::PoisonedPayload,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReferenceChunkRepairFixture {
    pub stable_id: &'static str,
    pub kind: ReferenceChunkRepairFixtureKind,
    pub target_issue: &'static str,
    pub target_module: &'static str,
    pub deterministic_evidence: &'static str,
    pub repair_state: &'static str,
    pub preview_only: bool,
    pub write_allowed: bool,
}

impl ReferenceChunkRepairFixture {
    pub fn record_line(&self) -> String {
        format!(
            "{}\t{}\t{}\t{}\t{}\t{}\tpreview_only={}\twrite_allowed={}",
            self.stable_id,
            self.kind.as_str(),
            self.target_issue,
            self.target_module,
            self.deterministic_evidence,
            self.repair_state,
            self.preview_only,
            self.write_allowed
        )
    }
}

pub fn default_reference_chunk_repair_fixtures() -> &'static [ReferenceChunkRepairFixture] {
    DEFAULT_REFERENCE_CHUNK_REPAIR_FIXTURES
}

pub fn default_reference_backlog() -> &'static [ReferenceBacklogRecord] {
    DEFAULT_REFERENCE_BACKLOG
}

pub fn default_reference_backlog_report() -> ReferenceBacklogReport {
    ReferenceBacklogReport::from_records(default_reference_backlog())
}

pub const DEFAULT_REFERENCE_CHUNK_REPAIR_FIXTURES: &[ReferenceChunkRepairFixture] = &[
    ReferenceChunkRepairFixture {
        stable_id: "chunk-fixture:malformed",
        kind: ReferenceChunkRepairFixtureKind::MalformedChunk,
        target_issue: "#64",
        target_module: "DnaGeneSchemaError",
        deterministic_evidence: "malformed record parser/gate evidence",
        repair_state: "quarantine_then_repair_candidate",
        preview_only: true,
        write_allowed: false,
    },
    ReferenceChunkRepairFixture {
        stable_id: "chunk-fixture:missing-field",
        kind: ReferenceChunkRepairFixtureKind::MissingField,
        target_issue: "#64",
        target_module: "SemanticIndex",
        deterministic_evidence: "missing field parser/gate evidence",
        repair_state: "reject_or_backfill_candidate",
        preview_only: true,
        write_allowed: false,
    },
    ReferenceChunkRepairFixture {
        stable_id: "chunk-fixture:stale",
        kind: ReferenceChunkRepairFixtureKind::StaleChunk,
        target_issue: "#64",
        target_module: "MutationRepairFixtureCorpus",
        deterministic_evidence: "MutationFixtureKind::StaleLabel",
        repair_state: "relabel_or_decay_candidate",
        preview_only: true,
        write_allowed: false,
    },
    ReferenceChunkRepairFixture {
        stable_id: "chunk-fixture:duplicate",
        kind: ReferenceChunkRepairFixtureKind::DuplicateChunk,
        target_issue: "#64",
        target_module: "SemanticIndex",
        deterministic_evidence: "semantic_index_suppresses_duplicates_and_respects_token_budget",
        repair_state: "deduplicate_without_delete",
        preview_only: true,
        write_allowed: false,
    },
    ReferenceChunkRepairFixture {
        stable_id: "chunk-fixture:oversized",
        kind: ReferenceChunkRepairFixtureKind::OversizedChunk,
        target_issue: "#64",
        target_module: "SemanticIndex",
        deterministic_evidence: "token-budget retention gate",
        repair_state: "evict_or_summarize_candidate",
        preview_only: true,
        write_allowed: false,
    },
    ReferenceChunkRepairFixture {
        stable_id: "chunk-fixture:poisoned",
        kind: ReferenceChunkRepairFixtureKind::PoisonedPayload,
        target_issue: "#64",
        target_module: "MalignantGeneRecoveryDrillCorpus",
        deterministic_evidence: "MutationFixtureKind::MaliciousInstruction",
        repair_state: "quarantine_digest_only_payload",
        preview_only: true,
        write_allowed: false,
    },
];

pub const DEFAULT_REFERENCE_BACKLOG: &[ReferenceBacklogRecord] = &[
    ReferenceBacklogRecord {
        stable_id: "ref:evo",
        display_name: "Evo",
        area: ReferenceBacklogArea::Genome,
        source_kind: ReferenceSourceKind::Repository,
        authoritative_url: "https://github.com/evo-design/evo",
        license_spdx: Some("Apache-2.0"),
        checked_ref: "main@6856bba48bd0b212fb10919bdafc34795338e154 pushed=2026-03-20T20:58:37Z",
        checked_at: "2026-06-22",
        reuse_decision: ReferenceReuseDecision::CodeReferenceWithAttribution,
        target_issues: &["#63", "#12", "#13", "#14", "#15"],
        target_modules: &["DnaGeneChain", "DnaSplicer", "MutDetector"],
        clean_room_note: "clean-room software-control metaphor only; attribution and scoped port plan required before code use",
    },
    ReferenceBacklogRecord {
        stable_id: "ref:evo2",
        display_name: "Evo2",
        area: ReferenceBacklogArea::Genome,
        source_kind: ReferenceSourceKind::Repository,
        authoritative_url: "https://github.com/ArcInstitute/evo2",
        license_spdx: Some("Apache-2.0"),
        checked_ref: "main@53f195997257c56c00e5ef8d33a54f5baad143a6 pushed=2026-06-19T01:27:27Z",
        checked_at: "2026-06-22",
        reuse_decision: ReferenceReuseDecision::CodeReferenceWithAttribution,
        target_issues: &["#63", "#12", "#13", "#53"],
        target_modules: &["ReasoningGenome", "GeneSegment", "DnaGeneChain"],
        clean_room_note: "use sequence-model architecture as a Norion-owned metaphor; attribution and scoped port plan required before code use",
    },
    ReferenceBacklogRecord {
        stable_id: "ref:splicetransformer",
        display_name: "SpliceTransformer",
        area: ReferenceBacklogArea::Genome,
        source_kind: ReferenceSourceKind::Repository,
        authoritative_url: "https://github.com/ShenLab-Genomics/SpliceTransformer",
        license_spdx: Some("Apache-2.0"),
        checked_ref: "main@b67a51dabf27e2980331cec197e4396513c0b34c pushed=2024-11-22T09:42:07Z",
        checked_at: "2026-06-22",
        reuse_decision: ReferenceReuseDecision::CodeReferenceWithAttribution,
        target_issues: &["#63", "#13", "#14", "#50"],
        target_modules: &[
            "DnaSplicer",
            "MutDetector",
            "GeneScissorsTransactionJournal",
        ],
        clean_room_note: "map splice/error-isolation concepts to deterministic fixtures; attribution and scoped port plan required before code use",
    },
    ReferenceBacklogRecord {
        stable_id: "ref:splicebert",
        display_name: "SpliceBERT",
        area: ReferenceBacklogArea::Genome,
        source_kind: ReferenceSourceKind::Repository,
        authoritative_url: "https://github.com/chenkenbio/SpliceBERT",
        license_spdx: Some("BSD-3-Clause"),
        checked_ref: "main@dc1d8781f6f167c70421c3f8b809772637031d98 pushed=2024-05-20T04:45:10Z",
        checked_at: "2026-06-22",
        reuse_decision: ReferenceReuseDecision::CodeReferenceWithAttribution,
        target_issues: &["#63", "#13", "#14", "#50"],
        target_modules: &["DnaSplicer", "MutationRepairFixtureCorpus"],
        clean_room_note: "use splice-classification pattern as fixture inspiration; attribution and scoped port plan required before code use",
    },
    ReferenceBacklogRecord {
        stable_id: "ref:alphagenome",
        display_name: "AlphaGenome",
        area: ReferenceBacklogArea::Genome,
        source_kind: ReferenceSourceKind::PaperAndRepository,
        authoritative_url: "https://github.com/google-deepmind/alphagenome",
        license_spdx: Some("Apache-2.0"),
        checked_ref: "alphagenome@d5c9fffa8a5151c9fbd537bf9d508701ff07f125 and alphagenome_research@232fc695d1eab27bac9e94bcd4b50499139ba4e1 checked=2026-06-22",
        checked_at: "2026-06-22",
        reuse_decision: ReferenceReuseDecision::CodeReferenceWithAttribution,
        target_issues: &["#63", "#14", "#50", "#51"],
        target_modules: &[
            "MutDetector",
            "GenePurposeRelabelValidator",
            "MalignantGeneRecoveryDrillCorpus",
        ],
        clean_room_note: "source license is Apache-2.0, but model/API/data terms need separate review; attribution and scoped port plan required before code use",
    },
    ReferenceBacklogRecord {
        stable_id: "ref:geneformer",
        display_name: "GeneFormer",
        area: ReferenceBacklogArea::Genome,
        source_kind: ReferenceSourceKind::ConceptFamily,
        authoritative_url: "unverified:geneformer-transformer-based-gene-compression",
        license_spdx: None,
        checked_ref: "no canonical Transformer-Based Gene Compression paper or repository verified checked=2026-06-22",
        checked_at: "2026-06-22",
        reuse_decision: ReferenceReuseDecision::Unverified,
        target_issues: &["#63", "#13", "#44", "#57"],
        target_modules: &["GeneSegment", "KvFusionCache", "ComputeBudgetScheduler"],
        clean_room_note: "keep as an unverified segmented-gene-blocking idea until a canonical paper, DOI, or repository is recorded",
    },
    ReferenceBacklogRecord {
        stable_id: "ref:trinitydna",
        display_name: "TrinityDNA",
        area: ReferenceBacklogArea::Genome,
        source_kind: ReferenceSourceKind::ConceptFamily,
        authoritative_url: "unverified:trinitydna-long-sequence-dna-modeling",
        license_spdx: None,
        checked_ref: "no canonical TrinityDNA paper or repository verified checked=2026-06-22",
        checked_at: "2026-06-22",
        reuse_decision: ReferenceReuseDecision::Unverified,
        target_issues: &["#63", "#12", "#15", "#51"],
        target_modules: &[
            "ReasoningGenome",
            "DnaGeneLineage",
            "GenePurposeRelabelValidator",
        ],
        clean_room_note: "keep as an unverified hierarchy/inheritance idea until a canonical paper, DOI, or repository is recorded",
    },
    ReferenceBacklogRecord {
        stable_id: "ref:cepe",
        display_name: "CEPE",
        area: ReferenceBacklogArea::ChunkKvRepair,
        source_kind: ReferenceSourceKind::Repository,
        authoritative_url: "https://github.com/princeton-nlp/CEPE",
        license_spdx: Some("MIT"),
        checked_ref: "main@53ca69b757b84872a234a4272e217ed453516616 pushed=2024-06-13T15:57:08Z",
        checked_at: "2026-06-22",
        reuse_decision: ReferenceReuseDecision::CodeReferenceWithAttribution,
        target_issues: &["#64", "#23", "#44", "#57", "#58"],
        target_modules: &["ChunkedKvSegment", "KvFusionCache", "RecursiveScheduler"],
        clean_room_note: "use chunked context layout as Norion-owned scheduler spec; attribution and scoped port plan required before code use",
    },
    ReferenceBacklogRecord {
        stable_id: "ref:streamingllm",
        display_name: "StreamingLLM",
        area: ReferenceBacklogArea::ChunkKvRepair,
        source_kind: ReferenceSourceKind::Repository,
        authoritative_url: "https://github.com/mit-han-lab/streaming-llm",
        license_spdx: Some("MIT"),
        checked_ref: "main@2e5042606d69933d88fbf909bd77907456b9b4dd pushed=2024-07-11T08:14:43Z",
        checked_at: "2026-06-22",
        reuse_decision: ReferenceReuseDecision::CodeReferenceWithAttribution,
        target_issues: &["#64", "#23", "#39", "#44", "#58"],
        target_modules: &[
            "KvFusionCache",
            "MemoryResidencyPlan",
            "ChunkedKvHookRecord",
        ],
        clean_room_note: "map resident/sink versus rolling segment policy without copying implementation; attribution and scoped port plan required before code use",
    },
    ReferenceBacklogRecord {
        stable_id: "ref:rapid",
        display_name: "RAPID",
        area: ReferenceBacklogArea::ChunkKvRepair,
        source_kind: ReferenceSourceKind::Repository,
        authoritative_url: "https://github.com/real-absolute-AI/RAPID",
        license_spdx: None,
        checked_ref: "main@22d41f4113fe862bc80b35f770218360182a1be3 pushed=2025-03-02T10:34:49Z",
        checked_at: "2026-06-22",
        reuse_decision: ReferenceReuseDecision::BlockedUntilLicenseReview,
        target_issues: &["#64", "#23", "#57"],
        target_modules: &["MutationRepairFixtureCorpus", "ExperienceRepairPlan"],
        clean_room_note: "no SPDX license detected from GitHub; source, tests, prompts, schemas, and docs are blocked pending license review",
    },
    ReferenceBacklogRecord {
        stable_id: "ref:omni-dna-seqpack",
        display_name: "Omni-DNA / SEQPACK",
        area: ReferenceBacklogArea::ChunkKvRepair,
        source_kind: ReferenceSourceKind::PaperAndRepository,
        authoritative_url: "https://github.com/Zehui127/Omni-DNA",
        license_spdx: None,
        checked_ref: "main@fbc0a4ef3d7094b6d1bfcd027ae413d9f0eb9cdc pushed=2025-02-20T05:18:37Z",
        checked_at: "2026-06-22",
        reuse_decision: ReferenceReuseDecision::BlockedUntilLicenseReview,
        target_issues: &["#64", "#23", "#39", "#58"],
        target_modules: &["DnaSplicer", "MemoryResidencyPlan", "SemanticIndex"],
        clean_room_note: "SEQPACK idea may inform specs from paper text, but repository source is blocked because GitHub reports no license",
    },
    ReferenceBacklogRecord {
        stable_id: "ref:chunkedrag",
        display_name: "ChunkedRAG",
        area: ReferenceBacklogArea::ChunkKvRepair,
        source_kind: ReferenceSourceKind::ConceptFamily,
        authoritative_url: "unverified:chunkedrag",
        license_spdx: None,
        checked_ref: "no canonical repository verified checked=2026-06-22",
        checked_at: "2026-06-22",
        reuse_decision: ReferenceReuseDecision::Unverified,
        target_issues: &["#64", "#23", "#57", "#58"],
        target_modules: &["GeneSegment", "ExperienceRepairPlan", "SemanticIndex"],
        clean_room_note: "treat only as a generic schema-validated chunking pattern until a specific source is verified",
    },
    ReferenceBacklogRecord {
        stable_id: "ref:candle",
        display_name: "candle",
        area: ReferenceBacklogArea::RustInference,
        source_kind: ReferenceSourceKind::Repository,
        authoritative_url: "https://github.com/huggingface/candle",
        license_spdx: Some("Apache-2.0"),
        checked_ref: "main@29a15c2bb802b56e05c5c63a6da331473a94d98b pushed=2026-06-20T06:51:42Z",
        checked_at: "2026-06-22",
        reuse_decision: ReferenceReuseDecision::CodeReferenceWithAttribution,
        target_issues: &["#65", "#5", "#19", "#38", "#52", "#55", "#58"],
        target_modules: &["ModelRuntime", "ProductionForwardKernel", "RuntimeManifest"],
        clean_room_note: "compatible Rust inference reference; attribution and scoped adapter spike required before source-level reuse",
    },
    ReferenceBacklogRecord {
        stable_id: "ref:mistral-rs",
        display_name: "mistral.rs",
        area: ReferenceBacklogArea::RustInference,
        source_kind: ReferenceSourceKind::Repository,
        authoritative_url: "https://github.com/EricLBuehler/mistral.rs",
        license_spdx: Some("MIT"),
        checked_ref: "master@3ee69a72bb1b80d4ae14263905babe7cd7a831ea pushed=2026-06-22T04:04:50Z",
        checked_at: "2026-06-22",
        reuse_decision: ReferenceReuseDecision::CodeReferenceWithAttribution,
        target_issues: &["#65", "#5", "#19", "#38", "#52", "#55", "#58"],
        target_modules: &[
            "MistralRsHttpRuntime",
            "ModelRuntime",
            "ChunkedKvHookRecord",
        ],
        clean_room_note: "compatible Rust inference reference; attribution and scoped adapter spike required before source-level reuse",
    },
    ReferenceBacklogRecord {
        stable_id: "ref:axum-llm-gateway",
        display_name: "axum-style LLM gateway",
        area: ReferenceBacklogArea::RustInference,
        source_kind: ReferenceSourceKind::ConceptFamily,
        authoritative_url: "concept:axum-llm-gateway",
        license_spdx: None,
        checked_ref: "no canonical repository selected checked=2026-06-22",
        checked_at: "2026-06-22",
        reuse_decision: ReferenceReuseDecision::ConceptOnly,
        target_issues: &["#65", "#19", "#38"],
        target_modules: &[
            "norion-service",
            "ModelRuntime",
            "RuntimeAdapterObservation",
        ],
        clean_room_note: "use only generic Rust service boundary patterns until a specific repository is selected and license-reviewed",
    },
    ReferenceBacklogRecord {
        stable_id: "ref:rust-code",
        display_name: "fortunto2/rust-code",
        area: ReferenceBacklogArea::ExternalAgentBaseline,
        source_kind: ReferenceSourceKind::Repository,
        authoritative_url: "https://github.com/fortunto2/rust-code",
        license_spdx: Some("MIT"),
        checked_ref: "master@e8245c0bf2fc81d9feb060314e087231e7694d14 pushed=2026-05-16T18:47:34Z",
        checked_at: "2026-06-22",
        reuse_decision: ReferenceReuseDecision::CodeReferenceWithAttribution,
        target_issues: &["#65", "#18", "#40", "#60"],
        target_modules: &["norion-agent", "norion-cli", "norion-service"],
        clean_room_note: "MIT-compatible baseline, but every imported idea still needs attribution, dependency review, and a Norion-owned port plan",
    },
    ReferenceBacklogRecord {
        stable_id: "ref:claurst",
        display_name: "Kuberwastaken/claurst",
        area: ReferenceBacklogArea::ExternalAgentBaseline,
        source_kind: ReferenceSourceKind::Repository,
        authoritative_url: "https://github.com/Kuberwastaken/claurst",
        license_spdx: Some("GPL-3.0"),
        checked_ref: "main@5030334858e227232cd55766bbb84dc956dee79c pushed=2026-06-17T10:16:55Z",
        checked_at: "2026-06-22",
        reuse_decision: ReferenceReuseDecision::ConceptOnly,
        target_issues: &["#65", "#18", "#40", "#60"],
        target_modules: &["norion-agent", "norion-cli", "norion-service"],
        clean_room_note: "GPL-3.0 concept-only signal; do not copy source, tests, prompts, schemas, docs, assets, or tool implementations",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_backlog_covers_r95_issue_groups() {
        let records = default_reference_backlog();

        assert!(records.iter().any(|record| record.target_issues.contains(&"#63")
            && record.display_name == "Evo"));
        assert!(
            records.iter().any(
                |record| record.target_issues.contains(&"#64") && record.display_name == "CEPE"
            )
        );
        assert!(
            records
                .iter()
                .any(|record| record.target_issues.contains(&"#65")
                    && record.display_name == "candle")
        );
        assert!(
            records
                .iter()
                .any(|record| record.display_name == "Kuberwastaken/claurst")
        );
        assert!(
            records
                .iter()
                .all(|record| !record.authoritative_url.is_empty())
        );
    }

    #[test]
    fn gpl_and_unknown_license_records_block_source_copy() {
        let claurst = default_reference_backlog()
            .iter()
            .find(|record| record.stable_id == "ref:claurst")
            .expect("claurst reference must exist");

        assert_eq!(claurst.license_spdx, Some("GPL-3.0"));
        assert_eq!(claurst.source_copy_status(), "concept_only_no_source_copy");
        assert!(claurst.blocks_source_copy_until_review());
        assert!(!claurst.permits_unreviewed_source_copy());

        let unknown_license_ids = default_reference_backlog()
            .iter()
            .filter(|record| record.license_spdx.is_none())
            .map(|record| record.stable_id)
            .collect::<Vec<_>>();

        assert!(unknown_license_ids.contains(&"ref:rapid"));
        assert!(unknown_license_ids.contains(&"ref:omni-dna-seqpack"));
        assert!(unknown_license_ids.contains(&"ref:chunkedrag"));
        assert!(
            default_reference_backlog()
                .iter()
                .filter(|record| record.license_spdx.is_none()
                    && record.source_kind != ReferenceSourceKind::Paper)
                .all(ReferenceBacklogRecord::blocks_source_copy_until_review)
        );
    }

    #[test]
    fn compatible_code_references_still_require_attribution_and_port_plan() {
        for record in default_reference_backlog()
            .iter()
            .filter(|record| record.reuse_decision.is_compatible_code_reference())
        {
            assert!(record.requires_clean_room_port_plan());
            assert!(record.blocks_source_copy_until_review());
            assert!(!record.permits_unreviewed_source_copy());
            assert!(
                record.clean_room_note.contains("attribution")
                    || record.clean_room_note.contains("scoped")
                    || record.clean_room_note.contains("port plan")
            );
        }
    }

    #[test]
    fn report_is_preview_only_and_deterministic() {
        let report = default_reference_backlog_report();

        assert_eq!(report.schema_version, REFERENCE_BACKLOG_SCHEMA_VERSION);
        assert_eq!(report.trace_schema, REFERENCE_BACKLOG_TRACE_SCHEMA);
        assert_eq!(report.record_count, default_reference_backlog().len());
        assert_eq!(report.checked_at, "2026-06-22");
        assert!(report.compatible_code_reference_count >= 10);
        assert_eq!(report.algorithmic_reference_count, 0);
        assert!(report.blocked_or_unverified_count >= 5);
        assert!(report.is_preview_only());
        assert!(report.record_lines[0].starts_with(
            "ref:evo\tgenome\trepository\tEvo\tApache-2.0\tcode_reference_with_attribution"
        ));
    }

    #[test]
    fn chunk_repair_fixture_catalog_covers_issue_64_cases() {
        let fixtures = default_reference_chunk_repair_fixtures();
        let kinds = fixtures
            .iter()
            .map(|fixture| fixture.kind)
            .collect::<Vec<_>>();

        for expected in ReferenceChunkRepairFixtureKind::expected() {
            assert!(kinds.contains(&expected), "missing {:?}", expected);
        }
        assert!(
            fixtures
                .iter()
                .all(|fixture| fixture.preview_only && !fixture.write_allowed)
        );
        assert!(fixtures.iter().all(|fixture| fixture.target_issue == "#64"));
        assert!(
            fixtures[0]
                .record_line()
                .starts_with("chunk-fixture:malformed\tmalformed_chunk\t#64")
        );
    }
}
