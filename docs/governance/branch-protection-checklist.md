# Branch Protection Checklist

Status: repository governance checklist for `main` and any future protected
default branch.

Refs: #11, #27, #31, #41, #77.

## Current Protected Branch

As of 2026-06-26, the default branch is `main`.

GitHub branch protection for this branch should require:

- pull request before merge
- at least one approving review
- CODEOWNER review
- approval of the latest reviewable push
- required status check `focused Rust crates`
- required status check reports on every pull request, not only selected paths
- strict status checks against the latest branch state
- conversation resolution before merge
- required linear history
- admin enforcement
- force pushes disabled
- branch deletion disabled
- active repository ruleset for the default branch with no bypass actors
- repository-level squash merge only
- repository-level auto-merge disabled
- automatic head-branch deletion after merge
- PR branch update enabled when the head branch is behind `main`
- GitHub Actions default workflow token permission set to read-only
- GitHub Actions pull request review approval disabled
- first-time external contributors require workflow approval before CI runs
- Gitee mirror synchronization limited to `main`

The current repository setting reported by GitHub API matches this policy:

```text
required_status_checks.strict = true
required_status_checks.contexts = ["focused Rust crates"]
PR Validation pull_request trigger = all paths
required_pull_request_reviews.required_approving_review_count = 1
required_pull_request_reviews.require_code_owner_reviews = true
required_pull_request_reviews.require_last_push_approval = true
required_conversation_resolution.enabled = true
required_linear_history.enabled = true
enforce_admins.enabled = true
allow_force_pushes.enabled = false
allow_deletions.enabled = false
allow_squash_merge = true
allow_merge_commit = false
allow_rebase_merge = false
delete_branch_on_merge = true
allow_update_branch = true
allow_auto_merge = false
actions.default_workflow_permissions = read
actions.can_approve_pull_request_reviews = false
actions.fork_pr_contributor_approval = first_time_contributors
MIRRORED_BRANCHES = main
ruleset.name = "main contributor merge gate"
ruleset.enforcement = active
ruleset.conditions.ref_name.include = ["~DEFAULT_BRANCH"]
ruleset.bypass_actors = []
ruleset.pull_request.allowed_merge_methods = ["squash"]
ruleset.pull_request.required_approving_review_count = 1
ruleset.pull_request.require_code_owner_review = true
ruleset.pull_request.require_last_push_approval = true
ruleset.required_status_checks = ["focused Rust crates"]
```

## CODEOWNERS

`.github/CODEOWNERS` assigns `@yanghao1143` as the default owner and explicitly
marks protected rust-norion surfaces:

- self-evolution gates and ledgers
- trace/schema/model-service evidence
- disk-backed KV and memory admission
- Reasoning Genome and Gene Scissors surfaces
- governance and GitHub workflow files

CODEOWNER approval is a merge gate. It does not authorize writes, deployment, or
preview-to-write graduation by itself.

## PR Template Requirements

The pull request template must require:

- linked issue
- validation evidence
- formatting/check confirmation
- trace or benchmark evidence when relevant
- no generated target files, logs, credentials, model weights, `.ndkv`, or
  private data
- no raw prompt/answer payloads
- preview-only default for durable memory/genome/self-evolution writes
- preview-to-write graduation checklist for any requested write or active
  behavior change
- GPL-3.0 license compatibility
- clean-room external-source confirmation

## Issue Templates

The repository should keep issue templates for:

- implementation slices
- research spikes
- safety reviews
- benchmark evidence

Each template must preserve the same boundaries: no raw private payloads, no
unapproved durable writes, clean-room external references, and maintainer review
before merge.

## Maintainer Review Rule

External contributors may propose changes, but merge control stays with the
maintainer. Protected-branch merges use squash merge; merge commits and rebase
merges are disabled. A PR is not merge-ready until:

- the linked issue is clear
- the requested state is preview-only or has a satisfied graduation checklist
- CI checks pass
- CODEOWNER review passes
- conversations are resolved
- license/provenance notes are acceptable
- GPL-3.0 and third-party license constraints are preserved

## Permission Audit

As of 2026-06-26, the intended repository permission baseline is:

- public contributors use fork or topic branch PRs into `main`
- no routine contributor needs direct repository write access
- direct-collaborator audit reports only `@yanghao1143` with admin permission
- workflow tokens default to read-only and cannot approve pull request reviews
- first-time external contributors require workflow approval before CI runs
- reviewer and module-collaborator status does not imply merge authority
- maintainer expansion requires an explicit owner decision
- Gitee is a mirror-only surface for `main`, not a place to publish topic
  branches
- protected-branch merge remains gated by CODEOWNER review, latest-push
  approval, required checks, conversation resolution, and explicit maintainer
  action

Audit direct collaborators before broadening community access:

```powershell
gh api repos/yanghao1143/rust-norion/collaborators?affiliation=direct --paginate
```

## Audit Procedure

Run these checks before inviting broader collaboration or after changing GitHub
settings:

```powershell
gh repo view yanghao1143/rust-norion --json nameWithOwner,visibility,defaultBranchRef
gh api repos/yanghao1143/rust-norion/branches/main/protection
gh api repos/yanghao1143/rust-norion/rulesets
gh api repos/yanghao1143/rust-norion/rules/branches/main
gh api repos/yanghao1143/rust-norion --jq '{default_branch,allow_squash_merge,allow_merge_commit,allow_rebase_merge,delete_branch_on_merge,allow_update_branch,allow_auto_merge}'
gh api repos/yanghao1143/rust-norion/actions/permissions/workflow
gh api repos/yanghao1143/rust-norion/actions/permissions/fork-pr-contributor-approval
git show HEAD:.github/CODEOWNERS
git show HEAD:.github/pull_request_template.md
git ls-files .github/ISSUE_TEMPLATE
```

The audit result should be posted to the relevant governance issue when settings
change. If branch protection cannot be verified, self-evolution and
preview-to-write graduation work must remain blocked from merge.
