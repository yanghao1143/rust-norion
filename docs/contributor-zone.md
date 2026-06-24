# Contributor Zone

rust-norion is not looking for anonymous drive-by patches. It is looking for
people who want to help build a serious Rust AI control-layer project and have
their work remembered.

这里是贡献者专区。它的目的不是客套地说一句“欢迎 PR”，而是让贡献者明确知道：

- 我能做什么；
- 做完之后如何被看见；
- 贡献如何变成履历、署名、荣誉和影响力；
- 什么样的人可以升级成 reviewer / module collaborator / maintainer；
- 仓库如何保持秩序，不被乱七八糟的 PR 拖垮。

## Why Join

rust-norion 的机会在于：它不是又一个模型推理框架，而是在做模型外层的控制系统：记忆、路由、反思、runtime 边界、DNA-style reasoning genes、自进化门禁和可审计本地部署。

对贡献者来说，这意味着你可以展示的不是“我修了一个 typo”，而是：

- 我参与了一个 Rust AI control-plane 的早期架构；
- 我负责过某个模块的设计、测试或验证；
- 我写过可复现的 benchmark / runbook / runtime adapter；
- 我参与过开源治理、审核规则和 contributor ladder 的建设；
- 我的名字能出现在 README、贡献者专区、release notes、runbook 或模块文档里。

这就是这个专区要给足的“面子”：贡献要能被看见、被引用、被认领、被长期记录。

## Start Here

新贡献者建议按这个顺序进入：

1. 跑通 README 里的 no-model quickstart。
2. 选择一个 showcase lane。
3. 开 issue 或在已有 issue 下认领一个小任务。
4. 发 PR 时填写 Contributor Card。
5. 合并后请求把成果加入 Hall of Fame、release notes 或对应模块文档。

第一次贡献不需要很大。小而准、能验证、能复现，比“大 PR 乱改一堆”更容易被记住。

## Showcase Lanes

| Lane | What Counts | First Wins |
| --- | --- | --- |
| Core control layer | routing, hierarchy, reflection, scheduler, writer gates | focused tests, trace fields, clearer failure output |
| Memory | KV/Gist memory, experience retrieval, hygiene, semantic index | retrieval examples, state inspection docs, fixture cleanup |
| Runtime boundary | `ModelRuntime`, manifests, command runtime, conformance gates | ABI docs, reference-kernel tests, device profile evidence |
| Docs | README, architecture notes, FAQ, bilingual guides | quickstart fixes, diagrams, glossary, troubleshooting |
| Benchmark / CI | reproducible gates, trace schema checks, regression samples | benchmark fixtures, expected output snippets, CI docs |
| Governance | clean-room notes, branch rules, privacy/redaction, writer policy | checklist updates, review templates, safer examples |
| Runbooks | local/remote model chain, SmartSteam Forge, RustGPT Lab | tested command paths, failure recovery notes, screenshots |
| Community | issue triage, task splitting, PR review, contributor onboarding | label suggestions, weekly digest, contributor summaries |
| Research | paper notes, experiment reproduction, technical comparisons | reproduction logs, literature mapping, arXiv draft review |

## Contributor Card

Add this block to a pull request when you want your work to be easy to showcase:

```markdown
Contributor Card

- Name:
- GitHub / Gitee:
- Lane: core | memory | runtime | docs | benchmark | governance | runbook | community | research
- Impact:
- Validation:
- Related issue / doc / demo:
- Showcase request: README | Hall of Fame | release notes | module docs | none
```

Good examples:

- Impact: "added a no-model Windows quickstart with expected output"
- Impact: "documented the command-runtime ABI boundary for adapter authors"
- Impact: "added a focused regression fixture for memory hygiene inspection"
- Validation: "`cargo check -q --workspace` and local markdown link check passed"

## Contributor Showcase

Merged contributors can be recognized in several places:

| Place | Best For | Rule |
| --- | --- | --- |
| Hall of Fame on this page | visible project-level credit | merged PR or explicit permission |
| Release notes | meaningful shipped changes | maintainer selects notable contributions |
| Module docs | module ownership and expertise | repeated work in one lane |
| README links | high-impact contributor-facing assets | short, stable, useful to newcomers |
| Runbooks | operational or reproduction work | command tested and dated |

Do not invent contributors. Do not add someone without merged work or explicit
permission. Keep credit concrete: name the impact and link the PR, issue, or
doc.

## Hall of Fame

Use this table when public credits are added:

| Contributor | Lane | Highlight | Links |
| --- | --- | --- | --- |
| `@handle` | runtime | Connected a command-runtime conformance example. | PR / issue / doc |
| `@handle` | docs | Wrote the first Windows no-model quickstart. | PR / issue / doc |

## Badges

Badges are lightweight recognition labels. They can appear in PR comments,
release notes, or Hall of Fame entries.

| Badge | Meaning |
| --- | --- |
| `First Landing` | First merged contribution. |
| `Control Layer Builder` | Routing, hierarchy, reflection, scheduler, or writer-gate work. |
| `Memory Keeper` | KV/Gist, retrieval, hygiene, or semantic-index work. |
| `Runtime Adapter` | Backend, manifest, command runtime, conformance, or device profile work. |
| `Docs Pilot` | Onboarding, tutorial, runbook, FAQ, diagram, or bilingual docs. |
| `Benchmark Smith` | Benchmark, trace, fixture, CI, or reproducibility work. |
| `Governance Guard` | License, clean-room, privacy, writer gate, or review process work. |
| `Community Signal` | Issue triage, onboarding, task splitting, digest, or release-note support. |

Badges are recognition, not authority. Review and merge rights follow the role
rules in [Contributor Roles and Review Governance](governance/contributor-roles-and-review.md).

## Role Ladder

| Role | What It Means | How To Earn It |
| --- | --- | --- |
| Contributor | Has useful issues, docs, tests, examples, reviews, or PRs. | Land focused work with validation. |
| Trusted Contributor | Repeatedly ships narrow, reviewable work in one or more lanes. | 3+ merged PRs or equivalent docs/runbook/review impact. |
| Reviewer | Can provide trusted review recommendations in specific areas. | Sustained quality, good judgment, and maintainer invitation. |
| Module Collaborator | Helps triage and review a module or lane. | Repeated module work plus reliable reviews and issue shaping. |
| Maintainer | Can approve scoped areas when owner policy allows. | Owner decision, sustained trust, and protected-branch compliance. |

Detailed promotion, review, conflict, and removal rules live in
[Contributor Roles and Review Governance](governance/contributor-roles-and-review.md).

## How To Earn Review Trust

Review trust is earned by behavior, not by asking for a title.

Strong signals:

- PRs are small enough to review.
- Validation commands are real and repeatable.
- External references are documented clean-room style.
- The contributor explains rollback and state-write risk.
- Reviews catch real bugs or missing tests without bikeshedding.
- The contributor improves the repo's ability to accept more contributors.

Weak signals:

- large PRs with unrelated changes;
- code copied from unclear sources;
- benchmark claims without commands or outputs;
- touching memory/genome/self-evolution writes without gates;
- arguing for merge before tests, review, or license questions are settled.

## Good First Work

Good starter work:

- convert a long command into a tested runbook
- add expected output to a quickstart
- write a focused failing test for one gate or parser
- document a runtime adapter boundary
- improve an error message with a regression test
- reproduce a benchmark and record the machine/profile context
- triage related issues into a small implementation checklist
- translate a useful doc section while preserving technical terms

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

This helps maintainers turn merged work into release notes, roadmap updates, or
Hall of Fame entries without guessing the contributor's intent.

## Community Rhythm

Once repeat contributors appear, use a lightweight rhythm:

- weekly issue sweep: label good first issues, blockers, and review-needed PRs
- biweekly contributor digest: merged work, active lanes, next tasks
- monthly roadmap sync: keep control-layer scope tight and avoid runtime drift
- release notes: name meaningful contributors and link their work

The tone should be generous. The bar should stay real. Credit means more when
it points to merged, reproducible, reviewable work.
