# norion-core Failure-Return Matrix

This matrix is the root-adapter index for the compact failure-return APIs in
`crates/norion-core`. It is documentation only: adapters still own concrete
root `RuntimeError`, trace, and side-effect formatting.

## Call Discipline

For every row:

- call `failure_return_summary()` first and inspect `can_commit`,
  `should_return_failure`, `can_return_runtime_failure()`, the primary summary,
  batch counts, and accounting flags;
- convert the summary into `FailureReturnRoutingSummary` when root needs one
  common `(family, source_label)` trace key before branching into a
  family-specific report type;
- group multiple routing summaries with `FailureReturnRoutingBatchSummary`
  when root needs the first returnable route, total failure counts, blocker
  counts, accounting-drift counts, or family coverage in one adapter assertion;
- prefer `FailureReturnRoutingSelection::from_routes(...)` when root needs the
  batch, decision, and selected summary as one adapter-facing object;
- prefer `RuntimeKvPersistenceFailureReturnSelection::from_summaries(...)` for
  the two KV persistence summaries so namespace distribution is always checked
  before fusion persistence and swapped source order becomes repair accounting;
- use `routing_decision()` to choose between continuing, returning the selected
  runtime failure route, or repairing malformed failure-return accounting;
- use `select_route(...)` on the decision to recover the selected routing
  summary from the same route slice before materializing a report;
- when the selection wrapper is used, require
  `can_materialize_runtime_failure()` before calling the family-specific
  report materializer;
- materialize `runtime_failure_return_report()` only when
  `can_return_runtime_failure()` is true;
- treat `runtime_failure_return_report()` as the primary single failure to
  return, while the summary and batch fields preserve total failure counts;
- use `backend_message()`, `diagnostics_note()`, and `inference_error()` on the
  report type when the root adapter needs backend text, trace diagnostics, or a
  core `InferenceError`.

## Matrix

| Boundary / phase | Core object to call | Summary type | Report type | Source label(s) | Primary failure kinds | Root adapter use |
| --- | --- | --- | --- | --- | --- | --- |
| Runtime boundary commit | `RuntimeBoundaryCommitSummary` | `RuntimeFailureReturnSummary` | `RuntimeFailureReturnReport` | `boundary_commit` | Runtime, KV import/export, contract, context | Return the first failed request/response boundary gate before mutating response, reflection, memory, or KV side effects. |
| Runtime manifest boundary commit | `RuntimeManifestBoundaryCommitSummary` | `RuntimeFailureReturnSummary` | `RuntimeFailureReturnReport` | `manifest_boundary_commit` | Contract, plus nested boundary/KV failures | Gate manifest-aware request/response boundaries before exported-KV, reflection, and memory commits. |
| Runtime KV side-effect commit | `RuntimeKvSideEffectCommitSummary` | `RuntimeFailureReturnSummary` | `RuntimeFailureReturnReport` | `kv_side_effect_commit` | KV import/export or contract | Use one return path for import -> manifest boundary -> export side effects. |
| Adapter selection | `AdapterSelectionReport` commit summary | `AdapterFailureReturnSummary` | `AdapterFailureReturnReport` | `adapter_selection` | Contract | Fail before backend request construction when no allowed adapter or selection accounting is invalid. |
| Runtime adapter execution | `AdapterSelectionRuntimeSummary` commit summary | `AdapterFailureReturnSummary` | `AdapterFailureReturnReport` | `runtime_adapter_execution` | Contract | Fail response diagnostics when runtime-reported adapter is missing, drifted, or disallowed. |
| Adapter execution context | `AdapterExecutionContextSummary` commit summary | `AdapterFailureReturnSummary` | `AdapterFailureReturnReport` | `adapter_execution_context` | Contract | Fail hardware-to-runtime context handoff before routing, adapter selection, or request planning. |
| Runtime clamp | `AdapterRuntimeClampSummary` commit summary | `AdapterFailureReturnSummary` | `AdapterFailureReturnReport` | `runtime_clamp` | Contract | Fail when runtime metadata clamps or preserves context, precision, or parallelism inconsistently. |
| Hardware load snapshot | `HardwareLoadSnapshotSummary` commit summary | `HardwareFailureReturnSummary` | `HardwareFailureReturnReport` | `load_snapshot` | Contract | Fail hardware probing/adaptation before deriving a device execution plan. |
| Device execution plan | `DeviceExecutionPlanSummary` commit summary | `HardwareFailureReturnSummary` | `HardwareFailureReturnReport` | `device_execution_plan` | Contract | Fail device lane, pressure, prefetch, or budget shape before building a hardware plan. |
| Device execution adapters | `DeviceExecutionAdapterSummary` commit summary | `HardwareFailureReturnSummary` | `HardwareFailureReturnReport` | `device_execution_adapters` | Contract | Fail adapter family shape before adapter CSV/parity wiring. |
| Hardware plan | `HardwarePlanSummary` commit summary | `HardwareFailureReturnSummary` | `HardwareFailureReturnReport` | `hardware_plan` | Contract | Fail mapped hardware plan shape before deriving adapter execution context. |
| Hardware adapter bridge | `HardwareAdapterBridgeSummary` commit summary | `HardwareFailureReturnSummary` | `HardwareFailureReturnReport` | `adapter_bridge` | Contract | Fail bridge drift between hardware plan, execution plan, and adapter execution context. |
| Hardware runtime readiness | `HardwareRuntimeReadinessSummary` commit summary | `HardwareFailureReturnSummary` | `HardwareFailureReturnReport` | `hardware_runtime` | Contract | Fail the final hardware/runtime/device stage before manifest device handoff. |
| Runtime manifest validation | `RuntimeManifestValidationCommitSummary` | `ManifestFailureReturnSummary` | `ManifestFailureReturnReport` | `manifest_validation` | Contract | Return manifest validation errors through one runtime-failure path while warnings stay committable. |
| Runtime device handoff | `RuntimeDeviceHandoffCommitSummary` | `ManifestFailureReturnSummary` | `ManifestFailureReturnReport` | `runtime_device_handoff` | Contract | Fail hardware runtime, runtime clamp, or manifest execution compatibility before backend planning consumes the context. |
| Runtime KV import readiness | `RuntimeKvImportReadinessCommitSummary` | `RuntimeKvExchangeFailureReturnSummary` | `RuntimeKvExchangeFailureReturnReport` | `runtime_kv_import_readiness` | KV import or contract | Fail imported runtime KV shape before request planning or backend request serialization. |
| Runtime KV export readiness | `RuntimeKvExportReadinessCommitSummary` | `RuntimeKvExchangeFailureReturnSummary` | `RuntimeKvExchangeFailureReturnReport` | `runtime_kv_export_readiness` | KV export or contract | Fail exported runtime KV shape before persistence, reflection, or memory side effects. |
| KV namespace distribution | `KvNamespaceCountDriftCommitSummary` | `RuntimeKvPersistenceFailureReturnSummary` | `RuntimeKvPersistenceFailureReturnReport` | `kv_namespace_distribution` | Contract | Fail persistence when namespace counts drift before compaction or tiered-cache mutation. |
| KV fusion persistence | `KvFusionCommitSummary` | `RuntimeKvPersistenceFailureReturnSummary` | `RuntimeKvPersistenceFailureReturnReport` | `kv_fusion_persistence` | Contract | Fail fused-KV persistence when fusion accounting, namespace, or output shape blocks mutation. |
| Runtime planning acceptance | `RuntimePlanningAcceptanceCommitSummary` | `RuntimePlanningFailureReturnSummary` | `RuntimePlanningFailureReturnReport` | `runtime_planning_acceptance` | Contract or context | Fail planning before backend request construction when max-token, route pressure, adapter, FHT-DKE, or KV planning gates block execution. |
| Runtime request acceptance | `RuntimeRequestAcceptanceReport` | `RuntimeRequestFailureReturnSummary` | `RuntimeRequestFailureReturnReport` | `runtime_request_acceptance` | Contract or KV import | Fail malformed backend request envelopes before serialization or model execution. |
| Runtime response acceptance | `RuntimeResponseAcceptanceReport` | `RuntimeResponseFailureReturnSummary` | `RuntimeResponseFailureReturnReport` | `runtime_response_acceptance` | Contract or KV export | Fail parsed backend responses before exported-KV, reflection, memory, or final boundary commit. |

## Wiring Notes

- Preserve source labels as adapter trace fields. They are stable enough for root
  tests to assert exact phase routing without depending on verbose messages.
- Prefer the summary booleans and component counts for adapter decisions. Expand
  `failure_reports()` only for logs, Web Lab diagnostics, or compatibility tests.
- Keep route slices in explicit migration order. The first broad root adapter
  should order routes as planning acceptance, request acceptance, response
  acceptance, KV exchange readiness, then boundary commit, so earlier
  pre-request failures are returned before later side-effect gates.
- Within `RuntimeKvPersistence`, order `kv_namespace_distribution` before
  `kv_fusion_persistence` by using
  `RuntimeKvPersistenceFailureReturnSelection::from_summaries(...)`, so
  namespace count drift wins before root expands fused-KV persistence
  diagnostics and non-canonical source order does not materialize a runtime
  failure report.
- Keep nested gates visible. For example, KV side-effect commit can return the
  same report shape as boundary commit, but root tests should still assert the
  child import/manifest/export action and stage fields where available.
- Do not convert a clean summary into a report. Clean, warning-only, or
  committable summaries should stay side-effect ready even when they contain
  diagnostic signals.
