# Self-Evolution Validation Artifacts

Issue #20 requires self-evolution experiments to attach compiler, test,
benchmark, and trace-gate evidence before a candidate can move toward human
review. `SelfEvolutionValidationArtifact` is the compact contract for that
attachment.

## Artifact Types

- `cargo-check`: contributes to the compiler validation lane and rust-check
  ledger counts.
- `focused-tests`: contributes to the tests validation lane.
- `benchmark-gate`: contributes to the benchmarks validation lane.
- `trace-schema-gate`: contributes to the experiments validation lane.

Each artifact records only kind, label, item count, passed count, failed count,
derived evidence ID, stable digest, and source schema. It must not include raw
command output, prompts, answers, secrets, or hidden reasoning.

## Gate Behavior

Artifacts are added through `SelfEvolutionAdmissionEvidence` and become part of
the review packet. Passing artifacts can satisfy validation lanes; failed
artifacts add failed counts and keep the admission report held. The admission
report remains read-only and never enables mutation, memory-store, NDKV, model
weight, or git writes.

Operator approval and rollback apply remain separate gates. A validation
artifact proves that a check ran; it does not grant activation authority.
