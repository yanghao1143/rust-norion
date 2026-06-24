# Current HEAD Reconciliation

Date: 2026-06-24

Issue: #184 `[MVP M0] Reconcile current HEAD + branch cleanup before M1`

This report is the execution baseline for M1-M4. Future MVP work must start from
the current HEAD recorded here, not from the older `7f790da` snapshot.

## Baseline

| Item | Value |
| --- | --- |
| Work branch | `codex/m0-current-head-reconciliation` |
| Current HEAD | `6a17a77978e29d43855940c7a46495c12ebbe6af` |
| Current short HEAD | `6a17a7797 docs: update NewAPI model allowlist evidence` |
| Old anchor | `7f790da8b7f259a948e305539fdd5480622f5c12` |
| Old anchor short | `7f790da` |
| Total commits at current HEAD | `518` |
| Divergence, `7f790da...HEAD` | `0 281` |
| Commits in `7f790da..HEAD` | `281` |
| Commits in `HEAD..7f790da` | `0` |

Conclusion: `7f790da` is an ancestor of the current HEAD. Reverting the MVP plan
to `7f790da` would discard 281 later commits and a large amount of already
integrated architecture, tooling, runtime, governance, and NewAPI hygiene work.

## Commands Used

```powershell
git fetch --all --prune
git switch codex/m0-current-head-reconciliation
git rev-parse HEAD
git rev-parse 7f790da
git rev-list --count HEAD
git rev-list --left-right --count 7f790da...HEAD
git rev-list --count 7f790da..HEAD
git rev-list --count HEAD..7f790da
git branch -a --contains HEAD
git branch --list "codex/*" -vv
git worktree list --porcelain
gh pr view 175 --json number,state,title,headRefName,baseRefName,mergeable,isDraft,url,updatedAt,commits
gh pr view 178 --json number,state,title,headRefName,baseRefName,mergeable,isDraft,url,updatedAt,commits
git rev-list --left-right --count origin/codex/task-aware-hierarchy-mutations...gitee/codex/task-aware-hierarchy-mutations
git rev-list --left-right --count origin/main...gitee/main
```

## Key Work Since `7f790da`

The range `7f790da..HEAD` changes 1475 files in the full diff, including 1469
files under `crates`, `src`, `tools`, `docs`, `.github`, `Cargo.toml`, and
`Cargo.lock`. The diff is too broad to treat as disposable scaffolding.

Major reusable areas:

- Modular crates: `crates/norion-core`, `crates/norion-memory`,
  `crates/norion-agent`, `crates/norion-service`, `crates/norion-cli`,
  `crates/norion-test`, and `crates/norion-eval`.
- Root runtime and evaluation surfaces under `src/**`, including adaptive state,
  agent team flow, benchmarks, runtime evidence, memory reuse, routing evidence,
  self-evolution admission, and task-aware hierarchy mutation paths.
- Tooling under `tools/evolution-loop`, `tools/smartsteam-forge`,
  `tools/gemma-chain`, `tools/rustgpt-lab`, and `tools/model-pool-advice-core`.
- GitHub governance and CI: CODEOWNERS, issue templates, PR template, and PR
  validation workflow.
- Coordination and runbooks: parallel workstreams, Gemma local chain, Apple
  Silicon model pool, evolution-loop evaluation, SmartSteam daemon operation,
  NewAPI model discovery, and secret-safe NewAPI model pool guidance.
- Governance artifacts: branch protection checklist, clean-room audit,
  self-goal/evolution gates, coding eval profiles, privacy redaction corpus,
  writer-gate guidance, and memory consolidation plans.

Do not reimplement these areas blindly in M1-M4. Start by checking whether a
required capability already exists in one of the modules or runbooks above.

## Open PR Status

| PR | State | Head | Base | Mergeable | Draft | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| #175 | `OPEN` | `codex/task-aware-hierarchy-mutations` | `codex-runtime-device-abi` | `MERGEABLE` | `false` | Contains current HEAD `6a17a7797` and five commits through NewAPI allowlist evidence. |
| #178 | `OPEN` | `codex/daemon-recovery-main` | `main` | `MERGEABLE` | `false` | Separate daemon recovery line, two commits, not the M0/M1-M4 baseline. |

PR #175 is the closest public GitHub representation of the current active line.
PR #178 should not be merged into this M0 branch as part of reconciliation; it is
tracked separately because its base is `main`, not `codex-runtime-device-abi`.

## Branch And Worktree Disposition

`git branch -a --contains HEAD` shows the current HEAD is contained by:

- `codex/m0-current-head-reconciliation`
- `codex/m1-model-registry-mvp`
- `codex/m2-inference-backend-mvp`
- `codex/m3-outcome-routing-mvp`
- `codex/m4-profile-scoring-mvp`
- `codex/task-aware-hierarchy-mutations`
- `origin/codex/task-aware-hierarchy-mutations`
- `gitee/codex/task-aware-hierarchy-mutations`

Recommendations:

| Branch/worktree family | Current observation | Disposition |
| --- | --- | --- |
| `codex/m0-current-head-reconciliation` | Current M0 documentation branch at `6a17a7797` before this report commit. | Keep; this report is the only intended change. |
| `codex/m1-model-registry-mvp` through `codex/m4-profile-scoring-mvp` | All point at `6a17a7797`; several detached worktrees also point at the same commit. | Keep as MVP starting branches, but treat them as empty slices until each receives scoped commits. |
| `codex/task-aware-hierarchy-mutations` | Local branch checked out in `D:/rust-norion`; origin and gitee refs match current HEAD. | Keep as active PR #175 line. Do not rewrite. |
| `codex/daemon-recovery-main` | Checked out at `C:/tmp/rust-norion-daemon-main`; PR #178 line based on `main`. | Keep separate; review/merge via #178 policy, not M0. |
| `codex/newapi-model-discovery`, `codex/newapi-model-pool`, `codex/newapi-secret-safe-docs` | Local worktrees exist and their content is represented by commits now visible in #175/current HEAD. | Prefer retiring or archiving after confirming no unpushed local-only follow-up remains. |
| Older local branches without current remotes, for example `codex/r83-*`, `codex/r97-*`, and `codex/self-goal-queue-apply-plan` | They are merged into current HEAD or marked with gone upstreams. | Do not delete during M0; mark for later cleanup after owners confirm no active worktree depends on them. |
| Gitee repair/link branches | Some are based on `gitee/main` or `gitee/codex-runtime-device-abi`. | Keep isolated from MVP work; use only for mirror hygiene. |

No branch deletion, force push, reset, or history rewrite is part of this M0
issue.

## M1-M4 Reuse And Conflict Points

M1 model registry:

- Reuse NewAPI discovery and allowlist evidence from
  `docs/runbooks/newapi-model-discovery.md`,
  `docs/runbooks/newapi-model-pool-secret-safe.md`, and
  `docs/runbooks/newapi-model-pool.env.example`.
- Reuse existing model-pool and routing surfaces under `tools/evolution-loop`,
  `tools/rustgpt-lab`, and `tools/smartsteam-forge`.
- Conflict risk: do not add hardcoded NewAPI credentials or one-off provider
  config paths that bypass the existing secret-safe runbook.

M2 inference backend:

- Reuse `tools/smartsteam-forge`, `tools/gemma-chain`, `tools/rustgpt-lab`, and
  root runtime/provider code that already models health, readiness, SSE, session
  control, and model-pool status.
- Conflict risk: do not submit prompts or restart model/backend processes unless
  the prompt gate and runtime readiness runbooks say the chain is ready.

M3 outcome routing:

- Reuse task-aware hierarchy mutation work at current HEAD, root routing traces,
  `src/benchmark/summary_gate/**`, `src/agent_team/**`, and
  `crates/norion-agent/**`.
- Conflict risk: do not duplicate routing state or bypass existing gate evidence
  and review-packet flows.

M4 profile scoring:

- Reuse coding eval profiles, benchmark gates, `crates/norion-eval`,
  `crates/norion-test`, and NewAPI/model-pool evaluation surfaces.
- Conflict risk: profile scoring must line up with existing eval artifacts and
  not create an incompatible score schema without an adapter/migration plan.

## GitHub And Gitee Sync

Observed remotes:

- `origin`: `https://github.com/yanghao1143/rust-norion.git`
- `gitee`: `https://gitee.com/babalibaba/rust-norion.git`

Observed sync checks:

- `origin/codex/task-aware-hierarchy-mutations...gitee/codex/task-aware-hierarchy-mutations`
  returned `0 0`; the current active PR branch is mirrored between GitHub and
  Gitee.
- `origin/main...gitee/main` returned `1 3`; main branches are not identical.
  Treat main sync as a separate mirror task and do not fold it into M0.
- `gitee/codex/daemon-recovery-main` is at `f4a25c008`, while
  `origin/codex/daemon-recovery-main` is at `3afcca09c`; #178 is therefore a
  GitHub PR line and should not be assumed mirrored one-for-one on Gitee.

## NewAPI Secret Hygiene

NewAPI keys and provider credentials must remain in environment variables or a
secret manager only. Do not commit real keys, copied `.env` files, shell history,
HTTP traces with authorization headers, or generated config containing secrets.

Allowed repository artifacts:

- Secret-free examples such as `docs/runbooks/newapi-model-pool.env.example`.
- Runbooks that name environment variable keys without real values.
- Evidence that records model names, provider behavior, and allowlist results
  after redaction.

Disallowed artifacts:

- Real `NEWAPI_*` secrets.
- Bearer tokens or API keys in docs, tests, fixtures, logs, screenshots, or PR
  comments.
- NewAPI endpoint dumps that include request headers or credential-bearing URLs.

## Execution Rule For #179 And M1-M4

External executors must read this report before starting M1-M4. The execution
anchor is the current HEAD lineage recorded above, advanced by the M0 report
commit. The old `7f790da` anchor is historical evidence only and must not be used
as a restore point, reset target, or cherry-pick base for MVP work.

## M0 Validation Plan

This issue is documentation-only. The required local validation is:

```powershell
git diff --check
cargo test -q --manifest-path tools/evolution-loop/Cargo.toml --target-dir target\m0-reconciliation-check
```

If the Cargo gate fails, record the failure in the integration handoff and prefer
fixing this report over changing engine/runtime implementation code.
