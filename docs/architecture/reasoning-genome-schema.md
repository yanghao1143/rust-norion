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
| Gene | A bounded strategy record with ID, kind, label, purpose, tags, age, fitness, trust, drift, source evidence, lineage, and rollback anchor. |
| Gene age | Software freshness metadata. Older genes may need relabeling or renewed evidence. |
| Gene fitness | A normalized quality score derived from reward, reflection, and runtime evidence. |
| Gene trust | A normalized confidence score for reuse. It must stay in `0.0..=1.0`. |
| Gene mutation | A proposed metadata or behavior change, such as relabel, quarantine, rollback, splice, repair, or regenerate. |
| Gene scissors | The guarded mutation pipeline that detects, isolates, repairs, or regenerates bad records. |
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

`DnaGeneChain::to_kv_lines` emits tab-separated `dna_chain_v1` records with
escaped text fields. Each line repeats the genome ID, stable anchor, profile,
chain kind, lineage, source evidence, trust metadata, and preview safety flags so
append-only disk KV storage can replay or inspect individual records without
needing a separate header.
