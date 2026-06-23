# norion-eval Architecture

`crates/norion-test` and `crates/norion-eval` are intended to become the shared
test and evaluation vocabulary for the SmartSteam evolution loop. They are
independent crates: the root workspace will wire them later, and they do not
modify or replace `tools/evolution-loop`.

## Current Boundary

`tools/evolution-loop` remains the executable loop. It already owns:

- SSE continuity handling for `/v1/business-cycle-stream`.
- state consistency gate and report gate.
- local validation command execution.
- runtime response checks.
- JSONL ledger writing under `target/evolution`.

The new crates only define typed plans, records, summaries, gates, and rollback
decisions. They are pure Rust data/decision layers and do not run Gemma, shell
commands, or full-repo builds.

## norion-test

`norion-test` describes test plans that a runner can execute later:

- `SmartSteamCase`: a business-cycle prompt, endpoint, token budget, feedback
  expectation, self-improve expectation, and optional Rust check snippet.
- `StreamContinuityCheck`: the terminal SSE evidence needed to accept a stream:
  `done`, `final`, no `error`, and no incomplete buffered frame.
- `BackendHealthCheckPlan`: backend readiness requirements such as safe-device,
  experience hygiene, model readiness, and minimum runtime context.
- `ExperienceAuditPlan`: a data-only plan for `/v1/experience-cleanup-audit`
  that records the audit limit and Context Rot thresholds the runner should
  collect before allowing unattended evolution to expand.
- `CliUiSmokePlan`: one smoke-test bundle tying backend health, a SmartSteam
  case, a CLI verification plan, and an optional Web Lab URL.
- `ModelRole`, `ModelWorkerPlan`, and `ModelPoolSmokePlan`: data-only plans for
  future parallel model workers. They describe which worker role to run, which
  model and endpoint it should use, token/timeout limits, whether it may block
  the primary 12B path, and which validation plan checks that worker or merged
  output.
- `ModelPoolDevelopmentWindowPlan`: a data-only plan requiring Apple Silicon
  model-pool gains to hold across a multi-round window, not just a single
  lucky worker run.
- `ModelPoolDevelopmentAttributionPlan`: a data-only plan for proving that
  each Apple Silicon worker has latency, token, success, feedback, validation,
  duplicate/noisy output, and primary-12B blocking evidence before development
  gain is attributed to the model pool.
- `AdapterAcceptancePlan`, `ContextRotAcceptancePlan`, and
  `SelfEvolutionReadinessPlan`: data-only rollout plans for root adapter,
  Context Rot, and next-round readiness acceptance. They name the expected
  report schema, required gate inputs, verification command, and whether the
  stage may block the current runner.
- `LegacyLedgerReplayPlan`: a data-only compatibility plan for replaying
  existing `target\evolution\*.jsonl` ledgers while treating future worker,
  Context Rot, feedback/self-improve, and readiness reports as optional
  additive evidence. It names `legacy_ledger_replay_report_v1` as the report
  emitted by the replay check.
- `SteamRoundAcceptancePlan`: a data-only plan that ties one
  `/v1/business-cycle-stream` case to stream continuity, validation, and
  next-round readiness acceptance.
- `SteamCaseMatrixPlan`: a data-only plan that requires multiple
  `/v1/business-cycle-stream` cases, stable final JSON fields, unique case IDs,
  stream continuity, validation, and business-cycle verdicts before enforced
  root adapter rollout.
- `ValidationCommandCoveragePlan`: a data-only plan for requiring validation
  command line, phase, exit status, output-tail, pass result, and rust-check
  evidence before unattended rounds are allowed to continue.
- `RollbackReportPlan`: a data-only plan for surfacing rollback reason,
  actions, resume gate, and stop-scheduling policy without moving rollback
  execution into the crates.
- `ExperimentRolloutReportPlan`: a data-only plan for reporting experiment flag
  rollout state and enforcing clean Context Rot before expansion.
- `ExperimentKillSwitchReportPlan`: a data-only plan for requiring enabled
  experiments to have a documented kill switch, rollback report, resume gate,
  clean Context Rot, and owner acknowledgement before expansion.
- `FeedbackSelfImproveReportPlan`: a data-only plan for surfacing whether
  feedback and self-improve produced a closed loop before another unattended
  round is allowed.
- `LedgerGateReportPlan`: a data-only plan for surfacing ledger hygiene,
  runtime, stream, final-event, state/trace, Context Rot penalty, and
  last-success report-gate evidence before another round is scheduled.
- `RootAdapterAttributionReportPlan`: a data-only plan for surfacing prompt
  gate, backend 8686, final JSON, runtime response, and model-quality
  attribution without changing the current runner.
- `AdapterFixtureContractPlan`: a data-only plan for root adapter fixture
  coverage before wiring, including clean success, operational failures,
  runtime/stream failures, model-quality failure, and projection checks.
- `CurrentRunnerCompatibilityPlan`: a data-only aggregate plan for the final
  pre-wiring view across legacy replay, report bundle, schema drift, Apple
  Silicon development window/effect, self-evolution unattended prerequisites,
  adapter fixtures, Steam case matrix, validation coverage, promotion window,
  handoff, and runner tests.
- `EvalSchemaManifestPlan`: a data-only plan that names every schema required
  by the evolution-loop handoff.
- `EvalReportBundlePlan`: a data-only plan that names which report schemas are
  expected in shadow, report-only, and enforced rollout stages.
- `EvalSchemaDriftReportPlan`: a data-only plan for comparing the schema
  manifest, report bundle manifest, and adapter handoff checklist before root
  adapter wiring can become enforced.

It also keeps a data-only `VerificationPlan` for cargo/rustc commands. The plan
can describe commands like:

```powershell
cargo test --manifest-path .\tools\evolution-loop\Cargo.toml
```

but the crate does not execute them.

## norion-eval

`norion-eval` evaluates structured loop evidence:

- `LedgerRecord`: normalized fields from one evolution ledger row.
- `LedgerSummary`: success rate, ledger hygiene, validation pass rate,
  feedback totals, self-improve pass rate, state/trace gate pass counts, stream
  truncations, missing final-event failures, runtime response failures, missing
  runtime model count, zero-token count, and Context Rot/noise penalty.
- `ValidationGate`: blocks unchecked or failed validation depending on policy.
- `RuntimeResponseGate`: blocks zero/missing runtime tokens and missing runtime
  model evidence.
- `ReportGate`: decides whether the loop can enter the next round using summary
  thresholds.
- `LedgerGateReportSchema` and `LedgerGateReport`: project the full
  `LedgerSummary + ReportGate` result into a stable report surface.
- `RootAdapterAttributionReportSchema` and `RootAdapterAttributionReport`:
  project root adapter outage and model-quality attribution evidence.
- `LegacyLedgerReplayReportSchema` and `LegacyLedgerReplayReport`: project old
  JSONL replay compatibility and missing additive report gaps.
- `RollbackPlan`: describes stop/preserve/revalidate actions after a blocked
  gate.
- `ExperimentFlag`: records rollout state for future eval experiments.
- `ExperimentRolloutGate`: prevents enabled experiments from exceeding a safe
  rollout percentage, requiring ownership, or expanding while Context Rot is
  dirty.
- `CodeQualitySignal`: carries validation, Rust check, and warning evidence.
- `ContextRotSignal` and `ContextRotGate`: carry experience-audit noise,
  quarantine, legacy metadata, and duplicate-output evidence into a reusable
  gate.
- `ModelRole`: the role vocabulary for future model pools: `planner`,
  `reviewer`, `tester`, `summarizer`, and `high_quality`.
- `ModelWorkerRecord`: one worker's latency, runtime token use, success,
  feedback, validation, duplicate/noisy output flags, and whether it blocked
  the primary 12B model.
- `ModelPoolSummary` and `ModelPoolGate`: aggregate and gate the multi-model
  pool so adding workers remains measurable rather than chaotic.
- `ModelPoolDevelopmentEffect` and `AppleSiliconDevelopmentGate`: compare a
  model pool against the primary 12B baseline so Apple Silicon parallelism has
  to prove development value, not only produce more output.
- `ModelPoolDevelopmentAttributionEvidence`,
  `ModelPoolDevelopmentAttributionSummary`,
  `ModelPoolDevelopmentAttributionGate`, and
  `ModelPoolDevelopmentAttributionReport`: split per-worker development
  attribution from root-adapter readiness. They record each worker's
  latency/tokens/success/feedback/validation/duplicate/noisy/primary-blocking
  evidence and keep `chain_not_ready` / `model_unavailable` out of
  `model_quality_failure` accounting.
- `ModelPoolDevelopmentWindowRecord`,
  `ModelPoolDevelopmentWindowSummary`, and
  `AppleSiliconDevelopmentWindowGate`: aggregate multiple
  `ModelPoolDevelopmentReport` rounds so Apple Silicon gains require sustained
  feedback delta, bounded latency/tokens, no duplicate/noisy output, no primary
  12B blocking, and clean operational readiness attribution.
- `EvalSchemaManifest`, `EvalReportBundleManifest`,
  `EvalSchemaDriftEvidence`, `EvalSchemaDriftGate`, and
  `EvalSchemaDriftReport`: compare schema lists and deterministic checksums
  across the manifest, bundle, and handoff checklist before any enforced root
  adapter wiring.
- `AdviceContinuationObservation`, `AdviceContinuationEvidence`,
  `AdviceContinuationGate`, and `AdviceContinuationReport`: normalize
  higher-level continuation advice plus the latest ledger summary into a pure
  report about repeated advice, invalid advice, invalid commands, and
  latest-round success without pulling JSONL IO, daemon control, or runner
  state into eval.
- `SelfEvolutionUnattendedPrerequisiteEvidence`,
  `SelfEvolutionUnattendedPrerequisiteGate`, and
  `SelfEvolutionUnattendedPrerequisiteReport`: aggregate continuity,
  regression, readiness, rollback resume, Steam case matrix, validation command
  coverage, promotion-window, and Apple Silicon development-effect evidence
  before claiming unattended self-evolution progress.
- `StrictUnattendedSupervisorEvidence`,
  `StrictUnattendedAcceptanceReportSchema`, and
  `StrictUnattendedAcceptanceReport`: project supervisor/daemon strict
  unattended evidence plus report refresh, adapter closure, validation
  coverage, rollback resume, self-improve regression, readiness, and
  self-evolution prerequisite reports into one allow/failure-reason surface.
  The projection is pure data: it does not start daemons, send prompts, touch
  remote runtimes, execute validation commands, or parse helper prose.
- `CleanRoomContextObservation`, `CleanRoomContextEvidence`,
  `CleanRoomContextGate`, and `CleanRoomContextReport`: distinguish clean
  current-file and coordination-tail evidence from polluted old-thread reads,
  raw dialog payloads, and completed-window follow-up reuse. The report is a
  schema-level hygiene contract only; it does not read files, inspect threads,
  parse chat streams, or execute repair actions.
- `DaemonRoundTransitionEvidence`, `DaemonRoundTransitionGate`, and
  `DaemonRoundTransitionReport`: project normalized daemon transition facts
  such as `round_done_waiting_ledger_commit` into a display/report-only
  contract. The evidence must keep `side_effects=false` and never authorizes a
  runtime transition action; consumers may show the pending ledger commit but
  must not start/stop daemons, scan ledgers, write `.ndkv`, or touch remotes.
- `LiveStatusBundleDaemonState`, `LiveStatusBundleReportGateReadiness`,
  `LiveStatusBundleEvidence`, `LiveStatusBundleGate`, and
  `LiveStatusBundleReport`: compose daemon transition state, read-only
  service/CLI context hygiene, and report-gate readiness into
  `live_status_bundle_report_v1`. Downstream consumers may display
  `active_busy`, `ledger_pending`, or `ledger_synced`, but readiness and display
  fields never authorize dispatch, prompt replay, process start, memory writes,
  `.ndkv` writes, or making polluted/completed worker windows actionable.
- `NextRoundDecisionEvidence`, `NextRoundDecisionGate`, and
  `NextRoundDecisionReport`: compose `live_status_bundle_report_v1` with
  `readiness_next_round_v1` into `next_round_decision_report_v1`. The report
  can represent `safe_to_wait_current_round_active`,
  `safe_to_continue_after_current_round`, and `operator_attention_blocked`.
  It is explicitly read-only and report-only; it never authorizes daemon
  start/stop, dispatch, prompt replay, process start, memory writes, `.ndkv`
  writes, runner-state mutation, or remote/model calls.
- `NextRoundDownstreamStatusEvidence`, `NextRoundDownstreamStatusGate`, and
  `NextRoundDownstreamStatusReport`: project an existing
  `next_round_decision_report_v1` into
  `next_round_downstream_status_consumers_v1` for service/CLI display status,
  Forge operator display, agent assignment acceptance, and memory
  self-improve admission visibility. The projection accepts only normalized
  pure facts, marks all consumer status and no-side-effect fields as required,
  keeps round ids and failure reasons optional diagnostics, and forbids service
  calls, CLI execution, Forge calls, agent dispatch, assignment writes, memory
  admission writes, `.ndkv` writes, and runtime side effects.
  `project_next_round_decision_report_to_downstream_status` is the pure helper
  for already-emitted evolution-loop-shaped reports: it constructs downstream
  evidence and report values from an existing `NextRoundDecisionReport` only,
  without JSON parsing, IO, service/CLI/Forge calls, agent dispatch, memory
  admission, daemon control, or writes.

## Compatibility With Existing Clean Ledger

The current clean ledger sample is:

```text
target\evolution\runtime-model-gated-loop-20260613-175925.jsonl
```

It already contains the fields needed by `norion-eval`: `success`,
`runtime_tokens`, `runtime_model`, `validation_checked`, `validation_passed`,
`feedback_applied`, stream failure errors, and report-gate-friendly metadata.
The validation meta records the existing `tools/evolution-loop` test suite
passing with 67 tests.

Future wiring should parse each JSONL row into `LedgerRecord`, call
`LedgerSummary::from_records`, then evaluate:

1. `RuntimeResponseGate::strict()` per record.
2. `ValidationGate::strict()` per record.
3. `ReportGate` over the summary.
4. `RollbackPlan::from_decision` for any blocked result.

## Schema Drift Gate

Before the root adapter can move from report-only to enforced, the runner should
emit `schema_drift_report_v1` from pure contract data:

- expected schemas from `EvalSchemaManifest::evolution_loop_handoff_v1()`;
- bundle schemas from `EvalReportBundleManifest::for_stage(Enforced)`;
- handoff schemas from `AdapterHandoffChecklist::before_runner_wiring(Enforced)`;
- adapter report fields from `AdapterReportEmissionPlan::required_report_fields()`.

The report carries sorted schema lists plus stable checksums for all three
sources. Any missing schema, extra schema, or duplicate schema name blocks
enforced wiring. This catches report bundle/checklist drift before a root
runner change can silently omit a gate such as `model_worker_gate_report_v1`,
`root_adapter_attribution_report_v1`, or `schema_drift_report_v1` itself.
It also carries report-field contract checksums for the adapter emission plan.
Missing, extra, or duplicate adapter report fields block enforced wiring as
schema drift; old ledger replay may still classify those reports as additive
gaps.
The report-field contract includes representative Apple Silicon and Context Rot
fields, including `apple_silicon_effect.feedback_applied`,
`model_pool_attribution.validation_checked`,
`model_pool_budget.missing_required_roles`,
`context_rot_trend.latest_noisy_records`, and
`context_rot_remediation.allow_experiment_rollout`. If any of those fields
disappears from `AdapterReportEmissionPlan::required_report_fields()`, schema
drift blocks runner wiring before promotion or handoff can claim coverage. This
is still contract drift, not `model_quality_failure`.

The gate remains pure: no filesystem scan, ledger parsing, JSON parsing, HTTP,
or command execution. Legacy ledgers may lack `schema_drift_report_v1`; that is
an additive replay gap, not a replay failure.

## Apple Silicon Development Window

`model_worker_gate_report_v1` and `model_pool_budget_fairness_report_v1` judge a
single model-pool observation. `model_pool_development_attribution_report_v1`
then checks whether the per-worker evidence can actually be attributed to
development gain. `model_pool_development_window_report_v1` adds a multi-round
window before claiming the Apple Silicon pool improves development.
`apple_silicon_baseline_comparison_report_v1` then checks paired rounds against
the primary 12B baseline so the claim is "better development", not merely "more
workers ran".

The window counts a development-claim round only when:

- feedback delta versus the primary 12B baseline is positive;
- pool and development gates both pass;
- duplicate output, noisy output, and primary 12B blocking are all zero;
- root adapter failure kind is `none`.

`chain_not_ready` and `model_unavailable` are counted as operational readiness
rounds. They block a development claim until fixed, but they are not counted as
`model_quality_failure`. This preserves the 8686-down plus prompt-gate rule
while still preventing missing infrastructure from being reported as a model
pool win.

The single-round model-pool report exposes both rates and raw counts:
`model_pool.successful_workers`, `model_pool.validation_checked`,
`model_pool.validation_passed`, `model_pool.latency_ms_max`, and
`model_pool.runtime_tokens_max` sit beside the total latency/token fields. Max
latency and runtime-token values are report-only evidence for the strict
per-worker caps, so a pool cannot claim development improvement merely because
the aggregate total looks acceptable.

`model_pool_development_attribution_report_v1` is the single-round guard in
front of the window. It reports worker ids, roles, latency, runtime tokens,
success, feedback applied, validation checked/passed, duplicate output, noisy
output, and primary-12B blocking. In enforced rollout it requires successful
workers to have nonzero latency/tokens, all worker validation to be checked and
passed, feedback to be applied, duplicate/noisy output to be zero, and no worker
to block the primary 12B path. It also compares classified root-adapter failures
with any reported quality-failure count. If prompt-gate blocked while 8686 was
down, the failure kind is `chain_not_ready`; if prompt-gate passed and 8686 was
down, it is `model_unavailable`. Both block `allow_development_claim`, but the
reported model-quality count must remain zero unless final JSON, runtime model,
runtime tokens, and a failed business-cycle verdict are all present.

`apple_silicon_baseline_comparison_report_v1` is the paired comparison guard.
Each record carries a round number, primary 12B feedback/latency/tokens,
model-pool feedback/latency/tokens, primary 12B success and validation, pool
success and validation, duplicate/noisy output, primary-12B blocking, and the
classified root adapter failure kind. The summary exposes paired rounds,
feedback gain rate, feedback delta total, success/validation regression rounds,
latency and token budget misses, duplicate/noisy output, primary-12B blockers,
operational readiness rounds, model-quality failure rounds, and max latency and
token multipliers. In enforced rollout it blocks if there are too few paired
rounds, not enough feedback-gain rounds, feedback delta is too low, success or
validation regresses, latency/token budgets are exceeded, duplicate/noisy
output appears, the pool blocks the primary 12B, or readiness/quality failures
are unresolved. `chain_not_ready` and `model_unavailable` are readiness
failures for this report too; they block the claim without consuming
model-quality budget.

`apple_silicon_development_effect_report_v1` is the pre-wiring aggregate gate
for the question "did the Apple Silicon model pool improve development?" It
does not replace worker, budget, or window reports; it joins them into a single
claim decision:

| Input | Required before enforced claim |
| --- | --- |
| `model_pool_development_attribution_report_v1` | per-worker latency, tokens, success, feedback, validation, duplicate/noisy output, primary-12B blocking, and failure attribution are complete |
| `model_pool_budget_fairness_report_v1` | planner/reviewer/tester work is balanced enough to expand the pool |
| `model_pool_development_window_report_v1` | gain holds across the required multi-round window |
| `apple_silicon_baseline_comparison_report_v1` | paired model-pool rounds beat the primary 12B baseline without regressions or operational/quality confusion |
| `root_adapter_attribution_report_v1` | root failure kind is classified before any quality claim |

The aggregate report exposes worker count plus the per-worker proof rows:
worker ids, roles, latency, runtime tokens, success, feedback applied,
validation checked/passed, duplicate output, noisy output, and primary-12B
blocking. It also exposes
attribution/window/budget allow bits, `chain_not_ready`, `model_unavailable`,
classified `model_quality_failure`, reported quality-failure count,
`model_quality_failure_allowed`, `operational_failure_counted_as_quality`,
duplicate/noisy output counts, primary-12B blockers, and whether successful
workers had latency/tokens. It also exposes
`apple_silicon_effect.baseline_comparison_allowed`. Enforced rollout blocks if
any input report blocks, if the paired baseline comparison blocks, if worker
metric rows are inconsistent, if operational readiness failures are present, or
if reported quality failures differ from classified quality failures after
readiness ordering. Prompt-gate blocked plus 8686 down remains
`chain_not_ready`; prompt-gate passed plus 8686 down remains
`model_unavailable`; neither is allowed to become `model_quality_failure`.

The effect gate now also emits explicit worker metric coverage before accepting
an Apple Silicon improvement claim. Every worker row must carry latency,
runtime tokens, success, feedback, validation checked/passed,
duplicate-output, noisy-output, and primary-12B blocking columns. In enforced
rollout, any missing row, unchecked or failed validation, duplicate/noisy
output, or primary-12B blocking event blocks the development-effect claim even
if another aggregate report looks healthy. `model_quality_failure_allowed` is
true only for a classified `model_quality_failure`; if prompt-gate blocked and
8686 is also down, the report must show `chain_not_ready` and
`operational_failure_counted_as_quality=false`. This keeps "more workers" from
being confused with "better development".

The attribution report schema must include both
`model_pool_attribution.validation_checked` and
`model_pool_attribution.validation_passed`. A worker that reports a pass without
the checked bit is incomplete evidence, not proof that the Apple Silicon pool
improved development. This keeps the worker-level latency, tokens, success,
feedback, validation, duplicate/noisy, and primary-12B-blocking columns aligned
with the aggregate `apple_silicon_effect.*` report fields.

The enforced report bundle must contain both
`model_pool_development_attribution_report_v1` and
`apple_silicon_development_effect_report_v1`. The first proves the per-worker
evidence and readiness attribution; the second proves the aggregate Apple
Silicon development-effect claim. Omitting either is a bundle/report coverage
failure, not a model quality failure.

`AdapterReportEmissionPlan::apple_silicon_development_effect()` fixes the
future report emission order and required final report fields for this
aggregate without moving IO into `norion-eval`:

1. Project `model_worker_v1` from current ledger rows plus future worker
   events.
2. Project `root_adapter_attribution_report_v1` from backend/prompt/final JSON
   and runtime evidence.
3. Emit `model_worker_gate_report_v1` and
   `model_pool_development_attribution_report_v1`.
4. Emit `model_pool_budget_fairness_report_v1`.
5. Emit `model_pool_development_window_report_v1`.
6. Emit `apple_silicon_baseline_comparison_report_v1`.
7. Emit `apple_silicon_development_effect_report_v1`.
8. Emit `context_rot_report_v1`, `context_rot_trend_report_v1`, and
   `context_rot_remediation_report_v1`.
9. Emit `steam_case_matrix_report_v1` and
   `validation_command_coverage_report_v1`.
10. Emit `self_evolution_continuity_report_v1`,
   `self_evolution_regression_report_v1`, and `rollback_resume_report_v1`.
11. Emit `readiness_next_round_v1` after Apple Silicon effect, Context Rot
    snapshot/trend/remediation, Steam case matrix, validation command coverage,
    rollback resume, and root adapter attribution reports.
12. Emit `adapter_report_emission_report_v1`.
13. Emit `adapter_future_event_coverage_report_v1`.
14. Emit `report_bundle_gate_report_v1`.
15. Emit `adapter_promotion_window_report_v1`.
16. Emit `self_evolution_unattended_prerequisites_report_v1` after continuity,
    regression, readiness, Context Rot trend/remediation, rollback resume,
    Steam case matrix, validation command coverage, Apple Silicon
    development-effect, and promotion-window evidence.

The future worker events are `worker_output_fingerprint`, `worker_noise_score`,
`worker_primary_wait_ms`, and `worker_failure_kind`. Root attribution evidence
events include backend 8686 health, prompt-gate status, final JSON presence,
runtime model/tokens, and the business-cycle verdict. Steam and validation
readiness events include
`steam_case_id`, `steam_case_endpoint`, `steam_case_kind`,
`validation_command_phase`, `validation_command_line`,
`validation_command_status_code`, and `validation_output_tail`. The emission plan itself never blocks the current
runner; blocking remains owned by promotion, handoff, and current-runner
compatibility gates after report-only evidence has been observed.

`adapter_report_emission_report_v1` is the observed counterpart to that plan.
It records planned report names, observed report names, planned future events,
observed future events, planned report fields, observed report fields, and
`adapter_emission.missing_report_fields`. In enforced rollout it blocks report
promotion when the adapter omits a planned report, emits a dependent report
before its inputs, misses required future events such as worker output
fingerprints, worker failure kind, backend/prompt/runtime attribution evidence,
Steam case evidence, or validation command evidence, or omits required
attribution, effect, and budget report fields such as
`model_pool_attribution.validation_checked`,
`model_pool_attribution.validation_passed`,
`model_pool_attribution.blocked_primary_12b`,
`model_pool_attribution.chain_not_ready_count`,
`model_pool_attribution.model_unavailable_count`,
`apple_silicon_effect.feedback_applied`,
`apple_silicon_effect.operational_readiness_failure_kind`,
`apple_silicon_effect.quality_failure_blocked_by_readiness_order`, and
`apple_silicon_effect.operational_failure_counted_as_quality`, plus
`model_pool_budget.missing_required_roles`,
`model_pool_budget.dominant_runtime_token_roles`, and
`model_pool_budget.runtime_token_share_by_role`. The same field coverage
contract now includes Context Rot report fields: `context_rot.noisy_records`,
`context_rot.max_noise_penalty`, `context_rot.duplicate_outputs`,
`context_rot_trend.latest_noisy_records`,
`context_rot_trend.latest_duplicate_outputs`,
`context_rot_trend.remediation_improved_noise`,
`context_rot_trend.remediation_improved_duplicates`,
`context_rot_trend.allow_unattended_continuation`,
`context_rot_remediation.quarantine_candidates`,
`context_rot_remediation.clean_gists_backfilled`,
`context_rot_remediation.duplicate_outputs_removed`, and
`context_rot_remediation.allow_experiment_rollout`.
Promotion, handoff, and current-runner compatibility consume this as
`adapter_report_field_coverage_passed`; the adapter must not treat omitted
worker/effect fields as model quality failures. A missing worker metric is a
report coverage failure, and a missing Context Rot trend/remediation field is
also report coverage failure, while backend 8686 outage or prompt-gate blockage
remains `model_unavailable` or `chain_not_ready`.
`readiness_next_round_v1` must be emitted after Context Rot
snapshot, trend, and remediation reports, Steam case matrix, validation command
coverage, rollback resume, Apple Silicon development-effect, and root adapter
attribution reports;
`adapter_promotion_window_report_v1` must be emitted after readiness and the
report bundle gate. `self_evolution_unattended_prerequisites_report_v1` is a
post-promotion aggregate; it consumes promotion evidence and must not be used as
an input to promotion itself.

`adapter_future_event_coverage_report_v1` compares future events required by
adapter contracts with the events planned by `adapter_report_emission_report_v1`.
It specifically protects cross-contract events such as worker output
fingerprints, worker noise scores, primary-12B blocking reasons, root
attribution evidence (`backend_8686_reachable`, `prompt_gate_blocked`,
`final_json_present`, runtime model/tokens, and `business_cycle_passed`),
Context Rot audit/trend/remediation events, Steam case ids/endpoints/kinds,
validation command phases/lines/status codes/output tails, and paired baseline evidence
(`baseline_12b_feedback_applied`, `baseline_12b_latency_ms`,
`baseline_12b_runtime_tokens`, `baseline_12b_success`, and
`baseline_12b_validation_passed`). In report-only mode missing coverage is
visible but non-blocking. In enforced mode missing required events block adapter
emission promotion before the report bundle can claim completeness.

## Evolution Loop Adapter Shape

Do not move ledger IO or command execution into `norion-eval`. The adapter
should live beside the current runner and translate the existing row shape into
pure eval records.

Current ledger row to `LedgerRecord` mapping:

- `round` -> `round`
- `case` -> `case_name`
- `success`, `error`, `runtime_tokens`, `runtime_model`, `answer`,
  `elapsed_ms`, `delta_chars` -> same logical fields
- `business_cycle_passed`, `feedback_applied`, `rust_check_checked`,
  `rust_check_passed`, `rust_check_feedback_applied` -> same logical fields
- `validation_checked`, `validation_passed`, `self_improve_passed`,
  `state_gate_checked`, `state_gate_passed`, `trace_gate_checked`,
  `trace_gate_passed` -> gate evidence
- presence of `final_preview` or a parsed final SSE event -> `final_present`
- experience audit `max_noise_penalty` or future Context Rot output ->
  `context_noise_penalty`

The adapter should keep legacy behavior for missing fields: a missing validation
field means unchecked, a missing final event means report-gate failure, and a
missing runtime model after successful token generation means runtime response
failure.

## Root Business-Cycle Adapter Gate

The root `/v1/business-cycle-stream` final JSON has stronger field evidence
than the outer JSONL ledger for some gates. Before wiring it into the root
runner, use `RootBusinessCycleAdapterPlan::root_business_cycle_json()` as the
contract for what can be enforced and what must remain advisory.

The root crate now contains a thin shadow adapter at
`src/gemma_business/eval_adapter.rs`. It projects backend health JSON plus a
business-cycle final JSON body into `RootAdapterFailureEvidence`, then delegates
classification to `norion_eval::classify_root_adapter_failure`. This adapter
does not run Gemma, write ledgers, or change runner behavior; it exists to keep
root outage attribution aligned with the eval crate before enforcement.

Strong mappings are direct business-cycle verdicts and can be enforced once the
adapter has a clean fixture:

| Target | Root JSON field | Why strong |
| --- | --- | --- |
| `LedgerRecord.business_cycle_passed` | `$.business_cycle.passed` | direct verdict |
| `LedgerRecord.feedback_applied` | `$.business_cycle.feedback_applied` | direct counter |
| `LedgerRecord.rust_check_checked` | `$.business_cycle.rust_check_checked` | direct checked flag |
| `LedgerRecord.rust_check_passed` | `$.business_cycle.rust_check_passed` | direct pass flag |
| `LedgerRecord.self_improve_checked` | `$.business_cycle.self_improve_checked` | direct checked flag |
| `LedgerRecord.self_improve_passed` | `$.business_cycle.self_improve_passed` | direct pass flag |
| `LedgerRecord.state_gate_checked` | `$.business_cycle.state_gate_checked` | direct checked flag |
| `LedgerRecord.state_gate_passed` | `$.business_cycle.state_gate_passed` | direct pass flag |
| `LedgerRecord.trace_gate_checked` | `$.business_cycle.trace_gate_checked` | direct checked flag |
| `LedgerRecord.trace_gate_passed` | `$.business_cycle.trace_gate_passed` | direct pass flag |

Weak mappings are generation evidence. They are useful for reports, but they
must not become model-quality failures until prompt and infrastructure gates
prove generation actually ran:

| Target | Root JSON field | Why weak |
| --- | --- | --- |
| `LedgerRecord.runtime_model` | `$.generate.runtime_model` | missing model can mean prompt-gate, 8686 outage, or truncation |
| `LedgerRecord.runtime_tokens` | `$.generate.runtime_token_count` | missing tokens before generation is not model quality evidence |
| `LedgerRecord.answer` | `$.generate.answer` | answer quality requires terminal final JSON |
| `ModelWorkerRecord.model` | `$.generate.runtime_model` | synthetic worker projection should not enforce during outages |
| `ModelWorkerRecord.latency_ms` | `$.generate.elapsed_ms` | latency is comparable only after generation starts |
| `ModelWorkerRecord.runtime_tokens` | `$.generate.runtime_token_count` | missing tokens before generation is not model quality evidence |
| `ModelWorkerRecord.success` | `$.business_cycle.passed` | synthetic worker success is advisory until outage attribution is clean |
| `ModelWorkerRecord.feedback_applied` | `$.business_cycle.feedback_applied` | single-worker projection is reportable but not enough for pool effect |
| `ModelWorkerRecord.validation_checked` | `$.business_cycle.rust_check_checked` | checked evidence must stay separate from pass evidence |
| `ModelWorkerRecord.validation_passed` | `$.business_cycle.rust_check_passed` | pass without checked evidence is incomplete worker coverage |

Rollout gate stages:

1. `ShadowOnly`: compute adapter projection in tests or logs only; no output or
   runner behavior changes.
2. `ReportOnly`: add an additive summary while weak mappings remain
   non-enforcing.
3. `Enforced`: allow strong mapping gates to block only after outage
   attribution, crate tests, evolution-loop tests, and workspace tests are
   green.

Failure attribution must run before any model-quality decision. If the prompt
gate blocked the request, classify it as `chain_not_ready` even when 8686 is
unreachable and `runtime_model` is missing. If the prompt gate passed but 8686
is unreachable, classify it as `model_unavailable`. Only classify
`model_quality_failure` after final JSON exists, runtime model/tokens are
present, and `business_cycle.passed=false`.

## Steam Case Matrix Gate

`steam_case_matrix_report_v1` is the coverage gate between a single passing
Steam round and enforced root adapter wiring. It stays pure data: the runner or
future adapter owns HTTP/SSE execution and final JSON parsing, then passes rows
to `SteamCaseCoverageReport::from_rows_and_gate()` or prebuilt
`SteamCaseCoverageEvidence` to `from_gate_and_evidence()`.
`tools/evolution-loop` does not need to call this helper until it has a thin
row adapter; the stable contract is that eval receives normalized rows, never
raw HTTP/SSE output.

The default enforced gate requires at least four unique
`/v1/business-cycle-stream` cases covering `planning`, `validation`,
`rollback`, and `apple_silicon_model_pool`. Every case must have stream
continuity, validation passed, `business_cycle.passed=true`, and these final
JSON fields:

| Required field | Why |
| --- | --- |
| `business_cycle.passed` | strong business-cycle verdict |
| `business_cycle.feedback_applied` | feedback closed-loop evidence |
| `generate.runtime_model` | model projection evidence after generation starts |
| `generate.runtime_tokens` | runtime response evidence after generation starts |
| `validation.checked` | validation was not skipped |
| `validation.passed` | validation succeeded |
| `self_improve.checked` | self-improve was evaluated |
| `self_improve.passed` | self-improve succeeded |

Stable report fields:

| Report field | Stage | Source |
| --- | --- | --- |
| `steam_case_matrix.stage` | report-only | `SteamCaseCoverageGate::stage` |
| `steam_case_matrix.case_count` | report-only | `SteamCaseCoverageEvidence::case_count` |
| `steam_case_matrix.min_cases` | report-only | `SteamCaseCoverageGate::min_cases` |
| `steam_case_matrix.case_ids` | report-only | `SteamCaseCoverageEvidence::case_ids` |
| `steam_case_matrix.case_kinds` | report-only | `SteamCaseCoverageEvidence::case_kinds` |
| `steam_case_matrix.endpoints` | report-only | `SteamCaseCoverageEvidence::endpoints` |
| `steam_case_matrix.required_case_kinds` | report-only | `SteamCaseCoverageGate::required_case_kinds` |
| `steam_case_matrix.missing_required_case_kinds` | report-only | `SteamCaseCoverageEvidence::missing_required_case_kinds` |
| `steam_case_matrix.required_final_json_fields` | report-only | `SteamCaseCoverageGate::required_final_json_fields` |
| `steam_case_matrix.missing_required_fields` | report-only | `SteamCaseCoverageEvidence::missing_required_fields` |
| `steam_case_matrix.stream_passed_cases` | report-only | `SteamCaseCoverageEvidence::stream_passed_cases` |
| `steam_case_matrix.validation_passed_cases` | report-only | `SteamCaseCoverageEvidence::validation_passed_cases` |
| `steam_case_matrix.business_cycle_passed_cases` | report-only | `SteamCaseCoverageEvidence::business_cycle_passed_cases` |
| `steam_case_matrix.coverage_blocked` | enforced | `SteamCaseCoverageGate::evaluate` |
| `steam_case_matrix.failure_reasons` | enforced | `SteamCaseCoverageGate::evaluate reasons` |
| `steam_case_matrix.allow_enforced_adapter` | enforced | inverted gate decision |

During `ShadowOnly` and `ReportOnly`, missing matrix evidence is observational.
During `Enforced`, `allow_enforced_adapter=false` blocks root adapter promotion
until coverage is complete. This gate does not reinterpret outages: prompt-gate
blocked plus 8686 unavailable remains `chain_not_ready`, and 8686 unavailable
after prompt-gate pass remains `model_unavailable`.

## Validation Command Coverage Gate

`validation_command_coverage_report_v1` closes a gap between a boolean
`validation_passed=true` and auditable command evidence. The runner or adapter
still owns command execution. `norion-eval` only receives
`ValidationObservation` rows plus rust-check evidence and decides whether the
next unattended round has enough proof.
Adapters can call `ValidationCommandCoverageReport::from_observations_and_gate`
when they have observation rows plus rust-check booleans, or
`from_gate_and_evidence` when they have already built evidence.

The default enforced gate requires at least one post-round validation command,
an exit status, an output tail in stdout or stderr, a passing command result,
and rust-check checked and passed. `Both` phase observations also satisfy the
post-round requirement. If helper or command evidence requests strict coverage
or a strict coverage control, enforced validation coverage additionally requires
pre-existing coverage tooling evidence or a coverage report reference. The eval
crate only evaluates these normalized strings; coverage tool execution and report
discovery stay in the runner/tooling layer.

Stable report fields:

| Report field | Stage | Source |
| --- | --- | --- |
| `validation_command.stage` | report-only | `ValidationCommandCoverageGate::stage` |
| `validation_command.command_lines` | report-only | `ValidationCommandCoverageEvidence::command_lines` |
| `validation_command.phases` | report-only | `ValidationCommandCoverageEvidence::phase_names` |
| `validation_command.status_codes_present` | report-only | `ValidationCommandCoverageEvidence::status_codes_present` |
| `validation_command.output_tail_captured` | report-only | `ValidationCommandCoverageEvidence::output_tail_captured` |
| `validation_command.passed_commands` | report-only | `ValidationCommandCoverageEvidence::passed_commands` |
| `validation_command.rust_check_checked` | report-only | `ValidationCommandCoverageEvidence::rust_check_checked` |
| `validation_command.rust_check_passed` | report-only | `ValidationCommandCoverageEvidence::rust_check_passed` |
| `validation_command.strict_coverage_requested` | report-only | `ValidationCommandCoverageEvidence::strict_coverage_is_requested` |
| `validation_command.coverage_tooling_evidence` | report-only | normalized coverage tool evidence labels |
| `validation_command.coverage_report_evidence` | report-only | normalized coverage report evidence labels |
| `validation_command.coverage_tooling_or_report_evidence_present` | report-only | `ValidationCommandCoverageEvidence::coverage_tooling_or_report_evidence_present` |
| `validation_command.coverage_blocked` | enforced | `ValidationCommandCoverageGate::evaluate` |
| `validation_command.coverage_failure_kind` | enforced | `validation_command_coverage` when blocked |
| `validation_command.model_quality_failure_counted` | enforced | always false for validation coverage |
| `validation_command.failure_reasons` | enforced | `ValidationCommandCoverageGate::evaluate reasons` |
| `validation_command.allow_next_round` | enforced | inverted gate decision |

This gate should run before the final report gate is allowed to schedule another
round. In report-only mode it exposes missing command metadata; in enforced mode
`validation_command.allow_next_round=false` is a stop signal until command
evidence is complete.
Validation coverage failures must stay validation failures. They can block
readiness and rollback resume, but they must not increment root adapter or worker
`model_quality_failure` counters.

## Context Rot And Experiment Gates

Context Rot is the evidence that evolution is polluting its own prompt or
memory surface. The eval crate treats it separately from ordinary validation so
an experiment can be technically green but still blocked from wider rollout.

Current and future audit fields should map to `ContextRotSignal` as follows:

| `ContextRotSignal` field | Current or future source |
| --- | --- |
| `noisy_records` | experience cleanup audit `noisy_records` |
| `max_noise_penalty` | experience cleanup audit `max_noise_penalty` |
| `quarantine_candidates` | experience cleanup audit `quarantine_candidates` |
| `repairable_legacy_metadata_lessons` | audit `repairable_legacy_metadata_lessons` |
| `legacy_metadata_without_clean_gist` | audit `legacy_metadata_without_clean_gist` |
| `duplicate_outputs` | model-pool duplicate output fingerprints or future report evidence |

`ContextRotGate::strict()` requires all of those values to be zero. During early
rollout, the runner can use relaxed thresholds in report-only mode, but an
enforcing unattended loop should require a clean Context Rot gate before
increasing experiment rollout.

The adapter emission contract should make missing Context Rot inputs visible
before readiness, promotion, or unattended-prerequisite reports consume them.
Required future event names are:

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

`ExperimentRolloutGate::conservative()` is intentionally small: enabled
experiments need an owner, cannot exceed 10% rollout, and cannot advance while
the Context Rot gate is blocked. Disabled flags may remain present while noisy
evidence is investigated because they do not expand the runtime surface.

`ContextRotReportSchema::experience_audit_v1()` fixes the report field names for
that evidence before the runner is wired. Raw audit metrics are report-only at
first:

| Report field | Stage | Source |
| --- | --- | --- |
| `context_rot.noisy_records` | report-only | `ContextRotSignal::noisy_records` |
| `context_rot.max_noise_penalty` | report-only | `ContextRotSignal::max_noise_penalty` |
| `context_rot.quarantine_candidates` | report-only | `ContextRotSignal::quarantine_candidates` |
| `context_rot.repairable_legacy_metadata_lessons` | report-only | `ContextRotSignal::repairable_legacy_metadata_lessons` |
| `context_rot.legacy_metadata_without_clean_gist` | report-only | `ContextRotSignal::legacy_metadata_without_clean_gist` |
| `context_rot.duplicate_outputs` | report-only | `ContextRotSignal::duplicate_outputs` |
| `context_rot.gate_blocked` | enforced | `ContextRotGate::evaluate` |
| `context_rot.failure_reasons` | enforced | `ContextRotGate::evaluate` reasons |

`ContextRotReport::from_signal_and_decision()` is the pure projection for this
shape. The runner should keep HTTP and JSON extraction local, then hand only
`ContextRotSignal` plus the gate decision to `norion-eval` when emitting or
testing `context_rot_report_v1`.

`ContextRotAcceptanceContract` mirrors the adapter rollout stages. In
`ReportOnly`, dirty Context Rot can be displayed as advisory evidence but cannot
block the current runner or experiment rollout. In `Enforced`, the strict gate
can block unattended rounds and prevent enabled experiment flags from expanding
until the audit is clean.
Use `advisory_report()` when a report-only adapter should expose dirty evidence
without blocking, and `blocking_report()` when an enforced adapter should emit
the stage-aware gate result.

`ContextRotTrendGate` adds a cross-round window before unattended continuation.
It is stricter than a single audit snapshot: it blocks when noisy records or
duplicate outputs persist across rounds, when the latest round is still dirty,
or when remediation was applied but the latest signal did not improve relative
to the beginning of the window. This prevents a loop from repeatedly reporting
the same Context Rot without proving that cleanup changed the trajectory.
`ContextRotTrendReport::from_points_and_gate()` is the pure projection for
adapters that already have a window of `ContextRotTrendPoint` audit snapshots.

`ContextRotTrendReportSchema::context_rot_trend_v1()` fixes that report shape:

| Report field | Stage | Source |
| --- | --- | --- |
| `context_rot_trend.rounds` | report-only | trend window length |
| `context_rot_trend.first_round` | report-only | first round in window |
| `context_rot_trend.last_round` | report-only | latest round in window |
| `context_rot_trend.latest_noisy_records` | report-only | latest signal |
| `context_rot_trend.latest_duplicate_outputs` | report-only | latest signal |
| `context_rot_trend.noisy_records_delta` | report-only | latest minus first |
| `context_rot_trend.duplicate_outputs_delta` | report-only | latest minus first |
| `context_rot_trend.max_consecutive_noisy_rounds` | enforced | trend summary |
| `context_rot_trend.max_consecutive_duplicate_rounds` | enforced | trend summary |
| `context_rot_trend.remediation_applied_rounds` | report-only | trend window evidence |
| `context_rot_trend.remediation_improved_noise` | enforced | trend summary |
| `context_rot_trend.remediation_improved_duplicates` | enforced | trend summary |
| `context_rot_trend.trend_blocked` | enforced | `ContextRotTrendGate::evaluate()` |
| `context_rot_trend.allow_unattended_continuation` | enforced | inverted trend decision |
| `context_rot_trend.failure_reasons` | enforced | trend gate reasons |

`ContextRotRemediationReportSchema::context_rot_remediation_v1()` tracks the
cleanup path after Context Rot is detected:

| Report field | Stage | Source |
| --- | --- | --- |
| `context_rot_remediation.stage` | report-only | `ContextRotRemediationGate::stage` |
| `context_rot_remediation.quarantine_candidates` | report-only | remediation evidence |
| `context_rot_remediation.quarantined_records` | report-only | remediation evidence |
| `context_rot_remediation.repairable_legacy_metadata_lessons` | report-only | remediation evidence |
| `context_rot_remediation.repaired_legacy_metadata_lessons` | report-only | remediation evidence |
| `context_rot_remediation.legacy_metadata_without_clean_gist` | report-only | remediation evidence |
| `context_rot_remediation.clean_gists_backfilled` | report-only | remediation evidence |
| `context_rot_remediation.duplicate_outputs` | report-only | remediation evidence |
| `context_rot_remediation.duplicate_outputs_removed` | report-only | remediation evidence |
| `context_rot_remediation.remediation_blocked` | enforced | `ContextRotRemediationGate::evaluate()` |
| `context_rot_remediation.failure_reasons` | enforced | remediation gate reasons |
| `context_rot_remediation.allow_experiment_rollout` | enforced | inverted remediation decision |

In enforced rollout, quarantine candidates must be quarantined, repairable
legacy metadata must be repaired, clean gists must be backfilled, and duplicate
outputs must be removed before the adapter expands experiments or schedules
more unattended self-evolution rounds.

`ContextRotRemediationReport::from_gate_and_signal()` is the audit-adapter
shortcut for current runners that only have `ContextRotSignal` available. It
derives the report-only cleanup evidence from the signal and leaves actual
quarantine, repair, and duplicate-removal counts at zero until the runner wires
real remediation events.

`ExperimentRolloutReportSchema::experiment_rollout_v1()` fixes the report
surface for experiment flags:

| Report field | Stage | Source |
| --- | --- | --- |
| `experiment.name` | report-only | `ExperimentFlag::name` |
| `experiment.enabled` | report-only | `ExperimentFlag::enabled` |
| `experiment.rollout_percent` | report-only | `ExperimentFlag::rollout_percent` |
| `experiment.owner` | report-only | `ExperimentFlag::owner` |
| `experiment.rollout_blocked` | enforced | `ExperimentRolloutGate::evaluate()` |
| `experiment.failure_reasons` | enforced | `ExperimentRolloutGate::evaluate()` reasons |
| `experiment.requires_clean_context_rot` | enforced | `ExperimentRolloutGate::require_clean_context_rot` |

`ExperimentRolloutReport::from_flag_and_decision` projects the flag, rollout
gate, and decision into that shape. Report-only rollout can show the flag state;
enforced rollout can block expansion when ownership, rollout percentage, or
Context Rot gates fail.

`experiment_kill_switch_report_v1` is the companion escape-hatch report for
enabled experiments. It prevents an experiment from expanding just because its
rollout percentage is small.

| Report field | Stage | Source |
| --- | --- | --- |
| `experiment_kill_switch.name` | report-only | `ExperimentFlag::name` |
| `experiment_kill_switch.enabled` | report-only | `ExperimentFlag::enabled` |
| `experiment_kill_switch.owner` | report-only | `ExperimentFlag::owner` |
| `experiment_kill_switch.kill_switch_documented` | report-only | kill-switch evidence |
| `experiment_kill_switch.rollback_report_present` | report-only | rollback report evidence |
| `experiment_kill_switch.rollback_resume_gate_present` | report-only | resume gate evidence |
| `experiment_kill_switch.context_rot_clean` | report-only | Context Rot decision |
| `experiment_kill_switch.owner_acknowledged` | report-only | owner acknowledgement |
| `experiment_kill_switch.kill_switch_blocked` | enforced | `ExperimentKillSwitchGate::evaluate` |
| `experiment_kill_switch.failure_reasons` | enforced | kill-switch gate reasons |
| `experiment_kill_switch.allow_experiment_expansion` | enforced | inverted gate decision |

Disabled flags are non-blocking by default. Enabled flags require the escape
hatch, rollback path, clean Context Rot, and owner acknowledgement before
expansion.

`experiment_expansion_safety_report_v1` is the aggregate gate before expanding
an enabled experiment such as the Apple Silicon model pool. It joins rollout,
kill-switch, Context Rot, rollback resume, model-pool attribution, adapter
report emission, Apple Silicon development-effect, promotion window,
next-round readiness, Steam case matrix, validation command coverage, and
root-adapter attribution evidence.

| Report field | Stage | Source |
| --- | --- | --- |
| `experiment_expansion.enabled_flag_names` | report-only | `ExperimentExpansionSafetyEvidence::enabled_flag_names` |
| `experiment_expansion.rollout_report_passed` | report-only | rollout report evidence |
| `experiment_expansion.kill_switch_report_passed` | report-only | kill-switch report evidence |
| `experiment_expansion.context_rot_remediation_passed` | report-only | Context Rot remediation evidence |
| `experiment_expansion.rollback_resume_passed` | report-only | rollback resume evidence |
| `experiment_expansion.model_pool_attribution_passed` | report-only | model-pool attribution evidence |
| `experiment_expansion.adapter_report_emission_passed` | report-only | adapter report-emission evidence |
| `experiment_expansion.adapter_report_field_coverage_passed` | report-only | no missing required adapter report fields |
| `experiment_expansion.apple_silicon_development_effect_passed` | report-only | Apple Silicon effect evidence |
| `experiment_expansion.promotion_window_passed` | report-only | promotion window evidence |
| `experiment_expansion.readiness_passed` | report-only | next-round readiness evidence |
| `experiment_expansion.steam_case_matrix_passed` | report-only | Steam case matrix evidence |
| `experiment_expansion.validation_command_coverage_passed` | report-only | validation command coverage evidence |
| `experiment_expansion.root_adapter_failure_kind` | report-only | root adapter attribution |
| `experiment_expansion.allow_experiment_expansion` | enforced | inverted safety gate decision |

Enforced expansion requires adapter report emission, adapter report-field
coverage, and Apple Silicon development-effect gates to pass. The upstream
emission, promotion, handoff, and current-runner gates must also show no
missing required report fields before enforcement. This keeps multi-model pool
experiments from expanding merely because worker attribution passed while
report ordering, future-event coverage, or worker metric coverage is still
incomplete.

`experiment_switch_matrix_report_v1` is the multi-experiment view above the
per-experiment expansion report. It records every known flag, enabled flags,
which enabled flags have an expansion safety report, duplicate report
projections, unknown or disabled flag references, which reports are blocked, and
whether all enabled flags have exactly one report. Report-only rollout uses it
as a switchboard inventory; enforced rollout may block when an enabled flag has
no expansion safety report, when more than one report claims the same enabled
flag, when a report references an unknown or disabled flag, or when any
expansion safety report is blocked. This is the eval-side guard that keeps Apple
Silicon model-pool flags, tester pools, and future helper roles measurable as a
set instead of becoming independent toggles.

| Report field | Stage | Source |
| --- | --- | --- |
| `experiment_switch.enabled_flag_names` | report-only | `ExperimentSwitchMatrixEvidence::enabled_flag_names` |
| `experiment_switch.reported_enabled_flag_names` | report-only | `ExperimentSwitchMatrixEvidence::reported_enabled_flag_names` |
| `experiment_switch.missing_enabled_flag_reports` | report-only | `ExperimentSwitchMatrixEvidence::missing_enabled_flag_reports` |
| `experiment_switch.duplicate_enabled_flag_reports` | report-only | duplicate claims across expansion safety reports |
| `experiment_switch.unknown_enabled_flag_reports` | report-only | reports that name unknown or disabled flags |
| `experiment_switch.exactly_one_report_per_enabled_flag` | report-only | missing, duplicate, and unknown report checks |
| `experiment_switch.all_expansion_reports_passed` | report-only | per-flag expansion safety decisions |
| `experiment_switch.allow_experiment_switch_expansion` | enforced | inverted matrix gate decision |

## Feedback And Self-Improve Report

The current ledger already carries feedback and self-improve evidence through
`feedback_applied`, `validation_checked`, `validation_passed`,
`self_improve_checked`, and `self_improve_passed`. The
`feedback_self_improve_report_v1` schema makes that closed-loop evidence visible
without moving ledger IO into `norion-eval`.

Report-only fields are direct projections from `LedgerSummary`:

| Report field | Stage | Source |
| --- | --- | --- |
| `feedback.total_applied` | report-only | `LedgerSummary::feedback_applied_total` |
| `feedback.items` | report-only | `LedgerSummary::feedback_items` |
| `feedback.validation_checked` | report-only | `LedgerSummary::validation_checked` |
| `feedback.validation_passed` | report-only | `LedgerSummary::validation_passed` |
| `feedback.self_improve_checked` | report-only | `LedgerSummary::self_improve_checked` |
| `feedback.self_improve_passed` | report-only | `LedgerSummary::self_improve_passed` |
| `feedback.self_improve_pass_rate` | report-only | `LedgerSummary::self_improve_pass_rate()` |
| `feedback.closed_loop_blocked` | enforced | `ReportGate::evaluate()` |
| `feedback.failure_reasons` | enforced | `ReportGate::evaluate()` reasons |

`FeedbackSelfImproveReport::from_summary_and_decision` is a pure projection of
the summary and report-gate decision. Adapters that already have a summary can
use `from_summary_and_gate()`, while adapters holding normalized ledger rows can
use `from_records_and_gate()`. In report-only mode it should be emitted as
observability. In enforced mode it can block scheduling only when the existing
`ReportGate` has already identified missing feedback, failed validation, or a
self-improve pass-rate failure.

## Self-Evolution Continuity Report

`self_evolution_continuity_report_v1` checks that feedback from one round is
actually carried into the next round's self-improve path.
`SelfEvolutionContinuityReport::from_records_and_gate()` is the adapter-facing
projection when the runner already has adjacent ledger records.
The adapter remains responsible for reading ledger JSONL and normalizing it into
`LedgerRecord` values; `norion-eval` owns only the record-window projection and
gate decision.

| Report field | Stage | Source |
| --- | --- | --- |
| `self_evolution.previous_round` | report-only | previous `LedgerRecord::round` |
| `self_evolution.current_round` | report-only | current `LedgerRecord::round` |
| `self_evolution.previous_feedback_applied` | report-only | previous `LedgerRecord::feedback_applied` |
| `self_evolution.feedback_carried_forward` | report-only | continuity evidence |
| `self_evolution.self_improve_checked` | report-only | current `LedgerRecord::self_improve_checked` |
| `self_evolution.self_improve_passed` | report-only | current `LedgerRecord::self_improve_passed` |
| `self_evolution.validation_passed` | report-only | current `LedgerRecord::validation_passed` |
| `self_evolution.continuity_blocked` | enforced | `SelfEvolutionContinuityGate::evaluate()` |
| `self_evolution.failure_reasons` | enforced | continuity gate reasons |
| `self_evolution.allow_next_round` | enforced | inverted continuity decision |

In enforced rollout, adjacent rounds must remain contiguous, previous feedback
must be followed by a checked and passing self-improve round, and validation
must pass before another unattended round is scheduled.

## Self-Evolution Regression Report

`self_evolution_regression_report_v1` checks the recent self-evolution window
for regression before unattended continuation. Continuity proves feedback moved
from one round to the next; regression proves the recent window is not losing
feedback, validation, or self-improve quality.
`SelfEvolutionRegressionReport::from_records_and_gate()` keeps the window
summary, gate decision, and report projection inside `norion-eval`.
This is the stable window contract for future `tools/evolution-loop` adapters:
IO, parsing, and record selection stay outside the crate, while regression math
and enforced/report-only blocking semantics stay inside the crate.

| Report field | Stage | Source |
| --- | --- | --- |
| `self_evolution_regression.rounds` | report-only | window length |
| `self_evolution_regression.first_round` | report-only | first `LedgerRecord::round` |
| `self_evolution_regression.last_round` | report-only | latest `LedgerRecord::round` |
| `self_evolution_regression.feedback_delta` | report-only | latest feedback minus first feedback |
| `self_evolution_regression.validation_pass_rate` | report-only | window validation pass rate |
| `self_evolution_regression.self_improve_pass_rate` | report-only | window self-improve pass rate |
| `self_evolution_regression.tail_validation_failures` | enforced | consecutive validation failures at window tail |
| `self_evolution_regression.tail_self_improve_failures` | enforced | consecutive self-improve failures at window tail |
| `self_evolution_regression.regression_blocked` | enforced | `SelfEvolutionRegressionGate::evaluate()` |
| `self_evolution_regression.allow_unattended_continuation` | enforced | inverted regression decision |
| `self_evolution_regression.failure_reasons` | enforced | regression gate reasons |

In enforced rollout, the window must be long enough, validation and
self-improve pass rates must stay at threshold, feedback must not regress, and
the latest rounds must not end with validation or self-improve failures.

## Advice Continuation Report

`advice_continuation_report_v1` is the pure-data report surface for
higher-level continuation advice that the runner may extract from summaries,
operators, or follow-up plans. Eval only accepts normalized
`AdviceContinuationObservation` values plus `LedgerSummary`; it does not scan
report directories, replay JSONL, inspect runner state, call a model, restart a
daemon, or execute commands.

The report records repeated advice, invalid advice, invalid commands, ledger
round count, and whether the latest ledger round succeeded before continuation
was suggested. In enforced rollout, `AdviceContinuationGate` may still block the
advice-continuation report itself. When this report is lifted into
`readiness_next_round_v1` or
`self_evolution_unattended_prerequisites_report_v1`, it is only an additive
observable input: downstream reports may project whether continuation advice was
observed and whether it looked clean, but they must not add
`advice_continuation_report_v1` to the enforced report bundle or let it become a
new direct blocker for `readiness.can_schedule_next_round` or
`self_evolution_unattended.prerequisite_blocked`.

The current manifest and report-bundle contracts deliberately exclude
`advice_continuation_report_v1`. It is an additive, report-only observation
surface for higher-level continuation guidance, not a handoff-required schema
and not part of the enforced bundle completeness claim.

## Self-Evolution Unattended Prerequisites

`self_evolution_unattended_prerequisites_report_v1` is the aggregate claim
guard for "the loop can keep self-evolving unattended." It does not replace
`readiness_next_round_v1`; it records whether the already-emitted continuity,
regression, readiness, advice-continuation observation, Context Rot
trend/remediation, rollback-resume, Steam matrix, validation coverage,
promotion-window, adapter report-field coverage, and Apple Silicon effect
reports all passed before an unattended self-evolution claim is accepted.

| Report field | Stage | Source |
| --- | --- | --- |
| `self_evolution_unattended.stage` | report-only | `SelfEvolutionUnattendedPrerequisiteGate::stage` |
| `self_evolution_unattended.continuity_passed` | report-only | `self_evolution_continuity_report_v1` |
| `self_evolution_unattended.regression_passed` | report-only | `self_evolution_regression_report_v1` |
| `self_evolution_unattended.readiness_next_round_passed` | report-only | `readiness_next_round_v1` |
| `self_evolution_unattended.advice_continuation_observed` | report-only | `AdviceContinuationReport::allow_continuation observed` |
| `self_evolution_unattended.advice_continuation_passed` | report-only | `AdviceContinuationReport::allow_continuation` |
| `self_evolution_unattended.context_rot_trend_passed` | report-only | `context_rot_trend_report_v1` |
| `self_evolution_unattended.context_rot_remediation_passed` | report-only | `context_rot_remediation_report_v1` |
| `self_evolution_unattended.rollback_resume_passed` | report-only | `rollback_resume_report_v1` |
| `self_evolution_unattended.steam_case_matrix_passed` | report-only | `steam_case_matrix_report_v1` |
| `self_evolution_unattended.validation_command_coverage_passed` | report-only | `validation_command_coverage_report_v1` |
| `self_evolution_unattended.promotion_window_passed` | report-only | `adapter_promotion_window_report_v1` |
| `self_evolution_unattended.adapter_report_field_coverage_passed` | report-only | `AdapterReportEmissionReport::field_coverage_passed()` |
| `self_evolution_unattended.apple_silicon_development_effect_passed` | report-only | `apple_silicon_development_effect_report_v1` |
| `self_evolution_unattended.prerequisite_blocked` | enforced | `SelfEvolutionUnattendedPrerequisiteGate::evaluate()` |
| `self_evolution_unattended.allow_unattended_self_evolution_claim` | enforced | inverted prerequisite decision |
| `self_evolution_unattended.failure_reasons` | enforced | prerequisite gate reasons |

`AdviceContinuationReport` is intentionally additive here. The aggregate report
should record whether advice continuation was observed and whether the
continuation advice looked clean, but an observed failure must stay outside the
enforced prerequisite decision until the architecture explicitly promotes it.
Current tests require `advice_continuation_passed=false` to remain visible while
`prerequisite_blocked=false`.

In enforced rollout, any missing recovery proof, Steam coverage, validation
coverage, promotion-window proof, adapter report-field coverage, or Apple
Silicon effect proof blocks the self-evolution claim even if the local
continuity/regression windows look healthy. This keeps "the latest rounds
looked stable" separate from "the runner is ready to evolve unattended."

## Self-Improve Proposal Business Acceptance

`self_improve_proposal_acceptance_v1` remains the compatibility report for
clean self-improve proposal handling. A quarantined proposal with reasons may
still pass that legacy handling surface, because it proves the proposal was
classified safely. It is not the same as an accepted business improvement.

Use `self_improve_proposal.evidence_backed_business_improvement` when a
downstream adapter needs to claim that a model-suggested change became an
accepted unattended-evolution improvement. That field is true only when the
proposal has a source round, safe evidence ids, checked and passed validation,
a safe validation command source, a clean gist, no runtime side effects, and an
accepted memory-admission decision with reasons. Report-only suggestions and
quarantined candidates must remain `advisory_only=true` and must not be counted
as accepted business changes.

`SelfImproveProposalAcceptanceSummaryReport::from_reports` is the aggregate
surface for that distinction. It counts projected proposal reports, accepted
memory-admission candidates, evidence-backed business improvements, advisory
items, promotion-allowed items, repair-required items, and accepted candidates
that still lack business evidence. `tools/evolution-loop` emits this additively
as `self_improve_proposal_acceptance_summary_v1`; consumers should use the
summary count instead of scanning prose or treating all proposals as progress.

`SelfImproveProposalPromptGuidance::from_summary` is the pure decision surface
for feeding that summary back into the next unattended prompt. It mirrors the
summary counts and derives whether the next self-improve prompt should convert
advisory suggestions into evidence-backed business improvements, repair
unvalidated or accepted-without-evidence proposals, and require checked/passed
validation plus accepted memory admission. Runner-side code owns ledger parsing
and prompt string formatting only. Report surfaces may serialize the guidance
under `self_improve_proposal_acceptance_summary_v1.prompt_guidance`, but they
must not reimplement the guidance booleans outside eval.

`SelfImproveProposalActionPlan::from_summary` and `from_guidance` turn the same
pure guidance into a stable report-only action list. The plan reports whether
an action is required, the primary action, the ordered action ids, and whether
checked/passed validation plus accepted memory admission are required. It does
not apply proposals or mutate runtime state. `tools/evolution-loop` serializes
it under `self_improve_proposal_acceptance_summary_v1.action_plan` with
`auto_apply=false` and read-only side effects, so operators and downstream
status consumers can see the next concrete step without scraping prompt text.

`SelfImproveProposalActionAssignment::from_reports_and_plan` turns accepted
proposal reports plus that plan into explicit report-only targets.
`SelfImproveProposalActionAssignment::first_target_digest` projects the first
target into stable fields for prompt, status, and Forge consumers:
proposal id, source round, evidence ids, memory admission decision,
validation state, business-evidence state, advisory/repair flags, and missing
requirements. The `self_improve_proposal_action_assignment_v1` schema documents
those fields as report-only pure data. Consumers should format or display the
digest, not reopen raw proposal text or execute validation/model/memory work.
`norion_test::SelfImproveProposalActionAssignmentPlan` is the matching
acceptance-plan source for this schema, so eval and test contracts must keep
entrypoints, allowed inputs, produced outputs, report fields, and forbidden
capabilities in lockstep.

`self_improve_proposal_repair_factor_queue_v1` and
`self_improve_proposal_repair_factor_readiness_report_v1` are the DNA-inspired
repair-factor bridge for the same assignment surface. The queue relabels
targets into report-only `repair_factor:*` records. The readiness report then
checks whether each factor has evidence ids, a concrete target action, a real
old-label to new-label transition, and checked/passed validation before the
next prompt treats it as ready for a relabel-and-repair plan. Missing business
requirements remain repair objectives; missing evidence, validation, target
action, or repair labels remain blocked reasons. Both reports are read-only,
candidate-only, `auto_apply=false`, and do not authorize memory, NDKV, genome,
or adaptive-state writes.

When a repair factor is ready, the evolution-loop adapter now treats it as an
alternate `self_improve_proposal_memory_admission_request_report_v1` source for
the same proposal id. The request action is
`request_repair_factor_memory_admission`, uses the repair factor's evidence ids,
and replaces the older action-closure-blocked request candidate for that
proposal. This releases the repair factor into the writer-plan/dry-run/approval
queue without bypassing writer gates: memory-store and NDKV writes remain false
until an explicit admission writer and approval record authorize them.

## Ledger Gate Report Schema

`ledger_gate_report_v1` is the broad report-gate surface for the current
evolution ledger. It should be produced from `LedgerSummary::from_records` and
the `ReportGate::evaluate` decision via `LedgerGateReport::from_summary_and_gate`,
or directly from normalized ledger rows via `from_records_and_gate`.
It does not parse JSONL or read files; the runner-side adapter owns those steps.
`tools/evolution-loop` now emits this report additively in its report JSON from
the current summary adapter. While that adapter preserves legacy threshold
wording, it should use `from_summary_and_failure_reasons`; the legacy
`report_gate` field remains the current runner's authoritative stop surface for
helper, remote-chain, and model-pool policy checks that are outside the pure
ledger gate.

| Report field | Stage | Source |
| --- | --- | --- |
| `ledger.total_rounds` | report-only | `LedgerSummary::total_rounds` |
| `ledger.success_rate` | report-only | `LedgerSummary::success_rate()` |
| `ledger.validation_pass_rate` | report-only | `LedgerSummary::validation_pass_rate()` |
| `ledger.rust_check_checked` | report-only | `LedgerSummary::rust_check_checked` |
| `ledger.rust_check_passed` | report-only | `LedgerSummary::rust_check_passed` |
| `ledger.rust_check_feedback_applied_total` | report-only | `LedgerSummary::rust_check_feedback_applied_total` |
| `ledger.runtime_response_failures` | report-only | `LedgerSummary::runtime_response_failures` |
| `ledger.stream_truncations` | report-only | `LedgerSummary::stream_truncations` |
| `ledger.missing_final_failures` | report-only | `LedgerSummary::missing_final_failures` |
| `ledger.duplicate_rounds` | report-only | `LedgerSummary::duplicate_rounds` |
| `ledger.round_gaps` | report-only | `LedgerSummary::round_gaps` |
| `ledger.state_gate_pass_rate` | report-only | `LedgerSummary::state_gate_pass_rate()` |
| `ledger.trace_gate_pass_rate` | report-only | `LedgerSummary::trace_gate_pass_rate()` |
| `ledger.context_noise_penalty_max` | report-only | `LedgerSummary::context_noise_penalty_max` |
| `ledger.last_success` | report-only | `LedgerSummary::last_success` |
| `ledger.gate_blocked` | enforced | `ReportGate::evaluate()` |
| `ledger.failure_reasons` | enforced | `ReportGate::evaluate()` reasons |
| `ledger.allow_next_round` | enforced | inverted report-gate decision |

This report is the adapter-friendly replacement for duplicating report-gate
logic in each runner. In report-only mode it makes failures visible. In enforced
mode `ledger.allow_next_round=false` is the stop signal for ledger hygiene,
runtime response, stream/final, validation, feedback, self-improve, state/trace,
Context Rot penalty, and latest-round policy failures.

## Multi-Model Pool Eval

When multiple models participate in a round, each participant should emit a
`ModelWorkerRecord` before any merged answer is accepted. Roles should stay
small and stable:

- `planner`: proposes decomposition and next action.
- `reviewer`: challenges the proposed change and names risks.
- `tester`: chooses or synthesizes validation commands and expected evidence.
- `summarizer`: compresses round evidence for the next prompt context.
- `high_quality`: the primary 12B-quality path or another trusted arbiter.

The model-pool gate should be evaluated before the merged result can unblock the
main 12B path. Strict defaults require every worker to succeed, have validation
checked and passed, contribute at least one feedback update across the pool, and
produce zero duplicate outputs, noisy outputs, or primary-12B blocking events.
Optional per-worker caps can bound latency and runtime token cost.

For Apple Silicon, add a second question: did the pool improve development
effect relative to the primary 12B baseline? `AppleSiliconDevelopmentGate`
compares the pool with the baseline on:

- total latency and runtime tokens, bounded by configurable multipliers;
- success rate and validation pass rate, which must not regress from a passing
  12B baseline;
- feedback delta, which must be positive by default;
- duplicate outputs, noisy outputs, and primary-12B blockers, which default to
  zero tolerance.

This lets small local workers act as planner/reviewer/tester/summarizer only
when they add useful verified signal without starving or confusing the 12B path.

`model_pool_budget_fairness_report_v1` keeps the pool composition measurable
before an Apple Silicon experiment expands. In report-only rollout it must show
the worker count, success, feedback, latency, runtime tokens, runtime-token
share, missing required roles, and dominant runtime-token roles for each
configured role. In enforced rollout, missing planner/reviewer/tester coverage,
one role consuming more than the configured token share, or any primary-12B
blocking event prevents the pool from claiming development improvement.

`norion-test` owns the plan side of this flow through `ModelPoolSmokePlan`;
`norion-eval` owns the evidence side through `ModelWorkerRecord`. The adapter
between them should preserve worker IDs and roles exactly so a failed gate can
point back to the worker plan that produced the bad evidence.

`ModelPoolDevelopmentReport::from_summary_effect_and_decisions` projects the
pool summary, Apple Silicon baseline comparison, model-pool gate decision, and
development-effect gate decision into one report object. It carries latency and
token totals, success and validation pass rates, feedback delta, duplicate/noisy
output counts, primary-12B blockers, and the root-adapter failure kind. The
root-adapter value is attribution metadata: `chain_not_ready` or
`model_unavailable` must remain operational readiness evidence and must not be
counted as model quality failure.

## Report And Worker Event Schema

The future runner should add report fields with stable names so CI, Web Lab, and
runbooks can compare results across runs. `ModelPoolReportSchema` fixes the
initial field set:

| Report field | Stage | Source |
| --- | --- | --- |
| `model_pool.workers` | report-only | `ModelPoolSummary::workers` |
| `model_pool.success_rate` | report-only | `ModelPoolSummary::success_rate()` |
| `model_pool.validation_pass_rate` | report-only | `ModelPoolSummary::validation_pass_rate()` |
| `model_pool.latency_ms_total` | report-only | `ModelPoolSummary::latency_ms_total` |
| `model_pool.runtime_tokens_total` | report-only | `ModelPoolSummary::runtime_tokens_total` |
| `model_pool.feedback_applied_total` | report-only | `ModelPoolSummary::feedback_applied_total` |
| `model_pool.operational_readiness_failures` | report-only | per-worker `chain_not_ready` + `model_unavailable` |
| `model_pool.model_quality_failures` | report-only | per-worker `model_quality_failure` |
| `model_pool.duplicate_outputs` | enforced | `ModelPoolSummary::duplicate_outputs` |
| `model_pool.noisy_outputs` | enforced | `ModelPoolSummary::noisy_outputs` |
| `model_pool.primary_12b_blockers` | enforced | `ModelPoolSummary::primary_12b_blockers` |
| `model_pool.development.feedback_delta` | enforced | pool feedback minus baseline feedback |
| `model_pool.development.latency_multiplier` | enforced | pool latency divided by baseline latency |
| `model_pool.development.token_multiplier` | enforced | pool tokens divided by baseline tokens |
| `root_adapter.failure_kind` | report-only | `RootAdapterFailureKind::as_code()` |

Future per-worker evidence should use the `model_worker_v1` event schema. The
projection fields are enough for shadow/report-only mode; enforcement also
requires the safety fields:

| Worker event field | Projection | Enforcement |
| --- | --- | --- |
| `worker_id` | required | required |
| `role` | required | required |
| `model` | required | required |
| `latency_ms` | required | required |
| `runtime_tokens` | required | required |
| `success` | required | required |
| `feedback_applied` | required | required |
| `validation_checked` | required | required |
| `validation_passed` | required | required |
| `duplicate_output` | optional | required |
| `noisy_output` | optional | required |
| `blocked_primary_12b` | optional | required |
| `failure_kind` | optional | required |
| `development_claim_allowed` | report-only | required |
| `claim_blockers` | report-only | required |

`ModelWorkerGateReportSchema::model_worker_gate_v1()` is the per-worker gate
report that turns those events into an Apple Silicon development-effect claim.
It records worker ids, roles, model names, latency, runtime tokens, success,
feedback, validation, duplicate output, noisy output, and primary-12B blocking
as worker-indexed arrays. It also records per-worker `failure_kind` values so
parallel workers can distinguish `chain_not_ready`, `model_unavailable`, and
`model_quality_failure` before their outputs are merged. The report may only set
`model_worker.allow_development_claim=true` when the model-pool gate passes,
the Apple Silicon development gate passes, and root adapter attribution is
`none`. It also emits per-worker `development_claim_allowed` and
`claim_blockers` arrays so a pool cannot hide one weak worker behind aggregate
success. Blockers include missing latency or token evidence, no applied
feedback, unchecked or failed validation, duplicate/noisy output, primary-12B
blocking, operational readiness failure, and classified model-quality failure.

Operational readiness failures are visible in the same report but are not
quality failures:

- prompt-gate blocked, including when backend 8686 is also down, is
  `chain_not_ready`;
- prompt-gate passed but backend 8686 is down is `model_unavailable`;
- both values set `model_worker.operational_readiness_blocked=true`;
- neither value may set `model_worker.model_quality_failure_counted=true`.

`model_quality_failure` is counted only after final JSON exists, runtime model
and token evidence exist, and the business-cycle verdict failed. Until then,
Apple Silicon pool results can be observed, but they cannot be used to claim the
pool improved development.

## Apple Silicon Baseline Adapter Wiring

`AppleSiliconBaselineAdapterPlan::root_business_cycle_json()` is the pre-wiring
contract for deciding whether the Apple Silicon model pool actually improves
development compared with the primary 12B path. The current runner can project
pool-side fields, but it cannot make an enforced paired-baseline claim until
future baseline events exist.

Current strong or weak projections:

| `AppleSiliconBaselineComparisonRecord` field | Source | Strength | Enforcement note |
| --- | --- | --- | --- |
| `round` | current ledger `round` | strong | pairing key is already present |
| `pool_feedback_applied_total` | sum of `ModelWorkerRecord.feedback_applied` | strong | requires worker projection |
| `pool_latency_ms_total` | sum of `ModelWorkerRecord.latency_ms` | weak | only comparable after prompt/backend readiness |
| `pool_runtime_tokens_total` | sum of `ModelWorkerRecord.runtime_tokens` | weak | missing tokens during outage is readiness, not quality |
| `pool_success` | worker success plus `business_cycle.passed=true` | strong | enforce only after worker rows exist |
| `pool_validation_passed` | all required worker validation rows pass | strong | enforce only after worker validation coverage exists |
| `duplicate_outputs` | `ModelWorkerRecord.duplicate_output` | weak | needs worker output fingerprints |
| `noisy_outputs` | `ModelWorkerRecord.noisy_output` | weak | needs worker noise or Context Rot verdict |
| `primary_12b_blockers` | `ModelWorkerRecord.blocked_primary_12b` | weak | needs scheduler wait or blocking reason |
| `root_adapter_failure_kind` | root adapter classifier | strong | readiness failures block claims without counting as quality |

Required future events before enforcement:

| Future event | Target field |
| --- | --- |
| `baseline_12b_feedback_applied` | `baseline_12b_feedback_applied` |
| `baseline_12b_latency_ms` | `baseline_12b_latency_ms` |
| `baseline_12b_runtime_tokens` | `baseline_12b_runtime_tokens` |
| `baseline_12b_success` | `baseline_12b_success` |
| `baseline_12b_validation_passed` | `baseline_12b_validation_passed` |

Rollout order is additive until the final stage:

1. `shadow-project-pool-side-baseline-fields`: compute pool-side projection
   from current ledger and worker rows, but emit no development-gain claim.
2. `report-only-paired-baseline-coverage`: report missing baseline events as
   coverage gaps, not model failures.
3. `report-only-apple-silicon-baseline-comparison`: emit the comparison report
   only when paired evidence is available; keep readiness failures out of
   quality counters.
4. `enforced-apple-silicon-baseline-comparison`: block only after paired
   baseline events, worker metrics, outage attribution, and crate/workspace
   tests are stable.

The outage order is unchanged for this comparison: prompt-gate blocked plus
8686 down is `chain_not_ready`; prompt-gate passed plus 8686 down is
`model_unavailable`; neither value may increment
`model_quality_failure_rounds`. `model_quality_failure` is valid only after
final JSON, runtime model/tokens, and a failed business-cycle verdict exist.

`worker_root_failure_consistency_report_v1` is the pre-wiring consistency check
between the root adapter classifier and a legacy single-worker projection. In
report-only mode it records `worker_root_consistency.worker_failure_kinds`,
`worker_root_consistency.root_adapter_failure_kind`,
`worker_root_consistency.single_worker_consistent`, and
`worker_root_consistency.operational_quality_confusion`. In enforced mode it
may block only when a synthetic single-worker row disagrees with the root
failure kind, or when an operational readiness failure is mixed with
`model_quality_failure`. This protects the Apple Silicon pool metrics from
treating prompt-gate blocks or 8686 outages as model quality.

`ModelPoolDevelopmentAttributionReportSchema::model_pool_development_attribution_v1()`
is the stricter attribution report for future model-pool expansion. It remains
pure eval data: the runner supplies `ModelWorkerRecord` values and classified
`RootAdapterFailureKind` values, then the report projects:

| Report field | Stage | Source |
| --- | --- | --- |
| `model_pool_attribution.worker_ids` | report-only | `ModelWorkerRecord::worker_id` |
| `model_pool_attribution.roles` | report-only | `ModelWorkerRecord::role` |
| `model_pool_attribution.latency_ms` | report-only | `ModelWorkerRecord::latency_ms` |
| `model_pool_attribution.runtime_tokens` | report-only | `ModelWorkerRecord::runtime_tokens` |
| `model_pool_attribution.success` | report-only | `ModelWorkerRecord::success` |
| `model_pool_attribution.feedback_applied` | report-only | `ModelWorkerRecord::feedback_applied` |
| `model_pool_attribution.validation_passed` | report-only | `ModelWorkerRecord::validation_passed` |
| `model_pool_attribution.duplicate_output` | report-only | `ModelWorkerRecord::duplicate_output` |
| `model_pool_attribution.noisy_output` | report-only | `ModelWorkerRecord::noisy_output` |
| `model_pool_attribution.blocked_primary_12b` | report-only | `ModelWorkerRecord::blocked_primary_12b` |
| `model_pool_attribution.failure_kinds` | report-only | `ModelWorkerRecord::failure_kind` |
| `model_pool_attribution.worker_development_claim_allowed` | report-only | per-worker claim blocker summary |
| `model_pool_attribution.worker_claim_blockers` | report-only | `ModelWorkerRecord::development_claim_blockers` |
| `model_pool_attribution.root_adapter_failure_kinds` | report-only | `RootAdapterFailureKind::as_code` |
| `model_pool_attribution.operational_readiness_failures` | report-only | `chain_not_ready` + `model_unavailable` |
| `model_pool_attribution.model_quality_failure_count` | report-only | classified model-quality failures |
| `model_pool_attribution.reported_model_quality_failures` | report-only | runner projection |
| `model_pool_attribution.attribution_blocked` | enforced | attribution gate decision |
| `model_pool_attribution.allow_development_claim` | enforced | attribution gate decision |

The enforced gate requires planner, reviewer, and tester roles before the Apple
Silicon pool can claim it improved development. Successful workers must carry
latency/tokens, validation must be checked and pass, feedback must be applied,
duplicate/noisy outputs must be zero, and no worker may block the primary 12B.
Every worker must also have an empty claim-blocker list; this catches cases
where total feedback or success looks acceptable while one reviewer/tester row
has no useful contribution, missing metrics, duplicate/noisy output, or a
primary-12B blocking event.
The reported quality-failure count must equal the classified
`model_quality_failure` count after readiness ordering, so operational failures
cannot leak into model-quality metrics.

`apple_silicon_development_effect_report_v1` repeats the readiness-order guard
at the final claim boundary. It exposes
`apple_silicon_effect.operational_readiness_failure_kind` and
`apple_silicon_effect.quality_failure_blocked_by_readiness_order` next to the
per-worker latency, runtime tokens, success, validation, duplicate/noisy output,
primary-12B blocking arrays, plus the same worker claim-blocker arrays from the
attribution report. For the 8686 outage cases, prompt-gate blocked plus 8686
down must produce `chain_not_ready`, while prompt-gate passed plus 8686 down
must produce `model_unavailable`; both set the readiness-order guard and neither
may increment model-quality failure counters.

`ModelPoolBudgetFairnessGate` adds a second expansion check before a larger
Apple Silicon worker pool is considered healthy. It groups `ModelWorkerRecord`
values by role and requires planned roles to contribute useful work instead of
letting one worker consume the whole budget. The default Apple Silicon gate
requires planner, reviewer, and tester contributions, per-role feedback, no
primary-12B blocking, and no role using more than 60% of the pool runtime
tokens. Optional total latency and token ceilings can be tightened during an
enforced rollout.

`ModelPoolBudgetFairnessReportSchema::model_pool_budget_fairness_v1()` fixes
the report fields for that check:

| Report field | Stage | Source |
| --- | --- | --- |
| `model_pool_budget.roles` | report-only | per-role contribution list |
| `model_pool_budget.workers_by_role` | report-only | workers per role |
| `model_pool_budget.successful_workers_by_role` | report-only | successful workers per role |
| `model_pool_budget.feedback_by_role` | report-only | feedback applied per role |
| `model_pool_budget.runtime_tokens_by_role` | report-only | runtime tokens per role |
| `model_pool_budget.latency_ms_by_role` | report-only | latency per role |
| `model_pool_budget.total_runtime_tokens` | report-only | total worker tokens |
| `model_pool_budget.total_latency_ms` | report-only | total worker latency |
| `model_pool_budget.max_role_runtime_token_share` | enforced | largest role token share |
| `model_pool_budget.fairness_blocked` | enforced | budget/fairness gate decision |
| `model_pool_budget.allow_pool_expansion` | enforced | inverted gate decision |
| `model_pool_budget.failure_reasons` | enforced | gate decision reasons |

This gate is intentionally separate from model quality attribution. It answers
whether the pool composition is worth expanding; root adapter readiness still
decides whether a round can be judged at all.

## Root Adapter Acceptance Contract

`RootAdapterAcceptanceContract` ties the root business-cycle field map, model
pool report schema, worker event schema, and handoff test gates into one staged
contract. It is still a pure data/decision object; it does not parse JSON or
change the runner.

The contract exists to prevent adapter rollout from accidentally treating weak
generation evidence as model quality:

| Stage | Runner blocking | Required fields | Root mappings allowed to enforce |
| --- | --- | --- | --- |
| `ShadowOnly` | no | none; projection may be computed in tests/logs | none |
| `ReportOnly` | no | report-only fields plus projection worker fields | none |
| `Enforced` | yes | all report fields plus enforcement worker fields | strong `$.business_cycle.*` mappings only |

`ReportOnly` and `Enforced` require outage attribution before failure labels are
used. In particular, prompt-gate blocked evidence is `chain_not_ready`, and
backend 8686 unavailable evidence is `model_unavailable`. Missing
`$.generate.runtime_model` or token evidence during either condition must not be
reported as `model_quality_failure`.

The enforced stage still keeps `$.generate.*` root mappings out of the
enforceable mapping set. They remain useful for runtime response and worker
projection after generation has started, but the adapter must prove prompt,
backend, final JSON, and runtime response readiness before any quality verdict
is allowed.

## Adapter Fixture Contract

`adapter_fixture_contract_report_v1` is the pre-wiring fixture gate for the root
business-cycle adapter. It does not parse files or run HTTP. A future adapter or
test harness supplies `AdapterFixtureCase` rows with root evidence, projection
checks, and the expected `RootAdapterFailureKind`.

The enforced gate requires fixtures for:

- `none`: clean success;
- `chain_not_ready`: prompt-gate blocked, even when 8686 is unreachable;
- `model_unavailable`: prompt-gate passed but backend 8686 is unreachable;
- `stream_or_final_missing`: terminal final JSON is missing;
- `runtime_response_missing`: runtime model or tokens are absent after
  generation should have run;
- `model_quality_failure`: final JSON exists, runtime evidence exists, and
  `business_cycle.passed=false`.

Every fixture must also check the root fixture itself, the `LedgerRecord`
projection, the synthetic `ModelWorkerRecord` projection, and the report bundle
projection. Operational readiness failures must not be expected or reported as
`model_quality_failure`.

Stable report fields:

| Report field | Stage | Source |
| --- | --- | --- |
| `adapter_fixture.stage` | report-only | `AdapterFixtureGate::stage` |
| `adapter_fixture.case_names` | report-only | `AdapterFixtureEvidence::case_names` |
| `adapter_fixture.required_failure_kinds` | report-only | `AdapterFixtureGate::required_failure_kinds` |
| `adapter_fixture.expected_failure_kinds` | report-only | `AdapterFixtureEvidence::expected_failure_kinds` |
| `adapter_fixture.actual_failure_kinds` | report-only | `AdapterFixtureEvidence::actual_failure_kinds` |
| `adapter_fixture.classification_mismatches` | report-only | `AdapterFixtureEvidence::classification_mismatches` |
| `adapter_fixture.missing_projection_cases` | report-only | `AdapterFixtureEvidence::missing_projection_cases` |
| `adapter_fixture.operational_quality_confusions` | report-only | `AdapterFixtureEvidence::operational_quality_confusions` |
| `adapter_fixture.fixture_blocked` | enforced | `AdapterFixtureGate::evaluate` |
| `adapter_fixture.failure_reasons` | enforced | `AdapterFixtureGate::evaluate reasons` |
| `adapter_fixture.allow_runner_wiring` | enforced | inverted gate decision |

This gate is stricter than legacy replay. Legacy JSONL can miss additive report
fields; adapter fixtures are the proof that new wiring classifies failures and
projects records consistently before it can block the current runner.

## Current Runner Compatibility Report

`current_runner_compatibility_report_v1` is the aggregate pre-wiring view. It
does not replace the individual gates; it records whether each required gate has
already passed before enforced adapter wiring is allowed to affect the current
runner.

Stable report fields:

| Report field | Stage | Source |
| --- | --- | --- |
| `current_runner.stage` | report-only | `CurrentRunnerCompatibilityGate::stage` |
| `current_runner.legacy_replay_passed` | report-only | legacy replay result |
| `current_runner.report_bundle_complete` | report-only | report bundle gate |
| `current_runner.schema_drift_passed` | report-only | schema drift gate |
| `current_runner.adapter_report_emission_passed` | report-only | adapter report emission gate |
| `current_runner.adapter_report_field_coverage_passed` | report-only | no missing required adapter report fields |
| `current_runner.adapter_future_event_coverage_passed` | report-only | adapter future-event coverage gate |
| `current_runner.model_pool_development_window_passed` | report-only | Apple Silicon development window gate |
| `current_runner.apple_silicon_development_effect_passed` | report-only | Apple Silicon development-effect aggregate gate |
| `current_runner.feedback_self_improve_passed` | report-only | feedback/self-improve gate |
| `current_runner.self_evolution_continuity_passed` | report-only | self-evolution continuity gate |
| `current_runner.self_evolution_regression_passed` | report-only | self-evolution regression gate |
| `current_runner.readiness_next_round_passed` | report-only | next-round readiness gate |
| `current_runner.self_evolution_unattended_prerequisites_passed` | report-only | unattended self-evolution prerequisite gate |
| `current_runner.context_rot_trend_passed` | report-only | Context Rot trend gate |
| `current_runner.context_rot_remediation_passed` | report-only | Context Rot remediation gate |
| `current_runner.rollback_resume_passed` | report-only | rollback resume gate |
| `current_runner.adapter_fixture_passed` | report-only | adapter fixture gate |
| `current_runner.steam_case_matrix_passed` | report-only | Steam case matrix gate |
| `current_runner.validation_command_coverage_passed` | report-only | validation command coverage gate |
| `current_runner.promotion_window_passed` | report-only | promotion window gate |
| `current_runner.handoff_passed` | report-only | handoff gate |
| `current_runner.evolution_loop_tests_passed` | report-only | runner test gate |
| `current_runner.workspace_tests_passed` | report-only | workspace test gate |
| `current_runner.compatibility_blocked` | enforced | `CurrentRunnerCompatibilityGate::evaluate` |
| `current_runner.failure_reasons` | enforced | compatibility gate reasons |
| `current_runner.allow_enforced_wiring` | enforced | inverted gate decision |

In `ReportOnly`, this report is a dashboard for remaining wiring debt. In
`Enforced`, schema drift, adapter report emission failure, missing adapter
report field coverage, missing adapter future-event coverage, model-pool window failure, Apple Silicon
development-effect failure, failed feedback/self-improve evidence, failed
self-evolution continuity/regression evidence, failed next-round readiness,
missing unattended self-evolution prerequisites, failed Context Rot
trend/remediation evidence, failed rollback-resume evidence, or any other
missing prerequisite blocks adapter wiring and keeps the existing
`tools/evolution-loop` runner authoritative.

`CurrentRunnerCompatibilityEvidence` is the corresponding thin lift point for
already-computed runner-adjacent reports. Besides
`with_handoff_report_pass_bits`, it supports direct report-to-bit helpers for
`AdapterReportEmissionReport`, `AdapterFutureEventCoverageReport`,
`EvalReportBundleGateReport`, `EvalSchemaDriftReport`, and
`AdapterFixtureReport`. These helpers only copy stable allow/pass bits from the
upstream reports; they do not execute commands, read files, or rebuild the
upstream gate decisions. `AdviceContinuationReport` remains intentionally
outside this direct current-runner input surface and only influences the system
through its additive/readiness projections.

## Root Adapter Attribution Report Schema

`root_adapter_attribution_report_v1` makes the root adapter classification
auditable before it affects quality metrics. It projects
`RootAdapterFailureEvidence` and `RootAdapterRolloutStage` through the same
`classify_root_adapter_failure` and rollback policy used by the gates.

| Report field | Stage | Source |
| --- | --- | --- |
| `root_adapter.backend_8686_reachable` | report-only | backend health evidence |
| `root_adapter.prompt_gate_blocked` | report-only | prompt-gate evidence |
| `root_adapter.final_json_present` | report-only | final JSON evidence |
| `root_adapter.runtime_model_present` | report-only | runtime response evidence |
| `root_adapter.runtime_tokens` | report-only | runtime response evidence |
| `root_adapter.business_cycle_passed` | report-only | business-cycle verdict |
| `root_adapter.failure_kind` | report-only | `classify_root_adapter_failure()` |
| `root_adapter.model_quality_failure_allowed` | enforced | stage and failure kind |
| `root_adapter.rollback_required` | enforced | root adapter rollback policy |
| `root_adapter.rollback_reason` | enforced | root adapter rollback policy |

The report preserves the attribution order: prompt-gate blocked evidence is
`chain_not_ready` even when backend 8686 is also unavailable; backend outage
after the prompt gate is `model_unavailable`; only final JSON with runtime
model/tokens and `business_cycle.passed=false` may set
`model_quality_failure_allowed=true` in the enforced stage.

## Root Adapter Rollback Policy

`rollback_plan_for_root_adapter_failure` maps root adapter failure kinds to
stable rollback actions. The policy deliberately separates readiness and
infrastructure failures from model quality failures:

| Failure kind | Rollback reason | Resume gate |
| --- | --- | --- |
| `chain_not_ready` | prompt chain or prompt gate blocked before generation | chain readiness gate |
| `model_unavailable` | backend 8686 or runtime model endpoint unavailable | runtime backend health check |
| `stream_or_final_missing` | stream ended without terminal final JSON | stream continuity validation |
| `runtime_response_missing` | runtime model or tokens missing after generation should have run | runtime response gate |
| `model_quality_failure` | final JSON exists and business-cycle verdict failed | model-quality review |
| `unknown` | evidence is incomplete or not classifiable | manual failure classification |

This keeps outage recovery operational. A down backend or blocked prompt chain
should pause enforcement and preserve evidence, but it should not consume the
model-quality failure budget or trigger quality rollback actions.

## Rollback Report Schema

`RollbackReportSchema::rollback_v1()` fixes the report surface for rollback
plans:

| Report field | Stage | Source |
| --- | --- | --- |
| `rollback.required` | report-only | `RollbackPlan::required` |
| `rollback.reason` | report-only | `RollbackPlan::reason` |
| `rollback.actions` | report-only | `RollbackPlan::actions` |
| `rollback.root_adapter_failure_kind` | report-only | `RootAdapterFailureKind::as_code()` |
| `rollback.resume_gate` | enforced | `RootAdapterFailureKind::resume_gate()` or planned validation |
| `rollback.stop_scheduling_new_rounds` | enforced | `RollbackPlan::actions` |

`RollbackReport::from_plan` projects either a root-adapter rollback or a generic
gate rollback into those fields. Root-adapter readiness and infrastructure
failures point to operational resume gates such as
`chain_readiness_gate` or `runtime_backend_health_check`; generic validation
rollbacks use `planned_validation_command`.

## Rollback Drill Matrix Report Schema

`RollbackDrillMatrixReportSchema::rollback_drill_matrix_v1()` fixes the
pre-enforcement drill surface for the root-adapter rollback policy. It proves
that every `RootAdapterFailureKind` maps to a stable rollback reason, stable
resume gate, and non-empty action list when rollback is required.

| Report field | Stage | Source |
| --- | --- | --- |
| `rollback_drill.stage` | report-only | `RollbackDrillMatrixGate::stage` |
| `rollback_drill.failure_kinds` | report-only | `RollbackDrillMatrixEvidence::failure_kinds()` |
| `rollback_drill.rollback_reasons` | report-only | `RollbackDrillMatrixEvidence::rollback_reasons()` |
| `rollback_drill.resume_gates` | report-only | `RollbackDrillMatrixEvidence::resume_gates()` |
| `rollback_drill.covered_failure_kinds` | report-only | `RollbackDrillMatrixEvidence::covered_failure_kinds()` |
| `rollback_drill.missing_failure_kinds` | report-only | `RollbackDrillMatrixEvidence::missing_failure_kinds()` |
| `rollback_drill.unstable_rollback_reasons` | report-only | `RollbackDrillMatrixEvidence::unstable_rollback_reasons()` |
| `rollback_drill.unstable_resume_gates` | report-only | `RollbackDrillMatrixEvidence::unstable_resume_gates()` |
| `rollback_drill.empty_action_failure_kinds` | report-only | `RollbackDrillMatrixEvidence::empty_action_failure_kinds()` |
| `rollback_drill.none_requires_rollback` | report-only | `RollbackDrillMatrixEvidence::none_requires_rollback()` |
| `rollback_drill.drill_blocked` | enforced | `RollbackDrillMatrixGate::evaluate()` |
| `rollback_drill.failure_reasons` | enforced | drill gate decision reasons |
| `rollback_drill.allow_enforced_rollback_policy` | enforced | inverted drill gate decision |

In enforced rollout, the drill blocks if any failure kind is missing, a root
adapter rollback reason differs from `RootAdapterFailureKind::as_code()`, a
resume gate differs from `RootAdapterFailureKind::resume_gate()`, a required
rollback has no actions, or `none` requires rollback. `chain_not_ready` and
`model_unavailable` remain operational recovery categories; they must never be
rewritten to `model_quality_failure`.

## Rollback Resume Report Schema

`RollbackResumeReportSchema::rollback_resume_v1()` fixes the report surface for
checking whether a rollback resume gate has passed before unattended evolution
rounds restart.
Adapters can project prebuilt evidence with
`RollbackResumeReport::from_evidence_for_stage()` when they only need the
rollout stage, or `from_gate_and_evidence()` when they already hold a configured
gate.

| Report field | Stage | Source |
| --- | --- | --- |
| `rollback_resume.stage` | report-only | `RollbackResumeGate::stage` |
| `rollback_resume.resume_gate` | report-only | `RollbackResumeEvidence::resume_gate` |
| `rollback_resume.chain_ready` | report-only | chain readiness evidence |
| `rollback_resume.backend_8686_reachable` | report-only | backend health evidence |
| `rollback_resume.stream_continuity_passed` | report-only | stream validation evidence |
| `rollback_resume.runtime_response_passed` | report-only | runtime response evidence |
| `rollback_resume.model_quality_review_passed` | report-only | quality review evidence |
| `rollback_resume.validation_command_passed` | report-only | planned validation evidence |
| `rollback_resume.steam_case_matrix_passed` | report-only | Steam case matrix evidence |
| `rollback_resume.validation_command_coverage_passed` | report-only | validation command coverage evidence |
| `rollback_resume.adapter_report_field_coverage_passed` | report-only | `AdapterReportEmissionReport::field_coverage_passed()` |
| `rollback_resume.manual_classification_done` | report-only | manual classification evidence |
| `rollback_resume.resume_blocked` | enforced | `RollbackResumeGate::evaluate()` |
| `rollback_resume.failure_reasons` | enforced | resume gate decision reasons |
| `rollback_resume.allow_unattended_rounds` | enforced | inverted resume gate decision |

`chain_readiness_gate` and `runtime_backend_health_check` are operational resume
gates. They can keep unattended rounds paused, but they do not count as
`model_quality_failure`; only `model_quality_review` uses the quality-review
resume path.
Even after the specific resume gate passes, unattended rounds remain paused
until Steam case matrix, validation command coverage, and adapter report-field
coverage evidence have passed.

## Self-Evolution Readiness Snapshot

`SelfEvolutionReadinessSnapshot` is the final pure decision combiner before a
future runner schedules another unattended round. It accepts decisions that have
already been computed elsewhere:

- `ReportGate` over `LedgerSummary`;
- `ContextRotAcceptanceContract` over experience-audit evidence;
- `ContextRotTrendGate` and `ContextRotRemediationGate` for cross-round rot
  and cleanup evidence;
- `ModelPoolGate` and `AppleSiliconDevelopmentGate` over worker evidence;
- `ExperimentRolloutGate` over enabled experiment flags;
- `ExperimentKillSwitchGate` and `ExperimentExpansionSafetyGate` before any
  enabled experiment expands;
- `AdapterReportEmissionGate` and `AppleSiliconDevelopmentEffectGate` as
  experiment-expansion inputs for Apple Silicon model-pool work;
- `RollbackResumeGate` before unattended rounds restart after any rollback;
- `SteamCaseCoverageGate` for the root business-cycle case matrix;
- `ValidationCommandCoverageGate` for post-round validation command evidence;
- `AdviceContinuationReport` as an advisory projection about higher-level
  continuation guidance;
- `RootAdapterFailureKind` from the root business-cycle adapter.

The snapshot does not parse ledgers or inspect HTTP responses. It only merges
the current decision layer and returns:

- `can_schedule_next_round`: false if any active gate blocks;
- `next_round_decision`: the merged reasons for the report gate;
- `rollback_plan`: a stable rollback plan, preferring root-adapter-specific
  rollback when enforced root adapter attribution produced a failure.

In `ReportOnly`, root adapter failures remain advisory. In `Enforced`, any
non-`none` root adapter failure blocks scheduling and maps through
`rollback_plan_for_root_adapter_failure`.

`SelfEvolutionReadinessReportSchema::next_round_v1()` fixes the output fields
for the final report surface:

| Report field | Stage | Source |
| --- | --- | --- |
| `readiness.stage` | report-only | `RootAdapterRolloutStage::as_str()` |
| `readiness.root_adapter_failure_kind` | report-only | `RootAdapterFailureKind::as_code()` |
| `readiness.advice_continuation_observed` | report-only | `SelfEvolutionReadinessSnapshot::advice_continuation_observed` |
| `readiness.advice_continuation_blocked` | report-only | `SelfEvolutionReadinessSnapshot::advice_continuation_decision` |
| `readiness.context_rot_trend_blocked` | enforced | Context Rot trend decision |
| `readiness.context_rot_remediation_blocked` | enforced | Context Rot remediation decision |
| `readiness.experiment_expansion_safety_blocked` | enforced | experiment expansion safety decision |
| `readiness.adapter_report_field_coverage_blocked` | enforced | adapter report field coverage decision |
| `readiness.rollback_resume_blocked` | enforced | rollback resume decision |
| `readiness.steam_case_matrix_blocked` | enforced | Steam case matrix decision |
| `readiness.validation_command_coverage_blocked` | enforced | validation command coverage decision |
| `readiness.can_schedule_next_round` | enforced | `SelfEvolutionReadinessSnapshot::can_schedule_next_round()` |
| `readiness.failure_reasons` | enforced | `SelfEvolutionReadinessSnapshot::next_round_decision()` |
| `readiness.rollback_required` | enforced | `SelfEvolutionReadinessSnapshot::rollback_plan()` |
| `readiness.rollback_reason` | enforced | `SelfEvolutionReadinessSnapshot::rollback_plan()` |
| `readiness.rollback_actions` | enforced | `SelfEvolutionReadinessSnapshot::rollback_plan()` |

`SelfEvolutionReadinessReport::from_snapshot` is a pure projection of the
snapshot into those fields. Future runner code can serialize that projection,
but serialization and JSON ownership stay outside `norion-eval`.

Advice continuation remains advisory at this layer. The readiness report may
surface whether continuation advice was observed and whether the advice gate
would have blocked, but `readiness.can_schedule_next_round` must continue to be
driven by the established enforced gates above, not by
`advice_continuation_report_v1`.

`norion-test::SelfEvolutionReadinessPlan` is the plan-side counterpart. It
requires the runner to provide decisions from `ReportGate`,
`ContextRotAcceptanceContract`, `ModelPoolGate`, `ExperimentRolloutGate`, and
`RootAdapterFailureKind` before the readiness projection is accepted.

For adapter-facing report inputs, keep using
`AdapterReadinessReportsInputContract::readiness_reports_v1()` and
`norion_test::AdapterReadinessReportsInputPlan`: the runner may pass already
computed `SteamRoundAcceptanceReport`, `SteamCaseCoverageReport`,
`ContextRotReport`, `ContextRotTrendReport`, `ContextRotRemediationReport`, and
`AdviceContinuationReport`, but it must not execute Steam HTTP/SSE, scan files,
spawn validation commands, call a model, or read runner state inside this
boundary. `AdapterReadinessReports` is the pure wrapper for those already
computed reports.

`AdapterEvidenceProjection` exposes thin convenience helpers over that same
boundary:

- `readiness_snapshot_with_steam_reports` for Steam round and case-matrix
  inputs.
- `readiness_report_with_steam_reports`,
  `rollback_report_with_steam_reports`, and
  `closure_report_with_steam_reports` for the same Steam report inputs when the
  runner wants finished readiness, rollback, or closure projections directly.
- `readiness_snapshot_with_context_reports` for Context Rot acceptance,
  trend, and remediation inputs.
- `readiness_report_with_context_reports`,
  `rollback_report_with_context_reports`, and
  `closure_report_with_context_reports` for the same Context Rot report inputs
  when the runner wants finished readiness, rollback, or closure projections
  directly.
- `readiness_snapshot_with_advice_continuation_report` for advisory
  continuation guidance.
- `readiness_report_with_advice_continuation_report`,
  `rollback_report_with_advice_continuation_report`, and
  `closure_report_with_advice_continuation_report` when the runner already has
  a normalized `AdviceContinuationReport` and wants the finished readiness,
  rollback, or closure projection without rebuilding `AdapterReadinessReports`
  by hand.

These helpers are only convenience entrypoints over the same pure-data adapter
contract. Advice continuation stays advisory across all of them: it may surface
`advice_continuation_observed` / `advice_continuation_blocked`, but it must not
become a new enforced stop signal for `readiness.can_schedule_next_round`,
`rollback.required`, or `adapter_closure.allow_next_round`.

## Steam Round Acceptance

`SteamRoundAcceptanceEvidence` and `SteamRoundAcceptanceGate` combine the
single-round evidence that the runner already gathers:

- `StreamContinuityCheck` for SSE `done`/`final`/buffer correctness;
- `ValidationObservation` for the post-round command result;
- `LedgerRecord` for runtime response and validation gate evidence;
- `SelfEvolutionReadinessSnapshot` for whether another unattended round can be
  scheduled.

`SteamRoundAcceptanceGate::strict()` blocks on stream failure, runtime response
failure, validation failure, and next-round readiness failure. It does not run
HTTP, parse SSE, execute cargo, or write reports. `norion-test` owns the
plan-side `SteamRoundAcceptancePlan`; `norion-eval` owns the pure decision.

`SteamRoundAcceptanceReportSchema::steam_round_v1()` fixes the report fields for
that decision:

| Report field | Stage | Source |
| --- | --- | --- |
| `steam_round.case_id` | report-only | `SteamRoundAcceptanceEvidence::case_id` |
| `steam_round.stream_passed` | report-only | `StreamContinuityCheck::passed()` |
| `steam_round.validation_checked` | report-only | `ValidationObservation::checked` |
| `steam_round.validation_passed` | report-only | `ValidationObservation::passed` |
| `steam_round.runtime_tokens` | report-only | `LedgerRecord::runtime_tokens` |
| `steam_round.runtime_model` | report-only | `LedgerRecord::runtime_model` |
| `steam_round.acceptance_blocked` | enforced | `SteamRoundAcceptanceGate::evaluate()` |
| `steam_round.failure_reasons` | enforced | `SteamRoundAcceptanceGate::evaluate()` reasons |
| `steam_round.can_schedule_next_round` | enforced | `SelfEvolutionReadinessSnapshot::can_schedule_next_round()` |

`SteamRoundAcceptanceReport::from_evidence` projects the evidence and gate
decision into that stable shape. The future runner can emit the projection in
report-only mode, then let the enforced fields drive stop/resume policy once the
adapter rollout reaches enforcement.

## Adapter Handoff Checklist

`AdapterHandoffChecklist` is the pre-wiring contract for moving these pure
decisions into `tools/evolution-loop`. It names the required schemas:

- `model_worker_v1`
- `model_worker_gate_report_v1`
- `worker_root_failure_consistency_report_v1`
- `model_pool_budget_fairness_report_v1`
- `model_pool_development_attribution_report_v1`
- `model_pool_development_window_report_v1`
- `apple_silicon_baseline_comparison_report_v1`
- `apple_silicon_development_effect_report_v1`
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
- `legacy_ledger_replay_report_v1`
- `feedback_self_improve_report_v1`
- `self_evolution_continuity_report_v1`
- `self_evolution_regression_report_v1`
- `self_evolution_unattended_prerequisites_report_v1`
- `readiness_next_round_v1`
- `steam_round_report_v1`
- `steam_case_matrix_report_v1`
- `validation_command_coverage_report_v1`
- `rollback_report_v1`
- `rollback_drill_matrix_report_v1`
- `adapter_handoff_report_v1`
- `report_bundle_gate_report_v1`
- `schema_drift_report_v1`
- `adapter_promotion_window_report_v1`
- `rollback_resume_report_v1`

It also names the handoff test gates:

- `cargo test --manifest-path crates/norion-test/Cargo.toml`
- `cargo test --manifest-path crates/norion-eval/Cargo.toml`
- `cargo test --manifest-path tools/evolution-loop/Cargo.toml`
- `cargo test --workspace`

In `ReportOnly`, the checklist remains crate-scoped and non-blocking. In
`Enforced`, it requires the runner test, workspace test, legacy ledger replay,
an observed report-only adapter output, a complete report bundle, a clean schema
drift report, passing adapter report emission, a passing Apple Silicon
development window and development-effect gate, passing Steam case matrix and
validation command coverage gates, and a passing promotion window before the
current runner may be blocked by the new gates.

`EvalSchemaManifest::evolution_loop_handoff_v1()` is the eval-side manifest for
the same schema set. It keeps schema names and rollout stage intent in one place
so the runner adapter, docs, and tests do not drift.

`EvalReportBundleManifest::for_stage` derives the report bundle expected at each
rollout stage. `ShadowOnly` emits no new bundle, `ReportOnly` emits only
observational schemas (`model_worker_v1`,
`worker_root_failure_consistency_report_v1`,
`apple_silicon_baseline_comparison_report_v1`,
`experiment_switch_matrix_report_v1`,
`rollback_drill_matrix_report_v1`, `context_rot_report_v1`, and
`adapter_future_event_coverage_report_v1`, and
`adapter_promotion_window_report_v1`), and `Enforced` emits the full schema set
including model worker gate, model-pool budget/fairness, model-pool development
window, Apple Silicon baseline comparison, ledger gate, experiment rollout,
root adapter attribution, legacy ledger replay, feedback and self-improve,
Context Rot remediation, readiness, Steam round, rollback, rollback drill,
self-evolution continuity, adapter handoff, report bundle gate, schema drift,
adapter promotion window, and rollback resume reports.

`EvalReportBundleGateReportSchema::report_bundle_gate_v1()` checks that the
future adapter actually emitted the bundle required for its rollout stage.

| Report field | Stage | Source |
| --- | --- | --- |
| `report_bundle.stage` | report-only | `EvalReportBundleManifest::stage` |
| `report_bundle.expected_schema_names` | report-only | `EvalReportBundleManifest::schema_names` |
| `report_bundle.observed_schema_names` | report-only | future adapter observation |
| `report_bundle.missing_schema_names` | report-only | `EvalReportBundleManifest::missing_from()` |
| `report_bundle.adapter_report_field_coverage_passed` | report-only | `AdapterReportEmissionReport::field_coverage_passed()` |
| `report_bundle.bundle_blocked` | enforced | `EvalReportBundleManifest::evaluate_bundle()` |
| `report_bundle.failure_reasons` | enforced | bundle gate decision reasons |
| `report_bundle.complete` | enforced | inverted bundle gate decision |

This gate is separate from legacy ledger replay. Old JSONL files may miss new
additive reports; a future report bundle for a wired adapter must not. In
enforced rollout, schema names being present is not enough: the bundle cannot
claim `complete=true` while `adapter_emission.missing_report_fields` is non-empty.

## Report Taxonomy Boundary

The current contract separates historical compatibility from future adapter
coverage:

| Taxonomy | Meaning | Representative schemas |
| --- | --- | --- |
| Additive and excluded | May be absent from old JSONL replay or direct handoff/current-runner input surfaces. These reports can add observability, or be projected through readiness, without becoming enforced bundle evidence by themselves. | `advice_continuation_report_v1`; legacy replay gaps such as future worker, Context Rot, readiness, rollback, handoff, bundle, schema drift, promotion, and resume reports |
| Report-only but required | Must be emitted as report-only evidence before enforced adapter wiring can claim coverage. In enforced rollout, the bundle/checklist/promotion gates may require the corresponding pass bit or stable observation window. | `model_worker_v1`, `worker_root_failure_consistency_report_v1`, `context_rot_report_v1`, `adapter_future_event_coverage_report_v1`, `adapter_promotion_window_report_v1`, plus enforced bundle schemas such as `ledger_gate_report_v1`, `schema_drift_report_v1`, `report_bundle_gate_report_v1`, `readiness_next_round_v1`, and `rollback_resume_report_v1` |

`AdviceContinuationReport` is the clearest additive/excluded example. It is
allowed to inform readiness as advisory context, but it remains outside
`EvalReportBundleManifest::for_stage(Enforced)`,
`AdapterHandoffChecklist::required_schemas`, and the direct current-runner or
handoff lift surfaces. A future `tools/evolution-loop` adapter must not wire it
as a required enforced bundle member or use it as a direct handoff blocker.

The current eval contract already reflects that boundary: direct handoff and
current-runner lifts copy stable pass bits from already-computed bundle, schema
drift, adapter emission, future-event coverage, and prerequisite reports. They
do not accept `AdviceContinuationReport` as a direct input. Continuation advice
can remain visible through readiness advisory fields, but it is not enforced
coverage by itself.

By contrast, report-only but required schemas are staging evidence. They may
start as report-only observations, but a wired adapter cannot claim an enforced
bundle is complete until `EvalReportBundleGateReport` and adapter report-field
coverage agree that the required schema names and fields are present. Legacy
ledger replay still treats their absence in old artifacts as additive gaps.

This distinction prevents additive readiness inputs from being mistaken for
enforced coverage. Future adapter code should route additive reports through
their documented projection path, such as readiness advisory bits, and should
route required schemas through the manifest, bundle gate, schema drift gate,
adapter report emission coverage, promotion window, and handoff checklist.

## Adapter Promotion Window Report Schema

`AdapterPromotionWindowReportSchema::adapter_promotion_window_v1()` records the
report-only observation window required before enforced adapter wiring. It keeps
operational readiness separate from model quality: `chain_not_ready` and
`model_unavailable` can block promotion, but they do not become
`model_quality_failure`.

| Report field | Stage | Source |
| --- | --- | --- |
| `promotion.stage` | report-only | `AdapterPromotionWindowGate::stage` |
| `promotion.observed_report_only_runs` | report-only | observation window evidence |
| `promotion.complete_bundle_runs` | report-only | bundle gate evidence |
| `promotion.adapter_report_emission_passed_runs` | report-only | adapter report emission gate evidence |
| `promotion.adapter_report_field_coverage_passed_runs` | report-only | adapter report field coverage evidence |
| `promotion.adapter_future_event_coverage_passed_runs` | report-only | adapter future-event coverage gate evidence |
| `promotion.apple_silicon_development_effect_passed_runs` | report-only | Apple Silicon development-effect report evidence |
| `promotion.apple_silicon_baseline_comparison_passed_runs` | report-only | Apple Silicon paired baseline comparison evidence |
| `promotion.experiment_switch_matrix_passed_runs` | report-only | experiment switch matrix evidence |
| `promotion.readiness_passed_runs` | report-only | next-round readiness report evidence |
| `promotion.context_rot_trend_passed_runs` | report-only | Context Rot trend report evidence |
| `promotion.context_rot_remediation_passed_runs` | report-only | Context Rot remediation report evidence |
| `promotion.rollback_resume_passed_runs` | report-only | rollback resume report evidence |
| `promotion.steam_case_matrix_passed_runs` | report-only | Steam case matrix gate evidence |
| `promotion.validation_command_coverage_passed_runs` | report-only | validation command coverage gate evidence |
| `promotion.model_quality_failure_runs` | report-only | root adapter attribution |
| `promotion.model_unavailable_runs` | report-only | root adapter attribution |
| `promotion.chain_not_ready_runs` | report-only | root adapter attribution |
| `promotion.worker_operational_readiness_failure_runs` | report-only | worker `failure_kind` attribution |
| `promotion.worker_model_quality_failure_runs` | report-only | worker `failure_kind` attribution |
| `promotion.worker_claim_blocker_runs` | report-only | worker claim-blocker evidence |
| `promotion.runtime_response_failure_runs` | report-only | root adapter attribution |
| `promotion.stream_or_final_missing_runs` | report-only | root adapter attribution |
| `promotion.min_report_only_runs` | report-only | promotion gate threshold |
| `promotion.min_complete_bundle_runs` | report-only | promotion gate threshold |
| `promotion.min_adapter_report_emission_passed_runs` | report-only | promotion gate threshold |
| `promotion.min_adapter_report_field_coverage_passed_runs` | report-only | promotion gate threshold |
| `promotion.min_adapter_future_event_coverage_passed_runs` | report-only | promotion gate threshold |
| `promotion.min_apple_silicon_development_effect_passed_runs` | report-only | promotion gate threshold |
| `promotion.min_apple_silicon_baseline_comparison_passed_runs` | report-only | promotion gate threshold |
| `promotion.min_experiment_switch_matrix_passed_runs` | report-only | promotion gate threshold |
| `promotion.min_readiness_passed_runs` | report-only | promotion gate threshold |
| `promotion.min_context_rot_trend_passed_runs` | report-only | promotion gate threshold |
| `promotion.min_context_rot_remediation_passed_runs` | report-only | promotion gate threshold |
| `promotion.min_rollback_resume_passed_runs` | report-only | promotion gate threshold |
| `promotion.min_steam_case_matrix_passed_runs` | report-only | promotion gate threshold |
| `promotion.min_validation_command_coverage_passed_runs` | report-only | promotion gate threshold |
| `promotion.promotion_blocked` | enforced | promotion gate decision |
| `promotion.failure_reasons` | enforced | promotion gate decision reasons |
| `promotion.allow_enforcement` | enforced | inverted promotion gate decision |

The default enforced gate requires at least three report-only observations, at
least three complete report bundles, at least three adapter report-emission
passes, at least three adapter report-field coverage passes, at least three
adapter future-event coverage passes, at least three
Apple Silicon development-effect passes, at least three Apple Silicon paired baseline comparison passes, at least three experiment
switch matrix passes, at least three next-round readiness passes, at least
three Context Rot trend passes, at least three Context Rot remediation passes,
at least three rollback-resume passes, at least three Steam case matrix passes,
at least three validation command coverage passes, no model-quality failures,
no runtime or stream/final failures, and no chain/model availability failures
in the promotion window.

Worker failures follow the same split before enforcement. Worker
`chain_not_ready` and `model_unavailable` observations increment
`promotion.worker_operational_readiness_failure_runs` and block promotion as
readiness or availability issues, not as model quality failures. Worker
`model_quality_failure` increments
`promotion.worker_model_quality_failure_runs` and blocks the quality-stability
part of the window. Any non-empty per-worker claim-blocker list increments
`promotion.worker_claim_blocker_runs`; enforced promotion requires this to stay
at zero so a multi-model pool cannot promote while individual workers still miss
latency/tokens, feedback, validation, output-hygiene, or primary-12B safety
evidence.

## Adapter Handoff Report Schema

`AdapterHandoffReportSchema::adapter_handoff_v1()` fixes the report surface for
the checklist itself. This keeps the final runner-wiring decision reviewable in
the same report bundle as the gates it protects.

| Report field | Stage | Source |
| --- | --- | --- |
| `handoff.stage` | report-only | `RootAdapterRolloutStage::as_str()` |
| `handoff.required_schemas` | report-only | `AdapterHandoffChecklist::required_schemas` |
| `handoff.test_gate_names` | report-only | `AdapterHandoffChecklist::test_gates` |
| `handoff.test_gate_commands` | report-only | `AdapterHandoffChecklist::test_gates` |
| `handoff.norion_test_passed` | report-only | `AdapterHandoffEvidence` |
| `handoff.norion_eval_passed` | report-only | `AdapterHandoffEvidence` |
| `handoff.evolution_loop_passed` | report-only | `AdapterHandoffEvidence` |
| `handoff.workspace_passed` | report-only | `AdapterHandoffEvidence` |
| `handoff.legacy_replay_checked` | report-only | `AdapterHandoffEvidence` |
| `handoff.report_only_observed` | report-only | `AdapterHandoffEvidence` |
| `handoff.report_bundle_complete` | report-only | `AdapterHandoffEvidence` |
| `handoff.schema_drift_passed` | report-only | `AdapterHandoffEvidence` |
| `handoff.adapter_report_emission_passed` | report-only | `AdapterHandoffEvidence` |
| `handoff.adapter_report_field_coverage_passed` | report-only | `AdapterHandoffEvidence` |
| `handoff.adapter_future_event_coverage_passed` | report-only | `AdapterHandoffEvidence` |
| `handoff.model_pool_development_window_passed` | report-only | `AdapterHandoffEvidence` |
| `handoff.apple_silicon_development_effect_passed` | report-only | `AdapterHandoffEvidence` |
| `handoff.feedback_self_improve_passed` | report-only | `AdapterHandoffEvidence` |
| `handoff.self_evolution_continuity_passed` | report-only | `AdapterHandoffEvidence` |
| `handoff.self_evolution_regression_passed` | report-only | `AdapterHandoffEvidence` |
| `handoff.readiness_next_round_passed` | report-only | `AdapterHandoffEvidence` |
| `handoff.self_evolution_unattended_prerequisites_passed` | report-only | `AdapterHandoffEvidence` |
| `handoff.context_rot_trend_passed` | report-only | `AdapterHandoffEvidence` |
| `handoff.context_rot_remediation_passed` | report-only | `AdapterHandoffEvidence` |
| `handoff.rollback_resume_passed` | report-only | `AdapterHandoffEvidence` |
| `handoff.steam_case_matrix_passed` | report-only | `AdapterHandoffEvidence` |
| `handoff.validation_command_coverage_passed` | report-only | `AdapterHandoffEvidence` |
| `handoff.promotion_window_passed` | report-only | `AdapterHandoffEvidence` |
| `handoff.may_block_current_runner` | enforced | `AdapterHandoffChecklist::may_block_current_runner()` |
| `handoff.handoff_blocked` | enforced | `AdapterHandoffChecklist::evaluate()` |
| `handoff.failure_reasons` | enforced | `AdapterHandoffChecklist::evaluate()` |
| `handoff.allow_runner_wiring` | enforced | `AdapterHandoffChecklist::evaluate()` |

In `ReportOnly`, the report records crate-local progress and remains
non-blocking. In `Enforced`, it blocks runner wiring until `norion-test`,
`norion-eval`, `tools/evolution-loop`, and workspace tests pass, legacy ledger
replay was checked, a report-only adapter output was observed, and the report
bundle gate confirmed a complete enforced bundle, schema drift passed, the
adapter report emission gate passed, adapter report-field coverage passed,
adapter future-event coverage passed, the
Apple Silicon development window passed, the Apple Silicon development-effect
aggregate gate passed, feedback/self-improve passed, self-evolution continuity
and regression passed, next-round readiness passed, the self-evolution
unattended prerequisites passed, the Context Rot trend and remediation gates
passed, rollback resume passed, and the adapter promotion window passed.

`AdapterHandoffEvidence` is the thin pure-data lift point for these upstream
reports. Besides the grouped operational helper, it now supports direct
report-to-bit lifts for `EvalReportBundleGateReport`, `EvalSchemaDriftReport`,
`AdapterReportEmissionReport`, `AdapterFutureEventCoverageReport`, and
`SelfEvolutionUnattendedPrerequisiteReport`. These helpers copy only stable
allow/pass bits into handoff evidence; they do not execute tests, scan
JSONL/files, or recompute the upstream reports. `AdviceContinuationReport`
remains intentionally outside this direct handoff input surface.

## Legacy Ledger Replay Compatibility

`LegacyLedgerReplayCompatibility` protects the current `tools/evolution-loop`
ledger surface while new reports are being staged. It is specifically for
replaying old JSONL artifacts; it does not lower the requirements for a future
enforced runner run.

`LegacyLedgerReplayReportSchema::legacy_ledger_replay_v1()` fixes the report
surface for that replay:

| Report field | Stage | Source |
| --- | --- | --- |
| `legacy_replay.stage` | report-only | replay rollout stage |
| `legacy_replay.ledger_rows_present` | report-only | replay evidence |
| `legacy_replay.base_ledger_fields_present` | report-only | replay evidence |
| `legacy_replay.required_existing_fields` | report-only | replay contract |
| `legacy_replay.optional_additive_reports` | report-only | replay contract |
| `legacy_replay.missing_additive_reports` | report-only | replay contract and evidence |
| `legacy_replay.compatibility_blocked` | enforced | replay decision |
| `legacy_replay.failure_reasons` | enforced | replay decision reasons |

The replay report itself is generated by the adapter check; it is not a field
that old JSONL ledgers are expected to contain. Missing worker, model worker
gate, Context Rot, root adapter, feedback, readiness, Steam round, rollback,
handoff, bundle gate, schema drift, model-pool attribution/window/baseline
comparison/effect, promotion window, and rollback resume reports are
compatibility gaps, not replay gate failures. Replay blocks only when rows or
required base fields are absent.

Required existing fields are the current row evidence:

- `round`
- `success`
- `runtime_tokens`
- `runtime_model`
- `validation_checked`
- `validation_passed`
- `feedback_applied`

Optional additive reports are allowed to be missing during replay:

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
- `feedback_self_improve_report_v1`
- `self_evolution_continuity_report_v1`
- `self_evolution_regression_report_v1`
- `self_evolution_unattended_prerequisites_report_v1`
- `readiness_next_round_v1`
- `steam_round_report_v1`
- `rollback_report_v1`
- `rollback_drill_matrix_report_v1`
- `adapter_handoff_report_v1`
- `report_bundle_gate_report_v1`
- `adapter_promotion_window_report_v1`
- `rollback_resume_report_v1`

Replay blocks only when the ledger has no rows or when required existing fields
are absent. Missing additive reports are reported as compatibility gaps, not
gate failures. This lets CI and runbooks compare old ledgers while the current
runner remains unchanged.

## ModelWorkerRecord Ledger Adapter Plan

The current `tools/evolution-loop` ledger is a single-runner ledger, so the
first adapter should project each round as one `high_quality` worker. This gives
the model-pool gate real evidence immediately without changing the runner or
requiring parallel execution.

Existing ledger fields that can populate `ModelWorkerRecord` now:

| `ModelWorkerRecord` field | Current source |
| --- | --- |
| `worker_id` | derive from `case`, or `case` plus `round` |
| `role` | derive as `high_quality` for the current single-worker runner |
| `model` | `runtime_model` |
| `latency_ms` | `elapsed_ms` |
| `runtime_tokens` | `runtime_tokens` |
| `success` | `success` plus `business_cycle_passed` |
| `feedback_applied` | `feedback_applied` |
| `validation_checked` | `validation_checked` |
| `validation_passed` | `validation_passed` |

Fields that need future worker events before they are enforceable:

| `ModelWorkerRecord` field | Needed future event |
| --- | --- |
| `duplicate_output` | `worker_output_fingerprint` or equivalent answer hash |
| `noisy_output` | `worker_noise_score` or a Context Rot/noise verdict |
| `blocked_primary_12b` | `worker_primary_wait_ms` or scheduler-block reason |
| `failure_kind` | `worker_failure_kind`, derived with root adapter precedence |

The pure plan is represented by
`ModelWorkerLedgerAdapterPlan::evolution_loop_ledger()` in `norion-eval`. It is
intentionally a contract, not a parser: the future runner-side adapter can use
the same field map while keeping IO and JSON parsing in `tools/evolution-loop`.

Non-blocking rollout order:

1. `shadow-project-current-ledger`: map the existing ledger into a synthetic
   single-worker `ModelWorkerRecord`; do not print or gate on it yet.
2. `report-only-worker-root-consistency`: emit
   `worker_root_failure_consistency_report_v1` and verify the single-worker
   projection agrees with `root_adapter.failure_kind` without blocking rounds.
3. `report-only-model-pool-summary`: add an additive model-pool summary to
   reports; existing report fields remain compatible.
4. `write-optional-worker-events`: add optional worker evidence for parallel
   runs; old ledgers continue to parse.
5. `advisory-model-pool-gate`: log model-pool gate failures without stopping
   rounds.
6. `enforcing-model-pool-gate`: only then allow the model-pool gate to block,
   after the crate, evolution-loop, and workspace tests all pass against a clean
   ledger fixture.

Recommended test gates before enforcement:

- `cargo test --manifest-path crates/norion-test/Cargo.toml`
- `cargo test --manifest-path crates/norion-eval/Cargo.toml`
- `cargo test --manifest-path tools/evolution-loop/Cargo.toml`
- `cargo test --workspace`

The first two commands prove the shared pure plan/eval contracts. The latter
two should be run when the main window allows `tools/evolution-loop` changes and
prove the current runner still behaves as before.

## Main Window Wiring Suggestion

When the root workspace is ready, add `crates/norion-test` and
`crates/norion-eval` as workspace members. Then add a small adapter in
`tools/evolution-loop` that maps its existing report structs to
`norion_eval::LedgerRecord`. Keep command execution, ledger IO, and SSE parsing
inside `tools/evolution-loop`; only delegate summary/gate decisions to
`norion-eval`.

This preserves the current working loop while making the acceptance policy
reusable by CI, Web Lab, and future Steam-series test runners.
