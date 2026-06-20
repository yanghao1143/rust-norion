# norion-memory Adapter Plan

This note describes the first adapter layer from the root Norion runtime into
`crates/norion-memory`. It is a migration plan only: existing `src/**` modules
and real `.ndkv` files stay read-only until the integration window explicitly
switches ownership.

## Adapter Boundary

Adapters should live outside the legacy modules and implement the trait surface
owned by `norion-memory`.

- `ExperienceStoreAdapter`: reads `src::experience::ExperienceStore` records and
  projects them into `ExperienceEnvelope` for governance, index rebuild planning,
  and Context Rot gates.
- `DiskKvStoreOffloadAdapter`: wraps `src::disk_kv::DiskKvStore` behind
  `DiskKvOffload` for isolated cold KV shard reads/writes in test state. During
  migration it must open production `.ndkv` stores in read-only inspection mode
  or use copied fixtures under `target/...`.
- `ServiceMemoryAdapter`: reports a `MemoryAdapterDescriptor` and
  `MemoryAdapterHealth` so core, agent, and service code can discover whether
  short-term KV, long-term recall, governance, and KVSwap are ready.
  Root adapters should use `MemoryAdapterDescriptor::has_capability(...)`,
  `MemoryAdapterDescriptor::capability_codes()`,
  `MemoryAdapterHealth::with_warning(...)`, and
  `MemoryAdapterHealth::not_ready(...)` instead of hand-rolled status strings,
  so service readiness, adapter snapshots, and startup evidence normalize the
  same codes.
- `TieredCacheAdapter`: projects current `kv_cache`, `tiered_cache`, or
  `infini_memory` entries into `MemoryPlacementCandidate` values so placement
  can be planned before bytes are promoted, demoted, or evicted.
- `ExperienceReplayAdapter`: projects replayable root records into
  `ReplayCandidate` values, carrying reward, memory ids, scope, replay signals,
  and legacy feedback notes.
- `ContextInjectionAdapter`: projects retrieval matches into `ContextCandidate`
  values and calls `DefaultContextInjectionGate` before text is injected into a
  model prompt.

The shared adapter contracts are:

Minimum root startup wiring:

1. Build read-only root adapters for `ExperienceStore`, `DiskKvStore`, and any
   tier/cache projection source.
2. Call `ExperienceStoreAdapter::snapshot_summary()` and
   `DiskKvStoreOffloadAdapter::catalog_summary()` immediately after projection.
   These rows prove the adapter opened, projected counts, and surfaced adapter
   health warnings before the larger governance plan runs.
3. Pass the collected summaries through
   `MemoryServiceShadowPlanInputs::with_adapter_snapshots(...)` alongside the
   projected envelopes and KV metadata.
4. Treat any `adapter_snapshot_warnings` review reason or
   `adapter_snapshots_clean` checklist warning as a startup review gate. The
   per-row `adapter_snapshot` detail codes expose warning labels as
   `warning:<code>`, while the compact service detail codes are
   adapter-qualified, for example
   `adapter_snapshot:experience_shadow:adapter_unhealthy`, and do not expose
   prompt text, gist text, transcripts, KV bytes, or vectors.

Root field mapping checkpoints:

- `src::experience::ExperienceStore` exposes `records()` as a read-only slice
  of `ExperienceRecord`. The adapter should map `id`, `prompt`, `lesson`, and
  clamped `quality` directly into `ExperienceEnvelope` for shadow-read. It
  should derive `ExperienceProjectionHints` from `profile`, runtime diagnostics,
  memory mode, and model/device fields when present; `gist_records`,
  `gist_memory_ids`, `used_memory_ids`, and `stored_runtime_kv_memory_ids` are
  evidence tags only and must not become prompt text. Isolated-write readiness
  requires projection tags and task scope because root admission currently
  mutates records through hygiene/index hooks.
- `src::disk_kv::DiskKvStore` exposes `path()`, `len()`, `keys()`,
  `keys_with_prefix(...)`, `contains_key(...)`, and `get(...)`. A production
  `.ndkv` adapter must use only those read APIs and must not call `put`,
  `delete`, or `compact`. The append-only format starts with `NDK1` records and
  validates key/value length plus checksum while scanning; adapter projection
  should treat checksum or scan failures as `MemoryAdapterHealth::not_ready(...)`
  warning evidence rather than attempting repair in place.
- Copied fixture adapters may use `DiskKvShardKeyspace` to pair
  `.../metadata` and `.../bytes` keys, decode `KvShardMetadata`, and run
  `DiskKvCatalogVerification`. Production shadow-read may summarize catalog
  metadata and byte counts, but byte movement, tombstone-delete verification,
  and compaction-isolation checks belong to copied fixtures under `target/...`.
- Root adapter code can query
  `AdapterProjectionContract::required_field_codes_for(...)` and
  `recommended_field_codes_for(...)` to generate wiring manifests from the same
  matrix used by startup gates. Coverage reports expose both
  `missing_required_codes()` and `missing_recommended_codes()` so dashboards can
  show field gaps without copying prompt, lesson, gist, KV bytes, or vectors.
  `AdapterProjectionContract::manifest_line(...)` emits the mapped, required,
  and recommended field code sets plus a note count; it intentionally omits note
  text so path names, prompt fragments, or operator comments cannot leak into
  startup telemetry. `MemoryServiceDryRun::startup_evidence()` includes these
  `adapter_projection_contract` lines whenever projection contracts are supplied,
  but core/service preflights without root projection contracts can still be
  complete. The compact `memory_shadow` row also carries
  `projection_contract_manifests=` so consumers that only ingest the aggregate
  line can detect whether the field mapping contracts were recorded.
  `MemoryServiceStartupEvidence::summary_line()` reports
  `projection_contracts=`, `projection_contract_manifests=`, and
  `projection_contract_manifest_gap=`. If projection coverage rows are present
  but the matching per-adapter manifest rows are missing, startup evidence is
  incomplete and includes `adapter_projection_contract` plus a
  `projection_contract_manifest_gap:<count>` detail code.
  `AdapterProjectionContractBundle::manifest_summary_line()` adds a
  bundle-level count row (`adapter_projection_contract_bundle_manifest`) for
  startup logs that want to compare total mapped/required/recommended fields
  and normalized adapter codes before expanding every per-adapter manifest.

- `ExperienceSnapshotAdapter`: exposes `snapshot`, `snapshot_for_scope`,
  `index_documents`, and a default `snapshot_summary()` that emits the
  `adapter_snapshot` row from the adapter descriptor and projected experience
  count. The default summary folds `MemoryAdapterHealth::warnings` into
  `warning_codes()` and adds `adapter_unhealthy` when the provider reports
  `ready=false`, so projection success does not hide a lagging or unhealthy
  source adapter. `detail_codes()` mirrors those normalized warning labels with
  `warning:<code>` entries for per-row startup evidence.
- `AdapterSnapshotSummary`: records the adapter name, write mode, projected
  experience count, projected KV-shard count, warning count, and compact
  `status_codes()` before the full read-only plan is built. Root adapters should
  log its `adapter_snapshot` summary row when projection succeeds but before
  governance, placement, or KVSwap planning expands the evidence. Empty
  snapshots, live-write mode, and projection warnings become stable status or
  warning codes without exposing prompt, lesson, gist, or KV bytes; warning
  detail codes stay bounded to normalized labels only.
- `ExperienceProjectionHints`: attaches stable adapter, task profile, runtime
  model, device profile, and memory-mode tags to projected envelopes. If the
  legacy record has no task scope, the task profile can become the fallback
  `MemoryScope.task_id`.
- `AdapterProjectionContract`: declares which root fields an adapter maps before
  it is trusted by service startup. `ExperienceStore` shadow-read requires id,
  prompt, lesson, and quality; isolated-write also requires projection tags and
  task scope. `DiskKvStore` shadow-read requires shard id, bytes, metadata, and
  checksum; isolated-write also requires tier, priority, last-access,
  tombstone-delete, and compaction-isolation coverage. The resulting
  `AdapterProjectionCoverageReport` emits stable blocker and warning codes plus
  adapter-qualified blocker and warning detail codes for startup logs. Root
  service wiring should prefer the
  presets `experience_store_shadow`, `experience_store_isolated_write`,
  `disk_kv_store_shadow`, `disk_kv_copied_fixture`, `tiered_cache_shadow`, and
  `service_memory_shadow` when the adapter maps the standard field set. Use
  `AdapterProjectionContractBundle::standard_shadow()` for the normal read-only
  startup manifest and `copied_fixture_isolated_write()` for copied fixture
  write-path tests. `AdapterProjectionBundleReport` summarizes bundle-level
  readiness, review state, blockers, warnings, and adapter-qualified detail
  codes in one stable log line. `with_projection_contract_bundle(...)` preserves
  the bundle name so `MemoryServiceDryRun::startup_evidence()` logs the aggregate
  `adapter_projection_bundle` row before the per-adapter projection rows.
  `MemoryServiceStartupEvidence` exposes typed projection coverage and bundle
  counters for ready contracts, missing required/recommended fields,
  blockers/warnings, manifest mapped/required/recommended field totals, bundle
  review counts, and bundle manifest totals, so root dashboards do not need to
  parse projection summary text.
- `DefaultAdapterProjectionAuditor`: checks projected `ExperienceEnvelope` and
  `KvShardMetadata` values before service integration. It blocks shadow reads on
  empty/duplicate ids, empty experience content, and invalid KV priority; it
  warns on missing task scope, missing projection tags, risky records without
  clean gists, empty KV shards, missing checksums, and out-of-range priority.
  `AdapterProjectionAudit::summary_line()` emits the stable
  `adapter_projection_audit` startup row with shadow/isolated-write readiness,
  projected record counts, issue totals, and sorted issue codes.
  `MemoryServiceStartupEvidence` also exposes
  `projection_audit_issue_count()`, `projection_audit_blocker_count()`,
  `projection_audit_warning_count()`, `projection_audit_issue_codes()`, and
  `projection_audit_detail_codes()` so dashboards can aggregate duplicate-id,
  empty-content, and missing-clean-gist risks across startup rows without
  parsing the summary text or logging raw source ids.
- `MemoryServiceStartupEvidence`: reads the startup rows back into stable
  service-facing fields. In addition to reason/detail code helpers, it exposes
  `governance_record_count()`, duplicate/noisy/context-rot governance counters,
  `memory_rebuild_required()`, rebuild action counters, and
  `memory_repair_item_count()` plus per-action repair/skipped counters for
  experience-library hygiene. It also exposes
  `memory_index_operation_count()`, per-operation index counters, and
  `memory_index_skipped_count()` for index work. Finally, it exposes
  `context_injection_decision_count()`, admit/summarize/reject counters,
  `context_injection_accepted_risk_count()`, and
  `context_injection_used_tokens()` for Context Rot gating. The
  `context_injection_detail_codes_for_reason(...)`,
  `context_injection_reject_risk_detail_codes()`,
  `context_injection_reject_risk_detail_codes_for_reason(...)`,
  `context_injection_missing_clean_gist_count()`,
  `context_injection_raw_fallback_count()`, and
  `context_injection_truncated_index_content_count()` helpers separate clean
  gist absence from raw/truncated index fallback without logging raw
  prompt/lesson text. Infini/retention/compaction startup rows are also exposed
  through selected/token/skipped Infini counters, retention before/after
  decay/removal counters, and compaction before/after merge/removal/skipped
  helpers. Replay/evolution rows expose planned/reinforced/penalized/held
  replay counters, memory update and feedback counters, Context Rot signal
  counts, external feedback counts, replay missing/invalid-id counts, and drift
  rollback counts. Adapter checklist rows expose report/item totals,
  satisfied-report counts, blocker/warning summary codes, failed
  blocker/warning/info item counts, failed item codes, and per-item failed
  detail codes. `adapter_checklist_item_detail_codes_for(...)` and
  `adapter_checklist_failed_item_detail_codes_for(...)` let dashboards retrieve
  the detail codes for a single checklist item such as `context_rot_risks_clean`
  or `kvswap_boundary_clean` without parsing `memory_adapter_checklist_item`
  text. Projection contract and bundle rows expose ready/blocked
  contract totals, missing field totals, manifest coverage totals, bundle review
  counts, and blocker/warning detail codes. Adapter status and capability
  coverage rows expose readiness, write-mode, provider-health, provider-count,
  warning, and coverage status counters. Migration readiness and approval rows
  expose isolated-write readiness, review state, approved/blocked phase counts,
  required write-mode counts, and blocker/warning detail codes. These helpers
  aggregate across repeated startup rows and deduplicate code sets, keeping UI
  and service-review logic off raw prompt/lesson text.
  Daemon, Web Lab, and adapter consumers that need the index-quality bundle
  should consume typed accessors and stable row prefixes only:
  `experience_index_quality_gate_*` for blocker/warning pressure,
  `memory_index_*` for operation counts and hex-id detail labels,
  `context_injection_*` for admission decisions, and
  `migration_evidence_live_store_targeted_count()` plus
  `adapter_status_live_write_count()` for side-effect boundaries. Helper prose,
  operator notes, or suggestion rows are non-contractual; they must not be
  parsed to infer live-write intent, real `.ndkv` mutation, or startup
  admission. The focused consumer regression keeps a helper-prose line that
  mentions live write and real-store rewrite advice in the emitted bundle, while
  the consumer still reports zero live-write requests, zero store mutations, and
  reads index refresh work only from `memory_index_plan detail_codes=...`.
  `MemoryStartupAdmissionEvidence` packages that rule as a public pure-data
  view: it reads startup/index-quality/context-injection/migration fields from
  stable prefixes, counts helper prose and stale-window payload as
  non-contract lines, and always reports `.ndkv` write permission as false for
  this read-only boundary.
- `DiskKvShardManifest`: pairs `DiskKvShardKeyspace` bytes/metadata keys with a
  decoded `KvShardMetadata` record. `catalog_manifests` extracts metadata from
  mixed append-only key/value entries while skipping bytes and unrelated keys.
- `DiskKvCatalogVerification`: validates copied fixture catalogs by matching
  each manifest to its shard bytes, then reporting missing bytes, byte-length
  mismatches, and checksum mismatches. `MemoryMigrationEvidence` can be built
  directly from this verification result. Its `summary_line()` is the stable
  copied-fixture preflight log, separating catalog readiness from checksum
  readiness, hex-encoding mismatch ids, and exposing `reason_codes()` for
  `byte_len_mismatch`, `checksum_mismatch`, and `missing_bytes`.
  `detail_codes()` adds per-shard hex-id labels for those same conditions, and
  the summary line includes them as `detail_codes=...`, so copied-fixture review
  can point at the corrupt or missing shard without reading production `.ndkv`
  bytes. Root adapters must not copy raw shard ids or shard bytes into startup
  filters; pass only `disk_kv_catalog:*:<id_hex>` labels through
  `MemoryMigrationEvidence` and `MemoryServiceStartupEvidence`.
- `KvShardCatalogAdapter`: exposes `kv_metadata`, `placement_candidates`, and a
  default `catalog_summary()` that emits the `adapter_snapshot` row from the
  adapter descriptor and projected KV-shard count. It uses the same health
  warning and `adapter_unhealthy` propagation as `snapshot_summary()`. Any
  `DiskKvOffload + MemoryAdapter` can use the default implementation.
- `ReadOnlyMemoryPlan`: combines experience envelopes, KV metadata, current
  scope, budgets, and an optional previous tier plan into governance, repair,
  index, context injection, placement, and KVSwap intent outputs. Its
  `summary_line()` is the compact preflight evidence row for the whole
  read-only plan, covering governance noise, Context Rot, rebuild/repair/index
  counts, prompt-context decisions, tier placement counts, and KVSwap pending
  state. `reason_codes()` aggregates prefixed child codes from governance,
  rebuild, repair, index, context injection, and KVSwap so the compact row can
  be filtered without expanding every child line. `detail_codes()` is the
  matching trace surface for root adapters: it combines governance and rebuild
  per-record labels, repair/index/context hex-id labels, and KVSwap hex-id
  action labels while leaving normal clean upserts and admitted context out of
  the detail set. The compact `memory_read_only_plan` line now includes those
  same `detail_codes=` directly, so service/core startup logs can locate a
  cross-task transcript, missing clean gist, refresh target, or KVSwap shard
  action even when a dashboard has not expanded the child governance, index,
  context, or KVSwap lines.
  Context risk rejects preserve the full normalized risk set. A raw fallback
  candidate rejected by `missing_clean_gist` can still carry
  `raw_fallback_index_content` and `truncated_index_content` detail labels, so
  root adapters and dashboards can distinguish prompt-safety rejection from a
  simple retrieval miss.
  Startup evidence also expands the child summary lines from
  `GovernanceReport`, `IndexRebuildPlan`, `MemoryRepairPlan`,
  `MemoryIndexPlan`, and `ContextInjectionPlan` so operators can identify which
  policy layer requires review. Governance and rebuild summary lines include
  sorted reason codes such as `missing_clean_gist`,
  `cross_task_transcript_pollution`, and `repair_missing_or_dirty_clean_gist`.
  `IndexRebuildPlan` keeps `dirty_gist_ids` as the compatible combined repair
  set, and also splits it into `missing_clean_gist_ids` and
  `dirty_clean_gist_ids` so adapters can tell an absent gist from a projected
  gist that is itself unsafe.
- `MemoryServiceShadowSummary`: mirrors read-only Context Rot risks onto the
  compact `memory_shadow` startup row with `context_rot_risks=`,
  `context_rot_risk_reason_codes=`, and `context_rot_risk_detail_codes=`.
  `context_rot_risk_detail_codes_for_reason(reason)` and
  `context_rot_risk_reason_count(reason)` provide the same evidence directly on
  the summary object when dashboards already have the dry-run result in memory.
  Dashboards should prefer those fields over parsing `memory_governance` prose
  when they need stable risky-record evidence.
  The adapter checklist mirrors the same signal as `context_rot_risks_clean`,
  separate from `context_gate_clean`, so a UI can show whether prompt injection
  rejected items and whether the projected experience library itself contains
  Context Rot risks.
  It also mirrors non-clean KVSwap boundary audits as the warning item
  `kvswap_boundary_clean`, carrying `boundary_issues` and
  `boundary_reason_codes` detail fields so service review can distinguish
  hot/cold partition drift from prompt-safety or migration-evidence blockers.
  It also exposes `migration_evidence_ready` for copied-fixture write phases,
  carrying migration guard codes plus `disk_kv_catalog:*` detail codes from
  `DiskKvCatalogVerification`, including missing bytes, byte-length
  mismatches, and checksum mismatches.
  `MemoryServiceStartupEvidence` mirrors the same pattern for index, context,
  Infini, and KVSwap with accessors such as `memory_index_detail_codes()`,
  `memory_index_detail_codes_for_kind(...)`,
  `memory_index_detail_codes_for_reason(...)`,
  `memory_index_skipped_detail_codes()`,
  `context_injection_detail_codes()`, `infini_memory_detail_codes()`, and
  `kvswap_action_detail_codes()`, so adapter dashboards can filter planned index
  work, prompt rejection, prompt-placement, and hot/cold actions without parsing
  payload-bearing keys. For prompt-placement drilldown, prefer
  `infini_memory_detail_codes_for_scope(...)`,
  `infini_memory_skipped_detail_codes()`, and
  `infini_memory_skipped_detail_codes_for_reason(...)` over splitting the
  `infini_memory` summary text.
- `KvSwapIntent`: summarizes tier migrations as one compact intent line plus
  delegated prefetch/eviction plan lines, giving service startup a stable
  operator view before copied fixture bytes move. Its intent summary includes
  `reason_codes()` such as `prefetch_promote`, `prefetch_missing`,
  `evict_demote`, `evict_keep_hot`, and the delegated planner reasons.
- `KvEvictionPlan` and `KvPrefetchPlan`: expose stable `summary_line()` values
  with action counts and hex-encoded shard ids so service logs can explain
  copied-fixture movement without writing or parsing production `.ndkv` state.
  The summary rows also include `detail_codes=` for promote, missing,
  already-hot, duplicate, demote, and keep-hot decisions, so fixture dashboards
  can filter per-shard KVSwap actions without expanding Rust-side plan objects.
  Root adapters and UI filters should consume `*_id_hex=` or `detail_codes=`
  labels only; raw shard ids remain execution inputs and must not become
  startup-evidence fields.
  Their `reason_codes()` separate action labels such as `prefetch_missing` and
  `evict_keep_hot` from planner labels such as `requested_ids` or
  `target_hot_bytes`. `KvPrefetchPlan::detail_codes_for_action(...)`,
  `KvPrefetchPlan::detail_codes_for_reason(...)`,
  `KvEvictionPlan::detail_codes_for_action(...)`, and
  `KvEvictionPlan::detail_codes_for_reason(...)` provide the same drilldown
  before service startup evidence is assembled. Once rows are logged,
  `MemoryServiceStartupEvidence` mirrors the filters through
  `kvswap_prefetch_detail_codes_for_action(...)`,
  `kvswap_prefetch_detail_codes_for_reason(...)`,
  `kvswap_eviction_detail_codes_for_action(...)`,
  `kvswap_eviction_detail_codes_for_reason(...)`,
  `kvswap_action_detail_codes_for_stage(...)`,
  `kvswap_action_detail_codes_for_action(...)`, and
  `kvswap_action_detail_codes_for_reason(...)`.
- `KvSwapStateSnapshot`: exposes hot/cold shard counts, unique metadata count,
  and hot/cold byte totals from `KvSwapManager` metadata plus cold catalog
  metadata. It is for service inspection and startup diagnostics; it must not
  read cold shard bytes or trigger prefetch. `shape_codes()` adds stable labels
  for empty, hot-only, cold-only, and mixed-tier states so prefetch/eviction
  boundary checks can be automated. `KvSwapManager::plan_prefetch` must consult
  the same merged metadata view as `metadata(id)`, so a cold shard discovered
  only through the backend catalog is planned as `prefetch_promote` instead of
  `prefetch_missing`; the actual byte read still happens only in
  `prefetch(...)`. When that read succeeds, hot metadata is rebuilt from the
  returned bytes: byte length and checksum are recomputed and `last_access`
  moves forward from the cold metadata value, preventing stale copied-fixture
  catalog rows from poisoning later eviction order.
- `KvSwapBoundaryAudit`: checks the hot/cold boundary without reading or
  logging shard bytes. It flags overlapping hot/cold shard ids, hot bytes
  missing metadata, stale metadata, and hot/cold tier mismatches. Its
  `summary_line()` emits a `kvswap_boundary` row with issue counts,
  `reason_codes()`, and hex-id `detail_codes()`. Root adapters that own a
  `KvSwapManager` should pass `manager.boundary_audit()` through
  `MemoryServiceShadowPlanInputs::with_kvswap_boundary(...)`; non-clean audits
  produce `kvswap_boundary_review` in the compact service shadow summary.
  Dashboards should consume `kvswap_boundary_reason_codes()` and
  `kvswap_boundary_detail_codes()` for overlap, missing-hot-metadata,
  stale-metadata, hot-tier-mismatch, and cold-tier-mismatch review without
  logging raw shard ids or bytes.
- `MigrationReadinessReport`: summarizes whether a read-only plan is ready for
  isolated-write testing, and lists blockers or review warnings. Its
  `summary_line()` records isolated-write readiness, review state, blocker
  count, warning count, and normalized blocker/warning codes in startup
  evidence before per-phase migration approvals are evaluated.
  `MemoryServiceStartupEvidence` exposes the same readiness and per-phase
  approval rows through typed counters for ready/review states, approved versus
  blocked phases, required write modes, blocker/warning totals, and
  blocker/warning detail codes.
- `MemoryServiceManifest`: combines adapter descriptors, health, and write mode
  into a capability manifest. `MemoryReadinessReport` checks that core, agent,
  service, or shadow-migration consumers have the required healthy providers
  before the service attempts governance, retrieval, repair, or KVSwap work. Its
  `summary_line()` emits the `memory_readiness` startup row with the required
  write mode, missing capability, write-mode blocker, unhealthy adapter, and
  warning codes.
  For the in-process `AgenticMemory` bundle, `MemoryPorts::readiness_for(...)`
  is the narrow core/agent/service bridge: root startup can check a profile
  directly from the short-term, long-term, and skill ports before adding the
  larger governance, replay, context, DiskKV, KVSwap, and inspection adapters.
  `MemoryAdapterStatus::summary_line()` emits the `memory_adapter_status` row
  for each adapter so startup logs preserve descriptor capabilities, health
  warning codes, read-only/write-mode state, and record counts before aggregate
  readiness is trusted.
  `MemoryCapabilityCoverage::summary_line()` expands that aggregate into
  `memory_capability_coverage` rows so startup logs show which adapter providers
  are healthy, writable, read-only, or blocked for every required capability.
  `MemoryServiceStartupEvidence` reads those rows back through typed counters
  for adapter readiness, read-only/live/isolated write modes, capability and
  record totals, health warning codes, provider totals, missing providers,
  no-healthy-provider gaps, write-mode blockers, and multiple-provider review
  cases.
- `MemoryServiceShadowPlan`: the preferred first service dry run. It combines
  manifest readiness, projection audit, read-only governance/index/repair/context
  planning, tier placement, KVSwap intent, replay planning/reporting, Infini
  planning, retention, compaction, evolution gate, migration readiness,
  inspection output, and optional root projection parity audits into one object
  without mutating root state. `MemoryServiceShadowPlanInputs` accepts
  `ReplayCandidate` values; the generated `ReplayReport` is recorded into
  `MemoryEvolutionLedger` before the evolution gate runs. It also accepts
  `AdapterProjectionContract` values plus a target (`ShadowRead` or
  `IsolatedWrite`); the resulting coverage reports appear in the shadow summary,
  adapter checklist, and operator-review decision. The adapter checklist keeps
  the item code stable and now also emits normalized detail codes for failed
  blocker and warning items, preserving the original detail string for humans
  while giving dashboards a machine-readable reason surface.
- `ReplayReport`: emits `summary_line()` with stable replay counts for planned
  reinforce/penalize/hold actions, touched memory ids, memory update intent,
  feedback application stats, average reward, and high-signal replay coverage.
  `ReplayReport::reason_codes()` keeps action labels, memory-update labels,
  feedback gaps, and Context Rot replay samples machine-comparable across
  service dry runs. `ReplayReport::detail_codes()` keeps per replay item,
  memory-update intent, signal, and feedback-gap evidence machine-readable with
  hex-encoded ids only, so dashboards can trace affected records without logging
  lesson text, notes, transcripts, or memory contents. Context Rot replay
  samples use `signal:context_rot:<id_hex>` in the replay row and
  `replay:signal:context_rot:<id_hex>` when lifted into
  `MemoryServiceShadowSummary` or startup evidence; adapters must not copy the
  replay lesson or original experience prompt/lesson into either row.
  This line complements `MemoryEvolutionLedger`: the report explains the current
  dry-run replay plan, while the ledger explains the accumulated self-evolution
  gate input.
- `MemoryServiceShadowSummary`: compact service/operator evidence derived from
  `MemoryServiceShadowPlan`. It exposes stable counts and review reason codes
  for logs, startup diagnostics, and migration approval dashboards. Its compact
  `memory_shadow` line also carries `detail_codes=`, a sorted union of
  readiness, adapter-health, capability-coverage, projection-contract,
  read-only-plan, replay, migration-readiness, evolution, inspection,
  retention/compaction maintenance-action, and projection-parity labels. Clean
  no-op compaction skips such as `not_enough_entries` stay out of the compact
  detail set; actual decay/removal/merge work and explicit disabled policies
  keep hex-id traces for audit. Consumers that only ingest the compact line can
  still distinguish Context Rot review from adapter health or KVSwap review.
- `MemoryServiceDryRun`: one-shot service startup result containing the shadow
  plan, compact summary, migration evidence, and migration approvals for
  requested phases. Root integration should prefer this when it wants to log
  evidence and decide `ReadOnlyShadow` or `IsolatedWrite` readiness in one call.
  Its `startup_evidence()` result includes completeness helpers:
  `required_line_prefixes()`, `missing_required_line_prefixes()`,
  `missing_required_codes()`, `is_complete()`, and `summary_line()`. The summary
  line includes `missing_codes=` for dashboard filtering. It treats
  `adapter_projection_audit`, `memory_adapter_status`,
  `memory_capability_coverage`, and `memory_adapter_checklist_item` as required
  core lines, so service startup can see raw projection blockers, adapter health
  state, capability provider coverage, and per-item checklist detail codes
  before reading aggregate shadow reasons.
  Service startup should check these before treating a dry run as
  operator-reviewable, because missing evidence is a wiring problem even when
  individual gates look clean.
  The startup summary also mirrors the most common review channels as stable
  fields: `context_rot_risks=`, `context_rot_risk_reason_codes=`,
  `context_rot_risk_detail_codes=`, `migration_guard_codes=`,
  `migration_detail_codes=`, `kvswap_boundary_issues=`,
  `kvswap_boundary_reason_codes=`, and `kvswap_boundary_detail_codes=`.
  Root adapters and dashboards should prefer the matching
  `MemoryServiceStartupEvidence` accessors over scraping child row text when
  they need Context Rot, copied-fixture, or KVSwap boundary status.
  When the dry run requires operator review, startup evidence also lifts stable
  machine-readable detail codes from the compact `memory_shadow` line and from
  projection or migration blocker/warning detail-code fields. It also lifts
  `memory_migration_evidence detail_codes=guard:<code>` and
  `disk_kv_catalog:<verification_detail>` labels so copied-fixture, catalog,
  checksum, live-target, record-count, and per-shard verification gaps remain
  visible in the startup-level summary. The final `migration_detail_codes=`
  field keeps DiskKv catalog ids hex-only and must not include raw shard ids or
  copied KV bytes. It intentionally does not parse every
  child `detail_codes=` field, so checklist wording, prompt text, gist text, KV
  bytes, vectors, and memory contents cannot bleed into the startup-level filter
  surface. Projection-parity rows, repair/index plan rows, and KVSwap
  prefetch/eviction rows are the child-row exceptions: projection parity lifts
  field/warning labels only, while repair/index/KVSwap `detail_codes=` values
  are action or skipped labels with hex-only ids, so startup evidence lifts them
  when operator review is required.
  Projection-contract coverage has one additional completeness rule: when
  `adapter_projection` rows exist, each row needs a matching
  `adapter_projection_contract` manifest row. A positive
  `projection_contract_manifest_gap=` means the dry run is not complete even if
  the aggregate shadow plan is otherwise clean.
- `DefaultMemoryMigrationGate`: turns a `MemoryServiceShadowPlan` into explicit
  phase approval for `read_only_shadow`, `copied_fixture_write`,
  `isolated_write`, and `live_write`. It requires read-only source evidence for
  every phase, verified copied fixture evidence before write-path tests, a clean
  shadow plan before isolated writes, clean projection parity evidence before
  isolated writes, and keeps live writes disabled by default. Projection-contract
  evidence includes `MemoryMigrationEvidence::guard_codes()` for copied-fixture,
  catalog, checksum, live-target, and record-count gaps, while the evidence
  summary line mirrors them as `detail_codes=guard:<code>` for dashboard filters
  that should not parse approval text. DiskKv copied-fixture evidence also
  mirrors `disk_kv_catalog:<verification_detail>` labels from catalog
  verification, keeping corrupt, byte-length stale, or missing shard ids
  hex-only. Projection-contract coverage is carried into approval output as explicit
  `projection_contract_blocker:<adapter>:<code>` blockers and
  `projection_contract:<adapter>:<code>` warnings instead of being hidden behind
  a generic shadow-plan review flag. Approval summaries preserve those exact
  adapter/field details in `blocker_details` and `warning_details`, while
  normalized `blocker_codes` and `warning_codes` collapse them to categories
  like `projection_contract_blocker:missing_required` for operator filters.
- `DefaultMemoryRetentionPlanner`: projects root `kv_cache` entries into
  read-only stale-decay, weak-removal, and high-similarity compaction plans.
  It exposes `RetentionPlanning` and `CompactionPlanning` as read-only adapter
  capabilities so service startup can require them before cache maintenance.
  `MemoryRetentionPlan::summary_line()` and
  `MemoryCompactionPlan::summary_line()` give startup logs stable maintenance
  evidence before any live cache state is mutated, with `reason_codes()` keeping
  stale-decay, weak-removal, high-similarity-merge, and policy-skip labels
  machine-comparable. `detail_codes()` adds per-decay, per-removal, per-merge,
  and skipped-policy labels using hex-encoded memory ids only; adapters must not
  log cache keys, vectors, prompt text, or gist text from these maintenance
  rows.
- `DefaultInfiniMemoryPlanner`: projects active recall matches and durable cache
  entries into local-window, global-memory, and skipped prompt-memory sets. It
  preserves the root `src/infini_memory` capacity, token-budget, score, and
  redundant-key filtering concepts without depending on root cache types.
  `InfiniMemoryPlan::summary_line()` gives service startup a stable
  prompt-placement evidence line with local/global/skipped counts, token totals,
  selected memory count, and normalized reason codes for selected memory and
  sparse-filter skips.
- `DefaultMemoryEvolutionGate`: checks a storage-neutral
  `MemoryEvolutionLedger` before self-evolution write paths continue. It blocks
  missing replay evidence or excessive missing memory updates, and warns on high
  Context Rot counts, drift rollbacks, or unusually heavy maintenance.
  `MemoryEvolutionLedger::reason_codes()` and
  `MemoryEvolutionAssessment::{blocker_codes,warning_codes}` provide stable
  labels for those gate decisions while threshold values remain in the detailed
  blocker/warning strings. When service dashboards need compact metric evidence,
  `MemoryEvolutionAssessment::{blocker_detail_codes,warning_detail_codes}`
  emits normalized labels such as `memory_update_missing_ratio:0.800`,
  `context_rot_items:9`, and `drift_rollbacks:1`.
- `DefaultMemoryInspectionBuilder`: produces `MemoryInspectionSnapshot` from
  memory entries, adapter statuses, projection audit, readiness, and evolution
  assessment. This is the crate-level subset of `src/state_inspect/*` for
  memory-layer service logs and operator review. Its summary includes
  `risk_codes()` so projection, readiness, and evolution risks can be compared
  without parsing severity/count detail. `risk_detail_codes()` gives compact
  `severity:risk:count` labels for dashboards that need the exact inspection
  severity and occurrence count without exposing adapter detail text, prompts,
  memory contents, or vectors.
- `DefaultRuntimeStateProjector`: exposes storage-neutral projection helpers for
  root `adaptive_state` and `state_inspect` parity checks. Root adapters should
  fill `AdaptiveStateMemoryProjection` and `StateInspectionMemoryProjection`,
  pass them into `MemoryServiceShadowPlanInputs`, and let the shadow plan merge
  them into `projection_parity_audit`. `MemoryProjectionAudit` mismatches or
  missing core inspection counts keep the service in review mode. Its
  `memory_projection_parity` summary row is required startup evidence; the row
  exposes mismatch/warning counts and `detail_codes=` field labels, but not
  expected/actual counter values.
- `AdapterWriteMode`: marks live adapters as read-only, isolated-write, or
  live-write. Production `.ndkv` migration starts in read-only mode.

## ExperienceStore Mapping

`ExperienceRecord` maps to `ExperienceEnvelope` as follows:

- `id`: `record.id.to_string()`.
- `prompt`: `record.prompt`.
- `lesson`: `record.lesson`.
- `quality`: clamped `record.quality`.
- `clean_gist`: project legacy `GistRecord` values into `MemoryGist` and call
  `DefaultCleanGistSelector::best_clean_gist`, preferring high-importance clean
  summaries that are not transcript-shaped and not legacy metadata-shaped.
  Service manifests can additionally require the read-only
  `clean_gist_selection` capability from `DefaultCleanGistSelector`; the
  `GistMemory` projection contract proves source fields are present, while the
  capability proves the selector policy is wired into readiness.
  Log `CleanGistSelectionReport::summary_line()` when auditing a batch of
  projected gists; it records candidate/rejection counts and selected structural
  metadata without writing gist text into startup evidence.
- `tags`: profile, route budget, reward action, runtime model id, adapter, device
  profile, memory mode, and any high-signal reflection issue codes.
- `scope.task_id`: a stable task/profile key. If no explicit task id exists,
  derive one from `TaskProfile` plus route/runtime lane metadata.
- `scope.session_id`: runtime session id when available; otherwise leave unset.
- `scope.agent_id`: service or agent role when available; otherwise leave unset.

The adapter should expose two modes:

- `snapshot() -> Vec<ExperienceEnvelope>` for full governance and rebuild plans.
- `snapshot_for_scope(scope: &MemoryScope)` for retrieval-time Context Rot checks
  before experiences are injected into a model prompt.
- `index_documents() -> Vec<MemoryIndexDocument>` to project governed envelopes
  into a storage-neutral index plan without writing the legacy store.

Implementation boundary:

- Production `ExperienceStore` adapters start as `ReadOnly` providers for
  `ExperienceGovernance`, `MemoryIndex`, `ContextInjection`, `ExperienceReplay`,
  `RetentionPlanning`, `CompactionPlanning`, `MemoryEvolution`, and
  `StateInspection` evidence. They should not implement live mutation while
  governance can still produce `missing_clean_gist`, `dirty_clean_gist`,
  `cross_task_transcript_pollution`, or duplicate-id rebuild work.
- Copied fixtures may expose `IsolatedWrite` only after the projection contract
  has all required fields, task-scope warnings are resolved, replay evidence has
  at least one clean run when the migration phase requires it, and
  `MemoryMigrationEvidence` points at the fixture instead of the live `.ndkv`.
- Agent retrieval should use `snapshot_for_scope` plus
  `DefaultContextInjectionGate` before prompt assembly. The adapter should never
  return raw cross-task transcript records as already-approved context; those
  records remain candidates until governance admits, summarizes, or rejects them.
  Raw fallback candidates without a selected clean gist default to prompt-time
  `reject_risk:missing_clean_gist`, even when their task scope matches the
  current request; adapters should surface the stable rejection and Context Rot
  reason codes instead of treating the fallback text as approved prompt input.

## Governance Flow

The service path should call governance before retrieval results become prompt
context:

1. Read `ExperienceStore` records into envelopes.
2. Call `ExperienceGovernance::assess_for_scope(records, current_scope)`.
3. Drop or quarantine records with `cross_task_transcript_pollution`,
   `dirty_clean_gist`, or high noise scores from retrieval candidates.
4. Call `DefaultExperienceGovernance::rebuild_plan_for_scope` in planning mode to
   produce repair, compact, refresh, and quarantine ids. Use
   `GovernanceReport::reason_codes()` and `IndexRebuildPlan::reason_codes()` as
   the stable operator labels for why a record was held back or scheduled for
   repair.
5. Apply repairs only in a later migration step after the plan has been reviewed
   against copied fixtures.

This keeps governance deterministic and makes the Context Rot gate independent
from the exact storage backend.

## Read-Only Planning Flow

The first service integration can build one `ReadOnlyMemoryPlan`:

1. `ExperienceSnapshotAdapter::snapshot_for_scope(current_scope)` reads
   candidate experiences.
2. `KvShardCatalogAdapter::kv_metadata()` reads cold/hot shard metadata.
3. Root adapters apply `ExperienceProjectionHints` so envelopes carry stable
   adapter/profile/runtime tags and a task scope fallback.
4. Root adapters declare an `AdapterProjectionContract` and log its
   `coverage_report(AdapterProjectionTarget::ShadowRead)`. Use the standard
   preset constructors when the adapter covers the expected root field set, and
   fall back to explicit fields only while a migration is incomplete. Missing
   required fields block service startup; missing recommended fields keep the
   run in operator-review mode. Prefer passing the contracts into
   `MemoryServiceShadowPlanInputs::with_projection_contracts`, or pass
   `AdapterProjectionContractBundle::standard_shadow()` through
   `with_projection_contract_bundle`, so contract coverage is part of the same
   dry-run evidence as governance, replay, KVSwap, and migration approval.
5. `DefaultAdapterProjectionAuditor` validates projected envelopes and KV
   metadata. Shadow migration can continue only when blockers are absent;
   isolated-write should also resolve missing task scope warnings.
6. Gist adapters project legacy gist records into `MemoryGist` and attach the
   selected clean gist before governance.
7. `ReadOnlyMemoryPlan::for_inputs(...)` runs governance, rebuild planning,
   repair planning, index planning, context injection planning, tier placement,
   and KVSwap intent generation. Its `reason_codes()` should be logged with the
   compact read-only evidence row so review causes are visible before the child
   rows are expanded. Use `detail_codes()` when dashboards need the exact
   affected projected experience id, context candidate id, index source id, or
   KV shard id; ids are hex encoded except governance detail labels, which
   mirror the projected experience id already visible in governance parity
   output.
   `ReadOnlyMemoryPlan::summary_line()` is the adapter-facing parity row for
   root `ExperienceStore::{hygiene_report,index_report,...}` comparisons: it
   preserves prefixed `index:` and `context:` reason/detail codes from
   `MemoryIndexPlan` and `ContextInjectionPlan`, but consumers must treat
   prompt, lesson, gist, and shard bytes as payload and never parse or filter on
   them.
8. `DefaultContextInjectionGate` rejects cross-task, risky, low-score, or
   over-budget context candidates before prompt assembly.
9. `DefaultInfiniMemoryPlanner` plans local-window/global-memory/skipped prompt
   placement for active matches and durable runtime cache entries before prompt
   assembly. `InfiniMemoryPlan::reason_codes()` records stable labels such as
   `local_window`, `global_memory`, `sparse_filter:low_score`, and
   `sparse_filter:redundant_key_overlap`.
10. `DefaultExperienceReplayPlanner` can run on replay candidates to produce
   reinforce, penalize, or hold batches plus `ReplayMemoryUpdate` intents.
   Service startup should pass projected candidates into
   `MemoryServiceShadowPlanInputs::with_replay_candidates` so the shadow dry run
   can emit `ReplayPlan`, `ReplayReport`, replay counts in
   `MemoryServiceShadowSummary`, and evolution-gate evidence in one object.
11. `apply_replay_updates_to_long_term` can apply approved replay memory updates
   to a `LongTermMemory` adapter in isolated tests before any live write path.
12. `DefaultMemoryRetentionPlanner` plans cache decay/removal/compaction from
   projected runtime KV, gist, and semantic memory entries before any live cache
   mutation. Its summary lines include `detail_codes=` so copied-fixture review
   can trace the affected memory ids without exposing cache keys or memory text.
13. `MemoryEvolutionLedger` records replay, replay-apply, retention,
   compaction, external feedback, Context Rot, and drift rollback evidence.
14. `DefaultMemoryEvolutionGate` decides whether isolated-write self-evolution
   can continue or whether operator review/rollback is required.
15. `DefaultMemoryInspectionBuilder` emits a `MemoryInspectionSnapshot` and
   summary line that includes memory counts, runtime-KV counts, vector-dimension
   buckets, top memories, adapter health, projection risks, readiness blockers,
   and evolution risks.
16. The service logs `requires_operator_review()` and the detailed plan.
17. `MigrationReadinessReport::from_plan` decides whether isolated-write testing
   is allowed and which warnings still need operator review.
18. Call `MemoryServiceShadowPlan::migration_approval(...)` for the requested
    phase. `ReadOnlyShadow` may be approved while still requiring operator
    review; `CopiedFixtureWrite` requires copied `.ndkv` or equivalent fixture
    evidence with catalog and checksum verification; `IsolatedWrite` additionally
    requires no shadow-plan blockers, review warnings, or projection parity
    mismatches; `LiveWrite` remains blocked by the default policy. When
    projection contracts are provided, approval warnings/blockers include the
    adapter name and missing field code so service startup can explain exactly
    which root mapping is not ready.
19. Project root `adaptive_state` and `state_inspect` evidence into
    `AdaptiveStateMemoryProjection` and `StateInspectionMemoryProjection`.
    Pass those projections into `MemoryServiceShadowPlanInputs`; the resulting
    `projection_parity_audit` must be clean, or the service should keep the run
    in shadow/review mode.
20. Only copied fixtures or isolated stores should be used for write-path tests.

For a full service dry run, use `MemoryServiceShadowPlan::for_inputs(...)`
instead of manually calling each planner. It accepts projected experiences,
KV metadata, cache entries, active Infini matches, adapter statuses, scope,
tier/KV budgets, maintenance policies, protected memory ids, an optional seed
`MemoryEvolutionLedger`, and optional root adaptive/state-inspection projections.
The resulting plan should be logged as the single migration evidence object
  before any isolated write path is enabled; clean copied fixtures should show
  `guard_codes=none detail_codes=none`, while incomplete fixtures should expose
  copied-fixture, catalog, checksum, live-target, or record-count guard codes and
  `guard:<code>` detail labels. DiskKv fixture failures also carry
  `disk_kv_catalog:<verification_detail>` labels for missing bytes, byte-length
  drift, or checksum drift. Service evidence consumers should read
  `migration_evidence_detail_codes()`/`migration_detail_codes=` for those three
  DiskKv catalog failure classes instead of parsing human text or raw shard ids.
  Service logs should prefer
  `MemoryServiceDryRun::startup_evidence().summary_text()` so operators can
  compare stable counts, the requested service requirement profile/capability
  row, the read-only plan evidence row, adapter checklist state,
  governance/rebuild/repair/index/context evidence, projection audit issue codes,
  projection-contract coverage, replay report evidence, Infini prompt-placement
  evidence, retention and compaction maintenance evidence, inspection state,
  migration evidence, KVSwap intent, optional KVSwap state snapshots, migration
  readiness, and migration approval codes without decoding the full plan.
`MemoryServiceShadowSummary::reason_codes()`
duplicates the sorted review reasons into a stable code field for dashboards
that only ingest the compact `memory_shadow` line. Use
`MemoryServiceShadowSummary::detail_codes()` when the dashboard needs the
underlying gate labels without expanding the full dry-run plan. Read-only-plan
details appear under `read_only_detail:*`, separate from broader review reason
categories such as `read_only:context:cross_task_scope`; replay details appear
under `replay:*` with hex ids only. Projection-audit issue details appear under
`projection_detail:<severity>:<issue>:source_id_hex:<hex>` when a projected
experience or KV shard id is known, avoiding prompt, lesson, gist, vector, or KV
bytes leakage. When `with_kvswap_state(manager.state_snapshot())` is supplied,
the same `memory_shadow` line includes `kvswap_state=true`, hot/cold shard
counts, metadata count, total bytes, and snapshot shape codes. Missing snapshots
stay explicit as `kvswap_state=false` and do not add review reasons; the review
gate should still be driven by KVSwap intent, catalog verification, migration
readiness, and projection/audit blockers. When
`with_kvswap_boundary(manager.boundary_audit())` is supplied, non-clean boundary
checks add `kvswap_boundary_review`, include `kvswap_boundary_issues=...` and
`kvswap_boundary_reason_codes=...` on `memory_shadow`, and emit a
`kvswap_boundary` child row whose `detail_codes=` are lifted into startup
evidence.
When all standard read-only adapters are present, pass
`AdapterProjectionContractBundle::standard_shadow()` into
`MemoryServiceShadowPlanInputs::with_projection_contract_bundle(...)` so the
ExperienceStore, DiskKvStore, gist-memory, Infini-memory, KV-cache,
tiered-cache, and service-memory field policies stay in one manifest. Use
`copied_fixture_isolated_write()` only with copied
ExperienceStore/DiskKvStore fixtures under test state. Log
`coverage_summary().summary_line()` before expanding the bundle into service
dry-run inputs when startup wants a manifest-level preflight line. When callers
use `with_projection_contract_bundle(...)`, the same aggregate line is retained
in `MemoryServiceShadowPlan::projection_bundle_summary` and included in service
startup evidence.
Coverage reports and bundle summaries expose `blocker_codes()` and
`warning_codes()` so missing required fields, write-mode drift, missing
recommended fields, and adapter notes can be filtered without parsing full
warning text.
Individual `AdapterProjectionCoverageReport` values also expose
`blocker_details()` and `warning_details()`, plus
`blocker_detail_codes()` and `warning_detail_codes()` for compact machine
filters. Bundle reports aggregate those via adapter-qualified detail strings, so
operator UIs can show exactly which ExperienceStore or DiskKvStore projection
contract is missing a field while still filtering by the normalized code. These
details name adapter contracts and field enums only; they must not include
prompt text, KV bytes, gist text, vectors, or projected memory contents.
Coverage and bundle summary lines include `blocker_detail_codes=` and
`warning_detail_codes=` so startup dashboards do not need to expand every child
report just to identify a missing field.
`MigrationReadinessReport::summary_line()` follows the same split: normalized
`blocker_codes` and `warning_codes` are stable filter categories, while
`blocker_details` and `warning_details` preserve the raw count-bearing evidence
for operator review. `blocker_detail_codes()` and `warning_detail_codes()` keep
bounded counters such as `context_rejections=1` while reducing any free-form
value to a stable key. Service shadow summaries carry the same bounded counters
under `migration_readiness_*_detail:*`, so dashboards can inspect the compact
`memory_shadow` row without expanding child migration-readiness output.
`MemoryMigrationApproval::{blocker_detail_codes,warning_detail_codes}` follows
the same non-leaking evidence rule for phase approvals: adapter and field labels
remain readable, projection source ids are hex encoded, and free-form mismatch
values are reduced to the affected field or bounded metric key.
For startup code that needs phase approvals immediately, use
`MemoryServiceDryRun::for_inputs(...)` with the desired phases and migration
evidence. It returns the full plan, summary, and approvals, plus helpers for
approved phases, phase lookup, adapter checklist generation, and startup
evidence lines. After building the evidence, call `is_complete()` and inspect
`status_codes()` before shipping logs to the operator UI; expand
`missing_required_line_prefixes()` and `detail_codes()` only when a row is
missing, review is required, or phase approvals are absent.
Use `MemoryServiceStartupEvidence::emit_to(...)` with a
`MemoryStartupEvidenceSink` when root service code wants to append the stable
startup rows and the final `memory_startup_evidence` row to a logger, telemetry
buffer, or test vector without duplicating the emission order.

Before step 1, service startup should build a `MemoryServiceManifest` and check
`MemoryServiceRequirement::for_profile(MemoryConsumerProfile::ShadowMigration,
AdapterWriteMode::ReadOnly)`. This catches missing governance, index, repair,
context, retention, compaction, placement, DiskKv, or KVSwap adapter coverage
before any prompt-time, cache-maintenance, or repair-time logic runs. The next
gate is
`MemoryConsumerProfile::Service` with `AdapterWriteMode::IsolatedWrite`, which
should fail whenever the only provider for a required capability is still a
read-only production `.ndkv` adapter.
`MemoryServiceDryRun::startup_evidence()` emits
`MemoryServiceRequirement::summary_line()` before the readiness report, and
`MemoryServiceStartupEvidence::missing_required_line_prefixes()` treats that row
as required. Core, agent, service, and shadow-migration launches should use the
same line shape when they log standalone readiness checks outside a full dry run.
The dry-run defaults to
`MemoryConsumerProfile::ShadowMigration` plus `AdapterWriteMode::ReadOnly`.
After the shadow evidence is clean, call
`MemoryServiceShadowPlanInputs::with_requirement(...)` with
`MemoryConsumerProfile::Service` and `AdapterWriteMode::IsolatedWrite` to reuse
the same projected experiences, KV metadata, replay inputs, migration evidence,
and adapter contracts for the next service preflight gate.
Pass adapter projection summaries from `ExperienceSnapshotAdapter::snapshot_summary()`
and `KvShardCatalogAdapter::catalog_summary()` through
`MemoryServiceShadowPlanInputs::with_adapter_snapshots(...)` so startup evidence
contains the `adapter_snapshot` rows next to readiness and projection-contract
rows. Clean snapshots are informational; snapshot warnings keep the shadow plan
in operator review. The source row carries `detail_codes=warning:<code>`, and
the compact `memory_shadow` line additionally exposes
`adapter_snapshot:<adapter>:<warning>` detail codes without logging projected
record contents. They also fail the warning-level
`adapter_snapshots_clean` checklist item, giving service UIs one checklist row
for unhealthy or lagging projection sources.
When the service owns a `KvSwapManager`, pass
`with_kvswap_state(manager.state_snapshot())` into the dry-run inputs so startup
evidence includes a `kvswap_state` row. This row is optional because pure
projection tests may only have KV metadata, but service-mode shadow runs should
prefer it before deciding whether a `prefetch_missing` item is a true catalog
gap or simply an unreported cold-only state.

## DiskKvStore Mapping

`DiskKvStore` is append-only and key-addressed. `DiskKvOffload` is shard-oriented.
The adapter should use `DiskKvShardKeyspace` key prefixes instead of changing the
`.ndkv` record format.

- Cold shard bytes: `DiskKvShardKeyspace::keys_for(id).bytes_key`.
- Metadata: `DiskKvShardKeyspace::keys_for(id).metadata_key`.
- Catalog: derive from `keys_with_prefix(DiskKvShardKeyspace::catalog_prefix())`.
  Feed mixed key/value entries through
  `DiskKvShardKeyspace::catalog_manifests`; it returns sorted
  `DiskKvShardManifest` values and rejects duplicate metadata rows.
- Fixture verification: for copied stores, feed materialized metadata and bytes
  entries through `DiskKvShardKeyspace::verify_catalog_entries`. The resulting
  `DiskKvCatalogVerification` must have no missing bytes, byte-length
  mismatches, or checksum mismatches before
  `MemoryMigrationPhase::CopiedFixtureWrite` or
  `MemoryMigrationPhase::IsolatedWrite` is approved. Log
  `DiskKvCatalogVerification::summary_line()` with the migration evidence so
  operators can distinguish missing bytes, byte-length drift, and checksum
  drift. The same row includes `reason_codes=` and hex-only `detail_codes=` so
  automated gates can distinguish stale byte lengths, corrupt checksums, and
  absent shard bytes. `MemoryServiceStartupEvidence` exposes
  `disk_kv_catalog_missing_bytes_count()`,
  `disk_kv_catalog_byte_len_mismatch_count()`,
  `disk_kv_catalog_checksum_mismatch_count()`, and
  `disk_kv_catalog_detail_codes_for(...)` so UI code can aggregate copied
  fixture catalog failures across multiple migration evidence rows without
  parsing migration detail text or logging raw shard ids. Duplicate detail codes
  collapse to one stable issue so retries and partial adapter passes do not
  inflate operator-facing counts.
- Delete: write tombstones through `DiskKvStore::delete` for both bytes and
  metadata keys.
- Compact: never compact production stores from the adapter. Compact only copied
  stores under `target/...` during tests.

Metadata can be serialized as a compact line format until the root workspace
chooses a typed storage dependency. Use `serialize_kv_metadata` and
`deserialize_kv_metadata` rather than hand-writing parser logic:

```text
id_hex=<hex> byte_len=<n> checksum=<u64> tier=cold priority=<f32> last_access=<u64>
```

The adapter must verify checksum on read and return `MemoryError::InvalidInput`
for mismatched shard bytes. `id_hex` is UTF-8 hex encoding so shard ids can
contain spaces, slashes, or task-specific separators without breaking `.ndkv`
keys. The metadata id decoded from the line must match the shard id decoded from
the metadata key; a mismatch indicates stale or corrupt catalog state and should
block migration.

Implementation boundary:

- Production `DiskKvStore` adapters start as `ReadOnly` providers for
  `DiskKvOffload` and `KvSwap` catalog evidence. They may list metadata and
  verify copied materialized bytes, but they must not compact, delete, or rewrite
  live shard keys during shadow migration.
- Copied fixtures may expose `IsolatedWrite` after
  `DiskKvCatalogVerification::is_verified()` passes and
  `MemoryMigrationEvidence` carries copied-fixture, isolated-root, catalog, and
  checksum guards. The fixture adapter can then exercise `write_cold_shard`,
  `read_cold_shard`, and `delete_cold_shard` through `KvSwapManager` without
  touching production `.ndkv` bytes.
- Root service dry-runs should pass the projected `kv_metadata` into
  `ReadOnlyMemoryPlan`, pass `manager.state_snapshot()` through
  `with_kvswap_state(...)` when a manager exists, and include
  `DiskKvCatalogVerification::summary_line()` in migration evidence for copied
  fixtures. This gives operators three separate signals: intended movement,
  observed hot/cold state, and fixture byte integrity.

## Tiered Cache Mapping

Existing `MemoryTier::{HotGpu,WarmRam,ColdDisk}` maps directly to
`norion-memory::MemoryTier`. Runtime KV metadata maps through
`MemoryPlacementCandidate`:

- `id`: stable runtime memory id or KV shard id.
- `byte_len`: known KV shard byte length, or an estimated tensor/cache footprint.
- `priority`: current retention score, process reward influence, or normalized
  cache value score.
- `last_access`: runtime clock or access counter.
- `current_tier`: existing tier if known.

`DefaultTieredMemoryPlanner` produces `TieredMemoryPlan`; callers compare it
with the previous plan to get `TierMigration` actions. These actions should feed
KVSwap planning, not write storage directly. `KvSwapIntent::from_migrations`
splits migration output into prefetch promotions and cold demotions so service
code can call KVSwap deliberately. Before executing those calls against copied
fixtures, log `KvEvictionPlan::summary_line()` and
`KvPrefetchPlan::summary_line()`; shard ids are hex encoded so scoped ids with
spaces or slashes remain one-line-safe. `KvSwapIntent::summary_lines()` emits
the compact intent line plus both delegated plan lines, which is what
`MemoryServiceStartupEvidence` records. The compact intent line carries
movement reason codes; `KvEvictionPlan` and `KvPrefetchPlan` now carry matching
action/planner reason codes while keeping actual shard ids hex-encoded for
copied-fixture review. Prefetch plans distinguish cold promotions, missing ids,
already-hot ids, and duplicate requests, so copied-fixture logs can separate a
real cold-catalog gap from a harmless repeated or already-hot request.
`KvSwapManager::stage_hot(...)` is a replacement operation for a shard id: if a
matching cold catalog entry exists, it is removed before the new hot metadata is
recorded, which keeps `KvSwapStateSnapshot` from reporting a stale mixed-tier
duplicate. Execution of manual or adapter-supplied plans also deduplicates
repeated promote/demote ids and leaves missing cold shards without hot
metadata. When service code owns a live
`KvSwapManager`, it can additionally log `KvSwapStateSnapshot::summary_line()`
before and after copied-fixture movement to prove hot/cold counts changed
without reading production cold bytes. `MemoryServiceStartupEvidence` exposes
`kvswap_state_present()`, `kvswap_state_hot_shard_count()`,
`kvswap_state_cold_shard_count()`, `kvswap_state_metadata_count()`,
`kvswap_state_total_byte_len()`, and `kvswap_state_shape_codes()` so UI code can
consume either the dedicated `kvswap_state` row or the compact `memory_shadow`
fields without parsing raw summary text. Those state accessors are safe to use
alongside `kvswap_boundary_*` accessors when a service startup needs both the
observed hot/cold shape and a boundary-review issue set in the same row set.
Boundary review also has per-issue accessors:
`kvswap_boundary_report_count()`,
`kvswap_boundary_clean_report_count()`,
`kvswap_boundary_review_count()`,
`kvswap_boundary_overlap_count()`,
`kvswap_boundary_missing_hot_metadata_count()`,
`kvswap_boundary_stale_metadata_count()`,
`kvswap_boundary_hot_tier_mismatch_count()`,
`kvswap_boundary_cold_tier_mismatch_count()`, and
`kvswap_boundary_detail_codes_for(...)`. Dedicated `kvswap_boundary` rows are
summed across repeated startup evidence lines, while the clean/review report
counts let UI code tell "no boundary audit supplied" apart from "audit supplied
and clean" and "audit supplied but requiring review". When no dedicated row exists,
the compact `memory_shadow` boundary issue count remains the fallback.
`KvPrefetchPlan::detail_codes()` and `KvEvictionPlan::detail_codes()` provide
sorted hex-id action labels for promote, missing, already-hot, duplicate,
demote, and keep-hot decisions when service dashboards need per-shard
traceability without parsing the full summary row. `MemoryServiceStartupEvidence`
also exposes typed action counters and reason/detail code helpers for both
plans: `kvswap_prefetch_promote_count()`,
`kvswap_prefetch_missing_count()`, `kvswap_prefetch_already_hot_count()`,
`kvswap_prefetch_duplicate_count()`, `kvswap_eviction_demote_count()`,
`kvswap_eviction_keep_hot_count()`, and `kvswap_eviction_target_hot_bytes()`.
These counters aggregate repeated startup rows while preserving hex-id detail
codes for deeper copied-fixture review.

## Gist, Infini, And KV Cache Contracts

The shadow-read projection manifest includes separate contract kinds for the
memory sources that feed prompt assembly and cache maintenance:

- `GistMemory` requires gist id, selected gist text, and source experience id;
  gist importance is recommended so clean-gist selection can be audited against
  legacy `gist_memory` ranking.
  `DefaultCleanGistSelector::selection_report(...)` emits
  `clean_gist_selection` rows with `reason_codes=` and payload-free
  `detail_codes=` such as `selected_level:section`, `rejected_transcript`, and
  `rejected_metadata`. Root adapters should log those labels for clean-gist
  coverage and rejection review, while treating gist title, gist summary, and
  source prompt text as payload. Service dry-runs can carry those rows through
  `MemoryServiceShadowPlanInputs::with_clean_gist_selection_reports(...)` so
  startup evidence records whether `gist_memory` produced usable clean summaries
  before prompt assembly or index refresh trusts them.
  `MemoryServiceStartupEvidence::clean_gist_selection_report_count()`,
  `clean_gist_selection_candidate_count()`,
  `clean_gist_selection_selected_count()`,
  `clean_gist_selection_no_selection_count()`,
  `clean_gist_selection_selected_level_codes()`,
  `clean_gist_selection_selected_level_count(level)`,
  `clean_gist_selection_rejected_empty_count()`,
  `clean_gist_selection_rejected_transcript_count()`,
  `clean_gist_selection_rejected_metadata_count()`,
  `clean_gist_selection_rejected_low_signal_count()`,
  `clean_gist_selection_reason_codes()`, and
  `clean_gist_selection_detail_codes()` are the stable UI-facing accessors.
  `clean_gist_selection_detail_codes_for(prefix)` gives adapter-facing drilldown
  for labels such as `selected_level:*` and `selected:none`; UI code should not
  parse gist titles or summaries from raw evidence text.
- `InfiniMemory` requires item id, local/global scope, and score; token estimate
  is recommended so prompt-placement budget drift can be reviewed without
  parsing individual entries.
- `KvCache` requires cache entry id, vector/value projection, and strength;
  namespace and last-access metadata are recommended so retention, compaction,
  and tier movement can be compared with root `kv_cache` and `tiered_cache`.

These contracts do not mutate stores. They make root adapters declare whether
they have enough read-only data to drive `DefaultCleanGistSelector`,
`DefaultInfiniMemoryPlanner`, and `DefaultMemoryRetentionPlanner` before the
service trusts startup evidence.

## Infini Memory Mapping

Existing `src/infini_memory` concepts map directly to the crate boundary:

- Active recall matches map to `InfiniMemoryActiveMatch`.
- Durable cache entries map through `RetentionMemoryEntry`.
- `LocalWindow`, `GlobalMemory`, and `Skipped` map to `InfiniMemoryScope`.
- Capacity limits, token budgets, minimum scores, and redundant-key overlap map
  to `DefaultInfiniMemoryPlanner` fields.

The planner decides prompt-memory placement only. It does not promote bytes,
write cache state, or mutate `.ndkv` catalogs. Tier movement remains owned by
`DefaultTieredMemoryPlanner` and `KvSwapIntent`. Service startup should log the
resulting `InfiniMemoryPlan::summary_line()` through
`MemoryServiceDryRun::startup_evidence()` before prompt assembly is trusted.
The line includes `reason_codes()` so prompt-placement reviews can distinguish
capacity, token-budget, redundancy, low-score, and missing-entry skips without
parsing score details from individual items. It also carries `detail_codes()`
with hex-encoded selected/skipped ids, so operator evidence can trace placement
decisions without logging memory keys or prompt text.
Adapter-facing tests should treat `InfiniMemoryActiveMatch::key` and durable
memory keys as payload, using only the hex-id `detail_codes=` values for
startup filters.

## Index And Rebuild Integration

The existing `ExperienceStore::{hygiene_report,index_report,legacy_metadata_repair_plan}`
remain useful during migration. The new crate should become the service-facing
policy boundary:

- `ExperienceIndexReport` can be compared against `IndexRebuildPlan` to validate
  that noisy or rotting envelopes point at the same records. Root adapters can
  use `ExperienceIndexFindingProjection` to normalize root finding reasons such
  as `duplicate_output`, `unstructured_long_transcript`,
  `overlong_single_document_without_clean_gist`, and
  `legacy_metadata_lesson_missing_clean_gist` into the `MemoryIndexPlan` rebuild
  reasons, operation reasons, and `ContextInjectionPlan` risk reasons that
  startup evidence already exposes.
- `ExperienceRepairPlan` can consume `dirty_gist_ids` and `compact_ids` once a
  write path is approved.
- `DeduplicationReport` gives the first exact-fingerprint pass before any vector
  index refresh.
- `GovernanceReport::detail_codes()` keeps the aggregate governance reason code
  set tied to individual projected experience ids. Root adapters should log
  these labels when comparing `ExperienceStore::hygiene_report` output with
  norion-memory governance, because they distinguish a missing clean gist on one
  legacy metadata record from cross-task transcript pollution on another.
- `DefaultMemoryIndexPlanner` converts `IndexRebuildPlan` into explicit
  `MemoryIndexOperation` entries: quarantine, delete duplicate, compact, refresh
  embedding, or upsert. This is intentionally read-only so service integration
  can log and review planned index changes before applying them. Its
  `MemoryIndexPlan::reason_codes()` mirrors rebuild reasons plus
  `full_rebuild_requested`, keeping index evidence aligned with governance and
  repair evidence. Per-operation refresh reasons distinguish
  `refresh_missing_clean_gist`, `refresh_dirty_clean_gist`, and
  `refresh_noisy_or_rotting_index`, so detail codes can explain why a vector
  needs refresh without exposing lesson or gist text. The
  `MemoryIndexPlan::summary_line()` row includes those `detail_codes=` using
  operation or skipped labels plus hex-only ids, so startup dashboards do not
  need to parse planned index payloads. `operation_detail_codes_for_kind(kind)`,
  `operation_detail_codes_for_reason(reason)`, and `skipped_detail_codes()` are
  the adapter-facing drilldown helpers for those same labels. `MemoryIndexDocument` metadata for raw
  fallback records should carry basis, risk tags, truncation state, and length
  counters only; prompt and lesson text remain index payload and must not be
  copied into adapter evidence fields.
- `DefaultMemoryRepairPlanner` converts `IndexRebuildPlan` into
  `MemoryRepairPlan` entries: clean-gist repair, compaction, quarantine, and
  duplicate delete. It also records skipped repairs such as legacy metadata
  lessons without a clean gist. Rebuild evidence reports
  `missing_clean_gist=` and `dirty_clean_gist=` separately while keeping
  `dirty_gist=` as the combined legacy repair count. The repair planner consumes
  those split sets when producing `repair_missing_clean_gist`,
  `repair_dirty_clean_gist`, `missing_clean_gist`, and `dirty_clean_gist`
  reason codes, so startup evidence can tell absent summaries from unsafe
  transcript-shaped summaries without reading gist text.
  `MemoryRepairPlan::reason_codes()` and `skipped_reason_codes()` keep
  executable actions and blocked repair work visible in startup evidence. Its
  `summary_line()` also includes `detail_codes=` for executable and skipped
  repair items, with affected record ids hex encoded and without gist text.
  `detail_codes_for_action(action)`, `detail_codes_for_reason(reason)`,
  `skipped_detail_codes()`, `skipped_detail_codes_for_action(action)`, and
  `skipped_detail_codes_for_reason(reason)` are the adapter-facing drilldown
  helpers for those same labels. `MemoryServiceStartupEvidence` mirrors them as
  `memory_repair_detail_codes_for_action(...)`,
  `memory_repair_detail_codes_for_reason(...)`,
  `memory_repair_skipped_detail_codes()`,
  `memory_repair_skipped_detail_codes_for_action(...)`, and
  `memory_repair_skipped_detail_codes_for_reason(...)` so dashboards can filter
  repair work without parsing `memory_repair_plan` text.
- `DefaultContextInjectionGate` converts recall candidates into admit,
  summarize, or reject decisions. This is the prompt-time Context Rot guardrail,
  and `ContextInjectionPlan::reason_codes()` records stable labels such as
  `missing_clean_gist`, `dirty_clean_gist`, `cross_task_scope`,
  `below_min_score`, or `max_tokens` without logging injected text. The
  `memory_context_injection` summary line also includes `detail_codes=` with
  hex-encoded candidate ids for rejected risk, scope, score, and budget
  decisions. Risk rejects retain every normalized risk reason on the candidate,
  so missing-clean-gist raw fallbacks keep raw/truncated index-quality evidence
  for repair and index dashboards. Startup evidence mirrors this as
  `read_only_detail:context:reject_risk:missing_clean_gist:<id_hex>` while
  `context_rot_risk_reason_codes=` keeps the full risk inventory, such as
  `missing_clean_gist|transcript_anchor_risk`, for adapter dashboards.
  Dashboards that need only prompt-time safety rejections should prefer
  `context_injection_reject_risk_detail_codes_for_reason(...)` over the broader
  `context_injection_detail_codes_for_reason(...)`, because the broader helper
  also includes admitted or summarized raw/truncated fallback candidates.
- `DefaultTieredMemoryPlanner` converts hot/warm/cold budgets into explicit
  placement and migration actions. This gives `kv_cache`, `tiered_cache`, and
  `infini_memory` a shared policy surface before any runtime byte movement.
- `DefaultExperienceReplayPlanner` converts replay candidates into
  `ReplayPlan`, preserving high-signal coverage for recursive runtime,
  live-memory feedback, and Context Rot samples while producing memory update
  intent for later application. `ReplayReport::detail_codes()` carries
  hex-id-only action, signal, feedback, and memory-update labels for dry-run
  review. Context Rot samples should appear as `signal:context_rot:<id_hex>`
  and then as `replay:signal:context_rot:<id_hex>` in service startup evidence,
  while replay candidate lessons remain payload and are never startup filter
  fields. `ReplayApplyReport` records applied, missing, and invalid memory
  updates, and `ReplayApplyReport::detail_codes()` identifies missing or invalid
  memory ids with hex-only labels for isolated-write review.
- `DefaultMemoryRetentionPlanner` converts cache entries into
  `MemoryRetentionPlan` and `MemoryCompactionPlan`. It follows the root
  `kv_cache` policy defaults but keeps mutation outside the crate, so adaptive
  state can persist policy while service code still gets an auditable plan.
  Its reason-code sets are the stable labels service logs should compare when a
  cache-maintenance decision changes. Its detail-code sets add hex-id traces for
  decay, removal, merge, and policy-skip decisions while keeping cache keys,
  vectors, and text out of startup evidence.
- `MemoryEvolutionLedger` maps to the memory-focused subset of
  `adaptive_state::EvolutionLedger`: replay runs/items, memory reinforcements
  and penalties, replay apply missing/invalid ids, live memory feedback,
  Context Rot items, retention decay/removal, compaction merge/removal, external
  feedback, and drift rollback counters. `DefaultMemoryEvolutionGate` is the
  service-facing policy boundary before local model self-evolution continues.
  The ledger and gate assessment expose reason-code sets so adapter parity tests
  can distinguish missing replay evidence, memory-update drift, Context Rot, and
  rollback pressure without depending on exact numeric thresholds. Detail-code
  methods retain those numeric thresholds as sanitized metric labels for copied
  fixture review without exposing replay lessons, prompts, transcripts, or
  memory text.
  `AdaptiveStateMemoryProjection` lets root adapters audit those counters
  against the crate ledger without depending on root types.
- `DefaultMemoryInspectionBuilder` maps to the memory-focused subset of
  `state_inspect::StateInspectionReport`: memory counts, runtime-KV counts,
  vector dimensions, top memories, adapter health, projection audit findings,
  readiness blockers, and evolution gate warnings. It is intentionally
  storage-neutral, so root `state_inspect` can wrap it rather than duplicate
  memory-layer summary logic. `MemoryInspectionSnapshot::risk_codes()` exposes
  stable labels alongside detailed risk severities and counts.
  `StateInspectionMemoryProjection` audits known root report fields against
  `MemoryInspectionSnapshot`; missing core counts or mismatched counts should
  require operator review.

## Test Strategy

Covered in `crates/norion-memory` unit tests:

- Legacy metadata lessons with missing clean gists, dirty projected gists,
  cross-task shell transcripts, Context Rot scoring, and rebuild detail codes.
- DiskKv catalog extraction, manifest key generation, mixed catalog entries,
  metadata id mismatch, duplicate metadata rows, copied-fixture verification,
  missing bytes, byte-length mismatch, checksum mismatch, and stable summary
  lines with reason and detail codes.
- KVSwap hot metadata, stale cold-catalog replacement, cold-only snapshots,
  priority/age eviction, prefetch missing/already-hot/duplicate ids, scoped
  shard id hex encoding, intent summaries, and service startup evidence.
- Projection audits for hint tags, fallback task scope, duplicate experience
  ids, empty content, risky records without clean gists, duplicate KV shard ids,
  missing checksums, empty shards, invalid priority, and adapter health.
- Projection contracts for ExperienceStore, DiskKvStore, gist, Infini, KV
  cache, tiered cache, and service-memory field coverage, including
  `standard_shadow`, `copied_fixture_isolated_write`, bundle manifests, and
  startup evidence generated from `with_projection_contract_bundle`.
- Service manifest, shadow-plan, migration-gate, retention, Infini, evolution,
  inspection, runtime-projection, replay, repair, index, context-injection, and
  startup-evidence tests covering the current read-only and isolated-write
  preflight surfaces.

Remaining root-integration coverage:

- Unit-test the eventual root `ExperienceStore` and `DiskKvStore` adapters with
  synthetic stores and copied fixture files under `target/...`, never production
  `.ndkv`.
- Compare root `ExperienceStore::{hygiene_report,index_report,legacy_metadata_repair_plan}`
  against `GovernanceReport`, `IndexRebuildPlan`, `MemoryRepairPlan`, and
  `MemoryIndexPlan` on copied fixtures.
- Compare root `disk_kv`, `kv_cache`, `tiered_cache`, `gist_memory`, and
  `infini_memory` projections against the crate-level manifests and startup
  evidence rows before any root write path is enabled.

The adapter phase is complete when service code can obtain a read-only
governance report and KVSwap can round-trip cold shards through copied `.ndkv`
fixtures without writing real user state, while adaptive/state-inspection
projection audits agree with the crate shadow evidence.
