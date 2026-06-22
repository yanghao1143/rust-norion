# Clean-Room Implementation Audit

R96 turns the R95 reference backlog into a source-provenance and license
contamination audit for issues #18, #40, and #60. It is still preview-only
governance: no external source is vendored, no model/data asset is imported, no
durable memory or genome write is enabled, and no license exception is granted
outside GPL-3.0.

The executable companion is `src/clean_room_audit.rs`, which exposes
`clean_room_audit_v1` and the compact evidence schema
`rust-norion-clean-room-audit-v1`.

## Audit Rules

| Rule | Result |
| --- | --- |
| Project-owned records can move to implementation spikes. | `ready_for_norion_owned_spike` |
| MIT/Apache/BSD references can guide behavior specs when attribution is recorded. | `spec_only` |
| MIT/Apache/BSD source-level work requires attribution, a scoped Norion-owned port plan, maintainer review, and no copied/vendored/generated external payload. | `ready_for_norion_owned_spike` or `attribution_port_plan_required` |
| GPL-3.0-compatible sources require attribution, a scoped Norion-owned port plan, and maintainer review before source-level work. | `ready_for_norion_owned_spike` or `attribution_port_plan_required` |
| Unknown-license or unverified sources cannot supply code, tests, prompts, schemas, docs text, assets, model artifacts, datasets, or generated fixtures. | `blocked_until_license_review` |
| Raw private prompts, raw answers, secrets, executable payload markers, or polluted context are rejected from provenance evidence. | `rejected_private_payload` |
| Copied/vendored external material without the required source policy is rejected or blocked. | `rejected_external_copy`, `blocked_gpl_source`, or `blocked_until_license_review` |

The audit distinguishes "concept can be discussed" from "source can be used."
No record permits blind copy, line-by-line translation, or mechanical porting.

## Default Manifest

| Record | Source | Target | Decision |
| --- | --- | --- | --- |
| `clean-room:rust-code:tool-contract-matrix` | `fortunto2/rust-code` MIT | `crates/norion-agent` / #18 | `spec_only` |
| `clean-room:rust-code:doctor-readiness` | `fortunto2/rust-code` MIT | `crates/norion-cli` / #18 | `spec_only` |
| `clean-room:claurst:permission-tool-assembly` | `Kuberwastaken/claurst` GPL-3.0 | `crates/norion-service` / #18 | `spec_only` until a port plan and attribution review land |
| `clean-room:claurst:bridge-boundary` | `Kuberwastaken/claurst` GPL-3.0 | future bridge boundary / #40 | `spec_only` until a port plan and attribution review land |
| `clean-room:candle:runtime-forward-kernel` | `huggingface/candle` Apache-2.0 | `ModelRuntimeForwardKernel` / #40 | `ready_for_norion_owned_spike` |
| `clean-room:mistral-rs:streaming-cancel-backpressure` | `mistral.rs` MIT | `MistralRsHttpRuntime` / #40 | `ready_for_norion_owned_spike` |
| `clean-room:cepe:chunked-context-scheduler` | CEPE MIT | `RecursiveScheduler` / #40 | `ready_for_norion_owned_spike` |
| `clean-room:streamingllm:residency-policy` | StreamingLLM MIT | `MemoryResidencyPlan` / #40 | `ready_for_norion_owned_spike` |
| `clean-room:splicetransformer:splice-fixtures` | SpliceTransformer Apache-2.0 | `DnaSplicer` / #40 | `ready_for_norion_owned_spike` |
| `clean-room:rapid:repair-plan-hold` | RAPID unknown license | `ExperienceRepairPlan` / #60 | `concept_only`, source import blocked |
| `clean-room:omni-dna:seqpack-hold` | Omni-DNA / SEQPACK unknown license | `DnaSplicer` / #60 | `concept_only`, source import blocked |

`fortunto2/rust-code` remains useful as an MIT architecture comparison, but the
R96 manifest keeps its rows spec-only until a concrete port plan and maintainer
review are attached to a future implementation issue.

`Kuberwastaken/claurst` is now license-compatible at the repository policy
level, but source-level work still requires a dedicated issue or pull request,
explicit attribution, maintainer review, and clean separation from private,
generated, or unreviewed material. Blind copy, line-by-line translation, and
mechanical porting remain rejected.

## Contamination Fixtures

The Rust tests in `src/clean_room_audit.rs` include fixtures that prove the
scanner catches:

- GPL source copy attempts;
- unknown-license source material;
- MIT/Apache/BSD source-level work missing attribution, port plan, or review;
- private or executable evidence markers;
- evidence packet lines that are not digest-only.

The default manifest is expected to pass because it contains only spec-only,
concept-only, or ready-for-Norion-owned-spike rows, with all writes disabled.

## Evidence Packet

Each manifest row emits a tab-delimited evidence packet:

`record_id`, `source_id`, `source_name`, SPDX or `NOASSERTION`, license class,
material kind, decision, source-copy flags, attribution/port/review flags,
Norion-owned status, and a `redaction-digest:*`.

These packets are safe to paste into issues and pull requests. They do not
include raw upstream source, prompts, private data, or model outputs.

## Follow-Up Routing

- #18 can proceed from architecture comparison to Norion-owned tool/service/CLI
  acceptance criteria, with `rust-code` spec-only rows and `claurst`
  concept-only rows.
- #40 can route verified permissive references into behavior specs for
  `DnaSplicer`, chunked KV, `RecursiveScheduler`, runtime adapters, and local
  model-service spikes.
- #60 now has a local Rust scanner surface: `default_clean_room_audit_report()`
  validates provenance records and produces compact source/license evidence.

R96 is complete when the clean-room audit tests pass, the roadmap and pursuit
queue advance to the next goal, and PR/issue evidence records that merge remains
subject to branch protection and maintainer approval.
