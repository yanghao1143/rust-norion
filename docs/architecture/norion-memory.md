# norion-memory Architecture

`crates/norion-memory` is the proposed memory boundary for Norion agents. It is
deliberately independent from the root package for now: the main integration
window can add it to the workspace and map existing `src/*` state into these
ports without changing live Gemma or service processes.

## Layers

- crate root: `AgenticMemory` and `MemoryPorts` bundle short-term KV,
  long-term memory, and skill-library ports for in-process agents. The ports
  implement `MemoryAdapter`, can emit a `MemoryServiceManifest`, and expose
  `readiness_for(profile, write_mode)` so core/agent/service startup code can
  check a `MemoryConsumerProfile` without manually constructing requirement
  wiring. Core readiness should pass with only these three ports; agent and
  service profiles intentionally surface missing governance, replay, context,
  KVSwap, retention, and inspection policy layers until those adapters are
  supplied.
- `adapters`: read-only adapter contracts. They define how root stores expose
  experience snapshots, KV shard catalogs, and combined migration plans without
  writing legacy state. They also include projection hints and projection audit
  reports so root adapters can validate stable ids, scope, tags, clean gists,
  and KV metadata before shadow or isolated-write migration.
  `AdapterProjectionAudit::summary_line()` exposes the audit readiness, issue
  counts, stable issue codes, and per-issue `detail_codes` as startup evidence
  before downstream policy plans are trusted. Projection-audit detail labels use
  `severity:issue[:source_id_hex:<hex>]`, keeping projected ids traceable without
  logging prompts, lessons, gists, or KV bytes.
  `AdapterProjectionContract` declares root-field coverage for `ExperienceStore`,
  `DiskKvStore`, gist memory, Infini memory, KV cache, service-memory, and
  tiered-cache adapters, then emits a stable `AdapterProjectionCoverageReport`
  for shadow-read and isolated-write startup gates. Standard preset constructors
  cover the common root adapter targets so service startup does not need to
  hand-maintain field lists. Coverage reports also expose
  `blocker_detail_codes()` and `warning_detail_codes()` with
  `adapter:reason:field` labels, so compact service evidence can identify the
  missing root adapter field without logging projected record contents. The
  per-adapter `summary_line()` carries those detail-code fields alongside
  aggregate blocker and warning codes.
  `AdapterProjectionContractBundle::standard_shadow()` groups the read-only
  ExperienceStore, DiskKvStore, gist-memory, Infini-memory, KV-cache,
  tiered-cache, and service-memory contracts;
  `copied_fixture_isolated_write()` groups the fixture-safe write-path
  contracts for copied ExperienceStore and DiskKvStore tests.
  `AdapterProjectionBundleReport` gives each bundle a stable aggregate
  summary line before service code expands it into per-adapter coverage,
  including aggregate blocker and warning detail codes for partial fixtures.
  `MemoryServiceShadowPlanInputs::with_projection_contract_bundle(...)` preserves
  the bundle name so service startup evidence logs this aggregate
  `adapter_projection_bundle` row next to the child `adapter_projection` rows.
  `AdapterSnapshotSummary` is the smaller projection-level evidence row for root
  adapters before full planning: it logs write mode, projected experience/KV
  counts, warning codes, and status codes such as `empty_snapshot`, `read_only`,
  or `live_write` without exposing prompt, lesson, gist, vector, or KV bytes.
  The row also includes `detail_codes=` for warning labels such as
  `warning:adapter_unhealthy`, giving dashboards a local per-row filter before
  the service summary prefixes those warnings with the adapter name.
  `ExperienceSnapshotAdapter::snapshot_summary()` and
  `KvShardCatalogAdapter::catalog_summary()` provide default summary builders so
  root adapters only need to implement the underlying snapshot/catalog read.
  These defaults also carry adapter health warnings and add `adapter_unhealthy`
  when `MemoryAdapterHealth.ready` is false.
  `ReadOnlyMemoryPlan::summary_line()` is the compact service preflight row for
  projected memory state. It carries aggregate `reason_codes=` and
  `detail_codes=` so core/service consumers that ingest only the compact row can
  still trace Context Rot, clean-gist repair, index refresh, and KVSwap actions
  by stable labels or hex ids without parsing prompt text, gist text, vectors,
  or KV bytes.
  `MemoryServiceShadowSummary` lifts Context Rot risk evidence onto the compact
  `memory_shadow` row as `context_rot_risks=`,
  `context_rot_risk_reason_codes=`, and `context_rot_risk_detail_codes=`, so UI
  and startup gates can filter risky records without parsing the child
  governance text. Summary-side helpers
  `context_rot_risk_detail_codes_for_reason(reason)` and
  `context_rot_risk_reason_count(reason)` expose the same evidence when callers
  consume `MemoryServiceDryRun` directly.
- `context`: prompt-injection gate. It turns recall candidates into explicit
  admit, summarize, or reject decisions using scope, Context Rot risk reasons,
  score thresholds, and token budgets. `ContextInjectionPlan::summary_line()`
  exposes the prompt gate decision mix, rejection reason codes, and hex-id
  `detail_codes=` without logging injected text. The default policy rejects
  `missing_clean_gist` as well as dirty gists, cross-task transcript pollution,
  transcript anchor risk, and high-noise quarantine labels. Rejected risky
  candidates retain the full normalized reason set, so a raw fallback can show
  both `missing_clean_gist` and index-quality labels such as
  `raw_fallback_index_content` or `truncated_index_content` without injecting
  the raw text.
- `gist`: clean gist value types and selection policy. It maps legacy
  hierarchical gist records into `MemoryGist` and selects a clean summary for
  `ExperienceEnvelope.clean_gist`. `DefaultCleanGistSelector` reports the
  read-only `clean_gist_selection` adapter capability, so service readiness can
  explicitly require clean-gist selection before trusting projected experience
  envelopes. `CleanGistSelectionReport::summary_line()` gives startup logs
  candidate, rejection, and selected-level counts without logging gist text.
- `short_term`: turn/session-scoped KV. It models the fast agent scratchpad and
  exposes `ShortTermKv` plus `InMemoryShortTermKv`.
- `long_term`: vector-like durable memory. It exposes `LongTermMemory`; the
  first implementation is an in-memory cosine/text retriever with optional
  `MemoryScope`, metadata filtering, reinforce, and penalize feedback. A qdrant
  adapter can implement the same trait later.
- `skills`: agent skills, playbooks, repair procedures, and tool recipes. It
  exposes `SkillLibrary` and an in-memory implementation.
- `disk_kv`: KVSwap and cold KV offload. `KvSwapManager` keeps hot bytes in RAM,
  keeps metadata for both hot and cold shards, and moves cold bytes through
  `DiskKvOffload`. Staging a hot shard replaces any cold catalog entry with the
  same id, so snapshots do not report a stale hot/cold duplicate. The included
  backends are in-memory and local-file only.
  `KvSwapStateSnapshot` reports hot/cold shard counts and byte totals from
  metadata/catalog state without reading cold shard bytes, and its
  `shape_codes()` labels empty, hot-only, cold-only, and mixed-tier states for
  startup evidence. `MemoryServiceShadowPlanInputs::with_kvswap_state(...)`
  lets service dry runs include that snapshot as an optional `kvswap_state`
  evidence row beside the KVSwap intent. `MemoryServiceShadowSummary` also
  copies the optional snapshot into compact hot/cold/metadata/byte counts and
  shape codes on the `memory_shadow` line, while keeping the snapshot
  observational so it does not by itself require operator review.
  `KvSwapBoundaryAudit` is the stricter hot/cold boundary check. It reports
  overlapping hot/cold ids, hot bytes without metadata, stale metadata, and tier
  mismatches with `reason_codes()` and hex-id `detail_codes()`. Passing it via
  `MemoryServiceShadowPlanInputs::with_kvswap_boundary(...)` adds the optional
  `kvswap_boundary` startup row and turns non-clean audits into
  `kvswap_boundary_review` on the compact `memory_shadow` line. Startup
  evidence mirrors overlap, missing-hot-metadata, stale-metadata, hot-tier, and
  cold-tier mismatch details through `kvswap_boundary_detail_codes()` using
  hex-only ids.
  Prefetching a cold shard always rebuilds hot metadata from the returned bytes:
  `byte_len` and checksum are recomputed, `tier` becomes hot, and `last_access`
  advances past the cold metadata value. This keeps backend-only catalog
  discoveries and copied fixture catalogs from leaving stale hot metadata.
  `KvEvictionPlan::summary_line()` and `KvPrefetchPlan::summary_line()` expose
  stable log lines with counts plus hex-encoded shard ids for service startup
  and copied-fixture operator evidence. The same rows carry `detail_codes=`
  entries for promote, missing, already-hot, duplicate, demote, and keep-hot
  decisions, while `reason_codes()` distinguish
  promote/missing/already-hot/duplicate and demote/keep-hot actions from
  planner reason labels. Execution also treats repeated promote/demote ids as a
  single state transition and never creates hot metadata for missing cold
  shards. Startup evidence consumers should filter on `*_id_hex=` and
  `detail_codes=` fields, not raw shard ids.
  `DiskKvShardKeyspace` and `DiskKvShardManifest` provide the read-only adapter
  codec for extracting shard metadata from mixed append-only key/value catalogs.
  `DiskKvCatalogVerification` compares copied fixture metadata against shard
  bytes, reporting missing bytes, byte-length mismatches, and checksum
  mismatches before any write-path migration phase is approved. Its
  `summary_line()` separates catalog and checksum readiness and hex-encodes
  mismatch ids for stable copied-fixture logs; `reason_codes()` normalizes
  copied-fixture failures as `byte_len_mismatch`, `checksum_mismatch`, and
  `missing_bytes`, while the same row now carries `detail_codes=` with
  `missing_bytes:<id_hex>`, `byte_len_mismatch:<id_hex>`, and
  `checksum_mismatch:<id_hex>` labels for dashboards that should not parse
  free-form log text.
- `evolution`: memory self-evolution ledger and gate. It summarizes replay,
  replay-apply, retention, compaction, external feedback, Context Rot, and drift
  rollback signals into `MemoryEvolutionLedger`; `DefaultMemoryEvolutionGate`
  decides whether isolated-write self-evolution can continue or needs operator
  review. The ledger summary includes reason codes for replay evidence,
  missing memory updates, Context Rot, maintenance actions, feedback gaps, and
  drift rollbacks; the assessment exposes normalized blocker and warning codes.
  `MemoryEvolutionAssessment::{blocker_detail_codes,warning_detail_codes}`
  preserves threshold values such as missing-update ratio, Context Rot count,
  and drift rollback count as machine-readable metric labels only.
- `governance`: experience-library hygiene: exact deduplication, clean-gist
  checks, noise scoring, Context Rot risk scoring, and index rebuild planning.
  `GovernanceReport::summary_line()` and `IndexRebuildPlan::summary_line()`
  expose stable hygiene and rebuild counts plus sorted reason codes for startup
  evidence. `GovernanceReport::detail_codes()` adds per-record labels such as
  `noise:<experience_id>:missing_clean_gist` and
  `context_rot:<experience_id>:cross_task_transcript_pollution`, so adapter
  parity checks can identify the exact legacy record that needs clean-gist
  repair, quarantine, or rebuild review. `IndexRebuildPlan` keeps the combined
  `dirty_gist_ids` repair set and also reports `missing_clean_gist_ids` versus
  `dirty_clean_gist_ids`, which lets repair UI and service evidence separate
  absent gists from unsafe gist text.
- `index`: read-only index planning. It projects memory envelopes into
  `MemoryIndexDocument` and turns governance rebuild output into explicit
  upsert, refresh, compact, quarantine, and duplicate-delete operations.
  `MemoryIndexPlan::summary_line()` reports operation counts and skipped rebuild
  targets plus reason codes without writing an index. The same row includes
  `detail_codes=` for executable operations and skipped rebuild targets using
  operation/skipped labels plus hex-encoded ids only. Refresh operations split
  `refresh_missing_clean_gist`, `refresh_dirty_clean_gist`, and
  `refresh_noisy_or_rotting_index` so index evidence lines up with governance
  and repair evidence. `operation_detail_codes_for_kind(kind)`,
  `operation_detail_codes_for_reason(reason)`, and `skipped_detail_codes()` let
  service/UI callers drill into those payload-free labels without parsing the
  summary string. Raw fallback document content remains index payload: metadata
  and startup evidence may expose `content_basis=raw_fallback`,
  `risk:missing_clean_gist`, length counters, and hex ids, but not prompt or
  lesson text. `ExperienceIndexFindingProjection` bridges root
  `ExperienceStore::index_report` findings into the same crate rebuild,
  operation, and context-injection risk reason codes, so copied-fixture parity
  tests do not need to parse root preview text.
- `infini`: Infini-memory prompt placement. It splits active/durable memory into
  local-window, global-memory, and skipped sets using capacity limits, token
  budgets, value scores, and redundant-key filtering. This is separate from
  `placement`: `infini` plans what enters model context, while `placement` plans
  hot/warm/cold storage tiers and KVSwap movement.
  `InfiniMemoryPlan::summary_line()` exposes stable local/global/skipped counts
  and token totals plus normalized reason codes so service startup can log
  prompt-placement evidence next to governance, replay, KVSwap, and migration
  gates. Skip labels such as `sparse_filter:low_score`,
  `sparse_filter:local_capacity`, and `sparse_filter:redundant_key_overlap`
  stay stable while detailed scores remain on individual items.
  `InfiniMemoryPlan::detail_codes()` adds hex-encoded item ids for selected and
  skipped placement decisions without logging memory keys; adapter/UI filters
  should treat active and durable memory keys as payload rather than evidence.
- `inspect`: state-inspection boundary. It turns memory entries, adapter
  readiness, projection audit, and evolution assessment into
  `MemoryInspectionSnapshot`, including vector-dimension buckets, top memories,
  runtime-KV summaries, risk counts, normalized risk codes, and a compact
  summary line. `MemoryInspectionSnapshot::risk_detail_codes()` adds compact
  `severity:risk:count` labels, normalizing metric values and keeping adapter
  names, prompts, memory text, and vector contents out of service evidence.
- `migration`: explicit migration approval gates. It models
  `read_only_shadow`, `copied_fixture_write`, `isolated_write`, and
  `live_write` phases, requires fixture evidence before any write-path test, and
  keeps live writes disabled by default. The gate derives blockers and warnings
  from `MemoryServiceShadowPlan` plus fixture/catalog/checksum evidence. It
  exposes `MemoryMigrationEvidence::guard_codes()` for missing copied fixtures,
  unverified catalogs/checksums, live-store targeting, and unknown record counts.
  `MemoryMigrationEvidence::summary_line()` also carries
  `detail_codes=guard:<code>` labels for those same guard failures. When the
  evidence is built from `DiskKvCatalogVerification`, it also preserves
  `disk_kv_catalog:<verification_detail>` labels such as
  `disk_kv_catalog:checksum_mismatch:<id_hex>`, so startup evidence can point at
  the affected copied shard without reading bytes. Clean copied fixtures report
  `guard_codes=none detail_codes=none`. `MemoryServiceStartupEvidence` mirrors
  those labels into `migration_guard_codes=` and `migration_detail_codes=` using
  the same hex-only shard identifiers, preserving missing bytes, byte-length
  mismatch, and checksum mismatch as separate accessor-visible DiskKv catalog
  failures; raw shard ids, KV bytes, prompt text, gist text, vectors, and memory
  contents must not become startup filter fields.
  The gate
  also surfaces projection-contract coverage as stable
  `projection_contract_blocker:<adapter>:<code>` blockers and
  `projection_contract:<adapter>:<code>` warnings, so service logs can point at
  the exact adapter mapping gap. `MemoryMigrationApproval::summary_line()` keeps
  those raw entries in `blocker_details` and `warning_details`, while
  `blocker_codes` and `warning_codes` normalize them into aggregate categories
  such as `projection_contract_blocker:missing_required` for dashboards.
  `blocker_detail_codes()` and `warning_detail_codes()` keep adapter/field
  labels, bounded metrics, hex-encoded projection source ids, and copied
  DiskKv `disk_kv_catalog:<issue>:<id_hex>` fixture details only, so
  automated gates can trace the phase failure without logging prompts,
  transcripts, gist text, KV bytes, vectors, or memory contents.
- `placement`: tiered cache planning. It models `hot_gpu`, `warm_ram`, and
  `cold_disk` placement, computes migration actions, and can project
  `KvShardMetadata` into tier candidates without moving bytes. `KvSwapIntent`
  converts placement migrations into prefetch/evict intent for KVSwap callers
  and exposes stable summary lines for service startup evidence. Its compact
  intent line carries reason codes for promote/demote/missing/keep-hot actions
  and delegated planner reasons, while child prefetch/eviction lines keep the
  hex-encoded shard ids. `KvPrefetchPlan::detail_codes()` and
  `KvEvictionPlan::detail_codes()` expose the same promote/missing/already-hot,
  duplicate, demote, and keep-hot decisions as sorted hex-id labels for compact
  dashboards that should not parse full summary rows.
- `replay`: experience replay planning. It converts projected candidates into
  reinforce, penalize, or hold batches, preserves high-signal replay coverage,
  and emits memory update intent plus summary stats for long-term self-evolution.
  `ReplayReport::summary_line()` emits stable replay-plan evidence for service
  startup, including action counts, touched memories, feedback counts, average
  reward, signal coverage, and reason codes for actions, memory updates,
  feedback gaps, and Context Rot samples. `ReplayReport::detail_codes()` adds
  per-item action, memory-update, signal, and feedback-gap labels with
  hex-encoded experience and memory ids only; adapters must not expand those
  labels into lesson text, notes, transcripts, or memory contents in startup
  evidence. `apply_replay_updates_to_long_term` can apply reinforce/penalize
  intent to a `LongTermMemory` implementation; its `ReplayApplyReport` exposes
  `detail_codes()` for missing and invalid memory ids using the same hex-only
  rule.
- `repair`: governance repair planning. It projects rebuild output into
  quarantine, duplicate delete, compaction, and clean-gist repair items without
  mutating the legacy experience store. `MemoryRepairPlan::summary_line()`
  separates executable repair items from skipped work such as missing clean
  gists, includes stable reason-code sets for both, and carries `detail_codes=`
  for executable and skipped repair items with hex-encoded source ids only.
  `detail_codes_for_action(action)`, `detail_codes_for_reason(reason)`,
  `skipped_detail_codes()`, `skipped_detail_codes_for_action(action)`, and
  `skipped_detail_codes_for_reason(reason)` provide payload-free repair
  drilldown without parsing the summary row.
- `retention`: memory retention and compaction planning. It mirrors the
  existing `kv_cache` policy shape (`stale_after`, `decay_rate`,
  `remove_below_strength`, `remove_after_failures`, similarity threshold,
  candidate limits, and merge limits) while returning read-only decay, removal,
  and merge plans instead of mutating runtime cache entries.
  `MemoryRetentionPlan::summary_line()` and
  `MemoryCompactionPlan::summary_line()` expose stable maintenance evidence for
  service startup before self-evolution or cache mutation is allowed, including
  sorted reason codes for stale decay, weak removals, high-similarity merges, or
  skipped compaction policies. Their `detail_codes()` methods add per-plan
  labels with hex-encoded memory ids, such as
  `decay:stale_decay:<id_hex>`,
  `remove:weak_stale_and_repeated_failures:<id_hex>`,
  `merge:same_namespace_high_similarity:<primary_hex>:<removed_hex>`, and
  `skipped:policy_disabled`, without logging cache keys, vectors, prompt text,
  or clean-gist text.
- `runtime_projection`: root-state parity helpers. It defines
  `AdaptiveStateMemoryProjection`, `StateInspectionMemoryProjection`, and
  `MemoryProjectionAudit` so future root adapters can compare
  `src/adaptive_state` and `src/state_inspect` counters against
  `MemoryEvolutionLedger` and `MemoryInspectionSnapshot` without importing root
  types into this crate. `MemoryProjectionAudit::summary_line()` emits the
  required `memory_projection_parity` startup row with mismatch counts, warning
  codes, and `detail_codes=` labels such as `mismatch:replay_runs`, while
  keeping expected/actual counter values out of startup-level filters.
- `service`: core/agent/service readiness planning. It combines
  `MemoryAdapterDescriptor`, `MemoryAdapterHealth`, and adapter write mode into
  a deterministic `MemoryServiceManifest`, then checks required capabilities for
  `core`, `agent`, `service`, or `shadow_migration` profiles.
  `MemoryReadinessReport::summary_line()` emits the `memory_readiness` startup
  row with the required write mode, missing capability codes, write-mode blocker
  codes, unhealthy adapter counts, and warning codes before deeper planning
  begins.
  `MemoryAdapterStatus::summary_line()` emits one `memory_adapter_status` row per
  adapter with descriptor capabilities, read-only/write-mode state, record
  count, health warning codes, and stable status codes.
  `MemoryServiceShadowPlan` is the service-level dry run: it combines readiness,
  projection audit, read-only governance, Infini planning, retention,
  compaction, replay planning/reporting, evolution gate, migration readiness,
  inspection output, and optional root projection parity audits without mutating
  root state. Replay candidates can be passed directly into
  `MemoryServiceShadowPlanInputs`; the resulting `ReplayPlan` and
  `ReplayReport` are recorded into `MemoryEvolutionLedger` so missing replay
  evidence can be satisfied by the dry run itself instead of only by external
  seeded counters. Adapter projection contracts can also be passed into
  `MemoryServiceShadowPlanInputs` directly or through
  `with_projection_contract_bundle`; their `AdapterProjectionCoverageReport`
  entries and optional `AdapterProjectionBundleReport` aggregate are included in
  the shadow summary, checklist, startup evidence, and operator-review decision
  so field-mapping gaps are visible before root
  adapters are trusted. Projection-contract detail codes are aggregated into the
  compact `memory_shadow` line under `projection_contract_*_detail:*`.
  `MemoryServiceChecklistItem::summary_line()` exposes
  each item with stable detail codes, and
  `MemoryServiceAdapterChecklist::summary_line()` aggregates failed blocker and
  warning detail codes so startup dashboards can filter the exact failing
  counter or nested code without parsing human-readable detail text.
  The checklist also includes `context_rot_risks_clean`, a warning-level item
  backed by `context_rot_risk_*` summary fields, so operators can distinguish
  prompt-gate rejections from the underlying Context Rot risk inventory.
  `migration_evidence_ready` mirrors copied-fixture guard codes and
  `disk_kv_catalog:*` detail labels for missing bytes, byte-length mismatches,
  and checksum mismatches, giving KVSwap/DiskKv fixture failures a checklist
  entry without parsing migration approval text.
  `MemoryCapabilityCoverage::summary_line()` expands readiness into one
  `memory_capability_coverage` row per required capability, including provider,
  healthy, writable, read-only, record-count, and status-code evidence. Service
  callers can add optional gates such as `clean_gist_selection` through
  `MemoryServiceRequirement::with_capabilities`. The active
  `MemoryServiceRequirement::summary_line()` is now carried on
  `MemoryServiceShadowPlan` and emitted as a required startup evidence row before
  readiness, so operators can see which profile and capability contract was
  evaluated even when readiness fails early.
  `MemoryServiceShadowPlanInputs::with_requirement(...)` can switch the same
  dry-run from the default `ShadowMigration/ReadOnly` check to later
  `Service/IsolatedWrite` preflight without rebuilding the rest of the evidence
  pipeline. `MemoryServiceShadowPlanInputs::with_adapter_snapshots(...)` carries
  optional `adapter_snapshot` rows from root projections into startup evidence;
  snapshot warnings increment `adapter_snapshot_warnings`, add
  `adapter_snapshot_warnings` to review reasons, and surface compact
  `adapter_snapshot:<adapter>:<warning>` detail codes. The adapter checklist also
  includes `adapter_snapshots_clean` as a warning-level item so operator UIs can
  flag unhealthy or lagging projection sources without expanding the full shadow
  summary.
  It also exposes `migration_approval(...)`, a convenience wrapper around
  `DefaultMemoryMigrationGate` for service dry runs. `MemoryServiceShadowSummary`
  is the compact log/operator view of the same plan, including counts and stable
  review reason codes. It also emits `detail_codes=` on the compact
  `memory_shadow` row, aggregating readiness, adapter-health,
  projection audit, projection-contract, read-only-plan, replay,
  migration-readiness, evolution, inspection, retention/compaction maintenance
  actions, and projection-parity labels for dashboards that do not expand the
  full dry-run plan. Projection-audit labels are prefixed with
  `projection_detail:` and use hex-encoded source ids only. Evolution
  detail labels include only normalized metric names and numeric values, never
  replay lessons, prompts, transcripts, or memory text. Migration-readiness
  detail labels carry readiness counters such as `context_rejections=1` or
  `repair_items=2` under `migration_readiness_*_detail:*`; the readiness report
  also exposes `blocker_detail_codes()` and `warning_detail_codes()` on its own
  summary line for dashboards that do not expand the service shadow row.
  Maintenance detail labels keep ids hex-encoded and are included only for
  actual decay/removal/merge work or explicit disabled compaction policies, not
  for clean no-op skips such as `not_enough_entries`. `MemoryServiceDryRun`
  bundles the plan, summary,
  migration evidence, and requested migration phase approvals into one
  root/service startup result.
  `MemoryServiceStartupEvidence` turns a dry run into stable log lines covering
  the shadow summary, readiness plus adapter status and per-capability coverage,
  read-only plan, projection audit, governance, rebuild, repair, index, context
  injection, adapter checklist plus per-item checklist rows, replay report,
  evolution ledger, Infini prompt placement, retention and compaction maintenance,
  inspection snapshot, migration evidence, KVSwap intent, adapter snapshot rows,
  projection-contract coverage, and migration approval results. It also exposes
  `required_line_prefixes()`,
  `missing_required_line_prefixes()`, `missing_required_codes()`,
  `status_codes()`, `detail_codes()`, `is_complete()`, and `summary_line()` so
  root service startup can fail fast when a core evidence line disappears before
  operators review the detailed gate output. `status_codes()` separates
  complete/incomplete evidence, review state, and phase-approval state without
  requiring dashboards to parse detail labels. It also exposes structured accessors
  for the review fields that dashboards commonly need:
  `context_rot_risk_count()`, `context_rot_risk_reason_codes()`,
  `context_rot_risk_detail_codes()`, `migration_evidence_guard_codes()`,
  `migration_evidence_detail_codes()`, `memory_repair_detail_codes()`,
  `memory_repair_detail_codes_for_action(...)`,
  `memory_repair_detail_codes_for_reason(...)`,
  `memory_repair_skipped_detail_codes()`,
  `memory_repair_skipped_detail_codes_for_action(...)`,
  `memory_repair_skipped_detail_codes_for_reason(...)`,
  `memory_index_reason_codes()`,
  `memory_index_detail_codes()`, `memory_index_detail_codes_for_kind(...)`,
  `memory_index_detail_codes_for_reason(...)`,
  `memory_index_skipped_detail_codes()`, `context_injection_reason_codes()`,
  `context_injection_detail_codes()`, `kvswap_boundary_issue_count()`,
  `kvswap_boundary_reason_codes()`, and `kvswap_boundary_detail_codes()`.
  The final `memory_startup_evidence` row mirrors those same groups as
  `context_rot_risks=`, `context_rot_risk_reason_codes=`,
  `context_rot_risk_detail_codes=`, `migration_guard_codes=`,
  `migration_detail_codes=`, `kvswap_boundary_issues=`,
  `kvswap_boundary_reason_codes=`, and `kvswap_boundary_detail_codes=`, so
  service dashboards can consume a single startup summary without parsing child
  prose. Startup detail codes always use
  `missing_line:<prefix_code>`, `incomplete_evidence`,
  `operator_review_required`, and `approved_phases:none` for evidence-shape
  failures. When the dry run already requires operator review, they also lift
  stable detail codes from the compact `memory_shadow` line, migration evidence
  guard detail fields, projection-parity rows, repair/index plan rows, KVSwap
  prefetch/eviction action rows, and projection or migration blocker/warning
  detail fields, while intentionally ignoring generic child `detail_codes=`
  fields that may be derived from checklist wording. Projection-parity lifted
  details are field/warning labels only; repair/index/KVSwap lifted child
  details are constrained to action labels and hex-only ids.
  `MemoryStartupEvidenceSink` is the small service-facing writer trait for this
  boundary: `MemoryServiceStartupEvidence::emit_to(...)` writes each evidence row
  plus the final `memory_startup_evidence` summary row to a caller-owned sink
  without forcing service code to duplicate summary-line loops.
  `MemoryStartupAdmissionEvidence` is the focused pure-data consumer contract
  for startup/index-quality admission checks. It derives index-quality pressure,
  Context Rot risk counts, context-admission counts, live-store targeting,
  adapter live-write status, helper-prose counts, and non-contract line counts
  only from stable startup prefixes and typed accessors. Helper prose, operator
  notes, stale window payload, or old-thread text can mention live writes,
  `.ndkv` rewrites, or larger admission windows, but those lines are counted as
  non-contract evidence and do not set live-store mutation, store mutation,
  `.ndkv` write, or admission-expansion fields.

`AgenticMemory` composes the three agent-facing layers:

```rust
pub trait AgenticMemory {
    type ShortTerm: ShortTermKv;
    type LongTerm: LongTermMemory;
    type Skills: SkillLibrary;
}
```

The main crate can start with `MemoryPorts<InMemoryShortTermKv,
InMemoryLongTermMemory, InMemorySkillLibrary>` and later swap each layer
independently. `MemoryPorts::adapter_status(...)` and
`MemoryPorts::service_manifest(...)` let core startup feed those three
agent-facing ports directly into `MemoryServiceManifest` readiness checks before
the root service adds governance, index, replay, KVSwap, and migration
adapters.

Service-facing callers should also carry `MemoryScope` and
`MemoryRequestContext`. `MemoryScope` captures optional agent, session, and task
ids, which lets governance distinguish a valid same-task shell transcript from a
cross-task Context Rot hazard before retrieved text reaches the prompt.
Long-term queries can carry the same scope: explicitly different task memories
are filtered out, while global memories without a task id remain recallable.
Before any retrieved text reaches a prompt, `DefaultContextInjectionGate` can
apply the same scope plus risk policy and return a `ContextInjectionPlan`.
`MemoryAdapterDescriptor` and `MemoryAdapterHealth` provide a small discovery
surface for core/agent/service wiring; the default in-memory ports and
`KvSwapManager` can report their capabilities through `MemoryAdapter`.
`MemoryServiceManifest` is the next boundary above those descriptors. It lets a
service assemble many adapters, ask whether a `MemoryConsumerProfile` has all
required capabilities, and identify missing, unhealthy, read-only-only, or
multi-provider capability coverage before any root integration mutates state.

## Mapping From Existing src Modules

Current code keeps memory and experience logic under root `src`:

- `src/experience.rs` and `src/experience/*` already define records, retrieval,
  hygiene, index reports, repair, and quarantine plans.
- `src/experience_replay.rs` and `src/experience_replay/*` already define replay
  sampling, reward actions, and feedback reports that map to
  `ReplayCandidate`, `ReplayPlan`, and `ReplayReport`.
- `src/disk_kv.rs` and `src/disk_kv/*` already provide a compact append-only
  local KV store.
- `src/gist_memory/*` provides clean summaries that map to
  `MemoryGist`; `DefaultCleanGistSelector` chooses the best clean summary for
  `ExperienceEnvelope.clean_gist`.
- `src/kv_cache.rs`, `src/tiered_cache.rs`, and `src/infini_memory.rs` already
  contain placement and memory-tier concepts that map to `MemoryTier`,
  `TieredMemoryPlan`, `KvTier`, `KvEvictionPlan`, and `KvPrefetchPlan`.
- `src/infini_memory/*` already plans local-window/global-memory prompt state.
  It maps to `InfiniMemoryActiveMatch`, `InfiniMemoryItem`,
  `InfiniMemoryPlan`, and `DefaultInfiniMemoryPlanner`.
- `src/kv_cache::{retention,compaction,model}` already define decay, removal,
  merge, and update reports that map to `MemoryRetentionPolicy`,
  `MemoryCompactionPolicy`, `RetentionMemoryEntry`, `MemoryRetentionPlan`, and
  `MemoryCompactionPlan`.
- `src/adaptive_state::EvolutionLedger` already persists detailed live/replay
  counters. The crate-level `MemoryEvolutionLedger` is the storage-neutral
  memory subset that adapters can fill from replay, retention, compaction, and
  external feedback reports before service gates approve continued evolution.
  `AdaptiveStateMemoryProjection` is the adapter-facing parity object for
  checking the root counters against the crate ledger.
- `src/state_inspect/*` already builds service/operator inspection reports. The
  crate-level `DefaultMemoryInspectionBuilder` is the memory-layer subset that
  can summarize adapter readiness, projection risks, evolution risks, vector
  dimensions, and top memory entries without depending on root engine types.
  `StateInspectionMemoryProjection` lets root inspection reports compare their
  memory counts and blocker counts against the crate snapshot.

The first integration step should be adapters, not rewrites:

1. Convert `ExperienceRecord` into `ExperienceEnvelope` for governance reports.
2. Declare an `AdapterProjectionContract` for each root adapter and check its
   `coverage_report(AdapterProjectionTarget::ShadowRead)` before service
   startup trusts the projection. Isolated write requires a separate
   `AdapterProjectionTarget::IsolatedWrite` report with isolated write mode.
   Prefer the standard contract presets for complete `ExperienceStore`,
   copied-fixture `DiskKvStore`, tiered-cache, and service-memory adapters.
   Service startup can pass those contracts into
   `MemoryServiceShadowPlanInputs::with_projection_contracts` so the same
   dry-run evidence object records coverage blockers and warnings.
3. Apply `ExperienceProjectionHints` to attach stable adapter/profile/runtime
   tags and a task scope when the legacy record does not already carry one.
4. Convert existing KV/cache records into `KvShardMetadata` and cold shard ids.
5. Run `DefaultAdapterProjectionAuditor` on the projected experiences and KV
   metadata. Shadow read blocks on empty/duplicate ids and invalid KV priority;
   isolated-write also requires task scope warnings to be resolved.
6. Convert governed envelopes into `MemoryIndexDocument` and use
   `DefaultMemoryIndexPlanner` to plan index writes before applying them.
7. Convert recall matches into `ContextCandidate` and use
   `DefaultContextInjectionGate` to decide prompt admission before injection.
8. Convert `KvShardMetadata` or runtime memory entries into
   `MemoryPlacementCandidate` and use `DefaultTieredMemoryPlanner` to plan
   hot/warm/cold movement before invoking KVSwap. Log the resulting
   `KvSwapIntent`, `KvEvictionPlan`, and `KvPrefetchPlan` summary lines before
   moving copied fixture bytes. Compare `KvSwapIntent::reason_codes()` to see
   whether a run is driven by promotions, demotions, missing prefetch targets,
   already-hot or duplicate requests, or keep-hot pressure before inspecting
   shard ids.
9. Convert active recall matches and durable runtime cache entries into
   `InfiniMemoryActiveMatch` and `RetentionMemoryEntry`, then use
   `DefaultInfiniMemoryPlanner` to decide local-window/global-memory/skipped
   prompt placement under token and redundancy budgets. Log
   `InfiniMemoryPlan::reason_codes()` so prompt-memory reviews can distinguish
   selected local/global memory from score, capacity, token-budget, redundancy,
   or missing-entry skips. Use `InfiniMemoryPlan::detail_codes_for_scope(...)`
   and `skipped_detail_codes_for_reason(...)` when dashboards need selected
   local/global or skipped-memory drilldown without parsing prompt keys.
10. Convert replayable experiences into `ReplayCandidate` and either pass them
   to `MemoryServiceShadowPlanInputs::with_replay_candidates` or call
   `DefaultExperienceReplayPlanner` directly to plan reinforce/penalize/hold
   batches. The service shadow plan records the resulting `ReplayReport` into
   `MemoryEvolutionLedger` before the evolution gate runs. Use
   `ReplayReport::reason_codes()` to compare replay action, feedback, and
   Context Rot signal changes between dry runs, and
   `ReplayReport::detail_codes()` when dashboards need the affected replay item,
   memory update, or signal id without logging replay lessons or feedback notes.
11. Convert `IndexRebuildPlan` into `MemoryRepairPlan` before applying any
   quarantine, compact, duplicate delete, or clean-gist repair.
12. Convert runtime cache entries into `RetentionMemoryEntry` and use
   `DefaultMemoryRetentionPlanner` to plan stale-memory decay, weak-memory
   removal, and high-similarity compaction before mutating cache state. Log
   `MemoryRetentionPlan::reason_codes()` and
   `MemoryCompactionPlan::reason_codes()` from startup evidence so maintenance
   reviews can distinguish policy skips from actual decay/removal/merge work.
   When a dashboard needs per-item traceability, use
   `MemoryRetentionPlan::detail_codes()` and
   `MemoryCompactionPlan::detail_codes()`; they use hex ids only and must not be
   expanded back into cache keys or text in startup logs.
13. Record replay, replay-apply, retention, compaction, external feedback, drift
    rollback, and Context Rot counts into `MemoryEvolutionLedger`, then call
    `DefaultMemoryEvolutionGate` before allowing self-evolution write paths to
    continue. Compare `MemoryEvolutionLedger::reason_codes()` and
    `MemoryEvolutionAssessment::{blocker_codes,warning_codes}` across dry runs
    instead of scraping threshold values from human-readable gate messages.
14. Build a `MemoryInspectionSnapshot` from projected cache entries, adapter
    statuses, projection audit, readiness, and evolution assessment so service
    logs expose the same memory-layer evidence used by migration gates. Use
    `MemoryInspectionSnapshot::risk_codes()` for stable operator labels while
    `risks` retains severity and count details.
15. Let model-service gates call `ExperienceGovernance::rebuild_plan` as a
   read-only planning step before any real repair/quarantine apply.
16. Keep root `.ndkv` files read-only during migration; write isolated state
   under `target/...` until governance plans are proven stable.
17. Build a `MemoryServiceManifest` from the adapters and require
    `MemoryConsumerProfile::ShadowMigration` in read-only mode first. Move to
    `MemoryConsumerProfile::Service` with `AdapterWriteMode::IsolatedWrite`
    only after copied fixtures prove DiskKv/KVSwap and governance repair plans
    are stable.
18. Prefer `MemoryServiceShadowPlan::for_inputs` as the first service
    integration target. It performs the full read-only dry run and emits one
    object containing projection-contract coverage, replay evidence, migration
    gates, inspection evidence, and optional `projection_parity_audit` output.
19. Use `DefaultMemoryMigrationGate` or
    `MemoryServiceShadowPlan::migration_approval(...)` to advance phases:
    `ReadOnlyShadow` can pass with review warnings, `CopiedFixtureWrite`
    requires a verified copied fixture, `IsolatedWrite` requires a clean shadow
    plan plus copied fixture evidence, and `LiveWrite` remains disabled unless a
    future integration policy explicitly enables it. Projection-contract
    coverage warnings remain approval warnings in read-only shadow mode; missing
    required contract fields become migration blockers for write-path targets.
    Log `MemoryMigrationEvidence::summary_line()` beside phase approvals so
    copied-fixture, catalog, checksum, live-target, and record-count gaps are
    machine-actionable through both `guard_codes=` and `detail_codes=`. For
    DiskKv copied fixtures, the detail set includes both `guard:<code>` and
    `disk_kv_catalog:<verification_detail>` entries.
20. Project root `adaptive_state` counters into
    `AdaptiveStateMemoryProjection` and root `state_inspect` counts into
    `StateInspectionMemoryProjection`, then pass them into
    `MemoryServiceShadowPlanInputs`. The shadow plan merges `audit_ledger` and
    `audit_snapshot` into `projection_parity_audit`; any mismatch or missing
    core inspection count becomes an operator-review item and blocks isolated
    write approval under the default migration policy. Log the resulting
    `memory_projection_parity` startup row beside inspection and
    migration-readiness evidence so adapter drift is visible without exposing
    counter values.
21. Log `MemoryServiceDryRun::startup_evidence().summary_text()` during shadow
    runs. It provides stable lines for projected experiences, KV shards, runtime
    memory, Context Rot rejections, repair items, replay evidence, KVSwap intent,
    Infini prompt placement, retention, compaction, evolution blockers,
    inspection risks, migration evidence, projection audit issue codes,
    projection contract coverage, projection parity drift, migration approval,
    and review reason codes. `MemoryServiceShadowSummary::reason_codes()` mirrors
    the sorted review reasons so operator dashboards can filter the compact
    `memory_shadow` line without parsing counts. `MemoryServiceStartupEvidence`
    also exposes `missing_required_codes()` and `status_codes()`, logging
    `missing_codes=` and `status_codes=` so missing evidence rows, review state,
    and phase-approval state are machine-actionable instead of raw prefix strings
    only. It also exposes Infini and KVSwap accessors such as
    `infini_memory_detail_codes()`,
    `infini_memory_detail_codes_for_scope(...)`,
    `infini_memory_skipped_detail_codes_for_reason(...)`,
    `kvswap_prefetch_detail_codes_for_action(...)`,
    `kvswap_eviction_detail_codes_for_reason(...)`, and
    `kvswap_action_detail_codes_for_stage(...)` so UI filters can consume
    hex-id evidence without scraping memory keys or raw shard ids from summary
    text.
22. Use `MemoryServiceDryRun::for_inputs` when startup wants one object
    containing the shadow plan, summary, and approvals for phases such as
    `ReadOnlyShadow` and `IsolatedWrite`. This keeps root/service integration
    from duplicating phase-loop approval logic.

When wiring several standard adapters at once, prefer
`AdapterProjectionContractBundle::standard_shadow()` with
`MemoryServiceShadowPlanInputs::with_projection_contract_bundle(...)` for the
read-only service dry run. For copied fixture tests that intentionally exercise
isolated writes, use `copied_fixture_isolated_write()` and copied
ExperienceStore/DiskKvStore state only. `coverage_summary().summary_line()` is
the preflight log line for this manifest-level check, and its blocker/warning
codes normalize missing fields, write-mode drift, and adapter notes for
operator UI filters.

See `docs/architecture/norion-memory-adapters.md` for the concrete
`ExperienceStore`, `DiskKvStore`, and tiered-cache adapter plan. The crate-level
`MemoryServiceRequirement::summary_line()` records the active consumer profile,
minimum write mode, and stable capability set before readiness is evaluated, so
core, agent, service, and shadow-migration boot paths can compare the same
profile contract in logs.
`ReadOnlyMemoryPlan` combines governance, repair, index, context injection,
placement, and KVSwap intent so service integration can log one deterministic
plan before any mutation. Its `summary_line()` emits stable counts for
governance noise, Context Rot, rebuild/repair/index work, context decisions,
tier placement, and KVSwap pending state, while `reason_codes()` aggregates
prefixed child codes such as `governance:*`, `repair:*`, `context:*`, and
`kvswap:*` for operator filters. `detail_codes()` is the matching non-leaking
trace surface: it aggregates governance per-record labels, repair/index/context
hex-id labels, and KVSwap hex-id action labels, while ordinary clean actions
such as index `upsert` and admitted context stay out of the detail set.
When the service passes a `KvSwapStateSnapshot`, the surrounding
`MemoryServiceShadowSummary` mirrors the snapshot's hot/cold/metadata counts,
total bytes, and shape codes on the compact `memory_shadow` row. Absence of a
snapshot is rendered as `kvswap_state=false` with no shape codes, so projection
fixtures do not need to fabricate cache state. If the service also passes a
`KvSwapBoundaryAudit`, the same compact row includes
`kvswap_boundary=true`, `kvswap_boundary_issues=...`, and
`kvswap_boundary_reason_codes=...`; non-clean boundary audits require operator
review and the startup evidence carries the audit's hex-id detail labels.
`MigrationReadinessReport` summarizes whether that plan is ready for
isolated-write testing or still needs operator review, and its `summary_line()`
is included in startup evidence before phase approval results are logged.
`MemoryReadinessReport` is complementary: it checks whether the adapter set
itself can satisfy a consumer profile at a requested write level before a
planning run begins. Its `summary_line()` is included in startup evidence so
core/agent/service integrations can compare missing capability and write-mode
blocker codes independently from shadow-plan review reasons.
`MigrationReadinessReport::summary_line()` includes
normalized blocker and warning codes plus raw blocker and warning details, so
isolated-write preflight review can compare context, governance, repair, and
KVSwap readiness without losing counts such as `context_rejections=1`.

## Future Storage Adapters

- `redb`: good candidate for local typed metadata and shard catalogs. Implement
  `DiskKvOffload` with redb tables for metadata and byte chunks.
- `sled`: useful for embedded append-friendly KV. Implement `DiskKvOffload`
  with shard id keys and metadata prefixes.
- `qdrant`: long-term vector memory adapter. Implement `LongTermMemory` by
  mapping `MemoryDocumentInput.embedding` and metadata into qdrant points.
- Existing append-only `src/disk_kv`: can be wrapped as a `DiskKvOffload`
  backend once the crate is connected to the workspace. Its first adapter should
  use `DiskKvShardKeyspace::catalog_manifests` to recover metadata entries from
  copied or read-only stores, rejecting duplicate metadata and key/metadata id
  mismatches before exposing a KVSwap catalog. Copied fixtures should also call
  `DiskKvShardKeyspace::verify_catalog_entries` and pass the resulting
  `DiskKvCatalogVerification` into `MemoryMigrationEvidence::copied_disk_kv_fixture`
  while logging `DiskKvCatalogVerification::summary_line()` so stale bytes,
  corrupt checksums, and missing shard data stay machine-actionable.
  `DiskKvCatalogVerification::detail_codes()` adds
  `missing_bytes:<id_hex>`, `byte_len_mismatch:<id_hex>`, and
  `checksum_mismatch:<id_hex>` labels so copied-fixture dashboards can point at
  the affected shard without exposing keys or bytes. These verification gaps
  must block migration approval when fixture bytes are incomplete or corrupt.

The boundary is intentionally trait-first so the running Gemma chain, current
experience `.ndkv` files, and tool control plane do not need to change while
the storage backend evolves.
