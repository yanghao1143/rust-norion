# Memory Consolidation Worker

Issue #76 adds a preview-only consolidation and forgetting worker for
self-evolving memory. The worker helps the local model keep useful experience
while reducing duplicate, stale, or low-confidence memory pressure without
modifying model weights and without deleting durable data directly.

## Snapshot Input

`SelfEvolvingMemoryConsolidationWorker` consumes digest-only
`MemoryConsolidationRecord` snapshots. A record carries:

- record id
- tenant scope
- evidence class
- source digest
- content digest
- task profile
- confidence and quality scores
- last touched step
- token estimate
- rollback anchor
- validation evidence count
- protected flag

The supported evidence classes are retrospective episodes, procedural
heuristics, tool reliability observations, and GeneSegment memory anchors.
`SelfEvolvingMemoryStore::consolidation_snapshot()` projects existing
self-evolving memory records into this format for the episode, heuristic, and
tool-reliability lanes; GeneSegment anchors can be supplied by the genome layer
using the same digest-only record type.

## Decisions

The worker emits `MemoryConsolidationDecision` records:

- `keep`: retain the record without change.
- `merge_preview`: compatible duplicate can be compacted into a primary record.
- `decay_preview`: stale confidence should be decayed.
- `tombstone_preview`: weak or unsafe memory should be hidden behind a
  reversible tombstone proposal.
- `merge_rejected`: a tempting merge was rejected, usually because tenant scope
  differs.

Merges are proposed only when tenant scope, evidence class, source digest,
content digest, and task profile match. Cross-tenant lookalikes generate an
explicit merge rejection so isolation evidence is visible instead of silently
ignored.

## Retention Interaction

This worker sits above the existing retention and maintenance policies:

- `stale_after_steps` decides when decay can be proposed.
- `decay_factor` lowers stale confidence in preview only.
- `tombstone_below_confidence` and `tombstone_below_quality` decide when a
  record should be proposed for reversible forgetting.
- Protected records keep their rollback anchors and are not merged away or
  tombstoned by the preview worker.

The existing mutable `SelfEvolvingMemoryStore::maintain()` API remains a local
maintenance primitive. The #76 worker is stricter: it produces replayable plans
from a snapshot and does not mutate memory, disk KV, genome state, or model
weights.

## Metrics

`SelfEvolvingMemoryConsolidationReport` exports before/after preview metrics:

- record count before and after preview
- token estimate before and after preview
- retrieval precision before/after/delta in milli-units
- replay safety in milli-units
- benchmark impact in milli-units

The report also emits digest-only record lines and a trace-gate-compatible
`consolidation_preview` JSON line. Raw prompts, answers, hidden reasoning,
private tenant payloads, or unreviewed external text must not appear in these
records.

## Operator Approval

All decisions are read-only by default:

- `read_only = true`
- `write_allowed = false`
- `durable_write_allowed = false`
- `applied = false`
- `applied_to_disk = false`

A merge, decay, or tombstone proposal can become durable only after the normal
writer path proves validation evidence, rollback anchors, privacy/license
checks, benchmark/trace gates, and maintainer/operator approval. Tombstones are
reversible proposals, not deletion commands.

## Replay Rule

The same snapshot and policy must reproduce the same `snapshot_digest`,
`plan_digest`, and decision record lines. This lets issue comments, experiment
ledgers, and future Rust CLI evidence packets replay the proposed memory
maintenance plan before any write gate is considered.
