# No-Weight Retrain Improvement Policy

Noiron should improve behavior through controllable runtime state before it
proposes any weight update. The default path is to keep the base model frozen
and promote only evidence-backed changes in memory, gene-chain expression,
routing thresholds, tool reliability, or runtime policy.

This keeps local learning cheap, reversible, and auditable:

- memory candidates can be admitted, held, quarantined, or rolled back without
  changing model weights
- reasoning genes can be relabelled, cut, spliced, or regenerated under
  Gene Scissors gates
- router and hierarchy thresholds can be previewed against benchmark deltas and
  regression budgets
- tool and runtime policy can adapt from reliability evidence and device
  constraints
- every durable proposal must carry privacy evidence, compiler/test/benchmark
  validation, a rollback anchor, and operator approval

## Improvement Lanes

`NoWeightRetrainGate` classifies each candidate into one lane:

- `memory`: disk-backed KV, gist, tool-reliability, or experience admission
- `gene`: reasoning genome metadata, expression, splice, repair, or
  regeneration
- `routing`: FHT-DKE router thresholds, hierarchy weights, task profile
  selection, or compute budgets
- `tool`: Toolsmith policy, tool trust, or local command admission
- `runtime`: device policy, KV precision, adapter selection, or service ABI
- `adapter_training_handoff`: a proposed LoRA/adapter training job that must
  stay outside automatic execution
- `model_weight_mutation`: direct base-model weight mutation, rejected by
  default

The first five lanes are no-weight lanes. They can be promoted only as
read-only scorecards or preview candidates until their downstream writer gates
also pass. `adapter_training_handoff` is not a no-weight improvement; it is a
request for human review after the control-plane levers are saturated.

## Scorecard Evidence

Each scorecard records:

- lane and decision
- benchmark delta
- regression budget
- saturation score
- redacted rollback anchor
- privacy evidence presence
- compiler/test/benchmark validation state
- operator approval state
- redacted evidence ids and digest
- blocked reasons

Scorecards must not include raw prompts, raw answers, private memory payloads,
hidden reasoning traces, secrets, or unreviewed third-party source text. The
digest lets the same candidate be correlated across issue comments, trace rows,
and benchmark logs without exposing the payload.

`SelfEvolutionPromotionScorecardGate` is the cross-lane scorecard used before a
candidate is allowed into human approval. It evaluates memory, genome, routing,
runtime-adapter, task-skill-gene, and tool-policy candidates against:

- correctness delta
- latency regression
- wasted compute regression
- privacy risk
- reproducible run count
- cross-task regression
- flaky run count
- rollback readiness
- compiler/test/benchmark validation
- digest-only artifact and trace references

Every lane has its own `SelfEvolutionRegressionBudget`. Genome and
task-skill-gene candidates use the strictest defaults, runtime-adapter
candidates get a separate adapter budget, and memory/routing/tool-policy
candidates use the balanced budget. A benchmark win is never enough by itself:
privacy risk, missing rollback anchors, raw artifact refs, failed validation,
or cross-task regressions block promotion. The scorecard remains
`read_only=true`, `report_only=true`, `preview_only=true`, `write_allowed=false`,
and `applied=false`.

## Adapter Training Handoff

Adapter or LoRA training can be proposed only when all of these are true:

- the policy explicitly enables adapter-training handoff
- no-weight memory/gene/routing/tool/runtime candidates show saturation
- privacy evidence and rollback anchors are present
- compiler, tests, and benchmarks pass
- the operator explicitly approves the handoff

Even then, the scorecard allows only a handoff proposal. It does not execute a
training run, write adapter artifacts, mutate model weights, or replace a
runtime kernel. The actual training job needs its own issue, experiment plan,
dataset review, rollback anchor, license review, benchmark gate, and maintainer
approval.

Direct base-model weight mutation remains rejected by default and should be
treated as a separate governance decision, not as part of normal
self-evolution.

## When Retraining Is Unnecessary

Retraining is unnecessary when a behavior improvement can be explained by one
of these lower-risk changes:

- better retrieval or memory admission
- cleaner gene labels, splice segments, or mutation repair
- lower wasted compute through routing and hierarchy updates
- more reliable tool choice
- safer runtime adapter selection or KV policy
- stronger reflection, replay, or benchmark gates

The engine should exhaust those levers first because they are local, testable,
and reversible. Weight or adapter training is reserved for repeated failures
that cannot be fixed by control-plane policy and are backed by reproducible
benchmarks.
