# Router Decision Trace Replay

Issue #71 adds a redacted, visualization-friendly routing trace for the FHT-DKE
and Noiron control plane. The trace is a pure data export around existing
adaptive routing, task-aware hierarchy, compute-budget scheduling, KV-Fusion
signals, and fallback decisions.

The trace does not change routing behavior and does not couple router code to a
frontend. CLI, benchmark, or dashboard tools can consume the stable JSON schema
without gaining write authority.

## Schema

`RouterDecisionTrace` exports `rust-norion-router-decision-trace-v1` through
`RouterDecisionTrace::to_visualization_json()`.

Top-level fields include:

- `trace_id`
- task `profile`
- `compute_budget`
- `base_threshold`, `adaptive_threshold`, and `threshold_delta`
- aggregate `budget_pressure`
- candidate and selected counts
- route fanout before/after
- KV lookup budget, planned lookups, and skipped lookups
- retained, saved, and avoided-token counters
- fallback flag and fallback reason
- selected candidate ids, selected lanes, rejected candidate ids
- read-only/write/applied/export flags
- redaction and payload-marker counters
- blocked reasons
- per-candidate decision rows

Each decision row includes:

- sanitized `candidate_id`
- source, action, selected lane, and route
- selected/rejected booleans
- score, threshold, score delta, compute pressure, and budget pressure
- estimated, retained, and saved token counts
- anchor requirement
- score components
- KV-Fusion lane and contribution evidence
- fallback-path flag
- sanitized reason text

## Replay Fixtures

`RoutingTraceReplayFixture` replays deterministic router inputs through the
existing `AdaptiveRoutingPlanner::plan_with_compute_budget` path. It records:

- expected selected candidate ids
- expected selected lanes
- expected fallback behavior

`RoutingTraceReplayReport` compares replay output with the expected fixture and
returns the trace, visualization JSON, adaptive threshold, selected ids/lanes,
fallback state, export state, and blocked reasons.

## Fail-Closed Rules

Trace export remains read-only and report-only. A trace blocks export when it
finds:

- row count or selected-count mismatch
- non-positive route fanout
- KV lookup budget overflow
- token accounting mismatch
- fallback without reason
- write-enabled or applied routing/budget state
- redactions or raw/private payload markers
- non-finite score, threshold, pressure, or KV-Fusion values
- skipped required anchors

Text fields are sanitized before JSON export. Raw prompt markers, raw answer
markers, transcript markers, secrets, bearer tokens, and `sk-` style keys do
not appear in visualization JSON. If such input is observed, the trace records
redaction evidence and sets `export_allowed=false`.

## Integration

This issue intentionally keeps the JSON exporter under `router`. The main trace
JSONL gate already verifies aggregate `adaptive_routing`, `task_hierarchy`, and
`compute_budget` evidence. `RouterDecisionTrace` adds the deeper per-candidate
surface needed by future tools:

- CLI replay and issue evidence packets
- dashboard route graphs
- benchmark fixture comparison
- routing-regression debugging
- KV-Fusion and budget-pressure inspection

Future work can embed this object into CLI commands or dashboard APIs without
changing the router planner or granting memory, genome, adaptive-state, or
approval writes.
