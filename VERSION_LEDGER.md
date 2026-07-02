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
| 0.30.26-issue-30-domain-write-field-gate | issue-30 negative-gate evidence that hides memory, genome, and self-evolution write_allowed=false fields behind one durable aggregate | #30, #305 |
| 0.30.27-issue-30-review-blocker-field-gate | issue-30 evidence packets that prove demo fields without explicitly marking release review, #31 signoff, and #19 runtime-surface blockers | #30, #19, #305 |
| 0.30.28-issue-30-runtime-surface-blocker-field-gate | issue-30 evidence packets that do not separate merged service/API runtime proof from unmerged runtime closed-loop counter proof | #30, #19, #305 |
| 0.30.29-issue-30-git-dirty-source-gate | issue-30 evidence packets that rely on hand-written dirty_worktree statements instead of git status evidence | #30, #305 |
| 0.30.30-issue-30-compute-benefit-complete-field-gate | issue-30 evidence packets that prove second-task compute benefit with avoided tokens but omit saved tokens or skipped KV lookups | #30, #305 |
| 0.30.31-issue-30-correctness-anchor-field-gate | issue-30 evidence packets that prove second-task compute savings without explicit quality drift failure and correctness-anchor preservation fields | #30, #305 |
| 0.30.32-version-lockstep-hard-gate | version ledgers that leave first-party Cargo manifests or Cargo.lock entries drifting from the latest issue-slice version | #30, #305 |
| 0.30.33-issue-30-tenant-boundary-evidence-gate | issue-30 tenant-boundary evidence that only exposes boolean denial without scope mode, actor/target digests, lane, or denial reason | #30, #37, #305 |
| 0.30.34-issue-30-problem-hypothesis-evidence-gate | issue-30 closed-loop evidence packets that count self-evolution admission without digest-bound #377 ProblemFinding and HypothesisCandidate proof | #30, #377, #305 |
| 0.30.35-issue-30-entry-chain-evidence-gate | issue-30 evidence packets that prove the roundtrip tail without digest-bound EnvironmentPressure, SelfOntology body, ReasoningFrame, backend action, and #379 control preview markers | #30, #375, #379, #385, #305 |
| 0.30.36-issue-30-demo-dispatch-evidence-gate | issue-30 evidence packets that list a demo command without explicit integration-test, dispatch path, and trace-schema execution proof | #30, #305 |
| 0.30.37-issue-30-approved-reuse-digest-gate | issue-30 evidence packets that show second-run compute savings or bad-candidate hold without digest-bound approved experience reuse and bad-candidate decision proof | #30, #305 |
| 0.30.38-issue-30-hidden-cot-redaction-gate | issue-30 evidence packets that redact prompts and answers but can still expose hidden chain-of-thought or COT payload fields | #30, #305 |
| 0.30.39-issue-30-git-rc-evidence-gate | issue-30 evidence packets that rely on hand-written RC SHA or branch values instead of local git-derived evidence | #30, #305 |
| 0.30.40-issue-30-release-review-evidence-gate | issue-30 evidence packets that rely on raw hand-written PR list, release review readiness, or review blocker fields | #30, #305 |
| 0.30.41-issue-30-issue-state-evidence-gate | issue-30 evidence packets that rely on raw hand-written issue signoff, runtime surface, or close-allowed fields | #30, #19, #31, #305 |
| 0.30.42-issue-30-demo-proof-evidence-gate | issue-30 evidence packets that rely on raw hand-written clean-checkout demo test, dispatch path, or trace gate execution fields | #30, #305 |
| 0.30.43-issue-30-roundtrip-proof-evidence-gate | issue-30 evidence packets that rely on raw hand-written persistent roundtrip, compute-budget, or negative write-gate fields | #30, #305 |
| 0.30.44-issue-30-context-proof-evidence-gate | issue-30 evidence packets that rely on raw hand-written environment pressure, self-ontology body, pre-reasoning frame, or problem-hypothesis context fields | #30, #305 |
