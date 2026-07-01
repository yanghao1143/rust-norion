# Version Ledger

| Version | Deprecations | Refs |
| --- | --- | --- |
| 0.30.1-issue-30-roundtrip-compute-budget-evidence | retired 0.1.0 issue-30 commit trailers and Cargo package versions; persistent roundtrip reports hiding second-run compute budget savings and all-device avoided-token summaries | #30, #19, #305 |
| 0.30.2-issue-30-negative-gate-roundtrip-packet | roundtrip evidence packets without unauthorized-write pollution rollback tenant-boundary and redaction negative gates | #30, #305 |
| 0.30.3-issue-30-clean-checkout-evidence-packet | issue-30 demo evidence that drops compute token counters or lacks a digest-only clean-checkout packet | #30, #305 |
| 0.30.4-active-version-metadata-gate | active project metadata that still advertises retired 0.1.0 or drifts from Cargo package versions | #30, #305 |
| 0.30.5-issue-30-roundtrip-trace-admission-gate | issue-30 roundtrip clean-checkout demos without trace schema, reasoning-genome, or self-evolution admission evidence | #30, #305 |
| 0.30.6-pr-ledger-edited-trigger | pull-request body version fixes that leave stale failed ledger checks because edited events do not rerun validation | #30, #305 |
| 0.30.7-issue-30-evidence-command-trace-gate | issue-30 evidence packets whose replay command omits trace schema gate or relies only on library-level packet rendering | #30, #305 |
| 0.30.8-issue-30-genome-write-gate-marker | issue-30 evidence packets that show reasoning genome events without explicit genome write gate denial | #30, #305 |
| 0.30.9-version-ledger-gate | version and deprecation records that only live in commit or pull-request text and force manual searching | #30, #305 |
| 0.30.10-issue-30-negative-gate-packet-proof | issue-30 evidence packet checks that only prove compute/genome gates while omitting visible negative-gate proof | #30, #305 |
| 0.30.11-issue-30-rollback-anchor-digest | issue-30 rollback or hold evidence that only exposes a boolean marker without a digest-bound evidence id | #30, #305 |
| 0.30.12-version-ledger-hard-gate | version ledgers that are missing ordered unique versions, non-empty deprecation records, issue refs, or current Cargo/CITATION alignment | #30, #305 |
| 0.30.13-issue-30-evidence-packet-privacy-redaction | issue-30 evidence packets that redact credentials but still expose local paths, AppData temp paths, raw prompt lines, or raw answer lines | #30, #305 |
| 0.30.14-issue-30-evidence-payload-line-redaction | issue-30 evidence packets that can still expose state-inspection key or lesson payload lines after path and credential redaction | #30, #305 |
| 0.30.15-ci-focused-rust-norion-test-bound | PR validation that can leave version-management checks stuck behind an unbounded full rust-norion test run | #30, #305 |
| 0.30.16-issue-30-evidence-field-gate | issue-30 evidence packets that render even when required proof fields are missing or rejected payload markers remain | #30, #305 |
| 0.30.17-ci-norion-cli-evidence-gate | pull-request validation that compiles norion-cli but does not execute the issue-30 evidence-packet field gate | #30, #305 |
| 0.30.18-ci-norion-cli-format-gate | pull-request validation that claims norion-cli formatting passed in text but does not execute norion-cli fmt as a machine gate | #30, #305 |
| 0.30.19-issue-30-state-file-field-gate | issue-30 clean-checkout evidence packets that prove memory file creation but do not require experience or adaptive state file creation | #30, #305 |
| 0.30.20-issue-30-compute-benefit-field-gate | issue-30 clean-checkout evidence packets that show a second-run compute benefit in text but do not require the compute-benefit field | #30, #305 |
| 0.30.21-issue-30-negative-gate-field-gate | issue-30 clean-checkout evidence packets that show negative-gate fields in text but do not require polluted-evidence rollback or provenance fields | #30, #305 |
| 0.30.22-issue-30-positive-loop-field-gate | issue-30 clean-checkout evidence packets that show replay, prompt digest, reasoning-genome, or self-evolution review fields without requiring them | #30, #305 |
| 0.30.23-issue-30-redaction-reject-gate | issue-30 clean-checkout evidence packets that assert digest-only output but do not reject local path, raw answer, payload, or secret markers | #30, #305 |
| 0.30.24-issue-30-rc-readiness-field-gate | issue-30 clean-checkout evidence packets that do not require RC SHA, branch/PR list, or dirty-worktree statement | #30, #305 |
| 0.30.25-issue-30-durable-write-field-gate | issue-30 negative-gate evidence that denies unauthorized writes without an explicit durable write_allowed=false field | #30, #305 |
