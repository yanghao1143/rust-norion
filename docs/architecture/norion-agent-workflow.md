# norion-agent Workflow

`crates/norion-agent` is the pure coordination layer for the six-window Smart
Steam workflow. It does not run models, write files, or persist memory. The
service or main window owns those side effects and drives this crate through
data-only plans, ports, reports, and gates.

## Multi-Window Collaboration Contract

- Window goals are declared as data before dispatch. The coordinator should
  translate each active Smart Steam window into an `AgentWindowSpec` with a
  stable window id, role, objective, lane, priority, dependencies, and isolated
  budget. Logical roles describe the work owner; `AgentRole::Custom("window-N")`
  may be used when the physical collaboration window is the stable owner. If a
  caller allows zero-budget windows for observation, strict dispatch still
  rejects only that depleted window and keeps other window budgets isolated.
- Ownership boundaries are explicit. A window may write only the paths owned by
  its slice, while `crates/norion-agent` records that boundary as task metadata,
  budget ledger rows, schedule waves, aggregation/conflict reports, and
  side-effect gates. Call `AgentWindowOwnershipReviewer` with each window's
  owned paths and reported changed files before treating writes as admitted.
  Shared changed paths or changed paths outside the owning slice close write
  admission and emit deterministic `collaboration-ownership-repair-*` planner
  tasks. Feed those tasks through `RecursiveAgentScheduler::plan_repair_first`
  or `AgentTaskQueue::with_repair_first` before resuming normal window work.
  Persist `AgentWindowOwnershipReviewSummary` rows in
  `AgentWindowOwnershipReviewSummaryHistory` when eval needs cross-handoff
  ownership health; repair health closes service advance until the ownership
  boundary is repaired.
  The crate never writes files, memory, commands, or adaptive state; it only
  returns admission decisions and repair-first queues.
- File locks are declared before dispatch and checked after handoff. Treat each
  window's owned paths as the write lock for that slice; read-only inspection
  does not grant write ownership. If a window reports a changed file outside
  its lock, or two windows report the same changed file, the coordinator should
  keep the write side effect closed and schedule ownership repair before normal
  queue promotion resumes.
- Every window handoff should report the same compact fields: changed files,
  test commands and results, queue or wave ids consumed, budget admission,
  aggregation/conflict status, repair task ids, side-effect admission flags,
  blockers, and the next suggested slice. Use stable field names in service/eval
  rows: `window_id`, `objective`, `owned_paths`, `changed_files`,
  `validation_commands`, `validation_status`, `budget_status`,
  `repair_task_ids`, `blocked_reasons`, and `next_slice`. Payload-free summaries
  and telemetry rows are preferred for eval dashboards; full task/message
  payloads stay at the service boundary that owns them.
- Conflict handling is repair-first. Duplicate aggregation pressure, unresolved
  conflict pressure, budget repair, reflection repair, or schedule repair closes
  memory-note promotion, side-effect admission, and ordinary next-task
  promotion. The coordinator must enqueue deterministic repair tasks before any
  normal task queue is admitted again.
- Scheduler handoff must preserve that ordering. Use the repair queue returned
  by the relevant gate directly, or call `RecursiveAgentScheduler::plan_repair_first`
  with repair tasks plus the requested business queue so memory notes,
  side-effect admission, and next-task promotion depend on repair completion.
  When a caller needs the queue rather than the planned waves, use
  `AgentTaskQueue::with_repair_first` for the same dependency injection instead
  of reimplementing repair queue merging inside an adapter or gate wrapper.
- R14 repair-first merge evidence: `AgentTaskQueue::with_repair_first`
  preserves every requested business task whose id is not replaced by the
  repair queue, appends each repair id as an ordinary dependency of those
  business tasks, and strips repair-task dependencies that point back at the
  preserved business queue before merging. That keeps repair waves ahead of
  business waves without manufacturing a dependency cycle. Final adapter
  handoff packets remain payload-free across windows: dirty persisted history
  can close admission and add deterministic repair ids, but it cannot carry an
  old window's task payload or context into the new handoff.

## Workflow Spine

1. Build an `AgentTask` queue for the active windows.
   - Use stable roles such as `Planner`, `Coder`, `Reviewer`, `Tester`,
     `MemoryCurator`, and `Aggregator`.
   - Use `AgentRole::Custom("window-3")` when the physical collaboration window
     matters more than the logical role.
   - Prefer `AgentWindowSpec` plus `AgentCollaborationPlanner` when the main
     window wants one data-only plan that includes the task queue, isolated
     budgets, active-window ids, duplicate-window blockers, and telemetry.
   - Use `AgentCollaborationPlan::summary` or `AgentCollaborationPlan::gate`
     before dispatch when service/eval needs a compact six-window admission row.
     The default gate expects six active windows, rejects duplicate window ids,
     rejects shared `AgentRole` budget slots, rejects zero-budget windows, and
     closes the side-effect boundary until the plan is complete and budget
     isolated.
   - Persist plan rows with `AgentCollaborationPlanSummaryHistoryRecorder` when
     service/eval needs cross-turn six-window planning health. Use
     `AgentCollaborationPlanHistoryGate`, or
     `record_plan_with_health_gate`, before the scheduler consumes a fresh
     plan: stable history preserves dispatch and side-effect-boundary
     admission, watch history remains observable, and current or persisted
     plan repair pressure emits deterministic `collaboration-plan-repair`
     tasks plus a repair-only queue before ordinary queue dispatch opens.
   - Store pending work in `AgentTaskQueue` and drain only tasks whose
     dependencies are already completed.
   - Use `RecursiveAgentScheduler` when the coordinator needs an explicit
     max-parallel wave plan before dispatch.
   - Use `RecursiveAgentScheduler::plan_repair_first` when an aggregation,
     conflict, budget, reflection, or schedule gate returned repair tasks. The
     scheduler adds those repair task ids as dependencies of the requested
     business queue, so repair waves are emitted before memory-note,
     side-effect-admission, or next-task-promotion waves.
     `ConflictReportHistoryGate` repair tasks are a direct input to this path:
     do not promote normal work from an unresolved conflict packet until its
     conflict repair wave has run first.
   - `RecursiveAgentSchedule::summary` exposes wave count, completed/blocked
     task counts, max wave parallelism, and average wave parallelism for
     service/eval dashboards.
   - `RecursiveAgentSchedule::gate` requires repair-first when dependency
     cycles or other blockers leave the schedule with blocked tasks or no
     dispatchable waves. A partial schedule with completed waves is still
     repair-first when any task remains blocked; completed wave evidence is
     observable but does not open the dispatch boundary.
   - `RecursiveAgentScheduleSummaryHistoryRecorder` persists schedule rows into
     `RecursiveAgentScheduleHealth`; blocked cycles or repeated empty waves
     become trend repair signals before dispatch opens.
   - Read `allows_service_advance` and `requires_repair_first` from schedule
     health or history records before service/eval admits the next dispatch
     boundary.
   - Use `RecursiveAgentScheduleHistoryGate` to apply persisted schedule health
     back onto a fresh wave plan; repair history emits deterministic
     `recursive-agent-schedule-repair` tasks and blocks dispatch even when the
     current wave plan is clean.
   - Use
     `RecursiveAgentScheduleSummaryHistoryRecorder::record_schedule_with_health_gate`
     when service/eval wants the schedule summary append, trend health, dispatch
     gate, repair tasks, and telemetry from one replayable pure-data record.
   - Prefer `AgentCollaborationDispatchPreflight::record_and_gate` when
     `norion-core`, service, or eval needs one atomic row before dispatch. It
     appends and gates the six-window plan, budget ledger, and recursive
     schedule together, then returns the admitted queue flags, side-effect
     boundary flag, merged repair queue, reasons, and telemetry visible before
     any engine, memory, service-command, or adaptive-state side effect opens.
     Store `AgentCollaborationDispatchPreflightSummary` when eval only needs
     the flat record counts, health statuses, admission flags, repair pressure,
     blocker count, and schedule-wave count for that boundary.
     Persist those flat rows in
     `AgentCollaborationDispatchPreflightSummaryHistory` and append with
     `AgentCollaborationDispatchPreflightSummaryHistoryRecorder::record_preflight_with_health`
     when service/eval needs cross-boundary health for dispatch rate,
     side-effect-boundary open rate, repair-first pressure, per-layer repair
     pressure, repair queue size, blocker count, and schedule-wave evidence.
     Plan-history repair is enforced at this combined boundary too: a clean
     current six-window plan with stable budget and schedule rows still cannot
     dispatch or open side-effect admission while persisted plan health is
     repair-first, and the preflight repair queue contains only the plan repair
     tasks for that blocker.
     Apply `AgentCollaborationOwnershipPreflightGate` when window ownership
     summaries are available for the same handoff. Stable ownership health
     preserves the preflight admission flags, while repair ownership health
     closes queue dispatch, memory-note promotion, side-effect admission, and
     next-task promotion even when the preflight itself is clean. Ownership
     repair ids stay ahead of preflight repair work in the merged repair queue.
     Persist the resulting `AgentCollaborationOwnershipPreflightGateSummary`
     with `AgentCollaborationOwnershipPreflightGateSummaryHistoryRecorder` when
     cross-resume service/eval needs dispatch, side-effect, memory-note, and
     next-task admission rates plus ownership repair pressure without expanding
     the nested preflight or ownership records.
     Use `AgentCollaborationDispatchPreflightHistoryGate`, or the recorder's
     `record_preflight_with_health_gate`, to apply that persisted trend back to
     the current preflight before the scheduler consumes it. Repair history
     closes both queue dispatch and side-effect-boundary admission, then merges
     deterministic `collaboration-dispatch-preflight-repair` tasks ahead of the
     ordinary business queue.
     Call `scheduler_handoff(requested_queue)` on the resulting gate record
     before handing work to `norion-core`: stable admission keeps the requested
     business queue as the effective queue, while repair-first admission
     injects the preflight repair queue ahead of the business queue with
     `AgentTaskQueue::with_repair_first`. Persist
     `AgentCollaborationDispatchPreflightSchedulerHandoffSummary` when eval
     needs the requested/effective queue sizes, repair queue size, admission
     flags, and blocker count without queued task payloads.
     Store those summaries in
     `AgentCollaborationDispatchPreflightSchedulerHandoffSummaryHistory` and
     append with
     `AgentCollaborationDispatchPreflightSchedulerHandoffSummaryHistoryRecorder::record_handoff_with_health`
     when service/eval needs trend health for dispatchable handoff rate,
     side-effect open rate, repair-first handoff pressure, repair-queue
     dependency-injection pressure, and blocker pressure.
     Apply `AgentCollaborationDispatchPreflightSchedulerHandoffHistoryGate`, or
     use the recorder's `record_handoff_with_health_gate`, before
     `norion-core` consumes the effective queue. Stable handoff history keeps
     the current handoff queue intact, while repair handoff history closes
     scheduler and side-effect admission and merges deterministic
     `collaboration-scheduler-handoff-repair-*` tasks into the effective queue
     with `AgentTaskQueue::with_repair_first`, so service/eval can audit the
     repair wave before the preserved business work resumes. Persist
     `AgentCollaborationDispatchPreflightSchedulerHandoffHistoryGateSummary`
     when eval only needs the gate health, admission booleans, repair ids,
     effective queue ids, reasons, and telemetry. Store those final gate rows
     in
     `AgentCollaborationDispatchPreflightSchedulerHandoffHistoryGateSummaryHistory`
     and append with
     `AgentCollaborationDispatchPreflightSchedulerHandoffHistoryGateSummaryHistoryRecorder`
     when service/eval needs a trend signal for repeated final-gate repairs,
     closed dispatch, closed side-effect admission, repair-task pressure, and
     effective-queue pressure before the next `norion-core` handoff. Apply
     `AgentCollaborationDispatchPreflightSchedulerHandoffHistoryGateTrendGate`
     before the next scheduler handoff if persisted final-gate trend health
     must be enforced: stable/watch trend health preserves the current queue,
     while repair trend health merges
     `collaboration-scheduler-handoff-gate-trend-repair-*` tasks into the
     current effective queue with `AgentTaskQueue::with_repair_first`. Store
     `AgentCollaborationDispatchPreflightSchedulerHandoffHistoryGateTrendGateSummary`
     rows in
     `AgentCollaborationDispatchPreflightSchedulerHandoffHistoryGateTrendGateSummaryHistory`
     when eval needs to track repeated trend-enforced repair, closed dispatch,
     closed side-effect admission, effective-queue pressure, and trend
     repair-task volume without retaining full task payloads.
2. Reserve isolated budgets with `DispatchPlanner`.
   - `BudgetLedger` stores per-role token, step, and message allowances.
   - `BudgetPolicy::strict()` can reject zero-budget tasks before any role
     allowance is consumed.
   - Budget exhaustion rejects only that task or role; it does not debit other
     windows.
   - `BudgetLedgerSummaryHistoryRecorder` persists per-role remaining-budget
     snapshots into `BudgetLedgerHealth`, making depleted roles and zero-budget
     pressure visible before dispatch or side-effect gates run. Read
     `allows_service_advance` and `requires_repair_first` from the health or
     history record when the service needs a direct budget-trend admission
     signal.
   - Use `BudgetLedgerHistoryGate`, or
     `BudgetLedgerSummaryHistoryRecorder::record_ledger_with_health_gate`,
     before the next scheduler/service boundary consumes a fresh ledger
     snapshot. Stable ledgers can dispatch and promote side-effect-admission
     evidence; current zero-budget roles or repair history close both paths and
     append deterministic `budget-ledger-repair` planner tasks.
   - `TaskDispatchPlan::summary` gives service/eval assigned/rejected counts,
     aggregate remaining budget, and assignment/rejection rates without
     expanding every task.
   - `TaskDispatchPlan::gate` closes execution when the wave has no assignments
     or any budget/policy rejection, returning deterministic repair-first
     reasons before an `EnginePort` adapter is called.
   - Persist `TaskDispatchPlanSummary` rows with
     `TaskDispatchPlanSummaryHistoryRecorder` when the service needs
     cross-turn dispatch health. `TaskDispatchHealth` turns budget rejections,
     empty assignment waves, low assignment rate, and low remaining-budget
     evidence into stable/watch/repair pressure before `norion-core` calls the
     engine adapter or `norion-memory` sees any promoted note.
   - Read `allows_service_advance` and `requires_repair_first` from dispatch
     health or history records before opening `EnginePort`; repair-level
     rejection or empty-assignment trends should enqueue repair work first.
3. Run accepted assignments through an adapter that implements `EnginePort`.
   - The adapter may call `norion-core`, a model service, or a test fake.
   - It returns `AgentResult` with structured `AgentMessage` values.
   - `AgentWaveExecutor` runs accepted tasks in dispatch order and records
     engine errors as `AgentExecutionFailure` values instead of fabricating
     successful results.
   - `AgentWaveExecution::summary` compacts results, accepted/rejected result
     counts, failures, failed task ids, and completion status into a stable
     service/eval row.
   - Persist those rows with `AgentWaveExecutionSummaryHistoryRecorder` when
     the service needs cross-turn execution health. `AgentWaveExecutionHealth`
     flags adapter failures, incomplete waves, rejected results, empty
     executions, and low completion/acceptance rates before the cycle is closed
     into report gates, memory submission, or service command planning.
   - Read `allows_service_advance` and `requires_repair_first` from execution
     health or history records before cycle close consumes wave results for
     memory, eval, or service-command side-effect planning.
4. Record results in `AgentRunLedger`.
   - Results may finish in any order.
   - The ledger merges dispatched results in dispatch order so reports remain
     deterministic. Any extra result that was not part of the dispatch plan is
     appended by task id, keeping loose helper reports stable without changing
     the admitted window order.
   - Read `AgentRunLedger::progress` before treating a run as closed. A run is
     not closable when any dispatched window is missing, any result is rejected,
     or any result came from a task that was not dispatched; those progress
     blockers close memory-note, file-write, adaptive-state, and external-call
     side-effect gates in `AgentRunLedger::report`.
   - Persist `AgentRunLedgerProgressSummary` rows with
     `AgentRunLedgerProgressSummaryHistoryRecorder` when dashboards or eval
     need close-readiness trends across windows. Repair health means the next
     service boundary should schedule progress repair before treating report
     aggregation, memory notes, adaptive writes, or external calls as admitted.
   - Use `AgentRunProgressReportGate` when both progress health and
     `AgentRunReportHealthGateRecord` are available. It preserves clean
     progress/report admission, but when progress is repair-first it injects
     `agent-run-ledger-progress` repair tasks ahead of report-health repair
     tasks and the requested business queue.
   - Persist the resulting `AgentRunProgressReportGateSummary` in
     `AgentRunProgressReportGateSummaryHistory` when eval needs admission-rate,
     progress/report repair mix, repair-task pressure, queue pressure, and
     blocker pressure for the combined close/report boundary.
   - When the service has loose agent messages before run close, use
     `AggregationConflictReviewer` to aggregate messages, append/gate
     aggregation health, run conflict detection on the stable aggregate, and
     append/gate conflict health in one packet. Only forward/promote when both
     aggregation and conflict gates are clean; duplicate pressure or unresolved
     conflict pressure should enqueue the packet's repair tasks before memory,
     reflection, adaptive-state, or eval promotion. Persist
     `AggregationConflictReviewSummary` rows in
     `AggregationConflictReviewSummaryHistory` when eval needs cross-turn
     health for forward/promotion closure, duplicate pressure, unresolved
     conflict pressure, and combined repair-task volume without replaying full
     messages. Apply `AggregationConflictReviewTrendGate` before the next
     message boundary when persisted review health must be enforced; repair
     history closes forwarding and side-effect promotion even if the current
     aggregate/conflict review is clean.
   - `RunBudgetAudit` compares reserved dispatch budget with
     `AgentResult::budget_spent` and reports overspends.
   - `RunBudgetAudit::summary` exposes overspend count and overspent
     token/step/message totals for eval rows.
   - `AgentRunReport::summary` compacts aggregation, conflict, budget, and
     side-effect gate evidence into one run-level row.
   - `AgentRunReport::gate` keeps memory-note promotion, adaptive-state writes,
     and external calls closed when conflicts, overspends, or blocked
     side-effect gates require repair-first.
   - `AgentRunReportSummaryHistoryRecorder` appends run-level rows and computes
     `AgentRunReportHealth` so service/eval can track clean-rate,
     conflict/budget pressure, side-effect blockers, and memory/adaptive
     admission trends across runs before the business loop promotes state.
   - `AgentRunReportHealthGate` converts that run-level trend health into the
     next scheduler handoff: stable and watch rows keep the current queue
     admitted, while repair rows block ordinary progression and merge
     deterministic `agent-run-report-health` repair tasks before
     `norion-memory`, `norion-core`, service commands, or eval promotion act on
     the run.
   - Use `AgentRunReportSummaryHistoryRecorder::record_*_with_health_gate`
     when service/eval wants the append, trend health, gate decision, merged
     next queue, and compact `AgentRunReportHealthGateSummary` from one
     replayable pure-data call.
   - Persist `AgentRunReportHealthGateSummary` rows in
     `AgentRunReportHealthGateHistory` when eval needs cross-run admission,
     repair-first, repair-task, queue-pressure, and latest-blocker trends
     without replaying full queues.
   - Pass `AgentRunReportHealthGateHistory::health` into
     `AgentRunReportHealthGateTrendGate` before the next scheduler handoff when
     persisted gate trends should preserve stable/watch queues or force
     `agent-run-report-health-gate` repair work first.
   - Use `AgentRunReportHealthGateTrendHandoff` when service/eval wants that
     compact summary append, trend-health recomputation, trend gate decision,
     merged next queue, and handoff summary from one replayable pure-data call.
   - Store `AgentRunReportHealthGateTrendHandoffSummary` rows in
     `AgentRunReportHealthGateTrendHandoffHistory` when eval needs
     payload-free trends for admitted rate, repair-first pressure,
     trend-health mix, queue pressure, repair tasks, and blocked reasons.
   - Apply `AgentRunReportHealthGateTrendHandoffGate` to the recorded
     handoff-history health before the next `norion-core` scheduler or
     service/eval boundary consumes the queue; repair health blocks admission
     and appends deterministic `agent-run-report-health-gate-handoff` work,
     while stable/watch health leaves the requested handoff intact.
   - Use `AgentRunReportHealthGateTrendHandoffMonitor` when the adapter wants
     one pure-data call that appends the compact handoff row, recomputes
     handoff-history health, gates the current next queue, and emits a compact
     monitor summary for `norion-core`, `norion-memory`, service, and eval to
     share.
   - Persist `AgentRunReportHealthGateTrendHandoffMonitorSummary` rows in
     `AgentRunReportHealthGateTrendHandoffMonitorSummaryHistory` when eval
     needs cross-boundary trend health for requested admission, effective
     admission, repair-first pressure, handoff-health mix, queue pressure,
     repair work, and blockers without retaining full task payloads.
   - Apply `AgentRunReportHealthGateTrendHandoffMonitorGate` before the next
     scheduler/service/eval boundary consumes the monitor queue; stable/watch
     monitor health keeps the queue admitted, while repair health appends
     deterministic `agent-run-report-health-gate-handoff-monitor` repair tasks
     and blocks ordinary progression.
   - Use `AgentRunReportHealthGateTrendHandoffMonitorHandoff` when adapters
     want the monitor-summary append, monitor-health gate, persisted telemetry,
     and gated next queue from one replayable service/eval packet.
   - Persist `AgentRunReportHealthGateTrendHandoffMonitorHandoffSummary` rows
     in `AgentRunReportHealthGateTrendHandoffMonitorHandoffSummaryHistory` when
     eval needs a payload-free trend of those final monitor-handoff packets
     before another scheduler/service boundary opens.
   - Apply `AgentRunReportHealthGateTrendHandoffMonitorHandoffGate`, or use
     `AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoff` for the
     append-and-gate path, before `norion-core`, `norion-memory`, service
     commands, or eval consume the final monitor-handoff queue. Stable/watch
     trend health preserves the current queue and telemetry, while repair trend
     health blocks admission and appends deterministic
     `agent-run-report-health-gate-handoff-monitor-handoff` repair work.
     Use the packet `summary()` when eval or service logs need final admission,
     repair task ids, queue ids, and blocker pressure without storing nested
     task payloads.
   - Persist those final packet summaries in
     `AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistory`
     and append them with
     `AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffSummaryHistoryRecorder`
     when service/eval needs a dashboard and health row for final-packet
     admission rate, repair-first pressure, repair work, queue pressure, and
     blocker pressure across scheduler boundaries.
   - Apply
     `AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffGate` before
     the next scheduler or service/eval consumer acts on that packet trend.
     Stable/watch health preserves the packet queue; repair health blocks
     ordinary admission and appends deterministic
     `agent-run-report-health-gate-handoff-monitor-handoff-handoff` repair
     tasks first.
   - Use
     `AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoff` when
     the adapter wants the final-packet summary append and final-packet gate in
     one replayable operation. The resulting record is the atomic handoff for
     `norion-core`, `norion-memory`, service commands, or eval jobs that must
     see the same history row, gate decision, next queue, repair tasks, and
     telemetry. Use its `summary()` as the final payload-free eval row for
     packet health, admission, repair-first state, queue ids, repair ids, and
     blocker pressure.
   - Persist those final admission rows in
     `AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistory`
     and append them with
     `AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffSummaryHistoryRecorder`
     when service/eval needs a dashboard and health row for final admission
     rate, repair-first pressure, packet-health mix, repair work, queue
     pressure, and blocker pressure before starting another business-loop
     scheduler boundary.
   - Read `records()`, `allows_service_advance`, and
     `requires_repair_first` from run-report health history records at each
     append boundary. Stable/watch rows remain admissible for observation and
     scheduler continuation; repair rows are the only helper state that should
     force repair-first admission before `norion-memory`, adaptive-state,
     service-command, or external-call effects are opened.
   - Apply
     `AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffGate`
     before the next scheduler or service/eval consumer starts from that final
     admission trend. Stable/watch health preserves the queue; repair health
     blocks ordinary progression and appends deterministic
     `agent-run-report-health-gate-handoff-monitor-handoff-handoff-handoff`
     repair tasks first.
   - Use
     `AgentRunReportHealthGateTrendHandoffMonitorHandoffHandoffHandoffHandoff`
     when service/eval wants the final admission summary append and final
     admission trend gate from one replayable packet before handing the gated
     queue back to the business-loop scheduler. Use its `summary()` as the
     final payload-free eval row for admission-trend health, admission,
     repair-first state, queue ids, repair ids, and blocker pressure.
5. Aggregate and detect conflicts.
   - `MessageAggregator` deduplicates repeated findings while preserving source
     ids and evidence in sorted order. Unique aggregate rows are emitted in
     stable fingerprint order, so concurrent Agent result arrival does not
     change downstream conflict or ledger ordering.
   - `AggregationReport::summary` gives service/eval a compact row for input
     count, unique count, duplicate groups, duplicate message pressure, and
     compression rate.
   - `AggregationSummaryHistoryRecorder` persists duplicate-pressure rows into
     `AggregationHealth`, making repeated duplicate reports visible before
     conflict resolution or side-effect admission consumes the message set.
   - Use `AggregationHistoryGate`, or
     `AggregationSummaryHistoryRecorder::record_report_with_health_gate`, before
     forwarding aggregated messages into conflict/run layers. Stable unique
     messages can continue; current or persisted duplicate pressure emits
     deterministic `aggregation-repair` tasks and blocks ordinary forwarding.
     Read `allows_service_advance` and `requires_repair_first` from the health
     or history record before allowing duplicate-heavy report trends to feed
     memory notes, adaptive state, or service commands.
   - `ConflictResolver` marks positive/negative stance collisions on the same
     topic.
   - `ConflictReport::summary` gives service/eval a compact row for conflict
     topics, resolved/unresolved counts, conflicted message count, and
     all-resolved status.
   - `ConflictReportSummaryHistoryRecorder` persists those rows into
     `ConflictReportHealth`; unresolved-conflict repair status must keep memory
     notes and side effects closed until a covering resolution is recorded.
     Read `allows_service_advance` and `requires_repair_first` from the health
     or history record when the service needs a direct conflict-trend admission
     signal.
   - Use `ConflictReportHistoryGate`, or
     `ConflictReportSummaryHistoryRecorder::record_report_with_health_gate`,
     before aggregated reports feed memory notes, adaptive-state writes, or
     service commands. Clean current reports can continue only when persisted
     conflict health is stable/watch; current unresolved conflicts or repair
     history close report forwarding and side-effect promotion, then append
     deterministic `conflict-report-repair` reviewer tasks.
   - `ConflictResolutionBook` records coordinator-approved resolutions; each
     resolution must name the topic, cover every conflicting message id, and
     include a rationale. Resolved current conflicts remain visible in
     summaries, but they do not close side-effect promotion when conflict health
     is stable and no unresolved conflict remains.
6. Run reflection and side-effect gates.
   - `ReflectionLoop` must complete `draft -> critique -> revision ->
     memory_note`.
   - `ReflectionLoop::summary` exposes entries, next stage, remaining stages,
     completion, memory-note readiness, and telemetry for service/eval rows
     without expanding the reflection transcript.
   - `ReflectionLoop::gate` tells adapters whether reflection can continue and
     whether a memory note is ready to promote; it does not write memory.
   - `ReflectionLoopSummaryHistoryRecorder` appends low-level reflection rows
     and computes `ReflectionLoopHealth` so service/eval can watch empty or
     incomplete reflection and repair repeated stalls before memory-note
     promotion.
   - Use `ReflectionLoopHistoryGate` before a reflection memory note is handed to
     `norion-memory`: stable/watch history preserves normal continuation or
     promotion, while repair history emits deterministic `reflection-loop-repair`
     tasks and blocks memory-note promotion.
   - Use `ReflectionLoopSummaryHistoryRecorder::record_loop_with_health_gate`
     when service/eval wants the reflection summary append, health row, memory
     promotion gate, repair tasks, and telemetry from one replayable pure-data
     record.
   - Read `allows_service_advance` and `requires_repair_first` from reflection
     health or history records when service/eval needs to observe incomplete
     reflection but block repeated-stall trends before memory-note promotion.
   - `AgentRunLedger::gate_side_effect` blocks memory notes, file writes,
     adaptive state writes, and external calls while conflicts remain
     unresolved.
   - `AgentRunLedger::report_with_resolutions` recomputes side-effect gates
     after accepted resolutions are applied.
7. Score the closed loop.
   - `ToolsmithPlan` records Rust-only tool proposals as ready, held, or
     rejected.
   - `ClosedLoopRewarder` combines quality, validation, runtime response,
     reflection completion, recursion pressure, toolsmith gates, conflicts, and
     side-effect admission into a `ProcessRewardReport`.
   - `EvolutionSignal` values tell the outer loop whether to reinforce, hold,
     or repair the agent workflow.
   - Persist `ToolsmithPlan::summary` rows with
     `ToolsmithPlanSummaryHistoryRecorder` when service/eval needs trend
     evidence for Rust-only gate failures, rejected tool requests, held work,
     ready proposal rate, and non-Rust proposal pressure before tool-building
     or adaptive promotion is admitted.
   - Use `ToolsmithPlanHistoryGate` to apply persisted toolsmith health to the
     current Rust-only plan before promotion; repair history emits deterministic
     `toolsmith-plan-repair` tasks and blocks ready proposals until non-Rust or
     rejected-tool drift is repaired.
   - Use `ToolsmithPlanSummaryHistoryRecorder::record_plan_with_health_gate`
     when service/eval wants the Rust-only plan summary append, trend health,
     ready-proposal gate, repair tasks, and telemetry from one replayable
     pure-data record.
   - Convert only final-admitted ready Rust proposals into `ToolBuildRequest`
     values with `ToolBuildRequest::admitted_by_evolution` before calling a
     service-owned `ToolBuildPort`. Held, rejected, and non-Rust proposals do
     not materialize build requests, and repair-first toolsmith, process-reward,
     or evolution-admission history returns an empty request set, so
     `norion-agent` can expose the tool-building boundary without executing
     tool side effects itself.
   - After the service-owned builder returns `ToolBuildReceipt` values, close
     them with `ToolBuildReport::from_requests_and_receipts`. Missing,
     unexpected, duplicate, held, or rejected receipts become repair-first
     evidence before any follow-up memory note, adaptive-state promotion, or
     next tool-building turn treats the build as clean.
   - Persist `ToolBuildReport::summary` rows with
     `ToolBuildReportSummaryHistoryRecorder` when service/eval needs
     cross-turn receipt health without storing artifact paths, diagnostics
     payloads, or queued task bodies. Empty report history is watch evidence,
     clean histories remain stable, and missing, unexpected, duplicate, held,
     or rejected receipt pressure beyond `ToolBuildReportHealthPolicy` requires
     repair before the next tool-building, memory-note, adaptive-state, or eval
     promotion boundary advances.
   - Use `ToolBuildReportHistoryGate`, or
     `ToolBuildReportSummaryHistoryRecorder::record_report_with_health_gate`,
     when that persisted receipt health must constrain the current boundary.
     Stable clean reports open the next service-owned tool-build boundary and
     keep memory-note, adaptive-state, and eval promotion available. Current or
     historical dirty receipt pressure emits deterministic
     `tool-build-report-repair` tasks and keeps those promotions closed until
     repair work is scheduled.
   - Persist `ProcessRewardReport::summary` rows with
     `ProcessRewardReportSummaryHistoryRecorder` after scoring. The resulting
     `ProcessRewardReportHealth` gives the self-evolution loop a compact
     stable/watch/repair signal for average reward, penalize pressure, low
     component pressure, and missing evolution signals before it reinforces,
     observes, or schedules repair.
     Pass a `ToolBuildReport` into `ProcessRewardInput` when the cycle includes
     service-owned tool builds; dirty build receipts lower the toolsmith reward
     component and add explicit tool-build repair evidence to the reward notes.
     When scoring happens through `AgentCycleOrchestrator`, carry the same
     report in `AgentCycleEvidence::tool_build_report` so the cycle closer feeds
     tool-build receipt health into process reward without the service/eval
     layer hand-assembling reward input. The closed cycle also preserves a
     `ToolBuildReportSummary`, so later report gates can inspect receipt
     pressure without artifact paths, diagnostic payloads, or reward-note
     parsing.
   - Use `ProcessRewardReportHistoryGate` to apply that persisted reward health
     to the current report before evolution signals or adaptive-state evidence
     are promoted. Stable/watch health with non-empty signals can continue;
     repair health, current penalties, low scores, or missing signals emit
     deterministic `process-reward-report-repair` tasks and block ordinary
     reinforcement. A high-score `Hold` report without an `EvolutionSignal` is
     still repair-first for self-evolution admission; score evidence alone does
     not open process reinforcement.
   - Use
     `ProcessRewardReportSummaryHistoryRecorder::record_report_with_health_gate`
     when service/eval wants the reward summary append, trend health, signal
     promotion gate, repair tasks, and telemetry from one atomic eval row.
   - Use `ReflectionRewardAdmissionGate` when a reward record and reflection
     record must cross the memory boundary together. It keeps `Reinforce` from
     promoting evolution signals or process reinforcement until
     `ReflectionLoopHistoryGate` can promote the memory note, while still
     allowing incomplete clean reflection to continue. If both sides require
     repair, reflection repair tasks are emitted before process-reward repair
     tasks.
   - Use `EvolutionAdmissionGate` after the toolsmith and process-reward
     append-and-gate records have been built. It merges ready-tool,
     evolution-signal, process-reinforcement, adaptive-state, repair-task, and
     blocked-reason decisions into one pure-data admission record.
   - The combined evolution admission record is the service/eval boundary before
     tool building, adaptive-state promotion, or self-evolution reinforcement.
     It never executes tool builds, writes memory, applies adaptive state, or
     sends service commands; adapters consume its booleans and telemetry.
   - Persist `EvolutionAdmissionSummary` rows with
     `EvolutionAdmissionSummaryHistoryRecorder` when service/eval needs trend
     evidence over combined self-evolution admission. The dashboard tracks
     admitted records, repair-first pressure, ready-tool and signal promotion,
     process reinforcement, adaptive-state admission, repair tasks, blocked
     reasons, and upstream toolsmith/reward repair pressure.
   - Use `EvolutionAdmissionHistoryGate`, or
     `EvolutionAdmissionSummaryHistoryRecorder::record_admission_with_health_gate`,
     before a combined admission record opens tool-building, reinforcement, or
     adaptive-state promotion. Stable history preserves the current admission,
     watch history allows observation without adaptive-state promotion, and
     repair history emits deterministic `evolution-admission-repair` tasks
     before any promotion boundary advances.
   - Build `EvolutionAdmissionHandoff` from the history-gated record and the
     pending business queue before returning control to the scheduler. It keeps
     the business queue intact on clean admission and merges
     `evolution-admission-repair` tasks into the next queue on repair-first
     trends; the handoff summary is the payload-light eval row for effective
     admission, promotion booleans, queue size, repair ids, and blocked reasons.
   - Persist `EvolutionAdmissionHandoffSummary` rows with
     `EvolutionAdmissionHandoffSummaryHistoryRecorder` before the scheduler
     consumes the next queue. Handoff health gives service/eval trend evidence
     for effective admission rate, repair-first pressure, repair-task pressure,
     blocked reasons, admission repair health, and next-queue pressure without
     executing the queued work.
   - Apply `EvolutionAdmissionHandoffTrendGate` to the persisted handoff
     history record and the current handoff immediately before scheduler
     consumption. Stable history preserves the handoff queue and promotion
     booleans, watch history keeps the queue observable while closing
     promotion/adaptive-state flags, and repair history appends deterministic
     `evolution-admission-handoff-trend-repair` tasks to the next queue before
     service/core/memory owners advance.
   - Prefer `EvolutionAdmissionHandoffTrendMonitor` when service/eval wants the
     handoff summary append, handoff-history health recomputation, final trend
     gate, and gated queue returned as one replayable packet. The monitor stays
     pure data and exposes only the history record, gate decision, compact
     telemetry, and next queue for norion-core scheduling.
   - Use `EvolutionAdmissionHandoffTrendContinuationPlanner` after the monitor
     when the next scheduler turn needs a durable pure-data input. The
     continuation carries the gated queue, handoff history, policy, health
     status, effective admission, promotion booleans, repair-first state, and
     telemetry so norion-core can schedule while service/eval and norion-memory
     can audit the same self-evolution boundary without executing side effects.
     Persist `EvolutionAdmissionHandoffTrendContinuationSummary` when eval only
     needs the payload-light row: health status, effective admission, promotion
     booleans, repair-first state, queue ids, queue size, and handoff-history
     depth.
     `EvolutionAdmissionHandoffTrendContinuationSummaryHistory` and its recorder
     aggregate those rows across turns so service/eval can repair on
     repair-first continuation drift, watch queue or history-depth pressure, and
     preserve clean effective-admission/adaptive-state rates before norion-core
     consumes another self-evolution queue.
     Apply `EvolutionAdmissionHandoffTrendContinuationHistoryGate` to the
     current continuation plus that history record immediately before scheduler
     consumption: stable histories keep promotion flags open, watch histories
     keep the queue observable while closing promotion/adaptive-state flags, and
     repair histories append deterministic
     `evolution-admission-handoff-trend-continuation-repair` tasks before
     service/core/memory owners advance.
     Use `EvolutionAdmissionHandoffTrendContinuationHistoryGateSummary` as the
     payload-light service/eval row after that gate; it keeps final health,
     admission/promotion flags, repair task ids, next queue ids, blocker count,
     and continuation-history depth without storing full task payloads.
   - Read `allows_service_advance` and `requires_repair_first` from toolsmith,
     tool-build report, process-reward, or evolution-admission health/history
     records before self-evolution promotes adaptive state or schedules
     tool-building work. `ToolBuildRequest::ready_requests` is only the
     Rust-ready proposal extractor; service adapters should call
     `ToolBuildRequest::admitted_by_evolution` after the final
     `EvolutionAdmissionHistoryGate` when they need the buildable request set
     that is admitted by toolsmith, process-reward, and evolution history.
     Read `can_open_tool_build_boundary`,
     `can_promote_memory_note`, `can_promote_adaptive_state`, and
     `can_finalize_eval` from `ToolBuildReportHistoryGateRecord` when the
     service/eval boundary needs explicit owner booleans. Project that record
     through `AgentAdapterBoundaryGate::from_tool_build_report_history_gate` or
     `AgentAdapterBoundarySnapshot::from_tool_build_report_history_gate` when
     the tool-build receipt close should enter the same final adapter boundary
     ledger as runtime service, memory-note, adaptive-state, and eval owners.
     Use
     `AgentAdapterBoundarySummaryHistoryRecorder::record_tool_build_report_history_gate_with_health`
     for the boundary row, `record_tool_build_report_history_gate_handoff_with_health`
     when scheduler repair tasks should be merged into the next queue, and
     `AgentAdapterBoundaryHandoffHistoryRecorder::record_tool_build_report_history_gate_with_health`
     when eval/service needs the handoff-health row appended in the same
     replayable packet.
8. Plan the next cycle.
   - `AgentCycleOrchestrator::plan_next_wave` drains ready tasks, applies
     `BudgetPolicy`, and returns an `AgentCycleDispatch`.
   - `AgentCycleOrchestrator::close_wave` accepts externally produced
     `AgentResult` values plus validation/reflection evidence, then returns an
     `AgentCycleReport`.
   - `AgentCycleOrchestrator::close_execution` accepts `AgentWaveExecution`,
     preserves execution failures in the report, and feeds the failure count
     into reward validation.
   - `AgentCycleSummary::from_report` converts the report into ledger-friendly
     counters for eval and business-cycle dashboards, including payload-free
     tool-build receipt counters for report presence, missing requests,
     unexpected receipts, duplicate receipts, held receipts, rejected receipts,
     and aggregate receipt repair pressure.
   - Persist `AgentCycleSummary` rows with
     `AgentCycleSummaryHistoryRecorder` when service/eval needs cross-cycle
     health before report-gate or business-loop admission. The dashboard tracks
     clean-cycle rate, memory-promotion rate, reward average, reward action
     mix, rejected tasks, unresolved conflicts, blocked side effects, budget
     overspends, execution failures, and tool-build receipt pressure without
     expanding the full report.
   - Read `allows_service_advance` and `requires_repair_first` from cycle
     health or history records before report-gate, memory, or adaptive-state
     consumers advance from a dirty cycle trend.
   - Reinforced cycles with clean side-effect, conflict, budget, and tool-build
     receipt gates emit `MemoryPromotion` values that wrap `MemoryNote`
     candidates.
   - `AgentCycleHandoff::from_report` packages memory notes, follow-up tasks,
     and blocked reasons for service adapters. Dirty tool-build receipts add
     explicit blocked reasons such as `tool_build_held_receipts=1`, keeping
     memory submission closed even when the reward notes are not parsed by the
     adapter.
   - `AgentCollaborationReview::from_report` projects the cycle report into the
     main-window audit row: summary counters, handoff data, memory and
     adaptive-state gate status, blocked reasons, and promotion booleans.
   - `AgentCollaborationReviewHistoryRecorder` appends that row to
     collaboration history and recomputes dashboard/health telemetry for
     cross-cycle memory admission, adaptive-state admission, blocked-review,
     blocked side-effect gate, conflict, and budget pressure.
   - `AgentCycleLedgerRecord` binds the summary, `MemorySubmissionReport`, and
     stable validation/runtime evidence refs into the row that the eval gate can
     audit.
   - `MemorySubmissionReport::summary` and `MemorySubmissionReport::gate`
     expose submitted, failed, blocked, attempted, clean, and repair-first
     evidence before the service advances the loop or records eval rows.
   - Read `allows_service_advance` and `requires_repair_first` from memory
     submission health/history records before admitting another memory or
     adaptive-state promotion boundary.
   - `AgentReportGate` turns that row into an accept/block decision with stable
     blocker codes and repair tasks. Dirty tool-build receipt counters become
     explicit `tool_build_*` blocker codes and fold into one `eval-tool-build`
     repair task before eval accepts the cycle.
   - `AgentReportGateDecision::summary` preserves accepted status, blocker-code
     order, repair lanes/roles, and blocker-family flags for service/eval
     dashboards. `memory_submission*` and `memory_promotion*` blockers both
     count as memory-family blockers and route repair work to the
     `eval-memory` lane. `tool_build_*` blockers count as their own
     tool-build family and route repair work to `eval-tool-build`, rather than
     being hidden inside generic review pressure.
   - Persist `AgentReportGateSummary` rows in
     `AgentReportGateSummaryHistory` and append them with
     `AgentReportGateHistoryRecorder` when eval needs cross-cycle acceptance,
     blocker-family, follow-up repair, and stable/watch/repair trend evidence.
   - `AgentReportGateHealthGate` feeds that trend health back into the next
     scheduler queue: stable/watch preserves queued business work, while repair
     blocks ordinary progression and adds deterministic
     `eval-report-gate-health` repair tasks before loopback or business-loop
     promotion can proceed.
   - Use `AgentReportGateHistoryRecorder::record_*_with_health_gate` when a
     service/eval adapter wants the appended history row, health, gate decision,
     merged next queue, and compact `AgentReportGateHealthGateSummary` from one
     replayable call.
   - Persist `AgentReportGateHealthGateSummary` rows in
     `AgentReportGateHealthGateSummaryHistory` and append them with
     `AgentReportGateHealthGateHistoryRecorder` when eval needs cross-cycle
     evidence about gate-admission rate, repair-first pressure, queued repair
     work, and blocked-reason pressure before another loopback boundary opens.
   - Read `allows_service_advance` and `requires_repair_first` from report-gate
     health/history records across the health-gate, trend-handoff, monitor,
     monitor-handoff, final-packet, and final-admission histories before
     loopback or service/eval accepts another report handoff.
   - `AgentReportGateHealthGateTrendGate` feeds that compact health back to the
     next scheduler queue: stable/watch keeps queued business work, while repair
     appends deterministic `eval-report-gate-health-gate-trend` repair tasks and
     blocks ordinary progression.
   - `AgentReportGateHealthGateTrendHandoff` wraps the append-and-gate sequence
     into one replayable packet when adapters need the trend record, gate
     decision, merged queue, and compact handoff summary together.
   - `AgentLoopbackPlanner` merges that decision with the service handoff into
     a next queue and an adaptive-state promotion decision.
   - `AgentLoopbackPlan::summary` exposes promotion admission, queued work,
     blocker pressure, task ids, repair lanes, and next-wave schedulability for
     service/eval rows.
   - Persist those summaries with `AgentLoopbackPlanSummaryHistoryRecorder`
     when service/eval needs trend evidence for adaptive-promotion rate,
     repair-first pressure, blocked loopbacks, repair-lane pressure, queued
     work, and next-wave schedulability before `AgentCycleLedger` or
     `AgentBusinessLoopController` admits adaptive-state promotion.
   - Read `allows_service_advance` and `requires_repair_first` from loopback
     health/history records before the business-loop controller consumes the
     next queue.
   - `AgentCycleLedger` stores the record, report decision, and loopback plan
     across cycles so promotion can use trend evidence rather than one run.
   - Persist `AgentCycleLedger::summary` rows with
     `AgentCycleLedgerSummaryHistoryRecorder` when service/eval needs dashboard
     health over ledger snapshots themselves. `AgentCycleLedgerHealth` reports
     acceptance-rate drift, reward drift, consecutive blockers,
     latest-blocked pressure, queued repair volume, tool-build report-gate
     blocker cycles, and adaptive-promotion rate before the business-loop
     controller emits service-facing commands.
   - Read `allows_service_advance` and `requires_repair_first` from ledger
     health or history records before `AgentBusinessLoopController` promotes
     adaptive state from trend evidence.
   - `AgentBusinessLoopController` packages ledger admission, the latest next
     queue, telemetry, and an optional adaptive-state candidate for the outer
     service.
   - `AgentBusinessLoopPlan::summary` exposes status, trend counters, queue
     size, candidate presence, adaptive promotion admission, repair-first
     status, tool-build blocked-cycle pressure, reasons, and evidence-ref
     counts for service/eval rows.
   - Persist those summaries with
     `AgentBusinessLoopPlanSummaryHistoryRecorder` before service command
     planning when dashboards need final in-crate control health. The companion
     health view tracks promote/hold/repair mix, adaptive-candidate admission,
     repair-first pressure, queued work, reasons, evidence refs, acceptance
     drift, and reward drift while all side effects remain outside the crate.
   - Read `allows_service_advance` and `requires_repair_first` from
     business-loop plan health/history records before service command planning
     or adaptive-state writes advance.
   - `AgentCollaborationBusinessLoopPlanner` can wrap that business-loop plan
     with collaboration dashboard/health, producing an effective
     promote/hold/repair status. It suppresses adaptive-state candidates unless
     both ledger admission and collaboration trends are stable, and emits
     collaboration repair tasks when health is repair-level.
   - `AgentCollaborationBusinessLoopPlan::summary` gives service/eval the
     compact final-admission row when it only needs statuses, promotion/repair
     booleans, candidate presence, counts, and telemetry.
   - `AgentCollaborationBusinessLoopPlan::close_service_execution` closes the
     receipt side of that collaboration-aware command boundary and returns a
     service execution report plus effective next queue and telemetry.
   - `AgentCollaborationServiceExecutionReport::summary` gives dashboards and
     eval logs the compact receipt-close row without expanding the nested
     collaboration plan or service execution report.
   - `AgentCollaborationServiceExecutionHistoryRecorder` appends those
     receipt-close summaries and returns dashboard/health telemetry for clean
     rate, dirty receipt pressure, repair-mode pressure, queued follow-up
     pressure, adaptive-promotion rate, and latest collaboration health.
   - `AgentCollaborationSelfEvolutionPlanner` combines collaboration-business
     admission with receipt-close health. Stable receipt trends preserve
     promotion, watch-level trends hold, and dirty receipt trends repair before
     adaptive evolution can continue. Its summary also carries service
     command-reason counters plus memory-promotion and tool-build
     command-reason execution pressure from the service-execution dashboard, so
     eval can explain a repair-first self-evolution decision without replaying
     command payloads.
   - `AgentCollaborationSelfEvolutionCloser` closes freshly returned service
     receipts, records the receipt-close summary, computes final admission, and
     merges current `service-feedback` repairs with trend-level collaboration
     service-execution repairs. Its close summary and close-history dashboard
     also preserve service command-reason counters plus memory-promotion and
     tool-build command-reason close pressure for final service/eval trend rows.
   - `AgentCollaborationSelfEvolutionCloseHistoryRecorder` appends the final
     close summary and recomputes dashboard/health telemetry for promotion
     rate, service-clean rate, repair-first pressure, final-repair pressure,
     blockers, and queued follow-up pressure.
    - `AgentCollaborationSelfEvolutionController` turns close-history health and
      the next queue into continue/observe/repair/idle mode, schedule admission,
      adaptive-evolution admission, reasons, and telemetry.
    - `AgentCollaborationSelfEvolutionControlHistoryRecorder` appends that
      compact control summary and recomputes dashboard/health telemetry for
      schedule rate, adaptive-admission rate, repair-first pressure, observe
      pressure, repair pressure, idle pressure, and queued work.
    - `AgentCollaborationSelfEvolutionControlMonitor` combines controller output
      and compact control-history recording when service/eval wants one
      post-close pure-data handoff.
    - `AgentCollaborationSelfEvolutionControlGate` lets that compact control
      trend constrain the current mode before service command planning: stable
      trends preserve, watch trends observe, and repair trends repair first.
    - `AgentCollaborationSelfEvolutionControlHandoff` composes the monitor and
      gate into one service/eval record with combined telemetry.
    - `AgentCollaborationSelfEvolutionServiceHandoff` composes final close
      history recording with the control handoff once service receipts have been
      closed.
    - `AgentCollaborationSelfEvolutionCloseAndAdmit` closes service receipts and
      produces the final command admission from one pure-data boundary.
    - `AgentCollaborationSelfEvolutionServiceEvalReflectionCloseContinuationPlanner`
      composes close-and-admit monitoring, packet-history recording,
      reflection-history recording, and reflection continuation into one
      persistable service/eval row.
   - `AgentClosedLoopStepper` is the one-call service helper after memory
     submission: it produces the report decision, loopback plan, appended ledger,
     and business-loop control plan.
   - `AgentServiceCommandPlanner` turns the business-loop plan into command data
     for the service executor while leaving all side effects outside the crate.
   - `AgentServiceCommandPlan::summary` exposes command count, command kinds,
     adaptive-write intent, repair/hold/enqueue intent, queued task count, and
     telemetry-command pressure before service IO runs.
   - `AgentServiceCommandReceipt` and `AgentServiceCommandAudit` capture what
     the service actually applied, failed, skipped, or forgot to run.
   - `AgentServiceCommandAudit::summary` exposes expected, received, missing,
     failed, skipped, clean, and blocked-reason evidence before feedback tasks
     are generated.
   - `AgentServiceFeedback` converts dirty command audits into repair tasks and
     a next queue for the coordinator.
   - `AgentServiceFeedback::summary` exposes audit cleanliness, repair-task
     count, repair queue size, blocker count, and feedback cleanliness.
   - `AgentServiceTurnover` merges service feedback repairs with the ordinary
     business-loop next queue through `AgentTaskQueue::with_repair_first`, so
     planned follow-ups wait for command-execution repair before becoming ready.
   - `AgentServiceTurnover::summary` exposes feedback cleanliness, merged queue
     size, repair-task pressure, blockers, and turnover cleanliness.
   - `AgentServiceExecutionReport` packages command planning, receipt audit,
     feedback, and turnover after service execution.
   - `AgentServiceExecutionReport::summary` gives service/eval the compact
     post-service row: command pressure, receipt drift, repair tasks, merged
     queue size, blockers, and clean status.
   - `AgentServiceExecutionHistory` stores those post-service rows in order and
     derives `AgentServiceExecutionDashboard` trend counters for clean rate,
     command drift, repair-task pressure, queued work, and latest blockers.
   - `AgentServiceExecutionHistory::health` and
     `AgentServiceExecutionDashboard::health` apply
     `AgentServiceExecutionHealthPolicy` to produce a service-local
     stable/watch/repair receipt-close trend before the broader closed-loop
     history is updated.
   - `AgentServiceExecutionHealthGate` turns that trend into a service/eval
     admission row. Stable and watch trends preserve the caller's next queue;
     repair trends block ordinary service command admission and merge
     deterministic `service-execution-health` repair tasks with the original
     queue. `AgentServiceExecutionHealthGateDecision::summary` keeps the same
     admission booleans, blocker list, and stable task ids without expanding the
     full queued task payload.
   - `AgentServiceExecutionHistoryRecorder` appends one summary and returns the
     updated dashboard in a single pure-data step for service/eval ledgers.
     Its `record_*_with_health` variants also return
     `AgentServiceExecutionHealthRecord` with health status and telemetry for a
     single append-and-gate row.
   - `AgentServiceExecutionHistoryRecorder::record_*_with_health_gate` is the
     one-call service/eval boundary after receipts close: it appends the
     service execution row, recomputes service-local health, applies
     `AgentServiceExecutionHealthGate`, and returns both the full merged queue
     and `AgentServiceExecutionHealthGateSummary` for persistence.
   - `AgentServiceExecutionHealthGateHistory` rolls those compact summaries
     into a dashboard for cross-turn admission rate, repair-first rate,
     repair-task pressure, queued work, latest health status, and latest
     blockers without expanding the task payload.
   - `AgentServiceExecutionHealthGateHistory::health` applies
     `AgentServiceExecutionHealthGateHealthPolicy` to that dashboard, turning
     empty history into watch, clean admission trends into stable, and
     repair-first pressure into repair before another service/eval command
     boundary opens.
   - `AgentServiceExecutionHealthGateTrendGate` can then apply that trend
     health to the next queue: stable/watch trends preserve admission, while
     repair trends block the next service command boundary and merge
     deterministic `service-execution-health-gate` repair tasks with the
     caller's queue.
   - `AgentServiceExecutionHealthGateTrendHandoff` composes the compact summary
     append, trend-health recomputation, and trend gate into one service/eval
     record for the next command boundary.
     `AgentServiceExecutionHealthGateTrendHandoffSummary` and history/dashboard
     types flatten those records into admitted rate, repair-first pressure,
     trend-health mix, repair-task pressure, queued work, and latest blockers.
     The handoff health policy and history recorder then convert that compact
     view back into stable/watch/repair evidence before service adapters attempt
     the following command boundary. The handoff monitor applies that trend to
     the current next queue, preserving stable/watch admission and appending
     deterministic `service-execution-health-gate-handoff` repair tasks when
     repair-level handoff drift must run first.
     Monitor summaries and dashboards persist the resulting admission,
     repair-first, handoff-health, queue, and blocker counters without full
     task payloads. Monitor summary history health/recorder types turn those
     compact rows into stable/watch/repair trend feedback for the next
     service/eval handoff.
   - `AgentClosedLoopExecutionReport` combines the agent-side closed-loop step
     and service-side execution report into one full-cycle audit envelope.
   - `AgentClosedLoopExecutionReport::service_summary` exposes the same compact
     service row from that envelope, so service/eval can persist the
     receipt-close row and the full-cycle row without re-reading nested fields.
   - `AgentClosedLoopExecutionSummary` compresses the full envelope into stable
     eval/dashboard counters and blocker lists.
   - `AgentClosedLoopExecutionHistory` keeps those summary rows in run order and
     derives `AgentClosedLoopExecutionDashboard` trend counters for clean rate,
     service command pressure, admission mix, queued repair volume, and latest
     blockers.
   - `AgentClosedLoopExecutionHealth` applies a policy to those counters and
     returns a stable/watch/repair status plus deterministic reasons.
   - `AgentClosedLoopNextTurnPlan` combines history health and the next queue
     into continue/observe/repair/idle scheduling intent, adaptive-evolution
     admission, telemetry, and reasons.
   - `AgentClosedLoopDispatchPreparer` turns the next-turn plan into an
     `AgentCycleDispatch` only when scheduling is allowed, preserving idle skips
     and budget-rejection audit data.
   - `AgentClosedLoopPreparedExecutor` executes only dispatchable prepared waves
     through `AgentWaveExecutor`, preserving skipped turns and engine failures
     as data.
   - `AgentClosedLoopPreparedCycleCloser` closes executed waves into
     `AgentCycleReport` values and keeps skipped turns out of the report ledger.
   - `AgentClosedLoopRuntimeTurnRunner` composes those next-turn, dispatch,
     execution, and cycle-close steps into one guarded service-facing runtime
     turn over `EnginePort`.
   - `AgentClosedLoopRuntimeBusinessTurnCloser` takes a runtime turn with a
     report through handoff, memory submission, report-gate, loopback, ledger,
     and business-loop planning.
   - `AgentClosedLoopRuntimeServiceRequestRunner` composes runtime execution,
     business-loop close, and command planning into the final crate-owned
     request before service side effects.
   - `AgentClosedLoopRuntimeServiceCommandPlanner` exposes the service command
     request that the outer service should execute and later receipt-audit.
   - `AgentClosedLoopRuntimeServiceCommandGate` records per-command side-effect
     admission before the service writes adaptive state, updates queues, or
     emits external telemetry.
   - `AgentClosedLoopRuntimeServiceDispatch` binds the command request and gate
     into the executable-or-blocked envelope for service command execution.
   - `AgentClosedLoopRuntimeServiceDispatchSummary` exposes the compact
     pre-execution row for executable status, command-gate status, side-effect
     gate counts, command kinds, and gate blockers.
   - `AgentClosedLoopRuntimeServiceReceiptIntake` validates executor receipts
     against the dispatch before they are allowed into outcome closure.
   - `AgentClosedLoopRuntimeServiceDispatchOutcome` carries the dispatch,
     receipt intake, and optional closed outcome after service execution.
   - `AgentClosedLoopRuntimeServiceIntakeRepairPlan` maps blocked intake reasons
     into `service-intake` repair tasks for the next coordinator wave.
   - `AgentClosedLoopRuntimeServiceDispatchContinuationPlanner` turns a clean
     dispatch outcome or blocked intake repair queue into the next runtime
     input.
   - `AgentClosedLoopRuntimeServiceDispatchContinuationSummary` provides the
     compact row for outcome closed, intake clean, repair task count, health,
     next queue size, and history length.
   - `AgentClosedLoopRuntimeServiceRunner` composes request dispatch,
     caller-owned receipts, intake-aware closure, and continuation planning
     without executing service side effects.
   - `AgentClosedLoopRuntimeServiceRunSummary` merges pre-execution dispatch
     counters, side-effect gate evidence, and post-execution continuation
     counters into one service/eval row.
   - `AgentClosedLoopRuntimeServiceRunStatus` labels that row as cleanly
     closed, dispatch-blocked, or intake-blocked.
   - `AgentClosedLoopRuntimeServiceRunHistory` and
     `AgentClosedLoopRuntimeServiceRunDashboard` track closed and blocked
     service-run attempts across turns.
   - `AgentClosedLoopRuntimeServiceRunHistoryRecorder` appends each completed
     service-run attempt summary and returns the updated dashboard and health.
   - `AgentClosedLoopRuntimeServiceRunHealth` turns that attempt dashboard into
     stable/watch/repair status for side-effect execution health.
   - `AgentClosedLoopRuntimeServicePreflight` combines execution health and
     service-run health before the next runtime turn is scheduled.
   - `AgentClosedLoopRuntimeServicePreflight::side_effect_admission` exposes
     that preflight mode as dispatch, memory-note, and adaptive-state gates for
     service/eval without executing them.
   - `AgentClosedLoopRuntimeServicePreflightFollowUpPlan` turns observe/repair
     preflight reasons into `service-preflight` lane tasks.
   - `AgentClosedLoopRuntimeServicePreflightContinuationPlanner` turns that
     merged preflight queue back into the next runtime input.
   - `AgentClosedLoopRuntimeServiceLoopStatePlanner` packages execution history,
     service-run attempt history, preflight continuation, and loop telemetry into
     the service-held state snapshot for the next turn.
   - `AgentClosedLoopRuntimeServiceLoopStateSummary` exposes that snapshot as a
     compact service/eval row with mode, health, schedule flags, history counts,
     follow-up volume, next queue size, reasons, and flattened side-effect
     admission gates.
   - `AgentClosedLoopRuntimeServiceLoopAdvancePlanner` composes a completed
     service run, attempt-history recording, preflight, and loop-state snapshot
     into the next service-held turn state.
   - `AgentClosedLoopRuntimeServiceLoopRunner` composes the receipt-aware
     service run and loop advance into one service-facing transition.
   - `AgentClosedLoopRuntimeServiceLoopRunSummary` exposes that transition as a
     compact service/eval row with command-gate status, side-effect gate counts,
     side-effect admission state, and next-mode evidence.
   - `AgentClosedLoopRuntimeServiceLoopRunHistory` and
     `AgentClosedLoopRuntimeServiceLoopRunDashboard` aggregate those rows across
     service-loop transitions, including command-gate allowed rate and
     side-effect gate pressure.
   - `AgentClosedLoopRuntimeServiceLoopRunHealth` evaluates that transition
     dashboard so service/eval can distinguish repair-first or intake drift
     from lower-severity watch signals.
   - `AgentClosedLoopRuntimeServiceLoopRunHistoryRecorder` appends one
     transition row and returns the updated dashboard, health, and telemetry.
   - `AgentClosedLoopRuntimeServiceLoopRunControlPlan` turns transition health
     plus the next queue into continue/observe/repair/idle admission for a
     daemon-style service loop.
   - `AgentClosedLoopRuntimeServiceLoopRunMonitor` combines transition-history
     recording and next-mode control planning after a completed loop run.
   - `AgentClosedLoopRuntimeServiceLoopRunControlSummary` compacts that monitor
     record into the flat service/eval row for dashboards and ledgers, including
     dispatch/memory/adaptive admission rates from the transition dashboard.
   - `AgentClosedLoopRuntimeServiceLoopRunControlSummaryHistory` and its
     dashboard aggregate those flat rows into daemon-control trends, including
     averaged dispatch and memory-note admission rates.
   - `AgentClosedLoopRuntimeServiceLoopRunControlHealth` turns those trends into
     stable/watch/repair gate data for the next self-evolution transition.
   - `AgentClosedLoopRuntimeServiceLoopRunControlSummaryHistoryRecorder` appends
     flat control rows and returns updated dashboard/health telemetry.
   - `AgentClosedLoopRuntimeServiceLoopRunDaemonRunner` composes the loop-runner,
     transition monitor, and flat control-history recorder when receipts are
     already available from the service-owned side-effect boundary.
   - `AgentClosedLoopRuntimeServiceLoopRunDaemonContinuation` packages the next
     runtime input, service-run history, transition history, flat control
     history, policies, admission flags, dispatch/memory admission rates, and
     health statuses for the next daemon turn.
   - `AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlanner` prepares the
     next pre-side-effect request boundary from persisted daemon continuation
     state, including dispatch/memory admission rates, before the service owns
     command execution and receipts.
   - `AgentClosedLoopRuntimeServiceLoopRunDaemonRequestRunner` runs that request
     plan only to the dispatch/expected-command boundary, and
     `AgentClosedLoopRuntimeServiceLoopRunDaemonReceiptCloser` closes returned
     receipts into the daemon record without re-running runtime ports.
   - `AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummaryHistoryRecorder`
     appends flat pre-receipt request rows and evaluates whether daemon request
     boundaries remain executable before command receipts arrive, while
     preserving dispatch/memory admission-rate evidence.
   - `AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredReceiptCloser`
     records request-boundary health and closes receipts into the daemon record
     in one service/eval persistence step.
   - `AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummary`
     compacts that close into request executable status, request health, daemon
     run status, daemon-control health, admission flags, history counters, and
     blockers.
   - `AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummaryHistory`
     and its recorder aggregate monitored-close rows into executable/closed
     rates, adaptive-admission rates, request repair pressure, daemon-control
     repair pressure, repair-first pressure, and health telemetry.
   - `AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseContinuation`
     packages the monitored continuation, monitored-close summary history,
     monitored-close health, request health, daemon-control health, next mode,
     admission flags, and telemetry after the trend row is recorded.
   - `AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredClosePlanner`
     carries that close-aware state into the next request plan while preserving
     monitored-close history and health beside request-summary history.
   - `AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseRunner`
     runs the close-aware plan to the next expected-command boundary and can
     close with receipts into the following monitored-close continuation.
   - `AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseRunSummary`
     gives eval the compact close-aware request-boundary row: executable
     status, command pressure, health states, history counters, blockers, and
     admission flags.
   - `AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredContinuation`
     packages request-summary history, request health, daemon-control health,
     admission flags, and the daemon continuation for the next boundary.
   - `AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredPlanner` builds
     the next request plan from that monitored continuation while keeping the
     request-summary history beside the plan for the next monitored close.
   - `AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredRunner` runs
     that monitored plan to the expected-command boundary and keeps the history
     attached in a monitored request record until receipts arrive.
   - `AgentClosedLoopRuntimeServiceLoopRunDaemonInputPlanner` rebuilds the next
     daemon input from persisted continuation state plus fresh service receipts,
     while preserving queue, budget, completed-task, and side-effect gates.
   - `AgentClosedLoopRuntimeServiceTurnCloser` audits service command receipts,
     produces a full execution report and summary, appends history, and returns
     the next queue.
   - `AgentClosedLoopRuntimeContinuationPlanner` converts the service turn into
     the next runtime input plus dashboard and health snapshots.
   - `AgentClosedLoopRuntimeServiceOutcomePlanner` joins command request,
     receipts, receipt audit, history update, dashboard health, and next runtime
     input when receipts are already available.
   - `AgentClosedLoopRuntimeServiceOutcomeSummary` gives service/eval a compact
     row for runtime mode, command count, receipt cleanliness, health, next
     queue size, and skipped reasons.
   - Reinforced cycles produce memory-curator follow-up tasks, hold cycles
     produce tester evidence tasks, and penalized cycles produce reviewer repair
     tasks. Unresolved conflicts keep the cycle out of reinforcement follow-up
     even when quality, validation, runtime response, and reflection are clean:
     memory notes stay empty, side-effect gates stay closed, and the next queue
     may only receive hold/repair work until a `ConflictResolutionBook` closes
     the conflicting message ids.

## Connection to norion-core

`norion-core` should remain the inference and routing contract layer. The future
adapter path is:

- Convert service input into a `norion_core::InferenceRequest`.
- Preserve model limits, `max_tokens`, profile, and experiment switches in the
  core request.
- For each accepted `AgentTask`, call a concrete `InferenceEngine` or service
  backend.
- Prefer `AgentWaveExecutor` when the service wants a single data-only helper
  around `EnginePort`.
- Convert the outcome into `AgentResult` messages:
  - route diagnostics become `Finding` or `Status` messages,
  - validation requirements become `Gate` messages,
  - model risks become `Risk` messages,
  - accepted actions become `Decision` messages.

The agent crate should not import `norion-core`; the adapter owns conversion in
the root runtime or service layer.

## Connection to norion-memory

`norion-memory` should own short-term, long-term, skills, disk KV, and
governance storage. The agent layer only proposes memory:

- `MemoryPort::recall` can feed prior lessons into task construction.
- `AgentMessageKind::MemoryNote` and `ReflectionLoop::memory_note()` propose
  lessons for later persistence.
- `MemoryPromotion` is the service-facing note candidate emitted by a reinforced
  clean cycle.
- `SideEffectKind::MemoryNote` must be allowed before `MemoryPort::propose_note`
  is called.
- `MemoryPromotionGate` is the pre-submit admission gate when the service has a
  reflection history gate, an aggregation/conflict trend gate, and persisted
  memory-submission health. It admits memory only when candidate notes exist,
  reflection is promotable, aggregation/conflict review is side-effect safe, and
  memory-submission history is stable. Watch history remains observable but
  does not auto-promote memory; empty candidate-note sets are observable no-ops
  rather than repair-first failures; repair history returns deterministic
  `memory-promotion-repair` tasks before `norion-memory` is called, even when
  the current candidate note, reflection gate, and aggregation/conflict gate are
  otherwise clean.
- `MemoryHandoffSubmitter` submits only unblocked `AgentCycleHandoff` notes and
  records `MemoryPort` failures in a `MemorySubmissionReport`.
- `MemorySubmissionReport::gate` is the post-submit adapter check: blocked
  handoffs and port failures require repair-first, while clean reports can
  continue and only report note-commit admission when a note was submitted.
- Persist `MemorySubmissionSummaryHistory` beside service/eval memory receipts
  when repeated memory failures or blocked handoffs should become trend health
  before another memory-note or adaptive-state promotion is admitted.
- Unresolved conflicts always block memory notes, even when reflection produced
  a note.
- Resolved conflicts may allow memory notes only when the resolution book covers
  the full conflict and the reflection loop completed a memory-note stage.

This keeps subagents read-only and preserves the main-window single-writer
policy from the existing `agent_team` planner.

## Connection to Service

The service loop should own IO and side effects:

- Create a run id and build tasks from active windows.
- Use `AgentCollaborationPlanner` when the service wants active-window ids,
  `AgentTaskQueue`, `BudgetLedger`, duplicate-window blockers, and telemetry in
  one packet before dispatch.
- Keep pending work in `AgentTaskQueue`; drain a ready wave, dispatch it, then
  mark completed task ids before draining the next wave.
- For larger prompts or recursive work, ask `RecursiveAgentScheduler` for
  `AgentExecutionWave` values and surface blocked task ids as coordinator repair
  work.
- Prefer `AgentCycleOrchestrator` as the service-facing facade once a loop needs
  both dispatch planning and reward-driven follow-up tasks.
- Reserve budgets before calling any model.
- Drive engine calls concurrently if the runtime is ready, but record all
  outputs into `AgentRunLedger`.
- Use `AgentRunReport` as the response envelope for coordinator status,
  blackboard summaries, and conflict stop conditions.
- Use `AgentCycleHandoff` as the final service handoff: submit memory notes only
  when `can_submit_memory()` is true, and enqueue follow-up tasks regardless of
  whether memory was blocked.
- Use `AgentCollaborationReview` when the main window needs a compact post-cycle
  view before memory or adaptive-state side effects are attempted.
- Keep an `AgentCollaborationReviewHistory` beside the longer-lived
  `AgentCycleLedger` when the service wants trend evidence that collaboration
  gates are stable before promoting adaptive state.
- Use `MemoryHandoffSubmitter` if the service wants a data-only wrapper around
  `MemoryPort::propose_note`; blocked handoffs should not call the port.
- Read the resulting `MemorySubmissionReport::summary` or
  `MemorySubmissionReport::gate` for service/eval telemetry, then convert the
  report plus validation/runtime evidence refs into `AgentCycleLedgerRecord`.
- Attach `MemoryPromotionLedgerSummary` with
  `AgentCycleLedgerRecord::with_memory_promotion_gate` before `AgentReportGate`
  runs; `NoCandidates` is a clean no-op when the cycle also reports zero memory
  promotions, so eval must not demand a `MemorySubmissionReport` for that path.
- Run `AgentReportGate` before promoting adaptive state or sending the row to
  long-lived business-loop metrics.
- Use `AgentReportGateDecision::summary` when dashboards need the accepted bit,
  blocker-code order, repair lanes, repair roles, and blocker families without
  expanding follow-up task payloads. Memory-promotion gate blockers and
  memory-submission blockers share the memory-family flag so eval trend health
  does not misclassify pre-submit memory closure as generic review pressure.
  Tool-build receipt blockers use a dedicated tool-build family so dashboards
  can separate builder drift from ordinary review blockers.
- Use `AgentReportGateHistoryRecorder` when dashboards need cross-cycle
  acceptance-rate, blocker-family, and follow-up repair trends before the
  loopback queue is admitted.
- Use `AgentLoopbackPlanner` to enqueue report-gate repair tasks ahead of
  ordinary handoff follow-ups when a report is blocked.
- Use `AgentLoopbackPlan::summary` before appending ledger state when
  service/eval needs the compact promotion, queue, blocker, repair-lane, and
  schedulability row.
- Append each completed loop to `AgentCycleLedger`; use its admission decision
  before applying long-lived adaptive-state changes.
- Use `AgentBusinessLoopController` as the final service-facing data adapter:
  it returns the next queue, telemetry, admission status, and candidate state
  promotion without applying side effects itself.
- Use `AgentBusinessLoopPlan::summary` before command planning when
  service/eval needs the compact status, trend, queue, candidate,
  repair-first, tool-build pressure, reason, and evidence-ref row.
- Use `AgentCollaborationBusinessLoopPlanner` when collaboration review history
  should gate adaptive-state promotion. A base promote can become hold while the
  collaboration trend is watch-level, or repair when conflicts, blocked reviews,
  or budget pressure recur.
- Use `AgentCollaborationBusinessLoopPlan::summary` for dashboards or eval logs
  that should not expand the nested business plan and collaboration dashboard.
- Pass `AgentCollaborationBusinessLoopPlan::effective_business_plan` to
  `AgentServiceCommandPlanner` when service commands must respect
  collaboration health. This prevents adaptive-state writes during watch/repair
  collaboration trends and enqueues collaboration repair tasks in repair mode.
- Use `AgentCollaborationBusinessLoopPlan::close_service_execution` after
  command receipts arrive when the service wants command audit, feedback,
  turnover, and collaboration telemetry in one report.
- Use `AgentCollaborationServiceExecutionReport::summary` for compact
  receipt-close logging of command kinds, adaptive-write status, promotion
  permission, repair mode, queue size, and blockers.
- Keep `AgentCollaborationServiceExecutionHistory` beside collaboration review
  history when service/eval needs to distinguish clean collaboration repair
  pressure from dirty receipt drift. Record each summary with
  `AgentCollaborationServiceExecutionHistoryRecorder`; dirty receipts default
  to repair health, while repeated clean repair-mode closures remain watch
  signals for the next self-evolution boundary.
- Use `AgentCollaborationSelfEvolutionPlanner` as the final pure-data
  admission check before adaptive evolution. It combines the
  collaboration-aware business-loop status with receipt-close health, suppresses
  adaptive-state candidates unless both are stable, and maps service execution
  repair health into `collaboration-service-execution-repair` tasks.
- Prefer `AgentCollaborationSelfEvolutionCloser` when command receipts have just
  arrived. It performs the collaboration-aware service close, records the
  service-execution history row, runs final self-evolution admission, and
  returns an effective business plan whose queue includes both current
  `service-feedback` repairs and trend-level service-execution repairs.
- Keep `AgentCollaborationSelfEvolutionCloseHistory` when service/eval needs a
  final-admission trend ledger. Record each close summary with
  `AgentCollaborationSelfEvolutionCloseHistoryRecorder`; its health output can
  gate future self-evolution on promotion rate, clean service receipts,
  repair-first pressure, final repair pressure, and blocker pressure.
- Use `AgentCollaborationSelfEvolutionController` when that final-admission
  trend should decide the next scheduler mode. It maps stable health to
  `Continue`, watch health to `Observe`, repair health to `Repair`, and an
  empty queue to `Idle`, while exposing schedule/adaptive-evolution booleans.
- Keep `AgentCollaborationSelfEvolutionControlHistory` when service/eval needs
  trend evidence for the controller's own decisions. Record each control summary
  with `AgentCollaborationSelfEvolutionControlHistoryRecorder`; stable trends
  can continue scheduling and adaptive evolution, while repair or repair-first
  pressure forces the next self-evolution slice back through repair.
- Use `AgentCollaborationSelfEvolutionControlMonitor` after a close-history
  record when service/eval wants the control plan, appended control-history
  record, and merged telemetry from one deterministic call before command
  planning.
- Use `AgentCollaborationSelfEvolutionControlGate` when service/eval wants the
  compact control trend to affect the current turn. It returns an effective mode,
  schedule/adaptive-evolution booleans, repair-first pressure, and reasons while
  keeping the original control plan available for audit.
- Use `AgentCollaborationSelfEvolutionControlHandoff` when callers want one
  post-close boundary: it records the compact control row, evaluates the gate,
  and returns both the persisted monitor evidence and effective scheduling
  decision.
- Use `AgentCollaborationSelfEvolutionServiceHandoff` after
  `AgentCollaborationSelfEvolutionCloser` when callers want close-history
  recording, control-history recording, and effective mode gating in one
  post-service pure-data packet.
- Call `AgentCollaborationSelfEvolutionServiceHandoffRecord::command_admission`
  before service command planning when the adapter needs one final boolean for
  command planning, adaptive evolution, and repair-first routing.
- Use `AgentCollaborationSelfEvolutionCloseAndAdmit` when service receipts are
  ready and the adapter wants close execution, close-history recording,
  control-history gating, and command admission in one deterministic packet.
- Read `allows_service_advance` and `requires_repair_first` from collaboration
  business-loop plans, self-evolution plans, close records, control plans,
  control monitor records, control gate decisions, service handoff records,
  close-and-admit records, or close-and-admit monitor records when service/eval
  needs the same admission signal without expanding nested histories.
- Persist `AgentCollaborationSelfEvolutionCloseAndAdmitSummary` rows in
  `AgentCollaborationSelfEvolutionCloseAndAdmitHistory` when service/eval needs
  a compact trend dashboard for command-planning pressure, idle pressure,
  adaptive admission, and repair-first routing.
- Use `AgentCollaborationSelfEvolutionCloseAndAdmitHistoryRecorder` when
  service/eval wants to append that row and recompute trend health in one step;
  empty or idle-heavy histories watch, while repair and repair-first pressure
  repair before the next command-planning turn.
- Use `AgentCollaborationSelfEvolutionCloseAndAdmitContinuationPlanner` when
  service/eval wants to turn that trend health into the next mode: continue for
  stable, observe for watch, repair-first for repair.
- Use `AgentCollaborationSelfEvolutionCloseAndAdmitMonitor` when service/eval
  wants close execution, close-and-admit trend recording, trend health, and the
  continuation plan from one post-receipt pure-data call.
- Use `AgentCollaborationSelfEvolutionServiceEvalPacket` or its planner when
  service/eval needs the monitor record projected into expected command kinds,
  dispatch readiness, adaptive-state write admission, repair-first readiness,
  blockers, and telemetry before any service command is executed.
- Keep `AgentCollaborationSelfEvolutionServiceEvalPacketHistory` when eval needs
  packet-level trends before the service executor runs commands. Its recorder
  tracks dispatch-blocked pressure, adaptive-write admission, repair-first
  readiness, latest mode/health, and command pressure.
- Use `AgentCollaborationSelfEvolutionServiceEvalPacketMonitor` when the adapter
  wants the packet, appended packet-history row, health, and merged telemetry
  from one close-and-admit monitor record.
- Use `AgentCollaborationSelfEvolutionServiceEvalReflectionPacket` after packet
  monitoring when watch/repair packet health should become explicit reflection
  focus, memory-note promotion gating, and adaptive-evolution admission evidence.
- Keep `AgentCollaborationSelfEvolutionServiceEvalReflectionHistory` when eval
  needs reflection-required pressure, memory-note promotion rate,
  adaptive-evolution admission rate, latest health, and total focus across
  service/eval turns.
- Use `AgentCollaborationSelfEvolutionServiceEvalReflectionMonitor` when the
  adapter wants reflection planning, reflection-history recording, trend health,
  and telemetry from one packet-monitor record.
- Use `AgentCollaborationSelfEvolutionServiceEvalReflectionContinuationPlanner`
  after reflection monitoring when service/eval needs the next continuation
  mode plus memory-note and adaptive-evolution admission booleans from one
  persisted pure-data plan.
- Use `AgentCollaborationSelfEvolutionServiceEvalReflectionCloseContinuationPlanner`
  when service/eval wants the full post-receipt chain in one data-only call:
  close-and-admit monitoring, packet-history recording, reflection-history
  recording, and the final reflection continuation summary.
- Keep `AgentCollaborationSelfEvolutionServiceEvalReflectionCloseContinuationHistory`
  beside packet and reflection histories when eval needs cross-turn pressure for
  continue/observe/repair mode, dispatchability, memory-note promotion,
  adaptive-evolution admission, and repair-first blockers.
- Use `AgentCollaborationSelfEvolutionServiceEvalReflectionCloseContinuationMonitor`
  when the adapter wants the close-continuation record plus appended
  close-continuation-history health from one service/eval persistence boundary.
- Use `AgentCollaborationSelfEvolutionServiceEvalReflectionCloseContinuationControlPlanner`
  after that monitor when trend health must constrain the latest mode before
  memory-note promotion or adaptive evolution is admitted.
- Keep `AgentCollaborationSelfEvolutionServiceEvalReflectionCloseContinuationControlHistory`
  when eval needs flat trend rows for requested/effective mode, dispatch
  admission, memory-note promotion, adaptive-evolution admission, repair-first
  pressure, and final control reasons.
- Use `AgentCollaborationSelfEvolutionServiceEvalReflectionCloseContinuationControlMonitor`
  when service/eval wants control planning plus appended control-history health
  from one post-reflection persistence boundary.
- Use `AgentCollaborationSelfEvolutionServiceEvalReflectionCloseContinuationControlGate`
  after that monitor when final admission must be constrained by the recorded
  control-history trend.
- Use
  `AgentCollaborationSelfEvolutionServiceEvalReflectionCloseContinuationControlAdmission`
  when the adapter wants the control-gated record projected into final dispatch,
  memory-note, adaptive-evolution, and repair-first booleans before side effects
  open.
- Keep
  `AgentCollaborationSelfEvolutionServiceEvalReflectionCloseContinuationControlAdmissionHandoffHistory`
  and its continuation history when eval needs stable/watch/repair trend rows
  over those final service/eval packets. Their health and history records expose
  `allows_service_advance` and `requires_repair_first`, so stable/watch rows can
  be observed while repair rows block memory notes and adapter commands.
- Use
  `AgentCollaborationSelfEvolutionServiceEvalReflectionCloseContinuationControlAdmissionHandoffContinuationHandoff`
  and
  `AgentCollaborationSelfEvolutionServiceEvalReflectionCloseContinuationControlAdmissionHandoffContinuationHandoffContinuation`
  when service/eval needs the handoff-continuation trend appended, gated, and
  replayed as the next pure-data boundary. Their health records expose the same
  service-advance helpers before `norion-core`, `norion-memory`, service, or
  eval opens a new writer boundary.
- Read `allows_service_advance` and `requires_repair_first` directly from the
  reflection close-continuation, control, admission, handoff, continuation,
  monitor, gate-decision, and final handoff wrapper records when adapters need
  the outer admission signal without expanding nested histories.
- Persist
  `AgentCollaborationSelfEvolutionServiceEvalReflectionCloseContinuationControlAdmissionHandoffContinuationHandoffContinuationHandoffHistory`
  when eval needs the final gate-applied packet trend before side-effect
  projection. Its health and history record expose the same helpers: watch rows
  can still be observed, stable rows advance, and repair rows force repair-first
  before memory notes or adapter commands.
- Prefer `AgentClosedLoopStepper::close` when the service has already collected
  a cycle report, handoff, validation/runtime evidence refs, memory submission
  outcome, and prior `AgentCycleLedger`; it returns the updated ledger and final
  business-loop plan together.
- Convert the returned `AgentBusinessLoopPlan` with
  `AgentServiceCommandPlanner`; execute those commands in the service layer
  according to local writer policy.
- Use `AgentServiceCommandPlan::summary` before execution when eval or operator
  UI needs command pressure, adaptive-write intent, repair/hold/enqueue intent,
  queued-task count, and telemetry-command count without expanding command
  payloads.
- Append those summaries into `AgentServiceCommandPlanSummaryHistory` before
  executing commands when service/eval needs a pre-writer trend view. Its
  dashboard and `AgentServiceCommandPlanHealthPolicy` expose empty-history,
  repair/hold, enqueue-pressure, and adaptive-write pressure reasons before
  real service side effects are attempted.
- Return one `AgentServiceCommandReceipt` for each attempted command and audit
  it with `AgentServiceCommandPlanner::audit`; missing, failed, or skipped
  commands should become coordinator-visible blocked reasons.
- Use `AgentServiceCommandAudit::summary` when eval needs expected/received
  command counts, missing/failed/skipped pressure, cleanliness, and blocked
  reasons before feedback tasks are generated.
- Append audit summaries into `AgentServiceCommandAuditSummaryHistory` when
  service/eval needs a post-receipt trend before repair tasks are generated.
  Its dashboard and `AgentServiceCommandAuditHealthPolicy` expose empty
  history, low clean-rate, receipt drift, and blocked-reason pressure as stable
  watch/repair reasons.
- Convert dirty audits with `AgentServiceFeedback::from_audit` so missing,
  failed, or skipped command execution becomes repair work in the next
  `AgentTaskQueue`.
- Use `AgentServiceFeedback::summary` when eval needs repair-task pressure and
  audit cleanliness before queue turnover.
- Append feedback summaries into `AgentServiceFeedbackSummaryHistory` when
  service/eval needs a trend of dirty-audit repair work before turnover merges
  it with the business queue. Its dashboard and
  `AgentServiceFeedbackHealthPolicy` expose empty history, low clean-rate,
  repair-task, next-queue, and blocked-reason pressure.
- Use `AgentServiceTurnover::from_feedback` to merge service-execution repair
  tasks with the business plan's next queue before scheduling the following
  wave. The returned queue carries repair task ids as dependencies of planned
  follow-ups, so scheduler waves repair service-command drift first.
- Use `AgentServiceTurnover::summary` to log the merged queue size, repair-task
  pressure, blockers, and clean status before scheduling.
- Append turnover summaries into `AgentServiceTurnoverSummaryHistory` when
  service/eval needs the final post-service queue trend before the next
  scheduler wave. Its dashboard and `AgentServiceTurnoverHealthPolicy` expose
  empty history, low clean-rate, service repair-task, blocked-reason, and merged
  next-queue pressure.
- Prefer `AgentServiceCommandPlanner::close_execution` when the service wants a
  single envelope from business plan plus command receipts to command plan,
  audit, feedback, turnover, and next queue.
- Use `AgentServiceExecutionReport::summary` for the compact service-side close
  row before combining it with the agent-side `AgentClosedLoopStep`.
- Append those rows to `AgentServiceExecutionHistory`, or use
  `AgentServiceExecutionHistoryRecorder`, when service/eval needs receipt-close
  trend counters before the broader closed-loop execution history is updated.
- Call `AgentServiceExecutionHistory::health` with
  `AgentServiceExecutionHealthPolicy` when missing/failed/skipped command
  pressure or generated service-feedback repairs should force repair-first
  handling before the full closed-loop execution trend is recomputed.
- Use `AgentServiceExecutionHistoryRecorder::record_summary_with_health` or
  `record_report_with_health` when the persistence layer wants the appended row,
  updated history, dashboard, health, and telemetry from one pure-data call.
- When the service already has an `AgentClosedLoopStep`, call
  `AgentClosedLoopStep::close_service_execution` to attach receipts and produce
  `AgentClosedLoopExecutionReport`.
- Use `AgentClosedLoopExecutionReport::service_summary` when the dashboard or
  eval writer wants the service-side close row from that combined envelope.
- Use `AgentClosedLoopExecutionReport::summary` for dashboard rows or eval rows
  that need counters instead of nested structures.
- Append execution summaries to `AgentClosedLoopExecutionHistory` and call
  `dashboard()` when the service needs a trend snapshot for operator UI,
  admission monitoring, or eval-led regression checks.
- Call `health(AgentClosedLoopExecutionHealthPolicy::default())` when the
  service needs a compact decision before another self-evolution turn:
  `Stable` continues, `Watch` keeps gathering evidence, and `Repair`
  prioritizes generated repair work.
- Build an `AgentClosedLoopNextTurnPlan` before calling the next scheduler wave:
  `Continue` can leave adaptive evolution open, `Observe` can schedule
  evidence work with adaptive evolution closed, `Repair` schedules repair work
  first, and `Idle` skips dispatch when there is no queued work.
- Use `AgentClosedLoopDispatchPreparer` to call
  `AgentCycleOrchestrator::plan_next_wave` from that turn plan. It avoids
  dispatching idle turns, preserves dispatch rejections from budget policy, and
  hands assigned task ids to the existing engine execution path.
- Use `AgentClosedLoopPreparedExecutor` before calling the model/runtime
  adapter when the service wants a guard around `EnginePort`: idle or
  non-dispatchable turns do not call the engine, and engine errors stay in
  `AgentWaveExecution.failures`.
- Use `AgentClosedLoopPreparedCycleCloser` after execution to close only real
  waves into `AgentCycleReport` values. Skipped turns remain skipped reasons;
  executed turns continue through reward scoring, memory promotion gates, and
  report-gate admission.
- Prefer `AgentClosedLoopRuntimeTurnRunner::run` when the service has a history,
  next queue, completed-task ids, budget ledger, health/budget policies,
  evidence, and an `EnginePort` adapter and wants the whole guarded runtime turn
  as one envelope with optional report, telemetry, and skipped reasons.
- Use `AgentClosedLoopRuntimeBusinessTurnCloser` when that runtime turn should
  continue into the business loop. It derives `AgentCycleHandoff`, submits
  memory via `MemoryPort`, and calls `AgentClosedLoopStepper`; skipped runtime
  turns do not call memory and do not fabricate a step.
- Use `AgentClosedLoopRuntimeServiceCommandPlanner` to expose the command plan
  before side effects are attempted. The service executes the commands under its
  own writer policy and returns receipts.
- Call `AgentClosedLoopRuntimeServiceCommandRequest::gate` before executing the
  command plan. Blocked gate entries should stop command execution and surface
  their deterministic reasons to the coordinator.
- Use `AgentClosedLoopRuntimeServiceRequestRunner::run` when the service wants
  one call that reaches the command boundary from a runtime input and business
  input. It returns the command request and prior history while leaving command
  execution outside the crate.
- Call `AgentClosedLoopRuntimeServiceRequest::command_gate` when using that
  composed request runner so the side-effect gate is audited from the same
  pre-execution envelope.
- Convert a request into `AgentClosedLoopRuntimeServiceDispatch`, or call
  `AgentClosedLoopRuntimeServiceRequestRunner::run_dispatch`, before handing
  commands to an executor. The dispatch exposes `command_plan()` only when the
  request has commands and all gate entries allow execution.
- Call `AgentClosedLoopRuntimeServiceDispatch::summary` before execution when
  service logs, operator UI, or eval rows need a compact proof of whether the
  dispatch was executable, whether the command gate allowed it, how many
  side-effect gates were checked, and which gate blockers stopped it.
- Append dispatch summaries into
  `AgentClosedLoopRuntimeServiceDispatchSummaryHistory` when service/eval needs
  a pre-execution runtime dispatch trend. Its dashboard and
  `AgentClosedLoopRuntimeServiceDispatchHealthPolicy` expose empty history, low
  executable rate, blocked dispatches, side-effect gate blockers, and blocked
  reasons before command receipts exist.
- Read dispatch history `allows_service_advance` and `requires_repair_first`
  when the adapter needs a pre-execution admission gate before service-owned
  command execution.
- After command execution, prefer
  `AgentClosedLoopRuntimeServiceDispatch::close_with_intake`. Receipt intake
  rejects receipts for blocked dispatches, unexpected command kinds, and
  duplicate command receipts before outcome closure is allowed.
- Use `AgentClosedLoopRuntimeServiceReceiptIntake::summary` when service/eval
  needs compact expected/accepted/rejected receipt counts, clean status, and
  blockers. Append those summaries into
  `AgentClosedLoopRuntimeServiceReceiptIntakeSummaryHistory` when receipt-intake
  drift should be trended before an outcome is allowed to close.
- Use receipt-intake history `allows_service_advance` and
  `requires_repair_first` as the post-executor admission signal before closing
  the dispatch outcome.
- If `AgentClosedLoopRuntimeServiceDispatchOutcome::has_outcome()` is false,
  surface `blocked_reasons()` as coordinator repair evidence instead of
  appending a service execution summary.
- Enqueue `AgentClosedLoopRuntimeServiceDispatchOutcome::repair_queue()` when
  intake is blocked. Those tasks live on the `service-intake` lane and route
  executor drift to reviewer, planner, or aggregator roles.
- Use `AgentClosedLoopRuntimeServiceDispatchContinuationPlanner` after
  `close_with_intake` when the service wants one continuation handoff. Clean
  outcomes reuse the updated history and next queue from the closed outcome;
  blocked intake keeps prior history and schedules the intake repair queue.
- Use `AgentClosedLoopRuntimeServiceDispatchContinuation::summary` when the
  service or eval layer needs compact counters for dispatch closure without
  expanding nested reports.
- Record that row with
  `AgentClosedLoopRuntimeServiceDispatchContinuationSummaryHistoryRecorder`
  when eval needs a stable trend over closed outcomes, dirty intake, repair
  pressure, and ordered continuation handoffs.
- Use continuation history `allows_service_advance` and
  `requires_repair_first` as the admission signal before routing the next
  runtime input into service-owned executors.
- Use `AgentClosedLoopRuntimeServiceRunner` when the service executor already
  returned receipts and the adapter wants one pure-data call from runtime input
  plus receipts to the next `AgentClosedLoopRuntimeTurnInput`.
- Use `AgentClosedLoopRuntimeServiceLoopRunner` when the adapter wants that
  receipt-aware run immediately advanced into updated service-run history,
  health, preflight, and a next loop-state snapshot. Receipts remain
  service-owned; engine and memory calls still go through ports.
- Read `AgentClosedLoopRuntimeServiceRun::run_summary` when the service needs a
  single eval row spanning dispatch executable status, command kinds, gate
  blockers, intake blockers, repair count, health, next queue, and history.
- Use `AgentClosedLoopRuntimeServiceRunStatus::Closed` as the only clean
  service-run state. `DispatchBlocked` means the side-effect gate stopped the
  executor; `IntakeBlocked` means returned receipts did not match the gated
  dispatch and should create intake repair work.
- Use `AgentClosedLoopRuntimeServiceRunHistoryRecorder` or
  `AgentClosedLoopRuntimeServiceLoopAdvancePlanner` when dashboards need
  attempt-level service data. This history includes blocked dispatches and
  intake drift, but it does not replace `AgentClosedLoopExecutionHistory` for
  successfully audited execution summaries.
- Call `AgentClosedLoopRuntimeServiceRunHistory::health` when the service needs
  a preflight signal for side-effect health. Empty history and low closed rate
  watch the loop; intake blockers and repair-task pressure move the service
  into repair.
- Read service-run health `allows_service_advance` and
  `requires_repair_first` when the adapter needs the attempt-level admission
  gate before combining it with execution-history health.
- Use `AgentClosedLoopRuntimeServicePreflightPlanner` before the next runtime
  turn when both execution history and service-run attempt history are
  available. It keeps idle turns idle, escalates any repair health to repair,
  downgrades watched service attempts to observe, and permits adaptive evolution
  only when both health views are stable.
- Call `AgentClosedLoopRuntimeServicePreflight::side_effect_admission` when the
  service/eval adapter needs the same preflight decision as an
  `AgentCollaborationAdapterSideEffectAdmission` row before dispatch, memory
  note promotion, or adaptive-state writes are attempted.
- Use `AgentAdapterBoundaryGate::from_runtime_service_preflight` and
  `from_runtime_service_preflight_continuation` when that preflight admission
  should be appended to adapter-boundary history. The continuation helper stores
  the merged follow-up queue, so eval can prove which service-preflight
  observation or repair tasks were handed back to `norion-core`.
- Call `AgentClosedLoopRuntimeServicePreflight::follow_up_plan` to convert
  observe and repair reasons into scheduler-visible `service-preflight` tasks.
  The returned queue keeps ordinary next-turn work while prioritizing repair
  tasks ahead of it.
- Use `AgentClosedLoopRuntimeServicePreflightContinuationPlanner` when the
  service wants the preflight merged queue packaged as the next
  `AgentClosedLoopRuntimeTurnInput` with budgets, policies, evidence,
  completed ids, and max parallelism.
- Use `AgentClosedLoopRuntimeServiceLoopAdvancePlanner` after a composed
  service run when the service wants one handoff from receipts to updated
  attempt history, service-run health, preflight follow-up tasks, and the next
  loop-state snapshot. It inherits the run's next runtime input budget,
  evidence, policy, completed ids, and max-parallel fields.
- After the service attempts the command plan, use
  `AgentClosedLoopRuntimeServiceTurnCloser` with the receipts. It creates
  `AgentClosedLoopExecutionReport`, appends the summary to history, and exposes
  the next queue, including service-execution repair tasks when receipts are
  missing, failed, or skipped.
- Use `AgentClosedLoopRuntimeContinuationPlanner` after the service turn to
  package updated history, next queue, budget policy, health policy, evidence,
  and max parallelism into the next `AgentClosedLoopRuntimeTurnInput`.
- Use `AgentClosedLoopRuntimeServiceOutcomePlanner` when command receipts are
  ready and the service wants receipt audit plus continuation planning in one
  envelope.
- Use `AgentClosedLoopRuntimeServiceOutcome::summary` when logs, dashboards, or
  eval rows need compact counters without expanding the nested command request,
  service turn, and continuation data.
- Apply file writes or state updates only after the corresponding
  `SideEffectGate` is allowed.

If `/health` or runtime state says the backend is busy, the service should hold
new `EnginePort` calls and emit status messages rather than double-send work.

## Connection to norion-eval

`norion-eval` should score the closed loop after the service has collected
evidence. `norion-agent` now owns the pure data shape and gate that the service
can pass toward eval without importing eval crates:

- Convert each `AgentCycleSummary` into an `AgentCycleLedgerRecord` with run id,
  task counts, duplicate counts, conflict counts, side-effect gate counts,
  budget overspend counts, execution failure counts, memory promotion counts,
  reward action, follow-up task counts, tool-build receipt pressure counts,
  `MemorySubmissionReport`, and `AgentReportEvidence`.
- Feed those rows into `LedgerSummary`, `ValidationGate`,
  `RuntimeResponseGate`, and service dashboards.
- Run `AgentReportGate` as the local admission rule before promotion. It blocks
  execution failures, unresolved conflicts, budget overspends, blocked side
  effects, dirty tool-build receipts, non-reinforce reward actions, low reward
  totals, missing validation/runtime evidence refs, and missing/blocked/failed
  memory submissions.
- Use blocked `AgentReportGateDecision` values to enqueue follow-up
  `AgentTask`s for tester, reviewer, planner, or memory curator roles.
- Feed accepted or blocked decisions into `AgentLoopbackPlanner`; accepted
  plans may promote adaptive state, while blocked plans keep adaptive state
  closed and return a deterministic repair queue.
- Record `AgentCycleLedgerEntry` values across runs and evaluate
  `AgentCycleLedger::admission` to promote, hold, or repair the business loop
  based on acceptance rate, average reward, latest blockers, and consecutive
  blocked cycles. Report-gate `tool_build_*` blockers roll into
  `tool_build_blocked_cycles`, so dirty builder receipts keep adaptive-state
  promotion closed until repaired.
- Use `AgentBusinessLoopPlan::can_promote_adaptive_state()` as the last local
  check before handing an `AdaptiveStateCandidate` to the service writer.
- Use `AgentClosedLoopStep` as the audit envelope when a dashboard needs to show
  exactly how a cycle moved from report to gate decision, loopback queue,
  appended ledger, and business-loop admission.
- Use `AgentServiceCommandPlan` as the eval/service boundary artifact when
  auditing why the runtime promoted adaptive state, held the loop, opened repair
  mode, enqueued tasks, or emitted telemetry.
- Use `AgentServiceCommandAudit::blocked_reasons()` as the service-execution
  feedback channel when an expected command was missing, failed, or skipped.
- Use `AgentServiceFeedback` to keep service execution failures in the same
  repair loop as report-gate and memory-submission failures.
- Use `AgentServiceTurnover` as the last queue handoff when both business
  follow-ups and service-execution repairs may exist.
- Use `AgentServiceExecutionReport` as the post-execution audit envelope in
  dashboards and eval rows; its summary keeps memory-promotion and tool-build
  command-reason counters beside receipt drift.
- Use `AgentClosedLoopExecutionReport` when the dashboard needs both the
  agent-side report-gate path and the service-side receipt path in one row.
- Use `AgentClosedLoopExecutionSummary` for stable metrics: clean status,
  reward, admission status, command result counts, next queue task ids, and
  blocked reasons.
- Use `AgentClosedLoopExecutionHistory` and
  `AgentClosedLoopExecutionDashboard` for multi-run metrics: clean rate, report
  blockers, loopback blockers, service dirty runs, missing/failed/skipped
  command pressure, promote/hold/repair admission counts, queued repair volume,
  average reward, latest run id, and latest blockers.
- Use `AgentClosedLoopExecutionHealth` as the eval-side regression gate when
  dashboard metrics should become a stable/watch/repair decision with audit
  reasons.
- Use `AgentClosedLoopNextTurnPlan` when eval and service need the health
  decision translated into scheduling intent, adaptive-evolution admission,
  service-advance admission, telemetry, and a queue to pass to the next
  scheduler turn.
- Use `AgentClosedLoopPreparedDispatch` as the eval row for the first part of
  the next cycle: it shows whether a turn was skipped, which tasks were
  assigned, and whether budget policy rejected ready work.
- Use `AgentClosedLoopPreparedExecution` to audit whether a prepared wave
  actually ran, how many results it produced, and how many engine failures must
  flow into the next cycle close.
- Use `AgentClosedLoopPreparedCycle` to distinguish an actual closed cycle from
  a skipped turn before appending anything to service/eval ledgers.
- Use `AgentClosedLoopRuntimeTurn` as the dashboard/eval envelope when the
  service runs the composed turn helper; it exposes mode, optional report,
  skipped reasons, and runtime result/failure telemetry.
- Use `AgentClosedLoopRuntimeBusinessTurn` when eval needs to see the runtime
  turn, memory submission result, optional closed-loop step, and business-loop
  telemetry in one envelope.
- Use `AgentClosedLoopRuntimeServiceCommandRequest` when eval needs the expected
  service command set before receipts arrive.
- Use `AgentClosedLoopRuntimeServiceCommandGate` when eval needs to prove a
  command was admitted before side effects were attempted. In particular,
  adaptive-state writes require the business loop to be in promote mode with a
  candidate.
- Use `AgentClosedLoopRuntimeServiceDispatch` when eval needs the exact
  executable-or-blocked handoff that the service executor saw before receipts.
- Use `AgentClosedLoopRuntimeServiceDispatchSummary` when eval only needs the
  compact pre-execution command count, command kinds, executable status,
  command-gate status, side-effect gate counts, and gate blockers.
- Use `AgentClosedLoopRuntimeServiceReceiptIntake` when eval needs to know
  whether executor receipts matched the gated dispatch.
- Use `AgentClosedLoopRuntimeServiceDispatchOutcome` when eval needs the
  intake-aware post-executor envelope and must distinguish a closed outcome from
  rejected receipts.
- Use `AgentClosedLoopRuntimeServiceIntakeRepairPlan` when eval needs the exact
  repair queue produced by blocked receipt intake.
- Use `AgentClosedLoopRuntimeServiceDispatchContinuation` when eval needs the
  next runtime input chosen after dispatch outcome handling.
- Use `AgentClosedLoopRuntimeServiceDispatchContinuationSummary` when eval only
  needs the compact outcome-closed, intake-clean, repair-count, health, queue,
  and history counters for dispatch continuation.
- Use `AgentClosedLoopRuntimeServiceRun` when eval needs the composed
  request-dispatch-intake-continuation envelope produced after service-owned
  command execution returns receipts.
- Use `AgentClosedLoopRuntimeServiceRunSummary` when eval only needs the
  compact one-row audit across pre-execution gate and post-execution receipt
  intake, including side-effect gate counts carried from dispatch.
- Use `AgentClosedLoopRuntimeServiceRunStatus` when eval dashboards need a
  stable categorical split between clean closure, pre-execution blocking, and
  receipt-intake drift.
- Use `AgentClosedLoopRuntimeServiceRunDashboard` when eval needs aggregate
  closed/blocked rates, command pressure, repair task volume, next queue volume,
  latest status, and latest blockers across service-run attempts.
- Use `AgentClosedLoopRuntimeServiceRunHistoryRecorder` when service/eval needs
  the updated attempt history, dashboard, health, and telemetry immediately
  after a `AgentClosedLoopRuntimeServiceRun`.
- Use `AgentClosedLoopRuntimeServiceRunHealth` when eval needs an
  attempt-level stable/watch/repair decision separate from execution-history
  health, including the `allows_service_advance` and `requires_repair_first`
  admission helpers.
- Use `AgentClosedLoopRuntimeServicePreflight` when eval needs the final
  service-facing next-turn mode after both execution-history health and
  service-run-attempt health are considered; read `allows_service_advance` or
  `requires_repair_first` there before expanding the admission gate row.
- Use `AgentClosedLoopRuntimeServicePreflightFollowUpPlan` when eval or service
  logs need to prove which preflight reasons became repair or observation
  tasks.
- Use `AgentClosedLoopRuntimeServicePreflightContinuation` when eval needs to
  inspect the exact runtime input chosen after preflight follow-up planning.
- Use `AgentClosedLoopRuntimeServiceLoopState` when eval or service needs the
  persisted loop snapshot: execution history, service-run attempt history,
  preflight continuation, next runtime input, and telemetry in one row.
- Use `AgentClosedLoopRuntimeServiceLoopStateSummary` when eval only needs the
  compact loop-state row for dashboards or service logs, including preflight
  side-effect admission health and dispatch/memory/adaptive gate booleans.
- Use `AgentClosedLoopRuntimeServiceLoopAdvance` when eval needs the composed
  post-run record: appended service-run history, loop state, compact summary,
  and telemetry from the same transition. Loop-state and loop-advance wrappers
  mirror the preflight service-advance helpers.
- Use `AgentClosedLoopRuntimeServiceLoopRun` when eval needs both the
  receipt-aware service run and the advanced loop-state record from a single
  service adapter transition.
- Use `AgentClosedLoopRuntimeServiceLoopRunSummary` when eval only needs the
  compact transition row: service-run status, blockers, preflight mode, health,
  attempts, repair-first flag, follow-up count, and next queue size.
- Use `AgentClosedLoopRuntimeServiceLoopRunHistory` when eval needs transition
  trends such as closed rate, repair-first rate, adaptive-admission rate,
  command pressure, follow-up task volume, and latest blockers.
- Use `AgentClosedLoopRuntimeServiceLoopRunHealth` when eval needs a
  transition-level stable/watch/repair decision before the next daemon-style
  service-loop transition.
- Use `AgentClosedLoopRuntimeServiceLoopRunHistoryRecorder` when service/eval
  wants to persist the transition row and health snapshot immediately after a
  `AgentClosedLoopRuntimeServiceLoopRun`.
- Use `AgentClosedLoopRuntimeServiceLoopRunController::plan` when service/eval
  needs a pure-data decision for whether the next service-loop transition may
  schedule, must observe, must repair first, or should idle; the plan exposes
  service-advance helpers without opening any side-effect boundary.
- Use `AgentClosedLoopRuntimeServiceLoopRunMonitor::record_and_plan` when the
  service has a completed loop transition and wants one record containing the
  appended transition history, health, next control plan, and telemetry. The
  returned control record delegates service-advance helpers to that plan.
- Use `AgentClosedLoopRuntimeServiceLoopRunControlRecord::summary` when service
  logs or eval ledgers need the compact latest-status, next-mode, transition
  rate, queue pressure, admission, and reason row.
- Use `AgentClosedLoopRuntimeServiceLoopRunControlSummaryHistory` when eval or
  operator dashboards need multi-transition daemon-control pressure without
  expanding monitor records.
- Use `AgentClosedLoopRuntimeServiceLoopRunControlHealth` when service/eval
  needs a stable/watch/repair gate from daemon-control trends before scheduling
  another self-evolution transition.
- Use `AgentClosedLoopRuntimeServiceLoopRunControlSummaryHistoryRecorder` when
  service/eval wants to append a flat daemon-control row and receive the
  updated dashboard, health gate, and telemetry in one data-only record.
- Use `AgentClosedLoopRuntimeServiceLoopRunDaemonRunner` when service/eval wants
  one data-only daemon transition record containing the completed loop run,
  transition monitor record, compact control summary, appended flat control
  history, health gates, next runtime input, and telemetry. The daemon record
  also exposes the same service-advance helper pair for adapter admission.
- Use `AgentClosedLoopRuntimeServiceLoopRunDaemonContinuationPlanner` when
  service/eval wants to persist the next daemon state or feed it into a later
  side-effect boundary without expanding nested record fields.
- Use `AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlanner` when service
  needs to expose the next request/command boundary before receipts exist; the
  resulting request plan can later be materialized with those receipts.
- Use `AgentClosedLoopRuntimeServiceLoopRunDaemonRequestRunner` when service
  needs a request record containing dispatch gate status, expected commands,
  skipped reasons, and telemetry before executing commands. Use
  `AgentClosedLoopRuntimeServiceLoopRunDaemonReceiptCloser` after receipts
  arrive to produce the daemon record without replaying the runtime request.
- Use `AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummaryHistoryRecorder`
  when service/eval needs a receipts-before/receipts-after split: request
  summaries catch skipped or blocked expected-command boundaries before command
  execution happens.
- Use `AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredReceiptCloser`
  when service/eval wants one record containing request-summary health and the
  post-receipt daemon transition result. Its close record mirrors daemon
  service-advance helpers before the next monitored request is planned.
- Use `AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseRecord::summary`
  when dashboards or eval ledgers need the compact row for request health,
  daemon run status, daemon-control health, history pressure, and blockers.
- Use `AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummaryHistoryRecorder`
  when service/eval wants to append monitored-close summary rows and evaluate
  cross-turn request/daemon-control pressure before opening the next boundary.
- Use `AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseContinuationPlanner`
  when service/eval wants the post-recording state packet that carries both
  request-boundary history and monitored-close health into the next boundary.
- Use `AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredClosePlanner`
  when service/eval wants to build the next request boundary from that packet
  without dropping monitored-close summary history.
- Use `AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseRunner`
  when service/eval wants a close-aware request record that can close with
  receipts and append the next monitored-close trend row.
- Use
  `AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseRunRecord::summary`
  when eval needs the compact close-aware request-boundary row without walking
  the nested plan and monitored request record.
- Use `AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredContinuation`
  when service/eval wants to persist request-boundary history and health beside
  the daemon continuation for the next loop.
- Use `AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredPlanner` when
  the next request boundary should be built from that monitored state without
  dropping the request-summary history needed by the following monitored close.
- Use `AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredRunner` when
  service/eval wants to run that boundary and keep request history attached
  until the record is closed with receipts.
- Use `AgentClosedLoopRuntimeServiceLoopRunDaemonInputPlanner` when service/eval
  wants to turn persisted daemon continuation state and newly collected receipts
  into the next full daemon input plus ledger telemetry.
- Use `AgentClosedLoopRuntimeServiceRequest` when eval needs to inspect the
  composed pre-side-effect boundary, including prior history, runtime/business
  telemetry, skipped reasons, and expected commands.
- Use `AgentClosedLoopRuntimeServiceTurn` when eval needs the final service-side
  receipt audit, updated execution history, summary row, and next queue from the
  same composed turn.
- Use `AgentClosedLoopRuntimeContinuation` when eval needs the post-turn
  dashboard, health decision, telemetry, and exact next runtime input.
- Use `AgentClosedLoopRuntimeServiceOutcome` when eval needs the command
  request, service receipt audit, continuation, and next runtime input in one
  row.
- Use `AgentClosedLoopRuntimeServiceOutcomeSummary` when eval only needs the
  compact post-service row: runtime mode, command count, service executed/clean,
  health status, next queue size, skipped reasons, and telemetry.
- Promote reusable lessons only when eval and `ClosedLoopRewarder` both allow a
  reinforce-oriented action, the report gate accepts the ledger record, and the
  multi-cycle ledger admission is not in repair mode.

The intended loop is:

```text
service request
  -> AgentTask queue
  -> RecursiveAgentScheduler waves
  -> AgentCycleOrchestrator::plan_next_wave
  -> DispatchPlanner + BudgetLedger
  -> AgentWaveExecutor over EnginePort adapter
  -> AgentRunLedger
  -> MessageAggregator + ConflictResolver
  -> ConflictResolutionBook
  -> ReflectionLoop
  -> SideEffectGate
  -> ClosedLoopRewarder + ToolsmithPlan
  -> AgentCycleOrchestrator::close_execution
  -> AgentCycleSummary ledger counters
  -> MemoryPromotion candidates
  -> MemoryPromotionGate
  -> AgentCycleHandoff service handoff
  -> MemoryHandoffSubmitter over MemoryPort
  -> MemorySubmissionReport
  -> MemorySubmissionSummary + MemorySubmissionGateDecision
  -> AgentCycleLedgerRecord + AgentReportEvidence
  -> AgentReportGate decision
  -> AgentReportGateSummary
  -> AgentLoopbackPlanner
  -> AgentLoopbackPlanSummary
  -> AgentCycleLedger admission
  -> AgentBusinessLoopController plan
  -> AgentBusinessLoopPlanSummary
  -> AgentCollaborationBusinessLoopPlanner effective admission
  -> AgentCollaborationBusinessLoopPlan::effective_business_plan
  -> AgentCollaborationBusinessLoopPlan::close_service_execution
  -> AgentCollaborationServiceExecutionReport::summary
  -> AgentCollaborationServiceExecutionHistoryRecorder
  -> AgentCollaborationSelfEvolutionPlanner final admission
  -> AgentCollaborationSelfEvolutionCloser receipt boundary
  -> AgentCollaborationSelfEvolutionCloseHistoryRecorder
  -> AgentCollaborationSelfEvolutionController next mode
  -> AgentCollaborationSelfEvolutionControlHistoryRecorder
  -> AgentCollaborationSelfEvolutionControlMonitor optional combined handoff
  -> AgentCollaborationSelfEvolutionControlGate effective mode
  -> AgentCollaborationSelfEvolutionControlHandoff one-call boundary
  -> AgentCollaborationSelfEvolutionServiceHandoff post-service boundary
  -> AgentCollaborationSelfEvolutionCloseAndAdmit receipt-to-admission boundary
  -> AgentCollaborationSelfEvolutionServiceEvalReflectionCloseContinuation service/eval row
  -> AgentClosedLoopStepper envelope
  -> AgentServiceCommandPlanner commands
  -> AgentServiceCommandPlanSummary
  -> AgentServiceCommandReceipt audit
  -> AgentServiceCommandAuditSummary
  -> AgentServiceFeedback repair queue
  -> AgentServiceFeedbackSummary
  -> AgentServiceTurnover merged queue
  -> AgentServiceTurnoverSummary
  -> AgentServiceExecutionReport
  -> AgentServiceExecutionReportSummary
  -> AgentClosedLoopExecutionReport
  -> AgentClosedLoopExecutionSummary
  -> AgentClosedLoopExecutionHistory dashboard
  -> AgentClosedLoopExecutionHealth
  -> AgentClosedLoopNextTurnPlan
  -> AgentClosedLoopDispatchPreparer
  -> AgentClosedLoopPreparedExecutor
  -> AgentClosedLoopPreparedCycleCloser
  -> AgentClosedLoopRuntimeTurnRunner
  -> AgentClosedLoopRuntimeBusinessTurnCloser
  -> AgentClosedLoopRuntimeServiceRequestRunner
  -> AgentClosedLoopRuntimeServiceCommandPlanner
  -> AgentClosedLoopRuntimeServiceCommandGate
  -> AgentClosedLoopRuntimeServiceDispatch
  -> AgentClosedLoopRuntimeServiceDispatchSummary
  -> AgentClosedLoopRuntimeServiceReceiptIntake
  -> AgentClosedLoopRuntimeServiceDispatchOutcome
  -> AgentClosedLoopRuntimeServiceIntakeRepairPlan
  -> AgentClosedLoopRuntimeServiceDispatchContinuationPlanner
  -> AgentClosedLoopRuntimeServiceDispatchContinuationSummary
  -> AgentClosedLoopRuntimeServiceRunner
  -> AgentClosedLoopRuntimeServiceRunSummary
  -> AgentClosedLoopRuntimeServiceRunStatus
  -> AgentClosedLoopRuntimeServiceRunHistory
  -> AgentClosedLoopRuntimeServiceRunHistoryRecorder
  -> AgentClosedLoopRuntimeServiceRunDashboard
  -> AgentClosedLoopRuntimeServiceRunHealth
  -> AgentClosedLoopRuntimeServicePreflight
  -> AgentClosedLoopRuntimeServicePreflightFollowUpPlan
  -> AgentClosedLoopRuntimeServicePreflightContinuationPlanner
  -> AgentClosedLoopRuntimeServiceLoopStatePlanner
  -> AgentClosedLoopRuntimeServiceLoopStateSummary
  -> AgentClosedLoopRuntimeServiceLoopAdvancePlanner
  -> AgentClosedLoopRuntimeServiceLoopRunner
  -> AgentClosedLoopRuntimeServiceLoopRunSummary
  -> AgentClosedLoopRuntimeServiceLoopRunHistory
  -> AgentClosedLoopRuntimeServiceLoopRunDashboard
  -> AgentClosedLoopRuntimeServiceLoopRunHealth
  -> AgentClosedLoopRuntimeServiceLoopRunHistoryRecorder
  -> AgentClosedLoopRuntimeServiceLoopRunControlPlan
  -> AgentClosedLoopRuntimeServiceLoopRunMonitor
  -> AgentClosedLoopRuntimeServiceLoopRunControlSummary
  -> AgentClosedLoopRuntimeServiceLoopRunControlSummaryHistory
  -> AgentClosedLoopRuntimeServiceLoopRunControlDashboard
  -> AgentClosedLoopRuntimeServiceLoopRunControlHealth
  -> AgentClosedLoopRuntimeServiceLoopRunControlSummaryHistoryRecorder
  -> AgentClosedLoopRuntimeServiceLoopRunDaemonRunner
  -> AgentClosedLoopRuntimeServiceLoopRunDaemonContinuationPlanner
  -> AgentClosedLoopRuntimeServiceLoopRunDaemonRequestPlanner
  -> AgentClosedLoopRuntimeServiceLoopRunDaemonRequestRunner
  -> AgentClosedLoopRuntimeServiceLoopRunDaemonRequestSummaryHistoryRecorder
  -> AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredReceiptCloser
  -> AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummary
  -> AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseSummaryHistoryRecorder
  -> AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseContinuationPlanner
  -> AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredClosePlanner
  -> AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredCloseRunner
  -> AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredContinuation
  -> AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredPlanner
  -> AgentClosedLoopRuntimeServiceLoopRunDaemonRequestMonitoredRunner
  -> AgentClosedLoopRuntimeServiceLoopRunDaemonReceiptCloser
  -> AgentClosedLoopRuntimeServiceLoopRunDaemonInputPlanner
  -> AgentClosedLoopRuntimeServiceTurnCloser
  -> AgentClosedLoopRuntimeContinuationPlanner
  -> AgentClosedLoopRuntimeServiceOutcomePlanner
  -> AgentClosedLoopRuntimeServiceOutcomeSummary
  -> norion-eval/service ledger
  -> next AgentTask queue
```

## Current Verification

Run the isolated crate tests with:

```powershell
cargo test --manifest-path crates\norion-agent\Cargo.toml
```

The important behavior now covered is:

- per-role budget exhaustion rejects without cross-role debit,
- run budget audit reports result spending that exceeds reserved dispatch
  budget,
- wave execution preserves dispatch order and reports engine failures as data,
- cycle close preserves execution failure counts and blocks memory promotion
  when a wave did not fully execute,
- strict budget policy can reject zero-budget tasks,
- recursive scheduling builds stable dependency waves and leaves cycles blocked,
- multiple agent results merge in deterministic dispatch order,
- unresolved conflicts block memory note side effects,
- audited conflict resolutions can unblock memory notes after completed
  reflection,
- reflection failure paths do not advance the loop.
- closed-loop reward scoring reinforces clean validated runs and penalizes
  unresolved conflict or failed validation paths.
- cycle orchestration plans ready dispatch waves and turns reward outcomes into
  concrete follow-up tasks.
- cycle summaries expose eval-friendly counters and identify whether memory
  promotion is currently allowed.
- clean reinforced cycles emit memory promotion candidates without persisting
  them directly.
- cycle handoffs package memory notes, follow-up tasks, and blocked reasons for
  future service adapters.
- memory handoff submission preserves blocked and port-error outcomes as data
  without implementing storage inside `norion-agent`.
- report gate records accept/block decisions with deterministic blocker order
  and follow-up task generation for validation, review, budget, and memory
  repair paths.
- report gate health gates preserve stable/watch next queues, but convert
  repair-level eval trends into deterministic `eval-report-gate-health` repair
  tasks before loopback or adaptive-state promotion can advance.
- report gate health gate records give service/eval one append-and-gate row
  with health, admission, repair-first, queue ids, repair task ids, and blocker
  pressure without persisting full task payloads.
- report gate health gate summary histories roll those compact rows into
  stable/watch/repair health for eval dashboards, letting the next boundary see
  whether report-gate admission itself is drifting.
- report gate health gate trend gates convert that dashboard health back into
  queue admission, so repeated repair-first report-gate pressure is repaired
  before adaptive-state promotion or loopback progression resumes.
- report gate health gate trend handoffs package the compact trend append and
  trend gate into one service/eval packet, avoiding adapter-side sequencing
  drift while still keeping all side effects outside `norion-agent`.
- report gate health gate trend handoff histories persist those packets as
  payload-free eval rows. Dashboards aggregate admitted, repair-first,
  trend-health, queue-pressure, and blocked-reason counts, then recompute
  stable/watch/repair health for the next service/eval boundary without
  replaying full task queues.
- report gate health gate trend handoff gates apply that persisted health to
  the next queue: stable/watch histories preserve current admission, while
  repair histories append deterministic
  `eval-report-gate-health-gate-trend-handoff` repair tasks and block further
  memory-note or adaptive-state side effects.
- report gate health gate trend handoff monitors compose the compact handoff
  row append and final handoff-history gate into one replayable service/eval
  packet. Monitor summaries expose requested admission, effective admission,
  repair-first pressure, queued work, repair task ids, and blocker pressure
  without replaying nested gate records.
- report gate health gate trend handoff monitor histories aggregate those
  monitor summaries into a payload-free dashboard. Eval can observe requested
  versus effective admission, monitor-health mix, repair task pressure, queued
  work, and blockers across cycles before allowing another memory-note or
  adaptive-state boundary to open.
- report gate health gate trend handoff monitor gates apply that cross-cycle
  monitor health back to queue admission. Stable/watch histories preserve the
  current monitor decision, while repair histories append deterministic
  `eval-report-gate-health-gate-trend-handoff-monitor` repair tasks and close
  effective admission before service/eval opens another side-effect boundary.
- report gate health gate trend handoff monitor handoffs package that
  monitor-summary append plus monitor-health gate into one service/eval packet.
  Their summaries are the compact rows for requested admission, effective
  admission, repair-first state, monitor-history depth, repair tasks, queued
  work, and blockers after the final monitor gate has been applied.
- report gate health gate trend handoff monitor handoff histories persist those
  final packet rows as payload-free dashboard evidence for requested admission,
  effective admission, monitor-health mix, repair-first pressure, repair work,
  queued work, and blockers before another memory-note, adaptive-state, service
  command, or eval boundary opens.
- report gate health gate trend handoff monitor handoff gates apply that final
  packet trend health to the next queue. Stable/watch histories keep the packet
  admitted; repair histories append deterministic
  `eval-report-gate-health-gate-trend-handoff-monitor-handoff` repair work and
  close ordinary side-effect admission.
- report gate health gate trend handoff monitor handoff handoffs compose the
  final packet summary append and final packet trend gate into one replayable
  service/eval record. Their summaries are the compact rows for final
  monitor-handoff health, requested/effective admission, repair-first state,
  queue ids, repair ids, and blocker pressure.
- report gate health gate trend handoff monitor handoff handoff histories
  persist those final packet summaries as dashboard health for admission rate,
  final-packet health mix, repair-first pressure, repair task pressure, queue
  pressure, and blockers before the next closed-loop service/eval boundary is
  admitted.
- report gate health gate trend handoff monitor handoff handoff gates apply
  that final packet dashboard health to the next queue. Stable/watch histories
  keep the queue admitted; repair histories append deterministic
  `eval-report-gate-health-gate-trend-handoff-monitor-handoff-handoff` repair
  work and keep side-effect admission closed.
- report gate health gate trend handoff monitor handoff handoff handoffs compose
  the final packet trend append and trend gate into one replayable admission
  record. Their summaries expose final packet health, requested/effective
  admission, repair-first state, queue ids, repair ids, and blocker pressure
  without expanding nested monitor-handoff payloads.
- loopback planning keeps adaptive-state promotion closed until the report gate
  accepts the cycle, and merges gate repairs plus handoff follow-ups into the
  next queue without duplicate task ids.
- multi-cycle ledger summaries track acceptance rate, average reward,
  consecutive blocked cycles, latest blockers, queued repair tasks, and
  promote/hold/repair admission.
- business-loop control packages a promote/hold/repair admission, next queue,
  telemetry, and adaptive-state candidate while keeping actual state writes in
  the service layer.
- collaboration-aware service execution histories aggregate receipt-close
  summaries into clean/dirty, blocked, repair-mode, queued-follow-up, adaptive
  promotion, and latest-health trends.
- collaboration review, service-execution, and self-evolution close history
  records expose `latest()`, `records()`, `allows_service_advance`, and
  `requires_repair_first` as stable service/eval read-model helpers.
- collaboration service-execution health keeps dirty receipts in repair mode
  while clean repair-mode pressure remains a watch signal for self-evolution
  admission.
- collaboration self-evolution planning admits adaptive-state promotion only
  when collaboration business-loop health and receipt-close health are both
  stable, and turns dirty service history into deterministic repair tasks.
- collaboration self-evolution close records merge freshly generated
  `service-feedback` repairs with trend-level service-execution repairs before
  returning the next effective business-loop queue.
- collaboration self-evolution close histories aggregate final admission rows
  into promotion, service-clean, dirty-receipt, repair-first, final-repair,
  blocker, and queue-pressure health signals.
- collaboration self-evolution control plans translate final-admission health
  plus the next queue into continue/observe/repair/idle mode, scheduling
  admission, and adaptive-evolution admission.
- collaboration self-evolution control histories aggregate those next-mode rows
  into stable/watch/repair trend evidence so service/eval can block adaptive
  evolution when controller outputs drift into repair-first, repair, observe, or
  idle pressure.
- collaboration self-evolution control history records expose
  `allows_service_advance` and `requires_repair_first` so service/eval can reuse
  the same admission signal before command planning.
- collaboration self-evolution control monitors package the current control
  decision with the updated compact control-history ledger without executing
  commands or memory writes.
- collaboration self-evolution control gates feed that compact control-health
  trend back into the current turn, preserving stable continues, observing watch
  pressure, and forcing repair-first behavior when the controller trend is dirty.
- collaboration self-evolution control handoffs combine the monitor and gate so
  service/eval can persist control evidence and use the effective mode from one
  pre-command pure-data record.
- collaboration self-evolution service handoffs combine final close-history
  persistence with control-history gating so the post-service boundary can feed
  the next command planner with a single effective-mode packet.
- self-evolution service command admission projects that packet into explicit
  pre-command booleans, keeping idle/repair/adaptive decisions out of service
  adapter conditionals.
- self-evolution close-and-admit records start at returned service receipts and
  end at those command-admission booleans, preserving every intermediate
  close-history and control-history record for audit.
- close-and-admit histories aggregate those records into command-planning,
  adaptive-admission, idle, repair, and repair-first trend counters for eval.
  Their summaries and history dashboards also carry service command-reason
  counters and memory-promotion command-reason close pressure before service
  command admission.
- close-and-admit history recorders attach stable/watch/repair health to those
  counters, turning idle and repair pressure into explicit pre-command evidence.
- close-and-admit history records expose `allows_service_advance` and
  `requires_repair_first` before the continuation planner chooses the next
  command-admission mode.
- close-and-admit continuation plans project that health into deterministic
  continue/observe/repair command-admission modes for the next service turn.
- close-and-admit monitors package receipt closure, trend recording, and
  continuation planning together while still leaving command execution outside
  `norion-agent`.
- service/eval packets flatten close-and-admit monitor records into expected
  service command pressure and pre-execution dispatch admission, including
  adaptive-state and repair-first blockers, while preserving service
  command-reason counters and memory-promotion command-reason close pressure.
- service/eval packet histories aggregate that pre-execution admission into
  dispatch, adaptive-write, repair-first, latest-mode, latest-health, and
  command-pressure dashboards. Packet health records expose
  `allows_service_advance` and `requires_repair_first` so service/eval can
  advance or repair without rehydrating close-and-admit records.
- reflection summaries and histories keep the same service command-reason
  counters and memory-promotion command-reason close pressure beside memory-note
  promotion admission.
  admission.
  observe watch pressure while blocking repair-first packet drift before
  adapters run.
- service/eval packet monitors compose packet planning and packet-history
  recording after close-and-admit monitoring, leaving command execution outside
  the crate.
- service/eval reflection packets turn packet health and blockers into a
  service-owned reflection gate before memory notes or adaptive evolution are
  promoted.
- service/eval reflection histories aggregate those reflection gates into
  trend health so repeated reflection pressure can watch or repair before
  promotion continues. Reflection health records expose the same
  service-advance helpers, keeping stable/watch rows replayable while making
  repair rows an explicit pre-memory-note blocker.
- service/eval reflection monitors compose reflection planning and trend
  recording after packet monitoring, preserving the no-side-effect boundary.
- service/eval reflection continuations translate reflection trend health into
  continue/observe/repair mode before memory notes or adaptive evolution are
  promoted.
- service/eval reflection close-continuations package close-and-admit,
  packet-monitor, reflection-monitor, and continuation evidence into one
  persistable row while preserving the same no-side-effect boundary.
- service/eval reflection close-continuation histories aggregate those rows into
  dashboard health for continue, observe, repair, promotion, adaptive-admission,
  dispatch, repair-first pressure, service command-reason counters, and
  memory-promotion command-reason close pressure. Their health records expose
  `allows_service_advance` and `requires_repair_first`, matching packet and
  reflection read models for stable/watch replay and repair-first blocking.
- service/eval reflection close-continuation monitors compose row planning and
  history recording so adapters can persist the boundary without hand-assembling
  nested packet and reflection monitors.
- service/eval reflection close-continuation control plans feed cross-turn
  close-continuation health back into the latest row, preserving stable modes,
  observing watch pressure, and forcing repair-first on repair trends.
- service/eval reflection close-continuation control histories aggregate those
  final gates into dashboard health for the next self-evolution admission,
  including service command-reason counters and memory-promotion command-reason
  close pressure. Their health records expose the same service-advance helpers,
  preserving watch control rows while making repair control trends repair-first
  blockers.
- service/eval reflection close-continuation control monitors combine the final
  gate and flat control-history recording so service/eval does not hand-assemble
  control persistence.
- service/eval reflection close-continuation control gates apply that recorded
  control-history health to the next admission, preserving stable, observing
  watch pressure, and forcing repair-first on repair pressure.
- service/eval reflection close-continuation control handoffs compose the
  control monitor and recorded-health gate so adapters can persist and admit the
  next service/eval boundary from one pure-data call.
- service/eval reflection close-continuation control admissions project that
  handoff into final dispatch, memory-note, adaptive-evolution, and repair-first
  side-effect booleans before service or memory adapters execute anything.
- service/eval reflection close-continuation control admission summaries compact
  those final booleans into eval rows without requiring consumers to expand the
  full monitor, gate, or handoff record, while preserving service
  command-reason counters and memory-promotion command-reason close pressure.
- service/eval reflection close-continuation control admission histories expose
  the service-advance helpers on final side-effect admission trend health, so
  eval can replay stable/watch rows and stop repair rows before adapters run
  with the same memory-promotion pressure evidence.
- service/eval reflection close-continuation control admission histories
  aggregate those final admission rows into stable/watch/repair health before
  service or memory adapters open another side-effect boundary.
- service/eval reflection close-continuation control admission monitors combine
  handoff admission projection and admission-history recording into one
  persistable boundary with merged telemetry.
- service/eval reflection close-continuation control admission gates apply
  recorded admission health to the current side-effect boundary, preserving
  stable, observing watch pressure, and forcing repair-first on repair pressure.
- service/eval reflection close-continuation control admission handoffs compose
  admission monitoring and recorded-health gating into one final service/eval
  packet for side-effect admission.
- service/eval reflection close-continuation control admission handoff summaries
  compact that final packet into gate-applied eval rows for downstream ledgers,
  preserving service command-reason counters and memory-promotion command-reason
  close pressure from the recorded admission dashboard.
- service/eval reflection close-continuation control admission handoff histories
  aggregate gate-applied final packets into stable/watch/repair trend health and
  expose the service-advance helpers for replaying stable/watch final packets or
  stopping repair-first handoff drift with the same memory-promotion pressure
  evidence.
- service/eval reflection close-continuation control admission handoff monitors
  compose final packet recording and handoff trend health into one persistable
  service/eval boundary without executing service commands or writing memory.
- service/eval reflection close-continuation control admission handoff
  continuations carry that final trend health into the next boundary's
  continue/observe/repair mode while leaving service and memory side effects in
  the adapters.
- service/eval reflection close-continuation control admission handoff
  continuation records package the persisted monitor row with that next-mode
  decision so service/eval can store and replay the boundary without expanding
  nested handoff histories.
- service/eval reflection close-continuation control admission handoff
  continuation summaries and histories flatten those next-mode decisions into
  dashboard health for continue, observe, repair, memory-note, adaptive, and
  repair-first pressure across service/eval turns, while preserving service
  command-reason counters and memory-promotion command-reason close pressure and
  exposing the service-advance helpers on the recorded continuation trend.
- service/eval reflection close-continuation control admission handoff
  continuation monitors append those rows and recompute continuation health in
  one pure-data record for service/eval persistence.
- service/eval reflection close-continuation control admission handoff
  continuation gates apply recorded continuation health to the next boundary,
  preserving stable continuation, observing watch pressure, and forcing
  repair-first before service or memory adapters run.
- service/eval reflection close-continuation control admission handoff
  continuation handoffs compose continuation monitoring and trend gating into
  one service/eval packet.
- service/eval reflection close-continuation control admission handoff
  continuation handoff summaries and histories aggregate those gate-applied
  packets into stable/watch/repair dashboard health while preserving service
  command-reason counters and memory-promotion command-reason close pressure.
- service/eval reflection close-continuation control admission handoff
  continuation handoff monitors append gate-applied packet rows and recompute
  health in one service/eval persistence record.
- service/eval reflection close-continuation control admission handoff
  continuation handoff continuations translate that recorded handoff health
  into the next boundary's continue/observe/repair mode, preserving stable
  dispatch and memory-note admission while forcing repair-first on dirty
  trends.
- service/eval reflection close-continuation control admission handoff
  continuation handoff continuation histories and monitors flatten those
  next-mode decisions into dashboard health so service/eval can persist the
  replayable boundary without expanding nested handoff monitors, preserving the
  same service command-reason counters and memory-promotion command-reason close
  pressure.
- service/eval reflection close-continuation control admission handoff
  continuation handoff continuation gates apply that recorded continuation
  health to the next side-effect boundary.
- service/eval reflection close-continuation control admission handoff
  continuation handoff continuation handoffs compose continuation monitoring
  and recorded-health gating into one replayable service/eval packet.
- service/eval reflection close-continuation control admission handoff
  continuation handoff continuation handoff summaries, histories, and monitors
  flatten that gate-applied packet into trend health for dashboards and ledgers,
  preserving service command-reason counters and memory-promotion command-reason
  close pressure while still avoiding command dispatch, memory writes, or
  adaptive-state promotion.
- `AgentCollaborationSideEffectBoundary` is the short adapter-facing snapshot:
  it projects the final handoff or monitor into external-call, memory-note, and
  adaptive-state gates plus mode, health, repair-first, reasons, and telemetry
  for service/eval consumers that should not expand the nested packet chain,
  while preserving service command-reason counters and memory-promotion
  command-reason close pressure from the final service/eval handoff dashboard.
- `AgentCollaborationSideEffectBoundaryHistoryRecorder` appends those short
  snapshots into eval/dashboard rows, reporting dispatch, memory-note,
  adaptive-state, repair, and repair-first rates before any adapter performs
  the actual side effects. Its health and history record expose
  `allows_service_advance` and `requires_repair_first` so watch rows stay
  observable while repair rows block writers, and its dashboard keeps the same
  command-reason pressure counters for downstream side-effect admission.
- `AgentCollaborationSideEffectBoundaryGate` applies that recorded boundary
  health back to the current adapter-facing gates, giving service/memory
  adapters a final pure-data admission decision immediately before they own
  execution.
- `AgentCollaborationSideEffectBoundaryHandoff` composes boundary recording,
  boundary-health evaluation, and final gating into one adapter-facing packet
  with combined telemetry and no side-effect execution.
- `AgentCollaborationSideEffectBoundaryHandoffHistoryRecorder` persists those
  final adapter handoff rows into service/eval trend health for dispatch,
  memory-note, adaptive-state, repair, repair-first, blocked-gate, and reason
  pressure. Its health and history record expose the same service-advance
  helpers for the final adapter-facing handoff trend, with command-reason and
  memory-promotion close pressure aggregated from the boundary rows.
- `AgentCollaborationSideEffectBoundaryHandoffMonitor` composes final boundary
  gating with handoff-history recording, giving adapters one replayable
  service/eval persistence record before they execute service commands, write
  memory notes, or admit adaptive-state changes.
- `AgentCollaborationSideEffectBoundaryHandoffMonitorSummary` flattens that
  monitor record into dashboard-ready mode, health, side-effect admission,
  repair-rate, repair-first-rate, blocked-gate counters, and the same
  command-reason pressure evidence.
- `AgentCollaborationSideEffectBoundaryHandoffMonitorSummaryHistoryRecorder`
  appends those flat monitor rows and recomputes cross-turn health so
  service/eval can distinguish final boundary repair pressure from handoff
  trend repair pressure before opening the next adapter side-effect boundary.
- `AgentCollaborationSideEffectBoundaryHandoffMonitorGate` applies that
  recorded monitor health to the next final adapter gate: stable preserves
  admission, watch observes, and repair closes dispatch, memory-note, and
  adaptive-state side effects until repair-first work runs.
- `AgentCollaborationSideEffectBoundaryHandoffMonitorHandoff` composes monitor
  summary recording and recorded-health gating into one final adapter packet,
  so service/eval can persist the row and read final side-effect admission from
  the same pure-data boundary without losing memory-promotion command-reason
  close pressure.
- `AgentCollaborationSideEffectBoundaryHandoffMonitorHandoffHistoryRecorder`
  persists those final gate-applied adapter packets as dashboard rows for
  final-mode, dispatch, memory-note, adaptive-state, blocked-gate, repair, and
  repair-first trends, plus service command-reason and memory-promotion
  command-reason close counters.
- `AgentCollaborationSideEffectBoundaryHandoffMonitorHandoffGate` applies that
  final-packet trend health to the next adapter boundary, preserving stable
  gates and forcing repair-first when final gate-applied packets drift.
- `AgentCollaborationAdapterSideEffectAdmission` is the short final read model
  for service/eval adapters: mode, health, dispatch, memory-note, adaptive-state
  admission, repair-first, gates, reasons, and telemetry without expanding the
  nested final handoff chain. It also carries the service command-reason and
  memory-promotion command-reason close counters preserved by the side-effect
  boundary.
- `AgentCollaborationAdapterSideEffectAdmissionSummary` and
  `AgentCollaborationAdapterSideEffectAdmissionHistoryRecorder` let service/eval
  persist that final read model as stable, ordered ledger rows. The companion
  dashboard and health policy turn adapter admission trends into stable/watch/
  repair pressure before the next service dispatch, memory-note promotion, or
  adaptive-state admission is attempted, while preserving those command-reason
  pressure counters for downstream monitors.
- `AgentCollaborationAdapterSideEffectAdmissionGate` applies those short
  adapter-admission trends to the next admission read model, preserving clean
  gates and closing all side-effect gates when the persisted final-admission
  trend requires repair-first.
- `AgentCollaborationAdapterSideEffectAdmissionMonitor` is the shortest
  service/eval close point for final side-effect admission: record the admission
  row, recompute adapter-admission health, apply the gate, and return one
  telemetry-rich record before the service attempts dispatch, memory-note, or
  adaptive-state side effects. Its summary/history dashboards keep the same
  memory-promotion command-reason close pressure.
- `AgentCollaborationAdapterSideEffectAdmissionMonitorSummaryHistoryRecorder`
  persists those final monitor records as ordered service/eval ledger rows,
  giving dashboards a compact trend for effective mode, admission health,
  dispatch, memory-note, adaptive-state, blocked-gate, repair, and repair-first
  pressure at the last adapter boundary, plus command-reason evidence.
- `AgentCollaborationAdapterSideEffectAdmissionMonitorGate` applies the final
  monitor-history health before the next side-effect attempt, so stable history
  keeps dispatch/memory/adaptive gates open while dirty final-monitor trends
  force repair-first and close all side-effect gates.
- `AgentCollaborationAdapterSideEffectAdmissionMonitorHandoff` composes final
  monitor-summary recording with that trend gate, returning one persisted
  adapter packet for service/eval before any external command, memory note, or
  adaptive-state write is attempted.
- `AgentCollaborationAdapterSideEffectAdmissionMonitorHandoffHistoryRecorder`
  persists those final adapter packets as a short handoff ledger, while
  `AgentCollaborationAdapterSideEffectAdmissionMonitorHandoffGate` reapplies
  the handoff-history health to the next service/eval boundary.
- Side-effect boundary handoff, handoff-monitor, monitor-handoff, adapter
  admission monitor, and adapter admission monitor-handoff records expose
  `allows_service_advance` and `requires_repair_first` directly, so adapters
  can read wrapper-level admission without expanding nested gate decisions.
- `AgentCollaborationAdapterSideEffectAdmission::from_adapter_monitor_handoff_gate`
  turns that final gate decision back into the adapter-admission read model used
  by the next closed-loop side-effect boundary.
- closed-loop step assembly ties report-gate, loopback, ledger append, and
  business-loop control into one deterministic service-facing envelope.
- service command planning converts the final business-loop plan into explicit
  promote, hold, repair, enqueue, and telemetry commands without executing IO.
- service command audit records applied, missing, failed, and skipped executor
  receipts so command execution drift can feed the next coordinator repair loop.
- service feedback turns dirty command audits into deterministic repair tasks
  and a next queue without executing any external side effects.
- service turnover merges command-execution repair tasks with planned business
  follow-ups so the next scheduler wave has one deterministic queue.
- service execution reports collect command planning, receipt audit, feedback,
  and turnover into one post-execution envelope.
- service execution health gates convert service-local receipt-close health
  into repair-first admission and a merged next queue before another service
  command boundary is opened.
- service execution health gate summaries expose that admission row as counts,
  booleans, stable repair ids, and stable next-queue ids for service/eval
  persistence.
- service execution health gate records combine append, health, gate, full
  next-queue handoff, and compact summary into the receipt-close boundary the
  outer service can persist before opening another command turn.
- service execution health gate histories aggregate those compact rows into
  cross-turn admitted/repair-first pressure for service/eval dashboards.
- service execution health gate history records append compact gate summaries
  and recompute stable/watch/repair trend health for the next service/eval
  boundary.
- service command plan, audit, feedback, turnover, execution, gate, handoff,
  monitor, and monitor-handoff health records expose `records()`,
  `allows_service_advance`, and `requires_repair_first` as the shared
  service/eval read model. Stable/watch rows are admissible for observation and
  queue continuation; repair rows force repair-first scheduling before another
  command receipt, memory, adaptive-state, or external side-effect boundary.
- service execution health gate trend gates convert that cross-turn health into
  the next boundary's admission decision and repair-first queue handoff.
- service execution health gate trend handoffs package append, trend health,
  trend gate, full next queue, and compact summary as one replayable
  service/eval row.
- service execution health gate trend handoff histories aggregate those rows
  without preserving full queue payloads, giving eval dashboards a compact view
  of trend-health and repair-first pressure.
- service execution health gate trend handoff health/recorder types let eval
  persist the compact row, recompute stable/watch/repair state, and feed that
  state back to `norion-core` scheduling without replaying task payloads or
  touching `norion-memory`.
- service execution health gate trend handoff monitors append the compact
  handoff summary, recompute handoff-history health, and gate the current next
  queue in one replayable service/eval boundary.
- service execution health gate trend handoff monitor summaries and dashboards
  are the payload-free eval rows for requested admission, effective admission,
  repair-first pressure, handoff-health mix, queued work, and blockers.
- service execution health gate trend handoff monitor summary history recorders
  append those rows and compute monitor health so the next service/eval boundary
  can observe or repair handoff drift without replaying full queues.
- service execution health gate trend handoff monitor gates apply that monitor
  health to the next scheduler queue: stable and watch health preserve current
  admission, while repair health blocks admission and appends deterministic
  `service-execution-health-gate-handoff-monitor` repair tasks without writing
  `norion-memory` notes or executing service commands.
- service execution health gate trend handoff monitor handoffs compose the
  monitor-summary append and monitor-health gate into one service/eval packet
  when adapters want the next-queue decision and persisted trend row from a
  single pure-data call.
- service execution health gate trend handoff monitor handoff summaries and
  histories persist those gate-applied rows as compact dashboard evidence for
  admission rate, repair-first pressure, monitor-health mix, queued work, and
  blockers before another service/eval boundary is opened.
- service execution health gate trend handoff monitor handoff gates convert
  that compact handoff trend health back into next-queue admission, preserving
  stable/watch rows and appending deterministic
  `service-execution-health-gate-handoff-monitor-handoff` repair tasks for
  repair-level trends.
- service execution health gate trend handoff monitor handoff handoffs compose
  the compact-row append and final trend gate into one service/eval packet so
  adapters can persist the row and consume the gated next queue without
  re-running either step.
- service execution final adapter projections use
  `AgentAdapterBoundaryGate::from_service_execution_final_handoff` or
  `from_service_execution_final_gate_decision` to turn that final packet into a
  service-adapter owner gate. Stable packets open service, memory, and adaptive
  promotion; watch/non-repair packets keep service observation open while
  closing memory/adaptive promotion; repair packets close the adapter boundary
  and preserve service-execution blockers for repair-first scheduling. Use the
  matching `AgentAdapterBoundarySnapshot::from_service_execution_final_*` and
  `AgentAdapterBoundarySummaryHistoryRecorder::record_service_execution_final_*`
  helpers when eval/service wants the projected row recorded beside the final
  gated queue.
- closed-loop execution reports bind the agent-side step and service-side
  execution report into one full-cycle audit object.
- closed-loop execution summaries expose compact eval/dashboard counters and
  blocker lists without requiring consumers to inspect nested reports.
- closed-loop execution history rolls those summary rows into multi-run clean
  rate, service pressure, admission mix, queued repair, reward, and latest
  blocker counters for eval and service dashboards.
- closed-loop execution health turns dashboard drift into stable/watch/repair
  decisions without applying any side effects.
- closed-loop execution health exposes `allows_service_advance` and
  `requires_repair_first`, keeping stable/watch rows observable while routing
  repair rows through repair-first scheduling before service/eval opens another
  memory, adaptive-state, command, or external side-effect boundary.
- closed-loop next-turn planning converts health plus queue state into
  continue/observe/repair/idle scheduling intent and adaptive-evolution
  admission.
- closed-loop dispatch preparation converts that intent into a skipped turn or
  budget-audited dispatch wave.
- closed-loop prepared execution prevents idle turns from reaching `EnginePort`
  and keeps runtime failures available for the ordinary cycle close path.
- closed-loop prepared cycle closing emits a report only for executed waves.
- closed-loop runtime turns compose the guarded next-turn path into one
  service-facing envelope while keeping all side effects behind ports and
  service commands.
- closed-loop runtime business turns bridge executed runtime reports into
  memory submission and business-loop admission while preserving skipped turns.
- closed-loop runtime service command requests expose expected side effects
  before the service executes them.
- closed-loop runtime service requests compose guarded runtime execution,
  memory handoff, business-loop close, and command planning up to the
  pre-side-effect boundary.
- closed-loop runtime service command gates block skipped requests and
  unadmitted adaptive-state writes before service receipts can pretend they
  succeeded.
- closed-loop runtime service dispatches expose command plans only when the
  pre-execution gate is clean.
- closed-loop runtime service dispatch summaries compact executable status,
  command kinds, and gate blockers before side effects are attempted.
- closed-loop runtime service receipt intake rejects receipts that do not belong
  to the executable dispatch before history can be appended.
- closed-loop runtime service intake repair plans turn rejected receipts into a
  scheduler-visible repair queue.
- closed-loop runtime service dispatch continuations choose between updated
  outcome history and prior-history repair scheduling for the next runtime turn.
- closed-loop runtime service dispatch continuation summaries expose compact
  outcome, intake, repair, health, queue, and history counters.
- closed-loop runtime service runners consume service-owned receipts and return
  the next runtime input through the intake-aware continuation path.
- closed-loop runtime service run summaries merge pre-execution and
  post-execution counters into one eval-friendly row.
- closed-loop runtime service run statuses classify clean closure,
  dispatch-gate blocking, and receipt-intake drift.
- closed-loop runtime service run histories preserve attempt-level dashboard
  evidence for both clean and blocked service-run attempts.
- closed-loop runtime service run history recorders append completed attempts
  and recompute dashboard/health telemetry without touching execution history.
- closed-loop runtime service run health classifies attempt-level service
  execution drift without appending fake execution summaries.
- closed-loop runtime service preflight combines execution and service-run
  health into the final next-turn mode before another runtime turn.
- closed-loop runtime service preflight side-effect admission projects that mode
  into dispatch, memory-note, and adaptive-state gates for service/eval.
- closed-loop runtime service preflight adapter-boundary projections reuse that
  side-effect admission as the service-adapter owner gate and let eval record
  either the original next-turn queue or the continuation's merged follow-up
  queue in the same adapter-boundary health ledger.
- closed-loop runtime service preflight follow-up plans convert observed or
  repaired preflight reasons into scheduler-visible tasks.
- closed-loop runtime service preflight continuations package the merged
  preflight queue back into the next runtime turn input.
- closed-loop runtime service loop state snapshots package execution history,
  service-run attempt history, preflight continuation, next runtime input, and
  telemetry for service/eval persistence.
- closed-loop runtime service loop state summaries compact those snapshots into
  service/eval rows without expanding queued tasks or histories, while carrying
  flattened side-effect admission gates.
- closed-loop runtime service loop advances compose a completed service run into
  updated attempt history, preflight, and the next loop-state snapshot.
- closed-loop runtime service loop runners compose receipt-aware service runs
  and loop advances into one adapter-facing transition.
- closed-loop runtime service loop-run summaries compact the whole transition
  into one service/eval row with command-gate status, side-effect gate counts,
  side-effect admission health, and gate booleans.
- closed-loop runtime service loop-run histories aggregate those transition rows
  into service/eval dashboards across turns, including dispatch, memory-note,
  adaptive admission rates, command-gate allowed rate, and side-effect gate
  pressure.
- closed-loop runtime service loop-run health classifies transition-level
  intake drift, repair-first pressure, low closed/adaptive rates, and latest
  blockers without changing side-effect ownership.
- closed-loop runtime service loop-run history recorders append transition
  rows and emit dashboard/health telemetry for service/eval persistence.
- closed-loop runtime service loop-run health and history records expose
  `records()`, `allows_service_advance`, and `requires_repair_first`, so
  stable/watch transition rows remain observable while repair-level drift
  schedules repair before another service/eval side-effect boundary.
- closed-loop runtime service loop-run control plans translate transition
  health and queue state into scheduler/adaptive-evolution admission for the
  next daemon-style transition. Use
  `AgentAdapterBoundaryGate::from_runtime_service_loop_control_plan` or
  `from_runtime_service_loop_control_record` when that control state is the
  final adapter boundary: `Continue` with stable health opens service, memory,
  and adaptive promotion; `Observe` keeps service scheduling open while closing
  memory/adaptive promotion; `Idle` is watch-level closed observation with an
  empty queue; and `Repair` closes the adapter boundary.
- closed-loop runtime service loop-run daemon continuations project persisted
  daemon state with `AgentAdapterBoundaryGate::from_runtime_service_loop_daemon_record`
  or `from_runtime_service_loop_daemon_continuation`. Stable `Continue`
  continuations open service, memory, and adaptive promotion for the next
  daemon runtime queue; watch health keeps service observation open while
  closing memory/adaptive promotion; repair transition or control health closes
  all side-effect lanes and feeds adapter-boundary repair handoff tasks.
- closed-loop runtime service loop-run daemon input plans project the
  receipt-bound next daemon input with
  `AgentAdapterBoundaryGate::from_runtime_service_loop_daemon_input_plan`. This
  is a conservative watch boundary: it preserves next queue ids and positive
  dispatch evidence for service/eval replay, but it does not authorize memory
  notes or adaptive-state promotion until a later stable health boundary exists.
- closed-loop runtime service loop-run daemon request plans project the earliest
  daemon-request adapter boundary with
  `AgentAdapterBoundaryGate::from_runtime_service_loop_daemon_request_plan` or
  `from_runtime_service_loop_daemon_request_record`. Stable `Continue` opens
  service scheduling, memory notes, and adaptive state for the current runtime
  queue; observe/watch-like plans keep only service scheduling observable; and
  repair-first or `Repair` mode closes all side-effect lanes before the request
  is executed or monitored.
- closed-loop runtime service loop-run daemon request monitored plans project
  the request/control-health boundary before close with
  `AgentAdapterBoundaryGate::from_runtime_service_loop_daemon_request_monitored_plan`
  or `from_runtime_service_loop_daemon_request_monitored_record`. Stable
  `Continue` opens service scheduling, memory-note promotion, and adaptive
  state for the current runtime queue; watch keeps service observation open but
  closes memory/adaptive promotion; repair turns request/control blockers into
  adapter-boundary repair handoff tasks before the request is closed.
- closed-loop runtime service loop-run daemon request monitored-close packets
  project the later daemon/request close boundary into the same adapter read
  model with
  `AgentAdapterBoundaryGate::from_runtime_service_loop_daemon_request_monitored_close_plan`
  or `from_runtime_service_loop_daemon_request_monitored_close_run_record`, or
  `from_runtime_service_loop_daemon_request_monitored_close_continuation`.
  The projection uses the packet's monitored-close health, request health
  status, daemon-control health status, mode, schedulability,
  side-effect-dispatch rate, memory-note rate, adaptive admission, and
  repair-first state. Stable `Continue` packets open service, memory, and
  adaptive promotion for the current runtime queue, with continuation packets
  reading that queue from the next daemon runtime input; watch packets keep
  only service observation/scheduling open; repair packets close all three
  side-effect gates and feed repair-first handoff helpers.
- closed-loop runtime service loop-run monitors combine transition recording
  and control planning into one service/eval persistence boundary.
- closed-loop runtime service loop-run control summaries expose flat
  dashboard/eval rows for latest status, next mode, health, rates, queue
  pressure, command-gate allowed rate, side-effect gate pressure,
  dispatch/memory/adaptive admission rates, admission booleans, and reasons.
- closed-loop runtime service loop-run control summary histories aggregate those
  rows into schedule, dispatch-admission, memory-note admission,
  adaptive-admission, repair-first, idle, queue-pressure, and latest-reason
  counters.
- closed-loop runtime service loop-run control health evaluates daemon-control
  trends into stable/watch/repair gates for service/eval before another
  self-evolution transition is admitted.
- closed-loop runtime service loop-run control health records expose
  `records()`, `allows_service_advance`, and `requires_repair_first` for
  daemon-control admission without opening side effects.
- closed-loop runtime service loop-run control summary history recorders append
  flat daemon-control rows and recompute dashboard/health telemetry in one
  pure-data step.
- closed-loop runtime service loop-run daemon runners compose receipt-aware
  loop execution, transition monitoring, flat daemon-control history, and
  telemetry into one service/eval record without owning side effects.
- closed-loop runtime service loop-run daemon continuations package next runtime
  input, histories, policies, admission flags, dispatch/memory admission rates,
  and health statuses for the next daemon turn.
- closed-loop runtime service loop-run daemon request planners prepare the
  pre-receipt command boundary while preserving daemon histories, policies, and
  dispatch/memory admission rates.
- closed-loop runtime service loop-run daemon request runners persist expected
  command/dispatch boundaries, and receipt closers finish those transitions
  without replaying runtime or memory ports.
- closed-loop runtime service loop-run daemon request summary histories expose
  pre-receipt executable rate, command-gate allowed rate, skipped/blocked
  pressure, command pressure, side-effect gate pressure, and dispatch/memory
  admission rates for service/eval repair gates.
- closed-loop runtime service loop-run daemon request health records expose
  `records()`, `allows_service_advance`, and `requires_repair_first` before the
  monitored receipt close consumes service-owned receipts.
- closed-loop runtime service loop-run daemon request monitored receipt closers
  bind request-boundary health to post-receipt daemon records for one persistence
  row across the service boundary.
- closed-loop runtime service loop-run daemon request monitored close summaries
    expose flat service/eval rows for request health, daemon run status,
    daemon-control health, request command-gate status, request side-effect gate
    pressure, dispatch/memory/adaptive admission evidence, history counters,
    and blockers.
- closed-loop runtime service loop-run daemon request monitored close summary
    histories aggregate those rows into cross-turn executable/closed rates,
    dispatch-admission, memory-note admission, adaptive-admission rates, repair
    pressure, request command-gate allowed rate, request side-effect gate
    pressure, repair-first pressure, and stable/watch/repair health.
- closed-loop runtime service loop-run daemon request monitored close health
    records expose `records()`, `allows_service_advance`, and
    `requires_repair_first` before another close-aware daemon request advances.
- closed-loop runtime service loop-run daemon request monitored close
    continuations package request-boundary state plus monitored-close trend
    health and dispatch/memory admission-rate evidence after the close-summary
    row is recorded.
- closed-loop runtime service loop-run daemon request monitored continuations
    preserve request-summary history, request health, daemon-control health, and
    dispatch/memory/adaptive admission evidence with the next daemon
    continuation.
- closed-loop runtime service loop-run daemon request monitored planners build
    the next request boundary from that monitored state without dropping the
    request-summary history or admission-rate evidence needed by the following
    close.
- closed-loop runtime service loop-run daemon request monitored runners keep
    request-summary history attached while running the next expected-command
    boundary and closing it with receipts, including dispatch/memory
    admission-rate telemetry on the record wrappers.
- closed-loop runtime service loop-run daemon input planners rebuild full daemon
  inputs from persisted continuation state and service-owned receipts without
  opening any runtime or side-effect gate, while exposing dispatch/memory
  admission rates on the input plan for service/eval replay. Use the adapter
  boundary `from_runtime_service_loop_daemon_input_plan` snapshot/recorder
  helpers when eval wants that assembled input stored as a watch-level ledger row
  before the daemon runner consumes it.
- closed-loop runtime service turns audit command receipts, update history, and
  expose the next queue for the following scheduler pass.
- closed-loop runtime continuations turn the service result back into the next
  runtime input with dashboard and health state attached.
- closed-loop runtime service outcomes combine receipt audit and continuation
  planning after receipts arrive.
- closed-loop runtime service outcome summaries expose compact service/eval rows
  with command-gate and side-effect gate evidence without changing side-effect
  ownership.
- eval/report final-packet admission histories aggregate final admission rows
  into stable/watch/repair dashboards before service/eval accepts another
  report handoff. The final handoff gate appends repair tasks for dirty trend
  history while leaving memory notes, service commands, and tool side effects
  outside `norion-agent`.

## Adapter Wiring Contract

Use this crate as the data boundary between the six-window coordinator and the
side-effect owners:

| Owner | Consumes from `norion-agent` | Returns to `norion-agent` | Must not bypass |
| --- | --- | --- | --- |
| `norion-core` | `AgentTaskQueue`, `RecursiveAgentSchedule`, `TaskDispatchPlan`, `AgentClosedLoopRuntimeTurnInput` | `AgentResult` through `EnginePort` or `AgentWaveExecution` | budget gates, dispatch gates, repair-first queue handoffs |
| `norion-memory` | `AgentCycleHandoff.memory_notes`, `MemorySubmissionGateDecision`, memory admission booleans | `MemorySubmissionReport` through `MemoryPort` | unresolved conflict gates, reflection gates, blocked handoffs |
| service adapter | service command requests, runtime service dispatches, side-effect admission gates | command receipts, intake results, service-run histories | command gates, receipt-intake validation, adaptive-state admission |
| eval/reporting | compact summaries, dashboards, health records, telemetry strings | validation/runtime evidence refs, gate policies, repair decisions | full queue mutation without the matching gate record |

The minimal closed-loop handoff should keep these rows together:

1. `AgentCollaborationDispatchPreflight` plus
   `AgentCollaborationDispatchPreflightHistoryGate` before the first scheduler
   wave, then `AgentCollaborationDispatchPreflightSchedulerHandoff` for the
   effective queue that `norion-core` should consume. Use
   `AgentCollaborationPlan::gate` only when budget, schedule, and preflight
   trend history are being gated separately by the caller.
2. `TaskDispatchPlan::gate` before `EnginePort` is called.
3. `AgentRunReport::gate` before memory, adaptive state, or external calls.
4. `MemoryPromotionGate`, then `AgentCycleLedgerRecord` with
   `MemoryPromotionLedgerSummary`, then `AgentCycleHandoff` plus
   `MemorySubmissionReport`, before a memory note is treated as persisted.
   The ledger summary is the payload-light pre-submit evidence row: it
   distinguishes no promotion candidates, watch/blocked promotion gates,
   repair-first promotion gates, and promotable notes before eval checks
   post-submit memory failures. `AgentCycleLedger::summary` rolls those rows
   up into memory-promotion status counters so service/eval can spot repeated
   pre-submit memory pressure without replaying nested gate payloads.
   Report-gate `tool_build_*` blockers roll up into
   `tool_build_blocked_cycles` on the same ledger, keeping builder receipt
   repair pressure visible without expanding report-gate payloads.
   `AgentCycleLedger::admission` treats blocked or repair-first promotion gate
   pressure, or unresolved tool-build blocker pressure, as repair admission
   before the business loop promotes adaptive state, even when the latest
   ordinary loopback queue would otherwise look promotable.
   `AgentBusinessLoopPlan::summary` carries those memory-promotion and ledger
   counters forward so service/eval can explain why an adaptive-state candidate
   was withheld without expanding the full ledger. When the service adapter
   turns that plan into commands, `AgentServiceCommandPlanSummary` preserves
   total repair/hold reason counts plus memory-promotion and tool-build reason
   counts so eval can distinguish memory-gate or builder-receipt repair mode
   from generic command pressure. `AgentServiceExecutionReportSummary` and
   `AgentServiceExecutionDashboard` preserve those reason-family counts after
   receipts close, so a clean executor response does not flatten builder or
   memory-gate repair context into generic service pressure.
   `AgentCollaborationServiceExecutionSummary` and its history/dashboard carry
   those command-reason counters forward again, including the memory-promotion
   and tool-build execution counts used by collaboration health policies to
   request repair before self-evolution or adapter handoff treats the service
   command boundary as clean.
5. `AgentReportGateDecision` or the corresponding health-gate handoff before a
   cycle is accepted by eval.
6. `AgentClosedLoopRuntimeServiceDispatchContinuation` or the daemon
   continuation packet before the next self-evolution turn is resumed.

Every persisted boundary should store the compact summary row, the health or
gate decision, and the stable queue ids that were visible at that boundary. A
consumer may observe `Watch`, but only `Stable` should promote memory or
adaptive state automatically. `Repair` must preserve the ordinary queue, append
the deterministic repair tasks produced by the gate, and close the side-effect
it was about to open.

`AgentAdapterBoundarySnapshot` is the shortest crate-owned read model for that
final adapter handoff. Build one snapshot from owner gates for `norion-core`,
`norion-memory`, the service adapter, and eval/reporting, then persist
`AgentAdapterBoundarySummary` with `AgentAdapterBoundarySummaryHistoryRecorder`
when eval needs trend health. The snapshot keeps dispatch and service commands
open for watch-level observation, but it does not allow memory-note submission
or adaptive-state promotion unless every owner is stable. Repair-level owner
gates close dispatch, memory, adaptive-state, and service-command admission and
surface the owner-qualified blockers for repair-first scheduling. The service
adapter gate also preserves service command-reason counts,
memory-promotion command-reason counts, and command-reason close pressure from
`AgentCollaborationAdapterSideEffectAdmission`; adapter boundary snapshots,
summaries, dashboards, records, and handoff summaries aggregate those counters
as read-model evidence only, without recomputing service execution health or
dispatching side effects.
Use `AgentAdapterBoundaryGate::from_dispatch_gate`,
`from_memory_promotion_gate`, `from_memory_submission_gate`, `from_report_gate`, and
`from_service_admission` to translate existing crate gate outputs into that
snapshot without re-implementing gate semantics in `norion-core`,
`norion-memory`, service, or eval adapters. When the adapter already has all
four owner decisions, prefer `AgentAdapterBoundarySnapshot::from_boundary_gates`
so the queue ids, owner ordering, watch/repair precedence, and memory/adaptive
promotion rules are computed once by `norion-agent` instead of being
hand-assembled at each boundary. When the service/eval boundary is driven by
the final run-report packet, use
`AgentAdapterBoundaryGate::from_run_report_final_handoff` or
`AgentAdapterBoundaryGate::from_run_report_final_gate_decision` to project it
as the service-adapter owner gate, and use the matching
`AgentAdapterBoundarySnapshot::from_run_report_final_*` helpers to store the
gated queue ids beside the boundary. Stable admitted packets open service,
memory, and adaptive promotion; watch/non-repair packets keep service
observation open but close memory/adaptive promotion; repair packets close the
adapter boundary and preserve run-report blocker reasons for repair-first
scheduling. Adapter-boundary history still dominates the final projection: a
clean current run-report packet remains closed for memory, adaptive-state, and
service-command side effects when prior adapter-boundary rows are in repair
health, and the handoff prepends deterministic `adapter-boundary-repair` work
ahead of the run-report gated queue. Use
`AgentAdapterBoundarySummaryHistoryRecorder::record_run_report_final_gate_decision_with_health`
or `record_run_report_final_handoff_with_health` when eval/service wants the
projected row appended into adapter-boundary health, and use
`record_run_report_final_gate_decision_handoff_with_health` or
`record_run_report_final_handoff_handoff_with_health` when the next scheduler
handoff should carry adapter-boundary repair tasks ahead of the run-report
gated queue. When the same boundary is driven by the final service-execution
packet, use
`AgentAdapterBoundaryGate::from_service_execution_final_handoff` or
`from_service_execution_final_gate_decision`, plus the matching
`AgentAdapterBoundarySnapshot::from_service_execution_final_*` and
`AgentAdapterBoundarySummaryHistoryRecorder::record_service_execution_final_*`
helpers. Stable service-execution final packets open service, memory, and
adaptive promotion, watch packets keep service observation open only, and
repair packets close the adapter boundary while keeping service-execution
repair work in the gated queue after adapter-boundary repair tasks. When the
boundary is the runtime service loop-run control plan, use
`AgentAdapterBoundaryGate::from_runtime_service_loop_control_plan` or
`from_runtime_service_loop_control_record`, plus
`AgentAdapterBoundarySnapshot::from_runtime_service_loop_control_*` and
`AgentAdapterBoundarySummaryHistoryRecorder::record_runtime_service_loop_control_*`.
This projects `Continue` as a stable open boundary, `Observe` as service-only
observation, `Idle` as a closed watch boundary with an empty queue, and
`Repair` as repair-first closure. When the final boundary is the persisted
daemon state rather than only the control row, use
`AgentAdapterBoundaryGate::from_runtime_service_loop_daemon_record` or
`from_runtime_service_loop_daemon_continuation`, plus the matching snapshot and
recorder helpers. Those helpers preserve the next daemon runtime queue, open
memory/adaptive promotion only for stable `Continue` with stable transition and
control health, and prepend adapter-boundary repair tasks when transition or
control health is repair-first. When the final boundary is the assembled next
daemon input, use
`AgentAdapterBoundaryGate::from_runtime_service_loop_daemon_input_plan`, plus
the matching snapshot and recorder helpers. That projection records the
receipt-attached input plan as watch-only evidence: it may keep service
observation against the queued runtime work, but keeps memory/adaptive promotion
closed until a later health-bearing boundary opens it. Use
`AgentAdapterBoundaryGate::from_runtime_service_loop_daemon_request_plan` or
`from_runtime_service_loop_daemon_request_record`, plus the matching snapshot
and recorder helpers, when the final adapter boundary is the raw daemon request
plan. Those helpers preserve current runtime queue ids, close memory/adaptive
promotion for observe/non-adaptive request plans, and prepend
adapter-boundary repair tasks before the current queue when the request plan is
repair-first. Use
`AgentAdapterBoundaryGate::from_runtime_service_loop_daemon_request_monitored_plan`
or `from_runtime_service_loop_daemon_request_monitored_record`, plus the
matching snapshot and recorder helpers, when the final adapter boundary is the
daemon request monitored plan. Those helpers preserve current runtime queue ids,
close memory/adaptive promotion for watch or non-adaptive monitored plans, and
prepend adapter-boundary repair tasks before the current queue when request or
daemon-control health is repair-first. Use
`AgentAdapterBoundaryGate::from_runtime_service_loop_daemon_request_monitored_close_plan`
or `from_runtime_service_loop_daemon_request_monitored_close_run_record`, or
`from_runtime_service_loop_daemon_request_monitored_close_continuation`, plus
the matching snapshot and recorder helpers, when the final adapter boundary is
the daemon request monitored-close packet instead of the control plan. Those
helpers preserve the current or next daemon runtime queue ids, close
memory/adaptive promotion for watch or non-adaptive close packets, and prepend
deterministic adapter-boundary repair tasks before any current queue work when
close health or daemon-control health is repair-first. Use
`AgentAdapterBoundarySummaryHistoryRecorder::record_boundary_gates_with_health`
when service/eval wants the snapshot append, dashboard/health recomputation,
and final promotion booleans from one replayable record. The record keeps a
clean current snapshot closed for memory/adaptive promotion if persisted
adapter-boundary history is already in repair-first health. Use
`record_boundary_gates_handoff_with_health` when the next scheduler boundary
also needs a repair-first queue handoff: it converts owner-qualified blockers
and adapter-history health reasons into deterministic `adapter-boundary-repair`
tasks, merges them with the caller's next queue, and still leaves execution of
those tasks to the scheduler and service owner. The returned effective queue is
the repair-before-business contract: preserved business tasks depend on the
generated `adapter-boundary-repair` ids, and `RecursiveAgentScheduler` must
place every referenced repair wave before the first business wave. Use
`AgentAdapterBoundaryHandoff::summary` as the payload-free eval row for
snapshot health, trend health, admission, repair task ids, next queue ids, and
blocked-reason pressure, plus the preserved service command-reason and
memory-promotion command-reason close counters. Persist those rows with
`AgentAdapterBoundaryHandoffHistoryRecorder` when eval/service needs trend
health over final adapter-boundary admission without storing queued task
payloads. Stable history remains admissible, watch history is observable, and
repair-first or repair-task-heavy history should route the next boundary
through adapter-boundary repair scheduling before memory, adaptive-state,
service-command, or eval promotion opens again.
For main-window handoff readiness aggregation, treat
`AgentAdapterBoundaryHandoffSummaryHistory::dashboard()` plus
`AgentAdapterBoundaryHandoffHealth::reasons` as the stable report surface:
dashboards expose admitted/repair-first counts, repair task counts, queue
pressure, blocked-reason counts, latest snapshot/health status, and
memory/adaptive promotion rates; summaries expose stable `repair_task_ids` and
`next_queue_task_ids` so repair work can be reported ahead of preserved
business queue ids. When eval needs reason-code rows rather than dashboard
counts, derive `AgentAdapterBoundaryHandoffHistoryRecord::report_gate_decision`
from the same handoff history record. This keeps pollution isolation,
repair-first, business preservation, and budget/conflict/ownership isolation
visible through structured counts, ids, and reason codes without reopening raw
task payloads.
Apply
`AgentAdapterBoundaryHandoffTrendGate` to that recorded health before consuming
the next handoff: stable trend health preserves admission, watch trend health
allows observation while closing memory/adaptive promotion, and repair trend
health appends deterministic `adapter-boundary-handoff-trend-repair` tasks to
the handoff queue. Persist `AgentAdapterBoundaryHandoffTrendGateSummary` rows
with `AgentAdapterBoundaryHandoffTrendGateHistoryRecorder` when eval/service
needs trend health over the applied gate decision itself. That final trend row
tracks requested vs effective admission, repair-first pressure, trend-repair
task ids, queue ids, blocker counts, and the preserved service command-reason
and memory-promotion command-reason close counters without retaining full task
payloads.
Use `AgentAdapterBoundaryHandoffTrendAdmissionGate` when service/eval wants one
pure-data call for the final adapter boundary: it records the applied
trend-gate decision, recomputes trend-gate health, closes memory/adaptive
promotion for watch history, and appends
`adapter-boundary-handoff-trend-gate-repair` tasks for repair history while
preserving the original business queue ids. Persist
`AgentAdapterBoundaryHandoffTrendAdmission::summary` when eval/service only
needs the final payload-free row for trend health, requested/effective
admission, promotion booleans, repair ids, queue ids, record count, and blocker
pressure plus the preserved service command-reason and memory-promotion
command-reason close counters. Keep
`AgentAdapterBoundaryHandoffTrendAdmissionSummaryHistory` beside the
service/eval adapter ledger and append rows with
`AgentAdapterBoundaryHandoffTrendAdmissionHistoryRecorder` when the next final
adapter boundary must be gated by cross-turn effective-admission,
memory/adaptive-promotion, repair-first, trend-history repair, queue-pressure,
blocker evidence, and those same command-reason counters. Prefer
`AgentAdapterBoundaryHandoffTrendAdmissionMonitor` when service/eval wants the
final admission gate, admission-history append, trend-health recomputation, and
gated queue returned as one replayable packet before any side-effect owner
executes work; the monitor record keeps the same counters visible without
opening service, memory, or adaptive-state side effects. After the monitor, use
`AgentAdapterBoundaryHandoffTrendAdmissionContinuationPlanner` to persist the
next adapter-boundary state: gated queue, handoff-trend history,
final-admission history, policies, health statuses, memory/adaptive promotion
booleans, repair-first state, and the preserved command-reason pressure
counters. On the following turn, build
`AgentAdapterBoundaryHandoffTrendAdmissionResumePlan` from that continuation
and call `monitor_next` with the fresh handoff plus handoff-history record; the
plan reuses the persisted histories, policies, and command-reason pressure
evidence while preserving repair-first trend gates. Use
`AgentAdapterBoundaryHandoffTrendAdmissionResumeRunner` when
service/eval wants that resume-plan, next monitor record, and next continuation
as one replayable packet. The resume record, summary, dashboard, history
telemetry, gate decision, gate summary, gate dashboard, gate monitor, monitor
gate, monitor handoff, monitor-handoff summary/dashboard, and final handoff
packet continue to preserve service command-reason counts, memory-promotion
command-reason counts, and memory-promotion command-reason close counters as
pure read-model evidence; none of those layers recomputes service health or
opens service, memory, adaptive-state, or eval side effects. Persist
`AgentAdapterBoundaryHandoffTrendAdmissionResumeSummary` rows with
`AgentAdapterBoundaryHandoffTrendAdmissionResumeHistoryRecorder` when dashboards
or repair gates need cross-resume effective admission, repair-first pressure,
promotion booleans, queue pressure, and prior-to-next history growth without
full task payloads, plus the preserved command-reason pressure. Apply
`AgentAdapterBoundaryHandoffTrendAdmissionResumeGate` to the resume record plus
that history record before the next side-effect boundary consumes the queue;
repair histories append deterministic
`adapter-boundary-handoff-trend-admission-resume-repair` tasks, while watch
histories keep observation but close memory/adaptive promotion. Persist the
resulting `AgentAdapterBoundaryHandoffTrendAdmissionResumeGateSummary` through
`AgentAdapterBoundaryHandoffTrendAdmissionResumeGateHistoryRecorder` as the
last service/eval row before side-effect owners consume the queue; the row
keeps admission booleans, repair task ids, next queue ids, blocked-reason
pressure, and memory/adaptive promotion booleans without carrying task
payloads. Prefer
`AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitor` when service/eval
needs the gate decision, appended final gate history row, final health, and next
queue audit as one packet. Pass that monitor record through
`AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorGate` before a
side-effect owner consumes the queue; it re-applies final gate-history health,
keeps stable/watch rows observable, and turns repair-level final gate drift into
deterministic
`adapter-boundary-handoff-trend-admission-resume-gate-monitor-repair` tasks.
Use `AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoff` when
service/eval wants the final row append and the final queue gate from one
pure-data call. Store
`AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffSummary` rows
with
`AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHistoryRecorder`
when the workflow needs a payload-free, trendable final service/eval row after
that one-call append+gate. The summary history keeps final gate health,
requested/effective admission, repair-first pressure, repair task ids, next queue
ids, blocked-reason pressure, and memory/adaptive promotion booleans visible to
norion-memory, norion-core, service, and eval wiring before any side-effect owner
consumes the queue, while still carrying the same command-reason and
memory-promotion close counters. Direct side-effect boundary and adapter
admission rows now also expose
`service_execution_tool_build_command_reason_count`, and adapter boundary
snapshots/summaries/dashboards preserve that count as
`agent_adapter_boundary_*_service_tool_build_command_reasons` telemetry for
eval dashboards. Side-effect boundary monitor-handoff summaries, dashboards,
history records, and gates preserve it into adapter admission, and adapter
admission monitor plus monitor-handoff summaries, dashboards, history records,
and gates keep the same builder pressure visible before the next adapter owner
gate. Handoff summaries, trend-gate dashboards, trend-admission
monitor records, resume records, resume-gate dashboards, and resume-gate
monitor-handoff summaries carry the same count forward as read-model evidence,
so service/eval can keep builder repair pressure visible without rehydrating
service-execution payloads. Apply
`AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffGate` to that
history record and the final handoff when the workflow needs the persisted final
row health to close memory/adaptive promotion and merge deterministic
`adapter-boundary-handoff-trend-admission-resume-gate-monitor-handoff-repair`
tasks into the queue before service/core/memory owners advance. Prefer
`AgentAdapterBoundaryHandoffTrendAdmissionResumeGateMonitorHandoffHandoff` when
the workflow should append the final summary-history row and apply that final
summary-history gate in one replayable packet before service/core/memory owners
advance.

For the self-evolution side of the workflow, after
`EvolutionAdmissionHandoffTrendContinuationHistoryGate` produces a final
decision, call `summary()` and persist the row with
`EvolutionAdmissionHandoffTrendContinuationHistoryGateHistoryRecorder`. The
resulting health is the small boundary that `norion-core`, `norion-memory`, the
service adapter, and eval can all share: stable history lets core schedule the
business queue and lets memory/adaptive-state/eval consume promotion evidence;
watch history keeps the row observable while closing promotion; repair history
routes the next queue through deterministic continuation repair before any
memory note, adaptive-state promotion, service command, or eval finalization is
treated as admitted. This keeps the final self-evolution handoff replayable as
pure data and leaves every side effect owned by its adapter.
When wiring adapters, read the promotion and admission booleans from the
resulting history record methods. They encode the stable/watch/repair split:
watch can advance service observation but cannot promote memory/adaptive/eval
state, while repair closes admission and exposes repair task ids plus
blocked-reason pressure for the next scheduler turn.
For the concrete side-effect boundary, pass the same record to
`AgentCollaborationSideEffectBoundary::from_evolution_admission_handoff_history`.
This gives `norion-core` a schedulable service-observation gate, gives
`norion-memory` an explicit memory-note gate, and gives the adaptive-state owner
an explicit write gate from the same final admission evidence. The adapter
still owns execution; `norion-agent` only computes the replayable gate state.
If the workflow also needs side-effect boundary history and handoff health in
one call, use
`AgentCollaborationSideEffectBoundaryHandoffMonitor::record_evolution_admission_handoff_history`.
It appends the computed boundary, recomputes boundary health, gates the handoff,
and records handoff-health history without executing any side effect.
If eval/service needs the final monitor-handoff gate packet as well, use
`AgentCollaborationSideEffectBoundaryHandoffMonitorHandoff::record_evolution_admission_handoff_history`.
Stable packets keep service, memory-note, and adaptive-state gates open;
watch packets remain observable while closing promotion; repair packets close
all gates and carry repair-first reasons into the final handoff gate.
When the same final self-evolution row must cross the service/eval adapter
boundary, call
`AgentCollaborationAdapterSideEffectAdmission::from_evolution_admission_handoff_history`
or the shorter adapter projections:
`AgentAdapterBoundaryGate::from_evolution_admission_handoff_history`,
`AgentAdapterBoundarySnapshot::from_evolution_admission_handoff_history`, and
`AgentAdapterBoundarySummaryHistoryRecorder::record_evolution_admission_handoff_history_handoff_with_health`.
When eval/service also needs to persist the final adapter handoff row in the
same call, use
`AgentAdapterBoundaryHandoffHistoryRecorder::record_evolution_admission_handoff_history_with_health`
with the adapter-boundary summary history, handoff-summary history, next queue,
and the two health policies.
Eval/reporting can then call
`AgentAdapterBoundaryHandoffHistoryRecord::report_gate_decision(run_id)` to
turn the recorded adapter handoff into the existing report-gate decision shape.
Stable handoffs are accepted, watch handoffs hold report acceptance without
creating repair-first tasks, and repair handoffs emit deterministic
`eval-adapter-boundary` follow-up tasks.
When eval also wants the decision persisted in its native report-gate ledger,
call `record_report_gate_with_health`; when the next queue should be constrained
by report-gate health in the same boundary, call
`record_report_gate_with_health_gate`. Those helpers append the adapter-derived
decision through `AgentReportGateHistoryRecorder` and keep the service/eval
history row aligned with the adapter-boundary handoff row that produced it.
Use `record_report_gate_trend_handoff` when the same adapter handoff must also
enter eval's health-gate trend handoff: the helper composes the report-history
append, health gate, health-gate trend append, trend gate, repair tasks, and
gated next queue into one replayable read-model packet. Use
`record_report_gate_trend_handoff_monitor` when the workflow needs the next
outer packet too: it appends trend-handoff history, gates that persisted
handoff health, and returns the monitor record with final admission flags,
repair tasks, and queue ids. Use
`record_report_gate_trend_handoff_monitor_handoff` when eval/service also needs
the monitor-summary history row appended and gated before the queue crosses the
final adapter boundary; stable packets keep the business queue, while repair
packets carry both inner report-gate repairs and outer monitor-history repair
tasks. Use `record_report_gate_trend_handoff_monitor_handoff_handoff` when the
monitor-handoff packet also needs its own trend-history append and gate before
the next eval/service handoff. Use
`record_report_gate_trend_handoff_monitor_handoff_handoff_handoff` when that
adapter-derived packet should be appended into eval's final admission history
and gated with the native
`AgentReportGateHealthGateTrendHandoffMonitorHandoffHandoffHandoff` wrapper.
Use `record_report_gate_trend_handoff_monitor_handoff_handoff_handoff_handoff`
for the final history append plus final handoff gate before service/eval
consumes the queue. Stable records preserve the business queue, repair records
merge deterministic eval repair tasks ahead of it, and no memory, service,
tool, or adaptive-state side effect is executed by these helpers.
These wrappers reuse the collaboration side-effect mapping, then expose the
result as a service-adapter owner gate. Stable final records keep service,
memory-note, and adaptive-state admission open; watch final records keep
service observation open while closing memory/adaptive promotion; repair final
records close the adapter boundary and merge deterministic
`adapter-boundary-repair` tasks ahead of the ordinary queue. When the service
execution history reports tool-build command-reason pressure, keep that count on
the side-effect boundary or adapter admission before projecting to
`AgentAdapterBoundaryGate::from_service_admission`; the adapter boundary read
models then preserve it for service/eval without executing tool builds.
For a dirty `ToolBuildReportHistoryGateRecord`, route through
`record_tool_build_report_history_gate_handoff_with_health` and consume the
returned `next_queue`; it carries adapter-boundary repair tasks as dependencies
of memory-note, adaptive-state, eval-finalize, and other ordinary follow-up
tasks, so unresolved builder pressure cannot be bypassed by reusing the
business queue directly.
If the workflow routes through side-effect boundary monitor-handoff or adapter
admission monitor-handoff gates first, use their summary/history rows as the
source of that same count instead of recomputing service execution health.
For runtime service loop wiring, pass
`AgentClosedLoopRuntimeServiceLoopState` through
`AgentAdapterBoundarySnapshot::from_runtime_service_loop_state` after preflight
continuation, and pass `AgentClosedLoopRuntimeServiceLoopAdvance` through
`AgentAdapterBoundarySnapshot::from_runtime_service_loop_advance` after a
service run is folded back into loop state. Both projections reuse preflight
continuation side-effect admission and preserve the object's next runtime
queue: `norion-core` consumes the effective queue from the handoff,
`norion-memory` reads `can_submit_memory_note`, service adapters read
`can_execute_service_commands`, and eval records the summary/health row with
the matching `record_runtime_service_loop_*_with_health` or
`record_runtime_service_loop_*_handoff_with_health` helper. Repair health
closes memory/adaptive promotion and prepends deterministic adapter-boundary
repair tasks while preserving service-preflight repair work ahead of the
runtime queue before any owner treats a side effect as admitted.
