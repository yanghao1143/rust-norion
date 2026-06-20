# norion-core

`crates/norion-core` is the pure Rust kernel contract layer for the Norion/Noiron
engine. It has no Candle, service, CLI, model-server, or root-crate dependency.
The root workspace is intentionally not wired yet; the main window should attach
this crate to the root workspace when parallel module work has settled.

For root-adapter failure-return routing, see
`docs/architecture/norion-core-failure-return-matrix.md`.

## Interface Surface

- `InferenceEngine`, `InferenceRequest`, `InferenceOutcome`,
  `InferenceOutcomeSummary`, `GeneratedToken`, `GeneratedTokenMetrics`,
  `InferenceError`, `RuntimeBackendMaxTokensCommitAction`,
  `RuntimeBackendMaxTokensCommitSummary`, `RuntimeFailureKind`,
  `RuntimeFailureReport`, `RuntimeFailureSummary`, `RuntimeFailureBatchSummary`
  - Stable inference boundary for local runtime, model service, or tests.
  - `InferenceRequest::max_tokens` and `prompt_tokens` are first-class request
    fields.
  - `InferenceRequest::generation_budget()` computes the effective context and
    max generation budget from `RuntimeMetadata`.
  - `RuntimeGenerationBudget` exposes known/unknown context state, restored
    context-window size, requested overflow tokens, generation deficit tokens,
    context exhaustion, soft truncation, focused context-budget signal
    component counts, aggregate signal presence, accounting consistency, and
    planned/remaining context fractions so adapters can gate `max_tokens`
    without parsing root runtime errors.
  - `RuntimeGenerationBudget::backend_max_tokens_commit_summary()` gives root a
    single pre-request action for committing the planned backend max-token cap,
    returning context exhaustion, or repairing malformed budget accounting.
  - `InferenceOutcome` carries `InferenceDiagnostics` so adapters can preserve
    runtime, embedding, route, KV, and hardware summaries without depending on
    root outcome types.
  - `InferenceOutcome::outcome_summary()` exposes answer length, token count,
    uncertainty-signal presence, route token counts, imported/exported KV
    counts, diagnostics KV parity, runtime execution signal, and diagnostics
    note count before root expands outcome payloads.
  - `InferenceOutcomeSummary` helpers classify route activity, diagnostics note
    presence, text/token shape drift, diagnostics KV count drift, missing
    runtime execution signal, and complete runtime response shape before root
    expands parsed response payloads. Focused response-shape problem counts and
    accounting helpers split text/token drift, diagnostics KV drift, missing
    runtime execution signal, clean response shape, and usable runtime outcome
    state before response-envelope validation.
  - `GeneratedTokenMetrics` summarizes token count, entropy, negative logprob,
    and uncertainty perplexity from generated tokens for adapter parity checks.
    Coverage and accounting helpers expose missing/partial entropy or logprob
    signals, malformed metric counts, and uncertainty-signal consistency before
    root compares runtime token diagnostics. Use `uncertainty_shape_is_clean()`
    and `can_use_token_uncertainty_metrics()` before root accepts compact token
    uncertainty summaries.
  - `RuntimeFailureReport` classifies runtime, KV import/export, contract, and
    context exhaustion failures into backend messages, diagnostics notes, trace
    labels, and recoverability hints without depending on root `RuntimeError`.
  - `RuntimeFailureReport::failure_summary()` exposes single-failure kind,
    trace-label, message-shape, recoverability, backend-error wrapping,
    diagnostics-note, and trace-confidence facts before root expands formatted
    failure strings. Use `failure_summary_shape_is_clean()` and
    `can_use_runtime_failure_report()` before mapping one core failure into a
    root runtime error or trace note.
  - `RuntimeFailureBatchSummary` counts runtime/KV/contract/context/unknown
    failures, recoverable failures, backend-error messages, diagnostics notes,
    and minimum trace confidence before root expands individual failure text.
  - `RuntimeFailureBatchSummary` helpers expose failure-class totals,
    class-count parity, recoverable/backend-error bounds, diagnostics note
    parity, trace-confidence presence/validity, runtime/KV/contract failure
    presence, zero-confidence trace state, aggregate failure-batch accounting,
    clean batch shape, and runtime-failure formatting readiness before root
    formats individual errors.
- `RuntimeAcceptanceContext`, `RuntimeBoundaryAcceptanceSummary`,
  `RuntimeBoundaryAdapterSummary`, `RuntimeBoundaryCommitReadinessSummary`,
  `RuntimeBoundaryCommitAction`, `RuntimeBoundaryCommitStage`,
  `RuntimeBoundaryCommitSummary`, `RuntimeBoundaryEnvelopeSummary`,
  `RuntimeBoundaryGateSummary`, `RuntimeBoundaryKvSummary`,
  `FailureReturnFamily`, `FailureReturnRoutingBatchSummary`,
  `FailureReturnRoutingDecision`, `FailureReturnRoutingKey`,
  `FailureReturnRoutingSelection`, `FailureReturnRoutingSummary`,
  `RuntimeKvPersistenceFailureReturnSelection`,
  `RuntimeFailureReturnReport`, `RuntimeFailureReturnSource`,
  `RuntimeFailureReturnSummary`,
  `RuntimeKvSideEffectProblemKind`,
  `RuntimeKvSideEffectReadinessSummary`, `RuntimeKvSideEffectStage`,
  `RuntimeManifestBoundaryCommitAction`,
  `RuntimeManifestBoundaryCommitProblemKind`,
  `RuntimeManifestBoundaryCommitReadinessSummary`,
  `RuntimeManifestBoundaryCommitSummary`,
  `RuntimeManifestBoundaryCommitStage`,
  `RuntimeManifestBoundaryKvStage`, `RuntimeManifestBoundaryKvSummary`
  - Adapter-facing saved runtime boundary context: originating
    `RuntimeRequestEnvelope`, mapped `HardwarePlan`, and imported runtime
    `KvBlock`s.
  - `RuntimeAcceptanceContext::from_request_parts(...)` builds the request
    envelope with `HardwarePlan::adapter_execution_context().clamp_for_runtime(...)`
    so hardware KV prefetch and precision stay inside runtime metadata limits.
  - Lets root run `request_acceptance_report()` before runtime request JSON, then
    reuse the same saved request/hardware facts for `response_acceptance_report`
    after response parsing.
  - `request_acceptance_summary()` and `response_acceptance_summary(...)`
    expose the same gate summaries directly from the saved context, keeping root
    request and response checks on one set of facts.
  - `request_planning_readiness_summary(...)` reuses the saved request envelope
    and concrete imported KV blocks to produce
    `RuntimeRequestPlanningReadinessSummary`, so root can check runtime
    planning, request-planning parity, and the request gate from one saved
    context before backend JSON serialization.
  - `manifest_request_planning_readiness_summary(...)` adds the runtime
    manifest to that saved context path, producing
    `RuntimeRequestManifestPlanningReadinessSummary` from the same request and
    imported KV facts before backend JSON serialization or KV materialization.
  - `runtime_diagnostics_seed()` and `inference_diagnostics_seed()` derive the
    response-side diagnostics baseline from the saved request and hardware
    plan, so planning-attached response parity starts with matching route,
    generation, runtime metadata, hardware pressure, headroom, and latency
    facts before root layers runtime-reported execution and KV fields.
  - `response_gate_summary(...)` reuses the saved request and hardware facts to
    produce `RuntimeResponseGateSummary` without making root rebuild the parsed
    response gate by hand.
  - `response_readiness_summary(...)` reuses the same saved request and
    hardware facts to produce `RuntimeResponseReadinessSummary`, so root can
    check response envelope, request parity, and response gate stage readiness
    from one context before side effects.
  - `response_manifest_kv_summary(...)` reuses the saved request envelope and a
    checked `RuntimePlanningManifestKvBridgeSummary` to produce
    `RuntimeResponseManifestKvSummary`, keeping response-side manifest KV
    commit checks on the same request facts as response acceptance.
  - `boundary_commit_readiness_summary(...)` provides the non-manifest
    request/response boundary gate. Use `RuntimeBoundaryCommitStage`,
    `first_unready_stage()`, `first_blocking_stage()`, per-stage signal/blocker
    counts, and `can_commit_runtime_boundary()` when root needs one compact
    gate over request acceptance, response acceptance, boundary envelope,
    adapter, KV, gate, and runtime response readiness. `failure_report_for(...)`,
    `failure_reports()`, `primary_failure_report()`,
    `primary_failure_summary()`, and `failure_batch_summary()` map boundary
    blockers into runtime/contract failure classes without copying root error
    formatting. `commit_summary()` returns `RuntimeBoundaryCommitSummary` with
    `RuntimeBoundaryCommitAction`, primary failure summary, failure batch,
    formatter readiness, and commit-decision accounting so adapters can choose
    commit vs. runtime failure from one value. Call
    `failure_return_summary()` on boundary, manifest-boundary, or KV
    side-effect commit summaries to get a common `RuntimeFailureReturnSummary`
    with source label, primary failure presence, failure batch class counts,
    formatter readiness, and `can_return_runtime_failure()`.
    `runtime_failure_return_report()` materializes a common
    `RuntimeFailureReturnReport` only for failed commit gates that can be
    returned as runtime failures; it carries the primary `RuntimeFailureReport`,
    backend message/diagnostics-note helpers, `InferenceError` conversion, and
    report-shape checks.
    `FailureReturnRoutingSummary` is the cross-family adapter snapshot for
    source label, family, commit/return flags, primary failure summary, failure
    batch counts, blocker/problem component counts, and public accounting
    consistency. Root can convert any failure-return summary into it before
    choosing a family-specific report type.
    `FailureReturnRoutingBatchSummary` aggregates those routing summaries,
    preserves the first returnable `(family, source_label)` key, counts route
    accounting drift, and exposes family-presence flags so root can test staged
    routing without formatting individual runtime errors.
    `FailureReturnRoutingDecision` names the adapter action as continue,
    return runtime failure, or repair accounting before a family-specific
    report is materialized. `FailureReturnRoutingSummary::matches_key(...)`
    and `FailureReturnRoutingDecision::select_route(...)` let adapters recover
    the selected routing summary without reimplementing family/source matching.
    `FailureReturnRoutingSelection::from_routes(...)` bundles the batch,
    decision, and selected routing summary for root adapters that want one
    assertion object before report materialization.
    `RuntimeKvPersistenceFailureReturnSelection::from_summaries(...)` is the
    KV-persistence-specific wrapper for namespace distribution plus fusion
    persistence. It fixes the adapter order as `kv_namespace_distribution` then
    `kv_fusion_persistence`, exposes the selected persistence source, and
    forces repair-accounting behavior when callers pass summaries in a
    non-canonical source order.
  - `manifest_boundary_kv_summary(...)` combines request manifest-planning
    readiness and response manifest-KV readiness with the same manifest-derived
    bridge. Use `RuntimeManifestBoundaryKvStage`, `first_unready_stage()`,
    `first_blocking_stage()`, and `can_commit_manifest_boundary_kv()` when root
    needs one compact manifest KV gate for a full request/response boundary.
  - `manifest_boundary_commit_readiness_summary(...)` wraps the normal
    request/response boundary commit readiness together with manifest-boundary
    KV readiness. Use `RuntimeManifestBoundaryCommitStage`,
    `first_unready_stage()`, `first_blocking_stage()`, and
    `can_commit_runtime_manifest_boundary()` when a runtime manifest is present
    and root needs one final gate before exported-KV, reflection, or memory
    side effects.
    `RuntimeManifestBoundaryCommitProblemKind`, `problem_kind_order()`,
    `first_problem_kind()`, and `problem_kind_component_count(...)` split that
    final gate into boundary-commit, request manifest-planning, response
    manifest-KV, and accounting-drift buckets for root failure mapping.
    `commit_summary()` returns `RuntimeManifestBoundaryCommitSummary` with
    `RuntimeManifestBoundaryCommitAction`, first failing stage/problem kind,
    contract-violation failure reports, primary failure summary, failure batch,
    formatter readiness, the child `RuntimeBoundaryCommitAction`, and
    commit-decision accounting.
  - `RuntimeKvSideEffectReadinessSummary` composes
    `RuntimeKvImportReadinessSummary`,
    `RuntimeManifestBoundaryCommitReadinessSummary`, and
    `RuntimeKvExportReadinessSummary` into the adapter side-effect order
    import -> manifest boundary commit -> export. Use
    `RuntimeKvSideEffectStage`, `first_unready_stage()`,
    `first_blocking_stage()`, and `can_commit_runtime_kv_side_effects()` when
    root wants one final KV side-effect gate without reimplementing stage
    ordering across modules. The summary also carries the child
    `RuntimeKvImportReadinessCommitAction` and
    `RuntimeManifestBoundaryCommitAction` and
    `RuntimeKvExportReadinessCommitAction` plus action-readiness consistency
    helpers, so adapters can verify nested import/manifest/export decisions
    without re-reading child counters.
    `RuntimeKvSideEffectProblemKind`, `problem_kind_order()`,
    `first_problem_kind()`, and `problem_kind_component_count(...)` expose a
    single failure-mapping surface for import, manifest boundary commit,
    manifest-planning, response manifest-KV, export, and accounting-drift
    diagnostics. `failure_report_for(...)`, `failure_reports()`, and
    `primary_failure_report()` map import/export blockers to KV failure kinds
    and manifest/boundary/accounting blockers to contract violations.
    `failure_report_count()`, `has_failure_reports()`, and
    `failure_batch_summary()` mirror the request/response acceptance report
    pattern for adapter-level failure aggregation. `primary_failure_summary()`
    and `can_format_runtime_failures()` let root use the same primary-summary
    and batch-formatting gates as planning, manifest, diagnostics, and
    request/response acceptance reports. `commit_summary()` returns
    `RuntimeKvSideEffectCommitSummary` with a stable
    `RuntimeKvSideEffectCommitAction`, first failing stage/problem kind,
    child import/manifest/export commit actions, primary failure summary,
    failure batch, and commit/failure booleans so the root adapter can choose
    between committing KV side effects and returning a runtime failure without
    re-reading nested import/export counters. The
    commit summary delegates `primary_failure_report()`, `failure_report_for(...)`,
    and `failure_reports()` back to the same readiness mapping, giving adapters
    one object for both action selection and runtime error materialization.
    `RuntimeKvImportReadinessCommitSummary` and
    `RuntimeKvExportReadinessCommitSummary` now also expose
    `failure_return_summary()` and `runtime_failure_return_report()` through
    the shared `RuntimeKvExchangeFailureReturnSummary` /
    `RuntimeKvExchangeFailureReturnReport` shape, so root can wire the
    single-sided import/export gates before adopting the full side-effect
    wrapper.
    `KvNamespaceCountDriftCommitSummary` and `KvFusionCommitSummary` expose the
    parallel `RuntimeKvPersistenceFailureReturnSummary` /
    `RuntimeKvPersistenceFailureReturnReport` shape for namespace distribution
    and fused-KV persistence blockers.
    When both persistence sources are routed together, prefer
    `RuntimeKvPersistenceFailureReturnSelection::from_summaries(...)` so
    `kv_namespace_distribution` is checked before `kv_fusion_persistence`,
    namespace count drift wins over fused-KV accounting drift at the storage
    mutation boundary, and non-canonical source order becomes an adapter
    accounting repair instead of a formatted runtime failure. When namespace
    distribution is clean, `kv_persistence_selection_returns_fusion_failure_after_clean_namespace_gate`
    locks the handoff to the fusion failure route.
    Its `failure_return_summary()` shares the same
    `RuntimeFailureReturnSummary` projection as the boundary and manifest
    boundary commit summaries, so root can use one adapter branch for
    commit-gate-to-runtime-failure conversion. Its
    `runtime_failure_return_report()` materializes the same
    `RuntimeFailureReturnReport` shape for side-effect failures.
  - `boundary_acceptance_summary(...)` combines request and response acceptance
    summaries for one saved request/hardware context, exposing total violation
    and failure-report counts before root expands verbose reports.
  - Boundary acceptance helpers expose request/response/KV/request-parity
    failure presence, clean acceptance, and total failure-report parity across
    the saved request and parsed response.
    Focused helpers split request acceptance, response acceptance, KV,
    request-parity, mapped failure-report presence, and aggregate boundary
    acceptance problem counts before root expands either side's verbose reports.
    `boundary_acceptance_accounting_is_consistent()` keeps request/response
    violation totals, failure-report totals, nested acceptance accounting, and
    accepted-state parity aligned before root maps either side into runtime
    errors. Use `runtime_boundary_acceptance_commit_signal_component_count()`,
    `runtime_boundary_acceptance_commit_blocker_component_count()`,
    `runtime_boundary_acceptance_commit_accounting_is_consistent()`, and
    `can_commit_runtime_boundary_acceptance()` when root needs one compact
    request+response acceptance decision before checking envelope, adapter, and
    KV boundary summaries.
  - `boundary_envelope_summary(...)` combines request and response envelope
    summaries from the same saved context so root can check token cap, imported
    KV request parity, diagnostics KV parity, token uncertainty coverage and
    accounting, runtime execution signal, adapter presence, and context pressure
    before acceptance reports are expanded. Use
    `runtime_boundary_envelope_commit_signal_component_count()`,
    `runtime_boundary_envelope_commit_blocker_component_count()`,
    `runtime_boundary_envelope_commit_accounting_is_consistent()`, and
    `can_commit_runtime_boundary_envelope()` when root needs one compact
    request/response envelope gate before boundary acceptance.
    Boundary envelope helpers split response-token drift, imported-KV drift,
    diagnostics-KV drift, missing runtime execution signal, missing request
    adapter signal, context-pressure signal, uncertainty coverage signals,
    uncertainty metric problems, shape-drift totals, aggregate envelope signal
    counts, envelope consistency, `boundary_envelope_shape_is_clean()`, and
    `can_use_runtime_boundary_envelope()`.
  - `boundary_adapter_summary(...)` compares the saved request selected adapter,
    runtime diagnostics selected adapter, hardware execution context, and
    optional planning adapter selection report so root can detect missing,
    drifted, or disallowed runtime adapter execution before response acceptance
    expands verbose diagnostics. Adapter boundary helpers split request-missing,
    runtime-missing/drift/outside-context, planning-selection, and aggregate
    signal/blocker counts before verbose adapter diagnostics. Use
    `adapter_boundary_commit_signal_component_count()`,
    `adapter_boundary_commit_blocker_component_count()`,
    `adapter_boundary_commit_accounting_is_consistent()`, and
    `can_commit_runtime_boundary_adapter()` before committing adapter-specific
    side effects.
  - `boundary_kv_summary(...)` compares concrete imported KV blocks, response
    imported/exported KV counts, diagnostics KV counts, runtime import/export
    limits, optional planning KV exchange, validation summaries, and namespace
    counts before root expands verbose KV payload violations. KV boundary
    helpers split legal activity/capability signals from blockers: imported,
    exported, diagnostics, runtime capability, planning, and namespace activity
    signals remain observable even for clean exchanges, while runtime exchange
    count drift, planning-bound drift, runtime-bound drift, namespace drift,
    validation failures, and aggregate KV problem component counts decide
    whether the response side effects may commit. Use
    `kv_boundary_shape_is_clean()` and `can_use_runtime_boundary_kv()` before
    accepting runtime KV exchange.
  - `boundary_gate_summary(...)` combines request/response acceptance,
    envelope shape and token uncertainty accounting, adapter parity, KV parity,
    request backend-wire problem counts, response-wire problem counts,
    request/response planning pre-request gate problems, and request/response
    planning pressure signals plus aggregate KV boundary signals into one final
    structured gate before root commits response side effects such as reflection
    or memory mutation.
  - `boundary_commit_readiness_summary(...)` combines boundary acceptance,
    envelope, adapter, KV, and final gate readiness into one adapter-facing
    migration checklist. It preserves per-stage ready bits and per-stage
    signal/blocker counts so root can follow the request acceptance -> response
    acceptance -> boundary acceptance -> envelope -> adapter -> KV -> final
    gate order without reconstructing every nested summary. Use
    `first_unready_stage()` and `first_blocking_stage()` with
    `RuntimeBoundaryCommitStage` when root needs to route a failed commit check
    to the first acceptance, envelope, adapter, KV, gate, or runtime-response
    adapter report.
  - Boundary gate helpers classify acceptance failures, boundary drift, mapped
    failure-report presence, total violation presence, clean commit readiness,
    request/response, envelope, adapter, and KV blocker component counts,
    request backend-wire problem counts, response-wire problem counts, direct
    request/response wire problems separate from planning pre-request blockers,
    planning pressure signal presence, KV boundary signal presence, response
    uncertainty coverage signals, response uncertainty metric problems,
    aggregate commit-gate signal counts, aggregate wire problem counts,
    aggregate problem-component presence, wire-problem absence for final
    commits, wire accounting, commit-gate accounting consistency, final boundary
    gate shape, runtime boundary commit signal/blocker counts, runtime boundary
    commit accounting, and runtime response/boundary commit readiness without
    inspecting verbose request or response violations.
    `commit_gate_signal_component_count()` is observational only; it does not
    add to `commit_blocker_component_count()`. Use
    `runtime_boundary_commit_signal_component_count()`,
    `runtime_boundary_commit_blocker_component_count()`,
    `runtime_boundary_commit_accounting_is_consistent()`, and
    `can_commit_runtime_boundary()` before root commits final reflection,
    memory, or exported-KV side effects for the request/response pair.
  - Does not own JSON formatting, model execution, trace display, reflection, or
    memory mutation.
- `RuntimeRequestEnvelope`, `RuntimeRequestEnvelopeSummary`,
  `RuntimeRequestPlanningParitySummary`, `RuntimeRequestGateSummary`,
  `RuntimeRequestPlanningReadinessStage`,
  `RuntimeRequestPlanningReadinessSummary`,
  `RuntimeRequestManifestPlanningReadinessStage`,
  `RuntimeRequestManifestPlanningReadinessSummary`,
  `RuntimeRequestAcceptanceReport`, `RuntimeRequestAcceptanceSummary`,
  `RuntimeRequestFailureReturnReport`, `RuntimeRequestFailureReturnSource`,
  `RuntimeRequestFailureReturnSummary`,
  `RUNTIME_REQUEST_SCHEMA`
  - Adapter-facing summary of the root runtime request wire contract. It keeps
    schema, runtime metadata, architecture, route, hierarchy, transformer layer
    count, hardware pressure, adapter count, and imported KV counts together
    without owning JSON serialization or root tool/agent/recursive payloads.
  - `RuntimeRequestEnvelope::contract_violations()` catches context exhaustion,
    invalid architecture, adapter absence, and KV import limit mismatches before
    root formats the runtime request.
  - Optional `RuntimePlanningDigest` attachment lets the envelope verify that
    root wire fields use the planned backend `max_tokens`, selected adapter, and
    KV prefetch/import count.
  - `RuntimeRequestEnvelope::planning_parity_summary()` exposes attached
    planning parity as structured fields for backend max tokens, generation
    budget, selected adapter, imported KV, KV prefetch, planned export count,
    planning contract violations, planning pre-request gate problems, and
    planning pressure signals before root expands verbose request diffs.
  - `RuntimeRequestPlanningParitySummary` helpers classify missing planning,
    token-budget drift, adapter drift, imported-KV drift, KV-prefetch drift, and
    planning contract drift into compact component counts before root serializes
    backend JSON.
    Focused helpers split planning attachment, max-token, generation-budget,
    adapter, imported-KV, KV-prefetch, planning-contract drift, planning
    pre-request gate problems, backend-wire problem totals, and pressure-signal
    presence before root expands verbose request parity text. Use
    `backend_wire_shape_is_clean()` and `can_use_backend_wire_request()` as the
    compact parity gate before JSON serialization.
  - When planning is attached, both actual imported KV block count and
    `kv_prefetch_blocks` must match `planned_kv_exchange().import_blocks`.
  - Optional `RecursiveScheduleSummary` attachment lets the request envelope
    check recursive prompt-token and execution-wave consistency without owning
    root recursive JSON formatting.
  - `RuntimeRequestEnvelope::envelope_summary()` exposes context generation
    pressure, runtime KV exchange capacity, architecture/layer parity, adapter
    candidates, hardware pressure band, planning KV exchange, and recursive
    attachment state before root expands verbose request diffs.
    Use `request_envelope_commit_signal_component_count()`,
    `request_envelope_commit_blocker_component_count()`,
    `request_envelope_commit_accounting_is_consistent()`, and
    `can_commit_runtime_request_envelope()` as the compact adapter-facing
    request-envelope gate before JSON serialization. The signal count remains
    observational; blockers are schema/layer/KV-limit drift, while
    `can_commit_runtime_request_envelope()` also requires generation capacity
    and a selected adapter.
  - `RuntimeRequestEnvelope::request_gate_summary(...)` combines request
    acceptance, envelope/planning parity, backend-wire problem counts, planning
    pre-request gate problems, planning pressure signals, imported runtime KV
    validation, and mapped failure-report counts into one final structured
    pre-JSON gate. Root should only serialize the backend request when
    `request_gate_shape_is_clean()` and `can_send_runtime_request()` are true.
  - `RuntimeRequestGateSummary` helpers classify request-contract failures,
    imported-KV failures, boundary drift, clean send readiness, and
    failure-report/count parity before JSON serialization.
    Focused helpers split acceptance failures, request-envelope drift, planning
    drift, mapped failure-report presence, focused request-contract/imported-KV
    and envelope/planning blocker counts, aggregate send blockers,
    backend-wire problem counts, direct backend-wire problems separate from
    planning pre-request blockers, imported-KV activity signals, planning
    pressure signal presence, aggregate send-gate signal counts,
    problem-component presence, backend-wire accounting, and send-gate
    accounting consistency before root expands request gate violations.
    `send_gate_signal_component_count()` is observational only; it does not add
    to `send_blocker_component_count()`. Use
    `runtime_request_commit_signal_component_count()`,
    `runtime_request_commit_blocker_component_count()`,
    `runtime_request_commit_accounting_is_consistent()`, and
    `can_commit_runtime_request()` before root serializes the runtime request
    JSON or mutates backend send state.
  - `RuntimeRequestPlanningReadinessSummary` bridges
    `RuntimePlanningReadinessSummary` -> `RuntimeRequestPlanningParitySummary`
    -> `RuntimeRequestGateSummary` immediately before backend request
    serialization. Its `RuntimeRequestPlanningReadinessStage` order,
    `first_unready_stage()`, `first_blocking_stage()`, per-stage
    signal/blocker counts, aggregate accounting, and
    `can_commit_runtime_request_planning()` let root verify the runtime plan,
    wire parity, and final request gate without rebuilding the nested
    summaries.
    `RuntimeRequestEnvelope::request_planning_readiness_summary(...)` builds
    that bridge from the envelope plus the already-checked runtime planning
    readiness and concrete imported KV blocks, keeping pre-JSON request
    readiness on one set of wire facts.
  - `RuntimeRequestManifestPlanningReadinessSummary` adds
    `RuntimePlanningManifestKvBridgeSummary` as the request-boundary pre-stage
    before `RuntimeRequestPlanningReadinessSummary`. Use
    `RuntimeRequestEnvelope::manifest_request_planning_readiness_summary(...)`
    when a runtime manifest is available so manifest KV policy and planning
    import/export counts are checked before backend JSON serialization or KV
    materialization.
  - `imported_kv_contract()` and `validate_imported_kv_blocks(...)` derive the
    runtime import boundary directly from the request envelope, so root can
    reject control-plane KV blocks before wire serialization.
  - `acceptance_report(...)` combines request contract violations and imported
    KV validation into one adapter-facing gate before runtime request JSON.
  - `RuntimeRequestAcceptanceReport::acceptance_summary()` exposes accepted
    state, request-contract failure count, imported-KV failure count, accepted
    imported block count, and mapped failure-report count without parsing
    violation text.
  - `RuntimeRequestAcceptanceReport::failure_batch_summary()` and
    `primary_failure_summary()` expose mapped request-boundary failure class mix
    and first-failure shape directly from the report before root formats runtime
    errors.
    `failure_return_summary()` and `runtime_failure_return_report()` provide a
    single-report failure-return path before root adopts the full boundary
    commit gate.
  - `RuntimeRequestAcceptanceSummary` helpers expose request/import failure
    classes, aggregate failure presence, clean acceptance, and mapped
    failure-report parity.
    Focused helpers split request-contract failures, imported-KV failures,
    mapped failure-report presence, and aggregate request acceptance problem
    counts before root maps failures into runtime errors.
    `request_acceptance_accounting_is_consistent()` checks accepted-state,
    violation-count, failure-class, mapped-report, and problem-count parity as
    the compact request-boundary shape gate. Use
    `runtime_request_acceptance_commit_signal_component_count()`,
    `runtime_request_acceptance_commit_blocker_component_count()`,
    `runtime_request_acceptance_commit_accounting_is_consistent()`, and
    `can_commit_runtime_request_acceptance()` when root needs the final request
    acceptance decision.
  - `RuntimeRequestAcceptanceReport::failure_reports()` maps request contract
    failures to `runtime_contract_violation` and imported KV payload failures to
    `runtime_kv_import_error`.
- `RuntimeResponseEnvelope`, `RuntimeResponseEnvelopeSummary`,
  `RuntimeResponseRequestParitySummary`, `RuntimeResponsePlannedKvSummary`,
  `RuntimeResponseManifestKvSummary`,
  `RuntimeResponseGateSummary`, `RuntimeResponseReadinessStage`,
  `RuntimeResponseReadinessSummary`,
  `RuntimeResponseAcceptanceReport`, `RuntimeResponseAcceptanceSummary`,
  `RuntimeResponseFailureReturnReport`, `RuntimeResponseFailureReturnSource`,
  `RuntimeResponseFailureReturnSummary`,
  `RUNTIME_RESPONSE_SCHEMA`
  - Adapter-facing summary of the root runtime response wire contract. It keeps
    answer length, generated token metrics, imported/exported KV counts, and
    runtime execution signal status together without owning JSON parsing.
  - `RuntimeResponseEnvelope::contract_violations(...)` checks non-empty
    answers, empty token payloads, KV count mismatches, and runtime diagnostics
    contract violations before root accepts a runtime response.
  - `RuntimeResponseEnvelope::request_contract_violations(...)` checks the
    parsed response against the originating `RuntimeRequestEnvelope`, including
    generated-token cap, imported/exported KV counts, selected adapter, and
    generation-budget parity.
  - `RuntimeResponseEnvelope::request_parity_summary(...)` exposes the same
    request backcheck as compact fields for token limits, KV counts, adapter
    parity, generation/route/hardware diagnostics, and planning-bound
    backend/KV/headroom/latency parity, planning pre-request gate problems, and
    planning pressure signals before root expands verbose violations.
  - `RuntimeResponseRequestParitySummary` helpers classify token, KV, adapter,
    generation-budget, route, hardware-pressure, and planning diagnostics drift
    into component counts so adapters can pick the first failing response layer
    before parsing violation strings. Focused helpers split request-token,
    planning-token, request/runtime/planning KV, request diagnostics, and
    planning diagnostics drift counts before verbose request-parity text.
    Response-wire helpers add planning pre-request gate problems, pressure
    signal presence, aggregate response-wire problem counts, and accounting
    consistency. Use `response_wire_shape_is_clean()` and
    `can_use_response_wire()` as the compact parsed-response parity gate before
    reflection or memory mutation.
  - `RuntimeResponseRequestParitySummary::planned_kv_summary()` returns
    `RuntimeResponsePlannedKvSummary`, a compact adapter gate for comparing
    response imported/exported KV counts with planning import/export bounds.
    Use `response_planned_kv_problem_component_count()`,
    `response_planned_kv_shape_is_clean()`, and
    `can_commit_planned_kv_response()` when root needs to enforce a
    planning-attached response boundary, including planned zero-export cases.
  - `RuntimeResponseRequestParitySummary::manifest_kv_summary(...)` combines
    the planned response KV view with
    `RuntimePlanningManifestKvBridgeSummary`, exposing response-vs-manifest
    import/export coverage, manifest bridge blockers, planned-KV blockers, and
    aggregate response manifest-KV accounting. Use
    `response_manifest_kv_shape_is_clean()` and
    `can_commit_response_manifest_kv()` after request manifest-planning
    readiness and before accepting response-side KV effects.
  - `RuntimeResponseEnvelope::envelope_summary()` exposes answer/token shape,
    uncertainty-signal presence, token uncertainty coverage/problem/accounting
    summaries, runtime execution signal, and exact imported/exported KV parity
    with diagnostics before root checks verbose response violations. Use
    `runtime_response_envelope_commit_signal_component_count()`,
    `runtime_response_envelope_commit_blocker_component_count()`,
    `runtime_response_envelope_commit_accounting_is_consistent()`, and
    `can_commit_runtime_response_envelope()` before handing a parsed envelope
    to response-wire or boundary gates. Legacy
    `response_envelope_shape_is_clean()` and
    `can_use_runtime_response_envelope()` delegate to the same commit-facing
    readiness checks.
  - `RuntimeResponseEnvelope::response_gate_summary(...)` combines response
    acceptance, envelope shape, request parity, exported runtime KV validation,
    response-wire problem counts, planning pre-request gate problems, planning
    pressure signals, and mapped failure-report counts into one structured
    parse-to-commit gate. Root should only accept parsed runtime output when
    `response_gate_shape_is_clean()` and `can_accept_runtime_response()` are
    true.
  - `RuntimeResponseGateSummary` helpers classify response-contract failures,
    request-parity failures, exported-KV failures, boundary drift, clean
    response readiness, and failure-report/count parity before reflection or
    memory mutation.
    Focused helpers split response acceptance failures, envelope drift,
    request-parity drift, exported-KV drift, mapped failure-report presence,
    focused response/request/export failure and boundary blocker counts,
    aggregate response blockers, response-wire problem counts, direct
    response-wire problems separate from planning pre-request blockers,
    exported-KV activity signals, planning pressure signal presence, aggregate
    response-gate signal counts, problem-component presence, response-wire
    accounting, and response-gate accounting consistency before root expands
    parse-to-commit violations. `response_gate_signal_component_count()` is
    observational only; it does not add to `response_blocker_component_count()`.
    Use `runtime_response_commit_signal_component_count()`,
    `runtime_response_commit_blocker_component_count()`,
    `runtime_response_commit_accounting_is_consistent()`, and
    `can_commit_runtime_response()` before root commits parsed runtime output
    into reflection, memory, or exported-KV side effects.
  - `RuntimeResponseReadinessSummary` bridges
    `RuntimeResponseEnvelopeSummary` -> `RuntimeResponseRequestParitySummary`
    -> `RuntimeResponseGateSummary` after response JSON parsing and before
    side effects. Its `RuntimeResponseReadinessStage` order,
    `first_unready_stage()`, `first_blocking_stage()`, per-stage
    signal/blocker counts, response-wire accounting, aggregate accounting, and
    `can_commit_runtime_response_readiness()` let root verify parsed response
    readiness without reconstructing the envelope, request-parity, and gate
    summaries.
  - `validate_exported_kv_blocks(...)` validates exported runtime KV payloads
    against the originating request envelope before root accepts memory updates.
  - `acceptance_report(...)` combines response diagnostics checks,
    request-envelope parity, and exported KV validation into one adapter-facing
    gate before root accepts runtime output.
  - `RuntimeResponseAcceptanceReport::acceptance_summary()` exposes accepted
    state, response-contract failure count, request-parity failure count,
    exported-KV failure count, accepted exported block count, and mapped
    failure-report count for root parity tests. Use
    `runtime_response_acceptance_commit_signal_component_count()`,
    `runtime_response_acceptance_commit_blocker_component_count()`,
    `runtime_response_acceptance_commit_accounting_is_consistent()`, and
    `can_commit_runtime_response_acceptance()` before root promotes parsed
    response acceptance into response-gate or side-effect decisions.
  - `RuntimeResponseAcceptanceReport::failure_batch_summary()` and
    `primary_failure_summary()` expose mapped response-boundary failure class mix
    and first-failure shape directly from the report before root formats runtime
    errors.
    `failure_return_summary()` and `runtime_failure_return_report()` provide a
    single-report failure-return path before root adopts the full boundary
    commit gate.
  - `RuntimeResponseAcceptanceSummary` helpers expose response/request/export
    failure classes, aggregate failure presence, clean acceptance, and mapped
    failure-report parity.
    Focused helpers split response-contract failures, request-parity failures,
    exported-KV failures, mapped failure-report presence, and aggregate response
    acceptance problem counts before root maps failures into runtime errors.
    `response_acceptance_accounting_is_consistent()` checks accepted-state,
    violation-count, failure-class, mapped-report, and problem-count parity as
    the compact response-boundary shape gate.
  - `RuntimeResponseAcceptanceReport::failure_reports()` maps response/request
    parity failures to `runtime_contract_violation` and exported KV payload
    failures to `runtime_kv_export_error`.
  - The request backcheck also compares route budget and hardware pressure, and
    when planning is attached, compute headroom and latency budget.
- `InferenceDiagnostics`, `InferenceDiagnosticsSummary`,
  `InferenceDiagnosticsRequestParitySummary`,
  `DiagnosticsPressureBand`, `RuntimeDiagnostics`, `RuntimeDiagnosticsSummary`,
  `RuntimeDiagnosticsContractSummary`, `RuntimeDiagnosticsRequestParitySummary`,
  `RuntimeHardwareDiagnosticsContractSummary`,
  `RuntimeHardwareDiagnosticsReport`, `RuntimeHardwareDiagnosticsSummary`,
  `EmbeddingDiagnostics`, `EmbeddingDiagnosticsSummary`,
  `EmbeddingCallDiagnostics`, `DeviceExecutionSource`
  - Adapter-facing diagnostics for root `src/reflection/**`,
    `src/runtime/**`, and `src/engine/**`.
  - `InferenceDiagnostics::from_request_envelope(...)` and
    `with_request_envelope(...)` seed route budget, generation budget, hardware
    pressure, and optional planning headroom/latency before runtime-specific
    diagnostics are attached.
  - `InferenceDiagnostics::diagnostics_summary()` exposes generation-budget
    presence, context truncation, route token counts, runtime KV exchange,
    runtime/embedding execution signals, hardware pressure band, recursive call
    count, and note count before root expands nested diagnostics.
  - `InferenceDiagnosticsSummary` helpers classify route activity, runtime KV
    exchange, runtime-or-embedding execution, recursive runtime use, notes, and
    complete diagnostics signal before nested diagnostics diffs.
  - `InferenceDiagnostics::request_parity_summary(...)` compares route budget,
    generation budget, hardware pressure, planning headroom/latency, and nested
    runtime diagnostics against the saved request envelope before response
    envelope parity expands verbose diagnostics diffs.
  - `InferenceDiagnosticsRequestParitySummary` helpers classify routing drift,
    missing/drifted generation budget, hardware pressure drift, planning
    headroom/latency drift, runtime diagnostics drift, missing required
    diagnostics reports, and aggregate request drift without parsing violation
    strings. Component-count helpers split routing, generation, hardware, nested
    runtime, and total diagnostics request drift for adapter tests. Accounting
    helpers expose aggregate diagnostics request drift presence and consistency
    before root expands verbose diagnostics parity.
    `diagnostics_request_parity_shape_is_clean()` and
    `can_accept_inference_diagnostics_request_parity()` are the compact
    response-diagnostics request parity gates.
  - `EmbeddingDiagnostics::diagnostics_summary()` exposes query embedding
    source/dimensions, optional memory-write embedding, gist-write source mix,
    runtime/fallback call counts, and total call count before embedding facts
    are folded into `InferenceDiagnostics`.
    Embedding summary helpers split query dimension, memory-write, gist-write,
    runtime/fallback, and mixed-source activity signals from zero query
    dimensions, memory-write shape drift, gist count drift, and runtime/fallback
    call-count drift. `embedding_accounting_is_consistent()` and
    `embedding_summary_is_clean()` keep the raw shape contract explicit;
    `can_use_embedding_diagnostics()` is the adapter-facing gate before root
    folds embedding facts into response diagnostics.
  - `RuntimeDiagnostics::from_request_envelope(...)` and
    `with_request_envelope(...)` seed missing model id, selected adapter,
    architecture, imported KV count, and KV precision from the request envelope
    without overwriting runtime-reported values.
  - `RuntimeDiagnostics::contract_violations(...)` checks model id,
    architecture, adapter, local-window, and KV precision claims against core
    metadata, architecture, and execution context.
  - `RuntimeDiagnostics::contract_summary(...)` exposes the same diagnostics
    contract checks as structured identity, architecture, adapter, and KV
    precision problem counts before root expands violation strings. Use
    `diagnostics_contract_shape_is_clean()` and
    `can_accept_runtime_diagnostics_contract()` before accepting runtime
    diagnostics claims.
  - `RuntimeDiagnostics::diagnostics_summary()` exposes model/adapter presence,
    architecture signal, layer-mode count, device execution source, forward/KV
    signal presence, KV exchange count, and KV precision validity before root
    expands field-level diagnostics diffs.
  - `RuntimeDiagnosticsSummary` helpers expose runtime identity, runtime KV
    exchange presence, and complete runtime signal state before root expands
    nested diagnostics payloads. Focused component helpers split runtime
    identity, architecture/layer-mode, device execution, forward, KV activity,
    and precision signals from missing identity, missing/malformed
    architecture, missing activity, and missing precision problems. Signal
    component counts are observational and may feed routing or diagnostics
    labels; `runtime_diagnostics_shape_is_clean()` and
    `can_use_runtime_diagnostics()` are the compact diagnostics-shape gates.
  - `RuntimeDiagnostics::request_parity_summary(...)` compares runtime
    diagnostics against the saved `RuntimeRequestEnvelope` for model, adapter,
    architecture, imported/exported KV bounds, and KV precision before root
    expands verbose response diagnostics diffs.
  - `RuntimeDiagnosticsRequestParitySummary` helpers classify model, adapter,
    architecture, missing report fields, KV count/export-bound, and precision
    drift without parsing request-parity violation text. Component-count helpers
    split missing reports, identity drift, architecture drift, KV drift,
    precision drift, and total runtime diagnostics drift for response adapters.
    Runtime drift accounting helpers keep missing reports, identity,
    architecture, KV, and precision counts aligned with the aggregate parity
    state. `runtime_request_parity_shape_is_clean()` and
    `can_accept_runtime_diagnostics_request_parity()` are the adapter-facing
    gates for accepting nested runtime diagnostics parity.
  - `RuntimeDiagnostics::hardware_contract_violations(...)` checks device
    profile, primary/fallback lanes, and memory mode against `HardwarePlan`.
  - `RuntimeDiagnostics::hardware_contract_summary(...)` exposes those hardware
    diagnostics checks as structured device-profile, lane, and memory-mode
    signals/problem counts before root expands violation strings. Missing
    runtime-reported hardware fields are clean but signal-free; malformed public
    summary shape is counted as a hardware contract problem. Use
    `hardware_contract_shape_is_clean()` and
    `can_accept_runtime_hardware_contract()` before trusting runtime-reported
    device execution claims.
  - `RuntimeDiagnostics::hardware_acceptance_report(...)` wraps those hardware
    checks in an adapter-facing report and maps failures to
    `runtime_contract_violation`.
  - `RuntimeHardwareDiagnosticsReport::diagnostics_summary()` exposes accepted
    state, hardware violation count, and mapped failure-report count before root
    converts the report into runtime errors.
  - `RuntimeHardwareDiagnosticsReport::failure_batch_summary()` and
    `primary_failure_summary()` expose hardware diagnostics failure class mix
    and first-failure shape before root formats device execution mismatches.
  - `RuntimeHardwareDiagnosticsSummary` helpers keep clean acceptance and
    failure-report/violation count parity structured at runtime response gates.
    Focused helpers split hardware violation presence, mapped failure-report
    presence, aggregate hardware-acceptance problem counts, and hardware
    acceptance accounting consistency. `hardware_acceptance_shape_is_clean()`
    and `can_accept_runtime_hardware_diagnostics()` name the acceptance gate
    before root expands device execution violation strings.
- `RuntimeMetadata`, `RuntimeMetadataShapeSummary`
  - Runtime/model capabilities: model id, tokenizer, context window, embedding
    dimensions, KV import/export support, KV block limits, hot/cold KV precision.
  - `RuntimeMetadata::shape_summary()` exposes context, embedding, KV exchange,
    block-capacity, and KV precision facts before adapters compute max-token or
    runtime request budgets. `RuntimeMetadataShapeSummary` component helpers
    split context/embedding, KV import/export capability, and precision signals
    from KV support/capacity contradictions and invalid precision problems, so
    root adapters can classify missing facts separately from malformed runtime
    ABI metadata. Use `metadata_shape_is_clean()` and
    `can_use_runtime_metadata_contract()` as the compact metadata gate before
    expanding field-level runtime ABI diagnostics. Use
    `runtime_metadata_adapter_missing_component_count()`,
    `runtime_metadata_adapter_blocker_component_count()`,
    `runtime_metadata_adapter_accounting_is_consistent()`, and
    `can_commit_runtime_metadata_adapter()` before root publishes concrete
    runtime adapter metadata; unknown context or embedding stays shape-clean but
    is not adapter-commit ready.
- `RuntimeGenerationBudget`
  - Adapter-facing result for requested context tokens, planned context tokens,
    effective max generation tokens, and context truncation/exhaustion.
  - Focused helpers split requested context overflow, requested generation
    deficit, hard context exhaustion, and soft context limiting before adapters
    expand verbose runtime planning diagnostics. Shape helpers validate that
    requested context, generated-token cap, planned context, and known context
    bounds remain internally consistent; root should use
    `context_budget_shape_is_clean()` and `can_use_backend_max_tokens()` before
    passing the planned max-token value to a concrete backend. Use
    `backend_request_signal_component_count()`,
    `backend_request_blocker_component_count()`,
    `backend_request_accounting_is_consistent()`, and
    `can_commit_backend_max_tokens()` at the backend request-send boundary so
    soft context truncation remains observable while context exhaustion blocks
    the concrete request.
    `RuntimeBackendMaxTokensCommitSummary` and
    `RuntimeBackendMaxTokensCommitAction` package that final max-token decision
    as commit, return context exhaustion, or repair malformed public
    context-budget accounting while preserving `backend_max_tokens` only on the
    commit path.
- `RuntimePlanningDigest`, `RuntimePlanningSummary`,
  `RuntimePlanningReadinessStage`, `RuntimePlanningReadinessSummary`,
  `RuntimePlanningAcceptanceReport`, `RuntimePlanningAcceptanceSummary`,
  `RuntimePlanningAcceptanceCommitSummary`,
  `RuntimePlanningAcceptanceCommitAction`,
  `RuntimePlanningFailureReturnReport`, `RuntimePlanningFailureReturnSource`,
  `RuntimePlanningFailureReturnSummary`,
  `RuntimePlanningKvExchange`, `RuntimePlanningKvClampSummary`,
  `RuntimePlanningKvClampReason`, `RuntimePlanningManifestKvBridgeSummary`
  - Adapter-facing planning summary that combines `InferenceRequest`
    context/max-token limits, runtime-clamped `AdapterExecutionContext`,
    adapter observation selection, and `FhtDkeBudgeter` output before root builds
    the runtime request JSON.
  - Reports whether context limited generation, whether KV prefetch was clamped
    by runtime metadata limits, FHT-DKE budget limits, or both, and which
    adapter was selected for execution.
  - Carries `AdapterSelectionReport` so root planning tests can compare allowed
    adapter count, matching observations, rejected observations, matched
    fraction, fallback usage, and fallback reason before backend request
    construction.
  - `backend_max_tokens()` and `planned_kv_exchange()` expose the exact values
    root adapters should compare with the concrete runtime request.
  - `kv_prefetch_clamp_summary()` exposes requested, runtime-clamped, and
    planned KV import counts plus runtime-metadata and FHT-DKE reduction counts
    before root mutates concrete KV import candidates.
  - `manifest_kv_bridge_summary(...)` derives manifest-aware import and export
    plans from `RuntimePlanningDigest::planned_kv_exchange()` so root can check
    manifest KV policy, runtime metadata capacity, architecture shape, and
    planning import/export counts in one adapter gate before materializing
    imported or exported runtime KV blocks. Use
    `manifest_kv_bridge_shape_is_clean()` and
    `can_use_runtime_planning_manifest_kv_bridge()` before treating a manifest
    as the source for both planned KV directions.
  - KV clamp helpers expose reduction-count parity, block-count parity,
    bounded requested/runtime/planned import counts, clamp-reason parity,
    unclamped/runtime-only/FHT-only/combined clamp state, clamp shape problem
    counts, accounting consistency, clean shape, and
    `RuntimePlanningSummary::kv_clamp_is_consistent()` before root drops or
    defers concrete KV candidates. Use
    `can_use_runtime_planning_kv_clamp()` as the compact clamp gate before root
    trusts planned imported KV counts.
  - `fht_dke_summary()` exposes dense/routed fractions, route pressure,
    token-split validity, routed-work state, routed tokens per KV exchange
    block, and planned KV exchange pressure at the same planning boundary.
  - `planning_summary()` gives root one structured pre-request report for
    generation budget, adapter selection/fallback, matched observations,
    FHT-DKE budget, KV exchange clamp reason, and hardware pressure before
    verbose field-level parity checks.
  - `RuntimePlanningSummary` helpers classify adapter selections from
    observations, missing allowed candidates, all-rejected observations,
    missing observations, fallback reasons, adapter-selection blockers,
    observation/fallback signal counts, hard context exhaustion, and soft
    context limiting before root expands verbose planning violations.
  - Pre-request gate helpers split backend request readiness blockers
    (generation, parallelism, adapter selection, FHT-DKE token split) from the
    full FHT-DKE budget commit blockers and KV-clamp consistency problems,
    while route-pressure, routed-work, routed-KV, clamp, FHT-DKE clamp, and
    adapter observation gaps remain visible classification signals.
    `pre_request_gate_signal_component_count()` aggregates those non-blocking
    observation/pressure signals; it does not add to
    `pre_request_gate_problem_component_count()`. Root should use
    `pre_request_gate_shape_is_clean()` and `can_send_backend_request()` as the
    compact send gate so a request-ready plan cannot bypass empty-budget,
    public FHT-DKE summary, or KV clamp drift. Use
    `backend_request_commit_signal_component_count()`,
    `backend_request_commit_blocker_component_count()`,
    `backend_request_commit_accounting_is_consistent()`, and
    `can_commit_backend_request()` before root writes the planning summary into
    a concrete backend request envelope or runtime JSON payload.
  - `RuntimePlanningReadinessSummary` combines
    `FhtDkePlanningReadinessSummary` and `RuntimePlanningSummary` into one
    staged adapter gate. It reports FHT-DKE planning, runtime pre-request, and
    FHT-DKE/runtime-boundary stages, preserves per-stage signal/blocker counts,
    and checks that the runtime planning summary still carries the same
    FHT-DKE budget facts before backend request or KV import mutation.
  - `acceptance_report()` gives root a planning gate before backend request
    construction; failures map context exhaustion to `runtime_context_exhausted`
    and malformed planning state, including missing allowed adapter candidates,
    to `runtime_contract_violation`.
  - `RuntimePlanningAcceptanceReport::acceptance_summary()` exposes accepted
    state, planning violation count, context exhaustion, contract failure count,
    and mapped failure-report count before root expands verbose violation text.
  - `RuntimePlanningAcceptanceReport::failure_batch_summary()` and
    `primary_failure_summary()` expose mapped planning failure class mix and
    first-failure shape directly from the report before root formats runtime
    errors.
  - `RuntimePlanningAcceptanceReport::commit_summary()` returns
    `RuntimePlanningAcceptanceCommitSummary` with
    `RuntimePlanningAcceptanceCommitAction`, mapped failure reports, primary
    failure summary, failure batch, commit/failure booleans, formatter
    readiness, and acceptance accounting so root can choose between serializing
    the backend request and returning a runtime failure from one object.
    `failure_return_summary()` and `runtime_failure_return_report()` expose the
    same pre-formatting gate and materialized primary failure shape for context
    exhaustion and planning contract failures.
  - `RuntimePlanningAcceptanceSummary` helpers expose planning-violation
    presence, context-exhaustion, contract-failure, mapped failure-report,
    aggregate planning-acceptance problem component counts, clean acceptance,
    accounting consistency, accepted-state/failure parity, shape problem counts,
    and failure-report/count parity for root tests that should not parse
    violation strings. Use `planning_acceptance_shape_is_clean()` and
    `can_accept_runtime_planning()` as the adapter-facing acceptance gates.
- `RecursiveSchedulerConfig`, `RecursiveScheduleDigest`,
  `RecursiveScheduleSummary`, `RecursiveScheduleValidationSummary`,
  `RecursiveChunk`, `RecursiveMergeRound`, `RecursiveExecutionWave`
  - Adapter-facing recursive runtime schedule contract for root
    `src/recursive_scheduler/**`.
  - Mirrors root token estimation, chunk/overlap planning, merge rounds, and
    execution waves without depending on root generation context or runtime
    execution.
  - Runtime-unit helpers expose single-pass/empty schedules, chunk runtime
    calls, merge overhead calls, max execution-wave width, and whether
    parallelism was requested or actually used.
  - `RecursiveScheduleDigest::schedule_summary()` and lightweight
    `RecursiveScheduleSummary` helpers split recursion, chunk, merge,
    execution-wave, and requested-parallelism signals from scheduler shape,
    recursion shape, and execution-wave shape problems before root expands
    concrete chunk rows. Use `schedule_shape_is_clean()` for public summary
    shape and `can_use_recursive_schedule()` before root treats a schedule as
    executable work; empty schedules remain clean but not usable.
  - `RecursiveScheduleDigest::validation_summary()` and
    `RecursiveScheduleSummary::validation_summary(...)` expose valid state plus
    shape/chunk/merge/execution-wave violation counts before root expands
    recursive scheduling violations. Validation helpers expose focused failure
    component counts, payload-failure presence, exact violation-count
    accounting, component accounting, aggregate problem presence, and clean
    validation readiness. `validation_shape_is_clean()` and
    `can_accept_recursive_schedule_validation()` are the compact validation
    acceptance gates.
- `RuntimeAdapter`, `AdapterObservation`, `AdapterSelection`,
  `AdapterSelectionReport`, `AdapterFallbackReason`, `AdapterExecutionContext`,
  `AdapterExecutionContextSummary`, `AdapterExecutionContextCommitSummary`,
  `AdapterExecutionContextCommitAction`, `AdapterRuntimeClampSummary`,
  `AdapterRuntimeClampCommitSummary`, `AdapterRuntimeClampCommitAction`,
  `AdapterFailureReturnReport`, `AdapterFailureReturnSource`,
  `AdapterFailureReturnSummary`,
  `AdapterSelectionCommitSummary`, `AdapterSelectionCommitAction`,
  `AdapterSelectionRuntimeSummary`, `AdapterSelectionRuntimeCommitSummary`,
  `AdapterSelectionRuntimeCommitAction`
  - Root hardware/runtime adapter vocabulary without depending on root
    `src/hardware` or `src/runtime` types.
  - `AdapterExecutionContext::clamp_for_runtime` keeps KV prefetch and precision
    inside runtime-advertised limits.
  - `AdapterExecutionContext::runtime_clamp_summary(...)` exposes before/after
    execution context summaries, runtime metadata shape, KV prefetch reduction,
    precision clamp state, and preservation checks for adapter count, pressure,
    execution shape, token budgets, and disk-spill state. Runtime clamp helpers
    keep legal KV-prefetch/precision reductions as signals while preservation
    drift, non-monotonic runtime-limit changes, and malformed post-clamp context
    remain problem components. Use
    `runtime_clamp_commit_signal_component_count()`,
    `runtime_clamp_commit_blocker_component_count()`,
    `runtime_clamp_commit_accounting_is_consistent()`, and
    `can_commit_runtime_clamp()` before root commits the clamped context into
    adapter selection, planning digest, or backend request state. Prefer
    `commit_summary()` when root needs `AdapterRuntimeClampCommitAction`,
    mapped runtime failure reports, primary failure summary, failure batch,
    formatter readiness, and commit-decision accounting in one object. Adapter
    commit summaries expose `failure_return_summary()` and
    `runtime_failure_return_report()` with `AdapterFailureReturnSummary` and
    `AdapterFailureReturnReport`, so root can convert adapter selection,
    runtime adapter execution, execution-context, and runtime-clamp blockers
    into a primary runtime failure without re-reading each commit summary.
  - `AdapterExecutionContext::context_summary()` exposes adapter count,
    hardware pressure, compute headroom, latency budget, parallel chunks, KV
    prefetch, KV precision, KV token budgets, and disk-spill state after root
    hardware mapping or runtime clamping. Context summary helpers split adapter
    candidates, pressure, execution capacity, KV budget, and precision signals
    from missing adapters, out-of-range pressure, invalid parallelism, and
    precision-shape problems. Use `adapter_execution_context_commit_*` helpers
    and `can_commit_adapter_execution_context()` before root builds routing or
    backend request state from the context. Prefer `commit_summary()` when root
    needs `AdapterExecutionContextCommitAction`, mapped runtime failure reports,
    primary failure summary, failure batch, formatter readiness, and
    commit-decision accounting for the mapped hardware/runtime execution
    context.
  - `AdapterExecutionContext::select_adapter(...)` ranks adapter observations
    inside the current execution context and falls back to the first allowed
    adapter when no observation is usable.
  - `select_adapter_report(...)` adds allowed-adapter, observation,
    matching-observation, rejected-observation, matched-fraction, and fallback
    reason summaries for adapter parity diagnostics.
  - `AdapterSelectionReport` helpers expose selected-from-observation,
    all-rejected observations, no-allowed-adapter fallback, and
    no-matching-observation fallback as direct adapter-boundary predicates.
    Selection report helpers also split catalog/observation/fallback signals
    from malformed matching-observation counts and fallback-reason drift, with
    commit signal/blocker counts, commit accounting, and commit-ready adapter
    selection gates before root builds a backend request. Use
    `adapter_selection_commit_*` helpers and `can_commit_adapter_selection()`;
    missing allowed adapters are commit blockers, while
    no-matching-observation fallback remains an observational signal when a
    selected adapter is usable. Prefer `commit_summary()` when root needs
    `AdapterSelectionCommitAction`, mapped runtime failure reports, primary
    failure summary, failure batch, formatter readiness, and
    commit-decision accounting for adapter selection. The same adapter
    failure-return projection is available from selection, runtime-selection,
    execution-context, and runtime-clamp commit summaries.
  - `AdapterExecutionContext::selection_runtime_summary(...)` compares a
    planned adapter selection report with the runtime-reported selected adapter
    so root response diagnostics can detect missing, drifted, or disallowed
    adapter execution before expanding verbose diagnostics.
  - `AdapterSelectionRuntimeSummary` helpers classify allowed-catalog and
    matching-observation signals, runtime-report confirmation signals,
    fallback-reason shape, missing/drifted/disallowed runtime adapter problems,
    aggregate runtime adapter problems, accounting consistency, and clean
    runtime adapter execution before root builds verbose diagnostics. Use
    `runtime_adapter_execution_commit_*` helpers and
    `can_commit_runtime_adapter_execution()` as the compact runtime adapter
    parity gate. Prefer `commit_summary()` when root needs
    `AdapterSelectionRuntimeCommitAction`, mapped runtime failure reports,
    primary failure summary, failure batch, formatter readiness, and
    commit-decision accounting for runtime-reported adapter execution.
- `DeviceClass`, `DeviceTier`, `DeviceProfileDescriptor`,
  `DeviceProfileDescriptorSummary`, `ComputeLane`, `DeviceMemoryMode`,
  `HardwareLoadSnapshot`, `HardwareLoadSnapshotSummary`, `HardwareLoadKind`,
  `HardwareLoadSnapshotCommitSummary`, `HardwareLoadSnapshotCommitAction`,
  `DeviceExecutionPlan`, `DeviceExecutionPlanSummary`,
  `DeviceExecutionPlanCommitSummary`, `DeviceExecutionPlanCommitAction`,
  `DeviceExecutionAdapterSummary`, `DeviceExecutionAdapterCommitSummary`,
  `DeviceExecutionAdapterCommitAction`, `HardwarePlan`, `HardwarePlanSummary`,
  `HardwarePlanCommitSummary`, `HardwarePlanCommitAction`,
  `HardwareAdapterBridgeSummary`, `HardwareAdapterBridgeCommitSummary`,
  `HardwareAdapterBridgeCommitAction`,
  `HardwareRuntimeReadinessSummary`, `HardwareRuntimeReadinessStage`,
  `HardwareRuntimeCommitSummary`, `HardwareRuntimeCommitAction`,
  `HardwarePressureBand`, `HardwareAllocator`
  - Adapter-facing hardware contract for root `src/hardware/**`.
  - `DeviceClass::{supported_profiles, explicit_profiles, descriptor}` and
    `DeviceProfileDescriptor::descriptor_summary()` expose device/tier/scope
    and alias-count parity before root compares probe descriptor tables.
  - Computes pressure, latency budget, KV token budgets, execution lanes,
    adapter hints, and an `AdapterExecutionContext` without taking over root
    probing or production device validation.
  - `HardwareAdapterBridgeSummary` compares the mapped `HardwarePlan`,
    `DeviceExecutionPlan`, and `AdapterExecutionContext` before root hands the
    execution context to runtime planning. Focused helpers split adapter-count,
    pressure/headroom, latency, parallelism, KV-prefetch, precision,
    token-budget, and disk-spill drift component counts, aggregate drift
    presence, and bridge accounting consistency before root expands full
    hardware diagnostics.
  - `HardwareLoadSnapshot::snapshot_summary()` exposes normalized CPU/GPU/RAM
    and disk load, dominant load kind, device tier, pressure value, and pressure
    band before root compares full hardware plans. Snapshot summary helpers
    split bounded load/pressure/tier signals from out-of-range load values,
    pressure-band drift, and device-tier drift, with a clean shape gate for
    root hardware probe adapters before planning. Use
    `can_use_hardware_snapshot()` before accepting mapped probe input. Use
    `hardware_snapshot_commit_signal_component_count()`,
    `hardware_snapshot_commit_blocker_component_count()`,
    `hardware_snapshot_commit_accounting_is_consistent()`, and
    `can_commit_hardware_snapshot()` before root commits normalized hardware
    probe facts into planning. Prefer `commit_summary()` when root needs
    `HardwareLoadSnapshotCommitAction`, mapped runtime failure reports, primary
    failure summary, failure batch, formatter readiness, and snapshot
    commit-decision accounting before constructing a hardware plan.
  - `DeviceExecutionPlan::execution_summary()` exposes primary/fallback lane,
    memory mode, adapter-hint count, parallel chunk count, KV prefetch count,
    KV precision, and disk-spill permission before root compares string
    diagnostics or full hardware plans.
  - `DeviceExecutionPlanSummary` helpers expose parallel and KV prefetch
    capacity, distinct fallback lanes, CPU/GPU/disk primary execution,
    disk-backed memory, compressed KV precision, hot/cold precision parity, and
    aggregate execution-shape risk component counts. Focused helpers split
    adapter hints, execution capacity, primary/fallback lane, memory mode, KV
    precision, and constrained execution signals from missing adapter hints,
    missing capacity, and precision problems. The legacy
    `execution_shape_risk_component_count()` remains an aggregate of execution
    problems plus constrained execution signals for existing root parity tests.
    `execution_shape_is_clean()` and `can_use_device_execution_plan()` are the
    strict no-problem execution shape gates; constrained but usable plans can
    still flow through the adapter bridge when the bridge is lossless. Use
    `hardware_execution_signal_component_count()`,
    `hardware_execution_blocker_component_count()`,
    `hardware_execution_accounting_is_consistent()`, and
    `can_commit_device_execution_plan()` before root publishes a device
    execution plan as hardware adapter input. Prefer `commit_summary()` when
    root needs `DeviceExecutionPlanCommitAction`, mapped runtime failure
    reports, primary failure summary, failure batch, formatter readiness, and
    commit-decision accounting before constructing the hardware adapter bridge
    or larger hardware runtime readiness summary.
  - `DeviceExecutionPlan::adapter_hint_summary()` groups adapter hints into
    portable, CPU, GPU, neural, multi-device, and custom families before root
    compares gate rows or adapter CSV strings. Adapter summary helpers expose
    fallback count, accelerator count, family member count, adapter-count
    parity, family spread, fallback-only or accelerator-only classifications,
    family signals, family problem counts, accounting consistency, and clean
    family shape before root compares adapter rows. Use
    `can_use_adapter_family()` before accepting non-empty adapter hints. Use
    `adapter_family_commit_signal_component_count()`,
    `adapter_family_commit_blocker_component_count()`,
    `adapter_family_commit_accounting_is_consistent()`, and
    `can_commit_device_execution_adapters()` before root commits mapped adapter
    hints to the bridge. Prefer `commit_summary()` when root needs
    `DeviceExecutionAdapterCommitAction`, mapped runtime failure reports,
    primary failure summary, failure batch, formatter readiness, and
    adapter-family commit-decision accounting.
  - `HardwarePlan::plan_summary()` exposes pressure band, parallelism reduction,
    KV prefetch contraction, hot KV compression, latency budget, disk-spill, and
    adapter-count facts for root parity tests before accepting hardware plans.
    Plan summary helpers count constrained pressure, reduced parallelism,
    minimal prefetch, compressed hot KV, latency budget, disk-spill absence, and
    notes as non-blocking plan constraint signals before root expands device gate
    rows. `plan_constraint_component_count()` remains a compatibility alias for
    `plan_constraint_signal_component_count()`. Plan public-shape helpers count
    device-tier drift, pressure/headroom shape, missing adapter/parallel
    capacity, and KV precision problems separately from non-blocking
    constraints. Use `hardware_plan_shape_is_clean()` and
    `can_use_hardware_plan()` as compact plan-summary accounting gates; pressure
    constraints remain observations. Use
    `hardware_plan_commit_signal_component_count()`,
    `hardware_plan_commit_blocker_component_count()`,
    `hardware_plan_commit_accounting_is_consistent()`, and
    `can_commit_hardware_plan()` before root commits a mapped hardware plan to
    execution planning or the adapter bridge. Prefer `commit_summary()` when
    root needs `HardwarePlanCommitAction`, mapped runtime failure reports,
    primary failure summary, failure batch, formatter readiness, and plan
    commit-decision accounting before deriving the device execution plan.
  - `HardwarePlan::adapter_bridge_summary()` compares `HardwarePlanSummary`,
    `DeviceExecutionPlanSummary`, and the derived `AdapterExecutionContextSummary`
    so root can assert hardware-to-runtime execution mapping is lossless before
    request planning. Bridge helpers classify adapter-count, pressure, latency,
    parallelism, KV-prefetch, precision, token-budget, disk-spill, and aggregate
    drift before verbose device ABI rows. Use `adapter_bridge_shape_is_clean()`
    and `can_use_hardware_adapter_bridge()` before runtime planning consumes the
    derived adapter context. Use
    `adapter_bridge_preservation_signal_component_count()`,
    `hardware_adapter_bridge_blocker_component_count()`,
    `hardware_adapter_bridge_accounting_is_consistent()`, and
    `can_commit_hardware_adapter_bridge()` before root commits the derived
    `AdapterExecutionContext`. Prefer `commit_summary()` when root needs
    `HardwareAdapterBridgeCommitAction`, mapped runtime failure reports,
    primary failure summary, failure batch, formatter readiness, and
    commit-decision accounting before building the larger hardware runtime
    readiness summary.
  - `HardwareRuntimeReadinessSummary` combines load snapshot, hardware plan,
    device execution, and adapter bridge gates into one hardware handoff
    checklist. `HardwareRuntimeReadinessStage`, `first_unready_stage()`, and
    `first_blocking_stage()` let root expand the first malformed probe, plan,
    execution, or bridge report without parsing full diagnostics rows. Use
    `HardwarePlan::runtime_readiness_summary(...)`,
    `hardware_runtime_accounting_is_consistent()`, and
    `can_commit_hardware_runtime()` before runtime planning consumes the derived
    `AdapterExecutionContext`. `failure_reports()`, `failure_batch_summary()`,
    `primary_failure_summary()`, and `commit_summary()` map hardware runtime
    blockers to runtime contract violations and expose
    `HardwareRuntimeCommitAction` so root can choose between committing the
    derived adapter context and returning a runtime failure before the
    manifest/device handoff gate.
  - `ComputeLane` and `DeviceMemoryMode` parse root-style diagnostic strings so
    runtime response diagnostics can be compared with core hardware plans.
- `RuntimeManifestDigest`, `TransformerRuntimeArchitecture`,
  `TransformerRuntimeArchitectureSummary`, `RuntimeKvPolicy`,
  `RuntimeKvPolicySummary`, `RuntimeQuantizationPolicy`,
  `RuntimeQuantizationPolicySummary`, `QuantizationBits`,
  `RuntimeManifestValidation`, `RuntimeManifestValidationSummary`,
  `RuntimeManifestValidationCommitSummary`,
  `RuntimeManifestValidationCommitAction`, `RuntimeManifestAbiSummary`,
  `RuntimeManifestAdapterCompatibilitySummary`,
  `RuntimeManifestExecutionCompatibilitySummary`,
  `RuntimeDeviceHandoffReadinessSummary`, `RuntimeDeviceHandoffStage`,
  `RuntimeDeviceHandoffCommitSummary`, `RuntimeDeviceHandoffCommitAction`,
  `ManifestFailureReturnReport`, `ManifestFailureReturnSource`,
  `ManifestFailureReturnSummary`
  - Adapter-facing runtime ABI digest for root `src/runtime_manifest/**`.
  - Keeps architecture validation, KV import/export policy, hot/cold KV
    quantization, and preferred adapter selection in core vocabulary without
    taking over root asset validation or device probing.
  - `RuntimeManifestValidation::failure_reports()` maps invalid manifest shape
    to `runtime_contract_violation`; warnings remain non-blocking through
    `warnings_only()`.
  - `RuntimeManifestValidation::failure_batch_summary()` and
    `primary_failure_summary()` expose manifest validation failure class mix and
    first-failure shape directly from the validation report before root formats
    runtime errors.
  - `RuntimeManifestValidation::commit_summary()` returns a single adapter
    decision object with `RuntimeManifestValidationCommitAction`, mapped
    failure reports, primary failure summary, failure batch, commit/failure
    booleans, and validation commit accounting so root can choose between
    accepting a manifest and returning a runtime contract failure before
    request planning reads ABI rows.
    Its `failure_return_summary()` and `runtime_failure_return_report()` expose
    the shared manifest failure-return projection, so invalid manifests can be
    converted to the primary `RuntimeFailureReport` / `InferenceError` without
    re-reading validation counters. Clean and warnings-only manifests do not
    materialize a return report.
  - `RuntimeManifestValidation::validation_summary()` exposes passed state,
    error count, warning count, warnings-only status, and failure-report count
    before root expands manifest validation text.
  - Validation summary helpers expose blocking-failure state, clean pass,
    warnings-only pass, error/warning/mapped failure-report signal components,
    aggregate validation signal presence, accounting consistency, and
    failure-report/error parity before root maps manifest errors to runtime
    contract failures.
    Focused helpers split non-blocking validation activity from blocking
    validation problems: warnings and mapped reports are visible activity,
    while blocking errors, failure-report drift, and warnings-only flag drift
    contribute to `validation_problem_component_count()`.
    Use `runtime_manifest_validation_commit_signal_component_count()`,
    `runtime_manifest_validation_commit_blocker_component_count()`,
    `runtime_manifest_validation_commit_accounting_is_consistent()`,
    `runtime_manifest_validation_commit_is_clean()`, and
    `can_commit_runtime_manifest_validation()` as the first manifest adapter
    commit gate. Warnings-only passes remain observable signals but are
    commit-ready; errors, failure-report drift, and warnings-only flag drift are
    blockers.
  - `TransformerRuntimeArchitecture::architecture_summary()` exposes layer,
    hidden, attention/KV head, local-window, and attention-head-dimension shape
    before root compares full manifest ABI. Architecture summary helpers split
    dimension and attention-geometry signals from zero-dimension, invalid
    head geometry, and local-window/context problems, with accounting checks
    before root expands transformer ABI rows. Use
    `transformer_runtime_architecture_commit_*` helpers for compact
    signal/blocker/accounting gates, with
    `can_commit_transformer_runtime_architecture(...)` as the adapter commit
    gate.
  - `RuntimeKvPolicy::kv_policy_summary()` exposes import/export capability,
    block capacity, and limit/capability consistency before root materializes
    runtime KV exchange plans. KV policy helpers split capability/capacity
    signals from import/export limit-capability drift and expose accounting for
    compact runtime-device ABI gates. `runtime_kv_policy_commit_*` helpers and
    `can_commit_runtime_kv_policy()` are the adapter commit checks for both
    disabled and enabled-but-consistent KV policies.
  - `RuntimeManifestDigest::abi_summary()` exposes effective context,
    embedding, transformer shape, KV exchange limits, quantization widths, and
    supported adapter count without requiring root adapters to parse summary
    strings. `RuntimeManifestAbiSummary` focused helpers split context,
    transformer, KV exchange, quantization, and adapter ABI signals from missing
    context/shape, local-window overflow, KV capability/limit contradictions,
    invalid precision, cold-wider-than-hot, and missing adapter catalog
    problems. Use `abi_accounting_is_consistent()` before expanding full ABI
    diffs, and `abi_shape_is_clean()` or `can_use_runtime_manifest_abi()` when
    root only needs the compact ABI accept/reject result. Use
    `manifest_adapter_signal_component_count()`,
    `manifest_adapter_blocker_component_count()`,
    `manifest_adapter_accounting_is_consistent()`, and
    `can_commit_runtime_manifest_adapter()` when root commits the manifest ABI
    digest as the planning source for runtime/device adapters.
  - `RuntimeQuantizationPolicy::quantization_summary()` exposes hot/cold KV and
    optional weight precision, compressed-KV state, and cold-not-wider-than-hot
    parity before root compares the full manifest ABI. Quantization helpers
    split supported precision, compression, and weight-precision signals from
    invalid public summary widths and cold-wider-than-hot problems. Use
    `runtime_quantization_policy_commit_*` helpers and
    `can_commit_runtime_quantization_policy()` before root builds runtime ABI
    rows.
  - `adapter_compatibility_summary(...)` compares manifest-supported adapters
    against the mapped hardware execution context and adapter observations
    before root enters runtime planning adapter selection. Compatibility helpers
    separate adapter-source problem counts from observation/fallback planning
    signals, so root can distinguish missing catalogs or disjoint execution
    hints from usable fallback selection before verbose device-gate rows.
    Focused helpers split adapter catalog, observation, and selection signals
    from adapter catalog and selection/source problems; use
    `runtime_manifest_adapter_compatibility_commit_signal_component_count()`,
    `runtime_manifest_adapter_compatibility_commit_blocker_component_count()`,
    `runtime_manifest_adapter_compatibility_commit_accounting_is_consistent()`,
    `runtime_manifest_adapter_compatibility_commit_is_clean()`, and
    `can_commit_runtime_manifest_adapter_compatibility()` before expanding
    adapter-source diagnostics. `can_use_runtime_adapter_plan()` is the compact
    root planning gate for manifest-supported adapters, hardware execution
    hints, selected adapter availability, selected adapter source, and clean
    compatibility accounting. `adapter_planning_signal_component_count()`
    remains a compatibility scalar that includes legacy adapter-source problem
    counts plus observation/fallback signals; do not use it as the blocking
    predicate.
  - `execution_compatibility_summary(...)` compares manifest KV capacity and
    quantization limits against the mapped `AdapterExecutionContext` before
    root builds runtime-device ABI rows. Helpers split import/export capacity,
    disabled import requests, KV prefetch limit drift, hot/cold precision drift,
    cold-wider-than-hot inversion, aggregate execution-contract problems, and
    accounting consistency. Use `execution_contract_shape_is_clean()` before
    `can_use_execution_kv_contract()` when root needs a compact runtime-device
    ABI gate. Signal helpers expose manifest KV capacity, execution KV prefetch
    request/acceptance, and manifest/execution precision coverage separately
    from device-gate problem components. Use
    `runtime_manifest_execution_device_commit_signal_component_count()`,
    `runtime_manifest_execution_device_commit_blocker_component_count()`,
    `runtime_manifest_execution_device_commit_accounting_is_consistent()`,
    `runtime_manifest_execution_device_commit_is_clean()`, and
    `can_commit_manifest_execution_device_gate()` at the runtime-device ABI
    boundary before root applies device-derived KV prefetch or precision rows.
  - `RuntimeDeviceHandoffReadinessSummary` combines hardware runtime readiness,
    runtime clamp readiness, and manifest execution-device readiness into the
    compact handoff gate immediately before request planning consumes the
    derived or clamped `AdapterExecutionContext`. `RuntimeDeviceHandoffStage`,
    `first_unready_stage()`, `first_blocking_stage()`, per-stage
    signal/blocker counts, `runtime_device_handoff_accounting_is_consistent()`,
    and `can_commit_runtime_device_handoff()` let root choose the first verbose
    hardware, clamp, or manifest-device report to expand without treating legal
    pressure or clamp signals as blockers. A clean runtime clamp is not
    sufficient to commit the handoff if a stricter manifest execution-device
    summary reports KV capacity or precision drift. `failure_reports()`,
    `failure_batch_summary()`, `primary_failure_summary()`, and
    `commit_summary()` map handoff blockers to runtime contract violations and
    expose `RuntimeDeviceHandoffCommitAction` so root can choose between
    committing the derived execution context and returning a runtime failure
    from one adapter-facing object. The commit summary also exposes
    `failure_return_summary()` and `runtime_failure_return_report()` with
    `ManifestFailureReturnSummary` / `ManifestFailureReturnReport`, preserving
    blocking stage text such as `manifest_execution_device`, and keeping
    manifest validation and runtime-device handoff failures on one
    commit-to-runtime-failure shape.
- `TaskProfile`, `HierarchyWeights`, `HierarchyWeightsSummary`,
  `ProfileHierarchyWeights`, `ProfileHierarchyWeightsSummary`,
  `ProfileHierarchyObservations`
  - Shared profile vocabulary for routing, attention, transformer planning, and
    higher-level agents.
  - Root `hierarchy::HierarchyWeights.convolution` maps to core `fusion`.
  - `HierarchyWeights::summary()` exposes normalized state, dominant focus, and
    total weight before root compares concrete global/local/fusion values.
    Focused helpers split active-weight, focus, and normalization signals from
    non-finite/negative weight shape, total drift, dominant-focus drift,
    normalization flag drift, and aggregate hierarchy problem counts before root
    expands concrete weight diffs. Use `hierarchy_shape_is_clean()` and
    `can_use_hierarchy_weights()` before feeding mapped hierarchy weights into
    route or transformer planning.
  - `ProfileHierarchyWeights::summary()` and
    `ProfileHierarchyObservations::{total, active_profile_count}` let root
    compare learned per-profile hierarchy state before moving controller
    ownership into core. Profile summary helpers expose normalized-profile and
    expected-focus signals separately from per-profile shape problems,
    normalized-profile drift, expected-focus drift, and accounting consistency.
    Observation helpers expose total/profile activity signals without forcing
    root to parse controller storage rows. Use
    `hierarchy_profile_shape_is_clean()`,
    `can_use_profile_hierarchy_weights()`,
    `profile_observation_shape_is_clean()`, and
    `can_use_profile_hierarchy_observations()` as the compact profile-state
    gate.
- `HierarchicalRouter`, `DefaultHierarchicalRouter`, `RoutingContext`,
  `RoutingDecision`, `RoutingDecisionSummary`, `RouteLayerCounts`,
  `RouteBudget`, `RouteBudgetReadinessSummary`,
  `RouteBudgetReadinessStage`, `RouteBudgetReadinessCommitAction`,
  `RouteBudgetReadinessCommitSummary`, `GenerationMetrics`,
  `RoutingFeedback`, `RoutingFeedbackSummary`,
  `RoutingFeedbackBatchSummary`
  - Lightweight hierarchical route choice across fast projection, local window,
    global attention, and fusion.
  - `RouteLayer::as_str()`, `RouteLayerCounts`, and
    `RoutingDecisionSummary::from_decisions(...)` give root adapter tests a
    stable layer-count, threshold-crossing, and score-range report before
    comparing individual token routes.
  - Route summaries expose fast-path, attention-route, all-attention,
    multi-layer, score-spread, layer-count parity, and route-budget parity
    helpers so root can classify route pressure before expanding token diffs.
    Focused helpers split route activity, layer, and score signals from
    layer/threshold partition drift, invalid score ranges, invalid threshold
    or attention fraction, and aggregate routing problem counts. Route-budget
    helpers expose attention/fast-token pressure signals separately from
    malformed budget fractions before root expands per-token router diffs. Use
    `routing_shape_is_clean()` and `route_budget_shape_is_clean()` for compact
    parity, then `can_use_route_summary()` and `can_use_route_budget()` before
    accepting non-empty route batches as adapter baselines.
  - `RouteBudgetReadinessSummary` combines a `RoutingDecisionSummary`, the
    `RouteBudget` consumed by downstream planning, and a parity stage into one
    adapter-facing checklist. It preserves `RouteBudgetReadinessStage` order,
    per-stage signal/blocker counts, first unready/blocking stage helpers,
    aggregate accounting, and `can_commit_route_budget_readiness()` so root can
    catch router-to-planning budget drift before FHT-DKE, attention, or
    transformer planning consumes the budget.
    `commit_summary()` packages that readiness into
    `RouteBudgetReadinessCommitSummary` so adapters can branch once before
    downstream pressure propagation: `CommitRouteBudget` exposes the committed
    `RouteBudget`, `WaitForRouteBudget` preserves a clean empty/unready route
    state without treating it as corruption, and `RepairRouteBudget` flags
    blocker or parity drift before attention, FHT-DKE, or transformer planning
    consumes route pressure.
  - `GenerationMetrics::routing_feedback(...)` preserves the root router's
    generation-quality scoring while giving adapters a core-owned observation
    path.
  - `RoutingFeedback::feedback_summary()` and `batch_summary(...)` expose
    profile distribution, quality/perplexity averages, low/high quality counts,
    and contradiction pressure before root compares adaptive threshold changes.
    Feedback helpers split low/high-quality and contradiction signals from
    invalid quality/perplexity shape, profile-count drift, quality-bucket drift,
    and aggregate feedback-batch accounting. Use `feedback_shape_is_clean()`,
    `feedback_batch_shape_is_clean()`, `can_use_routing_feedback()`, and
    `can_use_routing_feedback_batch()` before applying adaptive threshold
    observations.
- `RouterState`, `ProfileThresholds`, `ProfileObservations`
  - Lightweight router adaptation snapshot for persisting/restoring profile
    thresholds and observation counts while root router migration is staged.
  - `ProfileObservations::total()`, `active_profile_count()`, and
    `RouterState::has_observation_drift()` let root detect total/per-profile
    observation mismatch before replacing router state storage.
- `AttentionPolicy`, `ThresholdAttentionPolicy`,
  `ThresholdAttentionPolicySummary`, `AttentionCandidate`,
  `AttentionCandidateSummary`, `AttentionCandidateBatchSummary`,
  `AttentionDecision`, `AttentionDecisionSummary`,
  `AttentionSelectionReadinessSummary`, `AttentionSelectionReadinessStage`
  - Adaptive threshold port for selecting which routed tokens should pay the
    attention cost.
  - `ThresholdAttentionPolicy::policy_summary()` exposes base/min/max
    thresholds, learning rate, per-profile thresholds, bounded state, threshold
    spread, and adapted profile count before root compares individual
    observations or moves attention policy ownership.
  - `AttentionCandidate::candidate_summary()` and `batch_summary(...)` expose
    route-to-attention token, position, score, entropy, layer mix, attention
    candidate counts, and score/entropy pressure before root compares selected
    candidates.
  - Candidate batch helpers expose layer-count parity, fast/attention-only
    state, and multi-layer state before root maps selected attention tokens.
    Focused candidate helpers split legal token/attention/fast/score/entropy
    signals from empty-token, invalid score/entropy, attention-layer drift,
    candidate-count drift, score/entropy shape drift, attention-fraction drift,
    and aggregate candidate-batch accounting. Use `candidate_shape_is_clean()`
    and `candidate_batch_shape_is_clean()` for compact shape parity, then
    `can_use_attention_candidate()` and `can_use_attention_candidate_batch()`
    before accepting non-empty routed attention input.
  - `AttentionDecision::selection_fraction()` and `hit_selection_cap()` let root
    compare selected/rejected token pressure before moving attention selection.
  - `AttentionDecision::decision_summary()` also reports selected/rejected
    layer counts and attention-token counts, matching the route summary shape
    used by adapter parity tests.
  - Decision summary helpers expose selected/rejected accounting parity,
    selected/rejected attention fractions, rejected-attention pressure, and
    cap-pressure before root expands selected/rejected candidate diffs. Focused
    helpers split selection/cap/attention signals from selected/rejected count
    drift, selected-over-cap drift, invalid threshold, invalid selection
    fraction, and aggregate decision accounting. Use `decision_shape_is_clean()`
    and `can_use_attention_decision()` before replacing root attention
    selection.
  - `AttentionSelectionReadinessSummary` combines the mapped candidate batch,
    attention decision, and candidate-to-decision boundary into one
    adapter-facing checklist. It preserves
    `AttentionSelectionReadinessStage` order, per-stage signal/blocker counts,
    first unready/blocking stage helpers, layer/count parity helpers, aggregate
    accounting, and `can_commit_attention_selection_readiness()` so root can
    catch stale candidate batches or selection drift before transformer
    pressure and FHT-DKE planning consume the selected attention facts.
  - `ThresholdAttentionPolicySummary` helpers split bounded/base/spread/adapted
    threshold signals from non-finite thresholds, invalid learning rate, and
    out-of-bounds policy shape before root moves adaptive attention ownership.
    Use `threshold_policy_shape_is_clean()` and `can_use_threshold_policy()` as
    the compact threshold-policy gate.
- `TransformerPlanDigest`, `TransformerLayerBudget`,
  `TransformerLayerBudgetSummary`, `TransformerLayerBudgetBatchSummary`,
  `TransformerAttentionKind`, `TransformerPlanCounts`,
  `TransformerPlanSummary`, `TransformerPlanReadinessSummary`,
  `TransformerPlanReadinessStage`, `TransformerPlanningReadinessSummary`,
  `TransformerPlanningReadinessStage`, `TransformerPlanningPressureSummary`,
  `TransformerPlanningInput`, `TransformerPlannerContract`,
  `DefaultTransformerPlanner`, `TransformerTemplateKind`, `TransformerTemplate`,
  `TransformerForwardSummary`, `TransformerForwardBatchSummary`,
  `RuntimeKvExportPlan`, `RuntimeKvExportManifestPlanSummary`,
  `RuntimeKvExportPlanningSummary`,
  `RuntimeKvExportSummary`, `RuntimeKvExportBlockSummary`,
  `RuntimeKvExportReadinessSummary`,
  `RuntimeKvExportReadinessCommitSummary`,
  `RuntimeKvExportReadinessCommitAction`, `RuntimeKvExportReadinessStage`
  - Adapter-facing digest for transformer layer planning. It summarizes global,
    local-window, and fusion layer budgets without moving the root
    `src/transformer/**` planner implementation into core.
  - Root `transformer::AttentionKind::ConvolutionalFusion` maps to core
    `TransformerAttentionKind::Fusion`.
  - `TransformerLayerBudget::layer_summary()` exposes layer index, attention
    label, compute fraction, window size, and fusion state before root compares
    full per-layer planner rows.
    Layer summary helpers split legal compute/window/fusion/non-local signals
    from label, fusion-flag, compute, and window shape problems, with accounting
    checks before verbose planner diffs. Use `layer_budget_shape_is_clean()`
    and `can_use_transformer_layer_budget()` before an adapter accepts a compact
    layer-budget row.
  - `TransformerPlanDigest::layer_summaries()` and `layer_batch_summary()`
    expose a compact row-batch gate before root expands planner rows.
    `TransformerLayerBudgetBatchSummary` counts total, usable, and unusable
    rows plus aggregate signal/problem components. Use
    `layer_batch_shape_is_clean()` and
    `can_use_transformer_layer_budget_batch()` before accepting a non-empty
    root planner row set.
  - `TransformerPlanReadinessSummary` combines the route budget, compact plan
    summary, and layer-budget batch into one adapter-facing checklist. It
    preserves `TransformerPlanReadinessStage` order, per-stage signal/blocker
    counts, first unready/blocking stage helpers, aggregate accounting, and
    `can_commit_transformer_plan_readiness()` so root can gate
    route-budget-to-transformer-plan migration without expanding every layer
    row first.
  - `DefaultTransformerPlanner` mirrors the root planner's profile/template,
    hierarchy, route-pressure, layer-count, and base-window inputs so root can
    parity-test before replacing local planner decisions.
  - `RuntimeKvExportPlan` converts a forward vector and transformer layer
    summaries into runtime namespace KV blocks with stable layer/head/token
    slots.
  - `RuntimeKvExportPlan::from_manifest(...)` builds the export plan from a
    `RuntimeManifestDigest`, preserving manifest KV export policy limits after
    runtime metadata normalization. `manifest_plan_summary(...)` exposes
    manifest export capability, runtime export capability, requested export
    count, plan max blocks, and raw architecture layer/KV-head shape before root
    chooses whether exported-KV side effects are allowed. Use
    `manifest_bridge_shape_is_clean()` and
    `can_use_manifest_runtime_kv_export_plan()` before root treats a manifest as
    the export-planning source.
  - `TransformerForwardBatchSummary::from_summaries(...)` exposes forward
    summary count, attention-kind mix, compute/window bounds, activation shape,
    active layer count, and non-finite forward values before KV export block
    materialization.
    Forward batch helpers expose active-layer, non-local, fusion, compute,
    activation, and window-span signals separately from attention-count drift,
    active-count drift, invalid compute/activation/window shape, and non-finite
    forward values. Use `forward_batch_shape_is_clean()` for compact batch
    shape parity and `can_use_forward_batch()` before accepting a non-empty
    forward batch as the export baseline.
  - `RuntimeKvExportPlan::planned_block_count(...)` exposes the planned export
    count before block construction so root can parity-test export limits before
    payload comparison.
  - `RuntimeKvExportPlan::planning_summary(...)` compares forward-export
    planning with `RuntimePlanningDigest::planned_kv_exchange().export_blocks`
    so root can catch export-plan drift before materializing runtime KV blocks.
  - `RuntimeKvExportSummary` and `RuntimeKvExportPlanningSummary` expose
    forward-value presence, forward-summary parity, forward-summary count
    drift, non-finite forward summary signals, plan-limit hits, planned-block
    overflow, plan-vs-planning drift, planned-export overflow counts,
    payload/export-boundary signal and problem component counts, aggregate
    problem presence, accounting consistency, and clean export commit readiness
    before root expands per-block export differences. Use
    `export_payload_shape_is_clean()` and
    `can_use_runtime_kv_export_payload()` before materializing payload blocks;
    use `export_boundary_shape_is_clean()`,
    `runtime_kv_export_commit_signal_component_count()`,
    `runtime_kv_export_commit_blocker_component_count()`,
    `runtime_kv_export_commit_accounting_is_consistent()`,
    `runtime_kv_export_commit_is_clean()`, and
    `can_commit_runtime_kv_export()` before accepting the planned runtime KV
    export side effect.
  - `RuntimeKvExportBlockSummary::from_blocks(...)` and
    `from_block_summaries(...)` summarize materialized export blocks after
    `build_blocks(...)` but before root expands every payload. The summary
    counts planned vs materialized blocks, runtime-namespace blocks, aggregate
    block-shape signals, and runtime-exchange shape problems from
    `KvBlock::shape_summary()`. Use
    `runtime_kv_export_block_commit_signal_component_count()`,
    `runtime_kv_export_block_commit_blocker_component_count()`,
    `runtime_kv_export_block_commit_accounting_is_consistent()`,
    `runtime_kv_export_block_commit_is_clean()`, and
    `can_commit_runtime_kv_export_blocks()` before committing exported runtime
    KV blocks.
  - `RuntimeKvExportReadinessSummary` combines the forward batch, export
    payload, planning boundary, and materialized export block gates into one
    adapter-facing checklist. It preserves `RuntimeKvExportReadinessStage`
    ordering plus per-stage signal/blocker counts so root can expand the first
    failing forward/payload/planning/block report without copying the root
    forward implementation into core. No-op exports can be clean without
    materialized blocks; emitting exports must pass
    `can_use_forward_batch()`, `can_use_runtime_kv_export_payload()`,
    `export_boundary_shape_is_clean()`, and
    `can_commit_runtime_kv_export_blocks()`.
    Prefer `RuntimeKvExportPlan::readiness_summary(...)` when core should derive
    the export blocks from mapped forward summaries, and
    `RuntimeKvExportPlan::readiness_summary_for_blocks(...)` when root has
    already materialized/exported blocks and needs core to validate them against
    the same planning boundary before side effects. Use `commit_summary()` when
    root needs `RuntimeKvExportReadinessCommitAction`, mapped KV-export runtime
    failure reports, primary failure summary, failure batch, formatter readiness,
    and commit-decision accounting before applying exported KV side effects.
  - `TransformerPlanDigest::plan_summary()` and
    `RuntimeKvExportPlan::export_summary(...)` expose layer mix, compute/window
    bounds, forward batch shape, planned export count, limit hits, and
    empty-forward skips before root compares every layer or materialized KV
    block.
    Plan summary helpers classify layer-mix and pressure signals separately
    from count, fraction, compute, and window-shape problems so adapters can
    gate planner parity without parsing layer rows. Use
    `plan_summary_shape_is_clean()` for the compact shape gate and
    `can_use_transformer_plan()` before accepting a non-empty plan as a root
    planner baseline.
  - `TransformerPlanningPressureSummary::from_parts(...)` combines route budget,
    attention selection summary, and transformer plan summary so root can
    parity-test route pressure, selected attention pressure, non-local/fusion
    layer mix, and compute pressure in one adapter gate.
    Pressure summary helpers split route-token, attention-selection, and
    transformer-mix signals from invalid pressure fractions and route/attention
    to transformer delta drift. Use `planning_pressure_shape_is_clean()` and
    `can_use_planning_pressure()` before replacing root planner ownership.
  - `TransformerPlanningReadinessSummary` combines
    `RouteBudgetReadinessSummary`, `AttentionSelectionReadinessSummary`, and
    `TransformerPlanningPressureSummary` into one continuous
    router-to-attention-to-transformer checklist. It preserves
    `TransformerPlanningReadinessStage` order, per-stage signal/blocker counts,
    first unready/blocking stage helpers, pressure-vs-route and
    pressure-vs-attention parity helpers, aggregate accounting, and
    `can_commit_transformer_planning_readiness()` so root can catch stale
    pressure summaries before FHT-DKE or transformer planning consumes them.
    When the attention decision comes from `ThresholdAttentionPolicy`, preserve
    cap-hit and rejected-attention pressure through the same readiness boundary
    before FHT-DKE consumes route pressure. If the upstream candidate batch no
    longer matches the policy decision, FHT-DKE planning must stop at the
    transformer-planning stage even when the downstream budget summary is
    otherwise committable.
- `KvBlock`, `KvBlockShapeSummary`, `KvNamespace`, `KvNamespaceCounts`,
  `KvNamespaceCountDriftSummary`, `KvNamespaceCountDriftCommitSummary`,
  `KvNamespaceCountDriftCommitAction`, `KvCachePort`,
  `InMemoryKvCache`, `RuntimeKvCandidate`, `RuntimeKvImportPlan`,
  `RuntimeKvImportManifestPlanSummary`, `RuntimeKvImportSummary`,
  `RuntimeKvImportBlockSummary`, `RuntimeKvImportReadinessSummary`,
  `RuntimeKvImportReadinessCommitSummary`,
  `RuntimeKvImportReadinessCommitAction`, `RuntimeKvImportReadinessStage`,
  `RuntimeKvBlockContract`, `RuntimeKvBlockContractSummary`,
  `RuntimeKvBlockContractCheckSummary`, `RuntimeKvDirection`,
  `RuntimeKvValidationReport`, `RuntimeKvValidationSummary`,
  `RuntimeKvValidationBoundarySummary`
  - Runtime KV exchange data model and cache port. Namespace separation keeps
    runtime KV, semantic memory, gist memory, and agent KV from accidental fusion.
  - `RuntimeKvImportPlan` converts root memory/Infini candidates into
    `KvNamespace::Runtime` blocks using runtime metadata, transformer
    architecture, dimension fitting, and prefetch limits.
  - `RuntimeKvImportPlan::from_manifest(...)` builds the import plan from a
    `RuntimeManifestDigest`, preserving manifest KV import policy limits after
    runtime metadata normalization. `manifest_plan_summary(...)` exposes
    manifest import capability, runtime import capability, requested prefetch
    count, plan max blocks, embedding dimensions, and raw architecture
    layer/KV-head shape before root materializes imported KV blocks. Use
    `manifest_bridge_shape_is_clean()` and
    `can_use_manifest_runtime_kv_import_plan()` before root treats a manifest as
    the import-planning source.
  - `RuntimeKvImportPlan::import_summary(...)` exposes candidate counts,
    non-empty candidate counts, planned import blocks, import-limit hits, and
    embedding-dimension fitting before block materialization.
    Import summary helpers split enabled/candidate/non-empty/import/empty-skip,
    import-limit, and embedding-dimension signals from enabled-capacity,
    candidate-count, planned-block, limit-flag, disabled-import, and
    embedding-dimension shape problems. Use
    `runtime_kv_import_commit_signal_component_count()`,
    `runtime_kv_import_commit_blocker_component_count()`,
    `runtime_kv_import_commit_accounting_is_consistent()`,
    `runtime_kv_import_commit_is_clean()`, and
    `can_commit_runtime_kv_import()` as the compact adapter gates before root
    materializes runtime KV blocks.
  - `RuntimeKvImportBlockSummary::from_blocks(...)` and
    `from_block_summaries(...)` summarize materialized import blocks after
    `RuntimeKvImportPlan::build_blocks(...)` but before root expands every
    payload. The summary counts planned vs materialized blocks,
    runtime-namespace blocks, aggregate block-shape signals, and
    runtime-exchange shape problems from `KvBlock::shape_summary()`. Use
    `runtime_kv_import_block_commit_signal_component_count()`,
    `runtime_kv_import_block_commit_blocker_component_count()`,
    `runtime_kv_import_block_commit_accounting_is_consistent()`,
    `runtime_kv_import_block_commit_is_clean()`, and
    `can_commit_runtime_kv_import_blocks()` before committing imported runtime
    KV blocks.
  - `RuntimeKvImportReadinessSummary` combines the import-plan and
    materialized-block gates so root can check one adapter-facing readiness
    report after `RuntimeKvImportPlan::build_blocks(...)`.
    `RuntimeKvImportReadinessStage`, `first_unready_stage()`,
    `first_blocking_stage()`, per-stage signal/blocker counts,
    import-plan/materialized-block planned-count parity, and
    `can_commit_runtime_kv_import_readiness()` let root accept no-op imports
    cleanly while blocking malformed import plans, missing blocks, namespace or
    payload drift, and plan-vs-block count mismatches.
    Use `commit_summary()` when root needs
    `RuntimeKvImportReadinessCommitAction`, mapped KV-import runtime failure
    reports, primary failure summary, failure batch, formatter readiness, and
    commit-decision accounting before applying imported KV side effects.
  - `KvBlock::shape_summary()` exposes namespace, runtime-exchange status,
    layer/head/token range, vector lengths, empty-vector state, and finite-value
    state before validation or payload diffs.
  - `KvNamespaceCounts::from_blocks(...)` exposes runtime, semantic, gist,
    agent, and custom block counts before root compares concrete KV payloads or
    fused persistence rows.
  - `KvNamespaceCounts::drift_summary(...)` compares expected and actual
    namespace distributions with per-namespace count drift, total/non-runtime
    and active-namespace shape signals, aggregate distribution drift presence,
    accounting consistency, `namespace_distribution_shape_is_clean()`, and
    `can_use_namespace_distribution()` before root expands block-level payload
    diffs.
    Use `commit_summary()` when root needs `KvNamespaceCountDriftCommitAction`,
    mapped runtime failure reports, primary failure summary, failure batch,
    formatter readiness, and commit-decision accounting before imported,
    exported, or fused KV side effects mutate storage.
  - `KvNamespaceCounts` helpers distinguish runtime-only, non-runtime-only, and
    mixed runtime/non-runtime block sets, and expose runtime fraction for compact
    namespace-boundary assertions. Namespace boundary helpers split runtime
    exchange, non-runtime payload, namespace mix, and runtime/non-runtime mix
    signals from expected-vs-actual namespace distribution blockers.
  - `KvBlockShapeSummary` helpers expose empty token ranges, key/value length
    deltas, and paired finite vector state before root expands per-field payload
    errors. Runtime-exchange shape helpers split namespace/token/vector payload
    signals from runtime namespace drift, empty token range, key/value length
    drift, empty vectors, and non-finite values before request/response KV
    validation expands verbose payload errors.
  - `RuntimeKvBlockContract` validates runtime KV import/export blocks against
    namespace, layer/head bounds, token bounds, vector dimensions, and finite
    key/value payloads without depending on root runtime error types.
  - `RuntimeKvBlockContract::for_request_imports(...)` and
    `for_request_exports(...)` derive max block count and token bounds from the
    originating `RuntimeRequestEnvelope`; block counts above the contract are
    violations, not silent truncation.
  - `RuntimeKvBlockContract::contract_summary()` exposes max-block capacity,
    token upper bound, direction, and direction label before lower-level payload
    validation. Use `runtime_kv_block_contract_commit_signal_component_count()`,
    `runtime_kv_block_contract_commit_blocker_component_count()`,
    `runtime_kv_block_contract_commit_accounting_is_consistent()`,
    `runtime_kv_block_contract_commit_is_clean()`, and
    `can_commit_runtime_kv_block_contract()` before adapters call
    `validate_blocks(...)`; disabled zero-capacity export contracts can remain
    clean but not commit-ready. Contract signals expose capacity, token-bound,
    and direction observations while blockers capture malformed public contract
    shape.
  - `RuntimeKvBlockContract::block_check_summary(...)` exposes the same
    namespace, layer/head, token-bound, vector-length, and finite-value checks as
    `validate_block(...)` as booleans and focused problem counts. Use
    `namespace_problem_component_count()`,
    `layer_head_problem_component_count()`, `token_problem_component_count()`,
    and `vector_problem_component_count()` for focused diagnostics. Use
    `runtime_kv_block_contract_check_commit_signal_component_count()`,
    `runtime_kv_block_contract_check_commit_blocker_component_count()`,
    `runtime_kv_block_contract_check_commit_accounting_is_consistent()`,
    `runtime_kv_block_contract_check_commit_is_clean()`, and
    `can_commit_runtime_kv_block_contract_check()` when an adapter needs one
    stable block-level commit gate before expanding verbose payload validation
    strings. Contract-check signals are observational; blockers decide whether a
    single imported/exported `KvBlock` can be accepted.
  - `RuntimeKvValidationReport::validation_summary()` exposes accepted block
    count, violation count, and valid state before root maps verbose payload
    failures into runtime errors. Validation summary helpers split accepted,
    partial-acceptance, and rejected-all signals from violation and valid-flag
    drift blockers, with `runtime_kv_validation_commit_signal_component_count()`,
    `runtime_kv_validation_commit_blocker_component_count()`,
    `runtime_kv_validation_commit_accounting_is_consistent()`, and
    `runtime_kv_validation_commit_is_clean()` as the compact import/export
    payload commit gate.
  - `RuntimeKvBlockContract::validation_boundary_summary(...)` combines the
    import/export direction, failure trace label, limits, accepted count,
    violation count, and valid state into one adapter-facing KV boundary report.
    Use `maps_to_runtime_kv_failure()`,
    `runtime_kv_boundary_commit_signal_component_count()`,
    `runtime_kv_boundary_commit_blocker_component_count()`,
    `runtime_kv_boundary_commit_accounting_is_consistent()`,
    `runtime_kv_boundary_commit_is_clean()`, and
    `can_commit_runtime_kv_boundary()` before root formats import/export payload
    validation failures.
- `QuantizedVector`, `QuantizedKvBlock`, `QuantizedKvPayloadSummary`,
  `KvQuantizationPlan`, `QuantizationError`
  - Adapter-facing KV quantization contract for root `src/kv_quant/**`.
  - Runtime KV uses hot precision, while semantic/gist/agent/custom KV uses cold
    precision by default through `KvQuantizationPlan`.
  - `QuantizedKvBlock::vector_value_len()`, `packed_payload_len()`, and
    `compression_ratio()` give root codec parity tests a stable payload-shape
    summary before comparing encoded strings or decoded values.
  - `QuantizedKvBlock::payload_summary()` also exposes namespace, runtime hot
    KV bit selection, non-runtime cold bit selection, vector lengths, packed
    lengths, and empty-payload state as one adapter-facing report.
  - Payload summary helpers expose key/value length parity, packed-length
    parity, symmetric key/value shape, compressed payload state, and
    namespace-specific hot/cold bit parity before root expands byte-level codec
    diffs.
  - Quantized payload helpers split runtime-namespace, payload-presence, and
    compression signals from key/value length drift, packed-length drift,
    asymmetric key/value or packed shape, invalid compression ratios,
    empty-payload shape drift, and namespace bit-selection drift. Root should
    treat signal counts as codec activity. Use
    `quantized_payload_commit_signal_component_count()`,
    `quantized_payload_commit_blocker_component_count(...)`,
    `quantized_payload_commit_accounting_is_consistent(...)`,
    `quantized_payload_commit_is_clean(...)`, or
    `can_commit_quantized_payload(...)` before replacing persisted KV payloads.
- `MemoryTier`, `TieredMemoryCandidate`, `TieredMemoryCandidateSummary`,
  `TieredMemoryScheduler`, `TieredCachePlan`, `TieredCacheSummary`,
  `MemoryPlacement`, `TierMigration`, `TierMigrationSummary`
  - Adapter-facing tiered cache contract for hot GPU, warm RAM, and cold disk
    placement.
  - Root `src/tiered_cache/**` can map `MemoryEntry` and `MemoryMatch` into
    candidates while keeping persistence and device-specific eviction outside
    core.
  - `TieredMemoryCandidate::candidate_summary()` exposes strength,
    reliability, attempts, failures, last score, and active similarity before
    root compares scheduler placement rows.
    Candidate helpers split strength, feedback, failure, active-similarity, and
    failure-heavy signals from invalid strength, reliability, failure-count,
    score, and active-similarity shape before scheduler parity expands
    placement rows. Use `candidate_shape_is_clean()` and
    `can_use_tiered_memory_candidate()` before feeding a candidate into tiered
    placement.
  - `TieredCachePlan::summary()` exposes placement count, tier counts, hot,
    warm, and cold fractions, multi-tier distribution state, score range, and
    average score before root compares individual placement rows.
  - Cache summary helpers split multi-tier, score-spread, and cold-pressure
    distribution signals from placement-count drift, non-finite score fields,
    inverted score bounds, empty-score shape drift, and average-score drift.
    Use `cache_summary_is_clean()` for compact shape parity and
    `can_use_tiered_cache_summary()` before root accepts a non-empty tier plan.
    Use `cache_placement_signal_component_count()`,
    `cache_placement_blocker_component_count()`,
    `cache_placement_accounting_is_consistent()`,
    `tiered_cache_placement_commit_signal_component_count()`,
    `tiered_cache_placement_commit_blocker_component_count()`,
    `tiered_cache_placement_commit_accounting_is_consistent()`, and
    `tiered_cache_placement_commit_is_clean()` to separate observable
    placement activity from malformed placement summaries before root mutates
    cache storage. `can_commit_tiered_cache_placement()` stays false for empty
    clean baselines.
  - `TieredCachePlan::migration_summary_from(...)` gives root promote/demote,
    retain, new, evict, and observed migration-row counts before applying tier
    movement.
  - Tiered summary helpers expose placement-count parity, all-hot/warm/cold
    states, score spread, movement/capacity pressure, and migration
    changed/retained total parity before root expands placement rows.
  - Migration helpers split new-entry, tier-movement, and capacity-pressure
    signals from action-count/retention-balance drift, so root can observe
    valid cache movement without blocking persistence mutation unless the
    migration summary accounting is malformed. Use
    `tier_migration_commit_signal_component_count()`,
    `tier_migration_commit_blocker_component_count()`,
    `tier_migration_commit_accounting_is_consistent()`,
    `tier_migration_commit_is_clean()`, or `can_commit_tier_migration()` before
    root applies tier movement.
- `MemoryRecord`, `MemoryRecordSummary`, `MemoryRetentionPolicy`,
  `MemoryCompactionPolicy`,
  `RetentionReport`, `MemoryCompactionReport`, `MemoryGovernancePolicy`,
  `MemoryGovernanceReport`, `MemoryGovernanceSummary`, `MemoryUpdateSummary`,
  `MemoryUpdateBatchSummary`
  - Adapter-facing memory governance contract for root `src/kv_cache/**`.
  - Provides retention preview and compaction planning without mutating root
    cache storage or persistence.
  - `MemoryRecord::summary()` exposes namespace, vector length, strength,
    reliability, attempts, failure state, finite-value state, and age span
    before root compares full memory payloads.
  - `MemoryGovernanceReport::governance_summary()` exposes retention
    before/after counts, decayed/removed counts, compaction merge/removal
    counts, total removed ids, note count, noop state, and final record count
    before root mutates cache storage or disk state.
  - `MemoryUpdateReport::update_summary()` and `batch_summary(...)` expose
    applied/missing state, reinforce/penalize counts, removed records, requested
    amount totals, and net strength delta before root compares cache mutation
    side effects.
  - Memory summary helpers expose feedback/finite/failure-heavy record state,
    reinforce/penalize update state, applied-removal state, requested amount and
    strength-delta shape, update-count parity, mixed-action batches, net
    strength direction, update batch signal/problem counts, update batch commit
    readiness, memory update commit signal/blocker counts, single-update and
    batch-update commit accounting, retention/compaction count balance, pipeline
    balance, note parity, clean-noop governance state, governance
    signal/problem counts, and a compact governance commit gate. Memory update
    commit helpers expose applied/missing/reinforce/penalize/change/removal
    signals separately from shape blockers, and batch commit helpers expose the
    same adapter-facing split for aggregate feedback. Governance commit helpers
    expose note signals, aggregate commit signal/blocker counts, commit
    accounting, and mutation readiness without parsing report notes. After
    `can_commit_runtime_boundary()` passes, use `update_shape_is_clean()`,
    `memory_update_commit_is_clean()`, and `can_commit_memory_update()` for
    single updates, `update_batch_commit_is_clean()`,
    `memory_update_batch_commit_is_clean()`, or
    `can_commit_memory_update_batch()` for batch feedback, and
    `governance_commit_is_clean()` or `can_commit_memory_governance()` before
    root mutates memory storage.
- `KvFusionPolicy`, `ReinforcedKvFusionPolicy`, `KvFusionMerge`,
  `KvFusionMergeSummary`, `KvFusionCommitSummary`, `KvFusionCommitAction`,
  `KvFusionPair`
  - Reinforced KV-Fusion contract. Current implementation deduplicates matching
    namespace/layer/head/token-range blocks when key/value similarity passes the
    policy threshold.
  - `KvFusionMerge::merged_count()`, `merge_fraction()`, `changed()`, and
    `skipped_due_to_limit()` give root persistence adapters stable scalar
    checks before comparing fused block payloads.
  - `KvFusionMerge::merge_summary()` exposes collapsed-block state, skip-limit
    state, runtime/non-runtime result counts, namespace-mix counts, and grouped
    namespace counts as one adapter-facing report.
  - Fusion summary helpers expose merge/skip change causes, runtime/non-runtime
    result-count parity, namespace-count parity, focused accounting drift
    component counts, merge-fraction shape, namespace-mix/runtime-mix signal
    counts, aggregate drift presence, clean accounting, and
    all-runtime/all-non-runtime result shapes before root compares fused block
    payloads.
    Commit helpers add result namespace-boundary signals, aggregate commit
    signal/blocker counts, commit accounting, `fusion_commit_shape_is_clean()`,
    and `can_commit_kv_fusion_persistence()` so root can gate fused persistence
    mutation without parsing block rows. Prefer `commit_summary()` when root
    needs `KvFusionCommitAction`, mapped runtime failure reports, primary
    failure summary, failure batch, formatter readiness, empty-persistence
    problem detection, and commit-decision accounting before applying fused KV
    side effects.
    Boundary helpers split merge/skip/namespace-mix signals from accounting
    problems through `fusion_boundary_signal_component_count()`,
    `fusion_boundary_problem_component_count()`, and
    `fusion_boundary_is_consistent()` so root adapters can observe legal fusion
    activity without treating it as a persistence blocker. Use
    `fusion_boundary_shape_is_clean()` and `can_use_kv_fusion_merge()` before
    accepting a non-empty fused KV merge as the persistence baseline.
- `ExperimentSwitches`, `ExperimentSwitchesSummary`
  - Conservative defaults keep FHT-DKE, adaptive attention thresholds, reinforced
    KV fusion, and runtime device ABI disabled until callers opt in.
  - `switches_summary()` exposes enabled feature count, per-feature state,
    runtime-planning feature presence, attention/KV feature presence, and
    conservative attention/KV budgets before root expands UI labels or summary
    strings.
  - `enabled_labels()` and `summary()` remain display helpers for root CLI/Web
    Lab/agent diagnostics.
- `FhtDkeBudgeter`, `DeterministicFhtDkeBudgeter`, `FhtDkeInput`,
  `FhtDkeBudget`, `FhtDkeBudgetSummary`,
  `FhtDkePlanningReadinessStage`, `FhtDkePlanningReadinessSummary`,
  `FhtDkePlanningCommitAction`, `FhtDkePlanningCommitSummary`
  - Deterministic budget port for the future FHT-DKE kernel path. Disabled mode
    keeps all requested tokens dense; enabled mode splits dense/routed tokens and
    estimates KV import/export blocks from runtime metadata.
  - `FhtDkeBudget::route_pressure` exposes how much route-level attention demand
    affected the dense/routed split and routed KV exchange demand.
  - `budget_summary()` plus `dense_fraction()`, `routed_fraction()`,
    `has_route_pressure()`, `route_pressure_is_high()`, `has_routed_work()`,
    `routed_tokens_per_kv_exchange_block()`, `token_split_is_valid()`, and
    `has_kv_exchange()` give root parity tests a stable budget summary before
    comparing concrete KV import/export counts.
  - Budget summary helpers split token-split/KV-exchange shape problems from
    route-pressure, routed-work, KV-exchange, and KV-asymmetry signals so root
    can gate invalid budget accounting without treating legal pressure as a
    failure. Use `budget_shape_is_clean()` and `can_use_fht_dke_budget()` before
    passing a non-empty FHT-DKE budget into runtime planning.
  - Commit helpers expose pressure classification as
    `fht_dke_budget_commit_signal_component_count()` while empty budgets and
    budget-shape drift flow through
    `fht_dke_budget_commit_blocker_component_count()`. Use
    `fht_dke_budget_commit_accounting_is_consistent()` and
    `can_commit_fht_dke_budget()` at the adapter boundary before root writes
    the budget into request planning.
  - `FhtDkePlanningReadinessSummary` links
    `TransformerPlanningReadinessSummary` to the FHT-DKE budget commit gate.
    It checks that route pressure and attention threshold facts used by
    transformer planning match the downstream budget summary, reports the
    transformer-planning, budget-commit, and pressure-budget-boundary stages,
    and catches stale pressure/threshold drift before runtime planning or KV
    budget mutation consumes the budget.
    `commit_summary()` packages the same gate into
    `FhtDkePlanningCommitSummary`: `CommitFhtDkePlanning` exposes the committed
    budget summary, `WaitForFhtDkePlanning` preserves clean unready planning
    state, and `RepairFhtDkePlanning` blocks stale pressure, threshold drift, or
    malformed public accounting before runtime planning consumes the budget.

## Migration Route From Existing `src`

1. Move vocabulary first.
   - Source references: `src/hierarchy.rs`, `src/hierarchy/**`,
     `src/router.rs`, `src/router/**`.
   - Target: keep `TaskProfile`, `HierarchyWeights`, `RouteBudget`,
     `GenerationMetrics`, and routing feedback semantics in `norion-core`; root
     crate adapts old types to these.
   - Target: map root `RouterState`, `ProfileThresholds`, and
     `ProfileObservations` into core snapshots before replacing the root router
     implementation.
   - Target: use `profile_observation_total()` and
     `has_observation_drift()` as the first router-state parity checks.
2. Move runtime request contracts next.
   - Source references: `src/engine.rs`, `src/engine/**`,
     `src/runtime.rs`, `src/runtime/**`, `src/inference_runner.rs`.
   - Target: root runtime/backend implements `InferenceEngine` and maps service
     requests into `InferenceRequest`, preserving `prompt_tokens` and
     `max_tokens`.
   - Use `RuntimeGenerationBudget` before finalizing concrete runtime
     `max_tokens`.
   - Build `RuntimePlanningDigest` after route/hardware planning and before
     `RuntimeRequestEnvelope` so adapter selection, context truncation,
     FHT-DKE route pressure, and KV prefetch clamps are parity-tested together.
   - Run `RuntimePlanningDigest::acceptance_report()` before backend request
     construction so context exhaustion and malformed planning state use core
     failure labels instead of root-local string matching.
   - Map root `RuntimeDiagnostics`, `EmbeddingDiagnostics`, and runtime error
     notes into `InferenceDiagnostics`; leave reflection repair/report types in
     root until the main window chooses to migrate them.
   - Map root `RuntimeTokenMetrics` into `GeneratedTokenMetrics` from the same
     token stream before routing the metrics into replay or feedback adapters.
   - Map root `RuntimeRequest` into `RuntimeRequestEnvelope` before JSON wire
     formatting and attach `RuntimePlanningDigest` so schema, generation budget,
     architecture, planned adapter, backend max-token cap, and KV limits can be
     checked in core.
   - Map root `RecursiveSchedule` into `RecursiveScheduleDigest` or
     `RecursiveScheduleSummary`, then attach it to `RuntimeRequestEnvelope`.
   - Map root `RuntimeResponse` into `RuntimeResponseEnvelope` after JSON parse
     and before reflection/memory feedback so answer, token, KV, and diagnostics
     consistency can be checked in core.
   - Re-check the parsed response against the originating
     `RuntimeRequestEnvelope` so runtime output cannot exceed request/planning
     token, adapter, generation-budget, route, hardware, or KV exchange limits.
   - Use `InferenceDiagnostics::from_request_envelope(...)` or
     `with_request_envelope(...)` before response backchecks so route,
     generation-budget, hardware pressure, compute headroom, and latency fields
     are populated consistently.
   - Use `RuntimeDiagnostics::from_request_envelope(...)` or
     `with_request_envelope(...)` when runtime diagnostics omit model,
     selected-adapter, architecture, import-count, or KV precision fields.
   - Map root runtime backend failures into `RuntimeFailureReport` so KV import,
     runtime generation, KV export, contract, and context errors use one core
     vocabulary before becoming drafts, notes, or user-facing runtime errors.
   - Convert request/response acceptance report failures with
     `failure_reports()` or `primary_failure_report()` so root keeps core trace
     labels, diagnostics notes, confidence, and recoverability semantics.
3. Move KV exchange behind ports.
   - Source references: `src/kv_cache.rs`, `src/kv_cache/**`,
     `src/tiered_cache.rs`, `src/kv_quant.rs`, `src/runtime/kv_import.rs`.
   - Target: memory/runtime layers exchange `KvBlock` through `KvCachePort`;
     quantization and persistence stay outside core.
   - Target: root `runtime_kv_blocks_from_context(...)` maps memory or Infini
     candidates into `RuntimeKvCandidate`, applies root-owned tier filtering,
     then lets `RuntimeKvImportPlan` build runtime namespace blocks.
   - Target: root production/local runtimes call `RuntimeKvBlockContract` before
     accepting imported or exported runtime KV blocks; root remains responsible
     for turning validation reports into runtime errors.
   - Target: root validates imported blocks via
     `RuntimeAcceptanceContext::from_request_parts(...)` and
     `request_acceptance_report()` before request JSON formatting, then
     validates response diagnostics/request parity/exported KV via
     `response_acceptance_report(...)` after response parsing and before
     memory/reflection mutation. The lower-level envelope helpers remain
     available for adapter tests that need only one side of the boundary.
   - Target: root `src/kv_quant/**` maps to `QuantizedVector` and
     `QuantizedKvBlock`; persistence encoding may stay root-owned until
     migration tests compare both codecs.
   - Target: root compares quantized payload lengths and compression ratios
     before replacing root decode/encode paths.
   - Target: tiered memory placement uses `TieredMemoryCandidate` and
     `TieredCachePlan`; root cache storage stays outside core.
   - Target: root compares `TierMigrationSummary` before applying tier movement
     or eviction side effects.
   - Target: root retention/compaction policies map into
     `MemoryGovernancePolicy`; root applies `RetentionReport` and
     `MemoryCompactionReport` to its cache after adapter comparison tests pass.
   - Target: root compares `MemoryGovernanceReport::removed_ids()`,
     `total_removed()`, `is_noop()`, and
     `MemoryGovernanceSummary::governance_commit_is_clean()` before applying
     any cache mutation.
4. Move attention and fusion policy selection.
   - Source references: `src/transformer.rs`, `src/transformer/**`,
     `src/router/**`, `src/kv_cache/**`.
   - Target: transformer/runtime planning consumes `AttentionDecision`,
     `RouteBudget`, and `KvFusionMerge` instead of depending on service-local
     heuristics.
   - Target: root compares attention selected/rejected counts and fusion
     merged/skipped summaries before moving policy ownership.
   - Target: root `TransformerPlanner` maps to `TransformerPlanningInput` and
     compares output with `DefaultTransformerPlanner` before moving decision
     ownership.
   - Target: root `local_runtime::forward::kv_export` maps layer summaries into
     `TransformerForwardSummary` and compares planned export count plus exported
     runtime KV blocks with `RuntimeKvExportPlan`.
5. Add hardware adapters.
   - Source references: `src/hardware/**`.
   - Target: root `HardwareSnapshot` maps into `HardwareLoadSnapshot`, root
     `HardwarePlan` maps into `HardwarePlan`, and runtime code consumes
     `HardwarePlan::adapter_execution_context()`.
   - Target: runtime response diagnostics call
     `RuntimeDiagnostics::hardware_contract_violations(...)` after root maps
     the hardware plan into core.
   - Target: runtime response diagnostics can instead call
     `RuntimeDiagnostics::hardware_acceptance_report(...)` when root needs
     `RuntimeFailureReport` mapping before accepting device execution claims.
   - Hardware probing, environment detection, and production device gate reports
     remain in root.
6. Add runtime manifest adapters.
   - Source references: `src/runtime_manifest/**`, `src/kv_quant/**`,
     `src/hardware/**`.
   - Target: root `RuntimeManifest` maps into `RuntimeManifestDigest`; root
     asset paths and production file validation remain in root.
   - Use `RuntimeManifestDigest::runtime_metadata()` as the single source for
     runtime metadata passed to core budgeters.
   - Target: root compares `RuntimeManifestDigest::abi_summary()` before
     request planning so context, architecture, KV limits, quantization, and
     adapter-count drift are visible without string parsing.
   - Target: root runs `RuntimeManifestDigest::validate()` and maps validation
     errors with `RuntimeManifestValidation::failure_reports()` before runtime
     request planning.
7. Add root adapters.
   - Source references: `src/runtime/**`, `src/hardware/**`,
     `src/router/**`, `src/hierarchy/**`.
   - Target: root conversion helpers map existing types into
     `RuntimeAdapter`, `AdapterExecutionContext`, `RuntimeMetadata`,
     `RoutingContext`, `RouteBudget`, and `TransformerPlanDigest`.
  - Target: root runtime adapter observations map into `AdapterObservation`;
    root can call `AdapterExecutionContext::select_adapter(...)` after hardware
    filtering without moving experience matching into core.
  - Target: root can call `select_adapter_report(...)` during parity tests to
    compare allowed adapter count, matched observation count, rejected
    observations, and fallback reason before using only the selected adapter.
   - See `docs/architecture/norion-core-adapters.md`.
8. Replace deterministic FHT-DKE with the real kernel.
   - `DeterministicFhtDkeBudgeter` is only a small verified placeholder. The
     real FHT-DKE implementation can implement `FhtDkeBudgeter` without changing
     memory/agent/service call sites.

## Upper-Layer Ports

- memory
  - Implements or wraps `KvCachePort`.
  - Exports candidate `KvBlock`s to runtime and receives exported blocks after
    generation.
  - May run `KvFusionPolicy::fuse` before persistence or compaction.
- agent
  - Chooses `TaskProfile` and `ExperimentSwitches`.
  - Supplies feedback through `HierarchicalRouter::observe` and
    `AttentionPolicy::observe` after validation/reward scoring.
- service / CLI / Web Lab
  - Parses user request fields into `InferenceRequest`.
  - Keeps `max_tokens` on the request all the way to the concrete backend.
  - Reads `InferenceOutcome` for generated text, route budget, imported KV, and
    exported KV diagnostics.
- runtime backend
  - Implements `InferenceEngine`.
  - Uses `RuntimeMetadata` to advertise model limits and KV import/export
    capability.
  - Uses `RuntimeGenerationBudget` to clamp requested generation to known
    context limits before calling the concrete backend.
  - Can opt into `FhtDkeBudgeter`, `AttentionPolicy`, and `KvFusionPolicy` via
    `ExperimentSwitches`.

## Current Test Points

Run independently:

```powershell
cargo test --manifest-path crates\norion-core\Cargo.toml
```

Evidence index for recently locked R2 kernel boundaries:

- Context exhaustion is covered at runtime, planning, and request-envelope
  edges by
  `runtime_generation_budget_reports_exhausted_context_for_zero_requested_tokens`,
  `planning_acceptance_returns_context_exhausted_for_zero_requested_tokens_at_full_context`,
  `runtime_request_planning_readiness_blocks_context_exhausted_runtime_planning`,
  and
  `request_envelope_blocks_commit_when_zero_requested_tokens_exhaust_context`.
  The verified behavior is that zero requested max tokens normalize to one
  requested token, but a full known context window yields no backend max-token
  commit and blocks the concrete request path.
- Runtime metadata readiness is covered by
  `runtime_metadata_readiness_degrades_without_committing_missing_embeddings`,
  `request_envelope_blocks_missing_runtime_metadata_adapter_after_generation_degrade`,
  `request_envelope_blocks_unknown_context_runtime_metadata_after_budget_normalizes`,
  and
  `request_envelope_preserves_router_budget_but_blocks_missing_runtime_metadata_adapter`.
  The verified behavior covers both a known context window with missing
  embedding dimensions and an unknown context window with known embedding
  dimensions: generation budgeting can either degrade to a clamped backend
  token count or normalize without context pressure, and a real router-derived
  route budget can remain request/planning-parity clean, but runtime metadata
  adapter publication remains blocked and the request-envelope gate will not
  commit the backend wire request.
- Route-budget and hierarchy propagation are covered by
  `hierarchy_bias_can_promote_borderline_tokens_into_route_budget`,
  `route_budget_threshold_includes_equal_score_as_attention_pressure`,
  `hardware_pressure_discount_demotes_borderline_route_budget_and_blocks_stale_attention_budget`,
  `planning_digest_preserves_router_generated_high_route_pressure`,
  `runtime_planning_readiness_commits_router_threshold_through_fht_dke_boundary`,
  `runtime_planning_readiness_blocks_stale_low_pressure_route_budget_after_hardware_demote`,
  `inference_diagnostics_request_parity_blocks_stale_hierarchical_route_budget`,
  `inference_diagnostics_request_parity_blocks_stale_low_pressure_route_after_hardware_demote`,
  `response_request_parity_blocks_stale_router_budget_from_runtime_diagnostics`,
  `response_request_parity_blocks_stale_low_pressure_route_after_hardware_demote`,
  plus
  `request_manifest_planning_readiness_preserves_router_budget_kv_degrade`.
  The verified behavior is that real `DefaultHierarchicalRouter` decisions can
  turn a borderline token into an attention route, equal-threshold route scores
  count as attention pressure in route-budget summaries, hardware pressure can
  demote a borderline attention route into the fast path while stale
  attention-heavy budgets are rejected at route-budget parity, the concrete
  route threshold/budget can commit through route readiness, that pressure is
  preserved in FHT-DKE and runtime planning boundaries, stale low-pressure
  FHT-DKE planning facts are rejected when a high-pressure runtime digest has
  demoted the same token into a fast route, stale diagnostics route
  budgets are rejected against the saved request envelope both before and after
  response-envelope construction, diagnostics and response request parity both
  reject stale low-pressure route and hardware-pressure facts after a
  high-pressure request demotes the route to the fast path, KV prefetch
  degrades through the planned budget, and request/manifest gates still commit
  when imported KV facts and manifest limits match.
- Runtime planning KV budget boundaries are covered by
  `planning_digest_reports_runtime_and_fht_dke_kv_prefetch_limits` and
  `request_manifest_planning_readiness_commits_runtime_and_fht_dke_kv_prefetch_limits`,
  plus
  `request_envelope_blocks_fusion_skip_pressure_as_imported_kv_commit`.
  The verified behavior is that requested KV prefetch is first clamped by
  runtime metadata and then by the FHT-DKE import budget, with
  `RuntimeAndFhtDkeLimits` preserving both reductions before backend request
  commit; the request envelope and manifest bridge can commit only when they
  carry the final planned import/export counts rather than the unclamped
  execution prefetch request. KV fusion skip pressure remains a fusion
  persistence signal; it must not be added to request imported-KV counts.
- KV fusion persistence boundaries are covered by
  `reinforced_fusion_deduplicates_same_runtime_slot`,
  `fusion_keeps_namespace_boundaries_even_for_identical_vectors`,
  `fusion_report_marks_candidate_limit_skips`,
  `fusion_candidate_budget_from_experiment_switches_is_visible_but_committable`,
  `fusion_candidate_budget_full_scan_commits_without_skip_pressure`,
  `fusion_public_zero_candidate_budget_clamps_to_one_before_persistence`,
  `fusion_candidate_budget_skip_does_not_pollute_result_namespaces`,
  `fusion_zero_candidate_budget_skips_all_incoming_without_namespace_pollution`,
  `fusion_merge_summary_counts_accounting_drift_components`,
  `fusion_merge_summary_blocks_namespace_count_drift_before_persistence`,
  `fusion_merge_summary_blocks_public_fraction_drift`, and
  `empty_fusion_summary_is_clean_but_not_usable`. The verified behavior is
  limited to merge/noop/skip accounting, experiment-switch candidate budget
  visibility, full-scan candidate budgets clearing skip pressure before clean
  persistence commit, public zero-candidate budgets clamping to one candidate
  before persistence, candidate-budget skips staying out of result namespace counts,
  explicit internal zero-candidate-budget skips leaving only existing result namespaces,
  namespace isolation, grouped namespace-count drift detection, public summary
  drift detection, and empty-persistence failure-return gating before a root
  adapter applies fused KV side effects.
- Runtime diagnostics hardware acceptance is covered by
  `runtime_diagnostics_hardware_contract_accepts_control_plane_filled_device_execution`
  and
  `runtime_diagnostics_control_plane_filled_execution_still_blocks_hardware_drift`.
  The verified behavior is limited to matching device/lane/memory-mode aliases:
  control-plane-filled device execution remains distinguishable from
  runtime-reported execution while still passing the hardware diagnostics gate,
  and the same source label does not mask device, lane, or memory-mode drift
  when the reported values disagree with the hardware plan.
- Context-level device execution commit summaries are covered by
  `acceptance_context_submits_only_runtime_reported_device_execution_envelope`.
  The verified adapter boundary is intentionally read-only:
  `RuntimeAcceptanceContext::runtime_boundary_device_execution_commit_summary(...)`
  derives its action and accounting from runtime-reported device execution
  readiness, distinguishes wait-for-runtime-metadata from repair-envelope
  blockers, and does not perform the real device execution commit.
- Adaptive-attention to FHT-DKE planning is covered by
  `runtime_planning_readiness_commits_threshold_attention_decision_boundary`,
  `fht_dke_planning_readiness_blocks_stale_adaptive_attention_selection_boundary`,
  and
  `runtime_planning_readiness_blocks_stale_adaptive_attention_boundary`.
  The verified behavior is limited to real `ThresholdAttentionPolicy`
  decisions: a matching candidate-batch summary can carry selected/rejected
  attention pressure through transformer planning, FHT-DKE planning, and
  runtime planning readiness; a stale candidate-batch summary marks transformer
  planning repairable at the attention-selection boundary, keeps FHT-DKE from
  exposing a committed budget, and makes runtime planning inherit that blocker
  even when runtime pre-request checks and the FHT-DKE/runtime budget boundary
  still match.

Covered behavior:

- profile-aware routing layer selection and route budget counting
- route decision summaries for layer counts, threshold crossings, score ranges,
  inclusive equal-threshold crossings, empty decision sets, and route-budget
  parity
- route-budget readiness commit summaries for commit, clean wait, and repair on
  parity drift before downstream pressure planning
- generation metrics quality scoring and conversion into routing feedback
- router state snapshot restore/clamping, profile observation counts, and
  observation drift detection
- generated token metrics for entropy, logprob, and uncertainty summaries
- inference outcome summaries for answer/token/KV/route/diagnostics parity at
  the runtime response boundary
- adaptive attention threshold filtering and max-token cap, including
  `adaptive_threshold_profile_state_requires_enabled_switch_for_selection`
  coverage that learned profile thresholds affect selection only when the
  adaptive-threshold switch is enabled, and
  `threshold_policy_selects_equal_threshold_attention_without_fast_layer_leak`
  coverage that equal-threshold attention-layer candidates are selected while
  equal-threshold fast-path candidates remain rejected
- attention decision summaries for selected/rejected layer counts, cap hits,
  selection fraction, and attention-token totals
- real attention-policy cap pressure preserved through transformer planning
  readiness via `transformer_planning_readiness_preserves_real_attention_cap_pressure`
- KV cache namespace export and same-slot replacement
- runtime KV import planning with metadata limits, dimension fitting, and
  namespace isolation
- runtime KV import summaries, namespace count summaries, namespace boundary
  signal/problem gates, and block shape signal/problem gates before payload
  validation
- KV persistence failure-return selection for namespace-before-fusion ordering,
  clean continuation, namespace-first failure materialization, clean-namespace
  fusion failure materialization, and non-canonical source-order repair
- runtime KV import/export contract validation for layer/head, token range,
  dimension, namespace, and finite-value boundaries
- runtime KV validation summaries for accepted block count, violation count,
  valid state, validation signal/problem counts, and clean commit readiness
  before root error mapping
- request-derived runtime KV import/export contracts, including strict
  over-limit block count violations
- planning/request/response acceptance reports that combine envelope contract checks
  with request-derived KV payload validation
- request/response acceptance summaries for contract, parity, and KV failure
  counts
- acceptance report to `RuntimeFailureReport` mapping for request contracts, KV
  import, response/request parity, KV export boundaries, planning contract
  failures, and context exhaustion
- evidence index: context-exhaustion is covered through acceptance failure
  mapping; route-budget drift is covered by diagnostics/request parity and
  response backchecks; KV fusion boundaries are covered by namespace isolation,
  namespace-count/result parity, and persistence failure-return selection; and
  response diagnostics preserve device-execution source while blocking
  adapter/KV-precision drift via
  `response_boundary_blocks_adapter_and_precision_drift_with_device_signal`
- runtime acceptance context reuse of saved request, hardware, and imported KV
  facts across request and response gates
- runtime acceptance context summary helpers for request and response gates
- runtime boundary acceptance summaries combining saved-context request and
  response gate counts, including focused request/response/KV/request-parity
  failure components, aggregate boundary acceptance problem counts, and
  request+response acceptance commit readiness
- runtime boundary envelope summaries combining saved-context request/response
  token, KV, diagnostics, adapter, and context-pressure facts with focused
  shape-drift and aggregate envelope signal component counts
- context construction that clamps `HardwarePlan::adapter_execution_context()`
  against runtime metadata before building the request envelope
- runtime acceptance context preserves concrete imported KV counts after
  metadata-driven KV prefetch clamp, and request gate blocks commit when actual
  imports exceed the runtime/planning limit
- request envelope planning parity blocks stale imported-KV and KV-prefetch
  counts after a clean runtime-metadata KV clamp via
  `request_envelope_blocks_stale_kv_import_after_runtime_metadata_clamp`
- manifest KV bridge blocks commit after a clean request/runtime-planning path
  when the runtime manifest disables planned KV export, via
  `acceptance_context_manifest_bridge_blocks_export_disabled_after_clean_request`
- KV quantization vector encoding/decoding and namespace-aware hot/cold KV
  block quantization
- quantized KV payload summaries for namespace-aware hot/cold bit selection and
  packed payload shape, including length parity, symmetric key/value shape,
  compression state, payload signal counts, and commit blockers
- memory retention preview, namespace-scoped compaction planning, protected
  memory handling, governance ordering, and removed-id/noop report summaries
- tiered memory placement, cache summary distribution/score gates, migration
  classification, migration summaries, migration-row/action accounting gates,
  and tier parsing
- memory record summaries, memory update/governance count parity, and tiered
  cache summaries before mutation-level diffs
- reinforced KV fusion dedupe/merge with namespace isolation
- reinforced KV fusion candidate limits from experiment switches remain visible
  as skip signals without becoming persistence blockers when accounting is clean,
  and skipped candidates do not pollute result namespace counts
- attention policy state summaries, attention selection summaries, and KV
  fusion merge summaries for adapter parity, including threshold spread,
  adapted profile count, selected/rejected accounting parity, cap pressure,
  rejected-attention pressure, result namespace mix, grouped namespace counts,
  and runtime/non-runtime block counts, noop state, block-accounting balance,
  clean accounting, merge/skip change causes, all-runtime/all-non-runtime
  result shapes, and namespace-count/result parity
- adapter execution context clamping and adapter string parsing
- adapter execution context summaries for post-hardware and post-runtime-clamp
  pressure, KV prefetch, precision, parallelism, and adapter-count parity
- adapter runtime clamp summaries for before/after context shape, runtime
  metadata limits, KV prefetch reduction, precision clamps, and preserved
  non-runtime-limit fields
- adapter observation selection inside allowed runtime adapters, including
  selection report counts and fallback reasons
- runtime/embedding diagnostics, contract-violation detection, and
  `InferenceOutcome` diagnostics propagation
- diagnostics seeding from `RuntimeRequestEnvelope` and saved
  `RuntimeAcceptanceContext` for response parity checks
- inference diagnostics summaries for route tokens, generation truncation,
  runtime/embedding execution signals, pressure band, recursive calls, notes,
  complete diagnostics signal, and route/runtime-KV activity
- runtime diagnostics request-envelope seeding for missing model, adapter,
  architecture, imported KV, and precision signals
- runtime diagnostics summaries for architecture, layer modes, device execution
  source, forward/KV signal, KV precision, and hardware acceptance counts,
  including control-plane-filled device execution as a distinct accepted signal
- runtime diagnostics hardware contract checks for device, lanes, and memory
  mode, including acceptance-report failure mapping
- hardware load normalization, snapshot summaries, pressure planning, KV budget
  pressure reduction, conversion to `AdapterExecutionContext`, and hardware
  plan summary reporting
- device execution plan summaries for lane, memory mode, adapter hint,
  parallelism, KV prefetch, precision, disk-spill parity, execution adapter
  signal/blocker counts, and hardware execution commit readiness
- hardware runtime failure-return projection preserves a blocked
  `device_execution` stage through `HardwareRuntimeCommitSummary`, covered by
  `hardware_runtime_failure_return_preserves_device_execution_stage`
- hardware adapter bridge summaries for lossless hardware plan to runtime
  execution context mapping, including adapter-count, pressure, latency,
  parallelism, KV-prefetch, precision, token-budget, disk-spill, focused drift
  component counts, preservation signals, aggregate drift presence, bridge
  blockers, accounting consistency, bridge commit readiness, bridge commit
  actions, mapped failure reports, primary failure summaries, and failure-batch
  formatter readiness
- runtime manifest digest validation, metadata round trip, validation failure
  mapping, non-blocking warnings, quantization policy, and preferred adapter
  selection
- runtime manifest validation summaries for clean, warnings-only, and invalid
  ABI shapes before request planning, including blocking-failure state and
  error/warning/mapped failure-report signal counts, aggregate validation signal
  presence, commit signal/blocker counts, commit accounting, clean commit state,
  and failure-report/error parity
- runtime transformer architecture summaries for layer/head/window shape and
  head-dimension validity before ABI parity
- runtime KV policy summaries for import/export capability, block capacity, and
  limit/capability consistency before KV exchange planning
- runtime manifest ABI summaries for effective context, architecture, KV
  limits, quantization widths, and adapter counts before request planning
- manifest-aware runtime KV export plan summaries for manifest/runtime export
  capability, requested export count, plan max blocks, and raw architecture
  layer/KV-head shape before transformer forward block materialization
- runtime quantization policy summaries for hot/cold KV precision, optional
  weight precision, compressed KV state, and cold-not-wider-than-hot parity
- runtime manifest adapter compatibility summaries for supported/execution
  adapter intersections, rejected observations, selected-adapter availability,
  missing adapter source detection, runtime-planning readiness, and
  observed/fallback selection state before runtime planning, including bounded
  compatibility counts, compatible adapter/observation fractions, selected
  source usability, adapter compatibility commit signal/blocker counts,
  accounting, clean commit state, and aggregate adapter-source problems
- runtime manifest execution compatibility summaries for KV import/export
  capacity, execution prefetch limits, disabled import requests, hot/cold
  precision coverage, overflow block/bit counts, missing manifest capacity, and
  compact device-gate readiness, including capacity, prefetch, precision, and
  focused execution-contract signal/problem component counts, aggregate signal
  and problem presence, and accounting consistency
- runtime device handoff readiness summaries for hardware runtime, runtime
  clamp, and manifest execution-device stages, including stage order, first
  unready/blocking stage, per-stage signal/blocker counts, aggregate handoff
  accounting, and commit readiness before request planning
- runtime device handoff keeps clean hardware/runtime-clamp stages from masking
  stricter manifest execution-device KV capacity or precision drift, with
  `runtime_device_handoff_routes_manifest_blockers_after_clean_runtime_clamp`
  locking the first blocking stage and
  `runtime_device_handoff_failure_return_preserves_manifest_execution_stage`
  locking failure-return stage text
- runtime context/max-token budget behavior
- runtime metadata shape summaries for context, embedding, KV exchange limits,
  precision, disabled-KV zero-capacity normalization, and budget-adjacent
  adapter assertions, including
  `runtime_metadata_readiness_degrades_without_committing_missing_embeddings`
  for the missing-embedding downgrade/commit-block boundary
- runtime failure classification for backend messages, diagnostics notes,
  class-count parity, and runtime/KV/contract failure categories
- runtime request envelope schema and contract checks
- runtime request envelope summaries for context pressure, adapter presence,
  layer parity, hardware pressure band, planning KV exchange, and recursive
  attachment
- runtime request envelope planning-digest checks for backend max tokens,
  selected adapter, generation budget, and KV prefetch/import parity
- runtime request gate summaries for clean send readiness, request-contract
  failure classes, imported-KV failure classes, and failure-report/count parity
- request acceptance context rejects actual imported KV payload counts that
  drift from planned KV imports before runtime request JSON
- request acceptance context proves metadata-driven KV prefetch degradation does
  not mask over-limit concrete imports at the request commit gate
- request manifest-planning readiness preserves real router-budget FHT-DKE KV
  prefetch degradation, while blocking request envelopes that still carry the
  pre-degrade import count at request planning parity
- runtime planning readiness preserves the committed hierarchical router
  threshold and route pressure across the FHT-DKE planning and runtime boundary
- request acceptance context keeps manifest KV policy drift visible after clean
  request planning, blocking manifest-aware request commit at the manifest bridge
  stage
- saved acceptance context keeps response exported KV drift visible against the
  manifest plan, blocking manifest boundary KV readiness at the response
  manifest-KV stage
- runtime KV side-effect readiness keeps clean import and clean manifest
  boundary commits from masking missing materialized export blocks; the final
  side-effect commit blocks at runtime KV export and returns a KV-export
  failure report
- runtime planning digest for context/max-token clamps, route pressure,
  adapter selection reports, KV prefetch limits, structured KV clamp reduction
  summaries, KV clamp reasons, and clamp self-consistency checks
- runtime planning preserves high-pressure hardware-derived adapter execution
  context through runtime KV-import disablement as a legal clamp signal via
  `planning_digest_keeps_hardware_pressure_runtime_kv_clamp_as_signal`
- runtime planning summaries for generation, adapter fallback, matched
  observations, missing/all-rejected observation signals, specific fallback
  causes, adapter-selection blocker counts, context hard/soft limiting,
  FHT-DKE, KV exchange, and hardware pressure at one pre-request gate
- manifest-aware runtime KV import plan summaries for manifest/runtime import
  capability, requested prefetch, plan limits, embedding dimensions,
  architecture import shape, capacity drift, and clean import-bridge readiness
- runtime planning manifest KV bridge summaries that compare planned
  import/export counts with manifest-derived import/export plans before root
  materializes runtime KV blocks
- runtime planning readiness summaries for FHT-DKE planning, runtime
  pre-request, and FHT-DKE/runtime-boundary stages, including first
  unready/blocking stage, per-stage signal/blocker counts, aggregate accounting,
  boundary drift counts, and runtime planning commit readiness
- runtime planning acceptance summaries for context exhaustion, contract
  failures, clean acceptance, violation counts, mapped failure-report counts,
  aggregate planning-acceptance problem counts, and accounting consistency
  before backend request construction
- FHT-DKE budget summaries for dense/routed fractions, token split validity,
  route pressure, routed-work state, routed tokens per KV exchange block, and
  route-pressure-driven KV exchange counts, including budget commit
  signal/blocker counts, empty-budget blockers, commit accounting, and clean
  budget commit readiness
- recursive schedule chunk/merge/wave planning and request-envelope checks
- recursive runtime-unit summaries for empty, single-pass, recursive, and
  parallel-wave schedules
- recursive schedule summary signal/problem counts for scheduler shape,
  recursion shape, execution-wave shape, and requested parallelism
- recursive validation summaries for shape, chunk, merge, and execution-wave
  failure classes, exact violation-count accounting, aggregate problem
  presence, component accounting, and clean validation before request-envelope
  attachment
- runtime response envelope schema and diagnostics/KV consistency checks
- runtime response envelope summaries for answer/token uncertainty,
  import/export KV exchange totals, diagnostics KV parity, and runtime
  execution signal state
- runtime request/response envelope commit helpers for signal counts, blocker
  counts, accounting parity, and compact pre-wire readiness checks
- runtime response envelope request-contract checks for generated-token cap,
  selected adapter, generation budget, route budget, hardware pressure, compute
  headroom, latency budget, and KV import/export parity
- runtime response gate summaries for clean parsed-response readiness,
  response-contract failure classes, request-parity failure classes,
  exported-KV failure classes, envelope/request-parity/exported-KV boundary
  drift, aggregate response blockers, and failure-report/count parity before
  reflection or memory mutation
- transformer plan digest counts and route-pressure summaries
- transformer plan summaries for layer mix, compute, and window bounds
- transformer planning pressure summaries that bridge route attention,
  attention selection, non-local/fusion layer mix, and average compute pressure
- transformer template-aware planner contract for coding, writing, and
  long-document profiles
- runtime KV export planning from manifest/runtime ABI and transformer forward
  summaries, including manifest bridge summaries, planned export count, export
  summaries, readiness gates, and materialized block validation before exported
  KV side effects
- FHT-DKE disabled/enabled budget behavior and route pressure effects
  with dense/routed fraction and KV-exchange summaries
- FHT-DKE planning commit summaries for committed budgets and repair on stale
  pressure/threshold drift before runtime planning
- safe default experiment switches, structured experiment summaries, enabled
  labels, and budget-expansion detection

## Main-Window Wiring Still Needed

- Add `crates/norion-core` to the root workspace once parallel windows converge.
- Add adapters from root `src/hierarchy`, `src/router`, `src/runtime`, and
  `src/engine` types to `norion_core` types.
- Map root `router::GenerationMetrics` into core `GenerationMetrics`, then
  feed `RoutingFeedback` into the selected core or root router observation path.
- Map root router state snapshots into core `RouterState` so threshold
  persistence can be parity-tested before router ownership moves. Compare
  `profile_observation_total()` and `has_observation_drift()` before replacing
  root state storage.
- Compare `ThresholdAttentionPolicy::policy_summary()` before and after
  applying root attention feedback so base/min/max thresholds, per-profile
  threshold drift, bounded state, threshold spread, and adapted profile count
  are stable before moving attention selection ownership.
- Route Web Lab/CLI/model-service `max_tokens` into `InferenceRequest` and then
  into the current `RuntimeBackend::configure_generation` / runtime request path.
- Route prompt token count into `InferenceRequest::with_prompt_tokens` so
  `RuntimeGenerationBudget` can prevent context overflow.
- Compare `RuntimeMetadata::shape_summary()` before request planning so root
  adapter tests can assert context, embedding, KV exchange, and precision facts
  before applying max-token clamps. Use the metadata shape signal helpers for
  observed context/KV/precision coverage and the problem helpers for KV
  support/capacity or precision ABI contradictions before verbose metadata
  diffs. Disabled KV import/export capabilities must keep max KV import/export
  blocks at zero even if callers provide non-zero limits. Gate root runtime
  metadata publication with
  `can_commit_runtime_metadata_adapter()` after checking missing metadata and
  adapter blocker counts; gate backend request max tokens with
  `can_commit_backend_max_tokens()` after checking backend request blockers.
- Build `RuntimeRequestEnvelope` before root `runtime_request_json(...)` so
  request schema, architecture, adapter candidates, and imported KV limits are
  validated before wire formatting. Attach `RuntimePlanningDigest` so planned
  backend `max_tokens`, selected adapter, and KV prefetch decisions are checked
  against the request fields.
- Compare `RuntimeRequestEnvelope::envelope_summary()` first at the request
  wire boundary so context truncation, adapter candidates, layer parity,
  hardware pressure band, planning KV exchange, and recursive state are stable
  before root parses violation strings.
- Compare `RuntimeRequestEnvelope::request_gate_summary(&imported_kv_blocks)`
  after planning and recursive attachments and before
  `runtime_request_json(...)`; only send backend JSON when
  `RuntimeRequestGateSummary::can_send_request()` is true.
- Validate root runtime KV imports from the saved request envelope with
  `RuntimeAcceptanceContext::from_request_parts(...).request_acceptance_report()`;
  treat over-limit block counts as adapter failures instead of relying on
  truncation.
- Build `RuntimePlanningDigest` from the mapped `InferenceRequest`, route
  budget, adapter execution context, adapter observations, and selected
  `FhtDkeBudgeter`; pass `backend_max_tokens()` and `planned_kv_exchange()` into
  the current runtime request path before serialization.
- Compare `RuntimePlanningDigest::adapter_selection_report` before backend
  request construction so missing allowed adapters, matched-observation drift,
  and fallback reasons are visible before request-envelope validation.
- After `FhtDkePlanningReadinessSummary` and
  `RuntimePlanningDigest::planning_summary()` are both available, build
  `RuntimePlanningReadinessSummary::new(...)` and require
  `can_commit_runtime_planning_readiness()` before root writes the backend
  request envelope or mutates concrete KV import candidates. This keeps route
  pressure and attention-threshold facts aligned from transformer planning
  through FHT-DKE and runtime planning.
- Compare `kv_prefetch_clamp_summary()` before root mutates concrete KV import
  lists, so requested, runtime-clamped, and planned import counts plus
  runtime-metadata/FHT-DKE reductions are attributed before candidates are
  dropped or deferred. Use `planned_kv_exchange().clamp_reason` as the compact
  reason code for the same boundary.
  Prefer `is_consistent()`, `reductions_match_total()`,
  `block_counts_match_reductions()`, `clamp_reason_matches_reductions()`, and
  `RuntimePlanningSummary::kv_clamp_is_consistent()` before concrete KV
  candidate diffs.
- When a `RuntimeManifestDigest` is available, compare
  `RuntimePlanningDigest::manifest_kv_bridge_summary(...)` before root calls
  import/export materialization helpers. This confirms the manifest-derived
  import/export plan limits match `planned_kv_exchange()` and exposes manifest
  policy drift separately from planning count drift.
- At the request boundary, prefer
  `RuntimeRequestEnvelope::manifest_request_planning_readiness_summary(...)`
  once the manifest and imported KV blocks are available. This wraps the
  manifest KV bridge together with request planning readiness, preserving the
  order manifest KV policy -> runtime/request planning -> request parity ->
  request gate before backend JSON serialization.
- Treat a clean `RuntimeRequestPlanningReadinessSummary` as necessary but not
  sufficient when a runtime manifest is present; manifest KV policy drift must
  still block at `RuntimeRequestManifestPlanningReadinessStage::ManifestKvBridge`
  before root materializes KV imports or serializes backend JSON.
- Compare `FhtDkeBudget::dense_fraction()`, `routed_fraction()`,
  `has_route_pressure()`, `route_pressure_is_high()`, `has_routed_work()`,
  `routed_tokens_per_kv_exchange_block()`, `token_split_is_valid()`, and
  `has_kv_exchange()` before root interprets route-pressure-driven dense/routed
  split and KV exchange changes.
  Prefer budget-shape problem helpers for token split, KV block-sum, and
  KV-exchange flag drift before expanding dense/routed token and concrete KV
  block fields. Treat route/KV pressure component helpers as visible
  classification signals rather than blockers.
- When `RuntimePlanningDigest::from_request(...)` clamps KV prefetch through
  runtime metadata, propagate `planned_kv_exchange().import_blocks` into both
  request imported-KV count and KV prefetch. A digest with clean pre-request
  planning still must not commit an envelope carrying stale pre-clamp KV counts.
- Compare `ExperimentSwitches::switches_summary()` at root request planning
  boundaries so enabled experimental paths, runtime-planning features,
  attention/KV features, and budget expansion are checked before Web
  Lab/CLI/agent diagnostics format labels or summary strings.
- Run `RuntimePlanningDigest::acceptance_report()` before the current runtime
  request path is built; convert failures with
  `RuntimePlanningAcceptanceReport::failure_reports()` so root preserves
  `runtime_context_exhausted` and `runtime_contract_violation` labels.
  Prefer report-level `failure_batch_summary()` and `primary_failure_summary()`
  before formatting planning failures.
- After attaching `RuntimePlanningDigest`, keep the concrete imported runtime KV
  block count equal to `planned_kv_exchange().import_blocks`; mismatch is a
  request acceptance failure, not a runtime concern.
- When a runtime manifest is attached, run
  `manifest_request_planning_readiness_summary(...)` before response execution
  so manifest KV import/export policy drift blocks at the bridge stage even if
  request acceptance and runtime planning are otherwise clean.
- Attach root recursive schedule summaries to `RuntimeRequestEnvelope` so
  prompt-token, chunk, merge-round, execution-wave, runtime-unit, merge-overhead,
  and parallelism summaries are checked before runtime request JSON is emitted.
  Use schedule signal/problem component helpers to distinguish legal recursive
  execution shape from malformed summary fields before verbose chunk diffs.
- Compare `RecursiveScheduleDigest::validation_summary()` before mapping full
  root recursive schedules, and compare
  `RecursiveScheduleSummary::validation_summary(request_prompt_tokens)` before
  attaching the lightweight request envelope summary.
- Build `RuntimeResponseEnvelope` after root `parse_runtime_response_json(...)`
  so answer, token, exported KV, and diagnostics consistency are validated
  before reflection and memory updates. Then call
  `RuntimeAcceptanceContext::response_acceptance_report(...)` with the parsed
  outcome before accepting runtime output. This preserves route budget, hardware
  pressure, compute headroom, latency budget, adapter, generation-budget, token,
  and KV exchange parity.
- Compare `RuntimeResponseEnvelope::envelope_summary()` before response
  acceptance so answer/token uncertainty, runtime execution signal, and exact
  imported/exported KV diagnostics parity are visible in one adapter report.
- Compare `RuntimeResponseEnvelope::response_gate_summary(...)` after
  `request_parity_summary(...)` and before memory/reflection mutation; only
  accept parsed runtime output when `RuntimeResponseGateSummary::can_accept_response()`
  is true.
- Validate exported runtime KV blocks with
  `RuntimeAcceptanceContext::response_acceptance_report(...)` before converting
  them back into root memory/cache records.
- When a manifest is present, compare
  `RuntimeAcceptanceContext::manifest_boundary_kv_summary(...)` before exported
  KV side effects; a clean request manifest-planning bridge still must not
  commit if parsed response exports exceed the manifest-derived export plan.
- Map root generated tokens into `GeneratedToken` and compare root
  `RuntimeTokenMetrics` with core `GeneratedTokenMetrics` before moving token
  uncertainty summaries into core diagnostics. Gate compact parity with
  `uncertainty_shape_is_clean()` and `can_use_token_uncertainty_metrics()`
  before verbose entropy/logprob payload diffs.
- Compare `InferenceOutcome::outcome_summary()` after root maps the parsed
  runtime response into `InferenceOutcome`, before building verbose response
  diagnostics or mutating reflection/memory state.
  Use `has_complete_runtime_response_shape()`,
  `kv_count_drifted_from_diagnostics()`, `runtime_execution_missing()`,
  `text_without_tokens()`, and `tokens_without_text()` as the first parsed
  response shape checks, then gate side-effect candidates with
  `response_shape_is_clean()` and `can_use_runtime_outcome()`.
- Map root `RuntimeManifest` into `RuntimeManifestDigest` before enabling
  runtime-device ABI gating in core. Run `validate()` first; map errors with
  `RuntimeManifestValidation::failure_reports()` and keep warnings as
  diagnostics.
  Prefer `validation_summary().is_clean_pass()`,
  `is_warnings_only_pass()`, `has_blocking_failures()`,
  `failure_reports_match_errors()`, and
  `can_commit_runtime_manifest_validation()` before expanding validation
  messages.
  Prefer report-level `failure_batch_summary()` and `primary_failure_summary()`
  before formatting mapped manifest validation errors.
- Compare `RuntimeManifestDigest::abi_summary()` before root request planning
  so metadata, architecture, KV policy, quantization, and supported adapter
  counts are checked as one effective ABI report. Prefer focused ABI
  signal/problem component helpers and `abi_accounting_is_consistent()` before
  expanding field-level manifest diffs or validation messages. Commit the
  manifest adapter source with `can_commit_runtime_manifest_adapter()` after
  checking manifest adapter signal and blocker counts.
- Compare `TransformerRuntimeArchitecture::architecture_summary()` and
  `RuntimeKvPolicy::kv_policy_summary()` before the full manifest ABI summary
  when root needs focused architecture or KV-policy parity failures.
- Compare `RuntimeQuantizationPolicy::quantization_summary()` before ABI
  summary parity when root needs a focused hot/cold/weight quantization report.
- Map root runtime/embedding diagnostics into `InferenceDiagnostics`, then use
  `RuntimeDiagnostics::contract_violations` before accepting exported runtime KV
  blocks.
- Compare `RuntimeDiagnostics::diagnostics_summary()` before verbose
  diagnostics diffs so model/adapter presence, architecture signal, layer-mode
  counts, device execution source, KV exchange, and precision validity are
  checked as one adapter report. Use the focused runtime diagnostics signal and
  problem component helpers before expanding nested runtime diagnostics text;
  signal counts are observational, while problem counts classify missing
  identity, architecture/activity, or precision shape.
- Compare `RuntimeDiagnostics::request_parity_summary(saved_request)` before
  response acceptance so model id, selected adapter, architecture, runtime KV
  exchange counts, and KV precision drift are visible without parsing
  diagnostics violation strings. Treat device-execution source as an
  observational response diagnostic unless the hardware contract summary marks
  device, lane, or memory-mode drift; a clean response envelope with adapter or
  KV-precision drift must still block commit through diagnostics/request parity
  or response diagnostics contract checks.
- Seed `InferenceDiagnostics` from the saved `RuntimeRequestEnvelope` before
  adding runtime-specific diagnostics, then run response/request backchecks.
- Compare `InferenceDiagnostics::diagnostics_summary()` before nested
  diagnostics diffs so route-token totals, generation truncation, runtime KV
  exchange, embedding fallback, hardware pressure band, recursion, and notes are
  stable at the outcome boundary.
  Use `has_complete_diagnostics_signal()`, `has_route_activity()`,
  `has_runtime_kv_exchange()`, and `has_runtime_or_embedding_execution()` before
  expanding nested runtime or embedding diagnostics.
- Compare `InferenceDiagnostics::request_parity_summary(saved_request)` before
  response-envelope parity so route budget, generation budget, hardware
  pressure, planning headroom/latency, and nested runtime diagnostics drift are
  visible in one adapter report.
  Prefer the compact drift helpers for routing, generation budget, hardware
  pressure, planning headroom/latency, runtime diagnostics, missing required
  reports, and aggregate request drift before formatting verbose parity errors.
- Seed missing `RuntimeDiagnostics` fields from the saved request envelope, but
  preserve any runtime-reported values so mismatches remain visible.
- Map root `HardwareSnapshot` / `HardwarePlan` into core hardware contracts so
  routing, diagnostics, and FHT-DKE all read the same pressure and KV budgets.
- Compare `HardwareLoadSnapshot::snapshot_summary()` before root hardware
  planning so normalized probe loads, dominant load kind, device tier, pressure,
  and pressure band are stable before full plan diffs. Prefer
  `snapshot_shape_is_clean()`, `can_use_hardware_snapshot()`, and
  `snapshot_accounting_is_consistent()` before expanding probe rows; use
  snapshot signal counts as observability and snapshot shape problem counts as
  malformed probe mappings.
- Compare `HardwarePlan::plan_summary()` pressure band, parallelism reduction,
  KV prefetch, hot KV precision, and disk-spill facts before moving root device
  planning ownership. Prefer the focused plan constraint signal helpers and
  `plan_constraint_signal_accounting_is_consistent()` before verbose device
  gate rows; these are hardware pressure/constraint observations, not automatic
  runtime send blockers.
- Compare `HardwarePlan::adapter_bridge_summary()` after mapping root hardware
  execution into core, so adapter counts, pressure, compute headroom, latency,
  parallelism, KV prefetch, precision, token budgets, and disk-spill state match
  the derived `AdapterExecutionContext` before runtime planning consumes it. Use
  `adapter_bridge_shape_is_clean()` and `can_use_hardware_adapter_bridge()` as
  the compact lossless bridge gate.
- Use `failure_return_summary()` on
  `HardwareLoadSnapshotCommitSummary`, `DeviceExecutionPlanCommitSummary`,
  `DeviceExecutionAdapterCommitSummary`, `HardwarePlanCommitSummary`,
  `HardwareAdapterBridgeCommitSummary`, and `HardwareRuntimeCommitSummary` when
  root needs one hardware commit-to-runtime-failure branch. The shared
  `HardwareFailureReturnSummary` exposes the source label, primary failure
  presence, failure batch, formatter readiness, blocker count, and
  `can_return_runtime_failure()`. `runtime_failure_return_report()` materializes
  `HardwareFailureReturnReport` only for blocked hardware commits and carries
  the primary `RuntimeFailureReport`, backend message, diagnostics note,
  blocking stage text such as `device_execution`, and `InferenceError`
  conversion.
- Compare `DeviceExecutionPlan::execution_summary()` before root maps hardware
  execution into runtime diagnostics, so lane, memory-mode, adapter-hint,
  parallelism, KV prefetch, precision, and disk-spill drift are visible early.
  Prefer focused execution signal/problem helpers and
  `execution_shape_accounting_is_consistent()` before verbose execution
  diagnostics; use `execution_shape_is_clean()` only for strict no-problem
  execution shapes and `execution_shape_risk_component_count()` only when root
  needs the compatibility aggregate.
- Use `RuntimeDiagnostics::hardware_contract_violations` before accepting
  runtime-reported or control-plane-filled device, lane, or memory-mode
  diagnostics.
- Prefer `RuntimeDiagnostics::hardware_acceptance_report(...)` at runtime
  response boundaries when root needs stable `RuntimeFailureReport` conversion
  for device execution mismatches.
- Compare
  `RuntimeHardwareDiagnosticsReport::diagnostics_summary()` before mapping
  hardware diagnostics failures into root runtime errors or trace labels.
  Prefer `failure_batch_summary()` and `primary_failure_summary()` when root
  needs hardware diagnostics failure class mix or first-failure shape before
  formatting device execution errors.
- Map root memory/Infini import candidates into `RuntimeKvCandidate` and use
  `RuntimeKvImportPlan::from_manifest(...)` after root tier filtering to create
  runtime namespace `KvBlock`s when a `RuntimeManifestDigest` is available.
  Compare `RuntimeKvImportPlan::manifest_plan_summary(...)` first so
  manifest/runtime import capability, requested prefetch, plan max blocks, and
  architecture import shape are stable before candidates are dropped or
  materialized.
- Compare `RuntimeKvImportPlan::import_summary(...)` before materializing
  imported blocks, then compare each mapped block's `shape_summary()` before
  payload-level validation.
- Validate root runtime KV imports/exports through `RuntimeKvBlockContract`
  before accepting blocks from control-plane or kernel boundaries.
- Compare `RuntimeKvValidationReport::validation_summary()` before converting
  payload violations into root runtime errors, so accepted block counts and
  violation counts are stable without parsing strings. Prefer
  `runtime_kv_validation_commit_signal_component_count()`,
  `runtime_kv_validation_commit_blocker_component_count()`,
  `runtime_kv_validation_commit_accounting_is_consistent()`,
  `runtime_kv_validation_commit_is_clean()`, and
  `can_commit_runtime_kv_validation()` before expanding individual validation
  messages. Validation signals are observational; only commit blockers stop
  payload side effects.
- Compare `RuntimeKvBlockContract::validation_boundary_summary(...)` at the same
  boundary so import/export direction, runtime KV failure trace label, limits,
  accepted/violation counts, boundary commit signal/blocker counts, accounting,
  and commit readiness are checked before root maps payload failures into
  runtime errors.
- Compare `KvBlockShapeSummary::runtime_exchange_shape_is_clean()` before
  accepting imported/exported runtime KV blocks when root needs focused
  namespace/token/vector shape diagnostics ahead of full payload validation.
  Use `can_use_runtime_exchange_block()` when a root adapter needs one compact
  block-level exchange gate.
- Prefer the request/response envelope helper methods for root wire adapters so
  KV import/export limits are always derived from the originating request.
- Prefer the acceptance report helpers at root request/response boundaries so
  contract, diagnostics, request parity, and KV payload failures share one
  adapter-facing report.
- Prefer `RuntimeAcceptanceContext` when root has both the saved request envelope
  and mapped hardware plan available, so request and response gates cannot drift
  onto different core facts.
- Prefer `RuntimeAcceptanceContext::from_request_parts(...)` over manual request
  envelope construction at root runtime boundaries; it performs the required
  hardware execution clamp before setting request KV prefetch fields.
- Prefer `request_acceptance_summary()` and
  `response_acceptance_summary(...)` on the saved context when root tests need
  stable class counts before mapping verbose failures.
- Prefer `boundary_envelope_summary(...)` on the saved context when root tests
  need one shape report for request token limits, imported KV request parity,
  response diagnostics KV parity, token uncertainty coverage/accounting,
  runtime execution signal, adapter candidate presence, and context-limited
  requests. Use the focused token/KV/diagnostics/runtime/adapter/context and
  uncertainty component counts before expanding either request or response
  acceptance reports.
- Convert acceptance report failures through `failure_reports()` before root
  builds `RuntimeError`, trace labels, diagnostics notes, or draft fallback
  answers.
- Compare `RuntimeFailureReport::batch_summary(...)` after collecting mapped
  failures so root tests can assert failure class mix, recoverability, backend
  error count, diagnostics note count, and trace confidence before formatting
  individual errors. Use `failure_batch_shape_is_clean()` and
  `can_format_runtime_failures()` as the compact formatting gate.
- When starting from request/response acceptance reports or manifest
  validation, prefer the report-level `failure_batch_summary()` and
  `primary_failure_summary()` helpers so mapped failure aggregation stays tied
  to the same report that produced the verbose violations.
- For single mapped failures, compare
  `RuntimeFailureReport::failure_summary()` first so root tests can assert
  trace-label, recoverability, backend-error wrapping, diagnostics-note, and
  trace-confidence shape without parsing the formatted message.
- Compare `acceptance_summary()` at request and response gates before consuming
  verbose violation strings, so root tests can assert contract/parity/KV failure
  classes and accepted KV block counts directly.
- Map root local/production forward summaries into `TransformerForwardSummary`
  and compare `TransformerForwardBatchSummary::from_summaries(...)` before
  using `RuntimeKvExportPlan::from_manifest(...)`,
  `manifest_plan_summary(...)`, `export_summary(...)`,
  `planned_block_count(...)`, `readiness_summary(...)`, and
  `readiness_summary_for_blocks(...)` to turn exported `KvBlock`s back into root
  `RuntimeKvBlock`s.
  Prefer export payload and planning summary signal helpers for forward input,
  forward activity, emitted export blocks, empty-forward skips, plan-limit hits,
  and planning-allowed export before row-level diffs. Use
  `forward_batch_shape_is_clean()` and `can_use_forward_batch()` before
  accepting a non-empty batch as the runtime KV export baseline. Gate planned
  export side effects with `runtime_kv_export_commit_is_clean()` and
  `can_commit_runtime_kv_export()`, then summarize materialized blocks with
  `RuntimeKvExportBlockSummary::from_blocks(...)` and require
  `runtime_kv_export_block_commit_is_clean()` plus
  `can_commit_runtime_kv_export_blocks()` before applying exported block
  mutation. When root materializes blocks itself, prefer
  `readiness_summary_for_blocks(...)` as the final exported-KV mutation gate so
  planned zero-export responses, missing blocks, namespace drift, and malformed
  block shapes are checked in one value.
- Map root `RuntimeAdapterObservation` into core `AdapterObservation` and use
  `AdapterSelection` after root hardware and experience filters pick candidates.
- Compare `AdapterExecutionContext::runtime_clamp_summary(...)` after root maps
  hardware execution and runtime metadata is available, so before/after adapter
  count, pressure, latency, parallelism, KV prefetch, precision, token budgets,
  and disk-spill state cannot drift silently before runtime planning. Then use
  `context_summary()` for focused post-clamp assertions where needed.
  Prefer `adapter_context_signal_component_count()`,
  `adapter_context_problem_component_count()`,
  `runtime_clamp_signal_component_count()`, and
  `runtime_clamp_problem_component_count()` before expanding field-level
  adapter execution diffs; clamp signals are legal runtime-limit reductions,
  while clamp problems indicate preservation drift or malformed post-clamp
  context. When hardware pressure already reduced adapter KV prefetch, runtime
  metadata can still disable KV import and surface as a clean planning clamp;
  root should treat that as a planning-pressure signal unless the clamp
  accounting reports blockers.
- Prefer `AdapterExecutionContext::select_adapter_report(...)` while root
  migration tests are staged, so allowed adapter count, matching observation
  count, rejected observations, selected-from-observation state, all-rejected
  observation state, and fallback reason can be compared before only the
  selected adapter is used.
- Map root `QuantizedVector` / `kv_quant` codecs into core `QuantizedVector`
  and `QuantizedKvBlock`, then use `KvQuantizationPlan` at runtime KV
  import/export boundaries. Compare `packed_payload_len()` and
  `compression_ratio()` before replacing persistence codecs.
- Compare `QuantizedKvBlock::payload_summary()` first when root needs one
  stable report for namespace, selected bits, vector lengths, packed lengths,
  and compression ratio.
  Prefer `payload_shape_balanced()`, `key_value_lengths_are_symmetric()`,
  `packed_lengths_are_symmetric()`, `is_compressed()`, and
  `uses_expected_namespace_bits(...)` before byte/string parity. Use
  `quantized_payload_signal_component_count()`,
  `quantized_payload_problem_component_count(...)`, and
  `quantized_payload_commit_signal_component_count()`,
  `quantized_payload_commit_blocker_component_count(...)`,
  `quantized_payload_commit_accounting_is_consistent(...)`, and
  `quantized_payload_commit_is_clean(...)` as the compact codec mutation gate
  before replacing persisted payloads.
- Compare `KvFusionMerge::merge_summary()` before root applies fused KV
  persistence, so collapsed-block state, skipped candidates, namespace mix,
  grouped namespace counts, and runtime/non-runtime result counts are checked
  before payload diffs.
  Prefer `has_clean_accounting()`, `changed_due_to_merges()`,
  `changed_due_to_skips()`, focused accounting drift component helpers,
  merge-fraction shape helpers, namespace-mix signal helpers, fusion boundary
  signal/problem helpers, `fusion_boundary_shape_is_clean()`,
  `can_use_kv_fusion_merge()`, result namespace-boundary signal helpers,
  `fusion_commit_blocker_component_count()`,
  `fusion_commit_accounting_is_consistent()`,
  `can_commit_kv_fusion_persistence()`, and all-runtime/all-non-runtime helpers
  before expanding fused block payloads. Prefer `commit_summary()` when root
  needs `KvFusionCommitAction`, mapped runtime failure reports, primary failure
  summary, failure batch, formatter readiness, empty-persistence problem
  detection, and commit-decision accounting before applying fused KV side
  effects.
- Compare `KvNamespaceCounts::drift_summary(...)` before root applies
  imported/exported/fused KV side effects when namespace distribution parity is
  the migration boundary. Prefer namespace distribution commit signal/blocker
  helpers and `commit_summary()` when root needs `KvNamespaceCountDriftCommitAction`,
  mapped runtime failure reports, failure batch formatting readiness, and
  commit-decision accounting before storage mutation.
- Map root `MemoryEntry` into `MemoryRecord` and compare root
  retention/compaction reports against core `MemoryGovernanceReport` before
  replacing root mutation code. Use `removed_ids()`, `total_removed()`, and
  `is_noop()` as the first report-level parity checks before touching root
  storage. Prefer `governance_signal_component_count()`,
  `governance_note_signal_component_count()`,
  `governance_problem_component_count()`,
  `governance_commit_signal_component_count()`,
  `governance_commit_blocker_component_count()`,
  `governance_commit_accounting_is_consistent()`, and
  `governance_commit_is_clean()` before applying root cache or disk mutation.
- Compare `MemoryRecord::summary()` and `TieredCachePlan::summary()` before
  root applies retention, compaction, tier movement, or disk updates.
  Prefer memory update/governance and tiered migration summary helpers for
  count balance, clean-noop, capacity-pressure, migration signals/problems,
  cache summary problem counts, and score-spread checks before comparing
  concrete cache rows. Gate tiered placement mutation with
  `tiered_cache_placement_commit_is_clean()` or
  `can_commit_tiered_cache_placement()` after checking
  `cache_distribution_signal_component_count()`,
  `cache_placement_signal_component_count()`,
  `cache_summary_problem_component_count()`,
  `cache_placement_blocker_component_count()`,
  `tiered_cache_placement_commit_signal_component_count()`, and
  `tiered_cache_placement_commit_blocker_component_count()`,
  then gate tier movement with `tier_migration_commit_is_clean()` after checking
  `migration_signal_component_count()`, `migration_boundary_problem_component_count()`,
  `tier_migration_commit_signal_component_count()`, and
  `tier_migration_commit_blocker_component_count()`.
- Keep Candle/model-service implementation outside `norion-core`; only implement
  the core traits from runtime/service crates.
