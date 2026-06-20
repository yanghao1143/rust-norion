# norion-core Adapter Matrix

This matrix is the field-level handoff plan for root `src/**` adapters. It is
not an implementation file. The root crate should own these conversions until
`norion-core` is added to the root workspace and parity tests are green.

## Rules

- Adapter code lives in root `src/**` or a thin integration crate.
- `norion-core` owns stable vocabulary, budget summaries, and contract checks.
- Root keeps model execution, JSON formatting, file paths, persistence,
  hardware probing, CLI/Web Lab text, and user-facing diagnostics.
- Conversions should be pure and covered by small parity tests before behavior
  is moved out of root modules.

## Runtime Request

| Root source | Core target | Mapping | Root-only fields |
| --- | --- | --- | --- |
| `runtime::RuntimeRequest.prompt` | `InferenceRequest.prompt` | copy string | none |
| `runtime::RuntimeRequest.profile` | `TaskProfile` | direct enum conversion | none |
| current prompt token count | `InferenceRequest.prompt_tokens` | set with `with_prompt_tokens(...)` before runtime planning | tokenizer implementation |
| `runtime::RuntimeRequest.max_tokens` | `InferenceRequest.max_tokens` | set with `with_max_tokens(...)`; use `RuntimePlanningDigest.generation_budget.max_generated_tokens` for backend limit | original user request cap |
| `runtime::RuntimeRequest.runtime_metadata` | `RuntimeMetadata` | copy all fields; preserve KV support, limits, and hot/cold precision | production asset checks |
| mapped runtime metadata | `RuntimeMetadataShapeSummary` | compare context window, embedding dimensions, KV exchange support, block limits, and hot/cold precision before max-token and KV import planning | production asset checks |
| `runtime::RuntimeRequest.route_budget` | `RouteBudget` | copy threshold, attention/fast token counts, attention fraction | prompt tokenization path |
| root route decision batch | `RoutingDecisionSummary` | compare layer counts, attention fraction, threshold crossings, score min/max/average, and derived `RouteBudget` before per-token diffs | prompt tokenization path |
| `runtime::RuntimeRequest.hardware_plan` | `AdapterExecutionContext` | map with `HardwarePlan::adapter_execution_context()` after root hardware converts to core | probe reports, gate reports |
| mapped adapter execution context | `AdapterExecutionContextSummary` | compare adapter count, pressure, compute headroom, latency, parallel chunks, KV prefetch, precision, token budgets, and disk-spill state before and after runtime metadata clamping | probe reports, gate reports |
| mapped adapter execution plus runtime metadata | `AdapterRuntimeClampSummary` | compare before/after context summaries, runtime metadata shape, KV prefetch reduction, precision clamp state, and non-limit field preservation before runtime planning | probe reports, gate reports |
| `runtime::RuntimeRequest.runtime_adapter_observations` | `AdapterObservation` | copy adapter, score, reward, quality, energy, KV influence, experience id | experience filtering |
| mapped execution context plus observations | `AdapterSelectionReport` | compare allowed adapter count, observation count, matching observation count, rejected observation count, matched fraction, fallback reason, and selected adapter before root consumes the adapter choice | experience filtering and diagnostics text |
| saved adapter selection plus runtime diagnostics adapter | `AdapterSelectionRuntimeSummary` | compare selected adapter, fallback reason, allowed adapter count, matching observations, runtime adapter presence, selection match, and execution-context membership before verbose diagnostics diffs | response diagnostics and reflection text |
| request plus route/execution/observations | `RuntimePlanningDigest` | build before `RuntimeRequestEnvelope`; compare `backend_max_tokens()`, selected adapter, `adapter_selection_report`, route pressure, `runtime_kv_prefetch_blocks`, and `planned_kv_exchange()` | concrete backend request object |
| runtime planning digest | `RuntimePlanningSummary` | compare generation budget, context-limited status, selected adapter, fallback reason, matched observations, FHT-DKE summary, KV exchange clamp, hardware pressure, compute headroom, parallel chunks, and latency budget before backend request construction | concrete backend request object |
| planned KV prefetch reduction | `RuntimePlanningKvClampSummary` | compare requested, runtime-clamped, and planned import counts plus runtime metadata reduction, FHT-DKE reduction, total reduction, and compact clamp reason before concrete KV candidate diffs | concrete backend request object |
| planned KV exchange | `RuntimePlanningKvClampReason` | compare not-clamped/runtime-metadata/FHT-DKE/both clamp reason before root drops or defers concrete KV import candidates | concrete backend request object |
| FHT-DKE budget result | `FhtDkeBudgetSummary` | compare enabled state, dense/routed fractions, token split validity, route pressure, routed-work state, routed tokens per KV exchange block, and route-pressure-driven KV exchange counts before KV block parity | concrete backend request object |
| planning digest FHT-DKE result | `RuntimePlanningDigest::fht_dke_summary()` / `RuntimePlanningSummary` helpers | read the same budget summary at the request planning boundary; compare route-pressure active/high state, FHT-DKE KV prefetch clamp state, and routed KV exchange before mutating concrete KV import/export lists | concrete backend request object |
| root experiment switches | `ExperimentSwitchesSummary` | compare `switches_summary()` for enabled feature count, per-feature state, runtime-planning features, attention/KV features, conservative budget state, and budget expansion before request planning | Web Lab/CLI/agent toggles |
| root experiment switches | `ExperimentSwitches` display helpers | use `enabled_labels()` and `summary()` after structured summary parity for CLI/Web Lab/agent diagnostics | Web Lab/CLI/agent display text |
| planning digest before backend request construction | `RuntimePlanningAcceptanceReport` | call `RuntimePlanningDigest::acceptance_report()` and map failures before root formats backend limits or KV import plans | root planning/runtime error enum |
| planning acceptance report | `RuntimePlanningAcceptanceSummary` | compare accepted state, planning violation count, context exhaustion, contract failure count, and failure-report count before parsing verbose violations | root planning/runtime error enum |
| request plus architecture/transformer/hardware/import blocks | `RuntimeAcceptanceContext::from_request_parts(...)` | build the request envelope, clamp hardware execution with runtime metadata, and persist facts used by both request and response acceptance gates; pass exactly the concrete `KvBlock` imports selected by the root tier/filter path | JSON serialization, runtime execution, trace storage |
| saved context request | `RuntimeRequestEnvelope` | attach `RuntimePlanningDigest` / recursive summary when available, then run request acceptance before `runtime_request_json(...)`; `imported_kv_blocks` and `kv_prefetch_blocks` must both match `planned_kv_exchange().import_blocks` | JSON serialization, toolsmith, agent team, memory hint strings |
| saved context request plus attached planning digest | `RuntimeRequestPlanningParitySummary` | compare backend max tokens, generation budget, selected adapter, imported KV count, KV prefetch count, planned export count, and planning contract violation count before verbose request diffs | JSON serialization and root runtime error formatting |
| saved context request | `RuntimeRequestEnvelopeSummary` | compare `envelope_summary()` for context truncation, adapter candidates, selected adapter, runtime/transformer layer parity, hardware pressure band, planning import/export blocks, and recursive attachment before verbose request violations | root runtime error formatting |
| saved request plus concrete imported runtime KV blocks | `RuntimeRequestGateSummary` | call `request_gate_summary(imported_kv_blocks)` after planning and envelope summaries; only serialize backend JSON when `can_send_request()` is true | JSON serialization and root runtime error formatting |
| imported runtime KV blocks plus saved request envelope | `RuntimeAcceptanceContext::request_acceptance_report()` | combine request contract checks with request-derived imported KV validation before JSON formatting, including planned import count versus the concrete imported vector length | root runtime error formatting |
| saved request acceptance context | `RuntimeAcceptanceContext::request_acceptance_summary()` | read request-contract and imported-KV failure counts plus accepted imported block count from the saved request/hardware/import facts | root runtime error formatting |
| request acceptance report | `RuntimeRequestAcceptanceSummary` | compare accepted state, request-contract failures, imported-KV failures, accepted imported block count, and failure-report count before parsing verbose violations | root runtime error formatting |
| rejected request acceptance report | `RuntimeRequestAcceptanceReport::failure_reports()` | map request contract failures to `runtime_contract_violation` and imported KV failures to `runtime_kv_import_error` | root error enum and user-facing diagnostic text |

## Runtime Response

| Root source | Core target | Mapping | Root-only fields |
| --- | --- | --- | --- |
| `runtime::RuntimeResponse.answer` | `InferenceOutcome.answer` and `RuntimeResponseEnvelope.answer_chars` | copy answer; count chars in core envelope | reflection repair text |
| `runtime::RuntimeToken` | `GeneratedToken` | copy text, logprob, entropy | token streaming transport |
| root runtime token metrics | `GeneratedTokenMetrics` | recompute from `GeneratedToken` and compare with root metrics | UI formatting |
| mapped root runtime response | `InferenceOutcomeSummary` | compare answer length, token count, uncertainty signal, route token counts, imported/exported KV counts, diagnostics KV parity, runtime execution signal, and diagnostics note count before response-envelope validation | reflection repair text |
| `reflection::RuntimeDiagnostics` | `RuntimeDiagnostics` | copy model id, adapter, architecture, device execution, KV precision, latency and runtime notes | human-readable reflection report |
| mapped runtime diagnostics | `RuntimeDiagnosticsSummary` | compare model/adapter presence, architecture signal, layer-mode count, device execution source, forward/KV signal, KV exchange count, and precision validity before verbose diagnostics diffs | human-readable reflection report |
| mapped runtime diagnostics plus saved request envelope | `RuntimeDiagnosticsRequestParitySummary` | compare `request_parity_summary(...)` for model id, selected adapter, architecture, imported KV count, export bounds, and KV precision before verbose diagnostics diffs | human-readable reflection report and root runtime error enum |
| mapped runtime diagnostics plus saved hardware plan | `RuntimeHardwareDiagnosticsReport` | call `RuntimeDiagnostics::hardware_acceptance_report(...)` and map failures before trusting runtime-reported device execution | root runtime error enum and trace labels |
| runtime hardware diagnostics report | `RuntimeHardwareDiagnosticsSummary` | compare accepted state, hardware violation count, and mapped failure-report count before expanding device execution violation strings | root runtime error enum and trace labels |
| `engine::EmbeddingDiagnostics` | `EmbeddingDiagnosticsSummary` | compare query source/dimensions, optional memory-write embedding, gist write count/source mix, runtime/fallback call counts, and total calls before folding into inference diagnostics | root reflection text |
| saved request envelope | `InferenceDiagnostics` | seed with `from_request_envelope(...)` or `with_request_envelope(...)` before adding runtime/embedding diagnostics | root reflection text |
| mapped inference diagnostics | `InferenceDiagnosticsSummary` | compare generation-budget truncation, route token counts, runtime KV exchange, runtime/embedding execution signals, hardware pressure band, recursive calls, and note count before nested diagnostics diffs | root reflection text |
| mapped inference diagnostics plus saved request envelope | `InferenceDiagnosticsRequestParitySummary` | compare route budget, generation budget, hardware pressure, planning headroom/latency, and nested runtime diagnostics request parity before response-envelope parity | root reflection text and response fallback policy |
| saved request envelope | `RuntimeDiagnostics` | seed missing model id, selected adapter, architecture, imported KV count, and KV precision without overwriting runtime-reported values | runtime omission handling |
| `runtime::RuntimeResponse.exported_kv_blocks` | `KvBlock` with `KvNamespace::Runtime` | convert at export boundary and validate with `RuntimeKvBlockContract` | runtime error formatting |
| parsed root response | `RuntimeResponseEnvelopeSummary` | compare `envelope_summary()` for answer/token shape, uncertainty signal, imported/exported KV totals, exact diagnostics KV parity, and runtime execution signal before verbose response violations | root error enum and fallback answer policy |
| parsed root response plus saved request envelope | `RuntimeResponseRequestParitySummary` | compare `request_parity_summary(...)` for token caps, request/planned KV bounds, adapter parity, generation budget, route budget, hardware pressure, and optional planning headroom/latency before verbose request parity violations | root error enum and fallback answer policy |
| parsed root response plus saved request and hardware | `RuntimeResponseGateSummary` | call `response_gate_summary(...)` after envelope and request-parity summaries; only accept parsed runtime output when `can_accept_response()` is true | root error enum, fallback answer policy, memory update policy |
| saved acceptance context plus parsed response outcome | `RuntimeAcceptanceContext::response_gate_summary(...)` | prefer when root has the saved context so request, hardware, and response gate facts come from the same source | root error enum, fallback answer policy, memory update policy |
| parsed root response plus saved acceptance context | `RuntimeAcceptanceContext::response_acceptance_report(...)` | build after `parse_runtime_response_json(...)`; combine response diagnostics checks, request-envelope backcheck, and request-derived exported KV validation before memory/reflection mutation | JSON parser, trace steps, memory update policy |
| saved response acceptance context | `RuntimeAcceptanceContext::response_acceptance_summary(...)` | read response-contract, request-parity, and exported-KV failure counts plus accepted exported block count from the saved request/hardware facts | root error enum and fallback answer policy |
| saved request context plus parsed response outcome | `RuntimeBoundaryEnvelopeSummary` | compare `boundary_envelope_summary(...)` for request token limit, imported KV request match, response diagnostics KV parity, runtime execution signal, adapter candidate presence, and context-limited request state before acceptance reports | root error enum and fallback answer policy |
| saved request context plus parsed response outcome | `RuntimeBoundaryAdapterSummary` | compare request selected adapter, runtime diagnostics adapter, adapter candidate count, optional planning selection parity, and allowed execution-context membership before response acceptance reports | root error enum, trace labels, and reflection text |
| saved request context plus parsed response outcome | `RuntimeBoundaryKvSummary` | compare concrete imported blocks, response imported/exported counts, diagnostics counts, runtime import/export limits, planning exchange bounds, accepted/violating KV counts, and namespace counts before verbose KV payload violations | root error enum, memory update policy, and reflection text |
| saved request context plus parsed response outcome | `RuntimeBoundaryGateSummary` | compare request/response acceptance, envelope shape, adapter parity, KV parity, total violations, and mapped failure-report count as the final structured gate before committing response side effects | memory update policy and reflection mutation |
| saved request context plus parsed response outcome | `RuntimeAcceptanceContext::boundary_acceptance_summary(...)` | compare combined request/response accepted state, total violation count, total failure-report count, KV failure state, and request-parity state before expanding either side's verbose reports | root error enum and fallback answer policy |
| response acceptance report | `RuntimeResponseAcceptanceSummary` | compare accepted state, response-contract failures, request-parity failures, exported-KV failures, accepted exported block count, and failure-report count before parsing verbose violations | root error enum and fallback answer policy |
| rejected response acceptance report | `RuntimeResponseAcceptanceReport::failure_reports()` | map response/request parity failures to `runtime_contract_violation` and exported KV failures to `runtime_kv_export_error` | root error enum and fallback answer policy |
| root runtime errors | `RuntimeFailureReport` | map runtime, KV import/export, contract, and context exhaustion errors by kind | backend-specific error enum |
| mapped runtime failure batch | `RuntimeFailureBatchSummary` | compare failure class counts, recoverable count, backend-error count, diagnostics-note count, and minimum trace confidence before formatting root runtime errors | root error enum, trace labels, diagnostics notes |

## Router And Hierarchy

| Root source | Core target | Mapping | Root-only fields |
| --- | --- | --- | --- |
| `hierarchy::TaskProfile` | `TaskProfile` | direct enum conversion | none |
| `hierarchy::HierarchyWeights` | `HierarchyWeights` | map `global`, `local`, root `convolution` to core `fusion` | root naming compatibility |
| mapped hierarchy weights | `HierarchyWeightsSummary` | compare normalized state, total weight, dominant focus, and global/local/fusion values before routing context parity | root naming compatibility |
| `hierarchy::ProfileHierarchyWeights` | `ProfileHierarchyWeightsSummary` | compare per-profile normalized state and expected coding/local, writing/global, long-document/fusion focus before controller state parity | root controller storage |
| `hierarchy::ProfileHierarchyObservations` | `ProfileHierarchyObservations` helpers | compare per-profile counts, total observations, and active profile count before replacing hierarchy controller state storage | root controller storage |
| `router::RoutingContext` | `RoutingContext` | copy profile, context tokens, cache hit rate, pressure, compute headroom; feed hierarchy from core hardware plan | optional latency budget |
| `router::RouteBudget` | `RouteBudget` | copy all fields | none |
| `router::RoutingDecision` batch | `RoutingDecisionSummary` / `RouteLayerCounts` | count fast/local/global/fusion layers, threshold crossings, score range, and derived route budget before per-token parity checks | root scorer/tokenizer |
| `router::GenerationMetrics` | `GenerationMetrics` | copy perplexity, semantic consistency, contradiction count, token count | source of metrics |
| mapped routing feedback | `RoutingFeedbackSummary` | compare profile, quality, perplexity, contradiction count, low/high quality state, and contradiction pressure before observation | source of metrics |
| routing feedback batch | `RoutingFeedbackBatchSummary` | compare feedback count, profile distribution, quality/perplexity averages, contradiction total, and low/high quality counts before adaptive threshold diffs | source of metrics |
| `router::RouterState` | `RouterState` | copy threshold, observation count, profile thresholds, profile observations | storage path |
| restored router state snapshot | `RouterState` helpers | compare `profile_observation_total()` and `has_observation_drift()` before replacing root router state storage | storage path |
| `router::NoironRouter` | `DefaultHierarchicalRouter` | compare decisions and feedback before replacing root router ownership | root scorer/tokenizer |
| root attention policy state | `ThresholdAttentionPolicySummary` | compare base/min/max thresholds, per-profile thresholds, bounded state, threshold spread, and adapted profile count before and after observation | root scorer/tokenizer |
| mapped attention candidate | `AttentionCandidateSummary` | compare token, position, score, entropy, layer, attention-use flag, high-entropy state, and score threshold state before selection diffs | root scorer/tokenizer |
| attention candidate batch | `AttentionCandidateBatchSummary` | compare candidate count, attention candidate count, fast count, layer mix, average/max score, average/max entropy, and attention fraction before selected/rejected diffs | root scorer/tokenizer |
| root attention token selection | `AttentionDecision` helpers | compare selected/rejected counts, selection fraction, and cap-hit behavior before replacing attention selection | root scorer/tokenizer |
| root attention selected/rejected batches | `AttentionDecisionSummary` | compare selected/rejected layer counts, attention-token totals, selection fraction, and cap-hit status before per-token diffs | root scorer/tokenizer |
| root KV fusion report | `KvFusionMergeSummary` | compare merged count, merge fraction, changed/noop status, block-accounting balance, skipped-limit status, namespace-count/result-count parity, namespace mix, grouped namespace counts, and runtime/non-runtime result counts before applying fused KV persistence | persistence and compaction side effects |

## Hardware

| Root source | Core target | Mapping | Root-only fields |
| --- | --- | --- | --- |
| `hardware::HardwareSnapshot` | `HardwareLoadSnapshot` | map device and normalized CPU/GPU/RAM/disk load | environment detection |
| mapped root `HardwareSnapshot` | `HardwareLoadSnapshotSummary` | compare normalized load values, dominant load kind, device tier, pressure value, and pressure band before full hardware plan parity | probe descriptors and gate display rows |
| `hardware::DeviceClass` | `DeviceClass` | direct enum or parse from `as_str()` | probe descriptors |
| `hardware::DeviceProfileDescriptor` | `DeviceProfileDescriptorSummary` | compare device, tier, scope presence/length, alias count, auto-profile state, supported profile count, and explicit profile count before descriptor-table diffs | probe descriptors |
| `hardware::DeviceTier` | `DeviceTier` | direct enum or parse by tier string if adapter crate avoids shared enums | gate display rows |
| `hardware::ComputeLane` | `ComputeLane` | parse `as_str()` for diagnostics parity | root lane selection code until migration |
| `hardware::DeviceMemoryMode` | `DeviceMemoryMode` | parse `as_str()` for diagnostics parity | root memory-mode display |
| `hardware::RuntimeAdapterHint` | `RuntimeAdapter` | parse `as_str()`; unknown values fail adapter tests | hardware probe source |
| `hardware::DeviceExecutionPlan` | `DeviceExecutionPlan` | map lanes, memory mode, adapter hints, parallel chunks, KV prefetch, precision, disk spill | root gate summary |
| mapped root `DeviceExecutionPlan` | `DeviceExecutionPlanSummary` | compare primary/fallback lane, memory mode, adapter hint count, parallel chunks, KV prefetch, precision, and disk spill before full hardware plan parity | root gate summary |
| mapped execution adapter hints | `DeviceExecutionAdapterSummary` | compare portable fallback, CPU, GPU, neural, multi-device, custom, and total adapter counts before gate-row CSV diffs | root gate summary |
| `hardware::HardwarePlan` | `HardwarePlan` | map device, tier, pressure, latency, KV budgets, hierarchy, execution, notes | production validation |
| mapped root `HardwarePlan` | `HardwarePlanSummary` | compare pressure band, compute headroom, reduced parallel chunks, KV prefetch, hot/cold precision, adapter count, disk spill, and notes count before moving device planning | production validation and diagnostics text |
| mapped hardware plan to adapter context | `HardwareAdapterBridgeSummary` | compare hardware plan, execution plan, and derived adapter execution context for adapter counts, pressure, compute headroom, latency, parallelism, KV prefetch, precision, token budgets, and disk-spill parity before runtime planning | production validation and diagnostics text |

## Runtime Manifest

| Root source | Core target | Mapping | Root-only fields |
| --- | --- | --- | --- |
| `runtime_manifest::TransformerRuntimeArchitecture` | `TransformerRuntimeArchitecture` | copy layer count, hidden size, attention heads, KV heads, local window | asset-specific defaults |
| mapped transformer runtime architecture | `TransformerRuntimeArchitectureSummary` | compare layer count, hidden size, attention/KV heads, local window, attention head dimension, and head/window validity before full manifest ABI checks | asset-specific defaults |
| `runtime_manifest::RuntimeKvPolicy` | `RuntimeKvPolicy` | copy enabled flags and limits | manifest file path |
| mapped runtime KV policy | `RuntimeKvPolicySummary` | compare import/export capability, block capacity, and limit/capability consistency before runtime KV exchange planning | manifest file path |
| `runtime_manifest::RuntimeQuantizationPolicy` | `RuntimeQuantizationPolicy` | map hot/cold/weights `QuantizationBits` | production codec selection |
| mapped runtime quantization policy | `RuntimeQuantizationPolicySummary` | compare hot/cold KV precision, optional weight precision, compressed KV flags, and cold-not-wider-than-hot parity before full manifest ABI checks | production codec selection |
| `runtime_manifest::RuntimeManifest` | `RuntimeManifestDigest` | copy metadata, architecture, KV policy, quant policy, preferred adapter | asset validation and existence checks |
| mapped runtime manifest digest | `RuntimeManifestAbiSummary` | compare effective context window, embedding dimensions, transformer shape, KV exchange limits, quantization bits, and supported adapter count before request planning | asset validation and existence checks |
| mapped runtime manifest digest | `RuntimeManifestValidation` | run `validate()` and map errors with `failure_reports()` before request planning; keep warnings non-blocking | asset validation and existence checks |
| runtime manifest validation | `RuntimeManifestValidationSummary` | compare passed state, error count, warning count, warnings-only status, and failure-report count before verbose validation text | asset validation and existence checks |
| mapped manifest plus hardware execution adapters and observations | `RuntimeManifestAdapterCompatibilitySummary` | compare supported adapter count, execution adapter count, compatible adapter count, observation count, compatible observation count, rejected observation count, missing manifest catalog, missing execution hints, disjoint adapter sets, runtime-planning readiness, selected-adapter availability, observed/fallback selection state, preferred adapter, preferred observed adapter, and selected adapter before runtime planning consumes adapter selection | asset validation and experience filtering |
| mapped manifest plus hardware execution KV settings | `RuntimeManifestExecutionCompatibilitySummary` | compare manifest import/export capacities, execution KV prefetch, disabled import requests, prefetch limit drift, hot/cold precision coverage, and compact KV contract readiness before root expands device-gate failure strings | asset validation and device gate reports |

## Transformer

| Root source | Core target | Mapping | Root-only fields |
| --- | --- | --- | --- |
| `transformer::AttentionKind::Global` | `TransformerAttentionKind::Global` | direct enum conversion | none |
| `transformer::AttentionKind::LocalWindow` | `TransformerAttentionKind::LocalWindow` | direct enum conversion | none |
| `transformer::AttentionKind::ConvolutionalFusion` | `TransformerAttentionKind::Fusion` | root `convolution` naming maps to core `fusion` | root naming compatibility |
| `transformer::TransformerLayerPlan` | `TransformerLayerBudget` | copy layer index, attention kind, compute fraction, window size | planner internals |
| mapped transformer layer | `TransformerLayerBudgetSummary` | compare layer index, attention label, fusion state, compute fraction, and window size before full row diffs | planner internals |
| `transformer::TransformerRefactorPlan` | `TransformerPlanDigest` | copy template name and converted layers | root planner ownership |
| mapped transformer digest | `TransformerPlanSummary` | compare layer count, global/local/fusion counts, average compute fraction, and min/max window size before per-layer diffs | root planner ownership |
| mapped route budget plus attention and transformer summaries | `TransformerPlanningPressureSummary` | compare route attention fraction, selected attention fraction, cap/rejection pressure, non-local/fusion layer fractions, and average compute before planner ownership moves | root scorer/tokenizer and planner ownership |
| root planner inputs | `TransformerPlanningInput` | map profile, hierarchy, route budget, layer count, hidden size, local window, hardware pressure | root scorer |
| local/production forward summaries | `TransformerForwardSummary` | copy layer index, attention kind, token range, vector, activation scale | kernel execution |
| mapped forward summary batch | `TransformerForwardBatchSummary` | compare summary count, attention-kind mix, compute/window bounds, activation shape, active layer count, and non-finite forward values before export materialization | kernel execution |
| forward vector plus mapped summaries | `RuntimeKvExportSummary` | compare enabled state, planned blocks, forward vector length, summary count, export-limit hit, and empty-forward skip before materializing blocks | root export materialization |
| forward vector plus mapped summaries plus runtime planning digest | `RuntimeKvExportPlanningSummary` | compare planning export blocks, export plan max blocks, forward export summary, plan-limit match, and planned export count before materializing runtime KV blocks | root export materialization |
| forward vector plus mapped summaries | `RuntimeKvExportPlan::planned_block_count(...)` | compare planned export count with root runtime export count before comparing payload shape | root export materialization |
| forward vector plus mapped summaries | `RuntimeKvExportPlan::build_blocks(...)` | compare layer/head assignment, token range, key/value split, score, and export limit with root runtime KV exports | root export materialization |

## KV, Quantization, And Memory

| Root source | Core target | Mapping | Root-only fields |
| --- | --- | --- | --- |
| runtime KV exchange blocks | `KvBlock` | use `KvNamespace::Runtime`; copy layer, head, token range, key/value, score if present | runtime-specific block wrapper |
| gist memory KV | `KvBlock` | use `KvNamespace::Gist` | gist persistence |
| agent-local KV | `KvBlock` | use `KvNamespace::Agent(agent_id)` | agent scheduler |
| experiment/adapter KV | `KvBlock` | use `KvNamespace::Custom(id)` | experiment registry |
| mapped KV block batch | `KvNamespaceCounts` | compare runtime, semantic, gist, agent, and custom block counts before payload diffs or fused persistence mutation | runtime-specific block wrapper |
| memory/Infini candidates | `RuntimeKvCandidate` | copy id, vector, weight after root tier filtering | memory retrieval |
| runtime import plan inputs | `RuntimeKvImportPlan` | build from runtime metadata, architecture, and planned KV prefetch/import count | root import error conversion |
| runtime import candidates plus plan | `RuntimeKvImportSummary` | compare candidate count, non-empty candidate count, planned imports, import-limit hit, and embedding dimensions before materializing blocks | root import error conversion |
| mapped runtime KV block | `KvBlockShapeSummary` | compare namespace label, runtime-exchange flag, layer/head, token range, vector lengths, empty-vector state, and finite-value state before payload diffs | runtime-specific block wrapper |
| runtime import/export blocks | `RuntimeKvBlockContract` | validate namespace, layer/head, token bound, dimension, finite values, and block count; prefer request/response acceptance reports at root boundaries, and use `for_request_imports(...)` / `for_request_exports(...)` for focused KV tests | runtime error type |
| runtime KV validation report | `RuntimeKvValidationSummary` | compare accepted block count, violation count, and valid state before converting lower-level payload failures into root runtime errors | runtime error type |
| `kv_quant::QuantizedVector` | `QuantizedVector` | compare 4-bit and 8-bit payload round trips before replacing codec | old persistence readability |
| root KV block plus metadata | `QuantizedKvBlock` | use `KvQuantizationPlan`; runtime namespace uses hot KV, other namespaces cold KV | persistence format |
| quantized root KV block | `QuantizedKvPayloadSummary` | compare namespace label, runtime hot-bit selection, non-runtime cold-bit selection, vector lengths, packed lengths, compression ratio, and empty-payload state before byte/string codec parity | old persistence readability |
| quantized root KV block payload summary | `QuantizedKvBlock` helpers | compare `vector_value_len()`, `packed_payload_len()`, and `compression_ratio()` before byte/string codec parity | old persistence readability |
| `kv_cache::MemoryEntry` | `MemoryRecord` | copy id/key/namespace, vector, strength, hits/failures, score, timestamps | store mutation |
| mapped memory record | `MemoryRecordSummary` | compare namespace, vector length, strength, reliability, attempts, failure state, finite-value state, and age span before full vector payload diffs | store mutation |
| `kv_cache::reinforce` / `kv_cache::penalize` report | `MemoryUpdateSummary` | compare id, action, requested amount, applied/missing state, removed state, and strength delta before root checks concrete entry mutation | store mutation |
| feedback report batch | `MemoryUpdateBatchSummary` | compare report count, applied/missing count, action mix, removed count, requested amount total, and net strength delta before persistence side effects | store mutation |
| `kv_cache::MemoryMatch` | `TieredMemoryCandidate` | copy id/key/vector/strength and set active similarity | lookup path |
| mapped tiered candidate | `TieredMemoryCandidateSummary` | compare strength, reliability, attempts, failures, last score, and active similarity before scheduler placement rows | lookup path and tier scheduling |
| current tiered placements | `TieredCacheSummary` | compare placement count, tier counts, hot/warm/cold fractions, multi-tier state, average score, and score range before individual placement rows | persistence and eviction side effects |
| previous and current tiered placements | `TierMigrationSummary` | compare promote, demote, retain, new, and evict counts before applying tier side effects | persistence and eviction side effects |
| retention/compaction policies | `MemoryGovernancePolicy` | map root policies; compare reports before root mutation | persistence and eviction side effects |
| root retention/compaction result summaries | `MemoryGovernanceSummary` | compare `governance_summary()` for retention/compaction before-after counts, decayed/removed/merged counts, total removed ids, note count, noop state, and final record count before id-level diffs | persistence and eviction side effects |

## Recursive Scheduling

| Root source | Core target | Mapping | Root-only fields |
| --- | --- | --- | --- |
| `recursive_scheduler::RecursiveSchedule` | `RecursiveScheduleSummary` | copy prompt/window/chunk/overlap/fan-in/parallel counts and chunk/merge/wave counts | prompt chunk materialization |
| `RecursiveChunk` | `RecursiveChunk` | copy index, token range, estimated tokens, overlaps | text slicing |
| `RecursiveMergeRound` | `RecursiveMergeRound` | copy round, input units, output units | merge prompt text |
| `RecursiveExecutionWave` | `RecursiveExecutionWave` | copy wave, start/end chunk, chunk count | runtime call execution |
| root scheduler config | `RecursiveSchedulerConfig` | compare token estimator, chunk ranges, merge rounds, and execution waves | concrete scheduler ownership until parity |
| mapped recursive schedule digest | `RecursiveScheduleValidationSummary` | compare valid state plus shape/chunk/merge/execution-wave violation counts before verbose schedule violations | concrete scheduler ownership until parity |
| root recursive runtime diagnostics | `RecursiveScheduleDigest` helpers | compare empty/single-pass state, chunk runtime units, merge runtime units, total runtime units, recursion overhead, max wave width, and used parallelism | concrete runtime call execution |
| mapped recursive request summary | `RecursiveScheduleValidationSummary` | compare `RecursiveScheduleSummary::validation_summary(request_prompt_tokens)` before attaching recursive state to `RuntimeRequestEnvelope` | concrete runtime call execution |
| root recursive request summary | `RecursiveScheduleSummary` helpers | compare requested parallelism, minimum runtime units, and minimum recursion overhead before request-envelope attachment | concrete runtime call execution |

## Parity Test Order

1. Metadata shape, architecture, hardware enum, and adapter string round trips.
2. Runtime transformer architecture summary parity for layer/head/window shape,
   attention head dimension, and KV-head validity.
3. Runtime KV policy summary parity for import/export capability, block
   capacity, and limit/capability consistency.
4. Runtime quantization policy summary parity for hot/cold KV precision,
   optional weight precision, compressed KV state, and cold-not-wider-than-hot
   checks.
5. Runtime manifest ABI summary parity for effective context, transformer
   shape, KV exchange limits, quantization widths, and supported adapter count.
6. Adapter execution context summary parity before runtime metadata clamping.
7. Adapter runtime clamp summary parity for before/after context shape,
   runtime metadata limits, KV prefetch reduction, precision clamp state, and
   non-limit field preservation.
8. Adapter selection report parity for allowed/matching observations and
   fallback reason.
9. Runtime generation budget and `RuntimePlanningDigest` max-token,
   adapter-selection-report, KV exchange, and KV clamp-reason decisions.
10. Runtime planning KV clamp summary parity for requested, runtime-clamped, and
   planned import counts plus runtime metadata reduction, FHT-DKE reduction,
   total reduction, and compact clamp reason.
11. `ExperimentSwitchesSummary` parity for enabled feature count, per-feature
   state, runtime-planning features, attention/KV features, and budget expansion
   before request planning.
12. `RuntimePlanningAcceptanceReport` and `RuntimePlanningAcceptanceSummary` map
   context exhaustion and malformed planning contracts before root backend
   request construction.
13. `RuntimeRequestPlanningParitySummary` checks attached planning-digest
   max-token, adapter, generation-budget, and KV prefetch/import-count parity,
   before JSON serialization.
14. `RuntimeRequestEnvelope` contract checks after the structured planning
   parity summary, including attached planning-digest verbose violations.
15. `RuntimeRequestEnvelopeSummary` parity for context pressure, adapter
   presence, layer parity, hardware pressure band, planning KV exchange, and
   recursive attachment before verbose request diffs.
16. `RuntimeRequestGateSummary` is the final structured request-side gate for
   request acceptance, envelope/planning parity, imported KV validation, and
   failure-report counts before backend JSON serialization.
17. `RuntimeAcceptanceContext::from_request_parts(...)` clamps hardware execution
   with runtime metadata before request KV prefetch fields are accepted.
18. `RuntimeAcceptanceContext` persists request, hardware, and imported KV facts,
   and rejects drift between planned imports and concrete imported block count.
19. `RuntimeRequestAcceptanceReport` gate and acceptance-summary counts before
   runtime request JSON.
20. Request acceptance failure reports preserve contract and KV import trace
   labels.
21. Recursive schedule validation summary parity for valid state and
   shape/chunk/merge/execution-wave failure counts.
22. Recursive schedule summary parity for chunk, merge, wave, runtime-unit,
   recursion-overhead, and parallelism counts.
23. Inference outcome summary parity for answer length, token count,
   uncertainty signal, route token counts, imported/exported KV counts,
   diagnostics KV parity, runtime execution signal, and diagnostics note count.
24. Runtime diagnostics summary parity for model/adapter presence,
   architecture signal, layer modes, device execution source, forward/KV signal,
   KV exchange count, and precision validity.
25. Adapter selection runtime summary parity for missing runtime adapter
   reports, selected-adapter drift, and adapters outside the saved execution
   context.
26. Runtime response diagnostics and hardware contract checks, including
   `RuntimeHardwareDiagnosticsReport` failure mapping.
27. Runtime hardware diagnostics summary parity for accepted state, violation
   count, and mapped failure-report count.
28. `RuntimeResponseEnvelopeSummary` parity for answer/token uncertainty,
   exact diagnostics KV counts, and runtime execution signal before verbose
   response diffs.
29. `RuntimeResponseRequestParitySummary` parity for token cap,
   adapter, generation budget, route budget, hardware pressure, compute
   headroom, latency budget, and KV exchange counts before verbose violations.
30. `RuntimeResponseGateSummary` is the structured parse-to-commit gate for
   response acceptance, envelope shape, request parity, exported KV validation,
   and failure-report counts before memory/reflection mutation.
31. Runtime response versus originating request envelope verbose parity
   violations after the structured summary is checked.
32. `RuntimeBoundaryEnvelopeSummary` shape parity from one saved context before
    request/response acceptance reports are expanded.
33. `RuntimeBoundaryAdapterSummary` adapter parity from the saved context before
    response acceptance reports are expanded.
34. `RuntimeBoundaryKvSummary` KV parity from the saved context before response
    acceptance reports expand verbose KV payload violations.
35. `RuntimeResponseAcceptanceReport` gate and acceptance-summary counts before
    memory/reflection mutation.
36. `RuntimeBoundaryAcceptanceSummary` combines saved-context request and
   response gate counts before either side's verbose reports are expanded.
37. `RuntimeBoundaryGateSummary` is the final structured commit gate before
   reflection or memory mutation.
38. Response acceptance failure reports preserve contract and KV export trace
   labels.
39. `RuntimeFailureBatchSummary` parity for runtime/KV/contract/context
   failure counts, recoverability, backend-error wrapping, diagnostics notes,
   and minimum trace confidence before root formats runtime errors.
40. `EmbeddingDiagnosticsSummary` parity for query source/dimensions, memory
   write, gist write source mix, runtime/fallback call counts, and total calls
   before folding embedding facts into inference diagnostics.
41. Inference diagnostics summary parity for generation truncation, route token
   counts, runtime KV exchange, runtime/embedding execution signals, pressure
   band, recursive calls, and note count.
42. `InferenceDiagnosticsRequestParitySummary` parity for route budget,
   generation budget, hardware pressure, planning headroom/latency, and nested
   runtime diagnostics request parity before response-envelope parity.
43. Response diagnostics seeding from request envelope while preserving
   runtime-specific diagnostics.
44. Runtime diagnostics request-envelope seeding for missing model, adapter,
   architecture, imported KV count, and precision signals.
45. `RuntimeDiagnosticsRequestParitySummary` parity for model id, selected
   adapter, architecture, imported KV count, export bounds, and KV precision
   against the saved request envelope before verbose diagnostics diffs.
46. Router state and generation metrics feedback parity.
47. Attention policy state summary parity for base/min/max thresholds,
    per-profile threshold drift, bounded state, threshold spread, and adapted
    profile count before attention decision parity.
48. Hardware load snapshot summary parity for normalized CPU/GPU/RAM/disk load,
    dominant load kind, device tier, pressure value, and pressure band.
49. Device execution summary parity for lanes, memory mode, adapter hints,
    parallel chunks, KV prefetch, precision, and disk spill.
50. Hardware plan summary parity for pressure band, parallelism reduction, KV
    prefetch, precision, disk spill, and adapter count.
51. Hardware adapter bridge summary parity for mapped hardware plan, execution
    plan, and derived adapter execution context before runtime planning.
52. Transformer plan summary, digest, forward batch summary, forward KV export
    summary, and export-planning summary parity.
53. Runtime KV import summary, block shape summary, import/export validation
    summary, and quantized vector codec parity.
54. KV fusion merge summary parity for collapsed blocks, noop state,
    block-accounting balance, skipped candidates, namespace mix, grouped
    namespace counts, namespace-count/result parity, and runtime/non-runtime
    result counts.
55. `MemoryGovernanceSummary` parity for retention/compaction before-after
    counts, decayed/removed/merged counts, total removed ids, note count, noop
    state, and final record count before root cache mutation.
56. `TieredMemoryCandidateSummary` parity for strength, reliability, attempts,
    failures, last score, and active similarity before tier scheduler placement
    rows.
57. Memory record summary, tiered cache summary, retention, compaction, and
    tiered placement parity.
