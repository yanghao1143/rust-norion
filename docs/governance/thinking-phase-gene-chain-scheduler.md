# Thinking-Phase Gene-Chain Scheduler

Issue #74 adds a read-only scheduler for the model thinking loop. It treats a
request as a bounded gene-chain execution plan before model adapter generation:
planning, memory recall, genome expression, route selection, answer synthesis,
verification, and reflection.

This is scheduling and trace governance. It is not a durable memory or genome
writer.

## Phase Contracts

`ThinkingScheduler::schedule()` consumes existing preview evidence:

- `TaskAwareHierarchyPlan` for task mode, hierarchy lanes, and threshold
  pressure
- `DnaGeneChain` for express-chain and memory-chain gene records
- `AdaptiveRoutingPlan` for route decisions and retained/compressed segments
- `ComputeBudgetSchedule` for KV lookup, validation, reflection, and fallback
  budgets

The output is a `ThinkingScheduleReport` with one trace for each phase:

- `planning`
- `memory_recall`
- `genome_expression`
- `route_selection`
- `answer_synthesis`
- `verification`
- `reflection`

Every phase has a bounded token/step budget, status, selected gene digests,
route candidate digests, fallback reasons, skip reasons, and read-only write
flags.

## Selection Rules

Express-chain and memory-chain records are selected deterministically from the
DNA chain. Selection uses task-aware hierarchy thresholds, adaptive route
thresholds, trust, fitness, drift, decay, and phase affinity. Trace output uses
stable redaction digests instead of raw prompts, labels, purposes, tenant
identifiers, or route candidate payloads.

Route selection keeps digest-only summaries for retained adaptive route
decisions. If all route decisions are skipped, correctness anchors are not
retained, or the compute scheduler requested fallback, the scheduler records a
fallback reason and leaves answer synthesis in adapter-passthrough mode.

## Disabled Mode

`ThinkingSchedulerPolicy::enabled` is the feature flag. When disabled, all
phases report `disabled`, no genes or routes are selected, and
`adapter_passthrough` remains true. This allows the scheduler to be turned off
without changing model adapter behavior.

## Safety Boundary

The scheduler may expose proposal-ready planning evidence, but it cannot apply
memory writes, genome edits, experiment ledger updates, model weight changes, or
adapter behavior changes. Durable promotion still requires writer gates,
validation evidence, rollback anchors, privacy/license checks, and
maintainer/operator approval.
