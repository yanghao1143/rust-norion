# Preview-To-Write Graduation Checklist

Status: required governance checklist for issues and PRs that want to move a
preview-only rust-norion capability into a write-enabled or behavior-changing
path.

Refs: #6, #20, #27, #41, #54, #60, #62, #67, #77.

## Purpose

rust-norion is designed to evolve quickly, but the default state for memory,
Reasoning Genome, routing, Toolsmith, runtime, and self-evolution changes is
preview-only. A candidate may become write-enabled only after it has enough
evidence for a maintainer to approve the exact side effect.

This checklist is not an authorization by itself. It defines the evidence a
future writer or apply issue must attach before durable writes, active candidate
promotion, runtime policy changes, adapter-training handoff, or deployment
research changes may proceed.

## Graduation States

| State | Meaning | Allowed side effects |
| --- | --- | --- |
| `preview_only` | Candidate can be scored, traced, benchmarked, and reviewed. | No durable writes, no active behavior change. |
| `review_ready` | Evidence packets are complete enough for maintainer review. | Still no writes or activation. |
| `operator_approved` | The owner approved the exact redacted review packet refs. | Still no writes unless a writer/apply gate also passes. |
| `write_authorized` | A narrow writer/apply gate passed with rollback and approval evidence. | Only the scoped write described by the gate. |
| `applied` | The scoped write completed and emitted trace/rollback evidence. | No further side effects without a new gate. |
| `rolled_back` | The active change was reverted to its stable anchor. | Rollback trace and post-rollback validation only. |

Any missing, stale, inconsistent, or unredacted evidence keeps the candidate in
`preview_only` or `review_ready`.

## Required Evidence

Every graduation request must include these items in the issue or PR:

| Gate | Required evidence | Failure mode | Rollback expectation |
| --- | --- | --- | --- |
| Validation gate | `cargo check`, focused tests, and relevant full or package test output. | Hold if commands fail, are missing, or do not cover the touched surface. | No write; keep candidate preview-only. |
| Benchmark gate | Benchmark or trace-schema evidence for the claimed improvement or safety property. | Hold if benchmark scope is unrelated, regresses beyond budget, or omits baseline. | Reuse previous stable scorecard or rollback anchor. |
| Trace gate | JSONL/schema gate rows with flat counters, structured counters, and no unsafe write/applied flags. | Hold if schema fails, counters are missing, or active/write/applied flags appear before authorization. | Treat trace as unsafe and do not apply. |
| Redaction gate | Evidence uses ids, counts, stable digests, and summaries only. | Reject if raw prompts, answers, hidden reasoning, secrets, payloads, tickets, private refs, or unreviewed source text are exposed. | Delete or replace unsafe evidence before retry. |
| Rollback gate | Stable rollback anchor, replay plan, and post-rollback validation command are present. | Hold if the candidate cannot be reverted or replayed in isolation. | Roll back to the named anchor before any retry. |
| Operator approval gate | Maintainer approval is bound to exact review packet ids, evidence ids, rollback anchors, content digests, and source schemas. | Hold if refs are missing, extra, stale, or mismatched. | Keep approval invalid until a new review packet is approved. |
| License provenance gate | External references and copied code status are documented. | Reject if GPL/AGPL/commercial code was copied without an explicit license decision. | Remove contaminated code and rebuild clean-room notes. |
| Deployment research gate | Non-commercial deployment scope, resource limits, and rollback plan are documented. | Hold if it requests commercial permission or bypasses maintainer approval. | Disable deployment profile and revert to local preview. |

## Module Graduation Owners

Until ownership is delegated in CODEOWNERS or an issue, `@yanghao1143` owns
graduation approval for every module below.

| Module | Preview-only surface | Graduation owner | Blocked until |
| --- | --- | --- | --- |
| Disk-backed KV memory | Memory admission previews, KV-Fusion candidates, residency/compaction plans. | `@yanghao1143` | Writer gate proves privacy, dedupe/decay/quarantine handling, rollback anchor, and no unsafe `.ndkv` payload leakage. |
| Reasoning Genome | Dual-chain schema, gene labels, splicing, mutation repair, Gene Scissors, regeneration. | `@yanghao1143` | Gene transaction journal, mutation fixtures, relabel validator, quarantine tests, rollback anchors, and redacted lineage export pass. |
| Self-evolution | Admission, experiment ledger, rollback replay, operator approval, promotion preflight. | `@yanghao1143` | Admission, experiment, approval, promotion preflight, trace-schema, benchmark, and rollback evidence all agree on exact refs. |
| Routing and hierarchy | FHT-DKE router scoring, adaptive attention thresholds, task-aware hierarchy updates. | `@yanghao1143` | Replayable routing traces, compute-budget telemetry, benchmark deltas, and rollback threshold anchors pass. |
| Runtime/model service | Adapter registry, streaming, device/quantization policy, local deployment profiles. | `@yanghao1143` | Capability registry, resource guards, cancellation/backpressure tests, and non-commercial deployment runbook pass. |
| Toolsmith and agent team | Rust tooling, sub-agent aggregation, conflict/budget isolation, cross-window exchange. | `@yanghao1143` | Ownership isolation, polluted-context sanitizer, budget reports, and conflict aggregation evidence pass. |
| Adapter or weight training | Adapter-training handoff, LoRA request, direct base-model mutation. | `@yanghao1143` | No-weight lanes are saturated, dataset/license review is complete, and a separate human-approved training issue exists. |

Direct base-model weight mutation remains rejected by default and is outside
normal preview-to-write graduation.

## Evidence Packet Shape

Use this shape in issue comments, PR descriptions, and Rust-native evidence
tools:

```text
candidate_id:
linked_issue:
scope:
requested_state:
validation:
  cargo_check:
  focused_tests:
  package_or_full_tests:
benchmark_or_trace:
  baseline:
  current:
  regression_budget:
review_packet_refs:
  approval_review_packet_ids_count:
  evidence_ids_count:
  rollback_anchor_ids_count:
  content_digests_count:
  source_report_schemas_count:
redaction:
  raw_prompts_exposed: false
  raw_answers_exposed: false
  secrets_exposed: false
license:
  external_sources_reviewed:
  copied_gpl_or_agpl_code: false
rollback:
  stable_anchor:
  replay_command:
  post_rollback_validation:
operator_approval:
  maintainer:
  approval_ref:
  exact_refs_match:
write_scope:
  durable_memory_write:
  genome_write:
  runtime_policy_change:
  model_weight_or_adapter_write:
  deployment_change:
```

Evidence packets must not include raw private prompts, raw answers, hidden
reasoning, credentials, API keys, approval tickets, local paths containing
private data, unreviewed source text, or raw memory/genome payloads.

## PR Checklist

Any PR that changes a preview-only module must answer these questions:

- Which issue owns the graduation request?
- Which module and state transition is requested?
- Which writer/apply gate would authorize the side effect?
- Which command proves formatting and compilation?
- Which focused tests prove the changed gate behavior?
- Which benchmark or trace-schema gate proves the safety or performance claim?
- Which rollback anchor restores the previous stable state?
- Which evidence is redacted to counts/digests only?
- Which license/provenance notes cover external references?
- Which maintainer approval is required before merge?

If the answer to any question is missing, the PR may still merge as a preview
or documentation change, but it must not enable writes or active behavior
changes.

## Forbidden Shortcuts

These shortcuts block graduation:

- changing a default from preview-only to write-enabled without a writer gate
- treating a passing unit test as proof of a system-level benchmark claim
- using operator approval that is not bound to exact review packet refs
- copying GPL/AGPL/commercial source into the repository without an explicit
  license decision
- exposing raw prompts, raw answers, secrets, approval tickets, local memory
  payloads, or hidden reasoning in evidence
- enabling commercial use, paid hosting, sublicensing, or commercial deployment
  through a technical PR
- making a deployment profile bypass branch protection, maintainer approval, or
  non-commercial research constraints

## Maintainer Merge Rule

Public contributors may open issues and PRs, but merge remains maintainer
controlled. Graduation requires:

- linked issue and checklist evidence
- passing required status checks
- conversation resolution
- CODEOWNER review
- maintainer approval from `@yanghao1143`
- no commercial-use permission request
- no unreviewed external-source contamination

The maintainer can approve a preview-only change while rejecting the requested
write graduation. In that case the code must keep write, active, applied, and
durable mutation flags closed until a later issue supplies the missing evidence.

