# Gene Scissors Transaction Journal

Issue #73 adds the reversible transaction journal for Gene Scissors. The
journal records mutation previews as append-only, replayable software
transactions before any durable genome state can change.

The journal is still a preview surface. It does not authorize destructive
deletion, active-chain mutation, memory admission, or promotion by itself.

## Transaction Model

`GeneScissorsTransaction` is created from a `MutationPlan`. Each transaction
stores:

- deterministic transaction id
- state: quarantine, hold, reject, cut preview, regenerate preview, rollback
  preview, or promoted
- redacted source plan and target segment references
- replacement candidate reference when regeneration is proposed
- before/after digests
- reason class and evidence digest
- operator decision and validation status
- rollback anchor and stable anchor sources
- forensic copy digest outside active expression
- child lineage id for regenerated candidates
- active-expression and memory-admission gates
- preview-only write flags

The serialized journal line is deterministic and tab-delimited. Unsafe ids,
private prompts, secrets, hidden reasoning markers, executable payloads, and
long/control-character references are replaced with `redaction-digest:*`
references before they leave the transaction layer.

## Replay Rules

`GeneScissorsTransactionJournal::replay()` does not apply edits. It produces a
read-only report that says which segment refs must stay out of active
expression, which forensic copies are preserved, and which child lineages exist
for regeneration candidates.

Replay excludes quarantine, hold, reject, cut-preview, regenerate-preview, and
rollback-preview targets from active expression. Promoted state is represented
for future audited writer flows, but promotion still requires the
preview-to-write checklist: writer gate, validation evidence, rollback anchor,
privacy/license checks, and maintainer/operator approval.

## Duplicate Suppression

Appending the same deterministic transaction twice suppresses the duplicate and
records the duplicate id. The original entry stays in place, preserving
append-only ordering without letting repeated detector output inflate the
journal.

## Redacted Trace

`to_redacted_trace_lines()` emits digest-only trace rows for issue and PR
evidence. Trace rows include state, target digest, replacement digest,
before/after/evidence digests, reason class, validation status, rollback
digest, and write flags. They must not contain raw prompts, answers, secrets,
hidden reasoning, executable payloads, tenant payloads, or copied third-party
source text.

## Safety Boundary

No transaction means "delete now." A cut transaction is only a reversible cut
candidate. A regeneration transaction only links a child candidate to its
parent and source transaction. The original segment remains available as a
forensic digest and rollback target while active routing and memory admission
stay blocked.
