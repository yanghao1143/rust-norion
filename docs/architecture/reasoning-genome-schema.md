# Reasoning Genome Schema

This document defines the DNA-inspired terms in rust-norion as software-control
semantics. The vocabulary is a metaphor for memory, routing, and repair
governance. It is not a biological claim.

## Term Map

| Term | rust-norion meaning |
| --- | --- |
| DNA chain | A typed, auditable software record for reasoning behavior and retained experience. |
| `express_chain` | Current task-facing genes that may influence routing, reflection, tool use, budget, language, safety, and retrieval behavior. |
| `memory_chain` | Latent retained experience that can be retrieved later after privacy, validation, and approval gates. |
| Gene | A bounded strategy record with ID, kind, label, purpose, tags, age, decay, fitness, trust, drift, source evidence, lineage, and rollback anchor. |
| Gene age | Software freshness metadata. Older genes may need relabeling or renewed evidence. |
| Gene decay | A normalized software-control score derived from age, low fitness, and drift. It is not biological decay. |
| Gene fitness | A normalized quality score derived from reward, reflection, and runtime evidence. |
| Gene trust | A normalized confidence score for reuse. It must stay in `0.0..=1.0`. |
| Gene mutation | A proposed metadata or behavior change, such as relabel, quarantine, rollback, splice, repair, or regenerate. |
| Gene scissors | The guarded mutation pipeline that detects, isolates, repairs, or regenerates bad records. |
| Tombstone preview | A reversible cut candidate for a malignant or quarantined gene. It preserves the rollback anchor and requires validation plus operator approval before any durable deletion. |
| Stable anchor | A rollback target used before any durable genome or memory write can be admitted. |

## Preview Safety

The initial schema is intentionally read-only:

- `read_only = true`
- `write_allowed = false`
- each gene requires `operator_approval_required = true`
- `admission_write_authorized = false`
- `applied = false`

Durable writes must pass later writer gates, benchmark evidence, trace schema
checks, rollback-anchor checks, and explicit operator approval.

## Privacy Rule

Gene records carry source evidence through hashes, summaries, and optional prompt
digests. Raw prompt payloads are not a schema field. If a future importer marks a
record as containing raw prompt-derived material, `privacy_checked` must be true
and `prompt_digest` must be present before the record can validate.

## Disk KV Compatibility

`DnaGeneChain::to_kv_lines` emits tab-separated `dna_chain_v2` records with
escaped text fields. Each line repeats the genome ID, stable anchor, profile,
chain kind, lineage, source evidence, trust metadata, and preview safety flags so
append-only disk KV storage can replay or inspect individual records without
needing a separate header.

Version `dna_chain_v2` adds explicit `decay_score` metadata beside `age`,
`fitness_score`, `trust_score`, and `drift_score` so stale-useful genes can be
relabelled from evidence while malignant genes can be quarantined, regenerated,
or tombstoned through preview-only lifecycle records.

## Aging And Regeneration Evidence

`GenomeExpression` emits `GeneLifecycleRecord` entries for keep, relabel,
quarantine, regenerate, rollback, and cut decisions. Each record includes:

- age, last-confirmed purpose, decay, fitness, and drift metadata
- source evidence from health metadata, stable anchors, drift rollback, and
  high-fitness sibling genes when available
- validation status, rollback anchor, replacement candidate, and tombstone
  candidate IDs
- `admission_write_authorized = false` and `applied = false` during preview

`GeneSegment` splicing previews also carry age, last-confirmed-purpose, decay,
fitness, and drift metadata. Old but still useful segments become relabel
candidates so the system can refresh their purpose tags instead of deleting
validated experience.
