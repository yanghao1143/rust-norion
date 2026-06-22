# Self-Goal Proposal Preview

`self_goal_proposal_v1` is the first safe step toward letting rust-norion set
its own evolution goals. It does not execute goals, write adaptive state, create
branches, mutate memory, mutate the reasoning genome, or change the pursuit
queue. It only emits bounded candidate goals that a maintainer/operator can
review and admit later.

This answers the autonomy question in stages:

- Phase 1, available now: propose its own next goals as preview-only records.
- Phase 2, available now as preview: evaluate whether one proposed goal would
  be admitted after trace, test, benchmark or experiment-ledger evidence plus
  operator approval.
- Phase 3, available now as preview: emit a queue append preview packet for one
  preview-admissible goal without mutating the durable queue.
- Phase 4, later: execute admitted goals automatically inside budget,
  rollback, branch-protection, and writer-gate limits.

## Rust Surface

The executable companion is `src/self_goal_proposal.rs`, which exposes:

- `SELF_GOAL_PROPOSAL_SCHEMA_VERSION = self_goal_proposal_v1`
- `SELF_GOAL_PROPOSAL_TRACE_SCHEMA = rust-norion-self-goal-proposal-preview-v1`
- `SelfGoalProposalPolicy`
- `SelfGoalProposalCandidate`
- `SelfGoalProposalReport`
- `SelfGoalAdmissionGate`
- `SelfGoalAdmissionReport`
- `SelfGoalQueuePreviewGate`
- `SelfGoalQueuePreviewReport`
- `default_self_goal_proposal_report(queue)`
- `default_noiron_self_goal_proposal_report()`
- `default_self_goal_admission_report(proposal_report, runs)`
- `default_noiron_self_goal_admission_report()`
- `default_self_goal_queue_preview_report(queue, proposal_report, admission_report)`
- `default_noiron_self_goal_queue_preview_report()`

## Candidate Contract

Every proposed goal carries the same controls required by the pursuit queue:

- objective
- success gate
- stop condition
- rollback condition
- budget cap
- approval gate
- provenance digest
- conflict-isolation note
- read-only, write-denied, unapplied flags

Evidence is digest-only. Raw prompts, private chat content, secrets, hidden
reasoning markers, executable payloads, tenant identifiers, and unreviewed
source material are not exported into proposal records.

## Current Default Proposals

When the default Noiron pursuit queue is still active on R97, the proposal
preview emits four deterministic candidates:

- R97 endpoint and CLI runner wiring for coding service eval artifacts.
- R97 benchmark gate feed for the coding service eval runner.
- R98 self-evolving memory consolidation admission-preview feed.
- Self-goal proposal admission gate before autonomous execution.

This keeps the model pointed at the current roadmap instead of inventing
unrelated work. The R97 candidates close the active service/eval gap; the R98
candidate prepares the next self-evolving memory lane; the governance candidate
prevents proposed goals from becoming automatic execution until the admission
gate itself is proven.

## Safety Boundary

The proposal report is preview-only:

- `read_only = true`
- `write_allowed = false`
- `applied = false`

It cannot promote a candidate into the queue. Queue admission remains blocked
until the proposed goal has deterministic validation evidence, a rollback path,
trace/schema compatibility, budget bounds, privacy/license checks, and
maintainer/operator approval.

## Admission Preview

`SelfGoalAdmissionGate` is the second safe step. It evaluates proposal
candidates through the same `EvolutionGoalQueue` success, budget, rollback, and
approval logic used for normal pursuit goals. The gate can classify a candidate
as:

- `preview_admissible`: all required evidence and approval passed, and the
  preview admission limit has not been used;
- `held_for_prior_goal`: the current pursuit queue still has an active or
  blocked goal;
- `held_for_evidence`: required evidence is missing;
- `held_for_approval`: validation passed but operator/maintainer approval is
  still missing;
- `held_for_admission_limit`: another candidate already consumed the one-goal
  preview slot;
- `rejected`: rollback, failed required evidence, budget exhaustion, unsafe
  policy, invalid proposal governance, or unredacted evidence blocked the
  candidate.

The default admission report intentionally holds every candidate while the
default R97 pursuit goal is still active. In an empty or cleared queue, a
candidate with passing evidence and approval can become `preview_admissible`,
but the report still sets:

- `read_only = true`
- `write_allowed = false`
- `applied = false`

The output includes digest-only record lines and an optional queue-insert
preview digest. It never appends to the durable goal queue by itself.

## Queue Preview Packet

`SelfGoalQueuePreviewGate` is the third safe step. It consumes the current
`EvolutionGoalQueue`, a proposal report, and an admission report. If one
candidate is `preview_admissible`, not already present in the queue, and the
preview append limit has not been used, it emits an append packet with:

- existing queue digest;
- proposed goal id;
- append-record digest;
- resulting queue preview digest;
- redacted append record line;
- read-only, write-denied, unapplied flags.

The gate can classify a candidate as:

- `append_preview`: the candidate is ready as a writer-gate input;
- `held_for_admission_gate`: the admission gate has not made the candidate
  preview-admissible;
- `held_for_duplicate_goal`: the goal is already in the queue;
- `held_for_append_limit`: another candidate already consumed the one-goal
  preview slot;
- `rejected`: unsafe policy, failed admission report, non-preview queue state,
  missing candidate, or unredacted evidence blocked the packet.

This is still not a durable queue write. It only turns a fully reviewed
self-goal into the exact digest-only packet a future writer gate must approve.
