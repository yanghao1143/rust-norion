# Contributor Zone

rust-norion wants many contributors, but it should still feel sharp, serious,
and worth putting your name on. This page is the public contribution area: a
place for people to find a lane, make visible work, and get proper credit when
their work lands.

中文说明：这里是贡献者专区。目标很直接：让贡献者有方向可认领、有成果可展示、有署名可沉淀、有晋级路径可期待。

## Contributor Card

Add this block to a pull request when you want the contribution to be easy to
showcase:

```markdown
Contributor Card

- Name:
- GitHub / Gitee:
- Lane: core | memory | runtime | docs | benchmark | governance | runbook | community | research
- Impact:
- Validation:
- Related issue / doc / demo:
```

Good cards are short and concrete:

- Impact: "added a no-model quickstart for Windows contributors"
- Validation: "`cargo check -q --workspace` and link check passed"
- Related issue: "#123" or a docs link

## Showcase Lanes

| Lane | What Counts | Good First Contributions |
| --- | --- | --- |
| Core control layer | routing, hierarchy, reflection, scheduler, writer gates | focused tests, trace fields, clearer failure output |
| Memory | KV/Gist memory, experience retrieval, hygiene, semantic index | fixture cleanup, retrieval examples, state inspection docs |
| Runtime boundary | `ModelRuntime`, manifests, command runtime, conformance gates | ABI docs, reference-kernel tests, device profile evidence |
| Docs | README, architecture notes, FAQ, bilingual guides | quickstart fixes, diagrams, glossary, troubleshooting |
| Benchmark / CI | reproducible gates, trace schema checks, regression samples | small benchmark fixtures, expected output snippets |
| Governance | clean-room notes, branch rules, privacy/redaction, writer policy | checklist updates, review templates, safer examples |
| Runbooks | local/remote model chain, SmartSteam Forge, RustGPT Lab | tested command paths, screenshots, failure recovery notes |
| Community | issue triage, task splitting, PR review, contributor onboarding | label suggestions, weekly digest, contributor summaries |
| Research | paper notes, experiment reproduction, technical comparisons | reproduction logs, literature mapping, arXiv draft review |

## Hall of Fame Format

Do not invent contributors. Add people only after merged work or explicit
permission. Use this format when the project starts curating public credits:

| Contributor | Lane | Highlight | Links |
| --- | --- | --- | --- |
| `@handle` | runtime | Connected a command-runtime conformance example. | PR / issue / doc |
| `@handle` | docs | Wrote the first Windows no-model quickstart. | PR / issue / doc |

The README should stay short. Longer recognition, notes, and screenshots can
live here or in release notes.

## Badges and Credit Ideas

These are lightweight ways to give face without turning the repository into a
manual bookkeeping burden:

- `First Landing`: first merged PR.
- `Control Layer Builder`: core routing, hierarchy, reflection, scheduler, or
  writer-gate contribution.
- `Memory Keeper`: memory, retrieval, hygiene, KV/Gist, or semantic-index work.
- `Runtime Adapter`: backend, manifest, conformance, command runtime, or device
  profile contribution.
- `Docs Pilot`: onboarding, tutorial, runbook, FAQ, diagram, or bilingual docs.
- `Benchmark Smith`: benchmark, trace, fixture, CI, or reproducibility work.
- `Governance Guard`: license, clean-room, privacy, writer gate, or review
  process contribution.
- `Community Signal`: issue triage, contributor onboarding, task splitting, or
  release-note support.

Badges can appear in release notes, PR comments, or this page. They do not
grant merge rights by themselves.

## Contribution Ladder

| Stage | What It Means | How To Move Up |
| --- | --- | --- |
| Contributor | Has opened useful issues, docs, tests, examples, or PRs. | Land focused changes with clear validation. |
| Module Collaborator | Repeatedly contributes to one area and helps triage or review it. | Show module familiarity and reliable judgment. |
| Maintainer | Trusted to approve scoped areas after sustained quality work. | Requires owner decision and protected-branch policy. |

Maintainer approval, branch protection, attribution, license compatibility, and
validation gates still apply at every stage.

## How To Pick Work

Start small and visible:

1. Pick one lane from the table above.
2. Open or comment on an issue with the exact file/module you want to touch.
3. Keep the first PR narrow enough to review quickly.
4. Include a Contributor Card and validation command.
5. Ask for the work to be listed here when it materially helps the project.

Good starter work:

- convert a long command into a tested runbook
- add expected output to a quickstart
- write a focused failing test for one gate or parser
- document a runtime adapter boundary
- improve an error message with a regression test
- reproduce a benchmark and record the machine/profile context
- triage related issues into a small implementation checklist

## PR Showcase Template

Use this in a PR description when the change is meant to be visible:

```markdown
## Contributor Showcase

- What I improved:
- Why it matters:
- Who should use it:
- Validation:
- Follow-up work others can claim:
```

This helps the maintainer turn merged work into release notes, roadmap updates,
or a Hall of Fame entry without guessing the contributor's intent.

## Community Rhythm

Suggested lightweight rhythm once there are repeat contributors:

- weekly issue sweep: label good first issues and stale blockers
- biweekly contributor digest: merged work, active lanes, next tasks
- monthly roadmap sync: keep control-layer scope tight and avoid broad runtime
  drift
- release notes: name meaningful contributors and link their PRs

Keep the tone generous, but keep the bar real: visible credit is strongest when
it points to merged, reproducible, reviewable work.
