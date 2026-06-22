mod audit;
mod fixtures;
mod model;
mod schema;
mod splicing;

pub use audit::{
    DNA_LINEAGE_AUDIT_SCHEMA_VERSION, DnaLineageAuditEdge, DnaLineageAuditNode,
    DnaLineageAuditNodeKind, DnaLineageAuditPacket, DnaLineageRepairState,
    contains_blocked_payload_marker,
};
pub use fixtures::{
    MutationFixtureKind, MutationRepairCandidateFixture, MutationRepairFixture,
    MutationRepairFixtureCorpus, MutationRepairFixtureGateReport, MutationRepairFixtureReport,
    MutationRepairFixtureResult, default_mutation_repair_fixture_corpus,
};
pub use model::{
    GeneLifecycleAction, GeneLifecycleRecord, GeneLifecycleSourceEvidence, GeneLifecycleSourceKind,
    GeneScissorsIntent, GeneValidationStatus, GenomeExpression, GenomeExpressionInput,
    MutationPlan, ReasoningGene, ReasoningGeneKind, ReasoningGeneStatus, ReasoningGenome,
};
pub use schema::{
    DnaChainKind, DnaGeneChain, DnaGeneEvidenceKind, DnaGeneLineage, DnaGeneRecord,
    DnaGeneSchemaError, DnaGeneSourceEvidence,
};
pub use splicing::{
    ClassifiedGeneSegment, DnaSplicePreview, DnaSplicer, DnaSplicerPolicy, GeneKvResidency,
    GeneScissorsLifecycleRecord, GeneScissorsLifecycleState, GeneScissorsValidationStatus,
    GeneSegment, GeneSegmentClass, GeneSegmentDisposition, GeneSegmentSource, GeneVariantKind,
    GeneVariantSeverity, MutDetector, MutFixer, MutationFinding,
};

#[cfg(test)]
mod tests;
