# Public Collaboration Governance

This repository is public so researchers and contributors can inspect,
evaluate, deploy experimentally, and improve rust-norion together. It is not a
commercial-use grant.

## Allowed Use

Allowed without separate written permission:

- non-commercial research and education
- local evaluation and benchmarking
- reproducibility work
- safety analysis and validation
- experimental non-commercial deployment
- issue discussion and pull request development

Commercial use, paid hosted services, embedding in a commercial product,
selling or sublicensing the software, or using it primarily for commercial
advantage requires explicit written permission from the copyright holder.

## Merge Control

Public issues and pull requests are welcome, but contributors do not merge
directly. The default branch is protected and requires:

- a pull request before merge
- at least one approving review
- CODEOWNER review from `@yanghao1143`
- approval of the latest reviewable push
- required status check `focused Rust crates`
- conversation resolution before merge
- no force pushes or branch deletion

The branch protection checklist lives at
`docs/governance/branch-protection-checklist.md`.

The repository owner can reject, request changes, narrow scope, or revert a
contribution when it fails safety, license, roadmap, or validation gates.

## Required PR Evidence

Each PR must include:

- behavior summary
- linked issue or roadmap item, when applicable
- preview-to-write checklist evidence for any durable write or active behavior
  change request, using `docs/governance/preview-to-write-graduation.md`
- validation commands and outcomes
- rollback plan for self-evolution, memory, genome, routing, or durable-state
  changes
- confirmation that no commercial-use permission is requested
- confirmation that no local `.ndkv`, model weights, private data, generated
  output, logs, or credentials are included

## Clean-Room Reference Rules

External projects can inform design, but implementation must respect licenses.

- `fortunto2/rust-code`: MIT-compatible reference. Code imports or ports need
  attribution, review, and compatibility with the repository license.
- `Kuberwastaken/claurst`: GPL-3.0 reference. Treat it as architecture
  inspiration only. Do not copy source, tests, generated code, prompts, or
  assets unless the project explicitly accepts GPL-3.0 obligations in writing.
- Papers and public documentation: use as design input; cite in docs where a
  module relies on a published idea.

When in doubt, write a short design note describing the idea in rust-norion
terms before implementation. Avoid line-by-line translation from external code.

## Safety Gates

The roadmap issues define the main safety surfaces:

- #2 disk-backed memory admission
- #4 Reasoning Genome Chain and Gene Scissors
- #6 self-evolution gates and rollback ledger
- #7 closed-loop reflection to memory admission
- #10 benchmark and trace gates
- #11 public collaboration governance
- #20 experiment ledger and approval gates
- #27 non-commercial research license and contributor controls
- #77 preview-to-write graduation checklist

Durable memory or genome mutation must remain preview-only until the related
writer gates, validation evidence, rollback anchors, and maintainer approval
pass.

## Branch Protection Audit

As of 2026-06-21, GitHub reports the default branch as
`codex-runtime-device-abi`. Branch protection is enabled for that branch with
required status checks, code-owner review, last-push approval, admin
enforcement, required conversation resolution, and force-push/deletion blocks.
