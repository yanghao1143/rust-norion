# Evolution Goal Queue

Issue #79 adds a read-only pursuit-goal queue for rust-norion
self-evolution. The queue lets the project define multiple bounded objectives
and decide which one may run next without granting durable write authority.

This is project self-evolution governance. It is not autonomous uncontrolled
mutation.

## Goal Record

`EvolutionGoal` is deterministic and preview-only. Each record carries:

- stable id
- priority
- objective
- success gate
- stop condition
- rollback condition
- budget cap
- approval gate
- provenance digest
- read-only write flags

The record line is deterministic and tab-delimited. Private prompts, secrets,
hidden reasoning markers, executable payloads, or other blocked evidence are
replaced with `redaction-digest:*` before they enter the serialized record.

## Default Noiron Queue

`default_noiron_pursuit_goal_queue()` installs the current project pursuit
queue as preview-only planning state. After the #74 thinking-phase scheduler,
#75 English/Chinese/Rust coding evaluation profile, and #76 memory
consolidation worker baselines landed, the live default queue starts at:

- #78 local research deployment profiles and resource guards
- R94 self-evolution writer gate consolidation

This gives rust-norion a bounded sequence of next objectives. The queue can
advance only when the active goal has compiler/test/benchmark/trace or ledger
evidence for its success gate and maintainer/operator approval for promotion.
If a goal reaches its target, exhausts budget, fails a rollback gate, or waits
for approval, later goals remain isolated.

## Queue Evaluation

`EvolutionGoalQueue::evaluate()` returns a read-only
`EvolutionGoalQueueReport`. It does not mutate the queue, adaptive state,
memory, genome records, git state, model weights, or experiment ledgers.

The queue orders goals by priority and stable id. It evaluates one active goal
at a time:

- `queued`: waiting behind the active or blocked goal
- `active`: needs more evidence or budgeted work
- `passed`: success gate passed and approval evidence is present
- `failed`: required evidence failed without rollback policy
- `rolled_back`: rollback signal or rollback-required evidence failure fired
- `budget_exhausted`: attempts, steps, tokens, or runtime exceeded the cap
- `blocked_for_approval`: validation passed but maintainer/operator approval is
  still missing

When a goal is active, failed, rolled back, budget exhausted, or blocked for
approval, later goals remain queued with a conflict-isolation reason. This
prevents failed state or half-reviewed evidence from leaking into the next
objective.

## Gates

Success gates can require evidence such as:

- Cargo check
- focused Rust tests
- benchmark gate
- trace-schema gate
- experiment ledger
- operator approval

Budget caps bound attempts, steps, tokens, and runtime. Stop conditions decide
whether success, budget exhaustion, rollback, or approval hold stops the current
goal/queue. Rollback conditions decide whether failed required evidence,
trace-schema failure, or an explicit rollback signal turns the goal into a
rolled-back state.

## Safety Boundary

The goal queue only chooses the next preview/evaluation lane. Durable
self-evolution remains denied until the normal writer gates pass: validation
evidence, rollback plan, privacy/license checks, maintainer/operator approval,
and the preview-to-write graduation checklist.
