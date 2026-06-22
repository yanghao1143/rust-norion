# Genome Rejuvenation Simulation

`GenomeRejuvenationSimulationReport` is the benchmark evidence surface for
issue #51. It treats aging and rejuvenation as software maintenance of local
Reasoning Genome records: stale labels can be relabelled, low-fitness but still
useful genes can be refreshed, and malignant genes can be quarantined,
regenerated from stable anchors, or tombstoned as reversible candidates.

This is not a biological anti-aging claim. The simulation only models local
memory and routing metadata so rust-norion can reduce wasted compute and avoid
reusing polluted adaptive state without retraining model weights.

## Default Cases

The default suite covers four deterministic cases:

- Healthy gene: keep the current expression.
- Stale label gene: relabel missing purpose metadata.
- Low-fitness routing gene: refresh evidence while preserving a rollback anchor.
- Malignant safety gene: quarantine, regenerate, and tombstone candidate in
  preview-only mode.

Together these cases cover the required decision kinds: keep, relabel, refresh,
regenerate, quarantine, and tombstone.

## Evidence Rules

- Ledger lines use `redaction-digest:*` identifiers instead of raw prompts,
  answers, private payloads, or source text.
- Every non-keep decision requires a rollback anchor and operator approval
  before any durable write can be considered.
- The benchmark records before/after fitness, drift, wasted-compute proxy,
  routing-cost proxy, memory usefulness, validation status, and replay digests.
- Reports must remain read-only: `write_allowed=false` and `applied=false`.
- Malignant repair evidence is projected only from stable anchors and validated
  high-level metadata; polluted target payloads are not copied into replacement
  candidates.

## Local Gate

Use `run_default_genome_rejuvenation_simulation()` with
`GenomeRejuvenationSimulationGate::default()` for local evidence. The default
gate requires all decision kinds, replay digests, rollback readiness, digest-only
ledger output, non-regressing memory usefulness, reduced wasted compute, and no
durable writes.
