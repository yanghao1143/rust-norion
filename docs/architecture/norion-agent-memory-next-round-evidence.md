# norion-agent / norion-memory Next-Round Evidence Bridge

This contract documents the shared next-round evidence summary consumed by
`crates/norion-agent` assignment acceptance and `crates/norion-memory`
self-improve admission. It is intentionally a documentation-backed bridge, not a
shared Rust dependency: each crate owns its local data type and projects from
the same normalized evidence vocabulary at its existing boundary.

## Contract

The bridge input is a sanitized next-round evidence summary. Producers may map
from service, CLI, daemon, or fixture status, but consumers must only receive
the normalized fields below:

- `evidence_ids`: stable evidence identifiers, already stripped of raw prompt,
  thread, window, payload, and transcript content.
- `decision_status`: one of `safe_to_wait_current_round_active`,
  `safe_to_continue_after_current_round`, or `operator_attention_blocked`.
- `current_round_active`: true only while the active daemon round is still in
  progress.
- `live_status_synced`: true only when the live status and latest committed
  ledger/report evidence describe the same completed round.
- `transition_kind` and optional round numbers: compact routing evidence, never
  raw status payloads.
- Optional `round_id_evidence`: daemon-sourced `source_schema`, `active_round`,
  `ledger_latest_round`, and `latest_done_round` facts from the normalized
  downstream status contract.
- `read_only`, `report_only`, and `no_side_effects`: all must be true.
- Side-effect flags for dispatch, prompt replay, process start, memory write,
  and `.ndkv` write: all must be false.
- Raw-window and side-effect markers: empty for admission; sanitized marker codes
  only for rejection or quarantine evidence.

`norion-agent` projects this summary into
`SanitizedNextRoundDecisionFacts`, then
`plan_clean_room_assignment_from_next_round_decision(...)` and
`accept_clean_room_assignment_planning_evidence(...)`. The accepted bridge keeps
assignment planning read-only/report-only, preserves next-round and assignment
evidence ids plus optional `round_id_evidence`, drops raw payloads, and keeps
dispatch, prompt, process, thread, memory, and `.ndkv` side effects closed.

`norion-memory` projects the same summary into
`SelfImproveLearningEvidence` with `SelfImproveNextRoundDecision`,
optional `SelfImproveRoundIdEvidence`, `next_round_live_status_synced`,
`next_round_current_round_active`, and sanitized marker vectors.
`admit_self_improve_learning_candidate(...)` may expose a memory candidate only
for a synced `safe_to_continue_after_current_round` summary with no active round
and no markers; even then the plan remains read-only and reports
`live_store_mutation_allowed=false` and `ndkv_write_allowed=false`.

## Decision Matrix

| Normalized summary | Agent assignment acceptance | Memory self-improve admission |
| --- | --- | --- |
| `safe_to_wait_current_round_active`, active round present | Wait-only evidence. Preserve ids, return no assignment task ids, no side effects. | Reject as report-only wait evidence with `next_round_wait_current_round_active`; no envelope, no writes. |
| `safe_to_continue_after_current_round`, live status synced, no active round | Accept fresh clean-room assignment planning evidence when assignment evidence is also clean. | Candidate may be accepted only if validation, report, test, helper, clean-source, clean-gist, and tag gates also pass. |
| `operator_attention_blocked` or failed readiness/report/context hygiene | Reject planning evidence and require operator attention without creating work. | Reject with `next_round_operator_attention`; no envelope, no writes. |
| Any side-effect, raw-window, raw-payload, prompt/thread/process/memory/`.ndkv` marker | Reject or quarantine evidence; never surface assignment tasks. | Reject or quarantine using sanitized marker codes; never mutate live stores. |

## Boundary Rules

- Do not make `norion-agent` depend on `norion-memory`, or `norion-memory`
  depend on `norion-agent`, for this bridge.
- If a future orchestrator needs a shared Rust type, place it in an existing
  orchestration or contract boundary that both crates already depend on; do not
  introduce direct crate coupling.
- The summary is evidence-only. It must not start daemons, open threads, send
  prompts, dispatch assignments, call models, or write live `.ndkv` stores.
- Cross-crate traceability comes from stable ids and normalized status strings,
  not raw payload reuse.

## Current Evidence

- `norion-agent::assignment::SanitizedNextRoundDecisionFacts` carries the
  normalized status, evidence ids, active/synced round facts, readiness/report
  gates, optional `SanitizedRoundIdEvidence`, and side-effect closure flags.
- `norion-agent::assignment::CleanRoomAssignmentAcceptance` is the pure
  assignment acceptance bridge. It returns accepted assignment ids only after
  clean synced completion evidence, preserves optional daemon round-id evidence,
  and it forces all runtime/write side-effect fields closed.
- `norion-memory::governance::SelfImproveNextRoundDecision` mirrors the same
  decision vocabulary for memory admission visibility.
- `norion-memory::governance::SelfImproveLearningAdmissionPlan` admits a memory
  candidate only from synced continue evidence, exposes optional
  `SelfImproveRoundIdEvidence`, and keeps live-store and `.ndkv` writes disabled.
