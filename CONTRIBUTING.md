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
- Do not push directly to protected branches.
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
