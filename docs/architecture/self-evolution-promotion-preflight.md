# Self-Evolution Promotion Preflight

Issue #20 needs a clear boundary between "approved for review" and "actually
active". `SelfEvolutionPromotionPreflightGate` is that boundary.

Before this preflight, `SelfEvolutionPromotionScorecardGate` can now decide
whether a candidate is ready for human approval at all. The scorecard is
advisory and digest-only: it compares correctness, latency, wasted compute,
privacy risk, reproducibility, rollback readiness, cross-task regression, flaky
runs, validation evidence, and lane-specific regression budgets. Its positive
decision is `promote_for_approval`, not "apply". It does not mutate memory,
genome state, routing state, runtime policy, Git state, or model weights.

## Inputs

The gate requires three matching records:

- `SelfEvolutionAdmissionReport`: the candidate passed policy, Rust validation,
  validation lanes, benchmark gate, rollback budget, and adaptive preview checks.
- `SelfEvolutionExperimentRecord`: the append-only experiment ledger recorded an
  `admit_for_human_review` decision for the same candidate.
- `SelfEvolutionOperatorApprovalReport`: the operator approved the exact review
  packet references from the admission report.

If any packet IDs, evidence IDs, rollback anchors, content digests, or source
schemas differ, the preflight holds.

## Safety Contract

The report can emit `ready_for_explicit_promotion=true`, but it never activates a
candidate and never grants writes. These fields stay false:

- `activation_write_allowed`
- `active_candidate`
- `write_allowed`
- `applied`

Promotion still needs an explicit future apply path. This preflight only proves
that admission, experiment, and approval evidence are internally consistent and
ready for a human-controlled next step.
