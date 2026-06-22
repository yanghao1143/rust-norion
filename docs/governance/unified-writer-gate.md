# Unified Writer Gate

Status: R94 baseline for read-only self-evolution writer-gate consolidation.

The unified writer gate is a shared evidence and decision layer for four
durable-write domains:

- disk-backed memory admission
- Reasoning Genome / Gene Scissors edits
- self-evolution experiment-ledger promotion
- self-goal / evolution-goal queue append packets

It does not apply writes. It normalizes candidate evidence so the project can
review memory, genome, experiment-ledger, and goal-queue write requests through
one policy surface before any future apply issue is opened.

## Default Policy

`UnifiedWriterGatePolicy::default()` is deny-by-default:

- durable writes disabled
- review packet refs required
- validation evidence required
- trace or benchmark evidence required
- rollback anchor required
- privacy and license gates required
- operator approval required
- approval refs must match
- source reports with write, active, or applied flags are rejected

Evidence-ready candidates therefore remain `preview_only` unless an explicit
policy enables durable writes. Even then the decision is
`ready_for_explicit_apply`; the gate still does not mutate memory, genome state,
adaptive state, experiment ledgers, files, model weights, or Git state.

## Candidate Sources

The baseline adapters are:

- `UnifiedWriterGateCandidate::memory_admission_preview`
- `UnifiedWriterGateCandidate::genome_transaction_journal`
- `UnifiedWriterGateCandidate::experiment_promotion_preflight`
- `UnifiedWriterGateCandidate::self_goal_queue_preview`

Each adapter reduces its source to ids, counts, stable digests, source schemas,
boolean gate evidence, and read-only/write/applied flags. Raw prompts, answers,
hidden reasoning, approval tickets, private memory payloads, copied third-party
text, and executable payloads must not enter the gate report.

## Decisions

| Decision | Meaning |
| --- | --- |
| `preview_only` | All non-policy gates passed, but durable writes are disabled. |
| `hold` | Evidence is missing, mismatched, or incomplete, but not unsafe enough to reject. |
| `reject` | Privacy/license/source-write/source-active/source-applied safety failed. |
| `ready_for_explicit_apply` | All gates passed under a write-enabled policy, but a separate apply issue is still required. |

The JSONL trace schema currently accepts only preview/hold/reject evidence with
`read_only=true`, `write_allowed=false`, `durable_write_allowed=false`, and
`applied=false`. A `ready_for_explicit_apply` trace is intentionally rejected
until a future issue adds a scoped apply workflow and maintainer approval
binding.

## Trace Evidence

`UnifiedWriterGateReport::json_line()` emits
`rust-norion-unified-writer-gate-v1` as a count/digest-only record. The trace
schema gate aggregates:

- event and record counts
- memory/genome/experiment-ledger/evolution-goal-queue record counts
- ready/held/rejected/preview-only record counts
- reason-code count
- explicit-apply-required count
- unsafe write/durable/applied counters

The schema rejects record arrays, missing digests, inconsistent counters,
private/executable markers, ready records, or any write/applied flag.

## Graduation Boundary

R94 is a consolidation baseline, not write graduation. Any later durable write
must still attach:

- linked issue
- validation commands
- benchmark or trace evidence
- rollback anchor and replay plan
- privacy and license review
- exact review/evidence/content/schema refs
- maintainer/operator approval
- a separate writer/apply gate scoped to the requested side effect

This keeps fast self-evolution work auditable while preserving the repository's
single-writer and non-commercial research governance.
