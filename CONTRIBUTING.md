# Contributing

rust-norion accepts public collaboration through issues and pull requests.

The repository is public for non-commercial research, education, evaluation,
benchmarking, and experimental deployment. Commercial use, commercial
deployment approval, and license exceptions require explicit written permission
from the copyright holder.

## Ground Rules

- All changes must be reviewed and approved by the repository owner or
  maintainer before merge.
- Protected branches require a pull request, CODEOWNER approval from
  `@yanghao1143`, and approval of the latest reviewable push.
- Do not push directly to protected branches.
- Keep contributions compatible with the repository license: non-commercial
  research, education, evaluation, and experimental deployment only.
- Pull requests do not request or imply commercial-use permission.
- Do not copy GPL, AGPL, commercial, or otherwise incompatible code, tests,
  prompts, generated files, or assets into this repository.
- `fortunto2/rust-code` may be used as an MIT-licensed reference. Any code port
  requires explicit attribution and review.
- `Kuberwastaken/claurst` may be used only for clean-room architecture inspiration
  unless the project explicitly accepts GPL-3.0 obligations in a written
  decision.
- Do not commit local state, memory databases, `.ndkv` files, model weights,
  credentials, logs, generated `target` directories, or private datasets.

## Issue-First Development

Open or link an issue for non-trivial work. Use the roadmap tracker when the
change affects memory, routing, runtime service, self-evolution, genome,
governance, agent-team, or tooling behavior.

Small typo or documentation fixes can go straight to a pull request, but the
maintainer may still ask for an issue when the change affects project policy or
architecture.

## Pull Requests

Every pull request should include:

- A short description of the behavioral change.
- The validation commands that passed.
- Any rollback plan for self-evolving or memory-admission changes.
- Trace or benchmark evidence when changing routing, memory, genome, runtime,
  self-evolution, task hierarchy, or durable-state behavior.
- A clean-room note for changes inspired by external projects or papers.
- A note confirming that no commercial-use permission is being requested by the
  pull request itself.

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
