# Contributor Roles and Review Governance

This document defines how contributors can gain trust in rust-norion and how
review authority works without weakening repository safety.

中文摘要：贡献者可以晋升、可以参与审核、可以成为模块协作者；但受保护分支、CODEOWNER、许可证、验证门禁和仓库所有者最终审核权不被绕过。

## Principles

- Credit should be visible and generous.
- Authority should be earned, scoped, and reversible.
- Review power is not the same as merge power.
- High-risk areas need stricter review than docs or examples.
- The project should be easy to join without becoming easy to damage.

## Roles

| Role | Can Do | Cannot Do |
| --- | --- | --- |
| Contributor | Open issues, submit PRs, write docs/tests/runbooks, comment on reviews. | Merge PRs or approve protected-branch changes. |
| Trusted Contributor | Help shape issues, propose task splits, review low-risk PRs informally. | Override maintainer decisions or bypass validation. |
| Reviewer | Provide trusted review recommendations in approved lanes. | Merge without CODEOWNER/owner approval. |
| Module Collaborator | Triage and review a specific module/lane, help maintain docs and issue queues. | Approve outside their lane or bypass safety gates. |
| Maintainer | Approve scoped areas when owner policy and branch protection allow it. | Remove GPL/provenance/safety requirements or bypass protected branch policy. |
| Owner / CODEOWNER | Final merge gate for protected branches and protected surfaces. | Ignore license, privacy, or branch-protection obligations. |

## GitHub Permission Mapping

Community role names do not automatically grant repository permissions.

| Community Role | Default GitHub Permission | Merge Authority |
| --- | --- | --- |
| Contributor | None required; works from fork or topic branch PR. | None. |
| Trusted Contributor | Usually none or triage only, by owner decision. | None. |
| Reviewer | Triage or read/write only if explicitly granted for a lane. | Review recommendation only. |
| Module Collaborator | Triage or write for scoped branch work, if explicitly granted. | No protected-branch merge without owner/CODEOWNER gate. |
| Maintainer | Maintain/write/admin only after explicit owner decision. | May merge only when branch protection and owner policy allow it. |
| Owner / CODEOWNER | Admin / CODEOWNER. | Final protected-branch gate. |

The current default is conservative: contributors use issues and pull requests;
direct repository write permission is not needed for normal work. Any permission
increase should be documented in an issue or governance note, and it remains
revocable if validation, license, scope, or trust rules are bypassed.

## Promotion Criteria

| Promotion | Minimum Signals | Decision |
| --- | --- | --- |
| Contributor -> Trusted Contributor | 3+ useful merged PRs or equivalent docs/runbook/review impact; validation discipline; respectful review behavior. | Maintainer note or issue comment. |
| Trusted Contributor -> Reviewer | Repeated high-signal reviews; catches real bugs; understands license/provenance and validation gates; scoped lane expertise. | Owner/maintainer invitation. |
| Reviewer -> Module Collaborator | Sustained work in one module/lane; can triage issues; can recommend merge/hold with clear reasons. | Owner decision, documented in issue or governance note. |
| Module Collaborator -> Maintainer | Long-running trust, repeated sound judgment, low-risk merge recommendations, strong safety instincts. | Explicit owner decision and branch-protection update if needed. |

The numbers are minimum signals, not automatic entitlement. One high-quality
architecture/runbook/benchmark contribution can count more than several trivial
patches, but the decision must be explicit.

## Reviewer Scope

Reviewers are approved by lane. Example lanes:

- `core`: routing, hierarchy, reflection, scheduler, writer gates
- `memory`: KV/Gist, experience, semantic index, hygiene
- `runtime`: `ModelRuntime`, manifest, command runtime, conformance gates
- `docs`: README, architecture docs, tutorials, runbooks
- `benchmark`: trace schema, benchmark fixtures, CI gates
- `governance`: license, privacy, clean-room, branch policy

A reviewer can recommend approval in their lane. Cross-lane or protected-surface
changes still need the relevant maintainer/CODEOWNER review.

## Review Labels

Use these labels or equivalent issue/PR comments:

- `review-needed`: ready for review.
- `needs-validation`: missing command output, benchmark, trace, or reproduction.
- `needs-scope-cut`: PR is too broad or mixes unrelated changes.
- `needs-provenance`: external reference, license, or clean-room note missing.
- `safe-docs`: docs-only and low risk.
- `high-risk-state`: touches durable memory, genome, self-evolution, writer gate,
  privacy, trace, or deployment behavior.
- `reviewer-recommended`: reviewer thinks the PR is ready for maintainer review.
- `maintainer-required`: cannot merge without owner/CODEOWNER decision.

## Merge Rules

For protected branches:

- CODEOWNER approval is required.
- Owner or maintainer approval is required.
- Required checks must pass.
- Latest reviewable push must be approved.
- Conversations must be resolved.
- License/provenance notes must be acceptable.
- PRs into `main` use squash merge only; merge commits and rebase merges are
  disabled.
- Contributors can author PRs, but an owner or maintainer performs the merge
  after all required gates pass.
- Repository auto-merge is disabled, so protected-branch merges require an
  explicit maintainer action.

Reviewer approval is a strong signal, not a merge substitute.

For unprotected branches or draft work:

- Contributors can collaborate freely.
- High-risk state changes must remain preview-only unless the write gate,
  rollback, privacy, and validation requirements are satisfied.
- Do not treat draft branch activity as permission to merge into protected
  branches.

## High-Risk Surfaces

These areas require stricter review and should default to owner/CODEOWNER
approval:

- durable memory writes and `.ndkv` state behavior
- reasoning genome, Gene Scissors, mutation, relabel, or writer-gate behavior
- self-evolution queue, ledger, rollback, or automatic apply paths
- trace, benchmark, privacy, redaction, or raw prompt/answer handling
- runtime manifest, production kernel ABI, device contract, and KV import/export
- license, third-party references, clean-room policy, branch protection, CI
- scripts that launch remote models, daemons, unattended loops, or external
  services

## Reviewer Responsibilities

A reviewer should check:

- Does the change match the linked issue?
- Is the scope narrow and understandable?
- Are tests, checks, trace, benchmark, or docs validation enough for the risk?
- Are external references documented and license-compatible?
- Are private data, model weights, logs, `.ndkv`, credentials, and raw prompts
  excluded?
- Does the PR preserve the prototype boundary and avoid production-grade claims?
- Is rollback or preview-only behavior clear for state-changing work?

Good review comments are specific and actionable. Avoid vague gatekeeping.

## Conflict and Recusal Rules

Reviewers should not be the only reviewer for:

- their own PR;
- a change where they copied or ported the original code;
- a change involving private data, commercial work, or external obligations they
  cannot disclose;
- a disputed decision where the owner asks for another reviewer.

When in doubt, ask for a second reviewer or maintainer decision.

## Loss of Trust

Reviewer or collaborator status can be paused or removed when someone:

- repeatedly pushes broad unreviewable PRs;
- bypasses validation, branch protection, or owner review;
- copies incompatible external code;
- commits credentials, private data, model weights, `.ndkv`, logs, or generated
  build output;
- approves high-risk changes outside their lane;
- behaves in a way that makes contributors or maintainers avoid the project.

Removal should be documented briefly and calmly in a maintainer note or
governance issue. The goal is repository health, not drama.

## Public Recognition

Role promotions should be visible when the contributor agrees:

- mention in release notes;
- add a Hall of Fame entry in [Contributor Zone](../contributor-zone.md);
- update module docs when someone becomes a module collaborator;
- thank reviewers in PR summaries when their review materially improved the
  change.

Recognition does not create ownership over other contributors' work. It records
impact and helps new contributors find trusted people.

## Initial Policy

Until the repository has repeat contributors, the initial policy is:

- `@yanghao1143` remains default CODEOWNER and final protected-branch gate.
- Reviewers can be recognized as trusted reviewers before they receive any
  merge authority.
- Module collaborator status starts as triage/review authority, not direct merge
  authority.
- Maintainer expansion requires an explicit owner decision and, if needed,
  CODEOWNERS / branch-protection updates.
