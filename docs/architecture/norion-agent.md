# norion-agent

`crates/norion-agent` is a standalone Rust crate for multi-agent coordination.
It stays runtime-agnostic even when the root workspace includes it: the main
window can decide how it connects to `norion-core`, `norion-memory`, and the
model service.

## Scope

- `task`: `AgentRole`, `AgentTask`, `AgentTaskQueue`, `AgentResult`, and
  dispatch planning. `TaskDispatchPlanSummary` and
  `TaskDispatchGateDecision` expose assigned/rejected counts, aggregate
  remaining budget, rejection pressure, and execution admission before an
  engine adapter runs. `AgentTaskQueue::with_repair_first` is the shared
  repair-first queue combiner: it inserts deterministic repair tasks and adds
  those repair ids as dependencies of normal business tasks, so adapter, run,
  ports, and scheduler handoffs do not carry separate merge semantics.
  `TaskDispatchPlanSummaryHistory`,
  `TaskDispatchDashboard`, and `TaskDispatchHealth` aggregate assignment,
  rejection, empty-dispatch, and remaining-budget pressure across turns so
  service/eval can repair dirty dispatch trends before opening `EnginePort` or
  downstream side-effect gates. A task that arrives after its role budget has
  already been exhausted is rejected without debiting another role or reopening
  the dispatch gate. Dispatch health and history records expose
  `allows_service_advance` and `requires_repair_first`, preserving watch-level
  observation while blocking repair-level dispatch drift before engine calls.
- `budget`: per-role `AgentBudget` and `BudgetLedger` for isolated token/step/message limits.
  `BudgetLedgerSummary`, `BudgetLedgerSummaryHistory`, and
  `BudgetLedgerHealth` expose role counts, depleted roles, remaining totals, and
  zero-budget pressure across turns so one agent's exhausted allowance cannot be
  hidden by another role's available budget. Budget health and history records
  expose `allows_service_advance` and `requires_repair_first`, letting
  service/eval observe empty or watch-level ledgers while forcing repair before
  dispatch or writer gates when depletion is repair-level. Use
  `BudgetLedgerHistoryGate` when a fresh ledger snapshot must be admitted
  against persisted budget health: current zero-budget roles or repair history
  close dispatch and side-effect promotion, then emit deterministic
  `budget-ledger-repair` planner tasks. The
  `BudgetLedgerSummaryHistoryRecorder::record_ledger_with_health_gate` method
  is the append-and-gate packet for
  service/eval rows that need ledger health, dispatch admission, side-effect
  admission, repair tasks, and telemetry bound to the same budget snapshot.
- `aggregate`: `MessageAggregator` deduplicates messages into stable
  `AggregationReport` rows, sorting duplicate source ids and evidence so
  multi-window result arrival order cannot change the aggregate identity, while
  `AggregationHistoryGate` prevents duplicate pressure from reaching downstream
  owners. `AggregationConflictReviewer`
  composes aggregation health with `ConflictResolver` and
  `ConflictReportHistoryGate` so service/eval can use one pure-data packet
  before memory, reflection, or adaptive-state promotion: clean aggregation and
  clean conflict evidence can forward/promote, while aggregation repair or
  unresolved conflict pressure closes both paths and carries deterministic
  repair tasks. Persist `AggregationConflictReviewSummary` rows in
  `AggregationConflictReviewSummaryHistory` when eval needs payload-free trend
  health for repeated duplicate pressure, unresolved conflict pressure,
  forward/promotion closure, and combined repair-task volume. Apply
  `AggregationConflictReviewTrendGate` when persisted review health must
  constrain the next message boundary: stable/watch history preserves the
  current review, while repair history closes forward and side-effect promotion
  and emits deterministic `aggregation-conflict-review-trend-repair-*` work.
- `collaboration`: `AgentWindowSpec`, `AgentCollaborationPlanner`,
  `AgentCollaborationReview`, `AgentWindowOwnershipReviewer`, and
  collaboration review history/health types for turning active windows into a
  queue plus isolated budgets, projecting cycle reports into
  memory/adaptive-state gate summaries, and tracking cross-cycle collaboration
  pressure for the main window. The ownership reviewer compares each window's
  declared owned paths with its reported changed paths, normalizes path
  separators/case, blocks shared or out-of-bounds writes, and returns
  deterministic `collaboration-ownership-repair-*` planner tasks before
  memory notes, side-effect admission, or normal queue promotion can proceed.
  `AgentWindowOwnershipReviewSummaryHistory` and
  `AgentWindowOwnershipReviewHealth` persist the same ownership boundary as
  payload-free eval rows, aggregating write admission, repair-first pressure,
  conflicting paths, out-of-bounds writes, repair task ids, and reasons across
  handoffs. `AgentCollaborationOwnershipPreflightGate` composes that ownership
  health with the dispatch preflight record before scheduler or side-effect
  consumers advance: repair ownership history closes queue dispatch, memory-note
  promotion, side-effect-boundary admission, and next-task promotion, then
  prepends the deterministic ownership repair tasks to the repair queue.
  `AgentCollaborationOwnershipPreflightGateSummaryHistory` and its dashboard,
  health, and recorder types persist that composed boundary as flat service/eval
  evidence for dispatch rate, side-effect-open rate, memory-note promotion
  rate, next-task promotion rate, ownership repair-task pressure, merged repair
  queue size, and blocker count across resumes.
  `AgentHelperRoleRepairRoutingReport` is the clean-room bridge for incomplete
  helper evidence across summary, router, review, index, and test-gate helpers:
  it emits only pure data, strips dirty evidence/helper identifiers down to
  sanitized refs, closes side-effect dispatch/thread/message lanes, and
  materializes deterministic role-matched repair tasks on aggregator, planner,
  reviewer, memory-curator, and tester lanes. `AgentReviewHelperRepairRoutingReport`
  remains the review-specific compatibility surface.
  The collaboration plan gate exposes a compact pre-dispatch read model for
  six-window completeness, duplicate-window blockers, shared-role budget
  collisions, zero-budget windows, queue dispatch admission, and
  side-effect-boundary admission before any service adapter owns effects.
  `AgentCollaborationPlanSummaryHistory`, `AgentCollaborationPlanDashboard`,
  and `AgentCollaborationPlanHealth` aggregate six-window completeness,
  duplicate-window, shared-budget, zero-budget, blocked-reason, and schedule
  rate evidence across planning turns. Use `AgentCollaborationPlanHistoryGate`
  when a fresh plan must be admitted against persisted plan health: current
  plan blockers or repair history close queue dispatch and side-effect
  boundaries, then emit deterministic `collaboration-plan-repair` tasks plus a
  repair-only queue for the scheduler.
  `AgentCollaborationPlanSummaryHistoryRecorder::record_plan_with_health_gate`
  is the append-and-gate packet for service/eval rows that need the plan
  summary, health record, dispatch admission, repair tasks, and telemetry bound
  to the same six-window plan. `AgentCollaborationDispatchPreflight` composes
  that plan append-and-gate row with `BudgetLedgerHistoryGate` and
  `RecursiveAgentScheduleHistoryGate` into one pre-dispatch service/eval record:
  queue dispatch and side-effect-boundary admission open only when the current
  six-window plan, isolated budget ledger, and recursive schedule all remain
  admitted by their persisted health. Repair pressure from any layer is merged
  into a deterministic repair queue before `norion-core` consumes the plan.
  `AgentCollaborationDispatchPreflightSummary` is the payload-free eval row for
  that same boundary, carrying record counts, health statuses, admission flags,
  repair-task counts, blocker counts, schedule waves, and telemetry without
  expanding nested plan, budget, or schedule payloads.
  `AgentCollaborationDispatchPreflightSummaryHistory`,
  `AgentCollaborationDispatchPreflightDashboard`, and
  `AgentCollaborationDispatchPreflightHealth` aggregate those flat rows across
  scheduler boundaries so service/eval can see repeated dispatch closure,
  side-effect-boundary closure, repair-first pressure, per-layer repair tasks,
  blocker counts, and schedule-wave evidence before the next handoff.
  `AgentCollaborationDispatchPreflightSummaryHistoryRecorder::record_preflight_with_health`
  is the append-and-health packet for persisting that trend without expanding
  the nested preflight record. Use
  `AgentCollaborationDispatchPreflightHistoryGate`, or
  `record_preflight_with_health_gate`, when persisted preflight drift must
  constrain the current boundary: stable history preserves queue and
  side-effect admission, watch history remains observable, and repair history
  appends deterministic `collaboration-dispatch-preflight-repair` tasks before
  any `norion-core`, service, memory, or eval owner consumes the queue.
  `AgentCollaborationDispatchPreflightSchedulerHandoff` is the scheduler-facing
  projection of that gate: callers provide the requested business queue and the
  handoff returns the effective queue, preserving the business queue on stable
  admission and using `AgentTaskQueue::with_repair_first` on repair-first
  history or current preflight blockers. Repair tasks become dependencies of
  the requested business queue, so `norion-core` schedules repair waves before
  the normal six-window work can resume. Its summary gives eval a payload-free
  row for requested queue size, effective queue size, repair queue size,
  admission flags, and blocker count. Persist those summaries in
  `AgentCollaborationDispatchPreflightSchedulerHandoffSummaryHistory` and append
  them with
  `AgentCollaborationDispatchPreflightSchedulerHandoffSummaryHistoryRecorder`
  when service/eval needs trend health over dispatchable handoffs, side-effect
  boundary admission, repair-first handoffs, repair-queue dependency injection,
  queue pressure, and blocker pressure before another scheduler boundary opens.
  `AgentCollaborationDispatchPreflightSchedulerHandoffHistoryGate`, or
  `record_handoff_with_health_gate`, applies that persisted handoff health back
  to the current scheduler handoff: stable history preserves the effective
  queue, while repair history closes queue dispatch and side-effect-boundary
  admission, then emits deterministic
  `collaboration-scheduler-handoff-repair-*` tasks through
  `AgentTaskQueue::with_repair_first`, preserving the current effective queue
  behind repair dependencies before `norion-core`, service, memory, or eval
  consumers advance.
  `AgentCollaborationDispatchPreflightSchedulerHandoffHistoryGateSummary` is
  the payload-free row for that final decision, carrying health status,
  admission flags, repair ids, effective queue ids, reasons, and telemetry.
  Persist those rows in
  `AgentCollaborationDispatchPreflightSchedulerHandoffHistoryGateSummaryHistory`
  and append them with
  `AgentCollaborationDispatchPreflightSchedulerHandoffHistoryGateSummaryHistoryRecorder`
  so service/eval can detect repeated final-gate repair pressure, closed
  scheduler admission, closed side-effect boundaries, repair-task volume, and
  effective-queue pressure before the next `norion-core` boundary opens. Apply
  `AgentCollaborationDispatchPreflightSchedulerHandoffHistoryGateTrendGate`
  when that final-gate trend must constrain the next handoff: stable/watch
  trend health preserves the current final gate, while repair trend health
  closes dispatch and side-effect admission and injects deterministic
  `collaboration-scheduler-handoff-gate-trend-repair-*` work through
  `AgentTaskQueue::with_repair_first`, so the current effective queue resumes
  only after trend repairs complete. Persist
  `AgentCollaborationDispatchPreflightSchedulerHandoffHistoryGateTrendGateSummary`
  rows in
  `AgentCollaborationDispatchPreflightSchedulerHandoffHistoryGateTrendGateSummaryHistory`
  when eval needs payload-free trend-gate health for repeated trend-enforced
  repair, closed dispatch, closed side-effect admission, effective-queue
  pressure, and trend repair-task volume.
  Review dashboards also carry blocked
  side-effect gate pressure so service/eval can distinguish conflict or
  reflection blockers from ordinary low admission rates. It also exposes
  `AgentCollaborationBusinessLoopPlanner` as a stricter promotion gate that
  combines ledger admission with collaboration health before adaptive-state
  writes, plus collaboration service-execution history/health types for
  tracking receipt-close drift after the service attempts those commands.
  Side-effect boundary handoff, handoff-monitor, monitor-handoff, adapter
  admission monitor, and adapter admission monitor-handoff records expose
  `allows_service_advance` and `requires_repair_first` directly over their
  gate decisions, giving service/eval a uniform wrapper-level admission read
  model before command, memory-note, or adaptive-state effects open. The direct
  side-effect boundary and adapter admission rows also carry
  `service_execution_tool_build_command_reason_count` beside the existing
  command-reason and memory-promotion counters, so tool-build repair pressure is
  visible before any service adapter executes commands. Side-effect boundary
  monitor-handoff summaries, dashboards, history records, and gates preserve the
  same count into adapter admission; adapter admission monitor and
  monitor-handoff summaries, dashboards, history records, and gates keep it
  visible again before the next service-adapter owner gate.
  `AgentCollaborationSelfEvolutionPlanner` combines both collaboration review
  health and receipt-close health into the final self-evolution admission, and
  `AgentCollaborationSelfEvolutionCloser` closes receipts, records the
  receipt-close trend, and returns the final admission packet in one step.
  `AgentCollaborationSelfEvolutionCloseHistory` then aggregates those final
  close rows into cross-turn dashboard/health evidence for self-evolution.
  Review, service-execution, and self-evolution close history records expose
  `latest()`, `records()`, `allows_service_advance`, and
  `requires_repair_first`, giving service/eval a uniform read model before
  promotion or command planning advances.
  `AgentCollaborationSelfEvolutionController` turns that health plus the next
  queue into continue/observe/repair/idle scheduling intent, and
  `AgentCollaborationSelfEvolutionControlHistoryRecorder` records those control
  rows so service/eval can detect scheduling, adaptive-admission, idle, observe,
  and repair pressure before the next self-evolution turn. Services that want one
  pure-data boundary for that handoff can call
  `AgentCollaborationSelfEvolutionControlMonitor`; services that want the trend
  to constrain the current turn can pass that record through
  `AgentCollaborationSelfEvolutionControlGate`. `AgentCollaborationSelfEvolutionControlHandoff`
  composes those two steps into the service/eval boundary used before command
  planning. `AgentCollaborationSelfEvolutionServiceHandoff` composes final close
  recording with that control handoff after service receipts have been closed.
  `AgentCollaborationSelfEvolutionCloseAndAdmit` closes receipts and produces
  that command admission from one pure-data boundary.
  Business-loop plans, self-evolution plans, close records, control plans,
  control monitor records, control gate decisions, service handoff records, and
  close-and-admit records expose `allows_service_advance` beside
  `requires_repair_first`, so adapters can read the same repair-first decision
  at either the detailed plan layer or the outer service/eval wrapper.
  `AgentCollaborationSelfEvolutionServiceEvalReflectionCloseContinuationPlanner`
  composes close-and-admit monitoring, service/eval packet recording,
  reflection-gate recording, and reflection continuation into one persistable
  service/eval row without executing commands or promoting memory notes. The
  packet and reflection health records expose `allows_service_advance` and
  `requires_repair_first`, so service/eval can preserve stable/watch observation
  while blocking repair trends before memory or adapter side effects open.
  `AgentCollaborationSelfEvolutionServiceEvalReflectionCloseContinuationControlAdmissionHandoffMonitor`
  composes final side-effect admission packets with handoff trend health into
  one persistable service/eval boundary, still without executing service
  commands or writing memory. Its continuation planner carries that final
  handoff health into the next service/eval mode so `norion-core`,
  `norion-memory`, service adapters, and eval dashboards can agree on whether
  the following boundary should continue, observe, or repair first. Its
  continuation record keeps the persisted monitor row and that next-mode
  decision together for service/eval storage, and continuation histories
  aggregate those rows into stable/watch/repair trend health before the next
  service/eval boundary opens. Close-continuation, control, control-admission,
  control-admission-handoff, handoff-continuation,
  handoff-continuation-handoff, and handoff-continuation-handoff-continuation
  plus handoff-continuation-handoff-continuation-handoff health records expose
  the same service-advance helpers so downstream gates can consume the trend
  without unpacking dashboards. Their outer record, monitor, gate-decision, and
  handoff wrappers also expose `allows_service_advance` and
  `requires_repair_first` directly, keeping adapter admission available without
  expanding nested history packets. The continuation monitor composes that append
  and health
  computation into one replayable pure-data record. The continuation gate can
  then feed that trend health back into the next boundary, preserving stable
  continuation, observing watch pressure, and forcing repair-first on repair
  pressure without executing adapter side effects. The continuation handoff
  composes that append-and-gate path into one service/eval packet, and its
  histories aggregate those gate-applied packets into dashboard health. The
  continuation handoff monitor packages that append-and-health step into one
  replayable service/eval record.
- `schedule`: `RecursiveAgentScheduler` and `RecursiveAgentSchedule` for stable
  dependency waves under a max-parallel budget.
  `RecursiveAgentScheduler::plan_repair_first` composes a repair queue with the
  requested business queue by adding the repair task ids as dependencies of
  normal tasks, preserving repair-first ordering before memory-note,
  side-effect-admission, or next-task-promotion work can enter a later wave.
  Repair queues returned by `ConflictReportHistoryGate` should be passed
  through this path directly; unresolved conflict repair waves must complete
  before ordinary business waves resume.
  `RecursiveAgentScheduleSummary` and `RecursiveAgentScheduleGateDecision`
  expose wave pressure, parallelism, blocked-task counts, and repair-first
  admission for service/eval without expanding every wave.
  `RecursiveAgentScheduleSummaryHistory`, `RecursiveAgentScheduleDashboard`,
  and `RecursiveAgentScheduleHealth` aggregate blocked cycles, empty-wave
  pressure, schedule rate, and parallelism across turns so recursive scheduling
  drift can repair before dispatch or side-effect boundaries open. Schedule
  health and history records expose `allows_service_advance` and
  `requires_repair_first` as the service/eval admission signal for the next
  dispatch boundary. Use `RecursiveAgentScheduleHistoryGate` when a fresh
  schedule must be admitted against persisted schedule health: stable/watch
  history preserves dispatch for clean waves, while repair health emits
  deterministic `recursive-agent-schedule-repair` tasks and blocks the next
  wave before service, memory, toolsmith, or adaptive-evolution side effects
  can open. `RecursiveAgentScheduleHistoryGateRecord` is the append-and-gate
  packet for service/eval rows that need schedule health, dispatch admission,
  repair tasks, and telemetry bound to the same wave plan.
- `message`: structured `AgentMessage` values with evidence and conflict markers.
- `adapter`: `AgentAdapterBoundaryGate`, `AgentAdapterBoundarySnapshot`, and
  adapter boundary history/health rows for the final handoff between
  `norion-core`, `norion-memory`, service adapters, and eval/reporting. The
  snapshot turns owner-specific stable/watch/repair states into one compact
  admission read model: watch rows can still be observed, repair rows force
  repair-first, and memory/adaptive promotion only opens when every owner is
  stable and its gate allows the effect. `AgentAdapterBoundaryGate` can be
  projected directly from `TaskDispatchGateDecision`,
  `MemorySubmissionGateDecision`, `AgentReportGateDecision`, and
  `AgentCollaborationAdapterSideEffectAdmission`, so adapters do not need to
  duplicate dispatch, memory, eval, or service side-effect translation logic.
  `AgentAdapterBoundarySnapshot::from_boundary_gates` composes those four
  owner outputs plus the next queue into the final handoff row, preserving
  deterministic owner ordering and keeping watch-level observation separate
  from repair-first side-effect closure. Snapshot, summary, dashboard, history,
  and record telemetry preserve service command-reason, memory-promotion
  command-reason, and tool-build command-reason counts from the service-adapter
  owner gate. The same `service_execution_tool_build_command_reason_count`
  is carried through adapter handoff summaries, trend-gate summaries,
  trend-admission monitor records, resume records, resume-gate rows, and the
  resume-gate monitor-handoff row so eval can explain builder repair pressure
  without replaying service-execution payloads. `AgentAdapterBoundaryRecord` wraps the
  snapshot with its appended summary-history health so service/eval can persist
  and consume the final adapter boundary atomically; history-level repair keeps
  memory/adaptive promotion closed even when the current owner gates are clean.
  `AgentAdapterBoundaryHandoff` adds the scheduler-facing repair queue view:
  repair health turns owner blockers and adapter-boundary trend reasons into
  deterministic `adapter-boundary-repair` tasks and merges them with the
  caller's next queue, while stable/watch health remains observable without
  creating repair-first work. Its summary is the payload-free eval row for
  snapshot status, trend health, admission, repair task ids, next queue ids, and
  blocker pressure. Handoff summary histories, dashboards, health policies, and
  recorders make those rows trendable without preserving task payloads, so
  service/eval can repair repeated adapter-boundary repair-first pressure
  before another memory, adaptive-state, service-command, or eval side-effect
  boundary opens. `AgentAdapterBoundaryHandoffTrendGate` applies that persisted
  handoff health back to the next adapter-boundary handoff: stable trend health
  preserves admission and promotion booleans, watch trend health keeps the queue
  observable but closes memory/adaptive promotion, and repair trend health
  appends deterministic `adapter-boundary-handoff-trend-repair` work before any
  side-effect owner advances again. Trend-gate summaries and histories make
  that final applied decision payload-free and trendable as well, allowing
  eval/service to detect repeated effective-admission failures or trend-repair
  pressure before another adapter boundary consumes the queue.
  `AgentAdapterBoundaryHandoffTrendAdmissionGate` wraps that final step for
  service/eval: it applies the handoff trend gate, appends the compact
  trend-gate row, recomputes trend-gate health, and applies that health to the
  final queue. Stable history preserves the queue and promotion booleans, watch
  history keeps service observation open while closing memory/adaptive
  promotion, and repair history appends deterministic
  `adapter-boundary-handoff-trend-gate-repair` tasks before the business queue
  advances. Use `AgentAdapterBoundaryHandoffTrendAdmission::summary` as the
  final payload-free row for trend health, requested/effective admission,
  memory/adaptive promotion, decision repair ids, history repair ids, queue
  ids, record count, and blocker pressure.
  `AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory` lets service/eval
  aggregate those final rows across adapter turns. Its dashboard separates
  requested admission, effective admission, memory/adaptive promotion,
  decision-level repair work, trend-history repair work, queue pressure, and
  blockers; its health gate keeps empty or low-effective histories observable
  while forcing repair before repeated final-admission repair pressure can open
  another memory, adaptive-state, service-command, or eval side-effect
  boundary. Use `AgentAdapterBoundaryHandoffTrendAdmissionMonitor` when the
  adapter wants one replayable packet that applies final admission, records the
  admission summary, recomputes final-admission trend health, and returns the
  gated queue without executing the queued work or opening side effects.
  `AgentAdapterBoundaryHandoffTrendAdmissionContinuation` is the persisted
  next-boundary packet: it carries the gated queue, handoff-trend history,
  final-admission history, both policies, final health statuses, promotion
  booleans, and repair-first state so the following adapter turn can resume
  without rebuilding trend evidence from full task payloads.
  `AgentAdapterBoundaryHandoffTrendAdmissionResumePlan` turns that persisted
  continuation back into the next monitor input: it keeps the prior queue,
  trend/admission histories, policies, and prior promotion evidence together,
  then accepts a fresh handoff plus handoff-history record for the next adapter
  boundary. This lets service/eval replay final adapter admission across turns
  without manually unpacking histories or bypassing repair-first trend gates.
  `AgentAdapterBoundaryHandoffTrendAdmissionResumeRunner` is the one-call
  recovery wrapper for that path: it builds the resume plan, monitors the fresh
  handoff with persisted histories, emits the next monitor record, and packages
  the next continuation for the following turn without executing queued work.
  `AgentAdapterBoundaryHandoffTrendAdmissionResumeSummary` and its history let
  eval/service trend that recovery loop itself: effective resume admission,
  repair-first pressure, promotion booleans, queue pressure, and prior-to-next
  history growth can be stored as flat rows before another adapter boundary is
  resumed. Apply `AgentAdapterBoundaryHandoffTrendAdmissionResumeGate` after
  recording those rows when resume-history health must constrain the next
  queue: stable health preserves admission and promotion, watch health keeps
  observation open while closing memory/adaptive promotion, and repair health
  appends deterministic `adapter-boundary-handoff-trend-admission-resume-repair`
  tasks before the next side-effect owner advances.
  `AgentAdapterBoundaryHandoffTrendAdmissionResumeGateSummary` plus its
  history/dashboard/health/recorder form the final payload-free row before
  service/eval hands the queue to side-effect owners: they preserve requested
  versus effective admission, repair task ids, next queue ids, blocked-reason
  pressure, and memory/adaptive promotion booleans without replaying task
  payloads. Use
  `AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitor` when the adapter
  wants the gate decision, appended final gate history row, final health, and
  next queue audit as one replayable packet.
  `AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorGate` then applies
  that final gate-history health back to the queue: stable history preserves the
  final admission, watch history remains observable without memory/adaptive
  promotion, and repair history appends deterministic
  `adapter-boundary-handoff-trend-admission-resume-gate-monitor-repair` work
  before side-effect owners consume the queue.
  `AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoff` composes
  that final monitor append and final health gate into one service/eval packet
  so adapters can persist the last row and consume the gated queue without
  manually replaying either step. Persist
  `AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffSummary`
  rows with
  `AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHistoryRecorder`
  when service/eval needs the final monitor-handoff row to be trendable without
  carrying task payloads. Its history/dashboard/health layer preserves final
  gate health, requested versus effective admission, repair-first pressure,
  memory/adaptive promotion booleans, repair task ids, next queue ids, and
  blocked-reason pressure; repair-level drift remains a queue-first blocker
  before norion-memory, norion-core, service adapters, or eval consumers advance.
  Apply
  `AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffGate` after
  recording that row when the final summary-history health must be projected
  back onto the outgoing queue: stable history preserves memory/adaptive
  promotion, watch history remains observable, and repair history appends
  deterministic
  `adapter-boundary-handoff-trend-admission-resume-gate-monitor-handoff-repair`
  tasks before any side-effect owner consumes the final packet. Use
  `AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHandoff`
  when service/eval wants that final summary-history append and final
  summary-history gate as one pure-data packet with the admitted queue, health
  row, and promotion booleans already aligned.
- `aggregate`: `MessageAggregator` for deterministic duplicate merging.
  `AggregationSummary` exposes duplicate-message pressure and compression rate
  for service/eval rows without replaying all aggregated messages.
  `AggregationSummaryHistory`, `AggregationDashboard`, and `AggregationHealth`
  aggregate duplicate records and compression trends across windows so repeated
  report duplication can repair before it hides conflict or budget signals.
  `AggregationHistoryGate` applies that duplicate-pressure health to the current
  report before conflict/run consumers receive it; repair rows emit
  deterministic `aggregation-repair` tasks. `AggregationHistoryGateRecord` is the
  append-and-gate packet for service/eval rows that need aggregation health,
  forward admission, repair tasks, and telemetry tied to the same message set.
  Aggregation health and history records expose the same service-advance helpers
    as budget and side-effect trend rows, so watch-level compression drift stays
  observable while duplicate pressure blocks promotion first.
- `conflict`: `ConflictResolver`, `ConflictResolution`, and
  `ConflictResolutionBook` for positive/negative stance collisions plus
  auditable coordinator resolutions. `ConflictReportSummary` exposes
  resolved/unresolved conflict counts, conflicted message count, topics, and
  all-resolved status for dashboards and gates. `ConflictReportSummaryHistory`,
  `ConflictReportDashboard`, and `ConflictReportHealth` aggregate unresolved
  conflict pressure across turns so memory notes, adaptive writes, service
  commands, and other side effects can stay blocked until the coordinator has
  recorded covering resolutions. Conflict health and history records expose
  `allows_service_advance` and `requires_repair_first`, letting service/eval
  observe empty or resolved rows while unresolved conflict trends force repair
  before memory-note or adaptive-state promotion. Use
  `ConflictReportHistoryGate` when a fresh conflict report must be admitted
  against persisted conflict health: current unresolved conflicts or repair
  history close report forwarding and side-effect promotion, then emit
  deterministic `conflict-report-repair` tasks for the reviewer lane.
  `ConflictReportSummaryHistoryRecorder::record_report_with_health_gate` is the
  append-and-gate packet for service/eval rows that need the conflict summary,
  health record, side-effect admission booleans, repair tasks, and telemetry
  bound to the same message set.
- `reflection`: a lightweight state machine: `draft -> critique -> revision ->
  memory_note`. `ReflectionLoopSummary` and `ReflectionLoopGateDecision` expose
  compact service/eval rows for remaining stages, completion, memory-note
  readiness, and reflection-continuation admission without writing memory.
  `ReflectionLoopSummaryHistory`, `ReflectionLoopDashboard`, and
  `ReflectionLoopHealth` aggregate low-level reflection progress across turns so
  service/eval can distinguish ordinary in-progress work from missing memory
  notes or repeated stalls on the same stage before any memory promotion runs.
  `ReflectionLoopHistoryGate` applies that health to the current loop before a
  memory note is promoted, preserving stable/watch continuation and producing
  deterministic `reflection-loop-repair` tasks for repair-level history.
  `ReflectionLoopHistoryGateRecord` is returned by the recorder's
  append-and-gate path when adapters need the health row, memory-note decision,
  repair tasks, and telemetry from one replayable record.
  Reflection health and history records expose `allows_service_advance` and
    `requires_repair_first`, preserving empty or watch-level observation while
  forcing repair before repeated stalls can promote memory notes.
- `run`: `AgentRunLedger`, `RunBudgetAudit`, and `SideEffectGate` for
  deterministic result ordering, reserved-vs-spent budget checks, and
  conflict-aware side-effect admission. `AgentRunLedger::report` feeds that
  ordered result stream into aggregation, so window result arrival order cannot
  change duplicate grouping, source id ordering, or the compact report summary.
  `AgentRunLedgerProgress` records assigned, reported, accepted, rejected,
  missing, and unassigned result counts before a run can close; missing
  dispatched windows, rejected results, or unassigned result packets now close
  memory-note, file-write, adaptive-state, and external-call gates before
  aggregation/conflict/budget evidence can be promoted.
  `AgentRunLedgerProgressSummaryHistory` and `AgentRunLedgerProgressHealth`
  persist those close-readiness rows across runs, letting service/eval detect
  repeated missing-window, rejected-result, unassigned-result, and close-rate
  drift before another run report is admitted. `AgentRunProgressReportGate`
  composes that progress trend with `AgentRunReportHealthGateRecord`: progress
  repair closes the combined admission and is inserted ahead of report-health
  repair tasks and ordinary business work in the next queue.
  `AgentRunProgressReportGateSummaryHistory` and
  `AgentRunProgressReportGateHealth` persist that combined admission boundary,
  tracking requested/admitted rate, progress/report repair health mix,
  repair-task pressure, next-queue pressure, and blocker counts across runs.
  `RunBudgetAuditSummary`, `AgentRunReportSummary`, and
  `AgentRunGateDecision` compact run-level aggregation, conflict, budget, and
  side-effect evidence into service/eval rows before memory, adaptive-state, or
  external-call adapters execute.
  `AgentRunReportSummaryHistory`, `AgentRunReportDashboard`, and
  `AgentRunReportHealth` aggregate those rows across runs so service/eval can
  evaluate clean-rate, conflict pressure, budget overspend pressure, and
  memory/adaptive/external admission trends before promoting memory or adaptive
  state. `AgentRunReportHealthGate` turns that run-level trend health into a
  next-queue admission row: stable/watch trends preserve the caller's queue,
  while repair trends block ordinary progression and merge deterministic
  `agent-run-report-health` repair tasks before any memory, adaptive-state, or
  service-side command boundary is opened. The recorder's
  `record_*_with_health_gate` variants package append, health, gate, merged
  queue, and compact gate summary into one service/eval persistence boundary.
  `AgentRunReportHealthGateHistory` stores those compact summaries so eval can
  derive admission-rate, repair-first, repair-task, queue-pressure, and
  latest-blocker trends without persisting full task payloads. Run-report
  health values and the corresponding summary/history records expose
  `allows_service_advance` and `requires_repair_first`, with `records()` on
  record wrappers, so service/eval can treat stable/watch trends as observable
  progress and repair trends as repair-first admission blockers without
  opening memory, adaptive-state, service-command, or external-call side
  effects.
  `AgentRunReportHealthGateTrendGate` feeds that compact trend health back into
  the next scheduler queue, preserving stable/watch admission and appending
  deterministic `agent-run-report-health-gate` repair work for repair-level
  gate trends. `AgentRunReportHealthGateTrendHandoff` composes the compact
  summary append, trend health, trend gate, merged queue, and handoff summary
  into one replayable service/eval packet. Its handoff summary history,
  dashboard, health, and recorder types let eval keep a payload-free trend of
  admitted, repair-first, trend-health, queue-pressure, repair-task, and
  blocked-reason evidence. `AgentRunReportHealthGateTrendHandoffGate` applies
  that compact handoff-history health back to the current handoff queue before
  the next scheduler/service/eval boundary opens: stable and watch preserve the
  requested admission, while repair blocks admission and appends deterministic
  `agent-run-report-health-gate-handoff` repair tasks.
  `AgentRunReportHealthGateTrendHandoffMonitor` packages the handoff-summary
  append, handoff-history health, queue gate, and compact monitor summary into
  one replayable row for adapters that do not want to call recorder and gate
  separately. Its monitor summary history/dashboard/health/recorder types turn
  those gate-applied rows into payload-free trend evidence for requested
  admission, effective admission, repair-first pressure, handoff-health mix,
  queue pressure, repair work, and blocker pressure.
  `AgentRunReportHealthGateTrendHandoffMonitorGate` feeds that monitor trend
  health back into the next scheduler queue: stable and watch histories
  preserve the current monitor admission, while repair histories block
  admission and append deterministic
  `agent-run-report-health-gate-handoff-monitor` repair work.
  `AgentRunReportHealthGateTrendHandoffMonitorHandoff` composes the monitor
  summary append and monitor gate into one service/eval packet so adapters can
  persist the trend row and consume the gated next queue without replaying full
  task payloads. Its monitor-handoff summary history/dashboard/health/recorder
  types aggregate those packets into payload-free evidence for requested
  admission, effective admission, repair-first pressure, monitor-health mix,
  queue pressure, repair work, and blocker pressure.
  `AgentRunReportHealthGateTrendHandoffMonitorHandoffGate` applies that compact
  final-packet trend before another scheduler/service/eval boundary opens:
  stable and watch rows preserve the current queue, while repair rows block
  admission and append deterministic
  `agent-run-report-health-gate-handoff-monitor-handoff` work.
  `AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoff` composes the
  final monitor-handoff summary append and gate decision into one replayable
  run-level packet for adapters that persist and gate the boundary together;
  its summary projects the final health, requested/effective admission,
  repair-first state, queue ids, repair task ids, and blocker pressure into a
  compact eval row. Persist those rows in
  `AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistory`
  and record them with
  `AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistoryRecorder`
  when service/eval needs dashboard and health evidence over the final
  run-report handoff packet before admitting another scheduler boundary.
  `AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffGate` applies that
  final-packet history health back to the next queue: stable/watch keeps the
  packet admitted, while repair appends deterministic
  `agent-run-report-health-gate-handoff-monitor-handoff-handoff` repair work
  and blocks ordinary progression.
  `AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoff` composes
  that final-packet summary append and final-packet history gate into one
  replayable admission packet, preserving the appended history row, gate
  decision, next queue, repair tasks, and telemetry for service/eval adapters
  that need to persist and admit the boundary atomically. Its `summary()` gives
  eval a payload-free final admission row with packet health, requested and
  effective admission, repair-first state, queue ids, repair ids, and blocker
  pressure. Persist those rows in
  `AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistory`
  and append them with
  `AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistoryRecorder`
  when service/eval needs dashboard and health evidence over final admission
  trends before another business-loop scheduler boundary opens.
  `AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffGate`
  applies that final admission trend back to the next queue: stable/watch
  keeps the final queue admitted, while repair appends deterministic
  `agent-run-report-health-gate-handoff-monitor-handoff-handoff-handoff`
  work and blocks ordinary business-loop progression.
  `AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffHandoff`
  composes the final admission summary append and final admission trend gate
  into one replayable packet, so service/eval adapters can persist the final
  admission trend row and consume the gated queue atomically before handing
  control back to the business-loop scheduler. Its `summary()` is the final
  payload-free eval row for admission-trend health, requested/effective
  admission, repair-first state, queue ids, repair ids, and blocker pressure.
  Adapter code can project this final run-report packet directly into the
  service-adapter owner boundary with
  `AgentAdapterBoundaryGate::from_run_report_final_handoff` or, when it only
  has the final gate row,
  `AgentAdapterBoundaryGate::from_run_report_final_gate_decision`. Stable
  admitted records keep service command, memory-note, and adaptive-state
  admission open; watch/non-repair records keep service observation open while
  closing memory/adaptive promotion; repair records close the boundary and
  preserve run-report blocker reasons for repair-first scheduling. Use the
  matching `AgentAdapterBoundarySnapshot::from_run_report_final_*` helpers when
  eval/service needs the gated queue ids stored beside that adapter boundary
  row without re-running any side effect.
  `AgentAdapterBoundarySummaryHistoryRecorder::record_run_report_final_gate_decision_with_health`
  and `record_run_report_final_handoff_with_health` append that projected row
  into adapter-boundary health history; the corresponding
  `record_run_report_final_*_handoff_with_health` helpers also produce the next
  adapter handoff, preserving the run-report gated queue after any
  adapter-boundary repair tasks.
- `evolution`: `ToolsmithPlan`, `ClosedLoopRewarder`, `ProcessRewardReport`,
  and `EvolutionSignal` for Rust-only tool proposals and reinforce/hold/penalize
  decisions. `ToolsmithPlanSummaryHistory` and `ToolsmithPlanHealth` expose
  Rust-gate failures, rejected requests, ready proposal rates, and non-Rust
  tool pressure across turns. `ToolsmithPlanHistoryGate` applies that persisted
  health back onto the current plan before promotion: stable/watch health keeps
  Rust-ready proposals promotable, while repair health emits deterministic
  `toolsmith-plan-repair` tasks and blocks non-Rust or rejected tool drift.
  `ToolsmithPlanHistoryGateRecord` is the append-and-gate packet for service/eval
  rows that need toolsmith health, ready-proposal admission, repair tasks, and
  telemetry bound to the same Rust-only plan. `ToolBuildRequest::admitted_by_evolution`
  materializes only final-admitted ready Rust proposals for a service-owned
  `ToolBuildPort`, keeping held, rejected, non-Rust, toolsmith-repair, reward-repair,
  or admission-history-blocked proposals out of the tool side-effect boundary.
  `ToolBuildReport::from_requests_and_receipts` closes the service-owned build
  receipts back into a deterministic read model, surfacing missing, unexpected,
  duplicate, held, or rejected tool builds as repair-first evidence before the
  next self-evolution turn advances. When a cycle passes that report into
  `ProcessRewardInput`, dirty build receipts lower the toolsmith reward
  component and add explicit tool-build repair notes before reward health or
  evolution admission can reinforce the process. `AgentCycleEvidence` carries
  the same optional `tool_build_report` so `AgentCycleOrchestrator` can close a
  wave, score process reward, and preserve tool-build receipt health in one
  data-only cycle report. The cycle report stores the payload-free
  `ToolBuildReportSummary`, and `AgentCycleSummary` mirrors the receipt counts
  that matter for admission: report presence, missing requests, unexpected
  receipts, duplicate receipts, held receipts, and rejected receipts.
  `ToolBuildReportSummary`, `ToolBuildReportSummaryHistory`, and
  `ToolBuildReportHealth` turn those receipt-close rows into payload-free
  service/eval evidence. Dashboards aggregate requested, received, built, held,
  rejected, missing, unexpected, duplicate, diagnostic, clean, and repair
  counts without storing artifacts or tool payloads. Empty history is watch
  evidence, clean receipt history is stable, and any pressure beyond
  `ToolBuildReportHealthPolicy` requires repair before the next tool-building,
  memory-note, adaptive-state, or eval promotion boundary advances.
  `ToolBuildReportHistoryGate`, or
  `ToolBuildReportSummaryHistoryRecorder::record_report_with_health_gate`,
  applies that persisted receipt health back onto the current report. Stable
  clean reports open the next tool-build boundary and allow memory-note,
  adaptive-state, and eval promotion; current or historical repair pressure
  emits deterministic `tool-build-report-repair` tasks and keeps those
  side-effect promotions closed. Adapter owners can project the same record
  through `AgentAdapterBoundaryGate::from_tool_build_report_history_gate` or
  `AgentAdapterBoundarySnapshot::from_tool_build_report_history_gate`, making
  tool-build receipt health visible in the same service-adapter boundary rows
  used by runtime service, memory, adaptive-state, and eval handoffs.
  Use
  `AgentAdapterBoundarySummaryHistoryRecorder::record_tool_build_report_history_gate_with_health`
  when eval/service wants that adapter row appended to boundary health in one
  call, or `record_tool_build_report_history_gate_handoff_with_health` when the
  next scheduler handoff should carry adapter-boundary repair tasks ahead of the
  business queue. Dirty tool-build handoffs must keep memory-note,
  adaptive-state, and eval-finalize work behind the generated
  `adapter-boundary-repair` tasks by adding those repair ids as dependencies of
  the ordinary next queue; adapters should consume the returned handoff queue
  instead of reusing the caller's queue directly.
  `AgentAdapterBoundaryHandoffHistoryRecorder::record_tool_build_report_history_gate_with_health`
  records the resulting handoff health row as well, keeping receipt-close,
  adapter-boundary, and handoff-health evidence aligned.
  `ProcessRewardReportSummaryHistory` and
  `ProcessRewardReportHealth` expose reinforce/hold/penalize mix, average
  reward, missing evolution signals, and low component pressure before the
  self-evolution business loop promotes adaptive state or schedules repair.
  `ProcessRewardReportHistoryGate` applies that reward trend back onto the
  current report, allowing clean evolution signals to continue while repair
  health, current penalties, low scores, or missing signals emit deterministic
  `process-reward-report-repair` tasks before adaptive promotion.
  `ProcessRewardReportHistoryGateRecord` is the append-and-gate packet for
  service/eval rows that need reward history, signal-promotion admission, repair
  tasks, and telemetry to stay tied to the same scored report.
  `ReflectionRewardAdmissionGate` composes the reflection append-and-gate record
  with the process-reward append-and-gate record before memory-note or
  self-evolution promotion. A reward `Reinforce` can only promote evolution
  signals or reinforce the process when the reflection memory-note gate is also
  promotable; incomplete reflection stays observable without signal promotion,
  and repair tasks are merged in deterministic reflection-before-reward order.
  `EvolutionAdmissionGate` composes the toolsmith and process-reward
  append-and-gate packets into a single pure-data admission record. It exposes
  ready-tool promotion, evolution-signal promotion, process reinforcement,
  adaptive-state promotion, repair-first status, merged repair tasks, blocked
  reasons, and telemetry without performing any tool build, memory write,
  adaptive-state write, or service command.
  `EvolutionAdmissionSummaryHistory`, `EvolutionAdmissionDashboard`, and
  `EvolutionAdmissionHealth` aggregate combined admission outcomes across turns
  so service/eval can watch admitted records, repair-first pressure,
  adaptive-state admission, repair tasks, blocked reasons, and upstream
  toolsmith/reward repair pressure before the self-evolution loop promotes
  state. `EvolutionAdmissionHistoryGate` applies that persisted trend back onto
  the current combined admission: stable history keeps clean promotion booleans
  open, watch history remains observable without adaptive-state promotion, and
  repair history emits deterministic `evolution-admission-repair` tasks before
  tool-building, reinforcement, or adaptive-state promotion can continue.
  `EvolutionAdmissionHandoff` packages the history-gated admission with the
  pending scheduler queue: clean admissions preserve the business queue, while
  repair-first trends merge deterministic admission repair tasks into the next
  queue. Its summary gives eval effective admission, promotion booleans, repair
  ids, next-queue ids, blocked reasons, and record counts without expanding the
  full admission payload. `EvolutionAdmissionHandoffSummaryHistory`,
  `EvolutionAdmissionHandoffDashboard`, and `EvolutionAdmissionHandoffHealth`
  aggregate effective admission, repair-first pressure, repair-task pressure,
  blocked reasons, upstream admission repair status, and next-queue pressure
  before the scheduler consumes another self-evolution queue.
  `EvolutionAdmissionHandoffTrendGate` reapplies that persisted handoff health
  to the current queue: stable histories preserve promotion and queue state,
  watch histories remain observable while closing promotion/adaptive-state
  flags, and repair histories merge deterministic
  `evolution-admission-handoff-trend-repair` tasks before service/core/memory
  owners advance. `EvolutionAdmissionHandoffTrendMonitor` combines the handoff
  history append and final trend gate into one replayable packet for service/eval
  wiring, returning the history record, gate decision, compact telemetry, and
  gated queue without executing scheduler or memory side effects.
  `EvolutionAdmissionHandoffTrendContinuation` and its planner package that
  monitor result as the durable scheduler-facing self-evolution input: gated
  queue, handoff history, policy, health status, effective admission, promotion
  booleans, repair-first state, and telemetry.
  `EvolutionAdmissionHandoffTrendContinuationSummary` is the payload-light
  service/eval row for that continuation, keeping health status, promotion
  booleans, repair-first state, queue ids, queue size, and handoff-history depth
  without embedding task payloads.
  `EvolutionAdmissionHandoffTrendContinuationSummaryHistory`,
  `EvolutionAdmissionHandoffTrendContinuationDashboard`, and
  `EvolutionAdmissionHandoffTrendContinuationHealth` aggregate those final
  scheduler-facing rows across turns, repairing repair-first continuation
  drift, watching queue/history pressure, and preserving effective-admission and
  adaptive-state rates before norion-core consumes another self-evolution queue.
  `EvolutionAdmissionHandoffTrendContinuationHistoryGate` reapplies that
  persisted continuation health to the current continuation before scheduler
  consumption: stable rows preserve promotion, watch rows remain observable
  while closing promotion/adaptive-state flags, and repair rows merge
  deterministic `evolution-admission-handoff-trend-continuation-repair` tasks
  into the queue before service/core/memory owners advance.
  `EvolutionAdmissionHandoffTrendContinuationHistoryGateSummary` is the
  payload-light service/eval row after that final gate, preserving final health,
  admission/promotion booleans, repair ids, next queue ids, blocker count, and
  continuation-history depth without embedding task payloads.
  Toolsmith, process-reward, and
  evolution-admission health/history records expose
  `allows_service_advance` and
  `requires_repair_first` so repair-level self-evolution drift schedules tool,
  reward, or admission repair first.
- `cycle`: `AgentCycleOrchestrator`, `AgentCycleSummary`,
  `AgentCycleHandoff`, and `MemoryPromotion` for planning the next ready
  dispatch wave, closing an executed wave into run/reward reports, preserving
  execution failures, proposing gated memory notes, exposing eval-friendly
  ledger counters, and packaging the service handoff.
  `AgentCycleSummaryHistory`, `AgentCycleSummaryDashboard`, and
  `AgentCycleSummaryHealth` aggregate rejected tasks, unresolved conflicts,
  blocked side effects, budget overspends, execution failures, reward action
  mix, average reward, clean-cycle rate, memory-promotion rate, and
  tool-build receipt pressure before the report gate, loopback, ledger, or
  business-loop controller promotes adaptive state. Dirty tool-build receipts
  keep `ready_for_memory_promotion` false, appear in `AgentCycleHandoff`
  blocked reasons, and make default cycle health repair-first without requiring
  service/eval to parse reward notes or retain build payloads. Cycle health and
  history records expose `allows_service_advance` and `requires_repair_first`
  before report-gate, memory, or adaptive-state consumers advance from a dirty
  cycle trend.
- `execute`: `AgentWaveExecutor` for driving accepted dispatch assignments
  through an `EnginePort` adapter while collecting structured execution
  failures. `AgentWaveExecutionSummary`, `AgentWaveExecutionSummaryHistory`,
  and `AgentWaveExecutionHealth` expose result acceptance, adapter failures,
  incomplete waves, and empty-execution pressure across turns so runtime/model
  drift can be observed or repaired before the cycle is closed into memory,
  eval, or service-command side effects. Execution health and history records
  expose `allows_service_advance` and `requires_repair_first` before cycle
  close, memory handoff, or service-command planning consumes the wave result.
- `memory`: `MemoryHandoffSubmitter` for submitting clean
  `AgentCycleHandoff` memory notes through `MemoryPort` and preserving port
  failures as data. `MemorySubmissionSummary` and
  `MemorySubmissionGateDecision` flatten the result into submitted/failed/
  blocked counters, port-attempt evidence, repair-first reasons, and loop
  continuation admission for service/eval rows. `MemorySubmissionSummaryHistory`,
  `MemorySubmissionDashboard`, and `MemorySubmissionHealth` aggregate submitted
  notes, failed notes, blocked memory handoffs, clean rate, and port-attempt
  pressure across turns so repeated memory drift can repair before another
  self-evolution promotion is admitted. Memory health and history records expose
  `allows_service_advance` and `requires_repair_first` before another memory or
  adaptive-state promotion boundary advances. `MemoryPromotionGate` must treat
  repair-level memory-submission history as stronger than a clean current
  candidate: it keeps memory submission closed and emits deterministic
  `memory-promotion-repair` tasks until the submission trend is stable again.
- `eval`: `AgentCycleLedgerRecord`, `AgentReportEvidence`, and
  `AgentReportGate` for turning cycle summaries plus memory submission results
  into deterministic accept/block decisions and repair tasks.
  `AgentReportGateSummary` flattens each decision into accepted status,
  blocker-code order, repair lanes/roles, and blocker-family booleans for
  service/eval dashboards, including a dedicated tool-build blocker family
  separate from generic review pressure. `AgentReportGateSummaryHistory`,
  `AgentReportGateDashboard`, `AgentReportGateHealth`, and
  `AgentReportGateHistoryRecorder` aggregate those rows across cycles so eval
  can track acceptance rate, blocker-family pressure, follow-up repair work,
  and stable/watch/repair health before the self-evolution loop promotes or
  repairs the next cycle. Tool-build receipt pressure from `AgentCycleSummary`
  is evaluated here as explicit `tool_build_*` blocker codes and folds into one
  deterministic `eval-tool-build` repair task, so eval cannot accept a cycle
  whose build receipts are missing, unexpected, duplicate, held, or rejected.
  Report-gate dashboards expose `tool_build_blockers` directly, making receipt
  drift visible without parsing blocker-code strings.
  `AgentReportGateHealthGate` applies that trend health
  to the next scheduler queue: stable/watch health preserves the caller's
  queued work, while repair health blocks ordinary admission and appends one
  deterministic `eval-report-gate-health` repair task per health reason before
  loopback or business-loop promotion continues. `AgentReportGateHealthGateRecord`
  and `AgentReportGateHealthGateSummary` let service/eval adapters append the
  report-gate row, recompute trend health, gate the next queue, and persist a
  payload-free admission summary from one pure-data boundary. Eval report-gate
  health and history records expose `allows_service_advance` and
  `requires_repair_first` across the summary, health-gate, trend-handoff,
  monitor, monitor-handoff, final-packet, and final-admission histories, so
  service/eval can observe watch rows while repair trends force repair-first
  queue work before loopback or service handoff.
  `AgentReportGateHealthGateSummaryHistory`,
  `AgentReportGateHealthGateDashboard`,
  `AgentReportGateHealthGateHealth`, and
  `AgentReportGateHealthGateHistoryRecorder` aggregate those compact admission
  summaries across cycles so eval can track gate-admission rate, repair-first
  pressure, queued repair work, and blocker pressure without persisting full
  task payloads. `AgentReportGateHealthGateTrendGate` feeds that compact trend
  health back into the next scheduler queue: stable/watch trend health preserves
  the caller's queued work, while repair trend health appends deterministic
  `eval-report-gate-health-gate-trend` repair tasks before loopback can advance.
  `AgentReportGateHealthGateTrendHandoff` composes compact-summary append,
  trend-health recomputation, trend gate application, merged queue, and
  `AgentReportGateHealthGateTrendHandoffSummary` into one replayable
  service/eval packet.
- `loopback`: `AgentLoopbackPlanner` for combining service handoffs and report
  gate decisions into the next task queue plus an adaptive-state promotion bit.
  `AgentLoopbackPlanSummary` exposes promotion admission, queued task pressure,
  blocker pressure, task ids, repair lanes, and next-wave schedulability without
  expanding the queue payload. `AgentLoopbackPlanSummaryHistory`,
  `AgentLoopbackPlanDashboard`, and `AgentLoopbackPlanHealth` aggregate
  promotion rate, repair-first pressure, blocked loopbacks, queued work,
  repair-lane pressure, and schedulability across cycles so service/eval can
  repair a dirty handoff-to-business-loop bridge before adaptive state is
  promoted. Loopback health and history records expose `allows_service_advance`
  and `requires_repair_first` before the business-loop controller consumes the
  next queue.
- `ledger`: `AgentCycleLedger` for collecting multiple report-gated cycles,
  summarizing acceptance rate, consecutive blockers, queued repairs, and
  promote/hold/repair admission for the self-evolution loop.
  `AgentCycleLedgerSummaryHistory`, `AgentCycleLedgerDashboard`, and
  `AgentCycleLedgerHealth` aggregate ledger snapshots into stable/watch/repair
  evidence for acceptance-rate drift, reward drift, latest-blocked pressure,
  consecutive blockers, queued repair volume, and adaptive-promotion rate before
  `AgentBusinessLoopController` emits a service-facing control packet. Ledger
  health and history records expose `allows_service_advance` and
  `requires_repair_first` so service/eval can observe watch-level ledger drift
  while forcing repair before business-loop promotion.
- `control`: `AgentBusinessLoopController` for packaging ledger admission, the
  next task queue, telemetry, and an optional `AdaptiveStateCandidate` into one
  service-facing control plan. `AgentBusinessLoopPlanSummary` compacts that
  boundary into status, trend counters, queue size, candidate presence,
  adaptive promotion admission, repair-first status, and evidence-ref counts.
  `AgentBusinessLoopPlanSummaryHistory`, `AgentBusinessLoopPlanDashboard`, and
  `AgentBusinessLoopPlanHealth` aggregate final in-crate control decisions into
  promotion, hold, repair, candidate, repair-first, queue-pressure, reason, and
  evidence-ref trends before service command planning opens side effects.
  Business-loop plan health and history records expose `allows_service_advance`
  and `requires_repair_first` before service command planning or adaptive-state
  writes advance.
- `service`: `AgentServiceCommandPlanner`, `AgentServiceExecutionReport`, and
  `AgentServiceExecutionHistory` close the service-owned command receipt
  boundary as data. `AgentServiceExecutionHealthGate` turns service-local
  receipt-close health into an admission row: stable/watch trends preserve the
  next queue, while repair trends block ordinary command admission and prepend
  deterministic `service-execution-health` repair work for the scheduler.
  `AgentServiceExecutionHealthGateSummary` is the compact service/eval row for
  the same decision, and
  `AgentServiceExecutionHistoryRecorder::record_*_with_health_gate` packages
  append, health, gate, merged queue, and summary into one receipt-close
  boundary. `AgentServiceExecutionHealthGateHistory` aggregates those compact
  rows into admission-rate, repair-first, queue-pressure, and latest-blocker
  dashboard evidence, while `AgentServiceExecutionHealthGateHealth` turns that
  evidence into stable/watch/repair trend feedback for the next service/eval
  boundary. Service command plan, audit, feedback, turnover, execution, gate,
  handoff, monitor, and monitor-handoff health records expose `records()`,
  `allows_service_advance`, and `requires_repair_first`, so service/eval can
  observe stable/watch receipt trends while routing repair-level drift through
  scheduler repair before another command, memory, adaptive-state, or external
  side-effect boundary opens. `AgentServiceExecutionHealthGateTrendGate` applies
  that trend
  feedback to the next queue, preserving stable/watch admission and forcing
  repair-first when the gate trend is repair-level.
  `AgentServiceExecutionHealthGateTrendHandoff` packages the append, trend
  health, gate decision, merged queue, and compact summary into one service/eval
  record. Its summary history/dashboard types aggregate those rows into
  admitted-rate, repair-first, trend-health, queue-pressure, and latest-blocker
  evidence without persisting full task payloads. Its handoff health policy and
  recorder turn that compact dashboard back into stable/watch/repair trend
  evidence for the next service/eval boundary, and
  `AgentServiceExecutionHealthGateTrendHandoffMonitor` applies that trend back
  to the current handoff queue with deterministic
  `service-execution-health-gate-handoff` repair tasks when the compact handoff
  history is repair-level. Monitor summaries and dashboards give eval a
  payload-free row for requested admission, effective admission,
  repair-first pressure, handoff-health mix, queued work, and blockers. Monitor
  summary history health/recorder types turn those rows into stable/watch/repair
  trend feedback before the next service/eval handoff boundary is opened.
  `AgentServiceExecutionHealthGateTrendHandoffMonitorGate` feeds that monitor
  trend health back into the next queue: stable and watch rows preserve the
  current monitor admission, while repair rows block admission and append
  deterministic `service-execution-health-gate-handoff-monitor` repair work.
  `AgentServiceExecutionHealthGateTrendHandoffMonitorHandoff` composes the
  monitor-summary append and that gate decision into one replayable service/eval
  packet for adapters that do not want to call recorder and gate separately.
  Its summary/history/dashboard/health types persist the gate-applied handoff
  row as compact trend evidence for admission rate, repair-first pressure,
  monitor-health mix, queue pressure, and blocker pressure without replaying
  full queue payloads.
  `AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffGate` can then
  constrain the next queue from that compact handoff trend: stable and watch
  preserve the current queue, while repair appends deterministic
  `service-execution-health-gate-handoff-monitor-handoff` work before another
  service/eval boundary is admitted.
  `AgentServiceExecutionHealthGateTrendHandoffMonitorHandoffHandoff` composes
  that compact-row append and gate decision when adapters want one replayable
  persistence packet for the final monitor-handoff trend boundary. Adapter code
  can project that packet into the service-adapter owner boundary with
  `AgentAdapterBoundaryGate::from_service_execution_final_handoff` or
  `AgentAdapterBoundaryGate::from_service_execution_final_gate_decision`, and
  can store the matching queue ids with
  `AgentAdapterBoundarySnapshot::from_service_execution_final_*`. Stable
  admitted service-execution final packets keep service commands, memory-note
  promotion, and adaptive-state promotion open; watch/non-repair packets keep
  service observation open while closing memory/adaptive promotion; repair
  packets close the adapter boundary and preserve service-execution blockers
  for repair-first scheduling. Use
  `AgentAdapterBoundarySummaryHistoryRecorder::record_service_execution_final_*_with_health`
  to append that projected row into adapter-boundary health, or the matching
  `record_service_execution_final_*_handoff_with_health` helpers when the next
  scheduler handoff should carry adapter-boundary repair work before the
  service-execution gated queue.
- `step`: `AgentClosedLoopStepper` for closing one service cycle from
  `AgentCycleReport`, `AgentCycleHandoff`, `MemorySubmissionReport`, and the
  existing `AgentCycleLedger` into report-gate, loopback, updated-ledger, and
  business-loop control outputs. It also exposes compact execution summaries,
  multi-run execution history, and dashboard counters for eval/service views.
- `turn`: `AgentClosedLoopRuntimeTurnRunner` for running one guarded service
  turn from history, queue, budget, evidence, and an `EnginePort` into a
  runtime-turn envelope with optional `AgentCycleReport`, telemetry, and skipped
  reasons. `AgentClosedLoopRuntimeBusinessTurnCloser` then converts a runtime
  turn with a report into handoff, memory submission, report-gate, loopback,
  ledger, and business-loop control data.
  `AgentClosedLoopRuntimeServiceRequestRunner` composes the runtime turn,
  business closer, and service command planner into the last crate-owned
  boundary before service side effects.
  `AgentClosedLoopRuntimeServiceCommandPlanner` exposes the service command
  request that the outer service should execute.
  `AgentClosedLoopRuntimeServiceCommandGate` projects that request into
  per-command side-effect admission entries before the service writes adaptive
  state, updates queues, or emits external telemetry.
  `AgentClosedLoopRuntimeServiceDispatch` binds the command request and gate
  into a single executable-or-blocked envelope for the service executor.
  `AgentClosedLoopRuntimeServiceDispatchSummary` gives service/eval a compact
  pre-execution row with executable status, command-gate status, side-effect
  gate counts, command count, command kinds, and gate blockers.
  `AgentClosedLoopRuntimeServiceReceiptIntake` validates returned receipts
  against that dispatch before receipt audit can append history.
  `AgentClosedLoopRuntimeServiceDispatchOutcome` carries the dispatch, intake,
  and optional closed outcome together so blocked or unexpected receipts stay
  visible without being treated as successful service execution.
  `AgentClosedLoopRuntimeServiceIntakeRepairPlan` turns blocked intake reasons
  into deterministic `service-intake` repair tasks for the next coordinator
  wave.
  `AgentClosedLoopRuntimeServiceDispatchContinuationPlanner` converts either a
  closed dispatch outcome or blocked intake repair queue into the next
  `AgentClosedLoopRuntimeTurnInput`.
  `AgentClosedLoopRuntimeServiceDispatchContinuationSummary` exposes the compact
  continuation row: closed outcome, intake cleanliness, repair task count,
  health status, next queue size, and history length.
  `AgentClosedLoopRuntimeServiceDispatchContinuationSummaryHistory` turns those
  rows into a continuation dashboard and health read model so service/eval can
  detect low closure rate, dirty intake, repair pressure, and blocked
  continuations before handing the next runtime input to the wider loop.
  `AgentClosedLoopRuntimeServiceRunner` composes request dispatch, caller-owned
  receipts, intake-aware outcome closure, and continuation planning into one
  pure-data service-run envelope.
  `AgentClosedLoopRuntimeServiceRunSummary` merges the pre-execution dispatch
  row and post-execution continuation row so eval can audit executable status,
  command-gate status, side-effect gate counts, gate blockers, intake blockers,
  repair tasks, health, and next queue without expanding nested reports.
  `AgentClosedLoopRuntimeServiceRunStatus` classifies that row as `Closed`,
  `DispatchBlocked`, or `IntakeBlocked` so the service can distinguish a clean
  side-effect turn from a blocked gate or receipt drift.
  `AgentClosedLoopRuntimeServiceRunHistory` and
  `AgentClosedLoopRuntimeServiceRunDashboard` collect those run summaries across
  attempts, including blocked dispatches that must not append execution
  history.
  `AgentClosedLoopRuntimeServiceRunHistoryRecorder` appends a completed
  service-run summary into that attempt history and returns the updated
  dashboard, health, and telemetry in one pure-data record.
  `AgentClosedLoopRuntimeServiceRunHealth` applies a policy to that dashboard
  and returns stable/watch/repair status for side-effect gate and receipt-intake
  drift.
Its health and history record expose `allows_service_advance` and
`requires_repair_first`, giving service/eval the attempt-level admission signal
  before another service command is considered. Runtime service loop-run
  control plans translate transition health and queue state into the next
  daemon-style mode (`Continue`, `Observe`, `Repair`, or `Idle`). Adapter code
  can project that control plan into the same service-adapter owner boundary
  with `AgentAdapterBoundaryGate::from_runtime_service_loop_control_plan` or
  `from_runtime_service_loop_control_record`, and can store queue ids with the
  matching `AgentAdapterBoundarySnapshot::from_runtime_service_loop_control_*`.
  `Continue` with stable health opens service commands plus memory/adaptive
  promotion, `Observe` keeps service scheduling open while closing
  memory/adaptive promotion, `Idle` is a watch-level closed observation with an
  empty queue, and `Repair` closes the adapter boundary. Use
  `AgentAdapterBoundarySummaryHistoryRecorder::record_runtime_service_loop_control_*`
  helpers when eval/service wants that runtime-control projection appended into
  adapter-boundary health or converted into a repair-first handoff.
  The daemon record and daemon continuation can be projected with
  `AgentAdapterBoundaryGate::from_runtime_service_loop_daemon_record` or
  `from_runtime_service_loop_daemon_continuation` when the persisted daemon
  state is the final adapter read model. Those helpers use transition health,
  control health, mode, schedulability, side-effect-dispatch rate, memory-note
  rate, adaptive admission, and repair-first state to open stable `Continue`
  packets, keep watch packets service-observable without memory/adaptive
  promotion, or close all lanes for repair-first handoff tasks. The daemon
  input plan can be projected with
  `from_runtime_service_loop_daemon_input_plan` after receipts have been bound
  into the next full daemon input. Because that packet is an input assembly
  record rather than side-effect authority, it is always a watch-level
  observation: it preserves the next runtime queue for service/eval replay and
  may keep service scheduling observable when dispatch evidence is positive,
  but it never opens memory-note or adaptive-state promotion by itself.
  The daemon request plan can be projected before monitored request health
  exists with
  `AgentAdapterBoundaryGate::from_runtime_service_loop_daemon_request_plan` or
  `from_runtime_service_loop_daemon_request_record`. This earliest
  daemon-request adapter read model uses mode, schedulability,
  side-effect-dispatch rate, memory-note rate, adaptive admission, and
  repair-first state to decide whether the current runtime queue may cross the
  service boundary. Stable `Continue` opens service commands, memory notes, and
  adaptive promotion; watch/observe keeps service scheduling observable while
  closing memory/adaptive promotion; repair-first or `Repair` mode closes every
  side-effect lane and turns the request boundary into adapter repair work.
  The daemon request monitored plan can then be projected with
  `AgentAdapterBoundaryGate::from_runtime_service_loop_daemon_request_monitored_plan`
  or `from_runtime_service_loop_daemon_request_monitored_record`. It uses the
  monitored request health, daemon-control health, mode, schedulability,
  side-effect dispatch rate, memory-note rate, adaptive admission, and
  repair-first flag to decide whether service scheduling, memory notes, and
  adaptive state may cross the adapter boundary. Stable `Continue` opens all
  three lanes against the current runtime queue; watch preserves service
  observation while closing memory/adaptive promotion; repair closes all lanes
  and converts request/control blockers into adapter-boundary repair handoff
  tasks. Use the matching snapshot and recorder helpers when service/eval wants
  that monitored request boundary persisted before the request is closed.
  The later daemon request monitored-close packet can be projected through
  `AgentAdapterBoundaryGate::from_runtime_service_loop_daemon_request_monitored_close_plan`
  or `from_runtime_service_loop_daemon_request_monitored_close_run_record`, or
  `from_runtime_service_loop_daemon_request_monitored_close_continuation`.
  That projection reads only the close packet's stable fields: monitored-close
  health, request health status, daemon-control health status, mode,
  schedulability, side-effect dispatch rate, memory-note rate, adaptive
  admission, and repair-first state. Stable `Continue` packets open service
  commands, memory notes, and adaptive promotion against the current runtime
  queue, while continuation packets take that queue from the next daemon
  runtime input; watch packets may still let service observation advance while
  keeping memory/adaptive promotion closed; repair packets close every adapter
  side effect and preserve owner-qualified blockers for repair-first
  scheduling. Use
  the matching `AgentAdapterBoundarySnapshot::from_runtime_service_loop_daemon_request_monitored_close_*`
  and `AgentAdapterBoundarySummaryHistoryRecorder::record_runtime_service_loop_daemon_request_monitored_close_*`
  helpers when service/eval wants that final daemon-request boundary persisted
  or converted into an adapter-boundary repair handoff before the current
  runtime queue crosses into norion-memory, service, or eval.
  This keeps the adapter read model aligned with the admission booleans that
  later preflight and loop-state records already summarize.
  `AgentClosedLoopRuntimeServicePreflight` combines execution-history health and
  service-run attempt health into the final continue/observe/repair/idle
  preflight decision for the next runtime turn.
  It exposes `allows_service_advance` and `requires_repair_first` directly, so
  service/eval can read the same admission decision before expanding
  side-effect admission gates.
  `AgentClosedLoopRuntimeServicePreflight::side_effect_admission` projects that
  preflight decision into the same adapter admission read model used by the
  collaboration boundary, exposing dispatch, memory-note, and adaptive-state
  gates before the service attempts another side-effect boundary. When
  service/eval wants that preflight decision persisted in the final
  adapter-boundary ledger, use
  `AgentAdapterBoundaryGate::from_runtime_service_preflight` or
  `from_runtime_service_preflight_continuation` and the matching snapshot and
  recorder helpers. The preflight projection reuses `side_effect_admission`;
  the continuation projection stores the merged follow-up queue that will feed
  the next runtime input.
  `AgentClosedLoopRuntimeServicePreflightFollowUpPlan` converts observe/repair
  preflight reasons into `service-preflight` tasks and merges them into the next
  scheduler queue without executing side effects.
  `AgentClosedLoopRuntimeServicePreflightContinuationPlanner` packages that
  merged queue with budgets, policies, evidence, completed ids, and execution
  history into the next `AgentClosedLoopRuntimeTurnInput`.
  `AgentClosedLoopRuntimeServiceLoopState` and its planner bind execution
  history, service-run attempt history, the preflight continuation, and
  loop-state telemetry into one service-held snapshot for the next turn.
  The loop-state and loop-advance wrappers mirror the preflight
  `allows_service_advance` and `requires_repair_first` helpers, keeping
  stable/watch snapshots observable while repair snapshots block the next
  service advance.
  `AgentClosedLoopRuntimeServiceLoopStateSummary` gives service/eval a compact
  row for preflight mode, health statuses, schedule/adaptive flags, history
  sizes, follow-up volume, next queue size, reasons, and flattened
  side-effect admission health plus dispatch/memory-note/adaptive-state gates.
  `AgentClosedLoopRuntimeServiceLoopAdvancePlanner` composes service-run
  history recording and loop-state planning so one completed service run can
  advance into the next service-held loop snapshot.
  `AgentClosedLoopRuntimeServiceLoopRunner` composes the receipt-aware service
  runner and loop-advance planner into one service-facing transition over
  `EnginePort`, `MemoryPort`, caller-owned receipts, and service-run history.
  `AgentClosedLoopRuntimeServiceLoopRunSummary` compacts that transition into a
  service/eval row with run status, blockers, preflight mode, health, attempts,
  follow-up volume, next queue size, command-gate status, side-effect gate
  counts, side-effect admission health, and dispatch/memory/adaptive gate
  booleans.
  `AgentClosedLoopRuntimeServiceLoopRunHistory` and
  `AgentClosedLoopRuntimeServiceLoopRunDashboard` aggregate those transition
  rows across service-loop turns for closed rate, repair-first rate,
  dispatch-admission rate, memory-note admission rate, adaptive-admission rate,
  command-gate allowed rate, side-effect gate pressure, command pressure,
  follow-up volume, and latest blockers.
  `AgentClosedLoopRuntimeServiceLoopRunHealth` applies a transition-level
  policy to that dashboard so intake drift and repair-first pressure become
  repair signals while low closed/adaptive rates and latest blockers remain
  watch signals.
  `AgentClosedLoopRuntimeServiceLoopRunHistoryRecorder` appends one completed
  loop transition and returns the updated transition history, dashboard, health,
  and telemetry for service/eval persistence.
  Its health and history record expose `records()`, `allows_service_advance`,
  and `requires_repair_first`, keeping stable/watch loop transitions observable
  while forcing repair-level transition drift through repair-first scheduling.
  `AgentClosedLoopRuntimeServiceLoopRunControlPlan` then projects transition
  health plus the next queue into continue/observe/repair/idle mode,
  scheduler/adaptive-evolution admission, and deterministic reasons.
  `AgentClosedLoopRuntimeServiceLoopRunMonitor` combines recording and control
  planning after one transition so a daemon-style service loop can persist the
  transition row and decide the next mode from one pure-data result.
  The control plan, monitor record, daemon record, and monitored-close record
  all expose `allows_service_advance` and `requires_repair_first` as wrapper
  read models over the same repair-first decision; explicit command,
  memory-note, and adaptive-state gates remain the only side-effect authority.
  `AgentClosedLoopRuntimeServiceLoopRunControlSummary` compacts that monitor
  record into one service/eval ledger row with latest status, mode, health,
  transition rates, dispatch/memory/adaptive admission rates, queue size,
  admission booleans, and reasons.
  `AgentClosedLoopRuntimeServiceLoopRunControlSummaryHistory` and
  `AgentClosedLoopRuntimeServiceLoopRunControlDashboard` aggregate those flat
  rows across daemon-loop turns for schedule rate, dispatch-admission rate,
  memory-note admission rate, adaptive-admission rate, repair-first pressure,
  idle pressure, queue pressure, and latest reasons.
`AgentClosedLoopRuntimeServiceLoopRunControlHealth` applies a policy to that
flat dashboard so daemon-control trends become stable/watch/repair gate data
before the service opens another self-evolution turn.
Its health and summary history record expose `records()`,
`allows_service_advance`, and `requires_repair_first` as the daemon-control
service/eval admission surface.
`AgentClosedLoopRuntimeServiceLoopRunControlSummaryHistoryRecorder` appends a
flat control row and returns the updated summary history, dashboard, health,
and telemetry as one service/eval persistence record.
  `AgentClosedLoopRuntimeServiceTurnCloser` audits service command receipts from
  that business turn and appends the compact execution summary to history.
  `AgentClosedLoopRuntimeContinuationPlanner` turns that updated history and
  next queue into the next `AgentClosedLoopRuntimeTurnInput`.
  `AgentClosedLoopRuntimeServiceOutcomePlanner` combines receipt audit and
  continuation planning when the service already has receipts for a command
  request.
  `AgentClosedLoopRuntimeServiceOutcomeSummary` projects that outcome into a
  compact eval/service row with runtime mode, command count, clean status,
  health status, next queue size, and skipped reasons.
- `service`: `AgentServiceCommandPlanner` for translating a
  `AgentBusinessLoopPlan` into service-executable command data such as promote
  adaptive state, hold, open repair mode, enqueue tasks, and emit telemetry;
  it also audits `AgentServiceCommandReceipt` values after the service attempts
  those commands. `AgentServiceCommandPlanSummary` exposes command count,
  command kinds, adaptive-write pressure, repair/hold/enqueue intent, queued
  task count, and telemetry-command pressure before the service executes IO.
  `AgentServiceCommandAuditSummary` compacts receipt drift into expected,
  received, missing, failed, skipped, clean, and blocked-reason evidence.
  `AgentServiceFeedbackSummary`, `AgentServiceTurnoverSummary`, and
  `AgentServiceExecutionReportSummary` carry repair-task pressure, merged queue
  size, service cleanliness, and blocker evidence after receipts are audited.
- `ports`: `EnginePort`, `MemoryPort`, and `ToolBuildPort` traits plus the
  `ToolBuildRequest`, `ToolBuildReceipt`, `ToolBuildReport`, and
  `ToolBuildReportSummaryHistory` read models. These are adapter boundaries
  only; this crate does not call an LLM, HTTP service, disk memory, or tool
  builder directly. Tool-build report histories keep cross-turn build receipt
  health visible to service/eval without persisting artifacts or other build
  payloads, and `ToolBuildReportHistoryGate` converts that health into the
  next tool-build, memory, adaptive-state, and eval admission booleans.

## 6-window collaboration model

Each window should be represented as an `AgentRole` or `AgentRole::Custom("window-n")` plus an `AgentTask`. The coordinator gives every role its own `AgentBudget`; dispatch only deducts from that role. A busy or over-budget window is rejected without consuming another window's allowance.
After execution, `RunBudgetAudit` compares each assignment's reserved budget to
the reported `AgentResult::budget_spent`; overspends are preserved in
`AgentRunReport` and should block promotion until repaired.

`AgentWindowSpec` is the stable input shape when the main window already knows
the active collaboration windows. `AgentCollaborationPlanner` turns those specs
into an `AgentTaskQueue`, a per-role `BudgetLedger`, active-window ids,
duplicate-window blockers, and telemetry. Duplicate physical window ids are
blocked before they can overwrite another task in the queue, preserving the
single-writer rule for each window.
Use this as the multi-window contract: each window declares a stable id, role,
objective, lane, priority, dependencies, and isolated budget before dispatch;
each window writes only inside its assigned ownership boundary; and each handoff
reports task id, role/window id, status, accepted/rejected decision, budget
spent, changed files or artifact ids, blocker reasons, repair tasks, conflict
topics, and side-effect admission booleans. The coordinator must aggregate
those rows before any memory note, adaptive-state write, service command, or
tool-build side effect opens. Unresolved conflicts, duplicate window ids,
shared budget slots, budget exhaustion, or dirty repair-first history close
side-effect admission and produce repair-first work instead of promoting the
normal queue. When a caller intentionally allows zero-budget windows for
observation, strict dispatch still rejects only the depleted window while
preserving other roles' isolated budgets and assignments.

Multi-window handoff readiness is proven through fieldized rows, not raw
handoff payloads:

| Contract field | Evidence |
| --- | --- |
| Polluted-window isolation | `AgentWindowOwnershipReviewSummary` carries only counts and deterministic repair ids; `AgentCollaborationOwnershipPreflightGate` emits owner-prefixed reason codes before dispatch opens. |
| Repair-first | `requires_repair_first`, repair queue ids, and closed side-effect booleans are asserted by ownership, dispatch-preflight, scheduler-handoff, and adapter-boundary handoff tests. |
| Business task preservation | Scheduler handoff and adapter-boundary packet tests assert original queue ids remain present and depend on generated repair ids before business waves run. |
| Budget/conflict/ownership isolation | Collaboration plan, budget, conflict, and ownership gates expose isolated budget summaries, duplicate/shared-role blockers, conflict counts, ownership repair ids, and reason codes. |

`AgentWindowContextReallocationSummary` is the read-only contract for a main
window that detects context pollution after a handoff. It fieldizes
`clean`, `polluted`, `stale`, `paused`, and `clean-room-replacement` states,
keeps business task ids and evidence-backed result ids as the only carry-forward
identity, exposes normalized reason codes, and keeps
`side_effects_allowed=false`. A polluted, stale, or paused original window
blocks further assignment to that context; a clean-room replacement can receive
only the narrow requested task ids, not raw prior-window payload.

`AgentTaskQueue` holds pending work until dependencies are satisfied. The
coordinator can drain ready tasks in stable priority/id order, dispatch that
wave, then mark completed task ids before draining the next wave.
`RecursiveAgentScheduler` can produce the same wave plan up front and leaves
cycles or missing dependencies as blocked tasks for coordinator repair.

Messages from the six windows are submitted to `MessageAggregator` before the main window applies any changes. Duplicate findings collapse into one `AggregatedMessage` with `duplicate_count` and `source_ids`, so repeated reports strengthen evidence without bloating the transcript.

`ConflictResolver` marks messages when one window says to proceed and another
blocks the same topic. The main window should treat unresolved conflicts as a
stop condition for side effects: ask the coordinator to resolve, run additional
tests, or select a single writer. A `ConflictResolutionBook` may resolve a
conflict only when it references the same topic, covers all conflicting message
ids, and carries a non-empty rationale; side-effect gates are then recomputed
from the resolved report.

After reflection, `ClosedLoopRewarder` can score the run report with validation,
runtime-response, recursion, and toolsmith evidence. A high-confidence clean run
emits reinforce-oriented `EvolutionSignal` values; unresolved conflicts or
failed validation emit repair/penalty signals instead.

`AgentCycleOrchestrator` ties the pieces together without taking ownership of
runtime IO. It can plan the next ready wave from `AgentTaskQueue` and
`BudgetLedger`, then close externally produced `AgentResult` values into an
`AgentRunReport`, `ProcessRewardReport`, and follow-up tasks for reinforcement,
hold-for-evidence, or repair.
`AgentCycleSummary` is the narrow handoff shape for future `norion-eval`
adapters: assigned/rejected tasks, unique/duplicate message counts, unresolved
conflicts, blocked side effects, budget overspends, execution failures, reward
action, evolution signal count, and follow-up task count.
When a cycle is reinforced, has no unresolved conflicts or budget overspends,
and the memory side-effect gate is open, `MemoryPromotion` converts the
reflection memory note into a `MemoryNote` candidate with reward/report
evidence. The crate still does not persist it; the service must pass it through
`MemoryPort`.
If conflicts remain unresolved, high quality or complete reflection is not
enough to promote: memory notes stay empty, side-effect gates stay closed, and
follow-up work must be hold/repair oriented until a `ConflictResolutionBook`
covers every conflicting message id.
`AgentCycleHandoff` packages the notes that may be submitted, follow-up tasks to
enqueue, and blocked reasons that explain why memory submission is not allowed.
`AgentCollaborationReview` is the compact main-window audit view after a cycle
closes. It carries `AgentCycleSummary`, `AgentCycleHandoff`, memory and
adaptive-state gates, derived `can_submit_memory` and
`can_write_adaptive_state` booleans, blocked reasons, and telemetry. The service
can use it before deciding whether to call `MemoryPort`, promote adaptive
state, or ask a coordinator window to repair conflicts and budget drift.
`AgentCollaborationReviewHistory`, `AgentCollaborationDashboard`,
`AgentCollaborationHealthPolicy`, `AgentCollaborationHealth`, and
`AgentCollaborationReviewHistoryRecorder` aggregate those reviews across
cycles. They classify memory/adaptive-state admission rates, blocked-review
rate, unresolved-conflict pressure, and budget-overspend pressure as
stable/watch/repair data before the business loop promotes long-lived adaptive
state.
`AgentCollaborationBusinessLoopPlanner` combines that collaboration health with
`AgentBusinessLoopController`. A clean ledger may still be held when
collaboration history is missing or watch-level, and it is forced into repair
when collaboration health reports conflict, blocked-review, or budget pressure.
Stable collaboration health is required before the collaboration-aware plan
exposes an adaptive-state candidate for promotion.
`AgentCollaborationBusinessLoopSummary` is the compact service/eval row for
that final gate: base business status, collaboration health, effective status,
promotion/repair booleans, candidate presence, review count, repair-task count,
blocked-reason count, and telemetry.
After the service attempts memory submission, `AgentCycleLedgerRecord` can bind
the `AgentCycleSummary`, `MemorySubmissionReport`, and validation/runtime
evidence references into one eval-facing row. `AgentReportGate` accepts only
clean reinforced rows with no execution failures, unresolved conflicts, blocked
side effects, budget overspends, dirty tool-build receipts, missing evidence,
or failed memory submission; otherwise it returns stable blocker codes plus
tester, reviewer, planner, tool-build, or memory-curator repair tasks.
`AgentReportGateDecision::summary` is the compact row for dashboards: it
preserves blocker order, repair lane order, and the
memory/budget/validation/runtime/review/tool-build blocker families without
expanding the full follow-up task payloads.
`AgentLoopbackPlanner` is the final pure-data bridge back into the business
loop: accepted decisions may promote adaptive state and enqueue reinforcement
tasks, while blocked decisions enqueue gate repair tasks before ordinary
handoff follow-ups. `AgentLoopbackPlan::summary` is the service/eval row for
that bridge: it carries promotion admission, queued work, blocked-reason count,
task ids, repair lanes, and whether the next wave can be scheduled.
Across runs, `AgentCycleLedger` records the cycle row, report-gate decision, and
loopback plan for each iteration. Its summary exposes acceptance rate,
consecutive blocked cycles, adaptive promotions, queued repair volume, latest
blocked reasons, tool-build report-gate blocker cycles, and average reward.
Ledger dashboards and health policies keep those `tool_build_*` report-gate
blockers separate from generic review pressure, and
`AgentCycleLedger::admission` forces repair when that tool-build blocker
pressure is still present. The admission decision keeps the self-evolution path
in promote, hold, or repair mode based on that trend rather than one isolated
success.
`AgentBusinessLoopController` then turns the ledger into the service-facing
control packet: a next `AgentTaskQueue`, stable telemetry lines, and an
`AdaptiveStateCandidate` only when the multi-cycle admission status is
`Promote` and the latest loopback plan actually allowed adaptive-state
promotion. `AgentBusinessLoopPlan::summary` is the compact service/eval row for
that final in-crate control point: it carries admission status, trend counters,
next-queue size, candidate presence, promotion admission, repair-first status,
tool-build blocked-cycle pressure, reason count, and evidence-ref count before
service commands are planned.
When the service also keeps collaboration review history, prefer
`AgentCollaborationBusinessLoopPlanner` for the stricter final promotion check.
It preserves the base business plan, records collaboration dashboard and health,
computes an effective promote/hold/repair status, suppresses adaptive-state
candidates unless both ledger and collaboration trends are stable, and emits
repair tasks for collaboration-health failures.
Use `AgentCollaborationBusinessLoopPlan::summary` when service/eval only needs
the compact final-admission row rather than the nested business plan,
collaboration dashboard, and repair task list.
Use `AgentCollaborationBusinessLoopPlan::effective_business_plan` when the
service needs to hand the collaboration-aware decision to
`AgentServiceCommandPlanner`. The effective plan rewrites admission status and
reasons, suppresses unsafe adaptive-state candidates, and merges
collaboration-health repair tasks into the next queue before service commands
are produced.
Use `AgentCollaborationBusinessLoopPlan::close_service_execution` or
`AgentCollaborationServiceExecutionCloser` after the service has attempted those
commands and collected receipts. The resulting
`AgentCollaborationServiceExecutionReport` preserves the collaboration plan, the
effective business plan, the service execution report, next queue, promotion
gate status, and telemetry without executing any side effect itself.
Use `AgentCollaborationServiceExecutionReport::summary` when service/eval needs
the compact receipt-close row: effective status, collaboration health, clean
flag, command count/kinds, adaptive-write requirement, promotion permission,
repair-mode flag, memory-promotion and tool-build command-reason counts,
next-queue size, blocked-reason count, and telemetry.
Append those summaries to `AgentCollaborationServiceExecutionHistory` with
`AgentCollaborationServiceExecutionHistoryRecorder` when service/eval wants a
collaboration-specific receipt-close trend. The resulting dashboard tracks
clean/dirty rates, blocked receipt pressure, repair-mode pressure,
memory-promotion and tool-build command-reason executions, queued follow-up
pressure, adaptive-promotion rate, latest effective status, latest collaboration
health, and latest clean flag. Its health policy keeps dirty
receipts in repair mode by default while treating repeated clean repair-mode
closures as watch-level collaboration pressure for the next self-evolution
boundary. The returned history record exposes `latest()`, `records()`,
`allows_service_advance`, and `requires_repair_first` for service/eval
admission checks before promotion is considered.
Use `AgentCollaborationSelfEvolutionPlanner` after both collaboration review
history and collaboration service-execution history have been updated. It keeps
promotion open only when the collaboration-aware business plan is promotable and
receipt-close health is stable. Missing receipt history or watch-level service
execution trends downgrade the final status to hold; dirty receipt trends
downgrade it to repair, suppress the adaptive-state candidate, and append
`collaboration-service-execution-repair` tasks to the next queue.
Use `AgentCollaborationSelfEvolutionCloser` when receipts have just arrived and
the service wants the whole receipt boundary closed in one pure-data call. It
closes the collaboration-aware service execution report, appends the
receipt-close summary to `AgentCollaborationServiceExecutionHistory`, computes
the final self-evolution plan, and merges the current `service-feedback` repair
queue with the longer-lived `collaboration-service-execution-repair` queue
before exposing the final `AgentBusinessLoopPlan`.
Persist `AgentCollaborationSelfEvolutionCloseSummary` rows in
`AgentCollaborationSelfEvolutionCloseHistory` when service/eval needs trend
evidence at the final admission boundary. Its dashboard and health policy track
service-clean rate, dirty receipt pressure, promotion rate, repair-first
pressure, final-repair rate, blocked rate, queued follow-up pressure, latest
status, and latest service-execution health. The returned history record exposes
`latest()`, `records()`, `allows_service_advance`, and
`requires_repair_first` before the controller chooses the next mode.
Use `AgentCollaborationSelfEvolutionController` after close-history health is
available and the service has a candidate next queue. Stable health with queued
work produces `Continue` and keeps adaptive evolution open; watch health
produces `Observe`; repair health produces `Repair` and closes adaptive
evolution; an empty queue produces `Idle` with `next_queue_empty`.
Keep `AgentCollaborationSelfEvolutionControlHistory` beside the close-history
ledger when service/eval needs a trend view of controller decisions themselves.
Append each `AgentCollaborationSelfEvolutionControlSummary` with
`AgentCollaborationSelfEvolutionControlHistoryRecorder`; its dashboard and
health policy track schedule rate, adaptive-admission rate, repair-first
pressure, observe pressure, repair pressure, idle pressure, queued work, latest
mode, and latest close-health status. The returned record also exposes
`allows_service_advance` and `requires_repair_first` so service/eval can use the
same stable/watch/repair admission read model as the downstream gates.
Use `AgentCollaborationSelfEvolutionControlMonitor` when the adapter wants to
combine control planning and compact control-history recording after a close
history record has been produced. It returns the control plan, appended
control-history record, and merged telemetry without executing service commands
or writing memory.
Use `AgentCollaborationSelfEvolutionControlGate` before service command
planning when persisted control trends should constrain the current turn. Stable
control health preserves the requested mode, watch health downgrades a requested
`Continue` to `Observe`, repair health forces `Repair`, and an empty queue still
idles. The gate exposes effective schedule/adaptive-evolution booleans and
reason rows without mutating the original control plan.
Use `AgentCollaborationSelfEvolutionControlHandoff` when the adapter wants the
whole post-close control boundary in one call. It records the compact control
summary, evaluates the control-history gate, and returns both the monitor record
and the effective gate decision with combined telemetry.
Use `AgentCollaborationSelfEvolutionServiceHandoff` after
`AgentCollaborationSelfEvolutionCloser` has returned a close record and before
service command planning. It appends the final close summary, evaluates the
control handoff from the close record's next queue, and returns close-history
evidence, control-history evidence, the effective scheduling mode, and combined
telemetry in one pure-data packet.
Call `AgentCollaborationSelfEvolutionServiceHandoffRecord::command_admission`
when the adapter needs the final pre-command gate. The admission reports whether
service command planning should proceed, whether adaptive evolution remains
open, whether repair must run first, and why the handoff was held or repaired.
Use `AgentCollaborationSelfEvolutionCloseAndAdmit` when receipts are available
and the service wants the full post-service boundary in one call. It runs
`AgentCollaborationSelfEvolutionCloser`, appends close-history evidence, applies
control-history gating, and returns the final service command admission without
executing any commands.
Persist `AgentCollaborationSelfEvolutionCloseAndAdmitSummary` rows in
`AgentCollaborationSelfEvolutionCloseAndAdmitHistory` when service/eval needs a
compact multi-run view of that boundary. Its dashboard tracks clean receipts,
command-plannable records, adaptive admission, idle pressure, repair pressure,
repair-first pressure, and the latest effective mode.
Use `AgentCollaborationSelfEvolutionCloseAndAdmitHistoryRecorder` when callers
want that summary appended with dashboard health in one step. The health policy
marks empty history and high idle pressure as watch-level evidence, while repair
or repair-first pressure forces repair-level evidence before the next command
planning turn. The returned record exposes `allows_service_advance` and
`requires_repair_first` before the continuation planner chooses the next
command-admission mode.
Use `AgentCollaborationSelfEvolutionCloseAndAdmitContinuationPlanner` when the
service wants that trend health projected into the next command-admission mode:
stable trends continue, watch trends observe without adaptive evolution, and
repair trends force repair-first command planning.
Use `AgentCollaborationSelfEvolutionCloseAndAdmitMonitor` when service/eval
wants the whole post-receipt control packet in one call. It closes receipts,
records the close-and-admit summary, evaluates trend health, and returns the
continuation plan with combined telemetry.
Use `AgentCollaborationSelfEvolutionServiceEvalPacket` when that monitor record
must be flattened for service/eval before command execution. It recomputes the
expected `AgentServiceCommandPlan`, exposes command count/kinds and dispatch
readiness, blocks unadmitted adaptive-state writes, and requires repair-first
turns to carry an actual repair command without executing any command itself.
Persist `AgentCollaborationSelfEvolutionServiceEvalPacketSummary` rows in
`AgentCollaborationSelfEvolutionServiceEvalPacketHistory` when eval needs
cross-turn evidence for dispatch-blocked rate, adaptive-write admission rate,
repair-first command readiness, latest mode, latest health, and command
pressure. The recorder appends packet summaries and recomputes stable, watch,
or repair health without touching command execution.
Use `AgentCollaborationSelfEvolutionServiceEvalPacketMonitor` when the service
wants that packet plus the appended packet-history record from one call after a
close-and-admit monitor record has been produced.
Use `AgentCollaborationSelfEvolutionServiceEvalReflectionPacket` after packet
monitoring when service/eval needs a pure-data reflection gate. It turns
packet-health blockers into stable reflection focus strings, blocks memory-note
promotion while reflection is required, and only admits adaptive evolution when
the packet is dispatchable and its adaptive-state write is already admitted.
Persist `AgentCollaborationSelfEvolutionServiceEvalReflectionSummary` rows in
`AgentCollaborationSelfEvolutionServiceEvalReflectionHistory` when eval needs a
cross-turn dashboard for reflection-required pressure, memory-note promotion
rate, adaptive-evolution admission rate, latest health, and total reflection
focus.
Use `AgentCollaborationSelfEvolutionServiceEvalReflectionMonitor` when callers
want reflection planning, summary recording, trend health, and merged telemetry
from one packet-monitor record.
Use `AgentCollaborationSelfEvolutionServiceEvalReflectionContinuationPlanner`
after that monitor when the service wants one persisted continuation decision:
stable and clear reflection state continues, watch or required-reflection state
observes without promotion, and repair health forces repair-first before memory
notes or adaptive evolution are admitted.
Use `AgentCollaborationSelfEvolutionServiceEvalReflectionCloseContinuationPlanner`
when service/eval wants the whole post-receipt chain as a single pure-data
record. It can start from an existing close-and-admit monitor record or close
fresh receipts itself, then append packet and reflection histories, compute the
reflection continuation, and expose one compact summary for persistence.
Persist `AgentCollaborationSelfEvolutionServiceEvalReflectionCloseContinuationSummary`
rows in `AgentCollaborationSelfEvolutionServiceEvalReflectionCloseContinuationHistory`
when eval needs trend evidence for continue/observe/repair pressure,
dispatchability, memory-note promotion, adaptive-evolution admission, and
repair-first blockers. The recorder appends one row and recomputes dashboard
health without touching service commands or memory.
Use `AgentCollaborationSelfEvolutionServiceEvalReflectionCloseContinuationMonitor`
when service/eval wants that close-continuation record and the appended
close-continuation-history health record from one call.
Use `AgentCollaborationSelfEvolutionServiceEvalReflectionCloseContinuationControlPlanner`
after the monitor when the latest row must be constrained by cross-turn
close-continuation health. Stable trends preserve the requested mode, watch
trends observe before promotion, and repair trends force repair-first admission
before memory notes or adaptive evolution can continue.
Persist `AgentCollaborationSelfEvolutionServiceEvalReflectionCloseContinuationControlSummary`
rows in `AgentCollaborationSelfEvolutionServiceEvalReflectionCloseContinuationControlHistory`
when eval needs the final post-reflection control trend: requested mode,
effective mode, dispatch admission, memory-note promotion, adaptive-evolution
admission, repair-first pressure, and reasons.
Use `AgentCollaborationSelfEvolutionServiceEvalReflectionCloseContinuationControlMonitor`
when service/eval wants the latest control plan and appended control-history
health from one call after close-continuation monitoring.
Use `AgentCollaborationSelfEvolutionServiceEvalReflectionCloseContinuationControlGate`
when that recorded control-history health must constrain the next admission:
stable preserves, watch observes, and repair forces repair-first before service
commands, memory notes, or adaptive evolution continue.
Use `AgentCollaborationSelfEvolutionServiceEvalReflectionCloseContinuationControlHandoff`
when adapters want the close-continuation control monitor and gate decision from
one call, with combined telemetry ready for service/eval persistence.
`AgentCollaborationSelfEvolutionServiceEvalReflectionCloseContinuationControlAdmission`
projects that handoff into final side-effect booleans for service command
dispatch, memory-note promotion, adaptive-evolution admission, repair-first
handling, and stable blocker telemetry.
`AgentCollaborationSelfEvolutionServiceEvalReflectionCloseContinuationControlAdmissionSummary`
is the compact eval row for that final admission, preserving the effective mode,
side-effect booleans, repair-first flag, reason count, and summary telemetry
without expanding the nested handoff.
Persist those rows in
`AgentCollaborationSelfEvolutionServiceEvalReflectionCloseContinuationControlAdmissionHistory`
when eval needs cross-turn trend health for final dispatch, memory-note
promotion, observe pressure, repair pressure, and repair-first admission.
Use `AgentCollaborationSelfEvolutionServiceEvalReflectionCloseContinuationControlAdmissionMonitor`
when the adapter already has a control handoff and wants the admission,
admission-history record, health, and combined telemetry from one pure-data
call.
Use `AgentCollaborationSelfEvolutionServiceEvalReflectionCloseContinuationControlAdmissionGate`
after that monitor when recorded admission health must constrain the current
side-effect boundary: stable preserves, watch observes, and repair forces
repair-first before service commands, memory notes, or adaptive evolution run.
Use `AgentCollaborationSelfEvolutionServiceEvalReflectionCloseContinuationControlAdmissionHandoff`
when adapters want admission monitoring and the recorded-health gate composed
into one persistable service/eval packet with final side-effect booleans.
`AgentCollaborationSelfEvolutionServiceEvalReflectionCloseContinuationControlAdmissionHandoffSummary`
is the compact eval row for that packet, preserving gate-applied mode, health,
side-effect booleans, repair-first, and blocker counts.
Persist those summaries in
`AgentCollaborationSelfEvolutionServiceEvalReflectionCloseContinuationControlAdmissionHandoffHistory`
when eval needs trend health over the gate-applied final service/eval packets.
Use
`AgentCollaborationSelfEvolutionServiceEvalReflectionCloseContinuationControlAdmissionHandoffContinuation`
after admission-handoff monitoring when service/eval needs the next
continue/observe/repair mode plus dispatch, memory-note, adaptive-evolution,
and repair-first booleans from one pure-data boundary.
`AgentCollaborationSelfEvolutionServiceEvalReflectionCloseContinuationControlAdmissionHandoffContinuationHandoff`
then records that continuation and applies recorded-health gating into a
replayable packet.
Persist gate-applied packets with
`AgentCollaborationSelfEvolutionServiceEvalReflectionCloseContinuationControlAdmissionHandoffContinuationHandoffMonitor`;
its paired
`AgentCollaborationSelfEvolutionServiceEvalReflectionCloseContinuationControlAdmissionHandoffContinuationHandoffContinuation`
turns the recorded handoff health back into the next boundary mode without
executing service commands, writing memory notes, or promoting adaptive state.
Use its `Gate` and `Handoff` variants when service/eval wants that recorded
continuation health applied before another side-effect boundary opens; stable
keeps the requested mode, watch observes, and repair forces repair-first.
Persist the gate-applied packet with
`AgentCollaborationSelfEvolutionServiceEvalReflectionCloseContinuationControlAdmissionHandoffContinuationHandoffContinuationHandoffHistory`
or its monitor when eval needs trend health over final dispatch, memory-note,
adaptive-evolution, and repair-first outcomes. These rows remain pure evidence;
the outer service still owns command execution, memory writes, and adaptive
state promotion.
For adapter code that should not know the full nested handoff type,
`AgentCollaborationSideEffectBoundary` projects the final handoff or monitor
into a short service/eval snapshot: dispatch is exposed as an external-call
gate, memory promotion as a memory-note gate, adaptive evolution as an
adaptive-state gate, with the mode, health, repair-first flag, reasons, and
telemetry preserved.
Use `AgentCollaborationSideEffectBoundarySummary` and
`AgentCollaborationSideEffectBoundaryHistoryRecorder` when eval wants compact
trend rows over those adapter-facing gates. The dashboard reports dispatch,
memory-note, adaptive-state, repair, and repair-first rates; its health policy
turns repair pressure into `Repair` and low dispatch/memory admission into
`Watch`. The boundary health and history record expose the same
service-advance helpers used by the nested service/eval rows, so adapters can
observe stable/watch evidence while blocking repair trends before any writer
opens.
Use `AgentCollaborationSideEffectBoundaryGate` immediately before an adapter
executes side effects. It applies the recorded side-effect-boundary health back
to the current boundary: stable preserves the current gates, watch observes, and
repair forces repair-first while closing dispatch, memory-note, and
adaptive-state admission.
Use `AgentCollaborationSideEffectBoundaryHandoff` when the adapter wants the
whole sequence in one pure-data call: append the short boundary row, compute
boundary health, apply the gate, and receive final side-effect gates plus
combined telemetry before executing service commands or calling `MemoryPort`.
Persist `AgentCollaborationSideEffectBoundaryHandoffSummary` rows with
`AgentCollaborationSideEffectBoundaryHandoffHistoryRecorder` when service/eval
needs trend health over the final adapter-facing handoff itself: dispatch,
memory-note, adaptive-state, repair, repair-first, blocked-gate, and reason
rates are available without expanding nested final handoff records. Its health
and history record expose the same stable/watch advance and repair-first
helpers for the final adapter-facing handoff trend.
Use `AgentCollaborationSideEffectBoundaryHandoffMonitor` when the adapter wants
one pure-data call that gates the short boundary and records the gate-applied
handoff row for service/eval persistence before any side effect is attempted.
`AgentCollaborationSideEffectBoundaryHandoffMonitorSummary` flattens that
monitor record into mode, boundary health, handoff trend health, side-effect
admission booleans, repair rates, and blocked-gate counts for dashboards.
Persist those flat monitor summaries with
`AgentCollaborationSideEffectBoundaryHandoffMonitorSummaryHistoryRecorder` when
service/eval needs cross-turn health over the final adapter boundary itself.
Its dashboard separates boundary repair pressure from handoff-trend repair
pressure, while the health policy can force repair before the next adapter
side-effect boundary opens.
Apply that recorded health with
`AgentCollaborationSideEffectBoundaryHandoffMonitorGate`: stable monitor trends
preserve the current side-effect gates, watch trends observe, and repair trends
force repair-first while closing service dispatch, memory-note promotion, and
adaptive-state admission.
Use `AgentCollaborationSideEffectBoundaryHandoffMonitorHandoff` when service/eval
wants that append-and-gate path as one final adapter packet: it records the flat
monitor summary history, applies the recorded health gate, and returns the final
service dispatch, memory-note, and adaptive-state admission booleans with
telemetry.
Persist `AgentCollaborationSideEffectBoundaryHandoffMonitorHandoffSummary` rows
with `AgentCollaborationSideEffectBoundaryHandoffMonitorHandoffHistoryRecorder`
when dashboards need trend health over that final gate-applied adapter packet.
Those rows expose final mode, side-effect admission, blocked-gate pressure, and
repair-first pressure without expanding nested monitor records.
Apply that final packet trend with
`AgentCollaborationSideEffectBoundaryHandoffMonitorHandoffGate` before the next
adapter boundary opens; it preserves stable admission and forces repair-first
while closing dispatch, memory-note, and adaptive-state gates when final packet
history is dirty.
`AgentCollaborationAdapterSideEffectAdmission` is the shortest service/eval
read model for that final gate: adapters can inspect mode, health,
service-dispatch, memory-note, adaptive-state, repair-first, gates, reasons,
and telemetry without expanding the long monitor-handoff type chain.
`AgentCollaborationAdapterSideEffectAdmissionSummary`,
`AgentCollaborationAdapterSideEffectAdmissionHistory`, and
`AgentCollaborationAdapterSideEffectAdmissionHistoryRecorder` make that final
read model persistable as compact eval/dashboard rows. Their dashboard and
health policy expose adapter-level continue/observe/repair, dispatch,
memory-note, adaptive-state, and repair-first trends while still leaving all
service commands, memory notes, and adaptive writes outside `norion-agent`.
`AgentCollaborationAdapterSideEffectAdmissionGate` applies that short-history
health back to the next adapter admission: stable keeps the current gates, watch
observes, and repair forces repair-first while closing dispatch, memory-note,
and adaptive-state admission.
`AgentCollaborationAdapterSideEffectAdmissionMonitor` composes the short
history recorder and gate into one service/eval boundary. It returns the
persisted admission row, updated dashboard health, final gate decision, and
telemetry without asking adapters to expand any deeper monitor-handoff state.
`AgentCollaborationAdapterSideEffectAdmissionMonitorSummaryHistoryRecorder`
persists those monitor records as the final adapter-facing trend ledger. Its
dashboard health reports whether the side-effect admission boundary is still
stable, should be observed, or must repair-first before the service attempts
external dispatch, memory-note promotion, or adaptive-state writes.
`AgentCollaborationAdapterSideEffectAdmissionMonitorGate` applies that final
monitor trend health back to the next adapter boundary, preserving stable gates
and closing all three side-effect gates when the final monitor trend requires
repair-first work.
`AgentCollaborationAdapterSideEffectAdmissionMonitorHandoff` composes monitor
summary recording and final monitor trend gating into one adapter-facing record,
so service/eval can persist the last short ledger row and read the effective
side-effect gates from the same pure-data packet.
`AgentCollaborationAdapterSideEffectAdmissionMonitorHandoffSummary`,
`AgentCollaborationAdapterSideEffectAdmissionMonitorHandoffHistory`,
`AgentCollaborationAdapterSideEffectAdmissionMonitorHandoffHistoryRecorder`,
and `AgentCollaborationAdapterSideEffectAdmissionMonitorHandoffGate` make that
adapter packet itself trendable. This gives service/eval a final compact
handoff ledger with dashboard health and one more gate application before any
external dispatch, memory-note promotion, or adaptive-state write leaves the
process. `AgentCollaborationAdapterSideEffectAdmission::from_adapter_monitor_handoff_gate`
projects that last gate decision back into the same adapter-admission read
model for the next service/eval boundary.
Use `AgentReportGateHealthGateTrendHandoff` when eval wants one pure-data packet
that appends report-gate health-gate trend history and applies the trend gate
before loopback, adaptive-state promotion, or service commands advance.
Persist `AgentReportGateHealthGateTrendHandoffSummary` rows through
`AgentReportGateHealthGateTrendHandoffHistoryRecorder` when dashboards need the
same boundary without full queue payloads. Its dashboard and health policy track
admission rate, repair-first pressure, trend-health mix, queued-work counts,
repair task counts, and blocked reasons so the next service/eval boundary can
observe or repair report-gate drift before memory notes or adaptive writes are
allowed.
Apply that persisted state with `AgentReportGateHealthGateTrendHandoffGate`
when the current handoff must be checked against cross-run handoff health:
stable/watch health preserves the current gate decision, while repair health
adds `eval-report-gate-health-gate-trend-handoff` repair tasks and closes the
effective admission.
Use `AgentReportGateHealthGateTrendHandoffMonitor` when service/eval wants that
append-and-gate sequence as a single deterministic packet. Its summary is the
flat row for requested admission, effective admission, repair-first pressure,
queued work, repair task ids, and blockers after handoff-history health has
been applied.
Persist those rows with
`AgentReportGateHealthGateTrendHandoffMonitorSummaryHistoryRecorder` when eval
needs cross-cycle monitor health. The dashboard separates requested admission
from effective admission and tracks handoff-health mix, repair pressure, queued
work, and blockers without retaining nested gate payloads.
Apply `AgentReportGateHealthGateTrendHandoffMonitorGate` after recording that
history when monitor-level drift must control the next queue: stable/watch
health preserves current admission, while repair health adds
`eval-report-gate-health-gate-trend-handoff-monitor` repair work and blocks the
effective gate.
Use `AgentReportGateHealthGateTrendHandoffMonitorHandoff` when service/eval
wants the monitor-summary append and monitor-health gate applied together. Its
summary is the compact packet row for effective admission, repair-first state,
monitor-history depth, queued work, repair tasks, and blockers after the final
monitor gate.
Persist those rows in
`AgentReportGateHealthGateTrendHandoffMonitorHandoffSummaryHistory` and append
them with
`AgentReportGateHealthGateTrendHandoffMonitorHandoffSummaryHistoryRecorder`
when eval needs dashboard health over the final monitor-handoff packets. The
dashboard separates requested admission from effective admission and tracks
monitor-health mix, repair-first pressure, repair work, queued work, and
blockers before another memory-note, adaptive-state, or service command
boundary is allowed to open.
Apply `AgentReportGateHealthGateTrendHandoffMonitorHandoffGate` to that
history record when final monitor-handoff drift must control the next queue:
stable/watch histories preserve the current packet queue, while repair histories
append deterministic
`eval-report-gate-health-gate-trend-handoff-monitor-handoff` work and close
ordinary admission before side effects resume.
`AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoff` packages that
final-packet summary append and gate decision into one replayable service/eval
record. Use its summary as the compact eval row for final monitor-handoff
health, requested/effective admission, repair-first state, queue ids, repair ids,
and blocker pressure after the final packet trend gate has been applied.
Persist those summaries in
`AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistory` with
`AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistoryRecorder`
when eval needs a dashboard row over final packet admission rate, health mix,
repair-first pressure, repair task pressure, queue pressure, and blockers before
the next closed-loop service/eval boundary is opened.
Apply `AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffGate` to that
dashboard health before opening the next queue boundary: stable/watch histories
preserve the final packet queue, while repair histories append deterministic
`eval-report-gate-health-gate-trend-handoff-monitor-handoff-handoff` repair work
and keep side effects closed until the final packet trend is repaired.
`AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoff` composes
that final-packet trend append and trend gate into one replayable admission
record. Its summary gives eval and service adapters the final packet health,
requested/effective admission, repair-first state, queue ids, repair ids, and
blocker pressure without expanding nested monitor-handoff payloads.
For adapters that want one call after memory submission, `AgentClosedLoopStepper`
assembles the same pieces: it builds the `AgentCycleLedgerRecord`, evaluates the
report gate, plans loopback tasks, appends a ledger entry, and emits the final
`AgentBusinessLoopPlan`.
`AgentServiceCommandPlanner` is the final crate-owned handoff before real side
effects. It converts the business plan into command data; the service remains
responsible for executing adaptive-state writes, queue updates, dashboard
telemetry, or repair-mode transitions. `AgentServiceCommandPlan::summary`
is the compact pre-execution row for eval and operator UI, carrying command
kinds, command pressure, adaptive-write intent, repair/hold intent, enqueue
pressure, queued tasks, memory-promotion reason counts, tool-build reason
counts, and telemetry-command count.
Keep `AgentServiceCommandPlanSummaryHistory` when service/eval wants to inspect
that pre-execution command pressure before any writer runs. Its dashboard tracks
plan count, total commands, adaptive-write plans, repair/hold plans, enqueue
pressure, queued task pressure, memory-promotion and tool-build reason-family
runs, latest command kinds, and telemetry. Call
`AgentServiceCommandPlanSummaryHistory::health` with
`AgentServiceCommandPlanHealthPolicy` to turn an empty history into `Watch`,
repair/hold command pressure into `Repair`, and enqueue/adaptive-write policy
pressure into explicit service-local reasons. Use
`AgentServiceCommandPlanSummaryHistoryRecorder` immediately after command
planning when the service wants one append step that returns the updated
history, dashboard, health, and telemetry before executing service commands.
After execution, the service can return `AgentServiceCommandReceipt` values.
`AgentServiceCommandAudit` records missing, failed, and skipped commands as
blocked reasons so the next coordinator turn can repair execution drift without
pretending the side effects succeeded. `AgentServiceCommandAudit::summary`
is the compact post-receipt row for service logs and eval, carrying expected
command count, received receipt count, missing/failed/skipped counts,
cleanliness, and stable blocked reasons.
Keep `AgentServiceCommandAuditSummaryHistory` when service/eval needs a
post-receipt trend before converting dirty receipts into repair tasks. Its
dashboard tracks clean/dirty audit count, expected commands, receipts,
missing/failed/skipped drift events, blocked-reason pressure, latest blockers,
clean rate, and drift rate. Call
`AgentServiceCommandAuditSummaryHistory::health` with
`AgentServiceCommandAuditHealthPolicy` to turn empty history into `Watch`,
receipt drift or blocked-reason pressure into `Repair`, and low clean-rate
history into an explicit watch reason. Use
`AgentServiceCommandAuditSummaryHistoryRecorder` immediately after
`AgentServiceCommandPlanner::audit` when the persisted row should include the
updated history, dashboard, health, and telemetry before feedback tasks are
generated.
`AgentServiceFeedback` converts those audit blockers into concrete repair
`AgentTask` values and a next `AgentTaskQueue`, keeping command-execution drift
inside the same scheduler/reviewer workflow as model or memory failures.
Keep `AgentServiceFeedbackSummaryHistory` when service/eval needs to trend the
repair work generated from dirty command audits before it is merged into the
business queue. Its dashboard tracks clean/dirty feedback count, dirty-audit
count, repair-task pressure, feedback next-queue pressure, blocked-reason
pressure, latest blockers, clean rate, and repair-task rate. Call
`AgentServiceFeedbackSummaryHistory::health` with
`AgentServiceFeedbackHealthPolicy` to turn empty history into `Watch`, repair
task or blocked-reason pressure into `Repair`, and low clean-rate history into a
watch reason. Use `AgentServiceFeedbackSummaryHistoryRecorder` immediately after
`AgentServiceFeedback::summary` when the service wants one append step before
turnover merges repair work with the business queue.
`AgentServiceTurnover` then merges that feedback queue with the business plan's
ordinary next queue through `AgentTaskQueue::with_repair_first`, so planned
follow-ups depend on service-execution repair tasks instead of sharing the same
ready wave. The coordinator receives one queue, but command-execution repair
still runs before memory, adaptive-state, eval, or other business follow-ups.
Keep `AgentServiceTurnoverSummaryHistory` when service/eval needs the final
post-service queue trend before the next scheduler wave. Its dashboard tracks
clean/dirty turnover count, dirty feedback count, service repair-task pressure,
merged next-queue pressure, blocked-reason pressure, latest blockers, clean
rate, repair-task rate, and next-queue task rate. Call
`AgentServiceTurnoverSummaryHistory::health` with
`AgentServiceTurnoverHealthPolicy` to turn empty history into `Watch`, service
repair/blocker pressure into `Repair`, and low clean-rate or oversized
next-queue pressure into explicit watch reasons. Use
`AgentServiceTurnoverSummaryHistoryRecorder` after `AgentServiceTurnover::summary`
when the service wants to persist the exact queue handoff that the next
coordinator wave will consume.
`AgentServiceExecutionReport` is the one-call execution-side envelope: given a
business plan and service receipts, it contains the command plan, audit,
feedback, and turnover together. Use `AgentServiceFeedback::summary`,
`AgentServiceTurnover::summary`, and `AgentServiceExecutionReport::summary`
when service/eval needs repair-task counts, merged queue size, command drift,
pre-execution memory-promotion or tool-build reason-family counts,
cleanliness, and blocker evidence without expanding task or receipt payloads.
Keep `AgentServiceExecutionHistory` when service/eval needs trend evidence at
the receipt-close boundary before building the broader closed-loop row. Its
dashboard tracks clean rate, missing/failed/skipped command pressure, repair
tasks, queued work, memory-promotion and tool-build reason-family runs, latest
blockers, and telemetry. Call
`AgentServiceExecutionHistory::health` or
`AgentServiceExecutionDashboard::health` with
`AgentServiceExecutionHealthPolicy` when service/eval needs a service-local
stable/watch/repair receipt-close trend before the broader closed-loop
execution history is recomputed. Use `AgentServiceExecutionHistoryRecorder`
immediately after each
`AgentServiceExecutionReport::summary` when the service wants one append step
that returns both the updated history and dashboard. Use
`AgentServiceExecutionHistoryRecorder::record_summary_with_health` or
`record_report_with_health` when the persisted receipt-close row should also
carry `AgentServiceExecutionHealthRecord` status, reasons, and telemetry.
`AgentClosedLoopExecutionReport` combines the earlier `AgentClosedLoopStep`
with that service execution report, giving dashboards and eval a single object
for the full cycle from agent report through service receipts.
Call `AgentClosedLoopExecutionReport::service_summary` when service/eval wants
the compact receipt-close row from that same envelope without manually walking
the nested service report. The full-cycle summary reuses those service counters
so command drift, queue pressure, and blocker counts stay aligned across both
rows.
`AgentClosedLoopExecutionSummary` is the compact eval/dashboard row derived
from that report: clean flags, reward/admission status, command counts, next
queue counts, task ids, and blocked reasons.
`AgentClosedLoopExecutionHistory` stores those rows in run order and derives an
`AgentClosedLoopExecutionDashboard` with clean rate, report blockers, loopback
blockers, service command pressure, admission counts, queued repair volume,
average reward, and latest blockers. This gives the six-window coordinator a
trend view without asking consumers to inspect nested execution reports.
`AgentClosedLoopExecutionHealth` evaluates that dashboard with a policy and
returns `Stable`, `Watch`, or `Repair` plus stable reason strings. The service
can use this as a pure preflight gate before continuing adaptive-state
promotion. Its `allows_service_advance` and `requires_repair_first` helpers
let service/eval preserve stable/watch observation while forcing repair-level
closed-loop execution drift through scheduler repair before another boundary
opens.
`AgentClosedLoopNextTurnPlan` combines execution history health with the next
queue and returns a `Continue`, `Observe`, `Repair`, or `Idle` mode. It tells
the service whether scheduling can continue, whether adaptive evolution remains
open, whether service/eval may advance past the read-model boundary, and which
telemetry/reasons should be attached to the next turn.
`AgentClosedLoopDispatchPreparer` is the pure bridge from that next-turn plan to
`AgentCycleOrchestrator::plan_next_wave`: it skips idle turns, preserves
budget-rejection dispatch audits, and returns assigned task ids without running
any engine calls.
`AgentClosedLoopPreparedExecutor` then runs only dispatchable prepared waves
through `AgentWaveExecutor` and an `EnginePort`, preserving skipped turns and
engine failures as data instead of fabricating successful results.
`AgentClosedLoopPreparedCycleCloser` closes only executed waves into
`AgentCycleReport` values, while skipped or missing executions remain explicit
skipped reasons and do not fabricate reports.
`AgentClosedLoopRuntimeTurnRunner` composes next-turn planning, dispatch
preparation, guarded execution, and prepared-cycle closing into the single
service-facing call for a runtime turn. It still delegates all actual model or
runtime work through `EnginePort`.
`AgentClosedLoopRuntimeBusinessTurnCloser` continues from that runtime envelope:
it derives `AgentCycleHandoff`, submits memory notes through `MemoryPort`, and
invokes `AgentClosedLoopStepper` so accepted, blocked, and memory-failed turns
all reach the same business-loop admission path.
`AgentClosedLoopRuntimeServiceRequestRunner` is the composed pre-side-effect
boundary for service adapters. It runs the guarded runtime turn, submits allowed
memory handoffs, closes the business loop, and emits an
`AgentClosedLoopRuntimeServiceCommandRequest` plus the prior execution history.
The service can execute the command plan later and still have the exact history
needed for receipt audit.
`AgentClosedLoopRuntimeServiceCommandPlanner` turns the resulting business plan
into explicit service command data and stable command-request telemetry without
executing any side effects.
`AgentClosedLoopRuntimeServiceCommandGate` is the pre-execution guard for that
command request. It maps each command to a side-effect kind, records whether it
is allowed, and emits deterministic blockers such as an unadmitted adaptive
state promotion or an empty enqueue/telemetry command. The service should treat
blocked gate entries as a stop condition before attempting the command.
`AgentClosedLoopRuntimeServiceDispatch` is the pre-execution envelope the outer
service can hand to its command executor. It exposes a command plan only when
the request has commands and every gate entry is allowed; otherwise it carries
the blocked reasons and telemetry without letting the executor treat the plan as
ready.
`AgentClosedLoopRuntimeServiceDispatchSummary` is the compact pre-execution row
for service executors and eval traces. It records whether the dispatch is
executable, whether the command gate allowed execution, how many side-effect
gate entries were evaluated, how many were blocked, how many command
occurrences are expected, the command kinds in stable order, and any gate
blockers that must stop side effects.
Keep `AgentClosedLoopRuntimeServiceDispatchSummaryHistory` when service/eval
needs a runtime dispatch trend before executing service commands. Its dashboard
tracks executable vs blocked dispatches, command-gate admission, command
pressure, side-effect gate pressure, blocked side-effect gate pressure, latest
command kinds, latest blockers, executable rate, and blocked-gate rate. Call
`AgentClosedLoopRuntimeServiceDispatchSummaryHistory::health` with
`AgentClosedLoopRuntimeServiceDispatchHealthPolicy` to turn empty history into
`Watch`, blocked dispatch or side-effect gate pressure into `Repair`, and low
executable-rate history into an explicit watch reason. Use
`AgentClosedLoopRuntimeServiceDispatchSummaryHistoryRecorder` immediately after
`AgentClosedLoopRuntimeServiceDispatch::summary` when a service adapter wants a
pre-execution trend record before command receipts exist.
Its health and history record expose `allows_service_advance` and
`requires_repair_first` so pre-execution gate trends can stop service-owned
executors before receipts or outcome closure are even possible.
After the executor returns receipts, `AgentClosedLoopRuntimeServiceReceiptIntake`
checks that the dispatch was executable and that every receipt corresponds to
an expected command occurrence. Receipts for blocked dispatches, unexpected
commands, or duplicate command receipts are rejected before outcome closure.
Use `AgentClosedLoopRuntimeServiceReceiptIntake::summary` when service/eval
needs a compact row for executable status, expected/accepted/rejected receipt
counts, cleanliness, and blocked reasons without expanding receipt payloads.
Keep `AgentClosedLoopRuntimeServiceReceiptIntakeSummaryHistory` when the adapter
needs post-executor receipt-intake trends before an outcome is allowed to close.
Its dashboard tracks clean vs dirty intakes, executable vs non-executable
intakes, expected/accepted/rejected receipt pressure, blocked-reason pressure,
latest blockers, clean rate, and rejection rate. Call
`AgentClosedLoopRuntimeServiceReceiptIntakeSummaryHistory::health` with
`AgentClosedLoopRuntimeServiceReceiptIntakeHealthPolicy` to turn empty history
into `Watch`, non-executable intake or rejected receipts into `Repair`, and low
clean-rate history into an explicit watch reason.
Its health and history record expose `allows_service_advance` and
`requires_repair_first`, matching continuation history: watch receipt trends can
be observed, but repair receipt drift blocks service advance until intake repair
tasks run first.
`AgentClosedLoopRuntimeServiceDispatchOutcome` is the intake-aware close result:
it contains an `AgentClosedLoopRuntimeServiceOutcome` only when the dispatch was
executable and receipt intake was clean.
When intake is blocked, `AgentClosedLoopRuntimeServiceIntakeRepairPlan` converts
the intake blockers into reviewer, planner, or aggregator tasks on the
`service-intake` lane so executor drift returns to the same multi-agent repair
loop as report-gate and service-command audit failures.
`AgentClosedLoopRuntimeServiceDispatchContinuationPlanner` closes that branch:
when the dispatch outcome contains a clean closed outcome, it reuses the
outcome's continuation; when receipt intake is blocked, it preserves the prior
history, installs the `service-intake` repair queue as the next queue, and
passes through the caller's budgets, policies, evidence, and max parallelism.
`AgentClosedLoopRuntimeServiceDispatchContinuationSummary` is the compact row
for that branch. It lets service and eval surfaces see whether the outcome
closed, whether intake was clean, how many repair tasks were produced, the
resulting health status, the next queue size, and the history length without
opening the nested dispatch outcome.
`AgentClosedLoopRuntimeServiceDispatchContinuationSummaryHistory` records those
rows in order, exposes closed and intake-clean rates, and applies a continuation
health policy. Empty history starts at watch, clean closed continuations are
stable, and blocked or repair-heavy continuation history moves to repair before
the service adapter advances more work.
Its health and history record expose `allows_service_advance` and
`requires_repair_first` so service/eval adapters can treat watch trends as
observable but still stop immediately on repair continuation pressure.
`AgentClosedLoopRuntimeServiceRunner` is the higher-level receipt-aware helper:
it runs the request through `AgentClosedLoopRuntimeServiceRequestRunner`,
converts it to a gated dispatch, consumes caller-provided command receipts,
applies intake-aware closure, and returns the dispatch continuation plus its
summary. It still does not execute service commands; the caller owns those side
effects and supplies receipts.
`AgentClosedLoopRuntimeServiceRunSummary` is the one-row service/eval projection
for that composed helper. It combines dispatch executable status, command kinds,
command-gate status, side-effect gate counts, gate blockers, outcome closure,
intake blockers, repair task count, health, next queue size, and history
length.
Its `AgentClosedLoopRuntimeServiceRunStatus` is `Closed` only when the outcome
closed and receipt intake was clean. A non-executable dispatch becomes
`DispatchBlocked`; an executable dispatch with rejected receipts becomes
`IntakeBlocked`.
`AgentClosedLoopRuntimeServiceRunHistory` is the attempt-level companion to
`AgentClosedLoopExecutionHistory`. It can record closed runs, dispatch-gate
blocks, and receipt-intake drift without pretending every attempt produced a
service execution summary. Its dashboard reports closed/blocked counts, rates,
command pressure, repair task volume, next-queue volume, latest status, and
latest blockers.
`AgentClosedLoopRuntimeServiceRunHistoryRecorder` is the append helper for that
attempt log. Given prior service-run history and a completed
`AgentClosedLoopRuntimeServiceRun`, it records the run summary, recomputes the
dashboard, evaluates service-run health, and emits telemetry. This keeps
service-run attempts visible even when dispatch was blocked or receipt intake
failed before execution history could be appended.
`AgentClosedLoopRuntimeServiceRunHealth` evaluates that dashboard with
`AgentClosedLoopRuntimeServiceRunHealthPolicy`. Empty attempt history and low
closed rate are watch signals; intake blockers and repair-task pressure are
repair signals. This is an attempt-level preflight health check, separate from
`AgentClosedLoopExecutionHealth` over successfully appended execution summaries.
Read `allows_service_advance` and `requires_repair_first` from service-run
health or its history record when the adapter needs a single attempt-level gate:
watch keeps the loop observable, repair routes the next turn through repair
first.
`AgentClosedLoopRuntimeServicePreflight` is the convergence point for those two
health views. It preserves idle turns, escalates either execution-health or
service-run-health repair to `Repair`, downgrades clean execution with watched
service attempts to `Observe`, and allows adaptive evolution only when both
health views are stable. Its `side_effect_admission` method turns that decision
into an `AgentCollaborationAdapterSideEffectAdmission`: observe keeps service
dispatch schedulable but blocks memory-note and adaptive-state promotion,
repair closes all three side-effect gates, and continue opens them when both
health views are stable.
`AgentClosedLoopRuntimeServicePreflightFollowUpPlan` turns non-clean preflight
reasons into scheduler-visible tasks. Observe tasks ask tester/reviewer/planner
roles for more evidence, while repair tasks use higher priority so service-gate
or receipt-intake drift is handled before ordinary next-turn work.
`AgentClosedLoopRuntimeServicePreflightContinuationPlanner` is the bridge from
preflight planning back into runtime scheduling. It preserves the execution
history from the preflight turn plan, uses the follow-up merged queue as the
next queue, and carries caller-provided budgets, policies, completed ids,
evidence, and max parallelism into `AgentClosedLoopRuntimeTurnInput`.
`AgentClosedLoopRuntimeServiceLoopStatePlanner` wraps the same preflight and
continuation path into a service-held snapshot. The snapshot keeps the
execution history, service-run attempt history, next runtime input, preflight
mode, follow-up counts, and telemetry together so norion-service or norion-eval
can persist or display the closed-loop state without reconstructing it from
several reports. Its `AgentClosedLoopRuntimeServiceLoopStateSummary` is the
dashboard row for that snapshot: it preserves the continue/observe/repair/idle
mode, execution and service-run health statuses, schedule/adaptive booleans,
repair-first flag, history counts, follow-up task count, next queue count, and
preflight reasons. It also carries flattened side-effect admission health,
dispatch admission, memory-note admission, adaptive-state admission, and
admission reason counts so eval can compare schedulable service dispatch with
memory or adaptive-state writes.
`AgentClosedLoopRuntimeServiceLoopAdvancePlanner` is the composed service-run to
loop-state bridge. It records the completed `AgentClosedLoopRuntimeServiceRun`
into attempt history, preserves the run's next runtime input budgets, policies,
evidence, completed ids, and max parallelism, then produces the next loop-state
snapshot and summary. Intake drift stays attempt-level and repair-first; it is
not fabricated into execution history.
`AgentClosedLoopRuntimeServiceLoopRunner` is the service-facing shortcut for
that bridge. Given a receipt-aware service-run input, prior service-run history,
and a service-run health policy, it runs the existing service-run path through
`EnginePort` and `MemoryPort`, records the attempt, advances preflight, and
returns the next loop-state snapshot plus telemetry. It still does not own
external command execution; receipts remain caller-provided.
`AgentClosedLoopRuntimeServiceLoopRunSummary` is the compact row for that
runner. It keeps the service-run status and command counters beside the next
loop mode, execution/service health statuses, schedule flags, attempt count,
blocked reasons, preflight reasons, follow-up task count, next queue size, and
the command-gate/side-effect gate evidence from the service run plus flattened
side-effect admission gates inherited from loop-state preflight.
`AgentClosedLoopRuntimeServiceLoopRunHistory` is the multi-transition companion
to that row. Its dashboard rolls service-loop runs into closed, dispatch-blocked,
intake-blocked, repair-first, side-effect-dispatch-allowed,
memory-note-allowed, adaptive-allowed, command-gate-allowed, side-effect-gate,
command, follow-up, next-queue, latest status, latest mode, and latest reason
counters for service/eval views.
`AgentClosedLoopRuntimeServiceLoopRunHealth` evaluates that transition-level
dashboard with `AgentClosedLoopRuntimeServiceLoopRunHealthPolicy`. Empty
transition history watches, intake-blocked or repair-first pressure repairs,
and latest transition blockers stay attached as deterministic audit reasons.
`AgentClosedLoopRuntimeServiceLoopRunHistoryRecorder` is the append helper for
that layer: after a service adapter gets an
`AgentClosedLoopRuntimeServiceLoopRun`, it can record the compact transition row
and receive dashboard, health, and telemetry without hand-assembling them.
`AgentClosedLoopRuntimeServiceLoopRunControlPlan` is the service-facing
transition gate. It combines loop-run health with the next queue and returns
the daemon-loop mode, whether another transition can schedule, whether adaptive
evolution remains open, whether repair must run first, and stable telemetry.
`AgentClosedLoopRuntimeServiceLoopRunMonitor` composes the history recorder and
controller. Given prior transition history and a completed loop run, it appends
the compact row, recomputes transition health, uses the run's next queue for
control planning, and returns one telemetry bundle for service/eval storage.
`AgentClosedLoopRuntimeServiceLoopRunControlSummary` is the flat projection of
that monitor record for dashboards, service logs, or eval tables that should not
expand the full transition history, health dashboard, and next queue payload.
`AgentClosedLoopRuntimeServiceLoopRunControlSummaryHistory` is the trend view
for those flat rows. Its dashboard counts continue/observe/repair/idle records,
schedulable records, dispatch/memory/adaptive admission rates, repair-first
rows, next-queue pressure, and latest mode/health/reasons for operator UI and
eval regression gates.
`AgentClosedLoopRuntimeServiceLoopRunControlHealth` evaluates that dashboard
with `AgentClosedLoopRuntimeServiceLoopRunControlHealthPolicy`. Empty
daemon-control history watches, low schedule/adaptive rates watch, and repair or
repair-first pressure escalates to repair while preserving latest reasons.
`AgentClosedLoopRuntimeServiceLoopRunControlSummaryHistoryRecorder` is the
append helper for flat control rows. Given prior control-summary history and a
new `AgentClosedLoopRuntimeServiceLoopRunControlSummary`, it recomputes the
dashboard and daemon-control health without requiring service/eval callers to
hand-assemble trend state.
`AgentClosedLoopRuntimeServiceLoopRunDaemonRunner` is the composed
daemon-transition boundary. It runs one `AgentClosedLoopRuntimeServiceLoopRun`,
records and plans it through `AgentClosedLoopRuntimeServiceLoopRunMonitor`,
appends the flat control row through
`AgentClosedLoopRuntimeServiceLoopRunControlSummaryHistoryRecorder`, and returns
the loop run, monitor record, compact control summary, control-summary history
record, and telemetry together. The input carries prior transition history,
prior flat control history, and both health policies, while receipts still come
from the service-owned side-effect executor.
`AgentClosedLoopRuntimeServiceLoopRunDaemonContinuation` is the next-state
snapshot extracted from that record. It carries the next runtime input,
service-run history, transition history, flat daemon-control history, the three
health policies, admission booleans, dispatch/memory admission rates,
transition health, control health, and telemetry so service/eval can persist or
schedule the next daemon turn without walking the nested record graph.
`AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlanner` is the pre-receipt
assembly step for service daemons. It turns a continuation plus a fresh
`AgentClosedLoopRuntimeBusinessInput` into an
`AgentClosedLoopRuntimeServiceRequestInput`, default continuation input, daemon
histories, policies, admission flags, dispatch/memory admission rates, and
telemetry. This gives the service a stable command-request boundary before it
owns side effects and receipts.
`AgentClosedLoopRuntimeServiceLoopRunDaemonRequestRunner` runs that plan only up
to the command-request/dispatch boundary. It returns the dispatch, dispatch
summary, expected command plan, skipped reasons, and telemetry while leaving
service command side effects to the caller. `AgentClosedLoopRuntimeServiceLoopRunDaemonReceiptCloser`
then accepts receipts for that already-run boundary and closes the daemon
transition without running the runtime engine or memory submission a second
time.
`AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummary` is the flat
pre-receipt row for that boundary: executable flag, command count/kinds, daemon
mode/admission flags, command-gate status, side-effect gate counts,
dispatch/memory/adaptive admission rates, history counters, blocked reasons,
skipped reasons, and telemetry. Its summary history, dashboard, health policy,
health record, and recorder let service/eval detect whether the daemon is still
producing expected commands before receipts or side-effect results exist,
including command-gate allowed rate and side-effect gate pressure.
The request health and summary history record expose `records()`,
`allows_service_advance`, and `requires_repair_first` before the monitored
close boundary consumes receipts.
`AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredReceiptCloser`
combines the pre-receipt and post-receipt records. It appends the request
summary, evaluates request-boundary health, closes receipts into the daemon
record, and returns one telemetry bundle with request health plus daemon-control
health. This is the preferred service/eval handoff when both the expected
command boundary and the eventual receipts must be persisted together.
`AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummary` is the
compact row for that monitored close. It carries latest request executable
status, command count, request command-gate status, request side-effect gate
counts, request health, daemon run status, daemon-control health, mode,
scheduler/adaptive admission flags, dispatch/memory admission-rate evidence,
accumulated history counters, and merged blockers/skips without requiring
service/eval to expand the nested close record.
`AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummaryHistory`
and its dashboard aggregate those rows across daemon request/receipt closures.
The associated health policy, health record, and recorder classify request
executable rate, daemon closed rate, dispatch-admission rate, memory-note
admission rate, adaptive-admission rate, request command-gate allowed rate,
request side-effect gate pressure, request repair pressure, daemon-control
repair pressure, and repair-first pressure before the service opens another
self-evolution boundary.
The monitored-close health and summary history record expose `records()`,
`allows_service_advance`, and `requires_repair_first`, preserving watch rows
while blocking repair-level close drift before the next daemon request.
`AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseContinuation`
packages the result after that trend row is recorded. It keeps the ordinary
monitored continuation, monitored-close summary history, monitored-close health,
request health, daemon-control health, next mode, dispatch/memory/adaptive
admission evidence, and telemetry in one persisted state object for the next
daemon request boundary.
`AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredClosePlanner` and
`AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseRunner` carry
that close-aware state into the next request boundary. They reuse the ordinary
monitored planner/runner but preserve monitored-close summary history and
monitored-close health beside the new request plan, so the next receipt close
can append another trend row without reconstructing cross-turn state in the
service layer.
`AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseRunSummary`
is the compact audit row for that close-aware request boundary: executable
status, command pressure, command-gate status, side-effect gate counts,
request/daemon-control/monitored-close health, history counters, admission
flags, blockers, skips, and telemetry.
`AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredContinuation`
extracts the next persisted state from that monitored close. It keeps the
daemon continuation, request-summary history, request-boundary health,
daemon-control health, mode, scheduler/adaptive admission flags, and telemetry
together so the next service boundary does not lose the pre-receipt ledger.
`AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredPlanner` then turns
that monitored continuation directly into the next request plan while carrying
the request-summary history beside it for the following monitored close.
`AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredRunner` executes
that monitored request plan only up to the expected-command boundary and returns
`AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredRecord`, keeping the
request-summary history attached until receipts are ready to close.
`AgentClosedLoopRuntimeServiceLoopRunDaemonInputPlanner` turns that continuation
back into the next `AgentClosedLoopRuntimeServiceLoopRunDaemonInput` once the
service has a fresh business input and command receipts. It preserves the
persisted histories and policies, derives the default continuation input from
the next runtime input, exposes dispatch/memory admission rates on the input
plan, emits compact telemetry, and still lets runtime queue, budget,
completed-task, and side-effect gates block the next transition.
`AgentClosedLoopRuntimeServiceTurnCloser` then attaches service command receipts
to the step, builds `AgentClosedLoopExecutionReport`, stores its summary in an
updated history, and exposes the merged next queue for the following scheduler
turn.
`AgentClosedLoopRuntimeContinuationPlanner` packages the updated history,
dashboard, health, next queue, budgets, policies, and evidence into the next
runtime input so the service loop can continue without rebuilding the turn
state by hand.
`AgentClosedLoopRuntimeServiceOutcomePlanner` closes the service command
request, receipts, and continuation input in one data-only call. It is the
preferred handoff when a service executor returns receipts asynchronously after
the request boundary.
`AgentClosedLoopRuntimeServiceOutcomeSummary` is the compact row for dashboards
that do not need nested report trees: it keeps the runtime mode, command count,
command-gate status, side-effect gate counts, blocked command-gate reasons,
whether service execution happened, whether receipts were clean, health status,
next queue size, skipped reasons, and telemetry together.

## Integration points

`EnginePort` is the future bridge to `norion-core` or the model service. It should receive an `AgentTask` and return an `AgentResult` containing structured messages, not raw chat text.
`AgentWaveExecutor` can run the accepted tasks in dispatch order through that
port and returns `AgentWaveExecution`; engine errors stay as
`AgentExecutionFailure` records and are not converted into successful results.
Use `AgentCycleOrchestrator::close_execution` when closing a wave from
`AgentWaveExecution`; execution failures lower reward validation, appear in
`AgentCycleSummary`, and block memory promotion.

`MemoryPort` is the future bridge to `norion-memory`. Agents may recall memory and propose `MemoryNote` values, but they should not write persistent memory directly. The main service should validate notes after reflection and process reward gates pass.
`MemoryHandoffSubmitter` is the optional service-side helper for that final
proposal step. It submits only unblocked handoff notes, never bypasses
`AgentCycleHandoff::blocked_reasons`, and records `MemoryPort` errors in
`MemorySubmissionReport`. Call `MemorySubmissionReport::summary` when eval only
needs submitted, failed, blocked, attempted, clean, and port-attempt counters.
Call `MemorySubmissionReport::gate` before advancing the loop: blocked handoffs
and port failures require repair-first, while clean reports can continue and
may commit submitted notes if any were accepted.

The service loop can use this order:

1. Build tasks for active windows or agents.
2. Ask `AgentCycleOrchestrator` for the next ready dispatch wave with
   `BudgetLedger`.
3. Run accepted tasks through `AgentWaveExecutor` over an `EnginePort` adapter.
4. Run `ReflectionLoop` for accepted conclusions.
5. Attach any coordinator-approved `ConflictResolutionBook`.
6. Close the wave with `AgentCycleOrchestrator::close_execution`.
7. Submit `MemoryPromotion.note` values with `MemoryHandoffSubmitter` only after
   the main writer accepts them and the relevant `SideEffectGate` allows them.
8. Build an `AgentCycleLedgerRecord` from the summary, submission report, and
   validation/runtime evidence refs, then pass it through `AgentReportGate`
   before the business loop promotes adaptive state.
9. Use `AgentLoopbackPlanner` to merge gate repairs and handoff follow-ups into
   the next `AgentTaskQueue`.
10. Append the record, report decision, and loopback plan to `AgentCycleLedger`
    so service/eval can make trend-aware promote/hold/repair decisions.
11. Ask `AgentBusinessLoopController` for the final control plan before the
    outer service applies adaptive state or schedules the next wave.

Steps 8 through 11 can also be performed with `AgentClosedLoopStepper::close`
once the service has a cycle report, handoff, validation/runtime evidence, and
optional memory submission report.
After that, `AgentServiceCommandPlanner::plan` can turn the returned
`AgentBusinessLoopPlan` into command rows for the outer service executor.
`AgentServiceCommandPlanner::audit` can then verify the executor receipts before
the main window treats a command plan as applied.
If the audit is not clean, `AgentServiceFeedback::from_audit` should be used to
enqueue service-execution repair work for the next agent wave.
Use `AgentServiceTurnover::from_feedback` to combine those repair tasks with any
already-planned follow-up tasks before the next call to the scheduler.
`AgentServiceCommandPlanner::close_execution` performs the command-plan,
receipt-audit, feedback, and turnover steps in one pure-data call.
If a caller already holds an `AgentClosedLoopStep`,
`AgentClosedLoopStep::close_service_execution` attaches service receipts and
returns the full closed-loop execution envelope.
Use `AgentClosedLoopExecutionReport::service_summary` when the caller needs the
post-service receipt row from that envelope before writing the broader
full-cycle row.
Use `AgentClosedLoopExecutionReport::summary` when the caller only needs stable
counters and blockers instead of the full nested report tree.
Append those rows to `AgentClosedLoopExecutionHistory` when the service wants a
multi-run dashboard, then call `dashboard()` before sending trend metrics to
eval or operator UI surfaces.
Call `health(AgentClosedLoopExecutionHealthPolicy::default())` when the service
needs a small status decision: `Stable` can continue normal scheduling,
`Watch` should gather more evidence or keep humans in the loop, and `Repair`
should prioritize generated repair tasks before more self-evolution.
For service-local command receipts, pass `AgentServiceExecutionHealth` through
`AgentServiceExecutionHealthGate` with the current next queue before opening the
next command boundary. The gate keeps the original queue intact, exposes
admission booleans for service/eval, and merges repair tasks only when the
receipt-close trend is repair-level.
When the service wants that append and gate as one persistence packet, use
`AgentServiceExecutionHistoryRecorder::record_*_with_health_gate`; it returns
the updated service execution history, the full gate decision, and the compact
gate summary that eval dashboards can store.
Append those compact summaries to `AgentServiceExecutionHealthGateHistory` when
the service needs cross-turn admission and repair-first pressure without
persisting full task payloads.
Use `AgentServiceExecutionHealthGateHistoryRecorder` when that append should
also return dashboard telemetry and `AgentServiceExecutionHealthGateHealth` for
the next boundary.
Pass that trend health through `AgentServiceExecutionHealthGateTrendGate` when
repair-first gate pressure should block the next command boundary and enqueue
`service-execution-health-gate` repair tasks.
Use `AgentServiceExecutionHealthGateTrendHandoff` when the service wants that
append-and-gate sequence as one replayable row for the next scheduler handoff.
Persist `AgentServiceExecutionHealthGateTrendHandoffSummary` rows when eval only
needs rates, statuses, blocker lists, and stable task ids instead of full queue
payloads.
Use `AgentClosedLoopNextTurnPlan` when the service needs one final data packet
before calling the scheduler: `Continue` may keep adaptive evolution open,
`Observe` may schedule evidence-gathering work while closing adaptive evolution,
`Repair` schedules repair work first, and `Idle` avoids dispatch when the queue
is empty.
Then pass the plan through `AgentClosedLoopDispatchPreparer` with the current
completed-task set, `BudgetLedger`, `BudgetPolicy`, and max parallelism. The
preparer returns either a skipped reason or an `AgentCycleDispatch` ready for
the existing `AgentWaveExecutor` path.
Use `AgentClosedLoopPreparedExecutor` when the service wants one guard around
that executor: non-dispatchable turns do not call `EnginePort`, while real
engine errors remain `AgentWaveExecution.failures` for the normal cycle close
and report-gate path.
After execution, `AgentClosedLoopPreparedCycleCloser` can hand the
`AgentWaveExecution` plus validation/reflection evidence back to
`AgentCycleOrchestrator::close_execution`. This keeps skipped turns out of the
report ledger and keeps executed turns on the same reward, memory, and report
gate path as ordinary waves.
When the service wants that entire sequence as one operation, use
`AgentClosedLoopRuntimeTurnRunner::run`: it returns the optional report,
turn-mode telemetry, runtime result/failure counters, and skipped reasons
without applying memory, adaptive-state, filesystem, or service commands.
Then use `AgentClosedLoopRuntimeBusinessTurnCloser` when a runtime report should
continue into memory handoff and business-loop planning. If the runtime turn was
skipped, the closer does not call `MemoryPort` and does not fabricate a
closed-loop step.
Before executing side effects, use `AgentClosedLoopRuntimeServiceCommandPlanner`
to obtain the service command request. The service executes those commands under
its own writer policy and returns receipts.
Call `AgentClosedLoopRuntimeServiceCommandRequest::gate` or
`AgentClosedLoopRuntimeServiceRequest::command_gate` before applying the command
plan. The gate is the crate-owned proof that adaptive-state writes were admitted
by the business loop and that queue/telemetry commands are well formed.
When the executor wants one envelope, convert the request with
`AgentClosedLoopRuntimeServiceRequest::into_dispatch` or call
`AgentClosedLoopRuntimeServiceRequestRunner::run_dispatch`. Only use
`AgentClosedLoopRuntimeServiceDispatch::command_plan` for command execution; it
returns `None` when the request is skipped or the gate is blocked.
Use `AgentClosedLoopRuntimeServiceDispatch::summary` before execution when the
service needs a compact row for logs, operator UI, or eval. The summary is also
the easiest place to prove that blocked dispatches never reached the executor,
including the side-effect gate count and blocked-gate count.
After the executor returns receipts, prefer
`AgentClosedLoopRuntimeServiceDispatch::close_with_intake`. It rejects receipts
for blocked dispatches and unexpected command kinds before deciding whether an
outcome may be closed. If the intake is blocked, the service should surface the
intake blockers as repair work instead of appending a service execution summary
to history.
Use `AgentClosedLoopRuntimeServiceDispatchOutcome::repair_queue` to enqueue the
generated `service-intake` repair tasks. The queue is empty when the dispatch
closed into a clean outcome.
Use `AgentClosedLoopRuntimeServiceDispatchContinuationPlanner` after
`close_with_intake` when the service wants the exact next
`AgentClosedLoopRuntimeTurnInput`: closed outcomes continue from updated
history, while blocked intake continues from prior history with repair tasks.
Use `AgentClosedLoopRuntimeServiceDispatchContinuation::summary` when the
service needs a compact dispatch-continuation row for logs, dashboards, or eval
without expanding the nested outcome.
Use `AgentClosedLoopRuntimeServiceDispatchContinuationSummaryHistoryRecorder`
when service/eval needs an ordered continuation trend: it appends the compact
row, emits dashboard telemetry, and marks low closure rate, dirty intake, or
repair pressure before the next service turn is admitted.
Read `allows_service_advance` and `requires_repair_first` from the continuation
history health or record when wiring admission gates: watch is observable,
repair is blocking.
Use `AgentClosedLoopRuntimeServiceRunner` when the service already has command
receipts from its own executor and wants the composed request-dispatch-intake-
continuation path in one crate call. The runner consumes receipts but still
keeps all side effects outside `norion-agent`.
Use `AgentClosedLoopRuntimeServiceRun::run_summary` when service/eval needs one
stable row spanning the pre-execution gate and post-execution intake result.
Treat `DispatchBlocked` as a side-effect preflight stop and `IntakeBlocked` as
executor drift that should schedule intake repair work rather than append a
successful service execution history row.
Keep `AgentClosedLoopRuntimeServiceRunHistory` beside the normal execution
history when dashboards need to include attempts that stopped before execution
history could be appended.
Use `AgentClosedLoopRuntimeServiceRunHistoryRecorder` immediately after
`AgentClosedLoopRuntimeServiceRunner` when the service wants to update
attempt-level history, dashboard, and service-run health from one completed
run.
Call `AgentClosedLoopRuntimeServiceRunHistory::health` when service/eval needs
to decide whether side-effect execution should continue, be observed, or
prioritize receipt/gate repair before the next runtime turn.
Use `AgentClosedLoopRuntimeServicePreflightPlanner` when the service wants one
pure-data call from execution history, service-run history, next queue, and both
health policies to the final next-turn preflight mode.
Use `AgentAdapterBoundarySummaryHistoryRecorder::record_runtime_service_preflight_*`
when eval/service wants the same preflight mode recorded as an adapter-boundary
health row before the next runtime input is scheduled; use the continuation
variant when the row should carry the follow-up merged queue rather than the
original next-turn queue.
Call `AgentClosedLoopRuntimeServicePreflight::follow_up_plan` before dispatch
when observed or repaired preflight reasons should be converted into
`service-preflight` lane tasks and merged with the ordinary next queue.
Use `AgentClosedLoopRuntimeServicePreflightContinuationPlanner` when the service
wants that merged queue converted directly into the next runtime input without
hand-assembling budgets, evidence, and scheduler policies.
Use `AgentClosedLoopRuntimeServiceLoopStatePlanner` when the service wants the
same preflight continuation plus execution-history and service-run-history
snapshots in one object. This is the preferred boundary for a service-owned loop
state table or eval trace that needs to resume the next runtime input exactly.
Use `AgentClosedLoopRuntimeServiceLoopState::summary` when dashboards need the
compact loop-state row without expanding execution history, service-run history,
or queued task payloads. The row includes preflight side-effect admission fields
so a dashboard can show when dispatch remains schedulable while memory-note or
adaptive-state admission is still blocked.
Use `AgentClosedLoopRuntimeServiceLoopAdvancePlanner` when the service has just
completed a `AgentClosedLoopRuntimeServiceRun` and wants the updated attempt
history plus the next loop-state snapshot in one call. This is the shortest
pure-data path from receipts to the next runtime turn while preserving budget
and policy fields from the run's next runtime input.
Use `AgentClosedLoopRuntimeServiceLoopRunner` when the service adapter wants the
receipt-aware service run and loop-state advance in one transition. It is useful
for daemon-style loops that already hold prior service-run history and want the
next `AgentClosedLoopRuntimeTurnInput` plus attempt-level dashboard/health data.
Use `AgentClosedLoopRuntimeServiceLoopRun::compact_summary` when service logs or
eval dashboards need one row for the whole service-loop transition without
expanding the run, advance, and loop-state objects.
Keep `AgentClosedLoopRuntimeServiceLoopRunHistory` beside service-run attempt
history when dashboards need trend evidence at the full transition level rather
than just the command receipt boundary.
Call `AgentClosedLoopRuntimeServiceLoopRunHistory::health` when service/eval
needs a single transition-level stable/watch/repair decision before allowing
another daemon-style service-loop turn.
Use `AgentClosedLoopRuntimeServiceLoopRunHistoryRecorder` immediately after a
completed `AgentClosedLoopRuntimeServiceLoopRun` when the service wants the
updated transition history and health record in one pure-data call.
Use `AgentClosedLoopRuntimeServiceLoopRunController::plan` when a daemon-style
service loop needs to decide whether to run the next transition, observe, repair
first, or idle from persisted transition history and the queued follow-up work.
Use `AgentClosedLoopRuntimeServiceLoopRunMonitor::record_and_plan` when the
service has just completed a loop run and wants the persisted transition record
plus next daemon-loop admission without calling recorder and controller
separately.
Use `AgentClosedLoopRuntimeServiceLoopRunControlRecord::summary` when service
logs or eval only need the compact row for latest transition status, next mode,
health status, transition rates, scheduler/adaptive admission, queue pressure,
and reasons.
Keep `AgentClosedLoopRuntimeServiceLoopRunControlSummaryHistory` beside the
transition history when service/eval needs daemon-control trends without
expanding the full loop-run, monitor, or queue payloads.
Call `AgentClosedLoopRuntimeServiceLoopRunControlSummaryHistory::health` when
the service wants one stable/watch/repair decision from daemon-control trends
before scheduling the next self-evolution transition.
Use `AgentClosedLoopRuntimeServiceLoopRunControlSummaryHistoryRecorder` after
each monitor summary is emitted when the service wants one append-and-health
record for operator dashboards, eval rows, or persisted daemon-control ledgers.
Use `AgentClosedLoopRuntimeServiceLoopRunDaemonRunner` when the service already
has command receipts and wants one data-only call that runs the transition,
updates transition history, computes next daemon-loop admission, appends the
flat daemon-control ledger row, and returns all telemetry needed by service/eval.
Use `AgentClosedLoopRuntimeServiceLoopRunDaemonContinuationPlanner` after a
daemon record when the service wants the compact persisted state for the next
turn: next runtime input, all daemon histories, policies, admission flags,
dispatch/memory admission rates, and health statuses.
Use `AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlanner` when the service
needs the pre-side-effect request boundary first. Its request plan can later be
materialized with receipts, preserving the exact histories, policies,
dispatch/memory admission rates, and continuation input that were visible
before command execution.
Use `AgentClosedLoopRuntimeServiceLoopRunDaemonRequestRunner` when the service
must first run runtime/model and memory ports to discover the expected command
plan. After the service executor returns receipts, close the resulting
`AgentClosedLoopRuntimeServiceLoopRunDaemonRequestRecord` with
`AgentClosedLoopRuntimeServiceLoopRunDaemonReceiptCloser` or
`request_record.close_with_receipts(...)` to produce the daemon record without
re-running the request boundary.
Use `AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummaryHistoryRecorder`
right after a request record when service/eval needs a pre-receipt trend gate.
Skipped or blocked request records can repair before command execution, while
clean executable rows stay stable and expose expected command pressure plus
dispatch/memory admission-rate evidence.
Use `AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredReceiptCloser`
when the service wants one close operation that records request-boundary health
and then closes receipts into the daemon record. It avoids splitting the
pre-receipt ledger row from the post-receipt transition result in service code.
Use `AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseRecord::summary`
when service/eval needs the flat post-close ledger row for request health,
daemon run status, daemon-control health, next-mode admission, and merged
blockers.
Keep `AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummaryHistory`
when service/eval needs multi-turn monitored-close pressure. Use
`AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummaryHistoryRecorder`
after each monitored close summary to append the row and recompute dashboard
health in one pure-data step while preserving dispatch/memory admission-rate
evidence.
Use `AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseContinuationPlanner`
after recording monitored-close summary history when the service wants one
persisted state packet containing both request-boundary history and
monitored-close trend health plus dispatch/memory admission evidence for the
next boundary.
Use `AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredClosePlanner`
when that persisted state should open the next request boundary without losing
monitored-close history or admission-rate evidence. Use
`AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseRunner` when
service/eval wants the same close-aware wrapper around request execution; its
record can be closed with receipts to append the next monitored-close trend row.
Use
`AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseRunRecord::summary`
when eval only needs the compact close-aware request-boundary row instead of
the nested plan and request record, including the carried dispatch/memory
admission rates.
Use `AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredContinuation`
when service/eval wants the next daemon state plus request-boundary history and
health plus admission-rate evidence in one persisted object. Use
`AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredPlanner` to build the
next request plan from that object without dropping the request-summary history
or admission-rate evidence needed by the following monitored receipt close.
Use `AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredRunner` when the
service wants the same history-preserving wrapper around the request runner; its
record telemetry carries dispatch/memory admission rates and can be closed with
receipts without manually splitting the history from the request record.
Use `AgentClosedLoopRuntimeServiceLoopRunDaemonInputPlanner` when the next
daemon turn is ready to be assembled from persisted continuation state plus
fresh service receipts. The resulting plan includes the full daemon input and
structured dispatch/memory admission-rate evidence plus telemetry, but does not
execute the engine or bypass dispatch gates. If service/eval wants this
receipt-attached input assembly in the adapter-boundary ledger, use
`AgentAdapterBoundaryGate::from_runtime_service_loop_daemon_input_plan`,
`AgentAdapterBoundarySnapshot::from_runtime_service_loop_daemon_input_plan`, or
`AgentAdapterBoundarySummaryHistoryRecorder::record_runtime_service_loop_daemon_input_plan_*`.
Those helpers persist the next daemon runtime queue as a watch-level row and
intentionally keep memory-note/adaptive promotion closed until a later
transition or request boundary supplies stable health.
When service/eval already has an
`AgentClosedLoopRuntimeServiceLoopState` or
`AgentClosedLoopRuntimeServiceLoopAdvance`, project it through
`AgentAdapterBoundaryGate::from_runtime_service_loop_state`,
`AgentAdapterBoundarySnapshot::from_runtime_service_loop_state`,
`AgentAdapterBoundaryGate::from_runtime_service_loop_advance`, or
`AgentAdapterBoundarySnapshot::from_runtime_service_loop_advance`.
Those projections reuse the preflight-continuation side-effect admission while
carrying the persisted or advanced next runtime queue, so stable rows open
service, memory-note, and adaptive-state admission; watch rows keep service
observation open while closing promotion; and repair rows route deterministic
adapter-boundary repair work ahead of the runtime queue. Use the matching
`AgentAdapterBoundarySummaryHistoryRecorder::record_runtime_service_loop_*`
helpers when eval needs the adapter snapshot, health row, and scheduler handoff
from the same loop-state packet without executing service, memory, eval, or
adaptive-state side effects.
If the service wants one call up to that side-effect boundary, use
`AgentClosedLoopRuntimeServiceRequestRunner::run`. It returns the command
request and the prior history, but still does not apply service commands.
After the service has attempted the emitted command plan, pass receipts to
`AgentClosedLoopRuntimeServiceTurnCloser`. It audits applied, missing, failed,
or skipped commands, records a new execution summary in history, and keeps
service-repair tasks in the next queue without applying the side effects itself.
Finally use `AgentClosedLoopRuntimeContinuationPlanner` to produce the next
`AgentClosedLoopRuntimeTurnInput` from the service turn. The continuation also
exposes dashboard and health snapshots for operator/eval surfaces.
When those receipt-audit and continuation steps should be closed together, pass
the command request, prior history, receipts, and continuation input through
`AgentClosedLoopRuntimeServiceOutcomePlanner`.
Use `AgentClosedLoopRuntimeServiceOutcome::summary` when eval, service logs, or
operator dashboards need one compact row instead of the full command request,
service turn, and continuation envelope. The summary preserves command-gate and
side-effect gate evidence for that single service boundary; cross-run
dispatch/memory admission rates remain on the service loop-run and daemon
histories.
For eval/report final-packet admission, keep
`AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistory`
next to the service/eval read model and close it through
`AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoffHandoff`.
That final handoff records admission health, turns dirty trend pressure into
repair tasks, and preserves the business queue without executing memory,
service, or tool side effects.

For self-evolution admission handoff, persist
`EvolutionAdmissionHandoffTrendContinuationHistoryGateSummary` rows when the
service/eval layer only needs the final gate result instead of the nested
handoff, monitor, continuation, and queue payload. Keep those rows in
`EvolutionAdmissionHandoffTrendContinuationHistoryGateSummaryHistory` and
append them with
`EvolutionAdmissionHandoffTrendContinuationHistoryGateHistoryRecorder`. The
dashboard exposes effective-admission rate, promotion counts, repair-first
pressure, repair-task pressure, queue pressure, blocked-reason pressure, and
continuation repair pressure as read-model evidence. Stable rows keep
ready-proposal, evolution-signal, process-reward, and adaptive-state promotion
open; watch rows stay observable but close promotion; repair rows require the
next scheduler turn to consume deterministic repair tasks before memory,
adaptive-state, service-command, or eval promotion can advance.
Adapters should prefer
`EvolutionAdmissionHandoffTrendContinuationHistoryGateHistoryRecord` query
methods such as `is_effectively_admitted`,
`can_promote_ready_proposals`, `can_promote_evolution_signals`,
`can_reinforce_process`, and `can_promote_adaptive_state` instead of
recombining summary booleans and history health locally.
When the collaboration layer needs a concrete side-effect boundary from that
self-evolution final row, use
`AgentCollaborationSideEffectBoundary::from_evolution_admission_handoff_history`.
It maps stable/effectively-admitted rows to continue mode and opens service,
memory-note, and adaptive-state gates from the record's promotion methods.
Watch rows become observe mode: service dispatch remains open for observation,
but memory-note and adaptive-state gates are closed. Repair rows become repair
mode, close all three gates, and preserve final-row health reasons, repair task
ids, and blocked-reason pressure for scheduler repair.
When dashboards or eval need the boundary recorded and gated in the same step,
call
`AgentCollaborationSideEffectBoundaryHandoffMonitor::record_evolution_admission_handoff_history`
with the final evolution history record plus the existing boundary and handoff
histories.
When they also need the final monitor-handoff gate packet, use
`AgentCollaborationSideEffectBoundaryHandoffMonitorHandoff::record_evolution_admission_handoff_history`.
That wrapper computes the boundary, records boundary health, records handoff
health, applies monitor-handoff trend health, and returns the final side-effect
gate state for service, memory, and adaptive-state adapters without executing
those side effects.
For adapter-level service/eval wiring, the same final evolution record can now
be projected into
`AgentCollaborationAdapterSideEffectAdmission::from_evolution_admission_handoff_history`
and then into
`AgentAdapterBoundaryGate::from_evolution_admission_handoff_history` or
`AgentAdapterBoundarySnapshot::from_evolution_admission_handoff_history`.
Use
`AgentAdapterBoundarySummaryHistoryRecorder::record_evolution_admission_handoff_history_with_health`
when eval needs the adapter snapshot plus trend health, or
`record_evolution_admission_handoff_history_handoff_with_health` when the next
scheduler turn also needs repair tasks merged ahead of the business queue.
Use
`AgentAdapterBoundaryHandoffHistoryRecorder::record_evolution_admission_handoff_history_with_health`
when eval/service wants the final handoff summary appended and health-checked
from that same evolution record without manually composing the intermediate
adapter snapshot and handoff.
After that append, eval/reporting can call
`AgentAdapterBoundaryHandoffHistoryRecord::report_gate_decision(run_id)` to
reuse the normal `AgentReportGateDecision` shape for adapter-boundary
acceptance. The projection accepts only stable handoffs, treats watch as a
non-repair hold with explicit memory/adaptive closure reasons, and turns repair
history into deterministic `eval-adapter-boundary` follow-up tasks.
For persistence, call
`AgentAdapterBoundaryHandoffHistoryRecord::record_report_gate_with_health` to
append that adapter-derived decision to `AgentReportGateSummaryHistory`, or
`record_report_gate_with_health_gate` to append the row and immediately apply
the report-gate health gate to the next queue. This keeps eval/reporting on its
existing report-gate ledger while preserving the adapter-boundary provenance.
When eval also needs the health-gate trend handoff, call
`record_report_gate_trend_handoff`; it records the adapter-derived report row,
applies the report-gate health gate, appends the health-gate summary trend, and
returns the same `AgentReportGateHealthGateTrendHandoffRecord` used by native
eval/report workflows. Use `record_report_gate_trend_handoff_monitor` when the
same adapter packet should also append trend-handoff history and apply the
outer monitor gate before service, memory, core, or eval owners consume the
next queue. Use `record_report_gate_trend_handoff_monitor_handoff` when
eval/service needs the monitor-summary append and monitor-health gate in the
same adapter packet; the returned monitor-handoff record preserves final
admission flags, repair tasks, blocker reasons, and queue ids without executing
any owner side effect. Use
`record_report_gate_trend_handoff_monitor_handoff_handoff` when that
monitor-handoff packet must itself be appended to trend history and gated before
the next eval/service boundary consumes the queue. When eval wants the same
adapter-derived packet to reach the final native admission rows, use
`record_report_gate_trend_handoff_monitor_handoff_handoff_handoff`; it appends
the monitor-handoff-handoff summary history, applies that packet health, and
returns the `AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoff`
record used by native eval workflows. Use
`record_report_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff` when
the final admission row also needs its own history append and gate before the
next service/eval handoff. These wrappers preserve the business queue on stable
admission, merge deterministic eval repair tasks on repair health, and still do
not execute memory, service, tool, or adaptive-state side effects.
This is a single-owner service-adapter projection; when `norion-core`,
`norion-memory`, and eval/reporting have their own fresh owner gates, compose
all four owners with `AgentAdapterBoundarySnapshot::from_boundary_gates`.
Stable evolution records open service/memory/adaptive admission, watch records
remain observable while closing promotion, and repair records close the adapter
boundary before any service, memory, adaptive-state, or eval side effect is
treated as admitted.

## Clean-room assignment guard

Use `AgentHelperRoleRepairRoutingReport` as the pure-data assignment guard when
helper-stage evidence is carried across clean-room windows. The report sanitizes
helper and evidence identifiers before building repair tasks, rejects raw
dialog/context markers such as old thread bodies, raw history, payload fields,
and raw dialog/context labels, and keeps `side_effect_dispatch_allowed=false`
for both observe and repair decisions.

Completed helper evidence is counted for observability only. Its evidence ids
are not promoted into `inherited_evidence_result_ids`, so a completed old
window cannot seed new work. Incomplete helper evidence may produce
deterministic matching-role repair tasks after identifier sanitization:
summary to aggregator, router to planner, review to reviewer, index to memory
curator, and test-gate to tester. Callers should consume the report fields as
data and let the scheduler decide whether a fresh clean-room repair task is
needed; the guard does not start threads, send messages, read old-window
payloads, or dispatch side effects.

## Current constraints

The crate has no external dependencies and is registered in the root workspace,
but it remains testable on its own. Run focused tests with:

```powershell
cargo test --manifest-path crates\norion-agent\Cargo.toml
```
