# norion-core Adapter Migration

This note defines the smallest adapter path from the current root `src` modules
to `crates/norion-core`. It is intentionally a design document only; root
`src/**` should keep running until the main window wires the adapters.

For field-level source/target mappings, see
`docs/architecture/norion-core-adapter-matrix.md`.
For compact runtime-failure return routing, see
`docs/architecture/norion-core-failure-return-matrix.md`.

## Adapter Boundary

Adapters should live in the root crate or thin integration crates, not inside
`norion-core`. The core crate owns stable vocabulary and policy ports:

- runtime: `RuntimeMetadata`, `RuntimeMetadataShapeSummary`,
  `RuntimeGenerationBudget`, `RuntimeBackendMaxTokensCommitAction`,
  `RuntimeBackendMaxTokensCommitSummary`, `InferenceRequest`, `InferenceOutcome`,
  `InferenceOutcomeSummary`, `GeneratedToken`, `GeneratedTokenMetrics`,
  `RuntimeFailureKind`, `RuntimeFailureReport`, `RuntimeFailureSummary`,
  `RuntimeFailureBatchSummary`, `RuntimeRequestEnvelope`,
  `RuntimeRequestEnvelopeSummary`, `RuntimeRequestPlanningParitySummary`,
  `RuntimeRequestGateSummary`, `RuntimeRequestPlanningReadinessStage`,
  `RuntimeRequestPlanningReadinessSummary`,
  `RuntimeRequestManifestPlanningReadinessStage`,
  `RuntimeRequestManifestPlanningReadinessSummary`,
  `RuntimeResponseEnvelope`,
  `RuntimeResponseEnvelopeSummary`,
  `RuntimeResponseRequestParitySummary`,
  `RuntimeResponseManifestKvSummary`,
  `RuntimeResponseGateSummary`, `RuntimeResponseReadinessStage`,
  `RuntimeResponseReadinessSummary`,
  `RuntimeRequestAcceptanceReport`, `RuntimeResponseAcceptanceReport`,
  `RuntimeRequestAcceptanceSummary`, `RuntimeResponseAcceptanceSummary`,
  `RuntimeRequestFailureReturnReport`, `RuntimeRequestFailureReturnSource`,
  `RuntimeRequestFailureReturnSummary`, `RuntimeResponseFailureReturnReport`,
  `RuntimeResponseFailureReturnSource`, `RuntimeResponseFailureReturnSummary`,
  `RuntimeAcceptanceContext`, `RuntimeBoundaryAcceptanceSummary`,
  `RuntimeBoundaryAdapterSummary`, `RuntimeBoundaryCommitReadinessSummary`,
  `RuntimeBoundaryCommitAction`, `RuntimeBoundaryCommitStage`,
  `RuntimeBoundaryCommitSummary`, `RuntimeBoundaryEnvelopeSummary`,
  `RuntimeBoundaryGateSummary`, `RuntimeBoundaryKvSummary`,
  `RuntimeFailureReturnReport`, `RuntimeFailureReturnSource`,
  `RuntimeFailureReturnSummary`,
  `RuntimeKvSideEffectProblemKind`,
  `RuntimeKvSideEffectReadinessSummary`, `RuntimeKvSideEffectStage`,
  `RuntimeManifestBoundaryCommitAction`,
  `RuntimeManifestBoundaryCommitProblemKind`,
  `RuntimeManifestBoundaryCommitReadinessSummary`,
  `RuntimeManifestBoundaryCommitSummary`,
  `RuntimeManifestBoundaryCommitStage`,
  `RuntimeManifestBoundaryKvStage`, `RuntimeManifestBoundaryKvSummary`,
  `RuntimePlanningDigest`,
  `RuntimePlanningSummary`, `RuntimePlanningReadinessSummary`,
  `RuntimePlanningReadinessStage`, `RuntimePlanningAcceptanceReport`,
  `RuntimePlanningAcceptanceSummary`, `RuntimePlanningAcceptanceCommitSummary`,
  `RuntimePlanningAcceptanceCommitAction`, `RuntimePlanningFailureReturnReport`,
  `RuntimePlanningFailureReturnSource`, `RuntimePlanningFailureReturnSummary`,
  `RuntimePlanningKvExchange`,
  `RuntimePlanningKvClampSummary`, `RuntimePlanningKvClampReason`,
  `RuntimePlanningManifestKvBridgeSummary`,
  `RUNTIME_REQUEST_SCHEMA`,
  `RUNTIME_RESPONSE_SCHEMA`
- recursive schedule: `RecursiveSchedulerConfig`, `RecursiveScheduleDigest`,
  `RecursiveScheduleSummary`, `RecursiveScheduleValidationSummary`,
  `RecursiveChunk`, `RecursiveMergeRound`, `RecursiveExecutionWave`
- diagnostics: `InferenceDiagnostics`, `InferenceDiagnosticsSummary`,
  `InferenceDiagnosticsRequestParitySummary`, `DiagnosticsPressureBand`,
  `RuntimeDiagnostics`, `RuntimeDiagnosticsSummary`,
  `RuntimeDiagnosticsContractSummary`, `RuntimeDiagnosticsRequestParitySummary`,
  `RuntimeHardwareDiagnosticsContractSummary`,
  `RuntimeHardwareDiagnosticsReport`, `RuntimeHardwareDiagnosticsSummary`,
  `EmbeddingDiagnostics`,
  `EmbeddingDiagnosticsSummary`, `DeviceExecutionSource`
- runtime manifest: `RuntimeManifestDigest`,
  `RuntimeManifestAdapterCompatibilitySummary`,
  `RuntimeManifestExecutionCompatibilitySummary`,
  `RuntimeDeviceHandoffReadinessSummary`, `RuntimeDeviceHandoffStage`,
  `RuntimeDeviceHandoffCommitSummary`, `RuntimeDeviceHandoffCommitAction`,
  `ManifestFailureReturnReport`, `ManifestFailureReturnSource`,
  `ManifestFailureReturnSummary`,
  `RuntimeManifestValidationSummary`, `RuntimeManifestAbiSummary`,
  `TransformerRuntimeArchitecture`, `TransformerRuntimeArchitectureSummary`,
  `RuntimeKvPolicy`, `RuntimeKvPolicySummary`, `RuntimeQuantizationPolicy`,
  `RuntimeQuantizationPolicySummary`, `QuantizationBits`
- routing/hierarchy: `TaskProfile`, `HierarchyWeights`,
  `HierarchyWeightsSummary`, `ProfileHierarchyWeights`,
  `ProfileHierarchyWeightsSummary`, `ProfileHierarchyObservations`,
  `RoutingContext`, `RouteBudget`, `RouteLayerCounts`,
  `RoutingDecisionSummary`, `RouteBudgetReadinessSummary`,
  `RouteBudgetReadinessStage`, `RouteBudgetReadinessCommitAction`,
  `RouteBudgetReadinessCommitSummary`, `GenerationMetrics`,
  `RoutingFeedback`, `RouterState`, `ProfileThresholds`,
  `ProfileObservations`
- attention: `AttentionCandidate`, `AttentionDecision`,
  `AttentionCandidateSummary`, `AttentionCandidateBatchSummary`,
  `AttentionDecisionSummary`, `AttentionSelectionReadinessSummary`,
  `AttentionSelectionReadinessStage`, `AttentionPolicy`,
  `ThresholdAttentionPolicy`, `ThresholdAttentionPolicySummary`
- adapter ABI: `RuntimeAdapter`, `AdapterObservation`, `AdapterSelection`,
  `AdapterSelectionReport`, `AdapterFallbackReason`, `AdapterExecutionContext`,
  `AdapterExecutionContextSummary`, `AdapterExecutionContextCommitSummary`,
  `AdapterRuntimeClampSummary`, `AdapterRuntimeClampCommitSummary`,
  `AdapterFailureReturnReport`, `AdapterFailureReturnSource`,
  `AdapterFailureReturnSummary`,
  `AdapterSelectionCommitSummary`, `AdapterSelectionRuntimeSummary`,
  `AdapterSelectionRuntimeCommitSummary`
- hardware: `DeviceClass`, `DeviceTier`, `DeviceProfileDescriptor`,
  `DeviceProfileDescriptorSummary`, `HardwareLoadSnapshot`,
  `HardwareLoadSnapshotSummary`, `HardwareLoadKind`,
  `HardwareLoadSnapshotCommitSummary`, `HardwareLoadSnapshotCommitAction`,
  `HardwarePlan`, `HardwarePlanSummary`, `HardwarePlanCommitSummary`,
  `HardwarePlanCommitAction`, `HardwareAdapterBridgeSummary`,
  `HardwareAdapterBridgeCommitSummary`,
  `HardwarePressureBand`, `DeviceExecutionPlan`, `DeviceExecutionPlanSummary`,
  `DeviceExecutionPlanCommitSummary`, `DeviceExecutionPlanCommitAction`,
  `DeviceExecutionAdapterSummary`, `DeviceExecutionAdapterCommitSummary`,
  `DeviceExecutionAdapterCommitAction`, `HardwareRuntimeReadinessSummary`,
  `HardwareRuntimeReadinessStage`, `HardwareRuntimeCommitSummary`,
  `HardwareRuntimeCommitAction`, `HardwareFailureReturnReport`,
  `HardwareFailureReturnSource`, `HardwareFailureReturnSummary`,
  `HardwareAllocator`
- transformer digest: `TransformerPlanDigest`, `TransformerLayerBudget`,
  `TransformerLayerBudgetSummary`, `TransformerLayerBudgetBatchSummary`,
  `TransformerAttentionKind`,
  `TransformerPlanSummary`,
  `TransformerPlanReadinessSummary`, `TransformerPlanReadinessStage`,
  `TransformerPlanningReadinessSummary`, `TransformerPlanningReadinessStage`,
  `TransformerPlanningPressureSummary`,
  `TransformerPlanningInput`,
  `TransformerPlannerContract`, `DefaultTransformerPlanner`,
  `TransformerForwardSummary`, `TransformerForwardBatchSummary`,
  `RuntimeKvExportPlan`, `RuntimeKvExportManifestPlanSummary`,
  `RuntimeKvExportPlanningSummary`,
  `RuntimeKvExportSummary`, `RuntimeKvExportBlockSummary`,
  `RuntimeKvExportReadinessSummary`,
  `RuntimeKvExportReadinessCommitSummary`,
  `RuntimeKvExportReadinessCommitAction`, `RuntimeKvExportReadinessStage`
- KV exchange: `KvNamespace`, `KvNamespaceCounts`, `KvBlock`,
  `KvNamespaceCountDriftSummary`, `KvNamespaceCountDriftCommitSummary`,
  `KvNamespaceCountDriftCommitAction`, `KvBlockShapeSummary`,
  `KvCachePort`, `RuntimeKvCandidate`, `RuntimeKvImportPlan`,
  `RuntimeKvImportManifestPlanSummary`, `RuntimeKvImportSummary`,
  `RuntimeKvImportBlockSummary`, `RuntimeKvImportReadinessSummary`,
  `RuntimeKvImportReadinessCommitSummary`,
  `RuntimeKvImportReadinessCommitAction`, `RuntimeKvImportReadinessStage`,
  `RuntimeKvExchangeFailureReturnReport`,
  `RuntimeKvExchangeFailureReturnSource`,
  `RuntimeKvExchangeFailureReturnSummary`,
  `RuntimeKvPersistenceFailureReturnReport`,
  `RuntimeKvPersistenceFailureReturnSource`,
  `RuntimeKvPersistenceFailureReturnSummary`,
  `RuntimeKvPersistenceFailureReturnSelection`,
  `RuntimeKvBlockContract`, `RuntimeKvBlockContractSummary`,
  `RuntimeKvBlockContractCheckSummary`, `RuntimeKvDirection`,
  `RuntimeKvValidationReport`, `RuntimeKvValidationSummary`,
  `RuntimeKvValidationBoundarySummary`, `KvFusionPolicy`, `KvFusionMergeSummary`,
  `KvFusionCommitSummary`, `KvFusionCommitAction`
- KV quantization: `QuantizedVector`, `QuantizedKvBlock`,
  `QuantizedKvPayloadSummary`, `KvQuantizationPlan`, `QuantizationError`
- tiered memory: `TieredMemoryCandidate`, `TieredMemoryCandidateSummary`,
  `TieredMemoryScheduler`, `TieredCachePlan`, `TieredCacheSummary`,
  `TierMigration`, `TierMigrationSummary`
- memory governance: `MemoryRecord`, `MemoryRecordSummary`,
  `MemoryGovernancePolicy`,
  `RetentionReport`, `MemoryCompactionReport`, `MemoryGovernanceReport`,
  `MemoryGovernanceSummary`, `MemoryUpdateSummary`,
  `MemoryUpdateBatchSummary`
- FHT-DKE: `FhtDkeInput`, `FhtDkeBudgeter`, `FhtDkeBudget`,
  `FhtDkeBudgetSummary`, `FhtDkePlanningReadinessStage`,
  `FhtDkePlanningReadinessSummary`, `FhtDkePlanningCommitAction`,
  `FhtDkePlanningCommitSummary`
  and experiment diagnostics through `ExperimentSwitches` and
  `ExperimentSwitchesSummary`

The root crate keeps model-server, Candle, process, HTTP, hardware probing,
persistence, quantization, diagnostics, and CLI/Web Lab formatting.

Failure-return adapters should first convert each family-specific
`failure_return_summary()` into `FailureReturnRoutingSummary`. Use
`FailureReturnRoutingKey` for stable `(family, source_label)` trace routing,
`can_route_runtime_failure()` before materializing a family-specific
`runtime_failure_return_report()`, and the batch/blocker fields for adapter
tests that should not depend on verbose error text.
When a root path evaluates more than one gate, build
`FailureReturnRoutingBatchSummary` from the routing summaries and use
`first_returnable_key`, `accounting_problem_count`, and
`can_select_runtime_failure_route()` before branching into the family-specific
report materializer.
Prefer `FailureReturnRoutingSelection::from_routes(...)` for the root adapter
branch because it returns the batch summary, routing decision, and selected
route from the same route slice.
For KV persistence, prefer
`RuntimeKvPersistenceFailureReturnSelection::from_summaries(...)` instead of a
hand-built two-route slice. It fixes the adapter order as namespace
distribution before fusion persistence, exposes the selected persistence source,
and turns non-canonical source order into repair-accounting behavior before a
runtime failure report is materialized.
Prefer `routing_decision()` as the final adapter branch: `Continue` means no
runtime failure should be materialized, `ReturnRuntimeFailure(key)` selects the
family/source report path, and `RepairAccounting` keeps malformed summary
accounting out of root runtime error formatting.
After `ReturnRuntimeFailure(key)`, use `select_route(...)` against the same
routing-summary slice to recover the selected summary for adapter assertions
before calling the family-specific `runtime_failure_return_report()`. When the
selection wrapper is used, require `can_materialize_runtime_failure()` before
materializing the report.

Recommended multi-gate route order for the first root failure-return adapter is
runtime planning acceptance -> runtime request acceptance -> runtime response
acceptance -> runtime KV exchange readiness -> runtime boundary commit. Add
adapter, hardware, manifest, and persistence families around that spine as
those root adapters are wired.

## Minimal Migration Order

1. Add pure conversion helpers in the root crate.
   - `src/hierarchy::TaskProfile` -> `norion_core::TaskProfile`
   - `src/hierarchy::HierarchyWeights` -> `norion_core::HierarchyWeights`
     mapping root `convolution` to core `fusion`
   - `src/hierarchy::ProfileHierarchyWeights` ->
     `norion_core::ProfileHierarchyWeights`
   - `src/hierarchy::ProfileHierarchyObservations` ->
     `norion_core::ProfileHierarchyObservations`
   - `src/router::RouteBudget` -> `norion_core::RouteBudget`
   - `src/router::GenerationMetrics` -> `norion_core::GenerationMetrics`
   - `src/router::RouterState` -> `norion_core::RouterState`
   - `src/router::ProfileThresholds` -> `norion_core::ProfileThresholds`
   - `src/router::ProfileObservations` -> `norion_core::ProfileObservations`
   - `src/hardware::RuntimeAdapterHint` -> `norion_core::RuntimeAdapter`
   - `src/runtime::RuntimeAdapterObservation` ->
     `norion_core::AdapterObservation`
   - `src/hardware::HardwarePlan` -> `norion_core::AdapterExecutionContext`
2. Route request limits through core.
   - Convert the current prompt token count and requested `max_tokens` into
     `InferenceRequest::with_prompt_tokens(...).with_max_tokens(...)`.
   - Call `InferenceRequest::generation_budget()` before invoking the concrete
     runtime.
   - If `RuntimeGenerationBudget::can_generate()` is false, summarize/repack
     context or return a root-level planning error before model execution.
   - Prefer `RuntimeGenerationBudget::context_exhausted()` and
     `truncated_but_can_generate()` over string matching when root needs to
     distinguish hard context exhaustion from a shortened generation request.
   - Compare `requested_context_overflow_tokens()`,
     `requested_generation_deficit_tokens()`, `planned_context_fraction()`, and
     `remaining_context_fraction()` before backend request construction so
     adapter tests can assert `context/max_tokens` pressure directly.
   - Gate context pressure with
     `requested_context_overflow_component_count()`,
     `requested_generation_deficit_component_count()`,
     `context_exhaustion_component_count()`,
     `context_soft_limit_component_count()`,
     `context_budget_signal_component_count()`,
     `has_context_budget_signal_components()`, and
     `context_budget_accounting_is_consistent()` before expanding verbose
     runtime planning diagnostics.
   - Validate public budget shape with `requested_context_matches_parts()`,
     `max_generated_not_above_requested()`,
     `planned_context_matches_generation()`,
     `known_context_window_bounds_are_consistent()`,
     `context_budget_shape_problem_component_count()`,
     `context_budget_shape_accounting_is_consistent()`,
     `context_budget_shape_is_clean()`, and `can_use_backend_max_tokens()`
     before passing the planned backend max-token value into root runtime code.
   - Prefer `RuntimeGenerationBudget::backend_max_tokens_commit_summary()` as
     the single root adapter branch immediately before backend request
     construction. `CommitBackendMaxTokens` exposes `backend_max_tokens`,
     `ReturnContextExhausted` maps to the planning/request failure path, and
     `RepairContextBudget` keeps malformed public context-budget accounting out
     of concrete backend calls.
   - Build `RuntimePlanningDigest` from the request, route budget, adapter
     execution context, adapter observations, and the active `FhtDkeBudgeter`.
   - Compare `RuntimePlanningDigest::adapter_selection_report` before backend
     request construction; missing allowed adapters should fail planning
     acceptance, while no matching observations should remain a visible fallback
     reason. Use `adapter_selection_commit_signal_component_count()`,
     `adapter_selection_commit_blocker_component_count()`,
     `adapter_selection_commit_accounting_is_consistent()`, and
     `can_commit_adapter_selection()` before expanding individual observation
     rows or fallback strings.
   - Compare `AdapterExecutionContext::selection_runtime_summary(...)` after
     runtime diagnostics are available so the runtime-reported adapter is
     checked against the planned selection and allowed execution context before
     verbose diagnostics diffs.
     Use its adapter-source signals, runtime-report confirmation signals,
     fallback-reason problem count, aggregate runtime adapter problem count,
     accounting consistency, and clean execution gate before mapping response
     diagnostics into root trace text.
   - Compare `RuntimePlanningDigest::planning_summary()` as the first
     pre-request adapter assertion so generation budget, adapter fallback,
     matched observations, FHT-DKE summary, KV exchange clamp, and hardware
     pressure are checked together before root expands field-level diffs.
   - Prefer the planning summary helpers for selected-from-observation,
     missing allowed candidates, all-rejected adapter observations,
     no-allowed-adapter fallback, no-matching-observation fallback, hard context
     exhaustion, and soft context limiting before matching verbose strings.
   - Use `adapter_selection_blocker_component_count()`,
     `adapter_observation_signal_component_count()`, and
     `adapter_planning_signal_component_count()` to separate hard adapter
     planning blockers from missing/all-rejected observation fallback signals.
   - Gate backend request readiness with
     `generation_readiness_blocker_component_count()`,
     `parallelism_readiness_blocker_component_count()`,
     `adapter_selection_blocker_component_count()`,
     `fht_dke_token_split_blocker_component_count()`,
     `request_readiness_blocker_component_count()`,
     `has_request_readiness_blockers()`, and
     `request_readiness_accounting_is_consistent()`.
   - Gate the full pre-request adapter boundary with
     `fht_dke_budget_commit_signal_component_count()`,
     `fht_dke_budget_commit_blocker_component_count()`,
     `fht_dke_budget_commit_accounting_is_consistent()`,
     `fht_dke_budget_shape_problem_component_count()`,
     `kv_clamp_consistency_problem_component_count()`,
     `pre_request_gate_problem_component_count()`,
     `has_pre_request_gate_problem_components()`, and
     `pre_request_gate_accounting_is_consistent()` before root serializes a
     backend request. This gate catches empty FHT-DKE budgets and public
     budget-shape drift even when the narrower request-readiness blockers are
     clear.
   - Use `backend_request_commit_signal_component_count()`,
     `backend_request_commit_blocker_component_count()`,
     `backend_request_commit_accounting_is_consistent()`, and
     `can_commit_backend_request()` as the final planning-summary commit gate
     before root writes the backend request envelope or runtime JSON payload.
   - After `FhtDkePlanningReadinessSummary` and
     `RuntimePlanningDigest::planning_summary()` are both available, build
     `RuntimePlanningReadinessSummary::new(...)` and require
     `can_commit_runtime_planning_readiness()` before backend request
     serialization or concrete KV import mutation. Use its stage order,
     first-unready/blocking stage, per-stage signal/blocker counts, and
     FHT-DKE/runtime-boundary drift count before expanding verbose planning
     fields.
   - After building the request envelope, request planning parity, and request
     gate, build `RuntimeRequestPlanningReadinessSummary::new(...)` from the
     already-checked runtime planning readiness plus the request summaries.
     Require `can_commit_runtime_request_planning()` before backend request
     serialization, and use `RuntimeRequestPlanningReadinessStage`,
     `first_unready_stage()`, `first_blocking_stage()`, per-stage
     signal/blocker counts, request-planning parity drift counts, request-gate
     blocker counts, and aggregate accounting before expanding verbose request
     diffs.
     Prefer `RuntimeRequestEnvelope::request_planning_readiness_summary(...)`
     once the concrete imported KV blocks exist, so root does not rebuild
     planning parity and request gate summaries from separate request facts.
     Prefer `RuntimeAcceptanceContext::request_planning_readiness_summary(...)`
     when the saved request/imported-KV context is available.
   - Use `pre_request_gate_signal_component_count()` and
     `has_pre_request_gate_signals()` for non-blocking adapter-observation and
     route/FHT/KV pressure observability; do not treat those as send blockers.
   - Use `planning_pressure_signal_component_count()` plus the route-pressure,
     high-pressure, and routed-KV signal helpers as classification signals for
     diagnostics, not automatic blockers.
   - Gate planning with `RuntimePlanningDigest::acceptance_report()` before
     root builds the backend request; context exhaustion maps to
     `runtime_context_exhausted`, while malformed planning state maps to
     `runtime_contract_violation`.
    - Compare `RuntimePlanningAcceptanceReport::acceptance_summary()` before
      verbose planning violations so root tests can assert context exhaustion,
      contract failure counts, total planning violations, and failure-report
      counts without string matching.
    - Use `RuntimePlanningAcceptanceReport::failure_batch_summary()` and
      `primary_failure_summary()` when root needs planning failure class mix or
      the first mapped planning failure shape without collecting reports by hand.
    - Use `RuntimePlanningAcceptanceReport::commit_summary()` when root needs
      one object for backend-request acceptance,
      `RuntimePlanningAcceptanceCommitAction`, mapped failure reports, primary
      failure summary, failure batch, and runtime-error formatting gates. Call
      `failure_return_summary()` before formatting root planning errors, then
      materialize `runtime_failure_return_report()` only when the summary can
      return a runtime failure.
    - Use `RuntimePlanningAcceptanceSummary::is_clean_acceptance()` and
      `failure_report_matches_failures()` for the first acceptance check; expand
     violations only after those summary counts drift. Prefer
     `planning_acceptance_shape_is_clean()` or `can_accept_runtime_planning()`
     when the root adapter only needs one planning acceptance gate.
     Prefer the focused component helpers for planning-violation presence,
     context exhaustion, contract failure, mapped failure-report presence,
     aggregate planning-acceptance problems, accepted-state parity,
     failure-report parity, shape problems, and acceptance accounting
     consistency before matching verbose planning text.
   - Use the digest to parity-test selected adapter, context truncation,
     FHT-DKE route pressure, and KV prefetch clamping before root serializes the
     runtime request.
   - Compare `kv_prefetch_clamp_summary()` so root can distinguish requested,
     runtime-clamped, and planned import counts, including runtime metadata and
     FHT-DKE reduction amounts, before changing concrete KV import lists.
   - Use `planned_kv_exchange().clamp_reason` as the compact reason code after
     the structured clamp summary has been checked.
   - When a runtime manifest is available, build the concrete import plan with
     `RuntimeKvImportPlan::from_manifest(...)` and compare
     `RuntimeKvImportPlan::manifest_plan_summary(...)` before materializing
     imported runtime KV blocks. This keeps manifest import policy, runtime
     metadata capacity, requested prefetch, embedding dimensions, and
     architecture layer/KV-head shape in one adapter-facing bridge.
   - Before materializing either import or export blocks from a manifest, compare
     `RuntimePlanningDigest::manifest_kv_bridge_summary(...)` so the
     manifest-derived import/export plan limits match
     `planned_kv_exchange()`. Use
     `can_use_runtime_planning_manifest_kv_bridge()` as the compact
     manifest+planning KV gate.
   - After the request envelope has the same planning digest attached, prefer
     `RuntimeRequestEnvelope::manifest_request_planning_readiness_summary(...)`
     when a runtime manifest is available. This keeps the manifest KV bridge,
     request-planning parity, and request gate in one pre-JSON readiness value.
     Prefer `RuntimeAcceptanceContext::manifest_request_planning_readiness_summary(...)`
     when the saved request/imported-KV context already exists, so request
     manifest-planning readiness and later response manifest-KV checks share
     one set of request facts.
   - Prefer `RuntimePlanningKvClampSummary::is_consistent()`,
     `clamp_counts_are_bounded()`, `reductions_match_total()`,
     `block_counts_match_reductions()`, `clamp_reason_matches_reductions()`,
     `clamp_shape_problem_component_count()`,
     `clamp_shape_accounting_is_consistent()`, `clamp_shape_is_clean()`, and
     `can_use_runtime_planning_kv_clamp()` plus
     `RuntimePlanningSummary::kv_clamp_is_consistent()` before expanding
     concrete KV candidate diffs.
   - Build `RuntimeRequestEnvelope` from the mapped request, architecture,
     route budget, hierarchy, transformer digest, execution context, and
     imported KV count before root formats the JSON wire request.
   - Compare `RuntimeRequestEnvelope::planning_parity_summary()` after
     attaching `RuntimePlanningDigest` so backend max tokens, generation budget,
     selected adapter, imported KV, KV prefetch, planned export count, and
     planning contract violations are structured before verbose request diffs.
     Include `planning_pre_request_problem_count` and
     `planning_pressure_signal_count` so root can block invalid pre-request
     plans while preserving route/FHT/KV pressure as classification signals.
  - Classify request/planning drift with
    `planning_missing_from_request()`, `token_drift_component_count()`,
    `adapter_drift_component_count()`, `kv_drift_component_count()`, and
    `request_planning_drift_component_count()` before expanding request
    parity violations or serializing backend JSON.
    Use `planning_attachment_drift_component_count()`,
    `max_token_drift_component_count()`,
    `generation_budget_drift_component_count()`,
    `imported_kv_drift_component_count()`,
    `kv_prefetch_drift_component_count()`, and
    `planning_contract_drift_component_count()` when root tests need focused
    request/planning parity counts before verbose violations.
    Use `planning_pre_request_gate_problem_component_count()`,
    `planning_pressure_signal_component_count()`,
    `backend_wire_problem_component_count()`,
    `has_backend_wire_problem_components()`, and
    `backend_wire_accounting_is_consistent()` as focused wire parity checks.
    Use `backend_wire_shape_is_clean()` and `can_use_backend_wire_request()` as
    the final compact wire parity gate before serializing backend JSON.
   - Compare `RuntimeRequestEnvelope::envelope_summary()` before verbose
     request checks so context pressure, adapter presence, layer parity,
     hardware pressure band, planned KV exchange, and recursive attachment are
     visible as one stable adapter report.
   - Run `RuntimeRequestEnvelope::request_gate_summary(imported_kv_blocks)`
     after `planning_parity_summary()` and `envelope_summary()` and before
     `runtime_request_json(...)`; only serialize the backend request when
     `RuntimeRequestGateSummary::request_gate_shape_is_clean()` and
     `RuntimeRequestGateSummary::can_send_runtime_request()` are true.
  - Compare `RuntimeRequestGateSummary::is_clean_send_gate()` and
    `failure_report_matches_failures()` first, then use
    `request_gate_shape_is_clean()` to catch public summary accounting drift
    before JSON serialization; if they fail, classify the drift with
    `has_request_contract_failures()` and `has_imported_kv_failures()` before
    expanding verbose violations.
    Use `acceptance_failure_component_count()`,
    `boundary_drift_component_count()`, `backend_wire_problem_component_count()`,
    `direct_backend_wire_problem_component_count()`,
    `planning_pre_request_gate_problem_component_count()`,
    `planning_pressure_signal_component_count()`,
    `imported_kv_activity_signal_component_count()`,
    `send_gate_signal_component_count()`, and
    `send_blocker_component_count()` when root needs one pre-JSON blocker count
    for request acceptance, envelope drift, planning drift, backend-wire
    parity, and mapped failure reports. `send_gate_signal_component_count()`
    should feed observability and routing, while `send_blocker_component_count()`
    remains the send-stop predicate. Use
    `runtime_request_commit_signal_component_count()`,
    `runtime_request_commit_blocker_component_count()`,
    `runtime_request_commit_accounting_is_consistent()`, and
    `can_commit_runtime_request()` before root serializes runtime request JSON
    or mutates backend send state.
   - Attach the same `RuntimePlanningDigest` to the request envelope so root
     catches mismatched backend `max_tokens`, selected adapter, or KV prefetch
     count before JSON serialization.
   - Ensure the concrete imported runtime KV block vector length equals
     `RuntimePlanningDigest::planned_kv_exchange().import_blocks`; root should
     drop or defer extra candidates before building the acceptance context, and
     missing blocks should fail request parity tests.
   - Prefer `RuntimeAcceptanceContext::from_request_parts(...)` to build the
     request envelope and saved acceptance context from the mapped request,
     architecture, route budget, hierarchy, transformer digest, mapped
     `HardwarePlan`, and concrete imported runtime KV blocks. This constructor
     clamps `HardwarePlan::adapter_execution_context()` with the request runtime
     metadata before request KV prefetch fields are set.
   - Gate the concrete runtime request with
     `RuntimeAcceptanceContext::request_acceptance_report()`; imported block
     counts above the request-derived contract should fail adapter parity tests
     instead of being silently truncated.
   - Prefer `RuntimeAcceptanceContext::request_acceptance_summary()` for the
     first request-boundary parity assertion when a saved context is available.
    - Convert request acceptance failures with
      `RuntimeRequestAcceptanceReport::failure_reports()`; request contract
      failures become `runtime_contract_violation`, while imported KV payload
      failures become `runtime_kv_import_error`.
    - Use `RuntimeRequestAcceptanceReport::failure_batch_summary()` and
      `primary_failure_summary()` when root needs failure class mix or the first
      mapped failure shape without collecting reports by hand.
    - Use `failure_return_summary()` and `runtime_failure_return_report()` on
      `RuntimeRequestAcceptanceReport` when root wants primary failure
      materialization before wiring the full boundary commit gate.
    - Compare `RuntimeRequestAcceptanceReport::acceptance_summary()` before
      verbose failure text so root tests can assert request-contract and
     imported-KV failure classes plus accepted imported block count directly.
  - Prefer `RuntimeRequestAcceptanceSummary::is_clean_acceptance()`,
    `has_failures()`, and `failure_report_matches_failures()` for compact
    request-boundary assertions before mapping `RuntimeFailureReport`s.
    Use `request_contract_failure_component_count()`,
    `imported_kv_failure_component_count()`, and
    `request_acceptance_problem_component_count()` when root tests need focused
    request acceptance class counts before verbose failure text. Gate adapter
    imports with `request_acceptance_accounting_is_consistent()` so accepted
    state, violation totals, failure classes, mapped reports, and aggregate
    problem counts agree before JSON serialization. Use
    `runtime_request_acceptance_commit_signal_component_count()`,
    `runtime_request_acceptance_commit_blocker_component_count()`,
    `runtime_request_acceptance_commit_accounting_is_consistent()`, and
    `can_commit_runtime_request_acceptance()` when the adapter only needs one
    request acceptance decision.
   - Convert root `RecursiveSchedule` into `RecursiveScheduleSummary` and attach
     it with `RuntimeRequestEnvelope::with_recursive_schedule(...)`.
3. Keep existing runtime execution, but expose metadata through core.
   - Map root `src/runtime/types/metadata.rs` to `norion_core::RuntimeMetadata`.
   - Preserve KV import/export flags, import/export block limits, and hot/cold
     KV precision.
   - Compare `RuntimeMetadata::shape_summary()` before max-token and KV import
     planning so root tests can assert context, embedding, KV exchange capacity,
     and precision shape without parsing summary strings.
     Prefer `metadata_shape_signal_component_count()` for observed
     context/embedding, KV capability, and precision coverage, and
     `metadata_shape_problem_component_count()` for malformed KV
     support/capacity or precision ABI contradictions. Use
     `metadata_shape_is_clean()` and `can_use_runtime_metadata_contract()` as
     the compact metadata contract gate before root expands runtime metadata
     ABI diagnostics or computes the generation budget.
     Use `runtime_metadata_adapter_missing_component_count()` and
     `runtime_metadata_adapter_blocker_component_count()` to separate absent
     context/embedding facts from malformed ABI metadata, then require
     `can_commit_runtime_metadata_adapter()` before publishing root runtime
     adapter metadata as the concrete planning source.
   - Compare `RuntimeManifestDigest::abi_summary()` before request planning
     when root has manifest data available. Use `abi_shape_is_clean()` or
     `can_use_runtime_manifest_abi()` for the compact manifest ABI gate before
     expanding context, transformer, KV, quantization, or adapter catalog diffs.
   - Compare `TransformerRuntimeArchitecture::architecture_summary()` before
     full ABI checks. Gate mapped architecture with
     `transformer_runtime_architecture_commit_*` signal/blocker/accounting
     helpers and
     `can_commit_transformer_runtime_architecture(native_context_window)` so
     local-window overflow, invalid head geometry, and zero dimensions are
     caught before backend request construction.
   - Compare `RuntimeKvPolicy::kv_policy_summary()` and
     `RuntimeQuantizationPolicy::quantization_summary()` before materializing
     runtime KV exchange or quantization ABI rows. Use
     `runtime_kv_policy_commit_*` and `runtime_quantization_policy_commit_*`
     helpers for focused signal/blocker/accounting assertions, then
     `can_commit_runtime_kv_policy()` and
     `can_commit_runtime_quantization_policy()` as the adapter checks.
   - Run `RuntimeManifestValidation::validation_summary()` before mapping
     manifest failures into root runtime errors. Use
     `runtime_manifest_validation_commit_signal_component_count()`,
     `runtime_manifest_validation_commit_blocker_component_count()`,
     `runtime_manifest_validation_commit_accounting_is_consistent()`,
     `runtime_manifest_validation_commit_is_clean()`, and
     `can_commit_runtime_manifest_validation()` for the final validation
     accept/reject gate; warnings-only passes remain acceptable.
   - Clamp `AdapterExecutionContext` with `clamp_for_runtime(...)` before
     planning KV prefetch. `RuntimeAcceptanceContext::from_request_parts(...)`
     performs this clamp for the request envelope path.
   - Compare `AdapterExecutionContext::runtime_clamp_summary(...)` before
     request planning so runtime metadata KV prefetch and precision reductions
     are visible while adapter count, pressure, latency, parallelism, token
     budgets, and disk-spill preservation stay structured.
     Use `adapter_context_signal_component_count()` and
     `adapter_context_problem_component_count()` for the mapped context shape,
     then `runtime_clamp_signal_component_count()` and
     `runtime_clamp_problem_component_count()` to keep legal runtime-limit
     reductions separate from preservation drift or malformed post-clamp
     context. Gate the mapped context with
     `adapter_execution_context_commit_*` helpers and
     `can_commit_adapter_execution_context()`, then gate the clamp summary with
     `runtime_clamp_commit_*` helpers and `can_commit_runtime_clamp()`.
     Prefer `AdapterExecutionContextSummary::commit_summary()` when root needs
     one object for `AdapterExecutionContextCommitAction`, mapped runtime
     failure reports, primary failure summary, failure batch, formatter
     readiness, and mapped-context commit-decision accounting.
     Prefer `AdapterRuntimeClampSummary::commit_summary()` when root needs one
     object for `AdapterRuntimeClampCommitAction`, mapped runtime failure
     reports, primary failure summary, failure batch, formatter readiness, and
     runtime-clamp commit-decision accounting.
4. Convert root diagnostics into core diagnostics.
   - Map root `reflection::RuntimeDiagnostics` into
     `norion_core::RuntimeDiagnostics`.
   - Map root `engine::EmbeddingDiagnostics` into
     `norion_core::EmbeddingDiagnostics`.
   - Compare `EmbeddingDiagnostics::diagnostics_summary()` before folding
     embedding facts into `InferenceDiagnostics` so query, memory-write,
     gist-write, runtime-call, fallback-call, and source-mix parity stay
     structured.
     Use `embedding_signal_component_count()` for legal embedding activity and
     `embedding_shape_problem_component_count()`,
     `embedding_accounting_is_consistent()`, and `embedding_summary_is_clean()`
     to block zero-dimension, memory-write, gist-count, or total-call drift
     before response diagnostics are built. Use
     `can_use_embedding_diagnostics()` as the final compact adapter gate.
   - Copy route budget, generation budget, hardware pressure, compute headroom,
     latency budget, recursive runtime call count, and compact runtime error
     notes into `InferenceDiagnostics`.
   - Prefer `RuntimeAcceptanceContext::inference_diagnostics_seed()` when the
     saved request/hardware context exists; otherwise use
     `InferenceDiagnostics::from_request_envelope(...)` or
     `with_request_envelope(...)` so request/planning fields are populated once
     and runtime-specific diagnostics can be layered on top.
   - Prefer `RuntimeAcceptanceContext::runtime_diagnostics_seed()` when root
     only needs the nested runtime baseline for model, selected adapter,
     architecture, imported KV count, and KV precision parity.
   - Compare `InferenceDiagnostics::diagnostics_summary()` before nested
     diagnostics diffs so root tests can assert generation truncation, route
     token counts, runtime KV exchange, runtime/embedding execution signals,
     pressure band, recursive calls, and note count in one report.
   - Prefer `has_complete_diagnostics_signal()`, `has_route_activity()`,
     `has_runtime_kv_exchange()`, and `has_runtime_or_embedding_execution()`
     before expanding nested runtime or embedding diagnostics.
   - Compare `InferenceDiagnostics::request_parity_summary(saved_request)`
     before response-envelope parity so route budget, generation budget,
     hardware pressure, planning headroom/latency, and nested runtime
     diagnostics request parity are structured before verbose diffs.
   - Use the request parity drift helpers for routing drift, missing/drifted
     generation budget, hardware pressure drift, planning headroom/latency
     drift, runtime diagnostics drift, missing required diagnostics reports, and
     aggregate request drift before formatting verbose parity errors.
     Prefer `routing_drift_component_count()`,
     `generation_drift_component_count()`, `hardware_drift_component_count()`,
     `diagnostics_request_drift_component_count()`,
     `has_diagnostics_request_drift_components()`, and
     `diagnostics_request_accounting_is_consistent()` for first-pass adapter
     assertions. Use `diagnostics_request_parity_shape_is_clean()` or
     `can_accept_inference_diagnostics_request_parity()` when the adapter only
     needs a single accept/reject gate.
   - Prefer `RuntimeDiagnostics::from_request_envelope(...)` or
     `with_request_envelope(...)` when runtime output omits model id, selected
     adapter, architecture, imported KV count, or KV precision; preserve
     runtime-reported values when present.
    - Compare `RuntimeDiagnostics::diagnostics_summary()` before verbose
      diagnostics diffs so root tests can assert model/adapter presence,
      architecture signal, layer-mode count, device execution source, forward/KV
      signal, KV exchange count, and precision validity without string parsing.
   - Prefer `RuntimeDiagnosticsSummary::has_runtime_identity()`,
     `has_runtime_kv_exchange()`, and `has_complete_runtime_signal()` when root
     tests only need compact runtime diagnostics shape checks.
     Use `runtime_diagnostics_signal_component_count()` for observational
     model/adapter, architecture/layer-mode, device execution, forward, KV, and
     precision signal coverage, and `runtime_diagnostics_problem_component_count()`
     for the compact missing-identity, architecture/activity, or precision
     diagnostics-shape gate before verbose runtime diagnostics diffs.
      Prefer `runtime_diagnostics_shape_is_clean()` and
      `can_use_runtime_diagnostics()` at adapter boundaries that need one
      reusable runtime diagnostics gate.
    - Compare `RuntimeDiagnostics::contract_summary(...)` before
      `contract_violations(...)` when root needs diagnostics-vs-metadata,
      architecture, adapter, or KV precision drift classes. Prefer the focused
      identity, architecture, adapter, and precision problem helpers plus
      `can_accept_runtime_diagnostics_contract()` before mapping failures to a
      runtime contract violation.
    - Compare `RuntimeDiagnostics::request_parity_summary(saved_request)` before
      response-envelope acceptance so root tests can assert model id, selected
      adapter, architecture, runtime KV counts, export bounds, and KV precision
      against the saved request without string parsing.
   - Use the request parity drift helpers first when classifying runtime
     diagnostics failures: `model_drifted()`, `adapter_drifted()`,
     `architecture_drifted()`, `kv_count_drifted()`,
     `exported_kv_block_overflow()`, `precision_drifted()`, and
     `missing_required_runtime_report()`.
     Prefer `missing_report_component_count()`,
     `identity_drift_component_count()`, `architecture_drift_component_count()`,
     `kv_drift_component_count()`, `precision_drift_component_count()`, and
     `runtime_drift_component_count()` before verbose diagnostics parity text.
     Use `has_runtime_drift_components()` and
     `runtime_drift_accounting_is_consistent()` before mapping nested runtime
     diagnostics drift into response errors or trace notes. Use
     `runtime_request_parity_shape_is_clean()` or
     `can_accept_runtime_diagnostics_request_parity()` when root needs one
     parity acceptance decision.
   - Use the focused missing-report helpers for model id, selected adapter,
     architecture, and KV precision when root needs to distinguish absent
     runtime diagnostics from drifted runtime diagnostics.
   - Use `AdapterExecutionContext::selection_runtime_summary(...)` with the
     saved planning report and `RuntimeDiagnostics::selected_adapter` to catch
     missing adapter reports, adapter drift, and adapters outside the saved
     execution context before reflection text is built.
     Prefer `runtime_adapter_execution_commit_signal_component_count()`,
     `runtime_adapter_execution_commit_blocker_component_count()`,
     `runtime_adapter_execution_commit_accounting_is_consistent()`, and
     `can_commit_runtime_adapter_execution()` before expanding verbose adapter
     diagnostics. Runtime adapter signal counts stay observability, not
     response blockers.
   - Convert root runtime/draft tokens into `GeneratedToken` and compare root
     `RuntimeTokenMetrics` with `GeneratedTokenMetrics::from_tokens(...)`.
     Use `uncertainty_coverage_signal_component_count()`,
     `has_uncertainty_coverage_signals()`,
     `uncertainty_metric_problem_component_count()`, and
     `uncertainty_accounting_is_consistent()` before expanding entropy/logprob
     payload diffs. Gate compact parity with `uncertainty_shape_is_clean()` and
     `can_use_token_uncertainty_metrics()`.
   - Convert root `RuntimeResponse` into `InferenceOutcome`, then build
     `RuntimeResponseEnvelope` after JSON parsing and before reflection/memory
     mutation.
   - Compare `RuntimeResponseEnvelope::envelope_summary()` before verbose
     response checks so answer/token shape, uncertainty signal,
     uncertainty coverage/problem/accounting, runtime execution signal, and
     imported/exported diagnostics KV parity are visible without parsing
     strings.
   - Compare `RuntimeResponseEnvelope::request_parity_summary(...)` before
     verbose request-parity violations so token limits, KV counts, adapter
     parity, generation budget, route budget, hardware pressure, and optional
     planning headroom/latency checks remain structured.
   - Classify response/request parity drift with
     `token_drift_component_count()`, `kv_drift_component_count()`,
     `adapter_drift_component_count()`, `diagnostics_drift_component_count()`,
     and `request_drift_component_count()` before expanding verbose response
     parity violations. Use the focused helpers for missing adapter reports,
     generation-budget absence, route/hardware drift, and planning-bound
     token/KV/headroom/latency drift.
   - Prefer the split response parity counts for root routing:
     `request_token_drift_component_count()`,
     `planning_token_drift_component_count()`,
     `request_kv_drift_component_count()`,
     `runtime_kv_drift_component_count()`,
     `planning_kv_drift_component_count()`,
     `request_diagnostics_drift_component_count()`, and
     `planning_diagnostics_drift_component_count()` before verbose
     request-parity text.
   - Use `RuntimeResponseRequestParitySummary::planned_kv_summary()` when the
     root response adapter needs a single planned KV response decision. Gate
     planning-attached KV side effects with
     `can_commit_planned_kv_response()` and check
     `response_kv_matches_planned_zero_export()` before exporting KV from
     short requests whose planning digest chose `export_blocks = 0`.
   - When the request path has already checked
     `RuntimeRequestEnvelope::manifest_request_planning_readiness_summary(...)`,
     pass that same `RuntimePlanningManifestKvBridgeSummary` into
     `RuntimeResponseRequestParitySummary::manifest_kv_summary(...)` before
     response side effects. Require
     `can_commit_response_manifest_kv()` so response imported/exported KV counts
     stay inside the manifest-derived import/export plan, not only the attached
     planning digest.
     Prefer `RuntimeAcceptanceContext::response_manifest_kv_summary(...)` when
     the saved request/hardware context is available, so the response manifest
     KV gate uses the same request facts as response acceptance.
     Use `RuntimeAcceptanceContext::manifest_boundary_kv_summary(...)` when root
     wants the request manifest-planning gate and response manifest-KV gate in
     one value. Check `RuntimeManifestBoundaryKvStage`, `first_unready_stage()`,
     `first_blocking_stage()`, per-stage signal/blocker counts, aggregate
     accounting, and `can_commit_manifest_boundary_kv()` before exported-KV or
     memory side effects.
     Use `RuntimeAcceptanceContext::boundary_commit_readiness_summary(...)` as
     the normal non-manifest request/response boundary commit gate. Prefer
     `commit_summary()` when root needs `RuntimeBoundaryCommitAction`, primary
     failure summary, failure batch class counts, formatter readiness, and
     commit-decision accounting before returning a runtime failure or allowing
     side effects. Use `failure_return_summary()` as the adapter-facing
     projection when root only needs to decide whether the commit gate can be
     converted into a formatted runtime failure. Use
     `runtime_failure_return_report()` when root needs the primary
     `RuntimeFailureReport`, backend message, diagnostics note, or
     `InferenceError` conversion from the same commit gate.
     When root is ready for the final side-effect gate, prefer
     `RuntimeAcceptanceContext::manifest_boundary_commit_readiness_summary(...)`
     so the normal boundary commit readiness and manifest-boundary KV readiness
     are checked together. Use `RuntimeManifestBoundaryCommitStage`, aggregate
     signal/blocker counts, accounting, and
     `can_commit_runtime_manifest_boundary()` before exported-KV, reflection, or
     memory mutation. Use `RuntimeManifestBoundaryCommitProblemKind`,
     `first_problem_kind()`, and `problem_kind_component_count(...)` when root
     maps failures into boundary-commit, request manifest-planning, response
     manifest-KV, or accounting-drift diagnostics. Prefer `commit_summary()`
     when root needs `RuntimeManifestBoundaryCommitAction`, primary failure
     summary, failure batch, formatter readiness, and commit-decision accounting
     for the manifest boundary gate itself. The manifest boundary gate also
     carries the child `RuntimeBoundaryCommitAction` and
     `boundary_commit_action_matches_readiness()` so root can verify the nested
     boundary decision without re-reading the boundary counters. Its
     `failure_return_summary()` has the same `RuntimeFailureReturnSummary`
     shape as the normal boundary gate, and `runtime_failure_return_report()`
     has the same `RuntimeFailureReturnReport` materialization path.
     If root also has concrete KV import/export readiness summaries, wrap them
     with the manifest boundary commit gate in
     `RuntimeKvSideEffectReadinessSummary`. Check `RuntimeKvSideEffectStage`,
     first unready/blocking stage, per-stage signal/blocker counts, aggregate
     accounting, and `can_commit_runtime_kv_side_effects()` before committing
     import materialization, exported KV, reflection, or memory mutation. Use
     child import/manifest/export commit actions and
     `child_commit_actions_match_readiness()` to confirm nested side-effect
     decisions agree with the aggregate gate before root mutates state. Use
     `RuntimeKvSideEffectProblemKind`, `first_problem_kind()`, and
     `problem_kind_component_count(...)` to map failures without manually
     descending into import, manifest-boundary, and export summaries. Use
     `failure_report_for(...)`, `failure_reports()`, and
     `primary_failure_report()` when root wants `RuntimeFailureReport` values
     directly from the side-effect gate. Use `failure_report_count()`,
     `has_failure_reports()`, and `failure_batch_summary()` when root needs the
     same class-count summary shape used by request/response acceptance reports.
     Use `primary_failure_summary()` and `can_format_runtime_failures()` to keep
     side-effect failure formatting aligned with planning, manifest,
     diagnostics, request, and response reports. Use the side-effect commit
     summary's `failure_return_summary()` when root needs the same
     `RuntimeFailureReturnSummary` projection after import/manifest/export
     aggregation. Use `runtime_failure_return_report()` for the matching
     primary failure report and `InferenceError` conversion.
   - Use `planning_pre_request_gate_problem_component_count()`,
     `planning_pressure_signal_component_count()`,
     `response_wire_problem_component_count()`,
     `has_response_wire_problem_components()`, and
     `response_wire_accounting_is_consistent()` to keep invalid planning gates
     separate from route/FHT/KV pressure diagnostics before accepting runtime
     output. Use `response_wire_shape_is_clean()` and
     `can_use_response_wire()` as the compact parsed-response wire parity gate
     before reflection or memory mutation.
  - Run `RuntimeResponseEnvelope::response_gate_summary(...)` after
    `envelope_summary()` and `request_parity_summary(...)` and before
    reflection/memory mutation; only accept parsed runtime output when
    `RuntimeResponseGateSummary::can_commit_runtime_response()` is true. Use
    `runtime_response_commit_signal_component_count()`,
    `runtime_response_commit_blocker_component_count()`,
    `runtime_response_commit_accounting_is_consistent()`, and
    `can_commit_runtime_response()` before applying reflection, memory, or
    exported-KV side effects.
     Gate the envelope itself with
     `runtime_response_envelope_commit_signal_component_count()`,
     `runtime_response_envelope_commit_blocker_component_count()`,
     `runtime_response_envelope_commit_accounting_is_consistent()`, and
     `can_commit_runtime_response_envelope()` so schema, answer, token
     presence, KV/diagnostics parity, runtime execution signal, and
     uncertainty accounting are stable before response-wire checks. The legacy
     `response_envelope_shape_is_clean()` and
     `can_use_runtime_response_envelope()` helpers delegate to this commit
     surface.
  - After `envelope_summary()`, `request_parity_summary(...)`, and
    `response_gate_summary(...)` are available, build
    `RuntimeResponseReadinessSummary::new(...)` and require
    `can_commit_runtime_response_readiness()` before reflection, memory, or
    exported-KV side effects. Use `RuntimeResponseReadinessStage`,
    `first_unready_stage()`, `first_blocking_stage()`, per-stage
    signal/blocker counts, response-wire drift, response-gate blockers, and
    aggregate accounting before expanding verbose response violations.
    Prefer `RuntimeAcceptanceContext::response_readiness_summary(...)` when
    the saved request/hardware context is available so root does not rebuild
    envelope, request-parity, and response-gate summaries from separate facts.
  - Compare `RuntimeResponseGateSummary::is_clean_response_gate()` and
    `failure_report_matches_failures()` first, then use
    `response_gate_shape_is_clean()` to catch public summary accounting drift
    before reflection or memory mutation; if they fail, classify the drift with
    `has_response_contract_failures()`, `has_request_parity_failures()`, and
    `has_exported_kv_failures()`.
  - For the saved request/parsed response pair, prefer
    `RuntimeAcceptanceContext::boundary_envelope_summary(...)` with
    `runtime_boundary_envelope_commit_signal_component_count()`,
    `runtime_boundary_envelope_commit_blocker_component_count()`,
    `runtime_boundary_envelope_commit_accounting_is_consistent()`, and
    `can_commit_runtime_boundary_envelope()` before expanding request/response
    envelope diffs.
    Use `response_wire_problem_component_count()`,
    `direct_response_wire_problem_component_count()`,
    `planning_pre_request_gate_problem_component_count()`,
    `planning_pressure_signal_component_count()`,
    `exported_kv_activity_signal_component_count()`,
    `response_gate_signal_component_count()`, and
    `response_wire_accounting_is_consistent()` when root needs to separate
    direct response drift from planning pre-request blockers and non-blocking
    route/FHT/KV pressure signals. `response_gate_signal_component_count()`
    should feed observability and routing, while `response_blocker_component_count()`
    remains the accept-stop predicate.
    Use `acceptance_failure_component_count()`,
    `boundary_drift_component_count()`, and
    `response_blocker_component_count()` when root needs one parse-to-commit
    blocker count for response acceptance, envelope drift, request parity,
    exported KV validation, and mapped failure reports.
   - Compare `InferenceOutcome::outcome_summary()` before response-envelope
     validation so answer length, token count, uncertainty signal,
     imported/exported KV counts, route counts, diagnostics KV parity, runtime
     execution signal, and diagnostics note count are stable before payload
     diffs.
   - Prefer `has_complete_runtime_response_shape()`,
     `kv_count_drifted_from_diagnostics()`, `runtime_execution_missing()`,
     `text_without_tokens()`, and `tokens_without_text()` as compact parsed
     response shape checks before root expands token or KV payload diffs.
     Use `text_token_shape_problem_component_count()`,
     `kv_diagnostics_drift_component_count()`,
     `runtime_execution_signal_problem_component_count()`,
     `response_shape_problem_component_count()`, and
     `response_shape_accounting_is_consistent()` when root needs one
     pre-envelope shape gate. Gate parsed runtime outcomes with
     `response_shape_is_clean()` and `can_use_runtime_outcome()` before
     accepting side-effect candidates.
   - Gate the parsed runtime response with
     `RuntimeAcceptanceContext::response_acceptance_report(...)` before
     reflection/memory mutation so response token count, KV import/export count,
     selected adapter, generation budget, route budget, hardware pressure,
     compute headroom, latency budget, diagnostics, and exported KV payload
     limits cannot drift from the request/planning contract.
   - Prefer `RuntimeAcceptanceContext::response_gate_summary(...)` when a saved
     context is available so root reuses the same request and hardware facts for
     the structured parsed-response gate.
   - Prefer `RuntimeAcceptanceContext::response_acceptance_summary(...)` for
     the first response-boundary parity assertion when a saved context is
     available.
    - Convert response acceptance failures with
      `RuntimeResponseAcceptanceReport::failure_reports()`; response/request
      parity failures become `runtime_contract_violation`, while exported KV
      payload failures become `runtime_kv_export_error`.
    - Use `RuntimeResponseAcceptanceReport::failure_batch_summary()` and
      `primary_failure_summary()` when root needs failure class mix or the first
      mapped failure shape without collecting reports by hand.
    - Use `failure_return_summary()` and `runtime_failure_return_report()` on
      `RuntimeResponseAcceptanceReport` when root wants primary failure
      materialization before wiring the full boundary commit gate.
    - Compare `RuntimeResponseAcceptanceReport::acceptance_summary()` before
      verbose failure text so root tests can assert response-contract,
     request-parity, exported-KV failure classes, and accepted exported block
     count directly.
  - Prefer `RuntimeResponseAcceptanceSummary::is_clean_acceptance()`,
    `has_failures()`, and `failure_report_matches_failures()` for compact
    response-boundary assertions before mapping `RuntimeFailureReport`s.
    Use `response_contract_failure_component_count()`,
    `request_parity_failure_component_count()`,
    `exported_kv_failure_component_count()`, and
    `response_acceptance_problem_component_count()` when root tests need focused
    response acceptance class counts before verbose failure text. Gate adapter
    output with `response_acceptance_accounting_is_consistent()` so accepted
    state, violation totals, failure classes, mapped reports, and aggregate
    problem counts agree before reflection or memory mutation. Use
    `runtime_response_acceptance_commit_signal_component_count()`,
    `runtime_response_acceptance_commit_blocker_component_count()`,
    `runtime_response_acceptance_commit_accounting_is_consistent()`, and
    `can_commit_runtime_response_acceptance()` when the adapter only needs one
    response acceptance decision.
   - Compare `RuntimeAcceptanceContext::boundary_acceptance_summary(...)` when a
     root test has both the saved request context and parsed outcome; it combines
     request/response accepted state, violation counts, KV failure state, and
     failure-report counts before root expands either side's verbose reports.
  - Use `RuntimeBoundaryAcceptanceSummary::is_clean_acceptance()` and
    `failure_report_matches_parts()` before expanding either request or
    response acceptance details.
    Compare `request_acceptance_failure_component_count()`,
    `response_acceptance_failure_component_count()`,
    `kv_failure_component_count()`,
    `request_parity_failure_component_count()`, and
    `boundary_acceptance_problem_component_count()` when root needs focused
    saved-context acceptance counts before verbose reports. Use
    `boundary_acceptance_accounting_is_consistent()` before final commit gates
    so request/response nested accounting, total violations, total mapped
    reports, and accepted-state parity stay aligned. Prefer
    `runtime_boundary_acceptance_commit_signal_component_count()`,
    `runtime_boundary_acceptance_commit_blocker_component_count()`,
    `runtime_boundary_acceptance_commit_accounting_is_consistent()`, and
    `can_commit_runtime_boundary_acceptance()` as the compact saved-context
    acceptance gate before expanding envelope, adapter, or KV boundary
    summaries.
  - Compare `RuntimeAcceptanceContext::boundary_envelope_summary(...)` first
    when root only needs shape parity for request token limits, imported KV
    request matching, diagnostics KV parity, token uncertainty coverage and
    accounting, runtime execution signal, adapter candidates, and
    context-limited generation.
    Use `response_token_drift_component_count()`,
    `imported_kv_drift_component_count()`,
    `diagnostics_kv_drift_component_count()`,
    `runtime_execution_signal_missing_component_count()`,
    `request_adapter_signal_missing_component_count()`,
    `context_pressure_signal_component_count()`,
    `response_uncertainty_coverage_signal_component_count()`,
    `response_uncertainty_metric_problem_component_count()`,
    `response_uncertainty_accounting_is_consistent()`,
    `boundary_shape_drift_component_count()`, and
    `boundary_envelope_signal_component_count()` for focused saved-context
    envelope assertions before verbose request/response reports. Use
    `boundary_envelope_shape_is_clean()` and
    `can_use_runtime_boundary_envelope()` as the compact saved-context envelope
    gate.
   - Compare `RuntimeAcceptanceContext::boundary_adapter_summary(...)` before
     response acceptance when root has the saved request/hardware context; it
     keeps request selected adapter, runtime diagnostics adapter, allowed
     execution context membership, and optional planning selection parity in one
     report.
     Prefer `request_adapter_problem_component_count()`,
     `runtime_adapter_problem_component_count()`,
     `planning_selection_problem_component_count()`, and
     `adapter_boundary_problem_component_count()` before expanding verbose
     adapter diagnostics. Use `adapter_boundary_commit_signal_component_count()`,
     `adapter_boundary_commit_blocker_component_count()`,
     `adapter_boundary_commit_accounting_is_consistent()`, and
     `can_commit_runtime_boundary_adapter()` before committing
     adapter-specific side effects.
   - Compare `RuntimeAcceptanceContext::boundary_kv_summary(...)` before
     response acceptance when root needs KV parity; it keeps concrete imported
     blocks, response imported/exported counts, diagnostics counts, runtime
     import/export limits, planning exchange bounds, validation counts, and
     namespace counts together.
     Prefer `runtime_exchange_count_drift_component_count()`,
     `planning_bound_drift_component_count()`,
     `runtime_bound_drift_component_count()`,
     `namespace_drift_component_count()`,
     `validation_failure_component_count()`, and
     `kv_boundary_problem_component_count()` before expanding verbose KV
     payload diagnostics. Use `kv_boundary_shape_is_clean()` and
     `can_use_runtime_boundary_kv()` before accepting runtime KV exchange.
   - Compare `RuntimeAcceptanceContext::boundary_gate_summary(...)` as the
     final structured report before reflection or memory mutation; it combines
     request/response acceptance state with envelope, adapter, KV boundary
     consistency, request backend-wire problem counts, response-wire problem
     counts, request/response planning pre-request gate problems, and
     request/response planning pressure signals plus aggregate KV boundary
     signals. KV boundary signals and response uncertainty coverage remain
     observable activity signals; KV drift and response uncertainty
     metric/accounting drift contribute to commit blocking.
   - Prefer `RuntimeAcceptanceContext::boundary_commit_readiness_summary(...)`
     when root needs the complete adapter migration checklist in one value. It
     preserves request acceptance, response acceptance, boundary acceptance,
     envelope, adapter, KV, final gate, and runtime-response ready bits plus
     per-stage signal/blocker component counts. Use `first_unready_stage()`,
     `first_blocking_stage()`, and `RuntimeBoundaryCommitStage` to route a
     failed commit check to the first root adapter report that should be
     expanded.
   - Treat `RuntimeBoundaryGateSummary::boundary_gate_shape_is_clean()`,
     `can_commit_runtime_response()`, and `can_commit_runtime_boundary()` as
     the final commit checks; use `is_clean_commit_gate()` for the narrower
     commit-blocker state, and use `has_acceptance_failures()`,
     `has_boundary_drift()`, and `has_failure_reports()` to route failures to
     the right root adapter.
     Prefer `acceptance_failure_component_count()`,
     `boundary_drift_component_count()`,
     `request_backend_wire_problem_component_count()`,
     `direct_request_backend_wire_problem_component_count()`,
     `request_planning_pre_request_gate_problem_component_count()`,
     `request_planning_pressure_signal_component_count()`,
     `kv_boundary_signal_component_count()`,
     `response_wire_problem_component_count()`,
     `direct_response_wire_problem_component_count()`,
     `planning_pre_request_gate_problem_component_count()`,
     `planning_pressure_signal_component_count()`,
     `response_uncertainty_coverage_signal_component_count()`,
     `commit_gate_signal_component_count()`,
     `runtime_boundary_commit_signal_component_count()`,
     `runtime_boundary_commit_blocker_component_count()`,
     `runtime_boundary_commit_accounting_is_consistent()`,
     `response_uncertainty_metric_problem_component_count()`,
     `response_uncertainty_accounting_is_consistent()`,
     `total_wire_problem_component_count()`, and
     `commit_blocker_component_count()` before expanding request, response,
     adapter, or KV violation payloads. `has_wire_problem_components()` must be
     false, and `wire_accounting_is_consistent()`,
     `commit_gate_accounting_is_consistent()`, and
     `response_uncertainty_accounting_is_consistent()` must hold before root
     commits side effects. `commit_gate_signal_component_count()` should feed
     observability and routing, while `commit_blocker_component_count()`
     remains the commit-stop predicate.
    - Convert root `RuntimeError` sites in runtime generation, KV import, KV
      export, and contract handling into `RuntimeFailureReport` before building
      draft answers, trace labels, diagnostics notes, or `InferenceError`.
    - Compare `RuntimeFailureReport::failure_summary()` for each mapped failure
      before formatting root errors. Use the trace-label, recoverability,
      backend-error, diagnostics-note, and trace-confidence helpers plus
      `can_use_runtime_failure_report()` for single-failure adapter assertions.
    - Compare `RuntimeFailureReport::batch_summary(...)` after root collects
      mapped failures so class counts, recoverability, backend-error wrapping,
     diagnostics notes, and trace confidence stay structured before formatting
     individual errors.
   - Use `failure_counts_match_total()`, `has_runtime_failures()`,
     `has_kv_failures()`, `has_contract_failures()`,
     `diagnostics_notes_match_failures()`, `trace_confidence_is_valid()`,
     `failure_batch_problem_component_count()`, and
     `failure_batch_accounting_is_consistent()`,
     `failure_batch_shape_is_clean()`, and `can_format_runtime_failures()` to
     assert mapped failure batches before root formats user-facing errors.
   - Run `RuntimeDiagnostics::contract_violations(...)` before accepting runtime
     device execution claims or exported runtime KV blocks.
    - Run `RuntimeDiagnostics::hardware_contract_violations(...)` before trusting
      runtime-reported device profile, primary/fallback lane, or memory mode.
    - Compare `RuntimeDiagnostics::hardware_contract_summary(...)` before
      expanding hardware contract strings. Use the focused device-profile, lane,
      and memory-mode problem helpers plus
      `can_accept_runtime_hardware_contract()` when root adapter tests need
      structured device-execution classifications.
    - Prefer `RuntimeDiagnostics::hardware_acceptance_report(...)` at root
      runtime boundaries when hardware diagnostics failures need to become
      `RuntimeFailureReport`s.
   - Compare
     `RuntimeHardwareDiagnosticsReport::diagnostics_summary()` before root maps
     device execution failures into runtime errors or trace labels.
   - Use `RuntimeHardwareDiagnosticsReport::failure_batch_summary()` and
     `primary_failure_summary()` before formatting mapped hardware diagnostics
     failures.
   - Use `RuntimeHardwareDiagnosticsSummary::is_clean_acceptance()` and
     `failure_report_matches_violations()` before expanding hardware diagnostic
     violation strings.
     Prefer `hardware_failure_component_count()`,
     `mapped_failure_report_component_count()`,
     `hardware_acceptance_problem_component_count()`, and
     `hardware_acceptance_accounting_is_consistent()` when root adapter tests
     need device-execution acceptance details before mapping runtime errors.
     Use `hardware_acceptance_shape_is_clean()` or
     `can_accept_runtime_hardware_diagnostics()` when root only needs one
     compact hardware acceptance gate.
5. Convert hardware plans into core hardware contracts.
   - Map root `src/hardware::HardwareSnapshot` into
     `norion_core::HardwareLoadSnapshot`.
   - Map root `DeviceClass`, `DeviceTier`, `ComputeLane`, and
     `DeviceMemoryMode` by string or direct enum conversion.
   - Compare `DeviceClass::supported_profiles()`, `explicit_profiles()`, and
     `DeviceProfileDescriptor::descriptor_summary()` before hardware probe
     descriptor parity so device/tier/scope and alias-count drift is visible.
   - Compare `HardwareLoadSnapshot::snapshot_summary()` before hardware plan
     parity so normalized CPU/GPU/RAM/disk load, dominant load, tier, pressure,
     and pressure band are visible before root expands probe or gate reports.
     Gate malformed probe mappings with `snapshot_shape_is_clean()` and
     `can_use_hardware_snapshot()` plus `snapshot_accounting_is_consistent()`
     before allocator parity tests. Prefer
     `HardwareLoadSnapshotSummary::commit_summary()` when root needs
     `HardwareLoadSnapshotCommitAction`, mapped runtime failure reports,
     primary failure summary, failure batch, formatter readiness, and snapshot
     commit-decision accounting before constructing a hardware plan.
   - Use `HardwarePlan::adapter_execution_context()` as the canonical bridge
     into runtime adapter selection, routing context, diagnostics, and FHT-DKE
     KV prefetch planning.
   - Compare `AdapterExecutionContext::runtime_clamp_summary(...)` after
     hardware mapping and runtime metadata clamping so adapter count, pressure,
     latency, parallelism, KV prefetch, KV precision, token budgets, and disk
     spill remain visible before request planning. Use
     `adapter_execution_context_commit_*` /
     `can_commit_adapter_execution_context()` on the post-clamp context and
     `runtime_clamp_commit_*` / `can_commit_runtime_clamp()` on the clamp
     report. Prefer `AdapterExecutionContextSummary::commit_summary()` when
     root needs `AdapterExecutionContextCommitAction`, mapped runtime failure
     reports, primary failure summary, failure batch, formatter readiness, and
     commit-decision accounting for the derived or post-clamp context. Prefer
     `AdapterRuntimeClampSummary::commit_summary()` when root needs
     `AdapterRuntimeClampCommitAction`, mapped runtime failure reports, primary
     failure summary, failure batch, formatter readiness, and commit-decision
     accounting before request planning. Adapter commit summaries also expose
     `failure_return_summary()` and `runtime_failure_return_report()`, giving
     root one `AdapterFailureReturnSummary` / `AdapterFailureReturnReport`
     shape for adapter selection, runtime adapter execution, execution-context,
     and runtime-clamp failures.
   - Compare `AdapterExecutionContext::selection_runtime_summary(...)` after
     runtime response diagnostics report the selected adapter. Use
     `runtime_adapter_execution_commit_*` /
     `can_commit_runtime_adapter_execution()` for the parity gate, and prefer
     `commit_summary()` when root needs `AdapterSelectionRuntimeCommitAction`,
     mapped runtime failure reports, primary failure summary, failure batch,
     formatter readiness, and commit-decision accounting for missing, drifted,
     or disallowed runtime adapter reports. Use
     `runtime_failure_return_report()` when root needs the primary
     `RuntimeFailureReport`, backend message, diagnostics note, or
     `InferenceError` conversion for adapter parity failures.
  - Compare `DeviceExecutionPlan::execution_summary()` before full hardware
     plan parity so root tests can assert primary/fallback lane, memory mode,
     adapter hint count, parallel chunks, KV prefetch, precision, and disk-spill
     permission without parsing diagnostics strings.
   - Prefer focused execution signal/problem helpers before verbose execution
     diagnostics: adapter hints, execution capacity, primary/fallback lane,
     memory mode, KV precision, and constrained execution signals remain
     visible before row-level diffs. Use `execution_shape_is_clean()` and
     `can_use_device_execution_plan()` only for strict no-problem execution
     shape parity.
     observable, while missing adapter hints, missing capacity, and precision
     failures are problem components. Use
     `execution_shape_risk_component_count()` only when root needs the
     compatibility aggregate for missing hints/capacity, constrained lanes,
     disk-backed execution, compressed hot KV, or precision inversion. Use
     `hardware_execution_signal_component_count()` and
     `hardware_execution_blocker_component_count()` as the hardware execution
     adapter boundary, then require `can_commit_device_execution_plan()` before
     publishing the execution plan to later adapter selection.
     Prefer `DeviceExecutionPlanSummary::commit_summary()` when root needs
     `DeviceExecutionPlanCommitAction`, mapped runtime failure reports, primary
     failure summary, failure batch, formatter readiness, and execution-plan
     commit-decision accounting before constructing the adapter bridge.
   - Compare `DeviceExecutionPlan::adapter_hint_summary()` before gate-row CSV
     diffs so portable fallback, CPU, GPU, neural, multi-device, and custom
     adapter coverage are visible as scalar counts.
   - Use adapter family helpers for fallback count, accelerator count, family
     member count, adapter-count parity, family spread, fallback-only plans,
     accelerator-only plans, family accounting, and clean family shape before
     comparing adapter CSV rows. Use
     `adapter_family_commit_signal_component_count()`,
     `adapter_family_commit_blocker_component_count()`,
     `adapter_family_commit_accounting_is_consistent()`, and
     `can_commit_device_execution_adapters()` before committing mapped adapter
     hints to the bridge. Prefer `DeviceExecutionAdapterSummary::commit_summary()`
     when root needs `DeviceExecutionAdapterCommitAction`, mapped runtime
     failure reports, primary failure summary, failure batch, formatter
     readiness, and adapter-family commit-decision accounting.
   - Compare `HardwarePlan::plan_summary()` before replacing root device
     planning: pressure band, reduced parallel chunks, minimal KV prefetch,
     compressed hot KV, latency budget, disk-spill permission, and adapter count
     should match root diagnostics. Gate summary accounting with
     `hardware_plan_shape_is_clean()` and `can_use_hardware_plan()`; constraint
     signals remain observations, not automatic request blockers.
   - Prefer the focused `*_constraint_signal_component_count()` helpers, or the
     aggregate `plan_constraint_signal_component_count()`, before detailed
     device gate rows when root needs one constrained-plan count for pressure,
     parallelism, prefetch, precision, latency budget, disk spill, and notes.
     `plan_constraint_component_count()` is kept as a compatibility alias.
     Prefer `HardwarePlanSummary::commit_summary()` when root needs
     `HardwarePlanCommitAction`, mapped runtime failure reports, primary
     failure summary, failure batch, formatter readiness, and plan
     commit-decision accounting before deriving the execution sub-plan.
   - Compare `HardwarePlan::adapter_bridge_summary()` after mapping root
     hardware execution into core, so the mapped plan, execution plan, and
     derived `AdapterExecutionContext` preserve adapter counts, pressure,
     compute headroom, latency, parallelism, KV prefetch, precision, token
     budgets, and disk-spill state before runtime planning consumes them.
     Prefer `adapter_bridge_drift_component_count()` and the focused drift
     helpers for adapter count, pressure, latency, parallelism, KV prefetch,
     precision, token budget, and disk spill before verbose device ABI rows.
   - Build `HardwarePlan::runtime_readiness_summary(...)` after snapshot,
     plan, execution, and bridge summaries are available. Compare
     `HardwareRuntimeReadinessStage` order, `first_unready_stage()`,
     `first_blocking_stage()`, per-stage signal/blocker counts,
     `hardware_runtime_accounting_is_consistent()`, and
     `can_commit_hardware_runtime()` before runtime planning consumes the
     derived `AdapterExecutionContext`.
     Prefer `commit_summary()` when root needs one object for
     `HardwareRuntimeCommitAction`, mapped hardware contract failures, primary
     failure summary, failure batch, formatter readiness, and the final
     hardware-runtime commit decision.
   - Use core `ComputeLane` and `DeviceMemoryMode` string parsing for runtime
     diagnostics parity with root `src/runtime/contract.rs`.
   - Keep root probe, environment hints, and device gate reports in root code.
6. Convert runtime manifests into core digests.
   - Map root `TransformerRuntimeArchitecture` into
     `norion_core::TransformerRuntimeArchitecture`.
   - Map root `RuntimeKvPolicy` into `norion_core::RuntimeKvPolicy`.
   - Compare `TransformerRuntimeArchitecture::architecture_summary()` before
     full manifest ABI parity so layer count, hidden size, attention/KV heads,
     local window, and attention head dimension failures are isolated.
   - Compare `RuntimeKvPolicy::kv_policy_summary()` before runtime KV planning
     so import/export capability, block capacity, and limit/capability
     consistency are visible before root materializes KV exchange blocks.
   - Map root `RuntimeQuantizationPolicy` and `QuantizationBits` into
     `norion_core::RuntimeQuantizationPolicy` and `QuantizationBits`.
   - Compare `RuntimeQuantizationPolicy::quantization_summary()` before the
     full manifest ABI summary so hot/cold KV precision, optional weight
     precision, compressed KV state, and cold-not-wider-than-hot parity are
     visible as one focused report.
   - Keep root asset paths and production filesystem checks out of core.
   - Use `RuntimeManifestDigest::runtime_metadata()` when passing metadata into
     FHT-DKE, routing, runtime budget, and device ABI gates.
   - Use `RuntimeKvExportPlan::from_manifest(...)` when root turns manifest
     runtime-device ABI into transformer export planning. Compare
     `RuntimeKvExportPlan::manifest_plan_summary(...)` first so manifest export
     capability, runtime export capability, requested export count, plan max
     blocks, and raw architecture layer/KV-head shape are visible before
     forward payload or materialized block diffs.
   - Compare `RuntimeManifestDigest::abi_summary()` before request planning so
     root tests can assert effective context, transformer shape, KV limits,
     quantization widths, and supported adapter count without parsing summary
     strings. Prefer focused ABI signal/problem component helpers for context,
     transformer, KV exchange, quantization, and adapter catalog coverage before
     expanding field-level manifest diffs. Use
     `manifest_adapter_signal_component_count()` and
     `manifest_adapter_blocker_component_count()` as the root manifest adapter
     commit boundary, then require `can_commit_runtime_manifest_adapter()`
     before root treats the manifest ABI digest as the concrete planning
     source.
   - Run `RuntimeManifestDigest::validate()` before request planning; convert
     errors with `RuntimeManifestValidation::failure_reports()` and treat
     warnings as diagnostics.
     Prefer `RuntimeManifestValidation::failure_batch_summary()` and
     `primary_failure_summary()` when root needs mapped manifest failure class
     mix or the first failure shape before formatting root errors.
     Prefer `RuntimeManifestValidation::commit_summary()` when root needs one
     object for manifest acceptance, `RuntimeManifestValidationCommitAction`,
     mapped failure reports, primary failure summary, failure batch, and
     runtime-error formatting gates.
     Use `failure_return_summary()` before formatting root runtime errors; only
     invalid manifests should produce `runtime_failure_return_report()` with a
     primary failure report and `InferenceError` conversion.
   - Compare `RuntimeManifestValidation::validation_summary()` before verbose
     validation text so root tests can assert clean, warnings-only, and invalid
     ABI shapes without string matching.
     Prefer `is_clean_pass()`, `is_warnings_only_pass()`,
     `has_blocking_failures()`, and `failure_reports_match_errors()` before
     expanding validation messages.
     Use the `runtime_manifest_validation_commit_*` helpers as the first
     adapter-facing manifest validation gate before ABI summaries or
     runtime-device rows are consumed.
     Use `validation_activity_signal_component_count()` for non-blocking
     warning/report observability and `validation_problem_component_count()`,
     `warnings_only_flag_matches_shape()`, and
     `validation_accounting_is_consistent()` as the compact manifest-validation
     gate before request planning.
   - Compare `RuntimeManifestDigest::adapter_compatibility_summary(...)` after
     root maps hardware execution adapters and runtime adapter observations, so
     manifest-supported adapter count, hardware execution adapter count,
     compatible adapter count, compatible observations, and selected preferred
     adapter are visible before runtime planning consumes an adapter.
     Prefer `compatibility_counts_are_bounded()`,
     `compatible_adapter_fraction()`, `compatible_observation_fraction()`,
     `selected_adapter_has_source()`, `selected_adapter_is_usable()`,
     `adapter_source_problem_component_count()`, `adapter_source_problem()`,
     `runtime_manifest_adapter_compatibility_commit_signal_component_count()`,
     `runtime_manifest_adapter_compatibility_commit_blocker_component_count()`,
     `runtime_manifest_adapter_compatibility_commit_accounting_is_consistent()`,
     `runtime_manifest_adapter_compatibility_commit_is_clean()`, and
     `can_commit_runtime_manifest_adapter_compatibility()` before expanding
     adapter-source diagnostics.
   - Check `selected_adapter_available()`, `selected_from_observation()`, and
     `selected_from_fallback()` before root expands device-gate rows; a missing
     adapter intersection is a planning failure, while fallback selection is a
     visible but usable compatibility state.
   - Prefer `adapter_catalog_signal_component_count()`,
     `adapter_observation_signal_component_count()`,
     `adapter_selection_signal_component_count()`,
     `adapter_compatibility_signal_component_count()`,
     `adapter_catalog_problem_component_count()`,
     `adapter_selection_problem_component_count()`,
     `adapter_compatibility_problem_component_count()`, and
     `adapter_compatibility_accounting_is_consistent()` for new
     manifest/device ABI assertions. These helpers keep legal catalog,
     observation, fallback, and selection activity separate from missing
     catalogs, missing execution hints, disjoint adapter sets, bounded-count
     drift, unavailable selection, and selected-source drift.
   - Use `observation_selection_signal_component_count()` and
     `adapter_planning_signal_component_count()` when root needs one scalar that
     includes rejected observations, fallback selection, and the legacy
     adapter-source problem total. Keep that scalar in diagnostics; use
     `runtime_manifest_adapter_compatibility_commit_blocker_component_count()`
     for blocking gates.
   - Use `missing_supported_adapter_catalog()`,
     `missing_execution_adapter_hints()`, `adapter_sets_are_disjoint()`, and
     `can_plan_runtime_adapter()` as the compact manifest/device gate before
     root emits detailed hardware failure rows.
   - Compare `RuntimeManifestDigest::execution_compatibility_summary(...)`
     before root expands manifest/device KV gate failures so KV import/export
     capacities, execution KV prefetch, hot/cold precision coverage, disabled
     import requests, and the compact `can_use_execution_kv_contract()` state
     are visible without parsing failure strings. Use
     `execution_contract_shape_is_clean()` as the compact clean-shape gate,
     execution-contract signal helpers for manifest KV capacity, execution
     prefetch, and precision coverage, and problem helpers for disabled imports,
     limit drift, precision drift, and cold-wider-than-hot failures. Use
    `runtime_manifest_execution_device_commit_signal_component_count()`,
    `runtime_manifest_execution_device_commit_blocker_component_count()`,
    `runtime_manifest_execution_device_commit_accounting_is_consistent()`, and
    `runtime_manifest_execution_device_commit_is_clean()` as the
   runtime-device ABI mutation boundary, then require
   `can_commit_manifest_execution_device_gate()` before applying device-derived
   KV prefetch or precision rows. Keep `execution_device_*` helpers for focused
   diagnostics and legacy assertions.
   - Build `RuntimeDeviceHandoffReadinessSummary::new(...)` after hardware
     runtime readiness, runtime clamp readiness, and manifest execution-device
     compatibility are available. Compare `RuntimeDeviceHandoffStage` order,
     `first_unready_stage()`, `first_blocking_stage()`, per-stage
     signal/blocker counts, `runtime_device_handoff_accounting_is_consistent()`,
     and `can_commit_runtime_device_handoff()` before request planning consumes
     the derived or clamped `AdapterExecutionContext`.
     Prefer `commit_summary()` when root needs one object for
     `RuntimeDeviceHandoffCommitAction`, mapped handoff contract failures,
     primary failure summary, failure batch, formatter readiness, and the final
     device-handoff commit decision.
     Use the same `failure_return_summary()` /
     `runtime_failure_return_report()` projection as manifest validation when
     root needs to convert a failed handoff gate into a root runtime error.
   - Compare `kv_prefetch_overflow_blocks()`,
     `hot_precision_overflow_bits()`, `cold_precision_overflow_bits()`,
     `manifest_kv_capacity_missing()`, and
     `execution_kv_contract_failure()` before root expands the concrete device
     ABI failure rows.
     Prefer `kv_capacity_problem_component_count()`,
     `kv_prefetch_problem_component_count()`,
     `precision_problem_component_count()`, and
     `execution_contract_problem_component_count()` for first-pass
     runtime-device ABI assertions.
7. Introduce core routing as an observation path.
   - Compare `HierarchyWeights::summary()` after mapping root
     `convolution` to core `fusion` so normalized state, total weight, and
     dominant focus are stable before routing consumes hierarchy weights.
   - Compare `ProfileHierarchyWeights::summary()` and
     `ProfileHierarchyObservations::{total, active_profile_count}` before
     replacing root hierarchy controller state storage.
   - Use `active_weight_signal_component_count()`,
     `focus_signal_component_count()`, `normalization_signal_component_count()`,
     `hierarchy_signal_component_count()`,
     `weight_shape_problem_component_count()`,
     `focus_problem_component_count()`,
     `normalization_problem_component_count()`,
     `hierarchy_problem_component_count()`, and
     `hierarchy_accounting_is_consistent()` as the compact root hierarchy
     parity gate before expanding per-weight diffs. Use
     `hierarchy_shape_is_clean()` and `can_use_hierarchy_weights()` before
     route or transformer planning consumes mapped hierarchy weights.
   - Use `normalized_profile_signal_component_count()`,
     `expected_focus_signal_component_count()`,
     `hierarchy_profile_signal_component_count()`,
     `per_profile_problem_component_count()`,
     `normalized_profile_problem_component_count()`,
     `expected_focus_problem_component_count()`,
     `hierarchy_profile_problem_component_count()`, and
     `hierarchy_profile_accounting_is_consistent()` before replacing profile
     hierarchy storage. Observation activity should use
     `profile_observation_signal_component_count()` and
     `profile_observation_accounting_is_consistent()` for diagnostics; it is
     not a blocker by itself. Use `hierarchy_profile_shape_is_clean()`,
     `can_use_profile_hierarchy_weights()`,
     `profile_observation_shape_is_clean()`, and
     `can_use_profile_hierarchy_observations()` as compact profile-state gates.
   - Build `RoutingContext` via `AdapterExecutionContext::routing_context(...)`.
   - Convert root `GenerationMetrics` with
     `GenerationMetrics::routing_feedback(profile)` before calling
     `HierarchicalRouter::observe(...)`.
   - Compare `RoutingFeedback::feedback_summary()` and
     `batch_summary(...)` before observation so profile distribution,
     quality/perplexity averages, low/high quality counts, and contradiction
     pressure are stable before threshold deltas are checked.
   - Round-trip root `RouterState` through core `RouterState` to verify
     threshold clamping and profile observation counts before replacing state
     storage.
   - Compare `RouterState::profile_observation_total()` and
     `has_observation_drift()` before replacing root state storage.
   - Compare current `src/router::NoironRouter` output against
     `DefaultHierarchicalRouter` in tests before replacing the root router.
   - Compare `RoutingDecisionSummary` before individual route diffs so layer
     counts, attention fraction, threshold crossings, and score ranges are
     visible when root router behavior drifts.
   - Prefer `layer_counts_match_tokens()`, `route_budget_matches(...)`,
     `has_fast_path()`, `has_attention_route()`, `all_attention_route()`,
     `uses_multiple_layers()`, and `has_score_spread()` as the compact route
     adapter checks before expanding token-level diffs.
   - Use `route_activity_signal_component_count()`,
     `route_layer_signal_component_count()`,
     `route_score_signal_component_count()`,
     `routing_signal_component_count()`,
     `route_count_problem_component_count()`,
     `route_score_problem_component_count()`,
     `routing_problem_component_count()`, and
     `routing_accounting_is_consistent()` as the first root router parity gate.
     Signals describe legal route activity and pressure; problem counts cover
     count drift, threshold partition drift, malformed score ranges, and
     invalid thresholds/fractions. Use `routing_shape_is_clean()` and
     `can_use_route_summary()` before accepting a non-empty route batch as the
     root-router baseline.
   - Check `RouteBudget::route_budget_signal_component_count()`,
     `route_budget_problem_component_count()`, and
     `route_budget_accounting_is_consistent()` before root expands concrete
     attention/fast-token budget diffs. Use `route_budget_shape_is_clean()`
     and `can_use_route_budget()` before feeding route pressure into attention,
     FHT-DKE, or transformer planning.
   - Build `RouteBudgetReadinessSummary::new(route_summary, route_budget)`
     before passing route pressure to attention, FHT-DKE, or transformer
     adapters. Use `stage_order()`, `first_unready_stage()`,
     `first_blocking_stage()`, per-stage signal/blocker counts, aggregate
     accounting, and `can_commit_route_budget_readiness()` to catch
     router-to-planning budget drift without expanding every token route.
   - Prefer `RouteBudgetReadinessSummary::commit_summary()` as the final root
     adapter branch before route pressure feeds attention, FHT-DKE, or
     transformer planning. `CommitRouteBudget` exposes the committed
     `RouteBudget`; `WaitForRouteBudget` means route facts are clean but absent
     or not yet usable; `RepairRouteBudget` means blocker counts,
     shape-accounting drift, or route-budget parity drift must be repaired
     before downstream planning consumes pressure.
   - Use `RoutingFeedbackSummary` and `RoutingFeedbackBatchSummary` focused
     signal/problem/accounting helpers to separate legal low-quality,
     high-quality, contradiction, mixed-profile, and quality-pressure signals
     from invalid quality/perplexity shape, profile-count drift, and
     quality-bucket drift before adaptive threshold assertions. Gate feedback
     mutation with `feedback_shape_is_clean()`,
     `feedback_batch_shape_is_clean()`, `can_use_routing_feedback()`, and
     `can_use_routing_feedback_batch()`.
   - Convert routed tokens into `AttentionCandidate` and compare
     `candidate_summary()` plus `batch_summary(...)` before attention policy
     selection so token position, score, entropy, layer mix, and attention
     candidate pressure are stable before selected/rejected diffs.
   - Use `candidate_signal_component_count()`,
     `candidate_problem_component_count()`,
     `candidate_batch_signal_component_count()`,
     `candidate_batch_problem_component_count()`, and
     `candidate_batch_accounting_is_consistent()` before expanding individual
     candidate rows. Signals cover legal token, layer, score, entropy, and
     candidate pressure; problems cover token shape, score/entropy shape,
     attention-layer drift, candidate-count drift, and attention-fraction drift.
     Gate compact parity with `candidate_shape_is_clean()`,
     `candidate_batch_shape_is_clean()`, `can_use_attention_candidate()`, and
     `can_use_attention_candidate_batch()` before accepting non-empty routed
     attention input.
   - Use `decision_signal_component_count()`,
     `decision_problem_component_count()`, and
     `decision_accounting_is_consistent()` before replacing root attention
     selection. Signals capture selected/rejected/cap and attention pressure;
     problems capture selected/rejected count drift, selected-over-cap drift,
     invalid thresholds, and invalid selection fraction. Use
     `decision_shape_is_clean()` and `can_use_attention_decision()` before
     replacing root attention selection.
   - Build `AttentionSelectionReadinessSummary::from_decision(...)` after root
     maps the candidate batch and selection result. Use `stage_order()`,
     `first_unready_stage()`, `first_blocking_stage()`, per-stage
     signal/blocker counts, candidate-to-decision layer/count parity helpers,
     aggregate accounting, and
     `can_commit_attention_selection_readiness()` before selected attention
     pressure flows into transformer or FHT-DKE planning.
8. Put recursive scheduling behind core summaries.
   - Convert root `src/recursive_scheduler::RecursiveSchedule` into
     `RecursiveScheduleDigest` or `RecursiveScheduleSummary`.
   - Compare root chunk, overlap, merge-round, execution-wave, and
     max-parallel fields with core `RecursiveSchedulerConfig` before replacing
     scheduling ownership.
   - Compare `RecursiveScheduleDigest::validation_summary()` before verbose
     schedule violations so shape, chunk, merge, and execution-wave failure
     classes, exact violation-count accounting, component accounting, aggregate
     problem presence, and clean validation readiness are stable.
   - Compare `is_single_pass()`, runtime-unit counts, recursion overhead,
     max execution-wave width, and parallelism summaries before moving runtime
     call diagnostics into core vocabulary.
   - Prefer `schedule_signal_component_count()`,
     `scheduler_shape_problem_component_count()`,
     `recursion_shape_problem_component_count()`,
     `execution_wave_problem_component_count()`, and
     `schedule_accounting_is_consistent()` before expanding full schedule rows.
     Use `schedule_shape_is_clean()` for compact shape parity and
     `can_use_recursive_schedule()` before root treats a non-empty schedule as
     executable recursive work.
   - Compare `RecursiveScheduleSummary::validation_summary(request_prompt_tokens)`
     before request-envelope attachment so prompt-token and execution-wave drift
     are visible without parsing violation strings. Gate validation with
     `validation_shape_is_clean()` and
     `can_accept_recursive_schedule_validation()` after checking focused
     failure counts.
   - Keep prompt chunk materialization and recursive execution in root.
9. Add a transformer digest adapter.
   - Convert root `src/transformer::TransformerRefactorPlan` into
     `TransformerPlanDigest`.
   - Map root `AttentionKind::ConvolutionalFusion` to core
     `TransformerAttentionKind::Fusion`.
   - Convert root `TransformerPlanner::plan(...)` inputs into
     `TransformerPlanningInput`.
   - Compare root planner output to `DefaultTransformerPlanner` for coding,
     writing, and long-document parity before replacing root decisions.
   - Compare `TransformerPlanDigest::plan_summary()` before per-layer diffs so
     layer mix, average compute, and window bounds are visible in one stable
     adapter assertion.
     Prefer `plan_mix_signal_component_count()`,
     `plan_pressure_signal_component_count()`,
     `plan_summary_problem_component_count()`, and
     `plan_summary_accounting_is_consistent()` before expanding layer rows.
     Use `plan_summary_shape_is_clean()` for compact shape parity and
     `can_use_transformer_plan()` before accepting a non-empty root planner
     baseline.
   - Compare `TransformerLayerBudget::layer_summary()` before full layer rows
     so layer index, attention label, fusion state, compute fraction, and
     window size are stable before verbose planner diffs.
     Use `layer_budget_signal_component_count()`,
     `layer_budget_problem_component_count()`, and
     `layer_budget_accounting_is_consistent()` for compact diagnostics. Gate
     adapter acceptance with `layer_budget_shape_is_clean()` and
     `can_use_transformer_layer_budget()`.
   - Prefer `TransformerPlanDigest::layer_batch_summary()` when root already
     has a full layer row set. Gate total/usable/unusable row parity with
     `layer_batch_accounting_is_consistent()`, `layer_batch_shape_is_clean()`,
     and `can_use_transformer_layer_budget_batch()` before verbose row diffs.
   - Build `TransformerPlanReadinessSummary::from_digest(route_budget, digest)`
     before root consumes the mapped plan. Use `stage_order()`,
     `first_unready_stage()`, `first_blocking_stage()`, per-stage
     signal/blocker counts, aggregate accounting, and
     `can_commit_transformer_plan_readiness()` to gate the
     route-budget-to-transformer-plan boundary without copying planner
     implementation into core.
   - Compare `TransformerPlanningPressureSummary` after route and attention
     summaries are available so root can gate route pressure, selected attention
     pressure, non-local/fusion layer mix, and average compute together before
     replacing planner ownership.
     Prefer route-token, attention-selection, transformer-mix, shape, delta,
     and accounting helpers before formatting route/attention/planner drift.
     Use `planning_pressure_shape_is_clean()` and `can_use_planning_pressure()`
     as the compact adapter-facing pressure gate.
   - Build `TransformerPlanningReadinessSummary::new(...)` from route-budget
     readiness, attention-selection readiness, and planning pressure before
     FHT-DKE or transformer planning consumes the pressure facts. Use
     `stage_order()`, `first_unready_stage()`, `first_blocking_stage()`,
     pressure-vs-route and pressure-vs-attention parity helpers, per-stage
     signal/blocker counts, aggregate accounting, and
     `can_commit_transformer_planning_readiness()` as the continuous
     router-to-attention-to-transformer adapter gate.
   - Use `TransformerPlanDigest::from_route_budget(...)` only as a fallback
     when root has no concrete layer plan yet.
   - Convert root `local_runtime::forward::LocalLayerSummary` and production
     forward summaries into `TransformerForwardSummary`, then compare
     `TransformerForwardBatchSummary::from_summaries(...)`,
     `RuntimeKvExportPlan::from_manifest(...)`,
     `RuntimeKvExportPlan::manifest_plan_summary(...)`,
     `RuntimeKvExportPlan::export_summary(...)`,
     `RuntimeKvExportPlan::planning_summary(...)`, `planned_block_count(...)`,
     `readiness_summary(...)`, `readiness_summary_for_blocks(...)`, and full
     `RuntimeKvExportPlan` output with current runtime KV export blocks before
     moving export planning.
  - Use `TransformerForwardBatchSummary` helpers for active-layer fraction,
    non-local/fusion fraction, window span, attention-count drift,
    active-count drift, forward batch signal/problem component counts, and
    `forward_batch_accounting_is_consistent()` before export payload checks.
    Use `forward_batch_shape_is_clean()` for compact shape parity and
    `can_use_forward_batch()` before accepting a non-empty forward batch as the
    runtime KV export baseline.
   - Use `RuntimeKvExportPlanningSummary::export_plan_limit_drift_blocks()`,
     `export_plan_exceeds_planning_limit()`, and
     `planned_export_overflow_blocks()` as the first adapter parity assertions
     when root detects export count drift.
   - Gate those drift checks with
     `export_boundary_problem_component_count()`,
     `has_export_boundary_problem_components()`, and
     `export_boundary_accounting_is_consistent()` so root can report a compact
     export-boundary mismatch before materialized block diffs. Use
     `export_boundary_shape_is_clean()` as the final compact boundary gate.
   - Use `RuntimeKvExportSummary::forward_batch_matches_summary_count()` and
     `planned_blocks_within_limit()` before comparing materialized exported
     `KvBlock` payloads.
   - Compare materialized export blocks with
     `RuntimeKvExportBlockSummary::from_blocks(...)` or
     `from_block_summaries(...)` after `build_blocks(...)`. Gate planned vs
     materialized count drift, runtime namespace drift, aggregate block-shape
     problems, `runtime_kv_export_block_commit_signal_component_count()`,
     `runtime_kv_export_block_commit_blocker_component_count()`,
     `runtime_kv_export_block_commit_accounting_is_consistent()`,
     `runtime_kv_export_block_commit_is_clean()`, and
     `can_commit_runtime_kv_export_blocks()` before root expands individual
     exported block payload diffs.
   - Prefer `RuntimeKvExportPlan::readiness_summary(...)` when core is allowed
     to derive export blocks from mapped forward summaries. Prefer
     `RuntimeKvExportPlan::readiness_summary_for_blocks(...)` after root has
     materialized/exported blocks and needs one validation gate before response
     state mutation. Use `RuntimeKvExportReadinessStage` order to route failures
     through forward batch, export payload, export planning, then materialized
      blocks. Compare `first_unready_stage()`, `first_blocking_stage()`,
      per-stage signal/blocker counts,
      `runtime_kv_export_readiness_accounting_is_consistent()`, and
      `can_commit_runtime_kv_export_readiness()` before applying any exported KV
     mutation. Treat clean no-op exports as parity success when no blocks are
     materialized. Prefer `commit_summary()` when root needs
     `RuntimeKvExportReadinessCommitAction`, mapped KV-export runtime failure
     reports, primary failure summary, failure batch, formatter readiness, and
     commit-decision accounting before mutating response KV state. Use
     `failure_return_summary()` as the pre-formatting gate and
     `runtime_failure_return_report()` only when the summary can return a
     runtime failure; this gives root the shared
     `RuntimeKvExchangeFailureReturnReport` shape before it adopts the full
     side-effect wrapper.
  - Summarize export payload mismatches with
     `forward_summary_count_drift_component_count()`,
     `planned_block_limit_overflow_component_count()`,
     `non_finite_forward_summary_component_count()`,
     `export_payload_problem_component_count()`,
     `has_export_payload_problem_components()`, and
     `export_payload_accounting_is_consistent()` before expanding individual
     runtime KV block differences. Gate materialization with
     `export_payload_shape_is_clean()` and
     `can_use_runtime_kv_export_payload()`.
   - Use export signal helpers for forward input, forward activity, emitted
     export blocks, empty-forward skips, and plan-limit hits so root can log
     normal export pressure separately from malformed payloads.
   - Gate export mutation with
     `runtime_kv_export_commit_signal_component_count()`,
     `runtime_kv_export_commit_blocker_component_count()`,
     `runtime_kv_export_commit_accounting_is_consistent()`,
     `runtime_kv_export_commit_is_clean()`, and
     `can_commit_runtime_kv_export()` after checking both payload and planning
     boundary problem counts.
10. Put tiered memory behind core placement contracts.
   - Convert root `MemoryEntry` into `TieredMemoryCandidate`.
   - Fold active root `MemoryMatch.similarity` into
     `TieredMemoryCandidate::with_active_similarity(...)`.
   - Compare `TieredMemoryCandidate::candidate_summary()` before scheduler
     planning so strength, reliability, attempts, failures, last score, and
     active similarity are stable before placement diffs.
     Prefer `candidate_signal_component_count()`,
     `candidate_problem_component_count()`,
     `candidate_accounting_is_consistent()`, `candidate_shape_is_clean()`, and
     `can_use_tiered_memory_candidate()` before expanding candidate fields.
   - Compare root `TieredCacheScheduler` output to `TieredMemoryScheduler`
     before replacing scheduling decisions.
   - Compare `TieredCachePlan::summary()` before individual placement rows so
     placement count, tier counts, hot/warm/cold fractions, multi-tier state,
     score range, and average score are stable.
   - Prefer `counts_match_placements()`, `has_score_spread()`, all-tier
     helpers, and `cold_dominates_hot()` before expanding placement rows.
   - Use `cache_distribution_signal_component_count()` for legal multi-tier,
     score-spread, and cold-pressure activity; use
     `cache_summary_problem_component_count()` and `cache_summary_is_clean()`
     to block malformed placement counts or score shapes before root mutates
     cache storage. Use `can_use_tiered_cache_summary()` before accepting a
     non-empty tiered placement baseline.
   - Use `cache_placement_signal_component_count()` and
     `cache_placement_blocker_component_count()` as the root placement
     mutation boundary, then require
     `tiered_cache_placement_commit_signal_component_count()`,
     `tiered_cache_placement_commit_blocker_component_count()`,
     `tiered_cache_placement_commit_accounting_is_consistent()`,
     `tiered_cache_placement_commit_is_clean()`, or
     `can_commit_tiered_cache_placement()` before writing placement rows back to
     root cache storage.
   - Compare `TieredCachePlan::migration_summary_from(...)` before applying
     promote, demote, new, retain, or evict side effects in root storage.
   - Use migration helpers for changed/retained total parity, action-count to
     migration-row parity, new-entry signals, tier movement, and capacity
     pressure before comparing individual migration rows.
   - Treat `migration_signal_component_count()` as visible persistence activity
     and `migration_boundary_problem_component_count()` as the mutation blocker;
     root should only apply cache movement when
     `tier_migration_commit_signal_component_count()`,
     `tier_migration_commit_blocker_component_count()`,
     `tier_migration_commit_accounting_is_consistent()`,
     `tier_migration_commit_is_clean()`, or `can_commit_tier_migration()` is
     clean.
   - Keep persistence, disk paths, and cache materialization in root code.
11. Put memory governance behind core preview/planning contracts.
   - Convert root `MemoryEntry` into `MemoryRecord`, preserving namespace, hits,
     failures, last score, timestamps, vector, and strength.
   - Compare `MemoryRecord::summary()` before full vector payload diffs so
     namespace, vector length, reliability, attempts, failure state,
     finite-value state, and age span are stable.
   - Convert root `MemoryRetentionPolicy` and `MemoryCompactionPolicy` into
     `MemoryGovernancePolicy`.
   - Compare root `RetentionReport` / `MemoryCompactionReport` with core
     `MemoryGovernanceReport` before replacing root cache mutation paths.
   - Compare `MemoryGovernanceReport::governance_summary()` before root applies
     removals, merges, or disk updates so retention, compaction, total removed,
     note, noop, and final-count parity are structured before id-level diffs.
   - Prefer governance helpers for retention/compaction count balance, pipeline
     balance, total-removed phase parity, note parity, and clean noop before
     expanding removed id lists.
   - Use `governance_signal_component_count()`,
     `has_governance_signals()`, `governance_problem_component_count()`,
     `has_governance_problem_components()`,
     `governance_accounting_is_consistent()`, and
     `governance_commit_is_clean()` or `can_commit_memory_governance()` as the
     compact root mutation gate.
     Retention/compaction activity is a signal; count, pipeline, removed-id, or
     note parity drift is the blocker.
   - Compare `MemoryUpdateReport::update_summary()` for single reinforce or
     penalize feedback, then use `update_signal_component_count()`,
     `update_problem_component_count()`, `update_accounting_is_consistent()`,
     `update_shape_is_clean()`, `memory_update_commit_signal_component_count()`,
     `memory_update_commit_blocker_component_count()`,
     `memory_update_commit_accounting_is_consistent()`,
     `memory_update_commit_is_clean()`, and `can_commit_memory_update()` before
     mutating an individual memory record. Wire this after
     `can_commit_runtime_boundary()` so parsed responses cannot update memory
     until the request/response boundary and the single-record update summary
     are both clean.
   - Compare `MemoryUpdateReport::batch_summary(...)` for batched feedback.
     Use `applied_missing_counts_match()`, `action_counts_match()`,
     `removed_count_within_applied()`, `delta_counts_within_applied()`,
     `update_batch_signal_component_count()`,
     `update_batch_problem_component_count()`,
     `update_batch_accounting_is_consistent()`, and
     `update_batch_commit_is_clean()`. Prefer
     `memory_update_batch_commit_signal_component_count()`,
     `memory_update_batch_commit_blocker_component_count()`,
     `memory_update_batch_commit_accounting_is_consistent()`,
     `memory_update_batch_commit_is_clean()`, and
     `can_commit_memory_update_batch()` as the root-facing batch mutation gate
     before applying batch cache side effects.
   - Keep root persistence, on-disk compaction, and actual cache mutation in
     root until migration tests are green.
12. Put KV quantization behind core codec contracts.
   - Convert root `src/kv_quant::QuantizedVector` into
     `norion_core::QuantizedVector`.
   - Convert runtime import/export KV blocks with `KvQuantizationPlan` so
     runtime namespace blocks use hot KV precision and non-runtime namespaces use
     cold KV precision.
   - Compare `QuantizedKvBlock::payload_summary()` before byte/string codec
     parity so namespace, selected bits, vector lengths, packed payload length,
     and compression ratio are stable.
  - Prefer `payload_shape_balanced()`, symmetric key/value helpers,
     `is_compressed()`, and `uses_expected_namespace_bits(...)` before
     expanding encoded byte/string diffs.
   - Use `quantized_payload_signal_component_count()` to classify runtime
     namespace, payload presence, and compression activity, and use
     `quantized_payload_problem_component_count(...)`,
     `quantized_payload_commit_signal_component_count()`,
     `quantized_payload_commit_blocker_component_count(...)`,
     `quantized_payload_commit_accounting_is_consistent(...)`, plus
     `quantized_payload_commit_is_clean(...)` or
     `can_commit_quantized_payload(...)` to block malformed codec payloads
     before persistence mutation.
   - Compare `QuantizedKvBlock::packed_payload_len()` and
     `compression_ratio()` before replacing root codec persistence paths.
   - Keep existing persistence files readable during migration by comparing root
     and core encodings in tests before replacing root decode paths.
13. Put KV exchange behind namespace-safe conversion.
   - Runtime KV imports use `KvNamespace::Runtime`.
   - Gist memory uses `KvNamespace::Gist`.
   - Agent-local KV uses `KvNamespace::Agent(agent_id)`.
   - Adapter-specific or experiment-specific blocks use
     `KvNamespace::Custom(adapter_or_experiment_id)`.
   - Do not fuse blocks unless namespace, layer, head, and token range match.
   - Convert root memory/Infini KV import candidates into `RuntimeKvCandidate`
     after root tier filtering.
   - Build importable runtime blocks with
     `RuntimeKvImportPlan::new(metadata, architecture, kv_prefetch_blocks)` so
     layer/head assignment, vector fitting, and import limits are parity-tested
     before replacing `src/runtime/kv_import.rs`.
   - Compare `RuntimeKvImportPlan::import_summary(...)` before block
     materialization so candidate count, non-empty candidate count, planned
     imports, import-limit hits, and embedding dimensions are visible.
   - Use `import_signal_component_count()` to classify enabled import activity,
     candidate presence, empty-candidate skips, import-limit pressure, and
     embedding-dimension availability as legal adapter signals.
   - Gate runtime KV block materialization with
     `import_shape_problem_component_count()`,
     `import_accounting_is_consistent()`, `import_shape_is_clean()`,
     `runtime_kv_import_commit_signal_component_count()`,
     `runtime_kv_import_commit_blocker_component_count()`,
     `runtime_kv_import_commit_accounting_is_consistent()`,
     `runtime_kv_import_commit_is_clean()`, and
     `can_commit_runtime_kv_import()` so enabled-capacity drift, candidate-count
     drift, planned-block overflow, limit-flag drift, disabled-import leftovers,
     and invalid embedding dimensions block root imports before payload
     construction.
   - Compare materialized import blocks with
     `RuntimeKvImportBlockSummary::from_blocks(...)` or
     `from_block_summaries(...)` after `RuntimeKvImportPlan::build_blocks(...)`.
     Gate planned vs materialized count drift, runtime namespace drift,
     aggregate block-shape problems,
     `runtime_kv_import_block_commit_signal_component_count()`,
     `runtime_kv_import_block_commit_blocker_component_count()`,
     `runtime_kv_import_block_commit_accounting_is_consistent()`,
     `runtime_kv_import_block_commit_is_clean()`, and
     `can_commit_runtime_kv_import_blocks()` before root expands individual
     imported block payload diffs.
    - Build `RuntimeKvImportReadinessSummary::new(...)` after import summary and
      materialized block summary are both available. Compare
     `RuntimeKvImportReadinessStage` order, `first_unready_stage()`,
     `first_blocking_stage()`, per-stage signal/blocker counts,
     `import_block_plan_matches()`,
      `runtime_kv_import_readiness_accounting_is_consistent()`, and
      `can_commit_runtime_kv_import_readiness()` before root applies imported KV
      blocks. No-op imports can be readiness-clean with zero materialized blocks;
      planned imports still require materialized runtime namespace blocks.
      Prefer `commit_summary()` when root needs
      `RuntimeKvImportReadinessCommitAction`, mapped KV-import runtime failure
      reports, primary failure summary, failure batch, formatter readiness, and
      commit-decision accounting before mutating runtime KV state.
   - Compare `KvBlock::shape_summary()` before payload-level diffs so namespace,
     runtime-exchange status, layer/head, token range, vector lengths, and
     finite-value state are stable adapter assertions.
   - Prefer `KvBlockShapeSummary::token_range_is_empty()`,
     `vector_len_delta()`, and `vectors_are_paired_and_finite()` before root
     expands individual KV payload validation messages.
   - Use `runtime_exchange_shape_problem_component_count()` and
     `runtime_exchange_shape_is_clean()` as the focused runtime KV block-shape
     gate before lower-level import/export validation expands payload errors.
     Use `can_use_runtime_exchange_block()` when root needs one block-level
     exchange decision.
   - Compare `KvNamespaceCounts::from_blocks(...)` before payload-level diffs
     so runtime, semantic, gist, agent, and custom block distribution is stable
     at import/export and fused-persistence boundaries.
    - Compare expected and actual distributions with
      `KvNamespaceCounts::drift_summary(...)`, then gate
     `runtime_count_drift_component_count()`,
     `semantic_count_drift_component_count()`,
     `gist_count_drift_component_count()`,
     `agent_count_drift_component_count()`,
     `custom_count_drift_component_count()`,
      `namespace_distribution_drift_component_count()`,
      `namespace_shape_signal_component_count()`, and
      `namespace_distribution_accounting_is_consistent()` before root expands
      individual KV block diffs. Use `namespace_distribution_shape_is_clean()`
      and `can_use_namespace_distribution()` as the compact distribution gate.
      Prefer `commit_summary()` when root needs
      `KvNamespaceCountDriftCommitAction`, mapped runtime failure reports,
      primary failure summary, failure batch, formatter readiness, and
      commit-decision accounting before imported/exported/fused KV mutation.
   - Treat `namespace_boundary_signal_component_count()` as legal namespace
     activity and `namespace_boundary_problem_component_count()` as the compact
     blocker before import/export or fused persistence mutation.
   - Use `only_runtime_exchange()`, `only_non_runtime_blocks()`,
     `has_runtime_and_non_runtime_blocks()`, and `runtime_fraction()` when root
     tests namespace separation at import/export and fusion boundaries.
   - Validate imported and exported runtime blocks with request-derived
     contracts through `RuntimeAcceptanceContext` before root turns validation
     reports into `RuntimeError`s. Use the lower-level envelope
     `acceptance_report(...)`, `validate_imported_kv_blocks(...)`, and
     `validate_exported_kv_blocks(...)` helpers for focused adapter parity
     tests.
    - Compare `RuntimeKvBlockContract::contract_summary()` before
      `validate_blocks(...)` when root is testing lower-level KV contracts
      directly. Gate max-block capacity, token-bound presence, direction label
      shape, `runtime_kv_block_contract_commit_signal_component_count()`,
      `runtime_kv_block_contract_commit_blocker_component_count()`,
      `runtime_kv_block_contract_commit_accounting_is_consistent()`,
      `runtime_kv_block_contract_commit_is_clean()`, and
      `can_commit_runtime_kv_block_contract()` before payload validation.
      Zero-capacity disabled export contracts may be clean but are not
      commit-ready.
    - Compare `RuntimeKvBlockContract::block_check_summary(...)` before parsing
      lower-level `validate_block(...)` strings. Assert namespace, layer/head,
      token-bound, vector-length, and finite-value problem counts as focused
      diagnostics, then gate the single-block payload with
      `runtime_kv_block_contract_check_commit_signal_component_count()`,
      `runtime_kv_block_contract_check_commit_blocker_component_count()`,
      `runtime_kv_block_contract_check_commit_accounting_is_consistent()`,
      `runtime_kv_block_contract_check_commit_is_clean()`, and
      `can_commit_runtime_kv_block_contract_check()`. Contract-check signals
      should feed diagnostics, while blockers decide whether that one
      imported/exported block can be accepted.
    - Compare `RuntimeKvValidationReport::validation_summary()` before root maps
      lower-level payload failures into runtime errors, so accepted block count
      and violation count are asserted without parsing validation text.
    - Compare `RuntimeKvBlockContract::validation_boundary_summary(...)` at the
      same import/export boundary so direction, failure trace label, limits,
      accepted/violation counts, and commit readiness are structured before root
      formats payload failures.
   - Use `RuntimeKvValidationSummary::rejected_all()` and
     `partially_accepted()` to keep all-or-some payload rejection checks
     structured.
   - Gate lower-level KV payload mutation with
     `runtime_kv_validation_commit_signal_component_count()`,
     `runtime_kv_validation_commit_blocker_component_count()`,
     `runtime_kv_validation_commit_accounting_is_consistent()`,
     `runtime_kv_validation_commit_is_clean()`, and
     `can_commit_runtime_kv_validation()` after checking accepted, partial,
     and rejected validation signals. Treat validation signals as diagnostics;
     only commit blockers should stop import/export side effects.
   - Gate root KV failure-trace mapping with
     `runtime_kv_boundary_commit_signal_component_count()`,
     `runtime_kv_boundary_commit_blocker_component_count()`,
     `runtime_kv_boundary_commit_accounting_is_consistent()`,
     `runtime_kv_boundary_commit_is_clean()`, and
     `can_commit_runtime_kv_boundary()` before formatting boundary violations.
   - Compare `KvFusionMerge::merge_summary()` before root applies fused KV
     persistence results so collapsed-block state, skip-limit state,
     runtime/non-runtime result counts, grouped namespace counts, and namespace
     mix are visible before payload diffs.
   - Prefer `is_noop()`, `block_accounting_balanced()`, and
     `namespace_counts_match_results()` as the first fused-persistence parity
     assertions before root compares individual retained or merged block
     payloads.
   - Use `result_namespace_boundary_signal_component_count()`,
     `fusion_commit_signal_component_count()`,
     `fusion_commit_blocker_component_count()`,
     `fusion_commit_accounting_is_consistent()`, and
     `can_commit_kv_fusion_persistence()` as the compact fused-persistence
     mutation gate. Prefer `KvFusionMergeSummary::commit_summary()` when root
     needs `KvFusionCommitAction`, mapped runtime failure reports, primary
     failure summary, failure batch, formatter readiness, empty-persistence
     problem detection, and commit-decision accounting before applying fused KV
     side effects.
   - When namespace distribution and fused persistence are both checked, pass
     their `failure_return_summary()` values into
     `RuntimeKvPersistenceFailureReturnSelection::from_summaries(...)` before
     formatting persistence failures. This keeps
     `kv_namespace_distribution -> kv_fusion_persistence` as the only accepted
     adapter order, lets namespace distribution drift win when both are
     returnable, and marks swapped source order as accounting repair.
   - Use `block_accounting_drift_component_count()`,
     `namespace_count_drift_component_count()`,
     `result_count_drift_component_count()`,
     `result_namespace_count_drift_component_count()`,
     `fusion_accounting_drift_component_count()`,
     `has_fusion_accounting_drift_components()`, and
     `fusion_accounting_is_consistent()` to separate true accounting drift from
     legal namespace-mix signals.
   - Use `merge_fraction_shape_is_valid()` and
     `merge_fraction_shape_problem_component_count()` before trusting compact
     merge-rate parity from root persistence reports.
   - Use `namespace_mix_signal_component_count()` and
     `runtime_namespace_mix_signal_component_count()` as adapter-visible
     classification signals, not automatic blockers.
   - Use `fusion_boundary_signal_component_count()`,
     `has_fusion_boundary_signals()`,
     `fusion_boundary_problem_component_count()`,
     `has_fusion_boundary_problem_components()`, and
     `fusion_boundary_is_consistent()` as the compact fused-persistence gate.
     Use `fusion_boundary_shape_is_clean()` and `can_use_kv_fusion_merge()`
     before accepting a non-empty fused KV merge as the persistence baseline.
     Merge, skip, and namespace-mix activity are signals; accounting or
     merge-fraction drift is the blocker.
   - Prefer `has_clean_accounting()`, `changed_due_to_merges()`,
     `changed_due_to_skips()`, `all_runtime_blocks()`, and
     `all_non_runtime_blocks()` for compact fused-persistence classification.
14. Enable FHT-DKE budget planning without changing the model kernel.
   - Build `FhtDkeInput` from prompt tokens, effective max generation tokens,
     core `RouteBudget`, runtime metadata, and `ExperimentSwitches`.
   - Compare `ExperimentSwitches::switches_summary()` before root enters
     runtime planning so enabled feature count, per-feature state,
     runtime-planning features, attention/KV features, and budget expansion are
     visible before UI labels or summary strings are formatted.
   - Use `enabled_labels()` and `summary()` for display diagnostics after the
     structured summary has been checked.
   - Treat `FhtDkeBudget::route_pressure` as the adapter-visible reason for
     changing dense/routed token split and routed KV exchange demand.
   - Compare `FhtDkeBudget::budget_summary()` or
     `RuntimePlanningDigest::fht_dke_summary()` before concrete KV block
     parity, including dense/routed fractions, token split validity, route
     pressure, and route-pressure-driven KV exchange counts.
   - Prefer `FhtDkeBudgetSummary::{has_route_pressure,
     route_pressure_is_high, has_routed_work, routed_tokens_per_kv_exchange_block}`
     for first-pass parity before root expands dense/routed token counts.
   - Gate invalid budget shapes with
     `token_split_invalid_component_count()`,
     `empty_budget_blocker_component_count()`,
     `kv_exchange_block_sum_drift_component_count()`,
     `kv_exchange_flag_drift_component_count()`,
     `budget_shape_problem_component_count()`,
     `has_budget_shape_problem_components()`, and
     `budget_shape_accounting_is_consistent()`. Use
     `budget_shape_is_clean()` and `can_use_fht_dke_budget()` before passing the
     non-empty budget into runtime planning.
   - Use `fht_dke_budget_commit_signal_component_count()`,
     `fht_dke_budget_commit_blocker_component_count()`,
     `fht_dke_budget_commit_accounting_is_consistent()`, and
     `can_commit_fht_dke_budget()` as the adapter commit gate for the FHT-DKE
     budget. Legal route pressure, routed work, KV exchange, and asymmetric
     import/export demand stay visible as signals; empty budgets and public
     budget-shape drift are blockers.
   - After route-budget readiness, attention-selection readiness, and
     `TransformerPlanningReadinessSummary` are available, build
     `FhtDkePlanningReadinessSummary::new(transformer_planning, budget_summary)`
     before handing the budget to runtime planning. Gate on
     `can_commit_fht_dke_planning_readiness()` so stale route pressure or
     attention-threshold drift cannot cross into KV budget planning.
   - Prefer `FhtDkePlanningReadinessSummary::commit_summary()` as the final
     adapter branch before runtime planning consumes the FHT-DKE budget.
     `CommitFhtDkePlanning` exposes the committed budget summary,
     `WaitForFhtDkePlanning` keeps clean unready planning out of failure
     formatting, and `RepairFhtDkePlanning` flags stale pressure, threshold
     drift, blocker counts, or public accounting drift before KV budget
     mutation.
   - Treat `route_pressure_signal_component_count()`,
     `high_route_pressure_signal_component_count()`,
     `routed_work_signal_component_count()`,
     `kv_exchange_signal_component_count()`,
     `kv_exchange_asymmetry_signal_component_count()`, and
     `budget_pressure_signal_component_count()` as adapter-visible
     classification signals, not automatic blockers.
   - Prefer `RuntimePlanningSummary::{route_pressure_is_active,
     route_pressure_is_high, fht_dke_limited_kv_prefetch,
     has_routed_kv_exchange}` at the request-planning gate so adapter tests can
     assert route-pressure and FHT-DKE KV clamp behavior without reaching into
     individual fields.
   - Keep the concrete FHT-DKE kernel behind `FhtDkeBudgeter` when it arrives.

## Main-Window Wiring Points

- Add a root adapter module that contains only conversion code and small tests.
- Thread prompt token count into the runtime backend before `max_tokens` is
  finalized.
- Use `RuntimeGenerationBudget::max_generated_tokens` as the generation limit
  passed to the current runtime request path.
- Build and check `RuntimeRequestEnvelope` before
  `runtime_request_json(...)`; keep root-owned toolsmith, agent-team,
  recursive, and hint arrays in the existing wire formatter.
- Compare `RuntimeRequestEnvelope::envelope_summary()` at the same boundary
  before expanding verbose violations.
- Attach `RuntimePlanningDigest` to `RuntimeRequestEnvelope` before
  `runtime_request_json(...)`; treat any max-token, adapter, generation-budget,
  actual KV import count, or KV prefetch mismatch as an adapter test failure.
- Validate imported runtime KV blocks from the same request envelope before
  `runtime_request_json(...)` through
  `RuntimeAcceptanceContext::from_request_parts(...).request_acceptance_report()`;
  over-limit block counts are parity failures, not successful truncation.
- Attach root recursive schedule summaries to `RuntimeRequestEnvelope` before
  `runtime_request_json(...)`; compare runtime-unit and parallelism summaries
  plus schedule signal/problem component counts while keeping recursive chunk
  execution in root.
- Build and check `RuntimeResponseEnvelope` after
  `parse_runtime_response_json(...)`; keep root-owned trace/reflection handling
  outside core.
- Compare `RuntimeResponseEnvelope::envelope_summary()` before response
  acceptance so token uncertainty and diagnostics KV drift are caught as field
  parity.
- Re-check `RuntimeResponseEnvelope` against the originating
  `RuntimeRequestEnvelope` after JSON parsing and before trace/reflection or
  memory feedback. Treat route budget, hardware pressure, compute headroom, and
  latency-budget drift as adapter parity failures.
- Validate exported runtime KV payloads with the originating request envelope
  after JSON parsing and before trace/reflection or memory feedback. Prefer
  `RuntimeAcceptanceContext::response_acceptance_report(...)` so diagnostics,
  request parity, and exported KV payload checks reuse the same request/hardware
  facts.
- Convert root generated token entropy/logprob fields into `GeneratedToken` and
  use `GeneratedTokenMetrics` for parity with existing runtime token metrics.
  Check uncertainty coverage and accounting helpers, then
  `uncertainty_shape_is_clean()` and `can_use_token_uncertainty_metrics()`,
  before verbose token-metric diffs.
- Compare `InferenceOutcome::outcome_summary()` after root maps the parsed
  runtime response and before reflection or memory mutation.
  Start with complete-response-shape, diagnostics KV drift, missing runtime
  execution, text-without-token, and token-without-text helpers before payload
  diffs, then require `response_shape_is_clean()` and
  `can_use_runtime_outcome()` before response acceptance and boundary gates.
- Convert `HardwarePlan.execution.adapter_hints` into
  `AdapterExecutionContext.adapters`; keep hardware probing outside core.
- Compare `HardwareLoadSnapshot::snapshot_summary()` immediately after mapping
  root hardware probes, before constructing or trusting the mapped core
  `HardwarePlan`. Use load-value, pressure-shape, and tier-shape problem counts
  to distinguish raw probe normalization bugs from pressure-band or device
  descriptor drift. Gate mapped probe input with `snapshot_shape_is_clean()`
  and `can_use_hardware_snapshot()`. Use
  `hardware_snapshot_commit_signal_component_count()`,
  `hardware_snapshot_commit_blocker_component_count()`,
  `hardware_snapshot_commit_accounting_is_consistent()`, and
  `can_commit_hardware_snapshot()` before committing normalized probe facts into
  root hardware planning. Prefer `HardwareLoadSnapshotSummary::commit_summary()`
  when root needs `HardwareLoadSnapshotCommitAction`, mapped runtime failure
  reports, primary failure summary, failure batch, formatter readiness, and
  snapshot commit-decision accounting before constructing a hardware plan.
- Prefer `HardwarePlan::adapter_execution_context()` once root hardware maps to
  core, so adapter selection and runtime diagnostics use the same execution
  facts.
- Compare `AdapterExecutionContext::runtime_clamp_summary(...)` before adapter
  selection when runtime metadata is available; use the embedded before/after
  context summaries for focused parity assertions. Treat clamp signal component
  counts as legal KV-prefetch/precision reductions, and clamp problem component
  counts as adapter execution ABI drift. Use
  `runtime_clamp_commit_signal_component_count()`,
  `runtime_clamp_commit_blocker_component_count()`,
  `runtime_clamp_commit_accounting_is_consistent()`, and
  `can_commit_runtime_clamp()` before writing the clamped context into the
  planning digest or backend request state.
- Compare `HardwarePlan::plan_summary()` before trusting a mapped hardware plan
  at runtime boundaries; use it as the first parity gate for pressure,
  parallelism, KV prefetch, precision, latency, disk spill, and adapter hints.
  Prefer `plan_constraint_signal_component_count()` and focused constraint
  signal helpers before verbose device gate rows; these classify hardware
  constraints without becoming automatic send blockers. Use
  `plan_shape_problem_component_count()`,
  `hardware_plan_commit_blocker_component_count()`,
  `hardware_plan_commit_accounting_is_consistent()`, and
  `can_commit_hardware_plan()` before committing a mapped plan to execution
  planning or the adapter bridge. Prefer `HardwarePlanSummary::commit_summary()`
  when root needs `HardwarePlanCommitAction`, mapped runtime failure reports,
  primary failure summary, failure batch, formatter readiness, and plan
  commit-decision accounting before deriving the execution sub-plan. Use
  `hardware_plan_shape_is_clean()` and `can_use_hardware_plan()` for compact
  plan-summary accounting parity.
- Compare `DeviceExecutionPlan::execution_summary()` when the root adapter only
  has the execution sub-plan available, especially before runtime diagnostics
  claim a device lane or memory mode.
- Prefer `DeviceExecutionPlanSummary` helpers for early execution-shape checks:
  `has_parallel_capacity()`, `has_kv_prefetch_capacity()`,
  `has_distinct_fallback_lane()`, `uses_cpu_primary_lane()`,
  `uses_gpu_or_accelerator()`, `uses_disk_streaming_lane()`,
  `uses_disk_backed_memory()`, `kv_precision_is_compressed()`, and
  `hot_and_cold_precision_match()`. Prefer
  `execution_shape_signal_component_count()`,
  `execution_shape_problem_component_count()`, and
  `execution_shape_accounting_is_consistent()` before expanding execution
  diagnostics. Use `execution_shape_is_clean()` and
  `can_use_device_execution_plan()` only for strict no-problem execution parity,
  and `execution_shape_risk_component_count()` when root needs one
  adapter-facing compatibility scalar. Use
  `hardware_execution_accounting_is_consistent()` and
  `can_commit_device_execution_plan()` before publishing the execution plan as
  hardware adapter input. Prefer `DeviceExecutionPlanSummary::commit_summary()`
  when root needs `DeviceExecutionPlanCommitAction`, mapped runtime failure
  reports, primary failure summary, failure batch, formatter readiness, and
  execution-plan commit-decision accounting before constructing the adapter
  bridge or full hardware runtime readiness summary.
- Use `DeviceExecutionPlan::adapter_hint_summary()` with
  `adapter_family_shape_is_clean()` and `can_use_adapter_family()` before
  accepting non-empty mapped adapter hints. Use
  `adapter_family_commit_signal_component_count()`,
  `adapter_family_commit_blocker_component_count()`,
  `adapter_family_commit_accounting_is_consistent()`, and
  `can_commit_device_execution_adapters()` before committing mapped adapter
  hints to the bridge. Prefer `DeviceExecutionAdapterSummary::commit_summary()`
  when root needs `DeviceExecutionAdapterCommitAction`, mapped runtime failure
  reports, primary failure summary, failure batch, formatter readiness, and
  adapter-family commit-decision accounting.
- Gate `HardwarePlan::adapter_bridge_summary()` with
  `adapter_bridge_shape_is_clean()` and `can_use_hardware_adapter_bridge()`
  before runtime planning consumes the derived adapter execution context. Use
  `adapter_bridge_preservation_signal_component_count()`,
  `hardware_adapter_bridge_blocker_component_count()`,
  `hardware_adapter_bridge_accounting_is_consistent()`, and
  `can_commit_hardware_adapter_bridge()` before committing the derived
  `AdapterExecutionContext`. Prefer `HardwareAdapterBridgeSummary::commit_summary()`
  when root needs `HardwareAdapterBridgeCommitAction`, mapped runtime failure
  reports, primary failure summary, failure batch, formatter readiness, and
  bridge commit-decision accounting before building the larger hardware runtime
  readiness summary.
- At the runtime request boundary, prefer
  `RuntimeAcceptanceContext::from_request_parts(...)` instead of hand-building
  the request envelope; this keeps hardware execution clamp, imported KV count,
  and saved response facts in one place.
- Use the saved context's request/response acceptance summary helpers before
  expanding verbose violations or mapped `RuntimeFailureReport`s. Compare the
  request/response acceptance accounting helpers at the same boundary so public
  summary field drift is caught before failure conversion.
- Use `boundary_envelope_summary(...)` when one saved context and one parsed
  response should expose request/response shape drift before acceptance
  summaries. Prefer its focused token, KV, diagnostics, runtime execution,
  adapter, context-pressure, uncertainty coverage, uncertainty accounting,
  shape-drift, and aggregate signal component counts before payload diffs.
- Build `RuntimePlanningDigest` before `RuntimeRequestEnvelope` so root can
  compare the current backend `max_tokens`, adapter choice, and KV prefetch
  count against core planning decisions. Use `backend_max_tokens()` as the
  backend generation cap and `planned_kv_exchange()` as the KV import/export
  comparison point.
- Compare the planning digest's `adapter_selection_report` at the same boundary
  so selected adapter, fallback reason, matched observation fraction, and empty
  allowed-adapter failures are visible before request JSON.
- Compare `RuntimePlanningDigest::kv_prefetch_clamp_summary()` when KV prefetch
  was clamped; root should attribute requested/runtime/planned import counts and
  runtime-metadata/FHT-DKE reductions before deciding which candidates to drop
  or defer. Keep `RuntimePlanningKvExchange::clamp_reason` as the compact
  reason code for request-envelope parity.
  Use clamp consistency helpers first so reduction totals, block-count deltas,
  clamp flags, and reason codes agree before root mutates the KV candidate
  vector.
- Run `RuntimePlanningDigest::acceptance_report()` before root creates the
  backend request; convert failures with `RuntimePlanningAcceptanceReport` so
  context exhaustion and malformed planning contracts use the same trace labels
  as later request/response acceptance gates.
  Prefer report-level `failure_batch_summary()` and `primary_failure_summary()`
  before formatting mapped planning errors.
- Run `RuntimeDiagnostics::hardware_contract_violations(...)` with the mapped
  `HardwarePlan` before accepting runtime-reported device execution details.
  Prefer `hardware_contract_summary(...)` first when root needs structured
  device-profile, lane, and memory-mode drift counts before verbose strings.
  Prefer `hardware_acceptance_report(...)` when mapping failures into root
  runtime errors or trace labels.
- Compare
  `RuntimeHardwareDiagnosticsReport::diagnostics_summary()` before expanding
  hardware diagnostic violation strings. Check hardware acceptance accounting
  and `can_accept_runtime_hardware_diagnostics()` before converting device
  execution failures into root runtime errors.
  Prefer report-level `failure_batch_summary()` and `primary_failure_summary()`
  before formatting mapped hardware diagnostics failures.
- Convert root `RuntimeAdapterObservation` into `AdapterObservation`, then call
  `AdapterExecutionContext::select_adapter(...)` after root filters experiences
  against the hardware plan.
- Prefer `AdapterExecutionContext::select_adapter_report(...)` for parity tests
  so allowed adapter count, matching observation count, rejected observations,
  matched fraction, selected-from-observation state, all-rejected observation
  state, and fallback reason are visible before root consumes only the chosen
  adapter.
- Convert root runtime/embedding diagnostics into `InferenceDiagnostics` and
  keep reflection reports as root-only user-facing artifacts for now.
- Compare `EmbeddingDiagnostics::diagnostics_summary()` before folding root
  embedding diagnostics into `InferenceDiagnostics`; gate summary field drift
  with embedding signal/problem counts, embedding accounting, and
  `can_use_embedding_diagnostics()`.
- Seed `InferenceDiagnostics` from `RuntimeAcceptanceContext` when possible, or
  from `RuntimeRequestEnvelope` otherwise, before adding root runtime/embedding
  diagnostics so response backchecks use the same route, generation, hardware,
  and planning fields as the request.
- Compare `InferenceDiagnostics::diagnostics_summary()` before nested runtime
  or embedding diagnostics diffs. Prefer complete diagnostics signal, route
  activity, runtime KV exchange, and runtime-or-embedding execution helpers as
  the first parity checks.
- Use `can_accept_inference_diagnostics_request_parity()` after
  `InferenceDiagnostics::request_parity_summary(saved_request)` when root needs
  one response-diagnostics parity gate.
- Seed missing `RuntimeDiagnostics` fields from `RuntimeRequestEnvelope` before
  response envelope construction; do not overwrite runtime-reported values.
- Compare `RuntimeDiagnostics::diagnostics_summary()` before verbose runtime
  diagnostics diffs at response boundaries. Prefer the focused runtime
  diagnostics signal/problem component counts when root tests need one compact
  adapter-facing shape report before expanding nested diagnostics text, and
  `can_use_runtime_diagnostics()` when the boundary needs a single gate.
- Compare `RuntimeDiagnostics::contract_summary(...)` before expanding
  `contract_violations(...)` so diagnostics model, architecture, adapter, and
  KV precision drift are classified without matching strings.
- Use `can_accept_runtime_diagnostics_request_parity()` after
  `RuntimeDiagnostics::request_parity_summary(saved_request)` for nested
  runtime diagnostics parity acceptance.
- Convert root runtime backend errors into `RuntimeFailureReport` so
  `runtime_error`, `runtime_kv_import_error`, `runtime_kv_export_error`,
  `runtime_contract_violation`, and context exhaustion labels are parity-tested.
- Compare each mapped failure with `RuntimeFailureReport::failure_summary()`
  before formatting root `RuntimeError`s or diagnostics notes.
- Convert request/response acceptance report failures with `failure_reports()`
  before root builds `RuntimeError`; use `primary_failure_report()` only when
  an existing call site can accept a single root error.
- Prefer report-level `failure_batch_summary()` and `primary_failure_summary()`
  for request/response acceptance reports and manifest validation before
  formatting mapped errors.
- Compare request/response `acceptance_summary()` values at the same gates so
  root tests can verify failure class counts and accepted KV block counts
  without parsing violation strings.
- Convert `RuntimeManifest` into `RuntimeManifestDigest` and preserve root-only
  production asset validation in root. Use `RuntimeManifestValidation` failure
  helpers so invalid core manifest shape becomes `runtime_contract_violation`;
  use `failure_batch_summary()` and `primary_failure_summary()` before turning
  those mapped failures into root runtime errors.
- Compare `RuntimeManifestDigest::abi_summary()` at the manifest adapter
  boundary before runtime planning, then use `runtime_metadata()` as the
  effective metadata passed to later budgeters. Use
  `abi_signal_component_count()`, `abi_problem_component_count()`, and
  `abi_accounting_is_consistent()` as the compact ABI shape gate before verbose
  manifest validation or runtime-device ABI rows.
- Convert `TransformerRefactorPlan.layers` into `TransformerPlanDigest.layers`
  for diagnostics and FHT-DKE planning.
- Convert root transformer planner inputs into `TransformerPlanningInput` and
  use `DefaultTransformerPlanner` parity tests before moving planner ownership.
- Convert root forward layer summaries into `TransformerForwardSummary` and use
  `TransformerForwardBatchSummary::from_summaries(...)`,
  `RuntimeKvExportPlan::from_manifest(...)`,
  `RuntimeKvExportPlan::manifest_plan_summary(...)`,
  `RuntimeKvExportPlan::export_summary(...)`, planned block count,
  `readiness_summary(...)`, `readiness_summary_for_blocks(...)`, and full block
  output for runtime KV export parity before replacing
  `local_runtime::forward::kv_export`. Prefer payload/boundary signal counts
  plus forward-batch signal/problem counts for normal export activity and
  `forward_batch_shape_is_clean()`, `can_use_forward_batch()`, and
  `export_payload_shape_is_clean()`, `export_boundary_shape_is_clean()`, and
  `runtime_kv_export_commit_is_clean()` before materializing exported runtime
  KV blocks. Then run `RuntimeKvExportBlockSummary::from_blocks(...)` and
  require `runtime_kv_export_block_commit_is_clean()` plus
  `can_commit_runtime_kv_export_blocks()` before applying those blocks to root
  response state; use `readiness_summary_for_blocks(...)` as the final compact
  gate when root already owns block materialization.
- Convert root runtime KV blocks to `KvBlock` only at import/export boundaries.
- Convert root `runtime_kv_blocks_from_context(...)` candidates into
  `RuntimeKvCandidate` and compare its output with `RuntimeKvImportPlan` before
  moving ownership of import planning. Require
  `runtime_kv_import_commit_is_clean()` before `build_blocks(...)`. After
  `build_blocks(...)`, run
  `RuntimeKvImportReadinessSummary::new(...)` and require
  `runtime_kv_import_readiness_accounting_is_consistent()` plus
  `can_commit_runtime_kv_import_readiness()` before applying imported blocks to
  root request state. When that readiness commit blocks, call
  `failure_return_summary()` before formatting root errors and use
  `runtime_failure_return_report()` for the primary `runtime_kv_import_error`.
  This mirrors the export readiness path with
  `RuntimeKvExchangeFailureReturnSummary`.
- Convert root `RuntimeKvBlock` into runtime-namespace `KvBlock`, then run
  `RuntimeAcceptanceContext` gates for imported/exported boundaries before
  accepting control-plane or kernel KV payloads.
- When testing lower-level contracts directly, compare
  `RuntimeKvBlockContract::contract_summary()` before `validate_blocks(...)` so
  zero-capacity, token-bound, and direction-label drift are visible before
  verbose payload validation. Use `runtime_kv_block_contract_commit_*` helpers
  as the adapter-facing four-piece gate before root accepts import/export
  contract capacity.
- For single imported/exported blocks, compare
  `RuntimeKvBlockContract::block_check_summary(...)` before
  `validate_block(...)`; use the focused namespace, layer/head, token, and vector
  problem helpers as adapter assertions instead of matching validation strings.
- Compare `KvBlock::shape_summary()` for imported/exported runtime blocks
  before comparing full vector payloads. Use runtime-exchange shape
  signal/problem helpers and `can_use_runtime_exchange_block()` before
  lower-level validation messages.
- Compare `KvNamespaceCounts::from_blocks(...)` before imported/exported/fused
  KV payload diffs when root needs namespace distribution parity. Use namespace
  boundary signal/problem helpers, namespace distribution commit
  signal/blocker helpers, and `commit_summary()` before applying
  imported/exported/fused KV mutation. Root adapters should return the mapped
  runtime failure batch when `KvNamespaceCountDriftCommitAction` is
  `ReturnRuntimeFailure`. Prefer `failure_return_summary()` before formatting
  root namespace errors, then materialize `runtime_failure_return_report()` for
  the shared `RuntimeKvPersistenceFailureReturnReport` shape.
- When namespace distribution and fusion persistence are evaluated together,
  prefer `RuntimeKvPersistenceFailureReturnSelection::from_summaries(...)` over
  a manual `FailureReturnRoutingSelection` route slice. It requires
  `kv_namespace_distribution -> kv_fusion_persistence`, lets count drift block
  storage mutation before fused-KV accounting errors are expanded, and treats
  swapped source order as adapter accounting repair.
- Compare `RuntimeKvValidationReport::validation_summary()` for focused
  import/export payload tests before mapping violations into root errors. Use
  runtime KV validation commit signal/blocker helpers,
  `runtime_kv_validation_commit_accounting_is_consistent()`, and
  `can_commit_runtime_kv_validation()` before accepting payload side effects.
- Compare `RuntimeKvBlockContract::validation_boundary_summary(...)` alongside
  the validation summary when root needs import/export failure trace-label
  parity and one compact KV boundary commit gate. Use
  `runtime_kv_boundary_commit_*` helpers for the adapter-facing four-piece
  gate; keep `boundary_*` helpers as focused diagnostics.
- Compare `ThresholdAttentionPolicy::policy_summary()` before and after root
  attention feedback so base/min/max thresholds, per-profile drift, bounded
  state, threshold spread, and adapted profile count are stable before moving
  attention policy ownership. Use
  `threshold_policy_signal_component_count()`,
  `threshold_policy_problem_component_count()`, and
  `threshold_policy_accounting_is_consistent()` before verbose policy diffs so
  non-finite thresholds, invalid learning rate, and out-of-bound policy shape
  are hard blockers while bounded/base/spread/adapted state remains
  observational. Gate compact policy parity with
  `threshold_policy_shape_is_clean()` and `can_use_threshold_policy()`.
- Compare `AttentionDecision::selected_count()`, `rejected_count()`,
  `selection_fraction()`, `hit_selection_cap()`, and
  `decision_summary()` before replacing root attention-token selection.
- Prefer attention summary helpers for candidate layer-count parity,
  all-attention candidate batches, selected/rejected accounting parity,
  selected/rejected attention fractions, rejected-attention pressure, and
  cap pressure before comparing individual selected/rejected candidates. Use
  candidate, candidate-batch, decision, and policy signal/problem/accounting
  helpers plus their clean/use gates as the first compact adapter gate.
- Compare `KvFusionMerge::merge_summary()` before replacing root KV fusion or
  persistence mutation. Use fusion commit signal/blocker helpers and
  `can_commit_kv_fusion_persistence()` before applying fused block changes to
  root storage. Prefer `KvFusionMergeSummary::commit_summary()` when root needs
  `KvFusionCommitAction`, mapped runtime failure reports, primary failure
  summary, failure batch, formatter readiness, empty-persistence problem
  detection, and commit-decision accounting before applying fused KV side
  effects. Use `failure_return_summary()` and
  `runtime_failure_return_report()` on that commit summary before root formats
  fused-persistence blockers.
- Convert tiered cache plans at the edge and keep root cache storage unchanged
  until all migration tests are green. Use `TierMigrationSummary` as the first
  migration parity check before comparing individual `TierMigration` rows.
  Compare `TieredMemoryCandidateSummary` signal/problem counts before scheduler
  planning, then compare `TieredCacheSummary` distribution signals and summary
  problem counts before root applies placement rows. Gate placement-row writes
  with `tiered_cache_placement_commit_is_clean()` or
  `can_commit_tiered_cache_placement()` after comparing placement signal and
  blocker counts plus tiered placement commit signal/blocker counts.
  Check action-count to migration-row parity, changed/retained total parity,
  tier movement, capacity pressure, migration signal counts, and boundary
  problem counts before row-level diffs. Apply root tier movement only after
  `cache_summary_is_clean()` and `tier_migration_commit_is_clean()` pass.
- Convert root memory retention/compaction policies into
  `MemoryGovernancePolicy` and apply core reports only after parity tests pass.
  Use `MemoryGovernanceReport` summary helpers for report-level parity before
  mutating root cache state. Gate mutation with
  `governance_commit_is_clean()` or `can_commit_memory_governance()` after
  checking governance signal/problem counts, note signals, commit
  signal/blocker counts, and commit accounting.
- Compare memory record and tiered cache summaries before applying root cache
  mutation or persistence updates.
- Compare `MemoryUpdateReport::update_summary()` for single reinforce/penalize
  calls, then `MemoryUpdateReport::batch_summary(...)` for batch cache feedback
  so applied/missing records, removed records, action mix, requested amount, and
  net strength delta are stable before root persistence side effects are
  checked.
  Use count parity, mixed-action, applied-removal, requested-amount shape,
  strength-delta shape, net direction, update/batch signal and problem counts,
  accounting helpers, `can_commit_memory_update()`, and
  `can_commit_memory_update_batch()` before expanding individual update rows.
- Quantize/dequantize KV blocks at runtime import/export boundaries through
  `KvQuantizationPlan` before moving root persistence codecs. Compare payload
  summary, length, and compression-ratio summaries before byte/string parity.
  Use payload shape parity, symmetric key/value shape, compressed state, and
  expected namespace bit helpers before byte/string diffs. Treat quantized
  payload signals as codec activity and require
  `quantized_payload_commit_signal_component_count()`,
  `quantized_payload_commit_blocker_component_count(...)`, and
  `quantized_payload_commit_is_clean(...)` before replacing persisted payloads.
- Keep root diagnostics as root types, but include core budget summaries in the
  existing diagnostic text.

## Tests To Add During Root Wiring

- root runtime request maps `max_tokens` through
  `InferenceRequest::generation_budget()`
- root runtime generation budget summaries agree on requested context overflow,
  requested generation deficit, hard context exhaustion, soft context limiting,
  aggregate context-budget signal presence, accounting consistency, public
  budget-shape problem counts, backend request signal/blocker counts, clean
  budget shape, backend max-token usability, and backend max-token commit
  readiness before backend request construction
- root runtime metadata shape summaries agree on context window, embedding
  dimensions, KV exchange support, block limits, hot/cold precision, metadata
  shape signal/problem counts, adapter missing/blocker counts, metadata-shape
  accounting, clean metadata shape, usable runtime metadata contract, and
  runtime metadata adapter commit readiness before max-token planning
- root runtime planning digest agrees with current backend context truncation,
  adapter selection report, route pressure, KV prefetch limits, and KV clamp
  reason
- root runtime planning KV clamp summary agrees on requested, runtime-clamped,
  and planned import counts plus runtime metadata reduction, FHT-DKE reduction,
  total reduction, compact clamp reason, reduction-count parity, block-count
  parity, unclamped/runtime-only/FHT-only/combined clamp state, and
  clean/usable clamp gates plus `kv_clamp_is_consistent()` before concrete KV
  candidate diffs
- root runtime planning summary agrees on generation budget, selected adapter,
  fallback reason, matched observations, FHT-DKE summary, KV exchange clamp, and
  hardware pressure before backend request construction
- root runtime planning summary agrees on adapter-selection blocker counts,
  missing/all-rejected observation signals, and total adapter-planning signal
  count before verbose planning diagnostics
- root runtime planning pre-request gate agrees on generation, parallelism,
  adapter, and FHT-DKE token-split readiness blockers, FHT-DKE budget
  commit signal/blocker counts, empty-budget blockers, budget-shape problems,
  KV clamp consistency problems, aggregate pre-request problem presence,
  request-readiness accounting, pre-request accounting, and pressure signal
  component counts before backend request serialization; use
  `pre_request_gate_shape_is_clean()`, `can_send_backend_request()`, and
  `can_commit_backend_request()` as compact adapter-facing send/commit gates so
  malformed public summary shape cannot slip through a narrower readiness check
- root runtime planning readiness tests compare FHT-DKE planning readiness,
  runtime pre-request commit, and FHT-DKE/runtime-boundary stages, including
  first unready/blocking stage, per-stage signal/blocker counts, route-pressure
  and attention-threshold drift from stale runtime summaries, aggregate
  accounting, and `can_commit_runtime_planning_readiness()` before backend
  request serialization or KV import mutation
- root runtime planning acceptance summary agrees on accepted state, context
  exhaustion, contract failure count, planning violation count, and mapped
  failure-report count, aggregate planning-acceptance problem count,
  accepted-state parity, failure-report parity, shape problem count, and
  accounting consistency plus clean/acceptable planning gates before backend
  request construction
- root runtime planning acceptance commit-decision tests assert
  `commit_summary()`, `RuntimePlanningAcceptanceCommitAction`, mapped failure
  reports, primary failure report/summary, failure batch, commit/failure
  booleans, formatter readiness, and
  `commit_decision_accounting_is_consistent()` for clean, context-exhausted,
  and contract-violating plans before backend request serialization
- root FHT-DKE budget summary agrees on dense/routed fractions, route pressure,
  token split validity, route-pressure-driven KV exchange counts,
  budget-shape problem component counts, aggregate shape problem presence,
  accounting consistency, pressure signal component counts, empty-budget
  blockers, budget commit signal/blocker counts, commit accounting, and clean
  FHT-DKE budget commit readiness before concrete KV parity
- root FHT-DKE planning readiness tests compare transformer-planning readiness,
  budget commit, and pressure-budget-boundary stages, including stage order,
  first unready/blocking stage, route-pressure and attention-threshold drift,
  per-stage signal/blocker counts, aggregate accounting, and
  `can_commit_fht_dke_planning_readiness()` before runtime planning or KV
  budget mutation consumes the budget
- root runtime request wire builds a `RuntimeRequestEnvelope` with
  `RUNTIME_REQUEST_SCHEMA` and reports architecture, adapter, context, and KV
  import-limit violations before JSON serialization
- root runtime request envelope summary agrees on context truncation, adapter
  presence, transformer/runtime layer parity, hardware pressure band, planning
  KV exchange, and recursive attachment before verbose violations
- root runtime request envelope exposes commit signal/blocker component counts,
  commit accounting parity, and `can_commit_runtime_request_envelope()` before
  JSON serialization
- root runtime request envelope reports planning-digest mismatches for backend
  `max_tokens`, selected adapter, generation budget, and KV prefetch/import
  count, plus backend-wire clean shape and usable backend-wire request state,
  before JSON serialization
- root runtime request gate summary agrees on clean send gate, backend-wire
  accounting, send-gate accounting, failure-report parity, clean request-gate
  shape, runtime request commit signal/blocker accounting, and runtime
  send/commit usability before JSON serialization
- root runtime request planning readiness tests compare
  `RuntimeRequestEnvelope::request_planning_readiness_summary(...)` against
  manual `RuntimeRequestPlanningReadinessSummary::new(...)` composition before
  backend JSON serialization
- root runtime manifest request planning readiness tests compare
  `RuntimeRequestEnvelope::manifest_request_planning_readiness_summary(...)`
  against manual `RuntimeRequestManifestPlanningReadinessSummary::new(...)`
  composition, including manifest KV bridge readiness, request planning
  readiness, stage order, first unready/blocking stage, aggregate signal/blocker
  accounting, and `can_commit_manifest_request_planning()`
- root request/response boundary tests can read
  `request_planning_readiness_summary(...)` from the saved context and compare
  runtime planning, request parity, request gate, first unready/blocking stage,
  per-stage signal/blocker counts, and
  `can_commit_runtime_request_planning()` before backend JSON serialization
- root runtime acceptance context reports actual imported KV block count drift
  from `planned_kv_exchange().import_blocks` before JSON serialization
- root runtime request acceptance summary agrees on request-contract failures,
  imported-KV failures, accepted imported block count, mapped failure reports,
  request acceptance accounting, clean acceptance shape, and final request
  acceptance before JSON serialization
- root recursive schedule and core `RecursiveSchedulerConfig` agree on token
  estimation, chunk ranges, overlap, merge rounds, execution waves, and
  parallel-wave regrouping, including summary signal/problem counts and
  schedule accounting, clean schedule shape, and recursive schedule usability
- root recursive runtime diagnostics agree with core single-pass,
  runtime-unit, recursion-overhead, and max-wave-width summaries
- root recursive validation summaries agree on valid state and shape/chunk/
  merge/execution-wave failure counts, exact violation-count accounting,
  component accounting, aggregate problem presence, clean validation shape, and
  validation acceptance before verbose violation text
- root runtime request envelope reports recursive prompt-token and execution
  wave mismatches before JSON serialization
- root runtime response wire builds a `RuntimeResponseEnvelope` with
  `RUNTIME_RESPONSE_SCHEMA` and reports answer, empty token, KV count, and
  runtime diagnostics violations before reflection/memory feedback
- root runtime response envelope summary agrees on answer length, generated
  token uncertainty, uncertainty coverage/problem/accounting,
  imported/exported KV counts, diagnostics KV parity, and runtime execution
  signal before verbose violations
- root runtime response envelope exposes commit signal/blocker component
  counts, commit accounting parity, and
  `can_commit_runtime_response_envelope()` before response-wire or memory
  mutation gates
- root runtime response envelope reports request-contract mismatches for
  generated-token cap, selected adapter, generation budget, route budget,
  hardware pressure, compute headroom, latency budget, and KV import/export count
  before reflection/memory feedback
- root response diagnostics can be seeded from `RuntimeAcceptanceContext` or
  `RuntimeRequestEnvelope` while preserving runtime-specific diagnostics
- inference diagnostics summaries agree on generation-budget truncation, route
  token counts, runtime KV exchange, runtime/embedding execution signals,
  hardware pressure band, recursive calls, note count, complete diagnostics
  signal, and route/runtime-KV activity
- embedding diagnostics summaries agree on query dimensions, memory-write shape,
  gist-write source counts, runtime/fallback call totals, source-mix signals,
  aggregate embedding problem counts, and embedding accounting before they are
  folded into inference diagnostics
- inference diagnostics request parity summaries agree on routing drift,
  missing/drifted generation budget, hardware pressure drift, planning
  headroom/latency drift, runtime diagnostics drift, missing required
  diagnostics reports, component drift counts, aggregate request drift presence,
  and diagnostics request accounting before verbose parity text
- runtime diagnostics request parity summaries agree on missing report,
  identity, architecture, KV, precision, and aggregate runtime drift component
  counts, presence, and accounting before verbose diagnostics parity text
- root runtime diagnostics can be seeded from `RuntimeRequestEnvelope` while
  preserving runtime-reported model, adapter, architecture, KV count, and
  precision mismatches
- root runtime backend errors map into `RuntimeFailureReport` with matching
  backend message, trace label, diagnostics note, confidence, and recoverability
- root runtime token metrics match `GeneratedTokenMetrics` for token count,
  entropy, negative logprob, uncertainty perplexity, coverage signals, malformed
  metric problem counts, and uncertainty accounting
- root inference outcome summaries agree on answer length, token count,
  uncertainty signal, route token counts, imported/exported KV counts,
  diagnostics KV parity, runtime execution signal, diagnostics note count,
  complete response shape, text/token drift, diagnostics KV count drift,
  response-shape problem counts, response-shape accounting, clean response
  shape, and runtime outcome usability
- root runtime failure batch summaries agree on class totals, class-count
  parity, recoverable/backend-error bounds, runtime/KV/contract failure
  categories, diagnostics-note parity, trace-confidence validity,
  zero-confidence trace state, aggregate failure-batch accounting, clean batch
  shape, and runtime-failure formatting readiness before root formats
  individual runtime errors
- known hardware adapter strings round trip into `RuntimeAdapter`
- known runtime diagnostic lane and memory-mode strings round trip into
  `ComputeLane` and `DeviceMemoryMode`
- root runtime diagnostics hardware checks agree with
  `RuntimeDiagnostics::hardware_contract_violations` for device, lane, and
  memory-mode mismatch or unknown-value failures
- root runtime diagnostics hardware acceptance report maps device execution
  mismatch or unknown-value failures to `runtime_contract_violation` and exposes
  report-level failure batch and primary failure summaries before formatting
- root runtime diagnostics hardware acceptance summary agrees on accepted
  state, hardware violation presence, mapped failure-report presence, aggregate
  problem count, and accounting consistency before root expands device
  execution violation strings
- root runtime diagnostics summaries agree on model/adapter presence,
  architecture signal, layer modes, device execution source, forward/KV signal,
  KV exchange count, and KV precision validity
- root runtime hardware diagnostics summaries agree on accepted state,
  violation count, and mapped failure-report count
- adapter observations select the best hardware-allowed adapter and fall back to
  the first execution adapter when no observation matches
- adapter execution context summaries preserve adapter count, pressure,
  latency, parallelism, KV prefetch, precision, token budgets, and disk-spill
  state before runtime planning
- adapter runtime clamp summaries preserve before/after context shape, runtime
  metadata limits, KV prefetch reduction, precision clamp state, and non-limit
  field preservation before runtime planning, including runtime clamp commit
  signals, blockers, accounting consistency, and commit readiness before root
  builds backend request state
- adapter selection reports preserve allowed adapter count, observation count,
  matching observation count, rejected observations, matched fraction, and
  fallback reason, including catalog/observation/fallback signal counts,
  matching-count and fallback-reason problem counts, accounting consistency,
  clean selection-report shape, and usable adapter selection before backend
  request construction
- runtime manifest adapter compatibility summaries agree on source-problem
  counts, observation/fallback planning signals, compatibility accounting,
  adapter compatibility commit signal/blocker counts, commit accounting, clean
  compatibility commit state, usable runtime adapter planning, and total
  adapter-planning signal counts before root expands adapter-source diagnostics
- high hardware pressure reduces route attention through `RoutingContext`
- root hardware snapshot summaries agree on normalized CPU/GPU/RAM/disk load,
  dominant load kind, pressure band, device tier, bounded load/pressure signals,
  pressure-band/tier drift problem counts, accounting consistency, clean
  snapshot shape, and hardware snapshot commit readiness before plan parity
- root hardware planning agrees with `HardwarePlanSummary` on pressure band,
  parallelism reduction, minimal KV prefetch, compressed hot KV, latency budget,
  disk spill, adapter count, focused plan constraint signals, aggregate
  constraint-signal count, plan public-shape blockers, plan-constraint
  accounting, and hardware plan commit readiness
- root hardware adapter bridge summaries agree that mapped hardware plan,
  execution plan, and derived `AdapterExecutionContext` preserve adapter counts,
  pressure, compute headroom, latency, parallelism, KV prefetch, precision,
  token budgets, disk-spill state, focused context signal/problem counts,
  runtime clamp signal/problem counts, focused bridge drift component counts,
  preservation signals, aggregate bridge drift presence, bridge blockers,
  bridge accounting consistency, and bridge commit readiness
- root hardware adapter bridge commit-decision tests assert
  `HardwareAdapterBridgeSummary::commit_summary()`,
  `HardwareAdapterBridgeCommitAction`, mapped contract failure reports, primary
  failure report/summary, failure batch, formatter readiness, focused bridge
  drift blockers, preservation signals, and
  `commit_decision_accounting_is_consistent()` before hardware runtime
  readiness or request planning consumes the derived `AdapterExecutionContext`
- root device execution planning agrees with `DeviceExecutionPlanSummary` on
  primary/fallback lane, memory mode, adapter hint count, parallel chunks, KV
  prefetch, precision, disk spill, focused execution signal/problem counts,
  hardware execution signal/blocker counts, aggregate risk count,
  execution-shape accounting, and hardware execution commit readiness
- root hardware runtime handoff tests compare `HardwareRuntimeReadinessSummary`
  stage order, first unready/blocking stage, per-stage signal/blocker counts,
  aggregate hardware runtime accounting, and `can_commit_hardware_runtime()`
  before request planning consumes the derived `AdapterExecutionContext`
- root hardware runtime commit-decision tests assert `commit_summary()`,
  `HardwareRuntimeCommitAction`, mapped contract failure reports, primary
  failure report/summary, failure batch, formatter readiness, and
  `commit_decision_accounting_is_consistent()` for clean hardware runtime,
  device-execution blockers, and adapter-bridge drift before the
  manifest/device handoff gate consumes the derived `AdapterExecutionContext`
- root hardware commit gates use `failure_return_summary()` before formatting
  root runtime errors. Clean snapshot, execution, adapter-family, plan, bridge,
  and runtime commits should not materialize a report; blocked commits should
  return `HardwareFailureReturnReport` with source label, primary failure,
  failure batch, backend message, diagnostics note, and `InferenceError`
  conversion.
- root device execution adapter family summaries agree on fallback count,
  accelerator count, family member count, adapter-count parity, family spread,
  family-only classifications, family signal/problem counts, accounting
  consistency, and clean family shape before adapter CSV parity
- root generation metrics convert to the same core routing feedback quality
  score used by `DefaultHierarchicalRouter::observe`
- root hierarchy adapters compare active-weight/focus/normalization signals,
  weight-shape/focus/normalization problems, profile normalized/focus drift,
  profile observation activity, and hierarchy accounting before router parity
  tests consume mapped hierarchy weights
- root route batches compare `RoutingDecisionSummary` before individual token
  decisions, including layer counts, threshold crossings, score ranges, and
  route-budget parity, then use focused route activity/layer/score signals,
  focused route count/score problems, route-budget signal/problem counts, and
  routing accounting for compact drift classification
- root route-budget readiness tests compare decision summary, downstream
  `RouteBudget`, and parity stages, including first unready/blocking stage,
  per-stage signal/blocker counts, aggregate accounting, clean-but-not-ready
  empty route behavior, and `can_commit_route_budget_readiness()` before route
  pressure reaches attention, FHT-DKE, or transformer planning
- root routing feedback batches agree on low/high quality, contradiction,
  mixed-profile, and quality-pressure signals, plus quality/perplexity shape,
  profile-count, quality-bucket, and feedback-batch accounting before adaptive
  threshold diffs
- root router state snapshots preserve threshold and per-profile observation
  counts after core restore/clamp, and report no total/per-profile observation
  drift
- KV import/export never crosses `Runtime`, `Gist`, `Agent`, or `Custom`
  namespaces
- root attention policy state parity preserves base/min/max thresholds,
  per-profile threshold drift, bounded state, threshold spread, and adapted
  profile count before and after observation, including focused policy
  signal/problem counts and accounting
- root attention selection parity preserves selected/rejected counts, selection
  fraction, cap-hit behavior, selected/rejected accounting parity, attention
  fractions, rejected-attention pressure, candidate signal/problem accounting,
  candidate-batch signal/problem accounting, and decision signal/problem
  accounting
- root attention selection readiness tests compare candidate batch, decision,
  and selection-boundary stages, including first unready/blocking stage,
  per-stage signal/blocker counts, stale candidate batch drift, empty
  clean-but-not-ready behavior, aggregate accounting, and
  `can_commit_attention_selection_readiness()` before downstream planning
- root KV fusion parity preserves merged count, merge fraction, skipped-limit
  status, changed/noop status, result namespace mix, grouped namespace counts,
  runtime/non-runtime block counts, focused accounting drift component counts,
  aggregate drift presence, clean accounting, legal namespace-mix signals,
  fusion boundary signal/problem counts, result namespace-boundary signals,
  fusion commit signal/blocker counts, boundary consistency, commit readiness,
  and merge/skip change causes
- root `runtime_kv_blocks_from_context(...)` and core `RuntimeKvImportPlan`
  agree on manifest-aware import plan bridge summary, import summary, import
  limit, layer/head assignment, token range, dimension fitting, runtime KV
  import commit signal/blocker accounting, weighted value vectors, materialized
  import block summary, runtime KV import block commit signal/blocker
  accounting, import readiness stage order, first unready/blocking stage,
  plan-vs-block count parity, no-op import behavior, aggregate import readiness
  accounting, and import readiness commit readiness
- root planning-to-manifest KV bridge tests compare
  `RuntimePlanningManifestKvBridgeSummary` before import/export materialization,
  including manifest import/export bridge cleanliness, planning import/export
  drift blocks, signal/problem component counts, accounting consistency, and
  `can_use_runtime_planning_manifest_kv_bridge()`
- root production/local runtime KV validation and core
  `RuntimeKvBlockContract` agree on namespace, layer/head, token bound,
  dimension, empty vector, finite-value failures, block-contract commit
  signal/blocker counts, block-contract commit accounting, zero-capacity
  clean-but-not-commit-ready state, block contract-check commit signal/blocker
  counts, contract-check commit accounting, and single-block commit readiness
- root lower-level runtime KV validation summaries agree on accepted block
  count, violation count, valid state, validation commit signal/blocker counts,
  validation commit accounting, clean validation commit state, boundary
  signal/blocker counts, boundary commit accounting, and boundary commit
  readiness before root error mapping
- root runtime KV block shape summaries agree on namespace, runtime-exchange
  status, layer/head, token range, vector lengths, empty-vector state, and
  finite-value state, including runtime-exchange signal/problem counts and
  clean shape readiness before payload diffs
- root KV namespace count summaries agree on runtime, semantic, gist, agent,
  custom, runtime/non-runtime mix, runtime fraction, expected-vs-actual
  distribution drift components, namespace shape signals, aggregate drift
  presence, boundary signal/problem counts, accounting consistency, and clean
  namespace boundary readiness before payload diffs or fused persistence
  mutation
- root KV persistence failure-return tests assert
  `RuntimeKvPersistenceFailureReturnSelection::from_summaries(...)`,
  canonical namespace-before-fusion source order, namespace drift winning when
  both persistence gates are returnable, clean continue behavior, swapped-source
  repair-accounting behavior, and `can_materialize_runtime_failure()` before
  root formats persistence failures
- root import/export KV validation derives limits from the saved
  `RuntimeRequestEnvelope`, and over-limit blocks are reported as violations
  before runtime JSON or memory feedback proceeds
- root request/response boundary tests use `RuntimeRequestAcceptanceReport` and
  `RuntimeResponseAcceptanceReport` to keep contract, request-parity,
  diagnostics, and KV payload failures visible in one gate
- root request/response boundary tests compare `acceptance_summary()` counts
  before checking verbose violation strings or mapped failure reports
- root request/response boundary tests classify request-contract,
  response-contract, request-parity, imported/exported-KV, mapped
  failure-report, and aggregate acceptance problem component counts before
  checking verbose violation strings
- root request gate tests compare clean send readiness, request-contract
  failure state, imported-KV failure state, and failure-report/count parity
  before backend JSON serialization
- root request gate tests classify acceptance, envelope/planning boundary
  drift, focused request-contract/imported-KV and envelope/planning blocker
  counts, mapped failure-report presence, total violation presence, aggregate
  send blocker counts, problem-component presence, send-gate accounting, and
  runtime request commit readiness before backend JSON serialization
- root request planning parity tests classify missing planning, max-token drift,
  generation-budget drift, adapter drift, imported-KV drift, KV-prefetch drift,
  planning-contract drift, planning pre-request gate problems, pressure-signal
  presence, backend-wire problem component counts, aggregate backend-wire
  problem presence, and accounting consistency before backend JSON
- root response gate tests compare clean response readiness, response-contract
  failure state, request-parity failure state, exported-KV failure state, and
  failure-report/count parity, clean response-gate shape, runtime response
  commit signal/blocker accounting, and runtime-response accept/commit
  readiness before reflection or memory mutation
- root runtime response readiness tests compare response envelope,
  response/request parity, and response gate stage order, first
  unready/blocking stage, per-stage signal/blocker counts, response-wire drift,
  response-gate blockers, aggregate accounting, and
  `can_commit_runtime_response_readiness()` before reflection or memory mutation
- root response gate tests classify acceptance, envelope/request-parity/
  exported-KV boundary drift, focused response/request/export failure and
  boundary blocker counts, mapped failure-report presence, total violation
  presence, aggregate response blocker counts, problem-component presence, and
  response-gate accounting consistency plus runtime response commit readiness
  before reflection or memory mutation
- root response parity tests classify token, KV, adapter, diagnostics, planning
  pre-request gate problems, pressure-signal presence, response-wire problem
  component counts, aggregate response-wire problem presence, accounting
  consistency, clean response-wire shape, and usable response-wire state before
  expanding request-parity violations
- root response parity tests classify request-token, planning-token,
  request/runtime/planning KV, request diagnostics, and planning diagnostics
  drift component counts before expanding request-parity violations
- root response parity tests can derive `RuntimeResponsePlannedKvSummary` from
  request parity and assert planned imported/exported KV limits, planned
  zero-export responses, aggregate planned-KV problem counts, and
  `can_commit_planned_kv_response()` before exported-KV side effects
- root response parity tests can derive `RuntimeResponseManifestKvSummary` from
  request parity plus the request-side manifest KV bridge and assert
  response-vs-manifest import/export coverage, manifest bridge blockers,
  planned-KV blockers, aggregate accounting, and
  `can_commit_response_manifest_kv()` before exported-KV or memory side effects
- root saved-context manifest KV tests can derive
  `RuntimeManifestBoundaryKvSummary` from one `RuntimeAcceptanceContext` and
  assert request manifest-planning readiness, response manifest-KV readiness,
  first unready/blocking stage, per-stage signal/blocker counts, aggregate
  accounting, and `can_commit_manifest_boundary_kv()` before final side effects
- root saved-context manifest boundary tests can derive
  `RuntimeManifestBoundaryCommitReadinessSummary` from one
  `RuntimeAcceptanceContext` and assert boundary commit readiness,
  manifest-boundary KV readiness, first unready/blocking stage, per-stage
  signal/blocker counts, aggregate accounting, and
  `can_commit_runtime_manifest_boundary()` before exported-KV, reflection, or
  memory mutation
- root saved-context manifest boundary tests can also assert
  `RuntimeManifestBoundaryCommitProblemKind` ordering, first problem kind, and
  per-kind problem counts so adapter failure mapping does not depend on raw
  internal counter fields
- root saved-context manifest boundary commit-decision tests can assert
  `commit_summary()`, `RuntimeManifestBoundaryCommitAction`, primary failure
  summary, failure batch, formatter readiness, and
  `commit_decision_accounting_is_consistent()` before root applies exported-KV,
  reflection, or memory mutation
- root KV side-effect tests can compose
  `RuntimeKvImportReadinessSummary`,
  `RuntimeManifestBoundaryCommitReadinessSummary`, and
  `RuntimeKvExportReadinessSummary` into
  `RuntimeKvSideEffectReadinessSummary`, then assert
  `RuntimeKvSideEffectStage` ordering, first unready/blocking stage,
  per-stage signal/blocker counts, child import/manifest/export commit actions,
  aggregate accounting, `child_commit_actions_match_readiness()`, and
  `can_commit_runtime_kv_side_effects()`
- root KV side-effect failure-mapping tests can assert
  `RuntimeKvSideEffectProblemKind` ordering, first problem kind, and per-kind
  problem counts so import, manifest-boundary, response manifest-KV, export,
  and accounting drift diagnostics stay stable across adapter wiring
- root KV side-effect failure-report tests can assert
  `primary_failure_report()` and `failure_reports()` map import blockers to
  `RuntimeFailureKind::KvImport`, export blockers to
  `RuntimeFailureKind::KvExport`, and manifest/boundary/accounting blockers to
  `RuntimeFailureKind::ContractViolation`
- root KV side-effect failure batch tests can assert `failure_report_count()`,
  `has_failure_reports()`, and `failure_batch_summary()` class counts before
  turning side-effect failures into root runtime errors
- root KV side-effect primary failure tests can assert
  `primary_failure_summary()` and `can_format_runtime_failures()` before root
  formats or logs side-effect runtime errors
- root KV side-effect commit-decision tests can assert `commit_summary()`,
  `RuntimeKvSideEffectCommitAction`, child import/manifest/export commit
  actions, first failing stage/problem kind, `primary_failure_summary`,
  `failure_batch`,
  `can_commit_runtime_kv_side_effects()`,
  `should_return_runtime_failure()`, and
  `commit_decision_accounting_is_consistent()` before the root adapter mutates
  imported/exported KV state or formats runtime errors
- root KV side-effect commit-decision tests can also read
  `primary_failure_report()`, `failure_report_for(...)`, and
  `failure_reports()` from the commit summary itself, keeping action selection
  and runtime error materialization on the same adapter-facing object
- root request/response boundary tests can save one `RuntimeAcceptanceContext`
  and reuse it across request JSON and response parsing gates
- root request/response boundary tests can read
  `response_readiness_summary(...)` from the saved context and compare stage
  order, first unready/blocking stage, per-stage signal/blocker counts, and
  `can_commit_runtime_response_readiness()` before side effects
- root request/response boundary tests can read request and response acceptance
  summaries directly from the saved context, including request/response
  acceptance accounting consistency
- root request/response boundary tests can compare
  `RuntimeBoundaryAcceptanceSummary` from one saved context before expanding
  either request or response failure details
- root request/response boundary tests classify saved-context request
  acceptance, response acceptance, KV, request-parity, mapped failure-report,
  and aggregate boundary acceptance problem component counts before expanding
  either side's verbose reports
- root request/response boundary tests can compare clean boundary acceptance,
  clean commit readiness, mapped failure-report presence, failure-report/count
  parity, and boundary acceptance accounting consistency before expanding
  either side's verbose reports
- root request/response boundary tests compare boundary acceptance commit
  signal/blocker counts, commit accounting parity, and
  `can_commit_runtime_boundary_acceptance()` before envelope/adapter/KV
  boundary gates
- root request/response boundary tests can compare
  `RuntimeBoundaryCommitReadinessSummary` from
  `boundary_commit_readiness_summary(...)` to verify the full migration order
  and per-stage signal/blocker accounting before final side effects
- root request/response boundary tests use `RuntimeBoundaryCommitStage`,
  `first_unready_stage()`, and `first_blocking_stage()` to choose the first
  verbose adapter report to expand after a failed readiness check
- root final commit gate tests compare request/response acceptance blockers,
  envelope/adapter/KV drift blockers, request backend-wire problem counts,
  response-wire problem counts, request/response planning pre-request blockers,
  pressure-signal presence, response uncertainty coverage signals,
  uncertainty metric/accounting drift blockers, mapped failure-report blockers,
  total violation presence, aggregate wire and commit blocker counts,
  problem-component presence, wire accounting, commit-gate accounting,
  runtime boundary commit signal/blocker counts, runtime boundary commit
  accounting, and final boundary commit readiness before reflection or memory
  mutation
- root boundary adapter tests compare request, runtime, planning-selection, and
  aggregate adapter problem component counts before expanding runtime adapter
  diagnostics
- root boundary KV tests compare runtime exchange count drift, planning-bound
  drift, runtime-bound drift, namespace drift, validation failure, and aggregate
  KV problem component counts before expanding KV payload diagnostics
- root boundary KV tests should also assert imported/exported/diagnostics KV
  activity, runtime KV capability, planning KV boundary, namespace activity, and
  aggregate KV boundary signal counts so clean runtime KV exchange is visible
  without being treated as a blocker
- root request/response boundary tests can compare
  `RuntimeBoundaryEnvelopeSummary` from one saved context before expanding
  request/response acceptance reports
- root request/response boundary tests classify saved-context token,
  imported-KV, diagnostics-KV, runtime execution, adapter signal, context
  pressure, uncertainty coverage/accounting, shape-drift, and aggregate
  envelope signal component counts before expanding request/response acceptance
  reports
- root request boundary tests cover
  `RuntimeAcceptanceContext::from_request_parts(...)` so hardware-derived KV
  prefetch is clamped by runtime metadata before request JSON
- root request/response acceptance failures map to `RuntimeFailureReport` with
  stable trace labels, diagnostics notes, confidence, and recoverability
- root local/production runtime KV export and core `RuntimeKvExportPlan` agree
  on manifest-aware export plan bridge summary, forward batch summary, export
  summary, planned block count, export limit, layer/head assignment, token
  range, vector splitting, and compute/activation scaling, including
  manifest/runtime export capability signal/problem counts, forward-batch
  signal/problem counts, payload/boundary signal counts, runtime KV export
  commit signal/blocker counts, materialized export block commit signal/blocker
  counts, accounting checks, and clean export commit readiness
- root local/production runtime KV export tests compare
  `RuntimeKvExportReadinessSummary` stage order, first unready/blocking stage,
  per-stage signal/blocker counts, no-op export handling, aggregate readiness
  accounting, `RuntimeKvExportReadinessCommitAction`, mapped KV-export runtime
  failure batch shape, and `can_commit_runtime_kv_export_readiness()` before
  expanding verbose forward, payload, planning, or materialized-block diffs
- root and core quantized vector encodings round trip the same representative
  4-bit and 8-bit payloads, including packed payload length and compression
  ratio summaries
- root and core quantized KV payload summaries agree on namespace, hot/cold bit
  selection, vector lengths, packed payload length, compression ratio, and
  empty-payload state, including key/value length parity, packed length parity,
  symmetric key/value shape, compressed state, payload signal counts, focused
  payload problem counts, quantized payload commit signal/blocker counts,
  accounting consistency, and clean codec commit readiness plus quantized
  payload commit usability
- root and core memory governance agree on stale removals, protected compaction
  ids, namespace isolation, retention-before-compaction ordering, removed-id
  summaries, noop detection, governance signal/problem counts, accounting
  consistency, note signals, governance commit signal/blocker counts, clean
  commit readiness, and governance commit usability
- root and core memory record summaries agree on namespace, vector length,
  reliability, attempts, feedback state, failure-heavy state, finite-value
  state, and age span
- root and core memory update summaries agree on applied/missing state,
  reinforce/penalize action counts, removed records, requested amount shape,
  strength-delta shape, update/batch signal and problem counts, accounting
  consistency, single-update and batch commit signal/blocker counts,
  single-update clean/commit gates, and batch feedback commit readiness
- root and core tiered cache summaries agree on placement count, tier counts,
  hot/warm/cold fractions, multi-tier state, count parity, score range, score
  spread, all-tier state, cold pressure, average score, distribution signal
  counts, placement signal counts, score/count problem counts, placement
  blocker counts, clean/usable cache summary gates, tiered placement commit
  signal/blocker counts, placement commit readiness, migration-row/action
  accounting, migration signal/problem counts, tier migration commit
  signal/blocker counts, and clean tier-mutation commit readiness
- root and core tiered candidate summaries agree on strength, reliability,
  attempts, failures, last score, active similarity, candidate signal/problem
  counts, candidate accounting, clean candidate shape, and candidate usability
  before scheduler placement parity
- root runtime manifest validation maps invalid core ABI shape to
  `runtime_contract_violation` while preserving warnings as non-blocking
  diagnostics
- root runtime manifest validation summaries agree on passed state, error
  count, warning count, warnings-only status, and failure-report count before
  request planning, including clean-pass, warnings-only-pass,
  blocking-failure, error/warning/mapped failure-report signal counts,
  aggregate validation signal presence, non-blocking validation activity
  signals, validation problem counts, warnings-only flag shape, commit
  signal/blocker counts, commit accounting, clean validation commit state,
  failure-report/error parity helpers, and final manifest validation acceptance
- root runtime manifest validation commit-decision tests assert
  `commit_summary()`, `RuntimeManifestValidationCommitAction`, mapped failure
  reports, primary failure report/summary, failure batch, commit/failure
  booleans, formatter readiness, and
  `commit_decision_accounting_is_consistent()` for clean, warnings-only, and
  invalid manifests before request planning consumes ABI rows
- root runtime manifest validation failure-return tests assert clean and
  warnings-only manifests do not materialize a return report, while invalid
  manifests return `ManifestFailureReturnReport` with source label, primary
  failure, failure batch, backend message, diagnostics note, and
  `InferenceError` conversion
- root runtime architecture summaries agree on layer/head/window shape,
  attention-head dimension validity, local-window/context fit, focused
  dimension/attention-geometry signal and problem counts, and architecture
  commit signal/blocker counts, commit accounting, and commit-ready architecture
  gates before full ABI summary checks
- root runtime KV policy summaries agree on import/export capability, block
  capacity, limit/capability consistency, focused capability/capacity signals,
  import/export drift problems, KV policy commit signal/blocker counts, commit
  accounting, and commit-ready KV policy gates before runtime KV exchange
  planning
- root runtime quantization policy summaries agree on hot/cold KV precision,
  optional weight precision, compressed KV state, and cold-not-wider-than-hot
  parity, focused supported-precision/compression/weight signals, invalid-width
  problem counts, quantization commit signal/blocker counts, commit accounting,
  and commit-ready quantization gates before full ABI summary checks
- root runtime manifest ABI summaries agree on effective context, embedding,
  architecture, KV exchange limits, hot/cold/weight quantization, and supported
  adapter count before request planning, including focused ABI signal/problem
  component counts, manifest adapter signal/blocker counts, ABI accounting,
  clean ABI shape, usable manifest ABI gates, and runtime manifest adapter
  commit readiness
- root runtime KV import tests compare
  `RuntimeKvImportPlan::manifest_plan_summary(...)` before block
  materialization, including manifest/runtime import capability signal/problem
  counts, requested prefetch, manifest/runtime/requested plan limits,
  embedding-dimension presence, architecture import shape, accounting
  consistency, clean manifest bridge shape, and
  `can_use_manifest_runtime_kv_import_plan()`
- root runtime manifest adapter compatibility summaries agree on supported
  adapter count, execution adapter count, compatible adapter count, compatible
  observations, selected preferred adapter, bounded compatibility counts,
  compatible adapter/observation fractions, selected source usability, and
  focused adapter catalog/observation/selection signal counts, focused adapter
  catalog/selection problem counts, aggregate compatibility problem state, and
  compatibility accounting, adapter compatibility commit signal/blocker counts,
  clean compatibility commit state, and usable runtime adapter planning before
  runtime planning
- root runtime manifest execution compatibility summaries agree on KV capacity,
  KV prefetch, precision, disabled import requests, hot/cold precision drift,
  cold-wider-than-hot inversion, focused and aggregate execution-contract
  signal/problem component counts, execution-device signal/blocker counts,
  runtime manifest execution-device commit signal/blocker counts, commit
  accounting, aggregate signal/problem presence, accounting consistency, clean
  execution-contract shape, and runtime-device commit readiness before verbose
  runtime-device ABI failure rows
- root runtime device handoff tests compare `RuntimeDeviceHandoffReadinessSummary`
  stage order, first unready/blocking stage, hardware/runtime-clamp/manifest
  per-stage signal and blocker counts, aggregate handoff accounting, and
  `can_commit_runtime_device_handoff()` before request planning consumes the
  derived or clamped `AdapterExecutionContext`
- root runtime device handoff commit-decision tests assert `commit_summary()`,
  `RuntimeDeviceHandoffCommitAction`, mapped contract failure reports, primary
  failure report/summary, failure batch, formatter readiness, and
  `commit_decision_accounting_is_consistent()` for clean handoff,
  manifest-execution blockers, and runtime-clamp blockers before request
  planning consumes the derived or clamped `AdapterExecutionContext`
- root runtime device handoff failure-return tests assert clean handoffs do not
  materialize a report, while blocked handoffs return
  `ManifestFailureReturnReport` with source label, primary failure, failure
  batch, backend message, diagnostics note, and `InferenceError` conversion
- root adapter execution context commit-decision tests assert
  `AdapterExecutionContextSummary::commit_summary()`,
  `AdapterExecutionContextCommitAction`, mapped contract failure reports,
  primary failure report/summary, failure batch, formatter readiness,
  adapter-count blockers, pressure-shape blockers, invalid parallelism,
  precision-shape blockers, `failure_return_summary()`,
  `runtime_failure_return_report()`, and
  `commit_decision_accounting_is_consistent()` before routing, adapter
  selection, or backend request construction consumes the mapped context
- root adapter selection commit-decision tests assert
  `AdapterSelectionReport::commit_summary()`, `AdapterSelectionCommitAction`,
  mapped contract failure reports, primary failure report/summary, failure
  batch, formatter readiness, missing-allowed-adapter blockers, and
  `failure_return_summary()`, `runtime_failure_return_report()`, and
  `commit_decision_accounting_is_consistent()` before backend request
  construction consumes the selected adapter
- root adapter runtime-clamp commit-decision tests assert
  `AdapterRuntimeClampSummary::commit_summary()`,
  `AdapterRuntimeClampCommitAction`, mapped contract failure reports, primary
  failure report/summary, failure batch, formatter readiness, legal clamp
  signals, preservation blockers, `failure_return_summary()`,
  `runtime_failure_return_report()`, and
  `commit_decision_accounting_is_consistent()` before planning consumes the
  clamped execution context
- root runtime adapter execution commit-decision tests assert
  `AdapterSelectionRuntimeSummary::commit_summary()`,
  `AdapterSelectionRuntimeCommitAction`, mapped contract failure reports,
  primary failure report/summary, failure batch, formatter readiness, missing
  runtime adapter reports, drifted runtime adapter reports, disallowed adapter
  reports, fallback-reason drift, `failure_return_summary()`,
  `runtime_failure_return_report()`, and
  `commit_decision_accounting_is_consistent()` before response diagnostics are
  accepted
- FHT-DKE enabled mode changes dense/routed token split and routed KV exchange
  demand when route pressure changes, while budget summary helpers preserve
  valid split, KV-exchange parity, and clean budget commit readiness
- root FHT-DKE pressure-to-budget readiness preserves route-pressure and
  attention-threshold parity from `TransformerPlanningPressureSummary` through
  `FhtDkeBudgetSummary`, including stage order, first unready/blocking stage,
  pressure/threshold drift counts, aggregate accounting, and
  `can_commit_fht_dke_planning_readiness()`
- root FHT-DKE planning commit-decision tests assert
  `FhtDkePlanningReadinessSummary::commit_summary()`,
  `FhtDkePlanningCommitAction`, committed budget-summary presence, repair on
  stale route pressure or attention-threshold drift, and
  `commit_decision_accounting_is_consistent()` before runtime planning consumes
  the budget
- root runtime planning readiness preserves the same FHT-DKE summary from
  `FhtDkePlanningReadinessSummary` through `RuntimePlanningSummary`, including
  FHT-DKE/runtime-boundary drift counts and
  `can_commit_runtime_planning_readiness()` before backend request construction
- root runtime request planning readiness preserves the checked runtime plan
  through request planning parity and the request gate, including stage order,
  first unready/blocking stage, per-stage signal/blocker counts,
  request-planning parity drift, request-gate blockers, aggregate accounting,
  and `can_commit_runtime_request_planning()` before backend JSON serialization
- root saved acceptance contexts can produce the same request planning
  readiness from the saved request envelope and concrete imported KV blocks,
  avoiding separate root rebuilds of parity and request-gate facts
- experiment switch summaries preserve conservative defaults, per-feature
  states, runtime-planning feature state, attention/KV feature state, budget
  expansion, and enabled feature labels before root planning
- root and core transformer planners agree on template name, layer counts, and
  local/global/fusion allocation for representative profiles
- root transformer plan readiness tests compare route budget, compact plan
  summary, and layer-budget batch stages, including first unready/blocking
  stage, per-stage signal/blocker counts, aggregate readiness accounting, and
  `can_commit_transformer_plan_readiness()` before planner rows are expanded
- root route, attention, and transformer planner summaries agree through
  `TransformerPlanningPressureSummary` before planner ownership moves
- root transformer planning readiness tests compare route-budget readiness,
  attention-selection readiness, and planning-pressure stages, including stale
  pressure summary drift, first unready/blocking stage, per-stage
  signal/blocker counts, aggregate accounting, and
  `can_commit_transformer_planning_readiness()` before FHT-DKE or transformer
  planning consumes pressure facts
- root route-budget commit-decision tests assert
  `RouteBudgetReadinessSummary::commit_summary()`,
  `RouteBudgetReadinessCommitAction`, committed route budget presence,
  wait-for-route-budget noop behavior, repair-on-parity-drift behavior, and
  `commit_decision_accounting_is_consistent()` before attention, FHT-DKE, or
  transformer planning consumes route pressure
- root runtime KV export planning summaries agree on export-plan limit drift,
  planned-export overflow, focused export-boundary problem component counts,
  aggregate problem presence, signal component counts, runtime KV export commit
  signal/blocker counts, accounting consistency, and commit readiness before
  exported `KvBlock` payload comparison
- root runtime KV export readiness summaries agree on forward batch, export
  payload, export planning, and export block readiness stages, including
  `first_unready_stage()`, `first_blocking_stage()`, per-stage signal/blocker
  counts, aggregate accounting consistency, and no-op export behavior before
  root expands verbose layer or block payloads
- root runtime KV export payload summaries agree on forward-summary count
  drift, planned-block limit overflow, non-finite forward summary signals,
  forward input/activity/export/skip/limit signal counts, focused forward-batch
  and payload problem component counts, aggregate problem presence, and
  accounting consistency before exported `KvBlock` payload comparison
