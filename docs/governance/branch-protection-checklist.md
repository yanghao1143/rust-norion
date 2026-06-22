# Branch Protection Checklist

Status: repository governance checklist for `codex-runtime-device-abi` and any
future protected default branch.

Refs: #11, #27, #31, #41, #77.

## Current Protected Branch

As of 2026-06-22, the default branch is `codex-runtime-device-abi`.

GitHub branch protection for this branch should require:

- pull request before merge
- at least one approving review
- CODEOWNER review
- approval of the latest reviewable push
- required status check `focused Rust crates`
- strict status checks against the latest branch state
- conversation resolution before merge
- admin enforcement
- force pushes disabled
- branch deletion disabled

The current repository setting reported by GitHub API matches this policy:

```text
required_status_checks.strict = true
required_status_checks.contexts = ["focused Rust crates"]
required_pull_request_reviews.required_approving_review_count = 1
required_pull_request_reviews.require_code_owner_reviews = true
required_pull_request_reviews.require_last_push_approval = true
required_conversation_resolution.enabled = true
enforce_admins.enabled = true
allow_force_pushes.enabled = false
allow_deletions.enabled = false
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
- non-commercial research/deployment compatibility
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
maintainer. A PR is not merge-ready until:

- the linked issue is clear
- the requested state is preview-only or has a satisfied graduation checklist
- CI checks pass
- CODEOWNER review passes
- conversations are resolved
- license/provenance notes are acceptable
- non-commercial constraints are preserved

## Audit Procedure

Run these checks before inviting broader collaboration or after changing GitHub
settings:

```powershell
gh repo view yanghao1143/rust-norion --json nameWithOwner,visibility,defaultBranchRef
gh api repos/yanghao1143/rust-norion/branches/codex-runtime-device-abi/protection
git show HEAD:.github/CODEOWNERS
git show HEAD:.github/pull_request_template.md
git ls-files .github/ISSUE_TEMPLATE
```

The audit result should be posted to the relevant governance issue when settings
change. If branch protection cannot be verified, self-evolution and
preview-to-write graduation work must remain blocked from merge.

