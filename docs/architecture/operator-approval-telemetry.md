# Operator Approval Telemetry

Issue #35 tracks the service-facing counters for self-evolution operator approval.
The counters are derived from `TraceSchemaGateReport`, so dashboards and local
model-service responses read the same gate summary that CI validates.

## Service Counter Contract

`SelfEvolutionOperatorApprovalServiceCounters` exposes:

- redacted event counts for approval, hold, review packets, evidence IDs,
  rollback anchors, content digests, source report schemas, missing refs,
  write-allowed flags, and applied flags
- status booleans: `data_present`, `approval_ready`, `review_required`, and
  `blocked`
- fixed deny-by-default capabilities: `activation_allowed`,
  `memory_write_allowed`, `genome_write_allowed`, and `kv_write_allowed`
- redacted `validation_failures` names for dashboard triage

The object intentionally does not expose operator IDs, ticket IDs, reasons,
raw prompts, raw answers, review packet payloads, hidden reasoning, or secrets.

## Fail-Closed Rules

The counter object becomes blocked when approval telemetry is present and any of
these conditions is observed:

- the trace schema gate failed
- approved plus held decisions do not equal total operator approval events
- an approved event has no review packet, evidence, rollback anchor, content
  digest, or source report schema count
- missing review packet refs were detected
- any approval report claims write permission or applied state
- any service capability flag is ever opened

An approved operator decision can make `approval_ready=true`, but it still
cannot activate a self-evolution candidate or write memory, KV, or genome state.
Those actions remain separate explicit gates with their own preview, rollback,
and maintainer approval requirements.
