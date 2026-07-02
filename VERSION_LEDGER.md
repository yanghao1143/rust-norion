# Version Ledger

Current package version: `0.305.3`

| Status | Version | Scope | Deprecations | Refs |
| --- | --- | --- | --- | --- |
| active | `0.305.3-issue-305-retired-version-commit-block` | issue #305 retired commit version gate | new commits that reuse retired `0.1.0` scaffold versions | #305, #19 |
| retired | `0.305.2-issue-305-version-ledger-row-gate` | issue #305 version ledger row gate | loose ledger rows without checked scope, deprecations, refs, and single-active-version invariants | #305, #19 |
| retired | `0.305.1-issue-305-version-ledger-gate` | issue #305 version gate | active Cargo/CITATION/lock `0.1.0` scaffold versions; commit/PR/issue ledgers without a checked version ledger | #305, #19 |
| retired | `0.1.0-issue-305-trace-benchmark-surface-gate-ci-fix` | issue #305 trace digest fix | generated digest-only trace metadata misclassified as raw trace surface | #305, #19 |
| retired | `0.1.0-issue-305-trace-benchmark-surface-gate` | issue #305 trace benchmark gate | polluted development evidence entering trace or benchmark hot surfaces without a reusable #305 gate | #305, #19 |

Retired active package versions:
- `0.1.0`: scaffold Cargo/CITATION/lock version; no current first-party manifest, lock entry, PR body latest version, issue comment latest version, or new commit version may use it.
