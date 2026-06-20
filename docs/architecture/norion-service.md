# Norion Service and CLI Boundary

`norion-service` owns the frontend-facing chat protocol. It is intentionally
small: `ChatRequest` carries messages, profile, output mode, optional
`max_tokens`, stream preference, and model routing hints; `ChatChunk` carries
ordered stream output; `ChatSession` owns history limits, partial answer state,
and stream state.

`norion-cli` owns terminal interaction semantics. The default contract is:
Enter sends the current buffer, Shift+Enter inserts a newline, Ctrl+X requests
stream cancel, and output append events carry a `ScrollIntent` so a UI can keep
following the latest content or preserve a manual scrollback position.
`CliInputConfig` also carries the current model-pool routing context, so a CLI
or TUI can change role/preference/worker before Enter builds the next
`ChatRequest`.

## Backend Connection

For local UI/CLI development, point the frontend adapter at the existing backend
instead of restarting it:

```text
http://127.0.0.1:7878
```

`127.0.0.1:7878` is the `rust-norion` service boundary for CLI/Web/Forge
adapters. The Gemma/mistralrs runtime remains behind the service at
`127.0.0.1:8686`, and Web Lab browser/SSE proxy ports such as `8787` or `8789`
should point back to 7878 instead of sending prompt traffic directly to the
runtime.

The service adapter should map:

- `ChatRequest.messages` to `/v1/chat-stream` JSON `messages`
- `ChatRequest.profile` to `profile`
- `ChatRequest.output` to `output`
- `ChatRequest.max_tokens` to `max_tokens` only when present
- `ChatRequest.routing_preference` to scheduler hints such as `prefer_fast` or
  `prefer_quality`
- `ChatRequest.model_role` to logical worker duties such as `assistant`,
  `reviewer`, `summarizer`, or `tester`
- `ChatRequest.endpoint_pinned()` and `endpoint_kind_label()` to explicit
  `endpoint_pinned` and `endpoint_kind` route metadata
- `ChatRequest.model_endpoint` to a pinned worker only when the user explicitly
  selects one
- backend SSE `delta/status/meta/final/done/error` events to ordered
  `ChatChunk` values

Prefer `norion_service::request_json(&ChatRequest)` for this outbound body. It
serializes history messages, stream mode, role/preference hints, explicit
`endpoint_pinned`/`endpoint_kind` route metadata, and only emits `max_tokens` or
`model_endpoint` when they are present, preventing accidental fallback to tiny
output budgets or unwanted worker pinning. Auto-routed requests serialize as
`endpoint_pinned=false` and `endpoint_kind=auto` with no `model_endpoint`;
operator-pinned built-in or custom workers serialize as `endpoint_pinned=true`
with `endpoint_kind=built_in` or `custom` plus the selected `model_endpoint`.
The canonical scheduler field is `routing_preference`. For compatibility with
simple local backends, `request_json` also emits `prefer_fast=true` or
`prefer_quality=true` when the preference is non-balanced; balanced requests
omit both boolean aliases.
Use `ChatRequest::wire_snapshot()` when a Web/TUI/CLI host needs the same
send-body contract without parsing JSON. It exposes the display-ready
role/preference labels, `prefer_fast` / `prefer_quality`, optional token-budget
presence, endpoint pin state, endpoint kind, and optional endpoint label that
`request_json` will serialize.
Use `ChatRequest::submission_snapshot_with_history_limit()` when the host needs
the full no-prompt-text send preview: route labels, route-wire aliases,
optional token-budget state, message counts, `single_turn`/`multi_turn`
classification, retained context size, and whether the next accepted submit
will hit or truncate the configured history window. This is the shared service
boundary for Web Lab, Forge TUI, and `norion-cli` request previews; terminal
strings may stay compact, but UI controls should not rederive history pressure
or worker pin state from those strings.
For route-only surfaces that do not have a prompt yet, use
`RoutingIntent::wire_snapshot()` instead. Startup flags, slash-command
previews, status bars, and worker-picker rows should copy those route aliases
instead of rebuilding `prefer_fast`/`prefer_quality` or endpoint pin state by
hand. This keeps Apple Silicon worker selection consistent across Web Lab,
Forge TUI, and `norion-cli`: role/preference changes remain scheduler hints,
and only explicit operator endpoint/worker selection serializes
`model_endpoint`.
Use `ChatRequest::routing_intent()` when a UI/CLI needs to preview the route
before sending. Its `summary()` distinguishes scheduler hints from an explicit
endpoint pin with `pinned=false` or `pinned=true`, so status bars can show
whether the backend remains free to choose a worker.
For display and picker state, use the shared helpers on `RoutingIntent` and
`ChatRequest`: `model_role_label`, `routing_preference_label`,
`endpoint_label`, `endpoint_kind`, `endpoint_kind_label`, `endpoint_auto`,
`endpoint_built_in`, and `endpoint_custom`. These helpers apply the same rule
everywhere: `endpoint_pinned=false` renders as `endpoint=auto` and
`endpoint_kind=auto` even if an embedding host carried an endpoint hint, while
only an operator-pinned request is classified as `built_in` or `custom`.
For inbound SSE, prefer `norion_service::apply_backend_event(&mut ChatSession,
event, data)`. It maps backend `delta`, `status`, `stage`, `heartbeat`, `meta`,
`final`, `done`, `error`, `queued`, `busy`, and `backpressure` events onto the
shared `ChatChunk`/`StreamState` model. If EOF arrives without `done`, call
`close_incomplete_stream`; it marks streams with partial output as
`Interrupted`, pressure states as `Interrupted` with the latest
queued/busy/backpressure reason, and streams without partial output or pressure
as `Failed`.
Use `StreamState::is_terminal`, `StreamState::is_pressure`, and
`StreamState::blocks_prompt_submit` as the shared state classification for
terminal styling, queued/busy/backpressure badges, and duplicate-send
prevention. UI hosts should not keep a separate state table unless they are
adapting these service helpers.
Adapters that receive raw `text/event-stream` data can split on blank lines,
then call `parse_sse_frame` or `apply_sse_frame` for each complete frame. The
parser handles CRLF, comments, default `message` events, and multi-line `data:`
fields; transport code remains responsible for retaining an incomplete trailing
frame until more bytes arrive or EOF is declared.
`SseFrameBuffer` provides that incremental buffering in pure Rust: push each
decoded text chunk, apply all completed frames to the session, and on EOF call
`finish()`. A non-empty leftover means the transport ended mid-frame and should
be closed with `close_incomplete_stream`.
If the adapter parses a complete assistant answer from the backend `final`
payload, call `apply_backend_final_answer(session, payload, Some(answer))`
instead of the generic `final` mapping. That reconciles streamed delta text with
the final answer before `done` records the assistant turn in history. Passing
`None` preserves the streamed text.
After a terminal event or EOF close, adapters can call `ChatSession::outcome()`
to summarize the state, partial/final answer text, pressure reason, error, and
history length. Use `StreamOutcome::snapshot()` or
`StreamOutcomeSnapshot::from_outcome()` for structured UI state, and let
terminal surfaces format that with `norion_cli::outcome_status`;
queued/busy/backpressure outcomes should render their pressure reason without
treating it as `last_error`.
Terminal stream handling is idempotent: after `Completed`, `Interrupted`, or
`Failed`, duplicate `done` frames and late `delta/status/meta/final/error`
frames are treated as no-ops. They do not append assistant history twice,
replace the reconciled final answer, change `last_error`, advance stream state,
or pollute the next prompt's context. A new turn should explicitly call
`begin_stream()` after the next accepted `submit_prompt` to reset partial output
for that request.
User cancel should be normalized through `ChatSession::cancel_stream()`. For
active `Queued`, `Busy`, `Backpressure`, or `Streaming` sessions it emits an
`Interrupted` chunk with `stream cancelled by user`, preserves partial text, and
does not record an assistant turn. `Pending` and terminal sessions ignore
cancel. CLI Ctrl+X and Forge cancel controls should use this helper so manual
cancel behaves like a deliberate interrupted stream rather than a backend
failure.
For CLI/TUI integrations, `CliInput::handle_key_canceling`,
`handle_key_with_gate_and_cancel`, and
`handle_key_with_model_pool_gate_and_cancel` apply that session cancel helper
directly and return `InputAction::StreamCancelled(ChatChunk)` when a stream was
actually interrupted. The existing `InputAction::CancelStream` remains available
for integrations that want an outer transport layer to own cancellation.

The Gemma runtime behind that backend is expected at `127.0.0.1:8686`, but the
CLI/UI should not call it directly. The backend remains the owner of model
runtime, memory, experience hygiene, and readiness checks.

SmartSteam status may optionally carry
`next_round_decision_status` / `next_round_decision_summary`, projected from a
`next_round_decision_report_v1`-compatible live status source. The DTO is
display evidence only: it exposes `safe_to_wait_current_round_active`,
`safe_to_continue_after_current_round`, `operator_attention_blocked`, optional
round numbers, evidence ids, and reason codes while keeping
`read_only=true`, `starts_daemon=false`, `sends_prompt=false`,
`starts_stream=false`, `writes_ndkv=false`, and `creates_thread=false`. If the
report is absent, both fields remain `None` and existing status consumers should
behave as before.
Current evolution-loop status producers can feed the optional DTO through
`SmartSteamNextRoundDecisionReportStatusSource`, using report-shaped fields such
as `decision_status`, `display_state` / `live_status_display_state`,
`current_round_active`, `readiness_can_schedule_next_round`,
`operator_attention_required`, `failure_reasons`, evidence ids, and round ids.
Reports that mark the source as non-read-only or permit process, prompt, stream,
dispatch, memory, or `.ndkv` side effects are ignored by this projection instead
of being displayed as safe status.

Frontend adapters should normalize backend health into `FrontendGateSnapshot`
before sending. `GateDecision::Allowed` means Enter/send may create the next
stream request. `GateDecision::Blocked` carries the `StreamState` and a short
reason that the UI can display without issuing another backend request.
`GateDecision::advice()` converts that decision into a UI action hint:
`send_now`, `wait_for_worker`, `wait_for_current_stream`, `retry_later`, or
`repair_gate`. This advice is a rendering/control hint for status bars and
buttons; `GateDecision` and `StreamState` remain the protocol truth.
`GateAdvice::action_label()` and `state_label()` expose the same stable labels
used by terminal status lines, so adapters can render buttons and pressure
badges without hand-mapping enum variants.
For prompt-send controls, prefer `GateDecision::send_control(prompt_present)`.
The returned `GateSendControl` is the shared Web/TUI/CLI control contract for
send buttons and Enter handling: it carries the advice action/state labels,
`send_allowed`, primary action label, disabled reason, prompt-present flag, and
whether a blocked prompt should be preserved or an allowed prompt should be
cleared. Blocked controls also carry `block_chunk`, a
`ChatChunkDisplaySnapshot` for the queued/busy/backpressure/failed reason, so
send buttons and status strips can render the same label and duplicate-submit
flags as streaming output. `norion-cli` exposes that same object as
`InputReadinessSnapshot.prompt_submit_control` and
`InputControlSnapshot.prompt_submit_control`, so UI hosts can render
`send_now`, `wait_for_worker`, `wait_for_current_stream`, `retry_later`, or
`repair_gate` without reimplementing queued/busy/backpressure policy.
For multi-worker health, normalize per-worker state into
`ModelPoolGateSnapshot` and call `decision_for_intent(request.routing_intent())`.
Global gates still win first: backend offline, safe-device failure, experience
hygiene failure, `engine_busy`, and total queue backpressure are checked before
route-specific worker state. The safety and hygiene gates intentionally take
precedence over busy/backpressure so governance failures are not rendered as
ordinary wait states. With no explicit endpoint, the auto route allows the
backend scheduler to choose any compatible available worker. Worker snapshots
may optionally declare supported `ModelRole` values and `RoutingPreference`
affinities. A worker with no declared roles/preferences is compatible with every
intent for backward compatibility. Once capabilities are declared, the unpinned
auto route only considers matching workers; if no worker matches, the gate
returns `Queued` with a no-match reason instead of stealing an unrelated
backend. With an explicit endpoint pin, a busy worker maps to `Busy`, a
saturated worker queue maps to `Backpressure`, an unregistered worker maps to
`Queued`, and a declared capability mismatch also maps to `Queued` until the
operator changes role, preference, endpoint, or worker registration.
`ModelPoolGateSnapshot` keys endpoint scoping off
`RoutingIntent.endpoint_pinned`, not merely `model_endpoint.is_some()`. If an
embedding UI carries an endpoint label while `endpoint_pinned=false`, the gate,
route status, and worker-picker rows still treat the request as auto-routed.
Only explicit operator actions that set `endpoint_pinned=true` scope capacity
and busy/backpressure to one worker.
Use `ModelPoolGateSnapshot::status()` for status bars that need pool capacity:
it reports total, available, busy, and saturated worker counts plus aggregate
`queued_requests` and `queue_limit`. The summary string intentionally stays
compact for terminal status bars, while structured UI controls can use the
queue-depth fields and helpers (`has_workers`, `has_available_workers`,
`has_busy_workers`, `has_saturated_workers`, `has_queued_requests`, and
`queue_is_saturated`) for badges and throttling hints. Send/no-send decisions
should still come from `decision_for_intent`.
For display-only pool badges, use `capacity_state`, `capacity_state_label`,
`capacity_state_is_pressure`, and `capacity_state_blocks_prompt_submit` instead
of re-deriving available/queued/busy/backpressure from raw counters.
When worker capabilities are declared, pair that with
`ModelPoolGateSnapshot::route_status(intent)`. The route status reports matching
total, available, busy, saturated, queued, and queue-limit counts for the
current role/preference/endpoint intent, which prevents a UI from showing only
global capacity when the selected reviewer/summary/tester lane has no available
worker or is saturated. With `endpoint=auto pinned=false`, it counts compatible
workers in the scheduler pool. With `pinned=true`, it scopes that capacity to
the selected endpoint so another idle worker does not hide pinned-worker
busy/backpressure. Its helpers (`has_matching_workers`,
`has_matching_available_workers`, `has_matching_busy_workers`,
`has_matching_saturated_workers`, `has_matching_queued_requests`, and
`matching_queue_is_saturated`) are the structured route-lane equivalents of the
global pool helpers, so UI controls do not need to compare counters or split
`route_pool=matching ...` terminal text.
Route status exposes the same `capacity_state*` helpers for the current lane,
so an Apple Silicon model picker can badge the selected reviewer, summarizer,
tester, or assistant lane without parsing terminal text.
If the pinned worker declares incompatible capabilities, route status reports
`matching total=0 ...`; the correct operator action is to change role,
preference, endpoint, or worker registration rather than waiting for an
unrelated worker.
For terminal input, `CliInput::handle_key_with_gate` should be used when a
fresh health snapshot is available. A blocked Enter returns `InputAction::Blocked`
with a renderable `ChatChunk`, keeps the user's prompt buffer intact, and does
not create a `ChatRequest`.
When worker-level health is available, prefer
`CliInput::handle_key_with_model_pool_gate` or
`handle_key_with_model_pool_gate_and_record`. Those methods evaluate the
current `CliInputConfig::routing_intent()` before Enter builds a request, so an
operator-pinned busy worker blocks locally while an unpinned `auto` route can
still proceed if another worker is available.
Capability no-match decisions use the same local block path: Enter returns
`InputAction::Blocked`, keeps the prompt buffer intact, and does not create a
`ChatRequest`. The `handle_key_with_model_pool_gate_and_start` path also avoids
recording user history or emitting a start chunk when a pinned worker does not
match the selected role/preference.
Status bars can use `CliInput::routing_summary()` or
`CliInputConfig::routing_intent()` to show the active role, preference, optional
worker, and `pinned=true/false` without reparsing input commands.
For richer terminal/TUI status bars, use `norion_cli::CliStatusSnapshot` or
`cli_status_line`. It combines routing, session stream state, history count,
current history limit, current `max_tokens`, partial-answer character count,
optional `last_error`, and optional gate advice into one stable line. Event
output should still go through `OutputViewport`; status snapshots are for
persistent UI chrome rather than transcript lines. The structured
`model_role_label`, `routing_preference_label`, `endpoint_label`,
`endpoint_pinned`, endpoint-kind fields, `history_limit`, and
`max_tokens_label` fields are intentionally available even though the compact
terminal status line still renders route and token policy as text. State
classification fields (`state_is_terminal`, `state_is_pressure`, and
`state_blocks_prompt_submit`) come from the shared `StreamState` helpers, so
Web/TUI hosts can show route chips, endpoint-kind badges, pin badges,
wait/terminal state, and context-window controls without changing terminal
text.
UI hosts that do not need to parse terminal text should prefer the structured
snapshot fields: `routing_intent` plus route labels for
role/preference/endpoint/pinned display, `pool` / `route_pool` for
worker-capacity badges, `pool_queue_label` / `route_pool_queue_label` plus
the `pool_has_*`, `pool_queue_is_saturated`, `route_pool_has_matching_*`, and
`route_pool_queue_is_saturated` fields for queue and capacity badges,
`pool_capacity_state_label` / `route_pool_capacity_state_label` plus their
pressure and duplicate-submit classification fields for display-ready pool and
route-lane badges,
`workers` for endpoint rows and capability tags, `route_workers` for
worker-picker rows under the current intent,
`gate_advice_detail` for action/state/reason, and `send_allowed` plus
`send_block_state` for send-button enablement. Under a model-pool gate,
`route_gate_advice_detail`, `route_gate_advice_action_label`,
`route_gate_advice_state_label`, `route_gate_advice_reason`,
`route_send_allowed`, and `route_send_block_state_label` preserve the raw
route decision before local session pressure is applied. `InputControlSnapshot`
mirrors those fields and also provides `advice_state_label` / `advice_reason`
for the final send-button hint after local session pressure and governance
gates are merged. This lets a UI show both "this terminal already has an active
stream" and "the selected reviewer lane is queued/ready/backpressured" without
confusing the two. Its `route_options` selector snapshot follows
`readiness.routing_intent`, which is the same route boundary used by
Enter/send and request previews. If an embedding host manually combines a fresh
readiness snapshot with an older status snapshot, `status.routing_intent` may
describe stale status-bar chrome, but `route_options` still describes the next
accepted `ChatRequest` route. `status_route_matches_readiness` and
`status_route_is_stale` make that sampling boundary explicit; when stale is
true, treat `status`, `pool`/`route_pool`, and `route_workers` as status chrome
from an older route and refresh worker health before using those rows as the
active picker. `route_workers`
marks each
registered worker with `endpoint_selected`, `route_match`, `selectable`, and
the worker-local `decision`, so a UI can show compatible auto-route candidates,
pinned-worker pressure, and operator switch targets without parsing
`/workers` text. Each row also carries `picker_action` and
`picker_action_label`: `current` means this is the explicitly pinned endpoint,
`select` means an operator click can pin this endpoint now, `wait` means the
row matches the current route but is queued/busy/backpressured, `repair_gate`
means a frontend governance or backend-readiness gate must be fixed first, and
`unavailable` means the row is not a valid target for the current
role/preference. These labels are the structured worker-picker boundary for Web
Lab, Forge TUI, and CLI controls on Apple Silicon multi-model pools; they
should drive row buttons, wait badges, and disabled states before falling back
to terminal text. These fields use the same precedence as the terminal line:
repair gates block first, then local active session pressure, then ordinary worker-pool
queued/busy/backpressure. The string fields remain stable terminal renderings;
Web/TUI controls should treat the structured fields as authoritative when
building model-role selectors, endpoint pin indicators, worker pickers,
availability displays, and action buttons such as wait/retry/repair.
When a global frontend repair gate is active, such as safe-device, experience
hygiene, or backend offline, `route_workers` keeps the route match and
selection target visible but copies the repair decision into each row and sets
`selectable=false`. This lets an operator see the worker that would be pinned
after repair without presenting an enabled send or pin action while governance
is blocking the route.
Treat the service state sources as layered originals rather than equivalent
strings:

- `FrontendGateSnapshot::decision().display_snapshot(0)` is the original
  frontend-gate display source for offline/repair/busy/backpressure. If a host
  only needs the global gate reason, prefer this over rebuilding an error/busy
  row from `status_line`.
- `ModelWorkerSnapshot::status_display_snapshot()` is the original per-worker
  health source. It owns the stable `busy` / `backpressure` worker labels and
  duplicate-submit flags used by CLI, Forge, and any future host worker list.
- `ModelRouteWorkerSnapshot::decision_display_snapshot()` is the route-scoped
  picker decision source. It explains why a route row is `select`, `current`,
  `wait`, `repair_gate`, or `unavailable`, and it may differ from the worker's
  raw health row when a capability mismatch or frontend repair gate is active.
  Under a safe-device repair gate, the row still carries worker
  role/preference, health, the repair decision display chunk, and pinned
  selection wire fields, while the final picker action remains `repair_gate`.
- `ModelPoolRouteSnapshot::send_block_chunk` is the route-level blocked-send
  summary for the current intent. It is derived from the same route decision as
  `send_block_state*` and should be used for the main send/Enter wait badge,
  while row-level worker lists keep using `route_workers[*]`.
- `ModelPoolRouteSnapshot.workers` should be treated as the same structured
  picker rows returned by `ModelPoolGateSnapshot::route_workers(intent)`. Hosts
  should not maintain separate logic for snapshot rows vs helper rows; both are
  the same route-worker contract carried through different service entry
  points.
- `ModelPoolRouteSnapshot::workers_host_snapshot()` is the service-side
  read-only DTO projection for future Web Lab / Forge consumption. It maps the
  verified route, worker health, picker decision, repair reason, and pinned
  selection-wire fields without carrying request preview, stream chunks,
  input-action snapshots, or history mutation.

In practice, hosts should render the highest-fidelity source they already
have: route send state from `send_block_chunk` / `send_block_state*`, worker
rows from `route_workers[*]`, raw worker health from `workers[*]`, and only
then compact terminal summaries like `route_pool=matching ...` or
`endpoint=... status=...` when a structured source is unavailable.

Evidence anchors for this layering:

- `crates/norion-service/src/gate.rs`:
  `frontend_gate_decision_display_snapshot_is_stable_for_hosts`,
  `worker_status_display_snapshot_is_stable_for_busy_and_backpressure_hosts`,
  `route_workers_keep_frontend_repair_gate_over_worker_availability`,
  `route_snapshot_keeps_same_worker_rows_as_route_workers_helper`,
  `workers_host_snapshot_projects_service_dto_under_repair_gate_without_side_effects`
- `crates/norion-service/src/stream.rs`:
  `status_and_heartbeat_frames_do_not_clear_pressure_gate`,
  `status_and_heartbeat_frames_do_not_pollute_stream_context`,
  `read_timeout_interrupt_does_not_pollute_retry_context`
- `crates/norion-service/src/session.rs`:
  `cancel_stream_interrupts_active_stream_and_keeps_partial_without_history`,
  `cancel_stream_recovery_respects_history_limit_without_partial_context`,
  `timeout_interrupted_snapshot_keeps_retry_gate_open_without_partial_context`
- `crates/norion-cli/src/input.rs`:
  `model_pool_status_and_workers_stay_read_only_under_engine_busy_and_health_preflight`,
  `model_pool_status_commands_stay_read_only_when_route_is_backpressured`,
  `model_pool_status_commands_preserve_structured_host_snapshot_under_gate_pressure`
- `crates/norion-cli/src/status.rs`:
  `workers_host_snapshot_projects_read_only_dto_for_web_and_forge`
- `crates/norion-cli/src/output.rs`:
  `status_and_workers_host_snapshots_keep_local_envelope_under_gates`,
  `workers_snapshot_projects_web_forge_fields_under_repair_gate_without_stream_side_effects`,
  `route_backpressure_host_outputs_preserve_worker_rows_and_route_lane_snapshot`,
  `request_preview_snapshot_exposes_send_boundary_without_prompt_text`,
  `request_preview_can_carry_history_policy_without_changing_terminal_line`,
  `started_turn_preview_snapshot_exposes_start_boundary`,
  `started_turn_preview_keeps_prompt_text_out_of_local_send_line`,
  `stream_outcome_output_carries_structured_terminal_and_pressure_state`,
  `outcome_status_distinguishes_completed_interrupted_and_failed`,
  `outcome_status_preserves_pressure_close_reason_after_incomplete_stream`
- `crates/norion-cli/src/input.rs`:
  `control_snapshot_request_preview_keeps_context_messages_distinct_from_backend_default_tokens`

Each picker row also carries a selection target: `selection_intent`,
`selection_summary`, selection role/preference/endpoint labels, selection
endpoint-kind fields, and `selection_wire_*` aliases. These fields describe the
exact operator-pinned route that should be applied if the user clicks that
worker row. A row can be `selectable=false` because it is busy, saturated, or
capability-mismatched; its selection target still tells the host what route
would be pinned after an explicit operator action, without forcing the UI to
reconstruct `/worker ENDPOINT` from display text.
Role/preference changes (`prefer_fast`, `prefer_quality`, or a reviewer,
summary, tester, or assistant role) do not create a worker pin. In the default
auto route they keep `endpoint_pinned=false`, so the scheduler can choose any
compatible MLX/Metal worker; if an operator has already pinned a worker, they
preserve that explicit pin until the operator selects the auto endpoint. Only
the row selection target, `/endpoint WORKER`, `/worker WORKER`, or a
`/model ... ENDPOINT` command should set `endpoint_pinned=true` and send
`model_endpoint`; selecting the auto endpoint clears that pin.
For row rendering, use `ModelWorkerSnapshot::endpoint_label`, `status_label`,
`status_state`, `status_state_label`, `status_is_pressure`,
`status_blocks_prompt_submit`, `queue_label`, `active_request_label`,
`role_labels`, and `preference_labels` instead of splitting
`ModelWorkerSnapshot::summary()`. `status_label` preserves terminal wording:
`available`, `busy`, or `backpressure`; `status_state` maps that into the
shared `StreamState` contract as `Pending`, `Busy`, or `Backpressure` for UI
badges and send-disable logic. For the current-route decision on a picker row, use
`ModelRouteWorkerSnapshot::decision_action_label`,
`decision_state_label`, `decision_state_is_terminal`,
`decision_state_is_pressure`, `decision_state_blocks_prompt_submit`, and
`decision_reason`; those helpers are derived from the same
`GateDecision::advice()` and `StreamState` contracts as CLI status and Enter
blocking.
When worker-level health is available, use
`CliStatusSnapshot::from_model_pool_gate` or `cli_model_pool_status_line` so the
same line also includes model-pool capacity and the route-specific gate advice
for the current `CliInputConfig`.
Status rendering must still combine the frontend or worker-pool decision with
local session pressure. Even without a fresh gate snapshot, if the local
`ChatSession` is already `Queued`, `Busy`, `Backpressure`, or `Streaming`, the
terminal should show local wait/backpressure advice. When a gate snapshot is
present and ready, local pressure still overrides `send_now`. Local pressure
also overrides ordinary worker-pool `Queued`, `Busy`, or `Backpressure` advice
for duplicate Enter attempts, because the terminal user first needs to know
that their current session is already active. `Failed` repair gates, including
backend-offline, safe-device, and experience hygiene failures, remain higher
priority than local busy state and must not be hidden. This keeps `/status`,
`/workers`, and persistent status bars from inviting a duplicate prompt while
preserving governance failures as repair states rather than ordinary wait
states.
For per-key input results, Web/TUI hosts should call
`InputAction::snapshot(&CliInputConfig)` or `CliInput::action_snapshot` after
handling a key. `InputActionSnapshot` exposes an `InputActionKind`, current
`routing_intent`, route labels, endpoint pin/kind fields, route-wire aliases
(`wire_model_role_label`, `wire_routing_preference_label`,
`wire_prefer_fast`, `wire_prefer_quality`, `wire_endpoint_pinned`,
`wire_endpoint_kind_label`, `wire_sends_model_endpoint`, and
`wire_model_endpoint_label`), request metadata counts for send/start actions,
optional `SessionConfigUpdate`, structured `session_config_update_detail`,
stream pressure state/reason for blocked or cancelled actions, and start
sequence/state for immediate-stream paths. For blocked/cancelled actions,
`stream_chunk` carries the service-owned `ChatChunkDisplaySnapshot`; for
immediate-stream actions, `start_chunk` carries the same snapshot for the start
chunk. Hosts should prefer those snapshots for queued/busy/backpressure labels,
button blocking, and transcript rows, instead of reconstructing display state
from `stream_state_*`, `start_state_*`, or bracketed terminal text. The legacy
fields remain as display-stable aliases:
`kind_label`, `stream_state_label`, and `start_state_label` use the same
`InputActionKind::as_str` and `StreamState::as_str` contracts as terminal
status. For blocked/cancelled actions and start-stream actions, it also mirrors
the shared `StreamState` classification with
`stream_state_is_terminal`, `stream_state_is_pressure`,
`stream_state_blocks_prompt_submit`, `start_state_is_terminal`,
`start_state_is_pressure`, and `start_state_blocks_prompt_submit`. It
deliberately uses message counts and
character counts rather than prompt text, so UI panels can update route chips,
send buttons, config badges, and cancel/wait states without parsing terminal
lines, Rust enum debug output, or copying prompt content into status logs. For
`Send(ChatRequest)` and `StartStream(StartedChatTurn)` actions, the snapshot's
top-level route and `wire_*` fields come from the action's request, not from a
possibly stale render config. For blocked, cancelled, routing, status, and
session-config actions there is no outbound request, so those route fields come
from the supplied current `CliInputConfig` and describe what the next accepted
prompt would use after the wait or local command. For
`RoutingChanged` actions, read the action snapshot's route and `wire_*` fields
instead of parsing `local_status`; `local_status` remains the stable terminal
line.
Before handling Enter, use `CliInput::readiness`,
`readiness_recording`, `readiness_starting`, or their `*_with_gate` /
`*_with_model_pool_gate` variants for a non-mutating
`InputReadinessSnapshot`. Pick the variant that matches the Enter handler the
host will actually call: preview sends return `enter_action=Send` without
recording history, recording sends return `Send` with
`records_user_on_enter=true`, and immediate stream paths return
`StartStream` with both `records_user_on_enter` and
`starts_stream_on_enter` true. `submit_mode_label` exposes that host choice as
`preview`, `record`, or `start_stream`. The snapshot classifies the current buffer as
empty, prompt, status command, worker-status command, routing command,
session-config command, or invalid command; reports the likely Enter action;
and exposes `send_allowed`, `enter_submits_prompt`,
`enter_runs_local_command`, `enter_is_blocked`, `primary_action_label`,
`primary_action_enabled`, `primary_action_disabled_reason`, `block_state`, and
`block_reason` when session pressure, `engine_busy`, safe-device/experience
gates, or pinned-worker pressure would block a prompt. Blocked prompt
readiness also carries `block_chunk`, a `ChatChunkDisplaySnapshot` with the
same output label, appended text, and terminal/pressure/duplicate-submit flags
used by stream output. Web Lab, Forge TUI, and CLI controls should prefer
`block_chunk` for wait rows and disabled send buttons, using `block_state` and
`block_reason` as compatibility aliases. `primary_action_label` is the
display-ready Enter/send control label: `send` for accepted prompt
submission, `show_status` or `show_workers` for local status commands,
`apply_route` for role/preference/endpoint changes, `apply_config` for token
or history policy changes, `fix_command` for invalid slash commands, and the
gate advice label such as `wait_for_worker`, `wait_for_current_stream`,
`retry_later`, or `repair_gate` when Enter is blocked. The same readiness
object also exposes the current route as display-ready fields: `model_role_label`,
`routing_preference_label`, `endpoint_label`, `endpoint_pinned`,
endpoint-kind fields (`endpoint_kind`, `endpoint_kind_label`, `endpoint_auto`,
`endpoint_built_in`, `endpoint_custom`), and route-wire aliases
(`wire_model_role_label`, `wire_routing_preference_label`,
`wire_prefer_fast`, `wire_prefer_quality`, `wire_endpoint_pinned`,
`wire_endpoint_kind_label`, `wire_sends_model_endpoint`, and
`wire_model_endpoint_label`). These fields are populated even when
`enter_action=Blocked`, so a Web/TUI send button can show the selected role,
preference, optional endpoint, and queued/busy/backpressure state without
parsing `RoutingIntent::summary()` or deriving whether an endpoint would be
sent. The default route and role/preference-only selections report
`endpoint_pinned=false` and `wire_sends_model_endpoint=false`; only an operator
endpoint/worker selection reports a pinned endpoint. `buffer_kind_label` and
`enter_action_label` mirror the enum values through `InputBufferKind::as_str` and
`InputActionKind::as_str`, so Web/TUI hosts can branch on stable protocol
labels without parsing Rust enum debug output. Local commands remain available under
busy/backpressure because they do not create a backend request or append
history. For local slash commands, `InputReadinessSnapshot.command_preview`
contains the parsed command result without mutating `CliInputConfig` or
`ChatSession`: routing commands expose the post-command `RoutingIntent` and
stable route summary, session-config commands expose the pending
`SessionConfigUpdate` plus `session_config_update_detail`, and invalid commands
expose the exact input error that Enter would render. `InputCommandPreview`
carries matching `buffer_kind_label` and `enter_action_label` values for
command menus and inline validation. For routing commands it also exposes
display-ready `routing_summary`, role/preference/endpoint labels,
`endpoint_pinned`, and endpoint-kind fields so route preview chips do not need
to call helper methods or parse summary text. The CLI execution path consumes
that same preview contract, so the pre-Enter UI preview and the post-Enter
`InputAction` cannot drift into different role/preference/endpoint or config
behavior.
For token/history commands, `session_config_update_detail` exposes
`kind_label`, `summary`, `changes_max_tokens`, `changes_history_limit`,
`max_tokens`, display-ready `max_tokens_label`, `max_tokens_backend_default`,
and `history_limit`, so controls can update badges without matching on enum
variants or parsing strings such as `max_tokens=backend-default`.
Prompt readiness also includes `advice_action`: `send_now` for an allowed
prompt, `wait_for_worker` for queued worker selection, `wait_for_current_stream`
for local or backend busy pressure, `retry_later` for backpressure, and
`repair_gate` for safe-device/experience/backend-offline failures. UI buttons
should use this structured action rather than mapping raw stream states by
hand. `InputReadinessSnapshot`, `InputControlSnapshot`, and `CliStatusSnapshot`
also carry display-ready `advice_action_label` and pressure state labels
(`block_state_label` or `send_block_state_label`) derived from the same
`GateAdviceAction::as_str` and `StreamState::as_str` contracts. Terminal output
remains a stable line protocol, but Web/TUI hosts should prefer those structured
fields for queued/busy/backpressure/repair rendering instead of parsing
`advice=...` text.
The readiness/control snapshots also expose `block_state_is_terminal`,
`block_state_is_pressure`, and `block_state_blocks_prompt_submit`, using the
same service-side `StreamState` helpers as status/output snapshots.
They also expose `preserves_buffer_on_enter` and `clears_buffer_on_enter`.
Ready prompt sends and successful local commands clear the input buffer;
queued, busy, backpressure, repair-gate, and invalid-command responses preserve
the current draft. Web Lab, Forge TUI, and terminal hosts should use these
booleans for Enter handling instead of guessing from display text, especially
when `request_preview` is still present for a prompt that cannot be submitted
yet.
`InputControlSnapshot` mirrors the readiness-level `enter_*` and
`primary_action_*` fields alongside `send_enabled`, so a host that only binds to
the combined control object can render the same button state without reaching
back into `readiness` or parsing terminal output. It also mirrors the final
`block_chunk` after readiness and status pressure are merged, so a single
control object can render `queued`, `busy`, `backpressure`, or repair wait rows
from the same service display snapshot used by stream/action output.
`CliStatusSnapshot` and `InputControlSnapshot` additionally expose
`send_block_state_*` for the local send button and `route_send_block_state_*`
for the model-pool worker lane. Route fields are `None` when no model-pool gate
is attached, `Some(false)` when the route exists and is currently sendable, and
`Some(true)` for queued/busy/backpressure route pressure.
`send_block_reason` and `route_send_block_reason` mirror the same reason at the
send-boundary level, so Web/TUI hosts can label a disabled send button or
worker-lane wait badge without knowing whether the reason originally came from
session pressure, a frontend gate, or route advice. The compact terminal line
continues to render the existing `advice=...` text.
Prompt readiness also carries a non-mutating `request_preview` for prompt
buffers. It is built through the same `ChatSession::request_for_prompt` plus
`CliInputConfig::apply_to_request` path as Enter, so it shows the outbound
message count, `context_messages`, last-message role/length, last-user length,
optional `max_tokens`, display-ready `max_tokens_label`, stream mode, route labels
(`model_role_label`, `routing_preference_label`, `endpoint_label`), and whether
an endpoint is truly pinned without copying prompt text into status UI.
It also exposes `history_messages`, `context_kind`, `context_kind_label`,
`has_context`, and `is_single_turn`, so hosts can render single-turn versus
multi-turn context badges and the retained-history count without comparing
`context_messages` by hand. `last_message_role_label`,
`last_message_chars`, and `last_message_is_user` let hosts verify that the
outbound request ends with the current user prompt while still keeping prompt
text out of status, wait, and send-preview surfaces.
When the preview is produced from `CliInput::readiness*` or
`InputControlSnapshot`, it also carries session-policy fields for the next
submit: `history_limit`, `history_remaining`,
`history_messages_after_submit`, `history_at_limit_after_submit`, and
`history_truncates_on_submit`. Use these fields to render context-window and
truncation hints next to the send button even while queued/busy/backpressure
is preserving the draft. Standalone action snapshots that only have a
`ChatRequest` leave those policy fields empty rather than guessing a session
limit.
Web/TUI hosts that want one object for the send button, wait/retry/repair
action, route chips, worker picker, and multi-turn context badge can use
`InputControlSnapshot` via
`CliInput::control_snapshot_with_model_pool_gate`. It combines
`InputReadinessSnapshot` and `CliStatusSnapshot` using the same gate/session
precedence as Enter and `/status`; `send_enabled=false` means the host should
not submit or record the prompt. `primary_action_label`,
`primary_action_enabled`, and `primary_action_disabled_reason` are the
display-ready control fields for the current buffer, while `request_preview` may
still be shown as the would-be request once the selected worker or governance
gate clears.
For model-pool controls, it also carries the status snapshot's
`route_gate_advice_detail`, `route_gate_advice_action_label`,
`route_send_allowed`, and `route_send_block_state_label`, so a single control
object can render both the local send button state and the selected route's
worker-lane state.
The same control object mirrors the model-pool capacity fields from
`CliStatusSnapshot`: `pool_status`, `pool_queue_label`, `pool_has_*`,
`pool_queue_is_saturated`, `route_pool_status`, `route_pool_queue_label`,
`pool_capacity_state*`, `route_pool_has_matching_*`,
`route_pool_queue_is_saturated`, and `route_pool_capacity_state*`. These fields
are `None` when no model-pool gate is attached, `Some(...)` for the whole pool
when worker health is attached, and route-pool values only when worker
capabilities make route-lane capacity meaningful. UI controls can therefore
show queued/busy/backpressure badges from the one input-control snapshot
without parsing `/status` output or walking worker rows.
It also mirrors `workers` and `route_workers` at the top level, matching the
same rows carried by `CliStatusSnapshot`. Use `route_workers` for the worker
picker under the current role/preference/endpoint intent; use `workers` for the
full registered endpoint list. Both are `None` when no model-pool gate is
attached.
`InputControlSnapshot.route_options` carries the same role/preference/built-in
endpoint option sets from `norion-service`, plus the canonical `auto` endpoint
label. It exposes those option sets both as the strongly typed
`ModelRole`/`RoutingPreference`/`ModelEndpoint` values and as display-ready
`role_labels`, `preference_labels`, and `built_in_endpoint_labels`, plus
`selected_role_label`, `selected_preference_label`, the current endpoint label,
`selected_endpoint_kind_label`, `selected_endpoint_auto`,
`selected_endpoint_built_in`, `selected_endpoint_custom`, and
`endpoint_pinned`. Web/TUI hosts can render selector choices, selected values,
endpoint-kind badges, and pin badges from one snapshot instead of maintaining a
parallel list of role or preference strings, comparing endpoint text, or
reparsing the status line.
For role and preference dropdowns, prefer `route_options.role_options` and
`route_options.preference_options` over raw label arrays. Each option carries
`selected`, `selection_intent`, `selection_summary`, and `selection_wire_*`
aliases for the exact route that a click would apply. In the default auto
route, choosing a role or preference keeps `endpoint_pinned=false` and
`selection_wire_sends_model_endpoint=false`; if an operator has already pinned
a worker, choosing a new role or preference preserves that explicit pin. Only
an endpoint option, worker row, `/endpoint`, `/worker`, or `/model ... ENDPOINT`
selection should create or clear a worker pin.
For endpoint dropdowns, prefer `route_options.endpoint_options` over raw
`built_in_endpoint_labels`. It contains the Auto row plus built-in endpoints,
each with `selected`, endpoint-kind fields, `selection_intent`,
`selection_summary`, and `selection_wire_*` aliases. The Auto option is the only
unpinned option; built-in endpoint options are explicit operator pins and carry
`selection_wire_sends_model_endpoint=true`. Custom pinned workers remain visible
through `selected_*` but are not falsely marked as one of the built-in endpoint
options.
The route options snapshot also includes `auto_endpoint_selected`,
`auto_selection_intent`, `auto_selection_summary`, and display-ready
`auto_selection_*` / `auto_selection_wire_*` fields for the "Auto" endpoint
row. Use that row to clear an operator-pinned worker without rebuilding a
`RoutingIntent`: it keeps the current role/preference, renders
`endpoint=auto pinned=false`, reports `wire_endpoint_pinned=false`, and leaves
`wire_sends_model_endpoint=false`.
If a host carries an endpoint label while `endpoint_pinned=false`, the selector
kind remains `auto` and the scheduler is still free to choose any compatible
worker.
`InputControlSnapshot.session_policy` names the token/history controls on the
same object: current stored history messages, current `history_limit`, and
optional default `max_tokens`. It also includes a display-ready
`max_tokens_label` (`backend-default` when unset), `history_remaining`, and
`history_at_limit`, plus `has_history` and `is_empty_history`, so Web/TUI hosts
can render token and multi-turn context controls without parsing terminal
lines, formatting `Option` values, comparing counters, or recomputing remaining
context-window slots. This duplicates the relevant
structured status fields intentionally so a single Web/TUI control snapshot can
drive route selectors, session-policy controls, send enablement, and request
preview without depending on CLI text.
When those controls change, call `CliInput::select_model_role`,
`select_model_role_label`, `select_routing_preference`,
`select_routing_preference_label`, `select_model_endpoint`,
`select_model_endpoint_label`, `select_model_route_labels`, or
`select_routing_intent`. These methods update `CliInputConfig` and return
`InputAction::RoutingChanged` without clearing the prompt buffer, mutating
`ChatSession`, or creating a backend request. Invalid role or preference labels
return local `InputAction::InputError` and leave the current route unchanged.
The next accepted Enter is still the only point where the selected route is
attached to `ChatRequest`.
For token and context-window controls, call
`CliInput::set_default_max_tokens`, `set_default_max_tokens_label`,
`set_history_limit`, `set_history_limit_label`, or the generic
`apply_session_config_update` instead of injecting `/max-tokens` or
`/history-limit` into the prompt buffer. These selector-style methods apply the
same `SessionConfigUpdate` contract to `ChatSession`, return local
`InputAction::SessionConfigChanged`, preserve the current prompt buffer, and do
not create a `ChatRequest`. The `*_label` helpers accept the same text values as
the terminal commands, so a Web/TUI text field can use `8192`, `auto`, or `off`
without replaying a slash command. The next readiness snapshot and accepted
Enter use the updated optional `max_tokens` and truncated history through the
same `ChatSession::request_for_prompt` / `submit_prompt` path as terminal slash
commands.
If a host needs non-mutating validation before applying a text field, call
`SessionConfigUpdate::default_max_tokens_from_label` or
`SessionConfigUpdate::history_limit_from_label`, render the returned summary or
input error locally, and only apply the update once the operator confirms. This
keeps token/history validation out of prompt text and still shares the exact
parser used by `/max-tokens` and `/history-limit`.

Use `ChatSession::request_for_prompt` when an adapter needs a non-mutating
preview. Use `ChatSession::submit_prompt` or
`CliInput::handle_key_with_gate_and_record` for real sends: the request includes
the current history plus the new user prompt, and the session records that user
turn exactly once. If the gate blocks the send, the CLI path leaves the prompt
buffer and session history unchanged.
Routing hints are attached to that `ChatRequest` after the multi-turn context
is assembled: role and preference travel with the request, `max_tokens` keeps
its optional backend-default policy, and `model_endpoint` remains absent unless
the operator explicitly selected a worker. The service protocol provides
`ChatRequest::with_routing_intent` as the atomic boundary for this handoff:
`ModelRole` and `RoutingPreference` are always copied, but
`ModelEndpoint` is copied only when `RoutingIntent.endpoint_pinned=true`.
Unpinned intents render and serialize as `endpoint=auto pinned=false`, even if
an embedding UI accidentally carried an endpoint label alongside the auto
route.
Terminal and TUI hosts can render the accepted request with
`norion_cli::request_preview_status` or
`OutputViewport::append_request_preview`. Hosts that need structured data should
read `OutputUpdate.request_preview` from the append helper, or use
`RequestPreviewSnapshot::from_request` / `from_started_turn` when no
`OutputUpdate` is being produced. The preview
includes route intent, display-ready role/preference/endpoint labels,
`pinned=true/false`, total message count, `context_messages` before the newly
submitted prompt, last-message role/length/is-user fields, last user prompt
length, `max_tokens`, stream mode, and for start-immediate paths the start
sequence/state plus `start_state_label`, but not the prompt text itself. This
gives the operator a visible send boundary
without leaking prompt contents into local status logs, and lets Web/TUI hosts
show whether a send is single-turn or carrying multi-turn context without
reparsing `ChatRequest.messages` or route summary text.
For defensive real-send paths, prefer `ChatSession::try_submit_prompt` or
`CliInput`'s guarded recording handlers. They refuse a new prompt while the
session is already `Queued`, `Busy`, `Backpressure`, or `Streaming`, return a
renderable blocked chunk, and do not append a user turn to history. This guards
against a UI accidentally recording a second user prompt while a previous
stream is still active.
Adapters that immediately open a stream after accepting a prompt can use
`ChatSession::try_submit_and_begin_stream`. It returns both the outbound
`ChatRequest` and the initial `start` chunk, or the same blocked chunk as
`try_submit_prompt`. This keeps request creation, user-history recording, and
stream-start emission in one atomic UI action.
For CLI/TUI surfaces that start transport immediately after Enter, use
`CliInput::handle_key_starting`,
`CliInput::handle_key_with_gate_and_start`, or
`CliInput::handle_key_with_model_pool_gate_and_start`. These return
`InputAction::StartStream(StartedChatTurn)` with both the routed request and
start chunk. Under a model-pool gate, the request still carries the assembled
multi-turn history, optional `max_tokens`, role, and preference. It keeps
`endpoint=auto pinned=false` unless the operator selected an endpoint before
pressing Enter, so the scheduler can pick an available compatible worker at the
actual send boundary. The older `InputAction::Send(ChatRequest)` path remains
useful for adapters that create the transport in a separate layer.
Use `norion_cli::started_turn_preview_status` or
`OutputViewport::append_started_turn_preview` when the terminal should show that
the send was accepted and the local session entered `Streaming` before the
transport begins consuming backend chunks.
After a stream finishes or stops, use `norion_service::StreamOutcomeSnapshot`
for structured UI state and `norion_cli::outcome_status` for terminal text. The
snapshot exposes `state`, `history_messages`, completed `answer_chars`,
nonterminal/interrupted `partial_chars`, `last_error`, and
`pressure_reason`. This keeps completed answers, interrupted partials, and
queued/busy/backpressure waits distinguishable without forcing Web/TUI
surfaces to parse strings such as `backpressure reason=...`.

## Model Pool Routing

The pure protocol types are:

- `ModelEndpoint`: a physical or named worker, for example `quality-12b`,
  `fast-reviewer`, `summary-tester`, or a custom worker label such as
  `mlx-reviewer-8b`.
- `ModelRole`: the logical duty requested by the UI, currently `assistant`,
  `reviewer`, `summarizer`, or `tester`.
- `RoutingPreference`: scheduler intent, currently `balanced`, `prefer_fast`,
  or `prefer_quality`.
- `ModelRole::ALL`, `RoutingPreference::ALL`, and
  `ModelEndpoint::BUILT_INS`: stable option sets for UI/CLI selectors. Custom
  worker labels remain valid through `ModelEndpoint::Worker`, but the built-in
  list gives Apple Silicon frontends a no-hardcoded default picker. Treat
  `ModelEndpoint::is_auto_label(...)` as the shared parser for labels that
  clear an operator pin.
- `RoutingIntent::auto_route(...)` and `RoutingIntent::operator_pinned(...)`:
  constructors for the two route modes. Use `auto_route` for normal UI choices
  such as role plus fast/quality preference, and `operator_pinned` only after an
  explicit endpoint/worker selection.
- `ModelWorkerSnapshot.roles` / `preferences`: optional worker capability
  declarations used by the frontend gate before a request is sent. Empty lists
  mean "compatible with any intent"; non-empty lists let Apple Silicon pools
  keep reviewer, summary, tester, and quality lanes from stealing each other.
- `ModelRouteWorkerSnapshot`: a per-worker view for the current intent. It
  copies the worker row and exposes whether the endpoint is currently selected,
  whether it is in the route pool, whether it is selectable now, and what
  worker-local decision would block it. Decision helper methods expose
  action/state labels, pressure/terminal/submit-blocking classification, and
  the operator-facing reason. Selection fields (`selection_intent`,
  `selection_summary`, selection endpoint-kind labels, and `selection_wire_*`
  aliases) describe the pinned route produced by choosing that row; built-in
  workers report `selection_wire_endpoint_kind_label=built_in`, while custom
  labels such as `mlx-reviewer-8b` report `custom`.
- `ModelPoolRouteSnapshot`: the one-call route panel for UI/CLI adapters. It
  packages the current `RoutingIntent`, route labels, endpoint pin state,
  endpoint-kind fields (`auto`, `built_in`, or `custom`), pool-level decision,
  full pool status, route-specific capacity, `decision_state_*` /
  `send_block_state_*` classification fields, and `ModelRouteWorkerSnapshot`
  rows so a frontend does not compute those values from separate helpers with
  subtly different timing.

On Apple Silicon, the intended worker pool is one high-quality 12B lane plus
smaller specialized workers for review, summaries, and test feedback. UI/CLI
controls should default to `balanced` and no pinned endpoint. A user can choose:

- `prefer_quality` for normal chat or deep code reasoning, routed toward the
  12B quality worker when it is idle.
- `prefer_fast` with `ModelRole::Reviewer` for quick review passes.
- `ModelRole::Summarizer` or `ModelRole::Tester` for summary/test feedback,
  optionally pinned to `summary-tester` during operator debugging.

Pinning `ModelEndpoint` is an operator action, not the default UX. Most prompts
should send only role plus routing preference so the backend scheduler can avoid
overloading a single worker.

For terminal surfaces, route selection should live in the input configuration
or provider settings, not in the text buffer. Pressing Enter should preserve the
current `max_tokens` and history, then attach the configured `ModelRole`,
`RoutingPreference`, and optional `ModelEndpoint` to the outgoing request.
Use `CliInputConfig::routing_intent()` plus
`ChatRequest::with_routing_intent` (or `CliInputConfig::apply_to_request`) for
that final handoff so role/preference selection cannot accidentally become a
worker pin. Only `/endpoint`, `/worker`, `--endpoint`, `--worker`, or a
`/model ... ENDPOINT` argument should create `endpoint_pinned=true`.
The end-to-end path is intentionally the same for Web Lab, Forge TUI, and
`norion-cli`: selectors update `CliInputConfig`, readiness builds a
non-mutating `request_preview` from `ChatSession::request_for_prompt`, and the
accepted Enter/send path applies the same routing intent after history and
optional `max_tokens` are assembled. In the default path a role change,
`prefer_fast`, or `prefer_quality` remains an auto-route scheduler hint
(`endpoint=auto pinned=false`, `wire_sends_model_endpoint=false`). A concrete
worker label appears in `ChatRequest.model_endpoint` only after the operator
selects an endpoint row or endpoint command that explicitly pins it.
Graphical route selectors should use the `CliInput::select_*` methods instead
of injecting slash commands into the prompt buffer. Use the label variants when
the UI carries string values from menus or persisted settings; they share
`ModelRole::from_label`, `RoutingPreference::from_label`, and
`ModelEndpoint::from_label` with slash command parsing, but invalid role or
preference labels stay local input errors instead of becoming prompts. Slash
commands remain the terminal text interface; selector methods are the
structured UI interface, and both paths share the same `RoutingIntent` pin
boundary.
`CliInput::select_model_route_labels(role, preference, endpoint)` is the
structured counterpart to `/model ROLE [PREFERENCE] [ENDPOINT]`: role is
required, preference is optional, and endpoint is optional. Omitting endpoint
preserves the current pin state; passing `auto`, `default`, or `none` clears an
operator pin; passing any other endpoint label explicitly pins that worker.
If a host needs to validate or preview persisted route settings without
mutating live input state, call `CliInputConfig::with_model_route_labels` on a
clone of the current config. It returns a new config or a local input error and
uses the same role/preference/endpoint parser as `/model` and
`select_model_route_labels`.
`norion-cli` recognizes routing commands as input-layer commands rather than
prompts:

```text
/role reviewer
/prefer fast
/endpoint fast-reviewer
/worker fast-reviewer
/worker auto
/model reviewer fast
/model reviewer fast fast-reviewer
/model reviewer fast auto
/max-tokens 8192
/max-tokens auto
/history-limit 32
/status
/workers
```

`/endpoint auto`, `/worker auto`, `/endpoint default`, and `/endpoint none`
clear the pinned worker so the backend scheduler can choose. The combined
`/model` command uses the same endpoint parsing, so `/model reviewer fast auto`
updates role and preference while clearing any previous operator pin. Without
an endpoint token, `/model reviewer fast` updates role/preference only: an
unpinned route stays `endpoint=auto pinned=false`, while an existing operator
pin remains in force. A bare `/endpoint` or `/worker` is an input error and
keeps the current route unchanged; clearing a pin is an explicit operator
action, not a side effect of an incomplete command.
Any non-empty endpoint or worker label that is not `auto`, `default`, or `none`
is treated as a custom worker label and therefore pins the request to that
named endpoint. This keeps Apple Silicon pools extensible without requiring a
CLI release for each new local worker name.
Unknown or invalid slash commands are local input errors. They must not fall
through to `ChatRequest`, must not clear the prompt buffer, and should render as
a terminal-local `[input] ...` line so an operator can correct the command
without waking a backend worker. Combined commands such as `/model ...` should
apply atomically: if the role, preference, or argument count is invalid, the
existing routing configuration stays unchanged.
Valid routing commands are local too, even in recording or start-immediately
input paths: they return `InputAction::RoutingChanged`, clear only the command
buffer, and do not append session history, emit a start chunk, or wake a model
worker. The next non-command Enter is the first point where a `ChatRequest` is
created.
Hosts that render route changes directly can call
`OutputViewport::append_route_update` or `norion_cli::route_update_status`
with the current `RoutingIntent`. The terminal line stays `[route] ...`, while
the returned `OutputUpdate.route_update` carries the same role, preference,
endpoint kind, pinned state, and `wire_*` aliases that the next
`ChatRequest` preview will use. Role/preference-only updates therefore remain
`endpoint=auto pinned=false` and do not send `model_endpoint`; only an explicit
operator endpoint selection sets `endpoint_pinned=true`.
`/max-tokens` and `/history-limit` are session-configuration commands. In
recording CLI flows they apply directly to `ChatSession`; in non-recording
flows they return `SessionConfigUpdate` so a host UI can apply the same change
itself. They do not create a prompt or backend request. Terminal surfaces should
render successful changes through `norion_cli::session_config_status` or
`OutputViewport::append_session_config_update`, producing local `[config] ...`
lines distinct from backend stream output and input errors. The returned
`OutputUpdate` carries `session_config_update_detail` with the same
`SessionConfigUpdateSnapshot` used by input previews, so Web Lab and Forge TUI
can update max-token controls, history-limit controls, and backend-default
badges without parsing `max_tokens=...` or `history_limit=...` text. Startup flags use
the same optional-token policy: `--max-tokens N` pins a default output budget,
while `--max-tokens auto`, `default`, `backend`, `none`, or `off` leaves
`ChatRequest.max_tokens=None` so the backend applies its own default.
Graphical and embedded UIs should use the structured session-config selector
methods for sliders, steppers, or toggles; slash commands are the terminal text
interface. Both paths share `SessionConfigUpdate`, so changing a max-token
control does not disturb role/preference/endpoint routing, and lowering
`history_limit` truncates stored context before the next multi-turn request is
previewed or submitted.
`/status` and `/state` are local status commands. They return
`InputAction::Status` and should render through `OutputViewport` as `[status]
...`. Under a model-pool gate handler the status line includes worker capacity
and route-specific advice; when worker capabilities are declared it also
includes `route_pool=matching ...` capacity. For pinned endpoints that
route-pool count is intentionally endpoint-scoped, matching the send decision.
Even when that line contains `advice=repair_gate`,
`advice=wait_for_current_stream`, queued, busy, or backpressure wording, it is
still local terminal status rather than a backend stream chunk. It still does
not create a backend request. The output/view contract verifies the same
boundary for busy, backpressure, and repair gate cases: `/status` and
`/workers` remain `local_status` host snapshots, carry no request preview,
input action snapshot, or stream chunk, and do not change history or partial
stream state.
`/workers`, `/worker-status`, and `/endpoints` are local status commands too.
With a `ModelPoolGateSnapshot`, they render the current route, route-specific
advice, pool capacity, and one stable line per worker, for example
`endpoint=fast-reviewer status=busy queue=0/1 active=#9 review`. Without a
model-pool gate they return a local unavailable status instead of falling
through as a prompt. Workers that declare capabilities append `roles=...` and
`preferences=...`, so an operator can see whether a queued auto route is waiting
for a matching reviewer/summary/tester lane rather than any idle process. Those
worker-list lines also include `route_pool=matching ...` when capability filters
are active.
Structured hosts should render the same information from `CliStatusSnapshot`
instead of parsing this line: `workers` carries endpoint/status/queue/capability
helpers, while `route_workers` adds whether the endpoint is selected, matches the
current route, can be selected, and what the worker-local decision would advise.
The snapshot also carries `pool_queue_label` and `route_pool_queue_label` for
compact queue badges without splitting `pool=...` or `route_pool=...` text.

## Stream State

`StreamState` separates transport state from text content:

- `Pending`: request has not started
- `Queued`: request is accepted but waiting for a model worker
- `Busy`: the selected or required worker is currently running another request
- `Backpressure`: the model pool is saturated and the caller should slow down
- `Streaming`: chunks are still arriving
- `Completed`: backend sent `done`
- `Interrupted`: transport ended after partial output, user cancel, or a
  queued/busy/backpressure wait state was closed before `done`
- `Failed`: backend returned a hard error before usable output

When a stream breaks after deltas, keep `ChatSession.partial_answer()` and
surface an interrupted chunk. The UI should show the partial answer and an
inline status instead of discarding output. `/status` and `cli_status_line`
include `last_error=...` after interrupted or failed streams so a terminal can
show why the stream stopped while keeping partial text visible.
If the stream closes while the session is already `Queued`, `Busy`, or
`Backpressure`, preserve that pressure reason in the interrupted close, for
example `missing done after backpressure: pool queue full`. This is a
recoverable wait-state transport interruption, not a hard generation failure,
and the next accepted send should still go through the normal
`try_submit_and_begin_stream` recovery path.
Once the user starts a real follow-up stream with
`try_submit_and_begin_stream` or a `CliInput::*_and_start` handler, the new
`start` chunk clears stale partial text and `last_error`. Status bars should
then show the active `Streaming` turn with `partial_chars=0` and no previous
`last_error`, while the older interrupted transcript remains visible in the
scrollback/output layer.

## Busy Backend UX

If `/health` reports the engine as busy, the CLI/UI should not send another
prompt immediately. Show a short state such as:

```text
Backend is still generating. Wait, cancel the current stream, or retry later.
```

This mirrors the SmartSteam provider-busy behavior and avoids confusing double
sends while the evolution loop or Web/Forge adapter is using the backend at
`127.0.0.1:7878`.

For model pools, `Busy` means a selected worker is occupied, while
`Backpressure` means the pool queue itself is saturated. A UI should keep the
partial transcript visible, disable duplicate sends for that backend or worker,
and either wait, cancel the current stream, or retry after health shows capacity.
When no endpoint is pinned, a single busy worker should not force the UI into
`Busy` if another worker is available; display `pinned=false` and let the
scheduler choose. When `pinned=true`, show the specific worker label and block
duplicate sends to that worker until its health clears.
Use `/workers` or `norion_cli::cli_model_pool_workers_line(input, session,
gate)` for a terminal-friendly worker list before pinning. It exposes
available, busy, and backpressure states without sending a model prompt, and it
combines worker-pool advice with local session pressure just like `/status`.
This lets an operator keep auto routing, switch `prefer_fast`/`prefer_quality`,
or temporarily pin a specific endpoint for debugging without being shown
`send_now` while a local turn is still active.

The shared gate mapping is:

- backend offline -> `Failed`
- safe-device failure -> `Failed`
- experience hygiene failure -> `Failed`
- `engine_busy=true` -> `Busy`
- queue depth at capacity -> `Backpressure`
- pinned worker busy -> `Busy`
- pinned worker queue at capacity -> `Backpressure`
- unregistered pinned worker -> `Queued`

Terminal output should label pressure states by state, not by generic status:
`[queued]`, `[busy]`, and `[backpressure]`. This makes it clear whether the user
is waiting for a worker, blocked by a selected worker, or being throttled by the
pool.
Terminal output should also label recoverable stream closes as `[interrupted]`
instead of `[error]`; failed safe-device, hygiene, or hard backend rejection
chunks remain `[error]`. The service chunk kind may still be `error` for
wire-level compatibility, but terminal UX should key off `StreamState` when the
state carries more precise intent.
`OutputViewport::append_input_action` can render visible local input results:
blocked chunks, stream-cancelled chunks, input errors, routing changes, and
session config changes. It intentionally returns no output for transport-owned
actions such as `Send`, `StartStream`, bare `CancelStream`, `Quit`, and buffer
edits.
When the host has already called `InputAction::snapshot(&CliInputConfig)`, use
`OutputViewport::append_input_action_snapshot` instead of the raw action helper.
It renders the same visible rows but attaches structured payloads where the
snapshot has enough information: `route_update` for routing commands,
`session_config_update_detail` for `/max-tokens` and `/history-limit`, and
pressure stream chunks for blocked/cancelled states. This gives Web Lab and
Forge TUI a single input-snapshot-to-output path without reparsing local
terminal text or rebuilding route metadata after a command has mutated
`CliInputConfig`. The returned `OutputUpdate.input_action_snapshot` carries the
same snapshot back to the host so row renderers, status badges, and command
logs can share the exact input action boundary.
For a structured route-change row, call `OutputViewport::append_route_update`
with the post-command `RoutingIntent`. This is the output-layer counterpart to
`InputCommandPreview.routing_intent` and `InputActionSnapshot.routing_intent`:
the local `[route]` row becomes inspectable without splitting
`role=... preference=... endpoint=... pinned=...`, and the host can confirm
whether the endpoint would be omitted from or included in the next wire
request.
When a host already has a `CliStatusSnapshot`, it can call
`OutputViewport::append_status_snapshot` instead of passing a preformatted
`InputAction::Status` line. The terminal text remains `[status] ...`, but the
returned `OutputUpdate.status_snapshot` preserves send allowance,
queued/busy/backpressure state, route-pool capacity, and worker pressure
without reparsing the line. This is especially important for Apple Silicon
model pools: an unpinned auto route can show `queued` because all matching
reviewer workers are busy while still reporting `endpoint=auto pinned=false`;
only an explicit operator endpoint should display a pinned worker block.
`OutputViewport::append_workers_snapshot` does the same for `[workers] ...`
rows, using `CliStatusSnapshot::workers_line()` as the single terminal
formatter and carrying the same snapshot back on `OutputUpdate.status_snapshot`.
Worker pickers should read `route_workers[].picker_action_label` and
`selection_*` fields from that snapshot instead of inferring select/wait/repair
actions from the rendered worker list.
Every `OutputUpdate` carries the structured `state`, `scroll`, and `source`
enums plus display-ready `output_label`, `state_label`, `scroll_label`, and
`source_label`, so Web/TUI hosts can apply auto-scroll policy, row styling, and
status badges without parsing bracketed terminal labels or mapping
`ScrollIntent` / `StreamState` variants by hand. The source fields distinguish
backend/blocked stream chunks (`stream_chunk`), local route/status/config/send
lines (`local_status`), and explicit wait/retry/repair advice (`gate_advice`).
Use `is_pressure_stream_chunk` when a transcript row should be styled as real
queued/busy/backpressure output; a local `[status] ... advice=busy...` line may
contain busy wording but still reports `local_status` with
`is_pressure_stream_chunk=false`. `OutputUpdate` also carries
`state_is_terminal`, `state_is_pressure`, `state_blocks_prompt_submit`,
`is_stream_chunk`, `is_local_status`, `is_gate_advice`, and
`is_terminal_stream_chunk`, using the same `StreamState` helpers as readiness
and status snapshots. Terminal chunks still force `scroll_to_bottom`;
non-terminal deltas or local status lines use `follow_latest` or
`keep_position` according to the viewport setting.
For `[advice] ...` updates, `OutputUpdate` also mirrors
`gate_advice_detail`, `gate_advice_action_label`,
`gate_advice_state_label`, and `gate_advice_reason`. Web Lab and Forge TUI
should use those structured fields for wait/retry/repair buttons and badges
instead of parsing `wait_for_worker queued: ...` or
`retry_later backpressure: ...` from appended terminal text. Stream chunks and
local status/config/input lines leave the gate-advice fields empty.
For an explicit local send line, call `OutputViewport::append_request_preview`
or `append_started_turn_preview`; those helpers keep transport output separate
from the local protocol status line. Their returned `OutputUpdate` carries
`request_preview=Some(RequestPreviewSnapshot)` with the same route, wire,
token, context, and start-state metadata as the `[send]` text. Web Lab and
Forge TUI should read that field directly when adding transcript rows,
confirming the route that was accepted, or disabling duplicate Enter after a
start-stream preview, rather than reparsing the `[send] ...` terminal line or
rebuilding a second preview object.
When a host has the session history limit at the output boundary, use
`append_request_preview_with_history_limit`,
`append_started_turn_preview_with_history_limit`,
`RequestPreviewSnapshot::from_request_with_history_limit`, or
`from_started_turn_with_history_limit`. These keep the terminal `[send]` text
stable while filling `history_limit`, `history_remaining`,
`history_messages_after_submit`, `history_at_limit_after_submit`, and
`history_truncates_on_submit` on the structured preview row.
For `[route] ...` updates produced from `append_route_update`,
`OutputUpdate.route_update` mirrors `RouteUpdateSnapshot`. It carries
`routing_intent`, display labels, endpoint kind flags, and wire aliases but no
prompt text, token budget, or history. Stream chunks, advice rows, config rows,
status rows, worker rows, input errors, and send previews leave this field
empty. Hosts should use it to update role/preference/endpoint controls without
accidentally treating the local command as a prompt submission.
For `[config] ...` updates, `OutputUpdate.session_config_update_detail` mirrors
the `SessionConfigUpdateSnapshot` from the input boundary: `kind_label`,
`summary`, max-token backend-default state, and history-limit value. Stream
chunks, advice rows, input errors, status rows, and send previews leave this
field empty. Hosts should use it to keep local controls in sync while
preserving the rule that config changes never submit a prompt or pin a worker.
For `[status] ...` updates produced from `append_status_snapshot`,
`OutputUpdate.status_snapshot` mirrors `CliStatusSnapshot`. Stream chunks,
advice rows, config rows, input errors, and send previews leave this field
empty. `[workers] ...` updates produced from `append_workers_snapshot` also set
this field, because worker lists and route-pool status are another projection
of the same status snapshot. Hosts should prefer it for disabling Enter during
`queued`, `busy`, or `backpressure`, for showing whether pressure came from the
current session or the route pool, and for keeping the worker picker unpinned
unless `endpoint_pinned=true`.
For `[outcome] ...` updates produced from `append_stream_outcome`,
`OutputUpdate.stream_outcome` mirrors the service `StreamOutcomeSnapshot`.
Other update types leave it empty. Hosts should prefer it for transcript footer
badges, retry affordances after interrupted streams, and pressure-close
messages where `reason`, `pressure_reason`, and `has_partial` matter.
For status bars, `norion_cli::gate_advice_status` formats the same decision as
an action-oriented line, for example `wait_for_current_stream busy: ...` or
`retry_later backpressure: ...`. `OutputViewport::append_gate_advice` can append
that line when the UI wants an explicit wait/retry hint in addition to the
blocked stream chunk.
For completed or closed turns, prefer `StreamOutcomeSnapshot` over parsing
`outcome_status()`. It carries `state_label`, `reason`, `is_terminal`,
`is_pressure`, `state_blocks_prompt_submit`, `has_partial`, `answer_chars`, and
`partial_chars` so Web/TUI hosts can distinguish completed answers,
interrupted partials, hard failures, and queued/busy/backpressure outcomes with
one structured object. The terminal `outcome_status()` string remains the
stable compact rendering.
If the terminal or TUI appends a local outcome row, use
`OutputViewport::append_stream_outcome`; it renders the same `[outcome] ...`
text and attaches `OutputUpdate.stream_outcome=Some(StreamOutcomeSnapshot)`.
This keeps interrupted partials and pressure closes inspectable without
reconstructing state from `interrupted partial_chars=...` or
`backpressure reason=...` strings.
When a session has already received `queued`, `busy`, or `backpressure` from
the backend, later local submit/status checks should preserve that latest
pressure reason. Do not replace `worker fast-reviewer is busy` or
`pool queue full` with a generic local wait message unless no backend reason is
available.
`OutputViewport::append_gate_decision` renders blocked gate decisions through
the same path; allowed decisions do not append terminal output.

## Command Test Paths

Use the existing backend instead of starting another Gemma runtime:

```powershell
cd D:\rust-norion
cargo run --manifest-path tools\rustgpt-lab\Cargo.toml -- --backend 127.0.0.1:7878 --bind 127.0.0.1:8787
cargo run --manifest-path tools\smartsteam-forge\Cargo.toml -- --provider runtime --backend 127.0.0.1:7878
cargo run --manifest-path tools\smartsteam-forge\Cargo.toml -- --provider runtime --backend 127.0.0.1:7878 --max-tokens default
cargo run --manifest-path crates\norion-cli\Cargo.toml
cargo run --manifest-path crates\norion-cli\Cargo.toml -- --role reviewer --prefer fast
cargo run --manifest-path crates\norion-cli\Cargo.toml -- --role reviewer --prefer fast --endpoint fast-reviewer --max-tokens 8192
cargo run --manifest-path crates\norion-cli\Cargo.toml -- --max-tokens auto
cargo run --manifest-path crates\norion-cli\Cargo.toml -- --max-tokens off
cargo run --manifest-path crates\norion-cli\Cargo.toml -- --prefer quality
cargo test --manifest-path crates\norion-cli\Cargo.toml --test cli_smoke
```

- Web Lab is the browser/proxy path for manual SSE checks, temporary browser
  chat history, max-token controls, follow-output scrolling, and
  `/v1/chat-stream`/`/v1/generate-stream`/`/v1/business-cycle-stream`.
  Its browser context limit is enforced before each send; a limit of one sends
  only the current user prompt instead of leaking older short-term history.
- Forge TUI is the operator path for SmartSteam sessions, provider settings,
  diagnostics, hygiene/repair commands, gate reports, and Ctrl+X stream cancel.
  Its minimum context window is clamped to prompt plus one recent history
  message, with the budget visible through `/context-window`. Forge accepts
  `--max-tokens <count|default>` at startup and `/max-tokens <count|default>`
  in the TUI; `auto`, `default`, `backend`, `none`, or `off` clear the
  explicit budget and let the backend choose.
- `norion-cli` is the small protocol/input/output boundary crate used to pin
  Enter send, Shift+Enter newline, stream chunks, and scroll intent semantics.
  Its protocol shell accepts `--role`, `--prefer`, `--endpoint`/`--worker`,
  `--max-tokens`, and `--history-limit` flags so operator-selected routing can
  be inspected without starting a model or submitting a prompt. Omitting
  `--endpoint`/`--worker`, or passing `--endpoint auto`, leaves worker choice to
  the backend scheduler; for example `--role reviewer --prefer fast` sends
  reviewer/fast hints without pinning a worker. Passing any other non-empty
  `--endpoint` or `--worker` value pins that custom worker label exactly as
  typed.
  Passing `--max-tokens auto` or `--max-tokens off` clears the default budget,
  matching `/max-tokens auto` or `/max-tokens off` in the interactive input
  layer.
  Startup metadata is generated by `CliRuntimeConfig::startup_snapshot()`.
  Structured hosts should read its `routing_intent`, role/preference/endpoint
  labels, `endpoint_pinned`, endpoint-kind fields, `history_limit`, optional
  `max_tokens`, `max_tokens_label`, `route_options`, and `local_commands`
  fields instead of parsing terminal text. `route_options` is the startup
  selector model for
  role/preference/endpoint controls: it includes supported labels, structured
  `role_options`, `preference_options`, built-in endpoint labels, structured
  `endpoint_options`, the `auto` endpoint label, and the selected endpoint pin
  state plus endpoint-kind fields (`auto`, `built_in`, or `custom`). Use those
  kind fields to decide whether to render a custom worker affordance; do not
  infer custom status by comparing strings. The role/preference option rows
  expose the same `selection_intent`, `selection_summary`, and
  `selection_wire_*` aliases as endpoint rows, so startup UIs can preview
  "choose reviewer", "prefer fast", or "prefer quality" without reconstructing
  a route from display text. The same selector snapshot exposes
  `selected_wire_model_role_label`,
  `selected_wire_routing_preference_label`, `selected_wire_prefer_fast`,
  `selected_wire_prefer_quality`, `selected_wire_endpoint_pinned`,
  `selected_wire_endpoint_kind_label`,
  `selected_wire_sends_model_endpoint`, and
  `selected_wire_model_endpoint_label`, so a startup UI can preview the current
  worker-selection contract without reimplementing route serialization.
  Startup snapshots also mirror the request-preview wire aliases
  (`wire_model_role_label`,
  `wire_routing_preference_label`, `wire_prefer_fast`,
  `wire_prefer_quality`, `wire_sends_max_tokens`, `wire_max_tokens`,
  `wire_endpoint_pinned`, `wire_endpoint_kind_label`,
  `wire_sends_model_endpoint`, and `wire_model_endpoint_label`) so a host can
  render the effect of CLI flags before a prompt exists. `--role reviewer
  --prefer fast` sets the role/preference wire hints while leaving
  `wire_sends_model_endpoint=false`; `--worker mlx-reviewer-8b` is the operator
  action that sets `wire_sends_model_endpoint=true`. `CliRuntimeConfig::startup_lines()`
  is the stable terminal rendering of that same snapshot: it prints the route
  summary, history/max-token policy, and local command hints using the same
  `endpoint=auto pinned=false` wording as the interactive status line. The local
  command hints and `--help` must
  include `/role`, `/prefer`, `/endpoint`/`/worker`, `/model`, `/max-tokens`,
  `/history-limit`, `/status`, and `/workers` so operator route changes and
  session policy changes remain terminal-local commands rather than accidental
  prompts. `--help` documents the same worker-pinning boundary, the supported roles
  `assistant|reviewer|summarizer|tester`, the preferences
  `balanced|fast|quality`, and that `auto`/`default`/`none` clears a worker
  pin.
  The `cli_smoke` integration test runs that binary protocol shell with
  default, pinned-worker, help, and invalid-argument invocations. It is a
  no-backend test path for checking startup output and worker-pinning wording
  before attaching a real stream transport.
  In an attached terminal or TUI host, `/status` shows the current route and
  busy/backpressure advice; `/workers` lists pool workers and makes explicit
  whether the operator has pinned an endpoint.
Manual wait checks should cover the three frontend entry points without
starting another model process:

- Web Lab: open the Lab against the existing backend, set role/preference or
  token controls in the UI, then send only when `/health` reports no
  `engine_busy`. While busy, the send control should stay disabled and keep the
  draft/history visible; its control model should still expose the would-be
  route through `request_preview`, `endpoint_pinned=false` by default, and no
  `model_endpoint` unless the operator explicitly chose a worker.
- Forge TUI: use provider/runtime status, `/status`, `/workers`, `/max-tokens`,
  `/context-window`, and Ctrl+X cancel to validate the operator path. A busy
  or saturated backend should render wait/retry advice, preserve the input
  draft, and keep local diagnostics and repair commands usable because they do
  not create a `ChatRequest`.
- `norion-cli`: use the protocol shell and `cli_smoke` for no-backend checks,
  then use `/role`, `/prefer`, `/endpoint` or `/worker`, `/model`,
  `/max-tokens`, `/history-limit`, `/status`, and `/workers` in an attached
  host. Read `InputReadinessSnapshot`, `InputControlSnapshot`, and
  `CliStatusSnapshot` for `send_allowed`, `preserves_buffer_on_enter`,
  `send_block_state_label`, `send_block_reason`, `route_send_block_state_label`,
  and `route_send_block_reason`. When `engine_busy`, a session stream, worker
  busy, or route backpressure blocks Enter, those fields should explain the
  wait while `request_preview` keeps the next prompt's role/preference/history
  boundary visible without recording a user turn.
Request and outcome previews have structured counterparts:
`RequestPreviewSnapshot` carries route intent, route labels, endpoint pin
state, endpoint-kind fields, `context_messages`, `history_messages`, and
`context_kind_label` for the submitted multi-turn context plus
`has_context` / `is_single_turn`,
`last_message_role_label`, `last_message_chars`, `last_message_is_user`, and
`last_user_chars`, and
`StreamOutcomeSnapshot` carries terminal state,
  partial/error, and pressure reason fields for completed, interrupted, queued,
  busy, and backpressure states. `ChatChunkDisplaySnapshot`, available through
  `ChatChunk::display_snapshot()` and copied onto CLI `OutputUpdate.stream_chunk`,
  is the matching per-chunk display contract: it exposes `kind_label`,
  `state_label`, `output_label`, `content_chars`, `appended`, kind booleans, and
  the service-owned terminal/pressure/duplicate-submit state flags. Web Lab,
  Forge TUI, and terminal hosts should render streaming `queued`, `busy`,
  `backpressure`, `interrupted`, and `failed` chunks from that snapshot instead
  of splitting compact terminal strings such as `[queued] waiting`.
  The endpoint-kind fields mirror
  `InputRouteOptionsSnapshot`: `auto` means no worker pin, `built_in` means an
  operator selected a service-known endpoint, and `custom` means an operator
  pinned an arbitrary local worker label. Preview snapshots also expose
  wire-contract aliases: `wire_model_role_label`,
  `wire_routing_preference_label`, `wire_prefer_fast`,
  `wire_prefer_quality`, `wire_sends_max_tokens`, `wire_max_tokens`,
  `wire_endpoint_pinned`, `wire_endpoint_kind_label`,
  `wire_sends_model_endpoint`, and `wire_model_endpoint_label`. Web/TUI hosts
  should use these when showing exactly what `request_json` will send:
  auto-routed role/preference changes report `wire_prefer_fast=true` or
  `wire_prefer_quality=true` when selected, `wire_endpoint_pinned=false`,
  `wire_endpoint_kind_label=auto`, and `wire_sends_model_endpoint=false`, while
  only an explicit `/endpoint` or `/worker` selection sets
  `wire_sends_model_endpoint=true`. Optional token budgets are visible through
  `wire_sends_max_tokens` and `wire_max_tokens`, so a UI can distinguish
  backend-default output length from an operator budget without guessing from
  `max_tokens_label`. These aliases are a CLI projection of
  `ChatRequest::submission_snapshot_with_history_limit()`, which itself uses
  `ChatRequest::wire_snapshot()`, the same service object used by
  `request_json`; route-only aliases also exist on `RoutingIntent` for startup
  and command previews that do not have a `ChatRequest` yet. This keeps the
  service serializer, CLI previews, and startup flags on one protocol boundary.
  Started-turn previews additionally expose
  `start_sequence`, `start_state_label`, `start_state_is_terminal`,
  `start_state_is_pressure`, `start_state_blocks_prompt_submit`, and
  `start_chunk`, so a UI can show that the stream has begun and should block
  duplicate Enter from the same `ChatChunkDisplaySnapshot` contract used by
  ordinary stream output, without reparsing the local
  `[send] ... start_state=streaming` line.
Slash-command previews use the same route-only aliases before a request
exists. `InputCommandPreview` exposes `wire_model_role_label`,
`wire_routing_preference_label`, `wire_prefer_fast`,
`wire_prefer_quality`, `wire_endpoint_pinned`,
`wire_endpoint_kind_label`, `wire_sends_model_endpoint`, and
`wire_model_endpoint_label` for `/role`, `/prefer`, `/endpoint`, `/worker`,
and `/model`. This lets Web Lab, Forge TUI, and terminal hosts render the
pending selector effect without parsing `routing_summary` or waiting for
Enter. `/model reviewer fast auto` reports `wire_endpoint_pinned=false` and
`wire_sends_model_endpoint=false`, even if the current config is pinned, while
`/worker mlx-reviewer-8b` reports `wire_endpoint_pinned=true`,
`wire_endpoint_kind_label=custom`, and `wire_sends_model_endpoint=true`.
Session-policy commands continue to use `session_config_update_detail` for
`/max-tokens` and `/history-limit`; they do not claim a route wire change
until the next prompt is turned into a `ChatRequest`.
Worker-picker UIs should use `CliStatusSnapshot::route_workers` from
`from_model_pool_gate` instead of splitting the `/workers` line. In auto mode,
compatible available workers have `route_match=true` and `selectable=true`.
With a pinned endpoint, only that endpoint has `endpoint_selected=true`; other
compatible workers can still show `selectable=true` as possible operator
switch targets, but they are not in the current route until the operator
explicitly changes `/endpoint` or `/worker`.
Current Web Lab and Forge code does not yet consume this full route-worker
shape directly. Web Lab reads `/api/backend-health` fields such as
`engine_busy`, `active_requests[]`, `readiness_ok`, `safe_device_ok`,
`*_failures`, and `experience_hygiene`, plus `/api/model-pool-status` /
`/api/model-pool-advice` fields such as `workers[]`, `capacity`,
`safe_to_enable_pool_workers`, `next_step`, `reason`, and helper-role counts.
Forge's model-pool provider enforces `read_only=true`,
`launches_process=false`, and `sends_prompt=false` on status/route responses,
then reads worker rows with `role`, `status`, `ready` / `role_ready`,
`base_url`, and `role_block_reason`, and route rows with `route_allowed`,
`reason`, `selected_role`, `selected_base_url`, `resource_precheck`, and
`dependency_precheck`. Those fields are coarse read-only inputs that should map
onto `workers`, `route_pool_*`, `send_block_reason`, and
`route_send_block_reason`; they are not a substitute for
`route_workers[*].picker_action_label`, `decision_display_snapshot()`,
`worker_status_display_snapshot()`, or `selection_wire_*` once a host needs a
real picker. Until those richer fields are exposed to Web Lab or Forge, the
documented integration point is a gap, not an already-complete UI contract.
For that integration step, `CliStatusSnapshot::workers_host_snapshot()` is the
read-only DTO projection to consume before parsing `/workers` text. It returns
`CliWorkersHostSnapshot` with explicit safety flags (`read_only=true`,
`launches_process=false`, `sends_prompt=false`, `starts_stream=false`, and no
request-preview / stream-chunk / input-action payload), route and route-wire
labels, send and route block reasons, pool summaries, and
`CliWorkerHostSnapshot` rows. Each worker row carries role/preference labels,
available/busy/backpressure health, optional worker-health display chunk,
route match/selectability, picker action, decision action/state/reason,
decision display chunk, and pinned `selection_wire_*` fields. The DTO is a
projection of already-built status data; it does not create `ChatRequest`,
does not mutate history, and does not start a stream.
Status bars should read `CliStatusSnapshot.wire_model_role_label`,
`wire_routing_preference_label`, `wire_prefer_fast`,
`wire_prefer_quality`, `wire_endpoint_pinned`,
`wire_endpoint_kind_label`, `wire_sends_model_endpoint`, and
`wire_model_endpoint_label` for the current route. These are route-only aliases
and intentionally exclude token budget fields, which live in
`max_tokens`/`max_tokens_label` and the send request previews. In a busy,
queued, or backpressure state, these status aliases still describe the route
that would be used after the wait clears; they must not be interpreted as a
new backend send.
Internally, `from_model_pool_gate` derives those fields from
`ModelPoolGateSnapshot::route_snapshot`, keeping send advice, capacity badges,
and worker-picker rows on the same route snapshot.
`ModelPoolRouteSnapshot` exposes route labels, endpoint pin state,
endpoint-kind fields (`endpoint_kind`, `endpoint_kind_label`,
`endpoint_auto`, `endpoint_built_in`, `endpoint_custom`), route-wire aliases
(`wire_model_role_label`, `wire_routing_preference_label`,
`wire_prefer_fast`, `wire_prefer_quality`, `wire_endpoint_pinned`,
`wire_endpoint_kind_label`, `wire_sends_model_endpoint`, and
`wire_model_endpoint_label`), decision/advice labels, decision reason,
`send_allowed`, optional `send_block_state_label`, `send_block_chunk`, pool
status, route-pool status, queue badge labels, and capacity-state labels for
both the whole pool and the current route as structured fields. The
`send_block_chunk` is a `ChatChunkDisplaySnapshot`; Web Lab, Forge TUI, and
terminal hosts should prefer it for queued/busy/backpressure display because it
uses the same service-owned `output_label`, `appended`, and duplicate-submit
flags as streaming chunks.
Web/TUI hosts should render those fields directly instead of
recomputing send-button state or labels from `RoutingIntent`, `GateDecision`,
or compact terminal status strings.
`CliStatusSnapshot::from_model_pool_gate` copies those service route-wire
fields into the CLI status snapshot, so model-pool status bars show the same
auto-vs-pinned worker contract as startup, slash-command preview, and actual
`ChatRequest` serialization.
It also copies `route_send_block_chunk`; ordinary status snapshots expose
`send_block_chunk` for the currently active session or frontend gate. Control
surfaces should use `InputControlSnapshot.block_chunk` for the immediate Enter
state and `InputControlSnapshot.route_send_block_chunk` when they need to show
the route-specific worker-pool wait separately from a session-level wait.
The terminal `/workers` renderer also reuses that route snapshot for advice and
capacity text, so terminal output, status snapshots, and worker-picker controls
share the same auto-vs-pinned route decision.
Each route-worker row can be rendered with service helpers rather than ad-hoc
string formatting: `endpoint_label` for the worker name, `worker_status_label`
for terminal wording, `worker_status_state_label` plus
`worker_status_is_pressure` / `worker_status_blocks_prompt_submit` for
available/busy/backpressure UI state, `decision_action_label` for
send/wait/retry, `decision_state_label` plus decision state classification
helpers for queued/busy/backpressure/failed, and `decision_reason` for the
operator-facing explanation. When a row needs a badge or tooltip for a busy,
queued, saturated, capability-mismatch, or repair-gate state, prefer
`worker_status_display_snapshot()` and `decision_display_snapshot()`; both
return the same `ChatChunkDisplaySnapshot` shape used by stream chunks and send
controls, including `output_label`, `appended`, and duplicate-submit flags.
For a click target, use the row's
`selection_intent` or `selection_wire_*` fields instead of rebuilding a pinned
endpoint from `endpoint_label`; this keeps custom-worker pins and built-in
worker pins on the same protocol boundary as `ChatRequest` serialization.

Before sending a prompt, check `/health`. If `engine_busy=true`, wait for the
active request summary to clear instead of submitting another stream. The Web
Lab send button and Lab REPL both block busy or failed readiness states; Forge
reports the busy request and lets the operator retry after the backend becomes
idle. CLI/TUI hosts should mirror that through `CliInput::readiness_with_gate`,
`readiness_with_model_pool_gate`, or `control_snapshot_with_model_pool_gate`:
`engine_busy` renders as busy pressure with `wait_for_current_stream`, while
saturated queues render as backpressure with `retry_later` and worker
capability misses render as queued with `wait_for_worker`. Those readiness and
control snapshots still carry route labels and `wire_*` aliases during the
wait. A default or role/preference-only route continues to report
`endpoint_pinned=false` and `wire_sends_model_endpoint=false`; only an explicit
operator endpoint or worker selection pins a worker for the next accepted
`ChatRequest`.

## Token and History Policy

`max_tokens` is an optional output budget and must not be hardcoded to `128`.
The caller may pass values such as `4096` or `8192`; if absent, the backend
default applies. History retention is separately controlled by
`ChatSessionConfig.history_limit`, so UI defaults can be tuned without changing
model output length.
Runtime controls should update session policy through
`ChatSession::set_default_max_tokens` and `ChatSession::set_history_limit`.
Clearing the token budget sends no `max_tokens` field and lets the backend
choose its default. Lowering the history limit immediately truncates the oldest
messages so the next `submit_prompt` cannot accidentally carry more context
than the UI currently displays.
