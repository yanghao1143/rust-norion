# Contributing

rust-norion accepts public collaboration through issues and pull requests.

The repository is released under the GNU General Public License v3.0.
Commercial use, deployment research, modification, and redistribution are
allowed under GPL-3.0 terms. Derivative works and redistributed modifications
must remain open source under GPL-3.0-compatible terms.

## Ground Rules

- All changes must be reviewed and approved by the repository owner or
  maintainer before merge.
- Protected branches require a pull request, CODEOWNER approval from
  `@yanghao1143`, and approval of the latest reviewable push.
- The default branch also has an active repository ruleset. Public
  contributors must go through the pull request gates; repository-admin bypass
  is reserved for owner-operated deadlock recovery after checks pass.
- Do not push directly to protected branches.
- Base normal work on `main`, create a topic branch or fork, and open a pull
  request back to `main`.
- Protected-branch merges use the repository's configured squash merge path.
  Merge commits and rebase merges are disabled.
- Repository auto-merge is disabled. A maintainer must explicitly merge after
  all gates pass.
- Keep contributions compatible with the repository license: GPL-3.0.
- Pull requests do not bypass maintainer review, branch protection, validation
  gates, attribution requirements, or third-party license obligations.
- Do not copy AGPL, proprietary, commercial-restricted, private, generated, or
  otherwise incompatible code, tests, prompts, generated files, or assets into
  this repository.
- `fortunto2/rust-code` may be used as an MIT-licensed reference. Any code port
  requires explicit attribution and review.
- `Kuberwastaken/claurst` is a GPL-3.0 reference. Any code import or port
  requires a dedicated issue or pull request, explicit attribution,
  compatibility review, and maintainer approval.
- Do not commit local state, memory databases, `.ndkv` files, model weights,
  credentials, logs, generated `target` directories, or private datasets.

## Issue-First Development

Open or link an issue for non-trivial work. Use the roadmap tracker when the
change affects memory, routing, runtime service, self-evolution, genome,
governance, agent-team, or tooling behavior.

Small typo or documentation fixes can go straight to a pull request, but the
maintainer may still ask for an issue when the change affects project policy or
architecture.

## GitHub Permissions and Merge Flow

Use GitHub as the primary review and merge surface. Gitee is a main-branch
mirror for access and synchronization, not a place to keep feature branches.

Public contributors do not need repository write access. External contributors
should:

- fork the repository or create a feature branch outside the protected branch
- open a pull request against `main`
- link the issue or roadmap item, unless the change is a small typo fix
- fill in the validation, safety, and provenance checklist in the PR template
- wait for CI, maintainer review, CODEOWNER approval, and conversation
  resolution

Contributors do not merge their own changes into protected branches. Repository
permissions are separate from community roles:

- Contributor / Trusted Contributor: no protected-branch merge permission.
- Reviewer / Module Collaborator: may review or triage within a scoped lane, but
  cannot bypass CODEOWNER review, CI, latest-push approval, or maintainer merge.
- Maintainer: may be granted repository permission only after an explicit owner
  decision and any required CODEOWNERS or branch-protection update.
- Owner / CODEOWNER: remains the final protected-branch gate.

Ordinary contributors should not receive repository `write`, `maintain`, or
`admin` permission. Use issue discussion, fork-based PRs, review comments, and
triage access for normal collaboration. Expanding maintainer or merge authority
requires an explicit owner decision and a matching update to CODEOWNERS,
collaborator permissions, and branch-protection settings.

Maintainers should merge only after the protected-branch requirements pass:

- PR review is approved, including CODEOWNER approval where required
- the latest reviewable push has approval
- required checks pass against the latest branch state
- required conversations are resolved
- license, clean-room, and safety gates are satisfied

The repository policy is squash merge only for protected-branch PRs. Merge
commits and rebase merges are disabled, auto-merge is disabled, and source
branches are deleted after merge. Accidental Gitee feature branches should be
deleted after verification so the mirror remains `main`-only.

GitHub Actions also follows least privilege for public collaboration: the
default workflow token is read-only, workflows cannot create or approve pull
request reviews, and first-time external contributors require workflow approval
before CI runs.

## Contributor Recognition

Contributors are encouraged to make their work visible. A pull request may
include a short contributor card:

- preferred display name and GitHub/Gitee handle
- contribution lane: core, memory, runtime, docs, benchmark, governance,
  runbook, community, or research reproduction
- one-sentence impact summary
- validation command or reproduction note
- optional link to an issue, demo, trace, benchmark, or design note
- optional showcase request: README, Hall of Fame, release notes, module docs,
  or none

Merged contributions can be referenced from
[docs/contributor-zone.md](docs/contributor-zone.md), release notes, runbooks,
or roadmap updates when they materially improve a module or developer workflow.

Recognition does not bypass review, branch protection, license compatibility,
validation gates, or maintainer approval. It is a way to give visible credit
for useful work while keeping the repository safe and reviewable.

Contributor roles, reviewer promotion, module collaborator scope, maintainer
promotion, recusal, and loss-of-trust rules are defined in
[docs/governance/contributor-roles-and-review.md](docs/governance/contributor-roles-and-review.md).
Reviewer status is earned, scoped, and reversible. It does not replace
CODEOWNER or repository-owner approval on protected branches.

## Pull Requests

### Contribution Flow

1. Sync from `main`.
2. Create a topic branch or fork.
3. Open a pull request into `main`.
4. Let CI, CODEOWNER review, latest-push approval, and conversation resolution
   finish.
5. The owner or maintainer explicitly merges by squash merge after all gates
   pass.

Every pull request should include:

- A short description of the behavioral change.
- The validation commands that passed.
- Any rollback plan for self-evolving or memory-admission changes.
- Trace or benchmark evidence when changing routing, memory, genome, runtime,
  self-evolution, task hierarchy, or durable-state behavior.
- A clean-room note for changes inspired by external projects or papers.
- A note confirming that external references, if used, are documented with
  license and attribution details.

## Safety Gates

For memory, genome, routing, runtime, and self-evolution changes:

- keep durable memory/genome writes preview-only until writer gates and
  maintainer approval pass
- include rollback anchors for state-changing plans
- avoid raw prompt/answer leakage in trace, benchmark, and review summaries
- include focused tests or benchmark gates that cover the changed behavior
- document any external reference used for implementation decisions

The maintainer may ask for focused tests, benchmark evidence, or a smaller
scope before approving a merge.

See [NOTICE.md](NOTICE.md) and
[docs/governance/public-collaboration.md](docs/governance/public-collaboration.md)
for license, clean-room, and branch-protection details.
See
[docs/governance/open-source-community.md](docs/governance/open-source-community.md)
for the focused community strategy and contributor path.
