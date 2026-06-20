# Evolution Loop and norion-eval Runbook

This runbook explains how to reuse the current `tools/evolution-loop` evidence
without changing the loop.

## Existing Evidence To Preserve

- Keep using `tools/evolution-loop` as the active runner.
- Keep the existing validation command shape:

```powershell
cargo test --manifest-path .\tools\evolution-loop\Cargo.toml
```

- Keep clean ledger artifacts in `target\evolution`. The current compatibility
  sample is:

```text
target\evolution\runtime-model-gated-loop-20260613-175925.jsonl
```

That ledger records successful runtime-model-gated rounds with
`validation_checked=true`, `validation_passed=true`, non-zero `runtime_tokens`,
and `runtime_model=google/gemma-4-12B-it`. Its validation meta shows the current
`tools/evolution-loop` suite passing 67 tests.

## How To Use The New Crates Later

1. The runner executes the existing loop exactly as it does today.
2. The runner reads JSONL ledger rows and maps them into
   `norion_eval::LedgerRecord`.
3. The runner summarizes with `LedgerSummary::from_records`.
4. The runner evaluates `RuntimeResponseGate`, `ValidationGate`, and
   `ReportGate`.
5. Before enforced adapter wiring, the runner emits `schema_drift_report_v1`
   from the eval manifest, report bundle manifest, and handoff checklist.
6. If any gate blocks, the runner surfaces `RollbackPlan` actions and stops
   scheduling new unattended rounds.

## Gate Defaults

Recommended strict defaults for unattended evolution:

- `runtime_tokens` must be present and greater than 0.
- `runtime_model` must be present.
- validation must be checked and passed.
- feedback must meet the configured minimum total.
- self-improve must pass when it was checked.
- state and trace gates must not report failures.
- stream truncations must be 0.
- missing final-event failures must be 0.
- runtime response failures must be 0.
- strict ledger hygiene must have no duplicate rounds, non-monotonic rounds,
  invalid rounds, or round gaps.
- latest round must be successful.
- Context Rot/noise penalty must be at or below the selected threshold.

Use `norion_eval::AdapterLedgerGateBoundaryContract::ledger_gate_v1` and
`norion_test::AdapterLedgerGateBoundaryPlan` before wiring ledger hygiene into
an adapter. The runner may read JSONL rows and normalize them into
`LedgerRecord` values, but eval owns only `LedgerSummary::from_records`,
`ReportGate::evaluate`, and `LedgerGateReport` projection. Keep JSONL/file IO,
HTTP/SSE, validation command execution, model calls, and runner state outside
this boundary. `stays_pure_data_boundary()` is the fixture helper for this
contract.

## Run-Mode Report Refresh Boundary

`tools/evolution-loop` remains responsible for running the daemon loop,
refreshing the report, and printing status such as:

```text
report rounds=278 ledger_lag=0 stale=false gate_failures=0
```

`norion-eval` owns only the pure projection after those values have been
normalized. Use `ReportFreshnessStatus` and `ReportFreshnessReport` for
`run-report-json` or `report_refresh` freshness, and use
`RemoteRuntimeAccelerationStatus` and `RemoteRuntimeAccelerationReport` for the
remote model-pool readiness view. Neither helper may read report files, inspect
daemon state, call SSH, call a remote Mac, call a model, or execute HTTP/SSE.

Stable fields for `report_freshness_report_v1`:

| Field | Source owned by runner | Eval responsibility |
| --- | --- | --- |
| `report_freshness.rounds` | current refreshed report round count | copy into freshness report |
| `report_freshness.ledger_lag` | refreshed report ledger lag | require zero for `fresh=true` |
| `report_freshness.stale` | refreshed report stale bit | require false for `fresh=true` |
| `report_freshness.gate_failures` | refreshed report gate failure count | require zero for `fresh=true` |
| `report_freshness.ledger_gate_total_rounds` | existing `LedgerGateReport` | copy aggregate count only |
| `report_freshness.ledger_gate_blocked` | existing `LedgerGateReport` decision | copy blocked bit only |
| `report_freshness.ledger_gate_allow_next_round` | existing `LedgerGateReport` decision | copy allow bit only |
| `report_freshness.fresh` | freshness status plus gate count | compute in eval |
| `report_freshness.allow_next_round` | freshness plus ledger gate allow bit | compute in eval |

`report_freshness_report_v1` must not copy ledger gate failure reasons. Those
remain owned by `ledger_gate_report_v1`; freshness only answers whether the
runner's current report view is up to date and whether that view agrees that the
next round may proceed.

Stable fields for `remote_runtime_acceleration_report_v1`:

| Field | Source owned by runner | Eval responsibility |
| --- | --- | --- |
| `remote_runtime.total_workers` | model-pool status count | copy into status/report |
| `remote_runtime.healthy_workers` | model-pool status count | compare with total workers |
| `remote_runtime.metal_workers` | model-pool status count | compare with total workers |
| `remote_runtime.quality_model` | selected quality model name | trim and check presence |
| `remote_runtime.all_workers_healthy` | status counts | compute in eval |
| `remote_runtime.all_workers_metal` | status counts | compute in eval |
| `remote_runtime.quality_model_present` | quality model string | compute in eval |
| `remote_runtime.acceleration_ready` | aggregate of the three readiness bits | compute in eval |
| `remote_runtime.failure_reasons` | aggregate readiness failures | compute in eval |

The remote runtime report is acceleration evidence, not a remote-control API.
It may prove that a 6/6 healthy, 6/6 Metal pool with a configured quality model
is ready for report-gate decisions. It must not carry `RemoteModelClient`,
`RemoteMacSession`, daemon handles, SSH sessions, or model probe output across
the eval boundary.

Run-mode report refresh differs from legacy post-run reporting:

| Mode | Runner responsibility | Eval responsibility | Compatibility rule |
| --- | --- | --- | --- |
| Run-mode `report_refresh` | refresh report during daemon rounds and expose summary counters | project `report_freshness_report_v1` and remote acceleration reports from normalized counters | enforced bundle may require these schemas |
| Legacy post-run report | preserve existing JSONL rows and additive historical reports | replay old rows through `LegacyLedgerReplayPlan` and gate only existing fields | missing freshness or remote runtime reports are additive gaps, not replay failures |

Drift guard:

- `remote_runtime_acceleration_report_v1` and `report_freshness_report_v1` must
  appear in `EvalSchemaManifest::evolution_loop_handoff_v1`,
  `EvalReportBundleManifest::for_stage(Enforced)`,
  `AdapterHandoffChecklist::before_runner_wiring(Enforced)`, and
  `AdapterReportEmissionPlan::required_report_fields()`.
- They must appear in the matching `norion-test` plan contracts:
  `AdapterReportEmissionPlan`, `LegacyLedgerReplayPlan`,
  `EvalSchemaManifestPlan`, and `EvalReportBundlePlan`.
- They must not appear in the report-only bundle unless their stage is
  deliberately downgraded by a future contract change.
- Missing `remote_runtime.*` or `report_freshness.*` fields are adapter report
  coverage/schema drift failures, not model quality failures and not evidence
  that the current runner is broken.

## Steam Case Matrix Gate

Use `norion_test::SteamCaseMatrixPlan` before promoting the root
`/v1/business-cycle-stream` adapter from report-only to enforced. The current
`tools/evolution-loop` runner can keep running single-round Steam checks; the
future adapter should pass observed `norion_eval::SteamCaseCoverageRow` values
to `norion_eval::SteamCaseCoverageReport::from_rows_and_gate`, or project
prebuilt evidence with `from_gate_and_evidence`.
Keep HTTP/SSE execution and final JSON extraction in the runner; the eval
contract starts after those observations have been normalized into rows.
Use `norion_eval::AdapterSteamEvidenceBoundaryContract::steam_reports_v1` and
`norion_test::AdapterSteamEvidenceBoundaryPlan` to keep that line explicit:
eval may consume `StreamContinuityCheck`, `ValidationObservation`,
`LedgerRecord`, `SelfEvolutionReadinessSnapshot`, Steam case rows/evidence, and
Steam gates to produce `SteamRoundAcceptanceReport` and
`SteamCaseCoverageReport`, but it must not perform Steam HTTP/SSE execution,
stream process execution, JSONL IO, validation-command spawning, model calls, or
runner-state reads.
Use `stays_pure_data_boundary()` on both sides as the fixture helper before any
future adapter wiring treats Steam case matrix evidence as eval-owned.

Minimum enforced coverage:

- at least four unique case IDs;
- required case kinds `planning`, `validation`, `rollback`, and
  `apple_silicon_model_pool`;
- every case uses `/v1/business-cycle-stream`;
- stream continuity passed for every case;
- validation checked and passed for every case;
- `business_cycle.passed=true` for every case;
- final JSON contains `business_cycle.passed`,
  `business_cycle.feedback_applied`, `generate.runtime_model`,
  `generate.runtime_tokens`, `validation.checked`, `validation.passed`,
  `self_improve.checked`, and `self_improve.passed`.

Report `steam_case_matrix.case_kinds`,
`steam_case_matrix.required_case_kinds`, and
`steam_case_matrix.missing_required_case_kinds` during report-only rollout so a
passing matrix cannot omit rollback or Apple Silicon model-pool coverage.

Recommended rollout:

1. `ShadowOnly`: keep matrix rows in fixtures or tests; do not print new runner
   output.
2. `ReportOnly`: print `steam_case_matrix.*` additive fields and missing-field
   gaps, but do not block the current runner.
3. `Enforced`: allow `steam_case_matrix.coverage_blocked=true` to stop adapter
   promotion until coverage is complete.

This gate is coverage evidence, not outage attribution. If 8686 is down while
the prompt gate blocked the chain, classify the root adapter state as
`chain_not_ready`. If prompt-gate passed and 8686 is unreachable, classify it
as `model_unavailable`. Neither case may be counted as
`model_quality_failure`.

## Validation Command Coverage Gate

Use `norion_test::ValidationCommandCoveragePlan` and
`norion_eval::ValidationCommandCoverageReport` before trusting a future adapter
to continue unattended rounds from a boolean validation result alone.
After command execution, pass normalized `ValidationObservation` rows and the
rust-check booleans to
`ValidationCommandCoverageReport::from_observations_and_gate`, or use
`from_gate_and_evidence` if evidence is already assembled.
Use
`norion_eval::AdapterValidationCommandCoverageBoundaryContract::validation_command_coverage_v1`
and `norion_test::AdapterValidationCommandCoverageBoundaryPlan` to keep command
execution out of eval: the runner may spawn cargo/rust-check commands and
capture `CommandOutcome`, but eval only consumes `ValidationObservation`,
`ValidationCommandCoverageEvidence`, `ValidationCommandCoverageGate`, and
rust-check booleans. `stays_pure_data_boundary()` must reject process handles,
validation command executors, runner state, JSONL/file IO, HTTP/SSE, and model
calls.

The adapter should preserve, per validation command:

- command line;
- phase, preferably `PostRound` or `Both`;
- exit status code;
- elapsed milliseconds;
- stdout tail or stderr tail;
- checked/passed booleans.

Enforced rollout should also require rust-check evidence to be checked and
passed. Missing command metadata should be reported as
`validation_command.coverage_blocked=true`, not folded into model quality or
runtime attribution.

Recommended rollout:

1. `ShadowOnly`: build fixture observations from existing validation output.
2. `ReportOnly`: print `validation_command.*` fields and missing evidence.
3. `Enforced`: allow `validation_command.allow_next_round=false` to stop the
   next unattended round until phase, status, output tail, pass result, and
   rust-check evidence are complete.

## Experiment Switch Matrix Report

Use `norion_eval::ExperimentSwitchMatrixReport::from_flags_and_reports` when
the adapter already has `ExperimentExpansionSafetyReport` values. The helper
keeps the switch matrix on the report side of the boundary: it reads
`ExperimentExpansionSafetyReport::enabled_flag_names`,
`ExperimentExpansionSafetyReport::flag_names`,
`ExperimentExpansionSafetyReport::expansion_blocked`, and
`ExperimentExpansionSafetyReport::allow_experiment_expansion` plus the planned
`ExperimentFlag` list, then emits `experiment_switch.*` fields.

Do not rebuild switch state from runner internals after expansion reports
exist. The runner may decide which experiment flags are configured and may
execute the underlying Steam, validation, ledger, and Context Rot checks, but
the matrix report only compares pure data:

- enabled flags that lack an expansion report;
- duplicate reports for the same enabled flag;
- reports that reference disabled or unknown flags;
- expansion reports that already blocked;
- the final `experiment_switch.allow_experiment_switch_expansion` bit.

The matrix helper must not read JSONL or files, call HTTP/SSE, spawn validation
commands, inspect runner state, or call a model. If an expansion report blocks
because validation command coverage, Steam case matrix, rollback resume,
Context Rot trend/remediation, or report-field coverage failed, the switch
matrix should carry that blocked report forward instead of re-running those
checks.

## Adapter Fixture Contract

Use `norion_test::AdapterFixtureContractPlan` before wiring the root adapter
into the active evolution runner. The fixture contract is stronger than legacy
ledger replay: legacy JSONL may lack additive reports, but adapter fixtures must
prove the new projection and attribution paths before they can block.

Minimum fixture set before enforcement:

- clean success -> `none`;
- prompt-gate blocked, including 8686 unavailable -> `chain_not_ready`;
- prompt-gate passed and 8686 unavailable -> `model_unavailable`;
- missing terminal final JSON -> `stream_or_final_missing`;
- missing runtime model or zero/missing runtime tokens after generation should
  have run -> `runtime_response_missing`;
- final JSON with runtime evidence and `business_cycle.passed=false` ->
  `model_quality_failure`.

Each fixture should mark these projection checks complete:

- root fixture evidence present;
- `LedgerRecord` projection checked;
- synthetic `ModelWorkerRecord` projection checked;
- report bundle projection checked.

Recommended rollout:

1. `ShadowOnly`: keep fixture cases in crate tests; do not print new runner
   output.
2. `ReportOnly`: emit `adapter_fixture.*` fields so missing projection checks
   and classification mismatches are visible.
3. `Enforced`: require `adapter_fixture.allow_runner_wiring=true` before root
   adapter wiring can block the current runner.

This fixture contract must preserve the attribution rule: prompt-gate blocked
and model unavailable cases are operational readiness failures, not model
quality failures.

## Current Runner Compatibility Report

Use `norion_test::CurrentRunnerCompatibilityPlan` as the last report-only view
before enforced adapter wiring. It aggregates the gates that otherwise live in
separate reports:

- legacy ledger replay;
- enforced report bundle completeness;
- schema drift report;
- adapter report emission order and future-event coverage;
- Apple Silicon model-pool development window;
- Apple Silicon development-effect aggregate gate;
- feedback/self-improve gate;
- self-evolution continuity and regression gates;
- next-round readiness gate;
- self-evolution unattended prerequisites;
- Context Rot trend and remediation gates;
- rollback resume gate;
- adapter fixture contract;
- Steam case matrix;
- validation command coverage;
- adapter promotion window;
- adapter handoff;
- `tools/evolution-loop` tests;
- workspace tests.

Recommended rollout:

1. `ShadowOnly`: keep the aggregate in crate tests.
2. `ReportOnly`: emit `current_runner.*` fields to show which prerequisite is
   still missing.
3. `Enforced`: require `current_runner.allow_enforced_wiring=true` before any
   new adapter gate can block the existing runner. This includes a clean schema
   drift report, passing `current_runner.adapter_report_emission_passed` and
   `current_runner.adapter_report_field_coverage_passed`, passing
   `current_runner.adapter_future_event_coverage_passed` evidence, a passing
   Apple Silicon model-pool development window/effect, passing
   `current_runner.feedback_self_improve_passed`,
   `current_runner.self_evolution_continuity_passed`,
   `current_runner.self_evolution_regression_passed`, and
   `current_runner.readiness_next_round_passed` evidence, and passing
   self-evolution unattended prerequisites plus Context Rot trend/remediation
   and rollback-resume evidence, not only a complete report bundle.

This report should stay conservative. A green aggregate means the current
runner remains protected while the adapter becomes enforceable; a red aggregate
means `tools/evolution-loop` stays authoritative.

Use `norion_eval::AdapterCurrentRunnerCompatibilityBoundaryContract::current_runner_compatibility_v1`
and `norion_test::AdapterCurrentRunnerCompatibilityBoundaryPlan` before wiring
this aggregate into an adapter. Eval may build
`CurrentRunnerCompatibilityEvidence`, evaluate
`CurrentRunnerCompatibilityGate`, copy
`AdapterReportEmissionReport::field_coverage_passed`, and emit
`CurrentRunnerCompatibilityReport` from upstream gate pass bits and test-result
pass bits. When the handoff report is already present, use
`CurrentRunnerCompatibilityEvidence::with_handoff_report_pass_bits` to lift
ledger hygiene, report bundle/schema drift, rollback, Steam case matrix,
validation coverage, promotion, handoff, and test-result bits without
re-reading runner artifacts. It must not read JSONL or files, call HTTP/SSE,
spawn cargo or workspace tests, call a model, switch runner wiring, mutate
runner state, or call a remote Mac.

When the adapter does not yet have a full handoff report, use the thinner
single-report helpers on `CurrentRunnerCompatibilityEvidence`:

- `with_adapter_report_emission_report`
- `with_adapter_report_field_coverage_from_report`
- `with_adapter_future_event_coverage_report`
- `with_report_bundle_gate_report`
- `with_schema_drift_report`
- `with_adapter_fixture_report`

These helpers only copy stable allow/pass bits from already-computed reports.
They do not execute tests, scan files, or recompute the upstream gates.
`AdviceContinuationReport` is also intentionally excluded from this direct
current-runner boundary. If continuation guidance matters here, it must arrive
through the already-projected readiness or handoff pass bits rather than as a
new direct input type.

R19 adapter-facing evidence index:

- `CurrentRunnerCompatibilityReport::from_gate_and_evidence` derives
  `current_runner.compatibility_blocked`, `current_runner.allow_enforced_wiring`,
  and `current_runner.failure_reasons` from one `CurrentRunnerCompatibilityGate`
  decision.
- `SelfEvolutionUnattendedPrerequisiteReport::from_gate_and_evidence` derives
  `self_evolution_unattended.prerequisite_blocked`,
  `self_evolution_unattended.allow_unattended_self_evolution_claim`, and
  `self_evolution_unattended.failure_reasons` from one
  `SelfEvolutionUnattendedPrerequisiteGate` decision.
- `AdapterClosureReport::from_reports` copies prior report surfaces and sets
  `adapter_closure.allow_next_round` from
  `readiness.can_schedule_next_round`; it does not manually rebuild upstream
  failure reasons.
- The fixture
  `adapter_facing_bundle_keeps_next_round_and_failure_reasons_aligned` is the
  clean-room evidence that ledger, Context Rot, validation, readiness,
  unattended-prerequisite, closure, and current-runner adapter surfaces keep
  allow bits and failure reasons aligned without introducing IO, HTTP/SSE,
  process spawning, model calls, or runner-state reads.

Stable report fields for `current_runner_compatibility_report_v1`:

- `current_runner.stage`
- `current_runner.legacy_replay_passed`
- `current_runner.report_bundle_complete`
- `current_runner.schema_drift_passed`
- `current_runner.adapter_report_emission_passed`
- `current_runner.adapter_report_field_coverage_passed`
- `current_runner.adapter_future_event_coverage_passed`
- `current_runner.model_pool_development_window_passed`
- `current_runner.apple_silicon_development_effect_passed`
- `current_runner.feedback_self_improve_passed`
- `current_runner.self_evolution_continuity_passed`
- `current_runner.self_evolution_regression_passed`
- `current_runner.readiness_next_round_passed`
- `current_runner.self_evolution_unattended_prerequisites_passed`
- `current_runner.context_rot_trend_passed`
- `current_runner.context_rot_remediation_passed`
- `current_runner.rollback_resume_passed`
- `current_runner.adapter_fixture_passed`
- `current_runner.steam_case_matrix_passed`
- `current_runner.validation_command_coverage_passed`
- `current_runner.promotion_window_passed`
- `current_runner.handoff_passed`
- `current_runner.evolution_loop_tests_passed`
- `current_runner.workspace_tests_passed`
- `current_runner.compatibility_blocked`
- `current_runner.failure_reasons`
- `current_runner.allow_enforced_wiring`

## Schema Drift Report

Use `norion_test::EvalSchemaDriftReportPlan` and
`norion_eval::EvalSchemaDriftReport` as the last contract-only check before
root adapter wiring starts to depend on the eval bundle.

Compare these sources:

- `EvalSchemaManifest::evolution_loop_handoff_v1()`;
- `EvalReportBundleManifest::for_stage(RootAdapterRolloutStage::Enforced)`;
- `AdapterHandoffChecklist::before_runner_wiring(RootAdapterRolloutStage::Enforced)`;
- `AdapterReportEmissionPlan::required_report_fields()`.

Use `norion_eval::AdapterSchemaDriftBoundaryContract::schema_drift_v1` and
`norion_test::AdapterSchemaDriftBoundaryPlan` before adapter wiring. Eval may
derive `EvalSchemaDriftEvidence` from the in-crate manifest, bundle manifest,
handoff checklist, adapter report-field contract, and
`AdapterClosureSchemaDocument`; it may fingerprint schema/report-field names and
emit `EvalSchemaDriftReport`. It must not read schema files, scan report
directories, read JSONL/files, call HTTP/SSE, spawn processes or validation
commands, call a model, or inspect runner state. Use
`stays_pure_data_boundary()` on both sides so schema drift remains a pure
contract comparison.

When an `AdapterReportEmissionReport` already reports missing fields, use
`EvalSchemaDriftEvidence::with_adapter_report_field_coverage_from_report` to
project that pure report-field coverage into schema drift. The same missing
field, for example `adapter_closure.allow_next_round`, should then block
`schema_drift.allow_runner_wiring`, `report_bundle.complete`, and
`current_runner.allow_enforced_wiring` without re-reading report JSON, scanning
directories, or touching runner state.

Stable report fields for `schema_drift_report_v1`:

- `schema_drift.manifest_schema_names`
- `schema_drift.bundle_schema_names`
- `schema_drift.checklist_schema_names`
- `schema_drift.manifest_checksum`
- `schema_drift.bundle_checksum`
- `schema_drift.checklist_checksum`
- `schema_drift.manifest_report_field_checksum`
- `schema_drift.adapter_emission_report_field_checksum`
- `schema_drift.bundle_missing_schema_names`
- `schema_drift.bundle_extra_schema_names`
- `schema_drift.checklist_missing_schema_names`
- `schema_drift.checklist_extra_schema_names`
- `schema_drift.duplicate_schema_names`
- `schema_drift.missing_report_field_names`
- `schema_drift.extra_report_field_names`
- `schema_drift.duplicate_report_field_names`
- `schema_drift.drift_blocked`
- `schema_drift.failure_reasons`
- `schema_drift.allow_runner_wiring`

Recommended rollout:

1. `ShadowOnly`: keep checksum comparison inside crate tests.
2. `ReportOnly`: print checksum and mismatch fields, but keep
   `tools/evolution-loop` authoritative.
3. `Enforced`: require matching manifest, bundle, checklist, and report-field
   contract checksums, no missing schemas, no extra schemas, no duplicate schema
   names, and no missing/extra/duplicate adapter report fields before root
   adapter wiring can block the runner.

The report-field contract check should name a small set of sentinel fields in
fixtures and report-only review: `apple_silicon_effect.feedback_applied`,
`model_pool_attribution.validation_checked`,
`model_pool_budget.missing_required_roles`,
`context_rot_trend.latest_noisy_records`, and
`context_rot_remediation.allow_experiment_rollout`. The run-mode refresh
contract adds `report_freshness.allow_next_round` and
`remote_runtime.acceleration_ready` as additional sentinels before enforced
report bundle promotion. If any sentinel is missing,
keep the adapter in report-only and classify the problem as schema drift or
adapter report coverage, not model quality.

Missing `schema_drift_report_v1` in old JSONL ledgers is an additive gap. It
must not make legacy replay fail, and it must not be confused with model
quality, 8686 availability, or prompt-gate readiness.

## Apple Silicon Model-Pool Window

Use `norion_test::ModelPoolDevelopmentWindowPlan` and
`norion_eval::ModelPoolDevelopmentWindowReport` before reporting that multiple
Apple Silicon workers genuinely improve development.

Before a window record is allowed to contribute to that claim, emit
`model_pool_development_attribution_report_v1` from
`norion_eval::ModelPoolDevelopmentAttributionReport`. The future adapter should
build it from the same per-worker `ModelWorkerRecord` values plus classified
`RootAdapterFailureKind` values. The report is additive in shadow/report-only
mode and only becomes blocking in enforced rollout.

Required per-worker fields:

- `worker_id`
- `role`
- `model`
- `latency_ms`
- `runtime_tokens`
- `success`
- `feedback_applied`
- `validation_checked`
- `validation_passed`
- `duplicate_output`
- `noisy_output`
- `blocked_primary_12b`
- `failure_kind`

The enforced attribution gate requires planner, reviewer, and tester roles,
nonzero latency/tokens for successful workers, checked/passing validation,
applied feedback, no duplicate or noisy output, and no primary-12B blocking.
It also compares the runner-reported quality-failure count with the classified
`model_quality_failure` count. Worker-level `failure_kind` values should remain
worker-indexed so a single unavailable helper does not turn into a whole-pool
quality failure.

Before claiming that the Apple Silicon pool improved development, the final
effect report must expose the same worker-indexed arrays plus two readiness
order guards: `apple_silicon_effect.operational_readiness_failure_kind` and
`apple_silicon_effect.quality_failure_blocked_by_readiness_order`. These fields
make 8686 outage attribution auditable at the claim boundary instead of only in
the root adapter report.

Before that final effect report can allow a claim, emit
`apple_silicon_baseline_comparison_report_v1`. It compares paired primary 12B
and Apple Silicon pool evidence for the same development rounds. The claim
requires more applied feedback without success or validation regression,
latency/token budget misses, duplicate/noisy output, primary-12B blocking, or
readiness/quality confusion.

Attribution order is mandatory:

1. If prompt-gate blocked, including when 8686 is also down, classify
   `chain_not_ready`.
2. Else if prompt-gate passed and backend 8686 is down, classify
   `model_unavailable`.
3. Count neither value as `model_quality_failure`.
4. Count `model_quality_failure` only after final JSON, runtime model,
   runtime tokens, and `business_cycle.passed=false` are all present.

`chain_not_ready` and `model_unavailable` block
`model_pool_attribution.allow_development_claim`, but they should increment
operational readiness counters, not model-quality counters.

Build each window record from an existing `ModelPoolDevelopmentReport`:

- `feedback_delta`;
- `latency_multiplier`;
- `token_multiplier`;
- `pool_gate_blocked`;
- `development_gate_blocked`;
- `duplicate_outputs`;
- `noisy_outputs`;
- `primary_12b_blockers`;
- `root_adapter_failure_kind`.

Stable report fields for `model_pool_development_window_report_v1`:

- `model_pool_window.rounds`
- `model_pool_window.first_round`
- `model_pool_window.last_round`
- `model_pool_window.development_claim_rounds`
- `model_pool_window.development_claim_rate`
- `model_pool_window.operational_readiness_rounds`
- `model_pool_window.model_quality_failure_rounds`
- `model_pool_window.duplicate_outputs`
- `model_pool_window.noisy_outputs`
- `model_pool_window.primary_12b_blockers`
- `model_pool_window.feedback_delta_total`
- `model_pool_window.max_latency_multiplier`
- `model_pool_window.max_token_multiplier`
- `model_pool_window.window_blocked`
- `model_pool_window.allow_development_claim`
- `model_pool_window.failure_reasons`

Stable report fields for `apple_silicon_baseline_comparison_report_v1`:

- `apple_silicon_baseline.stage`
- `apple_silicon_baseline.paired_rounds`
- `apple_silicon_baseline.feedback_gain_rate`
- `apple_silicon_baseline.feedback_delta_total`
- `apple_silicon_baseline.success_regression_rounds`
- `apple_silicon_baseline.validation_regression_rounds`
- `apple_silicon_baseline.latency_budget_exceeded_rounds`
- `apple_silicon_baseline.token_budget_exceeded_rounds`
- `apple_silicon_baseline.duplicate_outputs`
- `apple_silicon_baseline.noisy_outputs`
- `apple_silicon_baseline.primary_12b_blockers`
- `apple_silicon_baseline.operational_readiness_rounds`
- `apple_silicon_baseline.model_quality_failure_rounds`
- `apple_silicon_baseline.max_latency_multiplier`
- `apple_silicon_baseline.max_token_multiplier`
- `apple_silicon_baseline.comparison_blocked`
- `apple_silicon_baseline.failure_reasons`
- `apple_silicon_baseline.allow_development_gain_claim`

Recommended enforced defaults:

- at least three rounds;
- development-claim rate at least `0.67`;
- feedback delta total at least `3`;
- max latency multiplier at most `1.5`;
- max token multiplier at most `2.0`;
- zero duplicate outputs;
- zero noisy outputs;
- zero primary 12B blockers.

If prompt-gate blocks while 8686 is down, count the round as
`chain_not_ready`. If prompt-gate passed and 8686 is unreachable, count it as
`model_unavailable`. Both values block the Apple Silicon development claim as
operational readiness failures, but neither may increment
`model_quality_failure_rounds`, `model_quality_failure_count`, or
`reported_model_quality_failures`.

## Context Rot Gate

Plan the audit with `norion_test::ExperienceAuditPlan` and evaluate the result
with `norion_eval::ContextRotSignal` plus `ContextRotGate`.

Current experience-audit fields should map directly:

- `noisy_records` -> `ContextRotSignal::noisy_records`
- `max_noise_penalty` -> `ContextRotSignal::max_noise_penalty`
- `quarantine_candidates` -> `ContextRotSignal::quarantine_candidates`
- `repairable_legacy_metadata_lessons` ->
  `ContextRotSignal::repairable_legacy_metadata_lessons`
- `legacy_metadata_without_clean_gist` ->
  `ContextRotSignal::legacy_metadata_without_clean_gist`

Current experience cleanup/index audit reports exact repeated long lesson
outputs as `duplicate_outputs`; map that field to
`ContextRotSignal::duplicate_outputs`. Keep it report-only until the
evolution-loop gate exposes a dedicated duplicate-output threshold.

Before root adapter wiring, `AdapterReportEmissionPlan` should also require
these future Context Rot events so missing noise/remediation evidence appears as
an emission gap rather than a readiness or model-quality failure:

- `context_rot_noisy_records`
- `context_rot_noise_penalty`
- `context_rot_duplicate_outputs`
- `context_rot_quarantine_candidates`
- `context_rot_trend_window_rounds`
- `context_rot_consecutive_noisy_rounds`
- `context_rot_consecutive_duplicate_rounds`
- `context_rot_remediation_applied`
- `context_rot_clean_gist_backfilled`
- `context_rot_legacy_metadata_repaired`

Strict Context Rot gate defaults are zero tolerance for noisy records,
quarantine candidates, legacy metadata drift, duplicate outputs, and positive
noise penalty. Relaxed thresholds are acceptable in advisory mode only.

Use `norion_test::ContextRotAcceptancePlan` and
`norion_eval::ContextRotAcceptanceContract` when staging this evidence:

- `ShadowOnly`: run or fixture the audit without printing new report fields.
- `ReportOnly`: print additive `context_rot.*` raw metrics, but keep the gate
  advisory and do not block experiment rollout.
- `Enforced`: allow `context_rot.gate_blocked` and
  `context_rot.failure_reasons` to stop unattended expansion and block enabled
  experiment flags until the strict gate is clean.

Use `ContextRotAcceptanceContract::advisory_report()` for report-only adapters
and `blocking_report()` for enforced adapters so stage semantics stay in
`norion-eval`.

Stable report fields for `context_rot_report_v1`:

- `context_rot.noisy_records`
- `context_rot.max_noise_penalty`
- `context_rot.quarantine_candidates`
- `context_rot.repairable_legacy_metadata_lessons`
- `context_rot.legacy_metadata_without_clean_gist`
- `context_rot.duplicate_outputs`
- `context_rot.gate_blocked`
- `context_rot.failure_reasons`

Adapter boundary: keep audit HTTP calls and JSON field extraction in
`tools/evolution-loop`, then project the resulting `ContextRotSignal` and
`ContextRotGate` decision with `norion_eval::ContextRotReport`. This keeps
`norion-eval` data-only while making `context_rot_report_v1` reusable in tests
and future additive report output.

Stable report fields for `context_rot_trend_report_v1`:

- `context_rot_trend.rounds`
- `context_rot_trend.first_round`
- `context_rot_trend.last_round`
- `context_rot_trend.latest_noisy_records`
- `context_rot_trend.latest_duplicate_outputs`
- `context_rot_trend.noisy_records_delta`
- `context_rot_trend.duplicate_outputs_delta`
- `context_rot_trend.max_consecutive_noisy_rounds`
- `context_rot_trend.max_consecutive_duplicate_rounds`
- `context_rot_trend.remediation_applied_rounds`
- `context_rot_trend.remediation_improved_noise`
- `context_rot_trend.remediation_improved_duplicates`
- `context_rot_trend.trend_blocked`
- `context_rot_trend.allow_unattended_continuation`
- `context_rot_trend.failure_reasons`

During report-only rollout, compute the trend window from recent ledger/audit
rows and print deltas without stopping the runner. During enforced rollout,
require at least three rounds of evidence, block consecutive noisy or duplicate
outputs, and require remediation-applied windows to improve noise and duplicate
trends before unattended rounds continue.
When the runner already has a window of audit snapshots, build
`ContextRotTrendPoint` values locally and project them with
`ContextRotTrendReport::from_points_and_gate()`.

Stable report fields for `context_rot_remediation_report_v1`:

- `context_rot_remediation.stage`
- `context_rot_remediation.quarantine_candidates`
- `context_rot_remediation.quarantined_records`
- `context_rot_remediation.repairable_legacy_metadata_lessons`
- `context_rot_remediation.repaired_legacy_metadata_lessons`
- `context_rot_remediation.legacy_metadata_without_clean_gist`
- `context_rot_remediation.clean_gists_backfilled`
- `context_rot_remediation.duplicate_outputs`
- `context_rot_remediation.duplicate_outputs_removed`
- `context_rot_remediation.remediation_blocked`
- `context_rot_remediation.failure_reasons`
- `context_rot_remediation.allow_experiment_rollout`

During enforced rollout, remediation must quarantine all candidates, repair
legacy metadata, backfill clean gists, and remove duplicate outputs before
experiments expand or unattended rounds continue.

Until remediation events are wired, use
`ContextRotRemediationReport::from_gate_and_signal()` for adapter tests: it
projects the audit signal into report-only cleanup evidence and keeps applied
cleanup counts at zero so enforced gates still block dirty state.

## Experiment Rollout Gate

Use `ExperimentFlag` and `ExperimentRolloutGate` before expanding any new
self-evolution feature. Conservative rollout rules are:

- enabled flags must have a non-empty owner;
- enabled flags cannot exceed 10% rollout before the main window explicitly
  raises the cap;
- enabled flags require a clean Context Rot gate;
- disabled flags may remain present while dirty Context Rot evidence is being
  investigated, because they do not expand runtime behavior.

This keeps experiments from hiding behind green validation when the memory or
prompt surface is getting noisier.

Plan experiment rollout reporting with
`norion_test::ExperimentRolloutReportPlan` and project results with
`norion_eval::ExperimentRolloutReport::from_flag_and_decision`.

Stable report fields for `experiment_rollout_report_v1`:

- `experiment.name`
- `experiment.enabled`
- `experiment.rollout_percent`
- `experiment.owner`
- `experiment.rollout_blocked`
- `experiment.failure_reasons`
- `experiment.requires_clean_context_rot`

During report-only rollout, emit flag state and ownership fields. During
enforced rollout, require clean Context Rot when the gate is configured that way
and treat `experiment.rollout_blocked=true` as a stop signal for expanding the
experiment.

## Experiment Kill Switch Gate

Use `norion_test::ExperimentKillSwitchReportPlan` for any enabled experiment
before increasing rollout. This is separate from the rollout percentage gate:
even a 1% rollout needs a documented escape hatch.

Required evidence for enabled flags during enforced rollout:

- non-empty owner;
- documented kill switch;
- `rollback_report_v1` present;
- rollback resume gate present;
- clean Context Rot;
- owner acknowledgement of rollback responsibility.

Stable report fields for `experiment_kill_switch_report_v1`:

- `experiment_kill_switch.name`
- `experiment_kill_switch.enabled`
- `experiment_kill_switch.owner`
- `experiment_kill_switch.kill_switch_documented`
- `experiment_kill_switch.rollback_report_present`
- `experiment_kill_switch.rollback_resume_gate_present`
- `experiment_kill_switch.context_rot_clean`
- `experiment_kill_switch.owner_acknowledged`
- `experiment_kill_switch.kill_switch_blocked`
- `experiment_kill_switch.failure_reasons`
- `experiment_kill_switch.allow_experiment_expansion`

Disabled flags remain non-blocking by default, so a disabled experiment can
exist while cleanup work continues. Enabled flags must be able to roll back
before they can expand.

## Experiment Expansion Safety

Use `norion_test::ExperimentExpansionSafetyReportPlan` before expanding an
enabled experiment such as the Apple Silicon multi-model pool. This report is
the aggregate pre-expansion gate; it should remain report-only until all input
reports are observed.

Required enforced evidence:

- rollout report passed;
- kill-switch report passed;
- Context Rot report, trend, and remediation passed;
- rollback report present and rollback resume passed;
- model-pool development attribution passed;
- adapter report emission passed;
- adapter report field coverage passed;
- Apple Silicon development-effect gate passed;
- adapter promotion window passed;
- next-round readiness passed;
- Steam case matrix passed;
- validation command coverage passed;
- root adapter failure kind is `none`.

Stable report fields for `experiment_expansion_safety_report_v1` include:

- `experiment_expansion.model_pool_attribution_passed`
- `experiment_expansion.adapter_report_emission_passed`
- `experiment_expansion.adapter_report_field_coverage_passed`
- `experiment_expansion.apple_silicon_development_effect_passed`
- `experiment_expansion.promotion_window_passed`
- `experiment_expansion.readiness_passed`
- `experiment_expansion.steam_case_matrix_passed`
- `experiment_expansion.validation_command_coverage_passed`
- `experiment_expansion.root_adapter_failure_kind`
- `experiment_expansion.reported_model_quality_failures`
- `experiment_expansion.classified_model_quality_failures`
- `experiment_expansion.allow_experiment_expansion`

If adapter report emission, adapter report field coverage, Apple Silicon
development-effect, promotion window, readiness, Steam case matrix, or
validation command coverage is missing, keep the experiment advisory. Derive
field coverage from `AdapterReportEmissionReport::field_coverage_passed()` so
`adapter_emission.missing_report_fields` blocks expansion before any experiment
flag can claim Apple Silicon development impact. If root attribution reports
`chain_not_ready` or `model_unavailable`, block expansion as readiness or
availability, not as `model_quality_failure`.

Add `experiment_switch_matrix_report_v1` before multiple experiment flags can
expand together. It should report:

- `experiment_switch.flag_names`
- `experiment_switch.enabled_flag_names`
- `experiment_switch.reported_enabled_flag_names`
- `experiment_switch.missing_enabled_flag_reports`
- `experiment_switch.duplicate_enabled_flag_reports`
- `experiment_switch.unknown_enabled_flag_reports`
- `experiment_switch.expansion_report_count`
- `experiment_switch.blocked_expansion_reports`
- `experiment_switch.all_enabled_reported`
- `experiment_switch.exactly_one_report_per_enabled_flag`
- `experiment_switch.all_expansion_reports_passed`
- `experiment_switch.allow_experiment_switch_expansion`

In report-only rollout this is an inventory of active switches. In enforced
rollout every enabled flag must have exactly one expansion safety report, no
report may reference an unknown or disabled flag, and every expansion safety
report must pass. This prevents the Apple Silicon model pool, parallel tester
pool, and future helper-role experiments from expanding as separate
uncorrelated toggles.

Rollout sequence for future root adapter wiring:

1. `shadow-only`: collect flag names and report names without blocking current
   `tools/evolution-loop` scheduling.
2. `report-only`: emit duplicate, unknown, missing, and blocked report arrays in
   ledger/report bundles; treat any gaps as adapter coverage findings.
3. `enforced`: block experiment expansion only after norion-test,
   norion-eval, evolution-loop, and workspace tests are green with exactly-one
   report coverage.

## Feedback And Self-Improve Report

Use `norion_test::FeedbackSelfImproveReportPlan` to stage the closed-loop report
without changing `tools/evolution-loop`. The future adapter should build a
`LedgerSummary` and project it with
`norion_eval::FeedbackSelfImproveReport::from_summary_and_gate`, or pass
normalized ledger rows directly to `from_records_and_gate`.
Use
`norion_eval::AdapterFeedbackSelfImproveBoundaryContract::feedback_self_improve_v1`
and `norion_test::AdapterFeedbackSelfImproveBoundaryPlan` before wiring the
adapter: the runner owns JSONL/file reading and normalization, while eval only
accepts `LedgerRecord`, `LedgerSummary`, `ReportGate`, and `GateDecision` to
produce `LedgerSummary`, `GateDecision`, and `FeedbackSelfImproveReport`. This
boundary forbids JSONL/file IO, HTTP/SSE, process spawning, validation command
spawning, model calls, and runner state. `stays_pure_data_boundary()` is the
fixture helper for keeping the feedback/self-improve report as pure report-gate
data.

Stable report fields for `feedback_self_improve_report_v1`:

- `feedback.total_applied`
- `feedback.items`
- `feedback.validation_checked`
- `feedback.validation_passed`
- `feedback.self_improve_checked`
- `feedback.self_improve_passed`
- `feedback.self_improve_pass_rate`
- `feedback.closed_loop_blocked`
- `feedback.failure_reasons`

During report-only rollout, emit only the raw counters and pass rate. During
enforced rollout, `feedback.closed_loop_blocked=true` means the same
`ReportGate` that already protects feedback or self-improve has blocked the next
unattended round. This report should not invent a second policy; it makes the
existing closed-loop gate auditable.

## Self-Evolution Continuity Report

Use `norion_test::SelfEvolutionContinuityReportPlan` and project results with
`norion_eval::SelfEvolutionContinuityReport::from_records_and_gate` before
letting a report-only self-evolution adapter enforce cross-round continuity.
The current runner should keep ledger IO and JSON parsing local, then pass only
adjacent normalized `LedgerRecord` values into the eval helper.

Stable report fields for `self_evolution_continuity_report_v1`:

- `self_evolution.previous_round`
- `self_evolution.current_round`
- `self_evolution.previous_feedback_applied`
- `self_evolution.feedback_carried_forward`
- `self_evolution.self_improve_checked`
- `self_evolution.self_improve_passed`
- `self_evolution.validation_passed`
- `self_evolution.continuity_blocked`
- `self_evolution.failure_reasons`
- `self_evolution.allow_next_round`

During enforced rollout, previous feedback must be carried into the next
self-improve round, the rounds must be adjacent, self-improve must pass, and the
follow-up validation must pass before unattended rounds continue.

## Self-Evolution Regression Report

Use `norion_test::SelfEvolutionRegressionReportPlan` and project results with
`norion_eval::SelfEvolutionRegressionReport::from_records_and_gate` over a
recent ledger window before allowing unattended continuation.
The helper is the stable adapter surface for the selected window; the runner or
future adapter still owns window selection and record normalization.

Stable report fields for `self_evolution_regression_report_v1`:

- `self_evolution_regression.rounds`
- `self_evolution_regression.first_round`
- `self_evolution_regression.last_round`
- `self_evolution_regression.feedback_delta`
- `self_evolution_regression.validation_pass_rate`
- `self_evolution_regression.self_improve_pass_rate`
- `self_evolution_regression.tail_validation_failures`
- `self_evolution_regression.tail_self_improve_failures`
- `self_evolution_regression.regression_blocked`
- `self_evolution_regression.allow_unattended_continuation`
- `self_evolution_regression.failure_reasons`

During report-only rollout, emit the window counters and rates without stopping
the runner. During enforced rollout, require at least three rounds of evidence,
no feedback regression, passing validation/self-improve rates, and no tail
validation or self-improve failures before unattended rounds continue.

## Advice Continuation Report

Use `norion_test::AdviceContinuationReportPlan` and
`norion_test::AdviceContinuationBoundaryPlan` with
`norion_eval::AdviceContinuationReport` when the runner has already extracted
normalized continuation-advice observations from summaries, operators, or other
high-level loop artifacts. Eval may normalize `AdviceContinuationObservation`
values against `LedgerSummary` and emit `advice_continuation_report_v1`, but it
must not scan report directories, replay JSONL, execute commands, restart a
daemon, call a model, or inspect runner state.

Keep this report scoped to observability unless the architecture explicitly
promotes it later. It may project whether continuation advice repeated earlier
guidance, suggested invalid commands, or arrived before the latest ledger round
succeeded. When downstream readiness or unattended-prerequisite reports consume
it, they should lift only report-only observation bits and must not treat
`advice_continuation_report_v1` as part of the enforced report bundle.
It is also intentionally excluded from the current eval schema manifest,
report-only bundle, enforced bundle, and handoff required-schemas list.

## Self-Evolution Unattended Prerequisites Report

Use `norion_test::SelfEvolutionUnattendedPrerequisiteReportPlan` and project
results with `norion_eval::SelfEvolutionUnattendedPrerequisiteReport` before
claiming that self-evolution can continue unattended. This report consumes
already-produced report decisions; it must not parse JSONL, call backend 8686,
run validation commands, or change `tools/evolution-loop`.

Stable report fields for
`self_evolution_unattended_prerequisites_report_v1`:

- `self_evolution_unattended.stage`
- `self_evolution_unattended.continuity_passed`
- `self_evolution_unattended.regression_passed`
- `self_evolution_unattended.readiness_next_round_passed`
- `self_evolution_unattended.advice_continuation_observed`
- `self_evolution_unattended.advice_continuation_passed`
- `self_evolution_unattended.context_rot_trend_passed`
- `self_evolution_unattended.context_rot_remediation_passed`
- `self_evolution_unattended.rollback_resume_passed`
- `self_evolution_unattended.steam_case_matrix_passed`
- `self_evolution_unattended.validation_command_coverage_passed`
- `self_evolution_unattended.promotion_window_passed`
- `self_evolution_unattended.adapter_report_field_coverage_passed`
- `self_evolution_unattended.apple_silicon_development_effect_passed`
- `self_evolution_unattended.prerequisite_blocked`
- `self_evolution_unattended.allow_unattended_self_evolution_claim`
- `self_evolution_unattended.failure_reasons`

During report-only rollout, emit the pass bits as evidence. During enforced
rollout, block the unattended self-evolution claim unless continuity,
regression, next-round readiness, Context Rot trend/remediation, rollback
resume, Steam matrix, validation coverage, promotion window, adapter report
field coverage, and Apple Silicon development-effect reports all passed.
Derive field coverage from `AdapterReportEmissionReport::field_coverage_passed()`
so a clean local feedback window cannot mask missing cleanup, recovery, coverage
gates, or omitted worker/effect report fields.
When readiness has already been projected, pass the
`SelfEvolutionReadinessReport` through
`SelfEvolutionUnattendedPrerequisiteEvidence::with_readiness_report`; this maps
only `readiness.can_schedule_next_round` into
`self_evolution_unattended.readiness_next_round_passed`. Do not rebuild that bit
by scanning report directories, replaying JSONL, spawning validation commands, or
querying runner state.
When the surrounding operational reports are available together, use
`with_operational_gate_reports` to map the same pure report bits from readiness,
Context Rot trend/remediation, rollback resume, Steam case matrix, and validation
command coverage into the unattended prerequisite evidence. The runner still owns
collection and execution; eval only consumes report values.
When continuation advice has been normalized separately, use
`SelfEvolutionUnattendedPrerequisiteEvidence::with_advice_continuation_report`
to project only `AdviceContinuationReport::allow_continuation` plus the observed
bit into `self_evolution_unattended.advice_continuation_*`. Keep this additive:
the current contract requires advice continuation failures to stay visible while
`self_evolution_unattended.prerequisite_blocked` remains unchanged.
For this mapping, Steam coverage uses
`SteamCaseCoverageReport::allow_enforced_adapter`, while validation command
coverage uses `ValidationCommandCoverageReport::allow_next_round`.
Promotion window coverage uses
`AdapterPromotionWindowReport::allow_enforcement`; do not invent an
`allow_enforcement_promotion` field at the adapter boundary.
Mirror that adapter-facing contract in
`norion_test::SelfEvolutionUnattendedPrerequisiteReportPlan`: its
`operational_report_sources` should list only these eval report fields and its
`forbidden_capabilities` should reject JSONL IO, HTTP/SSE, process spawning,
validation-command spawning, model calls, and runner state.

## Ledger Gate Report

Use `norion_test::LedgerGateReportPlan` to stage the report gate itself. The
future adapter should read current JSONL rows, map them into `LedgerRecord`,
and project with `norion_eval::LedgerGateReport::from_records_and_gate`; callers
that already have a summary can use `from_summary_and_gate`.
The current `tools/evolution-loop` report JSON now emits an additive
`ledger_gate_report_v1` object from its existing ledger summary and threshold
gate failures; the legacy `report_gate` field remains the authoritative
current-runner stop surface while helper, remote-chain, and model-pool policy
checks are still staged outside the pure ledger gate report.
Use `from_summary_and_failure_reasons` in the current adapter when preserving
legacy failure wording is required.

For run-mode refresh, keep `run-report-json` / `report_refresh:start` /
`report_refresh:done` as runner-owned IO. A live status such as
`report rounds=275`, `ledger_lag=0`, `stale=false`, and `gate_failures=0`
proves the daemon refreshed its report view, but `norion-eval` should only
consume the normalized projection: `LedgerSummary` values, the
`LedgerGateReport` fields below, and explicit report-gate decisions. Do not
add daemon PID, report-file polling, JSONL scanning, or stale-report timers to
the eval crate.

Remote runtime acceleration evidence follows the same line. The runner may
observe that the remote pool is healthy, for example `6/6` workers available on
Metal, but eval consumes that only after it has been normalized into existing
pure data structures such as `ModelWorkerRecord`,
`ModelPoolDevelopmentAttributionReport`,
`AppleSiliconBaselineComparisonReport`, or
`AppleSiliconDevelopmentEffectReport`. Health checks, SSH, daemon control,
remote model calls, and Metal-device probing stay outside `norion-eval`; missing
or stale remote evidence should surface as incomplete worker/runtime evidence
or a blocked Apple Silicon development-effect report, not as eval-side IO.

Stable report fields for `ledger_gate_report_v1`:

- `ledger.total_rounds`
- `ledger.success_rate`
- `ledger.validation_pass_rate`
- `ledger.rust_check_checked`
- `ledger.rust_check_passed`
- `ledger.rust_check_feedback_applied_total`
- `ledger.runtime_response_failures`
- `ledger.stream_truncations`
- `ledger.missing_final_failures`
- `ledger.duplicate_rounds`
- `ledger.round_gaps`
- `ledger.state_gate_pass_rate`
- `ledger.trace_gate_pass_rate`
- `ledger.context_noise_penalty_max`
- `ledger.last_success`
- `ledger.gate_blocked`
- `ledger.failure_reasons`
- `ledger.allow_next_round`

During report-only rollout, emit the summary fields so duplicate rounds, round
gaps, missing final events, runtime response failures, state/trace failures, and
last-round failures are visible without stopping the runner. During enforced
rollout, `ledger.allow_next_round=false` is the report-gate stop signal.

## Multi-Model Pool Gate

Future multi-model runs should emit one `ModelWorkerRecord` per participant
before the round result is merged. The minimal record is:

- `worker_id`
- `role`: `planner`, `reviewer`, `tester`, `summarizer`, or `high_quality`
- `model`
- `latency_ms`
- `runtime_tokens`
- `success`
- `feedback_applied`
- `validation_checked` and `validation_passed`
- `duplicate_output`
- `noisy_output`
- `blocked_primary_12b`

Evaluate `ModelPoolSummary::from_workers` and `ModelPoolGate` before accepting
the pool output. Strict mode is intentionally conservative:

- every worker must succeed;
- every worker must have validation checked and passed;
- the pool must apply feedback;
- duplicate and noisy outputs must be zero;
- no worker may block the primary 12B path;
- optional caps can bound per-worker latency and runtime token cost.

`validation_passed=true` is not sufficient unless `validation_checked=true` is
also present for that worker row. Treat that mismatch as incomplete worker
coverage that blocks an Apple Silicon development-effect claim; do not route it
through model-quality failure accounting.

This lets planner/reviewer/tester/summarizer workers increase coverage without
silently slowing, duplicating, or starving the high-quality 12B path.

The model-pool report should preserve raw audit counts next to rates:
`model_pool.successful_workers`, `model_pool.validation_checked`,
`model_pool.validation_passed`, `model_pool.latency_ms_max`, and
`model_pool.runtime_tokens_max`. These fields make per-worker latency/token
caps and unchecked validation visible even when totals or percentages look
healthy.

When the runner begins emitting `model_pool_development_attribution_report_v1`,
carry `model_pool_attribution.validation_checked` separately from
`model_pool_attribution.validation_passed`. A pass bit without a checked bit is
treated as incomplete worker evidence. It blocks Apple Silicon development
claims through attribution/report-field coverage, not through
`model_quality_failure`.

The adapter emission gate should list those attribution fields in
`adapter_emission.planned_report_fields` before promotion:
`model_pool_attribution.latency_ms`,
`model_pool_attribution.runtime_tokens`, `model_pool_attribution.success`,
`model_pool_attribution.feedback_applied`,
`model_pool_attribution.validation_checked`,
`model_pool_attribution.validation_passed`,
`model_pool_attribution.duplicate_output`,
`model_pool_attribution.noisy_output`, and
`model_pool_attribution.blocked_primary_12b`. Also emit
`model_pool_attribution.chain_not_ready_count` and
`model_pool_attribution.model_unavailable_count` so 8686-down and prompt-gate
blocked rounds remain visible as readiness/availability failures instead of
being folded into model quality.

The plan side should be described with `norion_test::ModelPoolSmokePlan` before
execution:

- add one `ModelWorkerPlan` for each intended worker;
- keep `may_block_primary_12b=false` for helper workers by default;
- use a worker-level `VerificationPlan` for role-specific checks;
- use the merge `VerificationPlan` for the final combined output;
- carry each worker `id` and `role` into the produced `ModelWorkerRecord`.

## Adapter Steps For tools/evolution-loop

When the main window allows changes in `tools/evolution-loop`, add a thin
adapter only:

1. Convert each current ledger row into `norion_eval::LedgerRecord`.
2. Convert future per-worker round evidence into `ModelWorkerRecord`.
3. Keep SSE parsing, command execution, ledger file IO, and HTTP calls in the
   existing runner.
4. Replace duplicated report-gate calculations with calls to
   `LedgerSummary::from_records`, `ReportGate::evaluate`,
   `ModelPoolSummary::from_workers`, and `ModelPoolGate::evaluate`.
5. Preserve current JSONL fields while adding worker evidence as optional
   metadata, so old ledgers remain readable.

## Root Business-Cycle Adapter Gate

Before root adapter wiring, treat root business-cycle JSON fields as either
strong or weak evidence.

Strong fields can become enforced gates after clean fixtures exist:

- `$.business_cycle.passed` -> `LedgerRecord.business_cycle_passed`
- `$.business_cycle.feedback_applied` -> `LedgerRecord.feedback_applied`
- `$.business_cycle.rust_check_checked` -> `LedgerRecord.rust_check_checked`
- `$.business_cycle.rust_check_passed` -> `LedgerRecord.rust_check_passed`
- `$.business_cycle.self_improve_checked` -> `LedgerRecord.self_improve_checked`
- `$.business_cycle.self_improve_passed` -> `LedgerRecord.self_improve_passed`
- `$.business_cycle.state_gate_checked` -> `LedgerRecord.state_gate_checked`
- `$.business_cycle.state_gate_passed` -> `LedgerRecord.state_gate_passed`
- `$.business_cycle.trace_gate_checked` -> `LedgerRecord.trace_gate_checked`
- `$.business_cycle.trace_gate_passed` -> `LedgerRecord.trace_gate_passed`

Weak fields should stay report-only until prompt and infrastructure gates prove
generation ran:

- `$.generate.runtime_model` -> `LedgerRecord.runtime_model` and
  `ModelWorkerRecord.model`
- `$.generate.runtime_token_count` -> `LedgerRecord.runtime_tokens` and
  `ModelWorkerRecord.runtime_tokens`
- `$.generate.answer` -> `LedgerRecord.answer`
- `$.generate.elapsed_ms` -> `ModelWorkerRecord.latency_ms`

Rollout stages:

1. `ShadowOnly`: compute `LedgerRecord` and `ModelWorkerRecord` projections but
   do not print, persist, or block on them.
2. `ReportOnly`: print or write additive adapter summaries; weak mapping
   failures remain advisory.
3. `Enforced`: allow strong adapter gates to block only after outage
   attribution tests, `norion-test`, `norion-eval`, `tools/evolution-loop`, and
   workspace tests are green.

Outage attribution order:

1. If prompt-gate blocked, classify `chain_not_ready`.
2. Else if backend 8686 is unreachable, classify `model_unavailable`.
3. Else if final JSON is absent, classify `StreamOrFinalMissing`.
4. Else if runtime model or tokens are absent, classify `RuntimeResponseMissing`.
5. Else if `business_cycle.passed=false`, classify `model_quality_failure`.

This order prevents the root adapter from counting a missing model as model
quality failure when 8686 is unavailable or the prompt gate stopped generation.

## Apple Silicon Baseline Adapter Wiring Gate

Before claiming that the Apple Silicon model pool improves development, wire a
paired-baseline projection in stages. The current runner can supply only the
pool-side shadow projection; the primary 12B baseline comparison needs future
events.

Current projection fields:

- `round`: current ledger `round`;
- `pool_feedback_applied_total`: sum of worker `feedback_applied`;
- `pool_latency_ms_total`: sum of worker `latency_ms`;
- `pool_runtime_tokens_total`: sum of worker `runtime_tokens`;
- `pool_success`: required workers passed and root business-cycle passed;
- `pool_validation_passed`: required worker validations passed;
- `duplicate_outputs`: worker duplicate-output count;
- `noisy_outputs`: worker noisy-output count;
- `primary_12b_blockers`: worker primary-12B blocking count;
- `root_adapter_failure_kind`: readiness-before-quality classification.

Future events required before enforcement:

- `baseline_12b_feedback_applied`;
- `baseline_12b_latency_ms`;
- `baseline_12b_runtime_tokens`;
- `baseline_12b_success`;
- `baseline_12b_validation_passed`.

Rollout gate:

1. `shadow-project-pool-side-baseline-fields`: compute fields in memory or test
   fixtures only; do not print, persist, or block.
2. `report-only-paired-baseline-coverage`: report missing baseline events as
   coverage gaps.
3. `report-only-apple-silicon-baseline-comparison`: emit the additive baseline
   comparison report when paired evidence exists.
4. `enforced-apple-silicon-baseline-comparison`: allow blocking only after
   paired baseline events, worker metrics, outage attribution, and
   norion-test/norion-eval/evolution-loop/workspace tests are green.

The attribution rule is mandatory: prompt-gate blocked plus 8686 unavailable is
`chain_not_ready`; prompt-gate passed plus 8686 unavailable is
`model_unavailable`. Both block the development-effect claim as readiness or
availability, and both must keep `model_quality_failure_rounds`,
`model_quality_failure_count`, and `reported_model_quality_failures` at zero.

## Root Adapter Attribution Report

Use `norion_test::RootAdapterAttributionReportPlan` before letting root adapter
classification affect model-quality metrics. The future adapter should collect
prompt-gate, backend 8686, final JSON, runtime model/tokens, and
`business_cycle.passed` evidence, then project it with
`norion_eval::RootAdapterAttributionReport::from_evidence_for_stage`.

Stable report fields for `root_adapter_attribution_report_v1`:

- `root_adapter.backend_8686_reachable`
- `root_adapter.prompt_gate_blocked`
- `root_adapter.final_json_present`
- `root_adapter.runtime_model_present`
- `root_adapter.runtime_tokens`
- `root_adapter.business_cycle_passed`
- `root_adapter.failure_kind`
- `root_adapter.model_quality_failure_allowed`
- `root_adapter.rollback_required`
- `root_adapter.rollback_reason`

During report-only rollout, emit the evidence fields and failure kind as
observability. During enforced rollout, allow
`root_adapter.model_quality_failure_allowed=true` only after final JSON and
runtime evidence exist and the business-cycle verdict failed. If 8686 is down or
the prompt gate blocked the request, the report must show `chain_not_ready` or
`model_unavailable`, not `model_quality_failure`.

## ModelWorkerRecord Adapter Wiring Plan

The first wiring step should treat every current evolution round as one
synthetic `high_quality` worker. That lets the model-pool eval layer run on
today's ledger without starting parallel workers or blocking the existing
runner.

Current ledger fields to map now:

- `worker_id`: derive from `case`, or from `case` plus `round` when uniqueness
  matters.
- `role`: derive as `high_quality` until the runner emits per-worker roles.
- `model`: read from `runtime_model`.
- `latency_ms`: read from `elapsed_ms`.
- `runtime_tokens`: read from `runtime_tokens`.
- `success`: derive from `success` and `business_cycle_passed`.
- `feedback_applied`: read from `feedback_applied`.
- `validation_checked`: read from `validation_checked`.
- `validation_passed`: read from `validation_passed`.

Fields that need future optional worker events:

- `duplicate_output`: needs `worker_output_fingerprint` or equivalent answer
  hash comparison.
- `noisy_output`: needs `worker_noise_score`, Context Rot score, or an
  experience-audit verdict.
- `blocked_primary_12b`: needs `worker_primary_wait_ms`, a queue wait metric,
  or an explicit scheduler-block reason.
- `failure_kind`: needs `worker_failure_kind`, derived after prompt-gate,
  backend 8686, final JSON, runtime response, and business-cycle verdict
  attribution.
- `development_claim_allowed` and `claim_blockers`: derive from the fields
  above after projection. A worker is not allowed to support an Apple Silicon
  development-effect claim if it lacks latency/tokens, did not succeed, applied
  no feedback, skipped or failed validation, duplicated or polluted output,
  blocked the primary 12B, or carried any non-`none` failure kind.

## Apple Silicon Development Effect Gate

After the per-worker `ModelPoolGate` passes, compare the pool against the
primary 12B baseline with `AppleSiliconDevelopmentGate`.

Required evidence:

- baseline 12B latency and runtime tokens;
- baseline 12B success and validation result;
- baseline 12B feedback count;
- pool total latency and runtime tokens;
- pool success rate and validation pass rate;
- pool feedback total;
- pool duplicate/noisy output counts;
- pool primary-12B blocking count.

Conservative defaults:

- pool success cannot regress from a passing 12B baseline;
- pool validation cannot regress from a passing 12B baseline;
- pool feedback must exceed baseline feedback by at least one update;
- pool latency must stay within 1.5x the 12B baseline;
- pool runtime tokens must stay within 2.0x the 12B baseline;
- duplicate output, noisy output, and primary-12B blockers must be zero.

This is the gate that answers whether Apple Silicon parallelism actually helped
development. A pool that produces more text but fails validation, duplicates
answers, burns tokens, or blocks the main 12B path should remain advisory.

For report-only wiring, project the pool with
`ModelPoolDevelopmentReport::from_summary_effect_and_decisions`. It should show
the worker count, latency/tokens, success and validation pass rates, feedback
delta, duplicate/noisy output counts, primary-12B blockers, and merged gate
reasons. Carry `root_adapter.failure_kind` as attribution metadata only:
`chain_not_ready` and `model_unavailable` mean the prompt chain or backend was
not ready, not that the model pool had a quality failure.

Before allowing the pool to influence enforced development claims, emit
`apple_silicon_development_effect_report_v1` from existing eval reports. This
report is a pure aggregate: it consumes worker attribution, budget fairness,
the multi-round development window, paired baseline comparison, and root
adapter attribution. It should not parse JSON, call backend 8686, run
validation commands, or alter `tools/evolution-loop`.

The enforced aggregate gate requires:

- `model_pool_development_attribution_report_v1.allow_development_claim=true`;
- `model_pool_budget_fairness_report_v1.allow_pool_expansion=true`;
- `model_pool_development_window_report_v1.allow_development_claim=true`;
- `apple_silicon_baseline_comparison_report_v1.allow_development_gain_claim=true`;
- consistent per-worker rows for latency, runtime tokens, success, feedback,
  validation, duplicate output, noisy output, primary-12B blocking, and
  failure kind;
- `apple_silicon_effect.worker_ids`, `roles`, `latency_ms`,
  `runtime_tokens`, `success`, `feedback_applied`, `validation_checked`,
  `validation_passed`, `duplicate_output`, `noisy_output`,
  `blocked_primary_12b`, and `failure_kinds` present as reportable proof rows,
  not only hidden adapter state;
- `apple_silicon_effect.worker_development_claim_allowed` is true for every
  worker and `apple_silicon_effect.worker_claim_blockers` is empty for every
  worker;
- `apple_silicon_effect.worker_metric_coverage_passed=true`, meaning every
  worker has the metric columns above and no worker has unchecked/failed
  validation, duplicate output, noisy output, or primary-12B blocking;
- `apple_silicon_effect.baseline_comparison_allowed=true`;
- `apple_silicon_effect.model_quality_failure_allowed=false` for
  `chain_not_ready` and `model_unavailable`;
- `apple_silicon_effect.operational_failure_counted_as_quality=false`;
- no successful worker missing latency or runtime tokens;
- no operational readiness failures in the attribution report;
- reported quality failures equal classified `model_quality_failure` count.

Use the root adapter classification order before filling the report:

1. If prompt-gate blocked, classify `chain_not_ready`, including when 8686 is
   also unreachable.
2. Else if backend 8686 is unreachable, classify `model_unavailable`.
3. Else require final JSON plus runtime model/tokens before judging model
   quality.
4. Only then may a failed business-cycle verdict become
   `model_quality_failure`.

This makes Apple Silicon wins measurable without masking missing chain
readiness or backend availability as model quality.

When 8686 is down and prompt-gate blocked in the same round, do not increment
any worker or aggregate `model_quality_failure` counter. The report should show
`chain_not_ready`, zero classified quality failures, and a blocked
development-effect claim, with `model_quality_failure_allowed=false` and
`operational_failure_counted_as_quality=false`. When prompt-gate passed but
8686 is down, report `model_unavailable` with the same zero-quality-failure
rule.

Report emission order for future adapter wiring:

1. Emit `model_worker_v1` from the current ledger projection. Use future worker
   events for output fingerprint, noise score, and primary-12B blocking when
   they exist.
2. Emit `root_adapter_attribution_report_v1` from backend 8686 health,
   prompt-gate, final JSON, runtime model/tokens, and business-cycle verdict
   evidence.
3. Emit `ledger_gate_report_v1` from normalized ledger hygiene, runtime,
   stream, and report-gate evidence.
4. Emit `model_worker_gate_report_v1` and
   `model_pool_development_attribution_report_v1`.
5. Emit `model_pool_budget_fairness_report_v1`.
6. Emit `model_pool_development_window_report_v1`.
7. Emit `apple_silicon_baseline_comparison_report_v1`.
8. Emit `apple_silicon_development_effect_report_v1`.
9. Emit `context_rot_report_v1`, `context_rot_trend_report_v1`, and
   `context_rot_remediation_report_v1`.
10. Emit `steam_case_matrix_report_v1`.
11. Emit `validation_command_coverage_report_v1`.
12. Emit `self_evolution_continuity_report_v1`,
   `self_evolution_regression_report_v1`, and `rollback_resume_report_v1`.
13. Emit `readiness_next_round_v1` after Apple Silicon effect, Context Rot
   snapshot/trend/remediation, Steam case matrix, validation command coverage,
   rollback resume, and root adapter attribution reports.
14. Emit `rollback_report_v1` after readiness, validation command coverage,
   and root adapter attribution reports.
15. Emit `adapter_closure_report_v1` after ledger gate, validation command
   coverage, readiness, and rollback reports. This is still pure eval data:
   no JSONL reads, HTTP/SSE calls, process spawning, or model calls.
16. Emit `adapter_report_emission_report_v1`.
17. Emit `adapter_future_event_coverage_report_v1`.
18. Emit `report_bundle_gate_report_v1`.
19. Emit `adapter_promotion_window_report_v1` after readiness and the report
   bundle gate.
20. Emit `self_evolution_unattended_prerequisites_report_v1` after continuity,
   regression, readiness, Context Rot trend/remediation, rollback resume, Steam
   case matrix, validation command coverage, Apple Silicon development-effect,
   and promotion-window evidence. Do not feed this aggregate back into
   promotion; promotion is one of its inputs.

Do not let the emission step itself stop `tools/evolution-loop`; it is still
report production. Enforced blocking only starts after promotion, handoff, and
current-runner compatibility gates consume the report-only evidence and their
own tests pass.

Use `adapter_report_emission_report_v1` to compare the planned order with the
adapter's observed report order. It should include the observed future-event
names too, so missing worker fingerprints, noise scores, primary-12B blocking
events, backend 8686 health, prompt-gate state, final JSON, runtime
model/tokens, business-cycle verdict evidence, Steam case ids/endpoints/kinds, or
validation command phases/lines/status codes/output tails are visible before promotion.
It should also include planned and observed report fields for
`apple_silicon_development_effect_report_v1` and
`model_pool_budget_fairness_report_v1`, plus
`adapter_emission.missing_report_fields`; enforced promotion must block if the
effect report omits `apple_silicon_effect.feedback_applied`,
`apple_silicon_effect.operational_readiness_failure_kind`,
`apple_silicon_effect.quality_failure_blocked_by_readiness_order`, or
`apple_silicon_effect.operational_failure_counted_as_quality`, or if the budget
report omits `model_pool_budget.missing_required_roles`,
`model_pool_budget.dominant_runtime_token_roles`, or
`model_pool_budget.runtime_token_share_by_role`. The same coverage check must
include Context Rot report fields such as `context_rot.noisy_records`,
`context_rot.max_noise_penalty`, `context_rot.duplicate_outputs`,
`context_rot_trend.latest_noisy_records`,
`context_rot_trend.remediation_improved_noise`,
`context_rot_remediation.quarantine_candidates`,
`context_rot_remediation.clean_gists_backfilled`, and
`context_rot_remediation.allow_experiment_rollout`.
Treat this as adapter report field coverage. Missing worker/effect fields block
promotion, handoff, and current-runner compatibility in enforced rollout.
Missing Context Rot trend/remediation fields block those same coverage gates,
but neither case may be counted as `model_quality_failure`. If 8686 is down or
prompt-gate blocks the chain, keep the root cause as `model_unavailable` or
`chain_not_ready`.
For root adapter wiring, derive the aggregate booleans and promotion counts
from `AdapterReportEmissionReport::field_coverage_passed()` after the emission
report is built, instead of recomputing `missing_report_fields` in runner code.
Use `adapter_future_event_coverage_report_v1` immediately after the emission
report to compare contract-required future events against the planned emission
events. Missing worker fingerprints/noise/blocking events or paired baseline
events such as `baseline_12b_success` and `baseline_12b_validation_passed`,
root attribution events, Context Rot audit/trend/remediation events, Steam case
ids/endpoints/kinds, or validation command phases/lines/status codes/output tails remain report-only
observations until enforcement, then block promotion before the report bundle
can claim completeness.
The unattended-prerequisites aggregate is the post-promotion claim surface for
unattended self-evolution, not a reason to reclassify `chain_not_ready` or
`model_unavailable` as model quality.

## Steam Round Acceptance

Use `norion_test::SteamRoundAcceptancePlan` to describe a single
`/v1/business-cycle-stream` case plus its validation and readiness checks. After
the runner has collected evidence, evaluate it with
`norion_eval::SteamRoundAcceptanceGate::strict()`.

Required evidence:

- `StreamContinuityCheck`: must see `done`, `final`, no stream error, and no
  incomplete buffered frame.
- `ValidationObservation`: the planned validation command must be checked and
  pass.
- `LedgerRecord`: runtime response and validation fields must satisfy the
  strict gates.
- `SelfEvolutionReadinessSnapshot`: next-round readiness must allow scheduling
  before the runner adds another unattended round.

This keeps SSE correctness, runtime response quality, validation, Context Rot,
model-pool readiness, and rollback policy in one report-gate decision without
moving HTTP, SSE parsing, or command execution into the crates.

Stable report fields for `steam_round_report_v1`:

- `steam_round.case_id`
- `steam_round.stream_passed`
- `steam_round.validation_checked`
- `steam_round.validation_passed`
- `steam_round.runtime_tokens`
- `steam_round.runtime_model`
- `steam_round.acceptance_blocked`
- `steam_round.failure_reasons`
- `steam_round.can_schedule_next_round`

During report-only rollout, emit observation fields such as stream, validation,
runtime tokens, and runtime model. During enforced rollout, emit the full
`SteamRoundAcceptanceReport::from_evidence` projection and treat
`steam_round.acceptance_blocked=true` as a stop signal.

When `validation_command_coverage_report_v1` blocks, emit
`validation_command.coverage_failure_kind=validation_command_coverage` and keep
`validation_command.model_quality_failure_counted=false`. A failed or missing
Rust validation command can stop readiness, but it must not rewrite
`root_adapter.failure_kind` to `model_quality_failure`.
If review/helper output or a validation command requests strict coverage
(`--strict-coverage`, coverage enforcement, or 100 percent line coverage),
normalize the already observed coverage tooling/report evidence into
`validation_command.coverage_tooling_evidence` or
`validation_command.coverage_report_evidence` before allowing the enforced gate
to pass. Do not add coverage tool execution to `norion-eval`; missing tooling or
report evidence is a validation-command coverage block, not a model-quality
failure and not a reason to edit `tools/**` from this boundary.

## Report Fields And Worker Event Shape

When the runner reaches report-only mode, add these stable report fields:

- `model_pool.workers`
- `model_pool.success_rate`
- `model_pool.validation_pass_rate`
- `model_pool.latency_ms_total`
- `model_pool.runtime_tokens_total`
- `model_pool.feedback_applied_total`
- `root_adapter.failure_kind`

Only after enforcement is allowed should these fields participate in blocking
decisions:

- `model_pool.duplicate_outputs`
- `model_pool.noisy_outputs`
- `model_pool.primary_12b_blockers`
- `model_pool.development.feedback_delta`
- `model_pool.development.latency_multiplier`
- `model_pool.development.token_multiplier`
- `promotion.worker_operational_readiness_failure_runs`
- `promotion.worker_model_quality_failure_runs`
- `promotion.worker_claim_blocker_runs`

Future parallel worker events should use event name `model_worker_v1`. Required
projection fields are `worker_id`, `role`, `model`, `latency_ms`,
`runtime_tokens`, `success`, `feedback_applied`, `validation_checked`, and
`validation_passed`. Enforced mode also requires `duplicate_output`,
`noisy_output`, `blocked_primary_12b`, and `failure_kind`; the adapter emission
future events for those enforcement fields are `worker_output_fingerprint`,
`worker_noise_score`, `worker_primary_wait_ms`, and `worker_failure_kind`.

The corresponding per-worker gate report is `model_worker_gate_report_v1`.
Report-only wiring should emit the worker-indexed arrays for ids, roles, models,
latency, runtime tokens, success, feedback, and validation. Enforced wiring may
then use duplicate output, noisy output, primary-12B blocking, and failure kind
to decide whether the Apple Silicon pool really improved development.

Also emit `worker_root_failure_consistency_report_v1` during report-only
wiring for the current single-worker projection. It compares
`worker_failure_kind` with `root_adapter.failure_kind`; enforced wiring may only
proceed when the legacy single worker agrees with root attribution and no
operational readiness failure has been mixed into `model_quality_failure`.

The adapter promotion window carries the same worker-level split. Count
`worker_failure_kind=chain_not_ready` or `worker_failure_kind=model_unavailable`
under `promotion.worker_operational_readiness_failure_runs`; these block
enforcement as readiness or availability issues only. Count
`worker_failure_kind=model_quality_failure` under
`promotion.worker_model_quality_failure_runs`; this blocks only the quality
stability part of the promotion window. Count any worker with a non-empty
claim-blocker list under `promotion.worker_claim_blocker_runs`; enforced
promotion requires this to remain zero before the pool can affect development
claims.

Do not use the report to claim improvement when the root adapter is not ready.
If prompt-gate blocked the round, including the case where backend 8686 was also
down, set `root_adapter.failure_kind=chain_not_ready` and keep
`model_worker.model_quality_failure_counted=false`. If prompt-gate passed but
8686 was unavailable, set `root_adapter.failure_kind=model_unavailable` and
again keep `model_worker.model_quality_failure_counted=false`. Only final JSON
plus runtime model/tokens plus a failed business-cycle verdict can become
`model_quality_failure`.

Before expanding the Apple Silicon pool beyond advisory use, add
`model_pool_budget_fairness_report_v1`. The future adapter should group worker
records by role and report:

- role names;
- worker count and successful worker count per role;
- feedback applied per role;
- runtime tokens, runtime-token share, and latency per role;
- missing required roles;
- dominant runtime-token roles;
- total pool runtime tokens and latency;
- max role runtime-token share;
- whether budget/fairness blocked pool expansion.

The default gate should require planner, reviewer, and tester roles to each
contribute at least one successful feedback-bearing worker. No role should use
more than 60% of pool runtime tokens, and no helper role may block the primary
12B path. Treat failures here as pool-composition or budget failures; they are
not root model-quality failures.

In report-only rollout, use `missing_required_roles` and
`dominant_runtime_token_roles` as diagnostics only. In enforced rollout, either
field being non-empty should explain why `allow_pool_expansion=false`; the root
adapter should still classify 8686 or prompt-gate failures through
`chain_not_ready` / `model_unavailable`, not through this budget report.

Current `tools/evolution-loop` report-only wiring can consume a standalone
worker artifact with:

```powershell
cargo run --manifest-path .\tools\evolution-loop\Cargo.toml -- --report --ledger target\evolution\evolution-ledger.jsonl --pool-budget-fairness-json target\evolution\model-pool-budget-fairness.json --report-json target\evolution\report.json
```

The artifact may expose worker rows under `workers`, `model_workers`, or
`model_worker_v1`. Each row should include `role`, `success`,
`feedback_applied`, `runtime_tokens`, `latency_ms`, and
`blocked_primary_12b`. The report writes the additive top-level
`model_pool_budget_fairness_report_v1`; missing artifacts render as `null` so
legacy ledgers remain compatible. For the current Apple Silicon pool names,
`summary` satisfies the planner slot, `review` satisfies reviewer, and
`test-gate` satisfies tester.

Rollout sequence:

1. Shadow projection: build `ModelWorkerRecord` values from current ledger rows
   inside tests or report-only code, but do not change runner behavior.
2. Report-only summary: print or write an additive `ModelPoolSummary`; old
   report fields remain unchanged.
3. Optional worker events: allow new parallel worker metadata, but treat missing
   events as unknown rather than failure for legacy ledgers.
4. Advisory gate: evaluate `ModelPoolGate` and log failures without stopping
   rounds.
5. Enforcing gate: allow `ModelPoolGate` to block only after clean fixtures and
   all handoff tests pass.

Test gates before the enforcing stage:

- `cargo test --manifest-path crates/norion-test/Cargo.toml`
- `cargo test --manifest-path crates/norion-eval/Cargo.toml`
- `cargo test --manifest-path tools/evolution-loop/Cargo.toml`
- `cargo test --workspace`

The current crate-only phase should keep running the first two commands. The
last two become mandatory when the adapter is actually wired into
`tools/evolution-loop`.

Use `norion_eval::AdapterHandoffChecklist` as the final pre-wiring checklist.
For report-only rollout, crate tests are enough to keep the contract moving
without blocking the current runner. For enforced rollout, the checklist must
also prove:

- `tools/evolution-loop` tests pass;
- `cargo test --workspace` passes;
- legacy ledger replay was checked;
- report-only adapter output was observed before enforcement.
- the report bundle gate confirmed the enforced bundle is complete.
- the adapter report emission gate passed before enforcement.
- the adapter future-event coverage gate passed before enforcement.
- the Apple Silicon development-effect aggregate gate passed.
- the feedback/self-improve gate passed.
- the self-evolution continuity and regression gates passed.
- the next-round readiness gate passed.
- the self-evolution unattended prerequisites passed.
- the Context Rot trend and remediation gates passed.
- the rollback resume gate passed.
- the Steam case matrix gate passed.
- the validation command coverage gate passed.
- the adapter promotion window passed before enforcement.

The required schemas for the handoff are named by
`norion_test::EvalSchemaManifestPlan` and
`norion_eval::EvalSchemaManifest::evolution_loop_handoff_v1()`:

- `model_worker_v1`
- `model_worker_gate_report_v1`
- `worker_root_failure_consistency_report_v1`
- `model_pool_budget_fairness_report_v1`
- `model_pool_development_attribution_report_v1`
- `model_pool_development_window_report_v1`
- `apple_silicon_baseline_comparison_report_v1`
- `apple_silicon_development_effect_report_v1`
- `adapter_report_emission_report_v1`
- `adapter_future_event_coverage_report_v1`
- `ledger_gate_report_v1`
- `context_rot_report_v1`
- `context_rot_trend_report_v1`
- `context_rot_remediation_report_v1`
- `experiment_rollout_report_v1`
- `experiment_expansion_safety_report_v1`
- `experiment_switch_matrix_report_v1`
- `root_adapter_attribution_report_v1`
- `legacy_ledger_replay_report_v1`
- `feedback_self_improve_report_v1`
- `self_evolution_continuity_report_v1`
- `self_evolution_regression_report_v1`
- `self_evolution_unattended_prerequisites_report_v1`
- `readiness_next_round_v1`
- `steam_round_report_v1`
- `rollback_report_v1`
- `adapter_closure_report_v1`
- `rollback_drill_matrix_report_v1`
- `adapter_handoff_report_v1`
- `report_bundle_gate_report_v1`
- `adapter_promotion_window_report_v1`
- `rollback_resume_report_v1`

Use `norion_test::EvalReportBundlePlan` and
`norion_eval::EvalReportBundleManifest::for_stage` to choose the report bundle
for a rollout stage:

- `ShadowOnly`: emit no new eval report bundle.
- `ReportOnly`: emit `model_worker_v1`,
  `worker_root_failure_consistency_report_v1`,
  `apple_silicon_baseline_comparison_report_v1`,
  `experiment_switch_matrix_report_v1`,
  `rollback_drill_matrix_report_v1`, `context_rot_report_v1`, and
  `adapter_promotion_window_report_v1`.
- `Enforced`: emit the full schema set listed above.

Use `norion_eval::EvalReportBundleGateReport::from_manifest_and_evidence` to
compare expected schemas with the adapter output before promotion. The stable
schema is `report_bundle_gate_report_v1`.
Derive field coverage from
`AdapterReportEmissionReport::field_coverage_passed()`.
Use
`norion_eval::AdapterReportBundleGateBoundaryContract::report_bundle_gate_v1`
and `norion_test::AdapterReportBundleGateBoundaryPlan` before wiring this gate:
the runner may pass already-observed schema names and an adapter report-field
coverage bit, while eval owns `EvalReportBundleManifest`,
`EvalReportBundleEvidence`, `EvalReportBundleManifest::evaluate_bundle`, and
`EvalReportBundleGateReport`. Report directory scans, JSONL/file IO, HTTP/SSE,
process spawning, validation command spawning, model calls, and runner state
stay outside this boundary. Use `stays_pure_data_boundary()` on both sides so
the bundle gate remains a pure schema-name/report-field projection.

Report-only fields:

- `report_bundle.stage`
- `report_bundle.expected_schema_names`
- `report_bundle.observed_schema_names`
- `report_bundle.missing_schema_names`
- `report_bundle.adapter_report_field_coverage_passed`

Enforced fields:

- `report_bundle.bundle_blocked`
- `report_bundle.failure_reasons`
- `report_bundle.complete`

This gate answers a narrower question than legacy replay: did the current
adapter output include every report required for its rollout stage? Legacy JSONL
files may miss additive reports; a report bundle from a wired adapter may not.
During enforced rollout, all schema names being observed is not enough:
`report_bundle.complete` must remain false while
`adapter_emission.missing_report_fields` is non-empty.

Use `norion_eval::AdapterPromotionWindowReport::from_gate_and_evidence` before
moving from report-only observation to enforced wiring. The stable schema is
`adapter_promotion_window_report_v1`.
Use
`norion_eval::AdapterPromotionWindowBoundaryContract::promotion_window_v1` and
`norion_test::AdapterPromotionWindowBoundaryPlan` before wiring this gate: the
runner may pass report-only observation counts, gate-passed run counts, root
adapter failure counts, and already-computed adapter report-field coverage
results, while eval owns `AdapterPromotionWindowEvidence`,
`AdapterPromotionWindowGate`, and `AdapterPromotionWindowReport`. The boundary
must not read JSONL/files, scan report directories, call HTTP/SSE, spawn
processes or validation commands, call a model, execute promotion/enforcement
actions, or inspect runner state. `promotion.allow_enforcement=true` is a data
result for the thin adapter to consume after its own wiring checks; it is not an
eval-side runner mutation.

Report-only fields:

- `promotion.stage`
- `promotion.observed_report_only_runs`
- `promotion.complete_bundle_runs`
- `promotion.adapter_report_emission_passed_runs`
- `promotion.adapter_report_field_coverage_passed_runs`
- `promotion.adapter_future_event_coverage_passed_runs`
- `promotion.apple_silicon_development_effect_passed_runs`
- `promotion.apple_silicon_baseline_comparison_passed_runs`
- `promotion.experiment_switch_matrix_passed_runs`
- `promotion.readiness_passed_runs`
- `promotion.context_rot_trend_passed_runs`
- `promotion.context_rot_remediation_passed_runs`
- `promotion.rollback_resume_passed_runs`
- `promotion.steam_case_matrix_passed_runs`
- `promotion.validation_command_coverage_passed_runs`
- `promotion.model_quality_failure_runs`
- `promotion.model_unavailable_runs`
- `promotion.chain_not_ready_runs`
- `promotion.runtime_response_failure_runs`
- `promotion.stream_or_final_missing_runs`
- `promotion.min_report_only_runs`
- `promotion.min_complete_bundle_runs`
- `promotion.min_adapter_report_emission_passed_runs`
- `promotion.min_adapter_report_field_coverage_passed_runs`
- `promotion.min_adapter_future_event_coverage_passed_runs`
- `promotion.min_apple_silicon_development_effect_passed_runs`
- `promotion.min_apple_silicon_baseline_comparison_passed_runs`
- `promotion.min_experiment_switch_matrix_passed_runs`
- `promotion.min_readiness_passed_runs`
- `promotion.min_context_rot_trend_passed_runs`
- `promotion.min_context_rot_remediation_passed_runs`
- `promotion.min_rollback_resume_passed_runs`
- `promotion.min_steam_case_matrix_passed_runs`
- `promotion.min_validation_command_coverage_passed_runs`

Enforced fields:

- `promotion.promotion_blocked`
- `promotion.failure_reasons`
- `promotion.allow_enforcement`

The enforced promotion gate should require at least three report-only
observations, three complete report bundles, three adapter report-emission
passes, three adapter report-field coverage passes, three adapter future-event
coverage passes, three Apple Silicon
development-effect passes, three Apple Silicon paired baseline comparison
passes, three experiment switch matrix passes, three readiness passes, three
Context Rot trend passes, three Context Rot remediation passes, three rollback
resume passes, three Steam case matrix passes, and three validation command
coverage passes.
`chain_not_ready` and `model_unavailable` block promotion as readiness or
availability problems, not as model quality failures.

Before moving from report-only observation to enforced wiring, project
`norion_eval::AdapterHandoffReport::from_checklist_and_evidence`. The stable
schema is `adapter_handoff_report_v1`.
Use `norion_eval::AdapterHandoffBoundaryContract::handoff_v1` and
`norion_test::AdapterHandoffBoundaryPlan` before wiring this report: eval may
build `AdapterHandoffChecklist`, carry `AdapterTestGate` command text, evaluate
`AdapterHandoffEvidence`, and emit `AdapterHandoffReport`, but it must not
execute cargo/workspace/evolution-loop commands, read JSONL/files, call
HTTP/SSE, call a model, switch the runner, mutate runner state, or call a remote
Mac. The command strings in the report are evidence requirements for the thin
adapter and operator; they are not eval-owned execution.

When upstream operational reports are already available, use
`AdapterHandoffEvidence::with_operational_gate_reports` to lift only their
stable allow/pass bits into handoff evidence:

- `ContextRotTrendReport::allow_unattended_continuation`;
- `ContextRotRemediationReport::allow_experiment_rollout`;
- `RollbackResumeReport::allow_unattended_rounds`;
- `SteamCaseCoverageReport::allow_enforced_adapter`;
- `ValidationCommandCoverageReport::allow_next_round`.

This helper does not execute validation commands, run Steam cases, perform
rollback/resume actions, scan JSONL, or inspect runner state.

When bundle, drift, emission, or future-event coverage reports are already
available, use the thinner report-lift helpers instead of copying booleans by
hand:

- `AdapterHandoffEvidence::with_report_bundle_gate_report`
- `AdapterHandoffEvidence::with_schema_drift_report`
- `AdapterHandoffEvidence::with_adapter_report_emission_report`
- `AdapterHandoffEvidence::with_adapter_report_field_coverage_from_report`
- `AdapterHandoffEvidence::with_adapter_future_event_coverage_report`

These helpers only lift stable allow/pass bits from already-computed reports.
They do not re-run schema drift, report emission planning, or future-event
coverage collection.
`AdviceContinuationReport` is intentionally not a direct handoff input here; it
must stay on the additive/readiness path and remain outside
`AdapterHandoffChecklist.required_schemas`.

Report-only fields:

- `handoff.stage`
- `handoff.required_schemas`
- `handoff.test_gate_names`
- `handoff.test_gate_commands`
- `handoff.norion_test_passed`
- `handoff.norion_eval_passed`
- `handoff.evolution_loop_passed`
- `handoff.workspace_passed`
- `handoff.legacy_replay_checked`
- `handoff.report_only_observed`
- `handoff.report_bundle_complete`
- `handoff.schema_drift_passed`
- `handoff.adapter_report_emission_passed`
- `handoff.adapter_report_field_coverage_passed`
- `handoff.adapter_future_event_coverage_passed`
- `handoff.model_pool_development_window_passed`
- `handoff.apple_silicon_development_effect_passed`
- `handoff.feedback_self_improve_passed`
- `handoff.self_evolution_continuity_passed`
- `handoff.self_evolution_regression_passed`
- `handoff.readiness_next_round_passed`
- `handoff.self_evolution_unattended_prerequisites_passed`
- `handoff.context_rot_trend_passed`
- `handoff.context_rot_remediation_passed`
- `handoff.rollback_resume_passed`
- `handoff.steam_case_matrix_passed`
- `handoff.validation_command_coverage_passed`
- `handoff.promotion_window_passed`

Enforced fields:

- `handoff.may_block_current_runner`
- `handoff.handoff_blocked`
- `handoff.failure_reasons`
- `handoff.allow_runner_wiring`

The handoff report may only allow enforced runner wiring when the current
runner and workspace tests are green, legacy ledger replay was checked, and at
least one report-only adapter output was observed, and the report bundle gate
reported a complete enforced bundle, schema drift passed, adapter report
emission passed, adapter report-field coverage passed, adapter future-event
coverage passed, feedback/self-improve passed, self-evolution
continuity/regression passed, next-round readiness passed, self-evolution
unattended prerequisites passed, Context Rot trend/remediation passed,
rollback resume passed, and the adapter promotion window passed.
Until then, the adapter can produce reports, but it must not block the current
runner.

## Report Taxonomy Boundary

When wiring the future `tools/evolution-loop` adapter, keep these report
categories separate:

| Category | Adapter rule | Examples |
| --- | --- | --- |
| Additive and excluded | Preserve as observability or advisory readiness input. Do not add it to the enforced report bundle, handoff required schemas, or direct current-runner lift surface unless the contract changes first. | `advice_continuation_report_v1`; missing future reports listed by legacy replay as additive gaps |
| Report-only but required | Emit during report-only rollout, then require the matching pass bit, schema, or field coverage before enforced wiring can claim the bundle is complete. | `model_worker_v1`, `worker_root_failure_consistency_report_v1`, `context_rot_report_v1`, `adapter_future_event_coverage_report_v1`, `adapter_promotion_window_report_v1`, `ledger_gate_report_v1`, `schema_drift_report_v1`, `report_bundle_gate_report_v1`, `readiness_next_round_v1`, `rollback_resume_report_v1` |

`advice_continuation_report_v1` is the guardrail example: it may project into
readiness as advisory context, but it must not become an enforced bundle member
or direct handoff blocker. If continuation guidance affects scheduling, it
should arrive through the already documented readiness projection bits.

Keep the direct lift surface narrow when reviewing adapter patches. Handoff and
current-runner compatibility may copy pass bits from already-computed required
reports; they should not add a new direct `AdviceContinuationReport` input.
Likewise, a single report-only observation is not promotion. Required schemas
support enforced wiring only after the manifest, enforced bundle, schema drift,
adapter report emission, future-event coverage, promotion window, and handoff
checks all agree on the same coverage.

For report-only but required schemas, legacy JSONL replay may still show a
missing additive gap. A newly wired adapter is stricter: it must pass the
manifest, report bundle gate, schema drift gate, adapter report-field coverage,
promotion window, and handoff checklist before those reports can support
enforced runner wiring.

## Legacy Ledger Replay Compatibility

Use `norion_test::LegacyLedgerReplayPlan` and
`norion_eval::LegacyLedgerReplayCompatibility` when reading existing
`target\evolution\*.jsonl` artifacts.

Project the replay check with
`norion_eval::LegacyLedgerReplayReport::from_contract_evidence_and_decision`.
The report schema is `legacy_ledger_replay_report_v1`.

Replay must require the fields the current runner already writes:

- `round`
- `success`
- `runtime_tokens`
- `runtime_model`
- `validation_checked`
- `validation_passed`
- `feedback_applied`

Replay must not fail only because these future additive reports are absent:

- `model_worker_v1`
- `model_worker_gate_report_v1`
- `worker_root_failure_consistency_report_v1`
- `model_pool_budget_fairness_report_v1`
- `model_pool_development_attribution_report_v1`
- `model_pool_development_window_report_v1`
- `apple_silicon_baseline_comparison_report_v1`
- `apple_silicon_development_effect_report_v1`
- `adapter_report_emission_report_v1`
- `adapter_future_event_coverage_report_v1`
- `ledger_gate_report_v1`
- `context_rot_report_v1`
- `context_rot_trend_report_v1`
- `context_rot_remediation_report_v1`
- `experiment_rollout_report_v1`
- `experiment_kill_switch_report_v1`
- `experiment_expansion_safety_report_v1`
- `experiment_switch_matrix_report_v1`
- `root_adapter_attribution_report_v1`
- `adapter_fixture_contract_report_v1`
- `current_runner_compatibility_report_v1`
- `feedback_self_improve_report_v1`
- `self_evolution_continuity_report_v1`
- `self_evolution_regression_report_v1`
- `self_evolution_unattended_prerequisites_report_v1`
- `readiness_next_round_v1`
- `steam_round_report_v1`
- `steam_case_matrix_report_v1`
- `validation_command_coverage_report_v1`
- `rollback_report_v1`
- `adapter_closure_report_v1`
- `rollback_drill_matrix_report_v1`
- `adapter_handoff_report_v1`
- `report_bundle_gate_report_v1`
- `schema_drift_report_v1`
- `adapter_promotion_window_report_v1`
- `rollback_resume_report_v1`

This rule is only for old ledger replay. A future enforced run can still require
new worker, ledger gate, Context Rot, feedback/self-improve, readiness, Steam
round/case, validation-command, rollback, rollback-resume, handoff, bundle,
schema drift, adapter report-field coverage, and promotion-window evidence before
it schedules more unattended rounds. Keeping these paths separate preserves
today's `tools/evolution-loop` output while making the new eval surface testable.

Stable report fields for `legacy_ledger_replay_report_v1`:

- `legacy_replay.stage`
- `legacy_replay.ledger_rows_present`
- `legacy_replay.base_ledger_fields_present`
- `legacy_replay.required_existing_fields`
- `legacy_replay.optional_additive_reports`
- `legacy_replay.missing_additive_reports`
- `legacy_replay.compatibility_blocked`
- `legacy_replay.failure_reasons`

The replay report itself is emitted by the check; old JSONL files do not need to
contain `legacy_ledger_replay_report_v1`.

## Adapter Acceptance Checklist

Use `norion_test::AdapterAcceptancePlan` for the plan side and
`norion_eval::RootAdapterAcceptanceContract` for the eval side before changing
the runner.

Stage requirements:

- `ShadowOnly`: compute root and worker projections only in tests or local logs.
  Do not print new report fields, persist new worker events, or block the
  current runner.
- `ReportOnly`: emit only additive report fields such as
  `root_adapter.failure_kind` and model-pool totals. Worker events only need
  projection fields. Outage attribution is required before any failure label is
  shown.
- `Enforced`: allow blocking only after worker enforcement fields are present,
  strong root mappings are covered, attribution tests pass, and the handoff test
  gates are green.

Apple Silicon development-effect enforcement requires both per-worker rows and
aggregate counters. The report must include worker ids, roles, latency,
runtime tokens, success, feedback applied, validation checked/passed, duplicate
output, noisy output, primary-12B blocking, failure kind, worker claim allowance,
and worker claim blockers. It must also include operational readiness,
`chain_not_ready`, `model_unavailable`, model-quality failure, reported quality
failure, duplicate/noisy output, primary-12B blocker, worker-metric coverage,
validation unchecked/failed, successful-worker-missing-runtime, and final claim
counters. Missing any of these fields is adapter report coverage failure; it is
not evidence that any model produced a lower-quality answer.

Root mapping rule:

- Strong `$.business_cycle.*` fields may enforce in the final stage.
- Weak `$.generate.*` fields remain runtime/projection evidence and cannot be
  the direct cause of a model quality failure.

Outage attribution rule:

- `prompt_gate_blocked=true` wins over all other evidence and reports
  `chain_not_ready`.
- If the prompt gate did not block but 8686 is unavailable, report
  `model_unavailable`.
- `chain_not_ready` and `model_unavailable` block Apple Silicon effect claims as
  readiness or availability failures; they must leave model-quality failure
  counters at zero.
- Only report `model_quality_failure` when final JSON exists, runtime model and
  token evidence exist, and `business_cycle.passed=false`.

This checklist is deliberately stricter than report-only visibility. It lets the
main window add observability first, then enforce only after old ledgers,
current runner tests, and workspace tests remain compatible.

## Root Adapter Rollback Actions

When the future adapter produces `root_adapter.failure_kind`, feed that kind to
`rollback_plan_for_root_adapter_failure` before deciding whether the failure is
operational or model-quality related.

Stable actions:

- `chain_not_ready`: pause root adapter enforcement, preserve prompt-gate
  evidence, and resume only after the chain readiness gate passes.
- `model_unavailable`: pause model-pool enforcement, preserve backend 8686
  health evidence, and resume only after runtime backend health passes.
- `stream_or_final_missing`: stop scheduling rounds, preserve the stream
  transcript, and re-run stream continuity validation.
- `runtime_response_missing`: stop scheduling rounds, preserve runtime response
  metadata, and re-run the runtime response gate.
- `model_quality_failure`: stop scheduling rounds, preserve final JSON and
  validation evidence, and route the issue to model-quality review.
- `unknown`: pause enforcement, preserve raw adapter evidence, and classify the
  failure before resuming.

The first two actions are recovery paths, not model quality failures. This is
the guard that prevents 8686 outages or prompt-gate blocks from making Apple
Silicon model-pool quality metrics look worse than they are.

## Self-Improve Proposal Business Acceptance

When wiring self-improve proposal reports, keep
`self_improve_proposal_acceptance_v1` as the compatibility surface for safe
proposal handling and use
`self_improve_proposal.evidence_backed_business_improvement` for accepted
business improvements. The runner or adapter may treat a clean quarantined
candidate as advisory progress, but it must not count it as an accepted
unattended-evolution change.

The accepted-business-improvement projection is pure data. It requires a source
round, safe evidence ids, checked and passed validation evidence, a safe
validation command source, a clean gist, no runtime side effects, and an
accepted memory-admission decision with reasons. Do not rebuild this from helper
prose, live model calls, JSONL scanning, SSH state, daemon state, or command
execution.

Current `tools/evolution-loop` report JSON emits this projection inside each
`self_improve_proposal_artifact_v1.proposals[].business_improvement_acceptance`
object. The artifact loader may parse ledger text to find candidate proposals,
but the acceptance projection itself must remain report-only and must not run
validation or apply the proposal.

The same report JSON also emits
`self_improve_proposal_acceptance_summary_v1`. This additive summary is the
fast path for dashboards, gates, and operator status: use
`evidence_backed_business_improvement_count` for real accepted improvements,
`advisory_only_count` for clean suggestions, and `require_repair_count` /
`accepted_without_business_evidence_count` for accepted-looking proposals that
need validation repair. Do not infer these counts from helper prose.

`status-evolution-loop.ps1` should prefer the top-level
`self_improve_proposal_acceptance_summary_v1` when it exists. For older daemon
reports that only contain
`self_improve_proposal_artifact_v1.proposals[].business_improvement_acceptance`,
the status script may derive the same counts from those nested pure-data
reports. This keeps operator status useful before the daemon is restarted with
the newest report writer, while still avoiding helper prose parsing or proposal
application.

The runner prompt context also consumes the same ledger-backed artifact summary.
When proposal candidates exist, `prompt_context()` appends a compact
`self_improve_proposal_acceptance=source:ledger_artifact ...` line with
candidate, projected, evidence-backed business, advisory-only, repair-required,
and accepted-without-business-evidence counts. If the current window has only
advisory proposals and no evidence-backed business improvement, the prompt adds
`next_self_improve_should_convert_advisory_to_evidence_backed_business_improvement:true`.
If repair-required or accepted-without-business-evidence proposals exist, it
adds `next_self_improve_should_repair_unvalidated_or_unaccepted_proposals:true`.
This is still prompt guidance only: it must not apply proposals, rerun
validation, mutate memory, write `.ndkv`, start streams, call models, or control
the daemon.

Keep that prompt guidance decision in
`norion_eval::SelfImproveProposalPromptGuidance`. The runner may parse ledger
text into a `SelfImproveProposalAcceptanceSummaryReport` and format prompt
lines, but the boolean guidance fields should come from eval:
`should_convert_advisory_to_evidence_backed_business_improvement`,
`should_repair_unvalidated_or_unaccepted_proposals`, and
`requires_checked_passed_validation_and_accepted_memory_admission`.
The report JSON mirrors those same booleans under
`self_improve_proposal_acceptance_summary_v1.prompt_guidance` so status surfaces
and dashboards can display the same next-step guidance without parsing prompt
text.
The same summary also emits
`self_improve_proposal_acceptance_summary_v1.action_plan`, produced from
`norion_eval::SelfImproveProposalActionPlan`. That report-only plan exposes
`action_required`, `primary_action`, ordered `actions`, and
`requires_checked_passed_validation_and_accepted_memory_admission` with
`auto_apply=false` and read-only side effects. Consumers may display or route
the plan, but they must not treat it as permission to apply proposals, rerun
validation, write memory, or start model calls.

When a concrete follow-up target is needed, use
`norion_eval::SelfImproveProposalActionAssignment` and its
`first_target_digest()` helper. The digest is the stable
`self_improve_proposal_action_assignment_v1` surface for prompt/status/Forge
consumers: first target id, source round, evidence ids, current memory
admission decision, validation checked/passed, memory admission accepted,
evidence-backed business-improvement, advisory/repair flags, and missing
requirements. The runner may format those fields into compact prompt text, but
must not rebuild them from helper prose, raw old-window payloads, live model
calls, command execution, daemon state, or memory writes.
Keep `norion_test::SelfImproveProposalActionAssignmentPlan` aligned with
`self_improve_proposal_action_assignment_v1`; it is the plan-level acceptance
source for the schema, entrypoints, report fields, and forbidden side effects.
`status-evolution-loop.ps1` should prefer those JSON guidance fields when
available and may derive the same booleans from legacy nested
`business_improvement_acceptance` counts while a daemon report is still using an
older writer. The fallback is display-only and must not apply proposals or
change daemon scheduling.

## Rollback Report

Plan rollback reporting with `norion_test::RollbackReportPlan` and project
results with `norion_eval::RollbackReport::from_plan`. When
`readiness_next_round_v1` has already been emitted, project the rollback report
with `norion_eval::RollbackReport::from_readiness_report` instead of rebuilding
the snapshot. That path consumes only `readiness.rollback_required`,
`readiness.rollback_reason`, `readiness.rollback_actions`, and
`readiness.root_adapter_failure_kind`; it maps the failure code to a stable
resume gate and never reads JSONL/files, scans runner state, calls HTTP/SSE,
spawns validation commands, executes rollback actions, or calls a model.

Stable report fields for `rollback_report_v1`:

- `rollback.required`
- `rollback.reason`
- `rollback.actions`
- `rollback.root_adapter_failure_kind`
- `rollback.resume_gate`
- `rollback.stop_scheduling_new_rounds`

During report-only rollout, emit the reason and action fields as observability.
During enforced rollout, require a stable `rollback.resume_gate` and use
`rollback.stop_scheduling_new_rounds=true` as the scheduling stop signal for
generic validation/report-gate failures. Root adapter operational failures such
as `chain_not_ready` and `model_unavailable` should pause enforcement and point
to their operational resume gates instead of consuming model-quality budget.

## Rollback Drill Matrix Report

Plan rollback drill reporting with
`norion_test::RollbackDrillMatrixReportPlan` and project the root-adapter
policy matrix with `norion_eval::RollbackDrillMatrixReport`.
Use
`norion_eval::AdapterRollbackDrillMatrixBoundaryContract::rollback_drill_matrix_v1`
and `norion_test::AdapterRollbackDrillMatrixBoundaryPlan` before wiring this
into the runner: eval may derive `RollbackDrillCase` values from
`RootAdapterFailureKind`, build `RollbackDrillMatrixEvidence`, evaluate
`RollbackDrillMatrixGate`, and emit `RollbackDrillMatrixReport`, but it must not
execute rollback actions, execute resume actions, spawn validation commands,
call HTTP/SSE, call a model, or inspect runner state. The runner remains
responsible for any future action execution after this report has been accepted.
Use `stays_pure_data_boundary()` on both sides to keep the drill as policy
evidence, not rollback execution.

Stable report fields for `rollback_drill_matrix_report_v1`:

- `rollback_drill.stage`
- `rollback_drill.failure_kinds`
- `rollback_drill.rollback_reasons`
- `rollback_drill.resume_gates`
- `rollback_drill.covered_failure_kinds`
- `rollback_drill.missing_failure_kinds`
- `rollback_drill.unstable_rollback_reasons`
- `rollback_drill.unstable_resume_gates`
- `rollback_drill.empty_action_failure_kinds`
- `rollback_drill.none_requires_rollback`
- `rollback_drill.drill_blocked`
- `rollback_drill.failure_reasons`
- `rollback_drill.allow_enforced_rollback_policy`

During shadow and report-only rollout, emit the matrix as observability only.
During enforced rollout, require coverage for every `RootAdapterFailureKind`,
stable rollback reasons, stable resume gates, non-empty actions for required
rollbacks, and `none` remaining non-rollback. This is the last policy drill
before the future runner treats rollback actions as a scheduling stop signal.

If 8686 is unreachable while prompt-gate is already blocked, the drill must
preserve the root attribution as `chain_not_ready`; if prompt-gate passed and
8686 is unreachable, it is `model_unavailable`. Neither case is
`model_quality_failure`.

## Rollback Resume Report

Plan rollback-resume reporting with `norion_test::RollbackResumeReportPlan` and
project results with `norion_eval::RollbackResumeReport::from_evidence_for_stage`
or `from_gate_and_evidence` when a configured gate is already available.

Stable report fields for `rollback_resume_report_v1`:

- `rollback_resume.stage`
- `rollback_resume.resume_gate`
- `rollback_resume.chain_ready`
- `rollback_resume.backend_8686_reachable`
- `rollback_resume.stream_continuity_passed`
- `rollback_resume.runtime_response_passed`
- `rollback_resume.model_quality_review_passed`
- `rollback_resume.validation_command_passed`
- `rollback_resume.steam_case_matrix_passed`
- `rollback_resume.validation_command_coverage_passed`
- `rollback_resume.adapter_report_field_coverage_passed`
- `rollback_resume.manual_classification_done`
- `rollback_resume.resume_blocked`
- `rollback_resume.failure_reasons`
- `rollback_resume.allow_unattended_rounds`

During enforced rollout, do not resume unattended rounds until the specific
`rollback.resume_gate` has passed. `chain_readiness_gate` and
`runtime_backend_health_check` are operational recovery paths for prompt-gate
or backend 8686 failures; they must not be counted as model quality failures.
Only `model_quality_review` uses the quality-review path.
After the specific resume gate passes, still keep unattended rounds paused until
the Steam case matrix, validation command coverage, and adapter report field
coverage gates pass. Derive the field coverage bit from
`AdapterReportEmissionReport::field_coverage_passed()`.

## Next-Round Readiness

Before scheduling another unattended round, the future adapter should construct
`SelfEvolutionReadinessSnapshot` from already-computed gate decisions:

- `ReportGate::evaluate` for ledger hygiene and latest-round policy;
- `ContextRotAcceptanceContract::blocking_decision` for Context Rot;
- `ContextRotTrendGate::evaluate` and `ContextRotRemediationGate::evaluate`
  so a clean latest round cannot hide cross-round rot or incomplete cleanup;
- `ModelPoolGate::evaluate` and Apple Silicon development-effect results;
- `ExperimentRolloutGate::evaluate` for enabled feature flags;
- `ExperimentKillSwitchGate::evaluate` and
  `ExperimentExpansionSafetyGate::evaluate` before enabled experiments expand;
- adapter report-emission and Apple Silicon development-effect gate decisions
  as inputs to experiment expansion safety;
- `RollbackResumeGate::evaluate` before restarting unattended rounds after a
  rollback;
- `SteamCaseCoverageGate::evaluate` for the root business-cycle Steam matrix;
- `ValidationCommandCoverageGate::evaluate` for post-round command coverage;
- `AdviceContinuationReport` as advisory continuation evidence;
- `RootAdapterFailureKind` from root adapter attribution.

Plan the handoff with `norion_test::SelfEvolutionReadinessPlan`. Its required
gate inputs should remain the same list above, and its report schema name should
stay `readiness_next_round_v1`.

Use `can_schedule_next_round` as the final scheduling answer and
`next_round_decision` as the report reason list. If the snapshot blocks, surface
`rollback_plan` and stop adding unattended rounds until its resume gate passes.

In report-only stages, root adapter failure kinds remain visible but advisory.
In enforced stages, any root adapter failure other than `none` blocks scheduling
and uses the root-adapter-specific rollback actions above.

Stable report fields for `readiness_next_round_v1`:

- `readiness.stage`
- `readiness.root_adapter_failure_kind`
- `readiness.advice_continuation_observed`
- `readiness.advice_continuation_blocked`
- `readiness.context_rot_trend_blocked`
- `readiness.context_rot_remediation_blocked`
- `readiness.experiment_expansion_safety_blocked`
- `readiness.adapter_report_field_coverage_blocked`
- `readiness.rollback_resume_blocked`
- `readiness.steam_case_matrix_blocked`
- `readiness.validation_command_coverage_blocked`
- `readiness.can_schedule_next_round`
- `readiness.failure_reasons`
- `readiness.rollback_required`
- `readiness.rollback_reason`
- `readiness.rollback_actions`

During report-only rollout, emit only `readiness.stage` and
`readiness.root_adapter_failure_kind`. During enforced rollout, emit the full
projection from `SelfEvolutionReadinessReport::from_snapshot` and let
`readiness.can_schedule_next_round=false` be the final stop signal.
If advice continuation evidence exists, project it through
`SelfEvolutionReadinessSnapshot::with_advice_continuation_report` so
`readiness.advice_continuation_observed` and
`readiness.advice_continuation_blocked` remain visible as advisory context. Do
not let those fields become a new stop signal; the current contract keeps them
outside the enforced scheduling decision.
If `readiness.experiment_expansion_safety_blocked=true`, do not schedule the
next unattended round even when the latest Steam case itself passed; first fix
adapter report emission, Apple Silicon development-effect evidence,
kill-switch readiness, or expansion safety.
If `readiness.adapter_report_field_coverage_blocked=true`, do not schedule the
next unattended round until `AdapterReportEmissionReport::field_coverage_passed()`
is true and `adapter_emission.missing_report_fields` is empty.
If `readiness.rollback_resume_blocked=true`, keep unattended rounds paused
until the specific rollback resume gate reports `allow_unattended_rounds=true`.
If `readiness.steam_case_matrix_blocked=true`, keep the adapter in report-only
or stop scheduling until the Steam matrix has enough unique cases and required
final JSON fields. If `readiness.validation_command_coverage_blocked=true`,
keep unattended rounds paused until command lines, phases, status codes, output
tails, and Rust check evidence are present. Preserve the root adapter attribution
that caused the round to stop; validation coverage blocking is not a model
quality failure by itself.

## Adapter Closure Report

Use `norion_eval::AdapterEvidenceProjection::from_normalized_evidence` as the
thin adapter boundary from `tools/evolution-loop` into eval data. The runner may
normalize ledger rows, report-gate settings, helper-stage fields, and validation
command observations into `LedgerRecord`, `ReportGate`,
`HelperStageContractSummary`, `ValidationCommandCoverageEvidence`, and
`ValidationCommandCoverageGate`; after that, eval owns the ledger gate,
validation coverage report, readiness, rollback, and closure decisions. Mirror
this boundary with `norion_test::AdapterNormalizedEvidenceProjectionPlan`.
Neither side may add JSONL IO, HTTP/SSE, process spawning, validation-command
spawning, model calls, or runner state to the projection contract.
Use `stays_pure_data_boundary()` on
`AdapterNormalizedEvidenceProjectionContract` and
`AdapterNormalizedEvidenceProjectionPlan` to fixture-test that ledger hygiene,
report-gate evaluation, helper-stage evidence, and validation-command coverage
are mapped from normalized data only, never from runner IO or command
execution.
Use `norion_eval::AdapterReadinessReportsInputContract::readiness_reports_v1`
and `norion_test::AdapterReadinessReportsInputPlan` for optional readiness
inputs: the runner may pass already-computed `SteamRoundAcceptanceReport`,
`SteamCaseCoverageReport`, `ContextRotReport`, `ContextRotTrendReport`,
`ContextRotRemediationReport`, and `AdviceContinuationReport`, but it must not
execute Steam HTTP/SSE, Context Rot scans, validation commands, or model calls
inside this eval boundary.
When `AdviceContinuationReport` is present here, it remains advisory input for
`SelfEvolutionReadinessSnapshot`: project it through
`AdapterReadinessReports::with_advice_continuation`, keep
`readiness.advice_continuation_observed` and
`readiness.advice_continuation_blocked` visible, and do not promote it into a
new enforced stop signal for `readiness.can_schedule_next_round`.
Use `stays_pure_data_boundary()` on the contract and plan so tests can reject
any readiness input path that tries to pass a Steam client, filesystem scanner,
runner state, command executor, or model call into eval.
Use `norion_eval::AdapterContextRotTrendBoundaryContract::context_rot_trend_v1`
and `norion_test::AdapterContextRotTrendBoundaryPlan` for Context Rot trend
reports: the runner owns signal collection and any JSONL/file scanning, while
eval only accepts `ContextRotSignal`, `ContextRotTrendPoint`, and
`ContextRotTrendGate` to produce `ContextRotTrendWindowSummary`, `GateDecision`,
and `ContextRotTrendReport`. This boundary must not scan files, execute
cleanup, write clean gists, delete duplicate outputs, spawn validation commands,
call Steam HTTP/SSE, call a model, or read runner state. Use
`stays_pure_data_boundary()` on both sides before wiring trend reports into
readiness or closure aggregation.
Use
`norion_eval::AdapterContextRotRemediationBoundaryContract::context_rot_remediation_v1`
and `norion_test::AdapterContextRotRemediationBoundaryPlan` for remediation
reports: eval may turn `ContextRotSignal`, `ContextRotRemediationEvidence`, and
`ContextRotRemediationGate` into `ContextRotRemediationReport`, but quarantine
actions, clean-gist writes, duplicate-output deletion, filesystem scans, HTTP,
commands, and model calls stay outside this boundary.
Use `stays_pure_data_boundary()` on both sides as the adapter-facing fixture
helper: it must stay true before the runner may treat remediation evidence as
eval-owned.

Use `norion_eval::AdapterEvidenceProjection::closure_report_with_reports` as
the final eval-side sink after the runner has normalized ledger rows, helper
stage fields, validation command evidence, Steam reports, Context Rot reports,
readiness inputs, and root adapter attribution. This report is a pure data
closure over already-computed evidence; it must not read JSONL files, perform
HTTP/SSE calls, spawn validation commands, or call a model.
If the runner already has `ledger_gate_report_v1`,
`validation_command_coverage_report_v1`, and `readiness_next_round_v1`, use
`AdapterClosureReport::from_reports` as the thinner adapter-facing entrypoint.
That helper copies the upstream report fields, derives rollback through the
readiness report, and sets `adapter_closure.allow_next_round` from
`readiness.can_schedule_next_round`; it must not rebuild ledger summaries,
rerun validation coverage, or recreate a readiness snapshot.
When `readiness_next_round_v1` exists, closure rollback fields should be
projected through `RollbackReport::from_readiness_report` so
`adapter_closure.rollback_required` and
`adapter_closure.rollback_resume_gate` remain aligned with
`readiness.rollback_required`, `readiness.rollback_reason`, and
`readiness.rollback_actions`.

Use `norion_eval::AdapterClosurePureDataContract::adapter_closure_v1` as the
adapter-facing boundary check before runner wiring. The contract allows only
normalized eval inputs such as `LedgerRecord`, `ReportGate`,
`HelperStageContractSummary`, `ValidationCommandCoverageEvidence`,
`ValidationCommandCoverageGate`, `AdapterReadinessReports`, and
`RootAdapterFailureKind`, plus already emitted report inputs such as
`LedgerGateReport`, `ValidationCommandCoverageReport`, and
`SelfEvolutionReadinessReport`. It must expose
`AdapterClosureReport::from_reports` as the report-input entrypoint and forbid
JSONL IO, HTTP/SSE, process spawning, validation-command spawning, model calls,
and runner state. The emission step must depend only on `ledger_gate_report_v1`,
`validation_command_coverage_report_v1`, `readiness_next_round_v1`, and
`rollback_report_v1`, with no future events and no current-runner blocking.
Use `stays_pure_data_boundary()` on the closure contract and plan to fixture-test
that the final closure accepts only normalized eval inputs, depends only on the
four prior reports above, and rejects JSONL readers, validation executors,
runner state, HTTP/SSE, process spawning, and model calls.
Use `AdapterClosurePureDataContract::schema_document` as the JSON/schema-style
drift guard: its field list must match `AdapterClosureReportSchema`, and every
documented closure field must be present in adapter report field coverage.
Use `AdapterClosurePureDataContract::schema_document_within_boundary` as the
negative guard before wiring: reject documents that add runner inputs, runner
state fields, HTTP/SSE or process wording in schema names, or that drop required
forbidden capabilities such as `jsonl_io`, `http_sse`, `process_spawn`, and
`model_call`.
Mirror this in `norion_test::AdapterClosureSchemaDocumentPlan` so plan-side
fixtures can require the same document fields, boundary sources, forbidden
capabilities, and emission field coverage before any runner wiring.
When checking runner-wiring readiness, project the document through
`EvalSchemaDriftEvidence::from_adapter_closure_schema_document`; missing
`adapter_closure.*` emission fields must block wiring as schema drift rather
than being treated as runner IO, validation execution, or model quality.
Use `norion_eval::AdapterRollbackResumeBoundaryContract::rollback_resume_v1`
and `norion_test::AdapterRollbackResumeBoundaryPlan` for the resume side: eval
may map `rollback.resume_gate`, `RollbackResumeEvidence`, and
adapter-report-field coverage into `RollbackResumeReport`, but neither eval nor
the thin runner adapter may execute rollback actions, run resume commands, call
models, or mutate runner state at this boundary.
Use `stays_pure_data_boundary()` on the contract and plan to verify the resume
gate list, report-field coverage input, forbidden execution capabilities, and
absence of runner state before adapter wiring.

Stable report fields for `adapter_closure_report_v1`:

- `adapter_closure.stage`
- `adapter_closure.helper_stage_useful`
- `adapter_closure.ledger_gate_blocked`
- `adapter_closure.validation_command_coverage_blocked`
- `adapter_closure.readiness_can_schedule_next_round`
- `adapter_closure.rollback_required`
- `adapter_closure.rollback_resume_gate`
- `adapter_closure.allow_next_round`

During report-only rollout, emit the stage, helper usefulness, ledger-gate
state, and validation-command coverage state as additive diagnostics. During
enforced rollout, emit the full schema and let
`adapter_closure.allow_next_round=false` be the final stop signal for
unattended scheduling. Keep `rollback.resume_gate` as the resume contract:
generic report/validation/readiness failures resume through
`planned_validation_command`, while root-adapter failures keep their specific
resume gates such as `runtime_backend_health_check` or
`chain_readiness_gate`.

The closure report should be emitted after `readiness_next_round_v1` and
`rollback_report_v1`, or derived from the same normalized inputs. Missing
closure fields are adapter report coverage gaps, not model-quality failures.

## Clean-Room Context Hygiene

Use `norion_eval::CleanRoomContextBoundaryContract::clean_room_context_hygiene_v1`
and `norion_test::CleanRoomContextBoundaryPlan` for worker-window context
hygiene before accepting eval/test contract changes. The runner or coordinator
may normalize evidence labels into `CleanRoomContextObservation` values, but
eval only classifies the normalized source kinds and emits
`clean_room_context_hygiene_report_v1`.
For report-only adapter surfaces, `evolution-loop` extracts JSON fields such as
`raw_old_thread_dialog_included`, `raw_source_omitted`, and
`fresh_clean_room_assignment_exists`, then passes those already-normalized facts
through `CleanRoomReportOnlyContextHygieneEvidence`. `norion-eval` maps that
pure data into `CleanRoomContextEvidence` and `CleanRoomContextReport`; it does
not parse source JSON or inspect old-window payloads.

Allowed evidence kinds are current workspace files and the explicit
coordination tail. Polluted evidence kinds are old-thread reads, raw dialog
payloads, and reuse of a completed worker window for follow-up work. In strict
mode, at least one current-file observation must be present, coordination-tail
evidence is allowed, and any polluted observation blocks
`clean_room_context.allow_clean_room_eval`.

This contract is pure data. It must not read old threads, inspect chat
transcripts, parse raw dialogs, read files, scan ledgers, call HTTP/SSE, spawn
commands, start or stop daemon/model/Web Lab/Forge processes, call models,
write `.ndkv` records, or execute repair actions. Completed windows remain
completion evidence only; a fresh clean-room window should be used for new
work.

Daemon transition evidence follows the same rule. A normalized
`round_done_waiting_ledger_commit` observation may be reported for display
while stdout has a done marker and the ledger/report is still behind, but the
eval contract requires `report_only=true`, `display_only=true`,
`side_effects=false`, and `allow_runtime_transition_action=false`. This evidence
does not permit daemon control, ledger scans, prompt replay, stream startup,
remote access, `.ndkv` writes, or any other runtime side effect.

For the combined live status view, use
`norion_eval::LiveStatusBundleReport::from_gate_and_evidence` with normalized
daemon fields, an existing `CleanRoomContextReport`, and
`LiveStatusBundleReportGateReadiness`. The bundle may show `active_busy` for an
in-progress daemon round or `ledger_pending` for a done round waiting on ledger
commit. Even when `report_gate_ready=true`, the emitted report keeps
`dispatch_work_allowed=false`, `prompt_replay_allowed=false`,
`process_start_allowed=false`, `memory_write_allowed=false`,
`ndkv_write_allowed=false`, and
`polluted_or_completed_windows_actionable=false`. `norion-test` mirrors this as
`LiveStatusBundlePlan`; it is a plan contract only and must not call service,
CLI, daemon, Forge, model, SSH, stream, file, or memory-write APIs.

For next-round decision evidence, use
`norion_eval::NextRoundDecisionReport::from_gate_and_evidence` after the runner
has already produced `live_status_bundle_report_v1` and
`readiness_next_round_v1`. The report is a pure decision surface with three
stable statuses:

- `safe_to_wait_current_round_active`: the daemon has a current round in
  progress, the live-status bundle is displayable, readiness evidence is clean,
  and the right action is to wait.
- `safe_to_continue_after_current_round`: no current round is active and
  readiness says another unattended round would be schedulable.
- `operator_attention_blocked`: read-only/report-only/no-side-effect checks,
  clean-room context hygiene, report-gate readiness, live-status display, or
  `readiness_next_round_v1` blocked.

Stable fields for `next_round_decision_report_v1` include:
`next_round_decision.decision_status`,
`next_round_decision.current_round_active`,
`next_round_decision.live_status_display_state`,
`next_round_decision.readiness_can_schedule_next_round`,
`next_round_decision.report_gate_ready`,
`next_round_decision.context_hygiene_passed`,
`next_round_decision.read_only`,
`next_round_decision.report_only`,
`next_round_decision.no_side_effects`,
`next_round_decision.dispatch_work_allowed`,
`next_round_decision.prompt_replay_allowed`,
`next_round_decision.process_start_allowed`,
`next_round_decision.memory_write_allowed`,
`next_round_decision.ndkv_write_allowed`,
`next_round_decision.operator_attention_required`, and
`next_round_decision.failure_reasons`.

This report is not a daemon-control API. It must keep
`dispatch_work_allowed=false`, `prompt_replay_allowed=false`,
`process_start_allowed=false`, `memory_write_allowed=false`, and
`ndkv_write_allowed=false` even when the decision is safe. The coordinator or
runner remains the owner of any future scheduling decision.

For downstream status consumers, project the already-built
`next_round_decision_report_v1` through
`norion_eval::NextRoundDownstreamStatusReport::from_gate_and_evidence`. This
creates `next_round_downstream_status_consumers_v1` for four consumers only:
service/CLI display status, Forge operator display, agent assignment
acceptance, and memory self-improve admission visibility. The projection
accepts normalized pure facts only and is not allowed to call service/CLI,
Forge, agent dispatch, memory admission, daemon, HTTP/SSE, model, file, JSONL,
or `.ndkv` APIs.

Required downstream fields:

- `next_round_downstream.source_decision_status`
- `next_round_downstream.effective_decision_status`
- `next_round_downstream.service_cli_display_status`
- `next_round_downstream.forge_operator_display_status`
- `next_round_downstream.agent_assignment_acceptance`
- `next_round_downstream.memory_self_improve_admission_visibility`
- `next_round_downstream.operator_attention_required`
- `next_round_downstream.read_only`
- `next_round_downstream.report_only`
- `next_round_downstream.no_side_effects`
- `next_round_downstream.dispatch_work_allowed`
- `next_round_downstream.prompt_replay_allowed`
- `next_round_downstream.process_start_allowed`
- `next_round_downstream.memory_write_allowed`
- `next_round_downstream.ndkv_write_allowed`

Optional downstream diagnostics:

- `next_round_downstream.current_round_active`
- `next_round_downstream.live_status_display_state`
- `next_round_downstream.active_round`
- `next_round_downstream.ledger_latest_round`
- `next_round_downstream.latest_done_round`
- `next_round_downstream.readiness_can_schedule_next_round`
- `next_round_downstream.failure_reasons`

When the source decision is `safe_to_wait_current_round_active`, service/CLI and
Forge may display a safe-to-wait state, agent assignment acceptance must defer
until the current round completes, and memory self-improve admission visibility
must remain waiting-only. When the source decision is
`safe_to_continue_after_current_round`, agent assignment acceptance may show
`accept_next_round_assignment` and memory visibility may show
`visible_admission_safe`; this still does not authorize dispatch or writes. If
the downstream projection sees non-normalized facts, a missing consumer, or any
side-effect marker, the effective decision becomes `operator_attention_blocked`
for all four consumers.

For fixture and thin-adapter tests, use
`project_next_round_decision_report_to_downstream_status` after the runner has
already normalized or emitted the `next_round_decision_report_v1` facts. This
helper returns downstream evidence plus the downstream report, but it must stay
inside the pure eval boundary: no JSON/file reads, service/CLI/Forge calls,
agent dispatch, memory admission, daemon control, prompt replay, process start,
model calls, or `.ndkv` writes.

## What Not To Change In This Phase

Do not move command execution into `norion-test`.
Do not move ledger IO into `norion-eval`.
Do not replace `tools/evolution-loop` report tests until the main window wires
the crates into the workspace and adds an adapter.
