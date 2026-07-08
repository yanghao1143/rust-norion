# Local Research Deployment Profiles

Issue #78 adds deterministic local research deployment profiles for running
rust-norion experiments without turning preview evidence into durable memory,
genome, or experiment-ledger writes.

This is a guardrail layer for local deployment research. It is not a production
hosting quality guarantee and it does not relax branch protection, maintainer
approval, privacy review, GPL-3.0 or third-party license review, or
preview-to-write graduation.

## Profile Templates

`ResearchDeploymentProfile::template()` defines four stable modes:

| Profile | Device class | Adapter hint | Main use |
| --- | --- | --- | --- |
| `cpu-only` | CPU-only | portable Rust | Small local research runs and contributor machines without accelerators. |
| `single-gpu` | Discrete GPU | CUDA | One local accelerator with bounded context, KV, streaming, and background reflection. |
| `low-memory` | Embedded/constrained | portable Rust | Tight RAM or edge-style experiments with no disk spill and no background reflection. |
| `benchmark-replay` | CPU-only | portable Rust | Deterministic replay and benchmark runs with streaming and background reflection disabled. |

Aliases such as `cpu`, `local-cpu`, `gpu`, `cuda`, `lowmem`, `constrained`,
`benchmark`, and `replay` parse into the same stable profile kinds. Future CLI
or config selection should call `parse_research_deployment_profile()` before
constructing a runtime plan.

## Resource Guards

Each profile carries explicit limits for:

- context tokens
- generation tokens
- KV tokens
- concurrent requests
- background reflection workers
- stream buffer tokens
- stream chunks in flight
- cancellation polling interval
- request timeout
- streaming permission
- background-reflection permission
- disk-spill permission

`ResearchDeploymentProfile::guard()` evaluates a
`ResearchDeploymentRequest` and returns:

- `allow` when the request is inside the profile budget
- `backpressure` when streaming/cancellation/timeout pressure exceeds the
  profile's soft guard
- `reject` when context, KV, concurrency, disabled feature, license, or
  durable-write boundaries would be crossed

Guard reports are read-only and include a stable evidence digest so issue and
PR comments can cite the exact decision without storing raw prompts or private
payloads.

## Write Guards

All profiles default to `preview-only` write mode:

- durable memory writes: disabled
- genome writes: disabled
- experiment-ledger writes: disabled
- operator approval: required
- privacy gate: required
- preview-to-write gate: required

Any request for durable memory, genome, or experiment-ledger writes is rejected
with `durable_writes_require_preview_to_write_gate`. Future promotion to
`approval-gated` must pass the preview-to-write graduation checklist, rollback
plan, privacy/license checks, and maintainer/operator approval.

## Operator Health

`ResearchDeploymentProfile::operator_health()` emits the active profile, device
class, adapter hint, write mode, resource limits, license posture, and
write-disabled state. Health output remains read-only:

- `read_only=true`
- `write_allowed=false`
- `durable_write_allowed=false`
- `applied=false`

This gives contributors a local deployment posture they can inspect before a
run starts, while keeping self-evolution changes gated by explicit evidence and
human approval.

## Issue #62 Research Sandbox Runbook

issue62_research_targets=local|wsl|container|small-vps

| Target | Research mode | Persistent state | Local-only data |
| --- | --- | --- | --- |
| `local` | host checkout, no production service | `disk_kv_cache`, `runtime_state`, `experiment_ledger_preview`, `redacted_evidence_packets` | `model_artifacts`, `raw_traces`, `secrets`, `private_prompts` |
| `wsl` | WSL2 checkout, no shared host secrets | same as local | same as local |
| `container` | disposable container volume | same as local | same as local |
| `small-vps` | non-commercial preview host | same as local | same as local |

issue62_persistent_state=disk_kv_cache|runtime_state|experiment_ledger_preview|redacted_evidence_packets

issue62_local_only_data=model_artifacts|raw_traces|secrets|private_prompts

| Field | Required value |
| --- | --- |
| `noncommercial_only` | `true` |
| `contributor_pr_only` | `true` |
| `maintainer_approval_required` | `true` |
| `private_trace_publish_allowed` | `false` |
| `redacted_issue_comment_ready` | `true` |
| `wipe_test_state_supported` | `true` |
| `preview_only` | `true` |
| `write_allowed` | `false` |
| `durable_write_allowed` | `false` |
| `applied` | `false` |

issue62_evidence_packet_command=norion-cli evidence-packet --research-sandbox-input PATH

issue62_cleanup_steps=stop_runtime|delete_test_state_dir|delete_container_volume|keep_redacted_evidence_packet

issue62_policy_refs=#11|#27|#41

issue62_contributor_path=fork_or_branch_to_pr_only_no_direct_merge

## Completion Boundary

#78 is a completed baseline when the deterministic profile templates, parser,
guard reports, disabled-write defaults, operator-health evidence, focused
tests, and this policy document land. Follow-up work may wire these profiles
into CLI/config/runtime selection, but that wiring must keep the same
GPL-3.0-compatible and preview-only defaults.
