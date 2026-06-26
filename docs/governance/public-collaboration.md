# Public Collaboration Governance

This repository is public so researchers and contributors can inspect,
evaluate, deploy, and improve rust-norion together under the GNU General Public
License v3.0.

## Allowed Use

Allowed without separate written permission:

- research and education
- local evaluation and benchmarking
- reproducibility work
- safety analysis and validation
- experimental and production deployment under GPL-3.0 terms
- commercial use under GPL-3.0 terms
- issue discussion and pull request development

Derivative works and redistributed modifications must remain open source under
GPL-3.0-compatible terms. Operators remain responsible for applicable law,
privacy duties, safety requirements, third-party model or dataset licenses, and
local deployment policy.

## Merge Control

Public issues and pull requests are welcome, but contributors do not merge
directly. The default branch is protected. The current default branch is
`main`, and it requires:

- a pull request before merge
- at least one approving review
- CODEOWNER review from `@yanghao1143`
- approval of the latest reviewable push
- required status check `focused Rust crates`
- conversation resolution before merge
- required linear history
- no force pushes or branch deletion

Protected-branch merges use squash merge only. Merge commits, rebase merges,
and repository auto-merge are disabled, and GitHub deletes the head branch after
merge. Contributors submit pull requests; the owner or maintainer explicitly
performs the merge after all gates pass.

The repository also has an active default-branch ruleset named
`main contributor merge gate`. Public contributors must pass the pull request,
squash-only, code-owner review, latest-push approval, required status check,
linear-history, deletion, and non-fast-forward protections for `main`.
Repository-admin bypass is limited to owner-operated deadlock recovery after
checks pass.

## Contributor Permission Model

Contribution access and merge authority are intentionally separate:

- Public contributors can open issues and PRs without repository write access.
- Trusted contributors and reviewers can gain recognition and review scope
  without gaining protected-branch merge rights.
- Module collaborators may receive scoped triage or branch-work permission only
  after an explicit owner decision.
- Maintainer permission requires an explicit owner decision and any needed
  CODEOWNERS or branch-protection update.
- Gitee is a mirror and domestic access surface; GitHub `main` remains the
  authoritative merge surface unless the owner changes this policy explicitly.

Direct pushes to protected branches are not part of the contribution flow.
Routine contributions should use fork or topic branch pull requests into
`main`.

GitHub Actions is configured for least privilege: the default workflow token is
read-only, workflows cannot create or approve pull request reviews, and CI for
first-time external contributors requires maintainer approval before it runs.

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
- confirmation that external references, if used, include license and
  attribution notes
- confirmation that no local `.ndkv`, model weights, private data, generated
  output, logs, or credentials are included

## Clean-Room Reference Rules

External projects can inform design, but implementation must respect licenses.

- `fortunto2/rust-code`: MIT-compatible reference. Code imports or ports need
  attribution, review, and compatibility with the repository license.
- `Kuberwastaken/claurst`: GPL-3.0 reference. Code imports or ports require a
  dedicated issue or pull request, attribution, compatibility review, and
  maintainer approval.
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
- #27 GPL-3.0 license and contributor controls
- #77 preview-to-write graduation checklist

Durable memory or genome mutation must remain preview-only until the related
writer gates, validation evidence, rollback anchors, and maintainer approval
pass.

## Branch Protection Audit

As of 2026-06-26, GitHub reports the default branch as `main`. Branch
protection is enabled for `main` with required status checks, code-owner review,
last-push approval, required conversation resolution, required linear history,
and force-push/deletion blocks.

As of 2026-06-26, an active repository ruleset named
`main contributor merge gate` applies to the default branch. The effective
branch rules require pull requests, squash merge, code-owner review,
latest-push approval, required status check `focused Rust crates`, linear
history, and non-fast-forward/deletion blocks. Repository admins can bypass the
ruleset for owner-operated deadlock recovery.

As of 2026-06-26, the direct-collaborator audit reports only `@yanghao1143`
with admin permission. Routine public contributors do not need direct
repository access; they should contribute through issues and pull requests.

As of 2026-06-26, GitHub Actions default workflow permissions are read-only,
workflow pull request review approval is disabled, and first-time external
contributors require workflow approval before CI runs.
