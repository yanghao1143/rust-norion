# Gene Purpose Relabel Validator

Issue #72 adds a deterministic purpose ontology for Reasoning Genome records.
The validator treats relabeling as software metadata maintenance only: it can
propose a clearer label, purpose, and tags for an aged or low-fitness gene, but
it cannot mutate an active chain or write durable genome state.

This is not a biological rejuvenation claim. The DNA/gene language is a
software architecture metaphor for local routing, memory, reflection, and tool
metadata.

## Purpose Records

`GenePurposeRecord` stores the explicit purpose surface for a gene:

- ontology version
- stable gene and purpose digests
- task profile, task family, input shape, and output shape
- tenant scope and rollback anchor
- evidence class, freshness, fitness, trust, and drift scores
- label, purpose summary, normalized tags, and preview-only write gates

The serialized KV line is deterministic and tab-delimited. Identifiers and
provenance use `redaction-digest:*` values so review packets can be compared
without exposing prompts, answers, secrets, hidden reasoning, tenant payloads,
or copied third-party source text.

## Relabel vs Other Actions

Relabeling is the lightest action. It refreshes purpose metadata when the gene
is still useful but stale, ambiguous, or under-documented. A relabel proposal is
accepted only as `AcceptedPreview`, `validation_status=Pending`,
`approval_required=true`, `write_allowed=false`, and `applied=false`.

Regeneration is stronger. It creates a young replacement candidate from a stable
anchor or high-fitness sibling when a gene is too weak or polluted to refresh.

Deletion/cut is isolation-oriented. Malignant or unsafe genes are quarantined
first, then only become cut candidates after replay, rollback, privacy, and
operator approval gates pass.

Promotion is separate from all three. A preview can graduate to durable state
only through the preview-to-write checklist: writer gate, validation evidence,
rollback anchor, privacy/license checks, and maintainer/operator approval.

## Quarantine Reasons

`GenePurposeRelabelValidator` quarantines rather than applies when it sees:

- missing stable id, tenant scope, source digest, or rollback anchor
- missing privacy gate
- low trust or low fitness evidence
- ambiguous label or purpose
- conflicting, contradictory, stale, or malignant evidence
- private or executable payload markers in record or evidence text
- a current record that is not preview-only

Quarantine still produces a digest-only preview packet so the operator can see
why the relabel did not proceed. The proposal never permits writes.

## Evolution Goal Queue

The next automation target is a queue of explicit pursuit goals for
self-evolution. Each queued goal should carry:

- `success_gate`: validation commands, benchmark floors, or trace/schema checks
- `stop_condition`: the exact condition that ends evolution for that goal
- `rollback_condition`: the failure class that restores the previous state
- `budget_cap`: bounded compute, token, time, or replay budget
- `approval_gate`: maintainer/operator approval before durable mutation

The queue should advance one goal at a time. A goal stops when its success gate
passes, when its budget is exhausted, or when a rollback condition triggers.
Later goals must not inherit unsafe state from a failed goal.
